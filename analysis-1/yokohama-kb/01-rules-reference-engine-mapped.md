# Yokohama Rules Reference — Engine-Mapped Edition

> Generated: 2026-07-17, against branch `engine-completion` @ commit `b3c4646`.
> Companion to `01-rules-reference.md` (the prose rules KB). This document has two jobs:
> (1) map every implemented rule to the engine code that implements it, and (2) lay out the
> **complete** card/board data in tables for physical-box verification.

## Sources of truth and how to use this document

Every number in this document is transcribed — never re-derived from memory — from:

- `analysis-1/yokohama-kb/07-order-cards.yaml`, `08-tech-cards.yaml`, `09-building-sites.yaml`
  (the hand-verified YAML card data, three-way checked against sprite sheets, rulebook, and
  physical components as of 2026-07-17)
- `crates/yokohama-core/src/game/data.rs` — the **generated** Rust form of the three YAML files
  (`cargo run -p gen-data`; never hand-edited)
- The typed board/track models in `crates/yokohama-core/src/types/{cards,tracks,board}.rs`
- The game logic in `crates/yokohama-core/src/game/**/*.rs`

**YAML + `data.rs` are the engine's source of truth.** Where the prose KB
(`01-rules-reference.md`) disagrees with them, YAML/`data.rs` wins, and the disagreement is
called out with a **⚠ DISCREPANCY** note at the point it occurs, plus a roundup in
[§9](#9-discrepancies-and-engine-completeness-findings).

**To use this for a physical box check:** work through §2–§6 table by table with the box open.
Every row is a claim about a specific physical card or board space — if what you see in the box
doesn't match a row, that's a reportable mismatch (note the section, row, and what you actually
see). §9 additionally lists things this audit found that are worth Jeremy's attention even though
they aren't box-comparable (dead code, unwired features) — read that section even if the box
matches everything.

---

## 1. Order Cards — all 36

Sorted by printed corner index (`OrderCardId`, the number printed in the card's bottom-right
corner — this is the canonical identity; country+requirements does **not** uniquely identify a
card, see #10/#32 below). Requirements and rewards are transcribed from `data.rs`'s
`standard_orders()` (generated from `07-order-cards.yaml`). Countries: US = United States, DE =
Germany, NL = Netherlands, FR = France, GB = Great Britain.

`OrderRewards` also models `copper`/`silk`/`tea`/`seafood`/`any_trade_good` reward fields (for a
card that rewards raw goods or a wild pick) — **verified zero on all 36 standard cards**; no
column below because the column would be empty. "Warehouse" = the Warehouse icon (take 1 token,
by default an assistant, from Warehouse to hand — see the icon-semantics note under §2).
"Movement" = the Movement icon (move an assistant one hop; v1 no-op, see §9).

| ID | Country | Requirements | Rewards |
|----|---------|--------------|---------|
| 1 | US | 1 tea, 2 seafood | 1 VP, 1 Warehouse |
| 2 | FR | 2 tea, 1 seafood | 3 VP |
| 3 | GB | 2 copper, 1 seafood | 4 VP, 1 Yen |
| 4 | DE | 2 silk, 1 tea, 1 seafood | 5 VP, 1 Warehouse |
| 5 | FR | 1 copper, 2 tea | 1 VP, 1 Import |
| 6 | NL | 1 copper, 1 tea, 2 seafood | 2 VP, 1 Yen, 1 Import |
| 7 | FR | 1 copper, 3 tea | 3 VP, 1 Import |
| 8 | NL | 1 copper, 1 silk, 1 tea | 3 VP, 1 Import |
| 9 | DE | 3 silk, 1 seafood | 2 VP, 1 Yen, 1 Import |
| 10 | GB | 2 copper, 1 tea, 1 seafood | 4 VP, 2 Movement |
| 11 | NL | 1 silk, 1 tea, 1 seafood | 5 VP |
| 12 | US | 2 silk, 1 tea, 3 seafood | 9 VP, 1 Yen |
| 13 | GB | 2 copper, 1 silk, 2 seafood | 8 VP, 1 Import |
| 14 | DE | 1 copper, 2 silk, 2 tea | 6 VP, 2 Yen |
| 15 | GB | 3 copper, 1 silk, 1 tea | 9 VP, 1 Import |
| 16 | US | 1 copper, 2 tea, 3 seafood | 8 VP, 1 Import |
| 17 | DE | 1 copper, 2 silk, 2 seafood | 8 VP, 1 Yen |
| 18 | US | 2 copper, 3 seafood | 6 VP, 1 Import |
| 19 | NL | 1 copper, 1 silk, 1 tea, 1 seafood | 8 VP |
| 20 | GB | 2 copper, 1 silk, 2 tea | 8 VP, 1 Import |
| 21 | NL | 2 silk, 2 tea, 2 seafood | 11 VP |
| 22 | US | 1 copper, 4 seafood | 8 VP |
| 23 | FR | 1 copper, 2 silk, 3 tea | 9 VP, 1 Import |
| 24 | GB | 3 copper, 1 silk, 2 seafood | 7 VP, 2 Yen, 1 Import |
| 25 | DE | 3 silk, 1 tea | 4 VP, 1 Import |
| 26 | FR | 3 tea, 1 seafood | 1 VP, 2 Yen |
| 27 | GB | 2 copper, 2 seafood | 1 VP, 2 Yen, 1 Warehouse |
| 28 | US | 1 silk, 4 seafood | 3 VP, 1 Import, 1 Warehouse |
| 29 | FR | 1 silk, 4 tea | 8 VP |
| 30 | FR | 2 silk, 3 tea | 4 VP, 2 Yen |
| 31 | US | 1 copper, 3 seafood | 2 VP, 2 Movement |
| 32 | GB | 2 copper, 1 tea, 1 seafood | 5 VP, 1 Import |
| 33 | DE | 3 silk, 3 tea | 11 VP |
| 34 | FR | 2 copper, 3 tea | 9 VP, 1 Import |
| 35 | US | 2 silk, 4 seafood | 7 VP, 2 Yen |
| 36 | DE | 4 silk, 2 tea | 5 VP, 2 Yen, 1 Warehouse |

**Non-uniqueness note:** #10 and #32 are both GB, both require 1 seafood + 1 tea + 2 copper — they
are only distinguishable by printed corner index, which is exactly why `OrderCardId` is keyed on
that index rather than country+requirements (decision log, 2026-07-17).

**Per-country count (for box sorting):**

| Country | Count |
|---------|-------|
| US (United States) | 8 |
| DE (Germany) | 7 |
| NL (Netherlands) | 5 |
| GB (Great Britain) | 8 |
| FR (France) | 8 |
| **Total** | **36** |

**Engine code:** type = `types/cards.rs::OrderCard` / `OrderRewards` / `TradeGoodRequirement`;
data = `game/data.rs::standard_orders()` (generated from the YAML); registry =
`game/cards.rs::OrderRegistry`; fulfillment = `game/cards.rs::can_fulfill_order` /
`fulfill_order` (Additional Action Phase) and `apply_move.rs::apply_fulfill_order` (the move
actually applied — note there are two independent order-fulfillment code paths; see §9).

---

## 2. Technology Cards — 20 titles / 30 physical cards

`industry_cost` and `effect` are transcribed from `data.rs::standard_techs()`. "Copies" = physical
cards of that title. Country icon is for Foreign-Mercenary set collection, same as Order cards.

**⚠ Icon-semantics note carried over from the Order/Building-site data (decision log
2026-07-17):** the rulebook's "house with red up-arrow" icon is the **Warehouse** icon (take 1
token — by default an assistant — from the Warehouse tile to hand); the "meeple with arrow onto a
pad" icon is the **Movement** icon (move an assistant one hop, no President fee). The YAML source
files name these keys after their *visual appearance* (`shophouse_build` for the house icon,
`assistants` for the meeple icon) but the engine fields are named after their *rule meaning*
(`warehouse_tokens`, `movement`) — so `shophouse_build → warehouse_tokens` and `assistants →
movement`, which is the **reverse** of what the key names suggest at a glance. This bit the
project once already; double-check it before reporting an order/site-space mismatch.

| Title | Copies | Cost | Country | Effect (`TechEffect`) | What the engine does with it |
|---|---|---|---|---|---|
| Language School | 2 | 3 | US | `LanguageSchool` | Fulfilling an Order: +1 Yen (stacks — each copy re-fires independently in `fulfill_order`, no per-copy check) |
| Telegram | 2 | 6 | US | `Telegram` | Placement Option B: after 2 in one area, may place 1 more in an adjacent area (same fee rules) |
| Telephone | 2 | 6 | US | `Telephone` | Recovery: may leave 1 assistant behind in the action area |
| Newspaper | 2 | 5 | NL | `Newspaper` | Placement Option A: up to 4 areas instead of 3 |
| Exposition | 2 | 5 | DE | `Exposition` | Immediate +10 VP the instant the card is acquired (not end-game scoring) |
| Ball | 2 | 3 | GB | `Ball` | Fulfilling an Order: +2 VP |
| Gaslight | 2 | 3 | FR | `Gaslight` | Building a Shophouse: +2 VP |
| Station | 1 | 4 | GB | `Station` | Teleport-to-station movement is fully implemented — **but nothing ever places the token** (see §9); non-functional as shipped |
| University | 2 | 3 | GB | `University` | Designed: once/turn, pay 1 Yen → 1 assistant from warehouse to hand. **Not reachable — no `Move` variant invokes it** (§9) |
| Letterpress Printing | 2 | 3 | NL | `LetterPressPrinting` | Acquiring a (different) Technology card: +1 Yen per *other* owned copy |
| Postal System | 2 | 3 | GB | `PostalSystem` | Designed: may end movement in an opponent-occupied area. **Not reachable — the movement enumerator never consults tech** (§9); non-functional as shipped |
| Winery | 1 | 3 | FR | `Winery` | Church placement: +3 VP per *owned* copy (all copies, not just others') |
| Electrical Light | 1 | 4 | FR | `ElectricalLight` | Building a Trading House: +3 VP |
| Mining Technology | 1 | 3 | DE | `MiningTechnology` | +1 Copper per owned copy, per receipt, whenever Copper is gained (Copper Mine, Chinatown `YenToCopper`, 5-Power Copper token, Building-Site Copper/any-good spaces) — only fires if the base receipt is >0 |
| Textile Mill | 1 | 4 | GB | `SpinningMill` | Designed: once/turn, discard 1 Silk → 2 Yen. **Not reachable — no `Move` variant invokes it** (§9) |
| Patent System | 1 | 3 | FR | `PatentSystem` | Acquiring a (different) Technology card: +2 VP per *other* owned copy |
| Stagecoach | 1 | 3 | GB | `Stagecoach` | Movement: pass through opponent Presidents free (waives both the cost and the owner's credit) |
| Tram | 1 | 5 | US | `Tram` | Movement: move President to any area without an opponent President (uses the ordinary move generator/cost, not a special free-teleport) |
| Brickyard | 1 | 3 | DE | `Brickyard` | Customs placement: +4 VP per *owned* copy |
| Stock Market | 1 | 3 | NL | `SystemOfStock` | Designed: turn start, if Yen ≤1, refill to 2. **Not reachable — `advance_phase` never calls the turn-start-effects function** (§9) |

**Per-copy stacking, as actually implemented** (verified against `tech_effects.rs` and
`game/actions/cards.rs::apply_lab_acquisition_bonuses`):

- **Patent System (+2 VP)** and **Letter Press Printing (+1 Yen)**: counted at the moment a new
  Tech card is acquired, over the buyer's *other* owned copies (the just-bought physical card
  never counts toward its own bonus).
- **Winery (+3 VP)** and **Brickyard (+4 VP)**: counted at Church/Customs-placement time, over
  *all* owned copies (no self-exclusion needed — the tech was already fully owned before that
  turn's action started).
- **Mining Technology (+1 Copper)**: counted per receipt, over *all* owned copies, but only if the
  triggering event's base Copper amount is nonzero (a zero-Copper receipt gets zero bonus even
  with copies owned).

**Per-country count (unique titles / physical cards):**

| Country | Titles | Physical cards |
|---|---|---|
| US | 4 (Language School, Telegram, Telephone, Tram) | 7 |
| GB | 6 (Ball, Station, University, Postal System, Textile Mill, Stagecoach) | 9 |
| NL | 3 (Newspaper, Letterpress Printing, Stock Market) | 5 |
| DE | 3 (Exposition, Mining Technology, Brickyard) | 4 |
| FR | 4 (Gaslight, Winery, Electrical Light, Patent System) | 5 |
| **Total** | **20** | **30** |

**Industry-cost distribution (unique titles):** 3 ×12, 4 ×3 (Station, Electrical Light, Textile
Mill), 5 ×3 (Newspaper, Exposition, Tram), 6 ×2 (Telegram, Telephone).

**Physical-card id ↔ title (all 30, for a box recount):** 1–2 Language School · 3–4 Telegram ·
5–6 Telephone · 7–8 Newspaper · 9–10 Exposition · 11–12 Ball · 13–14 Gaslight · 15 Station ·
16–17 University · 18–19 Letterpress Printing · 20–21 Postal System · 22 Winery ·
23 Electrical Light · 24 Mining Technology · 25 Textile Mill · 26 Patent System · 27 Stagecoach ·
28 Tram · 29 Brickyard · 30 Stock Market.

**⚠ DISCREPANCY (prose KB vs. verified data):** `01-rules-reference.md`'s "Technology Cards
Reference" section states **Ball (Industry 5)** and **Brickyard (Industry 4)**; the verified YAML
and `data.rs` both say **Ball = 3** and **Brickyard = 3**. See §9 for a related, more serious
finding: a dead helper function in the engine itself (`TechEffect::base_industry_cost()`) also
disagrees with the verified costs, on 7 of 20 titles, including these two.

**Engine code:** type = `types/cards.rs::TechCard` / `TechEffect`; data =
`game/data.rs::standard_techs()`; registry = `game/cards.rs::TechRegistry`; effect logic =
`game/tech_effects.rs` (all `has_tech_effect`/bonus functions) and
`game/actions/cards.rs::apply_lab_acquisition_bonuses` (Patent System / Letter Press / Exposition,
fired right after a Lab purchase).

---

## 3. Achievement Cards

Three categories (A/B/C); **one card per category is drawn at random into a given game**
(`game/setup.rs::ACHIEVEMENTS_A/B/C` + `.choose(&mut rng)`), so only 3 of these 12 are ever in
play at once — but the box should contain all 12 (4 per category). VP columns are
`AchievementType::first_place_vp()` / `subsequent_vp()` (`types/cards.rs`); requirement text is
transcribed from the `AchievementType` doc comments and `achievements.rs::can_claim_achievement`.

### Category A — Trade Good Holdings (no payment; just demonstrate ownership) — 1st 8 VP / 2nd+ 6 VP

| Type | Requirement |
|---|---|
| `Copper5` | Own ≥5 Copper |
| `Silk6` | Own ≥6 Raw Silk |
| `Tea7` | Own ≥7 Tea |
| `Seafood7` | Own ≥7 Seafood |

### Category B — Collection Goals — 1st 9 VP / 2nd+ 7 VP

| Type | Requirement |
|---|---|
| `Money10` | Own ≥10 Yen |
| `Tokens5` | Own ≥5 total (5-Power tokens taken + Foreign Mercenaries total, used or not) |
| `Orders5` | Have completed ≥5 Order cards |
| `Technologies4` | Own ≥4 Technology cards |

### Category C — Building Presence — 1st 10 VP / 2nd+ 8 VP

| Type | Requirement |
|---|---|
| `Production4` | Buildings (Shophouse or Trading House) in ≥4 distinct Production areas |
| `Commercial4` | Buildings in ≥4 distinct Commercial areas |
| `Prod3Comm2` | Buildings in ≥3 Production areas **and** ≥2 Commercial areas |
| `Prod2Comm3` | Buildings in ≥2 Production areas **and** ≥3 Commercial areas |

A player may claim each in-play achievement at most once; claiming locks 1 assistant from hand
permanently onto the card (never recovered).

**Engine code:** type = `types/cards.rs::AchievementType` / `AchievementCategory` /
`AchievementCard`; validation + claim = `game/achievements.rs::can_claim_achievement` /
`claim_achievement`; move = `types/moves.rs::Move::FulfillAchievement`; category membership
(Production/Commercial) reuses `types/board.rs::AreaType::is_production()` / `is_commercial()`.

---

## 4. Building-Site Cards — all 24

Each area tile (except Canal) gets **one** Building Site card, dealt randomly at setup; the other
23 return to the box for that game (`09-building-sites.yaml` footer note, confirmed by
`game/board_gen.rs::generate_board_layout`'s shuffle-and-assign). **There is no fixed
site-id ↔ area-tile mapping** — do not expect "site_005 always sits on the Tea Plantation"; which
physical card lands on which board tile is randomized per game by the setup seed. The table below
is the generic pool, keyed by the card's box-sheet id, for counting/checking the pool itself.

**Card anatomy, true for all 24 (per the YAML header and `game/cards.rs::building_sites_match_adjudicated_anatomy`):**
top-left (TL) space is always `1 VP`; middle-left (ML) space is always blank. Only TR and MR vary,
plus the Trading House VP. Columns below omit TL/ML since they never change.

| Site | TR | MR | Trading House VP |
|---|---|---|---|
| site_001 | 3 VP | 3 Seafood | 7 |
| site_002 | 3 Seafood | 2 Seafood | 7 |
| site_003 | 4 VP | 2 Tea | 7 |
| site_004 | 2 Warehouse | 2 Tea | 8 |
| site_005 | 2 Silk | 1 Copper | 9 |
| site_006 | 2 Copper | 2 Silk | 9 |
| site_007 | 2 AnyGood | 1 AnyGood | 7 |
| site_008 | 2 AnyGood | 1 AnyGood | 6 |
| site_009 | 4 VP | 2 Movement | 8 |
| site_010 | 2 Warehouse | 2 Movement | 6 |
| site_011 | 2 Warehouse | 2 Movement | 7 |
| site_012 | 2 Warehouse | 2 Movement | 5 |
| site_013 | 3 VP + 1 Yen | 2 Movement | 6 |
| site_014 | 3 Warehouse | 2 Warehouse | 5 |
| site_015 | 3 VP | 2 Warehouse | 8 |
| site_016 | 2 VP + 1 Yen | 2 Warehouse | 8 |
| site_017 | 2 VP + 1 Yen | 1 VP + 1 Yen | 9 |
| site_018 | 2 VP + 1 Yen | 2 VP | 8 |
| site_019 | 4 VP | 2 Yen | 9 |
| site_020 | 4 VP | 2 Yen | 6 |
| site_021 | 3 VP | 1 VP + 2 Yen | 5 |
| site_022 | 4 VP | 1 VP + 2 Yen | 5 |
| site_023 | 4 VP | 3 VP | 9 |
| site_024 | 3 VP | 2 VP | 6 |

"Warehouse" and "Movement" above are already translated into engine/rule meaning — see the
icon-semantics note in §2 (the YAML's raw keys are `shophouse_build` → Warehouse, `assistants` →
Movement). "AnyGood" = wild trade-good token (engine auto-resolves to Copper on receipt — v1
simplification, §8).

**Trading House VP distribution (for box counting):** 5 ×4, 6 ×5, 7 ×5, 8 ×5, 9 ×5 (24 total).

**Engine code:** type = `types/board.rs::BuildingSite` / `SpaceReward`; data =
`game/data.rs::standard_sites()`; assignment to tiles = `game/board_gen.rs::generate_board_layout`
(shuffles both tiles and sites, assigns 1:1 to every non-Canal area); building moves =
`types/moves.rs::Move::BuildShophouse` / `BuildTradingHouse`; application =
`apply_move.rs::apply_build_shophouse` / `apply_build_trading_house` /
`apply_move.rs::apply_space_reward`.

---

## 5. Boards

### 5.1 Port (board-art verified 2026-07-17)

6 card slots: indices 0–1 = Low zone, 2–3 = Mid, 4–5 = High (`types/cards.rs::PortBoard::zone_of`).

| Power | Accessible zones | Slots reachable |
|---|---|---|
| 0–1 | none | 0 |
| 2 | Low | 2 |
| 3 | Low, Mid | 4 |
| 4–5 | Low, Mid, High | 6 |

Take 1 card free, or take 2 distinct cards for 2 Yen **or** 1 Imported Good (player's choice);
capped by the 3-card Order hand limit. **Power 5 also grants +3 VP**, awarded regardless of
whether 1 or 2 cards are taken (but not on Skip — see §8). Refill: full left-compaction of the
remaining cards, then refill the newly-empty slots **rightmost-first** from the deck (KB-explicit,
confirmed against the rulebook's "move all cards to the left ... refill from deck to rightmost
spaces").

**Engine code:** `types/cards.rs::PortBoard`; `game/actions/cards.rs::port_accessible_zones` /
`port_power_5_bonus_vp` / `validate_port_action` / `execute_port_action`;
`game/cards.rs::refill_port`; enumeration `game/legal_moves.rs::generate_port_actions`.

### 5.2 Laboratory (board-art verified 2026-07-17)

6 tech slots; position surcharge on top of the card's printed Industry cost:

| Slot (0-indexed) | 0 | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| Surcharge | +0 | +0 | +1 | +1 | +2 | +2 |

Industry Value sources: Power (1:1, capped at 5), 2 Yen = 1 IV, 1 Imported Good = 1 IV. The 5th+
Technology card owned costs an *additional* 2 Yen or 1 Imported Good
(`fifth_tech_penalty_required`: true once the buyer already owns ≥4). Refill: the vacated slot
slides the remainder left by one (only ever a single gap, since one Lab purchase removes exactly
one card), then the new rightmost empty slot is refilled from the deck.

**Engine code:** `types/cards.rs::LabBoard::position_modifier`;
`game/actions/cards.rs::lab_position_surcharge` / `calculate_industry_value` /
`fifth_tech_penalty_required` / `validate_lab_action` / `execute_lab_action` /
`apply_lab_acquisition_bonuses`; `game/cards.rs::remove_and_slide_lab` / `refill_lab`; enumeration
`game/legal_moves.rs::generate_lab_actions`.

### 5.3 Customs (re-modeled 2026-07-17 from board art — supersedes the old 8-slot goods-keyed model)

7 printed spaces, left to right:

| Space | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|
| Printed cost (Imported Goods) | 1 | 1 | 2 | 2 | 3 | 3 | 4 |
| VP | 5 | 4 | 10 | 9 | 16 | 15 | 22 |
| Rider | +2 Warehouse-takes | +2 Yen | +1 Warehouse-take | +1 Yen | — | — | — |

A payment must **exactly** match a space's printed cost. Cost 4 has no pair (space 7 alone); every
other cost has a 2-space pair. The engine auto-places on the higher-VP (leftmost) empty space of
the matching pair, falling back to the other space if it's occupied; if both are occupied (or, in
2-player games, space 4 is blocked by the printed dummy pawn — see below), the action is illegal
(no goods are paid for a placement that doesn't happen). **2-player games: space 4 (VP 9) is
pre-blocked by a printed dummy pawn** — an eligibility rule the engine encodes as a lookup
condition, not a `slots` entry, so end-trigger assistant counts stay untouched.
`customs_positions` records the **space number** (1–7) reached, not the goods paid.

**Engine code:** `types/tracks.rs::CustomsTrack` (`spaces_for_cost`/`vp_reward`/`rider`);
`game/actions/tracks.rs::customs_max_imported_goods` / `validate_customs_action` /
`eligible_customs_space_for_cost` / `execute_customs_action`; enumeration
`game/legal_moves.rs::generate_customs_actions`.

### 5.4 Church

8 spaces, Faith values 4–11:

| Faith | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 |
|---|---|---|---|---|---|---|---|---|
| VP | 3 | 4 | 5 | 6 | 7 | 8 | 11 | 14 |
| Movement rider (board art; v1 no-op) | 1 | 1 | 2 | 2 | 2 | 2 | 3 | 3 |

Faith = Power (**linear**, 1→1, 2→2, 3→3, 4→4, 5→5, capped at 5 — Jeremy-adjudicated 2026-07-17;
the rulebook's "2 Power: 1 Faith" is a text typo) **plus 1 per distinct payment type contributed**
(copper/silk/tea/seafood/yen/imported good — max 1 of each, no repeats). The engine auto-places on
the **highest** empty eligible space with value ≤ total Faith (any excess Faith is lost); Faith <4,
or no eligible empty space, makes the action illegal (goods are never paid for a placement that
doesn't happen). **2-player games: the Faith-7 space is pre-blocked** by a printed dummy pawn,
same encoding style as Customs space 4. The movement riders in the last row are **not represented
anywhere in the data model** — there is no `ChurchTrack::rider()` analogous to
`CustomsTrack::rider()` — they are a pure, unimplemented no-op per the standing movement-icon
policy (§8).

**Engine code:** `types/tracks.rs::ChurchTrack` (`faith_to_index`/`vp_reward`);
`game/actions/tracks.rs::church_faith_from_power` / `calculate_church_faith` /
`validate_church_action` / `highest_eligible_church_space` / `execute_church_action`; enumeration
`game/legal_moves.rs::generate_church_actions`.

### 5.5 Employment Agency

| Power | Tokens allowed |
|---|---|
| 0–1 | 0 |
| 2 | 1 |
| 3 | 2 |
| 4 | 3 |
| 5 | 4 |

Costs (paid when the token moves **warehouse → hand**, not at Building time): Assistant free;
Shophouse 2 Yen each; Trading House at the take-order progression **4 / 5 / 6 / 7 Yen**, indexed
by how many the player has already taken out of the warehouse this game (already-in-hand +
already-built). The Building step later places a Trading House **from hand for free** — the
progression cost was already paid here (rulebook p.7 "place ... from your hand"; corrected
2026-07-17, see §9's TH-cost note).

**Engine code:** `game/actions/commercial.rs::employment_agency_tokens_allowed`;
`apply_move.rs::apply_employment_agency`; `types/player.rs::TradingHouseState::next_cost`;
enumeration `game/legal_moves.rs::generate_employment_agency_actions`.

### 5.6 Chinatown (not in 2-player games)

Number of exchanges in one action ≤ Power (checked both at enumeration and at application —
`validate_chinatown_exchanges`). All 9 rates, transcribed from
`game/actions/commercial.rs::apply_chinatown_exchange`:

| Pay | Receive |
|---|---|
| 1 Copper | 2 Yen |
| 1 Raw Silk | 2 Yen |
| 1 Tea | 1 Yen |
| 1 Seafood | 1 Yen |
| 2 Yen | 1 Copper *(+1 more per owned Mining Technology copy)* |
| 2 Yen | 1 Raw Silk |
| 1 Yen | 1 Tea |
| 1 Yen | 1 Seafood |
| 4 Yen | 1 Imported Good |

Imported Goods cannot be sold for money (no exchange rate exists for it).

**Engine code:** `types/moves.rs::ChinatownExchange`;
`game/actions/commercial.rs::apply_chinatown_exchange` / `validate_chinatown_exchanges`;
enumeration `game/legal_moves.rs::generate_chinatown_actions` (BFS over exchange sequences, depth
capped at `min(power, 4)` — a documented v1 enumeration bound, §8).

### 5.7 Bank & Production areas

| Power | Bank (Yen) | Copper Mine | Silk Mill | Tea Plantation | Fishing Ground |
|---|---|---|---|---|---|
| 1 | 1 | 0 | 0 | 1 | 1 |
| 2 | 2 | 0 | 1 | 2 | 2 |
| 3 | 3 | 1 | 2 | 3 | 3 |
| 4 | 4 | 2 | 3 | 4 | 4 |
| 5 | 5 | 3 | 4 | 5 | 5 |

Copper Mine (and every other Copper-yielding source) gets **+1 per owned Mining Technology copy**
on any nonzero receipt.

**Engine code:** `game/actions/production.rs` (`copper_mine_reward`/`silk_mill_reward`/
`tea_plantation_reward`/`fishing_ground_reward`); `game/actions/commercial.rs::bank_reward`;
`game/tech_effects.rs::copper_bonus`.

### 5.8 5-Power tokens

20 tokens total (verified physical component count), one dealt per non-Canal area tile at setup,
shuffled independently of the building-site deal:

| Effect | Count | Amount |
|---|---|---|
| Imported Good | 10 | 1 |
| Copper *(+ Mining Technology bonus)* | 2 | 2 |
| Silk | 2 | 2 |
| Seafood | 2 | 3 |
| Tea | 2 | 3 |
| Money | 2 | 3 |

`types/board.rs::FivePowerEffect` also has `VictoryPoints` and `Movement` variants (the latter
recovers N assistants from board to hand) — both are fully implemented in
`apply_move.rs::apply_take_5_power`, but neither is ever constructed by the canonical 20-token
pool actually used by `Rules::standard()`; they're exercised only by hand-rolled test fixtures.
This matches the verified physical set (no VP/Movement 5-power tokens in the box), so it's not a
bug — just unused enum capacity worth knowing about if a future rules variant needs it.

**Engine code:** `game/board_gen.rs::canonical_five_power_tokens`; assignment
`generate_board_layout`; taking one `apply_move.rs::apply_take_5_power`.

---

## 6. Setup Constants

### Player-count scaling

| | 2P | 3P | 4P |
|---|---|---|---|
| Area tiles on board | 10 | 14 | 18 |
| Port boards | A only | A + B | A + B |
| Laboratory boards | A only | A only | A + B |
| Chinatown / Canal present | no | yes | yes |
| Church/Customs end-trigger (assistants placed) | 4 | 5 | 6 |
| Order cards removed at setup | 16 | 5 | 0 |
| Production tiles removed | 1 each of all 4 types | 1 each of Copper/Silk/Tea (both Fishing kept) | none |

**⚠ DISCREPANCY (prose KB vs. verified engine):** `01-rules-reference.md`'s "Components by Player
Count" table states **Area Boards = 12 / 15 / 18**. The engine (`game/board_gen.rs`, following the
rulebook's explicit removal-rule text rather than that summary row) produces **10 / 14 / 18**,
and this is called out in the engine source itself:
`board_gen.rs` — *"Note: The KB table shows 12/15/18 but removal rules give 10/14/18. Following
the explicit removal rules from the rulebook."* Recommend a physical box recount of area tiles at
each player count to settle which is right; the engine's arithmetic (18 total − explicitly-listed
removals) is internally consistent and matches its own test suite, but hasn't been checked against
a live 2P/3P table setup.

### Starting resources (every player, same regardless of turn order except money)

| Resource | Amount |
|---|---|
| Money | 3 Yen (start player) / 4 Yen (all others) |
| Assistants | 8 in hand + 12 in warehouse (20 total) |
| Shophouses | 2 in hand + 6 in warehouse (8 total) |
| Trading Houses | 0 in hand + 4 in warehouse (4 total) |
| Copper / Silk / Tea / Seafood | 1 each |
| Starting Order card | draw 2, keep 1 (the other returns to the removed pile — or, for a live-tracked opponent whose hand is kept hidden, *both* dealt cards go to the removed pile, since which one they kept is unknowable) |

**Engine code:** `types/state.rs::GameConfig::starting_money`; `types/player.rs::AssistantState::new`
/ `ShophouseState::new` / `TradingHouseState::new`; `types/player.rs::PlayerState::new`;
`game/setup.rs::new_game` / `finalize_setup` / `StartingOrderChoice`.

### Deck sizes after setup (derived; 4P cross-checked by
`game/setup.rs::new_game_is_seed_deterministic_and_decks_are_correct`)

| | 2P | 3P | 4P |
|---|---|---|---|
| Order deck after setup | 36 − 16 removed − 4 dealt (2×2) − 6 Port-A fill = **10** | 36 − 5 removed − 6 dealt (3×2) − 12 Port-A+B fill = **13** | 36 − 0 removed − 8 dealt (4×2) − 12 Port-A+B fill = **16** ✓ |
| Tech deck after setup | 30 − 6 Lab-A fill = **24** | 30 − 6 Lab-A fill = **24** | 30 − 12 Lab-A+B fill = **18** ✓ |

### Achievements & mercenaries

- 1 Achievement per category (A/B/C) drawn at random into play (§3); the other 3-per-category
  return to the box.
- Foreign Mercenary supply: English 3, American 2, French 2, German 2, Dutch 1 (11 total) —
  `types/tracks.rs::MercenarySupply::new` / `types/ids.rs::Country::mercenary_supply`. A player
  gains 1 of a country's Mercenary every time their completed-Orders-plus-owned-Techs count for
  that country's icon crosses an even number (2, 4, 6…), while supply lasts.

---

## 7. Rule → Engine Map

One row per rule; "code" is `file.rs::function`.

### Turn phases (`types/moves.rs::TurnPhase`, machine driven by `apply_move.rs::apply_move`)

AdditionalActionFirst → Placement → Movement → AreaAction → FivePowerBonus → Building →
Recovery (automatic) → AdditionalActionSecond → TurnEnd. A mercenary use runs a **nested** sub-turn
(AreaAction→FivePowerBonus→Building→Recovery only) and then returns to whichever
AdditionalAction phase it was invoked from (`is_mercenary_action` / `mercenary_return_phase`).

### Placement

| Rule | Code |
|---|---|
| Option A: 1 assistant in each of up to 3 (4 with Newspaper) distinct areas | `legal_moves.rs::generate_placement_moves_with_tech` |
| Option B: 2 in one area (+1 in an adjacent area with Telegram) | same function; `Move::PlaceTwoSameArea` |
| Cannot place in Canal | `types/board.rs::AreaType::allows_placement` |
| 1 Yen per assistant placed, per opponent President occupying the area | `legal_moves.rs::calculate_placement_cost`; charged `apply_move.rs::apply_placement`/`apply_placement_same_area`; credited to each owner `apply_move.rs::credit_placement_fees` |

### Movement

| Rule | Code |
|---|---|
| President in hand → place in any Possible Action Area | `legal_moves.rs::generate_movement_moves_with_tech` (InHand branch) |
| President on board → step through areas with your own assistants, may stop in any Possible Action Area | `movement.rs::find_reachable_areas` / `is_possible_action_area` |
| Cannot stop where an opponent President sits (unless Postal System) | `movement.rs::is_possible_action_area_full` — **⚠ see §9: the Postal System branch of this exact check is never actually consulted by the production move generator** |
| Pay 1 Yen per opponent President passed through (unless Stagecoach) | `movement.rs::calculate_movement_cost_full`; credited `apply_move.rs::credit_movement_president_fees` |
| Canal toll 1 Yen (goes to the bank, not credited to anyone) | same cost function; canal tolls are deliberately excluded from the credit function |
| Cannot return to the turn's starting area | `apply_move.rs::validate_movement_destination` / `president_start_area` |
| Option B: return President to hand, optionally recover any assistants, skip straight to Phase C | `apply_move.rs::apply_return_to_hand`; always legal, including from-hand (2026-07-17 ruling — the required Movement step must never dead-end) |
| Station: free teleport to the token's area | `movement.rs::can_teleport_to_station_with_tech` / `generate_station_teleport_move` (mechanic works; nothing ever places the token — §9) |

### Area Action (all six "permissive" areas support `AreaAction::Skip`; Production areas + Bank do not — 2026-07-17 optionality ruling)

| Area | Code |
|---|---|
| Copper Mine / Silk Mill / Tea Plantation / Fishing Ground | §5.7; `apply_move.rs::apply_area_action` (Produce* arms) |
| Bank | §5.7; same function, `Bank` arm |
| Port A/B | §5.1; `game/actions/cards.rs::execute_port_action`; enum `game/legal_moves.rs::generate_port_actions` |
| Laboratory A/B | §5.2; `execute_lab_action`; `generate_lab_actions` |
| Employment Agency | §5.5; `apply_move.rs::apply_employment_agency`; `generate_employment_agency_actions` |
| Chinatown | §5.6; `apply_chinatown_exchange`; `generate_chinatown_actions` |
| Church | §5.4; `execute_church_action`; `generate_church_actions` |
| Customs | §5.3; `execute_customs_action`; `generate_customs_actions` |
| `Skip` (decline all optional components) | `apply_move.rs::apply_area_action` `Skip` arm; legality gate `types/board.rs::AreaType::allows_area_action_skip` |
| Trading House passive income (any area with an opponent-owned Trading House pays that owner 1 Yen, fires even on Skip) | `apply_move.rs::apply_trading_house_passive_income` |

### 5-Power Bonus / Building / Recovery

| Rule | Code |
|---|---|
| 5-Power token still present → take it, resolve, flip face-down | `legal_moves.rs::generate_five_power_moves`; `apply_move.rs::apply_take_5_power` |
| Build Shophouse (4+ Power action; one per area per player) | `types/board.rs::BuildingSite::next_shophouse_slot`; `apply_move.rs::apply_build_shophouse` |
| Build Trading House (4+ Power; from hand, free — cost paid at Employment Agency) | `BuildingSite::can_build_trading_house`; `apply_move.rs::apply_build_trading_house` |
| Recovery: return all assistants from the action area (Telephone: may leave 1) | `apply_move.rs::apply_recovery_with_leave` |

### Additional Actions (Phase A/C)

| Rule | Code |
|---|---|
| Fulfill Order card (pay goods, gain reward, check Mercenary eligibility) | `game/cards.rs::fulfill_order`; applied move `apply_move.rs::apply_fulfill_order` (note: two separate implementations — see §9) |
| Fulfill Achievement (once per player per card) | `game/achievements.rs::can_claim_achievement` / `claim_achievement` |
| Use Foreign Mercenary (once per turn total; bypasses opponent Presidents, no fees; flips face-down after use) | `apply_move.rs::apply_mercenary`; movement inside the sub-turn `is_possible_action_area_mercenary` / `calculate_movement_cost_mercenary` |
| Foreign Mercenary awarded at 2/4/6… of a country icon | `game/cards.rs::check_mercenary_eligibility` |

### End-game triggers & final scoring

| Trigger | Code |
|---|---|
| A player built all 4 Trading Houses | `end_game.rs::check_end_game_triggers` (`TradingHousesComplete`) |
| A player built all 8 Shophouses | same fn (`ShophousesComplete`) |
| Order deck empty **and** a Port slot is empty | same fn (`OrderDeckEmpty`); `types/cards.rs::PortBoard::has_empty_slot` |
| Church track reaches the player-count threshold | same fn (`ChurchTrackFull`) |
| Customs track reaches the player-count threshold | same fn (`CustomsTrackFull`) |
| Finish current round, then one full final round, then score | `apply_move.rs::advance_phase` (`final_round` bookkeeping); `is_game_over` |
| 1. Church majority: 1st 6 VP / 2nd 3 VP (ties → rightmost) | `scoring.rs::calculate_church_bonus` |
| 2. Customs majority: 1st 8 VP / 2nd 4 VP (ties → rightmost) | `scoring.rs::calculate_customs_bonus` |
| 3. Technology total Industry Value: 1st 10 VP / 2nd 5 VP (ties → closest to start player clockwise) | `scoring.rs::calculate_technology_bonus` |
| 4. Country sets (5 diff=12, 4=8, 3=4, 2=2 VP; each icon used once, greedy largest-set-first) | `scoring.rs::calculate_country_bonus` |
| 5. Remainder: unused Mercenary 1 VP each, Imported Good 1 VP each, every 2 Yen = 1 VP, every 3 trade goods = 1 VP | `scoring.rs::calculate_remainder_bonus` |

---

## 8. v1 Engine Limitations (by design, tracked in the decision log)

These are deliberate simplifications, not bugs — flagged here for Phase-2 planning:

- **Movement-icon rewards are no-ops.** The icon grants an optional, declinable assistant move;
  v1 has no move encoding for the choice and auto-resolving it could harm the player. Affects
  Order rewards, Building-Site MR spaces, and the Church per-space rider.
- **Warehouse-icon rewards auto-take 1 free assistant.** The rulebook lets the player pick any one
  token type at Employment Agency cost; assistants are free and weakly dominant, so v1 always
  takes one, up to warehouse stock (`apply_move.rs::apply_warehouse_takes`).
- **Any-good rewards auto-pick Copper.** `apply_move.rs::apply_any_good`.
- **Chinatown enumeration caps at a 4-exchange sequence** even though a legal 5-exchange move
  exists at Power 5 (`legal_moves.rs::CHINATOWN_ENUM_DEPTH`) — a bounded-enumeration choice, not a
  rules gap; `apply_area_action`'s validator still enforces the true `exchanges ≤ power` rule.
- **`AreaAction::Skip` grants no area-attached bonuses** — e.g. a full-hand 5-Power Port Skip
  forgoes the +3 VP, since that requires an actual take.
- **Church/Customs auto-placement** always chooses the highest-Faith / highest-VP eligible empty
  space; the rulebook permits choosing a lower one, which the engine treats as strictly dominated
  and doesn't enumerate.
- **Station's token is never placed by anything** in the current engine (a deliberate, if
  under-documented, deferral — see §9 for the practical consequence).

## 9. Discrepancies and Engine-Completeness Findings

Everything above is a straight data/rule transcription. This section is what the verification pass
turned up that goes beyond that — ranked by how much it matters.

**1. Postal System (2 physical copies, GB, cost 3) cannot function in real play — HIGH PRIORITY.**
`movement.rs::is_possible_action_area_full` correctly implements "may stop where an opponent
President sits, given Postal System," and `is_possible_action_area_with_tech` exposes that check —
but grep confirms `is_possible_action_area_with_tech` has **zero callers anywhere in the crate**.
The actual production path, `movement.rs::find_reachable_areas` (called from
`legal_moves.rs::generate_movement_moves_with_tech`, the sole source of legal Movement moves),
hard-codes the tech-blind `is_possible_action_area` internally and has no parameter to pass a
`TechRegistry` through at all — and this is true for *both* of that function's branches (President
on board, and President still in hand). So a Postal-System-owning player is never offered a
`Move::MovePresident`/`Move::PlacePresident` destination that lands on an opponent-occupied area,
regardless of tech owned. `calculate_movement_cost_with_tech` in the same file *does* correctly
thread tech (for Stagecoach) — only the reachability filter was missed. This needs a fix (thread
`Option<&TechRegistry>` through `find_reachable_areas`, or call the `_with_tech` variant at both
of `generate_movement_moves_with_tech`'s existing call sites, which already receive
`tech_registry` as a parameter and simply don't use it for this check) before Postal System can be
considered implemented.

**2. Three Technology effects are fully coded, unit-tested, and completely unreachable in
play.** `SpinningMill` (Textile Mill), `University`, and `SystemOfStock` (Stock Market) each have a
`can_use_*`/`apply_*` pair in `tech_effects.rs` with passing tests — but grepping the whole crate
shows every call site for these functions is inside `tech_effects.rs`'s own `#[cfg(test)]` module.
There is no `Move` variant for "use Spinning Mill" or "use University" in `types/moves.rs::Move`,
and `apply_move.rs::advance_phase` (the turn-transition function) never calls
`apply_turn_start_effects`. A player who owns any of these three titles currently gets no benefit
from them at all. (Contrast with the §8 list: those are *intentional* simplifications of which
choice to auto-resolve; these three are simply not wired to anything.)

**3. Station's acquisition effect is a known but under-documented gap.** Grepping for
`station_location =` assignments shows the only ones anywhere in the crate are inside test
fixtures (`power.rs`, `legal_moves.rs`); no production code path ever places the token. The
teleport mechanic itself (`can_teleport_to_station`, `generate_station_teleport_move`) is correctly
implemented and would work fine *if* a token were ever placed. This is called out in a code
comment (`apply_lab_acquisition_bonuses`: *"Station's token placement on acquisition is a
deliberate v1 engine limitation"*) but isn't in the decision log or CLAUDE.md, so it's easy to miss
— now listed in §8 too.

**4. A dead function contains stale, pre-verification Tech costs that disagree with the verified
data on 7 of 20 titles.** `types/cards.rs::TechEffect::base_industry_cost()` — its own doc comment
admits *"Some costs are uncertain in the reference; using best estimates"* — has **zero callers
anywhere in the crate** (confirmed by grep; all real cost lookups go through
`TechCard.industry_cost`, sourced from `data.rs`/the verified YAML). Its values disagree with the
verified `industry_cost` on: PostalSystem (4 vs. verified 3), Ball (5 vs. 3), Winery (4 vs. 3),
Brickyard (4 vs. 3), University (4 vs. 3), Newspaper (4 vs. 5), Telegram (3 vs. 6). Harmless today
because nothing calls it, but it's a landmine if a future refactor ever wires it in. Recommend
deleting it, or replacing its body with a registry lookup so it can't drift again.

**5. Two direct prose-KB contradictions**, already noted inline: `01-rules-reference.md` says Ball
is Industry 5 and Brickyard is Industry 4; the verified YAML/`data.rs` say 3 for both. (These
happen to be 2 of the same 7 values the dead function in #4 also gets wrong — likely the same
stale source propagated to both places before the 2026-07-17 card-verification pass.)

**6. Area-tile count table mismatch**, already noted inline in §6: KB says 12/15/18 for 2P/3P/4P,
the engine (matching the rulebook's explicit removal-rule text, per its own code comment) computes
10/14/18. Recommend a physical recount to resolve which is authoritative.

**7. Minor, no action needed:** `game/board_gen.rs::five_power_tokens_for_player_count` is unused
(`Rules::standard()` always threads the full 20-token canonical pool through
`generate_board_layout`, which shuffles-then-truncates internally; this helper's own
truncate-without-shuffle behavior is never exercised outside its own test). `two` `FivePowerEffect`
variants (`VictoryPoints`, `Movement`) are fully implemented but never constructed by the canonical
pool — matches the verified physical set, not a bug.
