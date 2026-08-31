//! Phase-2 CLI game driver: Session/REPL/wizard over yokohama-core
//! (design docs/plans/2026-07-18-phase2-cli-driver-design.md).

pub mod bga;
pub mod board_render;
pub mod bot;
pub mod card_prompt;
pub mod card_render;
pub mod decision_render;
pub mod describe;
pub mod index_render;
pub mod input;
pub mod panel_render;
pub mod render;
pub mod repl;
pub(crate) mod reveal;
pub mod screen;
pub mod screen_model;
pub mod session;
pub mod style;
pub mod suggest;
pub mod view_render;
pub mod why;
pub mod wizard;
