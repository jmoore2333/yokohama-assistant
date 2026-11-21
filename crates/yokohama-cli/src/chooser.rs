//! The area-action choosers' models (cycle 2, plan 2a): the Church's here, as
//! a pure module over the legal list. Nothing in this file decides legality:
//! whether a kind is offered, whether a selection can be confirmed and which
//! faiths the list holds are each answered by finding listed moves among the
//! ones handed in (`docs/plans/2026-11-21-web-area-action-choosers-design.md`
//! §3). `calculate_power` and `calculate_church_faith` are read for what is
//! DISPLAYED only, as `describe`'s Church arm reads them.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use yokohama_core::game::actions::{calculate_church_faith, chinatown_rate, ChinatownGood};
use yokohama_core::*;

use crate::describe::{acting_area, chinatown_exchange_text, church_kind_words, describe_move};
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

/// The listed move holding exactly `counts`, in whatever order the engine
/// listed it. The legal list decides: counts the list does not hold match
/// nothing.
pub fn chinatown_match(legal: &[Move], counts: [u8; 9]) -> Option<Move> {
    legal.iter().find(|mv| chinatown_counts(mv) == Some(counts)).cloned()
}

/// `legal` is walked ONCE into a map from count arrays to the first listed move
/// holding them; `can_add`, `offered`, `max_exchanges` and the preview are each
/// answered from that map, never from the stock, the rate or the power.
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
        max_exchanges: sets.keys().map(total).max().unwrap_or(0),
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
    let mut out = format!(
        "Chinatown — up to {} exchanges (add by number):\n",
        model.max_exchanges
    );
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

/// Test fixtures shared with other modules' cells (`pub(crate)` so `describe.rs`
/// and, later, `repl.rs` reach the same position).
#[cfg(test)]
pub(crate) mod test_support {
    use yokohama_core::*;

    /// A 2P position at the Church with 42 listed Church moves: the acting
    /// seat's president on the Church with 4 assistants there (power 5), stock
    /// copper 1, silk 5, tea 2, seafood 3, money 2, imported goods 1, the
    /// track's faith 4, 5 and 6 slots filled like the dummy's faith-7 slot, so
    /// only faith 8 and up reaches an empty space (three kinds or more).
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
        let dummy = state.church_track.slots[3].expect("the 2P dummy holds faith 7");
        for slot in 0..3 {
            state.church_track.slots[slot] = Some(dummy);
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
}

#[cfg(test)]
mod tests {
    use super::test_support::{
        issue_position, ordinary_position, ordinary_position_power_1, rich_position,
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
    }
}
