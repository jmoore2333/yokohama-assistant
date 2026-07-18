# Blind re-transcription: Technology cards

**Source:** `tts-assets/organized/cards/tech_cards_8x4.jpg` (4240x3196, 8 cols x 4 rows, cell 530x799)
**Method:** each cell cropped at native resolution with ImageMagick and read individually. Produced WITHOUT opening any YAML file or prior verification report.
**Date:** 2026-07-16/17

## Legend

- Position `(row,col)`, 1-based, row-major.
- `industry_cost` = the number in the dark gear at top-left of each card.
- `country` = the flag roundel + country name printed upside-down at the bottom edge of each card.
- Effect icons: `yen(N)` silver coin, `vp(N)` blue star, `copper` red-square ingot, `silk` white-square bale.

## Cards

| Pos | Name | industry_cost | Country | Effect (verbatim text; icons bracketed) |
|---|---|---|---|---|
| (1,1) | Language School | 3 | United States of America | "When you fulfill an Order: [yen(1)]" |
| (1,2) | Telegram | 6 | United States of America | "Placement Step: When you place 2 Workers in the same area, you may additionally place 1 Worker in an adjacent area." |
| (1,3) | Telephone | 6 | United States of America | "Collection Step: You may leave 1 Worker in an area." |
| (1,4) | Newspaper | 5 | Netherlands | "Placement Step: When you place Workers in different areas, you may place 4 Workers instead of 3." |
| (1,5) | Exposition | 5 | Germany | "[vp(10)]" (no trigger text; flat 10 VP) |
| (1,6) | Ball | 3 | Great Britain | "When you fulfill an Order: [vp(2)]" |
| (1,7) | Gaslight | 3 | France | "When you build a Shop: [vp(2)]" |
| (1,8) | Station | 4 | Great Britain | "When you gain this card, place the Station token. Movement Step: You may move your President to the area containing the Station token for free." |
| (2,1) | University | 3 | Great Britain | "Once Per Turn: You may gain a Worker from your Warehouse by paying 1 yen." |
| (2,2) | Letterpress Printing | 3 | Netherlands | "When you gain a Technology card: [yen(1)]" |
| (2,3) | Postal System | 3 | Great Britain | "Movement Step: You may place your President in an area occupied by another President." |
| (2,4) | Winery | 3 | France | "When you carry out a Church action: [vp(3)]" |
| (2,5) | Electrical Light | 4 | France | "When you build a Trading House: [vp(3)]" |
| (2,6) | Mining Technology | 3 | Germany | "Whenever you gain one or more Copper: +1 [copper]" |
| (2,7) | Textile Mill | 4 | Great Britain | "Once Per Turn: [silk] -> [yen(2)]" (convert 1 Silk to 2 yen) |
| (2,8) | Patent System | 3 | France | "When you gain a Technology card: [vp(2)]" |
| (3,1) | Stagecoach | 3 | Great Britain | "Movement Step: You may move your President over areas occupied by other President markers without paying." |
| (3,2) | Tram | 5 | United States of America | "Movement Step: You may move your President to any area (except for areas occupied by other Presidents)." |
| (3,3) | Brickyard | 3 | Germany | "When you carry out a Customs action: [vp(4)]" |
| (3,4) | Stock Market | 3 | Netherlands | "Start of Your Turn: If you have less than 2 yen in your hand, increase it to 2 yen." |
| (3,5) | Language School (2nd copy) | 3 | United States of America | identical to (1,1) |
| (3,6) | Telegram (2nd copy) | 6 | United States of America | identical to (1,2) |
| (3,7) | Telephone (2nd copy) | 6 | United States of America | identical to (1,3) |
| (3,8) | Newspaper (2nd copy) | 5 | Netherlands | identical to (1,4) |
| (4,1) | Exposition (2nd copy) | 5 | Germany | identical to (1,5) |
| (4,2) | Ball (2nd copy) | 3 | Great Britain | identical to (1,6) |
| (4,3) | Gaslight (2nd copy) | 3 | France | identical to (1,7) |
| (4,4) | University (2nd copy) | 3 | Great Britain | identical to (2,1) |
| (4,5) | Letterpress Printing (2nd copy) | 3 | Netherlands | identical to (2,2) |
| (4,6) | Postal System (2nd copy) | 3 | Great Britain | identical to (2,3) |
| (4,7)-(4,8) | CARD BACK (dark, gear silhouette) x2 | — | — | — |

## Sheet totals

- 30 technology cards + 2 backs = 32 cells. No unreadable cells.
- 20 unique titles. 10 titles have exactly 2 copies on the sheet (Language School, Telegram, Telephone, Newspaper, Exposition, Ball, Gaslight, University, Letterpress Printing, Postal System); 10 titles have 1 copy (Station, Winery, Electrical Light, Mining Technology, Textile Mill, Patent System, Stagecoach, Tram, Brickyard, Stock Market). Each duplicate pair matches in cost, country, and effect text.
- Country distribution (cards / unique titles): USA 7/4, Great Britain 9/6, Netherlands 5/3, Germany 4/3, France 5/4.
- Cost distribution (unique titles): cost 3 x13 titles... breakdown: 3 = Language School, Ball, Gaslight, University, Letterpress Printing, Postal System, Winery, Mining Technology, Patent System, Stagecoach, Brickyard, Stock Market (12 titles); 4 = Station, Electrical Light, Textile Mill (3); 5 = Newspaper, Exposition, Tram (3); 6 = Telegram, Telephone (2).

## Low-confidence notes

- None; all names, costs, effect texts, and (upside-down) country bands were legible at native resolution.
- Note for the manifest: the sheet contains 30 tech cards, not 20 — the 10 duplicated titles physically exist twice in the TTS asset.
