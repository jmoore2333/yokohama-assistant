# Yokohama AI Assistant - Technical Design

## Overview

An augmented reality assistant for the board game Yokohama that observes a physical tabletop via overhead camera and provides real-time strategic recommendations during live play.

**Key characteristics:**
- Live game assistant (openly assisted play with opponents aware)
- Fixed overhead camera setup for reliable CV
- Cloud VLM for state extraction (Gemini 3 Pro / Claude 4.5 Opus)
- Local compute for AI (Rust MCTS on Xeon + RTX 5000)
- Web UI on laptop for feedback and recommendations
- 30-second thinking time budget per move

## Information Model

Yokohama is **nearly perfect information**:
- **Public:** Board state, all resources, all tracks, all buildings, all technology cards, all order cards picked up or completed
- **Hidden:** Only each player's starting Order card (draw 2, keep 1 at game start)

This simplifies MCTS significantly - no complex hidden state handling needed.

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      WEB UI (Browser)                       │
│  - Camera feed display    - "Turn Complete" button          │
│  - Board state viz        - Move recommendations            │
│  - Manual correction UI   - Game history                    │
└─────────────────────────┬───────────────────────────────────┘
                          │ WebSocket + REST
┌─────────────────────────▼───────────────────────────────────┐
│               PYTHON API LAYER (FastAPI)                    │
│  - HTTP/WebSocket routing     - Session management          │
│  - VLM orchestration          - State diffing logic         │
│  - Image handling             - Response formatting         │
└──────────┬──────────────────────────────┬───────────────────┘
           │ PyO3 bindings                │ HTTPS
┌──────────▼──────────────┐    ┌──────────▼───────────────────┐
│   RUST CORE (yokohama-core) │    │      CLOUD VLM              │
│  ┌────────────────────┐ │    │  (Gemini 3 / Claude 4.5)    │
│  │ Game Engine        │ │    │  - Image → JSON extraction  │
│  │ - Rules/validation │ │    │  - Delta detection          │
│  │ - State management │ │    │  - Clarification queries    │
│  └────────────────────┘ │    └──────────────────────────────┘
│  ┌────────────────────┐ │
│  │ MCTS Engine        │ │
│  │ - Parallel search  │ │
│  │ - UCT selection    │ │
│  └────────────────────┘ │
│  ┌────────────────────┐ │
│  │ Heuristic Eval     │ │
│  │ - Position scoring │ │
│  │ - Resource value   │ │
│  └────────────────────┘ │
└─────────────────────────┘
```

**Design rationale:**
- Rust core handles all compute-intensive work (MCTS needs parallel execution, Python GIL is a bottleneck)
- Python/FastAPI is thin glue for HTTP and VLM orchestration
- Cloud VLM eliminates need for custom CV model training
- "No stacking tokens" gameplay constraint eliminates hardest CV problem

---

## Component Details

### 1. Rust Core (yokohama-core)

#### Game Engine

Models complete Yokohama rules:
- 18 area types with specific actions
- Turn phases: Placement → Movement → Action → Build → Recovery
- Additional actions: Foreign Mercenaries, Order fulfillment, Achievements
- 5 end-game triggers
- 5 end-game scoring categories

```rust
pub struct GameState {
    board: Board,
    players: Vec<PlayerState>,
    current_player: PlayerId,
    turn_phase: TurnPhase,
    church_track: TrackState,
    customs_track: TrackState,
    available_orders: Vec<OrderCard>,
    available_techs: Vec<TechCard>,
    order_deck_remaining: u8,
}

// Core API
fn get_legal_moves(state: &GameState) -> Vec<Move>;
fn apply_move(state: &GameState, mv: &Move) -> GameState;
fn validate_state(state: &GameState) -> ValidationResult;
fn is_game_over(state: &GameState) -> bool;
fn calculate_final_scores(state: &GameState) -> Vec<(PlayerId, u32)>;
```

#### MCTS Engine

Parallel tree search with UCT selection:

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Thinking time | 30 seconds | Matches human pace |
| Target iterations | 100,000-500,000 | Conservative for hardware |
| Rollout depth | 8-12 turns | Deeper strategic lookahead |
| Threads | Match Xeon cores | Full CPU utilization |

```rust
pub struct MctsEngine {
    exploration_constant: f64,
    max_iterations: u32,
    max_time_ms: u64,
    num_threads: usize,
}

pub struct SearchResult {
    best_move: Move,
    confidence: f64,
    principal_variation: Vec<Move>,
    nodes_explored: u64,
}
```

#### Heuristic Evaluator

Multi-factor evaluation capturing Yokohama's "point salad" scoring:

```rust
pub struct EvalWeights {
    // Direct Scoring
    victory_points: f64,

    // Majority Races (position-dependent marginal value)
    church_marginal: MarginalWeights,
    customs_marginal: MarginalWeights,
    technology_race_marginal: MarginalWeights,

    // Set Collection
    country_set_progress: [f64; 6],

    // Assistant Economy
    assistant_in_hand: [f64; 9],     // Value curve by count
    assistant_in_warehouse: f64,
    assistant_locked_penalty: f64,

    // Resources (context-dependent)
    resource_toward_order: f64,
    resource_excess: f64,
    imported_goods_base: f64,
    money_base: f64,

    // Engine Components
    technology_base: f64,
    synergy_bonuses: HashMap<SynergyCluster, f64>,
    foreign_mercenary: f64,
    five_power_token: f64,

    // Buildings (site-aware)
    shophouse_vp_multiplier: f64,
    trading_house_vp_multiplier: f64,
    trading_house_traffic_income: f64,

    // Positional (topology-aware)
    area_reachability: [f64; 4],
    president_blocking_value: f64,
    canal_penalty: f64,

    // Strategic State
    order_completion_potential: f64,
    order_trap_penalty: f64,
    end_game_proximity: EndGameWeights,

    // Phase Modifier
    phase_engine_weight: f64,
    phase_vp_weight: f64,
}
```

**Key strategic factors:**
- Majority races use marginal value (overtaking 1st vs hopeless positions)
- Assistant economy tracks hand vs warehouse vs board vs locked
- Technology synergies detected explicitly (Ball + orders, Gaslight + shops)
- Trading House passive income from opponent actions
- Building site VP varies by randomly-placed cards
- Board topology affects movement and blocking value

---

### 2. Cloud VLM Integration

#### Multi-Pass Setup Extraction

Single-shot extraction is too complex. Use 3 phases:

1. **Phase 1 - Board Layout:** Area tiles, positions, adjacencies
2. **Phase 2 - Building Sites:** VP values on each area's card
3. **Phase 3 - Cards:** Technology and Order cards on boards

Each phase validates before proceeding.

#### Turn Delta Extraction

Compare two images (previous turn end → current turn end):
- Two-image comparison if provider supports
- Otherwise: annotate previous image with state overlay

Key extractions:
- President pawn movements
- Assistant placements and recoveries
- Buildings constructed
- Cards taken (visible gaps in rows)
- Track changes (Church/Customs)

#### Confidence Scoring

VLM self-reports confidence 0.0-1.0:
- \>0.9: Accept automatically
- 0.7-0.9: Accept with logging
- <0.7: Trigger clarification

#### Clarification System

Batch all uncertainties into single UI interaction:
- Image with numbered circles on ambiguous regions
- Quick-tap answers for each question
- Submit all at once (low friction)

---

### 3. Python API Layer

Thin orchestration layer:

```python
# Core endpoints
POST /game/new                    # Initialize session
POST /game/{id}/setup             # Process setup image
POST /game/{id}/turn              # Process turn image
GET  /game/{id}/state             # Current game state
GET  /game/{id}/recommend         # Trigger AI recommendation
WS   /game/{id}/ws                # Real-time updates

# State manager responsibilities
- Game state history tracking
- Delta application with validation
- Reconciliation when corrections applied
- WebSocket event emission
```

---

### 4. Web UI

React + TypeScript + Tailwind application.

**Key screens:**
1. **Setup Wizard:** Player count, color, camera connection, initial capture
2. **Setup Verification:** Detected layout with tap-to-correct
3. **Main Game:** Camera feed, state display, "Turn Complete" button
4. **AI Recommendation:** Visualized move with explanation
5. **Clarification Modal:** Batched questions with image highlights
6. **Game History:** Turn-by-turn log for analysis

**Key components:**
- SVG-based board visualization (better than Canvas for interaction)
- WebSocket for real-time state updates
- Camera feed via WebRTC (phone over WiFi)

---

## Interaction Flow

### Per-Turn Flow

```
1. Opponent completes their turn
2. User taps "Turn Complete" on UI
3. System captures overhead image
4. VLM extracts delta (what changed)
5. Rust engine validates state transition
6. If ambiguous: show clarification modal
7. If valid: update state, check if user's turn
8. If user's turn: run MCTS (30 sec), display recommendation
9. User executes move physically
10. System captures result, validates, repeat
```

### Initial Order Card Handling

Only hidden information input needed:
- At game start, user selects which of 2 dealt Order cards to keep
- All subsequent Order card acquisitions are tracked by AI (it recommends which to take)

---

## Parallel Development Modules

### Module A: Rust Core
- A1: Game Engine (rules, state, validation)
- A2: MCTS Engine (parallel search)
- A3: Heuristic Evaluator (multi-factor scoring)
- A4: PyO3 Bindings (Python interop)

### Module B: VLM Pipeline
- B1: Prompt Engineering (multi-pass, delta)
- B2: VLM Client (Gemini + Claude)
- B3: Clarification System (batching, UI prep)

### Module C: Web UI
- C1: Core App Shell (React, routing, WebSocket)
- C2: Camera & Capture (WebRTC, upload)
- C3: Board Visualization (SVG, animations)
- C4: Game Panels (state display, recommendations)
- C5: Interaction Flows (wizards, modals)

### Module D: Python API
- D1: FastAPI Server (endpoints, WebSocket)
- D2: State Manager (history, validation)
- D3: Orchestration (wire everything together)

### Module E: Integration & Tuning
- E1: End-to-End Testing
- E2: AI Strength Tuning (self-play, weights)
- E3: Performance Optimization

### Timeline Dependencies

```
Week 1-2:  A1, C1-C2 (engine, UI shell) — parallel
Week 2-4:  A2, A3, B1 (MCTS, evaluator, prompts) — parallel
Week 4-5:  A4, B2-B3 (bindings, VLM client) — parallel
Week 5-7:  D1-D3, C3-C5 (API, UI panels) — parallel
Week 7-9:  E1-E3 (integration, tuning)
```

---

## Hardware Requirements

**Development/Runtime:**
- NVIDIA Quadro RTX 5000 (or similar GPU)
- Intel Xeon (multi-core)
- 64GB RAM
- Overhead camera mount for phone

**Network:**
- Local WiFi for phone → laptop camera feed
- Internet for cloud VLM API calls

---

## Design Constraints

1. **No stacking tokens** - Physical gameplay rule eliminates hardest CV problem
2. **Fixed overhead camera** - Consistent perspective for reliable extraction
3. **Turn-based capture** - User triggers capture, not continuous monitoring
4. **Cloud VLM** - No custom model training, leverage SOTA vision models
5. **Local AI compute** - MCTS runs locally for low latency, no cloud dependency for "brain"

---

## Open Questions for Implementation

1. **Camera connectivity:** Direct USB vs phone-over-WiFi vs IP webcam app?
2. **VLM provider:** Start with Gemini (better structured output) or Claude (better uncertainty handling)?
3. **State persistence:** Save/resume games across sessions?
4. **Opponent modeling depth:** How much computation budget for inferring opponents' starting orders?

---

## Success Criteria

1. **CV Accuracy:** >90% correct state extraction under controlled lighting
2. **AI Strength:** Consistently beats random baseline; competitive with intermediate human players
3. **Latency:** <35 seconds from "Turn Complete" tap to recommendation display
4. **Reliability:** Graceful degradation with manual correction when CV fails
