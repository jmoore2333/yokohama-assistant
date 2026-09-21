# M6.5b2 "the front door, installable and offline" — specification

**Status: DRAFT, written 2026-09-21 from the brainstorm with the owner on branch `web-room-m6.5b2`
(stacked on `web-room-m6.5b` at `369a1f2`).** Its authority over this cycle is the owner's rulings in
§1; where it disagrees with the entry handover
`docs/plans/2026-09-21-web-frontend-m6.5b2-handover.md`, this document wins and §0.3 says where.

---

## 0. What this is

### 0.1 The sentence

A person opens the app with no login, plays the bot in their browser from open to close with no
server present, and their games survive the browser, the device and the release.

Three things make that true and none of them exists at HEAD:

1. **A front door.** Today the whole hostname is behind Cloudflare Access, so the very first fetch
   of the shell, the module and the art is challenged. Single player cannot be offline while its
   first byte needs a login.
2. **A service worker.** Today every asset is fetched from the origin. M6.5b's "level 2" is the API
   unreachable with the page already loaded; this is level 3, the origin down.
3. **A way out and back in.** Today a downloaded save is a CLI recovery path and the browser has no
   restore, so a cleared browser, a new device or an iOS home-screen install loses everything.

### 0.2 Read order

1. This document's §1 (the rulings) and §2 (the mechanism).
2. The entry handover `docs/plans/2026-09-21-web-frontend-m6.5b2-handover.md` — §2 is the record of
   what M6.5b measured, and every figure in it still stands except where §0.3 below corrects one.
3. The M6.5b close `docs/plans/2026-09-21-web-frontend-m6.5b-close.md` §5 (the rulings), §6.3 (the
   owner items, two of which this cycle closes) and §7 (what it does not claim).
4. The umbrella `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md` — R-6.5.3 (the door),
   R-6.5.8 (test in the live setup), R-6.5.12 as amended by R-6.5b.16 (Workers Free). **R-6.5.3 is
   AMENDED by R-6.5b2.2 below**: the hostname-wide Access application becomes path-scoped, and the
   app stops depending on Access existing at all.

### 0.3 What this cycle measured that changes a figure already written down

Every one of these was measured on 2026-09-21 on the owner's M5 workstation and is reproducible.

- **Cloudflare Access answers two different ways, and which one depends on ONE header.** A
  navigation, and any request without `X-Requested-With`, gets a `302` to
  `bdvc.cloudflareaccess.com`. A request carrying `X-Requested-With: XMLHttpRequest` gets a **`401`
  with `WWW-Authenticate: Cloudflare-Access resource_metadata=…`** and
  `access-control-allow-credentials: true`. `Accept: application/json`, `Sec-Fetch-Mode: cors` and
  `Sec-Fetch-Dest: empty` were each measured NOT to trigger it. The 401 is same-origin, so the
  client can read that header; the 302 is to another origin and a `fetch` following it fails
  opaquely. **This is the mechanism §2.1's expired-session handling rests on, and it is a
  measurement and not a documented promise.**
- **The module's size is a choice, not a fact.** Nine builds, same machine, same harness, the bot
  figure being a whole first turn through the module including the keep:

  | profile | raw | gzip | bot's first turn |
  |---|---|---|---|
  | shipped (`release`, no LTO) | 2,842,659 B | 753,763 B | 28.6 ms |
  | fat LTO, `codegen-units = 1` | 2,535,671 B | 750,602 B | 26.2 ms |
  | LTO + `opt-level = "s"` | 1,859,295 B | 556,089 B | 31.1 ms |
  | LTO + `s` + `wasm-opt -Oz` | 1,349,525 B | 514,933 B | 31.4 ms |
  | LTO + `opt-level = "z"` | 1,782,177 B | 507,250 B | 36.6 ms |
  | **LTO + `z` + `wasm-opt -Oz`** | **1,168,913 B** | **452,633 B** | **35.2 ms** |

  Nine seeds per timing, median reported, `wasm-opt` 132, `wasm-bindgen` 0.2.128. Builds ran under
  20–35 s and each `wasm-opt` pass under 4 s. **The handover's §2 figure of 2.84 MB / 758 KB is the
  `release` row and stays true of HEAD**; the gzip differs from its 758 KB because this reading used
  `gzip -9`. The raw figure also differs from the close's 2,842,550 B by 109 bytes for the same
  asset: two builds of the same source, not two different artifacts, and the close records it as
  build variance so the two figures do not read as a contradiction.
- **The release is about 14.6 MB today, not "about 8 MB".** The charter's estimate predates the
  help pictures. Measured: art 7.4 MB (**131 WebP files plus `manifest.json`**, not the 132 WebP
  files `CLAUDE.md` and the handover both say — the 132nd file is the manifest; **`CLAUDE.md` and
  the handover both need amending at the close**), help 4.2 MB (five
  1710×869 PNGs, 611–954 KB each), the module 2.84 MB, the client bundle 329 KB, its CSS 35 KB, the
  Worker chunk 13 KB, `index.html` 1.4 KB. §2.7 brings it to about 10 MB.
- **A finished game is small.** The committed 595-event pair is 54,103 B for the record and 554 B
  for its table document, so about 91 B per event. Storage pressure is not a risk in this design;
  eviction and a cleared browser are.
- **The offline archive controls are not drawn at all, and Abandon deletes without trying.**
  `archiveKind()` reads the server's facts, whose default is `archive: "off"` until `/api/health`
  answers (`web/src/local/facts.ts:65`). Offline it never answers, so "Send to the archive" fails
  its render condition (`App.tsx:1719`) and `Abandon`'s `abandon()` skips `sendFirst` entirely
  (`Abandon.tsx:110`) and deletes. **What already holds and is worth keeping**: a send that FAILS
  keeps the game (`sendFirst` returns false, the delete never runs, the sentence is
  `UNREACHABLE`), and a retry is safe because the server answers `identical` for a second send of
  the same bytes. §2.6 closes the gap without disturbing either property.

---

## 1. What the owner ruled (2026-09-21)

- **R-6.5b2.1 — the front door, and everyone sees it.** `#/` is a door with two ways on: play on
  this device, or play with others. A person with a live Access session reaches the lobby from it
  with no challenge, so the cost to a regular is one click and no login. Single player stops being
  a section above the lobby and becomes a channel of its own.
- **R-6.5b2.2 — Access is a deployment concern and the app never depends on it.** Owner: *"we're
  using access as a dev front door, and protection from access doesn't require the app to even be
  aware of it, so if we wanted to allow internal self hosting by users that didn't want to use
  identity, we would have it be available."* The hostname-wide application becomes path-scoped
  applications on the protected prefixes, and a deployment with no Access at all is a supported
  shape. **This AMENDS umbrella R-6.5.3**, which put one application on the hostname.
- **R-6.5b2.3 — `claimed` becomes a development build feature.** Owner: *"the identity mode of
  allowing a free type email is a dev build flag, that was the intention on the original mode but
  if not we should in this build be able to disable that."* A build without the feature refuses to
  start in that mode, naming the feature. The NAS keeps the feature for now; the owner ruled
  explicitly to **stay on `claimed` there** rather than move to `cf-access` this cycle.
- **R-6.5b2.4 — multiplayer is a deployment switch, and off means NOT SERVED.** Not gated, not
  hidden: the game, chat, stream and archive routes do not exist, so there is nothing to probe and
  no Access application to forget. This is the answer to *"if we did want to host the single player
  being available with the multiplayer being unavailable without additional gating"*.
- **R-6.5b2.5 — the hostname splits.** `yokohama.moore.nyc` is **reserved for the Cloudflare
  Worker**, which has not been deployed with wrangler. **The NAS moves to
  `yokohama-nas.moore.nyc`.** Browser storage is per origin, so the NAS's existing local games do
  not travel; nothing of consequence is stored there (owner: no games in progress, in-person
  testing only) and §2.5's envelope is the path if one is ever wanted.
- **R-6.5b2.6 — the public build's configuration is M6.5c's call.** This cycle BUILDS both switches
  and proves them locally. Whether the public Worker serves multiplayer behind Access, or single
  player alone, is decided when it is deployed.
- **R-6.5b2.7 — a single-player visitor invokes the Worker ZERO times.** Owner: *"if we have them
  as static we could have the door be unlimited serving for single player which is attractive."*
  Static asset requests on Workers are free and unlimited and carry no egress charge, so this makes
  the door uncapped and free rather than merely cheap. §2.2 is how the three routes come off the
  path.
- **R-6.5b2.8 — the release is cached whole, keyed by its own contents, and an update never
  switches by itself.** Cache-first navigations, a background install, one quiet offer, and the
  switch only on the player's click.
- **R-6.5b2.9 — installable, with the platforms treated differently and neither nagged.** A real
  button where the browser offers one, one short dismissible instruction where it does not.
  Persistent storage requested at the first committed move, never at boot.
- **R-6.5b2.10 — one backup envelope, for one game or for the whole device.** Per-game control on
  the game and its row; "Back up everything" on the first screen. Record and table document as
  strings, validated in Rust, shared with the archive route.
- **R-6.5b2.11 — a restore clash refuses and names the row.** Identical bytes read "already on this
  device" and change nothing, the way the archive route already answers `identical`.
- **R-6.5b2.12 — an incompatible save is kept, listed and downloadable, never migrated or erased.**
  A record at another `SEMANTICS_REV` RESTORES and lands as an unplayable row that says why.
  Refusing it outright would strand the only copy.
- **R-6.5b2.13 — the outbox.** A finished local game that cannot be uploaded is queued with its
  reason and retried on the browser's `online` event, at the next launch, and on the player's
  button. **Abandon does not join the queue**: offline it refuses and offers two named choices,
  because throwing a game away must not happen implicitly when the thing that would have saved it
  was unreachable.
- **R-6.5b2.14 — uploading requires passing the door.** An unauthenticated write route is not
  opened on the public internet, and the M6.5b close's §6.3 item 6 (an untrusted `GameRecord`
  reaching the engine's setup path) is still open.
- **R-6.5b2.15 — the module takes a profile of its own.** `[profile.release]` stays byte-identical
  for every other crate and for the fifteen-gate roster. `wasm-opt` becomes a build prerequisite
  that refuses loudly when absent.
- **R-6.5b2.16 — one cycle, with the front door deployed mid-cycle.** The offline exit cannot be
  measured until the shell is fetchable without a login, so the door lands and goes live first.
- **R-6.5b2.17 — the exit runs on the owner's iPhone; the Android emulator is a second reading.**
  Owner: *"I don't think we have to push too far testing so many version of ios and android, this
  is a fairly straightforward mobile browser comptibility test."* Recorded, not gated.
- **R-6.5b2.18 — the fonts are self-hosted.** The page loads Fraunces and JetBrains Mono from
  Google's CDN, so an offline launch silently loses its typefaces. Both are Open Font License, so
  this is permitted outright and is the only way the release is self-contained. **The licence text
  travels with the files**: each family's OFL copy committed beside it and named in `NOTICE`, with
  `ASSETS.md` pointing at the path, since that file today lists only what carries NO grant and a
  granted third-party asset needs its grant recorded where a reader will look. The
  `<link rel="icon" href="data:,">` in `index.html` is superseded by the manifest's icons and the
  `apple-touch-icon`, and goes.

---

## 2. The mechanism

### 2.1 The front door and the turnstile (R-6.5b2.1, R-6.5b2.2)

**The paths split in two, and the split lives at the edge, not in the app.**

| public | protected |
|---|---|
| `/`, `/assets/*`, `/art/*`, `/help/*` | `/api/games*` |
| `/manifest.webmanifest`, `/sw.js`, the icons | `/api/chat` |
| `/api/health`, `/api/labels`, `/api/art`, `/api/me` | `/api/stream` |
| | `/api/archive` |
| | `/enter` |

Nothing in `crates/yokohama-web` enumerates that table: it is five Access applications on the
protected prefixes and no application on the hostname (§3.2). The origin's own rules are unchanged
— every `/api/games` route still requires an identity and answers 401 without one — which is what
makes a deployment with no Access in front of it a supported shape rather than an open door.

**`/enter` is a turnstile and nothing else.** A new `GET /enter` on the server answers `302` to
`/#/lobby`. Access challenges it, the browser acquires `CF_Authorization` for the hostname, and
every later `fetch` to a protected path carries it. "Play with others" is therefore a plain link
and never a `fetch`, so the login is an ordinary navigation the browser already knows how to do.
**The service worker never intercepts it** (§2.3) and the SPA fallback must not answer it, so it is
a real route and not a path that falls through to `index.html`.

**It is served whenever ANY protected route is configured**, which is multiplayer on OR an archive
on — not multiplayer alone. R-6.5b2.4 is about the multiplayer routes not existing; the turnstile is
about acquiring the cookie for whatever protected route does. The first draft of this section tied
it to multiplayer and thereby pointed §2.6's outbox remedy and R-6.5b2.14 at a 404 on precisely the
deployment §3.1 describes, single player free and anonymous with a door to pass once to contribute
a save. With nothing protected at all it is not served, and the door draws no link.

**An expired session is read, not guessed.** Every call `api.ts` makes gains
`X-Requested-With: XMLHttpRequest`, measured in §0.3 as the one header that turns Access's redirect
into a same-origin `401`. A 401 carrying `WWW-Authenticate: Cloudflare-Access` is the EDGE saying
the session is gone and renders "your session has expired" with a link to `/enter`; a 401 in this
API's own `{"error": …}` JSON shape is the SERVER saying there is no signed-in user and keeps its
present meaning. The two are distinguished by that header and by the content type, and a cell pins
both directions.

**The door screen.** `#/` is the door for everyone:

- **On this device** — the local games, and "Play the bot here". Unchanged from M6.5b except for
  its position and the new storage line (§2.4) and backup control (§2.5).
- **Play with others** — a link to `/enter`. Under a build with multiplayer off (R-6.5b2.4) it is
  absent and one line says the deployment plays single player only.
- **Offline readiness** — one line: installing, ready to play offline, or a release that failed to
  cache (§2.3).

A signed-in person reaching `#/lobby` sees today's lobby unchanged. The door does not probe whether
the session is live, because it does not have to: the link costs one click either way.

### 2.2 Zero Worker invocations for single player (R-6.5b2.7)

Three routes come off the first-screen path, and two of them disappear rather than becoming static.

- **`/api/art` becomes `/art/manifest.json`.** The route reads `assets/art/manifest.json` and
  re-serializes it (`lib.rs:659`); the identical file is already served as a static asset by
  `art::serve`. The client fetches the file. A cell asserts the route's body and the file
  deserialize to the same `ArtManifest`, so the two cannot drift, and the manifest is precached.
- **`/api/labels` is answered by the local host.** It is computed from the engine's registries
  (`api.rs:323`), is deployment-independent, and the corpus replay already proves both hosts emit
  it byte-identically. The boot's server-first-with-a-bound chain (`App.tsx:740`) becomes
  local-first for the door and for any local game.
- **`/api/health` is not read at the door, but it IS read before the archive fact is used.** Every
  fact it carries — the transport, the chat caps, the identity mode, whether an archive exists — is
  a multiplayer or past-the-door concern, so the door itself asks nothing. **The last answer is
  remembered** (one key, the shape of `chatFold.ts`) and the read is made at three points: entering
  multiplayer, a local game reaching game over, and Abandon being clicked on a local game. It is
  bounded the way the labels read already is, and a browser that has an answer never asks again
  unless it is stale.

  **This is not a weakening of R-6.5b2.7 and the reason matters.** `archiveKind()` drives the whole
  LOCAL channel — the send control's render condition (`App.tsx:1719`), "Abandon and save"
  (`Abandon.tsx:110`) and the outbox's entry condition (§2.6) — so a spec that removed the boot read
  and gave the local channel nothing would make §0.3's measured bug permanent for ONLINE sessions
  too: a single-player visitor would never learn `archive: "dir"`, would see no send control, and
  would have Abandon delete silently. Browsing and playing still cost zero Worker invocations; the
  cost is at most one per browser, at the moment a game ends, on a person who has already decided
  to keep something.

After the first visit the service worker serves the shell, the bundle, the module and the art from
the cache, so a returning single-player visitor touches the network not at all.

### 2.3 The service worker and the release (R-6.5b2.8)

**The release is identified by a digest over its own contents.** A build script
(`web/scripts/precache.mjs`) runs after `vite build`, reads Vite's own manifest and the art
manifest rather than restating either, adds the static extras, computes each file's content hash,
and writes `dist/sw.js` from a template with the sorted list and a cache name derived from it.
Nothing has to be remembered or bumped. It also catches what a version number would miss: the art
files have stable names rather than hashed ones (`art.rs:112` serves them `max-age=3600` for
exactly that reason), so a re-cut card changes no filename and would otherwise never reach a
browser that had already cached it.

**What is precached**: `index.html`, the hashed JS, CSS, Worker chunk and `.wasm`, the 131 art
images and their manifest, the five help pictures, the web app manifest and its icons, and the
self-hosted fonts (R-6.5b2.18).

**The install is all-or-nothing.** Every entry is fetched with `redirect: "error"` and checked for
`200` and the expected content type; one failure fails the install and leaves the previous release
in place. That is the charter's "cached whole before you say offline" as a mechanism.

**Mixing an old bundle with a new module is impossible rather than forbidden.** The cached
`index.html` names the hashed assets of its own generation and they live in the same cache
generation, so the set is matched by construction.

**Three rules at runtime**, and they are one pure function so they can be tested without a browser:

1. `/api/*` and `/enter` are never intercepted. Not cached, not read, not written.
2. A precached path is served from the cache.
3. Everything else goes to the network and is not cached.

Navigations are served the cached `index.html` immediately. The update path is the standard
lifecycle: the browser re-fetches `sw.js`, sees different bytes, installs the new cache in the
background, and waits. The app then shows one quiet line offering a reload; `skipWaiting` is called
only on that click, so a switch can never happen mid-move. Old caches are deleted after activation
and only those matching this app's name prefix.

**Cache eviction structurally cannot reach a saved game.** The Cache Storage API and IndexedDB are
different stores with different eviction; deleting a cache generation touches no game. The charter's
hardest sentence is satisfied by the choice of API rather than by a rule anyone has to follow.

**Registration is production-only** (`import.meta.env.PROD`), because Vite's dev server serves no
`dist` and a worker registered there would serve a stale module tree. `/sw.js` must 404 when absent
rather than fall through to `index.html`; `assets.rs` already refuses the SPA fallback for `.js`
paths, and a cell pins that it still does for this one.

**`/sw.js` is served `Cache-Control: no-store`.** `assets.rs`'s rule sets a header on `/assets/`
and on nothing else, so today this file would go out with no freshness header at all and a browser
may heuristically cache it. Update detection is a byte comparison of this one file, so a cached copy
delays every update by the cache window. It is the one path besides `/assets/` that gets a header,
and the existing cell that asserts "hashed assets are immutable and nothing else carries one" is
widened rather than weakened.

**The e2e must not register it by accident, and that is a design decision rather than a discovery.**
`npm run e2e` runs `build:client`, which is a production build, so without a rule every one of the
existing 46 cells would register a worker and start a ten-megabyte precache per context, and every
reload-based cell would start reading a cache-first navigation. Registration therefore consults a
flag an `addInitScript` sets, a shared fixture turns it OFF for every spec, and the offline spec
turns it on. **The convention is held by a test and not by memory**, in the shape
`every_spec_can_skip.spec.ts` already uses: a spec that neither takes the fixture nor opts in is
named at run time.

**The install is visible and bounded.** The door shows it happening and says "ready to play
offline" only when the release is complete. When `navigator.connection.saveData` is set, the art
waits for an explicit request while the shell and module still install, so a metered connection is
never charged nine megabytes unannounced.

### 2.4 Installable and persistent (R-6.5b2.9)

**A web app manifest** with `display: standalone`, an explicit `id` so a later `start_url` change
does not produce a second installed app, colours from the stylesheet's own tokens, and **original
icons drawn in-repo** at 192, 512 and 512-maskable plus a 180 `apple-touch-icon`. Never a crop of
the card art: `ASSETS.md` carves those out as carrying no grant, and an app icon is the most
distributed image a product has.

**Android Chrome** fires `beforeinstallprompt`, which the door captures for a real Install button,
and the browser grants persistent storage on install without a prompt.

**iOS Safari** offers no programmatic install, so the door detects Safari on iOS, detects that it
is not already standalone (`display-mode: standalone`, or `navigator.standalone`), and shows one
short instruction naming Share and Add to Home Screen. Dismissible, remembered per browser the way
the chat fold is, never a modal and never repeated.

**Two WebKit facts make the iOS install worth offering, and the second must be said on the screen
that offers it.** WebKit deletes all script-writable storage after seven days without interaction,
and the first-party domain of a home-screen web app is exempt. But an installed iOS app gets its
own storage container and shares nothing with Safari, **so games played in the browser do not
appear in the installed app.** The install offer therefore carries the backup control beside it;
the envelope in §2.5 is the only bridge across that boundary.

**Persistence** is requested with `navigator.storage.persist()` at the first committed move of a
local game, never at boot, so a visitor who never plays is never asked. The grant is read back with
`persisted()` rather than assumed, and `estimate()` gives usage against quota. "On this device"
reports three things: whether the browser has promised to keep these games, how much is stored, and
a backup control **offered in every state**. A grant is useful and an export is still necessary, so
no wording claims safety. A quota failure already has a path — M6.5b's `unsaved` and its "APPLIED
BUT NOT SAVED" line — and this cycle points it at backup rather than inventing a second screen.

### 2.5 Backup, restore and the envelope (R-6.5b2.10 – .12)

**One envelope, one game or all of them**, so there is a single format rather than two:

```json
{
  "kind": "yokohama-backup",
  "envelope": 1,
  "created_at": 1789000000,
  "origin": "yokohama-nas.moore.nyc",
  "app_version": "0.5.0-m6.5b2",
  "semantics_rev": 7,
  "rules_hash": "d85445c479cb1ead",
  "games": [{ "id": "local-…", "record": "…", "table": "…" }]
}
```

**`record` and `table` are STRINGS and the browser never parses them.** The record's terminal digest
is over its exact bytes and its seed is a `u64` that `JSON.parse` rounds — the rule `recordText`,
the archive route and the IndexedDB rows already follow.

**Validation lives in Rust and is shared with the archive route.** `archive_post` already
strict-replays a record and checks `TableDoc::agrees_with`; that becomes one function two callers
use, so a backup and an upload cannot disagree about what a valid game is. **One** new
`yokohama-wasm` export wraps it: `restore_read(envelope_json) -> report_json` validates and returns
a per-game verdict with the player count, the event count and whether it is finished.

**Writing the envelope is the PAGE's job and deliberately not the module's.** The page already
holds both documents as strings — `localPair` is what the archive send reads — so building the
envelope is a wrapper around strings it has, not a parse. The reason to keep it out of wasm is the
case that matters most: a module that fails to load is exactly when a player most needs their games
out, and a `backup` export would die with it. Restore needs the engine and cannot be moved; backup
does not and is not.

**The two failures get different answers**, which is R-6.5b2.12:

- **Rejected** — not a Yokohama record, a table that does not agree with its record, or a replay
  that fails. Named in the report; nothing is written.
- **Restored but unplayable** — a record at another `SEMANTICS_REV`. It lands, it is listed, it can
  be saved out again, and the row says why it will not open. M6.5b's broken-save screen is already
  the shape for it.

**The clash rule is the Worker's** and needs no Rust: for each accepted game it reads IndexedDB and
writes only where nothing is present. Present and byte-identical reads "already on this device";
present and different **refuses and names the row**. So a restore run twice is harmless, which is
the property that makes it usable when you are not sure the first one worked.

**A restored cross-rev game must actually LIST, and that is a separate claim from the report.**
M6.5b's broken-save screen is a rev-mismatched record under a table document written at the CURRENT
schema; a restored cross-rev pair may be foreign on both sides, and `TableDoc`'s shape check is what
decides whether the row appears at all. A cell restores such a pair on a fresh profile and asserts
the row is IN the listing with its reason, not merely that `restore_read` called it unplayable.

**The raw `<id>.json` export stays exactly as it is**, because the terminal loads that and only
that. Two controls, each saying in a few words what it is for.

Two payoffs fall out: the envelope is the bridge across iOS's separate storage for an installed
app, and it is the migration path across the hostname move in R-6.5b2.5.

### 2.6 The outbox (R-6.5b2.13, R-6.5b2.14)

A third IndexedDB store, `outbox`, keyed by game id: the reason a finished local game has not
reached the archive, and when it was last tried.

- **Entered** when a local game ends and the deployment is known to keep an archive and the upload
  does not land. The reason is recorded and shown, because **"no connection" and "not signed in"
  are different problems** and only the second has something the player can do about it now.
- **"Not signed in" names BOTH doors under `claimed`.** Passing `/enter` gets the Access cookie;
  `POST /api/archive` then still refuses in this API's own JSON shape until the in-app sign-in has
  supplied `X-Yokohama-Claimed-User`. So the remedy is mode-aware, read off `/api/health.identity`:
  under `claimed` it is two steps and says so, under `cf-access` and `worker` the turnstile is the
  whole of it.
- **Drained** on the browser's `online` event, at the next launch, and on the player's own button.
  **Deliberately not the Background Sync API**, which Safari does not implement: a portable
  mechanism beats one that works on one platform.
- **Safe to retry by construction**, because the server answers `identical` for a second send of
  the same bytes and the client already treats 200 and 201 alike.
- **Never authorises a delete.** Only a landed upload does, which is `Abandon`'s existing rule.

**Abandon stays deliberate, and "unknown" is not "off".** Three states, not two. A deployment
KNOWN to keep no archive deletes as it always has, which is correct and is not a bug. A deployment
known to keep one sends first and keeps the game if the send fails, which is HEAD's behaviour and is
kept. **Unknown triggers the bounded health read at the click**: if it answers, the answer decides;
if it does not answer, Abandon REFUSES and offers two named choices, back it up to a file or abandon
without saving. It never joins the outbox and it never deletes quietly, which is the behaviour §0.3
measured at HEAD and the one thing in this area that must not regress.

**Uploading requires passing the door** (R-6.5b2.14). A pending game whose reason is "not signed
in" offers the same `/enter` link the door does.

### 2.7 The module's size and the release budget (R-6.5b2.15, R-6.5b2.18)

**A named profile, so `[profile.release]` is byte-identical for every other crate** and the
fifteen-gate roster builds exactly what it builds today:

```toml
[profile.release-wasm]
inherits = "release"
opt-level = "z"
lto = "fat"
codegen-units = 1
```

`npm run build:wasm` builds `--profile release-wasm` and then runs `wasm-opt -Oz --strip-debug
--strip-producers`. From §0.3 that is **2,842,659 → 1,168,913 B raw and 753,763 → 452,633 B
gzipped, for 28.6 → 35.2 ms on a whole bot turn.** The raw figure is the one that matters most,
because it is what a phone compiles on every cold start.

`wasm-opt` becomes a prerequisite. Absent, the build **refuses loudly** naming
`brew install binaryen` and the apt package, with `YOKOHAMA_WASM_OPT_OPTIONAL=1` turning the
refusal into a build that says out loud it produced an unoptimized module — the shape
`YOKOHAMA_WEB_E2E_OPTIONAL` and the capture corpus already use. `deploy/Dockerfile`'s Rust stage
installs it.

**Two more cuts, and then a budget.** The five help pictures are 4.2 MB of PNG at 1710×869
(§0.3); re-cut as WebP by a small generator in the style of `docs/tools/mkcardart.py`, and
committed the same way. The fonts come off Google's CDN and into `/assets/` with the hashed
pipeline, which removes a render-blocking third-party request from every load and is what makes the
release self-contained at all.

| | today | after |
|---|---|---|
| art (131 images + manifest) | 7.4 MB | 7.4 MB |
| help pictures | 4.2 MB | ~0.8 MB |
| the module | 2.84 MB | 1.17 MB |
| bundle, CSS, Worker chunk | 0.38 MB | 0.38 MB |
| fonts, icons, manifest, shell | 0 (third-party) | ~0.3 MB |
| **total** | **~14.6 MB** | **~10 MB** |

**The budget is asserted inside the precache generator**, not in a separate test. That script
already reads every file's bytes to compute the release digest, so a ceiling on the module and a
ceiling on the release total is one more line in something that runs on every release build. Both
ceilings carry their measurement date in-source and are set with headroom, so ordinary growth does
not trip them and a dependency that doubles the module does.

---

## 3. The deployment

### 3.1 The three switches, and what each refuses

| | what it is | absent or off |
|---|---|---|
| `dev-identity` | a cargo feature on `yokohama-web`, in `default` beside `server` | `YOKOHAMA_WEB_IDENTITY=claimed` **exits 1 naming the feature**; `cf-access` and `worker` unaffected. A public build turns it off with `--no-default-features --features server`, which is the one spelling that keeps the server half and drops this |
| `YOKOHAMA_WEB_MULTIPLAYER` | `on` (default) or `off` | the game, chat and stream routes are **not served**; `/api/health` reports `multiplayer: false`; the door draws single player only. **`/api/archive` is NOT one of them** — it is governed by its own variable below, so a single-player deployment can still collect saves |
| `YOKOHAMA_WEB_ARCHIVE_DIR` | unchanged from M6.5a | no archive; the outbox never enters |

They are **independent**, and `/enter` follows their OR rather than either one: it is served when
multiplayer is on or an archive is configured, and only a deployment with nothing protected at all
does without it. Multiplayer off with an archive on is a coherent deployment: play single player
free and anonymous, pass the door once to contribute a save. A bad value for any of them
exits 1 naming the variable, which is `main.rs`'s existing rule.

`YOKOHAMA_WEB_MULTIPLAYER=off` removes the routes from the layer's route TABLE, so an unknown path
is the table's own 404 and there is no second code path that "hides" a route. A cell asserts every
protected pattern answers 404 under the switch and that the public ones still answer.

### 3.2 The hostname move and the Access applications (R-6.5b2.5, R-6.5b2.2)

**The NAS takes `yokohama-nas.moore.nyc`**, consistent with `yokohama-cli.moore.nyc`. Four steps,
each recorded in `deploy/RUNBOOK.md`:

1. A proxied CNAME for the new name to the tunnel.
2. An ingress block in `/volume1/docker/cloudflared/config/config.yml` above the catch-all, and the
   existing `yokohama.moore.nyc` block removed once the new one answers.
3. **Five Access applications on the new hostname**, each on a protected prefix: `/api/games`,
   `/api/chat`, `/api/stream`, `/api/archive`, `/enter`. The owner's existing allow-list policy on
   each. **No application on the hostname itself.**
4. `yokohama.moore.nyc` freed and held for the Worker.

**Verified rather than assumed, at the mid-cycle deploy**: each public path answers 200 with no
session, each protected path answers 302 to the login for a navigation and 401 with
`WWW-Authenticate` for an `X-Requested-With` request, and `/enter` round-trips to the lobby. The
readings go in the RUNBOOK beside the previous deployments'.

**Local games do not travel** across the move. Nothing of consequence is stored (R-6.5b2.5) and the
envelope is the path if one is ever wanted.

### 3.3 What the self-host gets

`deploy/Dockerfile` gains `binaryen` in its Rust stage and nothing else changes. An operator who
wants no identity runs with multiplayer off and no Access in front; one who wants the full thing
runs as the NAS does. Neither configuration is expressed in the app.

---

## 4. Wording

Every sentence a player can read is in one of four places, and none of them is new prose invented
twice:

- **The door**: "Play on this device" and "Play with others"; under multiplayer off, one line
  saying this deployment plays single player only.
- **Readiness**: "Preparing to play offline", "Ready to play offline", and a named failure when the
  install could not complete.
- **Storage**: whether the browser has promised to keep these games, the usage, and on iOS the
  sentence that an installed app does not see the browser's games.
- **Refusals**: the expired-session line with its link; the restore report's per-game verdicts; the
  outbox's two reasons; Abandon's offline refusal with its two named choices.

`UNREACHABLE` and "APPLIED BUT NOT SAVED" are carried verbatim from M6.5b, not restated.

---

## 5. The wire and the corpus

Additive and small, which is the property to check at review:

- **`/api/health` gains `multiplayer: bool`.** Every `health*` fixture moves by one key; nothing
  else in the corpus moves. That is what proves this cycle changed no payload shape.
- **`/enter` has no payload.** It is a 302 and is pinned by status and `Location`, not a fixture.
- **The envelope is NOT a wire payload** — it is a file the player holds — but it gets a committed
  fixture of its own so the client's reader and the Rust validator are pinned to one document.
  Generated by the same test that generates the corpus, from the committed browser-played pair.
- **`/api/art` keeps answering**, unchanged, so nothing that reads it breaks. The client stops
  calling it; the cell asserting route and file agree is what keeps the two honest.

---

## 6. The tests

### 6.1 Rust (`cargo test -p yokohama-web`, `-p yokohama-wasm`)

- The envelope: shape and version refusals, the per-game verdicts, and that a rev-mismatched
  record is `unplayable` rather than `rejected`.
- That the archive route and the envelope validator are the same function over one pair.
- The multiplayer switch: every protected pattern 404s under it, every public one answers.
- The `dev-identity` refusal, by name.
- `GET /enter` is a 302 to `/#/lobby`; it is SERVED with multiplayer off and an archive on, and
  absent only when nothing is protected.
- `/sw.js` 404s when absent rather than answering HTML, and is served `no-store` while `/assets/`
  keeps `immutable` and nothing else carries a header.
- `/api/art`'s body and `assets/art/manifest.json` deserialize to the same `ArtManifest`.
- `yokohama-wasm`'s two new exports, native cells under the existing test lock.

### 6.2 Node (`npm test`, `npm run typecheck`)

- The precache generator: the list is complete against Vite's manifest and the art manifest, it is
  deterministic, the cache name moves when any file's bytes move, and both ceilings are asserted.
- The service worker's routing decision as a pure function, including that `/api/*` and `/enter`
  are never intercepted.
- The door: both entries, the multiplayer-off shape, the readiness line's three states.
- The expired-session read: a 401 with `WWW-Authenticate` versus a 401 in the API's own JSON shape.
- The install logic on both platforms, including that the iOS instruction is not shown when already
  standalone and not shown again once dismissed.
- The outbox state machine and its two reasons.
- Restore's report rendering, including the clash naming its row.
- Storage reporting across granted, denied and unavailable.
- The archive fact: unknown triggers the read; a known `"off"` deletes on Abandon; a known archive
  sends first; unknown with no answer REFUSES. The four arms are the finding §9 records.
- The outbox's remedy under `claimed` names two steps and under `cf-access` one.

### 6.3 Playwright (`npm run e2e`)

This is where the cycle's claims actually live. Six cells, and the first is the exit in miniature:

1. **Offline play.** Install the worker, `context.setOffline(true)`, reload, play a whole local
   game to a winner with the network off, and assert zero network requests succeeded.
2. **Close and reopen.** Offline, close the context, reopen with the same storage state, resume the
   game, finish it, export it.
3. **The update.** A second release served; the offer appears, the app does NOT switch until the
   reload is clicked, and the old cache is gone after activation.
4. **Restore on a fresh profile.** A new context with empty storage restores the envelope and plays
   the restored game. A cross-rev pair restored the same way **appears in the listing** with its
   reason, which is a different claim from the report calling it unplayable.
5. **The clash.** A restore over a present, different game refuses and names the row; a restore of
   identical bytes reads "already on this device".
6. **The outbox.** Finish a game offline, come back online, and the upload lands with nobody
   clicking anything.

The run's wall is already 4 m 47 s; these add to it and the close records the new figure.

### 6.4 The phone (R-6.5b2.17)

**The iPhone runs the exit**, by hand: install to the home screen, airplane mode, play, close the
browser entirely, reopen, resume, finish, export. Then an update and a restore on a fresh profile.
**The Android emulator on this workstation is a second reading**, recorded in the close, not gated.
Nothing else is tested, and the close says so.

### 6.5 The mutation battery

Written with the spec, the way M6.5b's §6.6 was, and run at the fix-wave head. At least: a precache
entry dropped from the list; the install's all-or-nothing turned into best-effort; `skipWaiting`
called on install; `/api/*` allowed into the cache; the cache name pinned to a constant instead of
the digest; the clash rule turned into an overwrite; a rev-mismatched record rejected instead of
kept; the outbox draining on a failed send; Abandon deleting after a failed upload; the
`X-Requested-With` header removed; the multiplayer switch leaving one route served; `/enter` tied to
the multiplayer switch alone; the archive fact's lazy read removed so `archiveKind()` stays `"off"`;
Abandon treating "unknown" as "off"; `/sw.js` losing its `no-store`.

---

## 7. What this spec does NOT claim, and the LEAVE list

- **Nothing hosted.** No Workers, no objects, no R2. M6.5c inherits the outbox and the archive
  layout unchanged; a browser-sent save already lands under `browser/<caller>/rev7/{over,abandoned}/`
  and the bucket writes the same relative paths.
- **The public build's configuration is not decided** (R-6.5b2.6). Both switches are built and
  proven locally; which way the Worker runs is M6.5c's.
- **Access is not made secure by this cycle.** `claimed` still authenticates the door and not the
  seat, and the owner ruled to keep it on the NAS. What changes is that a build can now refuse it.
- **A missed Access application on a new protected route would be open**, because the NAS stays on
  `claimed`. Moving to `cf-access` is what makes that fail closed and is deliberately not taken
  here.
- **`POST /api/archive` still hands an untrusted `GameRecord` to the engine's setup path** (M6.5b
  close §6.3 item 6). This cycle keeps it behind the door rather than closing it.
- **Two tabs are still settled by Web Locks where it exists**, and the no-Web-Locks fallback is
  still invisible (M6.5b close §6.3 item 7).
- **Background suspension is not measured.** Whether a suspended tab keeps its lock is still
  unknown; the exit closes and reopens the browser, which is a different thing.
- **Storage is still best effort even when granted.** A grant reduces eviction; it does not prevent
  a player clearing site data, and no wording says otherwise.
- **The seven-day WebKit rule and the separate storage container for an installed iOS app are read
  from WebKit's own documentation**, not measured here. The exit exercises the install; it does not
  wait a week.
- **The e2e proves the mechanism in Chromium.** The phone is the only reading on WebKit and it is
  one device, by hand, once.
- **The expired-session read covers the STREAM too, and that is a gain rather than a gap.** A
  review of this spec proposed carrying "`EventSource` cannot send `X-Requested-With`, so an expired
  Access session mid-game loops against 302s" as a non-claim. It rests on a false premise: this
  client has never used `EventSource` — `sse.ts:102` is a `fetch` and the file's own first paragraph
  says why — so the stream carries the header like every other call and an expired session mid-game
  becomes the same readable 401. What was offered as a defect is closed by the same line of §2.1.
- **Inherited and untouched**: the vs-bot gate livelock, the e2e `CAP = 4000`, `walk.json`'s
  419 KB pretty-printed diff, "who starts" on the local form, and nothing in CI building the
  self-host image.

---

## 8. Carried to M6.5c

- Which configuration the public Worker runs (R-6.5b2.6), and therefore whether the card art is
  published.
- `cf-access` or another identity method in place of `claimed`, which is what makes a path-scoped
  door fail closed.
- The archive moving to R2, inheriting this cycle's outbox and the existing layout function.
- **The SPA-fallback cells must be re-proven on the Worker.** `/sw.js` 404ing rather than answering
  HTML, and `/enter` reaching the origin rather than the fallback, are proven here against axum's
  own map. Workers' static assets have their own `not_found_handling`, so the same two properties
  are a different mechanism there and a green cell in this cycle is not evidence about that one.
- The measurements the umbrella's §6.1 still owes: cold replay, a whole bot turn, `why` and the
  archive replay inside a live Free-tier object.

---

## 9. What the review changed (so a sentence can be traced to its reason)

An adversarial read of the first draft, 2026-09-21, before the plan. Its verdict was that the
mechanism held and the honesty list was sound; what it found was one contradiction I introduced in
my own self-review and three execution traps that would have surfaced mid-cycle.

- **The contradiction, and it was mine.** Making the archive independent of the multiplayer switch
  (§3.1) and tying `/enter` to that switch (§2.1) were two edits made in the same pass, and together
  they pointed the outbox's remedy at a 404 on exactly the deployment the spec describes. `/enter`
  now follows the OR of the two.
- **The archive fact had no read point.** Removing the boot's health read to satisfy R-6.5b2.7 left
  `archiveKind()` at its `"off"` default forever, which would have made §0.3's measured bug
  permanent for ONLINE single-player sessions — no send control, and Abandon deleting silently.
  §2.2 now names three lazy read points and §2.6 adds the three-state Abandon rule, where "unknown"
  is not "off".
- **"Not signed in" named one door of two.** Under `claimed` the turnstile is not enough; the
  in-app sign-in is the second step. The remedy is mode-aware.
- **The service worker would have run in all 46 existing e2e cells**, since `build:client` is a
  production build. §2.3 makes the fixture and its structural guard a design decision.
- **`storageState` cannot carry what cell 2 is about.** §6.3 names `launchPersistentContext`.
- **The profile move has three call sites and an output path that moves with it**, the Dockerfile's
  own wasm build among them. §2.7 names all of them and rules on `build:wasm:debug`.
- **Accepted as an improvement on the design, not a correction to it**: `backup` leaves wasm for the
  page. A module that will not load is exactly when a player most needs their games out, and an
  export living inside it would die with it. Only `restore_read` needs the engine.
- **Rejected, with evidence**: the `EventSource` non-claim (§7). The client reads the stream with
  `fetch` and has since M6.3.
- Smaller, all taken: `/sw.js` served `no-store`; a cell that a restored cross-rev game LISTS and
  not merely that the report calls it unplayable; the OFL texts committed beside the fonts and named
  in `NOTICE`; the superseded `data:,` icon; the corpus replay's new exemption; the 109-byte build
  variance and the `CLAUDE.md` art-count debt recorded in §0.3; the Workers `not_found_handling`
  re-proving carried to §8.
- **Checked and found not to bite**: the source scanner's per-item rule, which keys on regions
  opened by the literal `server` attribute and is blind to a `dev-identity` gate elsewhere. The one
  shape that would trip it is named in §2.7.
