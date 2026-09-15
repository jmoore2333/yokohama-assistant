# Web frontend — M6.2 "the lobby": spec

**Written 2026-09-15 on branch `web-room-m6.2` at `48ae908` (one commit over `web-room`'s
tip `937ce92`, which is PR #31 against `master`), from the brainstorm held the same morning
between the owner and the orchestrating session, and corrected the same day by an
adversarial read against HEAD (twenty-three findings, all folded in; the method is the
lane's: try the mechanism the sentence names).** The plan it executes the second cycle
of is `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` (its §0 carries the owner's
rulings R-P1–R-P11, cited here and not restated; its §2 carries this cycle's charter). The
entry artifact is the M6.1 close, `docs/plans/2026-09-15-web-frontend-m6.1-close.md`,
whose §8 names what M6.1 built for this cycle and whose §6 is the LEAVE list this cycle
inherits. **M6.3's spec was written beside this one on the same day**
(`docs/plans/2026-09-15-web-frontend-m6.3-transport-and-spectating-spec.md`), so that the
one mechanism the two share — a person arriving at a table they were not named at — is
designed once. The two cycles are executed one after the other, each with its own plan,
deployment, live review and close.

The name is what the cycle delivers: the first screen a person sees is the lobby, a table
can be opened with seats to fill, a link brings someone to it, and a person holding only
that link sits down and plays.

---

## 0. The problem, and the approach

### 0.1 What is true at HEAD

- **Every seat is named at create.** `POST /api/games` takes one `SeatSpec` per seat,
  `{"user": …}` or `{"bot": true}`, and refuses anything else
  (`crates/yokohama-web/src/games.rs:234`, `config_of`). A table is `dealing` from its
  first write (`games.rs:587`, `create`) and `TableStatus` has two values, `Dealing` and
  `Playing` (`table.rs:50`). There is no way to sit at a table you were not named at.
- **The list is the tables you sit at**, newest first, unbounded (`games.rs:918`,
  `list_for`, already sorted `(created_at desc, id asc)`), read from table documents
  alone — no session is rebuilt for a listing, which is the property this cycle keeps.
- **The game's NAME is its id.** `NewGame.name` becomes the storage id after the
  `is_safe_id` check (`games.rs:597-608`; `store.rs:152`), so a name with a space is
  refused (M6.1 close §6.2, live review Minor 1).
- **A human seat is an email and nothing else**: `SeatDoc::Human { user }` (`table.rs:37`).
  The client words a seat through `web/src/tables.ts` — `seatName` returns the email,
  `holder` and `tableWord` build on it — and `Dealing.tsx:31` imports `holder` from it.
- **The client has no URL routing.** `web/src/main.tsx` renders `App` and nothing reads
  `location.hash` or `history` anywhere under `web/src/`; a game is reached by clicking a
  row in the form's saved-tables list (`web/src/NewGame.tsx:88`, `savedTables`). There is
  no address for a table to hand to someone. The browser harness assumes the same:
  `web/e2e/harness.ts`'s `startGame` (`:141`) and `openFromList` (`:133`) both begin by
  asserting the "New game" heading.
- **Not seated is 403 on every session route** (M6.1 spec R-1.5): `seated()`
  (`lib.rs:375`) reads the TABLE first, deliberately — "a stranger holding an id learns
  that it is not theirs, and learns nothing about whether it is dealing"
  (`lib.rs:373-374`) — and then the handle, whose `Dealing` arm is the 409. `events` is
  the one session route that does not go through `seated` (`lib.rs:366-373`), because it
  answers a dealing table.
- **`table_lock` is one process-wide mutex** held across `create`, `keep`, `delete`, the
  settle path (`settle`, `games.rs:702`, is the acquirer; `settle_locked` runs with it
  held) and `finalize`'s bot pump (`games.rs:745`, `bring_alive`): while the bot searches
  its opening turn at one table, every create and keep at every table waits (M6.1 close
  §6.2, Server).
- **`games.rs` is 2,956 lines**, about 1,600 of them tests (`#[cfg(test)]` from `:1359`),
  with a clean seam between the table lifecycle and the record lifecycle that the close
  said to cut when this cycle touched the file. `TABLE_SCHEMA` is 1 (`:39`), written once
  (`:631`) and read nowhere.
- **The "New game" in-flight guard the room plan lists as inherited already shipped in
  M6.1**: `NewGame.tsx:103` (`inFlight`, a ref, because state could not hold it) with its
  cell `NewGame.test.tsx:238` ("a double-clicked Deal … posts create exactly once"). It is
  not this cycle's to build; it moves with the form.
- **The whole-game gate is out of CI** since `48ae908`: `.github/workflows/test.yml:46`
  runs `cargo test -p yokohama-web --lib --test wire_fixtures`, because the gate's
  OS-entropy game has read 857 s on identical code and PR #31's check sat in it for half
  an hour. Locally the gate is per-edit and unchanged.

### 0.2 The approach

**Chosen: the table document grows one status and one seat kind; a join is one route;
the lobby is two scopes over the listing that already exists; the link is an address.**
`TableStatus::Open` before `Dealing`, `SeatDoc::Open` for an unfilled seat, and
`POST …/{id}/join` that fills one — the join that fills the last seat flips the table to
`dealing` in the same write. Nothing about the deal, the keep, the finalize, the per-seat
views or the fences changes; a joined seat is a `Human` seat like a named one, and from
the flip onward M6.1 runs unmodified.

**Visibility is the house's, and a password is the gate (R-2.1).** Every signed-in person
sees every open table. A creator who ticks the box gives the table a password, stored as
a salted hash, presented on join. There is no token in a link and nothing to revoke: the
link is `#/t/<id>`, and what a stranger can do with it is bounded by Access in front of
the server and by the password at the table.

**Rejected — a capability token in the link.** A table invisible unless you hold its
link, the token stored on the document and carried in the URL. It puts a secret in browser
history and chat logs, needs a second grant for watching in M6.3, and buys privacy the
room does not need behind Access. The owner chose the house model with a password instead.

**Rejected — a `start` route.** The creator pressing Start once the seats are full. A
seat is a person or a bot, so a full table has nothing left to decide; a Start button
adds a pause and a route. "Start when full" means the last join deals (R-2.2).

### 0.3 The consequence worth stating first

A person who is not seated can now read a table — an OPEN one — and write to it, by
joining. That is the first widening of R-1.5 and the only one this cycle makes: at
`dealing`, `playing` and over, not seated stays 403 until M6.3's spectator. The fence
that matters is that an open table has no game behind it — no record, no session, no
hand — so what the widening exposes is the seat list and the name, which the lobby shows
to everyone anyway.

---

## 1. Rulings specific to this cycle (2026-09-15)

Beyond the plan's R-P1–R-P11:

- **R-2.1 — all tables are visible to the house; a password gates joining.** The lobby
  lists every open table on the server to every signed-in person. A table's creator may
  set one password at create; it is required to join (and, from M6.3, to watch) and
  nothing else. It is a shared secret with no complexity rule beyond non-empty, no
  lockout and no rate limit — "simple to run" (owner, 2026-09-15). Seats the creator named
  by email need no password.
- **R-2.2 — start when full is automatic.** The join that fills the last open seat flips
  the table to `dealing` in the same write. There is no `start` route and never a full
  table still marked open.
- **R-2.3 — the display name is a per-seat field beside the email, which stays the key.**
  `SeatDoc::Human { user, name }`. Sent on `join`, on `create` for the seats the creator
  holds, and on `keep` for a seat the creator named by email (that person's first write at
  the table). The browser remembers the last name typed. The close's §8 sentence holds:
  never a replacement for `user`.
- **R-2.4 — the game's name is a label, not its id.** The id is drawn (`sim-…`, as a
  default id is today) unless the request supplies one under the `is_safe_id` rules,
  which the fixture harness and the lib cells do. `name` is free text, spaces allowed,
  1–64 characters. This closes M6.1 close §6.2's first item.
- **R-2.5 — a join link is an address.** `#/t/<id>` on the site, no token. The client
  gets hash routes for the first time: `#/` (the lobby), `#/new` (the form), `#/t/<id>`
  (a table, opened by its status).
- **R-2.6 — abandon is the creator's delete**, the route that exists (`DELETE …/{id}`,
  creator-fenced), now answering at `open` as well. No new status.
- **R-2.7 — an open table is readable and joinable by any signed-in caller.**
  `GET …/{id}/table` and `POST …/{id}/join` answer a non-seated caller while the table is
  open; every other session route, and `table` at every other status, keeps R-1.5's 403
  — read FIRST, as `seated` reads it today, so a stranger never learns a table's status
  from a refusal. `TableView.my_seats` may therefore be empty: the sentence "Empty is
  impossible" at `web/src/types.ts:495-497` and its server twin at `lib.rs:387-393` both
  change.
- **R-2.8 — `sha2` enters the web crate.** The workspace has no hash crate
  (`fnv1a_64` in `crates/yokohama-core/src/game/semantics.rs:185` is a digest, not a
  password hash). `sha2 = "0.10"` is added to `crates/yokohama-web/Cargo.toml` and
  declared in its dependency paragraph as this cycle's owner-ruled addition; the salt is
  drawn through the crate's existing `draw_seed` (`games.rs:221`, OS entropy). No
  `argon2`: the threat is a shared game password behind Access.
- **R-2.9 — the lock splits by table.** `table_lock` stays as the GLOBAL lock for id
  allocation in `create` only; `join`, `keep`, `settle` (the acquirer today), `finalize`,
  `flip_to_playing` and `delete` take a per-table lock. This is M6.1 close §6.2's
  "finalize outside `table_lock`" restated so that the single-writer-per-table invariant
  every M6.1 race cell proves is kept: finalize leaves the global lock, not the table's.
- **R-2.10 — `games.rs` is cut along its seam** in the plan's first task, behaviour
  preserved and cell counts unmoved: `tables.rs` (create, join, keep, deal, settle,
  finalize, list, table, delete) and `games.rs` (handle, live handle, persist, the turn
  mirror, eviction). The names of the two files are the implementer's; the seam is not.
- **R-2.11 — inherited LEAVE items this cycle closes**: the unbounded list (paging,
  §3.3), an over table listing without the winner (§2, §3.3), the game's name as its id
  (R-2.4). The "New game" in-flight guard is NOT among them: it shipped in M6.1 (§0.1)
  and the room plan's charter line was stale. Every other §6 item stays on the list and
  is re-listed in this cycle's close.

---

## 2. The table document, widened

`crates/yokohama-web/src/table.rs`. `TABLE_SCHEMA` becomes **2**. A document at schema 1
(every table the NAS holds today) reads with every new field at its default; a document
above 2 is refused typed by `check_shape` — **a NEW arm**: today `schema` is written and
never read (§0.1). No document is rewritten on read.

```rust
pub enum SeatDoc {
    Human { user: String, #[serde(default)] name: Option<String> },
    Bot,
    Open,                                  // NEW — an unfilled seat
}
pub enum TableStatus { Open, Dealing, Playing }   // `Open` NEW, first
pub struct TurnMirror {
    pub to_move: u8,
    pub over: bool,
    #[serde(default)] pub winner: Option<u8>,     // NEW — the engine's winner, once over
}
pub struct Lock { pub salt: String, pub hash: String }   // NEW — hex of 16 random bytes; hex SHA-256(salt_bytes ‖ password_bytes)
pub struct TableDoc {
    …as today…,
    #[serde(default)] pub lock: Option<Lock>,    // NEW
}
```

`check_shape` (`games.rs:323`, moving with R-2.10) gains, in this order: `schema` at most
`TABLE_SCHEMA`; an `Open` seat only while `status == Open`; `status == Open` only while at
least one seat is `Open`; a `Lock`'s `salt` 32 hex characters and `hash` 64; every seat
`name` 1–64 characters with no control characters; `winner < players`. Every one of these
is read under a lock on a hand-editable file, which is the M6.1 §4.4 rule: **on a path
that holds a lock, every field read out of a stored document is an input.** The same
`name` rule is applied on WRITE (§3.2, §3.4), so a document this server wrote is never
one it refuses to read.

`TableDoc` gains `open_seats() -> Vec<PlayerId>`, `is_full() -> bool`, and
`check_password(&self, given: &str) -> bool` (constant-time over the hash bytes: a fold
of XORs, no `subtle` dependency, with a unit cell that a one-byte difference refuses).

`mirror_of` (`games.rs:364`) sets `winner` from the ENGINE's winner — the same value
`/result` projects as `Placing.won` (`outcome.rs:96`, `won: s.player == r.winner`), and
deliberately not `standings[0]`, which `outcome.rs:71` documents as rank order and which
the cell `a_tie_on_points_is_ranked_and_only_one_seat_won` (`:147`) exists to keep
distinct. One source.

### 2.1 `TableView`, the projection

Gains `locked: bool` (a `Lock` is present) and `winner: Option<PlayerId>`; a seat
serializes as `{"kind":"human","user":…,"name":…}`, `{"kind":"bot"}` or
`{"kind":"open"}`. **Neither the salt nor the hash is ever serialized**: `Lock` derives
`Serialize` for the document's sake, and `TableView` has no field of that type — it
carries the boolean. §7 says how that
is pinned.

---

## 3. The routes

### 3.1 Create — `POST /api/games`

`NewGame` (`games.rs:125`) changes shape; `deny_unknown_fields` stays and **`seed` stays**
(`games.rs:134`, the fixture harness's `new_game_body` sends it and the seed pin's
anti-vacuity partner reads it):

```json
{ "players": 3, "colors": ["Red","Blue","Green"], "start_seat": 0,
  "seats": [ {"user":"me@x.test"}, {"open":true}, {"open":true} ],
  "name": "Thursday's game",            // a LABEL now (R-2.4), optional, 1–64 chars
  "id": "wire",                          // optional; tests and fixtures; is_safe_id rules
  "display_name": "Jeremy",             // optional; written onto every seat the creator holds
  "password": "sesame",                 // optional; non-empty after trim, ≤ 64
  "seed": "20260831" }                  // unchanged: optional WireSeed, tests only
```

`SeatSpec` gains `open: bool`; exactly one of `user`, `bot`, `open` per seat. Refused in
the order a client reads, which is today's order with the new arms inserted: players
2..=4; colours; start seat; seat count; each seat's one-of; **"at least one seat must
stay human"** (the existing sentence, `games.rs:273-275`, whose predicate changes from
"every seat is a bot" to "no seat is `Human`" — so a table of open seats, with or without
a bot, is refused HERE, before the creator check, and the order cell's `.contains("human")`
at `:1614-1615` keeps its meaning); the creator holds a seat (R-1.6,
unchanged, so an all-open table is refused too); the bot at 3–4P (R-P10, the evaluator's
sentence, unchanged); `name` shape; `display_name` shape (the §2 rule); `password`
non-empty and ≤ 64; `id` safe. The cell `create_refuses_in_the_order_a_client_reads_and_binds_the_creator`
extends by the new arms in that order. The status written is `Open` when any seat is
open and `Dealing` otherwise — a 2P table with the bot is full at create and deals at
once, M6.1's path byte-for-byte. Answers **201 with the `TableView`**, as today.

### 3.2 Join — `POST /api/games/{id}/join`

```json
{ "seat": 1, "name": "Alex", "password": "sesame" }     // name and password optional
```

Under the table's lock (R-2.9), refused in this order, each a sentence in the API's
`{"error": …}` shape: 404 no such game; **403 "you are not seated at this table"** is
NOT an arm here — a stranger is exactly who joins; **409 `DEALING_SENTENCE`** ("this
table is still dealing") at `dealing`, and **409 `PLAYING_SENTENCE`** ("this table is
playing") at `playing` and over — the two status arms of §3.4, each carrying `status`
beside the sentence, so a client that arrived by link draws the right screen; 400 seat out of range; 400 `name` shape (§2's
rule: 1–64 characters, no control characters, after trim); **409 "seat N is taken"**
(a `Human` or `Bot` seat — a seat the creator named by email is `Human` and is therefore
taken, never joined); **403 "this table is locked — send its password"** (a `Lock` and
no password); **403 "wrong password for this table"**. On success the seat becomes
`Human { user, name }` and, if no `Open` seat remains, `status` becomes `Dealing` in the
same `write_if_version` — the flip and the fill are one write (R-2.2). Answers **200 with
the `TableView`** for the caller (`my_seats` now includes the seat). A person may join a
second seat (hot-seat is M6.1's and stays).

### 3.3 List — `GET /api/games?scope=mine|open&limit=N&before=CURSOR`

`ListQuery` is a query struct under `QueryArgs` with `deny_unknown_fields`, the pattern
of `IndexQuery` and `LogQuery`. `scope` defaults to `mine`; `limit` defaults to 20 and is
capped at 100; `before` is a cursor of the form `<created_at>,<id>` naming the last row
the caller saw, and the reply carries `next` in the same form when more rows exist:

```json
{ "games": [ …TableView… ], "next": "1789000000,sim-0123456789abcdef" }
```

`mine` is `list_for`'s rows: tables where the caller holds a seat, every status, newest
first, with `to_move`, `over` and now `winner`. `open` is every table whose status is
`Open`, any creator, newest first. Both read table documents ALONE — the cell for it
gives the store a record reader that panics, and the listing must not touch it. The sort
is `(created_at desc, id asc)`, which `list_for` already does and which makes the cursor
total. A malformed `before` is a 400; an unknown key is a 400.

### 3.4 Table, deal, keep, delete, and the status refusals

- `GET …/{id}/table`: at `Open`, any signed-in caller (R-2.7); otherwise seated only, as
  today. The view is the caller's (`my_seats` may be empty).
- `GET …/{id}/deal` and `POST …/{id}/keep`: unchanged, except that `Keep` gains
  `name: Option<String>`, checked by §2's rule (400) and written onto the seat when the
  seat's name is `None` (R-2.3).
- `DELETE …/{id}`: unchanged; the creator-fence answers at `Open` too (R-2.6). The
  race cell from M6.1 (`a_delete_cannot_interleave_with_the_keep_that_finalizes`) stays
  and gains a sibling for a join: a delete cannot interleave with the join that fills the
  last seat.
- **The status refusals.** `GameError::Dealing` and `DEALING_SENTENCE` ("this table is
  still dealing", `games.rs:162`, `:188`) survive unchanged; beside them TWO new arms of
  the same shape, `GameError::Open` with `OPEN_SENTENCE` = "this table is still open"
  and `GameError::Playing` with `PLAYING_SENTENCE` = "this table is playing". All three
  map to **409** with `ApiError.table_status` (`lib.rs:81`) set to `"open"` /
  `"dealing"` / `"playing"`: `From<GameError> for ApiError` (`lib.rs:120-135`) gains the
  two arms beside `E::Dealing => Self::dealing()`. `E::Conflict` keeps
  `table_status: None` (`lib.rs:130`) and is never used for a status refusal. The
  client's `DealingErrorBody.status` (`types.ts:534-536`, today `string`) narrows to
  that union.
- **Who sees which refusal at an open table.** A SEAT HOLDER calling any session route
  other than `table`, `join` and `DELETE` gets the 409 `open`. A STRANGER gets 403 "you
  are not seated at this table" from the table read that comes first (§0.1), on every
  route but `table` and `join`; `DELETE` answers a stranger 403 as today.

### 3.5 The lock split (R-2.9)

`Games` gains `locks: Mutex<HashMap<GameId, Arc<Mutex<()>>>>`; `lock_for(id)` hands out
the table's own mutex. `create` holds `table_lock` (global) across the occupancy check
and the first write and nothing else. `join`, `keep`, `settle` (so `settle_locked` keeps
its "already held" contract), `finalize`, `flip_to_playing` and `delete` hold the table's
lock. **The M6.1 race cells are the proof that nothing was lost** — with one re-pointing
R-2.4 forces and the cut (R-2.10) makes cheap: `two_creates_of_one_name_cannot_both_succeed`
(`games.rs:2548`) and `an_in_flight_write_cannot_land_on_a_different_game_that_took_the_name`
(`:2426`) build their requests through `req(name)`, which sets `NewGame.name`, the id
today; under R-2.4 the helper sets `id` instead and every assertion body is unchanged.
`a_delete_cannot_interleave_with_the_keep_that_finalizes` and
`a_delete_is_not_undone_by_a_move_already_in_flight` pass as they are. The new cell is
the one the split exists for: with a two-gate store parking table A's record write inside
`finalize`, a `join` at table B completes and answers 200 before A's gate opens.

### 3.6 The cut (R-2.10)

The plan's first task: `games.rs` becomes the record lifecycle and `tables.rs` the table
lifecycle, every function and every test moved with its subject, no body edited, the
crate's test count identical before and after (**115 lib at `48ae908`**), clippy clean.
It is a separate commit so the diff of every later task reads as change and not as
motion.

---

## 4. The password

- **At create**: `password` trimmed; empty after trim is a 400 "a password is at least one
  character"; over 64 a 400. Stored as `Lock { salt, hash }` — 16 bytes (two `draw_seed`
  draws) as lowercase hex, and `hex(SHA-256(salt_bytes ‖ password_bytes))`.
- **At join**: the given password trimmed the same way; `check_password` compares in
  constant time. A table without a `Lock` ignores a password sent to it.
- **Never on the wire.** Two catchers, neither the other's fallback. (1) A by-key cell:
  the serialized `TableView` of a locked table has no key named `salt`, `hash` or `lock`,
  and `locked` is `true`. (2) A by-VALUE search in `wire_fixtures.rs`: the walk creates
  its table with the constant password `WIRE_PASSWORD`; at the end of the walk the test
  reads the table DOCUMENT out of the memory store (`api.state.games.store()`), extracts
  this run's salt and hash, and searches every payload the run produced — the in-memory
  corpus, before it is compared with the committed files — for the password's text, the
  salt's hex and the hash's hex. The salt is drawn from OS entropy, so the search must be
  over THIS run's values and cannot be a constant, which is why the seed pin's shape is
  reused only in part. The anti-vacuity partner is that the document does carry both. And
  a salt that reached a payload would also fail `the_committed_wire_fixtures_are_what_the_server_emits`
  (`wire_fixtures.rs:1193`) on the next run, since the committed file could not contain
  it — recorded as the third catcher, not relied on.
- **Not a security boundary**, said in the same place R-1.4 is said: `claimed` lets
  anyone past Access claim any email; the password is what keeps a friend from sitting
  down at the wrong game, not what keeps an attacker out. No lockout, no rate limit, no
  password change after create (abandon and recreate).

---

## 5. The client

### 5.1 Routes — `web/src/route.ts` (new)

`parseRoute(hash: string): Route` with `Route = {kind:"lobby"} | {kind:"new"} |
{kind:"table", id: string}`; unknown hashes parse as the lobby. `useRoute(): Route`
reads `location.hash` at mount and on `hashchange`. `navigate(route)` sets the hash. The
app's screen selection after the door becomes a function of the route:

| route | screen |
|---|---|
| `#/` | the lobby (§5.2) |
| `#/new` | the form (§5.3) |
| `#/t/<id>` | by the table's status: `open` → the open-table screen (§5.4); `dealing` → `Dealing` (as today); `playing`/over → the game (as today, through `open(id)`) |

A `#/t/<id>` the caller cannot see (403) or that does not exist (404) shows the sentence
above the lobby and stays at `#/`. `open(id)` (`App.tsx:270`) sets the hash; leaving a
game sets `#/`. Back and forward work because the hash is the state.

### 5.2 The lobby — `web/src/Lobby.tsx` (new)

The first screen after the door. Two sections, each from `api.list(scope, before)`:

- **Your games** (`mine`): one row per table — the name (or the id when unnamed), the
  seats through `seatNames`, and the state through `tableWord` (§5.6): "open · 2 of 3
  seated", "dealing", "**your turn**", "waiting on <name>", or "over · <name> won". A
  "more" control pages with `next`.
- **Open tables** (`open`): every open table on the server, the same row with a lock
  marker when `locked`; a row is a link to `#/t/<id>`.

A **New table** button goes to `#/new`. The lobby refetches both lists on the same
rules as a game's signal: from a visible tab only, every 3 s, once on becoming visible,
backoff on failure (§5.5). The "You are" strip and sign-out sit above it as they sit above
the form today (`App.tsx:548-559`, `who`).

### 5.3 The form — `NewGame.tsx`, re-pointed

- The saved-tables list leaves the form (it is the lobby now).
- Seat rows gain **open** as a kind beside me / another email / bot; defaults: 2P is me +
  bot as today, 3P and 4P are me + open + open (+ open).
- **Name** is a free label (R-2.4); the "not a usable game name" refusal path is gone
  from the form and the server's `id` is never typed here.
- **Your name** (display name), prefilled from `localStorage["yokohama.display_name"]`
  and written back on submit; sent as `display_name`.
- **Password** — a checkbox "Ask for a password to join" that reveals a text field; sent
  only when ticked and non-empty.
- **The in-flight guard stays exactly as shipped** (`NewGame.tsx:103`, its cell at
  `NewGame.test.tsx:238`); the re-point must keep both green.
- On 201: `navigate({kind:"table", id})`. An open table lands on §5.4; a full one on the
  dealing screen, as today.

### 5.4 The open table — `web/src/OpenTable.tsx` (new)

One component, two faces by `my_seats`:

- **Not seated**: the seat picker. Every seat drawn with its colour; a filled seat shows
  `seatName` (§5.6) or "the bot"; an open seat is a button. Above them, **Your name**
  (prefilled as in §5.3) and, when `locked`, a **Password** field. Clicking a seat posts
  `join`; a 403 shows its sentence in place and keeps the fields; a 409 "taken" refetches
  the table; a 409 with a `status` navigates by that status.
- **Seated** (the creator, or someone who just joined): the same seats, read-only; the
  table's **link** (`location.origin + "/#/t/" + id`) in a read-only field with a Copy
  button; **Abandon** for the creator only (the existing `api.remove`, `api.ts:204`, with
  a confirm step that is a second click, never a `window.confirm`); "waiting for N more"
  as the status.
- Both faces refetch `/table` on the §5.5 rules and, on reading `dealing`, hand over to
  the dealing screen exactly as `open()` does today (`setDealing(id)`); on 404 (abandoned)
  they navigate to the lobby with the sentence.

### 5.5 The poll, generalized — `signal.ts`

`useSignal(game, active, onSignal)` keeps its name and contract (its cells stay green
unchanged). Its body moves into

```ts
interface Source<T> { key: string; once: () => Promise<T>; stream: string }
function usePolled<T>(source: Source<T> | null, active: boolean,
                      onValue: (v: T) => void, onEnd?: (end: End) => void): Link
interface End { reason: "deleted" | "gone" | "refused"; sentence?: string }
```

which owns the visibility rule (R-P8), the 3 s cadence and the backoff. **The effect
depends on `source.key` and `active` alone**; `once`, `onValue` and `onEnd` are held
through refs exactly as `onSignal` is today (`signal.ts:53-58` states the hazard: a
consumer's closure is a new function on every render, and an effect that depended on it
would re-arm the timer faster than its own interval). `stream` is the resource's stream
URL, carried and UNUSED in this cycle — M6.3 reads it (its §5.1), which is why the
signature is fixed here and not there. `onEnd` is unused in this cycle for the same
reason. `useSignal` becomes the instance whose `key` is the game id, `once` is
`api.events(game)` and `stream` is `…/{id}/stream`. The dealing screen (`Dealing.tsx:192`)
is the second instance today; the open table and the lobby are the third and fourth.
**One hook, four consumers**, and the transport under it changes once.

### 5.6 Where a seat is printed — `tables.ts`, widened

`web/src/tables.ts` is already the one place a seat is worded (§0.1). `seatName` becomes
`seat.kind === "bot" ? "the bot" : seat.name ?? seat.user`, and gains an `open` arm
("open seat"); `tableWord` gains the open and the over-with-winner words of §5.2, reading
`winner` through `holder`. `tables.test.ts` grows by the new arms and by one cell per
surface that prints a seat — the lobby rows, the open table, the dealing screen's
"waiting on" line (`Dealing.tsx:256-257`) — over a fixture with a named seat and an
unnamed one, asserting the name where present and the email where not. No new file.

### 5.7 Wire types — `types.ts`

`SeatDoc` gains `name: string | null` on the human arm and an `{kind:"open"}` arm;
`TableStatus` gains `"open"`; `TableView` gains `locked` and `winner`, and `my_seats`'s
doc comment loses "Empty is impossible" (R-2.7); `GamesList` gains `next: string | null`;
`NewGameRequest` follows §3.1; `JoinRequest` is new; `DealingErrorBody.status` narrows to
`"open" | "dealing" | "playing"` (§3.4). `contract.ts` type-checks all of it against the
regenerated corpus (§7).

### 5.8 The browser harness — `web/e2e/harness.ts`

`startGame` (`:138`) goes through the lobby: sign in, expect the lobby's heading, click
New table, then the form as today. `openFromList` (`:132`) opens the row under Your games
by the table's name or id. Both are called by five default-glob specs
(`plays_a_whole_game`, `the_playback`, `the_screen_itself`, `the_table`, `two_browsers`)
and the two opt-in generators (`gates`, `shots`), which is why the change is in the
harness and not in any spec.

---

## 6. Fences

- **Open is the only widening.** At `dealing`, `playing` and over, every session route
  answers a non-seated caller 403 exactly as at `48ae908`; the M6.1 cell sweeping every
  route as a signed-in stranger is extended to run at all FOUR statuses and to assert the
  two exceptions at `open` by name, and that at `open` a stranger's refusal on every
  other route is the 403 and never the 409 (§3.4).
- **A join is admitted only at `open`, only on an `Open` seat, only with the password
  when locked**, in that order, and the fill-and-flip is one write.
- **The listing builds no session and reads no record.** Pinned with the panicking
  record reader (§3.3).
- **The acting-seat fence, the undo rule, masking and the seed pin are untouched** and
  their cells run unchanged; the seed pin gains no exemption.
- **`check_shape` bounds every new field** on the locked path (§2), and every field it
  bounds on read is bounded on write.
- **A joined seat is dealt exactly as a named one**: the two-identity gate arm
  (§8.3) is the proof, because it now arrives through `join`.

---

## 7. The wire: routes and the corpus

Routes at the close of this cycle: the twenty of M6.1 (`lib.rs:1401-1420`) plus
`POST …/{id}/join`. Every request body and query struct carries `deny_unknown_fields`.

The corpus (`web/src/wire/`, **54** files at `48ae908`, emitted as `<name>.json` by
`wire_fixtures.rs:1204`) is regenerated, the variable-unset run refuses on any drift, and
every diff is READ and reported in the close. Expected:

- **Re-cut**: every fixture carrying a `TableView` — `games.json`, `table.json`,
  `table_dealing.json`, `deal.json`, `deal_waiting.json`, and the 4xx fixtures that
  embed one — gains `locked`, `winner` and the seat `name`s; `games.json` gains `next`.
- **New**: `table_open.json` (an open, LOCKED 3P table read by a NON-seated caller:
  `my_seats: []`, one human seat named, two open); `games_open.json` (`scope=open`
  with a locked and an unlocked table); `games_next.json` (`scope=mine&limit=1`, a
  `next` present); `join_409_not_open.json`, `join_409_taken.json`,
  `join_403_locked.json`, `join_403_password.json`; `open_409.json` (`GET …/{id}` as a
  seat holder at an open table, the `status: "open"` sibling of `dealing_409.json`).
- **The walk** (`wire_fixtures.rs:316`) gains, before its dealing steps: the walker
  creates the table with `id: "wire"`, one open seat and `WIRE_PASSWORD`; a second
  identity reads it, joins with the password; the flip is what the existing dealing steps
  then see. `SETUP_SEED` (`:60`) is still sent.
- **The value pin** (`no_committed_payload_carries_the_setup_seed`, `:1134`) gains the
  sibling of §4, `no_payload_of_this_run_carries_the_password_or_its_hash`, searching the
  run's own payloads for the run's own values.
- **The client's key-set rows** (`contract.test.ts`) gain the new fixtures.

---

## 8. Tests, gates, and the mutations that must be caught

### 8.1 Server (`cargo test -p yokohama-web`)

- **Documents**: the schema-1 `two_seat` literal at `games.rs:2064-2083` (a real
  `TableDoc`, with `schema`, `seed` and `keeps`) reads with `lock: None`, every seat
  unnamed, `winner: None`; `schema: 3` refuses with the sentence; `check_shape`'s new
  arms each refuse a hand-built document with the sentence; `check_password` accepts the
  password and refuses a one-byte variant and the empty string; two `Lock`s for one
  password differ (salt).
- **Create**: each new refusal by sentence in order, the existing order cell extended;
  `open` when any seat is open and `dealing` otherwise; a 2P bot table deals at create
  (unchanged cell); `display_name` lands on every creator seat and no other; `name` with
  a space accepted; `id` absent draws a `sim-` id; `id` present and unsafe refuses.
- **Join**: the refusals in order (§3.2); success binds the seat with the name; the
  filling join flips in one write (the store's write count is 1, and the document read
  back is `dealing` with no `Open` seat); a second join by the same person takes a second
  seat; a named-by-email seat is "taken"; a 500-character name is 400 and the document is
  unchanged; a delete cannot interleave with the filling join.
- **Keep**: `name` written when the seat's is `None`, left alone when set; a bad name is
  400; at `open` refused with `status: "open"`.
- **List**: `mine` and `open` membership by construction (three tables, two callers);
  order; `limit` and `before` page without gap or overlap over 5 tables at limit 2;
  malformed `before` and unknown keys 400; the record-reader-panics cell; a malformed
  table document is skipped with its log line (the existing cell, unchanged).
- **Views**: `table` at `open` for a stranger 200 with `my_seats: []`; at every other
  status 403; the status sweep of §6; `TableView` serializes no `salt`, `hash` or `lock`
  key (§4's by-key cell).
- **Locks**: §3.5's two-gate cell; the four M6.1 race cells, two of them re-pointed to
  `id`.
- **Mirror**: `winner` set once over, equal to the seat whose `/result` placing has
  `won: true`; `None` while playing.

### 8.2 Client (`npm run typecheck && npm test`)

Component cells over the new fixtures only: the lobby renders both sections from
`games.json` and `games_open.json` with the lock marker, the whose-turn line, the
winner's name and the "more" control (from `games_next.json`); a row navigates
(`location.hash`); the form's open kind and its defaults at 2/3/4P, the password box
revealing the field, the display name read from and written to storage, the in-flight
guard's existing cell green after the re-point; the open table's two faces from
`table_open.json` and `table.json`, the seat buttons, a 403 shown in place, a copy of the
link, abandon's second click; `route.ts`'s parse table incl. unknown hashes;
`usePolled`'s visibility and backoff rules (the moved `signal.test.ts` cells) plus the
ref rule (a consumer re-rendering with a new `once` closure does not re-arm the timer)
and one instance cell for the lobby; `tables.ts`'s new arms and the per-surface cells
(§5.6).

### 8.3 The gates that play

- `crates/yokohama-web/tests/play_a_whole_game.rs`: the two-identity arm (`:266`) is
  re-pointed through the lobby — red creates a 2P table with one open seat and a
  password, blue lists `scope=open`, finds it, joins with the password and a name, both
  keep, and the game plays to a winner with the same per-reply assertions as today (the
  requester's seat, its own hand the only unmasked one). The vs-bot arm (`:141`) is
  unchanged. **Both stay out of CI (§0.1) and per-edit locally.**
- `web/e2e/joins_by_link.spec.ts` (new, the seventh browser spec): browser A signs in,
  creates a 2P open table with the password `sesame` and the name "Link test"; browser B
  signs in as another email and navigates DIRECTLY to `/#/t/<id>`, never through the
  lobby; B sees the seat picker, types a name and the password, sits; A's open-table
  screen shows B's name WITHOUT a reload (the poll; the cell asserts no navigation
  happened on A); both reach the dealing screen, keep, and play **six moves** alternately
  (the exit criterion is "joins and plays"; a winner is `two_browsers.spec.ts`'s job and
  stays there). Then A's lobby lists the table under Your games with B's name and whose
  turn it is. **The file carries the file-level `test.skip(!haveBrowser, …)` and the
  `haveBrowser` import from `../playwright.config`**, because `every_spec_can_skip.spec.ts`
  reads the directory at run time and FAILS on a spec without them (`:40`, `:44`) — it
  checks the convention; it does not exempt a new file from it.
- The harness re-point (§5.8) is what carries the other five specs and the two generators.

### 8.4 Mutations that must be caught (recorded in the close, per R-P11)

1. `join` admitted at a table whose status is not `Open`.
2. `join` admitted on a seat that is `Human` or `Bot`.
3. `check_password` returning true for a wrong password (or the lock ignored when no
   password is sent).
4. The salt or the hash serialized onto `TableView` under any key: the by-key cell AND
   the run-value search must both fire (they are the two catchers of §4).
5. The filling join writing the seat but not the flip (two writes, or no flip).
6. `scope=mine` including a table where the caller holds no seat.
7. The `id` derived from `name` again (a name with a space becoming a refused id).
8. The listing rebuilding a session (the panicking record reader stays silent).
9. A join or keep writing an unbounded `name` (the 500-character cell).
10. The e2e's "B's name appears on A's screen" passing because A was reloaded (the cell
    asserts no navigation happened on A between B's join and the name appearing).

### 8.5 Byte-identical

`git diff master -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest
crates/yokohama-bridge crates/gen-data` is EMPTY at the close; `SEMANTICS_REV` is 7;
`cargo test -p yokohama-cli` passes with its goldens and `guide_sync` untouched (this
cycle touches the CLI nowhere); clippy clean at `-D warnings`; the five documented
suites of `CLAUDE.md` re-counted at the close HEAD in one sequence.

---

## 9. Deployment

- The image tag is `0.5.0-m6.2`; the recipe is `deploy/RUNBOOK.md` §1 (build), §2
  (deliver) and §3 (run), by reference; the record is a new §3.8 addendum after §3.7's
  M6.1 record. No new environment variable. The container `Env` keeps
  `YOKOHAMA_WEB_IDENTITY=claimed`.
- The store on the volume holds schema-1 table documents from M6.1; they list and open
  unchanged (§8.1's schema-1 cell is the proof before the deploy). The owner deletes
  `m6.1-live-review` first, as the M6.1 close asked; the live review creates its own.
- Verification from the shell (WARP on): `GET /api/games?scope=open` with the identity
  header is `{"games":[],"next":null}` or the open tables; a create with one open seat
  and a password by one header, a `join` by another with the wrong password 403, with
  the right one 200 and the table `dealing`.
- The live review is against the live site in BOTH layouts and from two browsers as two
  people, one of whom arrives by link only (R-P11).

---

## 10. What this spec does NOT claim

- **No spectators, no live transport, no chat, no hosting**: M6.3–M6.5. The lobby and
  the open table poll at 3 s from a visible tab; M6.3 removes the poll.
- **The password is a shared secret and not a security boundary**: no lockout, no rate
  limit, no change after create, stored as a salted SHA-256 and compared in constant
  time, behind Access and beside a `claimed` door that is itself not a boundary (R-1.4).
- **Every signed-in person sees every open table's seat list, emails included.** M6.1
  showed other seats' emails to seated players; the house model shows an open table's
  to the house. A display name is what most people will see; the email is on the wire.
  No per-seat privacy of who is at a table.
- **No invitation to a named person by link, no kick, no seat change after joining, no
  colour choice at join** (colours are the creator's at create), no reopening a full
  table. Abandon and recreate.
- **No hosted visibility flag.** A public deployment wanting private tables is M6.5's
  configuration, on the seam this cycle leaves: one status, one gate.
- **No size or wall figure for any of this**; the whole-game gate's wall stays a
  recorded reading, not a budget (§0.1).
- Every count, line number and route name above is at `48ae908`; the implementation
  plan re-reads them at its own HEAD.

---

## 11. Read order for the implementer

1. `CLAUDE.md` — the Web (server) and Web (browser half) blocks, and the
   `crates/yokohama-web` + `web/` architecture bullet.
2. `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0, §1 and §2.
3. `docs/plans/2026-09-15-web-frontend-m6.1-close.md` §1, §4.4, §6 and §8 — what is
   built, the finding that justified it, the LEAVE list, and what this cycle starts from.
4. This document.
5. `docs/plans/2026-09-15-web-frontend-m6.3-transport-and-spectating-spec.md` §3.4,
   §3.5 and §5.1 — what the next cycle does with the seams this one leaves (the
   password's `unlock`, the open-table read, the poll hook).
6. `docs/plans/2026-09-14-web-frontend-m6.1-identity-and-seats-spec.md` §4 and §5 —
   the table document and the fences this cycle widens by exactly one status.
7. `deploy/RUNBOOK.md` §0, §1–§3, §3.7.
