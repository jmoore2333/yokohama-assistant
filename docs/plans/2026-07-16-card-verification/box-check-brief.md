# Physical box-check brief (2026-07-17)

One sitting, box open, with `analysis-1/yokohama-kb/01-rules-reference-engine-mapped.md` beside you — that document holds every value the engine believes, laid out card-by-card and board-by-board. **Exceptions only: report any row that disagrees with a physical component; silence = confirmed.** Priorities below are ordered by how the engine got the value — image transcription first (highest residual risk), rulebook-derived last.

## Priority 1 — boards (values read from board ART, not the rulebook text)

Open the mapped reference's **Boards** section and compare each physical board:

1. **Port boards (A and B)**: exactly SIX card spaces, left→right zones LOW, LOW, MID, MID, HIGH, HIGH.
2. **Laboratory boards (A and B)**: exactly SIX tech slots; surcharge icons +1 on slots 3–4, +2 on slots 5–6 (slots 1–2 free).
3. **Customs board**: SEVEN spaces; printed costs 1,1,2,2,3,3,4; VP 5,4,10,9,16,15,22; bonus icons on spaces 1–4 = 2 warehouse-takes, 2 Yen, 1 warehouse-take, 1 Yen (spaces 5–7 no bonus). 2-player side/overlay: space 4 (9 VP) blocked by a printed dummy pawn.
4. **Church board**: EIGHT spaces, faith 4–11, VP 3,4,5,6,7,8,11,14; movement-icon riders 1,1,2,2,2,2,3,3. 2-player: faith-7 space blocked by a printed dummy pawn.
5. **Church faith-from-power strip**: LINEAR — 1,2,3,4,5 faith for power 1–5 (the rulebook's "2 Power: 1 Faith" was adjudicated a typo — the board is the arbiter; this one matters).
6. **Warehouse tile / Employment costs**: assistants free; shophouse take 2 Yen; trading-house takes printed 4, 5, 6, 7 in take order; tokens-per-power 0/1/2/3/4.
7. **Chinatown board**: nine exchange rates — sell copper/silk → 2 Yen, sell tea/seafood → 1 Yen; buy copper/silk ← 2 Yen, buy tea/seafood ← 1 Yen, buy imported good ← 4 Yen; "number of exchanges = power".

## Priority 2 — cards (blind re-transcription, adjudicated 2026-07-17; spot-check against the mapped tables)

8. **Order cards (36)**: the mapped reference lists all 36 by printed corner index with country + requirements + rewards. Full pass if you have the patience; otherwise at minimum the box-audit priority items (`box-audit.html` in this folder) plus any 5 random cards.
9. **Technology cards (30 physical / 20 titles)**: check the copy counts (10 titles have 2 copies), each title's industry cost + country icon; effects at a glance.
10. **Achievement cards (A/B/C)**: requirements and the first/subsequent VP splits.
11. **Building-site cards (24)**: per area tile — the four space rewards (TL/TR/ML/MR) and the trading-house VP. The engine re-modeled these wholesale from re-transcription, so a full-ish skim is worth it; site_003's TH VP (corrected to 7) specifically.

## Priority 3 — setup constants (rulebook-backed, lowest risk)

12. 2P removes 16 order cards (not 10); starting money 3 (start player) / 4 (others); per-player 8 assistants in hand + 12 warehouse, 2 shophouses hand + 6 warehouse, 4 trading houses warehouse; 1 of each trade good; draw 2 orders keep 1.

## How to report

Anything that disagrees: note the component + the mapped-reference row (section + id). The YAML in `analysis-1/yokohama-kb/07..09-*.yaml` gets patched, `cargo run -p gen-data` regenerates the engine data, and the fuzz gate re-runs — that's the whole correction loop.
