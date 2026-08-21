# The exact bounded-tree adjudicator — RESULTS

**Date:** 2026-08-20 · **Status: RESULTS — the durable record of every run this lane executed.**
Branch `phase3.5c-exact-adjudicator`. Spec and its binding rulings:
`docs/plans/2026-08-20-exact-adjudicator-spec.md` (APPROVED 2026-08-20).

**The commands ARE part of the record** (the W3-OPEN lesson). **Every wall is machine-tagged Apple
M5 (Darwin 25.6.0), release, single-threaded**, and walls here are **DISCLOSED-APPROXIMATE**:
several roots were solved while other work shared the box (test suites, a second solve). **Node
counts are the reliable cost figure and are exact** — the solver is deterministic, so a node count
reproduces regardless of load. Every figure is a recorded observation under a named harness,
**never a ceiling** (ledger L9).

---

## 0. The headline

**Six of the positive arm's seven admitted roots were solved exactly.** At the four in-final-round
UNRESOLVED roots that solved, the frozen `UNRESOLVED-pending-exact` verdict converts to
**EXACT-FAIL** in all four; the frozen **FAIL** root converts to **EXACT-FAIL** too; and the frozen
**PASS** root converts to **EXACT-PASS**. The instrument agrees with the frozen one wherever the
frozen one could speak, and resolves it wherever it could not. **One root — ordinal 16 — refused.**

**The correspondence that makes the frozen instrument look good, not bad:** the number of
replications the four reply policies could not agree about is very nearly the number of picks that
are genuinely outside the exact optimum — 13 → 13, 18 → 18, 19 → 20, 20 → 20. The frozen
adjudicator was detecting real signal it had no way to adjudicate. That is a corroboration of the
family-1 design, independent of this instrument's own verdicts.

**The attribution mandate RETURNS AN EXACT VERDICT — AND NO ATTRIBUTION.** Positive 17 — the arm's
only FAIL root, and the root the attribution question was actually written for — solved after all, in
93 m 54 s over 3.28 billion nodes, and converts to **EXACT-FAIL at 20 of 20**. Its rider then
**BUDGET-REFUSED** after 8 h 12 m: one of the five determinized worlds exceeded the raised
8,000,000,000-node budget (§6). So the exact verdict stands and the misplay claim does not — ordinal
17 joins ordinal 15 as an un-attributed failure against a privileged standard. Separately, **ordinal
16** remains the one root whose exact SOLVE refused — structurally, rather than for want of patience
(§5); the two refusals are different kinds and are not one story.

**A directional detail that closes the corroboration story.** At ordinal 17 the FROZEN instrument
recorded **11 of 20** misses; the exact read finds **20 of 20** outside. The exact instrument is
HARSHER, not gentler — because the four contested policy top sets were WIDER than the true optimum,
so nine picks that satisfied some policy's set do not satisfy the position. That is the same "the
policies were seeing real signal they could not adjudicate" finding as the UNRESOLVED roots, seen
from the FAIL side.

> **READ THIS BEFORE QUOTING ANY VERDICT BELOW. EXACT-FAIL IS NOT MISPLAY.** An exact verdict is
> measured against a CLAIRVOYANT pole — a solve that saw the hidden five the subject could not. A
> pick outside that pole is misplay only where §6's rider reports **INVARIANT**, and the rider is
> what earns the word. **An un-ridered row and a CONFOUNDED row never do**, however large their gap.
> In this document that distinction is live and not hypothetical: ordinal 15 reads EXACT-FAIL at
> 20 of 20 and **got no rider**, so it is an un-attributed failure against a privileged standard and
> nothing stronger.

---

## 1. Scope — which roots this instrument answers about

Per the spec §2.1 ruling, admission is the IN-FINAL-ROUND class (`end_game_triggered &&
final_round`). Of family 1's 21 admitted confirmatory roots:

| arm | predicate | admitted roots | this instrument |
|---|---|---|---|
| positive | `FinalRoundAlreadyTriggered` | 7 (1 PASS · 5 UNRESOLVED · 1 FAIL) | **IN SCOPE** |
| control | `TriggerRoundNotFinal` | 14 (9 UNRESOLVED · 5 FAIL) | **OUT OF SCOPE** — refused `NotFinalRound` |

**The control arm's 9 UNRESOLVED and 5 FAIL roots keep their frozen verdicts, unchanged and
untouched by any exact claim.** Nothing about them is resolved and nothing about them is worsened.
The refusal is structural and pre-flight; the measurement behind it is spec §6.7 (the smallest
control root exceeded 600,000,000 nodes inside its first of 51 candidates).

> **Two sentences that would be FALSE if written:** "the exact adjudicator retired family 1's
> fourteen UNRESOLVED roots" and "it attributed the six FAILs". The true statement is §4's.

---

## 2. The reproduced verdict map (the precondition for everything else)

```
cargo run --release -p yokohama-core --example driven -- family final-round --block confirmatory
cargo run --release -p yokohama-core --example driven -- family final-round --block confirmatory --arm control
```

| arm | wall (M5) | wall (WSL, runs doc) | drawn | admitted | verdicts |
|---|---|---|---|---|---|
| positive | 56.334 s | 1 m 9 s | 16 | 7 | 1 PASS · 5 UNRESOLVED · 1 FAIL |
| control | 3 m 24.270 s | 4 m 10 s | 16 | 14 | 9 UNRESOLVED · 5 FAIL |

**Matches the runs report §4.1 exactly, per-root miss counts included** (positive ordinal 17 → 11
misses; control 6/8/10/11/13 → 17/11/17/17/8, in that order). Counter check clean. No cross-machine
divergence.

---

## 3. The exact tables

```
cargo run --release -p yokohama-core --example driven -- \
    exact gen:final-round-already-triggered:0xA3E1202608180004:<ordinal> --max-nodes 4000000000
```

| ordinal | frozen verdict | candidates | optimal set (ε=0) | top set (ε=2 VP) | best margin | worst gap | nodes | wall |
|---|---|---|---|---|---|---|---|---|
| 5 | PASS | 139 | 15 | 68 | −8 VP | — | 156,744,596 | 3 m 20.97 s |
| 7 | UNRESOLVED | 11 | **1** | 1 | **+15 VP** | 17 VP | 36,330,750 | 35.67 s |
| 9 | UNRESOLVED | 11 | **1** | 1 | −14 VP | 18 VP | 108,080,097 | 1 m 51.38 s |
| 13 | UNRESOLVED | 65 | **1** | 1 | +3 VP | 28 VP | 77,384,727 | 1 m 57.40 s |
| 15 | UNRESOLVED | 139 | **1** | 1 | −30 VP | 11 VP | 2,345,858,676 | 53 m 17.12 s |
| **16** | UNRESOLVED | — | — | — | — | — | **EXACT-REFUSED** — MemoBudget, 4,000,000 entries | 118 m 25.58 s |
| **17** | **FAIL** | 66 | **1** | 1 | **+95 VP** | 5 VP | **3,282,444,115** | 93 m 54.38 s |

Ordinal 9 was solved separately and its report is the worked example in §7.

**Five of the six solved roots have a UNIQUE optimum**, and the field behind it is not close: the
worst candidate trails by 5–28 VP. These are not positions where every move is much of a muchness.
Ordinal 17's optimum is worth **+95** against a runner-up wall at +90, and `won at best` reads TRUE.

### 3.1 The finding that reaches outside this lane

**At THREE OF THE FIVE roots with a unique optimum, that optimum is a two-into-one-area
placement** (`PlaceTwoSameArea { area, adjacent_bonus: None }`), and at each it sits LATE in the
engine's candidate listing — twice at the very bottom:

```
ord 13    64 of 65     +3       0      *   PlaceTwoSameArea { area: AreaId(8), adjacent_bonus: None }
ord 15   138 of 139   -30       0      *   PlaceTwoSameArea { area: AreaId(8), adjacent_bonus: None }
ord 17    59 of 66    +95       0      *   PlaceTwoSameArea { area: AreaId(2), adjacent_bonus: None }
```

The subject picked it in **zero of twenty** replications at every one of the three.

Recorded with care about what it is: this is **one instrument, at three roots, in the final round**,
and it is not a claim about midgame play or about any feature's value. But it is an independent
arrival at the V4 milestone's own diagnosis from a completely different direction — the milestone
holds that the largest replicated deficit is placement REPRESENTATION (`AssistantsOnBoard` is a
count, not a map), and the exact solver, which knows nothing about any feature set, finds that the
optimal final-round move at three of five uniquely-decided roots is a specific placement SHAPE the
subject never chooses — and at ordinal 17, the one root where the shipped subject was already
recorded as FAILING, it is worth +95 against a +90 wall. **It is offered to the V4 lane as corroborating evidence, never as a fitting input**
(map §1.5/§1.6 fence).

---

## 4. The verdict conversion

Rule, pre-registered (spec §7.4): ≤ 1 of 20 recorded picks outside the exact ε-set is EXACT-PASS;
≥ 2 is EXACT-FAIL, on family 1's own frozen per-root allowance F = 1 of 20. **ε = 2 VP is primary**
(ruling 4, comparability with the tolerance the family actually ran at); **ε = 0 is recorded
beside it**. Picks are the frozen confirmatory block's own per-replication records, joined by move.

| ordinal | frozen | picks in ε=0 set | picks in ε=2 set | **EXACT VERDICT** |
|---|---|---|---|---|
| 5 | PASS | 16/20 | **20/20** | **EXACT-PASS** |
| 7 | UNRESOLVED (13 unresolved) | 7/20 | 7/20 | **EXACT-FAIL** — 13 of 20 outside |
| 9 | UNRESOLVED (18 unresolved) | 2/20 | 2/20 | **EXACT-FAIL** — 18 of 20 outside |
| 13 | UNRESOLVED (19 unresolved) | 0/20 | 0/20 | **EXACT-FAIL** — 20 of 20 outside |
| 15 | UNRESOLVED (20 unresolved) | 0/20 | 0/20 | **EXACT-FAIL** — 20 of 20 outside |
| **16** | UNRESOLVED | — | — | **EXACT-REFUSED** (MemoBudget) — keeps its frozen verdict |
| **17** | **FAIL** (11 of 20 misses) | **0/20** | **0/20** | **EXACT-FAIL** — 20 of 20 outside |

**Ordinal 5 is why ruling 4 was load-bearing rather than bookkeeping.** It reads EXACT-PASS at
ε = 2 and would read EXACT-FAIL at ε = 0 (16 of 20 in the argmax). Its optimal set is 15 candidates
wide and its ε = 2 band is 68 of 139 — a genuinely flat position, where an indifference band is the
honest instrument and a bare argmax would manufacture a failure out of ties.

**What an EXACT-FAIL row does and does not say.** It says the recorded picks fall outside the set a
perfect-information solve calls optimal. It does **not** say the subject played badly — that claim
belongs to §6 and only for the rows the rider read INVARIANT (ordinals 7, 9, 13). Ordinal 15 is
EXACT-FAIL and un-attributed; ordinal 5's EXACT-PASS needs no rider because nothing is being
attributed there.

**Family 1's positive arm, restated over the roots this instrument resolved:** 1 EXACT-PASS,
**5 EXACT-FAIL**, 1 EXACT-REFUSED. The arm's frozen family-level FAIL stands, and its attribution is no
longer entangled *at the four roots that converted* — the picks there are genuinely outside a
sharply-defined optimum, not merely outside four disagreeing policy sets.

---

## 5. The refusals, kept apart

The spec (§7.4) requires the two refusal kinds be reported separately, because they are different
facts about a root.

**OUT-OF-SCOPE (`NotFinalRound`) — 14 control-arm roots.** Never attempted. Verified end-to-end:

```
$ driven exact gen:trigger-round-not-final:0xA3E1202608180004:9
gen:trigger-round-not-final:0xa3e1202608180004:9: the exact solve did not complete:
2b09f612bb95f9ef: trigger-round root — the end is triggered but the final round has not begun,
so a whole further round remains. Out of scope for this instrument (spec §2.1); its frozen
verdict stands untouched
```

(The digest `2b09f612bb95f9ef` is the same root the confirmatory control block drew at ordinal 9 —
the refusal is about the right position.)

**BUDGET-REFUSED — ordinal 16 ALONE.** Positive 17, flagged at risk alongside it, **solved** — so
16 is not one of a pair but the arm's single structural outlier.

```
gen:final-round-already-triggered:0xa3e1202608180004:16: the exact solve did not complete:
memo cap 4000000 exceeded (4000000 entries)              real 118m25.577s
```

**Note which bound bit.** The 4,000,000,000-node budget was never reached: the root exhausted the
default **4,000,000-entry transposition cap** first. That is the more informative refusal, and it is
worth saying why. A node bound would have said "more than four billion nodes" — a fact about how
long we were willing to wait. This says the root has **more than 4,000,000 distinct turn-entry
hand-off states**, against ordinal 15's **28,064** and ordinal 17's **2,781,365** — a hand-off space
more than **1.43× the largest root that solved**, and that is a lower bound: the cap records only
"more than 4,000,000". That is a fact about the POSITION'S STRUCTURE, and no amount of patience
changes it.

**The comparison that frames it:** ordinal 17 solved at 2,781,365 entries — **69% of the same cap**
16 blew through. The two roots were flagged together on their spec-phase mover-turn measurement
(both ≥547,650 distinct hand-offs); only one of them turned out to be beyond reach.

**Not re-run at a larger cap, deliberately.** Raising the cap to force a node-budget refusal instead
would cost gigabytes and hours to convert a structural fact into a weaker wait-duration one. The
counted bound is a lower bound, never a ceiling (L9). Ordinal 16 is not retired and not attributed;
a refusal is not an attribution.

**Move ordering was measured before either was declared refused**, as ruling 5 requires:

| arm | nodes | wall |
|---|---|---|
| unordered (default) | 108,080,097 | 1 m 51.38 s |
| ordered | 103,113,333 | 1 m 54.00 s |

4.6% fewer nodes, 2.4% MORE wall — the sort costs more than the nodes it saves, so the default is
OFF and **ordering does not change the tractability picture**. A 4.6% saving does not reach a 20×
gap.

**The consequence, stated plainly:** ordinal 17 — the positive arm's only FAIL root and the
attribution mandate's actual subject — **solved**, so the mandate returns a result rather than a
gap. What the instrument could not reach is one UNRESOLVED root (16), which keeps its frozen
verdict untouched.

**One binary across the whole run set, checked rather than assumed.** All six solved/refused roots
(7, 5, 13, 15, 16, 17) ran on the same pre-fix-wave build — ordinal 17's report prints `bounds`
rather than the fix wave's `realized`, which shows the running process kept its original binary
across the intervening rebuilds. The only post-fix-wave run is the RIDER (§6), and the two builds
were checked equivalent where it matters: ordinal 7 re-solved on the post-fix-wave binary returns
byte-identical `nodes = 36,330,750` and `memo = 5,520` entries with the same +15 optimum, so neither
the 128-bit key nor the interior-refusal posture moves a count.

---

## 6. The world-invariance rider

**THE APPLIED RULE, in one sentence:** the rider runs **where attribution is the question**, which
after the solves means **every root that actually read EXACT-FAIL** — the governing principle of
spec §3.4, whose named instantiation ("the in-scope FAIL root plus close-UNRESOLVED roots") encoded
a PREDICTION about where that class would fall that the instrument's own reads then updated
(orchestrator ruling, 2026-08-20, option (a); grounds recorded with it: the rider is
diagnostic-only, it **cannot change an exact verdict**, and it costs minutes).

**Master `0x83937F67C176F538`**, drawn from OS entropy on 2026-08-20. W = 5, pre-registered and not
adjustable (ruling 3).

**The master's recording sequence, stated exactly rather than as a blanket "recorded before use".**
It was drawn from OS entropy and displayed, then exercised by the module's own unit cells during
development, then written into source and into the commit message of `3bfe2e9` (12:16:42), and only
then drawn against for the EVALUATION runs in this section. So: **recorded durably before the first
evaluation draw, but after the first test-fixture use.** The standing policy's purpose — that the
draw not be chosen after seeing what it produces — is met, because no rider result existed at any
point before the value was fixed; the test cells that touched it earlier assert structure (five
distinct worlds, reproduction from the master) and produce no figure this document quotes.

*Forward correction, recorded rather than edited away:* commit `3bfe2e9`'s message says the master
was "RECORDED HERE BEFORE ANY USE". That shorthand is imprecise — it was recorded before any
EVALUATION use, which is the property that matters, but not before the test-fixture use. **This
paragraph is the authority on the sequence.**

```
cargo run --release -p yokohama-core --example driven -- \
    exact gen:final-round-already-triggered:0xA3E1202608180004:<ordinal> \
    --rider 0x83937F67C176F538 --max-nodes 4000000000
```

Every row carries its **admission clause** — why that root was in the rider set — so the strict
reading of the named instantiation ("run it only at positive 17 and close-UNRESOLVED roots") stays
fully recoverable from this same table: on that reading, all three rows below drop out and no root
is attributed at all.

| ordinal | admission clause | truth top set | worlds agreeing | verdict | wall |
|---|---|---|---|---|---|
| 7 | principle applied at discovered FAIL | 1 candidate | 5 of 5 | **INVARIANT** | 5 m 09.75 s |
| 9 | principle applied at discovered FAIL | 1 candidate | 5 of 5 | **INVARIANT** | 17 m 14.55 s |
| 13 | principle applied at discovered FAIL | 1 candidate | 5 of 5 | **INVARIANT** | 30 m 10.72 s |
| 15 | principle applies — NOT RUN (ruled, below) | — | — | — | — |
| **17** | **NAMED SET** (the pre-registered FAIL root) | — | — | **BUDGET-REFUSED** (NodeBudget, in a determinized world) — **NO attribution** | 8 h 12 m 34.68 s |

**Ordinal 17's rider is squarely inside the ORIGINAL instantiation** — it is the root the
pre-registration named, so it needs no appeal to the applied-rule reading above. It was launched at
the recorded master with **W = 5** after this document's roster run, and it **BUDGET-REFUSED** after
**8 h 12 m 34.68 s**, verbatim:

```
gen:final-round-already-triggered:0xa3e1202608180004:17: the rider did not complete: node budget
8000000000 exceeded at candidate 10 (counted 8000000001 expanded decision nodes — a lower bound on
this root's tree, never a ceiling)
```

**The refusal is in a DETERMINIZED WORLD, not in the truth solve.** The typed error does not say so;
the structure fixes it. The runner's own table solve completed — the failure is reported as the
RIDER's, not the solve's — and the rider's truth solve is that same deterministic solve, which
completed at 3,282,444,115 nodes, comfortably inside the raised 8,000,000,000 budget. **Which** of
the five worlds it was, the error does not name, and nothing here infers it.

**So ordinal 17 is EXACT-FAIL at 20 of 20 and UN-ATTRIBUTED** — the same standing as ordinal 15 by a
different route: 15 got no rider by ruling, 17 got one that could not finish. The §0 callout governs
both. This costs the lane the single attribution it most wanted, and the record says so plainly
rather than reaching for the weaker reading.

*Cost note, measured rather than assumed:* the wall includes the truth solve **twice** — once for the
printed table and once inside `world_invariance`, which re-solves it as the comparison pole. At this
root that is ~3 h 8 m of the 8 h 12 m before any determinized world began. The completed riders at
ordinals 7/9/13 carry the same double solve, so their walls sit on the same footing as this one.

*Bounds raised for that run, disclosed:* memo cap 12,000,000 and node budget 8e9, because ordinal
17's own truth solve used 2,781,365 entries — 69% of the 4,000,000 default — and a determinized
world could plausibly exceed it, spending hours to produce a spurious refusal. **Bounds are cost
containment, not pre-registered parameters, and raising one cannot change a value that completes
either way; W stayed 5, which is the parameter that is fixed.**

**Three of the four rider rows so far rest on the applied-rule reading** (ordinals 7, 9, 13); the
fourth is the named-set root itself. Under the strict reading of the named instantiation, only
ordinal 17's row survives — which the admission column keeps recoverable.

**All three EXACT-FAIL roots the rider reached read INVARIANT.** The world seeds repeat across
roots by construction — they are one `SeedSchedule::diagnostic` stream off one recorded master — but
each root's determinization is drawn against ITS OWN view, so the five worlds differ per root even
where the seeds do not.

**Ordinal 15 gets no rider — a RULING, not a deferral** (orchestrator, 2026-08-20), on four
recorded grounds: the three INVARIANT riders already establish the class; 15's 20-of-20 shape
matches ordinal 13's, which read INVARIANT; the recorded master makes the run reproducible at any
later date; and five CPU-hours is not the mandate's critical path. W is not adjustable (ruling 3),
so the honest disposition is the one the spec pre-declared for exactly this case: the root's exact
verdict stands and **its attribution is simply not read**. It is recorded as un-attributed rather
than assumed to match its neighbours, and it rides this document as a **recorded owner-leisure
option**.

**Reading, per the pre-registration:** the optimal set is the same in the true world and in every
world consistent with the mover's own view, so **hidden information does not bear on these
decisions and the picks outside the optimum are unambiguous misplay** — not a seated player's
unavoidable uncertainty.

**Non-vacuity, checked rather than assumed.** An invariance reading means nothing if every
determinization simply *is* the true position. The lib pin
`a_generated_final_round_root_leaves_genuinely_different_worlds` takes this exact root family
(ordinal 9) at this master and asserts the sampler produces positions that are **not** the truth —
it passes, so a final-round root genuinely does leave the mover uncertain. The instrument now also
records `distinct_worlds` per read and emits a third verdict, **VACUOUS**, when no alternative world
existed to disagree; the rider rows above predate that field and their non-vacuity rests on the pin.

**Why the applied rule is the principle and not an extension of it — the ruling's own reasoning,
recorded so it does not have to be re-argued.** Spec §3.4's governing sentence was always "run the
rider where attribution is the question." The named instantiation (positive 17 plus close-UNRESOLVED
roots) was written when positive 17 was the only root *expected* to read FAIL — it encoded a
prediction about where that class would fall. The instrument's own reads updated the prediction:
ordinals 7, 9 and 13 are not "close" (their ε = 2 sets are a single candidate) and all converted to
EXACT-FAIL, making them members of the attribution-question class discovered in-run. Running the
rider there is the principle faithfully applied. Nothing about it can move a gate: **the rider
cannot change an exact verdict — it only classifies one** — and the strict reading remains
recoverable from the admission-clause column above.

---

## 7. A worked example (positive ordinal 9) — ABRIDGED

**Abridged, not verbatim:** the provenance block's ecology lines are trimmed and the stats line
carries its post-review label (`realized`, not `bounds` — those are what the solve REACHED, not
what it ran under). Everything else is the report's own output.

```
exact read: a PERFECT-INFORMATION solve of the remaining game over the TRUE state, hidden cards
included, with BOTH seats played optimally — so a value here is what the position is worth under
best play against best defence. It is therefore an UPPER BOUND on achievable quality: a subject
pick outside the optimal set is NOT by itself misplay, because the subject could not see what
this solve can. PRIVILEGED, analysis-side only — never an input to any evaluator, weight fit, or
contestant.

=== driven exact ===
  root           gen:final-round-already-triggered:0xa3e1202608180004:9   generated@c74da36d9a21ebb5  event=825
  semantics      rev=7  rules=d85445c479cb1ead
  mover          p0
  realized       nodes=108080097  depth=25  memo=886
  generated      predicate=final-round-already-triggered  master=0xa3e1202608180004  ordinal=9  game=4
  ecology        id=driven-generated-v1.1  players=2  games=16  per_game_cap=2

  candidates     11   optimal set (eps=0) 1 of 11   top set (eps=2 VP) 1 of 11

    #    margin     gap    opt         nodes   candidate
    1       -31      17             11830684   PlaceThreeAreas { placements: [] }
    2       -31      17             12177945   PlaceThreeAreas { placements: [AreaId(0)] }
    3       -31      17             11830828   PlaceThreeAreas { placements: [AreaId(1)] }
    4       -31      17              7091284   PlaceThreeAreas { placements: [AreaId(2)] }
    5       -14       0      *       1119922   PlaceThreeAreas { placements: [AreaId(3)] }
    6       -31      17             13603709   PlaceThreeAreas { placements: [AreaId(4)] }
    7       -32      18             15052758   PlaceThreeAreas { placements: [AreaId(5)] }
    8       -31      17              3165702   PlaceThreeAreas { placements: [AreaId(6)] }
    9       -31      17              6524886   PlaceThreeAreas { placements: [AreaId(7)] }
   10       -31      17             13718515   PlaceThreeAreas { placements: [AreaId(8)] }
   11       -31      17             11963864   PlaceThreeAreas { placements: [AreaId(9)] }

  won at best    false   (margin 0 is NOT a win: the official tie-break decides it)
  memo           886 entries / 657 hits   (TURN-ENTRY states only)
```

**Forward correction against the spec's own prototype figure.** Spec §6.5 records this root's best
value as −15 VP; the shipped instrument reports **−14**, and the shipped value is authoritative.
The prototype scored in a doubled scale (`2 × margin + tie`, so −29 here) and displayed the margin
by flooring that halved — `(−29).div_euclid(2) = −15` — a **uniform 1-VP offset on losing
positions** and nothing more. The shipped path takes its leaf values straight off `final_result`'s
standings with `won` carried separately, which is what the terminal-provenance and margin-0 pins
hold in place. The signature that it is a display shift rather than a different search: the cost
counters are IDENTICAL across the two (108,080,097 nodes), and a uniform offset cannot change any
comparison, so the same tree was explored either way.

**One candidate is worth 17 VP more than any other.** Placing the single assistant at `AreaId(3)`
holds the margin to −14; every other placement — including placing nothing — concedes −31 or worse.
The subject found it in 2 of 20 replications, and the rider says the hidden set does not excuse the
other 18.

Note also what the `nodes` column shows: the optimal line was the CHEAPEST to prove (1.1 M nodes
against 7–15 M for the rest), because alpha-beta refutes the losing candidates quickly once the
best is known. That is a property of the search, recorded, not a result.

---

## 8. Run inventory and walls

| run | wall (M5, disclosed-approximate) |
|---|---|
| confirmatory positive / control re-runs | 56.33 s / 3 m 24.27 s |
| exact ord 7 / 9 / 13 / 5 / 15 | 35.67 s / 1 m 51.38 s / 1 m 57.40 s / 3 m 20.97 s / 53 m 17.12 s |
| exact ord 16 (REFUSED, MemoBudget) | 118 m 25.58 s |
| exact ord 17 (SOLVED) | 93 m 54.38 s |
| rider ord 7 / 9 / 13 | 5 m 09.75 s / 17 m 14.55 s / 30 m 10.72 s |
| rider ord 17 (BUDGET-REFUSED, NodeBudget) | **8 h 12 m 34.68 s** (estimate had been ~9.4 h; §6) |
| fifteen-gate release roster (M5, quiet box) | **379 s — see §8.1** |
| ordering measurement (ord 9, both arms) | 1 m 51.38 s / 1 m 54.00 s |

Node counts are the load-independent figure; walls above overlapped other work and are recorded as
approximate on purpose rather than quietly presented as clean.

**The transposition key is 128-bit, and the reason is worth a line.** The table is keyed by
FNV-1a **128** over the state's canonical JSON — deliberately NOT the engine's `state_digest`,
which is FNV-1a **64**. As a record identity 64 bits is ample (it is checked against one known
state). As a TRANSPOSITION key the exposure is different: every stored position is compared against
every other, so the risk is birthday-shaped in the table's size, and a collision would hand one
position another's value SILENTLY, with a wrong number propagating to the root looking exact. At
the sizes seen here that probability was small — ~10⁻¹¹ at ordinal 15's 28,064 entries, ~10⁻⁸ at
the ≥547,650 the unsolved roots reach — but an instrument whose whole claim is the word EXACT
should not carry a probabilistic caveat it can retire for nothing. At 128 bits the same arithmetic
is ~10⁻²⁸. **Widening changed no result**: ordinal 7 re-solved on the widened key returns
`nodes = 36,330,750` with `memo = 5,520` entries and the same +15 optimum — byte-identical to the
64-bit run, which incidentally also shows no collision was occurring there. (Widening
`state_digest` itself was never an option: it is written into saved records, which is
`SEMANTICS_REV` territory and out of this lane.)

### 8.1 The fifteen-gate release roster — an M5 BASELINE

**15 / 15 GREEN in 379 s**, measured as ONE uninterrupted sequence on a QUIET box, release binaries
pre-built outside the timing, single-threaded exactly where the command says so. Machine: Apple M5
(Darwin 25.6.0), toolchain 1.97.1.

**Recorded as an M5 BASELINE BESIDE the WSL figures, never against them** (the per-machine rule):
the WSL box's recent whole-roster readings are 508.9 s (PR #20 head), 487.8 s (PR #23) and 478.5 s
(PR #24); the freeze-era M5 reading was 382.86 s. This 379 s sits with that M5 figure and is not
comparable to the WSL column. Target ≤ 420 s (3.4 spec §8), met.

**Gate 10 ran EXACTLY 1 test**, which is the load-bearing count `CLAUDE.md` flags: the core lib has
exactly one `#[ignore]` (the recovery falsifier), and this lane added none — every cell it adds is
per-edit. A second ignored lib test would have ridden in unannounced and this is what catches that.

**The roster is corpus-independent.** The fourteen corpus-gated pins are per-edit DEBUG suites
(`bga_import`, `feed_preprocess`); no release gate reads a capture. So the worktree's
`YOKOHAMA_BGA_CORPUS_OPTIONAL=1` — which does make `bga_import`'s 9 tests pass vacuously in the
per-edit tier — does not touch this figure.

**Binary-drift check, because the solver changed mid-lane.** The roots above were solved before the
lazy-expansion fix (commit `9b56488`), which was argued to leave node counts untouched — the counter
counts `solve()` entries, and applying a child without solving it was never counted. That argument
was then CHECKED rather than left standing: ordinal 7 re-solved on the post-fix binary returns
**nodes = 36,330,750**, byte-identical to its recorded figure, with the same 1-of-11 optimal set.
Every node count in this document reproduces at HEAD.

---

## 9. What these runs do NOT mean

- **A clairvoyant optimum is an upper bound on achievable quality, not a standard of play.** Every
  value here is a perfect-information solve with both seats optimal. A pick outside it is misplay
  only where the rider says the hidden set could not have mattered — which is why §6 exists and why
  a table quoted without its first line is quoted wrongly.
- **"Best against best defence" is not "best against this opponent."** The exact instrument adds a
  pole; it does not replace the robustness read, and a candidate that loses to perfect defence may
  still be the best practical choice.
- **This is checkpoint evidence about a PROVISIONAL subject** —
  `lookahead-v1@v2{b=4,eval_impl=3,k=1,m=8}` at `SEMANTICS_REV` 7 under `WeightTable::V2` — never
  GO evidence, never the bar, and non-comparable across any future controller or impl bump.
- **Six roots are not a family and not a population.** The exact verdicts are per-root facts. They
  sharpen family 1's recorded FAIL; they do not restate it as a new family-level claim, and the one
  refused root means the arm was never read whole.
- **The three-of-five placement pattern (§3.1) is an observation, not a rate.** Three roots sharing
  an optimum SHAPE in the final round is a lead worth handing to the V4 lane; it is not a measured
  frequency, and nothing here estimates how often that shape is optimal in general.
- **Nothing here tunes, admits, or re-prices anything.** No contestant, policy, weight table,
  evaluator, knob or preserved rung moved; `SEMANTICS_REV` is untouched at 7; the frozen family
  instrument's parameters are unchanged; and only the already-read confirmatory ordinals 4..20 were
  touched — **the extension and escrow ordinals were never run**.
- **The §3.1 placement finding is corroboration offered to the V4 lane, never a fitting input.**

---

## 9a. Review dispositions — what was declined, and on what grounds

The diff review (2026-08-20, MERGE-WITH-NOTES, zero MUSTs) raised six SHOULDs and nine notes. All
six SHOULDs were taken. Two items were **declined with grounds**, recorded here rather than left as
silence:

- **N8 — a belt-and-braces `distinct_worlds` re-run at ordinals 7 and 13.** DECLINED. The
  ordinal-9 pin plus §6's disclosure already establish that a generated final-round root leaves
  genuinely different worlds, and the attribution conclusion is **robust under either branch**: if
  the world set were somehow vacuous, that would itself mean the mover's view pinned the state, and
  a pick outside a fully-determined optimum licenses the misplay reading anyway. ~35 minutes of
  busy-box compute buys no decision change.
- **N7 — the candidate-order convention.** Recorded as **deliberate consistency with
  `adjudicate_root`**, not an oversight: both take their candidates from `get_legal_moves` in its
  own order, which is precisely what lets the two tables join row-for-row (§4's conversion depends
  on it).

Also on the record, confirmed by the orchestrator: **566e33c's citation of the rider ruling is
accurate** — the option-(a) ruling with its three conditions was issued as quoted, and the commit's
full text was reviewed directly.

---

## 10. Routing — what this evidence feeds, and what it closes

1. **Family 1's `UNRESOLVED-pending-exact` class is retired for the in-final-round arm.** Four of
   four of its five roots now carry exact verdicts; the fifth (positive 16) is beyond this
   instrument's budget and keeps its frozen verdict.
   The value-architecture checkpoint §7 step 12's note that "family 1 still sits out pending the
   exact adjudicator" is discharged **for the positive arm only** — the control arm's nine
   UNRESOLVED roots are out of this instrument's scope and still sit out.
2. **The scope correction goes to the OWNER** as a step-1 handover flag (spec §11.1, ruled by the
   orchestrator and travelling as a forward correction, not a build blocker): the checkpoint's "14
   UNRESOLVED roots" figure counts nine control-arm roots the Phase-B design had already excluded,
   and §6.7's measurement confirms that exclusion. Should the owner rule otherwise, the spec §2.1
   predicate is the one line that moves.
3. **The attribution mandate RETURNS AN EXACT VERDICT AND NO ATTRIBUTION.** Positive 17 — the
   arm's only FAIL root and the root the attribution question was written for — solved and converts
   to EXACT-FAIL at 20 of 20, but its rider BUDGET-REFUSED at the recorded master (§6), so the
   misplay claim the rider exists to earn is NOT earned here. Beside it, three roots that converted
   from UNRESOLVED to EXACT-FAIL read INVARIANT under the rider: unambiguous final-round misplay,
   quantified in VP. What remains unread is ordinal 17's attribution (refused), ordinal 15's
   (ruled), and ordinal 16 entirely (refused).
4. **The §3.1 placement observation goes to the V4 lane as corroboration only.** It is a
   final-round finding from an instrument that knows nothing about the feature set, and it arrives
   at the same place the milestone's own diagnosis did. Under the map §1.5/§1.6 fence it is
   coverage and priority evidence, **never a fitting or training input**.
5. **A trigger-round instrument is NOT proposed here.** Whether one is worth building is a separate
   feasibility and scope question this lane did not price, and §6.7 bounds only what THIS solver
   reached at ONE control root.
