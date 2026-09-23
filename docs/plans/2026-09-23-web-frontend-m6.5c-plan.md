# M6.5c "the objects" — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task — **the execution shape is §"Execution shape" below, which records the owner's answers** (the Opus floor for every implementer and reviewer, Fable for the whole-branch review). Steps use checkbox (`- [ ]`) syntax for tracking. **Read the spec first**; this plan argues from it and does not restate its reasons.

**Goal:** Hosted multiplayer runs on Cloudflare at `yokohama.moore.nyc` behind the same Access application as today. It uses a stateless Worker, one Durable Object per table, one house object and one archive object per caller. Every object runs the same table layer the NAS runs, so every payload is byte-identical to the committed corpus. The NAS stays exactly as it is.

**Architecture:** A TypeScript shell (`worker/`) over a per-object WebAssembly module (`crates/yokohama-worker`, a `cdylib` over `yokohama-web` without its server half). The Worker validates Access's assertion, resolves the caller and forwards by name. It never loads the module; static assets are served without invoking it.
- **Each object** owns one module instance, created through a factory-wrapped glue. It persists the layer's documents in its own SQLite in ONE synchronous batch per request, then sends its side channels.
- **The side channels:** hibernatable WebSocket frames per viewer, a durable slot pushed to the house, and an asynchronous R2 upload carried out as a receipt.
- **Code outside the new host:** the layer gains two functions' worth of change. The client gains one transport arm and one outbox branch, and the corpus emitter gains two request scripts.

**Tech Stack:**
- **Rust:** 1.97.1 (pinned), `wasm32-unknown-unknown`, `wasm-bindgen 0.2.128` `--target web`, `wasm-opt` (binaryen **132**).
- **The Worker:** TypeScript on Cloudflare Workers (Free) with SQLite-backed Durable Objects, the WebSocket Hibernation API, alarms and R2; `wrangler` 4.136.x.
- **Tests:** Cloudflare's Workers Vitest integration (`@cloudflare/vitest-plugin` 1.2.3, or its older name `@cloudflare/vitest-pool-workers` 0.22.0; `vitest` 4.1), running inside workerd.
- **Web and live:** React, vitest and jsdom in `web/`; Playwright 1.62.1 against the live Worker; Cloudflare Access with a Service Auth credential.

**Spec:** `docs/plans/2026-09-22-web-frontend-m6.5c-objects-spec.md` at **`92592f2`** (R-6.5c.1–16). §0 is what is TRUE at HEAD with citations, §2 the mechanism, §5 the instruments every task below ends in, §5.7 the battery, §6 what is not claimed, and §9.1–§9.4 what the reviews changed. **The umbrella is `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md`**, and the entry artifact is `docs/plans/2026-09-22-web-frontend-m6.5c-handover.md`. **The spike readings** are `docs/temp/m6.5c-spike-readings-2026-09-22.md` (gitignored). **The spike's throwaway source**, if the scratchpad still has it, is a shape to start from and never an authority.

---

## Global Constraints

- **Branch `web-room-m6.5c`**, stacked on `web-room-m6.5b2`; **the base for every diff is `2621fbe`**.
  - Stage with explicit paths, never `git add -A`. `git status` is clean after every commit.
  - Nothing is pushed until Task 16. The PR goes against `web-room-m6.5b2` and the merge order is the owner's.
- **The five fenced crates and the CLI stay byte-identical, and so does the browser module.** This command prints nothing at every commit:
  `git diff --stat 2621fbe -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli crates/yokohama-wasm`
  `SEMANTICS_REV` stays **7**. No bot behaviour moves.
- **The layer changes by EXACTLY TWO items** (R-6.5c.4): `Games::archive_over_retry` in `games.rs`, and `check_shape` made `pub` in `tables.rs`, each with its own cells.
  - `git diff 2621fbe -- crates/yokohama-web/src` shows those and nothing else, at every commit after Task 1.
  - A third layer change is a STOP: the orchestrator records it as a ruling with its architectural reason, or it does not land.
  - Test code in `crates/yokohama-web/tests/wire_fixtures.rs` is Task 1's and changes nothing it emits except the two new scripts.
- **The committed corpus `web/src/wire/*.json` stays byte-identical.** `web/src/wire-script/` gains exactly two files at Task 1: `walk-host.json` and `archive-walk.json`.
  - Regenerate with `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` **ONLY at Task 1**, and READ the diff.
- **The shell LOGS what the layer cannot** (spec §2.8; the layer's `eprintln!` is mute on this host). It writes one `console.log(JSON.stringify({ event, id, … }))` line for each of these, in the task that builds it:
  - every receipt outcome and every stamp;
  - every slot push and every acknowledgement;
  - every trap and every poison;
  - every expiry, marker end and operator action;
  - every copy `house_copy` refuses, and every save an expiry drops.
  Each task's reviewer checks its task's lines.
- **`web/` changes are only these:**
  - the WebSocket arm (`web/src/ws.ts` is new; `signal.ts`, `api.ts`, `types.ts` and `local/facts.ts` are widened by the `"ws"` value);
  - the outbox's 404 branch (`web/src/outbox.ts`), with its one writer `rememberArchiveOff` in `web/src/local/facts.ts`;
  - the tests of both (Task 8);
  - the live configuration of two Playwright specs (Task 15).
  Nothing else in `web/` moves.
- **The Worker's facts, verbatim from the spec (§3.1):**
  - the Worker is **`yokohama-web`**, on the custom domain **`yokohama.moore.nyc`**, with `workers_dev: false`, `preview_urls: false` and `observability.enabled: true`;
  - `compatibility_date` **`2026-09-15`** (pinned; ≥ 2026-02-24, so `deleteAll()` also deletes the alarm);
  - three SQLite-backed classes in ONE migration: `Table`, `House`, `Archive`;
  - the bucket **`yokohama-archive`** bound as **`ARCHIVE`**;
  - the three Durable Object namespaces bound as **`TABLE`**, **`HOUSE`** and **`ARCHIVER`** (the last because `ARCHIVE` is the bucket's name).
- **The Worker's variables (§3.1):**
  - `ACCESS_TEAM_DOMAIN`, `ACCESS_AUD`, `DEV_DOOR_CLIENT_ID`;
  - `YOKOHAMA_DEV_DOOR` (`on` this cycle), `YOKOHAMA_WEB_ARCHIVE` (`r2` | `off`, absent = off), `YOKOHAMA_WEB_TABLE_TTL_DAYS` (absent = never), `YOKOHAMA_WEB_ADMINS` (absent = the operator routes are the layer's 404), `YOKOHAMA_WEB_COUNT_WAKES` (off except in the exit's window);
  - the three chat variables at the NAS's names and defaults.
  - **One test-only variable this plan adds: `YOKOHAMA_WEB_TEST_CLOCK`.** It is `frozen:<n>` or `tick:<n>`, and it is set ONLY in `worker/wrangler.test.jsonc`. RUNBOOK §6 names it "never set in production".
- **Secrets:** `.dev.vars` holds the `yokohama-worker` Service Auth credential (gitignored since `952a68c`).
  - **Never print it, never commit it, never paste it into a report.** Scripts read it from the file.
  - **Its two keys are named `CF-Access-Client-Id` and `CF-Access-Client-Secret`** — the header names Access expects. The lines are `KEY=VALUE`. The names are stated here so nobody has to open the file to learn them.
  - **A shell command presents it WITHOUT putting the secret on argv or on screen.** A one-off `node -e` reads `.dev.vars` and writes a header file of two `Name: value` lines into the session scratchpad, `chmod 600`. Then `curl -H @<that file> …`, and the file is deleted afterwards.
  - **`YOKOHAMA_WEB_ADMINS` is set with `wrangler secret put`**, so the owner's address never enters a committed file (a plan ruling, see Ruling P-1 below).
- **Live actions on the owner's account are the ORCHESTRATOR's and never a subagent's**: `wrangler deploy`, the bucket, variables and secrets, `wrangler tail`, and the live Playwright and replay runs.
  - Each is recorded in RUNBOOK §6 with its readings.
  - A subagent prepares the code and the commands; the orchestrator runs them.
- **Per-edit gates after EVERY task.** Re-count at your own HEAD; a figure in any document is a reading, not a constant.
  - **Rust:**
    - `cargo test -p yokohama-web` (289 at the base) and `cargo test -p yokohama-wasm` (5);
    - **`cargo test -p yokohama-worker`** (from Task 2);
    - `cargo clippy --workspace --all-targets -- -D warnings`;
    - `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`;
    - `cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm` and **`cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker`** (from Task 2).
  - **In `web/`:** `npm run typecheck`, `npm test` (1024 in 55 at the base) and `npm run test:wasm` (32).
  - **In `worker/` (from Task 7):** `npm run typecheck`, `npm run test:scripts` and `npm test` (the pool, over the debug module).
  - **From Task 12, the pool's corpus replay is part of `npm test`.**
  - **`npm run e2e` in `web/`** is owed by any task that changes what a browser DOES (Task 8). Its run is sequenced, never two Playwright runs at once.
- **The whole-game gate's vs-bot arm livelocks on some OS-entropy seeds** (M6.4 close §6.3). Re-run it ALONE at the same commit before believing a red.
- **Every 400 sentence the layer's parsers produce is axum's word for word.** A body goes through `api::parse_json` and a query through `api::parse_query`. The module never re-serializes a request body (spec §0.4, §2.2).
- **The record and the table document are STRINGS end to end.** Nothing in the shell `JSON.parse`s a record: the seed is a `u64` that a JS number rounds.
- **Any edit to a workflow file** gets `python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/test.yml` in its gate (handover §4 item 12).
- **The ledger** is `.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/` (gitignored).
  - `progress.md` holds one line per task, one `Ruling:` line per deviation from this plan, and every deploy's readings.
  - Every agent writes its full report to a file there and replies with the PATH and three lines; the notification channel truncates at about 4k.
- **Commit messages end with the EXECUTING agent's own attribution lines**, never the plan author's.
- **`cargo test` takes ONE positional filter.** Where a step names several cells, run one command per cell, or a shared substring.

### Rulings this plan takes (each is recorded in the ledger at the task that meets it)

- **P-1 — `YOKOHAMA_WEB_ADMINS` is a `wrangler secret`, not a `vars` entry.** The spec (§3.1) lists it among the variables and says "no secrets". Its VALUE is not secret, but it is the owner's email address, and `worker/wrangler.jsonc` is committed to a repository headed for open source. A secret binding reads as an environment variable exactly as a var does, so no code differs. RUNBOOK §6 says so.
- **P-2 — the SHELL pins the module's clock before EVERY module call that can reach the layer's clock** — `create`, `dispatch`, `events`, `restore_picker`, `stamp_archive` and `expire` — and records the reading it pinned. `events` and `restore_picker` belong on that list because a handle rebuild can pump an owed bot move, persist it, and reach the game-over archive site. That is how the shell always knows the reading a plan was made at (spec §2.6: `archive_owed` is "the plan and its clock reading"), game over included, where the move's dispatch is not re-run.
  - The reading is `Math.floor(Date.now() / 1000)` in production.
  - `YOKOHAMA_WEB_TEST_CLOCK=frozen:<n>` gives `n` for every dispatch. That is the corpus replay's arrangement: the node replay's `1_789_000_000`.
  - `tick:<n>` gives `n`, `n+1`, … one per dispatch. That is what the archive cells need: an unpinned re-dispatch must stamp a DIFFERENT `archived_at` (spec §5.2). workerd's `Date.now()` does not advance while code runs, so a wall clock cannot give them that.
  - The receipt loop's re-dispatch and `stamp_archive` pin the RECORDED reading instead of a fresh one. That is the whole of the clock-pin rule, and battery 17 is its mutant.
  - **ANY reply's `archive` plan is committed as `archive_owed`** in that request's batch, with the reading pinned for the call that made it: the wake's `restore_picker`, a GET's `dispatch`, a move's. It is never only a move's (plan review M4).
  - The module's own `Date.now()` arm is only a fallback for a caller that pinned nothing.
- **P-3 — `worker/` is its own npm package.** It has its own `package-lock.json` and the Cloudflare toolchain as dev dependencies. `web/`'s package gains nothing from this cycle.

---

## Review Focus

The spec does not name these five inputs, and no task's tests would meet them unless someone put them there. Each line says what a person would expect; its test lives in the owning task and is marked **(Review Focus N)** there.

1. **A create body with whitespace or a newline after its closing brace** — `{"players":2,…}\n`, which `curl --data @file` and most editors produce. The textual id insertion must find the object's final `}`, not the body's last byte. Expected: the table is created at the object's name. (Task 3)
2. **A tab hidden and shown again during a game, on the WebSocket arm.** The arm closes its own socket on hide (R-P8) and must reopen on show and resynchronise from the first frame, without a probe, a reload or a "reconnecting…" that stays. (Task 8)
3. **A deploy during a game** — every socket closes with 1006 and every object restarts (spec §5.5 names the no-deploy window for this reason). Expected:
   - every client's arm probes once, is answered, and reconnects on the ladder;
   - an open picker survives, because its lines are re-fed;
   - no acknowledged move is lost.
   (Task 8 for the client, Task 10 for the object: sockets open, then `abortAllDurableObjects()`, then a move from a reconnected socket's screen.)
4. **A table name and a display name with non-ASCII text** (an accent, an emoji, a CJK name), through the create, the house copy, the lobby row and the operator listing. Expected: the same characters everywhere, byte-identical to what the NAS would list. (Task 5 natively, Task 12 through the objects)
5. **One person with two tabs on one table**: two sockets under one attachment user. Expected: both receive every frame, and a move made in one arrives in the other. (Task 10)

---

## Execution shape (the owner's answers, asked when this plan was written)

- **Mode (the owner, 2026-09-23): SUBAGENT-DRIVEN, EVERY TASK.**
  - Each of Tasks 1–16 gets a fresh **Opus** implementer and a fresh **Opus** task reviewer. **Fable** does Task 16's whole-branch review.
  - **The orchestrator runs every step marked ORCHESTRATOR itself**, never through a subagent: the deploys, the bucket, secrets, `wrangler tail`, and the live Playwright and replay runs. Those act on the owner's account and read the credential.
  - Standing process rules, from the lane's memory:
    - never follow up an implementer's DONE without reading HEAD first;
    - a reviewed SHA is never amended — a fix is a new commit;
    - while an implementer works, run nothing in its tree and arm no timers;
    - wake a stalled agent by message rather than spawning another;
    - every agent writes its full report to a file in the ledger and replies with the path and three lines.
- **The TDD commit shape (the owner, 2026-09-23; handover §4 item 26): ONE GREEN COMMIT PER TASK.**
  - Tests are written first and RUN RED. The red run's output — the failing cell names, and the compile error or assertion — is quoted in the commit message's body and in the ledger.
  - Every commit builds and passes every per-edit gate, so bisect works and "suite green after every task" holds.
  - A task reviewer checks that the quoted red run exists and names the task's new cells.

---

## Pool facts (MEASURED by the plan's review; every pool cell obeys them)

The plan's review built a minimal project on the pinned pool and ran it: `@cloudflare/vitest-plugin` 1.2.3, `vitest` 4.1.11, `wrangler` 4.136.3, and workerd 2026-09-21 at `compatibility_date` 2026-09-15. The report is `.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/review-plan.md`, and its scratch project is `…/scratchpad/plan-review/pool/`. These are its findings, and each one changes how a cell must be written.

1. **To hold, fail or observe an RPC method, patch the PROTOTYPE, never the instance.** The pool's RPC resolver refuses an own property: `if (Object.hasOwn(instance, key)) throw new TypeError("The RPC receiver does not implement the method …")` (`@cloudflare/vitest-plugin/dist/worker/lib/cloudflare/test-internal.mjs:510-521`).
   - So a `runInDurableObject(house, h => { h.apply = … })` makes every call THROW. It never hangs, and that made the first battery-25 cell vacuous a second time: correct code and mutant both answered `true`.
   - Patch `House.prototype.apply`, imported from `../src/house`, and keep the original.
   - **To HOLD a call**, await a gate the cell resolves: `let release!: () => void; const gate = new Promise<void>(r => (release = r));`. **To hold ONE revision**, key a `Map<number, Promise<void>>` by `slot.revision`. **To fail once**, a patch that restores itself and throws.
2. **Restore every patch in a `finally`**, and BEFORE any `runDurableObjectAlarm` that follows an abort. A module-level patch SURVIVES `abortAllDurableObjects()` and `evictDurableObject()`, so an unrestored never-settling gate hangs the alarm's own delivery.
3. **A stub taken before `abortAllDurableObjects()` is dead.** Using it fails with `Application called abortAllDurableObjects().`. Take a FRESH `env.TABLE.get(env.TABLE.idFromName(name))` afterwards.
4. **An eviction discards everything on the live instance.** `evictDurableObject` and `abortAllDurableObjects` both re-run the constructor, so an `Instance`, a config or a field replaced on the live object through `runInDurableObject` is GONE at the wake the cell is about.
   - A cell that must survive an eviction patches at MODULE scope: `Instance.prototype`, or a function exported from `../src/module`, restored in `finally`.
   - For configuration, set the variable in `wrangler.test.jsonc`.
5. **Storage is isolated per FILE, not per test.** Durable Object storage and R2 persist across the tests of one file (measured: test 2 read test 1's put), and `idFromName("house")` is one object for the whole file. So:
   - every cell uses ids and callers of its own, e.g. `${cellName}-t`;
   - every house listing a cell asserts on is filtered to that cell's own ids;
   - `reset()` from `cloudflare:test` in an `afterEach` is the alternative, where a file wants it.
6. **A WebSocket close reason longer than 123 UTF-8 bytes THROWS** (`websocket_close_reason_byte_limit`, on by default since 2026-03-03 and so at this compatibility date). `close(4404, <136 bytes>)` raised `SyntaxError` and left the socket open. Every close reason is truncated at a UTF-8 boundary to 123 bytes; the `end` frame before it carries the meaning.
7. **Measured to work as the plan uses them:**
   - `cloudflareTest({ wrangler: { configPath } })` in `plugins`;
   - a `CompiledWasm` import in a test file;
   - the `ASSETS` binding (`/index.html` → 200, a missing file → 404 under `html_handling: "none"`);
   - calling an exported `handle()` directly;
   - a WebSocket upgrade through `stub.fetch` (101);
   - `evictDurableObject` hibernating a socket with its attachment intact;
   - `abortAllDurableObjects()` closing sockets 1006;
   - `runDurableObjectAlarm`'s true and false;
   - `sql.databaseSize`;
   - `transactionSync` with `setAlarm` in one turn;
   - `ctx.waitUntil` in a Durable Object;
   - `blockConcurrencyWhile` around awaited R2 I/O;
   - R2's `onlyIf: new Headers({ "If-None-Match": "*" })` returning `null` on a present key;
   - Durable Object to Durable Object RPC.
8. **`ctx.id.name` is the object's name inside the object**, in `fetch` and `alarm` alike (Cloudflare, 2026). The plan uses it, and `meta` does not store an `id`. Task 9's cell asserts it inside `alarm()`. If that cell fails, fall back to `meta.id`, and record a ruling.

---

## Names shared across tasks

An implementer sees only its own task. The names below are the contract between tasks and are used verbatim.

**The module's exports** (`crates/yokohama-worker`, through `worker/src/module.ts`):

| export | signature (JS side) | task |
|---|---|---|
| `init(config)` | `(config: string) => void` — throws a string naming the variable on a bad value | 2 |
| `dispatch(...)` | `(user: string \| undefined, method: string, path: string, query: string, contentType: string \| undefined, body: Uint8Array) => Reply` | 2 |
| `load(doc, id, text)` | `doc` ∈ `"record" \| "table" \| "chat" \| "house"`; a CREATE (refuses a present document) | 2 |
| `read(doc, id)` | `=> string \| undefined` — the store's exact bytes | 2 |
| `set_clock(s)` | `(seconds: number \| undefined) => number \| undefined` — returns the PRIOR setting (`undefined` = the wall clock) | 2 |
| `identity(user)` | `(user: string \| undefined) => Reply`. 200 `{"user":"<normalized>"}`, or 401 carrying `identity::resolve`'s `worker`-mode sentence in the layer's error shape. It is what the house's `/api/stream` admits with: that route is the axum ROUTER's and not in the layer's table, so `dispatch` would answer it 404. | 2 |
| `create(user, body, id, contentType)` | `=> Reply` | 3 |
| `events(id, user, query)` | `=> Reply` (status + body; `bumps` is always `[]`) | 3 |
| `restore_picker(id, row)` | `(id: string, row: string) => Reply`. `row` is the stored `picker` row's JSON, or `"null"` when there is none. It ALWAYS builds the table's handle first, which is the replay, and that is what makes a record that traps a LOAD trap (Task 9). `Reply.picker` is `"[]"` when the list was cleared and absent when it was kept; `Reply.bumps` carries an owed bot pump the rebuild made, and `Reply.archive` the plan that pump's game-over put deferred (plan review M4 — it is drained exactly as `dispatch` drains it). At an open or dealing table, where `handle()` answers `Err(Open \| Dealing)`, it answers 200 with nothing to report, and the wake treats that as normal (m13). | 4 |
| `house_copy(id)` | `=> string \| undefined` — throws `check_shape`'s reason | 5 |
| `put_copy(id, json)` / `remove_copy(id)` | the house's store, replace / remove | 5 |
| `house_frame(seq, chat)` | `(seq: number, chat: number) => string`. The crate's own two-field struct, serialized; a cell compares it with the SSE frame's `format!` at `lib.rs:699-701` (spec §2.2). | 5 |
| `receipt(planKey, outcome)` | `(planKey: string, outcome: string) => void` | 6 |
| `stamp_archive(id)` | `=> Reply` (calls `Games::archive_over_retry`) | 6 |
| `expire(id)` | `=> Reply` — the layer's own `DELETE /api/games/{id}` as the stored table's `created_by` | 6 |
| `debug_trap()` | under `--features debug` only | 2 |

**`Reply`** (a wasm-bindgen class with getters; the shell reads its fields, then calls `free()`):
- `status: number`, `body: string`;
- `bumps: string` — JSON: a list, in call order, of `{"bump": id}`, `{"bump_table": id}`, `{"bump_house_chat": null}`, `{"end": id}`;
- `archive: string | undefined` — JSON: the deferred **plan** (Task 6);
- `picker: string | undefined` — JSON: the table's new picker list, `"[]"` when cleared, absent when unchanged (Task 4).

**The plan** (Task 6, `archive`):
```json
{"key": "<sha256 hex over record_key|table_key|record_digest|table_digest>",
 "record_key": "test/rev7/over/wire.json", "table_key": "test/rev7/over/wire.table.json",
 "record": "<the record's exact text>", "table": "<the stripped table's exact text>"}
```

**The outcome** (Task 6, `receipt`): `{"outcome": "landed" | "identical" | "conflict" | "failed", "detail": "<R2's message or the key>"}`.

**The init config** (`init`'s JSON; the shell builds it from `Env` in `worker/src/config.ts`, Task 7):
```json
{"archive": "r2" | "off", "caller": "<the Archive object's address>" | null,
 "lobby_chat": "<raw value>" | null, "chat_keep": "<raw value>" | null, "chat_text": "<raw value>" | null}
```

**The shell's names** (`worker/src/`):
- `Env` (bindings and variables, Global Constraints) in `env.ts`;
- `shellConfig(env): ShellConfig | { error: string }` in `config.ts`;
- `route(method, pathname): Target` in `route.ts` (Task 11);
- `resolveIdentity(headers, keys, config): Identity` in `identity.ts` (Task 11);
- `Instance` (the guarded module) in `module.ts` (Task 7);
- `docs` / `meta` storage helpers in `storage.ts` (Task 9);
- `frame(event, data)` in `frames.ts` (Task 10);
- the slot's ordering helpers in `slot.ts` (Task 12);
- `executePlan(bucket, plan)` in `r2.ts` (Task 13);
- `markerSentence(m: Marker)` in `marker.ts` (Task 14);
- the classes `Table` (`table.ts`), `House` (`house.ts`) and `Archive` (`archive.ts`), exported from `index.ts`.

**Internal headers the Worker sets and the objects trust** (every client `x-yokohama-*` header is stripped first, spec §2.3):
- `X-Yokohama-User` — the caller;
- `X-Yokohama-Table-Id` — a create's object name, percent-encoded;
- `X-Yokohama-Operator` — an operator's address, on `/api/admin/*` only (Task 14).

---

## File structure

**Created**
- `crates/yokohama-worker/Cargo.toml`
- `crates/yokohama-worker/src/lib.rs` — the exports and the one `Games`
- `crates/yokohama-worker/src/signal.rs` — `RecordingSignal`
- `crates/yokohama-worker/src/config.rs` — `init`'s config
- `crates/yokohama-worker/src/create.rs` and `events.rs` — Task 3
- `crates/yokohama-worker/src/picker.rs` — Task 4
- `crates/yokohama-worker/src/house.rs` — Task 5
- `crates/yokohama-worker/src/receipt.rs` — Task 6
- `worker/package.json`, `package-lock.json`, `tsconfig.json`
- `worker/wrangler.jsonc` (production) and `wrangler.test.jsonc` (the pool)
- `worker/vitest.config.ts` (the pool) and `vitest.scripts.config.ts` (node)
- `worker/static/_headers` and `_redirects`
- `worker/scripts/`:
  - `build-module.mjs`, `factory-glue.mjs`, `assemble-assets.mjs`, `test-assets.mjs`, `deploy-guard.mjs` (Task 7);
  - `read-deploy.mjs` (Task 11);
  - `live-replay.mjs` (Task 15);
  - and their `*.test.mjs`.
- `worker/src/` — `env.ts`, `config.ts`, `module.ts`, `index.ts`, `identity.ts`, `route.ts`, `storage.ts`, `frames.ts`, `table.ts`, `house.ts`, `archive.ts`, `slot.ts`, `r2.ts`, `marker.ts`, `admin.ts`
- `worker/test/` — the pool cells, and:
  - `helpers.ts` (Task 9);
  - `gates.ts` and `walk.ts` (Task 12), and `corpus.test.ts` (Task 12);
  - `fixtures/owed-pump.{table.json,json}` (Task 2) and `fixtures/owed-game-over.{table.json,json}` (Task 6) — Rust-made, committed.
- `web/src/ws.ts` and `web/src/ws.test.ts`
- `web/src/wire-script/walk-host.json` and `archive-walk.json` — generated at Task 1, committed
- `web/e2e/live.ts` — the live target's context helper (Task 15)
- the close, dated the day it is written (Task 16)

**Modified**
- `Cargo.toml` (the workspace member)
- `crates/yokohama-web/src/games.rs` and `tables.rs` (Task 1's two)
- `crates/yokohama-web/tests/wire_fixtures.rs`
- `web/src/signal.ts`, `api.ts`, `types.ts`, `local/facts.ts`, `outbox.ts` (and their tests)
- `web/e2e/two_browsers.spec.ts`, `joins_by_link.spec.ts`, `web/playwright.config.ts` (Task 15)
- `.github/workflows/test.yml` (Task 7)
- `.gitignore` (`worker/gen/`, `worker/assets/`, `worker/.test-assets/`, `worker/node_modules/`, `worker/.wrangler/`, `worker/test/fixtures.gen.ts`)
- `deploy/RUNBOOK.md` (§6 new from Task 11; §4 rewritten at Task 11)
- `CLAUDE.md`, `docs/handover.md` (Task 16)

**Deleted** (Task 11, after the strip rule is carried into `worker/src/identity.ts`): `deploy/worker/`, `deploy/modal_app.py`.

---

## Task order and why

The order runs from the layer outward, so that each task's tests stand on something already proven:

1. **Task 1** is the only task that touches `crates/yokohama-web`. It lands first so every later task builds on the final layer and the final request scripts.
2. **Tasks 2–6 build the module natively.** Every rule that can be wrong in Rust is proven under `cargo test` before any JavaScript calls it.
3. **Task 7** builds `worker/`: the build, the glue factory, the assets and the pool.
4. **Task 8 is the client.** It lands BEFORE the first deploy, so the first release the Worker serves already understands `"ws"`. A browser that caches a release without the arm would sit on "reconnecting…" until its update is clicked.
5. **Tasks 9–10 are the table object.**
6. **Task 11 is the Worker and THE FIRST DEPLOY**, which re-reads the spike with the real code (spec §3.2).
7. **Tasks 12–14** each add one leg and deploy: the house and slot, the archive on R2, then expiry and the operator. From Task 12 on, the pool replays the whole corpus at every task.
8. **Task 15** is the two live test tiers.
9. **Task 16** is the exit, the battery, the reviews and the close.

```
1 ─► 2 ─► 3 ─► 4 ─► 5 ─► 6 ─► 7 ─► 8 ─► 9 ─► 10 ─► 11 (deploy) ─► 12 (deploy) ─► 13 (deploy) ─► 14 (deploy) ─► 15 ─► 16
```

Task 8 depends only on the spec. It runs in sequence like every other task: the standing rule is that nothing runs in a tree while an implementer works there. The one exception is a separate git worktree (superpowers:using-git-worktrees), made by the orchestrator. It must merge before Task 11.

---

### Task 1: The layer's two changes, and the corpus's two new request scripts (R-6.5c.4; spec §5.3)

**Files:**
- Modify: `crates/yokohama-web/src/games.rs`:
  - add `pub fn archive_over_retry` directly below `fn archive_if_over` (`:682`);
  - add its cells in the file's test module (the `RecordingArchive` double is at `:1865`, `play_to_the_end` at `:1911`).
- Modify: `crates/yokohama-web/src/tables.rs:362` — `pub(crate) fn check_shape` becomes `pub fn check_shape`.
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs`:
  - `struct Walk` (`:725`), `walk_with`'s return (`:1738-1741`), `corpus()` (`:2071`) and the writer test (`:2738`);
  - the `hosts_own` filter at `:2131` keeps its meaning.
- Create (generated, committed): `web/src/wire-script/walk-host.json`, `web/src/wire-script/archive-walk.json`.

**Interfaces:**
- Produces:
  - `impl Games { pub fn archive_over_retry(&self, id: &str) -> Result<(), GameError> }`
    - Completes the game-over archive through the EXISTING `archive_if_over` against the stored record's exact bytes.
    - `Ok(())` and no put for a table that is stamped, unfinished, or has its archive off.
    - `Err(GameError::NotFound(..))` for an id with no record.
  - `pub fn yokohama_web::tables::check_shape(doc: &TableDoc) -> Result<(), String>`.
  - `walk-host.json`: a JSON list of `{"after": i64, …ScriptRow's fields}`.
    - Each is a host-owned request the walk made (`hosts_own`), in walk order.
    - `after` is the index in `walk.json` of the last kept row before it, or `-1`.
    - It also holds ONE row the emitter synthesizes: `{"after": -1, "who": null, "method": "GET", "path": "/api/art", "query": "", "body": null, "status": 200, "fixture": "art_manifest"}`.
  - `archive-walk.json`: a JSON list of `ScriptRow` — the archive `Api`'s calls in order, the eight `archive_*` fixtures each named once.
- Consumes: nothing new.

**Why `archive_over_retry` exists** (spec §2.6):
- On a Durable Object, the game-over put is an R2 upload that happens AFTER the move's reply, so `persist`'s own call cannot complete it.
- The stamp, and its retry on a version conflict, must stay the layer's code rather than a host's copy.
- It is the whole of the retry: read the handle, read the stored record, call `archive_if_over`.

- [ ] **Step 1: Write the three failing layer cells** in `games.rs`'s test module:

```rust
/// R-6.5c.9: a host whose put was DEFERRED completes the game-over archive
/// later, and the stamp is the layer's own — one put, the stored bytes, and
/// idempotent once stamped.
#[test]
fn archive_over_retry_completes_a_game_over_put_that_failed() {
    let games = games().with_archive(RecordingArchive::failing());
    let (_, repl) = play_to_the_end(&games, "late");
    assert!(repl.session.is_over(), "premise: the game is over");
    let (doc, _) = games.read_table("late").unwrap().unwrap();
    assert_eq!(doc.archived_at, None, "premise: the failed put left the stamp unset");

    let archive = RecordingArchive::recording();
    let games = games.with_archive(archive.clone());
    // `play_to_the_end` drives a SECOND, bot-seated `Repl`: the table's own
    // resident handle is still at the opening, so without this eviction the
    // retry reads a game that is not over and the cell fails for the right
    // code (measured by the plan's review). In production the resident handle
    // IS the game, so the function itself must not evict.
    games.evict_for_test("late");
    games.archive_over_retry("late").expect("the retry");
    let stored = games.store().read(Doc::Record, "late").unwrap().unwrap().0;
    {
        let puts = archive.puts.lock().unwrap();
        assert_eq!(puts.len(), 1, "one put");
        assert_eq!(puts[0].why, crate::archive::Why::Over);
        assert_eq!(puts[0].prefix, "", "a server's own game carries the empty prefix");
        assert_eq!(puts[0].record, stored, "the archived bytes ARE the stored bytes");
    }
    let (doc, _) = games.read_table("late").unwrap().unwrap();
    assert!(doc.archived_at.is_some(), "the stamp landed");
    games.archive_over_retry("late").expect("a second retry");
    assert_eq!(archive.puts.lock().unwrap().len(), 1, "a stamped game is not put again");
}

/// Nothing owed, nothing put: an unfinished game, an archive that is off.
#[test]
fn archive_over_retry_does_nothing_for_an_unfinished_game_or_an_archive_that_is_off() {
    let archive = RecordingArchive::recording();
    let running = games().with_archive(archive.clone());
    one_move_in(&running, "mid");
    running.evict_for_test("mid");
    running.archive_over_retry("mid").expect("an unfinished game is not an error");
    assert!(archive.puts.lock().unwrap().is_empty(), "nothing is owed for a game still running");

    // NoArchive. The eviction is what makes this half NON-VACUOUS: without it
    // the stale resident handle reads "not over", and the cell would stay green
    // even under a mutant that dropped the archive-off check.
    let off = games();
    play_to_the_end(&off, "off");
    off.evict_for_test("off");
    off.archive_over_retry("off").expect("an archive that is off is not an error");
    assert_eq!(off.read_table("off").unwrap().unwrap().0.archived_at, None);
    assert!(matches!(off.archive_over_retry("nobody"), Err(GameError::NotFound(_))));
}

/// The stamp's version-conflict retry is the layer's code, and this is the one
/// cell that reaches it: an archive whose put WRITES the table document — the
/// shape of an `unlock` landing in the same instant — so `archive_if_over`'s
/// compare-and-swap meets a version it did not read, re-reads, and stamps over
/// the newer document without losing it.
#[test]
fn archive_over_retry_stamps_through_a_concurrent_table_write_and_keeps_it() {
    struct Meddling {
        store: Arc<dyn Store>,
    }
    impl crate::archive::Archive for Meddling {
        fn put(&self, _: &str, _: crate::archive::Why, id: &str, _: &str, _: &str)
            -> Result<crate::archive::PutOutcome, crate::archive::PutError> {
            let (text, version) = self.store.read(Doc::Table, id).unwrap().unwrap();
            let mut doc: crate::table::TableDoc = serde_json::from_str(&text).unwrap();
            doc.unlocked.push("meddler@t.test".to_string());
            let json = serde_json::to_string(&doc).unwrap();
            self.store.write_if_version(Doc::Table, id, &json, Some(version)).unwrap();
            Ok(crate::archive::PutOutcome::Landed)
        }
        fn kind(&self) -> &'static str { "meddling" }
    }
    let games = games().with_archive(RecordingArchive::failing());
    play_to_the_end(&games, "raced");
    let store = Arc::clone(games.store());
    let games = games.with_archive(Arc::new(Meddling { store }));
    games.evict_for_test("raced"); // the resident handle is the opening's — see the first cell
    games.archive_over_retry("raced").expect("the retry");
    let (doc, _) = games.read_table("raced").unwrap().unwrap();
    assert!(doc.archived_at.is_some(), "the stamp was retried over the concurrent write");
    assert!(doc.unlocked.iter().any(|u| u == "meddler@t.test"), "and the concurrent write was kept");
}
```

- [ ] **Step 2: Watch them fail.**

```bash
cargo test -p yokohama-web archive_over_retry
```

Expected: they fail to COMPILE — there is no `archive_over_retry`.

- [ ] **Step 3: The function**, directly below `archive_if_over`:

```rust
    /// The game-over site's retry as a public door (M6.5c, R-6.5c.4, R-6.5c.9).
    ///
    /// A host whose archive put is ASYNCHRONOUS — a Durable Object uploads to R2
    /// after the move's reply — cannot complete it inside [`Games::persist`], so
    /// it calls this once the upload has landed. The stamp and its retry on a
    /// version conflict stay [`Games::archive_if_over`]'s own code, never a
    /// host's copy. Idempotent: a stamped, unfinished or archive-off table puts
    /// nothing.
    pub fn archive_over_retry(&self, id: &str) -> Result<(), GameError> {
        let handle = self.handle(id)?;
        let repl = handle.lock().expect("a game");
        let Some((record_json, _)) = self.store.read(Doc::Record, id).map_err(GameError::Server)? else {
            return Err(GameError::NotFound(format!("no game named {id:?}")));
        };
        self.archive_if_over(id, &repl, &record_json);
        Ok(())
    }
```

And in `tables.rs`, `pub(crate) fn check_shape` becomes `pub fn check_shape`. Add one doc line saying why: a host that cannot hear the layer's `eprintln!` refuses a copy its lobby would drop (M6.5c, R-6.5c.8).

- [ ] **Step 4: Watch the cells pass**, then the whole binary:

```bash
cargo test -p yokohama-web archive_over_retry
cargo test -p yokohama-web
```

- [ ] **Step 5: The emitter's two scripts — first the failing check.**

In the writer test `the_committed_wire_fixtures_are_what_the_server_emits`, after the `walk.json` block, check two more files exactly as `walk.json` is checked: written under `UPDATE_WIRE_FIXTURES`, compared byte for byte otherwise. Then add a new cell:

```rust
/// **The two scripts a Worker host replays beside `walk.json`** (M6.5c, spec
/// §5.3). Each is DERIVED from the walk and named by value, never written
/// down: a script that stopped naming a payload would move that payload into
/// the Worker replay's third column with nothing saying so.
#[tokio::test]
async fn the_host_scripts_name_the_payloads_only_a_host_or_an_archive_serves() {
    let Corpus { script, host_script, archive_script, .. } = corpus().await;
    // walk-host.json: every row is a host-owned route, ordered by `after`,
    // `after` indexes a real walk row, and the four host payloads are each
    // named exactly once — the three `events*` and the synthesized art row.
    let mut last = -1i64;
    for h in &host_script {
        assert!(hosts_own(&h.row.path), "{} is not host-owned", h.row.path);
        assert!(h.after >= last, "host rows keep the walk's order");
        assert!(h.after >= -1 && h.after < script.len() as i64, "after={} indexes no walk row", h.after);
        last = h.after;
    }
    let named = |rows: &[&ScriptRow]| {
        let mut n: Vec<String> = rows.iter().filter_map(|r| r.fixture.clone()).collect();
        n.sort();
        n
    };
    assert_eq!(
        named(&host_script.iter().map(|h| &h.row).collect::<Vec<_>>()),
        ["art_manifest", "events", "events_dealing", "events_open"],
    );
    let art = &host_script[0];
    assert_eq!((art.after, art.row.path.as_str(), art.row.who.as_deref()), (-1, "/api/art", None));
    // archive-walk.json: the archive `Api`'s calls, the eight each named once.
    assert!(archive_script.iter().all(|r| r.method == "POST" && r.path == "/api/archive"));
    assert_eq!(
        named(&archive_script.iter().collect::<Vec<_>>()),
        [
            "archive_400_digest", "archive_400_replay", "archive_400_rev", "archive_400_table",
            "archive_409", "archive_post", "archive_post_abandoned", "archive_post_identical",
        ],
    );
}
```

```bash
cargo test -p yokohama-web --test wire_fixtures the_host_scripts_name
```

Expected: COMPILE failure — `Corpus` has no `host_script` / `archive_script`.

- [ ] **Step 6: Emit them.**
  - **`Walk` gains `archive: Vec<ScriptRow>`**: the archive `Api`'s calls, kept apart. They are also still pushed into `aside`, whose classification job is unchanged.
  - **In `corpus()`, the host rows are split out before `script.retain(|r| !hosts_own(&r.path))` runs.** Walk the script with a running count of KEPT rows; a host-owned row becomes `HostRow { after: <kept - 1>, row }`, and every other row increments the count.
  - **Prepend the synthesized art row**, with `fixture: Some("art_manifest")` and `digest: [0; 32]`: it is never bound by digest.
  - **Bind the archive rows by value** with `digest_of`, as the primary binding does: each of the eight `archive_*` payloads finds the first unbound archive row whose frozen response equals it. A payload that matches no row is a hard failure.
  - **`HostRow` serializes flat**: `#[serde(flatten)] row: ScriptRow` beside `after`.

- [ ] **Step 7: Regenerate, and READ the diff.**

```bash
UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures
git status --short web/src/wire web/src/wire-script
git diff --stat web/src/wire web/src/wire-script/walk.json
```

**Expected, and nothing else**:
- two NEW files, `web/src/wire-script/walk-host.json` (four rows at the base — three `events*` and the art row) and `archive-walk.json` (the archive `Api`'s calls);
- `web/src/wire/` and `walk.json` byte-identical (the `diff --stat` prints nothing).

A changed payload or a changed `walk.json` is a FINDING. Stop and report it rather than committing it.

- [ ] **Step 8: Run the checks WITHOUT the variable**, and the node replay, which reads none of the new files and keeps its `NOT_SERVED` list unchanged:

```bash
cargo test -p yokohama-web --test wire_fixtures
cd web && npm run build:wasm && npm run test:wasm
```

- [ ] **Step 9: The per-edit gates** (Global Constraints), including `git diff 2621fbe -- crates/yokohama-web/src`, which must show exactly `archive_over_retry` plus its cells and the `pub` on `check_shape`.

- [ ] **Step 10: Commit.**

```bash
git add crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire-script/walk-host.json web/src/wire-script/archive-walk.json
git commit -m "feat(web): the layer's two M6.5c changes, and the corpus's host and archive request scripts"
```

---

### Task 2: `crates/yokohama-worker` — the core: `init`, `dispatch`, the recording signal, the side channel, the clock (spec §2.2)

**Files:**
- Modify: `Cargo.toml` (root):
  - `members` gains `crates/yokohama-worker`;
  - the `[profile.release-wasm]` comment says "The browser host's module, and NOTHING else", which becomes false at this task. It now names both hosts' modules, and keeps the M6.5b2 figures as the BROWSER module's (plan review m11).
- Create: `worker/test/fixtures/owed-pump.table.json` and `owed-pump.json` — generated, committed (below).
- Create: `crates/yokohama-worker/Cargo.toml`, `src/lib.rs`, `src/signal.rs`, `src/config.rs`.

**Interfaces:**
- Consumes (the layer, public at `2621fbe` plus Task 1):
  - `Games::with_parts(rules, store, chat, signal, clock, archive)` (`games.rs:287`), `Games::load_document` / `read_document` (`:352`, `:359`);
  - `api::dispatch(games, &Host, Identity, &ApiRequest) -> ApiResponse` (`api.rs:1984`);
  - `identity::resolve(IdentityMode::Worker, &http::HeaderMap) -> Result<User, String>` (`identity.rs:157`) — the `worker` mode's ONE header, `X-Yokohama-User`, where `anonymous` counts as absent;
  - `chat::ChatConfig::from_env_values(lobby, keep, text)` (`chat.rs:165`);
  - `signal::Signal` (`signal.rs:37`), `archive::NoArchive`, `store::{MemStore, Doc}`.
- Produces: the exports `init`, `dispatch`, `load`, `read`, `set_clock`, `debug_trap` and the `Reply` class, exactly as §"Names shared across tasks" states. Also, for Tasks 3–6, the Rust items:
  - `static GAMES: Mutex<Option<Games>>`, and `static STORE: Mutex<Option<Arc<MemStore>>>` — the SAME `Arc` the `Games` holds, so Tasks 4–5 can replace and remove documents through the `Store` trait;
  - `static SIGNAL: Mutex<Option<Arc<RecordingSignal>>>`;
  - `pub fn dispatch_inner(user: Option<&str>, method: &str, path: &str, query: &str, content_type: Option<&str>, body: &[u8]) -> ReplyData`;
  - `pub struct ReplyData { pub status: u16, pub body: String, pub bumps: Vec<Bump>, pub archive: Option<String>, pub picker: Option<String> }`;
  - `pub fn identity_of(user: Option<&str>) -> Result<User, String>`, exported as `identity(user)`. Its inner form `identity_inner` answers 200 `{"user":"<normalized>"}` or 401 in the layer's error shape. Add one cell beside the 401 cell below: `identity_inner(None)` is 401 with the `worker` sentence, and `identity_inner(Some("Red@Wire.Test"))` is 200 `{"user":"red@wire.test"}`;
  - `const HOST: Host = Host { identity_name: "cf-access", signal_kind: "ws", serves_games: true, multiplayer_offered: true }`.

**The crate's shape is `crates/yokohama-wasm`'s** (read `crates/yokohama-wasm/src/lib.rs` and `Cargo.toml` first; its doc comments carry reasons this crate inherits):
- `crate-type = ["cdylib"]` and `publish = false`;
- `yokohama-web = { path = "../yokohama-web", default-features = false }`, `yokohama-core`, and `yokohama-cli` (Task 4 reads `repl::SubPrompt`);
- `wasm-bindgen = "=0.2.128"`, `js-sys = "0.3"`, `serde`, `serde_json`, `http = "1"` and `sha2` (Task 6) — each at the version `Cargo.lock` already holds (check with `cargo tree -p yokohama-web -i <name>`);
- `[features] debug = []`.

**Every export is a thin `#[wasm_bindgen]` wrapper over a native `_inner` function**, so everything that can be wrong is reachable under `cargo test`: wasm-bindgen's non-wasm stubs cannot make a `JsValue`.

- [ ] **Step 1: The crate compiles for both targets with one trivial export**, and the workspace sees it:

```bash
cargo build -p yokohama-worker
cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker
```

- [ ] **Step 2: Write the failing cells** in `src/lib.rs`'s test module. They share the global `GAMES`, so they serialize on a `SERIAL` mutex taken through poison, as `yokohama-wasm`'s do.

```rust
fn config(archive: &str) -> String {
    format!(r#"{{"archive":"{archive}","caller":null,"lobby_chat":null,"chat_keep":null,"chat_text":null}}"#)
}

/// The objects read the caller from `X-Yokohama-User` (spec §2.3), so a
/// missing caller is the layer's 401 in the `worker` mode's OWN sentence —
/// `identity::resolve`'s, never a restatement — and the literal `anonymous`
/// is no caller at all.
#[test]
fn a_missing_caller_is_the_worker_modes_401_and_anonymous_is_missing() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    for user in [None, Some("anonymous"), Some("  ")] {
        let r = dispatch_inner(user, "GET", "/api/games", "", None, b"");
        assert_eq!(r.status, 401, "{user:?}");
        assert!(r.body.contains("X-Yokohama-User from the edge Worker"), "{}", r.body);
    }
    let r = dispatch_inner(None, "GET", "/api/me", "", None, b"");
    assert_eq!((r.status, r.body.as_str()), (200, r#"{"mode":"cf-access","user":null}"#));
    let r = dispatch_inner(Some("Red@Wire.Test"), "GET", "/api/me", "", None, b"");
    assert_eq!(r.body, r#"{"mode":"cf-access","user":"red@wire.test"}"#, "normalized once, as the layer does");
}

/// This host's four deployment facts on `/api/health` (spec §4's second class).
#[test]
fn health_reports_cf_access_ws_multiplayer_and_the_archive_switch() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    let body = dispatch_inner(Some("red@wire.test"), "GET", "/api/health", "", None, b"").body;
    for fact in [r#""identity":"cf-access""#, r#""signal":"ws""#, r#""archive":"off""#, r#""multiplayer":true"#] {
        assert!(body.contains(fact), "{fact} in {body}");
    }
}

/// `init` refuses a bad value NAMING the variable — the Worker's form of the
/// NAS refusing to start (spec §2.6) — and the chat variables are the NAS's,
/// through the layer's own parser.
#[test]
fn init_refuses_a_bad_value_by_the_variables_name() {
    let _s = serial();
    let err = init_inner(&config("s3")).unwrap_err();
    assert!(err.contains("YOKOHAMA_WEB_ARCHIVE"), "{err}");
    let err = init_inner(r#"{"archive":"off","caller":null,"lobby_chat":"maybe","chat_keep":null,"chat_text":null}"#).unwrap_err();
    assert!(err.contains("YOKOHAMA_WEB_LOBBY_CHAT"), "{err}");
    init_inner(r#"{"archive":"off","caller":null,"lobby_chat":"off","chat_keep":"all","chat_text":"280"}"#).unwrap();
    let body = dispatch_inner(Some("red@wire.test"), "GET", "/api/health", "", None, b"").body;
    assert!(body.contains(r#""lobby_chat":false"#) && body.contains(r#""chat_text":280"#), "{body}");
}

/// The side channel IN and OUT, over all four kinds, and `load` is a CREATE.
#[test]
fn load_and_read_cover_the_four_kinds_and_load_refuses_a_present_document() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    for (kind, id) in [("record", "t1"), ("table", "t1"), ("chat", "t1"), ("house", "house")] {
        load_inner(kind, id, "{}").unwrap();
        assert_eq!(read_inner(kind, id).unwrap().as_deref(), Some("{}"), "{kind}");
        assert!(load_inner(kind, id, "{}").is_err(), "{kind}: a present document is refused");
    }
    assert!(load_inner("picker", "t1", "[]").is_err(), "the picker is the HOST's row, never the layer's");
}

/// `set_clock` hands back what it replaced, so a caller that pins a reading
/// can restore a test configuration's frozen clock (spec §2.6).
#[test]
fn set_clock_returns_the_prior_setting() {
    let _s = serial();
    assert_eq!(set_clock(None), None);
    assert_eq!(set_clock(Some(1_789_000_000.0)), None);
    assert_eq!(set_clock(Some(1_789_000_500.0)), Some(1_789_000_000.0));
    assert_eq!(now(), 1_789_000_500);
    assert_eq!(set_clock(None), Some(1_789_000_500.0));
}
```

The signal cell comes next, and the planner's warning goes with it.
- **Drive the eight bump sites through `dispatch`**, never by calling layer functions directly:
  - create → `{"bump": id}` (`tables.rs:776`);
  - join filling the last seat → `{"bump": id}` (`:1409`);
  - the first keep → `{"bump_table": id}` (`:1511`);
  - the last keep, which finalizes → `{"bump_table": id}` then `{"bump": id}` (`:1507-1511`, then `:1262`'s `flip_to_playing`);
  - an applied `input` → `{"bump": id}` (`games.rs:668`, `persist`);
  - a table chat post → `{"bump_table": id}` (`:958`);
  - a house chat post → `{"bump_house_chat": null}` (`:972`);
  - the creator's `DELETE` → `{"end": id}` (`:1742`).
- **Use the walk's own bodies** (`web/src/wire-script/walk.json`):
  - create `{"players": 2, "seats": [{"user": "red@t.test"}, {"open": true}], "seed": 20260831, "start_seat": 0, "colors": ["Red", "Blue"]}`;
  - join `{"seat": 1}`;
  - keep `{"card": 1, "seat": 0}`;
  - chat `{"text": "…"}`;
  - house chat `{"name": "Red", "text": "…"}`.
  - For the input, read the frame's first option number off `GET /api/games/{id}` as the mover, and post `{"line": "<n>"}`.
- **Assert each reply's `bumps` EXACTLY, in order.** Then the cold pump:
  - The pump arrangement is the COMMITTED fixture `worker/test/fixtures/owed-pump.{table.json,json}`: a vs-bot table whose record's human moves were applied through `Session` with no pump, until the bot is to move. The layer's finalize itself pumps (`tables.rs:1211`), so no API sequence leaves a bot owed. Generate it once, as the first cell below says.
  - `init` afresh, `load` both documents, and send a request that builds the handle and THEN refuses: `GET …/record` answers 409 while the game runs, after `Games::handle` (`api.rs:1839-1850`).
  - Assert the reply carries `{"bump": id}`: the pump happened, and it is recorded because it happened (`lib.rs:520-538`).

```rust
fn call(user: &str, method: &str, path: &str, body: Option<&str>) -> ReplyData {
    let bytes = body.map(|b| b.as_bytes().to_vec()).unwrap_or_default();
    dispatch_inner(Some(user), method, path, "", body.map(|_| "application/json"), &bytes)
}
fn tagged(r: &ReplyData) -> String {
    serde_json::to_string(&r.bumps).unwrap()
}

/// Every layer bump site reaches the reply, tagged and in call order — driven
/// through `dispatch` exactly as the Worker drives it (spec §5.1).
#[test]
fn every_bump_site_arrives_in_the_reply_tagged_and_in_order() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    let create = r#"{"id":"b","players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,
                     "seats":[{"user":"red@t.test"},{"open":true}]}"#;
    let r = call("red@t.test", "POST", "/api/games", Some(create));
    assert_eq!((r.status, tagged(&r).as_str()), (201, r#"[{"bump":"b"}]"#), "create");
    let r = call("blue@t.test", "POST", "/api/games/b/join", Some(r#"{"seat":1}"#));
    assert_eq!((r.status, tagged(&r).as_str()), (200, r#"[{"bump":"b"}]"#), "the join that fills the last seat");
    let r = call("red@t.test", "POST", "/api/games/b/keep", Some(r#"{"seat":0,"card":1}"#));
    assert_eq!((r.status, tagged(&r).as_str()), (200, r#"[{"bump_table":"b"}]"#), "a keep that does not finalize");
    let r = call("blue@t.test", "POST", "/api/games/b/keep", Some(r#"{"seat":1,"card":1}"#));
    // TWO bumps, in this order (tables.rs:1507-1511): the keep's own write bumps the
    // table, and the finalize it then runs is a second write — `flip_to_playing`
    // bumps the house. A recorder that merged them would change what the table frames on.
    assert_eq!(
        (r.status, tagged(&r).as_str()),
        (200, r#"[{"bump_table":"b"},{"bump":"b"}]"#),
        "the last keep finalizes",
    );
    let r = call("red@t.test", "POST", "/api/games/b/input", Some(r#"{"line":"1"}"#));
    let state: serde_json::Value = serde_json::from_str(&r.body).unwrap();
    assert!(state["events"].as_u64().unwrap() > 0, "premise: line 1 APPLIED a move at the first prompt");
    assert_eq!((r.status, tagged(&r).as_str()), (200, r#"[{"bump":"b"}]"#), "an applied move persists and bumps");
    let r = call("red@t.test", "POST", "/api/games/b/chat", Some(r#"{"text":"hello"}"#));
    assert_eq!((r.status, tagged(&r).as_str()), (201, r#"[{"bump_table":"b"}]"#), "a table chat post");
    let r = call("red@t.test", "POST", "/api/chat", Some(r#"{"name":"Red","text":"anyone?"}"#));
    assert_eq!((r.status, tagged(&r).as_str()), (201, r#"[{"bump_house_chat":null}]"#), "a house chat post");
    let r = call("red@t.test", "DELETE", "/api/games/b", None);
    assert_eq!((r.status, tagged(&r).as_str()), (204, r#"[{"end":"b"}]"#), "the creator's delete");
}

/// **The owed-pump FIXTURE** (plan review M3). Task 9's pool cell needs the SAME
/// bytes — a record whose last events leave the bot to move — and only Rust can
/// make them. They cannot be re-derived byte-identically, because the finalize
/// draws the bot seat's agent seed from OS entropy. So they are GENERATED once,
/// under `UPDATE_WORKER_FIXTURES=1`, COMMITTED, and loaded by both cells.
/// Without the variable this cell asserts the committed pair's premise instead.
#[test]
fn the_owed_pump_fixture_owes_the_bot_a_move() {
    let _s = serial();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/test/fixtures");
    if std::env::var_os("UPDATE_WORKER_FIXTURES").is_some() {
        init_inner(&config("off")).unwrap();
        let prior = set_clock(Some(1_789_000_000.0));
        let create = r#"{"id":"owed","players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,
                         "seats":[{"user":"red@t.test"},{"bot":true}]}"#;
        assert_eq!(call("red@t.test", "POST", "/api/games", Some(create)).status, 201);
        assert_eq!(call("red@t.test", "POST", "/api/games/owed/keep", Some(r#"{"seat":0,"card":1}"#)).status, 200);
        let table = read_inner("table", "owed").unwrap().unwrap();
        let record: yokohama_core::GameRecord =
            serde_json::from_str(&read_inner("record", "owed").unwrap().unwrap()).unwrap();
        let mut session =
            yokohama_cli::session::Session::from_record(record, yokohama_core::Rules::standard()).unwrap();
        let bot = yokohama_core::PlayerId::new(1);
        while session.state().current_player != bot {
            let mv = session.legal()[0].clone();
            session.apply(mv, vec![]).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("owed-pump.table.json"), &table).unwrap();
        std::fs::write(dir.join("owed-pump.json"), session.to_record_json().unwrap()).unwrap();
        set_clock(prior);
    }
    let record: yokohama_core::GameRecord =
        serde_json::from_str(&std::fs::read_to_string(dir.join("owed-pump.json")).expect("the committed fixture")).unwrap();
    let session = yokohama_cli::session::Session::from_record(record, yokohama_core::Rules::standard()).unwrap();
    assert_eq!(session.state().current_player, yokohama_core::PlayerId::new(1), "premise: the bot is owed a move");
}

/// **A GET can write** (`lib.rs:520-538`): a cold handle rebuilt with a bot to
/// move pumps that move and persists it — and the bump is in the reply even
/// though the request it rode in on is then REFUSED, because it happened. The
/// shell (Task 9) relies on this: it compares documents after EVERY dispatch.
/// The record route builds the handle before it refuses (`api.rs:1839-1850`,
/// measured by the plan's review).
#[test]
fn a_cold_handles_owed_bot_move_is_recorded_before_a_later_refusal() {
    let _s = serial();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/test/fixtures");
    let table = std::fs::read_to_string(dir.join("owed-pump.table.json")).expect("the committed fixture");
    let owed = std::fs::read_to_string(dir.join("owed-pump.json")).expect("the committed fixture");
    init_inner(&config("off")).unwrap();
    load_inner("table", "owed", &table).unwrap();
    load_inner("record", "owed", &owed).unwrap();
    let r = call("red@t.test", "GET", "/api/games/owed/record", None);
    assert_eq!(r.status, 409, "premise: the record route refuses a running game — AFTER the handle is built");
    assert_eq!(tagged(&r), r#"[{"bump":"owed"}]"#, "the owed pump happened, so its bump is recorded");
    assert_ne!(read_inner("record", "owed").unwrap().unwrap(), owed, "and the pump was persisted");
}
```

Generate the fixture ONCE, before the cells are expected to pass. Commit both files, and do not regenerate them unless the engine's replay of them breaks:

```bash
UPDATE_WORKER_FIXTURES=1 cargo test -p yokohama-worker the_owed_pump_fixture
```

- [ ] **Step 3: Add the crate's platform scan**, copied from `yokohama-wasm`'s `this_host_makes_no_platform_call_but_the_clock` with the same eight patterns and both anti-vacuity partners. It reads EVERY `src/*.rs` of THIS crate.

- [ ] **Step 4: Watch everything fail.** `cargo test -p yokohama-worker` — it fails to compile.

- [ ] **Step 5: Implement.**
  - **`signal.rs`:**
    - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)] #[serde(rename_all = "snake_case")] pub enum Bump { Bump(String), BumpTable(String), BumpHouseChat(()), End(String) }`. The `()` newtype is what serializes `{"bump_house_chat":null}` and not a bare string.
    - `pub struct RecordingSignal { calls: Mutex<Vec<Bump>> }` implements `Signal` by pushing, and has `pub fn drain(&self) -> Vec<Bump>`.
    - It knows no socket: sending is the shell's, after the write.
  - **`config.rs`:** `#[derive(Deserialize)] #[serde(deny_unknown_fields)] struct InitConfig { archive: String, caller: Option<String>, lobby_chat: Option<String>, chat_keep: Option<String>, chat_text: Option<String> }`.
    - `archive` is `"off"`, or `"r2"`. At THIS task `"r2"` is refused with the sentence "YOKOHAMA_WEB_ARCHIVE=r2 needs the receipt archive, which this build does not have yet"; Task 6 replaces that arm.
    - Any other value is `"YOKOHAMA_WEB_ARCHIVE must be r2 or off — got {v:?}"`.
    - The chat values go through `ChatConfig::from_env_values`, whose refusals already name their variables.
  - **`lib.rs`:**
    - `init_inner(config: &str) -> Result<(), String>` builds a FRESH `Arc<MemStore>`, a fresh `Arc<RecordingSignal>` and `Games::with_parts(Arc::new(Rules::standard()), store.clone(), chat, signal.clone(), Arc::new(now), Arc::new(NoArchive))`, and stores all three statics. It is a rebuild, never a once.
    - `identity_of` builds an `http::HeaderMap` holding `x-yokohama-user` when a value is given and calls `identity::resolve(IdentityMode::Worker, &headers)`. A value that is not a valid header is `User::parse`'s sentence.
    - `dispatch_inner` resolves the identity as an `api::Identity`: `Ok(&user)`, or `Err(sentence)` carried into `dispatch` exactly as `yokohama-wasm` does. It dispatches, then drains the signal into `bumps`.
    - `load_inner` / `read_inner` go through `Games::load_document` / `read_document`. The four names `record`, `table`, `chat` and `house` map to `Doc::Record`, `Doc::Table`, `Doc::Chat` and `Doc::House`, and every other name is refused. The house chat's one id is `chat::HOUSE` (`"house"`), which `Store` enforces.
  - **The clock is `yokohama-wasm`'s shape**: `FROZEN_ON` / `FROZEN`, `Date.now()` on `wasm32` and a ticking counter natively. `set_clock` returns the prior `Option<f64>`.
  - **`Reply`** is a `#[wasm_bindgen] pub struct` with getters for `status`, `body`, `bumps` (the list serialized), `archive` and `picker` (both `Option<String>`).

- [ ] **Step 6: Watch them pass**, then the per-edit gates, including both builds of this crate.

- [ ] **Step 7: Commit.**

```bash
git add Cargo.toml Cargo.lock crates/yokohama-worker worker/test/fixtures/owed-pump.table.json worker/test/fixtures/owed-pump.json
git commit -m "feat(worker): the per-object module's core — init, dispatch with recorded bumps, the side channel and the clock"
```

---

### Task 3: The two routes this host answers itself — `create` and `events` (spec §2.2, §2.3; Review Focus 1)

**Files:**
- Create: `crates/yokohama-worker/src/create.rs`, `src/events.rs`
- Modify: `crates/yokohama-worker/src/lib.rs` (the two exports and their cells)

**Interfaces:**
- Consumes (from Task 2): `dispatch_inner`, `identity_of`, `ReplyData`, and `GAMES`. From the layer:
  - `api::parse_json::<tables::NewGame>(content_type, body)` (`api.rs:97`) and `tables::NewGame` (`tables.rs:140`, its `id: Option<String>` at `:162`);
  - `api::parse_query::<api::EventsQuery>` (`:126`, `:1575`), `api::events_at` (`:1641`) and `Events` (`:1545`, `Serialize`);
  - `ApiError`'s public fields (`lib.rs:117-129`).
- Produces:
  - `create_inner(user: Option<&str>, body: &[u8], id: &str, content_type: Option<&str>) -> ReplyData`, and `#[wasm_bindgen] create(...) -> Reply`;
  - `events_inner(id: &str, user: Option<&str>, query: &str) -> ReplyData`, and `#[wasm_bindgen] events(...) -> Reply`. Its `bumps` is always empty.
  - `pub(crate) fn error_body(e: &ApiError) -> String` — `error_response`'s shape RESTATED: `{"error": …}`, plus `"status"` when `table_status` is set.

**The create rule (spec §2.2), in order:**
1. **Parse the body as the layer's `NewGame` through the layer's `parse_json`.** A body the layer would refuse is dispatched UNCHANGED, and the layer's own 400 answers it byte for byte. That covers a duplicate key, an unknown field, a missing field, a non-string `id`, a missing content type, an unparseable body, and a non-object the layer refuses (e.g. `[1,2]`).
1b. **A body that PARSES but is not a JSON object** — serde's derived `Deserialize` accepts a struct as a positional ARRAY, and the plan's review measured `[2,["Red","Blue"],0,null,[…]]` answering 201 on the layer directly — is refused **400** `"a game must be created from a JSON object"` (host-written; spec §4 at §9.4). The Worker's router reads `id` only from an object, so a positional id could name a table no object holds. The layer's own acceptance of arrays is a pre-existing quirk of the NAS too, and is LEAVE-listed for the cleanup cycle.
2. **`id: Some(s)`**: `s` is the name the Worker routed by. Dispatch unchanged.
   - `s != id` cannot happen through the Worker once 1b holds: an object body is read the same way by both parsers, and one they read differently fails the layer's parse. It is still refused **500** `"the create's id does not name the table it was routed to"`, so that a routing defect is loud instead of a 201 for an object that holds nothing.
   - It is host-written, and spec §4 lists it (added at §9.3).
3. **`id: None` and the body carries an `id` key** (`"id": null`): refuse **400** `"a game id, when given, must be a string"` (host-written, spec §4). Asking whether the key is present is a second parse into `serde_json::Map`, and it only decides.
4. **`id: None` and no key**: insert `,"id":<serde_json::to_string(id)>` TEXTUALLY immediately before the object's FINAL `}`, then dispatch. Find that brace as the last byte that is not JSON whitespace (space, tab, `\n`, `\r`). A body that parsed always has a member before it, since `seats` is required.

**The events rule (spec §2.2), in axum's handler's order (`lib.rs:579-603`):**
1. the caller — `identity_of(user)`; a refusal is 401 with `resolve`'s sentence;
2. the query — `parse_query::<EventsQuery>`; an unknown key is the layer's 400;
3. the CAP — `wait > 25` is 400 `"a request can be held for at most 25 seconds, not {wait}"`, restated from `lib.rs:586-590` word for word;
4. `events_at` — its `Ok` is `serde_json::to_string(&events)` at 200, and its `Err` goes through `error_body`.

`wait` is validated and never held (spec §2.3).

- [ ] **Step 1: The failing create cells.**

```rust
const CREATE: &str = r#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"open":true}]}"#;

fn created_id(r: &ReplyData) -> String {
    serde_json::from_str::<serde_json::Value>(&r.body).unwrap()["id"].as_str().unwrap().to_string()
}

/// The Worker names the object BEFORE the layer runs, so a body with no id
/// gets the object's name inserted, and every 201 names the object that holds
/// the table (spec §2.2).
#[test]
fn an_absent_id_is_inserted_and_every_201_names_the_object() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    let r = create_inner(Some("red@t.test"), CREATE.as_bytes(), "sim-00000000000000aa", Some("application/json"));
    assert_eq!(r.status, 201, "{}", r.body);
    assert_eq!(created_id(&r), "sim-00000000000000aa");
    assert!(read_inner("table", "sim-00000000000000aa").unwrap().is_some(), "stored under the object's name");
}

/// **Review Focus 1**: what `curl --data @file` and every editor send — the
/// closing brace is NOT the last byte.
#[test]
fn trailing_whitespace_after_the_closing_brace_still_inserts_before_it() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    for (n, tail) in ["\n", "  \r\n", "\t \n\n"].iter().enumerate() {
        let id = format!("sim-000000000000000{n}");
        let body = format!("{CREATE}{tail}");
        let r = create_inner(Some("red@t.test"), body.as_bytes(), &id, Some("application/json"));
        assert_eq!((r.status, created_id(&r)), (201, id.clone()), "tail {tail:?}: {}", r.body);
    }
}

/// `"id": null` is the one body the layer reads as ABSENT after the Worker has
/// named the object — measured by the re-review to answer 201 for a table no
/// object held. Refused, and nothing stored.
#[test]
fn a_null_id_is_refused_and_nothing_is_stored() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    let body = CREATE.replacen('{', r#"{"id":null,"#, 1);
    let r = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000bb", Some("application/json"));
    assert_eq!(r.status, 400);
    assert_eq!(r.body, r#"{"error":"a game id, when given, must be a string"}"#);
    assert!(read_inner("table", "sim-00000000000000bb").unwrap().is_none());
}

/// A body the layer would refuse reaches it UNCHANGED, so its 400 is the NAS's
/// byte for byte — including the missing field whose column the first text's
/// insertion moved from 68 to 96 (spec §0.4).
#[test]
fn a_refused_body_is_forwarded_unchanged_and_answered_in_the_layers_words() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    let cases: [(&str, &str); 6] = [
        ("a duplicate key", r#"{"players":2,"players":2,"seats":[{"user":"red@t.test"}]}"#),
        ("an unknown field", r#"{"players":2,"seats":[{"user":"red@t.test"}],"bogus":1}"#),
        ("a missing field", r#"{"seats":[{"user":"red@t.test"}],"colors":["Red","Blue"],"start_seat":0,"seed":1}"#),
        ("a non-string id", r#"{"id":5,"players":2,"seats":[{"user":"red@t.test"}]}"#),
        ("a non-object", "[1,2]"),
        ("an unparseable body", "{\"players\":"),
    ];
    for (what, body) in cases {
        let via_create = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000cc", Some("application/json"));
        let direct = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", Some("application/json"), body.as_bytes());
        assert_eq!(via_create.status, 400, "{what}: {}", via_create.body);
        assert_eq!((via_create.status, via_create.body), (direct.status, direct.body), "{what}: forwarded unchanged");
    }
    let missing = create_inner(Some("red@t.test"), cases[2].1.as_bytes(), "sim-00000000000000cc", Some("application/json"));
    let column = cases[2].1.len();
    assert!(missing.body.contains(&format!("column {column}")), "the ORIGINAL body's column: {}", missing.body);
    let no_type = create_inner(Some("red@t.test"), CREATE.as_bytes(), "sim-00000000000000cc", None);
    let direct = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", None, CREATE.as_bytes());
    assert_eq!((no_type.status, no_type.body), (direct.status, direct.body), "a missing content type is the layer's own answer");
}

/// A `u64` seed survives exactly: nothing is re-serialized (spec §0.4).
#[test]
fn a_u64_seed_is_stored_exactly() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    let body = CREATE.replace("\"seed\":20260831", "\"seed\":\"18446744073709551615\"");
    let r = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000dd", Some("application/json"));
    assert_eq!(r.status, 201, "{}", r.body);
    assert!(read_inner("table", "sim-00000000000000dd").unwrap().unwrap().contains("18446744073709551615"));
}

/// A positional ARRAY body the layer itself would accept (plan review m3) is
/// refused here: this host names its object from an object's `id` only.
#[test]
fn an_array_body_the_layer_would_accept_is_refused() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    // Positional, in `NewGame`'s FIELD ORDER (tables.rs:140-180) — adjust the array to
    // that order until the premise below holds; the review's measured body was
    // `[2,["Red","Blue"],0,null,[…]]`.
    let body = r#"[2,["Red","Blue"],0,null,[{"user":"red@t.test"},{"open":true}]]"#;
    let direct = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", Some("application/json"), body.as_bytes());
    assert_eq!(direct.status, 201, "premise: the LAYER accepts this array positionally — {}", direct.body);
    init_inner(&config("off")).unwrap();
    let r = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000ff", Some("application/json"));
    assert_eq!(r.status, 400, "{}", r.body);
    assert_eq!(r.body, r#"{"error":"a game must be created from a JSON object"}"#);
    assert!(read_inner("table", "sim-00000000000000ff").unwrap().is_none());
}

/// Unreachable through the Worker, and loud if it ever is not (spec §2.2, §4).
#[test]
fn a_string_id_that_is_not_the_objects_name_is_a_routing_defect() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    let body = CREATE.replacen('{', r#"{"id":"elsewhere","#, 1);
    let r = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000ee", Some("application/json"));
    assert_eq!(r.status, 500);
    assert!(read_inner("table", "elsewhere").unwrap().is_none() && read_inner("table", "sim-00000000000000ee").unwrap().is_none());
}
```

- [ ] **Step 2: The failing events cells.**

```rust
/// The handler's ORDER, pinned by probes that each carry TWO defects — a
/// single-defect probe pins nothing about order (the lesson three M6.2 tasks
/// relearned).
#[test]
fn events_refuses_in_axums_handler_order() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    assert_eq!(create_inner(Some("red@t.test"), CREATE.as_bytes(), "t", Some("application/json")).status, 201);
    let e = events_inner("t", None, "bogus=1");
    assert_eq!(e.status, 401, "no caller AND an unknown key: the caller first — {}", e.body);
    let e = events_inner("t", Some("red@t.test"), "bogus=1&wait=26");
    assert!(e.status == 400 && e.body.contains("bogus"), "an unknown key AND wait=26: the parse first — {}", e.body);
    let e = events_inner("nope", Some("red@t.test"), "wait=26");
    assert_eq!(e.status, 400, "wait=26 AND an unknown id: the cap before the read — {}", e.body);
    assert_eq!(e.body, r#"{"error":"a request can be held for at most 25 seconds, not 26"}"#);
    let e = events_inner("t", Some("red@t.test"), "wait=25");
    assert_eq!(e.status, 200, "25 is allowed, and answered at once: {}", e.body);
    assert_eq!(e.body, r#"{"events":0,"to_move":null,"status":"open","your_turn":false,"chat":0}"#);
}

/// The restated error body is `dispatch`'s own, for the same refusal — both
/// arms of it, the `status` arm through a constructed `ApiError` compared with
/// the layer's dealing 409.
#[test]
fn a_refusal_body_is_the_one_dispatch_writes() {
    let _s = serial();
    init_inner(&config("off")).unwrap();
    let e = events_inner("nope", Some("red@t.test"), "");
    let d = dispatch_inner(Some("red@t.test"), "GET", "/api/games/nope/table", "", None, b"");
    assert_eq!((e.status, e.body.as_str()), (d.status, d.body.as_str()), "the unknown id's 404");
    let body = r#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"user":"blue@t.test"}]}"#;
    assert_eq!(create_inner(Some("red@t.test"), body.as_bytes(), "dealt", Some("application/json")).status, 201);
    let d = dispatch_inner(Some("red@t.test"), "GET", "/api/games/dealt", "", None, b"");
    assert_eq!(d.status, 409, "premise: a dealing table's session route is the 409 that carries `status`");
    let message = serde_json::from_str::<serde_json::Value>(&d.body).unwrap()["error"].as_str().unwrap().to_string();
    let restated = error_body(&ApiError { status: http::StatusCode::CONFLICT, message, table_status: Some("dealing") });
    assert_eq!(restated, d.body);
}
```

- [ ] **Step 3: Watch them fail.** `cargo test -p yokohama-worker create` and `cargo test -p yokohama-worker events` — they fail to compile.

- [ ] **Step 4: Implement** `create.rs` and `events.rs` as the two rules above say, plus the exports in `lib.rs`. Nothing re-serializes a body: the only writes into a body are the inserted bytes.

- [ ] **Step 5: Watch them pass, then run the per-edit gates.**

- [ ] **Step 6: Commit.**

```bash
git add crates/yokohama-worker
git commit -m "feat(worker): create names its object textually and refuses a null id; events restates axum's handler"
```

---

### Task 4: The picker across hibernation (R-6.5c.7; spec §2.4)

**Files:**
- Create: `crates/yokohama-worker/src/picker.rs`
- Modify: `crates/yokohama-worker/src/lib.rs` (`dispatch_inner` fills `picker`; the `restore_picker` export)

**Interfaces:**
- Consumes:
  - `Games::handle(id)` (public) and `yokohama_cli::repl::SubPrompt` (`repl.rs:189`; "open" is `!matches!(repl.sub, SubPrompt::None)`, with `SubPrompt::PlaceA` among the open ones);
  - `api::input(games, &User, id, &api::Input { line, expect })` (`api.rs:1031`, `Input` at `:989`);
  - `Store::read(Doc::Record, id)`.
- Produces:
  - `static PICKERS: Mutex<HashMap<String, PickerList>>`;
  - `pub struct PickerList { record: String /* sha256 hex of the record bytes the list was built at */, lines: Vec<PickerLine> }`;
  - `pub struct PickerLine { user: String, line: String, expect: Option<String> }`.
  - `ReplyData.picker`: `Some(<the list's JSON>)` when it grew, `Some("[]")` when it was cleared, `None` when unchanged.
  - `restore_picker_inner(id: &str, row: &str) -> ReplyData` and the `restore_picker` export (§"Names shared across tasks").
  - **The stored row's JSON** is `{"record": "<hex>", "lines": [{"user": …, "line": …, "expect": …}]}`.

**The rule** (spec §2.4, verbatim where it matters):
- **When a line is appended.** After a `POST /api/games/{id}/input` that answered 200 and **left the RECORD's bytes unchanged**, read `games.handle(id)` and `repl.sub`. A picker still open appends `(user, line, expect)`; a closed one clears the list.
- **When the list is cleared.** After ANY dispatch that touched `/api/games/{id}…`, **a changed record clears the list**: a move, an undo, a finalize, or a cold bot pump on a GET. A chat post, an unlock, a keep that does not finalize and a watcher arriving all leave it.
- **The 65th line CLEARS the list**, so nothing partial is ever re-fed.
- **`restore_picker(id, row)` first builds the handle.** If the stored list's `record` digest is not the CURRENT record's, it clears WITHOUT re-feeding anything.
  - This is the plan's guard: a list is only ever re-fed at the exact record it was built at. Every line is deterministic there, so no re-fed line can apply a move.
  - Otherwise it re-feeds each line through `api::input` as its user, discarding the replies. A line that fails, changes the record, or leaves `sub` closed clears the list.

- [ ] **Step 1: The failing cells.**
  - **The arrangement is MEASURED** (the plan's review, seed 20260831, through `dispatch`). On a 2P table from Task 2's create body, joined and both kept, as red:
    - `1` APPLIES the opening move, AdditionalActionFirst → Placement, so `events` becomes 1;
    - `1` opens the **PlaceA** picker (`prompt.kind == "place_a"`, `selected: []`) — at that prompt `1` is Option A, `2` Option B and `3` "Place no assistants";
    - `A09` TOGGLES Customs (A09) into `selected` (`[9]`), and a second `A09` toggles it out;
    - `confirm` places: "Red: 1 assistant placed in Customs (A09)", and `events` becomes 2.
  - **Do not follow the walk's `2` here.** It opens PlaceB (`walk.json` rows 49–50 name `place_b` and `place_b_selected`). Assert every premise above as the cell runs: the kind, the `selected` list and the event counts.
  - **Write a helper `at_place_a(id)`** in the test module that creates, joins, keeps both, posts `1`, posts `1`, and asserts `place_a` with `selected: []`.
  - **The cells:**

```rust
#[test]
fn an_open_picker_grows_the_list_and_a_move_clears_it() { /* at_place_a — the reply to the second `1` carries
    picker ["1"]; `A09` → picker ["1","A09"] and selected [9]; `confirm` applies (events 1 → 2, the record changed)
    and its reply carries picker "[]" */ }

#[test]
fn a_chat_post_between_two_clicks_leaves_the_list() { /* at_place_a; POST …/chat (its reply's picker is None);
    `A09` → picker ["1","A09"] — battery 21's native catcher */ }

#[test]
fn the_sixty_fifth_line_clears_the_list() { /* at_place_a (1 line), then `A09` 63 times — the picker stays open,
    each a 200, toggling — for 64 lines; the next `A09` is the 65th and its reply carries "[]" */ }

/// Umbrella §4's H-4 acceptance, natively: a PlaceA picker mid-selection
/// survives a new instance, and the next line commits the move the player
/// intended.
#[test]
fn restore_picker_rebuilds_a_place_a_picker_so_confirm_commits_the_intended_move() { /* at_place_a; `A09`
    (selected [9]); capture the picker row, the table and the record; init() afresh (a new instance); load both;
    restore_picker(row) keeps the list (its picker None); `confirm` → events 2, the placement names A09.
    THE CONTROL: the same arrangement WITHOUT restore_picker — `confirm` answers 200 with "Unrecognized input" and
    the record UNCHANGED (events stays 1); and `confirm` sent WITH `"expect":"place_a"`, as the client sends it,
    answers 409 "this game is asking \"options\", not \"place_a\" …" (both measured by the plan's review) */ }

#[test]
fn a_list_built_at_another_record_is_cleared_without_being_refed() { /* a row whose `record` digest is not
    the current record's → Reply.picker "[]", the record untouched, no bump */ }

#[test]
fn restore_picker_at_a_table_with_no_session_answers_200_and_does_nothing() { /* an open table and a dealing
    one: handle() is Err(Open | Dealing); restore_picker(id, "null") answers 200 with no picker, no bumps and no
    archive (plan review m13) */ }
```

  These cells follow the same structure as Tasks 2 and 3: `call(...)`, `tagged(...)`, and assertions on `ReplyData.picker`. Every step of their arrangements is named above; write them in full.
  - **A body comment is not a placeholder here.** Each one names every request and every assertion, and nothing is left to decide.
  - **The CONTROL half of the restore cell is load-bearing.** Without it, the cell would pass even if `restore_picker` did nothing.
  - `restore_picker_inner` drains `archive` exactly as `dispatch_inner` does (plan review M4): a rebuild whose owed pump reaches game over carries its plan out.

- [ ] **Step 2: Watch them fail.**

- [ ] **Step 3: Implement** `picker.rs`, and the post-dispatch step in `dispatch_inner`:
  - the path's id is `/api/games/{id}` and below, never the create;
  - read the record's bytes before and after;
  - apply the rule; set `picker`.

- [ ] **Step 4: Watch them pass, then run the per-edit gates.** Then commit:

```bash
git add crates/yokohama-worker
git commit -m "feat(worker): an open picker survives a new instance by re-fed lines, cleared only when the game changes"
```

---

### Task 5: The house copy — the scrub, and the house's store (R-6.5c.8; spec §2.5; Review Focus 4)

**Files:**
- Create: `crates/yokohama-worker/src/house.rs`
- Modify: `crates/yokohama-worker/src/lib.rs` (three exports)

**Interfaces:**
- Consumes:
  - `table::TableDoc` (`table.rs:170`, all fields public), `table::Lock` (`:104`), `tables::check_shape` (public since Task 1);
  - `STORE` (Task 2) through the `Store` trait: `write_if_version`, `delete`, `read`.
- Produces:
  - `house_copy_inner(id) -> Result<Option<String>, String>`:
    - the table's STORED document, scrubbed: `seed` → 0; a `lock` → a `Lock` whose salt and hash are zeroed hex of the stored lengths; every `keeps[i]` → `Some(1)` if `Some(_)`, else `None`; everything else as stored;
    - serialized with `serde_json::to_string`;
    - then `check_shape` on the copy — its `Err` is returned as the refusal;
    - `None` when there is no table.
  - `put_copy_inner(id, json) -> Result<(), String>` — REPLACE-or-create in the house's `MemStore`, through `STORE`.
  - `remove_copy_inner(id) -> Result<(), String>` — `Store::delete`, where absent is not an error.
  - `house_frame_inner(seq: u64, chat: u64) -> String` — a `#[derive(Serialize)] struct HouseFrame { seq, chat }` serialized. Its cell asserts `house_frame_inner(3, 7) == r#"{"seq":3,"chat":7}"#`, the exact text the SSE frame's `format!` writes at `lib.rs:699-701`.
  - The exports `house_copy` (throws the reason), `put_copy`, `remove_copy` and `house_frame`.

**Why `Some(1)`** (spec §2.5): `check_shape` accepts a keep only as `None | Some(1 | 2)` (`tables.rs:444-449`), and `list` silently drops a document it refuses. A `Some(0)` scrub was measured to take a kept table off the lobby.

- [ ] **Step 1: The failing cells.**
  - **Build the originals through `dispatch`, at every status and lock state.**
    - Open: unlocked, and locked with `"password":"sesame"`.
    - Dealing: one keep in, so a keep of card **2** is present.
    - Playing, and over.
    - An over table is the COMMITTED finished pair, `load`ed as-is and **under its own id, `local-242a5429453c593c`**: `crates/yokohama-web/tests/fixtures/browser-played-2p.json` and `.table.json`, 595 events, against the bot, with `turn.over` true. `write_table` keys by the document's `id` (`tables.rs:810-813`), so under any other key a later write would make a phantom document and conflict (plan review m12).
      - Do not try to play a game to its end through the API with a naive policy. The layer's own whole-game gate livelocks on some seeds for exactly that reason.
    - One table is named `"Café ☕ 東京"` with `"display_name": "Zoë"` (**Review Focus 4**).
  - **For every viewer kind** (a seat holder, the creator, a stranger, a stranger on the `unlocked` roster after a correct `unlock`), capture `GET /api/games` for `scope=mine`, `open` and `playing`.
  - **Then compute each table's `house_copy`.** `init` a fresh instance, `put_copy` every copy into it, and capture the same requests.

```rust
#[test]
fn the_scrubbed_copies_list_exactly_what_the_originals_list_for_every_viewer() {
    /* the arrangement above; assert every (viewer, scope) response is BYTE-equal between the two instances,
       and that the non-ASCII name and display name appear verbatim in both (Review Focus 4) */
}

/// The value pin (battery 3) and its anti-vacuity partner (plan review M6):
/// the ORIGINAL stored document carries the decimal seed, the lock's salt and
/// hash, and a keep of 2 — and no copy carries any of them. The PASSWORD and the
/// seed in HEX are absence-only needles: the stored document never holds either
/// (the lock is `Lock { salt, hash }`, and the seed is a decimal JSON number), so
/// they are searched for in the copy and never claimed present in the original.
#[test]
fn no_house_copy_carries_a_secret_that_its_original_does() { /* per table: read the ORIGINAL's decimal seed,
    salt, hash and keeps out of its stored text (a keep of 2 is arranged on the dealing table); assert each of those
    needles IS in the original's text and is NOT in the copy's; assert the password "sesame" and the seed's hex
    are absent from the copy; parse the copy and assert every keep is 1 or null */ }

/// A scrub defect is loud at the source (R-6.5c.8): a copy `check_shape`
/// rejects is refused with its reason, never listed.
#[test]
fn house_copy_refuses_a_copy_that_check_shape_rejects() { /* load a table document with "schema": 99;
    house_copy's Err contains "schema" */ }
```

  **The three cells are specified by their bodies' comments, which name every request, needle and assertion.** Write them in full.

- [ ] **Step 2: Watch them fail. Step 3: implement. Step 4: pass, gates, commit.**

```bash
git add crates/yokohama-worker
git commit -m "feat(worker): the house copy — a scrub the layer's own list reads identically, refused loudly when check_shape would drop it"
```

---

### Task 6: The receipt archive, the stamp, and `expire` (R-6.5c.9, R-6.5c.14, R-6.5c.15; spec §2.6, §2.4 step 1)

**Files:**
- Create: `crates/yokohama-worker/src/receipt.rs`
- Modify: `crates/yokohama-worker/src/config.rs` (`"r2"` becomes the `ReceiptArchive`), `src/lib.rs` (the exports, and `ReplyData.archive`)
- Create: `worker/test/fixtures/owed-game-over.json` and `owed-game-over.table.json` — generated once with `UPDATE_WORKER_FIXTURES=1 cargo test -p yokohama-worker the_owed_game_over_fixture`, committed, and drift-checked on every run

**Interfaces:**
- Consumes:
  - `archive::{Archive, PutOutcome, PutError, Why, paths}` (`archive.rs:36-135`) — `paths(rev, prefix, why, id)` with the rev `DirArchive::put` passes (read it at `archive.rs:256`);
  - `Games::archive_over_retry` (Task 1);
  - `sha2::Sha256`.
- Produces:
  - `pub struct ReceiptArchive { receipts: Mutex<HashMap<String, Outcome>>, pending: Mutex<Option<Plan>>, caller: Option<String> }`, implementing `Archive`, with `kind()` = `"r2"`;
  - `Plan` and `Outcome` as §"Names shared across tasks" states;
  - `ReplyData.archive`: the plan drained after each dispatch;
  - the exports `receipt`, `stamp_archive` and `expire`;
  - `init`'s `"r2"` arm, which builds the `ReceiptArchive` with the config's `caller`.

**`put`** (spec §2.6):
1. Compute the keys with `paths`, then apply the `test/` ROOT:
   - for a server game (`prefix == ""`), when the TABLE's `created_by` ends in `.test` (read it from the stripped table JSON; `strip_for_archive` keeps `created_by`);
   - for a browser save (`prefix` starts `browser/`), when the Archive object's `caller` — the full normalized address from `init` — ends in `.test`, **never the 64-character segment**.
2. Compute `key` = SHA-256 hex over `record_key|table_key|sha256(record)|sha256(table)`.
3. A stored receipt for `key` is REMOVED and returned: `landed` is `Ok(Landed)`, `identical` is `Ok(Identical)`, `conflict` is `Err(Conflict(record_key))` and `failed` is `Err(Failed(detail))`.
4. With no receipt, the plan is recorded and the answer is `Err(Failed("deferred to the host"))`.

**`expire(id)`** reads the stored table's `created_by` and returns `dispatch_inner(Some(created_by), "DELETE", "/api/games/{id}", "", None, b"")`. The creator is always seated (spec §0.2), so it passes both of `delete`'s checks; this is R-6.5c.4's "the layer's own `delete` as `created_by`".

**`stamp_archive(id)`** calls `archive_over_retry(id)` and returns a `ReplyData`:
- 200 with an empty body, or the error's status with `error_body`;
- the drained `bumps` and `archive`, so that a plan re-created by a changed body is visible to the shell.

- [ ] **Step 1: The failing cells** — each driven through `dispatch` and the exports:

```rust
/// No receipt: the abandon is refused as deferred, with its plan and NOTHING
/// deleted; the receipt for the EXACT plan lets the SAME request, at the SAME
/// pinned reading, delete — and its bumps say `end`.
#[test]
fn an_abandon_defers_then_completes_on_its_receipt_at_the_pinned_reading() {
    /* init "r2"; a 2P table created by red@t.test with one move applied; let prior = set_clock(Some(1_789_000_100.0));
       DELETE → not 204, the reply's archive is a plan whose record_key is "test/rev7/abandoned/<id>.json", the table
       still readable; receipt(plan.key, {"outcome":"landed"}); DELETE again at the same pinned reading → 204, bumps
       [{"end":id}]; set_clock(prior) */
}

/// A receipt whose digests differ is ABSENT (a re-dispatch at a different
/// reading stamps a different `archived_at`, so its plan's key differs).
#[test]
fn a_receipt_for_another_plan_is_not_this_plans() { /* the plan at reading R; receipt for it; re-dispatch
    at R+1 → still deferred, a NEW plan with a different key */ }

/// The `test/` root, and the truncation it must not be fooled by (battery 33).
#[test]
fn the_test_root_reads_the_full_address_never_the_truncated_segment() { /* created_by red@t.test → "test/rev7/…";
    created_by red@example.com → "rev7/…"; a browser save with init caller = a 70-character address ending ".test"
    → keys under "test/browser/<the 64-character segment, which has LOST .test>/…"; the same save with a caller
    red@example.com → "browser/…" */ }

/// Game over defers too, and the stamp completes it (spec §2.6's game-over
/// site): the move answers 200, `archived_at` stays unset, the plan rides out;
/// the receipt then `stamp_archive` at the plan's reading stamps it.
/// **The owed-game-over FIXTURE** (plan review M3): the committed finished pair,
/// cut back to the START of its trailing run of bot moves, so a cold rebuild
/// pumps the game to its END. Task 13's pool cell needs these exact bytes, and
/// only Rust can cut them (`state_at`, `stamp_terminal_digest` — the seed and the
/// bot's agent seed are both above 2^53, so no TypeScript may touch them). The
/// cut IS deterministic, so this cell is a DRIFT check: it rebuilds the partial
/// and compares it with the committed file, and writes it only under
/// `UPDATE_WORKER_FIXTURES=1`. The table is the committed pair's, unchanged,
/// under its OWN id `local-242a5429453c593c` (plan review m12).
#[test]
fn the_owed_game_over_fixture_is_the_committed_pair_cut_at_its_last_bot_run() { /* read
    crates/yokohama-web/tests/fixtures/browser-played-2p.json; find the largest n such that the state at n has the
    bot (seat 1) to move and every event from n to the end is the bot's (assert it as the premise); build the partial
    exactly as wire_fixtures.rs's `partial_of` does — truncate, `yokohama_core::state_at`,
    `GameRecord::stamp_terminal_digest`, `serde_json::to_string` — (copy those eight lines into this test module);
    under UPDATE_WORKER_FIXTURES write it to worker/test/fixtures/owed-game-over.json and copy the committed
    .table.json beside it as owed-game-over.table.json; always assert both committed files equal what was built */ }

#[test]
fn game_over_defers_its_put_and_stamp_archive_completes_it() { /* load worker/test/fixtures/owed-game-over.table.json
    and .json under the id local-242a5429453c593c; `GET …/record` rebuilds the handle, whose owed pump plays the
    bot's run to the END and persists it — the game-over site meets the deferred put: the reply's archive is a plan
    with why Over (keys under rev7/over/, no test/ root — the pair's creator is local@browser) and archived_at stays
    unset; receipt(landed); stamp_archive(id) with the clock pinned to the reading the plan was made at → archived_at
    set; a second stamp_archive puts nothing */ }

/// `expire` IS the creator's own delete: same reply, same plan, same leaf, at
/// every status (open, dealing, playing with and without moves, over).
#[test]
fn expire_answers_exactly_as_the_creators_delete_at_every_status() { /* for each status: snapshot the docs;
    run DELETE as created_by; restore the docs into a fresh instance; run expire(id); compare status, body and
    the plan's keys (same pinned reading) */ }

/// A `conflict` receipt is final where the layer says so: the abandon is
/// refused naming the archive; the game-over stamp is not written.
#[test]
fn a_conflict_receipt_refuses_the_abandon_and_leaves_game_over_unstamped() { /* as named */ }
```

  **Write the seven in full**, from the arrangements their comments name.

- [ ] **Step 2: Watch them fail. Step 3: implement.**

- [ ] **Step 4: Watch them pass, then run the per-edit gates, including both builds of this crate.**

- [ ] **Step 5: Commit.**

```bash
git add crates/yokohama-worker worker/test/fixtures/owed-game-over.json worker/test/fixtures/owed-game-over.table.json
git commit -m "feat(worker): the receipt archive, the pinned-reading stamp, and expire as the creator's own delete"
```

---

### Task 7: `worker/` — the package, the module build and its glue factory, the static assets, the test pool, and CI (spec §2.2, §2.3, §3.1, §5.6)

**Files:**
- Create:
  - `worker/package.json`, `package-lock.json`, `tsconfig.json`, and `wrangler.jsonc` — production, WITHOUT the object bindings until Task 11;
  - `worker/wrangler.test.jsonc`, `vitest.config.ts`, `vitest.scripts.config.ts`;
  - `worker/static/_headers`, `worker/static/_redirects`;
  - `worker/scripts/build-module.mjs`, `factory-glue.mjs`, `factory-glue.test.mjs`, `assemble-assets.mjs`, `assemble-assets.test.mjs`, `test-assets.mjs`, `deploy-guard.mjs`, `deploy-guard.test.mjs`;
  - `worker/src/env.ts`, `config.ts`, `config.unit.test.ts`, `module.ts`, and a placeholder-free `index.ts` (below);
  - `worker/test/factory.test.ts`.
- Modify: `.gitignore`, `.github/workflows/test.yml`.

**Interfaces:**
- Consumes: the module's exports (Tasks 2–6) and `web/scripts/wasm-opt.mjs` (the optimizer wrapper that REFUSES when binaryen is absent — one source for that rule).
- Produces:
  - `createModule(compiled: WebAssembly.Module): Exports` — generated into `worker/gen/factory.js`;
  - `class Instance` in `src/module.ts`:
    - `get(): { mod: Exports; fresh: boolean }` — `fresh` tells the caller it must load;
    - `guarded<T>(f: (m: Exports) => T): { ok: T } | { trapped: string }` — a `WebAssembly.RuntimeError` DISCARDS the instance;
    - `drop(): void`;
  - `type Bump = { bump: string } | { bump_table: string } | { bump_house_chat: null } | { end: string }`;
  - `type Replied = { status: number; body: string; bumps: Bump[]; archive?: string; picker?: string }`, and `readReply(r: Reply): Replied` — reads every getter, parses `bumps`, then calls `r.free()`. Every later task takes a module reply through it and never holds a `Reply`;
  - `shellConfig(env: Env): { ok: ShellConfig } | { error: string }` and `moduleConfig(env: Env, caller: string | null): string` in `src/config.ts`;
  - `ShellConfig`:
    ```ts
    { archive: "r2" | "off"; ttlDays: number | null; admins: string[] | null; countWakes: boolean; devDoor: boolean;
      clock: { kind: "wall" } | { kind: "frozen"; at: number } | { kind: "tick"; next: number } }
    ```
  - `precacheEntriesFrom(swText): { path: string; digest: string }[]` and `verifyTree(root, entries)` in `scripts/assemble-assets.mjs`.
  - The npm scripts:
    - `build:module` — release: `cargo build --profile release-wasm`, `wasm-bindgen --target web --out-dir gen`, the factory, `wasm-opt` through the web wrapper;
    - `build:module:debug` — `cargo build --release --features debug`; no `wasm-opt`, exactly as `web/`'s debug build;
    - `build:assets`, `build` (release module + assets);
    - `typecheck`; `pretest` (runs `test-assets.mjs`), `test` (the pool), `test:scripts` (node);
    - `deploy` (`node scripts/deploy-guard.mjs && wrangler deploy`).

**Pins** (P-3): `wrangler` **4.136.3**, `@cloudflare/vitest-plugin` **1.2.3**, `vitest` **^4.1.11** (the web's), `typescript` **^5.7.0**, and `@cloudflare/workers-types` **^5.20260921.1**. That last one is an explicit devDependency: it is only an OPTIONAL peer of wrangler 4.136.3, and installing wrangler pulls none in (plan review m9).
- `Env` is written BY HAND in `src/env.ts` from the Global Constraints' list; `wrangler types` is not used.
- The test pool's config entry is `cloudflareTest(options)`, a Vite plugin, in 1.2.3's `dist/pool/index.d.mts:87`. **Read the pinned version's README and types before writing the config; do not copy an older `defineWorkersConfig` example.**

- [ ] **Step 1: The factory transform, test-first** (`scripts/factory-glue.test.mjs`, node). The transform is a pure `factoryGlue(source: string): string`, and a synthetic glue text is enough to pin it:

```js
import { describe, expect, it } from "vitest";
import { EXPECTED, factoryGlue } from "./factory-glue.mjs";

const glue = (names, extra = "") =>
  [
    "let wasm;",
    "export class Reply {}",
    ...names.map((n) => `export function ${n}(a) { return wasm; }`),
    "function initSync(module) { if (wasm !== undefined) return wasm; wasm = {}; }",
    extra,
    "export { initSync, __wbg_init as default };",
  ].join("\n");

describe("the factory transform (spec §2.2, §0.3 row 4)", () => {
  it("wraps the whole glue so every call builds its own instance", () => {
    const out = factoryGlue(glue(EXPECTED));
    expect(out).toMatch(/^export function createModule\(compiled\) \{/m);
    expect(out.match(/^export /gm)).toHaveLength(1);
    expect(out).toContain("initSync({ module: compiled });");
    expect(out).toContain("debug_trap: typeof debug_trap === \"function\" ? debug_trap : undefined");
  });
  it("refuses a glue that lost an expected export, naming it", () => {
    expect(() => factoryGlue(glue(EXPECTED.filter((n) => n !== "events")))).toThrow(/events/);
  });
  it("refuses an export it does not know how to wrap", () => {
    expect(() => factoryGlue(glue(EXPECTED, "export const surprise = 1;"))).toThrow(/survived/);
  });
});
```

  Then write `factory-glue.mjs`:
  - `EXPECTED` = `["init","dispatch","create","events","identity","load","read","set_clock","house_copy","put_copy","remove_copy","house_frame","restore_picker","receipt","stamp_archive","expire"]`.
  - `factoryGlue` strips `export ` from `export class` and `export function` lines, and drops the `export { initSync, __wbg_init as default };` line.
  - It refuses a surviving `export`, and refuses a missing `function <name>(` for every name in `EXPECTED` and for `initSync`.
  - It emits `export function createModule(compiled) { <body> initSync({ module: compiled }); return { <EXPECTED>, debug_trap: typeof debug_trap === "function" ? debug_trap : undefined }; }`.
  - It has a CLI main that reads `gen/yokohama_worker.js` and writes `gen/factory.js`, plus a hand-written `gen/factory.d.ts` template kept in `scripts/` and copied beside it.

- [ ] **Step 2: The asset assembly, test-first** (`scripts/assemble-assets.test.mjs`, node, over a temp directory).
  - `precacheEntriesFrom` reads the built worker's `path: "…", digest: "…"` pairs — esbuild prints the substituted `__PRECACHE__` with unquoted keys (`web/src/precache.ts:399-420`).
  - It refuses zero pairs, and refuses a pair count that differs from the count of `digest: "` (its own anti-vacuity).
  - `verifyTree` refuses a missing file and a file whose SHA-256 differs, naming the path.
  - The cells:
    - two entries that verify;
    - a missing one, and a changed byte in one — each refused by name;
    - a `sw.js` with no entries;
    - a `sw.js` with a digest the regex cannot pair.

  Then `assemble-assets.mjs`:
  1. Clean `worker/assets/`.
  2. Copy `web/dist/**`, then `crates/yokohama-web/assets/art/**` under `art/`, then `worker/static/_headers` and `_redirects`.
  3. Run the two checks over the result, and print the entry count and the total bytes.

  **The committed `_headers` and `_redirects`**, exactly spec §2.3's:

```
# worker/static/_headers — the axum host's cache policy (assets.rs:106-108, art.rs:125), restated
/assets/*
  Cache-Control: public, max-age=31536000, immutable
/sw.js
  Cache-Control: no-store
/art/*
  Cache-Control: public, max-age=3600
```

```
# worker/static/_redirects — "/" is index.html WITHOUT a redirect (html_handling "none", spec §0.3 row 7)
/ /index.html 200
```

- [ ] **Step 3: The deploy guard, test-first** (`scripts/deploy-guard.test.mjs`, node). `deploy-guard.mjs` refuses unless all five hold, naming the one that failed:
  - `gen/factory.js` exists and does NOT contain `debug_trap(` — the release module, M6.5b's `grep -c debug_trap` rule;
  - `worker/assets/sw.js` exists and `verifyTree` passes over it;
  - `wrangler.jsonc` has `"workers_dev": false` and `"preview_urls": false`;
  - no `YOKOHAMA_WEB_TEST_CLOCK` appears in `wrangler.jsonc`;
  - **`wrangler.jsonc`'s `assets` block is EXACTLY spec §2.3's**: `"html_handling": "none"`, `"not_found_handling": "none"` and `"run_worker_first": ["/api/*", "/enter"]`. The same check runs over `wrangler.test.jsonc`, so the pool's copy cannot drift from what ships.
    - **Why this cell carries battery rows 13 and 14** (plan review M5): the pool reads only `wrangler.test.jsonc`. An asset-layer SPA fallback answers a missing file WITHOUT invoking the Worker, so no pool cell and no `handle` call can see a mutated production block. This check is the per-commit catcher, and the live reading at each deploy is the second.

  One cell per refusal, plus the passing case. It reads both configs as JSONC: strip `//` comments before `JSON.parse`, since configs carry no seed.

- [ ] **Step 4: `config.ts`, test-first** (`src/config.unit.test.ts`, run by `test:scripts` in node — `config.ts` imports nothing Workers-specific). The cells:
  - `YOKOHAMA_WEB_ARCHIVE`:
    - absent is `off`;
    - `r2` WITH an `ARCHIVE` binding is `r2`;
    - `r2` WITHOUT one is the error `"YOKOHAMA_WEB_ARCHIVE=r2 needs the ARCHIVE bucket binding"`;
    - `s3` is `"YOKOHAMA_WEB_ARCHIVE must be r2 or off — got \"s3\""`.
  - `YOKOHAMA_WEB_TABLE_TTL_DAYS`: absent is `null`, `30` is 30, and `0`, `-1`, `2.5` and `thirty` are each an error naming the variable.
  - `YOKOHAMA_WEB_ADMINS`: absent is `null`; `" A@b.com, c@d.com "` is `["a@b.com","c@d.com"]`; an empty entry is an error.
  - `YOKOHAMA_WEB_COUNT_WAKES` and `YOKOHAMA_DEV_DOOR`: `on` | `off` | absent (off); anything else is an error.
  - `YOKOHAMA_WEB_TEST_CLOCK`: absent is `wall`, `frozen:1789000000` and `tick:1789000000` parse, and `frozen:x` is an error.
  - `moduleConfig` carries the three chat values RAW, since the module parses them with the layer's own function, and `caller` verbatim.

- [ ] **Step 5: `module.ts` and the pool, test-first** (`test/factory.test.ts`, run by the pool over the DEBUG module):

```ts
import { describe, expect, it } from "vitest";
import compiled from "../gen/yokohama_worker_bg.wasm";
import { createModule } from "../gen/factory.js";

const CONFIG = JSON.stringify({ archive: "off", caller: null, lobby_chat: null, chat_keep: null, chat_text: null });

describe("the factory (spec §2.2, §0.3 row 4)", () => {
  // Battery 12's catcher: the stock glue shares ONE instance per isolate.
  it("gives every call its own instance: two share nothing", () => {
    const a = createModule(compiled);
    const b = createModule(compiled);
    a.init(CONFIG);
    b.init(CONFIG);
    a.load("table", "x", "{}");
    expect(a.read("table", "x")).toBe("{}");
    expect(b.read("table", "x")).toBeUndefined();
  });
  it("a trapped instance leaves another usable", () => {
    const a = createModule(compiled);
    const b = createModule(compiled);
    a.init(CONFIG);
    b.init(CONFIG);
    expect(() => a.debug_trap!()).toThrow(WebAssembly.RuntimeError);
    const r = b.dispatch("red@t.test", "GET", "/api/health", "", undefined, new Uint8Array());
    expect(r.status).toBe(200);
    r.free();
  });
});
```

  - `wrangler.test.jsonc` at this task:
    - `main: "src/index.ts"`, `compatibility_date: "2026-09-15"`;
    - the `CompiledWasm` rule over `**/*.wasm`;
    - `assets` over `./.test-assets` with spec §2.3's handling;
    - `vars`: `ACCESS_TEAM_DOMAIN: "test.cloudflareaccess.com"`, `ACCESS_AUD: "test-aud"`, `DEV_DOOR_CLIENT_ID: "test-client.access"`, `YOKOHAMA_DEV_DOOR: "on"`, `YOKOHAMA_WEB_ARCHIVE: "off"`, `YOKOHAMA_WEB_TEST_CLOCK: "frozen:1789000000"`, `YOKOHAMA_WEB_COUNT_WAKES: "on"`.
    - `COUNT_WAKES` is on in the pool: the pool facts say a live-instance override dies at the eviction a wake cell performs, and the spec's qualifier keeps a probe from writing anything even with it on (plan review M2).
  - `test-assets.mjs` (the `pretest` hook) does two things:
    - it builds `worker/.test-assets/` from a one-line `index.html` and a COPY of `crates/yokohama-web/assets/art/manifest.json` under `art/`, so `/api/art` compares against the real manifest (Task 12);
    - it writes `worker/test/fixtures.gen.ts` (gitignored), exporting each file under `worker/test/fixtures/` as its TEXT: `export const OWED_PUMP_JSON = <JSON.stringify(text)>;` and so on.
  - That module is how the pool reaches the Rust-made fixtures (Tasks 2, 6) as strings. A JSON import would parse them, and their seeds are above 2^53.
  - `src/index.ts` at this task is the honest minimum: `export default { fetch: () => new Response(null, { status: 404 }) } satisfies ExportedHandler<Env>;`. Task 11 replaces it.

- [ ] **Step 6: Watch everything fail, then build and pass it.**

```bash
cd worker && npm install      # the FIRST time only: this creates package-lock.json, which is committed; npm ci afterwards
npm run test:scripts          # the factory, the assembly, the guard, the config — red, then green
npm run build:module:debug    # the module the pool loads
npm test                      # the factory cells in workerd
npm run typecheck
cd ../web && npm run build && cd ../worker && npm run build:assets   # a real assembly: expect "153 entries" or the current count
```

- [ ] **Step 7: CI** (`.github/workflows/test.yml`, the `web-server` job, after the web's `npm run typecheck` step):
  - `actions/setup-node`'s `cache-dependency-path` becomes BOTH lock files (a multi-line value).
  - Then, each in `working-directory: worker`: `npm ci`, `npm run build:module:debug`, `npm run typecheck`, `npm run test:scripts`, `npm test`.
  - Then at the repository root: `cargo test -p yokohama-worker` and `cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker`.
  - Comment each step with its reason, in the file's own style.
  - Run the parse check: `python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/test.yml`.
  - `.gitignore` gains `worker/gen/`, `worker/assets/`, `worker/.test-assets/`, `worker/node_modules/`, `worker/.wrangler/` and `worker/test/fixtures.gen.ts`.

- [ ] **Step 8: The per-edit gates** — the new `worker/` commands join them from here on.

- [ ] **Step 9: Commit.**

```bash
git add worker .gitignore .github/workflows/test.yml
git commit -m "feat(worker): the package, a per-instance glue factory, an asset assembly verified against the release, the test pool and CI"
```

---

### Task 8: The client — the WebSocket arm and the outbox's 404 (R-6.5c.6, R-6.5c.14; spec §2.7; Review Focus 2, 3)

**Files:**
- Create: `web/src/ws.ts`, `web/src/ws.test.ts`
- Modify:
  - `web/src/types.ts:455` (`Health.signal`);
  - `web/src/local/facts.ts:34`, `:51`, `:145` (the union, and the normalization that must now accept `"ws"`);
  - `web/src/api.ts` (`signalKind` / `setSignalKind`, the union);
  - `web/src/signal.ts` (`open()` gains the arm; its SSE frame handler and connected callback become named closures both arms share);
  - `web/src/signal.test.ts`, `web/src/outbox.ts` (`offer` and the drain's send site), `web/src/outbox.test.ts`.

**Interfaces:**
- Consumes: `serverHeaders()`, `call` (`api.ts:467`) and `isSessionExpired` (`api.ts:245-267`); `Frame` (`sse.ts:37`); `BACKOFF_MS` / `nextDelay`; `patchServerFacts`, `rememberServerFacts` (`local/facts.ts:252`), `recallServerFacts` (`:186`) and `resetFactsForTest` (`:287`); `forget` (`outbox.ts:272`).
- Produces: `readSocket(path: string, signal: AbortSignal, onFrame: (f: Frame) => void, onOpen?: () => void): Promise<"ended" | "closed">` in `ws.ts`, and `PING_MS = 30_000`.

**The arm** (spec §2.7), inside `open()`'s `try` and beside `readStream`:

```ts
if (signalKind() === "ws") {
  const outcome = await readSocket(src.current.stream, mine.signal, onFrame, onConnected);
  if (stopped || outcome === "ended") return;
  // A close this arm made ITSELF — a hidden tab (R-P8), a teardown — asks nothing
  // (Review Focus 2): exactly as the stream reader returns silently on its own abort.
  if (mine.signal.aborted) return;
  // THE PROBE: one plain fetch through `call`, so an expired Access session is
  // classified exactly as the stream arm classifies it (the catch below ends the
  // source with the `/enter` remedy); an answer is a free resync, delivered.
  const v = await src.current.once();
  if (stopped) return;
  latest.current(v);
  throw new Error("the socket closed"); // then the existing ladder
}
```

**`readSocket`:**
- It opens `new WebSocket(<path resolved against location, http→ws, https→wss>)` and calls `onOpen` on `open`.
- On `open` it starts `setInterval(() => ws.send("ping"), PING_MS)`.
- A message that is not JSON, or not `{event: string, data}` — the auto-response's `"pong"` among them — is dropped.
- Any other message is handed over as `{ event, data: JSON.stringify(data) }`. The handlers `JSON.parse` `data`, as they do the stream's.
- An `end` frame sets `ended` and is handed over.
- `close` clears the interval and resolves `"ended"` if an `end` came first, whatever the close's code (spec §2.7). Otherwise it resolves `"closed"`.
- The `AbortSignal` closes the socket with `1000`.
- A socket that errors before `open` fires `close` too, so it resolves `"closed"`.

**The outbox branch** (spec §2.6):
- A 404 from `api.archive` — the layer's "this server keeps no archive" — calls a NEW `rememberArchiveOff(): void` in `local/facts.ts`, and `forget`s the row.
  - `rememberArchiveOff` rewrites the STORED server record's `archive` to `"off"`, KEEPING its `takenAt`: the other fields are no fresher than before, so their age must not change. It also patches the in-memory facts.
  - Neither of the existing functions does this. `rememberServerFacts` (`:252`) needs a whole `Health` and restamps `takenAt`, and `patchServerFacts` persists nothing (plan review m2).
- **It branches on the STATUS, never the sentence** (`reasonFor`'s own rule).
- It covers every site that sends: `offer`, and any drain path that calls `api.archive`.

- [ ] **Step 1: The failing cells.** In `ws.test.ts`, use a `FakeWebSocket` (`vi.stubGlobal("WebSocket", …)`) whose tests drive `open`, `message` and `close`, and fake timers:
  - a `{event, data}` message reaches `onFrame` with `data` stringified;
  - `"pong"` and a non-JSON message do not;
  - `ping` is sent every 30 s and stops at close;
  - an `end` frame then a `4404` close resolves `"ended"` with ONE `end` frame handed over (**battery 38**);
  - a `1006` close with no `end` resolves `"closed"`;
  - an abort closes with `1000`.

  In `signal.test.ts`, with `setSignalKind("ws")`:
  - **(battery 23)** A socket that closes `1006` before opening is followed by ONE probe. It rejects with `new ApiError(ACCESS_CHALLENGE, 401)`, and `onEnd` is called once with that sentence, as the SSE arm's cell asserts. No second socket is scheduled.
  - **(battery 37, Review Focus 2)**
    - Hiding the tab closes the socket, and the fetch spy counts ZERO probes.
    - Showing it opens a new socket, whose first frame is handed to `onValue`. The link reads `"refreshing"` and then `"live"`.
  - **(Review Focus 3)** A `1006` mid-game with no `end`:
    - the probe answers 200, and its value is handed to `onValue`;
    - the link reads `"reconnecting"`;
    - a new socket opens after `BACKOFF_MS[0]`, and nothing is lost.
  - An `end {"reason":"deleted"}` frame then a `4404` close calls `onEnd` exactly once.

  In `outbox.test.ts` (**battery 28**): a send answered `404 {"error":"this server keeps no archive"}` leaves NO pending row. `archiveAnswer()` reads `off`, and after `resetFactsForTest()` (`:287`, which keeps storage) `recallServerFacts()` still reads `archive: "off"` with the prior `takenAt`. The remembered copy is what changed, not only the in-memory one. A cell in `local/facts.test.ts` pins `rememberArchiveOff` itself: `takenAt` kept, `archive` off, and every other field untouched.

- [ ] **Step 2: Watch them fail.**

```bash
cd web && npx vitest run src/ws.test.ts src/signal.test.ts src/outbox.test.ts
```

- [ ] **Step 3: Implement.**
  - Widen the three unions and the normalization.
  - Write `ws.ts`.
  - Refactor `open()`'s inline stream handlers into named `onFrame` / `onConnected` closures. This is a pure refactor: the SSE cells must stay green without an edit.
  - Add the arm, and the outbox branch.

- [ ] **Step 4: Run the web gates, and the local e2e.** The arm is inert where health says `"sse"`, and the e2e is what proves that — the lesson M6.5b's close records about tasks that change what a browser does.

```bash
cd web && npm run typecheck && npm test && npm run test:wasm && npm run e2e
```

- [ ] **Step 5: Commit.**

```bash
git add web/src/ws.ts web/src/ws.test.ts web/src/signal.ts web/src/signal.test.ts web/src/api.ts web/src/types.ts web/src/local/facts.ts web/src/local/facts.test.ts web/src/outbox.ts web/src/outbox.test.ts
git commit -m "feat(web client): a WebSocket arm that probes an unexplained close, and an archive route's 404 read as no archive"
```

---

### Task 9: The table object — storage, the wake, the request path, the traps (spec §2.4)

**Files:**
- Create: `worker/src/storage.ts`, `worker/src/table.ts`, `worker/test/helpers.ts`, `worker/test/table.test.ts`
- Modify: `worker/src/index.ts` (`export { Table }`), `worker/wrangler.test.jsonc` (the `TABLE` binding and a `migrations` entry with `new_sqlite_classes: ["Table"]`)

**Interfaces:**
- Consumes: `Instance`, `shellConfig` and `moduleConfig` (Task 7); the exports `dispatch`, `create`, `load`, `read`, `restore_picker` and `set_clock` (Tasks 2–4).
- Produces:
  - `class Table extends DurableObject<Env>`, whose `fetch(req)` answers every `/api/games/{id}…` request and the create, with the caller in `X-Yokohama-User` and a create's name in `X-Yokohama-Table-Id` (percent-encoded).
  - Methods later tasks extend:
    - `private commit(id: string, reply: Replied, extra: BatchParts): void` — ONE `transactionSync` plus the `setAlarm` in the same turn;
    - `private wake(id: string): Replied | null` — the wake path's own reply, whose bumps, picker and `archive` plan ride into the request's commit;
    - `private reading(): number` — P-2's pinned reading;
    - the object's name is `this.ctx.id.name` (pool fact 8), in `fetch` and `alarm` alike.
  - (`Replied` and `readReply` are Task 7's.)
  - In `storage.ts`: `hasTables`, `ensureTables`, `readDocs`, `upsertDoc`, `deleteDoc`, `getMeta`, `setMeta` and `deleteMeta`.

**Storage** (spec §2.4, verbatim):
- `docs(kind TEXT PRIMARY KEY, json TEXT NOT NULL, written_at INTEGER NOT NULL)` holds `record`, `table`, `chat` and `picker`.
- `meta(k TEXT PRIMARY KEY, v TEXT NOT NULL)` holds `generation`, `revision`, `slot`, `acked`, `archive_owed`, `marker`, `held`, `load_traps`, `poison` (the kind whose load trapped) and `wakes`. It holds no `id`: the object's name is `ctx.id.name`.
- **Both are CREATED INSIDE THE FIRST WRITE BATCH, never in the constructor.** A read before any write asks `sqlite_master` and treats absence as empty.

**The request path**, in this order:
1. Read `X-Yokohama-User`, and the id: `ctx.id.name`, which equals the path's segment or a create's decoded `X-Yokohama-Table-Id`.
2. **The body, read once (`arrayBuffer`). This is the LAST `await` before the reply**, and it comes BEFORE the wake. A wake done before an await lets another request interleave between the wake and the dispatch, before the wake's reply is committed (plan review m4). From here to the reply, wake → dispatch → commit is ONE synchronous turn, as spec §2.4 says.
3. Poison check: `meta.load_traps >= 2` answers **500** `{"error":"this table's stored <meta.poison> cannot be loaded"}`.
4. The wake, if this instance has not loaded yet:
   - `Instance.get()`; `load` each present `docs` row of `record`, `table` and `chat`;
   - `restore_picker(id, <picker row> ?? "null")`, with P-2's pin;
   - with `countWakes` AND a table present, `meta.wakes += 1` in this request's batch.
   - A `WebAssembly.RuntimeError` during the wake increments `meta.load_traps`, sets `meta.poison = "record"` in its own batch, drops the instance and answers the trap's 500. A wake that succeeds resets `load_traps` if it was set.
5. **P-2's pin**: `set_clock(reading)`. Then `create` or `dispatch` under `Instance.guarded`. A trap answers **500** `{"error":"the table's engine stopped — try again"}` and COMMITS NOTHING.
6. `commit`:
   - For each of `record`, `table` and `chat`, compare `read(kind, id)` with the stored mirror and upsert (with `written_at` = the reading) or delete what differs.
   - **This compare runs after EVERY dispatch, whatever the method**: a GET can pump an owed bot move and persist it (Task 2's cold-pump cell).
   - Also: the `picker` row from `Reply.picker` (the wake's first, then the request's), and `wakes`. ANY reply's `archive` plan — the wake's or the call's — is Task 13's `archive_owed` (P-2). At this task the pool's archive is `off`, so none arrives. Every write goes in ONE `ctx.storage.transactionSync(() => …)`, and any `setAlarm` beside it in the same turn (spec §2.4, §6).
7. The reply: `new Response(body || null, { status, headers: body ? { "content-type": "application/json" } : {} })`.
8. The side channels, which Tasks 10, 12 and 13 fill.

- [ ] **Step 1: The failing pool cells** (`test/table.test.ts`). `helpers.ts` provides:
  - `tableStub(name)`;
  - `call(stub, user, method, path, body?)`, which sets `X-Yokohama-User` and, for a create, `X-Yokohama-Table-Id`;
  - `sqlOf(stub, query)`, a `runInDurableObject` read;
  - `seedPlaying(name)`, which creates a 2P table, joins, keeps both and posts `1`.

```ts
import { env, evictDurableObject, runInDurableObject } from "cloudflare:test";
import { describe, expect, it } from "vitest";
import { call, seedPlaying, sqlOf, tableStub } from "./helpers";

describe("the table object (spec §2.4)", () => {
  // Battery 16: probing writes nothing — no table, no row, no alarm.
  it("probing an unknown id leaves sqlite_master empty", async () => {
    const stub = tableStub("never-created");
    const res = await call(stub, "red@t.test", "GET", "/api/games/never-created/table");
    expect(res.status).toBe(404);
    expect(await sqlOf(stub, "SELECT count(*) AS n FROM sqlite_master")).toEqual([{ n: 0 }]);
    expect(await runInDurableObject(stub, (_o, state) => state.storage.getAlarm())).toBeNull();
  });

  it("a create persists its table in one batch, and the object knows its own name", async () => {
    const stub = tableStub("fresh");
    const res = await call(stub, "red@t.test", "POST", "/api/games", CREATE_BODY, "fresh");
    expect(res.status).toBe(201);
    expect(await sqlOf(stub, "SELECT kind FROM docs ORDER BY kind")).toEqual([{ kind: "table" }]);
    // Pool fact 8: ctx.id.name inside the object — and the cell that proves it in alarm() is below.
    expect(await runInDurableObject(stub, (_o, state) => state.id.name)).toBe("fresh");
  });

  // The shell compares documents after EVERY dispatch: a GET's owed pump is persisted.
  it("a GET that pumps an owed bot move writes the record", async () => { /* the COMMITTED owed-pump fixture's TEXT,
     from test/fixtures.gen.ts (OWED_PUMP_TABLE_JSON, OWED_PUMP_JSON — never JSON.parse'd, pool fact and Global
     Constraints), written straight into docs through runInDurableObject under the object named "owed"; then
     evictDurableObject; GET …/record → 409; the docs.record row now differs from the owed record */ });

  // Battery 9 and 21 in the pool: evictions between the clicks, a chat post between them too.
  it("an open picker survives an eviction and a chat post", async () => { /* seedPlaying (whose `1` is the opening
     move); `1` opens PlaceA (Task 4's measured path); evictDurableObject; POST …/chat; evictDurableObject; `A09`;
     `confirm` — the record's last placement names A09 */ });

  it("a trapped dispatch persists nothing and answers 500", async () => { /* seedPlaying; runInDurableObject replaces
     the live Instance's module `dispatch` with one that calls its own `debug_trap()`; a move answers 500 with the
     sentence; the docs rows are byte-identical to before */ });

  it("two traps during the wake poison the table, named", async () => { /* seedPlaying; patch at MODULE scope (pool
     facts 2, 4) — `Instance.prototype.get` from ../src/module, wrapped so the module it returns has a
     `restore_picker` that throws new WebAssembly.RuntimeError("unreachable"), the original kept; evict, request →
     500; evict, request → 500; the third request is 500 "this table's stored record cannot be loaded" and the
     patched `get` was NOT called for it; restore the original in `finally` */ });

  it("ctx.id.name is the object's name inside alarm()", async () => { /* a table; set an alarm through
     runInDurableObject; patch Table.prototype.alarm at module scope to record this.ctx.id.name, restored in
     finally; runDurableObjectAlarm → the recorded name equals the table's id. If this fails, fall back to a
     meta `id` row written in the create's batch, and record a ruling (pool fact 8) */ });

  // Battery 1 (the shell half; "before the record is read" rests on the native M6.3 sweep).
  it("a spectator's move is refused", async () => { /* a playing unlocked table; a stranger POSTs input → 403;
     the record row unchanged */ });
});
```

  The bodies given as comments name every request and assertion; write them in full. `CREATE_BODY` is Task 2's `create` body without its `id`.

- [ ] **Step 2: Watch them fail. Step 3: implement `storage.ts` and `table.ts`, as the path above says.** Nothing in the shell `JSON.parse`s a record.

- [ ] **Step 4: Watch them pass. Then run the per-edit gates, including `npm test` in `worker/` — which rebuilds nothing, so run `npm run build:module:debug` first whenever a Rust task has changed the module.**

- [ ] **Step 5: Commit.**

```bash
git add worker
git commit -m "feat(worker): the table object — one batch per request, a wake that re-feeds the picker, traps that cost one instance"
```

---

### Task 10: The table's sockets and transport routes (R-6.5c.6; spec §2.4 "Sockets" and "The framing"; Review Focus 3, 5)

**Files:**
- Create: `worker/src/frames.ts`, `worker/src/frames.unit.test.ts`, `worker/test/sockets.test.ts`
- Modify: `worker/src/table.ts`

**Interfaces:**
- Consumes: the `events(id, user, query)` export (Task 3), `Replied.bumps`.
- Produces:
  - `frame(event: "signal" | "house" | "end", dataText: string): string`, which returns `` `{"event":${JSON.stringify(event)},"data":${dataText}}` `` with the data embedded RAW, byte-identical to the SSE frame's data;
  - `Table.sendFrames(id: string, bumps: Bump[]): void`, called synchronously right after `commit` (the output gate holds every send until the batch is durable, spec §0.4).

**The routes** (spec §2.3 transport parity):
- `…/events`: a `GET` is `events(id, user, url.search.slice(1))`; any other method is **405** with an empty body.
- `…/stream`: a non-`GET` is **405**; a `GET` that is not an upgrade is **426** `{"error":"this host streams over WebSocket"}`.
- An upgrade is admitted by `events(id, user, "")`:
  - on **200**, `ctx.acceptWebSocket(server, ["t"])`, `serializeAttachment({ user })`, and send `frame("signal", body)` at once;
  - on **403 or 404**, ACCEPT, send `frame("end", '{"reason":"gone"}')`, and close with **4403** or **4404**, the layer's sentence as the reason, passed through `closeReason` (below). A reason over 123 UTF-8 bytes makes `close()` throw and leaves the socket open (pool fact 6);
  - on **401**, answer the 401 unupgraded (a browser cannot read it, and §2.7's probe does);
  - on ANY other status (an unusable id's refusal, a 500), answer it unupgraded, exactly as the layer answered it.
  - `closeReason(sentence: string): string` in `frames.ts` truncates at a UTF-8 boundary to at most 123 bytes. Every close goes through it.
    - Today no real 403 or 404 sentence reaches that length: a safe id is at most 64 ASCII characters (`store.rs:173-178`), so `no game named "<id>"` is at most 80 bytes. The bound costs one function and keeps a longer future sentence from leaving a socket open (pool fact 6).
    - Its cell is a node unit test (`frames.unit.test.ts`): a 200-byte ASCII sentence, one that ends in a multibyte character straddling byte 123, and a short one kept whole.

**Frames on bumps:**
- **Coalesced: ONE frame per socket per REPLY**, however many bumps the reply carries. A finalizing keep carries `bump_table` then `bump` (Task 2), and two identical frames would each wake a screen (plan review m5). `end` wins over any other bump in the same reply.
- If the reply's bumps name this table (`bump` or `bump_table`), then for every socket (`ctx.getWebSockets()`), take its attachment's `user` and compute `events(id, user, "")` — P-2's pin applies, since `events` can reach the clock. A 200 is sent as `signal`; a 403/404 is `end {"reason":"gone"}` plus a 4403/4404 close; anything else sends NOTHING (the SSE's comment arm).
- An `end` bump sends `end {"reason":"deleted"}` and closes **4404** (a short fixed reason).
- `webSocketMessage` ignores everything: a socket carries no move. `setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"))` is set in the constructor.

- [ ] **Step 1: The failing pool cells.** An upgrade in the pool is `stub.fetch(url, { headers: { Upgrade: "websocket", "X-Yokohama-User": user } })`; then `res.webSocket!.accept()`, and messages collected into an array.

```ts
describe("the table's sockets (spec §2.4)", () => {
  it("the first frame on accept is the events route's answer, framed", async () => { /* seedPlaying; open as red;
     the first message equals frame("signal", <the body of GET …/events as red>) BYTE for byte */ });

  it("one frame per socket per reply, each for its own viewer", async () => { /* red and blue each hold a socket;
     red makes the opening move; each socket receives exactly ONE signal frame, computed for its own viewer: its
     your_turn matches whether that viewer's seat is to move */ });

  it("a finalizing keep's two bumps are ONE frame per socket", async () => { /* a dealing table; red's socket open;
     red keeps, then blue's keep finalizes — the reply carries bump_table then bump — and red's socket receives ONE
     signal frame for it, not two (plan review m5) */ });

  // Review Focus 5.
  it("one person with two tabs gets every frame on both", async () => { /* two sockets as red; blue moves; both
     receive the same signal frame */ });

  // Battery 24: an admission lost under a live socket.
  it("a stranger holding an OPEN table's stream is ended when the last seat fills", async () => { /* an open table;
     a stranger opens a socket (admitted: events answers any caller at open); blue joins and fills the table →
     the stranger's socket receives end {"reason":"gone"} and closes 4403 */ });

  it("admission refused at the upgrade is accepted, ended and closed with the layer's code", async () => { /* a
     dealing table; a stranger's upgrade → end gone then close 4403 with the layer's sentence as the reason; an
     unknown id → 4404 */ });

  it("a delete ends every socket as deleted, 4404", async () => { /* a playing table; sockets as red and as blue;
     red, the creator, DELETEs (204); each socket receives exactly {"event":"end","data":{"reason":"deleted"}} and
     then a close with code 4404; no signal frame arrives after it */ });

  it("a socket message is never a move", async () => { /* send '{"line":"1"}' and "1"; the record row unchanged */ });

  it("a failure other than 403/404 sends nothing", async () => { /* replace the live module's events with one
     answering 500; bump; no message arrives within the cell */ });

  // Spec §2.4 "Traps": a trap in a SIDE CHANNEL comes after the batch and the reply.
  it("a trap while framing discards the instance, sends nothing, and the next request is answered afresh", async () => {
     /* replace the live module's events with one that calls debug_trap(); a move answers 200 and its batch is
        stored; no frame arrives; the next GET re-instantiates, reloads and answers 200 */ });

  it("the transport routes answer as axum's router does", async () => { /* POST …/events 405 empty; POST …/stream
     405 empty; GET …/stream without Upgrade 426 with the host sentence; GET …/events?wait=26 is the restated
     400 */ });

  // Hibernation, and battery 8.
  it("a hibernated socket keeps its viewer and receives the next frame; wakes count the eviction", async () => {
    /* COUNT_WAKES on; a socket as blue; evictDurableObject(stub) — hibernates the socket; red moves; blue's socket
       receives the frame computed for BLUE; meta.wakes rose by one */ });

  // Review Focus 3, the object's half.
  it("an abort with sockets open loses no acknowledged move", async () => { /* red moves (200); sockets open as red
     and blue; abortAllDurableObjects(); both sockets report close 1006; take a FRESH stub (pool fact 3 — the old one
     is dead); a new socket as red receives a first frame whose events count includes the move; red's next move
     answers 200 */ });


});
```

  Write each in full from its comment.

- [ ] **Step 2: Watch them fail. Step 3: implement. Step 4: pass, and the gates.**

- [ ] **Step 5: Commit.**

```bash
git add worker
git commit -m "feat(worker): the table's hibernatable sockets — SSE's frames in JSON, one per viewer, refusals ended by close code"
```

---

### Task 11: The Worker, the first cut of the house and the archive object, and THE FIRST DEPLOY (R-6.5c.1, .3, .10; spec §2.3, §3.2, §3.3)

**Files:**
- Create:
  - `worker/src/identity.ts`, `worker/src/identity.unit.test.ts`, `worker/src/route.ts`, `worker/src/route.unit.test.ts`;
  - `worker/src/house.ts`, `worker/src/archive.ts`;
  - `worker/test/worker.test.ts`, `worker/scripts/read-deploy.mjs`.
- Modify:
  - `worker/src/index.ts` — the real `handle`, plus `export { Table, House, Archive }`;
  - `worker/wrangler.jsonc` — the three DO bindings, ONE migration `{"tag":"v1","new_sqlite_classes":["Table","House","Archive"]}`, the custom domain and the variables;
  - `worker/wrangler.test.jsonc` — the three bindings;
  - `deploy/RUNBOOK.md` — §6 new; §4 rewritten.
- Delete: `deploy/worker/`, `deploy/modal_app.py`.

**Interfaces:**
- Produces:
  - `handle(req: Request, env: Env, ctx: ExecutionContext, keys: (refresh: boolean) => Promise<JwkSet>): Promise<Response>`, and `default { fetch: (req, env, ctx) => handle(req, env, ctx, (refresh) => teamKeys(env.ACCESS_TEAM_DOMAIN, { refresh })) }`;
  - `resolveIdentity(headers: Headers, keys: JwkSet, c: IdentityConfig): Promise<Identity>`, where `Identity = { kind: "person"; email } | { kind: "dev"; email } | { kind: "none"; unknownKid?: true }`;
  - `teamKeys(teamDomain, { refresh?: boolean })` — the certs, cached per isolate. `handle` calls `resolveIdentity`; on `{ kind: "none", unknownKid: true }` it fetches the keys again with `refresh: true`, AT MOST once a minute, and resolves once more. That is how a key rotation is survived without letting a forged `kid` force a fetch per request (plan review m14);
  - `route(method, pathname): Target` — `missing | enter | art | not-allowed | create | table{name} | archive | admin{rest} | house`;
  - `class House`, `class Archive`;
  - `const BODY_LIMIT = 2 * 1024 * 1024`, and `BODY_LIMIT_SENTENCE` (Step 7 pins it).

**The identity function** — security-critical, so it is given here in full:

```ts
export type Identity =
  | { kind: "person"; email: string }
  | { kind: "dev"; email: string }
  | { kind: "none"; unknownKid?: true };
export interface JwkSet { keys: Array<JsonWebKey & { kid?: string }> }
export interface IdentityConfig {
  teamDomain: string; aud: string; devDoorClientId: string; devDoor: boolean; nowSeconds: number;
}

const bytes = (b64url: string): Uint8Array => {
  const b64 = b64url.replace(/-/g, "+").replace(/_/g, "/").padEnd(Math.ceil(b64url.length / 4) * 4, "=");
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
};
const json = (b64url: string): Record<string, unknown> => JSON.parse(new TextDecoder().decode(bytes(b64url)));

/** R-6.5c.3, in order. A missed Access destination, a forged token, another
 *  application's token and an expired one ALL fail closed: `none`, and the
 *  layer's own 401 answers. */
export async function resolveIdentity(h: Headers, keys: JwkSet, c: IdentityConfig): Promise<Identity> {
  const token = h.get("cf-access-jwt-assertion");
  const parts = token?.split(".");
  if (!parts || parts.length !== 3) return { kind: "none" };
  let head: Record<string, unknown>, claims: Record<string, unknown>;
  try {
    head = json(parts[0]!);
    claims = json(parts[1]!);
  } catch {
    return { kind: "none" };
  }
  if (head.alg !== "RS256") return { kind: "none" };
  const jwk = keys.keys.find((k) => k.kid === head.kid);
  if (!jwk) return { kind: "none", unknownKid: true }; // the caller may refresh the keys once
  const key = await crypto.subtle.importKey("jwk", jwk, { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" }, false, ["verify"]);
  const signed = new TextEncoder().encode(`${parts[0]}.${parts[1]}`);
  if (!(await crypto.subtle.verify("RSASSA-PKCS1-v1_5", key, bytes(parts[2]!), signed))) return { kind: "none" };
  const aud = Array.isArray(claims.aud) ? claims.aud : [claims.aud];
  if (!aud.includes(c.aud)) return { kind: "none" };
  if (claims.iss !== `https://${c.teamDomain}`) return { kind: "none" };
  if (typeof claims.exp !== "number" || claims.exp <= c.nowSeconds) return { kind: "none" };
  if (typeof claims.nbf === "number" && claims.nbf > c.nowSeconds + 60) return { kind: "none" };
  if (typeof claims.email === "string" && claims.email.trim() !== "") {
    return { kind: "person", email: claims.email.trim().toLowerCase() };
  }
  if (c.devDoor && claims.common_name === c.devDoorClientId) {
    const claimed = h.get("x-yokohama-claimed-user")?.trim().toLowerCase();
    if (claimed) return { kind: "dev", email: claimed };
  }
  return { kind: "none" };
}
```

**`handle`**, in this order (spec §2.3):
1. `route`:
   - `missing` answers **404** with an EMPTY body, never `index.html`;
   - `not-allowed` answers **405** empty — a non-GET/HEAD on `/api/art` or `/enter`, as axum's router answers;
   - `enter` answers **302** `Location: /#/lobby`.
2. `shellConfig(env)`: an error answers **500** `{"error": <the sentence naming the variable>}` for every `/api/*`.
3. The body, read ONCE for a method that carries one, with a streaming bound. Over `BODY_LIMIT` it answers **400** `{"error": BODY_LIMIT_SENTENCE}`.
4. `art` (GET/HEAD) answers `env.ASSETS.fetch(new URL("/art/manifest.json", url))`'s body as `application/json` (an empty body for HEAD).
5. `resolveIdentity`. A certs fetch that fails is `none`, so the request fails CLOSED.
6. **The strip rule** — a fresh `Headers` copied WITHOUT any header whose name starts `x-yokohama-` (`deploy/worker/src/index.ts:71` stripped only `x-yokohama-user`, and that was impersonation). Then `X-Yokohama-User` is set when there is an identity.
7. Forward, with `new Request(url, { method, headers, body })`. The upgrade headers ride along, so an upgrade forwards, and the object's `101` is returned as it is.
   - `create`: `Table` named by the body's `id` when the body `JSON.parse`s to an object whose `id` is a string, else by `sim-` + 16 hex from `crypto.getRandomValues`. The name goes in `X-Yokohama-Table-Id`, percent-encoded.
   - `table{name}`: `Table.idFromName(name)`. An EMPTY segment routes to `house` instead.
   - `archive`: `ARCHIVER` named `"archive:" + email` with an identity, and `house` without one (the layer's 401).
   - `admin`: `house`, whose `dispatch` answers the layer's 404 — the absent-variable path (Task 14 fills it).
   - `house`: `HOUSE.idFromName("house")`.

**The first cut of `House`** (Task 12 completes it):
- A module instance whose store holds the house chat, persisted as `docs(kind='house')` in the same batch shape as the table's.
- Every request but the lobby socket is `dispatch`: health, me, labels, chat, `GET /api/games` (an empty store until Task 12's copies) and every unrouted `/api/*`.
- The lobby socket `/api/stream` is admitted by the `identity` export: 401 unupgraded, else accepted with `{ user }` under the tag `"lobby"`, and nothing is sent on connect (spec §2.5).
- A `bump_house_chat` sends `frame("house", house_frame(seq, chat))` to every lobby socket. `seq` is an in-memory counter; `chat` is the house chat document's `last`, and that document may be `JSON.parse`d — it carries no seed.

**The first cut of `Archive`** (Task 13 completes it): one per caller, whose `init` config carries `caller`. It writes nothing. With the archive `off`, `POST /api/archive` is the layer's 404 `"this server keeps no archive"`.

- [ ] **Step 1: The failing pure cells** (`test:scripts`, node).
  - `identity.unit.test.ts`: generate an RSA key pair with WebCrypto, export its public JWK with a `kid`, and sign crafted tokens with a small helper in the test. Then:
    - a person's token answers `person`;
    - a person's token WITH a client `X-Yokohama-Claimed-User` answers `person`, and the header is ignored;
    - the credential's token (`common_name` = the client id, no `email`) with the door on and a claimed header answers `dev`;
    - the same with the door off, or with no claimed header, answers `none`;
    - another `aud`, another `iss`, an expired `exp`, a future `nbf`, a forged signature (signed by a second key under the first key's `kid`), `alg: "none"`, and a token that is not three parts each answer `none` (**battery 15**);
    - an unknown `kid` answers `none` WITH `unknownKid: true`, and a `handle` cell shows one refetch and then `none` when the refetched set still lacks the `kid`.
  - `route.unit.test.ts`: every row of spec §2.3's table, including the empty segment, non-GET `/api/art` and `/enter`, `GET /api/games` as `house`, and `POST /api/games` as `create`.

- [ ] **Step 2: The failing pool cells** (`test/worker.test.ts`), which call `handle` directly with a TEST key source, so no network and no backdoor is involved:
  - a missing file that reaches the Worker answers **404** with an empty body, and never `index.html`. This is the Worker's own answer. Battery 14's mutant lives in the asset layer's `not_found_handling`, which answers without invoking the Worker; Task 7's deploy guard catches it, and the live reading at each deploy reads it (plan review M5);
  - `/enter` answers 302;
  - a body of 2 MiB + 1 answers 400 with the sentence;
  - a bad `YOKOHAMA_WEB_ARCHIVE` answers 500 naming it on `/api/health`, and `/` is untouched — it never reaches `handle`, and the asset layer answers it;
  - **(battery 2)** a person's assertion PLUS client `x-yokohama-user: evil@x.test`, `x-yokohama-claimed-user: evil@x.test`, `x-yokohama-operator: evil@x.test` and `x-yokohama-table-id: elsewhere` gives `GET /api/me` the PERSON's address. The same forged four on a create land it at the body's own id, not at `elsewhere`, and never reach an operator route — those are exactly the names the objects trust (plan review m16);
  - a `dev` identity gives `/api/me` the claimed address;
  - no identity gives `/api/me` `user: null`, and `POST /api/archive` the layer's 401;
  - a create with no body `id` lands at a `sim-` + 16 hex object, and answers 201 naming it;
  - `GET /api/art` equals `crates/yokohama-web/assets/art/manifest.json` BY VALUE;
  - an upgrade to `/api/stream` with no identity answers 401 unupgraded; with one, 101.

- [ ] **Step 3: Watch them fail. Step 4: implement. Step 5: pass, then run every gate.**

- [ ] **Step 6: Retire the old Worker** (R-6.5.13, spec §3.3).
  - The strip rule is now in `identity.ts`'s caller and is pinned by battery 2's cell. Delete `deploy/worker/` and `deploy/modal_app.py`.
  - Rewrite RUNBOOK §4 as the record of why they were retired, citing the strip defect (`x-yokohama-claimed-user` was not stripped) and umbrella R-6.5.13.
  - `grep -rn "deploy/worker\|modal_app" --include=*.md --include=*.yml --include=*.ts --include=*.rs .` — every hit that is not a dated record is updated.

- [ ] **Step 7: THE FIRST DEPLOY — the ORCHESTRATOR runs this step, never a subagent** (spec §3.2).
  - **Before the deploy:**
    1. The production `wrangler.jsonc` carries `YOKOHAMA_WEB_ARCHIVE: "off"` (the bucket is Task 13's), `YOKOHAMA_DEV_DOOR: "on"`, the three Access values (the team domain and audience of the `yokohama` application, spec §0.5, and the `yokohama-worker` client id, which is public) and the chat variables at their defaults. It carries NO TTL and NO admins yet.
    2. **Pin the body-limit sentence first.** Over WARP as the owner, with no credential, run:
       `curl -s -o - -w '%{http_code}' -X POST -H 'content-type: application/json' --data-binary @<(head -c 2097153 /dev/zero | tr '\0' 'a') https://yokohama-nas.moore.nyc/api/games`
       Set `BODY_LIMIT_SENTENCE` to the exact `error` text, and update the pool cell if it differed.
    3. Then: `cd web && npm run build && cd ../worker && npm run build && npm run deploy`.
  - **The readings.** Record each in the ledger AND in RUNBOOK §6, the fourteenth deployment's record (numbering continues the container era's).
    1. **Protection from an unenrolled vantage** — the spike's: an alpine container on the NAS through Portainer, running curl.
       - `/`, `/sw.js`, `/api/health` and `/enter` each answer **302** to the Access login.
       - A WebSocket upgrade with no credential answers **403**.
       - `yokohama-web.<account subdomain>.workers.dev` answers **404**.
    2. **Identity on an upgrade**, with `curl --http1.1 -i` and the upgrade headers against `/api/stream`:
       - as the owner over WARP it answers 101;
       - with the credential (read from `.dev.vars` by the command, NEVER echoed) and `X-Yokohama-Claimed-User: probe@deploy.test` it answers 101;
       - with the credential and no claimed header it answers the layer's 401.
    3. **The static assets**, with `node scripts/read-deploy.mjs assets`. For every entry in the DEPLOYED `/sw.js` it checks 200, the type's essence and the SHA-256. Then:
       - `/index.html` answers 200, not 307;
       - a missing `/assets/x.js` answers 404 with an empty body;
       - `wrangler tail --format json` running beside it shows ZERO invocations for the real files, and ONE for the missing file;
       - the service worker installs completely in headless Chromium: the credential is the context's headers, the worker is active, the release cache holds every entry, and `/index.html` is cached clean.
    4. **Cold start** (the deploy's "Worker Startup Time"), and **the Worker's CPU with JWT verification** off the tail (R-6.5c.5 left it unmeasured).
    5. `/api/health` over WARP reads `identity: "cf-access"`, `signal: "ws"`, `archive: "off"`.
  - **A reading that differs from the spike's** (§0.3) is a FINDING. Stop, and bring it to the owner before Task 12.

- [ ] **Step 8: Commit** (the RUNBOOK record included).

```bash
git add worker deploy/RUNBOOK.md
git rm -r deploy/worker deploy/modal_app.py
git commit -m "feat(worker): the Worker — Access validated, the dev door, the strip rule; the first deploy re-reads the spike"
```

---

### Task 12: The house and the slot — and from here the pool replays the whole corpus (R-6.5c.8; spec §2.5, §5.3's pool half; Review Focus 4)

**Files:**
- Create: `worker/src/slot.ts`, `worker/src/slot.unit.test.ts`, `worker/test/gates.ts`, `worker/test/walk.ts`, `worker/test/slot.test.ts`, `worker/test/corpus.test.ts`
- Modify: `worker/src/table.ts` (the generation, the slot, `deliverSlot`, the removal and the create's 409), `worker/src/house.ts` (the copies, `apply`, the lobby frames)

**Interfaces:**
- Consumes: `house_copy`, `put_copy`, `remove_copy` and `house_frame` (Task 5); `frame` (Task 10); `route` and `handle` (Task 11).
- Produces:
  - In `slot.ts`, all pure:
    - `type Generation = { at: number; rand: string }` — `at` is the create's P-2 reading (equal to the table's `created_at`, which the layer stamps from that same pinned clock; the shell never parses the table document for it), and `rand` is 16 hex drawn once at create;
    - `cmpGen(a, b): number`;
    - `type Slot = { id: string; generation: Generation; revision: number; copy: string | null /* null = REMOVAL */; last_active: number; bytes: number; attempt: number }`;
    - `type HouseRow = { current: { generation: Generation; revision: number } | null; retired: Generation[] }`;
    - `decide(row: HouseRow, slot: Slot): "apply" | "ignore"`;
    - `advances(acked: { generation: Generation; revision: number } | null, g: Generation, r: number): boolean`;
    - `nextRetry(attempt: number): number` — 30 s, 2 min, 10 min, then hourly.
  - `Table.deliverSlot(): Promise<void>` — the ONE delivery routine. The side channel after a reply, the alarm, and the corpus replay's flush all call it.
  - `House.apply(slot: Slot): Promise<{ generation: Generation; revision: number }>` — an RPC method. It ACKS every push, applied or ignored, so a straggler is cleared too.
  - The house's `copies` table `(id PRIMARY KEY, json, gen_at, gen_rand, revision, last_active, bytes)` and `retired` table `(id, gen_at, gen_rand, at, PRIMARY KEY (id, gen_at, gen_rand))`.

**The rules** (spec §2.5, verbatim where it matters):
- **The generation** is drawn once, in the create's batch.
- **When the scrubbed copy DIFFERS from the one last put in motion** — the slot's pending copy if occupied, else `acked`'s digest — the table increments `revision` and writes the slot in the request's batch, with its alarm at +30 s.
  - `house_copy` is computed BEFORE the `transactionSync`.
  - A copy it refuses is LOGGED and writes no slot (R-6.5c.8).
- **After the reply**: `ctx.waitUntil(this.deliverSlot())`. On the ack of `(g, r)`, ONE batch:
  - the slot is cleared ONLY if it still holds `(g, r)`;
  - `acked` is set ONLY if `advances(acked, g, r)`;
  - the alarm is re-armed to the next deadline, or deleted when nothing is due.
- **The house applies** by `decide`. An applied removal retires its generation and deletes the row, and retired generations are pruned after 90 days. Every applied copy or removal raises `seq` and sends `frame("house", house_frame(seq, chat))` to every lobby socket.
- **A delete's 204** (an `end` bump), in ONE batch:
  - the `docs` rows and the `picker` row are removed;
  - the slot takes a REMOVAL (`copy: null`, `revision + 1`), with the alarm.
  At THIS task the object calls `deleteAll()` as soon as the removal is acknowledged; Task 14 puts the marker's 90 days between the two.
- **A create at an id whose slot holds an unacknowledged REMOVAL** is **409** `{"error":"this name was used recently — try another"}` (spec §2.4; battery 36).

- [ ] **Step 1: The failing pure cells** (`slot.unit.test.ts`, node): `decide` over every row of spec §2.5.
  - A current generation with a higher revision applies; with an equal or lower one it is ignored.
  - A retired generation is ignored whether or not a newer one is current — BOTH orders (**battery 18**).
  - A new generation with no current applies only if it orders after every retired one.
  - `advances` holds for a newer revision and for a newer generation, and not for an equal or older one.
  - `nextRetry` answers 30 000, 120 000, 600 000, 3 600 000 and 3 600 000.

- [ ] **Step 2: Two test helpers, then the failing pool cells.**
  - **`test/gates.ts`** — the pool facts' technique, in one place:
    - `holdApply(byRevision?: number[]): { release(rev?: number): void; restore(): void }` wraps `House.prototype.apply` (imported from `../src/house`, the original kept). A held call awaits a gate; with `byRevision`, only those revisions are held, in a `Map<number, Promise<void>>` keyed by `slot.revision`.
    - `failApplyOnce(): { restore(): void }` — a prototype patch that throws once, then restores itself.
    - Every cell calls `restore()` in `finally`, and before any `runDurableObjectAlarm` that follows an abort (pool fact 2).
  - **`test/walk.ts`** — the corpus replay's driver, shared with the battery-35 cell. It replays `walk.json` (and, when asked, `walk-host.json` merged by `after`) against the objects, as Step 3 describes, and calls `onRow(index, row, reply)` after each row. Step 3's replay is its main client.
  - **Every cell uses ids of its own**, and filters every house listing it asserts on to them (pool fact 5).

```ts
describe("the durable slot (spec §2.5)", () => {
  it("a create's copy reaches the house and the lobby lists it", async () => { /* create "slot-a"; deliverSlot via
     runInDurableObject; GET /api/games?scope=open as a stranger through the house → a row for "slot-a" */ });

  it("a lower revision after a higher one leaves the row at the higher (battery 5)", async () => { /* push r2 then r1
     straight into the house's apply through runInDurableObject for id "slot-b"; the stored copy is r2's */ });

  it("a stale ack of r after the slot was rewritten to r+1 — or to a REMOVAL — leaves the slot (battery 6, 22)", async () => {
     /* holdApply([r]) where r is the create's revision on "slot-c"; the create's push is held; a join changes the
        copy → slot r+1 (its push is not held, so r+1 is applied and acked, clearing the slot); release(r): r's late ack
        arrives — the slot is EMPTY and must stay empty, acked stays r+1. Then the REMOVAL arm: holdApply([r+2]); red's
        keep changes the copy (a keep's presence is in the scrub) at r+2, held; the creator deletes the dealing table
        → slot = REMOVAL (r+3); release(r+2): its ack does not clear the removal; deliverSlot delivers the removal and
        the house drops the row */ });

  it("acked never regresses, so a copy that returns to an earlier value is still delivered (battery 35)", async () => {
     /* the natural case, needing no undo: with two humans the scrubbed copy alternates, because `turn.to_move`
        flips — seat 1 is to move (copy X), then seat 0 (copy Y), then seat 1 again (copy X, BYTE-equal to the first
        when nothing else in the table document changed).
        Arrange with test/walk.ts on the walk's own table `wire`, in a stretch AFTER its last unlock and join, watching
        meta.slot at each row: when a row puts copy X in motion at revision r, holdApply([r]); let the next turn change
        put copy Y (r+1) through, applied and acked; release(r) — its ack arrives LATE; let the walk run to the next
        change back to X (r+2), asserting the premise that r+2's copy is byte-equal to r's; deliver; the house's row
        for `wire` shows `turn.to_move` == 1.
        Under the mutant — `acked` recorded without the advance check — r's late ack regresses `acked` to X's digest,
        r+2's copy equals it, no push is made, and the row still shows to_move 0 */ });

  it("a failed push is retried by the alarm", async () => { /* failApplyOnce(); a create on "slot-e"; the push fails;
     runDurableObjectAlarm on the table is true; the house lists it */ });

  // Battery 25. The FIRST text used evictDurableObject, which drains the push (vacuous); the SECOND patched the
  // instance, which makes the RPC THROW rather than hang (vacuous again — measured true/true by the plan's review).
  it("the alarm is set IN the batch: an abort mid-push is finished by the alarm", async () => {
    /* const held = holdApply(); a move that changes the copy on "slot-f" (its push hangs on the gate, neither failing
       nor succeeding); abortAllDurableObjects() — no drain; held.restore() BEFORE anything else (pool fact 2);
       take FRESH stubs (pool fact 3); runDurableObjectAlarm(freshTable) is TRUE and the house lists the new copy.
       Under the mutant ("armed only after a failure") no alarm exists: FALSE — measured by the plan's review. */ });

  it("a move that leaves the copy alone writes no slot and leaves the alarm where it was", async () => {
     /* a playing two-human table "slot-g" after its first delivery (slot empty; read getAlarm() → A); red's opening
        `1` applies a move that keeps seat 0 to move, so the copy is unchanged; meta.slot is still absent, revision is
        unchanged, and getAlarm() is still A */ });

  it("a create while a removal is unacknowledged is 409; after the ack the object is gone and the id is free", async () => {
     /* tick clock on this object (set through runInDurableObject before its first request) so the re-created
        generation orders after the retired one (plan review m7); holdApply(); the creator deletes "slot-h" (removal
        held); create at "slot-h" → 409 with the sentence; held.release(); deliverSlot; sqlite_master empty;
        create → 201 and the house lists the NEW table (battery 36) */ });

  // Review Focus 4, through the objects.
  it("a non-ASCII table name and display name list exactly as created", async () => { /* create "slot-i" named
     "Café ☕ 東京" with display_name "Zoë"; deliver; GET /api/games as the creator and as a stranger: the
     characters verbatim */ });

  it("the lobby socket is silent on connect and hears every applied copy", async () => {
     /* open /api/stream on the house as red; wait until a second, unrelated request to the house has answered —
        nothing has arrived; create "slot-j" and deliver: exactly ONE message arrives,
        {"event":"house","data":{"seq":<n>,"chat":<m>}} with the data byte-equal to house_frame(n, m) */ });
});
```

- [ ] **Step 3: The failing corpus replay** (`test/corpus.test.ts`), driven by `test/walk.ts`. Its shape is `web/src/local/corpus.node.test.ts`'s, and **read that file first**. It is ONE file whose objects no other file shares (pool fact 5), and the walk's own ids are fixed, so nothing else may create `wire`, `wire2` or `wire4` in it.
  - **The rows:** `walk.json`'s rows, with `walk-host.json`'s rows run each after the walk row its `after` names (the art row first). Each goes to the object `route` names, with `X-Yokohama-User` set to the row's `who`, as the Worker would set it. A create's `X-Yokohama-Table-Id` is its body's `id`; the art row goes through `handle`.
  - **Before any row the house answers,** `deliverSlot()` is flushed on every table touched since the last such row. That flush is how the pool models "the push landed", and nothing else about the order is changed.
  - **Assertions:**
    - every row's STATUS;
    - every NAMED row deep-equal to its committed file, with `HOST_FIELDS = { health: ["identity", "signal"], me: ["mode"], me_signed_out: ["mode"] }`;
    - each exempted field asserted to DIFFER from the committed value, since M6.5b's battery showed an exemption that differs nowhere is a hole.
  - **The third column** is derived as the node replay derives it, and asserted equal to `["explanation_terminal", "house_chat_off_404", "locked", "site_space_words", "watching", "why", "why_refused", "why_refusals", …the eight archive_* until Task 13]`.
  - **The clock** is `frozen:1789000000` (P-2).

- [ ] **Step 4: Watch them fail. Step 5: implement. Step 6: pass, and run every gate.** The corpus replay is now part of `npm test` in `worker/` (**battery 7**).

- [ ] **Step 7: DEPLOY and read — the ORCHESTRATOR.**
  - Create a table as `probe@deploy.test` through the credential. It is listed for its creator (`scope=mine`) and for a second `.test` identity (`scope=open`), and the listing's JSON carries no `seed` value and no `lock` salt or hash.
  - A lobby socket opened with `curl --http1.1` hears the house frame when the table is joined.
  - Abandon the table. Its row leaves the lobby, and the object is gone.
  - Record it all as RUNBOOK §6's next deployment.

- [ ] **Step 8: Commit.**

```bash
git add worker deploy/RUNBOOK.md
git commit -m "feat(worker): the house over scrubbed copies, a durable ordered slot per table, and the corpus replayed through the objects"
```

---

### Task 13: The archive on R2 — the receipt loop, game over, the switch, and the bucket (R-6.5c.9, R-6.5c.14; spec §2.6)

**Files:**
- Create: `worker/src/r2.ts`, `worker/src/r2.unit.test.ts`, `worker/test/archive.test.ts`
- Modify:
  - `worker/src/table.ts` — the abandon's loop, `archive_owed`, the stamp;
  - `worker/src/archive.ts` — the caller's loop;
  - `worker/src/config.ts` — already validates the switch, so this is only its use;
  - `worker/wrangler.test.jsonc` — an `r2_buckets` binding `ARCHIVE`, and `YOKOHAMA_WEB_ARCHIVE: "r2"`;
  - `worker/test/corpus.test.ts` — `archive-walk.json`, and `health`'s `archive` becomes a HOST field;
  - `worker/wrangler.jsonc` and `deploy/RUNBOOK.md` (Step 7).

**Interfaces:**
- Consumes: `receipt`, `stamp_archive` and `set_clock` (Tasks 2, 6); `Replied.archive` (the plan).
- Produces:
  - `executePlan(bucket: R2Bucket, plan: Plan): Promise<Outcome>` in `r2.ts`, in `DirArchive::put`'s ORDER (spec §2.6):
    1. `get` the RECORD key — the same bytes are `identical`, different bytes `conflict`, absent continues;
    2. `put` the TABLE key unconditionally;
    3. `put` the RECORD key CREATE-IF-ABSENT. If the precondition fails, re-read it: the same bytes are `identical`, different ones `conflict`;
    4. any other failure is `failed`, carrying R2's message.
    The conditional form is `onlyIf: new Headers({ "If-None-Match": "*" })`; Step 7 re-reads it live.
  - `type ArchiveOwed = { plan: Plan; reading: number; attempt: number }` in `meta.archive_owed`.

**The loops:**
- **Abandon** (`DELETE` by the creator) and **the browser's save** (the Archive object) each run under `ctx.blockConcurrencyWhile`:
  1. P-2's reading is taken ONCE and pinned for the first `dispatch`.
  2. If its `archive` plan is present: `executePlan`, `receipt(plan.key, outcome)`, and the SAME request dispatched AGAIN at the SAME reading, whose answer is the answer.
  3. `set_clock(prior)` restores the pin in a `finally`.
  4. A delete that owes no save completes in its first dispatch.
- **Game over**: ANY reply's `Replied.archive` — the wake's `restore_picker`, a GET's, an `events` call's, a move's — whose plan the layer did NOT re-dispatch. Its `meta.archive_owed = { plan, reading, attempt: 0 }` and alarm (+30 s) go IN THAT REQUEST'S BATCH, with the reading pinned for the call that made it (P-2; plan review M4). After the reply comes `executePlan`:
  - on `landed` or `identical`: `receipt` then `stamp_archive(id)` at the RECORDED reading, and the stamped table row, the cleared `archive_owed` and the slot the changed copy owes land in ONE batch;
  - on `failed`: the alarm retries by `nextRetry(attempt)`, re-planning at the recorded reading after an eviction;
  - on `conflict`: logged, and `archive_owed` cleared ("final at every site").
- **The switch**, while `off`: an owed save is KEPT and not retried, and the alarm does not re-arm for it. Switching back on resumes it at the next request or alarm.
- **Every outcome is logged** (spec §2.8). The layer's own lines are mute on this host.

- [ ] **Step 1: The failing pure cells** (`r2.unit.test.ts`), over an in-memory fake bucket:
  - landed;
  - identical — the record present with the same bytes;
  - conflict — different bytes;
  - a HALF PAIR: the table present, the record absent, a stale table body — which LANDS and overwrites the table (**battery 20**);
  - the create-if-absent losing a race to identical bytes (`identical`) and to different ones (`conflict`);
  - a `put` that throws (`failed`, with its message).

- [ ] **Step 2: The failing pool cells** (`test/archive.test.ts`).
  - **The archive cells run with `tick:<n>`** (P-2), set through `runInDurableObject` on the live instance before its first request, so that an unpinned re-dispatch really does stamp a different `archived_at`. An eviction returns the object to the config's `frozen` clock (pool fact 4); the retry cells rely only on the RECORDED reading, which is stored.
  - **Every cell uses its own table ids and callers** (pool fact 5).
  - An abandon whose put FAILS leaves the object, its storage and the layer's refusal in place (**battery 10**). The failure is arranged by replacing `this.env` on the LIVE instance with a copy whose `ARCHIVE.put` throws. The replacement lasts until an eviction, and this cell performs none.
  - An abandon whose put lands deletes. Its keys are `test/rev7/abandoned/<id>.json` and `.table.json`, the record's bytes equal the stored record's, and the re-dispatch's `archived_at` equals the first's (**battery 17**).
  - A half pair, then a second abandon, LANDS.
  - A landed-but-unstamped game-over copy retried at a NEW reading answers `identical`, and stamps.
  - **Game over**, from the COMMITTED owed-game-over fixture: `OWED_GAME_OVER_TABLE_JSON` and `OWED_GAME_OVER_JSON` from `test/fixtures.gen.ts`, TEXT only, written into `docs` through `runInDurableObject` under the object named **`local-242a5429453c593c`**, its own id (plan review m12). Its first request's wake pumps the bot's run to the end (plan review M4). In order:
    1. `archive_owed` is written in THAT request's batch (the wake's plan);
    2. the upload happens after the reply;
    3. the receipt, then the stamped row with `archive_owed` cleared;
    4. a failing put is retried by `runDurableObjectAlarm`;
    5. a retry after `evictDurableObject` happens at the RECORDED reading;
    6. the "no longer over" clear (an undo at a one-person table).
  - A browser save through the Archive object answers the layer's 201, 200 `identical` and 409.
  - **A chat post arriving during a slow R2 put lands.** It must be the GAME-OVER upload, which runs after the reply outside any `blockConcurrencyWhile`; an abandon's put runs under it, so nothing can land during that one (plan review m6). Hold the put with a gate on a patched `ARCHIVE.put` the cell releases.
  - **A chat post arriving during a slow HOUSE push lands** (spec §5.2's other half): `holdApply()` (Task 12's helper), a move that changes the copy, a chat post that answers 201 while the push is held, then `release()` and deliver.
  - **The switch:**
    - `off` answers health `off`; game over puts nothing and arms nothing; an abandon deletes without a put; the route is the layer's 404;
    - an owed save is not retried while off (**battery 27**) and resumes when on.
  - A `conflict` at game over clears `archive_owed` and logs it.

- [ ] **Step 3: The corpus replay takes `archive-walk.json`,** routed to the Archive object named `archive:red@wire.test`. Each test file meets an empty bucket, which the script's first send assumes (spec §0.2). `health`'s `archive` joins `HOST_FIELDS` (it reads `r2` against the committed `off`), and the eight `archive_*` leave the third column.

- [ ] **Step 4: Watch them fail. Step 5: implement. Step 6: pass, and run every gate.**

- [ ] **Step 7: The bucket, and DEPLOY — the ORCHESTRATOR.**
  - **Create `yokohama-archive`** through the account API (the Cloudflare MCP's `execute`, `POST /accounts/{account_id}/r2/buckets` with `{"name":"yokohama-archive"}`). If that API refuses, it is an owner item.
  - Add the binding and `YOKOHAMA_WEB_ARCHIVE: "r2"` to `wrangler.jsonc`, and deploy.
  - **Ask the owner** whether to add a lifecycle rule on the bucket's `test/` prefix, where every live tier's saves accumulate (spec §7). It is optional, and it is theirs to set.
  - **Readings:**
    1. **The conditional put's form** (spec §2.6, §3.2). The REFUSAL — a create-if-absent put meeting a present record — is proven in the pool, against miniflare's R2. Live, the reading is that the chosen form is ACCEPTED by production R2: the first real put succeeds, and the tail shows no `TypeError` from `onlyIf`.
       - If it throws, switch to the object form (`onlyIf: { etagDoesNotMatch: "*" }`), re-run the pool cell, and redeploy.
       - Record which form shipped. §6 of the close says the refusal itself was read in the pool only.
    2. **An abandoned game lands.** Through the credential, create a locked `.test` table, play three moves as its two claimed seats, and abandon it. It lands at `test/rev7/abandoned/<id>.json` and `.table.json`. Read both back through the account API:
       - the record loads in the CLI (`cargo run -p yokohama-cli -- <file>`) with exactly those moves;
       - the table has no `lock` and no `unlocked` (`strip_for_archive`);
       - `archived_at` is set.
       A FINISHED game's game-over put is read at the exit (Task 16), where a whole game is played.
    3. `/api/health` reads `archive: "r2"`.
  - Add **"to deploy with no archive"** (R-6.5c.14) to RUNBOOK §6, and record the deployment.

- [ ] **Step 8: Commit.**

```bash
git add worker deploy/RUNBOOK.md
git commit -m "feat(worker): the R2 archive as a receipt in DirArchive's order, game over completed by the layer's own stamp, and a clean off switch"
```

---

### Task 14: Expiry, the marker, and the operator surface (R-6.5c.15, R-6.5c.16; spec §2.4 "Delete, expiry and the MARKER", §2.9)

**Files:**
- Create: `worker/src/marker.ts`, `worker/src/marker.unit.test.ts`, `worker/src/admin.ts`, `worker/test/expiry.test.ts`, `worker/test/operator.test.ts`
- Modify: `worker/src/table.ts`, `worker/src/house.ts` (the listing), `worker/src/index.ts` (the admin gate), `worker/wrangler.jsonc` and `deploy/RUNBOOK.md` (Step 6)

**Interfaces:**
- Consumes: `expire` (Task 6), `deliverSlot` (Task 12), the receipt loop (Task 13).
- Produces:
  - `type Marker = { at: number; reason: "abandoned" | "expired" | "operator"; ttl_days?: number; kept: "archive" | "nothing" | "off" }` in `meta.marker`;
  - `markerSentence(m: Marker): string` — head then tail, UTC dates `YYYY-MM-DD`:
    - heads: `this table was abandoned on D` | `this table expired on D after N days without activity` | `this table was removed by an operator on D`;
    - tails: ` — its game is in the archive` | ` — no move had been made, so there was nothing to keep` | ` — this server keeps no archive, so its game was not kept`;
  - `Table.operate(email, action: "expire" | "force")`, reached only with `X-Yokohama-Operator`;
  - `House.listing()` — the operator payload: per copy `id, name, status, players, created_at, last_active, bytes, over, winner, archived_at`.

**The rules** (spec §2.4, §2.9 — read them in full; the cells below name each clause):
- **Activity** is the newest `written_at` of `record`, `table` and `chat`, so it costs no row of its own.
- **The alarm's three rules** (spec §2.4):
  - a write that makes a delivery due sets its retry;
  - the success that discharges it re-arms to the next deadline;
  - nothing else touches it.
  An alarm that fires before `last activity + TTL` re-reads the activity and re-arms to it. `held` removes the expiry deadline from the schedule.
- **Expiry** — at `now >= activity + TTL`, under `blockConcurrencyWhile`:
  - it runs `expire(id)` through Task 13's receipt loop;
  - a failed put leaves the table to retry;
  - a `conflict` writes `held`, stops arming the expiry deadline, and logs it;
  - with the archive off, it deletes without a save and logs any owed save it drops.
- **Every ending through the layer**, in ONE batch: the docs, the picker and `archive_owed` removed; the marker (`kept` computed as spec §2.4 step 2 defines it); the REMOVAL slot; the alarm.
- **While a marker stands**, every request answers **404** `{"error": markerSentence(marker)}`: an upgrade is answered 404 unupgraded, so the client's probe reads the sentence. A create is **409** (Task 12's sentence).
- **At `marker.at + 90 days`**, AND only once the removal is acknowledged, `deleteAll()`. `deleteAll` deletes the alarm at this compatibility date.
- **The operator's plain expire** runs the SAME path, with reason `operator`, and answers the layer's refusal if its save fails.
- **`?force`** ends the object whatever it holds — a table, a poisoned one, a `held` one, a marker — with no layer and no save:
  - the REMOVAL (unless the house acknowledged one), then `deleteAll()` once the house acknowledges;
  - NO marker;
  - it answers 204, awaiting `deliverSlot()` in the request, or 503 `{"error":"the lobby has not confirmed the removal — the alarm will finish it"}`;
  - on an object holding nothing, 204 and nothing written.
- **The gate** (in `handle`, before routing `admin`):
  - `YOKOHAMA_WEB_ADMINS` absent sends it to the house — the layer's ordinary 404;
  - a `dev` or `none` identity is **403** `{"error":"this address is not an operator here"}`, and so is a `person` not on the list — a dev-door identity NEVER counts, even when its claimed address is on the list (**battery 32**);
  - a listed person passes, with `X-Yokohama-Operator` set;
  - `GET /api/admin/tables` goes to the house's `listing()`, `POST /api/admin/tables/{id}/expire[?force]` to that table.

- [ ] **Step 1: The failing pure cells** (`marker.unit.test.ts`): all nine head-and-tail sentences, a UTC date across midnight, and `ttl_days` in the expired head.

- [ ] **Step 2: The failing pool cells** (`test/expiry.test.ts`, `test/operator.test.ts`). Time moves by BACKDATING stored times through `runInDurableObject` — `docs.written_at`, `marker.at` — then `runDurableObjectAlarm`. workerd's clock does not advance.
  - A write does not expire an active table: an early alarm re-reads and re-arms (**battery 30**: the mutants are `written_at` not advanced, and an alarm that expires without re-reading).
  - An idle table expires at the TTL, saved first, AS ITS CREATOR. An expiry's delete before its save lands is **battery 29**.
  - A failed save at expiry retries, and the table stays.
  - A `conflict` at expiry writes `held` and arms no expiry (**battery 39**).
  - Archive off expires without a save, dropping an owed save with a log line.
  - **The tail by `kept` under each head:**
    - a saved game;
    - a game stamped at game over, whose delete puts nothing;
    - an open table, a dealing one, and a playing one with no move;
    - archive off.
  - The marker's 404 on a read, on `…/events`, and on an upgrade (**battery 31**); a create under it is 409.
  - `deleteAll()` only after BOTH the acknowledgement and the 90 days (**battery 11**); with no TTL, no expiry.
  - **The operator cells:**
    - a non-operator is 403, and a credential-claimed operator address is 403;
    - with the variable absent, the layer's 404;
    - the listing includes a dealing table;
    - expire;
    - **force** on a poisoned, a `held`, a live and a marker-only object — each leaving no marker and an empty `sqlite_master`, and a create at the id then answering 201. The re-create's object runs with `tick:` set before its first request, so the new generation orders after the retired one; under the frozen clock the two tie and the house drops the new one about half the time (plan review m7);
    - force on an empty object is 204 with nothing written;
    - force with the house's push failing answers 503, a create meanwhile is 409, and the alarm finishes it.

- [ ] **Step 3: Watch them fail. Step 4: implement. Step 5: pass, and run every gate.**

- [ ] **Step 6: DEPLOY and read — the ORCHESTRATOR.**
  - **Ask the owner for the TTL value first** (an open owner item). Until they answer, deploy with the variable ABSENT: never expire.
  - Set `YOKOHAMA_WEB_ADMINS` with `npx wrangler secret put YOKOHAMA_WEB_ADMINS` (P-1).
  - **Readings, over WARP as the owner** (no credential):
    - `GET /api/admin/tables` lists a scratch `.test` table;
    - `POST …/expire?force` on it answers 204, and a create at its id then answers 201;
    - a second scratch table abandoned reads its marker sentence on `GET …/table`.
  - With the credential and the owner's address claimed, `GET /api/admin/tables` is 403.
  - Add **"to keep every table"** (the TTL unset) and **the operator playbook** to RUNBOOK §6, and record the deployment.

- [ ] **Step 7: Commit.**

```bash
git add worker deploy/RUNBOOK.md
git commit -m "feat(worker): tables expire as their creator's abandon, a 90-day marker that says why, and a person-only operator surface"
```

---

### Task 15: The live tiers — Playwright against the Worker, and the live corpus replay (R-6.5c.12; spec §5.3's live half, §5.4)

**Files:**
- Create: `web/e2e/live.ts`, `worker/scripts/live-replay.mjs`, `worker/scripts/live-replay.test.mjs`
- Modify: `web/playwright.config.ts`, `web/e2e/two_browsers.spec.ts`, `web/e2e/joins_by_link.spec.ts`

**Interfaces:**
- Consumes: the deployed Worker (Tasks 11–14), `.dev.vars` (never printed), and the three request scripts.
- Produces:
  - In `web/e2e/live.ts`:
    - `LIVE: boolean` — `process.env.YOKOHAMA_WEB_E2E_TARGET === "live"`;
    - `credential(): { id: string; secret: string }` — the lines `CF-Access-Client-Id=…` and `CF-Access-Client-Secret=…` of the repository root's `.dev.vars` (Global Constraints), read by path. When the file or a key is absent it throws a sentence that names the FILE and the KEY, and never a value;
    - `identityHeaders(email: string): Record<string, string>` — the claimed header, plus the two `CF-Access-Client-*` headers when `LIVE`.
  - In `live-replay.mjs`, exported pure functions `freezeClock(value)`, `filterLobby(name, committed, got, ids)`, `normalizeHouseChat(name, got, base, identities)` and `firstSendAccepts(committed, status, got)`, plus a CLI main.

**Playwright, live** (spec §5.4):
- With `LIVE`:
  - **the exported `BASE_URL` constant itself** (`playwright.config.ts:32`) is `https://yokohama.moore.nyc`, and so is the config's `baseURL`. `watchForBreakage` flags only responses whose URL starts with `BASE_URL` (`harness.ts:73`), and `two_browsers.spec.ts:292` requests `${BASE_URL}/api/…`. Changing only the option would leave the breakage watch watching nothing and send those requests to a refused localhost (plan review M9);
  - it has no `webServer`, `testMatch` is only the two specs, and `workers: 1`;
  - the two-browser game's budget (`two_browsers.spec.ts:102`, 20 minutes) is DOUBLED. Every request crosses the edge, and the local run already took 457 s. The ledger records the choice and the live wall (plan review m15).
- Each context takes `extraHTTPHeaders: identityHeaders(<its person>)` at `@e2e.test`. §0.4 measured context headers reaching a page-opened WebSocket upgrade.
- The service-worker declaration each spec carries (`every_spec_declares_its_worker`) is kept.
- Each spec abandons the tables it created, in a `finally`, as their creator (`page.request.delete`).
- **Locally, nothing about either spec changes.** `identityHeaders` without `LIVE` is the claimed header the specs already send.

**The live replay** (spec §5.3's live half, and the §9.3 house-chat correction below). Run from the owner's workstation:
1. **Force** `wire`, `wire2` and `wire4` with `POST /api/admin/tables/<id>/expire?force`, sent WITHOUT the credential, so WARP makes it the owner's assertion. It refuses to go on unless all three answer 204.
2. **Read the house chat's `last`** as a `.test` identity: that is `base`.
3. **Replay** `walk.json` with `walk-host.json` merged by `after`, then `archive-walk.json`, each row with the credential and `X-Yokohama-Claimed-User: <who>`. A row with no `who` sends no claimed header.
4. **Compare**, for every row its status, and for every named row its committed file after:
   - `freezeClock` — `created_at` and a cursor's time become `1_789_000_000`, as `wire_fixtures.rs`'s `freeze_clock` does;
   - `HOST_FIELDS` — `health`'s `identity`, `signal` and `archive`, and `me` / `me_signed_out`'s `mode` — each asserted to DIFFER from the committed value;
   - `filterLobby` — on `games*.json`, `games` is filtered to the replay's three ids, and `next` is compared only where the committed cursor names one of them;
   - `normalizeHouseChat` — on `house_chat_post.json` and `house_chat.json`, `seq` and `last` are taken back by `base`, and the page is filtered to messages with `seq > base` from the replay's own identities;
   - `firstSendAccepts` — on `archive_post.json` and `archive_post_abandoned.json`, the committed 201, or the same body at 200 with `"identical": true`.
5. **Force the three ids again.** Print a report of rows run, named rows compared, the exemptions spent and the mismatches, and exit non-zero on any mismatch.

- [ ] **Step 1: The failing pure cells** (`live-replay.test.mjs`, node), one per function:
  - `freezeClock` mirrors `freeze_clock`, including its refusal of a non-clock `created_at`;
  - `filterLobby` drops a foreign row, keeps a null `next`, and compares `games_next.json`'s cursor;
  - `normalizeHouseChat` takes `seq: base+1` back to `seq: 1`, and drops a foreign message;
  - `firstSendAccepts` accepts the committed 201 and the once-rule's 200, and refuses a 200 without `"identical": true` and a 409.

- [ ] **Step 2: Watch them fail. Step 3: implement the four functions and the main, then `live.ts` and the two specs' live branch.**

- [ ] **Step 4: The local e2e is unchanged** (sequenced, not beside another Playwright run):

```bash
cd web && npm run e2e
```

- [ ] **Step 5: RUN LIVE — the ORCHESTRATOR.**
  - **FIRST, the owner's zone decision** (spec §7, moved here by the plan's review, H3). Both live specs assert that no console error occurred (`two_browsers.spec.ts:479-481`, `joins_by_link.spec.ts:463-464`), and the zone's Web Analytics beacon logs a CORS error on every page load (spike row 8). Ask the owner:
    - to turn Web Analytics' auto-install off for this hostname; or
    - to accept it, in which case the LIVE branch allow-lists exactly that beacon's CORS error by its URL (`static.cloudflareinsights.com/beacon.min.js`), as a recorded host fact.
    JavaScript detections and Bot Fight Mode can wait for Task 16: they did not challenge the harness. Record the answer in the ledger BEFORE running a live game.

```bash
cd web && YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers joins_by_link
cd ../worker && node scripts/live-replay.mjs && node scripts/live-replay.mjs
```

  - **The replay runs TWICE.** The second run is the evidence that it is repeatable, which is the whole of the re-review's HIGH.
  - Record both runs' reports, the e2e's wall, and the moves the two-browser game played. Also record whether the socket arm reconnected — the page's own "reconnecting…" count, read as the M6.3 cell reads the stream count.

- [ ] **Step 6: Commit.**

```bash
git add web/e2e/live.ts web/playwright.config.ts web/e2e/two_browsers.spec.ts web/e2e/joins_by_link.spec.ts worker/scripts/live-replay.mjs worker/scripts/live-replay.test.mjs
git commit -m "test(live): the multiplayer specs and the corpus replayed against the Worker, repeatable by the operator's force"
```

---

### Task 16: The exit, the battery, the reviews, and the close (spec §5.5, §5.7; umbrella §3.3)

**Files:**
- Create: `docs/plans/<the day it is written>-web-frontend-m6.5c-close.md`, and the next cycle's entry artifact if the owner wants one (the cleanup cycle's).
- Modify: `CLAUDE.md`, `docs/handover.md`, `deploy/RUNBOOK.md`, `docs/plans/2026-09-22-web-frontend-m6.5c-objects-spec.md` (only the ledger's recorded amendments, as a dated §10, never an edit to a ruled sentence).

This task is the **close checklist**. Every box is a gate, and the close cites each one's evidence.

- [ ] **0. The whole-branch review.**
  - **Fable reviews the whole branch** against `2621fbe`, with the spec and this plan. Its brief is the lane's method: try the mechanism a sentence promises rather than re-reading the sentence. Ask for "escalations outside the brief".
  - **Then ONE fix wave, then a scoped re-review.** The fix wave's head is the reviewed head for everything below.
- [ ] **1. The mutation battery** (spec §5.7, **39 rows at `92592f2`** — enumerate from the spec AT EXECUTION, never from this count). Each mutation is applied at the fix-wave head, shown to fail by NAME, reverted, and mapped to its catcher.
  - Row 4 is RECORDED as uncatchable by the platform's guarantee.
  - A mutation that stays green is a FINDING that gets a cell, or a disposition the owner rules on.
  - Write the report to the ledger.
- [ ] **2. The owner's remaining zone decisions**, BEFORE the headed walk (spec §7): JavaScript detections and Bot Fight Mode. Web Analytics was decided before Task 15's live run, and the headed walk inherits that answer.
- [ ] **3. THE EXIT, in a window with NO deploy** (spec §5.5) — the ORCHESTRATOR. First deploy with `YOKOHAMA_WEB_COUNT_WAKES=on`; that deploy opens the window. Then:
  - **Two people play a whole game and a third watches, HEADED**: three Playwright contexts with `identityHeaders`, or the owner and two contexts. The table object is observed HIBERNATING between moves: `wakes` rises across pauses longer than the object's idle eviction.
  - **The move-to-other-screen wall**, measured as M6.3 measured it, with the cold path's CPU beside it off `wrangler tail`.
  - **A finished game and an abandoned one are in the bucket.** `rclone`-sync it, using an R2 key the owner creates, read-only — an owner item. Both load in the CLI, and `why` reconstructs a bot move: play one of the two against the bot for that.
  - **The whole corpus is byte-identical live**: Task 15's replay, run again.
  - **Requests AND rows written per game**, counted off the account's analytics (GraphQL: the Workers and Durable Objects invocation datasets, and SQLite rows written). This is R-6.5c.6's push trigger, read.
  - **The headed walk** owed across two releases (spec §7 item 2): the door and its controls at 390 px, and the five error strips.
  - Then deploy with `COUNT_WAKES` off.
- [ ] **4. The phones** — the owner's, recorded and not gated (spec §7 item 1).
- [ ] **5. The counts, in ONE sequence at the close HEAD**:
  - every per-edit gate of the Global Constraints, the new `worker/` commands and `cargo test -p yokohama-worker` included;
  - `npm run e2e`;
  - the release-tier roster if the owner wants it for the merge.
  Record the figures. Copy none from this plan.
- [ ] **6. The fences**, stated with their commands' output:
  - the fenced crates, the CLI and `yokohama-wasm` byte-identical to `2621fbe`;
  - `crates/yokohama-web/src` changed by exactly the two items;
  - the corpus byte-identical;
  - `SEMANTICS_REV` 7.
- [ ] **7. Write the close**, on the M6.5b2 close's shape:
  - what shipped, by task;
  - the counts;
  - the deploy records (RUNBOOK §6);
  - the reviews and the battery;
  - the rulings taken in execution with their costs — P-1 to P-3 and every ledger `Ruling:`;
  - the LEAVE list and the owner items;
  - **§7, what this cycle does NOT claim** — spec §6 is its starting point, corrected by what execution found.
- [ ] **8. The docs:**
  - `CLAUDE.md`'s gates gain the `worker/` commands and `cargo test -p yokohama-worker`, with their counts;
  - its "where things stand" paragraph records M6.5c;
  - `docs/handover.md` points at the close;
  - the spec gains a dated §10 carrying the ledger's amendments.
- [ ] **9. Push, and open the PR** against `web-room-m6.5b2`. The merge order is the owner's.

---

## Self-review

**Spec coverage.** Every ruling maps to a task:

| Ruling | Task(s) |
|---|---|
| R-6.5c.1 | 11 (the Worker built for the public door, deployed private) |
| .2 | 2–7 |
| .3 | 11 |
| .4 | 1 |
| .5 | 11's CPU reading, 16 |
| .6 | 8 and 10 |
| .7 | 4 and 9 |
| .8 | 5 and 12 |
| .9 | 6 and 13 |
| .10 | 7 and 11 |
| .11 | carried; the outbox is unchanged but for 8's branch |
| .12 | 7 (pool), 12 (corpus), 15 (live) |
| .13 | 11's re-reading |
| .14 | 2, 6, 8, 13 |
| .15 | 6, 14 |
| .16 | 14 |

Every §5 group has a home:

| Spec section | Task(s) |
|---|---|
| §5.1 | 1–6 |
| §5.2 | 7, 9–14 |
| §5.3 | 1 (the scripts), 12–13 (pool), 15 (live) |
| §5.4 | 15 |
| §5.5 | 16 |
| §5.6 | 7 (CI) |
| §5.7 | 16, with each row's catcher named at its task |

**Battery rows and their tasks:**

| Rows | Task |
|---|---|
| 1 | 9 |
| 2 | 11 |
| 3 | 5 |
| 4 | recorded |
| 5–6 | 12 |
| 7 | 12–13, 15 |
| 8 | 10 |
| 9 | 4, 9 |
| 10 | 13 |
| 11 | 14 |
| 12 | 7 |
| 13–14 | 7 (the deploy guard's exact `assets` block — the per-commit catcher, since the pool never reads the production config), and the live reading at each deploy |
| 15 | 11 |
| 16 | 9 |
| 17 | 13 |
| 18 | 12 |
| 19 | 5 |
| 20 | 13 |
| 21 | 4, 9 |
| 22 | 12 |
| 23 | 8 |
| 24 | 10 |
| 25 | 12 |
| 26 | 3 |
| 27 | 13 |
| 28 | 8 |
| 29–31 | 14 |
| 32 | 14 |
| 33 | 6 |
| 34 | 3 |
| 35 | 12 |
| 36 | 12 |
| 37–38 | 8 |
| 39 | 14 |

**Four things this plan deliberately does NOT do**, so an executor does not go looking for them:
- It moves nothing on the NAS (R-6.5.1; the cleanup cycle reconciles the two hosts).
- It publishes nothing and does not deploy the path-scoped Access door (R-6.5c.1).
- It builds no push transport (R-6.5b.11 stays deferred, with the exit's counts as its trigger).
- It gives the layer no logging seam (spec §6).

**Two places an executor will be tempted to widen scope, and must not:**
- The layer is two items. A third is a STOP, not a convenience.
- A payload that differs on this host is a FINDING. The answer is never a fixture regenerated to match.

**The riskiest task is 12**, because its failure mode is invisible: a lobby that is merely stale looks healthy. Its abort cell (battery 25) and the corpus replay are what can see it.

**Placeholder scan.** Cells whose arrangement is long are given as named arrangements in their bodies' comments. Each names every request, every premise and every assertion — the house style M6.5b2's Task 10 used — and each carries an instruction to write it in full. The first text of this plan left four bodies as a bare `/* … */`, battery 35's only catcher among them, while this paragraph claimed otherwise. The plan's review found them (M8), and the fix wave specified all four. Nothing says "add error handling", "similar to", or TBD.

**Type consistency.** The module's exports and the shell's names are defined ONCE, in §"Names shared across tasks", and each task's Interfaces block quotes them. Three of those names moved while this plan was drafted:
- `restore_picker` returns a `Reply`, because the wake's replay can pump;
- `identity` was added for the house's `/api/stream` admission;
- `house_frame` was added for the spec's crate struct.
All three were changed at the table, which is where an executor reads them.

**Review Focus.** Five lines, each with its cell in its owning task, marked "(Review Focus N)".


---

## Revision, 2026-09-23: what the plan's adversarial review changed

One Opus review read this plan against the spec and the code. It MEASURED rather than reasoned:
Task 1's code and Tasks 2–3's cells ran verbatim in a scratch copy of the repository, and a minimal
project on the pinned pool ran the platform APIs the plan leans on. It returned **0 CRITICAL / 4 HIGH /
10 MEDIUM / 16 MINOR**; its report is `.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/review-plan.md`.
Every finding was accepted, the orchestrator spot-checked the two that change an expected value
(`tables.rs:1507-1511`'s two bumps, `evict_for_test`), and one fix wave applied them all. The design
half is the spec's §9.4.

**The four HIGH:**
- **H1 — battery 25's catcher was vacuous a second time.** A method patched on a live instance makes the pool's RPC THROW (the resolver refuses an own property), so correct code and mutant both answered `true`. It is now the §"Pool facts" section, binding on every pool cell: prototype gates, restored before the alarm, and fresh stubs after an abort (measured `true`/`false`). Task 12's stale-ack cell uses revision-keyed gates for the same reason.
- **H2 — Task 1's three cells could not go green.** One did not compile: a local binding shadowed the `games()` helper. The other two failed against the right code, because `play_to_the_end` drives a second `Repl` and leaves the resident handle at the opening. They now evict it first — never inside `archive_over_retry`, which in production reads the real handle — and the eviction is also what makes the archive-off half non-vacuous.
- **H3 — the live Playwright tier would fail on the zone's analytics beacon.** Both specs assert that no console error occurred, and the beacon logs one on every page load. The Web Analytics decision now comes BEFORE Task 15's live run.
- **H4 — Task 2 asserted one bump for the finalizing keep.** The layer emits `bump_table` and then `bump`. The cell says so; Task 10 coalesces the frames to one per socket per reply, and the spec's §5.2 now says the same.

**The MEDIUM:**
- **M1:** Task 4 opened PlaceB, not PlaceA. The measured path is `1`, `1`, `A09`, `confirm`, and a bare `confirm` is a 200 that applies nothing, not a refusal.
- **M2:** two pool cells defeated themselves with live-instance patches an eviction discards. They now patch at module scope, and `COUNT_WAKES` is on in the test config.
- **M3:** two pool arrangements needed bytes only Rust can make. They are committed fixtures now: the owed pump, generated once, since its bot seed is entropy; and the owed game over, drift-checked. The pool reaches them as TEXT through a generated module.
- **M4:** game over can be reached by ANY reply — the wake's, a GET's, `events`' — not only a move's. P-2 now pins before every module call that can reach the clock, and any reply's plan becomes `archive_owed`.
- **M5:** batteries 13 and 14 had no per-commit catcher. The deploy guard now pins the production `assets` block exactly.
- **M6:** the value pin's partner claimed needles the stored document never holds.
- **M7:** the credential file's key NAMES are stated, with a header-file recipe that keeps the secret off argv.
- **M8:** four `/* … */` placeholders are now specified, battery 35's natural alternating-copy race among them, and the self-review's sentence about placeholders is corrected.
- **M9:** under `LIVE` the exported `BASE_URL` constant must move, or the breakage watch watches nothing.
- **M10:** pool storage is per FILE, so every cell takes its own ids.

**The MINOR** are applied where they fell:
- close reasons bounded at 123 bytes (a platform rule at this compatibility date), and any other admission status answered unupgraded;
- a persisting `rememberArchiveOff` for the outbox;
- a non-object create body refused (the layer accepts a positional array);
- the body read before the wake;
- one frame per reply;
- a ticking clock for any cell that re-creates an id;
- `markerSentence(m)`;
- the `workers-types` pin, and `npm install` before a lock exists;
- Task 8 sequenced;
- the `release-wasm` comment;
- the committed pair under its own id;
- `restore_picker` at a table with no session;
- a key-refresh reason out of `resolveIdentity`;
- the live game's budget;
- battery 2's cell forging all four internal headers.

Two simplifications were also taken: `ctx.id.name` replaces a stored `meta.id`, and the generation's time is the create's pinned reading.

**What the review verified TRUE, and this wave did not touch:**
- the whole Rust surface the plan names, compiled for both targets, and every symbol public as cited;
- Task 3's seven cells, passing verbatim;
- Task 5's listing equality — 25 listings byte-equal;
- the real glue's shape;
- the pool's `cloudflareTest` entry, `CompiledWasm`, `ASSETS`, WebSocket upgrades, hibernation with the attachment intact, `runDurableObjectAlarm`, `sql.databaseSize`, `ctx.waitUntil`, `blockConcurrencyWhile` around R2 I/O, and R2's `If-None-Match: *`.
