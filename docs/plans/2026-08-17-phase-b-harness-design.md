# The driven-fixture harness — design (Phase B item 1)

**Written 2026-08-17 (Phase B analysis session). The spec is the ratified amendment's D2 harness
paragraph (`docs/plans/2026-08-11-phase3.5c-scoping-amendment.md`) — this design executes it and
adds nothing beyond its named contents. Branch `phase3.5c-harness`, PR onto master under OD-0.**

## 1. What it is

In-crate cells over the public agent surface for ROOTED runs: play from a supplied `GameState`
to terminal with named seats, honest per-seat observation, and recorded provenance. It is the
instrument that D7's mechanical separator, family 1's "genuinely dead" adjudication, and (later)
the layer-2 driven families run on. **Off the fifteen-gate roster** — a graduating fixture joins
`scenario_suite`'s never-delete registry, never this module. Not in `yokohama-cli`, not in
`yokohama-oracle`.

## 2. Placement

`crates/yokohama-core/src/game/driven.rs` beside `eval_screen.rs`; runner
`crates/yokohama-core/examples/driven.rs` on the `eval_probe`/`eval_screen_round` precedent.
**All tests live in the lib module (and, if an integration surface needs it, a `tests/` file) so
NO new documented test command is required** — deliberate contrast with `eval_probe`'s
example-hosted tests; the example stays thin argv glue over public functions. (The standing rule:
a suite no gate line names is a suite that never runs — this design avoids creating one.)

## 3. Public surface

- **`RootSource`** — where a root comes from:
  - `Constructed` (a test/fixture hands a `GameState` directly — the `scenario_suite` grain);
  - `RecordPrefix { record, event_index }` — `record::state_at` does the work; the five
    session-produced owner-game roots enter this way;
  - `Generated { predicate, master, ordinal }` — draw seeded self-play games at a recorded
    master and take the ordinal-th root satisfying the predicate. v1 ships the TYPE plus exactly
    one predicate, `FinalRoundAlreadyTriggered` (family 1's need); layer 2 adds variants at
    family freeze.
- **`ReplyPolicy`** — the amendment's declared menu: `FamilyWeighted` | `GreedyAtFinal` |
  `LookaheadAtFinal`. The fourth menu slot (one adversarial policy declared as a *rule over
  state*) is deliberately ABSENT in v1 — it is declared at family freeze (Phase C), and shipping
  a placeholder would invite hand-picked adversaries.
- **`SeatSpec`** — `Contestant(...)` | `Reply(ReplyPolicy)`; both materialize to the honest
  chain (observe → `BeliefSampler::from_view` → decide/pick → legality check → apply). Sampler
  failure is a **typed hard error with no fallback arm** — the arena's own posture, for the same
  corpus-honesty reason.
- **`drive_to_terminal(root, seats, rules, master, opts) -> DrivenOutcome`** — the generalized
  `drive_greedy_2p`: moves applied, final state + `final_result`, per-seat search tallies and
  counters (cap/budget/ply shapes), the §0.3(b) explanation-identity count, decision count. A
  generous decision-count guard turns a non-terminating constructed root into a **typed refusal,
  never a hang** (organic states always terminate; only hand-built fixtures can trip it).
- **`root_table(root, seat, contestant, rules, decision_seed) -> RootTable`** — the decision
  analyzed, built on the SAME high-level path `why`/the CLI use: `observe` →
  `Contestant::build(rules, seed)` (arena/stats.rs:637) → `choose(&view)`, reading the ranked
  root values off the explain surface. Reported: per-candidate values, top-set size, **the
  ≤1-point top-BAND width** (the e113 lesson: 100/139 inside one point — `tied_at_top` alone
  under-describes the degeneracy), distinct-value count, winner identity, and the FULL
  `SearchStats` counter set (the shipped `why` prints 2 of 15 counters; D7's step-one check
  needs all of them, so this table is where they become readable). **This is the mechanical
  separator's instrument** (same root, `lookahead-v1` at b=4 reproduction then b=8/b=16 via the
  knob grammar, compare band structure).
- **`adjudicate_root(root, seats, replications, opts)`** — family 1's adjudicator: force each
  root candidate, `drive_to_terminal` the remainder under the named reply policies, K seeded
  replications; report per-candidate final margins. v1 is playout-under-named-policy — **a
  robustness read, labeled so, never exact action ground truth**; the exact bounded-tree
  upgrade is layer-2 work and only family 1's in-final-round roots qualify for it.
- **Provenance** — `RootedProvenance { root descriptor (source kind + record digest/table or
  master+ordinal or constructed marker, event index, root-state digest), semantics_rev,
  rules_hash, seat descriptors (contestant echo / policy name), master }`, stamped on every
  outcome; the runner serializes outcome+provenance JSON to a sidecar path on request. **Never a
  `GameRecord`** — a rooted run is not a from-setup game and must not masquerade as one (no
  fake `new_game` seed, no replayable-from-setup claim).
- **Registry** — `driven::registry()`: named fixtures. v1 entries: the five session roots as
  `RecordPrefix` entries on `docs/temp/owner-games/` paths — **runner/analysis material,
  absent-tolerant with a typed miss naming the path; the TEST SUITE never depends on them**
  (corpus-free tests are a hard constraint — no new panic-when-absent pins, no new env var) —
  plus the constructed/self-generated entries the tests own.

## 4. Seeding and the drift pin

Two seed disciplines exist in the repo and the harness keeps them distinct rather than
inventing a third. **Driven runs use the ARENA discipline**: `agent_stream_seed(master, seat,
slot)` (arena.rs:219) seeds ONE long-lived per-seat RNG advanced across the whole run — masters
recorded before use by the caller, per standing policy. **Single-decision reproduction uses an
explicit `decision_seed`**: `root_table` takes it verbatim (a fresh agent per decision, the W5
shape); the CLI's own `splitmix64(bot_seed ^ event_count)` derivation stays in `yokohama-cli`
and is NOT re-implemented in core — a caller reproducing a recorded CLI decision passes the
seed the record implies (the runner takes `--decision-seed`).

The load-bearing honesty test is the **arena-reproduction drift pin**: from a `new_game` root
at a diagnostic master, `drive_to_terminal` reproduces `run_arena_game`'s game move-for-move.
Preferred implementation: split the arena's private per-game loop (`drive`, arena.rs:816 —
currently `PendingSetup`-fed: it finalizes setup then loops observe → choose →
`is_move_legal(Canonical)` → `apply_move_unchecked` → record/tally) into a setup half and a
play-from-`GameState` half that both drivers share — a behavior-preserving refactor of a
gate-covered module, so the review re-runs the arena gate smoke, the determinism cells, and the
committed goldens as its verification floor. If the code shape resists a clean split, the
harness hosts its own loop and the drift pin is the assertion that keeps it honest; either way
the pin is committed, not implied.

## 5. Reuse, and what this module deliberately does not do

Reused, not rebuilt: `search_world`/`Contestant::build` (root values via the shipped agents),
`record::replay_from` + `state_at` (replay-to-root; `state_at(record, n)` is the state the
mover decided in before event n — k counts events APPLIED, out-of-range is a typed refusal),
the probe's `snapshot_game` walk pattern and `observation_leakage`'s `decide_in_world` pattern.
The latter two are example-/test-local (`decide_in_world` is a private fn inside a tests
binary — crate code has no linkage path to it), so the harness hosts its own thin equivalents
and names the source it mirrors in-source. Deliberately absent: CLI/argv coupling
in-crate; gate registration; weight knobs (contestants only — a weight change is a versioned
controller change, L12); new `RecordedEvent` kinds; `GameRecord` emission; any denial-seeking
fixture semantics (canon 12/13/14).

## 6. Testing (TDD, corpus-free)

Determinism (same master → identical outcome digest, twice); the arena-reproduction drift pin;
a `state_at`-rooted continuation on a record generated in-test; the adjudicator on a constructed
near-terminal fixture with hand-computable margins; tie-structure fields on a constructed root
with known ties; reply-policy chain pins (the rung chain preserves W5's grandfathered
semantics); provenance round-trip; the non-termination guard's typed refusal; the sampler-failure
typed error. Counts recorded at close; per-edit gates after every task.

## 7. This session's consumers (after merge)

The F1/F5 mechanical-separator runs (b=4 reproduction, then b=8/b=16) recorded into the D7
triage; the family-1 adjudications at the two dead-final-turn roots (g1 e228, g3 e177–183).
