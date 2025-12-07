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

---

## Information Model

Yokohama is **nearly perfect information**:
- **Public:** Board state, all resources, all tracks, all buildings, all technology cards, all order cards picked up or completed
- **Hidden:** Only each player's starting Order card (draw 2, keep 1 at game start)

This simplifies MCTS significantly - no complex hidden state handling needed.

### Order Card Tracking (Critical Insight)

The AI knows what Order cards **ALL players** currently hold, not just its own color:

- When ANY player takes an Order card from the Port, everyone sees which specific card left which slot
- The AI tracks every Port visit by every player
- Therefore: AI knows exactly which Order cards each opponent holds (except their starting card)
- Completed orders are placed face-up in front of each player (permanently visible)
- **Only uncertainty:** Each opponent's starting Order card (1 of 2 dealt, kept secretly at game start)

This makes opponent modeling largely deterministic. The AI can evaluate opponents' positions accurately because it knows 2 of their 3 possible Order cards with certainty, and the starting card uncertainty shrinks over time through behavioral inference and elimination.

### Board Layout Persistence

The board layout is **randomized at game setup** and persists for the entire session:

- 12-18 Area tiles shuffled and placed in a configuration
- Building Site cards randomly assigned to each area (different VP values)
- 5-Power tokens randomly placed on areas

**Captured once at setup:**
- Adjacency graph (which areas touch which)
- Building Site VP values per area
- Initial card placements (Tech cards on Labs, Order cards on Ports)

**Implications:**
- Board layout is required input for the Rust engine at game initialization
- Save/resume functionality must persist the board layout
- Evaluator's topology-aware factors depend on the specific adjacency graph

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      WEB UI (Browser)                       │
│  - Camera feed display    - Turn capture buttons            │
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
    board: Board,                      // Includes adjacency graph
    board_layout: BoardLayout,         // Persisted from setup
    players: Vec<PlayerState>,
    current_player: PlayerId,
    turn_phase: TurnPhase,
    church_track: TrackState,
    customs_track: TrackState,
    available_orders: Vec<OrderCard>,
    available_techs: Vec<TechCard>,
    order_deck_remaining: u8,
}

pub struct BoardLayout {
    areas: HashMap<AreaId, AreaConfig>,
    adjacency: HashMap<AreaId, Vec<AreaId>>,
    building_sites: HashMap<AreaId, BuildingSiteCard>,  // VP values per area
    canal_adjacent_pairs: Vec<(AreaId, AreaId)>,
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

**Preferred approach: Two-image comparison**

When the VLM provider supports multi-image input (Gemini does), send both:
- Previous turn end image
- Current turn end image

The VLM compares visually and identifies changes directly.

**Fallback approach: Annotated reference**

If two-image not supported:
- Annotate previous image with state overlay (circles, labels)
- Send annotated image + current image
- VLM uses visual reference rather than parsing JSON

Key extractions:
- President pawn movements
- Assistant placements and recoveries
- Buildings constructed
- Cards taken (visible gaps in rows)
- Track changes (Church/Customs)

#### Color Calibration

Player colors (Red, Blue, Yellow, Green) may shift under variable lighting. To help VLM distinguish:

**Physical option:** Place a color reference card in the camera's field of view with labeled swatches.

**Digital option:** Overlay color calibration strip on captured images:
```python
class ImagePreprocessor:
    def add_color_reference(self, image: bytes, player_colors: dict) -> bytes:
        """Add labeled color swatches in corner matching actual component colors."""
        # Swatches labeled "RED", "BLUE", "YELLOW", "GREEN"
        # Colors sampled from physical components under setup lighting
```

**Prompt option:** Include in VLM prompt:
```
Note: In this image, player colors appear as:
- Red: burgundy/maroon wooden pieces
- Blue: navy/dark blue wooden pieces
- Yellow: gold/tan wooden pieces
- Green: forest green wooden pieces
```

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
POST /game/{id}/deviation         # Handle player not following recommendation
GET  /game/{id}/state             # Current game state
GET  /game/{id}/recommend         # Trigger AI recommendation
WS   /game/{id}/ws                # Real-time updates

# State manager responsibilities
- Game state history tracking (becomes training data)
- Delta application with validation
- Reconciliation when corrections applied
- Deviation detection and handling
- WebSocket event emission
```

---

### 4. Web UI

React + TypeScript + Tailwind application.

**Key screens:**
1. **Setup Wizard:** Player count, color, camera connection, initial capture
2. **Setup Verification:** Detected layout with tap-to-correct
3. **Main Game:** Camera feed, state display, turn capture buttons
4. **AI Recommendation:** Visualized move with explanation
5. **Clarification Modal:** Batched questions with image highlights
6. **Game History:** Turn-by-turn log (also serves as training data)

**Key panels (Main Game screen):**

| Panel | Contents |
|-------|----------|
| **Player State** | Resources, assistants (hand/warehouse/board), held orders |
| **Majority Tracks** | Church, Customs, Technology standings with gap indicators |
| **End Game Triggers** | Progress toward each of 5 triggers, acceleration/delay guidance |
| **Available Technologies** | Lab A and Lab B card listings with costs |
| **Recommendation Feedback** | "Did you follow?" [Yes] [Partially] [No, different move] |
| **Manual Correction** | Tap-to-edit mode for fixing CV errors |

**Turn capture flow (addressing button ambiguity):**

```
┌──────────────────────────────────────────────────────┐
│  Current turn: BLUE                                  │
│                                                      │
│  After Red's turn:  [ Capture Red's Move ]          │
│                                                      │
│  (Your turn next)   [ Get My Recommendation ]       │
└──────────────────────────────────────────────────────┘
```

Or single button with clear context:
```
[ Capture Turn ]
"Captures board state after [Red]'s turn"
```

**Key components:**
- SVG-based board visualization (better than Canvas for interaction)
- WebSocket for real-time state updates
- Camera feed via WebRTC (phone over WiFi)

---

## Interaction Flow

### Per-Turn Flow

```
1. Opponent completes their turn
2. User taps "Capture [Color]'s Turn" on UI
3. System captures overhead image
4. VLM extracts delta (two-image comparison preferred)
5. Rust engine validates state transition
6. If ambiguous: show clarification modal
7. If valid: update state, check if user's turn
8. If user's turn: run MCTS (30 sec), display recommendation
9. User executes move physically
10. User taps "Capture My Turn"
11. System captures result, validates
12. If deviation detected: prompt for confirmation (see below)
13. Repeat
```

### Initial Order Card Handling

Only hidden information input needed:
- At game start, user selects which of 2 dealt Order cards to keep
- All subsequent Order card acquisitions are tracked by AI (it recommends which to take)

### Deviation Handling

When user doesn't follow AI recommendation:

```
Scenario: AI recommends "draw Order from High row"
          User takes from Mid row instead

1. CV detects which card slot is now empty (Mid, not High)
2. System prompts:
   "Looks like you took [Mid card details] instead of [High card details]. Confirm?"
3. User confirms or corrects
4. AI updates internal state accordingly
5. Next recommendation accounts for actual card held
```

This also applies to movement, placements, and other actions. The CV serves as ground truth, with confirmation prompts when it differs from the recommendation.

### Recommendation Feedback Loop

After user executes their turn:

```
┌─────────────────────────────────────────────────────────┐
│  Did you follow the AI recommendation?                  │
│                                                         │
│  [Yes, exactly] [Partially] [No, did something else]   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

This feedback:
- Validates state reconciliation
- Flags potential CV errors for review
- Becomes training data for evaluator weight tuning

---

## Parallel Development Modules

### Module A: Rust Core
- A1: Game Engine (rules, state, validation)
- A2: MCTS Engine (parallel search)
- A3: Heuristic Evaluator (multi-factor scoring)
- A4: PyO3 Bindings (Python interop)

### Module B: VLM Pipeline
- B1: Prompt Engineering (multi-pass, delta, two-image)
- B2: VLM Client (Gemini primary + Claude fallback)
- B3: Clarification System (batching, UI prep)
- B4: Color Calibration (preprocessing, prompt tuning)

### Module C: Web UI
- C1: Core App Shell (React, routing, WebSocket)
- C2: Camera & Capture (WebRTC, upload)
- C3: Board Visualization (SVG, animations)
- C4: Game Panels (state, triggers, technologies, recommendations)
- C5: Interaction Flows (wizards, modals, deviation handling)
- C6: Manual Correction Mode (tap-to-edit state)

### Module D: Python API
- D1: FastAPI Server (endpoints, WebSocket)
- D2: State Manager (history, validation, deviation detection)
- D3: Orchestration (wire everything together)

### Module E: Integration & Tuning
- E1: End-to-End Testing
- E2: AI Strength Tuning (self-play, weights)
- E3: Performance Optimization
- E4: Training Data Pipeline (game logs → weight optimization)

### Timeline Dependencies

```
Week 1-2:  A1, C1-C2 (engine, UI shell) — parallel
Week 2-4:  A2, A3, B1 (MCTS, evaluator, prompts) — parallel
Week 4-5:  A4, B2-B4 (bindings, VLM client) — parallel
Week 5-7:  D1-D3, C3-C6 (API, UI panels) — parallel
Week 7-9:  E1-E4 (integration, tuning)
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
3. **State persistence:** Save/resume games across sessions? (Requires persisting board layout)
4. **Opponent modeling depth:** How much computation budget for inferring opponents' starting orders?
5. **Deviation tracking:** How granular should deviation logging be for training data?
6. **Weight tuning methodology:** Self-play tournaments, human game annotation, or hybrid?
7. **Starting order inference:** Active inference during play (adjust probabilities based on observed moves) or passive distribution tracking?

---

## Success Criteria

1. **CV Accuracy:** >90% correct state extraction under controlled lighting
2. **AI Strength:** Consistently beats random baseline; competitive with intermediate human players
3. **Latency:** <35 seconds from "Turn Complete" tap to recommendation display
4. **Reliability:** Graceful degradation with manual correction when CV fails
5. **Training Value:** Game history logs sufficient for iterative weight improvement
