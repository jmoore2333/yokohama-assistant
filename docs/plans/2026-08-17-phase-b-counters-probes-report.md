# Phase B — the V2-play counters baseline slice + the four-family exploratory probes

**Written 2026-08-17 (Phase B analysis session). Kickoff items 4 and 2, one report — they share
the corpus. The slice is D8's "V2-play baseline of the same counters run, taken cheaply before
W6 touches anything": the regime fact sheet the W6 regime check will be judged against. The
probes are layer-1 exploratory material for D2's four families — unfrozen, cheap, NEVER GO
evidence; they inform layer 2's generators and controls.**

## 1. Provenance (the command is part of the record — the W3-OPEN lesson)

```
cargo run --release -p yokohama-core --example eval_probe -- run 2 400 \
  --contestant lookahead-v1 --weights v2 --master 0xE7A1202608170001 \
  --blocks features,proxy,counters \
  --candidate-columns postclaim,church_majority_margin,customs_majority_margin,tech_majority_margin,achievement_race_proximity,clock_control_church,port_empty_slots,money
```

Master `0xE7A1202608170001` (0xE7A1_2026_0817_0001): the next free E7A1 probe-family serial by
date order, recorded in the execution ledger's PHASE-B-ANALYSIS OPEN row (commit `8c92c88`)
BEFORE the run, never redrawn. 2P, 400 games, release, per-move invariants ON, shipped
`lookahead-v1{b=4,k=1,m=8,weights=v2:0x9a240f2ff7624b0e}` self-play (uncapped — no cap token).
Corpus wall **214.31s = 535.8 ms/game (WSL box, Intel Core Ultra 7 265K; the R0-L 330.2
ms/game figure is an M5 reading — machines differ, do not compare walls)**. 116,521 applied
moves, 30,824 snapshots. Raw output retained in the session scratchpad
(`counters_slice.log`); the load-bearing lines are quoted below.

## 2. The regime fact sheet (vs R0-L, the V1-play lookahead corpus, N=2,000)

| Quantity | R0-L (V1 play) | THIS SLICE (V2 play) | Read |
|---|---|---|---|
| Game length, turns | median 38 · p90 54 · p99 74 · max 108 | **median 38 · p90 54 · p99 74 · max 98** | **UNCHANGED** — V2 did NOT lengthen games; the half-length-vs-greedy regime (greedy median 80) stands |
| Game length, moves | 286 / 406 / 551 / 757 | 290 / 409 / 558 / 696 | unchanged |
| End-trigger mix | Church 99.20% · Deck 0.75% · Customs 0.05% | **Church 98.25% [0.9643, 0.9915] · Deck 1.75%** | church monoculture stands; deck endings doubled (7/400), customs endings absent |
| Stage proxy, fixed cuts | **0.5090** (best in-sample 600000/800000 → 0.6238) | **0.6660** (best in-sample 500000/750000 → 0.6678; base rate 0.3421) | the headline regime change — see §3 |
| Proxy saturation (1000000 ppm) | 0.0663 of turn starts | 0.0696 | unchanged |
| Nonzero-recovery share | 0.004655 | **0.002660** | still ~74× the greedy rarity (0.000036); gate 15's floor rides the GREEDY corpus and is untouched |
| Retreat / area / build shares | 0.0127 / 0.1289 / 0.0155 | 0.0139 / 0.1282 / 0.0162 | stable |
| Candidates scored per decision | 749.97 | **959.81** | +28% — V2-play positions carry wider live candidate sets; per-decision wall derives to ~1.84 ms/move whole-corpus (WSL) |
| §0.3(b) explanation identity | 587,528/587,528, 0 mismatches | **116,521/116,521 over 800 blocks, 0 mismatches** | exact |
| Cap/budget counters | all 0 (uncapped shape) | **all 0** (ply 0 · free 0 · binds 0 · starved 0 · placement-cap 0) | healthy uncapped regime |
| EVAL TOTAL AUC (E/M/L) | 0.5314 / 0.7126 / 0.9107 | **0.5915 [0.5358, 0.6461] / 0.7731 / 0.9248** | the early near-chance finding (R0 F1) is GONE under V2 play |

**Known instrument gap, recorded not patched:** the counters block prints NO tie-share line
(D8's regime-check list names one — and F1 makes it THE interesting counter). Adding a
lookahead-block line is future instrument work; the GREEDY block stays frozen per the standing
ledger note. Until then, tie behavior is measured at rooted positions via the driven harness.

## 3. What moved and why it matters (the ecology under V2 play)

- **R0's F1/F2 are resolved in-ecology.** The two CONTRADICTED early rows are neutralized —
  `victory_points` EARLY 0.4651@w10 → 0.5046@w1 (no-signal at a near-zero weight),
  `assistants_on_board` EARLY 0.4464@w14 → 0.4734@w6 — and EVAL TOTAL EARLY now excludes
  chance. The round-1 early re-mix did in the shipped regime what it was fitted to do.
- **The stage proxy's mis-calibration was the clock-fill SHAPE, not game length.** Length is
  identical (median 38 both corpora) yet accuracy moved 0.5090 → 0.6660 (true-MID→pLATE
  misclassification fell 88% → 67%; the best in-sample cuts moved from 600000/800000 back to
  ~the fixed cuts). Working hypothesis, stated as such: V1's chur_l=16 drove early church
  filling so the clock front-ran the terciles; V2's halving (chur_l=8) spread the fill. R0-F3's
  "half-length games race the clocks" account is therefore incomplete — the LENGTH half of that
  story did not change and the calibration still recovered.
- **`church_bonus_now` LATE regained signal at the halved weight** (V1-play no-signal 0.5180@16
  → V2-play 0.6006 confirmed @8) — coherent with round 2's c1 rationale.
- **`hand_size` softened weights now sit on a real negative late signal** (0.4309 confirmed
  LATE at −2; V1-play was no-signal everywhere at −3/−6).
- **`assistants_in_hand_core` is still a near-constant** (mean 1.99–2.00 every stage,
  no-signal at weight 30, the table's largest) — F5's mechanism persists in-ecology; the
  recovery-hand-term calibration question stays open pending the family-1 adjudication.
- **Fresh L1 signal: `customs_majority_margin` separates from the shipped customs mirror under
  V2 play** — 0.5864/0.6799/**0.7271** vs `customs_bonus_now` 0.5812/0.6699/0.6853 (+0.042
  LATE, point estimates; intervals overlap). Round 3 measured the customs pair IDENTICAL to
  four decimals under V1 play — the duplication verdict is regime-conditional and this column
  re-enters candidacy on V2-play evidence. `tech_majority_margin`, `church_majority_margin`
  and `clock_control_church` still read as duplicates of their shipped mirrors;
  `port_empty_slots` is still structurally constant; `money` is still late-only-weak
  (0.5764 [0.5173, 0.6337] LATE — F4's representation gap, unchanged in class).
- **`achievement_race_proximity` keeps its edge over the shipped progress-sum, but the edge
  MOVED stages**: V1-play read +0.054 EARLY / +0.094 MID; V2-play reads −0.001 EARLY /
  **+0.021 MID / +0.076 LATE** (0.5717 vs 0.4959 LATE — the progress-sum's late sag is the
  banked-claim proxy effect, exactly what T3/postclaim reasoning predicts). The V3-candidate
  status is refreshed on current-regime evidence; any admission proposal re-derives at the
  then-current table per the ladder.

## 4. The four-family probe readout (layer-2 design inputs, never GO evidence)

**Family 1 — already-triggered final round.** The corpus holds the ecology: 6.96% of turn
starts sit at clock saturation. The owner games contribute three rooted candidates, all
verified: g3 e177–183 (the bot's genuinely-final turn: place → produce 3 seafood → skips, why
total pinned at −9 across the whole span, 0 yen / 0 imports / zero order cards), g1 e228 (the
dead-recall 4-way tie at margin 0; one UNAFFORDABLE order held), and — the triage's correction
— g1's true final turn e238–244 (re-place + produce copper). The harness's
`FinalRoundAlreadyTriggered` generator predicate plus `adjudicate_root` (force-each-candidate
playouts) is the layer-2 instrument; "genuinely dead" becomes a measured verdict per root.

**Family 2 — end-trigger timing under the inverted brake.** The clock ecology is still
church-monoculture (98.25%), so generators for non-church-clock roots must be CONSTRUCTED or
accept conditioning on the dominant clock; deck-ending roots now exist organically (7/400) if
a generator wants them. The stage proxy (a control input for "true remaining-turn count") is
materially better calibrated under V2 play. The corrupting confound is F9's verified
terminal-margin blindness: canon 15's accelerate-when-ahead ran on a self-read that omitted
~50 visible pending points — family-2 fixtures must supply TRUE arithmetic (the
terminal-margin-awareness debt-list target) or they test the corruption, not the policy.

**Family 3 — exclusive-race opponent awareness.** The owner games are the probe: the verified
F2 series (351 → 28/36 → 194 → −9 against public, computable threats) is the cleanest rooted
evidence, and the D7 triage routes it branch-3. Corpus side: `customs_majority_margin`'s fresh
late separation gives family 3 a measurable race-position signal to build controls around; the
masked-starting-order confound (opponent under-read mid-game, exact at terminal) stands from
the amendment and must shape fixture design. The no-denial control is canon (12/13).

**Family 4 — achievement proximity.** F8's verified on-policy contact (produce-to-7 →
same-turn claim → 8-VP first tier; `achievement_progress_sum` 6→1 with the opponent's leg
untouched) is the existence demonstration; the corpus adds the population read (proximity's
MID/LATE edge over the shipped sum). The pre-bar trigger test D2 names (proximity → step
conversion at rooted positions) has its instrument in the harness's `root_table` +
`drive_to_terminal`.

## 5. What this report deliberately does not do

No weight change is proposed, no admission is proposed, no family is frozen, and no GO/pivot
reading is taken — those are Phase C acts with their own pre-registration and canon gates. The
tie-share instrument gap and the harness dependency (family-1 adjudication, the F1/F5
separator) are the named follow-ups.
