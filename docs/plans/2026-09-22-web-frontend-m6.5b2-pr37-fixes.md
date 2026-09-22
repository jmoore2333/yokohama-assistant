# M6.5b2 — the PR #37 review's three defects: an export that can lose moves, an update that strands the other tab, and a retry the door never hears

**Status:** a fix wave on branch `web-room-m6.5b2` (PR #37, https://github.com/jmoore2333/yokohama-assistant/pull/37) after the M6.5b2 close (`docs/plans/2026-09-22-web-frontend-m6.5b2-close.md`, frozen — corrections are recorded FORWARD, here). Base HEAD `672c604`. The findings are the review `docs/plans/2026-09-22-web-frontend-m6.5b2-pr37-review.md` (Codex, at `672c604`); the owner's direction 2026-09-22 was that they "should be reviewed and likely addressed" before M6.5c. The rulings are §1, the design §2, the tasks §3 (the SDD shape), the gate and the battery §4, the record §5 (written at the close of the wave). This document is spec, plan and record in one because the wave is small — the 2026-09-06 owner-corrections precedent.

## 0. What was reviewed, and what was reproduced

**All three findings reproduce at `672c604`, re-run by the orchestrator on 2026-09-22 with the reviewer's own scripts** against the built `web/dist` (F1, F3) and the bundled `web/src/sw.ts` (F2). The scripts were written to `/tmp` by the review and are kept, unedited, in the wave's gitignored workspace `.superpowers/sdd/2026-09-22-web-frontend-m6.5b2-pr37-fixes/repro/`:

| Finding | Script | Reading at `672c604` |
|---|---|---|
| **F1 [P1]** a backup can omit committed moves | `yokohama-pr37-backup.cjs` | IndexedDB `diskEvents: 1`, the downloaded envelope `backupEvents: 0`, both `playing` |
| **F2 [P2]** an unreloaded tab cannot fetch its own release after another tab updates | `yokohama-pr37-sw.cjs` | both generations present, the old asset IN the retained cache, the page's fetch `TypeError: Failed to fetch` |
| **F3 [P2]** a successful "Try again" is never adopted | `yokohama-pr37-retry.cjs` | worker `activated`, the door still reading `Preparing to play offline…` |

**The root causes, read in the code:**

- **F1.** `backup.ts:213–222` (`pairOf`) takes `live?.record ?? stored.record` and `live?.table ?? stored.table`, where `live` is the Worker's `pair` answer (`worker.ts:530–541`) — `wasm.read` of THIS TAB's module store. A tab's store is loaded from IndexedDB when its Worker boots (`rebuild`) and is written back only for a game this tab HOLDS (`persist`'s `not-held` arm). **Any tab that does not hold a game can therefore hold a stale snapshot of it**, and a cold read in such a tab can even pump a bot move that nobody commits. The spec sentence behind the code (§2.5 lines 457–463: *"the control prefers the Worker's snapshot when the Worker answers and falls back to the database when it does not"*) states its own reason — *"a game with a move applied but not yet written"* — and that move exists only in the tab that holds the game. The field-by-field `??` can also pair one snapshot's record with another's table. **The same `pair` arm serves four more callers** — `recordText` (`api.ts:645`, "Save game file"), `sendToArchive` (`App.tsx:1643`), Abandon's `sendFirst` (`Abandon.tsx:114`) and the outbox's `attempt` (`outbox.ts:361`, which drains in whichever tab hears `online`) — so a stale tab can also SEND a stale record to the archive.
- **F2.** `skipWaiting()` is registration-wide: every client controlled by the old worker is handed to the new one and receives `controllerchange`. The page reloads only the tab that clicked (`swRegister.ts:209–213`), by design. The unreloaded tab is still running release N−1's bundle, but its fetches now reach release N's worker, whose `swRoute` knows only release N's list (`sw.ts:42`, `sw.ts:253–258`) and whose `fromCache` opens only release N's cache (`sw.ts:268–275`). A release N−1 hashed name is `network`: offline, or once the origin has dropped the file, it fails. **Spec §2.3 retains the previous generation for exactly this tab** (lines 337–339: *"because an open tab still asking for its own hashed module must not have it deleted out from under it"*), and `swInstall.ts:101–105` says the same — but nothing READS the retained generation, so the retention protected bytes no request could reach. The existing update cell (`offline.spec.ts:408`) could not see it: its releases differ only in the stable-named `manifest.webmanifest`, and it uses one page.
- **F3.** `swRegister.ts:276–278`: when the initial `register()` failed, `registration.current` is `null`, and "Try again" calls `register()` again and discards the registration it resolves to — never stored, no `statechange` watched, no `updatefound` wired. The door stays on "installing" until a reload, and a later update is never offered in that page.

**The review's "Testing to prioritize" items 2–5 are already carried by the M6.5c entry artifact** (`docs/plans/2026-09-22-web-frontend-m6.5c-handover.md`) and are NOT this wave's: the phones (§4 item 1), real browser lifecycle — the v1 tab blocking the migration (§6 item 8), the no-Web-Locks fallback and background suspension (§4 item 10) — the deployed UI and authentication (§4 items 2, 3, 5), and the M6.5c deployment spike (§4 items 8 and 11). Item 1, the regression cells for F1–F3, is §3 below.

## 1. Rulings (each with its cost if wrong)

- **R-F1 — an export takes ONE WHOLE PAIR from ONE source, and the Worker's memory is that source only where this tab's Worker HOLDS the game AT THE CURRENT OWNERSHIP GENERATION.** Otherwise the Worker answers the COMMITTED pair, read from IndexedDB in one transaction inside the same queued operation. The page takes the Worker's answer whole when the Worker answers inside the bound, and its own IndexedDB pair whole when it does not — never one document from each. **This AMENDS spec §2.5 lines 457–463 forward**: "prefers the Worker's snapshot when the Worker answers" becomes "prefers the snapshot of the Worker that owns the game". The generation check, not `held` alone, is what decides, because `held` is cleared by the steal's `AbortError` asynchronously and never at all under the no-Web-Locks fallback (`worker.ts:397–409`), while the generation is advanced inside IndexedDB by every acquisition. The rule lives in the Worker's `pair` arm, so all five callers take it — the two backups, Save game file, Send to the archive, Abandon and save, and the outbox. **The recovery property is kept, and a cell holds it**: the holding tab's applied-but-not-saved move is still exported (`plays_locally.spec.ts:430`'s quota cell, widened in §3). *Cost if wrong:* an export from the holding tab under a stolen lock reads the store rather than memory — which is the committed state the new owner is building on. **[Refined in execution — §5.4 R-F1′: a holder whose generation cannot be READ answers memory, and an unreadable store is a refusal, never a trap.]**
- **R-F2 — the worker answers a same-origin GET its own release does not list from a RETAINED generation of OURS when one holds the path, before the network; nothing is written at fetch time.** Rule 1 (`/api/*` and `/enter` never intercepted) stays FIRST; navigations still get THIS release's `index.html`; a path this release lists is still served from this release's cache; only a cache under `CACHE_PREFIX` is ever read (the origin's Cache Storage is shared, `swInstall.ts:107–109`). **This AMENDS spec §2.3 lines 322–326 forward** — rule 3, "Everything else goes to the network and is not cached", becomes "…goes to a retained generation that holds it, else the network, and is never cached" — and it is what makes lines 337–339 true. `swInstall.ts`'s retention doc is corrected in the same task: the old tab exists, controlled by the NEW worker. *Cost if wrong:* a path an older release precached and the current one dropped is served from the retained copy for one more release; the only paths a generation can hold are ones a precache listed, and the generator refuses `/api` and `/enter` (`precache.mjs`).
- **R-F5 — the sweep keeps the previously ACTIVATED generation, not the newest-INSTALLED one** (added 2026-09-22 from the F2 task review's escalation E1). `generationsToDelete` (`swInstall.ts:111–118`) keeps the other generation with the newest `createdAt` — the time it was INSTALLED. A waiting release that is REPLACED before anyone clicks (release N active with a game tab open, N+1 installed and waiting, N+2 installed later and replacing it, then a click) leaves N+1's cache behind with the newest stamp, so N+2's sweep keeps N+1 and deletes N — the generation the open tab is running — and R-F2's failure returns. So a worker stamps its OWN generation `activatedAt` when it activates, and the sweep keeps the other generation with the newest `activatedAt`; a generation with no `activatedAt` ranks below every one that has it, and among those the old `createdAt` order stands — which is what keeps the currently active `0.5.0-m6.5b2` generation, stamped before this rule existed, on this release's first activation. **This AMENDS nothing in the spec's words**: §2.3 lines 337–339 say "retains the PREVIOUS generation", and this is what "previous" has to mean for the reason those lines give. *Cost if wrong:* one more generation's bytes kept for one release in the transition case. **[Both this sentence and "AMENDS nothing in the spec's words" above are CORRECTED in §5.5.]**
- **R-F6 — `/cdn-cgi/` is a server path, never intercepted** (added 2026-09-22 from the F2 task review's note on files F2 did not touch). Rule 2 answers EVERY same-origin navigation with the cached shell. `/cdn-cgi/` is Cloudflare's reserved prefix on the app's OWN hostname — the documentation places `/cdn-cgi/access/get-identity` and `/cdn-cgi/access/logout` on the protected domain, and Access's return from the identity provider is a navigation to `/cdn-cgi/access/authorized`, the hop that sets `CF_Authorization` there. A controlled page answering that hop with the shell would swallow the login: the cookie is never set and `/enter` loops. Under the NAS's hostname-wide application that is reachable on any session expiry in a browser the worker controls. `isServerPath` gains `/cdn-cgi` and everything under it; the app serves nothing there, so the bypass costs nothing. **This AMENDS spec §2.3 rule 1 (line 324) forward** — "`/api/*` and `/enter`" becomes "`/api/*`, `/enter` and `/cdn-cgi/*`". *Cost if wrong:* none — a bypassed path is the request the browser would have made with no worker installed.
- **R-F3 — a successful retry adopts its registration exactly as the initial success does**: stored, its installing and waiting workers watched, `updatefound` wired, settled, and every listener removed at unmount — one wiring, reached from both paths, rather than a second copy. No spec sentence changes; §2.3's *"a hang is a FAILED install with a visible line and a retry"* is what this makes true. *Cost if wrong:* none beyond the hook.
- **R-F4 — process.** SDD for all three tasks with Opus the floor for implementers and reviewers (the owner's PF-1 of 2026-09-21, `docs/plans/2026-09-22-web-frontend-m6.5b2-close.md` §5), run in the order F1 → F2 → F3 in ONE tree with one Playwright run at a time (R-F5 and R-F6 join Task F2's fix round, and F3 carries two comment corrections the F2 review found in files F2 did not touch); a whole-branch Fable review, one fix wave if it finds anything, a mutation battery mapped to named catchers, the per-edit suites counted at the wave's head, and the reviewer's three scripts re-run against the fixed build as the browser-level reading. **The point release `0.5.0-m6.5b2.1`, its deploy and the push to PR #37 are the owner's call** and are asked at the end, not assumed. *Cost if wrong:* wall-clock.

## 2. Design

**F1.** A pure module the Worker IMPORTS decides the source (`swRoute.ts`'s reason: a tested model beside an untested copy is two implementations, and the tested one would not be the one that runs) — given the held generation (or none) and the stored generation, it answers memory or store. `worker.ts`'s `pair` arm reads the stored generation with `readGeneration` (`db.ts:248`) inside its queued operation; on "memory" it answers `wasm.read` of both documents as today (and falls to the store where the module has no table for the id); on "store" it answers `readPair(db, id)` (`db.ts:379`, one transaction). `backup.ts`'s `pairOf` becomes `live ?? stored`, whole. The shared bound, the module-dead fallback and the `table === null` rules are unchanged.

**F2.** For a request `swRoute` classifies `network`, `sw.ts` answers from the first retained generation holding the path — every cache named under `CACHE_PREFIX` other than this release's own, looked up by exact path — and from `fetch(request)` where none does. `swRoute` stays pure and gains no cache knowledge; if the implementer gives the new branch its own `Route` value, `swRoute.ts`'s doc and `swRoute.test.ts` move with it, and the header comment that says `network` is "answered by NOT answering" is corrected either way.

**F3.** The wiring that follows a successful `register()` becomes one function the effect installs and the retry reaches (a ref set by the effect and cleared at its cleanup is the obvious shape); the retry's `register()` resolves into it, guarded by the effect's `live`.

## 3. Tasks

Each task is TDD (a failing cell first, written against the defect as reproduced), commits with explicit paths, and runs the per-edit gates its files touch (§4). Each task's reviewer checks the brief's cells exist, fail on the defect, and that the claims in the task's own comments are true.

- **Task F1 — the export snapshot.** Files: a new pure module under `web/src/local/` (+ its vitest file), `worker.ts`, `backup.ts` (+ `backup.test.ts`), `e2e/plays_locally.spec.ts` (and/or `offline.spec.ts`). Cells:
  1. **Unit, the chooser:** held at the current generation → memory; held at an OLDER generation (stolen, and the no-locks case) → store; not held → store.
  2. **Unit, `backup.ts`:** the Worker's pair is taken WHOLE — a Worker answering `{record: null, table: T′}` beside a stored `{R, T}` exports `{null, T′}`, never `{R, T′}`; the Worker silent → the stored pair whole; the existing "prefers the Worker's snapshot" cell kept.
  3. **E2E, the reviewer's scenario:** a local game EXISTS and has moves before tab B's Worker boots on the door (so B's store really holds a snapshot of it — the review's precondition, which the existing two-tab cell does not have, because its B boots before A's game exists); A plays on; B's "Back up everything" carries IndexedDB's record for that game BYTE FOR BYTE with more events than when B booted; that file restored in a FRESH context lands the same event count.
  4. **E2E, ownership takeover:** B takes the game over and commits ≥1 move; A, read-only, exports the game (its per-game backup and/or Save game file) and gets B's committed record, not A's memory.
  5. **E2E, unsaved-state recovery:** the quota cell (`plays_locally.spec.ts:430`) additionally asserts that the holding tab's per-game BACKUP envelope carries the move the store refused — the constraint that the fix must not simply ignore memory.
- **Task F2 — the retained generation.** Files: `sw.ts`, possibly `swRoute.ts` (+ `swRoute.test.ts`), `swInstall.ts` (the doc), `sw.node.test.ts`, `e2e/offline.spec.ts`. Cells:
  1. **Node, the real bundled worker (`sw.node.test.ts`):** a path this release does not list but a retained generation holds is answered from it with NO network fetch; a path no generation holds goes to the network; a cache NOT under `CACHE_PREFIX` holding the path is never read; `/api/…` and `/enter` are not answered even when a retained generation is planted with them; a navigation is still this release's `index.html`; nothing is `put` at fetch time.
  2. **E2E, the platform behaviour the original design misread:** two tabs controlled by release 1; release 2 shipped with a GENUINELY RENAMED hashed asset — the local Worker chunk renamed on disk, its one reference patched, `precache.mjs` re-run — so release 1's name is gone from the origin AND from release 2's list; tab A (the door) takes the update and reloads onto release 2; tab B, never reloaded, is asserted to be controlled by release 2's worker; B's HTTP cache is cleared (the chunk was served `immutable` and a warm HTTP cache would mask the defect — Cache Storage must be asserted intact after the clear) and B goes offline; B's local host is restarted (the `debug_trap` path `plays_locally.spec.ts:482` uses) and a move COMMITS. `dist` is put back byte for byte and PROVEN inside the `finally` (`offline.spec.ts:408`'s pattern and its reason).
- **Task F3 — the adopted retry.** Files: `swRegister.ts`, `swRegister.test.ts`. Cells (fake `SwHost`): the initial `register()` rejects → `failed` with its reason; "Try again" → `register()` resolves → the installing worker's `statechange` through `activated` reaches `ready`; a later `updatefound` with a waiting worker sets `updateWaiting`, and `applyUpdate` posts `skip-waiting` to it (which proves the registration was stored, not merely watched); unmount removes every listener the retry path attached. An e2e is NOT required: whether Playwright can answer the worker SCRIPT's own fetch with a 503 is a platform question, and the reviewer's script is the browser-level reading (§4).

## 4. The gate, the battery and the readings

**Per-edit, per task** (CLAUDE.md's web browser half): `npm run typecheck`, `npm test`, and — for F2 — `npm run test:wasm` (it holds `sw.node.test.ts`); the task's own e2e spec file(s) with `npm run e2e -- <file>` or the equivalent. `npm run build:wasm` first where the typecheck needs the glue. The Rust side is untouched by all three tasks and needs no run unless a task touches it.

**At the wave's head, once:** `npm run typecheck`, `npm test`, `npm run test:wasm`, the WHOLE `npm run e2e` in one run (the lesson M6.5b recorded: the e2e is owed by any change to what a browser DOES), `cargo test -p yokohama-web` and `cargo clippy --workspace --all-targets -- -D warnings` as the no-regression readings for a branch whose Rust did not move, and the three repro scripts re-run against the rebuilt `dist` — each must read the FIXED outcome (F1 `diskEvents == backupEvents`; F2 the old asset's body from the unreloaded tab; F3 the door out of "Preparing…"). The F2 script bundles `src/sw.ts` itself, so it reads the fix directly.

**The battery**, at the reviewed head, each mutation reverted after its run: (F1) the chooser answers memory whenever `held` has the id, ignoring the generation; `pairOf` restored to field-by-field `??`; the Worker's `pair` arm restored to memory always; (F2) the retained lookup reads ANY cache (drop the prefix filter); the retained lookup placed before rule 1; the retained lookup removed; (R-F5) the sweep ranks by `createdAt` alone again; (R-F6) `/cdn-cgi` dropped from the server-path rule; (F3) the retry discards the resolved registration again; the retry stores it but wires no `updatefound`. Each must be CAUGHT by a named cell; a survivor is recorded with its disposition.

## 5. Record (written at the close of the wave, 2026-09-22)

### 5.1 What landed

Eighteen commits over `672c604`, ending `13dc65f`, on `web-room-m6.5b2`. They cover 29 files and **no Rust**: `git diff --stat 672c604..13dc65f -- crates/ Cargo.toml Cargo.lock` is empty. So the five fenced crates, the CLI, the web server crate and the wasm crate are byte-identical, `SEMANTICS_REV` is 7, the wire corpus under `web/src/wire/` is untouched, and no bot behaviour moved. **Nothing was pushed or deployed** (§5.8).

| Task | Commits | What it did |
|---|---|---|
| (docs) | `a4d7e4e`, `d8bc2d3` | the review committed and this document; R-F5 and R-F6 added mid-wave |
| F1 | `984dfee`, then fix rounds `c213f67`, `b9b75c4`, `1d5ee71`, `84430eb` | R-F1: a pure chooser `web/src/local/exportPair.ts` that the Worker imports; the `pair` arm answers memory only for the owner at the current generation, else the committed pair; `pairOf` takes one pair whole; an unreadable store is a typed refusal (`UnreadableReply`, one compile-checked spelling) rather than a trap; rule 6 of `worker.ts` scoped to what is true; Send to the archive no longer calls a storage failure "the archive could not be reached"; Save game file and Abandon show the sentence bare |
| F2 | `134fae1`, then `60a0026`, `4666180`, and the comment round `0a4f722` | R-F2: `fromRetained` answers an unlisted path from a retained generation of OURS (`retainedGenerations`, pure); R-F5: an activation stamp and `byRetention`, with a same-generation re-install keeping its stamp; R-F6: `/cdn-cgi` bypassed; `only-if-cached` moved into `swRoute`; `sw.ts`'s header lists every decision it makes, pinned and unpinned |
| F3 | `506073a`, then `11a7c39`, `6c443a2` | R-F3: ONE wiring (`adoptRegistration`, lent through a ref) reached by both the first registration and the retry. E1 (escalated by the review, MEASURED before it was changed): a failed FIRST install removes the registration and `update()` is refused, so the retry now registers anew. E2: a tab the old worker did not control reloads when the worker it asked reaches `activated`. The door's double period. The two misread comments in `swRegister.ts` and `FrontDoor.tsx` |
| final fix wave | `008c6c4`, `48695c3`, `a8afbdf`, and the orchestrator's `13dc65f` | R-orch-5 (below); the retry's refused `register()` pinned; Abandon's bare sentence; three wording fixes and a test-message remedy |

### 5.2 The readings (at `a8afbdf`, one uninterrupted sequence, 712 s, this Mac)

`13dc65f` is a comment-only change to one test file, re-run as 29/29 plus typecheck.

| Gate | Reading | Was, at `672c604` |
|---|---|---|
| `npm run typecheck` (four projects) | clean | clean |
| `npm test` | **1024 passed in 55 files** | 988 in 54 |
| `npm run test:wasm` | **32 passed in 4 files** | 17 in 4 |
| `npm run e2e`, whole, one run | **61 passed in 11 files, 9.5 min, no retry** (`plays_locally` 12, `offline` 9) | 53 in 11 (`plays_locally` 7, `offline` 6) |
| `cargo test -p yokohama-web` | **289** (275 lib + 2 `browser_played_pin` + 2 whole-game gate, both arms in 26.4 s + 10 `wire_fixtures`) | 289 |
| `cargo test -p yokohama-wasm` | 5 | 5 |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean. This is a CACHED reading, because no Rust changed | clean |
| `npm run build` (release) | 153 entries, 9,936,972 B; module 1,175,223 B; `debug_trap` 0 | 153 entries |
| The reviewer's three scripts (§0) against the release `dist` | F1 `diskEvents: 1, backupEvents: 1`; F2 `fetchFromUnreloadedTab: "release 1"`; F3 the door reads "Ready to play offline." | 1/0; `TypeError: Failed to fetch`; "Preparing to play offline…" |

### 5.3 The battery (§4's rows, widened, run at `a8afbdf`)

**16 of 16 CAUGHT, 0 survived.**

| # | Mutation | Catcher(s) |
|---|---|---|
| 1 | the chooser ignores the generation | `exportPair.test.ts:73` and the no-Web-Locks e2e cell `plays_locally.spec.ts:1064` |
| 2 | `pairOf` completes one document from each source | `backup.test.ts:160` |
| 3 | the `pair` arm answers memory always | `plays_locally.spec.ts:898` and `:971` |
| 4 | an unreadable store traps | `plays_locally.spec.ts:629` |
| 5 | the refusal catch drops `instanceof` | the trap cell's export extension, `:693` |
| 6 | the retained lookup removed | `sw.node.test.ts` (a) and `offline.spec.ts` cell 7 |
| 7 | the prefix filter dropped | node (c) and `swInstall.test.ts:191` |
| 8 | the lookup before rule 1 | node (d) and three more cells |
| 9 | the sweep by install time | `swInstall.test.ts:76` and `:98`, node `:417`, and the e2e update cell (4→5) |
| 10 | `/cdn-cgi` dropped | `swRoute.test.ts:76`, node `:543`, `precache.test.ts:174` |
| 11 | the retry discards its registration | three `swRegister` cells and `offline.spec.ts` cell 8 |
| 12 | the retry wires no `updatefound` | `swRegister.test.ts:674` |
| 13 | always `update()` | `swRegister.test.ts:249` and cell 9 |
| 14 | no `activated` path for an uncontrolled tab | `swRegister.test.ts:477` |
| 15 | the retry's `.catch(said)` swallowed | `swRegister.test.ts:713` |
| 16 | `installing` restored to the retry's condition | `swRegister.test.ts:288` |

**Two facts the battery found about its own instruments.**

- The unit cell `client.test.ts:409` is NOT a catcher for row 4. It drives the fake Worker and reads 21/21 under the mutation. So **the real Worker's trap/refusal split (rows 4 and 5) is pinned by the e2e alone.**
- F1's first report said no browser cell caught row 1. That stopped being true once the no-Web-Locks cell existed: it catches row 1 as well.

Every implementer also mutation-checked its own cells at its own head. Those readings are in the task reports, and one of them SURVIVED a first draft: F2's cell (c) before it gained a retained generation beside the foreign cache.

### 5.4 The reviews, and the rulings taken during execution

**Reviews.**

- **F1:** CHANGES REQUIRED (1 Important). Then two re-reviews: APPROVED WITH MINORS, then APPROVED.
- **F2:** APPROVED WITH MINORS. Plus E1, a defect outside its brief, and a note about Access. Two re-reviews, both APPROVED WITH MINORS, then a comment-only round.
- **F3:** APPROVED WITH MINORS. Plus two escalations, E1 (Important) and E2. One re-review: APPROVED WITH MINORS.
- **Whole-branch review (Fable):** **SHIP**, 0 Critical / 0 Important / 5 Minor / 4 new false-or-stale sentences.
- **Scoped re-review of the final fix wave (Fable):** **APPROVED**, one nit (`13dc65f`).

Every report is a file in the wave's workspace, beside its ledger `progress.md`.

**R-F1′ — the refinement of R-F1** (F1 review I-1). The `pair` arm now awaits IndexedDB, and the first build posted a failed READ there as a module TRAP. The page restarted the Worker and lost the very unsaved move the export exists to rescue. So:

- a holder whose generation cannot be read answers MEMORY: the store is unreadable, and this tab is the last known owner;
- a committed read that fails is an ordinary `{unreadable}` reply, which the five callers each name as the storage sentence;
- a module fault still traps.

The storage reads that STILL trap are named in `worker.ts` rule 6 and not fixed: a hold's acquisition, startup, and `adopt`.

**R-F5 and R-F6** were added to §1 mid-wave (`d8bc2d3`), from the F2 review.

- **R-F5.** The activate sweep kept the newest-INSTALLED other generation. So a waiting release replaced before any click deleted the generation an open tab was running, and R-F2's failure came back.
- **R-F6.** The worker answered Access's `/cdn-cgi/access/authorized` NAVIGATION with the cached shell. That hop is the one that sets `CF_Authorization` on the app's hostname, and the Cloudflare documentation places `/cdn-cgi/access/*` on the protected domain. Under the NAS's hostname-wide application this was reachable on any session expiry in a controlled browser. It is **reasoned, not measured live** (§5.7).

**The orchestrator's rulings**, each in the ledger with its reason and cost:

- **R-orch-1.** A failed ACTIVATION STAMP is documented, not hardened against. The guard that would cover it would also keep every replaced waiting release (about 10 MB) for a release, to cover a ~100-byte write failing right after a 10 MB install succeeded.
- **R-orch-2.** The browser cell for the refused `/sw.js` and the door's double period were added. They had been offered as optional.
- **R-orch-3.** The six `sw.ts` rules the comment round found UNPINNED are carried, not pinned in this wave (§5.7).
- **R-orch-4.** E1 and E2 were folded into F3 rather than recorded forward, E1 only after a browser measurement confirmed the spec reading.
- **R-orch-5.** The retry takes `update()` only when the registration has a WAITING or ACTIVE worker. In the milliseconds between the worker's `install-failed` message and the registration's removal, `installing` still names the dying worker. A `register()` there queues behind it and registers anew, and against an install really in progress it resolves with the same registration (Register's shortcut).

**Deviations the ledger accepted:**

- one commit per task, rather than a failing-first commit followed by a fix — the red readings are in the reports instead, which departs from the repository's strictly-TDD history and is said here out loud;
- the fake Worker's default `pair` is the committed pair;
- a same-generation re-install keeps `activatedAt`;
- two extra guard cells: `alreadyThere`, and the first registration's after-unmount `live`.

### 5.5 Corrections to §0–§4 (recorded here; the text above is left as ruled)

- **R-F5 DOES amend the spec's words.** Spec §2.3 line 338 reads "deletes only generations older than that". R-F5 deletes a never-activated generation that is NEWER by install time. So line 338 is amended forward too: "older" is now read in ACTIVATION order.
- **R-F5's cost sentence described the wrong case.** The residual is this: a pre-R-F5 build that was WAITING and was REPLACED before this release first activates leaves both candidates unstamped. The first activation then sweeps by install time and can delete the generation the open tabs run. That is the original defect, once, in that transition. A second residual is the same defect for one release on a device whose activation stamp could not be written (R-orch-1).
- **R-F1 is refined** (§5.4 R-F1′).
- **§0–§3's file:line references are at the base HEAD `672c604`**, not at the close. At the close the quota cell is `plays_locally.spec.ts:465` and the `!hasLocks()` arm is `worker.ts:407–419`. The refusal of `/api`, `/enter` and now `/cdn-cgi` in a release lives in `precache.ts`'s `buildPrecache`, through `isServerPath`, and not in `precache.mjs`.
- **§1 R-F2's "the only paths a generation can hold are ones a precache listed"** holds for served paths only. Every generation also holds its stamp, `META_PATH`, which `fromRetained` never serves; a cell pins that.

### 5.6 The platform, as this wave found it

The pre-fix comments misread the platform in two ways.

1. **The handover.** `skipWaiting()` hands EVERY client the old worker controlled to the new worker. Not calling `clients.claim()` changes only pages NO worker controlled. Spec §2.3 knew this (lines 333–335); what it lacked was a fetch rule that reads the retained generation. The misreading "not claiming keeps the old tab on the old worker" was in the CODE comments.
2. **The update after a failed first install.** The platform removes a registration whose FIRST install failed, and `update()` on the page's dead copy is refused. This was measured in Chromium through Playwright 1.62.1: the door read "…The object is in an invalid state" and `/sw.js` was never fetched again.

Two harness facts were measured and are worth keeping:

- **A warm HTTP cache hides F2's defect.** The release-1 install writes the chunk into the HTTP cache (`cache: "reload"`), and the chunk is `immutable`. Cell 7 passed against the pre-fix worker until CDP's `Network.clearBrowserCache` was added.
- **`context.route` reaches the worker's own fetches**, both the registration's `/sw.js` and the precache entries. That is what made cells 8 and 9 cheap.

### 5.7 What this wave does NOT claim, and what it carries to M6.5c

- **E2 is pinned by unit cells only.** No browser measured an uncontrolled tab's click.
- **R-F6 is reasoned from Cloudflare's documentation and the Service Worker specification, and was not measured live.** The live check is one login after a session expiry in a browser the worker controls. It belongs with the M6.5c handover's §4 items 3 and 5, the clean-browser reading and `cf-access`.
- **The real Worker's trap/refusal split (§5.3 rows 4 and 5) has no unit or node catcher.** Only the e2e pins it.
- **`Abandon.tsx:212` (`abandonAnyway`'s catch) has no catcher.** The same mutation there leaves every cell green.
- **Six `sw.ts` decisions have no catcher** (R-orch-3; the header's "unpinned" list names them):
  - the whole-install deadline;
  - the per-fetch timeout;
  - `credentials: "same-origin"`;
  - the stamp written first;
  - `includeUncontrolled`;
  - the "took too long" wording.

  The claim "the other lanes are ABORTED" is satisfied in its cell by the per-fetch timeout fifteen seconds later, not by the abort. The smallest instrument is a fake-timer cell asserting the abort lands BEFORE `FETCH_TIMEOUT_MS`.
- **Storage reads that fail outside the export arm still trap:** a hold's acquisition, startup, and `adopt`.
- **`backupEverything` iterates the STORE's ids.** A game whose create was refused by quota is listed on the door but absent from "Back up everything", while "Back up" on the game exports it from the holder's memory.
- **The two transition residuals of §5.5.**
- **Nothing on a phone was measured.** Nothing was deployed; nothing was pushed.

These join the M6.5c entry artifact's §4, `docs/plans/2026-09-22-web-frontend-m6.5c-handover.md`. That document is not edited: this section is the forward record, and the board (`docs/handover.md`) points at it.

### 5.8 Owner items

1. **The point release.** Build and deploy `yokohama-web:0.5.0-m6.5b2.1` from `13dc65f` to the NAS by `deploy/RUNBOOK.md` §1–§2, recorded as a §3.16 deployment record. The live `0.5.0-m6.5b2` carries all three defects.
   - All three fixes are client-side. A player gets them once they take the update offer, or once every tab on the old release has closed.
   - The first activation over the live generation keeps that generation (§5.5).
2. **The push.** The wave's eighteen commits and this record's commit are local; PR #37 does not have them yet.
3. **The E2 and R-F6 browser readings**, carried as §5.7 says.
