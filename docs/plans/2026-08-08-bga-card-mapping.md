# Phase 3.5b — BGA card-mapping evidence table (H0 discovery output)

**Date:** 2026-08-08 · **Status:** EVIDENCE, not a shipped table. Every row below is derived from ONE captured game and carries its provenance; nothing here is a guess, and every gap is written down as a gap. H1's card mapper is built from this, but the mapper's own checked table is a source file with its own tests — this document is the argument for what that table may legally contain.

**Corpus:** `bridge-captures/895724349.ndjson` (gitignored, **491 non-empty lines measured at this HEAD** — 488 `packets` + 3 `snapshot`; the ledger's live-run row reads "490 captures", a one-line difference left unreconciled here because nothing below depends on it — 1.85 MB) — BGA table **#895724349**, a COMPLETE 2-player game, **XeroxMan** `88889596` (the capturing session's own seat) vs **acmuriel** `88368036`. Three `gamedatas` snapshots (two mid-game, one end-of-game), four `notificationHistory` XHR responses (fullest: 187 packets), 484 WebSocket frames. Duplicates are normal (at-least-once, two sockets); everything below deduped on payload identity, never on `received_at_ms` (ledger N-4).

**Engine reference:** `crates/yokohama-core/src/game/data.rs` at branch `phase3.5b-bga-bridge`. **The engine was not touched to produce this** — the fence held; this is a read-only analysis.

**Scratch matcher:** written for this pass at `…/scratchpad/match_cards.py` (session scratchpad, not committed). It parses the 36 orders + 30 techs out of `data.rs` by regex, verifies the uniqueness claim, extracts every card observation from the corpus, and matches.

---

## 1. The identification key, verified rather than assumed

Design §1 asserts "all 36 of our order requirement+reward vectors are unique". **Re-verified independently as a step of this pass, and it holds:**

| Check | Result |
|---|---|
| 36 engine orders → distinct `(req_copper, req_silk, req_tea, req_seafood, vp, money, imports, warehouse, movement)` vectors | **36 / 36 — zero collisions** |
| Reward channels BGA does **not** carry (`copper`, `silk`, `tea`, `seafood`, `any_trade_good` as *rewards*) nonzero on any card | **NONE** — so the key is not lossy: BGA's five bonus fields plus the four requirements exhaust the engine's discriminating content |
| Requirement 4-tuple **alone** sufficient? | **NO** — `(copper 2, silk 0, tea 1, seafood 1)` is shared by **O10 and O32**. The reward half of the key is load-bearing, and this game observed *both* of those cards (BGA 22 → O10, BGA 28 → O32), so the collision is not hypothetical. |

That last row is the reason the mapper's key must be requirement **+** reward and a mapper written on requirements alone would have mis-identified two cards in this very game.

**Field correspondence used throughout** (BGA → engine): `req_copper→copper`, `req_silk→silk`, `req_tea→tea`, **`req_fish→seafood`**, `bonus_vp→victory_points`, `bonus_yen→money`, `bonus_import→imported_goods`, `bonus_wh→warehouse_tokens`, `bonus_move→movement`. All BGA values arrive as **strings**; the mapper parses, and a non-numeric string is a hard error, not a `0`.

---

## 2. Observed orders → engine orders

**20 distinct BGA `order_id`s observed. All 20 matched, each to exactly one engine card. Zero ambiguities, zero misses.**

| BGA `order_id` | `order_key` | BGA `country` | req c/s/t/f | vp/yen/imp/wh/mov | engine `OrderCardId` | label | engine `Country` |
|---|---|---|---|---|---|---|---|
| 2 | 20 | 4 | 1/0/1/2 | 2/1/1/0/0 | 6 | O06 | Dutch |
| 3 | 11 | 4 | 1/1/1/0 | 3/0/1/0/0 | 8 | O08 | Dutch |
| 5 | 18 | 4 | 0/2/2/2 | 11/0/0/0/0 | 21 | O21 | Dutch |
| 7 | 7 | 2 | 0/2/1/1 | 5/0/0/1/0 | 4 | O04 | German |
| 10 | 19 | 2 | 1/2/0/2 | 8/1/0/0/0 | 17 | O17 | German |
| 11 | 9 | 2 | 0/3/0/1 | 2/1/1/0/0 | 9 | O09 | German |
| 13 | 15 | 1 | 1/0/0/3 | 2/0/0/0/2 | 31 | O31 | American |
| 21 | 8 | 5 | 2/1/2/0 | 8/0/1/0/0 | 20 | O20 | English |
| 22 | 6 | 5 | 2/0/1/1 | 4/0/0/0/2 | 10 | O10 | English |
| 23 | 5 | 5 | 3/1/1/0 | 9/0/1/0/0 | 15 | O15 | English |
| 24 | 1 | 5 | 2/0/0/1 | 4/1/0/0/0 | 3 | O03 | English |
| 25 | 3 | 5 | 2/0/0/2 | 1/2/0/1/0 | 27 | O27 | English |
| 26 | 16 | 5 | 3/1/0/2 | 7/2/1/0/0 | 24 | O24 | English |
| 27 | 2 | 5 | 2/1/0/2 | 8/0/1/0/0 | 13 | O13 | English |
| 28 | 12 | 5 | 2/0/1/1 | 5/0/1/0/0 | 32 | O32 | English |
| 29 | 17 | 3 | 1/0/3/0 | 3/0/1/0/0 | 7 | O07 | French |
| 30 | 13 | 3 | 0/0/2/1 | 3/0/0/0/0 | 2 | O02 | French |
| 33 | 10 | 3 | 0/1/4/0 | 8/0/0/0/0 | 29 | O29 | French |
| 34 | 14 | 3 | 1/2/3/0 | 9/0/1/0/0 | 23 | O23 | French |
| 36 | 4 | 3 | 0/2/3/0 | 4/2/0/0/0 | 30 | O30 | French |

**Vector self-consistency:** every `order_id` was observed multiple times (snapshots, `orderAcquisition.order_card`, `orderRefresh.order_card`, `orderCompletion.order`, `orderDiscarded.order`, and the bulk `orders` maps). **Across all observations no `order_id` ever changed its vector** — the identity is stable within a table, as a mapper needs it to be.

**Key stability:** `order_key` → `order_id` is one-to-one and never re-bound (checked across the whole corpus). `order_key` is the *instance* handle (deal/deck slot), `order_id` the *card identity*; the mapper reads `order_id`, but H2 will want `order_key` for tracking the same physical card through locations.

### 2.1 The permutation is GLOBAL, not per-table — cross-check against the 2026-08-07 story

Design §1 recorded five pairs measured on a **different** table (#895608640, 2026-08-07): *their 5→our O21, 10→O17, 11→O09, 21→O20, 28→O32*. All five appear in this game's data, and **all five reproduce exactly**:

| Design §1 pair (table #895608640) | This corpus (table #895724349) | Agreement |
|---|---|---|
| BGA 5 → O21 | BGA 5 → O21 | ✅ |
| BGA 10 → O17 | BGA 10 → O17 | ✅ |
| BGA 11 → O09 | BGA 11 → O09 | ✅ |
| BGA 21 → O20 | BGA 21 → O20 | ✅ |
| BGA 28 → O32 | BGA 28 → O32 | ✅ |

That is the first evidence that BGA's `order_id` is a **stable global card identity**, not a per-table shuffle label. It does *not* license shipping a static number table: the vector match is what the mapper runs, and the stability is a cross-check the mapper's tests can assert, not a shortcut it may take. (Design §1's own ruling — "numbers can never transfer" — stands; what transfers is the *verification*.)

**The four cards from the 2026-08-07 manual-tracking story** (guidingpath 2026-08-07 row: mis-entered **O36** starting order and **O24** port take; correctly-entered fulfilled **O09** and opponent's **O17** starting order):

| Card | In this corpus? | Note |
|---|---|---|
| **O09** | ✅ BGA `order_id` 11 | the manual entry that cross-checked perfectly; the vector match reproduces it |
| **O17** | ✅ BGA `order_id` 10 | ditto |
| **O24** | ✅ BGA `order_id` 26 | O24 **exists and is identifiable** — the 2026-08-07 failure was ENTRY (recording O24 for a card BGA never showed the owner holding), not a representation gap. Its vector `3/1/0/2 → 7/2/1/0/0` is unique and would have been matched. |
| **O36** | ❌ not in this game | O36 was one of the 16 cards removed at 2P setup here (§5). Its engine vector — req `0/4/2/0`, rewards `5/2/0/1/0` (vp/yen/imp/wh/mov) — is unique among the 36, so it is identifiable *when it appears*; this corpus simply does not contain it. **UNOBSERVED**, not unmatchable. |

---

## 3. BGA country code → engine `Country`

Derived from the 20 matched pairs, then corroborated twice independently.

| BGA `country` | engine `Country` | BGA flag noun in log text | BGA counter field | matched-pair support |
|---|---|---|---|---|
| 1 | **American** | *(none observed)* | `flags_us` | 1 pair (BGA 13 → O31) |
| 2 | **German** | `Germany` | `flags_ge` | 3 pairs |
| 3 | **French** | `France` | `flags_fr` | 5 pairs |
| 4 | **Dutch** | `Netherlands` | `flags_ne` | 3 pairs |
| 5 | **English** | `Great Britain` | `flags_gb` | 8 pairs |

**Zero conflicts** — no BGA code ever mapped to two engine countries across the 20 pairs.

**Corroboration 1 — flag nouns in `orderCompletion` / `techAcquisition` log text.** Each completion's `gain_list` names the flag gained, and the noun always agrees with the matched card's engine country (e.g. `order_id` 3, `country: "4"` → "1 **Netherlands** flag"; `order_id` 7, `country: "2"` → "1 **Germany** flag").

**Corroboration 2 — the flag counters balance exactly.** End-of-game `counters` versus the flags earned from completed orders and acquired techs:

| Player | source events | predicted | `counters` at end | agree |
|---|---|---|---|---|
| XeroxMan | orders c5×3, c2×1, c3×1, c4×1 + techs c5×2, c2×2 | gb 5, ge 3, fr 1, ne 1, us 0 | `flags_gb 5, flags_ge 3, flags_fr 1, flags_ne 1, flags_us 0` | ✅ |
| acmuriel | orders c3×2, c5×1, c2×1 | fr 2, gb 1, ge 1, ne 0, us 0 | `flags_fr 2, flags_gb 1, flags_ge 1, flags_ne 0, flags_us 0` | ✅ |

**Honest scope of code 1 = American.** No order or tech of BGA country 1 was ever completed or acquired in this game, so there is **no flag-noun evidence** for it. It rests on (a) elimination — codes 2–5 are pinned by name *and* by arithmetic, and the counter suffix set is exactly `{us, gb, ge, fr, ne}`; (b) the one matched pair BGA 13 → O31, whose engine country is American; (c) the tech-class multiset agreement of §4.2, which fails unless 1 = American. That is three independent supports and no contradicting evidence — but it is **derived, not directly named**, and H1's table should carry that annotation.

---

## 4. Technologies

### 4.1 What the corpus contains

All **30** BGA `tech_id`s were observed (the `gamedatas.techs` map enumerates the entire tech pool from the first snapshot on), each with `country` and `req_gears`. `tech_key` → `tech_id` is one-to-one and stable, and no `tech_id` ever changed its `(country, req_gears)`.

**But `(country, req_gears)` is NOT an identifying key** — the engine has 30 physical cards over 20 titles (10 titles × 2 copies, 10 × 1), and its `industry_cost` + `country` collapse into 11 classes, one of which (English / 3 gears) holds seven cards. Direct name evidence exists for only four acquisitions, and only **one** of those carries the name and the `tech_id` in the same notification.

### 4.2 Structural cross-check: the class multisets agree exactly

Grouping BGA's 30 tech observations by `(mapped country, req_gears)` and the engine's 30 by `(country, industry_cost)`:

| class | BGA `tech_id`s | n | engine titles | n |
|---|---|---|---|---|
| American / 3 | 11, 21 | 2 | Language School ×2 | 2 |
| American / 5 | 4 | 1 | Tram | 1 |
| American / 6 | 13, 19, 23, 29 | 4 | Telegram ×2, Telephone ×2 | 4 |
| Dutch / 3 | 5, 20, 30 | 3 | Letterpress Printing ×2, Stock Market | 3 |
| Dutch / 5 | 17, 27 | 2 | Newspaper ×2 | 2 |
| English / 3 | 6, 12, 16, 18, 22, 26, 28 | 7 | Ball ×2, University ×2, Postal System ×2, Stagecoach | 7 |
| English / 4 | 1, 7 | 2 | Station, Textile Mill | 2 |
| French / 3 | 8, 10, 15, 25 | 4 | Gaslight ×2, Winery, Patent System | 4 |
| French / 4 | 3 | 1 | Electrical Light | 1 |
| German / 3 | 2, 9 | 2 | Mining Technology, Brickyard | 2 |
| German / 5 | 14, 24 | 2 | Exposition ×2 | 2 |
| **total** | | **30** | | **30** |

**Perfect multiset agreement, class for class.** This is a real result rather than bookkeeping: it independently corroborates the §3 country map (which was derived from *orders*, a disjoint evidence source), and it confirms that BGA's tech pool is the same 30 physical cards the engine ships. It is also what makes the singleton- and uniform-class derivations below legitimate.

### 4.3 Direct name evidence

Only four `techAcquisition` notifications occurred, all by XeroxMan, and only one of them carries `tech_card` alongside `tech_name`:

| packet | move | `tech_name` | `tech_card` present? | flag gained (log) |
|---|---|---|---|---|
| 6 | 6 | **University** | ❌ (history-form args) | Great Britain |
| 31 | 30 | **Ball** | ❌ | Great Britain |
| 107 | 102 | **Exposition** | ❌ | Germany |
| 127 | 120 | **Mining Technology** | ✅ `tech_id: "2", country: "2", req_gears: "3"` | Germany |

(A fifth tech event, packet 191, is `techRefusal` by acmuriel — no card. acmuriel acquired **zero** techs all game, confirmed by `tech_count: "0"` in the end snapshot; so these four acquisitions are the complete set and the four owned `tech_key`s account for all of them.)

**The ownership-timeline chain** closes two more. Owned techs (`location: "4"`) accumulate across the three snapshots:

| snapshot `captured_at` (unix s) | position in the packet stream | owned `tech_key` → `tech_id` |
|---|---|---|
| 1786155511 | after University (pkt 6, t=…277), **before** Ball (pkt 31, t=…645) | {1 → 12} |
| 1786156733 | after Exposition (pkt 107, t=…687), before Mining Tech (pkt 127, t=…034) | {1 → 12, 3 → 26, 4 → 14} |
| 1786158312 | after all four | {1 → 12, 3 → 26, 4 → 14, 5 → 2} |

Snapshot 1 shows `tech_id 12` as the **sole** owned card at a moment strictly between the University and Ball acquisitions ⇒ **12 = University**. `tech_id 14` is German/5, a class that is uniformly Exposition ⇒ **14 = Exposition**. `tech_id 2` is Mining Technology by the direct evidence above. The remaining owned card, `tech_id 26` (English/3), is therefore **Ball** — by elimination against the four-acquisition list.

### 4.4 The resolved table (titles only) and the gaps

| BGA `tech_id` | class | engine **title** | basis | strength |
|---|---|---|---|---|
| 2 | German/3 | **Mining Technology** | `techAcquisition` carrying `tech_name` + `tech_card` (pkt 127) | **DIRECT** |
| 12 | English/3 | **University** | ownership timeline (sole owned card between pkt 6 and pkt 31) | DERIVED |
| 26 | English/3 | **Ball** | ownership timeline + elimination over the four acquisitions | DERIVED |
| 14 | German/5 | **Exposition** | class is uniformly Exposition (×2) | FORCED by class |
| 24 | German/5 | **Exposition** | class is uniformly Exposition (×2) | FORCED by class |
| 9 | German/3 | **Brickyard** | class = {Mining Technology, Brickyard}; `tech_id 2` is Mining Technology | FORCED by elimination |
| 3 | French/4 | **Electrical Light** | singleton class | FORCED by class |
| 4 | American/5 | **Tram** | singleton class | FORCED by class |
| 11 | American/3 | **Language School** | class is uniformly Language School (×2) | FORCED by class |
| 21 | American/3 | **Language School** | class is uniformly Language School (×2) | FORCED by class |
| 17 | Dutch/5 | **Newspaper** | class is uniformly Newspaper (×2) | FORCED by class |
| 27 | Dutch/5 | **Newspaper** | class is uniformly Newspaper (×2) | FORCED by class |
| 1 | English/4 | **UNOBSERVED** | Station *or* Textile Mill | — |
| 7 | English/4 | **UNOBSERVED** | Station *or* Textile Mill | — |
| 5, 20, 30 | Dutch/3 | **UNOBSERVED** | Letterpress Printing ×2 *or* Stock Market | — |
| 6, 16, 18, 22, 28 | English/3 | **UNOBSERVED** | the residue of {University, Ball, Postal System ×2, Stagecoach} after 12 and 26 | — |
| 8, 10, 15, 25 | French/3 | **UNOBSERVED** | Gaslight ×2, Winery, Patent System | — |
| 13, 19, 23, 29 | American/6 | **UNOBSERVED** | Telegram ×2, Telephone ×2 | — |

**12 of 30 `tech_id`s have a determined title; 18 are UNOBSERVED.** By title: 9 of the engine's 20 titles are evidenced (Mining Technology, University, Ball, Exposition, Brickyard, Electrical Light, Tram, Language School, Newspaper).

**Every FORCED-by-class row is conditional on §4.2's multiset agreement and on §3's country map** — both well-supported, neither directly observed for that particular card. H1's table should record the basis per row, so a future contradiction is diagnosable rather than mysterious.

### 4.5 Two things H1 must decide rather than discover

1. **Title vs copy.** For a duplicated title the corpus can never say *which* engine `TechCardId` a `tech_id` is (`tech_id 14` and `24` are both Exposition; only one of them can be `TechCardId(9)`). The two copies are rules-identical, so the choice is free — but it must be **deterministic and stated**, not incidental, or two importer runs produce different `GameRecord`s for the same game.
2. **Effect text.** The corpus carries **no** tech effect text at all. `techAcquisition`'s log describes the *acquisition rewards* ("gains: `Great Britain` flag, 1 Foreign Agent", "10 VPs"), never what the card does. The negative result the ledger already records — no bulk static card table exists client-side — means effect text is **not** recoverable from captures; the title is the only bridge to `TechEffect`, and an unresolved title must be a **hard error** at import, never a defaulted effect.

### 4.6 The honest path to closing the 18

Nothing about the gap is structural — it is *sample size*. Each future captured game that includes a `techAcquisition` **delivered over the WebSocket** resolves one more `tech_id` directly, because the live-form args carry `tech_card` while the history-form args do not (§4.3's table is exactly that contrast: one ws-sourced acquisition has the card, three history-sourced ones do not). **That is a concrete capture instruction, not a hope:** techs resolved during live-follow are worth more than techs read out of a history replay, so a capture session should be running before a Laboratory action, and a game imported purely from history yields names without ids.

---

## 5. Coverage statement

| | evidenced by this one game | of the engine's full set |
|---|---|---|
| **Orders — distinct BGA `order_id` observed** | **20** | — |
| **Orders — matched to an engine card** | **20 (100% of observed)** | **20 / 36 engine orders (55.6%)** |
| **Orders — ambiguous or unmatched** | **0** | — |
| **Techs — distinct BGA `tech_id` observed** | **30 (all of them)** | 30 / 30 physical cards |
| **Techs — title determined** | **12** | **9 / 20 engine titles (45%)** |
| **Techs — UNOBSERVED (title-ambiguous)** | **18** | — |

**Engine orders NOT evidenced here:** O01, O05, O11, O12, O14, O16, O18, O19, O22, O25, O26, O28, O33, O34, O35, O36 — **exactly 16**.

**That 16 is not a coincidence, and it is a rules cross-check, not a coverage lament.** `GameConfig::order_cards_to_remove()` removes **16** order cards at 2 players (36 − 16 = 20 in play), and this BGA game used **exactly 20 order instances** — `order_key` 1..20, contiguous, with no key 21 anywhere in the corpus and `eog_triggers.deck_count: "1"` at the end. **The engine's 2P order-deck reduction agrees with BGA's, independently confirmed.** So 20/36 is not a shortfall of the capture: it is the *entire* card set a 2P game contains. Full order coverage needs 3P/4P captures (4P removes none), or several more 2P games.

---

## 6. Structural facts the mapper and H2 need, found while doing this

These are corpus observations, recorded here because they were established by the same pass and would otherwise be re-discovered expensively.

**6.1 Location codes.** `location` on both orders and techs: **1** = deck, position always `"0"`; **2** = the face-up board display, position `1..6`; **4** = held by a player (`player_id` set); **5** = the setup discard (observed only on the two `orderDiscarded` cards). Both the Port and the Laboratory display **6 slots**, matching CLAUDE.md's board-art verification ("port boards have SIX card slots… lab boards SIX tech slots").

**6.2 The port refill rule is confirmed externally.** Tracing the display across `orderAcquisition` → `orderRefresh` pairs:

```
pkt 115  [11, 33,  3, 28, 30, 34]
pkt 119  [11, __,  3, 28, 30, 34]   ← acquisition opens a hole at slot 2
pkt 119  [11,  3, 28, 30, 34, 13]   ← refresh: LEFT-COMPACT, new card at slot 6
pkt 137  [__,  3, 28, 30, 34, 13]
pkt 138  [ 3, 28, 30, 34, 13, 26]
pkt 155  [ 3, __, 30, 34, 13, 26] → [ 3, 30, 34, 13, 26, 29]
pkt 184  [__, __, 34, 13, 26, 29] → [34, 13, 26, 29,  5, 10]   ← two taken, two refilled
```

This is CLAUDE.md's "port refill is rightmost-first after total left-compaction", observed in an independent implementation. `position` is therefore a **volatile slot index, not a card property** — a mapper keying on it would be wrong within one turn.

**6.3 The LOW/MID/HIGH port zone is NOT a BGA field.** BGA carries only the 1..6 slot index. The engine's "2 slots per zone" grouping has **no evidence in this corpus** — do not guess which slots form which zone. H1 either derives it from a zone-dependent effect observed in a future capture, or treats it as an owner question.

**6.4 `gamedatas` freshness is NOT uniform across its fields — a checkpoint hazard.** The mid-game snapshot captured at `1786156733` reports `notifications.last_packet_id: "104"` (packet 104's own timestamp is `1786156668`), yet its `techs` map already contains the Exposition acquired at **packet 107** (`t=1786156687`). The `last_packet_id` marker and the card/board maps are **not guaranteed to be at the same packet position**. H2 must anchor a checkpoint on notification-derived position and treat `gamedatas` counters as "at or after" that marker — a strict-equality cross-check against `last_packet_id` will produce spurious divergences.

**6.5 Packet coverage of this corpus is 179 of 211 ids — 32 are MISSING** (7, 10, 21, 26, 27, 32, 35, 39, 41, 43, 49, 52, 56, 58, 60, 63, 64, 68, 69, 74, 77, 78, 80, 82, 86, 88, 90, 92, 94, 99, 102, 103). The four history XHRs were fetched with `from=2/17/112/213` and the gaps sit between them; the WebSocket stream only began after the extension+tab reload. **This corpus is therefore NOT a complete move stream** and cannot alone drive an end-to-end catch-up replay. It is complete enough for card mapping (every card identity is redundantly carried by the `orders`/`techs` bulk maps that ride many notifications) — but H2 needs a capture whose history fetch is taken **`from=0` in one shot**, and that is a capture-procedure requirement, not a translator problem.

> **STALE — re-measured at the H2/H3 close (six-lens lens-4 finding; T2(d) annotate-in-place precedent).** Both the count and the derived requirement above are wrong. Measured over `bridge-captures/895724349.ndjson`: the four history XHRs (`from=2/17/112/213`, 1/16/106/187 packets) union to **199 of 212** packet ids, missing exactly **THIRTEEN** — {27, 64, 69, 102, 103, 122, 123, 129, 130, 148, 157, 158, 169} — and **`move_id` runs 1..194 with ZERO gaps** (the `from=213` response is CUMULATIVE, 187 packets covering m13..194, not a window between fetches). History ∪ socket reaches 207/212 (missing only {27,64,69,102,103}). The 13 absent ids are the SUPERSEDED packets from the nine takebacks, not fetch gaps — 27 of the 32 ids listed above as "MISSING" are in fact PRESENT with empty `data[]`, and 8 genuinely-absent ids (122,123,129,130,148,157,158,169) were omitted from that list. **The `from=0`-in-one-shot capture-procedure requirement is WITHDRAWN**: H2's shipped acceptance run replays 153/194 moves FROM MOVE 1 on exactly this corpus, which §6.5 called impossible. This agrees with the two docs committed alongside this one — `2026-08-08-bga-notification-inventory.md` §7 (same 13-gap set, "`move_id` runs 13…194 contiguously") and `2026-08-08-phase3.5b-bridge-plan.md` §0 ("the union of the four covers 1..194 with no gaps"). `docs/bridge-setup.md` §6's runsheet is correct as written; it needs no `from=0` step.

**6.6 Supersession, for the record.** The fullest history response carries 9 `undoRestorePoint` notifications, and the move_id regressions the ledger names (118→117, 123→122, 148→147) are in this data. Card identity is unaffected by supersession — a rewound acquisition still tells the truth about *which card* — so the mapper may run ahead of the H2 supersession decision.

---

## 7. Hidden-information findings — read before H1 writes the importer

Three things in `gamedatas` are visible to the capturing client that a seated player at a physical table does **not** know. None of them is a rules divergence; all of them are **honesty hazards for the record the importer emits**, which is live-mode semantics with the opponent's hand hidden.

**7.1 The order deck is fully identified, IN DRAW ORDER.** The first snapshot lists 8 cards at `location: "1"` (keys 13..20 = ids 30, 34, 13, 26, 29, 5, 10, 2). The subsequent refills arrived in **exactly ascending `order_key` order** — 30, 34, then 13 (pkt 119), 26 (pkt 138), 29 (pkt 155), 5 and 10 (pkt 184) — leaving id 2 as the single card matching `deck_count: "1"` at the end. So `order_key` is the deck position and BGA hands the client the whole future deck. The engine's own `GameState.order_deck` is the same information (CLAUDE.md's hidden-information paragraph), so populating it from the capture is *representationally* fine and makes replay faithful — **but it must never be routed into a `PlayerView`, and the decision to use it has to be explicit in H1's design rather than a side effect of copying the snapshot.**

**7.2 The opponent's DISCARDED setup order is revealed.** `orderDiscarded` at packet 3 carries acmuriel's full card (`order_id 25` → O27) in the history XeroxMan's client fetched. In the physical game that card goes back unseen. Combined with 7.1, the opponent's *kept* starting order becomes derivable by elimination. **The importer must not derive it.** The engine's live-tracking model is `starting_order_hidden = true` with both dealt cards in `orders_removed`, and that is the model the record must carry — `StartingOrderChoice::Unknown` for the opponent seat, regardless of what the capture would permit.

**7.3 The opponent's held order was visible at the end-of-game snapshot** (`order_key 12`, id 28, `completed: "0"`, `player_id` acmuriel, matching his `orders_hand: "1"`). This is almost certainly ordinary end-of-game reveal — **the corpus cannot distinguish that from a mid-game leak**, because at both mid-game snapshots acmuriel's `orders_hand` was `0` and the question never arose. **Design §6's private-info check is therefore NOT settled by this corpus.** The test that settles it: a mid-game snapshot taken while the opponent holds an uncompleted order. Until then, treat opponent hand contents as hidden by policy, and note that everything the opponent acquired in this game came from the **face-up** port display — publicly known at the table too, so no `orderAcquisition` observed here is evidence of a leak.

---

## 8. What H1 may and may not take from this document

**MAY:**
- Match orders by the requirement+reward vector, with the uniqueness property (§1) asserted as a test over `data.rs` rather than trusted — it is a property of card data that a `gen-data` run could in principle break.
- Assert the §2 pairs as regression fixtures, including the five design-§1 pairs (§2.1) as a cross-table stability check.
- Use the §3 country map, with code 1 annotated as elimination-derived (§3).
- Seed the tech table with the **12** rows of §4.4, each carrying its basis string.

**MAY NOT:**
- Ship any of the 18 UNOBSERVED tech rows. An unresolved `tech_id` at import is a **hard error naming the `tech_id`, its class, and its candidate titles** — the bridge's standing hard-error-never-guess posture, and the one place where a plausible default would silently corrupt a record.
- Assume a port zone grouping (§6.3).
- Populate the opponent's starting order from the capture (§7.2).
- Treat the `orders`/`techs` maps as frozen at `notifications.last_packet_id` (§6.4).

---

*Provenance: table #895724349, captured 2026-08-08 by the H0 extension + relay; analysed 2026-08-08 against `data.rs` at branch `phase3.5b-bga-bridge`. Like ledger note N-6, the BGA-side facts here are dated observations carrying their table id — a reviewer cannot re-derive them from the tree without the gitignored corpus.*
