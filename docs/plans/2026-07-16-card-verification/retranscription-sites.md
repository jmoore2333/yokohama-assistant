# Blind re-transcription: Building site cards

**Source:** `tts-assets/organized/cards/building_site_cards_5x5.jpg` (2650x3996, 5 cols x 5 rows, cell 530x799)
**Method:** each cell cropped at native resolution with ImageMagick and read individually. Produced WITHOUT opening any YAML file or prior verification report.
**Date:** 2026-07-17

## Card layout (identical on all 24 cards)

Six rounded zones in a 2-column grid:

- **TL** (top-left): large blue star with number — on every card this is **1**.
- **TR** (top-right): a reward cell (star VP, goods, coin+star combo, or build icon).
- **ML** (middle-left): always empty (plain tan).
- **MR** (middle-right): a second reward cell.
- **Bottom** (spans width, star at left): large blue star over a translucent right-pointing arrow watermark — the trading-house zone; number 5-9.

Reading of the structure: TL = VP for the 1st shophouse space, TR = reward printed on the 2nd (shophouse) space, MR = reward on the 3rd space, Bottom = trading-house VP. (Semantics to be confirmed against rulebook/YAML in the diff step; positions are what was transcribed.)

## Icon legend

- `vp(N)` = blue star with number (small stars inside reward cells are written the same way).
- `yen(N)` = silver coin.
- `fish(N)` / `tea(N)` / `silk(N)` / `copper(N)` = goods squares as on order cards.
- `wild(N)` = square split into 4 colored triangles (purple/red/green/white) = any-good.
- `shophouse_build(N)` = house icon with red up/right arrow.
- `assistants(N)` = yellow meeple + red arrow onto gray pad (arrow orientation varies; same icon family).
- Combined cells (two icons in one cell) written with `+`.

## Cards

| Pos | TL | TR | MR | Bottom (trading house) |
|---|---|---|---|---|
| (1,1) | vp(1) | vp(3) | fish(3) | vp(7) |
| (1,2) | vp(1) | fish(3) | fish(2) | vp(7) |
| (1,3) | vp(1) | vp(4) | tea(2) | vp(7) |
| (1,4) | vp(1) | shophouse_build(2) | tea(2) | vp(8) |
| (1,5) | vp(1) | silk(2) | copper(1) | vp(9) |
| (2,1) | vp(1) | copper(2) | silk(2) | vp(9) |
| (2,2) | vp(1) | wild(2) | wild(1) | vp(7) |
| (2,3) | vp(1) | wild(2) | wild(1) | vp(6) |
| (2,4) | vp(1) | vp(4) | assistants(2) | vp(8) |
| (2,5) | vp(1) | shophouse_build(2) | assistants(2) | vp(6) |
| (3,1) | vp(1) | shophouse_build(2) | assistants(2) | vp(7) |
| (3,2) | vp(1) | shophouse_build(2) | assistants(2) | vp(5) |
| (3,3) | vp(1) | vp(3) + yen(1) | assistants(2) | vp(6) |
| (3,4) | vp(1) | shophouse_build(3) | shophouse_build(2) | vp(5) |
| (3,5) | vp(1) | vp(3) | shophouse_build(2) | vp(8) |
| (4,1) | vp(1) | vp(2) + yen(1) | shophouse_build(2) | vp(8) |
| (4,2) | vp(1) | vp(2) + yen(1) | vp(1) + yen(1) | vp(9) |
| (4,3) | vp(1) | vp(2) + yen(1) | vp(2) | vp(8) |
| (4,4) | vp(1) | vp(4) | yen(2) | vp(9) |
| (4,5) | vp(1) | vp(4) | yen(2) | vp(6) |
| (5,1) | vp(1) | vp(3) | vp(1) + yen(2) | vp(5) |
| (5,2) | vp(1) | vp(4) | vp(1) + yen(2) | vp(5) |
| (5,3) | vp(1) | vp(4) | vp(3) | vp(9) |
| (5,4) | vp(1) | vp(3) | vp(2) | vp(6) |
| (5,5) | CARD BACK (brick-wall pattern, white outline) | — | — | — |

## Sheet totals

- 24 building site cards + 1 back = 25 cells. No unreadable cells.
- TL is vp(1) on all 24 cards.
- Trading-house (bottom) VP distribution: 5 x4, 6 x5, 7 x5, 8 x5, 9 x5. (Corrected 2026-07-17: an earlier draft of this line miscounted as 5 x3 / 6 x6; the per-card table above is authoritative and gives 5 x4 / 6 x5.)

## Low-confidence notes

- (1,5) MR ingot is drawn pale tan on a red square; icon family matches copper (red square) rather than silk (white square) — recorded as copper(1).
- Assistant-icon arrow orientation: on (2,4),(2,5),(3,1),(3,2),(3,3) the red arrow points left onto the gray pad (order-card version points down). Treated as the same "gain/place assistants" icon; noting in case the YAML distinguishes them.
