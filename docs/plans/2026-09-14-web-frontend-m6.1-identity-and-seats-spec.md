# Web frontend — M6.1 "identity and seats": spec

**Written 2026-09-14 on branch `web-room` at `2533001` (master's tip that day), from the
brainstorm held the same day; the plan it executes the first cycle of is
`docs/plans/2026-09-14-web-frontend-m6-room-plan.md` (its §0 carries the owner's rulings,
R-P1–R-P11, which this spec cites and does not restate).** M6's entry artifact
(`docs/plans/2026-09-06-web-frontend-m6-handover.md`) is the record this cycle opens from;
M5.5's spec and close, and the 2026-09-06 owner-corrections record, stay the authority for
the table, the walk and the set-table screen this cycle re-points. Nothing here re-opens
an M4 or M5.5 ruling except where a section says which and why (§5.1 on the replay pin,
§4.2 on the hot-seat pass).

The name is what the cycle delivers: a request has an identity, a seat has a person, and
a payload is built for the person asking. With those three, two people at two browsers can
play each other, and the honest interim the corrections wave shipped — one browser serving
every seat, secrecy by removing an earlier chooser's cards from the DOM — becomes what the
handover said it should: a per-seat view like every other payload this server serves.

---

## 0. The problem, and the approach

### 0.1 What is true at HEAD

- **The server reads no headers and knows no one.** `GET /api/games` lists every game id
  to any caller; every payload is built for `viewing_seat(repl)`, the ACTING seat
  (`crates/yokohama-web/src/client.rs:225`), which follows the mover. Safe only because
  Cloudflare Access gates the hostname and the origin is reachable no other way
  (`deploy/RUNBOOK.md` §0).
- **One browser holds every human seat.** The M5.5 set table asks each human seat for its
  keep IN SEQUENCE (`web/src/NewGame.tsx:118`, the `turn` index and its "Next seat"
  button), and `POST /api/games/preview` returns every human seat's two cards to that one
  browser; `create` then takes every keep at once (`games.rs:87`, `keeps`). The bot's deal
  never reaches the wire (the corrections' finding 1), and that is the only per-seat
  secrecy the wire has.
- **The one real secret is the starting order**, and in a simulation game the engine
  knows every seat's (`starting_order_hidden` is false at every seat; `observe.rs:34`
  refuses a view for a seat whose own deal is unrecorded, which never happens in sim).
  `screen_model::build_hand` (`screen_model.rs:2315`) lists a hand with
  `is_starting_order` marked whenever the hand is not hidden — so a frame that shows
  another seat's hand shows its secret. Today that is fine at one screen and would be a
  leak at two.
- **The setup seed crosses the wire once**, in the preview, and round-trips back into
  `create` as a string (`WireSeed`), under a corpus pin that names the exemption
  (`wire_fixtures.rs`, `no_committed_payload_carries_the_setup_seed_except_the_preview`).
- **The Worker already forwards an identity** as `X-Yokohama-User` and strips a
  client-supplied one (`deploy/worker/src/index.ts`, `CLIENT_MUST_NOT_SET`), so that the
  origin reading it later is a safe change. The tunnel path forwards Access's own
  `Cf-Access-Authenticated-User-Email`. Neither is read.
- **Undo is the CLI's**: `Session::undo` (`session.rs:717`) pops the trailing move with
  its bound reveals and re-derives the state by replay; `Repl` unwinds a trailing bot run
  with the human move by replay-derived attribution. The grammar allowlist admits `undo`,
  `back` and `privacy` (`grammar.rs:56`).
- **The evaluator is measured at two players only** (`eval.rs:1044`); the server admits a
  bot at three and four (`games.rs:195-223` checks only the seat index).

### 0.2 The approach, and the two rejected

**Chosen (R-P4): identity at the origin, seat bindings in a document beside the record,
views scoped in the server.** One extractor over one configured source; one `table`
document per game in the same store; every game route resolving the caller to a seat and
passing seat facts the SERVER computes into the existing `build_frame`, so masking is the
mechanism M5.5 already has (`SeatFacts.hand_hidden`) and no new masking code is written.
Core, the `GameRecord` and the CLI's behaviour stay untouched; the NAS and a hosted
container run identical code.

**Rejected — identity and seat-mapping at the edge.** The Worker would resolve the person
and forward a seat; the origin would stay identity-free. The NAS has no Worker
(cloudflared runs no logic), so the NAS test and the hosted deploy would be different
code, and the seat map would need a second store at the edge.

**Rejected — seat bindings inside the `GameRecord`** through its `SeatProvenance`
(`record.rs:106`). It touches `yokohama-core`, it puts emails into save files that get
pulled and shared for review, and it couples identity to the `SEMANTICS_REV` key.

### 0.3 The consequence worth stating first

The deal moves. A game is created in a `dealing` state; each seated person sees only their
own two cards over the set table and posts their keep; the server finalizes when the last
human has chosen. The `preview` route retires with it, and with it the last place the seed
crosses the wire. Hot-seat (one person in several seats) and human-vs-bot go through the
same steps and lose their pass screen.

---

## 1. Rulings specific to this cycle (2026-09-14)

Beyond the plan's R-P1–R-P11:

- **R-1.1 — the identity is a normalized email string.** Trimmed, lowercased, exactly one
  `@` with a non-empty local part and domain, no whitespace or control characters, at
  most 254 bytes. A shape check, not RFC validation. Normalized ONCE, in the extractor.
- **R-1.2 — every mode reads exactly one header** and ignores every other identity
  header. Pinned by a request carrying all three with three different values.
- **R-1.3 — no default mode.** `YOKOHAMA_WEB_IDENTITY` unset refuses to start, naming the
  accepted values — the Worker's fail-closed shape for its secrets.
- **R-1.4 — the NAS runs `claimed` behind Access for now** (R-P2). It is a development
  convenience, not a security boundary: anyone past Access can claim any email. Said
  plainly in the runbook and in `/api/me`'s doc.
- **R-1.5 — not seated is 403.** Spectating is M6.3's; until then a caller with no seat
  at a table gets `{"error": "you are not seated at this table"}`.
- **R-1.6 — the creator holds at least one seat.** A table is created by someone who sits
  at it; the lobby may relax that.
- **R-1.7 — a hand is shown to its own seat only**, bots included, hot-seat included. The
  other seats show as counts. The alternative projection — other seats' PUBLIC cards
  shown with the unfulfilled starting order masked, which is what `observe.rs` hands a
  bot — is a later view option, not this cycle's.
- **R-1.8 — `suggest` answers the acting seat's holder only.** A caller who does not
  hold the acting seat gets 403 "it is not your turn", so the advisor never ranks a move
  for a seat the caller cannot play.
- **R-1.9 — the health line names the mode.** `GET /api/health` gains `identity: "<mode>"`
  so a deployment can be verified from the shell without a browser.

---

## 2. Identity

### 2.1 The extractor

`crates/yokohama-web/src/identity.rs` (new):

```rust
pub enum IdentityMode { Claimed, CfAccess, Worker }
pub struct User(String);                    // R-1.1 normalized; Display = the email
pub struct Identity(pub User);              // axum FromRequestParts<Arc<AppState>>
```

| mode | `YOKOHAMA_WEB_IDENTITY` | header read | absent or invalid |
|---|---|---|---|
| Claimed | `claimed` | `X-Yokohama-Claimed-User` | 401 |
| CfAccess | `cf-access` | `Cf-Access-Authenticated-User-Email` | 401 |
| Worker | `worker` | `X-Yokohama-User`; the Worker's `anonymous` counts as absent | 401 |

`AppState` gains `identity: IdentityMode`. `AppState::new()` (`lib.rs:169`) reads the
variable and `main.rs` exits 1 with the sentence when it is missing or unrecognized.
`AppState::in_memory()` is `Claimed` — the mode every test drives with a header — and
`AppState::in_memory_with(mode)` exists for the mode cells. Reserved and unbuilt:
`cf-access-jwt` (verify `Cf-Access-Jwt-Assertion` against the team's certs endpoint and
audience) and `neon`; each is a new arm of the same `match`, and each is a JWT plus JWKS
dependency the runbook prices when a deployment needs it.

**The 401 is this API's `{"error": …}` shape**, worded per mode: in `claimed`, "no signed-in
user: send X-Yokohama-Claimed-User"; in the other two, "no identity reached the server:
this deployment expects <header> from <door>". `web/src/api.ts` already reads the status
before the body, so an expired Access session and a missing claimed header both surface
as sentences.

### 2.2 What requires an identity

Public: `/api/health`, `/api/labels`, `/api/art`, `/art/*`, the client bundle, and
`GET /api/me`. Everything under `/api/games` requires `Identity`. The route table in §8
says so per route.

### 2.3 `GET /api/me`

```json
{ "mode": "claimed", "user": "a@example.com" }
{ "mode": "cf-access", "user": null }
```

Answers without identity and never 401s: `user` is what the mode resolved, or null — an
absent header and an invalid one both resolve to null here, so the sign-in screen is the
answer to a bad stored value as well as to none. The client's rule (§7.1): null under
`claimed` means "show the sign-in screen"; null under any other mode means "this
deployment is wrong", shown as an error naming the door, never as a form.

### 2.4 The runbook's two sentences

`cf-access` is trustworthy only because port 8080 is unpublished and cloudflared is the
one path to it (`RUNBOOK.md` §3: "Do not publish 8080 to the LAN"). `claimed` behind
Access authenticates the DOOR and not the SEAT (R-1.4). Both go beside the mode in the
runbook's deployment addendum, not in code comments alone.

---

## 3. The store seam

### 3.1 The trait, widened

`store.rs`'s `SessionStore` (`store.rs:18`) becomes:

```rust
pub enum Doc { Record, Table }
#[derive(Clone, Copy, PartialEq, Eq)] pub struct Version(u64);   // opaque to callers
pub enum WriteError { Conflict { current: Option<Version> }, Io(String) }

pub trait Store: Send + Sync {
    fn list(&self) -> Result<Vec<GameId>, String>;       // ids with a Table doc
    fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String>;
    fn write_if_version(&self, doc: Doc, id: &str, json: &str,
                        expected: Option<Version>) -> Result<Version, WriteError>;
    fn delete(&self, id: &str) -> Result<(), String>;    // both documents
}
```

- `expected: None` means "create: refuse if present"; `Some(v)` means "replace only if the
  current version is `v`". There is no unconditional write: a caller with no base reads
  first and pins what it read.
- **`FsStore`**: `<id>.json` and `<id>.table.json` in the same directory; `Version` is
  the content digest `Games::persist` computes today (`games.rs:740`), so two processes
  writing the same bytes agree. Check-then-write is under the store's own mutex, atomic
  within this process; across processes it DETECTS and does not PREVENT, exactly as
  `persist` does today and says so (`games.rs:586-600`). `write` stays temp-and-rename.
- **`MemStore`** mirrors it for tests.
- **`list` is the tables.** A record with no table document is not a game this API serves
  (R-P7): it is neither listed nor openable, answers 404, and is left on disk untouched
  for the Docker-archive pull (`docs/handover.md` Track 3).
- A Postgres implementation (M6.5) does `write_if_version` as `UPDATE … WHERE version =
  $1`, which is atomic across processes; nothing in the trait presumes a filesystem.

### 3.2 `Games::persist` re-expressed

The `written: HashMap<GameId, u64>` base map (`games.rs:408`) becomes
`HashMap<GameId, Version>` and the three guards stay, in the same order and with the same
tests: identity by seed first (a name is not a game), then `write_if_version(Record, id,
json, Some(base))`, then the no-base fallback that reads, applies the event-count test,
and pins what it read. A `Conflict` evicts the handle; an `Io` does not (`games.rs:710`).
`create` no longer writes a record — the record is first written at finalize (§4.3); the
table document is written under `create_lock`, which is renamed `table_lock` and also
serializes every keep (§4.3).

---

## 4. Tables and the deal

### 4.1 The table document

`crates/yokohama-web/src/table.rs` (new). Server-side (`TableDoc`, serialized to
`<id>.table.json`):

```json
{
  "schema": 1,
  "id": "sim-0123456789abcdef",
  "name": "Thursday",
  "created_by": "a@example.com",
  "created_at": "2026-09-14T20:11:03Z",
  "players": 2,
  "colors": ["Red", "Blue"],
  "start_seat": 0,
  "seats": [ { "kind": "human", "user": "a@example.com" }, { "kind": "bot" } ],
  "keeps": [ null, null ],
  "seed": 1234567890123456789,
  "status": "dealing"
}
```

- `seats[i]` is `human` with its normalized email, or `bot` (the shipped production
  contestant, `PROVISIONAL_DEFAULT_BOT`, as the CLI seats it; the record's
  `SeatProvenance` carries the controller name once the game exists).
- `keeps[i]` is `1` or `2` once a HUMAN seat has chosen; a bot seat's stays null — the
  bot's keep is computed at finalize by `bot_keep` (`games.rs:169`) and lives in the
  record, never here.
- `seed` is the setup seed, drawn server-side by `draw_seed` or supplied for diagnostics,
  and it never leaves this document: no wire type carries it.
- `status` is `dealing` or `playing`. "Over" is read from the session and never stored.
- `schema` is the document's own shape version, unrelated to `SEMANTICS_REV`.

On the wire (`TableView`), what a seated caller sees:

```json
{ "id": "…", "name": "Thursday", "created_by": "a@example.com",
  "created_at": "…", "players": 2, "colors": ["Red","Blue"], "start_seat": 0,
  "seats": [ { "kind": "human", "user": "a@example.com" }, { "kind": "bot" } ],
  "chosen": [ true, true ],
  "my_seats": [ 0 ],
  "status": "playing",
  "to_move": 0,
  "over": false }
```

`chosen[i]` is true for a bot seat from the start and for a human seat once it has kept.
`to_move` and `over` are present when `playing`, null while `dealing`. Other seats'
emails are on the wire to seated players (they were invited by email); a display name is
M6.2's.

### 4.2 Create

`POST /api/games`, `#[serde(deny_unknown_fields)]`:

```json
{ "players": 2, "colors": ["Red","Blue"], "start_seat": 0, "name": "Thursday",
  "seats": [ { "user": "a@example.com" }, { "bot": true } ],
  "seed": "1234567890123456789" }
```

`keeps` and `bot_seats` are gone; sending either is a 400 by the existing fence. Rules, in
the order a client reads the sentences: players 2–4; colours one per seat, distinct;
`start_seat` in range; `seats` one per seat, each exactly one of `user` / `bot: true`;
every `user` normalized by R-1.1; at least one human; the creator among the humans
(R-1.6); the same email may hold several seats; a bot at three or four players refused
with "the bot is measured at two players only" (R-P10). `seed` optional, a decimal string
as `WireSeed` already parses, for fixtures and diagnostics only.

The reply is **201 with the `TableView`**, status `dealing`, no cards. The table document
is written with `expected: None` under `table_lock`; the record does not exist yet. The
game id keeps the `sim-<16 hex>` shape and is drawn independently of the seed.

**The hot-seat pass retires.** With the deal served per seat (§4.3), a person holding two
seats keeps for the lower one and is then served the next; the `turn` index, the "Next
seat" button and the "cards gone from the DOM after the pass" rule of the corrections'
finding 3 have nothing left to do. This supersedes that finding's mechanism, not its
intent: the intent (no seat sees another's deal) is now the server's.

### 4.3 The deal, per seat

`GET /api/games/{id}/deal` — seated, or 403; 409 `{"error": "this table is playing"}` once
finalized.

```json
{ "table": { …TableView… },
  "seat": 0,
  "cards": [ { "card": {…OrderCard…}, "keep_score": 14 },
             { "card": {…OrderCard…}, "keep_score": 9 } ],
  "keep_note": "  (the Keep score is what the built-in bot would prefer, …",
  "frame": { …HistoricalFrame, technologies and goals withheld… } }
```

- `seat` is the caller's LOWEST seat that has not yet kept; `cards` is that seat's two.
  When every seat the caller holds has kept, `seat` and `cards` are null and the client
  shows "waiting for …".
- The deal is regenerated on each call from the config and seed — `new_game(config,
  rules, seed)` and `pending.deals[seat]` — exactly as `preview` does today
  (`games.rs:324`), with `withhold_until_the_choice` (`games.rs:306`) still applied to
  the frame. The cost is one throwaway setup per call, for seated callers only, which
  closes the handover's "unauthenticated and now expensive" item on this route.
- `keep_note` is `wizard::KEEP_SCORE_NOTE` (`wizard.rs:1384`) made `pub` and served, so
  the client's copy (`NewGame.tsx:340`) is deleted. The one CLI touch of this cycle.

`POST /api/games/{id}/keep` `{ "seat": 0, "card": 1 }` — the caller must hold `seat`
(403 otherwise); `card` is 1 or 2; a seat that has kept is 409. Under `table_lock`: read
the document with its version, set the keep, and

- if any human seat is still unkept: `write_if_version(Table, …, Some(v))`; reply 200
  with the `TableView`;
- else FINALIZE: bot keeps by `bot_keep`; `finalize_setup` with every seat's
  `StartingOrderChoice`; `BotSeats::new(seats, draw_seed())` as `create` does today; the
  `Session` and `Repl` built as `Games::create` builds them now; the record written FIRST
  (`write_if_version(Record, …, None)`), then the table flipped to `playing` and written
  (`Some(v)`), then the live handle inserted. Reply 200 with the `TableView` at `playing`.

**Self-repair.** A table read at `dealing` whose human keeps are all present and whose
record exists (a crash between the two writes) is served as `playing`, and the document is
rewritten on that read. A table at `dealing` with keeps all present and NO record is
finalized on the next `keep` or `deal` call by the same code path. Pinned both ways.

**`bot_keep` is unchanged** and the M5.5 keep-score column is the same
`score_starting_order` (`games.rs:98`).

### 4.4 List and delete

`GET /api/games` → `{ "games": [ …TableView… ] }`: every table where `my_seats` is
non-empty, newest `created_at` first, each with `status`, `to_move` and `over` so the
list can say "your turn" (R-P3). `DELETE /api/games/{id}`: the creator's, 403 otherwise;
removes both documents and evicts the handle.

---

## 5. Per-seat views and the route fences

### 5.1 The viewer

`client.rs` gains

```rust
pub fn viewer_seat(table: &TableDoc, user: &User, state: &GameState) -> Option<PlayerId>
```

— the acting seat if the caller holds it, else the caller's lowest seat, else `None`
(403, R-1.5). Every game route resolves it before touching the `Repl`.
`viewing_seat(repl)` (`client.rs:225`) and `replay_seat(repl)` (`client.rs:268`) are
deleted: the first followed the mover, the second existed to give an UNAUTHENTICATED
replay route one stable seat, and an authenticated route's stable seat is the caller's
own. M4's security sentence for `/at/{k}` ("a sweep over `0..=total` returns that seat's
hands and no other seat's") is kept and is now per caller (§9's sweep cell).

### 5.2 Masking is the mechanism that exists

`ClientState::of(id, repl, viewer, messages)` and `HistoricalFrame::at(…, viewer)` take
the viewer and pass `build_frame` a `Vec<SeatFacts>` the SERVER computes:

```rust
SeatFacts { is_bot: bots.is_bot(seat), hand_hidden: seat != viewer }
```

`Repl::seat_facts()` and its `privacy` flag are no longer consulted by this crate. With
`hand_hidden` true for every seat but the viewer, `build_hand` (`screen_model.rs:2315`)
emits a count and no cards for the others, the starting-order mark never leaves the
viewer's own panel, `build_decision_for` scopes options to the viewer as it already does,
and `prompt_of_seat` is the viewer's. `your_turn` is `current_player == viewer`.

### 5.3 The routes, fenced

| route | requires | built for / fence |
|---|---|---|
| `GET /api/games` | identity | tables where the caller sits |
| `POST /api/games` | identity | creator seated (R-1.6) |
| `GET …/{id}/table` (new) | seated | `TableView` |
| `GET …/{id}/deal`, `POST …/{id}/keep` | seated | §4.3; keep only for a seat the caller holds |
| `GET …/{id}` | seated | viewer; **409 while `dealing`**, naming the status |
| `POST …/{id}/input` | seated | **the caller holds the ACTING seat**, else 403 "it is seat N's turn (their@email)"; grammar as §5.4 |
| `…/ports`, `…/labs`, `…/areas`, `…/log`, `…/index/{kind}`, `…/result` | seated | viewer (the log needs no masking: the CLI's wording names cards at public moments only, and words the one secret as "hidden starting order") |
| `GET …/{id}/at/{k}` | seated | viewer, stable per caller |
| `GET …/{id}/events` (new) | seated | §6 |
| `GET …/{id}/suggest` | seated | acting seat's holder only (R-1.8) |
| `GET …/{id}/why` | seated | viewer's visibility; the existing masking of the roads not taken |
| `DELETE …/{id}` | seated | creator only |
| `POST /api/games/preview` | — | **removed** |

**Every route that needs a session is 409 while dealing** — the state, `ports`, `labs`,
`areas`, `log`, `at/{k}`, `index`, `result`, `input`, `suggest`, `why` — carrying
`{"error": "this table is still dealing", "status": "dealing"}` so a client that opened a
game by id knows which screen to show; `table`, `deal`, `keep`, `events` and `DELETE`
answer at both statuses. Locked sessions keep the M3 rule: projection routes answer,
`suggest` and `why` refuse.

### 5.4 The grammar and undo

- `privacy` leaves `HEADS` (`grammar.rs:56`): every table is bound and each seat sees its
  own hand; `grammar::refusal` words it ("a seated table has no privacy toggle").
- `undo` / `back` (R-P6), checked in the `input` handler BEFORE `Repl::step`, with the
  table in hand:
  - the table has ONE distinct human email (vs-bot, or one person in every human seat):
    admitted, and `Repl::step("undo")` behaves exactly as today;
  - otherwise admitted only when the trailing event's mover is the caller's seat AND
    `state.current_player` is still that seat. The mover of event `len-1` is
    `record::state_at(record, len-1).current_player` — replay-derived, the CLI's own
    attribution. Refused with 403 "undo is not offered once your turn has passed".
  Under the second rule one `Move` (with its bound reveals) pops, since there is no bot
  run to unwind at a bot-free table.
- Every other head is unchanged; the allowlist stays an allowlist.

---

## 6. The events endpoint and the poll (the signal seam's first form)

`GET /api/games/{id}/events?after=N` — seated. Reply, immediately, in a few bytes and
without building a frame:

```json
{ "events": 37, "to_move": 1, "status": "playing", "your_turn": false }
```

- `status` is `dealing` / `playing` / `over`; while dealing `events` is 0 and `to_move`
  null, so the dealing screen polls the same endpoint as the game.
- `after` is the resume cursor (R-P9). This cycle validates it (a non-negative integer)
  and does not act on it: the reply is immediate whether or not `events` moved. M6.3's
  held request and stream answer the same shape with the same cursor. The `IndexQuery`
  fence applies: `#[serde(deny_unknown_fields)]` through `QueryArgs`.
- The handler takes the game's lock only long enough to read three fields.

**The client's poll** (`web/src/signal.ts`, new — the one consumer M6.3 replaces):

- Runs when the tab is VISIBLE and the game is not over and it is not the caller's turn
  (or the table is dealing). Interval 3 s. Never from a hidden tab (R-P8): on
  `visibilitychange` to hidden the timer stops; to visible, the client shows
  "refreshing…", asks once, and resumes.
- On a failed fetch: "reconnecting…", retry at 2, 4, 8, 16, 30 s while visible.
- On a reply: if `status` moved `dealing → playing`, open the game; if `events` is greater
  than the last known, `fetchRun(prev, next)` and the M5.5 walk (`App.tsx:42`, `landed`
  at `App.tsx:137` re-pointed to take a count rather than a reply); if smaller (another
  person's mid-turn undo), reload the state without a walk; if equal, nothing.
- The walk's fetch guard (`TableSections`' `scrubbing`) and `data-busy` are untouched.

---

## 7. The client

### 7.1 Sign-in and the shell

- On load: `GET /api/me`. `claimed` with null user → the sign-in screen: one email
  field, a Sign in button; the value is held in `localStorage` under
  `yokohama.claimed_user` and attached as `X-Yokohama-Claimed-User` by `api.ts`'s `call`
  on every request — only when the mode is `claimed`.
- The shell's strip reads "You are a@example.com", with "sign out" in `claimed` mode only
  (clears the key and returns to sign-in). In the other modes the strip shows the door's
  identity and no sign-out; a null user there renders an error screen naming the header
  the deployment expects.
- A 401 from any call in `claimed` mode returns to the sign-in screen with the sentence.

### 7.2 The games list

The tables from `GET /api/games`: name (or the id), player count, the seats as names
with a bot marked, status, and either "your turn" or "waiting on <who>", newest first.
"Over" tables list with the winner's name. The M5 `GameMenu` is the component that
changes; its "stays open after a game is chosen" item stays on the LEAVE list.

### 7.3 The new-game form

Player count, colours, start seat, name, and ONE ROW PER SEAT: `me` / another email / bot.
Bot is offered at two players only, with the R-P10 sentence beside the choice; at three
and four the option is absent, and the server refuses it anyway. The form posts §4.2's
body and opens the dealing screen on 201.

### 7.4 The dealing screen

The M5.5 set table over `deal.frame`, with the caller's two cards, keep score and the
served `keep_note` under each, and a Keep button per card. The other seats read "chosen"
or "waiting". After Keep: if `/deal` serves another of the caller's seats, its cards
appear (hot-seat, by construction); else the screen waits on the poll (§6) and opens the
game when `status` flips. `NewGame.tsx`'s `turn`, `passing` and "Next seat" retire.

### 7.5 In the game

Nothing about the table, the walk, `TurnAccount`, the explanation panel or the scrubber
changes. What changes is WHO the payload is for and WHEN it refreshes: the decision zone
appears when `your_turn`; otherwise the poll drives the walk. The hot-seat pass between
human seats in one browser is gone because each seat's turn brings its own payload.

### 7.6 Wire types

`web/src/types.ts` gains `Me`, `TableView`, `Seat`, `Deal`, `Events`, `GamesList` and the
new `NewGameRequest`; loses `Preview`. `web/src/wire/contract.ts` type-checks every new
fixture against them under the existing `tsc` gate.

---

## 8. The wire: routes and the corpus

Routes: §5.3. The corpus (`web/src/wire/`, regenerated by `UPDATE_WIRE_FIXTURES=1 cargo
test -p yokohama-web --test wire_fixtures`, run again without the variable, the diff read
and reported in the close):

| change | fixtures |
|---|---|
| removed | `preview.json`, `preview_bot.json` |
| added | `me.json`, `me_signed_out.json`, `games.json`, `table_dealing.json`, `table.json`, `deal.json`, `deal_waiting.json`, `events.json`, `events_dealing.json`, `dealing_409.json`, `not_seated_403.json`, `wrong_seat_403.json`, `undo_refused_403.json` |
| re-cut | every `ClientState` fixture is now built for a NAMED seat under a claimed identity, so `hidden_hand.json` shows the other seat as a count and the viewer's own hand open |

The deterministic bot-free 2P game `wire_fixtures.rs` drives is played by TWO claimed
identities, `red@wire.test` and `blue@wire.test`, alternating with the turn; the
whole-game gate gains a second arm the same way (§9.3). `health.json` gains `identity`.

The seed pin loses its exemption: `no_committed_payload_carries_the_setup_seed` searches
every fixture for the driving seed's decimal and hex forms and finds none, with the
anti-vacuity partner being that the same test asserts the seed IS in the table document
the memory store holds — so the search cannot pass by searching for nothing.

---

## 9. Tests, gates, and the mutations that must be caught

### 9.1 Server (`cargo test -p yokohama-web`)

- **Identity:** each of the three modes resolves its own header from a request carrying
  all three with different values (R-1.2); each is 401 without it; `worker` treats
  `anonymous` as absent; normalization accepts `" A@Example.COM "` as `a@example.com` and
  rejects a missing `@`, two `@`, whitespace inside, a control character, 255 bytes; a
  forged claimed header under `cf-access` changes nothing; `AppState::new()` refuses an
  unset or unknown variable with the sentence.
- **Store:** `write_if_version(None)` refuses a present document; `Some(stale)` conflicts
  and names the current version; the digest of identical bytes from two writers is equal;
  `list` returns table ids and not record-only ids; a record-only id is 404 on every
  route; the persist guards' existing cells pass unchanged through the new seam
  (`a_divergent_write_of_the_same_length_is_refused` among them).
- **Tables and the deal:** every create refusal by sentence, in order; a creator not
  seated is refused; a bot at 3P and at 4P is refused with the R-P10 sentence; 201 carries
  no cards and no seed; `deal` for A carries A's two cards and never B's (by value: the
  card ids equal the test's own `pending.deals[A]` and are disjoint from `deals[B]`);
  `deal` for a caller whose seats have all kept is null-carded; `keep` by a non-holder is
  403; a second keep on a seat is 409; the last keep finalizes, the record exists and
  carries a digest, the table reads `playing`, the bot's keep equals `bot_keep`'s; a
  hot-seat creator is served seat 0 then seat 1; both self-repair arms.
- **Views and fences:** every route in §5.3 is 403 for a signed-in non-seat; `input` as the
  non-acting holder is 403 naming the seat; a payload for A has `hand_hidden` false at A
  and true at every other seat, and no `cards` at any other seat, over EVERY committed
  `ClientState` and `HistoricalFrame` fixture (the cross-seat shape pin); a sweep of
  `0..=total` as A yields A's hands only and as B B's only; `suggest` for the non-acting
  holder is 403; `privacy` is refused by the grammar; undo mid-turn is admitted and
  undoes one event; undo after the turn passed is 403; undo at a vs-bot table unwinds the
  bot run as before (the existing cell, unchanged).
- **Events:** the reply's four fields at dealing, playing and over; `after` validated;
  unknown query keys 400.

### 9.2 Client (`npm run typecheck && npm test`)

Component cells over the new fixtures only: sign-in renders on `me_signed_out.json` and
the header is attached after; the error screen for a null user in `cf-access`; the list's
"your turn" from `games.json`; the form's seat rows and the absent bot option at 3P; the
dealing screen from `deal.json` and `deal_waiting.json`; the poll: advances → the walk
runs, decreases → reload without a walk, hidden → no request, visible again → one
request and "refreshing…", a failure → "reconnecting…" and backoff.

### 9.3 The gates that play

- `crates/yokohama-web/tests/play_a_whole_game.rs` gains
  `two_people_play_to_a_winner`: two claimed identities, each request under its own
  header, seeded-random choices; on every reply the payload's `seat` is the requester's
  and its own hand is the only unmasked one. The vs-bot arm is re-pointed through
  create → keep → play and stays.
- `web/e2e/` gains `two_browsers.spec.ts`: two Playwright contexts sign in as two emails,
  one creates naming the other, both keep, they play alternately to a winner, and after
  every move each context's DOM is searched for the other's card art and text and finds
  none. `plays_a_whole_game.spec.ts` is re-pointed at the new create flow as a vs-bot game.
  The Playwright server is started with `YOKOHAMA_WEB_IDENTITY=claimed`.

### 9.4 Mutations that must be caught (recorded in the close, per R-P11)

1. The extractor reading any of the three headers regardless of mode.
2. `hand_hidden` computed as `false` for a non-viewer human seat.
3. `input` skipping the acting-seat check.
4. The undo rule accepting a trailing event whose mover is the caller when the turn has
   passed.
5. `deal` indexing `pending.deals` by the caller's HIGHEST seat, or serving every deal.
6. `write_if_version(Some(v))` writing on a mismatch.
7. The seed reappearing on any payload under any key (the value search).
8. The e2e DOM search finding nothing because the selector matches nothing (the cell
   must first find the caller's OWN cards by the same selector).

### 9.5 Byte-identical

`git diff master -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest
crates/yokohama-bridge crates/gen-data` is EMPTY at the close; `SEMANTICS_REV` is 7;
`cargo test -p yokohama-cli` passes with its goldens and `guide_sync` untouched; clippy
clean at `-D warnings`.

---

## 10. Deployment

- The image tag is `0.5.0-m6.1` (the workspace version is the owner's to move). The
  container's create body (`RUNBOOK.md` §3) gains `Env: ["YOKOHAMA_WEB_IDENTITY=claimed"]`;
  the recipe by reference (§3.5), the record as a §3.7 addendum with R-1.4's two
  sentences.
- The existing store on the volume keeps its records; none has a table document, so
  `GET /api/games` lists nothing until a table is created (R-P7), and the pull recipe is
  unaffected.
- Verification from the shell (WARP passes Access): `/api/health` shows
  `"identity":"claimed"`; `/api/games` without the header is 401; with it, `[]`.
- The final review is against the live site in BOTH layouts and from two browsers as two
  people (R-P11), including the §0 trap of the M6 handover (a Chrome window that will not
  resize: `innerWidth` is the truth).

---

## 11. What this spec does NOT claim

- No lobby, no join links, no display names, no spectators, no transport beyond the poll,
  no chat, no hosting: M6.2–M6.5.
- No JWT verification. `cf-access` trusts the tunnel's header for the reason §2.4 gives;
  `claimed` is not a security boundary (R-1.4).
- Other seats' emails are visible to seated players. No per-seat privacy of who is at the
  table.
- No rate limiting on `/deal` (one throwaway setup per call; seated callers only).
- Hot-seat secrecy at one browser rests on one person choosing to sign out and in; the
  server cannot tell two seats held by one email apart, by design.
- No size or wall figure for any of this.
- Every count, line number and route name above is at `2533001`; the implementation plan
  re-reads them at its own HEAD.

---

## 12. Read order for the implementer

1. `CLAUDE.md` — the Web (server) and Web (browser half) blocks, and the
   `crates/yokohama-web` + `web/` architecture bullet.
2. `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0 and §1.
3. This document.
4. `docs/plans/2026-09-06-web-frontend-m5.5-owner-corrections.md` §1 and §5 — the deal
   flow this cycle replaces, and why its intent survives.
5. `docs/plans/2026-09-01-web-frontend-m4-spec.md` §5 and §8 — the replay pin's argument,
   which §5.1 retires with reasons.
6. `deploy/RUNBOOK.md` §0, §3, §3.5.
