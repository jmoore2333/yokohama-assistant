# Choice Encoding Implementation Plan (2026-07-18)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/plans/2026-07-18-choice-encoding-design.md` — replace every v1 auto-resolution with real engine choices (pending-choice queue), harden the wizard, make the Canal crossable, and land the test & effectiveness module — completing everything the Phase-2 live gate is blocked on.

**Architecture:** A `pending_choices: Vec<PendingChoice>` queue on `GameState`; `get_legal_moves` surfaces only the head choice's resolution moves while non-empty; executors push choices instead of auto-resolving; the end-of-`apply_move` automation defers until the queue drains. Choice resolutions are ordinary `Move`s → ordinary `RecordedEvent::Move`s (no record-schema change). CLI renders new variants automatically via `describe_move` (wildcard-free, compiler-enforced).

**Tech Stack:** Rust workspace (`yokohama-core`, `yokohama-cli`), serde, seeded `rand_chacha` (existing). No new dependencies.

## Global Constraints

- **TDD strictly.** Every task: failing test first; implementer reports MUST paste the literal RED output before implementing (a Phase-2 task was caught skipping this).
- **Suites green + zero workspace warnings after every task**: `cargo test -p yokohama-core && cargo test -p yokohama-cli` (baseline at branch start: 390 + 39 passing).
- **`describe_move` and `describe_area_action` stay wildcard-free.** Every task that adds a `Move` variant modifies `crates/yokohama-cli/src/describe.rs` in the same task (the workspace will not compile otherwise — this is by design).
- **`GameState` stays history-free and cheap to clone.** The queue is current-decision state; never store history or counters in it. All new state fields derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize` and use `#[serde(default)]` for save compatibility.
- **Bound reveals push in the same `Session::apply` commit as their move** (choice moves never draw cards, so this holds by construction — do not change it).
- **Conservative projection**: never offer a move the executor can't express; generators and executors share pure helpers so legal == applicable structurally (house pattern).
- **Never touch** `.codex/`, `.vscode/`, `AGENTS.md`, `tts-assets/`. `git add` specific paths only, never `-A`. Commit style `feat:/fix:/test:/docs:`.
- **Branch**: `choice-encoding` (already created off master @ f93149a; design doc committed).
- **Model tiers**: sonnet implementers, sonnet per-task reviewers, fable only for the final whole-branch review. Rules questions → orchestrator adjudicates against `analysis-1/yokohama-kb/` + `analysis-1/00-yokohama-rulebook.pdf`, writes a guidingpath decision-log row; contested facts → Jeremy.
- **Fuzz gate before merge**: `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored`; full 5000×3 sweep with NEW baselines at close (Task 18) — distributions shift by design.
- Design references below (§x.y) are sections of `2026-07-18-choice-encoding-design.md`.

## File Structure

New files: `crates/yokohama-core/src/types/choices.rs` (PendingChoice + supporting enums), `crates/yokohama-core/src/game/choices.rs` (push/resolve/option helpers), `crates/yokohama-core/tests/choice_encoding.rs` (Task 15), `crates/yokohama-core/examples/record_stats.rs` (Task 16), `crates/yokohama-cli/src/reveal.rs` (Task 10, extracted), `crates/yokohama-cli/src/input.rs` (Task 14, shared parse helpers).
Heavily modified: `types/state.rs`, `types/moves.rs`, `game/apply_move.rs`, `game/legal_moves.rs`, `game/actions/tracks.rs`, `game/board_gen.rs`, `game/movement.rs`, `game/setup.rs`, `types/tracks.rs`, `types/player.rs`; CLI `describe.rs`, `repl.rs`, `session.rs`, `render.rs`, `wizard.rs`; `docs/cli-guide.md`/`.html`.

---

### Task 1: Pending-choice queue + AnyGood end-to-end

**Files:**
- Create: `crates/yokohama-core/src/types/choices.rs`
- Create: `crates/yokohama-core/src/game/choices.rs`
- Modify: `crates/yokohama-core/src/types/state.rs` (GameState field), `types/moves.rs` (Move variants), `types/mod.rs` + `game/mod.rs` (exports), `game/legal_moves.rs:8-20` (queue-first gate), `game/apply_move.rs` (dispatch arms, deferral gate, `apply_fulfill_order`, `apply_space_reward`, `apply_build_shophouse`, delete `apply_any_good`)
- Modify: `crates/yokohama-cli/src/describe.rs` (one arm)
- Test: in-module `#[cfg(test)]` in `game/choices.rs` + additions to `game/apply_move.rs` tests

**Interfaces:**
- Produces: `PendingChoice` (variants this task: `AnyGood`; later tasks add the rest), `TradeGood { Copper, Silk, Tea, Seafood }`, `GameState.pending_choices: Vec<PendingChoice>`, `Move::ChooseAnyGood { good: TradeGood }`, `game::choices::{generate_choice_resolutions, resolve_choice_move, push_choice}`. Later engine tasks add variants to `PendingChoice`/`Move` and arms to these three functions.
- Consumes: existing `copper_bonus`, `apply_fulfill_order`, `apply_space_reward` sites.

- [ ] **Step 1: Write the failing tests** (append to `game/apply_move.rs` tests; use the existing `create_minimal_state`/registry fixture patterns in that module):

```rust
#[test]
fn fulfill_order_with_any_good_queues_choice_and_gates_legal_moves() {
    // Fixture: an order whose rewards have any_trade_good: 2, player holds it + goods.
    let (mut state, rules) = any_good_order_fixture(2); // helper below
    state.turn_phase = TurnPhase::AdditionalActionFirst;
    let after = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(1) }, &rules).unwrap();
    assert_eq!(after.pending_choices, vec![PendingChoice::AnyGood, PendingChoice::AnyGood]);
    let legal = get_legal_moves(&after, &rules);
    assert_eq!(legal.len(), 4); // exactly the four ChooseAnyGood moves, nothing else
    assert!(legal.iter().all(|m| matches!(m, Move::ChooseAnyGood { .. })));
    // No copper was auto-granted:
    assert_eq!(after.current_player().copper, state.current_player().copper - 0);
}

#[test]
fn choose_any_good_resolves_head_and_grants_chosen_good() {
    let (state, rules) = any_good_order_fixture(1);
    let mut s = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(1) }, &rules).unwrap();
    let silk_before = s.current_player().silk;
    s = apply_move(&s, &Move::ChooseAnyGood { good: TradeGood::Silk }, &rules).unwrap();
    assert_eq!(s.current_player().silk, silk_before + 1);
    assert!(s.pending_choices.is_empty());
}

#[test]
fn choice_moves_illegal_when_no_choice_pending_and_wrong_choice_rejected() {
    let (state, rules) = any_good_order_fixture(1);
    assert!(apply_move(&state, &Move::ChooseAnyGood { good: TradeGood::Tea }, &rules).is_err());
}

#[test]
fn shophouse_space_any_good_queues_and_defers_recovery() {
    // Fixture: Building phase, president on an area whose site TR space has any_good: 1,
    // player has a shophouse in hand and an assistant on the acting area.
    let (state, rules) = building_any_good_fixture();
    let mid = apply_move(&state, &Move::BuildShophouse { slot: 1 }, &rules).unwrap();
    assert_eq!(mid.pending_choices, vec![PendingChoice::AnyGood]);
    // Recovery DEFERRED: the acting-area assistant has NOT been recovered yet.
    assert!(mid.current_player().assistants.on_board.contains_key(&acting_area()));
    let done = apply_move(&mid, &Move::ChooseAnyGood { good: TradeGood::Copper }, &rules).unwrap();
    // Resolving the last choice runs the deferred Recovery + transition in the same apply.
    assert!(!done.current_player().assistants.on_board.contains_key(&acting_area()));
    assert_eq!(done.turn_phase, TurnPhase::AdditionalActionSecond);
}

#[test]
fn any_good_copper_choice_fires_mining_bonus_per_icon() {
    // Player owns 1 Mining Technology; order rewards: copper 1, any_trade_good 1.
    let (state, rules) = mining_any_good_fixture();
    let s1 = apply_move(&state, &Move::FulfillOrder { order_id: OrderCardId(1) }, &rules).unwrap();
    // Explicit copper receipt fired at the trigger: +1 copper +1 mining.
    let s2 = apply_move(&s1, &Move::ChooseAnyGood { good: TradeGood::Copper }, &rules).unwrap();
    // Any-good-as-copper is its OWN receipt: +1 copper +1 mining again (design §5.3-2).
    assert_eq!(s2.current_player().copper, state.current_player().copper + 4);
}
```

Fixture helpers (`any_good_order_fixture`, `building_any_good_fixture`, `mining_any_good_fixture`) follow the module's existing pattern: `create_minimal_state`, a hand-built `OrderRegistry`/`Rules` with the needed card, goods pre-granted. Write them concretely in this task.

- [ ] **Step 2: Run to verify RED**: `cargo test -p yokohama-core fulfill_order_with_any_good` — expected: compile error (`PendingChoice`/`ChooseAnyGood` undefined). Paste output in the report.

- [ ] **Step 3: Implement.** `types/choices.rs`:

```rust
//! Pending reward/recovery choices (design 2026-07-18 §5.1-5.2).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeGood { Copper, Silk, Tea, Seafood }

/// Unresolved choice for the current player. Length of the queue is bounded
/// by one reward bundle's icon count plus its cascades (always small).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PendingChoice {
    /// Gain 1 trade good of choice (order any_trade_good / space any_good).
    AnyGood,
}
```

`types/state.rs`: add to `GameState`:

```rust
    /// Unresolved reward/recovery choices for the current player, FIFO.
    /// Serialized state (not history). Empty at every player hand-off.
    #[serde(default)]
    pub pending_choices: Vec<PendingChoice>,
```

(and initialize `pending_choices: Vec::new()` at every construction site — compiler will list them). `types/moves.rs`: add to `Move`:

```rust
    // Choice resolutions (design 2026-07-18): legal only while the matching
    // PendingChoice heads GameState::pending_choices.
    ChooseAnyGood { good: TradeGood },
```

`game/choices.rs`:

```rust
//! Pending-choice queue mechanics (design 2026-07-18 §5.1).
use crate::types::*;
use crate::game::*;

/// Resolution moves for the HEAD pending choice. Non-empty by construction
/// (every choice type offers decline or has >=1 outcome, else it was
/// auto-resolved at push time).
pub fn generate_choice_resolutions(state: &GameState, head: &PendingChoice, _rules: &Rules) -> Vec<Move> {
    match head {
        PendingChoice::AnyGood => vec![
            Move::ChooseAnyGood { good: TradeGood::Copper },
            Move::ChooseAnyGood { good: TradeGood::Silk },
            Move::ChooseAnyGood { good: TradeGood::Tea },
            Move::ChooseAnyGood { good: TradeGood::Seafood },
        ],
    }
}

/// Push a choice, auto-resolving singletons inline (design §5.1). For
/// AnyGood the option set is always 4, so it always queues.
pub fn push_choice(state: &mut GameState, choice: PendingChoice, _rules: &Rules) {
    state.pending_choices.push(choice);
}

/// Apply a Choose* move against the head pending choice. Validates the move
/// is a resolution OF the head (type + membership in the generated set),
/// pops the head, applies the effect. Zero mutation on error.
pub fn resolve_choice_move(state: &mut GameState, mv: &Move, rules: &Rules) -> Result<(), MoveError> {
    let head = state.pending_choices.first().cloned()
        .ok_or(MoveError::RuleViolation("No pending choice to resolve"))?;
    if !generate_choice_resolutions(state, &head, rules).contains(mv) {
        return Err(MoveError::RuleViolation("Move does not resolve the pending choice"));
    }
    state.pending_choices.remove(0);
    let player = state.current_player;
    match (head, mv) {
        (PendingChoice::AnyGood, Move::ChooseAnyGood { good }) => {
            let mining = match good {
                TradeGood::Copper => copper_bonus(state, player, 1, &rules.techs),
                _ => 0,
            };
            let p = state.player_mut(player);
            match good {
                TradeGood::Copper => p.copper += 1 + mining as u32,
                TradeGood::Silk => p.silk += 1,
                TradeGood::Tea => p.tea += 1,
                TradeGood::Seafood => p.seafood += 1,
            }
        }
        _ => unreachable!("membership check above guarantees type match"),
    }
    Ok(())
}
```

`game/legal_moves.rs` — queue-first gate at the top of `get_legal_moves`:

```rust
    if let Some(head) = state.pending_choices.first() {
        return crate::game::choices::generate_choice_resolutions(state, head, rules);
    }
```

`game/apply_move.rs` — dispatch arm inside the match:

```rust
        Move::ChooseAnyGood { .. } => {
            crate::game::choices::resolve_choice_move(&mut new_state, mv, rules)?;
        }
```

— and gate the Recovery/mercenary automation block: `if new_state.turn_phase == TurnPhase::Recovery` becomes `if new_state.pending_choices.is_empty() && new_state.turn_phase == TurnPhase::Recovery`. Replace the two `apply_any_good` call sites: `apply_fulfill_order` pushes `rewards.any_trade_good` AnyGood choices (loop `for _ in 0..rewards.any_trade_good { push_choice(...) }`); `apply_space_reward` changes signature to `fn apply_space_reward(state: &mut GameState, player: PlayerId, reward: &SpaceReward, mining_bonus: u8)` and pushes likewise (update its `apply_build_shophouse` call site; the mining precompute there drops `reward.any_good` from `total_base_copper` — explicit copper only, per design §5.3-2). Delete `apply_any_good`. `describe.rs` arm:

```rust
        Move::ChooseAnyGood { good } => format!("Take 1 {} (any-good reward)", trade_good_name(*good)),
```

with a small `trade_good_name` helper (`"copper" | "silk" | "tea" | "fish"`).

- [ ] **Step 4: GREEN**: `cargo test -p yokohama-core` — all pass. `cargo test -p yokohama-cli` — all pass (describe arm compiles).
- [ ] **Step 5: Zero warnings check**: `cargo build --workspace 2>&1 | grep -c warning` → `0`.
- [ ] **Step 6: Commit**: `git add crates/yokohama-core/src crates/yokohama-cli/src/describe.rs && git commit -m "feat: pending-choice queue + AnyGood choice encoding (design §5.1-5.2)"`

**Reviewer notes:** verify the deferral gate by hand-tracing `BuildShophouse`→queue→resolve→Recovery; verify no remaining `apply_any_good` callers (`grep -rn apply_any_good crates/`); verify the fuzz smoke still passes: `cargo test -p yokohama-core --test fuzz_full_games` (non-ignored tests).

---

### Task 2: WarehouseTake + singleton auto-resolution + shared take-cost helper

**Files:**
- Modify: `types/choices.rs` (+`WarehouseItem`, +`PendingChoice::WarehouseTake`), `types/moves.rs` (+`ChooseWarehouseTake`/`DeclineWarehouseTake`), `types/player.rs:105-120` (`TradingHouseState::take_cost`), `game/choices.rs` (arms + real `push_choice`), `game/apply_move.rs` (`apply_fulfill_order`, `apply_space_reward`, `apply_employment_agency` cost loop, delete `apply_warehouse_takes`), `game/legal_moves.rs` (employment mirror uses `take_cost`), `game/actions/tracks.rs` (customs riders push instead of auto-take)
- Modify: `crates/yokohama-cli/src/describe.rs` (two arms)

**Interfaces:**
- Produces: `WarehouseItem { Assistant, Shophouse, TradingHouse }`, `TradingHouseState::take_cost(already_taken: usize) -> u32` (the 4/5/6/7 progression — single source, replaces the three inline copies), `warehouse_take_options(p: &PlayerState) -> Vec<WarehouseItem>` in `game/choices.rs`, real singleton logic in `push_choice`.
- Consumes: Task 1's queue machinery.

- [ ] **Step 1: Failing tests** (in `game/choices.rs` tests + `game/actions/tracks.rs` tests):

```rust
#[test]
fn warehouse_take_offers_affordable_items_plus_decline() {
    // warehouse: 1 assistant, 1 shophouse, 2 TH; money 4; nothing taken yet.
    let (state, rules) = warehouse_choice_fixture(/*money*/ 4);
    let legal = get_legal_moves(&state, &rules); // state pre-loaded with WarehouseTake head
    // assistant (free) + shophouse (2) + TH (first take costs 4) + decline = 4
    assert_eq!(legal.len(), 4);
}

#[test]
fn warehouse_take_th_repriced_across_consecutive_takes() {
    // Two queued takes (customs space-1 shape), money 9, TH already out: 0.
    let (state, rules) = double_warehouse_choice_fixture(9);
    let s1 = apply_move(&state, &Move::ChooseWarehouseTake { item: WarehouseItem::TradingHouse }, &rules).unwrap();
    assert_eq!(s1.current_player().money, 9 - 4);
    let s2 = apply_move(&s1, &Move::ChooseWarehouseTake { item: WarehouseItem::TradingHouse }, &rules).unwrap();
    assert_eq!(s2.current_player().money, 9 - 4 - 5); // progression advanced
    assert_eq!(s2.current_player().trading_houses.in_hand, 2);
}

#[test]
fn warehouse_take_auto_resolves_when_only_decline_possible() {
    // Empty warehouse: pushing the choice must NOT queue (singleton auto, design §5.1).
    let (mut state, rules) = warehouse_choice_base(/*warehouse all zero*/);
    push_choice(&mut state, PendingChoice::WarehouseTake, &rules);
    assert!(state.pending_choices.is_empty());
}

#[test]
fn decline_warehouse_take_changes_nothing_but_pops() {
    let (state, rules) = warehouse_choice_fixture(4);
    let s = apply_move(&state, &Move::DeclineWarehouseTake, &rules).unwrap();
    assert_eq!(s.current_player().money, 4);
    assert!(s.pending_choices.is_empty());
}

#[test]
fn customs_space_one_rider_queues_two_warehouse_takes() {
    // Customs action paying 1 import with only space 1 open -> inline place + 2 queued takes.
    let (state, rules) = customs_space1_fixture();
    let s = apply_move(&state, &Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: 1 } }, &rules).unwrap();
    assert_eq!(s.pending_choices, vec![PendingChoice::WarehouseTake, PendingChoice::WarehouseTake]);
}
```

- [ ] **Step 2: RED**: `cargo test -p yokohama-core warehouse_take` — compile error. Paste.
- [ ] **Step 3: Implement.** `TradingHouseState::take_cost` (in `types/player.rs`, replacing `next_cost`'s inline table; keep `next_cost` delegating):

```rust
    /// Printed Employment-Agency take progression, indexed by tokens already
    /// out of the warehouse (hand + built). SINGLE SOURCE (design §5.2).
    pub fn take_cost(already_taken: usize) -> u32 {
        match already_taken { 0 => 4, 1 => 5, 2 => 6, _ => 7 }
    }
```

Replace the two inline progressions (`apply_employment_agency`'s loop, `legal_moves.rs` employment mirror) with `TradingHouseState::take_cost(..)`. `warehouse_take_options`:

```rust
pub fn warehouse_take_options(p: &PlayerState) -> Vec<WarehouseItem> {
    let mut v = Vec::new();
    if p.assistants.in_warehouse >= 1 { v.push(WarehouseItem::Assistant); }
    if p.shophouses.in_warehouse >= 1 && p.money >= 2 { v.push(WarehouseItem::Shophouse); }
    let already = p.trading_houses.in_hand as usize + p.trading_houses.total_built();
    if p.trading_houses.in_warehouse >= 1 && p.money >= TradingHouseState::take_cost(already) { v.push(WarehouseItem::TradingHouse); }
    v
}
```

`push_choice` gains the real singleton rule:

```rust
pub fn push_choice(state: &mut GameState, choice: PendingChoice, rules: &Rules) {
    // Compute the resolution set AS IF this choice were head; if it has
    // exactly one element, resolve inline (design §5.1 singleton rule).
    let opts = generate_choice_resolutions_for(state, &choice, rules);
    if opts.len() == 1 {
        state.pending_choices.insert(0, choice);
        let mv = opts.into_iter().next().unwrap();
        resolve_choice_move(state, &mv, rules).expect("singleton resolution is legal by construction");
        return;
    }
    state.pending_choices.push(choice);
}
```

(refactor `generate_choice_resolutions` to delegate to `generate_choice_resolutions_for(state, choice, rules)` so push can query a not-yet-queued choice). WarehouseTake arms: generation = `warehouse_take_options(p)` mapped to `ChooseWarehouseTake` + always `DeclineWarehouseTake`; resolution charges `0 / 2 / take_cost(already)` and moves the token `in_warehouse -= 1; in_hand += 1`. Replace `apply_warehouse_takes` call sites (`apply_fulfill_order`, `apply_space_reward`) with `for _ in 0..n { push_choice(state, PendingChoice::WarehouseTake, rules) }`; in `execute_customs_action`, replace the rider's auto-take with the same push loop (rider yen stays immediate). Delete `apply_warehouse_takes`. Describe arms: `"Take a {item} from the warehouse ({cost})"` / `"Decline the warehouse take"`.

NOTE: `push_choice`'s singleton path can CASCADE (an auto-resolution could itself push) — the recursion is data-bounded; keep it simple recursion.

- [ ] **Step 4: GREEN** + full suites + zero warnings.
- [ ] **Step 5: Commit**: `feat: WarehouseTake choice + singleton auto-resolution + shared TH take-cost (design §5.2)`

**Reviewer notes:** grep `apply_warehouse_takes` (zero callers, deleted); verify Employment Agency behavior unchanged (its existing tests still pin the progression through `take_cost`); hand-verify the singleton-insert-at-front trick resolves against the right head.

---

### Task 3: AssistantMove + 5-Power Movement token

**Files:**
- Modify: `types/choices.rs` (+`MoveSource`, +`PendingChoice::AssistantMove`), `types/moves.rs` (+`ChooseAssistantMove`/`DeclineAssistantMove`), `game/choices.rs` (arms + `assistant_move_options`), `game/apply_move.rs` (`apply_fulfill_order` movement push, `apply_space_reward` movement push, `apply_take_5_power` Movement arm)
- Modify: `crates/yokohama-cli/src/describe.rs` (two arms)

**Interfaces:**
- Produces: `MoveSource { Hand, Area(AreaId) }`, `assistant_move_options(state: &GameState) -> Vec<(MoveSource, AreaId)>`.
- Consumes: Tasks 1-2 machinery. Task 4 queues these from church riders; Task 11 adds the `m` grammar over these moves.

- [ ] **Step 1: Failing tests**:

```rust
#[test]
fn assistant_move_options_cover_hand_and_own_areas_excluding_canal() {
    // 3p board (has Canal). Own assistants: 1 in area A, hand 1. Opponent president in area B.
    let (state, rules) = assistant_move_fixture();
    let legal = get_legal_moves(&state, &rules); // AssistantMove head queued
    // sources: Hand + Area(A); dests: every non-Canal area != source. Decline included.
    assert!(legal.contains(&Move::ChooseAssistantMove { from: MoveSource::Hand, to: area_b() })); // opp president dest OK, free
    assert!(legal.iter().all(|m| match m {
        Move::ChooseAssistantMove { to, .. } => *to != canal_area(),
        _ => true,
    }));
    assert!(legal.contains(&Move::DeclineAssistantMove));
}

#[test]
fn assistant_move_is_free_and_conserves_pawns() {
    let (state, rules) = assistant_move_fixture();
    let money = state.current_player().money;
    let s = apply_move(&state, &Move::ChooseAssistantMove { from: MoveSource::Area(area_a()), to: area_b() }, &rules).unwrap();
    assert_eq!(s.current_player().money, money); // no fees, even into an opponent-president area
    assert_eq!(s.current_player().assistants.on_board.get(&area_a()), None); // cleaned up at 0
    assert_eq!(s.current_player().assistants.on_board[&area_b()], 1);
}

#[test]
fn five_power_movement_token_queues_n_moves_instead_of_recovering() {
    // Movement(2) token at the acting area, player has board assistants.
    let (state, rules) = movement_token_fixture();
    let s = apply_move(&state, &Move::Take5PowerToken, &rules).unwrap();
    assert_eq!(s.pending_choices, vec![PendingChoice::AssistantMove, PendingChoice::AssistantMove]);
    // Nothing was auto-recovered:
    assert_eq!(s.current_player().assistants.in_hand, state.current_player().assistants.in_hand);
}

#[test]
fn assistant_move_auto_declines_when_no_pawn_anywhere() {
    // All 20 assistants in warehouse/locked: only Decline possible -> singleton auto.
    let (mut state, rules) = no_movable_assistant_fixture();
    push_choice(&mut state, PendingChoice::AssistantMove, &rules);
    assert!(state.pending_choices.is_empty());
}
```

- [ ] **Step 2: RED** (compile error). Paste.
- [ ] **Step 3: Implement.** `assistant_move_options`: sources = `Hand` (if `in_hand >= 1`) plus each area with `on_board >= 1`; dests = every `AreaId` in `board_layout.areas` whose `area_type != AreaType::Canal` and `!= from` (for area sources). Resolution: decrement source (hand or on_board with remove-at-zero, matching `apply_recovery_with_leave`'s cleanup style), increment `on_board[to]`. p.10 semantics in a comment: no President fees, each icon separate. Push sites: `apply_fulfill_order` (`rewards.movement`), `apply_space_reward` (`reward.movement` — delete the two movement no-op comments), `apply_take_5_power`'s `Movement(n)` arm (replace the recover-to-hand loop with the push loop; delete the stale comment). Describe: `"Move an assistant from {hand|area} to {area} (reward move)"` / `"Decline the assistant move"`.
- [ ] **Step 4: GREEN** + suites + zero warnings. The existing `test_five_power_movement_token`/`_partial` tests will go RED — rewrite them to pin the new queue behavior (decision: design §5.2, rules-correct per p.10; note the rewrite explicitly in the report).
- [ ] **Step 5: Commit**: `feat: AssistantMove choice encoding; 5-Power Movement token queues real moves (design §5.2)`

---

### Task 4: ChurchSpace choice + movement riders data

**Files:**
- Modify: `types/tracks.rs` (`ChurchTrack::movement_rider`), `types/choices.rs` (+`ChurchSpace { total_faith: u8 }`), `types/moves.rs` (+`ChooseChurchSpace { space: u8 }`), `game/actions/tracks.rs` (`execute_church_action` split; `eligible_church_spaces` plural helper; `place_church_assistant`), `game/choices.rs` (arms)
- Modify: `crates/yokohama-cli/src/describe.rs` (one arm)

**Interfaces:**
- Produces: `eligible_church_spaces(track, total_faith, player_count) -> Vec<u8>` (ascending), `place_church_assistant(state, player, space, rules)` (assistant hand-preferred/area-fallback per I3, VP + Winery, queues `movement_rider(space)` AssistantMoves), `ChurchTrack::movement_rider(space: u8) -> u8` = `[1,1,2,2,2,2,3,3]` for faith 4-11 (decision log 2026-07-17).
- Consumes: Task 3's AssistantMove.

- [ ] **Step 1: Failing tests** (in `game/actions/tracks.rs`):

```rust
#[test]
fn church_movement_riders_match_decision_log() {
    assert_eq!((4..=11).map(ChurchTrack::movement_rider).collect::<Vec<_>>(), vec![1,1,2,2,2,2,3,3]);
}

#[test]
fn church_with_multiple_eligible_spaces_queues_space_choice() {
    // Faith 6, spaces 4,5,6 empty -> 3 eligible -> queue, nothing placed yet.
    let (state, rules) = church_fixture(/*faith yields*/ 6);
    let s = apply_move(&state, &church_power_only_move(), &rules).unwrap();
    assert_eq!(s.pending_choices, vec![PendingChoice::ChurchSpace { total_faith: 6 }]);
    assert_eq!(s.church_track.total_assistants(), 0);
    let legal = get_legal_moves(&s, &rules);
    assert_eq!(legal, vec![
        Move::ChooseChurchSpace { space: 4 },
        Move::ChooseChurchSpace { space: 5 },
        Move::ChooseChurchSpace { space: 6 },
    ]);
}

#[test]
fn choosing_church_space_places_scores_and_queues_riders() {
    let (state, rules) = church_fixture(6);
    let mid = apply_move(&state, &church_power_only_move(), &rules).unwrap();
    let s = apply_move(&mid, &Move::ChooseChurchSpace { space: 6 }, &rules).unwrap();
    assert_eq!(s.church_track.total_assistants(), 1);
    // faith-6 space: 5 VP (existing vp table) + rider move 2 queued
    assert_eq!(s.pending_choices, vec![PendingChoice::AssistantMove, PendingChoice::AssistantMove]);
}

#[test]
fn church_with_single_eligible_space_places_inline_no_choice_event() {
    // Faith 4, only space 4 empty-eligible -> inline placement (singleton, design §5.1).
    let (state, rules) = church_single_space_fixture();
    let s = apply_move(&state, &church_power_only_move(), &rules).unwrap();
    assert_eq!(s.church_track.total_assistants(), 1);
    assert!(s.pending_choices.iter().all(|c| !matches!(c, PendingChoice::ChurchSpace { .. })));
}

#[test]
fn church_2p_faith7_dummy_space_never_offered() {
    let (state, rules) = church_2p_fixture(/*faith*/ 8);
    let s = apply_move(&state, &church_power_only_move(), &rules).unwrap();
    let legal = get_legal_moves(&s, &rules);
    assert!(!legal.contains(&Move::ChooseChurchSpace { space: 7 }));
}
```

- [ ] **Step 2: RED**. Paste. (The existing highest-eligible tests will need adapting to the plural helper — list every test you change in the report.)
- [ ] **Step 3: Implement.** `execute_church_action` keeps ALL current validation + payment; then `let spaces = eligible_church_spaces(..)`; `match spaces.len() { 0 => unreachable (validated), 1 => place_church_assistant(state, player, spaces[0], rules), _ => state.pending_choices.push(PendingChoice::ChurchSpace { total_faith }) }`. `place_church_assistant` is the extracted current placement tail (assistant consumption with I3 hand-preferred/area-fallback, arrival order, VP by space, Winery bonus) plus `for _ in 0..ChurchTrack::movement_rider(space) { push_choice(state, PendingChoice::AssistantMove, rules) }`. Resolution arm for `ChooseChurchSpace` validates membership (generated set) then calls `place_church_assistant`. IMPORTANT: the ACTING AREA for the I3 area-fallback must come from the mercenary-aware helper the executor already uses — pass it into `place_church_assistant` or recompute identically. Describe: `"Place on Church faith-{space} space ({vp} VP)"`.
- [ ] **Step 4: GREEN** + suites + zero warnings. Church-full end-trigger tests still pass (trigger checked at end of every `apply_move`, including the resolution).
- [ ] **Step 5: Commit**: `feat: ChurchSpace choice + faith-space movement riders (design §5.2)`

---

### Task 5: CustomsSpace choice

**Files:**
- Modify: `types/choices.rs` (+`CustomsSpace { cost: u8 }`), `types/moves.rs` (+`ChooseCustomsSpace { space: u8 }`), `game/actions/tracks.rs` (`execute_customs_action` split; `eligible_customs_spaces_for_cost` plural; `place_customs_assistant`), `game/choices.rs` (arms), `crates/yokohama-cli/src/describe.rs` (one arm)

**Interfaces:**
- Produces: `eligible_customs_spaces_for_cost(track, cost, player_count) -> Vec<u8>` (VP-descending, i.e. lower space number first), `place_customs_assistant(state, player, space, rules)` (VP + Brickyard + riders: yen immediate, warehouse takes via `push_choice`).
- Consumes: Task 2 (warehouse riders).

- [ ] **Step 1: Failing tests**: mirror Task 4's shape — both-open queues (`pending == [CustomsSpace { cost }]`, legal = the two `ChooseChurchSpace`-analog moves, VP-order), one-open places inline, choosing space 1 queues two WarehouseTakes + grants its VP, 2p space-4 dummy never offered, Brickyard fires at placement (stacking pinned already — extend to the choice path). Write the five tests concretely, fixtures per the module's existing customs fixtures.
- [ ] **Step 2: RED**. Paste.
- [ ] **Step 3: Implement** exactly per the Task-4 pattern (Task 2's customs-rider push moves from `execute_customs_action` into `place_customs_assistant`). Describe: `"Place on Customs space {space} ({vp} VP)"`.
- [ ] **Step 4: GREEN** + suites + zero warnings.
- [ ] **Step 5: Commit**: `feat: CustomsSpace choice encoding (design §5.2)`

---

### Task 6: Telephone Recovery choice (+ lone-assistant fix)

**Files:**
- Modify: `types/choices.rs` (+`RecoveryLeave`), `types/moves.rs` (+`ChooseRecoveryLeave { leave_one: bool }`), `game/apply_move.rs` (extract `resolve_recovery` + `finish_recovery_transition` from the end-of-apply block; fix `apply_recovery_with_leave` count==1; correct its pinned test), `game/choices.rs` (arms), `crates/yokohama-cli/src/describe.rs` (one arm)

**Interfaces:**
- Produces: `resolve_recovery(state, rules)` — queues `RecoveryLeave` when the current player owns Telephone AND has ≥1 assistant in the acting area; else recovers-all and runs `finish_recovery_transition` (the existing mercenary-return/AdditionalActionSecond logic, extracted).
- Consumes: existing `apply_recovery_with_leave`, Telephone ownership via `rules.techs` (match the Stagecoach-lookup idiom in `credit_movement_president_fees`).

- [ ] **Step 1: Failing tests**:

```rust
#[test]
fn recovery_with_telephone_queues_leave_choice() {
    let (state, rules) = telephone_building_fixture(/*assistants on acting area*/ 2);
    let s = apply_move(&state, &Move::SkipBuilding, &rules).unwrap();
    assert_eq!(s.turn_phase, TurnPhase::Recovery); // paused, not auto-completed
    assert_eq!(s.pending_choices, vec![PendingChoice::RecoveryLeave]);
    assert_eq!(get_legal_moves(&s, &rules), vec![
        Move::ChooseRecoveryLeave { leave_one: false },
        Move::ChooseRecoveryLeave { leave_one: true },
    ]);
}

#[test]
fn telephone_leave_one_keeps_lone_assistant() {
    // count == 1: leaving means recovering ZERO (design §5.3-5; fixes the old pin).
    let (state, rules) = telephone_building_fixture(1);
    let mid = apply_move(&state, &Move::SkipBuilding, &rules).unwrap();
    let s = apply_move(&mid, &Move::ChooseRecoveryLeave { leave_one: true }, &rules).unwrap();
    assert_eq!(s.current_player().assistants.on_board[&acting_area()], 1);
    assert_eq!(s.turn_phase, TurnPhase::AdditionalActionSecond);
}

#[test]
fn recovery_without_telephone_stays_automatic() {
    let (state, rules) = plain_building_fixture(2);
    let s = apply_move(&state, &Move::SkipBuilding, &rules).unwrap();
    assert_eq!(s.turn_phase, TurnPhase::AdditionalActionSecond); // no pause
    assert!(s.pending_choices.is_empty());
}
```

- [ ] **Step 2: RED**. Paste. `test_recovery_with_telephone_only_one_assistant` goes RED with the count==1 fix — rewrite its expectation (leave keeps the pawn), citing design §5.3-5 in a comment.
- [ ] **Step 3: Implement.** `apply_recovery_with_leave`: `if leave_one && count > 1` → `if leave_one && count >= 1` with the `count - 1` recovery (0 at count 1) and `insert(area, 1)`. End-of-apply block becomes: `if new_state.pending_choices.is_empty() && new_state.turn_phase == TurnPhase::Recovery { resolve_recovery(&mut new_state, rules); }` where `resolve_recovery` either pushes `RecoveryLeave` (and RETURNS, leaving phase = Recovery) or calls `apply_recovery_with_leave(state, false)` + `finish_recovery_transition(state)`. The `ChooseRecoveryLeave` resolution arm calls `apply_recovery_with_leave(state, leave_one)` + `finish_recovery_transition(state)`. Mercenary sub-turn Recovery flows through the same path (the transition fn holds the existing return logic verbatim). Describe: `"Recover all assistants from the action area"` / `"Leave 1 assistant behind (Telephone)"`.
- [ ] **Step 4: GREEN** + suites + zero warnings. Verify a mercenary-sub-turn Telephone case in a test (queue fires inside the sub-turn; transition returns to the right window).
- [ ] **Step 5: Commit**: `feat: Telephone Recovery choice; lone-assistant leave fixed (design §5.2, §5.3-5)`

---

### Task 7: UseUniversity + UseSpinningMill

**Files:**
- Modify: `types/moves.rs` (+`UseUniversity`, `UseSpinningMill`), `game/legal_moves.rs:23-101` (`generate_additional_action_moves`), `game/apply_move.rs` (two arms), `crates/yokohama-cli/src/describe.rs` (two arms)

**Interfaces:**
- Consumes: `can_use_university`/`apply_university`, `can_use_spinning_mill`/`apply_spinning_mill` (`game/tech_effects.rs` — already implemented + unit-tested; check their exact signatures before wiring and adapt call sites, not the helpers).

- [ ] **Step 1: Failing tests** (in `game/legal_moves.rs` tests): university offered in `AdditionalActionFirst` when owned + money≥1 + warehouse≥1 + unused; NOT offered after use (flag) or without the tech; spinning mill offered when owned + silk≥1 + unused; both offered in `AdditionalActionSecond`; apply arms: university pays 1 yen and moves warehouse→hand, spinning mill discards 1 silk and gains 2 yen; second use same turn rejected. Six tests, concrete fixtures via the module's tech-fixture pattern.
- [ ] **Step 2: RED**. Paste.
- [ ] **Step 3: Implement**: two `moves.push(..)` gated on the `can_use_*` helpers in `generate_additional_action_moves`; two apply arms calling the `apply_*` helpers (map their error types into `MoveError` as the helpers dictate). Describe: `"Use University: pay 1 yen, take an assistant from the warehouse"` / `"Use Textile Mill: discard 1 silk, gain 2 yen"` (printed name "Textile Mill", cli-guide §8.2). Decision-log row §5.3-4 (windows-only) is drafted by the orchestrator at Task 18.
- [ ] **Step 4: GREEN** + suites + zero warnings.
- [ ] **Step 5: Commit**: `feat: wire University and Textile Mill once-per-turn moves (design §5.2)`

---

### Task 8: Port 5-Power +3 VP on the action (adjudication §5.3-1)

**Files:**
- Modify: `game/apply_move.rs` (`apply_area_action` — single grant site), `game/actions/` (remove the take-path grant — locate with `grep -rn "3" crates/yokohama-core/src/game/actions/commercial.rs | grep -i vp` / read `execute_port_action`)

- [ ] **Step 1: Failing tests**: `skip_at_five_power_port_grants_plus_3vp` (Skip at a Port with power 5 → +3 VP); `port_take_at_five_grants_exactly_once` (take path not double-granted); `port_at_four_power_grants_nothing`; `mercenary_port_action_at_five_grants` (sub-turn path).
- [ ] **Step 2: RED**. Paste.
- [ ] **Step 3: Implement**: in `apply_area_action`, after the action match succeeds, compute the acting area type (mercenary-aware, same idiom as the `Skip` arm) and:

```rust
    // Port 5-Power bonus attaches to the ACTION, not the take (rulebook p.11
    // "In addition, you gain 3 Victory points"; decision-log 2026-07-18,
    // supersedes the v1 Skip-forfeits reading).
    if power == 5 && matches!(area_type, AreaType::PortA | AreaType::PortB) {
        state.player_mut(player).victory_points += 3;
    }
```

and delete the old grant inside the port executor. Any existing test pinning "Skip grants no VP at 5-power Port" gets rewritten with a comment citing the adjudication.
- [ ] **Step 4: GREEN** + suites + zero warnings.
- [ ] **Step 5: Commit**: `fix: Port 5-Power +3 VP attaches to the action, not the take (design §5.3-1)`

---

### Task 9: Canal topology (final-review I4, in-phase)

**PRE-DISPATCH (ORCHESTRATOR-DIRECT):** resolve the rules check against rulebook p.6 + the board art: (a) crossing = straight-across the Canal tile only; (b) 1 Yen per crossing, each crossing in a path pays; (c) the toll goes to the bank. Write the decision-log row; paste the ruling verbatim into the dispatch. Escalate to Jeremy only if the sources conflict.

**Files:**
- Modify: `game/board_gen.rs` (shared `derive_canal_crossings`, adjacency edges), `game/setup.rs` (reconcile_board uses the shared fn), `game/movement.rs` (`calculate_movement_cost_full` start-edge fix)

**Interfaces:**
- Produces: `derive_canal_crossings(...) -> Vec<(AreaId, AreaId)>` — extracted from the two existing inline copies (read both; they must reduce to one function used by both callers), now ALSO fed into the adjacency builder so flanking areas become adjacent.

- [ ] **Step 1: Failing tests**:

```rust
#[test]
fn canal_flanking_areas_are_adjacent_and_marked_as_crossings() {
    let (layout, _) = seeded_3p_layout_with_canal();
    for &(a, b) in &layout.canal_crossings {
        assert!(layout.adjacent_areas(a).contains(&b));
        assert!(layout.is_canal_crossing(a, b));
    }
    assert!(!layout.canal_crossings.is_empty()); // 3p always has a Canal
}

#[test]
fn president_can_cross_canal_paying_one_yen_to_bank() {
    // Own assistant on the far side; path = [far]; start flanks the canal opposite it.
    let (state, rules) = canal_crossing_fixture();
    let legal = get_legal_moves(&state, &rules);
    assert!(legal.contains(&cross_move())); // MovePresident { path: [far], destination: far }
    let s = apply_move(&state, &cross_move(), &rules).unwrap();
    assert_eq!(s.player(mover()).money, state.player(mover()).money - 1); // toll
    // Bank toll: NOT credited to any player.
    let others_total: u32 = s.players.iter().filter(|p| p.id != mover()).map(|p| p.money).sum();
    let others_before: u32 = state.players.iter().filter(|p| p.id != mover()).map(|p| p.money).sum();
    assert_eq!(others_total, others_before);
}

#[test]
fn start_edge_crossing_is_tolled() {
    // The FIRST step of the path crosses the canal (regression for the
    // path.windows(2) gap that missed start -> path[0]).
    let (state, rules) = canal_first_step_fixture();
    let s = apply_move(&state, &first_step_cross_move(), &rules).unwrap();
    assert_eq!(s.player(mover()).money, state.player(mover()).money - 1);
}

#[test]
fn stagecoach_does_not_waive_canal_toll() {
    let (state, rules) = canal_crossing_with_stagecoach_fixture();
    let s = apply_move(&state, &cross_move(), &rules).unwrap();
    assert_eq!(s.player(mover()).money, state.player(mover()).money - 1);
}

#[test]
fn two_player_boards_unchanged() {
    let (layout, _) = seeded_2p_layout();
    assert!(layout.canal_crossings.is_empty());
}
```

Plus a reconcile test: re-typing a 3p board through `reconcile_setup` yields crossings/adjacency identical to generating that layout directly (extend `tests/live_reconciliation.rs`).
- [ ] **Step 2: RED**. Paste.
- [ ] **Step 3: Implement**: extract the shared derivation; for the Canal cell at grid (r,c), the crossing pairs are ((r,c-1),(r,c+1)) and ((r-1,c),(r+1,c)) where both cells exist and are non-Canal areas; add each pair to the adjacency builder (bidirectional) AND to `canal_crossings`. In `calculate_movement_cost_full`, scan crossings over `[start] ++ path` consecutive pairs instead of `path.windows(2)` alone (the mover's current area is the start — thread it in the same way the president-fee component gets it). Keep tolls uncredited (bank).
- [ ] **Step 4: GREEN** + suites + zero warnings + `cargo test -p yokohama-core --test fuzz_full_games` (non-ignored smoke — movement enumeration grew at 3-4p).
- [ ] **Step 5: Commit**: `fix: Canal is crossable — flanking adjacency + toll incl. first step (I4, design §5.6)`

---

### Task 10: Extract `reveal.rs` from `repl.rs` (pure refactor)

**Files:**
- Create: `crates/yokohama-cli/src/reveal.rs`
- Modify: `crates/yokohama-cli/src/repl.rs`, `crates/yokohama-cli/src/lib.rs` (module decl)

Move the reveal machinery (`Pending` enum, `collect_reveal`, `reveal_prompt`, `refilled_port_slots`, `lab_b_changed`, `claimed_token_area`, `lock_on_order_desync`) into `reveal.rs` with `pub(crate)` visibility; `repl.rs` keeps the `step` routing. ZERO behavior change.

- [ ] **Step 1**: `cargo test -p yokohama-cli` before — record the pass count.
- [ ] **Step 2**: perform the move; `cargo test -p yokohama-cli` after — identical pass count, zero warnings, no test text changes.
- [ ] **Step 3**: `wc -l crates/yokohama-cli/src/repl.rs` — expect < 700.
- [ ] **Step 4: Commit**: `refactor: extract reveal choreography into reveal.rs (no behavior change)`

**Reviewer notes:** diff must show pure moves (no logic edits); reject any behavioral drift.

---

### Task 11: Choice banner + `m` grammar + reveal-menu `m`

**Files:**
- Modify: `crates/yokohama-cli/src/describe.rs` (+`describe_pending_choice`), `repl.rs` (banner in `prompt()`, `m` grammar in `step`), `reveal.rs` (token menu `m`)
- Test: CLI scripted-session tests (existing pattern)

**Interfaces:**
- Produces: `describe_pending_choice(head: &PendingChoice, state: &GameState, rules: &Rules) -> String`.
- Consumes: all engine choice types (Tasks 1-6).

- [ ] **Step 1: Failing scripted tests**: (a) drive a sim session into an AnyGood cascade → prompt contains `Choose:` and `1 choice pending` and the numbered `ChooseAnyGood` options; (b) an AssistantMove head accepts `m 4 9` (applies the matching move) and `m hand 9`; (c) `m 4 99` prints a `not one of the legal assistant moves` explanation and applies nothing; (d) undo mid-cascade steps back exactly one choice (prompt shows the same choice again); (e) live-mode 5-Power flip prompt accepts `m` → `RevealFivePowerToken { effect: Movement(2) }` recorded. Write the input-line vectors concretely against a fixed seed (derive the seed's cascade by running the wizard path once and pinning what it shows — the implementer computes and hard-codes the transcript pins, as every scripted test in this crate does).
- [ ] **Step 2: RED**. Paste.
- [ ] **Step 3: Implement**: `prompt()` — when `session.state().pending_choices` is non-empty, the header's phase slot renders `Choose: {describe_pending_choice(head)} — {n} choice(s) pending`. `describe_pending_choice` covers all six variants (wildcard-free match, e.g. WarehouseTake lists the live options with costs: `warehouse take (assistant free / shophouse 2¥ / TH {take_cost}¥)`). Grammar: `["m", src, dst]` where src is `hand` or an area number → build `ChooseAssistantMove`, exact-match against `session.legal()` (reuse the placement-grammar match idiom); miss explains. Reveal menu: add `m` → `FivePowerEffect::Movement(2)` beside the existing six letters (design §5.3-6 note in a comment). HELP_TEXT gains the `m <from> <to>` line.
- [ ] **Step 4: GREEN** + suites + zero warnings.
- [ ] **Step 5: Commit**: `feat: choice banner, assistant-move grammar, movement-token reveal letter (design §5.4)`

---

### Task 12: Delta-summary extensions + display single-sourcing

**Files:**
- Modify: `crates/yokohama-cli/src/session.rs` (delta lines), `render.rs` (customs costs via core, church 2p dummy marker, president-label consistency), `crates/yokohama-core/src/types/tracks.rs` (+`CustomsTrack::space_cost`)

- [ ] **Step 1: Failing tests**: core: `space_cost` returns `[1,1,2,2,3,3,4]` for spaces 1-7. CLI scripted: after a `ChooseAssistantMove`, the delta block contains `assistant moved`; after a `ChooseWarehouseTake`, `took shophouse (2 yen)`-style line; after `ChooseChurchSpace`, `placed on Church faith-N (+M VP)`; after a build, `built shophouse`; after a mercenary award, `gained mercenary`; render: `tracks` view at 2p marks the church faith-7 dummy with `X` (same convention as Customs space 4); `render.rs` no longer contains a `[1, 1, 2, 2, 3, 3, 4]` literal (grep assertion in a test or reviewer check).
- [ ] **Step 2: RED**. Paste.
- [ ] **Step 3: Implement**: `CustomsTrack::space_cost(space: u8) -> u8` (the inverse of the existing cost→index map — single source, render calls it); delta computation extends the existing state-diff walk in `session.rs` (assistants on_board/in_hand/in_warehouse diffs → placed/moved/recovered/took lines; church/customs track diffs → placement lines; building_sites diffs → built lines; mercenary/token count diffs); president labels unified on `describe::area_label`.
- [ ] **Step 4: GREEN** + suites + zero warnings.
- [ ] **Step 5: Commit**: `feat: delta lines for choice moves; single-source customs costs; 2p church dummy marker (design §5.4, §5.7-4/5)`

---

### Task 13: Live-driver hardening cluster

**Files:**
- Modify: `crates/yokohama-core/src/game/record.rs` (replay error carries event index), `types/errors.rs` (WrongPhase removal — SKIP AND WIRE INSTEAD IF JEREMY VETOES §8-7), `crates/yokohama-cli/src/session.rs` (typed apply error, `is_over` gate, fsync, load surfaces index), `reveal.rs` (tech desync assert; typed-outcome use), `repl.rs` (HELP_TEXT `q`, view commands stop reprinting the prompt block)

- [ ] **Step 1: Failing tests**: (a) `replay` on a record with an illegal event at position k returns an error naming k (engine test: build a 3-event record, corrupt event 1, assert the index); (b) `Session::apply` on a finished game returns the typed `GameOver` outcome (construct via a finished fixture); (c) `Session::apply` illegal-move outcome is the typed `Illegal` variant — and `lock_on_order_desync` keys on the type, not on `"Illegal:"` string content (change the check, keep the pinned desync tests green); (d) a lab-refill reveal whose typed card does not match the post-apply board locks the session (mirror of the port desync scripted test); (e) scripted: `board` output is NOT followed by the full numbered-options block, only the one-line input hint; (f) `help` output contains `q`.
- [ ] **Step 2: RED**. Paste.
- [ ] **Step 3: Implement**: `replay` returns `Result<GameState, ReplayError>` with `pub struct ReplayError { pub event_index: usize, pub message: String }` (Display includes both; callers in tests/CLI updated); `Session::apply`/`apply_standalone` return `Result<Vec<String>, ApplyError>` with `enum ApplyError { GameOver, Illegal(String), Invariant(String), Reveal(String) }` (repl maps to user text; the lockdown check matches `ApplyError::Illegal`); explicit `if self.is_over() { return Err(ApplyError::GameOver) }` gate; `save` does `f.sync_all()` before the rename; tech desync assert mirrors `lock_on_order_desync` for lab boards; delete `MoveError::WrongPhase` (compiler lists any stragglers — decision-log row §5.3-7 at Task 18).
- [ ] **Step 4: GREEN** + suites + zero warnings.
- [ ] **Step 5: Commit**: `fix: typed session outcomes, replay event index, is_over gate, fsync, tech desync assert; drop dead WrongPhase (design §5.7)`

---

### Task 14: Wizard robustness

**Files:**
- Create: `crates/yokohama-cli/src/input.rs` (shared `parse_order_id`, `parse_tech_entry`, `tech_name` — extracted from the wizard's three copies + repl's near-verbatim copy)
- Modify: `crates/yokohama-cli/src/wizard.rs` (validation context, unlimited re-ask + `abort`, review/edit step), `repl.rs` (use `input.rs` helpers)

- [ ] **Step 1: Failing scripted tests**: (a) entering a duplicate order id in the live Port-A tail re-asks AT THAT QUESTION with a `already entered` message (no abort, no 30-question deferral); (b) an order id outside 1-36 re-asks with `not a valid order card`; (c) a tile answer that over-fills the player-count multiset re-asks (`no cm tiles remaining for this board`); (d) three consecutive invalid answers do NOT abort (assert the 4th valid answer is accepted — kills 3-strikes); (e) `abort` at any question exits the wizard cleanly; (f) after the last question a summary block prints (contains the entered port cards and tiles), `edit ports` re-runs the port questions and the summary reflects the change, `confirm` proceeds to a live session; (g) sim tail: same re-ask behavior on an invalid keep answer. Update every existing wizard test that pinned 3-strikes.
- [ ] **Step 2: RED**. Paste.
- [ ] **Step 3: Implement**: `ask()` loses the `for _ in 0..3` loop → `loop { }` with `abort` short-circuit returning `Err("aborted")`; per-question validators get a `&WizardSeen` context (sets of entered order ids / tech ids / site indices, remaining tile multiset) threaded through the live tail; review step renders the collected `SetupObservations` + config through the existing display helpers and dispatches `edit <section>` to the section's ask-block (sections: `colors`, `seed`, `save`, `deal <seat>`, `tiles`, `sites`, `achievements`, `porta`, `portb`, `laba`, `labb`); `confirm` falls through to the existing reconcile+finalize tail unchanged.
- [ ] **Step 4: GREEN** + suites + zero warnings.
- [ ] **Step 5: Commit**: `feat: wizard parse-time validation, unlimited re-ask with abort, review/edit step (design §5.5)`

---

### Task 15: `tests/choice_encoding.rs` — behavioral proof + organic census (§6.1-1)

**Files:**
- Create: `crates/yokohama-core/tests/choice_encoding.rs` (uses `tests/common/mod.rs`)

- [ ] **Step 1**: Write the module: one public-API scenario per choice type (drive with `get_legal_moves`/`apply_move` from a `new_game` state or crafted fixture — no helper internals); the three cascade compositions (church→riders, customs-1→2 takes, Movement(2)→2 moves); an undo-equivalence test (record a cascade, truncate one event, replay = state before that choice); a mid-cascade save/load test (serialize `GameRecord` mid-cascade, replay, assert `pending_choices` non-empty and prompts resume); and the census:

```rust
#[test]
fn every_choice_variant_fires_organically_in_random_play() {
    let mut seen_pending: BTreeSet<&'static str> = BTreeSet::new();
    let mut seen_moves: BTreeSet<&'static str> = BTreeSet::new();
    for seed in 0..CENSUS_SEED_CAP {
        run_random_game_observed(seed, |state, mv| {
            if let Some(head) = state.pending_choices.first() { seen_pending.insert(variant_name(head)); }
            seen_moves.insert(choice_move_name(mv)); // None for non-choice moves
        });
        if all_variants_seen(&seen_pending, &seen_moves) { return; }
    }
    panic!("unreached after {CENSUS_SEED_CAP} games: pending {:?} moves {:?}", missing(..));
}
```

with `run_random_game_observed` added to `tests/common/mod.rs` (a callback-accepting twin of `run_random_game_from`). `CENSUS_SEED_CAP = 200` to start; RecoveryLeave and `ChooseRecoveryLeave{leave_one:true}` require Telephone ownership in random play — if the cap can't reach them organically, seed a near-state scenario for exactly those (the fuzz gate's rare-end-trigger precedent) and document which variants are scenario-proven vs organic in the test comments.
- [ ] **Step 2: RED first** (the census test names helpers that don't exist yet — paste the compile RED), then GREEN as the module fills in.
- [ ] **Step 3**: `cargo test -p yokohama-core --test choice_encoding` — all pass; note wall time (< 60s debug or mark the census `#[ignore]`+release like the fuzz gate and wire it into Task 18's gate list — decide by measurement, record the decision in the report).
- [ ] **Step 4: Commit**: `test: choice-encoding behavioral module + organic-coverage census (design §6.1-1)`

---

### Task 16: Effectiveness metrics — selfplay block + `record_stats` (§6.1-2/3)

**Files:**
- Modify: `crates/yokohama-core/examples/selfplay.rs`
- Create: `crates/yokohama-core/examples/record_stats.rs`

- [ ] **Step 1**: selfplay gains a per-run choice-metrics block printed after the existing branching stats: per-`PendingChoice`-type prompted counts/game (detect: queue non-empty before the applied move), mean options offered (`legal.len()` at those moves), decline rates (share of `Decline*` resolutions), cascade-depth distribution (consecutive runs of choice moves), and church/customs inline-auto share (a `Church`/`Customs` area action NOT followed by a space choice = auto-placed singleton). `record_stats.rs`: `cargo run --release -p yokohama-core --example record_stats -- <save.json>` loads a `GameRecord`, replays it step-by-step with the same counters, prints the same block for that one game (works on tester saves and the gate game).
- [ ] **Step 2**: Test by running: `cargo run --release -p yokohama-core --example selfplay -- 100 2 0` — block prints with non-zero AnyGood/WarehouseTake counts (orders with those icons are common). `record_stats` on a save produced by a scripted CLI session prints without error. Paste both outputs in the report.
- [ ] **Step 3: Commit**: `feat: choice-effectiveness metrics in selfplay + record_stats example (design §6.1-2/3)`

---

### Task 17: Tester-guide update

**Files:**
- Modify: `docs/cli-guide.md`, `docs/cli-guide.html`

Rewrite §8.6 (v1 differences → the new choice prompts; keep any still-true caveats), add the `m <from> <to>` grammar to §4/§5, the `m` reveal letter to §6, the wizard re-ask/review behavior to §3 (drop the 3-tries text and the end-of-tail-typo quirk box — both fixed), the canal-crossing note to §8.5, and the choice banner to §4's prompt anatomy. Keep the md and html in sync (the html is a styled mirror — regenerate/edit to match). Verify: `grep -n "3 tries" docs/cli-guide.md` → no hits; every §8.6 auto-resolution bullet either deleted or rewritten as current behavior.
- [ ] **Commit**: `docs: cli-guide updated for choice encoding, wizard flows, canal (design §5.4)`

---

### Task 18: Gates, baselines, decision log, close-out (ORCHESTRATOR-DIRECT)

1. Full suites: `cargo test -p yokohama-core && cargo test -p yokohama-cli`, zero workspace warnings.
2. Release fuzz gate: `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored` — expected pass; investigate ANY failure with one-commit-per-fix discipline (never weaken invariants).
3. Fresh sweeps: `cargo run --release -p yokohama-core --example selfplay -- 5000 <n> 0` for n=2,3,4 — record NEW baselines (moves/sec, game length, per-phase branching, choice-metrics block) in guidingpath's Current state.
4. Census: Task 15's test (release if `#[ignore]`d) + `record_stats` on one scripted save — numbers into the ledger close-out entry (§6.1-4).
5. Decision-log rows (guidingpath.md): §5.3-1 Port +3 VP; §5.3-2 per-icon mining receipts; §5.3-3 warehouse decline; §5.3-4 once-per-turn windows; §5.3-5 Telephone lone-assistant; §5.3-6 movement-token pool box-check flag + reveal letter; §5.3-7 WrongPhase removal (or wiring, per veto); canal ruling (from Task 9); queue architecture + singleton-auto row; `m` grammar supersession row.
6. True-ups: guidingpath Phase-2 section (gate-blockers RESOLVED; testing path; new counts), CLAUDE.md (test counts, v1-limitations paragraph rewritten, canal line), `.superpowers/sdd/progress.md` close-out.
7. Dogfood smoke: piped 2p sim game exercising at least one cascade; a 3p game exercising a canal crossing.
8. Report to Jeremy with `superpowers:finishing-a-development-branch` options. The PHASE gate (live physical game) is scheduled by Jeremy — not claimable by this plan.

---

## Plan Self-Review (performed at authoring)

**Spec coverage:** §5.1 queue → T1-T2; §5.2 all seven choice rows → T1 (AnyGood), T2 (WarehouseTake), T3 (AssistantMove + token), T4 (ChurchSpace + riders), T5 (CustomsSpace), T6 (RecoveryLeave), T7 (University/SpinningMill); §5.3 adjudications → T2 (decline), T3 (token), T6 (lone assistant), T8 (+3 VP), T13 (WrongPhase), T18 (rows; per-icon mining in T1); §5.4 CLI → T10-T12 (+ reveal letter T11); §5.5 wizard → T14; §5.6 canal → T9; §5.7 riding map → T10 (extraction), T12 (space_cost, dummy marker, president labels, deltas), T13 (3a-d, 6, WrongPhase, dogfood smalls), T14 (helpers); §6 testing → per-task + T15; §6.1 module → T15-T16, close-out duty T18; §7 execution → header + task notes. No uncovered spec sections.

**Placeholders:** none — every code step shows code; fixture helpers are named and their construction pattern specified against existing module fixtures (implementers write them concretely; reviewers verify against the pinned assertions).

**Type consistency:** `PendingChoice` variants and `Move` variants match across T1-T7 and T11-T12 usage; `take_cost(usize) -> u32` consistent T2/T12-describe; `ReplayError`/`ApplyError` introduced and consumed only in T13; `describe_pending_choice` introduced T11, used nowhere earlier.

**Known sequencing constraints:** T2 before T5 (customs riders); T3 before T4 (church riders); T1 first; T10 before T11-T13 (reveal.rs exists); T15-T16 after T1-T9; T17 after T11/T14 (documents their behavior); T18 last.
