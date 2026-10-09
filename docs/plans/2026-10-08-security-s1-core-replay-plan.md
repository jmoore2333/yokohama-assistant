# Security S1, the core PR — the predicate replay check: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. Every code task subagent-driven, the implementer's tier in each task's title — **Opus** for
> Tasks 1–3 — every task reviewer **Fable**, and a **Mythos** orchestrator that conducts the plan review at the
> top and the whole-branch review at the end ITSELF, on exports (the owner's final shape, 2026-10-08: "opus
> implementers, and fable reviewers, dispatched by a mythos orchestrator that also conducts the overall plan review
> at the top and end"); the gates and the PR are the orchestrator's too (Task 4). Steps use checkbox
> (`- [ ]`) syntax. **Read the spec first** (`docs/plans/2026-10-08-security-remediation-spec.md`: §1 R-SEC1 and
> R-SEC2, §3.1 the core PR, §6 the fences), then this plan's §"Names shared across tasks" — an implementer sees only
> its own task, and that section is how it learns what its neighbours produce. Every `file:line` here was read at
> `d0816172`; **find by content once an earlier task has moved a file.** The implementer PROBES the brief's code
> before its own review and reports `DONE_WITH_CONCERNS` when a sentence here proves false.

**Goal:** The engine's strict replay gains an ADDITIVE, PROVEN-EQUIVALENT way to check each recorded move without
materializing a legal list per event, so that the web layer (in S1 proper) can replay an untrusted record at a cost
that no longer follows the legal families' sizes — with `SEMANTICS_REV` at 7, the oracle untouched, and
`apply_move`, `get_legal_moves`, `replay_strict` and `fold_one_event` unedited.

**Architecture:** Two additions to `crates/yokohama-core` and one proof. `move_legality::is_listed_move` answers
"is this move IN `get_legal_moves`" for one candidate: for the Movement family's two variants it is the conjunction
of `is_move_legal(.., PathMode::Canonical)` (the phase and queue gates) and membership in the family's own exact,
small, compute-per-call list `route_enum::movement_routes_family` — because `is_move_legal` deliberately accepts any
legal WITNESS path for a label the list prints one lex-least witness for, the family's list is the only exact check;
for every other variant `is_move_legal` is exact by the module's contract. `record::replay_strict_with(record,
rules, MoveCheck)` is `replay_strict`'s preamble, loop and digest check with the move check chosen by the caller:
`Listed` folds through `apply_move` exactly as today, `Predicate` through `is_listed_move` then
`apply_move_unchecked`. The proof is two module cells (every listed move passes; every constructed candidate, the
non-canonical witnesses included, reads as the list reads it; the phase gate is taken first) and one integration
binary (24 recorded games fold to the same digest; five tampered records and a battery of witness tampers refuse at
the same index with the same sentence).

**Tech Stack:** Rust 1.97.1 (`rust-toolchain.toml`), `crates/yokohama-core` only; the crate's own test helpers
(`move_legality.rs`'s `drive`/`candidates`/`legal`, `tests/common/mod.rs`'s `run_recorded_game`); `cargo test`,
`cargo clippy --workspace --all-targets -- -D warnings`.

**Spec:** `docs/plans/2026-10-08-security-remediation-spec.md` (§3.1 is this PR; R-SEC1, R-SEC2; §6 the fences).

## Global Constraints

- `SEMANTICS_REV` (`crates/yokohama-core/src/game/semantics.rs`) stays **7**. Nothing here changes what any record
  means, which is the claim the proof cells carry.
- `crates/yokohama-oracle` is untouched (the ORACLE INDEPENDENCE RULE).
- No edit to `apply_move`, `apply_move_unchecked`, `get_legal_moves`, `replay_strict`, `fold_one_event`,
  `rebuild_initial_state` or any existing test. Additions only, in two files, plus one new test binary and one
  sentence in `CLAUDE.md`.
- The new test binary is **per-edit** (no `#[ignore]`): it runs under `cargo test -p yokohama-core` and adds no
  sixteenth release gate (spec §7).
- The wire corpus (`web/src/wire/`) changes by **zero bytes**: `cargo test -p yokohama-web --test wire_fixtures`
  green without `UPDATE_WIRE_FIXTURES`.
- TDD: the failing test first, then the code, then green; `cargo clippy --workspace --all-targets -- -D warnings`
  clean after every task; no `cargo fmt`; stage with explicit paths, never `git add -A`.
- A refusal's sentence under `Predicate` is `apply_move`'s own: a non-member is `MoveError::NotLegal`
  (`crates/yokohama-core/src/types/errors.rs:17`, Display at `:33`), so the two checks word a refusal identically.
- The branch is `security-s1-core-replay` from `master` at `d0816172`; one PR; the owner's word to merge.

## Review Focus

Five inputs the spec implies and this plan pins, each to the task that owns the code:

1. **A bare teleport encoding (`path: []`) for a destination a zero-cost walk also reaches.** The list prints only
   the walk (the generator's suppression, `legal_moves.rs:289-300`); `is_listed_move` must refuse the teleport
   encoding though `PathMode::RulesLegal` accepts it. Task 1's membership cell meets it through `candidates()`'s
   "bare teleport encoding for a walked destination" mutant.
2. **A President in hand.** The family's list is `PlacePresident` over areas there, not `MovePresident`; Task 1's
   membership cell meets it in every game's opening decisions, and its phase cell checks hand placements one state
   later.
3. **A `MovePresident` while a choice is queued.** `get_legal_moves` short-circuits to the choice's resolutions;
   `is_move_legal` mirrors that, and the family's list does not — the conjunction's order is what refuses it. Task
   1's phase cell meets it wherever a listed move's successor state queues a choice.
4. **A record with no events.** Both replays must agree on the initial state's digest. Task 2's cell
   `an_empty_record_folds_to_the_same_initial_state_under_both_checks`.
5. **A record every event of which applies but whose state breaks an invariant.** The invariant check must run under
   `Predicate` exactly as under `Listed`. Task 2's tamper cell, case (g).

## Execution shape

- Tasks 1–3: implementer **Opus**, reviewer **Fable** (the owner's final shape, 2026-10-08). Task 4: the
  orchestrator, **Mythos**, who conducts the "before" plan review and the "after" whole-branch review itself, on a
  `git archive` export, never in the implementer's tree.
- Every dispatched agent: full report to a file, a three-line reply; no `rm -rf`, `rm -r` or bulk `rm -f`; no
  Monitor, no event waits; long commands in the foreground or polled.
- The plan review (Task 4, step 1) EXECUTES Tasks 1–2's cells in an export before any implementation: a cell that
  cannot go red on the mutant it names is a plan defect, fixed here first (spec §9).

## Names shared across tasks

| name | where | meaning |
|---|---|---|
| `is_listed_move(state: &GameState, mv: &Move, rules: &Rules) -> bool` | `crates/yokohama-core/src/game/move_legality.rs`, `pub`, re-exported at the crate root through `pub use move_legality::*` (`game/mod.rs:144`) | exact membership in `get_legal_moves(state, rules)` for one candidate (Task 1) |
| `MoveCheck { Listed, Predicate }` | `crates/yokohama-core/src/game/record.rs`, `pub`, re-exported through `pub use record::*` (`game/mod.rs:136`) | which move check a replay folds with (Task 2) |
| `replay_strict_with(record: &GameRecord, rules: &Rules, check: MoveCheck) -> Result<GameState, ReplayError>` | `record.rs`, `pub` | `replay_strict`'s preamble, loop and digest check under `check` (Task 2) |
| `fold_one_event_with(state, ev, rules, check) -> Result<GameState, String>` | `record.rs`, private | one event under `check` (Task 2) |
| `state_digest(&GameState) -> String` | `crates/yokohama-core/src/game/semantics.rs`, re-exported through `pub use semantics::*` (`game/mod.rs:138`); used by `record.rs:19` | the digest both replays are compared by (Task 2's cells) |
| `movement_routes_family(state, rules) -> Vec<Move>` | `crates/yokohama-core/src/game/route_enum.rs:689`, `pub(crate)` | the Movement family's exact list, phase-blind by contract (used by Task 1) |
| `drive(seeds: u64, steps: usize, visit: impl FnMut(&GameState, &Rules, &[Move]))`, `candidates(state, legal_moves) -> Vec<Move>`, `legal(state, rules, mv) -> bool` | `move_legality.rs` `mod tests`, lines 709, 743, 673 | the existing random-walk driver, the mutant generator and the `RulesLegal` shorthand Task 1's cells reuse |
| `common::run_recorded_game(seed: u64, players: u8, rules: &Rules) -> (GameState, GameRecord)` | `crates/yokohama-core/tests/common/mod.rs:282` | a 40-move random game and its record (`RECORDED_GAME_MOVES`, `:269`), unstamped — Task 2's cells call `stamp_terminal_digest` |

## File structure

- Modify `crates/yokohama-core/src/game/move_legality.rs`: `is_listed_move` after `is_move_legal` (which ends at
  line 268); two cells in `mod tests`.
- Modify `crates/yokohama-core/src/game/record.rs`: `MoveCheck`, `fold_one_event_with`, `replay_strict_with` after
  `replay_strict` (which ends at line 520).
- Create `crates/yokohama-core/tests/replay_check_equivalence.rs`: the record-level proof (uses `mod common;`).
- Modify `CLAUDE.md`: one sentence in the Architecture section's `game/` bullet (Task 3).

## Task order and why

Task 1 first: the predicate is what Task 2 folds with, and its two cells are the exactness argument at the move
level. Task 2 builds the replay and the record-level proof on it. Task 3 writes the one standing sentence. Task 4 is
the orchestrator's frame around all three: the "before" review that executes the cells, then the "after" review,
the battery, the gates and the PR.

---

### Task 1: `is_listed_move` — exact single-candidate membership — Opus (spec §3.1; Review Focus 1–3)

**Files:**
- Modify: `crates/yokohama-core/src/game/move_legality.rs` (after line 268, the end of `is_move_legal`; the cells in
  `mod tests`, which begins near line 660 — find `fn drive(` at line 709 and add the cells after
  `accepted_candidates_apply_and_add_no_new_labels`, which ends near line 902)
- Test: the same file's `mod tests`

**Interfaces:**
- Consumes: `is_move_legal`, `PathMode::Canonical` (this file); `crate::game::route_enum::movement_routes_family`
  (`route_enum.rs:689`, `pub(crate)`); in the cells `drive`, `candidates`, `legal` (this file's tests), `get_legal_moves`
  and `apply_move_unchecked` (`use crate::game::{get_legal_moves, …}` at the file's top; add
  `use crate::game::apply_move_unchecked;` inside `mod tests` if it is not already imported there).
- Produces: `pub fn is_listed_move(state: &GameState, mv: &Move, rules: &Rules) -> bool`.

- [ ] **Step 1: Write the two failing cells** at the end of `mod tests` in `move_legality.rs`:

```rust
    /// `is_listed_move` is EXACTLY membership in the legal list (the 2026-10-08
    /// ruling R-SEC2's move-level half): at every decision point of random
    /// games at 2/3/4P, every listed move passes, and every constructed
    /// candidate reads as the list reads it. The count of candidates
    /// `RulesLegal` ACCEPTS but the list LACKS is the anti-vacuity: those are
    /// the non-canonical witnesses where `is_move_legal` and membership
    /// disagree, and where this function must side with the list.
    #[test]
    fn is_listed_move_is_exactly_membership_in_the_legal_list() {
        let mut listed = 0usize;
        let mut candidates_seen = 0usize;
        let mut wider_cases = 0usize;
        drive(3, 400, |state, rules, legal_moves| {
            for mv in legal_moves {
                assert!(
                    is_listed_move(state, mv, rules),
                    "a listed move must pass at {:?}: {mv:?}",
                    state.turn_phase,
                );
                listed += 1;
            }
            for mv in candidates(state, legal_moves) {
                candidates_seen += 1;
                let in_list = legal_moves.contains(&mv);
                assert_eq!(
                    is_listed_move(state, &mv, rules),
                    in_list,
                    "disagrees with the list at {:?}: {mv:?}",
                    state.turn_phase,
                );
                if legal(state, rules, &mv) && !in_list {
                    wider_cases += 1;
                }
            }
        });
        assert!(listed > 1_000, "vacuous sweep: only {listed} listed moves");
        assert!(candidates_seen > 1_000, "vacuous sweep: only {candidates_seen} candidates");
        assert!(
            wider_cases > 0,
            "no candidate separated is_move_legal from membership — the exactness claim is untested",
        );
    }

    /// The phase dimension: a move the list held one state ago is judged at
    /// the NEXT state exactly as the next state's list judges it. The Movement
    /// family's list is phase-blind by contract (`route_enum.rs`), so this is
    /// what proves the conjunction takes `is_move_legal`'s phase and queue
    /// gates first; the counter proves a movement move was judged outside its
    /// phase at least once.
    #[test]
    fn is_listed_move_reads_the_phase_the_list_reads() {
        let mut checked = 0usize;
        let mut movement_refused_later = 0usize;
        drive(2, 300, |state, rules, legal_moves| {
            let Some(first) = legal_moves.first() else { return };
            let next = apply_move_unchecked(state, first, rules).expect("a listed move applies");
            let next_list = get_legal_moves(&next, rules);
            for mv in legal_moves {
                let in_next = next_list.contains(mv);
                assert_eq!(
                    is_listed_move(&next, mv, rules),
                    in_next,
                    "disagrees one state later at {:?}: {mv:?}",
                    next.turn_phase,
                );
                checked += 1;
                if !in_next && matches!(mv, Move::MovePresident { .. } | Move::PlacePresident { .. }) {
                    movement_refused_later += 1;
                }
            }
        });
        assert!(checked > 1_000, "vacuous sweep: only {checked} checks");
        assert!(
            movement_refused_later > 0,
            "no movement move was ever judged outside its phase — the gate is untested",
        );
    }
```

- [ ] **Step 2: Run them to verify they fail to compile**

Run: `cargo test -p yokohama-core --lib move_legality::tests::is_listed_move 2>&1 | tail -20`
Expected: a compile error naming `is_listed_move` as not found.

- [ ] **Step 3: Write the function** directly after `is_move_legal`'s closing brace (line 268):

```rust
/// Whether `mv` is IN `get_legal_moves(state, rules)` — a single-candidate
/// answer that never materializes the list, and, unlike [`is_move_legal`],
/// EXACT on the one family where acceptance is wider than membership.
///
/// `is_move_legal` accepts any legal WITNESS path for a `MovePresident` label
/// the generator emits (the module doc's "witnesses, not list membership"),
/// while the list prints one lex-least witness per label. So for the Movement
/// family — `MovePresident` and `PlacePresident` — the only exact check is the
/// family's own list, [`crate::game::route_enum::movement_routes_family`]:
/// byte for byte the list's Movement subsequence (`action_source_equivalence`
/// pins it), small (max observed 694 adversarial / 66 organic), compute-per-
/// call. That family consults nothing about `turn_phase` or the choice queue by
/// contract, so `is_move_legal`'s phase and queue gates are taken FIRST. For
/// every other variant acceptance and membership coincide (the module doc), so
/// `is_move_legal` IS the answer.
///
/// The proof: `is_listed_move_is_exactly_membership_in_the_legal_list` and
/// `is_listed_move_reads_the_phase_the_list_reads` below, and
/// `tests/replay_check_equivalence.rs` for the replay built on this (the
/// 2026-10-08 ruling R-SEC2). What this function is FOR: a replay of an
/// untrusted record whose cost must not follow the legal families' sizes.
pub fn is_listed_move(state: &GameState, mv: &Move, rules: &Rules) -> bool {
    match mv {
        Move::MovePresident { .. } | Move::PlacePresident { .. } => {
            is_move_legal(state, mv, rules, PathMode::Canonical)
                && crate::game::route_enum::movement_routes_family(state, rules)
                    .iter()
                    .any(|listed| listed == mv)
        }
        _ => is_move_legal(state, mv, rules, PathMode::Canonical),
    }
}
```

- [ ] **Step 4: Run the two cells and the module's existing cells**

Run: `cargo test -p yokohama-core --lib move_legality:: 2>&1 | tail -15`
Expected: every cell PASS, the two new ones included; the counters' asserts hold (if `wider_cases` reads 0 at
`drive(3, 400, …)`, raise the seeds to `drive(5, 400, …)` — `accepted_candidates_apply_and_add_no_new_labels`
reads non-canonical witnesses at 3/400, so 0 means the predicate, not the sweep, is wrong: STOP and report).

- [ ] **Step 5: Try the mutants before the reviewer does** (each reverted after its reading):
  - M1 — drop the family conjunct (the `MovePresident | PlacePresident` arm returns `is_move_legal(.., Canonical)`
    alone): `is_listed_move_is_exactly_membership_in_the_legal_list` must go RED on a non-canonical witness.
  - M2 — the arm with `PathMode::RulesLegal` and no family conjunct: RED, same cell.
  - M3 — the arm with the family membership alone (no `is_move_legal`): `is_listed_move_reads_the_phase_the_list_reads`
    must go RED.
  Record the three readings (the failing assertion's text) in the task report.

- [ ] **Step 6: Lint and commit**

Run: `cargo clippy -p yokohama-core --all-targets -- -D warnings`
Expected: clean.

```bash
git add crates/yokohama-core/src/game/move_legality.rs
git commit -m "core: is_listed_move — exact single-candidate membership in the legal list, proven over every decision point and candidate (R-SEC2)"
```

---

### Task 2: `MoveCheck`, `replay_strict_with`, and the record-level proof — Opus (spec §3.1; Review Focus 4–5)

**Files:**
- Modify: `crates/yokohama-core/src/game/record.rs` (after line 520, the end of `replay_strict`)
- Create: `crates/yokohama-core/tests/replay_check_equivalence.rs`
- Test: that binary

**Interfaces:**
- Consumes: `is_listed_move` (Task 1), `crate::game::apply_move::apply_move_unchecked` (`apply_move.rs:12`),
  `fold_one_event` (`record.rs:348`, private, same file), `rebuild_initial_state`, `state_digest`, `SEMANTICS_REV`,
  `check_invariants` — all as `replay_strict` uses them at `record.rs:478-520`; in the cells `common::run_recorded_game`
  and the tamper shapes of `tests/game_record.rs:118-128` (an out-of-phase move), `:258-273` (a wrong digest),
  `:129-154` (a rules-hash mismatch), `:156-172` (a broken state though every event applied), `:223-234` (a rev
  mismatch), `:323` (no provenance) — read each before writing the cell.
- Produces: `pub enum MoveCheck { Listed, Predicate }`; `pub fn replay_strict_with(record: &GameRecord, rules: &Rules, check: MoveCheck) -> Result<GameState, ReplayError>`.

- [ ] **Step 1: Write the failing integration binary** `crates/yokohama-core/tests/replay_check_equivalence.rs`:

```rust
//! The 2026-10-08 ruling R-SEC2's record-level proof: `replay_strict_with(..,
//! MoveCheck::Predicate)` accepts EXACTLY the records `replay_strict` accepts,
//! folds them to the same states, and refuses the same tampered records at the
//! same index with the same sentence — so the strict replay can stop
//! materializing a legal list per event with `SEMANTICS_REV` unmoved.
//! Companion to `move_legality.rs`'s `is_listed_move_is_exactly_membership_in_the_legal_list`
//! (the move-level half). Per-edit, never `#[ignore]`d: a proof you can defer
//! is a proof that rots.
mod common;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, finalize_setup, get_legal_moves, is_game_over, new_game, replay_strict,
    replay_strict_with, state_digest, GameConfig, GameRecord, GameState, Move, MoveCheck, PlayerId, RecordedEvent,
    ReplayError, Rules, StartingOrderChoice, SEMANTICS_REV,
};

type Shape = Result<String, (usize, String)>;

/// A 2P record whose hidden-starting-order player (P1) holds a FULL order hand
/// (3, the limit `check_invariants` enforces), built from legal moves only —
/// `game_record.rs:21-50`'s helper, repeated here because an integration
/// binary sees only `common`. `None` for a seed that never fills the hand.
fn record_with_full_order_hand(seed: u64, rules: &Rules) -> Option<(GameState, GameRecord, PlayerId)> {
    let hidden = PlayerId::new(1);
    let config = GameConfig::for_player_count(2);
    let pending = new_game(config, rules, seed).ok()?;
    let record_config = pending.state.config.clone();
    // P1 keeps its starting order PRIVATE, so a later RevealStartingOrder is a
    // legal event for it.
    let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown];
    let mut record = GameRecord::new(seed, record_config, choices.clone(), rules);
    let mut state = finalize_setup(pending, &choices).ok()?;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for _ in 0..4000 {
        if state.player(hidden).order_cards_in_hand.len() >= 3 {
            return Some((state, record, hidden));
        }
        if is_game_over(&state) {
            return None;
        }
        let moves = get_legal_moves(&state, rules);
        if moves.is_empty() {
            return None;
        }
        let mv = moves[rng.gen_range(0..moves.len())].clone();
        state = apply_move_unchecked(&state, &mv, rules).ok()?;
        record.push_move(mv);
    }
    None
}

fn shape(r: Result<GameState, ReplayError>) -> Shape {
    match r {
        Ok(state) => Ok(state_digest(&state)),
        Err(e) => Err((e.event_index, e.message)),
    }
}

/// Both replays of one record: the strict one, then the predicate one.
fn both(record: &GameRecord, rules: &Rules) -> (Shape, Shape) {
    (shape(replay_strict(record, rules)), shape(replay_strict_with(record, rules, MoveCheck::Predicate)))
}

#[test]
fn the_predicate_replay_folds_every_recorded_game_to_the_strict_replays_state() {
    let rules = Rules::standard();
    let mut games = 0usize;
    for players in [2u8, 3, 4] {
        for seed in 0..8u64 {
            let (final_state, mut record) = common::run_recorded_game(seed, players, &rules);
            record.stamp_terminal_digest(&final_state);
            let (strict, predicate) = both(&record, &rules);
            assert!(strict.is_ok(), "premise: {players}P seed {seed} replays strictly: {strict:?}");
            assert_eq!(predicate, strict, "{players}P seed {seed}: the predicate replay diverged");
            games += 1;
        }
    }
    assert_eq!(games, 24);
}

#[test]
fn the_listed_check_is_replay_strict_move_for_move() {
    let rules = Rules::standard();
    let (final_state, mut record) = common::run_recorded_game(3, 3, &rules);
    record.stamp_terminal_digest(&final_state);
    assert_eq!(
        shape(replay_strict_with(&record, &rules, MoveCheck::Listed)),
        shape(replay_strict(&record, &rules)),
    );
}

#[test]
fn an_empty_record_folds_to_the_same_initial_state_under_both_checks() {
    let rules = Rules::standard();
    let (_, mut record) = common::run_recorded_game(11, 2, &rules);
    record.events.clear();
    record.terminal_digest = None;
    let (strict, predicate) = both(&record, &rules);
    assert!(strict.is_ok(), "an empty record replays to the initial state: {strict:?}");
    assert_eq!(predicate, strict);
}

/// Every tampered record `game_record.rs` refuses, both replays refuse at the
/// same index with the same sentence — the duplicated preamble and digest
/// check in `replay_strict_with` are pinned to `replay_strict`'s words here.
#[test]
fn the_two_replays_refuse_the_same_tampered_records_with_the_same_words() {
    let rules = Rules::standard();
    let (final_state, clean) = common::run_recorded_game(7, 2, &rules);
    let stamped = {
        let mut r = clean.clone();
        r.stamp_terminal_digest(&final_state);
        r
    };
    let mut refusals = 0usize;
    let refuse = |record: &GameRecord, what: &str, refusals: &mut usize| {
        let (strict, predicate) = both(record, &rules);
        assert!(strict.is_err(), "premise: {what} is refused by replay_strict");
        assert_eq!(predicate, strict, "{what}: the two replays disagree");
        *refusals += 1;
    };

    // (a) an out-of-phase move at event 1 (game_record.rs:118-128)
    let mut r = stamped.clone();
    r.events[1] = RecordedEvent::Move(Move::Take5PowerToken);
    refuse(&r, "an out-of-phase move", &mut refusals);

    // (b) a wrong terminal digest (game_record.rs:258-273)
    let mut r = stamped.clone();
    r.terminal_digest = Some("ffffffffffffffff".into());
    refuse(&r, "a wrong terminal digest", &mut refusals);

    // (c) a rules-hash mismatch (game_record.rs:129-154)
    let mut r = stamped.clone();
    r.rules_hash = "0000000000000000".into();
    refuse(&r, "a rules-hash mismatch", &mut refusals);

    // (d) no semantics provenance (game_record.rs:323)
    let mut r = stamped.clone();
    r.semantics_rev = None;
    refuse(&r, "a record with no provenance", &mut refusals);

    // (e) another semantics rev (game_record.rs:223-234)
    let mut r = stamped.clone();
    r.semantics_rev = Some(SEMANTICS_REV + 1);
    refuse(&r, "a semantics-rev mismatch", &mut refusals);

    // (f) every walked president move replaced by a repeated-hop witness — a
    //     non-member the list refuses; the two replays must agree on it.
    let mut tampered = 0usize;
    for (k, ev) in stamped.events.iter().enumerate() {
        if let RecordedEvent::Move(Move::MovePresident { path, destination }) = ev {
            if let Some(&head) = path.first() {
                let mut r = stamped.clone();
                let mut repeated = path.clone();
                repeated.insert(0, head);
                r.events[k] = RecordedEvent::Move(Move::MovePresident { path: repeated, destination: *destination });
                let (strict, predicate) = both(&r, &rules);
                assert_eq!(predicate, strict, "event {k}: the two replays disagree on a repeated-hop witness");
                tampered += 1;
            }
        }
    }
    assert!(tampered > 0, "seed 7 at 2P held no walked president move — choose a seed that does");

    // (g) a broken state though every event applied (the shape of
    //     game_record.rs:156-172): a RevealStartingOrder into a hand already
    //     holding 3 orders APPLIES as an event and leaves a state
    //     `check_invariants` refuses — the invariant check pinned under both.
    let (state, mut r, hidden) = (0..64u64)
        .find_map(|seed| record_with_full_order_hand(seed, &rules))
        .expect("some seed must fill P1's order hand, else this case proves nothing");
    let card = *state
        .orders_removed
        .last()
        .or_else(|| state.order_deck.last())
        .expect("removed pile holds P1's two Unknown-parked cards at minimum");
    r.push_event(RecordedEvent::RevealStartingOrder { player: hidden, card });
    refuse(&r, "a broken state though every event applied", &mut refusals);

    assert_eq!(refusals, 6, "every refusal shape above ran");
}

/// Recorded, never asserted: the wall-clock of both replays over the longest
/// recorded game, printed under `--nocapture` for the task report.
#[test]
fn the_two_replays_walls_are_recorded() {
    let rules = Rules::standard();
    let (final_state, mut record) = common::run_recorded_game(0, 4, &rules);
    record.stamp_terminal_digest(&final_state);
    let t = std::time::Instant::now();
    let strict = replay_strict(&record, &rules).map(|s| state_digest(&s));
    let strict_wall = t.elapsed();
    let t = std::time::Instant::now();
    let predicate = replay_strict_with(&record, &rules, MoveCheck::Predicate).map(|s| state_digest(&s));
    let predicate_wall = t.elapsed();
    assert_eq!(predicate.ok(), strict.ok());
    println!(
        "replay walls over {} events: strict {:?}, predicate {:?}",
        record.events.len(),
        strict_wall,
        predicate_wall
    );
}
```

The crate's dev-dependencies already carry `rand` and `rand_chacha` (`tests/game_record.rs` imports both); case
(g) and its helper are `game_record.rs:21-50` and `:156-172` repeated, so this binary depends on `common` alone.

- [ ] **Step 2: Run it to verify it fails to compile**

Run: `cargo test -p yokohama-core --test replay_check_equivalence 2>&1 | tail -20`
Expected: compile errors naming `replay_strict_with` and `MoveCheck` as not found in `yokohama_core`.

- [ ] **Step 3: Write the additions** directly after `replay_strict`'s closing brace (`record.rs:520`):

```rust
/// How [`replay_strict_with`] checks each recorded MOVE against the legal
/// list (the 2026-10-08 ruling R-SEC2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveCheck {
    /// Membership in the materialized `get_legal_moves` list — `apply_move`,
    /// exactly the path [`replay_strict`] takes.
    Listed,
    /// The single-candidate exact check
    /// [`crate::game::move_legality::is_listed_move`], then the raw transition
    /// `apply_move_unchecked`: the same accepted set and the same states
    /// without a list materialized per event
    /// (`tests/replay_check_equivalence.rs` is the proof). A replay's cost then
    /// stops following the legal families' sizes — the retreat family reaches
    /// 442,368 moves in legal play — which is what an UNTRUSTED record (an
    /// archive upload, a restored save) must get.
    Predicate,
}

/// One recorded event folded under `check`: `Listed` is [`fold_one_event`]
/// unchanged; `Predicate` takes the same three non-move arms and, for a move,
/// `is_listed_move` then `apply_move_unchecked`, refusing a non-member with the
/// `MoveError::NotLegal` that `apply_move` returns — so the two checks word a
/// refusal identically.
fn fold_one_event_with(
    state: &GameState,
    ev: &RecordedEvent,
    rules: &Rules,
    check: MoveCheck,
) -> Result<GameState, String> {
    match check {
        MoveCheck::Listed => fold_one_event(state, ev, rules),
        MoveCheck::Predicate => {
            let step: Result<GameState, MoveError> = (|| {
                Ok(match ev {
                    RecordedEvent::Move(mv) => {
                        if !crate::game::move_legality::is_listed_move(state, mv, rules) {
                            return Err(MoveError::NotLegal);
                        }
                        crate::game::apply_move::apply_move_unchecked(state, mv, rules)?
                    }
                    RecordedEvent::RevealOrders(cards) => {
                        let mut s = state.clone();
                        s.arrange_orders_on_top(cards)?;
                        s
                    }
                    RecordedEvent::RevealTechs(cards) => {
                        let mut s = state.clone();
                        s.arrange_techs_on_top(cards)?;
                        s
                    }
                    RecordedEvent::RevealStartingOrder { player, card } => {
                        let mut s = state.clone();
                        s.reveal_starting_order(*player, *card)?;
                        s
                    }
                })
            })();
            step.map_err(|e| e.to_string())
        }
    }
}

/// [`replay_strict`] with the move check chosen by the caller: the same
/// preamble (the rules hash; the semantics rev, fail-closed on unknown
/// provenance), the same per-event invariant check, the same terminal-digest
/// check, the same `ReplayError` indices and sentences. `Listed` is
/// `replay_strict` move for move; `Predicate` is the replay an UNTRUSTED
/// record should get.
///
/// ADDITIVE by design (R-SEC2): `replay_strict` is not edited, and the
/// preamble below REPEATS its sentences rather than factoring them out, so the
/// existing function's text is untouched; the cells in
/// `tests/replay_check_equivalence.rs` hold the two functions' refusals equal,
/// index and sentence, on every tampered record `game_record.rs` builds.
pub fn replay_strict_with(record: &GameRecord, rules: &Rules, check: MoveCheck) -> Result<GameState, ReplayError> {
    let expected = rules.content_hash();
    if record.rules_hash != expected {
        return Err(ReplayError { event_index: 0, message: format!(
            "rules_hash mismatch: record \"{}\" vs running \"{expected}\" (strict mode rejects)", record.rules_hash) });
    }
    match record.semantics_rev {
        None => {
            return Err(ReplayError { event_index: 0, message: format!(
                "missing semantics provenance: record carries no semantics_rev (pre-Phase-2.9 record; running SEMANTICS_REV={SEMANTICS_REV}; strict mode rejects)") });
        }
        Some(rev) if rev != SEMANTICS_REV => {
            return Err(ReplayError { event_index: 0, message: format!(
                "semantics_rev mismatch: record {rev} vs running {SEMANTICS_REV} (strict mode rejects)") });
        }
        Some(_) => {}
    }
    let mut state = rebuild_initial_state(record, rules)?;
    for (i, ev) in record.events.iter().enumerate() {
        state = fold_one_event_with(&state, ev, rules, check)
            .map_err(|message| ReplayError { event_index: i, message })?;
        crate::game::invariants::check_invariants(&state, rules)
            .map_err(|m| ReplayError { event_index: i, message: format!("invariant violated: {m}") })?;
    }
    if let Some(expected_digest) = &record.terminal_digest {
        let actual_digest = state_digest(&state);
        if &actual_digest != expected_digest {
            return Err(ReplayError {
                event_index: record.events.len(),
                message: format!(
                    "terminal-state digest mismatch: record {expected_digest} vs replayed {actual_digest} (strict mode rejects)"
                ),
            });
        }
    }
    Ok(state)
}
```

Before writing, read `replay_strict` at `record.rs:478-520` once more and make the three sentences above equal to
its sentences character for character — the tamper cells compare them, and a stray space is a red cell, which is the
point.

- [ ] **Step 4: Run the binary and the record tests**

Run: `cargo test -p yokohama-core --test replay_check_equivalence --test game_record -- --nocapture 2>&1 | tail -25`
Expected: every cell PASS; the walls line printed (expect the predicate replay at or below the strict one — a
40-move record is short, so the two may read close; the reading goes in the task report).

- [ ] **Step 5: Try the mutants before the reviewer does** (each reverted after its reading):
  - M4 — `replay_strict_with` without the `check_invariants` call: case (g) must go RED.
  - M5 — the `Predicate` arm applying with `apply_move_unchecked` and NO `is_listed_move` check: cases (a) and (f)
    must go RED (a non-member applies raw instead of refusing).
  - M6 — one character changed in the rules-hash sentence: case (c) must go RED.
  Record the three readings in the task report.

- [ ] **Step 6: Lint and commit**

Run: `cargo clippy -p yokohama-core --all-targets -- -D warnings`
Expected: clean.

```bash
git add crates/yokohama-core/src/game/record.rs crates/yokohama-core/tests/replay_check_equivalence.rs
git commit -m "core: replay_strict_with and MoveCheck — the strict replay under a predicate move check, proven equivalent to replay_strict record for record (R-SEC2)"
```

---

### Task 3: The standing sentence in `CLAUDE.md` — Opus (spec §3.1)

**Files:**
- Modify: `CLAUDE.md`, the Architecture section's `game/` bullet — the sentence that ends
  "`apply_move_unchecked` is the internal fast path (moves already drawn from the list)."

**Interfaces:**
- Consumes: the names `is_listed_move`, `MoveCheck::Predicate`, `replay_strict_with` (Tasks 1–2).
- Produces: one sentence; nothing else in `CLAUDE.md` moves (it holds standing rules only).

- [ ] **Step 1: Add the sentence** immediately after "(moves already drawn from the list)." in that bullet:

```
**`is_listed_move` (`game/move_legality.rs`) is the single-candidate EXACT membership check, and `replay_strict_with(.., MoveCheck::Predicate)` (`game/record.rs`) the strict replay built on it — the replay an UNTRUSTED record gets (an archive upload, a restored save), proven to accept exactly what `replay_strict` accepts by `tests/replay_check_equivalence.rs` (R-SEC2, 2026-10-08); `replay_strict` itself is unedited, and a direct `is_move_legal` replay is never the answer, since that predicate is wider on a president move's witness path.**
```

- [ ] **Step 2: Check the file still reads as one bullet** (no new line break inside the bullet; the sentence
  joins the existing one with a space).

Run: `grep -c 'is_listed_move' CLAUDE.md`
Expected: `1`.

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: CLAUDE.md names the predicate replay check and its proof (R-SEC2)"
```

---

### Task 4: The reviews, the battery, the gates and the PR — ORCHESTRATOR (R-SEC1; spec §3.1, §5)

- [ ] **Step 1: The "before" review — the orchestrator's own.** On a `git archive` export of `master`, EXECUTE
  Tasks 1–2's cells against a scratch implementation of the two functions in the export (the plan's own code
  blocks), read each mutant M1–M6 red, and record any cell that cannot go red on its mutant, any name here that the
  tree does not spell, and any sentence of §3.1 or this plan that the code contradicts. The findings go in the
  spec's §9 and are fixed in this plan BEFORE Task 1 is dispatched; the scratch implementation is discarded with the
  export, never carried into the branch.

- [ ] **Step 2: The branch.** `git checkout -b security-s1-core-replay master`; Tasks 1–3 subagent-driven, each
  with a Fable reviewer who reruns the task's mutants on an export.

- [ ] **Step 3: The per-edit suites at the code head**, in this order, every row rc 0:
  `cargo test -p yokohama-core` (the new binary among them) · `cargo test -p yokohama-cli` · `cargo test -p yokohama-oracle`
  · `cargo test -p yokohama-difftest` · `cargo test -p yokohama-core --example eval_probe` · `cargo test -p yokohama-core --example solver`
  · `cargo test -p yokohama-bridge` · the four node suites of the bridge · `cargo clippy --workspace --all-targets -- -D warnings`
  · **the web lane's rows** (`docs/web-lane.md` §2, since `crates/yokohama-core` moved): `cargo test -p yokohama-web`
  with `wire_fixtures` green WITHOUT `UPDATE_WIRE_FIXTURES` (zero corpus bytes moved), `cargo test -p yokohama-wasm`,
  `cargo test -p yokohama-worker`, the `wasm32` check, both wasm builds, and `cd worker && npm run build:module:debug && npm test`.
  Record every count against the board's "Gates at HEAD" readings; a count that differs is a finding.

- [ ] **Step 4: The "after" review — the orchestrator's own.** Whole-branch, on a `git archive` export of the
  branch head: the equivalence argument re-derived from the code (not from this plan), the six mutants re-run, the
  fences of spec §6 checked (`git diff master -- crates/yokohama-oracle` empty; `apply_move`, `get_legal_moves`, `replay_strict`,
  `fold_one_event` unedited by `git diff master -U0` over those functions; `SEMANTICS_REV` 7). One fix wave as new
  commits; a scoped re-review of the fix.

- [ ] **Step 5: The fifteen release gates** as ONE uninterrupted sequence at the reviewed head (core moved), release
  binaries pre-built, machine-tagged, the wall recorded beside the previous reading (`CLAUDE.md` Commands, the two-tier
  gate policy).

- [ ] **Step 6: The measurement for S1's plan.** In the web crate's export (`…/scratchpad/r2/export` holds the
  review's worst-case bodies), replay the review's 23,606-event record both ways through a scratch test that parses
  the body's `record` and calls `replay_strict` then `replay_strict_with(.., Predicate)`; record both walls in the
  task report — the figure S1's event-count cap and semaphore are sized from. Not a gate.

- [ ] **Step 7: The PR** against `master`, its description naming the ruling (R-SEC1, R-SEC2), the proof binary, the
  six mutants' readings, the gate sequence's wall, and the measurement; the owner's word merges it. Then the
  decision-log row for the core PR's close, the ledger's A2 row (`core landed <merge commit>`), and the board's
  Track 4 next step (S1's plan, written against the merged tree).

## Review record

(The plan review's findings — Task 4, step 1 — dated, with each correction made to this plan; the file is frozen at
the PR's close.)
