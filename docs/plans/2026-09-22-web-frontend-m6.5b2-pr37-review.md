# PR #37 review — M6.5b2 readiness for M6.5c

Date: 2026-09-22. Reviewer: Codex, using the `review-agent` skill.

Reviewed [PR #37](https://github.com/jmoore2333/yokohama-assistant/pull/37),
“M6.5b2: the front door, installable and offline”, at head `672c604` on
`web-room-m6.5b2`, against merge base
`369a1f275584927203454d28d1084d81340eae32` with `web-room-m6.5b`.
Source line references below refer to that reviewed head.

The owner's request was a report of defects and testing needed before M6.5c,
not implementation. The review made no source changes. This document saves
that report for the orchestrating agent; it does not amend the frozen spec,
close record, or project rulings, and it does not authorize implementation.

## Findings

### F1 — [P1] Export the newest authoritative game snapshot

Location: `web/src/backup.ts:218–220`, `pairOf()`.

Backup prefers this tab's in-memory Worker snapshot over freshly read
IndexedDB data, even when another tab owns and advances the game. Each tab's
Worker can retain an older snapshot; a non-null response is not evidence
that it is current. This affects both backup paths that use `pairOf()`.
The independent null fallbacks for record and table can also combine documents
from different snapshots.

**Confirmed browser reproduction:**

1. Restore `web/src/wire-script/backup-envelope.json` in a fresh browser context.
2. In tab A, open `/#/t/local-at-the-deal` so the game is playing with zero events.
3. Open the door in tab B, allowing its Worker to load that snapshot.
4. In tab A, take the offered “End additional-action phase” move.
5. In tab B, download “Back up everything” and compare its record with IndexedDB.

Observed: IndexedDB contained **1 event**, while the downloaded backup contained
**0 events**. Both described a playing game. The export silently lost committed
progress from its recovery copy; the stored game itself remained intact.

**Required behavior / regression coverage:** select a coherent record/table pair
using ownership and freshness, while preserving export of the current owner's
unsaved moves. Cover two-tab export followed by restore, ownership takeover,
and unsaved-state recovery. Do not solve this simply by always ignoring memory:
that would discard the existing unsaved-recovery purpose.

### F2 — [P2] Serve retained release assets to tabs that did not reload

Location: `web/src/sw.ts:253–258`; related `fromCache()` at lines 268–275 and
`web/src/swRoute.ts`.

Accepting an update changes the service-worker controller across tabs, but the
page code deliberately reloads only the accepting tab. The new worker recognizes
only its own release's precached paths and reads only its own release cache.
Requests for older hashed assets therefore go to the network, even though the
previous release's cache is retained. An unreloaded tab that later restarts its
local Worker can fail offline, or after the old asset disappears from the origin.

**Confirmed browser reproduction:** bundle the actual `src/sw.ts` into two
minimal releases with different asset hashes; open two controlled tabs on
release 1; accept release 2 in one tab; take the browser offline; request a
release-1 asset from the unreloaded tab. Both release caches were present, and
direct inspection found the old asset, but the page's fetch failed with
`TypeError: Failed to fetch`.

This reproduces the routing defect using the real service-worker implementation;
it was not a full application upgrade/Worker-restart walkthrough. The existing
update test checks retention/sweeping with a single page and does not prove that
another tab can still fetch an old hash.

**Required behavior / regression coverage:** retained release assets must remain
usable by tabs continuing that release, without caching API or authentication
traffic. Exercise two tabs, genuinely changed hashes, update acceptance in one
tab, and a subsequent offline Worker start/restart in the other.

### F3 — [P2] Adopt the registration returned by a successful retry

Location: `web/src/swRegister.ts:276–278`.

When initial registration fails, `registration.current` remains null. “Try again”
calls `register()` but discards its successful result: it neither stores the
registration nor attaches the lifecycle listeners used by the initial success
path. The UI stays in the installing state, and update detection remains
disconnected until the page reloads.

**Confirmed browser reproduction:** serve the built application with the first
`/sw.js` request returning 503 and subsequent requests succeeding. Click
“Try again” after the initial registration error. The service worker reached
`activated`, but the door still displayed **“Preparing to play offline…”**.

**Required behavior / regression coverage:** a successful retry must adopt and
observe the registration just as initial success does. Test initial registration
failure followed by success, readiness reaching ready, and a later update offer.
Existing retry tests do not cover this initial-failure-to-success path.

## Assessment for the orchestrating agent

M6.5c planning can proceed, but carry F1–F3 as explicit prerequisites to calling
M6.5b2 stable. F1 is the highest priority because a successful-looking backup can
omit committed progress. F1 and F2 require corrections to snapshot selection and
update compatibility, rather than additional testing alone.

The review found no evidence requiring a redesign of the shared game engine.
The five fenced crates and CLI were unchanged against the merge base. This is
not a claim that the future hosted runtime or all deployment configurations have
been validated.

For M6.5c, the handover's architectural decisions remain essential: isolate game
state per object, persist the state needed after hibernation, and require a
successful archive receipt before deletion. Resolve those designs and validate
them early in the live runtime.

## Testing to prioritize before substantial M6.5c implementation

These are test gaps or carried handover items, not additional introduced-defect
findings. Authority for existing opening items remains
`docs/plans/2026-09-22-web-frontend-m6.5c-handover.md`, especially sections 4–5.

1. **Regression checks for F1–F3:** two-tab backup and restore with current and
   unsaved state; two-tab update with changed hashes and offline Worker restart;
   initial registration failure followed by successful retry.
2. **Physical phones:** iPhone installation, airplane-mode play, complete app
   closure, reopening/resuming, finishing/exporting, then update and restore.
   Repeat on an Android handset. The handover records that phone behavior was
   not measured in M6.5b2.
3. **Real browser lifecycle:** an old database-version tab blocking migration,
   recovery after closing that tab, and background suspension/resumption during
   lock takeover. The no-Web-Locks fallback remains an outstanding limitation
   to assess.
4. **Deployed UI and authentication:** the owed headed desktop and 390-pixel
   walkthrough of door, lobby backlink, backup/restore and refusal surfaces;
   a fresh browser logging in once through `/enter`, protected destinations
   working afterward, and usable session-expiry recovery for normal requests,
   streaming and downloads. Verify fail-closed identity handling before exposing
   a public door. Public access and art configuration remain owner decisions.
5. **Early M6.5c deployment spike:** authenticated WebSocket upgrade, rejection
   of client-supplied identity headers, WASM cold startup and randomness, and
   representative replay/bot/explanation/archive workloads inside the actual
   target runtime against the agreed budgets. Re-prove static fallback behavior
   for `/sw.js` and `/enter` on the new host.

## Validation and limits

Checks run during this review:

| Check | Result |
| --- | --- |
| `npm test -- --reporter=dot` | 988 passed across 54 files |
| `cargo test -p yokohama-web --lib` | 275 passed |
| `npx vitest run -c vitest.node.config.ts src/sw.node.test.ts src/backup.node.test.ts src/local/corpus.node.test.ts` | 16 passed across 3 files |
| `npm run typecheck` | Passed |
| Workflow YAML parse | Passed |
| Five fenced crates and CLI diff | Unchanged |
| Targeted isolated Chromium reproductions | F1, F2 and F3 reproduced |

The full end-to-end suite, physical devices, live deployment, Docker rebuild,
and a fresh WASM build were not run. Application reproductions used the existing
built application; the service-worker reproduction bundled the actual source.
Passing suites do not establish coverage of the three reproduced failures.

Temporary reproduction scripts were created outside the repository at
`/tmp/yokohama-pr37-backup.cjs`, `/tmp/yokohama-pr37-sw.cjs`, and
`/tmp/yokohama-pr37-retry.cjs`. They are session-local aids and may disappear;
the scenarios and observed results above are the durable evidence summary.

No fixes, commits, pushes, or GitHub review comments were made. The only
repository change requested after the review is this report.
