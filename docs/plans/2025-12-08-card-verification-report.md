# Card Transcription Verification Report

**Date:** 2025-12-08
**Purpose:** Verify transcribed card data against TTS sprite sheets
**Status:** READY FOR USER REVIEW

---

## How to Verify

1. Open each sprite sheet image (listed below)
2. Compare each card against the transcribed data
3. Mark any discrepancies in the "Corrections" section at the bottom
4. Cards are numbered by position: (row, column) starting from (0,0) at top-left

**Sprite Sheet Locations:**
- Order Cards: `tts-assets/organized/cards/order_cards_8x5.jpg`
- Tech Cards: `tts-assets/organized/cards/tech_cards_8x4.jpg`
- Building Sites: `tts-assets/organized/cards/building_site_cards_5x5.jpg`

**YAML Files:**
- `analysis-1/yokohama-kb/07-order-cards.yaml`
- `analysis-1/yokohama-kb/08-tech-cards.yaml`
- `analysis-1/yokohama-kb/09-building-sites.yaml`

---

## 1. ORDER CARDS (36 cards)

### Legend
- **Country:** 米=US, 独=DE(Germany), 蘭=NL(Netherlands), 仏=FR(France), 英=GB(Britain)
- **Resources:** C=Copper, S=Silk, T=Tea, F=Fish(Seafood)
- **Rewards:** VP=Victory Points, Y=Yen, I=Import, M=Movement, W=Warehouse

### Verification Table

| Pos | ID | Country | Requirements | Rewards | ✓ |
|-----|-----|---------|--------------|---------|---|
| 0,0 | 001 | US | C:3 S:1 T:2 F:0 | 8VP I:1 | ☐ |
| 0,1 | 002 | DE | C:0 S:0 T:1 F:3 | M:1 | ☐ |
| 0,2 | 003 | DE | C:1 S:1 T:2 F:0 | 6VP Y:2 | ☐ |
| 0,3 | 004 | NL | C:1 S:0 T:1 F:1 | 5VP | ☐ |
| 0,4 | 005 | US | C:3 S:0 T:1 F:2 | 9VP | ☐ |
| 0,5 | 006 | US | C:0 S:0 T:2 F:4 | 5VP Y:2 W:1 | ☐ |
| 0,6 | 007 | DE | C:2 S:1 T:0 F:2 | 8VP Y:1 | ☐ |
| 0,7 | 008 | NL | C:0 S:1 T:1 F:0 | 3VP | ☐ |
| 1,0 | 009 | DE | C:1 S:0 T:0 F:3 | 2VP Y:1 I:1 | ☐ |
| 1,1 | 010 | DE | C:0 S:1 T:1 F:2 | 5VP W:1 | ☐ |
| 1,2 | 011 | DE | C:0 S:0 T:3 F:3 | 11VP | ☐ |
| 1,3 | 012 | NL | C:1 S:0 T:2 F:1 | 2VP Y:1 I:1 | ☐ |
| 1,4 | 013 | US | C:0 S:0 T:4 F:1 | 7VP Y:2 | ☐ |
| 1,5 | 014 | US | C:0 S:0 T:4 F:2 | 8VP | ☐ |
| 1,6 | 015 | NL | C:0 S:2 T:2 F:0 | 1VP Y:2 W:1 | ☐ |
| 1,7 | 016 | GB | C:0 S:1 T:1 F:0 | 5VP I:1 | ☐ |
| 2,0 | 017 | GB | C:2 S:0 T:0 F:1 | 8VP I:1 | ☐ |
| 2,1 | 018 | GB | C:3 S:0 T:0 F:1 | 2VP Y:2 W:1 | ☐ |
| 2,2 | 019 | GB | C:1 S:0 T:0 F:2 | 4VP Y:1 | ☐ |
| 2,3 | 020 | GB | C:2 S:0 T:0 F:1 | 7VP Y:2 I:1 | ☐ |
| 2,4 | 021 | GB | C:0 S:0 T:1 F:1 | 9VP I:1 | ☐ |
| 2,5 | 022 | US | C:4 S:0 T:0 F:1 | 3VP Y:1 W:1 | ☐ |
| 2,6 | 023 | GB | C:1 S:1 T:0 F:2 | 4VP Y:2 W:1 | ☐ |
| 2,7 | 024 | GB | C:0 S:2 T:0 F:1 | 8VP Y:2 | ☐ |
| 3,0 | 025 | US | C:2 S:1 T:0 F:0 | 1VP W:1 | ☐ |
| 3,1 | 026 | US | C:3 S:0 T:0 F:2 | 6VP I:1 | ☐ |
| 3,2 | 027 | FR | C:2 S:1 T:3 F:0 | 9VP I:1 | ☐ |
| 3,3 | 028 | FR | C:2 S:0 T:3 F:0 | 9VP | ☐ |
| 3,4 | 029 | FR | C:0 S:3 T:1 F:0 | 1VP Y:2 | ☐ |
| 3,5 | 030 | FR | C:0 S:1 T:3 F:0 | 3VP | ☐ |
| 3,6 | 031 | NL | C:1 S:1 T:1 F:1 | 8VP | ☐ |
| 3,7 | 032 | FR | C:0 S:0 T:2 F:1 | 1VP | ☐ |
| 4,0 | 033 | FR | C:0 S:2 T:3 F:0 | 4VP Y:2 | ☐ |
| 4,1 | 034 | NL | C:2 S:2 T:2 F:0 | 11VP | ☐ |
| 4,2 | 035 | FR | C:0 S:0 T:1 F:2 | 3VP | ☐ |
| 4,3 | 036 | FR | C:0 S:1 T:4 F:0 | 8VP | ☐ |

### Order Cards Summary
- **Total:** 36 cards
- **By Country:** US:10, DE:7, NL:6, FR:8, GB:5

---

## 2. TECHNOLOGY CARDS (20 unique cards)

### Legend
- **Cost:** Industry tokens required
- **Country:** US, DE, NL, FR, GB

### Verification Table

| Pos | Name | Cost | Country | Effect Summary | ✓ |
|-----|------|------|---------|----------------|---|
| 0,0 | Language School | 3 | US | Order: +1 Yen | ☐ |
| 0,1 | Telegram | 6 | US | Place 2 workers → +1 adjacent | ☐ |
| 0,2 | Telephone | 6 | NL | Leave 1 worker in area | ☐ |
| 0,3 | Newspaper | 5 | NL | Place 4 workers instead of 3 | ☐ |
| 0,4 | Exposition | 5 | DE | +10 VP at game end | ☐ |
| 0,5 | Ball | 3 | DE | Order: +2 VP | ☐ |
| 0,6 | Gaslight | 3 | GB | Build shop: +2 VP | ☐ |
| 0,7 | Station | 4 | GB | Free move to Station area | ☐ |
| 1,0 | University | 3 | GB | Pay 1Y → worker from warehouse | ☐ |
| 1,1 | Letterpress Printing | 3 | NL | Tech card: +1 VP | ☐ |
| 1,2 | Postal System | 3 | GB | Move to occupied President area | ☐ |
| 1,3 | Winery | 3 | FR | Church action: +3 VP | ☐ |
| 1,4 | Electrical Light | 4 | FR | Trading House: +3 VP, +1 Y | ☐ |
| 1,5 | Mining Technology | 3 | DE | Copper gain: +1 Copper | ☐ |
| 1,6 | Textile Mill | 4 | FR | 1 Fish → 2 Silk | ☐ |
| 1,7 | Patent System | 3 | FR | Tech card: +2 VP, +1 Y | ☐ |
| 2,0 | Stagecoach | 3 | GB | Move over Presidents free | ☐ |
| 2,1 | Tram | 5 | US | Move to any area | ☐ |
| 2,2 | Brickyard | 3 | DE | Customs action: +4 VP | ☐ |
| 2,3 | Stock Market | 3 | NL | Turn start: min 2 Yen | ☐ |

### Tech Cards Summary
- **Total:** 20 unique cards
- **By Country:** US:3, DE:4, NL:4, FR:4, GB:5
- **By Cost:** Cost 3:12, Cost 4:3, Cost 5:3, Cost 6:2

### Duplicates in Sprite Sheet
The sprite sheet contains duplicates at positions 2,4 through 3,5. Please verify if any have different countries:
- 2,4: Language School (US?)
- 2,5: Telegram (US?)
- 2,6: Telephone (US? vs NL at 0,2)
- 2,7: Newspaper (NL?)
- 3,0: Exposition (DE?)
- 3,1: Ball (GB? vs DE at 0,5)
- 3,2: Gaslight (FR? vs GB at 0,6)
- 3,3: University (GB?)
- 3,4: Letterpress Printing (GB? vs NL at 1,1)
- 3,5: Postal System (GB?)

---

## 3. BUILDING SITE CARDS (24 cards)

### Legend
- **SH[1-4]:** Shophouse slots 1-4 VP values
- **TH:** Trading House VP value
- **Bonus:** Resource bonuses (copper, fish, tea, silk, yen, import, any)

### Verification Table

| Pos | ID | SH[1,2,3,4] | TH VP | SH Bonuses | TH Bonus | ✓ |
|-----|-----|-------------|-------|------------|----------|---|
| 0,0 | 001 | [1,3,3,3] | 7 | s1:fish, s3:fish | - | ☐ |
| 0,1 | 002 | [3,1,2,2] | 7 | s1:fish, s3:fish | - | ☐ |
| 0,2 | 003 | [3,4,2,2] | 9 | s2:tea, s4:tea | - | ☐ |
| 0,3 | 004 | [2,2,2,2] | 8 | s2:copper | import | ☐ |
| 0,4 | 005 | [2,1,1,1] | 9 | s1:fish, s2:copper | copper | ☐ |
| 1,0 | 006 | [2,1,2,2] | 9 | s1:copper | - | ☐ |
| 1,1 | 007 | [2,1,2,2] | 7 | s1:any | - | ☐ |
| 1,2 | 008 | [2,2,1,1] | 6 | s2:silk, s3:tea | - | ☐ |
| 1,3 | 009 | [2,4,2,2] | 8 | s2:tea | copper | ☐ |
| 1,4 | 010 | [2,1,1,1] | 6 | s3:copper | import | ☐ |
| 2,0 | 011 | [1,2,2,2] | 7 | s2:copper, s4:yen | - | ☐ |
| 2,1 | 012 | [2,1,2,2] | 5 | s2:copper, s4:yen | - | ☐ |
| 2,2 | 013 | [2,2,1,2] | 6 | s2:copper, s3:yen | yen? | ☐ |
| 2,3 | 014 | [3,1,2,2] | 5 | s2:copper, s4:yen | - | ☐ |
| 2,4 | 015 | [3,1,2,2] | 8 | s2:copper | import | ☐ |
| 3,0 | 016 | [1,1,2,2] | 8 | s2:copper, s3:yen | - | ☐ |
| 3,1 | 017 | [1,1,2,1] | 9 | s2:copper, s4:yen | - | ☐ |
| 3,2 | 018 | [1,1,2,2] | 8 | s3:yen | - | ☐ |
| 3,3 | 019 | [1,1,2,2] | 9 | s3:yen | yen | ☐ |
| 3,4 | 020 | [1,4,2,2] | 6 | - | yen | ☐ |
| 4,0 | 021 | [1,1,2,2] | 5 | s3:yen | - | ☐ |
| 4,1 | 022 | [1,1,4,2] | 5 | s3:yen | - | ☐ |
| 4,2 | 023 | [1,1,4,3] | 9 | - | - | ☐ |
| 4,3 | 024 | [1,1,3,2] | 6 | s3:yen | - | ☐ |

### Building Sites Summary
- **Total:** 24 cards
- **Trading House VP Range:** 5-9
- **Shophouse VP Range:** 1-4

---

## CORRECTIONS LOG

Please note any corrections needed below:

### Order Cards Corrections
```
Position | Field | Transcribed | Actual
---------|-------|-------------|-------
         |       |             |
         |       |             |
```

### Tech Cards Corrections
```
Position | Field | Transcribed | Actual
---------|-------|-------------|-------
         |       |             |
         |       |             |
```

### Building Site Cards Corrections
```
Position | Field | Transcribed | Actual
---------|-------|-------------|-------
         |       |             |
         |       |             |
```

---

## Notes

1. **Image Quality:** Some icons are difficult to distinguish in the sprite sheet. Please verify resource bonuses carefully.

2. **Tech Card Duplicates:** The sprite sheet has duplicate tech cards. Some duplicates may have different country flags - please verify.

3. **Order Card Rewards:** Some rewards may have been missed or misidentified. Look for small icons in the reward area.

4. **Building Site Layout:**
   ```
   [slot1] [slot2]
   [slot3] [slot4]
   [trading_house]
   ```

5. **After Verification:** Once all corrections are noted, I will update the YAML files and implement the Rust registries.
