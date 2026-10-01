# Web frontend — M6 "the room": the plan across its five cycles

**Written 2026-09-14 on branch `web-room` at `2533001` (master's tip that day), from the
brainstorm held the same day between the owner and the orchestrating session.** M6's entry
artifact (`docs/plans/2026-09-06-web-frontend-m6-handover.md`) carried a charter and three
open questions with leans; this document turns the charter into an ordered plan and settles
the questions that had to be settled before the first cycle could be specified. It is the
PLAN for the whole of M6 and the authority for the ORDER and the SEAMS; each cycle enters
through its own spec, and the first cycle's spec is
`docs/plans/2026-09-14-web-frontend-m6.1-identity-and-seats-spec.md`, written beside this.

The room is human-vs-human play in a browser — two people at least, three or four when
wanted — with the front door, lobby, live updates and chat that need. It is built and
tested on what runs today (the Synology, the cloudflared tunnel, Cloudflare Access, a file
store on a named volume) and designed so that the move to a hosted shape is a change of
configuration at three seams rather than a rewrite.

---

## 0. What the owner ruled on 2026-09-14 (the settled inputs)

These are owner direction. A later cycle may ask to revisit one; none is re-litigated
inside a cycle.

- **R-P1 — the order.** Identity and the front door, then seats bound to identities with
  per-seat views, then the lobby, then live transport and chat, then the hosting move.
  Sequential; each cycle its own brainstorm → spec → plan.
- **R-P2 — identity is pluggable and, for now, loose.** The server reads ONE configured
  identity source. Cloudflare Access is one source; a typed email the client claims is
  another, allowed in these early versions because it makes testing, getting started and
  portability easy; "dropping in the identifier" from Access, Neon Auth or another provider
  later must be straightforward, and the claimed source must be easy to disable downstream.
  The design is for what is in use today; a public distribution of this software must not
  be bound to Cloudflare.
- **R-P3 — cadence.** Designed for LIVE play first (everyone at a browser at once), with
  asynchronous play over hours or days as a consequence of the same design: an event
  stream with a resume cursor serves both, the lobby shows whose turn it is, and turn
  notifications stay out of scope until asked for.
- **R-P4 — the architecture.** Identity at the origin, seat bindings in a document beside
  the record, views scoped in the server (approach 1 of three in the brainstorm; the two
  rejected are recorded in the M6.1 spec §0.2).
- **R-P5 — the store.** Files on the named volume now. A database is never put INSIDE the
  container: the point of moving to Postgres is to take state OUT of the container so
  containers become disposable and plural, and that is an external database (Neon when
  hosted; a separate Postgres container if ever wanted on the NAS). The store seam widens
  in M6.1 so that swap is configuration.
- **R-P6 — undo.** Human vs bot keeps today's undo. At a table with more than one person,
  undo is possible only while the caller's own turn has not fully completed.
- **R-P7 — legacy games.** Records on the NAS from before M6 are review material for the
  AI track (pulled as CLI saves); the API need not serve them.
- **R-P8 — no polling from a hidden tab.** A tab that is not visible asks for nothing.
  When it comes back the client shows a refreshing state, asks once, and resumes; a
  container spinning up reads as a short wait, not an error.
- **R-P9 — the signal is separate from the fetch.** The client consumes one small message
  ("this game now has N events, seat S to move") and fetches frames only when it moved.
  Its transport is the third pluggable seam, beside identity and the store. M6.1 ships the
  polled form of it; M6.3 chooses among a held request, server-sent events and a Durable
  Object notifier, designing toward the last.
- **R-P10 — a bot seat at two players only.** The evaluator's own doc: "Two-player only.
  Every corpus, fit, scenario and screen in the trail is 2P. Nothing has measured what
  these weights do at 3–4P" (`crates/yokohama-core/src/game/eval.rs:1044`). The room
  refuses a bot at three and four players with that sentence; relaxing it is the owner's.
- **R-P11 — process.** As the M6 handover §5 wrote it: Opus implementers and per-task
  reviewers, the orchestrating session running one clean final review against the LIVE
  site in both layouts; the SDD workspace convention; explicit `git add` paths; the five
  fenced crates byte-identical and `SEMANTICS_REV` at 7 through every cycle; no bot
  behaviour rides this lane.

---

## 1. The three seams

Everything portable about the room lives at three interfaces, each with one implementation
built for today and a named later implementation the hosted shape wants. A cycle that
adds behaviour on one side of a seam must not reach across it.

| seam | today (NAS) | hosted later | where it is decided |
|---|---|---|---|
| **Identity** — who a request is from | `claimed` (typed email, behind Access at the door); `cf-access` (the tunnel's header) | `worker` (the Worker's forwarded identity, already stripped from clients); JWT modes (`cf-access-jwt`, `neon`) as arms of the same extractor | `YOKOHAMA_WEB_IDENTITY`, one variable, no default |
| **Store** — where records, tables and later chat live | files on the named volume, one document per kind per game, conditional writes by content digest | Postgres behind the same trait, conditional writes by version column, "my games" and open tables as queries | a connection string; the file store stays the default and the memory store the test double |
| **Signal** — how a browser learns something changed | `GET …/events?after=N`, polled at 3 s from a visible tab, never from a hidden one | a held request or SSE from an in-process broadcast; on the hosted path a Durable Object holding hibernating WebSockets that the origin pokes on every persisted move | M6.3's spec; the client's consumer is one function either way |

**Why the signal seam matters for the hosted shape, in numbers.** A three-player game of
about ninety minutes with three open tabs costs, through a Worker: ~5,400 requests if the
full frame is polled at 3 s; ~2,000 polling a tiny `events` endpoint with backoff; ~650
with a 25-second held request; 3 with SSE; 3 connections with a Durable Object notifier.
The Workers free tier is on the order of 100k requests a day, so none of these is the
problem for a few people. The problem is the container: polling, held requests and SSE
all keep a live request at the origin, so a container never scales to zero while a tab is
open. Only the notifier lets the container idle between moves, which is what asynchronous
play over days needs. Cloudflare Containers front each container with a Durable Object
already, so on that path the notifier and the container's front door are the same object.

---

## 2. The five cycles

Each cycle: brainstorm → spec → implementation plan → execution under R-P11 → deploy →
final review against the live site → close record → handover. Exit criteria are what the
close must show, not what the spec promises.

### M6.1 — identity, tables, per-seat views, the events poll

**Spec:** `docs/plans/2026-09-14-web-frontend-m6.1-identity-and-seats-spec.md`.
**Builds:** the identity extractor with three modes; the store trait widened to typed
documents with conditional writes; the table document and the per-seat deal (create →
dealing → each seat keeps from its own view → finalize); every game route resolving the
caller to a seat and building the payload for that seat with the existing `SeatFacts`
masking; the seat fence on `input`; the undo rule; the `events` endpoint and the client's
visible-tab poll; sign-in, a per-seat new-game form, the dealing screen, the opponent's
turn walking from a poll.
**Exit:** two people play a two-player game from two browsers on the NAS as two claimed
identities to a winner; every fence in the spec's §5 holds and its mutations are recorded;
the five crates byte-identical; deployed and verified live in both layouts and from two
browsers.

### M6.2 — the lobby

**Builds:** open tables and join links; taking a seat; start when full; a display name
beside the email; "my games" with whose turn it is (the R-P3 requirement); abandon by the
creator. The front door proper: the first screen a person sees is the lobby, not a form.
**Inherits:** M6.1's table document (gains `open` before `dealing`), the identity seam
(join links are the first place a person arrives without being named in advance), the
bot-at-2P rule. Also inherits from the M6 handover's LEAVE list: the unbounded saved-game
list (becomes the lobby's paging) and the "New game" in-flight guard.
**Exit:** someone holding only a link joins a table and plays.

### M6.3 — transport and spectating

**Builds:** the second implementation of the signal seam at the origin — a held
`events?after=N` or SSE over an in-process broadcast that `persist` publishes to, with
the hidden-tab rule kept; the Durable Object notifier designed (built in M6.5); the
masked spectator view (`Viewer::Spectator`, every hand a count, no options); the M5.5
walk as the spectator primitive, fed from the signal. Decides SSE vs held request on the
tunnel (Cloudflare closes idle sockets at 100 s; the tunnel config notes ttyd's 30 s
pings for the same reason).
**Exit:** no polling on the NAS; a spectator link shows a game walking.

### M6.4 — chat

**Builds:** per-table messages as their own store document (never in the `GameRecord`),
delivered over the same signal; a message strip in the side column; the lobby's chat if
the lobby wants one.
**Exit:** two players talk during a game and a third reads it as a spectator.

### M6.5 — the hosting move

**Builds:** the Postgres store behind the trait (a `games` table with the record as
`jsonb` plus a version column; `tables` with the seat bindings as columns; `chat_messages`
as rows), selected by a connection string; the `worker` identity mode live behind the
existing Worker (`deploy/worker/src/index.ts`), with JWT modes if the door changes; the
container on Modal (`deploy/modal_app.py`, with `max_containers=1` and the commit loop
removed once the store is external) or on Cloudflare Containers; the Durable Object
notifier deployed; scale-to-zero verified. The NAS stays the development environment.
**Exit:** a whole game played through the hosted path by two people.

---

## 3. What stays fixed through every cycle

- `crates/yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge` and
  `gen-data` byte-identical; `SEMANTICS_REV` 7. The CLI is touched only where a seam the
  web already consumes needs a `pub` (M6.1 has one), and its nine `assert_capture` goldens
  plus the 18 `guide_sync` cells are how that is known.
- The server ROUTES, SCOPES and SERIALIZES and computes nothing about the game; every
  model is `screen_model`'s, every word `describe`'s, every move goes through `Session`'s
  legal-list gate. Seat scoping is a projection over the existing masks, not a new rule.
- The wire corpus is a contract: regenerate, run without the variable, read the diff,
  report it. No hand-authored payload in a test.
- No bot behaviour moves. A bot seat in the room is the shipped production contestant
  exactly as the CLI seats it; the AI lane owns anything else (the M6 handover's
  "web lane's V5 notes" stay parked there).

---

## 4. What this plan does NOT claim

- No cycle after M6.1 is specified. §2's "Builds" lines are the charter each cycle's
  brainstorm starts from, and its exit line is what its close must show.
- No sizing of any cycle, and no date.
- Nothing about the hosted shape is verified: the Worker and the Modal app are written and
  undeployed (`deploy/RUNBOOK.md` §4), and the request arithmetic in §1 is an estimate
  from the poll cadence, not a measurement.
- The identity sources are portable by construction of the seam, not by demonstration: the
  third and later modes are unwritten until a deployment needs one.
