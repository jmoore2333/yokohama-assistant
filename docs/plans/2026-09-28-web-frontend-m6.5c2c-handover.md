# M6.5c2 cycle C — the handover from cycle B (the entry for cycle C's planning session)

**What this is.** Cycle B ("the screens") of M6.5c2 is closed on branch `web-room-m6.5c2b`. This document is the ENTRY
for the session that plans cycle C ("the hosts and the close"): a fresh orchestrator writes cycle C's plan from it, the
spec's §4 row C, the drafts D6–D8 and the standing items below, against this branch's REAL tree. Every fact here was
read off the tree at the reviewed head, never off cycle B's plan. Authority order is unchanged: `guidingpath.md`, then
`CLAUDE.md`, then the spec (`docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md`, revision 2.6), then this note.

## 1. Where cycle B left the tree

- **Branch** `web-room-m6.5c2b`, stacked on `web-room-m6.5c2a` (code base `f2a3dd61`); **reviewed head `3ed27f07`**
  (the plan's commit, nine tasks' green commits and their fix commits, then the close's fix wave); **PR #40**
  (https://github.com/jmoore2333/yokohama-assistant/pull/40) against `web-room-m6.5c2a`. Cycle C's branch is `web-room-m6.5c2c`, on `-b`.
- **The suites at the reviewed head** (Darwin arm64, macOS 27.0, this workstation; node v26.10.0; rustc 1.97.1; every count a reading, not a constant):
  - `cargo test -p yokohama-web` **339** = lib 320 + `browser_played_pin` **4** (Task 1: the `ClientState.warnings`
    field witness, R-A19 closed) + `play_a_whole_game` 2 + `wire_fixtures` 13; `cargo test -p yokohama-wasm` **7**;
    `cargo test -p yokohama-worker` **64**; `cargo clippy --workspace --all-targets -- -D warnings` clean;
  - `web/`: `npm run typecheck` clean; `npm test` **1195 in 64** (was 1067 in 56); `npm run test:wasm` **104 in 9** (was
    95 in 7); `npm run e2e` **68 in 13 (5.7 m)** (was 61 in 11: + the door, the name step, the lobby, the four forms cells);
  - `worker/`: `npm run typecheck` clean; `npm run test:scripts` **352 in 18**; the pool `npm test` **197 in 9**.
- **Both fences** (`git diff --stat f2a3dd61 -- <the five crates + the CLI>` and `… -- crates/yokohama-web/src
  crates/yokohama-wasm crates/yokohama-worker worker/`) printed nothing at every commit; the corpus (`web/src/wire/`)
  is byte-identical to A's; `SEMANTICS_REV` 7; the workspace version `0.5.0` (the bump is C's, R-6.5c2.19).
- **The e2e runs**, one per task and the close's: Task 3 62 (7.3 m; its fix 6.2 m), Task 4 63 (4.9 m; fix 6.3 m), Task 5 64 (7.3 m; fix 6.6 m), Task 6 68 (3.8 m; fix 5.9 m), Task 7 68 (7.1 m), Task 8 68 (7.2 m; fix 5.0 m), Task 9 68 (6.7 m), the wave 68 (7.6 m), the close 68 in 13 (5.7 m). `plays_locally`'s quota cell is a KNOWN flake (a
  seed whose first option opens a sub-picker); every task that met it passed it alone and then the full suite.

## 2. The client as it IS — what cycle C ships and deploys

- **The screens by file, and the mock each answers to:** `FrontDoor.tsx` + `OnThisDevice.tsx` + `BackupControls.tsx`
  (mock 01, 06 for state E, 11); `Door.tsx` + `SignIn.tsx` (03, 04, 05); `Lobby.tsx` + `Band.tsx` + `Chips.tsx` (02,
  10, 11); `NewGame.tsx` (07, 11); `OpenTable.tsx` + `SeatCard.tsx` + `Unlock.tsx` + `LockScreen.tsx` (08, 09, 11);
  `Dashboard.tsx`'s Players panel (10). The shared pieces are `BoardMark.tsx`, `Band.tsx`, `Chips.tsx`, `SeatCard.tsx`
  and the `styles.css` block "M6.5c2: the setup screens' shared parts"; the rules are `nameRule.ts` (`User::parse`,
  pinned two-way over `identity_cases.json`'s 41 rows and to `\p{White_Space}`) and `seedRule.ts` (`WireSeed`, pinned
  over `seed_cases.json`'s 26 rows). The pictures of every screen beside its mock are in the gitignored ledger
  (`.superpowers/sdd/2026-09-27-web-frontend-m6.5c2b-plan/task-<n>-look/`, with READMEs).
- **The accessible names the live specs and the exit spec now READ** — C's live tier and headed exit run against
  these words: the name step's heading "Choose a name" (`level: 3`), its field "Name", "Continue"; the corner's
  "Playing as <name>" with **"switch player"** under `open` and **"change name"** behind an edge door, and a
  `.identity-pill` at phone width; the lobby's lists **"Your tables" / "Open tables" / "Being played"** with row
  buttons named `"<label>: <table name>"` ("Play: …", "Copy link: …" → "Copied: …", "Take a seat: …", "Spectate: …",
  "View: …") and `span.stranded` "can't continue here"; the forms' **"New table"** (a heading at `level: 3`; the band's
  last crumb is the `h2`), "New game against the bot", the seat comboboxes `"<Colour> seat"`, `"<Colour> player's
  name"` / `"…address"`, the radio groups "Players" / "First player", "Ask for a password to join or spectate", "Table
  password", "Seed" under "Advanced", "Deal" / "Open table" / "Cancel"; the open table's **"Join as <Colour>"** (one
  free seat) / **"Sit here as <Colour>"** (several), "Invite link", "Copy link" → "Copied", "Table password"; the
  unlock face's **"Spectate"** and "Table password"; the banner's **"You are spectating"** and the chat marker
  **" · spectating"**; the door's "New game", "Go to the lobby" (an `<a href="/enter">`), "Games on this device",
  "Will not open here". The wire's `watching` / `watcher` / `can_watch` are protocol keys and unchanged.
- **`nameThisBrowser(page, name)`** (`web/e2e/harness.ts`) writes `localStorage["yokohama.display_name"]`; the
  claimed identity under `open` is `localStorage["yokohama.claimed_user"]` (`api.ts`'s `CLAIMED_USER_KEY` /
  `DISPLAY_NAME_KEY`); a name typed at the name step stores the KEY (`ben castle`) and the typed form (`Ben Castle`)
  BEFORE it re-asks `/api/me`.
- **`YOKOHAMA_PROJECT_URL`** is a BUILD variable: `web/scripts/project-url.mjs` checks it (`https://`/`http://`, no
  whitespace or control character, ≤ 200 characters; unset or empty draws no link; a bad value fails `vite build`
  naming the variable) and `vite.config.ts` defines `__YOKOHAMA_PROJECT_URL__`. The Worker's `cd web && npm run build`
  takes it from the environment; `deploy/Dockerfile` has `ARG YOKOHAMA_PROJECT_URL` above `RUN npm run build:client`.
  **C's RUNBOOK recipe names the value used.**
- **`web/src/wire/production_bot.json`** (`{"identity": "v4-step9-draft1", "label": "V4"}`) is the ONE place the
  door's label comes from (imported directly); a re-point regenerates it through `wire_fixtures`.
- **The dev proxy** forwards WebSocket upgrades (`ws: true` on `/api`); the probe after a self-made close is bounded
  by `BOUND_MS` and dropped when its attempt was aborted; the two self-posting archive sites remember "off" on a 404.

## 3. What cycle C builds, and its raw material

**Spec §4 row C, restated:** §2.1 "The Worker's two doors" whole, with the client's ONE change — the `claimed=`
upgrade query parameter under `open` (§2.1 "The claim's carriers"); §2.5's Worker half; §2.7 as amended (the
`hasTables` memo, the picker invariant, the scrub fence, the pending list, the time unit, L-8, L-4, the module's
figures, and the restatements IMPORTED — the module and the browser host switch to A's exports); §2.8 as amended (the
LOW findings); §2.9 whole; R-6.5c2.19's bump and its ripple (§8.2); §2.10 and §2.11 whole; then §3's close in full —
the review, the battery, the ONE recorded sequence, the deploys, the live tiers, the screenshots, the close document,
the merge (R-6.5c2.17) and the `v0.6.0` tag.

**Raw material** (under `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/` unless committed): the drafts D6
(`drafts/D6-tasks-17-19.md`), D7 (`D7-tasks-20-23.md`) and D8 (`D8-tasks-24-28.md`), the assembly's notes and
`review-plan-opus.md`; the retired review's P-9 (the secret's hand-off: ONE `wrangler deploy --secrets-file` at the
close, nothing before) and P-10 (counts as deltas over the implementer's own HEAD); cycle A's §7 standing items,
VERBATIM below with their state; the words in §2 above, because C runs the live specs against them.

**Cycle A's §7 items, each marked:** the R-A19 witness — **DONE** (Task 1); the live replay's reset recipe once after
the bump — OPEN; the whole-game gate back into CI on Task 8's reading — OPEN; the import switch (`api::error_body`,
`api::MAX_WAIT_SECONDS`/`check_wait`, `api::NOT_AN_OBJECT`, `signal::house_frame`, `games::HostClock`,
`platform::{CALLS, calls_in}`; `identity_of` through `identity::admit`; the Worker's receipt handing the held text
back so `PutError::Conflict.held` is `Some`) — OPEN; spec R2.2-m4 (`receipt.rs` picks `test/` by `ends_with(".test")`
while `a.test` is a legal plain name) — OPEN; the house copy's `turn.bot.params.weights` above 2^53 beside three
"carries no seed" justifications for parsing it — OPEN; the documents left for C (`deploy/RUNBOOK.md:2419`,
`worker/README.md:168`, `RUNBOOK.md:2115`, `deploy/cloudflared-config.yml:58`, RUNBOOK's and CLAUDE.md's `claimed` /
`dev-identity`) — OPEN; `events`' refusal order with no two-defect probe — OPEN; the two `bots.rs` edges (a
`bga-human` seat reads foreign; a same-name re-point reads "played against V4 … now runs V4") — OPEN, the AI lane's;
the Worker's `settleUpload` reading a `foreign_bot` 409 as final-for-now — OPEN; "only `finalize` stamps `run_id`"
unpinned — OPEN; `RUNBOOK.md:1427` spelling the owner's address — OPEN (the document pass); the `@`-shaped name under
`open` — CLOSED by R-B2 (server parity; the hint copy reads the plain-name rule).

## 4. The rules cycle C's plan carries

- The standing set (cycle B's handover §4 minus the screen rules): one Opus implementer and one Opus reviewer per
  task, Fable for the whole branch; every agent's full report to a file in the ledger with a three-line reply;
  reviewers' mutants on an EXPORT with `cp` + `touch` restores and one shared `CARGO_TARGET_DIR` under the scratchpad;
  never follow up a DONE without reading HEAD; a fix is a NEW commit; while an implementer works nothing runs in its
  tree; a stalled agent is woken by message; the e2e SEQUENCED, detached to a file and waited on in the FOREGROUND
  with a bounded loop — never a Monitor, an event wait or a re-armed timer; counts as deltas over the implementer's
  own HEAD; every non-ASCII code point in code written as a `\u` escape through a post-pass (an edit tool decodes a
  typed `\uXXXX` — three implementers hit it).
- **Every account step is the orchestrator's** (deploys, secrets, buckets); the quota guard before ANY live tier
  (RUNBOOK §6.7); the live replay's reset recipe BEFORE its first run (owed since A's Task 5 and again by the bump);
  the two pool projects for the two doors; the deploy guard's per-door rules; the close in M6.5c's order (spec §3);
  P-9's single `--secrets-file` deploy at the close.
- The owner's standing authorization (2026-09-27): *"prior authorization for deployment both on the NAS and
  cloudflare, and for a push and PR at your discretion"* — held through B by ruling R-B5, spent in C.

## 5. What cycle B does NOT claim

- The four identity states, the single-player build's four controls, the game route's foreign screen and a
  completed foreign Remove are jsdom-held and HAND-photographed (one-off scripts in exports), not walked — no local
  server reaches them; state E (mock 06) was rendered once by the whole-branch reviewer.
- The bounded probe and the socket arm are cell-held (the local walk serves "sse"); `exit_game.spec.ts`'s new words
  ran nowhere (opt-in, live); the two live specs were REWORDED and type-checked, not run; `exit_game`'s "one link per
  table" is read at C's live run.
- The mocks are a design, not a pixel contract: each task's README says where its screen differs — the board's
  bottom three tiles' hues, the device card's storage line and backup note, the lobby's stacked Abandon (creator rows
  ~91 px against the mock's ~62), the Players panel's tag order, the open table's chat in the main column.
- The whole-branch review's carried items (`final-review.md`, `fix-wave-review.md`): the foreign SERVER game's screen
  on the game route is the un-rebuilt chrome (`.setup.broken-save`, the who-strip); the device foot's grid squeezes a
  failure sentence into ~200 px at 390; the `\u`-escape rule was applied unevenly to VISIBLE glyphs (every invisible
  code point is escaped everywhere); a refused copy that later succeeds leaves the refusal beside "Copied" on both
  hosts; a single-player build's Game menu lists the local games the door sets aside as foreign, which open onto
  their host-aware refusal screen. Each is a §7 item below; none touches the game screen, the corpus or the fences.

## 6. Every ruling taken in this cycle's ledger (2026-09-27/28, the orchestrator; one line each)

- **R-B1** (planning, 2026-09-27): the R-A19 witness is Task 1 of this cycle, one commit on B's branch; A's PR stays as reviewed.
- **R-B2** (planning): the name step keeps SERVER PARITY — an `@`-shaped name is admitted under `open` as `User::parse` admits it; the hint copy reads the plain-name rule (mock 03).
- **R-B3** (planning): L-6 (`outbox.ts`) is Task 9's, against spec §4 row C's placement, because it is client code beside Task 8's items.
- **R-B4** (planning): the disk was cleared by the owner before Task 1 (~83 GiB free).
- **R-B5** (setup, 2026-09-27): the owner's deploy authorization is HELD, not spent — the plan scopes every deploy and live tier to C; the push and PR land at the close.
- **R-B6** (Task 1): the commit's `Claude Opus` trailer stands (a reviewed SHA is never amended); every later dispatch says `Claude Opus 5.5`.
- **R-B7** (Task 1): the `web/` and `worker/` suites were not re-run for a one-file Rust test edit no npm command reads; from Task 2 on every implementer runs the full per-edit set.
- **R-B8** (Task 2): the 64-character display name's overflow at 390 is fixed against the plan's "take the stylesheet as written" — the spec's "nothing wider than the window at 390 px" wins (685 cases measured).
- **R-B9** (Task 2): comment-truth and test-title Minors ride in the same fix commit as an Important; the rest are deferred to the close's triage.
- **R-B10** (Task 3, mid-task): "Ready to play offline" has no period (spec §2.2's copy); `offline.spec.ts`'s four reads change as word-only edits — the commit's 26th file.
- **R-B11** (Task 3): the title/board overlap at 601–820 px is fixed against the draft's 600 px breakpoint — the hero stacks at 860; a `Range` read at 700 pins it.
- **R-B12** (Task 3): the device card keeps the storage line and the backup note mock 01 omits (standing rulings R-6.5b2.9/.10) — an owner item; the board's bottom three hues likewise.
- **R-B13** (Task 4): every non-ASCII code point in the name rule's code is a `\u` escape and `WHITE` is pinned two-way to `\p{White_Space}` (the brief's rule; an edit tool decodes typed escapes).
- **R-B14** (Task 4): a display name stored under `open` showing in the Access corner is as the spec mandates — the two hosts are different origins; "change name" is the one-origin remedy.
- **R-B15** (Task 5): the lobby's one-line check measures the action BUTTON, and the button comes onto the row's line (the stacked Abandon in its own grid cell).
- **R-B16** (Task 6): the form's `.fold` rules are scoped to `details.fold` — an unscoped rule reached the game screen's turn-account and chat folds.
- **R-B17** (Task 6): the Advanced fold is HELD OPEN while the seed is refused (the summary's click cancelled) — the brief's rule over the draft's shape.
- **R-B18** (Task 8): the lock screen's and the local deal's "Lobby" partners in a multiplayer build are pinned (two cells).
- **R-B19** (Task 8): a LOCAL foreign game's Abandon sends first like every local Abandon (`local` on the foreign arm); the archive's check never reads the bot.
- **R-B20** (the close): the whole-branch review's Important 1 — a single-player build's Game menu asking the server for its tables — TAKEN in the wave (the local host's games listed under `!multiplayerOffered()`, a cell that nothing reaches `fetch`).
- **R-B21** (the close): the review's triage adopted as written — five FIX items, three "FIX if a wave runs" lines and the Copied/announcement fold in ONE wave of ten commits; every C line carried to §7; every drop dropped with the reviewer's reason.

## 7. Cycle C's standing items this cycle created or found (appended to A's §7)

- **The bot's version in the saves, digestibly, and in the FILE NAME / R2 key** (the owner, 2026-09-28): today the
  record's `run.seats[i]` names a bot seat `{controller: "v4-step9-draft1", version: 2, params: {b, eval_impl, k, m,
  weights: <u64>}, agent_seed: <u64>}` beside top-level `engine_version` / `semantics_rev` / `rules_hash`; the backup
  envelope keeps the record as EXACT text with the table beside it; tables written since A mirror the descriptor as
  `turn.bot`; the human label ("V4") is NOT in any file — it is `bots::label` over the identity. For C's planning:
  (1) put the label/version where a bucket listing reads it — the ARCHIVE KEY (`rev7/over/<id>.json` → a version
  segment or suffix) is the layer's and the Worker's, needs no record byte change and keeps A's byte-for-byte save
  pin; (2) a label in the record BODY touches core's `RunProvenance` — a fenced-crate change and the AI lane's, so C
  scopes it as the KEY + the envelope's metadata unless the owner rules otherwise; (3) the label must disambiguate by
  VERSION (the same-name re-point edge); (4) the training-corpus fence (map §1.5/§1.6) decides which records C files
  under which key. The owner's purpose: organizing saves by bot version for R2 and the AI pipeline's training volume.
- `whoami.ts`'s operator sentence names "the self-host guide on identity" — C's `deploy/README.md` (spec §2.11) makes
  it true (Task 4's M8).
- The lobby's finished games under "Being played" read "Spectate" where "View" may be truer (Task 5's M5); the
  secondary controls at 390 draw ~30 px against the primaries' 44 (Task 7); the Players panel's tag order vs mock 10
  (the gold `V4` box would sit against the gold `¥4` chip in the mock's order) — the owner's design calls.
- The two plan defects execution found: the plan's verbatim no-stream cell was vacuous as written (Task 8 rewrote it);
  the ready line's period broke four `offline.spec.ts` reads (R-B10).
- **From the whole-branch review, carried (each with the reviewer's fix):** the foreign server screen takes the lock
  screen's shape — `.band-screen` + `LobbyBand(["Lobby", name])` with `serverCorner` + `.notice-card`, the local
  broken save likewise under `["On this device"]` (Minor 6); `.device-foot .offline-state` spans the row when its text
  is not the one-line "Ready…" (Minor 7; verify in `offline.spec.ts`'s refused-registration screenshot); the escape
  rule stated once in C's document pass — "invisible code points always escaped; visible glyphs may be raw" (Minor
  10); the refusal cleared when a later copy succeeds, on the open table and the lobby (the Opus re-review's one
  Minor); the single-player Game menu carries the door's foreign filter — one shared stranded predicate from
  `tables.ts` read by the menu, plus `unplayableIds()` for the cross-rev rows (the wave's concern (a)).
- **One a11y pass** (C): the door's Close returns focus to "How to play"; the identity pill's name carries "Playing
  as" and its action; Escape cancels the corner's edit and closes the phone menu; the project link announces its new
  tab; the refused seed's summary carries `aria-describedby="seed-error"`; "Copied" is announced (done in the wave, jsdom-proven; on the lobby's
  buttons some screen readers may say it twice — one manual check).
- **Design polish left to the owner's eye** (C): `a.primary-link`'s box vs `button.primary`'s (~1.6 px); the device
  card's h2 → h4 skip (heading levels touch the walks' queries); state E's `.two-up` at 976 px vs mock 06's ~868;
  the band's crumbs wrapping at 390 under a long address; the open table's refusal far from its field; the "Being
  played" row of a finished game reading "Spectate".
- **Notes for C's e2e and archive work:** the lobby cell's every-row sweep would go red on a leftover 4-player table
  from a crashed run (its `finally` deletes its own); a refusing archive leaves a local foreign game undeletable (the
  exposure every local game has); an unreadable IndexedDB hides the foreign rows from "Will not open here" (Back up
  on the whole device still works); the `run` premise cell's "reserialization changes the bytes" line rides the
  top-level `seed` (the two-u64 premise is asserted); the held arm's `connected = true` is unpinned (dark on both
  hosts); Task 1's "warn exactly once" doc goes stale at the bump (C's bump ripple, §8.2); R-B14 is worth one line in
  C's self-host guide (a one-origin operator flip shows the open door's display name until "change name").
- The whole-branch review declined to judge four behaviours, each accepted as it stands: the Game menu's server
  listing under a MULTIPLAYER build (pre-existing M6.5b); `Door.tsx`'s C-before-refusal edge (spec §2.1 admits it);
  "Spectate" on a finished Being-played game; the Players panel's word order.
