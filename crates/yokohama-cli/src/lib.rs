//! Phase-2 CLI game driver: Session/REPL/wizard over yokohama-core
//! (design docs/plans/2026-07-18-phase2-cli-driver-design.md).

pub mod board_render;
pub mod bot;
pub mod describe;
pub(crate) mod input;
pub mod render;
pub mod repl;
pub(crate) mod reveal;
pub mod screen;
pub mod screen_model;
pub mod session;
pub mod style;
pub mod suggest;
pub mod wizard;
