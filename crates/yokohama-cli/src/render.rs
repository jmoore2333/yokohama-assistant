//! ASCII rendering of game state for the CLI: read-only views over
//! `GameState` plus a before/after delta summarizer (design
//! docs/plans/2026-07-18-phase2-cli-driver-design.md).
//!
//! Rendering rule: tests assert content (substrings/line counts), not
//! full-screen golden strings, so cosmetic polish during dogfooding doesn't
//! churn tests.

use std::collections::{BTreeMap, HashSet};

use yokohama_core::game::actions::resolve_acting_area;
use yokohama_core::*;

use crate::board_render;
use crate::decision_render;
use crate::describe;
use crate::input::tech_name;
use crate::panel_render;
use crate::screen_model;
use crate::style::{paint, GoodKind, Tone};

/// Per-seat presentation the panels need from the REPL (Phase 3.2 Task 12):
/// whose hand CONTENTS may be printed, and which seats a bot plays.
///
/// Per SEAT rather than one flag for the table because a mixed game is the
/// point: with a bot at the table, `privacy` OFF has to keep meaning "show ME
/// my cards" without also printing the bot's unfulfilled starting order to the
/// player it is playing against.
///
/// **What the hand mask is and is not.** It closes the panel channel, which is
/// where a bot's private card would otherwise sit in front of a human every
/// single frame. It is NOT an information barrier: in sim mode the human ran
/// the setup wizard (and saw every seat's deal preview there) and can `undo`
/// and `log`. Honest UI hygiene, not a guarantee.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeatDisplay {
    /// Seat index -> hide this seat's hand contents.
    hide_hand: Vec<bool>,
    /// Seat index -> a bot plays this seat.
    bot: Vec<bool>,
}

impl SeatDisplay {
    /// Every seat open and human -- what every caller outside the REPL wants,
    /// and what the pre-bot `players_panel(.., false)` meant.
    pub fn open(seats: usize) -> Self {
        Self { hide_hand: vec![false; seats], bot: vec![false; seats] }
    }

    /// Every seat's hand hidden, every seat human -- the privacy toggle ON with
    /// no bots at the table.
    pub fn hidden(seats: usize) -> Self {
        Self { hide_hand: vec![true; seats], bot: vec![false; seats] }
    }

    /// `hide_hand[i]` / `bot[i]` per seat, in `PlayerId` order.
    pub fn new(hide_hand: Vec<bool>, bot: Vec<bool>) -> Self {
        Self { hide_hand, bot }
    }

    /// Must this seat's hand contents stay masked? A seat past the end of the
    /// table reads as HIDDEN -- the safe direction if a caller ever hands over
    /// a table built for a different game.
    pub fn hides_hand(&self, seat: PlayerId) -> bool {
        self.hide_hand.get(usize::from(seat.0)).copied().unwrap_or(true)
    }

    /// Is a bot playing this seat? An unknown seat is NOT badged -- claiming a
    /// bot that was never configured is the misleading direction.
    pub fn is_bot(&self, seat: PlayerId) -> bool {
        self.bot.get(usize::from(seat.0)).copied().unwrap_or(false)
    }
}

/// [`panel_render::BOT_BADGE`] for a bot seat, nothing for a human one.
///
/// The string itself lives with the dashboard that pads a column to fit it —
/// see its doc for why there is one const and not two.
fn bot_badge(seats: &SeatDisplay, seat: PlayerId) -> &'static str {
    if seats.is_bot(seat) {
        panel_render::BOT_BADGE
    } else {
        ""
    }
}

/// A masked hand: the count, plus WHY it is masked. A bot's hand does not say
/// "'privacy' to toggle" -- the toggle does not reveal it, and pointing a
/// player at a control that will not do what the line promises is worse than
/// saying nothing.
pub(crate) fn masked_hand(cards: usize, is_bot: bool) -> String {
    let plural = if cards == 1 { "" } else { "s" };
    if is_bot {
        format!("{cards} card{plural} (hidden — bot seat)")
    } else {
        format!("{cards} card{plural} (hidden — 'privacy' to toggle)")
    }
}

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
        "Turn {} | {} | {} | {} {} | {} | hand {}",
        state.turn_number,
        paint(Tone::Player(p.color), color_name(p.color)),
        paint(Tone::Prompt, phase_text),
        paint(Tone::Good(GoodKind::Yen), &format!("{} yen", p.money)),
        paint(Tone::Vp, &format!("{} VP", p.victory_points)),
        goods_run(p),
        p.order_cards_in_hand.len(),
    )
}

/// The `c1 s2 t0 f1 i0` goods run, each token painted in its good's color
/// (Task 6 full-color treatment). Space-joined AFTER painting, so the plain
/// render is byte-identical to the old inline format string.
fn goods_run(p: &PlayerState) -> String {
    [
        paint(Tone::Good(GoodKind::Copper), &format!("c{}", p.copper)),
        paint(Tone::Good(GoodKind::Silk), &format!("s{}", p.silk)),
        paint(Tone::Good(GoodKind::Tea), &format!("t{}", p.tea)),
        paint(Tone::Good(GoodKind::Seafood), &format!("f{}", p.seafood)),
        paint(Tone::Good(GoodKind::Imports), &format!("i{}", p.imported_goods)),
    ]
    .join(" ")
}

/// A `─ Title ─────...` rule header opening every state view (Task 6, one
/// design language across views). 40 columns total. `pub(crate)` so the
/// repl-side views (`score`/`log`/`final_score`/`hand`) share the exact same
/// header machinery as the render.rs views rather than hand-rolling their own
/// (item 4, third review).
pub(crate) fn section(title: &str) -> String {
    let head = format!("─ {title} ");
    format!("{head}{}\n", "─".repeat(40usize.saturating_sub(head.chars().count())))
}

/// The staggered board, V3 (Phase-3.5a T2). The ADAPTER: it is the one place
/// that reads `&GameState` on the board's behalf, builds the screen model, and
/// hands it to [`crate::board_render`] — which draws it without ever seeing a
/// `GameState` at all (the R9 seam; read that module's import list).
///
/// Every caller inside this crate keeps calling this name, so the seam cost
/// one parameter: `rules`, which the model needs to recover a building site's
/// PRINTED index (a runtime site is a clone of the registry card and carries
/// no index of its own).
///
/// `legal` is the ENGINE's current legal list, and it is what tints the `◈`
/// destination marks (spec §0.7, added at T4). It is a PARAMETER rather than
/// something derived here for the reason the mark exists at all: the CLI
/// computes no legality, so the board can only show what the engine is already
/// offering. A caller drawing a board outside a decision passes `&[]` and gets
/// an unmarked one.
pub fn board_grid(state: &GameState, rules: &Rules, legal: &[Move]) -> String {
    let colors: Vec<PlayerColor> = state.players.iter().map(|p| p.color).collect();
    board_render::board_view(&screen_model::build_board(state, rules, legal), &colors)
}

/// The composed frame's DASHBOARD (Phase-3.5a T3): the players, tracks,
/// achievements and hand panels. [`board_grid`]'s twin, and the same ADAPTER
/// shape — it is the one place that reads `&GameState` on the dashboard's
/// behalf, builds the screen models, and hands them to
/// [`crate::panel_render`], which draws them without ever seeing a `GameState`
/// (the R9 seam; read that module's import list).
///
/// It replaces `compact_panel` and `compact_panel_oneline`, which built their
/// text straight out of `PlayerState` — the weld R9 exists to remove. `seat` is
/// the seat whose own hand the panel shows; `seats` carries the per-seat
/// privacy and bot policy, already folded by the REPL.
pub fn dashboard(state: &GameState, rules: &Rules, seats: &SeatDisplay, seat: PlayerId) -> String {
    let colors: Vec<PlayerColor> = state.players.iter().map(|p| p.color).collect();
    // `SeatDisplay` and `SeatFacts` hold the same two facts in two modules on
    // purpose (screen_model may not depend on the renderer), so the adapter is
    // where they meet.
    let facts: Vec<screen_model::SeatFacts> = state
        .players
        .iter()
        .map(|p| screen_model::SeatFacts {
            is_bot: seats.is_bot(p.id),
            hand_hidden: seats.hides_hand(p.id),
        })
        .collect();

    // Unmarked (`&[]`): the dashboard reads this model for AREA NAMES alone —
    // where a President stands, which area a track occupant sits in — and none
    // of its panels draws a `◈`. Handing it the legal list would build a
    // marked board nothing looks at.
    let board = screen_model::build_board(state, rules, &[]);
    let panels = screen_model::build_dashboard(state, rules, &facts, seat);
    panel_render::dashboard(&panels, &board, &colors)
}

/// The composed frame's DECISION ZONE (Phase-3.5a T4): the phase chain, the
/// deciding line, the numbered options, the near-miss rows and the standing
/// hint. [`board_grid`]'s and [`dashboard`]'s third sibling, and the same
/// ADAPTER shape — the one place that reads `&GameState` on the zone's behalf,
/// builds the model, words the options, and hands both to
/// [`crate::decision_render`], which draws them without ever seeing a
/// `GameState` (the R9 seam; read that module's import list).
///
/// `rows` is the caller's DISPLAY list, not the legal list: the REPL collapses
/// a Movement family into one entry per destination, the retreat family into
/// one line, Placement into its two options. That grouping stays with the
/// pickers it feeds (`Repl::display_options`), so this takes the entries and
/// their words already assembled and supplies the two things only the model
/// knows — the per-move fee, and whether the board marks a destination
/// reachable.
///
/// `legal` is the ENGINE's list, used for the `◈` marks and the near-miss
/// rows; it is the SAME list the caller grouped `rows` from, passed rather
/// than regenerated so the zone and the board cannot disagree about what is
/// offered.
pub fn decision_zone(
    state: &GameState,
    rules: &Rules,
    legal: &[Move],
    rows: &[DisplayRow],
    ctx: &decision_render::ZoneContext<'_>,
) -> String {
    let colors: Vec<PlayerColor> = state.players.iter().map(|p| p.color).collect();
    let board = screen_model::build_board(state, rules, legal);
    let model = screen_model::build_decision(state, rules, legal);
    let fees = price_rows(state, rules, rows);
    let zone_rows = zone_rows(rows, &fees);
    decision_render::decision_zone(&model, &zone_rows, &board, &colors, ctx)
}

/// How many pages [`decision_zone`] will draw this list in — what the `more`
/// command wraps on.
///
/// A separate entry point rather than a number the REPL could compute, because
/// a page is a budget of ROWS in the zone and how many options fill it depends
/// on how wide they render. Both functions build the same rows through the same
/// two helpers, so the pager and the footer cannot disagree about where page
/// two starts.
pub fn decision_pages(
    state: &GameState,
    rules: &Rules,
    legal: &[Move],
    rows: &[DisplayRow],
) -> usize {
    let board = screen_model::build_board(state, rules, legal);
    let fees = price_rows(state, rules, rows);
    decision_render::page_count(&zone_rows(rows, &fees), &board)
}

/// Every row's fee, priced ONCE by the model: `screen_model` owns the
/// arithmetic (including the placement half T0-review MF1 named), so neither
/// this adapter nor the renderer re-derives a price.
fn price_rows(
    state: &GameState,
    rules: &Rules,
    rows: &[DisplayRow],
) -> Vec<Option<screen_model::FeeModel>> {
    rows.iter()
        .map(|row| row.mv.as_ref().and_then(|mv| screen_model::option_fee(state, rules, mv)))
        .collect()
}

fn zone_rows<'a>(
    rows: &'a [DisplayRow],
    fees: &'a [Option<screen_model::FeeModel>],
) -> Vec<decision_render::Row<'a>> {
    rows.iter()
        .zip(fees.iter())
        .map(|(row, fee)| match &row.mv {
            Some(mv) => decision_render::Row::Move { mv, fee: fee.as_ref(), words: &row.words },
            None => decision_render::Row::Family { words: &row.words },
        })
        .collect()
}

/// One entry of the REPL's display list, handed to [`decision_zone`].
///
/// `mv` is `Some` for an entry that IS a single legal move and `None` for a
/// collapsed family — the distinction the bare-Enter ceremony turns on, carried
/// as data rather than re-derived from the words.
pub struct DisplayRow {
    pub mv: Option<Move>,
    pub words: String,
}

/// Interior text width of one numbered skeleton cell; a full cell adds 2
/// border columns, exactly as a full-size tile box does.
const SKEL_W: usize = 4;
/// Horizontal pitch between skeleton cells. Rows below indent by HALF of it —
/// the same brick offset [`board_grid`] draws at full tile size.
const SKEL_PITCH: usize = SKEL_W + 2;

/// The EMPTY numbered board a live operator is about to fill in: the real
/// staggered pyramid for `player_count` (`board_gen::row_lengths` — widest row
/// on top, each row below offset half a cell), every cell carrying the index
/// the setup wizard will ask for it by.
///
/// Drawn at cell size rather than [`board_grid`]'s full tile size because the
/// operator is matching a SHAPE and a numbering here, not reading tile
/// contents — so the whole board fits any terminal, at any player count.
///
/// Third review, finding I1: `Cell 3/18 tile?` on its own names no physical
/// tile, and entering the board apex-first (the order the rulebook's own p.3
/// diagram reads, since it prints the pyramid the other way up) yields a
/// scrambled graph that `reconcile_board` accepts silently — it checks only
/// that the tile multiset matches. This picture, the orientation wording the
/// wizard prints beside it, and the post-entry board check are the three
/// places that error can now be caught.
pub fn board_skeleton(player_count: u8) -> String {
    let mut out = String::new();
    let mut cell = 0usize;
    for (r, &len) in row_lengths(player_count).iter().enumerate() {
        let indent = " ".repeat(r * (SKEL_PITCH / 2));
        let (mut top, mut mid, mut bottom) = (indent.clone(), indent.clone(), indent);
        for _ in 0..len {
            top.push('┌');
            top.push_str(&"─".repeat(SKEL_W));
            top.push('┐');
            mid.push_str(&format!("│{cell:>3} │"));
            bottom.push('└');
            bottom.push_str(&"─".repeat(SKEL_W));
            bottom.push('┘');
            cell += 1;
        }
        out.push_str(&format!("{top}\n{mid}\n{bottom}\n"));
    }
    out
}

/// Legend footer under the board: the marks a V3 tile draws, the site-slot
/// token key, and the canal crossing rule on a board that has a Canal (2P
/// boards never do). The REPL's `board` command appends this after the grid.
///
/// The ADAPTER for [`crate::board_render::board_legend`], for the same reason
/// [`board_grid`] is one. The old legend keyed the 5-Power token LETTERS
/// (`i=import c=copper …`); V3 spells the face out on the tile itself
/// (`5-Power: 1 Import`), so that key described a board that no longer exists.
/// The letters live on only where they are still drawn — the setup wizard's
/// review summary, via [`five_power_face`].
///
/// The Canal question is answered off `state.board_layout` directly. This used
/// to build a whole second `BoardModel` — one per composed frame, on top of the
/// one [`board_grid`] already builds — to scan its tiles for a single boolean
/// (T2 review N3). `board_layout.areas` is where that fact lives, and asking it
/// there costs a lookup instead of a board.
pub fn board_legend(state: &GameState, _rules: &Rules) -> String {
    let has_canal = state.board_layout.areas.values().any(|a| a.area_type == AreaType::Canal);
    board_render::board_legend(has_canal)
}

/// Per Port board: slot number, zone, and the order card there via
/// `order_compact_line` (or `-` when empty), then the shared order-deck size.
pub fn ports_view(state: &GameState, rules: &Rules) -> String {
    let mut out = section("Ports");
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
    let mut out = section("Laboratories");
    render_lab_board(describe::area_type_name(AreaType::LaboratoryA), &state.lab_state.lab_a, rules, &mut out);
    if let Some(lab_b) = &state.lab_state.lab_b {
        render_lab_board(describe::area_type_name(AreaType::LaboratoryB), lab_b, rules, &mut out);
    }
    out.push_str(&format!("tech deck: {}\n", state.tech_deck.len()));
    out
}

/// Per player: color, money, VP, goods, imports, hand order codes, completed
/// order count, technology names, tokens/mercenaries, assistant/shophouse/
/// trading-house counts by zone (hand/board/warehouse, built) plus the
/// trading house's next Employment-Agency take cost, and President location.
/// Hand CONTENTS mask to a count for any seat `seats` hides (the privacy flag,
/// Task 6, now per-seat -- a bot's hand is always hidden, Phase 3.2 Task 12):
/// the kept starting order lives in the hand, and a live-table laptop gets
/// passed around.
pub fn players_panel(state: &GameState, rules: &Rules, seats: &SeatDisplay) -> String {
    let mut out = section("Players");
    for p in &state.players {
        // The `(seat N)` label is the `reveal <seat>` key (item 7, third
        // review): it appeared nowhere on the game screen, so the seat<->color
        // mapping the reveal command needs was learnable only from the wizard
        // transcript and this panel's ordering.
        out.push_str(&format!(
            "{} (seat {}): {} yen  {} VP  c{} s{} t{} f{}  imports {}{}\n",
            player_prefix(p.color), p.id.0, p.money, p.victory_points, p.copper, p.silk, p.tea, p.seafood, p.imported_goods,
            bot_badge(seats, p.id),
        ));

        let hand = if seats.hides_hand(p.id) {
            masked_hand(p.order_cards_in_hand.len(), seats.is_bot(p.id))
        } else {
            let codes =
                p.order_cards_in_hand.iter().map(|id| format!("O{:02}", id.0)).collect::<Vec<_>>().join(", ");
            if codes.is_empty() { "-".to_string() } else { codes }
        };
        out.push_str(&format!("  hand: {hand}\n"));

        out.push_str(&format!("  completed: {}\n", p.completed_orders.len()));

        let techs = p.technologies.iter().map(|id| tech_name(*id, &rules.techs)).collect::<Vec<_>>().join(", ");
        out.push_str(&format!("  tech: {}\n", if techs.is_empty() { "-" } else { &techs }));

        // Mercenaries print as "N (M unused)": `total()` alone cannot change
        // when one is spent, and it is never the VP-relevant number --
        // `calculate_remainder_bonus` awards 1 VP per UNUSED mercenary
        // (`unused_count()` = available only). Showing the split lets the
        // operator both reconcile face-up/face-down tiles against the table
        // and predict their own remainder score (R1, render.rs deep audit
        // 2026-07-20).
        out.push_str(&format!(
            "  tokens: 5power {}  mercenaries {} ({} unused)\n",
            p.five_power_tokens,
            p.mercenaries.total(),
            p.mercenaries.unused_count(),
        ));

        // ALL SIX `AssistantState` zones, so the printed numbers reconcile to
        // the player's true pawn count. Printing only hand/board/warehouse
        // invited the natural table-side conservation check (they must total
        // 20) and then failed it 72% of the time, sending the operator
        // hunting a pawn that was merely locked on a goal or track (R2,
        // render.rs deep audit 2026-07-20). The three locked zones are
        // omitted when zero to keep the common line short.
        let assistants_on_board: u32 = p.assistants.on_board.values().map(|&c| c as u32).sum();
        let mut locked = String::new();
        if p.assistants.on_achievements > 0 {
            locked.push_str(&format!("  goals {}", p.assistants.on_achievements));
        }
        if p.assistants.on_church > 0 {
            locked.push_str(&format!("  church {}", p.assistants.on_church));
        }
        if p.assistants.on_customs > 0 {
            locked.push_str(&format!("  customs {}", p.assistants.on_customs));
        }
        out.push_str(&format!(
            "  assistants: hand {}  board {}  warehouse {}{}\n",
            p.assistants.in_hand, assistants_on_board, p.assistants.in_warehouse, locked,
        ));
        out.push_str(&format!(
            "  shophouses: hand {}  warehouse {}  built {}\n",
            p.shophouses.in_hand, p.shophouses.in_warehouse, p.shophouses.built.len(),
        ));
        out.push_str(&format!(
            "  trading:    hand {}  warehouse {}  built {}  (next take {} yen)\n",
            p.trading_houses.in_hand,
            p.trading_houses.in_warehouse,
            p.trading_houses.built.len(),
            p.trading_houses.next_cost(),
        ));

        let pres = match p.president_location {
            PresidentLocation::InHand => "in hand".to_string(),
            PresidentLocation::OnBoard(area) => describe::area_label(state, area),
        };
        out.push_str(&format!("  president: {}\n", pres));
    }
    out
}

/// One `site_compact_line` per area holding a `BuildingSite`, in `AreaId`
/// order (`state.board_layout.building_sites`'s own `BTreeMap` iteration
/// order), each prefixed by the area label (`describe::area_label`'s
/// type-name + `(A00)` convention) and suffixed by the same `sh .. | th ..`
/// build/ownership marks the `board` grid's per-cell line shows
/// (`build_marks`, shared with `area_cell_lines` below so the two views
/// can never drift). The `S<n>` number `site_compact_line` prints is the
/// AREA's id, not the physical site card's printed index -- the
/// wizard-entered card number is used only to validate setup entry and
/// isn't persisted onto the runtime `BuildingSite` (Task 12, T2.2a;
/// docs/cli-guide.md §8.5 documents this). `rules` is accepted for
/// interface symmetry with the file's other `*_view` functions (same
/// precedent as `describe::describe_pending_choice`'s `_rules`) but
/// currently unused: site data lives entirely in `state`.
pub fn sites_view(state: &GameState, _rules: &Rules) -> String {
    let mut out = section("Building sites");
    for (&area, site) in &state.board_layout.building_sites {
        out.push_str(&format!(
            "{}  {}  {}  {}\n",
            describe::area_label(state, area),
            site_compact_line(area.0 as usize, site),
            build_marks(state, site),
            adjacency_suffix(state, area),
        ));
    }
    out
}

/// The `adj: ...` board-connectivity suffix for a `sites_view` line
/// (item 7), in TWO honestly separated categories: `adj:` = the ordinary
/// FREE movement neighbors ascending, then each tolled canal-crossing
/// partner as `canal-><id>`.
///
/// Both come straight off `board_layout` -- `adjacent_areas` (the
/// `adjacency` graph) and `canal_crossings` -- never re-derived from
/// geometry, so this can't disagree with what movement actually offers.
/// But `adjacency` is NOT the free-neighbor set on its own: since the
/// staggered-topology work it also carries every derived canal-crossing
/// edge AND the brick edges to the Canal tile itself. Printing it raw
/// (Codex review of PR #4, F1) showed a tolled crossing twice -- once
/// disguised as free adjacency -- and offered the Canal as a destination,
/// which it never is: it holds no assistant, so a President can never step
/// onto it and Telegram's adjacent bonus can never target it. `adj:` is
/// therefore derived by SUBTRACTION here: adjacency minus the crossing
/// partners minus the Canal tile(s). The Canal needs no line of its own --
/// its shore relationships are exactly the `canal->` entries, and the board
/// render draws it as water.
///
/// Each crossing pair is scanned in both orders so it shows on both
/// members' lines. `adj: -` is a defensive fallback for an area with no
/// ordinary neighbor left; the staggered geometry never produces it — every
/// non-Canal tile keeps at least one free brick-offset neighbor (a tip tile
/// still has two brick neighbors, at most one of them the Canal).
fn adjacency_suffix(state: &GameState, area: AreaId) -> String {
    let mut crossings: Vec<AreaId> = state
        .board_layout
        .canal_crossings
        .iter()
        .filter_map(|&(a, b)| {
            if a == area {
                Some(b)
            } else if b == area {
                Some(a)
            } else {
                None
            }
        })
        .collect();
    crossings.sort_unstable_by_key(|a| a.0);
    crossings.dedup();

    let is_canal = |id: &AreaId| {
        state.board_layout.areas.get(id).map(|c| c.area_type == AreaType::Canal).unwrap_or(false)
    };
    let mut ids: Vec<u8> = state
        .board_layout
        .adjacent_areas(area)
        .iter()
        .filter(|a| !crossings.contains(a) && !is_canal(a))
        .map(|a| a.0)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let adj = if ids.is_empty() {
        "-".to_string()
    } else {
        ids.iter().map(|n| format!("A{:02}", n)).collect::<Vec<_>>().join(", ")
    };

    let mut out = format!("adj: {adj}");
    for a in crossings {
        out.push_str(&format!("  canal->A{:02}", a.0));
    }
    out
}

/// POSITIONAL per-slot marks (Phase-2.9 per-slot enumeration fix). The
/// engine offers one `BuildShophouse` per EMPTY slot, so `shophouses_built`
/// carries gaps and a packed owners-only string would attribute owners to
/// the wrong printed rewards -- `sites_view` prints the four DIFFERENT
/// rewards positionally right beside these marks (`[1vp 4vp - 2yen]
/// sh ·B·R`). One mark per slot in printed order: the owner's color letter,
/// or `·` for an empty slot. `build_marks_attribute_each_built_slot_to_its_
/// owner_positionally` pins the attribution (this replaced the old
/// prefix-packing pin, whose premise the engine fix deliberately broke).
///
/// "sh ·B·R | th Y" build/ownership marks for one site: one mark per
/// shophouse slot in printed order (owner color letter or `·`), then the
/// trading-house owner's letter or `·`. Shared by `tile_content_lines`'s
/// board-grid cell and `sites_view`'s per-site line (Task 12) so the two
/// views can't drift on the convention documented at docs/cli-guide.md §8.5.
fn build_marks(state: &GameState, site: &BuildingSite) -> String {
    build_marks_pair(state, site).0
}

/// `(plain, painted)` twins of `build_marks`: owner letters paint in their
/// player's color (a who-owns-what surface), empty slots stay a plain `·`.
/// The plain twin is the layout/width authority per style.rs's rule.
fn build_marks_pair(state: &GameState, site: &BuildingSite) -> (String, String) {
    let mut sh = String::new();
    let mut sh_painted = String::new();
    for o in &site.shophouses_built {
        match o {
            Some(pid) => {
                let c = state.player(*pid).color;
                sh.push(color_letter(c));
                sh_painted.push_str(&paint(Tone::Player(c), &color_letter(c).to_string()));
            }
            None => {
                sh.push('·');
                sh_painted.push('·');
            }
        }
    }
    let (th, th_painted) = match site.trading_house_owner {
        Some(pid) => {
            let c = state.player(pid).color;
            (
                color_letter(c).to_string(),
                paint(Tone::Player(c), &color_letter(c).to_string()),
            )
        }
        None => ("·".to_string(), "·".to_string()),
    };
    (format!("sh {sh} | th {th}"), format!("sh {sh_painted} | th {th_painted}"))
}

/// A Church faith space's printed reward: VP plus its movement rider, in
/// `sites_view`'s reward vocabulary (`3vp 1move`). Single-sourced from the
/// `ChurchTrack` tables -- the same `vp_reward` `delta_summary` prints after a
/// placement, plus the `movement_rider` the engine queues as assistant-move
/// choices. Every faith 4-11 carries a movement rider, so it is always shown.
/// `pub(crate)`: the Church space-choice option text (`describe::describe_move`)
/// reuses this so the choice shows the SAME VP+rider as the `tracks` view.
pub(crate) fn church_reward_text(faith: u8) -> String {
    format!("{}vp {}move", ChurchTrack::vp_reward(faith), ChurchTrack::movement_rider(faith))
}

/// A Customs space's printed reward: VP plus its rider, in `sites_view`'s
/// vocabulary. `CustomsTrack::rider` returns `(warehouse_takes, yen)` --
/// rendered `Nwh`/`Nyen` -- and spaces 5-7 carry no rider, so those lines show
/// VP alone. Single-sourced from the `CustomsTrack` tables.
/// `pub(crate)`: the Customs space-choice option text (`describe::describe_move`)
/// reuses this so the choice shows the SAME VP+rider as the `tracks` view.
pub(crate) fn customs_reward_text(space: u8) -> String {
    let mut s = format!("{}vp", CustomsTrack::vp_reward(space));
    let (warehouse_takes, yen) = CustomsTrack::rider(space);
    if warehouse_takes > 0 {
        s.push_str(&format!(" {warehouse_takes}wh"));
    }
    if yen > 0 {
        s.push_str(&format!(" {yen}yen"));
    }
    s
}

/// One track slot's occupant marker: a real player as their color letter
/// painted in their color (S4), or the 2-player setup dummy
/// ([`PlayerId::DUMMY`], finding D1) as a dim `D` — a named occupant that
/// holds the space AND competes in that track's majority bonus, not the bare
/// `X` void it used to render as.
fn track_occupant(state: &GameState, player: PlayerId) -> String {
    if player.is_dummy() {
        paint(Tone::Dim, "D")
    } else {
        let c = state.player(player).color;
        paint(Tone::Player(c), &color_letter(c).to_string())
    }
}

/// Church Management Board (faith values 4-11, occupant marker or `-`) and
/// Customs Management Board (printed cost via `CustomsTrack::space_cost` --
/// single source, Task 12 -- occupant marker or `-`). In 2-player games the
/// setup dummy holds Church faith-7 and Customs space-4, shown as a dim `D`
/// (`track_occupant`). Each space line also prints its printed VP + rider
/// (`church_reward_text`/`customs_reward_text`, S5) after the occupant.
pub fn tracks_view(state: &GameState) -> String {
    let mut out = section("Tracks");

    // Finding D1 (named-sentinel display): when the 2P dummy holds any track
    // space, a legend names it and states that it competes in the majority
    // bonus -- the `D` in the occupant column is a scoring occupant, not
    // decoration.
    let dummy_present = state.church_track.slots.iter().flatten().any(|s| s.player.is_dummy())
        || state.customs_track.slots.iter().flatten().any(|s| s.player.is_dummy());
    if dummy_present {
        out.push_str(&format!(
            "{} = Dummy — the 2P neutral marker; it holds its space and competes in each track's majority bonus\n",
            paint(Tone::Dim, "D"),
        ));
    }

    out.push_str("Church:\n");
    for i in 0..8 {
        let faith = ChurchTrack::index_to_faith(i);
        let occ = match state.church_track.slots[i] {
            Some(slot) => track_occupant(state, slot.player),
            None => "-".to_string(),
        };
        out.push_str(&format!("  faith {:2}: {}  {}\n", faith, occ, church_reward_text(faith)));
    }

    out.push_str("Customs:\n");
    for i in 0..7 {
        let space = CustomsTrack::index_to_space(i);
        let occ = match state.customs_track.slots[i] {
            Some(slot) => track_occupant(state, slot.player),
            None => "-".to_string(),
        };
        out.push_str(&format!(
            "  space {} (cost {}): {}  {}\n",
            space,
            CustomsTrack::space_cost(space),
            occ,
            customs_reward_text(space),
        ));
    }

    // The Station token is board-level shared state like the two tracks
    // above -- it grants EVERY player +1 Power in its area
    // (`game::power::calculate_power`) -- so a live-tracking operator needs
    // to be able to read its position back and reconcile it against the
    // physical board. Before this line it was unrenderable anywhere in the
    // CLI, and placing it produced an empty delta (I4, external review
    // 2026-07-20).
    out.push_str("Station:\n");
    match state.station_location {
        Some(area) => out.push_str(&format!("  {} (all players +1 Power there)\n", describe::area_label(state, area))),
        None => out.push_str("  not placed\n"),
    }

    // The shared mercenary SUPPLY piles are public table state (owner scope
    // item 10 sufficiency gap) -- board-level like the Station, so they live
    // here. Country codes match §8.4's GB/US/FR/DE/NL vocabulary.
    let s = &state.mercenary_supply;
    out.push_str(&format!(
        "Mercenary supply:  GB {}  US {}  FR {}  DE {}  NL {}\n",
        s.english, s.american, s.french, s.german, s.dutch,
    ));

    out
}

/// Achievement cards in play (`state.achievements`'s fixed 3-card array,
/// `G0`-`G2`): the printed condition (`describe::achievement_desc`), claim
/// state (`open`, or `claimed by` the claiming color(s) in claim order --
/// MORE than one player can claim the same achievement, first claimant at
/// `first_place_vp`, each later one at `subsequent_vp`, so every name in
/// `claimed_by` is listed, not just the first), then one indented line per
/// player with their live progress (`achievement_progress`, Task 14/T2.3)
/// as `cur/need` -- the SAME arithmetic `can_claim_achievement` gates the
/// actual claim on, so the printed `cur/need` numbers themselves can never
/// disagree with the engine's. That arithmetic is not the WHOLE gate,
/// though: `achievement_progress` deliberately excludes
/// `can_claim_achievement`'s other two checks (already claimed by THIS
/// player, and having no assistant in hand to place) -- so a player can
/// still show `cur >= need` ("ready" by the numbers) here while an actual
/// claim attempt is refused for one of those two reasons instead.
/// `rules` is accepted for interface symmetry with the file's other
/// `*_view` functions (same precedent as `sites_view`'s `_rules`) but
/// unused: everything here comes from `state` alone.
pub fn achievements_view(state: &GameState, _rules: &Rules) -> String {
    let mut out = section("Goals");
    for (i, card) in state.achievements.iter().enumerate() {
        let claim_state = if card.claimed_by.is_empty() {
            "open".to_string()
        } else {
            // S4: each claimant's color name paints in that player's color.
            let names: Vec<String> = card
                .claimed_by
                .iter()
                .map(|&pid| {
                    let c = state.player(pid).color;
                    paint(Tone::Player(c), color_name(c))
                })
                .collect();
            format!("claimed by {}", names.join(", "))
        };
        // Item 6: the card's first/subsequent claim VP from the registry --
        // the same `first_place_vp`/`subsequent_vp` `vp_for_player` scores
        // with -- so claim value is visible before committing an assistant.
        out.push_str(&format!(
            "G{i}: {} ({}/{} VP) — {}\n",
            describe::achievement_desc(card.achievement_type),
            card.achievement_type.first_place_vp(),
            card.achievement_type.subsequent_vp(),
            claim_state,
        ));
        for p in &state.players {
            let (cur, need) = achievement_progress(state, p.id, card.achievement_type);
            // S6: a player already in `claimed_by` is marked, so a maxed-out
            // `cur/need` under `claimed by <them>` no longer reads as an
            // invitation to claim again. S4: the color prefix paints.
            let claimed = if card.claimed_by.contains(&p.id) { " (claimed)" } else { "" };
            out.push_str(&format!(
                "   {}: {}/{}{}\n",
                paint(Tone::Player(p.color), color_name(p.color)),
                cur,
                need,
                claimed,
            ));
        }
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
            lines.push(format!("{}: {}", player_prefix(a.color), parts.join(", ")));
        }

        let before_hand: HashSet<OrderCardId> = b.order_cards_in_hand.iter().copied().collect();
        let after_hand: HashSet<OrderCardId> = a.order_cards_in_hand.iter().copied().collect();
        let gained = sorted_order_codes(after_hand.difference(&before_hand).copied());
        let lost = sorted_order_codes(before_hand.difference(&after_hand).copied());
        if !gained.is_empty() {
            lines.push(format!("{}: hand +{}", player_prefix(a.color), gained.join(",")));
        }
        if !lost.is_empty() {
            lines.push(format!("{}: hand -{}", player_prefix(a.color), lost.join(",")));
        }

        let before_tech: HashSet<TechCardId> = b.technologies.iter().copied().collect();
        let mut gained_tech: Vec<TechCardId> =
            a.technologies.iter().copied().filter(|id| !before_tech.contains(id)).collect();
        gained_tech.sort_unstable_by_key(|id| id.0);
        if !gained_tech.is_empty() {
            let names = gained_tech.iter().map(|id| tech_name(*id, &rules.techs)).collect::<Vec<_>>().join(", ");
            lines.push(format!("{}: tech +{}", player_prefix(a.color), names));
        }

        assistant_lines(&mut lines, before, after, b, a);
        warehouse_take_lines(&mut lines, b, a);
        church_customs_lines(&mut lines, b, a);
        token_lines(&mut lines, b, a);

        if a.president_location != b.president_location {
            lines.push(format!(
                "{}: President {} -> {}",
                player_prefix(a.color),
                president_label(after, b.president_location),
                president_label(after, a.president_location),
            ));
        }
    }

    building_lines(&mut lines, before, after);
    achievement_lines(&mut lines, before, after);

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

    // Station token: board-level, and it moves every player's Power in its
    // area, so resolving `PendingChoice::StationPlace` must not produce an
    // EMPTY delta window the operator can't see (I4, external review
    // 2026-07-20).
    if before.station_location != after.station_location {
        match (before.station_location, after.station_location) {
            (None, Some(to)) => lines.push(format!(
                "Station token placed at {} (all players +1 Power there)",
                describe::area_label(after, to)
            )),
            (Some(from), Some(to)) => lines.push(format!(
                "Station token {} -> {}",
                describe::area_label(before, from),
                describe::area_label(after, to)
            )),
            (Some(from), None) => {
                lines.push(format!("Station token removed from {}", describe::area_label(before, from)))
            }
            (None, None) => unreachable!("guarded by the inequality above"),
        }
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
/// A Church/Customs placement drawn from the acting area (rather than hand,
/// when hand is empty -- `take_church_or_customs_assistant` prefers hand,
/// falls back to the acting area) is reserved out of `board_diff` below so
/// it isn't ALSO reported as a spurious "recovered" line. WHICH area that is
/// comes from the engine's own `resolve_acting_area` against the pre-move
/// state, so the mercenary sub-turn case (target area, not the president's)
/// is covered rather than mis-attributed (S1, fixed 2026-07-20).
///
/// One caveat remains, accepted for a v1 CLI display feature: the
/// reservation can mis-attribute by one unit in the rare case where a
/// singleton movement-reward AND a hand-empty Church/Customs placement land
/// in the same move's diff (both draw on the same acting-area count). Not
/// reached by any move shape the current engine actually produces outside a
/// contrived fixture -- and unlike the S1 case, this one does not depend on
/// a state the fuzzer under-samples.
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
    // Acting area comes from the ENGINE's own resolver against the PRE-move
    // state, not from `president_location`: during a mercenary sub-turn the
    // action runs at `mercenary_target_area` instead ("the target area, not
    // the president's", per resolve_acting_area's own callers). Reading the
    // president's location here reserved the wrong area, leaving the real
    // source at -1 and the president's area at +1 -- which the pairing loop
    // below rendered as an "assistant moved from X to Y" that never happened
    // (S1, render.rs deep audit 2026-07-20).
    //
    // Only the acting player can have `from_board > 0`, so resolving against
    // `a.id` is safe even though a mercenary sub-turn's target is global.
    if from_board > 0 {
        if let Ok(area) = resolve_acting_area(before, a.id) {
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
            player_prefix(a.color),
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
                player_prefix(a.color), n, if n == 1 { "" } else { "s" }, describe::area_label(after, area),
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
                player_prefix(a.color), n, if n == 1 { "" } else { "s" }, describe::area_label(after, area),
            ));
        } else {
            lines.push(format!(
                "{}: assistant moved from hand to {}",
                player_prefix(a.color), describe::area_label(after, area),
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
    lines.push(format!("{}: took {} ({})", player_prefix(color), word, cost_text));
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
            player_prefix(a.color), ChurchTrack::vp_reward(space),
        ));
    }
    for &space in &a.customs_positions[b.customs_positions.len()..] {
        lines.push(format!(
            "{}: placed on Customs space {space} (+{} VP)",
            player_prefix(a.color), CustomsTrack::vp_reward(space),
        ));
    }
}

/// "gained mercenary (Country)" / "spent mercenary (Country)" / "gained
/// 5-Power token" lines. A per-move gain above 1 (not reachable under
/// current rules -- `award_mercenaries_for_country_icons` awards at most 1
/// per country per call) is suffixed `x{n}` rather than pluralized, so the
/// line still contains the literal substring "gained mercenary" regardless.
///
/// GAINS are detected on `available + used` (a gain raises the total),
/// SPENDS on `available` alone. Keying spends off the total would be
/// structurally silent: using a mercenary moves the token `available ->
/// used`, which leaves the total untouched (R1, render.rs deep audit
/// 2026-07-20 -- 802/456/492 spends per 300 games went unnarrated).
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
                player_prefix(a.color), describe::country_name(country), suffix,
            ));
        }

        // A spend is a fall in `available` that is NOT a fall in the total
        // (that would be a gain reversal, which no rule produces). The
        // operator owes the table a physical act here -- flip the tile
        // face-down -- so it must be named, and named per country.
        let before_avail = b.mercenaries.available.get(&country).copied().unwrap_or(0);
        let after_avail = a.mercenaries.available.get(&country).copied().unwrap_or(0);
        if after_avail < before_avail && after_n >= before_n {
            let n = before_avail - after_avail;
            let suffix = if n == 1 { String::new() } else { format!(" x{n}") };
            lines.push(format!(
                "{}: spent mercenary ({}){}",
                player_prefix(a.color), describe::country_name(country), suffix,
            ));
        }
    }

    if a.five_power_tokens > b.five_power_tokens {
        let n = a.five_power_tokens - b.five_power_tokens;
        let suffix = if n == 1 { String::new() } else { format!(" x{n}") };
        lines.push(format!("{}: gained 5-Power token{}", player_prefix(a.color), suffix));
    }
}

/// "built shophouse"/"built trading house" lines from `board_layout.
/// building_sites` diffs -- never removed once set, so a `None -> Some`
/// transition on a slot/trading-house-owner is the only case to detect.
/// Board-level (not per-player), since a building site's ownership fields
/// name the owner directly.
/// "claimed achievement" lines from `achievements[i].claimed_by` growth.
/// Board-level like `building_lines`, and for the same reason: the card
/// names its claimant directly.
///
/// A claim also LOCKS one of the claimant's assistants onto the card, which
/// is a physical act the operator owes the table -- and the VP line alone
/// ("Red: +8 VP") never cued it, so the pawn stayed in hand and the count
/// ran high for the rest of the game (R2, render.rs deep audit 2026-07-20).
/// `claimed_by` is append-only and MORE than one player may claim the same
/// card, so this walks the tail rather than assuming a single new claimant.
fn achievement_lines(lines: &mut Vec<String>, before: &GameState, after: &GameState) {
    for (i, after_card) in after.achievements.iter().enumerate() {
        let before_claims = before.achievements.get(i).map(|c| c.claimed_by.len()).unwrap_or(0);
        for &claimant in after_card.claimed_by.iter().skip(before_claims) {
            lines.push(format!(
                "{}: claimed achievement G{} ({}) -- 1 assistant locked onto the card",
                player_prefix(after.player(claimant).color),
                i,
                describe::achievement_desc(after_card.achievement_type),
            ));
        }
    }
}

fn building_lines(lines: &mut Vec<String>, before: &GameState, after: &GameState) {
    for (area, after_site) in &after.board_layout.building_sites {
        let before_site = before.board_layout.building_sites.get(area);
        for slot in 0..4 {
            let before_owner = before_site.and_then(|s| s.shophouses_built[slot]);
            if before_owner.is_none() {
                if let Some(owner) = after_site.shophouses_built[slot] {
                    lines.push(format!(
                        "{}: built shophouse at {}",
                        player_prefix(after.player(owner).color), describe::area_label(after, *area),
                    ));
                }
            }
        }
        let before_th = before_site.and_then(|s| s.trading_house_owner);
        if before_th.is_none() {
            if let Some(owner) = after_site.trading_house_owner {
                lines.push(format!(
                    "{}: built trading house at {}",
                    player_prefix(after.player(owner).color), describe::area_label(after, *area),
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
                // Country + one-line effect (item 5): single-sourced from the
                // card's own `country`/`effect` via `describe`, so a player
                // weighs a tech without cross-referencing a table.
                Some(card) => format!(
                    "{} [{}] (cost {}) — {}",
                    card.name,
                    describe::country_name(card.country),
                    card.industry_cost,
                    describe::tech_effect_text(card.effect),
                ),
                None => format!("T{:02}", tech_id.0),
            },
            None => "-".to_string(),
        };
        out.push_str(&format!("{}. +{}  {}\n", i + 1, surcharge, line));
    }
}

/// The one-letter code for a face-up 5-Power token (matching the
/// players-panel goods run and the wizard's token letters) and its `GoodKind`
/// for color. Magnitude is fixed per kind, so the letter alone identifies the
/// face.
///
/// Since the V3 board (T2) this is the WIZARD's vocabulary, not the board's:
/// a tile now spells its face out (`5-Power: 1 Import`, via
/// `board_render`'s own token), and the setup wizard's review summary is the
/// remaining caller — where a one-column code is what fits.
pub(crate) fn five_power_face(e: FivePowerEffect) -> (char, GoodKind) {
    match e {
        FivePowerEffect::ImportedGoods(_) => ('i', GoodKind::Imports),
        FivePowerEffect::Copper(_) => ('c', GoodKind::Copper),
        FivePowerEffect::Silk(_) => ('s', GoodKind::Silk),
        FivePowerEffect::Tea(_) => ('t', GoodKind::Tea),
        FivePowerEffect::Seafood(_) => ('f', GoodKind::Seafood),
        FivePowerEffect::Money(_) => ('y', GoodKind::Yen),
    }
}

/// A player's color name painted in that player's own color -- the leading
/// token of `players_panel`'s per-player block and of every per-player
/// `delta_summary` line, so the eye attributes a line to its player before
/// reading a word of it. Delta lines are ephemeral display text (never
/// cached into `Session::event_descriptions`, never serialized), so paint
/// here can't leak into logs or save files.
fn player_prefix(c: PlayerColor) -> String {
    paint(Tone::Player(c), color_name(c))
}

pub fn color_name(c: PlayerColor) -> &'static str {
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
    use crate::style::test_support::strip_ansi;

    fn test_state_2p() -> GameState {
        test_state_np(2)
    }

    /// The pre-bot default for `state`: every hand open, every seat human.
    fn seats_open(state: &GameState) -> SeatDisplay {
        SeatDisplay::open(state.players.len())
    }

    /// Every seat a bot (the badge fixtures' "worst case" for width).
    fn seats_all_bots(state: &GameState) -> SeatDisplay {
        let n = state.players.len();
        SeatDisplay::new(vec![true; n], vec![true; n])
    }

    /// A REAL generated board at `n` players (3P/4P carry a Canal tile; 2P
    /// does not), so topology-sensitive assertions run against the shipped
    /// geometry rather than an injected fixture.
    fn test_state_np(n: u8) -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(n), &rules, 1).unwrap();
        finalize_setup(pending, &vec![StartingOrderChoice::Keep(0); n as usize]).unwrap()
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

    /// S4 (pre-gate view batch): the header's current-player name is a
    /// who-is-where surface and paints in that player's color, alongside the
    /// phase text that already paints. The plain (disabled) header is
    /// unchanged -- so the existing `prompt_header_*` plain-text tests keep
    /// passing -- and `strip_ansi` round-trips the painted header to it.
    #[test]
    fn prompt_header_paints_the_current_player_name() {
        let state = test_state_2p(); // Red is the start/current player
        crate::style::force(true);
        let colored = prompt_header(&state, "Placement");
        let painted_name = paint(Tone::Player(PlayerColor::Red), "Red");
        crate::style::force(false);
        let plain = prompt_header(&state, "Placement");
        assert!(colored.contains(&painted_name), "current-player name must paint: {colored:?}");
        assert_eq!(strip_ansi(&colored), plain);
    }

    #[test]
    fn board_grid_has_one_cell_per_area_with_ids() {
        let state = test_state_2p();
        let g = board_grid(&state, &Rules::standard(), &[]);
        for id in state.board_layout.areas.keys() {
            assert!(g.contains(&format!("A{:02}", id.0)), "missing A{:02} in\n{g}", id.0);
        }
    }

    /// This function is an ADAPTER since the V3 cutover (T2): it reads state,
    /// builds the screen model, and delegates. So what it owes is that
    /// delegation — that it passes the state's own seat colours and adds
    /// nothing of its own. The board's GEOMETRY is pinned where the geometry
    /// now lives, in `board_render`, across all three player counts.
    #[test]
    fn board_grid_delegates_the_model_to_the_v3_renderer() {
        for n in [2u8, 3, 4] {
            let state = test_state_np(n);
            let rules = Rules::standard();
            let colors: Vec<PlayerColor> = state.players.iter().map(|p| p.color).collect();
            assert_eq!(
                board_grid(&state, &rules, &[]),
                crate::board_render::board_view(
                    &crate::screen_model::build_board(&state, &rules, &[]),
                    &colors,
                ),
                "{n}P: the adapter must add nothing to the renderer's output",
            );
        }
    }

    // ---- Third review I1: the numbered entry skeleton ----

    /// The skeleton draws the SAME staggered shape the engine plays, at every
    /// player count: `board_gen::row_lengths` row for row, widest row on top,
    /// each row indented half a cell pitch below the one above it. If this
    /// ever drifts from `row_lengths`, a live operator fills in a board shape
    /// the engine does not have.
    #[test]
    fn board_skeleton_shape_matches_row_lengths_at_every_player_count() {
        for players in [2u8, 3, 4] {
            let rows = row_lengths(players);
            let s = board_skeleton(players);
            let lines: Vec<&str> = s.lines().collect();
            assert_eq!(lines.len(), rows.len() * 3, "3 lines per row at {players}P:\n{s}");
            for (r, &len) in rows.iter().enumerate() {
                let top = lines[r * 3];
                assert_eq!(top.matches('┌').count(), len as usize, "{players}P row {r} cell count: {top:?}");
                let lead = top.chars().take_while(|c| *c == ' ').count();
                assert_eq!(lead, r * (SKEL_PITCH / 2), "{players}P row {r} brick offset: {top:?}");
            }
        }
    }

    /// The 2P skeleton, byte for byte. This one IS the operator-facing
    /// artifact -- it is what a tester holds against the physical board -- so
    /// unlike the rest of the renderer (content-asserted by house rule), its
    /// alignment is worth pinning: a half-cell offset that drifts by a column
    /// stops reading as a brick pattern, which is the whole point of the
    /// picture.
    #[test]
    fn board_skeleton_2p_is_the_exact_brick_pyramid() {
        assert_eq!(
            board_skeleton(2),
            concat!(
                "┌────┐┌────┐┌────┐┌────┐\n",
                "│  0 ││  1 ││  2 ││  3 │\n",
                "└────┘└────┘└────┘└────┘\n",
                "   ┌────┐┌────┐┌────┐\n",
                "   │  4 ││  5 ││  6 │\n",
                "   └────┘└────┘└────┘\n",
                "      ┌────┐┌────┐\n",
                "      │  7 ││  8 │\n",
                "      └────┘└────┘\n",
                "         ┌────┐\n",
                "         │  9 │\n",
                "         └────┘\n",
            )
        );
    }

    /// Cell indices run 0..n row-major, top row first -- the exact order
    /// `wizard::ask_tiles` asks for them and `ObservedBoard.tiles` is indexed
    /// in. The apex cell of a 4P board is 17, never 0.
    #[test]
    fn board_skeleton_numbers_cells_row_major_from_the_widest_row() {
        let s = board_skeleton(4);
        let numbers: Vec<usize> = s
            .lines()
            .filter(|l| l.contains('│'))
            .flat_map(|l| l.split('│').filter_map(|c: &str| c.trim().parse::<usize>().ok()).collect::<Vec<_>>())
            .collect();
        assert_eq!(numbers, (0..18).collect::<Vec<_>>(), "row-major 0..18:\n{s}");
        let first_row = s.lines().nth(1).unwrap();
        assert!(first_row.contains("  0 "), "cell 0 opens the WIDEST row:\n{s}");
        let last_row = s.lines().nth(10).unwrap();
        assert!(last_row.contains(" 17 "), "cell 17 closes the NARROWEST row:\n{s}");
    }

    /// Every line pads to ONE total width (the frame composer joins the board
    /// with the side panel line-by-line, so a ragged right edge would shear
    /// it), and the 2P board's width is the documented 120 columns — four
    /// 30-column V3 tiles on its widest row (T2; the 16-column tiles it
    /// replaced made it 72).
    #[test]
    fn board_grid_lines_share_one_width_and_2p_is_120_cols() {
        let state = test_state_2p();
        let g = board_grid(&state, &Rules::standard(), &[]);
        for glyph in ['─', '│', '┌', '┐', '└', '┘'] {
            assert!(g.contains(glyph), "missing {glyph} in:\n{g}");
        }
        let widths: HashSet<usize> = g.lines().map(|l| l.chars().count()).collect();
        assert_eq!(widths.len(), 1, "all board lines must share one width:\n{g}");
        assert_eq!(g.lines().next().unwrap().chars().count(), 120, "2P board width:\n{g}");
    }

    /// V3 tiles carry the FULL area name, not the abbreviation the 16-column
    /// tile had to squeeze into (`Employ Ag`, `Copper Mn`).
    #[test]
    fn board_grid_names_tiles_in_full() {
        let state = test_state_2p();
        let g = board_grid(&state, &Rules::standard(), &[]);
        // The 2P tile pool always includes these four (Chinatown/PortB/LabB/
        // Canal are removed at 2P).
        for name in ["CHURCH", "CUSTOMS", "BANK", "EMPLOYMENT AGENCY"] {
            assert!(g.contains(name), "missing tile name {name} in:\n{g}");
        }
        for abbreviation in ["Employ Ag", "Copper Mn", "Tea Plant", "Chrch", "Cstms"] {
            assert!(!g.contains(abbreviation), "squeezed name {abbreviation} must be gone:\n{g}");
        }
    }

    /// The Canal renders its transit-only shape — no 5-Power line, no
    /// presence line, no site strip, because it has none of those things.
    /// The renderer keys on the tile TYPE, so a flipped fixture suffices.
    #[test]
    fn board_grid_canal_tile_renders_its_transit_only_shape() {
        let mut state = test_state_2p();
        let tip = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.grid_position == (3, 0))
            .expect("2P pyramid has a (3,0) tip tile")
            .0;
        state.board_layout.areas.get_mut(&tip).unwrap().area_type = AreaType::Canal;
        let g = board_grid(&state, &Rules::standard(), &[]);
        assert!(g.contains("CANAL"), "the canal tile must be named:\n{g}");
        assert!(g.contains("transit only"), "canal transit note expected:\n{g}");
        assert!(g.contains("to the bank / crossing"), "canal toll expected:\n{g}");
    }

    /// The Station token's area is marked `[St]` on the tile title (it grants
    /// EVERY player +1 Power there, so it must be visible at a glance, not
    /// only in the `tracks` view).
    #[test]
    fn board_grid_marks_the_station_area() {
        let mut state = test_state_2p();
        let rules = Rules::standard();
        let area = *state.board_layout.areas.keys().next().unwrap();
        assert!(
            !board_grid(&state, &rules, &[]).contains("[St]"),
            "unplaced Station must not mark anything"
        );
        state.station_location = Some(area);
        assert!(board_grid(&state, &rules, &[]).contains("[St]"), "placed Station must mark its tile");
    }

    /// The legend keys what the V3 tiles actually draw — the marks and the
    /// site-slot tokens — and adds the canal crossing rule ONLY on a board
    /// that has a Canal (2P boards never do).
    #[test]
    fn board_legend_keys_the_tile_marks_and_names_the_canal_only_when_present() {
        let mut state = test_state_2p();
        let rules = Rules::standard();
        let without = board_legend(&state, &rules);
        assert!(without.contains("site slots"), "{without}");
        assert!(without.contains("[St]"), "{without}");
        assert!(
            !without.contains("5-Power tokens (face-up)"),
            "the letter key described the 16-column tile; V3 spells the face out: {without}",
        );
        assert!(!without.contains("Canal"), "no canal on this board: {without}");

        let tip = *state.board_layout.areas.keys().next().unwrap();
        state.board_layout.areas.get_mut(&tip).unwrap().area_type = AreaType::Canal;
        let with = board_legend(&state, &rules);
        assert!(with.contains("Canal"), "{with}");
        assert!(
            with.contains("¥1 to the bank"),
            "the toll and its recipient (bank) belong in the legend: {with}",
        );
    }

    #[test]
    fn five_power_face_maps_each_token_to_its_good_letter() {
        assert_eq!(five_power_face(FivePowerEffect::ImportedGoods(1)), ('i', GoodKind::Imports));
        assert_eq!(five_power_face(FivePowerEffect::Copper(2)), ('c', GoodKind::Copper));
        assert_eq!(five_power_face(FivePowerEffect::Silk(2)), ('s', GoodKind::Silk));
        assert_eq!(five_power_face(FivePowerEffect::Tea(3)), ('t', GoodKind::Tea));
        assert_eq!(five_power_face(FivePowerEffect::Seafood(3)), ('f', GoodKind::Seafood));
        assert_eq!(five_power_face(FivePowerEffect::Money(3)), ('y', GoodKind::Yen));
    }

    /// The board shows a face-up token's FACE (rulebook p.3 step 5 — tokens
    /// are face-up), and since V3 it shows it in WORDS rather than as the
    /// one-letter code the 16-column tile had room for. A claimed slot says
    /// `taken` rather than going blank, which the old tile could not
    /// distinguish from a tile that never had one.
    ///
    /// The retired half of this test asserted no `*` appeared anywhere on the
    /// board — the 2026-07-21 fix that replaced a boolean `*` token marker
    /// with its face. V3 cannot regress that (a face is words, not a mark),
    /// and `*` now MEANS something on a tile: an any-good site space.
    #[test]
    fn board_tiles_spell_out_the_five_power_face_and_say_when_it_is_taken() {
        let mut state = test_state_2p();
        let rules = Rules::standard();
        let area = *state.board_layout.areas.keys().next().unwrap();
        state.board_layout.five_power_tokens.insert(area, Some(FivePowerEffect::Copper(2)));
        let g = board_grid(&state, &rules, &[]);
        assert!(g.contains("5-Power: 2 Copper"), "a Copper token spells its face: {g}");

        state.board_layout.five_power_tokens.insert(area, None);
        let claimed = board_grid(&state, &rules, &[]);
        assert!(claimed.contains("5-Power: taken"), "a claimed token says so: {claimed}");
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

    /// Labs country + effect (pre-gate view batch item 5): each populated lab
    /// slot line shows the tech's country (`[Country]`) and its one-line
    /// effect (`describe::tech_effect_text`), single-sourced from the card
    /// data -- so a player can weigh a tech without cross-referencing a table.
    #[test]
    fn labs_view_shows_country_and_effect_per_card() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let text = labs_view(&state, &rules);
        let mut checked = 0;
        for slot in state.lab_state.lab_a.slots.iter().flatten() {
            let card = rules.techs.get(*slot).expect("a lab card is in the registry");
            assert!(
                text.contains(&format!("[{}]", describe::country_name(card.country))),
                "want country of {} in\n{text}",
                card.name
            );
            assert!(text.contains(describe::tech_effect_text(card.effect)), "want effect of {} in\n{text}", card.name);
            checked += 1;
        }
        assert!(checked > 0, "fixture must populate at least one lab slot:\n{text}");
    }

    /// F2 (Codex adversarial review of PR #4) lengthened five tech effect
    /// one-liners to carry the qualifiers that change a purchase decision,
    /// which lengthens the `labs` lines that print them. §2 of the guide
    /// promises a 140-column terminal clears every screen, so no lab line
    /// may exceed that. Measured over EVERY card in the registry (each
    /// slotted into a real lab board and run through the real `labs_view`,
    /// never a copy of its format string), so a future wording edit that
    /// would wrap on a tester's terminal fails here.
    #[test]
    fn labs_view_lines_fit_the_documented_terminal_width() {
        let mut state = test_state_2p();
        let rules = Rules::standard();
        let mut worst = (0usize, String::new());
        for card in rules.techs.iter() {
            state.lab_state.lab_a.slots[0] = Some(card.id);
            for line in labs_view(&state, &rules).lines() {
                let w = crate::screen::visible_width(line);
                if w > worst.0 {
                    worst = (w, line.to_string());
                }
            }
        }
        assert!(worst.0 <= 140, "a labs line is {} columns wide (140 max): {}", worst.0, worst.1);
    }

    /// Item 7 (third review), carried over the T3 cutover: every panel labels
    /// each player with its SEAT number -- the `reveal <seat>` key, which used
    /// to appear nowhere on the game screen (learnable only from the wizard
    /// transcript and the players ordering). The full `players` view spells it
    /// out; the dashboard carries it as a chrome annotation at the right of the
    /// player's header line.
    ///
    /// The compact/one-line halves of this test moved with the panels they
    /// measured: `compact_panel` and `compact_panel_oneline` are gone, and the
    /// property they pinned is pinned here against [`dashboard`] instead.
    #[test]
    fn panels_show_the_seat_number_for_the_reveal_key() {
        let state = test_state_2p(); // Red = seat 0 (current), Blue = seat 1
        let rules = Rules::standard();

        let full = players_panel(&state, &rules, &seats_open(&state));
        assert!(full.contains("(seat 0)"), "players view labels seat 0: {full}");
        assert!(full.contains("(seat 1)"), "players view labels seat 1: {full}");

        let dash = dashboard(&state, &rules, &seats_open(&state), PlayerId::new(0));
        let red = dash.lines().find(|l| l.contains("RED")).expect("a RED line");
        let blue = dash.lines().find(|l| l.contains("BLUE")).expect("a BLUE line");
        assert!(red.contains("seat 0"), "dashboard: current player RED is seat 0: {red}");
        assert!(blue.contains("seat 1"), "dashboard: BLUE is seat 1: {blue}");
    }

    /// Locked pawns (goals/church/customs) surface as a `locked N` tally so the
    /// panel's numbers still reconcile to the 20-pawn total. Re-pointed from
    /// `compact_panel` to its replacement; the assertion is unchanged.
    #[test]
    fn the_dashboard_tallies_locked_assistants() {
        let mut state = test_state_2p();
        let rules = Rules::standard();
        state.players[0].assistants.on_church = 2;
        state.players[0].assistants.on_achievements = 1;
        let text = dashboard(&state, &rules, &seats_open(&state), PlayerId::new(0));
        assert!(text.contains("3 locked"), "{text}");
    }

    /// The dashboard is privacy-safe BY CONSTRUCTION at every seat but the
    /// viewing one: a player's hand appears as a COUNT on their President line
    /// and card identities live only in the hand panel, which empties itself
    /// when masked. Re-pointed from `compact_panel`'s equivalent assertions.
    #[test]
    fn the_dashboard_never_prints_another_seats_cards() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let seats = SeatDisplay::new(vec![false, true], vec![false, false]);
        let text = dashboard(&state, &rules, &seats, PlayerId::new(0));
        assert!(text.contains("1 order in hand"), "the count still shows: {text}");
        assert!(text.contains("(hidden)"), "a masked seat says so: {text}");
        // Seat 1's kept starting order must not appear anywhere on the panel.
        let hidden_card = state.players[1].order_cards_in_hand[0];
        let printed = rules.orders.get(hidden_card).expect("a known card");
        assert!(
            !text.contains(&describe::country_name(printed.country).to_uppercase()),
            "seat 1's card country leaked into the dashboard: {text}",
        );
    }

    /// The dashboard paints after layout (strip-equality pin). Re-pointed from
    /// the two `colored_compact_panel*` tests, which measured the same property
    /// on the two retired panels.
    #[test]
    fn a_painted_dashboard_strips_back_to_plain() {
        let state = test_state_2p();
        let rules = Rules::standard();
        crate::style::force(true);
        let colored = dashboard(&state, &rules, &seats_open(&state), PlayerId::new(0));
        crate::style::force(false);
        let plain = dashboard(&state, &rules, &seats_open(&state), PlayerId::new(0));
        assert_ne!(colored, plain, "forced mode must actually paint something");
        assert_eq!(strip_ansi(&colored), plain);
    }

    #[test]
    fn players_panel_lists_each_player_by_color_and_money() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let v = players_panel(&state, &rules, &seats_open(&state));
        assert!(v.contains("Red"), "{v}");
        assert!(v.contains("Blue"), "{v}");
        assert!(v.contains("3 yen"), "{v}");
        assert!(v.contains("4 yen"), "{v}");
    }

    /// Task 6 privacy: with `hide_hands` the full players view masks hand
    /// CONTENTS (the kept starting order lives there) but keeps the count,
    /// so a live-table laptop can be passed around without leaking.
    #[test]
    fn players_panel_masks_hand_contents_when_hidden() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let open = players_panel(&state, &rules, &seats_open(&state));
        assert!(open.contains("hand: O"), "open mode shows codes: {open}");
        let masked = players_panel(&state, &rules, &SeatDisplay::hidden(state.players.len()));
        assert!(!masked.contains("hand: O"), "masked mode must hide codes: {masked}");
        assert!(masked.contains("1 card (hidden"), "count survives masking: {masked}");
    }

    /// T12: the mask is PER SEAT, so one seat's hand can be open while
    /// another's is not -- the mixed human/bot table the sub-phase exists for.
    #[test]
    fn players_panel_masks_one_seat_while_another_stays_open() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let mixed = players_panel(&state, &rules, &SeatDisplay::new(vec![false, true], vec![false, true]));
        let seat0 = mixed.split("(seat 1)").next().expect("seat 0 section");
        let seat1 = &mixed[mixed.find("(seat 1)").expect("seat 1 section")..];
        assert!(seat0.contains("hand: O"), "seat 0 stays open: {seat0}");
        assert!(seat1.contains("hidden"), "seat 1 is masked: {seat1}");
        assert!(seat1.contains("[bot]"), "and badged: {seat1}");
        assert!(!seat0.contains("[bot]"), "the human seat is not badged: {seat0}");
    }

    /// T12 (SHOULD item), re-pointed at the T3 cutover: the bot badge is
    /// WIDTH-NEUTRAL in the dashboard -- it rides the end of a player's name
    /// label, which is padded to a fixed column ([`panel_render`]'s `NAME_COL`,
    /// sized off the longest badged header the line can carry). This is what
    /// keeps guide §2's published frame widths true of a game with bots in it,
    /// and keeps `guide_sync`'s frame pins -- which measure a bot-free `Repl` --
    /// from having to be re-derived every time the badge changes.
    ///
    /// It used to measure the two retired panels, whose slack it stated in
    /// columns (23 of 40; 99 of a 108-column 4P board -- a figure T2's V3 board
    /// had already made stale at 180). Measured rather than stated now: the
    /// widest badged panel line must equal the widest unbadged one, at every
    /// player count, with every seat badged.
    #[test]
    fn bot_badge_is_width_neutral_in_the_dashboard() {
        let rules = Rules::standard();
        for n in [2u8, 3, 4] {
            let state = test_state_np(n);
            let widest = |s: &str| s.lines().map(|l| l.chars().count()).max().unwrap_or(0);

            let plain = dashboard(&state, &rules, &seats_open(&state), PlayerId::new(0));
            let badged = dashboard(&state, &rules, &seats_all_bots(&state), PlayerId::new(0));
            assert!(badged.contains("[bot]"), "the badge must actually render at {n}P: {badged}");
            assert_eq!(
                widest(&plain),
                widest(&badged),
                "the badge widened the {n}P dashboard:\n{badged}"
            );
        }
    }

    /// The dashboard and the `players` view mark a bot seat with the SAME
    /// string — the T3 review's note, since the two used to hold separate
    /// copies of it and agreed only by coincidence.
    ///
    /// Both halves are asserted against [`panel_render::BOT_BADGE`] rather than
    /// against a literal, so the pin follows a deliberate change to the marker
    /// instead of having to be re-typed alongside it; what it refuses is the two
    /// surfaces DIVERGING. The first assert is anti-vacuity — a badge that
    /// stopped rendering on either surface would satisfy any comparison between
    /// them — and the second closes the substring hole the first leaves open,
    /// since a surface that went back to spelling `" [bot 1]"` inline still
    /// CONTAINS `" [bot"`. Every `[bot` a surface draws has to be the whole
    /// badge and nothing else.
    #[test]
    fn the_bot_badge_is_one_string_on_every_surface() {
        let rules = Rules::standard();
        let state = test_state_np(2);
        let seats = seats_all_bots(&state);

        let dash = dashboard(&state, &rules, &seats, PlayerId::new(0));
        let players = players_panel(&state, &rules, &seats);
        let stem = panel_render::BOT_BADGE.trim_end_matches(']');
        for (surface, text) in [("the dashboard", &dash), ("the `players` view", &players)] {
            assert!(
                text.contains(panel_render::BOT_BADGE),
                "{surface} must draw the badge {:?}:\n{text}",
                panel_render::BOT_BADGE,
            );
            for (i, _) in text.match_indices(stem) {
                assert!(
                    text[i..].starts_with(panel_render::BOT_BADGE),
                    "{surface} draws a marker that only LOOKS like the badge at byte {i}; \
                     every {stem:?} must be exactly {:?}:\n{text}",
                    panel_render::BOT_BADGE,
                );
            }
        }
    }

    /// Owner scope item 10 (sufficiency): the shared mercenary SUPPLY piles
    /// are public table state and must be readable -- `tracks` (the
    /// board-level-shared-state view, same precedent as the Station line)
    /// shows one count per country.
    #[test]
    fn tracks_view_shows_the_mercenary_supply() {
        let mut state = test_state_2p();
        state.mercenary_supply.french = 9; // sentinel: proves counts are live
        let v = tracks_view(&state);
        let line = v.lines().find(|l| l.contains("Mercenary supply")).expect("supply line present");
        let s = &state.mercenary_supply;
        for (code, n) in
            [("GB", s.english), ("US", s.american), ("FR", s.french), ("DE", s.german), ("NL", s.dutch)]
        {
            assert!(line.contains(&format!("{code} {n}")), "want {code} {n} in: {line}");
        }
    }

    /// Finding D1, redesigned display: the 2P dummy is a NAMED occupant --
    /// the `D` mark stays in the occupant column, and a legend line explains
    /// what it is and that it competes in the majority bonus. No dummy on
    /// the board (a 3P/4P game, or an injected clean track) = no legend.
    #[test]
    fn tracks_view_names_the_dummy_in_a_legend() {
        let state = test_state_2p();
        let v = tracks_view(&state);
        assert!(v.contains("D = Dummy"), "{v}");
        assert!(v.contains("majorit"), "the legend must state it competes in majorities: {v}");

        let mut clean = test_state_2p();
        clean.church_track.slots = Default::default();
        clean.customs_track.slots = Default::default();
        let v2 = tracks_view(&clean);
        assert!(!v2.contains("D = Dummy"), "no dummy, no legend: {v2}");
    }

    /// Task 6 restyle: the state views share one design language -- a
    /// `─ Title ─...` rule header opens each of them. This covers the six views
    /// that live HERE as free functions; the four that live in repl.rs
    /// (`score`/`log`/`final_score`/`hand`) are held to the same header by
    /// `repl::tests::all_state_views_share_the_rule_header_style`, which drives
    /// every `VIEW_MENU` row plus `final_score` (item 4, third review -- this
    /// test alone structurally could not see those four).
    #[test]
    fn views_share_the_rule_header_style() {
        let state = test_state_2p();
        let rules = Rules::standard();
        for (name, text) in [
            ("ports", ports_view(&state, &rules)),
            ("labs", labs_view(&state, &rules)),
            ("tracks", tracks_view(&state)),
            ("sites", sites_view(&state, &rules)),
            ("goals", achievements_view(&state, &rules)),
            ("players", players_panel(&state, &rules, &seats_open(&state))),
        ] {
            assert!(text.starts_with("─ "), "view '{name}' must open with a rule header: {text}");
        }
    }

    /// Task 6 full color: the prompt header's goods/yen/VP tokens paint
    /// (strip-equality keeps the plain text authoritative).
    #[test]
    fn prompt_header_paints_goods_and_yen() {
        let state = test_state_2p();
        crate::style::force(true);
        let colored = prompt_header(&state, "Placement");
        crate::style::force(false);
        let plain = prompt_header(&state, "Placement");
        assert_eq!(strip_ansi(&colored), plain);
        // The goods run itself must carry paint, not just the earlier
        // player/phase tokens: escapes must appear AFTER the first goods
        // token (its own close, and the later goods' opens).
        let goods_start = colored.find("c1").expect("goods token present");
        assert!(colored[goods_start..].contains('\u{1b}'), "goods tokens must paint: {colored:?}");
    }

    /// Task 11 (T2.1): the panel must show assistant/shophouse/trading-house
    /// token counts by zone (hand/board/warehouse, built) plus the trading
    /// house's next Employment-Agency take cost, appended after the existing
    /// `tokens:` line -- the brief's exact line shapes.
    #[test]
    fn players_panel_shows_the_full_economy() {
        let mut state = test_state_2p();
        let rules = Rules::standard();
        let area = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type != AreaType::Canal)
            .expect("a non-Canal area exists")
            .0;
        state.players[0].assistants.on_board.insert(area, 3);
        state.players[0].assistants.in_hand -= 3;
        state.players[0].shophouses.built.insert(area, 0);
        state.players[0].shophouses.in_warehouse -= 1;
        state.players[0].trading_houses.in_hand = 1;
        state.players[0].trading_houses.in_warehouse -= 1;

        let text = players_panel(&state, &rules, &seats_open(&state));
        assert!(text.contains("assistants: hand"), "{text}");
        assert!(text.contains("shophouses: hand"), "{text}");
        assert!(text.contains("trading:"), "{text}");
        assert!(text.contains("next take"), "{text}");
        // Spot-check real numbers from the fixture (Red is players[0]).
        assert!(text.contains("  assistants: hand 5  board 3  warehouse 12"), "{text}");
        assert!(text.contains("  shophouses: hand 2  warehouse 5  built 1"), "{text}");
        assert!(text.contains("  trading:    hand 1  warehouse 3  built 0  (next take 5 yen)"), "{text}");
    }

    /// Task 12 (T2.2a): `site_compact_line` exists in core with zero
    /// non-test callers before this task -- `sites_view` wires it up, one
    /// line per area holding a `BuildingSite`.
    #[test]
    fn sites_view_renders_a_compact_line_per_sited_area() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let text = sites_view(&state, &rules);
        assert!(text.contains('S'), "{text}");
        assert!(text.lines().count() >= 1);
    }

    /// Each line is prefixed by the area label and suffixed by the same
    /// `sh .. | th ..` build/ownership marks the `board` grid shows, so a
    /// build recorded on the site is visible from `sites` too.
    #[test]
    fn sites_view_labels_the_area_and_shows_build_marks() {
        let mut state = test_state_2p();
        let rules = Rules::standard();
        let area = *state.board_layout.building_sites.keys().next().expect("a building site exists");
        state.board_layout.building_sites.get_mut(&area).unwrap().shophouses_built[0] = Some(PlayerId::new(0));

        let text = sites_view(&state, &rules);
        let label = describe::area_label(&state, area);
        let line = text.lines().find(|l| l.starts_with(&label)).unwrap_or_else(|| panic!("missing {label} in {text}"));
        assert!(line.contains("sh R···"), "{line}"); // Red is players[0]; positional marks
    }

    /// Board connectivity (pre-gate view batch item 7): each `sites` line
    /// gains an `adj:` suffix listing its adjacent area ids ascending, sourced
    /// from `board_layout.adjacency` (never re-derived) so a state inventory
    /// reads as a navigable board. On the 2P board raw adjacency IS the
    /// free-neighbor set (no Canal tile exists, hence no crossings), so
    /// comparing against it directly is sound here; the three-category
    /// separation the 3P/4P boards need is pinned by
    /// `sites_view_separates_free_neighbors_from_crossings_and_the_canal_tile`.
    #[test]
    fn sites_view_lists_adjacent_area_ids_ascending() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let text = sites_view(&state, &rules);

        let (&area, _) = state.board_layout.building_sites.iter().next().expect("a building site exists");
        let mut ids: Vec<u8> = state.board_layout.adjacent_areas(area).iter().map(|a| a.0).collect();
        ids.sort_unstable();
        ids.dedup();
        let expected = format!("adj: {}", ids.iter().map(|n| format!("A{:02}", n)).collect::<Vec<_>>().join(", "));

        let label = describe::area_label(&state, area);
        let line = text.lines().find(|l| l.starts_with(&label)).unwrap_or_else(|| panic!("{label} line in\n{text}"));
        assert!(line.contains(&expected), "want '{expected}' in '{line}'");
    }

    /// Board connectivity item 7: a canal-crossing pair shows as `canal-><id>`
    /// on BOTH members' lines (crossings are bidirectional), sourced from
    /// `board_layout.canal_crossings`. Injected directly here -- the renderer
    /// only reads the pair list, so geometry is irrelevant to this display.
    #[test]
    fn sites_view_shows_canal_crossings_bidirectionally() {
        let mut state = test_state_2p();
        let rules = Rules::standard();
        let sited: Vec<AreaId> = state.board_layout.building_sites.keys().copied().collect();
        let (area, other) = (sited[0], sited[1]);
        state.board_layout.canal_crossings.push((area, other));

        let text = sites_view(&state, &rules);
        let label = describe::area_label(&state, area);
        let line = text.lines().find(|l| l.starts_with(&label)).unwrap_or_else(|| panic!("{label} line in\n{text}"));
        assert!(line.contains(&format!("canal->A{:02}", other.0)), "want forward crossing in '{line}'");

        let other_label = describe::area_label(&state, other);
        let other_line =
            text.lines().find(|l| l.starts_with(&other_label)).unwrap_or_else(|| panic!("{other_label} line"));
        assert!(other_line.contains(&format!("canal->A{:02}", area.0)), "want reverse crossing in '{other_line}'");
    }

    /// The two connectivity categories a `sites` line prints, parsed back
    /// off the rendered text: the `adj:` ids and the `canal->` ids.
    fn connectivity_of(line: &str) -> (Vec<u8>, Vec<u8>) {
        let tail = line.split("adj: ").nth(1).unwrap_or_else(|| panic!("no adj: suffix in '{line}'"));
        let mut parts = tail.split("  canal->");
        let adj_text = parts.next().unwrap_or("").trim();
        let parse = |t: &str| {
            t.trim().trim_start_matches('A').parse::<u8>().unwrap_or_else(|_| panic!("area id in '{line}'"))
        };
        let adj: Vec<u8> =
            if adj_text == "-" { Vec::new() } else { adj_text.split(", ").map(parse).collect() };
        (adj, parts.map(parse).collect())
    }

    /// F1 (Codex adversarial review of PR #4): `adj:` must be the
    /// President's ORDINARY free-neighbor list and nothing else. Since the
    /// staggered-topology work `board_layout.adjacency` carries three
    /// different relationships at once -- brick edges, edges to the
    /// unplaceable Canal tile, and the derived canal-crossing edges -- so
    /// printing it raw showed a TOLLED crossing as if it were free (and
    /// again as `canal->`), and offered the Canal itself, which can never
    /// hold a President and is never a legal Telegram bonus target. The
    /// three categories are separated by SUBTRACTION off the authoritative
    /// layout (geometry is never re-derived here): `adj:` = adjacency minus
    /// crossing partners minus Canal; `canal->` = exactly the crossing
    /// partners. Run on REAL generated 3P and 4P boards -- the ones that
    /// carry a Canal.
    #[test]
    fn sites_view_separates_free_neighbors_from_crossings_and_the_canal_tile() {
        for players in [3u8, 4] {
            let state = test_state_np(players);
            let rules = Rules::standard();
            let canal: Vec<AreaId> = state
                .board_layout
                .areas
                .iter()
                .filter(|(_, cfg)| cfg.area_type == AreaType::Canal)
                .map(|(&id, _)| id)
                .collect();
            assert!(!canal.is_empty(), "the {players}P board must carry a Canal tile");
            let text = sites_view(&state, &rules);

            let mut shores_checked = 0usize;
            let mut subtraction_was_load_bearing = false;
            for &area in state.board_layout.building_sites.keys() {
                let label = describe::area_label(&state, area);
                let line = text
                    .lines()
                    .find(|l| l.starts_with(&label))
                    .unwrap_or_else(|| panic!("{label} line in\n{text}"));
                let (adj, crossings) = connectivity_of(line);

                let mut want_crossings: Vec<u8> = state
                    .board_layout
                    .canal_crossings
                    .iter()
                    .filter_map(|&(a, b)| match () {
                        _ if a == area => Some(b.0),
                        _ if b == area => Some(a.0),
                        _ => None,
                    })
                    .collect();
                want_crossings.sort_unstable();
                want_crossings.dedup();

                let raw: Vec<u8> =
                    state.board_layout.adjacent_areas(area).iter().map(|a| a.0).collect();
                let mut want_adj: Vec<u8> = raw
                    .iter()
                    .copied()
                    .filter(|n| !want_crossings.contains(n) && !canal.iter().any(|c| c.0 == *n))
                    .collect();
                want_adj.sort_unstable();
                want_adj.dedup();
                if want_adj.len() != raw.len() {
                    subtraction_was_load_bearing = true;
                }

                assert_eq!(crossings, want_crossings, "canal-> list on '{line}'");
                assert_eq!(adj, want_adj, "adj: list on '{line}'");
                for c in &canal {
                    assert!(!adj.contains(&c.0), "the Canal tile A{:02} must never be an adj: entry: {line}", c.0);
                }
                for c in &want_crossings {
                    assert!(!adj.contains(c), "tolled crossing A{c:02} must not read as free adjacency: {line}");
                }
                if !want_crossings.is_empty() {
                    shores_checked += 1;
                }
            }
            assert!(shores_checked > 0, "the {players}P board must have shore areas with crossings");
            assert!(
                subtraction_was_load_bearing,
                "the {players}P board must actually exercise the subtraction (raw adjacency \
                 containing a Canal id or a crossing partner)"
            );
        }
    }

    #[test]
    fn tracks_view_shows_church_and_customs_with_2p_block() {
        let state = test_state_2p();
        let v = tracks_view(&state);
        assert!(v.contains("Church"), "{v}");
        assert!(v.contains("Customs"), "{v}");
        // Finding D1: the 2P dummy holds Church faith-7 and Customs space-4,
        // shown as a named `D` occupant (not the old bare `X` void).
        let church7 = v.lines().find(|l| l.contains("faith  7:")).expect("faith-7 line");
        assert!(church7.contains("faith  7: D"), "{church7}");
        let customs4 = v.lines().find(|l| l.contains("space 4 ")).expect("space-4 line");
        assert!(customs4.contains(": D "), "{customs4}");
    }

    /// I4 (external review 2026-07-20): the Station token grants EVERY
    /// player +1 Power in its area, but was rendered by no view at all, so a
    /// live-tracking operator could not read it back to reconcile against the
    /// physical board. Both states must be legible -- unplaced and placed.
    #[test]
    fn tracks_view_shows_the_station_token_location() {
        let mut state = test_state_2p();
        assert_eq!(state.station_location, None, "fixture starts with the Station unplaced");

        let unplaced = tracks_view(&state);
        let station_line = unplaced
            .lines()
            .skip_while(|l| !l.starts_with("Station:"))
            .nth(1)
            .expect("a Station section with a value line");
        assert!(station_line.contains("not placed"), "{unplaced}");

        let area = *state.board_layout.areas.keys().next().expect("board has areas");
        state.station_location = Some(area);
        let placed = tracks_view(&state);
        assert!(
            placed.contains(&describe::area_label(&state, area)),
            "placed Station must name its area: {placed}"
        );
        assert!(!placed.contains("not placed"), "{placed}");
    }

    /// I4 companion: resolving `PendingChoice::StationPlace` must not produce
    /// an EMPTY delta window. `delta_summary` is a hand-maintained
    /// field-by-field diff and simply had no `station_location` arm, so the
    /// one move that places the token reported nothing at all.
    #[test]
    fn delta_summary_reports_a_station_placement() {
        let rules = Rules::standard();
        let before = test_state_2p();
        let mut after = before.clone();
        let area = *before.board_layout.areas.keys().next().expect("board has areas");
        after.station_location = Some(area);

        let lines = delta_summary(&before, &after, &rules);
        assert!(
            lines.iter().any(|l| l.contains("Station token placed")),
            "placing the Station must be visible in the delta: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains(&describe::area_label(&after, area))),
            "the delta must name WHERE it landed: {lines:?}"
        );
    }

    /// Finding D1 (rulebook p.9 NOTE, Jeremy's edition -- not the bundled PDF; see guidingpath's 2026-07-21 DL rows): in 2p games the setup dummy holds
    /// Church faith-7, so `tracks_view` renders it as a named `D` occupant
    /// (which competes in the majority bonus), not the bare `X` it used to be.
    #[test]
    fn tracks_view_marks_church_faith_7_dummy_at_2p() {
        let state = test_state_2p();
        let v = tracks_view(&state);
        // Computed via the same `{:2}` width the SUT formats with, so this
        // assertion doesn't hardcode a specific padding guess. The occupant
        // now sits directly after the colon (S5 appends the printed VP/rider
        // after it), so the marker is checked there rather than at line-end.
        let needle = format!("faith {:2}:", 7);
        let line = v.lines().find(|l| l.contains(&needle)).expect("faith-7 line present");
        assert!(line.contains(&format!("{needle} D")), "{line}");
    }

    /// S5 (pre-gate view batch): every track space line must show its PRINTED
    /// VP and rider icon(s), single-sourced from the engine's `ChurchTrack`/
    /// `CustomsTrack` tables -- the same tables `delta_summary` reads after a
    /// placement. Church carries a movement rider (`Nmove`); Customs a
    /// warehouse/yen rider (`Nwh`/`Nyen`, absent on spaces 5-7). Vocabulary
    /// matches `sites_view` (`Nvp`, `Nmove`, `Nwh`, `Nyen`).
    #[test]
    fn tracks_view_shows_printed_vp_and_riders() {
        let state = test_state_2p();
        let v = tracks_view(&state);

        for faith in 4u8..=11 {
            let needle = format!("faith {:2}:", faith);
            let line = v.lines().find(|l| l.contains(&needle)).unwrap_or_else(|| panic!("faith {faith} line in\n{v}"));
            let expected = format!("{}vp {}move", ChurchTrack::vp_reward(faith), ChurchTrack::movement_rider(faith));
            assert!(line.contains(&expected), "faith {faith}: want '{expected}' in '{line}'");
        }

        for space in 1u8..=7 {
            let needle = format!("space {} (cost {}):", space, CustomsTrack::space_cost(space));
            let line = v.lines().find(|l| l.contains(&needle)).unwrap_or_else(|| panic!("space {space} line in\n{v}"));
            assert!(line.contains(&format!("{}vp", CustomsTrack::vp_reward(space))), "space {space} vp in '{line}'");
            let (wh, yen) = CustomsTrack::rider(space);
            if wh > 0 {
                assert!(line.contains(&format!("{wh}wh")), "space {space} wh in '{line}'");
            }
            if yen > 0 {
                assert!(line.contains(&format!("{yen}yen")), "space {space} yen in '{line}'");
            }
            if wh == 0 && yen == 0 {
                assert!(!line.contains("wh") && !line.contains("yen"), "space {space} must be riderless: '{line}'");
            }
        }
    }

    /// S4 (pre-gate view batch): `tracks_view` occupant color letters are a
    /// who-is-where surface and must paint in the player's color -- painted
    /// AFTER any layout, so the alignment invariant `strip_ansi(colored) ==
    /// plain` still holds (the players_panel/board_grid precedent). Occupants
    /// injected on both tracks so painting has something to do.
    #[test]
    fn colored_tracks_view_still_contains_the_plain_text() {
        let mut state = test_state_2p();
        state.church_track.slots[0] = Some(ChurchSlot { player: PlayerId::new(0), arrival_order: 0 });
        state.customs_track.slots[2] = Some(CustomsSlot { player: PlayerId::new(1), arrival_order: 0 });
        crate::style::force(true);
        let colored = tracks_view(&state);
        crate::style::force(false);
        let plain = tracks_view(&state);
        assert_ne!(colored, plain, "forced mode must actually paint something");
        assert_eq!(strip_ansi(&colored), plain);
    }

    /// Task 14 (T2.3) brief Step 1: one `G{i}` line per achievement card
    /// (`state.achievements` is always a fixed 3-card array -- one per
    /// category), and progress shown as `cur/need` somewhere in the text.
    #[test]
    fn achievements_view_lists_condition_claim_state_and_progress() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let text = achievements_view(&state, &rules);
        assert_eq!(text.lines().filter(|l| l.starts_with("G")).count(), 3, "{text}");
        assert!(text.contains("/"), "progress as cur/need expected: {text}");
    }

    /// Claim state must name the ACTUAL claimant and stay "open" for a
    /// still-unclaimed card -- the Step-1 test above only pins line COUNT
    /// and that SOME "/" appears, not the words themselves.
    #[test]
    fn achievements_view_shows_open_then_names_the_claimant_once_claimed() {
        let mut state = test_state_2p();
        let rules = Rules::standard();

        let before = achievements_view(&state, &rules);
        let g0_before = before.lines().find(|l| l.starts_with("G0")).expect("a G0 line exists");
        assert!(g0_before.contains("open"), "{g0_before}");

        // State-inject a claim directly onto the card (mirrors
        // achievements.rs's own claim tests) rather than driving
        // `claim_achievement` through a full legal-move sequence -- this
        // view only reads `claimed_by`, so a direct injection exercises
        // exactly the field it reads.
        state.achievements[0].claimed_by.push(PlayerId::new(0));
        let after = achievements_view(&state, &rules);
        let g0_after = after.lines().find(|l| l.starts_with("G0")).expect("a G0 line exists");
        assert!(!g0_after.contains("open"), "{g0_after}");
        assert!(g0_after.contains("claimed by Red"), "{g0_after}"); // Red is players[0]
    }

    /// The numbers shown are exactly what `achievement_progress` computes,
    /// not just "some digits with a slash" (the Step-1 test's `contains
    /// ("/")` check would pass even for a stub like "0/0" on every line) --
    /// checked for every card x player without depending on which 3
    /// achievements this seed happened to draw.
    #[test]
    fn achievements_view_progress_numbers_match_achievement_progress() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let text = achievements_view(&state, &rules);
        for card in &state.achievements {
            for p in &state.players {
                let (cur, need) = achievement_progress(&state, p.id, card.achievement_type);
                let expected = format!("{}: {}/{}", color_name(p.color), cur, need);
                assert!(text.contains(&expected), "missing '{expected}' in:\n{text}");
            }
        }
    }

    /// Goals show claim VP (pre-gate view batch item 6): each `G{i}` line must
    /// print the card's first/subsequent claim VP from the registry
    /// (`AchievementType::first_place_vp`/`subsequent_vp`), so claim value is
    /// visible before committing an assistant. Single-sourced -- computed from
    /// the same methods `AchievementCard::vp_for_player` scores with.
    #[test]
    fn achievements_view_shows_claim_vp_from_registry() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let text = achievements_view(&state, &rules);
        for (i, card) in state.achievements.iter().enumerate() {
            let g_line = text
                .lines()
                .find(|l| l.starts_with(&format!("G{i}:")))
                .unwrap_or_else(|| panic!("G{i} line in\n{text}"));
            let expected =
                format!("{}/{} VP", card.achievement_type.first_place_vp(), card.achievement_type.subsequent_vp());
            assert!(g_line.contains(&expected), "G{i}: want '{expected}' in '{g_line}'");
        }
    }

    /// S6 (pre-gate view batch): a player already in `claimed_by` gets an
    /// explicit ` (claimed)` marker on their progress line, so a maxed-out
    /// `Red: 7/7` under `claimed by Red` no longer reads as an invitation to
    /// claim again. A player who has NOT claimed keeps a bare `cur/need`.
    #[test]
    fn achievements_view_marks_a_claimant_progress_line() {
        let mut state = test_state_2p();
        let rules = Rules::standard();
        state.achievements[0].claimed_by.push(PlayerId::new(0)); // Red claims G0

        let text = achievements_view(&state, &rules);
        // The G0 block: from its `G0:` header up to (not including) `G1:`.
        let g0_block: Vec<&str> = text
            .lines()
            .skip_while(|l| !l.starts_with("G0:"))
            .skip(1)
            .take_while(|l| !l.starts_with("G1:"))
            .collect();
        let red_line = g0_block.iter().find(|l| l.trim_start().starts_with("Red:")).expect("a Red progress line");
        assert!(red_line.contains("(claimed)"), "claimant must be marked: {red_line}");
        let blue_line = g0_block.iter().find(|l| l.trim_start().starts_with("Blue:")).expect("a Blue progress line");
        assert!(!blue_line.contains("(claimed)"), "non-claimant must NOT be marked: {blue_line}");
    }

    /// S4 (pre-gate view batch): `achievements_view` color names -- both the
    /// `claimed by <names>` list and each per-player progress prefix -- are a
    /// who-is-where surface and paint in the player's color. The alignment
    /// invariant `strip_ansi(colored) == plain` still holds (there is no
    /// fixed-width layout here, but the equality is the paint-discipline pin).
    #[test]
    fn colored_achievements_view_still_contains_the_plain_text() {
        let mut state = test_state_2p();
        let rules = Rules::standard();
        state.achievements[0].claimed_by.push(PlayerId::new(0)); // paint the claimed-by list too
        crate::style::force(true);
        let colored = achievements_view(&state, &rules);
        crate::style::force(false);
        let plain = achievements_view(&state, &rules);
        assert_ne!(colored, plain, "forced mode must actually paint something");
        assert_eq!(strip_ansi(&colored), plain);
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

    /// R1 (render.rs deep audit, 2026-07-20): SPENDING a mercenary moves the
    /// token `available -> used`, leaving `available + used` unchanged --
    /// which is exactly what `token_lines` used to compare, so the delta was
    /// silent 100% of the time (802/456/492 spends per 300 games at 2P/3P/4P,
    /// none narrated). The operator owes the table a physical act here (flip
    /// the tile face-down); an empty delta reads as "nothing happened".
    #[test]
    fn delta_summary_reports_a_spent_mercenary() {
        let rules = Rules::standard();
        let mut before = test_state_2p();
        before.players[0].mercenaries.available.insert(Country::English, 1);
        before.players[0].mercenaries.used.insert(Country::English, 0);

        let mut after = before.clone();
        after.players[0].mercenaries.available.insert(Country::English, 0);
        after.players[0].mercenaries.used.insert(Country::English, 1);
        // The sum is deliberately invariant -- that is the whole bug.
        assert_eq!(
            before.players[0].mercenaries.total(),
            after.players[0].mercenaries.total(),
            "fixture must hold total() constant, else it isn't testing the real failure"
        );

        let lines = delta_summary(&before, &after, &rules);
        assert!(
            lines.iter().any(|l| l.contains("spent mercenary")),
            "spending a mercenary must be narrated: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("Britain")),
            "the line must name WHICH country's tile to flip: {lines:?}"
        );
    }

    /// Phase-2.9 per-slot enumeration fix: the engine now offers one
    /// `BuildShophouse` per EMPTY slot, so `shophouses_built` is no longer
    /// prefix-packed -- gaps are ordinary. `build_marks` must therefore be
    /// POSITIONAL: one mark per slot in printed order (owner color letter or
    /// `·`), so each built slot is attributed to the reward printed right
    /// beside it in `sites_view` (`[1vp 4vp - 2yen]  sh ·B·R`). This replaces
    /// the old `shophouse_slots_fill_prefix_packed_so_build_marks_stay_
    /// unambiguous` pin, whose premise the engine fix deliberately broke.
    #[test]
    fn build_marks_attribute_each_built_slot_to_its_owner_positionally() {
        let state = test_state_2p(); // players[0] = Red, players[1] = Blue
        let mut site = BuildingSite::new([1, 4, 0, 2], 8);
        site.shophouses_built = [None, Some(PlayerId::new(1)), None, Some(PlayerId::new(0))];

        // Slot 1 (printed reward "4vp") is Blue's, slot 3 ("2yen") is Red's;
        // slots 0 and 2 are open. A packed "sh BR" would attribute Blue to
        // "1vp" and Red to "4vp" -- rewards neither of them took.
        assert_eq!(build_marks(&state, &site), "sh ·B·R | th ·");
    }

    /// Positional-marks companion: an untouched site shows one `-` per slot
    /// (never a collapsed single dash, which would break the slot<->reward
    /// column correspondence), and the trading-house mark still carries the
    /// owner's color letter independently of the shophouse slots.
    #[test]
    fn build_marks_show_a_dot_per_empty_slot_and_the_th_owner() {
        let state = test_state_2p();
        let mut site = BuildingSite::new([1, 4, 0, 2], 8);
        assert_eq!(build_marks(&state, &site), "sh ···· | th ·");

        site.trading_house_owner = Some(PlayerId::new(0));
        assert_eq!(build_marks(&state, &site), "sh ···· | th R");
    }

    /// S1 (render.rs deep audit, 2026-07-20): the track-placement reservation
    /// read `president_location`, but the engine resolves the acting area via
    /// `resolve_acting_area`, which returns `mercenary_target_area` during a
    /// mercenary sub-turn -- its own doc says "the target area, not the
    /// president's". This was the ONE place `render.rs` recomputed an engine
    /// quantity rather than delegating, and it recomputed it wrong.
    ///
    /// Unreached in 3,800 random games, but reachable at a table: a late-game
    /// player with an empty assistant hand spending a mercenary at the Church
    /// is ordinary play. Uniform-random play rarely combines those three --
    /// exactly the blind-spot class I6 documented, so "the fuzzer never hit
    /// it" is not evidence of unreachability here.
    #[test]
    fn track_placement_in_a_mercenary_sub_turn_reserves_the_target_not_the_president() {
        let rules = Rules::standard();
        let mut before = test_state_2p();
        let mut areas = before.board_layout.areas.keys().copied();
        let target = areas.next().expect("board has areas");
        let president_area = areas.next().expect("board has a second area");

        {
            let p = &mut before.players[0];
            // Empty hand forces the Church placement to draw off the BOARD --
            // `take_church_or_customs_assistant` prefers hand, falls back to
            // the acting area.
            p.assistants.in_hand = 0;
            p.assistants.on_board.clear();
            p.assistants.on_board.insert(target, 1);
            p.assistants.on_board.insert(president_area, 1);
            p.president_location = PresidentLocation::OnBoard(president_area);
            p.church_positions.clear();
        }
        before.is_mercenary_action = true;
        before.mercenary_target_area = Some(target);
        assert_ne!(target, president_area, "fixture needs the two areas distinct to discriminate");

        // The mercenary Church action locks the TARGET area's pawn on the track.
        let mut after = before.clone();
        {
            let p = &mut after.players[0];
            p.assistants.on_board.remove(&target);
            p.assistants.on_church += 1;
            p.church_positions.push(6);
        }

        let lines = delta_summary(&before, &after, &rules);
        // Reserving the president's area instead of the target leaves the
        // target at -1 and the president's area at +1, which the pairing loop
        // renders as a move that never happened.
        assert!(
            !lines.iter().any(|l| l.contains("assistant moved from")),
            "no assistant moved between areas; reserving the wrong area fabricates one: {lines:?}"
        );
        assert!(
            !lines.iter().any(|l| l.contains("recovered from")),
            "the pawn went onto the church track, not back to hand: {lines:?}"
        );
    }

    /// R2 (render.rs deep audit, 2026-07-20): the panel printed 3 of
    /// `AssistantState`'s SIX zones -- and the three it printed are exactly
    /// the ones an operator would sum to 20 for a conservation check. Across
    /// 1,200 end-of-game states the sum was short in 861 (72%), worst gap 5,
    /// with nothing on screen explaining where the pawns went, so the
    /// operator hunts a pawn that isn't lost.
    #[test]
    fn players_panel_accounts_for_every_assistant_zone() {
        let rules = Rules::standard();
        let mut state = test_state_2p();
        {
            let p = &mut state.players[0];
            p.assistants.on_achievements = 1;
            p.assistants.on_church = 2;
            p.assistants.on_customs = 3;
        }
        let total = state.players[0].assistants.total();

        let panel = players_panel(&state, &rules, &seats_open(&state));
        let line = panel
            .lines()
            .find(|l| l.contains("assistants:"))
            .expect("panel has an assistants line");

        // The three locked zones must be visible somewhere on that line.
        assert!(line.contains("goals 1"), "achievement-locked pawns hidden: {line}");
        assert!(line.contains("church 2"), "church-locked pawns hidden: {line}");
        assert!(line.contains("customs 3"), "customs-locked pawns hidden: {line}");

        // And the printed numbers must actually reconcile to the true total,
        // which is the whole point of showing them.
        let sum: u32 = line
            .split_whitespace()
            .filter_map(|t| t.parse::<u32>().ok())
            .sum();
        assert_eq!(
            sum, total as u32,
            "the panel's assistant numbers must sum to the player's true pawn count: {line}"
        );
    }

    /// R2 companion: claiming an achievement moved a pawn `hand ->
    /// on_achievements` and printed only "+N VP". The physical act it
    /// requires -- moving an assistant onto the achievement card -- was never
    /// prompted, so the pawn stays in hand and the count runs 1 high for the
    /// rest of the game. 286/379/302 claims per 300 games, all silent.
    #[test]
    fn delta_summary_reports_an_achievement_claim() {
        let rules = Rules::standard();
        let before = test_state_2p();
        let mut after = before.clone();
        let claimer = after.players[0].id;
        after.players[0].assistants.in_hand -= 1;
        after.players[0].assistants.on_achievements += 1;
        after.achievements[0].claimed_by.push(claimer);

        let lines = delta_summary(&before, &after, &rules);
        assert!(
            lines.iter().any(|l| l.contains("claimed achievement")),
            "claiming an achievement must be narrated: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("assistant")),
            "the line must cue the pawn that moves onto the card: {lines:?}"
        );
    }

    /// R1 companion: `players_panel` printed `mercenaries.total()`
    /// (available + used) -- a number that cannot change when one is spent,
    /// and never the VP-relevant one. `scoring.rs`'s remainder awards 1 VP
    /// per UNUSED mercenary (`unused_count()` = available only), so the panel
    /// must show the unused split or the operator cannot predict their score.
    #[test]
    fn players_panel_shows_the_unused_mercenary_split() {
        let rules = Rules::standard();
        let mut state = test_state_2p();
        state.players[0].mercenaries.available.insert(Country::English, 1);
        state.players[0].mercenaries.used.insert(Country::French, 2);

        let panel = players_panel(&state, &rules, &seats_open(&state));
        let line = panel
            .lines()
            .find(|l| l.contains("mercenaries"))
            .expect("panel has a mercenaries line");
        // 3 held, 1 unused (= 1 VP at scoring), 2 already spent.
        assert!(line.contains('3'), "total still shown: {line}");
        assert!(line.contains("1 unused"), "unused split must be visible: {line}");
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

    /// Task 16 (T2.5): painting must happen AFTER fixed-width layout --
    /// ANSI escapes are zero-display-width bytes, so padding computed on an
    /// already-painted string breaks column alignment. Stripping the
    /// escapes back out must therefore reproduce the plain render exactly.
    #[test]
    fn colored_players_panel_still_contains_the_plain_text() {
        let state = test_state_2p();
        let rules = Rules::standard();
        crate::style::force(true);
        let colored = players_panel(&state, &rules, &seats_open(&state));
        crate::style::force(false);
        let plain = players_panel(&state, &rules, &seats_open(&state));
        assert_ne!(colored, plain, "forced mode must actually paint something");
        assert_eq!(strip_ansi(&colored), plain);
    }

    /// Same alignment invariant for the board grid, whose president and
    /// assistant marks are painted INSIDE fixed-width tile cells -- the exact
    /// spot where escape-inflated padding would shear the frame. Held here at
    /// the ADAPTER, over a real state, beside `board_render`'s own model-level
    /// pin: they meet at different seams and a regression could sit in either.
    #[test]
    fn colored_board_grid_stays_aligned_with_plain_layout() {
        let mut state = test_state_2p();
        let area = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type != AreaType::Canal)
            .expect("a non-Canal area exists")
            .0;
        state.players[0].president_location = PresidentLocation::OnBoard(area);
        state.players[1].president_location = PresidentLocation::OnBoard(area);
        state.players[0].assistants.in_hand -= 3;
        state.players[0].assistants.on_board.insert(area, 3);
        state.players[1].assistants.in_hand -= 1;
        state.players[1].assistants.on_board.insert(area, 1);

        let rules = Rules::standard();
        let plain = board_grid(&state, &rules, &[]);
        crate::style::force(true);
        let colored = board_grid(&state, &rules, &[]);
        crate::style::force(false);
        assert_ne!(colored, plain, "forced mode must actually paint something");
        assert_eq!(strip_ansi(&colored), plain);
    }

}
