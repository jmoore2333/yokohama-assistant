# The driven-fixture harness — implementation plan

**Companion to `docs/plans/2026-08-17-phase-b-harness-design.md` (read it first; the amendment's
D2 harness paragraph is the spec of record). Branch `phase3.5c-harness`, its own PR onto master
under OD-0. Standing cadence: fresh Opus implementer per task (TDD failing-first, HEAD-guard
literal, never amend after DONE) → independent Opus reviewer (scratch harnesses, MUST/SHOULD/
NOTE, mutation probes) → orchestrator adjudicates, fix wave in its own commit(s), ledger row,
next task at the new HEAD. Per-edit gates after every task: the six cargo suites + four node
suites + clippy (own rc). Baseline counts at branch: core 1012+29+1 doc-test · cli 771+2 ·
oracle 191 · difftest 52+5 · bridge 81 · eval_probe 36.**

Module: `crates/yokohama-core/src/game/driven.rs`. Runner: `examples/driven.rs` (T5). ALL tests
in the lib module / existing suite structure — no new documented test command, no `#[ignore]`
tier, nothing joins the roster. Tests are CORPUS-FREE (no gitignored-file dependency, no new
env var, no new panic-when-absent pins).

**AMENDED 2026-08-17 after the cross-model design review (SOUND-WITH-FIXES; resolutions folded
into the design doc in place and into the task text below). T1 dispatched against the
pre-amendment text — its deltas (the record-free play-half seam, slot = seat index, the root
hygiene set) are T1 ADJUDICATION CRITERIA and land at its fix wave if the implementer's
latitude went elsewhere.**

## T1 — the driver: `drive_to_terminal` + the arena split + the drift pin

Split arena.rs's private `drive` (`:816`, `PendingSetup`-fed) into a setup half and a
play-from-`GameState` half, behavior-preserving; `drive_to_terminal(root, seats, rules, master,
opts)` drives the play half with agents built per seat (`SeatSpec::Contestant` via
`Contestant::build`; `SeatSpec::Reply(ReplyPolicy)` as sugar — `FamilyWeighted` =
`Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)`, `GreedyAtFinal` = `GreedyV1{m:8}`,
`LookaheadAtFinal` = `LOOKAHEAD_V1_DEFAULT`), arena seed discipline (`agent_stream_seed`),
sampler failure a typed hard error, decision-count guard a typed refusal. `DrivenOutcome`:
applied moves per seat, final state + `final_result`, per-seat tallies/counters, explanation
identity, decision count. RED first: determinism (same master → identical outcome twice), the
**arena-reproduction pin** (from a `new_game` root at a diagnostic master, move-for-move equal
to `run_arena_game`), the non-termination refusal, the sampler-failure error. Reviewer floor:
behavior preservation of the arena split verified the STRONG way — arena gate smoke +
determinism cells + committed goldens re-run, plus reviewer-chosen drift cells against
pre-split code.

## T2 — `root_table` (REWRITTEN per review MUST-1/S-8/N-1)

`observe` → CONCRETE agents constructed by MIRRORING `Contestant::build`'s arms
(`GreedyAgent::with_weights` / `LookaheadAgent::with_knobs(...).with_cap(...)`, naming
`WeightTable::V2` exactly as build's arms do — no weight parameter surfaces), because `build`'s
`Box<dyn ArenaAgent>` erases the concrete `last_roots()`/`last_stats()` surface the ranked
values live on and `ExplainHandle` carries counters only; this is the CLI's own W5 mirror
pattern, and the harness carries the same DRIFT PIN — mirror-vs-build move identity at the
same seed, per supported arm. Seat support: `lookahead-v1` + `greedy-v1` supported; `Policy`
rungs refused typed (sampled-not-valued); mcts/atomic-diag refused in v1. Report: top-set
size, the ≤1-point top-band width, distinct-value count, winner, full `SearchStats` (all
fifteen counters — the D7 step-one surface), and the SAMPLED-WORLD identity (sampler seed +
world digest) so the separator's same-worlds-across-beams premise is recorded. Knob-varied
contestants come through the existing grammar (`lookahead-v1:b=8`). RED: a constructed root
with known ties (band fields exact), counters propagated (uncapped shape `0/0/0`), determinism
at fixed seed, the mirror-vs-build pin, the rung typed refusal, and a cell proving b=4 vs b=8
tables can differ (anti-vacuity for the separator's discriminating power — any legal
difference in band structure counts; no claim about WHICH way it moves).

## T3 — `RootSource` + provenance + registry (AMENDED per review S-1/S-3/S-5/S-6/N-3/N-5)

`Constructed` | `RecordPrefix{record, event_index}` (via `state_at` — event_index is the
decided-in position, matching `why n`) | `Generated{predicate, master, ordinal}` with exactly
one predicate, `FinalRoundAlreadyTriggered` — the Generated DESCRIPTOR also records the
generating ecology (driver, seats, seed derivation), since `(predicate, master, ordinal)`
alone re-points if the generating play changes. `RootedProvenance` (root descriptor + digests
+ `semantics_rev` + `rules_hash` + seat descriptors carrying the RESOLVED contestant echo and
observation label — Reply seats included — + master), stamped on every `DrivenOutcome`; serde
round-trip. Root-eligibility precheck: hidden-deal seats (`starting_order_hidden`) refused up
front by name. `driven::registry()` with the five session entries as `RecordPrefix` on
`docs/temp/owner-games/` paths (workspace-root-relative, manifest-dir resolved) — runner
material, absent-tolerant typed miss — each entry carrying its recorded `decision_seed` and
recorded move where known (the triage §3 seeds), so reproduction runs by default. TESTS use a
record generated in-test (small arena game → `state_at(k)` → drive to terminal → termination +
provenance pins). Generated-arm test: a seeded self-play scan finds an ordinal-th final-round
root deterministically. Graduation mechanics stated in the design §3 (committed serialized
fixture with digest cross-check; never a `RecordPrefix` into `scenario_suite`).

## T4 — `adjudicate_root` (AMENDED per review S-2/N-7)

Force each root candidate, drive the remainder to terminal under named reply seats, K seeded
replications — per-replication worlds via `mix(SALT_DRIVEN_REPLICATION, [master,
replication_index])`, then `agent_stream_seed(world, seat, slot)`; never an ad-hoc third
discipline. Per-candidate final-margin table, labeled a robustness read (playout under named
policy, never exact ground truth) — and the label rides the RUNNER'S printed output in T5, not
only the docs. RED: a constructed near-terminal fixture with hand-computable margins (the
scenario_suite grain — equal-banked devices where needed); a genuinely-dead constructed root
reads all-candidates-equal; a terminal root returns the zero-move outcome; K>1 replication
variance recorded not asserted.

## T5 — the runner + session entries + close

`examples/driven.rs`: `table <fixture|path:event> --contestant <spec> --decision-seed <u64>`,
`run <fixture> [--seats a,b] [--master <u64>] [--replications K]`, `adjudicate <fixture> ...`,
`list`. Thin argv glue over public functions (USAGE names every knob; unknown anything is an
error, eval_probe's discipline; `adjudicate` output prints the robustness-read label). Registry
carries the five session roots: g1:113 (36-tie), g3:63 (26-tie), g3:93 (+116 control), g1:228
(dead-recall tie), g3:178 and g1:239 (the two final-turn roots). **Root-index rule, stated and
VERIFIED (review S-4): roots are taken at the turn's FIRST BRANCHING decision — the preceding
`EndAdditionalPhase` events were checked forced via `why` (g1 e238: 1 root candidate, 'No
runner-up'; g3 e177: 1 root candidate, same), so g1:239/g3:178 skip nothing.** Close:
CLAUDE.md architecture bullet + counts line, ledger row, guide pair UNTOUCHED (no CLI change),
branch gates green, PR body per house style.

## Sequencing and adjudication

T1 → T2 → T3 → T4 → T5, strictly serial, orchestrator adjudication between tasks. T1's
implementer and reviewer run at max effort (gate-covered refactor); T2–T5 at xhigh. The
orchestrator never edits the worktree while an implementer runs; fix waves are committed before
any revert-style mutation probe (the W4/T3 lessons, verbatim in the kickoff).
