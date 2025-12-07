# **Technical Feasibility and Architectural Specification for an Augmented Reality Board Game Assistant: The Yokohama Project**

## **1\. Strategic Overview and Feasibility Analysis**

The ambition to digitize the board game *Yokohama*—not merely as a screen-based video game, but as a perceptually aware, intelligent assistant capable of observing a physical tabletop session and advising a human player—represents a significant convergence of three distinct disciplines: computational game theory, computer vision (CV), and distributed systems engineering. The user’s request bifurcates the project into two core deliverables: a game engine capable of learning and playing *Yokohama* at a high level, and a frontend perception system that bridges the analog and digital worlds via a camera interface. This report provides an exhaustive analysis of the architectural requirements, algorithmic strategies, and hardware constraints necessary to realize this vision.

### **1.1 The Complexity of the Domain**

*Yokohama*, designed by Hisashi Hayashi, is a "heavy" Euro-style strategy game that presents computational challenges distinct from classical abstract games like Chess or Go. While Chess possesses a static 8x8 grid and a finite set of piece types, *Yokohama* features a modular board layout that changes every game, a high density of game components (over 150 individual tokens, cubes, and pawns), and a "point salad" scoring mechanism where victory points are derived from complex, interconnected sub-systems rather than a single win condition.1

From a Computer Vision perspective, the challenge is non-trivial. The system must not only detect objects but distinguish between visually similar components (e.g., distinguishing a copper resource token from a silk token, or identifying a stack of three assistant cubes versus a stack of two) under variable lighting conditions. The board itself is a graph of unconnected tiles, meaning the system must "reconstruct" the map topology at the start of each session.3

From an Artificial Intelligence perspective, the branching factor of *Yokohama* is massive. On a given turn, a player chooses between placing assistants (in multiple permutations of locations), moving their President pawn along a variable path (pathfinding on a dynamic graph), and executing actions that may trigger cascades of resource conversions.5 A brute-force solver is mathematically impossible; thus, the solution requires advanced heuristic search algorithms, specifically Monte Carlo Tree Search (MCTS) augmented by domain-specific evaluation functions.

### **1.2 Feasibility Verdict**

Despite these complexities, the project is **technically feasible** for a dedicated developer or small team, provided a stratified development approach is adopted. The existence of robust open-source libraries for object detection (YOLOv8), graph theory (NetworkX), and game state management creates a strong foundation. The primary risk lies in the reliability of the Computer Vision pipeline—specifically the "occlusion problem" where stacks of pieces hide information—and the computational cost of running MCTS simulations in real-time.

The analysis suggests a **Hybrid Mobile-Edge Architecture** is the optimal deployment strategy. While modern smartphones (iPhone 15, Pixel 8\) possess impressive Neural Processing Units (NPUs) capable of running quantized object detection models locally 7, the heavy lifting of the AI search (MCTS rollouts) is best offloaded to a local server (e.g., a PC with a dedicated GPU) or a cloud instance to ensure the user receives move suggestions within a distinct "thinking time" window of 5-10 seconds.

### **1.3 System Component Breakdown**

The proposed solution comprises three distinct subsystems, which will be analyzed in depth throughout this report:

**Table 1: System Component Architecture**

| Subsystem | Primary Responsibility | Core Technologies | Critical Challenge |
| :---- | :---- | :---- | :---- |
| **Logic Engine** | The "Ground Truth." Models the rules, board state, and legal moves. | Python, Pydantic, NetworkX | Modeling the complex dependency graph of technologies and movement rules. |
| **AI Agent** | The "Brain." Explores future states to determine optimal strategy. | MCTS, Minimax, Heuristic Evaluation | Managing the branching factor and evaluating "point salad" board states. |
| **Perception System** | The "Eyes." Converts camera feed into a structured JSON game state. | YOLOv8/v11, OpenCV, Homography | Accurate counting of stacked cubes and distinguishing small, similar tokens. |

The following sections will deconstruct each of these subsystems, providing a roadmap from theoretical design to practical implementation.

## ---

**2\. Domain Analysis: The "Yokohama" Problem Space**

To build a model of *Yokohama*, one must first translate its physical and rule-based realities into rigorous data structures. The game is effectively a resource management optimization puzzle played on a dynamic graph.

### **2.1 The Physical Component Taxonomy**

The Computer Vision system is constrained by what it must detect. Unlike digital implementations where the state is known, an AR assistant must derive state from pixels. The inventory of *Yokohama* includes specific challenges for object detection models.1

The "Assistant" components—small wooden cubes—are the most numerous and problematic. In a 4-player game, there are nearly 100 assistants on the board. They act as the "roads" for the President pawn. Crucially, they can be stacked. A standard overhead 2D camera sees a stack of three cubes as a single square, potentially indistinguishable from a stack of one. This **Stacking Occlusion** problem implies that the system cannot rely solely on vision for state maintenance; it must maintain a "logic history" and perhaps ask the user for clarification during setup or reconciliation phases.

The "President" pawns are taller and visually distinct, serving as the primary locators for player action. The resources (Copper, Silk, Tea, Fish) are distinct shapes and colors, making them excellent candidates for template matching or trained object detection, provided the resolution is sufficient. The board tiles themselves (Area Boards) are large rectangles with specific artwork (Bank, Port, Laboratory). Identifying these tiles and their relative positions is the first step in reconstructing the game graph.3

### **2.2 The Rule Structure as a State Machine**

The turn structure of *Yokohama* dictates the architecture of the Game Engine. It is not a simple "I go, you go" structure; it is a multi-phase operation involving conditional logic.10

The **Placement Phase** allows a player to place assistants. The rule allows placing "3 assistants on 3 different areas" OR "2 assistants on 1 area." This is a branching decision node. The Logic Engine must enforce valid placement: one cannot place assistants on the Canal (unless a specific tech is owned) or potentially on an opponent's President without paying a cost.

The **Movement Phase** is a pathfinding problem. The President can only move through a contiguous chain of their own assistants. This requires the board to be modeled as a graph where nodes are Area Boards and edges represent adjacency. The validity of a move from Node A to Node B is determined by the existence of a path $P \= \\{n\_1, n\_2,..., n\_k\\}$ where every $n$ in $P$ contains an assistant of the active player.11

The **Action Phase** introduces the concept of "Power." Power is calculated as the sum of the President, Assistants, Shops, and Trading Houses on the target tile. This value (ranging from 1 to 5\) determines the reward. The engine must query the state of the target node, sum these entities, and look up the corresponding reward in a lookup table (e.g., Power 4 at the Copper Mine yields 3 Copper, while Power 5 yields 4 Copper and a bonus).

### **2.3 Scoring Mechanics and Heuristics**

The "Point Salad" nature of *Yokohama* complicates the AI's objective function. There is no single "checkmate." Points are accrued through:

* **Order Fulfillment:** Trading resources for points (linear optimization).  
* **Church/Customs Majorities:** Area control mechanics that award points at the end of the game based on relative assistant counts (game theory/competitive equilibrium).  
* **Set Collection:** Collecting different country flags on cards provides exponential scoring bonuses.12  
* **Building Construction:** Placing Shops/Houses generates passive income and end-game points.

This implies that the AI cannot simply maximize "current turn points." It must maximize a **Heuristic Evaluation Function** that accounts for future potential. For instance, taking a low-scoring action now to place a Trading House might be optimal because it boosts all future actions at that location and secures area control points.4

## ---

**3\. Computational Logic and Game Engine Architecture**

The Logic Engine is the bedrock of the project. It serves as the simulator for the AI and the validator for the Computer Vision system. If the CV system detects an illegal board state (e.g., 5 Presidents on the board), the Engine provides the sanity check to reject the input.

### **3.1 State Representation and Immutability**

To support Monte Carlo Tree Search (MCTS), the game state must be copyable and forward-simulatable. A simulation involves cloning the current state, applying a move, advancing the game, and repeating this thousands of times. Therefore, the state data structures should be lightweight and, ideally, immutable (or support efficient copy-on-write).

We define the GameState class as a composite structure containing:

* **BoardGraph:** A representation of the physical layout.  
* **AreaState:** A dictionary mapping Tile IDs to their contents (Assistants list, President presence, Building types).  
* **PlayerState:** An array of objects tracking resources, hand cards, and active technology buffs for each player.  
* **GlobalTracks:** The state of the Church, Customs, and Management Boards (Order cards available).

**Python Data Classes** are an excellent choice here. Using @dataclass(frozen=True) ensures that the state cannot be accidentally mutated during an AI search, preventing subtle bugs where a "hypothetical" move accidentally changes the "real" game state.

### **3.2 Graph-Based Board Implementation**

The modular board of *Yokohama* is best represented using a graph library like **NetworkX**.13

* **Nodes:** Each Area Board is a node. Attributes include type (e.g., "Silk Mill"), capacity, and coordinates (for visualization).  
* **Edges:** Adjacency is represented by undirected edges.

Graph Construction Algorithm:  
At the start of a session, the CV system will identify the layout (e.g., a 4x5 pyramid). The Engine instantiates a NetworkX graph matching this topology.

Python

import networkx as nx

class YokohamaBoard:  
    def \_\_init\_\_(self, layout\_config):  
        self.graph \= nx.Graph()  
        self.areas \= {} \# Map ID \-\> AreaState  
        self.\_build\_graph(layout\_config)

    def get\_valid\_moves(self, current\_node\_id, player\_id):  
        \# Use BFS to find all reachable nodes via assistants  
        reachable \=  
        for neighbor in self.graph.neighbors(current\_node\_id):  
            if self.has\_assistant(neighbor, player\_id):  
                reachable.append(neighbor)  
                \# Recursively search (simplified for brevity)  
        return reachable

This graph approach allows the engine to instantly validate movement. When the AI wants to know "Can I move to the Bank?", the engine performs a traversal query on the graph, filtering edges based on the presence of the player's assistants.14

### **3.3 The Command Pattern for Actions**

Interaction with the engine should be encapsulated via the **Command Pattern**.15 Instead of calling methods like game.move\_president(), the AI or UI generates Action objects: MovePresidentAction(target=Bank), PlaceAssistantAction(targets=\[Port, Church\]).

This offers critical advantages:

1. **Serialization:** Action objects can be serialized to JSON and sent over a network (essential for the client-server architecture).  
2. **Undo/Redo:** The Engine can keep a stack of executed commands. If the AI wants to "take back" a move during a simulation, it simply calls action.undo().  
3. **Explainability:** The AI can output a list of Action objects which can be translated into natural language for the user: "First, place an assistant. Then, move your President."

### **3.4 Open Source Foundations**

Rather than writing a game loop from scratch, leveraging an existing framework like **OpenBoardGame** or **BoardGameArena (BGA)** logic patterns is advisable. The BGA documentation 17 details how to structure a PHP/SQL backend for board games, but for a local AI project, translating these concepts to Python is superior due to Python's dominance in the AI/ML ecosystem. A library like boardgame.io (JavaScript) handles state management well, but Python is preferred here for seamless integration with PyTorch/TensorFlow.19

## ---

**4\. Artificial Intelligence Strategy: The "Brain"**

Teaching the model to play *Yokohama* requires an agent that can navigate a large decision space with stochastic elements (drawing cards).

### **4.1 Algorithm Selection: MCTS vs. Minimax**

Classical **Minimax** (with Alpha-Beta pruning) dominates games like Chess because the state is fully deterministic and the search depth is manageable. However, *Yokohama* has a high branching factor (due to the combinatorics of placing assistants) and hidden information (random card draws). **Monte Carlo Tree Search (MCTS)** is the standard solution for modern board game AI (as seen in AlphaGo and agents for games like *Terra Mystica*).21

**Why MCTS for Yokohama?**

* **Asymmetric Growth:** MCTS focuses search effort on "promising" branches rather than exploring the entire tree evenly. In *Yokohama*, many moves are technically legal but strategically nonsensical (e.g., moving back and forth). MCTS quickly learns to ignore these.  
* **Anytime Algorithm:** MCTS can be stopped at any time (e.g., after 5 seconds of thinking) and will return the best move found so far. This is crucial for a consumer-facing app where latency matters.23

### **4.2 Implementing MCTS**

The MCTS agent operates in four phases 24:

1. **Selection:** Traverse the current game tree from the root to a leaf node using the **Upper Confidence Bound (UCT)** formula. This balances exploring new moves vs. exploiting moves known to be good.  
2. **Expansion:** If the leaf node is not a game-ending state, add one or more child nodes (possible moves) to the tree.  
3. **Simulation (Rollout):** Play out the game from the new child node to the end using a "light" policy (e.g., random moves or a greedy heuristic).  
4. **Backpropagation:** Update the win/loss stats of all nodes along the path based on the simulation result.

Optimization: Truncated MCTS  
Simulating a full game of Yokohama (approx. 20-30 turns per player) is computationally expensive. For a responsive assistant, we use Truncated MCTS. Instead of simulating to the very end, the agent simulates for a fixed depth (e.g., 3-5 rounds) and then applies a Heuristic Evaluation Function to estimate the value of the resulting state.

### **4.3 The Heuristic Evaluation Function**

The Evaluation Function $E(s)$ assigns a numerical score to a game state $s$. For *Yokohama*, this function is a linear combination of weighted factors 4:

$$E(s) \= w\_{vp} \\cdot VP \+ w\_{res} \\cdot V\_{res} \+ w\_{tech} \\cdot V\_{tech} \+ w\_{pos} \\cdot V\_{pos}$$

* **$VP$ (Victory Points):** The current score on the tracker.  
* **$V\_{res}$ (Resource Value):** The value of resources held. Not all resources are equal; "Import Goods" are generally worth more than "Copper."  
* **$V\_{tech}$ (Engine Value):** A score representing the player's engine. Owning Technology cards that discount actions or buildings that boost Power adds significant future value.  
* **$V\_{pos}$ (Positional Value):** A mobility score. A President surrounded by assistants has high mobility (high value); a President isolated on an empty board has low value.

**Tuning Weights ($w$):** Finding the correct weights is difficult manually. A practical approach is **Self-Play Optimization**. Create two versions of the AI with different weights and have them play 10,000 games against each other. Use a genetic algorithm or gradient descent to evolve the weights toward the optimal strategy.26

### **4.4 Handling Stochasticity**

*Yokohama* involves drawing cards (Orders, Technology). The AI cannot know what card will be drawn next. This is handled via **Determinization** in MCTS. During the Simulation phase, when a card needs to be drawn, the AI assumes a specific card is drawn (randomly sampled from the remaining deck). By averaging thousands of simulations with different random draws, the MCTS converges on a move that is robust against the randomness.25

## ---

**5\. Computer Vision and Perception Pipeline: The "Eyes"**

The requirement to "take a picture... and tell me what move to execute" demands a sophisticated Computer Vision (CV) pipeline. This system must bridge the gap between a noisy, uncontrolled real-world image and the structured JSON required by the Game Engine.

### **5.1 Image Acquisition and Perspective Correction**

A user taking a photo with a phone will rarely capture a perfect top-down view. The image will be angled, causing perspective distortion.

* **Homography Transformation:** The first step is to rectify the image. The system needs to find four known points on the board to calculate a Homography Matrix ($H$) that maps the distorted image to a flat 2D plane.28  
* **Fiducial Markers:** To make this robust, the physical setup should ideally include **ArUco markers** (small QR-code-like squares) placed at the corners of the play area. OpenCV can detect these markers instantly and compute the transformation matrix with sub-pixel accuracy.  
* **Without Markers:** If markers are not an option, the system must rely on **Feature Matching** (SIFT or ORB) to detect the corners of the Area Boards themselves. This is harder due to the variable board layout but feasible by detecting the strong rectangular contours of the tiles.30

### **5.2 Board Segmentation and Identification**

Once the image is flattened, the system must identify the layout.

* **Tile Segmentation:** Apply edge detection (Canny) and contour approximation to isolate individual Area Boards.  
* **Tile Classification:** A Convolutional Neural Network (CNN) is required here. A lightweight model (e.g., MobileNetV3) can be trained to recognize the artwork of the 18 distinct *Yokohama* tiles. The input is a cropped image of a tile; the output is its ID ("This is the Bank").  
* **Graph Reconstruction:** By analyzing the centroids of the detected tiles, the system calculates adjacency. If the distance between the centers of Tile A and Tile B is within a threshold, an edge is added to the Engine's graph.13

### **5.3 Object Detection: YOLOv8**

The core challenge is detecting the game pieces. *Yokohama* features small components that can be densely packed.

* **Model Architecture:** **YOLOv8 (You Only Look Once)** is the recommended architecture. It offers the state-of-the-art trade-off between inference speed and small-object detection accuracy.7  
* **Training Data Strategy:** Pre-trained models (trained on COCO dataset) detect cars and people, not meeples. You must create a custom dataset.  
  * **Synthetic Data:** Manually photographing and labeling thousands of board states is tedious. A superior approach is **Synthetic Data Generation**. Using a 3D tool like Blender, you can model the *Yokohama* components and board. A script can then randomly scatter these components on the board and render thousands of images from different angles and lighting conditions. Crucially, because the computer generates the image, it also generates the perfect "ground truth" bounding boxes automatically. This technique allows for training extremely robust models without hours of manual annotation.32  
* **Crowd Counting:** The "Stacking" problem remains. YOLO detects bounding boxes. If three cubes are stacked, YOLO might see one box or two overlapping boxes.  
  * **Logic-Assisted Vision:** The system should not rely on vision alone for counting. If the Logic Engine knows (from previous turns) that a stack has 3 cubes, and the vision system sees a "tall" cube, the system maintains the count of 3\. The UI should highlight these stacks and ask the user to verify counts periodically ("Is this stack height 2 or 3?").

### **5.4 State Mapping Algorithm**

The final step of the pipeline maps detections to logic.

1. **Coordinate Transform:** Convert the bounding box center $(u, v)$ from the image to logical coordinates.  
2. **Binning:** Determine which Tile contains the coordinate.  
3. **Aggregation:** Update the AreaState. If a "Red Meeple" is detected within the bounding polygon of the "Silk Mill," the Engine increments the Red Player's assistant count on the Silk Mill.

## ---

**6\. Hardware, Deployment, and Performance**

The choice of where to run these computations determines the user experience and the project cost.

### **6.1 Computational Requirements**

* **Computer Vision:** Running YOLOv8 detection on a 4K image requires significant matrix multiplication capabilities.  
* **AI (MCTS):** Running 1,000+ simulations per move requires high CPU single-threaded performance (or multi-core if parallelized).

### **6.2 Deployment Options**

**Option A: Cloud-Centric (Recommended for Performance)**

* **Architecture:** The mobile app is a "Thin Client." It captures the photo and uploads it to a backend server (AWS/GCP).  
* **Backend:** A Python **FastAPI** server running on a GPU instance (e.g., NVIDIA T4). It runs the YOLO inference and the MCTS solver.  
* **Pros:** High accuracy (can use larger YOLO models), fast MCTS (server CPU is faster than phone).  
* **Cons:** Latency (uploading 5MB image takes time), server costs ($$).

**Option B: Edge-Centric (Recommended for Privacy/Cost)**

* **Architecture:** Everything runs on the user's smartphone.  
* **CV:** YOLOv8 can be exported to **TFLite** (Android) or **CoreML** (iOS). Modern phones can run YOLOv8-Nano at 30 FPS.7  
* **AI:** MCTS is CPU-bound. On a modern Snapdragon or Apple A-series chip, a truncated MCTS is feasible but might be weaker than a server-side agent.  
* **Pros:** Zero server cost, works offline, instant feedback.  
* **Cons:** Drains battery, requires heavy optimization of the models.

**Option C: Local Server (The Hobbyist "Sweet Spot")**

* **Architecture:** The user runs the "Server" on their own laptop/PC on the same Wi-Fi network as their phone.  
* **Setup:** The phone app communicates with the PC via a local IP (e.g., 192.168.1.10:8000). The PC uses its GPU for vision and CPU for AI.  
* **Verdict:** This is the best scaffold for development. It avoids cloud bills and battery drain while offering full PC power.

### **6.3 Tooling Stack**

* **Language:** **Python 3.10+** (Universal standard for AI/CV).  
* **Vision Library:** **Ultralytics YOLOv8** \+ **OpenCV** (cv2).  
* **Graph Library:** **NetworkX**.  
* **API Framework:** **FastAPI** (High performance async).  
* **Frontend:** **Flutter** or **React Native** (Cross-platform).  
* **Data Annotation:** **Roboflow** (For managing the dataset).

## ---

**7\. Implementation Roadmap**

### **Phase 1: The "Headless" Engine (Weeks 1-4)**

* **Objective:** Build the Python logic engine that can play a full game of *Yokohama* in text mode.  
* **Tasks:**  
  * Implement GameState, Player, Board classes.  
  * Implement graph topology using NetworkX.  
  * Implement rule validation (President movement, placement legality).  
  * Implement GameLoop (Turn phases).  
  * **Milestone:** A script where two random-move bots play a valid game to completion.

### **Phase 2: The AI Agent (Weeks 5-8)**

* **Objective:** Create an agent that can beat a random bot consistently.  
* **Tasks:**  
  * Implement the MCTS class structure.  
  * Design the Heuristic Evaluation function (starting with simple weights).  
  * Run self-play experiments to tune weights.  
  * **Milestone:** The AI agent achieves \>90% win rate against the random bot.

### **Phase 3: The Vision Pipeline (Weeks 9-14)**

* **Objective:** Recognize the board state from a static image.  
* **Tasks:**  
  * **Data Gen:** Use Blender to create 500 synthetic images of the board \+ 100 real photos manually labeled via Roboflow.  
  * **Training:** Train YOLOv8-Nano on this dataset.  
  * **Integration:** Write the image\_to\_state() function that maps detections to the Engine's BoardGraph.  
  * **Milestone:** A script that takes a JPG and prints an accurate text representation of the board state.

### **Phase 4: Frontend & Integration (Weeks 15-20)**

* **Objective:** Connect the phone to the backend.  
* **Tasks:**  
  * Build a simple Flutter app with a Camera widget.  
  * Implement the API call to send images to the Python local server.  
  * Receive the JSON response (suggested move) and overlay an arrow on the screen.  
  * **Milestone:** A fully functional prototype where tapping the phone screen yields a strategic move suggestion.

## ---

**8\. Conclusion**

Building a model of *Yokohama* is a formidable engineering challenge that serves as a masterclass in modern AI system design. The project requires moving beyond simple "toy" problems into the realm of messy, real-world data and complex decision-making.

The feasibility analysis confirms that while challenges exist—particularly in the domains of occlusion-robust computer vision and real-time search optimization—they are solvable with current technology. By leveraging the **YOLOv8** architecture for perception and **Monte Carlo Tree Search** for reasoning, and binding them together with a robust **Graph-based Logic Engine**, one can create a system that not only understands the board but plays the game at a superhuman level. The recommended path is to begin with the Logic Engine; without a rigorous model of the game's universe, the AI has nothing to solve, and the eyes have nothing to see. The resulting system would be a pioneering example of Augmented Reality Board Gaming, blurring the line between physical play and digital assistance.

#### **Works cited**

1. Yokohama \- Synapses Games, accessed December 6, 2025, [https://www.jeuxsynapsesgames.com/products/yokohama](https://www.jeuxsynapsesgames.com/products/yokohama)  
2. Yokohama Board Game Review \- Tabletop Gaming, accessed December 6, 2025, [https://www.tabletopgaming.co.uk/reviews/yokohama-board-game-review/](https://www.tabletopgaming.co.uk/reviews/yokohama-board-game-review/)  
3. Game of the Week: Yokohama : r/boardgames \- Reddit, accessed December 6, 2025, [https://www.reddit.com/r/boardgames/comments/9f6d3t/game\_of\_the\_week\_yokohama/](https://www.reddit.com/r/boardgames/comments/9f6d3t/game_of_the_week_yokohama/)  
4. Yokohama Board Game Review | Blog \- Lelekan, accessed December 6, 2025, [https://lelekan.com.ua/en/articles/yokohama-game-review](https://lelekan.com.ua/en/articles/yokohama-game-review)  
5. Yokohama Review \- Board Game Quest, accessed December 6, 2025, [https://www.boardgamequest.com/yokohama-review/](https://www.boardgamequest.com/yokohama-review/)  
6. Yokohama – A Eurogame Classic That Still Holds Up \- Tabletopping, accessed December 6, 2025, [https://tabletopping.games/2025/04/16/yokohama/](https://tabletopping.games/2025/04/16/yokohama/)  
7. Run your YOLOv8 in an Android mobile\! \- GitHub, accessed December 6, 2025, [https://github.com/xforcevesa/YOLOv8-Mobile](https://github.com/xforcevesa/YOLOv8-Mobile)  
8. qualcomm/YOLOv8-Detection \- Hugging Face, accessed December 6, 2025, [https://huggingface.co/qualcomm/YOLOv8-Detection](https://huggingface.co/qualcomm/YOLOv8-Detection)  
9. 163-Piece Complete Upgrade for Yokohama \- Meeple Source, accessed December 6, 2025, [https://meeplesource.com/proddetail.php?prod=YokohamaComplete](https://meeplesource.com/proddetail.php?prod=YokohamaComplete)  
10. yokohama rules.pdf, accessed December 6, 2025, [http://boardgame.bg/yokohama%20rules.pdf](http://boardgame.bg/yokohama%20rules.pdf)  
11. Yokohama Game Review \- Meeple Mountain, accessed December 6, 2025, [https://www.meeplemountain.com/reviews/yokohama/](https://www.meeplemountain.com/reviews/yokohama/)  
12. Yokohama \- BGG How to Play \- YouTube, accessed December 6, 2025, [https://www.youtube.com/watch?v=xJaZFC8Bdi0](https://www.youtube.com/watch?v=xJaZFC8Bdi0)  
13. Graph Theory and Children's Games. | by Elijah Appelson | Medium, accessed December 6, 2025, [https://medium.com/@eappelson/graph-theory-and-childrens-games-6ca1711dd482](https://medium.com/@eappelson/graph-theory-and-childrens-games-6ca1711dd482)  
14. AI Movement in Games: Beyond A\*, accessed December 6, 2025, [https://tonogameconsultants.com/game-ai-movement/](https://tonogameconsultants.com/game-ai-movement/)  
15. Table of Contents \- Game Programming Patterns, accessed December 6, 2025, [https://gameprogrammingpatterns.com/contents.html](https://gameprogrammingpatterns.com/contents.html)  
16. Any patterns for modelling board games? \[closed\] \- Stack Overflow, accessed December 6, 2025, [https://stackoverflow.com/questions/361002/any-patterns-for-modelling-board-games](https://stackoverflow.com/questions/361002/any-patterns-for-modelling-board-games)  
17. Create a game in BGA Studio: Complete Walkthrough \- Board Game Arena, accessed December 6, 2025, [https://en.boardgamearena.com/doc/Create\_a\_game\_in\_BGA\_Studio:\_Complete\_Walkthrough](https://en.boardgamearena.com/doc/Create_a_game_in_BGA_Studio:_Complete_Walkthrough)  
18. Studio \- Board Game Arena, accessed December 6, 2025, [https://en.boardgamearena.com/doc/Studio](https://en.boardgamearena.com/doc/Studio)  
19. OpenBoardGameProject/obg-engine: A fully customizable open-source board game engine, accessed December 6, 2025, [https://github.com/OpenBoardGameProject/obg-engine](https://github.com/OpenBoardGameProject/obg-engine)  
20. boardgameio/boardgame.io: State Management and Multiplayer Networking for Turn-Based Games \- GitHub, accessed December 6, 2025, [https://github.com/boardgameio/boardgame.io](https://github.com/boardgameio/boardgame.io)  
21. Are there any AI models programmed to play and master modern board game designs? : r/boardgames \- Reddit, accessed December 6, 2025, [https://www.reddit.com/r/boardgames/comments/1e1qsc8/are\_there\_any\_ai\_models\_programmed\_to\_play\_and/](https://www.reddit.com/r/boardgames/comments/1e1qsc8/are_there_any_ai_models_programmed_to_play_and/)  
22. Leveraging Parallel Architectures in AI Search Algorithms for Games \- The Association for the Advancement of Artificial Intelligence, accessed December 6, 2025, [https://cdn.aaai.org/ojs/12693/12693-52-16210-1-2-20201228.pdf](https://cdn.aaai.org/ojs/12693/12693-52-16210-1-2-20201228.pdf)  
23. Injecting Combinatorial Optimization into mcts: Application to the Board Game boop. \- arXiv, accessed December 6, 2025, [https://arxiv.org/html/2406.08766v1](https://arxiv.org/html/2406.08766v1)  
24. Adapting the AlphaZero Algorithm to Pawn Dama: Implementation, Training, and Performance Evaluation \- DergiPark, accessed December 6, 2025, [https://dergipark.org.tr/en/download/article-file/4522861](https://dergipark.org.tr/en/download/article-file/4522861)  
25. M$^2$-Miner: Multi-Agent Enhanced MCTS for Mobile GUI Agent Data Mining, accessed December 6, 2025, [https://openreview.net/forum?id=Ms9MMN8STj](https://openreview.net/forum?id=Ms9MMN8STj)  
26. Building Placement Optimization in Real-Time Strategy Games \- Skat, accessed December 6, 2025, [https://skatgame.net/mburo/aiide14ws/papers/WS1-BuildingPlacement.pdf](https://skatgame.net/mburo/aiide14ws/papers/WS1-BuildingPlacement.pdf)  
27. (PDF) Building Placement Optimization in Real-Time Strategy Games \- ResearchGate, accessed December 6, 2025, [https://www.researchgate.net/publication/267268775\_Building\_Placement\_Optimization\_in\_Real-Time\_Strategy\_Games](https://www.researchgate.net/publication/267268775_Building_Placement_Optimization_in_Real-Time_Strategy_Games)  
28. The Power of Homographies · Computer Vision Learning Notes, accessed December 6, 2025, [https://shengchen-liu.github.io/gitbook\_computer\_vision/multiple\_views/The%20Power%20of%20Homographies.html](https://shengchen-liu.github.io/gitbook_computer_vision/multiple_views/The%20Power%20of%20Homographies.html)  
29. OpenCV \- Calculating the correct coordinates using homography matrix \- Stack Overflow, accessed December 6, 2025, [https://stackoverflow.com/questions/33242685/opencv-calculating-the-correct-coordinates-using-homography-matrix](https://stackoverflow.com/questions/33242685/opencv-calculating-the-correct-coordinates-using-homography-matrix)  
30. How to recognize hexagonal tiling in boardgame? \- Signal Processing Stack Exchange, accessed December 6, 2025, [https://dsp.stackexchange.com/questions/10358/how-to-recognize-hexagonal-tiling-in-boardgame](https://dsp.stackexchange.com/questions/10358/how-to-recognize-hexagonal-tiling-in-boardgame)  
31. FDD-YOLO: A Novel Detection Model for Detecting Surface Defects in Wood \- MDPI, accessed December 6, 2025, [https://www.mdpi.com/1999-4907/16/2/308](https://www.mdpi.com/1999-4907/16/2/308)  
32. Recognizing Catan Boards with Machine Learning \- HackMD, accessed December 6, 2025, [https://hackmd.io/@937dn7BYQSSGBulQVdbfFQ/r1Np9GoaJx](https://hackmd.io/@937dn7BYQSSGBulQVdbfFQ/r1Np9GoaJx)  
33. Ultralytics Android App: Real-time Object Detection with YOLO Models, accessed December 6, 2025, [https://docs.ultralytics.com/hub/app/android/](https://docs.ultralytics.com/hub/app/android/)