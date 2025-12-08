# Yokohama AI Assistant

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
| **Order Cards Data** | 🔄 Extracting | ~36 cards from TTS |
| **Tech Cards Data** | 🔄 Extracting | Country pairings needed |
| **Building Sites Data** | 🔄 Extracting | VP configurations |
| **5-Power Tokens Data** | 🔄 Extracting | Types and counts |

**Tests:** 153 passing (147 unit + 6 integration)

## What's Blocking A2/A3

The game engine logic is complete, but we need **concrete game data** before MCTS can run realistic simulations:

| Data Needed | Why It Matters | Source |
|-------------|----------------|--------|
| Order Cards | Can't simulate order fulfillment | TTS extraction |
| Tech Card Countries | Can't calculate mercenary eligibility | TTS extraction |
| Building Site VP Values | Can't score building actions | TTS extraction |
| 5-Power Token Effects | Can't resolve 5-power bonuses | TTS extraction |

**Current approach:** Extracting all assets from a Tabletop Simulator mod.

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
