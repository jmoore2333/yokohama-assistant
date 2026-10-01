# Web frontend — M6.3 "transport and spectating": the close

*Amended 2026-09-16 at the release; frozen after this.*

**SHIPPED AND LIVE 2026-09-16: `yokohama-web:0.5.0-m6.3`, built at `74a0e80` on
`web-room-m6.3`, serving `yokohama.moore.nyc` from container `f0c371c7…` since
19:20:51Z. The R-3.1 tunnel measurement was taken both ways and SSE SHIPS (§2.1); the
three-browser live review returned SHIP, 0 Critical / 0 Important / 3 Minor (§4.5); PR
#33 is open against `web-room-m6.2`
(https://github.com/jmoore2333/yokohama-assistant/pull/33). The build closed at
`9a4626d`; everything recorded below was measured there, and the amendment touches this
header, §1's opening, §2.1, §4.5, §7 and §8 — nothing else.**

The branch was cut from `web-room-m6.2` at `be21a4b` (the M6.2 close HEAD) and carries
**twenty-two execution commits** — nineteen through Task 9's fix round at `26da94f`, then
the final review's three-commit fix wave at `9214661`, `9f70aa2` and `9a4626d` — plus the
doc-pass commits this record is part of. It is stacked: `web-room-m6.2` is stacked on
`web-room` (PR #31, unmerged) and its own PR #33 is open against `web-room-m6.2` —
**merge order is the owner's**.

The authority for everything below is the spec,
`docs/plans/2026-09-15-web-frontend-m6.3-transport-and-spectating-spec.md`, under the
eleven owner rulings R-P1–R-P11 in `docs/plans/2026-09-14-web-frontend-m6-room-plan.md`
§0 and the spec's own R-3.1–R-3.14. The plan
`docs/plans/2026-09-15-web-frontend-m6.3-plan.md` with its R-plan-1–R-plan-5 is what was
executed. Both are frozen; where a sentence in either is wrong at this HEAD, §5 and §7
say so here rather than editing them.

**This document is M6.4's record of what M6.3 built. M6.4's ENTRY artifact is
`docs/plans/2026-09-16-web-frontend-m6.4-chat-handover.md`** (§8).

**One thing to read before trusting any sentence this milestone wrote about itself:**
§7, and then §4.2. The final whole-branch review found EIGHT false sentences at HEAD —
doc paragraphs and cell titles that described a mechanism the code no longer had — and
the fix wave corrected them. That is the same yield M2.5 (three) and M3 (five) recorded,
and it is the reason §7 exists.

---

## 1. What shipped

The room plan's M6.3 exit criterion is **"no polling on the NAS; a spectator link shows a
game walking."** **Both halves are now shown.** In the browser walk on this workstation a
watching tab played a whole two-person game to a winner having asked `GET …/events`
**zero** times and opened `GET …/{id}/stream` **once** (§2); and on the NAS, in the live
review of the deployed image, a watcher arriving by link asked `/events` zero times and
held ONE stream through two full turns, through a 444 s game of 1,471 events and through
a whole game against the bot (§4.5). Everything §1 describes was measured at the build
HEAD `9a4626d`; §2.1 and §4.5 are the release's own readings.

### 1.1 The signal — `crates/yokohama-web/src/signal.rs` (spec §2, Task 1)

A new module holds `Bump { seq, end }` and `Signals`: a `Mutex<HashMap<GameId,
watch::Sender<Bump>>>` of per-table channels and one house sender, with `bump_table`,
`bump`, `end`, `subscribe` and `subscribe_house`. `Games` owns it (`pub(crate) signals`,
`Games::signals()`), because every write site is there. `tokio` gains `sync` and later
`time`; `tokio-stream = { version = "0.1", features = ["sync"] }` enters for
`WatchStream`, declared in the M6.3 dependency paragraph the way M6.2 declares `sha2`
(R-3.11, R-2.1).

**Six bump calls, one per write that lands**, each in a success arm, each named in a cell:

| site | file | call | why |
|---|---|---|---|
| `persist` | `games.rs:503` | `bump` | AFTER `mirror_turn` returns, so a woken lobby reads the refreshed mirror |
| `create` | `tables.rs:719` | `bump` | the `Ok` arm of `write_table` |
| `flip_to_playing` | `tables.rs:901` | `bump` | the `Ok` arm — **and this is `finalize`'s bump too** |
| `join` | `tables.rs:1048` | `bump` | a seat filled, or the table left the open list |
| `keep` | `tables.rs:1150` | `bump_table` | the dealing screen's "waiting on" line; nothing a lobby shows changed |
| `delete` | `tables.rs:1334` | `end` | after the store delete and both map removals |

`finalize` has no bump of its own: its last statement is `flip_to_playing`, so a second
one would announce one write twice. A `Conflict` arm never bumps — the writer that won
bumped.

**A bump follows its write and never precedes it, and that is proven rather than
asserted.** Two GATED cells park the real write behind a store gate and read the receiver
QUIET at the one instant a hoisted bump would be loud: `FlipGate` for the flip, and —
added in Task 1's one fix round (R-1.1) — `MirrorGate` for `persist`, which is the only
site with two sequenced writes and where a hoisted bump is a stale lobby row. The other
four sites carry refused-write cells that assert nothing is sent when the write fails.
The final review named these "the best instruments on the branch".

### 1.2 The streams (spec §3.1–§3.3, Task 2)

`GET /api/games/{id}/stream` — `text/event-stream`, admitted by exactly the function
`GET …/{id}/events` uses. It writes the current `Events` at once (`WatchStream::new`
yields the channel's current value), then one `event: signal` frame per bump with the
object on one `data:` line, `event: end` with `{"reason":"deleted"}` when the table is
deleted, and a comment heartbeat every 25 s. **No `id:` line and no `retry:` line**
(R-3.14). The handler holds no game lock across an await.

`GET /api/stream` — the house channel, identity required and no other admission,
`event: house` with `{"seq": N}`, built from `WatchStream::from_changes` so it is silent
on connect.

`events` itself keeps its shape and gains two things: an `open` arm before
`Games::handle`, answering `events: 0, status: "open"` for a table with no record
(R-3.3), and the shared admission. Both transports are built by ONE function,
`events_at(state, id, user)`, which is what makes "`stream` and `events` are admitted
identically" a property rather than a coincidence.

`SIGNAL_KIND` is a `pub const` in `lib.rs` reading `"sse"` (R-pre-2, R-plan-4), served by
`Health.signal`; `HEARTBEAT` is 25 s and `AppState.heartbeat` carries it so a test can
construct 50 ms and read two heartbeats OFF THE WIRE (R-3.12). `EventsQuery` gains
`#[serde(default)] wait: Option<u64>` capped at 25 — **above the cap is a 400, not a
clamp**, because a silent clamp makes an early answer read as "nothing changed".

**The `Err` arm splits** (final review M-5, fix wave): a 403 or 404 from a re-admission
writes the `end` frame and the stream closes; **anything else writes an SSE comment** —
which the client's parser drops exactly as it drops a heartbeat — so a transient store
failure no longer tells every watcher the game is gone and navigates them away. A
`Flaky` store cell arms one failing table read, fires it with a bump, asserts the arming
was SPENT, and reads 200, no `end`, at least one comment and two `signal` frames.

### 1.3 The spectator (spec §3.4, §3.5, §4, Task 4)

`seated()` became **`admitted(state, id, user) -> (TableDoc, Viewer)`** with
`Viewer { Seat(PlayerId), Spectator }`; `viewer_of` is gone, its `None` arm being the
condition `admitted` now answers. Nine call sites became `admitted` + `Games::handle`
pairs (R-plan-2). `spectators_write_nothing(viewer)` is the one refusal `input` and
`suggest` add — the only two writes that went through `seated`; `deal`, `keep`, `join`
and `DELETE` resolve the caller inside `Games` and already refuse a seatless caller
before any record is read, which the write sweep proves behind a store that PANICS on any
record read (R-4.4).

`TableDoc` gains `#[serde(default)] unlocked: Vec<String>` (the schema stays **2**;
`check_shape` bounds it at 256 and requires each entry to parse as a `User`) and
`TableDoc::can_watch(user)`, a three-arm disjunction: a seat holder always, anyone at a
table with no lock, anyone on `unlocked`. `TableView` gains `can_watch: bool` — a fact
about the READER — and `unlocked` itself never serializes.

`POST …/{id}/unlock` answers seven ways under the table's own lock and bumps nothing
(R-pre-3): 404; 409 at `open` and `dealing` with the status beside it; 200 and NO write
at a table with no lock; 403 "wrong password for this table"; 409 at the 257th watcher;
otherwise 200 with the `TableView` whose `can_watch` now reads true. The list
deduplicates, and a seat holder unlocking their own table is a 200 no-op.

`table_for_anyone` widened to "not dealing, or seated" (R-3.13), written as a NEGATIVE so
a fourth `TableStatus` would have to be considered rather than admitted by default;
`Scope::Playing` parses `"playing"` and keeps `status == Playing`, over included.

**The projection** (`client.rs`): `facts_for(repl, Viewer::Spectator)` hides every hand;
the frame is built for a stand-in, `(current_player + 1) % players` (R-plan-3), which is
a NON-ACTING seat's frame by construction — `build_dashboard` uses `seat` only to draw
its own hand panel and `build_decision_for` returns `viewer_is_deciding: false` over an
empty list for any seat that is not acting. A hidden hand still yields a `Hand` panel, so
the projection **drops `Panel::Hand`** from a spectator's dashboard after the builder
returns it. `ClientState.seat` and `HistoricalFrame.seat` became `Option<PlayerId>`,
`null` on a watcher's payload and byte-identical for a seated one. `prompt_of_viewer`
answers `Watching { to_move }` unless the game is over or locked. The cells pin the
mechanism BEFORE the drop (the builder really does hand a spectator a masked panel, so
the drop removes something real) and the six consequences after it, at 2P, 3P and 4P.

**R-4.3, taken as a pre-review correction**: `events` and `stream` answer ANY signed-in
caller at an OPEN table (spec §3.5 row 2), through `table_for_anyone` before `admitted`,
so the joiner who arrived by link learns when the seats fill. `admitted` itself is
unchanged, so every other read keeps its 403 for a stranger at `open`.

### 1.4 The one CLI touch (R-3.8, Task 5)

`Repl::index_placements_for` takes `Option<PlayerId>`; `index_placements()` delegates
with `Some(current_player)` exactly as before; `None` pushes no `OwnHand` row, every
other location being face-up at the table. The doc says what `None` MEANS rather than
leaving the reader to infer it from the call site. The branch's whole CLI diff against
`web-room-m6.2` is `repl.rs`, **+39/−7**; the goldens and the eighteen `guide_sync` cells
are untouched and are how that is known.

### 1.5 The wire (spec §8, Task 6)

The corpus went **62 → 71** (§3). The walk gained a third identity that does the whole
arc — `green@wire.test` reads the open table, is refused at the deal, is refused at the
locked game, gets the password wrong, gets it right, and watches to the end — and a
fourth, `grey@wire.test`, who never knocks and whose `can_watch` therefore stays false,
which is the payload the client draws the unlock face from.

`wire_fixtures` gained `the_stream_s_first_frame_is_the_events_route_s_reply_by_value`:
through the real router it opens the stream at a playing table, reads to the first blank
line, and asserts the parsed `data:` line EQUALS the parsed `/events` reply — two
transports, one object, one instant. Off the wire it also asserts the content type is
exactly `text/event-stream`, that there is exactly one `data:` line, and that there is no
`id:` and no `retry:` line.

**The watcher's-email pin** searches every payload of a full run for GREEN's address and
for STRANGER's — the whole corpus, not a list of payloads "built for someone else", which
is stronger and needs no exemption list to stay true, because neither identity holds a
seat anywhere in the walk. Its **anti-vacuity partner** is the seed pin's shape: after
GREEN unlocks, the TABLE DOCUMENT the store holds is asserted to CONTAIN GREEN's address
— a string this codebase really does write down, which `contains` really does find —
while the unlock's own reply, which is on the wire, does not.

### 1.6 The client (spec §5.1–§5.4, Tasks 7 and 8)

**`web/src/sse.ts` is the transport and knows nothing about the signal.** `parseSse()` is
a stateful line parser over what this server writes — `event:`, `data:`, a blank line
ends a frame, a leading `:` is dropped — holding the tail of a chunk until a newline
finishes it, because a chunk is what a socket delivers and a frame is what the server
meant. `readStream(url, signal, onFrame, onOpen)` opens with `identityHeaders()` and
`accept: text/event-stream`, resolves `"ended"` on an `end` frame or `"closed"` when the
body stops, and rejects a non-200 with the server's own `{"error": …}` sentence.
`EventSource` is not used, because it cannot carry this server's identity header.

**`usePolled` keeps M6.2's signature and changes what runs between its rules.** Under
`signalKind() === "sse"` the effect opens one connection per visible tab and **arms no
timer at all**; hidden aborts the reader; visible again says "refreshing" and reopens,
its on-connect frame being the R-P8 ask; a failure says "reconnecting" and takes the
`BACKOFF_MS` ladder, which stays for the reason it was written; an `end` frame, a 404 or
a 403 fires `onEnd` and the consumer leaves. `VISIBLE_MS` is deleted.

**R-7.1 — `Source` gains one optional field, `frames?: "value" | "wake"`.** Under
`"value"` (the game and the dealing screen) the frame's `data` IS the value and is handed
to `onValue`, which is what keeps R-3.2 true: the SSE arm never calls `/events`. Under
`"wake"` (the open table and the lobby, the default) a frame triggers `source.once()` and
its result is the value, coalesced — one in flight, one repeat for the frames that
arrived under it — because the lobby's `once` is three table listings. The lobby listens
to `/api/stream` and refetches all three scopes on any house frame (R-3.4).

**The link word rests on the CONNECTION, not the request** (Task 7's one fix round): the
`"live"` word moved inside `readStream`'s `onOpen`, which fires after the 200 and before
any frame, so "reconnecting" no longer blinks off at every ladder step and a hanging
server no longer reads "live" forever; `failures` resets there too, so the house stream's
ladder is not monotone; and `open()` aborts its controller in a `finally`.

**The spectator screen** is the game screen with the viewer absent, selected by
`state.seat === null` — the server's own statement, not an inference from the prompt, a
player waiting on somebody else reading `watching` too. The "You are" strip reads **"You
are watching"** and drops the address; `Decision` suppresses the whole answer bar
(options, pick rows, `retreat`, `place_a`/`place_b`, Confirm/Cancel, Undo, and the
near-miss rows, which explain a list a watcher was never offered); the Suggest surface is
filtered out of the banner. `Why`, `Log`, `Areas`, `Help`, the four card indexes, the
replay scrubber and the final score all stay.

**The walk's arrival.** `landed`'s fourth parameter was a nullable seat that conflated
two facts; it became `Arrival = { via: "reply" } | { via: "signal"; viewer: PlayerId |
null }`, and `hasOpponentStep` takes `PlayerId | null` where `null` makes every step an
opponent's. A spectator collides with the old encoding exactly: their run arrives through
the signal AND their viewer is genuinely absent, so `null` read as "the reply to my own
move" and selected the bot gate at a table of people.

**The unlock face** (`Unlock.tsx`) draws at `#/t/<id>` for a caller who is not seated at a
playing table with `can_watch: false` — the table's name, its seats through `TableFacts`
so the lede cannot be a third statement of the seat wording, one password field, a
submit guarded by a REF as well as `busy`. **Games in progress** is the lobby's third
section, drawn through the same row builder so M6.2's ruling-#13 Abandon control stays on
Your-games rows alone. The help panel gained a **Watching a game** section (R-8.1).

### 1.7 The gates (spec §9.4, Task 9)

**The whole-game gate's two-identity arm gained a third identity.** `green@wire.test` is
refused at the locked table, unlocks with the password, and then at every position the
players reach reads the live frame and `…/at/{k}` while a player reads the same `k`.
Three helpers carry it: `assert_watching` (seat is `null`, every row `hand_hidden`, and
**no `Hand` panel at all** — three separate statements, because a server can make any two
true and leak through the third); `public_dashboard`, which strips the hand panels and
normalises `hand_hidden` — measured: without the normalisation the compare fails at
position 0 and `hand_hidden` is the ONLY field that differs across the whole dashboard;
and `watch_position`, which compares the board, the title and the stripped dashboards and
counts the player frames that carried an OPEN hand before the strip. The in-loop read is
keyed on the EVENT COUNT, not the loop pass. Measured in the seeded game: 1,041 positions
watched.

**The browser walk gained a third browser.** In `two_browsers.spec.ts` a watcher signs
in, unlocks, and holds its screen across the whole game, asserting per move that the
account row it lights follows the play — `data-event` on the `TurnAccount` row is a
PRODUCT attribute added for the test, like `data-busy` before it — and, at the end, that
it asked `/events` **zero** times and opened this table's `/stream` **exactly once**.

**The finding that cell was written to find, and it found one.** At `877283d` the watcher
opened `/stream` **2,643 times in one game — exactly one per applied move**. The cause
was the client's, from Task 7: the game signal's `active` term included `!busy`, and the
arrival handler itself sets `busy` around the frame fetch and the walk, so every arrival
tore the reader down and reopened it. The poll's reason for that term — "my own request
is in flight" — does not transfer to a held connection. **Ruling R-9.1 fixed the cause**
(`80f0469`): `active` drops `!busy`, because a player's writes on the game screen happen
only while `your_turn` is true, which already closes the stream; and a frame landing
mid-run is coalesced into ONE follow-up run, never dropped and never concurrent, which is
the hazard the `busy` term was hiding. After the fix the same walk reads `opened /stream
1 times` at every checkpoint. Bounding the reconnects at `checks + 1` instead would have
shipped one TCP and TLS setup per move under the sentence "one connection per tab".

The fix moves the PLAYERS' screens too: a non-acting player now holds its stream across
the opponent's move and its own walk.

### 1.8 The fences held

`crates/yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge` and
`gen-data` are **byte-identical to `master`** at this HEAD; `SEMANTICS_REV` is **7**; no
bot behaviour moved. `crates/yokohama-cli`'s diff against `web-room-m6.2` is `repl.rs`
alone (§1.4). Against `master` the CLI shows M6.1's own `wizard.rs` line as well, because
this branch is stacked — the honest check on a stacked branch is against its base, and
the per-commit check is `git show --stat 0aef313`, which is `repl.rs` and `lib.rs` and
nothing else.

---

## 2. The counts at the close HEAD

Measured at `9a4626d` in **ONE uninterrupted sequence**, 17:48–18:26Z on 2026-09-16, on
the **Mac** (darwin 27.0.0, toolchain 1.97.1), with the BGA captures present so the CLI
figure is a FULL reading. Nothing below is copied from a task report. The log is
`.superpowers/sdd/2026-09-15-web-frontend-m6.3-plan/counts-sequence.log` (the SDD
workspace is gitignored; the figures are quoted here).

| suite | at the open (`be21a4b`) | at this HEAD (`9a4626d`) | wall |
|---|---|---|---|
| `cargo test -p yokohama-web` | **149** (141 lib + 2 gate + 6 wire) | **191** (**181** lib + 0 main + **2** gate + **8** wire) | lib 15.13 s; the gate see below |
| `cargo test -p yokohama-cli` | 850 + 2 ignored | **851 + 2 ignored** (823 lib + 9 `bga_import` + 18 `guide_sync` + 1 `piped_dogfood`) | 38.0 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | 1.4 s (warm) |
| `cd web && npm run typecheck` | clean | clean | — |
| `cd web && npm test` | **522** in **33** files | **587** in **35** files | — |
| `cd web && npm run e2e` | **39** in 8 spec files, 11.7 min | **39** in 8 spec files | **6.9 min** |
| `ls web/src/wire/*.json \| wc -l` | **62** | **71** | — |

**The whole-game gate FAILED in the sequence, and the cause is not this cycle's code.**
`tests/play_a_whole_game.rs` read `1 passed; 1 failed` after **1813.02 s**: the vs-BOT
arm livelocked on an OS-entropy seed and hit its 20,000-input cap. The two-identity arm —
the one M6.3 changed — passed. The binary was re-run alone immediately afterwards at the
same commit and read **`2 passed` in 26.00 s**. Both readings are recorded because both
are true of this code: the arm is a coin flip on the seed the OS draws, diagnosed in §6
as an OWNER ITEM and pre-existing (it is the M6.1 close's "13 s / 857 s / 16 s" wall).

The exact lines:

```
=== counts sequence at 9a4626d 2026-09-16T17:48:15Z    dirty=0    disk 9.4 GiB free
cargo test -p yokohama-web
  test result: ok.     181 passed; 0 failed; 0 ignored   (src/lib.rs, 15.13s)
  test result: ok.       0 passed; 0 failed; 0 ignored   (src/main.rs)
  test result: FAILED.   1 passed; 1 failed; 0 ignored   (tests/play_a_whole_game.rs, 1813.02s)
  test result: ok.       8 passed; 0 failed; 0 ignored   (tests/wire_fixtures.rs)
cargo test -p yokohama-cli
  test result: ok.     823 passed; 0 failed; 0 ignored   (src/lib.rs, 30.61s)
  test result: ok.       9 passed; 0 failed; 0 ignored   (tests/bga_import.rs — the captures are present)
  test result: ok.      18 passed; 0 failed; 0 ignored   (tests/guide_sync.rs)
  test result: ok.       1 passed; 0 failed; 2 ignored   (tests/piped_dogfood.rs)
clippy_exit=0      typecheck_exit=0
vitest    Test Files  35 passed (35)   ·   Tests  587 passed (587)
e2e       39 passed (6.9m)
corpus    71
=== done 2026-09-16T18:26:41Z
=== gate re-run (the vs-bot arm livelocked in the sequence above) 2026-09-16T18:27:09Z
  test result: ok. 2 passed; 0 failed; 0 ignored   (26.00s)
```

**The browser walk's own line**, from the same sequence:

```
two-browser game sim-83f36018ed3fb7a2 finished in 2276 moves, 0 idle looks, 309s;
largest hand seen 3; the watcher was held to 1762 applied moves, asked /events 0 times
and opened /stream 1 times   (1,1,1,…,1 at the checkpoints — 88 of them, all 1)
```

**Read the wall at its real size.** At the open the same cell cost **10.1 min / 604 s
with 999 idle looks**; here it is **309 s with 0 idle looks**. The idle looks are the
figure that means anything: they counted a tab waiting out a 3 s poll, and there are none
left. **The wall is now the GAME's length, not the poll's** — the server draws the seed,
and this cycle's readings of the cell were 151 s, 202 s, 187 s, 310 s, 308 s and 309 s,
over games of 1,347 to 3,855 loop iterations. "Moves" on that line are loop iterations,
not applied moves; a raced no-op counts the same, and the watcher's own figure (applied
moves) is the one printed beside it. **No wall here is a budget** (§7).

The lib grew by 40 cells (141 → 181): Task 1's fifteen signal cells plus its fix round's
`MirrorGate`, Task 2's stream and `?wait` cells, Task 4's admission grid, write sweep,
card-index and `why` cells with `table.rs`'s two and `tables.rs`'s six (unlock's ladder,
the write-free unlock by COUNT, dedup and the two-watcher claim, the 257th watcher,
`table_for_anyone`, the `playing` scope) and `client.rs`'s four projection cells, and the
fix wave's one `Flaky`-store cell. `wire_fixtures` went 6 → 8: the stream's by-value
compare and the watcher's-email pin. The client grew by 65 (522 → 587) across two new
files (`sse.test.ts`, `Unlock.test.tsx`) and the rewritten hook cells, the spectator and
arrival cells, and the fix wave's four.

**The blast radius**, at this HEAD:

```
git diff --stat master -- crates/yokohama-core crates/yokohama-oracle \
                          crates/yokohama-difftest crates/yokohama-bridge crates/gen-data
  (empty — 0 files)

git diff --stat web-room-m6.2 -- crates/yokohama-cli
  crates/yokohama-cli/src/repl.rs | 46 ++++++++++++++++++++++++++-------
  1 file changed, 39 insertions(+), 7 deletions(-)

SEMANTICS_REV = 7   (crates/yokohama-core/src/game/semantics.rs:180)
```

**The fifteen-gate release roster was NOT re-run**, for the reason the M6.1 and M6.2
closes gave: every crate the roster measures is byte-identical to `master`, so a re-run
could only re-measure master's own reading. Nothing added in this cycle is `#[ignore]`d;
the whole-game gate stays out of CI and per-edit locally.

### 2.1 R-3.1 — the tunnel measurement, TAKEN both ways: SSE SHIPS

**Taken 2026-09-16 against the deployed stream, not against an rc.** The owner authorized
the release in-session, so plan Task 3's rc step was folded into Task 10: HEAD itself was
deployed as `yokohama-web:0.5.0-m6.3` (container `4f37b4bb…`, started 19:04:04Z) and both
readings were taken through the tunnel on the table `sim-f68b6accb83c4073`, created for
the purpose and deleted afterwards. Every figure below is quoted from the logs
`reading-a.log` and `reading-b.log` in the session scratchpad; the deployment record is
`deploy/RUNBOOK.md` §3.9.

- **Step 1 — the rc.** *Folded into the release* (owner authorization; nobody was playing
  on the NAS). The only extra image cut was reading B's `-rc2`, and it was removed again.
  *Authorized: the owner, in-session, 2026-09-16 · deployed at: 19:04:04Z.*
- **Step 2 — reading A, with the heartbeat** (19:06:29Z–19:16:29Z, seat 1 holding
  `…/stream` while seat 0 played). On-connect frame **19:06:30.3Z** —
  `event: signal` / `data: {"events":0,"to_move":0,"status":"playing","your_turn":false}`.
  Heartbeat comments: **23 of them, each 25 s after the last MESSAGE on the stream** —
  19:06:55.3, 19:07:20.3, 19:07:45.3 … 19:15:15.3 (twenty-one before the move), then
  19:15:55.3 and 19:16:20.3 after it. The keep-alive restarts from a frame, not from the
  previous comment, which is why the one gap that is not 25 s is the 40 s across the move.
  The minute-nine move: A posted `1` (EndAdditionalPhase) at **19:15:29Z (540 s)**
  and the frame `{"events":1,"to_move":0,"status":"playing","your_turn":false}` arrived at
  **19:15:30.3Z**, under one second. Still open at minute ten: **yes** — the hold ended
  because `curl`'s own `--max-time` expired (exit 0), not because anything closed it.
- **Step 3 — reading B, without the heartbeat.** `HEARTBEAT` was set to 3600 s in a
  working copy only (reverted; `git status` clean), built and deployed as
  `yokohama-web:0.5.0-m6.3-rc2` (container `33021c62…`, live ~19:17:5xZ–19:20:3xZ for the
  reading alone). B held the IDLE stream from **19:18:05Z**: the on-connect frame at +0 s
  and nothing after it. *Drop observed at:* **19:20:10Z, +125 s**, `curl` exit 92 — an
  HTTP/2 stream error, i.e. the edge and not the origin. The spec expected it near 100 s;
  125 s is the measurement. The release image was then redeployed (container `f0c371c7…`,
  started 19:20:51Z) and a 30 s hold showed the on-connect frame at +0 s and the heartbeat
  at +25 s again.
- **Step 4 — the decision.** Reading A held ten minutes with the frame arriving, so
  ***SSE ships and `SIGNAL_KIND` stays `"sse"`.*** The held-request arm stays built, dark
  and one constant away; R-F.1's residual at `HELD_REASK_MS` stands unexercised. Reading B
  is what makes the decision a measurement rather than a hope: without the 25 s comment
  the same stream dies in about two minutes, so the heartbeat constant is now load-bearing
  against a number, and the Yokohama ingress comment in `deploy/cloudflared-config.yml`
  says so at the place someone would change it.

---

## 3. The corpus diff

`web/src/wire/` is a contract, not a snapshot. It was regenerated at Tasks 2, 4 and 6,
and every diff was read. **62 files at the open, 71 at the close**; `4,253` insertions
and `9` deletions across twenty files including the two contract modules.

| where | change | why |
|---|---|---|
| Task 2 | `health.json` gains `"signal": "sse"`; **NEW** `events_open.json` (the `events` route at an OPEN table, captured as a seat holder: `events: 0`, `status: "open"`) | `Health.signal` names the transport; R-3.3's open arm |
| Task 4 | **EIGHT** `TableView`-bearing files gain `"can_watch"` and NOTHING else — `deal.json`, `deal_waiting.json`, `games.json` (2 rows), `games_next.json`, `games_open.json` (2 rows), `table.json`, `table_dealing.json`, `table_open.json`. **Every `ClientState` and `HistoricalFrame` fixture is BYTE-IDENTICAL** | `Option<PlayerId>` serializes `Some(n)` as `n`, which the corpus is what proves |
| Task 6 | **EIGHT NEW**: `spectator.json` (a watcher's live frame at a position with an assistant really on the board — `seat: null`, every hand hidden, prompt `watching`, no `Hand` panel), `spectator_at_k.json` (the SAME stop as `at_k`, so the pair differs in exactly one thing: who asked), `spectator_over.json` (`game_over`, scores present, still no hand), `games_playing.json` (`scope=playing`), `table_watchable.json` (a locked playing table read by the caller who never knocks: `can_watch: false`), `unlock_403.json`, `unlock_409_open.json`, `watch_403_locked.json`. **ZERO existing files changed** | the third and fourth identities; `wire2` was already an open unlocked table at HEAD, so the `STRANGER` → `GREEN` rename was byte-neutral |
| the fix wave | none | it changed docs, one server arm and client cells; no payload shape moved |

The client's `contract.ts` gained nine `Widen<T>` rows (the eight new files plus
`events_open.json`, which Task 2 committed and no client row had ever read), two §1b
prompt rows, and the two spectator `ClientState`s in `EVERY_STATE`; `contract.test.ts`
gained seven root-key rows, the mask row's null branch and three cells. `KEYS` needed no
change — `can_watch` and `signal` were already listed, because Tasks 2 and 4 made the
two-line client change in the same commit as the server field (R-2.2, R-4.1), so vitest's
root-key rows never went red between tasks.

---

## 4. The reviews

Nine implementer dispatches, nine per-task Opus reviews, **six fix rounds** (Tasks 1, 2,
4, 7, 9 and the final wave), every fix re-reviewed in scope; one Fable whole-branch
review; one fix wave; one mutation battery — and, at the release, one Fable live review
against the DEPLOYED site (§4.5, added by the amendment). Every report is in
`.superpowers/sdd/2026-09-15-web-frontend-m6.3-plan/` — **the SDD workspace is
gitignored**, so the findings are quoted here rather than linked to.

### 4.1 The per-task reviews

| task | verdict | C/I/M | fix rounds |
|---|---|---|---|
| 1 — the channels | spec ✅ with ⚠️ | 0 / 1 / 5 | 1 (`ef4c257`) |
| 2 — the streams | spec ✅ with ⚠️ | 0 / 2 / 7 | 1 (`2b32737`) |
| 4+5 — the spectator, the CLI touch | spec ✅ both, two sanctioned deviations | 0 / 3 / 8 | 1 (`ce07c5c`) |
| 6 — the wire | COMPLIANT, all three deviations accepted on merit | 0 / 0 / 6 | 0 |
| 7 — the client's reader | compliant | 0 / 1 / 8 | 1 (`b4cfc1f`) |
| 8 — the spectator screen | Compliant, both out-of-radius touches judged strengthenings | 0 / 0 / 7 | 0 |
| 9 — the gates | Compliant in substance | 0 / 1 / 9 | 1 (`26da94f`) |

The four worth carrying:

- **Task 1 (Important 1 → R-1.1):** `persist`'s bump-after-`mirror_turn` ordering was
  unasserted. Spec §2's "the cell for each site parks the write" is binding over the
  brief's "one gated cell", and `persist` is the only site with two sequenced writes, so
  a hoisted bump there is a stale lobby row. The `MirrorGate` cell is the fix.
- **Task 7 (Important 1):** the link read "live" when the request went OUT. "Reconnecting"
  blinked off at every ladder step and a hanging server read "live" forever — §1.6.
- **Task 9 (the cell doing its job):** the 2,643 reopens — §1.7. The spec's own §9.4 says
  "a reconnect during the wait fails the cell and is reported as a finding", and it was.
- **Task 9 (Important 1):** `await watch(page)` sat inside the `try` whose `catch` was
  written for a click that raced the poll, so on a turn-ending move every per-move watcher
  assertion was SWALLOWED and the anti-vacuity counter counted attempts, not successes.
  Proven in both directions: a probe throwing only on a turn-ending hold fails the fixed
  cell and passes with the hold back inside the catch — 911 swallowed holds, green.

One process finding, restated because it cost round trips again: **crossed messages**.
Task 9's implementer committed the gates before ruling R-9.1 reached it. `877283d`
STANDS — this repo's rule is a new commit and never an amend — and R-9.1 landed as two
commits on top, so nothing in the branch ever asserts `streams === 1` against the
unfixed client.

### 4.2 The whole-branch review (`final-review`, Fable, on `be21a4b..26da94f`)

Nineteen commits, fifty-eight files, read in four passes; the reviewer ran six focused
lib cells (6/6 in 9.3 s), five vitest files (71/71), a spot check of the eight new
fixtures and the three fences, and did not mutate the tree. Verdict **"ready to merge
WITH FIXES"**: **0 Critical, 3 Important (one dormant), 8 Minor**, and **eight false
sentences at HEAD**.

The three Importants, all fixed in the ONE fix wave:

- **I-1 — the coalescing loop's "fetch again" pass had no witness.** The cell resolved
  the held read with the count as it stood at RELEASE, so the pending frame hit `continue`
  and the follow-up pass fetched nothing; `frame = pending.current` → `frame = null` left
  every assertion holding. Product code correct by reading; the CLAIM was unproven.
- **I-2 — a real abandon reached the dealing and open-table screens with NO sentence, and
  the two cells that said otherwise drove a different path.** The delete path is an `end`
  frame with a reason and no words; both cells modelled a dropped connection and a 404 on
  the reopen — reachable (a server restart), but not what `delete` does.
- **I-3 (dormant) — the held arm could delay a missed bump by up to 28 s**, because
  `HELD_REASK_MS` was 3 s for EVERY source including the game's, whose `once` is
  `events?wait=25` and HOLDS. It makes the plan's own sentence "the fallback is a one-line
  change in the reader" untrue, so it was fixed now rather than at the flip (R-F.1).

**The eight false sentences**, all corrected in the wave: the `events` doc paragraph
("seated, like every other game route"; "the stranger's 403 comes from `Games::table`" —
neither true after R-4.3); `NOT_SEATED`'s "a stranger cannot tell the routes apart by
their refusals" (the sweep two lines below pins exactly that difference); two cell titles
on the abandon path (I-2); two `App.test.tsx` sentences claiming "one follow-up read
answered all three frames" where the assertion is `toHaveLength(2)`, which is no follow-up
at all (I-1); the gate's "a locked table refuses a stranger's read before the password",
which would pass at an UNLOCKED table; `table.rs`'s "`dealing` is the only status that
withholds the roster" (a seat holder reads it at every status) and the same paragraph
dating M6.2's open widening to M6.3; `App.tsx`'s "a frame that lands during a run is
REMEMBERED rather than dropped" (only the newest is, which is correct because counts are
cumulative); and the plan's own R-plan-5.

And the sentences the reviewer verified TRUE **by trying the mechanism**, which is the
method that finds the other kind: `signal.rs`'s module doc (both gated cells park the
real write and read the receiver quiet); `stream`'s "subscribes only after the admission
has answered" (the 404 cell counts senders) and "no await holds a game lock";
`client.rs`'s stand-in paragraph, re-read at `screen_model.rs:2675–2711`; `signal.ts`'s
"arms NOTHING while connected" (zero timers across sixty fake seconds, one `/stream`,
zero `/events`); `App.tsx`'s `active` paragraph (`api.input`'s two call sites are both
turn-gated); `events_at`'s "the ONLY row where the two part company"; both gates' headers.

### 4.3 The fix wave (`9214661`, `9f70aa2`, `9a4626d`)

Eleven items — the three Importants, the four fix-before-merge doc corrections, and the
cheap named-fix minors R-F.2 swept in while the files were open. **11/11 addressed**, the
scoped re-review PASS. Four mutations run: restoring the single `gone` arm fails the
`Flaky` cell; `frame = null` fails the new follow-up witness while the no-op cell stays
green (which also confirms the review's finding that the old cell could not fail);
`HELD_REASK_MS` for every source fails the held-at-once cell; dropping `?? endSentence`
fails all three end-frame cells.

**One deviation from the review's wording, stated plainly:** the review asked the I-1
cell to reach three account rows; it ends at TWO, because `setRun` REPLACES and the two
passes walk `base→base+1` and `base+1→base+3`. The three events are carried by the
positions assertion `[base, base+1, base+1, base+2, base+3]`, and the cell's doc says so.

The re-review then found **three new doc-only breakages** — the `NOT_SEATED` paragraph
listing `keep` among the writes that answer it, and the abandon's attribution on three
screens. **Ruling R-F.3** parked them for this doc pass rather than a second wave; they
are fixed in the commit that precedes this one.

### 4.4 The mutation battery (`mutation-battery`, Opus, at `9a4626d`)

Spec §9.5's twelve plus this cycle's three. **15 of 15 CAUGHT; none survived.** One at a
time, applied by a scripted edit, `git status --short` asserted EMPTY before the next,
nothing committed, the first failing assertion recorded verbatim. The catchers span the
whole branch: a spectator's `hand_hidden` flipped to false fails two `client.rs` cells; a
spectator admitted to `input` fails the write sweep and the admission grid; `can_watch`
ignoring the lock fails **nine** cells across four modules; `persist` not bumping, the
hook arming a timer while connected, `unlock` writing on a wrong password, the roster
re-served under a new key, the frame built for the ACTING seat, the served run taking
`hasBotStep`, `active` reading `busy` again, `"live"` before the 200, and the coalescing
loop dropping its pending frame — each caught by the instrument written for it.

**Two readings this close carries because a re-run cannot produce them:**

1. **Mutation 8 is the only row where a NAMED catcher did not fire, and it is not a
   finding.** The roster re-served under a new key was caught by BOTH Rust catchers — a
   by-key unit cell and the corpus value pin — while the client's `contract.test.ts` did
   not fire, because it reads the COMMITTED corpus and the battery deliberately did not
   regenerate it. The client catcher fires at regeneration time, as designed.
2. **Mutation 11's catcher is one checkpoint earlier than the spec's sentence suggests.**
   The visibility assertion sits behind `watchChecks % 20 === 0` and is unreachable when
   the tab is hidden from the start, because no hold ever succeeds; the account-row
   assertion at the FIRST hold is what makes the "no `/events`" claim non-vacuous.

Both browser mutations aborted in about two minutes rather than a green run's thirteen,
because the cell stops at its first failed hold; neither figure is comparable with a
passing run's wall.

### 4.5 The live review against the DEPLOYED site (`live-review`, Fable, 2026-09-16)

Added at the release. A clean Fable subagent with no part in the build drove **three
Chromium contexts as three people** — two playing, one arriving by link and watching —
against `https://yokohama.moore.nyc` running `yokohama-web:0.5.0-m6.3`, in both layouts
(desktop 1440×900 and mobile 390×844), counting every request off each page's own request
log rather than estimating, and photographing 82 screens.

**Verdict: SHIP — 0 Critical, 0 Important, 3 Minor.** Twenty-five claims the close's §1
makes about what a player and a watcher see were tried and found TRUE in both layouts,
with **zero console errors on all three screens** across the review.

**The exit criterion, on the NAS.** The watcher asked `GET …/events` **zero** times and
opened `…/{id}/stream` **once, with zero aborts**, through two full turns (a 5.5 s window
desktop, 7.1 s mobile: 9 frames, 16 `/at/{k}`, 1 `…/table`), through a 444 s game that
reached **1,471 events** (`/events` 0, stream opens 1, frames 1,464, `/at` 2,926) and
through a whole game against the bot (485 events: `/events` 0, stream opens 1). The tab
was asserted `visible` at every applied move. The watcher's frame fetch left **0–9 ms
BEFORE the mover's own reply landed** — the bump is sent after the write and before the
reply returns — so the signal reaches a watcher inside the mover's round trip.

**Two readings worth carrying beside it.** The PLAYERS reconnect once per turn boundary by
design (144 and 145 opens over 287 mover changes in the 444 s game, median gap 1.3 s = the
other player's turn): "one connection per tab" is the sentence, and "one connection per
turn" is a player's reading of it. And the link strip drew **no word at all** while live —
a MutationObserver on every game screen in every run recorded `null` throughout, so
"refreshing…" and "reconnecting…" never appeared in 444 s through the tunnel.

**The three Minor findings**, none of which holds the release and none of which produced a
point release:

1. **M-1 (reproduced) — a watcher on a FINISHED game is not told when the creator abandons
   it, and stays on the dead screen.** `useSignal`'s `active` is `!!state && !state.over
   && !state.your_turn` (`web/src/App.tsx:653`), so the stream is CLOSED at game over and
   the `end` frame has nobody to reach; the next per-game GET 404s into a panel, which
   renders `no game named "…"` and leaves the screen where it is. **This falsifies the fix
   wave's I-2 sentence for one status** — "an abandon leaves a SENTENCE on every screen" is
   true at `playing` (claim 24 below was measured at 64 ms) and false at `over`. Either
   one-liner closes it: route a 404 from any per-game route on the game screen to
   `toLobby(endSentence("gone"))`, or drop `!state.over` from `active` at the cost of one
   idle connection per finished-game tab. → §6.2.
2. **M-2 (reproduced) — every arrival at `#/t/<id>` by link or reload draws the LOBBY for
   one round trip**, opening the house stream and firing three listings that are thrown
   away (`/api/stream` open at 43.15 s, three `?scope=` reads, abort at 43.23 s, the table
   stream open at 43.23 s). `App`'s screen selection falls through to `<Lobby>` while
   `open(id)` is still awaiting `GET …/table`. Fix: render "opening…" while a table route
   is pending. → §6.2.
3. **M-3 (suspicion, NOT reproduced) — a player whose TURN it is has no stream open** (by
   design), so an abandon under them is learned only when they post, and the refusal lands
   in the error strip with the screen staying put. In the run that abandoned mid-game it
   was the other seat's turn, so that seat left for the lobby with the sentence in 6 s.
   Same class and same first fix as M-1; recorded so it is not mistaken for tested. → §6.2.

**Not reachable with this instrument:** the hidden-tab "refreshing…" word. Playwright
cannot hide a tab — with a second page brought to front in the same context
`document.visibilityState` stayed `visible` in headless AND headed Chromium — so the
stream was never let go and no word could appear. The mechanism is held by the client's
unit cells; on the live site it is untried, and a human with two real tabs is the
instrument. **Game over at a table of two PEOPLE** was also not reached (1,471 events of
random play without an end), so the game-over claims were tried at a bot table.

**Housekeeping:** seven tables were created during the review; five were deleted by their
creator and two were abandoned through the lobby UI as the finding under test. The
measurement table `sim-f68b6accb83c4073` was deleted afterwards, and the house is empty.

The report, the per-identity request logs, the shell stream reads and the 82 screenshots
are in the session scratchpad; the SDD workspace is gitignored, so the findings are quoted
here rather than linked to.

---

## 5. The rulings taken in execution, with their costs

**Twenty-one rulings** taken by the orchestrator, in the order they were taken. **Cost**
is what is wrong if the ruling is wrong. The whole-branch review read them and disputed
none. (The plan's own R-plan-1–R-plan-5 are restated after the table.)

| # | task | ruling | cost if wrong |
|---|---|---|---|
| R-pre-1 | T4+5 | Tasks 4 and 5 go to ONE implementer with Task 5 committed FIRST, so `card_index` passes `Some(viewer)` for one commit and is re-pointed on the `Viewer` in the next — **no commit ever passes a stand-in seat to the card index** | none; the CLI diff is still its own commit |
| R-pre-2 | T2 | `SIGNAL_KIND` is a `pub const` in `lib.rs`, not an `AppState` field; `Health.signal` reads it. R-plan-4's text and Task 2's code both say so; the plan's Interfaces line was the stray | a one-line field later |
| R-pre-3 | T4 | **`Games::unlock` bumps NOTHING.** A watcher's admission changes nothing a lobby or a table screen shows — `can_watch` is per-caller and the caller has the reply in hand — and spec §2 enumerates the sites without it | a screen watching another caller's `can_watch` would not refresh; no such screen exists |
| R-pre-4 | T7 | `readStream(url, signal, onFrame)`, headers from `identityHeaders()` inside | none |
| R-pre-5 | T7 | Under `held`, after `once` resolves the hook re-asks at `HELD_REASK_MS = 3000` rather than at once, because the lobby's and the open table's `once` have no waiting endpoint and a reopen at zero is a hot loop | up to 3 s added latency on the dark arm only — **and this was later HALVED by R-F.1**, which re-asks at 0 for the sources whose own request holds |
| R-pre-6 | all | Execution order T1, T2, T4+5, T6, T7, T8, T9, the review, the battery, the counts — **then stop for the owner** (T3, T10, T11's live review). `SIGNAL_KIND` stays `"sse"` throughout, since the held arm ships regardless | if the measurement falls back, the flip is the constant plus the held cell — both built |
| R-1.1 | T1 | `persist`'s bump-after-`mirror_turn` ordering is a FIX, not a LEAVE: spec §2's "the cell for each site parks the write" is binding over the brief's "ONE gated cell" | ~20 test lines nobody needed |
| R-2.1 | T2 | tokio may gain `"time"` in the same M6.3 paragraph (the `?wait` arm's `timeout` and the test cadence need it; R-3.11 named only `sync` because the spec did not foresee the held arm's timer) | one feature flag |
| R-2.2 | T2 | Task 2 ALSO makes the two-line client change (`Health.signal` in `types.ts`, `"signal"` in `contract.ts`'s HEALTH key list) so vitest's root-key row stays green when `health.json` is regenerated | two lines Task 6 would have written |
| R-4.1 | T4 | The same shape for `TableView.can_watch`; `ClientState.seat`/`HistoricalFrame.seat` stay `PlayerId` in the client until Task 6, the fixtures being byte-identical | two lines Task 6 would have written |
| R-4.2 | T4 | **`join` keeps M6.2's refusal order at every status and is NOT routed through `admitted`** — spec §3.5 row 2 says "join as M6.2" and the route is already status-gated | a 409 where the spec's writes column reads 403, at one route |
| R-4.3 | T4 | **`events` and `stream` answer ANY signed-in caller at an OPEN table** (spec §3.5 row 2), via `table_for_anyone` before `admitted`; `admitted` unchanged, so every other read keeps its 403 at open. Sent as a PRE-REVIEW correction, not counted as a fix round | an `Events{0, open}` object reaches strangers — the same facts `table` already answers them |
| R-4.4 | T4 | `deal`/`keep`/`join`/`delete` are NOT routed through `admitted` — they never went through `seated`, and `Games` refuses a seatless caller before any record read. `spectators_write_nothing` covers `input` and `suggest`, the two that were behind `seated` | none observable; M6.2's refusal order preserved |
| R-6.1 | T6 | **No `wire.houseFrame` fixture.** The plan's interface list names it "not a route file — see below" and nothing below defines it; the house frame is `{"seq": N}` and Task 7's fake stream scripts it | one three-line JSON file later |
| R-6.2 | all | Commit trailers are the CONTROLLER's (`Claude Fable 5.1` + this session's `Claude-Session`), not an implementer's own environment line — the attribution is the session's | cosmetic |
| R-7.1 | T7 | **`Source<T>` gains ONE optional field, `frames?: "value" \| "wake"`** (default `"wake"`), because two of the four consumers want a different value type than an `Events` frame — the open table re-reads `api.table`, the lobby re-reads its lists. M6.2 fixed the `Source` signature so M6.3 could swap the transport beneath it, and one additive optional field is inside that intent | one optional field to remove; the plan's "the lobby's `onValue` always refetches" is the same behaviour expressed at the hook |
| R-8.1 | T8 | Task 8 also writes the help panel's **Watching a game** section, following M6.2's precedent and `Help.test.tsx`'s emphasis-boundary rule — a live review would flag a help panel silent on the cycle's user-facing feature | a paragraph to strike |
| R-9.1 | T9 | **FIX THE CAUSE of the 2,643 reopens in Task 9**, blast radius widened to `App.tsx` with its own commit: `active` drops `!busy` (a player's writes on the game screen happen only while `your_turn`, which already closes the stream) and the handler coalesces a mid-run frame into one follow-up run. The spec's count governs; bounding at `checks + 1` would have shipped one TCP and TLS setup per move under the sentence "one connection per tab" | if some other write path existed on the game screen while `your_turn` is false, a self-bump could be walked as an opponent's — the handler's `events` compare and the coalescing make that a no-op, and the two-browser cell plays a whole game under it |
| R-F.1 | wave | I-3 is fixed NOW, not at the R-3.1 flip: under `held`, re-ask at 0 for `frames === "value"` sources and keep `HELD_REASK_MS` for `"wake"`. The plan's "the fallback is a one-line change" must be TRUE when the constant flips. **The residual is written at the constant** (§7) | two lines in a dark arm |
| R-F.2 | wave | The wave also takes the reviewer's named-fix minors that are cheap while the files are open — M-5's `Err` split, the four hand-authored `Events` frames, T8 #1/#2, T9 #4, and the doc sentences | a slightly larger wave to re-review |
| R-F.3 | close | The three attribution sentences the re-review found are fixed in the CLOSE's doc pass, not in a second wave; nothing on the wire, nothing blocking | three false comment lines for one commit |

### Restated from the plan (R-plan-1 … R-plan-5)

- **R-plan-1** — the measurement is Task 3, after the stream exists. **Taken at the
  release, both ways; SSE ships** (§2.1).
- **R-plan-2** — `admitted` returns `(TableDoc, Viewer)` and routes needing a handle ask
  after; the nine `seated` sites became `admitted` + `handle` pairs. Done.
- **R-plan-3** — the stand-in is `(current_player + 1) % players`, one line, and the cells
  prove the three consequences at 2P/3P/4P rather than the choice of seat. Done.
- **R-plan-4** — `Health.signal` from a compile-time constant, not an environment
  variable: the fallback is a code decision recorded in the close, and a deployment
  cannot flip it. Done (R-pre-2 places the constant).
- **R-plan-5** — under the fallback the reader is the same `fetch` to `…/events?wait=25`
  and the `?wait` arm ships regardless. Done, **and its sentence was false as written**
  until R-F.1 (§4.2, I-3).

### Deviations from the spec's letter, recorded here

- **§8 — the stream's frame compare.** The spec says the first frame is compared against
  the committed `events.json`; it is compared against a LIVE `/events` read on a second
  table in the same instant. Arguably stronger (two transports, one object, one instant),
  and it is what the task's brief asked for.
- **§9.4 — the two-browser cell's `CAP`.** The spec does not name one; the cell has
  `CAP = 4000` loop iterations and **it is being hit on long OS-entropy games** (§6).
- **§4 — the replay-frame spectator cell runs at 3P only** where §4 says 2P/3P/4P; the
  live payload and the before-drop mechanism are pinned at all three.
- **§9.5 — the battery is fifteen mutations, not twelve**: the spec's twelve plus this
  cycle's three (`active` reading `busy`, `"live"` before the 200, the dropped pending
  frame), all three of them findings this cycle made about itself.
- **Task 6's three fixture choices**: `unlock_403` uses the disjoint `WIRE_WRONG_PASSWORD`
  needle rather than the brief's `"open"` (M6.2's needle lesson); `spectator_at_k` is at
  `total/2` following the `at_k` precedent, because at `k == total` the payload IS the
  live frame and at `k == 0` the board is empty; `games_playing` carries ONE row, because
  two would race the clock for their order.
- **Task 8 kept the `locked` block and the finished game's standing** beside the
  `watching` line where the brief said "render ONLY the watching line" — a spectator at a
  finished game carries prompt `game_over`, so the brief's wording would have rendered
  nothing at all under "The game is over."

---

## 6. The LEAVE list

### 6.1 Inherited and not touched by this cycle

Every M6.2 close §6.1 and §6.2 item stands except where §6.3 below says otherwise. The
M6.1 items carried through M6.2 carry through here unchanged.

**Closed by this cycle:** the M6.1 "a DELETED game reads reconnecting… forever" item is
now closed in BOTH halves — the one-shot half closed in M6.2 through `usePolled`'s
`onEnd`, and the stream half closes here with the `end` frame and the 403/404 arms; the
two-browser cell's cost is transformed rather than closed (309 s of GAME instead of 604 s
of poll).

**Still open, and checked at this HEAD rather than assumed: `Dealing` still never
re-reads while waiting.** Spec §2 gives `keep` a `bump_table` with the reason "the dealing
screen's 'waiting on' line must refresh", and the bump is sent — but the screen's
`onValue` (`web/src/Dealing.tsx:203`) acts only when the status LEAVES `dealing` and does
nothing with a `dealing` frame, so the 3P+ staleness M6.2 recorded stands. The transport
is in place for it; the handler is one line short.

### 6.2 This cycle's own — the final review's LEAVE triage, grouped as it groups it

**Task 1.** The `end` cell's `||` asserts the weaker property (split it into two asserts).
The house sender is `Mutex<Option<watch::Sender<u64>>>` purely to buy `#[derive(Default)]`
— an eager `impl Default` drops the `Option` and a branch from the hot path. The keep
cell's second half asserts only the house; one `rx.has_changed()` after blue's keep would
pin the table's second wake. `MirrorGate` identifies `mirror_turn`'s write POSITIONALLY
(the first `Doc::Table` write after arming inside `persist`), so a table write added to
`persist` before `mirror_turn` would re-point it with the cell green.

**Task 2.** `/stream` is in no route sweep and nothing ties `session_routes()`
(`lib.rs`) to the router — a cell walking the router's own route list against
`session_routes` plus a named exemption list would fence the next absent route; the
admission grid covers `/stream`'s admission per cell. The absence of `id:`/`retry:` is
asserted over body TEXT (`wire_fixtures` also reads them per line). **The `?wait` 400 is
returned BEFORE admission**, so a stranger asking `wait=26` gets a 400 and not a 403; the
order is unpinned and leaks nothing. `wire_fixtures` now plays a SECOND full game
(create + two keeps) for the stream compare — a small cost in one of two CI jobs.
**Blocking store I/O runs inside the stream's `poll_next`** once per bump per connection,
with a game-lock acquisition, and no `spawn_blocking`: the crate's existing handler
pattern multiplied by open connections rather than by requests, bounded by open tabs, and
the vs-bot and hot-seat games never open one.

**Task 4.** The viewer seat is computed under a DIFFERENT lock acquisition from the frame
(`admitted` locks and drops; the route re-locks), so at a hot-seat table a turn advancing
between the two makes `input`'s acting-seat fence 403 spuriously ONCE, self-healing; the
review traced both directions and it can never admit wrongly. **`events_at` reads and
settles the table TWICE per call** at dealing/playing (once via `table_for_anyone`, once
inside `admitted`); an `admitted_with(table, …)` overload removes it, and after Task 7
the route is the stream — once per bump per connection, not every 3 s. `input`'s
spectator arm is written twice (`?` then `unwrap_err`); a `let … else` gets the
exhaustiveness in one statement. `NOT_SEATED` is shadowed by two test-local constants.
The write sweep's `overt` arm is indistinguishable from `playt` under the record-refusing
store. **Nothing asserts R-pre-3** (that `unlock` bumps no signal) — one `senders()` line
beside the success arm. The replay-frame spectator cell is 3P only. The unlock cell's doc
says "every adjacent link" while `(status, can_watch)` has no probe.

**Task 6.** Four naming and doc polish items; the corpus binary's growth is recorded
above.

**Task 7.** `reread()`'s `src.current!.once()` non-null assertion on a value that can
become null (the throw lands inside the `try` and is harmless). **No cell drives a
`"wake"` source under `held`.** `App.tsx`'s `api.health().catch(() => {})` records
nothing — a silent catch in a strict repo, though the default `"sse"` is the right guess.
**`usePolled` is still named `usePolled` and nothing it does is a poll** — the spec fixed
the NAME along with the signature, and renaming it is a decision, not a task's. At mount
`link` starts `"live"`, so a FIRST connection that hangs shows the resting word. The
harness's `openStream()` ignores aborts. One cell title over-claims.

**Task 8.** The unlock gate's `status === "playing"` arm is tautological by the type
(three arms, two returned above) — kept deliberately and the comment now says it is a
restatement. The mutation-10 cell's human-mover fact was a comment and is now an
assertion. The unlock 409 path has a committed fixture and no client cell. The "You are
watching" strip drops the signed-in address — an owner wording option ("watching as
&lt;email&gt;"). Four smaller polish items.

**Task 9.** `hasMove` is now duplicated a third time. Eight further polish items in the
report. The stream closure's ~25 s heartbeat cost is recorded, not gated.

**The live review's three Minors (added at the release, 2026-09-16; §4.5 has the
evidence).** **M-1** — a watcher (and by the same code a player) on a FINISHED game is not
told of an abandon, because the stream is closed at game over and a per-game 404 renders
inside the panel that asked for it; this is the one status at which the fix wave's "a
sentence on every screen" is not true, and either of two one-line fixes closes it. **M-2**
— every arrival by link or reload draws the lobby for one round trip, opening and aborting
the house stream and throwing away three listings; the fix is an "opening…" line while a
table route is pending. **M-3** — a player whose turn it is holds no stream by design, so
an abandon under them is learned on their next post; a suspicion, not reproduced, and the
same fix as M-1's first option. All three LEAVE: none is on the wire, none holds a
release, and M-1 and M-3 are one small follow-up together.

### 6.3 OWNER ITEMS — decisions that are not an agent's

1. **The email roster reaches any signed-in caller in one page.** Through `?scope=playing`
   (R-3.9) and `GET …/{id}/table` at playing and over (R-3.13), a signed-in caller reads
   every such table's id, free-text name, creator email, **each seat's email** and chosen
   name, lock state, turn, `over`, winner and their own `can_watch`. The spec authorises
   each route; it never names the LISTING property. On the NAS behind Cloudflare Access
   "any signed-in caller" is the owner's own allow-list, so today this is a small circle;
   **on the hosted path (M6.5) it is a decision.** Options: a display-name-only roster in
   the listing (the per-seat `name` M6.2 added exists for exactly this), or leave as is.
   Beside it: **`unlock` is a password oracle with no rate limit**, exactly as `join` has
   been since M6.2.
2. **The whole-game gate's vs-BOT arm LIVELOCKS on some OS-entropy seeds — now
   DIAGNOSED.** Two failures in eight runs during Task 9 (1,690.7 s and 1,932.4 s; the
   other six 10–25 s) and one in the counts sequence (1,813.0 s), hitting
   `play_a_whole_game.rs:189`'s 20,000-input cap at turn 7,295 / 40,125 events with the
   bot holding 9,077 money and 5,453 seafood and **no end trigger advancing**: the arm's
   driving policy places nothing and leans on the bot to end the game. This is the M6.1
   close §6.1 "13 s / 857 s / 16 s" wall. **Candidate fix: the placement probe the
   two-people arm already has.** Not this cycle's code; the owner's call.
3. **The e2e's `CAP = 4000` is being hit**, and a hit reports as a stuck table. Readings
   across this cycle: 3,559 / 3,555 / 2,963 / **4,001 (failed)** / 1,828 / **4,001
   (failed)** / 2,276 / 2,104. A cap hit surfaces through `deadEnd` as "nothing on the
   screen is answerable", which is the message for a genuinely stuck table — so the
   choice is a larger cap, a distinguishing message, or both.
4. **The disk on this machine sits at ~9.4–10 GiB free**, and **a FAILED Playwright run
   writes a multi-GB trace** (`web/test-results/`, 2.2 GB from one run; gitignored). It
   was deleted three times during this cycle, and on both failed runs above the trace hit
   ENOSPC — **the artefact that would explain a failure is what this machine cannot
   write.** Worth knowing before the live review.

---

## 7. What this cycle does NOT claim

Spec §11, restated at this HEAD, with what the build adds.

- **DEPLOYED, and the three bullets that stood here at the build close are now
  answered** (this is the amendment; everything else in §7 is unchanged and was written at
  `9a4626d`). `yokohama-web:0.5.0-m6.3` is live on the NAS in container `f0c371c7…` since
  19:20:51Z, recorded in `deploy/RUNBOOK.md` §3.9.
- **The tunnel reading has been taken, both ways** (§2.1). `SIGNAL_KIND` stays `"sse"`
  because a stream held ten minutes through the edge with a move arriving in it, and
  because the same stream without its 25 s heartbeat was reset at 125 s. The held-request
  arm is still dark and still one constant away, with R-F.1's residual unexercised.
- **The exit criterion is shown, on the NAS.** In the live review of the deployed image a
  watcher arriving by link asked `GET …/events` **zero** times and held **one** stream
  through two full turns, through a 444 s game of 1,471 events and through a whole game
  against the bot (§4.5). What is still NOT claimed about it: nothing was measured with
  more than three browsers, no game between two PEOPLE was played to its end, and the
  hidden-tab word remains untried on the live site.
- **The held arm has shipped DARK and has a residual gap**, written at
  `HELD_REASK_MS`'s own doc: a bump landing inside one request's round trip — after the
  server answered, before the next subscribe — still waits for the bump after it or for
  the 25 s deadline. Closing it needs the server to answer at once when `after` differs,
  which is a route change and not a reader one.
- **The live review is DONE and returned SHIP** (§4.5), with three Minor findings that
  LEAVE. It is three browsers driven by one machine for a few minutes, not a week of real
  play: it says the mechanism works where it is deployed, not that nothing else is wrong.
- **The notifier is designed only** (spec §7): the Durable Object, the poke's
  authentication, the message shape and the drop-and-recover rule are decided; none of it
  is built. M6.5 builds it.
- **No wall figure is a budget.** The two-browser cell's 309 s is recorded beside the
  open's 604 s, never gated; the 0 idle looks is the figure that means something.
- **The `CAP = 4000` is the cell's, not the spec's**, and it is being hit (§6.3).
- **The second arm of the whole-game gate is still NOT a witness to the acting-seat
  fence** — M6.1's own sentence stands, because the walk always posts as the seat whose
  turn it is. The third identity added here watches; it never posts.
- **No spectator count or watcher list is projected to anyone, and no kick.** The
  `unlocked` list is stored, capped at 256, for the table's life; there is no re-lock and
  no expiry.
- **The stream is not a security boundary**: what it carries is what the `events` route
  would answer the same caller.
- **`EventsQuery.after` stays validated-and-inert** (R-3.14); no server-side resume.
- **Every count, line number and route name in the spec is at `48ae908`**; the plan
  re-read them at M6.2's close HEAD and this close re-measured at `9a4626d`.

---

## 8. The handover into M6.4 — chat

**M6.4's entry artifact is `docs/plans/2026-09-16-web-frontend-m6.4-chat-handover.md`**,
written at this HEAD by the orchestrating session: the charter, the three seams chat
stands on (the signal, the store, identity and seats), the rulings that bind it, the
questions the brainstorm must settle, and the process that has worked for three cycles.
Read it after this document's §1, §5, §6 and §7.

**The three steps that were owed to the owner are DONE, all on 2026-09-16**, under the
owner's in-session authorization; nothing about M6.4 waits on them any longer:

1. **Plan Task 3** — folded into the release rather than run as a separate rc, because the
   owner authorized the release and the NAS had nobody playing on it. The R-3.1 reading was
   taken against the deployed stream: **A** held ten minutes with the minute-nine frame
   arriving, **B** (a throwaway `-rc2` with no heartbeat, deployed for the reading alone
   and removed after) was reset by the edge at 125 s. §2.1 has both.
2. **Plan Task 10** — `yokohama-web:0.5.0-m6.3` built at `74a0e80`, live from 19:04:04Z and,
   after the rc2 reading, from **19:20:51Z** in container `f0c371c7…`; verified through the
   tunnel (health carrying `"signal":"sse"`, the bare 401, all three scopes, the bundle) and
   by the milestone's own create/deal/keep/stream check. Recorded in `deploy/RUNBOOK.md`
   §3.9, with the heartbeat sentence added to the Yokohama ingress comment in
   `deploy/cloudflared-config.yml`.
3. **Plan Task 11 step 2** — the three-browser live review: **SHIP**, 0 Critical, 0
   Important, 3 Minor, 25 claims true in both layouts, zero console errors. §4.5.

**And two things still waiting on the owner:** PR #33 is **open** against `web-room-m6.2`
(https://github.com/jmoore2333/yokohama-assistant/pull/33), which is stacked on
`web-room`/PR #31 — **merge order is the owner's**, and nothing here is merged. M6.2's
ruling #13 — the delete control on the lobby's Your-games rows — was kept on the owner's
word during this cycle and needs nothing further.

**This record has now been amended ONCE, at the release**, exactly as promised: the header,
§1's opening, §2.1's slots, the new §4.5 and its three Minors in §6.2, §7's first three
bullets and this section. **It freezes here**, like every other dated artifact; anything
later is recorded forward.
