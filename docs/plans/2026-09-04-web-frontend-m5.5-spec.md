# Web frontend — M5.5 "the table reads": spec

**Written 2026-09-04 on branch `web-frontend` at `e4f41e4` (the M5 execution handover's
commit), from the owner's first two rounds of play on the deployed M5 client and the
rulings taken in conversation the same day (§1).** M5's spec
(`docs/plans/2026-09-01-web-frontend-m5-spec.md`) stays the authority for the table this
milestone changes; M5's execution handover
(`docs/plans/2026-09-02-web-frontend-m5-execution-handover.md`) is the record this
milestone closes (§3.0). Nothing here re-opens an M4 or M5 ruling; where one is touched,
the section says which and why.

Three findings from play, one owed close, and a handover into M6. The name is what the
three findings share: a table that reads at a glance — the pieces on it, what the
opponent just did on it, and what was on it when you chose your first card.

---

## 0. The problem, from play

### 0.1 The three findings

**F1 — a crowd of cubes is unreadable.** Four or more assistants of one colour on a tile
draw as ONE cube and a count (`web/src/Pieces.tsx:57`, `CUBES_DRAWN = 3`), whatever room
the band has. The owner's screenshot is a 16px cube beside a "4" that has to be read
rather than seen. The band is `web/src/tile_rects.json:10` — x 0.50 to 0.97 under the
site card, 0.123 of the tile tall — and its glyphs already shrink to fit
(`web/src/styles.css:400-431`); what does not scale is the cap.

**F2 — the first card is chosen blind.** The rulebook, p.4 step 12, verbatim: *"From this
deck, draw and place Order cards face up in each space of Port A (and B) Order Form Board
until all spaces on both Port A (and B) Order Form Board are filled. Each player now draws
two Order cards from the remaining deck. In secret, each player must choose 1 Order card
to keep and return the other card to the box."* The port is on the table before anyone
chooses, and so are the goals (step 10), the lab (step 11), the areas, their site cards
and their tokens (steps 2–5). The client's setup screen (`web/src/NewGame.tsx`) shows the
two dealt cards and nothing else, and the CLI's wizard shares the gap — deferred by the
owner (§1, R3).

**F3 — the opponent's turn is a paragraph in a small box.** After a bot's turn the reply
carries the CLI's applied-move lines as prose, and the side column prints them in a box
capped at 9rem with a scrollbar (`web/src/styles.css:1168`, `web/src/LastMove.tsx`). A
turn of eight events is read after the fact, out of order with what changed on the table,
and nothing on the table moves.

### 0.2 What was measured before ruling

**The card draw, in the engine and the web path.** A throwaway test (deleted, never
committed) ran `new_game` then `finalize_setup` at every player count and counted every
pile. The take-two-keep-one is modelled exactly as step 12 reads, the un-kept card goes to
the box (`orders_removed`) and never back into the deck, and the setup removal is IN
ADDITION to the discards:

| Players | Boxed at step 12 | Port fill | Dealt | Kept | Discarded to box | Deck after setup | Accounted |
|---|---|---|---|---|---|---|---|
| 2 | 16 | 6 (Port A) | 4 | 2 | 2 | 10 | 36 |
| 3 | 5 | 12 (A and B) | 6 | 3 | 3 | 13 | 36 |
| 4 | 0 | 12 (A and B) | 8 | 4 | 4 | 16 | 36 |

The removal counts are `order_cards_to_remove` (`crates/yokohama-core/src/types/state.rs:87-94`),
Port B at three players is `has_port_b: player_count >= 3` (`state.rs:32`) and matches
p.3 step 1, and the 36-card conservation is checked on every applied move
(`crates/yokohama-core/src/game/invariants.rs:89-107`). The web goes through the identical
path: `Games::create` calls `Session::new_sim`, which is `new_game` then
`finalize_setup` with the client's keeps (`crates/yokohama-cli/src/session.rs:129-149`).

**The one difference is sequence, and it is cosmetic.** The engine boxes, deals, then
fills the port (`crates/yokohama-core/src/game/setup.rs:96-104`, then `:151-154`); the
rulebook boxes, fills, then deals. Under a shuffled deck the two give the same distribution
over what lands where. Matching the rulebook's order would change only which cards a GIVEN
seed puts in the port versus the deals — every seeded artifact (records, fixtures, the wire
corpus, the owner's games) would move, and that is a `SEMANTICS_REV` bump for no rules
fidelity. **Not done** (§1, R2); recorded forward in guidingpath (§6.1).

**The pending setup already holds the set table.** `PendingSetup.state` carries the
filled port, lab, goals, board, sites and tokens BEFORE any keep is applied
(`setup.rs:151-166`), so F2 is a visibility fix with no engine change.

**The bots' Keep, traced.** One valuation, `greedy::score_starting_order`
(`crates/yokohama-core/src/game/greedy.rs:520-532`), a pure function of the card's printed
economics with hand-set weights (`greedy.rs:469-494`: VP 10, imported good 6, warehouse
token 5, trade good and movement 4, money 3, country scarcity 120 ÷ icons of that country
across orders and techs, requirements −8 per good); its own doc: *"UNMEASURED and hand-set,
spec §8's own terms — revisited at 3.5."* Every evaluator contestant keeps by its argmax
over the two dealt cards, ties to card 1 (`greedy.rs:733`, `lookahead.rs:1893`,
`mcts.rs:2735`; every named table is `KeepKind::EvaluatorScored`,
`crates/yokohama-core/src/game/arena/stats.rs:869`), the rungs keep by a seeded coin, and
the arena's from-setup driver asks the agent — so **every V4 figure was recorded with the
bot keeping by this argmax** (`crates/yokohama-core/src/game/arena.rs:63-70` on why the rungs
were never moved onto it). The CLI wizard prints the score beside each card and asks the
OPERATOR to choose, bot seats included (`crates/yokohama-cli/src/wizard.rs:1122-1137`,
`:1353-1369`, `:1384-1385`); the web shows no score and lets the human click either of the
bot's cards, defaulting to card 1.

**The cube band, mocked.** A throwaway page (not in the repository) reproduced the tile
DOM of `web/src/Board.tsx` over the real `styles.css`, on Church (icon block reaching
lowest) and Tea Plantation (open ground), and was shot in Playwright Chromium at device
scale factor 2 on this Mac. Four rules × five crowds × three tile sizes; the readings that
decided the ruling, cube height in CSS px at the 2P fit (15.31rem; the 4P fit is ×0.79):

| Crowd | V0 today | V1 room-aware cap, same band | V2 band widened to x 0.30 | V3 = V2 + a pile for 4+ |
|---|---|---|---|---|
| pawn + 4 blue | 16.1, "cube + 4" | 16.1, all drawn | 16.1, all drawn | 11.2 in a 2×2 pile |
| 2 pawns + 4 red + 3 blue | 12.4, red collapsed | 9.6, all drawn | 13.7, all drawn | 11.2 pile + 16.1 singles |
| pawn + 6 blue + 3 red | 14.4, blue collapsed | 14.4, blue collapsed | 12.3, all drawn | 11.2 pile + 16.1 singles |
| 4 pawns + 3×4 cubes + Station (17 glyphs) | 5.1 | 6.7, all collapsed | 9.5, all collapsed | 9.5, all collapsed |

Three facts fell out. The band's width-to-height ratio is fixed by the rect, so a floor
on glyph height is a pure glyph-count rule: the 0.47-wide band holds nine glyphs, the
0.67-wide one thirteen, at every tile size. V2 crosses x 0.50 on every crowded tile by
≈0.08 of the tile, entering the icon-block RECT (`web/src/tileRects.ts:11-13`) but not the
printed INK, which on Church ends near x 0.33. Nothing rescues the 17-glyph 4P crowd: every
rule collapses all four colours, and V2/V3 still lift it from 5px to 9.5px.

---

## 1. The rulings (owner, 2026-09-04, in conversation over the readings above)

- **R1 — the split.** M5.5 is one milestone of three parts — A the pieces band, B the set
  table, C the turn playback — plus M5's owed close as Task 0 (§3.0). M6, the multi-game
  layer, is its own spec after this one, and C's walk is what M6's spectator inherits.
- **R2 — no `SEMANTICS_REV` change.** The owner is playtesting and tuning V4; a re-order
  of the setup draws to match the rulebook's sequence is unnecessary and, if ever, M7 at
  the earliest. Recorded in guidingpath as an M5.5 wrap-up row (§6.1).
- **R3 — the CLI's setup visibility gap stays deferred.** F2 is fixed in the web alone.
- **R4 — the set table (B, shape 1).** After "Deal" the setup screen IS the table at step
  12 — the pyramid with its site cards and tokens, the strips — with the keep asked beside
  it. The owner's words: *"while you don't see the tech or achievement cards yet you do see
  where all the building cards are on the areas so seeing the areas on the board is the way
  to go."* Per steps 10–11 the goals and the lab ARE on the table at step 12, so the set
  table shows them too; nothing on it is hidden that the rulebook has face up.
- **R5 — the bot's card is exposed, and the bot chooses it.** The bot's two cards stay
  visible on the set table (the bot AI "hasn't been coloured in" for the choice), and the
  server keeps for every bot seat by the same argmax the arena used — *"it is better for
  now than the user keeping it and it makes for easier apples to apples comparisons
  against bot play with human players."* The owner's correction to the valuation — it
  prices every required good at 8 while copper and silk are much harder to get than tea
  and seafood — is a V5 note (§6.2), not an M5.5 change.
- **R6 — the pieces band is V3, and wider.** The pile for four or more, the room-aware
  cap, and the band pushed further left than the mock's 0.30 *"even if it intrudes into the
  data section"* — the criterion is printed INK, not the rect; the measurement in §3.1.2.
- **R7 — the playback is automatic.** Auto-play when the reply lands, with skip; no
  follow-up click, since the bot's moves land almost at once and *"waiting for a click to
  appear also seems unnecessary."* A replay-again button is the one extra control — *"I
  don't want to clutter the UI too much."* The defaults in §3.2 were agreed as stated.
- **R8 — order of build A → C → B** (§3.4), and **Task 0 first**.
- **R9 — the verdict on M5's pictures.** The owner's two rounds of play are the V2/V3
  verdict M5's close was waiting for: pictures accepted; the pieces band's legibility
  carried to A; the log box's shape carried to C.

---

## 2. What was verified against the code, beyond §0.2

- `HistoricalFrame` (`web/src/types.ts:358-377`) is deliberately NOT a `ClientState`: it
  drops the fields that would be present-tense lies at a past position (`your_turn`,
  `prompt`, `warnings`, `messages`). M4's Q5 (`docs/plans/2026-09-01-web-frontend-m4-spec.md:45`)
  ruled it narrow for THAT reason; the port and lab offers at k were face up at the time
  and are properties of the position, so widening the frame with them crosses no M4
  ruling. `TableSections.tsx:10-13` collapses the fetched strips while replaying only
  because the server does not serve them at k.
- The mover of event k is `frame(k).to_move` and the phase it was in is
  `frame(k).decision.phase_chain[current_phase]` (`types.ts:263-270`) — both on the
  position BEFORE the event, which the replay route already serves. A client that has the
  frames needs no typed seat on the log line and never parses "Blue:" out of prose.
- The `input` handler already reads the record's length before `step`
  (`crates/yokohama-web/src/lib.rs`, `let before = repl.session.record().events.len()`),
  so the event count reaches the payload from a number the handler holds.
- `HistoricalFrame::at(&repl, k)` (`crates/yokohama-web/src/client.rs:499`) builds a
  position from a `Repl` and a record with `state_at`; at `k == 0` it is the position after
  setup with NO pump. A throwaway `Repl` over `Session::new_sim` is therefore a preview
  of the set table with nothing persisted and no bot moved.
- `Session::new_sim` accepts `StartingOrderChoice::Unknown` (the live-mode fiction,
  `setup.rs:231-262`): a human seat previewed as `Unknown` shows no card in hand, which is
  what a seat that has not chosen yet should show.
- The replay viewer is pinned to the lowest human seat (`client.rs:260-266`), which is the
  human's own seat in every 2P game and the first human's in hot-seat 3–4P — the same seat
  the live payload builds for in those games.
- The M4 prep's replay cost slope, 1.50 µs per event on that Mac at `513284d`, means a walk
  of fifteen positions over a 300-event record replays ~4,500 events in single-digit
  milliseconds of engine time; the walk's cost is HTTP round trips, prefetched in parallel.

---

## 3. The design

### 3.0 Task 0 — M5's close, with R9 as its verdict

Exactly the list M5's execution handover §7 owes, run first, on the branch, at the then-HEAD:

1. Spec §8's close items of the M5 spec, in the spec's order; the guidingpath decision-log
   row (append-only); the `docs/plans/` close record, `2026-09-0x-web-frontend-m5-close.md`.
2. `CLAUDE.md`'s browser-half counts re-measured (the handover's §5 read 296 vitest in 19
   files, 32 e2e in 5 spec files plus the two opt-in generators, 72 server tests, five help
   pictures at 4,199,746 bytes, 132 art files) with the M5 paragraph beside the M4 one and
   the art regeneration sentence (source-width crops, sites rotated).
3. `README.md`'s browser-client section re-checked against the shipped pictures.
4. The blast-radius check verbatim from the M5 plan: `git diff master..HEAD --
   crates/yokohama-core` EMPTY, `SEMANTICS_REV` 7, `git diff --stat 054e762..HEAD -- crates/`
   only `assets/art/**` and the two rows of `art.rs`.
5. The spec's §3.3 slip (15.4 vs 15.31 rem) recorded as a correction forward.
6. The verdict recorded in R9's words. The SDD workspace of the M5 plan deleted after.

Task 0 ships no code. The memory update it owes is the last line of this milestone's close
instead (§6.4), so one update covers both.

### 3.1 A — the pieces band

#### 3.1.1 The rule

`Pieces.tsx` gains one pure function, `layoutPieces(crowd, band) → Layout`, and the CSS
reads its result instead of a glyph COUNT. Every horizontal cost is a multiple of the glyph
height `h`, as the existing proof requires (`styles.css:400-417`):

| Element | Width in units of h |
|---|---|
| pawn (20:28 viewBox at height h) | 0.71 |
| single cube (0.72 h tall, 1:1) | 0.72 |
| pile column (two cubes of 0.50 h, 1:1) | 0.50 |
| Station (0.8 h, square) | 0.80 |
| gap between groups / within a group | 0.10 / 0.05 |

- A colour with **four or more** cubes is a **pile**: two rows, ⌈n/2⌉ columns, the odd cube
  on the top row, every cube 0.50 h. Four is 2+2, five 3+2, six 3+3, eight 4+4.
- `units` = Σ element widths + gaps. `h = min(H, W / units)` where W and H are the band's
  width and height in CSS px — the CSS form is `--piece-h: min(100cqh, 100cqw /
  var(--piece-units))`, the existing rule with the count replaced by the sum.
- **The floor: h ≥ 0.55 H.** If the crowd misses it, the LARGEST group collapses to one
  cube plus its count (as today), `units` is recomputed, and the check repeats. A collapsed
  group costs 0.72 for the cube, 0.05 for the inner gap, and 0.36 per digit of the count
  (a 0.6 h font at about 0.6 em per digit).
- Pieces pack to the band's RIGHT edge (`justify-content: flex-end`): a small crowd looks
  exactly as it does today, and only a large one grows leftwards over the art.
- `piecesWords` and the accessible name are unchanged; a pile announces "4 blue
  assistants" as it always did.

The 0.55 was the mock's number and is a named constant beside `layoutPieces`, not a magic
value in CSS; the mock readings in §0.2 are what it was chosen against.

#### 3.1.2 The rect, measured before it is set (R6)

`tile_rects.json`'s `pieces` rect becomes `left = L, width = 0.97 − L`, same top and
height. **L is a measurement, not a choice**, by the method the M5 rects were measured with
(`docs/tools/tilerects.py`, M5 spec §4.1): for each of the thirteen area arts, the
rightmost pixel of printed INK inside the band's vertical range y ∈ [0.462, 0.585] and
x < 0.50, where ink is anything that fails the pale-ground test the site-card rect used
(V > 0.784, S < 0.18); L = max over the arts of that x, plus a 0.02 margin. The mock read
Church's ink ending near 0.33, so L is expected near 0.35; the plan records the reading
per art and the value it set, and `tilerects.py` draws the new band on the contact sheet
so the owner sees it on every art before the value lands.

The geometry test in `tileRects.test.ts` changes meaning with it: today the pieces rect
must be disjoint from the icon-block rect; after A it is ALLOWED inside the rect and must
stay right of the measured ink edge, which the test carries as the same constant the rect
was set from.

#### 3.1.3 What the CSS block loses and keeps

`.pieces`'s `--piece-h` divisor reads `--piece-units`; `.piece.cube` keeps 0.72 h; a new
`.pile` rule draws the two rows (a column flex of two rows, each a row flex of 0.50 h
cubes, no gaps inside the pile — cubes in a pile touch, as on a table); `.piece-n` is
unchanged. The `overflow: hidden` belt stays. The proof comment is rewritten for the sum.

### 3.2 C — the turn playback

#### 3.2.1 The wire, all additive

1. `ClientState.events: usize` — the record's length after the reply.
2. `HistoricalFrame.ports: PortsPanel` and `HistoricalFrame.labs: LabsPanel` — the offers
   at k, built by the same `screen_model` builders the live `/ports` and `/labs` routes
   call, over `state_at(k)`. At `k == total` they equal the live routes' payloads (pinned).

Nothing else moves. The log route and its panel are unchanged; the CLI crate is untouched.

#### 3.2.2 The run

The client remembers the `events` count of the state it answered from (`prev`) and reads
the reply's `events` (`next`).

- `next < prev` (undo) or `next == prev` (a picker step, a refusal): no walk; the box shows
  the reply's prose lines exactly as today.
- `next > prev`: the run is the events `[prev, next)`. The client fetches `/at/{k}` for
  every k in `prev ..= next` in parallel — `prev` included, because the live payload
  carries no port or lab and the first step's diff needs a full position on both sides —
  and `/log?tail=next−prev` for the lines. For event e: the line is the log entry at e;
  the mover is `frame(e).to_move`; the phase is `frame(e).decision`'s current phase; the
  position shown is `frame(e+1)`.
- **The walk plays only when the run contains at least one event whose mover is a bot
  seat** (`is_bot` on the frame's player rows, `types.ts:156`). A run of the human's own
  events applies instantly as today: a two-second dwell on every one of your own choices
  would be the wrong fix for the right complaint.
- While the walk runs, `App`'s `past` is the frame of the current step, so the table takes
  M4's `replaying` outline and its tiles are inert `<div>`s; the strips draw from
  `past.ports` / `past.labs` instead of collapsing (`TableSections.tsx:10-13` becomes
  false and is rewritten); the deck shows a one-line strip — "Blue is playing · skip" —
  in place of the live options, so nothing answerable sits under a past position. The
  strip is driven by the walk's own flag, not by `past` alone: the walk's last step is
  `frame(next)`, whose index equals its total, and M4's `atPresent` rule correctly drops
  the outline there while the step still dwells.
- When the walk ends, or on skip, `past` is cleared and the live reply is on screen: the
  deck shows the live prompt, the strips refetch as they do after every state change.
- A create whose start seat is a bot returns with that bot's opening run applied
  (`bring_alive`); `prev` is 0 for a fresh game, so the first thing a player sees is the
  bot's opening playing out, and then their own prompt.

#### 3.2.3 Cadence and dwell (R7's agreed defaults)

- Cadence c ∈ {1 s, 2 s, 3 s}, default 2 s, remembered per browser under
  `yokohama.playback.cadence` in the `sections.ts` idiom (`sections.ts:20`; a storage that
  throws reads as the default).
- A step whose frame is IDENTICAL to the previous one — the board's tiles, the Players
  panel, the port and lab slots all equal — dwells 0.4 s. That is what "End
  additional-action phase", "Skip the 5-Power token" and "Skip the building" read as; the
  test is equality of the wire objects, never a rules judgement.
- Controls, all in the turn account (§3.2.5): pause/play, step back, step forward, skip
  to live, replay (re-runs the last walk from its first step), and the cadence as a
  three-way control in the account's header. Nothing new in the banner or the deck beyond
  the "is playing · skip" strip.

#### 3.2.4 Highlights, by diffing frames

`diffFrames(a, b) → Changes` is a pure function over two `HistoricalFrame`s and knows no
rule:

- a tile is **changed** when its `presidents`, `assistants`, `has_station`, or its
  `site.slots` / `site.trading_house_owner` differ (`types.ts:71-100`);
- a player row is changed when its yen, VP, goods, warehouse counts or hand count differ;
- a port or lab slot is changed when its `card` differs.

A changed tile carries `.tile.changed` (an outline pulse in the mover's seat colour);
changed player values and slots carry the same class. The class stays until the next step
replaces it; the last step's highlights stay on the live table until the player's next
answer, so the end state of the turn is still readable after the walk.

#### 3.2.5 The turn account (the box, restated)

`LastMove` becomes `TurnAccount`, the same slot in the side column (M5 spec §3.5):

- **Header**: the mover's turn — "Blue's turn 4" in the seat colour — with the cadence
  control and the replay button; folded/unfolded as today.
- **Body**: one row per event of the run, in order: the phase chip (the CLI's own phase
  label from `labels.phases`), the mover's colour mark where the mover changes, the log's
  line. The current step is lit while the walk runs, and the rows scroll to keep it in
  view. A row is a button that jumps the walk to that step (the `log-jump` idiom,
  `styles.css:819-826`), so stepping is also a click.
- **When there is no run** (undo, picker steps, refusals, a cold load): the reply's prose
  lines, as today. The `role="status"` container stays mounted either way, for the reason
  `LastMove.tsx`'s comment gives.
- The 9rem cap goes; the account takes the height it needs inside the side column, which
  scrolls on its own above 1201px (`styles.css:200-205`).

The M4 log drawer and its scrubber are unchanged; the walk is a second driver of the same
`past` state, and the drawer's own past positions now show the strips too, a side benefit.

### 3.3 B — the set table

#### 3.3.1 The preview payload

`Preview` (`types.ts:409-412`) becomes:

```
{ seed: string,
  table: HistoricalFrame,          // index 0, total 0: the position after setup
  deals: [{ seat, is_bot, cards: [{ card: OrderCard, keep_score: number }, ×2],
            bot_keep: 1 | 2 | null }] }
```

Built server-side from a THROWAWAY `Repl` over `Session::new_sim(config, seed, choices,
rules)` with `set_run_seats` from the request's bot seats, where `choices` is
`StartingOrderChoice::Unknown` for every human seat and `Keep(argmax)` for every bot seat;
then `HistoricalFrame::at(&repl, 0)`. Not persisted, no id, no pump, dropped on return.
`keep_score` is `score_starting_order` per card, `bot_keep` its argmax with ties to card
1 — the function reached through core, never restated (`greedy.rs:520-532`, `:733`).
The seed rides as the string it does today (`games.rs`, `WireSeed`), for the reason
`NewGame.tsx`'s header comment gives.

#### 3.3.2 Create (R5)

`NewGame.keeps` (`games.rs:81-83`) becomes `Vec<Option<u8>>`, one entry per seat: `1` or
`2` for a human seat, `null` for a bot seat. A value on a bot seat is a 400 naming the seat
("seat 1 is a bot; its keep is the bot's"); `null` on a human seat is a 400 the same way.
`config_of` (`games.rs:118-170`) fills every bot seat with the argmax over that seat's
deal, which is deterministic from the seed and therefore the card the preview showed.
`create` is otherwise unchanged, and every record it writes has the shape it has today.

#### 3.3.3 The screen

The first step of setup is unchanged: players, seed, bot seat, "Deal", and the saved-games
list with its two-click delete. After "Deal", the screen is the M5 table over
`preview.table` — the pyramid, the strips the browser has enabled (Port, Laboratory,
Goals, Building sites; Tracks shows the 2P dummies and nothing else, honestly), the banner
reading "setting up · seed …" — with the side column holding the deals in place of the
Players and Hand panels:

- a human seat: its two cards drawn as cards, the Keep score under each in the CLI's own
  hedge (*"what the built-in bot would prefer, from the card's printed economics alone —
  hand-set, unmeasured, and no substitute for your own read"*, `wizard.rs:1384-1385`, carried
  verbatim), and a keep toggle;
- a bot seat: its two cards, both scores, and the kept one marked "bot keeps this";
- "Start" enables when every human seat has chosen; "Re-deal" returns to the first step.

The table is inert (no `onTileClick`), the tile popover works, and the Boards menu works,
so a player can read a site card at full size before choosing. Above 1201px the grid is
M5's; below it the column stack is M5's (R8 of the M5 spec stands: unchanged there).

### 3.4 Order of build, and why

**Task 0, then A, then C, then B.** A is one file's rule plus one measured rect and lands
first because it is the finding on the owner's screenshot. C reshapes how the strips take
their data — from a frame, when there is one, instead of a fetch — and B's set table is
exactly that path over `preview.table`, so C builds the road B drives on. B last also
means the widened `HistoricalFrame` and its fixture exist before the preview reuses the
type.

---

## 4. The work, with its guard

Every part is TDD on the branch, task-reviewed as M5 was (Opus implementers, Opus per-task
reviewers, Fable as orchestrator and final whole-branch review; no Sonnet or Haiku
dispatch — nothing here is a scripted chore). The per-edit commands are the four the
CLAUDE.md web lines name: `npm run typecheck`, `npm test`, `npm run e2e`, `cargo test -p
yokohama-web`, plus the clippy gate. The fifteen-gate roster is untouched: nothing here is
in core.

### 4.1 Server cells (`crates/yokohama-web`)

- `ClientState.events` equals the record's length; on a reply that applied a bot run it is
  greater than the count before by the run's length.
- `HistoricalFrame` at `k == total` carries `ports`/`labs` byte-equal to `/ports` and
  `/labs`; at `k == 0` the port holds six cards and the lab six; at the M4 fixture's
  mid-game k the port differs from the live one (anti-vacuity: the fixture is chosen so a
  take has happened between k and the end).
- The preview's `table` is the position after setup with every human hand empty and the
  bot's kept card in the bot's hand; `bot_keep` equals the argmax of `keep_score`, and the
  cell is MUTATION-PROVEN by inverting the comparison; a preview and a create at the same
  seed put the same card in the bot's hand.
- `keeps`: a value on a bot seat is a 400 naming the seat; `null` on a human seat is a 400;
  the well-formed request is a 201 (the `deny_unknown_fields` cell's shape).
- The wire corpus regenerates (`UPDATE_WIRE_FIXTURES=1 …`) and **its diff is read as a
  contract**: `at_k.json` gains two keys, `preview.json` changes shape, no other file moves.
  `no_committed_payload_carries_the_setup_seed_except_the_preview` keeps its exemption and
  the preview's `table` is asserted seed-free (the `TitleModel::seed: None` rule).

### 4.2 Component cells over the corpus (`web/src/*.test.tsx`)

- `layoutPieces`: the pile threshold, the unit sum, the floor, the largest-group-first
  collapse, and the mock's five crowds as table-driven cells whose expected collapses are
  the mock's column for V3 — mutation-proven by shifting the floor.
- `Pieces` renders a pile as two rows with the odd cube on top; the accessible name is
  unchanged for a pile and for a collapse.
- `tileRects.test.ts`: the band's left edge is at or right of the measured ink constant
  and the rect is inside the tile; the disjointness cell against the icon block is
  RETIRED with a comment naming this spec.
- `diffFrames`: two corpus frames that differ in one tile's assistants yield exactly that
  tile; identical frames yield nothing; a port slot change is seen — each mutation-proven
  by editing the fixture copy in the test.
- The walk, under fake timers: plays every event of a run in order, dwells the cadence on
  a changed frame and 0.4 s on an unchanged one, does NOT start for a run with no bot
  event, and skip lands `past` on `null` with the live prompt on screen; the cadence
  survives a re-render through the storage seam and reads the default when storage throws.
- `TurnAccount`: one row per event with the mover's colour and the phase label; the lit
  row follows the step; prose when there is no run; the region stays mounted.
- `NewGame` over `preview.json`: the human's toggle, the bot's marked card, Start disabled
  until every human seat has chosen, and the request it sends (`null` on the bot seat).

### 4.3 Browser cells (`web/e2e/`)

- **A pile exists deterministically.** A 2P game at a fixed seed, no bot: the cell places
  two assistants in one area with Option B and leaves the President in hand (a President
  that acts in that area recovers its cubes at the Recovery step), the second seat passes,
  and the first seat places two more in the same area. Four cubes render as a pile; the
  cell reads the cube's height at 1710×869 and RECORDS it, asserting only that four cube
  elements exist and no count is printed.
- **A bot turn plays out.** A 2P game with a bot: after the human's answer the cell counts
  one `/at/{k}` request per event of the run, in ascending order; sees `.table.replaying`
  during the walk and the "is playing · skip" strip in the deck; presses skip and answers
  the live deck. A second run with the cadence set to 1 s finishes within its budget.
- **The set table.** After "Deal": six port cards in the Port strip, a site card on every
  non-Canal tile, the bot's kept card marked; choosing a card enables Start; the game that
  starts shows the chosen card in the human's hand.
- The width gate (`the_screen_itself.spec.ts`) re-runs as is; the set table at 390/1024/
  1201/1440 must not widen the page.

### 4.4 Visual gates — pictures delivered, no stop (the M5 precedent, R-GATES)

At each part's close one contact sheet for the owner: A — the new band on a real game at
the 2P and 4P fits, plus the tilerects sheet with the measured L on every art; C — the
frames of one bot turn as the walk showed them, with the account beside; B — the set table
at 1710×869. Generated by the opt-in shot generator, sent, recorded in the close.

### 4.5 Blast radius

`git diff e4f41e4..HEAD -- crates/yokohama-core crates/yokohama-cli crates/yokohama-oracle
crates/yokohama-difftest crates/yokohama-bridge` EMPTY at the close — this milestone touches
`crates/yokohama-web` and `web/` only. `SEMANTICS_REV` 7.

---

## 5. What is NOT claimed

- **No engine and no CLI change.** The CLI's setup shows scores and no port (R3), and
  the engine's draw order stays deal-first (R2).
- **No legibility claim at the 4P worst case.** Four pawns, twelve cubes and the Station
  collapse under every rule; the mock read 7.5px at the 4P fit. Recorded, not gated.
- **The pile cube's height is a mock reading until §4.3 reads it.** Every px figure in
  §0.2 is from a throwaway page on one machine at device scale factor 2.
- **No walk after a reload or a game switch.** The account starts with the first reply of
  the session; the M4 log drawer covers the past.
- **Hot-seat secrecy is not improved.** The set table shows every human seat's deal on one
  screen, as the setup screen does today.
- **Below 1201px** the column stack is M5's, unchanged (M5 spec R8).
- **No identity, lobby, chat, spectating or hosting change.** M6 (§6.3).
- **The keep valuation is not touched** (R5): its weights are `greedy.rs:469-494` as
  shipped, and the web quotes the CLI's hedge beside every score.

---

## 6. Wrap-up rows and the handover into M6

### 6.1 Guidingpath rows (append-only, at the close)

1. The M5 close row (Task 0).
2. The M5.5 close row.
3. **The draw-order note (R2):** the engine consumes the shuffled order deck deal-first
   (`setup.rs:96-104` then `:151-154`); the rulebook (p.4 step 12) fills the ports first
   and deals after. Equivalent in distribution; a re-order would move every seeded
   artifact and is a `SEMANTICS_REV` bump for no rules fidelity — not done; M7 at the
   earliest if ever (owner, 2026-09-04).

### 6.2 V5 notes for the AI lane (recorded forward; the next cycle's entry artifact lists them)

1. **A sighted Keep.** `score_starting_order` reads only the card; at step 12 the port,
   goals, lab and board are face up, and the set table now shows them at the moment of the
   choice. A Keep that reads them is a bot-behaviour change with its own pre-registration.
2. **Per-good requirement weights.** `KEEP_REQUIREMENT` prices every required good at 8;
   copper and silk are harder to obtain than tea and seafood (owner, 2026-09-04).

### 6.3 The handover into M6

The close doc of M5.5 is M6's entry artifact and carries, in this order:

- **The walk as the spectator primitive.** A spectator's client feeds `[prev, next)` from
  a stream of events instead of a reply; every component of §3.2 is reused as is.
- **M4's hosting list**, inherited unchanged (`m4-spec.md:507-524`): Access JWT
  verification does not exist and is not free; `GET /api/games` enumerates every game to
  any caller; the Worker shell (`deploy/worker/src/index.ts`) already forwards the Access
  identity as `X-Yokohama-User` and strips a client-supplied one.
- **M6's first three questions**, with the lean stated in conversation on 2026-09-04:
  the transport (server-sent events over the axum server, since both hosted shapes stream
  and one process serves three players); identity (verify the Access assertion at the
  origin, or let the Worker be the only door); the store (Neon Postgres only if the
  Worker takes a share of the state, else files behind the existing `SessionStore`).
- **M5's LEAVE list** items M6 inherits (handover §6): the unbounded saved-game list, the
  "New game" in-flight guard, the popover's browser cell.

### 6.4 Memory

`yokohama-web-lane-status.md` is updated ONCE at the M5.5 close: M5 closed, M5.5 closed,
M6 next with §6.3 as its entry.

---

## 7. Read order for whoever picks this up

1. `CLAUDE.md`, then this file.
2. The M5 spec's §3.1–§3.5 for the table this changes, and its §6 item 2 for M6's name.
3. M5's execution handover §7 for Task 0's list, and §6 for the LEAVE list.
4. `web/src/Pieces.tsx`, `web/src/tileRects.ts`, `web/src/App.tsx`'s `past`/`scrubK`
   comment block, `web/src/TableSections.tsx`'s header, `web/src/NewGame.tsx`, and
   `crates/yokohama-web/src/games.rs`'s `config_of`/`preview`/`create`.
