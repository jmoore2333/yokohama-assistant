# Blind re-transcription: Order cards

**Source:** `tts-assets/organized/cards/order_cards_8x5.jpg` (4240x3996, 8 cols x 5 rows, cell 530x799)
**Method:** each cell cropped at native resolution with ImageMagick and read individually. This transcription was produced WITHOUT opening any YAML file or prior verification report.
**Date:** 2026-07-16

## Legend

- Position `(row,col)`, 1-based, row-major from top-left.
- Requirements icons: `fish` (blue square, fish), `tea` (green square, leaf sprig), `silk` (white square, tied bale), `copper` (red square, brown ingot). On every card fish/tea appear on the upper requirements row, silk/copper on the lower row.
- Rewards (bottom strip): `vp(N)` = blue star with number; `yen(N)` = silver coin with number; `imports` = yellow crate stamped IMPORTS; `shophouse_build` = house icon with red up-arrow; `assistants(N)` = N + yellow meeple with red down-arrow onto a gray pad.
- `#N` = small card index printed in the bottom-right corner of each card (1-36; all 36 appear exactly once across the sheet — good internal consistency check).

## Cards

| Pos | # | Country | Requirements | Rewards |
|---|---|---|---|---|
| (1,1) | 16 | United States of America | 3 fish, 2 tea, 1 copper | vp(8), imports |
| (1,2) | 25 | Germany | 1 tea, 3 silk | vp(4), imports |
| (1,3) | 14 | Germany | 2 tea, 2 silk, 1 copper | vp(6), yen(2) |
| (1,4) | 11 | Netherlands | 1 fish, 1 tea, 1 silk | vp(5) |
| (1,5) | 12 | United States of America | 3 fish, 1 tea, 2 silk | vp(9), yen(1) |
| (1,6) | 36 | Germany | 2 tea, 4 silk | vp(5), yen(2), shophouse_build |
| (1,7) | 17 | Germany | 2 fish, 2 silk, 1 copper | vp(8), yen(1) |
| (1,8) | 8 | Netherlands | 1 tea, 1 silk, 1 copper | vp(3), imports |
| (2,1) | 9 | Germany | 1 fish, 3 silk | vp(2), yen(1), imports |
| (2,2) | 4 | Germany | 1 fish, 1 tea, 2 silk | vp(5), shophouse_build |
| (2,3) | 33 | Germany | 3 tea, 3 silk | vp(11) |
| (2,4) | 6 | Netherlands | 2 fish, 1 tea, 1 copper | vp(2), yen(1), imports |
| (2,5) | 35 | United States of America | 4 fish, 2 silk | vp(7), yen(2) |
| (2,6) | 22 | United States of America | 4 fish, 1 copper | vp(8) |
| (2,7) | 27 | Great Britain | 2 fish, 2 copper | vp(1), shophouse_build, yen(2) |
| (2,8) | 32 | Great Britain | 1 fish, 1 tea, 2 copper | vp(5), imports |
| (3,1) | 13 | Great Britain | 2 fish, 1 silk, 2 copper | vp(8), imports |
| (3,2) | 31 | United States of America | 3 fish, 1 copper | vp(2), assistants(2) |
| (3,3) | 3 | Great Britain | 1 fish, 2 copper | vp(4), yen(1) |
| (3,4) | 24 | Great Britain | 2 fish, 1 silk, 3 copper | vp(7), yen(2), imports |
| (3,5) | 15 | Great Britain | 1 tea, 1 silk, 3 copper | vp(9), imports |
| (3,6) | 28 | United States of America | 4 fish, 1 silk | vp(3), imports, shophouse_build |
| (3,7) | 10 | Great Britain | 1 fish, 1 tea, 2 copper | vp(4), assistants(2) |
| (3,8) | 20 | Great Britain | 2 tea, 1 silk, 2 copper | vp(8), imports |
| (4,1) | 1 | United States of America | 2 fish, 1 tea | vp(1), shophouse_build |
| (4,2) | 18 | United States of America | 3 fish, 2 copper | vp(6), imports |
| (4,3) | 23 | France | 3 tea, 2 silk, 1 copper | vp(9), imports |
| (4,4) | 34 | France | 3 tea, 2 copper | vp(9), imports |
| (4,5) | 26 | France | 1 fish, 3 tea | vp(1), yen(2) |
| (4,6) | 7 | France | 3 tea, 1 copper | vp(3), imports |
| (4,7) | 19 | Netherlands | 1 fish, 1 tea, 1 silk, 1 copper | vp(8) |
| (4,8) | 5 | France | 2 tea, 1 copper | vp(1), imports |
| (5,1) | 30 | France | 3 tea, 2 silk | vp(4), yen(2) |
| (5,2) | 21 | Netherlands | 2 fish, 2 tea, 2 silk | vp(11) |
| (5,3) | 2 | France | 1 fish, 2 tea | vp(3) |
| (5,4) | 29 | France | 4 tea, 1 silk | vp(8) |
| (5,5)-(5,8) | — | CARD BACK (brown, red "ORDER" stamp) x4 | — | — |

## Sheet totals

- 36 order cards + 4 backs = 40 cells. No unreadable cells.
- Country distribution (as transcribed): USA 8, Germany 7, Netherlands 5, Great Britain 8, France 8.
- Corner indices 1-36 all present exactly once.

## Low-confidence notes

- (5,1) corner index is faint; read as `30` (only 30 remained unassigned among 1-36, and the glyph is consistent with 30).
- (5,3) corner index is very faint; read as `2` (deduction also gives 2 — the only unassigned index).
- (2,6) corner index read as `22`.
- All goods counts and reward icons were clearly legible at native resolution.
