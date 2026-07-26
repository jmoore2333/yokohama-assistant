//! Game rules engine for Yokohama.

mod power;
mod movement;
mod legal_moves;
mod apply_move;
mod cards;
mod data;
mod achievements;
mod tech_effects;
mod end_game;
mod scoring;
mod setup;
mod board_gen;
mod rules;
mod display;
mod invariants;
mod record;
mod choices;
mod semantics;
mod observe;
mod belief;
mod agent;
mod search_actions;
mod route_enum;
mod move_legality;
mod seeds;
mod metrics;
mod policy;
mod arena;
pub mod actions;
/// Evaluator v1 (Phase 3.3 T1). NAMESPACED deliberately — the crate glob
/// re-exports every `pub use x::*` below to the crate root, and `score`/
/// `explain` are far too generic to live there (`yokohama_core::score` says
/// nothing about what it scores). Callers write `eval::score(&view, ..)`, the
/// same shape `actions` above uses; `pub use game::*` in `lib.rs` still makes
/// the MODULE reachable as `yokohama_core::eval`.
pub mod eval;

pub use power::*;
pub use movement::*;
pub use legal_moves::*;
pub use apply_move::*;
pub use cards::*;
pub use achievements::*;
pub use tech_effects::*;
pub use end_game::*;
pub use scoring::*;
pub use setup::*;
pub use board_gen::*;
pub use rules::*;
pub use display::*;
pub use invariants::*;
pub use record::*;
pub use choices::*;
pub use semantics::*;
pub use observe::*;
pub use belief::*;
pub use agent::*;
pub use search_actions::*;
pub use route_enum::*;
pub use move_legality::*;
pub use seeds::*;
pub use metrics::*;
pub use policy::*;
pub use arena::*;
