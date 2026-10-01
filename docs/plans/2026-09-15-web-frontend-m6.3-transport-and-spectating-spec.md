# Web frontend — M6.3 "transport and spectating": spec

**Written 2026-09-15 on branch `web-room-m6.2` at `48ae908`, beside the M6.2 spec
(`docs/plans/2026-09-15-web-frontend-m6.2-lobby-spec.md`), from the same brainstorm, and
corrected the same day by an adversarial read against HEAD (twenty-six findings, all
folded in).** The plan it executes the third cycle of is
`docs/plans/2026-09-14-web-frontend-m6-room-plan.md` (§0 the rulings R-P1–R-P11, §1 the
three seams, §2 this cycle's charter). **It is executed AFTER M6.2 ships**, on the seams
M6.2 leaves and names in its §5.5 (one poll hook, four consumers, the `Source` shape
fixed there so this cycle keeps the signature), §3.4 (the open-table read for a
non-seated caller), §4 (the password) and §3.5 (the per-table locks). Every "at HEAD"
fact below is at `48ae908`; the implementation plan re-reads them at M6.2's close HEAD
and records what moved (the `games.rs` cut above all).

The name is what the cycle delivers: a browser learns that something changed without
asking, and a person who is not playing can watch a game walk.

---

## 0. The problem, and the approach

### 0.1 What is true at HEAD

- **The signal is a poll.** `web/src/signal.ts`'s `useSignal` asks `GET …/{id}/events`
  every 3 s from a visible tab, never from a hidden one, once on becoming visible, with a
  backoff ladder on failure (R-P8; `VISIBLE_MS`, `BACKOFF_MS`, whose in-file reason is
  that a poll "that never backed off would hammer it hardest exactly while it was least
  able to answer", `signal.ts:25-31`). The route builds no frame
  (`crates/yokohama-web/src/lib.rs:1274`, `events`): four fields — `events`, `to_move`,
  `status` (`dealing` | `playing` | `over`), `your_turn`. `EventsQuery.after` is
  validated and not acted on (M6.1 spec §6).
- **The moment of change is known and unannounced.** Every write site in `games.rs` at
  HEAD: `create` (`:645`), `finalize`'s record write (`:752`), `flip_to_playing`
  (`:788`), `keep` (`:883`), `persist`'s record write (`:1219`) and `mirror_turn`
  (`:1251`, called at `:1231`), `delete` (`:1313`). `deal` is read-only. Nothing waits
  on any of them.
- **`AppState` is `rules`, `games`, `identity`** (`lib.rs:212`); tokio is built with
  `rt-multi-thread`, `macros`, `net`, `signal` and not `sync`
  (`crates/yokohama-web/Cargo.toml:66`); no stream crate is a dependency. axum 0.8
  exposes `response::sse` (`Sse`, `Event`, `KeepAlive`) ungated and unused here.
- **The viewer is a `PlayerId` at every layer**: `client::viewer_seat` (`client.rs:224`),
  `facts_for(repl, viewer)` (`client.rs:243`, `hand_hidden: p.id != viewer`),
  `ClientState::of(id, repl, viewer, messages)` with `ClientState.seat: PlayerId`
  (`client.rs:140`), `HistoricalFrame::at(repl, k, viewer)` with `seat: PlayerId`
  (`:473`), and `prompt_of_seat(repl, seat)` (`:276-283`), which answers
  `Watching { to_move }` to a seat that is not acting when the game is neither over nor
  locked. `seated()` (`lib.rs:375`) is the gate on nine session routes; `events` is the
  tenth and reads the table and the handle itself so it can ANSWER a dealing table
  (`lib.rs:366-373`). A caller with no seat is 403 (R-1.5; M6.2's R-2.7 widens `table`
  and `join` at `open` only).
- **The frame is built for a seat by the CLI's own builder.** `screen_model::build_frame(
  state, rules, seats, seat, legal, title)` (`crates/yokohama-cli/src/screen_model.rs:2698`)
  uses `seat` in exactly two places: `build_dashboard` draws that seat's OWN hand panel
  and nothing else from it (`build_hand(state, rules, seat, SeatFacts::for_seat(seats, seat).hand_hidden)`,
  pushed at `:2686-2689`) — and a HIDDEN hand still yields a panel,
  `Some(HandPanel { hidden: true, cards: [], card_count })` (`:2315-2338`; the
  committed `watching.json` carries one for a non-acting seat) —
  and `build_decision_for` (`:2515`) returns the full option list only when `seat` IS
  `current_player`, otherwise `viewer_is_deciding: false` over an empty list.
  `DecisionModel.seat` is the DECIDING seat, set to `state.current_player`
  (`:2478`), not the viewer.
- **`why` masks for every caller already**: `Why::granted` hardcodes
  `HandVisibility::Masked` (`explain.rs:410`); the handler passes nothing
  (`lib.rs:1037` is a comment naming the constant), so an unplayed `FulfillOrder` words as
  "Fulfill an order from hand" for anyone.
- **The card index marks the caller's own hand**: `Repl::index_placements_for(me)`
  (`crates/yokohama-cli/src/repl.rs:1745`) pushes `CardLocation::OwnHand` for `me`'s
  cards unless the REPL's own seat display hides that hand; `index_placements()`
  (`:1729`) delegates with `current_player`; the web passes the viewer (`card_index`).
- **The client reads `state.seat` as a number in one place**: `Decision.tsx:397`,
  a template literal (`Seat ${state.seat} — ${phase}.`) that `tsc` accepts with `null`
  under `strict` — and it sits in `asking()`'s `default:` arm, which a `watching`
  prompt never reaches: `asking()` switches on `state.prompt.kind` (`:376`) and the
  `case "watching"` arm at `:390` returns `Seat ${state.prompt.to_move} is deciding.` `landed` (`App.tsx:197`) already takes `watching: PlayerId | null`,
  where `null` means "this run did not arrive through the signal" and selects
  `hasBotStep` (`:207`); `hasOpponentStep(steps, viewer: PlayerId)` is `walk.ts:89`.
  `TurnAccount` reads movers, not the viewer; `Dashboard.tsx:554` reads `hand.seat` off
  the frame.
- **The browser's `EventSource` cannot send this server's identity header.** Every
  request carries it from `api.ts:110-117` (`call`), and every mode's extractor reads
  exactly one header and refuses without it (`identity.rs:127-136`). `claimed` is the
  deployed mode. `EventSource` takes a URL and `withCredentials` and nothing else.
- **The edge closes an idle socket at 100 s** (`deploy/cloudflared-config.yml:52`'s own
  comment; room plan §2); the tunnel's origin settings are `connectTimeout: 10s`,
  `keepAliveTimeout: 90s` (`:30-31`); the undeployed Worker returns the origin's body as
  a stream (`deploy/worker/src/index.ts:167`, `new Response(response.body, …)`).
- **The two-browser browser cell costs 11.8 of the walk's 13.0 minutes**, most of it the
  3 s poll (M6.1 close §2, §6.2, read at `44dd538`).
- **The whole-game gate is out of CI** since `48ae908` (`.github/workflows/test.yml:46`);
  per-edit locally.

### 0.2 The approach

**Chosen: server-sent events over an in-process channel, one stream per table and one
for the house, read by the client through `fetch`, with the choice held to a
measurement.** `Games` gains a `watch` channel per table id, bumped by every table write
and by `persist` after the record lands, and a house channel bumped by every table write
anywhere. `GET …/{id}/stream` writes the existing `Events` object on connect and on every
bump with a comment heartbeat every 25 s; `GET /api/stream` writes a sequence number on
every house bump. The client's one hook keeps M6.2's signature and its four consumers,
and changes its transport once: it opens the stream with the same `fetch` and the same
identity header every other call uses, reads the body as a stream, and parses the frames
itself — because `EventSource` cannot carry the header (§0.1), and because the held
request, if R-3.1 falls back to it, is the same `fetch` reading one frame and returning.

**Why SSE and not the held request** (the two the room plan named for the origin): one
connection per tab instead of a request per change, and the resume shape R-P3 asked for
is the `events` count that every frame already carries, so a reconnect is a compare and
not a protocol. The held request costs a request per change and the same heartbeat. What
SSE does NOT buy here, and an earlier draft claimed: the browser's own reconnection —
that belongs to `EventSource`, which cannot be used, so reconnection is the hook's, with
the ladder the poll already has. **The decision is held to the tunnel** (R-3.1): if a
heartbeated stream does not survive ten minutes through it, the held request ships
instead, on the same channel, behind the same hook.

**Rejected — WebSockets at the origin.** The tunnel carries them, but the origin gains a
second protocol for a message that flows one way, and the hosted shape puts sockets in a
Durable Object anyway (§7); an origin socket would be built to be thrown away.

**Rejected — polling with longer backoff.** The exit criterion is "no polling on the
NAS", and a slower poll is a poll.

### 0.3 The consequence worth stating first

A caller who is not seated can now read a PLAYING game. That is the second widening of
R-1.5, and with `table` opening to any signed-in caller at every status but `dealing`
(R-3.13) it is the last: the spectator sees what the M5.5 walk shows a player whose turn
it is not — every hand a count, the public board, the log, the replay, the bot's account
— and can send nothing. The password M6.2 put on the table is the one gate, presented
once.

---

## 1. Rulings specific to this cycle (2026-09-15)

Beyond the plan's R-P1–R-P11 and M6.2's R-2.1–R-2.11:

- **R-3.1 — SSE, held to a measurement.** The plan's first task holds
  `GET …/{id}/stream` open through the tunnel from the workstation for ten minutes with
  the 25 s heartbeat, makes one move at minute nine and records whether the frame
  arrives; then the same without the heartbeat, recording when the edge drops it
  (expected near 100 s). Both readings go in the close. **If the heartbeated stream
  drops, the held request ships** (`GET …/{id}/events?wait=25` answering on the first
  bump or after 25 s), on the same channel, and §5's hook changes its reader and nothing
  else.
- **R-3.2 — no polling.** The interval leaves `usePolled`. In the SSE arm the hook never
  calls `GET …/events` at all: the stream's on-connect frame (§3.1) IS the ask on
  becoming visible and IS the compare after a reconnect. Proven in a browser: sixty idle
  seconds with a connected, visible tab, **zero requests to `/events` and exactly one
  open request to `/stream`** in the network log (art and frame fetches are not the
  claim).
- **R-3.3 — the stream is per TABLE, not per game.** `…/{id}/stream` answers at `open`,
  `dealing`, `playing` and over, so the open-table screen and the dealing screen use it;
  `Events.status` gains `"open"`. A bump at a table with no game behind it carries
  `events: 0`.
- **R-3.4 — the house stream carries a sequence number only.** A `watch` channel
  coalesces, and two bumps naming different tables inside one wake would lose one; the
  lobby therefore refetches ALL its lists on any bump, which is three table-document
  listings and no session.
- **R-3.5 — who may watch.** Any signed-in caller not seated, at a table that is
  `playing` or over, when the table has no lock or the caller has presented the
  password once through `POST …/{id}/unlock`, which writes the caller's email onto the
  table document's `unlocked` list. At `dealing`, not seated stays 403: nobody watches a
  deal. A LOCKED table's lock gates joining and watching, never seeing that it exists:
  `table` and `stream` at an open locked table answer a stranger as M6.2 R-2.7 says.
- **R-3.6 — what a spectator gets.** Every read route: the frame, `ports`, `labs`,
  `areas`, `log`, `events`, `stream`, `at/{k}`, `index/{kind}`, `result`, `why`.
  `input`, `keep`, `deal`, `join`, `suggest` and `DELETE` refuse with the sentences a
  non-seated caller gets today, BEFORE the handle is touched. `undo` is a line of
  `input`.
- **R-3.7 — the spectator's payload.** `seat` is `null` on `ClientState` and
  `HistoricalFrame`; `your_turn` is false; the prompt is `watching` (or `game_over` /
  `locked` when the game is); every seat's `hand_hidden` is true; no options. The frame
  is a NON-ACTING seat's frame with every hand hidden (§4), which is the definition, not
  a stand-in.
- **R-3.8 — one CLI touch, behaviour preserved.** `Repl::index_placements_for` takes
  `Option<PlayerId>`; `index_placements()` delegates with `Some(current_player)` as it
  delegates today; `None` pushes no `OwnHand` row. The CLI's goldens and `guide_sync`
  are the proof it moved nothing. `build_frame` is NOT touched (§4 says why it need not
  be).
- **R-3.9 — the lobby gains a third scope**, `playing` (status `playing` or over, any
  creator, newest first), drawn as "Games in progress", so a watcher can find a game
  without a link. A row opens `#/t/<id>`, which lands a non-seated caller on the
  spectator screen or the unlock face.
- **R-3.10 — the notifier is designed and not built** (§7). M6.5 builds it.
- **R-3.11 — dependencies.** tokio gains the `sync` feature;
  `tokio-stream = { version = "0.1", features = ["sync"] }` enters for `WatchStream`.
  Both declared in the Cargo.toml dependency paragraph as this cycle's owner-ruled
  additions, the way M6.2 declares `sha2`.
- **R-3.12 — the heartbeat is 25 s**, under the edge's 100 s and the tunnel's 90 s
  keep-alive, sent by axum's `KeepAlive` as a comment line, so no client parses it. It
  is a field of `AppState` (`heartbeat: Duration`, default 25 s) so a test can construct
  a state with 50 ms and READ two heartbeats off the wire; nothing on a built `Sse` is
  inspectable.
- **R-3.13 — `GET …/{id}/table` answers any signed-in caller at `open`, `playing` and
  over.** M6.2 widened it at `open`; this widens it at `playing` and over, because a
  would-be watcher must read `locked` and `can_watch` to know whether to ask for a
  password, and the view carries the public facts the lobby's `playing` scope already
  lists. At `dealing` a stranger is 403 (R-3.5). M6.2's R-2.7 sentence "and `table` at
  every other status keeps R-1.5's 403" is superseded here, and §0.3 counts it.
- **R-3.14 — `Last-Event-ID` is ignored and `after` stays inert.** The resume cursor
  R-P3 asked for is the `events` count every frame carries; the client compares it with
  its own and fetches the frame when it moved. The server writes no `id:` line. M6.1
  spec §6's sentence "the held request and stream answer the same shape with the same
  cursor" is met by the frame shape, and `EventsQuery.after` remains validated-and-inert,
  listed in §11.

---

## 2. The channels — `crates/yokohama-web/src/signal.rs` (new)

```rust
pub struct Bump { pub seq: u64, pub end: bool }        // per table; `end` once, on delete

pub struct Signals {
    tables: Mutex<HashMap<GameId, watch::Sender<Bump>>>,
    house: watch::Sender<u64>,
}
impl Signals {
    pub fn bump_table(&self, id: &str);      // the table's channel only; creates it if absent
    pub fn bump(&self, id: &str);            // the table's channel AND the house
    pub fn end(&self, id: &str);             // sends Bump { end: true }, REMOVES the sender, bumps the house
    pub fn subscribe(&self, id: &str) -> watch::Receiver<Bump>;   // creates the sender if absent
    pub fn subscribe_house(&self) -> watch::Receiver<u64>;
}
```

`Games` owns a `Signals` and bumps at exactly these points, each named in a cell, and
they are every write site of §0.1: `create` → `bump`; `join` → `bump` (a seat filled, or
the table left the open list); `keep` → `bump_table` (nothing a lobby shows changed; the
dealing screen's "waiting on" line must refresh); `finalize` and `flip_to_playing` →
`bump`; `persist`, after the record write and `mirror_turn` → `bump` (the lobby's
whose-turn and over lines read the mirror); `delete` → `end`. **A bump is sent AFTER the
write lands, never before** — a subscriber that wakes and reads must read the new state
— and the cell for each site parks the write behind a gate and asserts the receiver does
not wake until the gate opens.

A `watch` channel coalesces: a subscriber that missed three bumps sees one and reads the
latest state, which is the property wanted PER TABLE and the reason the house channel
carries no per-table payload (R-3.4). `subscribe` on an id with no sender creates one, so
a subscriber of a table that has never been written still wakes on its first write; the
route in front of it has already answered 404 for a table that does not exist, so a
resurrected entry for a deleted id receives nothing and is harmless. `end` sends the
terminal bump and removes the sender, so every open stream on that table sees `end` and
closes; a stream opened after that finds no table and is 404 at the route. The map is
not pruned otherwise: one entry per table ever streamed, bounded by the table count
(§11).

`WatchStream::new` yields the channel's CURRENT value on creation, which is exactly the
on-connect frame the table stream wants; the house stream, which must not fire on
connect, is built with `WatchStream::from_changes`.

---

## 3. The routes

### 3.1 `GET /api/games/{id}/stream`

`text/event-stream`. **Admitted exactly as `GET …/{id}/events` is** — the route that
already answers a seat holder at every status — extended by §3.5 to a stranger at `open`
and to a watcher at `playing`/over; the frame route `GET …/{id}` keeps its 409 at `open`
and `dealing` and is NOT the comparator. On connect it writes one frame at once — the
current `Events` — and then one per bump; a comment heartbeat every `AppState.heartbeat`
(R-3.12). Each frame is `event: signal` followed by `data: <Events JSON on one line>`;
the `Events` object is the `events` route's, built by the same function, with `status`
gaining `"open"`. On a table's `end` the stream writes `event: end` with
`data: {"reason":"deleted"}` and closes. No `id:` line and no `retry:` line (R-3.14).
The handler holds no game lock across an await: it locks, builds the object, unlocks,
writes.

### 3.2 `GET /api/stream`

`text/event-stream`, identity required, no other admission. One frame per house bump,
`event: house`, `data: {"seq": N}`, the same heartbeat, no frame on connect
(`from_changes`).

### 3.3 `GET …/{id}/events`

Unchanged in SHAPE. Changed in two ways: it gains an `open` arm before `Games::handle`
(the same place its `dealing` arm sits today, `lib.rs:1280-1282`), answering
`events: 0, status: "open"` for a table with no record, and its admission widens per
§3.5 like every read route's. Under R-3.1's
fallback it gains `?wait=<seconds>` (≤ 25): the handler subscribes, answers on the first
bump or at the deadline. Absent the fallback the parameter does not exist and is a 400
like any unknown key.

### 3.4 `POST …/{id}/unlock`

```json
{ "password": "sesame" }
```

Identity required. 404 no such game; **409** at `open` and `dealing` through M6.2
§3.4's status arms (`GameError::Open` / `Dealing`, their sentences and `status` beside
them — there is nothing to watch yet, and a joiner sends the password to `join`); at a table with no lock, 200 and no
write; **403 "wrong password for this table"**; on success the caller's email is
appended to `TableDoc.unlocked` under the table's lock (deduplicated; capped at 256
entries, the 257th a 409 "this table has as many watchers as it can remember"). Answers
200 with the `TableView`, whose `can_watch` now reads true. A seat holder unlocking their
own table is a 200 no-op.

### 3.5 The admission function

`seated()` becomes `admitted(state, id, user) -> (TableDoc, Viewer)`, which reads the
table FIRST as today and returns NO handle: a route that needs the handle asks
`Games::handle` after, which is what `events` does at HEAD and what lets `stream` and
`events` answer a dealing table. Every route names its row:

| caller | status | `table` | `events`, `stream` | the other reads (§R-3.6) | the writes |
|---|---|---|---|---|---|
| holds a seat | any | 200 | 200 | 200 at `playing`/over; 409 `open`/`dealing` | as today |
| no seat | `open` | 200 (M6.2) | 200 | 403 not seated | `join` as M6.2; the rest 403 |
| no seat | `dealing` | 403 not seated | 403 | 403 | 403 |
| no seat; unlocked, or on `unlocked` | `playing` / over | 200, `can_watch: true` | 200 as `Spectator` | 200 as `Spectator` | 403 not seated |
| no seat; locked, not unlocked | `playing` / over | 200, `can_watch: false` | 403 "this table is locked — unlock it first" | 403 same | 403 not seated |

Every read route reads its `Viewer`; every write route refuses a `Spectator` with the
non-seated sentence before touching the handle. **The cell for this section runs the
whole table**: every row against every route, and asserts that `stream` and `events`
answer identically in every cell.

---

## 4. The spectator's projection

`client.rs`:

```rust
pub enum Viewer { Seat(PlayerId), Spectator }
pub fn facts_for(repl: &Repl, viewer: Viewer) -> Vec<SeatFacts>   // Spectator: every hand_hidden true
impl ClientState { pub fn of(id, repl, viewer: Viewer, messages) -> Self }   // seat: Option<PlayerId>
impl HistoricalFrame { pub fn at(repl, k, viewer: Viewer) -> … }              // seat: Option<PlayerId>
fn prompt_of_viewer(repl, viewer) -> PromptState   // Spectator: Watching{to_move} unless over/locked
```

**The frame.** `build_frame` is the CLI's and takes a `PlayerId`; it is not widened
(R-3.8). A spectator's frame is built with `seat = the first seat that is not
current_player` — at two, three and four players there is always one — and
`facts_for(Spectator)`, which hides every hand. That is the frame a non-acting player
sees minus their own hand, and §0.1 is why nothing leaks through the choice of seat:
`build_dashboard` uses `seat` only to draw its OWN hand panel, which the facts hide, and
`build_decision_for` returns `viewer_is_deciding: false` over an empty option list for
any seat that is not acting. `DecisionModel.seat` is the deciding seat by construction
and stays a `PlayerId` on the wire. **One thing the facts do NOT remove**: a hidden hand
still yields a `Hand` panel — `hidden: true`, no cards, a card count — for the stand-in
seat (§0.1), which on a spectator's screen would single out one seat as "yours". So the
projection DROPS the `Panel::Hand` from a spectator's dashboard in `ClientState::of` and
`HistoricalFrame::at`, after the builder returns it hidden; the other panels are
untouched. The cells (§9.1) pin four consequences by value, before and after the drop:
the builder's panel for the stand-in is hidden with no cards, the projected dashboard
has no `Hand` panel at all, `viewer_is_deciding` false, `options` empty — at 2P, 3P and
4P.

`ClientState.seat` and `HistoricalFrame.seat` become `Option<PlayerId>`; on the wire a
seated payload is byte-identical to today (`Some(n)` serializes as `n`) and the corpus
proves it. `your_turn` is `false` for a spectator by construction. `card_index` passes
`None` for a spectator (R-3.8). `why` is unchanged (§0.1). `suggest` refuses (R-3.6).
`result` answers: the final standings are public.

`TableDoc` gains `#[serde(default)] unlocked: Vec<String>` (schema stays 2 — an additive
default, and M6.2's schema-1 read cell extends to it); `check_shape` bounds it at 256
and requires each entry to parse as a `User`. `TableView` gains `can_watch: bool` — true
for a seat holder, for anyone at an unlocked table, and for a caller on `unlocked`;
`unlocked` itself is never serialized (a by-key cell), and §8's value pin searches every
payload built for someone else for a watcher's email.

---

## 5. The client

### 5.1 The hook re-plumbed — `signal.ts`

`usePolled<T>(source: Source<T> | null, active, onValue, onEnd?)` keeps M6.2's
signature (`Source<T> = { key, once, stream }`) and its rules, and changes what runs
between them. **The reader is a `fetch`**, made through the same path as every other
call so it carries the identity header, with `Accept: text/event-stream`; the body is
read through `ReadableStream` and a small line parser (`event:`, `data:`, blank-line
frame boundaries; a leading `:` is a heartbeat and is dropped). `EventSource` is not
used (§0.1).

- **Connected**: each `signal` frame's `data` is parsed and handed to `onValue`. `Link`
  reads `"live"`. No timer is armed.
- **Hidden**: the reader is aborted (`AbortController`). **Visible again**: `Link` reads
  `"refreshing"` and a new reader is opened; its on-connect frame is the R-P8 ask.
- **Failure** (the fetch rejects, the response is not 200, or the body ends without an
  `end` frame): `Link` reads `"reconnecting"`, the ladder `BACKOFF_MS`/`nextDelay`
  schedules the next open — the ladder stays, for the reason it was written (§0.1) —
  and the reopened stream's on-connect frame is the compare.
- **Ended** (an `end` frame): the reader closes and `onEnd` fires; the consumer
  navigates to the lobby with a sentence. M6.1 close §6.2's "a DELETED game reads
  reconnecting forever" closes here for the stream, and its one-shot half closes with it:
  a 404 from the stream's open is `onEnd` with reason `gone`, and a 403 is `onEnd` with
  reason `refused` and the sentence — never a "reconnecting" loop. The consumer decides:
  the game screen leaves for the lobby with the sentence on either; the arriving-watcher
  flow (§5.3) never opens the stream before `table` has answered `can_watch: true`, so a
  locked table's 403 reaches the unlock face through `table`, not through the stream.

Under R-3.1's held-request fallback the reader is the same `fetch` to
`…/events?wait=25`, one frame per request, reopened at once on a 200 and by the ladder
on a failure — one hook, two readers, selected by `/api/health`'s `signal` field
(§7 names it).

The hook's cells (`signal.test.ts`) are rewritten against a fake `fetch` that yields a
scripted stream, one cell per rule, and the ONE cell that must fail under the old
transport is "no timer is armed while connected" (a fake `setTimeout` that throws when
`Link` is `"live"`).

Instances: the game (`App.tsx:425`), the dealing screen (`Dealing.tsx:192`), the open
table and the lobby (M6.2 §5.5). The lobby's `Source` streams `/api/stream` and its
`onValue` refetches all its lists (R-3.4).

### 5.2 The spectator screen

The game screen with the viewer absent. `state.seat === null` selects: no answer bar
(`Decision` renders the `watching` prompt's line, `Decision.tsx:132`); the "You are"
strip reads "You are watching"; the Game menu has no Undo and no Suggest; the replay
scrubber, the log, the boards menu, `why` and the result all work as they do for a
player whose turn it is not. `ClientState.seat` and `HistoricalFrame.seat` become
`PlayerId | null` in `types.ts`. **The typecheck is NOT what finds the readers**: the one
reader at HEAD is a template literal (`Decision.tsx:397`) that `tsc` accepts with `null`.
The plan greps `\.seat\b` over `web/src/` at its HEAD and lists every reader of the two
nullable fields. A spectator's `Decision` line is the `watching` arm's, `:390`
("Seat N is deciding."), unchanged; the `:397` template is unreachable for a `watching`
prompt and is left as it is; a cell pins that a spectator renders `:390`'s line and the
"You are watching" strip.

**The walk.** `landed`'s fourth parameter conflates two facts today: `null` means "not
via the signal" and selects the bot gate (§0.1). It becomes an arrival:
`{ via: "reply" } | { via: "signal"; viewer: PlayerId | null }`, and `hasOpponentStep`
takes `PlayerId | null`, where `null` makes every step an opponent's. A spectator's run
always arrives via the signal with `viewer: null`, so every served run walks at the
remembered cadence. The cell drives the APP's `landed` path over a spectator run and
asserts the walk starts — the predicate alone would pass while the call site took the
wrong branch.

### 5.3 Arriving as a watcher

`#/t/<id>` for a non-seated caller at a playing or finished table: `GET …/{id}/table`
answers (R-3.13) and `can_watch` says which face to draw. If `can_watch`, the spectator
screen; if not, an **Unlock** face — the table's name, its seats, one password field, a
button — posting `unlock` and then opening the game. A row under the lobby's **Games in
progress** (R-3.9) does the same.

### 5.4 Wire types

`Events.status` gains `"open"`; `TableView` gains `can_watch`; `ClientState.seat` and
`HistoricalFrame.seat` are nullable; `UnlockRequest` is new; `GamesList`'s scope union
gains `"playing"`; `Health` gains `signal`. `contract.ts` type-checks the lot.

---

## 6. Fences

- **A spectator is handed no hand.** `facts_for(Spectator)` hides every seat; the
  cross-seat shape pin from M6.1 (every committed `ClientState` and `HistoricalFrame`
  fixture) gains the spectator fixtures and asserts `hand_hidden` true at EVERY seat,
  `cards` absent at every seat, and no `Hand` panel in the projected dashboard (§4).
- **A spectator writes nothing.** The write routes refuse before the handle is touched;
  a sweep cell posts every write route as a spectator at a playing and at a finished
  table.
- **A lock holds until unlocked, and only for the one who unlocked.** Two watchers, one
  unlock: the other is still 403.
- **`stream` and `events` are admitted identically** — §3.5's whole-table cell.
- **A bump follows every write and precedes none** — §2's gated cells, one per site.
- **The seed pin, the password pin and the acting-seat fence are untouched**; the
  watcher's-email pin is added beside them.

---

## 7. The notifier, designed only (for M6.5)

On the hosted path a container must idle between moves, so nothing may hold a request at
the origin. The design, to be built in M6.5 and verified there:

- **One Durable Object per table id** (on Cloudflare Containers, the object that already
  fronts the container). It holds hibernating WebSockets, one per tab.
- **The origin pokes it** on every bump: `POST https://<worker>/notify/{id}` with the
  same `Events` JSON the stream writes, under a shared secret header the Worker checks
  (`YOKOHAMA_WEB_NOTIFY_SECRET`, set in both places, no default — R-1.3's shape). The
  house bump pokes `/notify/house` with `{"seq": N}`. A poke that fails is logged and
  dropped: the client's on-connect compare (§5.1) is the recovery, exactly as on SSE.
- **The object fans the message out** to every socket and hibernates. The client's
  `usePolled` gains a third reader, `ws`, chosen by `/api/health`'s `signal` field —
  `"sse" | "held" | "ws"`, added in THIS cycle with its first two values, the way
  `identity` names the door.
- **What is decided here and not revisited in M6.5**: the message shape, the poke's
  authentication, the drop-and-recover rule, and that the client selects the reader from
  health. **What is M6.5's**: the object's code, its binding, the Worker route, and the
  measurement that a container scales to zero with a tab open.

---

## 8. The wire: routes and the corpus

Routes: M6.2's twenty-one plus `GET …/{id}/stream`, `GET /api/stream`,
`POST …/{id}/unlock`. The two streams are not JSON routes and are not corpus files;
`wire_fixtures` reads ONE frame from `…/{id}/stream` at a playing table, parses its
`data`, and asserts the VALUE equals the committed `events.json` parsed (the file is
pretty-printed and a `data:` line cannot hold a newline, so bytes cannot be compared),
which pins the stream to the route it mirrors.

Regenerated, the diff read: every `ClientState` and `HistoricalFrame` fixture is
byte-identical (the `Option` serializes the same); every `TableView` fixture gains
`can_watch`; `health.json` gains `signal`. New: `spectator.json` (a `ClientState` for a
watcher at a playing table: `seat: null`, every hand hidden, prompt `watching`, no `Hand`
panel), `spectator_at_k.json`, `spectator_over.json` (prompt `game_over`, `scores`
present), `games_playing.json` (`scope=playing`), `table_watchable.json` (a locked
playing table read by a stranger, `can_watch: false`), `unlock_403.json`,
`unlock_409_open.json`, `watch_403_locked.json`, `events_open.json`. The value
pins: the seed, the password and its hash (M6.2's), and now a watcher's email on any
payload not built for that watcher (the walk's third identity unlocks a locked table,
then every payload built for red and blue is searched for green's email).

---

## 9. Tests, gates, and the mutations that must be caught

### 9.1 Server (`cargo test -p yokohama-web`)

- **Signals**: every bump site (§2) with its receiver-wakes-after-the-write cell;
  coalescing per table (three writes, one wake, latest state); `end` closes an open
  stream and removes the sender; a subscribe before the first write wakes on it; the
  house channel bumps on every site but `keep`.
- **Stream**: the on-connect frame's value equals `events`'s reply; a bump produces a
  frame whose `events` count moved; the `end` frame on delete; the whole-table
  admission cell (§3.5) asserting `stream` ≡ `events`; **the heartbeat, read off the
  wire**: a state built with `heartbeat: 50 ms`, a stream held for 200 ms, at least two
  comment lines in the body (R-3.12); no `id:` and no `retry:` line in any frame.
- **Unlock**: the seven answers (§3.4); dedup; the cap; a seat holder's no-op; `unlocked`
  absent from every `TableView` (by key).
- **Spectator**: `facts_for(Spectator)` all hidden; the frame's three consequences by
  value at 2P, 3P and 4P (§4); `ClientState::of` and `at` for a spectator carry
  `seat: null`, `your_turn: false`, the watching prompt, no options; over → `game_over`
  with scores; every read route 200 and every write route 403 as a spectator (the
  sweep); at `dealing` 403; the two-watcher lock cell; `card_index` as a spectator has
  no `OwnHand` row (by value, over a game where the acting seat holds cards); `why` as a
  spectator equals `why` as the non-acting player (the same masked account).
- **Schema**: M6.2's schema-1 read cell extended to `unlocked: []`.

### 9.2 CLI (`cargo test -p yokohama-cli`)

Untouched goldens and `guide_sync`; one new lib cell that `index_placements_for(None)`
carries no `OwnHand` row over a state where `current_player` holds cards, while
`index_placements()` over the same state does. (The delegation itself is not asserted
against itself.)

### 9.3 Client (`npm run typecheck && npm test`)

The hook's rule cells against a scripted fake `fetch` (§5.1), including "no timer while
connected", the `end` frame, and the header on the stream request; the spectator screen
over `spectator.json` (no answer bar, "You are watching", no Undo/Suggest in the menu,
the scrubber present, the `Decision` line); the walk over a spectator run driven through
`landed` (every step walks); the unlock face over `table_watchable.json` and
`unlock_403.json`; the lobby's third section over `games_playing.json`; the lobby
refetching all lists on a house frame.

### 9.4 The gates that play

- `play_a_whole_game.rs`: the two-identity arm gains a **third identity that watches**,
  `green@wire.test`. M6.2's re-point creates that arm's table WITH a password, so green
  unlocks BEFORE its first read (a 403 first, then `unlock`, then 200); from the deal
  onward green reads `GET …/{id}` and `at/{k}` at every position both players see, and
  every one of its frames has every hand hidden and no `Hand` panel, and `frame.board`,
  `title` and the players' dashboards with their `Hand` panels removed are equal to
  green's at the same `k` (`decision` differs by construction and is excluded). The gate stays out of CI,
  per-edit locally.
- `web/e2e/two_browsers.spec.ts` gains a third context that opens `#/t/<id>` as a
  stranger, unlocks, and after every move by either player is asserted to (a) show the
  new move's account row — read by its EVENT INDEX, never by presence — within the
  cadence, (b) hold no card art or text of either hand, (c) have issued **no request to
  `/events` and exactly one open request to `/stream`** during the wait, with the tab
  asserted visible; a reconnect during the wait (a second `/stream` request) fails the
  cell and is reported as a finding, since under R-3.1's measurement it should not
  happen. The cell's own wall is RECORDED in the close against the 11.8-minute reading
  at `44dd538`.
- `web/e2e/joins_by_link.spec.ts` (M6.2's) is re-pointed by nothing: it asserts B's name
  appears on A's screen without a reload, which is now the stream's job.

### 9.5 Mutations that must be caught (recorded in the close, per R-P11)

1. `facts_for(Spectator)` computing `hand_hidden` as false for any seat.
2. `input` (or any write route) admitting a `Spectator`.
3. A locked table's reads served to a non-seated caller who has not unlocked.
4. `stream` admitted where `events` refuses, or the reverse.
5. A bump site removed (`persist` no longer bumps): the receiver cell sleeps.
6. The hook arming a timer while connected (the fake timer throws).
7. `unlock` writing the list on a wrong password, or at an open table.
8. `unlocked` or a watcher's email serialized on any payload built for someone else.
9. The spectator's frame built for the ACTING seat (options appear; the by-value cell).
10. The spectator's served run taking the `hasBotStep` branch (the app-level walk cell).
11. The e2e's "no `/events` during the wait" passing because the watcher tab was hidden
    (the tab is asserted visible and the stream open).
12. The e2e's account-row check passing on a stale row (the cell reads the row's event
    index).

### 9.6 Byte-identical

The five crates EMPTY in `git diff master`; `SEMANTICS_REV` 7; the CLI's diff exactly
the `Option` widening and its cell, goldens and `guide_sync` untouched; clippy clean;
the five suites re-counted at the close HEAD in one sequence.

---

## 10. Deployment

- The image tag is `0.5.0-m6.3`; the recipe is `deploy/RUNBOOK.md` §1–§3 by reference,
  the record a §3.9 addendum. No new environment variable on the NAS (the notifier
  secret is M6.5's).
- Before the swap, R-3.1's measurement is repeated against the DEPLOYED stream from the
  workstation through the tunnel and recorded beside the pre-deploy reading.
- Verification from the shell: `curl -N -H 'Accept: text/event-stream'` with the
  identity header on `…/{id}/stream` prints the on-connect frame and a heartbeat comment
  within 25 s; a move made from a browser prints a frame; `/api/health` reads
  `"signal":"sse"` (or `"held"`).
- The live review is against the live site in both layouts from THREE browsers as three
  people: two playing, one watching by link (R-P11), and it reads the watcher's network
  panel during a full turn.
- `deploy/cloudflared-config.yml`'s Yokohama ingress block gains the sentence that the
  stream relies on the 25 s heartbeat and that `keepAliveTimeout` is the pool's, not the
  stream's, with the measured reading.

---

## 11. What this spec does NOT claim

- **No chat, no hosting, no notifier built**: M6.4 and M6.5. §7 is a design.
- **No spectator count or watcher list is PROJECTED to anyone, and no kick.** The
  `unlocked` list IS stored on the table document, capped at 256, for the table's life;
  there is no re-lock and no expiry.
- **No per-connection limit** at the origin and no pruning of the signal map; the NAS
  serves a few people, and the house channel's "refetch all lists" is three
  table-document listings per bump.
- **The stream is not a security boundary either**: what it carries is what the
  `events` route would answer the same caller.
- **The SSE decision is held to a reading on ONE tunnel** on one day; the held-request
  fallback is what a different edge gets, by configuration of the same hook.
- **`EventsQuery.after` stays validated-and-inert** (R-3.14); no server-side resume.
- **No wall figure is a budget**: the two-browser cell's new wall is recorded beside the
  old, not gated.
- Every count, line number and route name above is at `48ae908`; M6.2 moves several
  (the `games.rs` cut above all), and the implementation plan re-reads them at M6.2's
  close HEAD.

---

## 12. Read order for the implementer

1. `CLAUDE.md` — the Web (server) and Web (browser half) blocks, and the
   `crates/yokohama-web` + `web/` architecture bullet.
2. `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0, §1 (the three seams, and
   the request arithmetic that motivates this cycle) and §2.
3. M6.2's close (written at its end) — the LEAVE list and what moved.
4. `docs/plans/2026-09-15-web-frontend-m6.2-lobby-spec.md` §3.4, §3.5, §4, §5.5 — the
   status refusals, the lock split, the password, and the poll hook this cycle re-plumbs.
5. This document.
6. `docs/plans/2026-09-05-web-frontend-m5.5-close.md` §1 and §4 — the walk, which is the
   spectator's primitive, and the sentence about a served turn's last step.
7. `docs/plans/2026-08-31-web-frontend-m3-spec.md` §4 — `why`'s masking, which a
   spectator inherits unchanged.
8. `deploy/RUNBOOK.md` §0, §1–§3, §3.2; `deploy/cloudflared-config.yml`.
