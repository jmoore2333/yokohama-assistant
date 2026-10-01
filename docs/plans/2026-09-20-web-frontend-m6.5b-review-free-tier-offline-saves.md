# M6.5b review — Free-tier hosting, offline play, and saves

Reviewed 2026-09-20 at `6bc137fa8046525de0a03debc6786a061b8b6575` on
`web-room-m6.5b`, at the owner's request before plan finalization. This is an
**advisory review**, not an amendment or an execution record. No implementation,
deployment, engine semantics, or bot behavior was changed.

References below use these short names and the line numbers at the reviewed HEAD:

- **Plan:** [M6.5b implementation plan](2026-09-20-web-frontend-m6.5b-plan.md).
- **Spec:** [M6.5b single-player spec](2026-09-20-web-frontend-m6.5b-single-player-spec.md).
- **Umbrella:** [M6.5 hosting spec](2026-09-19-web-frontend-m6.5-hosting-spec.md).

**Verdict: keep the architecture; revise the plan before execution.** Reusing
`api::dispatch`, keeping the engine and production bot unchanged, running local
play in a dedicated Worker, preserving record strings, and batching historical
frames are the right choices. The main risks are save lifecycle and host adapter
correctness. WASM compilation alone does not establish either durability or
Free-tier suitability.

## 1. Reconcile scope with the owner's current direction

**Free is the requested deployment target.** Umbrella R-6.5.12 (line 354), its
deployment section, and guidingpath's last decision row still say Workers Paid.
Record the owner's current Free-tier direction forward, amend the active
authority, and carry it into M6.5c. Do not let the next executor inherit Paid as
a settled constraint.

Cloudflare currently documents SQLite-backed Durable Objects on Free and a
default **30-second active CPU budget per DO invocation**. That supports the
owner's premise; it is not the CPU budget of the ordinary routing Worker.
That Worker still has **10 ms per HTTP request** on Free. Put gameplay,
replay, `suggest`, `why`, and archive verification in DO execution, including
the currently unmapped `/api/archive` route. Measure the actual deployment before
claiming it fits. [DO limits](https://developers.cloudflare.com/durable-objects/platform/limits/),
[Workers limits](https://developers.cloudflare.com/workers/platform/limits/).

**M6.5b does not currently deliver offline reopening.** Spec R-6.5b.2 and §8
(lines 140 and 748) explicitly defer the service worker and offline installation.
That is consistent with the written spec, but narrower than the owner's current
offline goal. Keep the separation if desired, but name and schedule the offline
delivery update as part of satisfying that goal; do not describe M6.5b's
API-independent play as finished offline support.

**A downloaded save is currently a CLI recovery path.** There is no browser
import/restore flow in this plan. IndexedDB supplies same-origin resume; a record
download does not yet restore the browser's local table, names, or seats.
Decide this explicitly before calling saves portable or backed up.

## 2. Corrections needed before execution

### R1 — P1: persist a dealing table even when no record exists

**Plan:1181–1183, Tasks 4 and 6.** `persist` returns when either document is
missing. At HEAD `Games::create` writes only a table; its record is created after
the last human keep. This is explicitly pinned by
`crates/yokohama-web/src/tables.rs:1791` ("no record until the last keep").

A new local game therefore receives a 201 without being stored in IndexedDB.
Reloading loses it. Acquiring a lock then calling `rebuild()` can discard it even
without reloading, because the rebuild replaces the entire in-memory store.

**Change:** persist the documents that actually exist, in one transaction:
table-only while dealing, table plus record after finalization, neither after
deletion. Do not manufacture an empty GameRecord. Acquire and retain ownership
through creation and the deal.

**Acceptance:** create → close/reopen before choosing a card; choose → reopen;
take over during dealing; all retain the exact setup and any acknowledged keep.

### R2 — P1: ownership must precede dispatch, including cold GETs

**Plan:1199–1204, 1272, 1519.** The lock is requested from `state?.id`, after a
`ClientState` has already been fetched. Dispatch happens before the ownership
check; a never-owning tab passes because `lost` is false, and GETs are exempt.
But `Games::live_handle` at `games.rs:389–415` may pump the bot and persist during
a GET. A supposedly read-only second tab can therefore write before acquiring
ownership. The dealing screen also has no playing `state` to supply that id.

The use of `steal` needs more than an `AbortError` handler. The Web Locks
specification explicitly warns that the old holder may continue executing after
its lock is stolen. JS notification timing cannot fence a running synchronous
WASM call or a queued IndexedDB write.
[Web Locks specification](https://w3c.github.io/web-locks/#dom-lockoptions-steal).

**Change:** derive ownership from the selected route/table id, acquire before
any potentially mutating dispatch, and enforce it in the Worker. Use an ownership
generation checked in the same IndexedDB transaction as the write; takeover
advances it transactionally. A stale generation cannot commit. Alternatively,
design a cooperative release protocol with an equally explicit forced-takeover
rule. Preserve the owner's Take over/Take back UX.

**Acceptance:** gate a cold GET and a mutation across A → B → A ownership
changes. Only the current generation commits; no old response changes the
screen. Test actual Workers and IndexedDB, not only the fake Worker.

### R3 — P1: serialize the Worker lifecycle and rebuild its persistence cache

**Plan:1164–1167, 1185, 1258–1267.** An async `onmessage` is not a serialized
actor: another message runs while an earlier handler awaits IndexedDB. A hold
can replace `Games` during another request's persistence; two holds can both
enter before `held.set`; release can arrive while acquisition is still pending
and then fail to release the subsequently acquired lock.

`lastWritten` also survives `rebuild()` unchanged. Example: A last wrote X; B
writes Y; A takes back and loads Y; an undo returns to X. A may skip writing X
because its old cache still says X, leaving Y on disk despite acknowledging X.

**Change:** use one explicit operation queue for dispatch/persist/rebuild and
generation-tagged acquisition/release. Do not hold that queue awaiting the
entire lifetime of a Web Lock. Cancel obsolete acquisitions on navigation.
Clear and reseed `lastWritten` from the committed snapshot on every rebuild.
Read the record/table pair for export as one coherent snapshot.

**Acceptance:** overlap read, write, takeover, export, and navigation; include
take-back followed by undo and reload. No mixed pair, stale overwrite, stranded
lock, or skipped persistence.

### R4 — P1: define one durable acknowledgement and storage-failure contract

**Spec:342–348 versus Plan:1204 and 1263.** The spec says persist *after* the
acknowledged reply; the sketch correctly awaits persistence *before* posting
the reply. Reconcile the authority in favor of commit-before-success. A module
reply is provisional; a successful page reply means its durable transaction has
completed. The claim that every stored state was necessarily seen is impossible
across a crash between commit and message delivery.

**Plan:1105–1130, 1264–1267.** Every exception is treated as a WASM trap. A quota
error therefore discards the provisional move and reports "reloaded", whereas
Spec §4 promises the move with `APPLIED BUT NOT SAVED`. The `writePair` test's
synchronous second-put exception also does not itself abort the transaction:
the Promise executor catches that exception, so the first queued write can
still commit. `deletePair` lacks an `onabort` rejection.

**Change:** explicitly abort on synchronous write setup failure, handle aborts,
and distinguish storage failure, module failure, and ordinary API refusal.
Choose either rollback-and-refuse, or retain an explicitly unsaved state with
raw export/retry available. Never label unsaved state saved or lose it silently.
Consider the standard strict durability hint where supported, without claiming
absolute protection against hardware loss.
[IndexedDB transactions](https://developer.mozilla.org/en-US/docs/Web/API/IDBTransaction).

**Acceptance:** hold transaction completion and assert no success reply;
inject quota error, explicit abort, and a synchronous second-write failure;
verify both documents and the promised recovery UI after reload.

### R5 — P1: recover from a trap with a fresh WASM instance

**Plan:918, 1264–1267.** `dispatch` holds the global `GAMES` mutex. A WASM
abort/trap does not unwind Rust and drop its guard. Calling the exported
`wasm.init()` on the same instance then attempts to acquire that still-held
mutex. The proposed recovery can itself trap repeatedly.

**Measured:** a temporary Rust module built with the pinned toolchain, the same
static `Mutex<Option<_>>` shape and `panic=abort` produced:

```text
initial init: passed
dispatch trap: unreachable
same-instance recovery: unreachable
fresh-instance recovery: passed
```

**Change:** recreate the Worker/WASM instance, settle all affected pending
requests, reacquire ownership, and reload committed storage. Announce recovery
only after it succeeds. Add page-side `error`/`messageerror` handling and a
bounded initialization failure path; a static module import failure occurs
before the worker can answer the message protocol.

**Acceptance:** an actual trap while holding the mutex, then continued play from
the last committed move; an invalid module URL gives a visible error rather
than a permanently pending local list.

### R6 — P1: an archive 409 is not proof that this complete save landed

**Plan:607–611, 1530; Spec §8's half-pair residual.** `DirArchive::put` writes
record first, table second. If the second write fails, retry finds the record
and returns `Exists`. The local Abandon handler treats every 409 as success and
deletes the only complete local pair. A different record at the same archive
path also produces that response; the readable caller prefix can collide.

This combines an explicitly accepted archive residual with a new destructive
client behavior. It deserves reconsideration before implementation.

**Change:** return an idempotent success/receipt only after verifying both stored
documents match the intended archive snapshot. Repair an incomplete identical
pair on retry; report a conflicting pair separately and keep the local game.
Use a commit marker or equivalent publication protocol for R2's two keys, and
an atomic no-clobber publication strategy for the directory implementation.
Do not delete based on a generic 409. Append a stable hash suffix to the readable
caller segment if unique attribution matters.

**Acceptance:** fail the second write, retry, and prove the complete pair exists
before local deletion; submit different bytes under the same id and prove the
local copy remains. Also state whether “once” means once per game or per leaf:
Task 2 currently deliberately accepts one id under both leaves.

### R7 — P1: prove the generated-module build on a clean checkout

**Plan:1018–1020, 1140, Tasks 3 and 7.** The Worker statically imports
`/wasm/yokohama_wasm.js` from Vite's public directory. A minimal production build
using the installed Vite and this exact Worker/import layout failed:

```text
[vite]: Rollup failed to resolve import "/wasm/yokohama_wasm.js" from "…/worker.js".
```

**Change:** choose a supported build graph: generate glue into a source/module
location Vite can bundle, or explicitly arrange a runtime URL import that
survives worker bundling. Type declarations must resolve before `tsc`; the
proposed build currently runs `tsc` before generating them. Use a dedicated
Worker TypeScript config rather than relying on adding WebWorker globals to the
window project.

Task 7 also needs these corrections:

- `src/**/*.test.ts` in the existing jsdom config already matches
  `src/local/corpus.node.test.ts`. Explicitly exclude the Node suite there;
  otherwise ordinary `npm test` and the unchanged web-client CI job require
  the supposedly separate generated module.
- The generated Node target must have a CommonJS package scope or compatible
  extension under `web/package.json`'s `"type": "module"`. Verify the actual
  generated output; `createRequire` alone does not change a file's module type.
- Native Rust tests deliberately reaching `JsValue::from_str` error paths
  cannot treat wasm-bindgen's non-WASM stubs as working JS values. Test a pure
  Rust helper natively or run ABI error tests in WASM. Isolate tests that share
  the global `GAMES` and clock to prevent parallel-test resets.

**Acceptance:** delete both generated output directories, then run the documented
build, typecheck, ordinary component tests, Node corpus replay, and production
browser load. Exercise the actual CI job separation.

### R8 — P1: keep the request script distinct from outbound leakage fixtures

**Plan:1548–1585.** `walk.json` contains every request body, including the
diagnostic setup seed and table passwords. But
`wire_fixtures.rs:1812–1830` scans every `corpus()` member for that seed; the
password and watcher fences also depend on corpus meaning outbound payloads.
Putting input traffic into that corpus without adapting the classification
breaks the existing fences. These are test credentials, not a discovered leak
of user data.

**Change:** explicitly distinguish test inputs from server responses, ideally
with separate manifests/directories. Keep leakage checks exhaustive over every
response fixture and retain their anti-vacuity controls. Bind fixture names to
specific recorded request sequence ids at capture time rather than later finding
the first equal JSON value. Include response status in replay comparisons.

**Acceptance:** regenerate and run all leakage tests; deliberately leak the
seed/password in a response and still fail. New input scripts must not weaken
response checking.

## 3. Offline play and saves: the minimum complete follow-up

Treat these as explicit product requirements for the offline update, not
features already delivered by this plan:

1. Cache a coherent release of HTML, JS/CSS, Worker glue, WASM, card art,
   manifest, and labels. A lazy WASM download must be completed before declaring
   “available offline.” Same-origin API/Access failures must not prevent local
   startup. Do not cache authenticated multiplayer responses as offline state.
   [Service worker lifecycle and caching](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API/Using_Service_Workers).
2. Use release/content-addressed asset paths and an update prompt. Install and
   verify a new cache fully before switching; avoid mixing old glue with new
   WASM or forcing a reload during a move. Cache eviction must never delete
   IndexedDB games. Keep the origin stable through the NAS-to-Workers move;
   a different hostname has different browser storage.
3. Request persistent storage when a player starts relying on local saves,
   inspect the grant, and expose storage failure and backup actions. IndexedDB
   is best-effort by default; browser eviction is distinct from the user clearing
   data. A grant is useful, but exports remain necessary.
   [Browser storage durability](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria).
4. Keep the existing raw `<id>.json` CLI export. Add a browser backup/restore
   format containing the raw record **as a string** and its table document,
   with envelope version/provenance. Validate in Rust, preserve every u64,
   handle id collisions explicitly, and never overwrite an existing save
   silently. Browser restore is currently missing, not implied by archive upload.
5. Keep incompatible saves listed and downloadable independently of a playable
   `ClientState`. Do not automatically migrate across `SEMANTICS_REV`, erase an
   unreadable row, or turn a storage-read error into “no games.” Resume on an old
   engine, if wanted, needs its own explicit compatibility policy.

Acceptance should include airplane mode → close all tabs/browser → reopen →
resume → finish → export, then an update and a restore on a fresh profile. Test
the owner's actual mobile browser, including suspension/backgrounding.

Two current plan details need attention even at level 2: the never-settling
fetch test only checks the local heading, and the labels fallback only runs on
rejection. A hanging `/api/labels` never falls back. Once local mode is selected,
get labels from that ready local host, or bound the server read. Also remove the
existing `App.tsx:503` sign-in guard for local routes, not just the four JSX door
returns. Test opening and reloading `#/t/local-…` while signed out and the API
never answers. A missing worker/IndexedDB error must be visible, not an empty list.

## 4. Free-tier optimization and the M6.5c handoff

**Budget the whole hosted path.** DO Free includes 100,000 requests/day,
13,000 GB-s/day, 5 million SQLite rows read/day, 100,000 rows written/day, and
5 GB total storage; exceeding a dimension makes its operations fail until the
daily reset where applicable. DO duration is wall time while non-hibernatable,
metered at 128 MB allocation. At 0.125 GB this is about **28.9 active object-hours
per day**, shared across the account (arithmetic, not a load forecast).
[DO pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/).

Consequently, hibernation and request/write amplification matter more than the
1.5 ms native bot-decision average:

- Keep hibernating WebSockets and edge auto-response heartbeats. Do not carry
  SSE or a periodic timer into DOs. Persist the multi-step picker beside the
  record, as umbrella H-4 anticipates; otherwise an ordinary thinking pause
  can force a player to restart it.
  [Hibernation behavior](https://developers.cloudflare.com/durable-objects/best-practices/websockets/).
- Serve shell/WASM/art through Static Assets without invoking the routing
  Worker for each asset. Scope `run_worker_first` to the necessary API routes
  while retaining the chosen Access protection. Static assets are free and
  unlimited; routing them through Worker execution consumes its quota and can
  make assets unavailable after quota exhaustion.
  [Static Assets billing](https://developers.cloudflare.com/workers/static-assets/billing-and-limitations/).
- Keep Task 1's batch route. Measure cold replay, whole bot turns, projection,
  serialization, `why`, archive strict replay, and storage separately on long
  saves, on the phone and in a live Free-tier DO. Record CPU separately from
  elapsed time, record bytes, WASM memory, and warm/cold cases. One short native
  determinism fixture is not a long-save performance result.
- Measure typical and long 2P records, and 3P/4P human tables for M6.5c. Keep
  the 2P WASM-bot determinism restriction. Do not change search budgets, RNG,
  or engine legality to meet a hosting budget.
- Keep the thin Worker below its own budget: no whole-save parse/stringify or
  strict replay there. Add a DO owner for browser archive validation; keep it
  out of the shared house object's critical path. Bound accepted upload size
  and event count from measured supported saves before expensive validation.
- The DO row limit is 2 MB; record actual maximum save bytes and define a
  recoverable refusal/chunking policy before that ceiling. Worker startup is a
  separate 1-second global-scope limit, not a one-second requirement for every
  replay. Current bundle limits are 64 MiB uncompressed; do not optimize
  against the retired 3 MB compressed Free limit.
  [DO limits](https://developers.cloudflare.com/durable-objects/platform/limits/),
  [Workers limits](https://developers.cloudflare.com/workers/platform/limits/).
- Update the house projection only when its visible fields change. A chat
  append or a picker-only step need not invalidate all lobby listings. Push
  viewer-scoped state/chat on table sockets with revision/sequence numbers,
  bounded queues, and reconnect hydration. Do not persist a separate copy of
  the same frame for every watcher.
- Avoid hydrating all historical records into every browser Worker on every
  acquisition as the library grows. Prefer a lightweight local catalog and
  loading the selected game's bytes. At minimum seed the write cache during
  load to avoid rewriting unchanged pairs. Keep exports opaque.
- Try a dedicated WASM release profile, LTO/strip, and measured `wasm-opt`
  variants only after correctness. Compare startup, size, peak memory, and
  decision latency; rerun corpus and native/WASM determinism pins. Smaller
  bytecode is not automatically faster. Free generated `Reply` handles
  deterministically after copying their fields where the ABI provides `free()`.

**Resolve the async archive seam before freezing M6.5c.** Task 2 retains
synchronous `Archive::put -> Result` called from synchronous `dispatch`, but
an R2 put is asynchronous. Define a prepare/upload/commit protocol with durable
pending work, retries, and completion receipts, or explicitly amend the layer's
async boundary. A queued upload is not a completed archive and cannot authorize
delete. Do not hold Rust mutexes or block the WASM thread across network I/O.
[R2 API](https://developers.cloudflare.com/r2/api/workers/workers-api-reference/).

The new browser crate's global `GAMES` is appropriate only for its per-tab WASM
instance. M6.5c must keep state owned by each Table object; importing the browser
singleton unchanged into a shared Worker isolate would defeat object isolation.

## 5. Execution and evidence notes for the finalizing agent

- Fix Task 1's cap test: it creates roughly one event then asks for `to=65`;
  the proposed handler checks out-of-range first and returns 404, while the
  test expects 400. Build a record with at least 64 events, or explicitly rule
  refusal precedence and test both defects separately.
- Several `cargo test` commands supply multiple positional filters. Cargo
  accepts one; use separate commands or a shared single filter.
- Add a real Worker/IndexedDB fault suite early in Task 4. A fake Worker tests
  page routing but cannot establish commit ordering, ownership, or trap recovery.
- Strengthen the no-network claim beyond `/api/games`: cover other play routes,
  Worker-originated requests, streams, and hanging requests after readiness.
  Distinguish initial asset download and optional archive upload from gameplay.
- Do not carry the plan's fixed Claude co-author/session trailers into commits
  made by another agent. Record the actual executor and ensure any required
  execution skills are available in that environment.
- Keep an explicit disposition per finding: corrected plan/spec, a bounded
  follow-up with an exit test, or an owner-accepted limitation. Do not treat
  the earlier reviews' counts as evidence these newly traced paths work.

Evidence in this review: source inspection of the spec/plan and existing
lifecycle, archive, API, test, and build code; current official platform/browser
documentation; isolated Vite-build and Rust/WASM-trap probes described above.
No application suite or live Cloudflare benchmark was run. An additional
browser IndexedDB probe was not executed because the sandbox refused its local
HTTP listener; the transaction finding is based on code and the documented
transaction semantics, not a claimed browser reproduction.
