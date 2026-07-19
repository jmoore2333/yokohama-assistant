# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Rust engine (plus planned CLI tracker and AI player) for the board game **Yokohama** (Hisashi Hayashi, 2016). Priority order: (1) engine-validated move-by-move tracking of live games via CLI, (2) a computer player that provably plays to win, (3) computer vision — deferred.

**Read `guidingpath.md` first — it is the authoritative status and roadmap** (its decision log carries every rules ruling). The status tables in README.md and agent-handover.md are historical and overstate completion.

**Engine-completion is COMPLETE (2026-07-17) on branch `engine-completion`** — the fuzz gate passed (15,000/15,000 random self-play games, per-move invariants; see guidingpath.md's Current state). **The Phase-2 CLI driver LANDED (2026-07-18) on branch `phase2-cli-driver`**: full simulated games play through the CLI with the complete live-tracking machinery (record v2 reveals, setup reconciliation, desync guard). **CHOICE ENCODING LANDED (2026-07-19) on branch `choice-encoding`** (plan `docs/plans/2026-07-18-choice-encoding-plan.md`, execution ledger `.superpowers/sdd/progress.md`): every v1 auto-resolution is a real queued choice (`pending_choices` on `GameState`, queue-first `get_legal_moves`, singleton auto-resolve with a pinned Decline invariant), the wizard validates at parse time with unlimited re-ask + review/edit, and the Canal is crossable — the PHASE gate (live-tracking a physical game) is UNBLOCKED and awaits Jeremy's scheduling. Untracked `.codex/`, `.vscode/`, `AGENTS.md` belong to Jeremy — never add or commit them.

## Commands

Toolchain is FIXED (2026-07-17): plain `cargo` works. (History: `~/.cargo/bin/rustup` was a dangling symlink; the durable fix was copying the real Mach-O from `/opt/homebrew/Cellar/rustup/*/libexec/bin/rustup` into `~/.cargo/bin/rustup` — Homebrew's `/opt/homebrew/bin/rustup` is a bash wrapper that resets argv[0] and breaks proxy multiplexing, so never symlink to it.)

- All tests: `cargo test -p yokohama-core` (466 passing as of 2026-07-19 post fix wave: 434 lib + 12 choice_encoding + 20 other integration, zero warnings) and `cargo test -p yokohama-cli` (85 passing)
- Lint gate (adopted 2026-07-19): `cargo clippy --workspace --all-targets -- -D warnings` must pass. rustfmt is deliberately NOT adopted (would reformat the whole repo, ~1k diff blocks — revisit at a phase boundary; don't run `cargo fmt` casually)
- Run the CLI driver: `cargo run -p yokohama-cli` (setup wizard → REPL; sim or live tracking); `cargo run -p yokohama-cli -- <save.json>` resumes a saved game
- Fuzz regression gate (run in release before merges): `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored` (1,500 games, ~8s)
- Full self-play sweep: `cargo run --release -p yokohama-core --example selfplay -- 5000 <players> <base_seed>`
- Single test: `cargo test -p yokohama-core test_name`
- One module: `cargo test -p yokohama-core board_gen`
- Regenerate card data after YAML changes: `cargo run -p gen-data` (rewrites `crates/yokohama-core/src/game/data.rs`; never hand-edit it)

## Architecture

Workspace crates: `crates/yokohama-core` (library), `crates/yokohama-cli` (Phase-2 game driver: `describe`/`render`/`session`/`repl`/`wizard` + thin `main`), and `crates/gen-data` (YAML→Rust data generator tool).

- `types/` — serde-serializable state. `GameState` is complete-and-resumable and **history-free by design** — it must stay cheap to clone for future MCTS; move history belongs in an external action log, never in state. `Move`/`AreaAction` enums in `types/moves.rs`. Typed `MoveError` in `types/errors.rs`.
- `game/` — pure functions over state: `get_legal_moves(&GameState, &Rules) -> Vec<Move>` and `apply_move(&GameState, &Move, &Rules) -> Result<GameState, MoveError>` (clones, never mutates). `Rules` (`game/rules.rs`) bundles static data: `Rules::standard()` from generated registries, `Rules::empty_for_tests()` for fixtures. Setup: seeded `new_game(config, rules, seed) -> PendingSetup` + `finalize_setup(pending, choices)`; decks live IN `GameState` (`order_deck`/`tech_deck`, last element = top; `orders_removed` for conservation).
- A turn is a phase state machine (`TurnPhase`): AdditionalActionFirst → Placement → Movement → AreaAction → FivePowerBonus → Building → Recovery (automatic) → AdditionalActionSecond → TurnEnd. Mercenary use runs a nested sub-turn (AreaAction→…→Recovery) — see `is_mercenary_action`.
- Hidden information is exactly one thing: each player's starting order (`starting_order`, or `starting_order_hidden` for live-tracked opponents — both dealt cards then sit in `orders_removed`; reveals use `extract_order_from_deck`, which searches deck then removed pile).
- Static card data: source of truth is `analysis-1/yokohama-kb/07..09-*.yaml` (**verified 2026-07-17**: 36 orders keyed by printed corner index, 20 tech titles × `copies:` = 30 physical cards, 24 sites with TL/TR/ML/MR `SpaceReward` spaces). `gen-data` emits `game/data.rs`.
- Boards (verified from board art 2026-07-17): port boards have SIX card slots (2 per LOW/MID/HIGH zone), lab boards SIX tech slots (surcharges +1,+1,+2,+2 on slots 3–6), customs board 7 spaces with printed costs [1,1,2,2,3,3,4], church board faith 4–11.

## Rules knowledge

- Complete reference: `analysis-1/yokohama-kb/01-rules-reference.md` (being corrected as board-art verification finds errors — the decision log lists corrections); rulebook PDF in `analysis-1/`; icon glossary is rulebook p.10.
- Commonly-missed rules that have bitten this project: Trading House passive income (owner gets 1 Yen when another player acts in that area); movement-icon rewards move *assistants* via queued declinable choices — there is NO 5-Power "Movement token" (box-verified 2026-07-18: no Movement or VP faces exist in the 20-token pool); mercenaries bypass opponent Presidents (no blocking, no fees); the President may not return to its starting area in the same turn; placement fees are 1 Yen PER ASSISTANT PLACED to each president owner (Option B's 2-into-one-area costs 2, not 1); movement pass-through fees are paid TO the president owners (1 Yen each), not the bank; Trading House and Shophouse both require Power 4+ (no separate higher gate for Trading House — corrected 2026-07-17, final-review finding C3); a player may only own 1 Technology card of the same name (10 of the 20 titles circulate as 2 physical copies so different players can each own one); trading-house tokens go warehouse→hand at the Employment Agency (printed 4/5/6/7 take progression) and build FREE from hand; port refill is rightmost-first after total left-compaction; Port at 5 Power also grants +3 VP; Chinatown exchange count is capped by action power; Movement Option B is legal with the President already in hand.
- Church faith = power LINEARLY (1,2,3,4,5) plus 1 per distinct payment type (max 1 each of copper/silk/tea/seafood/yen/import). The rulebook text's "2 Power: 1 Faith" is a TYPO (Jeremy-adjudicated 2026-07-17); the engine's `calculate_church_faith` is corrected in Task 13.
- Choice encoding LANDED (2026-07-19) — the former v1 auto-resolutions are real queued choices: any-good picks (no decline — gain is unconditional), warehouse-icon takes (Decline always offered; priced per the 4/5/6/7 progression), movement-icon assistant moves (Decline always offered per icon), church/customs space choice (queued only when ≥2 spaces eligible — exactly one place inline), Telephone's Recovery leave-one (may keep a LONE assistant — recovers zero at count 1), University/Spinning Mill as standing additional-action options. Singleton resolution sets auto-resolve silently and are structurally always no-op Declines (pinned invariant — CLI delta windows depend on it).
- Canal (box-ruled 2026-07-18/19): crossable for President movement via colinear through-lines only — (L,R), (U,D), and both diagonals, each only where both flank tiles exist (a 2-row grid has just the horizontal pair); 1 Yen per crossing to the BANK (not a player); bent transits are structurally impossible; the Canal itself is never placeable and Telegram's adjacent-bonus placement excludes crossing pairs (transit-only adjacency).

## Conventions

- TDD: failing test first. The repo's commit history is strictly TDD — keep it that way. Suite must stay green + zero warnings after every task.
- Design docs and implementation plans are dated files in `docs/plans/` (`YYYY-MM-DD-topic.md`). Workflow: brainstorm → spec → implementation plan → execute.
- Strong typing at boundaries: IDs have `try_new`/`TryFrom` for external input; bare `new` is for internal logic only.
- Update `guidingpath.md` at phase transitions and for new design decisions; never claim a phase complete unless its executable gate passes (for the engine: the self-play fuzz suite).
