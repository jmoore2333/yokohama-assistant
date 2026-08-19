# Phase C — the family runs: pilots, confirmatory blocks, and the D7 routings

**Written 2026-08-18 at the Phase-C close (branch `phase3.5c-families`). This is the durable
record of every run the phase executed — the commands are part of the record (the W3-OPEN
lesson), every wall is WSL-tagged (Debian 13 WSL2 · Intel Core Ultra 7 265K), and every
verdict below is the FROZEN instrument's own printed output (freeze:
`docs/plans/2026-08-18-phase-c-family-freeze.md`, code `ac96b5c..5d547d7` + `4002340`).
Raw logs: the session scratchpad `t7-pilots/` and `t7-confirmatory/`; each block's full
per-root tables are in its log and the load-bearing lines are quoted here.**

## 0. Command inventory (masters per ledger rows `ee8c8c7` + `490f367`, recorded before use)

Pilots (pre-freeze instrument, `--block pilot`, ten commands, ~3m13s total): the 9999-scratch
command smoke, then per arm `family <id> --block pilot --master <the arm's recorded master>`
for final-round (…0004 both arms), end-trigger (…0005/…0006), achievement (…0007/…000A),
exclusive-race (…0008/…0009), plus `opening --master …000F --games 200`. Probes (post-freeze,
pilot-layer): the family-3 generated-arm supply probe at the freshly-recorded …0010
(`table gen:exclusive-race-one-step:…0010:47`), and the two ε-derivation adjudications at the
recorded f1 masters (`adjudicate gen:…:…0004:<ordinal> --master …0004 --replications 8`).
Confirmatory (the frozen blocks, NO `--master` — refused by design): `family <id> --block
confirmatory [--arm control]` × the four families; the end-trigger pair re-ran once after the
recorded ecology extension (§3). Masters …000B–…000E remain RESERVED-UNUSED (the
single-master/two-salt design; recorded in the freeze §2), never redrawn.

## 1. The pilot readings (sized the freeze; never quoted as confirmatory)

Full logs in `t7-pilots/`. What the freeze took from them: family 2's admission
discriminates cleanly (1-of-4 per arm; every policy splitting 4/4); families 3/4 excluded
0-of-4 under the pre-freeze conjunction with the mechanisms visible per policy (the
family-weighted rung's 1-of-8 accidental splits; greedy's structural never-claims; the
race-rusher's near-universal claims) — answered by the frozen splitting rules; a forced
1-candidate root read a vacuous PASS — answered by the forced exclusion; empty-denominator
blocks printed PASS — answered by NOTHING-ADMITTED; family 1 read UNRESOLVED at every
trigger-round root at ε=1/K=4. Pre-reading on record (ledger `490f367`): the one admitted
f2-control pilot root showed the subject firing 20/20 where not-firing dominates under all
four policies.

## 2. The owed probes (recorded before family-1 verdicts are quoted)

**The ε derivation (freeze-review SHOULD-1's owed arithmetic).** Reproduced adjudications at
the recorded f1 masters, K=8: at the f1-POSITIVE live root (11 candidates) the per-candidate
mean-margin sd is **0.963** (95% UCB **1.535**), and the within-candidate SE (playout noise
per candidate, sd(margins)/√8) reads ~**0.8–0.9** — ε=2 covers both, so the frozen ε is at
or above the sanctioned noise scale at the positive arm. At the f1-CONTROL root (41
candidates) the cross-candidate dispersion is **3.813** — that is FIELD STRUCTURE (real
between-candidate differences), not noise, and it is exactly why the control arm's top sets
are narrow and policy-contested: the ε=2 verdicts there are genuine disagreement territory,
which is the exact-adjudicator's named domain (§4.1).

**The family-3 generated-arm supply probe (freeze §2's owed reading).** At …0010 the scan
found **7 `ExclusiveRaceOneStep` roots in 24 games** (typed `Exhausted` at ordinal 47) — the
generated arm is thin, and the builder arm stands as the family's declared primary source.

## 3. The end-trigger supply exhaustion and the recorded recovery

Both f2 confirmatory blocks REFUSED at the frozen v2 bound — typed, honest, quoted verbatim:
positive `draw 10: no root 10 for end-firable-ahead-in-truth: 10 found in 24 generated
games`; control `draw 7: … 7 found in 24 generated games`. The freeze §4's pre-declared
prefix-stable recovery applied: games 24 → 120, id → `driven-generated-v2.1-lookahead`
(commit `4002340`, two production lines, mutation-proven pins; freeze amendment `0f876f0`).
Every completed reading stands at its recorded ecology id; the re-run draws the identical
prefix plus the previously-unreachable ordinals.

## 4. The confirmatory verdicts and their D7 routings

**Step-one counter check, all families: CLEAN.** 0 counter-refusals anywhere (no cap hit, no
bind, no starvation at any read root) — no branch-0 instrument-health routing fired.

### 4.1 final-round (CONFIRMATORY, rules/final-round class) — FAIL, attribution pending-exact

Positive (16 drawn → 7 admitted after DEAD/FORCED exclusions): **1 PASS · 5
UNRESOLVED-pending-exact · 1 FAIL** (ordinal 17, 11 misses of 20). Control (16 drawn → 14
admitted): **9 UNRESOLVED · 5 FAIL** (17, 11, 17, 17, 8 misses). Family-level: FAIL against
allowance 0 in both arms.

**The routing.** The dominant phenomenon is that **the exact-adjudicator trigger fired at
scale at the frozen parameters**: 14 of 21 admitted roots read UNRESOLVED — the declared
policies genuinely disagree about these positions (the control field's 3.8-VP candidate
dispersion, §2, is the mechanism). The six FAILs are real recorded failures whose
ATTRIBUTION is entangled with that same contest: a pick missing all four contested top sets
8–17 times of 20 is either poor final-round play or an under-instrumented ground truth, and
the design pre-declared exactly this fork — **the exact bounded-tree adjudicator is the
named next instrument**, and family 1's verdict is recorded as **FAIL-pending-exact
attribution**. Not a pivot (one family, alone in its class). The instrument build is
harness work (the roots' remaining trees are final-round-small), the first work item of the
next phase.

### 4.2 end-trigger (CONFIRMATORY, end-timing/proximity class) — positive PASS; control VOID with a 20/20 recorded FAIL

After the recorded ecology extensions (§3; the second raise sized off the longer
measurement when the control exhausted again at 26 of 28 hits in 120 games — freeze §4
amendment 2, commit `0aa7385`), both arms ran to verdicts:

**Positive (24 drawn at v2.1 → 6 admitted): PASS — 0 misses at every admitted root.** Where
firing-the-end-trigger-while-ahead-in-truth demonstrably dominates under the frozen menu,
the subject fires, 20/20, at all six roots. Canon 15's accelerate-when-ahead half is
CONFIRMED at the frozen instrument (the reading stands at ecology id v2.1; prefix-stability
makes a v2.2 re-draw identical).

**Control (24 drawn at v2.2 → 1 admitted): VOID as confirmation (< 4)** — behind-by-10-plus
roots where NOT-firing demonstrably dominates are genuinely rare in the subject's own
self-play ecology (1 admitted of 24 drawn; ~28 predicate hits took ~130 games) — **and the
one admitted read is a 20/20 systematic FAIL: the subject fired the trigger in every world
at the root where not-firing dominates under every splitting policy.** The pilot's
pre-reading (ledger `490f367`) reproduced exactly at confirmatory grain. The no-extension
arithmetic (~4% admission over 8 extension ordinals ≈ 0.3 expected) applies as at family 4:
record-and-route as under-powered. The mechanism matches F9's verified account — the
subject's self-read carries no terminal-margin truth (the debt-list target
"terminal-margin awareness" never reached L1) — so this read is the F9 class made
mechanical on an admitted root, recorded beside its VOID label, never quoted as a
confirmation.

**Family-level verdict: the positive arm PASSES; the control arm is VOID** — family 2 is
NOT a family-level failure under the frozen rules (VOID is not FAIL), and the multiplicity
rule reads it so.

### 4.3 exclusive-race (EXISTENCE-DEMONSTRATION, opponent-reply class) — the gap is EMPTY at
admissible roots, and that is the finding

Positive (24 builder seeds → 2 admitted): **GAP RECORDED, 0 misses at both** — the subject
SECURED the contested step 20/20 wherever the menu demonstrated it worthwhile. Control (24 →
8 admitted): **0 misses at all eight** — the subject never takes the wasteful denial.

**The instrument insight, said plainly: admission-by-playout-dominance is structurally at
odds with the opponent-awareness question.** A root where the race's worth is visible to
playout policies is also visible to the subject's own search (the majority-now features);
a root where the worth is PROSPECTIVE-ONLY — F2's actual gap, the un-priced impending
opponent action — is excluded as not-demonstrable by the same filter. The family therefore
cannot mechanically isolate the D3-lane gap with this instrument, and its clean 0-miss
sheets are evidence about feature-visible race arithmetic, not about opponent awareness.
The F2 owner-game series (351 → 28/36 → 194 → −9) remains the branch-3 routing's evidence;
this structural limit is carried INTO the D3-lane ask as fixture-design input (a
cross-handoff diagnostic, not playout dominance, is the instrument shape that could see the
prospective gap).

### 4.4 achievement (CONFIRMATORY, end-timing/proximity class) — VOID both arms; the
recorded reads split sharply

Positive (24 drawn → 2 admitted): **VOID as confirmation** (< 4), and the two admitted reads
are both **PASS 0/20** — where claiming-now demonstrably dominates, the subject claims
(under-powered, positive-leaning, consistent with F8's live green). Corner (24 constructed
seeds → 2 admitted): **VOID**, and the two admitted reads are both **FAIL — 9 and 17 misses
of 20**: at constructed proximity roots where completing the leg within the turn dominates
under the splitting policies, the subject reliably does NOT complete it (the T5 fixture
facts, now on admitted roots: its search values `EndAdditionalPhase` far above the
completion line).

**The no-extension decision, with its arithmetic.** The pre-declared 8-ordinal extension at
~8% measured admission adds ~0.7 expected admitted roots — it cannot plausibly reach the
4-root floor, so the disposition is **record-and-route as under-powered** rather than
running an extension to a foregone conclusion (also noted: no runner grammar reaches the
extension ordinals — wiring one would be code churn in service of that same conclusion).

**The routing.** VOID means no confirmation is quoted — but the recorded corner reads are
evidence, and they join the counters report's `achievement_race_proximity` MID/LATE edge
and the f2-control 20/20
fired-while-behind read (see 4.2, VOID-but-recorded) as the END-TIMING/PROXIMITY cluster's mechanical exhibits — D7
branch 2's shape, whose disposition is **a narrow pre-bar V3 mini-cycle
(`achievement_race_proximity` the named first candidate)** — a Phase-C STOP, put to the
owner in the close handover.

### 4.5 The opening probe (C-D's baseline)

**Lab rush 195/200 = 0.9750, Wilson95 [0.9428, 0.9893]** at 200 fresh seeded boards, the
shipped subject, first-turn distribution over 39 distinct (placements, president, purchase)
shapes — president `LaboratoryA` + purchase `Laboratory` in all but 5 games. C-D ruled the
fixed opening not acceptable; this is the frozen baseline number the representation queue
(behind C-C) will be measured against. No gate.

## 5. The multiplicity read (the amendment's rule, applied)

Family-level failures this wave: **final-round (rules/final-round class)** — and no
other: family 2's control arm is VOID, not failed, so the end-timing/proximity class holds
zero failures. That is EXACTLY ONE family-level failure, alone in
its class — the pivot trigger (two failures in one strategic class) does NOT fire, and each
finding routes individually. Family 4 is VOID
(explicitly not a failure); family 3 declares no bar. Each failure routes through D7
individually as §4 records.

## 6. Walls (WSL, release)

Pilots: ten commands 3m13s total. Confirmatory first pass: ~36m (dominant: f3-ctrl 21m0s —
K=8 × 8 admitted × the no-denial builder's playouts; f1-ctrl 4m10s; f4-corner 4m0s; f3-pos
3m5s; f4-pos 2m43s; f1-pos 1m9s). The f2 re-runs: positive 1m51s (v2.1); control 61s
(second typed exhaustion at v2.1) then 2m3s to verdict at v2.2. The ε probes: 77ms and 9.6s. The supply
probe: 10.9s.

## 7. What these runs do not mean

Checkpoint evidence about a PROVISIONAL subject at `SEMANTICS_REV` 7 under
`WeightTable::V2`, never GO evidence and never the bar; a family FAIL is a D7 routing event
with its named next step, not a standalone verdict; the existence arm's clean sheets are
statements about feature-visible arithmetic under a robustness-read instrument; every wall
is per-machine; and nothing here tunes, admits, or re-prices anything — the STOP fence held
throughout (no V3 work, no D3 build, no weight/feature/knob change, no preserved-rung edit,
`SEMANTICS_REV` untouched).
