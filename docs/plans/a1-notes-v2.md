# A1 Notes (rulebook-first) – v2

Sources reviewed: `00-yokohama-rulebook.pdf`, `yokohama-kb` (rules/quick refs), `2025-12-07-A1-foundation.md`, `2025-12-07-A1-implementation.md`, `2025-12-07-yokohama-assistant-design.md`.

## Rulebook-aligned corrections to bake in
- Laboratory industry value is **2 Yen = 1 IV**, not “+0.5 per 2 Yen” (see mismatch below). Keep 1 Imported Good = 1 IV; cap power contribution at 5.  
``` 
61:65:/home/jmoore/Documents/yokohama/analysis-1/yokohama-kb/02-quick-reference.md
LABORATORY:
  # Power converts to Industry Value 1:1 (max 5)
  # Additional sources: 2 yen = 1 IV, 1 imported_good = 1 IV
  # Card cost = printed_cost + position_modifier (0, +1, or +2)
```
``` 
3955:3959:/home/jmoore/Documents/yokohama/docs/plans/2025-12-07-A1-implementation.md
**Requirements:**
- [ ] Base industry: Power (max 5)
- [ ] Bonus industry: +0.5 per 2 Yen spent, +1 per Imported Good spent
- [ ] Position surcharges: Slots 3-4 add +1/+2 to cost
```
- Church faith curve and “max 1 of each good per action” need to be enforced exactly (Power 2 still yields only 1 Faith).  
``` 
95:108:/home/jmoore/Documents/yokohama/analysis-1/yokohama-kb/02-quick-reference.md
CHURCH:
  faith_from_power:
    power_1: 1
    power_2: 1
    power_3: 3
    power_4: 4
    power_5: 5
  faith_from_goods:  # max 1 each per turn
    copper: 1
    silk: 1
    tea: 1
    seafood: 1
    yen: 1
    imported_good: 1
```
- Country set scoring: each icon can only be used once and sets must have distinct countries (rulebook Country Bonus). Keep greedy “largest sets first” but disallow duplicates per set.
- Order-deck trigger: fire only when the deck is empty **and** a Port slot cannot be refilled after a draw; check post-replenish.
- Mercenary use: once per turn; target area must have your assistant and no your president (opponent presidents allowed, no fee); after recovery return to same Additional Action phase.

## Gaps / TODOs in A1-Implementation plan
- Movement still marked TODO: Stagecoach free pass (no fees), Postal System stop-on-opponent, Station teleport (owner only, same-area action), assistant chain across full path, canal tolls and president fees.
- Placement legal-move gen: missing Newspaper (4 areas) and Telegram (adjacent bonus) plus president-fee affordability when entering occupied areas.
- Port: enforce hand limit 3, slot occupancy, second-card payment (2 Yen or 1 Imported Good), +3 VP at Power 5, and prevent >2 cards per action.
- Laboratory: fix IV math (above), position surcharges, fifth-tech surcharge (2 Yen or 1 Imported Good), enforce max 5 IV from power, slide/refill after purchase.
- Tracks: Church/Customs must require spending an assistant from hand and respect power-based max imported goods; apply Winery/Brickyard VP when placing.
- Building: trading house costs 4/5/6/7; apply Gaslight (+2 VP on shop) and Electrical Light (+3 VP on trading house).
- Recovery: Telephone leaves 1 assistant; default recovers all.
- Tech hooks: Mining Tech (+1 copper when gaining any copper), Ball (+2 VP per order), Language School (+1 Yen per order), Patent System (+2 VP per tech after), Letterpress Printing (+1 Yen per tech after), Spinning Mill (once/turn silk→2 Yen), System of Stock (start of turn top-up to 2 Yen if ≤1), University (pay 1 Yen once/turn to pull assistant from warehouse).
- 5-Power tokens: ensure effects apply once, token removed/owned, and movement-type bonuses are handled.
- Country bonus bug already noted in plan; implement distinct-country sets.

## Foundation follow-ups
- Ensure BoardLayout builder also validates 5-Power token placement and building-site presence for every in-play area (2–4p setups differ).
- Consider non-panicking constructors at API boundaries (CV ingest) even though `try_new` helpers are listed; keep both safe and `unchecked`.
- Persist hidden-info flag on starting Order card (only secrecy in base rules).

## Design doc notes
- Assumption that AI tracks opponents’ Order cards except the initial keep-one is correct; requires stable slot/card IDs from CV to be reliable.  
``` 
17:35:/home/jmoore/Documents/yokohama/docs/plans/2025-12-07-yokohama-assistant-design.md
Yokohama is **nearly perfect information**:
- **Public:** ... all order cards picked up or completed
- **Hidden:** Only each player's starting Order card (draw 2, keep 1 at game start)
...
The AI knows what Order cards **ALL players** currently hold...
```
- Board layout (adjacency, building-site VP, 5-Power tokens) must be captured once at setup and persisted for the Rust engine and evaluator; plan already calls this out—treat as required input, not optional.
- Hidden starting order needs a UX input at game start; reinforce in Python/UI flows.

## Recommended next actions (A1 scope)
1) Fix Laboratory IV math and write tests that spend Yen/Imported + position surcharge + 5th-tech penalty.  
2) Implement movement/placement tech hooks (Stagecoach/Postal/Station/Newspaper/Telegram) with affordability checks.  
3) Finish Port/Lab/Card flows with hand limits, costs, and power-5 VP; add integration tests.  
4) Wire tech bonuses (Ball/LanguageSchool/Gaslight/Electrical/Mining/etc.) and Telephone recovery.  
5) Correct Country Bonus to distinct-country sets and finalize end-game trigger ordering (post-refill).  
6) Validate Church/Customs consumption and reward logic against kb tables; add tests for faith/import caps.  
7) Ensure setup validators cover area removal for 2p/3p, building sites, 5-Power tokens, and card counts.

