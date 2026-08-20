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

At the four in-final-round UNRESOLVED roots the instrument could solve, **the frozen
`UNRESOLVED-pending-exact` verdict converts to EXACT-FAIL in all four**, and at the frozen PASS
root it converts to **EXACT-PASS**. The instrument agrees with the frozen one wherever the frozen
one could speak, and resolves it wherever it could not.

**The correspondence that makes the frozen instrument look good, not bad:** the number of
replications the four reply policies could not agree about is very nearly the number of picks that
are genuinely outside the exact optimum — 13 → 13, 18 → 18, 19 → 20, 20 → 20. The frozen
adjudicator was detecting real signal it had no way to adjudicate. That is a corroboration of the
family-1 design, independent of this instrument's own verdicts.

**Two roots were NOT solved** (positive 16 and 17) and one of them is the arm's only FAIL root, so
the attribution half of the mandate is answered for the roots that converted and *not* for the root
it was originally aimed at. §5 is explicit about that.

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
| 16 | UNRESOLVED | — | — | — | — | — | **NOT SOLVED** — >90 min CPU, still running at write-up | see §5 |
| 17 | **FAIL** | — | — | — | — | — | **NOT SOLVED** — queued behind 16 | see §5 |

Ordinal 9 was solved separately and its report is the worked example in §7.

**Four of the five solved roots have a UNIQUE optimum**, and the field behind it is not close: the
worst candidate trails by 11–28 VP. These are not positions where every move is much of a muchness.

### 3.1 The finding that reaches outside this lane

**At ordinals 13 and 15 the exact optimum is a two-into-one-area placement**
(`PlaceTwoSameArea { area, adjacent_bonus: None }`), and at both it sits at the **bottom of the
engine's candidate listing** — row 64 of 65, and row 138 of 139:

```
ord 13    64        +3       0      *   PlaceTwoSameArea { area: AreaId(8), adjacent_bonus: None }
ord 15   138       -30       0      *   PlaceTwoSameArea { area: AreaId(8), adjacent_bonus: None }
```

The subject picked it in **zero of twenty** replications at either root.

Recorded with care about what it is: this is **one instrument, at two roots, in the final round**,
and it is not a claim about midgame play or about any feature's value. But it is an independent
arrival at the V4 milestone's own diagnosis from a completely different direction — the milestone
holds that the largest replicated deficit is placement REPRESENTATION (`AssistantsOnBoard` is a
count, not a map), and the exact solver, which knows nothing about any feature set, finds that the
optimal final-round move at two of four roots is a specific placement SHAPE the subject never
chooses. **It is offered to the V4 lane as corroborating evidence, never as a fitting input**
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
| 16 | UNRESOLVED | — | — | **NOT SOLVED** — keeps its frozen verdict |
| 17 | FAIL | — | — | **NOT SOLVED** — keeps its frozen verdict |

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
4 EXACT-FAIL, 2 NOT SOLVED. The arm's frozen family-level FAIL stands, and its attribution is no
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

**BEYOND THE BUDGET — positive ordinals 16 and 17.** Attempted at a 4,000,000,000-node budget.
Their spec-phase measurement is why they were expected to be the hard ones: the MOVER'S TURN ALONE
at both exceeds 40,000,000 nodes with ≥ 547,650 distinct hand-offs, against ordinal 15's 28,064 —
roughly twenty times the scale of the largest root that did solve, and ordinal 15 itself needed
2.35 billion nodes and 53 minutes.

**Recorded at the state of the run rather than at a predicted outcome.** Ordinal 16 was still
searching when this document was written, having consumed **more than 90 minutes of CPU without
completing** — against the 53 minutes ordinal 15 needed to finish, and already past the point where
it has expanded more nodes than the largest root that solved. Ordinal 17 was queued behind it.

That is the honest statement available now, and it is already sufficient for the only comparison
this section needs to support: **both roots are beyond the largest root this instrument solved, so
neither is retired and neither is attributed.** The exact node figures their typed `NodeBudget`
refusals will carry — the bound and the counted total — belong in a one-line follow-up to this
section when the runs land; they will sharpen the number and cannot change the disposition.

**Move ordering was measured before either was declared refused**, as ruling 5 requires:

| arm | nodes | wall |
|---|---|---|
| unordered (default) | 108,080,097 | 1 m 51.38 s |
| ordered | 103,113,333 | 1 m 54.00 s |

4.6% fewer nodes, 2.4% MORE wall — the sort costs more than the nodes it saves, so the default is
OFF and **ordering does not change the tractability picture**. A 4.6% saving does not reach a 20×
gap.

**The consequence, stated plainly: positive ordinal 17 is the positive arm's only FAIL root, and it
is one of the two the instrument cannot solve.** The attribution the mandate was originally aimed
at therefore returns **no result** — not a negative one. What the lane delivered instead is
attribution at four roots nobody expected to need it, because they converted from UNRESOLVED to
EXACT-FAIL.

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
| 15 | principle applies — NOT RUN (cost, below) | — | — | — | — |
| 17 | **named set** (the pre-registered FAIL root) | — | — | NOT RUN — root unsolved (§5) | — |

**No row in this lane was admitted by the named set.** Positive 17, the one root the named
instantiation actually named, could not be solved, so every rider figure here rests on the
principle-applied reading and on nothing else. That is worth stating plainly rather than leaving a
reader to infer it from an empty cell.

**All three EXACT-FAIL roots the rider reached read INVARIANT.** The world seeds repeat across
roots by construction — they are one `SeedSchedule::diagnostic` stream off one recorded master — but
each root's determinization is drawn against ITS OWN view, so the five worlds differ per root even
where the seeds do not.

**Ordinal 15 got no rider, and the reason is cost, not choice.** It is the fourth EXACT-FAIL root,
and W = 5 means six solves of a root that needed 2.35 billion nodes and 53 minutes for one —
upwards of five hours. W is not adjustable (ruling 3), so the honest disposition is the one the
spec pre-declared for exactly this case: the root's exact verdict stands and **its attribution is
simply not read**. It is recorded here as un-attributed rather than assumed to match its three
neighbours, and a future run at that budget is the only thing that would change it.

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

## 7. A worked example (positive ordinal 9), verbatim

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
  bounds         nodes=108080097  depth=25  memo=886
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
| exact ord 16 (budget 4e9) | >90 min CPU, unfinished at write-up |
| exact ord 17 (budget 4e9) | queued behind 16 |
| rider ord 7 / 9 / 13 | 5 m 09.75 s / 17 m 14.55 s / 30 m 10.72 s |
| ordering measurement (ord 9, both arms) | 1 m 51.38 s / 1 m 54.00 s |

Node counts are the load-independent figure; walls above overlapped other work and are recorded as
approximate on purpose rather than quietly presented as clean.

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
- **Four roots are not a family and not a population.** The exact verdicts are per-root facts. They
  sharpen family 1's recorded FAIL; they do not restate it as a new family-level claim, and the two
  refused roots mean the arm was never read whole.
- **Nothing here tunes, admits, or re-prices anything.** No contestant, policy, weight table,
  evaluator, knob or preserved rung moved; `SEMANTICS_REV` is untouched at 7; the frozen family
  instrument's parameters are unchanged; and only the already-read confirmatory ordinals 4..20 were
  touched — **the extension and escrow ordinals were never run**.
- **The §3.1 placement finding is corroboration offered to the V4 lane, never a fitting input.**

---

## 10. Routing — what this evidence feeds, and what it closes

1. **Family 1's `UNRESOLVED-pending-exact` class is retired for the in-final-round arm.** Four of
   its five roots now carry exact verdicts; the fifth (positive 16) is beyond this instrument's
   budget and keeps its frozen verdict.
   The value-architecture checkpoint §7 step 12's note that "family 1 still sits out pending the
   exact adjudicator" is discharged **for the positive arm only** — the control arm's nine
   UNRESOLVED roots are out of this instrument's scope and still sit out.
2. **The scope correction goes to the OWNER** as a step-1 handover flag (spec §11.1, ruled by the
   orchestrator and travelling as a forward correction, not a build blocker): the checkpoint's "14
   UNRESOLVED roots" figure counts nine control-arm roots the Phase-B design had already excluded,
   and §6.7's measurement confirms that exclusion. Should the owner rule otherwise, the spec §2.1
   predicate is the one line that moves.
3. **The attribution mandate is partly unmet and the gap is named.** Positive 17 — the arm's only
   FAIL root, the root the attribution question was written for — is beyond the budget. What the lane
   supplies instead is attribution at three roots that converted from UNRESOLVED to EXACT-FAIL and
   read INVARIANT under the rider: unambiguous final-round misplay, quantified in VP.
4. **The §3.1 placement observation goes to the V4 lane as corroboration only.** It is a
   final-round finding from an instrument that knows nothing about the feature set, and it arrives
   at the same place the milestone's own diagnosis did. Under the map §1.5/§1.6 fence it is
   coverage and priority evidence, **never a fitting or training input**.
5. **A trigger-round instrument is NOT proposed here.** Whether one is worth building is a separate
   feasibility and scope question this lane did not price, and §6.7 bounds only what THIS solver
   reached at ONE control root.
