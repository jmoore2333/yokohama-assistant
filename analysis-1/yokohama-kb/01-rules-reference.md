# Yokohama Rules Reference

> Complete rules reference for the Yokohama board game AI assistant.
> Optimized for programmatic queries and game engine implementation.

## Game Overview

- **Players:** 2-4
- **Duration:** ~90 minutes
- **Theme:** Meiji-era Yokohama merchants competing for fame through trade
- **Information:** Nearly perfect. The only player-to-player secret is each player's starting Order card; the shuffled Order/Tech deck order and the boxed setup removals are hidden from everyone (symmetric uncertainty, no one's advantage)

---

## Components by Player Count

| Component | 2P | 3P | 4P |
|-----------|----|----|-----|
| Area Boards | 10 | 14 | 18 |
| Laboratories | 1 (A only) | 1 (A only) | 2 (A and B) |
| Ports | 1 (A only) | 2 (A and B) | 2 (A and B) |
| Order cards in deck | 20 | 31 | 36 |
| Church trigger | 4 assistants | 5 assistants | 6 assistants |
| Customs trigger | 4 assistants | 5 assistants | 6 assistants |

### Removed Components by Player Count

**2 Players remove:**
- Laboratory B, Port B, Chinatown, Canal
- 1× Copper Mine, 1× Silk Mill, 1× Tea Plantation, 1× Fishing Ground
- 16 Order cards from deck

**3 Players remove:**
- Laboratory B
- 1× Copper Mine, 1× Silk Mill, 1× Tea Plantation
- 5 Order cards from deck

**4 Players remove:** nothing.

### Board Layout — Area Boards (staggered pyramid)

The Area Boards form a **staggered pyramid**, widest row on top (rulebook p.3 step 2; owner-confirmed against the physical edition 2026-07-21). Rows, top to bottom:

| Players | Rows (tiles per row) | Tiles | Undirected edges |
|---------|----------------------|-------|------------------|
| 2P | 4 / 3 / 2 / 1 | 10 | 18 |
| 3P | 5 / 4 / 3 / 2 | 14 | 28 |
| 4P | 6 / 5 / 4 / 3 | 18 | 38 |

Rows are **brick-offset**: each tile touches up to **six** neighbors — 2 lateral (same row) + up to 2 in the row above + up to 2 in the row below, wherever those tiles exist. This is the adjacency graph that movement and pass-through fees read; it is NOT a rectangular grid. (An earlier engine model built 2×5 / 2×7 / 3×6 grids with orthogonal-only adjacency, which missed ~30% of the real edges and invented corner-only verticals — corrected 2026-07-21 [DL 2026-07-21].) Orientation is immaterial to the rules: the rulebook PDF and some editions print mirror-image orientations of the same pyramid, but the adjacency graph is identical, and it is the graph the engine models.

---

## Player Starting Resources

| Resource | Starting Player | Other Players |
|----------|-----------------|---------------|
| Money | 3 Yen | 4 Yen |
| President pawn | 1 (in hand) | 1 (in hand) |
| Assistant pawns in hand | 8 | 8 |
| Assistant pawns in warehouse | 12 | 12 |
| Shophouse tokens in hand | 2 | 2 |
| Shophouse tokens in warehouse | 6 | 6 |
| Trading House tokens in warehouse | 4 | 4 |
| Copper | 1 | 1 |
| Raw Silk | 1 | 1 |
| Tea | 1 | 1 |
| Seafood | 1 | 1 |
| Order cards | 1 (draw 2, keep 1) | 1 (draw 2, keep 1) |

---

## Turn Structure

Each turn consists of three phases executed in order:

### A. Additional Action Phase (First Half)
Optional actions, any order, repeatable (except Foreign Mercenaries):
- Use Foreign Mercenaries (once per turn total)
- Fulfill Achievement cards
- Fulfill Order cards

### B. Main Action Phase
Execute steps 1-6 in order:

1. **Placement Step** (required)
2. **Movement Step** (required)
3. **Area Action Step** (required if movement completed)
4. **5-Power Bonus Step** (optional, requires 5 Power action)
5. **Building Step** (optional, requires 4+ Power action)
6. **Recovery Step** (required if action completed)

### C. Additional Action Phase (Second Half)
Same options as Phase A.

---

## B1. Placement Step

Choose ONE option:

### Option A: Place 1 assistant in each of 3 different areas
- Select 3 different areas
- Place 1 assistant from hand into each

### Option B: Place 2 assistants in 1 area
- Select 1 area
- Place 2 assistants from hand into that area

### Placement Rules
- **President blocking payment:** If any President pawn(s) occupy the target area, pay 1 Yen to each President's owner per assistant placed
- **Canal restriction:** Cannot place assistants in the Canal
- **Minimum placement:** May place 0, 1, 2, or 3 assistants (not required to place all) — ruled legal explicitly 2026-07-18 (final whole-branch review finding I5) after this line sat unconnected to any decision-log row; see `guidingpath.md` decision log
- **Source restriction:** Must place from hand only (not from warehouse or management boards)

---

## B2. Movement Step

Choose ONE option:

### Option A: Move President Pawn

**If President is in hand:**
- Place directly into any Possible Action Area

**If President is on board:**
- Move step-by-step through adjacent areas
- Each step requires at least 1 of your assistants in the destination area
- Shophouses/Trading Houses do NOT count as assistants for movement
- May stop in any Possible Action Area
- Cannot stop in areas with other President pawns (must pay 1 Yen each and continue)
- Cannot return to starting area

**Possible Action Area definition:**
- Contains at least 1 of your assistant pawns AND
- Contains no other players' President pawns

### Option B: Return President to Hand
- Return President pawn to hand
- May return any number of assistant pawns from any areas to hand
- Cannot return assistants from management boards
- **Skip steps 3-6** and proceed to Phase C

---

## B3. Area Action Step

Execute the action of the area where your President pawn is located.

### Power Calculation

| Token Type | Power Value |
|------------|-------------|
| Your President pawn | 1 |
| Your Assistant pawn | 1 each |
| Your Shophouse token | 1 |
| Your Trading House token | 1 |
| Station token (if present) | 1 |

**Maximum effective Power: 5** (values 6+ are treated as 5)

### Trading House Income
If ANY opponent owns a Trading House in the action area, that opponent receives 1 Yen from the bank.
- Your own Trading House does NOT generate income for yourself

---

## B4. 5-Power Bonus Step (Optional)

**Requirement:** Executed a 5-Power action in step 3

**Effect:** If the 5-Power token is still present on the area:
1. Take the 5-Power token
2. Resolve its effect immediately
3. Flip token face-down and keep it

---

## B5. Building Step (Optional)

**Requirement:** Executed a 4+ Power action in step 3

Choose ONE option:

### Option A: Build Shophouse
- Must not already have a Shophouse on this area's Building Site card
- Place Shophouse from hand onto an empty Shophouse space
- Gain the reward shown on that space

### Option B: Build Trading House
- Trading House space must be empty
- Place Trading House from hand onto the Trading House space
- Gain the reward shown on that space

---

## B6. Recovery Step

Return ALL assistant pawns from the action area to your hand.

**Exception:** Technology card "Telephone" allows leaving 1 assistant behind.

---

## Area Types and Actions

### Production Areas (Factory icon)

#### Copper Mine
| Power | Reward |
|-------|--------|
| 1 | Nothing |
| 2 | Nothing |
| 3 | 1 Copper |
| 4 | 2 Copper |
| 5 | 3 Copper |

#### Silk Mill
| Power | Reward |
|-------|--------|
| 1 | Nothing |
| 2 | 1 Raw Silk |
| 3 | 2 Raw Silk |
| 4 | 3 Raw Silk |
| 5 | 4 Raw Silk |

#### Tea Plantation
| Power | Reward |
|-------|--------|
| 1 | 1 Tea |
| 2 | 2 Tea |
| 3 | 3 Tea |
| 4 | 4 Tea |
| 5 | 5 Tea |

#### Fishing Ground
| Power | Reward |
|-------|--------|
| 1 | 1 Seafood |
| 2 | 2 Seafood |
| 3 | 3 Seafood |
| 4 | 4 Seafood |
| 5 | 5 Seafood |

### Commercial Areas (Building icon)

#### Bank
| Power | Reward |
|-------|--------|
| 1 | 1 Yen |
| 2 | 2 Yen |
| 3 | 3 Yen |
| 4 | 4 Yen |
| 5 | 5 Yen |

#### Port (A and B)
Take Order cards from the corresponding Port Order Form Board.

**Layout (board-art verified):** each Port board has SIX card spaces, two per
zone, in order Low, Low, Mid, Mid, High, High (left to right).

| Power | Available Zones |
|-------|----------------|
| 1 | Cannot take |
| 2 | Low only (2 spaces) |
| 3 | Low, Mid (4 spaces) |
| 4 | Low, Mid, High (all 6 spaces) |
| 5 | Low, Mid, High + 3 VP |

**Card options:**
- Take 1 card: Free
- Take 2 cards: Pay 2 Yen OR 1 Imported Good

**Hand limit:** Maximum 3 Order cards. Cannot take if would exceed.

**Replenishment:** Slide cards left (full compaction across all 6 spaces), refill from deck to rightmost spaces.

#### Laboratory (A and B)
Acquire Technology cards from the corresponding Technology Card Placement Area.

**Ownership limit (added 2026-07-17 — omitted from this reference until the final whole-branch review found the engine had no ownership check at all):** Players may only own 1 Technology card of the same name at a time (rulebook General Rules: "There is no limit to the number of Technology cards in each player's hand. However, players may only own 1 card of the same name."). Ten of the 20 titles circulate as 2 physical copies each (Language School, Telegram, Telephone, Newspaper, Exposition, Ball, Gaslight, University, Letterpress Printing, Postal System) precisely so different players can each own one — but a single player may never acquire both. See `guidingpath.md` decision log.

**Layout (board-art verified):** each Laboratory board has SIX tech slots.
Slots 1-2 carry no surcharge; +1 on the 3rd and 4th spaces, +2 on the 5th and 6th.

**Industry Value required** = Card's printed cost + Board position modifier (+1 on the 3rd and 4th spaces, +2 on the 5th and 6th)

**Industry Value sources:**
| Source | Industry Value |
|--------|----------------|
| 1 Power | 1 |
| 2 Power | 2 |
| 3 Power | 3 |
| 4 Power | 4 |
| 5 Power | 5 (max from Power) |
| 2 Yen | 1 |
| 1 Imported Good | 1 |

**Extra-Technology surcharge (rulebook p.14 verbatim):** "Starting with your 4th card, each technology card requires an additional cost of 2 yen or 1 import" — EVERY card from the 4th onward pays it.

**Correction (owner-ruled 2026-08-08, D2):** this line previously read "Each Technology card after your 4th costs an additional 2 Yen OR 1 Imported Good" — a MISREADING that started the surcharge at the 5th card. The verbatim p.14 sentence above is the rule: the 4th card itself already pays. See `guidingpath.md` decision log 2026-08-08.

**Replenishment:** Slide cards left, refill from deck to rightmost space.

**Country icon matching:** After acquiring, check for Foreign Mercenary eligibility.

#### Employment Agency
Take tokens from your Warehouse tile to your hand.

| Power | Tokens Available |
|-------|------------------|
| 1 | 0 |
| 2 | 1 |
| 3 | 2 |
| 4 | 3 |
| 5 | 4 |

**Costs:**
| Token Type | Cost |
|------------|------|
| Assistant pawn | Free |
| Shophouse | 2 Yen each |
| Trading House (1st) | 4 Yen |
| Trading House (2nd) | 5 Yen |
| Trading House (3rd) | 6 Yen |
| Trading House (4th) | 7 Yen |

#### Chinatown (not in 2P)
Exchange resources. Number of exchanges = Power level.

**Exchange rates:**
| Pay | Receive |
|-----|---------|
| 1 Copper | 2 Yen |
| 1 Raw Silk | 2 Yen |
| 1 Tea | 1 Yen |
| 1 Seafood | 1 Yen |
| 2 Yen | 1 Copper |
| 2 Yen | 1 Raw Silk |
| 1 Yen | 1 Tea |
| 1 Yen | 1 Seafood |
| 4 Yen | 1 Imported Good |

**Note:** Cannot sell Imported Goods for money.

#### Church
Contribute Faith value to advance on Church Management Board.

**Faith Value sources:**
| Source | Faith Value | Restriction |
|--------|-------------|-------------|
| 1 Power | 1 | - |
| 2 Power | 2 | - |
| 3 Power | 3 | - |
| 4 Power | 4 | - |
| 5 Power | 5 | - |
| 1 Copper | 1 | Max 1 per turn |
| 1 Raw Silk | 1 | Max 1 per turn |
| 1 Tea | 1 | Max 1 per turn |
| 1 Seafood | 1 | Max 1 per turn |
| 2 Yen | 1 | Max 1 per turn |
| 1 Imported Good | 1 | Max 1 per turn |

**Correction (Jeremy-adjudicated 2026-07-17):** Faith from Power scales linearly (1,2,3,4,5). An earlier version of this table read "2 Power: 1 Faith" — that is a rulebook text typo, not the actual rule; see `guidingpath.md` decision log.

**Correction (owner-ruled 2026-08-08, D1):** the Yen payment type costs **2 Yen** for its 1 Faith — the printed church board shows 2 yen (rulebook p.12), confirmed on Jeremy's physical set. This table previously read "1 Yen | 1"; only the COST was wrong, the Faith contribution was always 1. See `guidingpath.md` decision log 2026-08-08.

**Process:**
1. Calculate total Faith value
2. Place 1 assistant on Church Management Board at corresponding space
3. Gain reward shown on that space
4. Assistant is locked permanently

**WHERE THE TRACK PAWN COMES FROM — a POTENTIAL RULEBOOK DIFF (owner-ruled 2026-08-08, D6, `SEMANTICS_REV` 7):** rulebook p.13 reads "place 1 Assistant pawn from the **Church Area or your hand**" — i.e. the printed rule offers the player a CHOICE of source. **BGA appears not to offer that choice: it takes the pawn from the activated Area** (live gate game table 896157897 m49 — the pawn moved Church-area → track with the player's hand untouched). The owner ruled for BGA parity, so this engine is **AREA-FIRST**: with own assistant(s) in the activated Church Area the pawn comes from the AREA, otherwise from hand. This is a deliberate divergence from the printed text, recorded here rather than papered over — the printed rule is the "Area or hand" choice above, the engine implements BGA's behaviour. Rationale (owner): the area pawn returns to hand at Recovery anyway and a movement icon can move a pawn from hand or from that tile, so the player is not worse off; parity buys simplicity and shrinks the 3.5c+ problem space. See `guidingpath.md` decision log 2026-08-08 for the full row.

**Church Management Board rewards** (Bonus column corrected 2026-07-17 from board art — every space carries a movement rider, values 1,1,2,2,2,2,3,3; see `guidingpath.md` decision log):
| Faith | VP | Bonus |
|-------|-----|-------|
| 4 | 3 | 1 Movement |
| 5 | 4 | 1 Movement |
| 6 | 5 | 2 Movement |
| 7 | 6 | 2 Movement |
| 8 | 7 | 2 Movement |
| 9 | 8 | 2 Movement |
| 10 | 11 | 3 Movement |
| 11 | 14 | 3 Movement |

#### Customs
Pay Imported Goods to advance on Customs Management Board.

| Power | Max Imported Goods payable |
|-------|---------------------------|
| 1 | 0 |
| 2 | 1 |
| 3 | 2 |
| 4 | 3 |
| 5 | 4 |

**Process:**
1. Pay Imported Goods (up to Power-based limit)
2. Place 1 assistant on Customs Management Board at corresponding space
3. Gain reward shown on that space
4. Assistant is locked permanently

**WHERE THE TRACK PAWN COMES FROM — the same POTENTIAL RULEBOOK DIFF as the Church (owner-ruled 2026-08-08, D6, `SEMANTICS_REV` 7):** rulebook p.14 reads "place 1 Assistant pawn from the **Customs Area or your hand**", a player CHOICE of source; the engine is **AREA-FIRST** (own assistant(s) in the activated Customs Area → the pawn comes from the AREA, otherwise from hand) to match BGA. See the Church section above for the evidence and the owner's reasoning, and `guidingpath.md` decision log 2026-08-08 for the full row.

**Customs Management Board rewards (board-art verified 2026-07-17 — CORRECTS the table below, which previously modeled a goods-keyed 8-slot track with 1/3/4-Imported-Good bonuses that don't exist on the physical board):**

7 printed spaces, numbered 1-7 left to right, each with a fixed printed cost (Imported Goods) and VP reward. A payment must EXACTLY match a space's printed cost — both spaces of a cost pair filled means that payment is unplaceable. The PLAYER chooses the space when both of a pair are empty (a queued `CustomsSpace` choice; exactly one eligible places inline) — the old "v1 auto-places on the higher-VP (leftmost) empty space" sentence described pre-choice-encoding engine behaviour and was corrected forward 2026-08-20 (six-lens mechanics verification; the Church space choice works the same way via `ChurchSpace`). In 2-player games, space 4 (9 VP) is occupied for the whole game by the dummy player's assistant — a pawn of an UNUSED PLAYER COLOR that setup places there (finding D1). It is **not printed board art**: at a physical table the operator must place it during setup, and nothing later reconciles a board that is missing it. Same for the Church board's faith-7. Being a placed assistant, it also competes in the final majority bonus (see Final Scoring).

| Space | Cost (Imported Goods) | VP | Rider |
|-------|------------------------|-----|-------|
| 1 | 1 | 5 | +2 Warehouse takes |
| 2 | 1 | 4 | +2 Yen |
| 3 | 2 | 10 | +1 Warehouse take |
| 4 | 2 | 9 | +1 Yen |
| 5 | 3 | 16 | - |
| 6 | 3 | 15 | - |
| 7 | 4 | 22 | - |

#### Canal (not in 2P)
- No pawns or tokens may be placed; the Canal is **unplaceable and unstoppable** (transit-only). Removed from the board entirely at 2 players.
- **Movement transit is ANY-TO-ANY** [DL 2026-07-21]: the Canal connects ANY of its adjacent tiles to ANY other, for **1 Yen per crossing paid to the BANK** (not to a player). This supersedes an earlier colinear-through-lines model, which had been reasoned on the old rectangular grid.
- Where two of the Canal's neighbors are ALSO directly adjacent to each other (a brick-offset edge), the free direct step governs: crossing the Canal between them is legal but strictly dominated — it reaches the same destination under the same conditions (you still need one of your own assistants there) and costs 1 Yen more, so a player never has reason to pay the toll on such a pair. The rule to derive from this: **every pair of Canal-adjacent tiles is connected for President movement** — free where the two tiles physically touch, otherwise for the 1-Yen toll.
- Placement adjacency (e.g. Telegram's adjacent-bonus) remains physical tile contiguity and EXCLUDES crossing pairs — the Canal is for transit, not placement.

---

## Foreign Mercenaries

### Acquiring Foreign Mercenaries
Every time you accumulate 2, 4, 6... of the same Country icon across completed Order cards and Technology cards, receive 1 Foreign Mercenary of that country (if available).

**Countries:** English, American, French, German, Dutch

**Supply:** English (3), American (2), French (2), German (2), Dutch (1)

### Using Foreign Mercenaries (Additional Action Phase)
**Limit:** Once per turn (across both Additional Action phases)

**Requirements for target area:**
- Contains at least 1 of your assistant pawns
- Does NOT contain your President pawn
- MAY contain opponent President pawns (no payment required)

**Effect:** Execute steps 3-6 of Main Action Phase for target area:
- 3. Area Action
- 4. 5-Power Bonus
- 5. Building
- 6. Recovery

Flip the Foreign Mercenary face-down after use.

---

## Order Cards

### Structure
- Country icon (for set collection)
- Required Trade Goods to complete
- Rewards (VP, money, goods, etc.)

### Fulfilling Orders (Additional Action Phase)
1. Place Order card face-up in front of you
2. Pay required Trade Goods to supply
3. Gain rewards shown on card
4. Check for Foreign Mercenary eligibility (Country icons)

### Hand Limit
- Maximum 3 Order cards
- Cannot discard Order cards

---

## Achievement Cards

Three categories (A, B, C), one of each in play per game.

### Category A: Trade Good Holdings
| Card | Requirement | 1st Place | 2nd+ Place |
|------|-------------|-----------|------------|
| Copper | Own 5 Copper | 8 VP | 6 VP |
| Raw Silk | Own 6 Raw Silk | 8 VP | 6 VP |
| Tea | Own 7 Tea | 8 VP | 6 VP |
| Seafood | Own 7 Seafood | 8 VP | 6 VP |

**Note:** Do NOT pay goods, just demonstrate ownership.

### Category B: Collection Goals
| Card | Requirement | 1st Place | 2nd+ Place |
|------|-------------|-----------|------------|
| Money | Own 10 Yen | 9 VP | 7 VP |
| Tokens | Own 5 total (5-Power + Foreign Mercenary) | 9 VP | 7 VP |
| Orders | Complete 5 Order cards | 9 VP | 7 VP |
| Technology | Own 4 Technology cards | 9 VP | 7 VP |

### Category C: Building Presence
| Card | Requirement | 1st Place | 2nd+ Place |
|------|-------------|-----------|------------|
| Production | Buildings in 4 Production areas | 10 VP | 8 VP |
| Commercial | Buildings in 4 Commercial areas | 10 VP | 8 VP |
| Mixed 3+2 | Buildings in 3 Production + 2 Commercial | 10 VP | 8 VP |
| Mixed 2+3 | Buildings in 2 Production + 3 Commercial | 10 VP | 8 VP |

**Note:** Multiple buildings in same area count as 1 for Achievement purposes.

### Fulfilling Achievements (Additional Action Phase)
1. Demonstrate requirement is met
2. Place assistant from hand onto Achievement card
3. Score VP (1st placer gets left value, subsequent get right value)
4. Each player may fulfill each Achievement only once

---

## End Game Triggers

Game end is triggered when ANY of these conditions occur:

1. **Trading Houses:** Any player has built all 4 Trading Houses
2. **Shophouses:** Any player has built all 8 Shophouses
3. **Order Deck:** After replenishing, Order deck is empty AND a space on Port Order Form Board is empty
4. **Church Track:** Required number of assistants placed (2P: 4, 3P: 5, 4P: 6)
5. **Customs Track:** Required number of assistants placed (2P: 4, 3P: 5, 4P: 6)

### End Game Procedure
1. Complete the current round (until player before Start Player finishes)
2. Play one final complete round (all players take 1 turn)
3. Proceed to Final Scoring

---

## Final Scoring

Calculate in this order:

### 1. Church Bonus
Compare assistant count on Church Management Board:
- 1st place: 6 VP
- 2nd place: 3 VP
- Ties: Rightmost assistant wins
- **2-player games (finding D1):** the dummy player's pawn — a setup-placed assistant of an unused color at faith-7 — COMPETES in this majority [DL 2026-07-21]. Rulebook p.9 NOTE (**Jeremy's edition**; the OKAZU Brand English rulebook does NOT carry this NOTE — verbatim quote and provenance in `docs/plans/2026-07-20-phase-2.9-ruling-list.md` §D1): "The dummy player's assistants are taken into account during final scoring to increase competition for majority bonuses." **It sits MID-track, not at the high end:** faith-7 is index 3 of the eight spaces (faith 4–11), leaving four spaces (faith 8–11) to its right. So under the rightmost tie-break it beats a tied real pawn on faith 4–6 and LOSES to a tied real pawn on faith 8–11; holding one pawn, it takes a place only on a tie it wins, and any two real pawns out-count it outright. (Earlier the engine treated it as an eligibility block only and scored over real players — corrected 2026-07-21.)

### 2. Customs Bonus
Compare assistant count on Customs Management Board:
- 1st place: 8 VP
- 2nd place: 4 VP
- Ties: Rightmost assistant wins
- **2-player games (finding D1):** the dummy player's pawn — setup-placed at space-4 — COMPETES in this majority [DL 2026-07-21], by the same rulebook p.9 NOTE (**Jeremy's edition**, not the OKAZU Brand English rulebook) as the Church bonus. It is mid-track here too: space-4 is index 3 of the seven spaces, leaving spaces 5–7 to its right, so it wins a tie against a real pawn on spaces 1–3 and loses one against spaces 5–7.

### 3. Technology Bonus
Compare total Industry value on Technology cards:
- 1st place: 10 VP
- 2nd place: 5 VP
- Ties: Closest to Start Player (clockwise) wins
- **2-player games (owner-ruled 2026-08-08, D5):** the dummy player competes in THIS majority too, at an industry value of **exactly 7.5** — rulebook p.9 final-scoring NOTE (**Jeremy's edition**, same NOTE as the church/customs competition): "the dummy player ends the game with an industry value of 7.5". Observed on BGA: owner 1st (10 VP), the dummy's 7.5 beat the opponent for 2nd, which was awarded to nobody. The half is exact — an IV-7 player loses to the dummy, an IV-8 player beats it; it can never tie a real player (whole-number totals). A place the dummy takes is consumed and awarded to nobody, exactly as on the church/customs tracks. See `guidingpath.md` decision log 2026-08-08.

### 4. Country Bonus
Form sets of different Country icons from completed Orders and Technologies:

| Set Size | VP |
|----------|-----|
| 5 different | 12 |
| 4 different | 8 |
| 3 different | 4 |
| 2 different | 2 |
| 1 | 0 |

Each icon may only be used in one set. Optimal: form largest sets first.

### 5. Remainder Scoring
| Item | VP |
|------|-----|
| Unused Foreign Mercenary (face-up) | 1 each |
| Imported Good token | 1 each |
| Every 2 Yen | 1 |
| Every 3 Trade Goods (any combination) | 1 |

---

## Winning

Highest VP wins. Tiebreaker: Closest to Start Player (clockwise).

---

## Technology Cards Reference

### Movement Technologies

**Tram** (Industry 5)
- Movement Step: Move President to any area without another President
- If used, cannot move again this turn

**Stagecoach** (Industry 3)
- Movement Step: Pass through areas with other Presidents without paying
- Still cannot stop in areas with other Presidents (unless have Postal System)

**Postal System** (Industry ?)
- May end movement in area with another President and take action
- Still must pay when passing through (unless have Stagecoach)

**Station** (Industry 4)
- On acquire: Place Station token in any area
- Station provides +1 Power to ALL players in that area
- You may move directly to Station area for free
- Cannot move there unless taking that area's action

### Building Technologies

**Gaslight** (Industry 3)
- When building Shophouse: +2 VP
- Only applies from turn acquired onward

**Electrical Light** (Industry ?)
- When building Trading House: +3 VP
- Only applies from turn acquired onward

### Production Technologies

**Mining Technology** (Industry 3)
- When receiving Copper: +1 additional Copper (per gain event, not per Copper)
- Applies to EVERY Copper receipt — verified card text (08-tech-cards.yaml): "Whenever you gain one or more Copper: gain 1 additional Copper". CORRECTED 2026-07-18: this line previously enumerated only "Copper Mine, Chinatown, 5-Power tokens, Building Site rewards" — an incomplete derivation; Order-card copper rewards and any-good icons resolved to Copper also qualify (decision log 2026-07-18)

**Spinning Mill** (Industry 4)
- Once per turn: Discard 1 Raw Silk → Gain 2 Yen
- May use on Raw Silk gained this turn

### Order/Tech Acquisition Technologies

**Ball** (Industry 3)
- When completing Order card: +2 VP

**Language School** (Industry ?)
- When completing Order card: +1 Yen

**Patent System** (Industry 3)
- When acquiring Technology card (after this one): +2 VP

**Letterpress Printing** (Industry ?)
- When acquiring Technology card (after this one): +1 Yen

### Track Technologies

**Winery** (Industry ?)
- When placing assistant on Church: +3 VP

**Brickyard** (Industry 3)
- When placing assistant on Customs: +4 VP

### Economy Technologies

**System of Stock** (Industry ?)
- Turn start: If you have ≤1 Yen, refill to 2 Yen

**University** (Industry ?)
- Once per turn: Pay 1 Yen → Take assistant from Warehouse to hand

### Placement Technologies

**Newspaper** (Industry ?)
- Placement Step Option A: May place in 4 different areas instead of 3

**Telegram** (Industry ?)
- Placement Step Option B: After placing 2 in one area, may place 1 more in adjacent area
- Pay President fee if applicable to adjacent area

**Telephone** (Industry 6)
- Recovery Step: May leave 1 assistant in the action area

### Instant Technologies

**Exposition** (Industry 5)
- Immediately gain 10 VP

---

## 5-Power Tokens

Each area has one 5-Power token placed **face-up** during setup (rulebook p.3 step 5: the tokens are *shuffled* face-down, then placed **face-up** on each Area Board's 5-space), so its effect is public information from the start. A player gains the effect when they take the token at 5 Power, then keeps the token face-down. Effects vary.

The complete physical pool (box-verified by Jeremy 2026-07-18, decision log "BOX CHECK (a)";
CORRECTED here 2026-07-20 — this section previously listed "Victory Points" and "Movement
abilities" faces, which DO NOT EXIST):
- 10× gain 1 Imported Good
- 2× gain 2 Copper · 2× gain 2 Silk
- 2× gain 3 Seafood · 2× gain 3 Tea
- 2× gain 3 Yen

(The engine's `FivePowerEffect` enum matches this pool exactly; the movement ICON on order
cards/site spaces is a separate, real mechanic — see the icon glossary.)

---

## Key Strategic Rules Clarifications

### Assistant Economy
- **Hand:** Available for placement and paying to Achievement/Church/Customs
- **Warehouse:** Must use Employment Agency to retrieve
- **Board:** On areas, returned to hand during Recovery
- **Locked:** On Management Boards (Church/Customs/Achievement), never returned

### Movement Icon (rulebook p.10) — the to-hand return
- The Movement icon reads "Move an Assistant pawn from your hand or from an Area to another Area", one move per icon, no President fees.
- **Owner-ruled 2026-08-08 (D3, BGA parity):** the icon may ALSO return one of your on-board assistants to your hand, where it simply stays. The printed text is AMBIGUOUS on this (and there are disagreements online); BGA's engine allows it, and the owner ruled to include it for parity with BGA while noting he generally believes the original area-to-area reading is accurate. Hand-to-hand is not a move (it would change nothing — decline instead). See `guidingpath.md` decision log 2026-08-08 for the verbatim rationale.

### Building Sites
- Each area has one Building Site card (randomly assigned at setup)
- 4 Shophouse spaces (varying VP rewards)
- 1 Trading House space (varying VP reward)
- Building in area requires 4+ Power action in that area

### Order Card Visibility
- Order cards in hand are hidden
- Completed Order cards are face-up (visible)
- When ANY player takes an Order card, ALL players see which specific card was taken

### Technology Card Visibility
- All Technology cards are public information
- Cards in Labs are visible
- Players' acquired Technologies are visible
