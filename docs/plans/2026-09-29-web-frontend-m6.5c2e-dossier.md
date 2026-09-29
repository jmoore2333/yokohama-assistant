# Cycle E — the finish: the dossier

**What this is.** The file:line facts cycle E's plan (`docs/plans/2026-09-29-web-frontend-m6.5c2e-plan.md`) is written
from, compiled read-only on 2026-09-29 by an Opus researcher. Each plan task cites its items by number ("dossier item
N") for WHERE and NOW, and carries the design itself. Line numbers are readings at `e93ab965`: **find by content once
a task before yours has moved a file.** A dated artifact, frozen; corrections go forward in the plan's ledger and the
close.

Compiled read-only at `e93ab965` on `web-room-m6.5c2e` (the code is cycle D's reviewed head `4212f6a0`). Nothing was
run. One outside source was read: Cloudflare's documentation (item 3). Paths are relative to the repository root. Each
entry: Where · Now · Fix touches (corpus / `types.ts`–`contract.ts` / pool / e2e) · Covered by · Premise — kept to
what the tasks need.

## Summary

- **Entries:** 27 items (Tasks 1–8; the polish rows are item 15), Part 2's instruments, Part 3's documents, the
  variable inventory (item 26) and the route list (item 27).
- **Premises FALSE at HEAD:**
  - **R-6.5c2.41's "`.bin` is not on the edge's default cache list" (item 3).** Cloudflare's default cached extensions
    include `BIN`, and a 200 with no `cache-control` or `expires` stays at the edge for 120 minutes. The NAS host is
    behind the zone's cache (`deploy/RUNBOOK.md` recorded `cf-cache-status: HIT` at :1670 and :1915), and
    `assets.rs::cache_control` gives `/shell.bin` no header.
  - **R-6.5c2.42's "E adds the cell that a finished table can be abandoned" (item 21).** Both hosts hold one already:
    `tables.rs:2984` and `archive.test.ts:892`.
  - **Item 10.** No comment names a review FILE; about thirty D-written comments cite D's reviews by label.
  - **Item 20.** The table's `status` is not readable when the expiry is scheduled (the strings rule), and the
    Worker's TTL is not on `/api/health`, since the module's `init` configuration does not carry it.
- **Partly false or drifted:** items 1, 4, 11, 13, 18, 23, 25, and several Part 2 and Part 3 lines; each entry's
  Premise says how.
- **The riskiest item: items 1–3 together.** A `/shell.bin` with no `Cache-Control` is kept by the NAS host's edge for
  up to 120 minutes after a deploy, so every install in that window fetches the previous shell and fails the digest
  check — today's defect in a new form. `cache: "reload"` bypasses the browser's HTTP cache, not the edge. A fix needs
  a header for `/shell.bin` in `assets.rs::cache_control` (and `worker/static/_headers`), or an extension off the
  list, proven by an update reading on the NAS inside two hours of a deploy. Runners-up: item 20 (a health field meets
  `HOST_FIELDS` in two replays); item 12 (a footer on every screen meets the ≥1201 px `.app` grid of `height: 100vh`
  and the fit rule); item 24 (the guard names `wrangler.jsonc` at five sites).
- **Where items meet:** the corpus moves in Task 2 (a constructed house frame, both exact-set lists), Task 5 (a
  `TableView` field, 14 files) and Task 6 (`health.json`, `HOST_FIELDS`) — sequence them, each regenerates it.
  `api.rs` holds `Health` (Task 6) and `routes()` (Task 8's list; R-6.5c2.45's map cell). `Lobby.tsx` is touched by
  Tasks 3, 4a/4b, 5 and 7; `App.tsx` by 3, 4b, 5 and 7; `styles.css` and `setup_screens.spec.ts` by 3, 4a and 4b.
  `log.rs`'s event cell is Task 2's.

## Task 1 — the shell precached from a static non-HTML asset

### Item 1 — the precache generator and its one reader
- **Where:** `web/scripts/precache.mjs:1-174`; the pure half `web/src/precache.ts:1-418`; the reader
  `worker/scripts/precache-list.mjs:1-46`.
- **Now:** The generator walks `dist` minus `sw.js` (`precache.mjs:91`), adds the art and its manifest, and calls
  `buildPrecache` (`precache.ts:296-372`), which refuses no `/index.html` (`need("index.html", …)` :326), no `.wasm`,
  script, stylesheet or art, a bypassed or duplicate path, or a ceiling. An entry is `{ path, digest, type }` with
  `type = expectedType(path)` (:199-235; the default arm `application/octet-stream` :232, mirrored from `assets.rs` by
  a cell). Ceilings: module 1,400,000 (:126), debug module 3,200,000 (:135), total 11,900,000 (:152), the debug total
  plus the module's 1,800,000 (`ceilingsFor` :160-178). The name is `yokohama-release-` + the first 16 hex of a
  SHA-256 over every `path digest` line (:368-369). `sw.js` is esbuild's bundle of `src/sw.ts` with
  `define: { __CACHE_NAME__, __PRECACHE__ }` (`precache.mjs:125-141`), checked by `checkSubstitution`
  (`precache.ts:396-418`), written at :151. The ONE reader `readPrecacheList(swText, "built" | "deployed")`
  (`precache-list.mjs:39-46`; regex `ENTRY` :12 requires `path`, a 64-hex `digest` and `type`, D's ruling 28) serves
  `assemble-assets.mjs:42-44` and `read-deploy.mjs:54-56`.
- **Fix touches:** `precache.ts` (the need; a `/shell.bin` entry on the default arm) and `precache.mjs` (write
  `dist/shell.bin` from `dist/index.html`, and filter it out of the walk as `sw.js` is: `offline.spec.ts:820` and
  :1441-1447 re-run the generator over its own output, which would list it twice). No corpus, types or pool fixture.
- **Covered by:** `precache.test.ts:52-148` (:125-128 asserts `/index.html` → `text/html`), :150-187 (:163 refuses by
  `/names no index\.html/`), :209-240, :277-343; `precache.node.test.ts:78-147` (`/index.html` :108, 131 art files
  :137, 140–200 entries :143-144).
- **Premise:** PARTLY — the reader is `worker/scripts/precache-list.mjs`, not `web/scripts/`.

### Item 2 — the worker's install and its navigation answer
- **Where:** `web/src/sw.ts:80-82`, :113-176 `installRelease`, :178-213 `installOne`, :229-237 `tellTheClients`,
  :338-366 the fetch handler, :375-383 `fromCache`, :404-424 `fromRetained`; `web/src/swRoute.ts:48-55`, :83-94;
  `web/src/swInstall.ts:49-51`, :62-71 `checkHead`, :84-86 `checkDigest`; `web/src/precache.ts:105`
  (`META_PATH = "/__release__"`); `web/src/OnThisDevice.tsx:244-266`.
- **Now:** Each entry is fetched with `cache: "reload"`, `redirect: "error"` and `credentials: "same-origin"`
  (:191-196); `checkHead` wants exactly 200 and the type's essence equal to `entry.type`; `checkDigest` words a
  mismatch `is <actual>, not the release's <expected>`. The body is stored with the SERVER's headers
  (`new Response(body, { status: 200, statusText, headers: response.headers })`, :202-205). A failure posts
  `{ yokohama: "install-failed", reason }` (:232), read at `swRegister.ts:237`; the door draws "Could not prepare to
  play offline — {reason}. Try again" (`OnThisDevice.tsx:258-265`), and "Ready to play offline" with NO period (:257).
  A navigation (`swRoute.ts:91`) is `fromCache("/index.html", request)` (`sw.ts:364`), which returns the stored
  response as it is, else the network.
- **Fix touches:** `sw.ts` — the navigation answer must build a `Response` over `/shell.bin`'s body with a
  `text/html; charset=utf-8` type it sets itself (the stored type will be `application/octet-stream`). A
  non-navigation GET of `/index.html` then routes to `fromRetained`, which can answer it out of a retained D-release
  generation.
- **Covered by:** `sw.node.test.ts:286-404`, :674-681 (a navigation answered with the shell — `.text()` only; no cell
  asserts its content type), :872-880, :887.
- **Premise:** TRUE.

### Item 3 — how each host serves the files, and the edge in front
- **Where:** NAS `crates/yokohama-web/src/assets.rs:50-78` `content_type`, :104-109 `cache_control`, :136-180
  `serve_path` (the SPA fallback :167-178); `lib.rs:847-864` `fallback`; `art.rs:125`. Worker
  `worker/wrangler.jsonc:36-42` (`html_handling none`, `not_found_handling none`,
  `run_worker_first ["/api/*", "/enter"]`), `worker/static/_headers:1-7`, `worker/static/_redirects:1-2`.
- **Now:** NAS: `.bin` is `application/octet-stream` (:77); `cache_control` gives `/sw.js` `no-store` and `/assets/*`
  `public, max-age=31536000, immutable`, and every other `dist` path no header — `/index.html` and a `/shell.bin`
  alike; `/art/*` carries `public, max-age=3600`. Worker: `_headers` restates those three rules; `_redirects` serves
  `/` as `/index.html`; no shell code names `/index.html`, `/sw.js` or `no-store`.
- **Fix touches:** `assets.rs::cache_control` and its cell
  `hashed_assets_are_immutable_the_worker_is_no_store_and_nothing_else_carries_a_header` (:246-280, which asserts no
  other path carries a header); `worker/static/_headers` (check 9 compares the assembled copy).
- **Premise:** FALSE for the edge. Cloudflare's default cached extensions list `BIN`, cached 120 minutes absent
  `cache-control`/`expires` (developers.cloudflare.com/cache/concepts/default-cache-behavior). The NAS host is behind
  the zone's cache; the Worker host should not be (a Worker is zoneless and the extension list does not apply, per the
  Workers cache docs; unverified live). The same rule already covers the NAS's stable-named precached files with no
  header (`/icons/*.png`, `/help/*.webp`). The zone's Cache Rules were not read.

### Item 4 — the assembly, guard checks 8 and 9, and the deploy readings
- **Where:** `worker/scripts/assemble-assets.mjs:42-63` (`precacheEntriesFrom`, `verifyTree`), :80+ `debugModuleIn`;
  `worker/scripts/deploy-guard.mjs:60-74`, :293-312 `staleModule`, :322-343 `staleAssets`;
  `worker/scripts/read-deploy.mjs:110-126` `checkEntry`, :176-214 `assets`, :263-315 `install`.
- **Now:** The assembly verifies every entry's SHA-256 against the assembled tree. Check 8: `gen/source.json` equals
  `sourceDigest(repo)`. Check 9: the tree's `sw.js` is `web/dist/sw.js` byte for byte, and each `worker/static` file
  equals its copy. `assets` allows an edge-rewritten HTML body only for `text/html` entries whose digest survives
  cutting the injected scripts (:118-124, the `EDGE` lines :182-196), and HOLDS with no wrong entry, `/index.html` at
  200 and `/assets/x.js` a 404 with an empty body. `install` HOLDS only when the worker is `activated`, nothing is
  missing, `cache.match("/index.html")` digests to the list's `/index.html` entry (:284, :302), and
  `innerText.includes("Ready to play offline.")` — WITH a period (:296, printed :309).
- **Fix touches:** `read-deploy.mjs:296`, :309 (the period) AND :284, :302 (`/shell.bin`, or no release can HOLD). The
  assembly and the guard name no `/index.html`; check 2 verifies `shell.bin` as an entry.
- **Covered by:** `assemble-assets.test.mjs:42-115`; `deploy-guard.test.mjs:150-530`; `read-deploy.test.mjs:28-44`,
  :81-128 (fixtures name `/index.html`). No cell covers `install`.
- **Premise:** PARTLY — the period is one of three edits.

### Item 5 — the e2e over the service worker
- **Where:** `web/e2e/offline.spec.ts:163-178` (`installed()`: "Ready to play offline", then
  `cacheHas(…, "/index.html")` at :172); cells :300 (installs, comes back offline, plays a game), :354, :445 (the
  update), :633, :717 (re-runs the generator at :820), :1280, :1352 (retries to "Ready to play offline", :1335,
  :1394), :1425.
- **Fix touches:** :172; the comment at :611 ("`scripts/precache.mjs` writes only `dist/sw.js`") turns false.
- **Premise:** TRUE; cells :300 and :354 catch a navigation served with the wrong type offline.

### Item 6 — the images and CI
- **Where / Premise:** `deploy/Dockerfile:222-224` and `deploy/Dockerfile.runtime:40-42` copy the WHOLE `dist`;
  `.github/workflows/web.yml` names no precache file. TRUE — `shell.bin` travels. The NAS image is built from the
  workstation's `web/dist` (`nas-build.sh`, Part 2).

## Task 2 — D's leftovers

### Item 7 — the `bots.rs` clause
- **Where:** `crates/yokohama-web/src/bots.rs:414-420`, in the cell at :379-429; `label` :82-87;
  `distinguishing_labels` :293-314.
- **Now:** It says knobless descriptors reach Row 4 "only when one controller is unparseable".
  `RolloutPolicy::from_str_name` accepts `"retreat_averse"` and `"family_weighted"`
  (`crates/yokohama-core/src/game/policy.rs:143-144`, read, never edited), so two PARSEABLE descriptors reach it.
- **Premise:** TRUE. One comment line.

### Item 8 — the house seed's silent `Err` arm, and the log seam
- **Where:** `crates/yokohama-web/src/lib.rs:464-505`, the seed :482-493; `tables.rs:1161-1163` `house_counts`;
  `signal.rs:165-182` `seed_house`; `log.rs:33-37`
  (`trait Log { fn line(&self, event: &'static str, fields: Value) }`), :64-77 (five constants), :81-90 `StderrLog`.
- **Now:** `if let Ok(counts) = games.house_counts() { signals.seed_house(counts); }` (:491-493) — an `Err` says
  nothing. `Games` was built `.with_log(Arc::new(log::StderrLog))` at :480. To copy:
  `self.log.line(crate::log::OVER_TABLE_GONE, serde_json::json!({ "id": id }))` (`games.rs:1037`); field values are
  strings.
- **Fix touches:** `lib.rs`, `log.rs` (a sixth constant renames `the_events_are_five_names_and_none_is_the_shells`,
  :116-137; `the_layer_prints_only_through_its_log`, :144-176, refuses an `eprintln!`).
- **Covered by:** `lib.rs:7403` (the `Ok` arm). **Premise:** TRUE.

### Item 9 — the house frame and a constructed fixture
- **Where:** `crates/yokohama-web/src/signal.rs:81-91` (`house_frame(seq, chat, rev)` → `{"seq":…,"chat":…,"rev":…}`),
  pinned at :372-378 and `crates/yokohama-worker/src/house.rs:614-618`. Hand-built frames `web/src/Lobby.test.tsx:586`
  (`{ seq: 1, chat: 0 }`, NO `rev`), :794, :798, :845, :849, :920, :955 (the reason :769-775); the parse
  `Lobby.tsx:106-116`. The corpus: `crates/yokohama-web/tests/wire_fixtures.rs:2150-2325` `constructed()`, :2361-2380
  `corpus()` (contracts pushed :2366-2378); the exact-set lists `web/src/local/corpus.node.test.ts:149` `NOT_SERVED`
  (29) and `worker/test/corpus.test.ts:96` `THIRD_COLUMN` (17); `web/src/wire/contract.ts` (imports :112-125, `KEYS`
  :788+). `types.ts` has no house-frame type.
- **Fix touches:** a named corpus move: `house_frame.json` from `yokohama_web::signal::house_frame`, in BOTH lists
  (ruling 31), typed in `types.ts` and `contract.ts`; the Lobby cells re-pointed.
- **Premise:** TRUE; the frame at :586 predates `rev`.

### Item 10 — comments citing untracked reviews
- **Where / Now:** the task's grep (`task-N-review`, `final-review.md`, `fix-wave-report` …) outside `docs/plans` and
  `.superpowers` finds NOTHING. Comments cite reviews by label; those written in D (by `git blame` into
  `014433df..4212f6a0`), with the section of the close `docs/plans/2026-09-29-web-frontend-m6.5c2d-close.md` to cite:
  - "the final review's C1" — `crates/yokohama-web/src/lib.rs:483`, :7388; `signal.rs:38`, :125, :166;
    `tables.rs:1155` → §4 and §5 ruling 46. "the final review's triage" — `lib.rs:4119`;
    `web/src/OnThisDevice.tsx:138`; `OnThisDevice.test.tsx:207`, :298 → ruling 45. "whole-branch review, Minor 1" —
    `web/src/local/db.ts:413` → §4.
  - "the Task 11 review's I-1" — `web/src/tables.ts:195`; `Lobby.test.tsx:477`; `OpenTable.test.tsx:535` → ruling 34.
    "m-2" and "I-3" — `App.tsx:753`; `Dealing.test.tsx:568` → §1's Task 11 row. "the Task 11 re-review's lost-message
    finding" — `Chat.tsx:391`; `Chat.test.tsx:387`, :395, :434; `Lobby.test.tsx:859`; `Lobby.tsx:245` → ruling 35.
  - Task 5's "the re-review measured …" — `games.rs:2514`, :2517 → rulings 19–20. "re-review of the fix wave" (Task 7)
    — `tables.rs:1186` → §1's Task 7 row.
  - Moved in D, written earlier: `worker/src/table/delivery.ts:88`, `worker/src/table/sockets.ts:38`,
    `worker/scripts/replay/judge.mjs:111`, `crates/yokohama-web/tests/browser_played_pin.rs:269`. About 200 older
    label citations remain (e.g. `web/e2e/live.ts:281`, `web/src/FrontDoor.test.tsx:45`).
- **Premise:** FALSE as grepped; about thirty D-written label citations.

### Item 11 — the hostname cell and the four `expect("a game")`
- **Where / Now:** `web/src/ws.test.ts:44-51` (the site's hostname at :49-50).
  `crates/yokohama-worker/src/picker.rs:368`, :377 and `receipt.rs:396` are production; `receipt.rs:1173` is inside
  `mod tests` (from :496). Each is `handle.lock().expect("a game")` on a single-threaded instance.
- **Premise:** PARTLY — the cell is in `web/src/`, not `worker/src/`; one site is a test.

## Task 3 — the door's three build values

### Item 12 — the build value, the door, and where a footer mounts
- **Where:** `web/scripts/project-url.mjs:15-39` (`PROJECT_URL_VAR`, `PROJECT_URL_MAX = 200`; `checkProjectUrl`: unset
  or empty is `null`; over 200, whitespace or a control character, no `http(s)://` start or no host is refused naming
  the variable); `project-url.d.mts:1-8`; `web/vite.config.ts:24-36` (`define: { __YOKOHAMA_PROJECT_URL__: … }`;
  `vitest.config.ts` merges it); `web/src/vite-env.d.ts:37` (the global; `VITE_MULTIPLAYER` :27);
  `web/src/projectLink.ts:9-21`. The door `web/src/FrontDoor.tsx:49-71` (props; `link` :68-70), :89-111 (`.door-title`
  :91, `<h1>Yokohama</h1>` :92, the link :93-107). `App.tsx:1518-1525` (`link={projectLink()}`), the game screen's
  `.app` :1987-1991 and `header.banner` :1992; `web/src/main.tsx:16-18`. The lobby `Lobby.tsx:498-499` (`LobbyBand`),
  :517-528 (`.lobby-actions`). CSS `styles.css:1946-1966` (the hero; `.project-link` 12.5px `--fg-dim`), :2081 (≤600px
  title), :2523 (`.project-link` 44px), `.muted` :156, `.section-hint` :2176. No footer class exists.
- **Fix touches:** the checker, `vite.config.ts`, `vite-env.d.ts`, readers, `FrontDoor.tsx` (the blurb between title
  and link), `Lobby.tsx`, a footer. **`App` has about fifteen `return` sites (:1576-1958) and no wrapper**: "every
  screen" is a mount in `main.tsx` or one per return. At ≥1201px `.app` is a grid, `height: 100vh`, rows
  `auto auto auto minmax(0, 1fr)` (`styles.css:228-245`), and the fit rule reads `.table`'s height (`100cqh`,
  :332-333): a footer inside adds a grid item; outside, the page passes the viewport.
- **Premise:** TRUE. The door's `/enter` link is `FrontDoor.tsx:148`.

### Item 13 — where the value is set and documented
- **Where / Now:** `deploy/Dockerfile:194-200` (`ARG` :199); `.github/workflows/docker.yml:99`;
  `deploy/README.md:119-135`; `deploy/RUNBOOK.md:2224` (§3.18: the build ran with it set); `worker/RUNBOOK.md:1697`
  (§18); `web/package.json:11-12`. `worker/RUNBOOK.md` §1 (:90-121) gives `cd web && npm run build` with no variable,
  and `worker/README.md` never names it.
- **Premise:** PARTLY — no Worker document states it in §1–§2.

### Item 14 — the cells and the width rules
- **Where:** `web/src/projectLink.test.ts:12-25`; `web/scripts/project-url.node.test.ts:9+`;
  `web/src/FrontDoor.test.tsx:115-128`. `web/e2e/setup_screens.spec.ts`: :50 the door at 1440 and 390 (`DESKTOP`
  1440×900, `PHONE` 390×844, `harness.ts:870-872`); :100-137 the title never meets the board at 861 and 700 px (text
  measured by `Range`); :176, :210 (44 px reads :370-374), :434 (the open table and device card; exemption :505-516);
  `noSidewaysScroll` `harness.ts:876`. The fit oracle `web/src/fit.ts:19-31` (`byH` from the `.table` container's
  height), used at `the_table.spec.ts:89-99` and `the_screen_itself.spec.ts:1187`.
- **Premise:** TRUE. A blurb enters the 861/700 px cell's text box.

## Tasks 4a and 4b — the owner's-eye polish (WHERE and NOW only)

### Item 15 — the rows
- **Row 58.** `styles.css:922-926` (`button.primary`: padding `0.45rem 1.1rem`, color `#061019`) and :1810-1814
  (`a.primary-link`: `inline-flex`, padding `0.5rem 1.05rem`, color `var(--bg)`); both widened at ≤600px (:2083-2085,
  :2158-2160).
- **Row 59.** `FrontDoor.tsx:120` (`two-up` when `VITE_MULTIPLAYER=false`, state E); `styles.css:1973-1974` (`.two-up`
  1fr 1.3fr, `max-width: 61rem` = 976 px; no literal 976), :2059-2061. Mocks
  `docs/plans/2026-09-26-web-frontend-m6.5c2-mocks/06-door-multiplayer-off.webp`, `05-identity-states.webp`.
- **Row 60.** `web/src/Band.tsx:67-88` (`LobbyBand`), :101 `IdentityCorner`; `styles.css:1835-1856`, :1872-1878 (the
  pill), :1923-1937 (≤600px: crumbs 16px, pill 36px), :2521 (pill 44px).
- **Row 61.** `web/src/Dashboard.tsx:198-210` (`.seat-head`: colour, "the bot `<VersionTag>`", "seat N", then `¥` and
  VP chips); `Chips.tsx:68-72`; `styles.css:1896-1900` (`.version-tag` in `--yen`), :538 (`.chip.yen`, the same gold).
- **Row 62.** `OnThisDevice.tsx:300-343` (`StorageLine`, ending " Back them up to hold a copy you can restore."),
  :178-191 (the foot), :386-388 (the iOS note); `styles.css:955-965`, :1998. The hues: `web/src/BoardMark.tsx:36-47`
  (customs `imports`, laboratory `faint`, employment agency `neutral`), `styles.css:1820-1828`.
- **Row 63.** `Lobby.tsx:612-624` (`<details className="fold">`); `styles.css:2344-2348` (the idiom), :2548-2587 (the
  Finished overrides; marker `--fg-dim`).
- **Row 64.** `Lobby.tsx:406-466` (`row()`; `.row-abandon` :460-462); `styles.css:2181-2218`
  (`.row-abandon button.ghost` `min-height: 24px`), :2226-2233 (≤600px). The device row `OnThisDevice.tsx:201-214`,
  `styles.css:1985-1997` (padding 0.45rem, 12px). The sweep's exemption `setup_screens.spec.ts:505-516`.
- **Row 66.** `styles.css:2514` and :2520 (both ≤600px; comment :2508).
- **Row 67.** `OpenTable.tsx:368-383` (the chat in the MAIN column, "not mock 08's rail"); `styles.css:2391-2396`;
  mock `…/08-waiting-for-players.webp`.
- **Row 68.** `aria-live="polite"` on the copy BUTTON whose text changes: `Lobby.tsx:438-448` (its `aria-label` also
  changes) and `OpenTable.tsx:457-460`. Other regions: `App.tsx:2055`, :2193, :2228, :2386, :2402 (`status`), :2435
  (`alert`); `Lobby.tsx:523`; `OpenTable.tsx:218`; `Dealing.tsx:263`, :354; `Abandon.tsx:266`; `LockScreen.tsx:98`;
  `Chat.tsx:505-506` (`log`).
- **Row 71.** `styles.css:160-185` (`.banner` wraps), :1052-1054, :1063-1064; `App.tsx:2054-2058` (the word, only when
  not `live`). Gate `the_screen_itself.spec.ts:969` ("one row at 1440 and 1280"; 1024 out, :952-966).
- **Row 72.** The who-strip survives on the deal screen alone (`Dealing.tsx:304`; `styles.css:1038-1042`, :1510-1512
  wraps at ≤480px; the band is `docs/plans/2026-09-19-web-frontend-m6.4-close.md:866-869`). Placeholders
  `NewGame.tsx:271`, :314, :494; `Index.tsx:295` (CC-2 :873-874).
- **"You are ben" against "Ben".** `App.tsx:1446` (`signedIn = door.me?.user`, the parsed key), :1494-1502
  (`whoStrip`); the seat card `SeatCard.tsx:33-38` fed at `OpenTable.tsx:344` from the seat's typed `name`
  (`api.ts:169-179`, `yokohama.display_name`).
- **Row 21.** Through `Refusal` (`web/src/Refusal.tsx:30+`, class `error`, no role): create `NewGame.tsx:441`, :555
  (cell `NewGame.test.tsx:449`); deal `Dealing.tsx:310`, :476 (`Dealing.test.tsx:510`); chat `Chat.tsx:533`
  (`Chat.test.tsx:493`); open table `OpenTable.tsx:235`, :362, :402 (`OpenTable.test.tsx:248`). `Refusal.test.tsx:54`
  fences a bare `<div className="error"`.

### Item 16 — the picture instruments
- **Where / Now:** `web/e2e/gates.spec.ts` (`YOKOHAMA_WEB_GATES=1`; writes `../docs/temp/web-review/gates`, :25;
  viewports 1440×900, 1710×869, 2560×1440, :26-30; the tiles, the table, the set table, the opponent's turn — :52,
  :166, :333, :443). `web/e2e/shots.spec.ts` (`YOKOHAMA_WEB_SHOTS=1`; 1710×869 :76; `web/help-shots/*.png`, then
  `python3 docs/tools/mkhelpart.py` cuts `web/public/help/*.webp`, :10-20; screens :178, :272). Neither shoots a setup
  screen: those are `look(page, …)` in `setup_screens.spec.ts` (:98, :135, :377, :396).
- **Premise:** TRUE.

## Task 5 — paging, "Abandon and save", the arrival

### Item 17 — lobby paging
- **Where:** `web/src/Lobby.tsx:91-101` `fetchLists` (three scopes, no cursor), :218-226 (`paged`), :342-355
  `page(cursor)` (`api.list("mine", cursor)` only), :468-476 (`nextCursor = paged ? paged.next : lists.mine?.next`),
  :558-562 ("more" under Your tables only); `web/src/api.ts:868-877`. The layer
  `crates/yokohama-web/src/tables.rs:713-743` `Scope` (`Playing` = playing or over), :754-755 (limits 20 and 100),
  :762-764 (`<created_at>,<id>`), :1741-1805 `list` (newest first, then `id`; strictly after the cursor; `next` only
  when a row is left); `api.rs:797-850` (`ListQuery` with `deny_unknown_fields`, `GamesList { games, next }`,
  `list_games`).
- **Now:** Every scope's reply carries `next`; only `mine` is paged. Being played and Finished split ONE `playing`
  scope by `over` (:473-475).
- **Fix touches:** `Lobby.tsx` and its cells; no server or corpus change. List files: `games.json`, `games_next.json`,
  `games_open.json`, `games_playing.json`.
- **Covered by:** `Lobby.test.tsx:210`, :237. **Premise:** TRUE; paging Being played also pages Finished.

### Item 18 — "Abandon and save"
- **Where:** `web/src/Abandon.tsx:94`, :112-115
  (`saves = archiveDisposition(archive) === "send" && table.status === "playing" && (local || !isLocal(table.id))`),
  :250-257 (the label; "would move eight fixtures"). `TableView` `crates/yokohama-web/src/table.rs:504-555` (18
  fields: `id`, `name`, `mine`, `creator`, `created_at`, `players`, `colors`, `start_seat`, `seats`, `chosen`,
  `my_seats`, `status`, `to_move`, `over`, `locked`, `winner`, `can_watch`, `bot`), built only by `TableDoc::view`
  (:409-436) from the document, no record read; `TurnMirror` :94-117 (`to_move`, `over`, `winner`, `bot`; no event
  count); `archived_at` :245, not in the view. `web/src/types.ts:573`; `contract.ts:658-662` (`TABLE`). The layer's
  rule `tables.rs:1884-1935`: saved iff the archive is on, `status == Playing`, no `archived_at`, and the record has
  an event.
- **Fix touches:** a named corpus move over 14 files (`deal.json`, `deal_waiting.json`, `deal_bot_table.json`,
  `games.json`, `games_next.json`, `games_open.json`, `games_playing.json`, `table.json`, `table_open.json`,
  `table_dealing.json`, `table_watchable.json`, `table_bot.json`, `table_foreign.json`, `table_two_open.json`),
  `types.ts`, `TABLE`, `Abandon.tsx`; a computable fact needs the mirror (refreshed by `persist`, conflicts ignored)
  or `archived_at`.
- **Premise:** PARTLY — `TableView` is in `table.rs`, not `api.rs`; 14 files, not eight.

### Item 19 — the arrival by link
- **Where:** `web/src/route.ts:45-57`; `web/src/App.tsx:820-860` (the route effect → `open(route.id)` once `door.me`
  is known), :654 (`open` awaits `api.table(id)`), :1722 (`!signedIn` → the lobby), :1810 (no `state`, `dealing`,
  `openTable` or `unlock` → `lobbyScreen(error)`).
- **Now:** A cold `#/t/<id>` mounts `<Lobby>` for one round trip: three listings and a `/api/stream` open. M6.3 close
  (`docs/plans/2026-09-16-web-frontend-m6.3-close.md:635-640`): "every arrival at `#/t/<id>` by link or reload draws
  the LOBBY for one round trip, opening the house stream and firing three listings that are thrown away … `App`'s
  screen selection falls through to `<Lobby>` while `open(id)` is still awaiting `GET …/table`. Fix: render "opening…"
  while a table route is pending." (§6.2 restates it at :818-821.)
- **Covered by:** `web/e2e/joins_by_link.spec.ts:75` (the one test); no cell asserts the lobby's absence. **Premise:**
  TRUE.

## Task 6 — the waiting-table expiry and the finished-table abandon

### Item 20 — the Worker's expiry, the marker and health
- **Where:** `worker/src/table.ts:251-263` (`onAlarm`), :842-853 `expiryDue` (`ttlDaysOf(this.config)`,
  `lastActive(sql)`, `expiryDeadline`, the backoff), :803-823 (`batch`: one `transactionSync`, then `arm` →
  `setAlarm`). `worker/src/table/lifecycle.ts:255-290` `expire` (the receipt loop over `m.expire(id)`, logged
  `expired`), :314-344 `expiry`, :356-370 `unexpired`, :537-555 `copyFacts` (the ONLY table parse, off
  `mod.house_copy(id)`). `delivery.ts:244-250` (`lastActive` = `max(written_at)` over `record`, `table`, `chat`).
  `worker/src/marker.ts:25` (`Reason`), :35-40 (`TAILS`), :48-62 (`markerSentence`: "this table expired on
  {YYYY-MM-DD} after {N days} without activity" + tail), :75-83 (`ttlDaysOf`, the ONE reader, ruling I-34); the
  marker's 404 at `lifecycle.ts:40`, :390. The module `crates/yokohama-worker/src/receipt.rs:473-494` `expire_inner`
  (a `DELETE` as the stored `created_by`; no creator → the layer's error or a 500), exported at `lib.rs:473-476`.
  Config `worker/src/config.ts:12-24`, :37-51 (`ROWS`, held to `Env`), :125-134 (`ttlDays`:
  "YOKOHAMA_WEB_TABLE_TTL_DAYS must be a whole number of days above zero — got …"), :262-273 (`moduleConfig`:
  identity, archive, caller, the chat variables — no TTL); `env.ts:61`; `wrangler.jsonc:89-92` and both pool files :42
  (`"90"`). Health: the layer's `Health` `api.rs:310-359`, `health()` :361-373 (ten fields), which the Worker's house
  object answers; `web/src/wire/health.json`; `contract.ts:653-656`; `types.ts:480`. `HOST_FIELDS`:
  `worker/scripts/corpus-walk.mjs:19` (`health: ["identity", "signal", "archive"]`, the pool and the live replay) and
  `web/src/local/corpus.node.test.ts:103-104` (`["identity", "signal", "multiplayer"]`); each refuses an exemption the
  hosts agree on (:296-301). Docs: `worker/RUNBOOK.md:178`, :227, :245; `worker/README.md:1333`, :1594;
  `deploy/README.md` has none.
- **Now:** One TTL from the last write, whatever the status; `expiryDue` runs off SQL at every batch and never sees
  `status` (the strings rule).
- **Fix touches:** `config.ts` (`Env`, `ROWS`, `ShellConfig`, a refusal); `marker.ts` (a second reader and a head);
  scheduling that knows the status without the module (a meta row in the batch, or `copyFacts` at the alarm); if
  reported, `moduleConfig`, `Health`, `health.json` (the NAS reports none), `HEALTH`, `types.ts` and
  `corpus-walk.mjs`'s `HOST_FIELDS`; `wrangler.jsonc`, the pool files; the RUNBOOK table and both READMEs.
- **Covered by:** `worker/test/expiry.test.ts:117-444` (:176 an idle table expires as its creator's abandon; :426 no
  TTL), :445-555 (the marker); `marker.unit.test.ts`; `config.unit.test.ts`.
- **Premise:** PARTLY FALSE — `expire_inner` is in `receipt.rs`; `status` is unreadable at scheduling; the TTL is not
  on health.

### Item 21 — abandoning a FINISHED table
- **Where:** `crates/yokohama-web/src/tables.rs:1870-1970` `delete_locked` (the archive block :1893-1935 is skipped
  when `archived_at` is set); cells :2886, :2922, :2984-3007
  (`abandoning_an_open_dealing_moveless_or_archived_table_archives_nothing`: "Finished — archived at over (one put) —
  then abandoned: no second put"). Pool `worker/test/archive.test.ts:892` (the refinish race: a 204, no archive line,
  nothing under `abandoned/`, :948-956). Client `web/src/tables.ts:202`
  (`mayAbandon = t.mine && t.my_seats.length > 0`, any status); the lobby row `Lobby.tsx:460-462`; the game screen's
  Abandon is local-only (`App.tsx:2206-2208`).
- **Premise:** PARTLY FALSE — both hosts' cells exist; missing are a plain finished-and-stamped pool cell and the live
  read.

## Task 7 — requests per move

### Item 22 — what a frame fetches
- **Where:** `web/src/App.tsx:1140-1200` (`arrive`: an unchanged `events` fetches nothing; else `api.get(game)`
  :1158), :487-500 (`landed`), :89-97 (`fetchRun`: `api.run` per `RUN_CAP` page, then `api.log`), :1372-1387
  (`useSignal`); `web/src/TableSections.tsx:87-104` (per `state` change, not while `scrubbing`: `api.ports`,
  `api.labs`, `api.areas`); `web/src/signal.ts:354-357` (a `"value"` frame is handed over; a `"wake"` frame calls
  `peek`, then `once()`), :536-559 `useSignal`; `web/src/ws.ts:38` (`PING_MS = 30_000`); `Chat.tsx:345-348` (reads
  only when `count > last`, unfolded); `Lobby.tsx:328-340` (`once: fetchLists`, three listings per house frame; every
  move bumps the house); `OpenTable.tsx:149-165` (`api.table` per frame).
- **Now / Premise:** TRUE. A watcher's move costs the state, the run, the log and up to three sections; an open lobby
  adds three listings per move anywhere (D's live reading: about 13 a move, the E handover §2).

### Item 23 — the counting instruments
- **Where / Now:** `web/e2e/two_browsers.spec.ts:115-131` and :200-219 (stream opens and `/events` asks, by the
  table's whole path). No per-route tally with timestamps exists in `web/e2e/`; the axum host logs no request
  (`lib.rs:905-925`, no tracing layer). Live: `worker/RUNBOOK.md:720-727` (the Observability query API: a total is a
  `calculations` `count`; a line is found by `needle`), :679 (the quota guard), :1793-1806 (§18's counts).
- **Premise:** PARTLY — the listeners are in the spec, not `harness.ts`; a per-10-second, per-route tally is new work.

## Task 8 — the door window's code side

### Item 24 — the configurations and the guard
- **Where:** `worker/wrangler.jsonc:10-97` (route :18; `workers_dev` :23 and `preview_urls` :24 false; `assets`
  :36-42; objects :47-54; migration :55; bucket :60; `vars` :70-97). `worker/wrangler.open.test.jsonc:16-46` differs
  by its name, no route, no `observability`, `assets.directory ./.test-assets`, and `vars` of the door `open`, archive
  `r2`, TTL `90`, `YOKOHAMA_WEB_TEST_CLOCK` and `YOKOHAMA_WEB_COUNT_WAKES` (no Access pair, no dev door, no chat
  variables). `worker/scripts/deploy-guard.mjs:1-78` (nine checks), :180 `POOLS`, :248-282 `doorRules` (check 3),
  :346-368 (checks 3–7), :421-437 (the CLI). It names `"wrangler.jsonc"` at :187, :286, :348, :354, :393 and takes no
  path; `worker/package.json:18` runs `wrangler deploy` with no `--config`. `worker/src/config.ts:188-218`
  (`openDoorRefusals`: the operator list, then the dev door, beside `open`).
- **Fix touches:** a committed `wrangler.open.jsonc` needs a guard that judges a named file (check 2's directory and
  3–7) and a deploy with `--config`; checks 5 and 6 would hold it to production's blocks.
- **Covered by:** `deploy-guard.test.mjs:415-530`, :532-545 (the committed files). **Premise:** TRUE.

### Item 25 — the route map for the Access application
- **Where:** `worker/src/route.ts:69-84` `route()`; `worker/src/admin.ts:77-89`; `worker/src/index.ts:109` `handle`;
  upgrades at `table/sockets.ts:178` and `house.ts:109`. The layer `crates/yokohama-web/src/api.rs:188-222`
  `routes()`, :233-235 `is_game_route`, :2293-2300 (three public routes, then the identity gate); axum-only
  `lib.rs:905-925`, `ENTER_PATH` :871. Drift-cell precedent `lib.rs:2412-2470`
  `session_routes_names_every_game_route_of_the_router` (reads `api::routes(true)` and `app()`'s SOURCE, both
  directions); `lib.rs:3752` `admission_routes()`. Client `FrontDoor.tsx:148`. M6.5b2 spec
  `docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:644-646`.
- **Now:** Every `/api/` path but `health`, `me`, `labels` and `art` needs an identity; the client reads the art
  manifest as a static file (`App.tsx:970-975`) and labels only when signed in (:955-967), so single player survives
  an application over `/api/` less `health`.
- **Premise:** PARTLY — the M6.5b2 spec says "FIVE DESTINATIONS" and lists six (`/api/games`, `/api/chat`,
  `/api/stream`, `/api/archive`, `/api/me`, `/enter`), none of `/api/labels`, `/api/art`, `/api/admin`.

## Part 2 instruments

- Live replay `worker/scripts/live-replay.mjs:1-40` (usage :5; resets :15-22; exemptions :28-33; its house post
  deleted :34-37). Live Playwright `YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers joins_by_link`
  (`CLAUDE.md:44`); `web/playwright.config.ts:18`, :173 (live `testMatch`), :193-202 (no trace live); the credential
  via `readCredential` (`worker/scripts/dev-vars.mjs:21`, :32, the repository root's `.dev.vars`), imported at
  `web/e2e/live.ts:50`.
- `web/e2e/live.node.test.ts` refuses, by file:line through the TypeScript compiler, any live `request.*` outside
  `liveRequest`, any other use of a context's `request`, and any `toBeOK()`. `liveRequest` is `web/e2e/live.ts:251`,
  not `harness.ts`.
- Opt-ins: `web/playwright.config.ts:163-166` (shots, gates); `exit_game.spec.ts` reads `YOKOHAMA_WEB_EXIT`
  (`live.ts:76`), `WAKE_PAUSE_MS` (:145) and `EXIT_SCREEN` (:283). `read-deploy.mjs` modes `assets`, `install`,
  `console` (:3-6, :487-504).
- `worker/RUNBOOK.md`: §1 :90, §2 :123, §3 :165, §4 :308 (the header-file recipe from :325 writes the two headers into
  a mode-600 file from `.dev.vars`, never on argv or screen), §6 :556, §7 :601 (quota guard :679, the query API :720),
  §15 :1420, §16 :1596, §17 :1654, §18 :1685. `deploy/RUNBOOK.md`: §2 Deliver (the runtime image) :176, §3.18 :2203,
  §5 :2410.
- The gitignored NAS scripts exist: `.superpowers/sdd/2026-09-28-web-frontend-m6.5c2d-plan/nas-build.sh` builds the
  image through Portainer's Docker proxy from a tarball of the musl binary, `web/dist` and the art under
  `Dockerfile.runtime` (`bash nas-build.sh <tag>`); `nas-swap.sh` stops, deletes, creates, starts and times the swap
  as one script (`bash nas-swap.sh <tag>`). Both read a Portainer token from a local file and print none.

## Part 3 documents

- The D8 draft `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/drafts/D8-tasks-24-28.md` (2,990 lines): Task 27
  :1909 (five passages :1913-1920); the lane document :1949 — §1 :1979, §2 :1997, §3 :2005, §4 :2038, §5 the traps
  :2042, §6 :2061, §7 :2089, §8 the door's preconditions :2125, §9 :2139, §10 :2159, §11 :2179; a Python script :2189;
  Track 3 :2403; Task 28 :2452.
- Byte-check precedent: `docs/plans/2026-08-19-docs-consolidation.md:7` (`sed` by line number, diff-verified), §4
  :69-87 (the script was reproduced in the PR); `docs/plans/2026-09-23-claude-md-consolidation.md` §1 :23-31, §3 :120+
  (one `### old line N` and one `text` fence per line not carried byte-identical).
- `guidingpath.md` (232 lines, 412,353 B; archive 209,378 B): Mission :9, Current state :21, the stall :39, Evidence
  :52, Roadmap :63, Decision log :140 (pointer row :144; rows 2026-08-09 → 08-30 :145-204; rows 2026-08-31 onward
  :205-232, out of date order at :228-229).
- `docs/handover.md` (551 lines): "Updated" :9; "previous update" paragraphs :11-100; Track 3 :194-524. Stale: :106
  "The two owner decisions that close V4"; :110 "The web client merged to master 2026-09-11…"; :204 "The phones… Still
  owed; recorded, not gated." (doubled; no "132" there now); :497-511, whose "No API route serves the raw record"
  (:501-502) is false since M6.5a; :535 "The sim-seat / named-identity bot seat option"; :544-551 names D's plan.
- `README.md` (725 lines): See it running :17, The browser client :199, Hosting :294, Status :439, Workspace crates
  :587, Download and run :601, Build from source :633, Data provenance :687, Where truth lives :705. The straw poll's
  lines: :9-10 (v0.5.0), :213-215 (opens **dealing**), :331-333 (the handover), :340-341 (drifted; "is in
  `CLAUDE.md`"), :388 (drifted; "a lobby"), :399-401, :415-417 and :428-437 (the closes stop at M6.5c) — all still
  false.
- `deploy/README.md:209-213` is already true (fixed in D). `worker/README.md` (1,848 lines): :1735 (drifted),
  :1831-1833, :1845; §1.4 :223-278 says the pictures "are recaptured in cycle D" (:225-226, false) and holds TEN
  images, not seven — `door-desktop.png` :231, `door-phone.png` :237, `lobby-desktop.png` :241,
  `open-table-desktop.png` :246, `join-desktop.png` :250, `deal-desktop.webp` :255, `game-desktop-player.webp` :260,
  `game-desktop-watcher.webp` :265, `game-phone.webp` :270, `marker-404.png` :274 (`worker/docs/img/`).
- `deploy/RUNBOOK.md` "The live build is …" :531 and :563 (§3.6, :462). `CLAUDE.md` (185 lines), the five passages:
  :16; :35-56; :87; :107-126; the records table :128-153. The records-table phrase is now :172.

## The variable inventory

| Name | Read at | Kind | Default | Meaning | Host |
|---|---|---|---|---|---|
| `YOKOHAMA_PROJECT_URL` | `web/scripts/project-url.mjs:15` via `web/vite.config.ts:32` | build value | unset: no link | the door's project link | both; `deploy/Dockerfile:199` |
| `VITE_MULTIPLAYER` | `web/src/FrontDoor.tsx:46` | build value | not `false`: on | whether the door draws the friends card | both; set by no script |
| `YOKOHAMA_WEB_DIST_DIR`, `YOKOHAMA_ART_DIR` | `web/scripts/precache.mjs:37`, :38 | build (generator) | `web/dist`; the art directory | what the generator reads | build |
| `YOKOHAMA_WASM_OPT_OPTIONAL` | `web/scripts/wasm-opt.mjs:49` | build hatch | unset: refuse without binaryen | an unoptimized module that says so | build |
| `YOKOHAMA_WORKER_PICKER_CODE` | `crates/yokohama-worker/build.rs:59` → `picker.rs:74` | compile-time stamp | computed | the core, CLI and layer sources' digest | Worker; not settable |
| `YOKOHAMA_WEB_IDENTITY` | `crates/yokohama-web/src/identity.rs:40`, :82 | launch | none: unset exits 1 | `open` / `cf-access` / `worker` | NAS |
| `YOKOHAMA_WEB_HOST` | `main.rs:87` | launch | `127.0.0.1` (image `0.0.0.0`) | bind address | NAS |
| `YOKOHAMA_WEB_PORT` | `main.rs:61` | launch | 8080 | port | NAS |
| `YOKOHAMA_WEB_DIST` | `assets.rs:24` | launch | `web/dist` (image `/srv/web/dist`) | the client root | NAS |
| `YOKOHAMA_WEB_ART` | `art.rs:91` | launch | the art directory (image `/srv/assets/art`) | the art root | NAS |
| `YOKOHAMA_WEB_DIR` | `store.rs:509` | launch | `$HOME/.yokohama/games` (image `/data/games`) | the game store | NAS |
| `YOKOHAMA_WEB_ARCHIVE_DIR` | `archive.rs:241`, :258 | launch | unset: no archive | the archive directory (`dir`) | NAS |
| `YOKOHAMA_WEB_MULTIPLAYER` | `lib.rs:315`, :338 | launch | on | `off` removes the game, chat and stream routes | NAS |
| `YOKOHAMA_WEB_LOBBY_CHAT` | `chat.rs:233`; `worker/src/config.ts:269` | launch / Worker var | on | the house chat | both |
| `YOKOHAMA_WEB_CHAT_KEEP` | `chat.rs:235`; `config.ts:270` | launch / Worker var | 500 (`all`) | messages kept per chat | both |
| `YOKOHAMA_WEB_CHAT_TEXT` | `chat.rs:237`; `config.ts:271` | launch / Worker var | 500 | characters per message | both |
| `YOKOHAMA_WEB_IDENTITY` | `worker/src/config.ts:235` | Worker var | none: a 500 naming it | `worker` / `open` | Worker (`worker`) |
| `ACCESS_TEAM_DOMAIN`, `ACCESS_AUD` | `worker/src/identity.ts:246`, :247 | Worker vars | none: no assertion verified | the Access team and audience | Worker |
| `DEV_DOOR_CLIENT_ID` | `identity.ts:249` | Worker SECRET | none: the dev door shut | the Service Auth client id | Worker |
| `YOKOHAMA_DEV_DOOR` | `config.ts:240` | Worker var | off | claimed identities for tests; refused beside `open` | Worker (`on`) |
| `YOKOHAMA_WEB_ARCHIVE` | `config.ts:116-123` | Worker var | off | `r2` needs the `ARCHIVE` bucket | Worker (`r2`) |
| `YOKOHAMA_WEB_TABLE_TTL_DAYS` | `config.ts:125-134` | Worker var | none: no expiry | idle days before an expiry | Worker (`90`) |
| `YOKOHAMA_WEB_ADMINS` | `config.ts:138-166` | Worker SECRET | none: the operator routes are a 404 | operator addresses; refused beside `open` | Worker |
| `YOKOHAMA_WEB_COUNT_WAKES` | `config.ts:239` | Worker var, orchestrator | off | counts wakes for the exit's window | Worker, by `--var` |
| `YOKOHAMA_WEB_TEST_CLOCK` | `config.ts:174-185` | test-only | the wall clock | `frozen:` / `tick:` | pool files only |
| `YOKOHAMA_WASM_OPTIONAL` | `web/scripts/typecheck-worker.mjs:21`; five `web/src/**/*.node.test.ts` (e.g. `precache.node.test.ts:29`); `web.yml:431` | test-only | unset: refuse | skip without the module | CI `web-client` |
| `YOKOHAMA_WEB_E2E_OPTIONAL` | `web/playwright.config.ts:60` | test-only | unset: refuse | skip without a browser | local |
| `YOKOHAMA_WEB_E2E_TARGET`, `YOKOHAMA_WEB_EXIT` | `web/e2e/live.ts:61`, :76 | test-only, orchestrator | local; off | the live tier; the exit game | live |
| `WAKE_PAUSE_MS`, `EXIT_SCREEN` | `exit_game.spec.ts:145`, :283 | test-only | 45000; 1280x832 | the exit game's pauses and screen | live |
| `YOKOHAMA_WEB_SHOTS`, `YOKOHAMA_WEB_GATES` | `playwright.config.ts:165`, :166 | test-only | off | the picture generators | local |
| `YOKOHAMA_WEB_UPDATE_PIN` | `web/e2e/plays_locally.spec.ts:365` | test-only | off | rewrites the browser-played pair | local |
| `YOKOHAMA_WEB_GATE_BOT_SEED` | `crates/yokohama-web/tests/play_a_whole_game.rs:297` | test-only | OS entropy | replays one gate game | local, CI |
| `UPDATE_WIRE_FIXTURES`, `UPDATE_WORKER_FIXTURES` | `wire_fixtures.rs:3632`, `backup.rs:904`; `crates/yokohama-worker/src/lib.rs:996` | test-only | off | regenerate the fixtures | local |
| `CI` | `web/playwright.config.ts:187` | test-only | unset | Playwright's CI behaviour | CI |

Not read by the web lane: `NO_COLOR`. `window.__YOKOHAMA_NO_SW__` (`web/e2e/sw.ts:31`) is a page flag.
`YOKOHAMA_BGA_CORPUS_OPTIONAL` (`release.yml:22`) is the CLI's and the bridge's. New in E, absent at HEAD:
`YOKOHAMA_SITE_BLURB`, `YOKOHAMA_SITE_NOTICE`, `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS`. **For R-6.5c2.45's cell:** the
Worker's names already have one compiler-held list (`ROWS`, `config.ts:37-51`); the NAS's are constants in four files
(`identity.rs`, `archive.rs`, `lib.rs`, `chat.rs`) and literals in four (`main.rs`, `assets.rs`, `art.rs`,
`store.rs`).

## The API route list

The layer's table (`crates/yokohama-web/src/api.rs:188-222`; with multiplayer off every `/api/games…` and `/api/chat…`
row is gone). "Admitted" is `admitted()`: a seat holder at any status; any signed-in caller at an `open` table for
`table`, `events`, `stream`; nobody seatless while dealing; a spectator at playing or over on an unlocked (or
unlocked-for-them) table.

| Method | Pattern | Who |
|---|---|---|
| GET | `/api/health`, `/api/me`, `/api/labels` | anyone, no identity (`me` answers `user: null`) |
| GET | `/api/art` | answered by the host before dispatch |
| GET, POST | `/api/games` (`?scope=&limit=&before=`) | signed in: the listings; open a table |
| GET, DELETE | `/api/games/{id}` | admitted (`ClientState`); the seated creator (abandon) |
| GET | `/api/games/{id}/table` | admitted |
| GET, POST | `/api/games/{id}/deal`, `…/keep` | a seat holder: their own cards and keep |
| POST | `/api/games/{id}/join`, `…/unlock` | signed in at an open table; a watcher at a locked one |
| GET, POST | `/api/games/{id}/chat` | admitted; a watcher's post is the one spectator write |
| DELETE | `/api/games/{id}/chat/{seq}` | the author or the table's creator |
| POST | `/api/games/{id}/input` | the acting seat only |
| GET | `/api/games/{id}/ports`, `…/labs`, `…/areas`, `…/log`, `…/at?from=&to=`, `…/at/{k}`, `…/why`, `…/index/{kind}` | admitted |
| GET | `/api/games/{id}/suggest` | a seat holder, on their own seat |
| GET | `/api/games/{id}/result` | admitted; 409 while running |
| GET | `/api/games/{id}/record` | a seat holder, finished games only |
| GET, POST, DELETE | `/api/chat`, `/api/chat/{seq}` | signed in; a delete by its author alone |
| POST | `/api/archive` | signed in: a browser's pair, strict-replayed |

The axum host's own (`lib.rs:905-925`; `fallback` :847-864): `GET /api/games/{id}/events`, `…/stream` and
`/api/stream` (server-sent events) with multiplayer on; `GET /enter` (302 to `/#/lobby`) with multiplayer or an
archive; other `/api/…` is "no such endpoint"; `/art/…` is static; the rest is the client with the SPA fallback.

The Worker shell's own (`worker/src/route.ts:69-84`, `admin.ts:77-89`): `GET|HEAD /enter` (302; else 405);
`GET|HEAD /api/art` (the static manifest projected); `POST /api/games` and `/api/games/{name}…` to the table object
(with `…/events` and the `…/stream` WebSocket); `POST /api/archive` to the caller's archive object;
`GET /api/admin/tables` and `POST /api/admin/tables/{id}/expire[?force]` past the operator gate; other `/api/…` to the
house (`/api/stream`'s upgrade among them); a path with no file is a 404 with an empty body.

The local host (`crates/yokohama-wasm/src/lib.rs`): `init` :115, `dispatch` :184 (the layer's `api::dispatch` over
`MemStore`), `load` :244, `read` :250, `set_clock` :285, `restore_read` :304, `debug_trap` :319 (debug feature),
called from `web/src/local/worker.ts` alone.
