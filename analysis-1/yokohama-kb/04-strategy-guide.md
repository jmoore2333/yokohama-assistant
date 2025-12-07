# Yokohama Strategic Evaluation Guide

> Strategic insights for heuristic evaluation and MCTS guidance.

---

## Game Phases

### Early Game (Turns 1-5)
**Priority:** Engine building, assistant expansion
- Acquire key technologies (Ball, Gaslight, Mining Tech)
- Build assistants from warehouse to hand
- Establish board presence with assistant placements
- Take synergistic Order cards

### Mid Game (Turns 6-12)
**Priority:** VP accumulation, track positioning
- Execute building strategy (Shophouse/Trading House)
- Complete Order cards
- Position on Church/Customs tracks
- Collect Country icon sets

### Late Game (Turns 13+)
**Priority:** End game trigger timing, final scoring optimization
- Control end game timing (accelerate if ahead, delay if behind)
- Maximize Country set bonuses
- Complete track majorities
- Convert resources to VP

---

## Scoring Sources Analysis

### Direct VP Sources

| Source | VP Range | Timing | Notes |
|--------|----------|--------|-------|
| Shophouse building | 1-8 per | Ongoing | Building Site dependent |
| Trading House building | 2-9 per | Ongoing | Building Site dependent |
| Order completion | 2-8 per | Ongoing | Plus tech bonuses |
| Church track | 3-14 per | Ongoing | Plus end-game majority |
| Customs track | 4-22 per | Ongoing | Plus end-game majority |
| Achievement cards | 6-10 per | Ongoing | First vs subsequent |
| 5-Power tokens | Variable | Ongoing | Some grant VP directly |
| Technologies | 0-10 | One-time | Exposition only |

### End Game Scoring

| Category | 1st Place | 2nd Place | Key Factor |
|----------|-----------|-----------|------------|
| Church majority | 6 VP | 3 VP | Assistant count, rightmost tiebreak |
| Customs majority | 8 VP | 4 VP | Assistant count, rightmost tiebreak |
| Technology race | 10 VP | 5 VP | Total Industry Value |
| Country sets | Up to 36+ VP | - | Icon diversity |
| Remainders | ~5-15 VP | - | Efficient conversion |

---

## Technology Synergies

### Tier 1: High Impact Combos

**Ball + Order Focus**
- +2 VP per order completion
- Synergizes with: Language School (+1 Yen per order)
- Strategy: Prioritize Port actions, complete many orders

**Gaslight + Shophouse Rush**
- +2 VP per shophouse
- Max potential: +16 VP (8 shophouses)
- Triggers end game, provides strong VP engine

**Mining Technology + Copper Orders**
- +1 Copper on every copper gain
- Copper is hardest resource to produce
- Makes copper-heavy orders feasible

### Tier 2: Strong Synergies

**Station + High-Traffic Area**
- +1 Power for all players BUT owner moves there free
- Best placement: High-value Commercial area (Lab, Church, Customs)
- Creates reliable action point

**Postal System + Stagecoach**
- Move through AND stop in President-occupied areas without paying
- Maximum movement flexibility
- Strong late game when board is contested

**Winery + Church Focus**
- +3 VP per Church action
- Church already grants VP + bonuses
- Combined with high Faith values = huge VP per action

**Brickyard + Customs Focus**
- +4 VP per Customs action
- Customs pays Imported Goods for VP
- Imported Goods more flexible to acquire

### Tier 3: Utility Technologies

**Telephone + Area Control**
- Leave 1 assistant after Recovery
- Maintains board presence
- Good for contested Production areas

**Newspaper + Placement Efficiency**
- 4 areas instead of 3 in Option A
- +33% placement efficiency
- Strong early game

**University + Assistant Economy**
- Pay 1 Yen for Warehouse assistant
- Bypasses Employment Agency
- Smooth action economy

---

## Track Strategies

### Church Track

**Optimal Positions:**
| Faith | VP | Bonus | Priority |
|-------|-----|-------|----------|
| 6 | 5 | 2 Movement | HIGH - movement valuable |
| 9 | 8 | 1 Imported | HIGH - imported goods flexible |
| 11 | 14 | None | MEDIUM - high VP but expensive |

**Faith Contribution Limits:**
- Max 1 of each: Copper, Silk, Tea, Seafood, Yen, Imported Good
- Most efficient: Low-power action + diverse goods
- Don't overpay for Faith 11 unless necessary

**Majority Considerations:**
- 6 VP first place, 3 VP second
- Rightmost assistant breaks ties
- Going last to a track level = guaranteed tiebreak win

### Customs Track

**Optimal Positions:**
| Goods | VP | Bonus | Priority |
|-------|-----|-------|----------|
| 3 | 10 | 1 Imported | HIGH - self-sustaining |
| 5 | 16 | None | MEDIUM - high VP, no bonus |
| 7+ | 22 | None | LOW - expensive, diminishing returns |

**Imported Goods Economy:**
- Sources: 5-Power tokens, Order rewards, Chinatown (4 Yen), Church bonus
- Chinatown conversion: 4 Yen → 1 Imported Good
- Plan Customs pushes around Imported Good availability

**Majority Considerations:**
- 8 VP first place, 4 VP second
- More competitive than Church (higher stakes)
- Monitor opponent Imported Good holdings

---

## Building Strategies

### Shophouse Rush
- Goal: Build 8 shophouses quickly
- Triggers end game
- Best with Gaslight (+2 VP each)
- Requires Employment Agency to cycle tokens

**Constraints:**
- Start with 2 in hand, 6 in warehouse
- Employment Agency costs: 2 Yen each from warehouse
- Need 4+ Power actions to build

### Trading House Income
- Trading Houses generate 1 Yen when opponents act in that area
- Best in high-traffic areas: Bank, Port, Lab
- Passive income compounds over game

**Priority Areas:**
1. Laboratory - everyone needs Technologies
2. Port - everyone needs Orders
3. Production areas near center

### Balanced Building
- Achievement Card C requires buildings in multiple area types
- 4 Production + 0 Commercial OR 3+2 OR 2+3 etc.
- Plan building locations around Achievement

---

## Assistant Economy

### Location States

| Location | Availability | Recovery Method |
|----------|--------------|-----------------|
| Hand | Immediate | - |
| Warehouse | Delayed | Employment Agency |
| Board | Locked until Recovery | Area Action |
| Management Boards | Permanent | Never |
| Achievement Cards | Permanent | Never |

### Efficiency Guidelines

**Hand Size Targets:**
- Minimum 4-5 for placement flexibility
- Maximum useful: 8-10 (more is wasteful)

**Warehouse Management:**
- Don't let warehouse empty completely
- Employment Agency can pull Shophouses/Trading Houses too
- Plan ahead for building tokens

**Locking Assessment:**
- Church/Customs assistants are permanent investment
- Achievement assistants are permanent investment
- Calculate: Is the VP worth the lost flexibility?

---

## Order Card Strategy

### Evaluation Factors

1. **Resource Match:** Do you already have these goods?
2. **Production Efficiency:** Can you produce remaining goods efficiently?
3. **Country Icon:** Does it help your set collection?
4. **Reward Quality:** VP, money, movement all valuable

### Hidden Information Exploitation

**Your advantage:**
- You see which Orders opponents take from Port
- You know 2/3 of their hand (except starting card)
- Infer their production targets

**Example inference:**
- Opponent takes copper-heavy Order
- They'll need Copper Mine actions
- Block or compete for Copper Mine access

### Hand Management

- Maximum 3 Orders in hand
- Cannot discard
- Take Orders you can complete
- Don't lock hand with unfulfillable Orders

---

## Country Set Optimization

### Icon Sources
- Completed Order cards (1 icon each)
- Technology cards (1 icon each)

### Set Values
| Set Size | VP | VP per Icon |
|----------|-----|-------------|
| 5 different | 12 | 2.4 |
| 4 different | 8 | 2.0 |
| 3 different | 4 | 1.33 |
| 2 different | 2 | 1.0 |

### Optimization Strategy

**Target:** Multiple 5-sets > fewer incomplete sets

**Example:**
- 5+5+3 icons (13 icons) = 12+12+4 = 28 VP
- 4+4+5 icons (13 icons) = 8+8+12 = 28 VP
- 5+4+4 icons (13 icons) = 12+8+8 = 28 VP
- 3+3+3+3+1 icons (13 icons) = 4+4+4+4+0 = 16 VP

**Key insight:** Completing 5-sets is always optimal, avoid leftover singles.

---

## End Game Timing

### Acceleration Triggers
Use when ahead to lock in advantage:
- Build 4th Trading House
- Build 8th Shophouse
- Push track to trigger count
- Take last Order cards

### Deceleration Tactics
Use when behind to catch up:
- Avoid track spaces that trigger
- Don't rush buildings
- Let opponents deplete Order deck
- Focus on point-dense actions

### Trigger Monitoring

| Trigger | Monitor | Control Level |
|---------|---------|---------------|
| Trading Houses | Opponent building count | Moderate |
| Shophouses | Opponent building count | Moderate |
| Order Deck | Cards remaining, take rate | Low |
| Church Track | Assistants placed | High |
| Customs Track | Assistants placed | High |

---

## Positional Evaluation Factors

### Board Topology

**Adjacency Value:**
- Central areas: More accessible, higher traffic
- Edge areas: Fewer connections, easier to control

**Canal Impact:**
- 1 Yen per crossing
- Significant cost over many turns
- Station placement can bypass

### President Blocking

**Blocking Value:**
- Opponent must pay 1 Yen to pass
- Cannot stop in your President's area
- High value in central/contested areas

**Counter-strategies:**
- Stagecoach (free passage)
- Postal System (can stop anyway)
- Return President to hand (mobility reset)

### Trading House Placement

**Income Projection:**
- Estimate opponent actions in area
- Lab/Port: High traffic = steady income
- Production: Traffic depends on game state

---

## Resource Valuation

### Base Values (relative)
| Resource | Value | Reasoning |
|----------|-------|-----------|
| Copper | 2.0 | Hardest to produce (min 3 Power) |
| Raw Silk | 1.5 | Medium difficulty (min 2 Power) |
| Tea | 1.0 | Easiest (1 Power = 1 Tea) |
| Seafood | 1.0 | Easiest (1 Power = 1 Seafood) |
| Imported Good | 3.0 | Versatile: Lab, Church, Customs, Port |
| Yen | 0.7 | Flexible but less efficient than goods |

### Context Multipliers

| Context | Multiplier | Example |
|---------|------------|---------|
| Toward held Order | 1.5x | Copper for copper Order |
| Excess (no use) | 0.5x | Extra Tea with no Tea orders |
| Blocked production | 2.0x | Copper when Copper Mine contested |
| End game conversion | 0.33x | 3 goods = 1 VP |

---

## Common Mistakes to Avoid

### Early Game
- Over-investing in track spaces early (locks assistants)
- Taking Orders you can't complete (hand lock)
- Ignoring Technology cards (engine missing)

### Mid Game
- Spreading too thin across areas (power dilution)
- Neglecting assistant economy (run out of placements)
- Missing Country set planning (incomplete sets)

### Late Game
- Triggering end game while behind
- Leaving resources unconverted (VP loss)
- Missing final turn optimization
