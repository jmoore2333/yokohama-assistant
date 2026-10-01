# Web frontend — M6.5 "the hosting move": the entry handover

**Written 2026-09-19 on branch `web-room-m6.4` at `b16ccdd`, at the close of M6.4, by the
orchestrating session, for the session that opens M6.5.** M6.5 is the FIFTH and last cycle
of the room plan (`docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §2). Everything
before it was built on what runs today — the Synology, the cloudflared tunnel, Cloudflare
Access, a file store on a named volume — and the room is fully usable there: two people
play, a third watches, and they can talk. **M6.5 is the cycle where that stops being one
machine's.**

**The next session's job is not to build. It is to brainstorm with the owner, write the
spec, and write the plan** — R-P1's "each cycle its own brainstorm → spec → plan", in that
order, with execution under R-P11 only once the plan is handed over. This document is the
entry artifact: the charter, what M6.4 leaves that hosting stands on, the rulings that
bind it, what M6.4 hands forward, the questions the brainstorm must settle, the process
that has worked for four cycles, and where the branches stand.

---

## 0. Read order for the session that opens M6.5

1. `CLAUDE.md` — the Web (server) and Web (browser half) command bullets and the
   `crates/yokohama-web` + `web/` architecture bullet, in particular its **IDENTITY, TABLES
   AND SEATS**, **THE LOBBY**, **TRANSPORT AND SPECTATING** and **CHAT** paragraphs.
2. `docs/handover.md` Track 3 — the owner's board: what is live, what PRs are open, and the
   owner items.
3. `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0 (R-P1–R-P11 — owner direction,
   not up for re-litigation inside a cycle), §1 (the three seams and the request arithmetic
   that motivates the notifier), §2's M6.5 paragraph (the charter).
4. `docs/plans/2026-09-19-web-frontend-m6.4-close.md` — M6.4's record of record: §1 what
   shipped, §5 the rulings, §6 the LEAVE list and the seven owner items, §7 what it does NOT
   claim.
5. The three closes below it, for the seams M6.5 configures rather than rewrites:
   `docs/plans/2026-09-16-web-frontend-m6.3-close.md` (the signal, the spectator, the
   notifier design), `docs/plans/2026-09-16-web-frontend-m6.2-close.md` (the table document,
   the per-table locks, the password) and
   `docs/plans/2026-09-15-web-frontend-m6.1-close.md` (identity, the store trait, the
   conditional write).
6. `deploy/RUNBOOK.md` — §0–§3 what the NAS deployment IS, §3.5–§3.11 the eight deployments
   with their numbers, **§4 Modal (written, measured, deliberately NOT deployed)** and §5
   rollback. `deploy/worker/src/index.ts` and `deploy/modal_app.py` are the two pieces of
   the hosted shape that already exist on disk and have never run.
7. This document.

Every "at HEAD" fact below is at `b16ccdd`. **Re-read each cited file at the HEAD you
open**; four cycles running, the first task of every plan has been to re-read the previous
spec's line citations, and each time something had moved.

---

## 1. The charter (room plan §2, verbatim in intent)

**Builds:** the Postgres store behind the trait — a `games` table with the record as
`jsonb` plus a version column, `tables` with the seat bindings as columns, `chat_messages`
as rows — selected by a connection string; the `worker` identity mode live behind the
existing Worker (`deploy/worker/src/index.ts`), with JWT modes if the door changes; the
container on Modal (`deploy/modal_app.py`, with `max_containers=1` and the commit loop
removed once the store is external) or on Cloudflare Containers; the Durable Object
notifier deployed; scale-to-zero verified. **The NAS stays the development environment.**

**Exit:** a whole game played through the hosted path by two people.

Three of the charter's five pieces exist as written-and-never-run code
(`deploy/worker/src/index.ts`, `deploy/modal_app.py`, the notifier's design in the M6.3
spec §7); the fourth, the Postgres store, is a second implementation of a trait that
already has two; the fifth, scale-to-zero, is the only one that is a MEASUREMENT rather
than a build, and it is the one the room plan §1 says the whole seam exists for — polling,
held requests and SSE all keep a live request at the origin, so a container never idles
while a tab is open, and only the notifier lets it.

---

## 2. What M6.4 leaves that hosting stands on

### 2.1 The store trait, and its THREE document kinds

`crates/yokohama-web/src/store.rs`. `pub trait Store: Send + Sync` (`:117`) is four
methods — `list(doc)`, `read(doc, id)`, `write_if_version(doc, id, body, expected)` and
`delete(id)` — over `Doc` (`:36`), which now has **four variants across three kinds of
file**: `Record` (`<id>.json`), `Table` (`<id>.table.json`), `Chat` (`<id>.chat.json`) and
`House`, which shares `Chat`'s suffix and is set apart by the underscore `FsStore::path`
prepends: `_house.chat.json` (`:220-226`).

Three things a Postgres implementation must keep, and each is a fence today:

- **The id allowlist and its TWO reservations.** `is_safe_id` (`:167`) minus
  `RESERVED_ID_SUFFIXES = [".table", ".chat"]` (`:157`), compared case-insensitively —
  APFS and NTFS fold case. **A FOURTH document kind needs a THIRD reservation**, and the
  derivation assert (`:779-789`) is what notices: it compares the two LENGTHS before it
  zips, so a suffix added without a matching `Doc` fails there. That assert was blind to
  this exact class before M6.4 and was fixed in the same task that bit it.
- **`check_house_id`** (`:200`), called from `FsStore::path` and from BOTH `MemStore`
  entry points (`:415`, `:434`), because `MemStore` keys `(Doc, GameId)` and asks only
  `is_safe_id` — without it the two implementations disagree about what exists. **A third
  implementation has to answer the same question**, and the cell that pins it iterates every
  store.
- **The conditional write is by CONTENT DIGEST** (`version_of`, `:84`, which digests the
  JSON alone and ignores the id). `FsStore::write` is atomic per file (temp + rename) and
  the store has no lock. Postgres's version column changes what a `Version` IS, and two
  places compare one: the store's own compare-and-swap, and `Games::persist`, which
  compare-and-swaps against a digest of the exact JSON this process last wrote or read.

### 2.2 The signal — `Signals`, and the EIGHT bump sites the notifier replaces

`crates/yokohama-web/src/signal.rs` holds `Bump { seq, end }` per table, `House { seq, chat }`
(`:42`) for the house, and a `Mutex<HashMap<GameId, watch::Sender<Bump>>>` plus one house
sender (`:50`). **It is one process's memory.** `Games` owns it because every write site
does, and every call sits on a success path — a failed or refused write bumps nothing.

| site | file:line | call |
|---|---|---|
| `persist` | `games.rs:543` | `bump` (after `mirror_turn`, so a woken lobby reads the refreshed mirror) |
| `create` | `tables.rs:769` | `bump` |
| `chat_append` | `tables.rs:951` | `bump_table` (M6.4) |
| `house_append` | `tables.rs:965` | `bump_house_chat` (M6.4) |
| `flip_to_playing` | `tables.rs:1255` | `bump` — and this is `finalize`'s bump too |
| `join` | `tables.rs:1402` | `bump` |
| `keep` | `tables.rs:1504` | `bump_table` |
| `delete` | `tables.rs:1693` | `end` (the terminal bump; the sender is removed) |

**This table is the notifier's specification.** Each row becomes a poke; the frame the poke
carries is `events_at`'s, five scalars, which the M6.3 spec §7 already chose — and M6.4
amends its house poke from `{"seq": N}` to `{"seq": N, "chat": M}` (R-6.4.7). **Four gated
cells prove a bump follows its write and never precedes it** (the two M6.3 sites and the two
M6.4 ones); a hosted notifier has to keep that ordering, because a poke that overtakes its
write is a client reading the state before the state exists.

`GET …/{id}/stream` and `GET /api/stream` sit in front of it (`lib.rs`), with `SIGNAL_KIND`
a `pub const` reading `"sse"` (`lib.rs:240`) and **`HEARTBEAT` 25 s** (`lib.rs:248`) — a
number that is load-bearing against a measurement, not a guess: M6.3 held a stream ten
minutes with it and watched an idle one reset by the edge at 125 s without it. The held-
request arm ships DARK behind that constant, with `HELD_REASK_MS = 3000` (`web/src/signal.ts:72`)
and a residual written at it.

### 2.3 Every piece of PER-PROCESS state, enumerated

This is the list a second replica breaks, read off `Games` (`games.rs:110-186`) and
`AppState` (`lib.rs:255-271`) at this HEAD:

| what | where | what a second replica does to it |
|---|---|---|
| `live: Mutex<HashMap<GameId, Arc<Mutex<Repl>>>>` | `games.rs:110` | a CACHE, bounded at 64; a miss is a replay, so plurality costs work and loses nothing |
| `written: Mutex<HashMap<GameId, Version>>` | `games.rs:118` | the CAS base this process last wrote or read; a second writer's write is DETECTED, not prevented — the digest compare is what catches it |
| `deleted: Mutex<HashSet<GameId>>` | `games.rs:152` | a request holding a handle cannot write back after a local `DELETE`; another replica's delete is invisible |
| `table_lock: Mutex<()>` | `games.rs:133` | serializes `create` within ONE process; two replicas can draw the same id |
| `locks: Mutex<HashMap<GameId, Arc<Mutex<()>>>>` | `games.rs:149` | the per-table lock M6.2 split out; per process, so two replicas can interleave a join and a keep |
| `signals: Signals` | `games.rs:163` | §2.2 — the thing the notifier replaces |
| `chat_last: Mutex<HashMap<GameId, u64>>` | `games.rs:176` | the count a frame carries; **a second process's appends are invisible until the count is a query.** The DOCUMENT is always the authority and every page is served from it |
| `house_lock: Mutex<()>` | `games.rs:184` | serializes house appends; two replicas both read `last` and both write `last + 1`, and the CAS saves one |
| `limits: ChatLimits` / `AppState.chat` | `games.rs:186`, `lib.rs:270` | read once at start from the launch variables; every replica must be started with the same values or they disagree about the rule |
| `AppState.identity` | `lib.rs:263` | read once at start, no default |
| `AppState.heartbeat` | `lib.rs:269` | a field only so a test can build 50 ms |
| `TableDoc.unlocked` | `table.rs:211` | **NOT per-process** — the watcher roster is stored, capped at 256, never serialized. It survives a replica |

**The honest summary for the brainstorm:** everything above is either a cache whose miss is
correct (`live`), a conflict DETECTOR that still detects (`written`), or a mutual-exclusion
assumption that a database's own transaction has to take over (`table_lock`, `locks`,
`house_lock`, `deleted`). `chat_last` is the one that is neither: it is a per-process
ANSWER served on the wire, and it is wrong the moment there are two processes.

### 2.4 Identity, and what `worker` needs at the door

`crates/yokohama-web/src/identity.rs`. `IdentityMode` (`:32`) is `Claimed` | `CfAccess` |
`Worker`, read from `YOKOHAMA_WEB_IDENTITY` (`:55`) with **NO default — an unset variable
exits 1**. Each mode reads **exactly ONE header and no other** (`:49-52`), so a forged
header for another mode changes nothing; `User::parse` trims and lowercases once, so a seat
is the same seat on the next request.

**The one thing `worker` already does that hosting depends on**: the literal `anonymous`
counts as ABSENT (`:132`), which is what makes an unprotected Worker fail CLOSED rather than
seating everyone as one person. The Worker in `deploy/worker/src/index.ts` is written and
has never run; the M6.5 question is what the door in front of it is (Access as today, a JWT
mode, or a provider) and whether `claimed` is retired at that point or kept as the local
development door it is documented to be.

### 2.5 The client's transport

`web/src/sse.ts` is a line parser over what this server writes and knows nothing about the
signal; `readStream` opens with `identityHeaders()` — **`EventSource` is not used, because
it takes a URL and `withCredentials` and cannot carry this server's identity header**, which
is a constraint any hosted door inherits. `usePolled(source, active, onValue, onEnd?)`
(`web/src/signal.ts`) arms **no timer at all** while connected, opens one connection per
VISIBLE tab (R-P8), and has four consumers: the game screen and the dealing screen under
`frames: "value"`, the open table and the lobby under `"wake"`. The game screen's `active`
is `!!state` since M6.4 (`App.tsx:823`) — held at every status behind the posting guard
(`App.tsx:596`).

A Durable Object notifier replaces what is behind `readStream`, not `usePolled`'s signature;
the consumer is one function either way, which is the room plan §1's own claim and is now
true of two transports rather than one.

### 2.6 The instruments M6.5 inherits

- **The whole-game gate's two arms** (`crates/yokohama-web/tests/play_a_whole_game.rs`): the
  vs-bot walk, and two claimed identities playing to a winner, each asserted to see only
  their own hand, now talking before the first move. **Stays out of CI** (M6.1 owner ruling),
  per-edit locally, and its vs-bot arm livelocks on some seeds (close §6.3 item 1).
- **`wire_fixtures`** — 82 committed payloads, the contract both sides are checked against:
  the server refuses when its output is not what is committed, `web/src/wire/contract.ts`
  type-checks the same files against the hand-written `types.ts`, and the value pins search
  the WHOLE corpus for the setup seed, the password and its salt and hash, and now a
  watcher's address. **Regenerate and READ THE DIFF**; M6.1 is the cycle that proved it cuts
  both ways.
- **The three-browser e2e** (`web/e2e/two_browsers.spec.ts`): two people play a whole game
  in two Chromium contexts while a third watches, and every claim about connections is
  counted off the pages' own request logs. It is the only instrument in this lane that can
  see the transport actually driving a second screen.
- **The live-review scaffold is MORTAL.** The M6.3 and M6.4 review scripts, request logs and
  photographs live in the session scratchpad and the gitignored SDD workspace; nothing of
  them is tracked. What IS worth carrying is the door recipe: **WARP must be connected, and
  after `warp-cli connect` a `warp-cli debug access-reauth` may be needed** before a fresh
  headless or headed Playwright context reaches the site on its own origin with zero
  redirects. WARP was found DISCONNECTED at the M6.4 wrap-up (close §6.3 item 7).

---

## 3. The rulings that bind M6.5

- **Room plan §0, R-P1–R-P11** — owner direction. Hosting leans hardest on **R-P5** (a
  database is NEVER put inside the container: the point of Postgres is to take state OUT so
  containers become disposable and plural — an EXTERNAL database, Neon when hosted), **R-P2**
  (identity is pluggable and a public distribution must not be bound to Cloudflare), **R-P9**
  (the signal is the third seam and the notifier is what it was designed toward), **R-P3**
  (async play over days is what scale-to-zero is FOR) and **R-P8** (a hidden tab asks for
  nothing).
- **Room plan §3, what stays fixed**: the five fenced crates byte-identical, `SEMANTICS_REV`
  7, the server computes nothing about the game, the corpus is a contract, no bot behaviour
  moves. Four cycles have held all five.
- **M6.4's rulings that constrain hosting:** **R-6.4.7** (delivery is a COUNT on the signal,
  never the text — so the notifier's poke stays a small object and the house poke is
  `{seq, chat}`); **R-6.4.8** (retention is a LAUNCH VARIABLE, so every replica must be
  started with the same three values, and a document longer than the keep is trimmed on its
  next append rather than refused on read); **R-6.4.9** (the file names and the seq rule —
  message numbers start at 1, are never reused, and `last` keeps counting across a trim, which
  is what a `chat_messages` table has to reproduce); **R-6.4.14** (no delete, no moderation,
  no rate limit — a decision made for one operator's NAS that a public door re-opens).
- **M6.3's R-3.14** (no `id:` line; the count is the cursor) and the M6.3 spec §7 notifier
  design, as amended by R-6.4.7.
- **M6.2's** R-2.9 (the per-table lock) and ruling #6 (delete tombstones before the unlink) —
  both are process-local exclusions a transaction replaces.

---

## 4. What M6.4 hands forward (its close §6, in one screen)

- **Owner items, seven, unresolved:** (1) the whole-game gate's vs-BOT arm livelocks on some
  OS-entropy seeds — diagnosed, did not fire once this cycle; (2) the e2e's `CAP = 4000`
  reports a hit as a stuck table — not hit this cycle, readings 1,758–3,366; (3) the
  workstation's disk and a failed Playwright run's multi-GB trace — 46 GiB free after the
  counts, no trace written; (4) **the house chat has NO delete, so its four lines stay until
  an operator edits `_house.chat.json` on the volume**; (5) a house message costs every
  visible lobby three listing reads; (6) **the per-process `chat_last` and in-process
  `Signals` are what a multi-process host breaks** — §2.3 is the enumeration; (7) WARP does
  not stay on by itself.
- **LEAVE items hosting will meet:** the double table read in `events_at`; **blocking store
  I/O inside the stream's `poll_next`**, once per bump per connection with no `spawn_blocking`
  — the crate's existing handler pattern multiplied by open connections, and a database
  driver changes what that costs; the two wake hosts' extra delta read at mount; the tick
  hosts' one delta GET per own post; `usePolled`'s name.
- **What M6.4 did NOT carry, and why it is here instead:** the M6.3 owner item 1 (the email
  roster). R-6.4.2 settled it "leave as is" for the NAS and widened it — a message serves its
  author's email on the wire — so the decision that remains is the HOSTED one. It is §5's
  question 4.

---

## 5. Questions the M6.5 brainstorm must settle (with the orchestrator's leans)

1. **The schema, and what a `Version` becomes.** Lean: `games(id, body jsonb, version)`,
   `tables(id, …seat bindings as columns…, version)`, `chat_messages(table_id, seq, created_at,
   author_kind, author_user, author_name, text)` with a primary key on `(table_id, seq)` —
   which gives R-6.4.9's "never reused" for free and makes the count a `MAX(seq)` query
   rather than a cache. **The open question is whether `Version` stays a content digest or
   becomes a version column**, because `Games::persist` compares one and its whole argument
   (a shortened record carries the digest of the shortened state, which is what makes undo
   work) is written in terms of digests. Lean: keep the digest as the `Version` VALUE even
   in Postgres, so nothing above the trait changes, and let the column hold it.
2. **The notifier's shape.** Lean: one Durable Object per TABLE, poked by the origin at each
   of §2.2's eight sites with the frame `events_at` already builds, plus one for the house.
   The open questions are the poke's authentication, what happens to a poke the DO misses
   (the M6.3 spec §7 chose drop-and-recover: the client re-asks on reconnect and the count
   is cumulative, so nothing is lost), and whether a hibernating WebSocket or SSE from the DO
   is what the browser holds — `EventSource` still cannot carry an identity header (§2.5).
3. **Modal or Cloudflare Containers.** Lean: whichever gives scale-to-zero with the DO in
   front, and the deciding fact is that Cloudflare Containers front each container with a
   Durable Object ALREADY, so on that path the notifier and the front door are one object.
   `deploy/modal_app.py` exists and is measured; both are undeployed, so this is a
   measurement the cycle must take rather than a preference.
4. **The email roster, and rate limits** (M6.3 owner item 1, carried). Through
   `?scope=playing`, `GET …/{id}/table` at playing and over, and now every chat message's
   author, a signed-in caller reads other people's email addresses. On the NAS behind Access
   "any signed-in caller" is the owner's own allow-list; **on a public door it is a
   decision.** Options: a display-name-only projection in the listing and on messages (the
   per-seat `name` M6.2 added and the per-message `name` M6.4 added exist for exactly this),
   or leave as is. Beside it: **`unlock` and `join` are password oracles with no rate limit**,
   and a public door is where that starts to matter.
5. **House-chat retention and moderation on a public door.** R-6.4.14 ruled out delete,
   moderation and rate limits for one operator's NAS. The house chat is the one surface a
   stranger can write to without holding a seat, and it has no delete at all. Lean: at
   minimum an operator-side delete before the door is public; the `chat_messages` table makes
   it one statement.
6. **How the per-process caches become shared or vanish** (§2.3). Lean: `chat_last` becomes a
   query and the cache goes; `table_lock`, `locks` and `house_lock` become row locks or
   `INSERT … ON CONFLICT`; `deleted` and `written` fall out of the conditional write;
   `live` stays a per-replica cache because a miss is a replay. **The one that needs a real
   decision is `create`'s id draw**, which today is serialized by a process-global mutex.
7. **How `claimed` is retired, or kept.** It is documented as a development door and not a
   security boundary, and the client's own prose says so. Lean: keep it for local
   development, refuse it when a hosted-mode variable is set, and make `worker` the hosted
   door — but that is R-P2 territory and the owner's.
8. **What the exit MEASUREMENT is.** The charter says "a whole game played through the hosted
   path by two people" and "scale-to-zero verified". Lean: the exit needs a number, not a
   demonstration — the container observed idle between moves, with the wall from a move to the
   other browser's screen measured through the DO the way M6.3 measured it through the tunnel.

---

## 6. The process that has worked (R-P11, refined over four cycles)

- **Brainstorm** with the owner (the superpowers brainstorming skill): one question at a
  time, the leans above as starting positions, the answers written into the spec's §1 as
  numbered rulings. M6.4 numbered its `R-6.4.n` rather than `R-4.n` because `R-4.1`–`R-4.4`
  already meant two other things in twenty-one code comments; **check the namespace before
  choosing the prefix.**
- **Spec** in M6.3's and M6.4's shape: §0 what is true at HEAD with line citations; §1 the
  rulings; the documents, the routes with their admission table, the client, the fences, the
  wire, the tests and the MUTATIONS THAT MUST BE CAUGHT, byte-identical, deployment, what it
  does NOT claim, the read order. Then an adversarial read of the spec against HEAD before
  the plan — M6.4's took 8 HIGH, 10 MEDIUM, 9 MINOR and 10 OPEN that way, with 80 claims
  verified true.
- **Plan** in the same shape: zero-context tasks each with files, interfaces, the code where
  it is known, the failing cells, the gate command and the `git add` line; plan-level rulings;
  a self-review section. Then subagent-driven development: Opus implementers and per-task
  Opus reviewers, Fable orchestrating and running the whole-branch review clean, one fix wave,
  one scoped re-review, the mutation battery, the counts in ONE uninterrupted sequence, the
  deploy, the HEADED live review, the close.
- **What M6.4 added to the recipe, and each earned its place:**
  - **Regenerate the corpus at every shape-changing task** (R-3-1), not at the end. The plan
    had two tasks regenerating and left `wire_fixtures` red for three tasks in between; the
    ruling made both suites green after every single task.
  - **Every agent writes its full report to a transcript file and replies with the path** —
    the notification channel truncates at about 4k.
  - **A subagent that backgrounds a long run and "waits for events" idles forever.** Dispatch
    says run in the FOREGROUND or poll the output file; when a status stalls, list the
    scratchpad by time and wake the agent with the facts.
  - **Never follow up an implementer without reading `git log -1` and `git status` first**,
    and never rewrite a reviewed SHA — a correction lands on top as a new commit. Held at all
    five fix rounds and both waves this cycle.
  - **Run the whole-branch review BEFORE the release** (R-order-1). M6.4 nearly did the
    reverse and the reviewer's own judgement was that four of its five Importants would
    otherwise have been a point release.
  - **The headed review's door recipe** (§2.6): WARP on, then `warp-cli debug access-reauth`.
  - **A headless browser never asks for a favicon**, and more generally: a claim about console
    errors is a claim about the browser that made it. The first headed review in this lane
    found a 404 that four headless ones had reported as "zero console errors".

---

## 7. Where the branches stand at this handover

- **Four stacked PRs, none merged, merge order the owner's:** `web-room` → `master` is **PR
  #31** (M6.1); `web-room-m6.2` → `web-room` is **PR #32**; `web-room-m6.3` →
  `web-room-m6.2` is **PR #33**; `web-room-m6.4` → `web-room-m6.3` is **PR #34**
  (https://github.com/jmoore2333/yokohama-assistant/pull/34). M6.5 branches from
  `web-room-m6.4`'s close HEAD as `web-room-m6.5`, the same shape — **unless the owner merges
  first**, which is the one decision that would change M6.5's base and is worth asking before
  the branch is cut.
- **The NAS runs `yokohama-web:0.5.0-m6.4.1`** in container `a62b17ad…` since 2026-09-20
  00:38:01Z, `YOKOHAMA_WEB_IDENTITY=claimed`, the three chat variables unset so their
  defaults apply, on the named volume `yokohama-web-data`. Fourteen image tags sit on the NAS
  and §5 of the runbook is rollback.
- **The SDD workspace for M6.4** (`.superpowers/sdd/2026-09-16-web-frontend-m6.4-plan/`,
  gitignored) holds every brief, report, review, the ledger and the live review's photographs.
  It is not tracked and it is not backed up; the close is what survives.
