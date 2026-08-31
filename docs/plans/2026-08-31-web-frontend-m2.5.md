# Web frontend — M2.5: the client gets a harness

**Written 2026-08-31, on branch `web-frontend`, at the M2.5 head.** The
companion to `2026-08-31-web-frontend-m2.md`, which closes with the sentence
this milestone exists to retire:

> **The client has NO automated test, and this is the largest hole in the
> lane.** … Until then, "the suites are green" is not a claim about the browser.

It is now. What follows is what was built, what each instrument proves, and —
the part that matters more — what each one **cannot** see.

**Owner ruling, 2026-08-31:** M2.5 goes in ahead of M3, *"especially so we can
build as we go once we get it up to date."* Infrastructure FOR M3, not a debt
pass: M3 roughly triples the client's surface, and building four more views on
an untested footing means finding the next four defects the same way the last
two were found — by hand, late.

**Owner direction on shape, same day:** *"a broad testing suite, from headless
browser to vitest to client payload since you will likely have to touch on
each."* All three pillars the M2 handover named, not a choice among them.

---

## 1. What the three pillars are, and why three

The handover named three candidate shapes and observed they were not exclusive.
They are not alternatives at all — each sees a class of defect the other two
are structurally blind to, and the two client defects M2 actually found sit in
two different classes.

| | Instrument | Sees | **Cannot see** |
|---|---|---|---|
| 1 | **The wire contract** — `crates/yokohama-web/tests/wire_fixtures.rs` + `web/src/wire/` | A field renamed, dropped or retyped on either side of a hand-written type | Anything about rendering. A payload can be perfectly agreed-upon and drawn wrong |
| 2 | **Component tests** — `web/src/*.test.tsx` under vitest + jsdom | What each branch renders, what it sends, and what it will not send twice | Anything that needs a browser: layout, paint, image loading, real focus order |
| 3 | **The browser walk** — `web/e2e/` under Playwright | That the SCREEN can be played to a winner, that pictures load, that a panel can be left by keyboard, that the page does not overflow | Anything it does not click. Its coverage is what the policy in `e2e/harness.ts` reaches, and §5 names what that excludes |

**The multiplier that makes it one harness rather than three suites:** pillar 1's
fixtures are pillar 2's props. Every state a component test renders is a payload
the server emitted and a golden refuses to let drift. The alternative — a
`makeState({...})` builder — is a third statement of the wire shape, and it
drifts in exactly the direction that matters: **a builder still compiles, and
every test using it still passes, after the server drops a field.**

## 2. Pillar 1 — the wire contract

### 2.1 The hole

`web/src/types.ts` is hand-written and says so. `npx tsc -b --noEmit` proved the
client agreed with **itself**; the whole-game gate was entirely server-side. A
field renamed on one side and not the other compiled on both and failed in a
browser.

### 2.2 What was built

`wire_fixtures.rs` drives the **real router** — `tower::ServiceExt::oneshot`, no
socket — through a deterministic 2P game and writes every payload shape the
client reads to `web/src/wire/*.json`. **30 files, ~800 KB pretty-printed.**
The test then refuses when what the server emits is not what is committed, and
refuses a committed file nothing emits any more. `UPDATE_WIRE_FIXTURES=1`
rewrites instead of checking — and *fails the run*, so a regeneration can never
be mistaken for a pass.

On the client side, `web/src/wire/contract.ts` runs under the **existing** type
gate: no new command to remember, and `npm run typecheck` gained teeth rather
than a sibling.

### 2.3 Two decisions worth not re-deriving

**The fixture game has NO bot.** `Games::create` draws the bot seed from OS
entropy (`games.rs`'s `config_of`) — deliberately — so a golden built over a bot
game would churn on every run. A 2P table of two human seats is what the CLI
calls one operator driving the table, is fully determined by the setup seed, and
reaches every prompt the client can be shown. The one payload that genuinely
needs a bot (`Watching`, which exists *because* the acting seat is a bot) is
built in process with a fixed bot seed.

**The walk PLACES.** The server-side gate answers every placement picker with
`confirm` on an empty selection, which is legal and keeps it from having to know
which tiles accept an assistant; it can afford that because a bot drives the
game to an end regardless. Without a bot, placing nothing means no assistant
ever reaches the board, no area action ever fires, and nothing triggers the end
of the game — **measured at turn 3,644, ten cards still in the deck, every seat
still holding eight assistants, nobody above zero VP.** So the placement arms
*ask the server* which areas it will take rather than restating a rule the
engine owns.

### 2.4 What the corpus covers

Every `PromptState` variant, enforced by a second test with **one named
exemption**: `reveal` is live-mode only and this API creates no live games. A
new variant that no fixture covers fails that test rather than shipping a
browser branch nobody rendered. A third test pins the tag list against the type
itself, so the exhaustiveness check cannot go vacuous by going stale.

Beyond the variants, the corpus carries the **shapes within a kind** that the
client branches on separately, because one payload per kind is a corpus in which
every main prompt is the one-row "end the additional-action phase":
`options_many` (3 rows against 186 legal moves), `options_priced` (one row with
a zero fee, one with none — `free` versus nothing), `retreat_answered`
(`remaining: 0`, where the count buttons must vanish), `place_a_selected` /
`place_b_selected` (a placement with something selected), `fee_nonzero` (a
placement that actually COSTS — see below), `hidden_hand` (a masked hand,
reached through the real `privacy` toggle), `refused` (a live prompt carrying a
refusal in `messages`), `options_4p` and `areas_4p` (four seats, the **Canal**,
and the Chinatown rung — none of which the 2P tile set contains), and three
`choice_*` payloads, one per queued-choice head the walk met.

**`fee_nonzero` took steering, and the reason is worth having.** A whole random
2P game produced NO fee above zero anywhere — measured — because the placement
probe tried area ids in ascending order and therefore placed in area 0 every
turn, and a placement only costs anything when another seat's President is
standing there. So the probe now tries president-occupied tiles first. That
reads the PAYLOAD (`tiles[*].presidents`), not a rule, and the server still
decides what it accepts. Without it the client's `pay ¥N (n to Blue)` wording
had no payload behind it at all and only `free` was ever rendered.

The walk **asserts** it reached each of those, so the corpus's file set is a
function of the three seed constants rather than of where a walk happened to go.

### 2.5 The limit, stated

TypeScript widens a JSON import's primitives: `"sim"` becomes `string`. So the
fixtures are checked against `Widen<ClientState>`, which applies the same
widening to the declared type. That catches every field the client declares and
the server no longer emits — the drift that breaks a browser — and **does not**
catch a field the server ADDS that the client never declares, because
excess-property checking applies to object literals and a JSON import is a
value.

§2 of `contract.ts` closes that at the payload **roots** with key lists pinned to
`keyof T` at compile time and to `Object.keys(payload)` at run time. Deeper
levels are covered in one direction only. That is written in the file rather
than left to be discovered, in the same spirit as `art.rs`'s "these tests prove
the join is TOTAL, not TRUE".

**Two further limits, both found by the review in §9 and both now closed rather
than merely stated.**

- **`PromptState` is the one union widening RUINS.** It is `#[serde(tag =
  "kind")]` — internally tagged, the one exception on the wire — so
  `Widen<{kind:"reveal"}>` and `Widen<{kind:"game_over"}>` both collapse to
  `{kind: string}`, and a union accepts a value matching ANY member. Every
  prompt payload matched that one, so all thirteen prompt fields the client
  renders were checked by nothing: replacing `retreat.json`'s whole prompt with
  `{"kind":"nonsense"}` type-checked clean. §1b now asserts each prompt fixture
  against its own member via `Extract`, and the same mutation names all five
  missing fields.
- **A TUPLE degrades to an unbounded array**, and that is forced rather than
  chosen: TypeScript types a JSON array as `T[]`, so a mapped form preserving
  `[number, number]` would reject every real fixture. `grid_position`,
  `assistants`, `progress` and `fee.to_players` are therefore checked for their
  element type at compile time and for their ARITY at run time, in
  `contract.test.ts`. `Widen<Record<string, X>>` is an index signature and `{}`
  satisfies one, so the keyed payloads get explicit non-emptiness cells there
  too.

## 3. Pillar 2 — the component tests, and the mutation record

**118 tests, 8 files, ~1.4s.** The suite is not a coverage exercise; almost every
cell is a defect this lane has already met, or a rule the terminal states that
the browser must not round off.

**Mutation-proven** — each mutation applied, the named test observed to fail,
the file restored:

| Mutation | Test that failed |
|---|---|
| `inFlight` ref → the `busy` **state** in `App.tsx` | both `a double-clicked … posts exactly once` |
| `prompt.remaining > 0 &&` guard removed in `Decision.tsx` | `offers no counts once every area is answered` |
| the live placement-price line removed | `states the price once an area is selected` |
| `loaded?.of === name` → `loaded?.data` in `Surfaces.tsx` | `a payload is never rendered through another surface's branch` — **and it reproduced the original crash exactly**, `panel.rows.flat()` on a `LabsPanel` |
| `panel.current?.focus()` removed | `takes focus when it opens` |
| the seed sent back through `String(Number(...))` | `goes back to the server as the exact string the preview returned` |
| a fixture key renamed on disk | `the_committed_wire_fixtures_are_what_the_server_emits`, naming the file |
| `end_triggers` renamed in `types.ts` | `contract.ts`, in both halves — the `Widen` assignment AND `KeysOf`, whose error reads `["this list is missing keys the type has:", "end_trigger"]` |
| `retreat.json`'s prompt replaced by `{"kind":"nonsense"}` | §1b's `PromptOf<"retreat">`, naming all five missing fields (**this passed before the review**) |
| a tenth `PromptState` variant added to `client.rs` | `wire_fixtures.rs` fails to COMPILE: `non-exhaustive patterns: &PromptState::Sleeping { .. } not covered` (**this passed before the review, all three tests green**) |
| every art URL prefixed so no file resolves | the browser walk's `every picture the client draws actually loads`, listing 31 broken images |

**One of those mutations changed the test rather than the code, and it is the
one worth reading.** The first draft of the double-click test used two separate
`fireEvent.click` calls; each is wrapped in `act`, which flushes state, so the
tile was already `disabled` by the second click and the test **passed with the
guard removed**. Both clicks now go inside ONE `act`, which is the batched case
the ref exists for and the case `busy` cannot cover. A test that cannot fail is
worse than no test, and this one could not until it was made to.

## 4. Pillar 3 — the browser walk

**7 tests, 16–40s warm, real Chromium against the real release binary.** The
spread is the whole-game walk: the setup seed is drawn by the SERVER, so the
game length varies run to run and so does the wall — two measured runs took 87
and 371 moves. The run PRINTS its own figure, including which affordances it
used (`371 moves via option×173, placement×39, count×38, confirm×28, only×93`),
and every assertion is structural rather than numeric for exactly that reason.

It clicks what is on the screen and **never reads the payload to decide**. A walk
that consulted `/api/games/{id}` would be testing the API a second time; the
question here is whether the screen offers a way forward at every point a game
can reach. When it does not, that IS the finding, and it fails with the decision
zone in the message.

Three things this pillar found about itself, all fixed and all worth recording:

1. **It read the screen mid-request.** Every control is `disabled` exactly while
   the client's `busy` is set, so a read taken between the click and the response
   sees nothing enabled and reports "nothing left to click" — a true statement
   about that instant and a false one about the game. It now waits on
   `.decision button[disabled]` clearing, which is the same condition the client
   renders from and therefore cannot drift from what a player can press.
2. **`reuseExistingServer` talked to a stranger.** A stale `yokohama-web` left
   listening on 8099 by an earlier session answered instead — with the
   operator's real `~/.yokohama/games` behind it. The walk read a saved game it
   had never played, and was writing into a directory it does not own. Reuse is
   now OFF and the port is 8479, so a collision is a loud bind failure.
3. **Five round trips per move became one.** Reading each affordance with its own
   locator both cost five round trips per click and could resolve a locator that
   a re-render then removed, which waits out the whole test budget on an element
   that is never coming back. One `evaluate` is faster and atomic against a
   render.

**A browser must be INSTALLED.** Absence is a refusal naming
`npx playwright install chromium`; `YOKOHAMA_WEB_E2E_OPTIONAL=1` turns it into a
skip that prints, out loud, that the run tested nothing. Same shape and same
reasoning as `YOKOHAMA_BGA_CORPUS_OPTIONAL`.

## 5. What is deliberately NOT covered — read this before trusting a green run

- **The walk's reach is its click policy's reach.** `e2e/harness.ts` answers
  option rows, placement pickers, retreat counts, Confirm and the sole-option
  button. It never sends `undo`, never opens a surface mid-game, never cancels a
  picker, and never drives the `locked` or `watching` prompts — the first three
  because they are component-tested, the last two because a browser cannot reach
  them without a fault or a bot mid-pump. Those five branches are covered by
  pillar 2 and by fixtures, and by nothing in a real browser.
- **`reveal` has no fixture and no test anywhere.** Live mode is out of v1 by
  owner decision, this API creates no live game, and the server-side gate panics
  if a simulation ever asks for one.
- **A phone layout is not asserted, and does not work.** MEASURED at 390 × 844:
  **545 px of horizontal overflow.** 1440 and 1024 are asserted. The client is a
  desktop surface — a staggered pyramid up to six tiles across, each carrying a
  site strip — and no phone layout was ever designed or asked for, so asserting
  one would be asserting a requirement nobody set. Recorded so that whoever takes
  it up starts from a number.
- **`Widen<T>` is one-directional below the roots.** §2.5 above.
- **The art join is still TOTAL, not TRUE.** `art.rs` proves every identifier
  resolves; the browser walk now proves every resolved file actually decodes.
  Neither can see a complete-but-wrong mapping, which is how the crop origin and
  the category-C achievement colours both shipped. **Regenerating and not
  looking still proves nothing.**
- **REQUEST bodies are outside the contract entirely.** The corpus and
  `contract.ts` cover responses. `NewGameRequest` and the `{line, expect}` input
  body are hand-written on the client and hand-written again as a `json!`
  literal in `wire_fixtures.rs`. Worse, `games.rs` marks `start_seat`, `seed`,
  `bot_seats` and `name` `#[serde(default)]` and nothing in the crate uses
  `deny_unknown_fields`, so a renamed request field is **not a 400 — it is a
  default**, and the game starts wrong with a 201. `NewGame.test.tsx` now pins
  the key set this client sends, which closes the client half; the server half
  is a decision (add `deny_unknown_fields`) and is left to the owner because it
  changes what the API accepts, which is not a harness change.
- **Four of seven queued-choice heads have no payload.** The corpus carries
  `AssistantMove`, `ChurchSpace` and `WarehouseTake`; `AnyGood`,
  `RecoveryLeave`, `StationPlace` and `CustomsSpace` are reachable in principle
  and the walk has not met them. `choiceWords` falls through to the raw enum key
  for an unknown head — a silent fallback, so nothing would fail; the player
  would simply be shown `StationPlace`. Named in `contract.ts`'s
  `CHOICE_HEADS_COVERED`, and the walk banks any head it meets.
- **The browser walk's `count` and `only` affordances are RECORDED, not
  required.** Three are asserted (`option`, `placement`, `confirm`); a guided
  retreat and a sole-option prompt are not guaranteed to arise in a given random
  game, and a required-but-occasional cell is a flake wearing a coverage badge.
- **A type error in a component test now fails `npm run build`**, because the
  build is `tsc -b && vite build` and the tests are inside `src`. Deliberate: the
  alternative is a second tsconfig whose only purpose is to let a broken test
  ship.
- **No test asserts a colour, a spacing or a font.** There is no visual
  regression baseline and none is proposed: the client is still being designed,
  and a screenshot baseline during a design phase is a test that fails on every
  intended change.
- **The component suite runs in jsdom**, which is a DOM implementation and not a
  browser. Anything it asserts about focus or visibility is a statement about the
  DOM, not about what a person sees; that is what pillar 3 is for.

## 6. What did not move

`crates/yokohama-core` is byte-identical to master and `SEMANTICS_REV` is 7. No
`*_render.rs`, `screen.rs`, `style.rs` SGR table, `HELP_TEXT`, `describe`
constant arm or `cli-guide.{md,html}` byte moved; the nine `assert_capture`
goldens and 18 `guide_sync` tests are unchanged.

**The client's production code is unchanged except two added TypeScript
interfaces** — `Health` and `ApiErrorBody`, both shapes the client already
depends on (`api.ts` reads `.error` off the second) and neither previously
declared. Every mutation in §3 was applied and reverted; the bundle is
**228.62 KB / 70.93 KB gzipped**, against M2's 229 KB, and the wire corpus is
**not** in it — nothing reachable from `main.tsx` imports it, and a grep of
`dist/` for a fixture string returns zero.

## 7. Counts at this head

| Suite | Count |
|---|---|
| `cargo test -p yokohama-web` | **33** (29 lib + 1 whole-game gate + 3 `wire_fixtures`) |
| `cargo test -p yokohama-cli` | 849 + 2 ignored — **unmoved** |
| `cargo test -p yokohama-core` | 1577 + 29 — **unmoved**, core byte-identical to master |
| `cargo test --workspace` | 46 test binaries + 6 doc-test targets, zero failures |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `(cd web && npm run typecheck)` | clean, both projects |
| `(cd web && npm test)` | **118** in 8 files, ~1.4s |
| `(cd web && npm run e2e)` | **7**, 16–40s warm (the walk's length is the server's seed) |
| four `bridge/tests/*.js` node suites | green |
| committed wire corpus | **30** JSON files, ~800 KB |

## 8. Registration

Done as part of the task, per the standing rule that an unregistered test
command is a test that never runs: CLAUDE.md's Commands section carries the
`yokohama-web` row (33) and a new **Web (browser half)** block naming all three
client commands and the browser prerequisite; README's gate paragraph — which
declares itself the binding list — carries the same three.

The `#[ignore]`d-gate roster is **untouched**: nothing added here is
`#[ignore]`d, and this lane still claims no place in the fifteen. Whether the
browser walk earns a roster row is the owner's call, and the honest argument
either way is in §4 (it wants a `--release` build and a browser, which is a
release-tier cost) and §5 (its coverage is its click policy's).

## 9. The adversarial review, and what it found

**Five lenses over the new test code, run after the three pillars were green,
with VACUITY as the first lens** — the right first question when the deliverable
IS tests. It returned 44 findings. The four high-severity ones were all real,
and three of them were false claims made by this milestone about itself:

1. **The `PromptState` exhaustiveness guarantee was vacuous.** §2.4 and the
   test's own doc claimed "a new variant is a non-exhaustive-match compile error
   here". There was no match: the only exhaustive one was `PromptState::kind` in
   the crate, so both hand-written lists in the test stayed nine items long. The
   reviewer added a tenth variant and **all three tests passed**. There is now a
   real `match` with no `_` arm in the test file, plus a load-bearing count; the
   same probe now fails to compile.
2. **`Widen<PromptState>` collapsed the union to `{kind: string}`.** §2.5 above.
   Replacing a fixture's whole prompt with `{"kind":"nonsense"}` type-checked
   clean, so the thirteen prompt fields the client renders — the structure with
   the most client branches in the whole surface — were checked by nothing.
3. **`npm test` and `npm run e2e` did not exist.** They were registered in
   CLAUDE.md, README, this file and the ledger, and `package.json` had four
   scripts. A `cd web && python3 …` had failed with `no such directory` in a
   compound command and the edit was silently lost. **The repo's rule is that an
   unregistered command is a test that never runs; this was the inverse and
   worse** — the command was registered everywhere and existed nowhere.
4. **The documented E2E opt-out exited 1.** `YOKOHAMA_WEB_E2E_OPTIONAL=1` set
   `testIgnore: /.*/`, which leaves Playwright with zero discovered tests, which
   Playwright treats as an error. On the one machine the escape hatch exists for
   it still failed the run. Every spec now carries `test.skip(!haveBrowser, …)`,
   so the reporter prints **7 skipped** and the run exits 0 — a skip you can
   count is not a skip you can miss.

The mediums worth naming, all fixed: a whole random 2P game produced **no fee
above zero anywhere** (the placement probe always chose area 0, so nothing was
ever placed under a President); `renderApp`'s bootstrap answered `/api/art`
before the caller's handler, so the art-404 test never entered the branch it
named; `watchForBreakage` had no origin filter, so six of seven browser tests
depended on `fonts.googleapis.com`; the reload test compared a banner string
that does not change with gameplay; the walk's "Apply the only option" branch
was unreachable; `mkdtempSync` at config scope leaked a temp directory per
invocation.

**And the review's own method is worth keeping.** The three false claims were
all claims *this milestone wrote about itself* — the kind a self-review does not
catch, because it re-reads the sentence rather than the mechanism. Each was
found by an agent that tried the mutation the sentence promised would fail.
