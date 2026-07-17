//! Typed errors for move application.

use serde::{Deserialize, Serialize};

/// Why a move could not be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveError {
    /// Move enum variant does not match the current TurnPhase.
    WrongPhase,
    NotEnoughMoney,
    NotEnoughAssistants,
    NotEnoughResources,
    HandLimitExceeded,
    CardNotFound,
    InvalidSlot,
    IllegalDestination,
    /// Catch-all carrying the legacy message; migrate call sites to
    /// specific variants opportunistically when touching them.
    RuleViolation(&'static str),
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveError::WrongPhase => write!(f, "move not legal in current phase"),
            MoveError::NotEnoughMoney => write!(f, "not enough money"),
            MoveError::NotEnoughAssistants => write!(f, "not enough assistants"),
            MoveError::NotEnoughResources => write!(f, "not enough resources"),
            MoveError::HandLimitExceeded => write!(f, "order hand limit (3) exceeded"),
            MoveError::CardNotFound => write!(f, "card not found"),
            MoveError::InvalidSlot => write!(f, "invalid slot"),
            MoveError::IllegalDestination => write!(f, "illegal destination"),
            MoveError::RuleViolation(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for MoveError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_error_displays_reason() {
        let e = MoveError::RuleViolation("Not enough assistants in hand");
        assert_eq!(format!("{}", e), "Not enough assistants in hand");
        assert_eq!(format!("{}", MoveError::WrongPhase), "move not legal in current phase");
    }
}
