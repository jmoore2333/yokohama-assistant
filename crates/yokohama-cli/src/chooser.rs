//! The area-action choosers' models (cycle 2, plan 2a; the Port's from plan
//! 2b): the Church's, Chinatown's and the Port's, as a pure module over the
//! legal list. Nothing in this file
//! decides legality: whether a kind or an exchange is offered, whether a
//! selection can be confirmed or added to, and which faiths the list holds are
//! each answered by finding listed moves among the ones handed in
//! (`docs/plans/2026-11-21-web-area-action-choosers-design.md` §3).
//! `calculate_power`, `calculate_church_faith` and `chinatown_rate` are read for
//! what is DISPLAYED only, as `describe`'s Church and Chinatown arms read them;
//! the Port's display-only reads are `PortBoard::zone_of` and the acting
//! board's slots (a card's zone and its place in the list), never its reach.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use yokohama_core::game::actions::{calculate_church_faith, chinatown_rate, ChinatownGood};
use yokohama_core::*;

use crate::describe::{
    acting_area, chinatown_exchange_text, church_kind_words, describe_move, port_board_name,
};
use crate::screen_model::{ChinatownOffer, CHURCH_YEN_COST};

/// A thing a chooser spends or gains: a trade good, yen, or an imported good.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Good {
    Copper,
    Silk,
    Tea,
    Seafood,
    Yen,
    Imports,
}

/// The Church's six payment kinds, in the order of `AreaAction::Church`'s fields.
pub const CHURCH_KINDS: [Good; 6] =
    [Good::Copper, Good::Silk, Good::Tea, Good::Seafood, Good::Yen, Good::Imports];

/// A good and how many of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    pub good: Good,
    pub amount: u32,
}

/// What stands under a chooser's selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preview {
    /// The legal list holds a move the selection names.
    pub legal: bool,
    /// `describe_move`'s words for that move; `None` when `legal` is false.
    pub words: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChurchKind {
    /// What to send: 1 to 6.
    pub number: usize,
    pub good: Good,
    /// 1, or `CHURCH_YEN_COST` for yen.
    pub amount: u32,
    /// `church_kind_words`.
    pub words: String,
    /// The acting seat's stock of `good`.
    pub have: u32,
    pub selected: bool,
    /// Some listed Church move pays this kind.
    pub offered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChurchModel {
    /// Always six, in `CHURCH_KINDS` order.
    pub kinds: Vec<ChurchKind>,
    pub preview: Preview,
    /// "the list offers faith 8 to 11 here".
    pub offered_faiths: Option<String>,
}

/// The acting seat's (`state.current_player`'s) stock of `good`.
pub fn stock(state: &GameState, good: Good) -> u32 {
    let p = state.current_player();
    match good {
        Good::Copper => p.copper,
        Good::Silk => p.silk,
        Good::Tea => p.tea,
        Good::Seafood => p.seafood,
        Good::Yen => p.money,
        Good::Imports => p.imported_goods,
    }
}

/// A listed Church move's six flags in `CHURCH_KINDS` order; `None` for any other move.
pub fn church_flags(mv: &Move) -> Option<[bool; 6]> {
    match mv {
        Move::ExecuteAreaAction {
            action: AreaAction::Church { copper, silk, tea, seafood, yen, imported_good },
        } => Some([*copper, *silk, *tea, *seafood, *yen, *imported_good]),
        _ => None,
    }
}

/// The listed move whose six flags equal `selected`. The legal list decides:
/// a selection the list does not hold matches nothing.
pub fn church_match(legal: &[Move], selected: [bool; 6]) -> Option<Move> {
    legal.iter().find(|mv| church_flags(mv) == Some(selected)).cloned()
}

/// The sentence over the lowest and highest faith the listed Church moves reach.
fn offered_faiths(state: &GameState, legal: &[Move]) -> Option<String> {
    let area = acting_area(state)?;
    let power = calculate_power(state, state.current_player, area);
    let mut faiths = legal.iter().filter_map(church_flags).map(|f| {
        calculate_church_faith(power, f[0], f[1], f[2], f[3], f[4], f[5])
    });
    let first = faiths.next()?;
    let (lo, hi) = faiths.fold((first, first), |(lo, hi), f| (lo.min(f), hi.max(f)));
    Some(if lo == hi {
        format!("the list offers faith {lo} here")
    } else {
        format!("the list offers faith {lo} to {hi} here")
    })
}

pub fn church_model(
    state: &GameState,
    rules: &Rules,
    legal: &[Move],
    selected: [bool; 6],
) -> ChurchModel {
    let kinds = CHURCH_KINDS
        .iter()
        .enumerate()
        .map(|(i, &good)| ChurchKind {
            number: i + 1,
            good,
            amount: if good == Good::Yen { CHURCH_YEN_COST } else { 1 },
            words: church_kind_words(good),
            have: stock(state, good),
            selected: selected[i],
            offered: legal.iter().filter_map(church_flags).any(|f| f[i]),
        })
        .collect();
    let matched = church_match(legal, selected);
    let preview = Preview {
        legal: matched.is_some(),
        words: matched.map(|mv| describe_move(&mv, state, rules)),
    };
    let offered_faiths = if preview.legal { None } else { offered_faiths(state, legal) };
    ChurchModel { kinds, preview, offered_faiths }
}

/// The terminal's screen for the model (plan 2a, "The words"). Ends with a
/// newline, as every sub-prompt screen in `repl.rs` does.
pub fn church_text(model: &ChurchModel) -> String {
    let mut out =
        String::from("Church — pay any of these, at most one of each (toggle by number):\n");
    for k in &model.kinds {
        let mark = if k.selected {
            "●"
        } else if !k.offered {
            "(not offered)"
        } else {
            ""
        };
        let row = format!(
            "  [{}] {:<17}{:<10}{}",
            k.number,
            k.words,
            format!("have {}", k.have),
            mark
        );
        out.push_str(row.trim_end());
        out.push('\n');
    }
    let chosen: Vec<&str> =
        model.kinds.iter().filter(|k| k.selected).map(|k| k.words.as_str()).collect();
    let chosen = if chosen.is_empty() { "(none)".to_string() } else { chosen.join(", ") };
    out.push_str(&format!("Selected: {chosen}\n"));
    match &model.preview.words {
        Some(words) => out.push_str(&format!("  -> legal — {words}\n")),
        None => {
            out.push_str("  -> NOT currently offered (adjust the selection)\n");
            if let Some(faiths) = &model.offered_faiths {
                out.push_str(&format!("     {faiths}\n"));
            }
        }
    }
    out.push_str("> type a number to toggle, 'confirm' to pay, or 'cancel'\n");
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChinatownRow {
    /// What to send to add one: 1 to 9.
    pub number: usize,
    /// The exchange gains yen.
    pub sells: bool,
    /// `chinatown_rate`'s, at the printed rate.
    pub spend: Amount,
    pub gain: Amount,
    /// `describe`'s: "1 copper -> 2 yen".
    pub words: String,
    /// How many of it are chosen.
    pub count: u8,
    /// The list holds the selection with one more of it.
    pub can_add: bool,
    /// Some listed move uses it.
    pub offered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChinatownModel {
    /// The most exchanges in one listed move.
    pub max_exchanges: usize,
    /// The selection's total.
    pub chosen: usize,
    /// The acting seat's six holdings, in `CHURCH_KINDS` order.
    pub stock: Vec<Amount>,
    /// Always nine, in `ChinatownOffer::all()` order.
    pub rows: Vec<ChinatownRow>,
    pub preview: Preview,
}

/// The engine's good, as the chooser's: no `_` arm, so a seventh good stops
/// this file compiling.
fn good_of(good: ChinatownGood) -> Good {
    match good {
        ChinatownGood::Copper => Good::Copper,
        ChinatownGood::Silk => Good::Silk,
        ChinatownGood::Tea => Good::Tea,
        ChinatownGood::Seafood => Good::Seafood,
        ChinatownGood::Money => Good::Yen,
        ChinatownGood::Imported => Good::Imports,
    }
}

/// A listed Chinatown move's count per exchange, by `ChinatownOffer::index`;
/// `None` for any other move.
pub fn chinatown_counts(mv: &Move) -> Option<[u8; 9]> {
    match mv {
        Move::ExecuteAreaAction { action: AreaAction::Chinatown { exchanges } } => {
            let mut counts = [0u8; 9];
            for e in exchanges {
                let slot = &mut counts[ChinatownOffer::index(*e)];
                *slot = slot.saturating_add(1);
            }
            Some(counts)
        }
        _ => None,
    }
}

/// The most exchanges one listed Chinatown move holds, an exchange made twice
/// counted twice; 0 when none is listed. The ONE answer to "up to K": the main
/// list's entry (`repl.rs`, `group_area_action`) and the chooser's heading
/// (`chinatown_model`'s `max_exchanges`) both read it, so they cannot disagree.
pub fn longest_exchange_set(legal: &[Move]) -> usize {
    legal
        .iter()
        .filter_map(chinatown_counts)
        .map(|c| c.iter().map(|&n| usize::from(n)).sum())
        .max()
        .unwrap_or(0)
}

/// The listed move holding exactly `counts`, in whatever order the engine
/// listed it. The legal list decides: counts the list does not hold match
/// nothing.
pub fn chinatown_match(legal: &[Move], counts: [u8; 9]) -> Option<Move> {
    legal.iter().find(|mv| chinatown_counts(mv) == Some(counts)).cloned()
}

/// `legal` is walked into a map from count arrays to the first listed move
/// holding them; `can_add`, `offered` and the preview are each answered from
/// that map, and `max_exchanges` from `longest_exchange_set` over the same
/// list, never from the stock, the rate or the power. The
/// map and `chinatown_match` agree because the list holds no two moves with the
/// same counts (cell 2's first loop requires it).
pub fn chinatown_model(
    state: &GameState,
    rules: &Rules,
    legal: &[Move],
    counts: [u8; 9],
) -> ChinatownModel {
    let mut sets: HashMap<[u8; 9], &Move> = HashMap::new();
    for mv in legal {
        if let Some(c) = chinatown_counts(mv) {
            sets.entry(c).or_insert(mv);
        }
    }
    let total = |c: &[u8; 9]| c.iter().map(|&n| n as usize).sum::<usize>();
    let rows = ChinatownOffer::all()
        .into_iter()
        .enumerate()
        .map(|(i, exchange)| {
            let rate = chinatown_rate(exchange);
            let mut one_more = counts;
            one_more[i] = one_more[i].saturating_add(1);
            ChinatownRow {
                number: i + 1,
                sells: rate.gain == ChinatownGood::Money,
                spend: Amount { good: good_of(rate.spend), amount: rate.spend_amount },
                gain: Amount { good: good_of(rate.gain), amount: rate.gain_amount },
                words: chinatown_exchange_text(exchange).to_string(),
                count: counts[i],
                can_add: one_more[i] > counts[i] && sets.contains_key(&one_more),
                offered: sets.keys().any(|c| c[i] > 0),
            }
        })
        .collect();
    let matched = sets.get(&counts).copied();
    let preview = Preview {
        legal: matched.is_some(),
        words: matched.map(|mv| describe_move(mv, state, rules)),
    };
    ChinatownModel {
        max_exchanges: longest_exchange_set(legal),
        chosen: total(&counts),
        stock: CHURCH_KINDS
            .iter()
            .map(|&good| Amount { good, amount: stock(state, good) })
            .collect(),
        rows,
        preview,
    }
}

/// The terminal's screen for the model (plan 2a, "The words"). Ends with a
/// newline, as every sub-prompt screen in `repl.rs` does.
pub fn chinatown_text(model: &ChinatownModel) -> String {
    let unit = if model.max_exchanges == 1 { "exchange" } else { "exchanges" };
    let mut out =
        format!("Chinatown — up to {} {unit} (add by number):\n", model.max_exchanges);
    for r in &model.rows {
        let lead = match r.number {
            1 => "  sell  ",
            5 => "  buy   ",
            _ => "        ",
        };
        let mark = if r.count > 0 {
            format!("x{}", r.count)
        } else if !r.offered {
            "(not offered)".to_string()
        } else {
            String::new()
        };
        let row = format!("{lead}[{}] {:<28}{mark}", r.number, r.words);
        out.push_str(row.trim_end());
        out.push('\n');
    }
    let chosen: Vec<&str> = model
        .rows
        .iter()
        .flat_map(|r| std::iter::repeat_n(r.words.as_str(), r.count as usize))
        .collect();
    let chosen_words = if chosen.is_empty() { "(none)".to_string() } else { chosen.join(", ") };
    out.push_str(&format!(
        "Selected ({} of {}): {chosen_words}\n",
        model.chosen, model.max_exchanges
    ));
    if model.chosen == 0 {
        out.push_str("  -> add an exchange to preview it\n");
    } else {
        match &model.preview.words {
            Some(words) => out.push_str(&format!("  -> legal — {words}\n")),
            None => out.push_str("  -> NOT currently offered (adjust the selection)\n"),
        }
    }
    out.push_str("> a number adds one, 'back <number>' takes one back, 'confirm', or 'cancel'\n");
    out
}

/// One order card in the Port's chooser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortCard {
    /// What to send: the card's bare id.
    pub number: u8,
    pub id: OrderCardId,
    /// The zone of the card's slot on the acting board. Displayed, never a rule.
    pub zone: PortZone,
    /// The card's face, as the ports panel carries it.
    pub printed: OrderCard,
    /// `order_prompt_line`'s words.
    pub words: String,
    pub selected: bool,
    /// The list holds a take of the selection with this card added.
    pub can_add: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortPreview {
    /// Some listed move takes exactly the selection.
    pub legal: bool,
    /// `describe_move`'s words of every listed move taking exactly the
    /// selection, in list order: none, one or two.
    pub ways: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortModel {
    /// Which port is acting, as the ports panel names its boards.
    pub board_b: bool,
    /// The most cards one listed move takes: 1 or 2.
    pub max_cards: usize,
    /// The cards some listed move takes, in the acting board's slot order.
    pub cards: Vec<PortCard>,
    pub preview: PortPreview,
}

/// The cards a Port move takes; `None` for any other move.
pub fn port_cards(mv: &Move) -> Option<&[OrderCardId]> {
    match mv {
        Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } => {
            Some(cards_taken)
        }
        _ => None,
    }
}

/// A card set's key: its numbers, sorted, so the order of either side of a
/// comparison is no part of it.
fn port_key(cards: &[OrderCardId]) -> Vec<u8> {
    let mut key: Vec<u8> = cards.iter().map(|c| c.0).collect();
    key.sort_unstable();
    key
}

/// Every listed move taking exactly `selected`, whatever the order of either,
/// in list order.
pub fn port_matches(legal: &[Move], selected: &[OrderCardId]) -> Vec<Move> {
    let want = port_key(selected);
    legal
        .iter()
        .filter(|mv| port_cards(mv).is_some_and(|cards| port_key(cards) == want))
        .cloned()
        .collect()
}

/// The distinct cards over the listed Port moves, in the order the list first
/// names them.
pub fn port_reach(legal: &[Move]) -> Vec<OrderCardId> {
    let mut reach: Vec<OrderCardId> = Vec::new();
    for card in legal.iter().filter_map(port_cards).flatten() {
        if !reach.contains(card) {
            reach.push(*card);
        }
    }
    reach
}

/// The most cards one listed Port move takes; 0 when none is listed. The ONE
/// answer to "1 card, or 2": the main list's entry and the chooser's heading
/// both read it.
pub fn port_most_cards(legal: &[Move]) -> usize {
    legal.iter().filter_map(port_cards).map(<[OrderCardId]>::len).max().unwrap_or(0)
}

/// `legal` is walked once into a map from a sorted card set to its listed
/// moves; each card's `can_add`, the preview and `max_cards` are answered from
/// that map and from `port_most_cards` over the same list, never from the
/// power, the hand, the yen or the imported goods. The acting board's slots
/// are read for what is DISPLAYED only: which listed cards it holds, in which
/// slot order and zone. A card of `selected` that the model does not hold
/// (another board's, or none in reach) is ignored.
pub fn port_model(
    state: &GameState,
    rules: &Rules,
    legal: &[Move],
    selected: &[OrderCardId],
) -> PortModel {
    let mut takes: HashMap<Vec<u8>, Vec<&Move>> = HashMap::new();
    for mv in legal {
        if let Some(cards) = port_cards(mv) {
            takes.entry(port_key(cards)).or_default().push(mv);
        }
    }
    let reach = port_reach(legal);
    let acting = acting_area(state)
        .and_then(|area| state.board_layout.areas.get(&area))
        .map(|config| config.area_type);
    let board = match acting {
        Some(AreaType::PortA) => Some(&state.port_state.port_a),
        Some(AreaType::PortB) => state.port_state.port_b.as_ref(),
        _ => None,
    };
    let slots: Vec<(usize, OrderCardId)> = board
        .map(|b| {
            b.slots
                .iter()
                .enumerate()
                .filter_map(|(i, slot)| slot.map(|id| (i, id)))
                .filter(|(_, id)| reach.contains(id))
                .collect()
        })
        .unwrap_or_default();
    let chosen: Vec<OrderCardId> =
        slots.iter().map(|&(_, id)| id).filter(|id| selected.contains(id)).collect();
    let cards = slots
        .iter()
        .map(|&(slot, id)| {
            let is_selected = chosen.contains(&id);
            let mut with = chosen.clone();
            with.push(id);
            let printed = rules.orders.get(id).expect("the registry holds every order card");
            PortCard {
                number: id.0,
                id,
                zone: PortBoard::zone_of(slot),
                printed: printed.clone(),
                words: order_prompt_line(printed),
                selected: is_selected,
                can_add: !is_selected && takes.contains_key(&port_key(&with)),
            }
        })
        .collect();
    let ways: Vec<String> = takes
        .get(&port_key(&chosen))
        .map(|moves| moves.iter().map(|mv| describe_move(mv, state, rules)).collect())
        .unwrap_or_default();
    PortModel {
        board_b: acting == Some(AreaType::PortB),
        max_cards: port_most_cards(legal),
        cards,
        preview: PortPreview { legal: !ways.is_empty(), ways },
    }
}

/// The terminal's screen for the model (plan 2b, "The words"). Ends with a
/// newline, as every sub-prompt screen in `repl.rs` does.
pub fn port_text(model: &PortModel) -> String {
    let take = if model.max_cards >= 2 { "take 1 card, or 2" } else { "take 1 card" };
    let mut out = format!("{} — {take} (toggle by number):\n", port_board_name(model.board_b));
    let mut zone = None;
    for card in &model.cards {
        let lead = if zone == Some(card.zone) {
            "        "
        } else {
            match card.zone {
                PortZone::Low => "  LOW   ",
                PortZone::Mid => "  MID   ",
                PortZone::High => "  HIGH  ",
            }
        };
        zone = Some(card.zone);
        let mark = if card.selected { "    ●" } else { "" };
        out.push_str(&format!("{lead}[{:>2}] {}{mark}\n", card.number, card.words));
    }
    let chosen: Vec<String> =
        model.cards.iter().filter(|c| c.selected).map(|c| format!("O{:02}", c.number)).collect();
    if chosen.is_empty() {
        out.push_str("Selected: (none)\n  -> pick a card to preview it\n");
    } else {
        out.push_str(&format!("Selected: {}\n", chosen.join(", ")));
        match model.preview.ways.as_slice() {
            [] => out.push_str("  -> NOT currently offered (adjust the selection)\n"),
            [way] => out.push_str(&format!("  -> legal — {way}\n")),
            ways => {
                out.push_str(&format!(
                    "  -> legal — {} ways to pay, asked at confirm:\n",
                    ways.len()
                ));
                for way in ways {
                    out.push_str(&format!("       {way}\n"));
                }
            }
        }
    }
    out.push_str("> a card number toggles it, 'confirm' to take, or 'cancel'\n");
    out
}

/// Test fixtures shared with other modules' cells (`pub(crate)` so `describe.rs`
/// and, later, `repl.rs` reach the same position).
#[cfg(test)]
pub(crate) mod test_support {
    use yokohama_core::*;

    /// A 2P position at the Church with 42 listed Church moves: the acting
    /// seat's president on the Church with 4 assistants there (power 5), stock
    /// copper 1, silk 5, tea 2, seafood 3, money 2, imported goods 1, the
    /// track's faith 4, 5 and 6 spaces held by the OTHER seat's assistants
    /// (taken from its warehouse, so its twenty are conserved) beside the
    /// dummy's faith 7, so only faith 8 and up reaches an empty space (three
    /// kinds or more).
    ///
    /// **Other seat's assistants, not copies of the dummy's slot**: a
    /// `Session` runs the engine's invariants after every apply, and their I9
    /// clause allows the 2P dummy on faith 7 alone, so a position with the
    /// dummy on faith 4 to 6 locks a session down at its first move.
    pub(crate) fn issue_position() -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 1).unwrap();
        let mut state = finalize_setup(
            pending,
            &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
        )
        .unwrap();
        let church = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == AreaType::Church)
            .expect("2P board has a Church")
            .0;
        let me = state.current_player;
        let p = &mut state.players[me.0 as usize];
        p.president_location = PresidentLocation::OnBoard(church);
        p.assistants.on_board.insert(church, 4);
        p.assistants.in_hand -= 4;
        p.copper = 1;
        p.silk = 5;
        p.tea = 2;
        p.seafood = 3;
        p.money = 2;
        p.imported_goods = 1;
        let other = PlayerId::new(1 - me.0);
        for slot in 0..3 {
            let arrival_order = state.church_track.next_arrival_order;
            state.church_track.slots[slot] = Some(ChurchSlot { player: other, arrival_order });
            state.church_track.next_arrival_order += 1;
            let q = &mut state.players[other.0 as usize];
            q.assistants.in_warehouse -= 1;
            q.assistants.on_church += 1;
        }
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(calculate_power(&state, me, church));
        state
    }

    /// A position at Chinatown for `players` seats: the acting seat's president
    /// on the first Chinatown a seeded board holds, `assistants` of its
    /// assistants there (power `1 + assistants`), the stock as `[copper, silk,
    /// tea, seafood, money, imported goods]`, `AreaAction` and the action power
    /// set.
    fn chinatown_position(players: u8, assistants: u8, stock: [u32; 6]) -> GameState {
        let rules = Rules::standard();
        let (pending, chinatown) = (1u64..200)
            .find_map(|seed| {
                let pending =
                    new_game(GameConfig::for_player_count(players), &rules, seed).unwrap();
                let town = pending
                    .state
                    .board_layout
                    .areas
                    .iter()
                    .find(|(_, c)| c.area_type == AreaType::Chinatown)
                    .map(|(id, _)| *id);
                town.map(|t| (pending, t))
            })
            .expect("a seeded board holds a Chinatown");
        let choices = vec![StartingOrderChoice::Keep(0); players as usize];
        let mut state = finalize_setup(pending, &choices).unwrap();
        let me = state.current_player;
        let p = &mut state.players[me.0 as usize];
        p.president_location = PresidentLocation::OnBoard(chinatown);
        if assistants > 0 {
            p.assistants.on_board.insert(chinatown, assistants);
            p.assistants.in_hand -= assistants;
        }
        (p.copper, p.silk, p.tea, p.seafood, p.money, p.imported_goods) =
            (stock[0], stock[1], stock[2], stock[3], stock[4], stock[5]);
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(calculate_power(&state, me, chinatown));
        state
    }

    /// A 3P position with the acting seat's president on Chinatown with 2 of its
    /// assistants there (power 3), stock copper 1, silk 0, tea 2, seafood 1,
    /// money 3, imported goods 0: 103 listed moves, 102 of them Chinatown moves,
    /// the longest of 3 exchanges.
    pub(crate) fn ordinary_position() -> GameState {
        chinatown_position(3, 2, [1, 0, 2, 1, 3, 0])
    }

    /// `ordinary_position`'s stock with no assistant on the tile (power 1).
    pub(crate) fn ordinary_position_power_1() -> GameState {
        chinatown_position(3, 0, [1, 0, 2, 1, 3, 0])
    }

    /// A 4P position at power 5 with copper, silk, tea and seafood 5, money 20,
    /// imported goods 2: a long list of Chinatown moves.
    pub(crate) fn rich_position() -> GameState {
        chinatown_position(4, 4, [5, 5, 5, 5, 20, 2])
    }

    /// A position at a Port for `players` seats: the acting seat's president on
    /// Port A (or Port B) with `assistants` of its assistants there (power
    /// `1 + assistants`), `hand` order cards in hand, `yen` yen and `imports`
    /// imported goods, `AreaAction` and the action power set. **Valid by
    /// construction** (a `Session` runs the invariants after every apply): the
    /// assistants come out of the seat's hand supply, and an order card added to
    /// the hand beyond the dealt one is popped off the order deck.
    fn port_fixture(
        players: u8,
        seed: u64,
        board_b: bool,
        assistants: u8,
        hand: usize,
        yen: u32,
        imports: u32,
    ) -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(players), &rules, seed).unwrap();
        let choices = vec![StartingOrderChoice::Keep(0); players as usize];
        let mut state = finalize_setup(pending, &choices).unwrap();
        let wanted = if board_b { AreaType::PortB } else { AreaType::PortA };
        let port = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == wanted)
            .expect("the board holds the port")
            .0;
        let me = state.current_player;
        assert!(state.players[me.0 as usize].order_cards_in_hand.len() <= hand);
        while state.players[me.0 as usize].order_cards_in_hand.len() < hand {
            let card = state.order_deck.pop().expect("the order deck has a card");
            state.players[me.0 as usize].order_cards_in_hand.push(card);
        }
        let p = &mut state.players[me.0 as usize];
        p.president_location = PresidentLocation::OnBoard(port);
        if assistants > 0 {
            p.assistants.on_board.insert(port, assistants);
            p.assistants.in_hand -= assistants;
        }
        p.money = yen;
        p.imported_goods = imports;
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(calculate_power(&state, me, port));
        state
    }

    /// `port_position`'s board with `assistants` at the Port, `hand` order
    /// cards in hand, `yen` yen and `imports` imported goods.
    pub(crate) fn port_position_with(
        assistants: u8,
        hand: usize,
        yen: u32,
        imports: u32,
    ) -> GameState {
        port_fixture(2, 7, false, assistants, hand, yen, imports)
    }

    /// A 2P position (setup seed 7) with the acting seat's president on Port A
    /// with 4 assistants there (power 5), its hand as dealt (one order), 5 yen
    /// and 1 imported good. Measured: Port A `[O10, O34 | O28, O18 | O02, O13]`,
    /// 36 listed Port moves, 6 singles and 15 pairs each listed twice.
    pub(crate) fn port_position() -> GameState {
        port_position_with(4, 1, 5, 1)
    }

    /// A 3P position (setup seed 7) with the acting seat's president on Port B
    /// at power 5 with room in hand for two cards, 5 yen and no imported good,
    /// so each pair is listed once.
    pub(crate) fn port_b_position() -> GameState {
        port_fixture(3, 7, true, 4, 1, 5, 0)
    }

    /// `port_b_position`'s table with the seat acting on Port A instead: Port B
    /// exists and is not the acting board.
    pub(crate) fn port_a_at_three_position() -> GameState {
        port_fixture(3, 7, false, 4, 1, 5, 0)
    }

    /// The two physical cards of the technology named `name`, by id.
    pub(crate) fn copies_of(name: &str) -> [TechCardId; 2] {
        let mut ids: Vec<TechCardId> =
            Rules::standard().techs.iter().filter(|t| t.name == name).map(|t| t.id).collect();
        ids.sort_unstable_by_key(|id| id.0);
        ids.try_into().unwrap_or_else(|ids| panic!("{name} is not two cards: {ids:?}"))
    }

    /// Lays `slots` out on Laboratory A (or B) **by moving cards that exist**:
    /// a card arriving on the board comes out of the tech deck, or off the
    /// other Laboratory's slot it sat in; a card leaving it takes the arriving
    /// card's place there, or goes to the bottom of the tech deck when no card
    /// arrives for it. A card a player owns is never moved (panics).
    pub(crate) fn lay_out_lab(state: &mut GameState, board_b: bool, slots: [Option<TechCardId>; 6]) {
        let board = |s: &GameState| -> [Option<TechCardId>; 6] {
            if board_b {
                s.lab_state.lab_b.as_ref().expect("the table has Laboratory B").slots
            } else {
                s.lab_state.lab_a.slots
            }
        };
        let old = board(state);
        let new: Vec<TechCardId> = slots.iter().flatten().copied().collect();
        let mut leaving: Vec<TechCardId> = old.iter().flatten().copied().filter(|c| !new.contains(c)).collect();
        for card in new.iter().copied().filter(|c| !old.contains(&Some(*c))) {
            assert!(state.players.iter().all(|p| !p.technologies.contains(&card)), "{card:?} is owned");
            let swap_in = if leaving.is_empty() { None } else { Some(leaving.remove(0)) };
            if let Some(k) = state.tech_deck.iter().position(|c| *c == card) {
                match swap_in {
                    Some(c) => state.tech_deck[k] = c,
                    None => {
                        state.tech_deck.remove(k);
                    }
                }
            } else {
                let other = if board_b { Some(&mut state.lab_state.lab_a) } else { state.lab_state.lab_b.as_mut() };
                let slot = other
                    .and_then(|b| b.slots.iter_mut().find(|s| **s == Some(card)))
                    .unwrap_or_else(|| panic!("{card:?} is in neither the tech deck nor the other Laboratory"));
                *slot = swap_in;
            }
        }
        for card in leaving {
            state.tech_deck.insert(0, card);
        }
        if board_b {
            state.lab_state.lab_b.as_mut().expect("the table has Laboratory B").slots = slots;
        } else {
            state.lab_state.lab_a.slots = slots;
        }
    }

    /// The acting seat's President on Laboratory A (or B) with `assistants` of
    /// its assistants there (power `1 + assistants`), `yen` yen and `imports`
    /// imported goods, `AreaAction` and the action power set. The assistants
    /// come out of the seat's hand supply.
    fn at_the_lab(state: &mut GameState, board_b: bool, assistants: u8, yen: u32, imports: u32) {
        let wanted = if board_b { AreaType::LaboratoryB } else { AreaType::LaboratoryA };
        let lab = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, c)| c.area_type == wanted)
            .expect("the board holds the Laboratory")
            .0;
        let me = state.current_player;
        let p = &mut state.players[me.0 as usize];
        p.president_location = PresidentLocation::OnBoard(lab);
        p.assistants.on_board.insert(lab, assistants);
        p.assistants.in_hand -= assistants;
        p.money = yen;
        p.imported_goods = imports;
        state.turn_phase = TurnPhase::AreaAction;
        state.current_action_power = Some(calculate_power(state, me, lab));
    }

    /// `lab_twin_position` owning `owned` other technologies: the lowest-id
    /// cards still in the tech deck whose title is neither Ball nor a title on
    /// Laboratory A, taken out of the deck into the seat's hand of
    /// technologies.
    fn lab_twin_fixture(owned: usize) -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 7).unwrap();
        let mut state = finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]).unwrap();
        let ball = copies_of("Ball");
        let keep: Vec<TechCardId> =
            state.lab_state.lab_a.slots.iter().flatten().copied().filter(|c| !ball.contains(c)).collect();
        let name = |id: TechCardId| rules.techs.get(id).expect("a registry card").name.clone();
        assert_eq!(
            (name(keep[0]), name(keep[1])),
            ("Mining Technology".to_string(), "Tram".to_string()),
            "premise: seed 7's Laboratory A opens with these two first",
        );
        lay_out_lab(&mut state, false, [Some(keep[0]), Some(ball[0]), Some(keep[1]), None, Some(ball[1]), None]);
        let on_board: Vec<String> = state.lab_state.lab_a.slots.iter().flatten().map(|c| name(*c)).collect();
        let mut deck: Vec<TechCardId> = state.tech_deck.clone();
        deck.sort_unstable_by_key(|c| c.0);
        let me = state.current_player.0 as usize;
        for card in deck.into_iter().filter(|c| !on_board.contains(&name(*c))).take(owned) {
            let k = state.tech_deck.iter().position(|c| *c == card).expect("in the deck");
            state.tech_deck.remove(k);
            state.players[me].technologies.push(card);
        }
        at_the_lab(&mut state, false, 3, 9, 2);
        state
    }

    /// A 2P position (setup seed 7) with BOTH copies of Ball on Laboratory A,
    /// in slot 1 (no surcharge) and slot 4 (+2), Mining Technology in slot 0
    /// and Tram in slot 2 between them, slots 3 and 5 empty (their cards back
    /// in the tech deck); the acting seat's President there with 3 assistants
    /// (power 4), 9 yen and 2 imported goods, owning no technology. Measured:
    /// the list reads Mining Technology (1 row), Ball's slot-1 copy (1 row),
    /// Tram (3 rows), Ball's slot-4 copy (2 rows), then the decline — Ball's
    /// three rows are not next to each other.
    pub(crate) fn lab_twin_position() -> GameState {
        lab_twin_fixture(0)
    }

    /// `lab_twin_position` with three other technologies owned, so the 4th
    /// card's fee is paid either way. Measured: Mining Technology 2 rows, Ball
    /// 6, Tram 5.
    pub(crate) fn lab_twin_position_owning_three() -> GameState {
        lab_twin_fixture(3)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{
        issue_position, ordinary_position, ordinary_position_power_1, port_a_at_three_position,
        port_b_position, port_position, port_position_with, rich_position,
    };
    use super::*;
    use crate::describe::describe_move;
    use crate::screen_model::CHURCH_YEN_COST;

    fn legal_of(state: &GameState) -> Vec<Move> {
        get_legal_moves(state, &Rules::standard())
    }

    fn church_moves(legal: &[Move]) -> Vec<&Move> {
        legal.iter().filter(|m| church_flags(m).is_some()).collect()
    }

    /// Flags in `CHURCH_KINDS` order from a list of goods.
    fn flags(goods: &[Good]) -> [bool; 6] {
        let mut out = [false; 6];
        for g in goods {
            out[CHURCH_KINDS.iter().position(|k| k == g).unwrap()] = true;
        }
        out
    }

    #[test]
    fn the_issue_position_lists_42_church_moves() {
        let state = issue_position();
        let legal = legal_of(&state);
        assert_eq!(legal.len(), 43, "42 Church moves and the decline");
        assert_eq!(church_moves(&legal).len(), 42);
    }

    #[test]
    fn a_selection_the_list_holds_previews_in_describe_s_words() {
        let state = issue_position();
        let rules = Rules::standard();
        let legal = legal_of(&state);
        let sel = flags(&[Good::Silk, Good::Tea, Good::Yen]);
        let model = church_model(&state, &rules, &legal, sel);
        assert!(model.preview.legal);
        let mv = church_match(&legal, sel).expect("the list holds it");
        let words = model.preview.words.clone().expect("words when legal");
        assert_eq!(words, describe_move(&mv, &state, &rules));
        assert_eq!(words, "Church: pay silk, tea, 2 yen -> faith 8 (power 5 + 3 kinds)");
        assert_eq!(model.offered_faiths, None);
    }

    #[test]
    fn a_selection_one_kind_short_is_not_offered() {
        let state = issue_position();
        let rules = Rules::standard();
        let legal = legal_of(&state);
        // Faith 7 is a taken space; two kinds, not one, so "any two kinds
        // will do" cannot pass this fixture's refusal.
        let model = church_model(&state, &rules, &legal, flags(&[Good::Silk, Good::Tea]));
        assert!(!model.preview.legal);
        assert_eq!(model.preview.words, None);
        assert_eq!(model.offered_faiths.as_deref(), Some("the list offers faith 8 to 11 here"));
    }

    #[test]
    fn every_flag_set_matches_exactly_when_it_is_listed() {
        let state = issue_position();
        let legal = legal_of(&state);
        let mut matched = 0;
        for bits in 0u32..64 {
            let sel: [bool; 6] = std::array::from_fn(|i| bits & (1 << i) != 0);
            if church_match(&legal, sel).is_some() {
                matched += 1;
            }
        }
        assert_eq!(matched, 42);
        for mv in church_moves(&legal) {
            let f = church_flags(mv).expect("a Church move has flags");
            assert_eq!(church_match(&legal, f).as_ref(), Some(mv));
        }
    }

    #[test]
    fn offered_is_read_from_the_list_not_from_the_stock() {
        let rules = Rules::standard();
        let mut state = issue_position();
        let me = state.current_player.0 as usize;
        state.players[me].money = 1;
        let model = church_model(&state, &rules, &legal_of(&state), [false; 6]);
        let yen = &model.kinds[4];
        assert_eq!((yen.have, yen.offered), (1, false));
        for (i, k) in model.kinds.iter().enumerate() {
            if i != 4 {
                assert!(k.offered, "{:?} is still offered", k.good);
            }
        }

        let mut state = issue_position();
        state.players[me].copper = 0;
        let model = church_model(&state, &rules, &legal_of(&state), [false; 6]);
        let copper = &model.kinds[0];
        assert_eq!((copper.have, copper.offered), (0, false));

        // Where a stock rule and the list PART: copper and silk are in hand,
        // but with everything else gone no payment reaches an empty space, so
        // the list holds no Church move. `have >= amount` would read both as
        // offered.
        let mut state = issue_position();
        let p = &mut state.players[me];
        (p.tea, p.seafood, p.money, p.imported_goods) = (0, 0, 0, 0);
        let legal = legal_of(&state);
        assert!(church_moves(&legal).is_empty(), "no Church move is listed");
        let model = church_model(&state, &rules, &legal, [false; 6]);
        assert_eq!((model.kinds[0].have, model.kinds[0].offered), (1, false));
        assert_eq!((model.kinds[1].have, model.kinds[1].offered), (5, false));
        assert_eq!(model.offered_faiths, None);
    }

    #[test]
    fn the_model_s_six_kinds_carry_their_numbers_costs_and_words() {
        let state = issue_position();
        let rules = Rules::standard();
        let model = church_model(&state, &rules, &legal_of(&state), [false; 6]);
        assert_eq!(model.kinds.len(), 6);
        assert_eq!(model.kinds.iter().map(|k| k.number).collect::<Vec<_>>(), [1, 2, 3, 4, 5, 6]);
        assert_eq!(model.kinds.iter().map(|k| k.good).collect::<Vec<_>>(), CHURCH_KINDS);
        assert_eq!(
            model.kinds.iter().map(|k| k.amount).collect::<Vec<_>>(),
            [1, 1, 1, 1, CHURCH_YEN_COST, 1]
        );
        assert_eq!(
            model.kinds.iter().map(|k| k.words.as_str()).collect::<Vec<_>>(),
            ["copper", "silk", "tea", "seafood", "2 yen", "imported good"]
        );
        assert_eq!(model.kinds.iter().map(|k| k.have).collect::<Vec<_>>(), [1, 5, 2, 3, 2, 1]);

        // The fixture's own stocks repeat (copper and imported goods 1, tea and
        // money 2), so a swap of two fields in `stock` passes the reading
        // above: six different stocks tell them apart.
        let mut state = issue_position();
        let p = &mut state.players[state.current_player.0 as usize];
        (p.copper, p.silk, p.tea, p.seafood, p.money, p.imported_goods) = (1, 5, 2, 3, 4, 6);
        let model = church_model(&state, &rules, &legal_of(&state), [false; 6]);
        assert_eq!(model.kinds.iter().map(|k| k.have).collect::<Vec<_>>(), [1, 5, 2, 3, 4, 6]);
    }

    #[test]
    fn a_list_offering_one_faith_says_so() {
        let rules = Rules::standard();
        let mut state = issue_position();
        let dummy = state.church_track.slots[3].expect("the 2P dummy holds faith 7");
        for slot in 4..7 {
            state.church_track.slots[slot] = Some(dummy);
        }
        let legal = legal_of(&state);
        // Only space 11 is empty, so only the six-kind payment is listed.
        assert_eq!(church_moves(&legal).len(), 1);
        let model = church_model(&state, &rules, &legal, flags(&[Good::Silk]));
        assert!(!model.preview.legal);
        assert_eq!(model.offered_faiths.as_deref(), Some("the list offers faith 11 here"));
    }

    #[test]
    fn the_church_screen_is_the_plan_s() {
        let state = issue_position();
        let rules = Rules::standard();
        let legal = legal_of(&state);

        let legal_screen = "\
Church — pay any of these, at most one of each (toggle by number):
  [1] copper           have 1
  [2] silk             have 5    ●
  [3] tea              have 2    ●
  [4] seafood          have 3
  [5] 2 yen            have 2    ●
  [6] imported good    have 1
Selected: silk, tea, 2 yen
  -> legal — Church: pay silk, tea, 2 yen -> faith 8 (power 5 + 3 kinds)
> type a number to toggle, 'confirm' to pay, or 'cancel'
";
        let model = church_model(&state, &rules, &legal, flags(&[Good::Silk, Good::Tea, Good::Yen]));
        assert_eq!(church_text(&model), legal_screen);

        let refused_screen = "\
Church — pay any of these, at most one of each (toggle by number):
  [1] copper           have 1
  [2] silk             have 5    ●
  [3] tea              have 2
  [4] seafood          have 3
  [5] 2 yen            have 2
  [6] imported good    have 1
Selected: silk
  -> NOT currently offered (adjust the selection)
     the list offers faith 8 to 11 here
> type a number to toggle, 'confirm' to pay, or 'cancel'
";
        let model = church_model(&state, &rules, &legal, flags(&[Good::Silk]));
        assert_eq!(church_text(&model), refused_screen);

        let none = church_text(&church_model(&state, &rules, &legal, [false; 6]));
        assert!(none.contains("\nSelected: (none)\n"), "{none}");

        // A kind no listed move pays carries the other mark.
        let mut broke = issue_position();
        let me = broke.current_player.0 as usize;
        broke.players[me].money = 1;
        let screen = church_text(&church_model(&broke, &rules, &legal_of(&broke), [false; 6]));
        assert!(screen.contains("\n  [5] 2 yen            have 1    (not offered)\n"), "{screen}");
    }
    /// **The guide pair's three quoted chooser screens are these fixtures'
    /// screens** (the fix wave's B5; the Port's from plan 2b), byte for byte,
    /// as `repl.rs`'s guided-retreat
    /// captures are tied to theirs: `docs/cli-guide.md`'s fenced blocks, and
    /// `docs/cli-guide.html`'s `<pre><code>` blocks with their three entities
    /// undone (`guide_sync`'s `unescape_html`). Each guide must hold a block
    /// EQUAL to the screen, not merely containing it. Regenerate by pasting
    /// the screens into BOTH formats from one run of these fixtures.
    #[test]
    fn the_guide_quotes_the_screens_these_fixtures_render() {
        const GUIDE_MD: &str = include_str!("../../../docs/cli-guide.md");
        const GUIDE_HTML: &str = include_str!("../../../docs/cli-guide.html");
        let rules = Rules::standard();
        let church = issue_position();
        let church_screen = church_text(&church_model(
            &church,
            &rules,
            &legal_of(&church),
            flags(&[Good::Silk, Good::Tea, Good::Yen]),
        ));
        let town = ordinary_position();
        let mut counts = [0u8; 9];
        (counts[0], counts[5]) = (1, 2);
        let town_screen = chinatown_text(&chinatown_model(&town, &rules, &legal_of(&town), counts));
        let port = port_position();
        let port_screen = port_text(&port_model(&port, &rules, &legal_of(&port), &ids(&[10, 34])));

        let md_blocks: Vec<&str> = GUIDE_MD.split("```").skip(1).step_by(2).map(|b| b.trim_matches('\n')).collect();
        let html_blocks: Vec<String> = GUIDE_HTML
            .split("<pre><code>")
            .skip(1)
            .map(|b| {
                let code = b.split("</code></pre>").next().expect("a closed block");
                code.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
            })
            .collect();
        for (what, screen) in
            [("the Church's", &church_screen), ("Chinatown's", &town_screen), ("the Port's", &port_screen)]
        {
            let screen = screen.trim_end_matches('\n');
            assert!(
                md_blocks.contains(&screen),
                "docs/cli-guide.md quotes no block equal to {what} screen as the fixture renders it:\n{screen}"
            );
            assert!(
                html_blocks.iter().any(|b| b.trim_matches('\n') == screen),
                "docs/cli-guide.html quotes no block equal to {what} screen as the fixture renders it:\n{screen}"
            );
        }
    }

    fn chinatown_moves(legal: &[Move]) -> Vec<&Move> {
        legal.iter().filter(|m| chinatown_counts(m).is_some()).collect()
    }

    /// The listed move's exchanges, or panic.
    fn exchanges_of(mv: &Move) -> Vec<ChinatownExchange> {
        match mv {
            Move::ExecuteAreaAction { action: AreaAction::Chinatown { exchanges } } => {
                exchanges.clone()
            }
            other => panic!("not a Chinatown move: {other:?}"),
        }
    }

    /// `describe`'s unit word for a good.
    fn unit_words(good: Good) -> &'static str {
        match good {
            Good::Copper => "copper",
            Good::Silk => "silk",
            Good::Tea => "tea",
            Good::Seafood => "seafood",
            Good::Yen => "yen",
            Good::Imports => "imported good",
        }
    }

    /// `legal` without every listed Chinatown move holding a sale of tea.
    fn without_tea_sales(legal: &[Move]) -> Vec<Move> {
        legal
            .iter()
            .filter(|mv| chinatown_counts(mv).is_none_or(|c| c[2] == 0))
            .cloned()
            .collect()
    }

    fn can_add_of(model: &ChinatownModel) -> Vec<bool> {
        model.rows.iter().map(|r| r.can_add).collect()
    }

    #[test]
    fn the_ordinary_position_lists_102_exchange_sets_of_at_most_three() {
        let state = ordinary_position();
        assert_eq!(state.current_action_power, Some(3));
        let legal = legal_of(&state);
        assert_eq!(legal.len(), 103, "102 Chinatown moves and the decline");
        let moves = chinatown_moves(&legal);
        assert_eq!(moves.len(), 102);
        assert_eq!(moves.iter().map(|m| exchanges_of(m).len()).max(), Some(3));
    }

    #[test]
    fn every_listed_set_matches_itself_whatever_its_order() {
        let state = ordinary_position();
        let legal = legal_of(&state);
        let mut reversed_differs = 0;
        for mv in chinatown_moves(&legal) {
            let counts = chinatown_counts(mv).expect("a Chinatown move has counts");
            assert_eq!(chinatown_match(&legal, counts).as_ref(), Some(mv));
            let mut backwards = exchanges_of(mv);
            backwards.reverse();
            if backwards != exchanges_of(mv) {
                reversed_differs += 1;
            }
            let reversed = Move::ExecuteAreaAction {
                action: AreaAction::Chinatown { exchanges: backwards },
            };
            assert_eq!(chinatown_counts(&reversed), Some(counts));
        }
        assert!(reversed_differs > 0, "the list holds sets whose order matters to a vector compare");
        // The engine lists some sets out of row order: this one among them.
        assert!(
            chinatown_moves(&legal).iter().any(|m| exchanges_of(m)
                == [ChinatownExchange::YenToSilk, ChinatownExchange::SilkToYen]),
            "[YenToSilk, SilkToYen] is listed"
        );

        // A list that holds every set with its exchanges the other way round
        // still matches each by its counts, and returns the move as listed.
        let backwards_list: Vec<Move> = legal
            .iter()
            .map(|mv| match mv {
                Move::ExecuteAreaAction { action: AreaAction::Chinatown { exchanges } } => {
                    let mut e = exchanges.clone();
                    e.reverse();
                    Move::ExecuteAreaAction { action: AreaAction::Chinatown { exchanges: e } }
                }
                other => other.clone(),
            })
            .collect();
        for mv in chinatown_moves(&backwards_list) {
            let counts = chinatown_counts(mv).expect("a Chinatown move has counts");
            assert_eq!(chinatown_match(&backwards_list, counts).as_ref(), Some(mv));
        }
        assert_eq!(
            chinatown_counts(&Move::ExecuteAreaAction { action: AreaAction::Skip }),
            None
        );
    }

    #[test]
    fn one_more_can_be_added_exactly_when_the_list_holds_it() {
        let state = ordinary_position();
        let rules = Rules::standard();
        let legal = legal_of(&state);
        // No silk to sell, and 3 yen does not buy an imported good.
        let model = chinatown_model(&state, &rules, &legal, [0; 9]);
        assert_eq!(
            can_add_of(&model),
            [true, false, true, true, true, true, true, true, false]
        );
        // Selling first is what makes the purchase addable.
        let mut sold = [0u8; 9];
        sold[0] = 1;
        let model = chinatown_model(&state, &rules, &legal, sold);
        assert!(model.rows[8].can_add);
    }

    #[test]
    fn a_selection_left_by_taking_the_sale_back_is_not_offered() {
        let state = ordinary_position();
        let rules = Rules::standard();
        let legal = legal_of(&state);
        let mut counts = [0u8; 9];
        counts[8] = 1;
        let model = chinatown_model(&state, &rules, &legal, counts);
        assert!(!model.preview.legal);
        assert_eq!(model.preview.words, None);
        assert_eq!(model.chosen, 1);
    }

    #[test]
    fn offered_is_read_from_the_list() {
        let rules = Rules::standard();
        let state = ordinary_position();
        let legal = legal_of(&state);
        let model = chinatown_model(&state, &rules, &legal, [0; 9]);
        // Row 9 is listed behind a sale and row 2 behind a silk purchase, yet
        // neither can be added from nothing: `offered` is not `can_add`.
        assert!(model.rows.iter().all(|r| r.offered));
        assert!(!model.rows[1].can_add && !model.rows[8].can_add);

        let state = ordinary_position_power_1();
        assert_eq!(state.current_action_power, Some(1));
        let legal = legal_of(&state);
        assert_eq!(chinatown_moves(&legal).len(), 7);
        let model = chinatown_model(&state, &rules, &legal, [0; 9]);
        let offered: Vec<bool> = model.rows.iter().map(|r| r.offered).collect();
        assert_eq!(offered, [true, false, true, true, true, true, true, true, false]);
    }

    #[test]
    fn the_rows_carry_the_engine_s_rates_and_describe_s_words() {
        let rules = Rules::standard();
        let mut state = ordinary_position();
        // The Mining Technology adds a copper to the copper-gaining exchange
        // when it is APPLIED; the row shows the printed rate, as the words do.
        let mining = rules.techs.by_code("tech_mining_technology").expect("Mining").id;
        let me = state.current_player.0 as usize;
        state.players[me].technologies.push(mining);
        let legal = legal_of(&state);
        let model = chinatown_model(&state, &rules, &legal, [0; 9]);
        assert_eq!(model.rows.len(), 9);
        for (i, (exchange, row)) in ChinatownOffer::all().into_iter().zip(&model.rows).enumerate() {
            let rate = chinatown_rate(exchange);
            assert_eq!(row.number, i + 1);
            assert_eq!(row.spend.amount, rate.spend_amount);
            assert_eq!(row.gain.amount, rate.gain_amount);
            assert_eq!(row.spend.good, good_of(rate.spend));
            assert_eq!(row.gain.good, good_of(rate.gain));
            let single = Move::ExecuteAreaAction {
                action: AreaAction::Chinatown { exchanges: vec![exchange] },
            };
            assert_eq!(
                format!("Chinatown: {}", row.words),
                describe_move(&single, &state, &rules)
            );
            assert_eq!(
                row.words,
                format!(
                    "{} {} -> {} {}",
                    row.spend.amount,
                    unit_words(row.spend.good),
                    row.gain.amount,
                    unit_words(row.gain.good)
                )
            );
            assert_eq!(row.sells, i < 4, "row {} sells", i + 1);
        }
        assert_eq!(model.rows[4].gain, Amount { good: Good::Copper, amount: 1 });
    }

    #[test]
    fn the_stock_is_the_acting_seat_s_six_holdings() {
        let rules = Rules::standard();
        let state = ordinary_position();
        let model = chinatown_model(&state, &rules, &legal_of(&state), [0; 9]);
        let amounts: Vec<u32> = model.stock.iter().map(|a| a.amount).collect();
        assert_eq!(amounts, [1, 0, 2, 1, 3, 0]);
        assert_eq!(model.stock.iter().map(|a| a.good).collect::<Vec<_>>(), CHURCH_KINDS);

        // The position's own holdings repeat (copper and seafood 1, silk and
        // imported goods 0): six different ones tell two swapped entries apart.
        let mut state = ordinary_position();
        let p = &mut state.players[state.current_player.0 as usize];
        (p.copper, p.silk, p.tea, p.seafood, p.money, p.imported_goods) = (1, 2, 3, 4, 5, 6);
        let model = chinatown_model(&state, &rules, &legal_of(&state), [0; 9]);
        let amounts: Vec<u32> = model.stock.iter().map(|a| a.amount).collect();
        assert_eq!(amounts, [1, 2, 3, 4, 5, 6]);
        assert_eq!(model.stock.iter().map(|a| a.good).collect::<Vec<_>>(), CHURCH_KINDS);
    }

    #[test]
    fn the_chinatown_screen_is_the_plan_s() {
        let rules = Rules::standard();
        let state = ordinary_position();
        let legal = legal_of(&state);

        let screen = "\
Chinatown — up to 3 exchanges (add by number):
  sell  [1] 1 copper -> 2 yen           x1
        [2] 1 silk -> 2 yen
        [3] 1 tea -> 1 yen
        [4] 1 seafood -> 1 yen
  buy   [5] 2 yen -> 1 copper
        [6] 2 yen -> 1 silk             x2
        [7] 1 yen -> 1 tea
        [8] 1 yen -> 1 seafood
        [9] 4 yen -> 1 imported good
Selected (3 of 3): 1 copper -> 2 yen, 2 yen -> 1 silk, 2 yen -> 1 silk
  -> legal — Chinatown: 1 copper -> 2 yen, 2 yen -> 1 silk, 2 yen -> 1 silk
> a number adds one, 'back <number>' takes one back, 'confirm', or 'cancel'
";
        let mut counts = [0u8; 9];
        (counts[0], counts[5]) = (1, 2);
        let listed = chinatown_match(&legal, counts).expect("the list holds it");
        assert_eq!(
            exchanges_of(&listed),
            [
                ChinatownExchange::CopperToYen,
                ChinatownExchange::YenToSilk,
                ChinatownExchange::YenToSilk
            ]
        );
        assert_eq!(chinatown_text(&chinatown_model(&state, &rules, &legal, counts)), screen);

        let empty = chinatown_text(&chinatown_model(&state, &rules, &legal, [0; 9]));
        assert!(
            empty.contains("\nSelected (0 of 3): (none)\n  -> add an exchange to preview it\n"),
            "{empty}"
        );

        // A selection the list does not hold.
        let mut row9 = [0u8; 9];
        row9[8] = 1;
        let refused = chinatown_text(&chinatown_model(&state, &rules, &legal, row9));
        assert!(
            refused.contains(
                "\nSelected (1 of 3): 4 yen -> 1 imported good\n  -> NOT currently offered (adjust the selection)\n"
            ),
            "{refused}"
        );

        let low = ordinary_position_power_1();
        let screen = chinatown_text(&chinatown_model(&low, &rules, &legal_of(&low), [0; 9]));
        assert!(
            screen.starts_with("Chinatown — up to 1 exchange (add by number):\n"),
            "{screen}"
        );
        assert!(screen.contains("\nSelected (0 of 1): (none)\n"), "{screen}");
        assert!(
            screen.contains("\n        [2] 1 silk -> 2 yen             (not offered)\n"),
            "{screen}"
        );
        // The selection's mark wins over `(not offered)`.
        let mut silk = [0u8; 9];
        silk[1] = 1;
        let screen = chinatown_text(&chinatown_model(&low, &rules, &legal_of(&low), silk));
        assert!(screen.contains("\n        [2] 1 silk -> 2 yen             x1\n"), "{screen}");
    }

    #[test]
    fn a_long_list_costs_what_it_costs() {
        let rules = Rules::standard();
        let state = rich_position();
        assert_eq!(state.current_action_power, Some(5));
        let legal = legal_of(&state);
        assert_eq!(chinatown_moves(&legal).len(), 2001);
        let mut one = [0u8; 9];
        one[0] = 1;
        let started = std::time::Instant::now();
        let model = chinatown_model(&state, &rules, &legal, one);
        let elapsed = started.elapsed();
        println!("a_long_list_costs_what_it_costs: model over 2001 listed moves built in {elapsed:?}");
        assert_eq!(model.rows.len(), 9);
        assert_eq!(model.chosen, 1);
    }

    #[test]
    fn the_flags_and_the_heading_are_read_from_the_list_handed_in() {
        let rules = Rules::standard();
        let state = ordinary_position();
        let legal = legal_of(&state);

        // The seat holds 2 tea, yet the slice holds no tea sale.
        let cut = without_tea_sales(&legal);
        assert!(cut.len() < legal.len());
        let model = chinatown_model(&state, &rules, &cut, [0; 9]);
        assert_eq!(model.stock[2].amount, 2);
        assert!(!model.rows[2].can_add && !model.rows[2].offered);
        for (i, row) in model.rows.iter().enumerate() {
            if i != 2 {
                assert_eq!(
                    row.can_add,
                    [true, false, false, true, true, true, true, true, false][i],
                    "row {}",
                    i + 1
                );
            }
        }

        // A list cut to its one-exchange moves: the heading is 1 at power 3,
        // and nothing can be added to a copper sale.
        let singles: Vec<Move> = legal
            .iter()
            .filter(|mv| chinatown_counts(mv).is_some_and(|c| c.iter().sum::<u8>() == 1))
            .cloned()
            .collect();
        let model = chinatown_model(&state, &rules, &singles, [0; 9]);
        assert_eq!(model.max_exchanges, 1);
        let mut sold = [0u8; 9];
        sold[0] = 1;
        let model = chinatown_model(&state, &rules, &singles, sold);
        assert!(model.rows.iter().all(|r| !r.can_add), "{:?}", can_add_of(&model));

        // A list cut to the sets that repeat an exchange: the longest is 3
        // exchanges, yet none holds more than 2 DIFFERENT ones, so a count of
        // different exchanges reads less.
        let repeating: Vec<Move> = legal
            .iter()
            .filter(|mv| chinatown_counts(mv).is_some_and(|c| c.iter().any(|&n| n >= 2)))
            .cloned()
            .collect();
        assert!(chinatown_moves(&repeating)
            .iter()
            .all(|m| chinatown_counts(m).unwrap().iter().filter(|&&n| n > 0).count() <= 2));
        let model = chinatown_model(&state, &rules, &repeating, [0; 9]);
        assert_eq!(model.max_exchanges, 3);
    }

    #[test]
    fn the_model_previews_every_listed_set_in_describe_s_words() {
        let rules = Rules::standard();
        let state = ordinary_position();
        let legal = legal_of(&state);
        let listed = chinatown_moves(&legal);
        assert_eq!(listed.len(), 102);
        for mv in listed {
            let counts = chinatown_counts(mv).expect("a Chinatown move has counts");
            let model = chinatown_model(&state, &rules, &legal, counts);
            assert_eq!(
                model.preview,
                Preview { legal: true, words: Some(describe_move(mv, &state, &rules)) },
                "{:?}",
                exchanges_of(mv)
            );
        }
    }

    // ---- The Port's chooser (plan 2b, Task 1) ----

    fn port_moves(legal: &[Move]) -> Vec<&Move> {
        legal.iter().filter(|m| port_cards(m).is_some()).collect()
    }

    fn ids(numbers: &[u8]) -> Vec<OrderCardId> {
        numbers.iter().map(|&n| OrderCardId(n)).collect()
    }

    /// The listed Port moves' cards as sorted numbers, each distinct set once, in
    /// first-listed order.
    fn listed_sets(legal: &[Move]) -> Vec<Vec<u8>> {
        let mut out: Vec<Vec<u8>> = Vec::new();
        for mv in port_moves(legal) {
            let mut set: Vec<u8> = port_cards(mv).unwrap().iter().map(|c| c.0).collect();
            set.sort_unstable();
            if !out.contains(&set) {
                out.push(set);
            }
        }
        out
    }

    #[test]
    fn the_fixtures_are_what_the_plan_measured() {
        let rules = Rules::standard();
        let state = port_position();
        let legal = legal_of(&state);
        let moves = port_moves(&legal);
        assert_eq!(moves.len(), 36);
        let lens: Vec<usize> = moves.iter().map(|m| port_cards(m).unwrap().len()).collect();
        assert_eq!(lens.iter().filter(|&&n| n == 1).count(), 6);
        assert_eq!(lens.iter().filter(|&&n| n == 2).count(), 30);
        for set in listed_sets(&legal).iter().filter(|s| s.len() == 2) {
            let times = moves
                .iter()
                .filter(|m| {
                    let mut c: Vec<u8> = port_cards(m).unwrap().iter().map(|c| c.0).collect();
                    c.sort_unstable();
                    &c == set
                })
                .count();
            assert_eq!(times, 2, "{set:?} is listed once per way to pay");
        }
        assert_eq!(port_reach(&legal).len(), 6);
        assert_eq!(port_most_cards(&legal), 2);
        assert_eq!(
            state.port_state.port_a.slots.map(|s| s.map(|c| c.0)),
            [Some(10), Some(34), Some(28), Some(18), Some(2), Some(13)]
        );
        assert_eq!(state.current_player().order_cards_in_hand, ids(&[24]));

        // Port B's seat: each pair once (no imported good).
        let b = port_b_position();
        let b_legal = legal_of(&b);
        assert_eq!(
            b.port_state.port_b.as_ref().unwrap().slots.map(|s| s.map(|c| c.0)),
            [Some(18), Some(2), Some(13), Some(6), Some(3), Some(35)]
        );
        assert_eq!(port_moves(&b_legal).len(), 6 + 15);
        assert_eq!(port_most_cards(&b_legal), 2);

        check_invariants(&state, &rules).expect("port_position is a valid state");
        check_invariants(&b, &rules).expect("port_b_position is a valid state");
        check_invariants(&port_position_with(4, 2, 5, 1), &rules)
            .expect("port_position_with is a valid state");
        check_invariants(&port_position_with(1, 1, 1, 0), &rules)
            .expect("port_position_with is a valid state");
    }

    #[test]
    fn every_listed_take_matches_itself_whatever_its_order() {
        let state = port_position();
        let legal = legal_of(&state);
        for mv in port_moves(&legal) {
            let cards = port_cards(mv).unwrap().to_vec();
            let found = port_matches(&legal, &cards);
            assert!(found.contains(mv), "{mv:?} matches itself");
            let mut reversed = cards.clone();
            reversed.reverse();
            assert_eq!(port_matches(&legal, &reversed), found, "reversed {cards:?}");
        }
        // A pair is held by exactly the two listed moves, in list order.
        let pair = ids(&[10, 34]);
        let expected: Vec<Move> = legal
            .iter()
            .filter(|m| {
                port_cards(m).is_some_and(|c| c.len() == 2 && c.contains(&pair[0]) && c.contains(&pair[1]))
            })
            .cloned()
            .collect();
        assert_eq!(expected.len(), 2);
        assert_eq!(port_matches(&legal, &pair), expected);
        let mut flipped = pair.clone();
        flipped.reverse();
        assert_eq!(port_matches(&legal, &flipped), expected);
        // Nothing listed takes a card no list holds, or a third card.
        assert!(port_matches(&legal, &ids(&[99])).is_empty());
        assert!(port_matches(&legal, &ids(&[10, 34, 28])).is_empty());
        assert!(port_matches(&legal, &[]).is_empty());
    }

    #[test]
    fn can_add_is_read_off_the_list() {
        let rules = Rules::standard();
        // (assistants, hand, yen, imports, cards in the model, most cards)
        let positions: [(u8, usize, u32, u32, usize, usize); 5] = [
            (4, 1, 5, 1, 6, 2),
            (4, 2, 5, 1, 6, 1), // two orders in hand: rival "two cards while the hand has room for one"
            (4, 1, 1, 0, 6, 1), // 1 yen: rival "a second card is free"
            (1, 1, 5, 1, 2, 2), // power 2: rival "every card on the board"
            // Three orders in hand: the list holds the decline alone, every zone
            // is open at power 5 and the board holds six cards. Rival: "a card
            // is in the model when its zone is accessible at the action power"
            // (`port_accessible_zones`).
            (4, 3, 5, 1, 0, 0),
        ];
        for (assistants, hand, yen, imports, in_model, most) in positions {
            let state = port_position_with(assistants, hand, yen, imports);
            check_invariants(&state, &rules).expect("the position is a valid state");
            let legal = legal_of(&state);
            let reach = port_reach(&legal);
            assert_eq!(reach.len(), in_model, "reach at {assistants}/{hand}/{yen}/{imports}");
            assert_eq!(port_most_cards(&legal), most);
            let mut selections: Vec<Vec<OrderCardId>> = vec![vec![]];
            for &a in &reach {
                selections.push(vec![a]);
                for &b in &reach {
                    if a.0 < b.0 {
                        selections.push(vec![a, b]);
                    }
                }
            }
            for selection in selections {
                let model = port_model(&state, &rules, &legal, &selection);
                assert_eq!(model.cards.len(), in_model);
                for card in &model.cards {
                    if selection.contains(&card.id) {
                        assert!(card.selected && !card.can_add, "{:?}", card.id);
                        continue;
                    }
                    let mut with = selection.clone();
                    with.push(card.id);
                    assert_eq!(
                        card.can_add,
                        !port_matches(&legal, &with).is_empty(),
                        "{assistants}/{hand}/{yen}/{imports}: {:?} on {selection:?}",
                        card.id
                    );
                }
            }
            // The positions separate the list from each restated rule.
            let none = port_model(&state, &rules, &legal, &[]);
            assert!(none.cards.iter().all(|c| c.can_add), "any one card can be added to nothing");
            if most == 1 {
                let one = port_model(&state, &rules, &legal, &reach[..1]);
                assert!(one.cards.iter().all(|c| !c.can_add), "no second card is listed");
            }
        }
        // Power 2: the board holds six cards, the model two.
        let low = port_position_with(1, 1, 5, 1);
        assert_eq!(low.port_state.port_a.slots.iter().flatten().count(), 6);
        // Three orders in hand: the board holds six cards at power 5 and the
        // list none of them.
        let full = port_position_with(4, 3, 5, 1);
        assert_eq!(full.port_state.port_a.slots.iter().flatten().count(), 6);
        assert_eq!(full.current_action_power, Some(5));
        assert!(port_moves(&legal_of(&full)).is_empty());

        // A list that holds both takes of a pair out: with O10 on, O34 cannot
        // be added and O28 can. Rival: "a second card is addable whenever some
        // listed move takes two" (`can_add` from the list's maximum).
        let state = port_position();
        let mut cut = legal_of(&state);
        cut.retain(|m| port_cards(m).is_none_or(|c| port_key(c) != [10, 34]));
        assert_eq!(port_most_cards(&cut), 2);
        let model = port_model(&state, &rules, &cut, &ids(&[10]));
        let can_add: Vec<(u8, bool)> = model.cards.iter().map(|c| (c.number, c.can_add)).collect();
        assert_eq!(
            can_add,
            [(10, false), (34, false), (28, true), (18, true), (2, true), (13, true)]
        );
    }

    #[test]
    fn a_third_card_cannot_be_added() {
        let rules = Rules::standard();
        let state = port_position();
        let legal = legal_of(&state);
        let model = port_model(&state, &rules, &legal, &ids(&[10, 34]));
        assert_eq!(model.cards.len(), 6);
        for card in &model.cards {
            if card.number == 10 || card.number == 34 {
                assert!(card.selected && !card.can_add, "{}", card.number);
            } else {
                assert!(!card.selected && !card.can_add, "{}", card.number);
            }
        }
    }

    #[test]
    fn the_preview_is_describe_s() {
        let rules = Rules::standard();
        let state = port_position();
        let legal = legal_of(&state);

        let single = port_matches(&legal, &ids(&[10]));
        assert_eq!(single.len(), 1);
        let model = port_model(&state, &rules, &legal, &ids(&[10]));
        assert_eq!(
            model.preview,
            PortPreview { legal: true, ways: vec![describe_move(&single[0], &state, &rules)] }
        );

        let model = port_model(&state, &rules, &legal, &ids(&[10, 34]));
        let pair = port_matches(&legal, &ids(&[10, 34]));
        assert_eq!(
            model.preview.ways,
            pair.iter().map(|m| describe_move(m, &state, &rules)).collect::<Vec<_>>()
        );
        assert!(model.preview.legal);
        assert_eq!(model.preview.ways.len(), 2);
        assert!(model.preview.ways[0].ends_with(", paying 2 yen"), "{:?}", model.preview.ways);
        assert!(model.preview.ways[1].ends_with(", paying 1 imported good"), "{:?}", model.preview.ways);

        let model = port_model(&state, &rules, &legal, &[]);
        assert_eq!(model.preview, PortPreview { legal: false, ways: vec![] });

        // With no imported good a pair is listed once: one way, not two.
        let one_way = port_position_with(4, 1, 5, 0);
        check_invariants(&one_way, &rules).expect("the position is a valid state");
        let one_legal = legal_of(&one_way);
        let model = port_model(&one_way, &rules, &one_legal, &ids(&[10, 34]));
        assert!(model.preview.legal);
        assert_eq!(model.preview.ways.len(), 1);
        assert!(model.preview.ways[0].ends_with(", paying 2 yen"), "{:?}", model.preview.ways);

        // The engine never lists a pair without its singles; the model must not
        // rest on that: remove O10's single and it alone reads not offered.
        let mut cut = legal.clone();
        cut.retain(|m| port_cards(m).map(|c| c.to_vec()) != Some(ids(&[10])));
        let model = port_model(&state, &rules, &cut, &ids(&[10]));
        assert_eq!(model.preview, PortPreview { legal: false, ways: vec![] });
        // The list now names O34 before O10; the cards stay in slot order.
        assert_eq!(port_reach(&cut)[0], OrderCardId(34));
        assert_eq!(
            model.cards.iter().map(|c| c.number).collect::<Vec<_>>(),
            [10, 34, 28, 18, 2, 13]
        );
        assert!(
            port_text(&model).contains("\n  -> NOT currently offered (adjust the selection)\n"),
            "{}",
            port_text(&model)
        );
    }

    #[test]
    fn the_cards_are_in_slot_order_with_their_zones() {
        let rules = Rules::standard();
        let state = port_position();
        let legal = legal_of(&state);
        let model = port_model(&state, &rules, &legal, &[]);
        assert_eq!(model.cards.iter().map(|c| c.number).collect::<Vec<_>>(), [10, 34, 28, 18, 2, 13]);
        assert_eq!(
            model.cards.iter().map(|c| c.zone).collect::<Vec<_>>(),
            [PortZone::Low, PortZone::Low, PortZone::Mid, PortZone::Mid, PortZone::High, PortZone::High]
        );
        for c in &model.cards {
            let printed = rules.orders.get(c.id).expect("the registry holds it");
            assert_eq!(c.id.0, c.number);
            assert_eq!(&c.printed, printed);
            assert_eq!(c.words, order_prompt_line(printed));
        }

        // The same cards with two slots swapped read the swapped order.
        let mut swapped = state.clone();
        swapped.port_state.port_a.slots.swap(0, 5);
        let model = port_model(&swapped, &rules, &legal_of(&swapped), &[]);
        assert_eq!(model.cards.iter().map(|c| c.number).collect::<Vec<_>>(), [13, 34, 28, 18, 2, 10]);
        assert_eq!(model.cards[0].zone, PortZone::Low);
        assert_eq!(model.cards[5].zone, PortZone::High);
    }

    #[test]
    fn port_b_is_the_acting_board() {
        let rules = Rules::standard();
        let state = port_b_position();
        let legal = legal_of(&state);
        let model = port_model(&state, &rules, &legal, &[]);
        assert!(model.board_b);
        let b = state.port_state.port_b.as_ref().expect("three players have Port B");
        assert_eq!(model.cards.len(), 6);
        for c in &model.cards {
            assert!(b.find_card(c.id).is_some(), "{} sits on Port B", c.number);
            assert!(state.port_state.port_a.find_card(c.id).is_none(), "{} is not Port A's", c.number);
        }
        assert!(port_text(&model).starts_with("Port B — take 1 card, or 2"), "{}", port_text(&model));

        // Port A is the other answer, at two players (no Port B on the table)
        // and at three, where Port B exists and is not the acting board.
        let a = port_position();
        assert!(!port_model(&a, &rules, &legal_of(&a), &[]).board_b);
        let three = port_a_at_three_position();
        check_invariants(&three, &rules).expect("the position is a valid state");
        assert!(three.port_state.port_b.is_some());
        let three_legal = legal_of(&three);
        let on_a = port_model(&three, &rules, &three_legal, &[]);
        assert!(!on_a.board_b);
        assert_eq!(on_a.cards.len(), 6);
        for c in &on_a.cards {
            assert!(three.port_state.port_a.find_card(c.id).is_some(), "{} sits on Port A", c.number);
            assert!(
                three.port_state.port_b.as_ref().unwrap().find_card(c.id).is_none(),
                "{} is not Port B's",
                c.number
            );
        }
        assert!(port_text(&on_a).starts_with("Port A — take 1 card, or 2"), "{}", port_text(&on_a));

        // A card in the acting board's slots that the list holds no take for
        // is ignored too: at power 2 only O10 and O34 are in reach, O28 is not.
        let low = port_position_with(1, 1, 5, 1);
        let low_legal = legal_of(&low);
        assert_eq!(
            port_model(&low, &rules, &low_legal, &ids(&[28])),
            port_model(&low, &rules, &low_legal, &[])
        );

        // A card of `selected` the acting board does not hold is ignored.
        let foreign = state.port_state.port_a.slots[0].expect("Port A holds a card");
        let model_with = port_model(&state, &rules, &legal, &[foreign]);
        assert_eq!(model_with, model);
        let own = ids(&[18]);
        let mixed = port_model(&state, &rules, &legal, &[foreign, own[0]]);
        assert_eq!(mixed, port_model(&state, &rules, &legal, &own));
    }

    #[test]
    fn the_screen_is_the_plan_s() {
        let rules = Rules::standard();
        let state = port_position();
        let legal = legal_of(&state);
        let plan = "\
Port A — take 1 card, or 2 (toggle by number):
  LOW   [10] O10 [Britain]  needs 2 copper + 1 tea + 1 fish  ->  4 VP + 2 move    ●
        [34] O34 [France]  needs 2 copper + 3 tea  ->  9 VP + 1 import    ●
  MID   [28] O28 [America]  needs 1 silk + 4 fish  ->  3 VP + 1 import + 1 warehouse
        [18] O18 [America]  needs 2 copper + 3 fish  ->  6 VP + 1 import
  HIGH  [ 2] O02 [France]  needs 2 tea + 1 fish  ->  3 VP
        [13] O13 [Britain]  needs 2 copper + 1 silk + 2 fish  ->  8 VP + 1 import
Selected: O10, O34
  -> legal — 2 ways to pay, asked at confirm:
       Port: take O10, O34, paying 2 yen
       Port: take O10, O34, paying 1 imported good
> a card number toggles it, 'confirm' to take, or 'cancel'
";
        let model = port_model(&state, &rules, &legal, &ids(&[10, 34]));
        assert_eq!(port_text(&model), plan);

        // The order of `selected` is not the order of the screen.
        let reversed = port_model(&state, &rules, &legal, &ids(&[34, 10]));
        assert_eq!(port_text(&reversed), plan);

        let none = port_text(&port_model(&state, &rules, &legal, &[]));
        let lines: Vec<&str> = none.lines().collect();
        assert_eq!(
            lines[lines.len() - 3..],
            [
                "Selected: (none)",
                "  -> pick a card to preview it",
                "> a card number toggles it, 'confirm' to take, or 'cancel'"
            ]
        );
        assert!(none.ends_with('\n'));

        // One card on: one way, in describe's words.
        let one = port_text(&port_model(&state, &rules, &legal, &ids(&[28])));
        assert!(one.contains("\nSelected: O28\n  -> legal — Port: take O28\n"), "{one}");

        // No listed move takes two: the heading drops "or 2".
        let single_only = port_position_with(4, 2, 5, 1);
        let screen =
            port_text(&port_model(&single_only, &rules, &legal_of(&single_only), &[]));
        assert!(screen.starts_with("Port A — take 1 card (toggle by number):\n"), "{screen}");
    }

    #[test]
    fn max_cards_is_read_off_the_list() {
        let rules = Rules::standard();
        for (assistants, hand, yen, imports, most) in
            [(4, 1, 5, 1, 2), (4, 2, 5, 1, 1), (4, 1, 1, 0, 1)]
        {
            let state = port_position_with(assistants, hand, yen, imports);
            check_invariants(&state, &rules).expect("the position is a valid state");
            let legal = legal_of(&state);
            assert_eq!(port_most_cards(&legal), most);
            assert_eq!(port_model(&state, &rules, &legal, &[]).max_cards, most);
        }
        assert_eq!(port_most_cards(&[]), 0);
    }
}
