# Phase C — the four driven families: THE FREEZE

**Frozen 2026-08-18 (branch `phase3.5c-families`), per the amended design
`docs/plans/2026-08-18-phase-c-families-design.md` §4–§5 and plan §T7 step 2. This document
plus the freeze-wave code commit ARE the freeze: the declarations in
`game/driven_families.rs::families()` carry the frozen masters, ordinals, and sizing as code,
and this document carries what code must not: the escrow, the extension ordinals, the error
arithmetic, the canon citations, and the WHAT-THIS-DOES-NOT-MEAN block. Every parameter below
was sized from the PILOT readings (draws 0..4 per arm, logs preserved in the session
scratchpad and quoted in the runs report) and frozen BEFORE any confirmatory draw. Nothing
confirmatory quotes a pilot.**

## 1. The subject

`lookahead-v1@v2{b=4,eval_impl=3,k=1,m=8,weights=-7339724794018772210}` — the shipped
provisional production contestant at `SEMANTICS_REV` 7, PROVISIONAL until D1's identity
freeze. The subject is not a runner flag; a family result is a statement about this echo.

## 2. The frozen families (labels, generators, masters, blocks, rules)

Masters from the ledger row `ee8c8c7` (recorded before any run; serials 0xA3E1 2026 0818
0004–000A consumed by the arms below; 000B–000E RESERVED-UNUSED — the landed single-master
design derives adjudication worlds and read streams from the block master under different
salts (0006/0008), so the separately-enumerated read masters were never needed; they stay
reserved, never redrawn; 000F = the opening probe).

| family | label | class | arm | generator | master | pilot | confirmatory | rule |
|---|---|---|---|---|---|---|---|---|
| final-round | CONFIRMATORY | rules/final-round | positive | `FinalRoundAlreadyTriggered` @ v1.1 rung ecology | …0004 | 0..4 | 4..20 | AdjudicatedBest ε=2 VP, decision grain |
| final-round | 〃 | 〃 | control | `TriggerRoundNotFinal` @ v1.1 (own ordinal space, same master) | …0004 | 0..4 | 4..20 | 〃 |
| end-trigger | CONFIRMATORY | end-timing/proximity | positive | `EndFirableAheadInTruth` @ v2 lookahead ecology | …0005 | 0..4 | 4..28 | Turn(EndFired) |
| end-trigger | 〃 | 〃 | control | `EndFirableBehindInTruth` @ v2 | …0006 | 0..4 | 4..28 | Turn(NotEndFired) |
| exclusive-race | EXISTENCE-DEMONSTRATION | opponent-reply | positive | `build_race_positive` seeds | …0008 | 0..4 | 4..28 | Turn(ContestedStepTaken) |
| exclusive-race | 〃 | 〃 | control | `build_race_no_denial` seeds | …0009 | 0..4 | 4..28 | Turn(DenialNotTaken) |
| achievement | CONFIRMATORY | end-timing/proximity | positive | `AchievementClaimAvailable` @ v2 | …0007 | 0..4 | 4..28 | Turn(AchievementClaimed) |
| achievement | 〃 | 〃 | corner | `build_achievement_proximity` seeds | …000A | 0..4 | 4..28 | Turn(AchievementClaimed) |

Supply capacity at the frozen ecologies: v1.1 (16 games × cap 2) = 32 strided ordinals; v2
(24 × 2) = 48. Confirmatory demand fits with extension + escrow (§4) exactly at capacity.
The proximity corner reads design §1's produce-to-7 sketch as within-window two-decision
completability via the Textile Mill conversion (T5's declared, review-endorsed reading — the
recording line the T5 review asked for). The `ExclusiveRaceOneStep` generated arm was NOT
frozen: its pilot role was supply measurement; the builder arm is the family's honest source
per the design's fallback clause.

## 3. The frozen sizing and admission semantics

- **R = 20** replications per admitted root; **per-root F = 1 of 20** (both grains). The
  in-code constants keep their draft names (`PROVISIONAL_REPLICATIONS`,
  `PROVISIONAL_PER_ROOT_F`) — THIS DOCUMENT freezes their values for Phase C's confirmatory
  reads; the names stay draft because W7's pre-registration may re-derive them.
- **Admission K = 8** at confirmatory (`FROZEN_ADMISSION_K` via `BlockOpts::freeze()`;
  pilots ran `PILOT_ADMISSION_K` = 4 — the K raise is noise-reduction on group means, priced
  trivial by the pilot walls; `family_block_opts` hands freeze() to confirmatory blocks and
  pilot() to pilot blocks, both arms, admission inversion off in both — seam-pinned).
- **δ = 5 VP** (`DELTA_VP`, unmoved from the pilot posture: the pilot's clean dominances
  read 8–38; its marginal fails −7..+4.5 — δ=5 separates the shapes the pilots actually
  produced).
- **ε = 2 VP** (`FROZEN_EPSILON_VP`, family 1 — was 1 at pilot; pilot evidence: ε=1 at K=4
  produced UNRESOLVED at every trigger-round root — adjudication noise, not established
  policy disagreement; the ε=2 top set provably contains ε=1's, leniency direction pinned).
- **Pilot-reproducibility caveat**: `BlockOpts::pilot()` is byte-unchanged, so a pilot
  re-run draws the same roots and adjudications; the JUDGMENTS over them moved with this
  freeze (forced exclusion, splitting conjunction, NOTHING ADMITTED, ε) — recorded pilot
  verdicts are the pre-freeze instrument's and are not expected to reproduce.
- **Admission splitting rules** (pilot-informed noise fixes, frozen in code): a policy
  ALL-TRUE or ALL-FALSE across its playouts is NON-DEMONSTRATING (recorded, non-blocking);
  a policy splits only at min(true_n, false_n) ≥ 2; admission needs ≥ 2 splitting policies,
  all dominating (control: inverse); a SPLITTING dissenter still blocks, and the
  lookahead-sole-dissenter guard keys on splitting policies only. The pilot mechanisms these
  answer: the family-weighted rung's 1-of-8 accidental splits, greedy's structural
  never-claims, the race-rusher's near-universal claims.
- **Forced-root exclusion** (decision grain): candidates < 2 ⇒ EXCLUDED/FORCED.
- **DEAD** (decision grain): zero wins under EVERY declared policy AND per-policy mean
  spread ≤ 4 VP — AND-ed across the menu.
- **Menus**: all four reply policies for every family (the splitting rules, not menu
  pruning, absorb the degenerate shapes — deliberately, so no policy is dropped by tuning).
- **Family-level allowance: 0 failed roots** (families 1/2/4); family 3 declares NO pass
  bar (existence arm — per-root gap output only). **VOID below 4 admitted** of the
  confirmatory draws; disposition: ONE extension to §4's pre-declared ordinals, else
  record-and-route as under-powered.

## 4. Extension and escrow ordinals (pre-declared; escrow lives ONLY here)

| family/arm | extension (VOID disposition) | escrow: post-fix re-read | escrow: final-controller |
|---|---|---|---|
| final-round (each arm) | 20..26 | 26..29 | 29..32 |
| end-trigger (each arm) | 28..36 | 36..42 | 42..48 |
| exclusive-race (each arm) | 28..36 | 36..42 | 42..48 |
| achievement (each arm) | 28..36 | 36..42 | 42..48 |

Burning = running table/adjudicate/read at an ordinal; supply counting does not burn. No
grammar names an escrow block; the `gen:` escape hatch reaches these ordinals, so protection
is PROCEDURAL — the runs report's commands-as-record is the audit. A games-bound raise at a
fixed master and cap is PREFIX-STABLE (a version-bumped extension, never a re-draw).

## 5. Error arithmetic (the declared benign-miss model)

Per-root F=1/20 means a root fails on ≥2 misses. Under a benign per-world miss rate p
(world-sensitivity of an honest subject at K=1 sampling): p=1% ⇒ per-root false-fail ≈1.7%,
≈10% family-level at 6 admitted roots; p=2% ⇒ ≈6% per root, ≈31% family-level. The second
layer is D7 routing: a 2/20 miss pattern reads as world-sensitivity and a 20/20 as
systematic — the report's per-root miss counts are what keep a benign trip from mis-routing.
A family FAIL is therefore a ROUTE-TO-D7 event, never a standalone conclusion.

## 6. Reply-policy menu (fixed at this freeze)

`family-weighted` (rung), `greedy-at-final`, `lookahead-at-final` (both provisional-until-D1
bindings, resolved echoes recorded on every report), and **`race-rusher`** — the declared
adversarial fourth slot, a rule over state: probe {AdditionalAction, AreaAction,
ChoiceResolution} candidates only, classify on the agent's own derived probe world
(claim → church progress → customs progress, queue-aware), take the first of the highest
class in legal-list order, else delegate the decision to an owned mirrored greedy agent.
A positional opportunist, not a planner (movement never probed) — do not over-read the
hostile arm. Admission seats = the same policy at every seat; playouts are robustness reads
under named policies, never ground truth.

## 7. Canon citations (the judgment priors each family encodes)

Family 1: rules-derivable (no canon needed). Family 2: canon 15 (the inverted brake —
guidingpath 2026-08-09) — fire-when-ahead-in-truth; the behind control passes on NOT firing
(the owner's F9 sentence). Family 3: canon 12/13 (race, never denial-as-default) + C-A
(participation value) + C-B-DISCOUNTED (recoverable flips price with opportunity cost) —
guidingpath 2026-08-18. Family 4: ruling 14 (achievements are agenda-setters) + C-A; no
denial claim — both seats can claim. The C-D probe (lab rush 97.5% [0.9428, 0.9893] at 200
fresh boards) rides the runs report, not a family.

## 8. Multiplicity and the strategic-class partition

One family-level failure → disposition ledger + D7 branch-0 triage first; two or more IN THE
SAME CLASS → the pivot conversation. The partition: {end-trigger, achievement} =
END-TIMING/PROXIMITY; {exclusive-race} = OPPONENT-REPLY; {final-round} =
RULES/FINAL-ROUND. Family 3's expected failures cannot combine with family 2/4's.

## 9. Instrument caveats carried into this freeze

- `placement_cap_binds` ANNOTATES rather than refuses: sound for admitted family roots
  (width ≤ 60 by predicate conjunct), and a `gen:`-escape or hand-supplied money-saturated
  placement root CAN cap-truncate at 512 with only an annotation — the width bound is the
  dependency (T4 review NOTE-1, recorded here as promised).
- The adjudicator is a ROBUSTNESS READ under named policies, never exact ground truth; the
  exact bounded-tree adjudicator exists only as a named trigger: a LIVE family-1 root whose
  per-policy top sets disagree AT THE FROZEN PARAMETERS reads UNRESOLVED-pending-exact, and
  building the exact instrument is the named next step if confirmatory reads produce that
  class on live roots.
- Pilot readings are development material: the admission rates that sized these blocks
  (family 2 ≈1/4 per arm; families 3/4 lower) are screening figures, and the confirmatory
  admission rates are the recorded ones.

## 10. WHAT THIS FREEZE DOES NOT MEAN

Checkpoint evidence about a PROVISIONAL subject, never GO evidence; a family PASS confirms
one canon behaviour at admitted roots under a robustness-read instrument — it is not a
strength claim, not the bar, and no Phase-3 claim rides it; a family FAIL routes through D7
and is never a standalone verdict; EXISTENCE-DEMONSTRATION output supports a pivot or a
routing, never a GO; every figure is 2P at `SEMANTICS_REV` 7 under `WeightTable::V2` and
non-comparable across any future controller/impl bump.
