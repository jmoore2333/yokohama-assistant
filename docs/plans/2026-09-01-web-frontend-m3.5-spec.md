# Web frontend — M3.5 "the screen": the ratified spec

**Written 2026-09-01 on branch `web-frontend`, at head `7d4cdd4`.**

**This is the APPROVED spec.** Its predecessor
`docs/plans/2026-08-31-web-frontend-m3.5-prep.md` was explicitly not one: it
proposed nine work items, priced them, and left six forks for the owner. Between
that document and this one an eight-cluster verification pass re-derived its
load-bearing claims from source and from live Chromium, and a two-lens
adversarial pass audited the result. **Seven HIGH and twenty-one MED findings
landed on the prep document's own sentences**, which is the fourth consecutive
milestone on this lane where that has been the largest category. §1 records the
owner's rulings; §2 records what the audit changed and is the reason this spec
is not simply the prep document's §3; §3 is the work.

**The prep document is not retracted and is not edited.** Where this spec
contradicts it, this spec wins and §2 says why, with the measurement.

Reading order: `-m2.md` → `-m2.5.md` §5 → `-m3-spec.md` → `-m3.md` §9 →
`-wave0.md` → `-m3.5-prep.md` → here.

---

## 0. The standing rule this milestone works under

Every claim in this spec that a reader might act on carries a `file:line` or a
printed number. **A sentence that cannot say how to check it is the shape that
keeps failing here** — that is the forward-corrections row's own wording
(`guidingpath.md:204`), and it is now four milestones old. Two consequences that
bind the work rather than the prose:

1. **No cost estimate is carried from the prep document.** Three of its costs
   were measured wrong (§2, H3/M16), and two of its rows were transcribed from
   untracked research with their cost cells attached (§2, H6).
2. **Every item lands with the assertion that guards it, or lands with the
   sentence saying no such assertion is possible.** The mutation test in §2.4 is
   why: today nothing in this repository can fail when this milestone breaks.

---

## 1. The owner's rulings (2026-09-01)

Six forks, ruled against re-measured evidence rendered as pictures
(the decision docket, artifact `74c81a9d`).

| # | Fork | **Ruling** |
|---|---|---|
| **F1** | The tile art: caption below, two anchored scrims, or leave it veiled | **Caption below.** The scrim fallback is dead on structure — §2.2 |
| **F2** | The board's width policy | **The 10/12/14rem ladder**, with the two caveats in §2.3 taken |
| **F3** | Scope | **Everything, corrected** — all nine items plus the seven-item tail |
| **F4** | CI | **`npm test` + `npm run typecheck`.** The browser walk stays local |
| **F5** | The last two parity surfaces | **Both** — `tracks` AND the per-area building-sites view |
| **F6** | The 5-Power request | **Superseded by the owner's own clarification — see §1.2.** The token FACE goes on the board tile in M3.5; the per-seat collected-token strip is **dropped, not deferred** |

### 1.1 The owner's clarification on F5/F6, and why it moves nothing

Mid-session the owner supplied the physical cards the request was about: **the 24
building-site cards** — a 2×2 grid of reward spaces over a trading-house band
carrying a VP star. *"They are what maps to the options for building."*

That is the card the **F5** per-area surface renders, so the request is answered
in M3.5 and not deferred. **F6's deferral applies only to the per-seat 5-Power
TOKEN strip**, which is a different object (a 128×128 token face saying what a
seat gained at 5 Power) and is the half that genuinely needs `state_at`
machinery. The two rulings are consistent and neither is re-opened.

### 1.2 The owner's clarification on F6 — it SHRANK, and moved forward

F6 was ruled "wait for M4" against a framing this spec's author supplied, and the
owner then corrected the framing:

> *"what I was talking about here was just the token faces visible on the board —
> in terms of collecting the tokens knowing which ones were taken isn't something
> we need to build beyond what the CLI has. This was an optimization to make it
> easier for a player to see the state of play while looking at the board without
> having to leave the page or the board to see things that influence their next
> move."*

**So the F6 ruling is superseded by its own premise being wrong**, and the answer
is smaller in both directions:

- **DROPPED, not deferred:** the per-seat "which tokens has each seat collected"
  strip. The CLI's count is enough. The replay projection over `state_at` is
  therefore **not being built at all**, in M3.5 or M4 — every paragraph about
  `apply_move.rs:927`, `resolve_acting_area` and the mercenary mis-attribution
  trap is now moot for this lane and is recorded only so a future reader does not
  re-derive it.
- **ADDED to M3.5:** the 5-Power token FACE drawn in its printed slot on the area
  tile — §3.10a. It needs no replay, no new field and no new route: `tile.token`
  is already on the wire for every tile.

**This is the milestone's thesis, stated by the owner better than the prep
document stated it**: everything that influences the next move should be legible
*on the board*. It is the same argument as the art unveil (§2.2) and the width
ladder (§2.3), and the three items are one intent.

**Verified live, because the prep document's guidance here was wrong in a way
that would have shipped an empty card (§2.5, H7):** the `AreasPanel` frame
carries the LIVE site —

```
rows[0][0].tile.site = {
  "index": 17,
  "slots": [ { "built_by": null, "reward": {...} }, ×4 ],   // TL,TR,ML,MR
  "trading_house_vp": 8,
  "trading_house_owner": null
}
```

`index` keys the committed art (`sites/S17.webp`), `slots` is TL/TR/ML/MR in
`types/board.rs:127-128`'s order, and `built_by` is the field that fills as
shophouses go up. **The card INDEX payload (`/index/sites`) is the pristine
registry card and its `shophouses_built` is `[null,null,null,null]` on all 24
faces forever** — an overlay built against it renders a permanently empty board.
`web/src/types.ts:629-631` already carries that caveat in prose; this spec makes
it structural (§3.10).

---

## 2. What the audit changed

Only the findings that move work. The full list is the falsification report; the
figures below were each taken at this head.

### 2.1 `inert` on `.stage` cannot satisfy `aria-modal` — the W2→W3 arrow is deleted

The prep document proposed `inert` on `.stage` as the honest alternative to a
focus trap, and made it the sole argument for doing the drawer before the
accessibility floor (its §8.2).

`App.tsx:73-167`: the children of `div.app` are `header.banner` (which CONTAINS
`nav.views`), `div.end-strip`, the warnings `div.decision`, `div.stage`,
`<Surface>`, `<Decision>`, and the error `div.decision`. **`.stage` is a SIBLING
of the nav and of all three decision blocks, not their parent.** Measured with
`.stage.inert = true`: focus escapes the open panel at **Tab 2** — strictly
worse than today's Tab 6 — and Chrome's accessibility tree still reports
`button "Port offer" ignored=false`.

**Adopted mechanism: `<dialog>` with `showModal()`.** Measured in the same
Chromium to close the tab order AND prune the background from the accessibility
tree, which the hand-rolled `aria-modal` div does not. It also deletes the
`inert` toggle the prep document's own Files line put in `App.tsx`, so the
drawer becomes a pure stylesheet change. **W3 has no dependency on W2.**

### 2.2 The scrim fallback is dead on structure, not on tuning

The printed power ladder occupies **y 0.630–0.910** of the tile (re-measured
against the art; the prep document's `y 0.61–0.97` is wrong at both ends and
sweeps in blank ground plus the printed card edge — its headline mean of 0.100
survives the error by coincidence, being 0.100 over the corrected band too).

The client's own lower text block — `.tile-row.pawns` through `.site-strip` —
occupies **y 0.607–0.956**. **They are the same 35% of the tile.** A scrim
anchored to the text bands is a scrim over the ladder: constructed and measured
at **0.087, worse than base's 0.100**, and at scrim alpha 0 the client still
draws `1 Import`, `Site 17` and the site chips as opaque foreground on exactly
those rows. There is no smarter scrim, which is why F1 collapsed to a look.

**And the legibility question that was supposed to gate the art fix is answered
in the affirmative at the smallest rung.** At a 160px tile a printed rung
numeral renders **7.90px cap height**, **0.34px taller** than the `.site-strip`
digits this design already ships on the same tile and treats as readable —
measured on all 17 non-Canal arts. This also falsifies `styles.css:10`'s own
justification for the veil (*"a tile at 15rem cannot render a printed power
ladder legibly"*). **F1(a) and F2 do not pull against each other**, and the
prep document's W4→W5 arrow survives only as an ordering convenience.

### 2.3 The width ladder fits, with two caveats the prep document does not carry

Measured across 3 player counts × 5 viewports × 4 tile widths (60 cells). The
ladder's headline reproduces exactly: at 1440 board scroll goes **384 → 0** (4P)
and **154 → 0** (3P), 2P unchanged at 0.

- **The 4P fit has 0.047px of headroom.** The realized row is 991.953px against a
  992px column — Chromium's row gap resolves to 6.390625px, not 6.4. At viewport
  1439 it clips 1px; at 1425, 15px. A classic 15px scrollbar (Windows, or macOS
  set to always-show) gives a 977px column and the exact fit breaks. **So the
  prep document's own proposed test — `scrollWidth - clientWidth === 0` at 1440
  — is the most brittle assertion in the plan and is NOT adopted** (§3.2).
- **1201–1439 is entirely unfixed, not just 1201.** The board column is
  `viewport − 448` across the whole row layout, so 1280 is equally dead and
  appears in no table of the prep document: base clipping 83/314/544 at 2/3/4P,
  and 83/154/160 under the ladder. The honest statement is *nothing from 1201 to
  1439 inclusive is fixed by F2*.

**And the implementation key changes.** The prep document says key on
`board.player_count`. That field exists (`types.ts:102`) and has **zero client
consumers today**; reading it would make `Board.tsx` the first place to assume
the pyramid's shape, against its own module doc at `:5-11` (*"Nothing here
assumes a pyramid — any geometry the engine hands over renders."*).
`Math.max(...ordered.map(([, l]) => l.length))` is already in hand at
`Board.tsx:40`, gives the same three answers, and is the term the width actually
depends on.

### 2.4 Nothing in this repository can fail when this milestone breaks

This is the finding that makes F4 a gap rather than a preference, and it governs
§3's every "guard" clause.

- `.github/workflows/test.yml:11` runs `cargo test -p yokohama-core` and nothing
  else. `release.yml` is tag-only and tests `yokohama-cli`.
  `grep -rn "npm\|clippy\|yokohama-web\|playwright\|node" .github/workflows/`
  returns nothing.
- **The 195 component tests never load the stylesheet.** `web/src/styles.css` is
  imported by `src/main.tsx:4` and nothing else, and vitest runs jsdom, which has
  no layout engine.
- **The browser walk misses the change class this milestone IS.** Mutation-tested
  against `the_screen_itself.spec.ts:282-296`: `.tile{width:20rem}`, a 600px
  banner, a 1200px push-down, `.rail{position:fixed}` over the board, 1px tiles
  and `color:transparent` are **all MISSED** — absorbed by `.board`'s own
  `overflow-x: auto`. What it does catch: `overflow-x:hidden`/`visible`, an
  over-wide rail, a stage margin.

There is no vertical assertion, no overlap assertion and no visual baseline
anywhere in the repository.

### 2.5 Seven further corrections that change an item's shape

| | The prep document says | Measured |
|---|---|---|
| **H2** | `max-height: 45vh` "does not scroll internally in ordinary play" (405px against 173–209) | Real prompts driven through the API measure **146 / 201 / 562** option rows — natural heights **5,721 / 7,827 / 21,646px**. The committed `pick.json` fixture alone has **10** rows; internal overflow engages at **8 rows @900px, 5 @700, 4 @600**. At 562 rows Confirm and Undo sit at internal offset **22,088** inside a 405px box with no page-level cue |
| **H5** | The risk is "three sticky bars" | The warnings strip **never sticks** (`bottom:0` only shifts a box upward). What happens is PLAY and ERROR pinning to the same edge at equal `z-index`; ERROR is later in DOM, so `elementFromPoint` on **"Apply the only option" and "Undo" returns `div.error`**. Scoping to `.decision.play` then leaves the error strip **off-screen at vTop 2237** |
| **H3** | Game-over standing: "5 lines" | `ClientState.scores` is `{seat,total}[]` in **seat order** with no rank and no `won` (`types.ts:335`); `Decision` receives five props and **no game id and no api handle** (`App.tsx:156-162`). A winner mark inferred from the margin is **the exact inference `Index.test.tsx:229-232` exists to forbid** — a margin of 0 is not a win |
| **H4** | `the_screen_itself.spec.ts:17` "would catch a broken swap" | Its only assertions are `naturalWidth !== 0` and `images.length > 10`. A swap that lost the box reservation loads the image and passes |
| **M18** | The drawer: "zero component changes", `.area-grid` measured | **11/11 surfaces contain cleanly at 1440 and 1280** — better than claimed. **But it regresses below 1201**, which the prep document never tests: `min(44rem,55vw)` shrinks faster than the modal, so `suggest`/`why` clip a `bar-name > code` by 6px at 1024/900 and **`table.outcome` escapes the body by 19px at 900** |
| **M9** | The nav grouping "un-buries the nav at 1280, 1201, 1024 **and 900** — measured" | **False at 900** (nav bottom 88.28 > 48; still two rows; 7/7 controls still unclickable). Correct at the other four, and the recoveries are *better* than claimed (56.09px vs "~60" at 1440) |
| **M3** | Rail art shrink + unstack = −130/−83/−51 | The **unstack half buys 0px at every player count** (the achievement strip's children are `width:100%`; the hand holds one card at turn 1). The whole delta is the art shrink: **−130 / −100 / −50**. At 3P/4P the row is already **caption-bound** at 3.4rem |
| **M13** | SpaceChip renders "1 movement", "1 warehouse tokens" | **Neither string is producible** — in the 24-card registry `movement` only ever takes 2 and `warehouse_tokens` only 2 or 3. The defect is **larger**: **55 of 96 printed spaces render differently from the CLI**, and `2 movement` actively misstates a rule (`board.rs:109-111`: it is the *assistant*-move icon) |
| **H7** | `areas.json` carries `built_by` "populated"; `types.ts:632-640` exposes the index fields "with no caveat" | Nothing is built at turn 1, so "populated" is not demonstrable from any committed or live payload — the contrast the document uses to steer the implementer is not observable. And the caveat **already exists** at `types.ts:629-631`, written in an ancestor of the document's own commit. The `PrintedSiteCard` split is the part still open |

### 2.6 The corrected dependency graph

```
  W1 pin ──┬── MUST carry the warnings/error strips' own answer (H5)
           └── MUST clamp the OPTIONS, not the zone (H2)
             │
             │ collision: 4px of margin at 1440, already lost at 1280
             ▼
  W2 drawer ── needs a sub-1201 floor (M18) ──▶ W7 nav (fixes the 5/11
                                                the drawer still buries)

  W3 a11y  ── NO dependency on W2 (§2.1). Needs §3.11's refusal predicate
              and three permanently-mounted containers, or role="status"
              announces a rejected click as an ordinary delta

  W4 width ── independent; key on max row length
     └── W5 art ── convenience only; legibility at 160px is measured

  W6 parity · W8 game-over · W9 rail art — independent
  F5 tracks · F5 per-area sites — new surfaces, after W6's vocabulary
```

---

## 3. The work

Numbered by the prep document's W-labels so the two documents can be read
together. Every item states its guard.

### 3.1 W1 — the pinned deck

**Not "one rule and one class".** The three `.decision` blocks
(`App.tsx:117`, `:156` → `Decision.tsx:101`, `:164`) become ONE sticky container
so there is no z-order between them and no strip can be pinned over another or
pushed off-screen. The `.first()` workaround in `web/e2e/harness.ts` stops being
load-bearing.

- `App.tsx`: wrap warnings + `<Decision>` + error in `<div className="deck">`.
- `styles.css`: `.deck { position: sticky; bottom: 0; z-index: 10 }`.
- **Clamp the OPTIONS list, not the zone** (H2). `.decision .options` takes the
  `max-height` and the internal scroll, so `.controls` — Confirm, Cancel, Undo,
  Apply — stays outside the scroller and on screen at 562 rows as at 3.
- A spacer equal to the deck's height, so no board tile is permanently
  unreachable during a placement picker.

**Guards.** (a) `App.test.tsx`: on a fixture carrying warnings AND an error,
exactly one `.deck` holds all three blocks. (b) `the_screen_itself.spec.ts`: at
1440×844, at scrollY 0 **and** at `document.body.scrollHeight`, the deck's
`rect.bottom` is within 1px of `innerHeight` and `rect.top ≥ 0` — two offsets is
what makes it "pinned" rather than "at the bottom of a short page". (c) The
tallest `.options` scrollHeight is **recorded** in the walk, as board scroll
already is at `:298-301`; no cheap assertion exists and the spec says so.

### 3.2 W4 — the tile ladder

`Board.tsx` sets `--tile-w` as an inline custom property on `.board`, keyed on
`Math.max(...)` of the realized row lengths (§2.3), at **10rem / 12rem / 14rem**
for widest-row 6 / 5 / 4. `.board` gains `scrollbar-width: thin; scrollbar-color:
var(--line-bright) transparent` so the residual clipping from 1201 to 1439 has an
affordance.

**Guards.** `Board.test.tsx` asserts the inline custom property per fixture —
jsdom reads inline style properties, no browser needed, and the ladder is the
actual regression. **The pixel fit is RECORDED, not asserted** (§2.3).

### 3.3 W5 — the art unveiled, caption below

1. Delete `--art-veil` (`styles.css:56-60`) and the `opacity` at `:159`.
2. Delete `.tile.has-art { aspect-ratio }` (`:156`); give the aspect to the image.
3. Delete `height: 100%` (`:167`) and the gradient (`:168-176`) from `.tile-body`
   — **`:177` is `text-shadow` and candidate (a) keeps it** (M10: the prep
   document's cited range `:163-177` would delete the caption's shadow).
4. `.tile-row.pawns { margin-top: auto }` (`:191`) goes inert under `height:
   auto`; remove it rather than leave a dead rule.
5. **No `--tile-w: 16rem`** — measured 51px of 2P clipping at 1440.

**Guards.** `getComputedStyle('.tile-art').opacity === "1"` and
`getComputedStyle('.tile-body').backgroundImage === "none"` in the browser walk —
literally the two rules deleted, no magic number. Plus a box-reservation test:
block `**/art/areas/*`, read `.board` height, release, re-read, assert equal
(measured 0.00px jump today in both base and candidate (a)).

### 3.4 W2 — the drawer, with a floor

`.surface { inset: 3rem 2rem 3rem auto; width: min(44rem, 55vw) }` **above
1201px only**; below it the panel reverts to today's full-width modal, because
the drawer regresses three panels there (M18). The deck (§3.1) takes a matching
right inset while a surface is open, or its longest option label runs under the
drawer — measured 4px of margin at 1440 and 156px of overlap at 1280.

**Guard.** Per-surface containment in the browser walk: `.surface-body`'s
`scrollWidth − clientWidth ≤ 0` and no descendant's right edge past the body's,
reusing `readOverflow`'s existing `past()`/`blame()` machinery, **at 900 as well
as 1440**. `covers 308 of 992`, `two columns` and `left: 704` are design choices
and are not asserted.

### 3.5 W3 — the accessibility floor

- **(a)** `<dialog>` + `showModal()` (§2.1), replacing the hand-rolled
  `role="dialog" aria-modal="true"` div and the `inert` toggle.
- **(b)** Restore focus to the opener on close. Verified sound: React keys the
  nav button by `s.key`, so it is the same DOM node across open→close.
- **(c)** The focus yank. The effect's dependency must become **`[name]`, not
  `[]`** — `<Surface>` is NOT remounted on a surface switch (measured), so `[]`
  leaves focus on the nav button when you switch panels. `onClose` moves out of
  the dependency list.
- **(d)** `role="status"` on the four message channels and a `<main>` landmark —
  **with the three conditionally-mounted containers made permanent**
  (`App.tsx:116`, `:163`, `Decision.tsx:291` all gate the container on content,
  and a live region inserted with its text is unreliably announced).
- **(e)** Two defects cheaper than anything above, found by this audit: the
  surface's own `<header>` makes a **second `banner` landmark** inside the
  dialog, and the four `section.panel`s are unnamed generics.
- **(f)** `--fg-faint` measures **2.97:1** on `.tile-id` — the area id, this
  project's canonical identification key — and on all four rail headings, and
  **3.42:1** on the whose-turn line. One token, and the largest measured
  accessibility win in the milestone.

**Guards.** `Surfaces.test.tsx`: rerender with a different `name`, assert focus
is the panel — this catches both the yank and the `[]` mistake. `App.test.tsx`:
on a fixture with no warnings, error or messages, four `[role=status]` containers
exist. The browser walk: 12 Tab presses from the open panel, every
`activeElement` inside the dialog or `<body>`.

### 3.6 W7 — the nav

Keep Areas / Log / Suggest / Why inline; move the four card indexes **and Final
score** behind one `Cards ▾` `<details>`, using the pattern at
`Board.tsx:220-221`. **`INDEX_KINDS` (`Surfaces.tsx:75-80`, not `:74-79`) covers
four keys and `result` is not among them** — it renders through its own arm at
`:182-184` — so deriving the group from that map alone ships a four-item
disclosure and leaves Final score inline (M8). The group is `INDEX_KINDS` plus
`result`, stated once.

**Guard.** With a surface open, `elementFromPoint` on every nav control returns
that control, at 1440/1280/1201/1024. **900 is excluded and the exclusion is
recorded** — measured, the grouping does not clear it (M9).

### 3.7 W6 — the parity rows already on the wire

Nine rows, all client-only: tolled canal-crossing pairs; the 5-Power face and
"already taken"; the site spaces in words with builder and trading-house VP and
owner; the Areas panel's board-row grouping (`Surfaces.tsx:312` currently
`.flat()`s them); the per-seat `hand_hidden` marker; the site index's TL/TR/ML/MR
grouping; the near-miss **area name** (`Decision.tsx:252` prints the bare id
while `state.frame.board` is in scope); the turn player's **colour**
(`Decision.tsx:104`; `colors` is already a prop at `:86`).

**Two corrections to the row list.** The canal row buys less than stated: the set
is 0–9 pairs depending on where the Canal lands, a 2P board has **no Canal at
all**, and both live boards measure at or near zero while the committed fixture
is near the maximum (M15). And the TL/TR/ML/MR row is a restructure rather than
8 lines: the `Site n` and trading-house chips are **peers** of the four space
chips at `Index.tsx:180-191`, and 24 of the 96 spaces are blanks rendered as a
bare `—`, indistinguishable from an absent space (M16).

**Guard.** One vitest assertion per row against a committed fixture — presence
of the datum, never a layout number. **W6 is the best-guarded item in the
milestone.**

### 3.8 W8 — the game-over block, at its true cost

`state.scores` cannot support the CLI's block (§2.5, H3). `App` fetches
`/result` when `state.over` and passes the outcome down; `Decision` renders it
through `OutcomeView` (`Index.tsx:328`), which already exists and already has the
winner mark, the rank order and the category breakdown. **No margin inference.**

### 3.9 W9 — the rail art

`.panel .card-face.achievement img` 5.2rem → 3.4rem (`styles.css:271`). **The
unstack half is dropped** — measured at 0px (M3) — and the spec records why
rather than shipping a rule that does nothing. Benefit: −130 / −100 / −50px.

**No guard exists.** Its whole benefit is a vertical pixel delta and this
repository has no vertical assertion at any layer. Recorded, not asserted.

### 3.10 F5 — the two new surfaces

**`tracks`.** Church and Customs with per-space VP and rider **as visible text**.
This absorbs a W6 row — today those are `title=` tooltips only
(`Dashboard.tsx:286`, `:303`), invisible on touch, to a screen reader and to any
assertion — and it fixes the live bug: `Surfaces.tsx:343-344` renders *"detail
lives on the {companion} surface"* naming a `tracks` surface that does not exist,
and `screen_model.rs:826` maps `Church | Customs → Tracks`, so it is **exactly
two dangling pointers on every board at every player count, by construction**.

**The per-area building sites** — the owner's cards (§1.1). Every area's site
card drawn from `sites/S{index}.webp`, with the four TL/TR/ML/MR spaces and the
trading-house band showing **what is built and by whom**, read from the LIVE
`SiteModel` in the areas frame and never from `/index/sites`. The overlay rects
carried by the prep document from an earlier review are **unverified and must be
re-measured against a crop before anything is positioned on them** — this spec
does not launder them.

### 3.10a The 5-Power token, in its printed slot on the tile (§1.2)

**Rendered on the area tile art at the printed 5-space, from `tile.token`, which
the wire already serves on every tile.** No new route, no new field, no replay.

**The slot is MEASURED, not carried.** Ridge-detected across all thirteen
non-Canal area arts (each art minus its own 7px box blur, then averaged, so a
line printed on every card survives and every illustration cancels):

| edge | normalized | ridge |
|---|---|---|
| left | **0.816** | +14.18 |
| right | **0.941** | +12.73 |
| top | **0.588** | +13.29 |
| bottom | **0.721** | +13.73 |

(The verticals at 0.984–0.993 are the card's own outer border, not the frame.)
The token draws centred in that box at **0.95×** its shorter side. Rendered and
LOOKED AT across four arts and all three shipped tile widths, per this repo's own
rule that regenerating art without looking proves nothing.

**Two findings that shape it:**

1. **Presence reads at every shipped size; the FACE is marginal at 160px.** At
   the 4P tile the token is ~18px and resolves as "a token is here"; which good
   and how much resolves at 192 and 224. **So the caption keeps
   `fivePowerWords(tile.token)`** — the slot answers *is the bonus still there*
   across eighteen tiles at a glance, the words answer *what is it* when you look
   at one. The overlay ADDS to `Board.tsx:170-176`; it does not replace it.
2. **"Taken" needs no mark.** The printed frame with the red `5` visible IS the
   empty-slot state, exactly as on the table. The existing `5-Power taken` text
   stays for the non-visual channel and for any assertion.

**No hidden-information question exists here**, which is why this is cheap:
`01-rules-reference.md:680` — the tokens are shuffled face-down and then placed
**face-up** on each Area Board's 5-space, *"so its effect is public information
from the start."* The client has always been served `tile.token`.

**Guard.** `Board.test.tsx`: a tile whose fixture carries a token renders the
token image; a tile whose token is `null` renders none and keeps the taken text.
The rect itself is a design constant and is recorded in-source with its
derivation, not asserted.

**And the trap closes structurally**: split the index face into a
`PrintedSiteCard` carrying only `spaces` + `trading_house_reward`, so the always-
empty `shophouses_built` / `trading_house_owner` cannot be reached from the index
at all. A prose caveat did not stop the prep document from steering an
implementer at them.

**Guard.** The companion pointer asserts the rendered companion **names a key
that exists in `SURFACES`** — which generalises to a future companion — rather
than `not.toMatch(/tracks/)`.

### 3.11 The tail

| | Item | Correction the audit forces |
|---|---|---|
| 1 | `autosaved` is a hardcoded `true` (`client.rs:355`) while the server can reply `APPLIED BUT NOT SAVED` | That string is in `lib.rs:400`, **not in `App.tsx`**, and the two sites are 40 lines and two page regions apart (M14) |
| 2 | Priced rows state the price twice | The defect spans **five** fee-carrying move kinds (`decision_render.rs:800-874`), not the two named. And "the only fee-carrying row in the corpus" is true at `prompt.rows` grain only — `fee_nonzero.json` carries **47** nonzero option fees (M12) |
| 3 | The refusal predicate misses the REPL's own `!` | A leading-`!` test is **insufficient**: `repl.rs:1505` and `:3469` carry no bang and match neither predicate. Correct predicate is bang **OR** the existing regex, and those two remain unstyled — recorded. **This is a PREREQUISITE of §3.5(d)**, not a fold-in: one `role="status"` over a mixed array announces a rejected click as an ordinary delta |
| 4 | Confirm rendered where `step_pick` has no `confirm` arm | Guarded by **one line** on a fixture already imported at `Decision.test.tsx:238`. The same one-liner covers the `retreat` case the prep document does not name |
| 5 | `Dashboard.tsx:46`'s silent `return null` | Follow `Surfaces.tsx:287-298`'s own pattern — render the unknown shape rather than nothing |
| 6 | SpaceChip's private vocabulary | **55 of 96 spaces differ from the CLI**; the two example strings in the prep document are unproducible (M13). Single-source the vocabulary from the CLI's own words |
| 7 | The `Tracks` companion pointer | Folded into §3.10 |

**Two more the audit adds, because W1 makes the first one worse:**

- **The 409 dead end.** `api.get` has exactly one call site
  (`NewGame.tsx:208`), reachable only from the setup screen, and `App.tsx:36-52`'s
  catch never calls `setState` — so every subsequent click re-sends the same
  stale `expect` while the server's own message says *"Re-read the game and
  answer what it asks."* **After W1 the player stares at a permanently visible
  bar every button of which 409s.** A re-read path is also what M4 needs.
- **`api.ts:25-26` parses before `res.ok`** — two lines, same file. Behind the
  documented Cloudflare Access deployment an expired session returns an HTML
  interstitial and the player sees a `SyntaxError` with the status discarded.

### 3.12 F4 — CI

`.github/workflows/test.yml` gains `npm ci`, `npm run typecheck` and `npm test`.
The browser walk stays local by ruling. **Cache headers and graceful shutdown
stay OWED** — they are server hygiene, in no file this milestone opens, and
saying so is better than a silent carry.

---

## 4. What this spec does not claim

- **It does not claim the milestone is guarded.** F4 buys the component and
  contract half. The layout half is guarded only while someone runs
  `npm run e2e` locally, and §2.4 is the measurement of what that is worth today.
- **1201–1439 stays clipped** at 83–239px after F2 (§2.3). Closing it means
  moving the breakpoint, which is a separate decision.
- **The site-card overlay rects are unmeasured** (§3.10).
- **`--tile-w: 10rem` at 4P fits by 0.047px** and that is a property of overlay
  scrollbars, not of the ladder (§2.3).
- **No gate, no roster place.** The fifteen release-profile gates are untouched
  and nothing added here is `#[ignore]`d. `crates/yokohama-core` stays
  byte-identical to master; `SEMANTICS_REV` stays 7.

## 5. Baseline at this head, re-measured rather than quoted

| Command | Count | Wall |
|---|---|---|
| `cargo test -p yokohama-web` | **65** (59 lib + 1 gate + 5 wire) | 29.0 s warm |
| `npm test` | **195** in 10 files | 9.3 s |
| `npm run typecheck` | clean, both projects | 2.0 s |
| `npm run e2e` | **14** passed, 0 skipped | 36.4 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings | 12.1 s |
| wire corpus | **41** `.json` (43 entries — `contract.ts` and its test) | — |

The prep document's `63` was true one commit earlier; `af706de` added two.

---

## 6. The close (2026-09-01) — what shipped, and where the spec was wrong

Six commits, `acc9e50..2d0bcf8`. Written after the fact against the tree, not
against §3.

### 6.1 Every item, and its verdict

| Item | Shipped | Where the spec was wrong |
|---|---|---|
| **W1** the pinned deck | YES | §3.1 was right that it is not "one rule and one class" and wrong about which child yields. It named the clamp on the OPTIONS but did not foresee that `.options` is OVERLOADED — the `retreat` arm reuses the class for a message and count buttons — nor that letting the answer list absorb the deck's shrink squeezes it to a height of **exactly 0** under a bot's commentary. Both were found by the browser walk, four failing specs each time |
| **W4** the tile ladder | YES | As specified, including keying on the widest row |
| **W5** the art | YES | §3.3's step 3 cited `:163-177`; `:177` is the `text-shadow` candidate (a) keeps. Deleted `:167-176` |
| **W2** the drawer | YES, with the sub-1201 floor | The floor was specified. Two LATENT bugs the drawer exposed were not: `.surface-body` never actually scrolled (no `min-height: 0`, so `.surface`'s `overflow: hidden` clipped it), and a `<dialog>` takes `height: fit-content` from the UA sheet, which beats a top/bottom inset pair — the panel computed to 855px in a 720px viewport |
| **W3** the a11y floor | YES, via `<dialog>` | As specified. The contrast token was measured at 3.09–3.42:1 here, not the audit's 2.97 |
| **W7** the nav | YES | **Its justification did not survive.** The spec kept the review's argument — that grouping lets you switch surfaces with a panel open — and `showModal()` makes the whole page inert, on purpose. It earns its place on banner height alone (93 → 56px at 1440). The e2e cell was re-aimed rather than deleted |
| **W6** parity | YES, all nine rows | The vocabulary row was bigger than "~8 lines": it needed a shared vocabulary AND a cross-language fixture |
| **W8** game-over | YES, via `/result` | As specified |
| **W9** rail art | HALF, deliberately | The unstack half measures 0px and was NOT shipped. §3.9 said so |
| **F5** tracks + building sites | YES | The overlay rects §3.10 declined to launder were re-measured and the carried ones turned out to be right |
| **§3.10a** the token in its slot | YES | The rect was measured. Sizing it with `height: 95%` fails SILENTLY — a 16px slot rendered a 122px token |
| **§3.11** the tail | YES, plus the 409 and `api.ts` | — |
| **F4** CI | YES | — |

### 6.2 The two vacuous guards, and why they are the close's real lesson

Both were caught by mutation, neither by reading, and both are the shape this
lane keeps shipping:

- **The 409 cell counted `GET /api/games/{id}` absolutely**, and `renderApp`
  already performs one to reach the board — so it passed with the entire re-read
  deleted. Counts a delta now.
- **The near-miss cell early-returned**, because no committed fixture reaches a
  near-miss row (verified over all 42). It constructs the state now.

**The general form: a guard is worth what its mutation proof says and nothing
more.** That is also why the CI job carries a paragraph on what it cannot see,
and why the tile-fit is RECORDED rather than asserted.

### 6.3 What is still owed

- **The layout half is checked by hand, on one machine.** F4 put the component
  and contract half in CI by ruling; the browser walk stays local. The seven new
  browser cells are the only assertions in the repository that can see a CSS
  change, and nothing runs them automatically.
- **1201–1439 still clips** 83–239px at 3P/4P (§2.3). Closing it means moving
  the breakpoint.
- **The 4P fit clears by 0.047px** and is a property of overlay scrollbars.
- **`repl.rs:1505` and `:3469` are still unstyled refusals** — they carry no bang
  and match no pattern. Closing that means a typed refusal on the wire.
- **Cache headers and graceful shutdown** stay owed, as §3.12 said they would.
- **The privacy toggle** — the third Tier-B row — is untouched; F5 closed the
  other two.
