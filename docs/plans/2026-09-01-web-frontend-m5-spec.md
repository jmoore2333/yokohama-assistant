# Web frontend — M5 "the table": the spec

**Written 2026-09-01 on branch `web-frontend`, at `22f35a0` — the M4 deploy head plus
four docs commits. The client at this head is byte-identical to the image serving
`yokohama.moore.nyc` (`git diff 641cb40..HEAD -- web crates/yokohama-web` is empty).**

**Every file:line in this document was opened at `22f35a0` while writing it.** The
lane's documented defect class is a document asserting something that is not true;
M4's spec §2 lists what its predecessors got wrong. Nothing here is copied forward
from an earlier document without being re-opened.

---

## 0. The problem, measured

The owner's complaint, in the owner's words: *"a couple of turns in … my viewport is
quite limited, and if I want to see what lab tech cards are available, or order cards
from the port are available, it's several clicks and scrolls … the player's eyes have
such a small amount of real estate."*

Measured on the live deployment on 2026-09-01, in Chrome on the owner's machine
(**1710×869 CSS px, device pixel ratio 2** — a 16-inch MacBook Pro at its default
scaling with the browser chrome taken out), on the owner's own saved game at turn 3:

| Region | Reading | Where it comes from |
|---|---|---|
| Banner + end-trigger strip | 56 + 28 = **84px** | `styles.css:98-118` |
| Board column | 1262px wide | `.stage` flex, `.rail` fixed 24rem — `styles.css:121-123` |
| Board (2P) | **915 wide × 1100 tall**, four rows of 261px | 4 tiles at 14rem — `Board.tsx:30-35` |
| Tile | 224 × 261: art 166 + caption **92** | `styles.css:187-193`, `:234-238` |
| Unused width beside the 2P pyramid | **~350px** | the ladder caps at 14rem regardless of column |
| Rail | **1382px** tall (Players 375, Tracks 204, Goals 369, Hand 387) | `Dashboard.tsx` |
| Deck, pinned, two options, no messages | **209px** | `styles.css:421-430` |
| Deck after a bot move (16 message lines) | **334px** | `Decision.tsx:360` |
| Board visible above the deck | **556px / 431px** (1.6 of 4 rows) | |
| Page height | 1715px | |
| Port offer drawer | modal, 704 × 773; six cards at 128px; body 697 client / 748 scroll | `styles.css:535-566`, `Surfaces.tsx:333` |
| Laboratory offer drawer | body 697 / 824 | `Surfaces.tsx:382` |
| Orders index drawer | body 706 / **2870** | `Index.tsx:238` |

Three things compound:

1. **The pyramid is tall in a wide viewport** and its width is capped by a ladder that
   knows nothing about the column it sits in (`Board.tsx:30-35` picks 10/12/14rem from
   the widest row alone).
2. **The pinned deck takes the page's height budget from the board whatever the board
   does.** M3.5 measured this the other way round — `.stage` is `max(board, rail)` and
   the rail runs 2.1–2.5× the board — and pinned the deck so the answer never costs a
   scroll. That was right, and it left the board with 431px.
3. **Every reference surface is a modal drawer.** Reading the port while placing means
   open, scroll, Escape; the page is inert while it is open, by design
   (`Surfaces.tsx:193-231`).

A browser zoom of 80% does not fix it: measured arithmetic gives 2.4 rows visible,
because the structure and not the scale is the constraint.

### 0.1 The throwaway mock, and what it proved

CSS injected into a local copy of the same build (nothing committed, nothing shipped):
`.app` as a two-column grid (`minmax(0,1fr) 31rem`), `.stage` as `display: contents`,
the deck in the right column's first row, the rail in its second with `overflow: auto`,
the board at 11rem with the caption's first two rows overlaid on the art.

| Reading | Value |
|---|---|
| Board rows (y / height) | 98, 284, 471, 657 / **180 each**; last row's bottom 836 |
| Page scroll height | **869 = viewport** — no page scroll |
| Deck in the side column (messages hidden) | 222px |
| Rail | 563px visible, 1294 content, scrolls independently |

Two things the mock got wrong are the design work below: the overlay hack collided
with the id chip, and a Port offer cloned verbatim into the side column ran **2013px
tall** (one card per row) — a docked surface needs its own shape.

---

## 1. The rulings (owner, 2026-09-01, in conversation over the mock)

| # | Question | Ruling |
|---|---|---|
| **R1** | Where does the answer bar live above 1201px? | **At the top of the right-hand column**, pinned. Not the bottom edge, not under the board column. |
| **R2** | What is the left region? | **A scrolling TABLE**: the pyramid first, then the real card areas laid out under it the way they sit on a physical table. On a wide screen they sit side by side under the pyramid and nothing scrolls; on the owner's screen the table scrolls. *"these items will likely need the space that the game area when scrolling will provide … if I were using an even larger display I could probably fit it all under the game area cards."* |
| **R3** | Which surfaces stay drawers? | **The big references**: the four card indexes, Suggest, Why, Log with the scrubber, Help, Final score. Everything else that is a drawer today becomes a table section. |
| **R4** | The tile's caption | **Both caption lines go.** The tile is the art with the pieces on it: *"if we placed the building cards onto the building areas like so, and overlay the pawns on the art to show what's used, with the right color, and show the president and workers as art on the cards themselves, this would be closest to how the game actually looks in the physical space, and be what many users are intuitively expecting."* This REVERSES the opening brief's *"we don't have other goals like having building cards on the areas themselves"* — ruled after seeing the mock, recorded as such. |
| **R5** | Growth | **Centered, and things grow to the room they have.** *"there is operationally more of a floor for how large things need to be before they break but I'm also focused on the ceiling."* |
| **R6** | The management boards' art | **Deferred.** The port, laboratory, church and customs sections draw the board SHAPES (slots, zone tabs, surcharge marks, spaces) without board art in M5; the art is a named upgrade (§6). |
| **R7** | CI and tests | **In this spec** (§4), not a follow-up. |
| **R8** | Below 1201px | **Unchanged.** M3.5's column stack with the bottom-pinned deck stands, with its cells. |
| **R9** | How the build is checked (owner, second pass) | **Visual gates in the process**: *"visual validation should be added into gates in the process to make sure this is on the right track as you build, if you are not highly confident you can also have these gates stop to ask me to see how it looks at different resolutions."* §4.1. |
| **R10** | The achievement cards | **Drawn landscape** — *"the achievement cards right now are not oriented horizontal, so I imagine once that is built as a goals section in the main view area, it will be oriented correctly."* Verified: the sheets store them sideways (§2); W0 rotates. |
| **R11** | Session control | **Undo keeps working, and the banner gets a session control** (saved games, new game, switching) in M5. Called out by the owner as the seed of **M6**: *"a section for player chat, lobby, game switching, loading game saves, opening game IDs from other players … from a drop down or modal layer."* §3.7, §6. |
| **R6′** | The management boards' art, second pass | The owner left it to this spec: *"if the management boards not having art in M5 is a decision you want to make, I'm ok with that … fit into an M6 or M7 future feature set."* **Deferred to M6/M7** — the crop is easy, the slot-rect measurement per board is W1-sized work again, and M5 is already the lane's largest client change. |

Carried forward, not re-opened:

- **Caption-below** (M3.5 F1) is moot under R4 — there is no caption. The ruling's
  substance, *no text over the printed ladder*, stands: nothing in §3.2 touches the
  bottom 37% of the art.
- **The 10rem legibility floor** (M3.5 §2.3: a rung numeral at 10rem renders 7.90px of
  cap height) stays the floor.
- **The scrubber lives in the Log drawer, the board on the stage** (M4 Q1). Unchanged;
  §3.6 says what the table sections do while replaying.
- **`/at/{k}` is a narrow `HistoricalFrame`** (M4 Q5) — which is why they do it.

---

## 2. What was verified against the assets

- **There is no printed frame for the site card on the area art.** All 13 non-Canal
  crops (`crates/yokohama-web/assets/art/areas/`, 440×330) were laid out on one sheet
  with candidate rectangles drawn on. The top-right quarter (x 52–97%, y 5–44%) is
  illustration on every one; the printed rules sit top-left (icon blocks reach y≈0.57
  on Church, Chinatown, Laboratory and Research Center), bottom (the ladder, y≥0.63)
  and at the measured 5-space (`styles.css:207-213`: x 81.6–94.1%, y 58.8–72.1%). The
  owner's photograph shows the physical site card placed over that top-right region,
  rotated a quarter turn with the trading-house band outward.
- **The site card art is portrait** (360×543, `sites/S00..S23.webp`), two columns of
  two spaces over a trading-house band; the built-space rects are already measured
  (`styles.css:915-943`).
- **The achievement art is cut SIDEWAYS.** `tts-assets/organized/cards/achievement_A_3x2.jpg`
  (1590×1598) stores the six faces portrait with their numerals rotated a quarter turn;
  `mkcardart.py:103-105` slices the sheet with no rotation, so every crop
  (`achievements/*.webp`, 360×543) is a landscape card on its side, and
  `AchievementCardView` (`Cards.tsx:149`) draws it that way in the rail. The physical card
  is landscape (the mat image's A/B/C slots). Reading the crop upright needs a 90°
  counter-clockwise turn.
- **No piece art exists.** The TTS mod models assistants as `BlockSquare` (cubes) and
  carries `PlayerPawn` objects that are TTS avatars, not game presidents
  (`tts-assets/asset_mapping.json`, `objects_by_type`). Pieces are drawn, not cropped
  (§3.2).
- **Management-board art EXISTS at modest resolution.** `tts-assets/images/full top
  board.jpg` (4400×2600) is the table mat: Port A/B, Church, Customs, Laboratory A/B
  boards at roughly 700×230px each, around the VP track. Not in the manifest, not
  cropped. §6.
- **Two of the three wide sections already have components:** `Ports`
  (`Surfaces.tsx:333`), `Labs` (`:382`), `Tracks` (`:664`), `BuildingSites` (`:754`),
  `SiteCard` (`:780`) — all laid out for a 704px drawer, none for a strip.

---

## 3. The design

### 3.1 Regions, above 1201px

`.app` becomes a grid: `grid-template-columns: minmax(0, 1fr) var(--side)`, rows
`auto auto auto minmax(0, 1fr)`, `height: 100vh`. Banner and end-trigger strip span
both columns as now (`App.tsx:192`, `:278`). `.stage` (`App.tsx:296`) is replaced by
two siblings:

- **`.table`** — column 1, rows 3–4, `overflow: auto`, `container-type: size`. Holds
  the `Board` first, centered, then `.table-sections` (§3.4).
- **`.side`** — column 2. Row 3 is the deck (`App.tsx:349`), which keeps every rule it
  has (`styles.css:421-430`: `max-height`, only `.options.scrolls` may clip) at
  `position: static` — pinned by the grid, not by `sticky`. Row 4 is the former rail
  (`App.tsx:310`), `overflow: auto`: Last move, Players, Hand, in that order (§3.5).

`--side` steps with the viewport: **24rem from 1201px** (the rail's current width,
`styles.css:123` — at 1440 a 4P board at the 10rem floor is 992px wide and a wider
column would push it past the table's 1024), **30rem from 1600px**, **36rem from
1900px**. The option rows that measure ~780px at 13px mono (`"Option A — place 1
assistant each in up to 3 areas (pick areas; 'a' grammar works too)"`) wrap to three
lines at 24rem, two at 30rem, one at 36rem.

**The drawers are untouched** (`Surfaces.tsx:136-331`, `styles.css:535-566`). They
open over the right side of the viewport as today; the board is on the left and stays
readable beside them, which was the drawer's whole reason. The rule
`.app:has(.surface[open]) .deck { padding-right }` (`styles.css:566`) is deleted — the
deck is no longer under the drawer's edge; it is under the drawer, and the page is
inert while a drawer is open, so nothing is lost.

**Below 1201px** the `@media (max-width: 1200px)` block (`styles.css:125-141`) keeps
the column stack and the bottom-pinned deck. The grid above is inside
`@media (min-width: 1201px)`; the sub-1201 DOM order is unchanged so its cells stand.

### 3.2 The tile is the art

`Tile` (`Board.tsx:153`) loses `.tile-body` (`:221-281`) and `SiteSlots` (`:110`).
`Legend` (`:292`) stays, below the board, re-worded for the pieces. The tile is the
`.tile-art-box` (`:199`) at the art's 4:3, with these overlays, each positioned in
percentages of the box:

| Overlay | Rect (of the tile) | Source | Notes |
|---|---|---|---|
| **Building-site card** | x 54–97%, y 4–42% | `siteArt(art, tile.site.index)` (`art.ts:34`) | Rotated **−90°** (top edge to the left, trading-house band to the right), the owner's photo. The four `site-space` rects and `site-trading-house` (`styles.css:939-943`) ride inside the rotated box unchanged, so a built space shows in its owner's colour on the printed space. `site.index === null` draws the typed `card-blank` as `SiteCardView` does (`Cards.tsx:225`). |
| **Pieces** | x 52–80%, y 44–58% | `tile.presidents`, `tile.assistants`, `tile.has_station` (`types.ts:91-94`) | §3.2.1 |
| **5-Power token** | as measured, `styles.css:207-213` | unchanged | The empty printed slot is the "taken" state, as today. |
| **Id badge** | x 50–57%, y 3–10% | `A${area}` | The printed title is the name (`Board.tsx:178`'s "the picture, then the caption" — the picture already carries the name). The id is what every prompt says (`"Church (A00)"`), so it stays on the tile, in the clear gap right of the longest printed title ("Employment Agency" ends at x≈0.49). |
| Reachable / selected | border treatments | `styles.css:240-241` | unchanged |

**The rects are candidates, to be measured the way the token slot was**
(`styles.css:195-206`: each art minus its own box blur, averaged, so a printed line
survives and every illustration cancels). The plan's first task computes, per art, the
"busy" mask of printed rules (icon blocks, ladder, 5-space, title) and asserts every
overlay rect above lies outside it on all 13 arts, with the sheet committed under
`docs/plans/` as evidence. A rect that fails on one art moves for all of them — the
overlays are one geometry, not thirteen.

#### 3.2.1 Pieces

Drawn as inline SVG in the seat's colour (`seatClass`, `Cards.tsx:24`) with a dark
outline and the existing `text-shadow` treatment's opacity: a **president** as a pawn
silhouette, **assistants** as cubes — the shape the TTS mod uses; to be confirmed
against the physical box before the plan freezes the glyph — up to three cubes drawn,
a count badge from four. The **Station** draws `tokenArt(art, "station")`
(`art.ts:48`; `tokens/station.webp`). The 2P dummy never appears on a tile
(`DUMMY_SEAT` is a track occupant only, `Dashboard.tsx:213-236`), so no dummy piece.

Everything `Pawns` (`Cards.tsx:251`) says in words — `title` attributes naming the
seat and the count — moves onto the piece group as its accessible name, so a screen
reader loses nothing the caption used to say.

#### 3.2.2 The popover

At the 10rem floor the site card is ~70px wide, below what its printed values need.
So a tile's hover or keyboard focus opens a non-modal popover (the Popover API,
`popover="manual"` anchored to the tile) showing the site card at 12rem, the area's
yield ladder in words (`yieldWords`, `Surfaces.tsx:430`), and the pieces with their
counts in words. It is the `Areas` drawer's row for one tile, brought to the tile.
Escape and blur close it; it never takes focus; it is not in the tab order.

### 3.3 The fit rule

One rule replaces `tileWidth` (`Board.tsx:30-35`):

```
rowH   = (100cqh − padding − 3·gap) / 4          -- four rows on every board
byH    = rowH · 4/3                               -- the art's aspect
byW    = (100cqw − padding − (n−1)·gap) / n       -- n = the realized widest row
tile-w = clamp(10rem, min(byH, byW), 22rem)
```

- `n` is still keyed on the **realized widest row**, never `player_count`
  (`Board.tsx:8-16`'s argument holds).
- **Four rows** is the one shape assumption this rule makes, and the engine's boards
  are all four rows (`board_gen`: 4/3/2/1, 5/4/3/2, 6/5/4/3). It is stated as `--rows`
  on `.board` from the realized row count, so a board with a different count still
  fits — the rule assumes nothing the data does not say.
- **10rem** is M3.5's measured floor. **22rem** is where a 2× display reaches the art's
  SOURCE width (709px per tile on `tts-assets/organized/boards/area_boards_base_3x4.jpg`,
  2128×2124 for a 3×4 sheet, ÷ 2 = 354 CSS px ≈ 22.1rem); past it the art upsamples.
  **The shipped crops are NOT at that width**: the manifest records `tile_width: 440`
  and `card_width: 360` (`manifest.json`, `provenance`), cut by `docs/tools/mkcardart.py`
  from 709×531 tiles and 530×799 cards. At the owner's 15.4rem on a 2× display a 440px
  crop is already upsampled 1.12×; at 22rem it would be 1.6×. So **W0 re-cuts the crops
  at source width** (tile 709, card 530) with the same generator, and the payload
  growth is recorded against the runbook's 3.0 MB (`deploy/RUNBOOK.md` §3.4). Without
  W0 the honest ceiling is 13.75rem, below what the owner's own screen fits.
- Computed in CSS from container-query units on `.table`, so there is no resize
  observer and no layout pass in JS. **The same formula is also a pure TS function**
  (`tileWidthFor(widest, rows, tableW, tableH)`) that exists ONLY as the test oracle:
  the e2e cell computes it from the measured container and asserts the rendered tile
  width matches within 1px. M3.5's audit found a CSS mechanism *"validated by a metric
  insensitive to the way it fails"* — sampling one row of four; the oracle measures
  every row.

Predicted, to be measured at the plan's close:

| Viewport | Side | Table (w × h, after 1rem padding each side) | 2P tile | 4P tile | Fits without scroll? |
|---|---|---|---|---|---|
| 1710×869 (owner) | 30rem | 1198 × 785 | **15.4rem** (height-bound) | 12.1rem (width-bound: 6 tiles) | 2P yes, 4P yes |
| 1440×900 | 24rem | 1024 × 816 | 15.7rem (width-bound) | 10.3rem (width-bound, just above the floor) | both |
| 2560×1440 | 36rem | 1952 × 1356 | **22rem** cap (height would allow 27) | 20.0rem (width-bound) | both, with the port and lab strips beside each other under the pyramid |

When the height-bound result is below the floor, the table scrolls; the deck is not in
the table, so that scroll costs nothing the M3.5 pin was protecting.

### 3.4 The table's sections

Under the pyramid, `.table-sections`: `display: flex; flex-wrap: wrap;
justify-content: center; gap: 1rem`. Each section is a `<section class="table-sec">`
at its natural width (`max-content`, capped at `100%`), so two strips sit side by side
where the table is wide enough and stack where it is not. A section that is off
renders nothing.

| Section | Shape | Data | Notes |
|---|---|---|---|
| **Port A / Port B** | one strip per board: six slots in a row; above each, the zone tab (LOW/MID/HIGH) and the power gate; in each, the order card at `--card-w` with its requirement and reward chips beneath | `api.ports` (`api.ts:99`), `Ports`' zone fold (`Surfaces.tsx:334-345`) | The physical board is a strip of six; the owner's photo. Both halves always drawn (`Cards.tsx:1-10`). |
| **Laboratory A / B** | six slots in a row; the surcharge mark `+1 +1 +2 +2` over slots 3–6 (`LabSlotModel.surcharge`, `types.ts:432`); the tech art and its effect line | `api.labs` (`:100`) | |
| **Tracks** | church spaces 4–11 and customs' seven, as `Tracks` draws them (`Surfaces.tsx:664`), each space with its VP and rider printed, the occupant on it; the mercenary supply and the Station beside | on every frame — `App.tsx:160-162` | Replaces the rail's `Tracks` panel (`Dashboard.tsx:259`), which leaves the side column. |
| **Goals** | the three achievement cards **landscape**, in a row, each with its first/later VP and per-seat progress (`AchievementCardView`, `Cards.tsx:149`, re-laid for a landscape face) | on every frame | Replaces the rail's `Achievements` panel (`Dashboard.tsx:350`). The face is landscape because W0 cuts it so (R10); the component gets its own `--achievement-aspect` (799/530) rather than the portrait `--card-aspect` (`styles.css:68`). |
| **Building sites** | the 24-card grid `BuildingSites` draws (`Surfaces.tsx:754`), at `--card-w` | `api.areas` (`:101`) | The tile shows the small card; this is the big one. |

`--card-w` follows the tile: `clamp(8rem, calc(var(--tile-w) · 0.75), 12rem)`, so the
cards grow with the board (R5) and a 12rem order card at 2× is at the art's native
width (360px source).

**Toggles.** The banner's surface buttons (`App.tsx:216-225`) split: the five sections
above become `aria-pressed` toggles with the lit style `.views button.on` already has
(`styles.css:108`); the drawer surfaces keep opening drawers. The enabled set persists
in `localStorage` under one key, read inside `try/catch` and defaulting to
`{port, lab, tracks}` when absent or unreadable. Persistence is a per-browser
convenience, not state — a game opened in another browser shows the default set.

**Refresh.** `Surfaces.tsx:1-8` fetches the four GETs on demand because *"none of them
changes between decisions in a way a player is watching for"*. A section on the table
is exactly that watching, so an enabled section re-fetches after every `setState` from
`/input`, `/games/{id}` and `create` (`App.tsx:79`, `:98`, `:160`) — at most three
small GETs per applied move, on the same debounce-and-drop pattern the scrubber uses
(`App.tsx:116-149`: a `live` flag, the stale response dropped). Tracks and Goals need
no fetch; they are on the frame.

**Replaying** (`replaying`, `App.tsx:177`): `/at/{k}` carries board and dashboard only
(M4 Q5), so the fetched sections cannot show the position the board is showing. While
replaying, each enabled fetched section collapses to one line — *"live offer, hidden
while replaying"* — and Tracks/Goals draw from the historical frame as the rail does
today. Nothing pretends.

### 3.5 The side column

Top to bottom, under the pinned deck:

1. **Last move** — `state.messages`, moved out of `Decision` (`Decision.tsx:360-366`)
   into a `<details open>` section capped at ~8 lines with its own scroll. The
   `role="status"` container stays mounted and empty-collapsed exactly as the current
   one does (`styles.css:445-447`), because a live region inserted with its text is
   unreliably announced. `isRefusal` styling (`Decision.tsx:392`) comes with it.
2. **Players** (`Dashboard.tsx:92`), unchanged.
3. **Hand** (`Dashboard.tsx:379`), unchanged.

Tracks and Goals leave the column for the table (§3.4). `Dashboard` still renders an
unknown panel as its JSON (`Dashboard.tsx:57`), so a fifth panel variant lands in
the column rather than nowhere.

### 3.6 What the drawers do now

Nothing new. A drawer opens modally over the right of the viewport
(`Surfaces.tsx:193-231`); the board is left of it and full size; the M4 scrub cells
(`the_screen_itself.spec.ts:812`, `:869`) keep their meaning. The one visible change
is the deck under the drawer instead of beside it, and the page is inert then anyway.

### 3.7 Session control, and what M6 brings (R11)

**Undo is untouched.** It is a line through `/input` like every other answer
(`Decision.tsx:350`, `onSend("undo")`), the server's compare-and-swap persist is the
guard (`games.rs`, the row at `guidingpath.md:205`), and neither moves. What M5 adds is
a cell that proves the NEW tile reflects an undo: pieces and a built mark that a move put
on a tile leave it when the move is unwound (§4, W6).

**The banner gets a session control.** Today the banner's meta span
(`App.tsx:194-197`) prints `mode · id · saved` and the only way back to the setup
screen — saved games, new game, delete — is a page reload (`App.tsx:90`'s own words:
*"the only escape was a page reload back into setup"*). M5 makes the meta span a
`<details>` menu in the `cards-menu` idiom (`styles.css:880-896`) named **Game**:

- the current id, mode and saved state, as now;
- **Saved games** — `api.list` (`api.ts:82`), each a button that `api.get`s it and
  replaces `state`, exactly as `NewGame` does (`NewGame.tsx:208`);
- **New game** — returns to the setup screen (`setState(null)`), which already lists
  and deletes saves (`NewGame.tsx:197-220`); nothing is deleted from the menu itself.

That is the whole of M5's session surface, and it is deliberately the smallest thing
that removes the reload. **M6 is the multi-game layer** the owner named — lobby, player
chat, switching between games, loading saves, opening another player's game id — and
it arrives *"from a drop down or modal layer"*. The Game menu is the drop-down it grows
from: M5 leaves the banner's right side (between the nav and "to move") for it, and
puts no other control there.

---

## 4. The work, with its guard

Each item names the test that fails before it and passes after. `npm test` and
`npm run typecheck` are the CI gate (`test.yml:54-68`, M3.5 F4); the browser walk is
local by that ruling and stays so — **CI still cannot see a layout regression**, and
the workflow file says so. R7 puts the test changes here; it does not re-rule F4. If
the owner wants the walk in CI, that is a ruling for the close, costed separately
(a Chromium install step and ~4 minutes).

| # | Item | Guard |
|---|---|---|
| **W0** | Re-cut the art at source width with `docs/tools/mkcardart.py` (`tile_width` 440 → 709, `card_width` 360 → 530), **and rotate the three achievement sheets 90° counter-clockwise before slicing** so the crops are landscape (543×360-shaped, w > h) with the manifest's `w`/`h` swapped accordingly; `provenance` records the widths and the rotation; `ASSETS.md`'s row for the derived crops re-counts its size | `art.test.ts` unchanged (the join is by key). New vitest cell: every `achievements` manifest entry has `w > h`, every other card family `h > w`. The e2e picture cell (`the_screen_itself.spec.ts:17`) unchanged. The payload delta is measured and written into `deploy/RUNBOOK.md` §3.4 beside the 3.0 MB figure — a number the runbook's egress arithmetic depends on. **Then LOOK** (`mkcardart.py:66`'s own rule): the twelve achievement faces are opened and read upright before anything else builds on them. |
| **W1** | The overlay geometry: the busy-mask script, the committed sheet, the four rects | A vitest cell reads the committed rects and asserts no pair intersects and none intersects the token slot. The e2e overlay cell (W4) measures them rendered on every art. |
| **W2** | `.app` grid, `.table`, `.side`; the deck at `position: static` in the grid; the sub-1201 media block untouched; `padding-right` rule deleted | `the_screen_itself.spec.ts:62` ("pinned to the bottom") is **re-aimed**: at ≥1201 the deck's top edge is inside the viewport at every table scroll offset; at ≤1200 the existing bottom assertion runs unchanged. `:204` (only the option list scrolls) unchanged. `:511` (nothing pushes the page sideways) runs with all five sections enabled. |
| **W2b** | The Game menu in the banner (§3.7): saved games, new game; the meta span becomes its summary | Vitest: the menu lists what `api.list` returns; choosing one calls `api.get` and the board's id changes; "New game" returns to `NewGame` without a reload (no `location.reload` in the tree — asserted by grep in the test, since a reload in jsdom is a no-op that would pass). E2e: with two saved games, switch between them from a running game and read both ids in the banner. `App.test.tsx`'s existing cells stand. |
| **W3** | The fit rule in CSS + the TS oracle | `Board.test.tsx:125` ("gives boards of different widths different tiles") is replaced by the oracle's unit cells at the three viewports × three counts. New e2e cell: at 1440×900, 1710×869, 2560×1440 × 2P/3P/4P, every `.board-row`'s bottom is inside the viewport where the oracle says it fits, the rendered `--tile-w` equals the oracle within 1px, and where it does not fit the table's `scrollHeight` exceeds its `clientHeight` (the scroll is real, not clipped). Mutation-proven: with the `min()` collapsed to the width term alone, the 1710×869 2P cell must fail. |
| **W4** | The art-only tile: site card rotated in its box, pieces, id badge, popover; `SiteSlots`, `.tile-body` deleted; `Legend` re-worded | `Board.test.tsx:140-171` (token on the art) unchanged. New unit cells: a built space renders the owner's colour inside the rotated box; the piece group's accessible name reads the seat and count; the popover opens on focus and closes on Escape. E2e: `:241` (art not veiled) and `:269` (box reserved) unchanged; new cell asserts every overlay's rect lies inside the tile and outside the token slot on all 13 arts at 10rem and 22rem. |
| **W5** | The five table sections, their strips, `--card-w`, toggles, persistence, refresh, replay collapse | Vitest: each section renders six slots from the committed fixtures (`wire/ports.json`, `wire/labs.json`, `wire/areas.json`); the toggle set round-trips through a stubbed `localStorage` and falls back to the default when it throws. E2e: enabling Port then playing a move issues exactly one `/ports` request (the request counter pattern of `:812`); scrubbing collapses the fetched sections and restores them at "now"; at 2560×1440 Port and Laboratory sit on one row (their `top`s equal), at 1710×869 they stack. |
| **W6** | The side column: Last move out of the deck, Tracks and Goals out of the column | Vitest: `Decision` renders no `.messages` for a non-empty `state.messages`; the Last-move region is mounted while empty and holds every line when full. E2e: after a bot move the deck contains no message line and the Last-move region holds the same lines the deck held before (the 16-line delta measured in §0), and the deck's height is RECORDED at 1710×869 beside the 334px it measured with the messages in it — recorded, not gated, because at 24–30rem the option rows wrap and a pixel bound here would gate the wrap, not the move. **Undo on the new tile**: play a placement that puts pieces on a tile, read the piece group's accessible name, press Undo, and read it gone — the pieces are the tile's only state now, so this is the cell that says the art-only tile follows the record. |
| **W7** | Help text and screenshots | `Help.tsx:43-139` re-worded for the table and the side column; `shots.spec.ts` regenerated (`YOKOHAMA_WEB_SHOTS=1`), the pictures LOOKED AT, `web/public/help/` re-cut. `every_spec_can_skip.spec.ts` unchanged. |
| **W8** | The guide pair | This lane has no `cli-guide` twin; the README's web section and `ASSETS.md`'s screenshot row (its count of four `.png`) are re-checked after W7. |
| **W9** | Deploy | The runbook's §1–§3 (`deploy/RUNBOOK.md`), unchanged in shape: musl build, `npm run build`, context tarball, `POST /build?remote=…`, container recreate. Tagged `yokohama-web:0.6.0` — a client-visible layout change is a minor bump. |

Order: W0 → W1 → W3 → W4 (the board is the thing everything else fits around), then
W2, W2b, W5, W6 together (one layout change, measured once), then W7–W9.

### 4.1 Visual gates (R9)

The cells above measure rectangles; they cannot say whether the screen looks right.
So the plan carries **three visual gates**, each a stop in the build, not a note at
the end:

| Gate | After | What is shot | Who looks |
|---|---|---|---|
| **V1 — the tile** | W4 | One 2P and one 4P board at 1440×900, 1710×869 and 2560×1440, at turn ~5 of a real game (pieces on tiles, a site built, a token taken), plus one tile's popover | The implementer, then **the owner** — this is the gate most likely to need taste, and the site-card and piece rects were eyeballed (§2), so confidence is not high by construction |
| **V2 — the table** | W2 + W2b + W5 + W6 | The same three viewports × 2P/4P with all five sections on; then Port + Laboratory only; then a drawer open; then a replay position with the sections collapsed; then the Game menu open | The implementer; **the owner** if anything in the flow surprised the implementer or if any section wrapped where §3.3's table says it should not |
| **V3 — the help** | W7 | The regenerated `web/public/help/*.png` | The owner — they are what a reader is told the product looks like |

Mechanics: the shots are taken by a Playwright script in `e2e/` beside `shots.spec.ts`
(opt-in the same way, `YOKOHAMA_WEB_GATES=1`), written to a gitignored directory, and
**sent to the owner** (the session's file channel) with the viewport and player count in
each caption. **The rule for stopping:** at V1 always; at V2 and V3 whenever the
implementer is not highly confident — and *"not highly confident"* includes any reading
in W3's table that came out more than 0.5rem from its prediction. A gate that stops
waits for the owner; it does not proceed on its own reading.

**Counts at this head, to be re-measured at the close rather than quoted:**
`npm test` 222, `npm run e2e` 21 (+2 opt-in shots), `cargo test -p yokohama-web` 65,
wire corpus 42 (M3.5 close, `guidingpath.md:206`; M4 added cells this document does not
re-count). Neither `crates/yokohama-core` nor `SEMANTICS_REV` (7) is touched; the
server is not touched — **M5 is a client-only milestone: its `git diff` against
`crates/yokohama-core` must be empty, and against `crates/yokohama-web` must touch
nothing but `assets/art/**` (W0's re-cut) and the one achievements row of
`art.rs`'s `emitted_art_keeps_the_sheet_aspect` (`art.rs:404`), which states the
sheet aspect W0 rotates.** *(Corrected while writing the plan, 2026-09-01: the first
wording said "against `crates/`", which W0 makes false — the art tree lives inside
the web crate.)*

---

## 5. What is NOT claimed

- **No wire change, no server change.** Every datum the tile and the sections draw is
  already on the wire (`types.ts:82-100`, `:416-438`). The majority bonuses the
  physical management boards print in their corners (church 6/3, customs 8/4,
  technology 10/5) are NOT on `TracksPanel` and are not drawn; that is a wire gap,
  recorded, not filled here.
- **No layout claim below 1201px.** R8.
- **Piece shapes are unverified against the box.** The cube is the TTS mod's choice;
  the plan confirms or corrects it before freezing the SVG.
- **The overlay rects in §3.2 are candidates** until W1 measures them. The sheet they
  were eyeballed on is `docs/temp/web-review/contact-areas.png` — gitignored, so W1
  commits its own.
- **The predicted tile sizes in §3.3 are arithmetic, not readings.** They become
  readings at W3's cell, machine-tagged.
- **The 22rem ceiling is conditional on W0.** With the shipped 440px crops the ceiling
  that means "not upsampled at 2×" is 13.75rem; the spec chooses to re-cut rather than
  to cap below what the owner's own screen fits.
- **CI still cannot see a layout regression** (§4).
- **The management boards have no art in M5** (R6, §6).
- **No lobby, no chat, no multi-tenant anything.** The Game menu switches between the
  saves this server already lists to any caller (`GET /api/games`, M4 spec §5). Identity,
  another player's game, and a lobby are M6 (§6), and the M4 close's hosting blockers
  (Access header verification, the enumerable id space) are M6's first cost.
- **Nothing here is an accessibility audit.** The pieces' accessible names and the
  popover's focus rules are the two a11y properties this spec adds; M3.5's floor
  (`a11y` cells at `:314`, `:598`, `:721`) is inherited, not re-examined.

---

## 6. Upgrades this spec names and does not build

1. **Management-board art (M6 or M7, R6′).** Crop the six boards from
   `tts-assets/images/full top board.jpg` (~700×230 each) into `assets/art/boards/`,
   add them to the manifest, and draw each strip over its board with the slot rects
   measured as the token slot was. The resolution is enough for a 24rem strip at 1×,
   soft at 2×; a better source would be a scan of the owner's copy. Carve-out: the same
   footing as every other art path in `ASSETS.md`.
2. **M6 — the multi-game layer (R11).** Lobby, player chat, switching between games,
   loading saves, opening another player's game id, from the banner's Game menu or a
   modal layer over it. It is the first web milestone that needs identity, so it
   inherits the M4 close's hosting list before it can inherit anything else.
3. **Piece art** from a photographed set, replacing the SVG silhouettes.
4. **The table below 1201px** — the column stack could adopt the table's sections;
   ruled out of M5 by R8.
5. **The majority bonuses on the wire**, so the Tracks section can print what the
   physical boards print in their corners.
6. **A density control** was considered and dropped: the fit rule makes it redundant
   above 1201px, and below it is R8's territory.

---

## 7. Read order for whoever picks this up

1. `CLAUDE.md` → `guidingpath.md` current state + decision-log tail
2. `2026-09-01-web-frontend-m3.5-spec.md` §1 (the rulings this one inherits) and §6.1
3. `2026-09-01-web-frontend-m4-spec.md` §1 and §3.1 (the drawer and the scrubber)
4. **This file** — the authority for M5
5. `deploy/RUNBOOK.md` for W9
