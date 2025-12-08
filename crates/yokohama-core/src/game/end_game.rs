//! End game trigger detection and final round handling.

use crate::types::*;

/// Check all end game triggers and update state if triggered.
pub fn check_end_game_triggers(state: &mut GameState) {
    if state.end_game_triggered {
        return; // Already triggered
    }

    // 1. Trading Houses: any player built all 4
    for p in &state.players {
        if p.trading_houses.total_built() >= 4 {
            trigger_end_game(state, EndGameTrigger::TradingHousesComplete(p.id), p.id);
            return;
        }
    }

    // 2. Shophouses: any player built all 8
    for p in &state.players {
        if p.shophouses.total_built() >= 8 {
            trigger_end_game(state, EndGameTrigger::ShophousesComplete(p.id), p.id);
            return;
        }
    }

    // 3. Order deck empty AND a Port slot is empty
    if state.order_deck_remaining == 0 {
        let port_has_empty_slot = check_port_empty_slot(state);
        if port_has_empty_slot {
            trigger_end_game(state, EndGameTrigger::OrderDeckEmpty, state.current_player);
            return;
        }
    }

    // 4. Church track: reached threshold
    let church_threshold = state.config.track_trigger();
    if state.church_track.total_assistants() >= church_threshold {
        trigger_end_game(state, EndGameTrigger::ChurchTrackFull, state.current_player);
        return;
    }

    // 5. Customs track: reached threshold
    let customs_threshold = state.config.track_trigger();
    if state.customs_track.total_assistants() >= customs_threshold {
        trigger_end_game(state, EndGameTrigger::CustomsTrackFull, state.current_player);
    }
}

fn trigger_end_game(state: &mut GameState, reason: EndGameTrigger, trigger_player: PlayerId) {
    state.end_game_triggered = true;
    state.trigger_reason = Some(reason);
    state.trigger_player = Some(trigger_player);
}

fn check_port_empty_slot(state: &GameState) -> bool {
    // Check Port A
    if state.port_state.port_a.low.is_none()
        || state.port_state.port_a.mid.is_none()
        || state.port_state.port_a.high.is_none()
    {
        return true;
    }

    // Check Port B if exists
    if let Some(ref port_b) = state.port_state.port_b {
        if port_b.low.is_none() || port_b.mid.is_none() || port_b.high.is_none() {
            return true;
        }
    }

    false
}

/// Determine if the game is completely over (final round finished).
pub fn is_game_over(state: &GameState) -> bool {
    if !state.end_game_triggered {
        return false;
    }

    if !state.final_round {
        // Need to complete current round first, then play final round
        return false;
    }

    // Final round complete when we've looped back after all players took a turn
    // The player before start player should be the last to play
    let player_before_start = PlayerId::new(
        (state.config.start_player.0 + state.config.player_count - 1) % state.config.player_count,
    );

    // Game is over if current player is at TurnEnd and is the player before start player
    state.current_player == player_before_start && state.turn_phase == TurnPhase::TurnEnd
}

/// Advance to final round after completing current round.
pub fn check_round_completion(state: &mut GameState) {
    if !state.end_game_triggered || state.final_round {
        return;
    }

    // Current round ends when we reach the player before start player at TurnEnd
    let player_before_start = PlayerId::new(
        (state.config.start_player.0 + state.config.player_count - 1) % state.config.player_count,
    );

    if state.current_player == player_before_start && state.turn_phase == TurnPhase::TurnEnd {
        state.final_round = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_trigger_by_player_count() {
        let config_2p = GameConfig::new(2, vec![PlayerColor::Red, PlayerColor::Blue], PlayerId::new(0));
        assert_eq!(config_2p.track_trigger(), 4);

        let config_3p = GameConfig::new(
            3,
            vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow],
            PlayerId::new(0),
        );
        assert_eq!(config_3p.track_trigger(), 5);
    }
}
