# Web frontend — M6.4 "chat": the close

**SHIPPED AND LIVE 2026-09-17: `yokohama-web:0.5.0-m6.4`, built at `571c446` on
`web-room-m6.4`, serving `yokohama.moore.nyc` from container `f5db3dfe…` since 03:53:45Z
(`deploy/RUNBOOK.md` §3.10). The HEADED live review of the deployed site returned SHIP
WITH FIXES — 0 Critical / 1 Important / 4 Minor (§4.5) — and all five findings were fixed,
re-reviewed clean and shipped on 2026-09-20 at 00:38:01Z as the point release
`yokohama-web:0.5.0-m6.4.1` from `b16ccdd`, container `a62b17ad…` (RUNBOOK §3.11). PR #34
is open against `web-room-m6.3`
(https://github.com/jmoore2333/yokohama-assistant/pull/34). The close HEAD is `b16ccdd`,
and every count in §2 was measured there.**

The branch was cut from `web-room-m6.3` at `2194b57` (M6.3's shipped tip) and carries
**five planning commits** to `f53f6d7` and then **sixteen execution commits** — fourteen
through Task 8 at `40507c9`, the whole-branch review's two-commit fix wave at `117f3cf`
and `571c446` (which is what shipped as `0.5.0-m6.4`), and the live review's two-commit
fix wave at `6f76b02` and `b16ccdd` — plus the two deployment-record commits `ebd9911`
and `15d3ad3` and the doc-pass commit this record is part of. It is stacked:
`web-room-m6.3` is stacked on `web-room-m6.2`, itself on `web-room` (PR #31), and
**merge order is the owner's** — #31 → #32 → #33 → #34, nothing merged.

The authority for everything below is the spec,
`docs/plans/2026-09-16-web-frontend-m6.4-chat-spec.md`, under the eleven owner rulings
R-P1–R-P11 in `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` §0 and the spec's own
R-6.4.1–R-6.4.16. The plan `docs/plans/2026-09-16-web-frontend-m6.4-plan.md` with its
R-plan-1–R-plan-7 is what was executed. Both are frozen; where a sentence in either is
wrong at this HEAD, §5 and §7 say so here rather than editing them.

**This document is M6.5's record of what M6.4 built. M6.5's ENTRY artifact is
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-handover.md`** (§8).

**One thing to read before trusting any sentence this milestone wrote about itself:** §7,
and then §4.2. The final whole-branch review found TEN false or overstated sentences at
HEAD — doc paragraphs and cell titles describing a mechanism the code did not have — and
the fix wave corrected them. That is the same yield M2.5 (three), M3 (five) and M6.3
(eight) recorded, and it is the reason §7 exists.

---

## 1. What shipped

The room plan's M6.4 exit criterion is **"two players talk during a game and a third reads
it as a spectator."** **Both halves are shown, twice over.** In the whole-game gate's
two-identity arm three claimed identities post before the first move and every frame each
of them reads afterwards carries `chat == 3`; in the browser walk two people talk at a
table, a watcher reads both lines and answers, and the MOVER reads the watcher's answer
during their own turn without touching anything; and on the NAS, in the headed live review
of the deployed image, the same arc was photographed in both layouts with a line reaching
the mover's open panel in 26 ms (§4.5).

Everything §1 describes was verified at the close HEAD `b16ccdd` by reading the code it
names.

### 1.1 The documents — `crates/yokohama-web/src/chat.rs` (new) and `store.rs` (spec §2.1–§2.3, Task 1)

A new module holds the document and its rules: `CHAT_SCHEMA` 1 (`chat.rs:28`), `HOUSE`
(`chat.rs:47`), `ChatDoc { schema, id, last, messages }` with `empty(id)` and `trim(keep)`
(`chat.rs:50`, `:210`, `:216`), `Message { seq, created_at, author, text }` (`chat.rs:61`),
and `Author` (`chat.rs:79`) as an internally tagged union — `seat` | `watcher` | `bot` —
whose third arm is read, written and produced by **no route in this cycle** (R-6.4.15).
`ChatLimits` (`chat.rs:100`) defaults 500/500.

**`trim` drops from the OLDEST end and leaves `last` alone**, which is what makes a
client's `after` cursor survive a trim: a number is never reused, and a cursor taken
before a trim still means what it did.

`check_text` (`chat.rs:230`) trims and returns, bounds by CHARACTERS rather than bytes,
and refuses an empty text or any control character. `check_shape` (`chat.rs:250`) bounds
every stored field: the schema ceiling, strictly increasing `seq`, non-empty control-free
text, `User::parse` on every author address, `tables::check_name` on every author name,
the seat bound against the table's own player count (and no seat at all at the house,
where `players` is `None`), `last` at or above the newest `seq` — and, since the fix wave,
`created_at` at or below `MAX_CREATED_AT` (`chat.rs:39`, 253,402,300,799 = the last second
of 9999). **Length is deliberately not a refusal in either direction** (R-6.4.8): an
over-cap text is served as stored and an over-keep document is served and trimmed on its
next append, so lowering either number on a running volume cannot take a table out.

**The store's second reservation.** `Doc` gains `Chat` (`<id>.chat.json`) and `House`
(`store.rs:49`, `:54`), which share a suffix and are set apart by the underscore
`FsStore::path` prepends: `_house.chat.json` (`store.rs:220-226`). The underscore IS the
reservation — no id `is_safe_id` admits may start with one — so the house file needs no
suffix of its own. `RESERVED_ID_SUFFIX` became `RESERVED_ID_SUFFIXES: [&str; 2] =
[".table", ".chat"]` (`store.rs:157`) and `spells_a_table_document` became
`spells_another_document` (`store.rs:187`), folding over the list, still
case-insensitively. Both stores' `delete` walk `[Record, Table, Chat]` and never the
house.

**`check_house_id` is one helper called from three places** (`store.rs:200`, called at
`:221`, `:415`, `:434`), because `MemStore` keys `(Doc, GameId)` and asks only
`is_safe_id`, which `"house2"` passes — so without it the two implementations would
disagree about what exists (R-pre-2). The derivation assert now checks the two LENGTHS
before it zips (`store.rs:779-789`): `zip` stops at the shorter side, so a third suffix
added without a matching `Doc` would have been derived by nothing with the cell green.
That lengths-first ordering is what caught mutation 8 twice (§4.4).

### 1.2 The append and the counters (spec §2.4, §4, Task 2)

`tables.rs` gains one section. `read_chat` (`:829`) reads `Doc::Chat` at an id when
`players` is `Some` and `Doc::House` at `HOUSE` when it is `None`; an ABSENT document is
an empty one at version `None`, never a refusal, and a document that will not parse is a
typed `Server` failure. `write_chat` (`:861`) runs the shape rule on the way OUT and then
compare-and-swaps. **`check_shape` therefore runs on READ and on WRITE**, which is what
makes "every stored field is an input" true wherever the field enters.

**One private appender behind two public seams** (R-2-2): `append_locked` (`:904`) holds
the number-from-`last + 1`, the push, the trim and the write; `chat_append` (`:934`) takes
the TABLE's own lock and ends in `bump_table(id)` at `:951`; `house_append` (`:961`) takes
its own `house_lock` and ends in `bump_house_chat()` at `:965`. The spec's "one writer"
names the PUBLIC seam and does not forbid a private helper, and the two public signatures,
the lock discipline and the gated cells are unchanged by the extraction.

**The write is keyed by the key it READ and never by the document's own `id`** — a Task 2
review finding, and a real one: `doc.id` comes out of the file, `check_shape` never bounds
it, and keyed off it an append at table A read A's chat and wrote A's messages AND A's
version to B's file. The cell that pins it plants B's document with the SAME BYTES, so its
version (a content digest) matches what A's read handed the writer and a misdirected
compare-and-swap would SUCCEED rather than be refused.

**A bump follows its write and never precedes it**, proven and not asserted: two gated
cells park the real `Doc::Chat`/`Doc::House` write behind a store gate and read the
receivers QUIET at the one instant a hoisted bump would be loud — the M6.3 `FlipGate`
shape. And a table message wakes its TABLE and not the house: `chat_append` calling `bump`
instead of `bump_table` would put three listing reads per message on every visible lobby
(mutation 21).

**`House { seq, chat }`** (`signal.rs:42`) replaces the house channel's bare `u64`
(R-plan-2): `bump`/`end` move `seq`, `bump_house_chat` (`signal.rs:81`) moves `chat`, and
the two are independent. **`chat_last`** (`tables.rs:1015`) is the per-process count a
stream's per-bump closure reads, so a count on a frame costs no document parse per bump
per connection — with twenty watchers on a table the alternative is twenty parses of one
file for one message. Its cold seed uses `or_insert` (`tables.rs:1020`, the fix wave):
under a plain `insert` a miss that raced an append overwrote the newer number with the
older one, and a count-driven host holding N-1 would not fetch message N until N+1 landed.
A failed read answers 0 and caches NOTHING; an absent document answers 0 and is cached.

### 1.3 The switch and the limits (spec §5, Task 3)

Three launch variables in `chat.rs`: `YOKOHAMA_WEB_LOBBY_CHAT` (`:114`),
`YOKOHAMA_WEB_CHAT_KEEP` (`:116`) and `YOKOHAMA_WEB_CHAT_TEXT` (`:118`), parsed by
`ChatConfig::from_env` / `from_env_values` (`:143`, `:159`) into `ChatConfig { lobby,
limits }` (`:131`). **Unlike the identity variable these HAVE defaults**, and the type's
own doc says why: an identity source is a decision about who may act, so an untold server
refuses to start; a retention number has a sane unwritten answer. A BAD value still exits
1 naming its own variable, because a server started with `CHAT_KEEP=lots` silently meaning
500 is a server whose operator believes a number that is not in force. `0` is refused for
both numbers — two ways to configure a chat that works and says nothing.

`AppState` gains `chat` (`lib.rs:270`) and `Health` gains `lobby_chat`, `chat_keep`
(null for `all`, because a number would be a lie) and `chat_text` (`lib.rs:355-360`), so
the client sizes its input from the SERVER's rule and never from a copy.

**The defaults are bound rather than restated** (R-3-2): the parser's absent arms read
`ChatLimits::default()` and a cell asserts `from_env_values(None, None, None).limits ==
ChatLimits::default()`. The two-step failure it guards against is the one this lane hunts
— a health cell goes red on a changed number, the developer fixes that number, and the
stale parse arm survives.

### 1.4 The routes and the admission (spec §3, Task 4)

Four routes (`lib.rs:2129`, `:2138`): `GET`/`POST /api/games/{id}/chat` and `GET`/`POST
/api/chat`. The reads are admitted by `admitted` exactly as any read is; the POST's order
is admission → the text rule → the seat/name rule → the append. `ChatPost` and `ChatQuery`
(`lib.rs:866`, `:876`) carry `deny_unknown_fields` and extract through the crate's own
`JsonBody`/`QueryArgs`, so `?afer=0` is a 400 in this API's shape on both routes.

**`author_of`** (`lib.rs:951`) reads a seat's name off the TABLE document and refuses a
body name with "a seated caller is named by their seat" — so nobody can post as another
seat — while a spectator's name goes through `tables::check_name` and carries that
function's own sentences. **`lobby_chat_on`** (`lib.rs:981`) is the house switch: off, both
house routes are 404 "the lobby chat is off", the counter never moves and the client draws
no panel.

**`Events` gained its fifth field**, `chat` (`lib.rs:1710`), on all three arms including
`open` and `dealing`; `events_at` reads `chat_last(id)` ONCE and passes it to each. The
house frame became two counters, `{"seq": N, "chat": M}`.

**A watcher's post is R-3.6's one named exemption, and the sweep names it**: the write
sweep runs behind a store that PANICS on any record read, and grey's `POST …/chat` is the
one write that answers 201 as a `Spectator` — "before the handle is touched" is a crash,
not a sentence. The admission grid gained `Kind::Chat` and `Kind::ChatPost` and asserts
**POST ≡ GET at every cell**, so a locked table's chat is behind its password both ways;
the stranger sweep gained the two rows, which is what keeps M6.2's open-table widening
from spreading to the messages. **`chat_read`'s doc says what it does and what it does
not** (the Task 4 fix round): the chat is `admitted`'s rows and NARROWER than the widening
`table` and `events` carry, so a stranger on the seat picker reads the table and the count
and is refused the messages.

### 1.5 The wire (spec §8, Task 5)

The corpus went **71 → 82** (§3). The walk gained a chat block before the move loop
(R-plan-6) — three posts, so `events.json` carries a real count rather than a constant —
and a FIFTH identity, `quiet@wire.test`, who unlocks the locked table and never speaks.

**The email pin narrowed to its true statement** (R-6.4.12) and became a TREE walk rather
than a text search, because the allowance is about WHERE a hit is and a text search has no
position: `a_silent_watcher_is_named_nowhere_and_a_posting_one_only_as_an_author` allows
`green@wire.test` only at an `author.user` whose SIBLING `kind` reads `"watcher"`, and
allows `quiet@wire.test` and `grey@wire.test` nowhere at all. It is no weaker than the
search it replaces: a string VALUE containing the needle is a hit, and so is an object
KEY. Three anti-vacuity partners close three different holes — the allowance is asserted
EXERCISED (`allowed_hits >= 2`), the M6.3 partner asserting green IS in the table document
is kept verbatim, and a new walk-side premise assertion reads QUIET's address off the
store, because without it the silent-watcher clause is a search for somebody who might not
be watching at all.

`every_chat_message_carries_the_frozen_clock` reads the six committed chat files and
asserts every `created_at` equals the frozen constant, with the count asserted at EIGHT as
anti-vacuity. It is a property of a NAME — the freezer pins by key — so nothing about the
type can hold it.

### 1.6 The client (spec §6, Tasks 6 and 7)

**`web/src/Chat.tsx` is ONE component on four hosts**, and it owns its list, its cursor,
its fold, its unread count, its draft, its in-flight post and its error line. `when`
(`:54`) is `HH:MM` today and the date before it; `isoStamp` (`:68`) returns `undefined`
rather than calling `toISOString()` on a value `Date` cannot parse, which is half of the
fix wave's Important 1. The label is R-6.4.2's three arms: a seat's colour and its stored
name or address, a watcher's name under a "· watching" marker **at a table only**, and the
defensive bot arm.

**One door to the fetch, and a coalescing rule behind it.** `read(why)` holds a `reading`
flag: a trigger arriving while a read is out is neither dropped nor sent but REMEMBERED,
and when the answer lands `read` re-asks only if the reason still stands — a tick's
unconditionally, since the delta is that host's only source, and a count's by re-reading
the count rule against the cursor the answer just moved. So a remembered-open panel whose
count is already above the nothing it holds issues exactly ONE `GET …/chat?after=0` on
mount (R-pre-4). A `generation` ref (`:104`) moves FIRST on a scope change, so a read — and,
since the fix wave, a POST's 201 (`:292`, `:299`, `:303`) — still out for the room the
panel has left is dropped instead of appended to the new room and moving its cursor past a
foreign `seq`.

**`web/src/chatFold.ts` is ONE localStorage key**, `yokohama.chat.open` (`:7`), holding the
set of scopes this browser has opened, newest 64 — the `sections.ts` idiom, folded by
default everywhere and folded again if the storage throws (R-pre-1 chose the spec's key
over the plan's `yokohama.chat.folded.<scope>`).

**The four hosts**: the game screen (`App.tsx:1390`, keyed by the game id, LAST inside
`aside.side`), the dealing screen (`Dealing.tsx:368`) and the open table
(`OpenTable.tsx:287`, only when seated), each reading the count off the frame or ticking
on their wake, and the lobby's fourth section (`Lobby.tsx:313`), rendered only under the
switch.

**The stream is now held at every visible status** (R-6.4.4): `useSignal`'s `active`
became `!!state` (`App.tsx:823`), from `!!state && !state.over && !state.your_turn`. **The
cost is one guard**: `send` marks itself `posting` and the arrival handler parks any frame
that lands meanwhile (`App.tsx:596`), re-comparing it after the reply has been applied, so
a self-bump is never walked as an opponent's move. M6.3's Minors M-1 and M-3 close as a
consequence, and the live review measured the abandon reaching a holding mover's screen in
54 ms (§4.5).

**The name at the door** (R-6.4.3): a "Your name" field in the lobby's who-strip and on the
`claimed` sign-in, both writing the ONE key the table forms already use
(`yokohama.display_name`), so the name a person joins under and the name they talk under
are one string. The help panel gained a **Talking** section, a sentence in the watching
paragraph, and — under R-7-1, the spec's third edit the plan omitted — one sentence in the
lobby's own paragraph for the name field.

### 1.7 The gates (spec §9.4, Task 8)

**The whole-game gate's two-identity arm talks.** Before the first move the watcher posts
with a name (201, `author.kind == "watcher"`), the two seats post without one (201, each
`("seat", n)`), a cursored read returns the name the JOIN stored on the table, all three
identities read the whole room, and each one's `GET …/events` carries `chat == 3`.

**The browser walk's three identities talk.** In `two_browsers.spec.ts` A says, B reads, B
says, C reads both, C says and is MARKED as a watcher — the needle is `"· watching"`, the
separator inside it, because the corpus text itself carries the word — and then the MOVER,
chosen by `hasMove` with the premise asserted, reads the watcher's line **during their own
turn**. That last assertion is the one that cannot pass under the old transport: under
`active`'s `!your_turn` term the mover's panel never learns and the wait fails.

**The players' no-reopen counters are read from the CREATE, not from the watcher's block**
— a deliberate deviation, and a measurement rather than a preference. Both players' game
screens are already up where the plan put the baseline, so a read there is `0 === 0`, true
of a correct product and equally true of a listener watching the wrong path. Assigned at
the create it reads **a 2, b 2** (the dealing screen, then the game screen), unmoved from
before the first move to the end of the game.

### 1.8 The fences held

`crates/yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge` and
`gen-data` are **byte-identical to `master`** at this HEAD; `SEMANTICS_REV` is **7**
(`semantics.rs:180`); no bot behaviour moved. **`crates/yokohama-cli` is byte-identical to
`2194b57`** — untouched this cycle, no `pub` needed — and its diff against `master` is
M6.1's R-4.1 and M6.3's R-3.8, +60/−6 over two files, from the earlier cycles. The nine
`assert_capture` goldens and the eighteen `guide_sync` cells are how that is known rather
than believed.

---

## 2. The counts at the close HEAD

Measured at `b16ccdd` in **ONE uninterrupted sequence**, 00:24:57–00:36:00Z on 2026-09-20,
on the **Mac** (darwin 27.0.0, toolchain 1.97.1), with the BGA captures present so the CLI
figure is a FULL reading, tree clean before and after. Nothing below is copied from a task
report. The log is `.superpowers/sdd/2026-09-16-web-frontend-m6.4-plan/counts-at-close.log`
(the SDD workspace is gitignored; the figures are quoted here).

| suite | at the open (`f53f6d7`) | at this HEAD (`b16ccdd`) | wall |
|---|---|---|---|
| `cargo test -p yokohama-web` | **191** (181 lib + 2 gate + 8 wire) | **230** (**219** lib + 0 main + **2** gate + **9** wire) | 56 s; gate 25.72 s, wire 12.24 s |
| `cargo test -p yokohama-cli` | 851 + 2 ignored | **851 + 2 ignored** (823 lib + 9 `bga_import` + 18 `guide_sync` + 1 `piped_dogfood`) | — |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | — |
| `cd web && npm run typecheck` | clean | clean | — |
| `cd web && npm test` | **587** in **35** files | **634** in **37** files | — |
| `cd web && npm run e2e` | 39 in 8 spec files | **39** in **8** spec files | **9.2 min** (554 s, release build included) |
| `ls web/src/wire/*.json \| wc -l` | **71** | **82** | — |

**Total wall 663 s**; disk 46 GiB free after; `web/test-results` 4 KB, so no failed run
wrote a trace (the M6.3 owner item did not bite). **The whole-game gate passed BOTH arms**
in this sequence, in 25.72 s — the vs-BOT arm's livelock did not fire here, and it fired
in none of this cycle's readings, which is what an intermittent OS-entropy defect looks
like rather than evidence it is gone (§6.3).

**The lib grew by 38** (181 → 219): Task 1's ten (three store cells, seven chat cells),
Task 2's eleven plus its fix round's write-key cell, Task 3's two, Task 4's six, and the
fix wave's eight (the clock bound; the gated cache race, the failed count read and the
unparseable document; the hot seat, the 409, the query fence and the house name-and-switch
order). `wire_fixtures` went 8 → 9, the frozen-clock cell. The client grew by 47 (587 →
634) across **two new files** — `Chat.test.tsx` at Task 6 and `chatFold.test.ts` in the fix
wave — with the rest in the hosts; the second fix wave's single cell is the 634th.

**The browser walk's own line**, from the same sequence:

```
two-browser game finished in 2,270 moves, 0 idle looks, 457s; the watcher was held to
1,704 applied moves, asked /events 0 times and opened /stream 1 times; the players
opened this table's stream a 2, b 2 times
```

The vs-bot walk ran 168 moves and the join-by-link cell 6 moves in 2 s. **Read the wall at
its real size**: at `ebd9911`, before the second fix wave, the same cell read **282 s over
2,158 loop iterations**. The wall tracks how long a seeded game ran — the server draws the
seed — and the figure that means something is the **0 idle looks**, unchanged. **No wall
here is a budget** (§7).

**The blast radius**, at this HEAD:

```
git diff --stat master -- crates/yokohama-core crates/yokohama-oracle \
                          crates/yokohama-difftest crates/yokohama-bridge crates/gen-data
  (empty — 0 files)

git diff --stat 2194b57 -- crates/yokohama-cli
  (empty — 0 files)

SEMANTICS_REV = 7   (crates/yokohama-core/src/game/semantics.rs:180)
```

**The fifteen-gate release roster was NOT re-run**, for the reason the M6.1, M6.2 and M6.3
closes gave: every crate the roster measures is byte-identical to `master`, so a re-run
could only re-measure master's own reading. Nothing added in this cycle is `#[ignore]`d;
the whole-game gate stays out of CI and per-edit locally.

---

## 3. The corpus diff

`web/src/wire/` is a contract, not a snapshot. It was regenerated at Tasks 3, 4 and 5 —
one regeneration per shape-changing task under R-3-1, rather than the plan's "Tasks 4 and
5 regenerate", which would have left `wire_fixtures` red between Task 3 and Task 5 — and
every diff was read. **71 files at the open, 82 at the close.** Neither fix wave
regenerated anything: both ran `wire_fixtures` WITHOUT `UPDATE_WIRE_FIXTURES` and it
passed, which is the stronger statement.

| where | change | why |
|---|---|---|
| Task 3 | `health.json` gains `"lobby_chat"`, `"chat_keep"`, `"chat_text"` | the client sizes its input from the server's rule (R-6.4.8) |
| Task 4 | `events.json`, `events_open.json`, `events_dealing.json` each gain `"chat": 0` | the fifth field on all three arms |
| Task 5 | **ELEVEN NEW**, and `events.json`'s `chat` 0 → 3 | the chat block precedes the walk loop (R-plan-6), so the first-sighting capture reads a real count |
| both fix waves | none | the corpus is byte-identical to `40507c9` and to `ebd9911` |

The eleven: `chat.json` (three messages — seat 0 "Red", seat 1 "Blue", watcher "Green"),
`chat_after.json` (`?after=2`: one message, `last` still 3 — the document's, not the
page's), `chat_post.json` and `chat_post_watcher.json` (the two author arms as 201s),
`chat_400_long.json` and `chat_400_seated_name.json` (the two 400 sentences the client
renders), `chat_403_not_seated.json` (green at the OPEN table — the one status where
`…/table` and `…/chat` visibly part company) and `chat_403_locked.json`,
`house_chat.json` (one WATCHER author whose address is a player's, because the house has
no seats), `house_chat_post.json`, and `house_chat_off_404.json` (captured from a second
`Api` built with the switch off).

**Every `ClientState`, `HistoricalFrame`, `TableView` and `DealView` fixture is
BYTE-IDENTICAL**, which is what proves no chat field entered those payloads. That the
panel is fed by the frame instead is `events.json`'s own key and the component's count
cells.

The client's `contract.ts` gained eleven imports and entries, six `Widen<ChatPage>` /
`Widen<ChatMessage>` assignments and five `Widen<ApiErrorBody>` ones, the §1c per-member
author rows, and `KeysOf` rows for `CHAT_PAGE`, `CHAT_MESSAGE`, `CHAT_AUTHOR_SEAT` and
`CHAT_AUTHOR_WATCHER`; `contract.test.ts` gained the per-`kind` runtime cell and five
key-root rows for the refusal bodies. `EVENTS` and `HEALTH` needed no change at Task 5,
because Tasks 3 and 4 made the two-line client change in the same commit as the server
field (R-3-1), so vitest's root-key rows never went red between tasks.

---

## 4. The reviews

Eight implementer dispatches, eight per-task Opus reviews, **five fix rounds** (Tasks 2,
3, 4, 6, 7), every fix re-reviewed in scope; one Fable whole-branch review; one fix wave
with its own scoped re-review; one mutation battery; and — at the wrap-up — one HEADED
Fable live review against the DEPLOYED site with a second fix wave, its re-review and a
point release, plus a Claude-in-Chrome pass under the owner's own profile. Every report is
in `.superpowers/sdd/2026-09-16-web-frontend-m6.4-plan/` — **the SDD workspace is
gitignored**, so the findings are quoted here rather than linked to.

### 4.1 The per-task reviews

| task | verdict | C/I/M | fix round |
|---|---|---|---|
| 1 — the documents | spec ✅, three deviations judged improvements | 0 / 0 / 6 | 0 |
| 2 — the append | spec ✅ | 0 / 2 / 6 | 1 (`46d3b7d`) |
| 3 — the switch and the limits | spec ✅ | 0 / 1 / 4 | 1 (`3bcb046`) |
| 4 — the routes | spec ✅, three deviations confirmed | 0 / 1 / 6 | 1 (`cc2c9ae`) |
| 5 — the wire | COMPLIANT | 0 / 0 / 4 | 0 |
| 6 — the panel | spec ✅, the `settle()` deviation kept | 0 / 3 / 10 | 1 (`dece144`) |
| 7 — the hosts | compliant | 0 / 1 / 4 | 1 (`b8324c7`) |
| 8 — the gates | clean, the `streamPath` deviation judged strictly stronger | 0 / 0 / 3 | 0 |

The four worth carrying:

- **Task 2 (Important 1 → R-2-2):** the write keyed off the STORED `doc.id`, which
  `check_shape` does not bound — an append at one table would have written its messages
  AND its version to another's file. §1.2 has the cell and why it is discriminating.
- **Task 3 (Important 1 → R-3-2):** the plan mandated writing `500` twice, in the parser's
  absent arms and again in `ChatLimits::default()`. A tested default drifting from the
  production default is a silent defect of exactly the kind this lane hunts, so the two
  are bound and pinned equal.
- **Task 6 (Important 1 → R-6-1):** the plan's own `queueMicrotask` scroll block runs
  BEFORE React commits, so "kept at the bottom on append" could never work. The spec's
  rule is the requirement and the plan's code failed it: "at bottom" is read before the
  append and `scrollTop = scrollHeight` applied after the commit, in an effect keyed on
  the messages. jsdom can see neither, so the browser walk and the live review are the
  confirmation.
- **Task 7 (Important 1 → R-7-1):** spec §6.6 binds THREE help edits and the task brief
  carried two. The third — one sentence in the lobby's paragraph for the name field — was
  written, because a help panel silent on a cycle's user-facing field is what a live review
  flags.

One process finding, restated because it cost round trips in M6.3 too: **crossed
messages**. The rule held this cycle — every fix landed as a NEW commit on top of the
reviewed SHA, never an amend, at all five rounds and both waves.

### 4.2 The whole-branch review (`final-review`, Fable, on `f53f6d7..40507c9`)

Fourteen commits, forty-eight files, +4580/−260, read in three passes, with the spec, the
plan's constraints and the whole ledger read first. The reviewer ran six focused lib cells,
five vitest files, a corpus and blast-radius check, and **tried each mechanism in a
detached scratch worktree** rather than re-reading the sentence about it. Verdict **"ready
to merge WITH FIXES"**: **0 Critical, 5 Important, 14 Minor**, **ten false or overstated
sentences at HEAD**, and a triage of the 39 deferred minors into **4 MUST FIX / 15 THIS
WAVE / 20 LEAVE**. It read the rulings and judged all of them sound.

The five Importants, all fixed in the ONE fix wave:

1. **`created_at` was the one stored field the shape rule did not bound, and an
   out-of-range value crashed the panel's render.** `toISOString()` THROWS above 8.64e15
   ms; a probe rendering the panel over a page whose one message carried `created_at: 1e19`
   threw out of React's render, and there is no error boundary — so a hand-edited or
   half-restored document took down the whole game screen for every browser at that table.
2. **The cell claiming the unfold's reset could not see it.** It drove a COUNT host, where
   the badge is `count - last` and the `unread` state is never read; removing the reset left
   123 of 123 cells green.
3. **On a wake host the MOUNT read's whole history counted as "new", every visit** — the
   lobby's folded header read "N new" for up to 500 retained messages on every visit, while
   the count hosts showed 0 for the same messages, so the two kinds of host disagreed about
   the first screen.
4. **A POST resolving after a scope change landed in the new room and moved its cursor past
   a foreign `seq`**, after which every new-room message numbered below it was filtered out
   until the numbering passed it.
5. **The house counter's purpose was described by three sentences and a struct doc that
   named a lobby which does not exist at HEAD** — one that compares both counters and
   refetches only the half that moved. The lobby is a `"wake"` consumer, drops the frame,
   and refetches all three listings on ANY house frame.

### 4.3 The fix wave (`117f3cf` server, `571c446` client)

Forty-three items — the five Importants, the ten sentences, the four MUST FIX and the
fifteen THIS WAVE, plus thirteen of the fourteen Minors. **42 of 43 addressed**; the one
not taken is minor 19, which is the review's own LEAVE call (§6.2). Every Important carries
a named RED-before-fix line and a mutation: restoring either clock guard fails the client
cell; removing `setUnread(0)` fails the new wake-host cell; `take(page)` without the mount
term fails its first assertion; removing the generation capture fails the dropped-post cell
twice.

**Two coverage cells were GREEN ON ARRIVAL** and the wave says so rather than burying it —
`chat_last`'s `Err` arm and the unparseable document pin behaviour that was already
correct. Both carry their own anti-vacuity (a spent-arming assertion, and a following read
that gets it right), so neither is a cell that cannot fail; they are coverage the report
asked for, not a defect found.

**One judgement call, stated plainly:** the report says to drop a post's 201 after a scope
change and clear `sending`; it says nothing about the DRAFT, and the wave left the text in
the box. The message did land in the other room, and clearing it would take the words from
someone who may want to say them again.

### 4.4 The mutation battery (`mutation-battery`, Opus, at `ebd9911` — code identical to `571c446`)

Spec §9.5's twenty-two. **22 of 22 CAUGHT; none survived.** One at a time in a detached
scratch worktree, applied by a scripted edit, `git status --short` asserted EMPTY between
each, the named cell re-run green before the next, the first failing assertion recorded
verbatim, nothing committed, the worktree removed and the main checkout never written to.
The catchers span the whole branch: both hoisted bumps fail their gated cells; a spectator
posting at a locked table fails the admission grid by name, table, route and status; a seat
name taken from the body, or served as `null`, fails the by-value cell; `chat` dropped from
the `open` arm, the switch ignored, a `.chat` id admitted, a `seq` reused, a control
character accepted, the fold not read, the panel fetching on an unmoved count, the panel
opening a connection of its own, the lobby section drawn under `lobby_chat: false`, `seat:
7` at a 2P table, a stored over-cap text refused on read, `active` regaining its old terms,
the posting guard removed, the watcher's address on a payload, and `bump` in place of
`bump_table` — each caught by the instrument written for it.

**`npm run e2e` was not run**, by the battery's own cheap-cells-first rule: mutation 11 is
the only row naming a browser read and its component spy cell caught it.

**Five readings this close carries because a re-run cannot produce them:**

1. **Mutation 18(b)'s NAMED cell did not fire; a sibling did, and the reason is a real
   asymmetry.** The spec's §9.3 lists both a COUNT-host cell and a wake-host cell for the
   unread count, and deleting the reset leaves the count-host one green. That is not a gap:
   the badge is `count - last` whenever a count is supplied and reads the `unread` state
   ONLY on a wake host, so on a count host the reset is dead code and no cell over that host
   could see it move. Recorded because the spec's parenthesis reads as though either cell
   would do.
2. **Mutation 20's runtime half did not fire, for the structural reason the M6.3 close
   recorded about its own mutation 8.** The compile-time `KeysOf` row fired and named the
   field; the runtime cell reads the COMMITTED corpus and the battery deliberately did not
   regenerate it, so that half fires at regeneration time, as designed. A second,
   unasked-for catcher also fired: `Chat.tsx:303` stopped compiling, because the panel
   really does read that field.
3. **Mutation 9's catcher is one layer deeper than "the trim cell".** Setting `last` back
   inside `trim` does not fail an assertion IN the trim cell; it fails the APPEND, because
   `write_chat` runs the shape rule on the way out and the `last < prev` clause refuses the
   document. A reused number cannot be WRITTEN at all, not merely noticed afterwards.
4. **Mutation 8 is caught TWICE, by two checks that are deliberately not the same one** —
   the id cell, and the derivation loop's LENGTH assert (`left: 1 right: 2`). The assert
   CLAUDE.md warned "would not notice a third kind" does notice a kind REMOVED, which is
   what the lengths-first ordering bought (§1.1).
5. **Mutation 3 needed a NARROWED edit to stay faithful.** "A spectator posting at a locked
   table" could have been realised by making `admitted` admit everyone, but that moves the
   GET side too and the grid asserts POST ≡ GET — both arms would have moved together and
   the cell would have failed for a reason that is not the mutation's. The edit applied is
   local to `chat_post`.

### 4.5 The live review against the DEPLOYED site, HEADED (`live-review`, Fable, 2026-09-19)

R-6.4.16's headed verification, run under the owner's 2026-09-19 instruction after its own
2026-09-17 removal was superseded by its author (§5). A clean Fable subagent with no part
in the build drove **three HEADED Chromium browsers as three people** — Ada and Bo playing
at a password-locked 2P table, Cy arriving by link and watching — against
`https://yokohama.moore.nyc` running `yokohama-web:0.5.0-m6.4`, in both layouts (desktop
1440×900; mobile 390×844 emulated, DPR 2, touch), each browser its own process, every
request counted off each page's own request log and every visual claim read BY EYE from the
photographs. The walk passes through all four hosts in one flow. **76 photographs in the two
counted runs**, 36 more from an aborted first mobile run and 5 from a one-browser mobile
probe, under `live-review/shots/`.

**Verdict: SHIP WITH FIXES — 0 Critical, 1 Important, 4 Minor.** Ten claims were tried in
both layouts and found TRUE except claim 9 (zero console errors), which is false by exactly
one pre-existing `favicon.ico` 404 per screen and by nothing in the application.

**The exit criterion, on the NAS.** A line drawn on the MOVER's open panel during their own
turn in **26 ms** (desktop) and 25 ms / 2 ms (the two mobile runs), with the mover's prompt
asserted on screen before and after; the folded badge moving in **1–3 ms**; a house line
reaching another browser's folded lobby badge in **77 ms** desktop and 185 ms mobile; the
watcher's line on the mover's panel in 1 ms; a line still arriving after two turn
boundaries (24 / 22 ms). **ONE table stream per player** from the first game frame through
both turns (A 1, B 1, in both layouts), the watcher exactly one from Watch with zero aborts,
and `GET …/events` **0 / 0 / 0** over the whole session. Page overflow was 0 px on all
twelve photographed screens in each layout, a 495-character line wrapped with nothing
overflowing, and the tabs were `visible` at every applied move.

**The five findings**, all reproduced:

1. **I-1 (Important, mobile) — at 390 px a long label squeezed the message into a column
   eight to eleven characters wide.** The row is `auto auto 1fr` with a `nowrap` label:
   under the ADDRESS label every line a person who typed no name carries, the tracks
   resolved to `103.5px 48px 158px` (22 characters per line), and a watcher's `· watching`
   marker costs 76 px more (11 per line). It clips nothing and overflows nothing.
2. **M-1 (Minor, mobile) — the who-strip wrapped the address over three lines and `sign
   out` over two**: M6.4's own "Your name" field joined a `flex` row with no `flex-wrap`,
   making the first screen's top line a 51 px block. This is the Chrome pass's CC-1.
3. **M-2 (Minor, both layouts) — every own post on an OPEN count-driven panel cost one
   delta GET**, because the table's bump reaches the poster's own stream BEFORE the 201
   lands: each POST was followed within 42–260 ms by a `?after=<n>` one below the posted
   number. No line was ever drawn twice, so the cost is one small request — **and one false
   sentence in spec §6.1**, whose "the frame it causes is a no-op compare" is not true on
   the live tunnel, where M6.3 had already measured the bump landing inside the mover's
   round trip.
4. **M-3 (Minor, both layouts) — the setup screens' heading rules leaked into the panel.**
   `.setup h2` is declared after `.panel h2` and wins on source order, so the open table's
   heading read a large letter-spaced CHAT where every other host draws 10 px mono small
   caps; `.setup h3` gave the lobby's folded box 26 px of dead space.
5. **M-4 (Minor, PRE-EXISTING, both layouts) — a `favicon.ico` 404 on every screen's first
   load.** **Headless Chromium never requests a favicon, which is why every earlier review
   in this lane read "zero console errors"** — this was the first headed one. It was the ONLY
   console error of the whole walk.

**Observations, not findings.** M6.3's M-3 suspicion does **NOT** reproduce under the held
stream: with the mover holding the prompt, the creator's delete sent all three screens to
the lobby with "this table was abandoned" in **54 / 56 / 60 ms**. M6.3's M-2 is still there
(an arrival by link draws the lobby for one round trip — LEAVE). The "chips" are coloured
TEXT, which is what the help promises. The fold is remembered per browser PER TABLE across
statuses. Retained history does NOT count as unread — the watcher arrived at a table holding
six lines with no badge. And below 1201 px the open panel's lower rows sit under the sticky
deck until the page is scrolled to its very end, which is the stacked layout's general
property and the spec's own words.

**Not reachable with this instrument**: a real phone or Safari; the hidden-tab word, as
Playwright cannot hide a tab; a bot's chat line, which no route produces; the retention
trim at a keep of 500; the dark held arm; game over at a table of two PEOPLE (the walk was
two turns); and load beyond two open lobbies.

**All five were fixed, re-reviewed and shipped.** The second fix wave (`6f76b02` client,
`b16ccdd` server doc sentences) took I-1 with a **CONTAINER query** rather than the
review's viewport one — `container-type: inline-size` on the messages box
(`styles.css:1381`) and the text on its own row below 34rem (`:1408`) — **because the same
defect was photographed at DESKTOP width in the side column**, which is ~322 px at 1280 and
at 2560 alike, so no viewport threshold separates it from the ~800 px setup panels. The
picture the media-query version left behind is kept beside the report. M-1 is
`flex-wrap: wrap` under 480 px (`:1444`); M-2 parks a count trigger while a post is in
flight and re-checks it in `send`'s `finally`, but only when no read is out, because a count
parked behind a READ is that read's tail's to consume; M-3 restates the panel's own
small-caps set on `.chat h2, .chat h3` and moves the lobby section's 1.6 rem to the section
(`:1366`); M-4 is `<link rel="icon" href="data:," />`. Each carries a cell with a verbatim
RED except M-4, which **cannot have one and says so** — headless Chromium never asks for a
favicon — and M-3's lobby half, which is checked by eye. The scoped re-review returned
**SHIP: 8/8 addressed, 0 open, no new breakage at any severity**, with both judgement calls
ACCEPTED on their evidence (§4.6). The point release `0.5.0-m6.4.1` went live at 00:38:01Z
and was verified through the tunnel, including a chat check at a throwaway table where
R-6.4.2's fence answered live: a seated caller's post carrying a name was refused **400 "a
seated caller is named by their seat"**, and without it 201.

**The headed RE-CHECK against the point release — the same reviewer, the same scaffold,
both layouts, `live-review-recheck-report.md` with 81 photographs under
`live-review/shots/recheck-*/` — read all five FIXED and found nothing new.** I-1: the text
takes its own full-width row under every narrow-panel label — **322 px / 46 characters per
line at 390** and **315 px / 45 in the 1440 side column** under an address label, "Cy ·
watching" and "Cyrus the Watcher · watching" alike (against 158 px / 22 and ~60 px before),
while the lobby's 763 px panel keeps one row (675 px / 97). M-1: at 390 the address on ONE
line (17 px), `sign out` on one (31 px), the name field on its own row, the strip 62 px;
1440 unchanged. M-2: **0** `GET …/chat?after=` on the poster's own log after each of three
solo posts on open game panels, both layouts (one per post before). M-3: the open table's
CHAT heading computes to **10px** uppercase mono (22.4px before); the lobby's folded box is
the game's 49 px box with the header 12 px below its top, the 26 px of dead space gone. M-4:
`link[rel=icon]` served and **zero console errors on all three screens in both layouts**.
Everything else re-read the same — one stream per player across both turn boundaries, the
watcher one, `/events` 0, page overflow 0 on every photographed screen, an abandon reaching
all three screens within 65 ms with the mover holding the prompt. Two tables were created
and deleted; one house line was posted ("live review 0.5.0-m6.4.1: hello from Ada
(desktop)"), which cannot be removed (§6.3 item 4).

### 4.6 The scoped re-reviews

**Of the first fix wave** (`rereview-fix-wave`, Opus, over `40507c9..571c446`, read-only,
no suite run): **ALL ADDRESSED, 0 open** — 42 of 43 findings fixed and verified at HEAD,
the one not taken being the previous review's own LEAVE call. No new Critical or Important
breakage. It ran a vacuity pass on every new or changed cell and traced which assertion
would actually fire — in particular that the RE-FOLD assertion, not the unfold one, is the
discriminating half of the new wake-host cell, since a hidden badge is null either way.
**Three wording notes** came out of it: the new `or_insert` doc said the entry "never
arrives from anywhere but an append" four lines above an `or_insert` that is itself an
entry-filling path; `append_locked`'s doc carried the same "before it took its lock"
overstatement the wave had just corrected at the route; and an unparseable chat document
reads 0 AND caches 0 for the life of the process, which the new cell pins only the first
half of. All three were fixed in `b16ccdd` with the second wave.

**Of the second fix wave** (`rereview-fix-wave-2`, Opus, over `ebd9911..b16ccdd`,
read-only): **SHIP — 8/8 addressed, 0 open, no new breakage.** It read the photographs
itself rather than the report's description of them, re-derived the 34 rem threshold from
the stylesheet (the side column is 322 / 418 / 514 px at `--side` 24 / 30 / 36 rem, always
under 544; the setup hosts are capped at 52 rem, always over — so 34 rem sits in a gap with
no host in it), checked what `contain: layout style` can break inside the messages box, and
confirmed the corpus and blast radius independently. It separates **witnesses from guards**:
M-1's sign-out assertion did not go red in this walk — the spec's address is shorter than
the live one — and is a regression guard, which the fix report had already said.

### 4.7 The Claude-in-Chrome pass (the controller, 2026-09-19)

R-6.4.16's second half, the owner's-eye read: the owner's own running Chrome, already
through the Access door, signed in at the claimed door as the owner's email, reading the
lobby, the game screen and the New-game form at 1440 and at 500 px. **Nothing was posted, no
move was made**, and the session signed out afterwards. **Instrument limit, stated first:**
Chrome on macOS refuses a window narrower than 500 px, so the "mobile" readings here are at
500×701 — the 390 px reading is the Playwright seat's.

The panel, its composer, the prefilled name and the chips all FIT at both widths with page
overflow 0. The game panel is the side column's LAST child, reached by scrolling the column;
in the stacked layout it sits above the sticky deck, and a reader who stops one screen short
of the page's end sees the deck rather than the panel. **Two Minors, both INHERITED and both
LEAVE**: **CC-1**, the identity strip's `sign out` label wrapping to two lines and "Your
name" splitting from its input at ≤500 px (M6.1's surface; the same row as the live review's
M-1, which the second fix wave fixed at 390 px — see §6.2 for what remains); and **CC-2**,
the New-game form's two placeholders truncating at ≤500 px (M6.2's form, cosmetic).

---

## 5. The rulings taken in execution, with their costs

**The owner's own words first**, quoted verbatim from the ledger, in the order they were
given.

- **Owner authorization, 2026-09-17, in-session:** *"when we get to task 9, just note
  you've got authorization to build and deliver"* — so the release was built and delivered
  without a further check-in. The PR and the merge order stayed the owner's, and were not
  part of it.
- **Owner ruling, 2026-09-17, in-session:** *"for task 10, I want to put aside the deeper
  fable agent walkthrough where it goes through headed, I'll go through that myself, run
  through the rest of plan 10, but unless you disagree I want to take out that headed live
  review since task 7/8 will have done it headless, to help us finish without going through
  a token intensive screenshot driven context - but in the wrapup and handover documents we
  can leave that as something we can consider reviewing before M6.5."*
- **Owner ruling, 2026-09-17, in-session:** *"we are likely to run out of usage before this
  completes, so task 9 if we make it there with this fix wave that's already started I'd
  like to get it up as quick as I can, and we will resume this chat when api access is
  restored in a few days, if the fix wave has breaking items then we will just have to wait
  and if it fails before the usage runs out then it will just have to wait"* → R-order-2.
- **Owner instruction, 2026-09-19, verbatim:** *"I'd like to run through the wrap up of M6.4
  … it can include the playwright or claude-in-chrome claude-led headed runthrough of M6.4
  previously skipped to check for visual fitting as well as prefilled text fitting etc. It
  can run through the close, and then after the M6.5 handover and PR against web-room-m6.3
  it can be pushed. Any changes made as part of the final review process here can be
  deployed to the NAS without additional authorization."*

**The 2026-09-17 removal of the headed review was SUPERSEDED by its author on 2026-09-19,
and the review was run** (§4.5). Under R-P11 that is a deviation recorded and then
reversed, and both halves are in this record rather than only the outcome: R-6.4.16 was
amended by the owner and then restored by the owner, and what the close would otherwise
have carried into the LEAVE list as "consider before M6.5" is instead §4.5's evidence and
a point release.

**Twenty rulings** were taken by the orchestrator, in the order they were taken. **Cost** is
what is wrong if the ruling is wrong. The whole-branch review read them and **judged all of
them sound**, with two notes rather than disagreements (below).

| # | task | ruling | cost if wrong |
|---|---|---|---|
| R-pre-1 | T6 | The fold is ONE key `yokohama.chat.open` holding the set of opened scopes (spec §6.1 and Task 6's own code), not R-plan-4's `yokohama.chat.folded.<scope>` — **the plan disagreed with itself and the spec wins** | nothing; either is a per-browser convenience |
| R-pre-2 | T1 | BOTH stores refuse a `Doc::House` id other than `chat::HOUSE`; `MemStore` gains the check `FsStore::path` has, because it asks only `is_safe_id` and `"house2"` passes | nothing; the cell iterates both stores |
| R-pre-3 | T2 | `chat::check_shape` is called path-qualified or aliased in `tables.rs`, never glob-imported over the table's own `check_shape` | a compile error otherwise; nothing |
| R-pre-4 | T6 | A `Chat` mount costs ONE fetch even when remembered open with a count above what it holds: the count effect must not duplicate the mount read, and coalesces instead | one redundant GET per mount |
| R-pre-5 | T6 | The dedupe cell's stub answers the full page to EVERY GET, so the delta genuinely carries held messages | a vacuous cell |
| R-pre-6 | T5 | `house_chat_off_404` is captured inside the async corpus assembly from a second `Api`, because `constructed()` is sync and an `Api` call is not | nothing |
| R-pre-7 | T4 | Task 4's first cell drops its garbled comment; the stored-name-by-value case is the second cell as written | nothing |
| R-pre-8 | T7 | Task 7's `optionName(...)` placeholder is `App.test.tsx`'s own `firstOptionButton()` | nothing |
| R-pre-9 | T0 | Task 0 is executed by the controller inline rather than dispatched — it IS the SDD pre-flight | nothing |
| R-2-1 | T2 | Task 2 touches `lib.rs` ONLY where the house channel's type change forces it; the `chat` field on the house frame is Task 4's | nothing; Task 4 re-touches the same line |
| R-2-2 | T2 | The seven-line number/push/trim/write block IS extracted into a private `append_locked` — the spec's "one writer" names the PUBLIC seam and does not forbid a private helper | nothing; a behaviour-preserving refactor |
| R-3-1 | T3, T4 | **A server task that changes a payload SHAPE regenerates the corpus, reads the diff, and makes the minimal additive client change in the same commit**, so both suites are green after every task — the plan's "Tasks 4 and 5 regenerate" left `wire_fixtures` red between Task 3 and Task 5 | Task 5's diff is smaller by exactly those lines |
| R-3-2 | T3 | The duplicate default is BOUND: the parser's absent arms read `ChatLimits::default()` and a cell pins the unset case equal to it | nothing |
| R-5-1 | T5 | Spec §6.5's per-MESSAGE `Extract` rows on the mixed pages are NOT written — a JSON import widens the tag to `string` and an array element types as the union of its members, so the rows cannot compile and the only alternative passes vacuously; the per-member rows live on the four single-author payloads and the mixed page is key-checked by `kind` at RUN TIME | a residual gap the reviewer filed Minor: an added key inside one of the three mixed-page authors is seen only at run time — which still fails the suite |
| R-6-1 | T6 | The plan's `queueMicrotask` scroll block is REPLACED by the spec's rule, because the block runs before React commits and so never follows | nothing beyond one effect |
| R-7-1 | T7 | Spec §6.6's THIRD help edit, which the plan omitted, IS written | one sentence and one cell |
| R-order-1 | wrap | With the headed review removed, the plan's Task 9 → Task 10 order lost its reason to release BEFORE the whole-branch review; the sequence became review → fix wave → battery + counts → release → close | nothing; **and it was the right call — Importants 1–4 would otherwise have been a point release** (the reviewer's own judgement) |
| R-order-2 | wrap | When the fix wave reports DONE with green gates the controller runs the release AT ONCE from the fix wave's HEAD, and the re-review, battery, counts and close defer to the resumed session. A fix wave that is BLOCKED or red does not release | a reviewed-but-not-re-reviewed image on the NAS for two days; the re-review then found 0 open |
| R-wrap-1 | wrap | The scoped re-review and the mutation battery run CONCURRENTLY — both read-only on the main checkout, the battery in its own worktree | CPU contention |
| R-wrap-3 | wrap | The `Claude-Session:` trailer on this session's commits carries this session's local id; the prior session's commits carried a claude.ai/code URL and this session has none | a trailer |
| R-wrap-4 | wrap | ONE fix wave over the live review's five findings and the re-review's two notes, then a scoped re-review, the counts re-run at the new HEAD, and a POINT RELEASE under the owner's 2026-09-19 authorization — the M6.2 precedent | one more image on the NAS and a second counts run |
| R-wrap-5 | wrap | ACCEPT the container-query fix for I-1 and the lobby section's margin move: the defect is the ROW's and not the viewport's, since the desktop side column showed it under a long label | a two-line revert (`@container` → `@media (max-width: 600px)`) or one deleted rule |
| R-wrap-6 | wrap | PR #34 is opened BEFORE the close commit, so the close can cite its number | nothing; the PR tracks the branch |

**The ledger records no R-wrap-2**; the numbering skips it and the source is silent on why.

### Restated from the plan (R-plan-1 … R-plan-7)

- **R-plan-1** — `Doc::House` takes exactly one id, refused otherwise. Done
  (`store.rs:200`), and R-pre-2 extended it to both stores.
- **R-plan-2** — the house channel's value becomes `House { seq, chat }`. Done
  (`signal.rs:42`); the existing house-receiver cells read `.seq`.
- **R-plan-3** — the text rule is one function run twice, by the route and again by the
  appender, so a second caller (a talking bot) is bounded by the same rule. Done, and
  re-verified at HEAD by the whole-branch review.
- **R-plan-4** — the fold is one module. Done, **with its KEY overruled by R-pre-1**: the
  plan's own Task 6 code and the spec both said one key holding a set.
- **R-plan-5** — the `chat_last` cache lives on `Games`, seeded on a miss under no lock.
  Done — **and that seed under no lock is exactly what made the review's minor 6 possible**;
  `or_insert` keeps the ruling while closing it.
- **R-plan-6** — the chat block runs BEFORE the move loop, so `events.json` reads `chat: 3`.
  Done; the corpus diff is the proof.
- **R-plan-7** — the e2e's no-reopen claim is counted from the first game-frame reply. Done,
  **and moved one step earlier on measurement** (§1.7): read where the plan put it, the
  assertion is `0 === 0`.

### Deviations from the spec's letter, recorded here

- **§6.1 — "the 201's message is appended locally and `last` advanced, so the frame it
  causes is a no-op compare" is FALSE on the live tunnel** (live review M-2). The bump
  reaches the poster's own stream before the 201 lands, so the count rule fired first. Fixed
  in the second fix wave; the dated spec was not edited and the correction lives at the code
  (`Chat.tsx:234-244`).
- **§6.1 — "reads everything retained ONCE at mount" wobbles by one cheap delta at the two
  WAKE hosts**, whose opening read ticks 0→1 right after mount. True of the component,
  approximately true of the host; LEAVE (§6.2).
- **§6.5 — the per-message `Extract` rows are not written** (R-5-1), and the mixed page is
  key-checked by `kind` at run time instead.
- **§9.5 — the battery is twenty-two mutations, not the task brief's "eighteen"**: the
  spec's own numbered list is the authority and was followed.
- **§10 — the headed live review** was removed by the owner on 2026-09-17 and restored by
  the owner on 2026-09-19; it ran, and §4.5 is its record.
- **Task 5's deviations**: the `chat_403_*` fixtures duplicate the CONTENT of existing 403
  bodies under new names, by design, because the client reads them by route; and a walk-side
  premise assertion was added beyond the brief, without which the silent-watcher pin searches
  for somebody who might not be watching.
- **Task 6 used a real-event-loop `settle()`** in place of a bare `act`, and added one CSS
  rule the brief lacked; the reviewer judged the deviation legitimate and it was kept.
- **Task 7 answered the panel's mount read inside the shared harness** and added
  `exact: true` at six selector sites, because a substring match on the Lobby heading now
  collides with "Lobby chat show" and failed 34 of 39 walks.

---

## 6. The LEAVE list

### 6.1 Inherited and not touched by this cycle

Every M6.3 close §6.1 and §6.2 item stands except where §6.3 below says otherwise, and the
M6.1 and M6.2 items carried through them carry through here unchanged. In particular the
two the handover predicted chat would meet are still open and now cost slightly more: the
double table read in `events_at` (once via `table_for_anyone`, once inside `admitted`), and
blocking store I/O inside the stream's `poll_next` — to which a chat-count read per bump is
now added, though the count is the CACHE and not a parse (§1.2).

**Closed by this cycle:** M6.3's Minors **M-1** (a watcher or player on a FINISHED game not
told of an abandon) and **M-3** (a player whose turn it is holds no stream) are closed by
R-6.4.4's always-held stream, and **M-3 was tried on the live site and did not reproduce**:
an abandon reached a holding mover's screen in 54 ms (§4.5). M6.3's **M-2** — an arrival by
link drawing the lobby for one round trip — is still there and still LEAVE.

**Still open and checked at this HEAD rather than assumed:** the dealing screen's "waiting
on" line. M6.3 left it one handler line short, and M6.4 did not take it — `Dealing.tsx`'s
handler now reads the chat count off the frame, but still acts on the status only when it
LEAVES `dealing`.

### 6.2 This cycle's own

**The final review's twenty LEAVE lines**, by its own grouping. *Task 1:* `HOUSE` passes
`is_safe_id` by accident (pinned indirectly — a `HOUSE` that failed the guard would fail
`MemStore`'s write); `check_shape` accepts a whitespace-only stored text (unreachable
through the trimming appender); normalized `User`/`check_name` values are discarded on read;
the generic store names `crate::chat::HOUSE`, R-plan-1's one layering exception. *Task 2:*
`keep: Some(0)` unguarded (the config refuses 0); `read_chat`'s `id` dead on the house
branch; thin coverage on three refusal paths; `keep_all_trims_nothing` runs ~3 s with a
load-bearing N; the house key rule stated twice. *Task 3:* the lobby refusal quotes its
value lowercased where the other two quote it as typed; a non-UTF-8 value folds to the
default; `ChatConfig`'s two public fields carry no doc comments. *Task 4:* `house_chat_post`
duplicates `author_of`'s `Spectator` arm; `chat_read` reads the table document twice.
*Task 5:* the mixed-page authors are checked at run time only (R-5-1's residual); the
frozen-clock cell is broader than asked; the two 403 fixtures duplicate existing sentences.
*Task 6:* the deferred re-ask recurses rather than looping; `unread` is dead state on a
count host, by design. *Task 7:* **the two wake hosts make TWO chat reads at mount**, so spec
§6.1's "ONCE at mount" wobbles by one cheap delta — the review's own call, and it wants a
host change; and the report's "six `exact: true` sites, not seven". *Task 8:* the players'
baseline of two stream opens is explained in the report and pinned by no assertion.

**The fix-wave re-review's three notes** were taken rather than left, in `b16ccdd` (§4.6).

**The second fix wave's three, all LEAVE:**

1. **The TICK hosts still pay one delta GET per own post.** The park is `why === "count"`
   only, and a tick read is unconditional because the delta is that host's only source; the
   finding scoped itself to count-driven hosts. Measured in the review's own request log.
2. **34 rem is a second design figure coupled to `--side`**, with ~30 px of headroom and
   nothing pinning the coupling: a widening past ~38 rem would silently flip the game panel
   back to one row, and the new side-column cell runs at 1280 and would not see it.
3. **The who-strip's 480 px threshold leaves a band** (481–~560 px) where a 35–40 character
   address can still wrap inside its own box and the connection-status span takes a third
   line. That is the row's pre-M6.4 behaviour and the finding was at 390 px. **This is what
   remains of CC-1** (§4.7): the Chrome pass's 500 px reading sits inside that band.

**The live review's own Minors** are §4.5's M-1 to M-4, all four FIXED in the point release;
what LEAVES from that review is the M6.3 M-2 observation and the "Not reachable" list (§7).
**CC-2** — the New-game form's truncated placeholders at ≤500 px — LEAVES, cosmetic and
M6.2's form.

### 6.3 OWNER ITEMS — decisions that are not an agent's

**M6.3's owner item 1 — the email roster — is NOT carried here.** R-6.4.2 settled it as
"leave as is" for the NAS and, in doing so, widened it: a message serves its author's email
on the wire, which is the same fact `GET …/{id}/table` already answers every reader about
every seat. **It moves into the M6.5 handover as a QUESTION** (§5 of that document),
because the decision the M6.3 close names is the HOSTED one, and the rate-limit half —
`unlock` and `join` are password oracles with no limit — goes with it.

1. **The whole-game gate's vs-BOT arm LIVELOCKS on some OS-entropy seeds**, diagnosed in
   the M6.3 close §6.3: the arm's driving policy places nothing and leans on the bot to end
   the game, hitting the 20,000-input cap. **It did not fire once in this cycle** — every
   reading here was 24–33 s with both arms green, including the counts sequence's 25.72 s —
   which is what an intermittent defect looks like, not evidence it is gone. Candidate fix:
   the placement probe the two-people arm already has. Not this cycle's code; the owner's
   call.
2. **The e2e's `CAP = 4000` loop iterations**, and a hit reports through `deadEnd` as
   "nothing on the screen is answerable", which is also the message for a genuinely stuck
   table. **This cycle's readings: 3,366 / 1,758 / 2,730 / 2,464 / 2,158 / 2,270 — no hit.**
   The choice is still a larger cap, a distinguishing message, or both.
3. **The disk.** It sat at 56 GiB free at Task 0 and 46 GiB after the counts sequence, well
   above the 12 GiB threshold, and `web/test-results` was 4 KB at the end — **no failed
   Playwright run wrote a trace this cycle**. The item stands because the mechanism does: a
   failed run writes a multi-GB trace, and on a full disk the artefact that would explain
   the failure is what the machine cannot write.
4. **The house chat has NO delete** (R-6.4.14), so **any line posted to it stays until an
   operator edits `_house.chat.json` on the volume.** Five lines are there now: the release
   check posted from the `0.5.0-m6.4` deployment verification, the live review's three
   ("hello from Ada (desktop)", "renamed line (desktop)", "hello from Ada (mobile)") and the
   re-check's one ("live review 0.5.0-m6.4.1: hello from Ada (desktop)"). The `0.5.0-m6.4.1`
   deployment's own chat check was deliberately run at a THROWAWAY TABLE and posted no house
   line. Retention and moderation on a public door are an M6.5 question.
5. **A house message costs every visible lobby three listing reads.** The lobby is a
   `"wake"` consumer, drops the frame and refetches all three of its listings on any house
   bump, chat included — spec §11's own LEAVE, now said out loud at the four places that
   used to contradict it. The fix is a `"value"` consumer of the house channel that compares
   both counters; the two-counter frame exists for exactly that and no consumer reads it
   today.
6. **The per-process `chat_last` cache and the in-process `Signals` are what a multi-process
   host breaks.** A second replica's appends are invisible to this one's frames, and a bump
   is a notification inside one process. That is a FACT for M6.5 rather than a defect here —
   the DOCUMENT is always the authority and every page is served from it — and the M6.5
   handover §2 enumerates every such piece of per-process state.
7. **WARP was found DISCONNECTED on this workstation at the resume**, and the headed review
   needed `warp-cli connect` followed by `warp-cli debug access-reauth` before a Playwright
   context could reach the site on its own origin. **The runbook's "WARP stays on" is an
   assumption the workstation does not keep by itself**, and the next session that drives the
   deployed site should check it first. It was LEFT CONNECTED at the end of this session —
   the owner authenticated it themselves mid-session, and one `warp-cli disconnect` returns it
   to the state it was found in.

---

## 7. What this cycle does NOT claim

Spec §11, restated at this HEAD, with what the build adds.

- **No edit, no delete, no moderation, no rate limit, no kick** (R-6.4.14). `unlock` and
  `join` remain password oracles without a rate limit. The house chat's no-delete is now a
  lived consequence (§6.3 item 4) rather than a design note.
- **No read state beyond the browser session.** The unread count is a session number and the
  fold is a per-browser key; nothing records what a person has read. The live review
  confirmed the consequence on the live site: retained history is not unread.
- **The name is per BROWSER, not per person**, and there is no profile. A person on two
  devices types it twice; a provider-supplied name on `/api/me` is a later line.
- **The count is per PROCESS.** A second process's appends are invisible to this one's
  frames until the store is a database and the count a query (§6.3 item 6).
- **The house frame's `chat` is read by no consumer.** It exists so a future `"value"`
  consumer can tell the two counters apart (§6.3 item 5).
- **The held arm's chat latency is the held arm's**, and the arm is DARK: `SIGNAL_KIND` is
  `"sse"`, measured in M6.3. Nothing about chat was tried on the fallback arm.
- **No page size on the read.** A cold `after=0` answers every retained message — at the
  defaults at most 500 messages of 500 characters, ≈250 KB worst case, once per mount per
  host. The keep IS the page size.
- **The talking bot is a SHAPE, not a feature** (R-6.4.15): the `Bot` author arm is produced
  by no route and is reachable in a test only off a JSON literal. No trigger, prompt,
  endpoint, key, HTTP client, cost rule or name is decided.
- **Nothing was measured with more than three browsers**, and the live review is minutes,
  not a week. What it says is that the mechanism works where it is deployed, not that nothing
  else is wrong.
- **What the live review could NOT reach** (§4.5): a real phone or Safari; the hidden-tab
  word, because Playwright cannot hide a tab; a bot's chat line; the retention trim at a keep
  of 500; game over at a table of two PEOPLE; and load beyond two open lobbies, where each
  house line cost 3 + 3 listing reads.
- **What the Chrome pass could NOT reach** (§4.7): anything below 500 px, because Chrome on
  macOS will not make the window narrower; and everything that needs a post or a move, since
  it made neither.
- **`favicon.ico` was fixed, and the lesson is the instrument's**: every earlier review in
  this lane reported "zero console errors" from a headless browser that never asks for a
  favicon. A claim about console errors is a claim about the browser that made it.
- **The stream is still not a security boundary**: what it carries is what the `events` route
  would answer the same caller, now with a count.
- **The second arm of the whole-game gate is still NOT a witness to the acting-seat fence** —
  M6.1's own sentence stands, because the walk always posts as the seat whose turn it is.
  The chat block added here posts as three identities and asserts nothing about that fence.
- **No wall figure is a budget.** The two-browser cell's 457 s is recorded beside 282 s at the
  previous HEAD; 0 idle looks is the figure that means something.
- **The `CAP = 4000` is the cell's, not the spec's**, and it was not hit this cycle.
- **The notifier is still designed only** (M6.3 spec §7, amended here: the house poke becomes
  `{"seq": N, "chat": M}`). None of it is built. M6.5 builds it.
- **Every count, line number and route name in the spec is at `2194b57`**; the plan re-read
  them at Task 0 and this close re-read them at `b16ccdd`.

---

## 8. The handover into M6.5 — the hosting move

**M6.5's entry artifact is
`docs/plans/2026-09-19-web-frontend-m6.5-hosting-handover.md`**, written at this HEAD by the
orchestrating session: the charter, what M6.4 leaves that hosting stands on with its code
citations, the rulings that bind it, the questions the brainstorm must settle, and the
process that has worked for four cycles. Read it after this document's §1, §5, §6 and §7.

**What is done and needs nothing further:** the release (`0.5.0-m6.4`, live 2026-09-17), the
whole-branch review and its fix wave, the mutation battery (22/22), the counts sequence at
`b16ccdd`, the HEADED live review and the Claude-in-Chrome pass R-6.4.16 asked for, the
second fix wave and its re-review, and the point release (`0.5.0-m6.4.1`, live 2026-09-20).

**What waits on the owner:** **PR #34 is open** against `web-room-m6.3`
(https://github.com/jmoore2333/yokohama-assistant/pull/34), which is stacked on PR #33 → PR
#32 → PR #31 — **merge order is the owner's**, and nothing is merged. The seven owner items
in §6.3 are the rest, of which two are new this cycle (the house chat's undeletable lines,
and WARP not staying on by itself) and one — the email roster on the hosted path — has moved
into M6.5's questions rather than staying an item here.

**This record freezes here**, like every other dated artifact; anything later is recorded
forward.
