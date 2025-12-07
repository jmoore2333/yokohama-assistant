# A1 Notes (rulebook-first) – v3

Sources reviewed: 2025-12-07-A1-foundation.md, 2025-12-07-A1-implementation.md, 2025-12-07-yokohama-assistant-design.md, yokohama-kb (rules/quick refs). Rulebook alignment called out where material.

## Executive summary
- Foundation types are solid and map well to rulebook concepts. Minimal risk areas: hidden info flagging, setup profiles by player count, and validated board layout construction.
- Implementation plan covers most flows with clear module boundaries. Remaining work clusters around movement/placement tech hooks, Port/Lab edge rules, track actions, five-power movement, recovery/Telephone, and endgame (triggering and scoring) polish.
- Biggest rules gaps to close: tech-aware movement/placement legality, Port/Lab payments and limits, Church/Customs exact caps, mercenary-blocking vs opponent presidents, and country-set scoring constraints.

---

## Rulebook-aligned corrections and confirmations
These are either corrections to earlier drafts or confirmations the plan now matches the rulebook. Add tests where noted.

1) Laboratory (Industry Value)
- Correct: Power contributes 1:1 IV (cap 5); 2 Yen = 1 IV; 1 Imported Good = 1 IV; no fractional IV; slot surcharges (+1/+2); fifth-tech penalty (pay 2 Yen OR 1 Imported Good).
- Action: Ensure tests cover combos (power cap interaction, surcharges, fifth-tech penalty) and reject underpayments.

2) Port
- Hand limit 3 must be enforced; accessible slots by power (2: Low; 3: Low/Mid; 4–5: all); max 2 cards per action; second card must be paid (2 Yen or 1 Imported Good); Power 5 grants +3 VP.
- Action: Tests for hand-limit violations, second-card payment, Power-5 bonus, and post-take slide/refill.

3) Church
- Faith-from-power mapping: 1→1, 2→1, 3→3, 4→4, 5→5; may add at most one of each good (Yen included) per action.
- Requires spending an assistant from hand to place on track; apply Winery bonus on placement.
- Action: Tests for power=2 special case, per-good maximum, and assistant availability.

4) Customs
- Max imported goods payable limited by power; requires spending an assistant from hand; apply Brickyard bonus on placement.
- Action: Tests for power-limited payments and assistant requirement.

5) Movement and stopping
- Must implement: assistant-chain traversal for entire path; canal tolls; president entry fees; tech exceptions:
  - Stagecoach: free passage through opponent presidents (no fees to pass; fees may still apply to enter without Stagecoach).
  - Postal System: may stop/act in an area with opponent president.
  - Station: +1 power for all in that area; owner may teleport there in Movement step regardless of other presidents.
- Action: Tests that demonstrate pass-through, stop permission, toll/fee accounting, and Station teleport eligibility.

6) Placement options and tech
- Default: either 3 different areas (1 each) or 2 in the same area.
- Newspaper: up to 4 different areas (Option A variant).
- Telegram: after placing 2 in same area, place 1 in an adjacent area (bonus), respecting board adjacency.
- Action: Legal-move generation must be tech-aware; add pruning to avoid combinatorial blowup; tests for both techs.

7) Building step
- Costs: Trading House price ladder 4/5/6/7 Yen; shophouse slot constraints (one per player per area) with site VP; apply Gaslight/Electrical Light VP on build; enforce power thresholds per rulebook for shop vs trading house.
- Action: Encode thresholds as constants (from quick ref) and test both builds and bonus VP.

8) Mercenary additional action
- Limit: once per turn across both additional-action halves; power calculation excludes president.
- Open point: Clarify whether an opponent president in target area still blocks the action (rulebook blocking vs Postal System exception). Current generation only checks “no own president.” If standard blocking applies, add an opponent-president check unless Postal System owned; otherwise, document exception.
- Action: Decide rule, implement check, and add tests including return-to-same-phase behavior after recovery.

9) Five-Power tokens
- Apply effect once, remove from board, mark collected (count suffices for scoring/achievements). Movement-type token still TODO—define exact effect and integrate with Movement step without double-moving.
- Action: Implement movement bonus; add idempotency test (no re-trigger once taken).

10) End-game triggers and flow
- Triggers: 4/5/6 assistants on Church/Customs (2p/3p/4p), any player builds all 4 Trading Houses, any player builds all 8 Shophouses, Order deck empty AND a Port slot cannot be refilled.
- Flow: Finish current round then one final round; end after player before start player completes TurnEnd.
- Action: Tests for each trigger and full final-round sequencing.

11) End-game scoring
- Majority bonuses: Church 6/3 VP, Customs 8/4 VP; Technology by total industry value 10/5 VP (tie: closest to start player clockwise).
- Country sets: sets must consist of distinct countries; each icon used at most once; score 2/4/8/12 for 2/3/4/5 different.
- Remainder: unused mercenaries 1 VP each; imported goods 1 VP each; 2 Yen = 1 VP; (if used) remainder trade goods 1 VP per 3.
- Action: Implement greedy matching for country sets with distinctness constraint; add tie-break tests for tech majority.

---

## Gaps to close in A1-Implementation (blocking A2)
- Movement: implement Stagecoach free pass, Postal stop/act, Station teleport; enforce assistant-chain along full path; canal tolls and president fees; affordability checks.
- Placement: integrate Newspaper and Telegram; ensure legality generation respects techs and avoids explosion.
- Port/Lab: complete validation and payments; ensure fifth-tech penalty; enforce no >2 cards; ensure slide/refill logic.
- Tracks: enforce assistant-from-hand; apply Winery/Brickyard VPs; cap church faith and customs payments per rules.
- Building: enforce power thresholds; apply Gaslight/Electrical VPs; one shophouse per player per area; trading-house ladder costs.
- Recovery: implement Telephone (leave exactly 1 assistant in action area) else recover all.
- Five-Power: implement movement token effect; ensure taken-once semantics.
- Mercenary: resolve opponent-president blocking rule; ensure once-per-turn flag covers both halves; sub-turn returns to correct additional-action phase.
- Country bonus: distinct-country set constraint; implement greedy largest-to-smallest packing.
- End game: validate “post-replenish” condition for Order-deck trigger; finalize final-round completion check.
- Setup glue: connect `GameConfig` to 2p/3p area removals and to presence/absence of Port B / Lab B.

---

## Foundation feedback (types/state)
- IDs: You already provide `try_new`/`TryFrom`—use these at API boundaries (CV ingest) to avoid panics; keep `new` for internal logic.
- BoardLayoutBuilder: Good validation coverage (bidirectional adjacency, site per area, 5-power presence); add variant-aware checks for 2p/3p removed areas.
- PlayerState: Hidden-info field for starting order is correct; assistants/shophouse/trading-house trackers look sufficient. Consider whether tracking the specific five-power effects collected is needed (count-only is okay for base scoring; optional enhancement for analytics).
- TechEffect: Coverage looks complete for base set; ensure Station semantics match movement rules (owner teleport and global +1 power in area).
- GameState: Additional-action sub-turn fields are appropriate; double-check `is_game_over` logic once final-round sequencing is implemented.

---

## Design doc alignment (no changes required, just emphasis)
- Perfect-information assumption is correct: only the starting order card is hidden. UI must capture which of the two starting orders is kept.
- Persist the randomized board layout (adjacency, site VP, five-power tokens) at setup; evaluator depends on topology.
- Turn-capture two-image delta flow is ideal; keep clarification batching for low-friction UX.

---

## Test plan additions (A1 scope)
Add or extend unit/integration tests for:
- Laboratory: IV calculation (power cap, 2Y=1IV, 1IG=1IV, surcharges, fifth-tech penalty).
- Port: hand-limit=3, second-card payment, max 2 cards, Power-5 +3 VP, slide/refill.
- Church: power→faith mapping (especially power=2), one-of-each-good constraint, assistant-from-hand, Winery bonus.
- Customs: power-limited payments, assistant-from-hand, Brickyard bonus.
- Movement: assistant-chain validation; canal tolls; president entry fees; Stagecoach free pass; Postal stop/act; Station teleport regardless of presidents.
- Building: shophouse slot restriction, trading-house ladder cost, Gaslight/Electrical VP.
- Recovery: Telephone leaves 1 assistant; default recovers all otherwise.
- Five-Power: claim at power 5, effect once, removal from board, movement token semantics.
- Mercenary: once-per-turn across both halves; power excludes president; opponent-president blocking per decided rule; returns to additional-action phase after recovery.
- Endgame: each trigger path; final-round order; scoring totals including majorities, tech tie-break, country sets distinctness, and remainder scoring.

---

## Recommended next actions (sequence)
1) Finish movement/placement tech hooks (Stagecoach, Postal, Station, Newspaper, Telegram) with legality + cost checks.
2) Lock down Port/Lab validations (hand limit, payments, surcharges, fifth-tech) and add comprehensive tests.
3) Complete Church/Customs with assistant-from-hand and bonus VPs; add tests for caps and thresholds by player count.
4) Implement Telephone and five-power movement token; add idempotency tests.
5) Implement country-set distinctness + greedy packing and technology-majority tie-breaks; finalize endgame triggers (post-replenish) and final-round completion.
6) Wire `GameConfig` to setup variants (2p/3p removals, Port B/Lab B presence) and add setup validators.

If you want, I can convert the above into concrete checklists inside the A1-implementation doc and add failing tests to drive the work.
