//! Pending-choice queue mechanics (design 2026-07-18 §5.1).
use crate::types::*;
use crate::game::*;

/// Resolution moves for the HEAD pending choice. Non-empty by construction
/// (every choice type offers decline or has >=1 outcome, else it was
/// auto-resolved at push time).
pub fn generate_choice_resolutions(_state: &GameState, head: &PendingChoice, _rules: &Rules) -> Vec<Move> {
    match head {
        PendingChoice::AnyGood => vec![
            Move::ChooseAnyGood { good: TradeGood::Copper },
            Move::ChooseAnyGood { good: TradeGood::Silk },
            Move::ChooseAnyGood { good: TradeGood::Tea },
            Move::ChooseAnyGood { good: TradeGood::Seafood },
        ],
    }
}

/// Push a choice, auto-resolving singletons inline (design §5.1). For
/// AnyGood the option set is always 4, so it always queues.
pub fn push_choice(state: &mut GameState, choice: PendingChoice, _rules: &Rules) {
    state.pending_choices.push(choice);
}

/// Apply a Choose* move against the head pending choice. Validates the move
/// is a resolution OF the head (type + membership in the generated set),
/// pops the head, applies the effect. Zero mutation on error.
pub fn resolve_choice_move(state: &mut GameState, mv: &Move, rules: &Rules) -> Result<(), MoveError> {
    let head = state.pending_choices.first().cloned()
        .ok_or(MoveError::RuleViolation("No pending choice to resolve"))?;
    if !generate_choice_resolutions(state, &head, rules).contains(mv) {
        return Err(MoveError::RuleViolation("Move does not resolve the pending choice"));
    }
    state.pending_choices.remove(0);
    let player = state.current_player;
    match (head, mv) {
        (PendingChoice::AnyGood, Move::ChooseAnyGood { good }) => {
            let mining = match good {
                TradeGood::Copper => copper_bonus(state, player, 1, &rules.techs),
                _ => 0,
            };
            let p = state.player_mut(player);
            match good {
                TradeGood::Copper => p.copper += 1 + mining as u32,
                TradeGood::Silk => p.silk += 1,
                TradeGood::Tea => p.tea += 1,
                TradeGood::Seafood => p.seafood += 1,
            }
        }
        _ => unreachable!("membership check above guarantees type match"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::create_test_game_state;

    #[test]
    fn generate_choice_resolutions_any_good_offers_all_four_goods() {
        let state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        let moves = generate_choice_resolutions(&state, &PendingChoice::AnyGood, &rules);
        assert_eq!(moves, vec![
            Move::ChooseAnyGood { good: TradeGood::Copper },
            Move::ChooseAnyGood { good: TradeGood::Silk },
            Move::ChooseAnyGood { good: TradeGood::Tea },
            Move::ChooseAnyGood { good: TradeGood::Seafood },
        ]);
    }

    #[test]
    fn push_choice_appends_to_queue_fifo() {
        let mut state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        push_choice(&mut state, PendingChoice::AnyGood, &rules);
        push_choice(&mut state, PendingChoice::AnyGood, &rules);
        assert_eq!(state.pending_choices, vec![PendingChoice::AnyGood, PendingChoice::AnyGood]);
    }

    #[test]
    fn resolve_choice_move_errors_when_queue_empty() {
        let mut state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        let result = resolve_choice_move(&mut state, &Move::ChooseAnyGood { good: TradeGood::Copper }, &rules);
        assert!(result.is_err());
        assert!(state.pending_choices.is_empty());
    }

    #[test]
    fn resolve_choice_move_pops_head_and_applies_chosen_good() {
        let mut state = create_test_game_state(2);
        let rules = Rules::empty_for_tests();
        push_choice(&mut state, PendingChoice::AnyGood, &rules);
        let tea_before = state.current_player().tea;
        resolve_choice_move(&mut state, &Move::ChooseAnyGood { good: TradeGood::Tea }, &rules).unwrap();
        assert!(state.pending_choices.is_empty());
        assert_eq!(state.current_player().tea, tea_before + 1);
    }
}
