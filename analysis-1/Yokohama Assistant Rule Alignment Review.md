# **Technical Audit and Compliance Report: Yokohama Game Engine Architecture**

## **1\. Executive Summary**

This report serves as a comprehensive technical audit of the "Yokohama Assistant" project, specifically evaluating the architectural alignment between the proposed A1 Implementation (yokohama-core) and the official *Yokohama* board game rules published by OKAZU Brand. The primary objective is to identify discrepancies, logic gaps, and data modeling insufficiencies that would compromise the engine's ability to enforce legal gameplay or support a robust Monte Carlo Tree Search (MCTS) artificial intelligence agent.

The "Yokohama Assistant" is designed as a hybrid system, utilizing a headless Rust core for high-throughput logic and a Python/Computer Vision layer for user interaction.1 While this separation of concerns represents a sound engineering decision for performance-critical applications, the audit reveals significant deficiencies in the translation of physical board game mechanics into software logic. The foundation documents 1 demonstrate a high-level understanding of the game's flow but fail to capture the granular "micro-rules" that govern the game's tight economic and spatial constraints.

The most critical findings include the complete omission of the "two-assistant" placement option, which fundamentally alters the game's strategic landscape; the absence of passive income logic for Trading Houses, which breaks the internal economy; and an insufficient topological model for movement, which neglects the "start area" return prohibition and specific traversal costs for the Canal and Station. If left unaddressed, these issues will result in an engine that prohibits legal tournament strategies, miscalculates resource distribution, and generates invalid game states during AI simulation.

## **2\. Architectural Analysis of State Representation**

The fidelity of any game engine is ultimately determined by the precision of its state representation. In a "heavy Euro" game like *Yokohama*, where victory is often determined by single-point margins and resource efficiency is paramount 2, the underlying data structures must rigorously enforce the physical constraints of the board. The proposed implementation in yokohama-core utilizes strong typing (e.g., PlayerId, AreaId) 1, which is a best practice for type safety in Rust. However, the modeling of the board's topology and component ownership requires significant expansion to meet the rulebook's requirements.

### **2.1 Board Topology and Graph Theory**

The board in *Yokohama* is not a static grid; it is a modular graph where connectivity and area availability vary significantly based on player count.1 The foundation document defines AreaId as a simple u8 wrapper and provides a BoardLayout struct with a placeholder for adjacency.1 This abstraction is insufficient for the complex movement rules inherent to the game.

The discrepancy arises in the handling of "void" spaces and variable layouts. For instance, in a 2-player game, specific areas such as Laboratory B, Port B, Chinatown, and the Canal are physically removed from the board.1 The current BoardLayoutBuilder is tasked with validation, but the engine lacks a robust graph implementation to handle the dynamic adjacency. A simple adjacency list is inadequate because connections in *Yokohama* are not merely binary (connected vs. not connected); they possess properties such as "Canal Crossing" (which incurs a cost) and "Station Presence" (which modifies movement costs).1

To accurately model the board, the system requires a weighted graph representation where edges are first-class citizens. The current model suggests canal\_crossings: vec\! as a separate list, which decouples the cost from the connection itself. This separation risks creating "logical disconnects" in the pathfinding algorithms (Dijkstra or A\*) used by the MCTS agent. If the pathfinder calculates distance based on the adjacency list but the cost calculator looks up a separate vector, synchronization errors can lead to the AI proposing illegal moves or underestimating movement costs.

### **2.2 Component Ownership and Resource Flow**

Resource management in *Yokohama* distinguishes strictly between a player's "Hand" (available for immediate deployment) and "Warehouse" (requiring retrieval actions).1 The AssistantState struct in the foundation document attempts to model this, but the proposed transition logic in the implementation plan 1 is incomplete regarding the directional flow of these components.

Specifically, the "Recovery" step (Step 6 of the Main Action) requires that assistants retrieved from the board return to the **Hand**, not the Warehouse.1 Conversely, the "Employment Agency" action allows players to move assistants from the **Warehouse** to the **Hand**.1 The implementation plan mentions "Recovery" but does not explicitly define the destination of the recovered tokens. In a Rust state machine, this distinction is critical. If the apply\_move function blindly returns tokens to a generic "supply" pool, the game's tight action economy—where players must frequently waste actions just to mobilize their workforce—will be lost. The MCTS agent, operating on this flawed model, would perceive an infinite supply of workers, leading to aggressive strategies that are impossible to execute in the physical game.

Furthermore, the OrderCard and TechCard registries 1 act as static databases. While efficient, the engine must ensure that the dynamic state (e.g., which orders are currently face-up on the Port board) interacts correctly with these static definitions. The rulebook implies that the Port board has specific slots (Low, Mid, High) with varying costs and availability.1 The current BoardLayout must track not just which cards are available, but specifically *where* they are located, as this dictates the action power required to claim them.

## **3\. Detailed Phase Audit and Discrepancy Analysis**

This section provides a rigorous comparison between the game phases defined in the Official Rulebook 1 and the logic planned in the A1 Implementation.1

### **3.1 Setup and Configuration**

The setup phase establishes the initial constraints of the game. The rules dictate precise removal of components based on player count.

* **2 Players:** Remove Lab B, Port B, Chinatown, Canal, and one of each production area (Copper, Silk, Tea, Fish). 16 Order cards removed. Trigger end game at 4 Church/Customs assistants.  
* **3 Players:** Remove Lab B and one of each production area. 5 Order cards removed. Trigger end game at 5 Church/Customs assistants.  
* **4 Players:** Use all components. Trigger end game at 6 assistants.1

The A1 Implementation manages this via GameConfig, which is a sound approach. However, the logic for the **5-Power Token** distribution presents a nuance. The rules state that 5-Power tokens are shuffled and placed *randomly* on each area's 5-space.1 The implementation plan mentions five\_power\_tokens as a HashMap in BoardLayout.1 The discrepancy lies in the initialization: the engine must support a randomization seed to ensure determinism for replayability while allowing true randomness for new games. The "Game Setup" snippet 3 confirms that tokens are placed *face-up*, meaning they are public information immediately. The engine's state initialization must reveal these tokens to the AI from turn zero, as their location (e.g., a "Get 3 Yen" token on the Bank) radically alters opening strategy.

### **3.2 Phase A: Additional Actions (Pre- and Post-Processing)**

The structure of the turn in *Yokohama* is unique: "Additional Actions" (Use Mercenary, Fulfill Order, Achievement) can be taken *before* or *after* the Main Action, but crucially, the specific sub-action of "Use Foreign Mercenary" can happen only **once per turn**.1

Discrepancy 3.2.1: Mercenary Usage State Tracking  
The implementation plan creates a state machine with AdditionalActionFirst and AdditionalActionSecond phases.1 While this captures the temporal flow, it lacks the persistent flag required to enforce the "once per turn" rule.

* **The Rule:** "If you use this action in A... you may not use it in C.".1  
* **The Gap:** The Move::UseMercenary action atomicity implies it triggers a sub-routine. Without a mercenary\_used\_this\_turn: bool flag stored in the ephemeral TurnState (distinct from the persistent GameState), the engine has no way to prevent a player from using a mercenary in Phase A and attempting to use another in Phase C.  
* **Implication:** An AI exploiting this bug could double its action efficiency in a single turn, breaking the game balance.

Discrepancy 3.2.2: Achievement Race Conditions  
Achievements in Yokohama are a first-come, first-served mechanic. The first player to claim scores the higher point value (leftmost), while subsequent players score lower.1

* **The Rule:** "The first player who achieved the requirements score the leftmost points... Note: Each player may only fulfill the same Achievement Card once in a game.".1  
* **The Gap:** The legal\_moves generator must check two conditions: (1) Does the player meet the criteria? (2) Has the player *already* claimed this specific achievement? The foundation document lists AchievementCard types but does not detail the claimed\_by: Vec\<PlayerId\> tracking within the GameState.  
* **Implication:** If the engine fails to track individual claims, a player could theoretically claim the same achievement multiple times (spamming the action for points), or the engine might fail to degrade the point value for the second claimant.

### **3.3 Phase B: Main Action \- Step 1: Placement**

This phase contains the most severe discrepancy identified in the audit.

Discrepancy 3.3.1: The Omission of Placement Option B  
The rulebook is explicit: "The player may choose 1 of the 2 following options: a) Place 1 Assistant pawn into 3 different areas... b) Place 2 Assistant pawns into the same area.".1

* **The Implementation:** The Move atomicity section in 1 lists Move::PlaceThreeAreas { areas } as the only placement move. There is no mention of a Move::PlaceTwoAssistants variant.  
* **Severity:** **Critical.** The option to place two assistants in a single area is the fundamental mechanism for "bursting" power. It allows a player to go from weak presence (0 assistants) to significant presence (2 assistants \+ 1 President) in a single turn sequence.  
* **Strategic Impact:** Without this option, the game becomes purely linear. Players are forced to spread out (wide strategy) and cannot focus fire (tall strategy) on contested resources. This fundamentally changes the nature of the game, making it impossible to block opponents effectively or secure critical high-value actions (like the 5-power Church bonus) before an opponent.  
* **Remediation:** The Move enum must be expanded to include PlaceAssistants(Vec\<AreaId\>). The validation logic must then accept vectors of length 3 (all unique) OR length 2 (both identical or single target with count 2).

Discrepancy 3.3.2: Placement Costs and Economy  
Placement is not always free. "If there is a President pawn in the area... you have to pay 1 Yen for each Assistant pawn you place to the player owning the President pawn.".1

* **The Implementation:** The document mentions "Validation" but does not detail the look-ahead logic required for legal\_moves.  
* **The Gap:** Generating legal moves in *Yokohama* is computationally expensive because legality depends on liquidity. A move PlaceThreeAreas is only legal if player.money \>= cost(area1) \+ cost(area2) \+ cost(area3).  
* **Implication:** The MCTS generator cannot simply permute all area combinations. It must calculate the cost for every permutation against the player's current wallet. Failure to implement this strict filter will cause the AI to select moves that fail execution, wasting simulation cycles.

### **3.4 Phase B: Main Action \- Step 2: Movement**

Movement in *Yokohama* is a constrained pathfinding problem. The President must move through a contiguous chain of assistants, stopping only where an assistant is present.1

**Discrepancy 3.4.1: The "Start Area" Return Prohibition**

* **The Rule:** "The President pawn may not return to the Area that it started in.".1  
* **The Implementation:** This rule is completely absent from the movement logic plans.1  
* **The Gap:** Without this constraint, a player could effectively "pass" their movement phase by moving to an adjacent tile and immediately moving back, provided they have assistants in both. This violates the spirit of the game, which forces forward momentum and positional churn.  
* **Remediation:** The TurnState must record president\_start\_node. The path validator is\_valid\_path(path) must assert path.last()\!= president\_start\_node.

**Discrepancy 3.4.2: Canal Traversal Costs**

* **The Rule:** "You may pay 1 Yen to the supply to move across the Canal.".1  
* **The Implementation:** The BoardLayout mentions canal\_crossings, but the cost logic is missing.1  
* **The Gap:** The canal is a topological shortcut with an economic barrier. In graph theory terms, edges marked as "Canal" have a weight of 1 \+ 1 (Yen).  
* **Implication:** If the AI sees the Canal as a standard connection, it will overestimate the connectivity of the board. It might plan a route to a vital resource, only to realize during execution that it lacks the 1 Yen toll, resulting in a failed turn.

**Discrepancy 3.4.3: The Station as a Movement Hub**

* **The Rule:** "Place the Station token in an Area... You may move your President pawn to this area... for free, and you do not need to pay even if there are other President pawns.".1  
* **The Implementation:** The Technology table lists the Station only under ModifyPower (+1 power).1 It is explicitly missing from ModifyMovement.  
* **The Gap:** The Station card is a dual-threat engine: it provides power *and* mobility. By ignoring the movement benefit, the engine undervalues the card significantly.  
* **Remediation:** A new hook ModifyMovementCost must be implemented. The logic must check: if destination \== station\_location && player\_has\_tech(Station) { cost \= 0 }.

### **3.5 Phase B: Main Action \- Step 3: Area Execution**

The core loop of the game involves executing the action of the tile where the President lands.

**Discrepancy 3.5.1: Trading House Passive Income**

* **The Rule:** "If there are any Trading Houses owned by other players in the Area... the owner receives 1 Yen from the bank.".1  
* **The Implementation:** The calculate\_power function correctly adds \+1 Power for a Trading House 1, but the passive income logic is missing.1  
* **Severity:** **Critical.** This rule is the primary economic engine for long-term strategy. It incentivizes building on high-traffic areas (Bank, Port, Lab).  
* **Architectural Challenge:** Implementing this in a pure functional style (state \+ move \= new\_state) is tricky because the move is taken by Player A, but the state change happens to Player B.  
* **Remediation:** The apply\_move function must identify all side effects before mutation. It should iterate through the buildings in the active area, identify owners distinct from the current player, and increment their money counters. This creates a "positive interaction" loop that the AI must be aware of—building a Trading House is an investment that pays dividends.

Discrepancy 3.5.2: Power Calculation and Caps  
The rules state a hard cap of 5 Power.1 The implementation correctly identifies this cap in game/power.rs.1 However, the calculation of power must strictly follow the order of operations: Base Power (President/Assistants) \+ Buildings \+ Station \+ Tech Bonuses \-\> Capped at 5\. The implementation plan seems to follow this, but rigorous testing is needed to ensure the cap applies after all bonuses.

### **3.6 Phase B: Steps 4 & 5: Bonuses (5-Power and Building)**

Discrepancy 3.6.1: Variable Construction Costs  
Building a Trading House becomes progressively more expensive.

* **The Rule:** "4 Yen for the 1st, 5 Yen for the 2nd, 6 Yen for the 3rd, 7 Yen for the 4th".1  
* **The Implementation:** Move::BuildTradingHouse is listed as a generic move.1  
* **The Gap:** The engine must dynamically calculate the cost based on the number of Trading Houses already on the board *and* in the player's supply.  
* **Validation:** The legal\_moves function must check player.money \>= 4 \+ count(built\_trading\_houses). If this dynamic cost is missing, the AI will incorrectly evaluate the feasibility of end-game building rushes.

### **3.7 Phase B: Step 6: Recovery**

**Discrepancy 3.7.1: Telephone Technology Implementation**

* **The Rule:** "Telephone: During the Recovery Step, you may leave 1 Assistant pawn in the Area.".1  
* **The Implementation:** The implementation mentions Telephone but provides a generic Recovery step.  
* **The Gap:** Recovery is usually automatic (mandatory recall). The Telephone makes it optional and granular.  
* **Remediation:** The Move enum requires a specific variant: Move::RecoverAssistants { leave\_one: bool }. This introduces a decision point in what is normally a deterministic phase. The AI must learn that leaving an assistant is valuable for maintaining presence for the *next* turn, even if it reduces the hand size for placement.

## **4\. AI and MCTS Implications**

The discrepancies identified above are not merely rule bugs; they are structural flaws that will severely degrade the performance of the Monte Carlo Tree Search agent.

### **4.1 Heuristic Evaluation Distortion**

The MCTS agent relies on a heuristic evaluation function to estimate the value of non-terminal game states.1

* **Passive Income:** Without the Trading House income logic (Discrepancy 3.5.1), the evaluator will systematically undervalue construction. It will see the high cost (4-7 Yen) without the recurring revenue, leading the AI to ignore building Trading Houses in favor of cheaper, short-term point scoring.  
* **Movement Costs:** Without the Canal cost logic (Discrepancy 3.4.2), the evaluator will overestimate the player's mobility. It might judge a state as "strong" because it sees a path to the Port, not realizing the path is blocked by a lack of funds.

### **4.2 Search Tree Topology**

* **Branching Factor:** The omission of "Place 2 Assistants" (Discrepancy 3.3.1) artificially reduces the branching factor of the game tree. While this makes the tree deeper to search, it makes the search **wrong**. The AI will not consider "burst" strategies and will assume opponents cannot burst either. When a human player executes a "Place 2" move, the AI will be pushed into an unexplored state, rendering its previous computations useless.  
* **Simulation Validity:** MCTS relies on random rollouts (simulations) to determine win rates. If the simulation engine allows illegal moves (like returning to start) or prohibits legal ones (like placing 2), the statistical signal generated by the rollouts will be noise. The "Two-Assistant" move is a high-frequency action in expert play; its absence in simulations means the AI is effectively playing a different game variant.

### **4.3 Performance vs. Accuracy**

The "headless" Rust architecture targets 10,000+ moves per second.1 However, accuracy cannot be sacrificed for speed.

* **Complex Validation:** Implementing the legal\_moves filters for money (placement costs, canal costs) is computationally more expensive than simple graph traversal.  
* **Recommendation:** To maintain throughput, the engine should utilize **Bitboards** or similar low-level optimizations for board state checking (e.g., a 128-bit integer representing opponent presence across the 18 areas) rather than iterating through HashMaps. This would allow rigorous rule enforcement without the performance penalty of high-level object manipulation.

## **5\. Computer Vision and User Interface Integration**

The audit also identifies implications for the Python/CV layer.1

### **5.1 State Reconciliation**

The engine assumes a deterministic flow. However, the Computer Vision layer introduces noise.

* **Scenario:** The user physically places 2 assistants (a legal move), but the engine's legal\_moves only expects "Place 3".  
* **Failure Mode:** The Python orchestration layer calls get\_legal\_moves() from Rust to validate the CV output. The Rust core returns a list excluding the user's move. The system flags the user's valid move as "Illegal."  
* **Consequence:** This breaks the user trust loop. The system forces the user to correct a move that was actually valid, creating frustration.  
* **Correction:** The Rust core *must* implement the superset of all physical possibilities, even those it deems sub-optimal, to correctly validate human input.

### **5.2 Manual Correction UI**

If the engine rules are updated to include "Place 2", the Web UI 1 must also be updated to support this input mode. The current design implies a "click 3 areas" workflow. It must support "click 1 area twice" or "drag 2 assistants" interactions.

## **6\. Remediation Specifications and Technical Recommendations**

To align the A1 Implementation with the *Yokohama* rules, the following technical changes are mandatory.

### **6.1 Data Structure Enhancements**

**Table 1: Required Struct Updates**

| Struct | Field/Method to Add | Purpose |
| :---- | :---- | :---- |
| BoardLayout | adjacencies: HashMap\<AreaId, Vec\<(AreaId, EdgeType)\>\> | Implement weighted graph for movement. |
| TurnState | mercenary\_used: bool | Enforce "once per turn" rule. |
| TurnState | president\_start\_node: AreaId | Enforce "cannot return to start" rule. |
| Move | PlaceAssistants(Vec\<AreaId\>) | Enable both 3-area and 2-area placement options. |
| Move | RecoverAssistants { leave\_one: bool } | Enable Telephone tech decision. |
| TechCard | hook\_gain\_resource: Option\<(Resource, u8)\> | Enable Mining Tech logic (+1 Copper). |

### **6.2 Logic Implementation Patterns**

Pattern 1: The Weighted Pathfinding  
The pathfinding algorithm in game/movement.rs must be updated to accept a closure for cost calculation.

Rust

// Remediation for Discrepancy 3.4.2 & 3.4.3  
pub fn calculate\_move\_cost(current: AreaId, next: AreaId, player: \&PlayerState, board: \&BoardLayout) \-\> u8 {  
    let mut cost \= 0;  
      
    // Check edge type for Canal  
    if let Some(edge\_type) \= board.get\_edge\_type(current, next) {  
        if edge\_type \== EdgeType::Canal {  
            cost \+= 1; // 1 Yen toll  
        }  
    }  
      
    // Check for opponents  
    if board.has\_opponent\_president(next) {  
        if\!player.has\_tech(Tech::Stagecoach) {  
            cost \+= 1; // 1 Yen toll  
        }  
    }  
      
    // Check Station exemption  
    if Some(next) \== board.station\_location && player.has\_tech(Tech::Station) {  
        return 0; // Station overrides all costs  
    }  
      
    cost  
}

Pattern 2: Side-Effect Resolution  
To handle Trading House income (Discrepancy 3.5.1) without violating the "pure function" ideal, the system should return a TransitionResult that includes both the new state and a log of side effects.

Rust

pub struct TransitionResult {  
    pub new\_state: GameState,  
    pub payouts: Vec\<(PlayerId, u8)\>, // PlayerId gains X Yen  
}

This allows the UI to display "Player Blue received 1 Yen" while keeping the state mutation logic encapsulated.

## **7\. Conclusion**

The "Yokohama Assistant" project stands on a solid architectural foundation with its Rust-based core and decoupled CV layer. However, the current implementation plan 1 contains significant gaps in its understanding of the *Yokohama* rule set. The omission of the **Two-Assistant Placement** option, **Trading House Passive Income**, and **Graph-Based Movement Costs** (Canal/Station) constitute critical failures that effectively simulate a different, simpler game.

These discrepancies will compound during MCTS training, leading to an AI that plays poorly and offers invalid advice. By implementing the specific data structure enhancements and logic patterns detailed in Section 6, the engineering team can bridge the gap between the current draft and a tournament-legal engine. Immediate priority must be given to refactoring the Move enum and the BoardLayout graph structures before further logic is written. Without these foundational corrections, the assistant will fail to serve its primary purpose: providing accurate, expert-level guidance to players of *Yokohama*.

#### **Works cited**

1. 2025-12-07-yokohama-assistant-design.md  
2. I find Yokohama overwhelming \- does anyone have any strategy advice? : r/boardgames \- Reddit, accessed December 7, 2025, [https://www.reddit.com/r/boardgames/comments/i3o66r/i\_find\_yokohama\_overwhelming\_does\_anyone\_have\_any/](https://www.reddit.com/r/boardgames/comments/i3o66r/i_find_yokohama_overwhelming_does_anyone_have_any/)  
3. Yokohama Rulebook, accessed December 7, 2025, [https://cdn.1j1ju.com/medias/00/9a/00-yokohama-rulebook.pdf](https://cdn.1j1ju.com/medias/00/9a/00-yokohama-rulebook.pdf)