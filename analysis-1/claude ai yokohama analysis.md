# Building an AI for the Yokohama board game with computer vision

**This project is feasible but challenging**—expect **6-9 months** for a working prototype as a hobby project. The critical insight: while chess CV+AI projects succeed because they leverage existing superhuman engines (Stockfish), Yokohama has no existing AI, meaning you must build both the game intelligence and the vision system from scratch. Start with the game engine, validate CV feasibility early with quick experiments, and build the system to gracefully degrade to manual state entry when vision fails.

Yokohama sits in an interesting complexity space—**BGG weight 3.29/5** with an estimated branching factor of **50-200 moves per turn** and nearly deterministic gameplay (luck rating 1/5). This makes it more tractable than hidden-information games like Poker but substantially harder than chess due to its multi-objective scoring, spatial network mechanics, and lack of established AI research. The good news: its low randomness and mostly public information make tree-search methods viable.

---

## Yokohama's complexity demands respect but not despair

Yokohama combines worker placement with network movement across a modular hexagonal board of **18 area tiles**. Each turn spans placement, movement, area actions, construction, and additional actions—creating interdependent decisions where optimal placement depends on intended movement paths three turns ahead. Victory comes from **six distinct scoring tracks**: order fulfillment, technology acquisition, building construction, achievements, church/customs majorities, and country set collection.

The estimated state space of **10^30 to 10^50 positions** puts Yokohama comparable to Chess (10^44) but with higher branching due to multi-phase turns. However, the near-deterministic nature (randomness only in setup) and mostly public information make this more tractable than it first appears. Monte Carlo Tree Search, which succeeded for Catan and Carcassonne AI, should work well here.

Component-wise, you're dealing with significant visual complexity: **20 assistants and 12 buildings per player**, four resource types, technology cards, order cards, and multiple track markers. This is substantially harder to detect than chess's 32 pieces on a uniform grid—a critical factor for the CV component.

---

## MCTS is your best path to competent game AI

For a hobby project targeting "competent play," **Monte Carlo Tree Search (UCT variant)** offers the optimal tradeoff between implementation complexity and playing strength. Pure MCTS with **1,000-2,000 simulations per move** can beat weak players without any domain knowledge, and adding Yokohama-specific heuristics to the rollout policy can push this to casual-competitive level.

The practical implementation timeline breaks down clearly. A random baseline agent takes days; basic MCTS takes 2-4 weeks; MCTS with domain heuristics requires 1-3 months; and AlphaZero-style neural network integration demands 3-6+ months plus significant GPU training time. For reference, the RL-trained Catan agents after weeks of training are "not quite at the level of a good human, but can still be fun to play against."

Several open-source frameworks provide excellent starting points. **alpha-zero-general** (4.3k GitHub stars) offers a clean, commented AlphaZero implementation that trained 6×6 Othello in roughly 3 days on a single GPU. **boardgame.io** (12.1k stars) handles turn-based game state management with built-in AI bot generation. **Catanatron** provides a complete Gymnasium RL environment for Catan that mirrors Yokohama's Euro-game mechanics.

The key architectural decision is whether to use Python throughout or hybrid Python/Rust. **Rust for MCTS tree search** with **Python for neural network inference** is the pattern used by top engines like KataGo—Rust eliminates Python's GIL limitations for the thousands of simulations MCTS requires, while Python's ML ecosystem remains unmatched for training and inference.

---

## Computer vision will be your hardest technical challenge

The core CV pipeline should follow this architecture: **image acquisition → preprocessing → board registration → component detection → state extraction → validation**. Traditional CV with YOLO object detection outperforms vision-language models for this use case—VLMs achieve only **70-80% accuracy** with unreliable spatial positioning, while properly trained YOLO models reach **85-95% accuracy**.

Board registration using OpenCV's homography transformation is well-documented and reliable. The recommended approach uses either ArUco fiducial markers on board corners or Canny edge detection with Hough transforms to find the board boundary, then applies `cv2.warpPerspective()` for top-down correction. A fixed overhead camera setup dramatically simplifies this—the Clio lite chess project moved to fixed cameras specifically because variable-angle CV proved too fragile.

For component detection, **YOLOv8** (Ultralytics) represents the current state-of-the-art balance of speed and accuracy. The nano model at just **3MB** runs in ~10ms on GPU; the small model at **22MB** provides better accuracy at ~20ms. Training requires annotated images—Roboflow handles annotation, augmentation, and export. Chess projects show this approach achieving 85-95% accuracy with proper training data.

Your biggest CV investment will be **dataset creation**. The Chess ID project required 10,000+ hand-labeled images to reach 99% accuracy. For Yokohama, plan for **500-2,000 annotated images** of your specific game components under your lighting conditions. Existing chess or Go datasets won't transfer—"The chess datasets I found online were created with either a different chess set or a different camera setup... which led me to create my own."

VLMs like GPT-4V should serve as a **validation layer** rather than primary detection. They excel at sanity-checking impossible game states and resolving ambiguities, but their spatial positioning remains unreliable and they struggle counting many small objects—both fatal limitations for board game state extraction.

---

## Local inference is entirely practical on modern hardware

The entire system can run locally on a modern laptop. **Apple Silicon** (M1/M2/M3) provides excellent ML performance through PyTorch's MPS backend—tasks that took 5 hours on CPU complete in 3 minutes on MPS. NVIDIA GPUs remain the gold standard with CUDA/TensorRT; even older cards like a GTX 1060 handle YOLOv8 inference easily.

For model deployment, **ONNX Runtime** provides cross-platform inference supporting CPU, NVIDIA CUDA, and Apple's CoreML execution provider from a single model file. This solves the "works on my machine" problem—train with PyTorch, export to ONNX, deploy anywhere.

Memory requirements are modest: YOLOv8 nano needs **~1GB RAM**, the small variant needs **~2GB**. If you want local LLM capabilities (for move explanation or validation), LLaVA 7B requires **16GB RAM** and runs via Ollama with a simple API. The complete CV pipeline including perspective correction, detection, and state extraction should achieve **50-300ms latency** on reasonable hardware—fast enough for comfortable interactive play.

Cloud APIs offer a fast prototyping path. GPT-4V costs roughly **$0.005 per high-detail image analysis**; at 100 moves per game playing 10 games daily, that's ~$5/day. Claude 3 Haiku drops this to ~$0.25/day. A hybrid approach—local YOLO for primary detection with cloud fallback for ambiguous cases—can achieve 90%+ local handling with ~10x cost reduction.

---

## The recommended tech stack balances speed and capability

For rapid prototyping, use **React/Vue frontend** with MediaDevices API for camera access, **OpenAI or Claude vision API** for initial CV, **FastAPI backend**, and focus on getting the game engine working. This validates the concept in weeks rather than months.

For the production system, move to **Electron + React** frontend (camera access via getUserMedia), **YOLOv8 via ONNX Runtime** for CV, **custom game engine in Python** (potentially with Rust MCTS), and FEN-style notation for game state serialization. The architecture flows: Camera → Image preprocessing (OpenCV) → Board registration (homography) → Component detection (YOLO) → State mapping → Game AI (MCTS) → Move recommendation → UI display.

Communication between components should use **JSON game state** rather than raw images between CV and AI. Define a schema covering board configuration, all piece positions, each player's resources and buildings, card states, and track positions. This creates a clean interface that allows independent testing of CV and AI components.

---

## A phased approach manages the substantial risks

**Phase 0 (1-2 weekends)**: Validate CV feasibility. Photograph your Yokohama board in 20 configurations, test basic edge detection, assess component color distinctiveness. If tiles aren't detectable with simple methods, reconsider scope.

**Phase 1 (4-6 weeks)**: Build the digital game engine. Implement complete Yokohama rules in Python, create a simple interface for manual play, test thoroughly. This has value even if CV ultimately fails—you'll have a Yokohama simulator.

**Phase 2 (4-6 weeks)**: Add basic AI. Implement MCTS with random rollouts, add simple heuristics (prioritize scarce resources, consider majorities). Target: makes moves a human finds reasonable.

**Phase 3 (6-8 weeks)**: Build CV pipeline. Fixed camera setup, consistent lighting, create labeled dataset, train YOLO model. Target: **85%+ accuracy** on test images under controlled conditions.

**Phase 4 (4-6 weeks)**: Integration and hardening. Connect components, build recommendation UI, handle errors gracefully, add manual correction capability.

The honest timeline for a working prototype is **6-9 months** at 10-15 hours weekly. The Raspberry Turk chess project (CV + robot arm + AI) took 5 months from an experienced software engineer working intensively.

---

## Conclusion: feasible with the right expectations

The project should proceed with three critical success factors. First, **the game engine provides standalone value**—build it first, and you'll have something useful even if CV proves intractable. Second, **define "done" pragmatically**—a system achieving 85% CV accuracy with easy manual correction that recommends moves humans find helpful is success; don't chase 99% accuracy into project death. Third, **build escape hatches**—design the system to work with manual state entry as the primary mode, with CV as an accelerator.

The major technical risks are CV accuracy under real conditions (high probability of issues), Yokohama's visual complexity exceeding what standard approaches handle (medium-high probability), and MCTS performance with Yokohama's large branching factor (medium probability). Mitigate these by validating CV early, using a fixed controlled camera setup, and implementing MCTS optimizations like progressive widening.

Reference projects demonstrate this class of system works: Raspberry Turk achieved end-to-end chess CV + robotics in 5 months; Chess ID reached 99% recognition accuracy with dedicated effort; Carcassonne CV tracked tile placement successfully as a class project. Yokohama is harder than these examples, but the core techniques transfer. Build incrementally, validate assumptions early, and you'll have a system that meaningfully enhances your board gaming experience.