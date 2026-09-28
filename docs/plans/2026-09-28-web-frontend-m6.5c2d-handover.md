# M6.5c2 cycle D — the wrap-up and polish: the handover (the ENTRY for cycle D's planning session)

Written 2026-09-28 at cycle C's close, every fact read off the tree at the reviewed head `ea4f174d` and the close commit
over it, never off the plan. Read in this order: `CLAUDE.md` → `docs/handover.md` (the board) → this file → the close
`docs/plans/2026-09-28-web-frontend-m6.5c2c-close.md` (§7 first) → the spec `docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md`
§4 row D and §10.7–§10.8 → the M6.5c close's §7 and §6.3.

## 1. Where cycle C left the tree

- **Branch** `web-room-m6.5c2c` on `web-room-m6.5c2b`; **the reviewed code head `ea4f174d`** (28 commits over `3ed27f07`);
  the close's documents commit on top; **PR #41** (`web-room-m6.5c2c` → `web-room-m6.5c2b`), pushed under the standing
  pre-authorization. The stack is #31 → #41, nothing merged, nothing tagged (R-6.5c2.20: D's).
- **CI's first runs of this branch** fired at the push and the pull request (run ids 36460015068 `test` on the push;
  36460020190 `test` and 36460020302 `docker` on the pull request — `docker.yml`'s `push` trigger is master-only, so the
  cold wall is the pull request's, R-C21). Their readings are the close's §8 addendum; if any is red at D's start, the fix
  is an adjustment in D, not a design.
- **Every suite's reading at `ea4f174d`, in ONE sequence** (the close §2): core 1577 + 29 · cli 851 + 2 · oracle 191 ·
  difftest 52 + 5 · eval_probe 214 · solver 132 · bridge 81 · web **343** (lib 324 + pin 4 + gate 2 + wire 13) · wasm 7 ·
  worker 75 · identity 8 · clippy clean · `cargo doc -p yokohama-worker` 0 warnings · the four bridge node suites ·
  worker `test:scripts` 387 in 19 · the pool 217 in 11 (two projects) · web `npm test` 1220 in 64 · `test:wasm` 114 in 9 ·
  the e2e 68 in 13 (67 of 68 in the sequence, 68 alone — §2.1) · the fifteen release gates 461 s on the Mac.
- **Both fence checks:** `git diff --stat 3ed27f07 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest
  crates/yokohama-bridge crates/gen-data crates/yokohama-cli` empty; `SEMANTICS_REV` 7; the layer's `src` in six named
  places; the corpus in Task 9's three files by the version alone; 101 files under `web/src/wire/`.
- **What is LIVE (2026-09-28):** the Worker `yokohama-web` at `yokohama.moore.nyc`, **version `36038818-4aa8-4544-961a-0bec2674782c`**
  (the twenty-third deployment; the `worker` door; `DEV_DOOR_CLIENT_ID` and `YOKOHAMA_WEB_ADMINS` the two SECRETS; the
  dev door ON for the live tiers; `worker/RUNBOOK.md` §15–§17); the NAS `yokohama-web:0.6.0-m6.5c2` at
  `yokohama-nas.moore.nyc` on **`cf-access`** (container `10feaa56a4fe…`, `deploy/RUNBOOK.md` §3.17); both serving the
  release `yokohama-release-605fe69f17100c46`; the bucket's `test/` rule `expire-test-after-30-days`.
- **The live tiers' ending state:** the corpus replay HOLDS twice (2,209/86); `joins_by_link` live; `two_browsers` red
  once (one unexplained watcher socket close, the tail disconnected at that moment) then a 1,795-move game with the
  watcher on ONE stream and 0 `/events` asks; the headed exit's four hibernations woken at 9–12 ms of CPU; rows read per
  move ~56 live against M6.5c's ~470, requests ~13.8 a move unchanged. The day's quota ended at 43,334 requests / 12,443
  rows written, under the 80,000 line.

## 2. What cycle D builds, and its raw material

Spec §4 row D (revision 2.7): **the lane document and the top-level refresh; the seven screenshots off the DEPLOYED
site; polish items 5–9 and what C's pictures show; the phones; the investigation of what remains; then the merge of the
stack and the `v0.6.0` tag**, and whatever overflowed C (§6 below). Raw material, each to be read against the tree, not
trusted:

- **The lane document** `docs/web-lane.md`: the draft is D8's retired Task 27, `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/drafts/D8-tasks-24-28.md:1911-2453`
  (eleven sections; the `CLAUDE.md` move script that reduces the web section to a pointer; the board's rewrite). Correct it
  for what C changed: the hosts (both on `0.6.0`, the doors), the counts (§1 above), `RUNBOOK` §6 → `worker/RUNBOOK.md`,
  the deploy guard's door rules, the secret, the lifecycle rule, the rows-read figures, the traps C paid for (§6).
- **The seven screenshots** (D8 Task 28's G, `drafts/D8-tasks-24-28.md:2800-2830`): recaptured live on the twenty-third
  deployment with three `.test` identities through the credential, into `worker/docs/img/` under the same names; no frame
  may show the owner's address; `worker/README.md` §1.4's captions follow.
- **Polish 5–9 and C's pictures:** the gitignored ledger `.superpowers/sdd/2026-09-28-web-frontend-m6.5c2c-plan/` holds
  `task-10-look/` (the Finished group at 1440 and 390, the closed fold, the padlock row), `task-11-look/` (thirteen
  screens with a README beside the mocks) and `fix-wave-look/` (the re-cut fold); the owner's-eye calls are the close's
  §6.3 item 4.
- **The phones** (R-6.5c2.27): the iPhone exit and the Android handset against both hosts at `0.6.0`; the Device Hub iOS
  simulator (`mcp__mobile__*`) is the option the owner named for D's plan.
- **The investigation of what remains** (the owner's "straw poll"): the LEAVE lists of every close since M6.1, the M6.5c
  close's §7 and §6.3, cycle C's close §6–§7, the public door's preconditions (Access on both hosts; the path-scoped
  application built and not deployed; the dev door OFF and its secret deleted; the art's publication; requests per move
  as the free plan's first limit), and the AI lane's re-entry pointer `docs/plans/2026-09-11-ai-lane-reentry-handover.md`.
- **The merge and the tag** (D8 Task 28's I.2–I.3, `drafts/D8-tasks-24-28.md:2929-2990`, corrected for the stack's
  length): `gh pr merge 31 --merge`, then for each of #32 … #41 and D's #42 in order `gh pr edit N --base master`,
  `gh pr checks N --watch` (a retargeted PR's checks take a moment to register; re-run until they show), `gh pr merge N
  --merge` — each iteration its own step, never one unattended loop; `delete_branch_on_merge` is false, so nothing
  retargets on its own. Then `git tag -a v0.6.0 … origin/master`, `git push origin v0.6.0`, `release.yml` read to green
  (three OS jobs and `publish`; the release carries three archives, `yokohama-cli-guide.html` and `SHA256SUMS`). The
  first merge onto master also fires `docker.yml`'s `push` trigger — read its wall.

## 3. The rules D carries

- **The standing set:** SDD for every task with Opus the floor for implementers and reviewers, Fable orchestrating and
  reviewing the whole branch; every agent's full report to a file, its reply the path and three lines; never a follow-up
  on an implementer's DONE without reading HEAD; a fix is a NEW commit; while an implementer works nothing runs in its
  tree; reviewers' and the battery's mutants in an EXPORT; Playwright runs sequenced and polled, never a Monitor or an
  event wait; the owner's address in no document (R-6.5c2.16; the scrub's counts read 0); every INVISIBLE non-ASCII code
  point escaped; the fences at every commit.
- **The owner's authorization:** the push and the PR were pre-authorized for C and are SPENT; **D's merge and tag need the
  owner's word again** (R-6.5c2.17, .19 as amended), each step separately.
- **No deploy in D unless a polish fix needs one.** If one does: the Worker by `worker/RUNBOOK.md` §1–§2 (`npm run build`
  in `web/` then `worker/`, the guard, `npm run deploy`; a deploy restarts every object and drops every open picker whose
  code changed; between games); the NAS by `deploy/RUNBOOK.md` §1–§3 with §3.17's shape (the token in the portainer-mcp
  checkout's `.dev.vars`; the context from a directory of its own; `.dev.vars` 404 before the POST; the swap's `Env`
  exactly as §3.17). The live tiers first check WARP (a 302 "Please authenticate" is the owner's), then the quota, then
  ONE run at a time with a FRESH `wrangler tail` attached — and read the tail's connection warnings before believing its
  counts (the close §3.8).
- **The documents' rules stand:** dated artifacts frozen; corrections forward (the spec's §10.8 shape); `CLAUDE.md`
  current-state only — D's `web-lane.md` move reduces its web section to a pointer.

## 4. What cycle C does NOT claim (the close §7, one line each)

No public door; no open-door Worker deployed (the `open` door is the pool's, the local e2e's and the smoke's) · CI
unobserved at the reviewed head until PR #41's runs · the NAS's `cf-access` read once, as the owner; no game played on it;
`run_id` and the mirror's `bot` on no document there yet · no not-over foreign-bot game on either host (the refusal is
cell-read) · the livelock bound measured over recorded seeds, not proven · overlapping uploads' pair consistency
best-effort; a refused stamp after a `landed` upload retries hourly until Abandon or expiry · the refinish rule proven in
the pool, no live conflict · requests per move unchanged (~13.8); rows read ~56 a move on ONE harness game · one
unexplained watcher socket close beside one clean re-run · `picker-dropped` not observed live · the memo's `SqlStorage`
premise a throwaway cell's · the L-5 rule stated in the two Worker replays, not the browser's · the bot's version by
ingestion, no key changed · the pictures Chromium's headless plus one look at the NAS lobby in the owner's browser; WebKit
and the phones unseen · the `test/` rule's first expiry is October's reading · the address in the git history · the
screenshots, the lane document, the top-level refresh, the merge and the tag are D's.

## 5. Every ruling taken in cycle C's ledger, one line each (2026-09-28; the costs are the close §5)

R-C1 thirteen tasks · R-C2 every diff's base `3ed27f07` · R-C3 L-9's helpers in `live.ts`, L-5 beside the replay ·
R-C4 L-8 is `hasDoc` · R-C5 the versioned refusal sentence's arms · R-C6 the Finished group's shape · R-C7 its witness
on `plays_a_whole_game` · R-C8 the `events` two-defect line · R-C9 `docker.yml` on web-lane inputs, a fifteen-minute
cold fallback · R-C10 the open door's 500 on a refused claim at `/api/archive`, accepted · R-C11 the door's value
trimmed and exact · R-C12 the `open-door` feature inert, kept as a declaration · R-C13 `stamp-refused`'s shape · R-C14
the no-creator arm reachable on a hand-made document, accepted (class amended) · R-C15 the foreign-bot site is
`games.rs::live_handle` · R-C16 `undo_allowed` through the pair · R-C17 the re-point cell; `({controller})` · R-C18 a
finished re-point's drift warning left · R-C19 `Opening = { page, id }` · R-C20 the browser replay's L-5 hole left ·
R-C21 the cold set; the wall from PR #41's run (amended twice) · R-C22 four cells red at 0.6.0 · R-C23 six places
beyond the brief's letter · R-C24 Abandon's own strip · R-C25 `failed` keyed on failure · R-C26 the scrub's rule outside
the plan's quote · R-C27 the review's triage adopted plus the two thrown sentences · R-C28 the loopback paragraph's true
limit · R-C29 the sequence first, the battery after · R-C30 the red live run re-run once, alone, with the tail attached.

## 6. Cycle D's standing items this cycle created or found

1. **The `plays_locally` quota cell's premise** (the close §2.1): assert the click APPLIED a move, or step through a
   sub-picker; red in two closes' sequences, green alone each time.
2. **The `two_browsers` first-run probe** (§3.4): read a fresh tail FIRST on D's next live run; a second unexplained close
   is a finding.
3. **`wrangler tail` drops and samples under a burst** (§3.8): D's live readings may go to the Workers Observability
   query API (`worker/RUNBOOK.md` §7 names it); a tail's count is never a whole.
4. **The figures to transcribe into the root `Cargo.toml`** (the ONE place): the Worker module 1,258,106 B and the
   browser module 1,197,069 B at `ea4f174d` under `wasm-opt` 133.
5. **The twelve remaining `RUNBOOK §6.x` citations** in code and configuration (`test.yml:30,40`, `deploy/Dockerfile:12`,
   `worker/wrangler.jsonc:61,86,91`, `read-deploy.mjs:9`, `read-deploy.test.mjs:53`, `exit_game.spec.ts:48,73` and two
   more): one sweep, one commit, naming the count it starts from.
6. **The polish calls that want the owner's eye:** the closed Finished fold's cue at 1×; the lobby's density at 390 (a
   hit box around a small Abandon); the crumbs wrapping under the pill at 390; a "connecting" word on a first visit.
7. **The deferred minors** of Tasks 2–12 as the close §6.2 lists them (comments, docs, small cells; the two drops); the
   34 rustdoc warnings in `yokohama-web`; the two once-flaky jsdom cells; `record.rs:181`'s sentence (FENCED — the AI
   lane's, never a web commit).
8. **R-C10, R-C14, R-C18, R-C20** as recorded limits: a mask in `client.rs` for a finished re-point's warning; a third
   statement of L-5 for the browser replay; the open door's 500 on a refused claim; the no-creator arm's class in its
   docs.
9. **The NAS swap as one script** (E.6 read 25.3 s of downtime across four proxy calls; §3.16's script read 8.45 s), and
   the Portainer token's places named in the recipe (the Keychain item was absent this session).
10. **The public door's preconditions** (§4 above) and the merge order — the owner's.
11. **CI refocusing — an OWNER DIRECTION given at cycle C's close (2026-09-28), D's item:** *"the CI checks we're doing
    here seem excessive — we wouldn't want such heavy handed CI for a segment in the project we're nearly ready to freeze
    as the focus of the project will return to the core crates and computer player, having ~ten minutes each for several
    tasks in github CI when we're running the test sequences locally doesn't seem appropriate … unless we've modified
    something in yokohama-web I don't see why we would want to have all of this in CI as changes to the computer player
    wouldn't impact the yokohama-web module, and if that somehow were to be the case, we would action that as part of the
    dev cycle."* The shape that follows: path-filter `test.yml`'s three web jobs (`web-server`, `worker`, `web-client`) to
    the web lane's inputs as `docker.yml` already is (`crates/yokohama-web/**`, `crates/yokohama-wasm/**`,
    `crates/yokohama-worker/**`, `web/**`, `worker/**`, the workflows and their scripts), keep `workflow_dispatch`, and
    leave the core `test` job as the only job a core-only push pays. The one coupling to state in the change, not to
    fear: the committed wire corpus pins the engine's wire-visible words and `rules_hash`, so a core change that moved
    them would fail `cargo test -p yokohama-web` — which the per-edit rule already runs locally after every task
    (`CLAUDE.md` Commands), and which a `SEMANTICS_REV` bump (a STOP) would make its author run in any case. Cycle C's
    first CI readings (the close §8) are the last full-run figures before the filter.
