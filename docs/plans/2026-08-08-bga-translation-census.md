# BGA translation census — H2 T7 (measurement BEFORE the translator)

**Date:** 2026-08-08 · **Phase:** 3.5b H2 · **Corpus:** `bridge-captures/895724349.ndjson` (gitignored; 491 lines, complete 2P game, surviving `move_id` 1..194)
**Plan:** `docs/plans/2026-08-08-phase3.5b-bridge-plan.md` §3 T7 · **Wire authority:** `docs/plans/2026-08-08-bga-notification-inventory.md`

Every figure below was measured from the corpus by parsing it (python3 over the NDJSON, the inventory §0 parse contract). Nothing is inferred from BGA documentation or memory. This census asserts **no threshold and no direction** — its job is to size T8 honestly and to state, before any translator ran, how much of the acceptance run the evidence supports.

---

## 1. THE HEADLINE: plan §0.5's bound is OVERTURNED — a move-1 replay is evidence-supported

Plan §0.5 said the initial Port A six and Lab A six "were never observed … not recoverable backwards", bounding this corpus's acceptance run away from move 1. That was measured against the notification stream alone. The snapshots overturn it, through a mechanism §0.5 did not consider: **BGA's `order_key`/`tech_key` are DEAL-ORDER SLOT KEYS, and the display mechanics are measurable.**

Measured, across all three snapshots and every socket-era card map:

- **Displays slide left and refill at the right with strictly ascending keys.** Port display: snapshot@A keys 7–12 (positions 1–6), snapshot@B keys 9–14, snapshot@C keys 14–19. Lab display: keys 2–7 → {2,5,6,7,8,9} → {2,6,7,8,9,10} — in every case the surviving keys keep their relative order and each refill is the LOWEST key not yet seen outside the deck.
- **The deck depletes in ascending key order**: order deck keys 13–20 → 15–20 → {20}; tech deck keys 8–30 → 10–30 → 11–30. Zero violations anywhere in the corpus.
- Working the mechanism backward: **initial Port A = `order_key` 5–10** (keys 5,6 are the two LOW-zone takes at m13/m14 — power 2 gave "access to Low", and the takes' identities sit at `location 4` in snapshot@A; keys 7–10 are snapshot@A display positions 1–4; keys 11,12 are the m14 refill pair at positions 5,6), and **initial Lab A = `tech_key` 1–6** (key 1 is the m6 University take, held at `location 4`; keys 2–6 are snapshot@A display positions 1–5; key 7 is the m6 refill at position 6).
- **Order keys 1–4 are the four dealt starting orders**, two consecutive keys per seat: key 1 = XeroxMan's discard (m2), key 2 = his kept card (`order_id 27`, the card inventory §7.2 proved he held all game), key 3 = acmuriel's discard (m3), key 4 = her kept card (revealed by completion at m18).

**The §8.3 deck fence is honored by construction**: no `location "1"` row's identity is ever read. Every identity above is taken from a row at location 2/4/5 — the moments our model calls reveals — and the key arithmetic uses only the KEY (a slot number), never an identity field. Refill reveals during the replay take each card's identity from the earliest snapshot/notification where that key sits at location 2/4.

Every consequence is verifiable at replay time: a key-model prediction that disagrees with any later observed display is a hard error, not a shrug.

## 2. Setup facts, with their evidence class

| Field | Status | Evidence |
|---|---|---|
| Seats, colors, start seat, own seat | **EVIDENCED** (H1 sheet) | `gamedatas.players`, first `assistantPlacement`, `/player/p88889596` channel |
| Board tiles (10, row-major) | **EVIDENCED** (H1 sheet) | tile links + six-link differential |
| Port A initial six | **EVIDENCED** (this census §1) | key arithmetic + location-2/4 identities: keys 5–10 = BGA order_ids 23, 22, 7, 21, 11, 33 |
| Lab A initial six | **EVIDENCED** (§1) | tech_keys 1–6 = tech_ids 12, 4, 26, 14, 2, 27 — ALL SIX in the committed ledger (University, Tram, Ball, Exposition, Mining Technology, Newspaper) |
| Starting orders | **EVIDENCED** | own seat Known (key 2 = BGA 27); opponent Unknown, revealed at m18 completion (key 4 = BGA 36) — the live model's normal path |
| 5-Power token faces | **4 of 4 codes seen mapped; 1 tile assumption** | powerBonus events × tile context: code 3 = Tea(3) (m102, Laboratory tile), code 5 = Seafood(3) (m36 Copper Mine, m88 Church — pool holds exactly 2), code 6 = Import(1) (m53/m73/m160; 5 on board, pool holds 10). Code 2 (Tea Plantation tile, never taken) is UNMAPPED → declared assumption, replay-inert |
| Achievements A/B/C | **A, B decidable at replay; C assumption** | A = class 1 qty 7 (Tea7\|Seafood7), claimed m43; B = class 2 qty 5 (Tokens5\|Orders5), claimed m90 — the claim's engine legality decides (engine Tokens5 counts power tokens + mercenaries: 4+1=5 at m90 ✓ matches BGA's counter). C = qty 2/3 (Prod2Comm3\|Prod3Comm2), NEVER claimed → declared assumption, replay-inert |
| Building sites | **3 constrained, 7 assumptions** | BGA `build_card` is NOT our numbering (permutation — proven: build_card 18's build granted "2 tea" at m115, and no site numbered 18/19 in our registry carries a tea space; only site_003/site_004 do). Constraints: t2 (Port) ∈ {site_003, site_004}; t10, t5 each ∈ the five sites with a `movement: 2` space (m37/m54 grant "2 move(s)"). Unbuilt tiles' sites are replay-inert declared assumptions |

**The record-shape consequence:** `reconcile_board` requires every non-Canal cell to carry a site and (when tokens are observed) a token face, and `SetupObservations.achievements` is all-or-nothing. The replay-inert unknowns above therefore MUST be filled to build a `Session`. They are filled with DECLARED assumptions — enumerated candidate sets, deterministic canonical pick, each verified replay-inert (never claimed / never built / never taken) — and the acceptance report carries the list. This is constraint-solving over stated candidates with the survivors recorded, not guessing: a candidate the replay or a checkpoint contradicts is an error.

## 3. The 194-move classification

Method: for each surviving `move_id`, the multiset of non-chrome notification kinds (chrome = the inventory §7.3 list), classified by the engine surface it drives. `[thin]` = history-only coverage (moves 1–106), `[sock]` = socket-enriched (107–194).

Class totals (per-move, classes are compound):

| n | class |
|---|---|
| 44 | EMPTY (no non-chrome notification — BGA phase-pump moves; 1 is the game-start packet) |
| 26 | PLACEMENT (2 with FEE-CORROBORATION — `payForPres` at m22, m76) |
| 44 | MOVEMENT+ACTION (president placement + area-action effect; 6 also TAKE-5POWER) |
| 13 | ASSIST-MOVE-CHOICE (movement-icon resolutions; 1 also TAKE-5POWER at m88) |
| 9 | PORT-TAKE (m13,14,24,25 thin; m114,128,145,169,170 sock) + 6 PORT-REFILL-REVEAL |
| 4 | LAB-TAKE+REVEAL (m6, 30, 102 thin; m120 sock) + 1 LAB-DECLINED (m176) |
| 10 | FULFILL-ORDER (m18,45,70,94 thin; m110,131,135,155,171,172 sock) |
| 2 | FULFILL-ACHIEVEMENT (m43 class A, m90 class B — both thin) |
| 3 | BUILD-SHOP (m37, m54 thin; m115 sock) |
| 4 | CHURCH (m82, m105 thin; m153, m180 sock) |
| 2 | CUSTOMS (m160, m189 — both sock) |
| 2 | WAREHOUSE-TAKE (m95 thin; m190 sock) |
| 3 | UNIVERSITY (m15, m44, m98 — all thin) |
| 1 | PORT-SECOND-DECLINED (m129, sock) · 1 RETURN-PRESIDENT (m193) · 1 RETURN-RECOVER-NONE+FINAL-SCORING (m194) · 1 EOG-TRIGGER (m183) · 1 LAST-ROUND (m185) |

Mercenary uses: 3 `resourceChange` mercenary-variant notifications (`fa_used` ends at 2 for XeroxMan, 1 for acmuriel). Every class is nonempty where the game exercised it; `stationPlacement`/`buildHouse`/`removeAssistants` never fired (inventory §7.4 — hard-error arms).

Evidence-ladder placement per class (plan T7's four classes):

- **Determined**: SETUP-DISCARD, FULFILL-ORDER (history keeps the full card), FULFILL-ACHIEVEMENT (class letter + unique in-play row), BUILD-SHOP (tile named; slot by reward-match), UNIVERSITY, RETURN-PRESIDENT+RECOVER, EOG markers, all [sock] card-bearing classes.
- **EngineFiltered**: PLACEMENT (names → areas; Option A/B by log template), MOVEMENT (destination name + fee corroboration against the legal list's `(destination, credit-vector)` labels — the 2P board has no Canal, one opponent, so `payForPres` pins the label), TAKE-5POWER/skip (presence/absence of `powerBonus` against the phase), CHURCH (res_list template + faith + `vp_count`/`move_count` against the church-track data), CUSTOMS, WAREHOUSE-TAKE, ASSIST-MOVE-CHOICE ([sock]: `from_id`/`to_id`; [thin]: the three log templates, direction-bearing).
- **SocketOnly → resolved by key model**: thin PORT-TAKE identities (m13/14 = keys 5,6; m24/25 = keys 7,8 — pinned by snapshot@A location-4 rows), thin PORT-REFILL-REVEAL and LAB-REVEAL identities (next ascending deck key, identity from the earliest location-2/4 observation of that key).
- **Underdetermined residue**: NONE that blocks translation — but see §4's risk sites, two of which are candidate ENGINE-RULES divergences rather than translation gaps.

## 4. Risk sites the translator will hit, named in advance

1. **m82 — church yen payment price.** BGA: `res_list "1 tea,1 fish,1 silk,1 copper,2 yens"`, `added_faith 5` — the yen TYPE cost **2 yen** for its +1 faith. Engine (`validate_church_action`/`apply`): the yen type costs **1 yen**. Faith agrees (5 types = +5); the yens counter diverges by 1 from m82 on. If real, this is an owner-gated RULES question (engine vs BGA church yen price), surfacing at the first yen-sensitive checkpoint after m82 — NOT an engine edit, NOT a translator hack.
2. **m106/m107, m154 — movement icon TO HAND.** BGA moves an assistant "from ⟨area⟩ to their hand" (m106 Customs→hand, m154 Bank→hand). Engine `ChooseAssistantMove{from, to: AreaId}` cannot target the hand. m106+m107 compose to a state-equivalent pair (Customs→hand, hand→EA ≡ Customs→EA + decline) — translatable with a stated composition rule. **m154 has no second leg** (the assistant PARKS in hand): state-inequivalent, untranslatable without widening the engine → candidate STOP at m154 (79% of the game) pending an owner rules ruling on movement-icon targets.
3. **m88 — 5-power take after movement-icon resolutions.** BGA fired `powerBonus` (Church token, 3 Fish) after three assistant-move icons resolved. Engine order (choices resolve before the FivePowerBonus phase) reaches the same evaluation point; whether Church still counts ≥5 power there is decided by the replayed state. If BGA grants where the engine does not, that is a phase-timing rules question.
4. **`power_tokens` counter ≠ take count — RESOLVED, no risk.** BGA's counter hit 5 by m90 on 4 board takes; the engine's `Tokens5` progress counts five-power tokens PLUS mercenaries, and XeroxMan held 1 mercenary — 4+1=5, both models agree.
5. **Snapshot `move_nbr` LAGS truth** (snapshot "99" contains m102's take; snapshot "16" contains m18's completion). Checkpoint anchoring must match on CONTENT (counters/card locations), never on `move_nbr`.

## 5. What T8/T9 build on this

- The translator is a STREAM consumer: notifications (socket form preferred, history form the spine) consumed against the engine's `legal()` + pending-choice queue; BGA move boundaries are bookkeeping only (choices span them: m37's build grants icons resolved at m39/m41).
- Reveal choreography: port/lab refills = `RevealOrders`/`RevealTechs` bound to the take move (key-model identities); the opponent's starting order = `apply_standalone(RevealStartingOrder)` immediately before its m18 completion.
- Skips are absence: a phase the engine must pass with no matching notification in the move window translates to `Skip`/`Decline`/`EndAdditionalPhase` — each still checked against `legal()`.
- T9 checkpoints: `counters` on every socket packet (m107+ dense), content-anchored snapshots (§4.5), and the `tableWindow finalScoring` terminal table (engine `final_scores` vs BGA's, per player, per category).

*Measured 2026-08-08. No engine, oracle, or difftest source was modified; the corpus is data, not source.*
