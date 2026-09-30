# M6.5c2 cycle F "the clean finish" — the specification (2026-09-30)

**What this is.** The design for the web lane's last cycle, written from the brainstorm the owner and the orchestrator held on 2026-09-30 over the F handover (`docs/plans/2026-09-30-web-frontend-m6.5c2f-handover.md`, the entry; its §7 items and the four the owner added in the session). Every ruling here carries the owner's words, so it can be found by search; the M6.5c2 spec's §10.13 (`docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md`) is the same table in that document's shape and points here. The plan is written from this file by the writing-plans skill; no task runs before the plan is approved. The record of record for E, the tree F starts from, is `docs/plans/2026-09-30-web-frontend-m6.5c2e-close.md` (read its §7 before trusting any E sentence).

**Where F starts.** `web-room-m6.5c2f` from `web-room-m6.5c2e` at `15de8117` (E's close documents; E's code head `c39da5a1`, its final reviewed head `3107eb8b`), its PR against `web-room-m6.5c2e`. The stack #31 → #43 is OPEN and UNMERGED by the owner's word; the wrap (the merge bottom-up, the `v0.6.0` tag, the workflows re-enabled) is F's last step, on the owner's word (R-F7). Both hosts run E's release `0.6.0` behind the hostname-wide Access application; CI stays dark until the wrap.

**The one-paragraph shape.** F closes the lane on the owner's goal that the site will one day run on Cloudflare with no Access barrier, for the owner and for anyone hosting it, on the Workers Free plan: *"the internet is filled with scanners and bots and automations and working it out so we keep our workers free tier invocations and metered readings down is the key goal"*. So F builds the door's no-click update path (§2.1) and a release notice (§2.2) without a single server request; lets an old-bot game play on under a new build (§2.3); gives the self-host the Worker's table expiry (§2.4); pins the door unmetered with a static 404 page, a guard, a probe and a ledger of every metered action (§2.5); builds Turnstile with the edge rule that makes bots free, behind a switch, and proves it in a public window (§2.6); adds the site's web presence (§2.7); pays E's minors (§2.8); writes the documents (§2.9); and wraps.

## 0. Read order and sources

1. This file, then the F handover for what E left (its §1–§6).
2. E's close (`2026-09-30-web-frontend-m6.5c2e-close.md`), §5 every ruling, §6 what the lane leaves, §7 what E does NOT claim.
3. The M6.5c2 spec's §10.11 (E's rulings R-6.5c2.37–.45), §10.12 (E's corrections) and §10.13 (this cycle's rulings, the owner's words).
4. `docs/web-lane.md` — the lane's operating record: §2 the suites, §8 the public door's preconditions, §12 every variable, §13 the API map, §15 by design.
5. `worker/RUNBOOK.md` §7 (the quota guard, the Observability query API) and §19 (E's deployment, the door window's record and the flip recipe); `deploy/RUNBOOK.md` §3.19 (the NAS's sixteenth deployment).
6. The identity freeze (`docs/plans/2026-08-30-phase3.5c-v4-close.md` and the freeze document it names) for why §2.3 is honest: a contestant's name never re-points.

Platform facts this specification rests on, each read on 2026-09-30 and cited where used: Workers pricing footnotes 3 and 4 (`developers.cloudflare.com/workers/platform/pricing/`); Workers Cache's configuration (`/workers/cache/configuration/`: the `cache.enabled` block, Wrangler 4.69 and up); static assets' billing (`/workers/static-assets/billing-and-limitations/`); Turnstile's plans (`/turnstile/plans/`: free, 20 widgets an account), its Spin flow (`/turnstile/spin/` and `prompt.md`), its testing keys (`/turnstile/troubleshooting/testing/`); clearance and pre-clearance (`/cloudflare-challenges/concepts/clearance/`); the `cf-mitigated: challenge` pattern for fetches (Cloudflare's post "Integrating Turnstile with the Cloudflare WAF to challenge fetch requests"); WAF custom rules' availability (`/waf/custom-rules/`: five on Free, every action but Log, no regular expressions). The one platform fact measured by this project before F: **a request for a path that matches no asset is processed by the Worker** — the M6.5c spec's spike, `docs/plans/2026-09-22-web-frontend-m6.5c-objects-spec.md` §0.3 row 7.

## 1. The rulings

Each row is the owner's decision from the 2026-09-30 session, the owner's words quoted, and where it lands. R-F7 is F's authorization; the merge and the tag stay on the owner's word.

| ruling | the owner's words | where it lands |
|---|---|---|
| **R-F1 — the wording path: the blurb, the notice and the link stay BUILD VALUES (R-6.5c2.40 stands), on both hosts.** A wording change is a rebuild and a deploy; F-1's update path delivers it. No `site.json`, nothing fetched at run time for words. | *"I agree with the build values, it helps keep parity as well with the docker/NAS version"*; *"We would not cut a new release on the repo because of a footer change, that's a build/deploy variable"*; *"footer changes or blurb changes may come sooner, and we do want clients to get those as soon as practible (15 minute to an hour delay is tolerable, but ideally not later)"* | §2.1 |
| **R-F2 — the release notice: ONE build value naming a release feed, its default derived from the project link by the build scripts and the guide, the check made by the door and the lobby in the visitor's browser, nothing sent to anyone, no server involved.** | *"we should include an opt-in variable at build time with the default value to check for that, same with build scrips for people the default value to be set and in the docs. Automatic notifcations won't be necessary, although the front door of the site on that local hosted version should say if it is enabled that a new version is available in a bubble/notice somewhere on the page so at least the operator can tell"*; *"we probably should also include it on the multiplayer lobby as well"* | §2.2 |
| **R-F3 — the update path: the client re-checks `/sw.js` on focus and every 15 minutes; the DOOR applies a ready release itself, no click; off the door the offer is one passive click line; nothing reloads by itself while someone plays; every mechanism free of Worker invocations.** | *"The footer change 15 minute is good same with the blurb"*; *"critically, I do not want to take an approach that causes worker invocations, this will be a free tier release"* | §2.1 |
| **R-F4 — the resume rule (amends R-6.5c2.10 forward): a build that reconstructs the EXACT recorded bot plays the game on; new games deal only the production bot; every game that truly cannot continue is refused with its reason; `SEMANTICS_REV` stays the compatibility marker and no shim for an older rev enters the web build.** | *"I think that we should adjust here so that a V5 build can still play V4, but only allows new games to be created in the most current version"*; *"we could be considering the SEMANTICS_REV as a marker, but we don't want the web build unless you disagree to carry a lot of technical debt of older cycles"*; *"we just need to account for how to commnuicate there is a reason a game can't be continued, and if there is (that's what drove the web lane's refusal doctrine) then we still do that, but if we can play it, we should allow them to continue"*; *"the resume rule we should be putting in F this is what wraps the web lane - v5 isn't focused or caring about web it's just focused on the computer player"* | §2.3 |
| **R-F5 — the door unmetered, as a fence with instruments: a static 404 page instead of a Worker-answered miss; the probe measured; a ledger of every metered action with its purpose, its totals carrying the harness caveat.** | *"the front door on the worker path will be static assets only so unlimited or unmetered use will be there as the design is holding now"*; *"Building a 404 as a static asset is definitely soemthing we want to include here"*; *"I think F should include the broader probe measurement"*; *"making sure we've understood the purposes behind the actions in the site that generate metered actions to make sure we're behaving efficently"*; *"note the metered-action ledger will suffer in the totals from the realities of the harness but even understanding the range and more importantly the actions will be valuable for both future users and our own final analysis at the merge and wrap"* | §2.5 |
| **R-F6 — Turnstile, built as the no-Access deployment's standing shape behind a switch: the widget at the multiplayer button and wherever a fetch meets a challenge, pre-clearance, siteverify on `/enter`, and ONE WAF Managed Challenge rule on `/enter` and `/api/*` that keeps bots off the Worker at the edge; three settings held together by the deploy guard; Spin's form-gating shape not taken.** | *"there is likely a point at which we would move the full site onto cloudflare without any access barrier - and so to help reduce invocations from bots and scanners that aren't just blocked by bot fight mode on the page, we have turnstile at the multiplayer button before any invocations to help reduce that workload, if that works"*; *"the goal is that myself or others would be hosting this without any access protections - and that we have built in some ways to have this not be so easily abused, to the extent that it's possible"*; *"we would likely alos need to add those as secrets and a third option in the worker as a build/deploy variable to enable or disable it entirely if people didn't want to enable it"*; *"I'm not concerned about spin's prebuild version if you have a petter approach given my overview of the goal behind it"*; *"The outbox hitting the turnstile is something we will need to consider … getting that mechanic right will be important"* | §2.6 |
| **R-F7 — F's authorization: the deploys, the push, the PR, the public window and its Access and zone changes are the ORCHESTRATOR's at discretion, each a dated record with the quota guard before it; the merge #31 → F's PR and the `v0.6.0` tag stay on the owner's word at the wrap.** | *"We can include in this plan turning access off for testing purposes, same too for disabling WARP"*; *"No problem making it public to check and trial"*; *"The window and changes can be made at the orchestrator's discretion no need to wait for me on those calls, same too for deploys and push and PR"* | §4, §8 |
| **R-F8 — an R2 "dropbox" for saves: considered, NOT taken in F.** R2 takes no anonymous write, a URL the browser could upload to must be minted by the Worker (the same one invocation the archive post costs), and the post is what runs the strict replay that keeps junk out of the archive. | *"we may want to consider a dropbox kind of approach in r2 as well but it seems we want to consider that in more depth"* | §6 |
| **R-F9 — the self-host sweep: the Worker's two TTL names and meanings; an hourly task ending idle tables through the creator's own delete; a TTL with no archive directory REFUSES to start; each pass lists what it will end before ending any; no dry-run variable.** | the handover: *"include the table TTL days clean up work for the docker release"*; the design accepted in the session | §2.4 |
| **R-F10 — no marker on the self-host**: a removed table answers "no such game"; the divergence from the Worker is named as such. | *"For question 7, no marker is fine. While parity in many ways between the two is a goal, for areas where there are huge divergence outside what we're getting into, it's understandable and given that this will later be publicly open source if someone wants to get a PR in for that they can."* | §2.4, the lane document §15 |
| **R-F11 — ONE cycle, E's shape**: about fifteen tasks in three parts and the wrap, one public window, one deploy pair; if the plan passes what one agent reads whole at drafting, it splits then. | *"approach 1 will work here even including the SEO/web presence optimziation task"* | §4 |
| **R-F12 — the web presence**: `robots.txt`, `llms.txt`, a sitemap, the page's metadata and structured data, static, with defaults, the repository link where it belongs. | *"I'd also like to include in this wave a task for SEO/robots.txt/llms.txt etc optimizations for search engines like google and llms so that's handy for any public deployments, default values, structured well and pointing to the GitHub page where appropriate."* | §2.7 |
| **R-F13 — the version stays `0.6.0`** and the tag is `v0.6.0`: F changes neither what a save means nor which bot plays (`docs/web-lane.md` §7's rule). | approved with the wrap's section | §4 |

## 2. The mechanism

### 2.1 The update path (F-1; R-F1, R-F3)

**What E left.** A returning visitor's first visit after a release is served the previous release by the old worker while the new one installs; the offer "A new version is ready. Reload to use it" renders on the door only (`web/src/OnThisDevice.tsx`, `Readiness`) and needs a click; the worker half (`web/src/sw.ts`) calls `skipWaiting()` only on the page's message and retains the previous generation, which is what keeps a tab that did not click working (PR #37, R-F2 and R-F5 of that cycle). The page half is `web/src/swRegister.ts` (`useReadiness`, `applyUpdate`, `readinessOf`). Nothing is stale at the edge: `/sw.js` and `/shell.bin` are `no-store` on both hosts.

**The client re-checks the release.** `useReadiness` calls `registration.update()` on `visibilitychange` to visible and on a 15-minute interval while the page is open. Each check is one fetch of `/sw.js` (29 KB, `no-store`), a static asset the Worker serves free (pricing footnote 3). The browser's own check on every page load stays. The interval is a named constant with a cell under fake timers; the focus path has a jsdom cell.

**The door applies a ready release itself.** When the route is the door (`web/src/route.ts`, `kind: "door"`; `useRoute` in `web/src/App.tsx`) and `readinessOf` reports a waiting worker beside an active one, the page calls `applyUpdate` — the SAME path the click takes, so the two "it has taken charge" events (`controllerchange` where the old worker controlled the tab, `activated` where nothing did) and the one-tab rule are unchanged. It fires at most once per page life (a ref), never on a first install (the offer needs a waiting worker AND an active one, `readinessOf`'s existing rule), and never while the route is not the door.

**Off the door, one passive line.** "A new version is ready. Reload to use it" is drawn on the lobby and the game screens as it is on the door, and the click is the only switch there. Its exact place on each screen is the orchestrator's call with a before-and-after picture (R-6.5c2.38's rule). Nothing reloads by itself while someone plays.

**What makes the in-game click safe**, and the cell that pins it: a game open under release A, release B built and served, the line clicked, and the SAME game continues. A local game resumes from the browser's storage (the `offline.spec.ts` close-and-reopen cell is the precedent; that spec already produces a second release by re-running the precache generator over its own output); a table game rejoins by its link (E's arrival path); §2.3 keeps an old-bot game opening under a new build. The cell is an e2e over two releases of the local binary.

**The worker half is untouched.** `sw.ts`, `swRoute.ts` and `swInstall.ts` do not change; the update policy's worker half stays "skip-waiting only on the message, never `clients.claim()`".

**The guard gains a check** (`worker/scripts/deploy-guard.mjs`, check 10): a `cache` block whose `enabled` is not `false` — at the top level, in any `env`, or under `exports` — is refused, read as text and as values. With Workers Cache on, "requests served from the Worker's cache are billed at the same per-request rate as requests that invoke the Worker. This includes requests to static assets" (pricing footnote 4; `/workers/cache/`). Absent is off, since the block is an opt-in; the check refuses the KEY with any value but `false`. Check 5's pin of `run_worker_first` to `["/api/*", "/enter"]` stands.

**The readings, both hosts** (§3.3), with E's pinned-browser instrument committed as `web/scripts/pinned-browsers.mjs` (orchestrator-only; its state directory gitignored; no cookie VALUE ever written — the ledger copy's own rule): a browser pinned to E's release lands on the door and runs F's within seconds with NO click; a browser with a game open shows the line after a focus change; the door visit at zero invocations (§2.5).

**Not built:** no `release.json`; no forced reload in a game; no words fetched at run time. The self-host is identical by construction (the worker is the client's), and the reading is taken on the NAS too.

### 2.2 The release notice (R-F2)

**One build value, `YOKOHAMA_RELEASE_FEED`.** The URL the door reads for the latest release. Read as `web/scripts/build-values.mjs` reads the three (`BUILD_VALUES`), defined into the bundle by `web/vite.config.ts`, and given to the Docker build as an `ARG` beside the three in `deploy/Dockerfile`. Three states, spelled so "empty" and "unset" cannot be confused:
- **unset**: the default — when `YOKOHAMA_PROJECT_URL` names a GitHub repository (`https://github.com/<owner>/<repo>`, a trailing slash tolerated), the feed is that repository's latest-release endpoint (`https://api.github.com/repos/<owner>/<repo>/releases/latest`); when the link is anything else, or unset, there is no feed and no check;
- **a URL**: that feed (a fork points its own build at its own repository through its project link, or here);
- **the literal `off`**: no feed and no check.
The build scripts and the guides SET nothing for it, so the derived default is what a self-hoster gets; blanking it is `off`.

**The client reads it once per page life on the door and on the lobby**, and at most once an hour per browser after that (a timestamp in browser storage, every access wrapped, `chatFold.ts`'s idiom), which keeps a busy device inside the feed host's anonymous limit of 60 requests an hour. A cross-origin request: the service worker leaves it alone (`sw.ts`'s same-origin rule), the Worker never sees it, and offline or on any failure nothing is drawn and nothing retries within that page life. The read goes through its own seam (`web/src/releaseFeed.ts`, an injectable fetch), so the door's "asks the server nothing" cell (`FrontDoor.test.tsx`, the throwing `fetch` stub) keeps its meaning: no same-origin request.

**The comparison.** The feed's `tag_name` against the build's own workspace version, defined into the bundle as `__YOKOHAMA_VERSION__` from `web/package.json` (the cell `the_two_npm_packages_carry_the_workspace_version` holds that file to the workspace). Both parsed as `vX.Y.Z` triples; a tag that does not parse draws nothing, never a false alarm; equal or older draws nothing.

**The notice**: one quiet line on the door and on the lobby — "A newer release is available: v0.7.0. This site runs 0.6.0" — linking to the release's page (`html_url`, or the project link's releases page when absent). Its place on each screen is the orchestrator's call with a picture (R-6.5c2.38).

**Both hosts, one mechanism.** The NAS and any self-host behave as the Worker does; the Worker's own door checks too, which is how the owner sees a tag the deployment lags.

**Proof** (§3.1): jsdom cells over a fixture feed — newer, equal, older, unparseable, blank — and the derivation of the default from the four link shapes. The repository is private today, so the feed answers 404 and the live reading records "no notice", correctly; the day the repository is public the check works with no change.

**Not built:** no server-side check, no notification of anyone, no self-updating server. The self-host guide gains an "Updating" section and `deploy/update.sh` (§2.9).

### 2.3 The resume rule (R-F4; amends R-6.5c2.10 forward)

**What R-6.5c2.10 built, and what it costs.** `crates/yokohama-web/src/bots.rs` marks a bot seat FOREIGN when its recorded `(controller, version, params)` differs from `production()`, the descriptor of `bot::PROVISIONAL_DEFAULT_BOT`; `Games::live_handle` (`games.rs`, the rule's one site) refuses to open a game that is not over with a foreign seat. Its own test names the cost: the fixture's foreign bot is `lookahead-v1`, *"which this CLI CAN run"*. The CLI's resume (`crates/yokohama-cli/src/bot.rs`, `BotSeats::from_provenance`) rebuilds the recorded bot through `Contestant::from_parts` and `eligible`, plays on, and only WARNS when the name's version drifted (`identity_drift_warnings`); an unrunnable seat resumes as a human with a warning. So a V5 ship would cut off every unfinished V4 game on the web while the build could still play V4.

**The rule.** A bot seat is foreign only when this build cannot reconstruct the exact bot the record names:
1. `Contestant::from_parts(controller, params)` fails (a name this build does not know, a knob the identity never had — `from_parts` already refuses a named identity at other knobs);
2. or the contestant is not an eligible seat (`bot::eligible`, the CLI's rule: a diagnostic never seats);
3. or the reconstructed contestant's `descriptor()` version differs from the recorded one — drift under the same name, a warning on the CLI, a REFUSAL here.
Otherwise the game opens and the recorded bot plays its seat, whatever the production bot is. This is the CLI's own resume rule with drift turned from a warning into a refusal.

**New games deal only the production bot**, as today: an undealt table, the door's bot card and the table form name `PROVISIONAL_DEFAULT_BOT` through the contract file `web/src/wire/production_bot.json`; nothing on the web offers an older identity.

**`runs_here` on the wire** (`TableBot.runs_here`, `bots::table_bot`) means the same predicate, so a V4 table under a V5 build reads "V4" in the lobby and opens.

**Every refusal stays, with its reason.** The authored sentence names the bot and WHY this server cannot run it (`bots::cannot_continue`, widened to carry the reason: unknown, ineligible, knobs, drift); the `why` refusal for a move by a truly foreign bot stays (the thirteenth refusal shape); a `SEMANTICS_REV` mismatch keeps "Will not open here" with export and remove; **no compatibility shim for an older rev ever enters the web build** — the owner's line on technical debt.

**The CLI is untouched**: it still resumes an unrunnable seat as a human with its warning; the web never converts a seat.

**What moves**: `bots::foreign`, `bots::table_bot`, the sentence; the one site in `games.rs`; the lobby's wording where `runs_here` was false; the corpus, whose foreign fixtures are re-derived from a GENUINELY foreign identity (a named identity at knobs it never had, or a drifted version), the diff read (a regenerated corpus freezes a leak as faithfully as a fix). The browser module follows, being the same layer.

**Cells** (§3.1): a `lookahead-v1` game resumes under the production build and its bot plays on (today's `games.rs` fixture, flipped from refusing to opening, and the bot's next move made); a knob-mismatched named identity and a drifted version refuse, naming why; a finished foreign game opens as today; `why` on a move by a runnable non-production bot reconstructs it by replay; the "Will not open here" list in the browser follows the predicate.

**Fences**: no engine file, no bot behaviour, `SEMANTICS_REV` 7; the change is the layer's rule about which recorded bot it seats. What makes it honest is the AI lane's naming rule: a contestant's name never re-points, and a grandfathered identity stays byte-identical (the identity freeze). One line in `docs/plans/2026-09-11-ai-lane-reentry-handover.md`: a V5 ship no longer cuts off V4 games on the web.

### 2.4 The self-host sweep (F-2; R-F9, R-F10)

**What E left.** The axum host reads no TTL, no table expires there, `/api/health` says so (`table_ttl_days: null`, `open_table_ttl_days: null` — `api::Host` and `HealthView` in `crates/yokohama-web/src/api.rs`), and `deploy/README.md` §8 is the by-hand recipe.

**Two launch variables, the Worker's names and meanings** (`worker/src/config.ts`'s rules, restated on the axum host):
- `YOKOHAMA_WEB_TABLE_TTL_DAYS`: whole days above zero a table stands idle before it expires; unset means no sweep, today's behaviour;
- `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS`: whole days above zero, default 7, inert without the first, for a table whose status is still `open` (a seat unfilled); a `dealing` table waits the first, as on the Worker (E's ruling 9-b).
A bad value stops the server at start naming the variable (the chat variables' shape, `main.rs`).

**Activity** is the latest write among the game's three documents — the record, the table and the chat — read as their modification times; the same three the Worker folds its `written_at` stamps over. The `Store` trait (`crates/yokohama-web/src/store.rs`) gains one read, `last_written(id) -> Result<Option<SystemTime>, String>`; `FsStore` reads the three files' metadata, `MemStore` stamps writes for the cells. The house chat is never a table (`is_the_house`).

**The sweep** is a tokio interval task started in `main.rs` beside the server: hourly, the first pass one minute after start. Each pass:
1. lists the tables (`Store::list(Doc::Table)`), reads each document's status and activity, and decides through ONE pure function (`sweep::ending(status, last_active, now, ttl, open_ttl) -> Option<Ending>`), the mirror of the Worker's `expiryReason`;
2. logs one line per table it is about to end — id, status, idle days, reason (`expired` or `waited`) — BEFORE ending any;
3. ends each through the same layer delete the creator's Abandon uses (`Games::delete(id, &creator)`, the creator read off the table document, `TableDoc::created_by`; the Worker's `expire` shape), so a finished game already stamped is never put twice, a mid-play game lands under `abandoned/`, an open table with no game archives nothing, and a failed archive put REFUSES the delete and keeps the table for the next pass (the existing rule);
4. logs each ending with where the copy went.

**A TTL with no archive directory refuses to start**, naming both variables: a sweep that discards games is not a configuration this host offers. Stricter than the Worker (whose marker says "this server keeps no archive"), by choice.

**No marker** (R-F10): a removed table answers "no such game" as today; the log line is the record. The lane document's §15 names the divergence.

**`/api/health` reports both values** as the Worker does, `null` when unset; the wire corpus's `health.json` is unchanged because the corpus host sets none.

**Where it lives**: the decision in a pure module beside the task (`crates/yokohama-web/src/sweep.rs`, no platform call, tested without tokio); the task itself and the store read behind the `server` feature, which `platform_calls_stay_behind_the_server_feature` holds.

**The documents**: `deploy/README.md` §4's variables table gains the two rows, its §8 is rewritten around the sweep (the listing, the archive rule, the log lines) keeping the by-hand recipe for the unset case; the lane document's §12 rows for the two names gain the NAS in their host column (the drift cell `worker/scripts/lane-variables.test.mjs` follows); `deploy/RUNBOOK.md` records the NAS's seventeenth deployment with the values the owner chooses.

**The reading** (§3.3): a scratch container over a copy of the NAS volume with a short TTL, its listing and its archive landings read; then the real deploy, its first pass's listing recorded.

### 2.5 The door unmetered (F-3; R-F5)

**The fence, stated once.** On the Worker a door visit, a local game to its end, and any request for a path the release does not serve cost zero invocations; the first metered request is `/enter` behind the multiplayer button, a link arrival's first API read, or a finished local game's archive upload. Pinned three ways: the guard on the configuration (§2.1's check 10, the checks below), an e2e cell on the client, and the live reading.

**The 404 page.** Today `not_found_handling` is `none` and, as the M6.5c spike measured, **a missing path is answered BY THE WORKER** — so a scanner's `/.env` or `/wp-login.php` is an invocation, and E measured a door VISIT, never a probe. F ships a static `404.html` in `web/public/` (the project's own words, a link home, `<meta name="robots" content="noindex">`) and sets `not_found_handling` to `404-page` in `worker/wrangler.jsonc`'s assets block and in the pool's two copies; the guard's check 5 is amended to that value and a new check (11) requires the file in the assets tree. Spec §2.3 of the M6.5c objects spec is corrected forward here: the value it chose was right for the install and wrong for the probe.
- **Why the install tolerates it**: the worker checks every entry's status and content type (`swInstall.ts`'s `checkHead`), so a 404 page fails an install as honestly as an empty 404 did; and a controlled browser never sees it, since the worker answers every navigation from the shell (`swRoute.ts` rule 2).
- **The axum host** serves the same page for a miss that is not a route, for parity (`assets.rs`'s `serve_path`).
- **Not precached**: the 404 page and §2.7's presence files are excluded by the precache generator's walk (`web/scripts/precache.mjs`, the `distFiles` filter), so the offline count is unchanged.

**The probe** runs in the window (§2.6): scanner-shaped requests for `/.env`, `/wp-login.php`, `/admin`, `/xmlrpc.php` and a random path, with and without navigation headers, read against the Observability query API's stateless count (`worker/RUNBOOK.md` §7's method), beside a control read that proves the instrument counts (one `/api/health` GET reads one). Zero is the pass. If a non-navigation miss still invokes the Worker after the 404 page, the close records it and weighs the remaining lever there — the platform's navigation flag and the edge rule are the candidates — rather than guessing now.

**`/api/health` joins the protected paths** for a state-2 door: `worker/scripts/access-paths.mjs` stops excepting `health`, its drift cell follows, and the lane document's §8 list changes. The lobby reads health after `/enter`; the deploy's smoke and any uptime probe carry the service credential, or watch `/` (a static asset). Under a state-1 door nothing is protected and the edge rule (§2.6) covers health.

**The ledger** is a new lane-document section, "The metered-action ledger": one row per request the client can make to the server, with its route, its trigger (the screen and the action), its purpose, its count per session or per applied move (E's figures — 7.9 requests a move, 8.6 per event with the setup in — and F's tally from `web/e2e/harness.ts`'s `YOKOHAMA_WEB_TALLY` through `web/scripts/tally-report.mjs`), whether it can happen before a clearance (§2.6), and whether a script could make it without a browser. A cell in `crates/yokohama-web/tests/lane_document.rs` holds every route of the router to a row (the API map's discipline). **The caveat, recorded in the section's first paragraph**: the totals are the harness's — the two-browser tier's watcher, the replay's forces — and the value is the actions and their ranges, for the wrap's own analysis and for anyone hosting it.

**The fence cell** (`web/e2e`): the door makes no same-origin request beyond the release's own files (no `/api/`, no `/enter`, no unknown path); a local game makes none before its upload; an unknown path is answered 404 without the shell. Against the local binary the axum host answers; the Worker's behaviour is the live reading.

### 2.6 Turnstile and the public window (F-4; R-F6, R-F7)

**What it can and cannot buy, by door state.** Under today's hostname-wide Access nothing reaches the Worker without a login, so a widget prevents no invocations. Under a state-2 door (the door public, the API behind Access) the same, with `/api/health` closed by §2.5. Under a state-1 door (no Access, the owner's end state) every API path is metered and public, and a script skips the door's button and calls `/api/*` directly — so a widget at the button alone, the shape Spin builds, gates one navigation and leaves the API open. What keeps bots off the Worker is an EDGE rule, with the widget as the way a person passes it once.

**Three settings, one guard** (the switch the owner asked for, *"a third option in the worker as a build/deploy variable to enable or disable it entirely"*):
- **`YOKOHAMA_TURNSTILE_SITEKEY`**, a client BUILD VALUE (the site key is public by nature); blank draws today's plain link and loads no script — the NAS build's case;
- **`YOKOHAMA_TURNSTILE`**, a Worker var, `off` or `on` (`worker/src/config.ts`'s `onOff` shape); `off` in `wrangler.jsonc` and `wrangler.gate.jsonc` today, `on` in `wrangler.open.jsonc`; `on` makes `/enter` verify a token;
- **`TURNSTILE_SECRET`**, a Worker SECRET, refused by name in any `vars` (the guard's check 7 list gains it), put through `wrangler secret put` — Spin's guarded retrieval (`wrangler turnstile widget get`, Wrangler 4.109 and up; the tree pins 4.136.3) or by hand; `on` without it answers every request 500 naming it (the `YOKOHAMA_WEB_ARCHIVE=r2`-without-a-bucket shape). The repository root's `.dev.vars` now carries a line of that NAME, inert for the Worker (wrangler reads the Worker directory's file; `worker/scripts/dev-vars.mjs` ignores other keys).
The guard's check 12 holds them together: `on` exactly when the bundle in the assets tree carries a site key (read as E's build check reads the blurb), `off` exactly when it does not. So each door state is its own build: production (Access) is built without the key and deployed `off`; the open configuration deploys a build WITH the key, `on`; the recipe names which build each deploy takes. The pool runs Cloudflare's published always-passing test pair (`/turnstile/troubleshooting/testing/`); the e2e runs on the test site key.

**The widget loads on demand only**: Cloudflare's script (`challenges.cloudflare.com/turnstile/v0/api.js`, cross-origin, so `sw.ts` leaves it alone) is loaded from the multiplayer button's click, from a challenged fetch, or for a pending upload on a visible tab — never at door load, never precached, nothing on the offline door but a sentence that the lobby needs a connection. Mode `managed`; pre-clearance ON at level `managed`, which is what lets a cleared browser pass a Managed Challenge rule; the widget's hostname the Worker's. The owner's existing widget is confirmed for mode, hostname and pre-clearance before anything is built (`wrangler turnstile widget get`, metadata only).

**The button.** With a key, the friends card renders the widget on the click; its token rides `/enter` as a FORM POST (`cf-turnstile-response`), so no token sits in a URL or a history. The Worker's route table (`worker/src/route.ts`) gains `POST /enter` under `on`: siteverify (`challenges.cloudflare.com/turnstile/v0/siteverify`, a subrequest, which Cloudflare does not bill; the hostname checked against the Worker's), then the 302 to `/#/lobby` as today; no token or a bad one is a 403 sentence; `GET /enter` under `on` is a sentence pointing at the door. Under `off`, `/enter` is untouched (`GET` 302, other methods 405). The axum host has no Turnstile half.

**The fetch handler.** Cloudflare marks a challenged fetch with a 403 carrying `cf-mitigated: challenge`, and its guidance for single-page applications is to render the pre-clearance widget then and retry. The client's one API call site (`web/src/api.ts`'s `call`, beside `isAccessChallenge`) sees the header, renders the widget once in an overlay ("Checking that this is a person…"), and retries the request after success; a second challenge on the same request is the ordinary error path; with no key in the build, a challenged fetch is the ordinary error path at once, naming the edge. So one mechanic covers every path in: a link arrival at a table meets its first API read challenged, the widget appears once, the read retries, the person is in; a clearance that lapses mid-game meets the same path at the next fetch, and the socket reconnects after it (the client's existing reconnect). The overlay is a component with a seam, so the cells drive it without the script.

**The outbox** (`web/src/outbox.ts`) gains a fifth `PendingReason`, `"challenged"`: a pass stops at the first challenged send and marks the row; the widget renders for it only in a VISIBLE tab from a launch or a click (the door's pending row offers it), never from the `online` event in a background tab; a landed retry clears the row as today. A game-over upload on a visible page meets the widget through the fetch handler like any fetch.

**The WAF rule.** ONE custom rule on the `moore.nyc` zone (five on Free; every action but Log; `starts_with()` is a function of the rules language, regular expressions are not on Free): expression `(http.host eq "yokohama.moore.nyc" and (http.request.uri.path eq "/enter" or starts_with(http.request.uri.path, "/api/")))`, action Managed Challenge, made through the API as the rate-limit rule was (`worker/RUNBOOK.md` §19) and kept by its id. **Enabled only while the door is open** — the live replay's and the live tier's service token cannot pass a challenge, and under Access the rule adds nothing — and ON in the public end state. The zone's one rate-limit rule at 20 per 10 s stays; the rule bounds bursts, not the quota (20 per 10 s from one address is 172,800 a day).

**The window** (R-F7, at the orchestrator's discretion; E's flip recipe in `worker/RUNBOOK.md` §19 extended):
1. the quota guard (`worker/RUNBOOK.md` §7);
2. a client build WITH the site key; the operator secret deleted first (the `open` shell refuses to start beside it); `npm run deploy:open` with `YOKOHAMA_TURNSTILE: "on"` and the secret set; the Worker's hostname taken off the Access application's destinations (three PUTs' discipline: the `aud` never changes); the WAF rule enabled; WARP off on the owner's machine for the owner's readings;
3. the readings, each a dated line: the probe burst at zero invocations with its control; curl-shaped requests to `/enter`, `/api/health` and `/api/archive` answered at the edge with `cf-mitigated: challenge` at zero invocations; the owner's browser through the button into the lobby, a table and a game (the open door's name step); a fresh context arriving by a table link, the widget once, then the table; a local game to game over uploading through the widget (`archive-landed`); a finished local game with the network cut then restored, its row waiting, landing at the door's next launch; the pinned-browser update reading, which needs no login;
4. the revert: the rule DISABLED, a client build without the key, `npm run deploy` (the worker door, `off`), the secret put back, the three hostnames back on the application, the after-check, one live replay HOLDING.

**What ships live after the window**: the code with the switch `off` under Access; the rule disabled by id; the recipe in the runbook.

**Cells** (§3.1): the button with and without a key, and offline; the fetch handler over a fixture 403 with the header (the widget seam called once, the retry made, a second challenge the error); the outbox's marking and its visible-tab rule; the Worker's `/enter` in the pool under `on` with the test secret (a token verified through a stubbed siteverify, the 302; a bad token 403; the hostname mismatch 403), `on` without the secret (500 naming it), `off` (today's 302 and 405); the guard's checks 7 and 12. Only the window proves the real widget.

**Not built**: Turnstile on the NAS; a per-request token on `/api/*`; anything loaded at door load; Spin's form-gating shape.

### 2.7 The web presence (R-F12)

**What it is**: static files and page metadata, free on both hosts, for search engines and language models, with defaults that hold for any public deployment. Nothing in it is fetched by the client, precached, or scripted; nothing names an address; the leakage scan covers the new files.

- **`robots.txt`** (`web/public/`): everything allowed but `/api/`, `/enter` and `/cdn-cgi/`, so a well-behaved crawler never reaches a metered path; a `Sitemap:` line when the site's address is known.
- **`llms.txt`**, in the `llmstxt.org` shape (a title, a summary blockquote, sections of links): what the site is, that a bot game runs entirely in the browser, that multiplayer sits behind a door, the repository link, the licence with the assets carve-out named (`ASSETS.md`), and the notice. Generated at build by a small script (`web/scripts/presence.mjs`, run by `build:client` after `vite build`) from the build values, so its words agree with the door's.
- **A fifth build value, `YOKOHAMA_SITE_URL`**: the deployment's canonical address (an absolute `https://` origin, checked as the project link is). With it: `sitemap.xml` with the one page, `<link rel="canonical">` and `og:url`. Without it, none of the three, and everything else holds.
- **The page's metadata**, written into the shell at build through Vite's HTML transform in `vite.config.ts`: `<meta name="description">` from the blurb or a default sentence; Open Graph (`og:title`, `og:description`, `og:type` website, `og:image` the app's own 512 px icon — the mark is original, `web/public/icons/`) and a Twitter summary card; JSON-LD for a `WebApplication` in the games category, free, under the Apache-2.0 licence, `codeRepository` the project link when set, and no claim of affiliation anywhere. The shell's bytes change, so it is a new release, as any build is.
- **Both hosts**: the axum host serves the files by extension (`assets.rs`); `worker/static/_headers` gives `/robots.txt`, `/llms.txt` and `/sitemap.xml` the art's hour of cache. The 404 page carries `noindex`.
- **Cells**: the built shell carries each tag exactly once and its JSON-LD parses; the repository appears in it and in `llms.txt` only when the link is set; `robots.txt` names the two metered paths; the sitemap exists only with the site address; none of the four files is in the precache list; the build-values checker refuses a bad site URL naming the variable.
- **The documents**: the variable's rows in the lane document's §12 and the self-host guide's build arguments, the Dockerfile's `ARG`, the README's getting-started.
- **Not built**: no server-rendered pages, no per-route metadata for a one-page app, no analytics or tag scripts.

### 2.8 E's deferred minors (F-5)

All in, from E's close §6: the three replay-scrubber cells' timeout raised from 5 s to 10 s (they read 5.9–6.1 s under the sequence's load; green alone); the e2e wall inside the sequence (14.0 minutes against 5–7 at earlier closes) measured per spec at F's code head, the cause named if one spec carries it, fixed only if it does; `crates/yokohama-web/src/api.rs:2193`'s "five in `CLAUDE.md`" comment fixed inside F's first layer change (any edit there re-stamps the picker code, so it rides a change that re-stamps anyway); the state-1 spec's m-4–m-7 (the e2e temp directory emptied per config load; the lobby's unreachable-server card; the final asserts that cannot fail; the anonymous fallback DELETE); `README.md:273`'s "for anyone with the link" and its two long lines; the relocation script's tree `check` given the sentence that says its failing past the plan's end is by design.

### 2.9 The documents (F's Part 3)

- **`docs/web-lane.md`**: §2's counts at F's head; §8's preconditions (Turnstile and the rule as the no-Access shape, health closed, the rows E could not read still owed); §10's operations (the window's recipe, the WAF rule by id, the pinned-browser instrument); §11 F's row; §12 the five new names (`YOKOHAMA_RELEASE_FEED`, `YOKOHAMA_SITE_URL`, `YOKOHAMA_TURNSTILE_SITEKEY`, `YOKOHAMA_TURNSTILE`, `TURNSTILE_SECRET`) and the two TTLs' host column; §13 the `/enter` post arm; the new ledger section; §15 by design (no NAS marker, the TTL-without-archive refusal, the in-game click line, the door built per door state).
- **`CLAUDE.md`**: the counts in place, naming F's reading; two sentences — the door-unmetered fence (a fourth fence line beside the lane's fence) and the resume rule (the "Two facts cross into the AI lane" passage becomes three); the pointer to F's record.
- **`README.md`**: the getting-started and defaults; the self-host's sweep; the resume behaviour in one sentence; the web presence; the release line at the wrap.
- **`deploy/README.md`**: §3's build arguments (the five values); §4's variables (the TTLs); §8 rewritten; an "Updating" section with `deploy/update.sh` (fetch the tag, rebuild, swap, what players then see: the door updates itself).
- **`worker/RUNBOOK.md`**: §3's variables and secrets; §20 F's deployment record with the window, the rule's recipe and id, the probe's figures, the Turnstile secret's presence by name.
- **`deploy/RUNBOOK.md`**: §3.20 the NAS's seventeenth deployment with the TTLs and the sweep's first listing.
- **The M6.5c2 spec**: §10.13 (F's rulings, written at this spec's commit) and §10.14 (corrections found in execution, written at the close).
- **`guidingpath.md`**: F's row, and the decision-log rows (R-F4 amending R-6.5c2.10; the door-unmetered fence; Turnstile as the no-Access shape).
- **The AI lane's re-entry handover**: one line (R-F4).
- **`docs/handover.md`**: Track 3 at F; the Updated line.
- **The close**, `docs/plans/2026-09-30-web-frontend-m6.5c2f-close.md` (or the day it closes), with its own "does NOT claim" section; the memory.

## 3. Testing

### 3.1 The per-edit cells, by section (every count a reading naming its head at the close)

- §2.1: `swRegister.test.ts` — the interval under fake timers, the focus re-check, the door's auto-apply (once; not on a first install; not off the door), the line's click; the e2e over two releases of the local binary (the game continues).
- §2.2: `releaseFeed.test.ts` — the five feed outcomes, the four link shapes, the hourly throttle under a broken storage; `FrontDoor.test.tsx`'s zero-server-request cell narrowed to same-origin.
- §2.3: `games.rs` — the flipped fixture (`lookahead-v1` opens and moves), knob-mismatch and drift refusals with their reasons, the finished foreign game; `bots.rs` — the predicate; `explain.rs` — `why` by replay for a runnable non-production bot; the regenerated corpus's diff read; the browser list.
- §2.4: `sweep.rs` — the pure decision per rule (unset; the TTL; `open` at the shorter; `dealing` at the TTL; the day boundary); `games.rs` — an ended finished game not put twice, a mid-play game under `abandoned/`, an open table archiving nothing, a failed put keeping the table; `main.rs`'s refusals (a bad value; the TTL without an archive); the health fields; the store's `last_written` on both stores.
- §2.5: `deploy-guard.test.mjs` — checks 10, 11 and 12 over committed files and mutants; the precache exclusion cell; the axum 404 cell; the ledger's route-coverage cell in `lane_document.rs`; the e2e fence cell.
- §2.6: `FrontDoor.test.tsx` — the button's three states; `api.test.ts` — the handler over the fixture 403; `outbox.test.ts` — the marking and the visible-tab rule; the pool — `/enter` under `on`, `off`, a bad token, a hostname mismatch, a missing secret; `config.unit.test.ts` — the var; `deploy-guard.test.mjs` — checks 7 and 12.
- §2.7: `build-values.node.test.ts` — the site URL; a cell over the built shell's tags and JSON-LD; `presence.node.test.ts` — the files' content with and without each value; the precache exclusion.
- §2.8: the raised timeouts; the state-1 spec's four fixes, each with its own assert.

### 3.2 The sequence

The ONE recorded sequence at the code head (R-6.5c2.13): the debug suites and the lint gate, the web lane's suites in their order rules (`docs/web-lane.md` §2), the fifteen release gates as one uninterrupted roster with its wall beside E's 2,881 s on the Mac; the per-spec e2e walls recorded (§2.8).

### 3.3 The live readings (Part 2; R-F7)

In order, each a dated section of the runbooks with the quota guard before it: both deploys (the Worker's F release `off` under Access; the NAS with the TTLs after the scratch-container reading); the update reading on both hosts with the pinned-browser instrument (no click on the door; the line after a focus change with a game open); the NAS sweep's first listing; the window (§2.6, its readings including the probe); the revert and one live replay holding; the tally figures for the ledger; the screenshots re-cut where a screen changed (the release notice, the update line, the widget).

### 3.4 The reviews

Every task reviewed by Opus on an EXPORT (`git archive`), never in the implementer's tree; a fix as a NEW commit; the Fable whole-branch review with the vacuity lens first (the top yield of every cycle is a sentence the cycle wrote about itself); one fix wave; scoped re-reviews; a mutation battery over the new cells (the guard's checks, the resume predicate, the sweep's decision, the fetch handler, the feed comparison).

## 4. The order of work (R-F11)

**Part 1, the code**, subagent-driven from the plan with a ledger, one implementer in the tree at a time; the tier marked in each task's title (Sonnet 5.5 where the brief carries the judgment already made, Opus where judgment is needed, every reviewer Opus, Fable orchestrating and reviewing the whole branch). The tasks, in an order the plan may refine: (1) the update path and the guard's cache check; (2) the release notice; (3) the resume rule and the corpus; (4) the sweep; (5) the 404 page, the guard's checks 11 and 12, the presence files, health into the protected list; (6) Turnstile's client half (the button, the fetch handler, the outbox); (7) Turnstile's Worker half (the var, the secret, `POST /enter`, siteverify) and the pool; (8) the ledger's section and its cell, the fence cell, the probe instrument; (9) E's minors; then the whole-branch review, the fix wave, the battery and the sequence.

**Part 2, the live readings** (§3.3).

**Part 3, the documents** (§2.9) and the close.

**The wrap, on the owner's word** (R-F7; E's recipe): the merge #31 → #43 → F's PR bottom-up, one at a time, each base moved to master and its commits read before its merge; master pulled and the fences read against it; the annotated tag `v0.6.0` (`Yokohama CLI v0.6.0 — <headline>`) and `release.yml` read to green; `test.yml` and `docker.yml` re-enabled (R-6.5c2.31); one documents commit on master (the memory, the board's Updated line, the README's release line, `CLAUDE.md`'s pointer). Then the project refocuses on the computer player from `docs/plans/2026-09-11-ai-lane-reentry-handover.md`.

## 5. The fences

Every commit: the five fenced crates (`yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge`, `gen-data`) byte-identical to `c06c4b36`; the CLI touched only by behaviour-preserving seam changes (F expects none); `SEMANTICS_REV` 7; no bot behaviour moved (§2.3 changes which recorded bot the layer seats, not what any bot does); **the door unmetered** (§2.5) as a fence of its own, pinned by the guard, the e2e cell and the reading; the corpus moved only by §2.3 and §2.4's health fields if a corpus host sets them, its diff read.

## 6. What F does NOT do

- No `site.json`, no `release.json`, no forced reload in a game, no words fetched at run time (R-F1, R-F3).
- No server-side release check, no notification of anyone, no self-updating server; the image stays unpublished (`ASSETS.md`).
- No compatibility shim for an older `SEMANTICS_REV`; no choosing an older bot at a new table; no CLI change (R-F4).
- No marker on the self-host (R-F10); no dry-run variable for the sweep (R-F9).
- No R2 dropbox for saves (R-F8).
- No Turnstile on the NAS; no per-request token on `/api/*`; no script at door load; Spin's form-gating shape (R-F6).
- No per-route metadata, no analytics, no tag scripts (R-F12).
- No version bump (R-F13).
- The window is vetted and REVERTED; the fully open end state waits for the public repository, as E ruled (R-6.5c2.39).

## 7. Risks the plan must answer

1. **A non-navigation miss may still invoke the Worker with `404-page` set.** The probe reads it (§2.5); the close weighs the remaining lever. The plan orders the probe before any claim in the ledger.
2. **The widget's real behaviour is provable only headed and only in the window** (§2.6). The plan carries the always-passing test pair for every automated cell and names the window's readings as the proof.
3. **The Access application's `aud` must not change across the window's PUTs** (E's recipe); a changed `aud` orphans every `ACCESS_AUD`. Read back after every PUT.
4. **`/api/health` behind Access breaks any unauthenticated uptime probe** the owner runs. The plan asks, in its first live block, and the smoke carries the credential.
5. **The resume rule's corpus regeneration** must not silently drop the foreign refusal from the samples: the fixtures move to a genuinely foreign identity, and the diff is read for the refusal's presence.
6. **The sweep's first pass on the real NAS ends every idle table at once** by design; the listing before the deletes is the record, the archive holds the copies, and the scratch-container reading comes first.
7. **The 15-minute re-check on a device with many tabs** is one 29 KB request per tab per 15 minutes, static; noted in the ledger, not gated.
8. **The feed's anonymous limit** (60 an hour per address) is respected by the once-per-page-life plus hourly rule; a shared address behind one NAT could still meet it, and the outcome is "no notice", never an error.
9. **Plan size**: if the plan passes what one agent reads whole, it splits into two cycles at drafting (R-F11).

## 8. Process

Subagent-driven development from the plan with a gitignored ledger under `.superpowers/sdd/`; one implementer in the tree at a time; a subagent that backgrounds a long run polls its output file in the foreground and arms no Monitor; every agent writes its full report to a file and replies with the path; Opus for every reviewer, a fix as a NEW commit, never an amend of a reported SHA; the deletion rule (no `rm -rf`, `rm -r` or bulk `rm -f` in any dispatch; exports and screenshot trees left in place, named in the report); no document spells the owner's address; no secret value in any text (the Turnstile secret and the site key included — the key is public but stays out of the documents); every invisible code point in code escaped; the fences (§5); the quota guard before every live block (80,000 a day, the owner's line); the live tiers and the screenshot generator need the DEV DOOR on; every count a reading that names its head; the deploys, the push, the PR and the window at the orchestrator's discretion, the merge and the tag on the owner's word (R-F7).

## 9. Review record

- **The self-review at writing (2026-09-30, the orchestrator):** no placeholder; the three settings of §2.6 and the guard's check 12 made consistent with "one build per door state" (an earlier draft allowed `off` with a key in the bundle, which would have rendered a widget under Access); the feed's three states spelled (`unset` derives, a URL is that feed, `off` is none) so empty and unset cannot be confused; §2.5's health change given its uptime-probe risk (§7 item 4).
- **The owner's review:** pending.
