# The driven-fixture harness — design (Phase B item 1)

**Written 2026-08-17 (Phase B analysis session). The spec is the ratified amendment's D2 harness
paragraph (`docs/plans/2026-08-11-phase3.5c-scoping-amendment.md`) — this design executes it and
adds nothing beyond its named contents. Branch `phase3.5c-harness`, PR onto master under OD-0.
AMENDED same day after the cross-model design review (owner-directed opus/fable combo; verdict
SOUND-WITH-FIXES, 2 MUST / 8 SHOULD / 8 NOTE): the resolutions are folded IN PLACE below — the
load-bearing two were the `root_table` construction path (`Contestant::build`'s `ExplainHandle`
carries counters only, so the ranked values are unreachable through the box; the corrected path
mirrors build's arms with a drift pin, the CLI's own W5 pattern) and the record-free play-half
seam (stated explicitly so no placeholder `GameRecord` ever threads through a rooted run). T1
was dispatched against the pre-amendment text; its deltas land at the T1 fix wave.**

**Traceability note (review N-7): `drive_to_terminal` + the registry + reply policies +
provenance are D2's four named additions verbatim; `root_table` and `adjudicate_root` are the
executable forms of two owner-ruled consumers — D7's mechanical separator ("reproduce the
frozen root at a larger beam") and D2 family 1's exact-adjudication clause ("force each
alternative and play out") together with F5's pending disposition. Nothing else rides in.**

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
  a placeholder would invite hand-picked adversaries. **The `*AtFinal` bindings are PROVISIONAL
  until D1's identity freeze** (review S-1): today they resolve to the shipped defaults
  (`GreedyV1{m:8}`, `LOOKAHEAD_V1_DEFAULT`) exactly as `bot.rs`'s `PROVISIONAL_DEFAULT_BOT`
  does, carry the same provisional-until-D1 comment with a re-point obligation, and provenance
  records the RESOLVED echo so a frozen family's reply identity cannot silently move under it.
- **`SeatSpec`** — `Contestant(...)` | `Reply(ReplyPolicy)`; both materialize to the honest
  chain (observe → `BeliefSampler::from_view` → decide/pick → legality check → apply). Sampler
  failure is a **typed hard error with no fallback arm** — the arena's own posture, for the same
  corpus-honesty reason.
- **`drive_to_terminal(root, seats, rules, master, opts) -> DrivenOutcome`** — the generalized
  `drive_greedy_2p`: moves applied, final state + `final_result`, per-seat search tallies and
  counters (cap/budget/ply shapes), the §0.3(b) explanation-identity count, decision count. A
  generous decision-count guard turns a non-terminating constructed root into a **typed refusal,
  never a hang** (organic states always terminate; only hand-built fixtures can trip it).
  **The record seam, stated (review MUST-2): the shared play half is RECORD-FREE** — it returns
  the applied `(seat, Move)` list plus final state/tallies, and the ARENA's setup wrapper folds
  that list into its own `GameRecord` afterward, byte-identically (the record is a pure fold of
  pushes; error paths drop it exactly as today; the digest stamps after the fold — the goldens
  and determinism cells verify). No `GameRecord` — placeholder, scratch, or otherwise — is ever
  constructed inside `driven.rs`. **Root hygiene (review S-7/N-4/N-5):** the incoming root is
  `check_invariants`-guarded on entry (a typed refusal NAMING THE ROOT, never misattributed to
  the first move — the arena's and scenario_suite's own posture); per-move invariants ON by
  default, any off-switch following the arena's documented bulk-generation posture; a
  seats-list length disagreeing with the root's player count is a typed error; an
  ALREADY-terminal root returns a zero-move outcome (documented and pinned, not a refusal —
  the adjudicator relies on it).
- **`root_table(root, seat, contestant, rules, decision_seed) -> RootTable`** — the decision
  analyzed. **Construction (review MUST-1): `observe` → CONCRETE agents constructed by
  MIRRORING `Contestant::build`'s arms** (`GreedyAgent::with_weights` /
  `LookaheadAgent::with_knobs(...).with_cap(...)`, naming `WeightTable::V2` exactly as build's
  arms do — no weight parameter surfaces, L12) — because `build`'s `Box<dyn ArenaAgent>` erases
  the concrete `last_roots()`/`last_stats()` surface the ranked values live on, and
  `ExplainHandle` carries counters only. This is the CLI's own W5 pattern (`bot.rs`/`suggest.rs`
  mirror build, drift-pinned); the harness carries the same **drift pin**: mirror-vs-build move
  identity at the same seed, per supported arm. **Seat support named (review S-8; amended at T2
  adjudication — the orchestrator's ratified narrowing):** `lookahead-v1` supported;
  `greedy-v1` REFUSED typed as `Unranked` — `GreedyAgent` exposes no ranked-roots surface
  anywhere (no `last_roots`, no `last_stats`, no `SearchStats` at all), the CLI itself serves
  greedy movers by DECLARING that absence (`why.rs`'s `ONE_PLY_ACCOUNT` swapped in by arm rather
  than a table printed), and constructing a table here would require a second evaluation
  derivation outside the agent — which this design's own MUST-1 resolution forbids; `Policy`
  rungs REFUSED typed as `Sampled` (sampled-not-valued, `why`'s posture); mcts/atomic-diag
  REFUSED typed as `Diagnostic` — a SCOPE refusal, not a capability one: mcts-v1 genuinely has
  no ranked-root surface, while the S2 atomic-diag arm IS a `LookaheadAgent` (`Contestant::build`
  routes it through the same `build_search`) and would mirror trivially, and is refused only as
  a labelled diagnostic no v1 consumer asks a table about (revisit with a consumer, not with a
  plumbing change). Reported: per-candidate values, top-set size, **the
  ≤1-point top-BAND width** (the e113 lesson: 100/139 inside one point — `tied_at_top` alone
  under-describes the degeneracy), distinct-value count, winner identity, the FULL
  `SearchStats` counter set (the shipped `why` prints 2 of 15 counters; D7's step-one check
  needs all of them, so this table is where they become readable), and **the sampled-world
  identity — the sampler seed (and a world digest) — so the separator's same-worlds-across-
  beams premise is recorded, not inferred** (review N-1; the lookahead agent draws its three
  stream words before any beam-dependent work, so equal decision seeds sample equal worlds at
  b=4/8/16 by construction). **This is the mechanical separator's instrument** (same root,
  `lookahead-v1` at b=4 reproduction then b=8/b=16 via the knob grammar, compare band
  structure).
- **`adjudicate_root(root, seats, replications, opts)`** — family 1's adjudicator: force each
  root candidate, `drive_to_terminal` the remainder under the named reply policies, K seeded
  replications; report per-candidate final margins. v1 is playout-under-named-policy — **a
  robustness read, labeled so, never exact action ground truth — and the label rides the
  RUNNER'S PRINTED OUTPUT, not only this document** (review N-7); the exact bounded-tree
  upgrade is layer-2 work and only family 1's in-final-round roots qualify for it.
  Per-replication worlds derive as §4 states (review S-2), never ad hoc.
- **Provenance** — `RootedProvenance { root descriptor (source kind + record digest/table or
  master+ordinal or constructed marker, event index, root-state digest), semantics_rev,
  rules_hash, seat descriptors, master }`, stamped on every outcome; the runner serializes
  outcome+provenance JSON to a sidecar path on request. **Seat descriptors record the RESOLVED
  contestant echo (knobs and all) plus the observation label, for Reply seats too** — a menu
  name alone is a symbol whose meaning moves at D1's identity freeze, and the amendment's
  "reply policies fixed at family freeze" needs the resolved identity on disk (review S-1/N-5).
  For `Generated` roots the descriptor also records the GENERATING ECOLOGY — driver, seats,
  and seed derivation — since `(predicate, master, ordinal)` alone re-points to a different
  root if the generating play ever changes (review S-5). **Never a `GameRecord`** — a rooted
  run is not a from-setup game and must not masquerade as one (no fake `new_game` seed, no
  replayable-from-setup claim).
- **Registry** — `driven::registry()`: named fixtures. v1 entries: the six session roots as
  `RecordPrefix` entries on `docs/temp/owner-games/` paths (WORKSPACE-ROOT-relative, resolved
  against the crate's manifest dir so the miss never fires CWD-dependently — review N-5) —
  **runner/analysis material, absent-tolerant with a typed miss naming the path; the TEST SUITE
  never depends on them** (corpus-free tests are a hard constraint — no new panic-when-absent
  pins, no new env var) — plus the constructed/self-generated entries the tests own. **Entries
  carry the recorded `decision_seed` and the RECORDED MOVE where known** (review S-3): the
  separator's b=4 reproduction check then runs by default, printing reproduced/diverged,
  instead of on hand-copied hex; `--decision-seed` overrides. **Durability and graduation
  (review S-6):** these roots live only in gitignored files (D10's NAS sidecar is the sole
  backup — said out loud); when a root becomes load-bearing for a committed fix or graduates
  toward `scenario_suite`, its `GameState` is serialized into a committed fixture (or
  re-constructed as a builder) with the provenance digest cross-checked — `scenario_suite`
  stays corpus-free, so graduation is never a `RecordPrefix`.

## 4. Seeding and the drift pin

Two seed disciplines exist in the repo and the harness keeps them distinct rather than
inventing a third. **Driven runs use the ARENA discipline**: `agent_stream_seed(master, seat,
slot)` (arena.rs:219) seeds ONE long-lived per-seat RNG advanced across the whole run — masters
recorded before use by the caller, per standing policy. **Single-decision reproduction uses an
explicit `decision_seed`**: `root_table` takes it verbatim (a fresh agent per decision, the W5
shape); the CLI's own `splitmix64(bot_seed ^ event_count)` derivation stays in `yokohama-cli`
and is NOT re-implemented in core — a caller reproducing a recorded CLI decision passes the
seed the record implies (the runner takes `--decision-seed`; registry entries carry recorded
seeds per §3). **Two derivations named so no third discipline gets invented (review S-2):
slot = the seat's index in the seats list** (and the drift pin's arena-side factory uses the
same assignment — that is what makes move-for-move equality expressible); **per-replication
adjudicator worlds derive via `mix` with a new named salt** (`SALT_DRIVEN_REPLICATION`, the
arena's own `pub(crate)` mix — `mix(salt, [master, replication_index])` → world, then
`agent_stream_seed(world, seat, slot)` per seat). **A continuation from a mid-game root does
NOT reproduce the source game's tail and never claims to** (review N-2): fresh agents start
their streams at position 0 where the original game's agents were mid-stream — the rooted
tests pin termination and provenance, not tail equality.

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
and names the source it mirrors in-source. **Root eligibility precheck (review N-3): a root
whose seats carry hidden deals (`starting_order_hidden` — live-tracked records) is REFUSED up
front by name, not left to surface as a mid-run `observe` failure at that seat's first
decision**; the five session roots are sim records, so this is forward-looking hygiene.
Deliberately absent: CLI/argv coupling
in-crate; gate registration; weight knobs (contestants only — a weight change is a versioned
controller change, L12); new `RecordedEvent` kinds; `GameRecord` emission; any denial-seeking
fixture semantics (canon 12/13/14).

## 6. Testing (TDD, corpus-free)

Determinism (same master → identical outcome digest, twice); the arena-reproduction drift pin;
a `state_at`-rooted continuation on a record generated in-test; the adjudicator on a constructed
near-terminal fixture with hand-computable margins; tie-structure fields on a constructed root
with known ties; the reply-policy SUGAR-EQUIVALENCE pin — `Reply(FamilyWeighted)` byte-identical
to `Contestant::Policy(FAMILY_WEIGHTED_DEFAULT)` through the driver (review N-6: W5's
grandfathered splitmix chain is the CLI's pinned property, not this module's; the harness rung
chain is the arena's `PolicyAgent` chain by declared discipline); the `root_table` mirror-vs-
build drift pin; provenance round-trip; the non-termination guard's typed refusal; the
sampler-failure typed error; the terminal-root zero-move outcome. **Wall discipline (review
N-8): loop-equality tests prefer rung/greedy seats** (a debug lookahead game is ~3s; the W5
`why`-pin's 22.5s is the standing cautionary figure), and the module's debug wall is recorded
at close. Counts recorded at close; per-edit gates after every task.

## 7. This session's consumers (after merge)

The F1/F5 mechanical-separator runs (b=4 reproduction, then b=8/b=16) recorded into the D7
triage; the family-1 adjudications at the two dead-final-turn roots (g1 e228, g3 e177–183).
