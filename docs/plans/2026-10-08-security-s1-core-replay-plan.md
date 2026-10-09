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
materializing the Movement phase's legal list per event, so that the web layer (in S1 proper) can replay an untrusted
record at a cost that no longer follows the retreat family's size — measured by this plan's review (§"Review record"):
the review's 4P worst-case record of 23,606 events replays in 54 ms under the predicate check against 29.0 s under
`replay_strict`, to the same digest — with `SEMANTICS_REV` at 7, the oracle untouched, and `apply_move`,
`get_legal_moves`, `replay_strict` and `fold_one_event` unedited. The two families `is_move_legal` DELEGATES (area
actions, choice resolutions) still materialize their own measured-small lists; the retreat product is what is never paid.

**Architecture:** Two additions to `crates/yokohama-core` and one proof. `move_legality::is_listed_move` answers
"is this move IN `get_legal_moves`" for one candidate: for the Movement family's two variants it is the conjunction
of `is_move_legal(.., PathMode::Canonical)` (the phase and queue gates) and membership in the family's own exact,
small, compute-per-call list `route_enum::movement_routes_family` — because `is_move_legal` deliberately accepts any
legal WITNESS path for a label the list prints one lex-least witness for, the family's list is the only exact check;
for every other variant `is_move_legal` is exact by the module's contract. `record::replay_strict_with(record,
rules, MoveCheck)` is `replay_strict`'s preamble, loop and digest check with the move check chosen by the caller:
`Listed` folds through `apply_move` exactly as today, `Predicate` through `is_listed_move` then
`apply_move_unchecked`. Every non-move event folds through `fold_one_event` itself under either check. The proof is two
module cells (every listed move passes; every constructed candidate, the non-canonical witnesses included, reads as
the list reads it; every REAL member of the phase-blind Movement family is refused under a queued choice and outside
the Movement phase; a move the list held is judged one state later as that state's list judges it) and one
integration binary (24 recorded games fold to the same digest; the eight tampered records `game_record.rs` builds,
non-canonical witness tampers, a family member planted at an AreaAction event and a move after the game's end refuse
at the same index with the same sentence; an engine-version drift and legal reveal events on the success path are
accepted to the same digest).

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

Seven inputs the spec implies and this plan pins, each to the task that owns the code:

1. **A bare teleport encoding (`path: []`) for a destination a zero-cost walk also reaches.** The list prints only
   the walk (the generator's `zero_cost_destinations` suppression in the on-board arm, `legal_moves.rs:295-383`);
   `is_listed_move` must refuse the teleport
   encoding though `PathMode::RulesLegal` accepts it. Task 1's membership cell meets it through `candidates()`'s
   "bare teleport encoding for a walked destination" mutant.
2. **A President in hand.** The family's list is `PlacePresident` over areas there, not `MovePresident`; Task 1's
   membership cell meets it in every game's opening decisions, and its phase cell checks hand placements one state
   later.
3. **A `MovePresident` while a choice is queued.** `get_legal_moves` short-circuits to the choice's resolutions;
   `is_move_legal` mirrors that, and the family's list does not — the `is_move_legal` conjunct is what refuses it
   (its place in the conjunction is cost: the family is never computed for a move the gates refuse). Task 1's
   MEMBERSHIP cell meets it directly: at every decision point with a queued choice it judges every REAL member
   of the phase-blind family and counts the refusals (`queued_family_refusals`, 28 in the review's sweep) — every one
   of which is out of phase as well, since in engine-produced states a queued choice never coexists with the Movement
   phase; so the queue gate's OWN pin is the hand-built cell `a_queued_choice_refuses_a_listed_family_member_at_movement`
   (R-12). A Movement move's successor never queues a choice, so the phase cell cannot meet it (R-3).
4. **A record with no events.** Both replays must agree on the initial state's digest. Task 2's cell
   `an_empty_record_folds_to_the_same_initial_state_under_both_checks`.
5. **A record every event of which applies but whose state breaks an invariant.** The invariant check must run under
   `Predicate` exactly as under `Listed`. Task 2's tamper cell, case (g).
6. **A Movement-family member planted where the phase is AreaAction.** The list there cannot hold it; only
   `is_move_legal`'s phase gate refuses it, and a family-alone predicate applies it raw. Task 2's case (i), which reads the
   member off `get_legal_moves` on a copy of the state with its phase set to Movement (the family consults nothing
   about the phase by contract).
7. **A move appended after the game's end.** The phase machine parks a finished game at `TurnEnd`, where the list
   is empty and no predicate arm answers — neither check has a game-over gate, and neither needs one. Task 2's case
   (h), on a record played to its end (2,515 events at 2P seed 0).

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
| `state_at(record, k, rules) -> Result<GameState, ReplayError>`, `is_move_legal`, `PathMode::Canonical`, `TurnPhase` | `record.rs:561`; `move_legality.rs:140` and `:96`; `types/` — all at the crate root | Task 2's binary rebuilds the initial state, judges a walked move's witness variants, and flips a copy's phase to read the family (cases (f), (i)) |

## File structure

- Modify `crates/yokohama-core/src/game/move_legality.rs`: `is_listed_move` after `is_move_legal` (which ends at
  line 268); two cells and their helper `witness_variants` in `mod tests`.
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

- [ ] **Step 1: Write the three failing cells and their helper** in `move_legality.rs`'s `mod tests`, directly after
  `accepted_candidates_apply_and_add_no_new_labels` (the one anchor; the module's end is NOT the spot), under a banner
  line in the module's own style so they do not read as part of its `(a) + (b)` section —
  `    // -- is_listed_move: exact membership (R-SEC2) -----------------------------` — with `witness_variants`
  between the first two:

```rust
    /// `is_listed_move` is EXACTLY membership in the legal list (the 2026-10-08
    /// ruling R-SEC2's move-level half): at every decision point of random
    /// games at 2/3/4P, every listed move passes, and every constructed
    /// candidate reads as the list reads it. Three counters are the
    /// anti-vacuity, each naming the mechanism it proves was exercised:
    /// `conjunct_cases` — candidates `PathMode::Canonical` ACCEPTS and the
    /// list LACKS (non-lex-least witnesses; `witness_variants` builds them
    /// from every walked move), where only the family conjunct can side with
    /// the list; `queued_family_refusals` — REAL members of the phase-blind
    /// Movement family judged while a choice heads the queue, where only the
    /// `is_move_legal` conjunct — its queue short-circuit — refuses them,
    /// since the family lists them (Review Focus 3); `out_of_phase_family_refusals`
    /// — the same outside the Movement phase, refused by its phase gate alone.
    /// The phase gate's presence is what this cell and the phase cell pin (the
    /// family-alone mutant turns both red); the queue gate's own pin is
    /// `a_queued_choice_refuses_a_listed_family_member_at_movement`, because
    /// in engine-produced states a queued choice never coexists with the
    /// Movement phase, so every queued member here is out of phase as well.
    /// Taking the gates first is the cost order, not what these counters prove.
    #[test]
    fn is_listed_move_is_exactly_membership_in_the_legal_list() {
        let mut listed = 0usize;
        let mut candidates_seen = 0usize;
        let mut conjunct_cases = 0usize;
        let mut queued_family_refusals = 0usize;
        let mut out_of_phase_family_refusals = 0usize;
        drive(3, 400, |state, rules, legal_moves| {
            for mv in legal_moves {
                assert!(
                    is_listed_move(state, mv, rules),
                    "a listed move must pass at {:?}: {mv:?}",
                    state.turn_phase,
                );
                listed += 1;
            }
            let mut pool = candidates(state, legal_moves);
            pool.extend(witness_variants(state, legal_moves));
            for mv in pool {
                candidates_seen += 1;
                let in_list = legal_moves.contains(&mv);
                assert_eq!(
                    is_listed_move(state, &mv, rules),
                    in_list,
                    "disagrees with the list at {:?}: {mv:?}",
                    state.turn_phase,
                );
                if !in_list && canonical(state, rules, &mv) {
                    conjunct_cases += 1;
                }
            }
            // The phase-blind family's REAL members, judged where the list
            // cannot hold them: under a queued choice, or in any phase but
            // Movement. The family is computed here exactly as the conjunct
            // computes it, so under the family-alone mutant every member
            // reads as listed.
            let queued = !state.pending_choices.is_empty();
            if queued || state.turn_phase != TurnPhase::Movement {
                for mv in crate::game::route_enum::movement_routes_family(state, rules) {
                    assert!(!legal_moves.contains(&mv), "premise: no movement move is listed here: {mv:?}");
                    assert!(
                        !is_listed_move(state, &mv, rules),
                        "a family member must be refused at {:?} (queued: {queued}): {mv:?}",
                        state.turn_phase,
                    );
                    if queued {
                        queued_family_refusals += 1;
                    } else {
                        out_of_phase_family_refusals += 1;
                    }
                }
            }
        });
        println!(
            "is_listed_move sweep: listed {listed}, candidates {candidates_seen}, conjunct cases {conjunct_cases}, \
             queued family refusals {queued_family_refusals}, out-of-phase family refusals {out_of_phase_family_refusals}"
        );
        assert!(listed > 1_000, "vacuous sweep: only {listed} listed moves");
        assert!(candidates_seen > 1_000, "vacuous sweep: only {candidates_seen} candidates");
        assert!(
            conjunct_cases > 0,
            "no Canonical-accepted non-member was judged — the family conjunct is untested",
        );
        assert!(
            queued_family_refusals > 0,
            "no family member was judged under a queued choice — the queue gate is untested",
        );
        assert!(
            out_of_phase_family_refusals > 0,
            "no family member was judged outside Movement — the phase gate is untested",
        );
    }

    /// Non-canonical WITNESSES for labels the list prints: for every walked
    /// `MovePresident`, the same route with a detour prepended through a
    /// neighbour adjacent to both the start and the first hop, and the direct
    /// hop where a longer walk's destination is adjacent to the start. Which
    /// variants are rules-legal, which share a listed move's label and which
    /// the generator happens to print is the LIST's business: every one is
    /// judged against it.
    fn witness_variants(state: &GameState, legal_moves: &[Move]) -> Vec<Move> {
        let PresidentLocation::OnBoard(current) = state.player(state.current_player).president_location
        else {
            return vec![];
        };
        let mut out = Vec::new();
        for mv in legal_moves {
            let Move::MovePresident { path, destination } = mv else { continue };
            let Some(&first) = path.first() else { continue };
            for &via in state.board_layout.adjacent_areas(current) {
                if via == first
                    || path.contains(&via)
                    || !state.board_layout.adjacent_areas(via).contains(&first)
                {
                    continue;
                }
                let mut detour = Vec::with_capacity(path.len() + 1);
                detour.push(via);
                detour.extend_from_slice(path);
                out.push(Move::MovePresident { path: detour, destination: *destination });
            }
            if path.len() >= 2 && state.board_layout.adjacent_areas(current).contains(destination) {
                out.push(Move::MovePresident { path: vec![*destination], destination: *destination });
            }
        }
        out
    }

    /// The phase dimension: a move the list held one state ago is judged at
    /// the NEXT state exactly as the next state's list judges it. The Movement
    /// family's list is phase-blind by contract (`route_enum.rs`), so this is
    /// what proves the conjunction keeps `is_move_legal`'s PHASE gate (the
    /// queue gate's pin is `a_queued_choice_refuses_a_listed_family_member_at_movement`
    /// — a movement move's successor never queues a choice, so this cell
    /// cannot meet it); the counter proves a movement move was judged outside
    /// its phase at least once.
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
        println!("phase sweep: checked {checked}, movement moves refused one state later {movement_refused_later}");
        assert!(checked > 1_000, "vacuous sweep: only {checked} checks");
        assert!(
            movement_refused_later > 0,
            "no movement move was ever judged outside its phase — the gate is untested",
        );
    }

    /// The queue gate's own pin for the conjunction. In engine-produced states
    /// a queued choice never coexists with the Movement phase (every queued
    /// member the membership cell judges is out of phase as well, so the phase
    /// gate refuses it too), so only a hand-built state can show that the
    /// conjunction relies on `is_move_legal`'s queue short-circuit: the diamond
    /// at Movement with an `AnyGood` choice queued, where the family lists the
    /// walk and the list holds only the choice's resolutions. A predicate that
    /// kept the phase gate and dropped the queue gate would accept it.
    #[test]
    fn a_queued_choice_refuses_a_listed_family_member_at_movement() {
        let mut s = diamond(2, &[]);
        s.pending_choices.push(PendingChoice::AnyGood);
        let rules = Rules::empty_for_tests();
        let walk = Move::MovePresident { path: vec![AreaId(1)], destination: AreaId(1) };
        assert_eq!(s.turn_phase, TurnPhase::Movement, "premise: the Movement phase");
        assert!(
            crate::game::route_enum::movement_routes_family(&s, &rules).contains(&walk),
            "premise: the family lists it",
        );
        assert!(!get_legal_moves(&s, &rules).contains(&walk), "premise: the list does not");
        assert!(!is_listed_move(&s, &walk, &rules), "the queue gate must refuse it: {walk:?}");
    }
```

- [ ] **Step 2: Run them to verify they fail to compile**

Run: `cargo test -p yokohama-core --lib move_legality::tests::is_listed_move 2>&1 | tail -20`
Expected: a compile error naming `is_listed_move` as not found.

- [ ] **Step 3: Write the function** directly after `is_move_legal`'s closing brace (line 268):

```rust
/// Whether `mv` is IN `get_legal_moves(state, rules)` — a single-candidate
/// answer that never pays the Movement phase's retreat product (the family
/// that reaches 442,368 moves in legal play; the two families `is_move_legal`
/// DELEGATES, area actions and choice resolutions, still materialize their
/// own measured-small lists — max 154 and 114) and, unlike [`is_move_legal`],
/// EXACT on the one family where acceptance is wider than membership.
///
/// `is_move_legal` accepts any legal WITNESS path for a `MovePresident` label
/// the generator emits (the module doc's "witnesses, not list membership"),
/// while the list prints one lex-least witness per label. So for the Movement
/// family — `MovePresident` and `PlacePresident` — the only exact check is the
/// family's own list, `crate::game::route_enum::movement_routes_family`:
/// byte for byte the list's Movement subsequence (`action_source_equivalence`
/// pins it), small (max observed 694 adversarial / 66 organic), compute-per-
/// call. That family consults nothing about `turn_phase` or the choice queue by
/// contract, so `is_move_legal`'s phase and queue gates are taken FIRST. For
/// every other variant acceptance and membership coincide (the module doc), so
/// `is_move_legal` IS the answer.
///
/// The proof: `is_listed_move_is_exactly_membership_in_the_legal_list`,
/// `is_listed_move_reads_the_phase_the_list_reads` and
/// `a_queued_choice_refuses_a_listed_family_member_at_movement` below, and
/// `tests/replay_check_equivalence.rs` for the replay built on this (the
/// 2026-10-08 ruling R-SEC2). What this function is FOR: a replay of an
/// untrusted record whose cost must not follow the retreat family's size.
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
Expected: every cell PASS (23: the module's 20 and the three new ones), and under `-- --nocapture` the two sweep cells' counter lines. The review's
readings at `drive(3, 400, …)` / `drive(2, 300, …)`: listed 412,372 · candidates 1,262,450 · conjunct cases 165 ·
queued family refusals 28 · out-of-phase family refusals 5,891 · phase sweep checked 207,129 with 1,682 movement
moves refused one state later (debug wall ~7 s for the module). A counter that reads 0 means the predicate, not the
sweep, is wrong (`accepted_candidates_apply_and_add_no_new_labels` reads non-canonical witnesses from the same
sweep): STOP and report.

- [ ] **Step 5: Try the mutants before the reviewer does** (each reverted after its reading):
  - M1 — drop the family conjunct (the `MovePresident | PlacePresident` arm returns `is_move_legal(.., Canonical)`
    alone): `is_listed_move_is_exactly_membership_in_the_legal_list` must go RED on a non-canonical witness — the
    review read `disagrees with the list at Movement: MovePresident { path: [AreaId(6), AreaId(2)], destination:
    AreaId(2) }`, left `true`. (Task 2's case (f) goes RED on this mutant too.)
  - M2 — the arm with `PathMode::RulesLegal` and no family conjunct: RED, same cell, same witness.
  - M3 — the arm with the family membership alone (no `is_move_legal`): the membership cell must go RED on a family
    member (`a family member must be refused at AdditionalActionSecond (queued: false): PlacePresident { destination:
    AreaId(8) }`) and `is_listed_move_reads_the_phase_the_list_reads` on `disagrees one state later at AreaAction:
    MovePresident { path: [AreaId(5)], destination: AreaId(5) }`. (Task 2's case (i) goes RED on it too.)
  - M3q — the queue gate alone dropped: the arm judges `is_move_legal` on a clone of `state` with `pending_choices`
    cleared (every other check kept): `a_queued_choice_refuses_a_listed_family_member_at_movement` must go RED
    (`the queue gate must refuse it: MovePresident { path: [AreaId(1)], destination: AreaId(1) }`) while BOTH sweep
    cells stay green with unchanged counters — which is why that cell exists (R-12).
  Record the four readings (the failing assertion's text) in the task report.

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
//! folds them to the same states, refuses the same tampered records at the
//! same index with the same sentence, and accepts what it accepts — an
//! engine-version drift, legal reveal events — so the strict replay can stop
//! materializing the retreat family per event with `SEMANTICS_REV` unmoved.
//! Companion to `move_legality.rs`'s `is_listed_move_is_exactly_membership_in_the_legal_list`
//! (the move-level half). Per-edit, never `#[ignore]`d: a proof you can defer
//! is a proof that rots.
mod common;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, finalize_setup, get_legal_moves, is_game_over, is_move_legal, new_game,
    replay_strict, replay_strict_with, state_at, state_digest, AreaId, GameConfig, GameRecord, GameState,
    Move, MoveCheck, OrderCardId, PathMode, PlayerId, PresidentLocation, RecordedEvent, ReplayError, Rules,
    StartingOrderChoice, SEMANTICS_REV,
};

type Shape = Result<String, (usize, String)>;

/// `apply_move`'s own word for a non-member (`types/errors.rs`) — the one
/// sentence both checks refuse a tampered move with.
const NOT_LISTED: &str = "move is not in the legal-move list for this state";

/// A 2P record whose hidden-starting-order player (P1) holds a FULL order hand
/// (3, the limit `check_invariants` enforces), built from legal moves only —
/// `game_record.rs`'s helper, repeated here because an integration binary
/// sees only `common`. `None` for a seed that never fills the hand.
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

/// A record of uniform-random legal play to `max_moves` applied moves or the
/// game's end, its terminal digest stamped: the long prefixes case (f) needs
/// (`common::run_recorded_game` stops at 40 moves, where a mover has too few
/// assistants on board for a detour to be steppable) and the finished game
/// case (h) needs.
fn recorded_game(seed: u64, players: u8, max_moves: usize, rules: &Rules) -> (GameState, GameRecord) {
    let config = GameConfig::for_player_count(players);
    let pending = new_game(config, rules, seed).expect("new_game");
    let record_config = pending.state.config.clone();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
    let mut record = GameRecord::new(seed, record_config, choices.clone(), rules);
    let mut state = finalize_setup(pending, &choices).expect("finalize_setup");
    for _ in 0..max_moves {
        if is_game_over(&state) {
            break;
        }
        let moves = get_legal_moves(&state, rules);
        assert!(!moves.is_empty(), "a live state offers a move");
        let mv = moves[rng.gen_range(0..moves.len())].clone();
        state = apply_move_unchecked(&state, &mv, rules).expect("a listed move applies");
        record.push_move(mv);
    }
    record.stamp_terminal_digest(&state);
    (state, record)
}

/// Non-canonical WITNESSES for a walked `MovePresident` at `state`: the same
/// route with a detour prepended through a neighbour adjacent to both the
/// start and the first hop, the direct hop where a longer walk's destination
/// is adjacent to the start, and the walk reversed. A variant is a TAMPER exactly when
/// `PathMode::Canonical` accepts it and the list lacks it — the witness-wider
/// case R-SEC2 names, where only the family conjunct sides with the list.
fn witness_variants(state: &GameState, path: &[AreaId], destination: AreaId) -> Vec<Move> {
    let PresidentLocation::OnBoard(current) = state.player(state.current_player).president_location
    else {
        return vec![];
    };
    let Some(&first) = path.first() else { return vec![] };
    let mut out = Vec::new();
    for &via in state.board_layout.adjacent_areas(current) {
        if via == first || path.contains(&via) || !state.board_layout.adjacent_areas(via).contains(&first) {
            continue;
        }
        let mut detour = Vec::with_capacity(path.len() + 1);
        detour.push(via);
        detour.extend_from_slice(path);
        out.push(Move::MovePresident { path: detour, destination });
    }
    if path.len() >= 2 {
        if state.board_layout.adjacent_areas(current).contains(&destination) {
            out.push(Move::MovePresident { path: vec![destination], destination });
        }
        let mut reversed = path.to_vec();
        reversed.reverse();
        let dest = reversed[reversed.len() - 1];
        out.push(Move::MovePresident { path: reversed, destination: dest });
    }
    out
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

/// Every tampered record `game_record.rs` refuses — an out-of-phase move (a),
/// a wrong digest (b), a rules-hash mismatch (c), no provenance (d), another
/// rev (e), a broken state though every event applied (g), a dropped last
/// event under the full run's digest (k), an illegal move inserted mid-record
/// (m) — both replays refuse at the same index with the same sentence (the
/// preamble and digest check `replay_strict_with` repeats are pinned to
/// `replay_strict`'s words here), and so do the three shapes only this binary
/// builds: a non-canonical witness (f), a family member out of its phase (i),
/// a move after the game's end (h).
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

    // (b) a wrong terminal digest (game_record.rs:258-268)
    let mut r = stamped.clone();
    r.terminal_digest = Some("ffffffffffffffff".into());
    refuse(&r, "a wrong terminal digest", &mut refusals);

    // (c) a rules-hash mismatch (game_record.rs:129-138)
    let mut r = stamped.clone();
    r.rules_hash = "0000000000000000".into();
    refuse(&r, "a rules-hash mismatch", &mut refusals);

    // (d) no semantics provenance (game_record.rs:323)
    let mut r = stamped.clone();
    r.semantics_rev = None;
    refuse(&r, "a record with no provenance", &mut refusals);

    // (e) another semantics rev (game_record.rs:223-231)
    let mut r = stamped.clone();
    r.semantics_rev = Some(SEMANTICS_REV + 1);
    refuse(&r, "a semantics-rev mismatch", &mut refusals);

    // (f) a walked president move replaced by a NON-CANONICAL WITNESS the list
    //     refuses and `PathMode::Canonical` accepts — the witness-wider case
    //     (R-SEC2) where only the family conjunct sides with the list. A
    //     conjunct-less predicate would APPLY the variant (same label, same
    //     successor state) and fold on to the very digest the record carries,
    //     answering Ok where `replay_strict` refuses at k — the divergence
    //     this case exists to see. Beside it, the repeated-hop witness the
    //     predicate refuses by itself. Long prefixes, walked ONCE with the
    //     decision state carried along: a 40-move record holds walks a mover
    //     cannot yet detour around.
    let mut walked = 0usize;
    let mut witness_tampers = 0usize;
    let mut repeated_hop_tampers = 0usize;
    let mut out_of_phase_tampers = 0usize;
    'games: for players in [2u8, 3, 4] {
        for seed in 0..3u64 {
            let (_, record) = recorded_game(seed, players, 400, &rules);
            let mut before = state_at(&record, 0, &rules).expect("the initial state rebuilds");
            for k in 0..record.events.len() {
                let RecordedEvent::Move(mv) = &record.events[k] else {
                    unreachable!("a recorded game holds moves only");
                };
                if let Move::MovePresident { path, destination } = mv {
                    if !path.is_empty() {
                        walked += 1;
                        let listed = get_legal_moves(&before, &rules);
                        let mut tampers = Vec::new();
                        if repeated_hop_tampers < 3 {
                            let mut repeated = path.clone();
                            repeated.insert(0, path[0]);
                            tampers.push((Move::MovePresident { path: repeated, destination: *destination }, false));
                        }
                        for variant in witness_variants(&before, path, *destination) {
                            if is_move_legal(&before, &variant, &rules, PathMode::Canonical)
                                && !listed.contains(&variant)
                            {
                                tampers.push((variant, true));
                            }
                        }
                        for (tamper, is_witness) in tampers {
                            let mut r = record.clone();
                            r.events[k] = RecordedEvent::Move(tamper.clone());
                            let (strict, predicate) = both(&r, &rules);
                            assert_eq!(
                                strict,
                                Err((k, NOT_LISTED.to_string())),
                                "premise: {players}P seed {seed} event {k}: replay_strict refuses {tamper:?}",
                            );
                            assert_eq!(
                                predicate, strict,
                                "{players}P seed {seed} event {k}: the two replays disagree on {tamper:?}",
                            );
                            if is_witness {
                                witness_tampers += 1;
                            } else {
                                repeated_hop_tampers += 1;
                            }
                        }
                        // (i) a MEMBER of the phase-blind Movement family planted at
                        //     the NEXT event, where the phase is AreaAction: the list
                        //     there cannot hold it, and only `is_move_legal`'s phase
                        //     gate refuses it — a family-alone predicate would apply
                        //     it raw and fold on to a different state. The
                        //     member is read off `get_legal_moves` on a copy of that
                        //     state with its phase set to Movement (the family consults
                        //     nothing about the phase by contract, so the copy's
                        //     Movement subsequence IS the family at the real state).
                        if out_of_phase_tampers < 4 && k + 1 < record.events.len() {
                            let after = apply_move_unchecked(&before, mv, &rules).expect("a recorded move applies");
                            let mut as_movement = after.clone();
                            as_movement.turn_phase = yokohama_core::TurnPhase::Movement;
                            let member = get_legal_moves(&as_movement, &rules)
                                .into_iter()
                                .find(|m| matches!(m, Move::MovePresident { .. } | Move::PlacePresident { .. }));
                            if let Some(member) = member {
                                assert_eq!(after.turn_phase, yokohama_core::TurnPhase::AreaAction, "premise: a walk hands off to AreaAction");
                                let mut r = record.clone();
                                r.events[k + 1] = RecordedEvent::Move(member.clone());
                                let (strict, predicate) = both(&r, &rules);
                                assert_eq!(
                                    strict,
                                    Err((k + 1, NOT_LISTED.to_string())),
                                    "premise: {players}P seed {seed} event {}: replay_strict refuses the out-of-phase member {member:?}",
                                    k + 1,
                                );
                                assert_eq!(
                                    predicate, strict,
                                    "{players}P seed {seed} event {}: the two replays disagree on an out-of-phase member {member:?}",
                                    k + 1,
                                );
                                out_of_phase_tampers += 1;
                            }
                        }
                        if witness_tampers >= 8 && out_of_phase_tampers >= 4 {
                            break 'games;
                        }
                    }
                }
                before = apply_move_unchecked(&before, mv, &rules).expect("a recorded move applies");
            }
        }
    }
    assert!(walked > 0, "no recorded game held a walked president move");
    assert!(repeated_hop_tampers > 0, "no repeated-hop tamper ran");
    assert!(
        witness_tampers > 0,
        "no Canonical-accepted non-member witness was built — the family conjunct is untested at the record level",
    );
    assert!(
        out_of_phase_tampers > 0,
        "no family member was planted out of its phase — the phase gate is untested at the record level",
    );
    refusals += 1;

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

    // (h) a move appended after the game's END. The phase machine parks a
    //     finished game at `TurnEnd` (`apply_move.rs`'s `advance_phase`), where
    //     the list is empty and no predicate arm answers, so both replays
    //     refuse the appended event at the past-the-end index with `NotLegal`'s
    //     words — neither check continues a finished game.
    let (end, mut done) = recorded_game(0, 2, 20_000, &rules);
    assert!(is_game_over(&end), "premise: the game ran to its end within 20,000 moves");
    let extra = done.events.len();
    done.push_move(Move::EndAdditionalPhase);
    let (strict, predicate) = both(&done, &rules);
    assert_eq!(strict, Err((extra, NOT_LISTED.to_string())), "premise: a move after the end is refused");
    assert_eq!(predicate, strict, "a move after the end: the two replays disagree");
    refusals += 1;

    // (k) the last event dropped under the full run's digest
    //     (game_record.rs:275-301): every event applies and the state is
    //     valid, so only the digest check refuses — at the past-the-end index,
    //     with the digest sentence.
    let mut r = stamped.clone();
    r.events.pop();
    let (strict, predicate) = both(&r, &rules);
    assert!(
        matches!(&strict, Err((i, m)) if *i == r.events.len() && m.starts_with("terminal-state digest mismatch")),
        "premise: a dropped last event is refused by the digest check: {strict:?}",
    );
    assert_eq!(predicate, strict, "a dropped last event: the two replays disagree");
    refusals += 1;

    // (m) an illegal move INSERTED mid-record (game_record.rs:66-80): refused
    //     at its own index, every event before it folded.
    let mut r = stamped.clone();
    let mid = r.events.len() / 2;
    r.events.insert(mid, RecordedEvent::Move(Move::FulfillOrder { order_id: OrderCardId(255) }));
    let (strict, predicate) = both(&r, &rules);
    assert_eq!(strict, Err((mid, NOT_LISTED.to_string())), "premise: an inserted illegal move is refused at its index");
    assert_eq!(predicate, strict, "an inserted illegal move: the two replays disagree");
    refusals += 1;

    println!(
        "tampers: walked {walked}, witness {witness_tampers}, repeated-hop {repeated_hop_tampers}, \
         out-of-phase members {out_of_phase_tampers}; the finished game held {extra} events"
    );
    assert_eq!(refusals, 10, "every refusal shape above ran");
}

/// The one provenance key strict replay deliberately does NOT gate (the
/// 2026-07-27 decoupling, game_record.rs:105-115): a record from another
/// release at the same `SEMANTICS_REV` is ACCEPTED. An archive upload or a
/// restored save from an older release is exactly such a record, so the
/// predicate replay must accept it too, to the same digest — a version gate
/// added to `replay_strict_with`'s preamble (mutant M7) is what this refuses.
#[test]
fn an_engine_version_drift_is_accepted_under_both_checks() {
    let rules = Rules::standard();
    let (final_state, mut record) = common::run_recorded_game(7, 2, &rules);
    record.stamp_terminal_digest(&final_state);
    record.engine_version = "0.0.0-other-release".into();
    let (strict, predicate) = both(&record, &rules);
    assert_eq!(strict, Ok(state_digest(&final_state)), "premise: replay_strict accepts the drift");
    assert_eq!(predicate, strict, "the predicate replay must accept the drift too");
}

/// LEGAL reveal events on the SUCCESS path — `RevealOrders` (the bottom deck
/// card and a removed-pile card: the conservation-by-exchange path),
/// `RevealTechs`, and `RevealStartingOrder` of a hidden player — interleaved
/// with random legal play to the game's end: both replays fold the record to
/// the live game's own terminal digest. Under `Predicate` every non-move event
/// is `fold_one_event` itself by construction; this cell holds that
/// construction (mutant M9: a `Predicate` arm that skips the deck reveals
/// folds on to a different state, and this cell goes red).
#[test]
fn legal_reveal_events_fold_identically_on_the_success_path() {
    let rules = Rules::standard();
    let hidden = PlayerId::new(1);
    let (mut orders, mut exchanges, mut techs, mut starting) = (0usize, 0usize, 0usize, 0usize);
    for seed in 0..3u64 {
        let pending = new_game(GameConfig::for_player_count(2), &rules, seed).expect("new_game");
        let record_config = pending.state.config.clone();
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown];
        let mut record = GameRecord::new(seed, record_config, choices.clone(), &rules);
        let mut state = finalize_setup(pending, &choices).expect("finalize_setup");
        let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x5EED);
        let mut step = 0usize;
        while !is_game_over(&state) && step < 20_000 {
            if step % 29 == 7 {
                let mut s = state.clone();
                let ev = match rng.gen_range(0..3u8) {
                    0 if state.order_deck.len() >= 2 => {
                        let mut cards = vec![state.order_deck[0]];
                        if let Some(&c) = state.orders_removed.first() {
                            cards.push(c);
                        }
                        let exchange = cards.len() == 2;
                        s.arrange_orders_on_top(&cards).ok().map(|_| {
                            orders += 1;
                            if exchange {
                                exchanges += 1;
                            }
                            RecordedEvent::RevealOrders(cards)
                        })
                    }
                    1 if state.tech_deck.len() >= 2 => {
                        let cards = vec![state.tech_deck[0], state.tech_deck[1]];
                        s.arrange_techs_on_top(&cards).ok().map(|_| {
                            techs += 1;
                            RecordedEvent::RevealTechs(cards)
                        })
                    }
                    2 if state.player(hidden).starting_order_hidden
                        && state.player(hidden).order_cards_in_hand.len() < 3 =>
                    {
                        let card = *state.orders_removed.last().expect("Unknown parks both dealt cards");
                        s.reveal_starting_order(hidden, card).ok().map(|_| {
                            starting += 1;
                            RecordedEvent::RevealStartingOrder { player: hidden, card }
                        })
                    }
                    _ => None,
                };
                if let Some(ev) = ev {
                    state = s;
                    record.push_event(ev);
                }
            }
            let moves = get_legal_moves(&state, &rules);
            assert!(!moves.is_empty(), "seed {seed}: a live state offers a move");
            let mv = moves[rng.gen_range(0..moves.len())].clone();
            state = apply_move_unchecked(&state, &mv, &rules).expect("a listed move applies");
            record.push_move(mv);
            step += 1;
        }
        assert!(is_game_over(&state), "seed {seed}: the game ran to its end");
        record.stamp_terminal_digest(&state);
        let (strict, predicate) = both(&record, &rules);
        assert_eq!(strict, Ok(state_digest(&state)), "seed {seed}: replay_strict reproduces the live game");
        assert_eq!(predicate, strict, "seed {seed}: the predicate replay diverged on a record with reveals");
    }
    println!("reveals on the success path: orders {orders} (exchanges {exchanges}), techs {techs}, starting {starting}");
    assert!(orders > 0 && exchanges > 0 && techs > 0 && starting > 0, "every reveal kind ran at least once");
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
(g) and its helper are `game_record.rs:21-49` and `:156-172` repeated, so this binary depends on `common` alone.
`recorded_game` plays uniform-random legal moves to a cap or the game's end (a 2P game runs ~2,500 events; the whole
binary ran in 0.65 s in debug at the review); `witness_variants` is the module helper's record-level twin.

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
    /// without the retreat family's product materialized per event
    /// (`tests/replay_check_equivalence.rs` is the proof). A replay's cost then
    /// stops following that family's size — 442,368 moves in legal play —
    /// while the two families `is_move_legal` delegates (area actions and
    /// choice resolutions, max 154 and 114 measured) and the Movement family's
    /// own small list stay materialized; that is what an UNTRUSTED record (an
    /// archive upload, a restored save) must get.
    Predicate,
}

/// One recorded event folded under `check`. `Listed`, and every NON-MOVE
/// event under either check, is [`fold_one_event`] itself — the ONE fold (its
/// own doc: a second hand-kept copy of that match drifts). `Predicate`
/// differs for a `Move` alone: `is_listed_move`, then the raw transition
/// `apply_move_unchecked`, refusing a non-member with the `MoveError::NotLegal`
/// that `apply_move` returns — so the two checks word a refusal identically.
fn fold_one_event_with(
    state: &GameState,
    ev: &RecordedEvent,
    rules: &Rules,
    check: MoveCheck,
) -> Result<GameState, String> {
    match (check, ev) {
        (MoveCheck::Predicate, RecordedEvent::Move(mv)) => {
            if !crate::game::move_legality::is_listed_move(state, mv, rules) {
                return Err(MoveError::NotLegal.to_string());
            }
            crate::game::apply_move::apply_move_unchecked(state, mv, rules).map_err(|e| e.to_string())
        }
        _ => fold_one_event(state, ev, rules),
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
/// index and sentence, on every tampered record `game_record.rs` builds and
/// on the binary's own shapes, and their acceptances equal on an
/// engine-version drift and on legal reveal events.
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
Expected: every cell PASS (7); the walls, tampers and reveals lines printed (the review read strict 6.2–8.5 ms
against predicate 4.1–6.4 ms over 40 events — a short record, so the two read close — and `tampers: walked 12,
witness 5, repeated-hop 3, out-of-phase members 4; the finished game held 2515 events`; the reveals line's four
counts must all be nonzero, which the cell asserts); the readings go in the task report.

- [ ] **Step 5: Try the mutants before the reviewer does** (each reverted after its reading):
  - M4 — `replay_strict_with` without the `check_invariants` call: case (g) must go RED — the review read
    `left: Ok("935e76b39e1aafe9") right: Err((1034, "invariant violated: P1 order hand size 4 > 3"))`.
  - M5 — the `Predicate` arm applying with `apply_move_unchecked` and NO `is_listed_move` check (delete the
    `if !is_listed_move { return Err(..) }` lines): cases (a) and (f) must go RED (a non-member applies raw instead
    of refusing) — at (a) the review read the raw arm's own sentence, `Err((1, "5-Power bonus requires the action
    to have been taken at power 5"))`, against `apply_move`'s `move is not in the legal-move list for this state`.
  - M6 — one character changed in the rules-hash sentence: case (c) must go RED (`(strict mode reject)` against
    `(strict mode rejects)` in the review).
  - M7 — an `engine_version` gate added to `replay_strict_with`'s preamble (`if record.engine_version !=
    env!("CARGO_PKG_VERSION") { return Err(ReplayError { event_index: 0, message: "engine_version mismatch".into() }); }`):
    `an_engine_version_drift_is_accepted_under_both_checks` must go RED — the predicate refusing what `replay_strict`
    accepts (R-15).
  - M9 — a `Predicate` arm that skips the deck reveals, placed before the delegation (`(MoveCheck::Predicate,
    RecordedEvent::RevealOrders(_) | RecordedEvent::RevealTechs(_)) => Ok(state.clone()),`):
    `legal_reveal_events_fold_identically_on_the_success_path` must go RED (R-16).
  Record the five readings in the task report.

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

- [x] **Step 1: The "before" review — the orchestrator's own (DONE 2026-10-08; §"Review record").** On a `git archive` export of `master`, EXECUTE
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

- [ ] **Step 6: The measurement for S1's plan.** The review's four worst-case archive bodies sit, GITIGNORED, in
  `docs/temp/security-review-2026-10-08/worst-case/` (`spread_4p_11_body.json` is the 23,606-event record; each body
  is `{ "record": …, "table": … }`). The plan review measured them in release on the Mac (Darwin 27.0.0 arm64, the
  M5), same digest each way: `replay_strict` 29.04 s → predicate 54.1 ms (4P), 10.46 s → 40.1 ms (3P), 11.77 s →
  22.5 ms (2P); the null-turn 4P body 107 ms → 41.3 ms. Re-measure at the reviewed head through a scratch test
  (never committed) that parses a body's `record` and calls `replay_strict` then `replay_strict_with(.., Predicate)`;
  record the walls in the task report beside the review's — the figure S1's event-count cap and semaphore are sized
  from. Not a gate.

- [ ] **Step 7: The PR** against `master`, its description naming the ruling (R-SEC1, R-SEC2), the proof binary, the
  six mutants' readings, the gate sequence's wall, and the measurement; the owner's word merges it. Then the
  decision-log row for the core PR's close, the ledger's A2 row (`core landed <merge commit>`), and the board's
  Track 4 next step (S1's plan, written against the merged tree).

## Review record

### 2026-10-08 — the "before" review (Task 4, step 1), the orchestrator's own, on a `git archive` export of `master` at `d0816172`

**Method.** A script read this plan's four `rust` blocks out of the file and applied them to the export verbatim (no
retyping); the two module cells, the binary and `game_record` ran; the six mutants were applied one at a time by a
second script and restored; `cargo clippy -p yokohama-core --all-targets -- -D warnings` ran; then the corrections
below were applied and the whole battery, the mutants and clippy re-run; the review's worst-case archive bodies were
replayed both ways in release. Every name this plan spells resolved in the tree (`drive` 709 / `candidates` 743 /
`legal` 673, `route_enum.rs:689` `pub(crate)`, `game/mod.rs:136/138/144`, `errors.rs:17/33`, `apply_move.rs:12/222`,
`record.rs:348/478-520`, `common/mod.rs:269/282`; the `game_record.rs` ranges end a line or two early; the cited
"ends near line 902" is 900). The plan's code as written compiled, its two module cells and clippy were green, and
M1, M2, M3, M5, M6 read RED where it said. The exactness argument was re-derived arm by arm from the code, not from
this plan: mercenary targets, orders, achievements, University, Spinning Mill, placement A and B, `PlacePresident`,
the retreat product, the 5-Power pair and the building arms each mirror their generator exactly; area actions and
choice resolutions are delegated to the generator; the Movement family is the list's subsequence by
`movement_routes_family_matches_generator_across_driven_games` and `action_source_equivalence`.

**Findings and corrections** (each applied to this plan above):

| id | severity | finding | correction |
|---|---|---|---|
| R-1 | HIGH (vacuity) | Case (f) never ran: the 2P seed-7 record holds no walked president move and the cell's own guard panicked (`seed 7 at 2P held no walked president move`); every reading past it, M4's included, was masked. | (f) walks long recorded prefixes (400 moves, 2/3/4P × 3 seeds) ONCE with the decision state carried along, and stops at 8 witness tampers. |
| R-2 | HIGH (proof strength) | The repeated-hop witness is refused by `is_move_legal` itself (a repeated area fails `is_president_path_legal`'s step 3), so the record-level battery never exercised the family conjunct: M1 could not be read at the record level. | `witness_variants` (a detour through a neighbour adjacent to the start and the first hop, the direct hop, the reversed walk) builds candidates `PathMode::Canonical` ACCEPTS and the list lacks; under M1 the predicate replay answers `Ok(<the record's own digest>)` where `replay_strict` refuses at k (read at 2P seed 1, event 117). |
| R-3 | HIGH (a false mechanism sentence) | Review Focus 3 said the phase cell meets the queued-choice case "wherever a listed move's successor state queues a choice" — a Movement move's successor never queues a choice, so no movement move was ever judged under a queue; and spec §3.1's "(3) … `Predicate` without the phase gate fails (2)" had no record-level catcher (M3 was caught only by the module cells). | The membership cell judges every REAL member of the phase-blind family at every queued decision point (28 refusals read) and every non-Movement point (5,891), counters asserted; the binary's case (i) plants a family member at an AreaAction event (under M3 the predicate errs at 65 where `replay_strict` refuses at 64). |
| R-4 | MEDIUM (doc truth) | `is_listed_move`'s doc said it "never materializes the list"; `is_move_legal` DELEGATES `ExecuteAreaAction` to `get_legal_moves` at the AreaAction phase and the queue arm to `generate_choice_resolutions` (max 154 / 114 measured), so those two small lists are still materialized. | The doc and the Goal say what is true: the Movement phase's retreat product is what is never paid; the measurement (R-8) is the evidence. |
| R-5 | MEDIUM (duplication) | `fold_one_event_with`'s `Predicate` arm repeated the three `Reveal*` arms of `fold_one_event` — the drift that function's own doc names (the a6faa87 lesson). | `Predicate` differs for a `Move` alone; every other event, and `Listed`, is `fold_one_event` itself. |
| R-6 | LOW (a weak counter) | `wider_cases` counted `RulesLegal`-accepted non-members, a weaker fact than the conjunct needs. | `conjunct_cases` counts `Canonical`-accepted non-members (165 read). |
| R-7 | LOW (a cite) | Review Focus 1's `legal_moves.rs:289-300` is the Option-B comment. | The `zero_cost_destinations` suppression in the on-board arm, `:295-383`. |
| R-8 | LOW (Step 6's path) | `…/scratchpad/r2/export` is a session scratchpad, and the bodies sit in `r2/`, not `r2/export`. | The four bodies were copied to the gitignored `docs/temp/security-review-2026-10-08/worst-case/` and measured now (Step 6 carries the figures). |
| R-9 | noted | Neither the generator nor the predicate has a game-over gate; the phase machine parks a finished game at `TurnEnd`, where both answer nothing. | Case (h) pins it on a record played to its end. |

**Readings at the end of the review** (the strengthened cells; debug unless stated; the Mac, Darwin 27.0.0 arm64):

| what | reading |
|---|---|
| module cells | 22 passed (the module's 20 + 2), ~7 s; counters as Task 1 Step 4 states |
| the binary | 5 passed in 0.65 s; `tampers: walked 12, witness 5, repeated-hop 3, out-of-phase members 4; the finished game held 2515 events`; walls over 40 events strict 7.1 ms / predicate 4.5 ms |
| M1 | RED: membership cell (`[AreaId(6), AreaId(2)] → AreaId(2)` at Movement) AND case (f) (`left: Ok("5044ab7ceefc27fb") right: Err((117, …))`) |
| M2 | RED: membership cell, same witness |
| M3 | RED: membership cell (a `PlacePresident` at AdditionalActionSecond), phase cell (`[AreaId(5)] → AreaId(5)` at AreaAction) AND case (i) (`Err((65, …))` against `Err((64, …))`) |
| M4 | RED: case (g) (`Ok("935e76b39e1aafe9")` against `Err((1034, "invariant violated: P1 order hand size 4 > 3"))`) |
| M5 | RED: case (a) (the raw arm's `5-Power bonus requires the action to have been taken at power 5` against `apply_move`'s sentence) |
| M6 | RED: case (c) (`reject)` against `rejects)`) |
| clippy | clean, plan code and strengthened code alike |
| the worst case, release | `spread_4p_11` 23,606 events: strict 29.04 s / predicate 54.1 ms; `spread_3p_13`: 10.46 s / 40.1 ms; `spread_2p_14`: 11.77 s / 22.5 ms; `null_4p`: 107 ms / 41.3 ms — the same digest each way |

The scratch implementation stayed in the export and is discarded with it; nothing of it reaches the branch but
through Tasks 1–3's own TDD. The export's path and the run logs: the review session's scratchpad `review/`.

### 2026-10-08 — Task 1's review (Fable), the implementer's concerns, and the fix round

The implementer (Opus) reported DONE_WITH_CONCERNS at `585ca676`, the blocks byte-identical to this plan's, with two
doc-only concerns the Fable task reviewer confirmed as plan-mandated Important findings, both wording: **R-10** the
cells' doc attributed their refusals to the conjunction's ORDER — a swapped conjunction (the family first, then
`is_move_legal`) keeps both cells green with every counter identical, so what the cells pin is that both gates are
PRESENT (the family-alone mutant M3 turns both red), and the order is cost only (the function doc's "taken FIRST"
is a cost sentence and stays); **R-11** the phase cell's doc claimed the queue gate, which that cell never meets
(0 of its 207,129 checks judge a movement move under a queued choice — R-3's own finding, left in the cell's doc).
Minors: the function doc's intra-doc link to the `pub(crate)` family raised one `rustdoc::private_intra_doc_links`
warning (`cargo doc` is no gate; the link is now plain backticks); the brief's two anchors for the cells conflicted
(Step 1 said the module's end, the Files section the `(a) + (b)` cell) — the Files anchor stands, under a banner
line; the function doc cites Task 2's binary before it exists (resolves when Task 2 keeps the name, which it does).
The corrections are applied above to the Task 1 and Task 2 blocks (comments and assertion strings only — no code,
no cell, no counter moves), to Review Focus 3 and 6, and to Step 1; the code follows in the fix round's commit,
blocks byte-identical. On record from the implementer's own probe: 148,250,705 candidates over full-length games
(`drive(4, 8000, ..)`) and 22 hand-built fixtures, malformed ids included, 0 disagreements with list membership and
0 panics; 317 conjunct cases under the cells' own `drive(3, 400, ..)` sweep against the counters' 165; the direct
hop `[2]→2` on the diamond fixture is unlisted because `[1,3,2]` is the lex-least witness of the same label, and
`is_listed_move` refuses it exactly as `apply_move` does.

### 2026-10-08 — Task 1's fix round 1 (`c11664d4`) and the queue gate's own pin (R-12)

The implementer applied R-10 and R-11 byte for byte and then tried the fix's own sentences. **R-12:** "the cells
pin that both gates are PRESENT" held for the phase gate only — every one of the 28 queued members the membership
cell judges sits outside the Movement phase (queued points by phase in that sweep: AdditionalActionFirst 1,
AdditionalActionSecond 3, FivePowerBonus 10, Recovery 1; 106 points and 215 members at full length, none at
Movement), so the phase gate refuses each of them too, and a faithful single-gate mutant that drops only the queue
gate (the arm judging `is_move_legal` on a clone with the queue cleared) keeps both sweep cells green; dropping only
the phase gate turns both red. In engine-produced states a queued choice never coexists with the Movement phase, so
the queue gate's pin for the conjunction must be a hand-built state. Taken: the third cell
`a_queued_choice_refuses_a_listed_family_member_at_movement` (the module's `diamond` at Movement with an `AnyGood`
choice queued; the family lists the walk `[1]→1`, the list holds only the resolutions), mutant M3q, and the two
sentences now name which cell pins which gate. Also on record: the lead's `grep -c private_intra_doc_links` check
cannot discriminate (rustdoc names a lint only on its first warning); the counts that do are "links to private item"
warnings 91 → 90 and total rustdoc warnings 235 → 234 (234 is the count before Task 1).

### 2026-10-08 — Task 1's fix round 2 (`a6432c7e`) and the function doc's proof sentence (R-13)

The third cell, M3q and the two sentences landed byte for byte; M3q turns the new cell red alone, the two sweep
cells green with unchanged counters, and the implementer showed "a queued choice never coexists with the Movement
phase" holds by code, not by sampling: only the two placement arms enter Movement, a placement move is listed only
with an empty queue, and placement queues nothing. **R-13:** the function doc's "The proof:" sentence named the two
sweep cells only, and its last sentence still read "the legal families' sizes" where R-4 settled on the retreat
family — both corrected in the block above; the code follows as a new commit.

### 2026-10-08 — Task 2's implementer concerns before its review (R-14, R-15, R-16)

The implementer (Opus) transcribed the blocks byte for byte (5 cells green, M4–M6 red as quoted, the core suite
1,585 passed / 29 ignored = cycle F's 1,577 + Task 1's 3 + these 5, clippy clean) and then probed: legal reveal
events on the success path over 6 finished 2P games (30,714 events; 314 order reveals, 353 tech, 6 starting);
finished games at 2/3/4P (predicate 1.4–1.9× faster than strict on RANDOM play — the large gains are the adversarial
records'); 240 cross-state plants and 1,420 malformed-id plants over every phase and the queued state, no panics,
every reading equal to `replay_strict`'s and to the list's own membership. Three concerns, all taken at the source:
**R-14** the `Predicate` doc repeated R-4's over-claim ("without a list materialized per event", "stops following the
legal families' sizes") — reworded to the retreat family, with the delegated families and the Movement family's own
list named as still materialized; **R-15** the binary omitted three shapes `game_record.rs` builds (an illegal move
inserted mid-record, a dropped last event under the full run's digest, an engine-version drift strict replay
ACCEPTS) while two docs claimed every one — and a version gate added to `replay_strict_with` (M7) survived 5/5: the
accept side of "accepts EXACTLY the records `replay_strict` accepts" was unpinned on the one provenance key strict
replay deliberately does not gate (the 2026-07-27 decoupling), the key an archive upload from an older release
carries — cases (k), (m) and the cell `an_engine_version_drift_is_accepted_under_both_checks` with M7; **R-16** no
record carried a reveal on the success path, and a `Predicate` arm skipping the deck reveals (M9) survived 5/5 —
the cell `legal_reveal_events_fold_identically_on_the_success_path` (the probe's P1 at three seeds) with M9. The doc
sentences name the shapes they cover.
