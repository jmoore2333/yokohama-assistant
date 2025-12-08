# A1 Data Collection Plan

**Created:** 2025-12-07
**Updated:** 2025-12-07
**Purpose:** Document the missing game data needed to complete A1 game engine
**Status:** IN PROGRESS - Extracting from Tabletop Simulator

---

## Progress Summary

| Component | Status | Notes |
|-----------|--------|-------|
| Board Generation Logic | ✅ DONE | `game/board_gen.rs` - 11 tests |
| Order Cards (36) | ✅ EXTRACTED | `tts-assets/organized/cards/order_cards_8x5.jpg` |
| Tech Cards (20) | ✅ EXTRACTED | `tts-assets/organized/cards/tech_cards_8x4.jpg` |
| Building Site Cards (24) | ✅ EXTRACTED | `tts-assets/organized/cards/building_site_cards_5x5.jpg` |
| Area Tiles | ✅ VERIFIED | 10/14/18 for 2P/3P/4P confirmed |
| 5-Power Tokens | ⚠️ INCOMPLETE | Only 8 found in TTS, need 17 for 4P |
| Data Transcription | ⏳ PENDING | Need to convert images to YAML/Rust |

---

## Overview

The A1 game engine has complete **logic** but is missing **concrete game data**. This document outlines what data is needed, where to source it, and the recommended format for documenting each component.

---

## Data Categories

### 1. Order Cards (~36 cards)

**What we need for each card:**
- Unique identifier (for CV recognition)
- Country icon (English/American/French/German/Dutch)
- Requirements: Copper, Silk, Tea, Seafood quantities
- Rewards: VP, Yen, Imported Goods, Trade Goods, Movement, etc.

**Source options:**
- [ ] Physical cards (photograph each)
- [ ] BoardGameGeek images/files
- [ ] Fan-made card lists online
- [ ] Manual transcription from physical game

**Recommended format:**
```yaml
order_cards:
  - id: "order_001"
    country: "english"
    requirements:
      copper: 2
      silk: 1
      tea: 0
      seafood: 0
    rewards:
      victory_points: 5
      money: 2
      imported_goods: 0
      # ... other rewards
```

**Questions to resolve:**
- [ ] Are card backs distinguishable? (affects deck composition tracking)
- [ ] Any special/variant cards in expansions we should exclude?

---

### 2. Technology Cards (20 cards)

**What we already have:**
- All 20 effects defined in `TechEffect` enum
- Base industry costs (some marked "?" - need verification)

**What we still need:**
- Country icon for each card
- Verification of industry costs
- Card names (for CV recognition)

**Source:** Rulebook has effects; physical cards needed for country icons

**Recommended format:**
```yaml
tech_cards:
  - id: "tech_tram"
    name: "Tram"
    effect: "Tram"  # matches TechEffect enum
    country: "german"
    industry_cost: 5

  - id: "tech_stagecoach"
    name: "Stagecoach"
    effect: "Stagecoach"
    country: "english"
    industry_cost: 3
```

**Unknown industry costs to verify:**
- PostalSystem (?)
- ElectricalLight (?)
- LanguageSchool (?)
- LetterPressPrinting (?)
- Winery (?)
- SystemOfStock (?)
- University (?)
- Newspaper (?)
- Telegram (?)

---

### 3. Building Site Cards

**What we need:**
- Total count of unique building site cards
- For each card:
  - Shophouse VP rewards (slots 1-4)
  - Trading House VP reward

**Important:** These are randomized during setup - we need the **complete set** of all possible cards, not a single game's layout.

**Recommended format:**
```yaml
building_sites:
  - id: "site_001"
    shophouse_rewards: [2, 3, 4, 5]  # slots 1-4
    trading_house_reward: 7

  - id: "site_002"
    shophouse_rewards: [1, 2, 3, 4]
    trading_house_reward: 6
```

**Questions to resolve:**
- [ ] How many unique building site cards exist?
- [ ] Are they all identical on the back? (for randomization)
- [ ] Any correlation between site card and area type?

---

### 4. 5-Power Tokens

**What we need:**
- Complete list of all token types
- Count of each type in the game
- Effect description for each

**Current enum (needs verification):**
```rust
pub enum FivePowerEffect {
    ImportedGoods(u8),
    Copper(u8),
    Silk(u8),
    Tea(u8),
    Seafood(u8),
    Money(u8),
    VictoryPoints(u8),
    Movement(u8),
}
```

**Recommended format:**
```yaml
five_power_tokens:
  - effect: "imported_goods"
    quantity: 2
    count_in_game: 3  # how many of this token exist

  - effect: "copper"
    quantity: 2
    count_in_game: 2

  - effect: "victory_points"
    quantity: 3
    count_in_game: 2
```

**Questions to resolve:**
- [ ] Are there combination tokens (e.g., "1 VP + 1 Movement")?
- [ ] Total count of tokens (should match area count: 18 for 4P)

---

### 5. Board Generation Rules

**Status:** Can implement NOW - rules are in rulebook

**What we know:**
- Board is a grid of area tiles
- Certain areas are always included
- Some areas removed for 2P/3P games
- Tiles are shuffled and placed randomly
- Adjacency follows grid positions

**From rulebook (to verify):**
```
4-player: 18 area tiles
3-player: 15 area tiles (remove 3)
2-player: 12 area tiles (remove 6)

Always included:
- Port A, Port B (3-4P only)
- Laboratory A, Laboratory B (4P only)
- Church, Customs, Bank, Employment Agency

Removed for 2P:
- Port B, Laboratory B, Chinatown, Canal
- 1x each production type

Removed for 3P:
- Laboratory B
- 1x each of 3 production types
```

**Grid layout questions:**
- [ ] Fixed grid dimensions? (e.g., 3x6 for 4P?)
- [ ] Canal placement rules?
- [ ] Any adjacency restrictions?

---

## Recommended Data Collection Approach

### Option A: Photography + Manual Transcription
1. Photograph all cards laid out clearly
2. Create a spreadsheet/YAML from the photos
3. Double-check against a second game copy if available

**Pros:** Accurate, catches edge cases
**Cons:** Time-consuming

### Option B: Community Resources
1. Check BoardGameGeek for card lists
2. Look for Tabletop Simulator mods (often have card data)
3. Search for fan-made reference sheets

**Pros:** Fast if resources exist
**Cons:** May be incomplete or have errors

### Option C: Hybrid
1. Find community resources first
2. Verify against physical components
3. Fill gaps with photography

**Recommended:** Start with Option C

---

## Suggested File Structure

```
analysis-1/
├── yokohama-kb/
│   ├── 07-order-cards.yaml      # Order card data
│   ├── 08-tech-cards.yaml       # Tech card data
│   ├── 09-building-sites.yaml   # Building site cards
│   ├── 10-five-power-tokens.yaml # 5-power token set
│   └── images/
│       ├── order-cards/         # Photos if needed
│       ├── tech-cards/
│       └── building-sites/
```

---

## Priority Order

| Priority | Component | Effort | Status |
|----------|-----------|--------|--------|
| 1 | Board Generation | 2-3 hrs | ✅ DONE |
| 2 | 5-Power Tokens | 30 min | 🔄 Extracting |
| 3 | Building Sites | 1 hr | 🔄 Extracting |
| 4 | Tech Cards (countries) | 30 min | 🔄 Extracting |
| 5 | Order Cards | 2-3 hrs | 🔄 Extracting |

**Note:** Order cards are listed last because the engine can simulate games without them (just skip order-related actions). However, for realistic MCTS playouts, they're essential.

---

## Current Approach: Tabletop Simulator Extraction

Extracting all game assets from a TTS mod, which should provide:
- High-quality card images
- Complete card sets
- Accurate counts

### TTS Extraction Checklist

- [ ] Extract Order card images → transcribe to YAML
- [ ] Extract Tech card images → verify countries, fill in missing industry costs
- [ ] Extract Building Site card images → document VP configurations
- [ ] Extract 5-Power token images → list all types and counts
- [ ] Count area tiles → verify 10/14/18 vs 12/15/18 for 2P/3P/4P

---

## DATA NEEDED FROM USER (Next Session)

### 1. Order Cards (~36 cards)
For each card, provide:
```yaml
- id: "order_001"           # Unique identifier
  country: "english"        # english/american/french/german/dutch
  requirements:
    copper: 0
    silk: 2
    tea: 1
    seafood: 0
  rewards:
    victory_points: 5
    money: 0
    imported_goods: 1
    copper: 0
    silk: 0
    tea: 0
    seafood: 0
    movement: 0
    any_trade_good: 0
    warehouse_tokens: 0
```

### 2. Tech Cards (20 cards)
Effects are already implemented. Need country pairing for each:
```yaml
- effect: "Tram"
  country: "german"         # Which country flag?
  industry_cost: 5          # Verify cost

- effect: "Stagecoach"
  country: "?"
  industry_cost: 3
```

**Missing industry costs to verify:**
- PostalSystem, ElectricalLight, LanguageSchool, LetterPressPrinting
- Winery, SystemOfStock, University, Newspaper, Telegram

### 3. Building Site Cards
How many unique cards? For each:
```yaml
- id: "site_01"
  shophouse_rewards: [2, 3, 4, 5]   # VP for slots 1-4
  trading_house_reward: 7
```

### 4. 5-Power Tokens

**EXTRACTED FROM TTS (8 tokens):**
```yaml
- effect: "imported_goods"
  quantity: 1
  count: 3

- effect: "silk"
  quantity: 2
  count: 2

- effect: "seafood"
  quantity: 3
  count: 1

- effect: "money"
  quantity: 3
  count: 2
```

**MISSING - Need to verify from physical game:**
- Total should be 17 tokens for 4P (one per non-Canal area)
- TTS only has 8 - where are the other 9?
- Possible missing types: Copper, Tea, VP, Movement?

**Questions for physical game check:**
- How many unique 5-power token types exist?
- What are ALL the effects (not just the 4 from TTS)?
- How many of each type?

### 5. Tile Count Verification
**✅ VERIFIED from TTS:**

| Player Count | Tile Count | Source |
|--------------|------------|--------|
| 2P | 10 | Base tiles only |
| 3P | 14 | Base + 3+ tiles |
| 4P | 18 | Base + 3+ + 4P tiles |

**Area breakdown:**
- Base (10): Port, Silk Mill, Tea Plantation, Laboratory, Bank, Church, Copper Mine, Customs, Employment Agency, Fishing Ground
- 3+ tiles (4): Canal, Chinatown, Dock (Port B), Fishing Ground #2
- 4P tiles (4): Copper Mine #2, Research Center (Lab B), Silk Mill #2, Tea Plantation #2

**Naming differences from code:**
- Port → PortA, Dock → PortB
- Laboratory → LaboratoryA, Research Center → LaboratoryB

---

## Next Steps

1. [x] ~~Implement board generation~~ → DONE: `game/board_gen.rs`
2. [x] ~~Complete TTS asset extraction~~ → DONE: `tts-assets/organized/`
3. [x] ~~Verify tile counts~~ → DONE: 10/14/18 confirmed
4. [ ] **Verify 5-power tokens from physical game** ← BLOCKING
5. [ ] Transcribe Order Cards (36) from sprite sheet to YAML
6. [ ] Transcribe Tech Cards (20) from sprite sheet to YAML
7. [ ] Transcribe Building Sites (24) from sprite sheet to YAML
8. [ ] Implement `FivePowerTokenSet::standard()`
9. [ ] Implement `BuildingSiteSet::standard()`
10. [ ] Implement `TechRegistry::standard()`
11. [ ] Implement `OrderRegistry::standard()`

---

## Notes for AI Assistant

When implementing the data:

1. **Order Cards:** Create `OrderRegistry::standard()` function that populates all cards
2. **Tech Cards:** Create `TechRegistry::standard()` function
3. **Building Sites:** Create a `BuildingSiteSet` that can be shuffled for setup
4. **5-Power Tokens:** Create a `FivePowerTokenSet` that can be shuffled for setup
5. **Board Generation:** Create `BoardLayout::generate(config: &GameConfig, rng: &mut impl Rng)` function

All data should be defined in code (not external files) for simplicity and type safety.
