# M6.5b2 "the front door, installable and offline" — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (Tasks 1, 2, 6, 8, 9, 10 — judgment-heavy) or superpowers:executing-plans (Tasks 3, 4, 5, 7, 11 — inline) to implement this plan task-by-task, per the owner's R-P11 refinement of 2026-09-20 (Opus is the floor for implementers and reviewers). Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A person opens the app with no login, plays the bot in their browser from open to close with no server present, and their games survive the browser, the device and the release.

**Architecture:** Three legs. A FRONT DOOR: Cloudflare Access moves from the hostname to five path prefixes, `/enter` becomes a turnstile whose only job is to make the browser acquire the cookie, and `#/` becomes a door with two ways on — so the shell, the module and the art are fetchable with no session. A SERVICE WORKER: a build script digests the release's own files into a cache name and writes `dist/sw.js`, the install is all-or-nothing, navigations are cache-first and an update never switches without a click. A WAY OUT AND BACK IN: one backup envelope carrying records as strings, validated in Rust by the same function the archive route uses, plus an outbox that retries a finished game's upload when the connection returns.

**Tech Stack:** Rust 1.97.1 (pinned), `wasm32-unknown-unknown`, `wasm-bindgen 0.2.128`, `wasm-opt` (binaryen 132); React + Vite + vitest (jsdom and a node project); Playwright 1.62.1; the Service Worker, Cache Storage, Web App Manifest and Storage APIs; Cloudflare Access and a cloudflared tunnel.

**Spec:** `docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md` (R-6.5b2.1–18; §0.3 is what was MEASURED and every task below cites it, §6 the instruments each task ends in, §9 what the adversarial read changed). The entry handover is `docs/plans/2026-09-21-web-frontend-m6.5b2-handover.md`; the M6.5b close `docs/plans/2026-09-21-web-frontend-m6.5b-close.md` is what HEAD IS.

## Global Constraints

- Branch `web-room-m6.5b2`, stacked on `web-room-m6.5b`; **base for every diff `369a1f2`**. Stage with explicit paths, never `git add -A`. A clean `git status` after every commit.
- **The five fenced crates and the CLI stay byte-identical**: `git diff --stat 369a1f2 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli` is EMPTY at every commit. `SEMANTICS_REV` stays **7**. No bot behaviour moves.
- **`[profile.release]` stays byte-identical** (R-6.5b2.15). The only profile this cycle adds is `[profile.release-wasm]`, which nothing selects unless it passes `--profile release-wasm`. Verify once at Task 5 with `git diff 369a1f2 -- Cargo.toml` showing an ADDED section and no changed line inside `[profile.release]` (there is none today to change).
- **Per-edit gates after EVERY task**: `cargo test -p yokohama-web` (**274 at the base**: 260 lib + 2 `browser_played_pin` + 2 whole-game gate + 10 `wire_fixtures` — **re-count at your own HEAD, never trust this figure**), `cargo test -p yokohama-wasm` (4), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`, `cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm`; and in `web/`: `npm run typecheck`, `npm test` (746 in 42 files at the base), `npm run test:wasm` (2). **`npm run e2e` is owed by ANY task that changes what a browser DOES, not only what it sees** — the lesson M6.5b's close records after Task 1 broke `the_playback.spec.ts` silently. That is Tasks 2, 3, 6, 7, 8, 9, 10 here.
- The whole-game gate's vs-bot arm **livelocks on some OS-entropy seeds** (M6.4 close §6.3): re-run it ALONE at the same commit before believing a red.
- **The corpus is a contract**: regenerate with `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` **ONLY at Task 1**, and READ the diff. The expected diff is stated there and is one key on the `health*` fixtures. Anything else in the diff is a finding, not a fixture to accept.
- **Every 400 sentence the layer's parsers produce is axum's word for word**; a new body goes through `parse_json`, a new query through `parse_query`; every request body carries `#[serde(deny_unknown_fields)]`.
- **The record and the table document are STRINGS end to end** and nothing in the browser parses either (the seed is a `u64` that `JSON.parse` rounds). That rule already governs `recordText`, `localPair`, the archive route and the IndexedDB rows, and it governs the envelope.
- **`.superpowers/sdd/2026-09-21-web-frontend-m6.5b2-plan/`** (gitignored) is the ledger: `progress.md` with one `Ruling:` line per deviation from this plan, and every review report written to a file whose PATH the agent reports back (the notification channel truncates at ~4k).
- Commit messages end with the EXECUTING agent's own attribution lines, never the plan author's.
- `cargo test` takes ONE positional filter: where a step names several cells, run one command per cell or a shared substring.

---

## File structure

**Created**
- `web/scripts/precache.mjs` — reads Vite's manifest and the art manifest, digests every file, writes `dist/sw.js` from the template, asserts both size ceilings (Task 6).
- `web/src/sw-template.js` — the worker's source, with `__PRECACHE__` and `__CACHE_NAME__` substituted by the script above. Never imported by the app (Task 6).
- `web/src/swRoute.ts` — the worker's routing DECISION as a pure function, so it is testable without a browser (Task 6).
- `web/src/swRegister.ts` — registration, the update offer, and the readiness state the door renders (Tasks 6, 7).
- `web/src/Door.tsx` **is MODIFIED, not created** (it exists since M6.5b as the server area's four states); the new first screen is `web/src/FrontDoor.tsx` (Task 3).
- `web/src/install.ts` — `beforeinstallprompt` capture, the iOS detection, the dismissal memory (Task 7).
- `web/src/storage.ts` — `persist()` / `persisted()` / `estimate()` behind three states (Task 7).
- `web/src/Backup.tsx` — the backup and restore controls and the restore report (Task 8).
- `web/src/backup.ts` — the envelope built page-side from strings; the restore's clash rule over IndexedDB (Task 8).
- `web/src/outbox.ts` — the pending store, the two reasons, the drain triggers (Task 9).
- `web/public/manifest.webmanifest`, `web/public/icons/*` — the app manifest and the original icons (Task 7).
- `web/public/fonts/*` + their OFL texts (Task 5).
- `crates/yokohama-web/src/backup.rs` — the envelope's types and `restore_read`, sharing one validator with `archive_post` (Task 8).
- `web/e2e/offline.spec.ts` — the six cells (Task 10).
- `web/e2e/sw.ts` — the fixture that turns the worker OFF, and the structural guard (Task 6).
- the close, dated the day it is written (Task 11).

**Modified**
- `crates/yokohama-web/src/identity.rs` (the `dev-identity` gate), `main.rs` (the multiplayer variable), `api.rs` (`Health.multiplayer`, the route table under the switch, `restore_read`'s dispatch is NOT added — the export is wasm-only), `assets.rs` (`cache_control` gains `/sw.js`), `lib.rs` (`GET /enter`, the router), `archive.rs`/`tables.rs` (the shared validator's call site).
- `Cargo.toml` (`[profile.release-wasm]`), `crates/yokohama-web/Cargo.toml` (the `dev-identity` feature), `crates/yokohama-wasm/src/lib.rs` (`restore_read`).
- `web/package.json` (`build:wasm`, `build`, `e2e`), `web/index.html` (the manifest link, the icons, the fonts, the `data:,` icon removed), `web/vite.config.ts` (`build.manifest`).
- `web/src/api.ts` (`X-Requested-With`, `settle`'s challenge arm, `api.art` retired), `web/src/App.tsx` (the three health read points, the front door route, the send control's rule), `web/src/Abandon.tsx` (the three-state rule), `web/src/local/facts.ts` (the remembered answer), `web/src/route.ts` (`#/lobby`), `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/OnThisDevice.tsx`, `web/src/Help.tsx`, `web/src/local/db.ts` (version 2, the `outbox` store), `web/src/local/client.ts`, `web/src/test/fakeWorker.ts`.
- `deploy/Dockerfile` (binaryen, the profile, both output paths), `deploy/cloudflared-config.yml`, `deploy/RUNBOOK.md`, `.github/workflows/test.yml`, `NOTICE`, `ASSETS.md`, `CLAUDE.md`, `docs/handover.md`.

---

## Task order and why

1–3 are the front door and must land before the deploy. **4 is the mid-cycle deploy** (R-6.5b2.16): the offline exit cannot be measured until the shell is fetchable without a login. 5 fixes what the release CONTAINS, so 6's precache list is the final one. 7 needs 6's readiness state. 8 and 9 are independent of 6 and of each other but both touch `OnThisDevice`. 10 is the e2e in one place because every cell shares the fixture. 11 is the phone, the close and the doc debts.

---

### Task 1: The three switches, the turnstile, and `/sw.js`'s header (R-6.5b2.2, .3, .4)

**Files:**
- Modify: `crates/yokohama-web/Cargo.toml` (the `dev-identity` feature in `default`)
- Modify: `crates/yokohama-web/src/identity.rs:67-95` (`from_env_value`'s `claimed` arm)
- Modify: `crates/yokohama-web/src/api.rs:231-256` (`Health`), `:126-165` (`routes()` takes the switch)
- Modify: `crates/yokohama-web/src/lib.rs:725-740` (the router gains `/enter`), and `Host`/`AppState` carry `multiplayer`
- Modify: `crates/yokohama-web/src/assets.rs:75-86` (`cache_control`), `:176-210` (the existing header cell)
- Modify: `crates/yokohama-web/src/main.rs:88-120` (the fourth variable)
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs` (regenerate; one key)
- Test: the test modules in `identity.rs`, `assets.rs`, `lib.rs`

**Interfaces:**
- Produces: `pub struct Multiplayer(pub bool)` with `pub fn from_env() -> Result<Self, String>` reading `YOKOHAMA_WEB_MULTIPLAYER` (`on` default, `off`, anything else an error naming the variable); `Health.multiplayer: bool`; `api::routes(multiplayer: bool) -> Vec<Route>`; `Host { identity_name, signal_kind, multiplayer: bool }`; `GET /enter -> 302 Location: /#/lobby`; `assets::cache_control` returning `Some("no-store")` for `/sw.js`.
- Consumes: `IdentityMode::from_env`, `ChatConfig::from_env`, `AppState::new`, the existing `Route` table and `dispatch`.

- [ ] **Step 1: Write the four failing cells**

In `identity.rs`'s test module:

```rust
/// R-6.5b2.3: `claimed` is a DEVELOPMENT door and a build can refuse it.
/// The feature is in `default`, so this cell runs in the accepting shape and
/// asserts the mode parses; the refusing shape is asserted by its negation
/// below, which compiles in both and is the one that would silently vanish if
/// the `cfg` were written the other way round.
#[test]
fn claimed_parses_only_where_the_development_feature_is_compiled_in() {
    let parsed = IdentityMode::from_env_value(Some("claimed"));
    if cfg!(feature = "dev-identity") {
        assert!(matches!(parsed, Ok(IdentityMode::Claimed)));
    } else {
        let err = parsed.expect_err("a build without dev-identity refuses claimed");
        assert!(err.contains("dev-identity"), "the refusal NAMES the feature: {err}");
        assert!(err.contains("cf-access") && err.contains("worker"), "and what to use instead: {err}");
    }
}
```

In `lib.rs`'s test module (beside the existing admission-grid cells):

```rust
/// R-6.5b2.4: OFF means NOT SERVED. Not gated, not hidden — the route is not
/// in the table, so an unknown path is the table's own 404 and there is no
/// second code path that "hides" anything.
#[tokio::test]
async fn multiplayer_off_serves_no_game_route_and_still_serves_the_public_ones() {
    let app = app_with_multiplayer(false).await;
    for uri in ["/api/games", "/api/games/x", "/api/games/x/input", "/api/chat", "/api/stream"] {
        let (status, _) = raw(app.clone(), "GET", uri, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri} must not be served with multiplayer off");
    }
    // **The anti-vacuity half**: a 404 everywhere would pass the loop above.
    for uri in ["/api/health", "/api/labels", "/api/art", "/api/me"] {
        let (status, _) = raw(app.clone(), "GET", uri, None).await;
        assert_eq!(status, StatusCode::OK, "{uri} is public and still answers");
    }
    // **And the archive is NOT one of them** (spec §3.1): it has its own
    // variable, so a single-player deployment can still collect saves.
    let (status, _) = raw(app.clone(), "POST", "/api/archive", Some("{}")).await;
    assert_ne!(status, StatusCode::NOT_FOUND, "the archive is governed by its own variable");
}

/// The turnstile (R-6.5b2.2). It exists so a browser can ACQUIRE the Access
/// cookie by navigating; the origin's only job is to send it back.
#[tokio::test]
async fn enter_is_a_turnstile_back_to_the_lobby() {
    let app = app_with_multiplayer(true).await;
    let (status, location) = raw_location(app, "GET", "/enter").await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(location.as_deref(), Some("/#/lobby"));
}

/// Its condition is the OR and not the multiplayer switch alone (spec §2.1,
/// §9): with multiplayer off and an archive on it is STILL served, because
/// the archive is behind the door and the outbox's remedy links here.
#[tokio::test]
async fn enter_follows_any_protected_route_and_not_multiplayer_alone() {
    let (status, _) = raw_location(app_with(false, /*archive*/ true).await, "GET", "/enter").await;
    assert_eq!(status, StatusCode::FOUND, "multiplayer off + archive on still needs the door");
    let (status, _) = raw_location(app_with(false, /*archive*/ false).await, "GET", "/enter").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "nothing protected, no turnstile");
}
```

In `assets.rs`'s test module — **widen the existing cell rather than adding a second one**, so the "nothing else carries a header" clause stays true and is not quietly split in two:

```rust
#[test]
fn hashed_assets_are_immutable_the_worker_is_no_store_and_nothing_else_carries_a_header() {
    assert_eq!(cache_control("/assets/index-abc123.js"), Some("public, max-age=31536000, immutable"));
    // Update detection is a BYTE COMPARISON of this one file, so a browser
    // that heuristically caches it delays every update by the cache window —
    // and with no header at all, heuristic caching is exactly what it may do.
    assert_eq!(cache_control("/sw.js"), Some("no-store"));
    for path in ["/", "/index.html", "/manifest.webmanifest", "/art/manifest.json", "/help/screen.webp"] {
        assert_eq!(cache_control(path), None, "{path} carries no freshness header");
    }
}
```

- [ ] **Step 2: Run them and watch them fail**

```bash
cargo test -p yokohama-web claimed_parses_only
cargo test -p yokohama-web multiplayer_off_serves
cargo test -p yokohama-web enter_is_a_turnstile
cargo test -p yokohama-web enter_follows_any_protected
cargo test -p yokohama-web hashed_assets_are_immutable
```

Expected: the first four fail to COMPILE (no `app_with_multiplayer`, no `raw_location`, no feature), the fifth fails on `/sw.js`.

- [ ] **Step 3: The feature and the identity gate**

`crates/yokohama-web/Cargo.toml`: `default = ["server", "dev-identity"]`, `dev-identity = []`. In `identity.rs`, the `"claimed"` arm of `from_env_value` becomes a `#[cfg(feature = "dev-identity")]` / `#[cfg(not(...))]` pair, the refusing arm naming the feature and the two production modes. **Do not put the gate inside a `#[cfg(feature = "server")]` region** — the source scanner reads a combined attribute on a column-0 item inside a region as ungated (spec §2.7).

- [ ] **Step 4: The multiplayer switch**

`Multiplayer::from_env` in `main.rs`'s style (a bad value exits 1 naming the variable, like `ChatConfig`); `AppState` and `api::Host` carry it; `api::routes(multiplayer)` omits the game, chat and stream rows when false. `Health.multiplayer` reports it.

- [ ] **Step 5: The turnstile and the header**

`GET /enter` on the axum router in `lib.rs`, answering `302` with `Location: /#/lobby` when `multiplayer || archive_kind != "off"`, and not registered at all otherwise. `cache_control` gains the `/sw.js` arm.

- [ ] **Step 6: Run the cells, then the whole binary**

```bash
cargo test -p yokohama-web
cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 7: Regenerate the corpus and READ the diff**

```bash
UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures
git diff --stat web/src/wire/
```

**Expected and nothing else**: `health.json` gains `"multiplayer": true`. Every other file byte-identical. A changed `ClientState`, `TableView` or `HistoricalFrame` fixture is a FINDING — stop and report it.

- [ ] **Step 8: The wasm corpus replay needs the new field EXEMPTED, and the exemption must be earned**

`Health` is one struct and the browser host builds its own, so the module now answers `multiplayer`
too — and it answers **`false`**, because a single local game has no multiplayer in it. That is a
real difference, so the field joins `health.identity`, `health.signal` and `me.mode` in
`web/src/local/corpus.node.test.ts`'s exemption list, and it satisfies the M6.5b close's own rule
for that list: an exempted field must really DIFFER between the two hosts or the replay refuses.

```bash
cd web && npm run build:wasm && npm run test:wasm
```

Expected: 2 passing. **A green run with `multiplayer` exempted but identical on both hosts is a
finding** — it would mean the module is claiming a multiplayer it does not serve.

- [ ] **Step 9: Prove the refusing build compiles and refuses**

```bash
cargo test -p yokohama-web --no-default-features --features server claimed_parses_only
```

Expected: PASS, exercising the `else` branch of the cell in Step 1.

- [ ] **Step 10: Commit**

```bash
git add crates/yokohama-web/Cargo.toml crates/yokohama-web/src/{identity,api,lib,assets,main}.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire/health.json web/src/local/corpus.node.test.ts
git commit -m "feat(web): the three deployment switches, the /enter turnstile and no-store on the worker script"
```

---

### Task 2: Zero Worker invocations, and an expired session that is READ (R-6.5b2.7, R-6.5b2.2)

**Files:**
- Modify: `web/src/api.ts:280-300` (`call`'s headers), `:316-345` (`settle`), `:480-492` (`api.art` retired, `api.health` unchanged)
- Modify: `web/src/App.tsx:735-760` (the boot reads)
- Modify: `web/src/local/facts.ts:56-85` (the remembered answer)
- Modify: `web/src/types.ts` (`Health.multiplayer`), `web/src/wire/contract.ts`
- Test: `web/src/api.test.ts`, `web/src/App.test.tsx`, a new `web/src/local/facts.test.ts`

**Interfaces:**
- Produces: `export const ACCESS_CHALLENGE: string` (the sentence the door renders); `settle<T>(status, statusText, text, challenged: boolean)`; `rememberServerFacts(h: Health): void` and `recallServerFacts(): ServerFacts | null` in `facts.ts`; `ensureArchiveFact(): Promise<ArchiveKind | "unknown">` in `api.ts` — the bounded, remembered read
  §2.2 names, whose `"unknown"` is Task 9's third Abandon state and is **not** a synonym for `"off"`;
  `ServerFacts` gains `identity: string` (Task 9's remedy is mode-aware and reads it) and
  `multiplayer: boolean`; `resetFactsForTest()` for the cells.
- Consumes: Task 1's `Health.multiplayer`; `isLocal`, `facts`, `patchServerFacts`.

- [ ] **Step 1: Write the failing cells**

In `web/src/api.test.ts`:

```ts
// spec §0.3: `X-Requested-With` is the ONE header that turns Access's redirect
// into a same-origin 401 whose headers this client can read. Measured against
// the live deployment; `Accept`, `Sec-Fetch-Mode` and `Sec-Fetch-Dest` do not.
it("sends X-Requested-With on every server call", async () => {
  const spy = vi.fn().mockResolvedValue(new Response("{}", { status: 200 }));
  vi.stubGlobal("fetch", spy);
  await api.health();
  expect(spy.mock.calls[0]![1].headers["X-Requested-With"]).toBe("XMLHttpRequest");
});

// The two 401s mean different things and must not render the same sentence.
it("tells the EDGE's 401 from the API's own", async () => {
  const edge = new Response("<html>", {
    status: 401,
    headers: { "www-authenticate": 'Cloudflare-Access resource_metadata="https://x/.well-known/y"' },
  });
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(edge));
  await expect(api.health()).rejects.toThrow(ACCESS_CHALLENGE);

  const own = new Response(JSON.stringify({ error: "no signed-in user" }), { status: 401 });
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(own));
  await expect(api.health()).rejects.toThrow("no signed-in user");
});
```

In `web/src/local/facts.test.ts` (new):

```ts
// spec §2.2 and §9: the archive fact drives the whole LOCAL channel, and a
// spec that removed the boot read and gave it nothing would make §0.3's bug
// permanent for ONLINE sessions. Three read points; this is the contract.
it("asks once, remembers, and never asks again", async () => {
  const spy = vi.fn().mockResolvedValue({ ...wire.health, archive: "dir" });
  vi.spyOn(api, "health").mockImplementation(spy);
  expect(await ensureArchiveFact()).toBe("dir");
  expect(await ensureArchiveFact()).toBe("dir");
  expect(spy).toHaveBeenCalledTimes(1);
});

it("survives a reload without asking, because the answer is remembered", () => {
  rememberServerFacts({ ...wire.health, archive: "dir" } as Health);
  resetFactsForTest();                       // what a fresh page load leaves
  expect(recallServerFacts()?.archive).toBe("dir");
});

it("reports the archive UNKNOWN rather than off when it has never asked", () => {
  resetFactsForTest();
  window.localStorage.clear();
  expect(recallServerFacts()).toBeNull();    // NOT `{archive: "off"}`
});
```

In `web/src/App.test.tsx`:

```ts
// R-6.5b2.7: the door costs the Worker nothing. The art manifest is a static
// file and the labels come from the local host, so neither is a server route.
it("the first screen asks the server for nothing", async () => {
  const spy = vi.fn().mockRejectedValue(new Error("no server in this test"));
  vi.stubGlobal("fetch", spy);
  render(<App />);
  await screen.findByText(/On this device/);
  const asked = spy.mock.calls.map((c) => String(c[0]));
  expect(asked.filter((u) => u.startsWith("/api/"))).toEqual([]);
  // The anti-vacuity half: it DID fetch, just not the API.
  expect(asked).toContain("/art/manifest.json");
});
```

- [ ] **Step 2: Run them and watch them fail**

```bash
cd web && npx vitest run src/api.test.ts src/local/facts.test.ts -t "X-Requested-With"
npx vitest run src/App.test.tsx -t "asks the server for nothing"
```

- [ ] **Step 3: The header and the challenge read**

`call` adds `"X-Requested-With": "XMLHttpRequest"` to `headers` (server arm only — the local arm has no fetch). `settle` gains a fourth parameter and, when it is true and the status is 401, throws `ApiError` carrying `ACCESS_CHALLENGE`. The call site reads `res.headers.get("www-authenticate")?.startsWith("Cloudflare-Access")`.

- [ ] **Step 4: The art manifest and the labels**

`api.art` is deleted; `App.tsx:747` fetches `/art/manifest.json` directly and parses it as `ArtManifest`. The labels chain at `App.tsx:740` becomes local-first: `api.labels("local")` with the server read kept only for a signed-in multiplayer session.

- [ ] **Step 5: The remembered facts and the three read points**

`facts.ts` gains `rememberServerFacts` / `recallServerFacts` over one `localStorage` key (wrapped in try/catch — a private window throws), and `server` is seeded from `recallServerFacts()` at module load. `ensureArchiveFact()` in `api.ts` returns the remembered value, else does ONE bounded `api.health()`, remembers it and returns it; a read that does not answer returns `"unknown"`. `ServerFacts` gains `identity` and `multiplayer` so `setServerFacts` carries the whole of Task 1's `Health`.

`App.tsx` drops the unconditional boot health read and calls `ensureArchiveFact()` at **all three
points §2.2 names**: entering multiplayer (the lobby needs the signal and chat facts anyway), a
local game reaching game over, and — in Task 9 — Abandon being clicked on a local game.

- [ ] **Step 6: Run the suites, including the browser walk**

```bash
cd web && npm run typecheck && npm test && npm run e2e
```

`npm run e2e` is owed here because this task changes what the browser FETCHES.

- [ ] **Step 7: Commit**

```bash
git add web/src/api.ts web/src/App.tsx web/src/local/facts.ts web/src/local/facts.test.ts web/src/api.test.ts web/src/App.test.tsx web/src/types.ts web/src/wire/contract.ts
git commit -m "feat(web client): the door costs the Worker nothing, and an expired Access session is read rather than guessed"
```

---

### Task 3: The front door at `#/` (R-6.5b2.1)

**Files:**
- Create: `web/src/FrontDoor.tsx`, `web/src/FrontDoor.test.tsx`
- Modify: `web/src/route.ts:29-47` (a `lobby` route reachable at `#/lobby`, `#/` becoming the door), `web/src/App.tsx` (the route switch), `web/src/OnThisDevice.tsx` (its new host), `web/src/Help.tsx`
- Test: `web/src/route.test.ts`, `web/src/App.test.tsx`

**Interfaces:**
- Produces: `Route` gains `{ kind: "door" }`; `parseRoute("#/") === {kind:"door"}` and `parseRoute("#/lobby") === {kind:"lobby"}`; `<FrontDoor multiplayer={boolean} />`. **It takes no readiness prop yet**: `Readiness` is Task 6's
  type and a forward reference to it here would be a name this task cannot define. Task 6 adds the
  prop and the line it renders.
- Consumes: Task 1's `Health.multiplayer`, Task 2's `recallServerFacts`.

- [ ] **Step 1: Write the failing cells**

```ts
// R-6.5b2.1: everyone sees the door. `#/` is no longer the lobby, and the
// lobby keeps a route of its own so `/enter` has somewhere to send people.
it("routes # to the door and #/lobby to the lobby", () => {
  expect(parseRoute("#/")).toEqual({ kind: "door" });
  expect(parseRoute("#/lobby")).toEqual({ kind: "lobby" });
  expect(parseRoute("")).toEqual({ kind: "door" });      // parseRoute stays TOTAL
});

// "Play with others" is a LINK and never a fetch, so the browser handles the
// Access login the way it handles any navigation.
it("offers multiplayer as a navigation to the turnstile", () => {
  render(<FrontDoor multiplayer />);
  const link = screen.getByRole("link", { name: /play with others/i });
  expect(link).toHaveAttribute("href", "/enter");
});

// R-6.5b2.4 on the client side: nothing to enter, nothing offered, and one
// line saying so rather than a control that 404s.
it("draws no multiplayer entry when the deployment serves none", () => {
  render(<FrontDoor multiplayer={false} />);
  expect(screen.queryByRole("link", { name: /play with others/i })).toBeNull();
  expect(screen.getByText(/single player only/i)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run them and watch them fail**

```bash
cd web && npx vitest run src/route.test.ts src/FrontDoor.test.tsx
```

- [ ] **Step 3: The route**

`Route` gains `door`; `parseRoute` maps `/` and anything unrecognized to it (staying total); `hashOf` maps it to `#/`; `#/lobby` is the lobby.

- [ ] **Step 4: The screen**

`FrontDoor.tsx` renders "On this device" (the existing component, moved not copied) and the multiplayer link or its absent-line. **No readiness line in this task** — Task 6 adds it along with the type that describes it.

- [ ] **Step 5: Run everything including the walk**

```bash
cd web && npm run typecheck && npm test && npm run e2e
```

**Expect existing e2e specs to need their entry updated** — a spec that starts at `#/` and immediately looks for the lobby now lands on the door. That is this task's work, not a surprise: fix each spec's entry to `#/lobby` or to a click on the door, and say in the commit which specs moved.

- [ ] **Step 6: Commit**

```bash
git add web/src/FrontDoor.tsx web/src/FrontDoor.test.tsx web/src/route.ts web/src/route.test.ts web/src/App.tsx web/src/App.test.tsx web/src/OnThisDevice.tsx web/src/Help.tsx web/e2e/
git commit -m "feat(web client): #/ is a front door with two ways on"
```

---

### Task 4: The hostname move, the five Access applications, and the MID-CYCLE DEPLOY (R-6.5b2.5, R-6.5b2.16)

**Files:**
- Modify: `deploy/cloudflared-config.yml` (the ingress block), `deploy/RUNBOOK.md` (a new §3.14)
- No source changes. **If this task wants one, stop and report it** — the whole point of R-6.5b2.2 is that the app does not know about Access.

**Interfaces:** none in code. Produces a deployed release and the readings §3.2 requires.

- [ ] **Step 1: Build and deploy the release** following `deploy/RUNBOOK.md` §1–§3, including the preflight `grep -c debug_trap web/src/local/wasm/yokohama_wasm.js` reading **0** after `npm run build`.

- [ ] **Step 2: DNS and ingress.** A proxied CNAME `yokohama-nas.moore.nyc` to the tunnel; an ingress block above the catch-all; `docker restart cloudflared-nas`. Leave `yokohama.moore.nyc`'s block in place until Step 4 passes.

- [ ] **Step 3: The five Access applications** on `yokohama-nas.moore.nyc`, each on one prefix — `/api/games`, `/api/chat`, `/api/stream`, `/api/archive`, `/enter` — with the owner's existing allow-list policy. **No application on the hostname itself.** This is owner-operated in the Cloudflare dashboard; the agent records what was configured, it does not configure it.

- [ ] **Step 4: Verify each property, from a shell with no session, and paste the readings into the RUNBOOK**

```bash
UA='Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/153 Safari/537.36'
H=https://yokohama-nas.moore.nyc
for p in / /assets/ /art/manifest.json /api/health /api/labels /api/me; do
  printf '%-22s ' "$p"; curl -sS -o /dev/null -w '%{http_code}\n' -A "$UA" "$H$p"
done
# protected: a navigation is a 302, an XHR is a 401 with the header (spec §0.3)
for p in /api/games /enter; do
  printf '%-22s nav=' "$p"; curl -sS -o /dev/null -w '%{http_code}' -A "$UA" "$H$p"
  printf '  xhr='; curl -sS -o /dev/null -w '%{http_code}\n' -A "$UA" -H 'X-Requested-With: XMLHttpRequest' "$H$p"
done
```

Expected: the first six **200**; each protected path **302** for the navigation and **401** for the XHR.

- [ ] **Step 5: Walk the deployed site by hand** — the door draws, "Play on this device" starts a local game with no sign-in, "Play with others" goes through Access and lands on the lobby, and a signed-in game still plays. **Headed, not headless**: the M6.4 close's §4.5 records that four headless reviews missed a 404 every headed one saw.

- [ ] **Step 6: Retire the old hostname's ingress block** and record in the RUNBOOK that `yokohama.moore.nyc` is now **held for the Worker** and that local games did not travel.

- [ ] **Step 7: Commit**

```bash
git add deploy/cloudflared-config.yml deploy/RUNBOOK.md
git commit -m "deploy: the NAS takes yokohama-nas.moore.nyc behind five path-scoped Access applications"
```

---

### Task 5: What the release CONTAINS — the module's profile, the help pictures, the fonts (R-6.5b2.15, R-6.5b2.18)

**Files:**
- Modify: `Cargo.toml` (`[profile.release-wasm]`), `web/package.json` (`build:wasm`, `build:wasm:debug` unchanged), `deploy/Dockerfile:70-76`, `.github/workflows/test.yml` (no change needed; verify)
- Create: `docs/tools/mkhelpart.py`, `web/public/help/*.webp`, `web/public/fonts/*` + `OFL.txt` per family
- Modify: `web/index.html` (the font links, the `data:,` icon), `web/src/styles.css` (`@font-face`), `NOTICE`, `ASSETS.md`
- Delete: `web/public/help/*.png`

**Interfaces:**
- Produces: a release module at `target/wasm32-unknown-unknown/release-wasm/yokohama_wasm.wasm`, ~1.17 MB raw; help pictures as WebP at the same five names; two self-hosted families.
- Consumes: nothing from earlier tasks.

- [ ] **Step 1: Add the profile and move all three call sites**

```toml
# Cargo.toml, at the root. Nothing selects it without `--profile release-wasm`,
# so `[profile.release]` and the fifteen-gate roster are untouched.
[profile.release-wasm]
inherits = "release"
opt-level = "z"
lto = "fat"
codegen-units = 1
```

`web/package.json`'s `build:wasm`: `--profile release-wasm`, **and both `wasm-bindgen` input paths become `../target/wasm32-unknown-unknown/release-wasm/yokohama_wasm.wasm`**. `deploy/Dockerfile:74-76` carries its OWN cargo and `wasm-bindgen` lines: both move the same way, and the stage installs `binaryen`. **`build:wasm:debug` stays on plain `release`** (spec §2.7).

- [ ] **Step 2: Add the `wasm-opt` pass and its refusal**

After `wasm-bindgen`, run `wasm-opt -Oz --strip-debug --strip-producers` over each emitted `_bg.wasm` in place. Absent, the script **exits 1** naming `brew install binaryen` and `apt-get install binaryen`; `YOKOHAMA_WASM_OPT_OPTIONAL=1` turns that into a build that PRINTS that it produced an unoptimized module.

- [ ] **Step 3: Verify the size and the bot, and record both**

```bash
cd web && npm run build:wasm
ls -l src/local/wasm/yokohama_wasm_bg.wasm     # expect ~1,168,913 B (spec §0.3)
npm run test:wasm                               # the corpus replay still passes byte for byte
cargo test -p yokohama-wasm
```

The corpus replay passing is the load-bearing check here: it is the claim that an aggressively optimized module still answers every one of the walk's 2,197 rows identically.

- [ ] **Step 4: Re-cut the help pictures**

`docs/tools/mkhelpart.py` in the style of `mkcardart.py` (Pillow), reading the five PNGs and writing WebP at the same base names. Commit the WebP, delete the PNG, update the five references. Expect ~4.2 MB → under 1 MB; record the real figure.

- [ ] **Step 5: Self-host the fonts**

Download the Fraunces and JetBrains Mono `woff2` files into `web/public/fonts/`, add `@font-face` rules to `styles.css`, remove the two `preconnect` links and the Google stylesheet `<link>` from `index.html`, and remove `<link rel="icon" href="data:,">` (superseded by Task 7's icons). **Commit each family's OFL text beside its files, name them in `NOTICE`, and add an `ASSETS.md` line pointing at the path** — that file today lists only what carries NO grant, and a granted third-party asset needs its grant recorded where a reader will look.

- [ ] **Step 6: Build, look at it, and run the walk**

```bash
cd web && npm run build && npm run e2e
```

Then open the built client and **look at the help panel and the typefaces**. The lane's own rule: regenerating art and not looking proves nothing.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml web/package.json deploy/Dockerfile docs/tools/mkhelpart.py web/public/help web/public/fonts web/index.html web/src/styles.css NOTICE ASSETS.md
git commit -m "build: the module takes a profile of its own, the help pictures become WebP, and the fonts come in-house"
```

---

### Task 6: The precache generator and the service worker (R-6.5b2.8)

**Files:**
- Create: `web/scripts/precache.mjs`, `web/scripts/precache.test.ts`, `web/src/sw-template.js`, `web/src/swRoute.ts`, `web/src/swRoute.test.ts`, `web/src/swRegister.ts`, `web/src/swRegister.test.ts`, `web/e2e/sw.ts`, `web/e2e/every_spec_declares_its_worker.spec.ts`
- Modify: `web/vite.config.ts` (`build.manifest: true`), `web/package.json` (`build:client` runs the generator after `vite build`), `web/src/main.tsx` (registration)

**Interfaces:**
- Produces: `Readiness = "installing" | "ready" | "failed" | "none"` **and the `readiness` prop on
  `FrontDoor`, added here** (Task 3 deliberately left it off rather than forward-reference this
  type); `buildPrecache({viteManifest, artManifest, extras, readFile}) -> {cacheName: string, entries: {path: string, digest: string}[]}`; `swRoute(url: URL, mode: string, precached: Set<string>) -> "bypass" | "cache" | "navigate" | "network"`; `useReadiness(): {readiness: Readiness, updateWaiting: boolean, applyUpdate: () => void}`.
- Consumes: Task 5's final file set (the list must be cut from the FINAL release, not an earlier one).

- [ ] **Step 1: Write the failing generator cells**

```ts
// spec §2.3: the cache name IS the content, so nothing has to be bumped.
it("changes the cache name when any file's bytes change", () => {
  const read = (p: string) => (p.endsWith("a.webp") ? "AAA" : "BBB");
  const a = buildPrecache({ ...fixture, readFile: read });
  const b = buildPrecache({ ...fixture, readFile: (p) => (p.endsWith("a.webp") ? "AAA!" : "BBB") });
  expect(a.cacheName).not.toEqual(b.cacheName);
});

// And it is DETERMINISTIC, or every build would invalidate every browser.
it("gives the same name for the same inputs", () => {
  expect(buildPrecache(fixture).cacheName).toEqual(buildPrecache(fixture).cacheName);
});

// The art files have STABLE names (art.rs:112 serves them max-age=3600 for
// exactly that reason), so a hashed-filename scheme alone would never reach a
// browser that had already cached a re-cut card. This is the cell that proves
// the digest earns its place.
it("busts the cache for a re-cut card, whose filename never changes", () => {
  const before = buildPrecache({ ...fixture, readFile: () => "old card" });
  const after = buildPrecache({ ...fixture, readFile: () => "new card" });
  expect(before.entries.map((e) => e.path)).toEqual(after.entries.map((e) => e.path));
  expect(before.cacheName).not.toEqual(after.cacheName);
});

// Completeness against BOTH manifests, so a new art family or a new Vite
// chunk cannot be silently left out of "available offline".
it("lists every file both manifests name", () => {
  const { entries } = buildPrecache(fixture);
  const paths = new Set(entries.map((e) => e.path));
  for (const f of Object.values(fixture.artManifest).flatMap((g) => Object.values(g)))
    expect(paths).toContain(`/art/${(f as { file: string }).file}`);
  for (const c of Object.values(fixture.viteManifest)) expect(paths).toContain(`/${(c as { file: string }).file}`);
});

// spec §2.7: the budget lives HERE, in the thing that runs on every release
// build, and not in a test someone might not run.
it("refuses a module over its ceiling", () => {
  expect(() => buildPrecache({ ...fixture, readFile: () => "x".repeat(2_000_000) })).toThrow(/ceiling/);
});
```

- [ ] **Step 2: Write the failing routing cells**

```ts
// The three rules, and the first is the one that matters most: a cached API
// reply would be a stale game state served as if it were live.
it("never intercepts the API or the turnstile", () => {
  for (const u of ["/api/health", "/api/games/x/input", "/api/stream", "/enter"])
    expect(swRoute(new URL(u, "https://h"), "cors", PRE)).toBe("bypass");
});
it("serves a precached path from the cache and a navigation from the shell", () => {
  expect(swRoute(new URL("/assets/index-abc.js", "https://h"), "cors", PRE)).toBe("cache");
  expect(swRoute(new URL("/t/anything", "https://h"), "navigate", PRE)).toBe("navigate");
});
it("lets anything else go to the network uncached", () => {
  expect(swRoute(new URL("/unknown.json", "https://h"), "cors", PRE)).toBe("network");
});
```

- [ ] **Step 3: Run both and watch them fail**

```bash
cd web && npx vitest run scripts/precache.test.ts src/swRoute.test.ts
```

- [ ] **Step 4: Implement the generator and the template**

`buildPrecache` is pure over its inputs. `precache.mjs` is the thin shell that reads the real files and writes `dist/sw.js` by substituting `__PRECACHE__` and `__CACHE_NAME__` into `src/sw-template.js`. The template's `install` fetches every entry with `redirect: "error"`, checks `200` and the content type, and fails the whole install on one failure. `activate` deletes only caches whose name carries this app's prefix and is not the current one.

- [ ] **Step 5: Registration, the readiness state and the update offer**

`swRegister.ts` registers only when `import.meta.env.PROD` **and** the flag in Step 6 is not set. It exposes `useReadiness()`; a `waiting` worker sets `updateWaiting`, and `applyUpdate()` posts `skipWaiting` and reloads. **Nothing calls `skipWaiting` on install** — the switch is the player's click and never happens mid-move.

- [ ] **Step 6: The e2e fixture and its structural guard**

`web/e2e/sw.ts` exports a fixture whose `addInitScript` sets `window.__YOKOHAMA_NO_SW__ = true`. **Every spec takes it except the offline one**, or 46 existing cells would each register a worker and start a ten-megabyte precache. `every_spec_declares_its_worker.spec.ts` reads the spec directory AT RUN TIME and names any file that neither imports the fixture nor opts in — the shape `every_spec_can_skip.spec.ts` already uses, and for the same reason: a convention written down once gets broken.

- [ ] **Step 7: Build, run everything, and read the numbers the generator prints**

```bash
cd web && npm run build && npm run typecheck && npm test && npm run e2e
```

Record the release total and the module size the generator prints. Expect about 10 MB (spec §2.7).

- [ ] **Step 8: Commit**

```bash
git add web/scripts web/src/sw-template.js web/src/swRoute.ts web/src/swRoute.test.ts web/src/swRegister.ts web/src/swRegister.test.ts web/src/main.tsx web/vite.config.ts web/package.json web/e2e/sw.ts web/e2e/every_spec_declares_its_worker.spec.ts web/e2e/
git commit -m "feat(web client): a release cached whole, keyed by its own contents, updating only on a click"
```

---

### Task 7: Installable and persistent (R-6.5b2.9)

**Files:**
- Create: `web/public/manifest.webmanifest`, `web/public/icons/{192,512,512-maskable,180-apple}.png`, `web/src/install.ts`, `web/src/install.test.ts`, `web/src/storage.ts`, `web/src/storage.test.ts`
- Modify: `web/index.html` (the manifest and apple-touch links), `web/src/FrontDoor.tsx`, `web/src/OnThisDevice.tsx`, `web/src/local/client.ts` (the persist call at the first committed move)

**Interfaces:**
- Produces: `captureInstallPrompt(): void`, `installState(): "available" | "ios-instructions" | "installed" | "none"`, `dismissInstall(): void`; `requestPersistence(): Promise<void>`, `storageReport(): Promise<{persisted: boolean, usage: number, quota: number} | null>`.
- Consumes: Task 6's `Readiness`, Task 3's `FrontDoor`.

- [ ] **Step 1: Write the failing cells**

```ts
// Two platforms, two mechanisms, and neither is nagged.
it("offers a real button where the browser offers one", () => {
  fireEvent(window, new Event("beforeinstallprompt"));
  expect(installState()).toBe("available");
});
it("shows the iOS instruction only in Safari on iOS and only when not installed", () => {
  stubIosSafari();
  expect(installState()).toBe("ios-instructions");
  stubStandalone();                                  // already added to the home screen
  expect(installState()).toBe("installed");
});
it("never shows it again once dismissed", () => {
  stubIosSafari();
  dismissInstall();
  expect(installState()).toBe("none");
});

// The sentence that must be on the screen that offers the install: an
// installed iOS app has its own storage container and shares NOTHING with
// Safari, so the games played here do not travel (spec §2.4).
it("says the installed app will not see these games, beside a way to carry them", () => {
  stubIosSafari();
  render(<OnThisDevice games={[localGame]} />);
  expect(screen.getByText(/will not see the games/i)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /back up/i })).toBeInTheDocument();
});

// Persistence is asked at the first committed move and never at boot, so a
// visitor who never plays is never asked.
it("asks for persistence at the first committed move and not at boot", async () => {
  const persist = vi.fn().mockResolvedValue(true);
  vi.stubGlobal("navigator", { ...navigator, storage: { persist, persisted: async () => false } });
  render(<App />);
  await screen.findByText(/On this device/);
  expect(persist).not.toHaveBeenCalled();
  await playOneLocalMove();
  expect(persist).toHaveBeenCalledTimes(1);
});
```

- [ ] **Step 2: Run them and watch them fail**

```bash
cd web && npx vitest run src/install.test.ts src/storage.test.ts src/OnThisDevice.test.tsx
```

- [ ] **Step 3: The manifest and the icons**

Draw the icons in-repo (an SVG source committed beside the rasters). **Never a crop of the card art** — `ASSETS.md` carves those out and an app icon is the most distributed image a product has. `display: standalone`, an explicit `id`, colours read from the stylesheet's tokens.

- [ ] **Step 4: `install.ts` and `storage.ts`**, with every storage accessor in `try`/`catch` (a private window throws) and every report rendering without them.

- [ ] **Step 5: Wire them into the two screens**, and add the persist call to the first committed local move in `local/client.ts`.

- [ ] **Step 6: Run everything, then look at it on a phone-width viewport**

```bash
cd web && npm run typecheck && npm test && npm run e2e
```

- [ ] **Step 7: Commit**

```bash
git add web/public/manifest.webmanifest web/public/icons web/src/install.ts web/src/install.test.ts web/src/storage.ts web/src/storage.test.ts web/src/FrontDoor.tsx web/src/OnThisDevice.tsx web/src/OnThisDevice.test.tsx web/src/local/client.ts web/index.html
git commit -m "feat(web client): installable on both platforms, and persistence asked for when a game is worth keeping"
```

---

### Task 8: Backup and restore (R-6.5b2.10, .11, .12)

**Files:**
- Create: `crates/yokohama-web/src/backup.rs`, `web/src/backup.ts`, `web/src/backup.test.ts`, `web/src/Backup.tsx`, `web/src/Backup.test.tsx`
- Modify: `crates/yokohama-web/src/lib.rs` (`mod backup`), `crates/yokohama-web/src/api.rs` (`archive_post` calls the shared validator), `crates/yokohama-wasm/src/lib.rs` (`restore_read`), `web/src/local/worker.ts` (a `restore` message), `web/src/local/db.ts`, `web/src/OnThisDevice.tsx`, `web/src/App.tsx`
- Test: the test module in `backup.rs`, `crates/yokohama-wasm/src/lib.rs`'s native cells

**Interfaces:**
- Produces, in Rust: `pub struct Envelope { kind, envelope: u32, created_at: u64, origin: String, app_version: String, semantics_rev: u32, rules_hash: String, games: Vec<BackupGame> }`; `pub struct BackupGame { id: String, record: String, table: String }`; `pub enum Verdict { Accepted { players: u8, events: usize, over: bool }, Unplayable { reason: String }, Rejected { reason: String } }`; `pub struct PairFacts { pub players: u8, pub events: usize, pub over: bool, pub semantics_rev: u32 }`;
  `pub fn check_pair(record: &str, table: &str) -> Result<PairFacts, String>` — **the one function
  `archive_post` also calls**, and `pub(crate) fn archive_would_accept(record: &str, table: &str) -> bool`
  beside it for the cell that compares the two callers; `pub fn restore_read(envelope: &str) -> Result<Vec<(String, Verdict)>, String>`.
- Produces, in the module: `restore_read(envelope: &str) -> Result<String, JsValue>` (the report as JSON).
- Produces, in the page: `buildEnvelope(pairs: {id, record, table}[]): string` — **strings in, string out, nothing parsed**; `restorePairs(envelope: string): Promise<RestoreReport>`, where
  `RestoreReport = { games: { id: string, outcome: "restored" | "already-here" | "clash" | "unplayable" | "rejected", reason?: string }[] }` —
  the five outcomes the report renders, and the only vocabulary `Backup.tsx` knows.
- Consumes: `localPair` (M6.5b), `TableDoc::agrees_with`, `replay_strict`.

- [ ] **Step 1: Write the failing Rust cells**

```rust
/// R-6.5b2.10: one validator, two callers. A backup and an upload cannot
/// disagree about what a valid game is, and this cell is what keeps them
/// from drifting — it calls BOTH and compares the verdict.
#[test]
fn the_archive_and_the_envelope_judge_a_pair_the_same_way() {
    let (record, table) = committed_browser_played_pair();
    assert!(check_pair(&record, &table).is_ok());
    assert!(crate::api::archive_would_accept(&record, &table), "the archive route's own read agrees");

    let mangled = table.replace("\"players\":2", "\"players\":3");
    assert!(check_pair(&record, &mangled).is_err(), "a table that does not agree with its record");
    assert!(!crate::api::archive_would_accept(&record, &mangled));
}

/// R-6.5b2.12: REJECTED and UNPLAYABLE are different answers. Refusing a
/// foreign-rev record outright would strand the only copy of a game.
#[test]
fn a_foreign_rev_record_is_unplayable_and_a_mangled_one_is_rejected() {
    let (record, table) = committed_browser_played_pair();
    let foreign = record.replace("\"semantics_rev\":7", "\"semantics_rev\":6");
    let env = envelope_of(&[("g1", &foreign, &table), ("g2", "not json at all", &table)]);
    let report = restore_read(&env).expect("the envelope itself is well formed");
    assert!(matches!(report[0].1, Verdict::Unplayable { .. }), "kept, listed, downloadable");
    assert!(matches!(report[1].1, Verdict::Rejected { .. }), "not a record at all");
}

/// The envelope's own shape is checked before any game is. A future version
/// is refused as a whole rather than half-read.
#[test]
fn an_envelope_of_another_version_is_refused_whole() {
    let env = envelope_of(&[]).replace("\"envelope\":1", "\"envelope\":2");
    let err = restore_read(&env).expect_err("a version this build does not know");
    assert!(err.contains("envelope"), "{err}");
}
```

- [ ] **Step 2: Write the failing page cells**

```ts
// The whole reason backup is NOT a wasm export (spec §2.5, §9): a module that
// will not load is exactly when a player most needs their games out.
it("builds an envelope with the module unavailable", async () => {
  breakTheWorker();
  const text = buildEnvelope([{ id: "local-1", record: RECORD_TEXT, table: TABLE_TEXT }]);
  expect(JSON.parse(text).games[0].record).toBe(RECORD_TEXT);   // the very string, unparsed
});

// R-6.5b2.11: a clash refuses and NAMES the row; identical bytes are a no-op.
it("refuses a clash by name and calls identical bytes already here", async () => {
  await putPair("local-1", RECORD_TEXT, TABLE_TEXT);
  const same = await restorePairs(buildEnvelope([{ id: "local-1", record: RECORD_TEXT, table: TABLE_TEXT }]));
  expect(same.games[0]).toMatchObject({ id: "local-1", outcome: "already-here" });

  const other = await restorePairs(buildEnvelope([{ id: "local-1", record: OTHER_RECORD, table: TABLE_TEXT }]));
  expect(other.games[0]).toMatchObject({ id: "local-1", outcome: "clash" });
  expect(await readPair("local-1")).toEqual({ record: RECORD_TEXT, table: TABLE_TEXT });  // untouched
});

// Run it twice and nothing is harmed — the property that makes a restore
// usable when you are not sure the first one worked.
it("is safe to run twice", async () => {
  const env = buildEnvelope([{ id: "local-9", record: RECORD_TEXT, table: TABLE_TEXT }]);
  expect((await restorePairs(env)).games[0]!.outcome).toBe("restored");
  expect((await restorePairs(env)).games[0]!.outcome).toBe("already-here");
});
```

- [ ] **Step 3: Run both and watch them fail**

```bash
cargo test -p yokohama-web the_archive_and_the_envelope
cd web && npx vitest run src/backup.test.ts
```

- [ ] **Step 4: Implement `backup.rs`** and refactor `archive_post` to call `check_pair`. **`archive_post`'s behaviour must not change**: run `cargo test -p yokohama-web archive` and confirm every existing archive cell is still green without being edited.

- [ ] **Step 5: Add `restore_read` to the module** with a native cell, and a `restore` message to the Worker.

- [ ] **Step 6: The page side and the screens** — `buildEnvelope` over strings, `restorePairs` doing the clash rule over IndexedDB, and `Backup.tsx` with a per-game control, "Back up everything", a file input and the report.

- [ ] **Step 7: Run everything**

```bash
cargo test -p yokohama-web && cargo test -p yokohama-wasm && cargo clippy --workspace --all-targets -- -D warnings
cd web && npm run typecheck && npm test && npm run test:wasm && npm run e2e
```

- [ ] **Step 8: Commit**

```bash
git add crates/yokohama-web/src/backup.rs crates/yokohama-web/src/{lib,api}.rs crates/yokohama-wasm/src/lib.rs web/src/backup.ts web/src/backup.test.ts web/src/Backup.tsx web/src/Backup.test.tsx web/src/local/{worker,db}.ts web/src/OnThisDevice.tsx web/src/App.tsx
git commit -m "feat: one backup envelope, validated by the function the archive route uses, refusing a clash by name"
```

---

### Task 9: The outbox, and Abandon's three states (R-6.5b2.13, R-6.5b2.14)

**Files:**
- Create: `web/src/outbox.ts`, `web/src/outbox.test.ts`
- Modify: `web/src/local/db.ts` (**version 2**, the `outbox` store — a version bump here is only ever a NEW STORE, never a migration of a document's shape), `web/src/Abandon.tsx:100-125`, `web/src/App.tsx:1464-1510` (the send handler), `web/src/OnThisDevice.tsx`
- Test: `web/src/Abandon.test.tsx`, `web/src/App.test.tsx`

**Interfaces:**
- Produces: `type PendingReason = "offline" | "signed-out"`; `enqueue(id, reason)`, `drain(): Promise<void>`, `pending(): Promise<{id, reason, lastTried}[]>`, `useOutbox()`; `archiveDisposition(kind: ArchiveKind | "unknown"): "send" | "delete" | "ask"`.
- Consumes: Task 2's `ensureArchiveFact`, Task 8's `localPair`, `api.archive`.

- [ ] **Step 1: Write the failing cells**

```ts
// spec §2.6 and §0.3: the measured gap. A finished game that could not be
// uploaded is not lost and not forgotten — it waits and retries itself.
it("queues a finished game that could not be sent, and drains when the connection returns", async () => {
  vi.spyOn(api, "archive").mockRejectedValueOnce(new Error("offline"));
  await finishLocalGame("local-7");
  expect(await pending()).toEqual([expect.objectContaining({ id: "local-7", reason: "offline" })]);

  const sent = vi.spyOn(api, "archive").mockResolvedValue({ path: "p", identical: false });
  window.dispatchEvent(new Event("online"));
  await waitFor(async () => expect(await pending()).toEqual([]));
  expect(sent).toHaveBeenCalledTimes(1);
});

// The two reasons are different problems and only one has a remedy now.
it("distinguishes offline from signed out, and names BOTH doors under claimed", async () => {
  vi.spyOn(api, "archive").mockRejectedValue(new ApiError(401, "no signed-in user"));
  patchServerFacts({ identity: "claimed" } as never);
  await finishLocalGame("local-8");
  render(<OnThisDevice games={[{ id: "local-8" }]} />);
  expect(screen.getByText(/sign in/i)).toBeInTheDocument();
  expect(screen.getByRole("link", { name: /sign in/i })).toHaveAttribute("href", "/enter");
  expect(screen.getByText(/then sign in here/i)).toBeInTheDocument();   // the SECOND step
});

// R-6.5b2.13 and spec §9: "unknown" is not "off". THREE states, and the
// middle one is the whole finding — at HEAD an offline Abandon deletes.
it("refuses Abandon when it cannot learn whether there is an archive", async () => {
  vi.spyOn(api, "health").mockRejectedValue(new Error("offline"));
  const remove = vi.spyOn(api, "remove");
  render(<Abandon table={localTable} local onDone={vi.fn()} onError={vi.fn()} />);
  await userEvent.click(screen.getByRole("button", { name: /abandon/i }));
  await userEvent.click(screen.getByRole("button", { name: /abandon/i }));
  expect(remove).not.toHaveBeenCalled();
  expect(screen.getByRole("button", { name: /back it up/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /without saving/i })).toBeInTheDocument();
});

it("still deletes where the deployment is KNOWN to keep no archive", async () => {
  vi.spyOn(api, "health").mockResolvedValue({ ...wire.health, archive: "off" });
  const remove = vi.spyOn(api, "remove").mockResolvedValue(undefined as never);
  await abandonTwice(localTable);
  expect(remove).toHaveBeenCalledTimes(1);      // correct, and not the bug
});
```

- [ ] **Step 2: Run them and watch them fail**

```bash
cd web && npx vitest run src/outbox.test.ts src/Abandon.test.tsx
```

- [ ] **Step 3: The store.** `db.ts` to version 2 adding `outbox` keyed by `id`. The `onupgradeneeded` must create it for a version-1 database as well as a fresh one; a cell opens a v1 database, closes it, reopens at v2 and asserts both stores exist and the games survived.

- [ ] **Step 4: `outbox.ts`** — enqueue with a reason, drain on `online`, at launch and on the button, each drain bounded and serialised so two triggers cannot post twice. **Deliberately not the Background Sync API**, which Safari does not implement.

- [ ] **Step 5: `archiveDisposition` and Abandon's three arms**, and the send handler queueing on failure.

- [ ] **Step 6: Run everything including the walk**

```bash
cd web && npm run typecheck && npm test && npm run e2e
```

- [ ] **Step 7: Commit**

```bash
git add web/src/outbox.ts web/src/outbox.test.ts web/src/local/db.ts web/src/Abandon.tsx web/src/Abandon.test.tsx web/src/App.tsx web/src/App.test.tsx web/src/OnThisDevice.tsx
git commit -m "feat(web client): an outbox for a game finished offline, and an Abandon that will not delete what it could not save"
```

---

### Task 10: The six browser cells (spec §6.3)

**Files:**
- Create: `web/e2e/offline.spec.ts`
- Modify: `web/playwright.config.ts` (a persistent-context project for cell 2)

**Interfaces:** consumes every earlier task. Produces the evidence the close cites.

- [ ] **Step 1: Write all six, then run them**

Each carries a file-level `test.skip(!haveBrowser, …)` (the fourteenth instrument holds this), and this spec is **the one that does NOT take the no-worker fixture**.

1. **Offline play.** Install, `context.setOffline(true)`, reload, play a whole local game to a winner. Assert every `/api/` request failed or was never made, and that the game reached `over`.
2. **Close and reopen.** **`launchPersistentContext(userDataDir)` and nothing else will do** — `storageState` carries cookies, localStorage and (at 1.62.1) optionally IndexedDB, but never the Cache Storage or the worker registration, which are the two things this cell is about. Offline, close, reopen, resume, finish, export.
3. **The update.** Serve a second release; assert the offer appears, that the controlled page does **not** change until the reload is clicked, and that the old cache is gone after.
4. **Restore on a fresh profile.** An empty context restores an envelope and plays the restored game. **A cross-rev pair restored the same way appears in the LISTING with its reason** — a different claim from the report calling it unplayable.
5. **The clash.** A restore over a present, different game refuses and names the row.
6. **The outbox.** Finish a game offline, go online, and the upload lands with nobody clicking anything.

```bash
cd web && npm run e2e
```

- [ ] **Step 2: Record the run's wall** against the 4 m 47 s base, in the ledger.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/offline.spec.ts web/playwright.config.ts
git commit -m "test(e2e): the six offline cells, from install to a restore on a fresh profile"
```

---

### Task 11: The phone, the battery, the close and the doc debts (R-6.5b2.17)

**Files:**
- Create: `docs/plans/<the day it is written>-web-frontend-m6.5b2-close.md` (this lane dates every
  artifact by the day it is written, never by the day it was planned)
- Modify: `CLAUDE.md`, `docs/handover.md`, `deploy/RUNBOOK.md`

- [ ] **Step 1: Run the mutation battery** (spec §6.5, eleven mutations) at the fix-wave head, each applied, shown to fail by NAME, and reverted. Write the report to the ledger.

- [ ] **Step 2: Deploy the release** and record it in the RUNBOOK as §3.15.

- [ ] **Step 3: The exit, on the owner's iPhone, by hand.** Install to the home screen, airplane mode, play, **close the browser entirely**, reopen, resume, finish, export. Then an update and a restore on a fresh profile. Then the Android emulator as a SECOND READING, recorded and not gated.

- [ ] **Step 4: Count everything in ONE sequence** at the close HEAD and record the figures. Do not copy a count from this plan.

- [ ] **Step 5: Write the close** on the M6.5b close's shape: what shipped by task, the counts, the corpus diff, the reviews, the rulings taken in execution with their costs, the LEAVE list and owner items, and **§7 what this cycle does NOT claim** (spec §7 is its starting point, corrected by what execution found).

- [ ] **Step 6: Pay the two doc debts §0.3 records** — `CLAUDE.md`'s art count is **131 WebP files plus the manifest**, not 132; and the close records the 109-byte build variance against the M6.5b close's 2,842,550 B figure so the two readings do not look contradictory.

- [ ] **Step 7: Commit, push, open the PR** against `web-room-m6.5b`, and leave the merge order to the owner.

---

## Self-review

**Spec coverage.** Every §1 ruling maps to a task: R-6.5b2.1 → 3; .2 → 1, 2, 4; .3 → 1; .4 → 1, 3; .5 → 4; .6 → carried, no task; .7 → 2; .8 → 6; .9 → 7; .10–.12 → 8; .13–.14 → 9; .15 → 5; .16 → the task order and 4; .17 → 11; .18 → 5. Every §6 test group has a home: §6.1 → 1, 8; §6.2 → 2, 3, 6, 7, 8, 9; §6.3 → 10; §6.4 → 11; §6.5 → 11.

**Three things this plan deliberately does NOT do**, so an executor does not go looking for them: it does not move the NAS to `cf-access` (the owner ruled to stay on `claimed`); it does not decide the public Worker's configuration (R-6.5b2.6 carries it to M6.5c); and it does not close the untrusted-record item in the M6.5b close's §6.3, which stays behind the door.

**Two places an executor will be tempted to widen scope, and must not.** Task 4 touches no source: if it wants to, the front door has failed its own premise that the app does not know about Access. Task 8 refactors `archive_post` to call the shared validator and must leave its behaviour untouched — every existing archive cell stays green **without being edited**, and editing one to make it pass is the finding, not the fix.

**The riskiest task is 6**, because it is the one whose failure mode is invisible: a service worker that caches something it should not, or that fails to update, looks fine until it does not. Its three pure functions exist so the logic is testable without a browser, and cell 3 of Task 10 is the only thing that proves the lifecycle end to end.
