# Yokohama content manifest (rulebook ground truth)

**Sources:** `analysis-1/00-yokohama-rulebook.pdf` pages 1-4 (Contents of the Game, Game Setup) and 14-16 (Achievement Cards, Technology Cards); Rust engine source under `crates/yokohama-core/src/` for the code cross-checks. Compiled 2026-07-17 as part of the three-way card verification (see `retranscription-*.md` beside this file). This baseline was written BEFORE opening any `yokohama-kb` YAML file.

## 1. Component counts (rulebook "Contents of the Game", p.1-3)

| Component | Count | Detail |
|---|---|---|
| Order cards | **36** | country icons: English, American, French, German, Dutch |
| Technology cards | **30** | country icon, required Industry value, effect |
| Building Site cards | **24** | per card: 4 shophouse building sites w/ rewards + 1 trading-house site w/ reward |
| 5-Power tokens | **20** | shuffled face-down; 1 per Area Board at setup; rest to box |
| Foreign Mercenaries | **10** | 3 English, 2 American, 2 French, 2 German, 1 Dutch |
| Imported Goods tokens | 23 | |
| Trade goods tokens | 30 Copper, 30 Raw Silk, 40 Tea, 40 Seafood | |
| Money | 52 x 1-Yen, 16 x 3-Yen | |
| Station token | 1 | |
| Achievement cards | 12 | 4 each of A, B, C; 1 of each drawn at setup |
| Area Boards | 18 | 2 category types: Production, Commercial |
| Management Boards | 6 | Port A, Port B, Laboratory A, Laboratory B, Church, Customs |
| Per player | 1 President, 20 Assistants, 1 VP counter, 8 Shophouses, 4 Trading Houses, 1 Warehouse tile | 4 colours |
| Other | 4 x 100-point counters, VP track, 4 player summaries, Start Player card | |

Order-country distribution: **not stated in the rulebook** (only the mercenary distribution is). The sprite-sheet transcription gives USA 8, Germany 7, Netherlands 5, Great Britain 8, France 8 (see `retranscription-orders.md`) — **CONFIRMED against the physical box by Jeremy, 2026-07-21: NL 5, FR 8, US 8, GB 8, DE 7 — exact match.** Tech-card country distribution ALSO box-confirmed same session: GB 9, US 7, NL 5, FR 5, DE 4 (copy-weighted) — exact match to the YAML. Both decks' country data is now verified sprite × YAML × physical box.

## 2. Setup facts (rulebook p.3-4)

- **Order cards removed by player count (setup step 12): 2P = 16, 3P = 5, 4P = 0.**
- After removal: Port A (and B) Order Form Board spaces are filled face-up from the deck; then each player draws 2 orders, secretly keeps 1, returns the other to the box (this kept card is the hidden starting order).
- Technology cards: shuffled; Laboratory A (and B) technology spaces filled face-up from the deck (setup step 11). Slot counts per board are not stated numerically in the setup text.
- **Starting money (setup step 9): start player 3 Yen, all other players 4 Yen.**
- **Assistants per player: 20 total — 8 in hand, 12 on the Warehouse tile.**
- Shophouses per player: 8 — 2 in hand, 6 on Warehouse tile. Trading Houses: 4, all on Warehouse tile. Plus 1 of each trade good (Copper, Raw Silk, Tea, Seafood) per player.
- Boards removed: 2P — Lab B, Port B, Chinatown, Canal, 1x Copper Mine, 1x Silk Mill, 1x Tea Plantation, 1x Fishing Port (management: Lab B, Port B); 3P — Lab B, 1x Copper Mine, 1x Silk Mill, 1x Tea Plantation (management: Lab B); 4P — none.
- Building Site cards: shuffled, 1 face-up per Area Board, **remainder returned to the box** (so 18 of 24 used in a 4P game).
- Achievement rewards: A = 8 VP first / 6 VP later; B = 9/7; C = 10/8.

## 3. Resolution of the two code conflicts

### 3a. Order cards removed (2P)

- Rulebook: **16** (2P), 5 (3P), 0 (4P).
- Code `GameConfig::order_cards_to_remove()` (`crates/yokohama-core/src/types/state.rs:84-91`): returns **10**/5/0.
- **Ruling: the code's 2-player value is WRONG; must be 16.** (The December-2025 audit figure of 16 relayed in the task brief agrees with the rulebook.) Follow-on: `game/setup.rs:90` computes `order_deck_remaining: 24 - order_cards_to_remove()`; the base 24 (= 36 minus 12 assumed pre-placed on port boards) and the 2P arithmetic need revisiting together (in 2P, Port B is removed, so fewer orders are pre-placed; `24 - 16 = 8` would be wrong if the base still assumes both ports).

### 3b. Starting money

- Rulebook: start player **3 Yen**, others **4 Yen**.
- Code `PlayerState::new` (`crates/yokohama-core/src/types/player.rs:218`): 3 / 4 — **correct**.
- Code `GameConfig::starting_money` (`crates/yokohama-core/src/types/state.rs:79-81`): 7 / 8 — **wrong**, and it wins: `create_players` (`game/setup.rs:14`) overwrites the correct value with it. Doc comment there ("First player: 7 Yen, Others: 8 Yen") is also wrong.
- **Ruling: 3/4 per rulebook; fix `GameConfig::starting_money` and its tests (`game/setup.rs:124-125`).**

### 3c. Assistants (cross-check requested by plan)

- Rulebook: 8 in hand + 12 on warehouse. Code `AssistantState::new` (`types/player.rs:32-41`): 8 + 12 — **correct**. (Shophouse 2+6 and Trading House 4 also match.)

## 4. Technology card names (rulebook p.15-16 rules text)

20 names listed; costs are NOT given in the rulebook text (Industry values are printed on the cards): Gaslight, Station, Mining Technology, Spinning Mill, Stagecoach, Exposition, Ball, Language School, Letterpress Printing, Winery, Postal System, Newspaper, Electrical Light, University, Telegram, Telephone, Brickyard, System of Stock, Patent System, Tram.

Naming variants (rulebook vs printed cards, same effects):

- rulebook **Spinning Mill** = card **Textile Mill** (once/turn: discard 1 Raw Silk, gain 2 Yen)
- rulebook **System of Stock** = card **Stock Market** (start of turn: if 1 Yen or less, refill hand to 2 Yen)
- rulebook Telephone text says "Recovery Step"; the card says "Collection Step" (same step; translation drift). Cards say "Worker(s)" where the rulebook says "Assistant pawn(s)".

Card counts by title (from sprite sheet, matches the rulebook total of 30): 2 copies each of Language School, Telegram, Telephone, Newspaper, Exposition, Ball, Gaslight, University, Letterpress Printing, Postal System; 1 copy each of Station, Winery, Electrical Light, Mining Technology, Textile Mill/Spinning Mill, Patent System, Stagecoach, Tram, Brickyard, System of Stock/Stock Market.

Engine note (not a card fact): `tech_deck_remaining: 20` and `order_deck_remaining: 24 - removal` in `game/setup.rs:90-91` encode assumptions (10 techs / 12 orders pre-placed on boards) that the rulebook does not state numerically; verify lab/port slot counts from board scans before Task 18.

## 6. Three-way diff (Step 3)

Comparison of `analysis-1/yokohama-kb/07..09-*.yaml` (opened only after sections 1-4 and the three retranscription files were written) x blind re-transcription x rulebook.

**Notation.** Positions are 1-based `(row,col)` as in the retranscription files; the YAML's 0-based `position` and `id` are given in the YAML column. Goods: `se` seafood (blue fish), `te` tea (green leaf), `si` raw silk (white bale), `cu` copper (red ingot). Rewards: `vp/yen/imp` + `house` (house icon w/ red up-arrow; YAML calls it `warehouse`) + `asst(N)` (yellow assistant + arrow onto pad). The rulebook does not print per-card data for orders or building sites, so that column is `box?` (= physical box must confirm at the human gate); for tech effects the rulebook rules text (p.15-16) is quoted as ground truth.

**Vocabulary alignment (not counted as mismatches):** YAML `warehouse: 1` = the house-with-arrow icon = my `house`; where the card instead shows the assistant icon (a different icon), YAML's `warehouse: 1` IS counted as a mismatch. YAML `import: 1` = IMPORTS crate = my `imp`.

### 6a. Order cards — 33 of 36 entries mismatch (3 country, 32 requirements, 10 rewards)

Fully matching: `order_031` (4,7), `order_033` (5,1), `order_036` (5,4). Everything else differs as follows (`#N` = printed corner index from the sprite):

| # | Pos | YAML (id: country; reqs; rewards) | Re-transcription | Rulebook/box | Proposed ruling |
|---|---|---|---|---|---|
| 16 | (1,1) | 001: US; 3cu 1si 2te; vp8 imp | US; 3se 2te 1cu; vp8 imp | box? | adopt re-transcription (YAML read seafood as copper, copper as silk) |
| 25 | (1,2) | 002: DE; 1te 3se; vp0 "movement 1" | DE; 1te 3si; vp4 imp | box? | adopt re-transcription (no movement icon exists on any order card) |
| 14 | (1,3) | 003: DE; 1cu 1si 2te; vp6 yen2 | DE; 2te 2si 1cu; vp6 yen2 | box? | adopt re-transcription (silk 2, not 1) |
| 11 | (1,4) | 004: NL; 1cu 1te 1se; vp5 | NL; 1se 1te 1si; vp5 | box? | adopt re-transcription (silk, not copper) |
| 12 | (1,5) | 005: US; 3cu 1te 2se; vp9 | US; 3se 1te 2si; vp9 yen1 | box? | adopt re-transcription (+1 yen reward missing in YAML) |
| 36 | (1,6) | 006: US; 2te 4se; vp5 yen2 house | DE; 2te 4si; vp5 yen2 house | box? | adopt re-transcription (country DE; silk not seafood) |
| 17 | (1,7) | 007: DE; 2cu 1si 2se; vp8 yen1 | DE; 2se 2si 1cu; vp8 yen1 | box? | adopt re-transcription |
| 8 | (1,8) | 008: NL; 1si 1te; vp3 | NL; 1te 1si 1cu; vp3 imp | box? | adopt re-transcription (copper req + imports reward missing) |
| 9 | (2,1) | 009: DE; 1cu 3se; vp2 yen1 imp | DE; 1se 3si; vp2 yen1 imp | box? | adopt re-transcription |
| 4 | (2,2) | 010: DE; 1si 1te 2se; vp5 house | DE; 1se 1te 2si; vp5 house | box? | adopt re-transcription |
| 33 | (2,3) | 011: DE; 3te 3se; vp11 | DE; 3te 3si; vp11 | box? | adopt re-transcription |
| 6 | (2,4) | 012: NL; 1cu 2te 1se; vp2 yen1 imp | NL; 2se 1te 1cu; vp2 yen1 imp | box? | adopt re-transcription (2 seafood 1 tea, not 2 tea 1 seafood) |
| 35 | (2,5) | 013: US; 4te 1se; vp7 yen2 | US; 4se 2si; vp7 yen2 | box? | adopt re-transcription |
| 22 | (2,6) | 014: US; 4te 2se; vp8 | US; 4se 1cu; vp8 | box? | adopt re-transcription |
| 27 | (2,7) | 015: NL; 2si 2te; vp1 house yen2 | GB; 2se 2cu; vp1 house yen2 | box? | adopt re-transcription (country GB) |
| 32 | (2,8) | 016: GB; 1si 1te; vp5 imp | GB; 1se 1te 2cu; vp5 imp | box? | adopt re-transcription (2 copper missing) |
| 13 | (3,1) | 017: GB; 2cu 1se; vp8 imp | GB; 2se 1si 2cu; vp8 imp | box? | adopt re-transcription |
| 31 | (3,2) | 018: GB; 3cu 1se; vp2 yen2 warehouse1 | US; 3se 1cu; vp2 asst(2) | box? | adopt re-transcription (country US; reward is 2 assistants, no yen) |
| 3 | (3,3) | 019: GB; 1cu 2se; vp4 yen1 | GB; 1se 2cu; vp4 yen1 | box? | adopt re-transcription |
| 24 | (3,4) | 020: GB; 2cu 1se; vp7 yen2 imp | GB; 2se 1si 3cu; vp7 yen2 imp | box? | adopt re-transcription |
| 15 | (3,5) | 021: GB; 1te 1se; vp9 imp | GB; 1te 1si 3cu; vp9 imp | box? | adopt re-transcription |
| 28 | (3,6) | 022: US; 4cu 1se; vp3 warehouse1 yen1 | US; 4se 1si; vp3 imp house | box? | adopt re-transcription (imports not yen) |
| 10 | (3,7) | 023: GB; 1cu 1si 2se; vp4 yen2 warehouse1 | GB; 1se 1te 2cu; vp4 asst(2) | box? | adopt re-transcription (2 assistants, no yen) |
| 20 | (3,8) | 024: GB; 2si 1se; vp8 yen2 | GB; 2te 1si 2cu; vp8 imp | box? | adopt re-transcription (imports not yen) |
| 1 | (4,1) | 025: US; 2cu 1si; vp1 warehouse1 | US; 2se 1te; vp1 house | box? | adopt re-transcription |
| 18 | (4,2) | 026: US; 3cu 2se; vp6 imp | US; 3se 2cu; vp6 imp | box? | adopt re-transcription |
| 23 | (4,3) | 027: FR; 2cu 1si 3te; vp9 imp | FR; 3te 2si 1cu; vp9 imp | box? | adopt re-transcription (2 silk 1 copper, not 1 silk 2 copper) |
| 34 | (4,4) | 028: FR; 2cu 3te; vp9 | FR; 3te 2cu; vp9 imp | box? | adopt re-transcription (imports reward missing in YAML) |
| 26 | (4,5) | 029: FR; 3si 1te; vp1 yen2 | FR; 1se 3te; vp1 yen2 | box? | adopt re-transcription |
| 7 | (4,6) | 030: FR; 1si 3te; vp3 | FR; 3te 1cu; vp3 imp | box? | adopt re-transcription |
| 5 | (4,8) | 032: FR; 2te 1se; vp1 | FR; 2te 1cu; vp1 imp | box? | adopt re-transcription |
| 21 | (5,2) | 034: NL; 2cu 2si 2te; vp11 | NL; 2se 2te 2si; vp11 | box? | adopt re-transcription (seafood, not copper) |
| 2 | (5,3) | 035: FR; 1te 2se; vp3 | FR; 1se 2te; vp3 | box? | adopt re-transcription |

Resulting country distribution: YAML body has US 8, DE 6, NL 6, FR 8, GB 8 (its own summary comment claims US 10 / GB 5, contradicting its body); re-transcription gives **US 8, DE 7, NL 5, GB 8, FR 8**.

### 6b. Technology cards — 8 of 20 YAML entries mismatch (4 country, 5 effect) + structure

| Card | Pos | YAML says | Re-transcription says | Rulebook (p.15-16) | Proposed ruling |
|---|---|---|---|---|---|
| Telephone | (1,3) | country NL | country US (both copies) | — | US |
| Ball | (1,6) | country DE | country GB (both copies) | — | GB |
| Gaslight | (1,7) | country GB | country FR (both copies) | — | FR |
| Textile Mill | (2,7) | country FR | country GB | — | GB |
| Textile Mill | (2,7) | "exchange 1 Fish for 2 Silk" | "Once Per Turn: [silk] -> [yen(2)]" | "Spinning Mill: Once per turn, you may discard 1 Raw Silk to gain 2 Yen" | card+rulebook agree: 1 Silk -> 2 Yen; YAML effect is wrong |
| Letterpress Printing | (2,2) | "gain 1 VP" on tech gain | coin icon = 1 Yen | "you gain an additional 1 Yen" | 1 Yen, not 1 VP |
| Electrical Light | (2,5) | "gain 3 VP and 1 Yen" | star(3) only | "you gain 3 additional Victory points" | 3 VP only; phantom yen removed |
| Patent System | (2,8) | "gain 2 VP and 1 Yen" | star(2) only | "you gain an additional 2 Victory points" | 2 VP only; phantom yen removed |
| Exposition | (1,5) | "Worth 10 VP at end of game" | star(10), no trigger text | "Immediately gain 10 Victory points" | immediate 10 VP (engine-relevant: not end-game scoring) |

Structural: the YAML lists 20 unique cards and relegates duplicates to comments; the physical deck has **30 cards** (10 titles x2 + 10 titles x1 — see section 4). The duplicate-position comments in the YAML are also partly wrong: the 2nd copies of Telephone/Ball/Gaslight match their firsts (US/GB/FR), and the 2nd Letterpress Printing at (4,5) is Netherlands, not GB. Costs (all 20) and the remaining 12 entries' countries/effects match the sprites. Rulebook cross-refs the YAML omits (fidelity, optional): Telegram "pay the usual cost if another President occupies the adjacent Area"; Stagecoach/Postal System "(unless you also own the ...)" clauses; Gaslight/Electrical Light "only from the turn implemented"; Winery/Brickyard "...and place an Assistant pawn".

### 6c. Building site cards — 24 of 24 YAML entries mismatch (structural mis-model + data)

The YAML forces a VP number into all four shophouse slots (`shophouse_vp: [a,b,c,d]`). The actual cards (confirmed by rulebook p.2: "4 Building Sites for Shophouses, and corresponding rewards" + 1 trading-house site) have four shophouse SPACES whose printed contents are: one space with vp(1) (top-left on every card), one or two reward spaces (VP, goods, wild, yen, combos, house, or assistants), and one or two blank spaces (middle-left is blank on all 24 cards). No card has four VP numbers. Every YAML entry therefore mis-states the slot data; most also carry phantom `copper`/`yen` bonuses (the coin+star combo cells and assistant icons were misread), and `site_003`'s trading-house VP is wrong (9 vs 7). The single reliable YAML field is trading-house VP (23 of 24 correct).

| Pos | YAML (site: shophouse_vp; bonuses; TH) | Re-transcription (spaces TL/TR/ML/MR; TH) | Rulebook/box | Proposed ruling |
|---|---|---|---|---|
| (1,1) | 001: [1,3,3,3]; fish@1,3; TH7 | vp1 / vp3 / blank / se(3); TH7 | box? | adopt re-transcription |
| (1,2) | 002: [3,1,2,2]; fish@1,3; TH7 | vp1 / se(3) / blank / se(2); TH7 | box? | adopt re-transcription |
| (1,3) | 003: [3,4,2,2]; tea@2,4; TH9 | vp1 / vp4 / blank / te(2); TH**7** | box? | adopt re-transcription (TH VP is 7) |
| (1,4) | 004: [2,2,2,2]; copper@2; TH8+import | vp1 / house(2) / blank / te(2); TH8 | box? | adopt re-transcription (no copper, no TH import; arrow is a watermark) |
| (1,5) | 005: [2,1,1,1]; fish@1 copper@2; TH9+copper | vp1 / si(2) / blank / cu(1); TH9 | box? | adopt re-transcription |
| (2,1) | 006: [2,1,2,2]; copper@1; TH9 | vp1 / cu(2) / blank / si(2); TH9 | box? | adopt re-transcription |
| (2,2) | 007: [2,1,2,2]; any@1; TH7 | vp1 / wild(2) / blank / wild(1); TH7 | box? | adopt re-transcription |
| (2,3) | 008: [2,2,1,1]; silk@2 tea@3; TH6 | vp1 / wild(2) / blank / wild(1); TH6 | box? | adopt re-transcription (wild squares, not silk/tea) |
| (2,4) | 009: [2,4,2,2]; tea@2; TH8+copper | vp1 / vp4 / blank / asst(2); TH8 | box? | adopt re-transcription |
| (2,5) | 010: [2,1,1,1]; copper@3; TH6+import | vp1 / house(2) / blank / asst(2); TH6 | box? | adopt re-transcription |
| (3,1) | 011: [1,2,2,2]; copper@2 yen@4; TH7 | vp1 / house(2) / blank / asst(2); TH7 | box? | adopt re-transcription |
| (3,2) | 012: [2,1,2,2]; copper@2 yen@4; TH5 | vp1 / house(2) / blank / asst(2); TH5 | box? | adopt re-transcription |
| (3,3) | 013: [2,2,1,2]; copper@2 yen@3; TH6+"1" | vp1 / vp3+yen1 / blank / asst(2); TH6 | box? | adopt re-transcription |
| (3,4) | 014: [3,1,2,2]; copper@2 yen@4; TH5 | vp1 / house(3) / blank / house(2); TH5 | box? | adopt re-transcription |
| (3,5) | 015: [3,1,2,2]; copper@2; TH8+import | vp1 / vp3 / blank / house(2); TH8 | box? (rulebook p.2 example card shows exactly {1, 3, houses, blank; TH8}) | adopt re-transcription |
| (4,1) | 016: [1,1,2,2]; copper@2 yen@3; TH8 | vp1 / vp2+yen1 / blank / house(2); TH8 | box? | adopt re-transcription |
| (4,2) | 017: [1,1,2,1]; copper@2 yen@4; TH9 | vp1 / vp2+yen1 / blank / vp1+yen1; TH9 | box? | adopt re-transcription |
| (4,3) | 018: [1,1,2,2]; yen@3; TH8 | vp1 / vp2+yen1 / blank / vp2; TH8 | box? | adopt re-transcription |
| (4,4) | 019: [1,1,2,2]; yen@3; TH9+yen | vp1 / vp4 / blank / yen(2); TH9 | box? | adopt re-transcription |
| (4,5) | 020: [1,4,2,2]; none; TH6+yen | vp1 / vp4 / blank / yen(2); TH6 | box? | adopt re-transcription |
| (5,1) | 021: [1,1,2,2]; yen@3; TH5 | vp1 / vp3 / blank / vp1+yen2; TH5 | box? | adopt re-transcription |
| (5,2) | 022: [1,1,4,2]; yen@3; TH5 | vp1 / vp4 / blank / vp1+yen2; TH5 | box? | adopt re-transcription |
| (5,3) | 023: [1,1,4,3]; none; TH9 | vp1 / vp4 / blank / vp3; TH9 | box? | adopt re-transcription |
| (5,4) | 024: [1,1,3,2]; yen@3; TH6 | vp1 / vp3 / blank / vp2; TH6 | box? | adopt re-transcription |

### 6d. Global / cross-file

| Item | YAML/code says | Re-transcription says | Rulebook says | Proposed ruling |
|---|---|---|---|---|
| Tech deck size | YAML: 20 cards; engine `tech_deck_remaining: 20` | 30 cards (10 titles x2) | 30 Technology cards | model 30 physical cards (add a `copies:` field) |
| Order country summary | YAML comment: US 10, DE 7, NL 6, FR 8, GB 5 | US 8, DE 7, NL 5, GB 8, FR 8 | not stated | fix summary after adjudication; confirm vs box |
| Order removal (2P) | code 10 (`state.rs:84-91`) | — | **16** (setup step 12) | 16 (see section 3a) |
| Starting money | code `GameConfig` 7/8 (`state.rs:79-81`) | — | **3 / 4** (setup step 9) | 3/4 (see section 3b) |

### 6e. Does country + requirements uniquely identify an order card?

**No.** Great Britain #10 at (3,7) and Great Britain #32 at (2,8) both require 1 seafood + 1 tea + 2 copper; they differ only in rewards (#10: 4 VP + 2 assistants; #32: 5 VP + imports). The other 34 cards are unique on (country, requirements). Display codes therefore need an extra discriminator — the printed corner index (1-36, unique across the deck, present on every card) is the natural key.

## 7. Status per component

| Component | Status |
|---|---|
| Orders | VALIDATED (adjudicated 2026-07-17: re-transcription adopted; physical-box exceptions patchable) — 33 corrections applied; re-keyed to printed corner index 1-36 |
| Techs | VALIDATED (adjudicated 2026-07-17: re-transcription adopted; physical-box exceptions patchable) — 8 corrections applied; `copies:` field models the 30-card deck |
| Building sites | VALIDATED (adjudicated 2026-07-17: re-transcription adopted; physical-box exceptions patchable) — re-modeled to TL/TR/ML/MR spaces + trading_house_vp; site_003 TH VP corrected to 7 |
| Global counts / setup values | VALIDATED (rulebook-backed); code fixes queued (2P removal 16, starting money 3/4) |
