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
use crate::describe::tech_name;
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
    let (panels, board, colors) = dashboard_inputs(state, rules, seats, seat);
    panel_render::dashboard(&panels, &board, &colors)
}

/// The composed frame's dashboard STACKED BELOW the board (Phase-3.5a T10), at
/// 3 and 4 players — [`dashboard`]'s twin over the same models and the same
/// adapter shape, differing only in which `panel_render` entry point draws
/// them.
///
/// `width` is the block the frame gives it, which [`crate::screen`] computes
/// from what is already on screen above it. It is a parameter rather than a
/// constant here because the answer is a fact about the composed frame — the
/// board's own width and the site-slot legend's — and this module cannot see a
/// frame.
pub fn dashboard_below(
    state: &GameState,
    rules: &Rules,
    seats: &SeatDisplay,
    seat: PlayerId,
    width: usize,
) -> String {
    let (panels, board, colors) = dashboard_inputs(state, rules, seats, seat);
    panel_render::dashboard_below(&panels, &board, &colors, width)
}

/// The three things both dashboard arrangements are drawn from. Shared rather
/// than built twice: the per-seat privacy and bot policy is the one input a
/// second copy could get wrong, and getting it wrong leaks a hand.
fn dashboard_inputs(
    state: &GameState,
    rules: &Rules,
    seats: &SeatDisplay,
    seat: PlayerId,
) -> (Vec<screen_model::Panel>, screen_model::BoardModel, Vec<PlayerColor>) {
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
    (panels, board, colors)
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
    let colors: Vec<PlayerColor> = state.players.iter().map(|p| p.color).collect();
    decision_render::page_count(&zone_rows(rows, &fees), &board, &colors)
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

// The `ports` and `labs` views RETIRED at T11 and moved to `view_render` as
// full-screen card surfaces, the way `tracks`/`sites`/`goals` moved at T6 —
// spec §0.5's whole roster now renders card-like, from typed models, over the
// same `card_render` emitter every other card on the screen is drawn by.
//
// The four properties their tests held travelled with them and are named at
// their new sites rather than left implied: the six slots and the deck count
// (`the_port_zone_labels_state_the_engines_own_power_gate` and
// `both_offers_state_their_shared_deck`), the per-slot surcharge
// (`a_lab_card_prices_the_slot_surcharge_into_its_industry_figure`), the
// country and one-line effect per card (`a_lab_card_shows_its_country_and_its
// _whole_effect_text`), and F2's width sweep over EVERY registry card — which
// is now a HEIGHT equality, since a card that is `exact()`-bounded cannot
// exceed its width and the thing a longer effect text costs is a wrapped line
// (`every_lab_card_is_the_same_height_across_the_whole_registry`).

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

/// A Church faith space's printed reward in the COMPACT option vocabulary
/// (`3vp 1move`). Single-sourced from the `ChurchTrack` tables -- the same
/// `vp_reward` `delta_summary` prints after a placement, plus the
/// `movement_rider` the engine queues as assistant-move choices. Every faith
/// 4-11 carries a movement rider, so it is always shown.
///
/// `pub(crate)`: the Church space-choice option text
/// (`describe::describe_move`) reuses it, so the two option surfaces agree.
///
/// **It is NOT the `tracks` view's wording, and this doc used to claim it
/// was** (T11). The full-screen view has drawn `★3 VP` and `+1 assistant
/// moves` in full words since T6; a decision ROW is a different budget from a
/// card, so the two are deliberately different renderings of the same two
/// engine numbers. What they share is the SOURCE — `ChurchTrack` — which is
/// the parity that actually matters and the one a reader should check.
pub(crate) fn church_reward_text(faith: u8) -> String {
    format!("{}vp {}move", ChurchTrack::vp_reward(faith), ChurchTrack::movement_rider(faith))
}

/// A Customs space's printed reward in the same compact option vocabulary.
/// `CustomsTrack::rider` returns `(warehouse_takes, yen)` -- rendered
/// `Nwh`/`Nyen` -- and spaces 5-7 carry no rider, so those lines show VP
/// alone. Single-sourced from the `CustomsTrack` tables.
///
/// `pub(crate)` and NOT the `tracks` view's wording, for the reasons
/// [`church_reward_text`] states.
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
/// current rules -- the engine's fulfil-order path awards one mercenary at
/// every SECOND completed order of a country, so at most 1 per country per
/// move; pinned in core by
/// `test_fulfill_order_awards_mercenary_at_every_second_completed_order_of_a_country`)
/// is suffixed `x{n}` rather than pluralized, so the
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
                "{}: claimed achievement G{} ({}) — 1 assistant locked onto the card",
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

    /// Task 6 restyle: the state views share one design language -- a
    /// `─ Title ─...` rule header opens each of them. This covers the views
    /// that live HERE as free functions; the four that live in repl.rs
    /// (`score`/`log`/`final_score`/`hand`) are held to the same header by
    /// `repl::tests::all_state_views_share_the_rule_header_style`, which drives
    /// every `VIEW_MENU` row plus `final_score` (item 4, third review -- this
    /// test alone structurally could not see those four).
    ///
    /// **FIVE of the six left this file across T6 and T11** and none lost a
    /// property by going: `tracks`, `sites`, `ports` and `labs` became
    /// full-screen CARD surfaces in `view_render` and `goals` became the
    /// achievement index in `index_render`, none of which opens with a rule
    /// header because none of them sits under a frame -- they replace it
    /// (spec §0.5/§0.6). Their own modules hold the width, height and content
    /// pins; what this row of the design language still covers is the views
    /// that print INTO a frame, which after T11 is `players` and `score`.
    ///
    /// One entry is not a vacuous sweep: `players` is the widest surface still
    /// drawn under a frame, and the cross-surface half of this property is
    /// `repl::tests::all_state_views_share_the_rule_header_style`, which drives
    /// every `VIEW_MENU` row and asserts each opens in exactly ONE of the three
    /// recognized families.
    #[test]
    fn views_share_the_rule_header_style() {
        let state = test_state_2p();
        let rules = Rules::standard();
        let players = players_panel(&state, &rules, &seats_open(&state));
        assert!(
            players.starts_with("─ "),
            "the 'players' view must open with a rule header: {players}",
        );
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

    /// No CLI module may hardcode the Customs cost progression; it comes from
    /// `CustomsTrack::space_cost`. Built via `format!` from bare numbers (not
    /// typed as the bracketed literal itself) so the assertion doesn't quine
    /// against its own source text.
    ///
    /// **Swept over every module, not just this one.** It was written when
    /// `render.rs` drew the track, and stayed pointed here after the redesign
    /// moved that job to `screen_model::track_panel` -- guarding a room its
    /// subject had left, where the forbidden literal could no longer appear at
    /// all. Anti-vacuity: the sweep asserts it actually read the module that
    /// holds the live caller, so a renamed file cannot empty the list silently.
    #[test]
    fn no_module_hardcodes_the_customs_cost_progression() {
        let forbidden = format!("[{}, {}, {}, {}, {}, {}, {}]", 1, 1, 2, 2, 3, 3, 4);
        let modules: [(&str, &str); 5] = [
            ("render.rs", include_str!("render.rs")),
            ("screen_model.rs", include_str!("screen_model.rs")),
            ("panel_render.rs", include_str!("panel_render.rs")),
            ("view_render.rs", include_str!("view_render.rs")),
            ("describe.rs", include_str!("describe.rs")),
        ];
        let live = modules
            .iter()
            .find(|(name, _)| *name == "screen_model.rs")
            .expect("the module holding the live space_cost caller is in the sweep");
        assert!(
            live.1.contains("CustomsTrack::space_cost"),
            "screen_model.rs no longer calls CustomsTrack::space_cost -- this sweep is aimed at \
             whichever module builds the Customs track, so re-aim it before trusting a pass",
        );
        for (name, src) in modules {
            assert!(
                !src.contains(&forbidden),
                "{name} hardcodes the Customs costs; single-source them via CustomsTrack::space_cost",
            );
        }
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
