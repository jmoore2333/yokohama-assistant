# Engine Completion Implementation Plan (merged Phases 0+1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `yokohama-core` play complete, rules-correct games of Yokohama end-to-end, proven by a self-play fuzz gate — plus the environment and documentation fixes to work on it honestly.

**Architecture:** Keep the existing pure-function engine (`get_legal_moves` / `apply_move` over a serde `GameState`). Add a `Rules` static-data context threaded through the API, move deck contents into `GameState` (seeded shuffle; live play reconciles reveals via promote-to-top), wire the four stubbed area actions, replace placeholder area-action enumeration, and finish with a random self-play fuzz suite + replay-deterministic `GameRecord`.

**Tech Stack:** Rust stable (1.93+), serde, rand + rand_chacha (promoted to full dependency), serde_yaml 0.9 + serde_json (gen-data tool only). No other new dependencies.

**Spec:** `docs/plans/2026-07-16-engine-completion-design.md`. **Two human gates:** Task 6 pauses for Jeremy's card adjudication; the manifest checkpoint before Task 18 requires the content manifest signed off.

## Execution Model (chosen 2026-07-16)

Subagent-driven (superpowers:subagent-driven-development): the orchestrator session (Fable) dispatches one fresh subagent per task via the Agent tool — task text verbatim plus the Global Constraints section — and reviews every diff between tasks. Use the Agent tool's `model` parameter per this table; the plan is written to zero-context detail precisely so cheaper models can execute the scripted middle:

| Tier | Tasks | Why |
|---|---|---|
| **Orchestrator-direct** (Fable, no subagent) | 1–3 (trivial — one inline pass); the Step-1 rules rulings of Tasks 13/14 and the optionality ruling in Task 17 (orchestrator resolves them against the KB BEFORE dispatching, and passes the ruling verbatim in the dispatch prompt); Task 18 Steps 4–5 (fuzz-failure diagnosis and fixes — open-ended debugging); both human-gate interactions; Task 20 (gate close-out narrative) | Judgment-heavy, or too small to be worth a dispatch |
| **Fable subagent** | Task 6 — MUST be a fresh subagent that has never seen the YAML files or `docs/plans/2025-12-08-card-verification-report.md`; blind vision transcription is the entire value (the orchestrator itself is contaminated — it has read rows of the old report) | Ground-truth data; vision accuracy determines everything downstream |
| **Sonnet subagent** | 4, 5, 7, 9, 11, 12, 15, 16, 17 (implementation steps), 18 Steps 1–3 (harness scaffolding), 19 | Careful mechanical work; the plan supplies the code |
| **Haiku subagent** | 8, 10 | Fully scripted, low blast radius |

Scheduling: dispatch Task 6 immediately and **in parallel** with Tasks 1–5 (it depends on nothing and gates Task 7). Everything else is sequential. Between tasks the orchestrator reviews the diff, runs the suite, and only then dispatches the next task.

## Global Constraints

- All engine code stays in `crates/yokohama-core`; `GameState` stays history-free and cheap to clone (no `Rc`, no interior mutability, no move history in state).
- Every task: TDD (failing test → minimal code → green), then commit. All pre-existing tests stay green (mechanical updates allowed when an API changes).
- Until Task 1 lands, run cargo as: `TCBIN="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"; PATH="$TCBIN:$PATH" "$TCBIN/cargo" test -p yokohama-core`. After Task 1: plain `cargo test -p yokohama-core`. Steps below write plain `cargo`; substitute the long form only if Task 1 hasn't landed.
- Never prepend `~/.cargo/bin` to PATH while its symlinks are broken (dangling `rustc` shadows the real one).
- Commit messages follow repo style: `feat: …` / `fix: …` / `test: …` / `chore: …` / `docs: …`, with body when non-obvious.
- Rules questions are settled by `analysis-1/yokohama-kb/01-rules-reference.md` + the rulebook PDF (`analysis-1/00-yokohama-rulebook.pdf`); every ruling made during implementation gets a row in `guidingpath.md`'s decision log.

---

### Task 1: Fix the Rust toolchain (spec A1)

**Files:**
- Create: `rust-toolchain.toml`
- Machine config: `~/.cargo/bin/rustup` symlink (outside repo — confirm with Jeremy if prompted)

- [ ] **Step 1: Repair the rustup proxy symlinks**

```bash
ln -sf /opt/homebrew/bin/rustup "$HOME/.cargo/bin/rustup"
```

(Every other entry in `~/.cargo/bin` is a relative symlink to `rustup` in the same dir; creating that one target fixes all of them.)

- [ ] **Step 2: Verify plain cargo works**

Run: `PATH="$HOME/.cargo/bin:$PATH" cargo --version && PATH="$HOME/.cargo/bin:$PATH" cargo test -p yokohama-core 2>&1 | tail -3`
Expected: `cargo 1.93.x`, then `test result: ok. 150 passed` … `ok. 6 passed`. If the user's shell profile doesn't already have `~/.cargo/bin` on PATH, tell Jeremy to add `export PATH="$HOME/.cargo/bin:$PATH"` (do not edit shell profiles unprompted).

- [ ] **Step 3: Pin the toolchain**

Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
```

- [ ] **Step 4: Verify pin resolves**

Run: `cargo test -p yokohama-core 2>&1 | tail -3`
Expected: same green results.

- [ ] **Step 5: Commit**

```bash
git add rust-toolchain.toml
git commit -m "chore: pin stable toolchain; document cargo symlink repair"
```

---

### Task 2: True up stale status docs (spec A2)

**Files:**
- Modify: `README.md` (top, after the title) and `agent-handover.md` (top, after the title)

- [ ] **Step 1: Add banner to README.md** directly under the `# Yokohama AI Assistant` heading:

```markdown
> **Status note (2026-07-16):** The status tables below are historical and overstate completion.
> `guidingpath.md` is the authoritative status and roadmap; the engine-completion work is specified
> in `docs/plans/2026-07-16-engine-completion-design.md`.
```

- [ ] **Step 2: Add the same banner to `agent-handover.md`** under its `# Agent Handover…` heading, and change its `**Status:**` line to end with `| SUPERSEDED BY guidingpath.md`.

- [ ] **Step 3: Verify + commit**

Run: `head -8 README.md agent-handover.md` — banners visible.

```bash
git add README.md agent-handover.md
git commit -m "docs: banner stale status docs; guidingpath.md is authoritative"
```

---

### Task 3: CI test workflow (spec A3, optional but cheap)

**Files:**
- Create: `.github/workflows/test.yml`

- [ ] **Step 1: Write the workflow**

```yaml
name: test
on:
  push:
  pull_request:
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test -p yokohama-core
```

- [ ] **Step 2: Commit** (verification happens on first push; do not push unless Jeremy asks)

```bash
git add .github/workflows/test.yml
git commit -m "chore: add CI workflow running cargo test"
```

---

### Task 4: `MoveError` enum (spec B1, part 1)

**Files:**
- Create: `crates/yokohama-core/src/types/errors.rs`
- Modify: `crates/yokohama-core/src/types/mod.rs` (add `mod errors; pub use errors::*;`)
- Modify: `crates/yokohama-core/src/game/apply_move.rs` (all `Result<…, &'static str>` fns)
- Modify: every test asserting on error strings (find them in Step 4)

**Interfaces:**
- Produces: `MoveError` — used as `apply_move`'s error type by every later task.

- [ ] **Step 1: Write the failing test** (in `types/errors.rs` `#[cfg(test)] mod tests`)

```rust
#[test]
fn move_error_displays_reason() {
    let e = MoveError::RuleViolation("Not enough assistants in hand");
    assert_eq!(format!("{}", e), "Not enough assistants in hand");
    assert_eq!(format!("{}", MoveError::WrongPhase), "move not legal in current phase");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p yokohama-core move_error_displays_reason`
Expected: FAIL — `MoveError` not found.

- [ ] **Step 3: Implement `types/errors.rs`**

```rust
//! Typed errors for move application.

use serde::{Deserialize, Serialize};

/// Why a move could not be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveError {
    /// Move enum variant does not match the current TurnPhase.
    WrongPhase,
    NotEnoughMoney,
    NotEnoughAssistants,
    NotEnoughResources,
    HandLimitExceeded,
    CardNotFound,
    InvalidSlot,
    IllegalDestination,
    /// Catch-all carrying the legacy message; migrate call sites to
    /// specific variants opportunistically when touching them.
    RuleViolation(&'static str),
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveError::WrongPhase => write!(f, "move not legal in current phase"),
            MoveError::NotEnoughMoney => write!(f, "not enough money"),
            MoveError::NotEnoughAssistants => write!(f, "not enough assistants"),
            MoveError::NotEnoughResources => write!(f, "not enough resources"),
            MoveError::HandLimitExceeded => write!(f, "order hand limit (3) exceeded"),
            MoveError::CardNotFound => write!(f, "card not found"),
            MoveError::InvalidSlot => write!(f, "invalid slot"),
            MoveError::IllegalDestination => write!(f, "illegal destination"),
            MoveError::RuleViolation(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for MoveError {}
```

- [ ] **Step 4: Migrate `apply_move.rs` signatures mechanically**

Change every `-> Result<(), &'static str>` / `-> Result<GameState, &'static str>` in `game/apply_move.rs` to use `MoveError`, wrapping each existing `Err("…")` as `Err(MoveError::RuleViolation("…"))`. Do NOT chase every helper in other files yet — only what `apply_move.rs` needs to compile (helpers in `actions/` keep `&'static str` for now and get wrapped at the call site with `.map_err(MoveError::RuleViolation)`).

Find affected tests: `grep -rn 'unwrap_err()' crates/yokohama-core | grep -v errors.rs` — update string assertions to match on the enum (e.g. `assert!(matches!(result, Err(MoveError::RuleViolation("Not enough money for movement"))))`).

- [ ] **Step 5: Run full suite**

Run: `cargo test -p yokohama-core`
Expected: all tests pass (count unchanged +1).

- [ ] **Step 6: Commit**

```bash
git add -A crates/yokohama-core
git commit -m "feat: typed MoveError for apply_move"
```

---

### Task 5: `Rules` context threaded through the API (spec B1, part 2)

**Files:**
- Create: `crates/yokohama-core/src/game/rules.rs`
- Modify: `crates/yokohama-core/src/game/mod.rs` (add `mod rules; pub use rules::*;`)
- Modify: `crates/yokohama-core/src/game/apply_move.rs`, `legal_moves.rs` (signatures), every caller/test fixture

**Interfaces:**
- Produces (used by ALL later tasks):
  - `pub struct Rules { pub orders: OrderRegistry, pub techs: TechRegistry, pub building_sites: Vec<BuildingSite>, pub five_power_tokens: Vec<FivePowerEffect> }`
  - `Rules::empty_for_tests() -> Rules` (empty registries, canonical tokens, no sites)
  - `pub fn get_legal_moves(state: &GameState, rules: &Rules) -> Vec<Move>`
  - `pub fn apply_move(state: &GameState, mv: &Move, rules: &Rules) -> Result<GameState, MoveError>`
- Consumes: `MoveError` (Task 4). `Rules::standard()` arrives in Task 7.

- [ ] **Step 1: Write the failing test** (in `game/rules.rs`)

```rust
#[test]
fn empty_rules_for_tests_has_canonical_tokens() {
    let rules = Rules::empty_for_tests();
    assert_eq!(rules.five_power_tokens.len(), 20);
    assert!(rules.orders.iter().next().is_none());
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p yokohama-core empty_rules_for_tests` → FAIL (no `Rules`).

- [ ] **Step 3: Implement `game/rules.rs`**

```rust
//! Static game data bundled for the engine API.

use crate::game::board_gen::canonical_five_power_tokens;
use crate::game::cards::{OrderRegistry, TechRegistry};
use crate::types::{BuildingSite, FivePowerEffect};

/// All static card/token data the engine needs. Built once, passed by reference.
#[derive(Debug, Clone)]
pub struct Rules {
    pub orders: OrderRegistry,
    pub techs: TechRegistry,
    pub building_sites: Vec<BuildingSite>,
    pub five_power_tokens: Vec<FivePowerEffect>,
}

impl Rules {
    /// Empty registries with canonical tokens — for unit tests that don't need card data.
    pub fn empty_for_tests() -> Self {
        Self {
            orders: OrderRegistry::new(),
            techs: TechRegistry::new(),
            building_sites: Vec::new(),
            five_power_tokens: canonical_five_power_tokens(),
        }
    }
}
```

- [ ] **Step 4: Collapse the optional-registry variants**

In `apply_move.rs`: make `pub fn apply_move(state: &GameState, mv: &Move, rules: &Rules) -> Result<GameState, MoveError>` the ONLY entry point; delete `apply_move_with_registries` and fold its body in (`order_registry: Option<&OrderRegistry>` becomes `&rules.orders`). In `legal_moves.rs`: `pub fn get_legal_moves(state: &GameState, rules: &Rules) -> Vec<Move>`; delete `get_legal_moves_with_tech` (its `tech_registry: Option<&TechRegistry>` becomes `&rules.techs`; the `is_some_and(...)` checks become direct lookups). Internal helpers (`fulfill_order`, movement/tech helpers) that took individual registries now take what they need from `&Rules` at the call site — do not change their own signatures unless required to compile.

- [ ] **Step 5: Sweep the call sites**

Find them: `grep -rn "apply_move(\|get_legal_moves(\|_with_registries\|_with_tech" crates/yokohama-core --include="*.rs"`. Update every test to construct `let rules = Rules::empty_for_tests();` and pass `&rules`. Test fixtures that duplicate `GameState` literals stay untouched in this task.

- [ ] **Step 6: Run full suite** — `cargo test -p yokohama-core` → all green.

- [ ] **Step 7: Commit**

```bash
git add -A crates/yokohama-core
git commit -m "feat: Rules context threaded through legal_moves/apply_move; drop optional-registry variants"
```

---

### Task 6: Three-way card verification (spec B2) — ends in a HUMAN GATE

> **Dispatch note:** fresh **Fable** subagent, prompt built ONLY from this task's text — do not include or reference the YAML contents or the 2025-12-08 verification report, and instruct the subagent not to open the YAML until Step 3's diff. Can run in parallel with Tasks 1–5.

**Files:**
- Create: `docs/plans/2026-07-16-card-verification/retranscription-orders.md`, `retranscription-techs.md`, `retranscription-sites.md`, `content-manifest.md`
- Modify (after adjudication): `analysis-1/yokohama-kb/07-order-cards.yaml`, `08-tech-cards.yaml`, `09-building-sites.yaml` (corrections + `verified:` header)

**Interfaces:**
- Produces: verified YAML (consumed by Task 7's generator) and `content-manifest.md` (consulted by the pre-Task-18 checkpoint).

- [ ] **Step 1: Blind re-transcription.** Read each sprite sheet image with the Read tool — `tts-assets/organized/cards/order_cards_8x5.jpg`, `tech_cards_8x4.jpg`, `building_site_cards_5x5.jpg` — WITHOUT opening the YAML files, and write a fresh transcription per sheet into the three `retranscription-*.md` files using position coordinates `(row,col)` and the same field names the YAML uses (country / requirements / rewards; effect / industry_cost / country; shophouse VPs / trading-house VP / bonus icons).

- [ ] **Step 2: Rulebook manifest.** Read the component list and setup pages of `analysis-1/00-yokohama-rulebook.pdf` (Read with `pages`) and record in `content-manifest.md` what the game SHOULD contain: total counts (orders/techs/sites/5-power tokens/mercenary flags), order-country distribution if stated, tech names+costs if listed, **order cards removed per player count** (code says 10/5/0; the Dec-2025 audit claimed 16 for 2P — resolve this), starting money per turn order (code has two conflicting values: `PlayerState::new` says 3/4, `GameConfig::starting_money` says 7/8 — resolve this too), and assistants per player (code: 8 hand + 12 warehouse).

- [ ] **Step 3: Three-way diff.** Compare re-transcription × YAML × manifest; write every mismatch into `content-manifest.md` as a table: `component | position | YAML says | re-transcription says | rulebook/box says | proposed ruling`. Also verify: does country+requirements uniquely identify each order card? Record the answer (matters for display codes).

- [ ] **Step 4: ⛔ PAUSE — human gate.** Present the mismatch table to Jeremy; he adjudicates (physical box is final arbiter). Do not proceed to Step 5 without his rulings.

- [ ] **Step 5: Apply rulings.** Correct the YAML files, add header comment `# verified: <date> — three-way (sprites x rulebook x physical), see docs/plans/2026-07-16-card-verification/` to each, mark `content-manifest.md` status per component (`VALIDATED` / `UNRESOLVED`).

- [ ] **Step 6: Commit**

```bash
git add docs/plans/2026-07-16-card-verification analysis-1/yokohama-kb
git commit -m "docs: three-way card verification (sprites x YAML x rulebook); YAML corrected and marked verified"
```

---

### Task 7: Data generator → `standard()` registries (spec B3)

**Files:**
- Create: `crates/gen-data/Cargo.toml`, `crates/gen-data/src/main.rs`
- Create (generated): `crates/yokohama-core/src/game/data.rs`
- Modify: root `Cargo.toml` (workspace member), `crates/yokohama-core/src/game/mod.rs` (`mod data;`), `game/cards.rs` (add `standard()` constructors), `game/rules.rs` (add `Rules::standard()`)

**Interfaces:**
- Consumes: verified YAML (Task 6), `OrderRegistry::insert(card, code)` / `TechRegistry::insert(card, code)` (existing).
- Produces: `OrderRegistry::standard()`, `TechRegistry::standard()`, `standard_building_sites() -> Vec<BuildingSite>`, `Rules::standard()` — used by setup (Task 9) and everything after.

- [ ] **Step 1: Write the failing tests** (in `game/data.rs`'s test module — write tests first in `game/cards.rs` tests referencing `OrderRegistry::standard()`)

```rust
#[test]
fn standard_registries_have_full_counts() {
    let orders = OrderRegistry::standard();
    assert_eq!(orders.iter().count(), 36);
    let techs = TechRegistry::standard();
    assert_eq!(techs.iter().count(), 20);
    assert_eq!(standard_building_sites().len(), 24);
}

#[test]
fn order_country_plus_requirements_is_unique() {
    let orders = OrderRegistry::standard();
    let mut seen = std::collections::HashSet::new();
    for card in orders.iter() {
        assert!(seen.insert((card.country, card.requirements.copper, card.requirements.silk,
                             card.requirements.tea, card.requirements.seafood)),
                "duplicate country+requirements: {:?}", card);
    }
}
```

(If Task 6 found duplicates, replace the second test with the adjudicated disambiguation rule.)

- [ ] **Step 2: Run to verify failure** — `cargo test -p yokohama-core standard_registries` → FAIL.

- [ ] **Step 3: Implement the generator.** `crates/gen-data/Cargo.toml`:

```toml
[package]
name = "gen-data"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
serde = { workspace = true }
serde_yaml = "0.9"
```

`main.rs`: deserialize the three YAML files (structs mirroring their fields; map YAML countries `US/DE/NL/FR/GB` → `Country::American/German/Dutch/French/English`; map YAML tech `effect` strings → `TechEffect` variant names verbatim), then emit `crates/yokohama-core/src/game/data.rs` with a `// GENERATED by gen-data — DO NOT EDIT` header containing three functions returning `Vec<…>` literals:

```rust
pub(crate) fn standard_orders() -> Vec<(OrderCard, &'static str)> { vec![ /* (card literal, "O01"), … */ ] }
pub(crate) fn standard_techs() -> Vec<(TechCard, &'static str)> { /* codes: "tech_tram", … */ }
pub(crate) fn standard_sites() -> Vec<BuildingSite> { /* 24 literals */ }
```

Check `types/board.rs` for `BuildingSite`'s exact fields before emitting; include per-slot bonus icons only if the type already models them — if it doesn't, record that as a follow-up in `content-manifest.md` rather than changing the type here.

- [ ] **Step 4: Wire constructors.** In `game/cards.rs`:

```rust
impl OrderRegistry {
    pub fn standard() -> Self {
        let mut r = Self::new();
        for (card, code) in crate::game::data::standard_orders() { r.insert(card, code); }
        r
    }
}
// TechRegistry::standard() identically; pub fn standard_building_sites() -> Vec<BuildingSite> re-exported from data.
```

In `game/rules.rs` add `Rules::standard()` composing all four.

- [ ] **Step 5: Generate and test** — `cargo run -p gen-data && cargo test -p yokohama-core` → green, counts pass.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/gen-data crates/yokohama-core
git commit -m "feat: gen-data tool and generated standard registries from verified YAML"
```

---

### Task 8: Display layer (spec B4)

**Files:**
- Create: `crates/yokohama-core/src/game/display.rs` (+ `mod display; pub use display::*;` in `game/mod.rs`)

**Interfaces:**
- Consumes: `OrderCard`, `TechCard`, `BuildingSite`, registries (Task 7).
- Produces: `order_prompt_line(&OrderCard) -> String`, `order_compact_line(&OrderCard) -> String`, `site_compact_line(index: usize, &BuildingSite) -> String` — Phase 2 CLI consumes these; fuzz stats may use compact lines.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn order_prompt_line_is_plain_words() {
    let card = OrderCard {
        id: OrderCardId(17),
        country: Country::English,
        requirements: TradeGoodRequirement::new(2, 0, 0, 1),
        rewards: OrderRewards { victory_points: 8, imported_goods: 1, ..Default::default() },
    };
    assert_eq!(order_prompt_line(&card),
        "O17 [Britain]  needs 2 copper + 1 fish  ->  8 VP + 1 import");
}

#[test]
fn order_compact_line_uses_short_words() {
    // same card
    assert_eq!(order_compact_line(&card), "O17 GB  2cop 1fish -> 8vp 1imp");
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p yokohama-core order_prompt_line` → FAIL.

- [ ] **Step 3: Implement.** Country display names: English→"Britain"/"GB", American→"America"/"US", French→"France"/"FR", German→"Germany"/"DE", Dutch→"Netherlands"/"NL". Resource words: copper/silk/tea/fish (prompt), `cop/silk/tea/fish` (compact). Reward words: `N VP` / `N yen` / `N import` / `N move` / `N any good` / `N warehouse` (prompt); `vp/yen/imp/move/any/wh` (compact). Skip zero-valued parts; join requirements with ` + `, rewards with ` + ` (prompt) or space (compact). Id renders as `O{:02}` from `card.id.0`.

- [ ] **Step 4: Green + commit**

```bash
git add crates/yokohama-core/src/game/display.rs crates/yokohama-core/src/game/mod.rs
git commit -m "feat: two-layer card display language (prompt + compact)"
```

---

### Task 9: Decks into `GameState` + real `new_game` setup (spec C1, C3)

**Files:**
- Modify: `crates/yokohama-core/src/types/state.rs` (fields), `crates/yokohama-core/src/types/player.rs` (one flag), `crates/yokohama-core/src/game/setup.rs` (new_game/finalize), `crates/yokohama-core/Cargo.toml` (rand_chacha → `[dependencies]`), `game/end_game.rs:28` (deck-empty check), `actions/cards.rs:144,316` (drop counter writes), all `GameState` literal fixtures

**Interfaces:**
- Consumes: `Rules::standard()` (Task 7), `generate_board_layout(&GameConfig, &mut impl Rng, Vec<BuildingSite>, Vec<FivePowerEffect>) -> Result<BoardLayout, BoardGenerationError>` (existing).
- Produces (used by Tasks 10-19):
  - `GameState.order_deck: Vec<OrderCardId>`, `GameState.tech_deck: Vec<TechCardId>` (last element = top), `GameState.orders_removed: Vec<OrderCardId>`; fields `order_deck_remaining`/`tech_deck_remaining` DELETED.
  - `PlayerState.starting_order_hidden: bool` (live-tracked opponent kept an unknown card).
  - `GameConfig.setup_seed: u64` (0 in legacy constructors; `new_game` sets it).
  - `pub enum StartingOrderChoice { Keep(u8), Unknown }`
  - `pub struct PendingSetup { pub state: GameState, pub deals: Vec<[OrderCardId; 2]> }`
  - `pub fn new_game(config: GameConfig, rules: &Rules, seed: u64) -> Result<PendingSetup, &'static str>`
  - `pub fn finalize_setup(pending: PendingSetup, choices: &[StartingOrderChoice]) -> Result<GameState, &'static str>`

- [ ] **Step 1: Failing tests** (in `setup.rs`)

```rust
#[test]
fn new_game_is_seed_deterministic_and_decks_are_correct() {
    let rules = Rules::standard();
    let a = new_game(GameConfig::for_player_count(4), &rules, 42).unwrap();
    let b = new_game(GameConfig::for_player_count(4), &rules, 42).unwrap();
    assert_eq!(a.state, b.state);
    assert_eq!(a.deals, b.deals);
    // 4p: 36 orders - 0 removed - 8 dealt (2x4) - 6 port fill (A+B) = 22 in deck
    assert_eq!(a.state.order_deck.len(), 22);
    // 20 techs - 8 lab fill (A+B, 4 each) = 12
    assert_eq!(a.state.tech_deck.len(), 12);
    assert!(a.state.port_state.port_a.low.is_some());
    assert!(a.state.lab_state.lab_b.as_ref().unwrap().slots.iter().all(|s| s.is_some()));
}

#[test]
fn finalize_setup_applies_keep_and_unknown_choices() {
    let rules = Rules::standard();
    let pending = new_game(GameConfig::for_player_count(2), &rules, 7).unwrap();
    let deals = pending.deals.clone();
    let state = finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown]).unwrap();
    // P0 kept deal[0]: it's in hand and recorded as starting order; deal[1] removed from play
    assert_eq!(state.players[0].order_cards_in_hand, vec![deals[0][0]]);
    assert_eq!(state.players[0].starting_order, Some(deals[0][0]));
    assert!(state.orders_removed.contains(&deals[0][1]));
    // P1 unknown: nothing in hand, flag set, both dealt cards out of the deck fiction
    assert!(state.players[1].order_cards_in_hand.is_empty());
    assert!(state.players[1].starting_order_hidden);
    assert!(state.orders_removed.contains(&deals[1][0]) && state.orders_removed.contains(&deals[1][1]));
}
```

Note: player-count removal counts (2P removes 10, per `GameConfig::order_cards_to_remove`) may change after Task 6's rulebook check — write deck-size assertions against `config.order_cards_to_remove()` rather than hardcoding where possible.

- [ ] **Step 2: Run to verify failure** — `cargo test -p yokohama-core new_game_is_seed` → FAIL.

- [ ] **Step 3: Implement.** In `types/state.rs`: replace the two `u8` counters with the three `Vec` fields; add `setup_seed: u64` to `GameConfig` (legacy constructors set 0). In `player.rs`: add `starting_order_hidden: bool` (default false). In `setup.rs`:

```rust
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn new_game(mut config: GameConfig, rules: &Rules, seed: u64) -> Result<PendingSetup, &'static str> {
    config.setup_seed = seed;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let board_layout = generate_board_layout(&config, &mut rng,
        rules.building_sites.clone(), rules.five_power_tokens.clone())
        .map_err(|_| "board generation failed")?;

    // Order deck: all 36, shuffle, remove by player count (to orders_removed), deal 2 per player.
    let mut order_deck: Vec<OrderCardId> = rules.orders.iter().map(|c| c.id).collect();
    order_deck.sort_by_key(|id| id.0);          // determinism before shuffle
    order_deck.shuffle(&mut rng);
    let mut orders_removed: Vec<OrderCardId> =
        (0..config.order_cards_to_remove()).filter_map(|_| order_deck.pop()).collect();
    let deals: Vec<[OrderCardId; 2]> = (0..config.player_count)
        .map(|_| [order_deck.pop().unwrap(), order_deck.pop().unwrap()]).collect();

    let mut tech_deck: Vec<TechCardId> = rules.techs.iter().map(|c| c.id).collect();
    tech_deck.sort_by_key(|id| id.0);
    tech_deck.shuffle(&mut rng);

    let mut state = /* assemble GameState as create_test_game_state does, but with
        board_layout, order_deck, tech_deck, orders_removed, and achievements drawn
        one per category with rng (see below) */;

    // Initial fills
    refill_port(&mut state.port_state.port_a, &mut state.order_deck);
    if let Some(port_b) = state.port_state.port_b.as_mut() { refill_port(port_b, &mut state.order_deck); }
    for _ in 0..4 { refill_lab(&mut state.lab_state.lab_a, &mut state.tech_deck); }
    if let Some(lab_b) = state.lab_state.lab_b.as_mut() {
        for _ in 0..4 { refill_lab(lab_b, &mut state.tech_deck); }
    }
    Ok(PendingSetup { state, deals })
}
```

(`refill_port`/`refill_lab` need a borrow-split: they take `&mut PortBoard` and `&mut Vec<…>` which now both live in `state` — split borrows via local `let (port, deck) = (&mut state.port_state.port_a, &mut state.order_deck);` which Rust allows across distinct fields.) Achievements: draw one random `AchievementType` per category A/B/C with `rng` (list the variants per category explicitly). `finalize_setup` walks choices: `Keep(i)` → push `deals[p][i]` to hand + `starting_order`, other card to `orders_removed`; `Unknown` → both to `orders_removed`, set `starting_order_hidden`.

- [ ] **Step 4: Sweep compile errors.** Every `GameState` literal fixture (grep `order_deck_remaining`) replaces the two counters with `order_deck: vec![], tech_deck: vec![], orders_removed: vec![]`; `end_game.rs:28` becomes `state.order_deck.is_empty()`; `actions/cards.rs` drops the two counter-write lines; end_game tests set `state.order_deck = vec![]` / `vec![OrderCardId(1)]` instead of counters.

- [ ] **Step 5: Full suite green** — `cargo test -p yokohama-core`.

- [ ] **Step 6: Commit**

```bash
git add -A crates/yokohama-core
git commit -m "feat: decks live in GameState; seeded new_game/finalize_setup with starting-order deals"
```

---

### Task 10: Reveal-reconciliation helpers (spec C2)

**Files:**
- Modify: `crates/yokohama-core/src/types/state.rs` (impl block)

**Interfaces:**
- Produces (Phase 2 CLI + Task 19 use these): `GameState::promote_order_to_top(&mut self, OrderCardId) -> Result<(), MoveError>`, `promote_tech_to_top(&mut self, TechCardId) -> Result<(), MoveError>`, `extract_order_from_deck(&mut self, OrderCardId) -> Result<(), MoveError>` (removes anywhere in deck → for revealing a live opponent's hidden starting order).

- [ ] **Step 1: Failing test**

```rust
#[test]
fn promote_order_to_top_swaps_within_deck() {
    let mut state = create_test_game_state(2);
    state.order_deck = vec![OrderCardId(1), OrderCardId(2), OrderCardId(3)]; // 3 = top
    state.promote_order_to_top(OrderCardId(1)).unwrap();
    assert_eq!(state.order_deck.last(), Some(&OrderCardId(1)));
    assert_eq!(state.order_deck.len(), 3);
    assert!(state.promote_order_to_top(OrderCardId(9)).is_err());
}
```

- [ ] **Step 2: FAIL run** — `cargo test -p yokohama-core promote_order_to_top` → method not found.

- [ ] **Step 3: Implement** (position-find + `swap`; `extract_order_from_deck` = find + `remove`, err `MoveError::CardNotFound` when absent).

- [ ] **Step 4: Green + commit** — `git commit -m "feat: deck reveal-reconciliation helpers (promote_to_top, extract)"`.

---

### Task 11: Wire the Port action (spec D1)

**Files:**
- Modify: `crates/yokohama-core/src/game/apply_move.rs:316-318` (Port arm), `crates/yokohama-core/src/game/actions/cards.rs` (`execute_port_action` signature: drop the `order_deck: &mut Vec<OrderCardId>` param — it now reads `state.order_deck`; drop `port_is_b` in favor of resolving from the president's area type)

**Interfaces:**
- Consumes: `apply_area_action` gains `rules: &Rules` (thread from `apply_move`); `state.order_deck` (Task 9).
- Produces: `Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, second_card_payment } }` fully functional.

- [ ] **Step 1: Failing test** (in `apply_move.rs` tests; fixture: president on a PortA area, power ≥3, deck seeded with known ids, port slots filled)

```rust
#[test]
fn port_action_takes_card_refills_and_pays_second() {
    let rules = Rules::standard();
    let mut state = /* fixture with president OnBoard(port_area), current_action_power: Some(4),
        port_a low/mid/high = O(1)/O(2)/O(3), order_deck = [O(10)], player money 5,
        turn_phase: AreaAction */;
    let mv = Move::ExecuteAreaAction { action: AreaAction::Port {
        cards_taken: vec![PortSlot::Low, PortSlot::Mid],
        second_card_payment: Some(PortPayment::TwoYen),
    }};
    let next = apply_move(&state, &mv, &rules).unwrap();
    let p = &next.players[0];
    assert_eq!(p.order_cards_in_hand, vec![OrderCardId(1), OrderCardId(2)]);
    assert_eq!(p.money, 3);                          // paid 2 yen for second card
    assert_eq!(next.port_state.port_a.low, Some(OrderCardId(3)));  // slid left
    assert_eq!(next.port_state.port_a.mid, Some(OrderCardId(10))); // refilled from deck
    assert!(next.order_deck.is_empty());
}
```

- [ ] **Step 2: FAIL run** — hand stays empty today (stub).

- [ ] **Step 3: Wire it.** `apply_area_action(state, action, rules)`; Port arm calls `execute_port_action(state, player, cards_taken, *second_card_payment, power, &rules.orders)` with the reworked signature (internally: resolve port A/B from the acting area's `AreaType`, split-borrow `port`/`order_deck` fields, keep all existing validation). Map `&'static str` errors with `.map_err(MoveError::RuleViolation)`.

- [ ] **Step 4: Green + regression** — `cargo test -p yokohama-core` (existing `execute_port_action` unit tests updated for the new signature).

- [ ] **Step 5: Commit** — `git commit -m "feat: wire Port action into apply_move with state-owned deck"`.

---

### Task 12: Wire the Laboratory action (spec D2)

**Files:**
- Modify: `apply_move.rs:319-321` (Lab arm), `actions/cards.rs` (`execute_lab_action`: drop `tech_deck` param, resolve `lab_is_b` + `slot_index` internally), `game/cards.rs` (`check_mercenary_eligibility`: make `pub(crate)` if not already reachable)

**Interfaces:**
- Consumes: `AreaAction::Laboratory { card_id, extra_yen, extra_imported, fifth_tech_payment }` — note the Move carries **card_id**, not slot: resolve `(lab_is_b, slot_index)` by searching `lab_a.slots` then `lab_b` for `card_id`; error `MoveError::CardNotFound` if absent.
- Produces: functional Lab acquisition incl. PatentSystem (+2 VP) / LetterPressPrinting (+1 Yen) acquisition bonuses and mercenary-eligibility recheck (techs carry country icons).

- [ ] **Step 1: Failing test** (fixture: lab_a slot 0 = tech with industry_cost 3, power 3, `turn_phase: AreaAction`)

```rust
#[test]
fn lab_action_acquires_tech_and_refills() {
    let rules = Rules::standard();
    let some_tech = rules.techs.iter().next().unwrap().id;
    let mut state = /* fixture; lab_a.slots[0] = Some(some_tech); tech_deck = [T(x)];
        current_action_power = Some(5) */;
    let mv = Move::ExecuteAreaAction { action: AreaAction::Laboratory {
        card_id: some_tech, extra_yen: 0, extra_imported: 0, fifth_tech_payment: None }};
    let next = apply_move(&state, &mv, &rules).unwrap();
    assert!(next.players[0].technologies.contains(&some_tech));
    assert!(next.lab_state.lab_a.slots[3].is_some()); // refilled
}
```

- [ ] **Step 2: FAIL run.** — stub does nothing.

- [ ] **Step 3: Wire + add acquisition bonuses.** After the existing execute body succeeds: check acquiring player's OTHER techs for `PatentSystem` (+2 VP) and `LetterPressPrinting` (+1 Yen) via `rules.techs`; then run the same country-icon mercenary check used by order fulfillment (`check_mercenary_eligibility(state, player, card_country, &rules.orders, &rules.techs)`).

- [ ] **Step 4: Green + commit** — `git commit -m "feat: wire Laboratory action with acquisition bonuses and mercenary recheck"`.

---

### Task 13: Church placement (spec D3)

**Files:**
- Modify: `actions/tracks.rs` (add executor), `apply_move.rs:327-331` (Church arm)

**Interfaces:**
- Produces: `pub fn execute_church_action(state: &mut GameState, player: PlayerId, power: u8, copper: bool, silk: bool, tea: bool, seafood: bool, yen: bool, imported_good: bool, rules: &Rules) -> Result<(), &'static str>` — pays goods, spends assistant from hand → `assistants.on_church`, writes `church_track.slots`, pushes `church_positions`, awards VP, Winery +3 VP.

- [ ] **Step 1 [ORCHESTRATOR-DIRECT]: Rule check.** Read the Church section of `analysis-1/yokohama-kb/01-rules-reference.md`. Confirm slot-selection rule when the exact faith slot is occupied (expected per rulebook: place on the **highest empty slot with value ≤ earned faith**; faith below 4 or no eligible slot ⇒ Church action yields no placement/VP — confirm exact wording) and confirm whether contributed Yen/goods go to supply. Record the ruling in `guidingpath.md`'s decision log.

- [ ] **Step 2: Failing test**

```rust
#[test]
fn church_places_assistant_and_awards_vp() {
    let rules = Rules::empty_for_tests();
    let mut state = /* fixture: president on Church area, power 3 (faith 3) + copper+silk (=5),
        player has 1 copper 1 silk, 1 assistant in hand, turn_phase: AreaAction */;
    let mv = Move::ExecuteAreaAction { action: AreaAction::Church {
        copper: true, silk: true, tea: false, seafood: false, yen: false, imported_good: false }};
    let next = apply_move(&state, &mv, &rules).unwrap();
    let p = &next.players[0];
    assert_eq!(p.assistants.in_hand, 0);
    assert_eq!(p.assistants.on_church, 1);
    assert_eq!(p.copper, 0); assert_eq!(p.silk, 0);
    assert_eq!(next.church_track.slots[ChurchTrack::faith_to_index(5).unwrap()].unwrap().player, p.id);
    assert_eq!(p.church_positions, vec![5]);
    assert_eq!(p.victory_points, ChurchTrack::vp_reward(5) as u32);
}
```

Plus: occupied-slot fallback test (faith 5 slot taken → placed at 4, VP for 4) and Winery bonus test (+3 VP).

- [ ] **Step 3: FAIL → implement executor** (use `calculate_church_faith`, `faith_to_index`, `next_arrival_order` incrementing; pay each contributed good; wire arm in `apply_move` replacing the TODO).

- [ ] **Step 4: Green + commit** — `git commit -m "feat: Church action places assistant on track with faith VP and Winery bonus"`.

---

### Task 14: Customs placement (spec D4)

**Files:**
- Modify: `actions/tracks.rs` (executor + fix `goods_to_index`), `apply_move.rs:332-335` (Customs arm), `types/tracks.rs` (if index mapping is wrong)

**Interfaces:**
- Produces: `pub fn execute_customs_action(state: &mut GameState, player: PlayerId, power: u8, imported_goods_paid: u8, rules: &Rules) -> Result<(), &'static str>` — mirrors Church: pay imports, assistant hand→`on_customs`, track slot write, `customs_positions` push, VP via `CustomsTrack::vp_reward`, Brickyard +4 VP.

- [ ] **Step 1 [ORCHESTRATOR-DIRECT]: Rule + bug check.** Verify against the KB: the customs board's spaces (1..7+ — is it 7 spaces or 8?), the VP table (current code: 1→5, 2→4, 3→10, 4→9, 5→16, 6→15, 7+→22 — the zig-zag needs confirming), and the occupied-slot rule. **Known suspect:** `CustomsTrack::goods_to_index` maps `>7` to `Some(6)` while the array has 8 slots and the comment says index 7 = "7+" — resolve mapping vs. reality, fix + unit-test whichever is right. Record rulings in the decision log.

- [ ] **Step 2: Failing test** (mirror of Church: pay 2 imports at power 3 → slot for 2, VP awarded, assistant moved; plus occupied-slot fallback + Brickyard test).

- [ ] **Step 3: FAIL → implement → green.**

- [ ] **Step 4: Commit** — `git commit -m "feat: Customs action places assistant on track; fix goods_to_index mapping"`.

---

### Task 15: Placement fees + production tech lookup (spec D5, D6)

**Files:**
- Modify: `apply_move.rs:195-226` (`apply_placement`, `apply_placement_same_area`), `legal_moves.rs:78` (make `calculate_placement_cost` `pub(crate)`), `actions/production.rs:52-70` (`apply_production` region with the TODO)

**Interfaces:**
- Consumes: `calculate_placement_cost(state, player, &[AreaId]) -> u32` (existing), `rules.techs` for `MiningTechnology` (+1 copper on copper receipt).

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn placement_charges_president_fee() {
    // Opponent president on area 1; placing 1 assistant there costs 1 yen.
    let rules = Rules::empty_for_tests();
    let mut state = /* fixture: turn_phase Placement, P1 president OnBoard(area1), P0 money 1 */;
    let next = apply_move(&state, &Move::PlaceThreeAreas { placements: vec![area1] }, &rules).unwrap();
    assert_eq!(next.players[0].money, 0);
}

#[test]
fn placement_fails_without_fee_money() { /* same but money 0 → Err(MoveError::NotEnoughMoney) */ }

#[test]
fn mining_technology_adds_copper() { /* player owns MiningTechnology tech; ProduceCopper at power 2 → reward+1 */ }
```

- [ ] **Step 2: FAIL → implement.** Both placement fns compute cost via `calculate_placement_cost` (for `PlaceTwoSameArea` + Telegram bonus area include the bonus area's own cost, matching what `generate_placement_moves` already prices), check `p.money`, deduct. Production: thread `rules` into `apply_production` (or check in the `apply_area_action` Produce arms — smallest diff wins) and add +1 copper when the player owns a tech with `TechEffect::MiningTechnology`.

- [ ] **Step 3: Green + commit** — `git commit -m "feat: charge placement president fees; mining technology production bonus"`.

---

### Task 16: Real enumeration — Port & Laboratory (spec E, part 1)

**Files:**
- Modify: `legal_moves.rs:274-338` (replace `generate_area_action_for_type` dispatch for Port/Lab with real generators; other areas keep placeholders until Task 17)

**Interfaces:**
- Consumes: `port_accessible_slots(power) -> Vec<PortSlot>`, `validate_port_action`, `validate_lab_action`, `lab_position_surcharge`, `calculate_industry_value` (all existing in `actions/cards.rs`), `rules`.
- Produces: `generate_port_actions(state, player, power, rules) -> Vec<Move>`, `generate_lab_actions(state, player, power, rules) -> Vec<Move>` (private to `legal_moves.rs`).

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn port_enumeration_covers_slots_pairs_and_payments() {
    // power 4, all three slots filled, money 2, imports 1, hand empty:
    // singles: Low, Mid, High = 3 moves
    // pairs: {L,M},{L,H},{M,H} x {TwoYen, OneImportedGood} = 6 moves
    let moves = /* get_legal_moves in AreaAction phase on port fixture */;
    let port_moves: Vec<_> = moves.iter().filter(|m| matches!(m,
        Move::ExecuteAreaAction { action: AreaAction::Port { .. } })).collect();
    assert_eq!(port_moves.len(), 9);
}

#[test]
fn port_enumeration_respects_hand_limit_and_power() {
    // power 2 → only Low accessible; hand already holds 2 orders → pairs would exceed limit 3?
    // 2 in hand + 2 taken = 4 > 3, so ONLY singles appear.
}

#[test]
fn lab_enumeration_offers_minimal_payments_only() {
    // power 3, slot0 cost 3 → one move with extra_yen 0/extra_imported 0.
    // slot2 cost 3 (+1 surcharge=4) → moves paying exactly 1 IV extra: {extra_yen:2} and {extra_imported:1} — no overpays.
}
```

- [ ] **Step 2: FAIL → implement.** Port: for each accessible filled slot → single-take move; for each accessible pair (if hand+2 ≤ 3) → one move per affordable payment variant. Lab: for each filled slot across lab A (and B when acting on LaboratoryB — mirror how the executor resolves the acting area), compute `deficit = (cost + surcharge).saturating_sub(power.min(5))`; emit payment mixes covering the deficit exactly using `extra_imported = k` (k ≤ imports) and `extra_yen = 2*(deficit-k)` for `k in 0..=deficit`, filtering by affordability; add `fifth_tech_payment` variants (`Some(true)` if money covers, `Some(false)` if an import spare) when `technologies.len() >= 4`. Every emitted move must pass its own validator — add a debug assertion:

```rust
debug_assert!(moves.iter().all(|m| apply_move(state, m, rules).is_ok()),
    "generated illegal area action");
```

(Keep the assertion in a `#[cfg(test)]` helper if apply-in-generate is too hot; the fuzz suite enforces the same property globally.)

- [ ] **Step 3: Green + commit** — `git commit -m "feat: real legal-move enumeration for Port and Laboratory"`.

---

### Task 17: Real enumeration — Church, Customs, Employment, Chinatown (spec E, part 2)

**Files:**
- Modify: `legal_moves.rs` (same dispatch; delete `generate_area_action_for_type` once all arms are real)

**Interfaces:**
- Consumes: validators from `actions/tracks.rs` + `actions/commercial.rs` (`employment_agency_tokens_allowed`, `validate_chinatown_exchanges`).
- Produces: full `generate_area_action_moves` coverage; Chinatown enumeration bounded at ≤ 4 exchanges (documented cap).

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn church_enumeration_is_goods_subsets() {
    // player holds 1 copper only, 1 assistant, power 3:
    // subsets: {} and {copper} → 2 moves. With no assistant in hand → 0 church moves
    // (but a non-church "do nothing" path must still exist — confirm rulebook: acting is optional?
    //  If the rulebook says the area action is optional, ensure a Skip/empty move exists for EVERY area;
    //  record ruling in decision log and encode once, uniformly.)
}

#[test]
fn customs_enumeration_ranges_over_payable() {
    // power 4 (max 3), imports 2, assistant available → pays 1 or 2 → 2 moves
}

#[test]
fn employment_enumeration_bounded_compositions() {
    // power 3 → allowed items N (read employment_agency_tokens_allowed(3) for exact N);
    // warehouse: 2 assistants, 1 shophouse, 0 TH; money 2 (shophouse costs 2 each from agency? —
    // consult existing apply_employment_agency cost logic and mirror it) → assert exact move set.
}

#[test]
fn chinatown_enumeration_is_bounded_and_legal() {
    // multisets of exchanges, size 0..=4, each passing validate_chinatown_exchanges
    // player: 1 copper, 2 yen → e.g. [CopperToYen], [YenToTea], [CopperToYen, YenToSilk], …
    // assert every generated move applies Ok, and the cap is respected.
}
```

- [ ] **Step 2: FAIL → implement.** Church: iterate the ≤64 bool-combos, filter by stock (`validate_church_action`), require assistant in hand. Customs: `1..=min(customs_max_imported_goods(power), imports)`. Employment: triple loop `a+s+t ≤ allowed`, filter warehouse stock + cost. Chinatown: BFS over exchange sequences to depth 4 on a copied resource tuple, canonicalize as sorted multiset to dedupe, emit `AreaAction::Chinatown { exchanges }`; log the cap once per generation at `debug` level — since the crate has no logger, expose the cap as `pub const CHINATOWN_ENUM_DEPTH: usize = 4;` with a doc comment referencing the spec instead of runtime logging.
**[ORCHESTRATOR-DIRECT]** Resolve the "is the area action optional?" rule question from Step 1 uniformly (single `Skip`-style move or empty-subset conventions) BEFORE dispatching this task, pass the ruling in the dispatch prompt, and record it in `guidingpath.md`.

- [ ] **Step 3: Green; then delete dead placeholder code + rerun.**

- [ ] **Step 4: Commit** — `git commit -m "feat: real enumeration for Church/Customs/Employment/Chinatown; retire placeholders"`.

---

### ⛔ CHECKPOINT (before Part F): content manifest sign-off

Do not start Task 18 until `docs/plans/2026-07-16-card-verification/content-manifest.md` shows every component `VALIDATED` against the rulebook/box (Task 6). If anything is `UNRESOLVED`, stop and check in with Jeremy. If all validated — proceed without asking.

---

### Task 18: Invariants + random self-play fuzz gate (spec F1, F2)

**Files:**
- Create: `crates/yokohama-core/src/game/invariants.rs` (+ `mod invariants; pub use invariants::*;`), `crates/yokohama-core/examples/selfplay.rs`, `crates/yokohama-core/tests/fuzz_full_games.rs`

**Interfaces:**
- Consumes: everything above; `Rules::standard()`, `new_game`, `finalize_setup`, `get_legal_moves`, `apply_move`, `is_game_over`, `calculate_final_scores` (check `scoring.rs` for the exact final-scoring fn name and use that).
- Produces: `pub fn check_invariants(state: &GameState, rules: &Rules) -> Result<(), String>`; `run_random_game(seed, players, rules) -> Result<GameStats, String>` (in the test/example, shared via `tests/` helper or example module).

- [ ] **Step 1: Write `check_invariants` with failing unit test** (deliberately corrupt state → Err):

```rust
pub fn check_invariants(state: &GameState, rules: &Rules) -> Result<(), String> {
    for p in &state.players {
        let total = p.assistants.total();
        if total != 20 { return Err(format!("P{} assistants {} != 20", p.id.0, total)); }
        // money/goods are u32 (non-negative by type); check hand limit:
        if p.order_cards_in_hand.len() > 3 { return Err(format!("P{} hand > 3", p.id.0)); }
    }
    // Order conservation: deck + ports + hands + completed + removed (+1 per unrevealed hidden start) = 36
    let in_ports = [state.port_state.port_a.low, state.port_state.port_a.mid, state.port_state.port_a.high]
        .iter().flatten().count()
        + state.port_state.port_b.as_ref().map_or(0, |b| [b.low, b.mid, b.high].iter().flatten().count());
    let in_hands: usize = state.players.iter().map(|p| p.order_cards_in_hand.len()).sum();
    let completed: usize = state.players.iter().map(|p| p.completed_orders.len()).sum();
    let hidden: usize = state.players.iter().filter(|p| p.starting_order_hidden).count();
    let total_orders = state.order_deck.len() + in_ports + in_hands + completed
        + state.orders_removed.len() + hidden;
    if rules.orders.iter().count() == 36 && total_orders != 36 {
        return Err(format!("order conservation broken: {}", total_orders));
    }
    // Tech conservation: deck + labs + owned = 20 (when standard rules)
    // Shophouse: per player, in_hand + in_warehouse + built.len() = 8; trading houses: warehouse + built = 4.
    // ≤1 shophouse per player per area is implied by the HashMap<AreaId, u8> key — instead check
    //   no two players share (area, slot): collect (area, slot) pairs across players, assert unique.
    // current_action_power ≤ 5 when Some.
    Ok(())
}
```

(Complete the commented checks as real code; each gets its own corrupt-state unit test.)

- [ ] **Step 2: Write the random driver** (`examples/selfplay.rs`): args `<games> <players> <base_seed>` via `std::env::args`; per game: `new_game` → `finalize_setup` with all `Keep(rng % 2)` → loop `get_legal_moves` / pick uniformly with `ChaCha8Rng` / `apply_move` / `check_invariants` until `is_game_over(&state)` or `moves.is_empty()` (empty legal moves in a non-terminal state is a FAILURE — print state JSON + move log and exit nonzero) or turn bound (`state.turn_number > 500` ⇒ failure: no termination). Print: games completed, mean turns, per-phase mean branching factor, end-trigger histogram, moves/sec.

- [ ] **Step 3: Write the gate test** (`tests/fuzz_full_games.rs`):

```rust
#[test]
fn fuzz_500_games_per_player_count() {
    // CI-friendly slice of the full gate; run the full 5000 via the example binary.
    let rules = Rules::standard();
    let mut triggers_seen = std::collections::HashSet::new();
    for players in 2..=4u8 {
        for seed in 0..500u64 {
            let stats = run_random_game(seed, players, &rules)
                .unwrap_or_else(|e| panic!("seed {seed} {players}p: {e}"));
            if let Some(t) = stats.trigger { triggers_seen.insert(std::mem::discriminant(&t)); }
        }
    }
    assert_eq!(triggers_seen.len(), 5, "all five end triggers must occur across the corpus");
}
```

- [ ] **Step 4 [ORCHESTRATOR-DIRECT]: Run it and fix what it finds.** This step is intentionally iterative: expect phase-machine dead-ends, off-by-ones, and rules gaps to surface. For each failure: minimal reproducing seed → failing focused test → fix → rerun fuzz. One commit per distinct fix (`fix: <what> (found by fuzz seed N)`). Rules ambiguities go to the KB + decision log. Do not weaken invariants to pass.

- [ ] **Step 5: Full gate run** — `cargo run --release -p yokohama-core --example selfplay -- 5000 4 0` (and 2p/3p): zero failures.

- [ ] **Step 6: Commit** — `git commit -m "test: full-game random self-play fuzz gate with state invariants"`.

---

### Task 19: `GameRecord` + replay determinism (spec F3)

**Files:**
- Create: `crates/yokohama-core/src/game/record.rs` (+ mod/pub use)
- Modify: `crates/yokohama-core/Cargo.toml` (add `serde_json` to `[dev-dependencies]`)

**Interfaces:**
- Produces (Phase 2's save format): `pub struct GameRecord { pub engine_version: String, pub seed: u64, pub config: GameConfig, pub choices: Vec<StartingOrderChoice>, pub moves: Vec<Move> }` (all serde); `pub fn replay(record: &GameRecord, rules: &Rules) -> Result<GameState, MoveError>`; `GameRecord::new(seed, config, choices)` + `push_move(&mut self, mv: Move)`.

- [ ] **Step 1: Failing test**

```rust
#[test]
fn record_replays_to_identical_state() {
    let rules = Rules::standard();
    // Drive a short random game (reuse run_random_game but capturing moves into a GameRecord).
    let (final_state, record) = run_recorded_game(7, 2, &rules);
    let replayed = replay(&record, &rules).unwrap();
    assert_eq!(replayed, final_state);
    // JSON round-trip preserves equality:
    let json = serde_json::to_string(&record).unwrap();
    let back: GameRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(replay(&back, &rules).unwrap(), final_state);
}
```

- [ ] **Step 2: FAIL → implement** (`replay` = `new_game(config, rules, seed)` → `finalize_setup(choices)` → fold `apply_move`; version field = `env!("CARGO_PKG_VERSION").to_string()`, checked with a warning-comment — replay does not refuse on mismatch in v1).

- [ ] **Step 3: Green + commit** — `git commit -m "feat: GameRecord save/replay format with determinism test"`.

---

### Task 20: Perf baseline + close out (spec F4)

**Files:**
- Modify: `guidingpath.md` (Current state + roadmap + perf note), `README.md` (status table row for engine), `agent-handover.md` (optional final pointer)

- [ ] **Step 1: Measure** — `cargo run --release -p yokohama-core --example selfplay -- 1000 4 0`; record games/sec and moves/sec.

- [ ] **Step 2: Update `guidingpath.md`:** flip the "Not done" table to done (with commit refs), record the perf baseline and per-phase branching factors (informs Phase 3 MCTS budgets), mark Phase 0+1 gate PASSED with the fuzz command + date, and set Phase 2 (CLI driver) as next.

- [ ] **Step 3: Verify honestly** — rerun `cargo test -p yokohama-core` one final time and paste the summary into the guidingpath gate note.

- [ ] **Step 4: Commit** — `git commit -m "docs: engine completion gate passed; record perf baseline; next: Phase 2 CLI"`.

---

## Self-review notes (kept for the executor)

- Spec coverage: A1→T1, A2→T2, A3→T3, B1→T4+T5, B2→T6, B3→T7, B4→T8, C1+C3→T9, C2→T10, D1→T11, D2→T12, D3→T13, D4→T14, D5+D6→T15, E→T16+T17, checkpoint→pre-T18, F1+F2→T18, F3→T19, F4→T20.
- Deliberately deferred (per spec non-goals): CLI, AI, CV, mid-game state entry, micro-decision move splitting.
- Fixture snippets marked `/* fixture … */` mean: build the state with `setup::create_test_game_state(n)` then set the listed fields — the pattern used throughout the existing test suite (see `apply_move.rs`'s test module). This is a construction instruction, not a placeholder.
- Two open rules questions are resolved INSIDE tasks (T13 church slot rule, T14 customs table, T17 optionality) with explicit rule-check steps against the KB — that is by design, not an omission; each ruling lands in `guidingpath.md`.
