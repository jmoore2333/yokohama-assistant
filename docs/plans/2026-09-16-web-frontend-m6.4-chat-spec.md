# Web frontend — M6.4 "chat": spec

**Written 2026-09-16 on branch `web-room-m6.4` at `2194b57` (M6.3's shipped HEAD, the tip of
PR #33), from the brainstorm held the same day between the owner and the orchestrating session.**
The entry artifact is `docs/plans/2026-09-16-web-frontend-m6.4-chat-handover.md` (the charter,
the three seams, the eight questions with their leans); the plan this executes the fourth cycle
of is `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` (§0 the rulings R-P1–R-P11, §1 the
three seams, §2 this cycle's charter). **It is executed on the seams M6.3 leaves** (its close,
`docs/plans/2026-09-16-web-frontend-m6.3-close.md`, §1 what shipped, §5 the twenty-one execution
rulings, §6 the LEAVE list and the four owner items). Every "at HEAD" fact below is at `2194b57`;
the implementation plan re-reads each citation at the HEAD it opens, as the last three plans did.

The name is what the cycle delivers: two players talk during a game, a third reads it as a
spectator, and the same three can talk in the lobby.

---

## 0. The problem, and the approach

### 0.1 What is true at HEAD

- **Nothing on this server carries a word a person typed to another person.** The two
  documents are the record (`<id>.json`) and the table (`<id>.table.json`),
  `crates/yokohama-web/src/store.rs:36-55` (`Doc::{Record, Table}` and `suffix()`); the id
  allowlist `is_safe_id` (`store.rs:152`) subtracts ONE reserved suffix, `.table`
  (`RESERVED_ID_SUFFIX`, `store.rs:140`, compared case-insensitively at
  `spells_a_table_document`); `FsStore::delete` and `MemStore::delete` each walk the array
  `[Doc::Record, Doc::Table]` (`store.rs:321`, `store.rs:393`); `FsStore::list` strips
  `.json` and keeps only ids that pass `is_safe_id` (`store.rs:265-290`). **A third document
  kind needs a second reservation the derivation assert would not notice** (`CLAUDE.md`, the
  identity paragraph): the store cell `the_two_kinds_are_separate_documents_under_one_id`
  (`store.rs:515`) proves two kinds under a dotted id, on both stores.
- **The signal is a `watch` channel per table and one for the house** (`signal.rs`): `Bump
  { seq, end }` per table, a bare `u64` for the house (`signal.rs:15-24`); `bump_table`,
  `bump`, `end`, `subscribe`, `subscribe_house` (`:42-74`). Six bump sites, every one after
  its write (M6.3 close §1.1): `create` (`tables.rs:759`), `flip_to_playing` (`:1011`),
  `join` (`:1158`), `keep` (`:1260`, table only), `persist` (`games.rs:503`), `delete`
  (`:1441`, `end`). `unlock` bumps nothing (R-pre-3, `tables.rs:870`).
- **The frame is four fields** — `Events { events, to_move, status, your_turn }`
  (`lib.rs:1492`), built by `events_of` (`:1602`) at `playing`/over and by literal arms at
  `open` and `dealing` inside `events_at` (`:1638-1666`); the stream handler maps every
  bump through `events_at` (`:1703-1750`), the house stream writes `{"seq": N}` from
  `from_changes` (`:1758`); the held arm `GET …/events?wait=N` answers the same object
  (`:1569`, `MAX_WAIT_SECONDS` 25 at `:1539`). The M6.5 notifier is designed to be poked
  with "the same `Events` JSON the stream writes" (M6.3 spec §7).
- **Admission is one function**: `admitted(state, id, user) -> (TableDoc, Viewer)`
  (`lib.rs:458`), `Viewer::{Seat(PlayerId), Spectator}` (`client.rs:244`); a seat holder at
  every status, a `Spectator` at `playing`/over once `TableDoc::can_watch`
  (`table.rs:256`) says so, a stranger 403 at `open`/`dealing` with `NOT_SEATED`
  (`lib.rs:511`) and 403 "this table is locked — unlock it first" at a locked playing table.
  `spectators_write_nothing` (`lib.rs:515`) refuses a `Spectator` on every write route
  before the handle is touched (R-3.6); the sweep behind a record-refusing store pins it.
- **A person is an email and, per SEAT, an optional name.** `identity::User::parse`
  (`identity.rs:92`) normalizes once; `SeatDoc::Human { user, name: Option<String> }`
  (`table.rs:38-47`, R-2.3: the name beside the email, never a replacement);
  `tables::check_name` (`tables.rs:448`, 1–64 characters, no control characters, trimmed).
  The client remembers a display name per BROWSER under `yokohama.display_name`
  (`web/src/api.ts:128-146`, `displayName`/`setDisplayName`), typed on the New table form
  (`NewGame.tsx:144`), the open table's seat picker (`OpenTable.tsx:67`) and sent with a
  keep (`Dealing.tsx:237`). The sign-in door (`SignIn.tsx`, `claimed` mode only) asks for
  the email and nothing else; under `cf-access`/`worker` no door renders and the lobby's
  who-strip reads `You are <email>` (`Lobby.tsx:204`, `App.tsx:847`).
- **A watcher is an email on a list nobody sees.** `TableDoc.unlocked: Vec<String>`
  (`table.rs:211`, cap `UNLOCKED_CAP` 256 at `tables.rs:668`), never serialized (a by-key
  cell); the corpus pin `no_payload_built_for_someone_else_carries_a_watchers_email`
  (`tests/wire_fixtures.rs:1714`) searches every payload for `green@wire.test` (the
  walk's watcher) and `grey@wire.test` (a caller who never knocked), with an anti-vacuity
  partner that asserts the address IS in the table document the unlock wrote. Nothing
  holds a name for a watcher, and at an UNLOCKED table a watcher never calls `unlock`.
- **The game screen holds its stream only while waiting on somebody else.** `useSignal`'s
  `active` is `!!state && !state.over && !state.your_turn` (`web/src/App.tsx:653-655`,
  ruling R-9.1: `busy` was removed from it because the arrival handler sets `busy` around
  its own fetch and tore the stream down per move — 2,643 reopens in one game). The
  arrival handler parks a frame that lands while a run is being walked
  (`arriving`/`pending`, `App.tsx:672-676`); the post handler `send` (`App.tsx:533`) holds
  `inFlight` and re-reads once for a hot-seat pass. So the player whose turn it is holds
  no stream, and nobody does after game over — the M6.3 close's Minors M-1 and M-3
  (§6.2) are this.
- **The hook has two frame modes** (`web/src/signal.ts`, `Source.frames`, R-7.1): `"value"`
  hands the frame's `data` to `onValue` (the game and the dealing screen, `T = Events`);
  `"wake"` (the default) drops the frame and hands `once()`'s result over (the open table
  re-reads `api.table`, `OpenTable.tsx:112`; the lobby re-reads its three listings,
  `Lobby.tsx:123`), coalesced one read at a time (`reread`, `signal.ts:222`). Under the
  dark held arm a `"wake"` source is re-asked every `HELD_REASK_MS` (3 s). A consumer's
  `active` must never toggle on its own handler (R-9.1).
- **The side column** is `aside.side` (`App.tsx:1259`): the turn account (`:1264`) then
  the Players/Hand dashboard (`:1284`); above 1201 px it is grid column 2 row 4 and
  scrolls, under the answer bar at row 3 (`web/src/styles.css:195-205`); below, it is
  the stack's middle. A panel is a `section.panel` with an `h2` carrying a `ghost fold`
  button and a `.messages` body that folds to `max-height: 0` (`TurnAccount.tsx:115-160`,
  `styles.css:1313-1314`). The dealing screen's side column holds the other seats
  (`Dealing.tsx:330`); the open table renders its seats as a list (`OpenTable.tsx:227`)
  and a seated caller's link field (`:243`); the lobby is three `section.saved` lists
  (`Lobby.tsx:224-275`) under a who-strip (`:204`). A per-browser remembered set is the
  `sections.ts` idiom (a storage that throws reads as the default).
- **Deployment facts reach the client on `/api/health`**: `Health { server_version,
  semantics_rev, rules_hash, identity, signal }` (`lib.rs:310`); the client reads it once
  at boot and keeps `signal` module-level (`api.ts:179`, `signalKind`/`setSignalKind`).
  Startup reads `YOKOHAMA_WEB_IDENTITY` with no default and exits 1 on an absent or
  unknown value (`identity.rs:55-80`, `main.rs:94-99`); `YOKOHAMA_WEB_PORT`,
  `YOKOHAMA_WEB_HOST`, `YOKOHAMA_WEB_DIR`, `YOKOHAMA_WEB_ART`, `YOKOHAMA_WEB_DIST` have
  defaults. `AppState` is `rules`, `games`, `identity`, `heartbeat` (`lib.rs:254`).
- **The instruments**: the corpus is 71 files under `web/src/wire/`, regenerated with
  `UPDATE_WIRE_FIXTURES=1` and read as a diff; `contract.ts` type-checks it against
  `types.ts` with a key list per root (`HEALTH` at `contract.ts:476`, `EVENTS` at `:488`);
  the whole-game gate's two arms (`tests/play_a_whole_game.rs:141`, `:370`) stay out of
  CI; `web/e2e/two_browsers.spec.ts` drives three Chromium contexts and asserts the watcher
  asked `/events` zero times on ONE `/stream` (`:177-261`, `:404`). The counts the M6.3
  close records: server 191 (181 lib + 2 gate + 8 wire), CLI 851, client 587 in 35 files,
  e2e 39 in 8. The plan re-counts at its HEAD.

### 0.2 The approach

**A count on the signal, a fetch for the text** (R-4.7). Messages live in a third document
per table and one for the house; `Events` gains a fifth field, the table's last message
number; the house frame gains a second counter. A client compares the number it holds and
asks `GET …/chat?after=N` for the delta, exactly as it compares `events` today, so R-P9 is
kept, every transport arm and the M6.5 poke carry the field by construction, and the
content never rides the coalescing channel. **One component, four hosts**: a single `Chat`
panel that owns its list, its fetch and its post, told by its host only where it is and
when to look. **The stream is held at every status while the tab is visible** (R-4.4),
because the person deciding their move is exactly the person being talked to, and "gg"
after the game is the commonest message in the world.

### 0.3 The consequence worth stating first

A watcher may WRITE one thing. R-3.6's "a spectator reads everything and writes nothing"
gains one named exception (R-4.11), and the fence that proves it changes shape rather than
disappearing: the write sweep keeps every route and names chat; the email pin narrows from
"a watcher is named nowhere" to "a SILENT watcher is named nowhere, and a posting one only
as the author of what they posted" (R-4.12). Both are stated at their real size in §7.

---

## 1. Rulings specific to this cycle (2026-09-16)

Beyond the plan's R-P1–R-P11, M6.2's R-2.x and M6.3's R-3.x, the owner ruled in the
brainstorm:

- **R-4.1 — who posts.** Seat holders AND admitted watchers. A bot seat never posts in
  this cycle, because no HTTP identity is a bot (§1's R-4.15 is where a bot's message would
  come from later). The write route admits exactly as a read does (`admitted`): a seat
  holder at `open`, `dealing`, `playing` and over; a watcher at `playing` and over, at a
  table with no lock or once they have presented its password. A stranger at an open table
  reads and writes nothing here (the seat picker is not a chat).
- **R-4.2 — one identity, the lobby's label rule.** The signed-in email is the key for
  every message, players and watchers alike ("that same identity that players use would be
  the same that spectators use, they are just taking a lobby action"). What other readers
  SEE follows the lobby's own rule for a seat, `name ?? email`, for everyone: a seat's
  message is labelled by the seat's colour chip and its stored name or email; a watcher's by
  their given name or email under a "watching" marker. **A seat's name is the SEAT's**
  (`SeatDoc::Human.name`, written at create, join or keep): a seat holder's post that
  carries a `name` is a 400, so nobody can post as another seat; a watcher's post may carry
  one. A message therefore serves its author's email on the wire, which is the same fact
  `GET …/{id}/table` already answers every reader about every seat; the M6.3 pin narrows
  accordingly (R-4.12). **This settles the M6.3 close's owner item 1 (the email roster) as
  "leave as is."**
- **R-4.3 — the name is offered at the door, and it is the browser's.** The lobby's
  who-strip, the first screen in every identity mode, gains a "Your name" field beside
  "You are <email>", and the `claimed` door gains the same field as optional under the
  email. Both read and write the ONE key the table forms already use
  (`yokohama.display_name`), so the name a person joins under and the name they talk under
  are one string. No server-side profile: "if it's what we have either through the identity
  provider or it's what we're collecting … that should be the path of least resistance."
  When an identity provider supplies a name, `/api/me` carries it and the client prefers it
  (R-P2's "drop in the identifier later"); that is a later cycle's line.
- **R-4.4 — the stream is held at every status while the tab is visible.** The game
  screen's `active` becomes "there is a game on screen"; R-P8's hidden-tab rule is the
  hook's and stands. **The cost is one guard**: the post handler marks itself in flight and
  the arrival handler parks any frame that lands meanwhile, re-comparing it after the reply
  has been applied, so a self-bump is never walked as an opponent's move (§6.4). M-1 and
  M-3 close as a consequence. **The hosted arithmetic, because the owner asked**: under SSE
  a held stream is ONE connection per visible tab however long it is held, and at HEAD each
  player's tab tears its stream down and reopens it at every turn boundary, so holding it
  always is fewer connections; on the hosted path the notifier is poked once per bump, so a
  message costs one Worker request against a free tier of ~100k a day, and the delay is the
  poke's. "Some delay would be acceptable but not too long": under the dark held arm a
  message waits at most `HELD_REASK_MS` plus the residual written at that constant.
- **R-4.5 — where chat exists.** The game screen (playing and over, seats and watchers),
  the dealing screen, the open-table screen for SEATED callers, and the lobby. The route
  answers at every status for a seat holder at no extra cost, which is what lets the open
  table and the dealing screen carry the same panel.
- **R-4.6 — the lobby chat is a switch.** `YOKOHAMA_WEB_LOBBY_CHAT` reads `on` (the
  default) or `off`; any other value exits 1 at start naming the variable. Unlike the
  identity variable it HAS a default, because a missing switch is not a missing door. Off:
  the two house routes answer 404 `{"error":"the lobby chat is off"}`, the house counter
  never moves, `Health.lobby_chat` is false and the client draws no lobby panel. Table chat
  has no switch.
- **R-4.7 — delivery is a count on the signal.** `Events.chat` (the table's last message
  number, 0 with no document) on every arm including `open` and `dealing`; the house frame
  `{seq, chat}`, two counters bumped independently. R-3.4 is amended from "a sequence only"
  to "counters only, never a per-table payload". The alternative — an `event: chat` frame
  carrying the text — was rejected: a `watch` channel loses one of two messages between
  reads, the held arm and the notifier would need a second message shape, and history on
  connect needs a fetch anyway. A second connection per tab was rejected on R-3.2 and the
  R-9.1 lesson.
- **R-4.8 — retention is a launch variable.** `YOKOHAMA_WEB_CHAT_KEEP` (messages retained
  per document; a positive integer, default 500; `all` for indefinite) and
  `YOKOHAMA_WEB_CHAT_TEXT` (characters per message; a positive integer, default 500), both
  reported on `/api/health` so the client sizes its input from the server's rule and never
  from a copy. **A document longer than the configured keep is trimmed on its NEXT APPEND
  and never refused on read**, and a stored text longer than the configured cap is served
  as stored, so lowering either number on a running volume cannot take a table out. The
  owner's reason is recorded: "people who end up using this strictly internally may want it
  to hold indefinitely or otherwise."
- **R-4.9 — the document.** `<id>.chat.json` beside the record and the table, under a
  second reserved suffix `.chat`; the house's at the fixed file `_house.chat.json`, which a
  leading underscore makes unspellable as a game id under the allowlist (`is_safe_id`'s
  first-character rule), so it collides with nothing and needs no reservation. Message
  numbers start at 1 and are never reused: the cap drops the OLDEST and `last` keeps
  counting, so a client's `after` cursor stays valid across a trim. Never undone, never in
  the record, a table's chat removed with the table (`delete`), the house's trimmed and
  otherwise kept.
- **R-4.10 — the panel is a bonus, not the main attraction.** LAST in the side column and
  FOLDED by default at every host ("the key focus is for the user trying to play the game
  as it is"), the fold remembered per browser per scope; folded, the header carries a
  "new" count, the panel opens no connection of its own, and on a screen whose frame
  carries the count it fetches nothing until unfolded (§6.1 states the wake-driven
  screens' cost); the lobby's opt-in ("enter the lobby chat") IS the unfold.
- **R-4.11 — a watcher's one write.** `POST …/{id}/chat` admits a `Spectator`; every other
  write route keeps R-3.6's refusal. The sweep names the exemption; a locked table's
  password gates the post exactly as it gates the read (`admitted`'s row).
- **R-4.12 — the email pin narrows to its true statement.** A silent watcher (on
  `unlocked`, never posted) and a caller who never knocked appear in NO payload; a posting
  watcher's address appears only as the `user` of a message they posted. The anti-vacuity
  partner stays.
- **R-4.13 — colours are the table's.** The corpus walk is 2P by its own construction
  (red, blue, a watcher), and a seat's chip is `colors[seat]` from the table document, so
  yellow and green are covered by construction; the seat-bound and label cells run at 2P,
  3P and 4P rather than assuming two, following the spectator-projection precedent.
- **R-4.14 — out.** No edit or delete of a message, no moderation, no rate limit on the
  NAS, no kick, no notification beyond the folded panel's count, no read state beyond the
  browser session.
- **R-4.15 — a talking bot is possible later, and three choices now keep it cheap.**
  The owner: "at some point in the future I could imagine a world where we hook in an LLM
  to allow the computer/bot player to 'chat' as well … feed the game play json and some
  instructions over a call that can originate on the container over an openai compatible
  call." Recorded here so the design keeps it in mind and goes no further: (a) a message's
  author is a TAGGED object (`seat` | `watcher` today, `bot` the arm no route produces
  yet), the `SeatDoc` shape, which gains an arm additively on both sides of the wire;
  (b) ONE internal writer, `Games::chat_append(id, author, text)`, with the route turning a
  `Viewer` into an author, so a bot's message is a second CALLER and not a second write
  path, and the signal design already handles a message landing seconds after a move from
  a non-HTTP author; (c) **anything fed to an external model is the bot's own VIEW and
  never the record** — the frame built for its seat, the log, its own `why` account, the
  chat — because a `GameRecord` carries the setup seed, which is the whole hidden set. The
  trigger, the prompt, the endpoint and key variables, the HTTP client dependency, cost and
  cadence and the personality's name are all §11's.

---

## 2. The documents — `crates/yokohama-web/src/chat.rs` (new) and `store.rs`

### 2.1 The store

```rust
pub enum Doc { Record, Table, Chat, House }   // .json · .table.json · .chat.json · the fixed file _house.chat.json
```

- `Doc::Chat` is `<id>.chat.json`. `RESERVED_ID_SUFFIX` becomes the pair `[".table",
  ".chat"]`, still case-insensitive, and `is_safe_id` refuses an id ending in either; the
  store's dotted-id cell becomes `the_three_kinds_are_separate_documents_under_one_id`
  (`a.b.json`, `a.b.table.json`, `a.b.chat.json`, both stores). `FsStore::list` needs no new
  branch — `<id>.chat` fails `is_safe_id` the way `<id>.table` does — and the redundant
  defence line gains the second suffix anyway, as the second of two.
- `Doc::House` is the ONE file `_house.chat.json`, whatever id is passed; callers pass
  `chat::HOUSE` (`"house"`), `FsStore::path` returns the fixed path for that id and refuses
  any other, `MemStore` keys `(Doc::House, "house")`. `delete(id)` walks
  `[Record, Table, Chat]` and never `House`; `list(Doc::House)` answers `["house"]` or `[]`.
- **Why a leading underscore and not a reserved NAME**: `FsStore::path` calls `is_safe_id`
  on every read and write, so reserving the id `house` would refuse the house document's
  own reads; a file no game can spell needs no reservation, and `list(Doc::Record)`
  excludes it by the first-character rule already there.

### 2.2 The document

```rust
pub const CHAT_SCHEMA: u32 = 1;
pub const HOUSE: &str = "house";

pub struct ChatDoc { pub schema: u32, pub id: String, pub last: u64, pub messages: Vec<Message> }

pub struct Message { pub seq: u64, pub at: u64, pub author: Author, pub text: String }

#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Author {
    Seat { seat: u8, user: String, name: Option<String> },
    Watcher { user: String, name: Option<String> },
    /// R-4.15. Serialized and read; PRODUCED BY NO ROUTE in this cycle.
    Bot { seat: u8, name: Option<String> },
}

pub struct ChatLimits { pub keep: Option<usize>, pub text: usize }   // None = all
```

`at` is Unix seconds, the `created_at` idiom (`table.rs:177`). `seq` is assigned as
`last + 1` under the lock and `last` is written with it. **The wire shape IS the stored
shape** for a message (`Message` serializes as is; the author's `user` rides, R-4.2), and
the read route's reply is `ChatPage { messages: Vec<Message>, last: u64 }`.

### 2.3 `check_shape(doc, players: Option<u8>)`

Run on read AND on write (the table document's rule, `tables.rs:358`): `schema <=
CHAT_SCHEMA`; `seq` strictly increasing and `last >= ` the newest `seq`; every `user`
parses as `User`; every `name` passes `check_name`; every `text` non-empty with no control
character; a `Seat` or `Bot` author's `seat < players` at a table (it reaches
`PlayerId::new`, which panics — the M6.1 lesson) and NO seat author at the house (`players`
is `None` there). **Not checked on read, deliberately (R-4.8)**: the message count against
`keep` (trimmed on the next append) and the text length against `text` (served as stored).
A document that fails is the typed `Server` error every other malformed document is, never
a panic under a lock.

### 2.4 The append

`Games::chat_append(&self, id: &str, author: Author, text: String) -> Result<Message,
GameError>` (R-4.15 (b)): `chat::check_text(&text, &limits)` first (trimmed, non-empty,
at most `limits.text` characters, no control character — a 400 with the sentence; the
ROUTE has already run the same function before building the author, so the order in
§3.2 holds, and the append re-runs it so a second caller is bounded too), then under
`lock_for(id)`
(`games.rs:219`, R-2.9): read `Doc::Chat` (absent → an empty document), `check_shape`,
push `Message { seq: last + 1, at: now, author, text }`, trim to the newest `keep` if set,
`write_if_version` on the version read (a `Conflict` is a 409, a cross-process write this
process did not make — `persist`'s shape), THEN `bump_table(id)`. `Games::house_append`
is the same under a dedicated `house_lock: Mutex<()>`, `Doc::House`, `players: None`, then
`bump_house_chat()`. **A bump follows its write and never precedes it**; each site gets the
gated cell (§9.1).

`Games::chat_last(id) -> u64` is what `Events.chat` reads: an in-process cache
`chat_last: Mutex<HashMap<GameId, u64>>` seeded from the document on a miss, written by
`chat_append` under the table lock, cleared by `delete`. The stream closure therefore reads
NO document for the count (it is called once per bump per connection, and a 500-message
document is a parse this closure must not pay). **Its limit, stated**: a second PROCESS's
appends are invisible to this one's count, which is true of the record's live handle today
and is M6.5's to change when the store is a database and the count is a query (§11).

---

## 3. The routes

### 3.1 `GET /api/games/{id}/chat?after=N`

`admitted` first (R-4.1's rows). Answers `ChatPage` with every retained message whose
`seq > after` (`after` defaults to 0), and `last`. `after` beyond `last` answers an empty
list and `last`. A table with no document answers `{ messages: [], last: 0 }`. No lock is
held (a read of one document); the body is a projection and serves a locked session (the
M3 lockdown rule).

### 3.2 `POST /api/games/{id}/chat` `{ text, name? }`

`admitted` first; then, in this order, pinned by adjacent-pair probes as M6.2 pins its join
order: (1) the text rule (empty, over the cap, a control character — each its own
sentence); (2) a `Seat` caller carrying `name` → 400 "a seated caller is named by their
seat"; (3) a `Watcher`'s `name` through `check_name` → 400 with its sentence; (4) the
append (a 409 on a store conflict). Answers **201** with the `Message`. The author: a
`Viewer::Seat(s)` → `Author::Seat { seat: s, user, name: the seat's stored name }`, where
`s` is the viewer seat (`client::viewer_seat`, the acting seat when the caller holds it,
else the caller's lowest — the same seat every other payload is built for); a
`Viewer::Spectator` → `Author::Watcher { user, name: body }`.

### 3.3 `GET /api/chat?after=N` and `POST /api/chat`

The house pair. Admission is the identity extractor's alone (a 401 without one), like the
house stream; **404 `{"error":"the lobby chat is off"}` on both when the switch is off,
before anything else**. The author is always `Watcher { user, name: body }` — the house has
no seats — and the post's order is (1) the switch, (2) the text rule, (3) the name rule,
(4) the append.

### 3.4 The admission table (M6.3 spec §3.5, extended)

| caller | status | `chat` GET | `chat` POST | the other writes |
|---|---|---|---|---|
| holds a seat | any | 200 | 201 | as today |
| no seat | `open` | 403 not seated | 403 not seated | `join` as M6.2; the rest 403 |
| no seat | `dealing` | 403 | 403 | 403 |
| no seat; unlocked, or on `unlocked` | `playing` / over | 200 as `Spectator` | **201** as `Watcher` | 403 not seated |
| no seat; locked, not unlocked | `playing` / over | 403 "locked — unlock it first" | 403 same | 403 not seated |

The M6.3 grid cell (six tables × three identities × eighteen routes) gains the two routes
as its nineteenth and twentieth and asserts the POST column equals the GET column at every
cell. The house pair is a separate two-row table: signed in → 200/201; the switch off →
404/404.

### 3.5 The bodies and the query

```rust
#[serde(deny_unknown_fields)] pub struct ChatPost  { pub text: String, #[serde(default)] pub name: Option<String> }
#[serde(deny_unknown_fields)] pub struct ChatQuery { #[serde(default)] pub after: Option<u64> }
```

Extracted through `JsonBody`/`QueryArgs` (`lib.rs:189`, `:211`), so an unknown key is a
400 in this API's `{"error": …}` shape; `after` is typed, so a negative cursor is serde's
400.

---

## 4. The signal — `signal.rs`, `lib.rs`

- `Events` gains `pub chat: u64` as its FIFTH field, on every arm: the `open` literal, the
  `dealing` literal and `events_of` all read `games.chat_last(id)`. At `open` the arm
  answers before `admitted` (the M6.3 execution ruling the close's §5 numbers R-4.3 — not this cycle's R-4.3), so a stranger on the seat picker reads the
  count; a count leaks nothing and the same caller's `chat` GET is 403.
- The house channel becomes `watch::Sender<House>` with `pub struct House { pub seq: u64,
  pub chat: u64 }`; `bump` increments `seq` as today; `bump_house_chat` increments `chat`
  only; `subscribe_house` returns `Receiver<House>`; the house stream writes
  `{"seq": N, "chat": M}` from `from_changes`, still silent on connect. **The house frame's
  `chat` is the CHANNEL's counter and not a document read**, so the house stream stays
  store-free; a `"value"` consumer of it is not written this cycle (§11).
- Two new bump sites, in success arms only, each after its write: `chat_append` →
  `bump_table`; `house_append` → `bump_house_chat`. `delete` clears the cache and its `end`
  is unchanged; the removed chat file needs no bump of its own.
- The stream handler is untouched beyond the field: `events_at` builds the frame. The held
  arm answers the same object. The M6.5 poke carries the field.

---

## 5. The switch and the limits — `main.rs`, `lib.rs`

```rust
pub struct ChatConfig { pub lobby: bool, pub limits: ChatLimits }
impl ChatConfig {
    pub fn from_env_values(lobby: Option<&str>, keep: Option<&str>, text: Option<&str>) -> Result<Self, String>;
}
```

The identity extractor's shape (`identity.rs:64`): the read split from the parse so the
unset case is testable without touching the process environment. `YOKOHAMA_WEB_LOBBY_CHAT`:
absent or `on` → true, `off` → false, anything else → `Err` naming the variable and the
two words. `YOKOHAMA_WEB_CHAT_KEEP`: absent → `Some(500)`, `all` → `None`, a positive
integer → `Some(n)`, anything else (0, negative, text) → `Err`. `YOKOHAMA_WEB_CHAT_TEXT`:
absent → 500, a positive integer → n, else `Err`. `main.rs` reads it beside the identity
mode and exits 1 on `Err` with the message. `AppState` gains `pub chat: ChatConfig`; the
in-memory constructors default it (`lobby: true`, 500/500) and gain
`in_memory_with_chat(ChatConfig)` for the cells that need the switch off or a small keep.

`Health` gains three flat keys — `lobby_chat: bool`, `chat_keep: Option<u64>` (`null` for
`all`), `chat_text: u64` — flat rather than nested because the contract's key rows are per
root object. The client reads them once at boot beside `signal` and keeps them
module-level (`lobbyChat()`, `chatText()` in `api.ts`).

---

## 6. The client

### 6.1 `web/src/Chat.tsx` — one component

```ts
<Chat scope={{ kind: "table", id }} | {{ kind: "house" }}
      count?: number      // the server's last message number, where the host holds it
      tick?: number       // "something may have changed", where the host's hook drops the frame
      colors?: PlayerColor[]   // a table's; absent at the house
      sendName: boolean />     // true for a watcher and at the house: the browser's name rides the post
```

It OWNS: the list (`messages`, `last` = the highest `seq` held), the fold (`folded`, read
from and written to `localStorage` under `yokohama.chat.folded.<table id | house>`, the
`sections.ts` idiom, DEFAULT TRUE everywhere per R-4.10), the unread count (messages that
arrived while folded; reset to 0 on unfold; never persisted), the draft, the in-flight
guard and the error line.

- **Fetching.** On mount, `api.chat(scope, 0)` — everything retained. When `count` rises
  above `last`, `api.chat(scope, last)`; when `count` is at or below `last`, NOTHING (the
  cell that catches a panel fetching on every move). When `tick` changes, `api.chat(scope,
  last)`. Results are appended after filtering `seq > last`, so a message this browser
  already holds is never drawn twice. **Folded, the two kinds of host differ, and the
  difference is stated rather than hidden**: on a COUNT-driven host (the game, the deal)
  the header's number is `count - last` and nothing is fetched until the unfold, which
  fetches once; on a WAKE-driven host (the open table, the lobby) the frame is dropped by
  the hook, so the panel asks the delta (`after=last`, a few bytes, usually empty) on each
  tick whether or not it is folded, holds what came back, and the header's number is what
  it holds unread. The delta IS the badge's only source there; the lists the same wake
  refetches cost more than it does.
- **Posting.** A `form` with one `input` (`maxLength` = `chatText()`), Enter or the button
  sends; `api.postChat(scope, { text, name? })` where `name` is sent ONLY when `sendName` is
  true — a watcher, and anyone at the house; a seated caller sends none (R-4.2) — and
  only when the browser holds one; the
  201's message is appended locally and `last` advanced, so the frame it causes is a
  no-op compare. A refusal renders in the panel's own error line, in the server's words.
  The input is disabled while a post is in flight, never the stream.
- **Rendering.** `section.panel.chat`, an `h2` with the title ("Chat" at a table, "Lobby
  chat" at the house), the unread count when folded and non-zero, and the `ghost fold`
  button (`aria-expanded`, `aria-controls`); a `.messages` body that is `max-height: 0`
  folded and a capped scroller open (the turn account's numbers, `styles.css:1313`),
  newest LAST, kept at the bottom on append unless the reader has scrolled up (the turn
  account's `scrollTop` rule, never `scrollIntoView`); one `li` per message with the
  label, a time (`HH:MM` local; the date before it when not today, R-P3's night away) and
  the text. **The label is R-4.2's**: `Seat` → a chip in `colors[seat]`'s class and `name
  ?? user`; `Watcher` → `name ?? user` and a `watching` marker; `Bot` (never received this
  cycle, rendered defensively) → the chip and `name ?? "bot"`. The whole panel takes
  `margin-top: auto` in the side column's flex so it sits at the column's foot when there
  is slack, and at the end of the scroll when there is not.

### 6.2 The four hosts

- **The game screen** (`App.tsx`): `chatCount` state, set from `e.chat` at the TOP of the
  arrival handler — before the parking check and before the `events` compare, because a
  count is harmless to apply at once and a parked frame must not delay the badge. `<Chat
  scope={{kind:"table", id}} count={chatCount} colors sendName={state.seat === null} />`
  LAST inside `aside.side`, after the dashboard, for players and watchers alike.
- **The dealing screen** (`Dealing.tsx:200`): the same, from its `useSignal` handler,
  which today acts only when the status leaves `dealing` and now also sets the count;
  the panel LAST inside its `aside.side` (`:330`).
- **The open table** (`OpenTable.tsx`): its hook is `"wake"` and drops the frame, so
  `take` (`:91`) increments a `tick`; `<Chat tick>` under the link field, rendered ONLY
  when `seated` (`:243`) — a stranger on the seat picker is 403 and must not be shown a
  panel that refuses.
- **The lobby** (`Lobby.tsx`): `take` (`:105`) increments a `tick`; a fourth
  `section.saved.lobby-chat` after Games in progress (`:263`), rendered ONLY when
  `lobbyChat()` is true. Folded by default (R-4.10). **The cost, stated**: a house message
  is one more house wake, and every visible lobby refetches its three listings on ANY
  house frame (R-3.4's rule) plus the chat delta; a `"value"` consumer of the house frame
  that refetched lists only when `seq` moved is the LEAVE item this creates (§11).
- Under the dark held arm a `"wake"` host is re-asked every `HELD_REASK_MS`, so its panel
  fetches the delta every 3 s; a `"value"` host's count rides the held reply. Recorded,
  not gated.

### 6.3 The name field (R-4.3)

`Lobby.tsx`'s who-strip gains `<label>Your name <input …/></label>`, prefilled from
`displayName()`, written through `setDisplayName` on change (trimmed; an emptied field
removes the key). `SignIn.tsx` gains the same field as optional under the email, written
on submit when non-empty. The New table form and the open table's picker keep theirs; all
four read one key.

### 6.4 The stream, held (R-4.4)

`active` becomes `!!state` (`App.tsx:653`). `send` (`:533`) sets `posting.current = true`
before `api.input` and, in its `finally`, clears it and — if `pending.current` holds a
frame — runs the arrival loop over it. The arrival handler's parking check becomes `if
(arriving.current || posting.current)`. **Why this is sufficient**: when the reply lands,
`stateNow` carries the new count, so the parked self-bump compares equal and is a no-op;
if an opponent moved during the round trip the parked frame's count is higher and it
walks, which is right. The hot-seat re-read is untouched. The doc block on `active` is
rewritten: R-9.1's sentence "on this screen a write happens only while `your_turn`" was
the reason `busy` could leave `active`; it is now the reason the POSTING guard exists.
`Dealing`, `OpenTable` and `Lobby` already pass `true`.

### 6.5 Wire types and `api.ts`

`types.ts`: `ChatAuthor` (the tagged union, three arms), `ChatMessage { seq, at, author,
text }`, `ChatPage { messages, last }`, `ChatPost { text, name? }`; `Events.chat: number`;
`Health.lobby_chat: boolean`, `Health.chat_keep: number | null`, `Health.chat_text:
number`. `contract.ts`: `EVENTS` gains `chat`; `HEALTH` gains the three; new `CHAT_PAGE`,
`CHAT_MESSAGE` rows and a per-arm row for the author union (the `SeatDoc` precedent).
`api.ts`: `chat(scope, after)`, `postChat(scope, body)` over the two paths; `lobbyChat()`,
`chatText()` with their setters, read at boot beside `setSignalKind` (`App.tsx:530`).

### 6.6 The help panel

A new section **Talking** (the panel, the fold, who is labelled how, the lobby chat and
its switch, that a message is never undone and never in the record); one sentence in
**Watching a game** ("you can talk, and your messages say you are watching"); one in the
lobby's paragraph for the name field. `Help.test.tsx`'s emphasis-boundary rule runs over
all of it.

---

## 7. Fences

- **A spectator writes nothing but chat.** The M6.3 sweep (every write route behind a
  store that panics on any record read) keeps every route and asserts `chat` POST is the
  ONE that answers 201 as a `Spectator`; the exemption is a named list of one, and a
  second entry is a review question.
- **A locked table's chat is behind its password both ways**: the grid cell (§3.4) asserts
  POST ≡ GET at every cell.
- **A seat's name cannot be forged**: a seated caller's post carrying `name` is 400
  (mutation 4); the author's `name` on the 201 is the SEAT's, asserted by value against a
  seat whose stored name differs from the body's.
- **A bump follows its write**: the two new sites' gated cells (§9.1), the M6.3 shape.
- **The email pin narrows (R-4.12)**: `no_payload_names_a_silent_watcher_and_a_posting_one_
  only_as_an_author` walks every corpus payload as a JSON tree; `grey@wire.test` may appear
  nowhere; `green@wire.test` may appear only as the `user` of an `author` object inside a
  `Message` (a `messages[]` element or a 201 body). The anti-vacuity partner is unchanged
  (the address IS in the table document). A second partner is added: the address IS in
  `chat.json`, so the allowance is exercised and not vacuous.
- **The seed, the password, the salt and the hash** stay under their existing pins, which
  search the whole corpus and therefore the new files.
- **`stream` ≡ `events`** by value, unchanged and now over five fields.
- **The count never fetches on a move**: the component cell counts `api.chat` calls across
  N frames whose `chat` did not move (mutation 10).
- **Stored fields are inputs**: `check_shape`'s cells include a hand-written `seat: 7` at a
  2P table (refused, not a panic) and a house document carrying a `Seat` author.

---

## 8. The wire: routes and the corpus

Routes: M6.3's twenty-four plus `GET`/`POST …/{id}/chat` and `GET`/`POST /api/chat`. New
fixtures (eleven), captured in the walk at the points named:

- `chat.json` — `GET …/wire/chat` as red at `playing`, after red, blue and green (the
  unlocked watcher, with a name) have each posted once: three messages, three author arms
  exercised (two `seat`, one `watcher`).
- `chat_after.json` — the same as `?after=2`: one message, `last: 3`.
- `chat_post.json` — red's 201; `chat_post_watcher.json` — green's 201 (`author.kind:
  "watcher"`, the body's name).
- `chat_400_long.json`, `chat_400_seated_name.json` — the two 400 sentences the client
  renders.
- `chat_403_not_seated.json` — green at the OPEN table; `chat_403_locked.json` — grey at
  the locked playing table.
- `house_chat.json` — `GET /api/chat` as blue after red posted; `house_chat_post.json` —
  red's 201; `house_chat_off_404.json` — from a second `Api` built with
  `in_memory_with_chat(lobby: false)`.

Changed: every `events*.json` gains `chat` (0 at `events_open`/`events_dealing`, 3 at
`events.json` since the posts precede it in the walk — the walk's order is what the plan
fixes); `health.json` gains the three keys. **Every `ClientState`, `HistoricalFrame`,
`TableView` and `DealView` fixture is byte-identical**, which is what proves the panel is
fed by the frame and the table carries no chat field. Regenerated, run without the
variable, the diff read and reported in the close.

---

## 9. Tests, gates, and the mutations that must be caught

### 9.1 Server (`cargo test -p yokohama-web`)

- **Store**: three kinds under a dotted id on both stores; `.chat` refused as an id suffix
  case-insensitively; `_house.chat.json` absent from `list(Doc::Record)`; `delete` removes
  the chat file and leaves the house's; `Doc::House` refuses an id other than `HOUSE`.
- **Document**: `check_shape`'s rules one by one, the seat bound at 2P/3P/4P (R-4.13), the
  house's no-seat rule, the not-refused-on-read pair (a 501-character stored text is
  served; a 600-message document is served and trimmed to 500 on the next append); `seq`
  never reused across a trim (append 501 at keep 500, read `after=0`, the oldest is gone
  and the newest is 501); `keep: None` never trims.
- **Append**: the text rule's three sentences; the seat/name 400; a watcher's name through
  `check_name`; the author by value for a `Viewer::Seat` at a seat with a stored name
  differing from the body's; the two-seat hot-seat caller posts as the viewer seat; a
  store `Conflict` is 409; the two gated bump cells (the write parked behind a gate, the
  receiver quiet until it opens); `chat_last` seeded from a document on a cold miss and
  advanced by an append.
- **Routes**: the §3.4 grid extended by two routes with POST ≡ GET; the refusal order by
  adjacent-pair probes; `after` beyond `last`; the house pair signed in and switched off;
  the M6.3 stranger sweep extended.
- **Signal**: `Events.chat` on all three arms; the house frame's two counters bumped
  independently (`bump` leaves `chat`, `bump_house_chat` leaves `seq`); the stream's frame
  after a chat bump carries the new `chat` and an unmoved `events`.
- **Config**: `ChatConfig::from_env_values` over the value grid (absent/on/off/other,
  absent/all/500/0/-1/text), each `Err` naming its variable.

### 9.2 CLI (`cargo test -p yokohama-cli`)

Untouched: goldens and `guide_sync`; no CLI line moves this cycle.

### 9.3 Client (`npm run typecheck && npm test`)

`Chat.test.tsx` (new): fetch on mount over `chat.json`; a rising `count` fetches
`after=last` and appends `chat_after.json`; a `count` at or below `last` across ten renders
makes ZERO calls; a `tick` fetches; the dedupe (a delta carrying a held `seq` draws once);
the post appends the 201 and advances `last`; a seated scope sends no `name`; the 400
sentence renders; the three labels (chip + name, chip + email, watching marker); the date
before yesterday's time; folded by default, the unread count, the reset on unfold, the
remembered fold under a storage that throws; the input's `maxLength` from health.
`App.test.tsx`: `active` true during `your_turn` and at `over` (the hook's cell reads
`useSignal`'s second argument through a spy); the parked frame during a post (a frame
landing between `api.input`'s call and its resolution is compared AFTER the reply and
fetches nothing); the panel last in the side column. `Lobby.test.tsx`: the section absent
under `lobby_chat: false`, present and folded under true, the name field writing the key.
`SignIn.test.tsx`: the optional field. `Dealing.test.tsx`, `OpenTable.test.tsx`: the panel
present (and the open table's absent for a stranger). `contract.test.ts`: the new root-key
rows, `health.json`'s three keys, `events.json`'s fifth.

### 9.4 The gates that play

- `play_a_whole_game.rs`: the two-identity arm's watcher (green) posts one message after
  the first turn and both players read it at `after=0`; each player posts once and green
  reads both; `Events.chat` on the players' `events` reads equals `last`. The vs-bot arm is
  untouched (its livelock is owner item 2 and not this cycle's). Out of CI, per-edit.
- `web/e2e/two_browsers.spec.ts` gains the exit: after the game starts, A unfolds the
  panel and posts "hello"; B's panel shows it without a reload and B posts; C, the
  watcher, reads both lines and posts under a name typed into the lobby's field; **A reads
  C's line WHILE A's screen shows A's own prompt** — the one cell that must fail under
  HEAD's `active`. The stream count extends from the watcher to the players, as a
  NO-REOPEN claim: after each player's first game-frame reply (`GET /api/games/{id}`),
  no further `/stream` open on this table's path occurs for that page to the end of the
  game, with the tab asserted visible — which is exactly the per-turn reopen HEAD makes
  and this cycle removes. The cell's wall is recorded beside M6.3's 309 s, never gated;
  `CAP` stays as it is (owner item 3).
- `joins_by_link.spec.ts` gains nothing: the name typed there is the same key.

### 9.5 Mutations that must be caught (recorded in the close, per R-P11)

1. `chat_append`'s bump hoisted above its write (the gated cell sleeps).
2. `house_append`'s bump hoisted likewise.
3. A `Spectator` posting at a locked table without the password (the grid cell).
4. A seat holder's `name` taken from the body (the by-value author cell).
5. `chat` dropped from the `open` or the `dealing` arm of `events_at` (the three-arm cell).
6. The house routes answering with the switch off (the 404 cell).
7. `delete` leaving `<id>.chat.json` behind (the store cell).
8. An id ending in `.chat` accepted by `is_safe_id` (the reservation cell).
9. A `seq` reused after a trim, or `last` not advanced (the trim cell).
10. The panel fetching on a frame whose `chat` did not move (the zero-calls cell).
11. `active` regaining `!your_turn` or `!over` (the spy cell, and the e2e's during-turn read).
12. The posting guard removed (the parked-frame cell: a self-bump fetched and walked).
13. `grey@wire.test`, or `green@wire.test` outside an author, on any payload (the pin).
14. A control character or an over-cap text accepted on POST (the text-rule cells).
15. The lobby section rendered under `lobby_chat: false` (the Lobby cell).
16. `check_shape` letting `seat: 7` through at a 2P table (the input cell).
17. A stored over-cap text or over-keep document REFUSED on read (the R-4.8 pair).
18. The fold not read from storage, or the unread count not reset on unfold.

### 9.6 Byte-identical

The five crates EMPTY in `git diff master`; `SEMANTICS_REV` 7; the CLI's diff EMPTY (no
`pub` is needed — every builder this cycle reads is already public); clippy clean; the
suites re-counted at the close HEAD in one sequence.

---

## 10. Deployment

- The image tag is `0.5.0-m6.4`; the recipe is `deploy/RUNBOOK.md` §3.5 by reference, the
  record a §3.10 addendum. **No new variable is REQUIRED on the NAS**: the three have
  defaults; the runbook's create-body paragraph records them as optional with their values,
  and the Dockerfiles' comment blocks beside `YOKOHAMA_WEB_IDENTITY` (`deploy/Dockerfile:81`,
  `Dockerfile.runtime:46`) gain one sentence each.
- Verification from the shell: `/api/health` carries `lobby_chat`, `chat_keep`, `chat_text`;
  a `POST /api/chat` with the identity header answers 201 and a `GET` reads it back; a
  table's `…/stream` frame carries `chat`.
- The live review is against the live site in both layouts from THREE browsers as three
  people: two playing, one watching by link, all three talking at the table and in the
  lobby; it reads that the mover's panel receives a line during their own turn, that the
  folded panel's count moves, and that each player's network panel holds ONE stream
  across a whole turn boundary.

---

## 11. What this spec does NOT claim

- **No edit, no delete, no moderation, no rate limit, no kick** (R-4.14). `unlock` and
  `join` remain password oracles without a rate limit, as the M6.3 close records.
- **No read state beyond the browser session**: the unread count is a session number;
  nothing records what a person has read.
- **The name is per BROWSER, not per person**, and there is no profile (R-4.3); a person
  on two devices types it twice; a provider-supplied name is a later line on `/api/me`.
- **The count is per PROCESS** (`chat_last`); a second process's appends are invisible to
  this one's frames until the store is a database and the count a query (M6.5).
- **The house frame's `chat` is unread by any consumer this cycle**; the lobby refetches
  its lists on any house frame, so a chatty lobby refetches listings per message — the
  LEAVE item §6.2 names, with its fix.
- **The held arm's chat latency is the held arm's**: `HELD_REASK_MS` for a wake-driven
  host and the residual written at that constant for a value-driven one; the arm is dark.
- **The talking bot is a shape, not a feature** (R-4.15): the `Bot` author arm is produced
  by no route; no trigger, prompt, endpoint, key, HTTP client, cost rule or name is
  decided.
- **Nothing is measured with more than three browsers**, and the live review is minutes,
  not a week.
- **The stream is still not a security boundary**: what it carries is what `events` would
  answer the same caller, now with a count.
- **The vs-bot gate arm's livelock, the e2e's `CAP`, and the workstation's disk** are the
  M6.3 close's owner items 2–4 and are not this cycle's code.
- Every count, line number and route name above is at `2194b57`; the plan re-reads them.

---

## 12. Read order for the implementer

1. `CLAUDE.md` — the Web (server) and Web (browser half) blocks, and the
   `crates/yokohama-web` + `web/` architecture bullet, in particular its IDENTITY, LOBBY
   and TRANSPORT AND SPECTATING paragraphs.
2. `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0, §1, §2.
3. `docs/plans/2026-09-16-web-frontend-m6.3-close.md` §1, §5, §6, §7 — what is there, the
   rulings, the LEAVE list and the owner items.
4. `docs/plans/2026-09-15-web-frontend-m6.3-transport-and-spectating-spec.md` §2, §3.5,
   §5.1, §7, §9.5 — the channels, the admission table this extends, the hook, the notifier
   the frame must fit, the mutation shape.
5. `docs/plans/2026-09-15-web-frontend-m6.2-lobby-spec.md` §3.4–§3.5, §4 — the table
   document, the per-table locks, the password.
6. `docs/plans/2026-09-16-web-frontend-m6.4-chat-handover.md` — the charter and the leans
   this document settles.
7. This document.
8. `deploy/RUNBOOK.md` §3.5, §3.9.
