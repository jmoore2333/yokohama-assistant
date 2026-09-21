# M6.5b2 "the front door, installable and offline" — execution handover

**Written 2026-09-21 at the close of the brainstorm, the spec and the plan, for the FRESH SESSION
that EXECUTES M6.5b2.** Branch `web-room-m6.5b2` exists and carries four commits, all documentation;
no source has moved. Everything below is either a pointer or a measurement — no decision here is
open, and where this document and the spec disagree, **the spec wins**.

---

## 0. Read order (the session's first fifteen minutes)

1. **`docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md`** — the authority. §1 (the eighteen
   owner rulings), §2 (the mechanism), **§10 (what the pre-implementation review changed)**. §0.3 is
   what was MEASURED; do not re-derive any of it.
2. **`docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md`** — eleven tasks. Its **Global Constraints**
   and **Revision** sections are the two to read whole.
3. `docs/plans/2026-09-21-web-frontend-m6.5b2-review-feedback.md` — the review that corrected both,
   committed beside them. Read it for its METHOD, not for open items: every finding is already
   folded in.
4. `docs/plans/2026-09-21-web-frontend-m6.5b-close.md` §5, §6.3, §7 — what HEAD is, its owner items
   (this cycle closes two) and what M6.5b does not claim.
5. `CLAUDE.md` — the operating facts and the gate commands. **Re-count every suite at your own HEAD.**

---

## 1. Where things stand

- **Branch `web-room-m6.5b2`**, stacked on `web-room-m6.5b`, **base for every diff `369a1f2`**.
  Four commits, all under `docs/plans/`. `git status` clean.
- **Nothing is deployed for this cycle.** The NAS runs `yokohama-web:0.5.0-m6.5b.1` on
  `yokohama.moore.nyc`, which this cycle MOVES to `yokohama-nas.moore.nyc` at Task 4.
- **Six pull requests are open and nothing is merged**; merge order is the owner's. This branch's
  PR is not open yet — Task 11 opens it against `web-room-m6.5b`.
- **No users and no games of consequence anywhere** (owner, this session): in-person testing only,
  and the Worker has never been deployed. That is what makes the hostname move free.

---

## 2. What the owner ruled, and the two things NOT ruled

Eighteen rulings are in the spec's §1 with the owner's own words. The five that most shape execution:

- **`claimed` stays on the NAS**, and becomes a build feature a public build can compile out.
- **Multiplayer OFF means the routes are not served**, which is the answer to gating rather than a
  policy over them. It does **not** take `/api/archive` with it.
- **`yokohama.moore.nyc` is reserved for the Worker; the NAS moves to `yokohama-nas.moore.nyc`.**
- **A restore clash refuses and names the row.** An incompatible save is kept, listed, downloadable.
- **The exit runs on the owner's iPhone**; the Android emulator is a second reading, recorded.

**Not ruled, deliberately, and not to be decided during execution:**

1. **The public Worker's configuration** (R-6.5b2.6) — whether it serves multiplayer behind Access
   or single player alone. This cycle builds BOTH switches and proves them locally. M6.5c decides.
2. **Whether the card art becomes public** — it follows from (1) and is therefore also M6.5c's.

---

## 3. How to execute

**The plan's header marks each task's weight** per the owner's R-P11 refinement of 2026-09-20:

| inline | subagent-driven, Opus the floor |
|---|---|
| 3, 4, 5, 7, 11 | 1, 2, 6, 8, 9, 10 |

- **Ledger**: `.superpowers/sdd/2026-09-21-web-frontend-m6.5b2-plan/` (gitignored). `progress.md`
  with one `Ruling:` line per deviation from the plan, and every review report **written to a file
  whose PATH the agent reports back** — the notification channel truncates at about 4k.
- **Task 4 is a DEPLOY and it is mid-cycle**, not at the end. The offline exit cannot be measured
  until the shell is fetchable without a login. It touches no source; if it wants to, the front door
  has failed its own premise.
- **Task 11 schedules the whole-branch review and its fix wave** before the mutation battery. Do not
  run the battery against an unreviewed head.
- **A headed live review beats four headless ones** (M6.4 close §4.5): the first headed pass found a
  404 on every screen that four headless passes had reported as "zero console errors".

---

## 4. The five traps this session already paid for

Each was found by measurement, is written into the plan, and must not be re-derived.

1. **Vite's manifest is not an inventory.** Measured at this HEAD: it names `assets/index-<hash>.js`
   and its stylesheet, and **neither the Worker chunk nor the WebAssembly module appears in it.** The
   generator walks the finished `dist` tree instead, and its headline cell runs against a REAL build.
   A manifest-shaped fixture agrees with a manifest-shaped bug.
2. **Five Access applications are five audiences.** A token minted at `/enter` would authorize
   `/enter` alone, and a `fetch` carrying `X-Requested-With` gets a 401 rather than the silent
   re-authentication a navigation gets. ONE application, six destinations. And the deploy
   **protects before it exposes**.
3. **Serving the game routes is not the same as offering multiplayer.** The browser host plays by
   calling `/api/games/…` through the same `api::dispatch`, so one flag for both would stop single
   player. Two fields. And three transport routes live on the axum router OUTSIDE the layer's table
   (`lib.rs:730-732`), so filtering the table cannot remove them.
4. **The service worker would register in all 46 existing e2e cells**, because `build:client` is a
   production build. The fixture and its structural guard are Task 6's, not a discovery.
5. **`storageState` cannot carry Cache Storage or a worker registration.** The close-and-reopen cell
   needs `launchPersistentContext`.

**Two more that cost real time and are cheap to avoid.** `localPair` goes through the Worker, which
awaits the module before handling any message — so backup reads IndexedDB from the PAGE. And a
scripted edit batch that fails partway can silently discard its earlier successful edits: **write
after every edit, or verify at the end.** A green log is not a saved file.

---

## 5. Facts not to re-derive (all measured 2026-09-21, M5 workstation)

| | |
|---|---|
| Access, navigation | `302` to `bdvc.cloudflareaccess.com` |
| Access, `X-Requested-With: XMLHttpRequest` | `401` + `WWW-Authenticate: Cloudflare-Access` |
| Headers that do NOT trigger the 401 | `Accept: application/json`, `Sec-Fetch-Mode`, `Sec-Fetch-Dest` |
| Module today | 2,842,659 B raw / 753,763 B gzip, bot's first turn 28.6 ms |
| Module with `z` + fat LTO + `wasm-opt -Oz` | 1,168,913 B raw / 452,633 B gzip, 35.2 ms |
| Release today | ~14.6 MB (art 7.4, help 4.2, module 2.84, bundle 0.38) |
| Release after Task 5 | ~10 MB |
| Art | **131 WebP files plus `manifest.json`** (`CLAUDE.md` says 132 and is wrong) |
| A finished 595-event game | 54,103 B record + 554 B table, ~91 B per event |
| Workers Free | 100k invocations/day; **static assets free and unlimited**; no egress charge |

**And the behaviour at HEAD this cycle fixes**: offline, `archiveKind()` stays `"off"` because
`/api/health` never answers, so the send control is not drawn AND `Abandon` skips its upload and
deletes. A send that FAILS already keeps the game; nothing tries in the first place.

---

## 6. Definition of done

- Eleven tasks, each with its per-edit gates green and its own commit.
- The whole-branch review, its fix wave, the scoped re-review, and the mutation battery at that head
  with every mutation mapped to a catcher.
- The release deployed and recorded in `deploy/RUNBOOK.md`.
- **The exit performed on the owner's iPhone by hand**: install, airplane mode, play, close the
  browser entirely, reopen, resume, finish, export; then an update and a restore on a fresh profile.
- A close on the M6.5b close's shape, whose §7 starts from the spec's §7 and is corrected by what
  execution actually found.
- The two doc debts paid: `CLAUDE.md`'s art count, and the 109-byte build variance recorded.
- The PR opened against `web-room-m6.5b`; merge order is the owner's.
