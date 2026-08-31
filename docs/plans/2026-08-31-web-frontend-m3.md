# Web frontend — M3: it explains

**Written 2026-08-31, on branch `web-frontend`, at the M3 head.** The spec this
milestone was built to is `2026-08-31-web-frontend-m3-spec.md`; read that first
for the rulings and the design, and `-m2.5.md` §5 for what the harness under all
of this still does NOT cover.

M3 is the milestone the plan calls "it explains": `suggest`, `why`, the four card
indexes, and the final score breakdown. The `why` screen is its signature
surface, being — in the plan's own words — "the one thing a terminal genuinely
could not do".

---

## 1. What landed, in the order it landed

| Wave | Contents |
|---|---|
| **A** | The `yokohama-cli` seam: visibility widening only (`pub mod why`, `pub mod card_prompt`, `Refusal`/`Mismatch`/`WhyAccount`/`Reconstruction`/`account_parts`, `bot::Decision`/`DecisionExplanation`, `Repl::index_placements`, a new `Repl::bots()`) |
| **B** | The request-body fence (owner ruling R2): `deny_unknown_fields` on all three request types, behind a `JsonBody`/`QueryArgs` extractor pair |
| **C** | `explain.rs` + `GET …/suggest` + `GET …/why` + four wire fixtures + the refusal exhaustiveness fence |
| **D** | The `why` chart (owner ruling R1), the suggest panel, their component tests and a dedicated browser spec |
| **E** | `outcome.rs` + `GET …/index/{kind}` + `GET …/result` + six more fixtures + the client views |
| **F** | The six-lens adversarial review, this record, and registration |

## 2. The two owner rulings, and what each bought

**R1 — the `why` chart is a diverging bar**, one row per feature, sorted by
|contribution| descending, growing from a centre zero line, with the total
footed at the bottom.

That last clause is the one doing the work. `eval::explain` guarantees

```
total == sum(terms.product) + terminal.value
```

All 34 ship — the zero-product ones included — the client COLLAPSES them behind
a disclosure rather than dropping them, prints how many it is hiding, and prints
the total under a rule as the sum of the column above it.

**Not because the sum needs them.** An earlier draft of this record and of the
code's own doc said "the identity only holds over all of them", and the review
did the arithmetic: a zero-product term contributes zero, so the sum over the
carrying terms alone is the same number. The real reason is the one
`Explanation` gives itself, and it is about the reader — reporting one term per
feature always lets a reader "see that a term contributed nothing rather than
wonder whether it was computed", and it is what makes "8 of 34" a count of
something the screen is showing. **A reader can add it up.** That is the difference
between a chart and an explanation, and it is asserted in four places: a route
test, a runtime cell over every committed fixture, a component test, and a
browser cell that reads the numbers off the SCREEN.

**R2 — the request bodies are fenced now, before M3 widened the hole.**
`NewGame`'s `start_seat`, `seed`, `bot_seats` and `name` were `#[serde(default)]`
with no `deny_unknown_fields` anywhere, so a client that sent `startSeat` got
seat 0, a 201, and a different game than it asked for. The mutation proof for the
new test prints the defect verbatim: `left: 201, right: 400`.

Fixing it turned up a second defect nobody had named: `axum::Json`'s rejection is
a PLAIN-TEXT body and a 422, while `web/src/api.ts` builds its `ApiError` by
reading `.error` off the body — so a malformed request reached the player as a
bare status code with no sentence at all. The fence is therefore an EXTRACTOR
rather than an attribute, and it sits before any handler runs, which also gives
the right precedence: a bad body against a missing game answers 400, not 404.

## 3. The four things the recon dissolved

A reconnaissance over the eight surfaces M3 consumes returned a list of "owner
decisions". Four were already settled by code that exists, and are recorded here
so nobody re-opens them:

- **A feature's wire key needed no core change.** `Feature::name()` is already
  `pub` and already the stable snake_case token, and its own doc says report
  columns, explanation rendering and the T4 probe all come from there "so none of
  them is hand-typed". No `Debug` string, no `Feature::label()` in core.
- **Wording is a web concern and the token stays on the wire.** The snake_case
  name is what greps against `eval_probe`, the scenario suite and every recorded
  figure. Display words are served from `/api/labels` beside it.
- **`why`'s reusable half was split out FOR this consumer**, and says so:
  *"A consumer that draws its own table wants the first half and none of the
  second, and it must not reimplement the first half: the refusals ARE the
  honesty gate."* The web calls `account_parts` and re-derives no refusal.
- **A `why` fixture needed no new request field.** `constructed()` with a fixed
  `BOT_SEED` was already the `watching` precedent.

## 4. The three design decisions with no precedent in the crate

Stated rather than left to whichever handler got written first:

1. **A typed refusal is a 200 with a refusal body.** `why` declining because the
   last move was a human's is the ANSWER, not a client error. Only a malformed
   request (400) or a missing game (404) is an error. The twelve refusals are
   data the browser renders — and the M2.5 lesson is that a branch nobody has
   ever seen is where defects live.
2. **Both search routes drop the per-game lock before searching.** Every other
   handler holds the `Mutex<Repl>` across its whole body, which is right for a
   projection and wrong for a multi-second search that would block the poll
   drawing the board. `GameState` is history-free and cheap to clone for exactly
   this reason.
3. **`lib.rs`'s "if it ever computes something about the game, that is a bug"
   gains its one named exception.** The crate computes nothing about the game
   STATE — it never advances one — and the two search routes read an opinion
   about a state without changing it.

## 5. What the browser found that no test would have

Three defects, all found by looking at the screen, which is why the dataviz
procedure's last step is "render it and look at it".

1. **`ranking[0]` is not the chosen move.** The ranking is sorted best-first, but
   the pick is one of the MAXIMISERS and ties are broken by a keyed draw —
   `tied_at_top` exists to say so. Measured on the committed fixture: two
   candidates tie at 78 and the played move is **row 1 of 186**. `slice(1)`
   therefore listed the move the bot actually played among the ones it beat, and
   hid a real runner-up. Fixed on the WIRE (`Ranked.chosen`, compared by `Move`
   identity server-side) rather than in the browser, because inferring identity
   from a display string is a display string doing a type's job.
2. **`(0 − 5) × 14` had no parentheses.** `0 - 5 × 14` reads by precedence as
   `0 - (5 × 14)`; on a row where those do not coincide the printed arithmetic
   contradicted the printed product.
3. **The zero line was invisible** — drawn at `--line-bright` inside a 10px track
   with a 2px row gap. A diverging chart without its zero is just two colours.

And two more on the second look:

4. **The Goals panel did not show what its own nav blurb promised.** The blurb
   said "how close every seat is"; the progress vector was in the payload and the
   component dropped it. That is the M2.5 lesson in miniature.
5. **The achievements had no art** while every other index drew its cards.

Plus one the CONTRACT found on the first typecheck: `trading_house_reward` on the
raw registry site card is a bare `number`, not a `SpaceReward`. The hand-written
type said otherwise and `contract.ts` rejected it against the committed payload
immediately — which is the entire reason the corpus exists.

## 6. Colour was computed, not chosen

The chart's pair is blue `#3d86e0` / orange `#e06a44`, validated against the
panel's surface: **ΔE 23.0 under protanopia, 29.8 in normal vision**, both inside
the dark-mode lightness band, contrast ≥ 3:1. The house `--green`/`--red` would
have been the classic mistake — the one pair a red-green-colourblind reader
cannot separate, on a chart whose entire meaning is which side of zero a row
falls on.

**Colour is never the only encoding.** Every row sits on its own side of a
painted spine AND carries its signed number in a monospace column, so the chart
reads in greyscale, in forced-colors mode, and to a reader who sees no difference
between the two hues.

## 7. Counts as M3 was IMPLEMENTED — before the review

**These are the figures the implementation closed on, kept because the
difference between them and §10 is what the adversarial pass cost and bought.**
§10 is the reading at the close.

| Suite | Count (pre-review) |
|---|---|
| `cargo test -p yokohama-web` | **54** (49 lib + 1 whole-game gate + 4 `wire_fixtures`) |
| `cargo test -p yokohama-cli` | **849 + 2** — UNMOVED across the seam widening |
| `cargo test -p yokohama-core` | **1577 + 29**, and byte-identical to master |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `(cd web && npm run typecheck)` | clean, both projects |
| `(cd web && npm test)` | **173** in 10 files |
| `(cd web && npm run e2e)` | **11** |
| four `bridge/tests/*.js` node suites | green |
| committed wire corpus | **40** JSON files, 1.4 MB |
| routes | **15** (was 11) |
| bundle | 241.84 KB / 74.77 KB gzipped (M2.5: 228.62 / 70.93) |

**The wire corpus is still not in the bundle** — re-verified with three probes
(`no_bot_move_yet`, `index_filtered`, `tech_bonus_now`), all zero hits in
`dist/`. The one apparent hit for `v4-step9-draft1` traces to a hardcoded string
in `NewGame.tsx` that predates M3.

The client's existing production components — `App`, `Board`, `Cards`,
`Dashboard`, `Decision`, `NewGame`, `main`, `art` — are **byte-unchanged**. M3
added two components, extended the surface registry, and appended to
`api.ts`/`types.ts`/`styles.css`.

## 8. What M3 deliberately did not do

- **`ts-rs` stays open.** M3 added hand-written types under the M2.5 contract, so
  the decision is no more urgent after M3 than before — and taking it inside a
  feature milestone would mean deciding it under delivery pressure.
- **No visual-regression baseline.** Still a design phase.
- **No phone layout.** Still 545 px of overflow at 390 px, measured and
  deliberately not asserted. M3 did not widen the claim.
- **No roster row.** The fifteen-gate roster is untouched and nothing added is
  `#[ignore]`d.
- **`reveal` is still tested nowhere**, because this API still creates no live
  games.
- **`why <n>` over an arbitrary event** is served by the API (`?event=n`) but the
  client only ever asks for the default. Wiring the log's rows to it is an M4
  shape, not an M3 one.

## 9. The adversarial review, and what it found

**Six lenses over the M3 diff, with VACUITY first**, run after the milestone was
green and registered. One wave, no verify fan-out — the M2.5 handover records
that re-litigating findings which already carry their own reproduction was the
one workflow worth killing, and every lens here was required to produce a
reproduction or say plainly that it could not.

**42 findings, 36 of them reproduced: 5 HIGH, 13 MEDIUM, 11 LOW, 13 notes.**
Alongside them the lenses verified **89 claims as TRUE**, which is the half worth
saying out loud: core byte-identical to master, the seam commit visibility-only
in every hunk, the counts as stated, the nine goldens and 18 `guide_sync` tests
unmoved, no field dropped in any DTO conversion, `CardFace` genuinely checked,
and the `ok | null` envelopes NOT the `PromptState` defect repeating.

### 9.1 The five HIGH findings

**1. The refusal fence's middle link did not exist — and it was M2.5's own
defect, restated as fixed.** The doc claimed a three-link chain guaranteeing a
thirteenth `why::Refusal` could not reach the browser uncovered. Links one and
three held; the middle one did not. There were THREE hand-written lists (the tag
array, the test's constructed samples, the fixture builder's vec) all agreeing
with each other and none pinned to the enum, so a thirteenth variant plus a
one-line `kind_of` arm reached the wire with all 54 tests green. The reviewer
reproduced it in an isolated worktree.

*Fixed by* collapsing three lists into one (`explain::refusal_samples()`, from
which the tag list, the corpus and the client's coverage test all derive) — and
by **rewriting the claim to what is true**, including the residual hole it cannot
close: plain Rust has no way to enumerate an enum's variants, so closing it
completely would mean a derive dependency, which is a dependency decision and not
a test change. The doc now names that.

**2. `Terminal` was declared, rendered and checked by NOTHING.** `terminal` is
`null` in both search fixtures, so `Widen<Explanation>` never descended into
`Widen<Terminal>`; a field serializing to `null` contributes no bytes for the
golden to compare; and the one client test of the branch HAND-AUTHORED its
payload — the single thing `contract.ts` §3 forbids. Renaming all five wire
fields left 54 Rust tests, the typecheck and 173 vitest cells green. Not
hypothetical: a search continuation that ends the game reaches a terminal leaf.

*Fixed by* an `explanation_terminal` fixture from a real finished bot-vs-bot
game (rank 2 of 2, value −1.042e9 — which also settles the precision question:
magnitudes are ~1e9 against JavaScript's 9e15 safe integer), a `TERMINAL` key
list in both halves, and a client test that renders the SERVER's payload.

**3. `CardLocation`'s permissive arm made the union vacuous — and was already
swallowing a real miss.** `types.ts` declared five of eight variants and ended in
`| Record<string, unknown>`, which makes every object assignable: the reviewer
type-checked `{"TotalGarbage": {...}}` and the bare string `"utter nonsense"`
clean. `OnArea` was the missing variant, ten of the twenty-four site cards carry
it, and the browser printed the literal Rust tag **`OnArea`** where the terminal
prints `on area A03`.

*Fixed by* declaring all eight, deleting the permissive arm, and making
`locationWords` exhaustive with a `never` check — so a ninth variant is a compile
error. A test walks the whole corpus and asserts no caption is a bare tag.

**4. The site index numbered from 1; every other surface numbers from 0.** The
board tile, the art-less card blank, the CLI's tile, its capture name and its own
site-index card all print `Site n` over the raw registry index — and `types.ts`
says so in as many words. A player who clicked a tile reading `Site 2` and opened
the panel to look it up landed on a different printed card. Silent, uniform,
off by one on all 24 rows.

*Fixed*, and the fix uncovered a worse one: `SiteCardView` prints the number only
in its art-less blank, so with art present the panel showed **no site identifier
at all**. A card index a player cannot identify a card with is not an index. The
number is now a chip.

**5. `locationWords`' fallback printed a raw tag as prose.** The same defect as
(3) from the client's side, with its own reproduction: ten DOM elements reading
`OnArea`, zero reading anything like `on area`.

### 9.2 The MEDIUM findings that were real defects

- **Two more nav blurbs claimed things the panel did not do**, found because the
  first one was. The techs panel promised "who owns which copy" and showed only
  the FIRST copy's location, so a title held by both seats read as held by one.
  The sites panel promised "what each space pays" and printed a COUNT of spaces
  and no reward, so all 24 cards rendered identically. Both blurbs also named an
  ORDERING ("by printed corner index", "by gear cost") that the server does not
  group by — it groups by country.
- **The rework found the real fix**: `Cards.tsx` already had
  `AchievementCardView` and `SiteCardView`, with art, chips and the per-seat
  progress bar. M3 had hand-rolled worse versions of both. Reusing them fixed the
  missing achievement art, the missing progress and the site rewards at once —
  and then a browser look found the tech cards OVERLAPPING, because `.card-face`
  carries a fixed width sized for the dashboard and the index lays out on a grid.
- **The lockdown gate was applied to one of three siblings.** `/suggest` refused
  a locked session; `/why` and `/index` did not. The rule is now stated and
  applied: **the lockdown fences COMPUTATION, not projection.** M2 already settled
  that a read-only GET serves a locked session; `why` is not a projection — it
  re-runs the bot's search against the record the driver declared faulty. Pinned
  in both directions, mutation-proven.
- **Six of thirteen key lists had no runtime half.** The reviewer added
  `__server_added_a_field` to six committed payloads and watched the typecheck
  and all 173 cells stay green. Ten rows added, plus pins for `Terminal`,
  `Counter`, `IndexGroup` and `IndexCard`.
- **The goal-progress test could not detect a dropped seat** — it asserted each
  `current/needed` string appeared somewhere, and in this fixture seat 1's
  strings are all duplicated by a seat-0 row. It counts rows per card now, and
  the mutation that slipped through before now fails it.

### 9.3 The claims that were false, and are now the claims that are true

The lens that paid most was the one that read every sentence M3 wrote about
itself and tried the mechanism:

- *"the identity only holds over all of them"* — arithmetically false. Corrected
  at all four sites; the real reason is auditability, not arithmetic.
- *"the conditions are one line each and every one is asserted by a test"* (the
  four `suggest` guards) — only the enum was walked. `Locked` is now pinned by a
  route test; `GameOver`, `BotSeat` and `LiveMode` have their reachability
  **measured and stated** rather than claimed.
- *"the CLI's summary line already states it and the two must not be able to
  disagree"* (`nonzero`) — they disagree by design on a terminal explanation,
  where the terminal deliberately counts the standing row and reads `1 of 35`
  against this field's `0`. Both are right about different things; the doc now
  says which.
- *"the same four refusals the terminal gives, UNWRAPPED"* — one of the four is
  unwrapped, one is byte-identical, and two are AUTHORED. Now listed one by one.
- *"one display form … so the browser and the terminal cannot spell the same bot
  differently"* — the field carried `descriptor().0` while the CLI prints
  `echo()`. A resumed non-default `lookahead-v1` seat would have rendered as an
  undifferentiated name in the browser. Now `echo()`, and the duplicate
  `controller_version` field is gone.
- **The crate's own dependency rationale still said `suggest` and `why` are not
  consumed** — the file that states what the CLI seam is allowed to be, left
  unchanged by the commit that widened it.
- `explanation.nonzero <= 34` was a near-vacuous assertion; the reviewer set the
  server's count to a constant 0 and every lib test stayed green.

### 9.4 What the review did NOT change

`why`'s `search_hit_ply_cap` now renders (it was typed, key-listed and drawn by
nothing); opening an index fired the identical GET twice and no longer does; the
dead CSS the rework left behind is deleted. The nav's height at 1024px with
eleven entries is RECORDED as a note and not fixed — no overflow, and the
milestone's layout is still a design surface.

**The method is the part worth keeping.** Three of M2.5's four HIGH findings were
false claims the milestone made about itself; five of M3's were too, in a
different vocabulary. A self-review re-reads the sentence. An adversarial one
adds the thirteenth variant, renames the five fields, and counts the DOM.

## 10. Counts at the close

Against §7: the review added one route test, one wire fixture and twenty-one
client cells, and moved no production count that was not a defect.

| Suite | Count |
|---|---|
| `cargo test -p yokohama-web` | **55** (50 lib + 1 whole-game gate + 4 `wire_fixtures`) |
| `cargo test -p yokohama-cli` | **849 + 2** — UNMOVED |
| `cargo test -p yokohama-core` | **1577 + 29**, byte-identical to master |
| `(cd web && npm test)` | **194** in 10 files |
| `(cd web && npm run e2e)` | **11** |
| committed wire corpus | **41** JSON files |
