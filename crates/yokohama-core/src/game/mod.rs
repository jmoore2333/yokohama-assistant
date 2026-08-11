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
mod budget;
mod arena;
pub mod actions;
/// Evaluator v1 (Phase 3.3 T1). NAMESPACED deliberately — the crate glob
/// re-exports every `pub use x::*` below to the crate root, and `score`/
/// `explain` are far too generic to live there (`yokohama_core::score` says
/// nothing about what it scores). Callers write `eval::score(&view, ..)`, the
/// same shape `actions` above uses; `pub use game::*` in `lib.rs` still makes
/// the MODULE reachable as `yokohama_core::eval`.
pub mod eval;
/// Greedy-v1, the one-ply evaluator rung (Phase 3.3 T3). NAMESPACED for the
/// same reason [`eval`] is: the crate glob re-exports every `pub use x::*`
/// below to the crate root, and `candidates` is far too generic to live there.
/// Callers write `greedy::GreedyAgent` / `greedy::candidates`.
pub mod greedy;
/// Lookahead-v1, the shallow-search rung (Phase 3.3 T5). NAMESPACED for the
/// same reason [`eval`] and [`greedy`] are: `search_world`, `SearchStats` and
/// `TieSite` are far too generic for the crate root. Callers write
/// `lookahead::LookaheadAgent` / `lookahead::search_world`.
pub mod lookahead;
/// MCTS-v1, the UCT contestant of the Phase 3.4 architecture experiment.
/// NAMESPACED for the same reason [`eval`], [`greedy`] and [`lookahead`] are —
/// and here it is load-bearing rather than merely tidy: this module's
/// `search_world` and `squash` would COLLIDE with `lookahead::search_world` at
/// the crate root. Callers write `mcts::MctsAgent` / `mcts::search_world`.
pub mod mcts;
/// The candidate-vs-incumbent WEIGHT SCREEN (Phase 3.5c, evaluator-v2 protocol
/// §4.2's L4). NAMESPACED for the same reason [`eval`], [`greedy`], [`lookahead`]
/// and [`mcts`] are: `run_screen`, `ScreenSpec` and `NullPosition` say nothing
/// about weights at the crate root. Callers write `eval_screen::run_screen`.
///
/// It is a SIBLING of [`arena`] rather than a submodule of it deliberately —
/// see its own module doc: the arena's cell grammar carries no weight knob BY
/// DESIGN (L12), and this instrument exists precisely to run the comparison that
/// grammar refuses to express.
pub mod eval_screen;

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
pub use budget::*;
pub use arena::*;
