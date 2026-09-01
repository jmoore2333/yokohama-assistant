# Web frontend — M3.5: "the screen". Prep and handover.

**Written 2026-08-31, on branch `web-frontend`, at the Wave-0 head (`513484d`).**

**THIS IS NOT AN APPROVED SPEC.** The owner has ruled the ORDER of the next two
milestones and nothing else. Everything below the ruling is a proposal: costed,
sequenced, and measured where measurement was possible, but not agreed. §9 is
the list of forks only the owner can settle, and the work list in §3 should be
read as "here is what it would take", never as "here is what we are doing".

Reading order: `2026-08-31-web-frontend-m2.md` → `-m2.5.md` (its §5, what the
harness does not cover) → `-m3-spec.md` → `-m3.md` (its §9) → `-wave0.md` (the
correctness floor that just closed) → this file.

**Every load-bearing claim below carries `file:line` and was re-checked against
the source tree at this head.** That is not a stylistic preference. Twelve of
the nineteen HIGH findings that opened Wave 0 were sentences this lane wrote
about itself, for the third consecutive milestone, and the recorded lesson is
that a document which cites another document is a document nobody can check. Two
consequences: the research this file draws on lives in **untracked** files under
`docs/temp/webresearch/`, so no fact is stated here on their authority — each is
restated with the citation a repo reader can open; and §2 and §3 record several
places where I measured a research figure and got a different answer. Those are
marked, not laundered.

---

## 1. What M3.5 is, and the ruling that ordered it

M3.5 is an INSERTED milestone between M3 and M4, on this lane's own M2.5
precedent — a milestone that exists because the next one is worse without it.

**Owner ruling (2026-08-31): layout before the scrubber.** The reasoning is
narrow and is the whole justification: where a replay scrubber lives is the one
M4 decision the code cannot answer without knowing the layout, and today's only
answer is a modal at `styles.css:364` — `.surface { position: fixed; inset: 3rem
2rem; z-index: 20 }` — which at 1440×900 measures **1376×804 px and covers 980
of the board's 992 px** (measured; §2.4). A scrubber cannot live in a panel that
hides the board it is scrubbing, and a scrubber cannot live on the page either,
because the page's own answer buttons already sit **1,086 px below the fold** at
4P (measured; §2.2).

The adversarial review's dependency analysis reached the same order
independently. That is corroboration, not authority: the ruling is the owner's.

**What M3.5 is not.** It is not the replay scrubber (M4), not multi-seat, not
identity or authorization, and not the four parity items that are genuinely off
the wire (§4.3). It has no gate and asks for no roster place; the fifteen
release-profile gates stay untouched and nothing proposed here is `#[ignore]`d.

---

## 2. The measurements that scope it

**Method, stated so the figures can be re-taken.** All of §2 was measured at
this head in real Chromium (Playwright 1.62.1, `deviceScaleFactor: 1`), against
the release binary `target/release/yokohama-web` serving the committed
`web/dist`, on **bot-free games created through `POST /api/games` with
`bot_seats: []`** at turn 1 — every seat human, so the hand panel renders its
cards rather than a mask. Where a figure differs from one already recorded in a
tracked document, both are given and the delta is named.

### 2.1 The horizontal reading — re-run, and it is green

`npx playwright test the_screen_itself.spec.ts -g "nothing pushes the page
sideways"` at this head, 3 passed, printing:

| | 390 px | 1024 px | 1201 px | 1440 px |
|---|---|---|---|---|
| **2P** page overflow | 0 | 0 | 0 | 0 |
| **2P** board scrolls inside its column | 565 | 0 | **162** | 0 |
| **3P** page overflow | 0 | 0 | 0 | 0 |
| **3P** board scrolls inside its column | 796 | 162 | 393 | **154** |
| **4P** page overflow | 0 | 0 | 0 | 0 |
| **4P** board scrolls inside its column | 1026 | 392 | 623 | **384** |

Wave 0's one-line fix holds: `styles.css:131` `.board-col { width: 100% }` inside
the `@media (max-width: 1200px)` block, and page overflow is 0 at all twelve
cells. **What is NOT fixed is the board fitting its own column at 1440**, which
the gate records and deliberately does not assert: 384 px of a 4P board and
154 px of a 3P board are off-screen behind `styles.css:136`'s `overflow-x: auto`,
and `.board` sets no `scrollbar-color`, no mask and no shadow — `grep -n
scrollbar web/src/styles.css` returns nothing — so on macOS overlay scrollbars
the clipping has **no visible affordance at all**. A `◈` Movement destination in
the last column is invisible and undiscoverable.

The 1201 column is the dead band, and it is worse than "3P and 4P": at 1201 the
row layout is still in force and the board column is 1201 − 40 − 24 − 384 =
753 px, so even a **2P board is clipped by 162 px**. That is where a laptop
lands.

The arithmetic behind all of it, re-verified in the stylesheet: `.stage { gap:
1.5rem; padding: 1.25rem }` (`:112`) and `.rail { flex: 0 0 24rem }` (`:114`)
leave `1440 − 40 − 24 − 384 = 992 px` of board column; `--tile-w: 14rem` (`:49`)
= 224 px with `.board-row { gap: 0.4rem }` = 6.4 px gives widest-row widths of
915.2 (2P, 4 tiles) / 1145.6 (3P, 5) / 1376 (4P, 6). 1376 − 992 = **384**, which
is what the browser printed.

### 2.2 The vertical reading — and the term that actually sets it

Measured at 1440×900, turn 1, one card in hand:

| | 2P | 3P | 4P |
|---|---|---|---|
| `document.scrollHeight` | **1847** | **2015** | **2160** |
| in 900 px screens | 2.05 | 2.24 | **2.40** |
| `header.banner` | 93 | 93 | 93 |
| `.end-strip` | 28 | 28 | 28 |
| `.stage` | 1553 | 1721 | 1866 |
| `.board` (h) | 728 | 728 | 728 |
| `.rail` (h) | **1513** | **1681** | **1826** |
| `.decision` top (page coords) | 1673 | 1842 | **1986** |
| `.decision` height | 173 | 173 | 173 |
| px of the decision zone below the fold | 773 | 942 | **1086** |

**The single most useful thing in this table is that the stage height is the
RAIL's, not the board's.** `.stage` is a row flex container with `align-items:
flex-start` (`styles.css:112`), so its height is `max(board-col, rail)`, and the
rail is **2.1× to 2.5× the board's height at every player count**. The board
contributes nothing to the page height. Three consequences run through §3 and
they are the reason this section exists:

1. **Deleting the tile's `aspect-ratio` recovers no page height.** Measured with
   the rule suppressed: the tile goes 224×168 → 224×**94** and `.board` goes 728
   → **433**, and `document.scrollHeight` stays at **1847 / 2015 / 2160**,
   unchanged to the pixel. *This contradicts the review's own H.3 sizing —
   "P2 + P3 … takes the stage from ~1610 px to ~1150 px". Measured, P2+P3 gives
   1423 px at 2P and P2 contributes zero of the difference.*
2. **The art fix is therefore free in vertical terms** (§2.3), which inverts the
   review's sequencing argument.
3. **Nothing on the realistic list gets the game onto one screen.** The rail's
   four panels measure, at 2P: Players 375, Tracks 205, Achievements 499, Hand
   407. Shrinking the achievement art from `5.2rem` (`styles.css:271`) to 3.4rem
   and unstacking `.panel .card-strip` (`:274`) measures **−130 px at 2P, −83 at
   3P, −51 at 4P** — real, and not the ~260 px the review estimated, and nowhere
   near enough. **So pinning the decision zone is not a convenience item. It is
   the item that makes the page height stop being the problem**, and everything
   else in §3 is worth doing on its own merits rather than as a height budget.

Recorded-figure reconciliation, because the numbers in `-wave0.md` §7 and in the
M3.5 brief differ slightly from mine: page **1950 px / 2.17 screens** and
`.decision` at **y = 1703** and rail **1534** were recorded; I measure **1847 /
2.05 / 1673 / 1513** on a bot-free 2P game at turn 1. The rail figure is within
21 px and the decision top within 30; the page height differs by 103. I could
not reproduce 1950 and I do not know what the earlier capture's state was (a bot
seat, a message in the decision zone and a longer end-trigger strip would each
add tens of pixels). **Both are stated; neither is retracted.** Everything §3
concludes holds under either.

### 2.3 What each layout candidate actually costs, measured

Every row below was produced by injecting the candidate CSS into the live page
and re-reading the rects. It is a price, not a design.

| candidate | tile | `.board` h | page h (2P/3P/4P) | board scroll at 1440 |
|---|---|---|---|---|
| **base** | 224×168 | 728 | 1847 / 2015 / 2160 | 0 / 154 / 384 |
| delete `.tile.has-art{aspect-ratio}` | 224×**94** | **433** | 1847 / 2015 / 2160 | unchanged |
| shrink rail art + unstack hand | — | 728 | **1717 / 1932 / 2126** | unchanged |
| both | 224×94 | 433 | 1717 / 1932 / 2126 | unchanged |
| **art unveiled, caption below** | 224×**260** | **1098** | **1847 / 2015 / 2160** | unchanged |
| art unveiled + rail shrunk | 224×260 | 1098 | 1717 / 1932 / 2126 | unchanged |
| `--tile-w: 10rem` | 160×120 | 537 | unchanged | **0 / 0 / 0** |
| `--tile-w: 12rem` | 192×144 | 632 | unchanged | 0 / **0** / 192 |
| art unveiled + `10rem`, at 4P | 160×**212** | 906 | 2180 | **0** |
| art unveiled + `12rem`, at 3P | 192×**236** | 1002 | 1995 | **0** |
| art unveiled + `16rem`, at 2P | 256×284 | 1194 | 1847 | **51 — 2P now clips** |

Four results worth stating in words:

- **`--tile-w` scaling works, exactly as the arithmetic predicted.** 4P fits
  992 px at 10rem (6×160 + 5×6.4 = 992 — zero headroom, and the browser prints
  0 px of scroll); 3P fits at 12rem (5×192 + 4×6.4 = 985.6, 6.4 px spare); 2P
  already fits at 14rem. **4P does not fit at 12rem** (192 px still clipped), so
  the ladder has to be three values, not two.
- **The art fix costs no page height at any player count**, because at the
  matching tile width the board tops out at 906/1002/1098 px against a rail of
  1826/1681/1513. It is free vertically today and stays free, because the tile
  height is a constant while the rail only grows with the game.
- **Do not bump `--tile-w` to 16rem.** The review's F.1 step 4 proposes it so the
  unveiled ladder is larger; measured, it puts **51 px of a 2P board off-screen
  at 1440**, which is a regression on the one board that fits today.
- **Pinning the decision zone works with one rule and no JS.** With
  `position: sticky; bottom: 0; z-index: 10; max-height: 45vh; overflow-y: auto`
  the 4P decision zone's viewport top goes from **1986 (off-screen,
  `inView: false`) to 727, pinned identically at scrollY 0, 400 and at the page
  bottom**. 45vh = 405 px against a measured height of 173–209, so it does not
  scroll internally in ordinary play.

### 2.4 The modal, the nav, and the focus model

`.surface { position: fixed; inset: 3rem 2rem }` (`styles.css:364`) measures
**1376×804 at 1440×900 and covers 980 of the board's 992 px** — 98.8 %. Injecting
`inset: 3rem 2rem 3rem auto; width: min(44rem, 55vw)` makes it a 704 px drawer at
`left: 704`, covering **308 px** of the board, and `.area-grid` still renders
**two columns** in the 702 px body. **Zero component changes** — measured, not
assumed.

`.surface`'s top edge is at `3rem` = 48 px. Measured banner geometry at 2P:

| viewport | banner h | nav rows | nav bottom edge |
|---|---|---|---|
| 1440 | 93 | 1 | **43** |
| 1280 | 101 | 1 | **88** |
| 1201 | 101 | 1 | **88** |
| 1024 | 174 | **2** | **125** |
| 900 | 174 | 2 | 125 |

**So the nav is underneath an open panel at 1280 and below, not only at 1024**,
and 1440 clears it by 5 px. `.views` is `flex-wrap: wrap` (`:98`) and the
eleven surface buttons (`Surfaces.tsx:53-70`) are what wrap. You cannot switch
from Areas to Site cards without pressing Escape first, at every width a laptop
uses.

Focus, measured behaviourally rather than read off the CSS:

- `Surfaces.tsx:145` asserts `aria-modal="true"`. **Six Tab presses from the open
  panel land on a nav `button.ghost` labelled "Port offer", with
  `closest('.surface') === null`.** There is no focus trap and nothing `inert`
  — `grep -rn inert web/src` returns one hit and it is a test name.
- **The focus yank is real and I isolated it.** `App.tsx:152` passes
  `onClose={() => setSurface(null)}` — a fresh identity on every `App` render —
  into an effect whose dependency list is `[onClose]` and whose body calls
  `panel.current?.focus()` (`Surfaces.tsx:132,138`). Procedure: open a surface,
  Tab to the Close button, `document.activeElement.blur()` (confirmed `BODY`),
  then fire a programmatic `.click()` on an option button underneath — which
  changes `App` state and moves no focus of its own. Result: `activeElement`
  becomes **`div.surface`**. Any state change in `App` recaptures focus.
- Escape does not restore focus. After the Tab walk above, Escape left
  `activeElement` on the `button.primary` underneath. (The review predicted
  `<body>`; either way it is not the control that opened the panel.)
- `grep -rn 'aria-live|role="alert"|role="status"' web/src` returns **nothing**,
  and `grep -rn '<main|role="main"' web/src web/index.html` returns nothing. The
  end-trigger strip, `warnings`, the error strip and every applied-move delta and
  engine refusal are announced to no assistive technology, and the page has no
  main landmark.

### 2.5 The art, re-measured at this head

Method, reproducing the review's: replace the first `.tile-art`'s `src` with a
709×531 solid-white SVG and then a solid-black one, screenshot the tile each
time, and difference the two row by row. `(white − black)/255` at a row is the
fraction of the art's dynamic range that reaches the viewer.

| | this head | review's figure |
|---|---|---|
| mean transmittance | **0.305** → **69.5 % obscured** | 0.304 |
| max at any row | **0.715** (row 58) | 0.715 |
| fraction of tile height below 0.25 | **52.7 %** | 52.7 % |
| fraction above 0.70 | 13.0 % | 13.2 % |
| printed power-ladder band (y 0.61–0.97) | **0.031–0.262, mean 0.100** | 0.036–0.24 |
| printed 5-space band (y 0.648–0.712) | **0.123–0.147** | 0.10–0.24 |

Row profile at 224×169: `0:0.00 · 16:0.17 · 32:0.31 · 48:0.58 · 64:0.71 ·
80:0.63 · 96:0.38 · 112:0.13 · 128:0.10 · 144:0.05 · 160:0.03`.

**The owner's complaint is confirmed and it is the AREA TILES, not the card
faces** — `.card-face` carries no overlay of any kind (`styles.css:260-284`).
Two rules stack on the tile: `--art-veil: 0.82` (`:60`) applied as a whole-image
`opacity` at `:159`, and `.tile-body`'s six-stop `linear-gradient` at `:168-176`
whose lightest stop is `rgb(9 11 14 / 0.12)`. The measured ceiling of 0.715 is
the veil's own arithmetic — `0.82 × (1 − 0.12) = 0.7216` — which is the check
that says the measurement is measuring the two rules and not something else.

**The worst band is exactly the part of the art that carries rules
information.** The printed power ladder 1–5 with its per-rung yield icons runs
across the bottom of every tile at a measured mean transmittance of **0.100**,
and there is **no power ladder anywhere else on the tile**: `Board.tsx:144-185`
renders id, name, reachability mark, pawns, the 5-Power token and the site strip,
and nothing else. The one surface that carries the ladder is the Areas panel
(`Surfaces.tsx:324-337`), which is the modal that covers 98.8 % of the board.

---

## 3. The work, as a ranked and costed list

Ranked by measured value per unit of diff, not by the review's original order.
Sizes are S / M in this lane's usual sense. **Every "measured" line in this
section is one of §2's readings; every "guess" line says so.**

### W1 — Pin the decision zone · **S** · no dependency

**What changes.** `styles.css:326` gains `position: sticky; bottom: 0; z-index:
10; max-height: 45vh; overflow-y: auto`. The rule needs a second class to scope
it, because `App.tsx` renders `className="decision"` **three** times — the
warnings strip at `:117`, the `<Decision>` component at `:156` (whose own root
is `.decision`, `Decision.tsx:102`) and the error strip at `:164` — and
`web/e2e/harness.ts` already works around exactly this (`.decision` is taken
`.first()`). Give `Decision.tsx:102` a `decision play` and scope the sticky rule
to `.decision.play`.

**Files.** `web/src/styles.css`, `web/src/Decision.tsx`; a matching selector fix
in `web/e2e/harness.ts` if the harness's `.first()` becomes unnecessary.

**Measured benefit.** 4P: the answer buttons go from **1,086 px below the fold**
to a fixed 727 px from the top of the viewport, at every scroll offset including
the page bottom. This is the difference between "scroll to answer" and "the
answer is always in view", and it is the only item on this list that does not
depend on the page getting shorter.

**Cost / risk.** The warnings and error strips are also `.decision`; if they stay
unscoped they stack three sticky bars. `max-height: 45vh` is a guess — 405 px
against a measured 173–209 px is generous, but a long `pick` list is untested and
the internal `overflow-y` is what catches it.

### W2 — Make the surface a right-hand drawer · **S** · wants W1 landed

**What changes.** One rule: `.surface { inset: 3rem 2rem 3rem auto; width:
min(44rem, 55vw) }` at `styles.css:364`. Pair it with `inert` on `.stage` while
a surface is open, which is what makes W3's focus trap honest rather than a
second hand-rolled mechanism.

**Files.** `web/src/styles.css`, `web/src/App.tsx` (the `inert` toggle).

**Measured benefit.** Board coverage **980 px → 308 px** of a 992 px board, and
`.area-grid` still lays out **two columns** in the 702 px body — measured, so
"zero component changes" is a reading and not a hope. The board stays visible
while you read the power ladder or the port offer, which is the whole complaint
in §2.4.

**Cost / risk.** `table.outcome` is `width: 100%` and `.index-cards` is
`auto-fill`, so those should also survive; **I measured only `.area-grid`.** The
other panels are a guess until someone opens each one in a 702 px drawer.

### W3 — The accessibility floor · **S–M** · wants W2

**What changes.** Four separate defects, all verified in §2.4:
(a) a real focus trap, or `inert` on `.stage` (W2), so `aria-modal="true"`
(`Surfaces.tsx:145`) stops asserting a containment the DOM does not have;
(b) restore focus to the opener on close;
(c) stop the yank — `useCallback` the `onClose` at `App.tsx:152`, or split the
Escape listener from the one-shot `focus()` so the effect that focuses runs on
mount only;
(d) `role="status"` on the four message channels (`App.tsx:117` warnings,
`App.tsx:164` errors, `Decision.tsx:291-297` messages, the `.end-strip` at
`App.tsx:112`) and a `<main>` landmark.

**Files.** `web/src/Surfaces.tsx`, `web/src/App.tsx`, `web/src/Decision.tsx`,
`web/src/styles.css`.

**Measured benefit.** (a) and (c) are measured defects, not predicted ones. (d)
is a category the page has zero of.

**Cost / risk.** (c) has a trap of its own: `useCallback`-ing `onClose` fixes the
yank but leaves the Escape listener re-registering, which is harmless; splitting
the effect is the cleaner fix and is the one that survives a future dep change.

### W4 — Scale `--tile-w` by tile count, and make the clipping visible · **S** · no dependency, but must land before W5

**What changes.** `Board.tsx` sets `--tile-w` as an inline custom property from
`board.player_count` — the same inline-`style` custom-property idiom
`Board.tsx:49` already reads (`calc((var(--tile-w) + 0.4rem) * ${r} / 2)`) — at **10rem
(4P) / 12rem (3P) / 14rem (2P)**. Plus `.board { scrollbar-width: thin;
scrollbar-color: var(--line-bright) transparent }` so macOS overlay scrollbars
stop hiding the clipping while it still exists at narrow widths.

**Files.** `web/src/Board.tsx`, `web/src/styles.css`.

**Measured benefit.** Board scroll at 1440 goes **384 → 0 (4P)** and **154 → 0
(3P)**; 2P is unchanged at 0. Every tile of every board is on screen at the width
the product is designed for, for the first time.

**Cost / risk.** 4P at 10rem fits **exactly** — 6×160 + 5×6.4 = 992 against a
992 px column, zero headroom. A one-pixel change to `.stage`'s padding, the
rail's width or the row gap re-breaks it, so the ladder wants a test that reads
the board's `scrollWidth - clientWidth` at 1440 and asserts 0, not a comment.
The dead band at 1201 is NOT fixed by this: 4P at 10rem in a 753 px column still
clips 239 px, and closing that means moving the breakpoint, which is a separate
decision. **And the legibility question is open** — at 10rem the tile's art is
160×120 and I did not measure whether an unveiled power ladder is readable at
that size (§9, Q2).

### W5 — The art: stop overlaying, caption below · **M** · depends on W4

**What changes**, restated in the engine's terms because this is the item the
owner asked about:

1. Delete `--art-veil` (`styles.css:56-60`) and the `opacity` at `:159`.
2. Delete `.tile.has-art { aspect-ratio: var(--tile-aspect) }` (`:156`) and give
   the aspect to the image instead: `.tile-art { position: static; display:
   block; width: 100%; aspect-ratio: var(--tile-aspect); object-fit: cover }`.
3. Delete the `linear-gradient` and `height: 100%` from `.tile-body`
   (`:163-177`).
4. **Do NOT bump `--tile-w` to 16rem** — measured, it puts 51 px of a 2P board
   off-screen at 1440 (§2.3). The legibility gain comes from removing the veil,
   not from growing the tile; W4 is what sets the width.

This is the pattern `Cards.tsx:1-10` already chose and defends in its own words
— *"every card here is the picture with the values BESIDE it"* — applied to the
one surface that did not get it.

**Files.** `web/src/styles.css` only, plus whatever `web/src/Board.tsx` needs if
the caption wants a wrapper.

**Measured benefit.** Transmittance over the printed power ladder goes from a
mean of **0.100 to 1.000**. **The ladder becomes readable on the board**, which
today only the Areas modal shows — so this **SUBSUMES the review's separate
"put the power ladder on the tile" proposal (P4)** and the ~30 lines of client
code and static `/areas` fetch it would have cost. The height cost is **zero
page pixels at every player count** (§2.3): the board grows 728 → 906 / 1002 /
1098 at 4P/3P/2P against a rail of 1826 / 1681 / 1513.

**Cost / risk.** The tile stops being a fixed box, so `.tile { overflow: hidden }`
(`:139`) and every `.tile-*` rule want re-checking; the `aspect-ratio` currently
also serves to reserve the box before the image arrives, and moving it to the
`<img>` preserves that only if the image is not `display: none` while loading.
The e2e "every picture the client draws actually loads" test
(`the_screen_itself.spec.ts:17`) is the one that would catch a broken swap.

**The fallback, if the overlay must stay.** Drop the `opacity` and anchor two
scrims to the two text bands instead of one full-bleed gradient. That leaves the
middle of the tile at 100 % instead of 13 % at 0.72 — but it still buries the
ladder, which is the whole complaint, which is why it is the fallback. *I did not
re-measure the review's text-band boundaries (head 4.4–15.3 %, pawns/site-strip
60.7–95.6 %); anyone taking this branch should re-measure them.*

### W6 — Parity closure, the half already on the wire · **M** · independent

Detailed in §4. A dozen small client edits over payloads the server already
sends, plus one live bug (the `Tracks` companion pointer).

### W7 — Collapse the nav · **S** · wants W2

**What changes.** Keep Areas / Log / Suggest / Why inline; move the four card
indexes and Final score behind one `Cards ▾` disclosure, using the `<details>`
pattern already at `Board.tsx:220-221`
(`<details className="legend"><summary>marks</summary>`, styled at
`styles.css:223-226`).

**Measured benefit.** Recovers a banner row (93 → ~60 px at 1440; 174 → ~101 at
1024) and un-buries the nav from under an open panel at **1280, 1201, 1024 and
900** — measured, and a wider band than the review reported.

**Cost / risk.** `Surfaces.tsx:53-70` is a flat registry with no grouping field;
adding one is a second statement of which surfaces are indexes beside the
existing `INDEX_KINDS` map (`:74-79`). Derive the group from that map rather than
writing a third list.

### W8 — Finish the game-over block · **S** · independent

**What changes.** `Decision.tsx:236-246` prints `seat 0` / `46 VP` and nothing
else: no seat colour, no winner mark, no breakdown — verified, and `state.scores`
on the wire is `{ seat, total }[]` (`types.ts:335`) with no `won` field. The star
that marks a winner exists only as `table.outcome tr.won td:first-child::after {
content: " ★" }` (`styles.css:590`), which is the `/result` surface's table and is
**a CSS pseudo-element, so it is in no `textContent` and in no assertion**. The
inline block has no winner mark at all.

`colors` is already a prop of `Decision` (`:87`) and `Placing.won` is already a
wire field on `/result` (`types.ts:702-715`). **The browser's parity claim
currently breaks exactly at the moment a game ends.**

### W9 — Shrink the rail's card art · **S** · independent

`.panel .card-face.achievement img` 5.2rem → 3.4rem (`styles.css:271`); drop
`flex-direction: column` from `.panel .card-strip` (`:274`) so two hand cards sit
per row in a 354.8 px rail.

**Measured benefit: −130 px (2P) / −83 (3P) / −51 (4P).** Real, and smaller than
the review's ~260 px estimate. With W1 landed this is a tidiness item, not a
budget item; it is on the list because the rail is the term that sets the page
height (§2.2) and this is the only cheap lever on it.

---

## 4. Parity closure

The review's §D table, reduced to what is actionable. **I re-verified nine rows
against the client at this head rather than copying them**; each row below says
how.

### 4.1 Already on the wire — client-only work

| Datum | CLI authority | Web today (verified at this head) | Cost |
|---|---|---|---|
| **Tolled canal-crossing pairs** | `view_render.rs:1172-1185` | `grep -rn canal_crossings web/src/*.tsx web/src/*.ts` → **one hit, `types.ts:99`**. Declared, rendered nowhere. `areas_4p.json` carries **6 areas × 3 crossings each** (A07, A08, A11, A13, A15, A16); `areas.json` (2P) carries **zero** — at 2P the tolled set is empty, so this row buys nothing below 3P | ~10 lines |
| Areas: the 5-Power face / "already taken" | `view_render.rs:1078-1094` | absent; `areas.json` carries `tile.token` on **10 of 10** cards | ~6 lines |
| Areas: site spaces in words, builder, trading-house VP + owner | `view_render.rs:1096-1140` | absent; `areas.json` carries `tile.site` on **10 of 10** cards, with `slots[].built_by` and `trading_house_owner` populated (live, unlike the index — §5) | ~20 lines |
| Areas: board-row grouping | `AreasPanel.rows` | flattened at **`Surfaces.tsx:312`** `panel.rows.flat()`; the payload's `rows` are `[4,3,2,1]` at 2P and `[6,5,4,3]` at 4P | 3 lines |
| Per-seat `hand_hidden` marker | `panel_render` `(hidden)` | `grep -rn hand_hidden web/src/*.tsx web/src/*.ts` → **one hit, `types.ts:157`**. Same shape as the crossings | 1 line |
| Tracks: per-space VP + rider as **visible text** | `t12-tracks-view` | `title=` tooltip only — **`Dashboard.tsx:286`** (church) and **`:303`** (customs). Invisible on touch, to a screen reader, and to any assertion | a `tracks` surface (§4.4) |
| Site index: TL/TR/ML/MR labels and per-space grouping | `t12-site-index` | **`Index.tsx:189-191`** maps all four `spaces` into one flat `.chips` row. **6 of 24 cards carry a multi-reward space** (7 such spaces of 96), verified on `index_sites.json`, so those render as peer chips with nothing saying which space they belong to | ~8 lines |
| Near-miss **area name** | `decision_render.rs:352` resolves `area_name(board, row.area)` | **`Decision.tsx:252`** prints `A{nn}` only; `state.frame.board` is in scope | 3 lines |
| Turn player's **colour** | `TURN 1 · BLUE's turn` | **`Decision.tsx:104`** prints `seat {d.seat}`; `colors` is already a prop at `:87`. (The BANNER does colour it, `App.tsx:99-103` — this row is the decision zone only) | 1 line |
| Game-over standing: colour, winner mark, breakdown | `repl.rs:3334-3355` | W8 above | 5 lines |

### 4.2 A live bug in the same region

`Surfaces.tsx:343-344` renders *"detail lives on the {companion} surface"*.
Verified on the committed fixtures: **`areas.json` carries two `"Tracks"`
companions and `areas_4p.json` carries two.** There is no tracks surface —
`Surfaces.tsx:53-70` lists eleven and none is `tracks`. **Opening Areas on any
board points the player at a surface that does not exist.** This is a one-line
symptom of §4.4 and should be fixed either way.

### 4.3 Genuinely not on the wire — server work, and NOT proposed for M3.5

The area-action prose gloss (`view_render.rs:1202`, a CLI-local fn), the nine
Chinatown exchange rates (`ChinatownOffer::rates()`, `view_render.rs:1052`), the
church 6/3 and customs 8/4 majority awards, and other seats' order-card
identities. The first three are a server widening; the fourth is a masking
decision that belongs with multi-seat, not before it.

### 4.4 The two open Tier-B rows the correction row names

`guidingpath.md:204` (the forward-corrections row) states, verified rather than
recalled, that **two Tier-B rows are open at this head**:

- **The privacy toggle.** `privacy` sits in the grammar allowlist
  (`grammar.rs:61`), so the server would take the line — but `grep -rn privacy
  web/src web/e2e` returns **nothing**, so no client code can produce it. There
  is no UI at all.
- **The per-area Building-sites view** — every area's site card, its builders and
  its canal tolls. `Surfaces.tsx`'s registry has no entry; its `sites` key is the
  Tier-C card **index** of the 24 printed cards (`api.index(g,"sites")`), a
  different CLI surface.

Together with the missing `tracks` surface these are the last three items between
this lane and *"the audited CLI parity gap is closed"* being a true sentence.
Whether they belong in M3.5 is §9 Q3.

---

## 5. Art — the owner's three items in the engine's terms

### 5.1 The scrim

Covered by **W5**. The one thing to carry from §2.5 into any discussion of it:
the fix is to **stop overlaying and caption below the art**, which makes the
printed power ladder legible (mean transmittance 0.100 → 1.000) and therefore
**subsumes the separate "power ladder on the tile" proposal entirely**. That is
the strongest argument for the primary fix over the two-scrim fallback, and it
is an argument about rules information rather than about looks.

### 5.2 Asset naming — `sites/S00..S23` and why it is worse than opaque

The manifest is the only join and **no client code constructs a filename**:
`web/src/art.ts:16-17` is the whole of it (`artUrl = entry ? BASE + entry.file :
undefined`), and the only `/art/` literal outside the wire fixtures is
`art.ts:14`. Verified by reading the file. So the client cost of a rename is
**zero**.

The four schemes: `orders/O01..O36` is the printed corner index (self-
explanatory); `techs/<slug>` is the printed title (self-explanatory, and keyed on
the title deliberately — `mkcardart.py:26-33` records that the `codes` slug is
hashed nowhere and that a `TechRegistry::code()` accessor was written and
reverted; do not re-add it); achievements / areas / five-power / mercenaries /
backs / tokens are serde names. **`sites/S00..S23` is the one genuinely opaque
scheme.**

**And it is worse than opaque.** Verified: `art.rs:361` derives the filename as
`format!("S{}", two_digit(k))` from the manifest key, and the key is the 0-based
index that `screen_model::printed_site_index` returns
(`screen_model.rs:2122-2135`) — which recovers it by **content match** against
`Rules::building_sites`, i.e. `09-building-sites.yaml`'s file order. There is no
printed index on a building-site card.

The hazard, stated precisely, because the review's version is imprecise:
`mkcardart.py:371-374` crops each card from its own `position` field and writes
it to `sites/S{i:02d}.webp` where `i` is the enumeration index. So a YAML reorder
**followed by a regeneration** is safe and shows up as changed blobs. **The
hazard is a reorder that regenerates `data.rs` and not the art** — then
`printed_site_index` returns new indices, the files hold the old pictures, and
**every test in `art.rs` still passes**, because those tests check the key set
(`:175`, `:233`), file existence (`:276`), name derivation (`:328`) and sheet
aspect (`:398`) — all of which are unchanged. `art.rs`'s own doc says it: the
tests *"prove the join is TOTAL, not TRUE"*.

**Proposal, and its real value.** Append the printed content to the name, which
both sides can derive: `sites/S13_vp1-vp4-blank-vp3_T9.webp`. `mkcardart.py` has
the YAML dict in hand at `:371`; `art.rs`'s tests already build
`Rules::standard()`. **The point is not the git diff** — a regenerated reorder
already shows as 24 changed blobs, so the review's stated rationale ("24 renames
instead of nothing") is weaker than the mechanism supports. **The point is that
`art.rs:328` becomes able to FAIL**: a name that carries the content can be
checked against `rules.building_sites[k]`, which converts one line of "TOTAL, not
TRUE" into TRUE for the 24 cards where the join is recovered rather than printed.

**True cost: three files and no client change.** `docs/tools/mkcardart.py`'s
emit f-string (`:374`), `art.rs:361`'s name closure, and a regeneration; the
`every_key_names_the_file_it_derives_and_no_file_is_named_twice` test at
`art.rs:328` must move in the same commit, because it is the only test that
catches a permutation. **Low priority; do it in whatever wave next touches
`mkcardart.py`.**

**A sibling worth one line.** `techs/station.webp` and `tokens/station.webp` are
two different pictures with the same basename. `art.rs:371-388` counts full
relative paths so it correctly does not fire — but a human moving files will.
The other seven `tokens/` entries are already prefixed; make it
`tokens/token_station.webp`.

**Dead assets, verified by count.** The art tree holds **131 `.webp` files and
the manifest holds 131 entries** — an exact bijection, independently computed
here. **14 of them are referenced by nothing**: `backs/` (6) and `tokens/` (8),
172 KiB, and their two exported helpers `backArt` (`art.ts:46`) and `tokenArt`
(`art.ts:48`) have **zero callers** — `grep -rn "backArt|tokenArt" web/src
web/e2e` returns only the two definitions. The eight tokens are exactly what the
good chips (`Cards.tsx:37-41`) and the yen/station chips want. **Wire them up or
delete both files and exports; do not leave them.**

### 5.3 The 5-Power / player-mat request — the model difference, stated plainly

The request was *"the five power cards should be cropped and overlaid on the
player mats so that we can see where the shops or trading houses are built and
what rewards are there."* Three things are crossed, and each has a citation:

1. **The six `five-power/*.webp` are not cards.** They are 128×128 round token
   faces — a numeral and a goods icon (`Copper_2`, `ImportedGoods_1`, `Money_3`,
   `Seafood_3`, `Silk_2`, `Tea_3`; `art.ts:56-59` builds the key). A 5-Power
   token says what you gain when you take it at 5 Power. It says nothing about
   shophouses.
2. **What says "where the shops or trading houses are built and what rewards are
   there" is the BUILDING SITE card** — `sites/S00..S23.webp`, **one per area,
   not one per player**: `types/board.rs:126-135` is `BuildingSite { spaces:
   [SpaceReward; 4] /* TL,TR,ML,MR */, trading_house_reward, shophouses_built,
   trading_house_owner }`, keyed `BTreeMap<AreaId, BuildingSite>` at `:201`.
3. **The player mat is the wrong home for either, and the engine deliberately
   does not remember the faces**: `types/player.rs:225` is
   `pub five_power_tokens: u8, // Just count, effects already resolved`
   (verified verbatim at this head). `Dashboard.tsx:136-138` already renders that
   count, which is the only honest per-player display from state alone.

**Two things are already built and should not be re-proposed.** `Board.tsx:78-108`
(`SiteSlots`) already renders the four spaces with a `⌂` built-mark in the
builder's seat colour on every board tile, and `Board.tsx:170-176` already
composites the 5-Power token's own art beside its words on the tile. So the
missing piece is narrower than the review reads: it is the **positional overlay
on the site card ART**, not the occupancy data and not the seat colouring.

**Measured overlay rects.** The review measured, on S00/S13/S23 (360×543,
uniform across all three), gutters at x = 0.489–0.497 w and y = 0.346–0.355 h and
0.646–0.650 h, giving:

| slot | `spaces[]` index | normalized rect on `sites/S{nn}.webp` |
|---|---|---|
| TL | 0 | x 0.03–0.49, y 0.06–0.35 |
| TR | 1 | x 0.51–0.97, y 0.06–0.35 |
| ML | 2 | x 0.03–0.49, y 0.36–0.645 |
| MR | 3 | x 0.51–0.97, y 0.36–0.645 |
| trading house | — | x 0.03–0.97, y 0.66–0.94 |

The TL/TR/ML/MR ↔ index-0..3 mapping matches `board.rs:127-128`. **I did NOT
re-measure these five rects** — they are carried from the review and should be
re-measured against a crop before anyone positions anything on them.

**The trap an implementer will fall into, verified.** `types.ts:632-640` exposes
`BuildingSiteCard.shophouses_built` and `.trading_house_owner` with no caveat
(only `trading_house_reward` carries one), but `build_site_index` clones the
**pristine registry** card. Verified on the committed payload
`web/src/wire/index_sites.json`: **24 faces, `shophouses_built` is
`[null,null,null,null]` on all 24, `trading_house_owner` is `null` on all 24.**
Live occupancy lives on `SiteModel` in the board frame — `areas.json`'s
`tile.site` carries populated `slots[].built_by`. Whoever builds the overlay will
reach for the index fields first and ship a wrong answer. **Fix it in the same
commit: document the emptiness on `types.ts:634-635`, or split the index face into a
`PrintedSiteCard` carrying only `spaces` + `trading_house_reward`.**

**And the correction that changes the owner's menu.** The review concluded that a
per-seat 5-Power token strip *"is not implementable without a core change …
STOP territory"*. **That conclusion does not follow, and both of the critic's
citations check out at this head.** `player.rs:225` destroys the face in STATE;
`apply_move.rs:923-928` preserves it in HISTORY:

```rust
let effect = state
    .board_layout
    .five_power_tokens
    .get_mut(&area)
    .and_then(|slot| slot.take())          // apply_move.rs:927
    .ok_or(MoveError::RuleViolation("No 5-Power token at the acting area"))?;
```

`slot.take()` leaves the board slot `None`, so **at the event index where a seat
takes its bonus, the pre-event state carries the face at the acting area and the
acting seat is known.** `record::state_at` (`record.rs:561`) plus a scan is
exactly the machinery `why` already uses. **A per-seat 5-Power token strip is a
replay-derived projection over the `GameRecord` the server already holds — no
`PlayerState` field, no serde change, no `SEMANTICS_REV` implication.**

*(Citation drift worth naming: the critic cited `apply_move.rs:921-925`; at this
head the expression spans **923–928** with `.take()` at **927**. The mechanism is
exact; the line numbers moved.)*

**So it is CHEAP AFTER M4, not impossible** — M4 builds the `state_at`-over-a-
record machinery this needs. **Proposed as M4-adjacent, explicitly NOT as M3.5
work**, and recorded here so the owner's request is answered as asked rather than
only as reinterpreted.

---

## 6. The accessibility floor

Consolidated from §2.4, restated as the checklist W3 would work from. Every item
was verified at this head, by grep or by driving the browser:

| # | Defect | Evidence |
|---|---|---|
| 1 | No focus trap and nothing `inert`, despite `aria-modal="true"` | **Measured**: six Tabs from the open panel land on `button.ghost` "Port offer" with `closest('.surface') === null`. `Surfaces.tsx:131-138` is `panel.current?.focus()` plus an Escape listener and nothing else; `grep -rn inert web/src` → one hit, a test name |
| 2 | Focus not restored on close | **Measured**: after the Tab walk, Escape leaves `activeElement` on the `button.primary` underneath, not on the opener |
| 3 | `onClose` identity churn yanks focus mid-interaction | **Measured and isolated**: blur to `BODY`, then a programmatic click on an option underneath (no focus move of its own) → `activeElement` becomes `div.surface`. Mechanism: `App.tsx:152` inline arrow → `Surfaces.tsx:138` `}, [onClose])` → `:132` `panel.current?.focus()` |
| 4 | Zero `aria-live` / `role="status"` / `role="alert"` anywhere | `grep -rn 'aria-live\|role="alert"\|role="status"' web/src` → **no output**. Four channels are silent: warnings (`App.tsx:117`), errors (`App.tsx:164`), messages (`Decision.tsx:291-297`), end-trigger strip (`App.tsx:112`) |
| 5 | No `<main>` landmark | `grep -rn '<main\|role="main"' web/src web/index.html` → **no output** |
| 6 | The nav is under the open panel at every laptop width | **Measured**: nav bottom edge 88 px at 1280 and 1201, 125 px at 1024 and 900, against `.surface`'s 48 px top (`styles.css:364`). Only 1440 clears it, by 5 px |
| 7 | Board clipping has no visible affordance | `grep -n scrollbar web/src/styles.css` → **no output**; 384 px of a 4P board is off-screen at 1440 behind `styles.css:136`'s `overflow-x: auto` |

---

## 7. The MEDIUM/LOW tail worth folding in — and what is not

### 7.1 Fold in — they are in the files M3.5 is already opening

| Item | Verified how | Why here |
|---|---|---|
| **`autosaved` is hardcoded** | `client.rs:355` is a literal `autosaved: true`; `App.tsx:77` prints "· saved" three lines above `App.tsx:117`'s `APPLIED BUT NOT SAVED` warning | The banner is a W7 file, and the two strings contradict each other on the same screen |
| **Priced rows state the price twice** | `describe.rs:862-868` words a teleport as `"Move President directly to {} — free teleport"` and `placement_fee_note` appends the fee to both placement arms (`:847`, `:859`); `Decision.tsx:153-157` then appends `feeWords(row.fee)` on top. `decision_render.rs:811` carries the CLI's own comment naming this exact failure — *"the fee clause could only ever have said 'free' a second time (`— free teleport · free`)"*. **Verified across all 41 fixtures: the only fee-carrying row in the corpus is a `PlacePresident` with `total: 0`**, so no test can see it | `Decision.tsx` is a W1 and W8 file |
| **The refusal heuristic misses the REPL's own `!`** | `Decision.tsx:294` tests `/illegal\|refus\|cannot\|not legal\|out of range/i`; `grep -c '"! ' crates/yokohama-cli/src/repl.rs` → **20** literals, none of which match — `! finish or cancel this prompt first`, `! pick a number 1-5, or 'cancel'`, `! A05 is not one of the eligible areas`. **Fix: test for a leading `!` first** | Same file; and it is what W3's `role="status"` will be announcing |
| **Confirm is rendered where it is never valid** | `Decision.tsx:264-276` renders Confirm on `pick`; `repl.rs`'s `step_pick` has arms for `cancel`, a digit, `more` and `list`, and a `_` arm at `:2132` answering `! finish or cancel this prompt first`. **There is no `confirm` arm.** Verified by reading the match | Same file, and it is a button that always refuses |
| **The `Tracks` companion pointer** (§4.2) | Verified on both committed area fixtures | One line, and it is a W6 file |
| **`Dashboard`'s silent `return null`** | `Dashboard.tsx:47`. A fifth `screen_model::Panel` variant, or a rename of one of the four, makes a hand, money, goods or track panel vanish with no message and no console output. The right pattern is three files over: `Surfaces.tsx:288-298`'s `yieldWords` ends `return JSON.stringify(y)` at `:298` *"so a new yield shape renders as its own JSON instead of silently as nothing"* | `Dashboard.tsx` is a W9 file |
| **`SpaceChip`'s private vocabulary** | `Index.tsx:132-144` renders `${n} ${kind.replace(/_/g," ")}` → **"1 movement"**, **"1 warehouse tokens"**, against `Board.tsx:91-98`'s own comment arguing the opposite — *"The CLI's alphabet, not one of our own … A browser inventing `m` and `w` gives a player cross-reading the two surfaces an unknown token"*. And `SpaceReward.movement` is the *assistant*-move icon (`board.rs:109-111`), so "1 movement" reads as President movement, which is wrong | It is the same cell W6 regroups into TL/TR/ML/MR |

### 7.2 Fold in only if the owner wants M3.5 to carry non-layout work

These are Wave 0's own §6 "owed" list plus two from the critic. **All still open,
verified at this head.** They are not layout and I do not propose them as M3.5
work, but two of them decide whether M3.5's own claims can be checked:

- **CI runs only `cargo test -p yokohama-core`** — `.github/workflows/test.yml:11`
  is the entire test job. Nothing in this lane can make CI red, and M3.5 is
  almost entirely CSS, which is the change class a human reviewer is worst at
  and a browser test is best at.
- **Cache headers are backwards.** `art.rs:124` sets `public, max-age=3600` on
  stable-named files; `grep -n CACHE_CONTROL crates/yokohama-web/src/assets.rs`
  returns **nothing**, so the content-hashed Vite bundle is served uncached.
- **No graceful shutdown**, with `tokio`'s `signal` feature enabled to prove it
  was considered.
- **The 409 dead end.** `api.get` has **exactly one call site in the whole
  client** — `NewGame.tsx:208` — so once a game is running there is no re-read
  path at all, while the server's 409 message (`lib.rs:367-368`) says *"Re-read
  the game and answer what it asks."* Every subsequent click re-sends the same
  stale `expect`; the only escape is a page reload into the setup screen. **This
  is also the machinery M4 needs**, which is the argument for doing it before M4
  rather than inside it.
- **`api.ts` parses before it checks `res.ok`.** `api.ts:25-26`: `const text =
  await res.text(); const parsed = text ? JSON.parse(text) : null;` — unguarded,
  before the `if (!res.ok)` at `:27`. Behind Cloudflare Access (the documented
  deployment) an expired session returns an HTML interstitial and the player sees
  a `SyntaxError`, with the status code — the only actionable fact — discarded.

### 7.3 Explicitly NOT in M3.5

- **The replay scrubber and everything it needs.** M4. And one M4 decision is
  already settled by a measurement recorded in `-wave0.md` §7 — `why?event=n`
  times a real `state_at` replay at **~1.2 µs per event**, so no checkpoint
  ladder is needed. *I did not re-measure that figure; it is carried.*
- **The per-seat 5-Power token strip.** M4-adjacent (§5.3).
- **The four §4.3 items genuinely off the wire.**
- **Identity, authorization, per-seat views.** Recorded as a blocker in
  `PLAN.md`; the ordering constraint that mattered (the setup seed must stop
  crossing the wire first) was already discharged by Wave 0.
- **`spawn_blocking` on the two search routes.** Real; a single-operator server
  does not feel it.
- **The `sites/` rename** (§5.2) — correct and cheap, but opportunistic.

---

## 8. A proposed sequence

**W1 → W2 → W3 → W4 → W5 → W6 → W7 → W8 → W9**, with §7.1 folded into whichever
step opens each file.

The dependency reasoning, stated so it can be disagreed with:

1. **W1 first, and alone if the milestone gets cut to one item.** It is the only
   change that makes the page's height stop mattering, and §2.2's measurement is
   that no realistic combination of the other items gets the page under one
   screen: the rail is 1513–1826 px and the board contributes nothing to it. W1
   converts "scroll 1,086 px to answer" into "the answer is always in view" with
   one rule and one class.
2. **W2 before W3**, because `inert` on `.stage` is the honest way to satisfy
   `aria-modal`, and putting `inert` on a panel that covers 98.8 % of the board
   is a different (and worse) decision from putting it beside a drawer.
3. **W4 before W5.** *This is where I disagree with the review's own sequencing.*
   The review put the art fix fourth "because it changes tile height and
   therefore wants items 1 and 3 already landed". Measured, the height cost is
   **zero page pixels at every player count** (§2.3). The real dependency is on
   **WIDTH**: the art's rendered size is `--tile-w × 531/709`, so W4 decides
   whether the unveiled ladder is 160, 192 or 224 px wide, and W5's fourth step
   (the review's `--tile-w: 16rem`) is a measured regression that W4 replaces.
   Landing W5 before W4 means measuring the art's legibility at a width that is
   about to change.
4. **W6 anywhere after W2**, because two of its rows (the Areas panel's token and
   site blocks) add height to a panel whose readable width the drawer decides.
5. **W7, W8, W9 last** — independent, small, and each improves a screen the
   earlier items have already made worth looking at.

**Honest sizing.** W1+W2+W4 is a day's work and buys most of the measured value.
W3+W5+W6 is the milestone. W7+W8+W9+§7.1 is a tail that can be cut without
anything above it becoming untrue. **M3.5 as scoped is smaller than M3 and larger
than M2.5**, and it is almost entirely CSS and client code — which is exactly the
change class §7.2's CI gap is worst at catching.

---

## 9. Open questions for the owner

Genuine forks only. Each states the impact chain before the options.

**Q1 — The art treatment: caption below, or two anchored scrims?**
*Impact chain.* The tile's printed power ladder is the fact a Movement
destination and an Area Action are chosen on, and it is currently at a measured
mean transmittance of **0.100** with no other on-board home — the only surface
that carries it is the Areas modal, which covers 98.8 % of the board. Removing
the veil and the scrim takes it to 1.000 and **subsumes the whole "put the power
ladder on the tile" work item**. It also changes what a tile IS: a picture with
a caption under it (168 → 260 px at 14rem), instead of a dark rectangle with
text on it. Measured: that costs **zero page height** at every player count.
*Options.* (a) Caption below — maximum legibility, changes the board's visual
character. (b) Two scrims anchored to the text bands only — one rule, leaves ~30 %
of the tile at 100 %, and **still buries the ladder**. (c) Neither; the owner
looks at (a) rendered and rules then.
**This is a look, and I can render it before it is ruled.**

**Q2 — Does the board fit its column at every player count, and is the ladder
legible when it does?**
*Impact chain.* Today 384 px of a 4P board and 154 px of a 3P board are
off-screen at 1440 with no visible scrollbar. Scaling `--tile-w` to 10rem (4P) /
12rem (3P) / 14rem (2P) puts every tile on screen — measured, 0 px of scroll at
all three. But at 10rem the tile's art is **160×120**, and **I did not measure
whether an unveiled printed ladder is readable at 160 px wide.** If it is not,
Q1(a) and Q2 pull against each other and one of them has to give.
*Options.* (a) Scale the width; accept whatever legibility 160 px gives at 4P.
(b) Keep 14rem and keep the horizontal scroll, but make it visible (a thin
scrollbar, ~3 lines). (c) Scale to 12rem everywhere and let 4P scroll 192 px.
**A screenshot at each width answers this in one pass and I have not taken it.**

**Q3 — Do the two open Tier-B surfaces (`tracks`, per-area `sites`) belong in
M3.5?**
*Impact chain.* They are the last two items — three with the missing tracks
companion pointer, which is a live bug pointing players at a nonexistent surface
— between this lane and *"the audited CLI parity gap is closed"* being a true
sentence, which `guidingpath.md:204` currently records as overstated. The
`tracks` surface is also what converts §4.1's tooltip-only per-space rewards into
visible text, so two rows collapse into one piece of work. But they are new
surfaces, not layout, and they roughly double W6.
*Options.* (a) Both in M3.5 and close the parity claim. (b) `tracks` only (it
absorbs a parity row and fixes the dangling pointer); the per-area `sites` view
waits. (c) Neither; fix the pointer to say nothing and defer both.

**Q4 — Does M3.5 carry any of the §7.2 "owed" list, and specifically CI?**
*Impact chain.* M3.5 is almost entirely CSS. `.github/workflows/test.yml:11` runs
`cargo test -p yokohama-core` and nothing else, so **no test in this lane runs in
CI** — the browser walk that would catch a layout regression is precisely the
thing CI does not run. Every claim M3.5 makes about layout will be a claim
checked once, on one machine, by hand. Adding the lane to CI means CI needs node,
an npm install and a Chromium download; that is a real cost and a real cadence
change.
*Options.* (a) Add `npm test` + `typecheck` only — cheap, no browser, catches the
component and contract half. (b) Add the browser walk too — catches the layout
half, which is the half M3.5 is. (c) Neither; M3.5's layout claims stay
hand-checked, which is the status quo and should at least be said out loud.

**Q5 — Is the 5-Power request answered by the M4-adjacent restatement?**
*Impact chain.* The request as worded asks for something the engine does not keep
in state (`player.rs:225` is a bare count). §5.3 establishes that it IS buildable
as a replay projection over the `GameRecord` — the same `state_at` machinery M4
builds — so it is cheap after M4 and expensive before it. Meanwhile the thing
that actually shows "where the shops and trading houses are built" is the
building-site card, whose occupancy is **already drawn on the board tile** in the
builder's seat colour (`Board.tsx:78-108`); the missing piece is only the
positional overlay on the card art.
*Options.* (a) Accept the split: the site-card overlay is candidate M3.5 work
(after Q1, since the target rect sits in the darkest band today), the per-seat
token strip is M4-adjacent. (b) Do the site-card overlay now and drop the token
strip. (c) Wait for M4 and do both together, which is the cheaper total.

---

## 10. What this document could not verify

Recorded rather than left to be discovered, in the shape `-m2.5.md` §5 set:

- **The five site-card overlay rects in §5.3** are carried from the review's
  Pillow measurement of S00/S13/S23. I did not re-measure them.
- **The F.1 fallback's text bands** (head 4.4–15.3 %, pawns/site-strip
  60.7–95.6 %) are carried and unverified.
- **The ~1.2 µs-per-event replay figure** in §7.3 is carried from `-wave0.md` §7.
- **`.area-grid` is the only panel I measured in a 702 px drawer** (§3, W2). The
  other ten are a reading of their CSS, not a measurement.
- **Legibility at 10rem** (§9 Q2) is unmeasured and is the one open question that
  could re-order §8.
- **My §2.2 page-height figures are ~100 px below the ones recorded in
  `-wave0.md` §7 and in the M3.5 brief** (1847 vs 1950 at 2P). I state my method
  and could not reproduce theirs; nothing in §3 turns on which is right.
- The suites at this head, re-run rather than quoted: `cargo test -p
  yokohama-web` **63** (57 lib + 1 gate + 5 `wire_fixtures`), `npm test`
  **195** in 10 files, `npm run typecheck` clean over both TS projects, and the
  overflow spec **3 passed** with the twelve readings in §2.1. The wire corpus is
  **41** files.
