# Web frontend — M6.1 "identity and seats": the close

**Written 2026-09-15 on branch `web-room` at `44dd538`**, twenty-six commits over `master`
(`2533001`): two planning commits, then twenty-four of execution — twenty-one through the
whole-branch review's fix wave at `5be1e54`, and three deployment-record commits after it. **The plan and the
spec are dated 2026-09-14, the day they were written; this close is dated the day the cycle
closed** — the dated-artifact convention fixes a document at its writing, and a close written
the next morning says so rather than back-dating itself into the plan's day.

The authority for everything below is the spec,
`docs/plans/2026-09-14-web-frontend-m6.1-identity-and-seats-spec.md`, under the eleven owner
rulings R-P1–R-P11 in `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0. The plan
`docs/plans/2026-09-14-web-frontend-m6.1-plan.md` is what was executed. Both are frozen; where
a sentence in either is wrong at this HEAD, §5 and §7 say so here rather than editing them.

The live deployment is **`yokohama-web:0.5.0-m6.1`, built from `5be1e54`**, swapped onto the NAS
at 06:59Z on 2026-09-15 and verified twice — `deploy/RUNBOOK.md` §3.7 is that record and is not
restated here.

**This document is M6.2's entry artifact** (§8).

---

## 1. What shipped

M6.1 is the first cycle of M6, "the room". Before it, the browser server read no header, scoped
no route to a person, and dealt every seat's cards onto one screen. After it, a person signs in,
holds named seats at named tables, and every payload the server builds is built for the caller.

### 1.1 Identity — one seam, one header per mode, no default

`crates/yokohama-web/src/identity.rs` (new, 234 lines). `IdentityMode` is `claimed` |
`cf-access` | `worker`, read from `YOKOHAMA_WEB_IDENTITY` at start-up with **no default** — an
unset variable exits 1, because a server that guesses its own door is a server whose door nobody
wrote down. Each mode reads exactly ONE header and no other:

| mode | header | who sets it |
|---|---|---|
| `claimed` | `X-Yokohama-Claimed-User` | the browser, from what the person typed |
| `cf-access` | `Cf-Access-Authenticated-User-Email` | the tunnel, behind Access |
| `worker` | `X-Yokohama-User` | the edge Worker, from `ctx.access` |

`User::parse` normalizes once — trim, lowercase, refuse empty — so a seat bound from a header
reading `  Red@Wire.TEST ` is the seat the next request finds. Under `worker`, the literal
`anonymous` counts as ABSENT, which is what makes an unprotected Worker fail closed rather than
open. `GET /api/me` reports the mode and the caller and never refuses; `GET /api/health` gained
`identity`.

**`claimed` is a development door and the client says so in its own prose**: the name you type
is the name you play under, and behind Cloudflare Access it is still Access that lets you in
(R-1.4). It is not a security boundary and the module doc says that where a reader will hit it.

### 1.2 The store — typed documents, conditional writes, one reserved suffix

`store.rs`'s trait now takes a `Doc` (`Record` | `Table`) on every call and offers
`write_if_version(doc, id, json, expected)` with a typed `WriteError::{Conflict, Io}`. An
unreadable present file is an `Err` and never a `None`: conflating "cannot be read" with "is not
there" is how a create overwrites a game.

Both documents live in one directory, so `<id>.table.json` would collide with the record of a
game legitimately named `<id>.table`. The fix is the GUARD, not a subdirectory layout (R-2.2):
`is_safe_id` refuses any id ending in `.table`, case-insensitively, because APFS and NTFS fold
case. One reserved suffix in the client-nameable id space is the whole cost, and the test derives
the suffix from `Doc` rather than restating it.

`Games::persist` keeps M5.5's compare-and-swap and now propagates a store read error through its
non-evicting path (R-2.1): a Conflict evicts the handle, an Io does not.

### 1.3 The table document — who may sit here

`crates/yokohama-web/src/table.rs` (new, 305 lines). `TableDoc` is stored beside the record as
`<id>.table.json` and carries `schema`, `id`, `name`, `created_by`, `created_at` (unix seconds,
R-plan-1), `players`, `colors`, `start_seat`, `seats` (`SeatDoc::Human { user }` |
`SeatDoc::Bot`), `keeps`, the setup `seed`, a `status` of `Dealing` | `Playing`, and a
`TurnMirror` of `to_move`/`over`.

Three sentences in that shape are load-bearing:

- **The seed lives here and NOWHERE on the wire.** `TableView` is the projection; the record is
  the other place it lives, and neither is served.
- **"Over" is not a status.** It is read from the session, because a finished game is a property
  of the record and a stored copy goes stale.
- **`TurnMirror` is a cache of two integers, refreshed by `persist`, and a lost mirror write is
  ignored** (R-plan-3). No play route reads it. It exists so the games list is built from table
  documents alone instead of rebuilding twenty sessions.

A hand-edited document cannot take the process down: `check_shape` bounds `players` to 2..=4
(R-3.2) and, after the whole-branch review, `start_seat < players` and every `keeps[i]` to
`{None, Some(1), Some(2)}` — all three are `PlayerId::new`/index panics on a path that runs under
`table_lock`, where a panic poisons the mutex and every later create, keep and repair in the
process dies.

### 1.4 Create, deal, keep, finalize — the deal moves per seat

`POST /api/games` takes one row per seat (`{"user": "…"}` or `{"bot": true}`), refuses a creator
who holds no seat (R-1.6), refuses a bot at three and four players in the evaluator's own words
(R-P10), and answers **201 with the table `dealing`**. Nothing is dealt onto one screen:

- `GET …/{id}/deal` serves the CALLER their own next unkept seat's two cards, the CLI's own
  `KEEP_SCORE_NOTE` hedge, and the M5 set table with the technologies and the goals withheld —
  the 2026-09-06 correction's rule, unchanged. Once every seat the caller holds has chosen,
  `seat` and `cards` are both `null` and the screen says who is still waiting.
- `POST …/{id}/keep` accepts a keep only for a seat the caller holds.
- The LAST human keep **finalizes**: the record is written first, the bot seat (at 2P) keeps by
  the same `score_starting_order` argmax it always did, the handle comes alive, and the table
  flips to `playing`.

`POST /api/games/preview` is **gone**, and with it the last surface that carried a setup seed.

### 1.5 The routes as built

Spec §5.3's table, at this HEAD: twenty route registrations — **four PUBLIC** outside
`/api/games`, and sixteen under it carrying **eighteen handlers over sixteen bodies** (`ports`,
`labs` and `areas` are three handlers from one macro). **`/api/me` is one of the four public ones,
and deliberately**: it is the only route that reads the identity headers WITHOUT the extractor,
because the extractor's answer to an absent identity is a 401 and that is exactly the case this
route exists to report. It answers `{"mode": …, "user": null}` to a caller with no identity, which
is what lets a fresh browser learn whether a sign-in screen is even meaningful here. `preview` removed; `table`, `deal`, `keep` and `events` added;
`DELETE` fenced to the creator. **Every one of the eighteen carries `Identity` and USES it.**

| route | requires | built for / fence |
|---|---|---|
| `GET /api/health`, `/api/labels`, `/api/art` | — | public |
| `GET /api/me` | — | public; the mode, and the caller or `user: null`. Never refuses |
| `GET /api/games` | identity | the tables where the caller sits, newest first |
| `POST /api/games` | identity | creator seated (R-1.6); 201 `dealing` |
| `GET …/{id}/table` | seated | `TableView` |
| `GET …/{id}/deal`, `POST …/{id}/keep` | seated | §1.4; keep only for a seat the caller holds |
| `GET …/{id}` | seated | the caller's viewer seat; **409 while `dealing`** |
| `POST …/{id}/input` | seated | **the caller holds the ACTING seat**, else 403 |
| `…/ports`, `…/labs`, `…/areas`, `…/log`, `…/index/{kind}`, `…/result` | seated | the caller's viewer seat |
| `GET …/{id}/at/{k}` | seated | the caller's viewer seat, stable per caller |
| `GET …/{id}/events` | seated | §1.7 |
| `GET …/{id}/suggest` | seated | the acting seat's holder only (R-1.8) |
| `GET …/{id}/why` | seated | the caller's visibility; the M3 masking of the roads not taken |
| `DELETE …/{id}` | seated | the creator only |

### 1.6 The fences

- **Identity per mode.** Exactly one header is read, and a forged header for another mode
  changes nothing. Proven live with a three-header probe (§4.2) and by mutation (§4.3, #1).
- **Seated, or 403.** Every session route resolves the caller to a seat through `seated()`
  before anything else. A signed-in stranger gets 403 "you are not seated at this table" — never
  a payload, never a 200 with an empty body.
- **The viewer.** `viewer_seat` is the acting seat when the caller holds it, else the caller's
  lowest seat. M4's `replay_seat` is retired: `at/{k}` is now the caller's own seat at every `k`,
  which is stronger than a pin nobody could re-key (R-4.2 reads "stable per caller" as "never
  follows another PERSON's seat"; a hot-seat caller's viewer moves only among their own seats).
  Masking is `SeatFacts`' — the CLI's own mechanism, widened, never re-implemented here.
- **The acting-seat fence, at EVERY status** (R-4.3). This is the best finding of the cycle and
  it arrived twice. First a seated watcher could read the acting seat's hand through
  `GET …/index/orders` (§4.4); then, with that closed, the fence dropped at game over, so a
  watcher could rewind a finished shared game. The rule now: the fence never drops for a
  non-acting caller at any status; only the 403's WORDING changes to the game-level sentence.
- **Undo** (R-P6, R-5.1, R-5.2): at a table with ONE distinct human email undo behaves exactly
  as the CLI's does today; otherwise it is admitted only while the trailing event's mover is the
  caller's seat AND `state.current_player` is still that seat. The mover is replay-derived
  (`record::state_at(record, len-1)`), the CLI's own attribution. The rule is checked BEFORE the
  acting-seat fence so an undo attempt is always answered in R-P6's words, and at a FINISHED
  shared game nobody undoes.
- **The seed pin lost its exemption.** `no_committed_payload_carries_the_setup_seed` searches
  every fixture for the driving seed in decimal and hex and finds none, with the anti-vacuity
  partner being that the same test asserts the seed IS in the table document the memory store
  holds — so the search cannot pass by searching for nothing. Default game ids still draw
  independently of the seed.
- **The corpus's four 4xx fixtures**: `dealing_409.json`, `not_seated_403.json`,
  `wrong_seat_403.json`, `undo_refused_403.json` — the last two captured in the final fix wave
  under R-F1, at a real turn boundary, because spec §8 named them and no task had carried them.

### 1.7 The signal seam, in its first form

`GET …/{id}/events?after=N` — seated, answered in a few bytes without building a frame, carrying
the event count, the seat to move, `your_turn` and the status. The client polls it at 3 s from a
VISIBLE tab only (R-P8); a hidden tab asks for nothing and, on return, shows a refreshing state,
asks once, and resumes. A run arriving through the signal WALKS whether its mover is a bot or a
person (R-9.1) — which is what makes a human opponent's turn play out on the table rather than
snap.

### 1.8 The client

- **Sign-in and the door.** `SignIn.tsx` + `whoami.ts`: the typed name is normalized by the
  server and stored under `yokohama.claimed_user`. A 401 from ANY call routes centrally through
  `api.ts`'s `onUnauthorized` (R-7.1) and returns to the door with the refusal sentence — one
  hook, registered once, so every present and future call site routes by construction.
- **The games list** is the tables where you sit, with the seats as names and a bot marked
  (R-7.2), newest first, showing whose turn it is.
- **The per-seat new-game form**: player count, and one row per seat set to me / another email /
  bot, with R-P10's sentence above it.
- **The dealing screen** (`Dealing.tsx`): the M5 set table over YOUR OWN two cards with the Keep
  score and the CLI's hedge under each, the technologies and the goals withheld until you choose,
  the other seats shown as chosen or waiting, and a poll that hands over when the table flips.
- **The walk for a person's turn**: `signal.ts`'s hook drives it; `hasOpponentStep(steps, viewer)`
  is the gate, and `TurnAccount` walks the run step by step at the cadence the browser remembers.
- Sign-out lives in the Game menu inside a game (R-8.1) and on the strip outside one: the M5
  one-row banner gate is a shipped measurement and it outranks a placement sentence.

### 1.9 The CLI touch, and its size

Two files, **24 insertions and 2 deletions**, behaviour byte-identical:

- `repl.rs` — `index_placements_for(me)` carries the existing body verbatim; `index_placements()`
  delegates at `current_player`. This is R-4.1, and it is how the Critical in §4.4 was fixed
  WITHOUT masking in the web crate, which spec §5.2 forbids.
- `wizard.rs` — `KEEP_SCORE_NOTE` became `pub` so the browser quotes it instead of copying it,
  closing one item on the M5.5 LEAVE list.

**The plan's Global Constraints said "`crates/yokohama-cli` moves by ONE line". It moved by 24.**
That was an estimate written before the Critical existed; the spec's binding constraint (§9.5) is
that the CLI's tests, goldens and `guide_sync` stay untouched, and they did — 850 + 2 ignored,
unmoved.

### 1.10 The deployment

`yokohama-web:0.5.0-m6.1` from `5be1e54`, with `YOKOHAMA_WEB_IDENTITY=claimed` in the container
`Env`. Verified from inside the container at 07:00Z and through the tunnel at 07:02Z once WARP
was back. The record, with every byte count and timestamp, is `deploy/RUNBOOK.md` §3.7; §3.2
gained the sentence that a Cloudflare challenge page from `curl` means WARP is off, not a failed
deploy.

---

## 2. The counts at the close HEAD

Re-run at `44dd538` in ONE uninterrupted sequence on the **Mac** (darwin 27.0.0, toolchain
1.97.1), not copied from any task's report. Wall figures are per-machine.

| suite | at the open (`7593857`) | at the close (`44dd538`) | wall |
|---|---|---|---|
| `cargo test -p yokohama-web` | **82** (76 lib + 1 gate + 5 wire) | **122** (115 lib + 0 main + **2** gate + 5 wire) | 37.9 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | 1.0 s (warm) |
| `cargo test -p yokohama-cli` | 850 + 2 ignored | **850 + 2 ignored** (822 lib + 9 + 18 + 1) | 28.0 s |
| `cd web && npm run typecheck` | clean | clean | 1.8 s |
| `cd web && npm test` | **393** in **24** files | **447** in **29** files | 6.2 s |
| `ls web/src/wire/*.json \| wc -l` | **44** | **54** | — |
| `cd web && npm run e2e` | 37 in 6 spec files* | **38** in **7** spec files | see below |

The exact lines the commands printed:

```
cargo test -p yokohama-web
  test result: ok. 115 passed; 0 failed; 0 ignored     (src/lib.rs)
  test result: ok. 0 passed; 0 failed; 0 ignored       (src/main.rs)
  test result: ok. 2 passed; 0 failed; 0 ignored       (tests/play_a_whole_game.rs, 13.27s)
  test result: ok. 5 passed; 0 failed; 0 ignored       (tests/wire_fixtures.rs, 10.10s)
  37.905 s total

cargo clippy --workspace --all-targets -- -D warnings
  Finished `dev` profile ... 0 warnings

YOKOHAMA_BGA_CORPUS_OPTIONAL=1 cargo test -p yokohama-cli
  test result: ok. 822 passed; 0 failed; 0 ignored     (src/lib.rs, 26.01s)
  test result: ok. 9 passed;   0 failed; 0 ignored     (tests/bga_import.rs)
  test result: ok. 18 passed;  0 failed; 0 ignored     (tests/guide_sync.rs)
  test result: ok. 1 passed;   0 failed; 2 ignored     (tests/piped_dogfood.rs)
  28.032 s total

npm run typecheck   →  tsc -b --noEmit && tsc -p tsconfig.node.json, clean
npm test            →  Test Files 29 passed (29) · Tests 447 passed (447) · 6.22s
ls web/src/wire/*.json | wc -l  →  54
```

\* The e2e row's open figure is `CLAUDE.md`'s, not a measurement: the browser walk was RED by
design from Task 3 through Task 8 and was not run at the baseline.

**The CLI command ran with `YOKOHAMA_BGA_CORPUS_OPTIONAL=1`**, because this worktree has no
`bridge-captures/` directory: all nine `bga_import` cells are corpus-gated and panic by name when
the captures are absent, so with the opt-out set that binary prints "9 passed" while asserting
nothing. That is the documented caveat, not a regression, and it means the CLI figure here is a
REGRESSION check on the 841 non-corpus cells rather than a full reading.

**The browser walk** — `npm run e2e`, run once against a fresh `--release` build:

```
game sim-a741d6484b4f3ace finished in 165 moves
  via option×77, placement×16, count×18, confirm×12, only×42
  ✓   6 e2e/plays_a_whole_game.spec.ts:25:1 › a whole two-player game plays through
        to a winner (14.4s)

two-browser game sim-4464836c83141f5e finished in 2402 moves, 1117 idle looks, 703s;
  largest hand seen 3
  ✓  38 e2e/two_browsers.spec.ts:65:1 › two people at two browsers play to a winner
        and neither sees the other's hand (11.8m)

  Slow test file: e2e/two_browsers.spec.ts (11.8m)
  38 passed (13.0m)
```

The `6` and the `38` on those two lines are Playwright's ordinal positions in the run, not counts:
the whole run is the `38 passed` on the last line, and `two_browsers.spec.ts` holds one test.

**Eight of the thirteen minutes are the two-browser cell's poll**, and its 2,402 "moves" are loop
iterations rather than applied moves — the loop counts a raced no-op the same as a move, so that
figure is NOT comparable with the vs-bot game's 165. Both readings are recorded, neither is
gated; §6 carries the fix that would shorten it without touching product code.

Thirty-seven of the thirty-eight drive real Chromium against the real release binary. The
thirty-eighth, `every_spec_can_skip.spec.ts`, declares no browser fixture, which is what lets it
run — and therefore check — in the very configuration it protects.

**The blast radius**, run at this HEAD against `master` (which IS a local ref in this worktree,
at `2533001`):

```
git diff --stat master -- crates/yokohama-core crates/yokohama-oracle \
                          crates/yokohama-difftest crates/yokohama-bridge crates/gen-data
  (empty)

git diff --numstat master -- crates/yokohama-cli
  22  1  crates/yokohama-cli/src/repl.rs
   2  1  crates/yokohama-cli/src/wizard.rs

SEMANTICS_REV = 7   (crates/yokohama-core/src/game/semantics.rs:180)
```

**The fifteen-gate release roster was NOT re-run, and the reason is the first line above**: every
crate the roster measures is byte-identical to `master`, so a re-run could only re-measure
master's own reading. Nothing added in this cycle is `#[ignore]`d; this lane still has no gate
and no roster place.

---

## 3. The corpus diff

`web/src/wire/` is a contract, not a snapshot. It was regenerated at Tasks 1, 3, 4, 6 and 10 and
in the final fix wave, and every diff was read. **44 files at the open, 54 at the close.**

| where | change | why |
|---|---|---|
| Task 1 | `health.json` re-cut (+`identity`); `me.json`, `me_signed_out.json` new | the mode is now a fact of the server; 43 gameplay payloads byte-identical |
| Task 3 | 7 new: `table_dealing.json`, `table.json`, `deal.json`, `deal_waiting.json`, `dealing_409.json`, `games.json`, `not_seated_403.json` | the table document and the per-seat deal; **every existing fixture byte-identical**, including the two previews — only the REQUEST body had changed |
| Task 4 | `hidden_hand.json` DELETED; 22 fixtures re-cut | the whole fixture diff is 24 `hand_hidden: false → true` lines: every `ClientState` fixture is now built for a NAMED seat, so the other seat is a count by construction and a single fixture pinning that property is redundant (R-plan-5) |
| Task 4, fix 1 | `index_orders.json`, `index_filtered.json` `location` values | **the leak was frozen in the corpus at `c6434bb`** — the fixture named both of the mover's cards to the other seat's holder — and corrected at `311c4a9`. Recorded here because a corpus can freeze a defect as faithfully as it freezes a fix |
| Task 6 | `events.json`, `events_dealing.json` new | the signal seam; nothing else moved |
| Task 10 | `preview.json`, `preview_bot.json` REMOVED | the route is gone. Git reports these as renames into `deal.json`/`deal_waiting.json` on content similarity; they are a removal and an addition |
| fix wave | `wrong_seat_403.json`, `undo_refused_403.json` new (R-F1) | spec §8 named them and no task had carried them |

The client's `contract.ts` and `contract.test.ts` moved with the corpus at each step, which is
the point of having two statements: a rename on the server fails `wire_fixtures`, a rename in the
client fails `tsc`.

---

## 4. The reviews

Four instruments, in the order they ran. Each found something the others could not.

### 4.1 The whole-branch code review (`final-code-review`, Fable, on `7593857..7fbaa64`)

Twenty commits, 84 files, +10,616/−4,218. Verdict **With fixes**: 0 Critical, **4 Important**,
12 Minor, and a "False sentences at HEAD" section of **25 bullets naming roughly 29 sites** —
every doc comment, `CLAUDE.md` line and test doc it found false by trying the mechanism the
sentence named.

The four Importants, and what was done with each:

1. **The setup seed could still reach the wire, through `persist`'s conflict sentence.** The one
   sentence on this server that printed a seed — `"{id} now holds a different game (stored seed
   {}, this handle's {})"` — rides into an `input` reply's `warnings`. Never in the corpus, so
   the value pin could not see it. **Fixed**: it prints neither number.
2. **`check_shape` bounded `players` and not `start_seat` or `keeps[i]`** — the same panic class
   R-3.2 was promoted for, on the same `table_lock` path. **Fixed**, with the cell arm.
3. **A `DELETE` racing the last keep left an orphan record and a name nobody could reuse.**
   `delete` took no `table_lock`, so a `finalize` in flight wrote the record AFTER both documents
   were unlinked. **Fixed**: `delete` takes the lock, and the cell arranges the real race with a
   two-gate store (the orphan was reproduced RED).
4. **The deployment documents said the opposite of what the server now does**, in the six files a
   deployer reads. **Fixed in Task 11**, which owns those files.

The fix wave (`5be1e54`) also carried the five MUST-FIX doc sentences, ten more false in-crate and
in-client sentences, and R-F1's two fixtures. Its re-review found all findings addressed with no
new breakage.

**It read the 24 rulings that had been taken by then and AGREED with all 24**, with two notes,
both taken: R-3.2 should have extended to `start_seat`/`keeps` (it does now), and R-8.2 is a spec
§7.3 DEVIATION rather than out-of-scope (§5 records it as one). **Six** more rulings were taken
AFTER it and are therefore unreviewed by it — R-F1, R-F2 and R-F3, which scoped the wave it asked
for, R-11.1 and R-12.1 at the deployment and this doc pass, and R-12.2 at this close's own fix
round.

### 4.2 The live review (`live-review-m61`, Fable, clean, two browsers)

Against `https://yokohama.moore.nyc` on `0.5.0-m6.1`, from the owner's Chrome and a Playwright
browser as two people, with a third identity for the stranger check. Verdict: **SHIP AS IS** —
0 Critical, 0 Important, **3 Minor**.

Verified by trying the mechanism, not by re-reading the sentence: R-1.1 through R-1.9; a
three-header probe returning only the configured mode's value; the per-seat deal with the
technologies and goals withheld; **no `seed` key and no 15-digit number on any of `/table`,
`/{id}`, `/deal`, `/events`, `/at/2`, `/ports`**; a human's turn WALKING on the other screen with
detection at 1.5–2.3 s; a mid-turn undo reloading the watcher WITHOUT a walk; undo after the turn
passed answering 403 in R-P6's words; the replay showing each browser its own hand at every `k`;
the stranger seeing an empty list and 403s; 0 page overflow at 1000/1200/1440/1500/1633; consoles
clean of JS exceptions.

The three Minors, all on the LEAVE list (§6):

1. The form's **NAME field is really the game id** and refuses a name with a space. A spec
   §4.1/§4.2 deviation (§5).
2. After **sign out**, the door briefly shows the raw 401 sentence above its prose. A fresh load
   shows only the prose.
3. **`why` on a human's last move** words the refusal as "no bot in this game" rather than "the
   last move was a human's". Correct for a bot-free table; recorded because it differs from the
   task's stated expectation.

One game, `m6.1-live-review`, was left on the NAS and should be deleted.

### 4.3 The mutation battery (`mutation-battery`, Opus, at `44dd538`)

Spec §9.4's eight mutations, applied to the LOCAL build one at a time, the named cells run, the
first failing assertion recorded verbatim, the file reverted. **8/8 caught.**

| # | mutation | caught by | time to fail |
|---|---|---|---|
| 1 | the extractor reads any of the three headers | 2 identity units + the 401 route cell | 0.01 s |
| 2a | `hand_hidden` false for a non-viewer HUMAN seat | 2 lib cells + 16 committed payloads | 9.4 s / 10.1 s |
| 2b | `hand_hidden: false` outright | 3 lib cells + the corpus | 9.4 s |
| 3 | `input` skips the acting-seat check | 3 lib cells | 9.4 s |
| 4 | the undo rule drops the current-player half | 1 lib cell | 9.4 s |
| 5a | `next_unkept_seat_of` returns my HIGHEST unkept seat | 2 lib cells | 9.4 s |
| 5b | `deal` serves every seat's cards | 5 lib cells + `deal.json` | 9.5 s / 10.1 s |
| 6 | `write_if_version(Some(v))` writes on a mismatch | 1 store unit cell | 9.5 s |
| 7 | the seed back on a payload under the key `nonce` | 3 lib VALUE cells + BOTH `wire_fixtures` cells | 9.5 s / 10.1 s |
| 8 | the e2e own-hand selector matches nothing | the spec's own anti-vacuity line, `two_browsers.spec.ts:113` | **3.8 s** into a test whose passing run is 13.4 minutes |

Mutation 2 was run in both forms, because the human-only form leaves green two cells that sit at
vs-bot tables. Mutation 5 was run as two sub-mutations. Mutation 4 is killed by the refusal
SENTENCE, not by the status — both answers are 403.

**Three predicted catchers did NOT fire, and each is a property rather than a gap.** They are
recorded because a predicted catcher that stays green is exactly the kind of sentence this lane
has been in the habit of writing about itself:

- **Mutation 3 — the two-identity gate arm stayed green, and the close must not cite it as a
  witness to the acting-seat rule.** The walk always posts as the seat whose turn it is, so a
  fence that refuses non-acting callers is never exercised by it. The gate is a whole-game
  reachability and masking claim, and that is all it is. `a_watcher_cannot_undo_a_finished_game`
  also stayed green, for a stated reason: R-P6's `undo_allowed` runs IN FRONT of the fence and
  refuses that watcher first. The mutation is the experiment proving the two checks are
  independent rather than one being the other's fallback.
- **Mutation 6 — both `persist` cells stayed green, so the close must not claim they cover the
  store's compare-and-swap.** `persist` performs its OWN digest compare before it ever calls the
  store. Two lines, two cells, no overlap.
- **Mutation 7 — the `table` unit cell did not fire**, because its fence searches the serialized
  text for the WORD "seed". A seed served under another key name walks through it. It is a good
  cell against a careless field name and blind to a deliberate one; the discriminating
  instruments are the three lib VALUE searches and the committed-corpus compare, which search for
  the value in decimal and hex.

### 4.4 The per-task reviews — the two that mattered most

Eleven tasks, each reviewed by a separate Opus reviewer. **Ten per-task fix rounds were
dispatched** — Tasks 1, 2, 3, 5, 7, 8, 9 and 10 once each and Task 4 twice — plus the
whole-branch fix wave of §4.1, and each round was re-reviewed before the next task started. Two
findings changed the shipped design:

- **The Critical at Task 4 — a seated watcher could read the acting seat's hand.**
  `card_index` passed `repl.index_placements()` unscoped, which marks `OwnHand` for
  `state.current_player`; a watcher opening `GET …/index/orders` saw the mover's hand, and
  subtracting the public takes names the hidden starting order. Fixed under R-4.1 by widening the
  CLI rather than masking in the web crate. It is the finding that justifies this cycle: a
  per-seat server with ONE unscoped projection is not a per-seat server.
- **The game-over `undo` hole**, found by the same task's re-reviewer after the Critical was
  closed: the `input` fence dropped at game over for every seated caller and `undo` is
  allowlisted, so a watcher could rewind a finished shared game — measured RED at 357 → 200
  events. Fixed under R-4.3, which is now the rule at every status.

- **The poisoned-lock class, found at Task 3** and promoted into its fix round rather than
  deferred: an unbounded `players` on a hand-edited table document panics inside a handler
  holding `table_lock`, and a poisoned mutex is a process-wide outage, not one bad request. The
  whole-branch review then found the same class twice more (§4.1, item 2). It is worth carrying
  as a shape: **on a path that holds a process-wide lock, every field read out of a stored
  document is an input.**

---

## 5. The rulings taken in execution, with their costs

**Thirty rulings**: twenty-five tabled below in the order they were taken, plus the five R-plan
rulings the plan itself carried, restated at the end. **Cost** is what is wrong if the ruling is
wrong.

**The whole-branch review read twenty-four of the thirty and would reverse none.** It saw the
nineteen taken during Tasks 0–10 plus the five R-plan rulings. **It never saw the other six**,
which were taken after it ran: R-F1, R-F2 and R-F3 scoped the fix wave it asked for, R-11.1 and
R-12.1 were taken at the deployment and at this doc pass, and R-12.2 at this close's own fix
round. Those six carry no external review beyond the Task 11 and Task 12 reviews of the commits
that landed them.

### Pre-flight (the conflict scan)

| # | ruling | cost if wrong |
|---|---|---|
| R-0.1 | Task 0's baselines are measured and ledgered by the orchestrator, not an implementer — the counts are deterministic | none |
| R-0.2 | Task 4's stranger-403 sweep omits `/events` (created at Task 6); Task 6 adds it to that list | none — the property is asserted either way |
| R-0.3 | `Games::store()` is a plain `pub fn`, not `#[cfg(test)]`, so Task 10's integration test can reach it | one extra public accessor on a crate-internal type |

### Task 1 — identity

| # | ruling | cost if wrong |
|---|---|---|
| Task 1 | Spec §2.1 says "`AppState::new()` reads the variable"; the built shape is `AppState::new(mode)` with `main.rs` reading it — **a deliberate deviation**: no environment read inside a library constructor, and the mode is injectable for the mode cells | none; behaviour identical at the process boundary |

### Task 2 — the store

| # | ruling | cost if wrong |
|---|---|---|
| R-2.1 | `persist` propagates a store READ error through its non-evicting path (spec §3.2: a Conflict evicts, an Io does not) | a transient read failure surfaces as "APPLIED BUT NOT SAVED" on one request instead of a silent retry — the honest outcome anyway |
| R-2.2 | The `<id>.table.json` collision is fixed by the GUARD (`is_safe_id` refuses a `.table` suffix), not by per-`Doc` subdirectories | one reserved suffix in the client-nameable id space. **Carried**: a THIRD document kind needs a second reservation and the derivation assert would not notice — M6.4's chat document is where that bites |

### Task 3 — tables

| # | ruling | cost if wrong |
|---|---|---|
| R-3.1 | `list_for` logs and skips an unparseable table document, naming the id; the plan's "skip with no error" stands for the LIST but not for the silence | one log line per corrupt table per list call |
| R-3.2 | The `players` bound is PROMOTED from a Minor into the fix round: an unbounded field panics under `table_lock` and poisons the mutex | one extra fence check per read. **Extended at the whole-branch review to `start_seat` and `keeps[i]`** |

### Task 4 — per-seat views

| # | ruling | cost if wrong |
|---|---|---|
| R-4.1 | The Critical's fix is a behaviour-preserving CLI widening (`index_placements_for`), because masking in the web crate is what spec §5.2 forbids and §9.5's binding constraint is that the CLI's tests, goldens and `guide_sync` stay untouched | one extra `pub fn` on `Repl`; CLI behaviour byte-identical by delegation |
| R-4.2 | `at/{k}` "stable per caller" means "never follows another PERSON's seat"; a hot-seat caller's viewer moves only among their own seats | none — no other person's hand is shown either way |
| R-4.3 | **The acting-seat fence never drops for a non-acting caller at any status**; at game over and lockdown only the 403's wording changes | a watcher at a finished game sees a 403 instead of a 200-with-message; nothing mutates either way |

### Task 5 — the grammar and undo

| # | ruling | cost if wrong |
|---|---|---|
| R-5.1 | The undo rule is checked BEFORE the acting-seat fence, so an undo attempt is always answered in R-P6's words | a non-acting caller's undo reads the R-P6 sentence instead of the turn sentence; both true, both 403, nothing mutates |
| R-5.2 | At a table with more than one distinct person, undo at a FINISHED game is refused with the status sentence; the one-person branch keeps today's CLI undo | a player who misclicked the final move at a shared table cannot take it back alone. **A mutual-consent undo is a later cycle's** |

### Task 7 — the door

| # | ruling | cost if wrong |
|---|---|---|
| R-7.1 | The 401 is routed CENTRALLY — `api.ts`'s `onUnauthorized`, invoked by `call()` — rather than threaded through every screen; the plan's per-site threading was its letter, the spec's "any call" its requirement | one module-level hook a test must register and clear |
| R-7.2 | §7.2's "seats as names with the bot marked" is built now; the brief had omitted the sentence | one small shared row component |
| R-7.3 | An "over" table listing with the WINNER's name is DEFERRED — `TableView` carries no winner and adding one is a server/wire change mid-client task | one §7.2 sentence unmet this cycle, visible only in the list. **LEAVE (§6)** |

### Task 8 — the form and the dealing screen

| # | ruling | cost if wrong |
|---|---|---|
| R-8.1 | Sign-out moves into the Game menu inside a game; the strip keeps "You are <email>". The M5 one-row banner gate is a shipped measurement and outranks a placement sentence | one extra click to sign out |
| R-8.2 | **Recorded as a spec §7.3 DEVIATION** (R-F3), not as out-of-scope: §7.3 lists the FORM's fields as "Player count, colours, start seat, name, and one row per seat", and the form derives `colors` and posts `start_seat: 0` exactly as the pre-M6.1 form did | a browser player cannot pick a colour or the starting seat, and **at a vs-bot table the human always starts** — unchanged from M5.5. **LEAVE (§6)** |

### Task 9 — the poll

| # | ruling | cost if wrong |
|---|---|---|
| R-9.1 | A run arriving THROUGH THE SIGNAL walks whether its mover is a bot or a person; the `send` path keeps `hasBotStep` | a person's turn plays back step by step where a snap might have been wanted; the cadence controls exist to skip it |

### The fix wave

| # | ruling | cost if wrong |
|---|---|---|
| R-F1 | The wave also captures spec §8's two uncaptured fixtures `wrong_seat_403.json`/`undo_refused_403.json` | two corpus files and two contract rows |
| R-F2 | Wave scope = the three server Importants + five must-fix doc sentences + the in-crate/in-client false sentences + R-F1; the deploy files fold into Task 11 and `CLAUDE.md` into Task 12 | none — the same edits, in the commits that own those files |
| R-F3 | R-8.2 is re-labelled a spec §7.3 deviation for this record | none |

### Task 11 and 12

| # | ruling | cost if wrong |
|---|---|---|
| R-11.1 | The one Important on the Worker comment (`index.ts:94-95`) folds into Task 12's doc commit rather than a separate fix round | a false comment lives one commit longer on an undeployed path |
| R-12.1 | Task 12's README scope is WIDENED from "the create flow" to the whole web section — `:252`, `:270-278` and `:293-311` still said "no authentication of any kind" and named `0.5.0-m5.5.4` as live | none |
| R-12.2 | This close's own review findings are fixed by AMENDING its commit in place rather than by a follow-up commit, so the append-only guidingpath row lands right the first time and no same-day forward correction is needed | a rewritten SHA on a branch only this worktree holds, unmerged and unpushed |

### Restated from the plan (R-plan-1 … R-plan-5)

- **R-plan-1** `created_at` is unix seconds, not a formatted string — a sort over an ISO string
  works until a timezone or a fractional second appears in one.
- **R-plan-2** the claimed header is attached whenever a name is stored; the server ignores it
  under the other modes, pinned.
- **R-plan-3** the table document mirrors `to_move`/`over`; one store read per persisted move
  beats a replay per list row, and a lost mirror write is ignored by design.
- **R-plan-4** `preview` survives to Task 10, so the client is never without a setup screen.
- **R-plan-5** `hidden_hand.json` retires at Task 4; the property it pinned moved to the
  corpus-wide mask row.

### Deviations the ledger did not name, recorded here

- **Spec §4.1/§4.2 — the game's `name` and `id` are one field.** The spec keeps them distinct;
  the client posts the typed name as both, so it must satisfy `store::is_safe_id` and a name with
  a space is refused. Found by the live review. **LEAVE (§6)**.
- **Spec §7.3 — R-8.2 above.**
- **Spec §8 — `wrong_seat_403.json` and `undo_refused_403.json`** were absent until the fix wave
  captured them under R-F1.
- **Spec §2.1 — `AppState::new(mode)`**, the Task 1 ruling above.
- **The plan's "the CLI moves by ONE line" — it moved by 24** (§1.9).

---

## 6. The LEAVE list

### 6.1 Inherited from the M6 handover §4 and not touched by this cycle

From the M5.5 close's §7 (handover §4.1):

- The **Game menu stays open** after a game is chosen from it.
- The **tile popover at 390 px** covers most of the screen.
- **Above 1201 px nothing scrolls the account's lit row into view** (R-F13), with R-F11's
  constraint on any fix: scroll the nearest scrollable ancestor that is not the document, by a
  computed `scrollTop`, **never `scrollIntoView`**.
- The **human's own closing event walks as step 1** (R-F7), and the **opening-run walk is
  unreachable from the wizard** (R-F8).
- **`useSections()` is a footgun by shape**: a third caller repeats the defect R-T12-4 fixed.
- The **whole-game server gate is OS-entropy seeded and has capped**: a 2P game the bot cannot
  end. **This cycle adds a measurement to it**: the gate's wall varied 13.30 s / 857.06 s /
  15.72 s across runs of identical code, and the 14-minute process was sampled and found doing
  real bot search with zero replay frames — NOT a rebuild loop. Note it wherever the gate runs
  under a timeout.
- **M5.5's own forty-four deferred minors**, in the gitignored archive
  `docs/temp/sdd-archive/2026-09-04-web-frontend-m5.5-plan.tar.gz` — **which is in the MAIN
  checkout, not this worktree**, `docs/temp/` being gitignored and therefore per-checkout.

From the corrections wave (handover §4.2):

- **R-C11 — the asking seat's label is half-clipped after the pass at 1000×563.**
- **The 1200/1201 strip-card artifact** — a strip card is WIDER at 1200 px (144) than at 1201 px
  (128), and no cell sits at that pixel.
- **The mobile stack does not scale** — a layout question, not a defect; 0 overflow at every
  width from 320 to 2560.
- **Nine deferred minors** itemised in that record's §5.7.

Two inherited items are now CLOSED by this cycle, and are struck from the list rather than
carried: **`POST /api/games/preview` built a full session per call, unauthenticated at the
origin** — the route is gone; and the set table's **Keep hedge was an unpinned copy** of the
CLI's `KEEP_SCORE_NOTE` — the constant is `pub` and the server serves it.

### 6.2 This cycle's own

**Product and layout**

- **The game's NAME is its id** and refuses a name with a space (live review Minor 1; spec
  §4.1/§4.2 deviation). A display label beside a `sim-…` id is the fix.
- **The sign-out transition shows the raw 401 sentence** above the door's prose (live review
  Minor 2); a fresh load does not. `onSignedOut` clearing the notice closes it.
- **Colours and the start seat are not selectable in the form** (R-8.2); at a vs-bot table the
  human always starts.
- **An "over" table lists without the winner's name** (R-7.3). Two integers on `TurnMirror`.
- **The 1024 px banner is two rows again** (101 px), and during an outage the "reconnecting…"
  strip wraps the banner at 1280 (22 px of room against a ~75 px strip) — untested in either
  direction, rendered only when not live.

**Client**

- **`Dealing` never re-reads `/deal` while waiting**, so "waiting on b, c" stays stale at a 3P+
  table until the flip. One `fetchDeal()` on a `dealing` tick.
- **A DELETED game reads "reconnecting…" forever** — `useSignal` treats 404 and 403 from
  `/events` as an outage. Route those two statuses to leave the game.
- **No `path !== "/api/me"` guard** on the central 401 hook. Unreachable today (`/api/me` never
  refuses, pinned), cheap insurance.
- `KeepRequest` is declared and read by nothing; `.deal-pick.chosen` is dead CSS; two stale e2e
  comments; the "You are" strip is unrendered in two unit hosts; no cell clicks the moved
  sign-out button; a 404ing saved row stays in the list; `App.tsx` is ~1,010 lines.

**Server**

- **`table_lock` is held across `finalize`'s bot pump and across a repair rebuild.** Correctness
  is fine and there is no lock-order cycle; while the bot searches its opening turn, every
  `create` and `keep` at every table waits. Write the keep under the lock and finalize outside it
  when the lobby makes the wait visible.
- **`undo_allowed` maps a `ReplayError` to a 403** — a record that will not replay is the
  server's fault; 500 is the honest status. It also replays `len-1` events per attempt.
- **The waiting caller's set table is built for `first_human_seat`**, which can name another
  person's seat. No card leaks (every human hand is `Unknown` on that projection), but
  `deal.frame.seat` says `0` to blue. `seats_of(user)[0]` removes the function.
- **`create` reads a store `Err` as OCCUPIED** and answers "already exists", discarding the I/O
  sentence. A log line would help an operator.
- **`games.rs` is 2,653 lines** (~1,300 of tests). The seam is clean and worth cutting when M6.2
  touches it: table lifecycle versus record lifecycle.
- **`the_create_flow_…` asserts the stranger's list is 200 and not EMPTY** — one `assert_eq!`
  short; the Games-layer cell covers emptiness.

**Tests and tooling**

- **The two-browser cell costs 11.8 of the walk's 13.0 minutes** at the close HEAD (§2), most of
  that the 3 s poll. The fix that does not touch product code: after each move, dispatch
  `visibilitychange` on the WATCHING page — the hook's return path asks at once, so a turn
  boundary costs one round trip instead of a poll interval, and R-P8's return path gets exercised
  in a real browser as a bonus. Its loop counts raced no-ops as moves (**2,402** iterations at the
  close HEAD, not comparable with the vs-bot game's **165**; Task 10's run read 2,356 against 180,
  and both pairs move with the server-drawn seed); the stuck-escape fires on any repeated prompt
  and never resets; `hasMove` belongs in `harness.ts`.
- **No committed fixture is a deal at a BOT table**, so the bot-deal key-set fence lives in four
  places and none of them is a corpus file.

**Deployment**

- **`deploy/modal_app.py` and both Dockerfiles set no `YOKOHAMA_WEB_IDENTITY`** by design — an
  image started without the container `Env` exits 1 (R-1.3). The Modal path is LEAVE; the runbook
  says the container `Env` supplies it.
- **The Worker rewrites the origin's 401/403 into a 502** on the undeployed Modal path, recorded
  in-file as a known defect.
- **`deploy/RUNBOOK.md` §3.5/§3.6 still say "The live build is …" in the present tense**, which
  is now two builds stale.

### 6.3 Closed as WRONG or obsolete

Seven deferred minors were carried through the ledger and are closed here rather than inherited,
because the whole-branch review found them already false at HEAD: Task 3's `NewGame.tsx`
`keeps()` note (the screen was replaced); Task 4's `SuggestRefusal::BotSeat` note (the variant's
doc now carries it); Task 5's lockdown asymmetry and its `lib.rs:533-534` unreachable closure
(both superseded by `1a978cb`); Task 6's untested `over` (Task 10's arm asserts it); Task 7's
duplicated row markup (`TableFacts.tsx` closed it); Task 8's `api.preview` dead code (removed in
Task 10).

---

## 7. What this cycle does NOT claim

Spec §11, restated at the close HEAD, with what changed marked.

- **No lobby, no join links, no display names, no spectators, no transport beyond the poll, no
  chat, no hosting.** M6.2–M6.5. Holds.
- **No JWT verification.** `cf-access` trusts the tunnel's header for the reason spec §2.4 gives;
  `claimed` is **not a security boundary** (R-1.4) and the client's own door prose says so. Holds,
  and it is the sentence to keep reading whenever someone calls this "authentication".
- **Other seats' emails are visible to seated players.** No per-seat privacy of who is at the
  table. Holds.
- **No rate limiting on `/deal`** — one throwaway setup per call, seated callers only. Holds.
- **Hot-seat secrecy at one browser rests on one person choosing to sign out and in.** The server
  cannot tell two seats held by one email apart, by design. Holds. What DID improve: the deal is
  now per seat rather than per screen, so M5.5's "the payload still carries every human seat's
  cards" is retired.
- **No size or wall figure for any of this.** Holds; §2's figures are per-machine readings, not
  budgets.
- **Every count, line number and route name in the spec is at `2533001`.** The plan re-read them
  at its own HEAD and this close re-measured at `44dd538`.

Added at the close:

- **The acting-seat rule is NOT witnessed by the two-identity gate arm** (§4.3, mutation 3). The
  gate is a whole-game reachability and masking claim. Three lib cells are the witnesses.
- **The `persist` cells do NOT cover the store's compare-and-swap** (§4.3, mutation 6). Two
  independent lines, one cell each.
- **The corpus contains no deal at a bot table**, so the bot-deal absence is pinned by the struct
  shape, a by-value cell, a masked-row count and the client's key sets — not by a fixture.
- **The five deviations of §5** — the name-as-id, the form's missing colour and start-seat
  fields, spec §8's two fixtures absent until the fix wave, spec §2.1's constructor sentence, and
  the CLI's 24 lines against the plan's one.
- **The CLI figure in §2 is a regression check on 841 cells, not a full reading**: this worktree
  has no BGA captures, so the **nine** corpus-gated `bga_import` cells assert nothing under the
  opt-out (850 − 9 = 841). `CLAUDE.md`'s "FOURTEEN pins" is the larger count, which adds the five
  in `feed_preprocess` — those are in `yokohama-bridge`, a crate this command does not run.

---

## 8. The handover into M6.2 — the lobby

From the room plan §2, verbatim as the charter:

> **Builds:** open tables and join links; taking a seat; start when full; a display name beside
> the email; "my games" with whose turn it is (the R-P3 requirement); abandon by the creator. The
> front door proper: the first screen a person sees is the lobby, not a form.
> **Inherits:** M6.1's table document (gains `open` before `dealing`), the identity seam (join
> links are the first place a person arrives without being named in advance), the bot-at-2P rule.
> Also inherits from the M6 handover's LEAVE list: the unbounded saved-game list (becomes the
> lobby's paging) and the "New game" in-flight guard.
> **Exit:** someone holding only a link joins a table and plays.

What M6.1 built that M6.2 starts from:

- **`TableDoc` is where `open` goes**, as a third `TableStatus` before `Dealing`. `check_shape`
  is the fence every stored field already passes through, `schema` is already on the document,
  and `status` is already what every route branches on. The rule from §4.4 rides with it: on the
  `table_lock` path, every field read out of a stored document is an input.
- **`Identity` is the seam a join link lands on.** A person arriving on a link is the first
  caller who is not named in the create body, so `SeatDoc::Human { user }` gets written at JOIN
  time rather than at create time — and `User::parse`'s single normalization is what makes the
  seat the same seat on their next request. A display name is a second field beside `user`, never
  a replacement for it: `user` is the key.
- **The bot rule (R-P10)** is already refused at create with the evaluator's own sentence; a
  lobby that lets someone add a bot seat to an open table refuses in the same place.
- **The signal seam's polled form** already answers "this game now has N events, seat S to move"
  in a few bytes without building a frame. A lobby that shows whose turn it is at twenty tables
  wants the same shape over the LIST, not twenty polls — which is what `TurnMirror` on the table
  document was put there for.
- **The `.table` suffix reservation carries to any new `Doc`.** M6.2 adds no third document kind;
  M6.4's chat does, and the derivation assert checks only `Table`. Whoever adds the third kind
  reserves its suffix in the same commit.
- **`Games::list_for` is the lobby's other half already written**: it reads table documents, skips
  and logs an unparseable one, and never rebuilds a session. Paging it is the M6 handover's
  unbounded-saved-game-list item, which §6.1 hands over.

Also carried into M6.2's planning: the LEAVE list above (§6), and the process (R-P11 / M6
handover §5) unchanged — Opus implementers, Opus per-task reviewers, one clean final review with
no prior context against the LIVE site in both layouts, the SDD workspace convention, explicit
`git add` paths, the five fenced crates byte-identical and `SEMANTICS_REV` at 7.
