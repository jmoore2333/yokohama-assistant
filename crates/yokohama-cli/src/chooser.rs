//! The area-action choosers' models (cycle 2, plan 2a): the Church's here, as
//! a pure module over the legal list. Nothing in this file decides legality:
//! whether a kind is offered, whether a selection can be confirmed and which
//! faiths the list holds are each answered by finding listed moves among the
//! ones handed in (`docs/plans/2026-11-21-web-area-action-choosers-design.md`
//! §3). `calculate_power` and `calculate_church_faith` are read for what is
//! DISPLAYED only, as `describe`'s Church arm reads them.

use serde::{Deserialize, Serialize};
use yokohama_core::game::actions::calculate_church_faith;
use yokohama_core::*;

use crate::describe::{acting_area, church_kind_words, describe_move};
use crate::screen_model::CHURCH_YEN_COST;

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
}

#[cfg(test)]
mod tests {
    use super::test_support::issue_position;
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
}
