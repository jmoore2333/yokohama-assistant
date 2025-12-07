# Yokohama VLM Extraction Prompts

> Prompt templates for extracting game state from overhead camera images.

---

## Setup Phase Prompts

### Phase 1: Board Layout Extraction

```
You are analyzing an overhead photograph of the board game Yokohama during initial setup.

TASK: Extract the board layout - identify each area tile and its position.

AREA TYPES TO IDENTIFY:
Production Areas (factory icon):
- Copper Mine (銅鉱山) - mountain/mining imagery
- Silk Mill (製糸場) - factory building imagery  
- Tea Plantation (茶畑) - green fields imagery
- Fishing Ground (漁場) - water/boats imagery

Commercial Areas (building icon):
- Bank (銀行) - grand building imagery
- Port A/B (港) - harbor/ships imagery
- Laboratory A/B (研究所) - scientific building imagery
- Employment Agency (斡旋所) - office building imagery
- Chinatown (中華街) - Chinese architecture imagery
- Church (教会) - church/cross imagery
- Customs (税関) - customs house imagery
- Canal (運河) - water/bridge imagery

OUTPUT FORMAT:
{
  "grid_layout": [
    ["row0_col0_area", "row0_col1_area", ...],
    ["row1_col0_area", "row1_col1_area", ...],
    ...
  ],
  "area_identifications": {
    "row0_col0": {
      "type": "copper_mine",
      "confidence": 0.95,
      "visual_cues": "mountain imagery, orange/brown coloring"
    },
    ...
  },
  "adjacencies": [
    {"from": "row0_col0", "to": "row0_col1"},
    {"from": "row0_col0", "to": "row1_col0"},
    ...
  ],
  "overall_confidence": 0.90
}

NOTES:
- Areas are placed in a connected grid pattern
- Adjacent areas share edges (not just corners)
- Canal areas create barriers (1 yen to cross)
- 2-player games have 12 areas, 3-player have 15, 4-player have 18
```

### Phase 2: Building Sites Extraction

```
You are analyzing the Building Site cards placed on each area of Yokohama.

TASK: Extract the VP rewards shown on each area's Building Site card.

Each Building Site card shows:
- 4 Shophouse spaces with VP values (typically 1-4 VP, or 3, 8 for special spaces)
- 1 Trading House space with VP value (typically 2-9 VP)

OUTPUT FORMAT:
{
  "building_sites": {
    "row0_col0": {
      "shophouse_vp": [3, 1, 8, 4],  // left to right
      "trading_house_vp": 9,
      "confidence": 0.85
    },
    ...
  },
  "overall_confidence": 0.80
}

VISUAL CUES:
- Building Site cards overlay the bottom portion of area tiles
- Blue star icons indicate VP values
- Shophouse spaces are smaller squares
- Trading House space is typically larger/different shape
```

### Phase 3: Initial Cards Extraction

```
You are analyzing the Technology and Order cards placed during Yokohama setup.

TASK: Extract the cards on Laboratory and Port management boards.

LABORATORY CARDS:
- 4 slots per Laboratory (A and optionally B)
- Each Technology card shows:
  - Name (in English and Japanese)
  - Industry cost (gear icon with number)
  - Country icon (English/American/French/German/Dutch flag)
  - Effect description

PORT CARDS:
- 3 slots per Port (Low/Mid/High): A and optionally B
- Each Order card shows:
  - Country icon
  - Required trade goods (icons with quantities)
  - Rewards (VP stars, money, goods)

OUTPUT FORMAT:
{
  "laboratory_a": [
    {
      "slot": 1,
      "name": "Gaslight",
      "industry_cost": 3,
      "country": "german",
      "effect_summary": "build_shophouse_+2vp",
      "confidence": 0.90
    },
    ...
  ],
  "laboratory_b": [...],  // null if not present
  "port_a": {
    "low": {
      "country": "english",
      "requires": {"copper": 2, "silk": 1},
      "rewards": {"vp": 5, "money": 2},
      "confidence": 0.85
    },
    "mid": {...},
    "high": {...}
  },
  "port_b": {...},  // null if not present
  "overall_confidence": 0.82
}

KNOWN TECHNOLOGY CARDS:
- Gaslight, Station, Mining Technology, Spinning Mill
- Stagecoach, Exposition, Ball, Language School
- Letterpress Printing, Winery, Postal System, Newspaper
- Electrical Light, University, Telegram, Telephone
- Brickyard, System of Stock, Patent System, Tram
```

---

## Turn Delta Extraction Prompts

### Two-Image Comparison (Preferred)

```
You are analyzing two overhead photographs of Yokohama - one from before a player's turn and one from after.

PLAYER COLORS:
- Red: burgundy/maroon wooden pieces
- Blue: navy/dark blue wooden pieces  
- Yellow: gold/tan wooden pieces
- Green: forest green wooden pieces

TASK: Identify all changes between the two images to determine what actions the player took.

LOOK FOR THESE CHANGES:

1. ASSISTANT PAWNS (small cylindrical pieces):
   - New placements (appeared in areas)
   - Removals (disappeared from areas, returned to hand)
   - Moved to management boards (Church/Customs tracks)

2. PRESIDENT PAWN (larger piece with distinctive top):
   - Movement from one area to another
   - Removal (returned to hand)

3. BUILDINGS:
   - New Shophouse tokens (small house shapes)
   - New Trading House tokens (larger building shapes)

4. CARDS:
   - Missing Order cards from Port board slots
   - Missing Technology cards from Laboratory slots
   - Cards refilled into empty slots

5. TRACK CHANGES:
   - New pawns on Church Management Board
   - New pawns on Customs Management Board

6. TOKEN CHANGES:
   - 5-Power tokens removed from areas
   - Station token placed (if Station technology acquired)

OUTPUT FORMAT:
{
  "acting_player": "red",
  "turn_actions": {
    "placements": [
      {"area": "silk_mill", "count": 1},
      {"area": "bank", "count": 1},
      {"area": "port_a", "count": 1}
    ],
    "president_movement": {
      "from": "tea_plantation",
      "to": "bank",
      "path": ["tea_plantation", "employment_agency", "bank"]
    },
    "area_action": {
      "area": "bank",
      "type": "bank",
      "power_estimate": 4
    },
    "building": {
      "type": "shophouse",
      "area": "bank",
      "slot": 2
    },
    "cards_taken": {
      "orders": [],
      "techs": []
    },
    "track_placement": null,
    "tokens_taken": {
      "five_power": false
    }
  },
  "inferred_resources": {
    "money_gained": 4,
    "money_spent": 0
  },
  "confidence": 0.88,
  "uncertainties": []
}
```

### Single-Image with State Reference

```
You are analyzing an overhead photograph of Yokohama after a player's turn.

KNOWN PREVIOUS STATE:
{previous_state_json}

TASK: Compare the image to the known previous state and identify changes.

[Same detection categories as two-image prompt]

VALIDATION CHECKS:
- Changes should be legal according to game rules
- Turn should follow: Placement → Movement → Action → Build → Recovery
- Power calculation should match visible pieces

OUTPUT FORMAT:
[Same as two-image prompt]

If changes don't match legal game flow, flag as uncertainty.
```

---

## Clarification Prompts

### Ambiguous Piece Color

```
In region [{x}, {y}, {w}, {h}] of the image, there is a game piece whose color is unclear.

The piece appears to be: {piece_type}

Please identify the color from these options:
A) Red (burgundy/maroon)
B) Blue (navy/dark blue)
C) Yellow (gold/tan)
D) Green (forest green)

Consider:
- Lighting conditions in the image
- Comparison to other pieces of known colors
- The piece's saturation and hue

Respond with the letter and confidence (0.0-1.0).
```

### Ambiguous Piece Count

```
In the area identified as {area_name}, the number of {piece_type} pieces is unclear.

The area appears to contain: {visual_description}

Please identify the count from these options:
A) {count_option_1}
B) {count_option_2}
C) {count_option_3}

Consider:
- Overlapping pieces
- Shadows
- Piece spacing

Respond with the letter, count, and confidence (0.0-1.0).
```

### Card Identification

```
A card in {location} needs identification.

Visible features:
- {partial_text}
- {icon_descriptions}
- {color_patterns}

Please identify from these possible cards:
{card_options_list}

Respond with the card name and confidence (0.0-1.0).
```

---

## Resource Counting Prompts

### Player Resource Verification

```
You are verifying the resources for the {color} player in this Yokohama image.

RESOURCE TYPES:
- Copper tokens: Orange/brown octagonal
- Raw Silk tokens: Beige/cream rectangular
- Tea tokens: Green leaf-shaped
- Seafood tokens: Blue fish-shaped
- Imported Goods tokens: Yellow/gold with pattern
- Money: 1-Yen (small) and 3-Yen (larger) coins

TASK: Count each resource type visible in the player's area.

Look for resources:
- On/near the player's Warehouse tile
- In front of the player's position
- Separate from the main supply

OUTPUT FORMAT:
{
  "player": "{color}",
  "resources": {
    "copper": 3,
    "silk": 2,
    "tea": 4,
    "seafood": 1,
    "imported_goods": 2,
    "money": 7
  },
  "confidence": 0.80,
  "notes": "Some tokens may be obscured by cards"
}
```

---

## Validation Prompts

### Turn Legality Check

```
Based on the detected turn actions:
{detected_actions_json}

And the game rules:
- Placement: 3 areas with 1 each, OR 1 area with 2
- Movement: Step through areas with own assistants
- Action: Power = president + assistants + buildings + station
- Building: Requires 4+ power, token available

Please verify:
1. Is this a legal turn sequence?
2. Does power calculation match visible pieces?
3. Are there any rule violations?

If violations detected, suggest corrections.
```

---

## Color Calibration Reference

Include in all prompts when lighting varies:

```
COLOR REFERENCE NOTE:
In this image, player colors appear as:
- Red player: {description from calibration}
- Blue player: {description from calibration}
- Yellow player: {description from calibration}
- Green player: {description from calibration}

The lighting condition is: {bright/dim/mixed/warm/cool}

Adjust color identification accordingly.
```

---

## Confidence Thresholds

```
CONFIDENCE HANDLING:

>0.90: Accept automatically, no human verification needed
0.70-0.90: Accept with logging, flag for potential review
0.50-0.70: Generate clarification question
<0.50: Require human verification before proceeding

When confidence is low:
1. Identify specific uncertain elements
2. Generate targeted clarification questions
3. Provide image region coordinates for UI highlighting
4. Offer multiple-choice options when possible
```
