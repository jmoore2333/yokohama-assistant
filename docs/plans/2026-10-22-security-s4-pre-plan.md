# Security S4-pre — the web lane's leftovers: implementation plan (written 2026-10-22)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clear the web lane's four code leftovers before the public-release cycles — a seat-aware admission on both hosts, the marker head's wording, the NAS's retired keyed-build reason, the secrets guard's door check — and re-record the nine sentences that still hand that work to "S4", so S4a starts with no web-lane backlog and the public door opens with seated players admitted past a crowd.

**Architecture:** Issue 1 lands in three places that already exist: the layer's one table read (`api::events_at`) gains a sibling that also answers whether the caller holds a seat; the NAS's `Streams` keeps a seated count beside its two and judges a watcher by a third figure; the Worker module carries the bit on `ReplyData` as `picker` and `archive` ride today, and the table object tags a seated socket so the watchers' count is one more tag lookup. Nothing on the wire moves: `Events` is unchanged and the corpus pins it. Issues 2–4 are one-file sentence and guard changes with their cells. The records task applies the specification's drafted sentences.

**Tech Stack:** Rust (axum, tokio; the layer compiled natively and to wasm32 for the Worker module), TypeScript (the Worker shell under `worker/` with vitest and the workerd pool), bash (`deploy/update.sh`), `gh` for the three posted issues.

**Spec:** `docs/plans/2026-10-22-security-s4-pre-spec.md` — every rule below argues from it (its Issues 1–4, its Part 2, its findings); the S4 design `docs/plans/2026-10-22-security-s4-public-release-design.md` §2.22–§2.25 and §5a holds the owner's rulings. An implementer reads the spec's issue before the task; line numbers in the spec are readings at `56460688` and are re-read at the branch's head.

## Global Constraints

- **The branch is `security-s4-pre`, cut from master AFTER PR #56 and `security-auth` have merged** (the design §2.9, §5a); every commit lands on it and the PR targets master. Stage with explicit paths (`git add <path>`), never `git add -A`. The wave bumps no version and deploys nothing: it ships with stage 2's `0.7.3` on both hosts (design §2.23).
- **The suites per edit** (`docs/web-lane.md` §2; named in each task's verify step): Rust — `cargo test -p yokohama-web` (the lib, `start_refusal`, `wire_fixtures`, `lane_document`, `browser_played_pin`), `cargo test -p yokohama-wasm`, `cargo test -p yokohama-worker`; the Worker shell — `cd worker && npm run build:module:debug` FIRST whenever `crates/yokohama-worker` or the layer changed (the pool tests a stale module green), then `npm run typecheck && npm test && npm run test:scripts`; the browser module — `cd web && npm run build:wasm && npm run test:wasm` whenever `crates/yokohama-web/src` changed; the lint gate `cargo clippy --workspace --all-targets -- -D warnings`. The whole web set (`npm run typecheck && npm test && npm run test:wasm && npm run e2e` in `web/`) runs once at the close. `cargo fmt` is never run.
- **The wire corpus does not move:** `Events` gains no field; `cargo test -p yokohama-web --test wire_fixtures` passes WITHOUT `UPDATE_WIRE_FIXTURES=1` in every task. A task that needs to regenerate it has misread the spec.
- **Figures are cost bounds a feature may not lean on** (the spec's Issue 1): 8 per caller, 64 per table, 32 watchers per table, OPEN streams and sockets alone counted; the house's caps unchanged; a seated caller refused only by the total and by their own 8; classification by the caller's standing at admission.
- **TDD, test-first, every commit** (the repo's history is strictly TDD); no `#[ignore]`d test anywhere in this wave; every invisible non-ASCII code point in code is a `\u` escape; sentences one line each in the house voice; every `ApiError` capped at construction as today.
- **The five fenced crates and `SEMANTICS_REV` do not move**, so no release-tier roster runs in this wave (the design §5a.6).
- **No exploit detail in any committed file;** the ledger's titles stay neutral.
- **Dispatch rules (the owner's standing rules):** every implementer and reviewer runs on an EXPORT (`git archive`) of the branch head, never in the orchestrator's tree, each export building in its OWN `target/` and with the worker package's `node_modules`, `gen`, `.test-assets`, `assets` and the two `test/*.gen.ts` symlinked in (the auth cycle's trap); every agent writes its full report to a transcript file and replies with the path and three lines; no agent runs `rm -rf`, `rm -r` or a bulk `rm -f` — exports are left in place and named; no agent waits on events, arms a Monitor, spawns a subagent or messages a peer. The secrets guard's command cells run ALONE, never beside a cargo build (they read red beside one).
- **Tiers:** as each task's header says (`impl: opus` for Issue 1's two tasks, `impl: sonnet` for the rest); every task reviewer `review: opus`; the whole-branch review is Fable's, dispatched on an export of the head; the pre-flight in "Before Task 1" is the orchestrator's; mutants named in each task are run on an export by the reviewer.
- **Commit messages** in the house style (a prose subject naming the change, its rule and its catcher), ending with the attribution trailer the committing session carries.

## Review Focus

Five inputs the spec implies that a person will meet, each pinned by a cell in the task that owns it:

1. **A seated player reconnecting into a table full of watchers** — a page reload, a phone waking — must be admitted (Task 1: `a_seat_holders_stream_is_a_200_into_a_table_whose_watcher_streams_are_full`; Task 2, the pool: "admits a seated player's socket into a table whose watcher sockets sit at their figure, and refuses the next watcher with the watchers' sentence and a 4429 close").
2. **A joiner who was watching before they took the seat**: the socket they opened as a watcher counts as a watcher's until it reconnects; the one they open after joining is seated (Task 1: `a_strangers_stream_at_an_open_table_is_counted_as_a_watchers_until_they_reconnect_holding_a_seat`; Task 2, the pool: "counts a joiner's earlier socket as a watcher's until it reconnects, and the one opened after the join as seated").
3. **A seated player's own tab storm** is still refused at the ninth with the caller's sentence, seat or no seat (Task 1: `a_seated_caller_is_still_held_to_the_per_caller_figure_and_to_the_rooms_total`; Task 2, the pool: "still refuses a seated player's ninth socket with the caller's sentence, and counts a dangling refusal toward neither pool").
4. **The lobby is untouched by the new arm**: its caps, its sentences and its verdict order read exactly as before (Task 1: the house passes `false` and the existing house cells stay green; Task 2: `capVerdict`'s rows unchanged in `sockets.unit.test.ts`, the lobby pool cell green, `house.ts` not edited).
5. **A marker stored before the head changed** renders the new head: the stored shape is unchanged and the head is composed at read (Task 3: the unit cell over a literal `Marker` and the two expiry cells).

And one for the tooling: **the secrets guard run alone against a file with no door** refuses in the shell's words rather than passing with nothing judged (Task 5, the command cell).

---

## Before Task 1 (the orchestrator)

- [ ] **The owner's merges first:** PR #56 (`security-s3` → `security-auth`) merged; the PR `security-auth` → master opened and merged on the owner's word (no new review: every commit on it was reviewed in its cycle and is what production runs). Confirm: `git fetch origin && git rev-list --count origin/master..origin/security-s3` reads `0`.
- [ ] **Cut the branch:** `git checkout -b security-s4-pre origin/master`. Its first commit is the three documents: `git add docs/plans/2026-10-22-security-s4-public-release-design.md docs/plans/2026-10-22-security-s4-pre-spec.md docs/plans/2026-10-22-security-s4-pre-plan.md` (the design committed here only if the owner has finished reading it; otherwise the spec and the plan, and the design in its own later commit).
- [ ] **The two sentences the owner writes**, with the defaults used if the owner does not replace them here (design §2.22, §2.24):
  - the watchers' refusal, one per host in that host's register — NAS `TOO_MANY_WATCHERS = "this table has too many open watcher streams — try again in a moment"` (its sibling: "this table has too many open streams — try again in a moment"); Worker `TOO_MANY_WATCHERS_TABLE = "this table has as many watchers as it can take — try again later"` (its sibling: "this table has as many connections as it can take — try again later");
  - the marker head: `this table waited {days} to start and was closed on {day}` (shape A, the spec's Issue 2).
- [ ] **The "before" review**, in an export of the head: run the suites the Global Constraints name once and write the baseline counts into the SDD ledger; read Task 1's and Task 2's cells against the spec's Issue 1 and the code they cite (`streams.rs`, `sockets.ts`, `events.rs`, `module.ts`) and correct this plan AT THE SOURCE before dispatch, each correction numbered P-n in the plan's Review record.
- [ ] **Open the SDD ledger** at `.superpowers/sdd/2026-10-22-security-s4-pre-plan/progress.md` (gitignored): one row per task with its tier, its export path, its transcript path and its verdict.
- [ ] **Disk before the first export.** On 2026-10-23, on the owner's word, the auth cycle's twenty export trees (1.8 GB) and the checkout's whole `target/` (`cargo clean`: 1,195,555 files, 232 GiB as cargo counts them) were removed, since every export builds in its own `target/` (the S3 lesson); the Data volume then read 132 GB free of 926 (86 % used), and the first `cargo` command in the checkout rebuilds from nothing. Read `df -h /System/Volumes/Data` and write it in the ledger before the first dispatch; each task's export is removed by the orchestrator once its review and re-review have read it, and the close names what was left.

---

### Task 1: Issue 1 — the layer's seat bit and the NAS admission · impl: opus · review: opus

The spec's Issue 1, "The change" items 1–3 and 5, the NAS and layer halves. The layer answers whether the caller holds a seat from the ONE table read it already makes; the NAS's `Streams` takes the bit, keeps a seated count per room, and refuses a watcher by a third figure in the order total → watchers → caller; a seated caller is refused only by the total and by their own 8.

**Files:**
- Modify: `crates/yokohama-web/src/api.rs` (`events_at` at ~2086 becomes a wrapper over the new sibling; the pin `events_at_reads_the_table_once` at ~4337 extended into a second cell beside it)
- Modify: `crates/yokohama-web/src/streams.rs` (the constants block at 1–23; `Counts` at ~31; `Slot` at ~44; `admit` at ~78; `open` at ~97; `Drop for Slot` at ~118; the module doc at 13–23 rewritten with the spec's Part 2 (d) "wave lands it" draft; the existing cell at ~142 passes `false`)
- Modify: `crates/yokohama-web/src/lib.rs` (the `stream` route at ~1013–1019; the house stream at ~1092; the route cells beside `a_callers_ninth_stream_is_a_429_before_any_frame_and_a_dropped_response_frees_it` at ~6240)
- Test: `streams.rs` (unit cells), `api.rs` (the layer cell), `lib.rs` (route cells)

**Interfaces:**
- Produces: `pub fn events_at_seated(games: &Games, id: &str, user: &identity::User) -> Result<(Events, bool), ApiError>` in `api.rs` — the `bool` is `!table.seats_of(user).is_empty()` over the table `games.table_for_anyone(id, user)?` returns, computed before any arm, so it holds at `open`, `dealing` and `playing` for a seat holder and a hot-seat holder, and is `false` for a stranger at an open table and a spectator at a playing one; `events_at` becomes `events_at_seated(games, id, user).map(|(e, _)| e)`. In `streams.rs`: `pub const WATCHERS_PER_TABLE: usize = 32`; `pub const TOO_MANY_WATCHERS: &str = "this table has too many open watcher streams — try again in a moment"` (the plan's default, kept by the owner 2026-10-23); `pub fn admit(self: &Arc<Self>, room: Room, caller: &User, seated: bool) -> Result<Slot, &'static str>`; `pub fn seated(&self, room: &Room) -> usize` (OPEN seated streams in `room`); `#[cfg(test)] pub(crate) fn is_quiet(&self) -> bool` (every map empty).
- Consumes: `TableDoc::seats_of` (`table.rs` ~288), `Games::table_for_anyone`, the test helpers `post_as`, `stream_response`, `stream_status`, `WHO` (`lib.rs` ~1427, ~3514, ~6212–6230) and the pin's `CountingStore`, `two_people`, `playing`, `who`, `SeatSpec`.

- [ ] **Step 1: Write the failing unit cells in `streams.rs`** (each a `#[test]` beside the existing one; `seated` is the new third argument; a "watcher" is `admit(.., false)`, a "seat holder" `admit(.., true)`; hold the returned `Slot`s in a `Vec`):

```rust
#[test] fn a_seated_callers_stream_is_admitted_into_a_room_whose_watchers_sit_at_their_figure() {
    // 4 watchers × 8 held (32 = WATCHERS_PER_TABLE); admit(room, &user("seat@t.test"), true) is Ok; open(&room) == 33, seated(&room) == 1
}
#[test] fn a_watchers_stream_past_the_watchers_figure_is_refused_in_the_watchers_sentence_with_room_to_spare_in_the_total() {
    // 32 watcher streams held; a fifth caller's first watcher stream is Err(TOO_MANY_WATCHERS); open(&room) == 32 (< STREAMS_PER_TABLE); the refusal took no slot
}
#[test] fn a_seated_caller_is_still_held_to_the_per_caller_figure_and_to_the_rooms_total() {
    // a seat holder's ninth is Err(TOO_MANY_STREAMS_CALLER) with 8 open; a room at 64 (five seated callers × 8 + 24 watchers — the module knows no seat count) refuses a sixth seated caller's FIRST stream with Err(TOO_MANY_STREAMS_TABLE)
}
#[test] fn the_verdicts_run_total_then_watchers_then_caller() {
    // two-defect probes at each adjacent pair (CLAUDE.md: a single-defect probe pins nothing about order):
    // total AND watchers hit, a watcher → TOO_MANY_STREAMS_TABLE; watchers AND caller hit, a watcher at 8 → TOO_MANY_WATCHERS; total AND caller hit, a seat holder at 8 → TOO_MANY_STREAMS_TABLE
}
#[test] fn a_dropped_stream_frees_its_seated_slot_and_a_quiet_room_leaves_nothing_in_either_map() {
    // one seated slot: seated == 1; drop it: seated == 0, open == 0, is_quiet(); the house admits with `false` and its figures are unchanged (HOUSE_STREAMS_PER_CALLER, HOUSE_STREAMS_TOTAL)
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p yokohama-web --lib streams::`
Expected: compile error — `admit` takes two arguments; `WATCHERS_PER_TABLE`, `TOO_MANY_WATCHERS`, `seated`, `is_quiet` not found.

- [ ] **Step 3: Implement the `streams.rs` change.** `Counts` gains `seated_by_room: HashMap<Room, usize>`; `Slot` gains `seated: bool`; `admit` judges, for `Room::Table`, the total first, then — only when `!seated` — `by_room − seated_by_room >= WATCHERS_PER_TABLE` → `Err(TOO_MANY_WATCHERS)`, then the caller's figure; for `Room::House` the two arms as today (the bit ignored); on `Ok` a seated stream also increments `seated_by_room`, and `Drop` releases it with `release`, so a quiet room holds no entry in any map (B8's lesson, `streams.rs:15`). Add `seated()` and `is_quiet()`. Update the existing cell to pass `false`. Rewrite the module doc with the spec's Part 2 (d) "if the wave lands" sentence.

- [ ] **Step 4: Run to verify they pass**

Run: `cargo test -p yokohama-web --lib streams::`
Expected: 6 passed.

- [ ] **Step 5: Write the failing layer cell in `api.rs`**, beside the single-read pin and built from its helpers:

```rust
#[test] fn events_at_answers_whether_the_caller_holds_a_seat_from_its_one_table_read() {
    // the pin's CountingStore, red/green, two_people, SeatSpec, playing(..): at "open" (seat 1 open) red → (_, true), green → (_, false); at "dealing" red → true;
    // at "playing" red → true, green (a spectator) → false; at "hot" (both seats `SeatSpec { user: Some("red@t.test".into()), bot: false, open: false }`) red → true —
    // if `Games::create` refuses one key in two seats, say so in the cell's doc and drop that row; every call reads the table EXACTLY once (store.table_reads() delta == 1)
}
```

- [ ] **Step 6: Run to verify it fails**

Run: `cargo test -p yokohama-web --lib events_at_answers_whether`
Expected: compile error — `events_at_seated` not found.

- [ ] **Step 7: Implement `events_at_seated` in `api.rs`** with the signature above; `events_at` wraps it. The bit is computed from the FIRST read, above the open arm; no second table read, no field on `Events`.

- [ ] **Step 8: Run to verify it passes, and that the wire is unmoved**

Run: `cargo test -p yokohama-web --lib events_at && cargo test -p yokohama-web --test wire_fixtures`
Expected: the two `events_at` cells pass; `wire_fixtures` passes with no regeneration.

- [ ] **Step 9: Write the failing route cells in `lib.rs`** (`#[tokio::test]`, in the module of the existing cap cells, from `post_as` / `stream_response` / `stream_status` and `state.streams.open` / `.seated`). **Every stream whose slot a later assertion counts is a `stream_response` HELD in a binding or a `Vec`** — `stream_status` drops its response on return, and the drop frees the slot at once (`Admitted` rides the body), so a count read after it reads the slot gone (P-2); `stream_status` serves only a status whose slot nothing later counts:

```rust
async fn a_seat_holders_stream_is_a_200_into_a_table_whose_watcher_streams_are_full() {
    // AppState::in_memory(); WHO creates [{"user": WHO}, {"open": true}]; four strangers w1..w4@wire.test hold 8 streams each — stream_response, kept in a Vec (32 = watchers at their figure);
    // WHO's stream_response is HELD in a binding (P-2): status 200, seated(&room) == 1, open == 33;
    // a fifth stranger's first stream (stream_response, its body read) is 429 with body["error"] == streams::TOO_MANY_WATCHERS and took no slot (open still 33, seated still 1)
}
async fn a_strangers_stream_at_an_open_table_is_counted_as_a_watchers_until_they_reconnect_holding_a_seat() {
    // WHO's open table; "blue@wire.test" HOLDS one stream (a stream_response kept in a binding, P-2): seated == 0, open == 1; blue joins seat 1 (post_as .. "/api/games/{id}/join", json!({"seat": 1})) → 200;
    // seated still 0 and open still 1 (classification is at admission; the first stream is still held); blue's second stream, held too → status 200, seated == 1, open == 2
}
```

- [ ] **Step 10: Run to verify they fail**

Run: `cargo test -p yokohama-web --lib a_seat_holders_stream a_strangers_stream_at_an_open_table`
Expected: compile error at the two `admit` call sites (two arguments).

- [ ] **Step 11: Wire the routes in `lib.rs`:** the `stream` route reads `let (_, seated) = api::events_at_seated(&state.games, &id, &user)?;` where it called `events_at`, and passes `seated` to `admit`; the house stream passes `false`. The comment at the cap keeps its first sentence and adds the watchers' arm in one clause.

- [ ] **Step 12: Run the suites**

Run: `cargo test -p yokohama-web && cargo test -p yokohama-wasm && cargo test -p yokohama-worker && cargo clippy --workspace --all-targets -- -D warnings && (cd web && npm run build:wasm && npm run test:wasm)`
Expected: every suite green; `yokohama-web`'s lib count is the baseline + 8; `wire_fixtures` unchanged; `test:wasm` at its baseline.

- [ ] **Step 13: Commit**

```bash
git add crates/yokohama-web/src/api.rs crates/yokohama-web/src/streams.rs crates/yokohama-web/src/lib.rs
git commit -m "web: a seat-aware admission on the NAS — the layer answers whether the caller holds a seat from its one table read (events_at_seated), and Streams refuses a watcher past 32 while a seated caller is held only to the total and their own 8, counted at admission (S4-pre Task 1; the spec's Issue 1)"
```

Mutants for the reviewer (on an export): the seat bit ignored (`seated` forced `false`); the watchers' arm applied to a seated caller; the per-caller arm skipped for a seated caller; the three verdicts permuted; `seated_by_room` not released on drop; the bit computed from a second table read (the pin's count goes to 2).

---

### Task 2: Issue 1 — the Worker module's field, the shell's tag, the verdict and the sentence · impl: opus · review: opus

The spec's Issue 1, items 4–5, the Worker halves. The module carries the bit as a host-internal reply field; the table object tags a seated socket, judges a watcher by the third figure, and keeps the lobby's verdict untouched.

**Files:**
- Modify: `crates/yokohama-worker/src/lib.rs` (`ReplyData` at ~220 and the `Reply` getters at ~258–300; `from_the_layer` at ~564; `host_refusal` at ~320; EVERY `ReplyData {` struct literal gains `seated: None` — `grep -n 'ReplyData {' crates/yokohama-worker/src/*.rs` reads 18 lines at `56460688` (`lib.rs` 9, `picker.rs` 4, `receipt.rs` 3, `create.rs` 1, `events.rs` 1), the definition and some `-> ReplyData {` signatures among them, so the compiler is the catcher: add the field until `cargo build -p yokohama-worker --all-targets` names no literal (P-1))
- Modify: `crates/yokohama-worker/src/events.rs` (`events_inner` at ~47)
- Modify: `worker/src/module.ts` (`Replied` at ~18; `readReply` at ~104–126)
- Modify: `worker/src/table/sockets.ts` (the constants at 25–28; `capVerdict` at 31 kept as it is for the house; the new `tableVerdict` beside it; `admit` at ~92–118)
- Modify: `worker/src/frames.ts` (the sentences at 91–94)
- Test: `crates/yokohama-worker/src/lib.rs` (the test module near ~1418), `worker/src/table/sockets.unit.test.ts`, `worker/src/frames.unit.test.ts` (~101–105), `worker/test/caps.test.ts` (the table cells at ~129–196)
- Not touched: `worker/src/house.ts` (it keeps `capVerdict(own, total, caps)` with the lobby's two figures)

**Interfaces:**
- Consumes: `api::events_at_seated` (Task 1).
- Produces: `ReplyData.seated: Option<bool>` (`Some(bit)` from `events` at 200; `None` from every other call and from a refusal); `Reply::seated(&self) -> Option<bool>` (`#[wasm_bindgen(getter)]`; after `npm run build:module:debug` the generated `gen/factory` type reads it as `boolean | undefined`); `Replied.seated?: boolean` read in `readReply`; in `sockets.ts` `export const WATCHERS_PER_TABLE = 32`, `export const SEATED_TAG = "s:seated"` (a tag no table id can spell: `isSafeId` admits only `[A-Za-z0-9._-]`, and `u:` is the caller tag's prefix), `export function tableVerdict(own: number, total: number, watchers: number, seated: boolean, caps: { perCaller: number; total: number; watchers: number }): "own" | "total" | "watchers" | undefined` (total first; then, for `!seated`, `watchers >= caps.watchers`; then own); in `frames.ts` `export const TOO_MANY_WATCHERS_TABLE = "this table has as many watchers as it can take — try again later"` (the plan's default, kept by the owner 2026-10-23).

- [ ] **Step 1: Write the failing module cell in `crates/yokohama-worker/src/lib.rs`**, in the shape of `events_refuses_in_axums_handler_order` (`serial()`, `init_inner(&config("off"))`, `create_inner`, `CREATE`):

```rust
#[test] fn events_inner_reports_a_seat_holder_seated_at_every_status_and_a_stranger_or_a_spectator_not() {
    // "t" from CREATE (red seated, seat 1 open): events_inner("t", Some("red@t.test"), "").seated == Some(true); ("t", Some("blue@t.test")) → status 200, seated == Some(false);
    // "dealt" from CREATE with {"open":true} → {"bot":true}: red → Some(true); blue → status 403 and seated == None; a 401 (no caller) → None;
    // "p" (P-3, the spec's "every status" and its spectator): CREATE with {"open":true} → {"user":"blue@t.test"}, then red and blue each keep card 1 through the test module's `call(..)`
    // as the cell near ~1041 does, so the table is PLAYING: red → status 200, Some(true); "green@t.test" (a spectator; CREATE sets no password) → status 200, Some(false)
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p yokohama-worker events_inner_reports`
Expected: compile error — no field `seated` on `ReplyData`.

- [ ] **Step 3: Implement the module half.** `ReplyData` gains `pub seated: Option<bool>` with a doc line; `from_the_layer` builds it `None`; every literal gains `seated: None` (grep them all — the CLAUDE.md lesson about literals the cap never saw); the `Reply` getter; `events_inner` calls `api::events_at_seated`, captures the bit in a `let mut seated = None;` the closure assigns, and sets `reply.seated = seated` on the 200 path only.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p yokohama-worker && cargo clippy -p yokohama-worker --all-targets -- -D warnings && (cd worker && npm run build:module:debug)`
Expected: green; the module rebuilt with the new getter in `gen/`.

- [ ] **Step 5: Write the failing unit cells in the shell** — `sockets.unit.test.ts` gains a second `it` and `frames.unit.test.ts` extends its literal cell:

```ts
// sockets.unit.test.ts — "judges a table's total first, then a watcher by the watchers' figure, then the caller's own, and leaves a seated caller to the total and their own"
const caps = { perCaller: SOCKETS_PER_CALLER, total: SOCKETS_PER_TABLE, watchers: WATCHERS_PER_TABLE };
expect(tableVerdict(7, 63, 31, false, caps)).toBeUndefined();
expect(tableVerdict(0, 63, 32, false, caps)).toBe("watchers");
expect(tableVerdict(0, 63, 32, true, caps)).toBeUndefined();          // seated: the watchers' figure does not apply
expect(tableVerdict(8, 63, 32, true, caps)).toBe("own");              // seated, their own ninth
expect(tableVerdict(0, 64, 32, false, caps)).toBe("total");           // two defects: total AND watchers → the total's sentence
expect(tableVerdict(8, 63, 32, false, caps)).toBe("watchers");        // watchers AND own → the watchers' sentence
expect(tableVerdict(8, 64, 0, true, caps)).toBe("total");             // total AND own, seated → the total's sentence
expect(capVerdict(8, 64, { perCaller: 8, total: 64 }), "the lobby's verdict is unchanged").toBe("total");
expect([WATCHERS_PER_TABLE, SEATED_TAG]).toEqual([32, "s:seated"]);
// frames.unit.test.ts — the fifth literal beside the four; the `it` title ("carries the four sentences") and the comment above it say FIVE, as does `frames.ts`'s doc line over the constants (P-5):
expect(TOO_MANY_WATCHERS_TABLE).toBe("this table has as many watchers as it can take — try again later");
```

- [ ] **Step 6: Write the failing pool cells in `worker/test/caps.test.ts`**, inside the D3 `describe`, from its own helpers (`openTable` seats RED with seat 1 OPEN; `openTab`, `upgrade`, `held`, `heldOpen`, `endWith`, `closeReason`, `callerTag`, `call`):

```ts
it("admits a seated player's socket into a table whose watcher sockets sit at their figure, and refuses the next watcher with the watchers' sentence and a 4429 close", ...);
//   four watchers w0..w3@t.test × 8 tabs (32); heldOpen(stub, SEATED_TAG) == 0; RED's tab → first frame contains '"event":"signal"', heldOpen(stub, SEATED_TAG) == 1;
//   a fifth watcher's tab → [endWith(TOO_MANY_WATCHERS_TABLE)] then { code: CLOSE_TOO_MANY, reason: closeReason(TOO_MANY_WATCHERS_TABLE) }; heldOpen(stub) == 33
it("still refuses a seated player's ninth socket with the caller's sentence, and counts a dangling refusal toward neither pool", ...);
//   RED × 8 tabs; RED's ninth → [endWith(TOO_MANY_OWN_TABLE)]; a watcher's refusal left dangling (upgrade(), unaccepted) with the watchers at their figure:
//   heldOpen(stub, SEATED_TAG) and the watchers' open count unchanged; after one watcher tab closes (poll ≤ 20 × 50 ms as the file does) the next watcher is admitted
it("counts a joiner's earlier socket as a watcher's until it reconnects, and the one opened after the join as seated", ...);
//   BLUE's tab at the open table: held(stub, SEATED_TAG) == 0; call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}', id) → 200 (the body is a STRING, as `seedPlaying` passes it — P-4); held(stub, SEATED_TAG) still 0;
//   BLUE's second tab → signal frame and held(stub, SEATED_TAG) == 1
```

- [ ] **Step 7: Run to verify they fail**

Run: `cd worker && npm run test:scripts && npm test -- caps`
Expected: `sockets.unit.test.ts` and `frames.unit.test.ts` fail on the missing exports; the three new pool cells fail the same way; the three existing D3 cells and every other cell at their baseline.

- [ ] **Step 8: Implement the shell half.** `frames.ts`: the fifth sentence. `sockets.ts`: the two constants and `tableVerdict`; in `admit`, read `own` as today, `total = openCount(this.ctx.getWebSockets())`, `seated = reply.seated === true`, `watchers = total − openCount(this.ctx.getWebSockets(SEATED_TAG))`, the verdict from `tableVerdict`, the tags `[id, callerTag?, SEATED_TAG if seated]` on `acceptWebSocket` (a refused socket is accepted with the same tags, as today), and the sentence by verdict (`"watchers"` → `TOO_MANY_WATCHERS_TABLE`). `module.ts`: `Replied.seated?: boolean`; `readReply` reads `r.seated` before `r.free()` and spreads it as `picker` is spread. The doc comment on `admit` names the third tag and the watchers' arm in two sentences.

- [ ] **Step 9: Run the shell's suites**

Run: `cd worker && npm run typecheck && npm test && npm run test:scripts`
Expected: green; the pool at its baseline + 3, `test:scripts` at its baseline + 1 (the new `it`), the lobby cell untouched and green.

- [ ] **Step 10: Commit** (two commits are fine: the module, then the shell)

```bash
git add crates/yokohama-worker/src/lib.rs crates/yokohama-worker/src/events.rs crates/yokohama-worker/src/create.rs crates/yokohama-worker/src/picker.rs crates/yokohama-worker/src/receipt.rs
git commit -m "worker module: the events reply says whether the caller holds a seat (ReplyData.seated, from the layer's events_at_seated), None from every other call (S4-pre Task 2a)"
git add worker/src/module.ts worker/src/table/sockets.ts worker/src/frames.ts worker/src/table/sockets.unit.test.ts worker/src/frames.unit.test.ts worker/test/caps.test.ts
git commit -m "worker: a seated socket is tagged and admitted past a room of watchers — tableVerdict judges total, then watchers at 32, then the caller's own; the fifth sentence; the lobby's verdict untouched (S4-pre Task 2b; the spec's Issue 1)"
```

Mutants for the reviewer (on an export): the seat tag left off `acceptWebSocket`; the watchers' count read over ALL sockets rather than OPEN ones; the watchers' arm applied to a seated caller; `tableVerdict`'s order permuted; `readReply` dropping `seated`; one `ReplyData` literal left without the field (a compile error is the catcher — confirm by removing it); `house.ts` switched to `tableVerdict` (the lobby cell must stay green under `capVerdict` and the diff must not touch the file).

---

### Task 3: Issue 2 — the marker head for a table closed before it started · impl: sonnet · review: opus

The spec's Issue 2, shape A: one neutral head, no new stored field; the Worker alone (the NAS has no marker).

**Files:**
- Modify: `worker/src/marker.ts:67` (the `waited` arm of `head`) and the doc comment at 44–51 (one clause)
- Modify: `worker/src/marker.unit.test.ts:62–70`, `worker/test/expiry.test.ts:78` (`HEAD.waited`)
- Docs: `worker/README.md:1485` and `:1508`, `worker/RUNBOOK.md:77` and `:439`, `docs/web-lane.md:162` (the head's quotation on each)

**Interfaces:**
- Produces: the head `this table waited {days} to start and was closed on {day}` (the plan's default, kept by the owner 2026-10-23; at 7 days: `this table waited 7 days to start and was closed on 2026-10-22`, at one: `this table waited 1 day to start and was closed on 2026-10-22`); `Reason`, `Marker` and the stored `meta.marker` JSON unchanged.

- [ ] **Step 1: Write the failing cells.** In `marker.unit.test.ts` rename the `it` to "names a table that waited to start, with the marker's own ttl_days" and set its two literals to the new head at 7 days and `1 day`, the tail unchanged; in `expiry.test.ts` set `HEAD.waited` to the new head (its two uses at ~496 and ~539 then read an `open` and a filled `dealing` table alike) and reword the `HEAD` doc comment at ~72–73 ("a table that waited for players" → "to start"; P-6).

- [ ] **Step 2: Run to verify they fail**

Run: `cd worker && npm run test:scripts && npm test -- expiry`
Expected: the renamed unit cell fails on its two literals; the two expiry cells fail on the head.

- [ ] **Step 3: Change the head** at `marker.ts:67` and the doc comment's clause ("a table that waited … had not started").

- [ ] **Step 4: Run to verify they pass, and that the words are gone from the Worker**

Run: `cd worker && npm run typecheck && npm test && npm run test:scripts && cd .. && git grep -n "for players" -- worker docs/web-lane.md worker/README.md worker/RUNBOOK.md`
Expected: green; once the five document lines are rewritten (Step 5) the grep prints exactly ONE line, `worker/README.md:294` — the lobby screenshot's alt text, "Open tables reads no table is waiting for players", which is about open tables and stays (P-6; `web/src/Lobby.tsx`'s sentence is that same one and is not in the grep's paths).

- [ ] **Step 5: Rewrite the five document lines** to quote the new head.

- [ ] **Step 6: Commit**

```bash
git add worker/src/marker.ts worker/src/marker.unit.test.ts worker/test/expiry.test.ts worker/README.md worker/RUNBOOK.md docs/web-lane.md
git commit -m "worker: the marker head for a table closed before it started reads 'waited … to start', true of an open and of a dealing table; no stored field moves (S4-pre Task 3; the spec's Issue 2)"
```

Mutants for the reviewer: the old head restored; a constant 7; "1 days"; the `expired` head under `waited`.

---

### Task 4: Issue 3 — the NAS's retired keyed-build reason · impl: sonnet · review: opus

The spec's Issue 3. Reword, do not re-decide: the rule stays (a keyed build is refused behind `cf-access` and `worker`, naming both), the REASON becomes the present one.

**Files:**
- Modify: `crates/yokohama-web/src/assets.rs` (the doc at 51–60; the refusal at 90–95; the cell's twin `keyed_behind` at 646–650 and its doc at 655–662)
- Modify: `crates/yokohama-web/src/main.rs:128–132` (the comment)
- Modify: `deploy/update.sh:34` (the comment) and `:120` (the `die` message)
- Test: `assets.rs` (the existing cell with its moved literal; one new cell); `crates/yokohama-web/tests/start_refusal.rs` builds its expectation from `keyed_build_refusal` itself and follows

**Interfaces:**
- Produces: the sentence `keyed_build_refusal` builds on its `Ok(true)` arm — the `Err(String)` the operator reads, `assets.rs` ~90–95 — rewritten (P-7: the function returns `Result<(), String>`; "the `Ok(true)` arm" names the match arm, not a return value). The reason the implementer writes is read from `web/src/outbox.ts:47–59` (rule 6b: a keyed build holds a nameless visitor's finished game for a name ONLY where the server's facts say `open`) and `deploy/README.md:136` and `:188` (the key "belongs only with `YOKOHAMA_WEB_IDENTITY=open` on this server"), and says: a keyed build belongs with `{IDENTITY_VAR}=open` on this server, because the challenge the key draws is the open door's and the public Worker's and this server behind `{door}` already has an identity provider in front; it names the file, the key variable and the door as today and keeps the two remedies. It must not say "is the open door's" or "reads the key as a door".

- [ ] **Step 1: Write the failing cell in `assets.rs`**, beside the existing one:

```rust
#[test] fn the_keyed_build_refusal_gives_the_clients_present_reason_and_not_the_retired_one() {
    // for door in [CfAccess, Worker]: let s = keyed_build_refusal(Stamp::Text(&stamp(true)), SHOWN, door).unwrap_err();
    // assert!(s.contains("YOKOHAMA_WEB_IDENTITY=open")); assert!(!s.contains("the open door's")); assert!(!s.contains("reads the key as a door")); assert!(s.contains(door.name()))
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p yokohama-web --lib the_keyed_build_refusal_gives`
Expected: FAIL on `!s.contains("the open door's")`.

- [ ] **Step 3: Rewrite the sentence** in `keyed_build_refusal`, move `keyed_behind`'s literal with it, reword the two doc comments and `main.rs`'s, and the two `update.sh` lines.

- [ ] **Step 4: Run to verify**

Run: `cargo test -p yokohama-web && cargo test -p yokohama-wasm && cargo test -p yokohama-worker && cargo clippy --workspace --all-targets -- -D warnings && (cd web && npm run build:wasm && npm run test:wasm) && bash deploy/update.sh --dry-run v0.7.2 && git grep -n -i "keyed build is the open door's\|reads the key as a door" -- . ':!docs/plans' ':!docs/temp'`
Expected: green (`start_refusal` included; the module suites at their baselines — `assets.rs` is in the layer's crate, so the replay runs); the dry run prints its steps; the grep prints nothing.

- [ ] **Step 5: Commit**

```bash
git add crates/yokohama-web/src/assets.rs crates/yokohama-web/src/main.rs deploy/update.sh
git commit -m "nas: the keyed-build refusal gives the client's present reason — a keyed build belongs with the open door on this server — in the refusal, its comments, the cell's twin and update.sh; the rule unchanged (S4-pre Task 4; the spec's Issue 3)"
```

Mutant for the reviewer: the old sentence restored (the new cell reads red; `start_refusal` stays green, which is why the new cell exists).

---

### Task 5: Issue 4 — the secrets guard refuses a file with no door or an unknown door · impl: sonnet · review: opus

The spec's Issue 4. A guard run alone must not pass a reading it cannot make.

**Files:**
- Modify: `worker/scripts/secrets-guard.mjs` (the header at 15–19; a `doorRefusal(judged)` beside `sourceRefusal` at ~73–82; `judgeSecrets`'s return at ~110)
- Modify: `worker/scripts/secrets-guard.test.mjs` (the row at ~94 replaced, an unknown-door row added, in the `it` at ~90–95; a command cell beside the source one at ~196–209)

**Interfaces:**
- Produces: `function doorRefusal(judged)` → `[]` for no judged file or a door of `"worker"` or `"open"`; otherwise one sentence `secrets-guard: ${file}'s YOKOHAMA_WEB_IDENTITY ${shell}; the secrets it requires cannot be judged without a door`, with `${shell}` the shell's own words (`worker/src/config.ts:168–169`): absent → "must be worker or open — it is not set, and there is no default: a door is a deployment decision"; otherwise "must be worker or open — got <JSON of the value>". The order of `judgeSecrets`'s sentences: the forbidden names, the door, the source, the missing secrets. The implementer reads `doorOf` in `deploy-guard.mjs` first, to know what an absent and an unknown door read as.

- [ ] **Step 1: Write the failing cells.** Replace the "no door" row with `expect(judgeSecrets("[]", { file: "wrangler.jsonc", door: undefined, source: "workos" })).toEqual(["secrets-guard: wrangler.jsonc's YOKOHAMA_WEB_IDENTITY must be worker or open — it is not set, and there is no default: a door is a deployment decision; the secrets it requires cannot be judged without a door"])` and add the unknown-door row (`door: "Worker"` → the "got \"Worker\"" sentence); the open-door rows stay as they are. Add the command cell "run alone, the command refuses a file with no door or an unknown one, and passes the open door" over the fake `npx` as the source cell at ~198 does: `judgedIn({})` → status 1 and the sentence on stderr; `judgedIn({ YOKOHAMA_WEB_IDENTITY: "Worker" })` → status 1 and the "got" sentence; `judgedIn({ YOKOHAMA_WEB_IDENTITY: "open" })` → `[0, ""]`.

- [ ] **Step 2: Run to verify they fail** (alone — no cargo build running)

Run: `cd worker && npm run test:scripts -- secrets-guard`
Expected: the replaced row and the new rows fail; the command cell fails on the status.

- [ ] **Step 3: Implement `doorRefusal`**, place it in `judgeSecrets`'s list, and rewrite the header's sentence on what the guard refuses.

- [ ] **Step 4: Run to verify they pass**

Run: `cd worker && npm run test:scripts`
Expected: green; the file's count at its baseline + 1.

- [ ] **Step 5: Commit**

```bash
git add worker/scripts/secrets-guard.mjs worker/scripts/secrets-guard.test.mjs
git commit -m "worker: the secrets guard refuses a file with no door or one the shell does not know, in the shell's sentence with the file named, before the source and the secrets (S4-pre Task 5; the spec's Issue 4)"
```

Mutants for the reviewer (in the shape of the source refusal's three): the door unjudged; only an absent door refused; only an unknown one refused; the door sentence placed after the missing-secret sentences.

---

## Before Task 6 (the orchestrator, on the owner's go)

- [ ] **Post the three issues** the wave does not build — the draft's Issues 5, 6 and 7 (`docs/temp/2026-10-22-web-lane-leftovers-issues.md`), each in the layout the specification's header names (Title; Labels; Context, What happens today, The change, Where, Acceptance, Size, Record to update when it lands). First the labels that do not exist: `gh label create web`, `worker`, `nas`, `client`, `polish`, `security-ledger` (GitHub's `documentation` stands in for `docs`); then `gh issue create --title … --label … --body-file …` three times, the bodies cut from the draft verbatim. Posting is outward-facing: it waits for the owner's word in the session. Record the numbers (#57 onward, since pull requests share the counter) in the SDD ledger; Task 6's brief carries them. The design's §2.24 (the owner, 2026-10-23): the posted issues' style is evaluated again once `CONTRIBUTING.md` and the templates exist — issues are editable, so S4a's community-files task carries that pass; nothing here waits for it.

### Task 6: the records — the nine "S4's" sites, the lane's §8, D32, passkeys, the board · impl: sonnet · review: opus

The spec's Part 2 ((a)–(d) and "Other sites"), its items 7 and 8, and the board. Every replacement sentence is drafted in the spec; the implementer transcribes the "wave lands it" variants with the commits of Tasks 1–5 and the posted issue numbers, and reads each site once before writing. Dated documents under `docs/plans/` are not edited.

**Files:**
- Modify: `docs/security-ledger.md` (row C1 at ~27: the clause → Part 2 (a); row D3 at ~33: the clause → Part 2 (c)'s "wave lands it" draft naming Task 1's and Task 2's commits and catchers, the "Recorded, not closed" tail rewritten to say a seated player is no longer locked out by a watcher fill; row D32 at ~62: the closing clause of item 7; the AUTH row at ~63: "(accepted undone, the owner, 2026-10-22)"; the `Updated` line at 5)
- Modify: `docs/web-lane.md` (§8's "Still owed" paragraph at ~414–423 rewritten as ONE paragraph from Part 2 (b) and findings 7–8 — the fresh-history and art preconditions answered by the S4 design §2.1 and §2.3, "on a public Worker that starts empty" retracted, the auth paragraph's header at ~425 brought to "stage 1 live since 2026-10-19; stage 2 is S4c's window"; §15 at ~998, ~1002 and ~1004 from the "Other sites" drafts, the two D3 rows naming the watchers' figure and Task 1's/Task 2's commits; §4 at ~162, the Worker's caps clause gaining the watchers' figure; §13's stream rows at ~818–819 naming the watchers' refusal)
- Modify: `worker/RUNBOOK.md` (the cap sentences at ~105–108 gain the fifth; the sentence at ~273 → the "Other sites" draft)
- Modify: `docs/handover.md` (line ~122, housekeeping item 4: the Rust sentences and the guard done in this wave with their commits, the sign-in block and the encoders → "issue #N"; line ~128, "S4's" → "superseded by the provider-signed door, 2026-10-22"; line ~140, items (c) and (d) done; rulings 10 and 11 closed with one line each; a line naming the two polish issues by number)
- Test: `cargo test -p yokohama-web --test lane_document` (§13 and §16 held to the code) and `cd worker && npm run test:scripts` (`lane-variables.test.mjs` holds §12)

**Interfaces:**
- Consumes: the commits of Tasks 1–5 (`git log --oneline origin/master..HEAD`), the issue numbers from "Before Task 6".

- [ ] **Step 1: Read each site** named above at the branch's head (the spec's line numbers are readings at `56460688`), and list in the transcript the line each sentence now sits on.
- [ ] **Step 2: Apply the drafts**, site by site, in the spec's words with the commits and numbers filled in; the lane's §8 paragraph as one rewrite, not a patch.
- [ ] **Step 3: Verify the cells and the greps**

Run: `cargo test -p yokohama-web --test lane_document && (cd worker && npm run test:scripts) && git grep -n -E "is S4's|S4's controls|recorded for S4|S4's public Worker" -- . ':!docs/plans' ':!docs/temp' ':!.superpowers'`
Expected: the two cell sets green; the grep prints nothing.

- [ ] **Step 4: Commit**

```bash
git add docs/security-ledger.md docs/web-lane.md worker/RUNBOOK.md docs/handover.md
git commit -m "docs: the web lane's deferrals re-recorded — D3 fixed with its catchers, C1 and the per-person ceiling superseded by the provider-signed door, the lane's §8 brought to the present, D32 closed, passkeys accepted undone, the two polish issues named (S4-pre Task 6)"
```

Review: the reviewer reads every rewritten sentence against the code and the record (the figures, the commits, the issue numbers), and confirms by grep that no dated document moved.

---

## After Task 6 (the orchestrator)

- [ ] **The suites at the head**, once, as one sequence in a fresh export: the Rust three, the lint gate, the Worker's four, and the whole web set (`cd web && npm run typecheck && npm test && npm run test:wasm && npm run e2e`); the counts go in the close.
- [ ] **The whole-branch review** (Fable, on an export of the head, the owner's branch rule): one wave of fixes as its own commit, a scoped re-review on an export of that commit.
- [ ] **The close**, `docs/plans/2026-10-22-security-s4-pre-close.md`: the commits, the counts at the head, the mutants and their verdicts, the posted issue numbers, what the window still owes (Issue 1's live reading at stage 2 — the design §8.5), the board's tracks and guidingpath's row for the wave, the ledger's `Updated` line — and, as its last section, **the handover into S4a**: where the wave left the tree and the record, the issue numbers, and the entry for S4a's planning session (the design's §6 read against the merged tree; the NOTICE's rights-holder paragraph written with the owner present, §6.3; the issues' style pass once CONTRIBUTING.md exists, §2.24). The session after this one opens there: a short brainstorm, then the S4a plan by the writing-plans skill.
- [ ] **The PR** `security-s4-pre` → master, the review record linked; merged on the owner's word. S4a is cut from master after it.

---

## Review record

The "before" review (the orchestrator, 2026-10-23, in the checkout at `56460688` with an export of the same head for the suites), Task 1's and Task 2's cells read against the spec's Issue 1 and the code they cite; each correction applied at its site above and tagged `(P-n)`:

- **P-1** — Task 2's file list counted "9" `ReplyData {` literals; `grep -n 'ReplyData {'` reads 18 lines across five files (`lib.rs` 9, `picker.rs` 4, `receipt.rs` 3, `create.rs` 1, `events.rs` 1), the definition and `-> ReplyData {` signatures among them. The compiler is the catcher, so the task says so rather than a figure.
- **P-2** — Task 1's two route cells read `seated`/`open` after a `stream_status`, which drops its response on return — and the drop frees the slot at once (`Admitted` rides the body; `a_callers_ninth_stream_…` holds `stream_response`s for the same reason). Every counted stream is a held `stream_response`.
- **P-3** — Task 2's module cell covered `open` and `dealing` where the spec's acceptance cell is "at every status and a stranger or a spectator not"; the cell is named as the spec names it and gains a playing table (the two-keep shape the cell near `lib.rs:1041` uses) with a spectator row.
- **P-4** — Task 2's third pool cell passed `call(..)` an object body `{ seat: 1 }`; the helper's `body` is a string (`helpers.ts:34–50`; `seedPlaying` passes `'{"seat":1}'`).
- **P-5** — Task 2's `frames.unit.test.ts` extension: the `it` title, its comment and `frames.ts`'s doc line all say "four sentences"; they say five with the fifth.
- **P-6** — Task 3's grep for `for players` over `worker` has one true survivor the plan did not name, `worker/README.md:294` (a screenshot's alt text quoting the lobby's empty state), and one more site to reword, `expiry.test.ts:72–73`'s `HEAD` doc comment. The expectation and Step 1 say so.
- **P-7** — Task 4's interface said "`keyed_build_refusal`'s `Ok(true)` sentence"; the function returns `Result<(), String>` and the sentence is the `Err` its `Ok(true)` match arm builds. Wording.

Also read, no change: the design's two edit artifacts named at the brainstorm's close ("(the owner:" dangling in §2.11; "a reader that weaker is the point" in §4) are not in the design at `2026-10-23` — the owner's edits removed them. Task 5's cited sites (`doorOf`, `judgeSecrets`'s rows, `judgedIn`, `config.ts:168–169`) read as the plan says. Task 2's `tableVerdict` rows were checked by hand against the verdict order (every row agrees). The vitest configs place `src/**/*.unit.test.ts` under `test:scripts` and `test/**/*.test.ts` under the pool, as Step 9's counts assume.
