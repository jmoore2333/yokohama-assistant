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
13. **Count the area tiles**: after removals the table should hold 10 / 14 / 18 tiles at 2P/3P/4P (2P removes Lab B, Port B, Chinatown, Canal + one of each production area; 3P removes Lab B + one copper/silk/tea). The KB's old component row said 12/15/18 — corrected 2026-07-17; the removal lists and the engine both say 10/14/18, but the physical count is the arbiter.
14. **Building Step power gate (added 2026-07-17, final-review finding C3)**: confirm the box/rulebook prints NO separate (higher) power requirement for Trading House vs Shophouse — both require the same 4+ Power action, per the rulebook text and the p.7 Seiji example. The engine previously (wrongly) gated Trading House at 5+; fixed in code, but flagging here in case some edition variant genuinely differs from this printing.

## Added 2026-07-18 (choice-encoding design) — Priority-1-equivalent, feeds imminent implementation

15. **5-Power tokens — count all 20 by face.** The engine's pool: 10× "1 Imported Good", 2× "2 Copper", 2× "2 Silk", 2× "3 Seafood", 2× "3 Tea", 2× "3 Yen" — **no Movement token, no VP token**. The project's rules notes say a Movement token exists (moves assistants, not the President). If any Movement (or other unlisted) face exists: how many copies, and exactly what the face shows (how many assistant-moves?). This decides engine data + the live reveal menu (plan Tasks 3/11).
16. **Port board power-5 badge**: the printed "3 VP" star at the 5-power position — plus table memory: did a 5-power Port action with NO card taken (full hand) still score the 3 VP at your table? Decides the +3-VP-on-action adjudication (plan Task 8, design §5.3-1).
17. **Canal crossing** (3–4p boards + rulebook p.6): confirm a President crosses straight across the Canal tile between the two flanking areas for 1 Yen — and that there is no diagonal/other crossing. Decides the adjacency derivation (plan Task 9, design §5.6).

## How to report

Anything that disagrees: note the component + the mapped-reference row (section + id). The YAML in `analysis-1/yokohama-kb/07..09-*.yaml` gets patched, `cargo run -p gen-data` regenerates the engine data, and the fuzz gate re-runs — that's the whole correction loop.
