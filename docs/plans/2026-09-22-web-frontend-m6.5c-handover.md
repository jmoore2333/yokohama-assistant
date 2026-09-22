# M6.5c "the objects" — entry handover

**Written 2026-09-22 at the close of M6.5b2, for the session that BRAINSTORMS, SPECS and PLANS
M6.5c** — the third and last sub-cycle of the hosting move, chartered by umbrella R-6.5.7 as "the
room on Durable Objects" after the seam (M6.5a) and the browser (M6.5b), with M6.5b2 inserted
between them by owner ruling. This document is the record of what the earlier cycles MEASURED that
the objects design depends on, and of the requirements as the owner already stated them. **It is
NOT a spec: every product decision below is the brainstorm's to take**, and where it and a frozen
artifact disagree, the artifact wins.

---

## 0. Read order

1. **This file.**
2. The umbrella spec `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` — **§1's thirteen
   rulings R-6.5.1–R-6.5.13** (owner direction, not re-litigated inside a sub-cycle), §2.2–§2.6 (the
   route-to-object map, the table object, the house object, the client's transport seam), §3.3 (the
   M6.5c charter and its exit), §4 (the mutations M6.5c's battery must catch), §5 (the deployment,
   written before Workers FREE), §6.1 (the OPEN list, O-1 … O-12).
3. The M6.5b2 close `docs/plans/2026-09-22-web-frontend-m6.5b2-close.md` — **§5** (every ruling with
   its cost), **§6.2/§6.3** (the LEAVE list and the nine OWNER ITEMS), **§6.4** (two frozen-spec
   amendments), **§7** (what that cycle does NOT claim), §8 (which now points here).
4. The M6.5b close `docs/plans/2026-09-21-web-frontend-m6.5b-close.md` **§8's "Into M6.5c"
   paragraph** — quoted verbatim in §3 below — and its §6.3 owner items.
5. `CLAUDE.md` — the operating facts and every gate command. **Re-count every suite at your own
   HEAD**; the close §2 table is a reading, not a promise.
6. `deploy/RUNBOOK.md` §3.15 (the twelfth deployment, the release M6.5c inherits) and §3.14 (the
   hostname move), plus §5 (rollback: M6.5a's schema sentence, and now M6.5b2's browser-side one).
7. The M6.5b2 spec `docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md` **§8 "Carried to
   M6.5c"** and §7; §0.3 is what that cycle measured and none of it is to be re-derived.

---

## 1. Where things stand

- **Branch `web-room-m6.5b2`**, HEAD `74396ee`, `git status` clean; the cycle's diff base is
  `369a1f2` and the release was built from `43d54a5` (the fix-wave head whose battery read 22 of 22).
- **LIVE: `yokohama-web:0.5.0-m6.5b2`**, image `sha256:c802a036c2cb…` (24,466,536 B), container
  `58ac45ba478a…` started **2026-09-22 19:08:27.10Z**, at **`yokohama-nas.moore.nyc`** — the NAS's
  new hostname, taken mid-cycle at Task 4 (RUNBOOK §3.14) — behind the **HOSTNAME-WIDE** Access
  application `yokohama` (id `11cbcf68-96cd-4310-baee-f2ade812155d`, audience `5fa7a989…`, three
  destinations, two allow policies, `session_duration` 730h). Health reads `identity: claimed`,
  `signal: sse`, `archive: dir`, `multiplayer: true`, `semantics_rev: 7`.
- **The path-scoped Access door is BUILT and proven locally and is NOT deployed** (owner ruling,
  close §5.1 PF-8): during development no path on the site is public, so **the zero-login
  single-player claim is not SHOWN live** and the plan's "public paths 200 with no session" readings
  were deferred and recorded as the 302s they are (close §7).
- **`yokohama.moore.nyc` is HELD for the Worker and has never been deployed with wrangler.** Its
  DNS record still points at the NAS tunnel and every path falls through to the catch-all **404**;
  one edge-cached immutable asset (`/assets/index-COGqoKfc.js`) still read 200 with
  `cf-cache-status: HIT` and ages out (RUNBOOK §3.14 tail). **Taking that name is M6.5c's first
  infrastructure step**, and umbrella §5's "give ttyd its new hostname first" is already satisfied —
  the terminal is on `yokohama-cli.moore.nyc`.
- **Seven pull requests are open, stacked, and NOTHING is merged; merge order is the owner's**:
  #31 `web-room`→`master`, #32 →`web-room`, #33 →`web-room-m6.2`, #34 →`web-room-m6.3`,
  #35 →`web-room-m6.4`, #36 →`web-room-m6.5`, **#37 `web-room-m6.5b2`→`web-room-m6.5b`**
  (https://github.com/jmoore2333/yokohama-assistant/pull/37). Confirm with `gh pr list --state open`.
- **`SEMANTICS_REV` is 7**; `crates/yokohama-core`, `-oracle`, `-difftest`, `-bridge`, `gen-data`
  and `crates/yokohama-cli` are **byte-identical to `369a1f2`** (close §1.11), so every
  fifteen-gate roster figure is unchanged by construction and none was re-run.
- **The wire corpus is 94 files** and moved this cycle by exactly one key (`health.json` gained
  `multiplayer`; close §3). `web/src/wire-script/` gained `backup-envelope.json` and
  `backup-report.json`.

---

## 2. What was MEASURED, and where each figure lives

- **The module.** `--profile release-wasm` (root `Cargo.toml`: `inherits = "release"`,
  `opt-level = "z"`, fat LTO, one codegen unit) plus `wasm-opt -Oz --strip-debug --strip-producers`,
  binaryen **132**: **1,783,361 → 1,175,223 B** raw, **456,054 B** under `gzip -9` (the
  `Cargo.toml` comment's 456,048 is the same 1,175,223 bytes; 6 bytes of gzip variance). The
  **Docker-built** module read **1,173,055 B** against the workstation's **1,169,222 B** at Task 5
  — 3,833 B apart with the SAME optimizer, `cargo` + `wasm-bindgen` on x86_64-linux versus
  aarch64-darwin, recorded and not diagnosed, and **not re-measured since Task 5**. **A ceiling must
  clear the larger host**; the release ceiling in force is **1,400,000 B** (debug 3,200,000).
  *RUNBOOK §3.15; close §2 and §7; ledger Task 5.*
- **The release.** The precache generator's own lines: **153 entries, 9,935,340 B** (total ceiling
  11,900,000 B); module 1,175,223 · bundle 410,991 · fonts 98,728 · shell 11,873 · icons 19,456 ·
  help 764,742 · art 7,454,327; **131 art files** plus `manifest.json` (the "132 WebP" in older
  docs counted the manifest); `sw.js` 27,220 B; cache name **`yokohama-release-19e8532f977bcba2`**,
  read back off the wire at the deploy. Help pictures went 4.21 MB of PNG → 764,742 B of WebP.
  Fraunces (67,388 B) and JetBrains Mono (31,340 B) are self-hosted woff2 through the HASHED
  `/assets/` pipeline, with OFL copies shipped at `dist/fonts/<family>/OFL.txt` (4,391 / 4,399 B)
  served `text/plain; charset=utf-8`. *RUNBOOK §3.15.*
- **The service worker's rules.** Install is all-or-nothing with `cache: "reload"` and a per-body
  digest; **nothing calls `skipWaiting()` by itself and `clients.claim()` is never called**;
  `activate` **RETAINS the previous generation** and deletes only what is older; `/api/*` and
  `/enter` are **never intercepted**, asked FIRST (`web/src/swRoute.ts:34`, `:40-42`); `/sw.js` is
  served `no-store` on the success arm AND the miss arm. The cache NAME is a digest of every
  entry's bytes, so a re-cut card busts it although its filename never changes. *Close §1.6; spec
  §2.3.*
- **Access answers two ways and one header decides.** A navigation, and any request without
  `X-Requested-With`, gets a **302** to `bdvc.cloudflareaccess.com`; a request carrying
  `X-Requested-With: XMLHttpRequest` gets a **401 with `WWW-Authenticate: Cloudflare-Access`** and
  `access-control-allow-credentials: true`. `Accept: application/json`, `Sec-Fetch-Mode` and
  `Sec-Fetch-Dest` were each measured NOT to trigger it. **One application is one audience** — five
  applications would be five audiences and a token minted at `/enter` would authorize `/enter`
  alone. *Spec §0.3, §2.1.*
- **IndexedDB at v2.** Database `yokohama-local`, **VERSION 2** (`web/src/local/db.ts:118`); stores
  `docs` keyed `[doc, id]` with kinds `record | table | compat` (`:32`), `meta` keyed `id` (the
  ownership generation), `outbox` keyed `id` — a tagged union, pending with one of **four reasons**
  `offline | signed-out | unknown | refused` (`:87`) or sent with its receipt. **A rollback across
  v2 is a browser-side schema rollback**, the same shape as M6.5a's server-side one.
- **The backup envelope.** `ENVELOPE_KIND = "yokohama-backup"`, `ENVELOPE_VERSION = 1`
  (`crates/yokohama-web/src/backup.rs:40-46`); a file of another version is refused AS A WHOLE. The
  record and table document cross every seam as **opaque strings, never parsed** by the client;
  validation is `check_pair` — **the archive route's own function, moved there verbatim** and NOT
  behind the `server` feature — plus `check_addressable` for the id contract `TableDoc::agrees_with`
  cannot supply. *Close §1.7.*
- **The outbox uploads by itself.** A finished LOCAL game uploads to the archive at game over with
  nobody clicking, and `reconsiderAll()` uploads every finished local game on the device the first
  time an archive is discovered. Spec-conformant (§2.6 rule 2); a player can notice it. *Close §6.3
  item 7.*
- **A finished game is small.** 54,103 B of record and 554 B of table document for 595 events —
  about **91 B per event**, against a SQLite row cap of 2 MB (umbrella §2.3). *Spec §0.3.*
- **The e2e.** **53 cells in 11 spec files, 7.4 min of tests / 450 s wall**, seed-dependent; it runs
  Chromium on one machine and stays local. *Close §2.*
- **The vantage.** This workstation is **WARP-enrolled: every curl from it is an authenticated
  read**, so unenrolled readings were taken from a throwaway alpine container on the NAS. A lapsed
  Access session reads 302 on every path while `warp-cli` still says Connected, which looks exactly
  like a broken deploy. *Close §4.5; RUNBOOK §3.15.*
- **Workers Free, as recorded** (umbrella §0.4, read 2026-09-19): **100,000 DO requests/day**,
  13,000 GB-s/day; **static asset requests are free and unlimited with no egress charge**; Worker
  bundle 64 MiB uncompressed; **startup 1 s** (O-2's risk against the module); DO CPU per request
  reads 30 s with no plan qualifier while plain-Workers Free reads 10 ms, **and which binds a
  Free-plan object was not tested**.

---

## 3. What M6.5c inherits as REQUIREMENTS (the owner's words, quoted so they are searchable)

From `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` §1:

- **R-6.5.1** — "Hosted multiplayer runs on Cloudflare Workers and Durable Objects… **The axum
  binary in Docker stays exactly as it is** — the NAS deployment, the self-host path". `wrangler
  dev` is NOT a self-host path.
- **R-6.5.2** (R-P5 amended) — "a table's documents live in its Durable Object's SQLite, the
  lobby's index in a house object, and **there is no Postgres in this lane**."
- **R-6.5.3** — "a hostname-based (self-hosted) Cloudflare Access application with the owner's
  allow-list… hostname-based and not the Worker-level feature, because the transport is WebSockets".
  **AMENDED by R-6.5b2.2** (M6.5b2 spec §1): "The hostname-wide application becomes path-scoped
  applications on the protected prefixes, and a deployment with no Access at all is a supported
  shape."
- **R-6.5.4** — a finished game is written to R2 by its table object "in the FILE STORE's own
  layout… under a `SEMANTICS_REV`-keyed prefix, so an `rclone sync` of the bucket is a directory the
  CLI loads"; chat excluded; **archived AT MOST ONCE**; "a failed archive at game over is logged and
  retried and **NEVER refuses a move**"; ONE shared layout function with a cell over both
  implementations (O-9).
- **R-6.5.5** — abandon-and-save: "**The delete AWAITS the archive and is REFUSED**, with a sentence
  naming the archive, when the put fails".
- **R-6.5.6** — single-player is entirely local. **This is DONE** (M6.5b/M6.5b2), and what it
  changes for M6.5c is concrete: **the Worker serves NOTHING for single player** — the door, the
  shell, the module and the art are static assets, and R-6.5b2.7's "a single-player visitor invokes
  the Worker ZERO times" is the property to preserve when the routes move.
- **R-6.5.7** — the order M6.5a → M6.5b → M6.5c; M6.5c is the last.
- **R-6.5.8** — "**test in the live setup during development**… M6.5c deploys to the real account
  behind Access EARLY and repeatedly… **The first thing the first deploy tests is an
  Access-protected WebSocket upgrade carrying the caller's identity** (O-6), … the second is a cold
  start of the module against the 1 s startup limit (O-2)."
- **R-6.5.9** — the fences: five crates byte-identical, `SEMANTICS_REV` 7, the CLI untouched, no bot
  behaviour moved, and "**the wire corpus is the contract across ALL THREE hosts**", with the
  host-parameterized second class (`/api/health`) named rather than exempted silently.
- **R-6.5.10** — `GET …/record` to a seat holder only once the game is over (409 before).
- **R-6.5.11** — the hosted Worker refuses BOTH client-set identity headers ("the rule, not a copied
  list"); on the hosted path the mode is `cf-access`; `ctx.access` is recorded UNAVAILABLE on this
  shape.
- **R-6.5.12 — AMENDED by R-6.5b.16 to Workers FREE.** The umbrella's §1 and §5 still read "Paid";
  the M6.5b close §8 and M6.5b2 spec §0.2 carry the amendment, and **Free is the target**.
- **R-6.5.13** — M6.5c deletes `deploy/worker/` and `deploy/modal_app.py` and rewrites RUNBOOK §4 as
  the record of why, carrying out only the strip rule; the NAS sections stay.

From the M6.5b close §8, verbatim: "gameplay, replay, `suggest`, `why` and the archive's strict
replay run INSIDE objects with `/api/archive` mapped to an object of its own; the layer's
`Archive::put` is synchronous and an R2 put is not, so **M6.5c defines a prepare/upload/commit
protocol with a receipt before a delete is authorised**; hibernating WebSockets with the picker
persisted (umbrella H-4); static assets served without the Worker…; the house projection updated
only when a visible field changes; the long-save measurements…; **the browser crate's global
`Games` is the per-tab instance's and never a shared isolate's**; R-6.5b.11's two push shapes."

---

## 4. The OPENING ITEMS the owner asked to carry here

Owner, 2026-09-22: the phone readings "can be noted as opening items for the 6.5c plan/spec
handover" (close §5.1). Each below names where it came from.

1. **The two phone readings** — the iPhone exit per R-6.5b2.17 (install, airplane mode, play, close
   the browser entirely, reopen, resume, finish, export; then an update and a restore on a fresh
   profile) and the **Android HANDSET** as the second reading, the emulator skipped by owner ruling.
   **Nothing on a phone was measured by M6.5b2.** *Close §5.1, §6.3 item 3, §7.*
2. **The HEADED walk of the deployed site, owed across TWO releases** (§3.14's and §3.15's): both
   deploy agents were blocked by Claude-in-Chrome reporting "extension is not connected". The list:
   the door and the lobby's back-link at 390 px, the door's backup and restore controls (no width
   gate covers the door), and the five error strips the Refusal sweep touched mid-flow. *Close §6.3
   item 1.*
3. **The clean-browser single-audience reading** — one login at `/enter`, then every destination
   without navigating to them. The enrolled workstation cannot take it. *Close §6.3 item 2.*
4. **The public Worker's configuration (R-6.5b2.6)** — multiplayer behind Access, or single player
   alone — **and therefore whether the card art becomes public**, which `ASSETS.md` carves out as
   carrying no grant of any kind. *Close §8.1 item 2; M6.5b2 spec §8.*
5. **`cf-access` (or another method) in place of `claimed`**, which is what makes a path-scoped door
   **fail closed**: today a missed Access application on a new protected route would be open,
   because `claimed` cannot. With it, the live deployment of the path-scoped door that was built and
   not deployed. *Close §7, §8.1 item 3.*
6. **The automatic upload of a finished local game** (spec §2.6) — a product fact the owner may want
   to keep, gate or announce, and `reconsiderAll()` sends games finished before the release.
7. **Four "Lobby"-labelled controls on LOCAL screens under a `VITE_MULTIPLAYER=false` build** lead
   to a lobby whose game routes do not exist — its own refusal and one link back, so nobody is
   stuck, but a product call the spec did not make. *Close §6.3 item 5.*
8. **The SPA-fallback cells must be RE-PROVEN on the Worker**: `/sw.js` 404ing rather than answering
   HTML, and `/enter` reaching the origin rather than the fallback, are proven against axum's own
   map; Workers' static assets have their own `not_found_handling`. *Close §8.1 item 5.*
9. **`POST /api/archive` still hands an untrusted `GameRecord` to the engine's setup path**, which
   moves from behind a development door to a public one the moment the Worker is public. *M6.5b
   close §6.3 item 6; M6.5b2 close §7.*
10. **The no-Web-Locks fallback is still invisible**, and **background suspension is unmeasured**.
11. **The umbrella §6.1 measurements still owed**: cold replay (O-3), a whole bot turn, `why`, and
    the archive's strict replay **inside a live Free-tier object**; plus O-1 (the panic policy),
    O-2 (the 1 s startup against the module), O-6 (the Access cookie on a WebSocket upgrade) and
    O-7 (`getrandom`'s `js` feature inside a Worker, which `create` depends on).
12. **The CI window `6f68b0b..d0363e2` in which the workflow did not PARSE** — no reading in it was
    a reading of anything. A process item: **any edit to a workflow file gets a local parse check in
    the gate** (`python3 -c "import yaml; yaml.safe_load(open('.github/workflows/test.yml'))"`).
    *Close §6.3 item 6.*
13. **`npm test` has a pre-existing order-dependent flake** (reproduced with the cycle's work
    stashed). *Close §6.3 item 4.*
14. **The server binary SHRANK 50,904 B** unexplained; checked to be from this tree, not diagnosed.
    *Close §6.3 item 9.*
15. **AppleDouble sidecars in every image cut on macOS** (141 beside 132 art files), harmless and
    recorded rather than fixed so the images stay comparable; **`deploy/Dockerfile`'s base tag still
    floats**. *Close §6.2.*
16. **The Docker-vs-workstation module byte gap** (§2 above), un-re-measured since Task 5.
17. **The two frozen-spec amendments** recorded at the close: the update offer renders on the **DOOR
    ONLY**, and §3.2's "FIVE destinations" enumerates six. *Close §6.4.*
18. **Inherited from the M6.5b close §6.3 and still open**: `walk.json`'s 419 KB pretty-printed
    wholesale diff; the first-screen boot instantiating the module on every visit; "who starts" on
    the local form; nothing in CI building the self-host image; the vs-bot whole-game gate arm's
    **livelock** on some OS-entropy seeds (it fired three times this cycle and read `2 passed` alone
    at the same commit in 24–26 s).

---

## 5. The decisions the M6.5c spec takes FIRST (questions, not answers)

1. **The object's storage shape.** Three rows or tables in SQLite (umbrella §2.3) — and what becomes
   of the layer's `Version`/compare-and-swap, which an actor does not need but whose `check_shape`
   must still run on read and write because every stored field is an input. **What does the browser
   crate's global `Games` become inside an object** — one instance per object, and what the cold
   wake costs (O-3).
2. **The R2 archive protocol.** `Archive::put` is synchronous and an R2 put is not: the
   prepare/upload/commit shape, the receipt before a delete is authorised, the alarm retry, and how
   "at most once" survives both reasons.
3. **How the outbox's `POST /api/archive` maps to an object** — an object of its own per the M6.5b
   close §8 — and what a browser-sent save costs inside a Free-tier object once the four checks and
   the strict replay run there.
4. **The house projection**: updated only when a visible field changes; the revision rule that
   stops two interleaved pushes regressing a row; the pending-deliveries list for the three pushes
   with no next bump.
5. **Hibernation and the picker (H-4)**: `SubPrompt` as a per-object row beside the record, on the
   socket attachment, or re-opened by the client — "doing nothing is not among the options".
6. **The Access configuration for the Worker** (R-6.5b2.6): multiplayer behind the door versus
   single player alone; whether the art is public; and whether the single-player paths stay
   invocation-free static assets.
7. **Whether the NAS keeps running as the self-host path in parallel** (R-6.5.1 says the axum binary
   stays as it is) — and if so, on which hostname and identity mode.
8. **What the e2e can prove against a Worker**: Miniflare / `wrangler dev` versus R-6.5.8's live
   setup, and which of the §4 M6.5c mutations each can catch.
9. **The panic policy inside an object (O-1)** and whether `getrandom` reaches `crypto` under
   `wrangler dev` (O-7) — both owed **before the plan is written**.

---

## 6. What NOT to re-derive (measured traps, paid for by M6.5b2 and M6.5b)

1. **Vite's manifest is not an inventory.** At this HEAD it names `assets/index-<hash>.js` and its
   stylesheet and **neither the module nor the Worker chunk**; a manifest-shaped fixture agrees with
   a manifest-shaped bug. The precache generator walks the finished tree and consults no manifest.
2. **Five Access applications are five audiences.** ONE application with several destinations, and
   the deploy **protects before it exposes**.
3. **Serving the game routes is not the same as offering multiplayer.** The browser host plays by
   calling the same `api::dispatch` routes; one flag for both would stop single player. Three
   transport routes also live on the axum ROUTER outside the layer's table.
4. **The service worker registers in every e2e cell unless the fixture turns it off**, because
   `build:client` is a production build.
5. **`storageState` cannot carry Cache Storage or a worker registration** — the close-and-reopen
   cell needs `launchPersistentContext`.
6. **A first CNAME lookup caches a miss for the SOA's negative TTL** (1800 s here): a new name can
   read NODATA on the owner's devices well after it exists.
7. **This workstation is WARP-enrolled**, so it cannot take an unenrolled reading; a lapsed Access
   session looks exactly like a broken deploy.
8. **An IndexedDB v1 tab has no `versionchange` handler** and caches its startup promise, so it can
   BLOCK a version bump; `onBlocked` on the page-side opens is a recorded LEAVE item.
9. **In a scripted edit batch, write after every edit or verify at the end** — a batch that fails
   partway can silently discard its earlier successful edits, and a green log is not a saved file.
10. **The vs-bot whole-game gate arm livelocks** on some seeds: a red gate there is READ before it
    is believed, and re-run alone at the same commit.
11. **Never two Playwright runs at once**, and **the orchestrator runs NOTHING in the tree while an
    implementer works** (owner ruling, close §5.1); a dead agent is WOKEN by message, never replaced
    by a fresh spawn, and a reviewed SHA is never amended.
12. **A headed live review beats four headless ones** (M6.4 close §4.5): the first headed pass found
    a 404 on every screen four headless passes had called clean.

---

## 7. Process

The same as M6.5b2's: a brainstorm with the owner, a spec reviewed adversarially before the plan, a
plan in dependency order ending in the spec's own cells, execution under R-P11 as refined
2026-09-20 and PF-1 (subagent-driven with Opus the floor unless a stage is very trivially
mechanical), a whole-branch review with one fix wave, a mutation battery mapped to named catchers at
the reviewed head, an EARLY and repeated deploy per R-6.5.8 rather than one at the end, a headed
live review, and a close on this cycle's shape whose §7 starts from the spec's and is corrected by
what execution measured.
