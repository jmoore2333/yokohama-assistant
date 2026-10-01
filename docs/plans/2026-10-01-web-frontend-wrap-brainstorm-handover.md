# The web lane's wrap — the handover into its brainstorm session (2026-10-01)

**What this is.** Cycle F "the clean finish" closed on 2026-10-01 (`docs/plans/2026-10-01-web-frontend-m6.5c2f-close.md`) and, by the owner's ruling R-OWN-F2, ended at the push and F's PR: *"task 15 we should plan to push and PR, we'll handle the merge stack after a review outside this session of the stack, leave the github workflows disabled, and provide a handover for a brainstorming session for the final wrap."* This document is that handover — what the wrap session decides, with the evidence each decision rests on, and what it does afterwards. It is the ENTRY to the wrap; the close is the record; `docs/web-lane.md` is the lane's operating document.

## 1. Where F left the tree and the hosts

- The branch `web-room-m6.5c2f` on top of `web-room-m6.5c2e` (`15de8117`): the code head `a954ca79` + Task 10.E (`d6f9885f`, `7670d5fa`) + the documents (Tasks 12–15); the final reviewed head is named in the close §2. The stack of PRs: **#31 → #43 → F's PR** (the number in the close §8), each PR's base the branch below it, none merged; the workflows `test.yml` and `docker.yml` DISABLED on GitHub since E; `v0.6.0` untagged (the version in the tree is `0.6.0`, R-F13).
- **Both hosts run F's release at `0.6.0` behind Cloudflare Access**: the Worker's thirty-first deployment `703e60d5` (`YOKOHAMA_WEB_IDENTITY worker`, `YOKOHAMA_DEV_DOOR on`, `YOKOHAMA_TURNSTILE off`, the keyless build `yokohama-release-1088dec85fe623b8`, the secrets `DEV_DOOR_CLIENT_ID` and `YOKOHAMA_WEB_ADMINS`); the NAS `yokohama-web:0.6.0-m6.5c2f` (deployment 17, `cf-access`, TTLs 90/7, the archive directory, the hourly sweep). The Access application `yokohama` has its three hostnames; its `aud` never changed.
- **In the zone `moore.nyc`:** the WAF custom rule `yokohama — the no-Access door's edge check (F)` (id `dc4c5d3e986144649617cbbaea58cd8a`) at position 1 of the `http_request_firewall_custom` ruleset `27fcc0c7398f42e3abbf549a917e5fa7`, DISABLED (version 16) — its expression is `node worker/scripts/edge-rule.mjs`'s output over the assembled tree, verified equal by `--verify` at the window; the phase holds five of the Free plan's five rules (one duplicate skip rule was merged into another to free the slot); the Turnstile widget `moore.nyc (Spin)` (managed, pre-clearance `managed`); Challenge Passage ONE WEEK (R-OWN-F3); Bot Fight Mode ON; managed robots.txt OFF.

## 2. What the window read (2026-10-01 13:13:51–13:57:31Z; the close §3; `docs/web-lane.md` §10)

1. **The fence works and costs nothing:** 800 scanner-shaped requests in four minutes → 0 Worker invocations; from a client Bot Fight Mode trusts, every sampled challenge was the rule's own. A miss under an allowed prefix is the asset layer's cached 404 page in every shape, 0 invocations.
2. **A person is cleared in about a second with no click:** the door's button pre-clears before the first metered call; a link arrival meets the predicate → overlay → one retry in 1.3 s; a challenged upload reads "waiting for a human check" and lands on "Check now" through an overlay of under a second; the WebSocket upgrade passes under the clearance; the no-click update observed live (7 s, no click).
3. **W-1 — Bot Fight Mode, not the rule, challenges a bare client on the allowed non-static paths** (`/`, `/sitemap.xml`, `/build-values.json`, `/manifest.webmanifest`, `/shell.bin`, `/index.html`, `/404.html`); the smoke's `--live` control passes only over WARP.
4. **W-2 explained / W-3 — a controlled browser without a clearance cannot install a new release while Bot Fight Mode is on:** the service worker's precache fetch of `/art/manifest.json` is challenged (read four times over 18 minutes in the pinned browser); its navigations come from the cached shell, so Bot Fight Mode's JavaScript detection never re-issues the clearance; only a metered call meeting the rule's widget, or an HTML response the edge answers, restores it. The door shows `Could not prepare to play offline — /art/manifest.json: answered 403. Try again`. F changed no code for it.

## 3. The decisions the wrap makes (each with the evidence and the orchestrator's reading; the owner rules)

1. **Bot Fight Mode under the no-Access door.** The rule alone refuses scanners at zero cost (the isolated bursts); Bot Fight Mode adds W-1 (a bare client challenged on the door's own files; crawlers that are not verified bots get 403 on `/`) and W-3 (a lapsed visitor's install fails). On the Free plan Bot Fight Mode takes no skip rule, so the rule's allow-list cannot exempt the release's files from it. *Reading: turn Bot Fight Mode OFF when the door opens, and keep the rule — or keep both and accept W-3 with the door's "Try again". A third path: give the precache's non-static entries static extensions (a rename of `/art/manifest.json`, `/shell.bin`, `/build-values.json`, `/manifest.webmanifest` — the last two are standards' names) — measured nowhere.*
2. **Challenge Passage's lived value** (one week today): with the rule's widget and Bot Fight Mode's clearance both keyed to it, a longer value is fewer overlays and a longer W-3 shadow; `challenge_ttl` takes up to a year.
3. **The smoke's positive control** (D-2): leave the runbook's "over WARP" sentence, or let the control accept Bot Fight Mode's challenge on the sitemap row (a code change the wrap would review).
4. **The merge and the tag:** #31 → #43 → F's PR bottom-up, one at a time (E's close §8 / the plan's Task 15.G recipe: each PR's base moved to master, its commits read, then merged; the fences read against master after F's); then `v0.6.0`, `release.yml` to green, the three assets; then `test.yml` and `docker.yml` re-enabled; the memory, the board, the README's release line and CLAUDE.md's pointer in one documents commit on master.
5. **The end state's switch:** the public door is a deploy (`npm run deploy:open` with a keyed build), a rule enable (by id) and an Access PUT, in the order the window ran (Access off LAST, back FIRST); the recipe is `docs/web-lane.md` §10 and `worker/RUNBOOK.md` §20. Nothing opens until the wrap says so.
6. **After the wrap:** the project refocuses on the computer player — `docs/plans/2026-09-11-ai-lane-reentry-handover.md`, the board's Track 1 — with the V5 ship's one CLI line owed (item 12 there).

## 4. The rules the wrap carries

The lane's fence (the five fenced crates byte-identical, the CLI by behaviour-preserving seams only, `SEMANTICS_REV` 7, no bot behaviour moved, the door unmetered); a keyed build is the open door (the guard's check 12 and the self-host's start refusal); the release-version rule (`docs/web-lane.md` §7); no secret and no address in any document; the site key is public by Cloudflare's design but is in no document; nothing metered at the door; the workflows stay dark until the merge.

## 5. What F does NOT claim (the close §7, in one line each)

The fully open end state; a public load; W-1's first-visitor case in isolation; W-3's fix; the no-click update on the NAS at F and the stale second tab's line on any host; the visible launch's retry of a challenged row; the widget's interactive mode; the feed's notice live; the handsets, rows 108 and 119, the exit spec, a cookie-keyed rate limit; the V5 ship's CLI line; the merge and the tag.

## 6. Every ruling of F

R-F1–R-F13 (the M6.5c2 spec §10.13), R-FP1–R-FP15 (the plan), the owner's R-OWN-F1–F3 and the ledger's rulings, each with its cost: the close §5.

## 7. The brainstorm's items (owner and orchestrator)

- §3's six decisions, in that order; the owner's words on each.
- Whether the wrap is one session (the merge, the tag, the switch) or two (the decisions first, the switch at leisure).
- What the first public week measures (the quota guard's daily read; the firewall events' sources; the door's install failures if Bot Fight Mode stays on).
- The refresh after the wrap: CLAUDE.md's web pointers to the merged tree; the board's Track 3 closed; guidingpath's web-lane row FINISHED and merged.
