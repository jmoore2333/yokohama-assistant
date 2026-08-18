# Phase B — D7 triage over the owner-games package

**Written 2026-08-17 (Phase B analysis session), against the feedback package
`docs/plans/2026-08-17-phase-b-owner-games-handover.md` and the scribe log
`docs/temp/owner-games/2026-08-17-notes.md`. The decision rule is the ratified amendment's D7
(owner-ruled YES): branch 0 defect triage first and cheap, with the counter check as step one of
every report; branches 1–3 route coherent / stage-proximity-end-timing / opponent-reply-required
findings; branch 4 is CLI work; the representation-vs-horizon call is made MECHANICALLY by the
separator, never by narrative.**

**Method.** Every number below was RE-DERIVED from the saves this session by a six-agent
evidence workflow (Opus/Sonnet lenses; `why <n>`/`log`/`score` against scratch copies, raw-JSON
event reads, `state_at` probes through the crate's own public API, and — for the un-truncated
rankings — an out-of-repo instrumented source mirror cross-checked byte-for-byte against the
shipped build on every shared figure; the repo tree was never written). Where the package or the
notes disagree with the measurement, §5 records both verbatim — the package documents are left
byte-untouched per the frozen-artifacts convention. This triage RECORDS routings and evidence;
the Phase-C STOP (any V3 mini-cycle or the D3 capability path) remains the owner's decision
point, and nothing here is GO evidence.

**Instrument note carried from the verification itself:** ~45 further `why` reconstructions ran
across both saves during this pass (on top of the session's ~25), zero mismatch refusals — the
W5 honesty gate's field record continues clean at this HEAD (`087362f`; both records answer
without version drift, verified explicitly).

---

## 1. Branch 0 — the defect filing: F1, Placement-root value separation

**The verified triple** (identical phase, identical contestant `lookahead-v1@v2{b=4,eval_impl=3,
k=1,m=8,weights=-7339724794018772210}`, identical 139-candidate root family, and a fully healthy
uncapped search at every root — `budget_binds 0 · budget_starved 0 · placement_cap_binds 0 ·
ply_cap_hits 0 · free_cap_hits 0` at all three, so this is NOT starvation, NOT a
candidate-generation gap, NOT truncation, NOT a sampler artifact):

| Root | Search | Top structure | Whole-field separation |
|---|---|---|---|
| g1 e113 | 2,468 scored / 139 candidates | **36-way tie at 0**, keyed draw | **6 levels, spread 9** on \|hand-off\| 228 (3.9%): 0×36 · −1×64 · −4×8 · −5×28 · −8×1 · −9×2 |
| g3 e63 | 4,452 / 139 | **26-way tie at 0**, keyed draw | 9 levels, spread 34 on 152 (22%): 0×26 · −2×8 · −4×2 · −15×11 · −17×2 · −28×47 · −30×34 · −32×8 · −34×1 |
| g3 e93 | 6,516 / 139 | **UNIQUE argmax, +116** over the next level — no tie note at all | 10 levels, spread 148 on 351 (42%): 0×1 · −116×3 · −122×1 · −129×1 · −133×1 · −140×25 · −142×63 · −144×35 · −146×8 · −148×1 |

**Classification: the named branch-0 defect class** — tie-mass / value-separation degeneracy,
the amendment D11 debt-list admission target ("Placement-root value separation") reproduced
live, owner-flagged, in both games. The e63-vs-e93 contrast pair (same game, same 139-wide
family, both roots walking to the same Church A02) confirms the handover's conditional:
placements separate when they enable a feature-visible scoring line inside the searched turn
(e93 sets up a faith-5 Church payment), and collapse into tie-walls when they don't.

**Two mechanism facts the verification added, both load-bearing for the fix:**

1. **The hand-off vector cannot see where the assistants went.** At g1 e113 the searched
   principal line walks the President to Fishing Ground (A09) and produces seafood — yet the
   placement that actually includes A09 ("Laboratory A + Fishing Ground") sits at **−1, BELOW
   the 36-way winning tie**. The placement enabling the line the bot valued is not in the
   winning set. `assistants_on_board` is a count, not a map; 36 different placements collapse
   to one number because the evaluator literally cannot represent the difference.
2. **The top is a BAND, not just a tie: 100 of 139 candidates (72%) sit inside one point** at
   e113 (36 at 0 + 64 at −1). A separator sized to "break the top tie" would still leave a
   64-wide near-tie one point down. The mechanical separator and any future fixture must assert
   on the whole top band, not `tied_at_top` alone.

**Pervasiveness (context, not part of the filing):** tie notes fire at twelve-plus distinct bot
placement decisions in game 3 alone (58-, 43-, 36-, 34-, 28-, 26-, 18-, 9-, 7-, 4-, 2-way), and
the pick at each is the position's keyed draw. The class is the norm at placement roots that
don't feed a scoring line, not an isolated corner.

**Disposition (branch 0 verbatim):** fix as a bug with its own regression pin, re-read affected
evidence, no V3 cycle, no architecture question. The fix path is the debt-list admission target
taken through its evidence ladder — L1 candidate columns that separate placement roots by
assistant LOCATION value — with the mechanical separator's reading (§3) taken first: persists at
larger beam ⇒ representation (feature admission, the predicted outcome); disappears ⇒ horizon.
Any `Feature::ALL` move is freeze-class blast radius and lands as its own TDD wave with owner
visibility per standing process. The three roots above are the regression fixtures-in-waiting
(e93 is the positive control: a separator that fixes e63 must not break e93's +116).

---

## 2. Routings

**F2 — the one-sided horizon → BRANCH 3 (opponent-reply-required), the recommended routing.**
The verified series (game 3): the bot's self-read plateaued at **351** (e93–e99, stable across
seven consecutive tables) while the owner sat on two unplayed orders; the owner fulfilled both
at e106–107 (+11 VP) and the next bot tables read **28, then 36** (e109; e110–115); it rebuilt
to **194** around its church/mercenary run (first at e137 — the searched line already contained
the church step — and again e142–149). Every threat component was PUBLIC and verified in the
log: the Language School purchase (e103), both port order-takes (e10: O12+O05; e119: O07+O29,
the cards visible in the players panel), the goods/imports accumulation (panel and the bot's own
`imported_goods` opponent column). The bot's LAST two tables read `customs_bonus_now 0 0` while
the owner stood one move from customs space 7; the +22 landed at e188, after the bot's final
decision (e183, self-read −9), final 85–49. The mechanism is architectural, not informational:
S1 values its own hand-over snapshot; K=1 samples hidden CARDS, not opponent BEHAVIOR; no
feature prices prospective opponent fulfilment — realized values only. **D3 pre-ruled this lane
in-scope; the six-clause instrument fence stands; the Phase-C STOP gates any capability build.**

**F9 — correct family-2 policy on family-3-corrupted inputs → rides F2's branch-3 routing, and
lights the SECOND debt-list admission target.** The g1 contrast table is verified exactly:
e163 (game 1, a forced single-candidate decision — a belief reading, not a choice) decomposes
−983 as `victory_points (19−62)×20 = −860`, `customs_bonus_now 0/8 −88`, `five_power 0/5 −55`,
`country −50`, `trading_houses −20`. Canon 15's inverted brake needs the TRUE margin and the
true remaining-turn count; no deficit-keyed term exists (registry 18 pins translation
invariance), and "terminal-margin awareness" is the debt-list name for the missing input.
Family-2 fixture design must supply true-arithmetic inputs or it tests the corruption, not the
policy.

**F3 — tech-ability blindness → representation admission class, CANON-GATED (batch item C-C).**
Verified end to end: Brickyard bought with `customs_bonus_now 0 0` in its own table (e10,
+70 = (5−0)×14); the turn-1 **Ball-vs-Patent-System keyed-draw coin flip** (g3 e3: margin-0
tie, 2-tie note printed, both "+2 yen" surcharge lines); the Gaslight turn's motive is
tech-majority recovery (+85 at w17, e129–131, self-read −374 at VP 19v37); the EA follow-up
never sources a shophouse (e145: exactly 2 candidates, take-1-assistant vs decline, +14); the
Stagecoach turn (g1 e180–182) steps `tech_bonus_now` 5→10 past the dummy. The lab-rush opening
is confirmed structurally in both on-disk games: the bot's first turn places into Laboratory A,
places the President there, and buys a tech (g1 e7–10 Brickyard; g3 e0–3 Ball). No ability
exists as a feature; no admission is proposed here — the canon batch carries the question.

**F4 — money and basic goods unrepresented → same class, evidence already measured** (T7/T8:
money separates late-only, 0.5627, not admitted). The turn-1 "+2 yen" surcharges are literally
costless in-model (verified in the e3/e10 tables); fees to opponents likewise (g1 e8's opening
placement already pays "fee 1 yen (1 to Red)" with no feature movement). Canon-gated alongside
C-C; the V2-play corpus's money column (this session's counters slice) is the fresh L1 read.

**F5 — recovery's hand terms → PENDING the family-1 adjudicator; the "defect" half may
dissolve.** Verified: at g3 e143 (a strong line available) Return-President reads **−146**
beside EA −136 / Bank −150, far under the chosen church walk — do-nothing is NOT near the top
when a real line exists. At g1 e228 the recall TIES 4-way at 0 (keyed draw) in a position
verified dead-ish (0 yen, 0 imports, one UNAFFORDABLE order — copper 3 + silk 1 + seafood 2
against copper 1 / silk 0 / seafood 7). At g3 e177–183 (the bot's genuinely last turn) the
why-total is IDENTICAL (−9) across e179–183 — the churn signature — in a position verified
stronger than claimed (0 yen, 0 imports, ZERO order cards). **Correction that re-shapes the g1
fixture: e228 was NOT the bot's final move** — it played a further real turn (e238–244:
re-place President at Copper Mine, produce copper), so g1 contributes TWO family-1 roots: the
e228 dead-recall tie and the true final turn at e238–244. If the adjudicator finds these
positions genuinely dead, the ties are honest indifference, not a defect; if not, F5 joins §1
as a second value-separation instance with a calibration follow-up (W6/V3 lane).

**F6 — the tech-majority step → fact base, verified in source.** `technology_majority`
(eval.rs:1394–1448): tiers 10/5 on DOUBLED industry values, 2P dummy entered at 15 (=7.5),
award loop skips the dummy so an out-ranking dummy CONSUMES the tier; wired as the live
`Feature::TechBonusNow` (eval.rs:1236/1260). The whole g1 arc reproduces: Brickyard → second
behind the dummy (5, first table why 21); owner's Newspaper (**e78**, flip visible at why 82) →
bot 0; Gaslight (e131) → 5 again; Stagecoach (e182) → **10, first past the dummy** (industry
9 doubled 18 > dummy 15 > Newspaper 10), exactly the owner's predicted recovery arc.

**F7 — stage vs felt-lateness → instrument note, no routing.** The proxy is clock-based and was
correct at each check (500000 = church 2/4 at the mid-game reads; 750000 at 3/4). Do not
conflate with the R0 self-play length finding.

**F8 — family 4 fired green; T3's first on-policy contact → positive evidence, recorded.**
why 200 verified exactly: 2 root candidates (claim vs end-phase, margin −216 to decline), the
claim taken same-turn after produce-to-7, first-tier 8 VP (the owner's later claim at e236 took
the subsequent 6), and `achievement_progress_sum` **6 → 1 with the opponent's leg pinned at 3**
— the 6 read from the preceding turn's tables (e179–182), the 1 in why-200's hand-over table;
T3's post-claim filter behaving exactly as designed, on policy, for the first time ever.

**Branch 4 — CLI (non-blocking, handover §5's five, plus one candidate this pass adds):** the
verification found the shipped `why` renders only TWO of SearchStats' fifteen counters (`scored`
and `root_candidates`; why.rs:453–461) — D7's mandatory step-one counter check (budget binds /
starved / ply / free-cap) is NOT readable from the shipped screen and had to be instrumented.
Candidate item 6 for the branch-4 list: surface the full counter line in `why`. (Orchestrator's
addition, not owner-flagged; the harness's root table prints the full set regardless, §3.)

---

## 3. The mechanical separator (F1/F5 roots) — protocol, runs pending the harness

Reproduce each frozen root under the shipped contestant (b=4, the reproduction check — the
decision seeds are recorded in the evidence: g1 e113 `0x15e80120dfcbf8c3`, g3 e63
`0x08d402cb00f70b0e`, g3 e93 `0x15eb620d98dc105b`), then at b=8 and b=16, same worlds; report
per beam the top-band structure (top-set size, the ≤1-point band width, distinct-value count,
winner identity) and the full counter set. Disappears ⇒ horizon; persists ⇒ representation
(the scribe's recorded prediction). e93 runs as the positive control at every beam. Readings
land in this document's §3 when the driven-fixture harness merges (its design/plan are this
session's companion docs).

---

## 4. The frame

The owner's closing verdict binds the two halves: a coin flip TO THE ENGINE is still a real
deficiency at the table, and playing to win requires opponent awareness (F2 — branch 3) plus
multi-turn stacking (F3/horizon — the representation queue). §1's keyed-draw placements and
§2's unpriced public threats are those two halves, measured.

---

## 5. Corrections ledger (package/notes vs measurement — both sides verbatim, documents left
untouched)

1. **"+336-era" → 351.** No table in either save prints 336; the pre-fulfilment plateau is
   `Score at that point: 351` (g3 e93–e99). The collapse the finding rests on is intact and
   slightly LARGER (351 → 28/36).
2. **The −983/e163 table is GAME 1's** (Seat 1 Blue; game 3 has its own unrelated e163 reading
   215). The notes' game-3 section interleaves game-1 material — every event in this triage is
   attributed by seat color + content + decision seed, not by section heading.
3. **"e228 was the bot's final move" — refuted.** g1's bot played e238–244 afterward
   (re-place + produce copper). e228 stands as a dead-recall tie root; the true final turn is a
   second root.
4. **"Base VP flatlined at 27 from ~e163" — the (index, value) pair never co-occurs.** Base VP
   was 19 at e163 (held 19 from e87), reached 27 only at e200 (the Seafood7 claim), then held
   27 to the end. The drought is real and LONGER than claimed; the value at e163 was 19.
5. **Newspaper step "~e85–89" → e78** (acquisition), flip visible at why 82; e85–89 are the
   bot's own church-payment events.
6. **Stagecoach "+90" is weight-fixed arithmetic, not a printed delta**: the printed products
   are 85 (e158, w17, stage 500000) → 180 (e180–182, w18, stage 750000), i.e. +95 raw; +90
   holds only at the later weight applied to both sides. "Past the dummy into first" is
   confirmed from source semantics + industry arithmetic, not from any printed line.
7. **e93's "runner-up wall at −116" refined**: exactly 3 candidates sit at −116; the modal wall
   is −140/−142/−144 (25/63/35 candidates). A separator sized off "the wall" should use −142.
8. **why-series semantics caveat (affects any narrative use):** `Score at that point` is the
   hand-off value of the searched PRINCIPAL LINE, not a position evaluation — g3's 194 (e137)
   → 58/44/48 (e139–141) → 194 (e142) inside one turn is projection-length variance, not a
   collapse. And g1's terminal tables read the −1081000000 loss sentinel (e238–244), which
   poisons any naive aggregation over why totals.
9. **The headline belief readings are FORCED decisions** (−983 at g1 e163, −9 at g3 e183, 194
   at g3 e149: all `1 root candidate`) — right evidence for what the bot BELIEVED (F2), wrong
   evidence for any claim about its choices.
10. **"Nothing fulfillable" at e228** means an unaffordable held order (O24), not an empty
    hand; at g3 e178 the hand really is order-empty.
11. **Unverified residue, said out loud:** the handover's "e213 EA visit" (second
    assistants-only take) was not re-derived this pass; the five-game lab-rush count rests on
    the two on-disk games (verified) plus the notes' banked casual-game data (the saves were
    discarded at owner instruction — three of five games are testimony, the mechanism is
    verified).

*Evidence provenance: workflow `d7-evidence` (6/6 agents), raw transcripts under the session
scratchpad (`d7/`); decision seeds, level distributions, and counter tables quoted above are in
the agents' structured reports. The verification pass wrote nothing into the tree.*
