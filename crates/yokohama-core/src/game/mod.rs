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
/// The DRIVEN-FIXTURE harness (Phase B item 1): rooted runs from a supplied
/// `GameState`. NAMESPACED for the same reason [`eval`], [`greedy`],
/// [`lookahead`], [`mcts`] and [`eval_screen`] are — `SeatSpec`, `ReplyPolicy`
/// and `drive_to_terminal` say nothing about ROOTED PLAY at the crate root, and
/// `registry` (T3's entry point) would be indefensible there. Callers write
/// `driven::drive_to_terminal`.
///
/// A SIBLING of [`arena`] rather than a submodule, like [`eval_screen`]: the
/// arena's subject is a from-setup CELL and its unit tabulation, and this
/// module's is a single position. What they SHARE is the play loop itself —
/// `driven` seats its agents in the arena's own bench and drives the arena's own
/// loop, which is the property its reproduction pin exists to keep true.
pub mod driven;
/// [`driven`]'s LAYER-2 READ MACHINERY (Phase C): the turn predicates, the
/// DEAD-root classifier, root admission and the binary read. NAMESPACED for the
/// same reason its subject is — `admission`, `family_read` and `PassRule` say
/// nothing about DRIVEN FAMILIES at the crate root.
///
/// SEPARATE from [`driven`] because the two answer different questions over one
/// instrument: `driven` answers "what happens from this position", and this
/// answers "does the subject's behaviour here satisfy a DECLARED RULE". Keeping
/// the rules in one file is what lets a reader check all of them at once against
/// the freeze document. Callers write `driven_families::family_read`.
pub mod driven_families;
/// [`driven`]'s EXACT adjudicator: an exhaustive perfect-information solve of
/// the remaining game at an IN-FINAL-ROUND root, where `driven`'s own
/// `adjudicate_root` can only play the remainder out under named policies.
/// SEPARATE from `driven` because it answers a different question — what the
/// position is worth under best play, rather than what those policies reach —
/// and its every number is a PRIVILEGED read that must never reach an
/// evaluator. Callers write `driven_exact::exact_table`.
pub mod driven_exact;
/// [`driven`]'s RUNNER: the argv grammar and the report rendering behind
/// `examples/driven.rs`. NAMESPACED for the same reason its subject is, and
/// SEPARATE from `driven` itself because the harness answers questions while
/// this only decides which question was asked and how to lay the answer out.
///
/// It lives in the LIB rather than in the example because an example's
/// `#[cfg(test)]` module runs under no command this repo documents — so a
/// grammar and a renderer written there would be untested by every per-edit
/// gate. Callers write `driven_runner::run`.
pub mod driven_runner;

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
