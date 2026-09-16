# Web frontend — M6.4 "chat": the entry handover

**Written 2026-09-16 on branch `web-room-m6.3` at `9a4626d`, at the close of M6.3's BUILD, by the
orchestrating session, for the session that opens M6.4.** M6.4 is the fourth cycle of the room plan
(`docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §2) and the LAST that the current local
design — the Synology, the cloudflared tunnel, Cloudflare Access, a file store on a named volume —
needs to be fully usable: two people play, a third watches, and they can talk. M6.5 (the hosting
move) is a change of configuration at the three seams and is not part of "usable".

**The next session's job is not to build. It is to brainstorm with the owner, write the spec, and
write the plan** — the room plan's R-P1 ("each cycle its own brainstorm → spec → plan"), in that
order, with execution under R-P11 only once the plan is handed over. This document is the entry
artifact: what the charter says, what M6.3 leaves that chat stands on, the rulings that bind it, the
questions the brainstorm must settle, and the process that has worked for three cycles.

---

## 0. Read order for the session that opens M6.4

1. `CLAUDE.md` — the Web (server) and Web (browser half) command bullets and the `crates/yokohama-web`
   + `web/` architecture bullet, in particular its **IDENTITY, TABLES AND SEATS**, **THE LOBBY** and
   **TRANSPORT AND SPECTATING** paragraphs.
2. `docs/handover.md` Track 3 — the owner's board, including the three M6.3 steps that were still
   OWED when this was written (the rc deploy and tunnel measurement, the release, the live review).
   **If they are still owed, they come before M6.4's brainstorm**: the exit of M6.3 is not shown until
   the stream runs on the NAS.
3. `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0 (R-P1–R-P11 — owner direction, not up for
   re-litigation inside a cycle), §1 (the three seams), §2's M6.4 paragraph (the charter).
4. `docs/plans/2026-09-16-web-frontend-m6.3-close.md` — M6.3's record of record: §1 what shipped, §5
   the nineteen execution rulings, §6 the LEAVE list and the owner items, §7 what it does NOT claim.
5. `docs/plans/2026-09-15-web-frontend-m6.3-transport-and-spectating-spec.md` — §2 (the channels), §3
   (the routes and the admission table), §4 (the spectator projection), §5.1 (the hook), §7 (the
   notifier design chat's delivery must fit), §11 (what it does not claim).
6. `docs/plans/2026-09-15-web-frontend-m6.2-lobby-spec.md` §3.4–§3.5 and §4 — the table document,
   the per-table locks, the password.
7. This document.

Every "at HEAD" fact below is at `9a4626d`. Re-read each cited file at the HEAD you open; three
cycles running, the first task of every plan has been to re-read the previous spec's line citations.

---

## 1. The charter (room plan §2, verbatim in intent)

**Builds:** per-table messages as their own store document (never in the `GameRecord`), delivered
over the same signal; a message strip in the side column; the lobby's chat if the lobby wants one.

**Exit:** two players talk during a game and a third reads it as a spectator.

The exit names three people. M6.3 built the third — a spectator admitted at a playing or finished
table — so the exit is reachable on the seams that exist, and the two-browser e2e already drives
three Chromium contexts (`web/e2e/two_browsers.spec.ts`): the cell that shows the exit is an
extension of it, not a new harness.

---

## 2. What M6.3 leaves that chat stands on

### 2.1 The signal — the seam chat's delivery rides

- `crates/yokohama-web/src/signal.rs`: `Signals` holds one `tokio::sync::watch` channel per table
  id and one for the house. `bump_table(id)` wakes the table's subscribers; `bump(id)` wakes the
  house too; `end(id)` sends `Bump { end: true }` and REMOVES the sender so every stream on the
  table closes. A `watch` channel coalesces: three bumps between two reads are one wake. **A bump
  is sent AFTER the write it announces lands, never before** — every write site has a cell, and two
  (the flip and the mirror) park the real write behind a gate and read the receiver quiet. A chat
  write is a new site and owes the same cell.
- `GET /api/games/{id}/stream` (`lib.rs`, the `stream` handler) writes the current `Events` object
  on connect and one `event: signal` frame per bump; `event: end` `{"reason":"deleted"}` on delete;
  a `:` comment heartbeat every 25 s; no `id:` or `retry:` line (R-3.14). `Events` is four fields —
  `events`, `to_move`, `status`, `your_turn` — built by `events_of`; the frame IS the `events`
  route's object (one function, `events_at`). **Chat's first design question lives here**: does the
  frame gain a fifth field (a message count the client compares, exactly as it compares `events`
  today — R-P9's shape), or does chat get its own event kind on the same stream? The notifier design
  (M6.3 spec §7) pokes the Worker with "the same `Events` JSON the stream writes", so whatever the
  frame carries is what M6.5's hosted path carries too.
- `GET …/{id}/events?wait=N` is the held-request fallback, dark behind `pub const SIGNAL_KIND: &str
  = "sse"` (`lib.rs`) until R-3.1's tunnel reading says otherwise. A design that only works on SSE
  frames does not work on the fallback; a count on the `Events` object works on both.
- The client: `web/src/signal.ts`'s `usePolled(source, active, onValue, onEnd?)` reads the stream
  through `fetch` (`web/src/sse.ts`; `EventSource` cannot carry the identity header) with
  `Source<T> { key, once, stream, frames?: "value" | "wake" }` — under `"value"` the frame's `data`
  is the value handed to `onValue` (the game and the dealing screen, `T = Events`); under `"wake"`
  a frame triggers `once()` and ITS result is the value (the open table, the lobby). A chat strip
  is a natural `"wake"` consumer of the game's frame, or a reader of the count on it.
- **The rule M6.3 learned the hard way** (ruling R-9.1, the close §1): a consumer's `active` must
  never toggle on the consumer's own handler — the game's `active` read `busy`, which the arrival
  handler set around its own frame fetch, and every arriving move tore the stream down and reopened
  it (2,643 reopens in one game; the e2e's "exactly one `/stream`" count found it). A chat strip
  that opens its own stream, or flips the game's `active` while it posts, repeats the defect.

### 2.2 The store — where messages live

- `crates/yokohama-web/src/store.rs`: `Store` takes a typed `Doc` (`Record` | `Table`) on every
  call, with `write_if_version` and a typed `Conflict`/`Io`; the file store keeps one file per kind
  per game in ONE directory, which is why `is_safe_id` REFUSES any id ending in `.table`
  (case-insensitively). **A third document kind needs a second reservation the derivation assert
  would not notice** — `CLAUDE.md` names M6.4's chat as exactly where that bites. The chat document
  is the charter's "own store document, never in the `GameRecord`"; its name, its reservation and
  its schema constant are the spec's first section.
- The per-table lock: `Games::lock_for(id)` serializes `join`, `keep`, `settle`, `finalize`,
  `flip_to_playing`, `delete` and `unlock`; `persist` has its own. A chat append is a
  read-modify-write under the table's lock, CAS'd on the version it read, and `delete` must remove
  the chat document with the table (today `delete` tombstones BEFORE the unlink — M6.2 ruling #6 —
  and `Signals::end` closes the streams; the chat document joins that sequence).
- `check_shape` (`tables.rs`) runs under a lock where a panic poisons the mutex and takes the
  process out: **every field read out of a stored chat document is an INPUT** and is bounded there
  (message length, count, author).

### 2.3 Identity, seats and the spectator — who is talking

- `identity::User` is the email, parsed once, lowercased. M6.2 added a per-seat display `name`
  beside it (never a replacement — `user` stays the key, R-2.3). A message's author is a
  `SeatDoc::Human { user, name }` or an admitted watcher; a bot seat has no email and never talks.
- `admitted(state, id, user) -> (TableDoc, Viewer)` (`lib.rs`) is the admission every read route
  starts from; `Viewer::Seat(PlayerId)` or `Viewer::Spectator`. A spectator today reads everything
  and WRITES NOTHING (R-3.6: every write route refuses before the handle is touched). **Whether a
  spectator may post a message is the brainstorm's** — the exit says the third person READS it.
- `TableDoc.unlocked: Vec<String>` (≤ 256) is who has presented a locked table's password; it is
  never serialized (a by-key cell) and the wire corpus's value pin searches every payload built for
  someone else for a watcher's email. A chat payload carrying authors' emails to other readers is a
  new exposure the spec must rule on — the display `name` exists for this, and the owner item on
  roster exposure (below) is the same question at listing scale.

### 2.4 The client's surfaces

- The game screen: above 1201 px the M5 table with a side column whose top is the pinned answer
  bar (`web/src/App.tsx`, `TableSections.tsx`, `Decision.tsx`); below it the stacked layout. The
  charter's "message strip in the side column" goes under the answer bar and above or beside the
  turn account (`TurnAccount.tsx`, which since M6.3 carries `data-event` on each row — a product
  attribute added for the e2e). The spectator screen is the same screen with `state.seat === null`
  ("You are watching", no answer bar, no Undo/Suggest).
- The lobby (`Lobby.tsx`): Your games, Open tables, Games in progress; ONE `usePolled` over the
  house stream as a `"wake"` source. "The lobby's chat if the lobby wants one" is the charter's
  optional clause; the lean recorded below is no.
- The help panel (`Help.tsx`, with `Help.test.tsx`'s emphasis-boundary rule) describes every
  screen; a chat strip is a paragraph there.

### 2.5 The instruments

- `crates/yokohama-web/tests/wire_fixtures.rs` freezes every payload the client reads (71 files
  under `web/src/wire/` at `9a4626d`); `web/src/wire/contract.ts` type-checks them; the value pins
  search every payload for the seed, the password and its hash, and a watcher's email. A chat
  payload joins the corpus and the pins.
- `tests/play_a_whole_game.rs` (per-edit locally, out of CI) plays a 2P game vs the bot and a 2P
  game between two identities with a third that watches; `web/e2e/two_browsers.spec.ts` plays it
  in three Chromium contexts and asserts the watcher issues zero `/events` requests on ONE `/stream`.
  Chat's exit cell is the third browser reading a message the first two typed.
- The mutation battery (spec §9.5, 15/15 caught at `9a4626d`) is the shape a chat spec's §9.5
  follows: name the mutations before the cells are written.

---

## 3. The rulings that bind M6.4

- **Room plan §0, R-P1–R-P11** — owner direction. Of these, chat leans on R-P3 (async play over
  hours or days: a message strip must read well after a night away, and the resume is the count),
  R-P8 (a hidden tab asks for nothing), R-P9 (the signal is separate from the fetch — a message is
  fetched, not pushed whole, unless the spec argues otherwise and the notifier design agrees),
  R-P11 (the process).
- **M6.3's R-3.3** (the stream is per TABLE — a chat message is a table event), **R-3.4** (the house
  stream carries a sequence only — the lobby refetches lists, so a lobby chat would refetch too),
  **R-3.5/R-3.6/R-3.7** (who may watch, what a spectator gets, and that a spectator's payload has no
  hand — a chat strip on the spectator's screen changes none of that), **R-3.14** (no `id:` line; the
  count is the cursor).
- **M6.3's execution rulings that touch chat** (the close §5): R-pre-3 (`unlock` bumps nothing —
  a chat append DOES, because a strip is showing it), R-7.1 (`frames`), R-9.1 (a consumer's `active`
  never toggles on its own handler), R-F.1 (the held arm re-asks at once only for a holding `once`).
- **M6.2's** R-2.3 (the display name beside the email), R-2.9 (the per-table lock), ruling #6
  (delete tombstones before the unlink), ruling #13 (the lobby's delete, owner-confirmed 2026-09-16).
- **What stays fixed** (room plan §3): the five fenced crates byte-identical, `SEMANTICS_REV` 7,
  the server computes nothing about the game, the corpus is a contract, no bot behaviour moves.

---

## 4. What M6.3 hands forward (its close §6, in one screen)

- **Owner items surfaced in M6.3, unresolved:** (1) `?scope=playing` plus `table` at playing/over
  hand every playing table's EMAIL roster to any signed-in caller in one page — spec-authorised
  route by route, never named as a listing property; chat's author field is the same decision;
  (2) the whole-game gate's vs-BOT arm LIVELOCKS on roughly one seed in four (turn 7,295, the bot
  holding 9,077 money, no end trigger; the arm's driving policy places nothing) — the M6.1 "13 s /
  857 s" wall, now diagnosed; candidate fix: the placement probe the two-people arm has; (3) the
  e2e's `CAP = 4000` loop iterations is being HIT on long OS-entropy games and reports as a stuck
  table; (4) the workstation's disk sits at ~10 GiB free and a failed Playwright run writes a 2.2 GB
  trace that hits ENOSPC.
- **LEAVE items chat will meet:** the double table read in `events_at` (once via `table_for_anyone`,
  once inside `admitted`) on the stream's per-bump closure; blocking store I/O inside the stream's
  `poll_next` (the crate's handler pattern, multiplied by watchers) — a chat document read per bump
  adds to it; `usePolled`'s name (nothing it does is a poll); `link` starting `"live"` at mount.
- **The held arm's residual** (written at `HELD_REASK_MS`): a bump landing inside one request's
  round trip waits for the next bump or the 25 s deadline. A chat message on the fallback arm has
  the same latency.

---

## 5. Questions the M6.4 brainstorm must settle (with the orchestrator's leans)

1. **The document.** One chat document per table (`<id>.chat.json` under a second `is_safe_id`
   reservation), a schema constant, an append-only list of `{ seq, at, author, text }`, capped
   (lean: a few hundred; older messages dropped from the document, never from a record), removed
   with the table. Lean: yes to all; the cap and the field bounds are `check_shape`'s.
2. **The delivery.** Lean: `Events` gains `messages: u64` (the count), the stream frame carries it
   by construction, the client compares and fetches `GET …/{id}/chat?after=N` — R-P9 kept, both
   transport arms served, the notifier's poke unchanged in shape, the corpus contract unbroken
   (`events*.json` gain one key). The alternative — an `event: chat` frame carrying the message —
   pushes content through the signal seam and does not survive the held arm.
3. **Who writes.** Lean: seat holders and admitted watchers both, each message labelled by seat
   colour or "watching"; a bot seat never; the write route admits like a read (`admitted`) but is a
   WRITE for the fences' purposes (a spectator who may not `input` may `chat`).
4. **Who is named.** Lean: the display `name` (M6.2) on the wire, never the email — and the value
   pin extended to search chat payloads for every seat's and watcher's email. This is the roster
   question again; settle both at once with the owner.
5. **The strip.** Lean: in the side column under the answer bar above 1201 px, in the stack below
   the account below it; the newest at the bottom, a one-line input, Enter to send, no history
   scrollback beyond the document's cap; the M3.5 width and modality rules; the help paragraph.
6. **The lobby's chat.** Lean: no — "if the lobby wants one" and it does not yet; a house-wide
   channel is a moderation question the local design need not answer.
7. **Retention and undo.** Lean: messages are never undone and never in the record; an abandoned
   table's chat goes with the table.
8. **Rate and size.** Lean: bounded at `check_shape` (text ≤ 500 chars, ≤ 500 messages) and by the
   per-table lock; no rate limit on the NAS.

---

## 6. The process that has worked (R-P11, refined in execution)

- **Brainstorm** with the owner (the superpowers brainstorming skill): one question at a time,
  the leans above as the starting positions, the answers written into the spec's §1 as R-4.x.
- **Spec** in the M6.3 spec's shape: §0 what is true at HEAD with line citations; §1 the rulings;
  the document, the routes with their admission table, the projection, the client, the fences, the
  wire, the tests and the MUTATIONS THAT MUST BE CAUGHT, byte-identical, deployment, what it does
  NOT claim, the read order. Then an adversarial read of the spec against HEAD before the plan (the
  M6.3 spec took twenty-six corrections that way).
- **Plan** in the M6.3 plan's shape: zero-context tasks each with files, interfaces, the code where
  it is known, the cells, the gate command and the `git add` line; plan-level rulings; a
  self-review section. Execution under subagent-driven development: Opus implementers and per-task
  reviewers, Fable orchestrating and running the whole-branch review, one fix wave, one scoped
  re-review, the mutation battery, the counts in one sequence, THEN the deploy with the owner, the
  live review from three browsers, the close.
- **Two rules the last three cycles wrote in blood:** every agent writes its full report to a
  transcript file and replies with the path (the notification channel truncates); never send an
  implementer a follow-up without reading `git log -1` and `git status` first (messages cross, and
  a reviewed SHA is never rewritten — a correction lands on top).

---

## 7. Where the branches stand at this handover

- `web-room` → master is PR #31 (M6.1); `web-room-m6.2` → `web-room` is PR #32 (M6.2);
  **`web-room-m6.3` is stacked on `web-room-m6.2` and its PR (#33) is opened once the owner's
  deploy steps land** — merge order is the owner's. M6.4 branches from `web-room-m6.3`'s close
  HEAD as `web-room-m6.4`, the same shape.
- The SDD workspace for M6.3 (`.superpowers/sdd/2026-09-15-web-frontend-m6.3-plan/`, gitignored)
  holds every brief, report, review and the ledger until the close is final; the M6.2 close is the
  precedent for what survives in the tracked record.
