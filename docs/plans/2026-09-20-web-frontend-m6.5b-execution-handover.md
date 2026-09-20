# M6.5b "single-player in the browser" — execution handover

**Written 2026-09-20 at the end of the session that deployed M6.5a, brainstormed M6.5b with the
owner, wrote its spec and plan, and carried them through four review waves — for the FRESH session
that executes the plan.** Branch `web-room-m6.5b` (stacked on `web-room-m6.5`; nothing merged; the
PR stack is #31 → #35 with #35 the M6.5a PR against `web-room-m6.4`). HEAD at the handover is the
commit that adds this file; every fact below names where it lives.

## 0. Read order (the session's first fifteen minutes)

1. `CLAUDE.md` — the Web (server) and Web (browser half) bullets; the `crates/yokohama-web`
   architecture bullet; the M6.5a sentence in "Where things stand".
2. `docs/handover.md` Track 3.
3. **The spec** `docs/plans/2026-09-20-web-frontend-m6.5b-single-player-spec.md` — §1 (R-6.5b.1–16),
   §2 (the mechanism), §6 (the instruments), §8 (what is not claimed), **§9 and §10** (what four
   review waves changed and why; a sentence that reads oddly traces to one of them).
4. **The plan** `docs/plans/2026-09-20-web-frontend-m6.5b-plan.md` — nine tasks; its Global
   Constraints; Task 4 is the SDD candidate.
5. The owner's pre-execution review `docs/plans/2026-09-20-web-frontend-m6.5b-review-free-tier-offline-saves.md`
   — advisory; every finding's disposition is spec §10, so read it for the REASONING behind
   Task 4's shape, not for open items.
6. The M6.5a close `docs/plans/2026-09-20-web-frontend-m6.5a-close.md` §1 (what the layer IS).

## 1. Where things stand

- **M6.5a is LIVE on the NAS** as `yokohama-web:0.5.0-m6.5a` (container `579cd1ff…` since
  2026-09-20 14:04:35Z; `deploy/RUNBOOK.md` §3.12 is the record: both archive sites read off the
  volume, the archived record byte-identical to the store's and to the record route's body, the
  served record loaded in the terminal with `why` reconstructing the bot's last move). The
  owner's one bot table on the volume is a schema-2 document and reads under the new binary.
- **The M6.5b spec is at its fourth revision** (commits `e888d78` → `945efee` → `e9e3159` →
  `54a9a1f` → `3f455d6` → `5f44aac`): an Opus adversarial read (2C/10I/13M), its scoped re-read
  (one new Critical — the archive's table check would have refused every real send), the owner's
  lightweight-agent read (the run route's 404 boundary one position too tight against the
  client's own `to = next + 1`), and the owner's pre-execution review of the PLAN (eight
  corrections, two reproduced: a static import of the glue from `public/` fails Rollup; a
  trapped wasm instance traps again on re-`init()`). All taken; §10 is the table.
- **The plan is at `4a46d43`** (1,927 lines; its Task 4 Worker rewritten to the spec's §2.2:
  one serialised actor, ownership from the ROUTE id, an ownership generation checked in every
  IndexedDB write, persistence of the documents that EXIST for HELD games BEFORE the reply,
  storage failure distinct from a trap, a fresh Worker on a trap; the archive put protocol
  `Landed` / `Identical` / a conflict the client keeps the game on; the glue bundled from
  `web/src/local/wasm/`; the walk script OUTSIDE the corpus at `web/src/wire-script/`).
- **Owner rulings of the day, all recorded:** the hosted target is Workers FREE — **R-6.5b.16
  amends umbrella R-6.5.12** (the decision-log row dated 2026-09-20 in `guidingpath.md` carries
  it; M6.5c inherits: gameplay, replay, `suggest`, `why` and the archive's strict replay INSIDE
  objects, static assets outside the Worker, the Free quotas budgeted then measured live).
  Wrangler on the workstation is current (4.135.0 — an npm global under `/opt/homebrew/lib/
  node_modules`, NOT a brew formula; `npm install -g --allow-scripts=esbuild,workerd
  wrangler@latest`, because the plain install skips workerd's postinstall); the retired proxy's
  pin follows it (`67fa945`).
- **Offline mode is its own cycle, recommended as "M6.5b2"**, after M6.5b's close and BEFORE
  M6.5c: its charter is spec §8's level-3 paragraph (a coherent cached release, a release-keyed
  update lifecycle, persistent storage, a browser backup/restore format validated in Rust,
  incompatible saves kept listed), written as a spec at M6.5b's close from the MEASURED
  artifacts. The owner asked whether to fold it into M6.5b; the recommendation given was no, for
  the reasons in the last message of the session (a spec's worth of product decisions; it
  depends on b's measured asset graph; b is already nine tasks; its asset lifecycle should be
  settled before c's static-assets deployment). **The owner agreed — §2 item 3.**

## 2. The three decisions, RULED (owner, 2026-09-20, at the handover)

1. **The execution mode: subagent-driven development for Tasks 2–8, inline for Tasks 1 and 9.**
   The owner opened the door to SDD across the board and asked what fits this phase; the session
   recommended this split and the owner's framing adopts it. Reason: this phase is NOT mostly
   mechanical — Tasks 3–6 and 8 are where every review wave found its Criticals (the Worker's
   queue and ownership generation, the trap recovery, the build graph, the four door gates, the
   real-browser fault cells), Task 2 touches a path-safety boundary and a cross-cycle trait, and
   Task 7's risk is classification; Task 1 is small, fully specified and deployable alone, and
   Task 9 is the orchestrator's by nature. **Opus is the floor for every implementer and
   reviewer; the orchestrator adjudicates.** The layered reviews stay on top: the whole-branch
   Opus review before the release, spec §6.6's mutation battery, and a HEADED live review of the
   deployed site. (The 2026-09-20 R-P11 refinement — small mechanical tasks inline — is applied,
   not reversed: these tasks are neither.)
2. **The two spec defaults STAND**: `labels` server-first with the Worker as the fallback; the
   readable caller directory (R-6.5b.15).
3. **Offline mode is its own cycle, "M6.5b2"**, after M6.5b's close and before M6.5c; its
   charter is spec §8's level-3 paragraph and its spec is written at M6.5b's close from the
   measured artifacts. It is NOT in the M6.5b plan and the plan is not to grow it.

## 3. How to execute (the process, exactly)

- Every deviation from the plan gets a `Ruling:` line in
  `.superpowers/sdd/2026-09-20-web-frontend-m6.5b-plan/progress.md` (gitignored; `mkdir -p` it)
  BEFORE the close reads it; every review report lands there too.
- The per-edit gates after every task are the plan's Global Constraints; the fenced-crate diff
  must be EMPTY at every commit; `SEMANTICS_REV` 7.
- The corpus is regenerated ONLY at Tasks 1, 2 and 7, and the diff READ against the expected
  diff the task states; anything else is a finding.
- `cargo test` takes ONE positional filter. The whole-game gate's vs-bot arm livelocks on some
  seeds: re-run it ALONE before believing a red.
- Commit attribution: the executing agent's own lines, never the plan author's.
- Subagent briefs: the full report to a file, the reply the path plus three lines; long commands
  in the FOREGROUND; never follow up an implementer without `git log -1` and `git status`; never
  rewrite a reviewed SHA.

## 4. Gotchas this session paid for (do not re-derive)

- **A python edit script chained before `git commit` without `&&` committed the plan alone while
  the message claimed the spec too** (`4a46d43`; corrected forward in `3f455d6`). Use
  whitespace-tolerant anchors (`\s+` between words) for prose edits, and `&&` before the commit.
- **Cloudflare answers `403 error code: 1010` to Python's default `User-Agent` through the
  tunnel**; curl's passes. Any scripted check sets a `User-Agent`.
- **WARP must be connected** for the workstation to reach `yokohama.moore.nyc` (`warp-cli
  status`; `warp-cli connect` and `warp-cli debug access-reauth` if not).
- **The NAS deploy recipe** is `deploy/RUNBOOK.md` §1–§3 with §3.12 as the latest record; the
  context tarball is served from the workstation with `python3 -m http.server 8899 --bind
  0.0.0.0` and fetched by the NAS through Portainer's `POST /build?remote=…`; the create body
  carries BOTH `Env` entries. A rollback across `0.5.0-m6.5a` is a SCHEMA rollback (§3.12, §5).
- **`.superpowers/sdd/.gitignore` is `*`**, so the ledger directory is ignored by construction.
- The M6.5a scratchpad driver that plays a vs-bot game through the JSON API (`drive.py`, the
  e2e harness's policy at the wire) lives only in that session's scratchpad; its shape is in
  RUNBOOK §3.12. Rebuild it if a live game-to-the-end is needed again.

## 5. Facts the executing session should not re-derive

- `wasm32-unknown-unknown` is installed on the pinned toolchain; the Node target's glue is
  CommonJS and `web/package.json` is `"type": "module"` (hence the `wasm-node/package.json`).
- `wasm-bindgen` is `0.2.128` in the lock; `wasm-bindgen-cli` matches it.
- `create` writes only a TABLE document; the record appears at the last keep
  (`tables.rs:1791`). A bot seat's keep lives in the record and is `None` in the table document.
- `at/{k}` refuses a position past the end with a 404 in THIS API's words; a non-numeric `k` is
  a 400 in axum's. The client's `fetchRun` sends `to = next + 1`.
- The four door gates are `App.tsx:922-926`, and with every fetch failing the FIRST (`door.error`)
  fires; the route effect's guard at `App.tsx:503` also gates local routes.
- `api.ts` holds TWO `fetch` calls (`call`, `recordText`) and `sse.ts`'s `readStream` is the third.
- The corpus's seed and password pins scan `corpus()`; a request script inside `web/src/wire/`
  would trip them.
