# Web frontend — M6.2 "the lobby": the close

**Written 2026-09-16 on branch `web-room-m6.2` at the close HEAD named in §2**, cut from
`web-room`'s tip `937ce92` (M6.1, PR #31 against `master`, unmerged): five planning commits
(`48ae908` the CI change, `9422a1d` the two specs, `64a21dc` and `5489705` the two plans,
`bd02a06` the handover), then **twenty commits of execution** — seventeen through the
whole-branch review's fix wave at `a214a19`, the deployment record `1330a21`, the live-review
fix `8dc6316`, and this close.
The spec and the plan are dated 2026-09-15, the day they were written; the cycle was executed
in one session that ran from that evening into this morning, so the close carries the day it
was written, as the M6.1 close did.

The authority for everything below is the spec,
`docs/plans/2026-09-15-web-frontend-m6.2-lobby-spec.md`, under the eleven owner rulings
R-P1–R-P11 in `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0 and the spec's own
R-2.1–R-2.11. The plan `docs/plans/2026-09-15-web-frontend-m6.2-plan.md` is what was executed.
Both are frozen; where a sentence in either is wrong at this HEAD, §5 and §7 say so here rather
than editing them.

The live deployment is **`yokohama-web:0.5.0-m6.2.1`, built from `8dc6316`** (the live-review
fix; its server half is `ad29715`, unchanged since `0.5.0-m6.2`), swapped onto the NAS at
02:47Z on 2026-09-16 after `0.5.0-m6.2` from `a214a19` had been swapped at 02:07Z and verified
through the tunnel at 02:07:46Z and by the spec's own §9 verification at 02:08Z —
`deploy/RUNBOOK.md` §3.8 is that record and is not restated here.

**This document is M6.3's entry artifact** (§8).

---

## 1. What shipped

The exit criterion of the room plan's M6.2 line holds on the NAS: someone holding only a link
joins a table and plays. On 2026-09-16 at 02:08Z a second identity, sent nothing but
`#/t/deploy-verify-m62` and a password, sat down at a table the first identity had opened, and
the table dealt.

### 1.1 The cut (R-2.10)

`games.rs` was cut along its seam BEFORE any behaviour moved: `crates/yokohama-web/src/tables.rs`
now holds the table lifecycle (create, join, deal, keep, settle, finalize, list, table, delete)
and `games.rs` the record lifecycle (handle, live handle, persist, the turn mirror, eviction).
No function body changed — the reviewer extracted all 73 bodies before and after and diffed
them — and the 26 tests moved with their subjects (10 stayed, 16 moved). Five methods and the
nine shared test helpers became `pub(crate)` because their callers crossed the new module line;
the crate's public surface is identical.

### 1.2 The table document, widened (spec §2)

`SeatDoc` gains `Open` and a per-seat `name: Option<String>`; `TableStatus` gains `Open`
before `Dealing`; `TurnMirror` gains `winner: Option<u8>`, read from the ENGINE's
`final_result().winner` (the same value `/result` projects as `Placing.won`) and never from
`standings[0]`; `TableDoc` gains `lock: Option<Lock>` — hex of 16 OS-entropy bytes and hex
SHA-256(salt ‖ password), compared by a fold of XORs; `TABLE_SCHEMA` is **2**, and a document at
schema 1 reads with every new field at its default (measured on the live volume, §1.10).
`check_shape` gains, in order: schema ≤ 2; an `Open` seat only while `status == Open`;
`status == Open` only with an `Open` seat; the lock's 32/64 hex lengths; every seat name 1–64
characters with no control character; `winner < players`; and — from the whole-branch review —
**`to_move < players`**, the one stored field handed to `PlayerId::new` that M6.1's own rule
("every field read out of a stored document is an input") had not yet bounded. `sha2 = "0.10"`
enters the web crate with its paragraph (R-2.8); nothing else is added to the tree.

### 1.3 Create, re-shaped (spec §3.1)

`POST /api/games` takes `{"user": …}`, `{"bot": true}` or `{"open": true}` per seat; `name` is
a LABEL (free text, 1–64 characters, spaces allowed) and the id is drawn as `sim-<16 hex>`
unless the request supplies one under `is_safe_id` (the fixture harness and the lib cells do);
`display_name` is written onto every seat the creator holds; `password` is trimmed, refused
empty or over 64, and stored as a `Lock`. The refusal order is the spec's, with the four new
arms after the creator and bot checks: `name`, `display_name`, `password`, `id`; it is PINNED by
six probes wrong in two ways at adjacent links, because the first review found that single-defect
probes pin nothing about order. A table with any open seat lands `open`; a full one deals at once,
M6.1's path byte-for-byte. Beside `GameError::Dealing` stand `Open` ("this table is still
open") and `Playing` ("this table is playing"), each a 409 carrying `status`.

### 1.4 Join, and the lock split (spec §3.2, §3.5)

`POST …/{id}/join {seat, name?, password?}` fills one open seat under the TABLE's own lock,
refused in the order a client reads: 404; 409 with the status at dealing/playing; 400 seat out
of range; 400 name; 409 "seat N is taken" (a `Human` or `Bot` seat — a seat the creator named by
email is taken, never joined); 403 "this table is locked — send its password"; 403 "wrong
password for this table". The join that fills the last seat flips the table to `dealing` in the
SAME write (R-2.2; the store's write count is asserted to be 1). `Games.locks` hands out one
mutex per table (R-2.9): `join`, `keep`, `settle`, `finalize`, `flip_to_playing` and `delete`
hold it; `create` alone keeps the global `table_lock`, because a drawn id is not yet "one
table". A finalize at one table no longer blocks a join at another — a two-gate store parks A's
record write inside finalize and B's join answers before the gate opens.

**The split cost one mutual exclusion the spec's sentence had promised was free.** Spec §3.5
says "the M6.1 race cells are the proof that nothing was lost"; the Task 4 review showed that
none of the four is create-versus-delete, and that with `create` global and `delete` per-table a
same-id delete could unlink both documents, let a create read them absent and write a new table,
and THEN tombstone the id — leaving the new game refusing every move as "deleted while in
flight". The fix (ruling #6) puts the tombstone down BEFORE the unlink, removes it again if the
unlink fails, and is arranged as a race over the gated store
(`a_delete_cannot_tombstone_the_table_a_create_put_in_its_place`), red on the pre-fix ordering.

`keep` gains `name`, written only when the seat's is `None` (R-2.3), and — a finding of the
Task 5 sweep, ruling #8 — checks the SEAT before the STATUS, so a stranger's keep is a 403 at
every status and learns nothing about the table.

### 1.5 The listing and the one widening (spec §3.3, R-2.7)

`GET /api/games?scope=mine|open&limit=N&before=<created_at>,<id>` reads table documents ALONE
(the cell arms a record reader that panics, against a PLAYING table with a record on the store),
sorts `(created_at desc, id asc)`, pages strictly-after the cursor with no gap and no overlap
(pinned at five same-second rows), and answers `{ games, next }`. `limit` defaults 20; 0 and
above 100 are 400 in the handler AND guarded in `Games::list`. `GET …/{id}/table` at `open`
answers any signed-in caller with `my_seats: []` — the only widening of R-1.5: `seated()` still
reads `Games::table`, and the stranger sweep, one two-dimensional cell over the router's
seventeen `/api/games/{id}…` routes at three statuses, asserts the two exceptions by an explicit
allowlist and that a stranger's refusal on every other route is the 403 and never a 409.

### 1.6 The wire (spec §7)

The corpus went **54 → 62** (§3). The walk now creates `wire` with one open seat and the
password `sesame`, a stranger reads it, a second identity is refused with the wrong password and
at a taken seat, joins with the right one, and the existing dealing steps see the flip. The
value pin `no_payload_of_this_run_carries_the_password_or_its_hash` reads THIS run's salt and hash
out of the memory store and searches every payload for the password's text, the wrong attempt's
text (`WIRE_WRONG_PASSWORD`, a needle that shares no substring with the password — the first
draft's `"not-sesame-9f3a"` did, and reported one leak as two), the salt and the hash; the
anti-vacuity partner is that the document carries both. The by-key cell in `table.rs` binds the
same values since the whole-branch review, so spec §8.4's "both catchers fire" is true (it was
measured false before that fix, §4.3).

### 1.7 The client (spec §5)

`web/src/route.ts`: `#/` the lobby, `#/new` the form, `#/t/<id>` a table by its status;
anything else the lobby. `usePolled(source, active, onValue, onEnd?)` with `Source { key, once,
stream }` owns the visibility rule, the 3 s cadence and the backoff; its effect depends on
`key` and `active` alone; `useSignal` is its first instance, the dealing screen, the open table
and the lobby the other three; a 404/403 ends the poll through `onEnd` — the M6.1 LEAVE item "a
deleted game reads reconnecting… forever" is closed for the one-shot half. The lobby is the
first screen: Your games (rows are LINKS — ruling #14 — with the seats, the state word and a
"more" control) and Open tables (with a lock marker), one `usePolled` for both scopes
(R-plan-4). The form lost its saved list and gained open seats, "Your name" (remembered in
`localStorage["yokohama.display_name"]`) and "Ask for a password to join"; its submit reads
"Open table" when a seat is open. `OpenTable` has the two faces of §5.4; hand-over is by
STATUS (a poll tick or a 409 may say `playing`, not only `dealing`). One `Abandon` component —
two-click, in-flight-guarded — serves both the open table (the seated creator) and the lobby's
Your-games rows, where the creator's delete was RE-HOMED (ruling #13) after retiring the saved
list had silently removed the client's only delete control. The keep carries the remembered
name. The help panel describes all of it (a whole-branch-review finding: it still described the
start-screen list).

### 1.8 The gates that play (spec §8.3)

The two-identity whole-game arm now arrives through the lobby: red opens a locked 2P table,
blue finds it under `scope=open`, is refused without the password, joins with it and a name,
and the game plays to a winner with the same masking assertions as before. `joins_by_link.spec.ts`
has a second browser arrive by the address alone, refuse on the wrong password, sit, and play
six moves while the creator's screen learns without a reload — and `framenavigated` was
MEASURED to fire on hash changes too, so that assertion is stronger than the spec asked and was
proven with a reload mutation.

### 1.9 The fences held

`crates/yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge` and
`gen-data` are byte-identical to `master` at the close HEAD, and `crates/yokohama-cli` is
byte-identical to `web-room`'s tip `937ce92` — against `master` it shows M6.1's own 24/2
lines, because this branch is stacked on `web-room` (§2); `SEMANTICS_REV` is **7**; no bot
behaviour moved. This cycle touched the CLI nowhere, as its plan's blast-radius line required,
and its suites read the full 850 + 2 with the BGA captures present.

### 1.10 The deployment

`yokohama-web:0.5.0-m6.2` from `a214a19`, `deploy/RUNBOOK.md` §3.8. Two readings the record
carries that a re-run cannot: the one schema-1 table document on the volume
(`m6.1-live-review`, M6.1's own review game) was read through the NEW server as its creator —
it listed and opened with every M6.2 field at its default — and only then deleted, as the M6.1
close had asked (ruling #15); and the spec's §9 verification ran against the live site: 201
open and locked with the creator's display name, a stranger listing it under `scope=open`, 403
on the wrong password, 200 `dealing` with both names on the right one.

---

## 2. The counts at the close HEAD

Re-run at `1330a21` (the deployment-record commit; the code HEAD is `a214a19`) in ONE
uninterrupted sequence on the **Mac** (darwin 27.0.0, toolchain 1.97.1), not copied from any
task's report, with the BGA captures present so the CLI figure is a FULL reading. The browser
walk was run separately, after the live review's own Chromium had finished (two Chromium sets
on one machine contend, and the two-browser cell's timeouts are real). Wall figures are
per-machine.

| suite | at the open (`bd02a06`) | at the close (`1330a21`) | wall |
|---|---|---|---|
| `cargo test -p yokohama-web` | **122** (115 lib + 0 main + 2 gate + 5 wire) | **149** (**141** lib + 0 main + **2** gate + **6** wire) | 37.2 s (the gate's game 10.5 s this time) |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | 1.1 s (warm) |
| `cargo test -p yokohama-cli` | 850 + 2 ignored | **850 + 2 ignored** (822 lib + 9 `bga_import` + 18 `guide_sync` + 1 `piped_dogfood`), captures PRESENT | 35.4 s |
| `cd web && npm run typecheck` | clean | clean | 2.1 s |
| `cd web && npm test` | **447** in **29** files | **522** in **33** files | 7.0 s |
| `ls web/src/wire/*.json \| wc -l` | **54** | **62** | — |
| `cd web && npm run e2e` | 38 in 7 spec files | **39** in **8** spec files | 8.7 m (two-browser cell 7.4 m) |

The exact lines the commands printed:

```
HEAD 1330a21 2026-09-16T02:16:45Z
cargo test -p yokohama-web
  test result: ok. 141 passed; 0 failed; 0 ignored     (src/lib.rs, 10.17s)
  test result: ok. 0 passed;   0 failed; 0 ignored     (src/main.rs)
  test result: ok. 2 passed;   0 failed; 0 ignored     (tests/play_a_whole_game.rs, 10.50s)
  test result: ok. 6 passed;   0 failed; 0 ignored     (tests/wire_fixtures.rs, 10.72s)
  37.183 s total

cargo clippy --workspace --all-targets -- -D warnings
  Finished `dev` profile ... 1.127 s, 0 warnings

cargo test -p yokohama-cli
  test result: ok. 822 passed; 0 failed; 0 ignored     (src/lib.rs, 26.47s)
  test result: ok. 9 passed;   0 failed; 0 ignored     (tests/bga_import.rs, 3.16s — the captures are present)
  test result: ok. 18 passed;  0 failed; 0 ignored     (tests/guide_sync.rs)
  test result: ok. 1 passed;   0 failed; 2 ignored     (tests/piped_dogfood.rs)
  35.375 s total

npm run typecheck   →  tsc -b --noEmit && tsc -p tsconfig.node.json, clean
npm test            →  Test Files 33 passed (33) · Tests 522 passed (522) · 7.0 s
ls web/src/wire/*.json | wc -l  →  62
```

The lib grew by 26 cells (115 → 141): Task 2's six, Task 3's four, Task 4's nine, Task 5's
five plus the route-level pager, Task 6's none (its pin is in `wire_fixtures`, 5 → 6), the fix
waves' none (they extended existing cells). The client grew by 75 (447 → 522) in four new files
(`route.test.ts`, `Lobby.test.tsx`, `OpenTable.test.tsx`, `Abandon.test.tsx`).

**The browser walk** — `npm run e2e`, run once at `8dc6316` (the live-review fix, the last
code commit) against the built release server, by that fix's implementer as its gate and taken
here as the close's reading (the docs commits after it change no code):

```
  ✓  39 e2e/two_browsers.spec.ts:65:1 › two people at two browsers play to a winner
        and neither sees the other's hand (7.4m)
        (440 s in-test; 1,333 loop iterations, 749 idle looks)
  joins_by_link.spec.ts: 6 moves across two browsers in 7.1 s; the seated open table at
        390 px reads 0 px page overflow

  Slow test file: e2e/two_browsers.spec.ts (7.4m)
  39 passed (8.7m)
```

The `39` on the first line is Playwright's ordinal, not a count; the run is the `39 passed`.
The two-browser cell's wall across this cycle's runs read 17.5 / 12.9 / 7.6 / 7.4 min — the
SERVER draws the seed, so the wall is the game's length — and its "moves" are loop iterations,
not applied moves (§6.1). Thirty-eight of the thirty-nine drive real Chromium against the real
release binary; `every_spec_can_skip.spec.ts` is the structural one, and it counted the new
file.

**The blast radius**, run at this HEAD against `master` (a local ref, at `abe4eec`'s lineage —
the web lane's merge):

```
git diff --stat master -- crates/yokohama-core crates/yokohama-oracle \
                          crates/yokohama-difftest crates/yokohama-bridge crates/gen-data
  (empty)

git diff --stat master -- crates/yokohama-cli
  crates/yokohama-cli/src/repl.rs   | 23 +
  crates/yokohama-cli/src/wizard.rs |  3 +-        ← M6.1's 24/2 (R-4.1 + the pub), because
                                                     this branch is STACKED on web-room; against
                                                     web-room's tip 937ce92 the CLI diff is empty

SEMANTICS_REV = 7   (crates/yokohama-core/src/game/semantics.rs:180)
```

**The fifteen-gate release roster was NOT re-run**, for the reason the M6.1 close gave: every
crate the roster measures is byte-identical to `master`, so a re-run could only re-measure
master's own reading. Nothing added in this cycle is `#[ignore]`d; the whole-game gate stays out
of CI (`48ae908`) and per-edit locally.

---

## 3. The corpus diff

`web/src/wire/` is a contract, not a snapshot. It was regenerated at Tasks 2, 3, 4, 5 and 6 and
checked (never moved) in the fix waves; every diff was read. **54 files at the open, 62 at the
close.**

| where | change | why |
|---|---|---|
| Task 2 (ruling #1) | `games.json`, `table.json`, `table_dealing.json`, `deal.json`, `deal_waiting.json` re-cut: each human seat gains `"name": null`; each `TableView` gains `"locked": false`, `"winner": null`; NOTHING else | the widened document serializes three new fields; the plan had this diff at Task 3, but the committed-corpus test refuses drift, so the regeneration moved to the task that changed the shape. The four 4xx fixtures embed no table and did not move |
| Task 3 | none | `"id": "wire"` beside `"name": "wire"` produces the identical view |
| Task 4 | none | nothing `join` serves was on the walk yet |
| Task 5 | `games.json` gains `"next": null` | the listing's cursor shape |
| Task 6 (rulings #10–#12) | EIGHT new: `table_open.json` (a 2P open LOCKED table read by a STRANGER, `my_seats: []`, one seat named "Red", one open), `games_open.json` (`scope=open`: `wire` locked and `wire2` unlocked), `games_next.json` (`scope=mine&limit=1` with a non-null cursor, the `created_at` inside it frozen by `freeze_clock`'s second arm), `open_409.json`, `join_403_locked.json`, `join_403_password.json`, `join_409_taken.json`, `join_409_not_open.json`; `table.json`/`table_dealing.json`/`deal.json`/`deal_waiting.json`/`games.json` re-cut with the names "Red"/"Blue", `locked: true`, and the second table `wire2` | the walk through an open, locked table; `table_open` keeps the walk's 2P shape rather than the spec's 3P (ruling #10); `wire2` is an OPEN unlocked table rather than a bot table so `games_open` carries both kinds (ruling #11); `wire2` is created FIRST so the cursor fixture does not depend on a clock tick (ruling #12) |
| fix waves | none | the value pin's second needle changes an INPUT (the wrong-password attempt), and no refusal echoes it |

The client's key-set rows (`contract.test.ts`) gained the eight fixtures, `locked`/`winner` on
`TABLE`, `next` on `GAMES_LIST` and `name` on the human seat's keys; `Widen<TableView>` makes a
server-dropped field fail the type-check. **A field the server ADDS that the client does not
declare still passes the type-check and is caught only by the root key sets** — CLAUDE.md's
stated limit, and the reason the two `contract.test.ts` rows went red at Task 2 and stayed red
by ruling until Task 6 brought the types up.

---

## 4. The reviews

### 4.1 The whole-branch code review (`final-review`, Fable, on `bd02a06..efe5a62`)

Report: `.superpowers/sdd/2026-09-15-web-frontend-m6.2-plan/final-review-report.md` (the SDD
workspace is gitignored; the report is quoted here). Verdict **"ready to deploy WITH FIXES"**:
0 Critical, 3 Important, 8 Minor; none of the fifteen execution rulings disputed. The three
Importants, all fixed in the ONE fix wave (`ad29715`, `a214a19`) before the image was built:

- **I1 — `to_move` was unbounded on the stored path**, and the blast radius was the LOBBY, not
  one game: `Games::list` runs `check_shape`, keeps the row, then calls `view()` on it, and
  `view()` does `PlayerId::new(t.to_move)` — probed with `to_move: 9`: `check_shape=Ok(())` and
  a panic `PlayerId must be 0-3, got 9`, with no `CatchPanicLayer`, so one hand-edited file on
  the volume would drop every player's `GET /api/games`. Fixed with the arm beside `winner`'s.
- **I2 — spec §8.4 mutation 4 says both catchers "must fire"; only one did.** The salt
  serialized under `nonce`: the by-key cell stayed GREEN (it searched the words `salt`/`hash`/
  `"lock"`), the run-value pin failed naming three surfaces. Fixed by binding the values in the
  by-key cell; the battery (§4.3) then measured both.
- **I3 — the help panel still described the pre-M6.2 client** ("the game is in the list on the
  start screen"). Rewritten; a cell now asserts the panel names the lobby and the link.

Its claims table is the part to keep: blast radius empty and `SEMANTICS_REV` 7 confirmed; the
four M6.1 race cells extracted body-by-body at `bd02a06` and at HEAD and found identical in
every assertion; §8.4 #1 caught (two cells); R-2.7 exactly one route; the lock order read at
every acquisition with no inversion; the listing's no-record claim read against the armed cell;
the seed, password, salt and hash pins green. And its deferred-minor triage: two of the ledger's
thirty-nine minors were promoted (both became I1 and I2), one was shown unreachable
(`written.remove`/`live.remove` after the unlink: `keep` holds the same per-table lock `delete`
holds), and the rest ride to §6.

The fix wave's own re-review verdicted all seven items ADDRESSED with three Minor observations
(§6.2).

### 4.2 The live review (`live-review`, Fable, clean, two browsers)

Report: `.superpowers/sdd/2026-09-15-web-frontend-m6.2-plan/live-review-report.md` (gitignored;
quoted here). A clean Fable subagent with no prior context drove `https://yokohama.moore.nyc`
(then `0.5.0-m6.2`) with three headless Chromium contexts as three people — the creator, a
link-only joiner, a stranger — at 1440×900 and two more at 390×844, through the ten steps the
brief set plus a paging probe and an art-load check, with curl probes of every join, password,
create and stranger-fence arm. Everything it created it deleted as its creator (29 tables); the
house's open list read empty afterwards.

**Verdict: SHIP WITH FIXES** — 0 Critical, 1 Important, 7 Minor, and zero console errors,
uncaught exceptions or unexpected 4xx/5xx on any page. **Measured:** the creator's screen
learned the joiner's name on the poll in 3.0 s and 2.5 s with no reload; the game opened on the
creator's screen 2.8 s after the last keep; 13 of 13 event-producing moves walked on the other
screen within about 3 s.

- **I-1 — the seated open-table screen scrolled sideways 32 px at 390 px** (`scrollWidth` 422 vs
  `clientWidth` 390): the link row `Link · <input> · copy` did not wrap and the input did not
  shrink, so the Copy button stuck out — a regression against the lane's recorded 0 px overflow
  property, on a screen the width gate did not cover. **Fixed the same morning** (`8dc6316`:
  the input shrinks and the row wraps; a browser assertion in `joins_by_link.spec.ts` now reads
  0 px page overflow on that screen at 390 px, with the copy button's right edge inside the
  viewport, and fails on the pre-fix CSS), re-reviewed in scope (3 addressed, no new breakage),
  and **redeployed as `0.5.0-m6.2.1` at 02:47Z** — the M5.5 precedent for a live-found layout
  regression. Two spec-letter minors rode in the same fix: the dealing screen's leave control
  read "Games" (now "Lobby", like every other screen's), and a refused link's sentence rendered
  BELOW the lobby's lists (spec §5.1: above; the cell now asserts document order).
- **The five minors that ride to §6:** Open tables shows only the first page of 20 with no
  "more" (spec-consistent; the twenty-first is invisible to the house); a stranger holding an id
  learns a table's status from `join`'s 409 while R-2.7 says a stranger never learns a status
  from a refusal — **the spec contradicts itself** (§3.2 orders `join`'s status arms by name,
  ruling #9), and this close rules §3.2 the letter: a joiner needs the status to draw the right
  screen, and behind Access beside a `claimed` door it is a wording matter; the Game menu's
  summary names the game by id, not by name; Help is reachable only from the game screen; the
  lock marker is drawn on a Your-games row at every status.

The report's step-by-step walk (with screenshot names) and its API arm table are the fuller
record; its cleanup ledger lists every id.

### 4.3 The mutation battery (`mutation-battery`, Opus, at `1330a21`)

Spec §8.4's ten mutations, one at a time on the local build, reverted by explicit path:
**10 of 10 caught** (report `mutation-battery-report.md`). Row by row: (1) `join`'s dealing arm
emptied → `join_refuses_in_the_order_a_client_reads` at `tables.rs:2726`; (2) the `Open`-seat
test replaced by `false` → two cells, the first `unwrap_err()` on an `Ok(TableView …)`; (3a)
`check_password` returning `true` → the lock cell at `table.rs:512` and the join cell; (3b) the
lock ignored when no password is sent → the join cell's `unwrap_err()` on a seated view; (4) the
salt under `nonce` → **BOTH** catchers: `table.rs:535 the salt reached the view under some key`
and `wire_fixtures.rs:1501 … the create's own reply carries the salt, the table view carries the
salt, an open lobby row carries the salt`; (5a) two writes → `one write for the seat AND the
flip — left: 2, right: 1`; (5b) no flip → `the filling join flips — left: Open, right: Dealing`;
(6) `scope=mine` as `true` → two listing cells; (7) the id derived from the name →
`"Thursday's game" is not a usable game id`; (8) a record read in the listing loop → `the
listing read a RECORD for d`; (9) `check_name` bypassed at join AND keep → three cells; (10)
verified by the Task 9 report's recorded reload mutation (`Error: A did not navigate while B sat
down`), not re-run — Playwright is off limits to the battery by design.

Three things the battery did NOT do, said plainly: it did not run the two whole-game arms or
any browser spec; row 4's "both catchers" is true only since the fix wave; and no cell was
written during the battery — a mutation no cell caught would have been a finding, and there was
none.

### 4.4 The per-task reviews — the findings that mattered most

Nine Opus task reviews, six fix rounds (Tasks 3, 4, 5, 6, 8 and the whole-branch wave), every
fix re-reviewed in scope. The four worth carrying:

- **Task 4 (Important 2):** the lock split dropped create-versus-delete mutual exclusion —
  §1.4. The spec's "the M6.1 race cells are the proof that nothing was lost" was a sentence the
  mechanism disproved; ruling #6 is its correction.
- **Task 5 (the implementer's own finding, ruling #8):** `keep` refused by status before
  checking the seat, so a stranger learned a table's status through it — caught by writing the
  sweep the spec asked for, before any reviewer saw it.
- **Task 7 (Important 1, ruling #13):** retiring the form's saved list removed the client's only
  delete control and `api.remove` had zero callers; the plan's `OpenTable` would have restored
  it at OPEN tables only. Re-homed onto the lobby rows — the one product decision this cycle
  took that the spec did not make.
- **Task 3 and Task 4 (the order lesson):** single-defect probes pin nothing about ORDER. Task
  3's review asked for two-defect probes at adjacent links; Task 4's implementer applied the
  lesson unprompted (two adjacent-pair probes in the join cell) and Task 5's sweep was built
  with `join` deliberately last.

And one process finding, recorded because it cost eight round trips: a reviewer's final message
reaches the controller truncated at about four thousand characters. The owner ruled mid-cycle
that every agent writes its full report to a transcript file and replies with the path; the
final review, the fix wave, the live review and the battery did.

---

## 5. The rulings taken in execution, with their costs

**Fifteen numbered rulings** taken by the orchestrator, tabled in the order they were taken,
plus the plan's own R-plan-1–R-plan-6 restated at the end. **Cost** is what is wrong if the
ruling is wrong. The whole-branch review read all fifteen and disputed none.

| # | task | ruling | cost if wrong |
|---|---|---|---|
| 1 | T2 | The corpus is regenerated at Task 2, not Task 3: adding `locked`/`winner`/`name` changes every served `TableView`, and the committed-corpus test refuses drift, so leaving it to Task 3 would have left `wire_fixtures` red between tasks against the per-edit gate. The client's two contract rows went red from Task 2 to Task 6 by the same ruling | a second, smaller corpus read at Task 3 (it was empty) |
| 2 | T5 | The stranger sweep asserts every other route FIRST, then `/table`, then `/join` LAST at the open table — a successful join seats the caller and flips a 2P table | a sweep asserting the wrong table's status; the reviewer would have caught it |
| 3 | all | The `Claude-Session:` trailer carries `cli/<session id>` — this was a CLI session with no claude.ai/code URL | cosmetic: a trailer value shaped unlike the 765 before it |
| 4 | T3 | The refusal order follows the SPEC (§3.1: … creator; bot; `name`; `display_name`; `password`; `id`), not the brief's snippet, which validated `display_name` before the seat loop | none functionally |
| 5 | T4 | Two cells the plan omitted but spec §3.4/§8.1 name are written: a delete cannot interleave with the filling join; a 500-character name on join is 400 with the document byte-unchanged | two extra cells |
| 6 | T4 | Create-versus-delete exclusion is restored by tombstone-BEFORE-unlink in `delete`, with the tombstone removed again if the unlink fails, plus a race cell red on the pre-fix ordering — the global-lock alternative was declined to keep R-2.9's letter | a transient false tombstone during a FAILING unlink refuses one move and the handle rebuilds; no store in the crate fails `delete` today |
| 7 | T5 | R-2.6's witness: the sweep's last step at the open table is the creator's DELETE → success, then 404 | one redundant assertion |
| 8 | T5 | `keep` checks the SEAT before the STATUS, so a stranger's keep is 403 at every status; the sentence stays keep's own "seat N is not yours" | a stranger's keep at an open table says "not yours" instead of "still open" — the spec's preferred direction |
| 9 | T5 | A stranger's `/join` at dealing/playing is 409 with the status (spec §3.2 says 403 is NOT an arm of join); the brief's "403 as well" was wrong | none |
| 10 | T6 | `table_open.json` keeps the walk's 2P shape (one human, one open, locked) rather than the spec's 3P — the walk is one continuous 2P game | the open-table cells see one "sit here" button instead of two |
| 11 | T6 | `wire2` is an OPEN, UNLOCKED 2P table (not a bot table) so `games_open` carries a locked and an unlocked table as §7 says, while `games_next` keeps its cursor | `wire2` never deals in the walk; the corpus already had no bot-table deal |
| 12 | T6 | `wire2` is created FIRST so the cursor fixture does not depend on a clock tick; `freeze_clock` gains a second arm normalizing the `created_at` inside the cursor string | if the normalization ever masked a real cursor change, the id half would still show it |
| 13 | T7→T8 | The creator's two-click abandon is RE-HOMED onto the lobby's Your-games rows: retiring the saved list had removed the client's only delete control, and the spec's `OpenTable` Abandon exists only at open tables. **The one product decision the spec did not make — the owner should read it as such** | one control to remove if the owner would rather not have delete in the lobby |
| 14 | T7→T8 | Lobby rows are `<a href="#/t/<id>">` links, not buttons — spec §5.2's word, and the affordance the cycle exists for (middle-click, copy, new tab) | one markup change back |
| 15 | T10 | The schema-1 `m6.1-live-review` table is READ through the new server as its creator BEFORE it is deleted — the only live proof of spec §9's "schema-1 documents list and open unchanged"; the end state is the M6.1 close's ask | a GET on a throwaway game before its deletion |

### Restated from the plan (R-plan-1 … R-plan-6)

- **R-plan-1** `Games`'s fields became `pub(crate)` for the cut (plus the five methods §1.1
  names, which the plan did not anticipate).
- **R-plan-2** `mirror_of` stayed in `games.rs` as `pub(crate)`; `winner` reads
  `final_result().winner`.
- **R-plan-3** the lib test helpers set `id`, not `name`.
- **R-plan-4** the lobby is ONE `usePolled` instance over both scopes.
- **R-plan-5** the browser harness was re-pointed in Task 7, so `npm run e2e` never went red.
- **R-plan-6** `display_name` lands on every seat whose `user` equals the creator.

### Deviations from the spec's letter, recorded here (the whole-branch review's M4)

- **§6 "all FOUR statuses"** — the sweep runs at THREE. `TableStatus` has three variants;
  "over" is a `Playing` table whose session is over, unreachable cheaply in a lib cell, and
  byte-identical to `playing` for a stranger (the sweep's doc argues it).
- **§3.3 "capped at 100"** — implemented as a 400 above 100, which is the better behaviour and
  not the spec's word.
- **§3.5 "the M6.1 race cells are the proof that nothing was lost"** — false, §1.4 and ruling #6.
- **§7's `table_open` shape and `games_open` contents** — rulings #10 and #11.
- **§5.2 / §5.4** — the lobby-row abandon, ruling #13.
- **The plan's Task 9 locators** — three were wrong at HEAD (`getByLabel("Password")` collided
  with the checkbox; `getByLabel("Link")` with `abandon Link test`; the "Lobby" button sits
  inside a closed `<details>`); all corrected by reading the components.

---

## 6. The LEAVE list

### 6.1 Inherited from the M6.1 close §6 and not touched by this cycle

Every M6.1 §6.1 item stands (the Game menu staying open, the 390 px tile popover, R-F13's lit
row, R-F7/R-F8, `useSections()`, the OS-entropy whole-game gate's wall — which read 13 s / 14
min+ / 19.8 s / 24.3 s / 13.4 s / 14.8 s / 13.0 s across this cycle's runs, so the variance is
confirmed and still un-diagnosed — and M5.5's forty-four archived minors).

From M6.1 §6.2, still open: the sign-out transition's raw 401 sentence; colours and the start
seat not selectable in the form (R-8.2); the 1024 px banner; `undo_allowed`'s 403 on a
`ReplayError`; the waiting caller's set table built for `first_human_seat`; `create` reading a
store `Err` as OCCUPIED; the two-browser cell's cost (this cycle's runs: 17.5 / 12.9 / 7.6 min —
the SERVER draws the seed, so the wall is the game's length) and its loop counting raced no-ops
as moves; `hasMove` belonging in `harness.ts` (now DUPLICATED a second time, in
`joins_by_link.spec.ts` — Task 9's reviewer judged the duplicate better than a third export
while the brief's blast radius forbade editing `two_browsers.spec.ts`); no committed fixture at a
bot table's deal; the Modal path and the Worker's 401→502; §3.5/§3.6's present tense.

**Closed by this cycle** (R-2.11): the game's name as its id; an over table listing without
the winner; the unbounded list; "a DELETED game reads reconnecting… forever" for the one-shot
half (`usePolled`'s `onEnd`; the stream half is M6.3's); `Dealing` never re-reading while
waiting is NOT closed (it polls `/events`, which answers `dealing` without a re-read — the
"waiting on b, c" staleness at 3P+ stands); `games.rs`'s size (cut).

### 6.2 This cycle's own

**Product**

- **The Game menu's list is one page of twenty.** `GameMenu.tsx`'s `api.list()` is
  `scope=mine&limit=20` with no paging; a player with more than twenty tables loses rows M6.1
  listed unbounded. Page it, or point the menu at the lobby.
- **Open tables is one page of twenty with no "more"** — spec-consistent (§5.2 pages Your games
  only); the twenty-first open table is invisible to the house.
- **A creator who holds no seat gets no Abandon on the open-table screen** (reachable if a create
  fills every seat with other people's emails); the lobby row covers it.
- **One scope's failure blanks both lobby sections** (`Promise.all`) and leaves "loading…"; the
  poll retries, the refusal cell never asserts "loading…" is gone.
- **A failed `open` from the Game menu costs a second request and evicts the player** (navigate
  before fetch); **Back during an in-flight `open` can leave a table drawn under `#/`**.
- **The lobby's poll passes no `onEnd`** (unreachable today: a scope cannot refuse).
- **`End.reason` declares `"deleted"`**, which nothing produces until M6.3.
- **The display name is remembered even when the create then fails** (written before
  `api.create`).
- **The table document is fetched twice per visit** (App's `open` and `OpenTable`'s opening
  effect).

**Server**

- **A transient false tombstone while an unlink is FAILING** (`delete`, between the insert and
  the compensating remove): one refused move, the handle rebuilds; no store fails `delete` today.
- **`locks` grows one entry per id and is never pruned** (by design — pruning would hand two
  writers two locks); **a poisoned table lock is now one permanently unusable GAME**, never
  cleared (better than a dead server; `check_shape`'s bounds are what keep it from happening).
- **`written.remove`/`live.remove` run after the unlink** — shown unreachable by the
  whole-branch review; the line rides for the record.
- **`ApiError::table_status`'s doc** says only `Dealing` reaches it (three producers now);
  **`holder_of`'s `_` arm** names `SeatDoc::Open` "the bot" (unreachable behind `handle`'s
  refusal); **`list_for`'s doc** is false above 100 rows; **two sentences for one class**
  (`create`'s "not a usable game id" vs `store.rs`'s "not a usable game name" ×3, the store's
  own guard on a path `create` never reaches); **`table.rs:88` cites `outcome.rs:71`** for `:96`;
  **`config_of` validates the label through `check_name` and re-trims in `create`**; **the
  constant-time cell is a correctness cell, not a timing witness**, and should say so; **the
  lock's HASH arm and a non-hex salt of the right length are unexercised**; **the links EARLIER
  in `config_of` (players … creator) remain unpinned relative to each other**; **`?limit=100`
  exactly is unasserted**; **`Dealing`'s leave carries no fallback sentence** (unreachable);
  **the route effect uses `toLobby` without listing it** (a `useCallback([])`, no eslint).

**Tests and tooling**

- **`api.list` always sends `?scope=mine`** though the server defaults to it — that cost 27
  stub-matcher edits and the converted matchers swallow `?scope=open`; `isGamesList(call,
  scope?)` exists now, the older matchers do not use it. **The `split("?")` substitution sits on
  POST matchers** where no query occurs. **Two expectations call `seatName`** rather than
  restating the mapping.
- **`freeze_clock`'s `next` arm is name-keyed**, not shape-keyed. **Neither password needle has
  an anti-vacuity partner** (a walk edit dropping the wrong-password join would leave
  `WIRE_WRONG_PASSWORD` a needle for a string no request sends).
- **The M1 cell's `process.on("unhandledRejection")` assertion has node's timing**; the screen
  assertions are the witness — drop that line if it flakes.
- **`OpenTable`'s "waiting for N more", "take a seat" and "· password required" have no cell**;
  `name.trim()` is never exercised with whitespace; `join_403_locked` (an empty password box at
  a locked table) has no client cell.
- **`Lobby.test.tsx:26`'s comment** says the control is on every row (Your-games rows only);
  **`Abandon.tsx`'s doc** says a refusal leaves the control confirming, which holds for the
  DELETE but not the `onDone` path; **`Abandon` deletes `table.id`** where the old copy deleted
  the `game` prop (agree by construction, unpinned).
- **`joins_by_link.spec.ts`'s move loop swallows every throw** where `two_browsers` rethrows on
  a still-offered move; its 403 exemption comment overstates the harness's console channel.
- **Task 1's cosmetic notes**: a 112-column line, a three-line `use`, test-import grouping
  carried from the base, one intra-doc link that now points across files.

### 6.3 Closed as wrong or obsolete

None of the inherited items was found false at HEAD; two of this cycle's ledgered minors were
promoted to the fix wave rather than closed (I1, I2), and the `written.remove` line above is kept
as unreachable rather than struck.

---

## 7. What this cycle does NOT claim

Spec §10, restated at the close HEAD.

- **No spectators, no live transport, no chat, no hosting.** The lobby and the open table poll
  at 3 s from a visible tab; M6.3 removes the poll. Holds.
- **The password is a shared secret and not a security boundary.** No lockout, no rate limit,
  no change after create; salted SHA-256 compared in constant time (a correctness cell, not a
  timing witness — §6.2); behind Access and beside a `claimed` door that is itself not a boundary
  (R-1.4). Holds.
- **Every signed-in person sees every open table's seat list, emails included.** Holds.
- **No invitation by link to a named person, no kick, no seat change, no colour choice at join,
  no reopening a full table.** Holds.
- **No hosted visibility flag.** Holds.
- **No size or wall figure for any of this.** Holds; §2's figures are per-machine readings.
- **Every count, line number and route name in the spec is at `48ae908`**; the plan re-read
  them at its own HEAD and this close re-measured at the close HEAD.

Added at the close:

- **The stranger sweep runs at three statuses, not four** (§5's deviations).
- **The lobby's delete control is this cycle's decision, not the spec's** (ruling #13).
- **The M6.1 race cells were NOT the proof that nothing was lost** in the lock split; the
  fifth cell is (§1.4).
- **The mutation battery did not run the browser** (row 10 is the recorded measurement) and did
  not run the whole-game arms.
- **`table_open.json` is a 2P fixture** (ruling #10), so no committed fixture shows an open
  table with two open seats.
- **The whole-game gate's wall is still un-diagnosed**: 13 s to over 14 minutes on identical
  code across this cycle's runs, an OS-entropy bot seed each time.

---

## 8. The handover into M6.3 — transport and spectating

From the room plan §2, and the M6.3 spec
`docs/plans/2026-09-15-web-frontend-m6.3-transport-and-spectating-spec.md` with its plan
`docs/plans/2026-09-15-web-frontend-m6.3-plan.md`, both written beside this cycle's and both
re-read against `48ae908`: **what moved under their line citations is what M6.3's implementer
re-reads first.**

What M6.2 built that M6.3 starts from, by the M6.3 spec's own sections:

- **§5.1 — the poll hook's `Source` shape is fixed** exactly as the M6.2 spec §5.5 wrote it:
  `usePolled(source: Source<T> | null, active, onValue, onEnd?)` with `Source { key, once,
  stream }` (`web/src/signal.ts`), four consumers (`useSignal` in the game, `Dealing`, `OpenTable`,
  `Lobby`), `stream` carried and unused, `End { reason: "deleted" | "gone" | "refused", sentence?
  }` with `"deleted"` produced by nothing yet. M6.3 replaces the transport under ONE hook.
- **§3.4 / §3.5 — the open-table read and the password's `unlock`**: `Games::table_for_anyone`
  is the one widening (`open` for anyone), `Lock::matches`/`TableDoc::check_password` is the
  gate a spectator's `unlock` reuses, and `lib.rs`'s `seated()` is untouched — the spectator is
  designed behind `admitted()` beside it, not inside it.
- **The per-table locks** (`Games::lock_for`) are where a per-table WATCH CHANNEL's bump sites
  sit: `join`, `keep`, `settle`, `finalize`, `flip_to_playing`, `delete`, and `persist` (its own
  lock) — every write that should notify a watcher already goes through one of them.
- **The house channel** M6.3 §3 describes ("a bare seq; watch coalesces") has its first
  consumer already: the lobby's `usePolled` over both scopes.
- **`TurnMirror.winner`** is on the document, so a lobby row's "over · <name> won" needs no
  session; **the two-scope cursor listing** is what a lobby stream re-reads on a bump.
- **Line numbers in the M6.3 spec and plan that name `games.rs`** now mostly name `tables.rs`
  (the cut); `seated()` moved by the `Open` arm in `handle`; `lib.rs`'s router grew one route
  (`join`), so route counts in the M6.3 spec's §7 are one higher.

Also carried into M6.3's execution: the LEAVE list above (§6); the process as ruled (R-P11,
Opus implementers and per-task reviewers, one clean live review), with one addition the owner
made mid-cycle — **every agent writes its report to a transcript file and replies with the
path**; the whole-game gate stays out of CI (`48ae908`) and per-edit locally; the SDD workspace
convention; explicit `git add` paths; the six fenced crates byte-identical and `SEMANTICS_REV`
at 7.

**Two things owed to the owner:**

- **PR:** `web-room-m6.2` is pushed and its pull request is opened against `web-room` (stacked
  on PR #31, which is unmerged), so the review diff is M6.2 alone; when #31 merges, GitHub
  retargets it. Merge order is the owner's.
- **Ruling #13** — a delete control in the lobby — is the one product decision this cycle took
  that the spec did not make. Keep it or strike it; §5 carries the cost either way.
