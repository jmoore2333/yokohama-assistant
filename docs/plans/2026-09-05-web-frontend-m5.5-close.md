# Web frontend — M5.5 "the table reads": the close

**Written 2026-09-05 on branch `web-frontend` at `cc01dab`**, forty-seven commits over the
spec's own base `652badc`. The spec
(`docs/plans/2026-09-04-web-frontend-m5.5-spec.md`) is the authority for everything below;
the plan (`docs/plans/2026-09-04-web-frontend-m5.5-plan.md`) is what was executed, and the
SDD ledger of the run is the record every figure and every ruling here is traceable to.
The live deployment is **`yokohama-web:0.5.0-m5.5.2`, built from `fb92c17`** (`deploy/RUNBOOK.md`
§3.6 and its addendum); `cc01dab` is that runbook addendum and this close is the commit after it.

**This document is M6's entry artifact** (spec §6.3, quoted verbatim in §7 below).

Nothing here re-opens an M4 or M5 ruling. Where a spec sentence was read differently, or
delivered differently, §3 names it and says which document it departed from.

---

## 1. What shipped, part by part

### Task 0 — M5's owed close (spec §3.0)

Ships no code. M5's §8 was written into the M5 spec itself (`4c8c2dc`, forty-five lines) rather
than into the separate close file spec §3.0 item 1 names — the plan's deviation 4, ratified as
ruling R-T0-1. The guidingpath decision-log row and `CLAUDE.md`'s browser-half counts landed with
it (`b92c508`), and one review round corrected two of that row's own sentences (`9b25096`): the
walk covers **four** viewports × three player counts, not three, and the art-regeneration sentence
spec §3.0 item 2 owes (M5's source-width re-cut, `CARD_W` 530 / `TILE_W` 709; the site and
achievement crops turned a quarter turn counter-clockwise) was added to `CLAUDE.md`'s `mkcardart.py`
line under ruling R-T0-2. The `README.md` re-check found nothing false. The M5 spec's own 15.4 rem
prediction for 2P at 1710×869 was recorded forward as the 15.31 rem the formula gives.

The counts at the milestone's base `b1ccaa4` matched the plan's expectations exactly and were
independently re-measured by the task reviewer: vitest 296 in 19 files, e2e 32, `cargo test -p
yokohama-web` 72, wire corpus 43, help pictures 5 at 4,199,746 B, art 132 files.

**R9's verdict, in the owner's words** (spec §1): the two rounds of play on the deployed M5 client
are the V2/V3 answer M5's close was waiting for — pictures accepted, the pieces band's legibility
carried into part A and the log box's shape into part C.

### Part A — the pieces band (Tasks 1, 2, 3, 4, 4b)

`web/src/piecesLayout.ts` holds one pure function, `layoutPieces(crowd, band) → Layout`: a colour
with four or more cubes becomes a **pile** (two rows, ⌈n/2⌉ columns, the odd cube on the top row,
every cube 0.50 h), `units` is the sum of element widths and gaps, `h = min(H, W / units)`, and the
floor is **h ≥ 0.55 H**, with the collapse loop repeating until it holds or until no group is left
whose collapse would help. The CSS reads `--piece-units` instead of a glyph count, `.pile` draws
the two rows, and the pieces pack to the band's right edge.

**The collapse rule as shipped is spec §3.1.1's rule with its own purpose applied** (ruling
R-T1-1): the candidate is the largest not-yet-collapsed group **among those whose collapse
reduces `units`**. A lone cube costs 0.72 drawn and 1.13 collapsed, and a four-cube pile costs
1.00 drawn and 1.13 collapsed, so collapsing either moves the row away from the floor — measured
at band width 0.61 for {4 pawns, [4,3,3,3], station}, where the plan's literal transcription
collapses indices 0 then 1 (h 0.5902) while index 1 alone clears the floor (h 0.5971). When no
remaining group's collapse reduces the row the loop ends with the honest h and returns the piles
it still has.

**The rect is a measurement, not a choice** (spec §3.1.2), and the measurement needed an eye
(ruling R-T3-1). `docs/tools/tilerects.py --ink` reports the rightmost ink inside the band's rows;
on nine of the thirteen area arts it reads *illustration* as ink (Port A's band rows are 55 of 66
pale sky, so its dark shapes register as ink on the panel). The raw readings and the
eye-confirmed ones, all thirteen:

| art | raw `--ink` | eye-confirmed printed ink |
|---|---|---|
| Bank | 0.432 | illustration |
| Chinatown | 0.465 | **0.320** |
| Church | 0.379 | **0.379** (raw correct) |
| Copper Mine | 0.451 | illustration |
| Customs | 0.468 | illustration |
| Employment Agency | 0.426 | illustration |
| Fishing Ground | 0.456 | illustration |
| Laboratory A | 0.463 | **0.395** (a roofline was the raw reading) |
| Laboratory B | 0.464 | **0.392** |
| Port A | 0.468 | illustration |
| Port B | 0.468 | illustration |
| Silk Mill | 0.460 | illustration |
| Tea Plantation | None | none |

Printed icons reach the band's rows on exactly four arts. **Laboratory A binds at 0.395, so
L = 0.395 + 0.02 = 0.415**: the band is x 0.415 → 0.97, width 0.555, aspect **6.025**. The spec
expected "near 0.35" from a Church-only prior that was about 0.05 low and never saw Laboratory;
the literal "max of thirteen" would have read 0.468 and *narrowed* the band. Two eyes checked the
contact sheet `docs/temp/web-review/tile-rects.png` before the value landed, and Laboratory clears
by exactly the margin — about 7 px at a 22 rem tile.

**Recorded readings** (this Mac, headless Chromium at device scale factor 1, CSS px; recorded,
never gated):

- pile cube **11.2 px** tall and wide on A00 (Laboratory), 2P at 1710×869, tile 245.0 × 184.0 —
  the e2e cell's own printed line, reproduced by the task reviewer on a second server-drawn seed
  and by the implementer on a third. The mock predicted 11.19 at L = 0.30; the measured band
  reads 11.2.
- the final review's read of the same thing on the **deployed** client: `.piece-group.pile` with
  rows [2, 2], four `.piece.cube` of 9.9 × 9.9 px, no `.piece-n`, `aria-label="4 Red assistants"`,
  `--piece-units: 1.000`.

**Task 4b was inserted by the owner at gate A** (ruling R-GA-1): the tile popover's enlarged art
mounts the *same* band the board tile draws, at the same `pct(PIECES)` rect, with the Canal
excluded and the words row unchanged. The two components' four inline `pct` strings are compared
to each other, so substituting a different rect fails.

**Gate A pictures** (`docs/temp/web-review/gates/`, sent to the owner 2026-09-04, no stop):
`a-pile-1710x869.png`, `v1-2p-1710x869.png`, `v1-4p-1710x869.png`, `tile-rects.png`, and
`v1-2p-2560x1440.png`, which is the frame the owner answered on. Task 4b re-shot
`v1-4p-popover.png` — the 4P popover showing the Yellow president on the enlarged Copper Mine art
at 53 px against the board tile's 30 px.

**The owner's gate-A answer:** *"when hovering over a card the assistants don't appear while the
workshop does, should we adjust for that? Otherwise they look good with the wider lens and the
sizing."* Then: *"let's take it as a small task 4b before part C starts."*

### Part C — the turn playback (Tasks 5, 6, 7, 8, 9, 9b)

**The wire, all additive** (spec §3.2.1). `ClientState.events` is the record's length after the
reply, read off the count the `input` handler already holds. `HistoricalFrame` gains `ports` and
`labs`, built by the same `screen_model` builders the live routes call, over `state_at(k)`. The
corpus regenerated as the contract it is: twenty `ClientState` fixtures gained `events`,
`at_k.json` gained the two panels, nothing else moved (21 files, +280 lines, 0 deletions).

The route cell that guards the frame's offers was made to discriminate (ruling R-T5-1): the
per-`k` `deck_remaining` identity could not tell a frame's port from a copy of the live one,
because both are `state.order_deck.len()`. The cell now plays until a port take has happened — the
premise asserted — then requires the `k == 0` slot cards to **differ** from the live port's and
the `k == total` ones to equal them. Fixing it exposed a latent defect in the lib test helper
`play`, which had stalled at 17 events on the first retreat sub-prompt since it existed; every
route cell built on it now walks deeper (a port take lands at 42 events) and the lib suite got
faster.

**The parts.** `diffFrames(a, b) → Changes` is a pure function over two frames that knows no rule;
`walk.ts` builds the run and reduces it; `playback.ts` holds the cadence and its storage seam;
`LastMove` became `TurnAccount`, one row per event with the phase chip, the mover's colour and
the log's own line, the lit row following the step; `App` drives it. The walk plays only when the
run contains a bot step, and the deck shows a one-line strip while it runs.

**Two owner rulings at gate C changed the design.** R-GC-1: `diffFrames`' slot rule became
**arrival** — a card new to the strip — because a laboratory compacts left after a take and refills
the rightmost slot, so the per-slot rule lit five slots and none of them was the card taken; a new
`techs` grain marks the taken technology in the acquirer's list, and every highlight **pulses once
per step for that step's dwell** (`--dwell-ms`, restarted by a beat-parity attribute, static under
`prefers-reduced-motion`) because spec §3.2.4 said "an outline pulse" and the plan's CSS had no
animation at all. R-GC-2, the owner's parity follow-up (*"does this logic apply to orders from the
dock/port as well as the tech cards, since it's the same mechanics?"*): a `hand` grain marks an
order card newly present in the **viewer's** hand; the bot's hidden hand marks nothing, because a
taken technology is public and a taken order is not.

**The observed turn's step count is seed-dependent and recorded, never gated.** The e2e cell prints
its own reading: `the observed bot turn walked 8 steps` at Task 9's commit and again at the final
review's gate run, 9 steps after Task 9's fix round. On the live site the final review's three
walks read 8, 8 and 9 steps, and the deploy's own on-screen check read 9. The network capture the
final review took of its second walk: `/at/14 … /at/22`, nine positions, ascending and contiguous,
plus `/log?tail=8`.

**Gate C pictures**: `c-step-1.png` … `c-step-6.png` and `c-live.png`, sent 2026-09-05;
`c-step-5.png` re-shot twice as R-GC-1 and then R-GC-2 landed, the final frame showing "Blue:
Laboratory: acquire Patent System" outlining exactly **one** lab slot — slot 6, holding University,
the card the deck turned over to refill — with the five shifted slots unmarked. `look-tech-chip-not-a-gate-shot.png`
is the separate look at the marked technology chip.

### Part B — the set table (Tasks 11, 12)

`POST /api/games/preview` returns `{ seed, table, deals }`, built from a **throwaway** `Repl` over
`Session::new_sim` with `StartingOrderChoice::Unknown` for every human seat and `Keep(argmax)` for
every bot seat, then `HistoricalFrame::at(&repl, 0)`. Nothing is persisted, no id is issued, no bot
is pumped. `NewGame.keeps` became `Vec<Option<u8>>`: a value on a bot seat is a 400 naming the
seat, `null` on a human seat is a 400 the same way, and `config_of` fills every bot seat from the
same argmax the preview showed — so a preview and a create at one seed put the same card in the
bot's hand, mutation-proven.

**The keep argmax needed two seeds to be proven at all** (ruling R-T11-2). Every cell and both
fixtures ran at seed 20_260_831, where seat 1's cards score 57 against 31 and the argmax *is* card
1 — so a constant `bot_keep = 1` passed the whole suite. The argmax, agreement and route cells now
also run at `ARGMAX_IS_CARD_2` = 20_260_830 (scores 9 against 36), and the re-reviewer forced the
constant to 1 and then to 2, each failing cells at the other seed.

**On screen** (spec §3.3.3): after Deal the setup screen is the M5 table over `preview.table` — the
pyramid with its site cards and tokens, the strips the browser has switched on, the banner reading
"setting up · seed …" — with the deals in the side column. The table is inert, the tile popover
works, and the Boards menu works. Three things the plan did not deliver and this task did:

- the **Boards menu** was missing from the set table's banner though spec §3.3.3 requires it
  (ruling R-T12-2). It became `web/src/BoardsMenu.tsx`, one component shared by the game banner and
  the set table — never duplicated.
- one **`useSections()`** in `App`, passed into `NewGame` as props (ruling R-T12-4). The first fix
  called the hook twice, so a strip switched off on the set table came back when the game opened,
  while two comments claimed the opposite. The cell now asserts the property end to end.
- the **height budget** (ruling R-T12-3). At 1710×869 the deck at 70vh took 557 px of an 808 px
  budget, leaving the side column 251 px for 423 px of bot content, so the bot's two Keep scores
  and its "bot keeps this" mark sat below the fold at scroll 0 — at the very viewport this
  milestone photographs. The deal cards now render without the reward-chip rows (the card art
  already prints requirements and rewards) and the cap is 62vh scoped to ≥1201px. Measured after:
  **deck 459 px holding 458 · side 349 px holding 349**, nothing scrolled, 26 px of slack on the
  bot panel.

**Recorded readings**: 0 px of page overflow for the set table at 390 / 1024 / 1201 / 1440, with
the board's own column scrolling 557 px inside itself at 390 (ruling R-T12-1 added that cell —
spec §4.3's last bullet had none in the plan).

**Two defects the pictures found before any reviewer did**, both fixed inside the task: the deck's
original 45vh cap cut the Keep scores, the CLI's hedge and the Start button out of frame, and the
plan's absolutely-positioned "bot keeps this" badge covered the kept card's own Keep score.

**Gate B pictures**: `b-set-table-1710x869.png` (re-shot at scroll 0 with the whole deal in view)
and `b-set-table-strips-1710x869.png` (scrolled to Port A and Laboratory A). Gates are pictures
delivered with no stop (spec §4.4), and gate B needed no owner answer.

### Deployment

Three images, each by the `deploy/RUNBOOK.md` §3.5 recipe through the Portainer Docker proxy, each
swapped onto the same address (172.21.0.4 on `cf-internal`, so the tunnel needed nothing) with the
named volume riding across, each **healthy at the first probe 30 s after start**:

| tag | built from | musl binary | context tarball | image | bundle |
|---|---|---|---|---|---|
| `0.5.0-m5.5` | `6d86989` | 6,105,720 B | 13,899,025 B | 25,993,976 B | `index-D9enc4gM.js` (282,553 B) |
| `0.5.0-m5.5.1` | `0689fe7` | unchanged | 13,899,474 B | 25,995,030 B | `index-BdX0BsM6.js` (283,439 B) |
| `0.5.0-m5.5.2` | `fb92c17` | unchanged | 13,899,577 B | 25,995,225 B | `index-CVFCAqs1.js` |

The musl binary moved once, by 16,064 B against M5.1's 6,089,656: the `events` count on the live
payload, the offers on the replay frame, and the preview-as-a-position with the bot's argmax keep
are the whole difference. It did not move again, because `crates/` did not. Six tags now sit on the
NAS as rollback points. Each deployment was verified through the tunnel from a WARP-enrolled shell
(`/api/health` 200 with `semantics_rev` 7 and `rules_hash` `d85445c479cb1ead`, `/` shipping that
build's own bundle, `/help/screen.png` 200) and then played on screen. The workstation's LAN
address is **192.168.1.221**, not the 192.168.1.16 the runbook's §1 names — read it before serving
the context.

Between them the on-screen checks and the final review left three saved games on the volume
(`sim-5c3b4e596bb8cf97`, `sim-7d949b884ebb2adb`, `sim-fa6b6bb5049a3929`), the owner's to delete.
The owner's own four saves predate this milestone: they were listed before and after every swap,
opened read-only where a check wanted one, never played on, and never deleted.

---

## 2. The rulings taken during execution

Forty-eight, every one in the ledger with its full reasoning. The cost column is each ruling's own
"costs if wrong" clause, compressed.

| # | scope | what it ruled | cost if wrong |
|---|---|---|---|
| R-P1 | global | commits carry the executing session's `Claude-Session` trailer | a trailer line |
| R-P1b | global | the resumed session's own trailer, not the prior one's | a trailer line |
| R-P2 | T0, T16 | `stat -f%z` for the help-picture byte total; macOS `du` has no `-b` | nothing |
| R-P3 | T8 | the account's heading renders an ASCII apostrophe, the spec's own text | one character |
| R-P4 | T9 | the playback cell's cube appears from `k >= 43`, and step 0's mover is the human | test constants |
| R-P5 | T9 | `landed` clears run **and** walk when the reply added no events | replay unavailable after a picker step |
| R-P6 | T9 | Task 9 also edits `NewGame.tsx`'s `onStarted` sites and the one expectation they break | nothing |
| R-P7 | T11 | `NewGame.test.tsx` is plan-mandated RED at Task 11's commit, restored at Task 12 | one red local commit |
| R-P8 | T4 | the pile cell runs the default bot at seat 1 on a server-drawn seed | a screen the script cannot answer throws |
| R-T0-1 | T0 | no separate M5 close file; the M5 spec's §8 **is** the close | one file the owner can ask for |
| R-T0-2 | T0 | the art-regeneration sentence spec §3.0 owes is added to `CLAUDE.md` | one sentence |
| R-T1-1 | T1 | the collapse candidate must be a group whose collapse **reduces** `units` | one condition and one cell |
| R-T2-1 | T2 | the third Pieces cell's comment is corrected and its assertions strengthened to the group↔index mapping | a test cell |
| R-T3-1 | T3 | **L = 0.415** — the eye-confirmed measurement, Laboratory binding | one JSON value, one constant, a re-shoot |
| R-T3-2 | T3 | the stale comment in `Pieces.test.tsx` is corrected in the same round | nothing |
| R-T4-1 | T4 | the pile cell gains a "Retreat President to hand" arm and a busy-wait | an e2e cell |
| R-T4-2 | T4 | the dead-message throw quotes `.decision:not(.notices)`; the dead eligibility guard becomes a real pin on the applied-move line | two lines of an e2e cell |
| R-GA-1 | gate A, **owner** | the tile popover mounts the same pieces band — Task 4b, inserted | one art box and two cells |
| R-T5-1 | T5 | the replay-offers cell must fail on a port copied from the live session | a route cell |
| R-T6-1 | T6 | the five unpinned diff grains get cells, each proven by dropping its field | test cells |
| R-T7-1 | T7 | the fake storage honours its key; the walk fixture pins the mover-**before** rule | test cells |
| R-T7-2 | T7 | three vacuous cells closed with the reviewer's own closers | test cells |
| R-T8-1 | T8 | folded account rows leave the tab order but stay in the accessibility tree | one attribute |
| R-T8-2 | T8 | the 9rem cap is dropped **only** above 1201px and kept below it | one media-query block |
| R-GC-1 | gate C, **owner** | the slot rule becomes arrival; a `techs` grain; every highlight pulses for the step's dwell — Task 9b, inserted | one rule, two components, one CSS block |
| R-T9-1 | T9 | cells for the two unasserted properties (which frame the table draws; the game switch clears the walk) | cells |
| R-T9b-1 | T9b | `data-beat` and `--dwell-ms` sit on `div.app`, not `main.table` — the side column is a sibling | an attribute's host element |
| R-GC-2 | gate C, **owner** | a `hand` grain: an order new to the viewer's hand marks and pulses | one grain and one class |
| R-T9b-2 | T9b | two false sentences (the Help's take rule; a claim that two lab boards deal the same ids) corrected | nothing |
| R-T11-1 | T11 | Task 11 carries the minimal `keeps` client change so no intermediate commit ships a broken client | two lines Task 12 rewrites |
| R-T11-2 | T11 | three cells run at a second seed where the argmax is card 2; server-side preview failures are 500 | test seeds and a status code |
| R-T12-1 | T12 | the set table asserts zero page overflow at 390/1024/1201/1440 | one e2e assertion |
| R-T12-2 | T12 | the Boards menu goes on the set table, extracted into one shared component | one component's placement |
| R-T12-3 | T12 | the deal cards drop the reward chips; the deck cap becomes 62vh above 1201px | one prop and one CSS value |
| R-T12-4 | T12 | one `useSections()` in App, passed into `NewGame` as props | a prop pair |
| R-F1 | final review | the held highlights are the **union** of the mover's steps, not the last step's | a fold over `run.steps` and one cell |
| R-F1a | final review | that union covers all **five** `Changes` sets, `hand` included | nothing (a wider union) |
| R-F2 | final review | the tile keeps its DOM node across the picker opening; `loading="lazy"` untouched | one element choice, and e2e selectors follow |
| R-F3 | final review | `.decision.walking` lays out as one line, pinned by measurement | one CSS rule |
| R-F4 | final review | the seat-row and strip-slot marks get mutation-proven unit cells | two cells |
| R-F5 | final review | the lit row is scrolled into view — spec §3.2.5 delivered | one effect and one cell |
| R-F6 | final review | the ink-edge cell is renamed to what it pins | a test name |
| R-F7 | final review, **LEAVE** | the human's own closing event stays step 1 of the run | a 400 ms flash read twice |
| R-F8 | final review, **LEAVE** | the opening-run walk stays unreachable from the wizard | nothing shipped changes |
| R-F9 | final review | the LEAVE list is routed: the R5 widening and the §4.1 slip to this close, three items to M6 | a record |
| R-F10 | fix wave | three residual false comments and one assertion operator are fixed as one commit the controller reads itself | an unreviewed comment |
| R-F11 | fix wave | the account scrolls **its own box** and never the page — R-F5's mechanism replaced | one effect rewritten twice |
| R-F12 | fix wave | the five R-F11 residual minors are parked for M6's entry | a folded account that unfolds at the list's end once |

Three of the forty-eight are the **owner's**, all three taken at a gate on a picture: R-GA-1,
R-GC-1 and R-GC-2. Two of them inserted tasks the plan did not have (4b and 9b).

---

## 3. Deviations — from the spec, and from the plan as written

### The plan's ten, decided while planning

1. **There is no Task 10** — the help paragraphs and gate C are inside Task 9, where the code they
   describe lands.
2. **The ink measurement gets a by-eye check.** The pale-ground test cannot tell illustration from
   icon ink on its own, so `tilerects.py --ink` counts only ink with a pale run to its right AND the
   tool draws candidate lines for the implementer to check on every art; the eye wins a
   disagreement (spec §3.1.2 named only the measurement).
3. **The collapse rule stops as soon as the floor holds**, one group at a time, largest first — the
   spec's own sentence read strictly; the mock's V3 column collapsed all four colours of the 4P
   crowd at once, so the plan's cells expect ONE collapse there.
4. **M5's close writes §8 into the M5 spec** rather than the separate close file spec §3.0 named;
   one document, no duplicate. (Ratified as ruling R-T0-1.)
5. **Gate A re-runs the V1 flow** rather than adding a flow; the pile cell takes its own picture.
6. **`readScreen` skips a running walk** — a harness behaviour the spec did not name, needed so
   every existing browser cell stays fast; the playback cell observes before it reads.
7. **The e2e `startGame` chooses the first card for every human seat** — spec §4.3 did not list the
   harness change the set table forces.
8. **The account shows rows for a run of the player's own events** (the walk does not play); spec
   §3.2.5's "when there is no run" is read as "when the reply added no event".
9. **A created game's opening bot run walks; a resumed game's does not** — `onStarted(state,
   opening)` carries the distinction the spec's §3.2.2 last bullet implies.
10. **The final review is one clean subagent against the live site** (owner, 2026-09-04) followed
    by a scoped re-review of the fix wave — the M5 shape with the live half added.

### Added in execution

11. **The pile cell runs a bot on a server-drawn seed** (R-P8), where spec §4.3 said "a fixed seed,
    no bot". Determinism is by construction of the script — Option B twice into one area, the
    President retreated recovering nothing, every other prompt answered by a fixed rule — and no
    bot turn can remove another seat's cubes.
12. **The collapse candidate must reduce the row** (R-T1-1). Spec §3.1.1's "the LARGEST group
    collapses" is read with the floor as its purpose; without the filter the rule can widen the row
    it is trying to narrow. Measured, and pinned by a cell the plan's code fails.
13. **L = 0.415, not the ≈0.35 the spec expected** (R-T3-1). The spec's own method produced it once
    the eye check ran; the band is wider than the mock's 0.30 and narrower than the naive maximum.
14. **Task 4b exists** (R-GA-1, owner). The plan has no such task; the popover's band is an owner
    addition made at gate A.
15. **The slot rule is arrival, and the highlights pulse** (R-GC-1, owner). Spec §3.2.4's per-slot
    `card` rule lit every slot a compaction shifted; its "outline pulse" had no animation in the
    plan's CSS at all.
16. **A fifth grain, `hand`** (R-GC-2, owner). Spec §3.2.4 named three grains; the shipped
    `Changes` has five (tiles, seats, slots, techs, hand).
17. **The 9rem cap is kept below 1201px** (R-T8-2). Spec §3.2.5 says "the 9rem cap goes"
    unqualified; below 1201px the side column is not the scroller, so dropping it grows the page
    stack without bound — a breach of the milestone's own M5-R8 constraint.
18. **`data-beat` and `--dwell-ms` live on `div.app`** (R-T9b-1), not the table the plan named: the
    seat rows and technology chips are in a sibling column, which a table-scoped custom property
    cannot reach.
19. **Task 11 carries the minimal client `keeps` change** (R-T11-1), so no intermediate commit
    ships a client every preview and create would 400.
20. **The set table asserts zero page overflow** at four widths (R-T12-1); spec §4.3's bullet had
    no cell in the plan.
21. **The Boards menu is a shared component on the set table** (R-T12-2) — spec §3.3.3 required it
    and the plan's banner JSX omitted it.
22. **The deal cards drop the reward-chip rows and the deck's cap is 62vh above 1201px**
    (R-T12-3), which is what makes spec §3.3.3's "both scores and the mark" true at 1710×869.
23. **One `useSections()`, hoisted to App** (R-T12-4).
24. **Spec §3.2.4's "the last step's highlights" is read as the turn's** (R-F1, R-F1a): the union
    of `changes` over the run's steps whose mover is the last step's. The literal reading delivers
    nothing on screen, because every bot turn ends with an event that changes nothing.
25. **The tile is one `<div role="button">` in both states** (R-F2). Inherited from before this
    milestone's range and fixed inside it, because the set table and the walk deliver the player to
    exactly that click.
26. **Spec §3.2.5's "the rows scroll to keep it in view" is delivered inside the account's own
    box** (R-F5, then R-F11). `scrollIntoView` was the first mechanism and it scrolled the *window*
    in the below-1201 stack; the shipped effect writes `scrollTop` on the element carrying the cap
    and does nothing when that element is not a scroller. **Above 1201px, where the cap is
    released, nothing scrolls the lit row into view** — recorded in §8.
27. **The human's own closing event walks as step 1** (R-F7, LEAVE), so a bot walk opens with a
    row in the player's own colour. Spec §3.2.2 walks the whole run; at 400 ms it is a flash.
28. **The opening-run walk is unreachable from the wizard** (R-F8, LEAVE): the bot-seat select
    excludes seat 0 and `start_seat` is fixed at 0, so spec §3.2.2's last bullet describes an
    API-only shape. The flag is pinned; the walk is not reachable from the screen.
29. **Spec §4.1's "no other file moves"** is a slip (R-F9): `events` lands in every `ClientState`
    fixture, which the plan expected and the corpus diff shows.

---

## 4. The final review, its findings, and what was done with them

**Task 14 was one clean-context Fable subagent** with the plan's brief verbatim, reviewing
`652badc..f82a06f` against the **live site** at `https://yokohama.moore.nyc` — playing it, not only
reading it. Its report is the record; the verdict was **READY WITH FIXES: 0 Critical, 2 Important,
6 Minor**, on a green gate. The mutation battery was 28 mutations, **24 caught**. Of the four
survivors, one is the diff module's own documented measurement (dropping `techs` from the "did
anything change" test survives because a technology change implies a seat change), one is the ink
constant that only an eye can pin (m6), and **two are the same gap** — the seat-row and strip-slot
marks (m3). The review's own summary tallies them as "two documented, one by design, one a real
gap", which its own mutation table does not quite bear out; the table is the record.

| finding | ruling | outcome |
|---|---|---|
| **I1** — the highlights the spec promises never outlive a real bot turn: `App` held only the last step's `changes`, and every bot turn ends with an empty-diff event, so the marks vanished the instant the walk ended (read live three times: `.tile.changed, .seat.changed, .strip-slot.changed` → 0) | R-F1, R-F1a | FIXED — the union of the mover's steps over all five sets; verified live at `0.5.0-m5.5.1`, where the same read gave tiles [A08], seats 1, strip-slots 1, four marks |
| **I2** — the board went dark for two to four seconds when the placement picker opened: the tile rendered as `<button>` only when pickable, so every Option A/B click remounted ten tiles and thirty lazy images (0 of 30 `complete` at ~1 s, 30 of 30 at ~4 s) | R-F2 | FIXED — one `<div role="button">` in both states; verified live, all 30 images and all 10 tiles identical (`===`) across the click |
| **m1** — the walk strip was a three-line block, 123 px tall, not the one-line strip | R-F3 | FIXED — verified live: computed `flex-direction: row`, 57 px |
| **m2** — the strip announces the human's own answer as step 1 | R-F7 | LEAVE, recorded |
| **m3** — the seat-row and strip-slot highlights were pinned by no unit cell (mutations M19 and M20 each left 367 of 367 green) | R-F4 | FIXED — two mutation-proven cells |
| **m4** — the account's lit row was not scrolled into view | R-F5, then R-F11 | FIXED, then re-fixed |
| **m5** — the opening-run walk is unreachable from the screen | R-F8 | LEAVE, recorded |
| **m6** — `ICON_INK_RIGHT` is pinned only by itself (mutation M3, `0.395 → 0`, left the cell green) | R-F6 | Cell renamed to what it pins, with R-T3-1 cited; the constant is the eye's reading and now says so |

**The fix wave was eight commits** (`f82a06f..fb92c17`): the six items, then one comment-truth
commit, then the R-F11 scroll fix. Both scoped re-reviews returned **addressed on every clause with
no Critical or Important breakage**.

**The wave's own regression is the finding worth carrying.** R-F5 was implemented as
`scrollIntoView({block:"nearest"})`, which is correct above 1201px and wrong below it: measured on
the live `0.5.0-m5.5.1` at a 1000 px viewport, a walk started from the board at scroll 0 yanked the
*page* to 1826, then 1851, then 1877 px, putting the lit row under the sticky walk strip and
scrolling the table the walk was highlighting out of view. Neither the code review nor the desktop
layout could see it. R-F11 replaced the mechanism with a `scrollTop` write on the element carrying
the cap, guarded by `scrollHeight > clientHeight`; verified live on `0.5.0-m5.5.2` at the same
viewport with `window.scrollY` 0 at every sample while the account's own 144 px box scrolled 0 → 38
and kept the lit row inside itself.

**Deferred minors.** The ledger records deferred minors at forty-four points across Tasks 0–12
(several of those entries carry more than one), plus the two LEAVE rulings above and the five the
R-F11 re-review found, parked by R-F12: a folded account is scrolled while clipped and
self-corrects at the next step; `box.offsetTop` and the box border are untested arithmetic; the
new e2e cell's pause is best-effort; that cell's admission depends on the server-drawn seed; and
one assertion in a carried cell cannot fail. None changes what a player sees in an unfolded
account. The whole list is the ledger's, and it is M6's inheritance.

---

## 5. The counts, at the close beside the open

Re-measured at `cc01dab`, not copied. `npm test` was re-run at this HEAD; the e2e and cargo figures
are the measurement taken at `fb92c17`, reused because the only commit since is
`deploy/RUNBOOK.md` (verified: `git diff --name-only fb92c17..HEAD` names that file alone).

| reading | at the open (`652badc` / `b1ccaa4`) | at the close (`cc01dab`) |
|---|---|---|
| `npm test` | **296** in **19** files | **381** in **23** files |
| `npm run e2e` | **32** in 5 spec files (+2 opt-in generators) | **36** in 6 spec files (+2 opt-in generators) |
| `cargo test -p yokohama-web` | **72** (66 lib + 1 gate + 5 `wire_fixtures`) | **80** (74 lib + 1 `play_a_whole_game` + 5 `wire_fixtures`) |
| wire corpus | **43** files | **44** files (`preview_bot.json` is the addition) |
| help pictures | **5**, 4,199,746 B | **5**, 4,210,470 B (all five re-shot at Task 12) |
| card art | **132** files, 7,552 KiB | **132** files, 7,552 KiB (unmoved) |
| `SEMANTICS_REV` | **7** | **7** (`semantics.rs:180`) |

Five component test files were added — `piecesLayout`, `diffFrames`, `walk`, `playback`,
`TurnAccount` — and `LastMove.test.tsx` was removed with the component it tested. One e2e spec file
was added, `the_playback.spec.ts`.

`cd web && npm run typecheck` and `cargo clippy --workspace --all-targets -- -D warnings` were both
re-run at `cc01dab` and are clean.

---

## 6. Blast radius

Spec §4.5, run at `cc01dab`:

- `git diff e4f41e4..HEAD -- crates/yokohama-core crates/yokohama-cli crates/yokohama-oracle
  crates/yokohama-difftest crates/yokohama-bridge` → **EMPTY** (0 bytes).
- `git diff master..HEAD -- crates/yokohama-core` → **EMPTY**.
- `SEMANTICS_REV` = **7**.
- `git diff --stat e4f41e4..HEAD -- crates/` names **six files, all in `yokohama-web`**:
  `src/client.rs` (+18), `src/explain.rs` (2 lines), `src/games.rs` (422), `src/lib.rs` (241),
  `tests/play_a_whole_game.rs` (2 lines), `tests/wire_fixtures.rs` (+23). Six, not four: the
  milestone's own expectation named `client`, `games`, `lib` and `wire_fixtures`, and `explain.rs`
  and `play_a_whole_game.rs` are two-line touches carried by the `keeps` type change. Recorded
  rather than glossed.

Everything else that moved is `web/**`, `docs/**`, `deploy/**`, `CLAUDE.md`, `README.md` and
`guidingpath.md`, as the constraint requires. **The fifteen-gate roster is untouched, nothing added
is `#[ignore]`d, and this lane still has no gate and no roster place.**

---

## 7. The handover into M6

Verbatim from spec §6.3:

> The close doc of M5.5 is M6's entry artifact and carries, in this order:
>
> - **The walk as the spectator primitive.** A spectator's client feeds `[prev, next)` from
>   a stream of events instead of a reply; every component of §3.2 is reused as is.
> - **M4's hosting list**, inherited unchanged (`m4-spec.md:507-524`): Access JWT
>   verification does not exist and is not free; `GET /api/games` enumerates every game to
>   any caller; the Worker shell (`deploy/worker/src/index.ts`) already forwards the Access
>   identity as `X-Yokohama-User` and strips a client-supplied one.
> - **M6's first three questions**, with the lean stated in conversation on 2026-09-04:
>   the transport (server-sent events over the axum server, since both hosted shapes stream
>   and one process serves three players); identity (verify the Access assertion at the
>   origin, or let the Worker be the only door); the store (Neon Postgres only if the
>   Worker takes a share of the state, else files behind the existing `SessionStore`).
> - **M5's LEAVE list** items M6 inherits (handover §6): the unbounded saved-game list, the
>   "New game" in-flight guard, the popover's browser cell.

**This milestone's own LEAVE list**, added to that inheritance (ruling R-F9 routed the first three;
the rest are the ledger's deferred entries at their most visible):

- The **Game menu stays open** after a game is chosen from it (M5's component, untouched here).
- The **tile popover at 390 px** covers most of the screen (M5 behaviour).
- **`POST /api/games/preview` builds a full session per call, unauthenticated at the origin.** The
  route predates this milestone; what changed is its cost. It used to call `new_game` and read the
  deals off the `PendingSetup`; it now runs `Session::new_sim` and `HistoricalFrame::at` for a whole
  throwaway position on every call. Same place on M4's hosting list, more work per anonymous
  request.
- **Above 1201px nothing scrolls the account's lit row into view.** R-F11's mechanism is guarded by
  `scrollHeight > clientHeight`, and above 1201px the cap is released and the side column is the
  scroller — so spec §3.2.5's sentence is delivered below 1201px and not above it.
- The **human's own closing event walks as step 1** (R-F7) and the **opening-run walk is
  unreachable from the wizard** (R-F8).
- The set table's **Keep hedge is an unpinned copy** of the CLI's `KEEP_SCORE_NOTE`: a reword in
  `wizard.rs` drifts the browser silently. Pinning it wants a `pub` and a wire field, which is
  outside this milestone's blast radius — the shape `site_space_words.json` exists for.
- **`useSections()` is a footgun by shape**: a third caller repeats the defect R-T12-4 fixed, and
  only a comment and a cell guard it. A context provider would close it.
- The **whole-game server gate is OS-entropy seeded and capped twice** in this milestone's runs
  (its 20,000-input cap hit at turn 7,295 with the bot hoarding ¥7,273), then passed 660
  consecutive times. Pre-existing, escalated here for both the web lane and the AI lane: a 2P game
  the bot cannot end.

---

## 8. What is NOT claimed

Spec §5, restated at the close with the changes this run made to it marked:

- **No engine and no CLI change.** The CLI's setup shows scores and no port (R3), and the engine's
  draw order stays deal-first (R2). Holds: the blast radius is empty across the five fenced crates
  and `SEMANTICS_REV` is 7.
- **No legibility claim at the 4P worst case.** Four pawns, twelve cubes and the Station collapse
  under every rule; the mock read 7.5 px at the 4P fit. Recorded, not gated.
- **The pile cube's height is now a measurement, not a mock reading** — spec §4.3 read it, which is
  what this bullet was waiting for: **11.2 px** in the e2e cell at 1710×869 on this Mac, and
  **9.9 px** on the deployed client at the final review's own viewport. Both are recorded, neither
  is gated, and the §0.2 figures remain a throwaway page's.
- **No walk after a reload or a game switch.** The account starts with the first reply of the
  session; the M4 log drawer covers the past. Verified live.
- **Hot-seat secrecy is not improved.** The set table shows every human seat's deal on one screen,
  as the setup screen does today.
- **Below 1201px** the column stack is M5's, unchanged (M5 spec R8). Two rulings were taken to keep
  it that way — R-T8-2 kept the account's 9rem cap there, and R-F11 replaced a fix that had begun
  scrolling the page in that stack.
- **No identity, lobby, chat, spectating or hosting change.** M6 (§7 above).
- **The keep valuation is not touched** (R5): its weights are `greedy.rs:469-494` as shipped, and
  the web quotes the CLI's hedge beside every score. **What R5 widens, recorded here rather than
  fixed:** the preview shows the bot's two dealt cards and which one it keeps, so the bot's
  starting order is known to the human for the whole game, and the in-play masking of the bot's
  hand no longer hides it. That is the owner's ruling and its consequence, recorded here rather
  than treated as a defect.
- **The account's lit row is kept in view below 1201px only** (new at the close; §7's LEAVE list).
- **No claim that a walk's step count is stable** (new at the close). It is a property of the game
  the server seeds; 8 and 9 were both observed, and the cell prints its own reading rather than
  asserting one.
