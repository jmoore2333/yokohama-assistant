# Yokohama AI Assistant

## Current status (2026-07-20): engine verified DIFFERENTIALLY — an independent rules oracle agrees with it everywhere

Four Rust workspace crates:

- **`yokohama-core`** — the complete engine: full rules-correct games at 2–4p, checked
  `apply_move` boundary (engine is the sole legality authority), seeded setup, save/replay
  (`GameRecord` with `rules_hash`, `semantics_rev`, and a terminal-state digest verified by
  `replay_strict`), history-free `GameState` cheap to clone for future search.
- **`yokohama-cli`** — the Phase-2 live-tracking driver: full simulated games today; wizard
  setup, numbered legal options + placement/assistant/retreat grammars, reveal choreography
  with a desync tripwire, save/load/undo, TTY color board. Tester guide:
  [`docs/cli-guide.md`](docs/cli-guide.md).
- **`yokohama-oracle`** — an **independently-authored rules oracle** (Phase 2.9): legal-move
  enumeration, final scoring + tie-break, and a prune-free route reference, written from the
  rulebook/KB/decision-log only (types-only dependency on the engine, mechanically audited).
- **`yokohama-difftest`** — the differential harness that compares them: per-decision-point
  sweeps under two policies, terminal scoring comparison, three-way route-pruning check,
  self-contained divergence artifacts with a `repro` tool.

**Phase 2.9 (2026-07-20) converged**: engine vs oracle agree on every legal-move set (240/240
games after fixes), every route label (7,272 decision points + adversarial batteries), and
every final score (540 games). The campaign found and fixed 4 engine bugs + 1 oracle bug that
self-play fuzzing structurally could not see, hardened every raw apply arm against ill-formed
payloads, mutation-proved the invariant suite, and produced an owner ruling list
([`docs/plans/2026-07-20-phase-2.9-ruling-list.md`](docs/plans/2026-07-20-phase-2.9-ruling-list.md)).
Full campaign report: [`docs/plans/2026-07-20-phase-2.9-final-report.md`](docs/plans/2026-07-20-phase-2.9-final-report.md).

- **Authoritative status + roadmap + every rules ruling:** [`guidingpath.md`](guidingpath.md)
- **Gates** — per-edit: `cargo test -p yokohama-core -p yokohama-cli -p yokohama-oracle -p yokohama-difftest`
  + `cargo clippy --workspace --all-targets -- -D warnings` · pre-merge release tier (~75s):
  fuzz `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored` · choice
  census `...--test choice_encoding -- --ignored` · differential
  `cargo test --release -p yokohama-difftest --test differential -- --ignored` · routes3 + scoring likewise.
- **Next:** the live physical-game tracking session (Phase-2 gate, scheduling is Jeremy's),
  then **Phase 3 — the computer player** (the differential harness is its regression net; the
  three mandatory handover items are recorded in `guidingpath.md`).
- Priority order (reset July 2026): **1)** engine-validated live-game tracking via CLI →
  **2)** a computer player that provably plays to win → **3)** computer vision, deferred.

> **Historical note:** everything below this line describes the original 2025 AR-first vision
> and its status tables — kept for the record; superseded by `guidingpath.md`.

An AR-powered AI assistant for the board game **Yokohama** that observes the physical tabletop, extracts game state, and provides real-time strategic recommendations.

## Project Vision

- **Overhead camera** observes the physical game board
- **Cloud VLM** (Gemini/Claude) extracts game state from images
- **MCTS AI** running on local hardware provides move recommendations
- **Web UI** displays suggestions and game state

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Camera    │────▶│  Python API │────▶│   Web UI    │
│  (overhead) │     │  (FastAPI)  │     │  (React)    │
└─────────────┘     └──────┬──────┘     └─────────────┘
                          │
                    ┌─────▼─────┐
                    │ Cloud VLM │
                    │(Gemini/   │
                    │ Claude)   │
                    └─────┬─────┘
                          │
                    ┌─────▼─────┐
                    │Rust Engine│
                    │(yokohama- │
                    │  core)    │
                    └───────────┘
```

## Current Status

| Module | Description | Status |
|--------|-------------|--------|
| **A1** | Game Engine | 🔄 Logic complete, data collection in progress |
| **A2** | MCTS Search | ⏳ Blocked by A1 |
| **A3** | Heuristic Evaluator | ⏳ Blocked by A1 |
| **B1** | VLM Prompts | ⏳ Not started |
| **C1** | Web UI | ⏳ Not started |

### A1 Progress Detail

| Component | Status | Notes |
|-----------|--------|-------|
| Core Types | ✅ Done | `types/*.rs` |
| Power Calculation | ✅ Done | `game/power.rs` |
| Movement & Pathfinding | ✅ Done | `game/movement.rs` |
| Legal Move Generation | ✅ Done | `game/legal_moves.rs` |
| Apply Move | ✅ Done | `game/apply_move.rs` |
| Scoring | ✅ Done | `game/scoring.rs` |
| Tech Effects | ✅ Done | `game/tech_effects.rs` |
| End Game Triggers | ✅ Done | `game/end_game.rs` |
| Board Generation | ✅ Done | `game/board_gen.rs` |
| Area Tiles | ✅ Verified | 10/14/18 for 2P/3P/4P |
| **Order Cards (36)** | 📷 Extracted | Need transcription |
| **Tech Cards (20)** | 📷 Extracted | Need transcription |
| **Building Sites (24)** | 📷 Extracted | Need transcription |
| **5-Power Tokens** | ⚠️ Incomplete | Only 8/17 found in TTS |

**Tests:** 153 passing (147 unit + 6 integration)

## What's Blocking A2/A3

The game engine logic is complete. TTS extraction is done, but we need to:

| Task | Status | Blocker |
|------|--------|---------|
| Verify 5-power tokens | ⚠️ Blocking | Only 8/17 found in TTS - need physical game |
| Transcribe Order Cards | ⏳ Pending | 36 cards from sprite sheet |
| Transcribe Tech Cards | ⏳ Pending | 20 cards from sprite sheet |
| Transcribe Building Sites | ⏳ Pending | 24 cards from sprite sheet |

**TTS Assets Location:** `tts-assets/organized/`

See [`docs/plans/2025-12-07-A1-data-collection.md`](docs/plans/2025-12-07-A1-data-collection.md) for the full extraction plan.

## Repository Structure

```
yokohama/
├── README.md                          # This file
├── agent-handover.md                  # Session context for AI assistants
├── Cargo.toml                         # Workspace root
├── crates/
│   └── yokohama-core/                 # Rust game engine
│       ├── src/
│       │   ├── lib.rs
│       │   ├── types/                 # Core type definitions
│       │   │   ├── mod.rs
│       │   │   ├── ids.rs             # PlayerId, AreaId, etc.
│       │   │   ├── board.rs           # BoardLayout, AreaType
│       │   │   ├── cards.rs           # OrderCard, TechCard, TechEffect
│       │   │   ├── player.rs          # PlayerState, resources
│       │   │   └── state.rs           # GameState, GameConfig
│       │   └── game/                  # Game logic
│       │       ├── mod.rs
│       │       ├── power.rs           # Power calculation
│       │       ├── movement.rs        # Pathfinding, tech integration
│       │       ├── legal_moves.rs     # Move generation
│       │       ├── apply_move.rs      # State transitions
│       │       ├── board_gen.rs       # Board layout generation
│       │       ├── cards.rs           # Order/Tech registries
│       │       ├── achievements.rs    # Achievement validation
│       │       ├── tech_effects.rs    # Technology effects
│       │       ├── scoring.rs         # Final scoring
│       │       ├── end_game.rs        # End game triggers
│       │       ├── setup.rs           # Game setup
│       │       └── actions/           # Area-specific actions
│       └── tests/
│           └── integration.rs
├── analysis-1/
│   ├── 00-yokohama-rulebook.pdf       # Official rules
│   └── yokohama-kb/                   # Knowledge base
│       ├── 01-rules-reference.md
│       ├── 02-quick-reference.md
│       └── ...
└── docs/plans/
    ├── 2025-12-07-A1-implementation.md
    ├── 2025-12-07-A1-verification.md
    └── 2025-12-07-A1-data-collection.md  # Current focus
```

## Quick Start

```bash
# Run tests
cargo test -p yokohama-core

# Run specific test module
cargo test -p yokohama-core board_gen

# Check test count
cargo test -p yokohama-core -- --list 2>&1 | grep -c "test"
```

## Key Documents

| Document | Purpose |
|----------|---------|
| [`agent-handover.md`](agent-handover.md) | Context for AI coding assistants |
| [`docs/plans/2025-12-07-A1-data-collection.md`](docs/plans/2025-12-07-A1-data-collection.md) | Current work: extracting game data |
| [`analysis-1/yokohama-kb/01-rules-reference.md`](analysis-1/yokohama-kb/01-rules-reference.md) | Complete game rules reference |
| [`analysis-1/yokohama-kb/02-quick-reference.md`](analysis-1/yokohama-kb/02-quick-reference.md) | Power/reward tables for quick lookup |

## Next Steps

1. **Complete TTS extraction** - Get all card/token images and data
2. **Transcribe to YAML** - Document all game components
3. **Implement data registries** - `OrderRegistry::standard()`, etc.
4. **Verify tile counts** - Confirm 10/14/18 vs 12/15/18 discrepancy
5. **Begin A2 (MCTS)** - Once A1 data is complete

## Hardware Target

- **CPU:** Xeon
- **GPU:** RTX 5000
- **RAM:** 64GB
- **Performance goal:** 10,000+ `apply_move` calls/second for MCTS
