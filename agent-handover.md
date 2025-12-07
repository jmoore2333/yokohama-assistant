# Agent Handover: Yokohama AI Assistant

**Last Updated:** 2025-12-07
**Session Summary:** Completed comprehensive design phase through collaborative brainstorming

---

## Project Overview

Building an AR assistant for the board game **Yokohama** that:
- Observes physical tabletop via overhead camera
- Extracts game state using cloud VLM (Gemini/Claude)
- Provides real-time strategic recommendations via MCTS AI
- Runs on local hardware (Xeon + RTX 5000 + 64GB RAM)

**Key insight discovered:** Yokohama is nearly perfect information - the AI knows ALL players' Order cards (except each player's starting card). This dramatically simplifies the AI architecture.

---

## Repository State

```
/home/jmoore/Documents/yokohama/
├── analysis-1/
│   ├── 00-yokohama-rulebook.pdf      # Official game rules (16 pages)
│   ├── AI Yokohama Board Game Assistant.md  # Initial feasibility analysis
│   └── claude ai yokohama analysis.md       # Technical analysis (outdated CV assumptions)
├── docs/plans/
│   └── 2025-12-07-yokohama-assistant-design.md  # MAIN DESIGN DOCUMENT
└── agent-handover.md                 # This file
```

**Primary reference:** `docs/plans/2025-12-07-yokohama-assistant-design.md` - comprehensive technical design covering architecture, components, and implementation modules.

---

## Key Design Decisions

### Architecture
- **Rust core** (`yokohama-core`) for game engine + MCTS (Python GIL is bottleneck for parallel search)
- **Python API layer** (FastAPI) for HTTP/WebSocket orchestration and VLM integration
- **Cloud VLM** (Gemini 3 Pro primary, Claude 4.5 fallback) - no custom CV model training
- **Web UI** (React + TypeScript + Tailwind) on laptop browser

### Critical Constraints
1. **"No stacking tokens"** gameplay rule - eliminates hardest CV problem
2. **Fixed overhead camera** - consistent perspective for reliable extraction
3. **Turn-based capture** - user triggers, not continuous monitoring
4. **30-second thinking time** - matches human turn pace

### Information Model
- Nearly perfect information game
- AI tracks ALL players' Order cards from Port pickups
- Only hidden info: each player's starting Order card (1 of 2 dealt)
- Board layout (adjacency, building sites) randomized at setup, persists for session

---

## What Was Completed This Session

1. **Brainstormed and refined architecture** through collaborative Q&A
2. **Researched current VLM capabilities** - updated from outdated analysis (70-80% → 80-81% accuracy)
3. **Designed Rust core components:**
   - Game engine data structures
   - MCTS parameters (100k-500k iterations, 8-12 turn rollout depth)
   - Heuristic evaluator with 20+ weighted factors
4. **Designed VLM pipeline:**
   - Multi-pass setup extraction (3 phases)
   - Two-image delta comparison (preferred approach)
   - Color calibration strategies
   - Confidence scoring and batched clarification
5. **Designed Web UI** with all panels and interaction flows
6. **Broke down into 5 parallel development modules** (A-E)
7. **Documented everything** in comprehensive design doc

---

## Next Steps (Implementation)

### Recommended Starting Point: Module A1 - Rust Game Engine

This is foundational with no dependencies. Create the game engine that models Yokohama rules.

**Tasks:**
```
1. Initialize Cargo workspace
   - yokohama-core (library)
   - yokohama-cli (for testing/debugging)

2. Define core data structures:
   - GameState, Board, BoardLayout
   - PlayerState, AreaState
   - Move, TurnPhase, AreaType
   - OrderCard, TechCard, BuildingSiteCard

3. Implement board graph:
   - Area adjacency from layout config
   - Movement pathfinding (BFS through assistants)
   - Canal handling (toll crossing)

4. Implement area actions (18 types):
   - Production: Copper Mine, Silk Mill, Tea Plantation, Fishing Ground
   - Commercial: Bank, Port, Laboratory, Employment Agency, Chinatown, Church, Customs
   - Special: Canal

5. Implement turn phases:
   - Placement (3 on different areas OR 2 on same)
   - Movement (pathfinding with payment rules)
   - Action (power calculation, rewards)
   - Build (shophouse/trading house with building site VP)
   - Recovery (return assistants)

6. Implement additional actions:
   - Foreign Mercenary usage
   - Order fulfillment
   - Achievement claiming

7. Implement end-game:
   - 5 trigger conditions
   - Final scoring (Church, Customs, Tech, Country sets, Remainder)

8. Write tests:
   - Legal move generation
   - State transitions
   - Scoring calculations
```

**Reference:** The rulebook PDF has complete rules. Key pages:
- p.5-7: Turn flow and phases
- p.10-14: Area actions in detail
- p.8-9: End game conditions and scoring

### Alternative Starting Points

**If you prefer UI work:** Start with Module C1-C2 (React shell, camera integration). Can use mock data until API exists.

**If you prefer VLM experimentation:** Start with Module B1 (prompt engineering). Take photos of your Yokohama board and iterate on extraction prompts with Gemini/Claude APIs.

---

## Project Scaffold (When Ready)

```bash
# Suggested structure to create:

yokohama/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── yokohama-core/           # Rust game engine + MCTS
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── game/            # Game state, rules
│   │       ├── mcts/            # Search algorithm
│   │       ├── eval/            # Heuristic evaluator
│   │       └── python/          # PyO3 bindings
│   └── yokohama-cli/            # CLI for testing
├── api/                          # Python FastAPI
│   ├── pyproject.toml
│   └── src/
│       ├── main.py
│       ├── routers/
│       ├── services/
│       └── models/
├── ui/                           # React frontend
│   ├── package.json
│   └── src/
├── docs/plans/                   # Design docs (exists)
└── analysis-1/                   # Research (exists)
```

---

## Open Questions (Need Resolution During Implementation)

1. **Camera connectivity:** Direct USB vs phone-over-WiFi vs IP webcam app?
2. **VLM provider:** Start with Gemini or Claude?
3. **State persistence:** Implement save/resume? (Requires board layout serialization)
4. **Weight tuning:** Self-play tournaments vs human annotation vs hybrid?

---

## Hardware Available

- NVIDIA Quadro RTX 5000 (16GB VRAM)
- Intel Xeon (multi-core)
- 64GB RAM
- Overhead camera mount for phone

---

## Key Files to Read

1. **Design document:** `docs/plans/2025-12-07-yokohama-assistant-design.md` - READ THIS FIRST
2. **Rulebook:** `analysis-1/00-yokohama-rulebook.pdf` - Complete game rules
3. **Initial analysis:** `analysis-1/AI Yokohama Board Game Assistant.md` - Background context (some outdated VLM assumptions)

---

## Commands to Get Started

```bash
cd /home/jmoore/Documents/yokohama

# View design document
cat docs/plans/2025-12-07-yokohama-assistant-design.md

# Check git status
git log --oneline

# Initialize Rust workspace (when ready)
cargo init --name yokohama
mkdir -p crates/yokohama-core/src
```

---

## Notes for Next Agent

- The user (Jeremy) has deep domain knowledge of Yokohama strategy - leverage his expertise for evaluator weight decisions
- He prefers practical, working solutions over theoretical completeness
- The "no stacking tokens" constraint was his idea - a smart simplification
- He's comfortable with Rust and Python; React is standard choice for UI
- 30-second thinking time is acceptable; don't over-optimize for speed initially
- Focus on getting a working vertical slice before polishing any single component
