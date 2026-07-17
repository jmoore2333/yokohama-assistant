# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Rust engine (plus planned CLI tracker and AI player) for the board game **Yokohama** (Hisashi Hayashi, 2016). Priority order: (1) engine-validated move-by-move tracking of live games via CLI, (2) a computer player that provably plays to win, (3) computer vision — deferred.

**Read `guidingpath.md` first — it is the authoritative status and roadmap.** The status tables in README.md and agent-handover.md are historical and overstate completion (they claim the engine is done; Port/Lab/Church/Customs are stubbed in `apply_move.rs` and the card registries are empty).

## Commands

`cargo` is not on PATH on this machine — every `~/.cargo/bin` entry is a broken symlink (homebrew rustup artifact). Until fixed (Part A of the engine-completion spec), invoke the toolchain directly:

```sh
TCBIN="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
PATH="$TCBIN:$PATH" "$TCBIN/cargo" test -p yokohama-core
```

- All tests: `cargo test -p yokohama-core` (156 passing as of 2026-07-16)
- Single test: `cargo test -p yokohama-core test_name`
- One module: `cargo test -p yokohama-core board_gen`

Do **not** prepend `~/.cargo/bin` to PATH while the symlinks are broken — its dangling `rustc` shadows the real one and cargo dies with "could not execute process `rustc -vV`".

## Architecture

One workspace crate, `crates/yokohama-core` (library only, no binaries yet):

- `types/` — serde-serializable state. `GameState` is complete-and-resumable and **history-free by design** — it must stay cheap to clone for future MCTS; move history belongs in an external action log, never in state. `Move`/`AreaAction` enums in `types/moves.rs`.
- `game/` — pure functions over state: `legal_moves.rs` (`get_legal_moves(&GameState) -> Vec<Move>`) and `apply_move.rs` (`apply_move(&GameState, &Move) -> Result<GameState, _>` — clones, never mutates the input). Supporting modules: power, movement (pathfinding + tech effects), scoring, end_game, tech_effects, board_gen, setup; per-area action helpers in `game/actions/`.
- A turn is a phase state machine (`TurnPhase`): AdditionalActionFirst → Placement → Movement → AreaAction → FivePowerBonus → Building → Recovery (automatic) → AdditionalActionSecond → TurnEnd. Mercenary use runs a nested sub-turn (AreaAction→…→Recovery) that returns to the additional-action phase — see the `is_mercenary_action` fields on `GameState`.
- Hidden information is exactly one thing: each player's starting order card (`PlayerState.starting_order`). Everything else is public — this drives all AI design (determinization suffices).
- Static card data: source of truth is `analysis-1/yokohama-kb/07..09-*.yaml` (36 orders / 20 techs / 24 building sites, pending verification). Registries (`OrderRegistry`, `TechRegistry`) are currently empty shells; they are being filled via generated code per the engine-completion spec — never hand-edit generated data files.

## Rules knowledge

- Complete reference: `analysis-1/yokohama-kb/01-rules-reference.md`; power/reward tables: `02-quick-reference.md`; official rulebook PDF in `analysis-1/`.
- Commonly-missed rules that have bitten this project before: Trading House passive income (owner gets 1 Yen when another player acts in that area); the 5-Power Movement token moves *assistants*, not the President; mercenaries bypass opponent Presidents (no blocking, no fees); Church at power 2 gives only 1 Faith; the President may not return to its starting area in the same turn; placement costs 1 Yen per area containing an opponent President; Trading House requires Power 5, Shophouse 4+.

## Conventions

- TDD: failing test first. The repo's commit history is strictly TDD — keep it that way.
- Design docs and implementation plans are dated files in `docs/plans/` (`YYYY-MM-DD-topic.md`). Workflow: brainstorm → spec → implementation plan → execute.
- Strong typing at boundaries: IDs have `try_new`/`TryFrom` for external input; bare `new` is for internal logic only.
- Update `guidingpath.md` at phase transitions and for new design decisions; never claim a phase complete unless its executable gate passes (for the engine: the self-play fuzz suite).
