# TTS Yokohama Asset Extraction Summary

**Mod Source:** https://steamcommunity.com/sharedfiles/filedetails/?id=1840951801
**Extraction Date:** 2025-12-07

## Successfully Extracted Assets

### Card Sprite Sheets (High Quality)

| File | Grid | Contents |
|------|------|----------|
| `cards/order_cards_8x5.jpg` | 8x5 | 36 Order cards + 4 backs |
| `cards/tech_cards_8x4.jpg` | 8x4 | 20 Tech cards (some duplicates) |
| `cards/building_site_cards_5x5.jpg` | 5x5 | 24 Building site cards + 1 back |
| `cards/achievement_A_3x2.jpg` | 3x2 | Achievement cards set A |
| `cards/achievement_B_3x2.jpg` | 3x2 | Achievement cards set B |
| `cards/achievement_C_3x2.jpg` | 3x2 | Achievement cards set C |

### Area Board Tiles

| File | Grid | Contents |
|------|------|----------|
| `boards/area_boards_base_3x4.jpg` | 3x4 | 12 base area tiles |
| `boards/area_boards_3plus_2x3.jpg` | 2x3 | 6 tiles for 3+ players |
| `boards/area_boards_4p_2x3.jpg` | 2x3 | 6 tiles for 4 players only |

### Documentation

- `misc/yokohama_rules.pdf` - Full rulebook (15MB)

## Data You Can Extract From These

### Order Cards (36 cards) - `order_cards_8x5.jpg`
Each card shows:
- **Country flag** (top-left corner): 米=USA, 独=Germany, 蘭=Netherlands, 仏=France, 英=Britain
- **Requirements** (middle): Copper, Silk, Tea, Fish icons with quantities
- **Rewards** (bottom): VP stars (blue), Yen (yellow), movement, etc.

### Technology Cards (20 cards) - `tech_cards_8x4.jpg`
Each card shows:
- **Industry cost** (top-left star number)
- **Name** (English at top)
- **Effect description** (middle text)
- **Country flag** (bottom)

### Building Site Cards (24 cards) - `building_site_cards_5x5.jpg`
Each card shows:
- **4 shophouse slots** with VP values (1-4 stars)
- **Trading house slot** with VP value (larger star at bottom)
- Some cards have **resource bonuses** in the trading house slot

### Area Tiles (18 total for 4P)
- Base: 12 tiles (always used)
- 3+ Players: +6 tiles
- 4 Players: +6 more tiles (some overlap)

### Tokens (All Extracted)

| File | Description |
|------|-------------|
| `tokens/5power_3_fish_x1.jpg` | 5-Power: Gain 3 Fish (1 in game) |
| `tokens/5power_1_import_x3.jpg` | 5-Power: Gain 1 Import (3 in game) |
| `tokens/5power_2_silk_x2.jpg` | 5-Power: Gain 2 Silk (2 in game) |
| `tokens/5power_3_yen_x2.jpg` | 5-Power: Gain 3 Yen (2 in game) |
| `tokens/copper.jpg` | Copper resource token |
| `tokens/silk.jpg` | Silk resource token |
| `tokens/tea.jpg` | Tea resource token |
| `tokens/fish.jpg` | Fish (Seafood) resource token |
| `tokens/imports.jpg` | Imported goods token |
| `tokens/1_yen.jpg` | 1 Yen currency token |
| `tokens/3_yen.jpg` | 3 Yen currency token |
| `tokens/station.jpg` | Station token |

### 5-Power Token Summary

| Token | Effect | Count |
|-------|--------|-------|
| 3 Fish | Gain 3 Seafood | 1 |
| 1 Import | Gain 1 Imported Good | 3 |
| 2 Silk | Gain 2 Silk | 2 |
| 3 Yen | Gain 3 Money | 2 |
| **Total** | | **8 tokens** |

## How to Use for Data Collection

### Recommended Workflow

1. **Order Cards**: Open `order_cards_8x5.jpg` and transcribe each card to YAML
   - Grid position (0,0) = top-left, (7,4) = bottom-right
   - Last 4 positions (row 5, cols 4-7) are card backs

2. **Tech Cards**: Open `tech_cards_8x4.jpg` and verify/complete your tech data
   - Match card names to your `TechEffect` enum
   - Extract industry costs and country associations

3. **Building Sites**: Open `building_site_cards_5x5.jpg` and document VP configurations
   - 24 unique cards (last slot is card back)

### Tile Count Verification

From the TTS mod:
- **Base tiles**: 12 (3x4 grid)
- **3+ player tiles**: 6 additional
- **4 player tiles**: 6 additional

This suggests: 12 (2P) → 18 (3P) → 24 (4P) but the mod separates 3P and 4P additions.

## Raw Files

All original cached images are in `images/` folder with URL-based filenames.
The mod JSON is at `1840951801.json` for reference.
Asset mapping is at `asset_mapping.json`.
