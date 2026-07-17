# Yokohama Assistant — Guiding Path

**Created:** 2026-07-16 · **Living document** — update at every phase transition and when a design decision is made.

**This file is the authoritative statement of project state and direction.** Where README.md or agent-handover.md disagree (both carry stale "A1 COMPLETE" claims), this file wins.

---

## Mission & priority order (re-set 2026-07-16)

Build, in this order:

1. **Engine-validated game tracking** — a CLI where a live game (at the physical table, or solo) is entered move-by-move. The engine offers only legal options, validates everything, tracks full state, supports undo and save/load. Basic ASCII presentation is acceptable well into the project; the focus stays on the engine. A thin API layer can later back a web or local GUI for easier turn entry if wanted.
2. **A computer player that provably plays to win.** The bar is *not* "beats random" — it must leverage the rules and board state: country flags/sets in order, opponent positions, end-game timing (Yokohama scores build fast at the end), and a few moves of lookahead. It may not beat a strong human at first, but it must visibly play purposefully.
3. **Computer vision / AR assistant** — deferred. When it comes, CV is "another input source" feeding the same validation loop as (1): CV proposes, engine validates, UI confirms.

The original AR-first design (`docs/plans/2025-12-07-yokohama-assistant-design.md`) remains the long-term vision; its Python/FastAPI/React/VLM stack is deferred along with it.

---

## Current state — honest assessment (2026-07-16)

**Solid foundation:**
- ~9,000 lines in `crates/yokohama-core`; **156 tests green** (150 unit + 6 integration) on macOS, rustc 1.93.
- All state types serde-serializable; `GameState` is complete-and-resumable by design.
- Turn already decomposed into a phase state machine with `get_legal_moves()` / `apply_move()` — the exact contract both a move-entry UI and a search AI need.
- Board generation (incl. canonical 5-power tokens), power, movement/pathfinding with tech effects, scoring, achievements, end-game triggers: implemented with unit tests.
- All card data transcribed to YAML (`analysis-1/yokohama-kb/07..09-*.yaml`): 36 orders, 20 techs, 24 building sites — **pending verification**.
- Only hidden information in the whole game: each player's starting order card.

**Not done (despite "A1 COMPLETE" claims elsewhere):**

| Gap | Where |
|---|---|
| Port/Lab/Church/Customs actions stubbed in the state machine (Port/Lab no-op; Church/Customs never place assistants on tracks) | `apply_move.rs:316-335` |
| Full Port/Lab executors exist but are unwired — they need real deck contents; `GameState` only stores a count | `actions/cards.rs` vs `types/state.rs` |
| Legal-move generation for those areas emits one hardcoded placeholder each | `legal_moves.rs:301` |
| Placement president-fees computed but never charged | `apply_move.rs:206,223` |
| Card registries have no data (`::standard()` constructors don't exist) | `game/cards.rs` |

**Consequence:** no complete game has ever been simulated; two of five end-game triggers are unreachable. The missing acceptance gate was always "play thousands of full games."

**Environment:** `~/.cargo/bin` symlinks are broken on this machine (homebrew rustup artifact) — `cargo` isn't on PATH. Workaround in CLAUDE.md; permanent fix is Part A of the engine-completion spec.

---

## Why the Dec-2025 stall happened

1. **A manual chore as a hard gate** — verifying ~80 card transcriptions by hand blocked everything downstream.
2. **No usable artifact on the horizon** — the old plan ran engine → MCTS → PyO3 → FastAPI → React → VLM before anything was usable.
3. **An unresolved design decision** (where do deck contents live?) silently blocked integration, and docs papered over it with "complete."

**Working agreements to prevent recurrence:**
- Every phase ends with something usable on its own.
- No completion claims without an executable gate passing.
- This file stays current; stale docs get banners, not silent trust.

---

## Evidence base (July 2026 research)

- **The niche is empty.** BGA's Yokohama (2020) is human-only — BGA has no real AI framework at all; no other digital or open-source Yokohama implementation exists. This engine is the only one.
- **Euro-bot evidence (Catanatron leaderboard):** a hand-crafted value function + depth-2 expectiminimax *beats vanilla MCTS*; no neural net in the best bot. Evaluator first; search deepens later. (https://docs.catanatron.com/advanced/making-catanatron-stronger)
- **Don't wrap AI frameworks.** OpenSpiel/PettingZoo impose flat-integer action models or provide only plumbing; Polygames is dead; Ludii isn't wrappable. Write search in Rust against the engine's own API. PyO3→Python only if/when the AlphaZero-lite stretch goal happens.
- **Front-end ladder:** CLI/REPL → egui/eframe (one Rust codebase, native + browser WASM) → JS/React only if polish becomes a goal. Reference architecture: BGA's server-authoritative state machine — every sub-choice is a state emitting its own legal-option list; the UI is a thin renderer of `(prompt, legal options)`; savepoint undo; full-state hydration.
- **Persistence/replay:** no PGN-equivalent exists for euros; the proven pattern is `{seed, ruleset_version, ordered action log}` (Ludii trials, boardgame.io). One format serves save/undo/live-tracking/replay/training data. Keep `GameState` history-free so search clones stay cheap.
- **Hidden info:** light enough that determinization suffices; ISMCTS only if exploitation is observed.

---

## Roadmap

### Phase 0+1 (merged) — Engine completion ← CURRENT
**Spec:** `docs/plans/2026-07-16-engine-completion-design.md` (environment/doc fixes ordered first, then engine work).
**Plan:** `docs/plans/2026-07-16-engine-completion-plan.md` — 20 TDD tasks, **ready to execute** via subagent-driven development with tiered models (see the plan's "Execution Model" section: Haiku/Sonnet subagents for scripted tasks, a blind Fable subagent for card transcription, orchestrator-direct for rules rulings and the fuzz-fix loop).
**Gate:** ≥5,000 seeded random self-play games across 2p/3p/4p complete without panic, with per-step invariants; all five end triggers observed; card data verified; replay determinism (same seed+log → same final state).

### Phase 2 — CLI game driver
New-game-from-seed → per-phase prompt with numbered legal options → apply; undo; save/load `GameRecord` JSON; ASCII board & state display; the card display-code "language" for matching physical cards to CLI entries (order cards are the hard case — match by country + requirements).
**Gate:** track one full physical game live, start to finish, without fighting the tool.

### Phase 3 — Computer player ladder
Random → **hand-crafted value function (greedy)** → flat Monte Carlo → **depth-1/2 expectiminimax** → MCTS with truncated heuristic rollouts + progressive widening → *(stretch)* AlphaZero-lite (PyO3→PyTorch, or pure-Rust burn).
**First milestone bar (set by Jeremy):** the bot *provably plays to win* — purposeful play leveraging country flags/sets, opponent positions, end-game timing, short lookahead. Beating random is not the bar; sits between rungs 2 and 4, in practice: a real evaluator plus shallow search.
Bot-vs-bot arena with win-rate tracking; every rung stays as a regression opponent. CLI gains "play vs bot" and "suggest my move."

### Phase 4 — Vision
The 2025-12-07 design doc's VLM pipeline, landing on a proven engine + UI: CV proposes a state delta, engine validates, Phase-2 UI confirms/corrects.

---

## Decision log

| Date | Decision | Why |
|---|---|---|
| 2026-07-16 | CLI-first front end; ASCII fine deep into the project; optional thin API later for web/local GUI | Fastest usable artifact; focus stays on engine (Jeremy) |
| 2026-07-16 | Decks live inside `GameState` (seeded shuffle); live tracking reconciles observed reveals via promote-to-top; one code path for sim and live | Serves both goals without chance-node machinery |
| 2026-07-16 | Card display codes = table↔CLI "language"; order cards matched by country + requirements | Physical usability is the constraint (Jeremy) |
| 2026-07-17 | Card adjudication (Task 6 gate): blind re-transcription adopted wholesale over YAML — orders 33/36 corrected, techs 8 fixes, sites full re-model; physical-box spot-check runs in parallel, any reported deviation patches YAML + regenerated data | Jeremy's ruling; transcription was per-cell at native resolution, rulebook corroborates the systematic errors |
| 2026-07-17 | Order identity = printed corner index 1–36 (canonical `OrderCardId` and display code). SUPERSEDES country+requirements matching: GB #10 and #32 share requirements (1 fish 1 tea 2 copper) | Uniqueness disproven by re-transcription; corner index verified all-present-once |
| 2026-07-17 | Tech deck = 30 physical cards (20 titles; YAML gains `copies:`; engine deals 30 distinct ids) | Rulebook component list + sprite sheet both say 30 |
| 2026-07-17 | Building sites re-modeled: 4 positional shophouse spaces (TL always 1 VP, ML always blank, TR/MR printed rewards incl. goods/wild/yen/assistants/build/combos) + trading-house VP. SUPERSEDES `shophouse_vp: [a,b,c,d]` model (matched zero real cards) | Re-transcription + rulebook p.2 card anatomy |
| 2026-07-17 | Setup constants per rulebook: 2P order removal 16 (code had 10), starting money 3/4 (GameConfig had 7/8 and overwrites the correct PlayerState value). Applied in Task 9, which rewrites setup | Rulebook setup steps 9 & 12 are explicit; Jeremy approved |
| 2026-07-16 | Card verification: Claude re-transcribes sprite sheets and diffs vs YAML; Jeremy adjudicates only mismatches | Kills the chore that stalled the project |
| 2026-07-16 | Evaluator before MCTS; AI ladder per Catanatron evidence; no framework wrapping | Strength-per-effort; hobby-scale evidence |
| 2026-07-16 | First AI milestone: provably purposeful play (flags, positions, endgame timing, lookahead), not beats-random | Yokohama is deep; random-vs-random proves nothing (Jeremy) |
| 2026-07-16 | Keep current `Move` granularity; split composite moves into micro-decisions only when the MCTS rung demands it | Phase machine already decomposes turns; avoid speculative refactor |
| 2026-07-16 | Docs: dated specs/plans in `docs/plans/`; workflow brainstorm → spec → plan → TDD implementation | Repo convention continues |
| 2026-07-16 | Card verification is three-way (TTS sprites × YAML × rulebook/box manifest); physical game is final arbiter; content manifest must be signed off before the self-play gate | TTS mod is fan-made and can itself be wrong (Jeremy) |
| 2026-07-16 | CLI display language: plain words in prompts, short readable abbreviations in tables; settled by real Phase-2 use; verification-report single-letter legend retired from product use | Single letters not intuitive for CLI manipulation (Jeremy) |
| 2026-07-16 | Plan execution: subagent-driven with tiered models — Haiku/Sonnet for scripted tasks, blind Fable subagent for card transcription (Task 6), orchestrator-direct for rules rulings, human gates, and fuzz-failure fixes | Frontier capability only where it changes outcomes; the plan's zero-context detail makes the middle cheap (Jeremy) |
