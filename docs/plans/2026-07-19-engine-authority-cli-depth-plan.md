# Engine-authority + CLI-depth Implementation Plan (2026-07-19)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the engine authoritative enough to train on (checked transition boundary, complete legal-move projection, authoritative terminal results, trustworthy records) and the ASCII CLI deep enough that playtesting is pleasant.

**Architecture:** Spec is `docs/plans/2026-07-19-engine-authority-cli-depth-design.md` (approved 2026-07-19). Tier 1 (tasks 1–10) lands the engine contract the bot phase consumes; Tier 2 (tasks 11–16) lands CLI depth above the one-way glass; Tier 3 rides (task 17) plus close-out (task 18). Branch: `engine-authority` (already created; design committed at f1c26c0).

**Tech Stack:** Rust workspace — `crates/yokohama-core` (engine), `crates/yokohama-cli` (driver). No new dependencies anywhere in this plan (color uses `std::io::IsTerminal`; hashing is hand-rolled FNV-1a).

## Global Constraints

- TDD strictly: failing test first, pasted RED output, then implementation. The repo's commit history is strictly TDD — keep it that way.
- After EVERY task: `cargo test -p yokohama-core` AND `cargo test -p yokohama-cli` green with **zero warnings**, AND `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Never run `cargo fmt`. Never touch `.codex/`, `.vscode/`, `AGENTS.md`, `tts-assets/`.
- `git add` by explicit path only (the working tree carries Jeremy's untracked files and unstaged deletions).
- Commit after every task with a conventional message (`feat:`/`fix:`/`refactor:`/`docs:`).
- Engine state stays history-free and cheap to clone; presentation logic never enters `yokohama-core` (one-way glass, design §2).
- Schema breaks in this plan (Move::ReturnPresidentToHand shape, GameRecord.rules_hash) are APPROVED clean breaks (design §4.7): update fixtures, no migration shims; dogfood-era saves become non-loadable by design.
- Baseline counts at branch start: 466 core tests + 85 CLI tests, zero warnings. Release fuzz gate: `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored` (~8s, 1,500 games) — run before merge and whenever a task's flag says so.

---

### Task 1: Checked transition boundary (`apply_move` / `apply_move_unchecked`)

**Files:**
- Modify: `crates/yokohama-core/src/game/apply_move.rs:8` (rename + new wrapper)
- Modify: `crates/yokohama-core/src/types/errors.rs:6-18` (new variant)
- Modify: `crates/yokohama-cli/src/session.rs:226`, `crates/yokohama-cli/src/repl.rs:335` (sort to unchecked)
- Modify: `crates/yokohama-core/tests/common/mod.rs:193,273` and `crates/yokohama-core/tests/fuzz_full_games.rs:127,166,200,257,262` (sort to unchecked)
- Create: `crates/yokohama-core/tests/checked_boundary.rs`
- Modify: `guidingpath.md` (§5.3-7 row justification correction)

**Interfaces:**
- Consumes: existing `apply_move(state: &GameState, mv: &Move, rules: &Rules) -> Result<GameState, MoveError>` (apply_move.rs:8), `get_legal_moves(state: &GameState, rules: &Rules) -> Vec<Move>` (legal_moves.rs:8).
- Produces: `pub fn apply_move_unchecked(state: &GameState, mv: &Move, rules: &Rules) -> Result<GameState, MoveError>` (the current body, renamed); `pub fn apply_move(...)` (same signature, membership-checked); `MoveError::NotLegal`. Every later task and the Phase-3 bot treat checked `apply_move` as the default entry.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/yokohama-core/tests/checked_boundary.rs
//! Design §4.1: the public transition is membership-checked; the raw
//! transition is explicit opt-in for prechecked hot paths.
mod common;

use yokohama_core::{
    apply_move, apply_move_unchecked, get_legal_moves, Move, MoveError, Rules,
};

/// A deterministic mid-game state: fold the first `n` random moves of a seed.
fn state_after(seed: u64, players: u8, n: usize) -> yokohama_core::GameState {
    let rules = Rules::standard();
    let (mut state, _) = common::new_seeded_game(seed, players, &rules);
    for _ in 0..n {
        let moves = get_legal_moves(&state, &rules);
        state = apply_move_unchecked(&state, &moves[0], &rules).unwrap();
    }
    state
}

#[test]
fn checked_apply_rejects_out_of_phase_move_with_not_legal() {
    let rules = Rules::standard();
    let state = state_after(4, 2, 0); // Placement phase, start of game
    // A Movement-phase move: free-path teleport the raw arm would happily apply.
    let mv = Move::MovePresident { path: vec![], destination: yokohama_core::AreaId::new(2) };
    assert!(!get_legal_moves(&state, &rules).contains(&mv));
    assert_eq!(apply_move(&state, &mv, &rules), Err(MoveError::NotLegal));
}

#[test]
fn checked_apply_rejects_in_phase_but_illegal_move_with_not_legal() {
    let rules = Rules::standard();
    let state = state_after(4, 2, 0);
    // In-phase (Placement) but not in the legal list: place into the same
    // area three times with Option A (generator never emits duplicates).
    let a = *state.board_layout.areas.keys().next().unwrap();
    let mv = Move::PlaceThreeAreas { placements: vec![a, a, a] };
    if get_legal_moves(&state, &rules).contains(&mv) {
        panic!("fixture assumption broken: pick a different illegal shape");
    }
    assert_eq!(apply_move(&state, &mv, &rules), Err(MoveError::NotLegal));
}

#[test]
fn checked_apply_accepts_every_legal_move_and_matches_unchecked() {
    let rules = Rules::standard();
    let state = state_after(9, 3, 5);
    for mv in get_legal_moves(&state, &rules) {
        let a = apply_move(&state, &mv, &rules).expect("legal move must pass the checked path");
        let b = apply_move_unchecked(&state, &mv, &rules).unwrap();
        assert_eq!(a, b, "checked and unchecked must agree on legal moves");
    }
}
```

If `common::new_seeded_game` does not exist, add it to `tests/common/mod.rs` as a thin wrapper over the existing seeded setup used by `run_random_game_from_observed` (new_game → finalize_setup with the same choice policy), returning `(GameState, Rules-agnostic setup info)`. Reuse the existing code paths — do not invent new setup logic.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p yokohama-core --test checked_boundary`
Expected: COMPILE FAIL — `apply_move_unchecked` and `MoveError::NotLegal` do not exist. Paste the error.

- [ ] **Step 3: Implement**

In `types/errors.rs`, add to `MoveError` (keep derives unchanged):

```rust
    /// The move is not in `get_legal_moves` for this state (checked boundary,
    /// design §4.1). Raw per-arm validation lives behind `apply_move_unchecked`.
    NotLegal,
```

Add a `Display` arm: `MoveError::NotLegal => write!(f, "move is not in the legal-move list for this state")`.

In `game/apply_move.rs`: rename the existing `pub fn apply_move` (line 8) to `pub fn apply_move_unchecked` (keep body byte-identical), then add:

```rust
/// The ONE checked public transition (design §4.1): membership in
/// `get_legal_moves` is the legality contract; per-arm validation then runs
/// in the raw transition. Prechecked hot paths (Session's gate, the fuzz
/// drivers, future search inner loops) call `apply_move_unchecked`
/// deliberately, AFTER choosing from an enumerated legal list.
pub fn apply_move(state: &GameState, mv: &Move, rules: &Rules) -> Result<GameState, MoveError> {
    if !crate::game::legal_moves::get_legal_moves(state, rules).contains(mv) {
        return Err(MoveError::NotLegal);
    }
    apply_move_unchecked(state, mv, rules)
}
```

**Call-site sort (design flag A — every site deliberate):**
- `crates/yokohama-core/src/game/record.rs:133` (`replay_from`): LEAVE as `apply_move` — it now gets the membership check. This is the point of the task.
- `crates/yokohama-cli/src/session.rs:226`: change to `apply_move_unchecked` — the gate at session.rs:223 just proved membership on the same staged state; keeping `apply_move` would double-enumerate every interactive apply.
- `crates/yokohama-cli/src/repl.rs:335` (live-mode scratch probe): change to `apply_move_unchecked` — the probed move came from the numbered legal list, and the real apply behind it goes through Session's gate.
- `crates/yokohama-core/tests/common/mod.rs` (both loops, lines ~193 and ~273): change to `apply_move_unchecked` — the driver picks from `get_legal_moves` by construction; keeping checked would roughly double fuzz-gate enumeration cost.
- `crates/yokohama-core/tests/fuzz_full_games.rs` lines 127/166/200 (seeded scenarios) and 249-263 (`legal_moves_all_apply_over_short_random_walk`): change to `apply_move_unchecked` — same argument; the walk test applies every legal move per state and would go quadratic under checked.
- All other test call sites: leave on `apply_move` unless a test's runtime visibly suffers; tests that intentionally apply illegal moves to hit per-arm errors (e.g. `NotEnoughMoney` assertions) must switch to `apply_move_unchecked` to keep reaching the raw arms — let the failing assertions identify them.
- Export both from `game/mod.rs` (the existing `pub use apply_move::*` already does).

In `guidingpath.md`, find the 2026-07-19 §5.3-7 row and edit ONLY its justification clause: replace `The Session legal-list gate plus replay-through-`apply_move` validates untrusted records strictly more strongly than a phase check would` with `The Session legal-list gate validates interactive input; replay paths gained real validation only with the checked apply_move boundary (2026-07-19 engine-authority plan Task 1) — this row's original justification overstated the load path, per the Phase-3 entry backlog note`.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p yokohama-core --test checked_boundary` → 3 PASS. Then full suites + clippy (Global Constraints). Then run the release fuzz gate and paste its wall time next to the pre-task time (expectation: unchanged within noise, because fuzz stays on the raw arm):
`cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored`

- [ ] **Step 5: Commit**

```bash
git add crates/yokohama-core/src/game/apply_move.rs crates/yokohama-core/src/types/errors.rs crates/yokohama-core/tests/checked_boundary.rs crates/yokohama-core/tests/common/mod.rs crates/yokohama-core/tests/fuzz_full_games.rs crates/yokohama-cli/src/session.rs crates/yokohama-cli/src/repl.rs guidingpath.md
git commit -m "feat: checked apply_move boundary; raw transition explicit as apply_move_unchecked (design 4.1)"
```

---

### Task 2: Board-membership invariants

**Files:**
- Modify: `crates/yokohama-core/src/game/invariants.rs` (extend `check_invariants`, currently 10 named checks ending at line 134)
- Test: same file's `#[cfg(test)]` module

**Interfaces:**
- Consumes: `check_invariants(state: &GameState, rules: &Rules) -> Result<(), String>` (invariants.rs:21); `GameState.board_layout.areas: BTreeMap<AreaId, _>`.
- Produces: the same function, now also rejecting any stored `AreaId` absent from `board_layout.areas`. Strict replay (Task 3) and the fuzz driver inherit it automatically.

- [ ] **Step 1: Write the failing test** (in invariants.rs tests, following that module's existing fixture style)

```rust
#[test]
fn out_of_board_area_ids_violate_invariants() {
    let rules = Rules::standard();
    let (state, _) = crate::game::setup_fixture_2p(&rules); // reuse the module's existing state fixture; if none exists locally, build via the same new_game/finalize path the neighboring tests use
    let ghost = AreaId::unchecked(99);

    let mut s = state.clone();
    s.players[0].president_location = PresidentLocation::OnBoard(ghost);
    assert!(check_invariants(&s, &rules).is_err(), "ghost president area must fail");

    let mut s = state.clone();
    s.players[0].assistants.on_board.insert(ghost, 1);
    // keep totals conserved so ONLY the membership check can catch it:
    s.players[0].assistants.in_hand -= 1;
    assert!(check_invariants(&s, &rules).is_err(), "ghost assistant area must fail");

    let mut s = state.clone();
    s.station_location = Some(ghost);
    assert!(check_invariants(&s, &rules).is_err(), "ghost station area must fail");

    let mut s = state.clone();
    s.president_start_area = Some(ghost);
    assert!(check_invariants(&s, &rules).is_err(), "ghost president_start_area must fail");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p yokohama-core out_of_board_area_ids` → FAIL (invariants currently pass all four). Paste output.

- [ ] **Step 3: Implement** — append to `check_invariants` before its final `Ok(())`:

```rust
    // Board membership (design §4.1): every stored AreaId must exist on the
    // board. A record or bot that fabricates areas must fail loudly here.
    let on_board = |a: &AreaId| state.board_layout.areas.contains_key(a);
    for p in &state.players {
        if let PresidentLocation::OnBoard(a) = p.president_location {
            if !on_board(&a) { return Err(format!("player {} president on non-board area {:?}", p.id.0, a)); }
        }
        for a in p.assistants.on_board.keys() {
            if !on_board(a) { return Err(format!("player {} assistants on non-board area {:?}", p.id.0, a)); }
        }
        for a in p.shophouses.built.keys() {
            if !on_board(a) { return Err(format!("player {} shophouse on non-board area {:?}", p.id.0, a)); }
        }
        for a in &p.trading_houses.built {
            if !on_board(a) { return Err(format!("player {} trading house on non-board area {:?}", p.id.0, a)); }
        }
    }
    if let Some(a) = state.station_location {
        if !on_board(&a) { return Err(format!("station token on non-board area {:?}", a)); }
    }
    if let Some(a) = state.president_start_area {
        if !on_board(&a) { return Err(format!("president_start_area is non-board area {:?}", a)); }
    }
```

Adjust field access to the real types as the compiler dictates (e.g. `trading_houses.built` iteration shape) — but every listed field MUST be covered. Also sweep `types/state.rs` for any other `AreaId`-bearing field (e.g. 5-Power token map, mercenary/area tracking) and bound each the same way; list what you covered in the task report.

- [ ] **Step 4: Run to verify pass** — targeted test, both suites, clippy.

- [ ] **Step 5: Commit** — `git add crates/yokohama-core/src/game/invariants.rs && git commit -m "feat: board-membership invariants for all stored AreaIds"`

---

### Task 3: Strict replay mode

**Files:**
- Modify: `crates/yokohama-core/src/game/record.rs` (new `replay_strict`, refactor shared fold)
- Test: `crates/yokohama-core/tests/game_record.rs`

**Interfaces:**
- Consumes: `replay(record, rules) -> Result<GameState, ReplayError>` (record.rs:175), `replay_from(initial, events, rules) -> Result<GameState, ReplayError>` (record.rs:128), checked `apply_move` (Task 1), `check_invariants` (Task 2), `ReplayError { event_index, message }`.
- Produces: `pub fn replay_strict(record: &GameRecord, rules: &Rules) -> Result<GameState, ReplayError>` — rejects version mismatch (event_index 0), membership-checks every move (inherited from checked `apply_move`, already live in `replay_from` since Task 1), and runs `check_invariants` after every event. Task 4 extends it with the rules-hash check. Training pipelines use ONLY this entry.

- [ ] **Step 1: Write the failing tests** (append to `tests/game_record.rs`)

```rust
#[test]
fn strict_replay_matches_normal_replay_on_a_clean_record() {
    let rules = Rules::standard();
    let (final_state, record) = common::run_recorded_game(7, 2, &rules);
    assert_eq!(replay_strict(&record, &rules).unwrap(), final_state);
}

#[test]
fn strict_replay_rejects_engine_version_mismatch_instead_of_warning() {
    let rules = Rules::standard();
    let (_, mut record) = common::run_recorded_game(7, 2, &rules);
    record.engine_version = "0.0.0-tampered".into();
    let err = replay_strict(&record, &rules).unwrap_err();
    assert_eq!(err.event_index, 0);
    assert!(err.message.contains("engine_version"));
    // and the lenient path still replays (warn-and-continue preserved):
    assert!(replay(&record, &rules).is_ok());
}

#[test]
fn strict_replay_rejects_an_out_of_phase_tampered_move_with_its_event_index() {
    let rules = Rules::standard();
    let (_, mut record) = common::run_recorded_game(7, 2, &rules);
    // Overwrite event k with a move that is never legal at that point.
    let k = 1;
    record.events[k] = RecordedEvent::Move(Move::Take5PowerToken);
    let err = replay_strict(&record, &rules).unwrap_err();
    assert_eq!(err.event_index, k);
}
```

(If `Take5PowerToken` happens to be legal at index 1 for seed 7, pick a different obviously-out-of-phase variant — assert first with `get_legal_moves` that it is illegal there, then tamper.)

- [ ] **Step 2: Run to verify failure** — `cargo test -p yokohama-core --test game_record` → COMPILE FAIL (`replay_strict` missing). Paste.

- [ ] **Step 3: Implement** in record.rs:

```rust
/// Strict replay for training pipelines (design §4.1): reject, don't warn.
/// - engine_version mismatch => Err at event_index 0
/// - every Move event goes through the checked `apply_move` (membership)
/// - `check_invariants` runs after EVERY event
pub fn replay_strict(record: &GameRecord, rules: &Rules) -> Result<GameState, ReplayError> {
    let running = env!("CARGO_PKG_VERSION");
    if record.engine_version != running {
        return Err(ReplayError {
            event_index: 0,
            message: format!(
                "engine_version mismatch: record \"{}\" vs running \"{running}\" (strict mode rejects)",
                record.engine_version
            ),
        });
    }
    let initial = rebuild_initial_state(record, rules)?; // extract the existing setup-preamble block of `replay` into this helper and reuse it in both
    let mut state = initial;
    for (i, ev) in record.events.iter().enumerate() {
        state = fold_one_event(&state, ev, rules)
            .map_err(|message| ReplayError { event_index: i, message })?;
        crate::game::invariants::check_invariants(&state, rules)
            .map_err(|m| ReplayError { event_index: i, message: format!("invariant violated: {m}") })?;
    }
    Ok(state)
}
```

Refactor mechanics: extract `replay`'s setup-preamble (new_game → optional reconcile → finalize, currently reporting index 0 on failure) into `fn rebuild_initial_state(record, rules) -> Result<GameState, ReplayError>`, and extract `replay_from`'s per-event match into `fn fold_one_event(state, ev, rules) -> Result<GameState, String>`; make `replay`/`replay_from` delegate to them so there is exactly ONE fold implementation (the a6faa87 lesson). Behavior of `replay`/`replay_from` must not change (the existing tests prove it).

- [ ] **Step 4: Run to verify pass** — game_record tests, both suites, clippy.

- [ ] **Step 5: Commit** — `git add crates/yokohama-core/src/game/record.rs crates/yokohama-core/tests/game_record.rs && git commit -m "feat: replay_strict — reject version mismatch, per-event invariants, one shared fold"`

---

### Task 4: Rules-hash in `GameRecord`

**Files:**
- Modify: `crates/yokohama-core/src/game/rules.rs` (`content_hash`), `crates/yokohama-core/src/game/board_gen.rs` (version const), `crates/yokohama-core/src/game/record.rs` (`GameRecord.rules_hash` + strict check + `new` signature)
- Modify: every `GameRecord::new` call site (compiler-driven; includes `crates/yokohama-cli/src/session.rs` constructors and `tests/common/mod.rs:242` `run_recorded_game`)
- Test: `crates/yokohama-core/tests/game_record.rs`, `crates/yokohama-core/src/game/rules.rs` tests

**Interfaces:**
- Consumes: `Rules { orders, techs, building_sites, five_power_tokens }` (rules.rs:8-14), `GameRecord` (record.rs:41-61), `replay_strict` (Task 3).
- Produces: `impl Rules { pub fn content_hash(&self) -> String }` (stable FNV-1a-64 hex over canonically-ordered data + `BOARD_DERIVATION_VERSION`); `GameRecord.rules_hash: String`; `GameRecord::new(..., rules: &Rules)` stamping it; `replay_strict` rejecting mismatch at event_index 0; lenient `replay` warning only. **This is a record schema break (approved; combined with Task 7's).**

- [ ] **Step 1: Write the failing tests**

```rust
// rules.rs tests
#[test]
fn content_hash_is_stable_and_distinguishes_data() {
    let a = Rules::standard().content_hash();
    let b = Rules::standard().content_hash();
    assert_eq!(a, b, "hash must be deterministic");
    assert_eq!(a.len(), 16, "fnv1a-64 hex");
    assert_ne!(a, Rules::empty_for_tests().content_hash());
}

// tests/game_record.rs
#[test]
fn records_stamp_the_rules_hash_and_strict_replay_rejects_a_mismatch() {
    let rules = Rules::standard();
    let (_, mut record) = common::run_recorded_game(7, 2, &rules);
    assert_eq!(record.rules_hash, rules.content_hash());
    record.rules_hash = "deadbeefdeadbeef".into();
    let err = replay_strict(&record, &rules).unwrap_err();
    assert_eq!(err.event_index, 0);
    assert!(err.message.contains("rules_hash"));
    assert!(replay(&record, &rules).is_ok(), "lenient replay warns but continues");
}
```

- [ ] **Step 2: Run to verify failure** — COMPILE FAIL (`content_hash`, `rules_hash` missing). Paste.

- [ ] **Step 3: Implement**

In `board_gen.rs`: `pub const BOARD_DERIVATION_VERSION: u32 = 1;` with a doc comment: "Bump on ANY change to board topology derivation (adjacency, canal crossings, tile multisets). Participates in Rules::content_hash."

In `rules.rs`:

```rust
/// Stable content hash of the rules data (design §4.6): FNV-1a 64 over a
/// canonical line-per-entry string in sorted-id order, plus the board
/// derivation version. Hand-rolled — std's Hasher is not stable across
/// releases and this hash lives in saved records.
pub fn content_hash(&self) -> String {
    fn fnv1a(bytes: &[u8], mut h: u64) -> u64 {
        for &b in bytes { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        h
    }
    let mut canon = String::new();
    for o in self.orders.iter_sorted_by_id() {
        canon.push_str(&format!("O|{:?}\n", o));
    }
    for t in self.techs.iter_sorted_by_id() {
        canon.push_str(&format!("T|{:?}\n", t));
    }
    for (i, s) in self.building_sites.iter().enumerate() {
        canon.push_str(&format!("S|{i}|{:?}\n", s));
    }
    for (i, f) in self.five_power_tokens.iter().enumerate() {
        canon.push_str(&format!("F|{i}|{:?}\n", f));
    }
    canon.push_str(&format!("B|{}\n", crate::game::board_gen::BOARD_DERIVATION_VERSION));
    format!("{:016x}", fnv1a(canon.as_bytes(), 0xcbf29ce484222325))
}
```

If the registries lack a sorted iterator, add `iter_sorted_by_id()` (collect + sort by id — registries are small and this is not hot). `{:?}` on registry entries is deliberate: any data-shape change changes the hash, which is the desired semantic.

In `record.rs`: add field `pub rules_hash: String,` after `engine_version`; extend `GameRecord::new` to take `rules: &Rules` and stamp `rules_hash: rules.content_hash()`; fix all call sites (compiler-driven). In `replay_strict` add, right after the version check:

```rust
    let expected = rules.content_hash();
    if record.rules_hash != expected {
        return Err(ReplayError { event_index: 0, message: format!(
            "rules_hash mismatch: record \"{}\" vs running \"{expected}\" (strict mode rejects)", record.rules_hash) });
    }
```

In lenient `replay`, mirror the existing engine_version warn block with a `rules_hash` warn (eprintln, continue).

- [ ] **Step 4: Run to verify pass** — both suites, clippy.

- [ ] **Step 5: Commit** — `git add -A crates/yokohama-core/src crates/yokohama-core/tests crates/yokohama-cli/src && git commit -m "feat: rules content hash stamped into GameRecord; strict replay rejects mismatch"`

---

### Task 5: Station token placement on acquisition

**Files:**
- Modify: `crates/yokohama-core/src/types/choices.rs` (new `PendingChoice::StationPlace`), `crates/yokohama-core/src/types/moves.rs` (new `Move::ChooseStationPlace`), `crates/yokohama-core/src/game/choices.rs` (generation + resolution arms), `crates/yokohama-core/src/game/apply_move.rs` (Laboratory arm hook + choice-move dispatch arm)
- Modify: `crates/yokohama-cli/src/describe.rs` (`describe_move` + `describe_pending_choice` arms — wildcard-free matches force both)
- Modify: `crates/yokohama-core/tests/choice_encoding.rs` (census lists 6→7 pending / 9→10 moves; two-prong dispatcher arm)
- Test: `crates/yokohama-core/src/game/actions/cards.rs` tests + `tests/choice_encoding.rs`

**Interfaces:**
- Consumes: `push_choice(state, choice, rules)` (choices.rs:114), `resolve_choice_move` (choices.rs:139), the Laboratory apply arm (apply_move.rs:492-498), `execute_lab_action` tech push (actions/cards.rs:376), `state.station_location: Option<AreaId>` (state.rs:236), `area_type.allows_placement()`.
- Produces: `PendingChoice::StationPlace`; `Move::ChooseStationPlace { area: AreaId }`; acquisition of the Station tech queues the choice; resolution sets `station_location`. Card basis (08-tech-cards.yaml:86-93 + KB:576-579): "When you gain this card, place the Station token" — "in any area"; we exclude only the Canal (`allows_placement()`), no other restriction.

- [ ] **Step 1: Write the failing tests**

In `actions/cards.rs`'s test module, reuse the existing lab-purchase fixture: find the test in that module that drives `execute_lab_action` to a SUCCESSFUL purchase (the C2 ownership-gate work added it — grep the module for `execute_lab_action(` inside `#[cfg(test)]` and pick the success-path test). Copy that test's setup verbatim into the new test below and swap the purchased card to Station (`TechCardId(15)` per data.rs:64), arranging the lab slot to hold it:

```rust
#[test]
fn acquiring_station_queues_a_station_place_choice() {
    // fixture: same shape as the existing successful-purchase test in this module,
    // with the lab slot holding TechCardId(15) (Station) and the buyer able to pay.
    // ... fixture code copied from the neighboring test ...
    execute_lab_action(/* fixture args for the Station purchase */).unwrap();
    assert_eq!(state.station_location, None, "token placement is a queued choice, not automatic");
    assert_eq!(state.pending_choices, vec![PendingChoice::StationPlace]);
}
```

In `tests/choice_encoding.rs`:

```rust
#[test]
fn station_place_resolutions_cover_all_placeable_areas_and_resolution_sets_the_token() {
    let rules = Rules::standard();
    let mut state = create_test_game_state(2);
    state.pending_choices.push(PendingChoice::StationPlace);
    let legal = get_legal_moves(&state, &rules);
    let placeable: Vec<AreaId> = state.board_layout.areas.iter()
        .filter(|(_, c)| c.area_type.allows_placement())
        .map(|(&a, _)| a).collect();
    assert_eq!(legal.len(), placeable.len(), "one ChooseStationPlace per placeable area, no Decline");
    let target = placeable[3];
    let next = apply_move(&state, &Move::ChooseStationPlace { area: target }, &rules).unwrap();
    assert_eq!(next.station_location, Some(target));
    assert!(next.pending_choices.is_empty());
}
```

Also extend `assert_decline_or_never_singleton` (choice_encoding.rs:378) with a `PendingChoice::StationPlace` arm asserting `resolutions.len() >= 2` (a real board always has ≥2 placeable areas; the push-time singleton rule covers the degenerate case structurally). Extend `ALL_PENDING` (line ~640) and `ALL_CHOICE_MOVES` (line ~642) — the census will now FAIL until Station fires organically; that is the intended RED for the hook step. Keep `CENSUS_SEED_CAP` at 300 initially; if the census cannot find an organic Station purchase within 300 seeds, raise the cap for the Station entries only by allowing up to 600 with a comment recording the first-hit seed (precedent: AnyGood at seed 222).

- [ ] **Step 2: Run to verify failure** — COMPILE FAIL (missing variants), then census RED. Paste both.

- [ ] **Step 3: Implement**

`types/choices.rs`: add `StationPlace,` variant (doc: "Place the Station token — queued when the Station tech is acquired (design §4.2). No Decline: the token must land."). `types/moves.rs`: add `ChooseStationPlace { area: AreaId },` beside the other 8 choice-resolution variants.

`game/choices.rs` — in `generate_choice_resolutions_for`'s match, add:

```rust
        PendingChoice::StationPlace => state.board_layout.areas.iter()
            .filter(|(_, c)| c.area_type.allows_placement())
            .map(|(&area, _)| Move::ChooseStationPlace { area })
            .collect(),
```

In `resolve_choice_move`'s `match (head, mv)`, add:

```rust
        (PendingChoice::StationPlace, Move::ChooseStationPlace { area }) => {
            state.station_location = Some(*area);
        }
```

`apply_move.rs`: add `ChooseStationPlace` to the choice-move dispatch arm at lines 138-147 (the `|`-chain funneling to `resolve_choice_move`). In the Laboratory arm (after `execute_lab_action` + `apply_lab_acquisition_bonuses`, lines 492-498), queue on acquisition:

```rust
            // Station (design §4.2): the token is placed on acquisition, as a
            // queued choice. copies:1, but the is_none() guard is principled.
            if new_state.station_location.is_none()
                && purchased_effect_is_station /* derive: rules.techs.get(card_id).map(|t| t.effect == TechEffect::Station) */
            {
                crate::game::choices::push_choice(&mut new_state, PendingChoice::StationPlace, rules);
            }
```

(Bind `card_id` from the `AreaAction::Laboratory` action fields already matched in that arm.) Also update `actions/cards.rs`'s v1-limitation doc comment (389-390) — the limitation is CLOSED; say the choice queues in the apply arm.

`describe.rs`: `describe_move` arm → `Move::ChooseStationPlace { area } => format!("Place the Station token in {}", area_label(state, *area))`; `describe_pending_choice` arm → `PendingChoice::StationPlace => "place the Station token (all players gain +1 Power in its area)".to_string()`.

- [ ] **Step 4: Run to verify pass** — targeted tests, census (record the first organic seed in the test comment), both suites, clippy.

- [ ] **Step 5: Commit** — `git add -A crates/yokohama-core/src crates/yokohama-core/tests crates/yokohama-cli/src/describe.rs && git commit -m "feat: Station token placement is a queued acquisition choice (design 4.2)"`

---

### Task 6: L2 route enumeration (engine)

**Files:**
- Modify: `crates/yokohama-core/src/game/movement.rs` (fee vector + route enumeration; refactor `calculate_movement_cost_full`)
- Modify: `crates/yokohama-core/src/game/legal_moves.rs:291-300` (swap enumeration into `generate_movement_moves_with_tech`)
- Modify: `crates/yokohama-core/src/game/apply_move.rs:341-377` (`credit_movement_president_fees` consumes the vector — single source)
- Test: `movement.rs` test module (board fixtures already exist there, tests at 640+)

**Interfaces:**
- Consumes: `find_reachable_areas_with_tech(state, player, from, reg) -> Vec<(AreaId, Vec<AreaId>)>` (movement.rs:109 — its step semantics: own-assistant areas, Postal System destination tolerance, canal-crossing edges), `calculate_movement_cost_full` (movement.rs:266 — fee/toll pricing incl. Stagecoach and the start→path[0] edge).
- Produces:
  - `pub type RouteVector = std::collections::BTreeMap<Option<PlayerId>, u32>;` (None = bank/canal tolls) in movement.rs.
  - `pub fn route_fee_vector(state: &GameState, player: PlayerId, path: &[AreaId], tech_registry: Option<&TechRegistry>, is_mercenary: bool) -> RouteVector` — the ONE pricing source; `calculate_movement_cost_full` becomes `route_fee_vector(...).values().sum()`.
  - `pub fn enumerate_president_routes(state: &GameState, player: PlayerId, from: AreaId, tech_registry: Option<&TechRegistry>) -> Vec<(AreaId, Vec<AreaId>)>` — one entry per distinct (destination, RouteVector) over simple affordable paths; lexicographically-least path per label; output sorted by (destination, vector) for determinism. Task 13 (CLI display) consumes `route_fee_vector`.

- [ ] **Step 1: Write the failing tests** (movement.rs test module; reuse the module's existing board-building fixtures)

```rust
#[test]
fn equal_cost_routes_with_different_fee_recipients_are_both_enumerated() {
    // Fixture: a board where dest D is reachable from S via
    //   route A: through area X holding opponent BLUE's president (1 yen fee to Blue)
    //   route B: across a canal crossing (1 yen toll to bank)
    // both priced 1; pre-L2 only one path per destination survived.
    // Build with this module's grid fixture helpers (see tests at movement.rs:640+).
    let routes = enumerate_president_routes(&state, p0, s, Some(&rules.techs));
    let to_d: Vec<_> = routes.iter().filter(|(d, _)| *d == dest).collect();
    assert_eq!(to_d.len(), 2, "two distinct fee vectors must both be offered: {to_d:?}");
    let vectors: Vec<RouteVector> = to_d.iter()
        .map(|(_, path)| route_fee_vector(&state, p0, path, Some(&rules.techs), false)).collect();
    assert!(vectors.contains(&BTreeMap::from([(None, 1)])), "bank-toll route offered");
    assert!(vectors.contains(&BTreeMap::from([(Some(blue), 1)])), "fee-to-Blue route offered");
}

#[test]
fn deliberately_pricier_routes_are_enumerated_when_affordable_and_pruned_when_not() {
    // Fixture: free direct route (through own assistants only) AND a 2-yen
    // detour (two canal tolls) to the same destination D.
    let mut state = /* same grid fixture as the previous test */;
    state.players[0].money = 2;
    let routes = enumerate_president_routes(&state, p0, s, Some(&rules.techs));
    let sums: Vec<u32> = routes.iter().filter(|(d, _)| *d == dest)
        .map(|(_, path)| route_fee_vector(&state, p0, path, Some(&rules.techs), false).values().sum())
        .collect();
    assert!(sums.contains(&0) && sums.contains(&2), "free AND pricier labels offered: {sums:?}");

    state.players[0].money = 0;
    let routes = enumerate_president_routes(&state, p0, s, Some(&rules.techs));
    let sums: Vec<u32> = routes.iter().filter(|(d, _)| *d == dest)
        .map(|(_, path)| route_fee_vector(&state, p0, path, Some(&rules.techs), false).values().sum())
        .collect();
    assert_eq!(sums, vec![0], "unaffordable labels pruned");
}

#[test]
fn multi_label_states_occur_organically_in_random_play() {
    // Design §7 census: within a bounded seed range, at least one random-play
    // state must offer >= 2 distinct labels to one destination (proves L2
    // enumeration fires in real games, not just fixtures). Mirror the
    // choice_encoding census loop (run_random_game_observed over seeds
    // 0..300, players cycling 2/3/4): inside the observer, when
    // state.turn_phase == TurnPhase::Movement, group the MovePresident
    // moves in get_legal_moves(state, rules) by destination and record
    // whether any destination has >= 2 entries; early-return on first hit,
    // panic with the scanned range if none found. Record the first-hit seed
    // in a comment (choice-encoding census precedent).
}

#[test]
fn route_enumeration_is_deterministic_and_canonical() {
    let a = enumerate_president_routes(&state, p0, s, Some(&rules.techs));
    let b = enumerate_president_routes(&state, p0, s, Some(&rules.techs));
    assert_eq!(a, b);
    // canonical representative: for any duplicated (dest, vector), the kept
    // path must be the lexicographically least — assert on the fixture's
    // known pair.
}

#[test]
fn cost_full_equals_vector_sum_on_random_walks() {
    // Property: for every path the old pricing produced on a few seeds,
    // calculate_movement_cost_full == route_fee_vector(...).values().sum().
}
```

Write the fixtures concretely against the module's helpers; each test must name explicit areas/owners so the reviewer can check the arithmetic by hand.

- [ ] **Step 2: Run to verify failure** — COMPILE FAIL (`enumerate_president_routes`, `route_fee_vector` missing). Paste.

- [ ] **Step 3: Implement**

1. `route_fee_vector`: port `calculate_movement_cost_full`'s logic (movement.rs:266-330 — intermediate president fees, Postal System destination stop fee, canal tolls via the full_route including the start edge, Stagecoach waiving president fees but not tolls) to accumulate into a `RouteVector` instead of a `u32`. Then rewrite `calculate_movement_cost_full` as `route_fee_vector(...).values().sum()` — deleting its duplicated logic. All existing pricing tests must stay green unchanged (they are the refactor's safety net).
2. `enumerate_president_routes`: DFS over SIMPLE paths (per-path visited set) using the exact step rule of `find_reachable_areas_with_tech` (extract its neighbor-step closure into a shared helper rather than copying it). Prune any prefix whose vector sum exceeds `state.player(player).money`. Collect `BTreeMap<(AreaId, RouteVector), Vec<AreaId>>`, keeping the lexicographically-least path per key; emit sorted. Simple paths + affordability bound termination and size (movement subgraphs are own-assistant areas — small; the close-out sweep measures reality).
3. `generate_movement_moves_with_tech` (legal_moves.rs:286-300): replace the `find_reachable_areas_with_tech` loop with `enumerate_president_routes` (the `cost <= p.money` guard stays as a belt-and-braces assert-style filter). Station/Tram teleports and `ReturnPresidentToHand` emission unchanged.
4. `credit_movement_president_fees` (apply_move.rs:341): rewrite to consume `route_fee_vector` — credit each `Some(player)` entry, ignore `None` (bank) — deleting its own fee re-derivation. Single pricing source end-to-end.
5. Keep `find_reachable_areas_with_tech` (mercenary paths and existing tests still use it); mark its doc: "single-min-cost view; president Movement enumeration uses enumerate_president_routes (L2, design §4.3)".

- [ ] **Step 4: Run to verify pass** — movement tests, both suites, clippy, AND the release fuzz gate (movement generation is on the fuzz hot path — paste before/after wall time; a modest rise is expected and must be reported, not hidden).

- [ ] **Step 5: Commit** — `git add crates/yokohama-core/src/game/movement.rs crates/yokohama-core/src/game/legal_moves.rs crates/yokohama-core/src/game/apply_move.rs && git commit -m "feat: L2 route enumeration — one move per (destination, fee vector); single pricing source"`

---### Task 7: Option B recovery — counts encoding + enumeration (engine)

**Files:**
- Modify: `crates/yokohama-core/src/types/moves.rs:78-81` (field change — APPROVED schema break)
- Modify: `crates/yokohama-core/src/game/apply_move.rs:70-75,433-450` (`apply_return_to_hand`)
- Modify: `crates/yokohama-core/src/game/legal_moves.rs:270-283,348-359` (enumeration at both emission sites)
- Modify: all fixtures the compiler flags (tests construct `recover_assistants_from`)
- Test: `legal_moves.rs` + `apply_move.rs` test modules

**Interfaces:**
- Consumes: current `ReturnPresidentToHand { recover_assistants_from: Vec<AreaId> }`; `apply_return_to_hand(state, areas: &[AreaId])` (apply_move.rs:433); the two emission sites (legal_moves.rs:281, 357).
- Produces: `ReturnPresidentToHand { recover: std::collections::BTreeMap<AreaId, u8> }` — entries are per-area counts ≥1 (absent = 0; empty map = plain retreat recovering nothing); enumeration emits the FULL cartesian product of 0..=count per occupied area (Jeremy's I5 ruling: "any number of them"); apply validates `count <= on_board[area]`. Task 8 builds the CLI grammar on this exact shape.

- [ ] **Step 1: Write the failing tests**

```rust
// legal_moves.rs tests
#[test]
fn option_b_enumerates_every_recovery_combination() {
    // Fixture: current player's president OnBoard; on_board = {A0: 2, A3: 1}.
    // Expect (2+1)*(1+1) = 6 ReturnPresidentToHand moves: {}, {A0:1}, {A0:2},
    // {A3:1}, {A0:1,A3:1}, {A0:2,A3:1}.
    let moves = get_legal_moves(&state, &rules);
    let retreats: Vec<&Move> = moves.iter()
        .filter(|m| matches!(m, Move::ReturnPresidentToHand { .. })).collect();
    assert_eq!(retreats.len(), 6, "{retreats:?}");
    assert!(retreats.iter().any(|m| matches!(m,
        Move::ReturnPresidentToHand { recover } if recover.is_empty())));
    assert!(retreats.iter().any(|m| matches!(m,
        Move::ReturnPresidentToHand { recover }
            if recover.get(&a0) == Some(&1) && recover.get(&a3) == Some(&1))));
}

// apply_move.rs tests
#[test]
fn partial_recovery_moves_exactly_the_requested_counts() {
    // on_board {A0: 2, A3: 1}, hand h. Apply recover {A0: 1}.
    // Expect: hand = h + 1, on_board {A0: 1, A3: 1}, president InHand.
}

#[test]
fn over_count_recovery_is_rejected_by_the_raw_arm() {
    // recover {A0: 3} with on_board {A0: 2} => apply_move_unchecked errors
    // (RuleViolation), state unchanged. (The checked path already rejects by
    // membership; this pins the apply-layer guard per the standing I1 lesson.)
}
```

- [ ] **Step 2: Run to verify failure** — COMPILE FAIL at the variant construction. Paste.

- [ ] **Step 3: Implement**

`types/moves.rs`:

```rust
    /// Return President to hand, recovering any number of assistants
    /// (per-area counts; empty = recover none). Owner-ruled 2026-07-19 (I5
    /// expanded wording); encoding break approved same day.
    ReturnPresidentToHand {
        recover: std::collections::BTreeMap<AreaId, u8>,
    },
```

`apply_move.rs` — rewrite `apply_return_to_hand`:

```rust
fn apply_return_to_hand(
    state: &mut GameState,
    recover: &std::collections::BTreeMap<AreaId, u8>,
) -> Result<(), MoveError> {
    let player = state.current_player;
    for (&area, &count) in recover {
        let have = state.player(player).assistants.on_board.get(&area).copied().unwrap_or(0);
        if count == 0 || count > have {
            return Err(MoveError::RuleViolation("recovery count exceeds assistants in area"));
        }
    }
    let p = state.player_mut(player);
    for (&area, &count) in recover {
        let slot = p.assistants.on_board.get_mut(&area).expect("validated above");
        *slot -= count;
        if *slot == 0 { p.assistants.on_board.remove(&area); }
        p.assistants.in_hand += count;
    }
    p.president_location = PresidentLocation::InHand;
    Ok(())
}
```

(Adapt the borrow shape to compile — validate-all-then-mutate, zero mutation on error, matching the file's house pattern.)

`legal_moves.rs` — replace BOTH single-move emissions (the InHand arm ~281 and the OnBoard arm ~357) with:

```rust
            for recover in enumerate_recovery_options(p) {
                moves.push(Move::ReturnPresidentToHand { recover });
            }
```

with, in the same file:

```rust
/// Cartesian product of 0..=count per occupied area (I5 ruling: any number).
/// Deterministic: BTreeMap area order, counts ascending. Includes the empty
/// map (recover nothing) and the all-max map (previous sole behavior).
fn enumerate_recovery_options(p: &PlayerState) -> Vec<std::collections::BTreeMap<AreaId, u8>> {
    let occupied: Vec<(AreaId, u8)> = p.assistants.on_board.iter()
        .filter(|(_, &c)| c > 0).map(|(&a, &c)| (a, c)).collect();
    let mut out = vec![std::collections::BTreeMap::new()];
    for (area, max) in occupied {
        let mut next = Vec::with_capacity(out.len() * (max as usize + 1));
        for base in &out {
            for k in 0..=max {
                let mut m = base.clone();
                if k > 0 { m.insert(area, k); }
                next.push(m);
            }
        }
        out = next;
    }
    out
}
```

Fixture sweep: the compiler and the suite will flag every constructor of the old field (`grep -rn "recover_assistants_from"`); update each to the counts map preserving the test's intent (old semantic = all-max map over the listed areas). `matches!(m, Move::ReturnPresidentToHand { .. })` patterns (e.g. legal_moves.rs:1175) survive unchanged.

- [ ] **Step 4: Run to verify pass** — targeted, both suites, clippy, release fuzz gate (branching changed; paste wall time + note the new Movement mean from the fuzz stats if printed).

- [ ] **Step 5: Commit** — `git add -A crates/yokohama-core && git commit -m "feat: Option B retreat recovers any number of assistants (counts encoding, full enumeration)"`

---

### Task 8: Option B CLI — retreat grammar + description

**Files:**
- Modify: `crates/yokohama-cli/src/repl.rs` (new `r` arm in `step`'s match at :121-161; parser; HELP_TEXT :553-576; INPUT_HINT :581)
- Modify: `crates/yokohama-cli/src/describe.rs` (ReturnPresidentToHand arm)
- Modify: `docs/cli-guide.md` (+ regenerate/patch `docs/cli-guide.html` the same way earlier tasks did)
- Test: repl.rs + describe.rs test modules

**Interfaces:**
- Consumes: Task 7's `ReturnPresidentToHand { recover: BTreeMap<AreaId, u8> }`; `parse_raw_area_token` (repl.rs:549); `find_placement_match` exact-match path (repl.rs:314).
- Produces: REPL grammar `r [<area>[:<n>]]...` — `r` alone = retreat recovering none; bare `<area>` = ALL assistants from that area; `<area>:<n>` = exactly n. Built move is exact-matched against the legal list (never bypasses the generator). Describe text used by prompts and the Task-15 log.

- [ ] **Step 1: Write the failing tests**

```rust
// repl.rs tests — house pattern (repl_2p fixture + step)
#[test]
fn r_grammar_builds_the_exact_retreat_move_and_applies_it() {
    // Drive a seed to a Movement phase where the current player has
    // on_board assistants (walk with step("0") until prompt shows Movement —
    // same walking style as the existing m-grammar tests).
    // Then: r <area>:1 must apply a ReturnPresidentToHand with that count.
    let out = r.step(&format!("r {}:1", area_tok));
    assert!(out.text.contains("President"), "{}", out.text);
    assert!(matches!(r.session.record().events.last(),
        Some(RecordedEvent::Move(Move::ReturnPresidentToHand { recover })) if recover.len() == 1));
}

#[test]
fn r_with_no_args_is_the_recover_nothing_retreat() { /* r -> empty map move applied */ }

#[test]
fn r_with_bad_count_reports_not_legal_without_applying() {
    // r <area>:9 — miss explains and event count is unchanged.
}

// describe.rs tests
#[test]
fn retreat_descriptions_show_counts_or_none() {
    // {}                => "Return President to hand (recover no assistants)"
    // {A00:2, A03:1}    => "Return President to hand, recovering 3 assistant(s) (A00 x2, A03 x1)"
}
```

- [ ] **Step 2: Run to verify failure** — `r ...` currently falls to the Unrecognized arm; describe arm doesn't exist for the new field shape (compile already fixed in Task 7 — the describe TEXT assertions fail). Paste.

- [ ] **Step 3: Implement**

`repl.rs` — add arms before the digits arm: `["r", rest @ ..] => self.try_retreat(rest),`. Parser:

```rust
/// `r [<area>[:<n>]]...` — bare area = all assistants there; :n = exactly n.
fn try_retreat(&mut self, rest: &[&str]) -> StepOutput {
    let mut recover = std::collections::BTreeMap::new();
    let state = self.session.state();
    let me = state.current_player;
    for tok in rest {
        let (area_tok, count_tok) = match tok.split_once(':') {
            Some((a, n)) => (a, Some(n)),
            None => (*tok, None),
        };
        let Some(area) = parse_raw_area_token(area_tok) else {
            return self.out(&format!("Bad area token '{area_tok}'. Usage: r [<area>[:<n>]]...\n"));
        };
        let count = match count_tok {
            Some(n) => match n.parse::<u8>() {
                Ok(k) if k > 0 => k,
                _ => return self.out(&format!("Bad count in '{tok}'.\n")),
            },
            None => state.player(me).assistants.on_board.get(&area).copied().unwrap_or(0),
        };
        recover.insert(area, count);
    }
    let candidate = Move::ReturnPresidentToHand { recover };
    match self.find_placement_match(&candidate) {
        Some(mv) => self.commit(mv),
        None => self.out("That retreat is not in the legal list (check areas/counts).\n"),
    }
}
```

(`find_placement_match` falls through to exact `==` for non-placement moves — sufficient because BTreeMap equality is canonical.) HELP_TEXT: add `  r [id[:n]]...   Movement Option B: retreat the President, recovering all (bare id) or n assistants per area ('r' alone recovers none)` and extend INPUT_HINT's command list mention. `describe.rs` ReturnPresidentToHand arm:

```rust
        Move::ReturnPresidentToHand { recover } => {
            if recover.is_empty() {
                "Return President to hand (recover no assistants)".to_string()
            } else {
                let total: u32 = recover.values().map(|&c| c as u32).sum();
                let parts = recover.iter()
                    .map(|(a, c)| format!("A{:02} x{}", a.0, c))
                    .collect::<Vec<_>>().join(", ");
                format!("Return President to hand, recovering {total} assistant(s) ({parts})")
            }
        }
```

`docs/cli-guide.md`: document the `r` grammar beside the `a`/`b`/`m` grammar section; note the numbered list still works and pagination applies. Patch the `.html` the same way previous guide tasks did (mirror edit).

- [ ] **Step 4: Run to verify pass** — CLI suite, core suite, clippy.

- [ ] **Step 5: Commit** — `git add crates/yokohama-cli/src/repl.rs crates/yokohama-cli/src/describe.rs docs/cli-guide.md docs/cli-guide.html && git commit -m "feat: retreat grammar r [id[:n]] + counted retreat descriptions"`

---

### Task 9: Head-of-queue normalization

**Files:**
- Modify: `crates/yokohama-core/src/game/choices.rs` (new `normalize_queue_head`), `crates/yokohama-core/src/game/apply_move.rs` (call at end of `apply_move_unchecked`, before the Recovery auto-run at :158)
- Modify: `crates/yokohama-core/tests/choice_encoding.rs` (invariant extension + new test)

**Interfaces:**
- Consumes: `generate_choice_resolutions` (the head-options source used by `resolve_choice_move`'s membership check, choices.rs:142), `resolve_choice_move`, the pinned two-prong invariant (Decline-bearing variants always co-list Decline).
- Produces: `pub(crate) fn normalize_queue_head(state: &mut GameState, rules: &Rules)` — loops: while the head's resolution set is exactly `[Decline*]` (a single Decline move), auto-resolve it. ONLY Decline-only heads normalize (design §4.4); a degraded-to-one Church/Customs head still prompts its single placement.

- [ ] **Step 1: Write the failing test**

```rust
// tests/choice_encoding.rs
#[test]
fn a_queued_take_that_becomes_decline_only_auto_resolves_without_prompting() {
    // Fixture: warehouse has EXACTLY 1 assistant, 0 shophouses, 0 trading
    // houses in warehouse; money covers one assistant take. Queue TWO
    // WarehouseTakes (the cross_type test at :342 shows the push pattern).
    // Resolve the first take (ChooseWarehouseTake for the assistant).
    // EXPECTED post-normalization: the queue is EMPTY (the second take became
    // {Decline} and auto-resolved) and the player state shows exactly one
    // take's effect — a true no-op for the second.
    let before_second = /* snapshot after first resolution effect fields */;
    assert!(state.pending_choices.is_empty(),
        "becomes-singleton head must auto-resolve, not prompt");
    // no second charge, no second token:
    assert_eq!(state.players[0].money, before_second.money);
}
```

Also extend the two-prong invariant driver (`push_choice_singletons_are_never_non_decline_effects`) with a becomes-singleton probe: after any resolution in its scripted scenarios, assert `state.pending_choices.first()` is never a head whose resolution set is a lone Decline.

- [ ] **Step 2: Run to verify failure** — the queue still holds the second WarehouseTake (it prompts). Paste.

- [ ] **Step 3: Implement** in choices.rs:

```rust
/// Design §4.4: after an apply, heads whose ONLY resolution is a Decline are
/// dead questions — resolve them silently (structural no-ops by the pinned
/// two-prong invariant). Loops down the queue. Deterministic pure-state fn,
/// so replays re-derive identical normalization.
pub(crate) fn normalize_queue_head(state: &mut GameState, rules: &Rules) {
    loop {
        let Some(head) = state.pending_choices.first().cloned() else { return };
        let opts = generate_choice_resolutions_for(state, &head, rules);
        let lone_decline = opts.len() == 1
            && matches!(opts[0], Move::DeclineWarehouseTake | Move::DeclineAssistantMove);
        if !lone_decline { return; }
        let mv = opts.into_iter().next().unwrap();
        resolve_choice_move(state, &mv, rules)
            .expect("lone-decline resolution is legal by construction");
    }
}
```

Call it in `apply_move_unchecked` immediately after the `match mv { ... }` block and BEFORE the Recovery auto-run gate (:158), so a queue emptied by normalization still triggers Recovery in the same apply. (Use the head-position resolution generator consistently: if `generate_choice_resolutions_for` computes "as if head", it is the right one here since the choice IS head.)

- [ ] **Step 4: Run to verify pass** — choice_encoding suite, both suites, clippy, release fuzz gate (queue semantics changed — paste time).

- [ ] **Step 5: Commit** — `git add crates/yokohama-core/src/game/choices.rs crates/yokohama-core/src/game/apply_move.rs crates/yokohama-core/tests/choice_encoding.rs && git commit -m "feat: normalize queue heads that become Decline-only (design 4.4)"`

---

### Task 10: Winner / tie-break API + CLI announcement

**Files:**
- Modify: `crates/yokohama-core/src/game/scoring.rs` (breakdown restructure + `final_result`)
- Modify: `crates/yokohama-cli/src/session.rs` (accessor), `crates/yokohama-cli/src/repl.rs` (`score_text` at game over)
- Test: scoring.rs tests + repl.rs tests

**Interfaces:**
- Consumes: `calculate_final_scores(state, order_registry, tech_registry) -> Vec<(PlayerId, u32)>` (scoring.rs:8-12) and its private helpers (church/customs/tech/country/remainder); `state.config.start_player: PlayerId` (state.rs:16); the clockwise-distance precedent at scoring.rs:141.
- Produces (in scoring.rs, re-exported via game/mod.rs):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreBreakdown {
    pub base_vp: u32,        // p.victory_points at game end (incl. achievements)
    pub church: u32,
    pub customs: u32,
    pub technology: u32,
    pub country: u32,
    pub remainder: u32,      // mercenaries/imports/money/goods conversions
    pub total: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing { pub player: PlayerId, pub breakdown: ScoreBreakdown, pub rank: u8 }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameResult { pub standings: Vec<Standing>, pub winner: PlayerId }

pub fn final_result(state: &GameState, rules: &Rules) -> GameResult
```

Official tie-break (KB 01-rules-reference.md:556): highest total; ties broken by clockwise closeness to the start player — `distance = (seat + n - start) % n`, distance 0 (the start player) closest. Ranks 1..=n, never shared. `calculate_final_scores` becomes a thin wrapper (`standings → (player, total)`) so existing callers/tests stay green. This is the bot phase's terminal utility.

- [ ] **Step 1: Write the failing tests**

```rust
// scoring.rs tests
#[test]
fn final_result_breaks_ties_clockwise_from_start_player() {
    // Fixture: 3p, engineered equal totals for players 1 and 2 (set raw
    // victory_points equal, empty everything else), start_player = 2.
    // distance: p2 = 0, p0 = 1, p1 = 2 → tied p1/p2 resolve p2 first.
    let r = final_result(&state, &rules);
    assert_eq!(r.standings.iter().map(|s| s.rank).collect::<Vec<_>>().len(),
               state.players.len());
    assert_eq!(r.winner, PlayerId(2));
    let ranks: Vec<(PlayerId, u8)> = r.standings.iter().map(|s| (s.player, s.rank)).collect();
    // p2 outranks p1 despite equal totals; no shared ranks.
}

#[test]
fn breakdown_components_sum_to_total_and_wrapper_matches() {
    let r = final_result(&state, &rules);
    for s in &r.standings {
        let b = &s.breakdown;
        assert_eq!(b.base_vp + b.church + b.customs + b.technology + b.country + b.remainder, b.total);
    }
    let flat = calculate_final_scores(&state, &rules.orders, &rules.techs);
    for s in &r.standings {
        assert!(flat.contains(&(s.player, s.breakdown.total)));
    }
}

// repl.rs tests
#[test]
fn game_over_screen_announces_winner_and_breakdown() {
    // Use the existing game-over fixture path (the dedicated GameOver arm
    // landed at a6faa87 — its test shows how to reach a finished state).
    let out = r.step("score");
    assert!(out.text.contains("WINNER"), "{}", out.text);
    assert!(out.text.contains("church"), "breakdown lines expected: {}", out.text);
}
```

- [ ] **Step 2: Run to verify failure** — COMPILE FAIL (`final_result` missing). Paste.

- [ ] **Step 3: Implement** — restructure `calculate_final_scores`'s body into a `fn breakdowns(state, orders, techs) -> Vec<(PlayerId, ScoreBreakdown)>` that captures each helper's contribution per player instead of folding into one `u32` (church/customs/tech helpers already return per-player vecs; country + remainder are per-player calls; base_vp = `p.victory_points`). Then:

```rust
pub fn final_result(state: &GameState, rules: &Rules) -> GameResult {
    let n = state.players.len() as u32;
    let start = state.config.start_player.0 as u32;
    let mut rows = breakdowns(state, &rules.orders, &rules.techs);
    rows.sort_by_key(|(p, b)| {
        let dist = (p.0 as u32 + n - start) % n;
        (std::cmp::Reverse(b.total), dist)
    });
    let standings = rows.into_iter().enumerate()
        .map(|(i, (player, breakdown))| Standing { player, breakdown, rank: (i + 1) as u8 })
        .collect::<Vec<_>>();
    GameResult { winner: standings[0].player, standings }
}

pub fn calculate_final_scores(state: &GameState, order_registry: &OrderRegistry, tech_registry: &TechRegistry) -> Vec<(PlayerId, u32)> {
    breakdowns(state, order_registry, tech_registry).into_iter()
        .map(|(p, b)| (p, b.total)).collect()
}
```

CLI: `Session::final_result(&self) -> GameResult` accessor; `score_text` when `is_over()` renders `WINNER: {color}` then one line per standing: `#{rank} {color}: {total} VP (base {..} + church {..} + customs {..} + tech {..} + country {..} + remainder {..})` using `render::color_name`.

- [ ] **Step 4: Run to verify pass** — both suites, clippy.

- [ ] **Step 5: Commit** — `git add crates/yokohama-core/src/game/scoring.rs crates/yokohama-core/src/game/mod.rs crates/yokohama-cli/src/session.rs crates/yokohama-cli/src/repl.rs && git commit -m "feat: final_result with official clockwise tie-break; CLI announces winner + breakdown"`

---

### Task 11: Status panel (T2.1)

**Files:**
- Modify: `crates/yokohama-cli/src/render.rs:106-135` (`players_panel`)
- Test: render.rs tests

**Interfaces:**
- Consumes: `PlayerState` fields — `assistants.{in_hand, in_warehouse, on_board}`, `shophouses.{in_hand, in_warehouse, built}`, `trading_houses.{in_hand, in_warehouse, built}` + `TradingHouseState::next_cost()` (player.rs:133); existing panel line shapes.
- Produces: the enriched panel every playtest reads. Exact new lines, appended per player after the existing `tokens:` line:

```
  assistants: hand N  board N  warehouse N
  shophouses: hand N  warehouse N  built N
  trading:    hand N  warehouse N  built N  (next take 4-7 yen)
```

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn players_panel_shows_the_full_economy() {
    let (state, rules) = panel_fixture(); // module's existing fixture
    let text = players_panel(&state, &rules);
    assert!(text.contains("assistants: hand"), "{text}");
    assert!(text.contains("shophouses: hand"), "{text}");
    assert!(text.contains("trading:"), "{text}");
    assert!(text.contains("next take"), "{text}");
    // spot-check real numbers from the fixture, e.g.:
    // assert!(text.contains("assistants: hand 3  board 0  warehouse 17"));
}
```

- [ ] **Step 2: Run to verify failure** — assertion FAIL (lines absent). Paste.

- [ ] **Step 3: Implement** — extend `players_panel` with the three lines; `board N` = `p.assistants.on_board.values().map(|&c| c as u32).sum::<u32>()`; trading line ends `format!("(next take {} yen)", p.trading_houses.next_cost())`. Keep existing lines byte-identical (other tests assert on them).

- [ ] **Step 4: Run to verify pass** — CLI suite, clippy.

- [ ] **Step 5: Commit** — `git add crates/yokohama-cli/src/render.rs && git commit -m "feat: players panel shows assistants and building tokens by zone (T2.1)"`

---

### Task 12: Rewards where decisions happen (T2.2a) + guide alignment

**Files:**
- Modify: `crates/yokohama-cli/src/describe.rs` (BuildShophouse/BuildTradingHouse arms :268-270; production-area arms of `describe_area_action` :129-217)
- Modify: `crates/yokohama-cli/src/repl.rs` (new `sites` command + HELP_TEXT), `crates/yokohama-cli/src/render.rs` (new `sites_view`)
- Consume (already exists, zero callers): `crates/yokohama-core/src/game/display.rs:198 site_compact_line(index, site) -> String`
- Modify: `docs/cli-guide.md:232-234` (+ `.html`): the site-line docs become true; the stale church-dummy "Known display gap" box is DELETED (tracks_view has marked the 2p faith-7 dummy with `X` since T12 of choice encoding — render.rs:143-171)
- Test: describe.rs + render.rs + repl.rs tests

**Interfaces:**
- Consumes: how a board area links to its `BuildingSite` + slot rewards (follow `area_cell_lines`'s existing site lookup, render.rs:550-587) and `state.current_action_power`.
- Produces: `render::sites_view(state, rules) -> String` (one `site_compact_line` per area with a site, prefixed by the area label + built/ownership marks); building prompts that print the slot's printed reward instead of bare "space N"; production-area action descriptions with a power preview.

- [ ] **Step 1: Write the failing tests**

```rust
// describe.rs
#[test]
fn build_shophouse_options_print_the_spaces_printed_reward() {
    // fixture state where BuildShophouse { slot } is legal for a known site:
    let text = describe_move(&Move::BuildShophouse { slot: 0 }, &state, &rules);
    assert!(!text.ends_with("(space 1)"), "bare space-number text must be gone: {text}");
    assert!(text.contains("1vp") || text.contains("VP"), "printed reward expected: {text}");
}

#[test]
fn production_area_actions_preview_the_yield() {
    // e.g. Copper Mine with current power 3:
    let text = describe_area_action(&action, &state, &rules);
    assert!(text.contains("power 3"), "{text}");
    assert!(text.contains("+3"), "{text}");
}

// render.rs
#[test]
fn sites_view_renders_a_compact_line_per_sited_area() {
    let text = sites_view(&state, &rules);
    assert!(text.contains("S"), "{text}");
    assert!(text.lines().count() >= 1);
}
```

(The `describe_area_action` signature currently discards state as `_state` — this task starts using it; adjust the underscore.)

- [ ] **Step 2: Run to verify failure** — asserts FAIL / compile FAIL for `sites_view`. Paste.

- [ ] **Step 3: Implement** — wire `yokohama_core::game::display::site_compact_line` (export it from core's `game/mod.rs` if not already `pub use`d) into: (a) a new `sites_view` in render.rs iterating areas-with-sites in AreaId order; (b) the BuildShophouse arm — look up the acting area's site + the slot's printed reward text (reuse the reward-formatting helpers `site_compact_line` uses; extract a `slot_reward_text(site, slot)` in core display.rs if needed so the CLI never re-derives reward semantics). (c) Production arms gain `(power {p} → +{p} {good})` using `state.current_action_power` when the state carries it for the acting area (fall back to the current text when power is not yet set). Add the `sites` REPL command + HELP_TEXT line + guide updates (make :232 true, delete the :234 warning box, document `sites`).

- [ ] **Step 4: Run to verify pass** — both CLI suites green, clippy.

- [ ] **Step 5: Commit** — `git add crates/yokohama-cli/src crates/yokohama-core/src/game/display.rs crates/yokohama-core/src/game/mod.rs docs/cli-guide.md docs/cli-guide.html && git commit -m "feat: printed rewards at decision points + sites view; guide aligned (T2.2a)"`

---

### Task 13: Movement option display (T2.2b)

**Files:**
- Modify: `crates/yokohama-cli/src/describe.rs` (MovePresident arm)
- Test: describe.rs tests

**Interfaces:**
- Consumes: `route_fee_vector` (Task 6) via the acting player + `mv.path`; `describe::area_label`.
- Produces: movement options that read `Move President to Chinatown (A05) — free` / `— pay 1 yen (canal toll, bank)` / `— pay 2 yen (1 to Blue, 1 canal toll)`. The design's destination-first sub-prompt is NOT built: post-L2 lists stay modest and pagination + explicit text covers recordability; noted as a contingency in the close-out if sweeps show typical Movement lists > 20 (documented deviation, design §4.3 note).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn movement_descriptions_name_cost_and_recipients() {
    // Canal fixture (reuse Task 6's) — the toll route:
    let text = describe_move(&toll_move, &state, &rules);
    assert!(text.contains("canal toll"), "{text}");
    // the fee route:
    let text2 = describe_move(&fee_move, &state, &rules);
    assert!(text2.contains("1 to "), "{text2}");
    // a free move:
    assert!(describe_move(&free_move, &state, &rules).contains("free"));
}
```

- [ ] **Step 2: Run to verify failure** — asserts FAIL. Paste.

- [ ] **Step 3: Implement** — in the MovePresident arm: empty path → keep the existing teleport wording; else compute `route_fee_vector(state, state.current_player, path-with-destination-as-the-arm-already-shapes-it, Some(&rules.techs), false)`, then render `free` when the vector is empty, else `pay {sum} yen ({parts})` with parts = `"{n} to {color}"` per player entry + `"{n} canal toll"` for the bank entry. Match the arm's existing path/destination composition exactly (read it first — the vector function takes the same `path` slice `apply_movement` prices).

- [ ] **Step 4: Run to verify pass** — CLI suite, clippy.

- [ ] **Step 5: Commit** — `git add crates/yokohama-cli/src/describe.rs && git commit -m "feat: movement options print cost and fee recipients (T2.2b)"`

---

### Task 14: Achievements view (T2.3)

**Files:**
- Modify: `crates/yokohama-core/src/game/achievements.rs` (public progress helper), `crates/yokohama-cli/src/render.rs` (`achievements_view`), `crates/yokohama-cli/src/repl.rs` (`goals` command + HELP_TEXT), `docs/cli-guide.md`/`.html`
- Test: achievements.rs + render.rs tests

**Interfaces:**
- Consumes: `state.achievements: [AchievementCard; 3]` (state.rs:230), `can_claim_achievement` (achievements.rs:6), the private counters (`count_production_buildings`:60, `count_commercial_buildings`:87), `describe::achievement_desc`.
- Produces: `pub fn achievement_progress(state: &GameState, player: PlayerId, achievement: AchievementType) -> (u32, u32)` (current, needed) in core — the SAME arithmetic `can_claim_achievement` uses, extracted so the two can't drift (refactor `can_claim_achievement` to compare `progress.0 >= progress.1`); `render::achievements_view(state, rules)`; REPL `goals`.

- [ ] **Step 1: Write the failing tests**

```rust
// achievements.rs
#[test]
fn progress_matches_the_claim_gate() {
    // For every achievement type in the fixture's three cards and every player:
    let (cur, need) = achievement_progress(&state, p, ach);
    assert_eq!(cur >= need, can_claim_achievement(&state, p, ach));
}

// render.rs
#[test]
fn achievements_view_lists_condition_claim_state_and_progress() {
    let text = achievements_view(&state, &rules);
    assert_eq!(text.lines().filter(|l| l.starts_with("G")).count(), 3, "{text}");
    assert!(text.contains("/"), "progress as cur/need expected: {text}");
}
```

- [ ] **Step 2: Run to verify failure** — COMPILE FAIL (`achievement_progress` missing). Paste.

- [ ] **Step 3: Implement** — extract each `match` arm of `can_claim_achievement` into `(current, needed)` pairs returned by `achievement_progress`; reimplement `can_claim_achievement` as the `>=` comparison. `achievements_view`: per card `G{i}: {achievement_desc} — {claimed by Color | open}` then one line per player `   {color}: {cur}/{need}`. Claim state comes from the `AchievementCard` field that `claim_achievement` writes (read the struct; use whatever field marks the claimant). REPL `goals` arm + HELP_TEXT + guide note.

- [ ] **Step 4: Run to verify pass** — both suites, clippy.

- [ ] **Step 5: Commit** — `git add crates/yokohama-core/src/game/achievements.rs crates/yokohama-cli/src/render.rs crates/yokohama-cli/src/repl.rs docs/cli-guide.md docs/cli-guide.html && git commit -m "feat: achievements view with live progress (T2.3); claim gate shares the progress arithmetic"`

---

### Task 15: Log historical fidelity (T2.4)

**Files:**
- Modify: `crates/yokohama-cli/src/session.rs` (description cache), `crates/yokohama-cli/src/repl.rs:495-518` (`log_tail`/`describe_event` read the cache)
- Test: session.rs + repl.rs tests

**Interfaces:**
- Consumes: `Session::apply` (the pre-commit `self.state` IS the historical state for the incoming move), `undo` (truncates events), `load` (full record), `replay_from` + `rebuild_initial_state`-equivalent already used by `Session::load`; `describe::describe_move`.
- Produces: `Session.event_descriptions: Vec<String>` — one entry per `record.events` entry, always in lockstep (assert in debug); `pub fn event_description(&self, i: usize) -> &str`. `log_tail` renders from the cache, so state-dependent text (TH take price, area power) is correct for WHEN IT HAPPENED. Not serialized.

- [ ] **Step 1: Write the failing test**

```rust
// session.rs tests
#[test]
fn log_descriptions_are_historical_not_current() {
    // Drive a sim session through a warehouse trading-house take at printed
    // cost 4, then advance until trading_houses.next_cost() != 4.
    // The cached description of the old take must still say the OLD cost.
    let idx = /* event index of the take, captured when it was applied */;
    assert!(s.event_description(idx).contains('4'), "{}", s.event_description(idx));
    let taker = /* the fixture's taking player id, known at fixture time */;
    assert_ne!(s.state().player(taker).trading_houses.next_cost(), 4,
        "fixture must have advanced the progression, else this test proves nothing");
}

#[test]
fn descriptions_stay_in_lockstep_across_apply_undo_load() {
    // after each operation: event_descriptions.len() == record().events.len()
}
```

- [ ] **Step 2: Run to verify failure** — COMPILE FAIL (cache API missing). Paste.

- [ ] **Step 3: Implement** — in `apply`: push descriptions computed against the PRE-commit states — reveals described statically (reuse `describe_event`'s reveal wording, moved into a free fn `describe_recorded_event(ev, state_at_that_point, rules)` in describe.rs), the move described against `staged` (the post-reveal, pre-move state). In `undo`: truncate to the new event count. In `load`: rebuild by folding — start from the same base state `load` already reconstructs, then for each event: describe against the current fold state, then `replay_from(state, slice_of_one, rules)` (the record_stats pattern, examples/record_stats.rs:71). `log_tail`'s `describe_event` call is replaced by `self.session.event_description(start + i)`; delete the now-unused current-state `describe_event` path (keep the reveal formatting fn).

- [ ] **Step 4: Run to verify pass** — CLI suite, clippy. Manually sanity-check `log` output in one piped sim run and paste it in the task report.

- [ ] **Step 5: Commit** — `git add crates/yokohama-cli/src/session.rs crates/yokohama-cli/src/repl.rs crates/yokohama-cli/src/describe.rs && git commit -m "feat: historical log descriptions via per-event fold cache (T2.4)"`

---

### Task 16: Semantic color + board art (T2.5)

**Files:**
- Create: `crates/yokohama-cli/src/style.rs`
- Modify: `crates/yokohama-cli/src/lib.rs` (module), `crates/yokohama-cli/src/render.rs` (panels/board/prompt/delta call sites), `crates/yokohama-cli/src/main.rs` (init)
- Test: style.rs + render.rs tests

**Interfaces:**
- Consumes: `std::io::IsTerminal` (std, no new deps), `NO_COLOR` env convention, `color_name`/`color_letter` (render.rs).
- Produces:

```rust
// style.rs
pub enum Tone { Player(PlayerColor), Good(GoodKind), Vp, Warn, Prompt, Dim }
pub fn init_from_tty();              // main() calls once: enabled = stdout tty && NO_COLOR unset
pub fn force(enabled: bool);         // tests
pub fn paint(tone: Tone, s: &str) -> String;  // wraps in ANSI when enabled, else returns s unchanged
```

Default state is DISABLED (so every existing test and piped run stays byte-identical without calling anything). Board grid upgraded to box-drawing (`┌─┬┐│└┴┘┼`) rulers in `board_grid` — same cell content, prettier frame.

- [ ] **Step 1: Write the failing tests**

```rust
// style.rs
#[test]
fn paint_is_identity_when_disabled_and_wraps_when_forced() {
    force(false);
    assert_eq!(paint(Tone::Vp, "5 VP"), "5 VP");
    force(true);
    let s = paint(Tone::Vp, "5 VP");
    assert!(s.starts_with("\u{1b}[") && s.ends_with("\u{1b}[0m") && s.contains("5 VP"));
    force(false);
}

// render.rs
#[test]
fn colored_players_panel_still_contains_the_plain_text() {
    force(true);
    let colored = players_panel(&state, &rules);
    force(false);
    let plain = players_panel(&state, &rules);
    assert_eq!(strip_ansi(&colored), plain); // strip_ansi: tiny test helper removing \x1b[..m runs
}

#[test]
fn board_grid_uses_box_drawing_rulers() {
    assert!(board_grid(&state).contains('─'));
}
```

- [ ] **Step 2: Run to verify failure** — COMPILE FAIL (`style` missing). Paste.

- [ ] **Step 3: Implement** — `style.rs` with a `static ENABLED: AtomicBool = AtomicBool::new(false)`; `init_from_tty()` sets it via `std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()`; SGR codes: players → their ANSI base color by `PlayerColor` (red 31, blue 34, green 32, yellow 33, purple/other 35), goods copper 33 / silk 37 / tea 32 / seafood 36 / yen 93, Vp 1;97, Warn 1;31, Prompt 1, Dim 2. Apply `paint` at: player color names in `players_panel` + delta lines' player prefixes, president/assistant letters in `area_cell_lines`, the prompt header phase line (Prompt), desync/lockdown banners (Warn), VP totals in score/game-over text. Swap `board_grid`'s `-` rulers for box-drawing (adjust the ruler builder; `fit_cell` width unchanged). `main.rs` calls `style::init_from_tty()` first thing.

- [ ] **Step 4: Run to verify pass** — CLI suite (all existing output tests must pass UNTOUCHED — that is the disabled-default proof), clippy. Run the CLI interactively once (`cargo run -p yokohama-cli`) and eyeball color; note it in the report.

- [ ] **Step 5: Commit** — `git add crates/yokohama-cli/src && git commit -m "feat: TTY-gated semantic color + box-drawing board (T2.5)"`

---

### Task 17: Hygiene + ruling pins (Tier 3)

**Files:**
- Modify: `crates/yokohama-core/src/types/cards.rs:114` (DELETE `base_industry_cost`)
- Test: `crates/yokohama-core/src/game/legal_moves.rs` tests (Tram pin), `crates/yokohama-core/tests/choice_encoding.rs` (Language School pin)

**Interfaces:**
- Consumes: rulings recorded 2026-07-19 (guidingpath decision log): Tram board-only; Language School Yen-first; `TradingHouseState::take_cost` (player.rs:116); `language_school_yen_bonus` credit site (apply_move.rs:1064-1070). Already-pinned (audited, no new test needed): no-return-to-start (`test_cannot_return_to_start_area`, apply_move.rs:2314), place-0 (`test_phase_transitions`, tests/integration.rs:173).
- Produces: two pin tests + a deleted footgun.

- [ ] **Step 1: Write the failing/pinning tests**

```rust
// legal_moves.rs tests — Tram is BOARD-ONLY (owner ruling 2026-07-19)
#[test]
fn tram_offers_nothing_from_hand() {
    // Fixture: current player owns Tram, president InHand, an area exists
    // with no own assistants and no opponent president.
    let moves = get_legal_moves(&state, &rules);
    assert!(
        !moves.iter().any(|m| matches!(m, Move::PlacePresident { destination } if *destination == assistantless_area)),
        "from hand, Tram must NOT unlock assistant-free placement (board-only ruling)"
    );
}

// tests/choice_encoding.rs — Language School Yen funds a same-order take
#[test]
fn language_school_yen_is_available_for_the_same_orders_warehouse_take() {
    // Fixture: player owns Language School, money = 3, warehouse has a
    // trading house, trading_houses.next_cost() == 4; fulfill an order whose
    // rewards include a warehouse-take icon (the warehouse_take test at :103
    // shows the fixture shape). After the fulfill move (money 3+1=4), the
    // queued WarehouseTake's resolutions MUST include the 4-yen trading-house
    // take.
    let legal = get_legal_moves(&state_after_fulfill, &rules);
    assert!(legal.iter().any(|m| matches!(m, Move::ChooseWarehouseTake { .. } /* trading-house option */)),
        "the fresh yen must count toward take affordability: {legal:?}");
}
```

(Both may pass immediately — they are PINS; the RED step for pins is demonstrated by mutating the guarded logic locally and watching them fail, per house convention. Show that transcript.)

- [ ] **Step 2: Run to verify** — run both; paste output; for each, paste the mutate-to-RED demonstration (e.g. temporarily make the InHand arm emit Tram placements / temporarily compute take options against pre-fulfill money).

- [ ] **Step 3: Delete `base_industry_cost`** (types/cards.rs:114 whole method + doc comment). Fix any test the compiler flags (grep says the only hits are the definition; if a test uses it, delete that test — it tested fiction values).

- [ ] **Step 4: Run to verify pass** — both suites, clippy.

- [ ] **Step 5: Commit** — `git add crates/yokohama-core/src/types/cards.rs crates/yokohama-core/src/game/legal_moves.rs crates/yokohama-core/tests/choice_encoding.rs && git commit -m "chore: Tram board-only + Language School yen-first pins; delete base_industry_cost fiction"`

---

### Task 18: Close-out — sweeps, baselines, census, docs

**Files:**
- Modify: `crates/yokohama-core/examples/choice_metrics/mod.rs:49-50` (`ALL_LABELS` 6→7 with "StationPlace")
- Modify: `guidingpath.md` (Current state + close-out decision-log row), `CLAUDE.md` (suite counts + any newly-stale line), `docs/cli-guide.md`/`.html` (final sweep for stale text)
- No new code.

**Interfaces:**
- Consumes: everything landed; the selfplay stats surface (`examples/selfplay.rs:82-97`), the census (choice_encoding.rs), the release fuzz gate.

- [ ] **Step 1: Full gates.** Run and paste ALL of:
  - `cargo test -p yokohama-core` and `cargo test -p yokohama-cli` (record the new totals; baseline was 466 + 85)
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored` (wall time vs the ~8s baseline)
  - `cargo run --release -p yokohama-core --example selfplay -- 5000 2 0`, `... 5000 3 0`, `... 5000 4 0`

- [ ] **Step 2: New baselines.** Record in guidingpath's Current state: moves/sec, mean game length, per-phase branching — **Movement's mean WILL be visibly higher than 3.2-3.7 (L2 + Option B; design flag B: expected, by design, the options were always legal at the table)** — prompts/game incl. StationPlace, and the first organic Station seed. If Movement's mean options exceed ~20 (a full prompt page), note the design's destination-first sub-prompt contingency as recommended next-CLI work.

- [ ] **Step 3: Docs true-up.**
  - guidingpath: Current state section gains the plan's close-out block (gates, counts, baselines); ONE decision-log close row (per-task detail stays in the SDD ledger).
  - CLAUDE.md: update test counts; add `apply_move` (checked) vs `apply_move_unchecked` to the architecture bullet; note `replay_strict` + `rules_hash` for training records; note the `r` retreat grammar and new REPL commands (`sites`, `goals`) in whatever line lists CLI surfaces.
  - cli-guide: verify every grammar/command added in Tasks 8/12/14 is documented; delete any line the plan made stale.

- [ ] **Step 4: Commit** — `git add guidingpath.md CLAUDE.md docs/cli-guide.md docs/cli-guide.html crates/yokohama-core/examples/choice_metrics/mod.rs && git commit -m "docs: engine-authority close-out — new baselines, census, guide true-up"`

- [ ] **Step 5: Final whole-branch review** (fable) per the SDD protocol vs merge-base master; fix wave; then the finishing-a-development-branch options go to Jeremy. The release fuzz gate must be green at the reviewed commit. The live gate game remains Jeremy's, after merge.

---

## Execution protocol reminders (proven, from the ledger)

- Sonnet implementers + per-task reviewers; fable ONLY for the final whole-branch review. Record BASE (commit SHA) before each dispatch; demand pasted RED and genuine `Compiling`/test transcripts; reviewers who idle get a SendMessage nudge; ONE fix subagent per findings list.
- Any state machinery `get_legal_moves` protects needs its `apply_move`-layer guard in the same task (standing final-review I1 lesson) — Tasks 5, 7, 9 each carry apply-layer validation for exactly this reason.
- Task order is the written order (1→18). Tasks 3–4 depend on 1; 13 depends on 6; 8 depends on 7; nothing else crosses.
