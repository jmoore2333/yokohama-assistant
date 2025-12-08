//! Yokohama board game core engine.
//!
//! This crate models the complete game rules for Yokohama,
//! a Meiji-era merchant trading game for 2-4 players.

pub mod types;
pub mod game;

pub use types::*;
pub use game::*;
