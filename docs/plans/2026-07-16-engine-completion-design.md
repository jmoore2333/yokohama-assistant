# Engine Completion Design (merged Phases 0+1)

**Date:** 2026-07-16
**Status:** Design approved in principle; awaiting spec review, then implementation plan
**Goal:** `yokohama-core` can play complete, rules-correct games of Yokohama end-to-end — proven by a self-play fuzz gate — plus the environment and documentation fixes needed to work on it honestly.

## Context

The engine's logic modules are individually solid (156 tests), but four area actions (Port, Laboratory, Church, Customs) are stubbed in `apply_move`, legal-move generation for them is placeholder-only, president fees are never charged, and the card registries are empty. The root cause of the stubs is an unresolved design decision — deck contents don't live anywhere in `GameState`, so the already-written Port/Lab executors could never be wired in. This spec resolves the deck architecture, fills in the data, wires everything, and defines the acceptance gate the project always lacked. Background and rationale: `guidingpath.md`.

## Non-goals

- CLI UX (Phase 2 spec) — except the card display-code *format*, defined here because it lives in the registries.
- AI (Phase 3), computer vision (Phase 4).
- Mid-game full-state entry (escape hatch remains: edit a saved JSON state).
- Splitting composite moves into micro-decisions (deferred until the MCTS rung demands it).

## Part A — Environment & truth-up (ordered first)

- **A1. Fix the Rust toolchain.** `ln -sf /opt/homebrew/bin/rustup ~/.cargo/bin/rustup` (repairs every proxy symlink in that directory); verify plain `cargo test -p yokohama-core` passes; ensure `~/.cargo/bin` is on PATH in the user's shell profile. Add `rust-toolchain.toml` pinning `stable` so any machine resolves the same way.
- **A2. True up stale docs.** README.md status section and agent-handover.md get a short banner pointing at `guidingpath.md` as authoritative, and their "COMPLETE"/"100%" claims are corrected. Historical content otherwise untouched.
- **A3. (Optional, cheap) CI.** GitHub Actions workflow running `cargo test` on push, so green-ness is continuously visible.

## Part B — Rules context & card data

- **B1. `Rules` context struct.** Static, built once at startup, passed by reference:
  ```rust
  pub struct Rules {
      pub orders: OrderRegistry,        // 36 cards
      pub techs: TechRegistry,          // 20 cards
      pub sites: BuildingSiteSet,       // 24 cards
      pub five_power_tokens: FivePowerTokenSet, // 20 tokens (already canonical in board_gen)
  }
  ```
  API becomes `get_legal_moves(&GameState, &Rules)` and `apply_move(&GameState, &Move, &Rules)`. One breaking change, done first, all call sites and tests updated in the same sweep. While the API is open, `apply_move`'s error type upgrades from `&'static str` to a `MoveError` enum (the CLI and AI both want typed, matchable errors).

- **B2. Card verification without the chore.** Claude re-transcribes the three sprite sheets (`tts-assets/organized/cards/*.jpg`) fresh — without looking at the YAML — then diffs against `analysis-1/yokohama-kb/07..09-*.yaml` and produces a mismatch report. Jeremy adjudicates **only the mismatches**, with the physical game as ground truth where images are ambiguous. Each YAML file then gains a `verified: 2026-MM-DD` header. During this pass, confirm the assumption that *country + requirements* uniquely identifies every order card; if any two collide, note the disambiguating reward in the report (this matters for B4).

- **B3. Registry generation.** A small workspace tool crate (`crates/gen-data`, run via `cargo run -p gen-data`) renders the verified YAML into a checked-in `game/data.rs` providing `OrderRegistry::standard()`, `TechRegistry::standard()`, `BuildingSiteSet::standard()`. The generated file carries a DO-NOT-EDIT header; YAML remains the source of truth. (Data-in-code was already the December decision — this keeps it while making regeneration mechanical.)

- **B4. Card display codes — the table↔CLI "language."** Stored in the registries now, exercised by Phase 2. ASCII-first (per the CLI direction). Legend matches the existing verification report: C=Copper, S=Silk, T=Tea, F=Seafood; VP, Y=Yen, I=Import, M=Movement, W=Warehouse.
  - **Order cards** — the hard case at the table. Canonical id `O01..O36` plus a human-matchable line built from what's *visible on the card*: country + requirements → rewards. Example: `O17  GB  C2 F1 -> 8VP +1I`. Entry flows match by country + requirements (B2 verifies uniqueness).
  - **Tech cards** — by name (`Tram`, `Telegram`, …); names are unique.
  - **Building sites** — `S01..S24` plus VP signature, e.g. `S03  [2,3,4,5] TH:7`.

## Part C — Decks & chance

- **C1. Decks move into `GameState`.** `order_deck: Vec<OrderCardId>` and `tech_deck: Vec<TechCardId>` (last element = top). The `order_deck_remaining`/`tech_deck_remaining` counters are removed (derivable). Setup shuffles with the seeded RNG (`rand_chacha` moves from dev-dependency to dependency); `GameConfig` gains `setup_seed: u64`. Player-count deck thinning (10/5/0 orders removed) happens at setup from the seed.

- **C2. One code path for simulation and live tracking.** `apply_move` stays deterministic given the state: refills pop the deck (existing `refill_port`/`refill_lab` logic, now operating on state-owned decks). Live tracking reconciles reality with a reveal override: when the physically revealed refill card differs from the engine's assumed top card, the tracker calls `GameState::promote_to_top(card)` (swap within the deck) *before* applying the move. The CLI wraps this as a "which card was revealed?" prompt shown only when a refill occurs. No chance-node machinery in v1; AI determinization later just reshuffles the deck's unknown suffix.

- **C3. Starting order cards (the one hidden-info wrinkle).** Simulation/setup: deal 2 per player, a setup-phase choice records the keep (bots choose randomly until Phase 3). Live tracking: your own card is entered; opponents' stay `None` until revealed — when an opponent fulfills their hidden starting order, the CLI enters it as a reveal (card added to their hand, then fulfilled). Validation must therefore accept fulfillment of an order not previously tracked in hand *only* via this reveal path.

## Part D — Wire the stubbed actions

Each item: rulebook/KB citation in tests; TDD.

- **D1. Port.** Wire the existing `execute_port_action` into `apply_move` (signature adapts: deck now comes from state, registry from `Rules`). Covers slot access by power, 1–2 cards, second-card payment, hand limit 3, power-5 +3VP, slide + refill.
- **D2. Laboratory.** Wire `execute_lab_action` similarly: industry-value payment (power cap 5, 2Y=1IV, 1I=1IV), slot surcharges (+1/+2), fifth-tech penalty. Tech acquisition triggers the mercenary-eligibility check (already implemented for orders — extend to techs if not shared).
- **D3. Church.** Full placement: spend an assistant from hand onto the track (arrival order recorded), faith mapping 1→1, 2→1, 3→3, 4→4, 5→5, at most one of each good/Yen/import contributed, Winery bonus on placement, VP per the KB faith table.
- **D4. Customs.** Same shape: assistant from hand, imported goods paid ≤ power cap, Brickyard bonus, VP/track placement per KB.
- **D5. Placement fees.** Charge president fees during assistant placement using the existing `calculate_placement_cost`; unaffordable placements are illegal (legal-move generation must agree — see Part E).
- **D6. Production tech lookup.** Resolve `actions/production.rs`'s TODO (tech-modified production, e.g. Mining Technology) via `Rules.techs`.

Already-working paths (order fulfillment, achievements, 5-power tokens, building, movement, mercenaries) get regression coverage only.

## Part E — Real legal-move enumeration

Replace the placeholder `generate_area_action_for_type`:

- **Port:** accessible slots by power × take-1 or take-2 combinations × payment variants (2Y / 1I), filtered by hand limit and affordability.
- **Laboratory:** each acquirable card × minimal-cost payment mixes only (enumerate extra_yen/extra_imported only as needed to reach the required IV — no strictly-dominated overpayments).
- **Church:** subsets of {C,S,T,F,Y,I} with ≤1 each (≤64 raw), pruned by player stock and rules caps.
- **Customs:** each payable count `1..=min(power cap, imports held)`.
- **Employment Agency:** compositions (assistants, shophouses, trading houses) with total ≤ allowed(power), bounded by warehouse stock and money.
- **Chinatown:** *bounded* enumeration — exchange multisets up to a depth cap (default 4 exchanges), pruned by resources. The validator accepts any legal multiset beyond the cap (UI can construct one manually). **This is the one heuristically-bounded enumeration in the engine; the cap is logged, documented here, and revisited when the MCTS rung lands.**
- **Movement/Placement:** existing enumeration gets regression tests plus the fee-affordability filter from D5.

Fuzz runs (Part F) log per-phase branching factors so we know the real numbers before the AI phase.

## Part F — The gate: self-play fuzz + replay

- **F1. Random-agent driver** (`examples/selfplay.rs`): uniform choice over legal moves; runs N seeded games and prints stats.
- **F2. Fuzz acceptance test:** ≥5,000 seeded games across 2p/3p/4p, every game terminating within a turn bound, with per-step invariants:
  - Component conservation: per player, assistants in hand+board+warehouse+tracks = their total; orders in deck+ports+hands+completed+removed-at-setup = 36; techs = 20; shophouses/trading houses/5-power tokens conserved.
  - Non-negative resources; power ≤ 5; ≤1 shophouse per player per area; order hand limit ≤ 3.
  - Final scoring runs on every finished game.
  - Across the corpus, **all five end-game triggers observed**.
- **F3. Replay determinism.** `GameRecord { engine_version, seed, config, moves: Vec<Move> }` as serde JSON; re-applying a record reproduces an identical final state. This is the exact save/replay format Phase 2's CLI uses — built and tested here.
- **F4. Perf baseline** (games/sec, moves/sec) recorded in `guidingpath.md` — sets the MCTS simulation budget expectations for Phase 3.

**Definition of done: F2 + F3 pass and B2 verification is signed off.**

## Testing strategy

TDD throughout (repo convention). All 156 existing tests stay green through the B1 API sweep (mechanical update). Each Part lands with its own unit tests; Part F is the acceptance gate and stays in the suite permanently as regression.

## Risks & mitigations

- **Rule ambiguities surfaced by fuzzing** → resolve against rulebook + KB; record each ruling in `guidingpath.md`'s decision log.
- **YAML transcription errors** → B2's independent re-transcription diff; physical game as final arbiter.
- **API-change churn** → B1 does the whole breaking sweep (`Rules` + `MoveError`) once, first.
- **Enumeration blowups (Chinatown, Lab payments)** → minimal-cost/bounded enumeration with logged caps; revisit at MCTS time.

## Execution order

A → B1 → (B2 ∥ B3 prep) → C → D → E → F. B2 (verification diff) can start immediately and run in parallel with anything.
