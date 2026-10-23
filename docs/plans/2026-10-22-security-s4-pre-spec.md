# Security S4-pre — the web lane's leftovers: specification

**Status:** the specification of the precursor wave `security-s4-pre`, cut from master after the security stack lands
and before S4a, as the S4 design orders (`docs/plans/2026-10-22-security-s4-public-release-design.md` §2.22, §5a).
It is the committed, trimmed copy of the gitignored draft `docs/temp/2026-10-22-web-lane-leftovers-issues.md`
(written 2026-10-22 against HEAD `56460688` by a Sonnet agent, every `file:line` checked): its Issues 1 to 4, which this
wave builds; its Part 2, the re-recordings the wave's records task applies; and its findings. Issues 5 to 7 of that
draft are NOT this wave's — they are posted as GitHub issues at the wave's close, in this document's layout, and keep
their text in the draft until then. The plan `docs/plans/2026-10-22-security-s4-pre-plan.md` is written against this
document.

**The owner's rulings that bind it (2026-10-22, the S4 design §2.22–§2.25):**

- Issue 1 keeps the hard per-table total of 64 on both hosts and adds a watchers' figure of 32: a caller with no seat
  is refused by the room's total first, then by the watchers' figure, then by their own 8; a seated caller is refused
  only by the total and by their own 8. Four seats × 8 + 32 cannot exceed 64, so the total never refuses a seated
  caller while the watchers' figure holds. Classification is by the caller's standing when the stream or socket is
  admitted.
- Issue 2 takes shape A: one neutral head, no new stored field.
- The "record and leave" boxes of the draft's Issues 6 and 7 stay recorded; they are not this wave's.
- The wave deploys WITH stage 2 of the auth cycle, as one release `0.7.3` on both hosts ("we should update the version
  number all together anyway so we don't end up with drift"). The wave itself bumps nothing and deploys nothing.
- The two sentences the owner writes — the watchers' refusal and the marker head — have defaults in the plan's
  "Before Task 1", used unless the owner replaces them there.

Line numbers below are the draft's readings at `56460688` and are re-read by the implementer at the branch's own head.

---

## Issue 1

**Title:** web: admit a table's seated players past a full room of watchers, on both hosts

**Labels:** `web`, `worker`, `nas`, `enhancement`, `security-ledger`

### Context

Finding D3 (no cap on streams or sockets per caller, tier D) closed in two halves, the Worker's at `935f80b2` and the
NAS's at `dd583791`, each with a stated residual. The ledger's row ends the NAS half with this clause: "the table's
total judges a seated player as it judges anyone — a full room refuses them until streams close — and neither figure is
a budget a feature may lean on; a seat-aware admission is S4's (the Task 1 review; the scanner's lockout shape)"
(`docs/security-ledger.md:33`). The same sentence is in the NAS module doc (`crates/yokohama-web/src/streams.rs:20-23`)
and twice in the lane document (`docs/web-lane.md:1002` and `:1004`).

The "lockout shape" is the S2 stack review's lens-2 reading, recorded in the S2c plan's Review record: "the lockout
shape (strangers filling a table's or the house's streams ahead of a seated player) measured on both hosts and recorded as
S4's" (`docs/plans/2026-10-13-security-s2c-nas-identity-plan.md:3841-3842`; the ledger text is transcribed at `:3087-3088`).
Its cheapest form was already fixed: S2b's Task 3 review measured that a refused socket stays CLOSING for as long as its
client declines to answer, and that "a stranger's 64 unanswered 403s locked a seated player out"
(`docs/plans/2026-10-12-security-s2b-worker-door-plan.md:3359-3361`), so both caps now count OPEN sockets only
(`worker/src/table/sockets.ts:37-44`, `crates/yokohama-web/src/streams.rs:4-6`). What remains is the lockout by OPEN
sockets held by other callers.

*What a seat is.* In the layer a table document lists its seats as `Human { user, name, bound_at_create }`, `Bot` or
`Open` (`crates/yokohama-web/src/table.rs:46-79`); `user` is the caller's key (a verified address under `cf-access` and
`worker`, a typed name under `open`) and `name` is a label. `TableDoc::seats_of(user)` returns the seats a key holds
(`table.rs:288-296`). A table has two to four seats, so at most four distinct people are seated at once; one person may
hold several (a hot seat). A seat is a reservation by key, not a verified caller: the creator may bind a seat to an
address before that person ever appears (`docs/web-lane.md:150-151`, the table document and "Seats, the deal" bullets).

*Who holds a socket at a table.* A seat holder at every status; any signed-in caller at an `open` table; a spectator at
a `playing` or finished table that has no lock, or after the password; nobody without a seat at `dealing`
(`docs/web-lane.md:151`). So watchers and seated players draw on the same per-table total.

*What the provider-signed door changes.* Under the open door a name is free, so the ledger records that "8 names × 8
sockets fill a table" and "32 × 8 the house's 256" for one person (`docs/security-ledger.md:33`, "Recorded, not
closed"). Under `worker` at source `workos` (the public Worker since stage 1) or `oidc`, and under `cf-access` on the NAS,
the mode rule admits only an email-shaped key (`crates/yokohama-web/src/identity.rs:278-284`), so the per-caller cap of 8
is per verified address and filling a table's 64 takes eight such addresses (seven and part of an eighth if a seated
player holds a socket). That is a real raise in cost over the open door. Two limits on that reading, both unmeasured in the record: the key grammar keeps address
spellings distinct beyond trim, space collapse and ASCII lowercase (`identity.rs:159-181`), so "eight addresses" counts
keys and not people, and what a provider's sign-up lets one mailbox mint is the provider's behaviour and is not read
anywhere in the record. The table must also be open to the crowd: an `open` table, or a `playing`/finished table with no
lock or a known password; a refused password costs the guesser the D2 throttle (20 wrong passwords per table and caller
per 10 s, `crates/yokohama-web/src/guesses.rs`).

*Related, not blocking.* Issue #53 (turn alerts) lists as its option 1 keeping a hidden tab's stream open while it is not
the viewer's turn, "one connection per waiting table". That would raise a seated player's steady socket use; reserving
seated players' room first makes that option cheaper to adopt.

### What happens today

- **Worker.** `capVerdict(own, total, caps)` returns `"total"` when the table object holds 64 OPEN sockets, else `"own"`
  when the caller holds 8, else admits (`worker/src/table/sockets.ts:31-35`). `admit` counts OPEN sockets under the
  caller's tag and in all (`:105-106`), and a socket over a cap is accepted, sent `end` with the cap's sentence and
  closed 4429 (`:108-113`; sentences at `worker/src/frames.ts:91-92`, close code `:88`). `admit` receives the caller and
  the layer's `events` reply but nothing that says the caller holds a seat (`:92`; `worker/src/table.ts:212` and `:418`).
  The sockets carry two tags, the table id and the caller's SHA-256 tag (`sockets.ts:107`).
- **NAS.** `Streams::admit(room, caller)` checks the room total, then the caller's count, and holds no seat information
  (`crates/yokohama-web/src/streams.rs:78-94`). The route calls `api::events_at(...)?` first, then `admit`
  (`crates/yokohama-web/src/lib.rs:1013` and `:1017-1019`); the house stream calls `admit` at `:1092`.
- **The layer already knows.** `events_at` reads the table once and, for a `playing` table, computes the viewer
  (`crates/yokohama-web/src/api.rs:2086-2130`, through `admitted_with` at `:620`, `Viewer::Seat` or `Spectator` at
  `client.rs:342`), then discards it. The single read is pinned by `events_at_reads_the_table_once` (`api.rs:4337`).
- **What a refused seated player meets.** The game screen ends its signal and goes to the lobby with the refusal's
  sentence (`web/src/App.tsx:1452-1472`; the four refusals that end a signal, `web/src/signal.ts:479-503`). Opening the
  table again from the lobby meets the same full room until a slot frees. The `…/events` read still answers (lane §15,
  `docs/web-lane.md:1004`).
- **The lobby** has its own caps, 8 per caller and 1,024 on the Worker (`worker/src/house.ts:26` and `:30`), 8 and 256 on
  the NAS (`streams.rs:37` and `:42`). A lobby has no seats; this issue does not touch it.

### The change

A rule, and its figures, that the owner rules on; the figures are cost bounds a feature may not lean on, as before.
The shape proposed:

1. **Reserve the table's room for its seated players.** Keep the per-table total at 64 on both hosts. Add a second figure
   for callers who hold no seat at the table, `WATCHERS_PER_TABLE = 32`, and a fifth sentence for it in the project's
   register (for example "this table has as many watchers as it can take — try again later"; the wording is the owner's).
   Verdict order, as today's two-step order extended: the room's total first (nothing changes for it), then, for a
   caller with no seat, the watchers' figure, then the caller's own 8. A seated caller is refused only by the total and
   by their own 8. The invariant that makes this safe: at most four seated people times 8 sockets (32) plus at most 32
   watcher sockets cannot exceed 64, so the total can never refuse a seated caller while the watchers' figure holds. If
   the owner prefers a smaller change, the alternative is to exempt a seated caller from the room's total and keep their
   own 8: the room then holds at most 64 plus 32. The first keeps the hard bound of 64; the second needs no watchers' figure and no fifth sentence.
2. **The layer says whether the caller is seated, from the read it already makes.** Return the bit beside `Events` from
   one function (an `events_at` sibling that `events_at` wraps), without a second table read and without a new field on
   the `Events` wire object, which the committed corpus pins byte for byte (`wire_fixtures`).
3. **NAS.** `Streams::admit` takes the bit; `Counts` gains a count of OPEN seated streams per room, released with the
   slot, so watchers are the room's count less the seated count. The maps still hold nothing for a quiet room (B8's
   lesson, `streams.rs:15`). The house passes no seat.
4. **Worker.** The module's reply carries the bit as a host-internal field (the precedent is `bumps`, `archive` and
   `picker` on `ReplyData`, `crates/yokohama-worker/src/lib.rs:220-243`, the `Reply` getters at `:258-300`, and the shell's
   `Replied`/`readReply`, `worker/src/module.ts:18` and `:104-126`). `admit` tags a seated socket with a third tag and
   `capVerdict` gains the watchers' arm, counting OPEN sockets under that tag. `ReplyData {` is spelt 18 times in
   `crates/yokohama-worker/src` (the definition, 17 literals; `grep -c`), so the field is added with the struct literals
   it touches in view (CLAUDE.md's lesson about literals the cap never saw). A cheaper variant: ask the module for the
   seat only when the count would refuse, which costs nothing on the ordinary admission.
5. **Classification is by the caller's standing when the socket is admitted.** A joiner's earlier socket counts as a
   watcher's until it reconnects; say so in the code and the lane document, with a cell.
6. **Not changed:** the per-caller figure 8; OPEN-only counting; the 4429 close and the NAS's 429 shapes; the `…/events`
   read; the lobby's caps.

Both hosts are involved: the Rust under `crates/yokohama-web/` (layer and NAS) and `crates/yokohama-worker/`, and the
TypeScript under `worker/`. The client changes nothing: the new sentence ends the signal like the four it joins.

### Where

- Layer: `crates/yokohama-web/src/api.rs` (`events_at`, `:2086`), beside `events_at_reads_the_table_once` (`:4337`).
- NAS: `crates/yokohama-web/src/streams.rs` (code and its test `the_caps_refuse_the_ninth_of_a_caller_and_the_sixty_fifth_of_a_table_and_a_dropped_slot_frees_the_count`, `:142`); `crates/yokohama-web/src/lib.rs` (`stream`, `:1005`; its route cells `a_callers_ninth_stream_is_a_429_before_any_frame_and_a_dropped_response_frees_it` and `a_cap_does_not_mask_the_admission_reads_refusal`, about `:6240-6300`).
- Worker module: `crates/yokohama-worker/src/events.rs:47` and `lib.rs:220-300`; its cell home is `lib.rs` near `:1418`.
- Worker shell: `worker/src/module.ts`, `worker/src/table/sockets.ts`, `worker/src/frames.ts:88-94`; tests `worker/src/table/sockets.unit.test.ts`, `worker/src/frames.unit.test.ts:101-105`, and the pool's `worker/test/caps.test.ts` (the table cells at `:129` and `:162`).
- Documents: see "Record to update".

### Acceptance

Cells named in the project's style. Every one gets a mutant that turns it red, run on an export.

NAS, `streams.rs` unit cells:
- `a_seated_callers_stream_is_admitted_into_a_room_whose_watchers_sit_at_their_figure`
- `a_watchers_stream_past_the_watchers_figure_is_refused_in_the_watchers_sentence_with_room_to_spare_in_the_total`
- `a_seated_caller_is_still_held_to_the_per_caller_figure_and_to_the_rooms_total`
- `the_verdicts_run_total_then_watchers_then_caller`, a two-defect probe at each adjacent pair (CLAUDE.md: a single-defect probe pins nothing about order)
- `a_dropped_stream_frees_its_seated_slot_and_a_quiet_room_leaves_nothing_in_either_map`

NAS route cells, `lib.rs`:
- `a_seat_holders_stream_is_a_200_into_a_table_whose_watcher_streams_are_full`
- `a_strangers_stream_at_an_open_table_is_counted_as_a_watchers_until_they_reconnect_holding_a_seat`
- the existing `a_cap_does_not_mask_the_admission_reads_refusal` unchanged and green

Layer:
- `events_at_answers_whether_the_caller_holds_a_seat_from_its_one_table_read`, extending the single-read pin, at `open`, `dealing` and `playing`, for a seat holder, a hot-seat holder, a stranger and a spectator

Worker module (`crates/yokohama-worker`):
- `events_inner_reports_a_seat_holder_seated_at_every_status_and_a_stranger_or_a_spectator_not`

Worker shell:
- `sockets.unit.test.ts`: `capVerdict` admits a seated caller at the watchers' figure, refuses a watcher there, keeps the total first, and keeps the seated caller's own 8
- `caps.test.ts` (pool): "admits a seated player's socket into a table whose watcher sockets sit at their figure, and refuses the next watcher with the watchers' sentence and a 4429 close"; "still refuses a seated player's ninth socket with the caller's sentence"; "counts a dangling refusal toward neither pool"
- `frames.unit.test.ts`: the fifth sentence at its literal, as the four are pinned at `:101-105`

Mutants to run: the seat bit ignored; the watchers' arm applied to a seated caller; the per-caller arm skipped for a seated caller; the order of the three verdicts permuted; the seat tag left off `acceptWebSocket`; the seated count read over all sockets rather than OPEN ones.

A live reading follows the deploy (not part of the cells): on the NAS, nine `curl -N` streams as watchers from four callers fill the watchers' figure while a seat holder's stream is a 200 (`deploy/RUNBOOK.md` §3.22's probe recipe is the model); on the Worker, the same with sockets, as `worker/RUNBOOK.md` §22 read the ninth-socket case.

### Size

Two code tasks and a records task, bundled by host: (1) the layer's seat bit and the NAS admission with their cells; (2) the Worker module field, the shell's tag and `capVerdict`, the sentence and its cells; Opus implementer and Opus reviewer for both; (3) the records. Cut finer (layer, NAS, Worker module, Worker shell) it is four code tasks, which is where the S4 design's "four to six tasks" (`…-design.md` §11, (d)) lands. Hosts: both. Rough sessions (an estimate): one SDD session for the code and its reviews, by the S3 precedent (seven code tasks took from 01:03 to 06:27 local on 2026-10-20 by the SDD workspace's file times, with all but one task needing a fix round and a scoped re-review), plus the live reading in whichever deploy window carries it.

### Record to update when it lands

- `docs/security-ledger.md` row D3 (`:33`): the clause "a seat-aware admission is S4's (…)" becomes `fixed <commit>` with the catchers named; the live column gets its reading.
- `crates/yokohama-web/src/streams.rs:13-23`, the module doc ("recorded for S4", "a seat-aware admission is S4's"). It changes the picker's code identity at the next Worker build, because `crates/yokohama-web/src` is one of the three digested roots (`crates/yokohama-worker/build.rs:37`); that costs nothing when the edit rides a release that deploys anyway.
- `docs/web-lane.md` §15 at `:1002` and `:1004`, §4 at `:162` (the Worker's caps), the API map's stream rows at `:818-819`, and `worker/RUNBOOK.md:105-108` (the cap sentences).
- The board's S3 item (g) if the live reading is taken in the owner's own session (`docs/handover.md:140`).

---

## Issue 2

**Title:** worker: reword the marker head for a table closed before it started, so a dealing table does not read "waited for players"

**Labels:** `worker`, `polish`

### Context

Security S3's Task 4 (finding D9) made a `dealing` table with a keep outstanding end on the waiting-table TTL on both
hosts (`9e397966`). On the Worker the ending's marker head is the one cycle E wrote when only an `open` table could end
this way: "this table waited 7 days for players and was closed on <date>" (R-6.5c2.42). A dealing table has all its
players; what it waited for is a keep. The Task 4 review recorded it as Minor 3, "plan-mandated"; the orchestrator
deferred it as the owner's call (R-29, "a head change is a sentence change with its own cells"; SDD ledger
`.superpowers/sdd/2026-10-19-security-s3-plan/progress.md:136` and `:138`, gitignored). The Fable whole-branch review
left it for the owner (`final-review-report.md` §5). The committed record is S3's design §7.4 and §7.5 ("the `waited`
head's 'for players' on a dealing table (the owner's call)") and the board: "the `waited` head's wording — 'this table
waited 7 days for players' now prints for a dealing table that waited on keeps … your call"
(`docs/handover.md:140`, item (d)).

### What happens today

- The head is composed at `worker/src/marker.ts:66-67`: ``this table waited ${days(m.ttl_days)} for players and was closed on ${day}``.
  The reason type is `marker.ts:28`; the alarm stamps `waited` whenever the table is "waiting" and the waiting TTL has
  passed (`expiryReason`, `marker.ts:128-132`), and "waiting" is `status === "open" || status === "dealing"`
  (`worker/src/table/lifecycle.ts:390-397`; the reason is taken at `:357-358`). The reason carries no status.
- Every request to the closed table is a 404 carrying this sentence for 90 days (`marker.ts:45-56`); the client shows the
  server's sentence as it is.
- **The NAS has no marker** (`docs/web-lane.md:898-901`: "the NAS has no marker or operator either way"; the lane's §1
  table, expiry row). Its sweep closes the same tables as `Ending::Waited` and logs the single word `waited`
  (`crates/yokohama-web/src/sweep.rs:132-179`, cells at `:465-476` and `:609-666`). No NAS sentence says "for players". So
  the sentence has one source and the change is the Worker's alone.
- Where the sentence is pinned: `worker/src/marker.unit.test.ts:62-70` (two literals, 7 days and 1 day);
  `worker/test/expiry.test.ts:78` (`HEAD.waited`), used for an `open` table at `:496` and for a table filled on day 6 and
  left `dealing` at `:539`, and through the arranged matrix at `:794-815`.
- The lobby's empty-state sentence "no table is waiting for players" (`web/src/Lobby.tsx:706`) is about open tables and
  stays true.

### The change

The wording is the owner's. Two shapes:

- **A (proposed): one neutral head**, "this table waited 7 days to start and was closed on <date>". True of an open and of
  a dealing table, no data change, `Reason` unchanged. A marker stored before the change re-renders with the new head
  because the head is composed at read time from `reason` and `ttl_days`. This brings the head to the words S3 already
  gave the knob: its sentence moved from "a table still WAITING for players" to "a table that has not started" at every
  site that states it (S3 design §3.2 rule 2).
- **B: two heads by status** ("for players" at `open`, "for every seat to choose" at `dealing`). It needs the status in
  `Reason` and in the stored `meta.marker` JSON (`marker.ts:33-36`), and a rollback then meets a marker it does not know;
  the lane already records that a rollback across cycle E renders a `waited` head as `undefined — …`
  (`docs/web-lane.md:34`; `worker/RUNBOOK.md` §19). Not proposed.

The tail ("— no move had been made, so there was nothing to keep") stays true for both.

### Where

`worker/src/marker.ts:67`; cells `worker/src/marker.unit.test.ts:62-70` and `worker/test/expiry.test.ts:78`, `:496`, `:539`;
documents `worker/README.md:1485` and `:1508`, `worker/RUNBOOK.md:77` and `:439`, `docs/web-lane.md:162`. One host.

### Acceptance

- `marker.unit.test.ts`: "names a table that waited to start, with the marker's own ttl_days", at 7 days and at `1 day`.
- `expiry.test.ts`: an `open` table and a filled `dealing` table both read the new head with the `nothing` tail; the
  `expired` head is unchanged.
- Mutants: the old head restored; a constant 7; "1 days"; the `expired` head under `waited`.
- After the change `git grep "for players" -- worker` reads only the documents that are rewritten in the same commit.

### Size

One task, one host. Sonnet implementer, Opus reviewer. About an hour of agent time (an estimate). The owner's wording
decision comes first.

### Record to update when it lands

The board's S3 item (d) (`docs/handover.md:140`); the five document sites above. S3's design §7.4 and §7.5 are frozen
records and stay as written.

---

## Issue 3

**Title:** nas: stop saying a keyed client build "is the open door's" in the start refusal, its comments and the update script

**Labels:** `nas`, `docs`, `polish`

### Context

The auth cycle's Task 6 changed the client: it no longer reads a Turnstile site key as the open door, it reads the
server's facts (`web/src/outbox.ts:47-59`, rule 6b; `heldForAName`, `:536-539`). The deploy guard's rule was narrowed to
match: "a keyed build is the open door or a provider-signed worker door" (`worker/scripts/deploy-guard.mjs:846-847`), and
`deploy/README.md:136` and `:188` say the client "no longer reads the key as the door". The Rust sentences were fenced
that cycle. The plan's Review record: "M-5 extends C-21 and is recorded forward … the same sentences in RUST —
`crates/yokohama-web/src/assets.rs` ~53, ~90–95 (the refusal the NAS operator reads), ~643 and `main.rs` ~122 — are fenced
this cycle and a later hotfix rewords them" (`docs/plans/2026-10-18-auth-cycle-plan.md:1090-1093`). The board carries it
as "the Rust sentences in `crates/yokohama-web/src/assets.rs` about 'a key is the door'" (`docs/handover.md:122`).

### What happens today

- `crates/yokohama-web/src/assets.rs:53-56`, the doc of `keyed_build_refusal`: "a client built with a Turnstile site key
  IS the open door (R-FP4). It reads the key as a door with no identity provider in front …".
- `assets.rs:90-95`, the refusal the operator reads on stderr: "… — a keyed build is the open door's: the client reads the
  key as a door with no identity provider in front, so here it would write every finished game "not signed in" and send
  none. Rebuild the client without YOKOHAMA_TURNSTILE_SITEKEY, or run it with YOKOHAMA_WEB_IDENTITY=open."
- `assets.rs:646-650`, the cell's twin of that sentence (`keyed_behind`), and its doc at `:655-660`;
  `crates/yokohama-web/src/main.rs:128-132`. `crates/yokohama-web/tests/start_refusal.rs:454-459` builds its expectation
  from `keyed_build_refusal` itself, so it follows.
- **Two sites the board does not name:** `deploy/update.sh:34` ("a keyed build is the open door's, R-FP4") and `:120`, the
  `die` message for a keyed build env, which repeats the phrase.
- The rule is meant to stay and is documented as staying: the server "reads the build's stamp at start and refuses a keyed
  build under `cf-access` or `worker`, naming both" (`deploy/README.md:136`), pinned by
  `a_keyed_build_starts_only_at_the_open_door_and_is_refused_behind_any_other_naming_both` (`assets.rs:655-670`).

### The change

Reword, do not re-decide. The refusal's reason, the two doc comments, the cell's twin and the two script lines say what
`deploy/README.md:136` and `:188` say: the client holds a nameless visitor's finished game for a name only where the
server's facts say `open`; a keyed build belongs to the open door and to the Worker's provider-signed door, and this server
behind `cf-access` or `worker` is neither; the widget is the no-Access Worker's (`deploy/README.md:188`). The implementer reads
`outbox.ts:47-59` before writing the new reason; the true reason is the code's, not this issue's. The sentence is an
operator-facing refusal on stderr: no wire fixture or corpus moves, and no version bump is due (lane §7 bumps for a
release that changes what a save means or which bot plays).

### Where

`crates/yokohama-web/src/assets.rs`, `crates/yokohama-web/src/main.rs`, `deploy/update.sh`; the cells in `assets.rs`
(`:636-730`) and `crates/yokohama-web/tests/start_refusal.rs`. NAS host and the self-host script; the Worker is not
involved. `update.sh` has no test of its own; `--dry-run` is its only read.

### Acceptance

- The existing refusal cell, with its literal moved in the same commit, still reads the sentence the function builds.
- A cell that the sentence no longer contains "the open door's" and does contain the reason the implementer read from
  `outbox.ts`; mutant: the old sentence restored.
- The check the task reports, run once: `git grep -n -i "keyed build is the open door's\|reads the key as a door"` finds
  nothing outside `docs/plans` and `docs/temp`.
- `bash deploy/update.sh --dry-run <tag>` still prints its steps.

### Size

One task. Sonnet implementer, Opus reviewer. About an hour (an estimate). Every edit under `crates/yokohama-web/src`,
comments included, changes the picker's code identity, so the Worker's next build drops open pickers
(`crates/yokohama-worker/build.rs:37`); it costs nothing when the edit rides a release that deploys anyway.

### Record to update when it lands

The board's housekeeping item 4 (`docs/handover.md:122`). The plan's M-5 is a frozen record.

---

## Issue 4

**Title:** worker: make the secrets guard refuse a Worker file with no door or an unknown door

**Labels:** `worker`, `polish`

### Context

The auth cycle's final wave taught `secrets-guard.mjs` to refuse a worker-door file with no or an unknown identity
SOURCE. Its report's second concern: "the secrets guard still passes a file with no/unknown DOOR (item 8 was scoped to the
source; the deploy guard refuses first) → recorded forward"
(`.superpowers/sdd/2026-10-18-auth-cycle-plan/progress.md:225`; the re-review repeats it at `:227`, gitignored). The board:
"the secrets guard's door reading" (`docs/handover.md:122`). The guard's own header gives the reason a pass is a hole: "this
guard run alone must not pass a reading it cannot make" (`worker/scripts/secrets-guard.mjs:15-19`).

### What happens today

- The command judges `{ file, door: doorOf(config.value), source: sourceOf(config.value) }` (`:127`). `requiredSecrets`
  returns `[]` unless `door === "worker"` (`:68-71`) and `sourceRefusal` returns `[]` for any door but `worker`
  (`:75-77`). A file whose `YOKOHAMA_WEB_IDENTITY` is absent, or neither `worker` nor `open`, passes with nothing judged.
- A cell pins that pass-through as intended: `worker/scripts/secrets-guard.test.mjs:90-95` ("requires nothing new under
  access, at the open door, or beside a door the shell does not know"), the "no door" row at `:94`.
- The shell's own words for the refusal exist: "YOKOHAMA_WEB_IDENTITY must be worker or open — it is not set, and there is
  no default: a door is a deployment decision" and "… — got <value>" (`worker/src/config.ts:168-169`).
- Nothing deploys through this today: every deploy script runs the deploy guard first, and it refuses such a file
  (`deploy-guard.mjs:478-481` reads the door). The hole is the guard run alone.

### The change

Add a `doorRefusal(judged)` beside `sourceRefusal` (`:73-82`), in the shell's sentence with the file named, for a door
that is absent or not one of `worker` and `open`. Order of the sentences: forbidden names, the door, the source, the
missing secrets. Replace the row at `:94` with a refusal, and add the unknown-door row. The open-door rows keep passing
(the open door requires nothing, and a stray source beside `open` is the deploy guard's refusal, not this guard's).

### Where

`worker/scripts/secrets-guard.mjs` (and its header, `:15-19`) and `worker/scripts/secrets-guard.test.mjs` (`:85-100`, the
command cells at `:196-209`). Worker tooling only.

### Acceptance

- "refuses a Worker file with no door, or one the shell does not know, quoting the shell's sentence with the file named"
  (`judgeSecrets`, two rows).
- "run alone, the command refuses a file with no door or an unknown one, and passes the open door" (over the fake `npx`,
  as the source cell does at `:198`).
- Mutants, in the shape of the source refusal's three at `:97-100`: the door unjudged; only an absent door refused; only
  an unknown one refused; the door sentence placed after the missing-secret sentences.
- Run alone, never beside a cargo build: the auth plan's Review record found the guard's command cells red beside a
  concurrent build and green alone.

### Size

One task. Sonnet implementer, Opus reviewer. About an hour (an estimate).

### Record to update when it lands

The board's housekeeping item 4; the guard's header comment. `worker/RUNBOOK.md` §2 describes the secrets guard
(`:262-275`) and needs a word only if it lists what the guard refuses.

---


---

# Part 2 — Not issues

## 6. Re-recordings the auth cycle made due

The public Worker's door is provider-signed since the auth cycle: a caller is a verified address, the public deployment is
`yokohama-web` itself re-pointed from `access` to `workos` by one variable (plan decision D-1, the owner's R-A12), and the
mode rule admits only an email-shaped key under `cf-access` and `worker` (`crates/yokohama-web/src/identity.rs:278-284`).
That supersedes two deferrals written for an open-door public Worker. The S4 design reads them the same way (§11 (f) and
(g)) and already places the re-recording in S4c's ledger-and-record task (§8.2: "C1's, D3's and §8's 'S4's' sentences
re-recorded … each row saying so"), so these drafts can be dropped into that task as they stand. Each draft below quotes what it replaces. The open door that self-hosters run on a home network keeps the seat
property by design, and each draft says so. Every edit to a live document is a separate, small docs commit; none is made
here.

### (a) `docs/security-ledger.md:27`, row C1, the clause "a layer rule for seats is S4's (decision D-2)"

*Replaces:* "Recorded, not closed: a seat is a reservation, not a caller — the layer does not judge a seat's user by the
door, and only the client's new-table form refuses an `@` there; a layer rule for seats is S4's (decision D-2)".

*Draft:* "Recorded, by design: a seat is a reservation, not a caller — the layer does not judge a seat's user by the door,
and only the client's new-table form refuses an `@` there. Where the door is provider-signed (the public Worker at source
`workos`, a self-hoster's at `oidc`, and Access on the NAS) the mode rule admits only a verified address as a caller
(`identity::admit`), so a seat is acted in only by the caller whose address is its key and no layer rule is needed. Under
a self-hoster's open door a seat's key is a typed name and anyone who types it may act in that seat, as for any name there
(a name is a label and a key, never a credential, S2b decision D-2); that is the open door's property, stated in the
lane's §15 and, once S4a writes it, in the web guide. No layer rule for seats is planned (settled 2026-10-22)."

Context read: the plan the clause comes from says "A seat is a reservation, not a caller: the layer does not judge a
seat's user by the door … a layer rule for seats belongs to S4, with the public open door"
(`docs/plans/2026-10-12-security-s2b-worker-door-plan.md:177-179`). The premise "with the public open door" is what the
auth cycle removed.

### (b) `docs/web-lane.md` §8, the sentence ending "S4's controls (the owner's R-3) its answer" (`:417-420`)

*Replaces:* "for an open-door Worker, a ceiling per person: the Worker's own two limits (security S2b, finding C2) count
creates by the connecting IP and writes by the IP and the name, and a name is free, so the write limit and the archive's
share are per NAME, never per IP — the zone's rate-limit rule and the edge rule stay the deployer's tools for that, and
S4's controls (the owner's R-3) its answer;".

*Draft:* "for a self-hoster's open-door Worker, a ceiling per person is not built: the Worker's own two limits (security
S2b, finding C2) count creates by the connecting IP and writes by the IP and the name, and a name is free, so there the
write limit and the archive's share are per NAME, never per IP, and the zone's rate-limit rule and the edge rule are the
deployer's tools for that (§15). The public Worker needs no further control for it: its door is provider-signed (source
`workos`), the write limit counts the verified address's digest, the anonymous archive is bounded by the per-key share and
the create limiter by IP, and a caller is a verified address held to 8 sockets (§15); the ceiling the owner's R-3 asked
for is the door's (settled 2026-10-22);".

The rest of that "Still owed" list (`:414-423`), read against the paragraph below it (`:425-447`), has more that is no
longer true; the list should be rewritten as one, not patched by clause. See "Found while writing", items 7 and 8.

### (c) `docs/security-ledger.md:33`, row D3, the clause "a seat-aware admission is S4's"

*Replaces:* "… and neither figure is a budget a feature may lean on; a seat-aware admission is S4's (the Task 1 review; the
scanner's lockout shape)."

*Draft if the issues are posted (Issue 1's number is N):* "… and neither figure is a budget a feature may lean on; a
seat-aware admission is issue #N (the Task 1 review; the scanner's lockout shape). On the provider-signed door the
lockout takes eight verified addresses on one table; on a self-hoster's open door it takes eight free names."

*Draft if the wave lands it:* "… a seat-aware admission is fixed `<commit>`: a watchers' figure of 32 beside the table's 64
on both hosts, a seated caller refused only by the total and by their own 8, counted at admission; catchers: …". The
row's "Recorded, not closed" tail stays true for watchers (one person with several names can still fill the watchers'
figure) and is rewritten to say that a seated player is no longer locked out by it.

### (d) `crates/yokohama-web/src/streams.rs:17-23`, the module doc

*Replaces:* "… where a name rotation escapes the per-caller figure and meets the room's total alone (the Worker's sibling
is `docs/web-lane.md` §15's socket-cap row, S2b's finding D3, and this module's own is the NAS stream-cap row beside it;
recorded for S4). The room's total is the bound a process relies on, and it judges a seated player as it judges anyone: …
Neither figure is a budget a feature may lean on; a seat-aware admission is S4's."

*Draft:* "… where a name rotation escapes the per-caller figure and meets the room's total alone; that is the open door's
property, recorded in the ledger's D3 and the lane's §15 (the Worker's sibling is §15's socket-cap row, S2b's finding D3,
and this module's own is the NAS stream-cap row beside it). The room's total is the bound a process relies on, and it
judges a seated player as it judges anyone: … Neither figure is a budget a feature may lean on; a seat-aware admission is
issue #N." (Or, if the wave lands: "… a seated caller is counted apart (`seated`) and refused only by the total and by
their own figure.") This is a Rust comment, so it changes the picker's code identity at the next Worker build; let it
ride a release that deploys anyway.

### Other sites that carry the same deferral, found while drafting

The brief names four; the record has more. Each gets the same replacement in its own words.

- `docs/web-lane.md:1002` (§15): "… a seat-aware admission is S4's, with the public open door. Under `worker` a caller is
  a verified address, held to 8." *Draft:* "… a seat-aware admission is issue #N (or fixed `<commit>`). The public Worker's
  door is provider-signed, so there a caller is a verified address held to 8 and filling a table's 64 takes eight such
  addresses; the open door's one-person fill is a self-hoster's property."
- `docs/web-lane.md:1004` (§15), the NAS's stream caps: ends "… until streams close; a seat-aware admission is S4's."
  *Draft:* the same two-word change, "is issue #N".
- `docs/web-lane.md:998` (§15): "… a layer rule for seats is S4's. **The owner's shape for S4, recorded and not decided
  here** (R-3): the public Worker at `yokohama.moore.nyc` runs the open door behind the Turnstile, with controls to be
  minted later; a second private Worker and the NAS stay behind Access; a self-hoster deploys as they like."
  *Draft:* "No layer rule for seats is planned: where the door is provider-signed a seat is acted in only by its address,
  and under a self-hoster's open door the seat property is the door's own. **The shape R-3 recorded for S4 did not stand:**
  the public Worker at `yokohama.moore.nyc` runs the provider-signed door (source `workos`, the auth cycle), not the open
  door; the private Worker (`yokohama-test.moore.nyc`) and the NAS stay behind Access; a self-hoster deploys as they
  like." This sentence is the furthest from the present shape: it says the public Worker runs the open door.
- `docs/handover.md:128`, the S2b paragraph: "D-3's open-door write key per (IP, claim) (a claim rotation defeats the
  per-name limit; the IP-only create limit is the bound; S4's)" is the same deferral as (b). It is a state narrative;
  the board's rule is to rewrite stale lines, so "S4's" becomes "superseded by the provider-signed door, 2026-10-22".
- `worker/RUNBOOK.md:273`: "S4's public Worker's first-deploy path is this recipe, and no guard code in S2b:". The public
  deployment is `yokohama-web` itself, so there is no new public Worker to deploy for the first time; the recipe was the
  private Worker's first deploy (S3 Task 7, `worker/RUNBOOK.md` §26). *Draft:* "This recipe was the private Worker's first
  deploy (§26); the deploy guard holds its record in `DEPLOYMENTS` since the auth cycle's Task 7:". The paragraph also
  says "with neither guard" while the guard does judge the new file's record, which the same sentence says; it wants
  one careful read.

## 7. D32 — resolved, nothing to build

*The ledger's row* (`docs/security-ledger.md:62`), in its status column: "accepted (R-S3-4, 2026-10-19: the transition's
cost — the door applies the release; the population is the Access allow-list until stage 2); the edge rule's `/api/me`
leg LIVE 2026-10-20 (the zone's one rate-limit rule, ruleset v5; read 21 × 401 then 429 `retry-after: 10`), the two
testers asked to open the front page once". The Worker runbook says the same and adds why the hole closes only that way:
"The hole is CLOSED only by each 0.6.3 browser opening the front page once (the door applies the release); the owner asks
the two testers to" (`worker/RUNBOOK.md:2551-2552`).

*The closing clause* (appended to the status column, after "once"): "; **closed 2026-10-22:** the owner confirmed that the
two testers have opened the front page, so the door has applied the release in the browsers known to hold the 0.6.3 shell;
the edge rule's `/api/me` leg stays as the bound for any other stale client".

*Lines that still ask the question and should be rewritten with it:* the board's rulings item 10 (`docs/handover.md:153-154`,
still "a per-IP bound on caller-less reads in the Worker (S3), or accepted as the transition's cost") and S3 item (c)
(`:140`, "the two testers open the front page once (D32, unchanged)"). The runbook's sentence is a dated record and stays.

## 8. Passkeys and the AuthKit custom domain — deferred, accepted undone (the owner, 2026-10-22)

*Sources:* the board's rulings item 11 (`docs/handover.md:156-160`) and the Worker runbook's stage-1 passkeys bullet
(`worker/RUNBOOK.md:2558-2574`).

*The one-line record:* "Passkeys are OFF in production (both WorkOS flags read back false, 2026-10-19) and were dismissed
by R-S3-6 (2026-10-20); on 2026-10-22 the owner accepted them undone: no AuthKit custom domain and no enrolment path of
our own are planned, and passkeys return only with both, as a cycle of their own."

*What it replaces:* the board's item 11 still reads as an open ruling ending "a cycle of its own, S4 or later", although the
S3 design's R-S3-6 says "the board's ruling 11 closes; the AUTH row's note reads `dismissed` with that reason; no enrolment
path is built" (`docs/plans/2026-10-19-security-s3-design.md:55-56`). The ledger's AUTH row already carries "passkeys
`dismissed` — R-S3-6, no custom WorkOS domain" (`docs/security-ledger.md:63`); the record adds "(accepted undone, the
owner, 2026-10-22)". No issue.

## 9. The owner's own items, not code

Listed for completeness, with their board lines.

- **Providers and the staging key** (Track 4 checklist item 2, `docs/handover.md:120`): Google, GitHub and Apple (Discord if
  wanted) need the owner's OAuth credentials in the WorkOS production dashboard before their flags go on; the staging API
  key, pasted into a session transcript on 2026-10-18, is to be retired.
- **Security S3's items** (`docs/handover.md:140`): (a) a LAN DNS record for `yokohama-test.moore.nyc`, since the LAN resolver
  is authoritative for `moore.nyc` and answers the new name with nothing; (b) the off-WARP reading at the test host, which
  meets `bdvc.cloudflareaccess.com`'s login; (c) the two testers' front-page visit, now confirmed (item 7 above);
  (g) the owner's own browser reads: a creator's reopen of a real joiner's seat, D2's 429 on production (its door is
  WorkOS, so the window's curl could not sign in), the NAS's next hourly sweep line, and the update reading on a pinned
  browser. Items (d) and (e) are covered elsewhere here: (d) is Issue 2; (e), the stack's merge, is PR #56 (open) and then
  `security-auth` to master, which the S4 design settles as "agreed" (§2.9).
- **Stage 1's unread rows**, "a dealt table and the socket under the cookie" (Track 4, `docs/handover.md:115`, and the
  checklist's item 1 at `:119`: "deal a table → the WebSocket opens under the cookie → sign out"). The S4 design folds them
  into the window's readings (§8.5).
- **One more, from Item 5:** whether the D10 sync runs, and what it carries (the board's rulings item 8, `:150`).

---

---

## Found while writing

Each item says what the brief or a document claims, and the evidence that it does not hold.

1. **Item 5 is not a web-lane decision.** The brief reads D10 as a web-lane decision about the browser-played pin's sidecar.
   D10 is the 3.5c scoping amendment's "Machines, going forward" (`docs/plans/2026-08-11-phase3.5c-scoping-amendment.md:128-130`):
   a standing NAS sync of gitignored captures, `docs/temp` and memories. The web lane has no such D10 (its only hits are
   cycle D's R-D10 and the security ledger's D10, both unrelated and both done). If the pin's sidecar was meant, it is
   tracked and a cell reads it (`crates/yokohama-web/tests/browser_played_pin.rs:121-130`). Whether the sync runs is not in
   the record; two Phase B handovers say "NAS-synced per D10" and the 2026-09-11 handover says "no row records it done".
   It matters for S4: the lab remote carries no gitignored file, and the two captures behind the FOURTEEN corpus pins, the
   six owner-game saves and the memories are all gitignored. See "Item 5" after Issue 7.
2. **Item 2's sentence has one host.** The brief expects pinning cells on both hosts. The `waited` head is composed in
   `worker/src/marker.ts:67` and nowhere else; the NAS has no marker (`docs/web-lane.md:898-901`) and its sweep logs only the
   word `waited` (`crates/yokohama-web/src/sweep.rs:609-666`). The change is the Worker's alone.
3. **Passkeys were already ruled.** The S3 design's R-S3-6 (2026-10-20) dismissed them and says "the board's ruling 11
   closes" (`docs/plans/2026-10-19-security-s3-design.md:55-56`); the ledger's AUTH row says `dismissed`
   (`docs/security-ledger.md:63`). The board still lists item 11 as an open ruling ending "S4 or later"
   (`docs/handover.md:156-160`). The 2026-10-22 acceptance completes a close that was never applied to the board.
4. **D32 is accepted in the ledger and still asked on the board.** The ledger row says accepted and the edge rule's leg live
   (`docs/security-ledger.md:62`); the board's rulings item 10 (`docs/handover.md:153-154`) and S3 item (c) (`:140`) still
   pose the question or the errand.
5. **The board names four of the auth cycle's recorded-forward items; the record's list is eight.** The plan says "seven
   recorded forward" (`docs/plans/2026-10-18-auth-cycle-plan.md:1178-1179`). The seven are the bullets of the final-wave
   brief's "Recorded, NOT this wave" list (gitignored): the sign-in block and the three encoders (one bullet), the discovery
   endpoint's raw text, R-42, Task 8's m-1 to m-3, the notice's placement, the Rust sentences, and `live.ts`'s cookie-only
   branch. An eighth, the secrets guard's door reading, came from the wave's own report (`.superpowers/sdd/2026-10-18-auth-cycle-plan/progress.md:225`). The board's
   housekeeping item (`docs/handover.md:122`) carries the Rust sentences, the guard, the block and the encoders, and nothing
   else. The discovery endpoint's text and Task 8's three minors are written down only in the gitignored brief.
6. **The Rust sentences are in `deploy/update.sh` too** (`:34`, `:120`), which no board line or plan note names, and the
   test literal at `crates/yokohama-web/src/assets.rs:646-650` has to move with the refusal.
7. **The "S4's" deferral is at nine sites, not four.** The brief's four plus `docs/web-lane.md:998` (which says the owner's
   shape for S4 is the open door behind Turnstile, the furthest from the present shape), `:1002`, `:1004`, `docs/handover.md:128`
   and `worker/RUNBOOK.md:273` ("S4's public Worker's first-deploy path"; there is no new public Worker). Part 2 drafts each.
8. **The rest of lane §8's "Still owed" paragraph is stale in three more places.** (i) "on a public Worker that starts empty"
   (`docs/web-lane.md:420-423`) is retracted by the paragraph under it (`:437-439`: "no longer the shape"). (ii) "the art's
   decision; a fresh-history repository" (`:414-416`, `:445-446`) is answered by the 2026-10-22 direction (history kept whole,
   the art credited); the S4 design already assigns that to S4c (`…-design.md` §8.2). (iii) The auth paragraph's header says
   "the window is pending, so none is READ live yet" (`:425`) while §1 records stage 1 live since 2026-10-19 (`:34`).
9. **"Eight signed-in addresses" counts keys, not people.** The S4 design's phrase (`…-design.md` §11 (d)) holds as a count
   of keys. The layer's key grammar merges no address variants beyond trim, space collapse and ASCII lowercase
   (`crates/yokohama-web/src/identity.rs:159-181`), and what one mailbox can sign up as is the provider's behaviour, which
   the record does not read.
10. **The S4 design's version note assumes the NAS does not move.** §8.3 leaves the NAS at `0.7.2` "(nothing it runs changed)".
    Issue 1's NAS half changes the binary; if Issue 1 ships with stage 2, the NAS deploys at `0.7.3` too.
11. **Posting needs labels that do not exist.** Only GitHub's nine defaults exist (`gh label list`); `enhancement` is the one
    of the suggested set that does. Issue #53 sets the title style used here.
12. **Issue #53 touches Issue 1.** Its option 1 (a hidden tab keeps its stream while it is not the viewer's turn) would raise
    seated players' steady socket use. The S4 design's non-goal "no new web features" (§3) leaves #53 open past the switch.
13. **Two parked Minors of the S2 stack review are in nobody's list.** The lobby's cap sentence without a retry control, and a
    429 at a stream's open ending the signal (`docs/handover.md:130`, `:134`): the board handed them to the auth cycle as "UX
    inputs", and the code still ends the lobby's signal with no retry (`web/src/Lobby.tsx:411-417`). They are a box in
    Issue 7.
14. **A rollback caution for Issue 2, option B only:** a head that needs a new field in the stored marker meets the rollback
    hazard the lane already records for cycle E (`docs/web-lane.md:34`). Option A avoids it.
