# Security S3 — the hosts, the operations and the install-failure path: design (2026-10-19)

> The third cycle of the 2026-10-08 security remediation (`docs/plans/2026-10-08-security-remediation-spec.md` §2.3,
> §3.4), brainstormed with the owner on 2026-10-19 after the auth cycle's stage 1 went live, and **stacked on the auth
> cycle's branch** (`security-s3` cut from `security-auth` at `4ce7493b`; the spec's §9.2 stacked-branch rule as its
> §9.7 closing paragraph reads it: S1 → S2 → the auth cycle → S3). It binds the plan that follows it
> (`docs/plans/2026-10-19-security-s3-plan.md`, by the project's brainstorm → spec → plan workflow) and the records S3
> closes. It is the owner's rulings (§1), the mechanisms as measured (§3), the orchestrator's decisions (§4) and what is
> left to the builder (§6). The findings' ids and neutral titles are the ledger's (`docs/security-ledger.md`); the
> report with their exploit detail stays gitignored and is never cited by content here.

## 0. Read order and sources

1. `docs/security-ledger.md` — the rows this cycle moves: B11, D2, D4, D9, D17, D19, D21, D22, D30, D31, D32, and the
   AUTH row's passkeys note.
2. `docs/plans/2026-10-08-security-remediation-spec.md` — §2.3 "S3 — the hosts and operations", §2.4, §3.4, §3.5, §7,
   and §9.7's closing paragraphs ("S4's identity input, revised", "The order from here, and the auth cycle's place").
3. `docs/web-lane.md` — §8 "The public door's preconditions" (the W-3 row of its table and the "Still owed" paragraphs),
   §15 "What is by design" (the W-3 bullet, lines 58–64 of the section), §7 "The release-version rule", §2 (the suites).
4. `worker/RUNBOOK.md` §20 (cycle F's window: W-1, W-2 explained, the no-click update) and §23–§25 (the auth cycle's
   stage 1, the session cap, the red button).
5. `docs/plans/2026-10-18-auth-cycle-design.md` — R-A5 (one model per deployment; Access stays; the private Worker),
   R-A12/D-1 (`yokohama-web` itself is the public deployment), §4.7's deploy-guard identities.
6. The code the five pieces touch, cited by file and line in §3.

## 1. The owner's rulings (2026-10-19), and what each binds

Quoted as given; each ruling names the rows or the mechanism it decides.

- **R-S3-1 — the Access-shaped rows are dismissed.** "the hostname leaves access when we set the repo to public, so I
  think the access shaping is something we can dismiss as not necessary - when we setup a new worker for private use,
  yokohama-test.moore.nyc as an example, that will be access protected but we can keep the same rules for the
  cli/nas/test worker because it would be for my own internal dev use only. The public worker will be open and the new
  identity layer will protect the multiplayer side to an extent." → **B11** and **D22** read `dismissed` in the
  ledger, in these words: one Access application and one audience across the owner's private hostnames (the NAS, the
  terminal, the test Worker) is the intended shape for an internal development population; the public Worker leaves
  Access on the public day and carries no Access-shaped exposure. The spec's R-SEC4 ("the application split is
  deferred hardening") closes as dismissed rather than deferred.
- **R-S3-2 — the NAS operations rows are the private host's.** "The NAS operations I'm less concerned because it will
  be private, so changes there would need to be for the benefit of the public movement." → **D19, D21, D30, D31** read
  `recorded` — left as the private host's, with the sentence that a change there is made only where it serves the
  public movement. None does: the build context (D19), the metrics bind (D21), the stopped container's token (D30)
  and the LAN-published services (D31) are reachable only from the owner's own network.
- **R-S3-3 — CI waits for the public move.** "The CI I also think we can wait until we move it over to public or just
  before hand" → **D17** moves to **S4**, its cycle column amended, as the "just before public" step: read-only
  permissions, SHA-pinned actions, a `v*` tag ruleset, a protected publish environment, and the fork-pull-request
  trigger question a public repository raises.
- **R-S3-4 — D32 is the transition's cost.** "the per ip bound it seems to be acceptable for the transition cost, the
  system is mostly not being used, so I don't see an issue here. It's mostly dark as this security work wraps and we
  prepare to go public." → **D32** reads `accepted`: the previous release's client storms `/api/me` until the door
  applies the release; the population is the Access allow-list until the public day; no per-IP bound on caller-less
  reads is built.
- **R-S3-5 — D2, D4 and D9 are code, before the public day.** "D2,D4,D9 we should layer in before we get to public."
  → the three code pieces of §3.2–§3.4.
- **R-S3-6 — passkeys are dismissed.** "passkeys we'll dismiss since we're not using a custom domain with workos." →
  the board's ruling 11 closes; the AUTH row's note reads `dismissed` with that reason; no enrolment path is built.
- **R-S3-7 — the test Worker is deployed.** "We can deploy the test worker just so we have it handy for testing but
  otherwise we're ready to move it forward in this cycle with these items." → §3.5: `yokohama-web-private` at
  `yokohama-test.moore.nyc`, under Access, this cycle.
- **R-S3-8 — Bot Fight Mode stays on; the install-failure path is S3's W-3 answer.** "I want to keep bot fight mode
  on, so we can turn it off for testing purposes but ultimately we want to keep it online - so coming up with an
  install-failure path would be the solution to develop here." → §3.1. The lane's three candidates (§15: Bot Fight
  Mode off, a clearance-refreshing read, the precache's extensions) resolve to the second, in the shape §3.1 gives it;
  the first is a TESTING switch only, never the operating state; the third is not built.
- **R-S3-9 — D9 takes both shapes, and the close exists.** "The creator's kick and the keep timeout on its own would be
  ideal although realistically I don't see someone setting up a game and waiting 90 days for someone to start it - but
  a creator could just kick someone or close the table out, and having tables expire if they have't started would
  address the issue." → §3.2: a seat the creator reopens, a table that has not started expiring on the waiting-table
  TTL, and the existing Abandon as the close.
- **R-S3-10 — D4 is the allow-list with its note.** "For D4 provided we note in the docs/docker build file what's
  required there I think that takes care of it, agreed on this" → §3.4: the allow-list ships with the variable
  documented in `deploy/README.md` and noted in the image's Dockerfile; the NAS's stack sets it.
- **R-S3-11 — the public day is one day.** "the hostname leaves access when we set the repo to public" → stage 2 of
  the auth cycle's window (`worker/RUNBOOK.md` §23: the keyed build, the Turnstile on, the edge rule enabled, the
  hostname off Access) is the public day's, after S4's repository work; S3 and the auth cycle merge before it. The
  live reading of §3.1 therefore belongs to that window; §3.1's code is proven by its cells and by a reading with Bot
  Fight Mode switched off for the test (R-S3-8's "for testing purposes") or on a real browser whose cookies are cleared,
  whichever the plan can reach before the public day.
- **R-S3-12 — the Access destination over the API, with a skip.** During the brainstorm Cloudflare's Zero Trust
  services were in a major outage and the owner could not add the test hostname to the Access application: "You can
  add that over the API or attempt to do that, when we get to it in the plan, if you get errors over the API, you can
  skip the deploy of the test worker and continue through the S3 and we'll get to that step afterwards - services
  should be working but changes don't seem possible at the moment." → §3.5's first step is the orchestrator's, over the
  Cloudflare API; a refused or failed change SKIPS the test Worker's deploy as a typed, recorded deferral and S3
  continues; the deploy is taken up after the close, by the same recipe.
- **R-S3-13 — the reopen is at `Open` only; a dealing table restarts** (2026-10-20, the builder check's question 1,
  §7.1). "I agree reopen at open only, otherwise if it made to starting the game they have to restart - just like if
  it stalled just a moment after that." → §3.2 rule 1 reopens a seat while the table is `Open` and refuses at
  `Dealing`; the creator's remedy at `Dealing` is Abandon and a new table, exactly as for a game that stalled a moment
  after it started, and the waiting-table TTL ends the table otherwise (rule 2). No seed is re-drawn and no leak is
  recorded: a seat whose cards were served is never reopened.
- **R-S3-14 — the `Host` check has an enable switch, on in every committed deploy file** (2026-10-20, after the plan's
  hand-over; a side agent's note that a NAS deploy reusing today's create body would answer every tunnelled request
  421 while the loopback smoke still passed). "I think if we are going this far we should have a second variable for
  YOKOHAMA_WEB_HOSTNAMES_ENABLED and have a boolean value there and then if it's enabled which we will by default in
  our file to be set in the public repo/our demo files we have it on but some people may not care about those
  boundries internally." → §3.4: a second variable, `YOKOHAMA_WEB_HOSTNAMES_ENABLED`, `on` or `off` in the layer's
  own convention (`Multiplayer::from_env_value`: absent or `on` is on, `off` is off, anything else refuses to start by
  name); every committed deploy file and example sets it `on` beside the list; a self-hoster who does not want the
  boundary sets `off` and the binary answers any `Host`, saying so once at start. And the catcher the note asked for:
  the smoke takes the hostname and probes with it, and the NAS recipe's reading through the tunnel is a REQUIRED step
  before a deploy is called done.
- **R-S3-15 — the window is the plan's last task, programmatic, waiting on nobody** (2026-10-20). "the NAS deploy will
  need to be layered at the very end of the plan, so it doesn't get hung waiting for me, unless it can be done over
  portainer and configured programatically" → the plan's Task 10: `0.7.2` on the NAS first, then the Worker, after the
  Fable review's fix round and the PR, driven from the workstation as S2's NAS window was (`deploy/RUNBOOK.md` §3.22:
  the context served on the LAN, the image built and the container swapped through Portainer's Docker proxy, every
  reading over WARP or by an exec inside a container); a host the orchestrator cannot reach is a typed, recorded skip
  that the owner's checklist carries. **And R-S3-12's step is DONE:** the owner added `yokohama-test.moore.nyc` to the
  Access application once the outage lifted ("yokohama-test was taken by the access policy so that is now done, the
  pass for the worker can be handled in the plan"), read back over the API at 04:41:55Z in both `self_hosted_domains`
  and `destinations` under the private file's `aud`; Task 7's first step is a read-back, and the private Worker's
  deploy runs in the plan.

## 2. Intent, goals and fences

**Intent.** Close the security series' third cycle so that S4 — the repository's public preparation — starts from a
tree with nothing of the review left undecided: the three rows the owner wants in the code before a stranger can
reach the site (D2, D4, D9), the one lane finding the public door still owed a mechanism for (W-3), the private test
Worker the owner wants at hand, and every other row recorded in the owner's words.

**Goals.**
1. A returning browser whose clearance lapsed installs a new release without a click, with Bot Fight Mode on.
2. A creator is never held by a joiner who takes a seat and does not keep: the seat reopens on the creator's word, and
   a table that has not started expires on the waiting-table TTL on both hosts.
3. The NAS binary throttles its own password oracle and answers only the host names it is told to.
4. `yokohama-web-private` runs at `yokohama-test.moore.nyc` under Access, the one application, deployable by the guarded
   script.
5. The ledger, the spec's review record, the lane's §8 and §15, the board and guidingpath say what S3 decided.

**Non-goals.** A ban list or a persistent block of a kicked player (under the public door a name is a verified
address; a block is a product feature, not this cycle's). A minimum password length. A `Host` check on the Worker (the
shell is not the binary and the wasm layer never reads `Host`). An Access application split (R-S3-1). Any change to the
NAS operations rows (R-S3-2). A per-IP bound on caller-less reads (R-S3-4). A passkey enrolment path (R-S3-6). The
precache's extensions renamed (R-S3-8). A second verifier arm, a second provider, or any change to the session cookie.

**Anti-goals** (what would make this cycle a failure even if it works). A widget that opens without a person's click
more than once per install attempt (a loop against the edge). A single new Worker invocation on the front door's
install or update path (the owner's push for static assets; §3.1 rule 7). A reopen that hands the next joiner a seat whose cards
a kicked player has seen. A limiter one caller can use to lock another out of a table. A `Host` check that refuses the
tunnel's own requests at the NAS because its default was wrong. A test Worker that answers a stranger for the minutes
between its route going live and Access covering the hostname. Exploit detail in the tree.

**Fences, unchanged.** The layer ROUTES, SCOPES and SERIALIZES and computes nothing about the game (`docs/web-lane.md`
§4). `SEMANTICS_REV` is not touched. No new `#[ignore]`d test (the roster rule in `CLAUDE.md`). The ledger's titles stay
neutral. The private Worker's door never flips (R-SEC6; `worker/RUNBOOK.md` §3). The hidden-information fence (R-SEC1)
governs §3.2's reopen rule.

## 3. The five pieces, as the code stands

### 3.1 W-3 — the install-failure path (the client; `web/`)

**As measured.** Under the no-Access door with Bot Fight Mode on, a controlled browser whose `cf_clearance` has lapsed
cannot install a new release: Bot Fight Mode challenges the precache's fetches of the allowed paths that are not static
resources by extension (`/art/manifest.json` among them), and the door's "Try again" refetches with no clearance.
`docs/web-lane.md` §15's W-3 bullet; §8's W-3 row; `worker/RUNBOOK.md` §20's "THE PINNED READING and W-2, EXPLAINED"
(the firewall events read `botFight managed_challenge` on `/art/manifest.json` one second after each of four
rechecks). What restores a clearance (§8's row): a metered call meeting the rule's challenge, whose widget sets one, or
a navigation the edge answers itself. The readings were taken on the pinned instrument, which holds no clearance by
design; a real browser with a lapsed clearance is unread.

**The code today.**
- `web/src/sw.ts` `installOne` (lines 197–227): each precache entry is fetched with `cache: "reload"`,
  `redirect: "error"`, `credentials: "same-origin"`; `checkHead` turns a wrong status or type into a thrown sentence;
  the throw names the path (`${entry.path}: ${messageOf(error)}`).
- `web/src/sw.ts` `tellTheClients` (lines 248–256): on a failed install the worker posts
  `{ yokohama: "install-failed", reason }` to every window, uncontrolled ones included, "so the door can NAME the
  failure and offer a retry".
- `web/src/swRegister.ts` (lines 266–272): the page reads that message into `reason` and sets readiness `failed`; its
  retry (lines 396–440) REGISTERS anew when the first install failed, because "the platform has marked the worker
  redundant, emptied the registration and removed it from the registration map, and `update()` on it is refused —
  measured in Chromium".
- `web/src/turnstile.ts`: `setChallengeHandler` (line 173) mounts the overlay's handler, which a keyless build never
  does; `passChallenge` (lines 186–214) runs the widget once in the overlay, rejects with `EDGE_CHECK` when no handler
  is mounted and with `EDGE_CHECK_REPEATED` when the check did not pass, and hands every challenge that arrives during
  a pass the SAME pass; `isEdgeChallenge` (lines 233–235) is the predicate: status 403 AND `cf-mitigated: challenge`,
  "readable by a same-origin fetch".

**The design.** The service worker names the edge's challenge; the door passes it and retries on its own.

1. In `installOne`, a response for which `isEdgeChallenge` holds (the predicate restated in the worker, which cannot
   import the page's module; the two kept equal by a cell) is a typed failure, distinct from every other: the install
   still FAILS (the platform's redundant-worker path is what the retry relies on), and the message the worker posts
   carries a `kind` beside the `reason`: `kind: "challenged"` for this case, `kind: "failed"` for every other. The
   `reason` text stays a sentence naming the path, so every reader of today's message reads what it read.
2. **One tab, and the door only.** The worker posts `install-failed` to every window (`sw.ts:248–256`), and
   `useReadiness` is mounted app-wide (`web/src/App.tsx:184–194`), so without an election N open tabs would open N
   widgets for one install — the anti-goal. The worker therefore ELECTS one window to carry `kind: "challenged"`: a
   focused window showing the door, else a visible one showing the door, else any window showing the door (the
   predicate on `WindowClient.url` is the builder's; the door is the route cycle F gave the auto-apply to,
   `useDoorAutoApply`, `App.tsx:200`); every other window receives the same message with `kind: "failed"`. With no door
   open anywhere, every window receives `failed` and the pass waits for a recheck that finds one — a game screen never
   gets an overlay in the middle of play, and a browser sitting on a game plays on from its retained generation.
3. The door, on `install-failed` with `kind: "challenged"`, calls `passChallenge()`:
   - when it resolves (the overlay's widget passed; the clearance is set), the door runs its existing retry
     (`swRegister.ts:396–440`): `update()` on the registration when a worker is active — the returning browser's own
     case, the one W-3 is about — and `register` anew only when the FIRST install failed and the platform emptied the
     registration; the install proceeds under the clearance;
   - when it rejects (`EDGE_CHECK`: no overlay, the keyless build; `EDGE_CHECK_REPEATED`: the check did not pass), the
     door lands exactly where it lands today: readiness `failed`, the reason drawn, "Try again" offered.
4. **One automatic pass per PAGE, and never a second without a click.** The page starts install attempts on its own —
   `update()` every `RECHECK_MS` (15 minutes) and on every `visibilitychange` to visible (`swRegister.ts:316–334`) — so a
   counter per install attempt would reopen the overlay on every recheck for a person who cancelled it: the loop the
   anti-goal forbids. The counter is the page's: after one automatic pass, a challenged install on that page draws the
   button and nothing else; a click on "Try again" is a person's act and its retry, if challenged, may run the widget
   once more; a fresh page load starts over.
5. The overlay's words are the ones a link arrival already shows ("Checking that this is a person…"); nothing new is
   written for the eye.
6. A page must be open for the pass to run; the message reaches no one otherwise, and the install fails as today, the
   next page open registering anew. That is the shape already in place and it is kept.
7. **The path costs NO Worker invocation — the owner's rule for the front door (2026-10-19: "we just want to make sure
   we're not adding metered invocations on the home page that was our push for static assets").** The overlay's widget
   posts to the edge's own challenge platform (`/cdn-cgi/challenge-platform/…`, answered by the edge: cycle F's link
   arrival, `worker/RUNBOOK.md` §20), the clearance is a cookie the edge sets, and the retry refetches the precache's
   entries, which the asset layer serves at zero invocations (`run_worker_first` names `/api/*` and `/enter` only;
   F's window read every static path and every prefix miss at 0). Nothing in this path requests anything under `/api`;
   in particular the lane's second candidate in its "clearance-refreshing READ" form — a metered call made on purpose
   to meet the rule's challenge — is NOT what is built, since that read would be an invocation. A cell pins the rule
   (the door's automatic pass and retry make no request whose path starts with `/api`), and the live reading counts the
   minute's invocations as F's window did, expecting the pass and the install to add none.

**What proves it.** Unit cells on the worker's install (a fake `fetch` answering 403 with `cf-mitigated: challenge`
on one entry: the elected window's message `challenged` and every other window's `failed`, with the reason; a 403
WITHOUT the header: `kind: "failed"` to all; a 200: no message; the election's order with focused, visible and
unfocused door windows and with no door open), on the door (a `challenged` message with a handler mounted: one
`passChallenge` call then the retry, on the `update()` branch with an active worker and on the `register` branch
without one; the handler rejecting: `failed` with the reason and no retry; no handler: `failed` at once; a second
`challenged` on the same page after an automatic pass: the button, no second pass; a recheck's challenged install after
the pass: silent; the whole path making no request under `/api`), and one e2e cell: there is no edge stub in
`web/e2e/` today, and a new `context.route` on the precache's fetch (the mechanism `offline.spec.ts:36–38` already
uses) answers one entry with the challenge shape — which, since the e2e client is built KEYLESS by design
(`web/scripts/e2e-build.mjs`, so no overlay mounts and `passChallenge` rejects at once), proves the typed `kind`
reaching the door and the keyless landing on `failed`, and NOT the pass. **The success path has no end-to-end proof
before the public day, and the close says so:** a keyed client starts under no door the test Worker or the e2e can
offer (the guard's check 12; §3.5), and no harness can stand in for Bot Fight Mode. Its proof is the door's unit cells
plus the live reading at the public day's window: a real browser holding the previous release, its cookies cleared, a
release deployed, a reload — the overlay under a second and "Ready to play offline" with no click, and the minute's
invocation count unmoved (R-S3-8, R-S3-11). A keyed e2e build (the NAS binary under `open` with Turnstile's testing
site key) is recorded as a harness piece a later cycle may add, not this one's.

### 3.2 D9 — the seat the creator reopens, and the table that has not started

**As the code stands.**
- `crates/yokohama-web/src/table.rs:78–92`: a table is `Open`, `Dealing` or `Playing`; "`Open` is M6.2's and is the
  one status with a SEAT invariant beside it: a table is open exactly while it has an open seat, both directions
  refused in `tables::check_shape`".
- `crates/yokohama-web/src/tables.rs:1692–1693`: a join that fills the last seat sets `TableStatus::Dealing`. No route
  reverses it (the review's R1B-10, carried by the ledger's D9 row).
- `table.rs:247–250`: `keeps: Vec<Option<u8>>` — `1` or `2` once a HUMAN seat has chosen, a bot seat's `None`; never on
  the wire (`table.rs:266`); the wire carries a boolean `chosen` per seat instead (`table.rs:20`, `:464`).
  `all_humans_kept` (`table.rs:350–353`): every human seat has chosen.
- `tables.rs:1447–1460` `settle`: self-repair of a table stuck at `dealing` with every human keep present (a crash
  between the last keep's two writes).
- `tables.rs:1972` `Games::delete`: creator-only ("only the table's creator can delete this table"), at ANY status, for
  a creator who HOLDS A SEAT — the function refuses any caller with no seat first (`tables.rs:1981–1983`), the create
  path requires the creator to take one (R-1.6, `tables.rs:225–227`, `:276–279`), and both expiry paths delete AS the
  creator (`sweep.rs:259`; `lifecycle.ts:146–148`). This is the client's "Abandon". So the owner's "close the table
  out" EXISTS; it is verified, not built — and it is why a creator's own seats are never reopened (rule 1).
- The TTLs: on the Worker `worker/src/table/lifecycle.ts:382–387` `waiting(id)` reads the house copy's `status` and is
  true only for `"open"`; `worker/src/marker.ts:125–129` `expiryReason` returns `waited` at the open-table TTL
  (`worker/src/config.ts:299`: 7 days) for a waiting table and `expired` at the table TTL (90 days) otherwise. On the
  NAS `crates/yokohama-web/src/sweep.rs:171–172`: `Waited` for `TableStatus::Open` past the open days. A DEALING table
  is therefore waited for by neither host; it waits the 90-day TTL on the Worker and the NAS's sweep's long TTL.
- The Worker shell never parses the table document, which carries the seed (`marker.ts:135–136`); `waiting` reads the
  house COPY's facts (`lifecycle.ts:606–613` `factsOf`: `created_by`, `archived_at`, `status`).

**The design, three rules.**

1. **The reopen.** `POST /api/games/{id}/reopen` with the body `{ "seat": n }` — the `Keep`/`Join` body's shape, the
   seat parsed through `PlayerId::try_new` as those two do (`api.rs:1281–1293`). The rule, held under the table's own
   lock as every table writer is:
   - the caller is the table's creator, or 403 with the delete's own sentence pattern ("only the table's creator can
     reopen a seat");
   - the table is `Open` — **and only `Open`** (R-S3-13). At `Dealing` 409 ("the deal has begun — abandon the table and
     set it again"): the moment a table fills, `GET …/deal` serves EVERY seated caller their two cards from the seed
     (`tables.rs:1565–1617`), chosen or not, and the dealing screen fetches them on mount (`web/src/Dealing.tsx:178–198`);
     a seat reopened then would hand the next joiner cards the kicked person has seen — **the hidden-information fence**
     (R-SEC1), and the anti-goal. At `Playing` 409 ("the game has started — a seat at play is a player"). At `Open` no
     deal has been served to anyone, so nothing a kicked person saw survives their seat.
   - the seat is a HUMAN seat (a joiner's, or one the creator bound at create for a named person); a bot seat and an
     open seat are 409 (nothing to reopen). At `Open` no seat has chosen, by construction (keeps are made at `Dealing`),
     so the rule needs no reading of `keeps`.
   - the write: the seat becomes `SeatDoc::Open`, the status stays `Open` (the seat invariant holds on both sides of
     the write), the signal bumps. **The creator's own seats are NEVER reopened** — 409 ("you
     cannot reopen your own seat; abandon the table instead"): the create path makes the creator take a seat (R-1.6),
     `Games::delete` refuses a seatless caller before it reads the creator, both expiry paths delete as the creator, and
     at `Dealing` a seatless creator is a stranger to `table_for_anyone` (`tables.rs:1380–1387`) — a creator with no
     seat could neither Abandon nor be expired nor see the roster. The hot-seat creator's extra seats fall under the same
     refusal; the simplicity is worth the edge case.
   - afterwards the kicked person is what anyone is at an open table: a would-be joiner — they may join again at
     once. The creator's remedies for a repeat are the table's password and Abandon; a ban list is a non-goal (§2).
   - the Worker: the shell's route tables for a table's paths (the review's "gameplay allowlist", held) gain the route;
     the write limiter counts it as it counts every POST; `access-paths.mjs` lists the `/api/games` prefix and needs no
     change (the builder confirms its drift cell).
   - the client: the creator's roster at `Open` draws a "Reopen" control beside every human seat that is not the
     creator's own; the control posts the route and re-reads the table; a refusal's sentence is drawn where the join
     form draws one. The dealing screen draws no such control: the creator's control there is Abandon (rule 3), and the
     players still seated at an open table see the seat reopen through the table's signal as they see a join today —
     the `Dealing` → `Open` transition the builder check's question 7 asked about cannot arise.
2. **The expiry.** A table that has not started expires on the waiting-table TTL on both hosts: the Worker's
   `waiting(id)` reads `open` OR `dealing` from the copy's facts; the NAS's sweep reads `Dealing` beside `Open`, AFTER
   the read that repairs (`settle`), so a table that is really finalized and only un-flipped is repaired to `Playing`
   and kept. **On the Worker the two hosts part in that one corner, by design:** the alarm reads `waiting` from the
   stored, un-settled document before the delete (`lifecycle.ts:356–357`, `:382–386`), `settle` runs only inside the
   delete's `events` read, and `gameOf` keeps a table only when a move was made (`marker.ts:141–144`) — so a dealing
   table that was finalized-but-unflipped between the last keep's two writes, and that NOBODY read for the whole
   waiting TTL (any read would have repaired it), is ENDED as `waited` on the Worker: a game with no moves that no one
   opened in seven days. That loss is accepted over a shell change that settles before it reads (the strings rule's
   territory for a corner no reading has met); the Worker's cell pins the ending and the NAS's cell pins the repair, and
   the lane's §4 records the difference in one sentence. The knob's sentence
   changes from "a table still WAITING for players" to "a table that has not started" at every site that states it
   (`api.rs:71–78` and `:433–440`, `worker/src/env.ts:97–100`, `worker/src/config.ts:42–45`, `deploy/README.md`'s
   table, `docs/web-lane.md` where it says so); the figure stays the one knob, 7 days by default. The `waited` reason
   and the sweep's log line are unchanged in shape.
3. **The close.** Abandon as it stands. The builder verifies the creator's Abandon control is drawn on the dealing
   screen; if it is not, it is added there with the sentence the open table uses.

**What proves it.** Layer cells for every arm of the reopen (creator / not; `Open` / `Dealing` / `Playing`; a joiner's
seat / a bound seat / the creator's own / a bot / an open seat; the seat invariant after the write; the signal's bump;
the kicked person's admission afterwards as a would-be joiner; a reopen racing the join that fills the last seat,
under the lock — the join lands first, flips the table to `Dealing`, and the reopen is refused with the dealing
sentence, or the reopen lands first and the join lands on a reopened seat — both orders pinned); the Worker's route
and limiter cells; the Worker's `waiting` cell for a `dealing` copy and the expiry's `waited` reason, and its cell for
the unread corner (D-16); the NAS sweep's `Dealing` cell with the settle-first order pinned by a
finalized-but-unflipped fixture that is REPAIRED, not waited; the client's control cells (drawn for the creator at
`Open` beside the right seats and nowhere else); an e2e cell where a creator reopens a seat a second browser took at
an open table, and the second browser's table view reads it as a would-be joiner's.

### 3.3 D2 — the password oracle's throttle (the layer; both hosts)

**As the code stands.** `POST /api/games/{id}/join` with `Join.password` (`api.rs:1031`) and
`POST /api/games/{id}/unlock` with `Unlock.password` (`api.rs:1055`) compare a presented password with the table's
(`Games::join`, `Games::unlock` in `tables.rs`); nothing in the layer counts attempts. The Worker's hostname is covered
by the zone's one rate-limit rule, 20 requests per 10 s on `…/join` and `…/unlock` by IP (`docs/web-lane.md` §8, the
password-oracles row: "25 wrong-password joins read 20 × 403 then 5 × 429"); the NAS hostname is not in it and the NAS
stays behind Access, which is why the row is a Low: only a listed player reaches the oracle. The precedent for a
per-caller count in the layer is security S2c's stream caps, `crates/yokohama-web/src/streams.rs` (`STREAMS_PER_CALLER`
8, `STREAMS_PER_TABLE` 64; `Streams::admit`, a `Slot` the body holds; OPEN streams only).

**The design.** A failure counter in the layer, keyed by **(table, caller key)**, 20 failures per 10 seconds:
- the caller key is the door's identity for the request — the verified address under `cf-access` and `worker`, the
  claimed name under `open` — and under `open` that is a recorded limitation, not a defect: a name is free there (the
  same sentence security S2b recorded for the Worker's open door, finding C2), and the open door is the self-hoster's
  and the LAN's (S2b's R-2), a trusted network by its own rule; the README says so beside the variable;
- a WRONG password counts one; a correct one clears the entry; a request refused by the counter counts nothing; a
  refusal is 429 with one sentence that names the wait ("too many wrong passwords for this table — wait ten seconds
  and try again") and NO `Retry-After` header: the stream caps' 429 carries none either (`lib.rs:182–183`, "since the
  client ends a signal on it rather than waiting"), `ApiError` has no header field, and the Worker's wasm reply carries
  status, body, bumps, archive, picker, dropped and logs and no headers (`crates/yokohama-worker/src/lib.rs:220–243`),
  so a header would cost a new reply field, a getter and the shell reading it, for a wait the sentence already states;
- the check runs after the caller is known and the table is read, before the compare, and the count-then-write is one
  operation under the counter's own lock (the lesson of S1b: a count-then-write is a race until a lock says otherwise);
  an entry lives only while its window is open (pruned on the way out, as `Games::with_table_lock` prunes a lock entry),
  so the map is bounded by the live windows and never grows with history;
- the figure 20 per 10 s mirrors the zone rule so both hosts read alike and the lane's §8 sentence about the rule stays
  true as the Worker-side default; on the Worker the zone rule answers first by IP and the layer's counter is the
  second fence behind it (one caller's digest, inside the Table object — process-local is the object);
- the client draws the 429's sentence where the join form and the unlock face draw a refusal, and the signal is not
  ended by it (a 429 at a join is not AD-4's stream refusal; the builder confirms the client's 429 handling at those
  two forms does not route through the stream's end).

**What proves it.** Layer cells: the twenty-first wrong password 429 with the sentence and no header, the twentieth
403; a correct password at the twentieth clears and the next wrong one counts from one; two callers at one table count
apart; one caller at two tables counts apart; a refused request does not count; the entry gone after its window; both
routes share the counter (a wrong join and a wrong unlock by one caller at one table count together — the oracle is
the table's password, whichever route asks it). A mutant per value class (the key's two halves, the figure, the window,
the clearing) read red at review. On the Worker, the same cells through the Table object's route.

### 3.4 D4 — the `Host` allow-list (the NAS binary)

**As the code stands.** `crates/yokohama-web/src/main.rs:106` reads `YOKOHAMA_WEB_HOST` as the BIND address
(`127.0.0.1` by default; the image sets `0.0.0.0`, `deploy/README.md:257–259`); `crates/yokohama-web/src/lib.rs:1331`
`app()` builds the router with no host filter; the server answers any `Host`. Behind the tunnel the request's `Host`
is the public hostname (`deploy/cloudflared-config.yml` sets no `httpHostHeader`), so `yokohama-nas.moore.nyc` arrives
at the binary. A self-host on a workstation under the README's default publish answers a rebound name the same.

**The design.** Two variables (R-S3-14): `YOKOHAMA_WEB_HOSTNAMES_ENABLED`, the switch, and `YOKOHAMA_WEB_HOSTNAMES`, a
comma-separated list of host names the binary answers:
- the switch reads as the layer's other switches do (`Multiplayer::from_env_value`, `lib.rs:374`): absent or `on`
  turns the check ON, `off` turns it OFF — the binary answers any `Host`, as it does today, and its start line says
  "answering any host name — YOKOHAMA_WEB_HOSTNAMES_ENABLED is off" — and any other value refuses to start naming the
  value. Every committed deploy file, example and recipe sets `on` explicitly beside the list, so the knob is visible
  to a self-hoster who may want it off; the binary's own default is on, the public repository's safe default;
- with the check on and the list absent, the default set is the loopback names (`localhost`, `127.0.0.1`, `::1`) plus
  the bind address when it is a specific one (not `0.0.0.0` or `::`) — so a workstation's `127.0.0.1:8080` and the
  smoke's `127.0.0.1:18080` pass with nothing set, and a container bound to `0.0.0.0` answers loopback only until the
  operator lists its hostname (the trap the side agent named: a NAS create body without the list answers every
  tunnelled request 421 — which is why the recipe's tunnel reading is required, below);
- the comparison is on the host part of the request's authority, case-insensitive, the port ignored (a rebinding
  attack rebinds a NAME; a port is not the question), IPv6 literals bracketed as the header carries them;
- an unlisted host is refused **421 Misdirected Request** with one sentence that echoes the offered host through
  `echo::echo` (64 characters, the one echo rule) and names no listed host; the check runs on every path and method
  before routing, static assets included (a rebinding page reads whatever it is answered);
- the server prints the list it answers at start, beside its header-set line, so an operator reads it in the log; a
  malformed entry (empty, a space inside, a scheme) stops the server at start naming the entry, as a malformed header
  file does (`deploy/README.md` §4);
- the deploy: `deploy/README.md` documents BOTH variables in its environment table — the switch `on`, the list with
  the NAS's value (`yokohama-nas.moore.nyc`) — and its §4 `docker run` example carries both; the root `README.md`'s
  self-host lines carry both; §2's LAN note tells a self-hoster to list the name or address their browsers use, or to
  set the switch `off` when the boundary is not wanted; the
  Dockerfile that builds the runtime image (`deploy/Dockerfile.runtime`) carries the note (R-S3-10: "note in the
  docs/docker build file what's required") and no default value, and the self-hoster's `deploy/update.sh` path
  documents it beside the other variables it passes. **On the NAS the container is NOT a Portainer stack** — the two
  stack files are the terminal's and cloudflared's — and its environment is the `Env` array of the Docker API create
  body in `deploy/RUNBOOK.md` §3 (`:244–254`), which gains `YOKOHAMA_WEB_HOSTNAMES=yokohama-nas.moore.nyc`; the
  deployment's reading gains two lines, both taken by `docker exec` INSIDE `yokohama-web`, whose image carries `wget`
  (`Dockerfile.runtime:91`), since the terminal container is on `cf-terminal` and cannot reach the binary (B10's
  negative probe reads exactly that): `wget` against `127.0.0.1:8080` with `Host: yokohama-nas.moore.nyc` reads 200 and
  with a foreign `Host` reads 421; and over the tunnel as the owner `/api/health` reads 200 as before — **the tunnel
  reading is a REQUIRED step of the create recipe from 0.7.2 on, before a deploy is called done** (R-S3-14: a body
  that forgets the list answers 421 to every tunnelled request while a loopback smoke passes); and
  `deploy/smoke-image.sh` takes an optional third argument, the hostname, runs the container with the switch `on` and
  the list set to it, and probes `/api/health` twice more — with `Host: <hostname>` expecting 200 and with
  `Host: smoke-foreign.example` expecting 421 — so an image whose check is broken fails the smoke; without the argument
  the smoke runs as today and says the host probe was not asked for;
- the Worker is untouched; the e2e harness and the vite dev server reach the binary at `localhost`, and the smoke at
  `127.0.0.1` (`deploy/smoke-image.sh:39`); both are in the default set.

**What proves it.** Layer cells: a listed host answers every route as before (the whole route table under one listed
host, byte-identical to today's answers — the corpus's wire fixtures unmoved); an unlisted host is 421 on a static
path, an API path and a `HEAD`; the port is ignored both ways; the default set with and without a specific bind
address; a malformed list refuses to start with the entry named; the switch absent is on, `off` answers a foreign
`Host` 200 and says so at start, a malformed switch value refuses to start naming it; the start-up line in both states;
the smoke with and without its hostname argument. A mutant on the comparison (the host part vs the whole authority),
on the default set, and on the switch's absent arm (absent read as off) read red.

### 3.5 The test Worker — `yokohama-web-private` at `yokohama-test.moore.nyc`

**As the code stands.** `worker/wrangler.private.jsonc` exists: name `yokohama-web-private`, `workers_dev: false`, its
own objects (`TABLE`, `HOUSE`, `ARCHIVER` with the `v1` migration), bucket `yokohama-archive-private`, rate-limit
namespaces `7303`/`7304`, `YOKOHAMA_WEB_IDENTITY` `worker` with `YOKOHAMA_IDENTITY_SOURCE` `access`, the one
application's `ACCESS_AUD`, the dev door `off`, `YOKOHAMA_TURNSTILE` `off`, the TTLs 90 and 7 — and NO `routes`.
`worker/scripts/deploy-guard.mjs:333–341` records it with `routes: null` and `source: "access"`; `worker/package.json:21`
has `deploy:private` through both guards. S2b's D-7: "a new Worker's first deploy runs `wrangler deploy` with NEITHER
guard", the guard holding a file to its record only once the record names the Worker's identity. Production binds its
hostname as `"routes": [{ "pattern": "yokohama.moore.nyc", "custom_domain": true }]` (`worker/wrangler.jsonc:19`). The
guard's check 12 refuses a keyed stamp under any door but `open` or `worker` with `oidc`/`workos`
(`deploy-guard.mjs` lines 163–169), so the private Worker, under `access`, deploys a KEYLESS client — today's
production build is keyless too (stage 1), so one `worker/assets` serves both until the public day; after it the private
Worker needs its own keyless build (`npm run build` without the site key), which the runbook's recipe states.

**The design, in this order** (the order is the anti-goal: never a minute where the hostname answers a stranger):
1. **Access first** — the hostname `yokohama-test.moore.nyc` added as a destination of the one application (the
   `yokohama` application whose `aud` the file names), over the Cloudflare API by the orchestrator (R-S3-12); read back.
   **On a refusal or an error from the API, steps 2–6 are SKIPPED, the skip recorded in the runbook and the close with
   the error's text, and the deploy is taken up after the close by this same recipe** (the owner's words in R-S3-12).
2. The file gains `"routes": [{ "pattern": "yokohama-test.moore.nyc", "custom_domain": true }]`; the guard's record
   gains `routes: ["yokohama-test.moore.nyc"]`; the guard's cells cover the private file by its record.
3. The bucket `yokohama-archive-private` created (`wrangler r2 bucket create`); its lifecycle rules as production's for
   the `test/` prefix (the builder reads `worker/RUNBOOK.md` for production's).
4. The first deploy by D-7's recipe AS THE AUTH CYCLE LEFT IT: the deploy guard RUNS (it records the private Worker
   since the auth cycle's Task 7, `deploy-guard.mjs:333–341`; `secrets-guard.mjs:28–33`: "the deploy guard still judges
   a file whose deployment it records"; the file's own header says the same) and only the SECRETS guard is skipped,
   since a Worker that does not exist holds no secret — `node scripts/deploy-guard.mjs --config wrangler.private.jsonc
   && wrangler deploy --config wrangler.private.jsonc`, by hand — from a keyless `worker/assets` (the current
   `web/dist` holds the e2e's DEBUG client — the board's note; `npm run build` restores the release client first); the
   operator secret `YOKOHAMA_WEB_ADMINS` put if the owner wants an operator there (owner's call; absent, no operator,
   as the open shell has none).
5. Every later deploy through `npm run deploy:private` (both guards).
6. The readings over WARP as the owner: `/api/health` (identity `worker`, source `access`, the version), a table
   dealt, a stranger off WARP meeting Access's login; `worker/RUNBOOK.md` gains the private Worker's deployment record
   as its next numbered section and `docs/web-lane.md` §1's host paragraph names three Workers' worth of hosts.

The Access application's session and address list stay as they are (R-S3-1); the private Worker's door never flips.

## 4. The decisions (the orchestrator's; D-1–D-12)

- **D-1 — the message's `kind`, not a new message.** `install-failed` gains `kind: "challenged" | "failed"` beside its
  `reason`, so the door's existing reader, the cells that pin the reason's sentence and the page's "Try again" path
  change by one field. A second message name would be a second reader to keep true.
- **D-2 — one automatic pass per install attempt.** The door's counter, reset by a click or a page load. The
  alternative — pass until it works — is a loop against the edge's challenge, the anti-goal.
- **D-3 — the retry after a pass is the existing retry.** `swRegister.ts`'s retry takes `update()` when a worker is
  active (W-3's own case, a controlled returning browser) and registers anew only when the first install failed and the
  platform emptied the registration (measured); a second mechanism for the same recovery would be a second thing to
  prove. The door's cell covers BOTH branches (the builder check's correction 4).
- **D-4 — the reopen route's shape is the keep's, and it works at `Open` alone.** `POST …/reopen` with
  `{ "seat": n }`, parsed as `Keep` and `Join` parse theirs; the refusals are 403/409/400 in the sentences §3.2 gives;
  at `Dealing` every seated caller has been served a deal, so the reopen is refused there and the creator restarts
  (R-S3-13, R-SEC1).
- **D-5 — a dealing table expires on the waiting-table TTL, one knob.** The owner's "tables expire if they haven't
  started" is the open-table TTL's sentence widened; a third knob would be one more figure to read alike on two hosts.
  The repairable case is handled by `settle` before the sweep's read (NAS) and by the delete path's layer read (Worker),
  proven by a cell, not assumed.
- **D-6 — the close is Abandon.** It exists at every status for the creator; S3 verifies the control's presence at
  `Dealing` and builds nothing new.
- **D-7 — D2's key is (table, caller key), failures only, 20 per 10 s, no `Retry-After`.** The reviewer's shape over a
  per-table bucket: a per-table bucket lets one caller's guessing refuse another caller's right password — the
  anti-goal. The caller key is the door's and no connecting address is read into the layer (a new trust input the
  NAS's B10 model does not have); under `open` the key is the name, recorded as the LAN's limitation. The wait is in
  the sentence, not a header: the layer and the Worker's reply carry no header channel and the stream caps set the
  precedent (the builder check's correction 1).
- **D-13 — the automatic pass runs on the door, in ONE tab the worker elects.** The worker already knows every window
  (`clients.matchAll`) and the page does not know its siblings; an election in the worker is one rule in one place,
  where a cross-tab lock in the page would be a new dependency. The door alone, because cycle F gave the door the
  auto-apply and a game screen must never draw an overlay mid-play (the builder check's question 3).
- **D-14 — the pass counter is the page's, not the install attempt's.** Rechecks every fifteen minutes and on every tab
  switch would otherwise reopen a cancelled overlay without end (the builder check's question 4).
- **D-15 — a creator's own seats are never reopened.** A seatless creator could neither Abandon nor be expired nor read
  a dealing roster (the builder check's question 2 and correction 3); one 409 arm keeps R-1.6's invariant whole.
- **D-16 — the hosts part in the unread dealing corner.** A finalized-but-unflipped table nobody opened for the waiting
  TTL ends as `waited` on the Worker and is repaired on the NAS; the loss is a game with no moves, and the shell is not
  taught to settle before it reads for a corner no reading has met (the builder check's question 6 and correction 2).
- **D-17 — the success path's proof is the unit cells and the public day's window.** No keyed client can start under a
  door S3 can reach, and no harness stands in for Bot Fight Mode; the close states the gap rather than adding a keyed
  e2e build to a cycle the owner wants wrapped (the builder check's question 5).
- **D-8 — D4's default set is loopback plus a specific bind address; 421; host part only; the switch on by default
  and explicit in every deploy file (R-S3-14).** A container bound to `0.0.0.0` answers loopback until told otherwise —
  the safe default, since the tunnel's hostname arriving at an unlisted binary reads 421 at the required tunnel reading
  rather than answering a rebound name silently; the switch is for the self-hoster who does not want the boundary, and
  its explicit `on` in the committed files is what makes the knob visible to them. The smoke's hostname argument is the
  image's catcher; the recipe's tunnel reading is the deployed container's.
- **D-9 — the private Worker goes Access-first, over the API, with a typed skip** (R-S3-12). A route that goes live
  before the hostname is covered is a minute of an open Worker — the anti-goal — so the order is fixed and the skip is
  the whole of steps 2–6, never a partial deploy.
- **D-10 — the version moves to `0.7.2` at the cycle's release**, by the lane's §7 steps. The client's service worker
  changes, and a release a service-worker generation is named by carries its own number in this series (0.7.0, 0.7.1).
- **D-11 — the wrap's records.** The ledger's rows (§1's readings; D2, D4, D9 `fixed` with their catchers at the close;
  D17's cycle column `S4`); the spec's §9 gains "§9.9 — 2026-10-19, S3's brainstorm, design and plan" at the close and
  the file is FROZEN there, as §9 says; `docs/web-lane.md` §8's W-3 row and "Still owed" paragraphs and §15's W-3
  bullet read the mechanism built and what is still the window's; `worker/RUNBOOK.md` the private Worker's section;
  `deploy/README.md` and `deploy/RUNBOOK.md` D4's variable and D2's sentence; the board's Track 4 → S3; guidingpath's
  row.
- **D-12 — the branch and the PR.** `security-s3` from `security-auth` at `4ce7493b`; the PR against `security-auth`;
  the stack (`security-auth` + `security-s3`) merges to master on the owner's word before S4's repository work, the
  auth PR's own timing being the owner's (the board's Track 4 housekeeping). The plan is written by the orchestrator
  alone and executed by subagent-driven development with a review per task on an export, as every cycle of the series
  was (`docs/plans/2026-10-08-security-remediation-spec.md` §8).

## 5. Constraints the plan inherits

- The lane's suites per edit (`docs/web-lane.md` §2): `yokohama-web` (lib, `start_refusal`, `wire_fixtures`), `-wasm`,
  `-worker` and the `identity::` slice, the wasm32 check; `web/` typecheck, vitest, `test:wasm`, the e2e; `worker/`
  typecheck, `test:scripts`, the pool. The roster crates are not touched; no roster run is owed unless the plan touches
  `crates/yokohama-core` or `crates/yokohama-cli` (it does not).
- The wire corpus (`web/src/wire/*.json`) pins the layer's output byte for byte; a changed sentence or field regenerates
  a fixture under `UPDATE_WIRE_FIXTURES=1` and the diff is READ (the lesson M6.1).
- TDD, test-first, every commit; no `#[ignore]`; no `cargo fmt`; invisible code points escaped.
- No exploit detail in any committed file; the ledger's titles neutral; the review report never cited by content.
- Every task's implementer probes the brief's code before reporting (the security core PR's lesson, R-10–R-18), and
  reports DONE or DONE_WITH_CONCERNS to a transcript file; no bulk `rm` in any dispatch; no peer messaging between
  agents; the orchestrator holds the plan.
- The release-version rule (§7 of the lane) at the close: the workspace version, both npm packages, the wire fixtures'
  version field, the backup envelope's, the runtime image's example tag.

## 6. What is left to the builder (the plan)

- The exact sentences of every new refusal and the start-up line, in the house voice (one sentence, the echo rule).
- The reopen route's place in the layer's route table and in every Worker-side path table; the client's control's
  placement and words on the open roster and the dealing screen; the e2e cell's shape.
- The worker's election predicate on `WindowClient.url` for "a window showing the door", and the e2e cell's
  `context.route` shape for the challenged precache fetch (the keyless landing it proves, §3.1).
- The counter's type in the layer (the stream caps' shape is the precedent; a `Mutex<HashMap<(String, String),
  Window>>` with prune-on-exit is the expected form) and the window's bookkeeping.
- The `Host` parser's handling of an IPv6 literal, a trailing dot, and a `Host` absent on HTTP/1.0.
- The Worker's `factsOf` extension for `dealing`, and the NAS sweep's settle-first read and its fixture.
- The private Worker's bucket lifecycle, the operator secret's presence (the owner's call), and the runbook's section.
- The task order (a suggestion, not a rule): D4 and D2 first (layer-only, corpus-visible, quick), then D9 (layer +
  Worker + client), then W-3 (client), then the private Worker (operations), then the wrap.

## 7. Review record

### 7.1 — 2026-10-19/20, the builder check (an Opus reader of this document at `4ce7493b`, read-only; its report in the
session's scratchpad, `s3-builder-check.md`)

Seven questions and six false or stale claims, each with a file and line. Dispositions:

- **Question 1 — at `Dealing` an UNCHOSEN seat has already been served its two cards** (`GET …/deal` answers any seated
  caller at `Dealing` from the seed, `tables.rs:1565–1617`; the dealing screen fetches it on mount, `Dealing.tsx:178–198`;
  the player count does not change on a reopen, so the next joiner is dealt the cards the kicked person saw). The
  design's §3.2 rule 1 as first written ("a human seat that has NOT chosen") broke its own anti-goal at exactly the
  status D9 lives at. **THE OWNER'S** — the options change what is built (below, §7.2).
- **Question 2 — a creator's own last seat.** Answered: never reopened (D-15).
- **Question 3 — which page runs the pass.** Answered: the door, in one tab the worker elects (D-13).
- **Question 4 — rechecks and the counter.** Answered: per page (D-14).
- **Question 5 — an end-to-end proof of the success path.** Answered: none before the public day; the close says so
  (D-17).
- **Question 6 — the Worker's finalized-but-unflipped dealing table.** Answered: ended as `waited`, the hosts part
  (D-16).
- **Question 7 — what the other seats see when a reopen returns a table to `Open`** (today the dealing screen reads
  every 409 from `/deal` as "the game started", `Dealing.tsx:175–192`, and their keep is refused 409 `open` until the
  seat refills, `tables.rs:1760–1762`). Rides on question 1: under a reopen at `Open` only it cannot arise; under any
  reopen at `Dealing` it is a client cell the plan must carry.
- **Corrections taken into the text:** (1) the stream caps' 429 carries no `Retry-After` and neither the layer nor the
  Worker's reply has a header channel — §3.3, D-7; (2) the Worker ends the unread dealing corner — §3.2 rule 2, D-16;
  (3) `Games::delete` refuses a seatless caller first — §3.2, D-15; (4) the retry's `update()` branch is W-3's own case
  — §3.1 step 3, D-3; (5) D4's deploy reading: the terminal cannot reach the binary, `update.sh` is the self-hoster's,
  the NAS container's environment is the create body's `Env` in `deploy/RUNBOOK.md` §3 — §3.4; (6) the private Worker's
  first deploy runs the deploy guard and skips only the secrets guard — §3.5 step 4; (7) no edge stub exists in
  `web/e2e/` and the smoke reads `127.0.0.1` — §3.1 and §3.4.

### 7.2 — 2026-10-20, the owner's answer to question 1

Reopen at `Open` only (R-S3-13): "I agree reopen at open only, otherwise if it made to starting the game they have to
restart - just like if it stalled just a moment after that." Taken into §3.2 rule 1 (the `Dealing` arm refuses with
the restart sentence; no `keeps` read; the creator's own seats refused by D-15), the client bullet (the control at
`Open` only; the dealing screen keeps Abandon), the proof list, and D-4. Question 7 closes with it: the
`Dealing` → `Open` transition cannot arise. The options not taken — a fresh seed at `Dealing`, a served-deal record, an
accepted leak — are recorded here and nowhere else.

### 7.3 — 2026-10-20, after the plan's hand-over: R-S3-14, the `Host` check's switch and its deploy catchers

A side agent's note to the owner: a NAS create body reused as it stands today would carry no list, so every tunnelled
request would read 421 while the loopback smoke passed. The owner's ruling (R-S3-14, §1) adds `YOKOHAMA_WEB_HOSTNAMES_ENABLED`,
on by default and set `on` explicitly in every committed deploy file, with `off` for a self-hoster who does not want the
boundary. Taken into §3.4 (the switch's semantics and start line; the smoke's hostname argument; the recipe's tunnel
reading made a required step), D-8, and the plan's Task 1 (the cells for both switch states and the malformed value,
the smoke's probe, both variables at every deploy site, the root README) and its "After the tasks" owner item.

The same evening, two more: **R-S3-15** — the window is the plan's Task 10, at the very end, programmatic from the
workstation through Portainer's Docker proxy and WARP, with a typed skip for a host it cannot reach; and **R-S3-12's
step completed by the owner** — `yokohama-test.moore.nyc` is a destination of the one Access application, read back
over the API (`updated_at` 2026-10-20T04:41:55Z; the session reads 24 h, for D22's row at the wrap), so Task 7's first
step is a read-back and the private Worker's deploy runs in the plan.

### 7.4 — 2026-10-20, the close (the run by subagent-driven development; the orchestrator's record)

**The code, on `security-s3` above `302babdb`:** Task 1 D4 `dc17ded9` + `72e15b08` (the switch read trimmed and case-insensitive, P-12) + `e876df18` (the entry rule de-dots first and refuses any non-hostname character by name; both variables through `var_os`); Task 2 D2 `e823fe5a` + `3c980e35` (one lock acquisition per guess, `Guess { Refused, Wrong, Right }`, the window anchored at the first failure — P-17); Task 3 D9 `82fb4346` + `7996644f` (the reopen; the 409 re-reads once as `join`'s does — P-21; the dealing screen's Abandon control, built because it did NOT exist — P-20); Task 4 D9 `9e397966` (the dealing table's TTL on both hosts); Task 5 W-3 `cf79f85c` + `7d84a1f3`; Task 6 W-3 `f51afbef` + `c08c8bd8`; Task 7 `4ef280da` + `abd949b2` (the private Worker's hostname) and the operations commit `b9fc20a9` (§26); the release `0.7.2` `92703601`. Every code task but Task 4, whose review was clean: an Opus review on an export, one fix round, a scoped Opus re-review — 0 Critical throughout; the Importants were Task 5's measured premise (below) and Task 6's two unpinned cancel paths, both closed.

**The counts at the release commit, one sequence:** web 599 (557 lib, browser_played_pin 5, lane_document 6, the whole-game gate 2, start_refusal 11, wire_fixtures 18), wasm 10, worker 83, clippy clean; web typecheck 0, vitest 1,586 in 76 files, test:wasm 396 in 16, e2e 83 in 5.1 min; worker typecheck 0, test:scripts 794 in 32, the pool 299 in 14 — 10:29–10:41Z, 12.5 min.

**What the run found that the design did not know.** (1) **`WindowClient.url` is a window's LOAD address in Chromium** (measured twice, by the Task 5 review and the Task 6 implementer, in Playwright's Chromium): it does not follow a hash navigation, so D-13's premise — "the worker already knows every window" — holds for load addresses only; the election stays the worker's (one window, by load address), the page's own route check before an automatic pass is the load-bearing fence (P-26), and the two corners are recorded in the lane's §15 (a tab that entered at the door and moved into a game can be elected and lands on `failed`; a tab that arrived on a game link and moved to the door is never elected until it reloads). (2) The dealing screen had NO Abandon control (rule 3's "verify, add if absent" fired). (3) The plan's test requests carried no `Host` header (P-1) and its wire seat was 1-based where the code's is 0-based (P-18). (4) A repairable table whose repair FAILS is now named by the NAS sweep every pass and never ended, where its TTL ended it before — consistent with the pass's standing "named, never ended" rule; recorded. (5) An export must never share the checkout's `target/`: cargo keys a workspace crate by its path relative to the workspace root (the SDD ledger's R-8). (6) The Access application's full-list PUT recipes would have dropped the new hostname — every list-writing recipe now keeps four (§26).

**Read live (2026-10-20):** the private test Worker — Access read back (four hostnames, the aud, 24 h), the bucket with production's `test/` rule, the host's own keyless build (the release `yokohama-release-b41736f304b7d374`), the first deploy by hand (version `251dba8b-…`), `/api/health` 200 at `0.7.1`, `/api/me` the owner, the enforced header set, `no-store` under `/api`, the lobby socket 101, a bot table dealt, kept, one move made and abandoned into the private archive (`worker/RUNBOOK.md` §26). **Not read live, and owed:** the window (Task 10 — `0.7.2` on the NAS with both `Host` variables, the smoke's hostname probes and the `0.6.3-s2` negative control, D2's 21 passwords and D9's reopen on both hosts, then the Worker and `npm run deploy:private`); the off-WARP Access login at the test host (the owner's); W-3's success path (the public day's window, D-17); the LAN resolver's record for `yokohama-test.moore.nyc` (the owner's).

**The deferred minors**, for the whole-branch review's triage: CI's and `update.sh`'s smoke calls without a hostname; the stored-id key's cell; the client's 429 sentence as a literal; the guard's pattern-only route comparison; the `waited` head's "for players" on a dealing table (the owner's call); the pass counter per hook instance; the dealing-screen Abandon's e2e; and the rest as §7.5's closing paragraph lists them in full (the SDD ledger, a gitignored workspace file, is the working record; this section is the committed one).

### 7.5 — 2026-10-20, the whole-branch review (Fable, on an export of `b8a25992`) and its one fix wave

**The verdict:** `Ready to merge: With fixes` — 0 Critical, 2 Important, 11 Minor; **no code defect found.** The two Importants were RECORD defects of the wrap: the ledger's D32 row left at "open" while every other record said "accepted" (I-1), and the close's "every task … one fix round" where Task 4 had a clean review and none (I-2). The Minors: the repair-fails corner unstated at the lane's §4, `pass`'s doc and the guide (M-1); two false TTL clauses in `web/src/types.ts` (M-2); a route cell's comment still saying the counter asks before the compare (M-3); the forward corner's "Try again" (M-4); the README's entry rule narrower than the code's (M-5); the board's stale Updated line (M-6); the worker README's "still `access`" (M-7); §7.4's deferred list incomplete (M-8); a sentence for a Host-rewriting reverse proxy (M-9); no Worker cell for "not ended a day early" (M-10, the one surviving mutant — near-equivalent, the alarm arms at the full TTL); the settle arm-2 note (M-11, recorded). **Mutants: 38 of 39 red** against the final head, every task's named set re-run by hand in the export. **The vacuity table:** every sentence the cycle wrote about itself has its mechanism but the two the Importants name. **The security read:** D4, D2 and D9 close their findings as §3 describes; W-3's anti-goals hold (one pass per page, nothing under `/api`, the page's route fence); the live halves are correctly recorded as the window's. Playwright was not run (both e2e cells read). The report: the SDD ledger's `final-review-report.md`.

**The wave** (`ca2f001b`, one Sonnet commit): I-1, I-2 and M-1–M-9 as the review asked; M-10 by a day-12 cell in the dealing block that reads red under a TWO-site mutant (the deadline a day early in `marker.ts`'s `expiryReason` AND the alarm's own gate in `lifecycle.ts` — the single-site mutant is unreachable from the pool, since the gate returns early at the unmutated 7 days, and `marker.unit.test.ts` already kills it); M-11 recorded. Its suites: expiry 35, the pool 299 in 14, lane-variables 4, lane_document 6, `sweep::` 14, the web typecheck and clippy clean. **One rule the wave crossed and this commit undoes:** I-2's dispatch had it edit guidingpath's 2026-10-20 row in place; the decision log is append-only, so that row is restored as the wrap wrote it and a correction row is appended after it.

**The scoped re-review** (Opus, on an export of `ca2f001b`): all twelve findings ADDRESSED with no new breakage (I-1 the D32 row; I-2 at the design and the board, guidingpath handled here; M-1 at its three sites; M-10 read red under the two-site mutant and green without the new line, the single-site mutant already killed at `marker.unit.test.ts:162`); five non-blocking observations, the chief one that §7.4 pointed at the gitignored SDD ledger — made self-contained in this commit; the re-reviewer disclosed one read-only `git check-ignore` in the checkout.

**What stays recorded, not fixed:** the deferred minors the SDD ledger lists (CI's and `update.sh`'s smoke calls without a hostname; the stored-id key's cell; the client's 429 sentence as a literal; the guard's pattern-only route comparison; the `waited` head's "for players" on a dealing table — the owner's call; the pass counter per hook instance; the dealing-screen Abandon's e2e; Task 3's hardening pair; the settle arm-2 note; Task 7's remaining wording items) and the parked findings (the `Host` header over an absolute-form request line; `ownHand(b) == []` at an open table).
