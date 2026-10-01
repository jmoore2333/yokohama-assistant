# M6.5b2 "installable and offline" — entry handover

**Written 2026-09-21 at the close of M6.5b, for the session that BRAINSTORMS and SPECS M6.5b2 with
the owner** — the cycle the M6.5b spec's §8 charters as "the FIRST follow-up after M6.5b, its own
brainstorm and spec", ruled by the owner on 2026-09-20 to sit BETWEEN M6.5b and M6.5c ("offline
mode is its own cycle, 'M6.5b2'"). This document is the record of what M6.5b measured that the
offline design depends on, and of the charter's requirements as the owner already stated them.
It is NOT a spec: every product decision below is the brainstorm's to take.

## 0. Read order

1. The M6.5b close `docs/plans/2026-09-21-web-frontend-m6.5b-close.md` — §1.3/§1.4 (the module,
   the Worker, IndexedDB), §6.3 (the owner items that touch this cycle), §7 (what is not claimed).
2. The M6.5b spec `docs/plans/2026-09-20-web-frontend-m6.5b-single-player-spec.md` — R-6.5b.2 (level
   2 versus level 3, the owner's words), **§8's first bullet** (the level-3 charter, verbatim below).
3. The owner's pre-execution review
   `docs/plans/2026-09-20-web-frontend-m6.5b-review-free-tier-offline-saves.md` §3 — where the
   charter's requirements came from.
4. The umbrella `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` — R-6.5.3 (single-player
   entirely local), R-6.5.8 (test in the live setup), R-6.5.12 as amended by R-6.5b.16 (Workers
   FREE; static assets served without the Worker), because the cache this cycle designs is what
   M6.5c's static-asset deployment will serve.

## 1. The charter, as the owner stated it (spec §8, verbatim requirements)

A service worker caching the shell, the module and the art with a cache keyed to the release and
bustable for a patch, an update prompt, about 8 MB per browser. Level 2 (M6.5b) is "the API
unreachable with the page loaded", never "the origin down"; level 3 is the origin down. The
requirements, carried from the pre-execution review's §3:

- **A coherent release cached WHOLE** (HTML, JS, the glue, the module, the art, the manifest, the
  labels) before "available offline" is said.
- **Release-keyed asset paths**, with an update installed and verified before the switch — never
  mid-move — and eviction that cannot touch IndexedDB.
- **A persistent-storage request** when a player starts relying on local saves, with the grant
  inspected and a backup offered.
- **A browser backup and restore format** carrying the record AS A STRING beside its table
  document with a versioned envelope, validated in Rust, refusing to overwrite silently — today a
  downloaded save is a CLI recovery path and the browser has no restore.
- **Incompatible saves kept listed and downloadable**, never migrated or erased.
- **The origin kept stable** across the NAS-to-Workers move, since a different hostname is
  different storage.
- **The exit**: airplane mode → close the browser → reopen → resume → finish → export, then an
  update and a restore on a fresh profile, on the owner's own phone.

## 2. What M6.5b measured that the design depends on

- **The module.** `yokohama_wasm_bg.wasm` is **2,842,550 B raw / 758,100 B gzip** (release;
  cargo's own output 3,322,799 / 859,761 before `wasm-bindgen`), against the umbrella's 1.30 MB /
  387 KB for the seam alone — it carries every route, `Games`, serde for every payload and the
  bot. Emitted by Vite as a HASHED asset (`dist/assets/yokohama_wasm_bg-<hash>.wasm`), so its
  path is already release-keyed. The workspace has NO `[profile.release]` tuning (`opt-level`,
  `lto`, `codegen-units`, `wasm-opt`); a size budget belongs to this cycle, which caches it.
- **The asset graph a release ships** (one build, `web/dist/assets/`): `index-<hash>.js` ~329 KB,
  `index-<hash>.css` ~35 KB, `worker-<hash>.js` ~13 KB (the Worker's own chunk), the `.wasm`
  above, plus `index.html`; the art — 132 WebP files, ~7.4 MiB — is served from `/art/` by the
  host out of `crates/yokohama-web/assets/art/` with a manifest at `/api/art`; the labels come
  from `/api/labels` (server first, the Worker as the fallback, bounded at five seconds). That is
  the "about 8 MB per browser" the charter names, and the manifest and the labels are the two
  things a coherent cache must hold beside the files.
- **The module is instantiated on EVERY first-screen load** to answer the local listing
  ("On this device" is R-6.5b.8's guarantee and needs the Worker); the browser caches the asset
  after the first visit, so the recurring cost is instantiation, measured as part of the reload
  wall below. An IndexedDB-only listing (reading the stored table documents without the module)
  is the alternative if the boot must be lazier.
- **Boot and replay.** Browser navigation → first frame for a 102-event local game:
  **98 / 101 / 102 ms** over three runs (Chromium, the debug module; includes the Worker's boot,
  the module's instantiation and the rebuild from IndexedDB). Natively, `Session::from_record`
  over the committed 595-event pair: **23.8 ms** (27.1 ms on the reviewer's run). Not like for
  like, and neither gated.
- **IndexedDB layout**: database `yokohama-local`, version 1; store `docs` keyed `[doc, id]`
  holding the layer's documents as opaque TEXT (their own schema is the layer's `TABLE_SCHEMA`,
  3 at this release); store `meta` keyed `id` holding the ownership generation. A version bump is
  only ever a new store. **Eviction that cannot touch this database** is the charter's hardest
  sentence: a cache's eviction and site-data clearing are different browser mechanisms, and only
  the second reaches IndexedDB — the persistent-storage request is what asks the browser not to.
- **The save formats that exist**: "Save game file" downloads the RECORD alone (`<id>.json`, the
  engine's format, what the terminal loads); "Send to the archive" posts the record AND the local
  table document as two strings to `POST /api/archive`, which is exactly the pair a browser
  backup would carry — the versioned envelope around them is this cycle's to define, validated
  by the layer (`TableDoc::agrees_with` and `replay_strict` already exist for the archive route).
- **A stored game after a rules change**: `Session::from_record` refuses a record at another
  `SEMANTICS_REV` with a 500 carrying the engine's sentence (not a trap); the row stays listed,
  Save works, a plain Abandon works — M6.5b's screen for it is the base for "incompatible saves
  kept listed and downloadable".
- **Web Locks on real WebKit**: measured PASS on an iPhone 18 Pro simulator (iOS 27.0, Safari
  27.0) inside a dedicated module Worker — `navigator.locks`, `indexedDB`, `crypto.getRandomValues`
  and `WebAssembly.instantiateStreaming` all present; the contention, the steal and the holder's
  `AbortError` all as the design expects. NOT measured: Android, a physical device, older iOS,
  background suspension (whether a suspended tab keeps its lock) — the last is the one the
  offline exit ("close the browser → reopen") makes this cycle's to measure.

## 3. What M6.5b leaves that touches this cycle (from the close's §6.3)

- The module's size and the absence of a release profile (above).
- `walk.json` is 419 KB of pretty-printed JSON rewritten wholesale on every regeneration — not an
  offline concern, but the same review that sizes the cache will meet it.
- The first-screen boot instantiates the module every visit (above).
- `npm run e2e` leaves the DEBUG module in `web/dist`; a deploy rebuilds with `npm run build`
  (RUNBOOK §1). A release-keyed cache must never be cut from an e2e-built `dist/`.
- The self-host `deploy/Dockerfile` builds the module in its Rust stage; nothing in CI builds that
  image.

## 4. Decisions the brainstorm takes FIRST

1. The cache key and the update lifecycle: what "the release" is (the crate version, a build hash,
   or the `.wasm`'s hash the pin's sidecar already records), when an update is fetched, verified
   and switched, and how a patch busts the cache.
2. The backup/restore envelope: fields, versioning, where validation lives in Rust (the layer, so
   the Worker and the archive route share it), and the refusal to overwrite.
3. The persistent-storage moment and the wording that asks for it.
4. What "installable" means on iOS Safari versus Android Chrome (a manifest, the add-to-home-screen
   path, the standalone display) — and which one the owner's phone is.
5. Whether the module's size is budgeted here (a `[profile.release]` for the wasm crate, `wasm-opt`)
   or recorded again.

## 5. Process

The same as M6.5b's: a spec written from the brainstorm, reviewed adversarially before the plan;
a plan in dependency order ending in the spec's own cells; execution under R-P11 as refined
2026-09-20 (small mechanical tasks inline, judgment-heavy tasks subagent-driven with Opus the
floor); a whole-branch review, a mutation battery, a HEADED live review of the deployed site on the
owner's phone (the charter's exit is a phone exit).
