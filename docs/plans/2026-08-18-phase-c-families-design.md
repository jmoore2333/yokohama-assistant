# Phase C — the four driven families frozen: design

**AMENDED same day after the Fable design review (the standing opus/fable cadence; verdict
SOUND-WITH-FIXES, 6 MUST / 9 SHOULD / 6 NOTE, ALL accepted): the resolutions are folded IN
PLACE below. The load-bearing six — the DEAD-root criterion re-defined to the measured F5
shape (`all_candidates_equal` would have classified all three session-dead roots LIVE); root
admission for the turn-grain families re-defined as PER-PLAYOUT hand-off-verdict partition
(candidate-level classification is uncomputable at the proximity corner); F set at freeze from
pilot world-sensitivity instead of a blanket 0 (unstated family-level false-alarm arithmetic);
the lookahead-sole-dissenter exclusion rule plus a minimum admitted denominator (admission
circularity); RaceRusher's classifier made queue-aware (a ≥2-eligible church/customs action
queues a choice and places nothing — the probed clone shows no gain); and per-game draw
striding (consecutive ordinals cluster ~14 to a game and are not independent draws).**

**Written 2026-08-18 (Phase C session). The spec is the ratified amendment's §3 Phase C plus
D2's layer-2 paragraph (`docs/plans/2026-08-11-phase3.5c-scoping-amendment.md`) — this design
executes them and adds nothing beyond their named contents plus the C-D-directed opening probe.
Branch `phase3.5c-families`, one PR onto master under OD-0. Inputs, all landed: the four canon
rulings (C-A/C-B-discounted/C-C/C-D, guidingpath's 2026-08-18 canon row), the 2026-08-09 canon
row (rulings 12/13/14/15, D3-to-hand), the D7 triage §3's measured verdicts (e113 ⇒
REPRESENTATION, e63 ⇒ HORIZON, all three family-1 session roots dead), the V2-play counters
baseline, and the merged harness (PR #23). ASK 2 is confirmed (the harness merge was not
D2-major).**

**The subject.** Every read in this phase runs against the shipped provisional contestant —
`lookahead-v1` at shipped knobs uncapped, `WeightTable::V2`, `SEMANTICS_REV` 7 (echo recorded
verbatim in the freeze document) — which is PROVISIONAL until D1's identity freeze. Phase C
findings are checkpoint evidence and D7 routing inputs, never GO evidence; the escrowed blocks
exist so post-fix re-reads and the eventual final-controller reads are uncontaminated.

**The STOP fence (amendment §3, verbatim obligation).** This phase freezes, runs, and ROUTES.
It does not fix what the routings reveal: no V3 mini-cycle (no weight change, no feature
admission, no `Feature::ALL` movement), no D3 capability build, no W6 knob work, no
preserved-rung edit, no `SEMANTICS_REV` or oracle change. A routing that lands on the e113
admission ladder or the e63/F2 depth lane is recorded as an owner ask in the close handover.
Consequence for D2's game cadence: if no branch-0 fix moves play behavior, this phase's merge
is NOT D2-major and no owner game rides it (final call at close, the standing pattern).

## 1. What gets frozen (per family)

A frozen family = a declared root GENERATOR + declared reply policies + pre-declared R/failure
counts + a declared PASS RULE + a CONFIRMATORY or EXISTENCE-DEMONSTRATION label + disjoint
pilot/confirmatory/escrow blocks. Generators are rules-plus-draws, never positions picked after
seeing play: a `Generated` scan (predicate + master + ordinal, ecology recorded) or a
PARAMETERIZED BUILDER (a constructing function + declared parameter ranges + seed-blocked
draws — the honest constructed form; a builder draw is a generator draw, and hand-picking
survives only in the pilot layer where nothing confirmatory rides it).

**Two read GRAINS, chosen per family and frozen with it.** Family 1 reads at the DECISION
grain: the subject's pick at the root, against an adjudicated top-set — right for final-round
roots, where the adjudicator forces exactly that decision's candidates. Families 2–4 read at
the TURN grain: drive the subject's WHOLE TURN from the root (a fresh instrument, §2's
`drive_turn`: the mover's decisions until the hand-off — `current_player` changes — or game
over) and evaluate a TURN PREDICATE on the hand-off state. The grain matters because canon 15,
F8 and the race question are all statements about what a turn accomplishes, not about move
ordering inside it: a mover who banks its area actions first and fires the trigger with its
last action played CORRECTLY, and a decision-grain read at the root would score that correct
turn as a failure (F8's own live green claimed AFTER producing, same turn). The step-one
counter check (§5) still runs at the ROOT decision's `root_table` for every family — the
instrument-health question is decision-shaped regardless of the read grain.

**Family 1 — already-triggered final round. CONFIRMATORY, rules-derivable (no canon needed).**
Generator: the shipped `FinalRoundAlreadyTriggered` predicate at a fresh recorded master over
the v1 rung ecology (supply measured: ~14/game), plus the nonterminal control predicate
`TriggerRoundNotFinal` (`end_game_triggered && !final_round && !is_game_over` — the amendment's
"nonterminal control earlier in the round"; g1:228's verified shape). Top-set rule:
ADJUDICATED-BEST per reply policy (see §4); pass = the subject's pick lands in EVERY declared
policy's top set; a pick in AT LEAST ONE but not all top sets is **UNRESOLVED-pending-exact**
(the policies disagree about the position — the exact-adjudicator trigger, §2's deliberately-
absent clause, and never a silent fail); a pick in NONE is the fail. Per-policy results
printed. A DEAD root is **the MEASURED F5 shape, not `all_candidates_equal`**: NO candidate
wins ANY replication under any declared policy AND the whole-field mean-margin spread is ≤ a
declared bound (draft 4 VP — the triage §3's own measured dead-field spread; the three
session-dead roots read 15/11/2 DISTINCT means, so elementwise equality would classify all
three LIVE and feed a CONFIRMATORY denominator with keyed-draw coin flips — the
healthy-subject-fails channel, review MUST-1). `all_candidates_equal` stays a stricter
PRINTED fact beside the classification, and the classifier is pinned on a constructed
adjudication reproducing the session shape (distinct means, zero wins ⇒ DEAD). A DEAD root is
recorded as honest indifference and EXCLUDED from the pass/fail denominator — it confirms
nothing and fails nothing; the family's R/F counts apply to LIVE roots only, and the pilot's
live-root supply rate decides how many strided ordinals (§4) the confirmatory block spans.
The three session roots stay what they are — resolved evidence (dead), not family draws.

**Family 2 — end-trigger firing. CONFIRMATORY (canon 15, the inverted brake).** Positive
generator: predicate `EndFirableAheadInTruth` — `!end_game_triggered`, a legal FIRING move
exists (a move whose application sets `end_game_triggered`, detected by apply-probe over the
legal list), and the mover's TRUE margin is ≥ +10 (the F9 lesson made a selection rule: the
family supplies true-arithmetic inputs or it tests the corruption; +10 ≈ one majority
bonus ≈ "one good turn of value", C-A's own calibration). Control generator:
`EndFirableBehindInTruth`, same but margin ≤ −10 (canon 15: behind → keep building; the
owner's F9 sentence "ending the game when you know you're going to lose … is a bad idea").
TRUE margin = `true_margin_now(state, rules)`: the mover's `calculate_final_scores` total
minus the best rival's, full-state truth (hidden cards included) — a SELECTION-side helper
that must not leak into the evaluator (terminal-margin awareness stays on the debt list; the
STOP fence covers it). Read: TURN grain — positive roots pass on `EndFiredByHandoff` (the
trigger is set when the mover hands off), control roots on `NotEndFiredByHandoff` (the behind
mover hands off WITHOUT firing — what it built instead is recorded, not gated). Ecology: the
v2 LOOKAHEAD ecology (§2) — the church-conditioned clock is the bot's own ecology (98.25%),
and the amendment's family-2 sentence pre-authorizes conditioning on the dominant clock; a
deck-clock corner root is recorded-not-gated if supply exists.

**Family 3 — exclusive-race opponent awareness. EXISTENCE-DEMONSTRATION (canon 12/13 +
C-A/C-B-discounted; supports a pivot/routing, never a GO).** This is the family that
discriminates D3's cross-handoff question, and F2's verified series already routed branch 3 —
the family's job is to make the gap MECHANICAL at frozen roots and leave escrow for the
post-capability re-read. Positive roots: positions where altering the opponent's NEXT-TURN
race outcome is demonstrably worthwhile (adjudication-demonstrated, §4) and the threat is
PUBLIC (the masked-starting-order confound shaped out by construction: the race arithmetic
rides board state and port-taken orders, never the hidden deal). Source: a parameterized
BUILDER on the `scenario_suite` construction grain (opponent one completed action from a
customs/church/tech race step; the mover holds a move that alters it; parameter ranges + seed
blocks declared at freeze) — plus a generated-arm PILOT probe (predicate
`ExclusiveRaceOneStep`) whose supply at the scan bound is measured and recorded; if the scan
exhausts, the recorded `Exhausted` refusal is the evidence that the builder arm is the
family's honest source. Read: TURN grain — positive roots pass on
`ContestedStepTakenByHandoff` (by hand-off the mover has secured the contested step — C-A's
participation framing: TAKE the space, which is race awareness, not denial-as-strategy) —
and the adjudication under the menu is what demonstrates "worthwhile" per root. Negative
control: a no-denial root — a denial-shaped move available but not worthwhile (canon 12/13)
— pass = the subject hands off WITHOUT having taken it. Reads at the current subject are
EXPECTED to fail the positive roots; the read's output is the demonstrated gap, per-root,
with the counter check and separator classification attached (§5).

**Family 4 — achievement proximity. CONFIRMATORY (ruling 14 — agenda-setters — plus C-A; no
denial claim, both seats can claim).** Positive generator: predicate
`AchievementClaimAvailable` (a legal claim move exists) over the v2 lookahead ecology — F8's
live-green shape. Read: TURN grain — pass on `AchievementClaimedByHandoff` (the mover's own
claimed set grew by hand-off), gated only at roots where the adjudication under the menu
confirms claiming-this-turn dominates (the F8 arithmetic); a root where adjudication reads
declining as equal-or-better is recorded, excluded from the denominator, and carried as
evidence about the family's own question. Proximity corner: a small builder block on the F8
shape itself — the leg completable WITHIN the turn (produce-to-7 then claim, two decisions) —
read on the same turn predicate, which is exactly what makes the turn grain necessary here. Recorded
rider, not a gate: at every family-4 root, the `achievement_race_proximity` candidate column
beside the shipped progress-sum — D2's named pre-bar trigger test (proximity → step
conversion at rooted positions) taken exactly where it was promised, plus T3's post-claim
filter observed on-policy.

## 2. Code shape (all in `yokohama-core`, off the fifteen-gate roster, corpus-free tests)

**`RootPredicate` evolves** (`driven.rs`): `holds(self, &GameState, &Rules) -> bool` — the
signature gains `&Rules` because layer-2 predicates probe the legal list; the shipped variant
ignores it (behavior byte-identical, pinned). New variants: `TriggerRoundNotFinal`,
`EndFirableAheadInTruth`, `EndFirableBehindInTruth`, `ExclusiveRaceOneStep`,
`AchievementClaimAvailable`. Firing detection and race arithmetic are shared helper functions
with their own unit pins (apply-probe over `get_legal_moves` via `apply_move_unchecked` on
clones — generator-side, full-state, never observation-bound: PREDICATES SELECT ON TRUTH; the
subject at the root still plays honest). **Scan cost discipline (review SHOULD-8; precondition set corrected at the plan review's
MUST-1): the apply-probing predicates SHORT-CIRCUIT on cheap state preconditions before any
probe** — and the precondition list is DERIVED as an exhaustive `match` over
`EndGameTrigger`'s five variants (a future variant becomes a compile error, never a silent
scan bias): church track one space from full · customs track one from full · order deck
near-empty · any player at 3 of 4 trading houses · any player at 7 of 8 shophouses. The
~10^5 per-game clone cost of probing every legal move at every scanned position never
materializes; the short-circuit's soundness (never skips a position where a firing move
exists) is its own pin on constructed states, INCLUDING the one-building-from-complete
shape.

**`GeneratingEcology::v2_lookahead()`** (`driven.rs`): 2P, both seats the shipped
`lookahead-v1` echo, arena driver, its own games bound and version string
(`driven-generated-v2-lookahead`). The v1 rung ecology serves family 1 — and it is AMENDED
with the phase, not untouched: it gains the §4 stride cap and a games-bound raise sized to
the block arithmetic, with its id string bumped per the type's own rule (no committed
Generated triple predates the change, verified). Scan-bound note: the bound rides the
ecology's `games` field; family 2/4 scans size it at freeze (pilot-measured supply, bound
recorded). The lookahead ecology
generates the subject's OWN reachable positions — the right population for an on-policy
checkpoint, said out loud in the freeze doc.

**`driven.rs` gains the turn-grain driver**: `drive_turn(root, subject, rules, decision_seed,
opts) -> TurnOutcome` — the mover's decisions from the root until `current_player` changes or
the game ends, through the same honest chain and root hygiene as `drive_to_terminal` (root
guard, invariants, typed refusals, a decision bound), returning the applied moves, the
HAND-OFF state, and the mover's WHOLE-TURN `SeatCounters` (review SHOULD-6: a mid-turn cap
hit or starvation inside a read must reach D7's step-one filter, not just replication 0's
root decision). Seed discipline, stated so no third discipline is invented (SHOULD-6): ONE
agent constructed at `decision_seed`, its stream advancing across the turn's decisions — the
arena's long-lived-stream discipline scoped to a turn, and at the FIRST decision it is
byte-identical to `root_table`'s fresh-agent-at-the-same-seed, which is what joins the
counter check to the read. One mover, one seat spec — the opponent never moves inside a turn
(verified: `current_player` has exactly one production writer, `advance_phase`'s end-of-turn
arm; mercenary sub-turns return without touching it), which is what makes this cheap and
reply-policy-free by construction.

**`game/driven_families.rs`** (new module, beside `driven.rs`): the layer-2 registry and the
read instrument.
- `PassRule` — computed rules, never move lists. Decision-grain:
  `AdjudicatedBest { epsilon }` (pick ∈ the candidates whose mean margin under a named reply
  policy is within epsilon of the best; wins printed beside). Turn-grain predicates on the
  hand-off state: `EndFiredByHandoff`, `NotEndFiredByHandoff`, `AchievementClaimedByHandoff`
  (the mover's claimed set grew), `ContestedStepTakenByHandoff` / `DenialNotTakenByHandoff`
  (the contested/denial step is part of the root's DECLARATION, named by the builder or
  predicate, never inferred at read time). Every rule resolves against root + hand-off at run
  time.
- `FamilyDecl` + `families()` — the four frozen declarations as code: family id, label
  (CONFIRMATORY | EXISTENCE_DEMONSTRATION), generator handle (predicate + ecology, or builder
  + parameter ranges), block ordinals for PILOT and CONFIRMATORY only (**escrow ordinals live
  in the freeze DOCUMENT, deliberately not in code — no command can burn an escrowed block by
  accident; releasing escrow is a doc-plus-code change with its own commit**), R, allowed
  failures, read grain + pass rule, reply menu, canon citations as doc strings. **The FROZEN
  masters and confirmatory ordinals are BAKED INTO `FamilyDecl` at the freeze commit** (review
  SHOULD-4 — the committed-diagnostic-master pattern the gates already use): before the
  freeze, the declarations carry no masters and every run supplies `--master` (pilot/scratch);
  after it, `family <id> --block confirmatory` runs the committed identity and a caller-typed
  master cannot silently substitute a non-frozen block. The freeze is therefore a CODE+DOC
  commit, and that is deliberate.
- **The adjudicator gains the per-playout hand-off verdict** (review MUST-2's resolution):
  adjudication accepts an optional TURN-PREDICATE hook; each playout records whether the
  MOVER's first turn (forced candidate + remainder to the first `current_player` change)
  satisfied it — evaluated online during the playout, stamped per `CandidatePlayout`. This is
  what makes turn-grain admission computable: family 4's proximity roots have NO claim
  candidate at the root (the claim is two decisions away — the design's own reason for the
  turn grain), so candidate-level pass-sets are structurally empty there, and the honest
  comparison is BEHAVIOR-vs-BEHAVIOR over playouts, not candidate-vs-candidate (§4).
- `family_read(root, subject, decl, master, R)` — the binary read: R replications, decision
  seed `mix(SALT_DRIVEN_FAMILY_READ, [master, root_word, r])` (new named salt; `root_word` =
  the root digest's first 8 bytes, so distinct roots draw distinct streams from one recorded
  master). Per replication: family 1, one `root_table` at that seed, pick ∈ the resolved top
  set; families 2–4, one `drive_turn` at that seed, the turn predicate on the hand-off state.
  Reports: per-replication verdicts (and picks/turn move lists), failure count, exact binomial
  note (rule-of-three at 0/R), and the FULL step-one counter check (D7's mandatory first
  step: ply/free caps, binds, starved, distinct values, tie count, top-band structure) — from
  ONE `root_table` at replication 0's seed PLUS, at turn grain, every replication's
  whole-turn `SeatCounters` (SHOULD-6: a mid-turn cap hit must not escape the filter). The
  hard-refuse/annotate split is §5.4's: cap/bind/starvation counters REFUSE the preference
  framing (branch-0, typed); tie/band counters annotate and send the root to the separator.
- Family 3/4 builders: parameterized constructors on the `scenario_suite` grain, seed-drawn,
  each with a construction-arithmetic doc and unit pins that the built state satisfies its own
  shape claims (`check_invariants` + the predicate-shaped assertions).

**`ReplyPolicy::RaceRusher`** (`driven.rs`) — the declared adversarial fourth menu slot, a
RULE OVER STATE. Probe scope BY FAMILY TAG (review NOTE-1): probe {`AdditionalAction`,
`AreaAction`, choice-resolution families}; never probe {`Placement`, `MovementRoutes`,
retreat}; `FivePowerBonus` and `Building` are EXCLUDED by decision (neither advances a
track). Classify each probed candidate by apply-probe ON THE AGENT'S OWN SAMPLED WORLD
(honest chain, same as every seat): (1) claims an achievement (`FulfillAchievement`) → (2)
church-race progress → (3) customs-race progress — where classes (2)/(3) are QUEUE-AWARE
(review MUST-5): the mover's track occupancy GREW in the probed clone **OR the probed clone
queues a church/customs space-placement `PendingChoice` for the mover** (a ≥2-eligible-spaces
action places nothing at apply time — the choice-encoding shape — and an occupancy-only
classifier would be blind to exactly the race-initiating moves this policy exists to model;
the ≥2-eligible case is pinned in the class-priority tests). Take the first candidate of the
highest nonempty class in the engine's legal-list order (deterministic, stated); otherwise
DELEGATE the decision to an INNER greedy@final agent (review SHOULD-7's construction: the
RaceRusher agent OWNS a mirrored greedy agent — same constructor arms as `Contestant::build`,
drift-pinned — and forwards the whole decision to it when no raceable candidate exists; the
apply-probes consume zero randomness, so the inner agent's stream is untouched by the
classification and the no-race fallback is byte-identical to a bare greedy seat at the same
seed BY CONSTRUCTION, with the drift pin asserting it). Seat materialization (SHOULD-7):
`SeatSpec`'s reply arm gains a second build path returning the boxed RaceRusher agent —
RaceRusher deliberately has NO `Contestant` (it must never be one), and the N-C
single-materialization-site argument is restated for the two-arm shape. Echo
`reply:race-rusher-v1{fallback=<greedy echo>}`; provisional-until-D1 note rides the fallback
exactly as the other `*AtFinal` bindings. Purpose: the conjunction's hostile arm — a reply
that RACES the tracks is the threat shape families 2–4 are about; it is a positional
OPPORTUNIST, not a planner (movement is never probed, so it races only from where it stands —
review NOTE-2, stated in the freeze doc's robustness caveat so the hostile arm is not
over-read). It is a harness instrument, never a contestant, never gate-eligible.

**Runner** (`driven_runner.rs` + the example, parse/render lib-tested, no new test command):
- Root specs grow `gen:<predicate>:<master-hex>:<ordinal>` wherever `<fixture|path:event>`
  parses today (table/run/adjudicate — the separator and adjudicator become runnable on
  generated roots).
- `family <id> --block pilot --master <hex>` / `family <id> --block confirmatory` — executes
  a declared block: resolve draws, adjudicate under the menu, run the binary read, print the
  family report (label first, counter check second, per-policy table, binomial line). The
  command IS the record (the W3-OPEN lesson). `--master` is MANDATORY for pilot blocks
  (recorded-before-use stays the caller's) and REFUSED for confirmatory blocks, whose masters
  are the `FamilyDecl`-committed identity (review SHOULD-4 — a typo cannot run a
  plausible-looking non-frozen block). Escrow hygiene (review NOTE-3): "burning" an ordinal
  means RUNNING table/adjudicate/read at it — supply counting does not burn; no grammar names
  an escrow block, the `gen:` escape hatch technically reaches escrow ordinals, and the
  protection is PROCEDURAL — the runs report's commands-as-record is the audit, said out
  loud.
- `opening --master <hex> --games <N> [--contestant <spec>]` — the C-D probe: N seeded fresh
  2P games (setup via the split arena setup half, the agents' own Keep resolutions — the
  setup path recorded in the probe's output per the ecology discipline, review NOTE-4), the
  subject's first TURN played move by move (a few decisions), the distribution over (first
  placement, president area, first purchase kind) printed with the lab-rush share and a
  Wilson interval. Cheap, recorded, feeds the representation queue behind C-C; no gate.

**Deliberately absent:** the exact bounded-tree family-1 adjudicator (built only if a live
family-1 root's per-policy top-sets DISAGREE — the freeze doc pre-declares that trigger and
the verdict class `UNRESOLVED-pending-exact`); any new `#[ignore]` release gate (everything
here is per-edit lib tests + runner commands); any eval/feature/weight edit; any
`yokohama-cli` coupling. The tie-share counters line for `eval_probe` (D8's named gap) is a
STRETCH task at the close, cut first, never load-bearing for this phase.

## 3. Pass-rule membership, stated exactly

- **EndFiredByHandoff**: `hand_off.end_game_triggered` is true (the root guaranteed it was
  false, so the mover's own turn set it). **NotEndFiredByHandoff**: its negation.
- **AchievementClaimedByHandoff**: the mover's claimed-achievement set at hand-off strictly
  contains its root set.
- **ContestedStepTakenByHandoff**: the CONTESTED step the root declaration names — a typed
  declaration covering all three arms (review NOTE-6): a specific CHURCH space, a specific
  CUSTOMS space, or a TECH-MAJORITY step declared as "the mover's doubled industry value
  ranks past the declared threshold" (the F6 arithmetic) — declared by the builder or
  predicate at generation time, never inferred at read time, and HELD by the mover at
  hand-off. **DenialNotTakenByHandoff**: the declared denial-shaped step is NOT held by the
  mover at hand-off.
- **AdjudicatedBest{epsilon}** (family 1, decision grain): run `adjudicate_root` (K, reply
  policy p) → top set = rows with `mean_margin >= best_mean − epsilon`; the subject's
  `root_table` pick at the replication seed is judged per §1's three-way rule (all sets /
  some sets / no set). Epsilon is declared at freeze, derived from ADJUDICATION-MARGIN noise
  at pilot roots as a conservative upper confidence bound on the sd of per-candidate means —
  the amendment's sanctioned form; the triage's eval-table levels are a different instrument
  in different units and are NOT the basis (review SHOULD-1; leniency direction stated:
  larger epsilon = easier pass). Wins and `all_candidates_equal` printed; DEAD roots handled
  per §1.
- **Turn-grain admission verdicts** (families 2–4, review MUST-2): the adjudicator's
  turn-predicate hook stamps each playout with whether the mover's first turn satisfied the
  family's pass predicate — §4's admission dominance compares the margin distributions of
  predicate-TRUE playouts against predicate-FALSE playouts, both groups drawn from the SAME
  forced-candidate sweep (a fire-later turn classifies TRUE wherever the playout policy
  fired, which is exactly the behavior-vs-behavior comparison the canon question asks).
- Firing-move detection (used by the family-2 PREDICATES, not by the read): a legal move
  whose `apply_move_unchecked` result has `end_game_triggered` true — apply-probe over
  `get_legal_moves`, generator-side.

## 4. Statistics, blocks, and masters (pre-declared; the freeze doc carries the values)

- **Unit**: one LIVE root. Per root: R = 20 replications (world-sensitivity read at K=1
  sampling — the subject is deterministic given the world, so replications measure
  verdict-stability over worlds). **Per-root F is SET AT FREEZE from pilot-measured
  world-sensitivity** (review MUST-3): admission dominance is measured in TRUTH while the
  subject decides in a sampled world, so a sampled deal can legitimately flip the in-world
  best line at a truth-dominant root — a blanket F = 0 makes family failure a Bernoulli event
  over ~120 trials with UNSTATED false-alarm arithmetic (≈45% family-level at a benign 0.5%
  per-world miss rate). The freeze doc states, per family: the declared benign-miss model,
  the F each root gets (draft posture: F = 1 of 20 for turn-predicate roots unless pilots
  read zero sensitivity, F = 1 of 20 for AdjudicatedBest), and the resulting family-level
  false-alarm bound. Pre-declaration is preserved — pilots precede the freeze, and the
  amendment sanctions conservative variance UCBs while forbidding only pilot-point MMEs.
  Every family report carries PER-ROOT miss counts: a 19/20 world-sensitivity miss and a 0/20
  systematic failure are different findings and route differently through D7.
- **Blocks and striding** (review MUST-6): consecutive scan ordinals are NOT independent
  draws — family 1's supply is ~14 hits per GAME (successive decision points of one final
  round), so unstrided blocks would quote "6 roots" that are near-duplicate positions of ≤2
  final rounds. The generator's ordinal space is therefore defined over STRIDED hits: at most
  2 hits per generated game enter the sequence (the scan skips the rest; the cap is an
  ECOLOGY field, so it is identity — adding it bumps the ecology version string per that
  type's own rule, and no committed triple predates the change, verified). Per family: pilot
  n = 4 strided draws, confirmatory m = 6, escrow 6 — ordinals disjoint and consecutive IN
  THE STRIDED SPACE at ONE recorded scan master per generator (builders: seed-blocked the
  same way, one root per seed by construction). The predicate/builder is FROZEN before the
  confirmatory draw; pilots may fix a predicate, and a fixed predicate restarts its ordinal
  space (old draws are spent, said in the doc). **The supply arithmetic is sized up front
  and the VOID disposition's extension ordinals are PRE-DECLARED at freeze** (plan review
  MUST-2): each generator's games bound must cover pilot + confirmatory + escrow +
  dead/excluded slack at its stride cap, and raising the games bound at a FIXED master and
  cap is PREFIX-STABLE — the strided ordinal sequence is identical, merely longer — so a
  supply exhaustion mid-protocol is recovered by a version-bumped extension, never a
  re-draw (the argument recorded so the bump cannot be read as one). **Escrow is SPLIT BY
  CONSUMER at freeze** (review SHOULD-3): 3 draws escrowed for the post-fix re-read, 3 for
  the final-controller read — one block cannot serve both, since the first consumer burns
  it.
- **Root admission (the "demonstrably worthwhile" clause, uniform across families 2–4)**: a
  drawn root enters the confirmatory denominator only where the menu-wide `adjudicate_root`
  (with the turn-predicate hook) demonstrates the pass BEHAVIOR dominates at that root — the
  mean margin of predicate-TRUE playouts beats the mean of predicate-FALSE playouts by ≥ δ
  with BOTH groups non-empty (review MUST-2's behavior-vs-behavior form; a root where every
  playout fires, or none does, cannot demonstrate and is recorded-excluded), and for
  family-2 controls the inverse (predicate-true playouts demonstrably lose by ≥ δ) —
  per-policy results printed, conjunction across the policies the family declares, the
  robustness-read label riding every table. **δ and the per-policy K are SIZED AT FREEZE from
  pilot-measured playout-margin noise** (review SHOULD-2: mid-game margins swing tens of VP;
  the freeze states K/δ such that the SE of the group mean-difference is < δ/2, quoting
  `CandidateRow` min/max beside every mean; draft δ = 5 VP is the starting point the pilot
  prices). A root failing its demonstration is recorded and EXCLUDED — the family reads the
  subject only where the ground demonstrably supports the canon behavior, which is what keeps
  a pass rule from encoding a strategy script. Family 1's admission is the LIVE/DEAD split
  (§1), the same idea at the decision grain.
- **The exclusion-pattern rule and the minimum denominator** (review MUST-4 — the admission
  circularity guard): the subject IS (provisionally) `lookahead@final`, so a subject defect
  that degrades post-candidate play deflates measured dominance under exactly that arm and
  would exclude exactly the roots where the defect matters — a broken subject passing by
  shrinking its own denominator. Therefore: an exclusion where the lookahead@final arm is the
  SOLE DISSENTER (dominance held under the other declared policies) is collected as
  potential subject-suppression evidence and ROUTED TO D7 AS A FINDING, never silently
  dropped; every family verdict carries its exclusion rate and per-policy dissent breakdown;
  and a confirmatory result quoting fewer than 4 admitted roots of the 6 drawn is VOID as
  confirmation — the pre-declared disposition is to extend to the next pre-declared strided
  ordinals (never re-draws) or to record-and-route as under-powered.
- **Adjudication sizing is where the phase's compute lives, and the pilot prices it.**
  Family-1 roots are final-round-cheap (playouts of ≤ 2 turns — the session's 148-candidate
  K=8 table ran same-day) and take the FULL menu. Families 2–4 sit mid/late-game, where one
  playout ≈ a half-game (~0.25s release under lookahead seats), so their generators DECLARE a
  candidate-width bound on admissible roots (draft: legal list ≤ 60 — the family's question
  does not need 139-wide placement roots, and the bound is a recorded generator constraint,
  not a post-hoc filter) and their admission adjudications run the full menu at K = 4 with
  the lookahead@final arm permitted a smaller pilot-priced K (≥ 2) if the pilot's wall
  demands it — all fixed in the freeze doc before any confirmatory draw, per-policy K's
  printed with the tables.
- **Family-level**: a family FAILS if live confirmatory roots exceed its declared failure
  allowance (draft: 0 roots for families 1/2/4, each root judged against its own per-root F).
  Family 3 declares NO pass bar — its output is the demonstrated per-root gap. Across
  families, the amendment's multiplicity rule verbatim: one family-level failure →
  disposition ledger + D7 branch-0 triage first; two or more in the same strategic class →
  the pivot conversation. **The strategic-class partition is pre-declared** (review SHOULD-9
  — the pivot trigger is undefined without it): families 2 and 4 form the
  END-TIMING/PROXIMITY class (D7 branch 2's own cluster); family 3 is the OPPONENT-REPLY
  class (branch 3); family 1 is the RULES/FINAL-ROUND class. The pivot fires on two failures
  WITHIN a class, and family 3's expected failures cannot combine with family 2/4's to fire
  it — they route to different lanes by design.
- **Masters**: one ledger row BEFORE any run records every master this phase will consume —
  family scan masters, builder seed masters, per-family read masters, adjudication masters —
  drawn as the next free serials in the committed A3E1 date family, by serial order, never
  redrawn. Same-day scratch smokes use the 9999 scratch convention and are quotable for cost
  only.
- **2P throughout** (the canon priors are 2P priors; NOTE-4's no-3–4P-adjudication note
  stands as carried).

## 5. The run protocol and D7 routing (the phase's second half)

1. Pilots (draws 1..n per family): predicate/builder shape checks, live-root supply rates,
   adjudication sanity, cost measurements. Nothing confirmatory quotes a pilot.
2. The FREEZE DOCUMENT (`docs/plans/…-phase-c-family-freeze.md`): subject echo; per-family
   generator identity (predicate/builder + ecology + master + block ordinals); reply menu
   (the four policies, RaceRusher's rule verbatim); R/F; labels; top-set rules; epsilon; the
   escrow ordinals; canon citations; the multiplicity rule; a WHAT-THIS-DOES-NOT-MEAN block
   (checkpoint evidence, provisional subject, robustness-read caveat, never GO evidence).
3. Confirmatory runs, one runner command per family block, outputs into the runs report doc.
4. **D7 routing per finding**: counter check first, with the two counter classes enumerated
   (review NOTE-5): **hard-refuse counters** — any ply/free-cap hit, budget bind, or starved
   decision refuses the preference framing outright and routes branch-0 (a truncated search's
   pick is not a preference reading); **annotate-and-route counters** — tie count and
   top-band structure ANNOTATE the read and send the root to the separator, never refuse it
   (family 3's expected tie-mass positives must still produce their gap output). Tie-mass
   roots get the mechanical separator (b=4 reproduce, b=8/b=16) BEFORE any argument rides
   them — persists ⇒ representation (the e113 ladder ask), disappears ⇒ horizon (the e63/D3
   lane ask); genuine defects (starvation/truncation/generation gap/sampler artifact) →
   branch-0 fix IN-phase with a regression pin (and if such a fix moves play behavior, the
   close handover flags this merge D2-major — the D2 cadence's final call at close).
5. Close: runs report + dispositions; the close handover carries the STOP asks (V3
   mini-cycle? D3 capability?), what's needed from the owner, and the D2 game-cadence call.

## 6. Testing (TDD, corpus-free, per-edit only)

Every predicate variant: positive + negative constructed states (the family-1 pair's existing
pins are the pattern). `true_margin_now`: hand-computable fixtures incl. the D1/D5 dummy and
a tie (margin 0 ≠ win). Ecology v2: identity fields pinned; v1's AMENDED identity (stride cap + games raise + id
bump) pinned. RaceRusher:
class-priority pins on constructed decisions (claim beats church beats customs beats
fallback), the fallback-equivalence pin (no raceable candidate ⇒ byte-identical to
greedy@final through the driver), determinism, and the small-families-only probe scope pinned
(a retreat/movement decision never probes). PassRule: each turn predicate on hand-built
root/hand-off pairs with known verdicts; AdjudicatedBest's epsilon arithmetic on a hand-built
adjudication. `drive_turn`: stops exactly at the hand-off (pinned on a rung seat's turn), the
game-over corner, root hygiene shared with `drive_to_terminal` (typed refusals, invariants),
and a determinism pin. `family_read`: seed-schedule determinism (same master → byte-identical
report twice), distinct-roots-distinct-streams, the counter-check refusal arm, binomial line
arithmetic, and per-grain smoke cells (one rung-seat turn read, one adjudicated decision read
at smoke scale).
Builders: shape self-checks + invariants. Runner: parse/render pins per new subcommand,
including the `gen:` spec round-trip, the escrow-absent property (no grammar names an escrow
block), and the confirmatory `--master` refusal. **Mutation-resistance additions (review
SHOULD-9, the repo's standing bar):** every turn predicate carries an inversion control (the
flipped predicate FAILS on a known fixture — the assert proven able to go red for its named
defect, the arena-gate precedent); the DEAD/LIVE classifier is pinned at the MEASURED session
shape (distinct means + zero wins ⇒ DEAD — rides MUST-1's fix); the admission δ
group-difference arithmetic gets its own hand-built adjudication cell (beside ε's); the
per-playout hand-off verdict is pinned against a hand-walked playout; and `family_read`'s
per-replication seed-distinctness pin matches the declared `mix` derivation word for word.
Wall discipline: builder/constructed cells prefer rung seats; the one lookahead-heavy cell (a
smoke-scale family read) states its cost. Debug wall recorded at close; counts at all four
doc sites.

## 7. Consumers and close artifacts

The freeze doc and runs report (§5); the dispositions ledger rows; guidingpath's Phase-C
decision row; CLAUDE.md's runner-command line and counts; the close handover with the owner
asks (the STOP routings, the D2 call, test-game work for the NEXT behavior-moving merge); the
PR under the standing cadence (Opus-max implementers, Fable-max reviews, Fable whole-branch
review before merge under the owner's granted authority).
