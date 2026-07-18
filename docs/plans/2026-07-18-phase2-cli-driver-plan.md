# Phase 2 — CLI Game Driver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Phase-2 CLI game driver per `docs/plans/2026-07-18-phase2-cli-driver-design.md`: engine record-v2 event log + live-reconciliation primitives in `yokohama-core`, then a new `yokohama-cli` crate (Session, REPL, wizard, ASCII display) that plays simulated games and live-tracks physical ones.

**Architecture:** Engine first (Tasks 1–7): new `GameState` reconciliation methods, `GameRecord` v2 with `RecordedEvent`, `SetupObservations`/`reconcile_setup`, property tests. CLI second (Tasks 8–13): library crate with a pure REPL step function over a `Session` wrapping `{GameRecord, GameState, Rules}`, thin `main.rs`. Task 14 is the verification/docs sweep.

**Tech Stack:** Rust 2021 workspace; existing workspace deps only (`serde`, `serde_json`; core also uses `rand`/`rand_chacha` in tests). The CLI crate adds **no** new external dependencies.

## Global Constraints

- TDD strictly: failing test first, then minimal code; suite green + **zero warnings** after every task (`cargo test -p yokohama-core` / `-p yokohama-cli` must end `warning: 0` — run `cargo build 2>&1 | grep warning` if unsure).
- `GameState` stays history-free; history lives only in `GameRecord`.
- Engine purity: `apply_move`/`replay`/`reconcile_setup` are deterministic pure functions; nothing reads clocks or ambient RNG.
- Display language: plain words in prompts, short readable abbreviations in tables (decision log 2026-07-16); order cards render as `O<corner-index>` via the existing `order_prompt_line`/`order_compact_line`.
- The CLI never applies a move `get_legal_moves` didn't offer (grammar input is matched against the legal list).
- Never touch `.codex/`, `.vscode/`, `AGENTS.md`, or `tts-assets/` (Jeremy's files). `git add` specific paths only, never `-A`.
- Commit messages follow the repo style: `feat:`/`fix:`/`test:`/`docs:` prefix, imperative summary. Work happens on branch `phase2-cli-driver`.
- Before the final merge: `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored` must pass (Task 14).

---

### Task 1: `arrange_orders_on_top` / `arrange_techs_on_top`

Multi-card deck arrangement (single `promote_*_to_top` swaps cannot compose — each swap disturbs the previous one; port refills draw up to 2+ cards in one `apply_move`).

**Files:**
- Modify: `crates/yokohama-core/src/types/state.rs` (methods after `promote_tech_to_top`, ~line 317; tests in the existing `mod tests`)

**Interfaces:**
- Produces: `GameState::arrange_orders_on_top(&mut self, cards: &[OrderCardId]) -> Result<(), MoveError>` and `GameState::arrange_techs_on_top(&mut self, cards: &[TechCardId]) -> Result<(), MoveError>`. Contract: after `Ok`, the next `cards.len()` `Vec::pop`s of that deck yield `cards` in order; relative order of all other deck cards unchanged; duplicate input → `MoveError::RuleViolation`, any card absent from that deck (removed pile does NOT count) → `MoveError::CardNotFound`; **no mutation on any error**; empty slice is a no-op `Ok`.

- [ ] **Step 1: Write the failing tests** (append inside `mod tests` in `state.rs`):

```rust
#[test]
fn arrange_orders_on_top_sets_pop_order_and_preserves_rest() {
    let mut state = create_test_game_state(2);
    state.order_deck = vec![OrderCardId(1), OrderCardId(2), OrderCardId(3), OrderCardId(4), OrderCardId(5)];
    state.arrange_orders_on_top(&[OrderCardId(2), OrderCardId(4)]).unwrap();
    // Next pops must yield 2 then 4; untouched cards keep relative order 1,3,5.
    assert_eq!(state.order_deck, vec![OrderCardId(1), OrderCardId(3), OrderCardId(5), OrderCardId(4), OrderCardId(2)]);
    assert_eq!(state.order_deck.pop(), Some(OrderCardId(2)));
    assert_eq!(state.order_deck.pop(), Some(OrderCardId(4)));
}

#[test]
fn arrange_orders_on_top_rejects_missing_and_duplicate_without_mutation() {
    let mut state = create_test_game_state(2);
    state.order_deck = vec![OrderCardId(1), OrderCardId(2)];
    state.orders_removed = vec![OrderCardId(9)];
    let before = state.order_deck.clone();
    // 9 is in orders_removed, not the deck: arrangement only serves future DRAWS.
    assert_eq!(state.arrange_orders_on_top(&[OrderCardId(9)]), Err(crate::types::MoveError::CardNotFound));
    assert_eq!(state.order_deck, before);
    assert!(matches!(
        state.arrange_orders_on_top(&[OrderCardId(1), OrderCardId(1)]),
        Err(crate::types::MoveError::RuleViolation(_))
    ));
    assert_eq!(state.order_deck, before);
    assert!(state.arrange_orders_on_top(&[]).is_ok());
    assert_eq!(state.order_deck, before);
}

#[test]
fn arrange_techs_on_top_sets_pop_order() {
    let mut state = create_test_game_state(2);
    state.tech_deck = vec![TechCardId(1), TechCardId(2), TechCardId(3)];
    state.arrange_techs_on_top(&[TechCardId(1)]).unwrap();
    assert_eq!(state.tech_deck.pop(), Some(TechCardId(1)));
    assert_eq!(state.tech_deck, vec![TechCardId(2), TechCardId(3)]);
    assert_eq!(state.arrange_techs_on_top(&[TechCardId(9)]), Err(crate::types::MoveError::CardNotFound));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p yokohama-core arrange_`
Expected: compile error `no method named 'arrange_orders_on_top'`.

- [ ] **Step 3: Implement** (in `impl GameState`, after `promote_tech_to_top`):

```rust
/// Arrange the order deck so the next `cards.len()` draws (`Vec::pop`) yield
/// `cards` in order. Every named card must currently be in `order_deck`
/// (the removed pile does not count — arrangement only serves future draws);
/// the list must be duplicate-free. No mutation on error. Relative order of
/// all unnamed cards is preserved.
pub fn arrange_orders_on_top(&mut self, cards: &[OrderCardId]) -> Result<(), crate::types::MoveError> {
    for (i, c) in cards.iter().enumerate() {
        if cards[..i].contains(c) {
            return Err(crate::types::MoveError::RuleViolation("duplicate card in deck arrangement"));
        }
    }
    if !cards.iter().all(|c| self.order_deck.contains(c)) {
        return Err(crate::types::MoveError::CardNotFound);
    }
    self.order_deck.retain(|id| !cards.contains(id));
    self.order_deck.extend(cards.iter().rev());
    Ok(())
}

/// Tech-deck analog of `arrange_orders_on_top`, same contract.
pub fn arrange_techs_on_top(&mut self, cards: &[TechCardId]) -> Result<(), crate::types::MoveError> {
    for (i, c) in cards.iter().enumerate() {
        if cards[..i].contains(c) {
            return Err(crate::types::MoveError::RuleViolation("duplicate card in deck arrangement"));
        }
    }
    if !cards.iter().all(|c| self.tech_deck.contains(c)) {
        return Err(crate::types::MoveError::CardNotFound);
    }
    self.tech_deck.retain(|id| !cards.contains(id));
    self.tech_deck.extend(cards.iter().rev());
    Ok(())
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p yokohama-core arrange_` then `cargo test -p yokohama-core`
Expected: new tests PASS; full suite green (370 + 3), zero warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/yokohama-core/src/types/state.rs
git commit -m "feat: arrange_orders/techs_on_top for multi-card live-reveal arrangement"
```

---

### Task 2: `reveal_starting_order`

**Files:**
- Modify: `crates/yokohama-core/src/types/state.rs`

**Interfaces:**
- Consumes: `extract_order_from_deck` (deck-then-removed search).
- Produces: `GameState::reveal_starting_order(&mut self, player: PlayerId, card: OrderCardId) -> Result<(), MoveError>`. Contract: requires that player's `starting_order_hidden == true` (else `RuleViolation`); extracts `card` via `extract_order_from_deck` (its `CardNotFound` propagates, no mutation); then pushes to that player's `order_cards_in_hand`, sets `starting_order = Some(card)`, clears the flag. Deliberately no hand-limit check (design §3 Q12 — `check_invariants`' hand ≤ 3 rule is the backstop).

- [ ] **Step 1: Failing tests** (in `state.rs` `mod tests`):

```rust
#[test]
fn reveal_starting_order_pulls_from_removed_pile_into_hand() {
    let mut state = create_test_game_state(2);
    state.players[1].starting_order_hidden = true;
    state.orders_removed = vec![OrderCardId(7), OrderCardId(8)];
    state.reveal_starting_order(PlayerId::new(1), OrderCardId(8)).unwrap();
    assert_eq!(state.players[1].order_cards_in_hand, vec![OrderCardId(8)]);
    assert_eq!(state.players[1].starting_order, Some(OrderCardId(8)));
    assert!(!state.players[1].starting_order_hidden);
    assert_eq!(state.orders_removed, vec![OrderCardId(7)]);
}

#[test]
fn reveal_starting_order_requires_hidden_flag_and_findable_card() {
    let mut state = create_test_game_state(2);
    state.orders_removed = vec![OrderCardId(7)];
    // Not hidden -> RuleViolation, nothing moves.
    assert!(matches!(
        state.reveal_starting_order(PlayerId::new(0), OrderCardId(7)),
        Err(crate::types::MoveError::RuleViolation(_))
    ));
    assert!(state.players[0].order_cards_in_hand.is_empty());
    // Hidden but card nowhere -> CardNotFound, flag stays set.
    state.players[1].starting_order_hidden = true;
    assert_eq!(
        state.reveal_starting_order(PlayerId::new(1), OrderCardId(99)),
        Err(crate::types::MoveError::CardNotFound)
    );
    assert!(state.players[1].starting_order_hidden);
}
```

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-core reveal_starting` → compile error (method missing).

- [ ] **Step 3: Implement** (in `impl GameState`):

```rust
/// Make a live-tracked opponent's hidden starting order public: pull `card`
/// out of the deck-or-removed fiction (`extract_order_from_deck`) and into
/// that player's hand as their revealed starting order.
///
/// Deliberately performs no hand-limit check: tracking must never reject
/// physical reality, and `check_invariants`' standing hand<=3 rule already
/// flags the physically-impossible case loudly (design 2026-07-18, Q12).
pub fn reveal_starting_order(&mut self, player: PlayerId, card: OrderCardId) -> Result<(), crate::types::MoveError> {
    let idx = player.0 as usize;
    if idx >= self.players.len() {
        return Err(crate::types::MoveError::RuleViolation("no such player"));
    }
    if !self.players[idx].starting_order_hidden {
        return Err(crate::types::MoveError::RuleViolation("starting order is not hidden for this player"));
    }
    self.extract_order_from_deck(card)?;
    let p = &mut self.players[idx];
    p.order_cards_in_hand.push(card);
    p.starting_order = Some(card);
    p.starting_order_hidden = false;
    Ok(())
}
```

- [ ] **Step 4: Run**: `cargo test -p yokohama-core reveal_starting` PASS; full core suite green, zero warnings.

- [ ] **Step 5: Commit**: `git add crates/yokohama-core/src/types/state.rs && git commit -m "feat: reveal_starting_order — hidden-fiction reveal into hand"`

---

### Task 3: `reveal_five_power_token`

**Files:**
- Modify: `crates/yokohama-core/src/types/state.rs`

**Interfaces:**
- Produces: `GameState::reveal_five_power_token(&mut self, area: AreaId, effect: FivePowerEffect) -> Result<(), MoveError>`. Contract (design §5.2): area must have a token slot (`RuleViolation("area has no 5-Power token slot")` if absent from the map) and it must be unclaimed (`RuleViolation("5-Power token already claimed at this area")` if `None`). Already matching → no-op `Ok`. Else swap with the lowest-`AreaId` OTHER area holding an unclaimed `Some(effect)`; if none, replace outright (board holds only a 10/13/17-of-20 subset; over-permissive by design — no pool accounting).

- [ ] **Step 1: Failing tests** (in `state.rs` `mod tests`; note `create_test_game_state` has an empty `board_layout`, so tests insert token entries directly):

```rust
#[test]
fn reveal_five_power_token_swaps_with_matching_unclaimed_area() {
    let mut state = create_test_game_state(2);
    state.board_layout.five_power_tokens.insert(AreaId::new(0), Some(FivePowerEffect::Copper(2)));
    state.board_layout.five_power_tokens.insert(AreaId::new(1), Some(FivePowerEffect::Money(3)));
    state.board_layout.five_power_tokens.insert(AreaId::new(2), Some(FivePowerEffect::Money(3)));
    state.reveal_five_power_token(AreaId::new(0), FivePowerEffect::Money(3)).unwrap();
    // Swapped with the LOWEST matching area id (1), not area 2.
    assert_eq!(state.board_layout.five_power_tokens[&AreaId::new(0)], Some(FivePowerEffect::Money(3)));
    assert_eq!(state.board_layout.five_power_tokens[&AreaId::new(1)], Some(FivePowerEffect::Copper(2)));
    assert_eq!(state.board_layout.five_power_tokens[&AreaId::new(2)], Some(FivePowerEffect::Money(3)));
}

#[test]
fn reveal_five_power_token_replaces_when_effect_absent_from_board() {
    let mut state = create_test_game_state(2);
    state.board_layout.five_power_tokens.insert(AreaId::new(0), Some(FivePowerEffect::Copper(2)));
    // Tea(3) is nowhere on the seeded board (it stayed in the unplaced 20-pool remainder).
    state.reveal_five_power_token(AreaId::new(0), FivePowerEffect::Tea(3)).unwrap();
    assert_eq!(state.board_layout.five_power_tokens[&AreaId::new(0)], Some(FivePowerEffect::Tea(3)));
}

#[test]
fn reveal_five_power_token_matching_is_noop_and_structural_errors_reject() {
    let mut state = create_test_game_state(2);
    state.board_layout.five_power_tokens.insert(AreaId::new(0), Some(FivePowerEffect::Silk(2)));
    state.board_layout.five_power_tokens.insert(AreaId::new(1), None); // claimed
    assert!(state.reveal_five_power_token(AreaId::new(0), FivePowerEffect::Silk(2)).is_ok());
    assert!(matches!(
        state.reveal_five_power_token(AreaId::new(1), FivePowerEffect::Silk(2)),
        Err(crate::types::MoveError::RuleViolation("5-Power token already claimed at this area"))
    ));
    assert!(matches!(
        state.reveal_five_power_token(AreaId::new(5), FivePowerEffect::Silk(2)),
        Err(crate::types::MoveError::RuleViolation("area has no 5-Power token slot"))
    ));
}
```

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-core reveal_five_power` → compile error.

- [ ] **Step 3: Implement**:

```rust
/// Reconcile a face-down 5-Power token with its physical reveal at `area`
/// (tokens are face-down until taken — KB 01-rules-reference.md:653, so the
/// seeded assignment is fiction until observed). Prefer swapping with the
/// lowest-AreaId other area holding an unclaimed matching token (multiset-
/// preserving); if the effect is absent from the board, replace outright —
/// the board only carries a random 10/13/17-of-20 subset of the pool, and
/// claimed identities are forgotten (None), so no pool accounting is
/// possible or attempted (design 2026-07-18 §5.2: tracking never rejects
/// physical reality).
pub fn reveal_five_power_token(&mut self, area: AreaId, effect: FivePowerEffect) -> Result<(), crate::types::MoveError> {
    match self.board_layout.five_power_tokens.get(&area) {
        None => return Err(crate::types::MoveError::RuleViolation("area has no 5-Power token slot")),
        Some(None) => return Err(crate::types::MoveError::RuleViolation("5-Power token already claimed at this area")),
        Some(Some(e)) if *e == effect => return Ok(()),
        Some(Some(_)) => {}
    }
    let donor = self
        .board_layout
        .five_power_tokens
        .iter()
        .find(|(a, t)| **a != area && **t == Some(effect))
        .map(|(a, _)| *a);
    let displaced = self.board_layout.five_power_tokens.insert(area, Some(effect)).flatten();
    if let Some(donor) = donor {
        self.board_layout.five_power_tokens.insert(donor, displaced);
    }
    Ok(())
}
```

(`FivePowerEffect` needs importing into `state.rs`'s `use crate::types::{...}` list.)

- [ ] **Step 4: Run**: `cargo test -p yokohama-core reveal_five_power` PASS; full suite green, zero warnings.

- [ ] **Step 5: Commit**: `git add crates/yokohama-core/src/types/state.rs && git commit -m "feat: reveal_five_power_token — swap-preferred face-down token reconciliation"`

---

### Task 4: `GameRecord` v2 — event log

**Files:**
- Modify: `crates/yokohama-core/src/game/record.rs`
- Modify: `crates/yokohama-core/tests/game_record.rs`
- Check/Modify: `crates/yokohama-core/tests/fuzz_full_games.rs`, `crates/yokohama-core/tests/common/mod.rs` (grep for `.moves` / `push_move` uses; `push_move` keeps compiling unchanged)

**Interfaces:**
- Consumes: Task 1–3 state methods.
- Produces:
  - `pub enum RecordedEvent { Move(Move), RevealOrders(Vec<OrderCardId>), RevealTechs(Vec<TechCardId>), RevealStartingOrder { player: PlayerId, card: OrderCardId }, RevealFivePowerToken { area: AreaId, effect: FivePowerEffect } }` (derives `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`).
  - `GameRecord.events: Vec<RecordedEvent>` **replacing** `moves` (breaking schema, sanctioned by design §6.1 — no v1 files exist outside tests).
  - `GameRecord::push_move(&mut self, mv: Move)` unchanged signature (wraps `RecordedEvent::Move`); new `GameRecord::push_event(&mut self, ev: RecordedEvent)`.
  - `replay` folds events: `Move` → `apply_move`; `RevealOrders` → `arrange_orders_on_top`; `RevealTechs` → `arrange_techs_on_top`; `RevealStartingOrder` → `reveal_starting_order`; `RevealFivePowerToken` → `reveal_five_power_token`.

- [ ] **Step 1: Failing test** (replace/extend `record.rs`'s `mod tests` + `tests/game_record.rs`):

In `record.rs` tests, change `push_move_appends_in_order` to assert events, and add a reveal-fold test:

```rust
#[test]
fn push_move_appends_in_order() {
    let mut record = GameRecord::new(0, GameConfig::for_player_count(2), vec![]);
    record.push_move(Move::EndAdditionalPhase);
    record.push_event(RecordedEvent::RevealOrders(vec![]));
    assert_eq!(
        record.events,
        vec![RecordedEvent::Move(Move::EndAdditionalPhase), RecordedEvent::RevealOrders(vec![])]
    );
}

#[test]
fn replay_applies_reveal_events() {
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(2);
    let pending = crate::game::setup::new_game(config.clone(), &rules, 3).unwrap();
    let expected_top = pending.state.order_deck[0]; // bottom card, definitely not next draw
    let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)];
    let mut record = GameRecord::new(3, config, choices);
    record.push_event(RecordedEvent::RevealOrders(vec![expected_top]));
    let state = replay(&record, &rules).unwrap();
    assert_eq!(state.order_deck.last(), Some(&expected_top));
}
```

In `tests/game_record.rs`, update the mid-list-illegal test to the event log:

```rust
    let mid = record.events.len() / 2;
    record.events.insert(mid, RecordedEvent::Move(Move::FulfillOrder { order_id: OrderCardId(255) }));
```

(and `assert!(!record.events.is_empty());`, plus `use yokohama_core::RecordedEvent;`).

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-core --test game_record` and `cargo test -p yokohama-core record` → compile errors (`events`/`RecordedEvent` unknown).

- [ ] **Step 3: Implement** in `record.rs`:

```rust
use crate::types::{AreaId, FivePowerEffect, GameConfig, GameState, Move, MoveError, OrderCardId, PlayerId, TechCardId};

/// One entry in a `GameRecord`'s event log. `Move` is ordinary play; the
/// `Reveal*` variants are live-tracking reconciliation events (design
/// 2026-07-18 §5.2) — they mutate hidden-information fiction (deck order,
/// hidden starting orders, face-down 5-Power tokens) and MUST be in the
/// record so `replay` reproduces the reconciled state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordedEvent {
    Move(Move),
    /// The next draws from the order deck are exactly these, in draw order.
    RevealOrders(Vec<OrderCardId>),
    /// The next draws from the tech deck are exactly these, in draw order.
    RevealTechs(Vec<TechCardId>),
    /// A hidden starting order became public.
    RevealStartingOrder { player: PlayerId, card: OrderCardId },
    /// A face-down 5-Power token was revealed at `area`.
    RevealFivePowerToken { area: AreaId, effect: FivePowerEffect },
}
```

`GameRecord`: rename field `moves: Vec<Move>` → `events: Vec<RecordedEvent>` (doc comment: "Every event applied after setup, in order — moves plus live-reveal reconciliations"); `new` initializes `events: Vec::new()`; `push_move` becomes `self.events.push(RecordedEvent::Move(mv))`; add:

```rust
/// Append one reconciliation/reveal event to the log.
pub fn push_event(&mut self, ev: RecordedEvent) {
    self.events.push(ev);
}
```

`replay`'s fold becomes an explicit loop:

```rust
    let mut state = finalize_setup(pending, &record.choices).map_err(MoveError::RuleViolation)?;
    for ev in &record.events {
        state = match ev {
            RecordedEvent::Move(mv) => apply_move(&state, mv, rules)?,
            RecordedEvent::RevealOrders(cards) => {
                let mut s = state;
                s.arrange_orders_on_top(cards)?;
                s
            }
            RecordedEvent::RevealTechs(cards) => {
                let mut s = state;
                s.arrange_techs_on_top(cards)?;
                s
            }
            RecordedEvent::RevealStartingOrder { player, card } => {
                let mut s = state;
                s.reveal_starting_order(*player, *card)?;
                s
            }
            RecordedEvent::RevealFivePowerToken { area, effect } => {
                let mut s = state;
                s.reveal_five_power_token(*area, *effect)?;
                s
            }
        };
    }
    Ok(state)
```

- [ ] **Step 4: Sweep remaining `.moves` users**

Run: `grep -rn "record.moves\|\.moves\b" crates/yokohama-core --include='*.rs' | grep -v moves_applied` — update any hit (expected: only `tests/game_record.rs`, already done; `tests/common/mod.rs` uses `push_move`, which still compiles).

- [ ] **Step 5: Run**: `cargo test -p yokohama-core` — all green (replay determinism tests prove v2 replays identically), zero warnings.

- [ ] **Step 6: Commit**: `git add crates/yokohama-core/src/game/record.rs crates/yokohama-core/tests/game_record.rs && git commit -m "feat: GameRecord v2 — RecordedEvent log replaces bare move list"`

---

### Task 5: `SetupObservations` + `reconcile_setup` (card zones + achievements)

**Files:**
- Modify: `crates/yokohama-core/src/game/setup.rs` (types + function + tests)
- Modify: `crates/yokohama-core/src/game/record.rs` (add `setup_observations` field + replay branch)

**Interfaces:**
- Produces:
  - `pub struct ObservedBoard { pub tiles: Vec<AreaType>, pub sites: Vec<Option<u8>> }` (derives `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`) — **board handling itself lands in Task 6**; this task defines the type and `reconcile_setup` returns `Err("board observations not yet supported")` if `board.is_some()` (replaced in Task 6).
  - `pub struct SetupObservations { pub board: Option<ObservedBoard>, pub achievements: Option<[AchievementType; 3]>, pub port_a: Option<[OrderCardId; 6]>, pub port_b: Option<[OrderCardId; 6]>, pub lab_a: Option<[TechCardId; 6]>, pub lab_b: Option<[TechCardId; 6]>, pub deals: Vec<(PlayerId, [OrderCardId; 2])> }` (same derives + `Default`).
  - `pub fn reconcile_setup(pending: PendingSetup, obs: &SetupObservations, rules: &Rules) -> Result<PendingSetup, &'static str>` — swap-based (design §5.2): each observed (zone, position, card) is satisfied by swapping the card in from wherever it currently sits (order zones: deck / removed / port A / port B / any deal; tech zones: deck / lab A / lab B). Achievements replaced outright. Validations: duplicate observed ids rejected; ids must exist in registries; `port_b`/`lab_b` observations only when config has those boards; `deals` player ids in range, no duplicate players.
  - `GameRecord.setup_observations: Option<SetupObservations>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`; `GameRecord::new` sets `None`; `replay` runs `reconcile_setup` between `new_game` and `finalize_setup` when present.

- [ ] **Step 1: Failing tests** (in `setup.rs` `mod tests`):

```rust
#[test]
fn reconcile_setup_makes_observed_zones_match_and_conserves_cards() {
    let rules = Rules::standard();
    // Two different seeds play the role of "engine fiction" vs "physical table".
    let pending = new_game(GameConfig::for_player_count(2), &rules, 11).unwrap();
    let table = new_game(GameConfig::for_player_count(2), &rules, 99).unwrap();
    let obs = SetupObservations {
        port_a: Some(std::array::from_fn(|i| table.state.port_state.port_a.slots[i].unwrap())),
        lab_a: Some(std::array::from_fn(|i| table.state.lab_state.lab_a.slots[i].unwrap())),
        deals: vec![(PlayerId::new(0), table.deals[0])],
        ..Default::default()
    };
    let before_multiset = order_multiset(&pending);
    let reconciled = reconcile_setup(pending, &obs, &rules).unwrap();
    for i in 0..6 {
        assert_eq!(reconciled.state.port_state.port_a.slots[i], Some(obs.port_a.unwrap()[i]));
        assert_eq!(reconciled.state.lab_state.lab_a.slots[i], Some(obs.lab_a.unwrap()[i]));
    }
    assert_eq!(reconciled.deals[0], table.deals[0]);
    assert_eq!(order_multiset(&reconciled), before_multiset); // conservation
    // Determinism: same inputs, same output.
    let again = reconcile_setup(new_game(GameConfig::for_player_count(2), &rules, 11).unwrap(), &obs, &rules).unwrap();
    assert_eq!(again, reconciled);
}

#[test]
fn reconcile_setup_rejects_bad_observations() {
    let rules = Rules::standard();
    let pending = new_game(GameConfig::for_player_count(2), &rules, 11).unwrap();
    // Duplicate order id across observed zones.
    let dup = SetupObservations {
        port_a: Some([OrderCardId(1), OrderCardId(1), OrderCardId(2), OrderCardId(3), OrderCardId(4), OrderCardId(5)]),
        ..Default::default()
    };
    assert!(reconcile_setup(pending.clone(), &dup, &rules).is_err());
    // Port B observed in a 2p game (no Port B).
    let no_b = SetupObservations {
        port_b: Some([OrderCardId(1), OrderCardId(2), OrderCardId(3), OrderCardId(4), OrderCardId(5), OrderCardId(6)]),
        ..Default::default()
    };
    assert!(reconcile_setup(pending.clone(), &no_b, &rules).is_err());
    // Unknown order id (0 is not a printed corner index; real ids are 1-36).
    let bad_id = SetupObservations {
        deals: vec![(PlayerId::new(0), [OrderCardId(0), OrderCardId(2)])],
        ..Default::default()
    };
    assert!(reconcile_setup(pending.clone(), &bad_id, &rules).is_err());
    // Achievements replacement works.
    let ach = SetupObservations {
        achievements: Some([AchievementType::Silk6, AchievementType::Orders5, AchievementType::Commercial4]),
        ..Default::default()
    };
    let r = reconcile_setup(pending, &ach, &rules).unwrap();
    assert_eq!(r.state.achievements[0].achievement_type, AchievementType::Silk6);
    assert!(r.state.achievements[0].claimed_by.is_empty());
}
```

with the shared helper (test module):

```rust
fn order_multiset(p: &PendingSetup) -> std::collections::BTreeMap<u8, usize> {
    let mut m = std::collections::BTreeMap::new();
    let mut add = |id: OrderCardId| *m.entry(id.0).or_insert(0) += 1;
    p.state.order_deck.iter().copied().for_each(&mut add);
    p.state.orders_removed.iter().copied().for_each(&mut add);
    p.state.port_state.port_a.slots.iter().flatten().copied().for_each(&mut add);
    if let Some(b) = &p.state.port_state.port_b {
        b.slots.iter().flatten().copied().for_each(&mut add);
    }
    p.deals.iter().flat_map(|d| d.iter()).copied().for_each(&mut add);
    m
}
```

Plus a replay-integration test in `record.rs` tests:

```rust
#[test]
fn replay_applies_setup_observations() {
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(2);
    let table = crate::game::setup::new_game(config.clone(), &rules, 99).unwrap();
    let obs = crate::game::setup::SetupObservations {
        port_a: Some(std::array::from_fn(|i| table.state.port_state.port_a.slots[i].unwrap())),
        ..Default::default()
    };
    let mut record = GameRecord::new(11, config, vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]);
    record.setup_observations = Some(obs.clone());
    let state = replay(&record, &rules).unwrap();
    for i in 0..6 {
        assert_eq!(state.port_state.port_a.slots[i], Some(obs.port_a.unwrap()[i]));
    }
}
```

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-core reconcile` → compile errors.

- [ ] **Step 3: Implement** in `setup.rs`. Private location model + swap plumbing:

```rust
/// Physical-table observations to reconcile into a seeded `PendingSetup`
/// before `finalize_setup` (design 2026-07-18 §5.2). All fields optional so
/// partial observation works; the live wizard fills everything observable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// Observed physical board arrangement (handled in reconcile_setup's board
/// pass): area tile per grid cell and building-site card per grid cell,
/// row-major over `board_gen::grid_dimensions`. `sites` is None exactly
/// where the tile carries no site (Canal).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedBoard {
    pub tiles: Vec<crate::types::AreaType>,
    pub sites: Vec<Option<u8>>,
}

/// Every place an order card can sit inside a PendingSetup.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OrderLoc {
    Deck(usize),
    Removed(usize),
    PortA(usize),
    PortB(usize),
    Deal { player: usize, slot: usize },
}

fn get_order(p: &PendingSetup, loc: OrderLoc) -> OrderCardId {
    match loc {
        OrderLoc::Deck(i) => p.state.order_deck[i],
        OrderLoc::Removed(i) => p.state.orders_removed[i],
        OrderLoc::PortA(i) => p.state.port_state.port_a.slots[i].expect("setup port slot occupied"),
        OrderLoc::PortB(i) => p.state.port_state.port_b.as_ref().expect("port B present").slots[i].expect("setup port slot occupied"),
        OrderLoc::Deal { player, slot } => p.deals[player][slot],
    }
}

fn set_order(p: &mut PendingSetup, loc: OrderLoc, card: OrderCardId) {
    match loc {
        OrderLoc::Deck(i) => p.state.order_deck[i] = card,
        OrderLoc::Removed(i) => p.state.orders_removed[i] = card,
        OrderLoc::PortA(i) => p.state.port_state.port_a.slots[i] = Some(card),
        OrderLoc::PortB(i) => p.state.port_state.port_b.as_mut().expect("port B present").slots[i] = Some(card),
        OrderLoc::Deal { player, slot } => p.deals[player][slot] = card,
    }
}

fn locate_order(p: &PendingSetup, card: OrderCardId) -> Option<OrderLoc> {
    if let Some(i) = p.state.order_deck.iter().position(|&c| c == card) {
        return Some(OrderLoc::Deck(i));
    }
    if let Some(i) = p.state.orders_removed.iter().position(|&c| c == card) {
        return Some(OrderLoc::Removed(i));
    }
    if let Some(i) = p.state.port_state.port_a.slots.iter().position(|&s| s == Some(card)) {
        return Some(OrderLoc::PortA(i));
    }
    if let Some(b) = &p.state.port_state.port_b {
        if let Some(i) = b.slots.iter().position(|&s| s == Some(card)) {
            return Some(OrderLoc::PortB(i));
        }
    }
    for (player, deal) in p.deals.iter().enumerate() {
        for slot in 0..2 {
            if deal[slot] == card {
                return Some(OrderLoc::Deal { player, slot });
            }
        }
    }
    None
}

/// Swap `card` into `dest` from wherever it currently sits. No-op if already there.
fn place_order(p: &mut PendingSetup, dest: OrderLoc, card: OrderCardId) -> Result<(), &'static str> {
    let src = locate_order(p, card).ok_or("observed order card not present anywhere in this setup")?;
    if src == dest {
        return Ok(());
    }
    let displaced = get_order(p, dest);
    set_order(p, dest, card);
    set_order(p, src, displaced);
    Ok(())
}
```

The tech analog is identical with `TechLoc { Deck(usize), LabA(usize), LabB(usize) }`, `get_tech`/`set_tech`/`locate_tech`/`place_tech` over `tech_deck` / `lab_state.lab_a.slots` / `lab_state.lab_b`.

Then the public function:

```rust
/// Reconcile physical-table observations into a seeded PendingSetup by
/// swapping observed cards into their observed zones (conservation by
/// exchange), replacing achievements, and (board pass) re-typing the grid.
/// Pure and deterministic; called by `replay` when a record carries
/// observations, and by the live wizard.
pub fn reconcile_setup(
    mut pending: PendingSetup,
    obs: &SetupObservations,
    rules: &Rules,
) -> Result<PendingSetup, &'static str> {
    // --- validation ---
    if obs.port_b.is_some() && pending.state.port_state.port_b.is_none() {
        return Err("port B observed but not in play at this player count");
    }
    if obs.lab_b.is_some() && pending.state.lab_state.lab_b.is_none() {
        return Err("lab B observed but not in play at this player count");
    }
    let mut seen_orders = std::collections::BTreeSet::new();
    let observed_orders: Vec<OrderCardId> = obs
        .port_a.iter().flatten().copied()
        .chain(obs.port_b.iter().flatten().copied())
        .chain(obs.deals.iter().flat_map(|(_, d)| d.iter().copied()))
        .collect();
    for c in &observed_orders {
        if rules.orders.get(*c).is_none() {
            return Err("observed order card id not in the registry");
        }
        if !seen_orders.insert(c.0) {
            return Err("duplicate order card across observations");
        }
    }
    let mut seen_techs = std::collections::BTreeSet::new();
    let observed_techs: Vec<TechCardId> =
        obs.lab_a.iter().flatten().copied().chain(obs.lab_b.iter().flatten().copied()).collect();
    for c in &observed_techs {
        if rules.techs.get(*c).is_none() {
            return Err("observed tech card id not in the registry");
        }
        if !seen_techs.insert(c.0) {
            return Err("duplicate tech card across observations");
        }
    }
    let mut seen_players = std::collections::BTreeSet::new();
    for (pid, _) in &obs.deals {
        if pid.0 as usize >= pending.deals.len() {
            return Err("observed deal for a player not in this game");
        }
        if !seen_players.insert(pid.0) {
            return Err("duplicate player in observed deals");
        }
    }
    if let Some(board) = &obs.board {
        reconcile_board(&mut pending, board, rules)?; // Task 6; stub errors until then
    }
    if let Some(ach) = &obs.achievements {
        pending.state.achievements = [
            AchievementCard::new(ach[0]),
            AchievementCard::new(ach[1]),
            AchievementCard::new(ach[2]),
        ];
    }
    // --- swaps, fixed order ---
    if let Some(cards) = &obs.port_a {
        for (i, c) in cards.iter().enumerate() {
            place_order(&mut pending, OrderLoc::PortA(i), *c)?;
        }
    }
    if let Some(cards) = &obs.port_b {
        for (i, c) in cards.iter().enumerate() {
            place_order(&mut pending, OrderLoc::PortB(i), *c)?;
        }
    }
    for (pid, deal) in &obs.deals {
        for slot in 0..2 {
            place_order(&mut pending, OrderLoc::Deal { player: pid.0 as usize, slot }, deal[slot])?;
        }
    }
    if let Some(cards) = &obs.lab_a {
        for (i, c) in cards.iter().enumerate() {
            place_tech(&mut pending, TechLoc::LabA(i), *c)?;
        }
    }
    if let Some(cards) = &obs.lab_b {
        for (i, c) in cards.iter().enumerate() {
            place_tech(&mut pending, TechLoc::LabB(i), *c)?;
        }
    }
    Ok(pending)
}

/// Board pass — implemented in the next task.
fn reconcile_board(
    _pending: &mut PendingSetup,
    _board: &ObservedBoard,
    _rules: &Rules,
) -> Result<(), &'static str> {
    Err("board observations not yet supported")
}
```

In `record.rs`: add the field + replay branch:

```rust
    /// Live-tracked games: physical-table observations reconciled into the
    /// seeded PendingSetup before finalize_setup. None = pure simulation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_observations: Option<SetupObservations>,
```

```rust
    let mut pending = new_game(record.config.clone(), rules, record.seed).map_err(MoveError::RuleViolation)?;
    if let Some(obs) = &record.setup_observations {
        pending = reconcile_setup(pending, obs, rules).map_err(MoveError::RuleViolation)?;
    }
```

(`GameRecord::new` gains `setup_observations: None`; import `reconcile_setup, SetupObservations` from `crate::game::setup`.)

- [ ] **Step 4: Run**: `cargo test -p yokohama-core` — green, zero warnings. `PendingSetup` derives `PartialEq` already (determinism assert works).

- [ ] **Step 5: Commit**: `git add crates/yokohama-core/src/game/setup.rs crates/yokohama-core/src/game/record.rs && git commit -m "feat: SetupObservations + swap-based reconcile_setup wired into replay"`

---

### Task 6: `ObservedBoard` re-typing

**Files:**
- Modify: `crates/yokohama-core/src/game/setup.rs` (replace the `reconcile_board` stub; tests)

**Interfaces:**
- Consumes: `board_gen::grid_dimensions(player_count) -> (u8, u8)` (pub), `rules.building_sites: Vec<BuildingSite>`.
- Produces: working `reconcile_board`: permutes `area_type` over the fixed row-major grid (AreaId i ↔ cell (i/cols, i%cols) — exactly how `generate_board_layout` assigns ids), reassigns `building_sites` from registry indices, rebuilds `five_power_tokens` (seeded token values reassigned in ascending order over the NEW non-Canal AreaIds), recomputes `canal_crossings` from the new Canal cell's orthogonal neighbors (empty when no Canal). Adjacency untouched. Validations: `tiles.len()` == area count; tile multiset equals the seeded layout's; `sites.len()` == `tiles.len()`; `sites[i]` is `None` iff `tiles[i] == Canal`; site indices distinct and `< rules.building_sites.len()`.

- [ ] **Step 1: Failing tests** (in `setup.rs` `mod tests`):

```rust
#[test]
fn reconcile_board_retypes_grid_sites_and_canal() {
    let rules = Rules::standard();
    let pending = new_game(GameConfig::for_player_count(3), &rules, 5).unwrap();
    // Observed board: the seeded tiles REVERSED (a valid permutation), sites 0..13 in order.
    let (rows, cols) = crate::game::board_gen::grid_dimensions(3);
    let n = (rows * cols) as usize;
    let mut tiles: Vec<AreaType> = (0..n)
        .map(|i| pending.state.board_layout.areas[&AreaId::new(i as u8)].area_type)
        .collect();
    tiles.reverse();
    let sites: Vec<Option<u8>> = tiles
        .iter()
        .scan(0u8, |next, t| {
            Some(if *t == AreaType::Canal { None } else { let s = *next; *next += 1; Some(s) })
        })
        .collect();
    let obs = SetupObservations {
        board: Some(ObservedBoard { tiles: tiles.clone(), sites: sites.clone() }),
        ..Default::default()
    };
    let r = reconcile_setup(pending, &obs, &rules).unwrap();
    let layout = &r.state.board_layout;
    // Types permuted as observed; grid positions unchanged (row-major ids).
    for i in 0..n {
        let id = AreaId::new(i as u8);
        assert_eq!(layout.areas[&id].area_type, tiles[i]);
        assert_eq!(layout.areas[&id].grid_position, ((i / cols as usize) as u8, (i % cols as usize) as u8));
    }
    // Sites follow the observed assignment; Canal has none.
    for i in 0..n {
        let id = AreaId::new(i as u8);
        match sites[i] {
            Some(s) => assert_eq!(layout.building_sites[&id], rules.building_sites[s as usize].clone_reset()),
            None => assert!(!layout.building_sites.contains_key(&id)),
        }
    }
    // Tokens: same count as before, all on non-Canal areas, all unclaimed.
    assert_eq!(layout.five_power_tokens.len(), n - 1);
    assert!(layout.five_power_tokens.values().all(|t| t.is_some()));
    assert!(!layout.five_power_tokens.contains_key(
        &AreaId::new(tiles.iter().position(|t| *t == AreaType::Canal).unwrap() as u8)
    ));
    // Canal crossings follow the Canal's new cell (its orthogonal neighbors).
    let canal_i = tiles.iter().position(|t| *t == AreaType::Canal).unwrap();
    assert!(!layout.canal_crossings.is_empty());
    assert!(layout.canal_crossings.iter().all(|(a, _)| *a == AreaId::new(canal_i as u8)));
}

#[test]
fn reconcile_board_rejects_multiset_and_site_violations() {
    let rules = Rules::standard();
    let pending = new_game(GameConfig::for_player_count(2), &rules, 5).unwrap();
    let n = pending.state.board_layout.areas.len();
    // Wrong multiset: all Bank tiles.
    let obs = SetupObservations {
        board: Some(ObservedBoard { tiles: vec![AreaType::Bank; n], sites: vec![Some(0); n] }),
        ..Default::default()
    };
    assert!(reconcile_setup(pending.clone(), &obs, &rules).is_err());
    // Duplicate site index.
    let tiles: Vec<AreaType> = (0..n)
        .map(|i| pending.state.board_layout.areas[&AreaId::new(i as u8)].area_type)
        .collect();
    let obs = SetupObservations {
        board: Some(ObservedBoard { tiles, sites: vec![Some(0); n] }),
        ..Default::default()
    };
    assert!(reconcile_setup(pending, &obs, &rules).is_err());
}
```

Note: if `BuildingSite` has no `clone_reset` helper, assert instead on `.spaces` and `.trading_house_reward` equality against `rules.building_sites[s as usize]` (built-state fields are fresh at setup) — implementer's choice, no new public API required.

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-core reconcile_board` → fails with "board observations not yet supported".

- [ ] **Step 3: Implement** `reconcile_board`:

```rust
fn reconcile_board(
    pending: &mut PendingSetup,
    board: &ObservedBoard,
    rules: &Rules,
) -> Result<(), &'static str> {
    use crate::types::AreaType;
    let layout = &mut pending.state.board_layout;
    let n = layout.areas.len();
    if board.tiles.len() != n {
        return Err("observed tile count does not match this player count's grid");
    }
    if board.sites.len() != n {
        return Err("observed site list length must match the tile list");
    }
    // Multiset equality vs the seeded layout (which already encodes the
    // player-count removals) — also the physical-verification tripwire
    // (design 2026-07-18 §5.2).
    let mut want: Vec<AreaType> = layout.areas.values().map(|a| a.area_type).collect();
    let mut got: Vec<AreaType> = board.tiles.clone();
    let key = |t: &AreaType| format!("{t:?}");
    want.sort_by_key(key);
    got.sort_by_key(key);
    if want != got {
        return Err("observed tiles are not a permutation of this player count's tile set");
    }
    let mut seen_sites = std::collections::BTreeSet::new();
    for (i, s) in board.sites.iter().enumerate() {
        match s {
            None if board.tiles[i] != AreaType::Canal => return Err("missing site on a non-Canal tile"),
            Some(_) if board.tiles[i] == AreaType::Canal => return Err("Canal cannot carry a building site"),
            Some(idx) => {
                if *idx as usize >= rules.building_sites.len() {
                    return Err("observed building-site index out of range");
                }
                if !seen_sites.insert(*idx) {
                    return Err("duplicate building-site card across cells");
                }
            }
            None => {}
        }
    }
    // Seeded token values, reassigned over the NEW non-Canal ids in order.
    let tokens: Vec<Option<crate::types::FivePowerEffect>> =
        layout.five_power_tokens.values().copied().collect();
    // Re-type areas (grid positions are fixed: AreaId i is cell (i/cols, i%cols)).
    for (i, t) in board.tiles.iter().enumerate() {
        layout.areas.get_mut(&AreaId::new(i as u8)).expect("grid area exists").area_type = *t;
    }
    // Reassign sites.
    layout.building_sites.clear();
    for (i, s) in board.sites.iter().enumerate() {
        if let Some(idx) = s {
            layout.building_sites.insert(AreaId::new(i as u8), rules.building_sites[*idx as usize].clone());
        }
    }
    // Reassign tokens over new non-Canal ids.
    layout.five_power_tokens.clear();
    let mut token_iter = tokens.into_iter();
    for (i, t) in board.tiles.iter().enumerate() {
        if *t != AreaType::Canal {
            layout.five_power_tokens.insert(
                AreaId::new(i as u8),
                token_iter.next().ok_or("token pool smaller than non-Canal cell count")?,
            );
        }
    }
    // Recompute canal crossings from the new Canal cell (if any).
    layout.canal_crossings.clear();
    let (rows, cols) = crate::game::board_gen::grid_dimensions(pending.state.config.player_count);
    if let Some(ci) = board.tiles.iter().position(|t| *t == AreaType::Canal) {
        let (r, c) = ((ci / cols as usize) as i16, (ci % cols as usize) as i16);
        for (nr, nc) in [(r - 1, c), (r + 1, c), (r, c - 1), (r, c + 1)] {
            if nr >= 0 && nr < rows as i16 && nc >= 0 && nc < cols as i16 {
                let neighbor = AreaId::new((nr as usize * cols as usize + nc as usize) as u8);
                layout.canal_crossings.push((AreaId::new(ci as u8), neighbor));
            }
        }
    }
    Ok(())
}
```

(Building-site state fields at setup are all-fresh, so cloning from the registry is exact. If sorting `AreaType` by `format!` offends, derive `Ord` — implementer's choice; keep it local.)

- [ ] **Step 4: Run**: `cargo test -p yokohama-core` — green, zero warnings.

- [ ] **Step 5: Commit**: `git add crates/yokohama-core/src/game/setup.rs && git commit -m "feat: ObservedBoard re-typing — tiles/sites/canal reconciliation on the fixed grid"`

---

### Task 7: Live-reconciliation property tests

**Files:**
- Create: `crates/yokohama-core/tests/live_reconciliation.rs`

**Interfaces:**
- Consumes: everything from Tasks 1–6; `tests/common/mod.rs` patterns (`ChaCha8Rng`, `get_legal_moves`, `apply_move`, `check_invariants`).

- [ ] **Step 1: Write the tests** (whole file; these ARE the deliverable — they fail only if Tasks 1–6 are wrong, which is the point):

```rust
//! Phase-2 live-reconciliation property tests (design 2026-07-18 §5.6):
//! a game recorded WITH reveal events replays to the exact final state,
//! both when reveals name the seeded draws (identity) and when they name
//! different cards (permuted — the live-tracking case).

mod common;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move, check_invariants, finalize_setup, get_legal_moves, is_game_over, new_game, replay,
    GameConfig, GameRecord, RecordedEvent, Rules, StartingOrderChoice,
};

/// Play up to `max_moves` random moves; before every deck-drawing move,
/// inject RevealOrders/RevealTechs events. `permute=false` names the exact
/// seeded upcoming draws (identity); `permute=true` names the BOTTOM cards
/// of the deck instead (a genuinely different assignment) when the deck is
/// deep enough, falling back to identity when it isn't.
fn play_with_reveals(seed: u64, players: u8, max_moves: usize, permute: bool, rules: &Rules) -> (yokohama_core::GameState, GameRecord) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let config = GameConfig::for_player_count(players);
    let pending = new_game(config.clone(), rules, seed).unwrap();
    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
    let mut record = GameRecord::new(seed, config, choices.clone());
    let mut state = finalize_setup(pending, &choices).unwrap();

    for _ in 0..max_moves {
        if is_game_over(&state) {
            break;
        }
        let moves = get_legal_moves(&state, rules);
        assert!(!moves.is_empty(), "dead end at turn {}", state.turn_number);
        let mv = moves[rng.gen_range(0..moves.len())].clone();

        let scratch = apply_move(&state, &mv, rules).expect("legal move applies");
        let d_orders = state.order_deck.len() - scratch.order_deck.len();
        let d_techs = state.tech_deck.len() - scratch.tech_deck.len();

        if d_orders > 0 {
            let cards: Vec<_> = if permute && state.order_deck.len() >= 2 * d_orders {
                state.order_deck.iter().take(d_orders).copied().collect() // bottom cards
            } else {
                state.order_deck.iter().rev().take(d_orders).copied().collect() // seeded draws
            };
            state.arrange_orders_on_top(&cards).unwrap();
            record.push_event(RecordedEvent::RevealOrders(cards));
        }
        if d_techs > 0 {
            let cards: Vec<_> = if permute && state.tech_deck.len() >= 2 * d_techs {
                state.tech_deck.iter().take(d_techs).copied().collect()
            } else {
                state.tech_deck.iter().rev().take(d_techs).copied().collect()
            };
            state.arrange_techs_on_top(&cards).unwrap();
            record.push_event(RecordedEvent::RevealTechs(cards));
        }

        state = apply_move(&state, &mv, rules).expect("legal move applies after arrangement");
        check_invariants(&state, rules).expect("invariants hold after every applied move");
        record.push_move(mv);
    }
    (state, record)
}

#[test]
fn identity_reveals_replay_to_identical_state() {
    let rules = Rules::standard();
    for players in [2u8, 3, 4] {
        for seed in 0..3u64 {
            let (final_state, record) = play_with_reveals(seed, players, 120, false, &rules);
            assert_eq!(replay(&record, &rules).unwrap(), final_state, "seed {seed} {players}p");
        }
    }
}

#[test]
fn permuted_reveals_replay_to_identical_state_and_hold_invariants() {
    let rules = Rules::standard();
    for players in [2u8, 3, 4] {
        for seed in 0..3u64 {
            let (final_state, record) = play_with_reveals(seed, players, 120, true, &rules);
            let replayed = replay(&record, &rules).unwrap();
            assert_eq!(replayed, final_state, "seed {seed} {players}p");
            check_invariants(&replayed, &rules).unwrap();
            // JSON round-trip of a reveal-bearing record preserves replay.
            let json = serde_json::to_string(&record).unwrap();
            let back: GameRecord = serde_json::from_str(&json).unwrap();
            assert_eq!(replay(&back, &rules).unwrap(), final_state);
        }
    }
}

#[test]
fn revealed_hidden_starting_order_replays() {
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(2);
    let pending = new_game(config.clone(), &rules, 21).unwrap();
    let hidden_deal = pending.deals[1];
    let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown];
    let mut record = GameRecord::new(21, config, choices.clone());
    let mut state = finalize_setup(pending, &choices).unwrap();

    state.reveal_starting_order(yokohama_core::PlayerId::new(1), hidden_deal[0]).unwrap();
    record.push_event(RecordedEvent::RevealStartingOrder {
        player: yokohama_core::PlayerId::new(1),
        card: hidden_deal[0],
    });
    check_invariants(&state, &rules).unwrap();
    assert_eq!(replay(&record, &rules).unwrap(), state);
}

#[test]
fn setup_observations_from_another_seed_replay_and_play_on() {
    // Full live-setup shape: board + achievements + ports + labs + own deal
    // observed from a DIFFERENT seed's game (standing in for the physical
    // table), then random play proceeds and the record replays.
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(3);
    let table = new_game(config.clone(), &rules, 777).unwrap();
    let n = table.state.board_layout.areas.len();
    let obs = yokohama_core::SetupObservations {
        board: Some(yokohama_core::ObservedBoard {
            tiles: (0..n)
                .map(|i| table.state.board_layout.areas[&yokohama_core::AreaId::new(i as u8)].area_type)
                .collect(),
            sites: {
                // Any valid distinct assignment works as an observation;
                // reuse indices 0.. in cell order, skipping the Canal cell.
                let mut next = 0u8;
                (0..n)
                    .map(|i| {
                        let t = table.state.board_layout.areas[&yokohama_core::AreaId::new(i as u8)].area_type;
                        if t == yokohama_core::AreaType::Canal { None } else { let s = next; next += 1; Some(s) }
                    })
                    .collect()
            },
        }),
        achievements: Some([
            table.state.achievements[0].achievement_type,
            table.state.achievements[1].achievement_type,
            table.state.achievements[2].achievement_type,
        ]),
        port_a: Some(std::array::from_fn(|i| table.state.port_state.port_a.slots[i].unwrap())),
        port_b: Some(std::array::from_fn(|i| {
            table.state.port_state.port_b.as_ref().unwrap().slots[i].unwrap()
        })),
        lab_a: Some(std::array::from_fn(|i| table.state.lab_state.lab_a.slots[i].unwrap())),
        lab_b: None,
        deals: vec![(yokohama_core::PlayerId::new(0), table.deals[0])],
    };
    let choices = vec![
        StartingOrderChoice::Keep(1),
        StartingOrderChoice::Unknown,
        StartingOrderChoice::Unknown,
    ];
    let mut record = GameRecord::new(31, config.clone(), choices.clone());
    record.setup_observations = Some(obs.clone());

    let pending = new_game(config, &rules, 31).unwrap();
    let pending = yokohama_core::reconcile_setup(pending, &obs, &rules).unwrap();
    let mut state = finalize_setup(pending, &choices).unwrap();
    check_invariants(&state, &rules).unwrap();

    let mut rng = ChaCha8Rng::seed_from_u64(31);
    for _ in 0..60 {
        if is_game_over(&state) {
            break;
        }
        let moves = get_legal_moves(&state, &rules);
        let mv = moves[rng.gen_range(0..moves.len())].clone();
        // Live twin: identity reveals for any draws this move makes.
        let scratch = apply_move(&state, &mv, &rules).unwrap();
        let d_o = state.order_deck.len() - scratch.order_deck.len();
        let d_t = state.tech_deck.len() - scratch.tech_deck.len();
        if d_o > 0 {
            let cards: Vec<_> = state.order_deck.iter().rev().take(d_o).copied().collect();
            record.push_event(RecordedEvent::RevealOrders(cards));
        }
        if d_t > 0 {
            let cards: Vec<_> = state.tech_deck.iter().rev().take(d_t).copied().collect();
            record.push_event(RecordedEvent::RevealTechs(cards));
        }
        state = apply_move(&state, &mv, &rules).unwrap();
        check_invariants(&state, &rules).unwrap();
        record.push_move(mv);
    }
    assert_eq!(replay(&record, &rules).unwrap(), state);
}
```

- [ ] **Step 2: Run**: `cargo test -p yokohama-core --test live_reconciliation`
Expected: all PASS (a failure indicts Tasks 1–6 — fix there, not here). Then the full suite: `cargo test -p yokohama-core` green, zero warnings.

- [ ] **Step 3: Commit**: `git add crates/yokohama-core/tests/live_reconciliation.rs && git commit -m "test: reveal-equivalence, permuted-reveal, hidden-order, and live-setup replay properties"`

---

### Task 8: `yokohama-cli` crate + `describe_move`

**Files:**
- Modify: `Cargo.toml` (workspace `members` += `"crates/yokohama-cli"`)
- Create: `crates/yokohama-cli/Cargo.toml`, `crates/yokohama-cli/src/lib.rs`, `crates/yokohama-cli/src/describe.rs`

**Interfaces:**
- Produces:
  - `describe::area_label(state: &GameState, area: AreaId) -> String` — `"Copper Mine (A03)"` (type plain-name + zero-padded id).
  - `describe::describe_move(mv: &Move, state: &GameState, rules: &Rules) -> String` — one plain-words line per Move/AreaAction variant (the CLI's option text).
  - `describe::area_type_name(t: AreaType) -> &'static str`, `describe::country_name(c: Country) -> &'static str`, `describe::achievement_desc(a: AchievementType) -> String`, `describe::phase_name(p: TurnPhase) -> &'static str` (plain words: "Additional actions (before)", "Placement", "Movement", "Area action", "5-Power bonus", "Building", "Recovery", "Additional actions (after)", "Turn end").

**Crate manifest:**

```toml
[package]
name = "yokohama-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
yokohama-core = { path = "../yokohama-core" }
serde.workspace = true
serde_json.workspace = true
```

`src/lib.rs` starts as:

```rust
//! Phase-2 CLI game driver: Session/REPL/wizard over yokohama-core
//! (design docs/plans/2026-07-18-phase2-cli-driver-design.md).

pub mod describe;
```

- [ ] **Step 1: Failing tests** (bottom of `describe.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_core::*;

    fn state_2p() -> GameState {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 1).unwrap();
        finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]).unwrap()
    }

    #[test]
    fn area_label_names_type_and_id() {
        let state = state_2p();
        let some_area = *state.board_layout.areas.keys().next().unwrap();
        let label = area_label(&state, some_area);
        assert!(label.contains(&format!("(A{:02})", some_area.0)), "{label}");
    }

    #[test]
    fn placement_moves_read_as_plain_words() {
        let state = state_2p();
        let a3 = AreaId::new(3);
        let a5 = AreaId::new(5);
        let rules = Rules::standard();
        let none = describe_move(&Move::PlaceThreeAreas { placements: vec![] }, &state, &rules);
        assert_eq!(none, "Place no assistants");
        let two = describe_move(&Move::PlaceTwoSameArea { area: a3, adjacent_bonus: Some(a5) }, &state, &rules);
        assert!(two.starts_with("Place 2 assistants in "), "{two}");
        assert!(two.contains("Telegram"), "{two}");
    }

    #[test]
    fn area_actions_read_as_plain_words() {
        let state = state_2p();
        let rules = Rules::standard();
        assert_eq!(
            describe_move(&Move::ExecuteAreaAction { action: AreaAction::ProduceCopper }, &state, &rules),
            "Produce copper"
        );
        assert_eq!(
            describe_move(&Move::ExecuteAreaAction { action: AreaAction::Customs { imported_goods_paid: 2 } }, &state, &rules),
            "Customs: pay 2 imported goods"
        );
        let skip = describe_move(&Move::ExecuteAreaAction { action: AreaAction::Skip }, &state, &rules);
        assert!(skip.contains("declin"), "{skip}");
        let ch = describe_move(
            &Move::ExecuteAreaAction {
                action: AreaAction::Chinatown { exchanges: vec![ChinatownExchange::CopperToYen] },
            },
            &state,
            &rules,
        );
        assert!(ch.contains("1 copper -> 2 yen"), "{ch}");
    }

    #[test]
    fn every_phase_has_a_plain_name() {
        for p in [
            TurnPhase::AdditionalActionFirst, TurnPhase::Placement, TurnPhase::Movement,
            TurnPhase::AreaAction, TurnPhase::FivePowerBonus, TurnPhase::Building,
            TurnPhase::Recovery, TurnPhase::AdditionalActionSecond, TurnPhase::TurnEnd,
        ] {
            assert!(!phase_name(p).is_empty());
        }
    }
}
```

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-cli` → compile errors (crate is new — first add it to the workspace `members`, create the manifest and `lib.rs`, then the missing-fn errors are the failing state).

- [ ] **Step 3: Implement `describe.rs`** — complete match coverage; core shape:

```rust
use yokohama_core::*;

pub fn area_type_name(t: AreaType) -> &'static str {
    match t {
        AreaType::CopperMine => "Copper Mine",
        AreaType::SilkMill => "Silk Mill",
        AreaType::TeaPlantation => "Tea Plantation",
        AreaType::FishingGround => "Fishing Ground",
        AreaType::Bank => "Bank",
        AreaType::PortA => "Port A",
        AreaType::PortB => "Port B",
        AreaType::LaboratoryA => "Laboratory A",
        AreaType::LaboratoryB => "Laboratory B",
        AreaType::EmploymentAgency => "Employment Agency",
        AreaType::Chinatown => "Chinatown",
        AreaType::Church => "Church",
        AreaType::Customs => "Customs",
        AreaType::Canal => "Canal",
    }
}

pub fn country_name(c: Country) -> &'static str {
    match c {
        Country::English => "Britain",
        Country::American => "America",
        Country::French => "France",
        Country::German => "Germany",
        Country::Dutch => "Netherlands",
    }
}

pub fn phase_name(p: TurnPhase) -> &'static str {
    match p {
        TurnPhase::AdditionalActionFirst => "Additional actions (before)",
        TurnPhase::Placement => "Placement",
        TurnPhase::Movement => "Movement",
        TurnPhase::AreaAction => "Area action",
        TurnPhase::FivePowerBonus => "5-Power bonus",
        TurnPhase::Building => "Building",
        TurnPhase::Recovery => "Recovery",
        TurnPhase::AdditionalActionSecond => "Additional actions (after)",
        TurnPhase::TurnEnd => "Turn end",
    }
}

pub fn area_label(state: &GameState, area: AreaId) -> String {
    let name = state
        .board_layout
        .areas
        .get(&area)
        .map(|a| area_type_name(a.area_type))
        .unwrap_or("Unknown area");
    format!("{name} (A{:02})", area.0)
}
```

`achievement_desc`: 12-arm match, e.g. `AchievementType::Copper5 => "Own 5 Copper".to_string()`, `Money10 => "Own 10 Yen"`, `Tokens5 => "Own 5 tokens (5-Power + mercenaries)"`, `Orders5 => "Complete 5 orders"`, `Technologies4 => "Own 4 technologies"`, `Production4 => "Buildings in 4 production areas"`, `Commercial4 => "Buildings in 4 commercial areas"`, `Prod3Comm2 => "Buildings in 3 production + 2 commercial areas"`, `Prod2Comm3 => "Buildings in 2 production + 3 commercial areas"`, `Silk6 => "Own 6 Silk"`, `Tea7 => "Own 7 Tea"`, `Seafood7 => "Own 7 Seafood"`.

`describe_move`: full match on `Move` (all 15 variants) and `AreaAction` (all 10):

```rust
pub fn describe_move(mv: &Move, state: &GameState, rules: &Rules) -> String {
    match mv {
        Move::UseMercenary { country, target_area } => {
            format!("Use a {} mercenary at {}", country_name(*country), area_label(state, *target_area))
        }
        Move::FulfillAchievement { achievement } => {
            format!("Claim achievement: {}", achievement_desc(*achievement))
        }
        Move::FulfillOrder { order_id } => match rules.orders.get(*order_id) {
            Some(card) => format!("Fulfill {}", order_prompt_line(card)),
            None => format!("Fulfill order O{:02}", order_id.0),
        },
        Move::EndAdditionalPhase => "End additional-action phase".to_string(),
        Move::PlaceThreeAreas { placements } if placements.is_empty() => "Place no assistants".to_string(),
        Move::PlaceThreeAreas { placements } => format!(
            "Place 1 assistant in each of: {}",
            placements.iter().map(|a| area_label(state, *a)).collect::<Vec<_>>().join(", ")
        ),
        Move::PlaceTwoSameArea { area, adjacent_bonus } => {
            let mut s = format!("Place 2 assistants in {}", area_label(state, *area));
            if let Some(adj) = adjacent_bonus {
                s.push_str(&format!(" and 1 in {} (Telegram)", area_label(state, *adj)));
            }
            s
        }
        Move::MovePresident { path, destination } if path.is_empty() => {
            format!("Move President directly to {} (free teleport)", area_label(state, *destination))
        }
        Move::MovePresident { path, destination } => format!(
            "Move President to {} via {}",
            area_label(state, *destination),
            path.iter().map(|a| area_label(state, *a)).collect::<Vec<_>>().join(" -> ")
        ),
        Move::ReturnPresidentToHand { recover_assistants_from } if recover_assistants_from.is_empty() => {
            "Return President to hand (recover no assistants)".to_string()
        }
        Move::ReturnPresidentToHand { recover_assistants_from } => format!(
            "Return President to hand, recovering assistants from {}",
            recover_assistants_from.iter().map(|a| area_label(state, *a)).collect::<Vec<_>>().join(", ")
        ),
        Move::PlacePresident { destination } => {
            format!("Place President from hand in {}", area_label(state, *destination))
        }
        Move::ExecuteAreaAction { action } => describe_area_action(action, state, rules),
        Move::Take5PowerToken => "Take the 5-Power token".to_string(),
        Move::Skip5PowerBonus => "Skip the 5-Power token".to_string(),
        Move::BuildShophouse { slot } => format!("Build a Shophouse (space {})", slot + 1),
        Move::BuildTradingHouse => "Build a Trading House".to_string(),
        Move::SkipBuilding => "Skip building".to_string(),
    }
}
```

`describe_area_action`: `ProduceCopper/Silk/Tea/Seafood` → `"Produce copper"` etc.; `Bank` → `"Take money at the Bank"`; `Port { cards_taken, second_card_payment }` → `"Port: take O07, O21"` + `", paying 2 yen"` / `", paying 1 imported good"` when `Some(TwoYen)`/`Some(OneImportedGood)`; `Laboratory { card_id, extra_yen, extra_imported, fifth_tech_payment }` → `"Laboratory: acquire <name from rules.techs.get>"` + `", +N yen"`/`", +N imported"` when nonzero + `", 5th-tech fee: 2 yen"`/`"...: 1 imported good"`; `EmploymentAgency { assistants, shophouses, trading_houses }` → `"Employment Agency: take "` + comma list of the nonzero parts (`"2 assistants"`, `"1 shophouse"`, `"1 trading house"`); `Chinatown { exchanges }` → `"Chinatown: "` + comma list mapping each `ChinatownExchange` variant to its trade text (`CopperToYen => "1 copper -> 2 yen"`, `SilkToYen => "1 silk -> 2 yen"`, `TeaToYen => "1 tea -> 1 yen"`, `SeafoodToYen => "1 fish -> 1 yen"`, `YenToCopper => "2 yen -> 1 copper"`, `YenToSilk => "2 yen -> 1 silk"`, `YenToTea => "1 yen -> 1 tea"`, `YenToSeafood => "1 yen -> 1 fish"`, `YenToImported => "4 yen -> 1 imported good"`); `Church { .. }` → `"Church: pay "` + list of the true flags (`copper/silk/tea/fish/yen/imported good`) or `"Church: pay nothing (faith from power only)"` when all false; `Customs { imported_goods_paid }` → `"Customs: pay N imported goods"` (singular "good" when 1); `Skip` → `"Take the area action, declining all optional parts"`.

- [ ] **Step 4: Run**: `cargo test -p yokohama-cli` PASS; `cargo test -p yokohama-core` still green; zero warnings.

- [ ] **Step 5: Commit**: `git add Cargo.toml crates/yokohama-cli && git commit -m "feat: yokohama-cli crate with plain-words describe_move over the full Move surface"`

---

### Task 9: Render module

**Files:**
- Create: `crates/yokohama-cli/src/render.rs`
- Modify: `crates/yokohama-cli/src/lib.rs` (`pub mod render;`)

**Interfaces:**
- Consumes: `describe::*`, core display helpers (`order_compact_line`, `site_compact_line`), registries.
- Produces (all `pub fn ... -> String` unless noted):
  - `prompt_header(state: &GameState) -> String` — one line: `"Turn 12 | Red | Placement | 5 yen 3 VP | c1 s2 t0 f1 i0 | hand 2"`.
  - `board_grid(state: &GameState) -> String` — the grid, one bordered cell per area: id+abbrev line, presidents line (`P: R`), assistants line (`R2 B1`), buildings line (`sh RB | th Y`), 5-power marker (`*` while unclaimed).
  - `ports_view(state: &GameState, rules: &Rules) -> String` — per board, slot number 1–6 + zone (`Low/Mid/High`) + `order_compact_line` (or `-` when empty), then `"order deck: N"`.
  - `labs_view(state: &GameState, rules: &Rules) -> String` — slots 1–6 with surcharge (`+0/+1/+2`), tech name + industry cost, then `"tech deck: N"`.
  - `players_panel(state: &GameState, rules: &Rules) -> String` — per player: color, money, VP, goods, imports, hand order codes, completed count, tech names, tokens/mercenaries, president location.
  - `tracks_view(state: &GameState) -> String` — church spaces (faith values, occupant color or `-`), customs spaces (cost, occupant or `-`, `X` on 2p dummy-blocked spaces).
  - `delta_summary(before: &GameState, after: &GameState, rules: &Rules) -> Vec<String>` — human lines for: per-player money/VP/goods/imports diffs (`"Red: +2 copper, -1 yen"`), hand gains/losses by order code, tech gains by name, president relocation, phase transition (`"Phase: Placement -> Movement"`), turn/player handoff, deck-size drops (`"order deck 16 -> 14"`), end-trigger announcement when `after.end_game_triggered && !before.end_game_triggered`.
- Rendering rule: assert **content, not screenshots** in tests (substring/line-count assertions), so cosmetic polish during dogfooding doesn't churn tests.

- [ ] **Step 1: Failing tests** (bottom of `render.rs`; seeded 2p state as in Task 8's `state_2p`, shared via a small `#[cfg(test)] mod` helper local to this file):

```rust
#[test]
fn prompt_header_shows_turn_color_phase_and_money() {
    let state = test_state_2p();
    let h = prompt_header(&state);
    assert!(h.contains("Turn 1"), "{h}");
    assert!(h.contains("Red"), "{h}");
    assert!(h.contains("Additional actions (before)"), "{h}");
    assert!(h.contains("3 yen"), "{h}");
}

#[test]
fn board_grid_has_one_cell_per_area_with_ids() {
    let state = test_state_2p();
    let g = board_grid(&state);
    for id in state.board_layout.areas.keys() {
        assert!(g.contains(&format!("A{:02}", id.0)), "missing A{:02} in\n{g}", id.0);
    }
}

#[test]
fn ports_view_lists_six_slots_and_deck_count() {
    let state = test_state_2p();
    let rules = Rules::standard();
    let v = ports_view(&state, &rules);
    assert!(v.contains("Port A"), "{v}");
    for zone in ["Low", "Mid", "High"] {
        assert!(v.contains(zone), "{v}");
    }
    assert!(v.contains(&format!("order deck: {}", state.order_deck.len())), "{v}");
}

#[test]
fn delta_summary_reports_a_production_gain() {
    let rules = Rules::standard();
    let state = test_state_2p();
    // Drive to the first Placement decision, then walk a fixed legal path:
    // EndAdditionalPhase is always legal in AdditionalActionFirst.
    let s1 = apply_move(&state, &Move::EndAdditionalPhase, &rules).unwrap();
    let lines = delta_summary(&state, &s1, &rules);
    assert!(lines.iter().any(|l| l.contains("Placement")), "{lines:?}");
}
```

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-cli render` → compile errors.

- [ ] **Step 3: Implement.** Straightforward string building; sizes: `board_grid` builds `rows × cols` cells of fixed width 16 from `grid_position` (cells collected into a `Vec<Vec<String>>` of per-cell lines, then joined row by row with `|` separators and `-` rulers); assistants line concatenates per-player `"{color_letter}{count}"` for counts > 0 (color letter = first char of color name); presidents from each player's `PresidentLocation::OnBoard(a)`; buildings read `building_sites[area].shophouses_built` / `trading_house_owner`. `delta_summary` compares fields directly (players zipped by index; goods compared per resource with signed diffs; hand diffs via id-set difference rendered as `O{:02}`; deck sizes; `turn_phase`; `current_player`; `end_game_triggered` edge announces `format!("END TRIGGERED: {:?}", after.trigger_reason)`). Keep every helper private except the six listed functions.

- [ ] **Step 4: Run**: `cargo test -p yokohama-cli` PASS, zero warnings.

- [ ] **Step 5: Commit**: `git add crates/yokohama-cli/src/render.rs crates/yokohama-cli/src/lib.rs && git commit -m "feat: ASCII render module — header, board grid, ports/labs, players, tracks, delta summary"`

---

### Task 10: `Session` (simulation core) + full-game driver test

**Files:**
- Create: `crates/yokohama-cli/src/session.rs`
- Modify: `crates/yokohama-cli/src/lib.rs` (`pub mod session;`)

**Interfaces:**
- Consumes: `GameRecord`/`RecordedEvent`/`replay`, `reconcile_setup`, engine loop functions, `render::delta_summary`.
- Produces:

```rust
pub enum TrackingMode { Simulation, Live }

pub struct Session {
    rules: Rules,
    record: GameRecord,
    state: GameState,
    pub mode: TrackingMode,
    pub save_path: Option<std::path::PathBuf>,
}

impl Session {
    pub fn new_sim(players: u8, seed: u64, choices: Vec<StartingOrderChoice>, rules: Rules) -> Result<Session, String>;
    pub fn new_live(config: GameConfig, seed: u64, obs: SetupObservations,
                    choices: Vec<StartingOrderChoice>, rules: Rules) -> Result<Session, String>;
    pub fn load(path: &std::path::Path, rules: Rules) -> Result<Session, String>; // mode = Live iff setup_observations.is_some()
    pub fn save(&self, path: &std::path::Path) -> Result<(), String>;             // pretty GameRecord JSON
    pub fn state(&self) -> &GameState;
    pub fn rules(&self) -> &Rules;
    pub fn record(&self) -> &GameRecord;
    pub fn legal(&self) -> Vec<Move>;
    pub fn is_over(&self) -> bool;
    /// Apply reveals+move as ONE atomic group: reveals fold first, then the
    /// move; invariants checked; on any error nothing is recorded/committed.
    /// Returns delta_summary lines. Autosaves when save_path is set.
    pub fn apply(&mut self, mv: Move, reveals: Vec<RecordedEvent>) -> Result<Vec<String>, String>;
    /// Apply a standalone reveal event (RevealStartingOrder) atomically.
    pub fn apply_standalone(&mut self, ev: RecordedEvent) -> Result<Vec<String>, String>;
    /// Replay-prefix undo of the last group (design §5.4): a trailing Move
    /// takes its contiguous preceding bound reveals with it; a trailing
    /// standalone reveal pops alone. Err when the log is empty.
    pub fn undo(&mut self) -> Result<(), String>;
    pub fn final_scores(&self) -> Vec<(PlayerId, u32)>; // calculate_final_scores(state, &rules.orders, &rules.techs)
}
```

- [ ] **Step 1: Failing tests** (bottom of `session.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_core::*;

    fn sim_2p(seed: u64) -> Session {
        Session::new_sim(
            2, seed,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        ).unwrap()
    }

    #[test]
    fn apply_then_undo_restores_previous_state() {
        let mut s = sim_2p(4);
        let s0 = s.state().clone();
        let mv = s.legal()[0].clone();
        let delta = s.apply(mv, vec![]).unwrap();
        assert!(!delta.is_empty());
        assert_ne!(*s.state(), s0);
        s.undo().unwrap();
        assert_eq!(*s.state(), s0);
        assert!(s.record().events.is_empty());
        assert!(s.undo().is_err()); // nothing left
    }

    #[test]
    fn illegal_move_is_rejected_atomically() {
        let mut s = sim_2p(4);
        let before_state = s.state().clone();
        let before_events = s.record().events.len();
        // A Building-phase move is never legal in AdditionalActionFirst.
        assert!(s.apply(Move::SkipBuilding, vec![]).is_err());
        assert_eq!(*s.state(), before_state);
        assert_eq!(s.record().events.len(), before_events);
    }

    #[test]
    fn save_load_round_trips_state_and_mode() {
        let mut s = sim_2p(4);
        for _ in 0..5 {
            let mv = s.legal()[0].clone();
            s.apply(mv, vec![]).unwrap();
        }
        let path = std::env::temp_dir().join(format!("yokohama_test_{}.json", std::process::id()));
        s.save(&path).unwrap();
        let loaded = Session::load(&path, Rules::standard()).unwrap();
        assert_eq!(loaded.state(), s.state());
        assert!(matches!(loaded.mode, TrackingMode::Simulation));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn autosave_writes_after_each_apply() {
        let mut s = sim_2p(4);
        let path = std::env::temp_dir().join(format!("yokohama_autosave_{}.json", std::process::id()));
        s.save_path = Some(path.clone());
        let mv = s.legal()[0].clone();
        s.apply(mv, vec![]).unwrap();
        assert!(path.exists());
        let loaded = Session::load(&path, Rules::standard()).unwrap();
        assert_eq!(loaded.state(), s.state());
        std::fs::remove_file(&path).ok();
    }

    /// The CLI-layer analog of the fuzz driver: a full random game driven
    /// entirely through Session::apply completes and its record replays.
    #[test]
    fn full_random_game_through_session_completes_and_replays() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(0);
        let mut s = sim_2p(0);
        let mut applied = 0u32;
        while !s.is_over() {
            assert!(s.state().turn_number <= 2000, "non-termination");
            let legal = s.legal();
            assert!(!legal.is_empty(), "dead end at turn {}", s.state().turn_number);
            let mv = legal[rng.gen_range(0..legal.len())].clone();
            s.apply(mv, vec![]).unwrap();
            applied += 1;
        }
        assert!(applied > 100);
        assert_eq!(replay(s.record(), s.rules()).unwrap(), *s.state());
        assert!(!s.final_scores().is_empty());
    }
}
```

(`rand`/`rand_chacha` become `[dev-dependencies]` of `yokohama-cli`: `rand.workspace = true`, `rand_chacha.workspace = true`.)

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-cli session` → compile errors.

- [ ] **Step 3: Implement.** Key bodies:

```rust
pub fn new_sim(players: u8, seed: u64, choices: Vec<StartingOrderChoice>, rules: Rules) -> Result<Session, String> {
    let config = GameConfig::for_player_count(players);
    let pending = new_game(config.clone(), &rules, seed).map_err(|e| e.to_string())?;
    let state = finalize_setup(pending, &choices).map_err(|e| e.to_string())?;
    let record = GameRecord::new(seed, config, choices);
    Ok(Session { rules, record, state, mode: TrackingMode::Simulation, save_path: None })
}

pub fn new_live(config: GameConfig, seed: u64, obs: SetupObservations, choices: Vec<StartingOrderChoice>, rules: Rules) -> Result<Session, String> {
    let pending = new_game(config.clone(), &rules, seed).map_err(|e| e.to_string())?;
    let pending = reconcile_setup(pending, &obs, &rules).map_err(|e| e.to_string())?;
    let state = finalize_setup(pending, &choices).map_err(|e| e.to_string())?;
    let mut record = GameRecord::new(seed, config, choices);
    record.setup_observations = Some(obs);
    Ok(Session { rules, record, state, mode: TrackingMode::Live, save_path: None })
}

pub fn apply(&mut self, mv: Move, reveals: Vec<RecordedEvent>) -> Result<Vec<String>, String> {
    let mut staged = self.state.clone();
    for ev in &reveals {
        match ev {
            RecordedEvent::RevealOrders(c) => staged.arrange_orders_on_top(c).map_err(|e| e.to_string())?,
            RecordedEvent::RevealTechs(c) => staged.arrange_techs_on_top(c).map_err(|e| e.to_string())?,
            RecordedEvent::RevealFivePowerToken { area, effect } => {
                staged.reveal_five_power_token(*area, *effect).map_err(|e| e.to_string())?
            }
            other => return Err(format!("event not allowed as a bound reveal: {other:?}")),
        }
    }
    let next = apply_move(&staged, &mv, &self.rules).map_err(|e| e.to_string())?;
    check_invariants(&next, &self.rules)
        .map_err(|e| format!("INVARIANT VIOLATION after apply (driver/engine bug): {e}"))?;
    let delta = crate::render::delta_summary(&self.state, &next, &self.rules);
    for ev in reveals {
        self.record.push_event(ev);
    }
    self.record.push_move(mv);
    self.state = next;
    self.autosave();
    Ok(delta)
}
```

`apply_standalone` mirrors it for `RevealStartingOrder` (staged clone → `reveal_starting_order` → invariants → push → commit → autosave; reject other variants). `undo`: pop trailing group off a cloned `events` (`Move` pops preceding contiguous `RevealOrders|RevealTechs|RevealFivePowerToken`; a trailing bound-reveal without a following move — possible only mid-crash — pops alone, as does `RevealStartingOrder`), rebuild `record`, `self.state = replay(&self.record, &self.rules)?`. `autosave`: `if let Some(p) = &self.save_path { let _ = self.save(p); }` — deliberate best-effort with the error surfaced by `save` when invoked explicitly; wait — silent-failure rule: autosave failures must be VISIBLE. Return them: make `autosave(&self) -> Option<String>` and have `apply`/`undo` append `"WARNING: autosave failed: .."` to the returned delta lines instead of swallowing. `save`: `std::fs::write(path, serde_json::to_string_pretty(&self.record))`. `load`: read + parse + `replay` + final `check_invariants` + mode inference.

- [ ] **Step 4: Run**: `cargo test -p yokohama-cli` (the full-game test runs in debug — expect a few seconds) then `cargo test -p yokohama-core`; zero warnings.

- [ ] **Step 5: Commit**: `git add crates/yokohama-cli && git commit -m "feat: Session — atomic apply/undo/save/load over GameRecord with invariant guard"`

---

### Task 11: REPL step + input parsing (simulation path)

**Files:**
- Create: `crates/yokohama-cli/src/repl.rs`
- Modify: `crates/yokohama-cli/src/lib.rs` (`pub mod repl;`)

**Interfaces:**
- Produces:

```rust
pub struct Repl {
    pub session: crate::session::Session,
    page: usize,
    pending: Pending,      // live-mode reveal collection state (Task 12); Pending::None in sim
    locked: bool,          // set on invariant-violation/desync; only save/log/quit allowed
}

enum Pending { None, /* Task 12 adds reveal-collection variants */ }

pub struct StepOutput { pub text: String, pub quit: bool }

impl Repl {
    pub fn new(session: crate::session::Session) -> Repl;
    /// The whole prompt screen for the current state: header, delta-free
    /// status, options page, input hint.
    pub fn prompt(&self) -> String;
    /// One REPL interaction: parse `line`, act, return output text (which
    /// ends with the next prompt unless quitting).
    pub fn step(&mut self, line: &str) -> StepOutput;
}
```

- Input grammar (sim path):
  - integer `k` → apply `legal()[k]` (global index, 0-based as displayed);
  - `a [id id id]` → `Move::PlaceThreeAreas` (0–3 area ids), `b <id>` / `b <id> t <adj>` → `Move::PlaceTwoSameArea` — both matched against the legal list **by placement-multiset equality** (the generator's internal ordering must not matter);
  - commands: `board`, `players`, `ports`, `labs`, `tracks`, `hand`, `score`, `log`, `more`, `list`, `undo`, `save [path]`, `help`, `quit`;
  - anything else → `"Unrecognized input — type 'help'"` + prompt.
- Options pagination: 20 per page; `more` advances (wraps to 0); `list` resets to page 0; page resets after every successful apply/undo.
- `score` prints current VP + money per player; when `is_over`, prints `final_scores()` as the authoritative table.
- On `is_over` becoming true after an apply, output includes final scores and the prompt degrades to `game over — commands only`.

- [ ] **Step 1: Failing tests** (bottom of `repl.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;
    use yokohama_core::*;

    fn repl_2p(seed: u64) -> Repl {
        Repl::new(Session::new_sim(
            2, seed,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        ).unwrap())
    }

    #[test]
    fn prompt_numbers_the_legal_options() {
        let r = repl_2p(4);
        let p = r.prompt();
        assert!(p.contains("[0]"), "{p}");
        assert!(p.contains("End additional-action phase"), "{p}");
    }

    #[test]
    fn numeric_input_applies_that_option_and_reprompts() {
        let mut r = repl_2p(4);
        let phase_before = r.session.state().turn_phase;
        let out = r.step("0");
        assert!(!out.quit);
        assert_ne!(r.session.state().turn_phase, phase_before);
        assert!(out.text.contains("Placement"), "{}", out.text); // delta line + new prompt
    }

    #[test]
    fn placement_grammar_matches_by_multiset() {
        let mut r = repl_2p(4);
        r.step("0"); // -> Placement
        let legal = r.session.legal();
        let some_a = legal.iter().find_map(|m| match m {
            Move::PlaceThreeAreas { placements } if placements.len() == 2 => Some(placements.clone()),
            _ => None,
        }).expect("a 2-area Option A exists");
        // Enter the two areas in REVERSED order — must still match.
        let line = format!("a {} {}", some_a[1].0, some_a[0].0);
        let events_before = r.session.record().events.len();
        r.step(&line);
        assert_eq!(r.session.record().events.len(), events_before + 1);
    }

    #[test]
    fn bad_input_and_illegal_grammar_are_rejected_gently() {
        let mut r = repl_2p(4);
        let out = r.step("banana");
        assert!(out.text.contains("Unrecognized"), "{}", out.text);
        let out = r.step("a 99"); // AreaId out of range
        assert!(out.text.to_lowercase().contains("area"), "{}", out.text);
        let out = r.step("999"); // option index out of range
        assert!(out.text.contains("out of range"), "{}", out.text);
    }

    #[test]
    fn undo_save_quit_commands_work() {
        let mut r = repl_2p(4);
        r.step("0");
        let out = r.step("undo");
        assert!(out.text.contains("Undid"), "{}", out.text);
        assert!(r.session.record().events.is_empty());
        let path = std::env::temp_dir().join(format!("yokohama_repl_{}.json", std::process::id()));
        let out = r.step(&format!("save {}", path.display()));
        assert!(out.text.contains("Saved"), "{}", out.text);
        assert!(path.exists());
        std::fs::remove_file(&path).ok();
        assert!(r.step("quit").quit);
    }

    #[test]
    fn pagination_pages_through_large_placement_lists() {
        let mut r = repl_2p(4);
        r.step("0");
        let n = r.session.legal().len();
        if n > 20 {
            let p0 = r.prompt();
            assert!(p0.contains("more"), "{p0}");
            let p1 = r.step("more").text;
            assert_ne!(p0, p1);
        }
    }
}
```

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-cli repl` → compile errors.

- [ ] **Step 3: Implement.** Parsing sketch:

```rust
pub fn step(&mut self, line: &str) -> StepOutput {
    let line = line.trim();
    if self.locked && !matches!(line, "save" | "log" | "quit") && !line.starts_with("save ") {
        return self.out("session locked after a driver/engine fault — only save/log/quit".into(), false);
    }
    let words: Vec<&str> = line.split_whitespace().collect();
    match words.as_slice() {
        [] => self.out(String::new(), false),
        ["quit"] | ["q"] => StepOutput { text: "bye\n".into(), quit: true },
        ["help"] => self.out(HELP_TEXT.into(), false),
        ["board"] => self.out(crate::render::board_grid(self.session.state()), false),
        ["players"] => self.out(crate::render::players_panel(self.session.state(), self.session.rules()), false),
        ["ports"] => self.out(crate::render::ports_view(self.session.state(), self.session.rules()), false),
        ["labs"] => self.out(crate::render::labs_view(self.session.state(), self.session.rules()), false),
        ["tracks"] => self.out(crate::render::tracks_view(self.session.state()), false),
        ["hand"] => self.out(self.hand_text(), false),
        ["score"] => self.out(self.score_text(), false),
        ["log"] => self.out(self.log_tail(), false),
        ["more"] => { self.page += 1; self.out(String::new(), false) }
        ["list"] => { self.page = 0; self.out(String::new(), false) }
        ["undo"] => match self.session.undo() {
            Ok(()) => { self.page = 0; self.out("Undid last entry.\n".into(), false) }
            Err(e) => self.out(format!("Cannot undo: {e}\n"), false),
        },
        ["save"] => self.save_cmd(None),
        ["save", path] => self.save_cmd(Some(path)),
        ["a", rest @ ..] => self.try_placement_a(rest),
        ["b", rest @ ..] => self.try_placement_b(rest),
        [n] if n.chars().all(|c| c.is_ascii_digit()) => self.pick_option(n.parse::<usize>().unwrap()),
        _ => self.out("Unrecognized input — type 'help'.\n".into(), false),
    }
}
```

`out(text, quit)` appends `self.prompt()` (unless quitting/locked) and returns. `pick_option(k)`: bounds-check against `legal()` (`"option {k} out of range (0..{n})"`), then `self.commit(mv)`. `try_placement_a/b`: parse each token via `AreaId::try_new(tok.parse::<u8>()...)` with friendly errors (`"'99' is not a valid area id (0-17)"`); build the candidate; find the legal counterpart by multiset equality:

```rust
fn find_placement_match(&self, candidate: &Move) -> Option<Move> {
    let key = |v: &Vec<AreaId>| { let mut k: Vec<u8> = v.iter().map(|a| a.0).collect(); k.sort_unstable(); k };
    self.session.legal().into_iter().find(|m| match (m, candidate) {
        (Move::PlaceThreeAreas { placements: a }, Move::PlaceThreeAreas { placements: b }) => key(a) == key(b),
        (m, c) => m == c,
    })
}
```

`commit(mv)`: sim path calls `self.session.apply(mv, vec![])` (Task 12 reroutes through reveal detection); on `Ok(delta)` reset page, join delta lines + (if now `is_over`) final-score table; on `Err(e)` — if the message starts with `INVARIANT VIOLATION` set `self.locked = true` and print the lockdown banner + seed/turn/log-tail dump; otherwise print `Illegal: {e}`. `HELP_TEXT` documents every command + the two placement forms.

- [ ] **Step 4: Run**: `cargo test -p yokohama-cli` PASS; zero warnings.

- [ ] **Step 5: Commit**: `git add crates/yokohama-cli && git commit -m "feat: REPL step — numbered options, placement grammar, commands, pagination"`

---

### Task 12: Live mode — reveal choreography

**Files:**
- Modify: `crates/yokohama-cli/src/repl.rs` (Pending state machine + reveal command + live commit path)
- Modify: `crates/yokohama-cli/src/session.rs` only if a helper is missing (interfaces exist from Task 10)

**Interfaces:**
- Consumes: `Session::{apply, apply_standalone}`, `TrackingMode`, Task 1–3 semantics.
- Produces (all inside `Repl`):
  - Live commit path: on selecting a move in `TrackingMode::Live`, scratch-apply (`apply_move(self.session.state(), &mv, rules)`) to diff draws and, when they exist, enter `Pending::OrderReveals { mv, slots, board_b, collected }` / `Pending::TechReveal { mv, board_b }`; for `Move::Take5PowerToken`, enter `Pending::TokenReveal { mv, area }` (area = the diffed `Some -> None` key of `five_power_tokens`). Prompts collect one card/effect per line; a complete collection calls `Session::apply(mv, reveals)`; `cancel` aborts collection.
  - Refill positions/order: order refill slots = indices where `scratch` board slot content ≠ current board slot content **and** the scratch occupant came from the deck; prompt in **descending slot order** (pop order, `refill_port` fills 5→0); label with 1-based slot + zone name. Tech reveal: the single changed lab slot.
  - Order-card input: `O17` or `17`; must be in `order_deck` and not already collected. Tech input: `T<id>` or a case-insensitive unique name prefix; resolves to the lowest matching id present in `tech_deck`.
  - Token input: `i`/`c`/`s`/`t`/`f`/`y` → `ImportedGoods(1)/Copper(2)/Silk(2)/Tea(3)/Seafood(3)/Money(3)` (the canonical pool's six kinds).
  - `reveal <seat> <O..>` command → `Session::apply_standalone(RecordedEvent::RevealStartingOrder{..})`.
  - Post-apply desync assertion: after a live apply with order reveals, the named board slots must hold exactly the typed cards; mismatch sets `locked` with a CRITICAL banner.

- [ ] **Step 1: Failing tests** (in `repl.rs` tests; build a live session whose "physical table" is another seed, as in Task 7):

```rust
fn live_repl() -> (Repl, Vec<OrderCardId>) {
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(2);
    let table = new_game(config.clone(), &rules, 900).unwrap();
    let obs = SetupObservations {
        port_a: Some(std::array::from_fn(|i| table.state.port_state.port_a.slots[i].unwrap())),
        lab_a: Some(std::array::from_fn(|i| table.state.lab_state.lab_a.slots[i].unwrap())),
        deals: vec![(PlayerId::new(0), table.deals[0])],
        ..Default::default()
    };
    let session = Session::new_live(
        config, 901, obs,
        vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown],
        rules,
    ).unwrap();
    let deck_bottom: Vec<OrderCardId> = session.state().order_deck.iter().take(2).copied().collect();
    (Repl::new(session), deck_bottom)
}

#[test]
fn live_port_take_collects_reveals_then_applies_them() {
    let (mut r, others) = live_repl();
    // Walk to AreaAction with the President on Port A: script the moves by
    // index-free grammar: end additionals, place nothing, then place the
    // President directly at the Port A area via the numbered list.
    r.step("0");        // EndAdditionalPhase
    r.step("a");        // place no assistants
    let port_area = *r.session.state().board_layout.areas.iter()
        .find(|(_, c)| c.area_type == AreaType::PortA).unwrap().0;
    let k = r.session.legal().iter().position(|m| matches!(
        m, Move::PlacePresident { destination } if *destination == port_area
    )).expect("PlacePresident at Port A is legal from hand at game start");
    r.step(&k.to_string());
    // Pick a 1-card port take.
    let take = r.session.legal().iter().position(|m| matches!(
        m, Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } if cards_taken.len() == 1
    )).expect("1-card port take available");
    let out = r.step(&take.to_string());
    // Live mode must now be ASKING for the refill card, not applied yet.
    assert!(out.text.to_lowercase().contains("slot"), "{}", out.text);
    let events_before = r.session.record().events.len();
    // Answer with a DIFFERENT card than the seeded top: bottom-of-deck card.
    let out = r.step(&format!("O{}", others[0].0));
    assert!(!out.text.contains("CRITICAL"), "{}", out.text);
    assert_eq!(r.session.record().events.len(), events_before + 2); // RevealOrders + Move
    // The typed card is now physically on the Port A board.
    assert!(r.session.state().port_state.port_a.slots.iter().any(|s| *s == Some(others[0])));
    assert_eq!(replay(r.session.record(), r.session.rules()).unwrap(), *r.session.state());
}

#[test]
fn reveal_command_makes_hidden_starting_order_public() {
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(2);
    let pending = new_game(config.clone(), &rules, 902).unwrap();
    let hidden = pending.deals[1][0];
    let session = Session::new_live(
        config, 902, SetupObservations::default(),
        vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown],
        rules,
    ).unwrap();
    let mut r = Repl::new(session);
    let out = r.step(&format!("reveal 1 O{}", hidden.0));
    assert!(out.text.contains("revealed"), "{}", out.text);
    assert_eq!(r.session.state().players[1].starting_order, Some(hidden));
    assert_eq!(replay(r.session.record(), r.session.rules()).unwrap(), *r.session.state());
}

#[test]
fn undo_after_live_reveal_group_removes_the_whole_group() {
    let (mut r, others) = live_repl();
    r.step("0");
    r.step("a");
    let port_area = *r.session.state().board_layout.areas.iter()
        .find(|(_, c)| c.area_type == AreaType::PortA).unwrap().0;
    let k = r.session.legal().iter().position(|m| matches!(
        m, Move::PlacePresident { destination } if *destination == port_area
    )).unwrap();
    r.step(&k.to_string());
    let before = r.session.state().clone();
    let events_before = r.session.record().events.len();
    let take = r.session.legal().iter().position(|m| matches!(
        m, Move::ExecuteAreaAction { action: AreaAction::Port { cards_taken, .. } } if cards_taken.len() == 1
    )).unwrap();
    r.step(&take.to_string());
    r.step(&format!("O{}", others[0].0));
    r.step("undo");
    assert_eq!(*r.session.state(), before);
    assert_eq!(r.session.record().events.len(), events_before);
}
```

(Also a small sim-mode guard test: the same port take in a `Simulation` session applies immediately with no reveal prompt.)

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-cli live_` → failures (no reveal prompting yet: events count off / port board lacks the typed card).

- [ ] **Step 3: Implement.** `commit(mv)` becomes:

```rust
fn commit(&mut self, mv: Move) -> StepOutput {
    if matches!(self.session.mode, TrackingMode::Live) {
        match apply_move(self.session.state(), &mv, self.session.rules()) {
            Err(e) => return self.out(format!("Illegal: {e}\n"), false),
            Ok(scratch) => {
                let d_orders = self.session.state().order_deck.len() - scratch.order_deck.len();
                let d_techs = self.session.state().tech_deck.len() - scratch.tech_deck.len();
                if d_orders > 0 {
                    let (slots, board_b) = refilled_port_slots(self.session.state(), &scratch);
                    self.pending = Pending::OrderReveals { mv, slots, board_b, collected: Vec::new() };
                    return self.out(self.reveal_prompt(), false);
                }
                if d_techs > 0 {
                    let board_b = lab_b_changed(self.session.state(), &scratch);
                    self.pending = Pending::TechReveal { mv, board_b };
                    return self.out(self.reveal_prompt(), false);
                }
                if matches!(mv, Move::Take5PowerToken) {
                    let area = claimed_token_area(self.session.state(), &scratch);
                    self.pending = Pending::TokenReveal { mv, area };
                    return self.out(self.reveal_prompt(), false);
                }
            }
        }
    }
    self.finish_apply(mv, vec![])
}
```

Helpers: `refilled_port_slots(before, after) -> (Vec<usize>, bool)` — pick the board whose slots changed; slots = indices `i` where `after.slots[i] != before.slots[i] && after.slots[i].is_some() && before-board doesn't contain after.slots[i]`… simpler and identity-independent: slots where `after.slots[i]` is a card that was in `before.order_deck`; sort DESCENDING. `claimed_token_area(before, after)` — the key whose value went `Some -> None`. `lab_b_changed` — whether `lab_b` differs.

While `pending != None`, `step` routes input into the collector first (before command parsing, except `cancel` → clear pending, `quit` still honored): each accepted order id appends to `collected`; when `collected.len() == slots.len()` → `finish_apply(mv, vec![RecordedEvent::RevealOrders(collected)])`, then the **desync assertion**: for each (slot, card) pair, `state.port_state.<board>.slots[slot] == Some(card)` else lockdown. Tech: one entry → `RevealTechs(vec![id])`. Token: one letter → `RevealFivePowerToken { area, effect }` bound-revealed with `Take5PowerToken`. `reveal <seat> <card>` parses seat (`PlayerId::try_new`) + order code → `apply_standalone`, output `"Seat N revealed O.. as their starting order"`.

- [ ] **Step 4: Run**: `cargo test -p yokohama-cli` PASS; `cargo test -p yokohama-core` untouched-green; zero warnings.

- [ ] **Step 5: Commit**: `git add crates/yokohama-cli && git commit -m "feat: live-mode reveal choreography — draws, tokens, hidden orders, desync guard"`

---

### Task 13: Wizard + `main.rs`

**Files:**
- Create: `crates/yokohama-cli/src/wizard.rs`, `crates/yokohama-cli/src/main.rs`
- Modify: `crates/yokohama-cli/src/lib.rs` (`pub mod wizard;`)

**Interfaces:**
- Produces:
  - `wizard::run_wizard(input: &mut dyn Iterator<Item = String>, out: &mut dyn FnMut(&str)) -> Result<crate::repl::Repl, String>` — fully scripted-testable AND incrementally-echoing: every prompt/error goes through `out` as it happens (tests capture into a `String`; `main` prints+flushes), every answer comes from `input`.
  - Wizard flow (each question re-asks on invalid input, max 3 retries then `Err`):
    1. Mode: `sim` or `live`.
    2. Players: 2/3/4. Colors: per seat from `red/blue/yellow/green` (distinct, validated). Start seat: index.
    3. Seed: integer, or blank → derived from `std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64` and echoed (`"seed: 123456789"`) — the ONLY nondeterminism in the crate, wizard-only, recorded in the `GameRecord` so everything downstream is reproducible.
    4. Save path: string, or blank → `yokohama-game-<seed>.json` in the cwd; sets `session.save_path` (autosave on).
    5. Sim: per seat, show the two dealt cards (`order_prompt_line`) by running `new_game` with the chosen config/seed, ask `keep 0/1`.
       Live: per seat, `known` (then the two cards + keep 0/1) or `hidden`; then observations: tiles per cell (abbrevs: `cm sm tp fg bank pa pb la lb ea ct ch cu canal`), site index per non-Canal cell (`S<idx>` or bare integer), achievements (3 entries: `a1`-style menu keyed by `AchievementType` name list printed with numbers), Port A/B contents left→right (`O..` ×6), Lab A/B contents left→right (`T<id>` or unique name prefix ×6).
    6. Build `Session::new_sim` / `Session::new_live`, wrap in `Repl`.
  - `main.rs`: `args[1]` present → `Session::load(path, Rules::standard())` → `Repl`; else wizard over stdin. Then loop: print `repl.prompt()` once, then per line print `step(line).text` until `quit` or EOF.

- [ ] **Step 1: Failing tests** (bottom of `wizard.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_core::*;

    fn feed(lines: &[&str]) -> (Result<crate::repl::Repl, String>, String) {
        let mut transcript = String::new();
        let mut it = lines.iter().map(|s| s.to_string());
        let result = {
            let mut sink = |s: &str| transcript.push_str(s);
            run_wizard(&mut it, &mut sink)
        };
        (result, transcript)
    }

    #[test]
    fn sim_wizard_builds_a_playable_session() {
        let (repl, transcript) = feed(&[
            "sim", "2", "red", "blue", "0", "42", "", // mode players colors start seed savepath(default)
            "0", "1",                                  // keep choices for seats 0 and 1
        ]);
        let repl = repl.unwrap();
        assert!(transcript.contains("keep"), "{transcript}");
        assert_eq!(repl.session.state().config.player_count, 2);
        assert_eq!(repl.session.record().seed, 42);
        assert!(repl.session.save_path.is_some());
        assert!(!repl.session.legal().is_empty());
        std::fs::remove_file("yokohama-game-42.json").ok(); // in case autosave fired
    }

    #[test]
    fn live_wizard_accepts_full_observations() {
        // "Physical table" = a different seed's setup, exactly as in the
        // core live tests.
        let rules = Rules::standard();
        let table = new_game(GameConfig::for_player_count(2), &rules, 555).unwrap();
        let n = table.state.board_layout.areas.len();
        let abbrev = |t: AreaType| match t {
            AreaType::CopperMine => "cm", AreaType::SilkMill => "sm",
            AreaType::TeaPlantation => "tp", AreaType::FishingGround => "fg",
            AreaType::Bank => "bank", AreaType::PortA => "pa", AreaType::PortB => "pb",
            AreaType::LaboratoryA => "la", AreaType::LaboratoryB => "lb",
            AreaType::EmploymentAgency => "ea", AreaType::Chinatown => "ct",
            AreaType::Church => "ch", AreaType::Customs => "cu", AreaType::Canal => "canal",
        };
        let mut lines: Vec<String> = vec![
            "live".into(), "2".into(), "red".into(), "blue".into(), "0".into(), "77".into(), "".into(),
            // seat 0 known deal + keep, seat 1 hidden
            "known".into(),
            format!("O{}", table.deals[0][0].0), format!("O{}", table.deals[0][1].0), "0".into(),
            "hidden".into(),
        ];
        for i in 0..n {
            lines.push(abbrev(table.state.board_layout.areas[&AreaId::new(i as u8)].area_type).into());
        }
        let mut site = 0u8;
        for i in 0..n {
            let t = table.state.board_layout.areas[&AreaId::new(i as u8)].area_type;
            if t != AreaType::Canal { lines.push(format!("{site}")); site += 1; }
        }
        for a in table.state.achievements.iter() {
            lines.push(format!("{:?}", a.achievement_type)); // menu accepts the type name
        }
        for i in 0..6 {
            lines.push(format!("O{}", table.state.port_state.port_a.slots[i].unwrap().0));
        }
        for i in 0..6 {
            lines.push(format!("T{}", table.state.lab_state.lab_a.slots[i].unwrap().0));
        }
        let mut transcript = String::new();
        let mut it = lines.into_iter();
        let repl = {
            let mut sink = |s: &str| transcript.push_str(s);
            run_wizard(&mut it, &mut sink).unwrap()
        };
        assert!(matches!(repl.session.mode, crate::session::TrackingMode::Live));
        // Observed zones took effect.
        for i in 0..6 {
            assert_eq!(repl.session.state().port_state.port_a.slots[i], table.state.port_state.port_a.slots[i]);
        }
        assert_eq!(repl.session.state().players[0].starting_order, Some(table.deals[0][0]));
        assert!(repl.session.state().players[1].starting_order_hidden);
        std::fs::remove_file("yokohama-game-77.json").ok();
    }
}
```

- [ ] **Step 2: Verify failure**: `cargo test -p yokohama-cli wizard` → compile errors.

- [ ] **Step 3: Implement** `wizard.rs` (an `ask` helper drives every question):

```rust
fn ask(
    input: &mut dyn Iterator<Item = String>,
    out: &mut dyn FnMut(&str),
    prompt: &str,
    parse: &mut dyn FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    for _ in 0..3 {
        out(prompt);
        out("\n");
        let line = input.next().ok_or("input ended mid-wizard")?;
        match parse(line.trim()) {
            Ok(()) => return Ok(()),
            Err(e) => out(&format!("  ! {e}\n")),
        }
    }
    Err(format!("gave up after 3 invalid answers to: {prompt}"))
}
```

Then linear flow filling local `mode/players/colors/start/seed/save_path/choices/obs`, with per-question closures doing the parsing (colors: match on the four names, reject duplicates; seed blank → SystemTime nanos; tile abbrevs per the table above; achievements: match input against `format!("{:?}", ty)` case-insensitively over all 12 `AchievementType` values; sites: parse `u8` with or without leading `S`; orders `O`-prefix optional; techs `T<id>` or name prefix resolved via `rules.techs.iter()`). For sim keep-choices, run `new_game(config.clone(), &rules, seed)` first and print each seat's two `order_prompt_line`s into the transcript before asking. Construct the session, set `save_path`, `Ok(Repl::new(session))`.

`main.rs`:

```rust
use std::io::{BufRead, Write};

fn main() {
    let rules = yokohama_core::Rules::standard();
    let args: Vec<String> = std::env::args().collect();
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines().map_while(Result::ok);

    let mut repl = if let Some(path) = args.get(1) {
        match yokohama_cli::session::Session::load(std::path::Path::new(path), rules) {
            Ok(s) => yokohama_cli::repl::Repl::new(s),
            Err(e) => {
                eprintln!("failed to load {path}: {e}");
                std::process::exit(1);
            }
        }
    } else {
        let mut echo = |s: &str| {
            print!("{s}");
            std::io::stdout().flush().ok();
        };
        match yokohama_cli::wizard::run_wizard(&mut lines, &mut echo) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("setup aborted: {e}");
                std::process::exit(1);
            }
        }
    };

    print!("{}", repl.prompt());
    std::io::stdout().flush().ok();
    for line in lines {
        let out = repl.step(&line);
        print!("{}", out.text);
        std::io::stdout().flush().ok();
        if out.quit {
            break;
        }
    }
}
```

The `out` closure keeps the wizard interactive (prompts echo as asked) while staying fully scripted-testable — the tests capture into a `String`, `main` prints and flushes.

- [ ] **Step 4: Run**: `cargo test -p yokohama-cli` PASS; manual smoke: `printf 'sim\n2\nred\nblue\n0\n42\n\n0\n1\n0\nquit\n' | cargo run -p yokohama-cli` — expect wizard transcript, prompt, one applied move, clean exit; `rm -f yokohama-game-42.json`.

- [ ] **Step 5: Commit**: `git add crates/yokohama-cli && git commit -m "feat: setup wizard (sim+live observation entry) and stdin shuttle main"`

---

### Task 14: Verification sweep, docs, fuzz gate

**Files:**
- Modify: `guidingpath.md` (Current state note + Phase-2 progress + decision-log rows)
- Modify: `CLAUDE.md` (Commands: CLI run line; suite count)
- No code changes expected; fix anything the sweep finds.

- [ ] **Step 1: Full verification**

```bash
cargo test -p yokohama-core 2>&1 | tail -5
cargo test -p yokohama-cli 2>&1 | tail -5
cargo build --workspace 2>&1 | grep -c warning   # expect 0
cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored 2>&1 | tail -3
```

Expected: both suites green (record the new totals), zero warnings, fuzz gate `1 passed` (~8s). Any fuzz failure = an engine regression from Tasks 1–7 — fix before proceeding (systematic-debugging skill).

- [ ] **Step 2: Manual dogfood smoke** — play ~15 REPL turns interactively (or piped) at 2p and 3p; confirm prompts readable, `board`/`ports`/`undo`/`save` behave; note friction for the dogfooding backlog rather than gold-plating now.

- [ ] **Step 3: Update `guidingpath.md`**: Phase-2 section gains a dated "driver landed" progress note (branch, suite counts, what works, phase gate still pending choice encoding); decision log gains rows for: GameRecord v2 event log; reveal-event semantics (arrange/starting-order/token swap-or-replace); ObservedBoard re-typing; hand-limit-free reveal with invariant backstop; player names dropped; phase-gate-awaits-choice-encoding. Update `CLAUDE.md` commands (`cargo run -p yokohama-cli` + one-line description; new test counts).

- [ ] **Step 4: Commit**: `git add guidingpath.md CLAUDE.md && git commit -m "docs: Phase-2 driver landed — guidingpath progress + decision-log rows; CLI commands"`

- [ ] **Step 5:** Invoke superpowers:finishing-a-development-branch for the merge decision (fuzz gate already green from Step 1; re-run if any code changed after it).
