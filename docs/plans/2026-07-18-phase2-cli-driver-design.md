# Phase 2 — CLI game driver: design (2026-07-18)

**Status: DESIGN — approved for planning.** Follow-on implementation plan: `2026-07-18-phase2-cli-driver-plan.md` (written after this doc per the repo workflow brainstorm → spec → plan → TDD).

## 1. Goal

A CLI that drives the completed `yokohama-core` engine for two jobs that share one code path (decision log 2026-07-16):

- **Simulation / solo play**: new game from a seed, per-phase prompt with numbered legal options, apply, undo, save/load, ASCII state display.
- **Live tracking**: the same loop, entering a physical table game move-by-move, with the engine's seeded hidden state reconciled to observed reality as it reveals itself.

**Phase-2 gate (unchanged):** track one full physical game live, start to finish, without fighting the tool.

**This plan's own acceptance** is narrower than the phase gate: the driver complete and TDD-verified (a full simulated game playable through the CLI; every reconciliation flow integration-tested; suite + release fuzz gate green). The phase gate additionally requires the **choice-encoding backlog** (`2026-07-18-phase2-entry-backlog.md` item 1) — a physical game will exercise choices the v1 engine auto-resolves (any-good picks, warehouse-icon takes, church/customs space choice, University/Spinning Mill/Telephone), and until those have Move encodings the tracked state can diverge from the table. Choice encoding is deliberately **not in this plan**; it is the next dated plan after the driver exists to render its new options.

## 2. What exists (verified against source, 2026-07-18)

- `new_game(config, rules, seed) -> PendingSetup { state, deals }` (public fields) → `finalize_setup(pending, &[StartingOrderChoice]) -> GameState`. Setup fills Port A/B (6 slots each) and Lab A/B before finalize; `StartingOrderChoice::{Keep(0|1), Unknown}`; `Unknown` parks both dealt cards in `orders_removed` (hidden-fiction, decision log 2026-07-17).
- `get_legal_moves(&GameState, &Rules) -> Vec<Move>`; `apply_move(&GameState, &Move, &Rules) -> Result<GameState, MoveError>` — pure, clones, handles ALL phase advancement internally (Recovery automatic). The canonical driver loop is `tests/common/mod.rs::run_random_game_from`: `is_game_over` → `get_legal_moves` → apply → `check_invariants`.
- `GameRecord { engine_version, seed, config, choices, moves }` + `replay(record, rules)` (`game/record.rs`) — replay-determinism proven incl. JSON round-trip.
- Reconciliation primitives on `GameState`: `promote_order_to_top`, `promote_tech_to_top` (swap-with-top), `extract_order_from_deck` (searches deck then `orders_removed`).
- Display helpers (`game/display.rs`): `order_prompt_line` ("O17 [Britain] needs 2 copper + 1 fish -> 8 VP + 1 import"), `order_compact_line`, `site_compact_line`. Display language decided 2026-07-16: plain words in prompts, short readable abbreviations in tables; order identity = printed corner index (O1–O36, decision log 2026-07-17).
- Deck consumption sites (complete list): setup fills/deals; `execute_port_action` (compaction then `refill_port`, which pops into empty slots **rightmost-first**, slot 5→0); `execute_lab_action` (`refill_lab`, one pop into the rightmost empty slot). Nothing else pops either deck.
- `board_gen::generate_board_layout` randomizes THREE things from the seed: area-tile placement in the grid, building-site card per area, 5-Power effect per area. Tiles and sites are physically face-up at setup; **5-Power tokens are face-down and revealed only when taken** (KB 01-rules-reference.md:653). `BoardLayout.five_power_tokens: BTreeMap<AreaId, Option<FivePowerEffect>>` (None = claimed).
- `starting_order_hidden` has **no consumer after setup** — no reveal path exists yet; a hidden opponent cannot currently fulfill their starting order in a tracked game.
- Sequential `promote_*_to_top` calls cannot arrange a multi-card draw: each promote swaps with the current top and disturbs the previous arrangement. Port refills draw up to 2+ cards in one `apply_move`.

## 3. Clarifying questions, resolved from the written record

| # | Question | Resolution | Source |
|---|---|---|---|
| 1 | Is choice encoding in scope? | No — separate standing Phase-2 work; this plan is the driver. The phase gate waits for both. | Backlog item 1; directive scope list |
| 2 | Save format? | Exactly `GameRecord` JSON — one format serves save/undo/live-tracking/replay/training. | guidingpath Evidence base; directive |
| 3 | Undo mechanism? | Replay-prefix: drop the last event group, re-fold. No new state machinery. | guidingpath Phase-2 Testing path |
| 4 | Do reveals go in the record? | Yes. Replay must reproduce a reconciled deck; reveals become record events (see §5.2). A record whose replay diverges is the engine-change tripwire, so everything state-affecting must be in the record. | guidingpath persistence principle |
| 5 | Prompt/display language? | Plain words in prompts, short abbreviations in tables; orders as `O<corner-index>`; single-letter codes retired. | Decision log 2026-07-16 |
| 6 | Placement phase has 169–788 legal moves — numbered list? | Direct mini-grammar parsed to a `Move`, then **exact-matched against `get_legal_moves`** (never applied unless the generator offered it). Numbered list stays for every other phase and as fallback. | BGA thin-renderer principle (guidingpath Evidence base) |
| 7 | Must live mode observe the board layout? | Yes — tiles, sites, and achievements are seed-random but face-up; adjacency/sites/tokens drive legality, fees, and rewards. 5-Power tokens are face-down → reveal-on-take event instead. | board_gen source; KB:653/:673 |
| 8 | Hidden vs open seats in live mode? | Per-seat at the wizard: tracker's own seat(s) `Keep` with observed deal; table opponents `Unknown` + later `RevealStartingOrder` event. | Hidden-fiction decision row 2026-07-17 |
| 9 | Where does code live? | New `crates/yokohama-cli` (bin + lib); engine additions (record v2, reconcile, arrange, reveals) in `yokohama-core` — core stays MCTS-clean, presentation stays out of core. | Workspace conventions |
| 10 | Player names in the save? | Dropped (YAGNI): seats are identified by color, which IS the table identity. Pure `GameRecord` save preserved. | This design |
| 11 | `MoveError::WrongPhase` wiring (backlog line 28)? | Still deferred. `apply_move` already phase-validates every event during replay of untrusted files; defense-in-depth wiring is a riding item, revisited after the gate. | Backlog line 28 |
| 12 | Hand limit vs hidden starting orders? | Limit is 3 including the starting order (KB:261). A hidden opponent's engine hand omits it, so the projection is over-permissive — acceptable: tracking must never reject a real table move. `reveal_starting_order` does not enforce the limit (physical reality is authoritative). | KB:261/:446; I5 rationale |

## 4. Approaches considered

**A. Where reconciliation lives.**
1. *CLI-side state surgery* — CLI mutates `GameState` fields directly, records nothing. Rejected: breaks replay determinism, undo, and save/load; the record would lie.
2. **Engine event log (chosen)** — `GameRecord` becomes an event log (moves + reveals + setup observations); `replay` folds events. One format serves everything; sim records are the degenerate case (no reveals).
3. *Chance-node / determinization API* — model deck draws as explicit chance nodes. Rejected for Phase 2: heavy machinery whose only customer (ISMCTS) may never be needed (guidingpath: "hidden info light enough that determinization suffices").

**B. Undo.**
1. *State-stack snapshots* — clone per move. Works, but adds a second history mechanism alongside the record and can drift from it.
2. **Replay-prefix (chosen, pinned by guidingpath)** — truncate the event log, re-fold from setup. At ~877k applied moves/sec (2p release baseline) a full-game replay costs single-digit milliseconds; interactive-instant.

**C. CLI architecture.**
1. *Monolithic `main.rs` REPL* — untestable; rejected (TDD is non-negotiable here).
2. **Library `Session` + thin IO shell (chosen)** — the driver is a library (`yokohama-cli/src/lib.rs`) whose REPL step is a function of (session, input line) → (output text, session′); `main.rs` only shuttles stdin/stdout. Scripted-session tests are the TDD backbone.
3. *Full TUI (ratatui/egui)* — deferred by the front-end ladder (CLI → egui later); ASCII is explicitly acceptable deep into the project.

**D. How the CLI learns how many cards a move will draw (for reveal prompts).**
1. *Engine helper predicting draw counts per move* — new API surface, must track executor behavior forever.
2. **Scratch-apply diff (chosen)** — `apply_move` is pure, so the CLI applies the candidate to a throwaway clone and diffs `order_deck.len()`/`tech_deck.len()`. Zero new engine surface; automatically correct for every current and future draw site, including mercenary sub-turns.

**E. Live board layout.**
1. *Use the seed's random layout* — impossible: the physical table's tiles/sites would not match; area actions, fees, and build rewards would be wrong from turn 1.
2. *Parallel deterministic constructor in `board_gen`* — a second build path that must be kept in lockstep with the random one forever.
3. **Re-type the seeded layout (chosen)** — the board grid is fixed per player count (2×5 / 2×7 / 3×6, `board_gen::grid_dimensions`) and adjacency is grid-orthogonal and type-independent, so reconciliation only has to permute `area_type` over the fixed cells (validating it is a permutation of the same tile multiset), reassign building sites, and recompute the canal-crossing list via the existing derivation. No second constructor; nothing else in the layout changes.

## 5. The design

### 5.1 Architecture

BGA-style thin renderer: the engine is the single authority on prompts and options; the CLI renders `(state, get_legal_moves)` and translates input into engine calls. The main loop mirrors `run_random_game_from` exactly — `is_game_over` → `get_legal_moves` → chosen move → `apply_move` → `check_invariants` (always on: microseconds per check, and in live mode it is the desync tripwire). New engine Move variants (choice encoding, later) appear in the CLI automatically as new numbered options plus one `describe_move` arm.

### 5.2 Core engine additions (`yokohama-core`)

**Record format v2** (`game/record.rs`; no v1 files exist outside tests, clean break, `engine_version` already in the file):

```rust
pub struct GameRecord {
    pub engine_version: String,
    pub seed: u64,
    pub config: GameConfig,
    /// Physical-table observations reconciled into the seeded PendingSetup
    /// before finalize_setup. None = pure simulation. serde(default).
    pub setup_observations: Option<SetupObservations>,
    pub choices: Vec<StartingOrderChoice>,
    pub events: Vec<RecordedEvent>,          // replaces `moves`
}

pub enum RecordedEvent {
    Move(Move),
    /// The next draws from the order deck are exactly these, in draw order.
    /// Bound to the Move event that immediately follows (undo group).
    RevealOrders(Vec<OrderCardId>),
    /// Same for the tech deck.
    RevealTechs(Vec<TechCardId>),
    /// A hidden starting order became public (standalone undo group).
    RevealStartingOrder { player: PlayerId, card: OrderCardId },
    /// A face-down 5-Power token was revealed at `area`. Bound to the
    /// following Move (normally Take5PowerToken).
    RevealFivePowerToken { area: AreaId, effect: FivePowerEffect },
}
```

`replay` v2: `new_game` → `reconcile_setup` (when observations present) → `finalize_setup` → fold events (`Move` → `apply_move`; reveals → the state methods below). `GameRecord::push_move` survives as a convenience wrapping `RecordedEvent::Move`; `tests/common/mod.rs::run_recorded_game` and the record/fuzz determinism tests update mechanically.

**New `GameState` methods** (all deterministic, all conservation-preserving, all TDD'd):

- `arrange_orders_on_top(&mut self, cards: &[OrderCardId]) -> Result<(), MoveError>` — after the call, the next `cards.len()` pops yield `cards` in order. Every named card must currently be in `order_deck` (not `orders_removed`) and the list must be duplicate-free; else `CardNotFound`/`RuleViolation` with no mutation. Implementation: remove the named cards preserving the rest's order, then extend the deck tail with the list reversed. Subsumes multi-card refills that single promotes cannot express; `promote_*_to_top` remain (equivalent to the 1-element case).
- `arrange_techs_on_top(&mut self, cards: &[TechCardId])` — same for the tech deck.
- `reveal_starting_order(&mut self, player: PlayerId, card: OrderCardId) -> Result<(), MoveError>` — requires `starting_order_hidden`; `extract_order_from_deck(card)` (deck-then-removed search is exactly the hidden-fiction contract); then push to that player's hand, set `starting_order = Some(card)`, clear the flag. Deliberately no hand-limit check (§3 Q12).
- `reveal_five_power_token(&mut self, area: AreaId, effect: FivePowerEffect) -> Result<(), MoveError>` — if the layout already assigns `Some(effect)` at `area`, no-op Ok. Otherwise swap with the lowest-`AreaId` area whose **unclaimed** token equals `effect`; error if none exists (a real physical/engine mismatch, surfaced loudly). Token multiset conserved.

**Setup reconciliation** (`game/setup.rs`):

```rust
pub struct ObservedBoard {
    /// Physical area tile in each grid cell, row-major over
    /// `board_gen::grid_dimensions(player_count)`. Must be a permutation of
    /// the seeded layout's tile multiset (which already encodes the
    /// player-count removals).
    pub tiles: Vec<AreaType>,
    /// Building-site card (registry index, displayed as S<index>) per grid
    /// cell, row-major; None exactly where the tile carries no site (Canal).
    pub sites: Vec<Option<u8>>,
}

pub struct SetupObservations {
    pub board: Option<ObservedBoard>,
    pub achievements: Option<[AchievementType; 3]>,
    pub port_a: Option<[OrderCardId; 6]>,
    pub port_b: Option<[OrderCardId; 6]>,
    pub lab_a: Option<[TechCardId; 6]>,
    pub lab_b: Option<[TechCardId; 6]>,
    /// Observed two-card deals — normally only the tracker's own seat.
    pub deals: Vec<(PlayerId, [OrderCardId; 2])>,
}

pub fn reconcile_setup(pending: PendingSetup, obs: &SetupObservations)
    -> Result<PendingSetup, &'static str>
```

Semantics: **swap-based** for card zones — for each observed (zone, position, card), if the seeded occupant already matches, done; otherwise locate the card (deck, removed pile, any port slot, any deal; techs: deck or lab slot) and swap it with the occupant. Order cards can legally sit in `orders_removed` at this point (player-count removal happens before fills), so observed cards are pulled from there too; conservation holds because every operation is an exchange. Achievements are replaced outright (no conservation concern — the three cards are constructed fresh). `board` re-types the seeded layout per §4-E: permute `area_type` over the fixed grid cells, reassign `building_sites`, recompute `canal_crossings` through the existing derivation; adjacency and the 5-Power assignment are untouched (the latter stays seed-random — face-down physically — and gets corrected by reveal events as tokens are claimed). Validation: duplicate observations rejected; `tiles` must be a permutation of the seeded multiset; `sites` must be distinct, in-registry, and `None` exactly on the no-site tile; port_b/lab_b observations must match `config`; every id must exist in the registries. The multiset check doubles as a physical-verification tripwire: if the real table's tile set ever differs from the engine's player-count removals (the KB's 12/15/18 vs the rulebook-derived 10/14/18 already noted in `board_gen`), the wizard rejects it on the spot instead of silently mistracking — that outcome is a rules-reference correction (YAML/board_gen + fuzz rerun), not a driver bug. `Option` fields everywhere so partial observation (tests, hybrid setups) works; the live wizard fills everything observable.

Every field of `PendingSetup` this touches is already `pub`; `reconcile_setup` lives in core so the record's `setup_observations` can replay without CLI involvement.

### 5.3 CLI crate (`crates/yokohama-cli`)

Workspace member; deps: `yokohama-core`, `serde`, `serde_json` (workspace versions). No clap — an interactive wizard beats flags at a physical table; `main.rs` accepts only `[path.json]` (resume) or nothing (wizard).

```rust
pub enum TrackingMode { Simulation, Live }

pub struct Session {
    rules: Rules,
    record: GameRecord,
    state: GameState,
    mode: TrackingMode,
    save_path: Option<PathBuf>,   // autosave target; set by the wizard
}
```

- **Wizard** (new game): mode → player count → seat colors → start player → seed (entered or generated from OS entropy and printed — the record stores it either way). Simulation: show each seat's deal via `order_prompt_line`, pick keep 0/1. Live: walk the observations — board grid (area tile per cell by name), site card per area (`S<index>`), achievements, port/lab contents (leftmost→rightmost), own deal + keep, per-opponent hidden/known — then `reconcile_setup` + `finalize_setup`.
- **REPL step** = pure function of (session, line) → (output, session′); `main.rs` is a ~30-line stdin/stdout shuttle. All logic testable by feeding lines.
- **Prompt**: header (turn, seat color, phase, money/VP/goods one-liner) + numbered options via `describe_move` (plain words), paginated at 20 (`more`). Input: option number, or a direct-entry form, or a command.
- **Direct entry (Placement only, where enumeration is unusable)**: `a [id id id]` = Option A in 0–3 areas; `b <id> [t <adj>]` = Option B (+ Telegram). Parsed to a `Move`, exact-matched against the legal list; a miss prints why it isn't legal (or the nearest engine error), never bypasses the generator.
- **Commands**: `board` (ASCII grid), `players`, `ports`, `labs`, `hand`, `score`, `log` (event tail), `undo`, `save [path]`, `reveal <seat> O<n>` (live: hidden starting order), `help`, `quit`. Card entry: `O17`/`17`; techs by title prefix or `T<id>`; areas by numeric `AreaId` as shown on the board grid.
- **Post-apply delta line**: after every apply, print what changed (money/goods/VP deltas per player, cards gained/lost, phase transition). This is the at-the-table trust mechanism — the tracker confirms the engine agrees with the physical outcome without reading full state.
- **ASCII display**: board grid from `grid_position` (cell = abbreviated area name, `AreaId`, president letters R/B/Y/G, assistant counts, buildings), port/lab card tables (`order_compact_line`), church/customs tracks, deck counts, players panel. Correctness first; polish via dogfooding (the gate's "without fighting the tool" is the arbiter).
- **Autosave**: after every apply/undo when `save_path` is set; on in live mode by default (a crashed laptop must not lose a tracked game). Save = pretty-printed `GameRecord` JSON, nothing else.
- **Load**: full `replay` (per-event legality via `apply_move` is the validation), final `check_invariants`, engine-version mismatch surfaced as a visible banner (replay-anyway policy unchanged).

### 5.4 Live-mode choreography

**Deck draws.** When a chosen move would draw (detected by scratch-apply diff, §4-D): prompt for the observed cards **in pop order** before committing. Port: pops fill empty slots rightmost-first after total compaction, and the CLI tracks the board exactly, so it prompts slot-by-slot ("Port A slot 6 (High-right) now shows: O__?"). Lab: one card ("new tech in Lab A, rightmost open slot: ?"), title-matched to whichever physical copy's id is in the deck (copies are physically identical — either id is correct; ties resolve to the lowest id in deck for determinism). Then push `RevealOrders`/`RevealTechs`, `arrange_*_on_top`, apply the move, and **assert** the post-apply board zones equal what the user typed (mismatch = engine/CLI bug, loud failure). Simulation mode skips all of this — the seeded deck is the truth.

**Hidden starting orders.** When an opponent reveals theirs at the table (usually to fulfill it): `reveal <seat> O<n>` → standalone `RevealStartingOrder` event → their `FulfillOrder` becomes legal next prompt.

**5-Power tokens.** When a player takes one in live mode, prompt for the revealed effect (small `FivePowerEffect` menu), push `RevealFivePowerToken` (swap semantics, §5.2), then apply `Take5PowerToken`.

**Undo groups.** `undo` removes the trailing `Move` event *plus* any contiguous immediately-preceding bound reveals (`RevealOrders`/`RevealTechs`/`RevealFivePowerToken`), or a single trailing standalone event (`RevealStartingOrder`); then `state = replay(prefix)`. Bound reveals are only ever pushed in the same commit as their move, so the grouping invariant holds by construction.

### 5.5 Error handling

- Engine rejections (`MoveError` Display) print as-is with the offending input; the session state never changes on a failed apply.
- Reveal-entry errors (card not in deck, duplicate, wrong count) reprompt without recording anything; nothing is pushed until the whole group (reveals + move) applies cleanly — the record never contains a half-committed group.
- `check_invariants` failure after an apply is a bug (engine or driver), not user error: dump seed/turn/event log (the fuzz harness's `dump_failure` shape) and refuse further input except `save`/`quit`.
- Load failures (unparseable JSON, replay error at event k) report the event index and error, then exit (no partial sessions).

### 5.6 Testing (TDD throughout, per the repo's standing convention)

Engine (in `yokohama-core`):
- Unit tests per new method (arrange: order/duplicates/missing-card/no-mutation-on-error; reveal_starting_order: flag gate, removed-pile extraction, hand insertion; reveal_five_power_token: match/swap/exhausted; reconcile_setup: post-conditions, conservation, determinism, validation failures).
- **Reveal-equivalence property test**: a recorded random game and its "live twin" (identical moves, with `RevealOrders`/`RevealTechs` injected before each Port/Lab move naming exactly what the seeded deck would draw) replay to the identical final state.
- **Permuted-reveal property test**: inject *different* legal assignments — replay succeeds, invariants hold after every event, and the observed zones contain the injected identities.
- Record v2 JSON round-trip + replay determinism (extend `tests/game_record.rs`).

CLI (in `yokohama-cli`):
- Scripted-session tests: input-line vectors in, transcripts + final state/record out (covers wizard, numbered pick, placement grammar, reveals, undo-across-reveals, save/load round-trip, delta lines, invariant-failure lockdown).
- One scripted full simulated mini-game (fixed seed) as the end-to-end golden test.

Standing gates before merge: `cargo test -p yokohama-core` + `cargo test -p yokohama-cli`, zero warnings, then `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored`. The record-schema and setup changes touch the engine, so the fuzz gate is mandatory, and a 5000×3 selfplay sweep runs at phase close per the backlog's regression net.

### 5.7 Out of scope (riding items unchanged)

Choice-encoding Move variants (next plan; blocks the phase gate, not this plan); Station token placement on acquisition; from-hand Tram question; Canal topology (I4 mini-plan); `WrongPhase` defense-in-depth decision; player names; TUI/egui; any bot work (Phase 3); the backlog's minor cleanups except where a task touches that exact code anyway.

## 6. Decisions for Jeremy's veto (will be decision-logged on merge)

1. Record format v2 is a breaking schema change (`moves` → `events` + optional `setup_observations`); no migration shim since no v1 files exist outside tests.
2. Reveal events are engine concepts (in core, in the record), not CLI-side bookkeeping — one format, one replay path.
3. Live board observation covers tiles/sites/achievements at setup; 5-Power tokens reconcile on take (face-down per KB:653).
4. `reveal_starting_order` skips the hand-limit check (tracking never rejects physical reality).
5. Placement is the only direct-grammar phase in v1; everything else is numbered options.
6. Player names dropped from v1 (colors identify seats); save files are pure `GameRecord` JSON.
7. The phase gate explicitly waits for the choice-encoding follow-up plan; this plan's driver is gated on simulated-game completeness + reconciliation test coverage + the standing fuzz gate.
