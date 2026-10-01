# M6.5b2 pre-implementation review feedback — 2026-09-21

**Recommendation: revise the spec and plan before starting implementation.** The overall direction is sound, but several proposed mechanisms do not yet deliver their stated guarantees. These are implementation-readiness findings, not requests to reopen the owner's product decisions.

Reviewed the spec and plan at `7fd9e25`, with the project guidance, handover, and relevant current Rust/TypeScript/build code. No original files or application code were changed. The only repository addition is this feedback file. I also ran Vite with `write: false` to inspect the actual generated manifest without writing a build. I did not run the test suites or change any live deployment. Platform references below support the platform-dependent findings; the proposed Access configuration has not been tested live by this review.

P1 means resolve before implementing the affected mechanism; P2 means an important correction to the execution plan or acceptance tests.

**1. [P1] Vite's manifest is not a complete inventory of this application's offline assets.**

Location: [Task 6, completeness test and generator](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:588).

This is measured, not hypothetical. At this HEAD, an in-memory Vite build with `build.manifest: true` emitted a manifest containing only `index.html → assets/index-BEX0dlqy.js`, with `assets/index-CEXGGp1D.css` in its `css` array. The build output also contained `assets/worker-C542ORIG.js` and `assets/yokohama_wasm_bg-BJZwkgIr.wasm`; **neither appeared anywhere in the manifest**. The proposed test iterates only each manifest entry's `.file`, so it also misses the CSS array.

A generator implementing that test can report complete coverage while omitting the entire local game engine. Fix the inventory to cover the actual emitted runtime files, including worker outputs, nested assets and CSS, plus the separately served art and static extras. A Vite output hook or a checked enumeration of the finished `dist` tree can supply what the manifest omits. Keep a test over a real build's output, not only synthetic manifests. Demonstrate that the generated cache contains the actual worker, `.wasm` and stylesheet before declaring readiness.

**2. [P1] Five separate Access applications do not establish the promised single `/enter` authentication flow.**

Locations: [spec turnstile](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:184), [Task 4](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:449).

The spec treats obtaining a hostname cookie at `/enter` as authorization for all four other applications. Cloudflare distinguishes the global SSO token from application tokens, and an application token has that application's audience. Sharing an allow-list does not make five applications one audience. The documented mechanism for pre-establishing access across destinations is a **single application with multiple destinations**, including its cookie settings. [Cloudflare authorization cookies](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/), [application token fields](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/application-token/).

The likely failure is a successful `/enter` navigation followed by API requests that still need another application's authentication redirect—or return the deliberately requested 401. Repeating `/enter` would not fix the missing API application token. Use one Access application covering the required paths, or explicitly design and verify how every application token is obtained. Do not accept only logged-out 302/401 readings as evidence: from a clean browser, authenticate **once** through `/enter`, then exercise games, both streams, chat and archive without navigating separately to each prefix.

Also reverse Task 4's exposure order: create the protection before activating the new public ingress. As written, Step 2 exposes a `claimed` server on the new hostname before Step 3 protects it. Verify exact and nested game paths, not just `/api/games`. For the supported `cf-access` mode, explicitly verify `/api/me`: it is currently classified as public, but the client relies on it receiving the trusted identity header. A cookie's presence on an unprotected request is not itself proof that Access injected that header.

**3. [P1] The multiplayer switch conflates HTTP exposure with the local dispatcher's gameplay routes.**

Locations: [Task 1 route filtering](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:183), [the wasm host must report false](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:205).

The plan introduces `Host.multiplayer`, removes game routes from the layer's table when false, and requires the wasm host to report false. But the local game works by calling those same `/api/games` routes through `api::dispatch`. Passing that false through the proposed filter removes create, list, input, replay and other local operations. The corpus replay will catch this, but the interface needs resolving before execution.

Separate the server's route-exposure policy from the local host's gameplay capability and reported deployment facts. Preserve local dispatch while proving that the HTTP server with multiplayer disabled serves none of the forbidden routes. Include the three host-owned transport routes: [the axum router](/Users/jmoore/Documents/Github/yokohama-assistant/crates/yokohama-web/src/lib.rs:728) registers `events` and both streams outside `api::routes()`, so changing the layer's table alone cannot remove them. Sweep actual methods and every route pattern; the five sample GETs are not a complete switch test.

**4. [P1] The door cannot discover its deployment configuration under the proposed zero-request rule, and remembered facts never expire.**

Locations: [spec lazy health reads](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:229), [Task 2 remembered facts](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:344), [Task 3 interface](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:376).

On a fresh profile there is no remembered `Health.multiplayer`. The door must both make no API request and know whether to show `/enter`; neither document gives it a source for that boolean. Choosing either default produces the wrong door on one supported deployment. Provide public static deployment metadata, an explicit build configuration, or another concrete mechanism consistent with the static-only goal.

The spec says facts can become stale, but the plan implements “asks once, remembers, and never asks again,” with no timestamp, release/configuration key or invalidation. A browser that once saw `archive: off` would continue authorizing plain deletion after the deployment enables the archive. Identity and chat configuration can also change. Define freshness and revalidation, especially before destructive Abandon decisions, and test a deployment change across reload.

Task 2 also misses an existing request: [App calls `useMe()` unconditionally](/Users/jmoore/Documents/Github/yokohama-assistant/web/src/App.tsx:142), and [that hook fetches `/api/me` on mount](/Users/jmoore/Documents/Github/yokohama-assistant/web/src/whoami.ts:50). Gate the identity hook by channel as part of removing all API calls from the door.

**5. [P1] A content-derived cache name does not verify that the installed bytes belong to that release.**

Locations: [spec install](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:264), [Task 6 install implementation](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:631).

The generator computes per-file digests, but installation checks only status and content type. An old art response from the HTTP cache is still a 200 WebP. The existing art server explicitly caches stable URLs for an hour. An install soon after a recut can therefore cache old art under the new release's digest and keep it indefinitely. A deployment changing between the script fetch and the `index.html` fetch can likewise cache a mismatched shell. `redirect: "error"` addresses neither case; Fetch's default cache mode can reuse a fresh HTTP-cache entry. [Fetch cache modes](https://developer.mozilla.org/en-US/docs/Web/API/Request/cache).

Verify each fetched body's digest against the build manifest, and specify how mutable URLs bypass/revalidate browser and edge caching. On a mismatch or failed install, retain the active generation and clean up the incomplete one. Add a stale-200-art test and a deployment-change-during-install test. The “matched by construction” sentence becomes defensible only once the served bytes are checked or all release resources have immutable, versioned URLs.

**6. [P1] Clicking update in one tab can switch another tab mid-game and delete the assets it still needs.**

Locations: [spec lifecycle](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:281), [Task 6 applyUpdate](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:635).

`skipWaiting()` is registration-wide. It can put the new worker in control of pages loaded with the old application, not just the tab whose player clicked. Deleting every old cache in `activate` then removes old hashed worker/module assets that another tab may still request. The proposed one-tab update cell cannot see this. The platform documentation explicitly identifies mixed-version clients as a consequence of `skipWaiting()`. [Service worker lifecycle](https://web.dev/articles/service-worker-lifecycle).

Define a multi-tab update policy: coordinate safe adoption across clients, or retain and route the assets required by old clients until they leave. Wait for the intended activation/controller change before reloading the clicking tab; posting a message and immediately reloading races activation. Exercise two tabs, including one with a local write in flight and one starting its local Worker after the other accepts an update.

Also narrow “an update never switches by itself”: a successfully installed waiting worker normally activates after all old controlled clients close, without a click. That is usually acceptable and safe; the spec should distinguish that case from changing an open game.

**7. [P1] Moving envelope serialization to the page does not make backup work when wasm is unavailable.**

Locations: [spec backup rationale](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:373), [Task 8 module-unavailable test](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:800).

The proposed test hands `buildEnvelope` record/table strings it already has, so it proves only that string wrapping needs no wasm. A freshly opened page does not already hold every game's documents. [localPair](/Users/jmoore/Documents/Github/yokohama-assistant/web/src/local/client.ts:343) sends a Worker message; [every message awaits wasm initialization](/Users/jmoore/Documents/Github/yokohama-assistant/web/src/local/worker.ts:465), and the pair handler reads the wasm store. The listing also depends on that Worker. If wasm fails to load, the proposed recovery path still fails before it gets the strings.

Add a page-accessible IndexedDB snapshot/export path independent of Worker initialization. Test the real backup control after preloading IndexedDB, reloading, and making wasm unavailable. Assert the downloaded document contains every stored pair byte-for-byte. Preserve a separate live-Worker snapshot path for an `APPLIED BUT NOT SAVED` game: reading only IndexedDB there would omit the very moves the recovery button is supposed to rescue.

**8. [P1] The backup format cannot represent all the saved states it promises to preserve.**

Locations: [spec envelope and compatibility](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:346), [Task 8 types](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:748).

`BackupGame.record` is a required string, and `check_pair` requires a replayable record. A real local game still at the initial deal has only its table document; [writeDocs explicitly supports that state](/Users/jmoore/Documents/Github/yokohama-assistant/web/src/local/db.ts:123). “Back up everything” must not silently omit it or fail the whole export. Give the envelope an explicit table-only/dealing representation and a corresponding Rust validation path, without loosening the archive route's record requirement.

For foreign revisions, the spec correctly adds the requirement that an unplayable game actually lists, but supplies no durable representation for that guarantee. The current [listing rejects unfamiliar table schemas](/Users/jmoore/Documents/Github/yokohama-assistant/crates/yokohama-web/src/tables.rs:1593). Keep the original bytes plus a durable compatibility report/index that does not require the current `TableDoc` parser to accept them. Define which structural checks remain possible without replaying a foreign revision, and distinguish preservation from successful validation. A backup containing several revisions also needs the envelope-level revision/hash fields defined as exporter provenance or replaced by per-game metadata; they cannot describe every enclosed game.

Acceptance cases: back up and restore before Keep, after a move, at game over, and with a genuinely unfamiliar table schema; reload and re-export the unplayable row without changing its bytes.

**9. [P2] Expired-session handling is only planned for one of three fetch paths.**

Locations: [Task 2 header/classifier](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:332), [spec claim that the stream is covered](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:662).

The plan modifies `call()` and `settle()`. But [readStream](/Users/jmoore/Documents/Github/yokohama-assistant/web/src/sse.ts:102) performs its own fetch and parses its own refusals; [recordText](/Users/jmoore/Documents/Github/yokohama-assistant/web/src/api.ts:446) does the same. Neither will acquire the new header or classifier from that change. Using fetch instead of EventSource makes the fix possible, but does not apply it automatically.

Share the server request headers and challenge classification across all three paths, while retaining the stream and raw-record response handling. Test an expired session on a stream reconnect and on Save game file, not just `api.health()`. Ensure an archive retry's 401 reports an outbox problem without unnecessarily discarding the local game screen or an otherwise valid claimed identity.

**10. [P2] Restore needs an atomic storage and identity contract, not just sequential clash examples.**

Locations: [spec clash rule](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:388), [Task 8 restore implementation](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:837).

Reading for absence and writing later is not sufficient across two tabs. The check and insertion of both documents must be one IndexedDB read/write transaction, coordinated with the existing ownership-generation rules. Treat a partially present pair as a conflict rather than an empty slot. Define duplicate IDs within one envelope and transaction failure/partial-batch reporting.

Bind the envelope ID to the table's ID, enforce the safe local ID namespace and the local player's admission requirements for playable imports. `TableDoc::agrees_with` compares setup fields, not the envelope ID or `local@browser` ownership. Otherwise an accepted import can be absent from “mine” or route to the server because its ID lacks `local-`.

After insertion, reconcile the existing Worker's resident store and refresh the listing without resetting an unrelated in-progress picker or losing unsaved state. Test restore into a running tab, concurrent same-ID restores in two tabs, and a clash raced against another tab's write. Sequential “run twice” cells do not prove those properties.

**11. [P2] A game finished offline before the first health answer is never enrolled in the outbox.**

Locations: [spec outbox entry](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:410), [Task 9](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:857).

A fresh visitor can install the shell without any health read, go offline, and finish their first game. The game-over health read fails, leaving the archive unknown. Entry requires a known archive and a failed upload, so no row is queued. Coming online only drains existing rows; this game's upload never happens. The headline outbox browser cell must include this fresh-profile scenario rather than pre-seeding facts.

Persist a pending discovery state, or explicitly rescan eligible finished games when archive discovery succeeds. Keep “archive unknown” distinct from “off.” Define a successful-upload receipt and what happens after a completed game is undone, deleted or changed while queued. The current two reasons also need a policy for permanent 400/409 refusals: those are neither offline nor signed out and should not produce misleading remedies or endless identical retries. Verify the actual automatic game-over hook; changing only the existing manual send handler will not provide it.

**12. [P2] The first IndexedDB version upgrade must handle an already-open old tab.**

Location: [Task 9 v1-to-v2 test](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:922).

The proposed test closes v1 before opening v2. Real M6.5b Workers hold their database connection for their lifetime, with no `onversionchange` close handler. A new v2 open is blocked by one such tab. The current `onblocked` rejects the promise, and the Worker caches its startup promise, so merely closing the old tab later does not establish that this tab recovers. This is the first cycle where that previously unreachable condition becomes ordinary. [IndexedDB upgrade rules](https://www.w3.org/TR/IndexedDB/).

Provide visible blocked-upgrade handling, safe recovery after the blocker closes, and `versionchange` handling for future upgrades. Test with v1 held open in another tab, then release it and recover without deleting site data. This needs an explicit strategy for old clients already shipped; adding a handler only to new code cannot retrofit them.

**13. [P2] The new build prerequisites and precache inputs are not wired into all build environments.**

Locations: [Task 5 build changes](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:483), [Task 6 generator integration](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-plan.md:554).

Three concrete execution failures need plan steps:

- CI runs `npm run build:wasm` but does not install Binaryen. Task 5's “no change needed; verify” on the workflow is insufficient for a newly mandatory executable. Install/verify the intended tool version in CI as well as Docker.
- The Docker client stage receives only `web/` and generated wasm. The art tree is copied only into the final runtime stage. A generator now running in `build:client` cannot hash the art files there. Give that stage the required art inputs and verify a clean image build.
- `npm run e2e` uses the old, unoptimized debug module and then `build:client`. The generator now enforces a release module ceiling on that debug build too. Its size is around 2.84 MB versus the intended optimized 1.17 MB. Define separate debug/release validation so e2e remains runnable without weakening the release budget or letting debug artifacts become a release cache.

Also note that Task 6 precedes Task 7's creation of the manifest and icons it says are mandatory precache entries. Adjust task ordering or state exactly how intermediate builds remain valid.

**14. [P2] Save-Data behavior and bounded installation are promises without an implementation path.**

Location: [spec final paragraph of §2.3](/Users/jmoore/Documents/Github/yokohama-assistant/docs/plans/2026-09-21-web-frontend-m6.5b2-offline-spec.md:309).

The spec promises that shell/module install while art waits for an explicit request under Save-Data. Task 6 always fetches the whole list, has no request/message protocol for deferred art, and exposes no “waiting for permission” readiness state. This also needs reconciling with “all-or-nothing release” and the readiness claim. Choose an explicit design: defer the entire precache, or model shell-ready separately from fully-offline-ready and define how the remaining assets join it. Test Save-Data and denied/deferred consent.

“Bounded” also needs an actual deadline and cancellation behavior: a request that hangs does not produce a failed status or reject the install promise. Define visible failure/retry behavior without disturbing the active release.

**Execution and test corrections to fold into the relevant tasks**

- `scripts/precache.test.ts` is outside [Vitest's `src/**` include](/Users/jmoore/Documents/Github/yokohama-assistant/web/vitest.config.ts:26) and outside the node TypeScript project's include list. Merely passing its path to the existing Vitest command does not expand that include. Wire it into a real test/typecheck project.
- The pure `swRoute.ts` tests must exercise the routing code actually embedded in `sw-template.js`. The plan specifies placeholder substitution for list/name only; it does not specify how the template consumes the tested function. Avoid a tested model beside a separate untested implementation.
- Add `.webmanifest → application/manifest+json` to the server's content-type map. It currently falls through to `application/octet-stream`; the proposed installer checks content types, so this belongs in its contract.
- Not registering `/enter` is insufficient to make it absent: the current extensionless SPA fallback serves `index.html` at that path. Explicitly exclude it when disabled, and test with a real built `dist`, not only an empty test directory.
- Task 4's curl loop checks only two of five protected prefixes and prints no `WWW-Authenticate` header despite saying it verifies that header. Use a real hashed asset instead of `/assets/`, which can be answered by the SPA fallback and falsely look successful. Include positive authenticated and negative unauthenticated checks for every protected family.
- Task 8's sample string replacements do not match the committed pretty-printed fixtures: they contain `"players": 2` and `"semantics_rev": 7`, with spaces. Construct malformed variants through Rust parsing or assert that each mutation changed its input before asserting its verdict. The tests also use envelope IDs different from the pair's stored ID; reconcile those fixtures with the import identity contract.
- Put the envelope fixture outside `web/src/wire/`, since it intentionally contains the seed. The spec asks the corpus generator to emit it, but Task 8 omits that step and the global constraint permits regeneration only at Task 1. Resolve that scheduling conflict without relaxing leakage checks.
- Preserve the archive route's refusal status, order and wording during validator extraction. A new `archive_would_accept` helper is not proof of the real route's behavior; compare actual dispatch responses for accepted and rejected inputs, including adjacent-pair refusal cases.
- The spec's §9 says §6.3 now names `launchPersistentContext`, but §6.3 still says storage state. The plan correctly uses a persistent context; make the authority agree. Likewise §9 claims §2.7 includes build-site/debug-profile/scanner details that are only in the plan.
- Task 11 calls the battery “eleven mutations,” while spec §6.5 enumerates fifteen. Enumerate and map the actual list to named catchers. Add an explicit whole-branch review/fix/re-review step before the battery's “fix-wave head”; the current plan references that head without scheduling its creation.

The strongest parts to preserve are the single Rust validation authority, opaque document strings, explicit unknown/off/on archive distinction, persistent-context offline test, and the engine/CLI fences. Once the findings above are resolved, those choices provide a good basis for implementation. The correction should be to make the mechanisms and their evidence match those promises, rather than expanding the cycle into the deferred hosting or engine work.
