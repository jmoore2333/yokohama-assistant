# The web lane — its operating record

**What this is.** The web lane's operating detail, moved out of `CLAUDE.md` at the lane's last cycle (M6.5c2
cycle E, "the finish"; R-6.5c2.15, R-6.5c2.32), so that sessions working on the engine and the computer player
stop paying for it. `CLAUDE.md` keeps a summary and the two facts that cross into the AI lane, and
`guidingpath.md` stays the authority.

It is current-state. A count is updated IN PLACE at a close, naming its reading, and how anything came to be is
told in the named close (§11). **Two cells hold two of its tables to the code**, and fail the build when either
drifts: `crates/yokohama-web/tests/lane_document.rs` holds §13's API map to the layer's route table and to the
routes each host answers itself, and `worker/scripts/lane-variables.test.mjs` holds §12's table to the variable
names the lane's code spells. No line here spells the owner's e-mail address or any secret value; a reading that
involved the address says "the owner's address".

1. [What the lane is, and where it stands](#1-what-the-lane-is-and-where-it-stands)
2. [The suites and the gates](#2-the-suites-and-the-gates)
3. [The testing strategy](#3-the-testing-strategy-when-each-tier-runs-what-it-proves-what-skipping-it-costs)
4. [The architecture and the standing rules](#4-the-architecture-and-the-standing-rules)
5. [What not to re-derive: the traps](#5-what-not-to-re-derive-the-traps)
6. [What a save file says](#6-what-a-save-file-says)
7. [The release-version rule](#7-the-release-version-rule)
8. [The public door's preconditions](#8-the-public-doors-preconditions)
9. [How the documents are built](#9-how-the-documents-are-built)
10. [Operating facts](#10-operating-facts)
11. [The records of record](#11-the-records-of-record)
12. [Every variable](#12-every-variable)
13. [The API map](#13-the-api-map)
14. [Getting started](#14-getting-started)
15. [What is by design](#15-what-is-by-design)

## 1. What the lane is, and where it stands

- **The web lane** (`crates/yokohama-web`, `crates/yokohama-wasm`, `crates/yokohama-worker`, `web/`, `worker/`) is not a phase: no gate, no roster place. M1–M5.5 merged to master 2026-09-11; the M6 "room" cycles M6.1 → M6.5c and M6.5c2's cycles A, B, C, D and E are SHIPPED on stacked branches (`web-room` → … → `web-room-m6.5c2e`, twelve open PRs from #31 against master to #42, and E's own PR, opened at its close; the merge order is the owner's and the merge is cycle E's last step, on the owner's word). **Two hosts are LIVE at `0.6.0`, both behind the hostname-wide Cloudflare Access application and both on the Access door:** the NAS as `yokohama-web:0.6.0-m6.5c2e` on `cf-access` at `yokohama-nas.moore.nyc` (`deploy/RUNBOOK.md` §3.19, its sixteenth deployment, 2026-09-30 — swapped as ONE script, 1.6 s from stop to listening), and the Cloudflare Worker `yokohama-web` at `yokohama.moore.nyc`, version `eb8c7839`, the twenty-eighth deployment (`worker/RUNBOOK.md` §19: the twenty-fifth, `e0ac38e7`, deployed E's release, the twenty-sixth and twenty-seventh were the door window's, and the twenty-eighth restored the release with the dev door on; §15–§17 the door's records: the `worker` door, `DEV_DOOR_CLIENT_ID` a SECRET, the `test/` prefix expiring after 30 days): hosted multiplayer on a stateless Worker, SQLite-backed Durable Objects and an R2 archive, every payload the layer's own. The release both hosts serve, `yokohama-release-50ee8137624586b2`, is built with the three build values — `YOKOHAMA_PROJECT_URL` (the door's link to the repository), `YOKOHAMA_SITE_NOTICE` and `YOKOHAMA_SITE_BLURB` (§14) — and installs its shell from `/shell.bin`, which frees a returning browser the edge's script had pinned to its last release (trap 33; the update reading PASSED on both hosts, read once per host in a headless persistent context on the workstation — the owner's own browser is the last reading). **A rollback of either host across cycle D takes down any chat that holds a deleted message** (`deploy/RUNBOOK.md` §5, `worker/RUNBOOK.md` §6 — the Worker's house chat holds several, since every live replay deletes one of its own posts), and a rollback of the Worker across E renders a `waited` marker's head as `undefined — …` (`worker/RUNBOOK.md` §19). **The public door was vetted in two states in E's window and REVERTED** (2026-09-30, §8): Access is hostname-wide again, the dev door on, and the zone's rate-limit rule stays at 20 requests per 10 seconds. **M6.5c2 "the cleanup" stands in cycle E "the finish"** (the plan `docs/plans/2026-09-29-web-frontend-m6.5c2e-plan.md`, entered from `docs/plans/2026-09-29-web-frontend-m6.5c2e-handover.md`): **its code — the shell as a static asset, D's leftovers, the three build values, the owner's-eye polish, lobby paging, "Abandon and save", the waiting-table expiry, requests per move bounded, the door window's instruments — and its live readings — both deploys, the update reading, the live tiers, the ten pictures, the phones, the rate limit, the door window — are DONE; its documents, its close, the merge of the stack from #31 through E's PR and the `v0.6.0` tag on the owner's word are the last steps.** After it the lane goes quiet and the project refocuses on the computer player. **The lane's fence, every cycle:** the five fenced crates (`yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge`, `gen-data`) byte-identical, the CLI touched only by behaviour-preserving seam changes, `SEMANTICS_REV` 7, no bot behaviour moved. Each cycle's record of record is in §11.

| concern | the NAS / Docker host | the Cloudflare Worker | shared |
|---|---|---|---|
| the code | `crates/yokohama-web` with its `server` feature (axum); `deploy/Dockerfile` and `deploy/Dockerfile.runtime` | `worker/` (the TypeScript shell) over `crates/yokohama-worker` (the module) | the layer — `crates/yokohama-web` without `server` — and the `web/` client, one `dist` for both |
| identity | `YOKOHAMA_WEB_IDENTITY`: `cf-access` behind Access (the NAS; the repository's default), `open` (a home network, LAN or VPN), or `worker` behind an identity-aware proxy | `YOKOHAMA_WEB_IDENTITY`: `worker` (Access's assertion verified by the shell; the repository's) or `open`; the dev door under `worker` | the key grammar (`User::parse`) and the mode rule (`identity::resolve`) |
| transport | server-sent events | hibernatable WebSockets carrying the SSE frames' data | the frames |
| storage | `FsStore` on a Docker volume | SQLite in each Durable Object | the `Store` trait |
| the archive | `DirArchive` under `YOKOHAMA_WEB_ARCHIVE_DIR` | R2 through the receipt, behind `YOKOHAMA_WEB_ARCHIVE`; `test/` expires after 30 days | `archive::paths`' layout, which the CLI loads |
| expiry and the operator | the Worker's two TTL variables, read by an hourly sweep through the creator's delete when set, refusing to start without an archive (cycle F, R-F9); no marker (R-F10), no operator surface; Portainer and the volume | a 90-day idle TTL, a 7-day TTL for a table with a seat still empty — status `open`; a dealing table waits the 90 (§15) — (cycle E), and the marker; `/api/admin` behind the secret `YOKOHAMA_WEB_ADMINS`, refused under `open` | — |
| configuration | the container's environment; the three build values at the client's build | `worker/wrangler.jsonc`'s `vars`, and the secrets `YOKOHAMA_WEB_ADMINS` and `DEV_DOOR_CLIENT_ID`; the door window's `wrangler.open.jsonc` and `wrangler.gate.jsonc`; the pool's `wrangler.test.jsonc` and `wrangler.open.test.jsonc` | the chat variables' names and defaults; the three build values, one client build for both |
| hostname and door | `yokohama-nas.moore.nyc` through cloudflared | `yokohama.moore.nyc`, a custom domain | the ONE `yokohama` Access application |
| documents | `deploy/README.md`, `deploy/RUNBOOK.md` | `worker/README.md`, `worker/RUNBOOK.md` | this file |
| a save's `run.run_id` | `web/server` | `web/worker` | a browser's own game: `web/browser` |
| tests | `cargo test -p yokohama-web`; the local e2e against the axum binary; `docker.yml` and `deploy/smoke-image.sh` for the image | `cargo test -p yokohama-worker`; `worker/`'s node cells and its two pool projects; the live tiers | `wire_fixtures` and the corpus; `test:wasm`'s node replay; the two cells over this file |

## 2. The suites and the gates

Every count below names its reading. **Re-count at your own HEAD** rather than trusting this file. These are the
web lane's per-edit suites; `CLAUDE.md`'s Commands keep the engine's, the CLI's, the oracle's, the harnesses', the
bridge's and the lint gate. **Run these whenever a change reaches what the layer compiles** — `crates/yokohama-core`,
`crates/yokohama-cli`, `SEMANTICS_REV` or the workspace version — because the committed wire corpus pins the
engine's output byte for byte, so an engine change shows here first.

- Web (server): `cargo test -p yokohama-web` — **401** (**374** lib + **5** `browser_played_pin` + **4** `lane_document` + **2** whole-game gate + **16** `wire_fixtures`; the M6.5c2 cycle E close's sequence at `3107eb8b` — 397 at the code head `c39da5a1`, before `lane_document`) — `lane_document` (cycle E, this file's API map; the close's sequence re-counts the whole); no `#[ignore]`d tier.
  - **The whole-game gate has TWO arms that prove different things**: a full 2P game against the shipped bot driven entirely over JSON — the web analogue of `piped_dogfood`, and the only thing that proves every phase, queued choice and sub-picker is REACHABLE through the API rather than merely modelled — and a full 2P game between TWO CLAIMED IDENTITIES, each asserted to see only its own hand. **It IS a witness to the acting-seat fence since cycle D's Task 10** (the other identity posts every line first and is refused by seat, beside the three lib cells). **Its vs-bot arm no longer livelocks**: its driving policy places through a placement probe and answers a stuck picker, and its cap (`VS_BOT_CAP`, 5,000) was measured over 500 seeds (2026-09-29, release profile: max 1,503 inputs, none over 2,500), so a red gate here is still read before it is believed — the cap rests on that sample. The dispatch-only `web-server` job of `.github/workflows/web.yml` (R-6.5c2.31) runs `cargo test -p yokohama-web --lib --test wire_fixtures --test browser_played_pin --test lane_document` and the gate binary too, LAST and BOUNDED — `timeout-minutes: 20`, `--nocapture` so a run the bound kills still logs its seed, `YOKOHAMA_WEB_GATE_BOT_SEED` to replay one; the gate stays per-edit locally, deliberately NOT `#[ignore]`d. A test-binary guard in the same job reads the layer's `tests/` binaries off `cargo metadata` and refuses one the job does not name.
  - **`browser_played_pin`** loads the committed browser-played pair `tests/fixtures/browser-played-2p.{json,table.json}` into a `Games` over `MemStore` and reconstructs EVERY bot move through the layer's `why` via `dispatch`; the pair carries a seed on purpose, so it sits OUTSIDE `web/src/wire/`, the only directory the corpus's seed pin scans.
  - **`wire_fixtures` is the wire contract**: it drives the real router through a deterministic bot-free 2P game (a bot draws its seed from OS entropy, so a golden built over one would churn) and emits every payload shape the client reads as committed JSON under `web/src/wire/` (**108** JSON files at the M6.5c2 cycle E code head, `c39da5a1` — cycle E added `house_frame.json`, the house frame `{seq, chat, rev}` built in process, since the walk never reads a stream), refusing when the server's output is not what is committed, when a committed file is one nothing emits any more, when the corpus misses a `PromptState` variant (`reveal`, live mode only, is the one named exemption), or when it misses a `why` `Refusal` variant — all twelve built by `explain::refusal_samples` and worded by the CLI's own `Display`. **That fence at its real size:** `explain::kind_of` is a `match` with no `_` arm, so a thirteenth variant fails to COMPILE there; but a variant added to `Refusal` AND `kind_of` and not to `refusal_samples`' `vec![…]` reaches the wire untagged with every test green — closing it is a `strum` dependency decision, not a test change. Regenerate with `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, and **read the diff**: it is a contract. Its leakage pins search the corpus for VALUES, never words — `no_committed_payload_carries_the_setup_seed` (decimal and hex), `no_payload_carries_an_address_that_is_not_the_readers_own` (the seat-email roster off the wire, cycle D; its partner `the_stored_table_document_carries_every_seats_address`), `no_payload_of_this_run_carries_the_password_or_its_hash` (the password, a wrong attempt, the salt, the hash) and `a_silent_watcher_is_named_nowhere_and_a_posting_one_only_as_an_author` — each with an anti-vacuity partner for every value the codebase stores. `site_space_words.json` is a CONTRACT rather than a route's payload: the terminal's own `card_render::space_reward_words` for all 24 building-site cards, frozen so the client's `Cards.spaceWords` cannot drift from it. It also writes the two request scripts a host that SERVES the corpus replays — `web/src/wire-script/walk-host.json` (the three `…/events` rows, positioned by the walk row each follows, and a `GET /api/art` it synthesizes) and `archive-walk.json` (the eight `archive_*` requests) — outside `web/src/wire/` for `walk.json`'s reason; the Worker's pool replays all three through the objects (M6.5c).
  - **Every request body and query struct carries `#[serde(deny_unknown_fields)]`** and is extracted through the crate's own `JsonBody`/`QueryArgs`, so an unknown key is a **400 in this API's `{"error": …}` shape** rather than a silent default (before the fence a client that renamed `start_seat` got a 201 and a different game). `SeatSpec` accepts exactly one of `{"user": …}`, `{"bot": true}` or `{"open": true}`; a `wait` above `EventsQuery`'s 25-second cap is a 400, never a silent clamp, because a clamped answer reads as "nothing changed".
  - **Also per-edit:** `cargo test -p yokohama-wasm` — **9** (the browser host's own cells, among them its platform-call scan, whose two allowed calls are `js_sys`'s clock and the console import); `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown` (the dependency fence — the layer builds with no server half); `cargo test -p yokohama-web --no-default-features --features server --lib identity::` — **8** cells in the one spelling a public build uses (`open-door` dropped), proving that shape compiles and refuses `YOKOHAMA_WEB_IDENTITY=open` by name; and `cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm`, which proves the crate compiles for the browser but is NOT the shipped module. `npm run build:wasm` builds that: `--profile release-wasm` (`opt-level = "z"`, fat LTO, one codegen unit — its own profile, never an edit to `[profile.release]`, which would re-time every gate wall and arena figure), then `wasm-opt -Oz --strip-debug --strip-producers` over both emitted modules. `wasm-opt` is a PREREQUISITE — its absence REFUSES the build naming `brew install binaryen`, and `YOKOHAMA_WASM_OPT_OPTIONAL=1` builds an unoptimized module that says so. **BOTH modules' size figures live in ONE place, the `release-wasm` comment in the root `Cargo.toml`** — the browser module's paragraph and, since M6.5c2 (`83b52710`), the Worker module's — cite it, never copy it, and read any figure as a READING, not a constant (two builds of one source differ by about a hundred bytes; the Docker stage's module read ~4.9 KB larger than the workstation's at the same commit under the same `wasm-opt` 133).
- Web (browser half) — **FOUR commands, all per-edit; `cd web && npm ci` once first, and `npm run build:wasm` once before the typecheck or `test:wasm` (it needs the pinned toolchain's `wasm32-unknown-unknown` target and `wasm-bindgen-cli 0.2.128`, `deploy/RUNBOOK.md` §1).** The counts are the M6.5c2 cycle E code head's readings (at `c39da5a1`).
  - `npm run typecheck` — `tsc -b --noEmit && tsc -p tsconfig.node.json && tsc -p tsconfig.sw.json && node scripts/typecheck-worker.mjs`: FOUR projects — the app; the node half (the build configs, the Playwright config, `e2e/`, the node corpus replay `src/local/*.node.test.ts`); the dedicated Web Worker (`tsconfig.worker.json`); and the service worker (`tsconfig.sw.json`) — the two worker projects on `lib: ["ES2022", "WebWorker"]` with no DOM, so the DOM and WebWorker libs never meet in one project. The worker project runs through a GUARD that refuses out loud naming `npm run build:wasm` when the generated glue is absent, unless `YOKOHAMA_WASM_OPTIONAL=1` — then it prints that it checked NOTHING (CI's `web-client` job sets it and stays toolchain-free). **The app half type-checks the committed wire corpus against the hand-written `web/src/types.ts`** via `web/src/wire/contract.ts`, so a rename on the server fails `wire_fixtures` and a rename in the client fails here; its stated limit, in-file: a field the SERVER adds that the client does not declare is caught only by the contract's root key sets.
  - `npm test` (`vitest run`) — **1342** in **66** files: component tests over jsdom. **Every payload they render comes from `web/src/wire/`; nothing is hand-authored**, because a builder is a third statement of the wire shape that keeps compiling after the server drops a field — and the fake Worker (`src/test/fakeWorker.ts`) answers every local call from the same corpus, the one permitted edit being a fixture's id rewritten to `local-…`. jsdom has no layout engine and never loads `styles.css`, so no cell here can see CSS. The load-bearing cells are MUTATION-PROVEN (the method: `docs/plans/2026-08-31-web-frontend-m2.5.md` §3).
  - `npm run e2e` (`npm run build:wasm:debug && npm run build:client && playwright test`) — **73** in **13** spec files, 14.0 min inside the M6.5c2 cycle E close's sequence at `3107eb8b` (6.9 min in the code head's run; 4.4–7.2 min across the lane's earlier closes, up to 10.1 min between them on the MacBook Air): real Chromium against the real release binary — the only instrument that sees images load, real focus, real layout width, CSS, the service worker, and whether the SCREEN plays to a winner. The DEBUG module it builds (it carries the `debug_trap` export one cell needs) is left in `web/src/local/wasm/` and `web/dist/` afterwards, so a deploy rebuilds with `npm run build` and checks that `grep -c debug_trap web/src/local/wasm/yokohama_wasm.js` prints 0 (`deploy/RUNBOOK.md` §1). **Needs a browser**: `npx playwright install chromium` once; its absence is a LOUD refusal naming that command, and `YOKOHAMA_WEB_E2E_OPTIONAL=1` turns it into a skip that says out loud it tested nothing — a hatch held by `web/e2e/every_spec_can_skip.spec.ts` (every spec needs a file-level `test.skip(!haveBrowser, …)`), while `web/e2e/every_spec_declares_its_worker.spec.ts` names any spec that does not declare whether the service worker is on. Four opt-in specs stay out of the run unless switched on, beside the exit game below: `shots.spec.ts` (`YOKOHAMA_WEB_SHOTS=1`; it re-cuts the help panel's pictures in `web/public/help/`, so leaving it in would dirty the tree), `gates.spec.ts` (`YOKOHAMA_WEB_GATES=1`, M5's visual-gate shooter, into a gitignored directory), and, since cycle E, `readme_shots.spec.ts` (`YOKOHAMA_WEB_README_SHOTS=1`: the ten README pictures into `worker/docs/img/`, on whichever target the run names, each looked at before it is committed) and `state1_game.spec.ts` (`YOKOHAMA_WEB_STATE1_GAME=1`: a plain two-person game through the open door, the door window's instrument). Release-tier by cost, **per-edit by rule when the client changes — and owed by any task that changes what a browser DOES, not only what it sees.** `two_browsers.spec.ts` is the expensive cell (its wall is the seeded game's length); its `CAP = 4000` loop iterations can be hit on a long game, which then reports through `capReached`, never `deadEnd` (`web/e2e/harness.ts`). `YOKOHAMA_WEB_TALLY=<file>` makes every page append one JSON line per request, and `web/scripts/tally-report.mjs` reads it as requests per applied move and per-10-second peaks by route — cycle E's instrument for requests per move (16.74 → 9.59 `/api` per applied move over the local walk) and for the rate-limit value. Two product attributes exist for this harness and must not be removed: `data-busy` on the app root (the settle signal) and `data-event` on each `TurnAccount` row. **Two specs also run LIVE against the Worker** — `YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers joins_by_link` — the orchestrator's alone, because the configuration reads the `yokohama-worker` credential from the gitignored `.dev.vars`; release-tier by cost, and under the quota guard (the Worker bullet below). Each live context takes Access's own `CF_Authorization` cookie, fetched ONCE by Node with the credential so Playwright never holds the secret, plus the claimed identity as a context-wide header (Playwright's request routing does not reach a page-opened WebSocket upgrade; context headers do); tracing is OFF under the live target; **every live `request.*` goes through `liveRequest`**, and a live response never meets `toBeOK()`, because a thrown API call's call log prints the cookie to the reporter and to `web/test-results/`; and the harness forgives EXACTLY two known shapes — the analytics beacon's two-line CORS error (the claimed header rides its cross-origin request) and the deal's designed 409 — counted, never silenced. **The structural cell `web/e2e/live.node.test.ts` enforces it**, reading the live specs and every e2e helper through the TypeScript compiler: a live `request.*` outside `liveRequest`, any other access to a context's `request` (an alias, an argument, a destructure), `newContext` through `request`, and any `toBeOK()` are refused by `file:line`. `testIgnore` keeps `*.node.test.ts` out of Playwright's collection. **A third live spec is opt-in: the headed exit game** — `YOKOHAMA_WEB_EXIT=1 YOKOHAMA_WEB_E2E_TARGET=live npx playwright test exit_game --headed` (optional `WAKE_PAUSE_MS`, default 45000; `EXIT_SCREEN=<w>x<h>`, the screen's logical size, default 1280x832) — three contexts, a short paced game with pauses past the object's idle eviction, table and lobby chat and a watcher. Its three windows are placed through the browser protocol, side by side where three of Chromium's narrowest (500 px) fit, else in a 2×2 grid under the menu bar's inset, with every window's bounds read back and any overlap refused: macOS marks a covered headed window hidden whatever Chromium's switches read, and every page's visibility is the spec's own precondition. It runs only inside the Worker's wake window (below). Since cycle E, `readme_shots` and `state1_game` also run live, and `live.node.test.ts` holds all five live specs to the same rules; its gating cells prove a plain run collects neither.
  - `npm run test:wasm` (`vitest run -c vitest.node.config.ts`) — **159** cells in **12** files (the cycle E close's sequence; 158 at the code head, the state-1 game's gating cell the one added): the tests that need a REAL BUILD or a bundler, and so cannot live in the toolchain-free `web-client` job — and, beside them, the live tier's pure cells in `e2e/harness.node.test.ts` and `e2e/live.node.test.ts` (among the 158), which need neither. Chief among them the corpus REPLAY: the walk re-run through the COMPILED module in Node (`web/wasm-node/`, the clock frozen at the corpus's `1_789_000_000`) from the walk's own request script `web/src/wire-script/walk.json` — OUTSIDE `web/src/wire/` because it carries request bodies (the setup seed, the passwords) that every leakage pin scans the corpus for — with every row's status asserted, every named row deep-equal to its committed file, the host fields exempted only where they really differ between hosts, and the files the walk does not name asserted as an exact set. Also `sw.node.test.ts` (it bundles `src/sw.ts` through the generator's own esbuild path and RUNS it against fakes with a real WebCrypto digest), `precache.node.test.ts` (a real `dist`: the module, the Worker chunk, the stylesheet, the shell as `/shell.bin` and no HTML entry, exactly 131 art files) and `backup.node.test.ts`. Refuses when the module is absent, naming `npm run build:wasm` (`YOKOHAMA_WASM_OPTIONAL=1` skips and says so).
  - Also: `npm run build` (`build:wasm && build:client`; `build:client` = `tsc -b && vite build && node scripts/precache.mjs`) writes `web/dist`, which the server serves. Its third stage is the precache GENERATOR: it walks the finished `dist` plus the art manifest, writes `dist/shell.bin` — `index.html`'s bytes under a name the edge does not rewrite, served `no-store` by both hosts because `.bin` is on the edge's default cached-extension list (cycle E, R-6.5c2.41; trap 33) — lists it in place of any HTML, digests every body, writes `dist/sw.js` under a cache name that is a digest of the whole inventory, and REFUSES a release with no WebAssembly module, no shell, no art, a bypassed path, a duplicate, or over either ceiling (a ceiling must clear the Docker stage's larger module). **Vite's manifest is deliberately NOT consulted** — it names neither the module nor the Worker chunk. `npm run dev` serves on 5173 and proxies `/api` and `/art` to a server on 8080.
  - **CI** (R-6.5c2.31): `.github/workflows/test.yml` runs the core suite on a pull request that changes a Rust input and by hand; `.github/workflows/web.yml` runs the three web jobs on dispatch only, and `docker.yml` on dispatch only. `web.yml` runs the first two of these in job `web-client` (the typecheck with `YOKOHAMA_WASM_OPTIONAL=1`, then `npm test`); job `web-server` installs `wasm-bindgen-cli 0.2.128` and the pinned binaryen 133, builds the module, and runs `npm run typecheck`, the test-binary guard, `cargo test -p yokohama-web --lib --test wire_fixtures --test browser_played_pin --test lane_document`, `cargo test -p yokohama-wasm`, the `wasm32` `cargo check`, the refusing build's eight `identity::` cells (asserting all eight RAN), `npm run test:wasm` and, last and bounded, the whole-game gate; **job `worker`** (its own since M6.5c2; it needs no binaryen, since the pool runs the DEBUG module) runs — in `worker/` — `npm ci`, `npm run build:module:debug`, `npm run typecheck`, `npm run test:scripts` (this file's variables cell among its files) and `npm test` (the pool, both projects), then `cargo test -p yokohama-worker` and the Worker crate's `release-wasm` build; `web-server`'s binaryen is pinned to **133**, the version the workstation built every deployment with (M6.5c close), and since M6.5c2 `deploy/Dockerfile` installs the same 133 tarball, so the two pins agree. **`.github/workflows/docker.yml`** (M6.5c2) builds the self-host image and smokes it under each door with `deploy/smoke-image.sh`, triggered by dispatch only (R-6.5c2.31); a cold run is one whose log shows `[server 10/10]` executing, and R-C9's fifteen-minute rule retired with the trigger (R-6.5c2.31). **The browser walk deliberately stays local, and so do both live tiers and the exit spec, which need the credential; CI never deploys, and the deploy guard's build-dependent and freshness checks run only where the deploy is made**, so a LAYOUT claim from this lane is still checked by hand on one machine — written into the job rather than implied.
- Web (the Worker — `crates/yokohama-worker` + `worker/`, M6.5c): per-edit whenever either moves, or the layer does; `cd worker && npm ci` once first. The counts are the M6.5c2 cycle E code head's readings (at `c39da5a1`).
  - `cargo test -p yokohama-worker` — **76**: the module's cells, native — every export's logic but two (`set_clock`, `debug_trap`) sits in a pure-Rust `_inner` function the cells call. `cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker` proves the deployed shape compiles, and `cargo doc -p yokohama-worker --no-deps` reads 0 warnings. **The crate's `build.rs` stamps the picker's code identity** — a digest of `crates/yokohama-core/src`, `crates/yokohama-cli/src` and `crates/yokohama-web/src` — so a deploy that changes any of the three drops every open picker rather than re-feed a line whose meaning may have changed.
  - **`npm run build:module:debug` FIRST** whenever `crates/yokohama-worker` or the layer changed: `npm test` rebuilds nothing, and the pool would test a stale module GREEN.
  - `npm run typecheck` — two projects (the shell with the pool's cells; the node configurations); its `pretypecheck` hook writes the gitignored `test/fixtures.gen.ts`, which a clean checkout lacks.
  - `npm run test:scripts` — **431** in **23** files (the cycle E close's sequence: 427 in 22 at the code head + 4 in `lane-variables.test.mjs`, cycle E, this file's variables table): in node, the build scripts' cells (the factory transform, the asset assembly, the deploy guard over the committed configurations — production's, the door window's two and the pool's two — the protected path list, the tally, the reading and replay scripts) and the shell's pure modules. **It needs `cargo` on `PATH`**: one cell walks the module's path dependencies with `cargo metadata`.
  - `npm test` — the POOL, **224** in **11** files in TWO projects (`worker`, over `wrangler.test.jsonc`, and `open`, over `wrangler.open.test.jsonc` — one per door, since M6.5c2): the shell's cells inside workerd (Cloudflare's Workers Vitest integration) with real SQLite, alarms, hibernatable sockets and a local R2, over the DEBUG module — and **the whole corpus replayed through the objects: 2,213 rows run, 90 named rows deep-equal, and the eighteen files no script names (`THIRD_COLUMN`) asserted as an exact set**. The pool's rules are the plan's "Pool facts" (patch an RPC target's PROTOTYPE, restore before any alarm, fresh stubs after an abort, storage per FILE).
  - **The release path is the orchestrator's alone** (`worker/RUNBOOK.md` §1–§2): `cd web && npm run build` (a local e2e leaves the DEBUG module in `web/dist`), then `cd worker && npm run build` (the release module through `wasm-opt`, the assets assembled with every precache entry's SHA-256 verified against the release's own `sw.js` and `worker/static/_headers` serving `/shell.bin` `no-store`, the build's source digest stamped into `gen/`, and a release smoke answering `/api/health` 200), then `npm run deploy` — the deploy guard's checks (M6.5c's nine, plus the door's own rules since M6.5c2: a `vars` block that names no door, `DEV_DOOR_CLIENT_ID` or the operator list in `vars`, a `workers.dev` URL under `worker`, the dev door under `open` are each refused), then `wrangler deploy`. **The door window deploys a FILE, never an override** (cycle E; trap 36): `npm run deploy:open` and `npm run deploy:gate` run the guard with `--config wrangler.open.jsonc` or `--config wrangler.gate.jsonc`, which judges the file named, prints its door line, and refuses it unless it equals production but for its door (a gate file must set the dev door `off`); the plain guard holds both files to production's blocks on every deploy. **The guard refuses a deploy made without a rebuild**: its checks 8 and 9 compare `gen/`'s stamp with the sources as they are, and the assembled `sw.js` and `worker/static` with the current `web/dist` and `worker/static`, byte for byte.
  - **The exit's wake window is two deploys**, run in `worker/`: `node scripts/deploy-guard.mjs && npx wrangler deploy --var YOKOHAMA_WEB_COUNT_WAKES:on` opens it, and the plain `npm run deploy` closes it; while it is open every counted wake logs `wake {id, wakes}`. Any deploy restarts every object and closes every socket, so the window opens between games.
  - **The live tiers are release-tier by cost**, the orchestrator's alone and over WARP (a `text/html` 403 of about 5 KB is the zone's Bot Fight Mode; Access's 302 "Please authenticate via the warp client" means WARP's device login has EXPIRED, which only the owner re-authorizes): the live corpus replay (`cd worker && node scripts/live-replay.mjs`, run TWICE, sequentially — it forces its three ids before and after, HOLDS only if every named row is byte-identical, and on a first send's 409 stops and prints its reset recipe) and the live Playwright tier (above). **The quota guard comes first**: read the UTC day's Worker requests and Durable Object rows written from the account's GraphQL analytics (the `date`-filter form, one dataset per call, the figures under `result.viewer` through the Cloudflare MCP; `worker/RUNBOOK.md` §7), and HOLD while either is above the owner's line (80,000, the owner's line since M6.5c; Free's 100,000 is shared with the account's other Workers). The rows' periodic rollup lags by hours, so a window's own requests are read off the Workers Observability query API (§10). A live two-browser game of 2,034 moves read 36,003 invocations in its window in cycle E — 17,818 the Worker's own requests, the rest the objects' — and **7.9 of the Worker's own requests a move** (about 13 in D); its rows could not be separated from the day's (the rollup's lag), so D's ~2 written and ~52 read a move stand; cycle E's whole UTC day of live readings came to 205,709 rows read and 20,839 written.

- Regenerate the web client's card art after a sprite-sheet or KB-YAML change: `python3 docs/tools/mkcardart.py` (needs Pillow + PyYAML; crops `tts-assets/organized/` into `crates/yokohama-web/assets/art/` with a manifest, and **never writes into `tts-assets/`**). **The output is COMMITTED** — **131 WebP files** (~7.7 MB, `deploy/RUNBOOK.md` §3.4) plus `manifest.json` beside them — so a container needs neither Python nor the 20 MB of sheets; that is the arena corpus's `.gitignore` reasoning read the other way round, since only this one is needed at RUNTIME. The crops are cut at SOURCE width (`CARD_W` 530, `TILE_W` 709 — the sheet cell's own size, because anything narrower is an upsample once the fit rule lets a tile reach 22rem at 2×), and the site and achievement crops are rotated a quarter turn counter-clockwise before emit (`SITE_ROTATE_CCW`/`ACH_ROTATE_CCW`) — both families are landscape on the table but stored sideways on their sheets — so no client draws a site card on its side. `crates/yokohama-web/src/art.rs` is what enforces it, not the script: every identifier the engine can produce resolves, nothing in the manifest names an identifier the engine lacks, every entry is a non-empty file, and the emitted aspect matches the sheet cell. **Those tests prove the join is TOTAL, not TRUE** — a wrong-but-complete mapping is exactly what they cannot see, and two such errors (the crop origin, and yellow being COMMERCIAL rather than production on the category-C achievements) were caught only by looking at the pictures. Regenerating and not looking proves nothing.

- **The two cells over this file** (cycle E, R-6.5c2.45), both per-edit:
  - `cargo test -p yokohama-web --test lane_document` — **4** cells: §13's rows marked `layer` are EXACTLY
    `api::routes(true)`, and its rows marked `axum`, `worker` or `browser` are EXACTLY the cell's own
    `HOST_ROUTES` list (the cycle E dossier's item 27, one line each); a route cell that does not parse is refused,
    never skipped. A route the layer gains fails here until §13 names it. What it cannot see: a host route added
    to `crates/yokohama-web/src/lib.rs` or to `worker/src/route.ts` and `admin.ts` with no line here and in the
    map — `worker/scripts/access-paths.test.mjs` catches a new `/api/<segment>` prefix, not a new route under an
    old one — and a host that stops answering `GET /api/art` itself: the map carries it as a `layer` row, since it
    is in `api::routes`, and both hosts intercept it before dispatch, which the cell does not read.
  - `worker/scripts/lane-variables.test.mjs`, under `npm run test:scripts` — **4** cells: every `YOKOHAMA_*`,
    `VITE_*` and `DEV_DOOR_CLIENT_ID` name the lane's tracked code spells (the three crates, `web/`, `worker/`,
    `deploy/`, `web.yml` and `docker.yml`; no Markdown, no JSON) is a row of §12, and every row's name is spelled
    somewhere in that code; one named exemption, `VITE_MULTIPLAER`, the deliberate misspelling in
    `web/src/vite-env.d.ts`'s comment. What it cannot see: a variable read under a name of another shape, a row
    whose default, meaning or host has drifted, and a variable the code stops reading while a comment still names
    it, since a comment's mention counts.

## 3. The testing strategy: when each tier runs, what it proves, what skipping it costs

**The cadence** (R-6.5c2.13, the owner). The per-edit tiers run at every task. The FULL sequence runs at a
cycle's close only, as ONE recorded, machine-tagged run, in this order:
1. every crate's and package's per-edit suites;
2. the fifteen release gates, when the workspace version or anything they compile moved;
3. `npm run e2e`;
4. the deploys, each followed by its deploy readings and, for a release that changes the service worker or
   what it installs, the update reading;
5. the live replay's reset recipe, when `wire`'s records changed;
6. the live replay, twice;
7. the quota guard, then the live Playwright tier and the headed game;
8. the mutation battery;
9. the whole-branch review, run before the deploys so that a fix wave never forces a second live sequence.

A live step checks WARP's device login first, then the quota guard, and runs one Playwright run at a time.
Cycle E's Task 9.D recorded the per-edit set and the e2e as one sequence at the code head `c39da5a1` The close's FULL sequence then ran once at the final reviewed head `3107eb8b` (2,881 s; one red row, the client's vitest run, re-run alone green — §2 and the close's §2).
(2026-09-30, the MacBook Air, Apple M5): every row green, the e2e 73 in 6.9 min. The close's full sequence runs at
the final head (the plan's Task 16).

| tier | command(s) | what it proves | when it runs | what skipping it costs |
|---|---|---|---|---|
| Rust, per-edit | `cargo test -p yokohama-web` (the whole-game gate and the lane document's cell among its binaries), `-p yokohama-wasm`, `-p yokohama-worker`; clippy; the `wasm32` check; the public build's `identity::` cells; both wasm builds | the layer and both WebAssembly hosts' logic, natively; the wire contract (`wire_fixtures`); §13's map | every task; CI's `web-server` and `worker` jobs, by dispatch | a layer or module regression reaches the pool or a browser first, where it is slower to read |
| client, per-edit | `cd web && npm run typecheck && npm test` | the client against the committed corpus, over jsdom (no CSS, no layout) | every task; CI's `web-client` | a client regression against the wire |
| builds, per-edit | `cd web && npm run build:wasm && npm run test:wasm` | the corpus through the COMPILED browser module in Node; the service worker; the precache; the live tier's pure cells | every task whose Rust the browser module compiles; CI's `web-server` | a module or precache regression the jsdom cells cannot see |
| the Worker, per-edit | `cd worker && npm run build:module:debug` FIRST, then `npm run typecheck`, `npm run test:scripts` (§12's cell among them), `npm test` (both pool projects, one per door) | the shell inside workerd — real SQLite, alarms, sockets, a local R2 — both doors, and the whole corpus through the objects | every task touching the Worker or the layer; CI's `worker` job | the batch, alarm, slot, receipt and door rules, untested anywhere else; **without `build:module:debug` the pool tests a stale module GREEN** |
| the self-host image | `.github/workflows/docker.yml`, by dispatch (R-6.5c2.31); `bash deploy/smoke-image.sh <image>` by hand | the image builds from a checkout, refuses to start with no door, reads the right header under each door, and carries the build values | by dispatch when the image's inputs change; by hand before a self-host deploy | the image breaks unseen: the NAS is deployed another way (`deploy/RUNBOOK.md` §1–§3) |
| the local e2e | `cd web && npm run e2e` | real Chromium against the axum binary: images, focus, layout at desktop width and 390 px, CSS, the service worker, a game to a winner | any task that changes what a browser DOES or SHOWS; a cycle's close | a layout, CSS or service-worker regression; not in CI |
| the release roster | the fifteen release gates (`CLAUDE.md`) | the engine, the bot, the oracle, the differential harness | before merges and phase closes; a close where the workspace version moved | nothing while the fence holds and the version does not move |
| the deploy readings | `worker/RUNBOOK.md` §5; `deploy/RUNBOOK.md` §3.2 | the deployed host's identity (`/api/me` naming you), assets, service worker, `/shell.bin` served `no-store`, protection | every deploy | a broken deploy found by a player |
| the update reading | a persistent browser context per host, prepared at the PREVIOUS release with its `cf_clearance` cookie cleared, then reloaded once after the deploy and its "Reload to use it" clicked (`worker/RUNBOOK.md` §19's Reading 3, `deploy/RUNBOOK.md` §3.19) | a returning browser installs the new release from `/shell.bin`, is offered it, activates it on the click and keeps the previous cache — the path that failed on 2026-09-29 (trap 33) | every release that changes the service worker or its inventory; cycle E read it PASSED on both hosts, once per host in a headless persistent context on the workstation, not on the owner's own browser | a release no returning browser can reach, found by a player |
| the live replay | `cd worker && node scripts/live-replay.mjs`, twice, sequentially, over WARP | the whole corpus byte-identical through the LIVE Worker, and repeatable | after a deploy that changes what the Worker serves; a cycle's close | a host-only payload difference seen by nothing but the pool's model of the platform |
| the live Playwright tier | `cd web && YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers joins_by_link` | two browsers play to a winner on the live Worker, one socket each, with no unknown console error; the Worker's requests per move, read off the query API | the quota guard first; before a release that changes the client or the transport; a cycle's close | the real edge's behaviour |
| the headed game | the exit spec (§2) | hibernation and the wake OBSERVED live, with chat and a spectator | a cycle's close that changes the wake path | hibernation is never observed live |
| the README pictures | `cd web && YOKOHAMA_WEB_README_SHOTS=1 YOKOHAMA_WEB_E2E_TARGET=live npx playwright test readme_shots` | the ten pictures in `worker/docs/img/` are the deployed screens, each LOOKED AT before it is committed | after a release that changes a pictured screen | a README that shows an older product |
| the door window | `worker/RUNBOOK.md` §19's recipe: `npm run deploy:open` and `npm run deploy:gate`, the Access application's destinations, the burst, `YOKOHAMA_WEB_STATE1_GAME=1 … state1_game` | the public door's two states behave as designed — the name step and no operator under `open`; the door, the shell and a local game with no login and the nine API paths behind Access under the gate — and the revert restores the hostname-wide door | before the public door opens for real; after a change to identity, the operator gate or the path list | the public door opens unread (§8) |
| the mutation battery | the spec's rows, at the reviewed head, each reverted | that the named catchers catch | a cycle's close | a catcher that cannot fail stays in the suite |
| the phones | the iPhone 17 Pro and iPad Pro 13-inch simulators and the Android emulator, through the mobile bridge over WARP, one agent per device with the three questions — the socket and the wake after the Home screen and a lock; persistent storage and a Home Screen install; anything that blocks — and a picture per screen | WebKit and Android Chrome: the wake, storage, the layouts at a phone's and a tablet's size | a cycle's close that changes the client (cycle E, `worker/RUNBOOK.md` §19) | "installs and runs offline" stays a Chromium claim; a lock is untestable on the iOS simulators, and the Android emulator refuses the install, so both stay unread there |

## 4. The architecture and the standing rules

- **`crates/yokohama-web` + `crates/yokohama-wasm` + `crates/yokohama-worker` + `web/` + `worker/` — the browser lane** (merged to master 2026-09-11; the M6 "room" charter is `docs/plans/2026-09-14-web-frontend-m6-room-plan.md`, whose §3 is what stays fixed through every cycle, and the M6.5 hosting umbrella is `docs/plans/2026-09-19-web-frontend-m6.5-hosting-spec.md`; each cycle's record is in §11). **ADDITIVE by construction** — the five fenced crates byte-identical and `SEMANTICS_REV` unmoved, and the CLI's nine `assert_capture` goldens + 18 `guide_sync` tests are how that is known rather than believed. **The layer ROUTES, SCOPES and SERIALIZES and computes nothing about the game** — every model is `screen_model`'s, every word is `describe`'s, every move goes through `Session`'s legal-list gate; a duplicate here would drift and the two frontends would disagree about a rule. `screen_model.rs`'s own module doc names "a later API or GUI layer" as the reason for its import fence, so the port is serde derives plus a handful of `pub(crate)` → `pub`, never an extraction.
  - **One layer, several hosts.** Every game and house route is a typed `pub fn` over `&Games` behind ONE route table and ONE `api::dispatch` (M6.5a), whose parsers make the same calls axum's extractors make, so every 400 sentence is axum's word for word (pinned against axum's own `body_text()`). axum is one host — the NAS/Docker binary, which stays the self-host path — and `crates/yokohama-wasm` is a second: ONE `Games` over `MemStore` with `NoSignal`, `NoArchive` and a settable clock, behind six `wasm-bindgen` exports that are thin wrappers over pure-Rust `_inner` functions, called only from the dedicated Web Worker `web/src/local/worker.ts`. Cloudflare is the third (M6.5c): a stateless Worker that verifies Access's assertion and forwards by NAME to SQLite-backed Durable Objects — a `Table` per table, ONE `House` for the lobby and the house chat, an `Archive` per caller — each running its own instance of `crates/yokohama-worker`, a `cdylib` over the layer without its server half, whose sixteen `wasm-bindgen` exports are called only from `worker/`'s shell (authority: `docs/plans/2026-09-22-web-frontend-m6.5c-objects-spec.md` §2, read through its §10; operations: `worker/RUNBOOK.md`). The platform sits behind seams on `Games` — `Signal`, `Clock` and `Archive` (`Signals`/`now_unix`/`DirArchive` under the `server` feature) — and `platform_calls_stay_behind_the_server_feature` reads the crate's own source and ENFORCES one `#[cfg(feature = "server")]` per platform-calling item; the wasm crate's `this_host_makes_no_platform_call_but_the_clock_and_the_console` holds that crate to the clock, the console the layer's log writes to, and the bindgen glue.
  - **Identity** (`identity.rs`): `YOKOHAMA_WEB_IDENTITY` = `open` | `cf-access` | `worker`, with **NO default — an unset variable exits 1**, because a server that guesses its own door is a server whose door nobody wrote down. Each mode reads exactly ONE header and no other (`X-Yokohama-Claimed-User` / `Cf-Access-Authenticated-User-Email` / `X-Yokohama-User`), so a forged header for another mode changes nothing; `User::parse` trims and lowercases ONCE, so a seat is the same seat on the next request; under `worker` the literal `anonymous` counts as ABSENT, which is what makes an unprotected Worker fail CLOSED. **`open` is the self-host door and also the dev door, not a security boundary** (anyone who can reach the server may use any name), and it is the `open-door` cargo feature: a public build (`--no-default-features --features server`) REFUSES `YOKOHAMA_WEB_IDENTITY=open` by name. Every payload the server builds is built for the caller.
  - **The store** takes a typed `Doc` on every call — `Record`, `Table` (`<id>.table.json`), `Chat` (`<id>.chat.json`) and `House` (`_house.chat.json`; `is_safe_id` refuses a leading underscore, so the underscore IS the reservation) — with `write_if_version` and a typed `Conflict`/`Io`. The kinds share one directory, so `is_safe_id` refuses any id ending in a `RESERVED_ID_SUFFIXES` entry (`.table`, `.chat`; case-insensitively, because APFS and NTFS fold case), and **a new document kind needs a new reservation** — the derivation assert compares the two lists' LENGTHS before it zips, so it says so. `check_house_id` is one helper called from `FsStore::path` and from both `MemStore` entry points.
  - **The table document** (`table.rs`) is who may sit here: seats `Human { user, name }` | `Bot` | `Open` (`user` is the key and `name` only a label), the keeps, the setup `seed` (which lives here and NOWHERE on the wire), a `status` of `Open` | `Dealing` | `Playing`, an optional `Lock { salt, hash }` (16 OS-entropy bytes and SHA-256(salt ‖ password), compared by a fold of XORs), the `unlocked` roster of watchers who gave the password (capped, never on the wire), and a `TurnMirror` of `to_move`/`over`/`winner`, the recorded `bot` (M6.5c2) and `has_events` (cycle E: whether the record holds an event — a bit, so it changes once per game and costs one write) — **a cache** that `persist` refreshes, whose conflict is IGNORED, and which exists so the lobby is built from table documents rather than rebuilt sessions. **"over" is NOT a status**: it is read from the session, because a stored copy goes stale. `TABLE_SCHEMA` is **3** and every earlier binary refuses a schema-3 table, so **rolling back across M6.5a is a SCHEMA rollback** (`deploy/RUNBOOK.md` §5). **Every field read out of a stored document is an INPUT**: `check_shape` bounds each one (`players`, `start_seat`, every `keeps[i]`, `to_move`, the lock) because it runs under the table's lock, where a `PlayerId::new` panic poisons the mutex and takes the whole process out. `games.rs` is the record lifecycle and `tables.rs` the table lifecycle: `join`/`keep`/`settle`/`finalize`/`flip_to_playing`/`delete` hold the TABLE's own `lock_for(id)` and `create` alone the global `table_lock` (a drawn id is not yet one table), which is why `delete` tombstones BEFORE it unlinks.
  - **Seats, the deal, and who may read.** A create answers 201 at `open` (a seat still open) or `dealing`, and the join that fills the last seat flips the table to `dealing` in the same write. Each seat reads only its OWN two cards from `…/deal` — another seat's deal never reaches the wire (`DealView.cards` is the caller's own, and null together with `seat` while they wait; no payload carries another seat's cards or keep) — and **the technologies and goals are withheld until the choice** (`withhold_until_the_choice`: a VIEW, not a rule, because the owner's edition takes the secret choice before they are dealt; the seed, the draw order and `SEMANTICS_REV` are untouched). A caller keeps only for a seat they hold — the seat is checked BEFORE the status, so a stranger's keep is a 403 at every status and learns nothing — and the LAST human keep finalizes record-first. **`admitted(state, id, user) -> (TableDoc, Viewer)`** is every route's door: a seat holder at any status; ANY signed-in caller at an `open` table, for `table`, `events` and `stream` only; nobody without a seat at `dealing` (nobody watches a deal); a `Spectator` at `playing` or over once the table has no lock or their email is on `unlocked` (`POST …/{id}/unlock`); and a locked table's reads refused with "this table is locked — unlock it first". **`viewer_seat`** is the acting seat when the caller holds it, else the caller's lowest, so `…/at/{k}` is the caller's own seat at every `k`; masking is `SeatFacts`' — the CLI's own mechanism, widened by `Repl::index_placements_for`, never re-implemented here.
  - **Spectators, the acting seat, and undo.** A spectator's projection is a NON-ACTING seat's frame: `facts_for(Spectator)` hides every hand, and the projection DROPS the `Hand` panel (`seat: null` on the wire), because a hidden hand still yields a panel that would single out one seat as "yours". **A spectator writes nothing but chat**, and the write sweep proves it behind a store that PANICS on any record read, so "refused before the handle is touched" is a crash and not a sentence. **The acting-seat fence on `input` NEVER drops for a non-acting caller at ANY status** — at game over and lockdown only the 403's wording changes. **Undo** (R-P6): at a table with ONE distinct human email it is the CLI's undo; otherwise only while the trailing event's mover is the caller's seat AND `current_player` still is (replay-derived from `record::state_at(record, len-1)`, checked BEFORE the fence so the answer is always in R-P6's words), and at a FINISHED shared game **nobody undoes**.
  - **The ONE mutation is `POST /api/games/{id}/input` over `Repl::step`** — `step` IS the state machine and a second dispatcher would fork the option numbering — **fenced by `grammar::is_gameplay_line`, an ALLOWLIST**: `step` is the TERMINAL's front door and also answers `save <path>` (an arbitrary filesystem write) and the sticky overlay words (`?`/`index`/`orders`/`techs`/`goals`/`sitecards`/`view`) after which a NUMBER selects a card rather than a move. TWO rules, not one: a HEAD may only be a listed word or a number/area, because `save` and `view` are ordinary alphanumeric words a single permissive rule lets through. What those words reached are read-only GETs instead: `…/ports`, `…/labs`, `…/areas`, `…/log`.
  - **NO play payload carries the setup seed** — hidden-set member #1 (`game/observe.rs`), which regenerates both decks, the boxed pile, the board and every deal. `TitleModel::seed` is `Option<u64>` (the CLI passes `Some`, this crate `None`), `ClientState` has no seed field at all, and default game ids keep the `sim-{…:016x}` shape but draw INDEPENDENTLY of the seed. Two fences that are deliberately NOT the same check: the server searches the committed corpus for the VALUE (the pin above, whose anti-vacuity partner asserts the seed IS in the stored table document and NOT in the create's reply), and the client asserts the SHAPE (`web/src/wire/contract.test.ts`). The one place the value crosses the wire is INBOUND, in `POST /api/games`, where `WireSeed` serializes as a STRING: a `u64` through a JS number rounds silently past 2^53 and deals a different game.
  - **Persistence: nothing exists only in memory, with three named exceptions.** Every applied move the server acknowledged is persisted as its `GameRecord`, and an absent handle is rebuilt through `Session::from_record`. What does not survive a rebuild: `Repl::sub` (an open multi-step picker — `Input::expect` exists for exactly that case), `Repl::privacy`, and any move whose WRITE failed, which is why the reply carries `APPLIED BUT NOT SAVED` out loud instead of reporting a refusal the server did not make. `Games::persist` stamps the terminal digest through `Session::to_record_json` (serializing `record()` directly would disarm the engine's own provenance refusal), compares record identity by seed FIRST (a name is not a game), and then **COMPARE-AND-SWAPS against a digest of the exact JSON this process last wrote or read**, holding the lock across the read, the compare and the write — an undo POPS events, which a length test cannot tell from a stale handle. It DETECTS a cross-process conflict and does not PREVENT one; where this process has no base it falls back to the length test. A CONFLICTING write EVICTS the handle so a stale `Repl` cannot go on answering 200; an I/O failure does not; `create` holds its own lock across check-then-write.
  - **Transport** (`signal.rs`): `Signals` holds one `tokio::sync::watch` channel per table and one for the house, bumped only in success arms, and **a bump follows its write and never precedes it** — proven by gated cells that park the real write and read the receiver quiet. `GET …/{id}/stream` is server-sent events: the current `Events` on connect, one `event: signal` frame per bump, `event: end` on a delete, a comment heartbeat every 25 s (under the edge's 100 s), no `id:` or `retry:` line, and no game lock held across an await; a 403 or 404 ends the stream and any other error writes a comment the client drops. `GET /api/stream` is the house channel, `{seq, chat, rev}` and silent on connect: the lobby re-lists on a frame whose `seq` moved, or one that moved nothing it recognises, and a frame that moved only `chat` or `rev` — a house message or its delete — re-lists nothing (cycle E, R-E9). `SIGNAL_KIND` is `"sse"` on the NAS, MEASURED through the tunnel both ways (M6.3 close §2.1); **the held-request arm is RETIRED since M6.5c2 cycle D (R-D5)** — `GET …/events` answers at once on every host, its `wait` and `after` parsed and ignored (a `wait` above 25 is still a 400); on the Worker it is `"ws"` — a hibernatable WebSocket carrying the SSE frames' data as `{"event","data"}`, one frame per socket per reply, each computed for that socket's own viewer, with `ping → pong` answered at the edge without waking the object. **The client reads the stream with `fetch`, not `EventSource`** (which cannot carry this server's identity header; `web/src/sse.ts` is the line parser), `usePolled` arms NO timer while connected on the stream arm — the socket arm's one timer is its 30 s keep-alive, which stops at hang-up and sends only while the socket is OPEN; after a close it did not make, the socket arm probes ONCE through the source's own `once()`; and no stream or socket opens before the deployment's transport is known (a wait bounded by `BOUND_MS`, M6.5c's I-86) — a browser holds ONE stream per VISIBLE tab and none in a background tab (R-P8), and a game screen holds its stream at every status of the game while visible — `send` parks any frame that lands while it posts, so a self-bump is never walked as an opponent's move. **A consumer's `active` must not toggle on its own handler**: that is a poll's reason, and on a held connection it reopened the stream once per applied move.
  - **The Worker host (M6.5c) — the standing rules.** **The strings rule**: the shell never `JSON.parse`s a record or a table document (a seed is a `u64` a JavaScript number rounds past 2^53) — its only parses are the house copy (seed zeroed; since cycle E the waiting-table expiry reads the table's status off one, made on a scratch module), an `events` reading, the house chat document (for a delete's tombstone and its `rev`, since cycle D), its own `meta` JSON and an archive plan — and Rust-made fixtures reach the pool as text. **Two time bases**: every time the shell keeps (`written_at`, alarm deadlines, retry dues, the marker's `at`, a generation's `at`) is `Date.now()` in milliseconds; the module's pinned reading, in whole seconds, feeds only what the layer stamps. **One batch per request**: a request's read, dispatch and write contain no `await`, every write and the alarm it makes due go in ONE `transactionSync` batch with `setAlarm` beside it, and frames, the house push and the R2 upload follow. **`X-Yokohama-*` is stripped**: the Worker removes every client header named `x-yokohama-…` and sets `X-Yokohama-User` only from a verified Access assertion, judged on the RAW claim (outside tab, space and visible ASCII it is no one) and lowercased by ASCII only; a foreign `Origin` on a WebSocket upgrade is refused. **The dev door** (`YOKOHAMA_DEV_DOOR=on`: the `yokohama-worker` Service Auth credential plus `X-Yokohama-Claimed-User`) is the `worker` door's test instrument — the open door's claim with a credential required, since the layer has had no `claimed` mode since M6.5c2 — it can claim any identity that is an address by `User::parse`'s rule, never an operator's rights; the shell refuses it beside `open`, its client id `DEV_DOOR_CLIENT_ID` is a Worker SECRET, and it goes off with the public door (in cycle E's window through `wrangler.gate.jsonc`, §8). **`YOKOHAMA_WEB_ADMINS` is a Worker SECRET** (`wrangler secret put`, from stdin), never a `vars` entry, and no document spells its value. **The deploy guard** (`worker/scripts/deploy-guard.mjs`, run by `npm run deploy`) refuses a debug or unoptimized module, an assets tree that is not the release, a `workers.dev` or preview URL, the test clock, any drift in the `assets` block, object bindings, migrations or bucket between production's configuration and the others (the pool's two, and since cycle E the door window's `wrangler.open.jsonc` and `wrangler.gate.jsonc`, each held to production but for its door), an operator list in `vars`, and a STALE build — a `gen/` whose source stamp no longer matches, or an assets tree assembled from an earlier `web/dist` or `worker/static`. **`wrangler tail --format json` is pretty-printed JSON**: parse it by whole objects, never by lines. **An exception escaping an object** is a host-written 500 `the server could not answer — try again` with an `object-failed` line. **A picker row carries the code's digest** (the engine's, the CLI's and the layer's sources), so a deploy that changes any of them drops open pickers rather than re-feed them, logged `picker-dropped`. **A generation's time is milliseconds**, so a rollback to a version that drew seconds (the eighteenth deployment or earlier) is a GENERATION-ORDER rollback — a re-created table goes unlisted until its retired generation is pruned (`worker/RUNBOOK.md` §6). **Two TTLs** (cycle E, R-6.5c2.42): `YOKOHAMA_WEB_TABLE_TTL_DAYS` (90 here) expires an idle table and `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS` (7, inert without the first) a table still WAITING for players — status `open`, a seat not yet taken; a `dealing` table waits the first TTL (§15) — decided at the alarm by the status read off the house copy, with its own marker head — "this table waited N days for players and was closed on …"; `/api/health` reports both — on the NAS, the sweep's (cycle F), `null` while unset — and `null` in the browser, which expires nothing. **A first visit reads "connecting…"** on both hosts until its first connection: the client waits (bounded by `BOUND_MS`) for the deployment's transport before it opens a stream or a socket.
  - **Chat** (`chat.rs`): a `ChatDoc` of `Message`s with a TAGGED author (`seat` | `watcher` | `bot`, the last produced by no route yet). `trim` drops from the OLDEST end and never reuses a number, `check_shape` runs on read AND on write, and **length is never a refusal in either direction** — an over-cap text is served as stored and an over-keep document trimmed on its next append, so lowering either limit on a running volume takes no table out. One appender behind two seams (a table's own lock then `bump_table`; the house's `house_lock` then `bump_house_chat`), **keyed by the key it READ, never the stored `id`**. **Delivery is a COUNT, never the text**: `Events` carries `chat` and the house frame is `{seq, chat, rev}` — a `watch` channel can lose one of two values between reads, so a frame carrying text could not survive the seam. A seated caller is named by their SEAT (a name in the body is a 400); a watcher may send one, and **a watcher's post is the one write a spectator may make** — POST ≡ GET at every admission cell, so a locked table's chat is behind its password both ways. Three launch variables WITH defaults — `YOKOHAMA_WEB_LOBBY_CHAT` (on), `YOKOHAMA_WEB_CHAT_KEEP` (500, or `all`) and `YOKOHAMA_WEB_CHAT_TEXT` (500) — are reported on `/api/health` so the client sizes its input from the server's rule, and a BAD value still exits 1 naming the variable.
  - **Deployment switches and the archive** (M6.5a, M6.5b2). `YOKOHAMA_WEB_MULTIPLAYER=off` takes the game and chat rows out of the route table and the three stream routes off the axum router, so there is nothing to probe; `/enter` is a 302 turnstile served when EITHER multiplayer or the archive is on, which is how Access moved off the hostname without the app ever learning that Access exists. **The archive** (`YOKOHAMA_WEB_ARCHIVE_DIR`; `/api/health`'s `archive` reads `off` or `dir`): a finished game is archived ONCE under `rev7/over/` and an abandoned one under `rev7/abandoned/` — a failed put at game over is logged and leaves `archived_at` unset, a failed put at abandon REFUSES the delete. **Whether an abandon saves is ONE rule**, `tables::saves_on_abandon` — the archive on, the table `Playing`, no `archived_at`, an event recorded — asked by the delete of the RECORD and carried on every `TableView` as `saves` over the mirror's `has_events`, so the lobby and the table say "Abandon and save" from the rule the delete applies (cycle E). `GET …/{id}/record` serves a seat holder's FINISHED game as the store's exact bytes (409 while it runs, 403 for a watcher). `POST /api/archive` strict-replays what a browser sends and files it under `browser/<caller_segment>/` by the leaf the replay shows; `backup.rs` is ONE validator (`check_pair`) that route and the backup envelope both call, so the two cannot drift.
  - **Single-player runs entirely in the browser** (M6.5b, M6.5b2). `web/src/local/` runs the layer in the Web Worker as ONE serialised actor with an ownership generation checked in every write; every move is committed to IndexedDB BEFORE the page is answered, and an unreadable store is a refusal, not a trap; the app asks to keep its storage at the FIRST COMMITTED MOVE, never at boot; two tabs on one game settle through the Web Locks API; an export takes one whole pair, from the Worker only where that tab OWNS the game at the current generation; and the client's local arm is chosen by the `local-` id prefix or the caller's intent. Every host serves the batch run route `GET …/at?from=&to=` the walk uses. The first screen (`#/`) is a door that asks the server for NOTHING. **The release is precached whole** under a name that is a digest of its own bytes (`web/src/sw.ts` is the service worker's SOURCE; the precache generator bundles it into `dist/sw.js` with the inventory and the cache name built in): installed all-or-nothing with every body digested, the shell installed from `/shell.bin` and never from `/index.html` and every navigation answered from those bytes with the HTML type the worker sets itself (cycle E, R-6.5c2.41; trap 33), the update offer rendered on the door alone and taken by a click, a waiting release that nobody clicks activating by itself once every old page has closed, the previously ACTIVATED generation RETAINED (a tab that did not click is served its own release from it), and `/cdn-cgi` never intercepted, because the Access login hop lives there. A game finished offline is queued with its reason and uploaded when the connection returns, and Abandon has THREE answers — send, delete, or ask — because "unknown" is not "off". A restore never erases: identical bytes read "already on this device", a clash REFUSES by name, and a save at another `SEMANTICS_REV` is kept and listed under "Will not open here" with a two-click Remove. The fonts and help pictures are self-hosted, so an offline launch keeps its typefaces.
  - **The explanation surface** (`explain.rs`, `web/src/Explain.tsx`): `GET …/suggest` and `GET …/why[?event=n]` are the only two routes that COMPUTE — they read an opinion about a state, never advance one, and DROP the per-game lock before searching. **A typed refusal is a 200 with a refusal body**, not a 4xx: `why` declining because the last move was a human's is the answer. The DTOs mirror `eval::explain` without touching core; the counters are `driven_runner::counter_rows`' fifteen named verbatim; every move is worded by `describe_move`; `why`'s twelve refusals are `why::account_parts`' own strings carried VERBATIM — **that gate is reached, never reimplemented**. `suggest`'s four refusals are the one place the layer words anything itself (`SuggestRefusal::message`'s doc rules on each: `GameOver` byte-identical to the CLI's line, `LiveMode` hand-wrapped, `BotSeat` and `Locked` AUTHORED). **The exact-sum identity `total == Σ product + terminal.value` rides the payload** and the client foots it on screen; all 34 terms ship, the zero ones included, for the READER — a term shown at zero is one that was computed. **`ranking[].chosen` is a wire field, not `index == 0`**: ties at the top are broken by a keyed draw. **`why` masks the roads not taken** under `HandVisibility::Masked` (an unplayed `FulfillOrder` words as "Fulfill an order from hand"); the move PLAYED is never masked, and `suggest` masks nothing.
  - **The card indexes and the final score** (`outcome.rs`, `web/src/Index.tsx`): `GET …/index/{orders|techs|goals|sites}[?filter=]` serves `screen_model::build_index` over `Repl::index_placements_for` — the CLI's `index_placements` WIDENED to take the caller's seat rather than copied, because that function's doc defends a leak fence (no other seat's hand); `target` is always `None`, so an index here numbers nothing; the filter is `card_prompt::parse_filter` — **one card-entry grammar in this project, never a second** — and an unparseable term is a 400 carrying the CLI's own refusal. `GET …/result` is the ranked breakdown, a 409 while the game runs; `Placing.won` is a wire field because **a margin of 0 is not a win**.
  - **THE LOCKDOWN RULE: the lockdown gate fences COMPUTATION, not PROJECTION.** A read-only GET serves a locked session (the prompt reports the lockdown), so the card indexes and the final score answer; `/suggest` and `/why` REFUSE, because both re-run the bot against the record the driver declared faulty. Pinned in both directions.
  - **Card art** is committed under `crates/yokohama-web/assets/art/`, keyed on the wire's own identifiers: orders by `OrderCardId` (the printed corner index), **techs by TITLE** (inside `rules_hash`; the registry's `codes` slug is hashed nowhere — **do not add a `TechRegistry::code` accessor for this**, it was written, reviewed and reverted), sites by the 0-based `printed_site_index` the CLI's tile also prints, achievements and area tiles by serde name.
  - **The client** (`web/src/`): hash routes in `route.ts` (a table's link is `#/t/<id>`); ONE `Chat.tsx` on its four hosts, its fold remembered per browser (`chatFold.ts`); `web/src/fit.ts` is the fit rule's oracle (`clamp(10rem, min(byW, byH), 22rem)`) and `web/src/tile_rects.json` one file for the code and the review sheet; the opponent's turn WALKS on the table through `diffFrames`, a pure function over two frames that knows no rule, and a slot lights on ARRIVAL, never on the shift a take causes (owner ruling); `TableSections` takes a `scrubbing` flag DISTINCT from `replaying`, the fetch guard without which a walk costs a second `/ports` per bot turn, and since cycle E is keyed on `state.events`: the ports and labs come from a landed run's last frame, fetched only on mount, after an undo's shrink or when the run never came, `/areas` only when the board's sites change, and a picker step fetches nothing (R-E9; 16.74 → 9.59 `/api` requests per applied move over the local walk); the lobby pages Your tables, Open tables and Being played with a "more" under each (20 a page; cycle E); a table's link opens straight to the table, drawing its band and "opening…" and never mounting the lobby (cycle E, R-E7); the three build values draw the door's blurb and link, the lobby's quiet link and the footer on every screen (cycle E, R-6.5c2.40); the lobby and the open table are chat COUNT hosts, each learning of a new message from its frame's `chat` count (cycle D); and the pieces band's left edge is a MEASURED constant (`ICON_INK_RIGHT`) that only an eye could set — on eleven of the thirteen area arts a pale-ground detector gives a different answer.

## 5. What not to re-derive: the traps

The measured traps M6.5c paid for are numbered 1–27 in
[the M6.5c2 handover](plans/2026-09-26-web-frontend-m6.5c2-handover.md) §6, a dated record. Read it before any
Worker or live-tier work. The standing rules in §2 and §4 carry the ones that became rules. M6.5c2 added:

28. **An `ARG` with no `--build-arg` is UNSET in the build's `RUN` steps; `--build-arg X=` sets it to the EMPTY
    string** (BuildKit, measured; `deploy/Dockerfile` says so beside its three `ARG`s). They are not the same
    input.
29. **GitHub runs an unspecified `run` shell as `bash -e {0}`.** A step that captures output with `out=$(…)` must
    `set +e` first, or a failing command prints nothing (`web.yml` and `docker.yml` both do).
30. **The backup envelope is regenerated by the LIB cell** (`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web
    --lib backup`), not by `wire_fixtures`. A change to what the envelope carries needs both commands.
31. **A pin over a fixture that agrees with the build can be vacuous.** The drift filter's cell had no drift to
    filter until the version moved. Prove such a cell with a mutant.
32. **The Docker workflow must be told about every crate the image COMPILES**, the CLI's own dependency on the
    BGA bridge included, not only the crates the image is named for. Since R-6.5c2.31 `docker.yml` runs on
    dispatch only, so the rule binds the day a `paths` trigger returns.

Cycle E added:

33. **The zone's JavaScript Detections injects a script into EVERY `text/html` response at the edge**, bound to
    the `cf_clearance` cookie of the visit that fetched it. A service worker that precached `/index.html` stored
    that script with it; once the clearance was gone, every returning browser failed the new release's install
    and stayed pinned to its last one (2026-09-29, spec §10.11). The fix is the shell precached from
    `/shell.bin` — `index.html`'s bytes as `application/octet-stream`, under a name the edge does not rewrite —
    and a navigation answered from those bytes with the HTML type the worker sets itself (R-6.5c2.41;
    `worker/RUNBOOK.md` §19's update reading). **The NAME does not keep it out of the edge's cache; the header
    does.** `.bin` IS on Cloudflare's default cached-extension list (spec §10.11's premise that it is not was
    false, ruling 9-e), so a `/shell.bin` served with no header would be the previous release's for two hours
    after a deploy (`web/src/sw.ts`). Both hosts serve it `no-store` — `assets.rs`'s `cache_control` on the NAS,
    `worker/static/_headers` on the Worker. The NAS read `cf-cache-status: BYPASS`; the Worker's assets read
    `HIT`, interpreted as the platform's content-addressed cache serving the new release's bytes, an
    interpretation `worker/RUNBOOK.md` §19 lists among what it does NOT claim. The snippet stays on `/index.html` by
    design: Bot Fight Mode stays on.
34. **A reader's expected sentence drifts with the screen.** `read-deploy.mjs install` waited for "Ready to play
    offline." with a period; cycle B dropped the period from the door, so the reader printed DOES NOT HOLD over a
    good install (spec §10.11). It now waits for the words without the period, pinned by
    `read-deploy.test.mjs`'s "waits for the sentence the door draws when the release is ready". A reading that
    waits on words takes them from the screen as it is.
35. **The Free plan allows ONE rate-limit rule per zone, shared across the zone's sites.** The Worker's oracle
    paths went into the zone's existing rule, whose `/` clause guards every host of `moore.nyc`: its value is
    20 requests per 10 s per IP and colo, over `/` and, on `yokohama.moore.nyc` alone, the join, unlock and
    archive paths (§8). A second rule is refused, and a change to the one rule changes it for the zone.
36. **`wrangler deploy --var NAME:value` prints `(hidden)` in place of the value, and the deploy guard cannot
    read it.** A deployment's door is therefore a FILE's `vars` — `wrangler.open.jsonc`, `wrangler.gate.jsonc` —
    that the guard judges under `--config`, never an override on the command line (Task 8 measured it: NO).
    `--var` stays right for the one variable no door depends on, the exit's `YOKOHAMA_WEB_COUNT_WAKES`.
37. **The operator secret refuses the `open` door.** The shell will not start under `open` with
    `YOKOHAMA_WEB_ADMINS` set, and a secret survives a deploy. So delete the secret BEFORE an `open` deploy, and
    put it back only AFTER the gate deploy that follows (ruling 10-b; the order followed in the window): put
    back first, it would 500 the open Worker until the gate deploy lands — derived from the shell's
    `openDoorRefusals` (`worker/src/config.ts`), which cells pin, and not measured.

## 6. What a save file says

Every record the web lane writes is the engine's own `GameRecord` — the JSON the CLI's `save` writes, which the
CLI resumes and `why` reconstructs. For an evaluation pipeline, this is what it can rely on:

| fact | field | written by |
|---|---|---|
| the rules | `semantics_rev`, `rules_hash`, `terminal_digest` | every record since Phase 2.9; `replay_strict` gates on them |
| the exact bot: its identity, version, every knob and its seed | `run.seats[i]`: `controller`, `version`, `params`, `agent_seed`, `observation` | every game the table layer finalizes (M6.1 on); an all-human game carries a `human` entry per seat |
| the release that wrote it | `engine_version` | always; meaningful from `v0.6.0` on (§7). Provenance, never a compatibility key |
| where it was played | `run.run_id`: `web/server` (the NAS or any self-host), `web/worker` (Cloudflare), `web/browser` (a single-player game in a browser); the CLI's BGA importer writes `bga-import/<table>` | every game finalized since M6.5c2's first deployment (2026-09-28: the Worker's twenty-first, the NAS's fourteenth) |

- **Key a record** by `(semantics_rev, rules_hash)` for whether it replays; by
  `run.seats[i].(controller, version, params)` for which bot played; by `engine_version` for the release; and by
  `run.run_id` for the host.
- **The bot's label** ("V4") is derived, never stored: `crates/yokohama-web/src/bots.rs` maps a recorded
  `(controller, version, params)` through `Contestant::from_parts`.
- **A game that is not over, whose recorded bot this build does not run, refuses to open on the web**
  (R-6.5c2.10). A finished one opens, scores and downloads. The CLI still loads either, where a foreign bot's seat
  resumes human with a warning.
- **Before M6.5c2's first deployment:** a record carries no `run_id`, and a table dealt earlier carries no `bot`
  in its mirror until its next move. A finished one never gains it.
- **`run` travels byte for byte through every save path** (pinned in M6.5c2): the store, the record route
  (`GET /api/games/<id>/record`), the archive on both hosts (`rev7/over/`, `rev7/abandoned/`,
  `browser/<caller>/rev7/…`), the browser's IndexedDB, the export, and the backup envelope.
- **A record carries its setup seed**, which regenerates every hidden card. Treat a save as the whole hidden set:
  saves go to gitignored places (`docs/temp/owner-games/`), never a tracked one.

## 7. The release-version rule

**One release number** (R-6.5c2.19, the owner): the root `Cargo.toml`'s `[workspace.package] version`, which
every crate inherits, and both npm packages' (`web/package.json`, `worker/package.json`) — `0.6.0` at this
writing. The cell `the_two_npm_packages_carry_the_workspace_version` (`crates/yokohama-web/src/lib.rs`) holds them
together.

**Bump it at every release that can change what a save means or which bot plays** — the AI lane's V5 ship
included. **Tag the release commit `vX.Y.Z` after the merge, on master.** The tag is annotated, its subject
`Yokohama CLI vX.Y.Z — <headline>`. It runs `.github/workflows/release.yml`: the CLI built for Linux, macOS and
Windows, its tests and the `piped_dogfood` gate, then `gh release create` from the tagged commit.

**The bump, as the move to 0.6.0 measured it:**
1. `Cargo.toml`'s version, then `cargo update --workspace --offline` — the nine members' lock lines, and nothing
   else.
2. `npm version X.Y.Z --no-git-tag-version --ignore-scripts` in `web/` and in `worker/` — each `package.json` and
   its lock's two root lines. A third-party package that happens to share the old number is untouched.
3. `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` for health's `server_version` and
   `archive-walk.json`'s records, AND `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --lib backup` for the
   envelope's `app_version` (trap 30). READ the diff: the version and nothing else.
4. `deploy/Dockerfile.runtime`'s example tag.
5. Before the suites, `cd web && npm run build:wasm` and `cd worker && npm run build:module:debug`: a stale module
   answers the old version, and goes red.
6. Before the next live replay, its reset recipe (`worker/RUNBOOK.md` §7, "The replay's reset").
   `archive-walk.json`'s records changed, so the first sends meet a 409.
7. After the deploy, every older game would draw the engine-version drift line. The layer drops that one line
   (`client::without_version_drift`); the CLI shows it, by design.
8. The deploy records name the new NAS tag and the Worker's `server_version` reading.

**A new bot is a web change too:**
- a new `Contestant` or `NamedIdentity` variant does not compile in `crates/yokohama-web/src/bots.rs` until it has
  a label — one line, since the match has no `_` arm;
- re-pointing `PROVISIONAL_DEFAULT_BOT` fails `wire_fixtures` until `web/src/wire/production_bot.json` is
  regenerated.

## 8. The public door's preconditions

Access stays hostname-wide on both hosts until the repository is public (the owner). The FULL list is spec
§2.10's six and the umbrella's three Access-door items (R-6.5.3, R-6.5c2.34). Cycle E VETTED the door in two
states on the Worker and REVERTED it, on 2026-09-30 between 12:12 and 12:37 UTC (`worker/RUNBOOK.md` §19, "The
door window", a record and the flip RECIPE a later operator runs):
- **State 1** — no door but the client's name step: the Worker deployed from `wrangler.open.jsonc`, the operator
  secret deleted first, the hostname taken off the Access application.
- **State 2** — the door on the API paths alone: the Worker deployed from `wrangler.gate.jsonc` (the `worker` door,
  the dev door `off`), the secret put back, and Access on the nine paths `worker/scripts/access-paths.mjs` prints
  (`/api/admin`, `/api/archive`, `/api/art`, `/api/chat`, `/api/games`, `/api/labels`, `/api/me`, `/api/stream`,
  `/enter`) while the door, the release and the shell stay public.
- **The revert** — `npm run deploy` (the dev door on) and the application back on the three hostnames, its `aud`
  unchanged through all three changes; the live replay HOLDS after it.

Each precondition, with its state after E:

| precondition | from | its state after cycle E |
|---|---|---|
| the path-scoped Access application (M6.5b2 spec §3.2) | §2.10 | **vetted in E's window** as state 2 and reverted. From a headed browser without WARP: `/` and `/shell.bin` 200; a `fetch` of `/api/me` or `/api/games` a 401 with `WWW-Authenticate: Cloudflare-Access`; a navigation to `/enter`, `/api/games` or `/api/admin/tables` Access's login page. Over WARP as the owner: the lobby through `/enter` with no login page, both sockets opened. The destinations are the router's (`access-paths.mjs`, with its drift cell) |
| the dev door off (`YOKOHAMA_DEV_DOOR`), and `DEV_DOOR_CLIENT_ID` deleted, the guard printing the door | §2.10 | **vetted in E's window**: `wrangler.gate.jsonc` sets it `off` and the guard prints its door line; a claimed header then named no one. **Recorded with its condition:** the client id stayed a secret through the window (inert while the door is off) — delete it at the real flip |
| the card art's publication decision (`ASSETS.md`) | §2.10 | **recorded, the owner's**: the art carries no grant (`ASSETS.md`). `/art/` is not among the nine paths, so in both states the release served it without a login for the window's minutes (a derivation from the path list: no art file was fetched in the window) |
| the project link set (`YOKOHAMA_PROJECT_URL`) | §2.10 | **met in D**, the first release built with it; E's release carries it with the blurb and the notice (§14) |
| requests per move under a public load | §2.10 | **recorded with its condition**: the live two-browser game read 7.9 of the Worker's own requests a move (cycle E, down from about 13 in D), and a plain two-person game under state 1 read 8.6 per event with its setup in, over 17 events. No public load was measured. The ceiling is Free's 100,000 requests a day, shared with the account's other Workers, under the owner's line of 80,000 |
| the archive route's untrusted record (M6.5b §6.3; spec §7) | §2.10 | **recorded with its condition**: strict replay, the 1 MiB cap and the check order (D) are its present guard, and the rate-limit rule covers `/api/archive`; under state 1 anyone could post, and a record reaching the engine from a stranger stays the named risk |
| the seat-email roster off the wire | umbrella | **met in D** (R-6.5c2.35): a human seat crosses as `{kind, name, you}`, with the address only for the caller's own seats, and a leakage pin asserts no payload built for one caller carries another seat's address — proven on the walked corpus and the stored documents, not on a live stranger |
| the password oracles' rate limits (`…/join`, `…/unlock`) | umbrella | **vetted in E's window, and KEPT**: the zone's one rule at 20 requests per 10 s (trap 35) — the live replay's archive rows peak at 8 in a window, doubled and rounded up; 25 wrong-password joins read 20 × 403 then 5 × 429 (error 1015) and a 403 again after 11 s — through the dev door from the workstation in 2.74 s, and from a clean context through the open door under state 1 in 2.22 s; the live replay HOLDS under it. **This is the Worker-side default.** The NAS hostname's `…/join` and `…/unlock` are not in the rule; the NAS stays behind Access |
| chat moderation | umbrella | **met in D** (R-D4): a message's author or the table's creator deletes it at a table, its author alone in the house, as a tombstone with a revision counter |

**What the window read under state 1:** the door and the shell public; the name step; `GET /api/admin/tables` the
layer's 404 for every typed name, the owner's own address typed as a name included (under `open` a name IS the
person, R-6.5c2.3, and there is no operator: the shell refuses to start beside the secret); a plain two-person game
of 17 events archived under the bucket's `test/` prefix. **What the window did NOT read:** row 108's
single-audience reading was taken as the owner's WARP identity, not as a second signed-in address; row 119's
refusal of an unlisted signed-in person was not read, since it needs a second identity only the owner can create.
Both are recorded in `worker/RUNBOOK.md` §19's "does NOT claim".

**Still owed before the door opens for real:** the art's decision; a fresh-history repository, since
`ASSETS.md`'s carve-outs and, before the M6.5c2 scrub, the owner's address live in the history; the dev door's
client id deleted; rows 108 and 119 read with a second identity; and, for an open-door Worker, a quota that is
anyone's to spend, with the rate-limit rule as the deployer's tool.

## 9. How the documents are built

- **Generated, and checked by a test:**
  - the CLI guide pair (`guide_sync`);
  - the wire corpus and its request scripts (`wire_fixtures`), and the backup envelope (`backup`'s lib cell);
  - `production_bot.json`, `identity_cases.json`, `seed_cases.json`, `site_space_words.json`, `house_frame.json`;
  - the card art (`docs/tools/mkcardart.py`), whose join the layer's `art.rs` checks and whose truth only an eye
    can;
  - the ten README pictures (`readme_shots.spec.ts`), whose truth only an eye can.
- **Checked by a test, written by hand:** this file's API map (§13, `lane_document.rs`) and its variables (§12,
  `lane-variables.test.mjs`).
- **Frozen at their close:**
  - the dated `docs/plans/` records. Corrections go forward, with one recorded exception for personal data
    (`guidingpath.md`'s row of the day it was taken);
  - `guidingpath.md`'s decision log (append-only) and its archives;
  - the two RUNBOOKs' deployment records.
- **Current-state pointers, rewritten at a close:**
  - `CLAUDE.md` and the board (`docs/handover.md`);
  - the READMEs — the root one, `deploy/README.md` and `worker/README.md`;
  - this file;
  - the RUNBOOKs' operating sections.

## 10. Operating facts

- **Live operations run from the owner's workstation.**
  - The NAS is operated through Portainer's Docker API (the Portainer MCP's `dockerProxy` is the same API), by
    `deploy/RUNBOOK.md` §1–§3's recipe: the musl binary and `web/dist` built on the workstation, the image built
    on the NAS from a context it fetches, and the swap as ONE script. The workstation's LAN address changes
    between sessions: read it from `ifconfig` for the context's fetch.
  - The Worker is operated through `wrangler` with the owner's login, by `worker/RUNBOOK.md` §1–§2; the Access
    application and the zone's rate-limit rule through the Cloudflare API (the MCP).
  - After every deploy, look at both layouts on screen.
- **WARP first.** A reading as the owner goes over WARP, and `warp-cli status` reading Connected is not enough:
  WARP's device login can expire (trap 25), and only the owner re-authorizes it.
- **The update reading** (§3): prepare one persistent browser context per host at the release BEFORE the deploy
  — opened until "Ready to play offline", then its `cf_clearance` cookie cleared — and after the deploy reload it
  once and read that the install fetched `/shell.bin` and no HTML, that two caches stand, and that "A new version
  is ready. Reload to use it" is offered; click it and read the new worker active and the old cache kept. A fresh
  profile beside it reads the new release alone. The record is `worker/RUNBOOK.md` §19's Reading 3.
- **The phones** (§3): one agent per device through the mobile bridge, over the workstation's WARP, so both hosts open
  signed in; the device ids from `mobile_list_available_devices` (the Android emulator starts under its `.ini`
  name, `Medium_Phone_API_36.1`, and that is also its bridge id); screens read by the element list, not by
  pictures alone; every Worker table an agent creates abandoned before it finishes. **A game screen has no
  Abandon for a hosted table** — the lobby row's square is the control. Landscape pictures come back rotated.
- **Counting on the live Worker.** The quota guard is GraphQL (`worker/RUNBOOK.md` §7): through the Cloudflare MCP
  the figures are under `result.viewer`, not `data.viewer`, and the `date` and `datetime_geq/leq` forms agree;
  the periodic rollup of rows read and written LAGS by hours, so a window's rows are read the day after. A
  window's requests are the Workers Observability query API's: every ad-hoc body needs a `queryId`,
  `$workers.durableObjectId is_null` splits the Worker's own requests from the objects', and a log line is found
  by its full-text `needle` (`archive-landed`, `picker-dropped`, `object-failed`, `wake`).
- **Cleaning out old games on the NAS.** With `YOKOHAMA_WEB_TABLE_TTL_DAYS` and `YOKOHAMA_WEB_ARCHIVE_DIR` set, the hourly sweep does it (cycle F, R-F9), its `sweep-listed` lines before its `sweep-ended` ones in the container's log; with no TTL set, no sweep runs, and the by-hand recipe (stop, remove the three files of an id, start) is `deploy/README.md` §8 beside the sweep's description.
- **Pulling a game as a save file.**
  - A FINISHED game is `GET /api/games/<id>/record` as one of its seats: the store's exact bytes. Over WARP on
    either host, the owner holds the owner's own seats.
  - The archives hold every finished and abandoned game too: the NAS's `/data/archive/rev7/{over,abandoned}/`,
    and the Worker's bucket `yokohama-archive` (the read-only key in `.dev.vars`, `worker/RUNBOOK.md` §7).
  - A RUNNING game on the NAS is the Docker archive endpoint through Portainer:
    `GET /containers/yokohama-web/archive?path=/data/games/<id>.json`, a tar holding the record.
  - Name a pulled copy `web-<date>-<id>.json` under the gitignored `docs/temp/owner-games/`.

## 11. The records of record

**The web lane's records of record** — read a record's "does NOT claim" section (§7 in every close from M6.1 on) before trusting a sentence its cycle wrote about itself; every release's deployment record is `deploy/RUNBOOK.md` §3 on the NAS and `worker/RUNBOOK.md` §8 onward on the Worker, and the lane's early working documents are untracked, in `docs/temp/webresearch/`.

| Cycle | Record of record |
|---|---|
| M2 — the card art, and the CLI parity gap | `docs/plans/2026-08-31-web-frontend-m2.md` |
| M2.5 — the client's test harness | `docs/plans/2026-08-31-web-frontend-m2.5.md` |
| M3 — it explains (§9 the six-lens review) | `docs/plans/2026-08-31-web-frontend-m3.md` |
| Wave 0 — the correctness floor, a fix wave between M3 and M3.5 | `docs/plans/2026-08-31-web-frontend-wave0.md` |
| M3.5 — the screen (§6 the close) | `docs/plans/2026-09-01-web-frontend-m3.5-spec.md` |
| M4 — the replay scrubber (§8 the review) | `docs/plans/2026-09-01-web-frontend-m4-spec.md` |
| M5 — the table (§8 the close) | `docs/plans/2026-09-01-web-frontend-m5-spec.md` |
| M5.5 — the table reads | `docs/plans/2026-09-05-web-frontend-m5.5-close.md` |
| The owner's first-run corrections (§5 the record) | `docs/plans/2026-09-06-web-frontend-m5.5-owner-corrections.md` |
| M6.1 — identity and seats | `docs/plans/2026-09-15-web-frontend-m6.1-close.md` |
| M6.2 — the lobby | `docs/plans/2026-09-16-web-frontend-m6.2-close.md` |
| M6.3 — transport and spectating | `docs/plans/2026-09-16-web-frontend-m6.3-close.md` |
| M6.4 — chat | `docs/plans/2026-09-19-web-frontend-m6.4-close.md` |
| M6.5a — the seam | `docs/plans/2026-09-20-web-frontend-m6.5a-close.md` |
| M6.5b — single-player in the browser | `docs/plans/2026-09-21-web-frontend-m6.5b-close.md` |
| M6.5b2 — the front door, installable and offline | `docs/plans/2026-09-22-web-frontend-m6.5b2-close.md` |
| The PR #37 fix wave (§5 the record) | `docs/plans/2026-09-22-web-frontend-m6.5b2-pr37-fixes.md` |
| M6.5c — the objects | `docs/plans/2026-09-26-web-frontend-m6.5c-close.md` (its spec's dated §10 carries the corrections) |
| M6.5c2 — the cleanup, cycle A "the layer" (PR #39) and cycle B "the screens" (PR #40) | each cycle's gitignored SDD ledger and its PR; the next cycle's handover restates what shipped — `docs/plans/2026-09-27-web-frontend-m6.5c2b-handover.md` §1 for A, `docs/plans/2026-09-28-web-frontend-m6.5c2c-handover.md` §1 for B; the spec `docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md` §4 is the cycle map |
| M6.5c2 cycle C — the hosts and the close (PR #41) | `docs/plans/2026-09-28-web-frontend-m6.5c2c-close.md` (the spec's dated §10.8 carries the corrections) |
| M6.5c2 cycle D — the code (PR #42; both hosts redeployed) | `docs/plans/2026-09-29-web-frontend-m6.5c2d-close.md` (the spec's dated §10.10 carries the corrections) |
| M6.5c2 cycle E — the finish (PR #43, opened at its close; the final reviewed head `3107eb8b`; both hosts redeployed, the door vetted and reverted; the merge of the stack from #31 through E's PR and the `v0.6.0` tag are its last steps) | the close `docs/plans/2026-09-30-web-frontend-m6.5c2e-close.md` (its §7 what the cycle does NOT claim); the plan `docs/plans/2026-09-29-web-frontend-m6.5c2e-plan.md`, entered from `docs/plans/2026-09-29-web-frontend-m6.5c2e-handover.md`; the live readings in `worker/RUNBOOK.md` §19 and `deploy/RUNBOOK.md` §3.19; the close is written at the plan's Task 16 |

## 12. Every variable

Every variable the web lane reads, in one table (R-6.5c2.45). **The kind** says who sets it: a *build value* is
read when the client or a module is built, a *launch variable* by the NAS/Docker binary at start, a *Worker var*
from `worker/wrangler.jsonc`'s `vars`, a *secret* through `wrangler secret put`, *test-only* by a test or a
harness, and *orchestrator-only* by the live operations of §10. A launch variable or a Worker var with a bad value
stops the server at start (the NAS, naming it) or answers every API request 500 naming it (the Worker); the
self-host guide's §3–§4 (`deploy/README.md`) and `worker/RUNBOOK.md` §3 give each one's refusals. The cell
`worker/scripts/lane-variables.test.mjs` holds this table to the names the code spells (§2).

| name | kind | default (unset) | what it means | host | read in |
|---|---|---|---|---|---|
| `YOKOHAMA_PROJECT_URL` | build value | no link | the door's link to the repository, and a quiet link among the lobby's actions; an absolute `http(s)` URL, at most 200 characters | both, one client build | `web/scripts/build-values.mjs` through `web/vite.config.ts`; `deploy/Dockerfile`'s `ARG` |
| `YOKOHAMA_SITE_BLURB` | build value | no paragraph | a paragraph under the title on the door, at most 400 characters | both | the same |
| `YOKOHAMA_SITE_NOTICE` | build value | no footer | one line at the foot of every screen, at most 200 characters | both | the same |
| `YOKOHAMA_RELEASE_FEED` | build value | derived from a GitHub `YOKOHAMA_PROJECT_URL`; any other link, or none, is no feed | the URL the door and the lobby read for the latest release, to draw the notice when its tag is ahead of this build's; unset or empty derives `https://api.github.com/repos/<owner>/<repo>/releases/latest` from a GitHub project link, a URL (`https://`, at most 300 characters) is that feed, the literal `off` is none; never set by a build script, and `npm run e2e`'s build sets `off` | both | `web/scripts/build-values.mjs` through `web/vite.config.ts`; `deploy/Dockerfile`'s `ARG` |
| `YOKOHAMA_TURNSTILE_SITEKEY` | build value | no key: the plain link | the public Turnstile site key, plain text with no whitespace or control character, at most 100 characters; blank draws the plain link | both | the same |
| `VITE_MULTIPLAYER` | build value | on | the literal `false` builds a client whose door draws no way to the server; set by no script | both | `web/src/FrontDoor.tsx`, declared in `web/src/vite-env.d.ts` |
| `YOKOHAMA_WEB_DIST_DIR` | build value | `web/dist` | the `dist` the precache generator walks | the build | `web/scripts/precache.mjs` |
| `YOKOHAMA_ART_DIR` | build value | `crates/yokohama-web/assets/art` | the art the precache generator lists | the build | `web/scripts/precache.mjs` |
| `YOKOHAMA_WASM_OPT_OPTIONAL` | build value | refuse without `wasm-opt` | `1` builds an UNOPTIMIZED module and says so | the build | `web/scripts/wasm-opt.mjs` |
| `YOKOHAMA_WORKER_PICKER_CODE` | build value | computed; not settable | the digest of the engine's, the CLI's and the layer's sources that every picker row carries, so a deploy that changes one drops open pickers | the Worker | `crates/yokohama-worker/build.rs`, read by `src/picker.rs` |
| `YOKOHAMA_WEB_IDENTITY` | launch variable; Worker var | none: the NAS exits 1, the Worker answers 500 naming it | the door: `cf-access`, `open` or `worker` on the NAS (`open` only in a build with the `open-door` feature); `worker` or `open` on the Worker | both | `crates/yokohama-web/src/identity.rs`; `worker/src/config.ts` |
| `YOKOHAMA_WEB_HOST` | launch variable | `127.0.0.1` (the image: `0.0.0.0`) | the address the binary binds | the NAS | `crates/yokohama-web/src/main.rs` |
| `YOKOHAMA_WEB_PORT` | launch variable | `8080` | the port | the NAS | `crates/yokohama-web/src/main.rs` |
| `YOKOHAMA_WEB_DIST` | launch variable | `web/dist` (the image: `/srv/web/dist`) | the client the binary serves | the NAS | `crates/yokohama-web/src/assets.rs` |
| `YOKOHAMA_WEB_ART` | launch variable | the crate's art directory (the image: `/srv/assets/art`) | the card art the binary serves | the NAS | `crates/yokohama-web/src/art.rs` |
| `YOKOHAMA_WEB_DIR` | launch variable | `$HOME/.yokohama/games` (the image: `/data/games`) | the game store | the NAS | `crates/yokohama-web/src/store.rs` |
| `YOKOHAMA_WEB_ARCHIVE_DIR` | launch variable | no archive | the archive directory; `/api/health` then reads `archive: "dir"` | the NAS | `crates/yokohama-web/src/archive.rs` |
| `YOKOHAMA_WEB_MULTIPLAYER` | launch variable | `on` | `off` takes the game, chat and stream routes away; single player still works | the NAS | `crates/yokohama-web/src/lib.rs` |
| `YOKOHAMA_WEB_LOBBY_CHAT` | launch variable; Worker var | `on` | the house chat | both | `crates/yokohama-web/src/chat.rs`; `worker/src/config.ts` |
| `YOKOHAMA_WEB_CHAT_KEEP` | launch variable; Worker var | `500` | messages kept per chat, or `all` | both | the same |
| `YOKOHAMA_WEB_CHAT_TEXT` | launch variable; Worker var | `500` | characters per message | both | the same |
| `ACCESS_TEAM_DOMAIN` | Worker var | none: no assertion is verified, so no one is signed in | the Access team's domain | the Worker | `worker/src/identity.ts` |
| `ACCESS_AUD` | Worker var | the same | the `yokohama` Access application's audience | the Worker | `worker/src/identity.ts` |
| `YOKOHAMA_DEV_DOOR` | Worker var | `off` | `on`: the dev door — the Service Auth credential plus a claimed header names any address, never an operator; refused beside `open` | the Worker | `worker/src/config.ts` |
| `YOKOHAMA_WEB_ARCHIVE` | Worker var | `off` | `r2`: the archive in the `ARCHIVE` bucket, which it needs | the Worker | `worker/src/config.ts` |
| `YOKOHAMA_TURNSTILE` | Worker var, a deploy declaration | `off` | `on` or `off`, never read by the Worker: the deploy guard's check 12 holds it to the assets tree's `build-values.json` — `on` exactly when the client build carries a Turnstile site key — and refuses any other value | the Worker | `worker/scripts/deploy-guard.mjs` |
| `YOKOHAMA_WEB_TABLE_TTL_DAYS` | launch variable; Worker var | none: no table expires | whole days a table stands idle before it expires, saved first where the archive is on; on the NAS the hourly sweep's, which refuses to start without `YOKOHAMA_WEB_ARCHIVE_DIR`, naming both | both | `crates/yokohama-web/src/sweep.rs`, read by `crates/yokohama-web/src/main.rs`; `worker/src/config.ts` |
| `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS` | launch variable; Worker var | `7`, inert while the first TTL is unset | whole days a table with a seat still empty (status `open`) stands idle before it expires, with its own marker sentence on the Worker; a `dealing` table waits the first TTL (§15) | both | the same |
| `YOKOHAMA_WEB_COUNT_WAKES` | orchestrator-only (a Worker var by `--var`) | `off` | `on` counts every wake in `meta.wakes` and logs `wake {id, wakes}`: the exit's window | the Worker | `worker/src/config.ts` |
| `DEV_DOOR_CLIENT_ID` | secret | none: the dev door shut | the `yokohama-worker` Service Auth credential's client id | the Worker | `worker/src/identity.ts` |
| `YOKOHAMA_WEB_ADMINS` | secret | none: the operator routes are the layer's 404 | the operators' addresses, comma-separated; the `open` shell refuses to start beside it | the Worker | `worker/src/config.ts` |
| `YOKOHAMA_WEB_TEST_CLOCK` | test-only | the wall clock | `frozen:<seconds>` or `tick:<seconds>`; the deploy guard refuses it | the pool's two files | `worker/src/config.ts` |
| `YOKOHAMA_WASM_OPTIONAL` | test-only | refuse without the browser module | `1` skips the worker typecheck and the node cells that need the module, and says so | CI's `web-client`; local | `web/scripts/typecheck-worker.mjs`; the `web/src/**/*.node.test.ts` cells |
| `YOKOHAMA_WEB_E2E_OPTIONAL` | test-only | refuse without Chromium | `1` skips every browser spec out loud | local | `web/playwright.config.ts` |
| `YOKOHAMA_WEB_E2E_TARGET` | orchestrator-only | the local binary | `live`: the specs run against the Worker with the credential | the Worker | `web/e2e/live.ts` |
| `YOKOHAMA_WEB_EXIT` | orchestrator-only | off | `1` collects the headed exit game | the Worker | `web/e2e/live.ts` |
| `WAKE_PAUSE_MS` | orchestrator-only | `45000` | the exit game's pauses | the Worker | `web/e2e/exit_game.spec.ts` |
| `EXIT_SCREEN` | orchestrator-only | `1280x832` | the screen's logical size for the exit game's three windows | the Worker | `web/e2e/exit_game.spec.ts` |
| `YOKOHAMA_WEB_README_SHOTS` | test-only | off | `1` collects `readme_shots.spec.ts`, which writes the ten README pictures on the run's target | local or live | `web/e2e/live.ts`, `web/playwright.config.ts` |
| `YOKOHAMA_WEB_README_SHOTS_DIR` | test-only | `worker/docs/img` | where those pictures go | local or live | `web/e2e/readme_shots.spec.ts` |
| `YOKOHAMA_WEB_STATE1_GAME` | orchestrator-only | off | `1` collects `state1_game.spec.ts`: a plain two-person game through the open door, with the tally | the Worker in state 1; local | `web/e2e/live.ts` |
| `YOKOHAMA_WEB_STATE1_OUT` | orchestrator-only | none | a file that game writes its table id to, for the query API | the same | `web/e2e/state1_game.spec.ts` |
| `YOKOHAMA_WEB_TALLY` | test-only | no tally | a file each page appends one JSON line per request to; `web/scripts/tally-report.mjs` reads it as requests per move and 10-second peaks | local or live | `web/e2e/harness.ts` |
| `YOKOHAMA_WEB_SHOTS` | test-only | off | `1` collects `shots.spec.ts`, which re-cuts the help panel's pictures | local | `web/playwright.config.ts` |
| `YOKOHAMA_WEB_GATES` | test-only | off | `1` collects `gates.spec.ts`, M5's visual-gate shooter | local | `web/playwright.config.ts` |
| `YOKOHAMA_WEB_UPDATE_PIN` | test-only | off | `1` rewrites the browser-played pair `browser_played_pin` loads | local | `web/e2e/plays_locally.spec.ts` |
| `YOKOHAMA_WEB_GATE_BOT_SEED` | test-only | OS entropy | replays one whole-game gate seed | local; CI's `web-server` | `crates/yokohama-web/tests/play_a_whole_game.rs` |
| `UPDATE_WIRE_FIXTURES` | test-only | off | regenerates the wire corpus and its scripts (`wire_fixtures`) and the backup envelope (the lib's `backup` cell); read the diff | local | `crates/yokohama-web/tests/wire_fixtures.rs`, `crates/yokohama-web/src/backup.rs` |
| `UPDATE_WORKER_FIXTURES` | test-only | off | regenerates the Worker's Rust-made fixtures under `worker/test/fixtures/` | local | `crates/yokohama-worker/src/lib.rs` |
| `CI` | test-only | unset | Playwright's CI behaviour | CI | `web/playwright.config.ts` |

Not the lane's: `NO_COLOR` (the CLI's), `YOKOHAMA_BGA_CORPUS_OPTIONAL` (the CLI's and the bridge's, in
`release.yml`), `TZ` (the operating system's; one marker cell sets it). The page flag
`window.__YOKOHAMA_NO_SW__` (`web/e2e/sw.ts`) is no variable: a spec sets it to keep the service worker off.

## 13. The API map

Every route, in one table (R-6.5c2.45). **The host** says who answers it: `layer` is the table layer's
`api::dispatch`, the same code on every host — the NAS through axum's adapter, the Worker inside its objects, the
browser inside its Web Worker; `axum` is the NAS/Docker binary's own; `worker` is the Cloudflare shell's own;
`browser` would be the local host's own, and it has none. **Who may call it** uses the layer's door:
`admitted` is a seat holder at any status, any signed-in caller at an `open` table for `table`, `events` and
`stream` only, nobody without a seat while the table is dealing, and a spectator at a playing or finished table
that has no lock or that they unlocked. With `YOKOHAMA_WEB_MULTIPLAYER=off` every `/api/games…` and `/api/chat…`
row and the three transport routes are gone. The cell `crates/yokohama-web/tests/lane_document.rs` holds this
table to the code (§2).

| route | host | who may call it | what it answers |
|---|---|---|---|
| `GET /api/health` | layer | anyone, no identity | the deployment's facts: the version, `SEMANTICS_REV`, the rules hash, the door, the transport, the archive, the chat limits, multiplayer, both TTLs |
| `GET /api/me` | layer | anyone | the door's mode and the caller as it reads them; `user: null` for no one |
| `GET /api/labels` | layer | anyone | every static enum the wire carries, in words: area types, phases, tech effects, achievements, the 34 features |
| `GET /api/art` | layer | anyone | the card art's manifest, answered by the HOST before dispatch (axum from its art directory, the Worker from its static manifest); the layer's own arm is a 404 |
| `GET /api/games` | layer | signed in | a listing, `{games, next}`: `?scope=mine` (the default), `open` or `playing`, `limit` 1–100 (20 by default), `before=<created_at>,<id>` for the next page |
| `POST /api/games` | layer | signed in, seated at the table they open | a table, 201, at `open` or `dealing`; no game yet. On the Worker the table object the body names, or a drawn one |
| `GET /api/games/{id}` | layer | admitted | the game's frame (`ClientState`) for the caller's seat, or a spectator's |
| `DELETE /api/games/{id}` | layer | the table's creator, seated | the abandon, 204; the game saved first where `saves` says so |
| `GET /api/games/{id}/table` | layer | admitted; any signed-in caller at an `open` table | the table (`TableView`): its seats, status, lock, `has_events` and `saves` |
| `GET /api/games/{id}/deal` | layer | a seat holder | the caller's own two cards, over the set table |
| `POST /api/games/{id}/keep` | layer | a seat holder, for a seat they hold | which card that seat keeps; the last human keep finalizes the game |
| `POST /api/games/{id}/join` | layer | signed in, at an `open` table, with its password if it is locked | a seat |
| `POST /api/games/{id}/unlock` | layer | a watcher, with the table's password | the caller admitted to watch a locked table |
| `GET /api/games/{id}/chat` | layer | admitted | the table's messages after a cursor, with its `last` and `rev` |
| `POST /api/games/{id}/chat` | layer | admitted; a watcher's post is the one write a spectator makes | a message, named by the caller's seat or a watcher's name |
| `DELETE /api/games/{id}/chat/{seq}` | layer | the message's author or the table's creator | the message as a tombstone |
| `POST /api/games/{id}/input` | layer | the acting seat only | one gameplay line through `Repl::step`, fenced by `grammar::is_gameplay_line`; the bot's trailing run returns inside the reply |
| `GET /api/games/{id}/ports` | layer | admitted | the ports board |
| `GET /api/games/{id}/labs` | layer | admitted | the laboratory board |
| `GET /api/games/{id}/areas` | layer | admitted | the area board |
| `GET /api/games/{id}/log` | layer | admitted | the game's log, worded |
| `GET /api/chat` | layer | signed in | the house chat after a cursor |
| `POST /api/chat` | layer | signed in | a house message |
| `DELETE /api/chat/{seq}` | layer | the message's author alone | the message as a tombstone |
| `POST /api/archive` | layer | signed in | a browser's finished or abandoned pair, strict-replayed and filed under `browser/<caller>/`; on the Worker, the caller's own archive object |
| `GET /api/games/{id}/at` | layer | admitted | a run of frames, `?from=&to=`: what an opponent's turn walks through |
| `GET /api/games/{id}/at/{k}` | layer | admitted | the frame after event `k`, as the caller's own seat |
| `GET /api/games/{id}/suggest` | layer | a seat holder, for their own seat | the production bot's opinion of the caller's position; a typed refusal is a 200 |
| `GET /api/games/{id}/why` | layer | admitted | a bot move reconstructed by replay, `?event=n`; a typed refusal is a 200 |
| `GET /api/games/{id}/index/{kind}` | layer | admitted | a card index — `orders`, `techs`, `goals` or `sites` — with `?filter=` in the CLI's grammar |
| `GET /api/games/{id}/result` | layer | admitted | the ranked final score; 409 while the game runs |
| `GET /api/games/{id}/record` | layer | a seat holder, at a finished game | the store's exact bytes; 409 while it runs, 403 for a watcher |
| `GET /api/games/{id}/events` | axum | admitted | the table's counters (`Events`) at once; `wait` and `after` parsed and ignored, a `wait` above 25 a 400 |
| `GET /api/games/{id}/stream` | axum | admitted | server-sent events: the counters on connect, a `signal` frame per change, `end` on a delete, a heartbeat every 25 s |
| `GET /api/stream` | axum | signed in | the house's server-sent events, `{seq, chat, rev}`, silent on connect |
| `GET /enter` | axum | anyone who reaches it | the turnstile: a 302 to `/#/lobby`, served when multiplayer or the archive is on; else a 404 saying the deployment plays single player only |
| `GET /enter` | worker | anyone who reaches it | the turnstile: a 302 to `/#/lobby`; `HEAD` the same; any other method 405 |
| `GET /api/games/{id}/events` | worker | admitted | the table object's counters, through the module's `events` |
| `GET /api/games/{id}/stream` | worker | admitted | a WebSocket upgrade to the table object: the SSE frames' data as `{"event","data"}`, hibernatable; not an upgrade, a 426 |
| `GET /api/stream` | worker | signed in | a WebSocket upgrade to the house: the lobby's socket |
| `GET /api/admin/tables` | worker | an operator (`YOKOHAMA_WEB_ADMINS`) | every table: id, name, status, players, times, size; with no operator list, the layer's 404 |
| `POST /api/admin/tables/{id}/expire` | worker | an operator | an expiry now, saved first; `?force` drops a save that cannot be made; `force` with a value is a 400 |

**On the Worker**, the shell routes by method and path alone: `POST /api/games` and everything under
`/api/games/{id}` to that table's object, `POST /api/archive` to the caller's archive object, `/api/admin…` past
the operator gate, and every other `/api/…` to the house (`worker/src/route.ts`). A path the asset layer has no
file for is a 404 with an empty body, never `index.html`. **On the NAS**, every other `/api/…` path is the layer's
404, `/art/…` is the art directory, and the rest is the client with the SPA fallback (`crates/yokohama-web/src/
lib.rs`, `fallback`). **In the browser**, the local host (`crates/yokohama-wasm`) answers the `layer` rows for a
`local-…` id through the same `api::dispatch` over `MemStore`, called from `web/src/local/worker.ts` alone
through its exports `init`, `dispatch`, `load`, `read`, `set_clock` and `restore_read` (and `debug_trap` in a
debug build); `/api/health` and `/api/me` have no local caller. The paths that are not the API — `/`,
`/shell.bin`, `/sw.js`, `/assets/…`, `/art/…` — are the static release, which the service worker precaches
whole; it answers a navigation from `/shell.bin`'s bytes.

## 14. Getting started

Three ways in:
1. **Play at a hosted site.** Open the Worker at `https://yokohama.moore.nyc`, or the NAS at
   `https://yokohama-nas.moore.nyc`, and pass Cloudflare Access's login (the `yokohama` application's policy
   decides who may). The door offers the bot, friends, and the games on this device. "Play with friends" goes
   through `/enter` to the lobby, where "New table" seats you, the bot or an open seat, optionally behind a
   password; share the table's link (`#/t/<id>`), each person keeps one of two dealt cards, and the game begins.
2. **Self-host in Docker.** `deploy/README.md` is the guide: build the image (the three build values are its
   `--build-arg`s), then run it with `YOKOHAMA_WEB_IDENTITY` set — `cf-access` behind a Cloudflare Access
   application, the repository's default, or `open` for a home network, LAN or VPN — and a named volume at
   `/data`.
3. **Play the bot with no server.** The door's bot card plays entirely in the browser: the layer in a Web Worker,
   every move committed to IndexedDB, the release precached whole so it runs offline once installed. A local game
   calls no server — cycle E read zero Worker invocations for one. Any host that serves the release will do, and
   a build with `VITE_MULTIPLAYER=false` offers only this.

### The defaults both deployments run

As cycle E's release set them (`deploy/RUNBOOK.md` §3.19, `worker/RUNBOOK.md` §19):

| setting | the NAS, `yokohama-nas.moore.nyc` | the Worker, `yokohama.moore.nyc` |
|---|---|---|
| the door | `cf-access`: Access's header | `worker`: Access's assertion, verified by the shell |
| the dev door | none | on, behind Access: the Service Auth credential plus a claimed header |
| the archive | `dir`, `/data/archive` | `r2`, the bucket `yokohama-archive` (`test/` expires after 30 days) |
| the transport (`/api/health`'s `signal`) | `sse` | `ws` |
| expiry | none: both TTLs `null` on `/api/health` | 90 days idle; 7 days for a table with a seat still empty |
| the operator | none: Portainer and the volume | `YOKOHAMA_WEB_ADMINS`, the owner |
| multiplayer and chat | on; 500 messages kept, 500 characters each | the same |
| the rate limit | the zone's rule on `/` only | the zone's rule: 20 requests per 10 s over `/`, `…/join`, `…/unlock` and `/api/archive` |
| the three build values | the same client build as the Worker | `YOKOHAMA_PROJECT_URL` = `https://github.com/jmoore2333/yokohama-assistant/`; `YOKOHAMA_SITE_NOTICE` = `Not affiliated with Asmodee`; `YOKOHAMA_SITE_BLURB` = the owner's placeholder, below |

The blurb both hosts carry, the owner's placeholder text:

> Yokohama, the board game, in your browser. Play the bot here, or play with friends. An independent open-source
> project, unaffiliated with the game's designer and publisher, working toward a better computer player for
> Yokohama.

## 15. What is by design

These read like gaps and are not; each is the owner's ruling or a stated property, with its reason.

- **A finished table has no finished-at rule** (straw poll row 74, R-6.5c2.42). The Worker's 90-day idle expiry
  retires finished tables, as the NAS's sweep does where its TTL is set (cycle F, R-F9); a NAS with no TTL keeps
  every table (R-6.5c2.2). A finished table can be abandoned by its
  creator: its game is already in the archive from game over, so the delete files nothing twice (both hosts hold
  the cell).
- **A seat holder's download of a finished record keeps its seed** (row 83, R-6.5c2.42). The route serves a
  seat holder's own FINISHED game only, and the record is the whole game: without its seed it does not replay,
  and the CLI's `why` reconstructs a bot move from the replay (this document's reading of the reason; the owner's
  word on the row is "Agreed", spec §10.11).
- **A finished local game uploads to the archive the deployment configures, with no per-game prompt** (row 91,
  R-6.5c2.42). Where the deployment keeps no archive it goes nowhere; the first time a browser finds an archive,
  every finished local game on the device goes (M6.5b2 close §6.3 item 7), and a player may notice one under
  `browser/<them>/`.
- **A NAS with no TTL keeps its waiting tables.** The 7-day TTL for a table with a seat still empty is inert
  without the table TTL on both hosts (R-6.5c2.42; cycle F, R-F9), so a NAS that sets none sweeps nothing and its
  owner clears a table by its Abandon; the NAS has no marker or operator either way (R-6.5c2.2, R-F10).
- **The sweep decides, then deletes** (cycle F, R-F9; spec §9 F-25). A pass reads each table's last write and
  then calls the creator's delete, which re-checks the seat and the creator but not idleness, so a move can land
  in the seconds between and the table is ended all the same. The move is not lost. The delete lays its tombstone
  before it reads the record it files, under the lock a save holds from its tombstone check to its write: a move
  whose save began before the tombstone has landed by then and is filed with the game like every move before it,
  and a move whose save begins after it is refused out loud ("…was deleted while this move was in flight"). Every
  Abandon runs the same order.
- **A table stuck at its deal waits the long TTL, not 7 days** (ruling 9-b, on cycle E's whole-branch review).
  The 7-day TTL is for status `open` — R-6.5c2.42's "without enough players" — and a `dealing` table has every
  seat taken and waits on a keep, so it expires by the 90-day idle TTL. Cost: a stuck dealing table lingers 90
  days.
- **Four `expect("a game")` sites in the Worker's module** — `crates/yokohama-worker/src/picker.rs` (two),
  `receipt.rs` (one in production, one in a test) — each `handle.lock().expect("a game")` on a module instance
  that is single-threaded by construction (one instance per object, called one request at a time). `lock()` returns
  its `Err` only for a POISONED mutex, and on `wasm32-unknown-unknown` a panic aborts rather than unwinds, so the
  guard is never dropped and the mutex is never poisoned (`crates/yokohama-wasm/src/lib.rs`'s `debug_trap` doc
  says the same); the `expect` never fires short of a defect, which would be worth the host-written 500.
