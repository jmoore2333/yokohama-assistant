# M6.5c2 — the cleanup: spec

**Status: DRAFT, revision 2.1, 2026-09-26.** Written at the close of the M6.5c2 brainstorm with the owner, from
[the M6.5c2 handover](2026-09-26-web-frontend-m6.5c2-handover.md).
- **Revision 1** (`6ce33fd6`) went to the owner and, in parallel, to a Fable adversarial review. The review found
  0 CRITICAL, 4 HIGH, 6 MEDIUM, 12 MINOR and 6 OPEN, and checked 50 claims true.
- **Revision 2** applies every finding. §10 records each finding with its disposition.
- **Revision 2.1** (this text) records the owner's answers to the two items revision 2 left open (§10.2).

**This document is the authority for M6.5c2.** The handover is its entry and is not a spec. Where this spec and a
frozen artifact disagree, the frozen artifact wins unless a ruling here names it and supersedes it:
- R-6.5c2.3 supersedes R-1.4's framing of `claimed`;
- R-6.5c2.11 supersedes the M6.5c spec §2.6 conflict arm.

**The cycle in one paragraph.** This is the last cycle of the web lane before it goes quiet and the project
returns to the computer player (the owner, 2026-09-22 and 2026-09-26). It finishes the front door and the
lobby, turning the development placeholders into designed screens. It turns the manual-identity "dev door"
into a real self-host door that needs no identity service. It makes every save say which bot, which release
and which host produced it, so saves can be evaluated when later bots are trained. It closes every design
debt the M6.5c reviews listed. It arranges the two hosts' code and documents, writes the testing strategy
down, and closes with one full recorded test sequence, both hosts deployed and the nine-PR stack merged.

---

## 0. Read order and sources

1. **This spec.** §1 is the rulings, §2 the mechanism, §3 testing, §4 the order of work, §10 the review record.
2. **The approved mockups**, committed beside this spec:
   `docs/plans/2026-09-26-web-frontend-m6.5c2-mocks/01-…11-*.webp`. Their HTML sources are in the gitignored
   `.superpowers/brainstorm/20585-1790461301/content/`. The screenshots are the record; the HTML is a
   working reference for colours and spacing. Neither is a pixel contract: the implementer builds the design
   in the app's own stylesheet and components (§2.2).
3. **The review report**, gitignored:
   `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-spec/review-spec-fable.md`. Every finding carries its
   evidence. Its "Checked and true" list holds the file:line answers this spec now cites.
4. **The M6.5c2 handover**: §4.6 is the debt list this spec adopts whole (R-6.5c2.1); §6 is the traps not
   to re-derive.
5. **The M6.5c close**: §6 the LEAVE list and owner items, §7 what it does not claim.
6. **The M6.5c spec** (`2026-09-22-web-frontend-m6.5c-objects-spec.md`), read through its §10.
7. **`CLAUDE.md`**: the gate commands. **Re-count every suite at your own HEAD.**

---

## 1. The rulings

Every ruling below was taken with the owner in the 2026-09-26 brainstorm, unless it says otherwise. The owner's
words are quoted where the owner gave them, so they can be found by search.

**R-6.5c2.1 — Scope: the whole debt list.** All three tiers the brainstorm presented are in: the wrap-up itself
(frontend, hosts, docs, testing strategy, account chores, the merge), the correctness debts (the refinish
issue, the whole-game gate's livelock, the module's structural invariants), and the refactor and cost debts
(the `table.ts` split, the rows-read cost, the restatements, the scripts' duplication, the reviews' LOW
findings and Task 8's minor items). Owner: "All three tiers". Each debt keeps the source the handover gives it
(§4.6); §2.6–§2.8 list them. The review confirmed every §4.6 debt and §5 owner item is placed here or left
with its reason (§7).

**R-6.5c2.2 — The NAS stays live, as the self-host path's reference host,** and is redeployed at this cycle's
close with this cycle's build. Owner: *"it's worth keeping the NAS live since it's the self-host path, and it
costs nothing to have available, and is less work then redeployign when we want to verify the self host
change … while we'll be working with cloudflare moving forward, this is still going to be of interest to those
that want to host on more traditional channels/infrastructure or self host."* The Worker remains the hosted
deployment the project develops against. The NAS gains **no** TTL, marker or operator surface: a self-hoster
owns the disk and the process, and those stay Worker features. This closes the handover's §4.2 question 2
(review o4).

**R-6.5c2.3 — The open door: the manual-identity mechanic becomes a real self-host mode.** Owner: *"If I were
someone that came across this project and wanted to self host in docker, I wouldn't expect that I would need to
setup an enterprise identity plane in cloudflare - so having that bypass, either by build flag or otherwise and
our design improvement on the frontend should keep that potentialitaty in mind"*; and *"we have a whole
mechanic for manually specifying identity that's in scope for overhaul here which we had as a flag"*. The
choice was **the open door only**: no house password and no accounts. §2.1 is the mechanism.
**Supersedes** R-1.4's framing of `claimed` as "a development convenience". The open door is a supported
self-host mode with a documented trust model: anyone who can reach the server may use any name.

**R-6.5c2.4 — The off-states are designed, not sentences** (mock 05, 06): A "Your sign-in has run out", B "This
server couldn't tell who you are" (the operator detail in a fold), C "Can't reach the server", D "Connecting to
the server…", E the door with multiplayer off. Owner: "yes, this holds well". §2.1 defines what decides
each.

**R-6.5c2.5 — The Worker's dev door stays a test instrument.** It has no screen, it's the one way the live tiers
play as several people over Access, and it stays ON in production's vars until the public door.
- `DEV_DOOR_CLIENT_ID` moves from `worker/wrangler.jsonc`'s `vars` to a Worker secret (M6.5c close §6.3
  items 13–14).
- Its value stays in git history and in RUNBOOK §6's records. That's accepted: it is the credential's
  *public* half by the M6.5c design.
- The close's suggestion that the guard print or require the dev door's state is **deferred with the public
  door**.

**R-6.5c2.6 — The front door is direction B, with just the title.** Owner, on the direction: "B"; on the copy:
*"I think we can just remove the line above or below - just call it yokohama. If we wanted real bonus points,
maybe put a link to the project below that, but we can make it a variable that it can be changed as part of the
build (cf) or install/run (docker) either way an env change … probably most cleanly a build error or run error
and calling it out in the docs."* The link is **build-time** on both hosts (`YOKOHAMA_PROJECT_URL`; §2.3),
because a run-time rewrite of `index.html` would break the precache's digest check. The cards: "2 players · You
against the bot", "2 to 4 players · Play with friends", and "On this device" with **Install app**. Owner:
*"should the last column include a button for offline installation on single player?"*; answered yes (mock 01).

**R-6.5c2.7 — The lobby is L1 ("three lists"), with "Spectate" and a password flag.** Owner: *"preferring L1
style - let's change the word Watch to Spectate. Also if a table is password protected there should be an icon
or flag visually representing that to the user."* Every word a person reads changes to the "spectate" family.
The wire's `watching`, `watcher` and `can_watch` are protocol keys and stay (mock 02).

**R-6.5c2.8 — The forms** (mock 07). Owner: *"(ID shown if left blank in lobby) and remove the subtext … remove
that alt-text for the password box … for advance, we should validate that value for the seed it's a valid
value. and have it happen in that window … remove the subtext line It stays here unless you send it
somewhere."* Plus the brainstorm's own changes, approved with the mock:
- "Your name" leaves the forms, because the lobby corner owns it.
- **First player** is chosen: You, The bot or Random on the local form; a colour or Random on the table form.
  This closes the "who starts" item carried since M6.5b; the server already accepts `start_seat`
  (`#[serde(default)]`).
- The password toggle says what it guards: "join or spectate".

**R-6.5c2.9 — Joining: one free seat means one button.** Owner: *"if Dana joins and there is only one seat they
should just take that seat, I think the sit here makes sense if there are multiple colors remaining"*. One
click, never zero: arriving by a link does not seat anyone (mock 09).

**R-6.5c2.10 — The bot's version is shown, never chosen, and every save names what produced it.** The owner, on
the label: *"I'm not suggesting we allow people to choose the version, it's more so they know if they're playing
a v4 or v5 version when they join later on … we just write that down somewhere when we iterate bot version and
it pulls it in somehow."* On saves: *"it's important we know the bot version with the save files, that's a
critical component for future save evaluation when we're training later versions of that bot mechanic"*, and
*"not just provanance but we need to know from the save file so when we have them in the pipeline for
evaluation we know what we're looking at in case users are still running an old version. It's ok to not let
people resume old games when they upgrade their server to new games that's a fair line to cross and we can
give an error to that effect."*
- **Read as the owner's words are, about resuming:** a game that is **not over** and whose recorded bot this
  build doesn't run refuses to open.
- A **finished** game has no bot left to move, so it still opens, downloads and scores as today, with its own
  bot's label shown (review M2).

§2.4 is the mechanism (mock 10).

**R-6.5c2.11 — The refinish rule: stamp on a same-game conflict.** At game over, a put that meets **an earlier
completion of the same game** (equal `seed`, `config` and `choices`) under `over/<id>` stamps the table as
archived. The archive keeps the first ending, and the table is no longer stranded.
- A record of a *different* game at that key can only be reached through a caller-chosen `NewGame.id`
  (tests, the fixture harness, the live replay). There it keeps today's per-host behaviour: on the NAS the save
  stays owed and logged; on the Worker the conflict stays final and the table stays stranded until the
  operator's `?force` (review M3).

Owner: "Stamp on conflict". **Supersedes** the M6.5c spec §2.6 arm that stranded a same-game refinish (§2.5).

**R-6.5c2.12 — The `test/` prefix expires after 30 days,** by an R2 object **lifecycle** rule, set once by the
orchestrator and read back to confirm the prefix is exactly `test/` (handover §4.5).
- The command is `npx wrangler r2 bucket lifecycle add yokohama-archive <name> test/ --expire-days 30`
  (the prefix is positional), and the read-back is `… lifecycle list yokohama-archive`.
- **Not** `wrangler r2 bucket lock add`: that is a retention rule, the opposite of what's wanted (review m8).

The plan re-reads R2's command reference before running it.

**R-6.5c2.13 — The full test sequence runs at the close only.** Owner: "Close only". The refactors are measured
task by task by the per-edit suites (the test pool and the whole corpus through the objects) against the M6.5c
close's recorded counts, re-counted at this cycle's first task. The standing testing strategy is written into
the lane document (§2.11).

**R-6.5c2.14 — CI: the Worker gets its own job, and the Docker image is built when its inputs change.** Owner:
*"split plus the docker build but by weekly it would be depending on the code changes, if nothing changes then
it shouldn't need a rebuild - so it should be dependent on those crates or codebase having any changes"* (§2.9).

**R-6.5c2.15 — The documents are laid out by host, with a lane document** (§2.11). Owner: "By host + a lane doc".

**R-6.5c2.16 — The owner's address is scrubbed.** Owner: "Scrub now". The review counted **nine tracked lines
in five files**, not five lines in one (review H4):
- the five `deploy/RUNBOOK.md` lines (§3.7 twice, §3.8, and §3.14 Step 4b twice), each replaced by "the
  owner's address" with a dated note in place;
- the layer's own test literal (`archive.rs`'s `caller_segment` cell), replaced with an example address;
- the board (`docs/handover.md`), whose `…_at_…` transliteration is trivially reversible;
- **Two lines in FROZEN plans:** `docs/plans/2026-09-14-web-frontend-m6.1-plan.md` (a `curl`) and
  `docs/plans/2026-09-20-web-frontend-m6.5b-plan.md` (the test literal). Dated artifacts are "never edited
  after their phase closes". **The owner ruled a recorded exception for personal data** (2026-09-26,
  "Record a personal-data exception"): the two lines are edited to "the owner's address", and a dated
  decision-log row names the exception and both lines. The frozen-record rule stands for everything else.

**R-6.5c2.17 — The merge, after the close:** #31 → #39 in stack order, **with merge commits**, never squash or
rebase. The TDD history stays intact and each cycle's PR stays its record. Owner: "In order, merge commits".

**R-6.5c2.18 — Process: the lane's standing rules** (handover §8). Every task is subagent-driven, with Opus
implementers and reviewers and a Fable whole-branch review. The orchestrator runs every step that touches the
account (deploys, secrets, the bucket, the live runs) and edits no code. Every agent writes its report to a
file. **Every task that changes what the client shows loads the `frontend-design` skill** (the owner named it
for this work), and builds against the committed mockups.

**R-6.5c2.19 — The release is versioned: `0.6.0`.** Proposed in revision 1 and **ruled by the owner** (2026-09-26,
"Bump to 0.6.0"): one release number across the workspace and both npm packages, and the release commit
tagged.
- Every crate takes the workspace version, `[workspace.package] version` in the root `Cargo.toml`, still
  `0.5.0`. So every save any build has written since the V4 release says `engine_version: "0.5.0"`.
  `GameRecord`'s own doc names that field as how "a human sees which build wrote the file".
- This release bumps it. The lane document records the rule going forward: **bump at every release that can
  change what a save means or which bot plays**, the AI lane's V5 ship included, and tag the release commit.
- **The bump is not free** (review H3): §5 lists everything it reaches, the most visible is handled in §2.6,
  and the live replay's reset follows it (§3). The owner took it knowing its cost.

---

## 2. The mechanism

### 2.1 Identity: the open door (R-6.5c2.3, .4, .5)

**The server, as two rules** (review H2). `User::parse` is the KEY grammar: it is how every seat binding, watcher
entry and archive directory is keyed. It is called with no mode from six places:
1. `resolve`;
2. create's `SeatSpec.user`;
3. `check_shape`'s watcher-list rule, which refuses a whole table document on read if an entry doesn't parse;
4. `archive::caller_segment`;
5. the Worker module's `identity_of` and `init`;
6. `backup::check_pair`.

The mode lives on `AppState` and in `resolve` alone. So:

1. **The key grammar, for every caller.** `User::parse` accepts an email (today's shape) **or a plain name**:
   - The value is trimmed, runs of internal spaces collapse to one, and it is lowercased by ASCII rules
     (`to_ascii_lowercase`, as today; the name travels in a header).
   - A plain name is 1–32 characters of ASCII letters, digits, space, `.`, `-` and `_`, and it **must begin with
     a letter or a digit**.
   - The literal `anonymous` is refused as a name, so it can never become a key. That is separate from the
     Worker's rule, where the header `anonymous` reads as absent.

   A plain name therefore always contains at least one `[a-z0-9]` character and never begins with `.`. It can
   never be empty, `.` or `..` (review H1, measured: an unconstrained `..` wrote a browser's save into the
   server's own `rev7/over/`).
2. **The mode rule, in `resolve` alone.** Under `open`, any key the grammar accepts is an identity. Under
   `cf-access` and `worker`, an identity must also have the email shape. A non-email header there now reads as
   **absent** (state B). That is a new mapping: today it is a 401 carrying `User::parse`'s own sentence, which
   the client draws as a plain refusal.
3. **A body-borne plain name under `cf-access` or `worker`** (a create seating `{"user": "ben"}`) is accepted as
   a key nobody can hold, exactly as a misspelled email is today. There's no new refusal.

**The archive's containment is restated, not assumed** (review H1, escalation 2):
- `caller_segment` keeps `[a-z0-9._-]` and maps everything else to `_`. For an email, `@` becomes `_at_`. For a
  plain name, the leading letter or digit guarantees a segment that is never empty, `.` or `..`.
- Its cell (today's email-only containment test in `archive.rs`) is **rewritten**, not left green. It feeds the
  names `.`, `..`, `. .`, `-` and `.x` to `User::parse` and asserts each is refused, and asserts the segment of
  every accepted name begins with `[a-z0-9]`.
- **Defence in depth:** `archive::paths` refuses any path component that is `.` or `..`, and `DirArchive::put`
  checks that the joined path stays under its root before it creates a directory.

**The rename.** The mode `claimed` becomes **`open`** (`YOKOHAMA_WEB_IDENTITY=open`), and the cargo feature
`dev-identity` becomes **`open-door`**, still a default feature.
- A build without it (`--no-default-features --features server`) still refuses the mode at start-up, by name.
  Its refusal sentence states the trust model instead of calling the mode a development convenience.
- The header stays `X-Yokohama-Claimed-User`: the *name* is claimed, and only the mode is renamed. There's no
  alias: the NAS is the only deployment on `claimed`, and it is redeployed with `open` at the close.
- `/api/me`'s `mode` and `/api/health`'s `identity` report `open`.
- `check_shape`'s watcher-list refusal sentence ("… not an address this server writes") is reworded, because a
  watcher may now be a name (escalation 5).
- The rename's ripple, all of which move in one task:
  - `identity.rs`, including the cell that asserts the feature's name;
  - `crates/yokohama-web/Cargo.toml`;
  - CI's refusing-build step, whose `test result: ok. 5 passed` grep moves with the new identity cells
    (review m4);
  - `web/playwright.config.ts`;
  - the corpus files `health.json`, `me.json` and `me_signed_out.json`;
  - `web/src/types.ts` and the client's four `claimed` literal sites;
  - RUNBOOK and `CLAUDE.md`.

**The client.**
- `SignIn.tsx` becomes the **name step** (mock 03): one field, Continue, the hint stating the rule above, and
  the honest sentence: "This server doesn't check who anyone is… Your tables stay with the name you sat down as,
  so switching names later leaves them behind."
- The name step normalizes exactly as `User::parse` does, pinned to it (§3). The typed form ("Ben") is kept as
  the display name, and the identity key is the normalized one ("ben").
- The storage key `yokohama.claimed_user` is kept, so a browser signed in before the rename stays signed in.
- The lobby corner reads "Playing as <display name or identity>" (the pill "Ben ▾" at phone width). Under
  `open` its action is **switch player**, which clears the stored name and returns to the name step. Under
  `cf-access` and `worker` it is **change name**, which edits only the display name.
- **What decides states A–E** (review M6). Today the door has three arms: `door.error` (any failed `/api/me`),
  "connecting…", and the `me.user === null` split. The states refine them:
  - **A:** the read was refused by the edge with Access's challenge (`WWW-Authenticate: Cloudflare-Access`,
    `ACCESS_CHALLENGE` in `api.ts`). The card has one action, "Sign in again", the `/enter` navigation
    `Refusal.tsx` offers today.
  - **B:** the server answered, and no identity reached it: `me.user === null` under `cf-access` or `worker`,
    including a non-email header under the new mode rule. `whoami.ts`'s header sentence moves into a "For the
    operator" fold.
  - **C:** no server answered: the `me` read or the health read failed without a response (a network error or
    offline). Today the lobby calls this "not connected". "Try again" re-runs both reads.
  - **D:** nothing has answered yet. "Connecting to the server…" with skeleton rows.
  - **E:** `VITE_MULTIPLAYER=false`. The door without the "Play with friends" card.

  Any other refusal of `/api/me` keeps today's `Refusal` rendering inside the new card.
- **R-N3:** `signal.ts`'s `Link` (`"live" | "refreshing" | "reconnecting"`) gains **"connecting"**, so a first
  connection never reads "reconnecting…" (M6.5c close §6.3 item 11).

**The Worker.** Its identity (`worker` mode, the verified Access assertion) is unchanged, apart from the mode
rule's new absent-mapping in the shared `resolve`.
- Its dev door is unchanged in behaviour. The live tier's claimed header (`web/e2e/live.ts`) is unaffected,
  because the header name stays.
- `DEV_DOOR_CLIENT_ID` becomes a secret (`wrangler secret put`, from stdin, as the operator list was).
  `env.ts` and `identity.ts` already read it from `env`, so no code changes. The deploy guard's check 7 extends
  to it, and exempts `wrangler.test.jsonc`, whose test value stays for the pool (review m9).

### 2.2 The setup screens (R-6.5c2.6 – .9)

The screens that share today's plain `.setup` column (`FrontDoor.tsx`, `OnThisDevice.tsx`,
`BackupControls.tsx`, `Lobby.tsx`, `NewGame.tsx`, `OpenTable.tsx`, `LockScreen.tsx`, `Unlock.tsx`,
`SignIn.tsx`, `Door.tsx`) are rebuilt in the approved look. The game screen and the deal are unchanged except
for the words in R-6.5c2.7 and the version tag in §2.4.

- **The shared parts.** The same palette, fonts and dark ground as the game screen, and no new colour roles.
  The pieces are:
  - a lobby band (the board mark, "Yokohama / Lobby / …", the identity corner);
  - raised cards with a coloured top band;
  - seat chips (a dot in seat colour; dashed when open; gold when that seat is to move);
  - the orange PASSWORD tag with a drawn SVG padlock (not an emoji), repeated on locked tables' action
    buttons;
  - the gold version tag;
  - seat cards.

  **The hero's board is an original drawing**: ten tiles in the 2P 4/3/2/1 pyramid, tinted by what each area
  trades, with presidents and assistants. It is one SVG component, in the app icon's spirit. No card art
  appears on these screens (`ASSETS.md`: the art carries no grant, and these screens must stand without it for
  a self-hoster or a public release).
- **The door** (mock 01; phone 11):
  - the title only, and the optional project link (§2.3);
  - three cards: the bot (2 players, the version tag, New game), with friends (2 to 4 players, Go to the lobby,
    shown when `VITE_MULTIPLAYER` isn't `false`), and on this device (the saved games with their bot's version,
    Install app per `install.ts`'s states, "Ready to play offline", Back up and Restore);
  - "How to play" opens the help panel.
- The door still **asks the server nothing** (R-6.5b2.7). Every label it shows comes from the build (§2.4) or
  from the local host.
- **The lobby** (mock 02): L1's three sections as rows (name and age · seat chips · one action), a gold edge on
  your own tables, lobby chat in a right rail (below the lists on a phone), folded by default (M6.4's ruling).
  Rows use only what `TableView` carries, plus its new `bot` field (§2.4).
- **The forms** (mock 07), per R-6.5c2.8:
  - the name placeholder "ID shown in the lobby if left blank", with no subtext;
  - no password placeholder;
  - players and first player as segmented buttons;
  - seats as colour rows in the engine's order (Red, Blue, Yellow, Green), with the bot offered at two players
    only, its version tag beside it.
- **The seed** sits under an Advanced fold and is **validated as it is typed, in the fold**, against the
  server's own rule (`WireSeed`: `trim().parse::<u64>()`, which is an optional `+` and decimal digits, at most
  18446744073709551615). A bad value explains itself in place and holds the submit button. The client's copy of
  the rule is pinned to the server's (§3).
- **Waiting and joining** (mock 08, 09):
  - Seat cards replace the seat lines.
  - The creator gets an "Invite people" panel (the link, Copy, and a reminder that the password travels
    separately), and Abandon becomes a quieter danger button.
  - The joiner types the password (where there is one) above the seats.
  - **One free seat shows one "Join as <colour>" button; two or more show "Sit here" on each.**
- A seat filled by name at create time is taken from the start, and shows the person's display name or their
  address.
- **The phone** (mock 11): one column at 390 px, thumb-sized buttons, and nothing scrolls sideways. The browser
  suite gets a width cell for each rebuilt screen.
- **"Spectate"** (R-6.5c2.7): the lobby's button and section, the game banner ("You are spectating"), chat's
  marker, `Help.tsx`'s section and `Unlock.tsx`'s screen. **Three server sentences** carry "watch", and each is
  reworded with its pin (review m2):
  - the record route's spectator refusal, "the record is the players' — watch, download nothing" (`api.rs`,
    pinned in `lib.rs`);
  - the 409 at a full unlock roster, "this table has as many watchers as it can remember" (`tables.rs`);
  - `check_shape`'s watcher-list refusal (`tables.rs`), reworded for §2.1 as well.
- **The single-player build's "Lobby" controls** (M6.5b2 close §6.3; handover §5 item 12): under
  `VITE_MULTIPLAYER=false`, no control on a local screen leads to a lobby.

### 2.3 The project link (R-6.5c2.6)

- **One build variable, `YOKOHAMA_PROJECT_URL`.**
  - It reaches the bundle through a **`define` in `web/vite.config.ts`**. Vite exposes only `VITE_`-prefixed
    variables to `import.meta.env`, and the config has no `define` today (review M6).
  - **The config validates it**: an absolute `https://` or `http://` URL, parsed by `new URL`, with no
    whitespace or control characters and at most 200 characters. A throw in the config exits the build
    non-zero, naming the variable and the reason.
  - Unset, the door draws no link.
- The link text is the URL without its scheme; it opens in a new tab.
- **The Worker** takes it from the environment of `cd web && npm run build`. The deploy guard's check 9 already
  refuses assets that aren't the current `web/dist`, and the RUNBOOK recipe names the value used.
- **The Docker image** takes it as `--build-arg YOKOHAMA_PROJECT_URL=…`. A self-hoster builds the image from the
  repository, so that is their install step.
- A run-time value is **not offered**. The release is precached under a digest of its own bytes, so a server
  rewriting `index.html` at start-up would fail the service worker's install check. `deploy/README.md` says so.

### 2.4 The bot's version, and what every save says (R-6.5c2.10, .19)

- **The label.** A new layer module (`crates/yokohama-web/src/bots.rs`) maps a recorded bot identity to its
  short label (`v4-step9-draft1` → "V4"). It works through `Contestant::from_parts(controller, params)` and an
  **exhaustive match with no `_` arm** over `Contestant` (six variants) and `NamedIdentity` (three), the
  discipline of the CLI's `eligible`.
  - When the AI lane adds a V5 identity to core, **the web layer will not compile until V5 has a label**: that
    one line is the only web edit a new bot needs.
  - The labels and the foreign-bot refusals (below) are words the layer authors itself. On the explanation
    surface, `why`'s foreign-bot refusal is answered **before** the `why` gate, whose twelve refusals stay
    `why::account_parts`' own strings, carried verbatim. That is a deliberate exception of the same kind as
    `suggest`'s four authored refusals, and this spec rules it in.
- **"The current production bot", with no request** (review M1). The door must ask nothing, and the local facts
  are hand-written literals (`web/src/local/facts.ts`), so a fifth literal would be a second truth.
  - `wire_fixtures` emits a **contract file**, `web/src/wire/production_bot.json`, holding
    `{"identity": …, "label": …}` of `PROVISIONAL_DEFAULT_BOT`. It is frozen and refused when stale, exactly as
    `site_space_words.json` is.
  - The door's bot card, the local form and the table form's bot option import it at build time.
  - When the AI lane re-points the production bot, `wire_fixtures` fails until the file is regenerated. That is
    the "pull".
- **The wire gains two fields,** and the corpus is regenerated and its diff read:
  - `ClientState.bots: { seat, label }[]`, from the record's `run.seats` (empty for an all-human game);
  - `TableView.bot: { label, runs_here } | null`, the table's bot seat: its label, and whether this build runs
    that bot.
- **Where the lobby gets it, without a second truth.** `SeatDoc::Bot` deliberately does not store which bot
  (`table.rs`: "a second statement of it would be a second thing to keep true"). So:
  - **A dealt table** reads it from **`TurnMirror`**, the record's cache, which `Games::persist` refreshes
    after every applied move. The mirror gains `bot: Option<BotDescriptor>`: the record's `(controller,
    version, params)`, so `runs_here` compares the same triple the refusal does (review m12), and the label is
    derived from it.
  - `TurnMirror` loses `Copy`, so its four by-value reads take a reference (review m1).
  - `TableDoc` carries no `deny_unknown_fields`, and `check_shape` bounds values only. A nested
    `#[serde(default)]` keeps **`TABLE_SCHEMA` at 3**. An older binary reads the new key and ignores it, and an
    older module's house copy drops it, which is harmless (review m1).
  - **An undealt table** shows the current production bot (`runs_here: true`), which is what `finalize` will
    deal.
  - **A table dealt before this deploy** has no `bot` in its mirror until its next applied move, and a
    **finished** one never gains it, because no `persist` follows game over. Its row shows "the bot" without a
    version. That's accepted, and honest (review m7).
- **The foreign-bot rule** (review M2; R-6.5c2.10). **The check has one site:** `Games::live_handle`, after the
  record's parse and **before** `Session::from_record`, through which every rebuild on all three hosts passes. So
  a converted human seat never exists, even for one request. A bot seat's recorded `(controller, version,
  params)` that differs from `PROVISIONAL_DEFAULT_BOT`'s descriptor is **foreign**.
  - **A game that is not over, with a foreign bot, refuses to open.** Every game route answers a typed refusal
    naming both labels: "This game was played against V4. This server now runs V5 and can't continue it." The
    lobby row shows `runs_here: false` and says so. `delete` reads the record's event count and never rebuilds,
    so Abandon still archives it.
  - **A finished game with a foreign bot opens as today:** frame, replay, `/result` and the record download.
    The bot's label comes from the record. The session's conversion warning ("…it resumes as a HUMAN seat, so
    you are now playing it") is **not shown**, and `why` on a move made by that seat answers the authored
    refusal "this move was made by V4, which this server doesn't run". It **never** says the move was a
    human's.
  - An all-human game has no bot to mismatch and opens as today.
  - **In the browser**, the local list is the module's `TableView` listing (review o6). A not-over foreign game is
    listed under **"Will not open here"**, the `SEMANTICS_REV` pattern, with export and a two-click Remove,
    built from the listing and never from parsing the record's text (`db.ts` forbids that). The plan confirms
    Remove and export work on a table the module refuses to open.
  - An owed game-over upload that meets a deploy with a new bot is finished, so it opens and stamps normally
    (review o3).
  - Nothing converts a bot's seat into a human seat on the web. The CLI's own resume behaviour is unchanged.
- **What every save names**, inside the file:

  | fact | field | status |
  |---|---|---|
  | the rules | `semantics_rev`, `rules_hash`, `terminal_digest` | already |
  | the exact bot: identity, version, every knob, its seed | `run.seats[i]` | already |
  | the release that wrote it | `engine_version` | the field is already there; made meaningful by R-6.5c2.19 |
  | where it was played | `run.run_id` = `web/worker`, `web/server` or `web/browser` | **new** |

  - Every web finalize already calls `set_run_seats` for every game, all-human included, so the layer can call
    the CLI's existing public `Session::set_run_id` after it. That needs no CLI change and can't trip its
    ordering assertion.
  - This follows the BGA importer's `bga-import/<table>` precedent for the field.
  - The host is a value each host passes when it builds `Games` (`Games::with_parts` on both wasm hosts, the
    axum constructor on the server).
  - A record written before this deploy keeps no `run_id` (review m7).
- **A pin, on every save path:** the store, the record route, the archive on both hosts, the browser's
  IndexedDB, the export and the backup envelope carry `run` through **byte-for-byte**. The lane document gets a
  "What a save file says" section for the evaluation pipeline, and the AI-lane handover points to it.

### 2.5 The refinish rule (R-6.5c2.11)

- **Where the rule lives.** It is a layer rule at the game-over archive site (`games.rs`: `archive_if_over` and
  `archive_over_retry`). The NAS cannot reach the race, because it stamps inline under `persist`'s flow, holding
  its lock across the put and the stamp.
- **The rule.** A `PutError::Conflict` at `Why::Over` is followed by a same-game check:
  - The archived record's `seed`, `config` and `choices` are compared with the current record's.
  - If they're equal, the table is stamped `archived_at` as on `Landed`, and the logging seam (§2.6) records
    "the archive keeps an earlier completion of <id>".
  - If they differ, the per-host behaviour of R-6.5c2.11 stands.
- **The comparison is done in Rust**, never in the shell (the strings rule).
  - On the NAS, `DirArchive` reads the held file.
  - **On the Worker, the conflict does not reach the module today** (review M3): `executePlan` reads the held
    text but returns only `{outcome: "conflict", detail}`, and `settleUpload`'s `conflict` arm deletes
    `archive_owed` without calling the module. Three shell changes make the rule reachable:
    1. `executePlan` returns the held record's text with a `conflict`;
    2. that text is installed as the receipt's outcome, so `receipt.rs`'s `Outcome` (`deny_unknown_fields`) and
       `parse_outcome` gain the field;
    3. **`settleUpload`'s `conflict` arm routes through `stamp_archive`**, so the layer's put meets
       `Err(Conflict)` carrying the text, and `stamped: true` is read back.
- **The archive's pair** (review o1). For the undo-then-refinish race, the pair already stays one game's:
  `executePlan` reads the record key **before** it puts the table, so a conflict writes nothing. A cell pins
  that. Two *overlapping* uploads of different records can still leave one completion's record beside another's
  table, and R2 offers no cross-key atomicity. That case is recorded as best-effort; no cell claims it.
- **A pool cell reproduces the race:** an undo during a held R2 put (a patched `R2Bucket.prototype.put`,
  restored in `finally`, per the pool facts), then a different finish. The table ends stamped, abandonable and
  expiring normally. `deliverArchive` runs outside `blockConcurrencyWhile`, so the undo can interleave. The plan
  confirms the shell's `uploads` map (one in-flight upload per plan key) does not serialize the two completions
  (review o2).

### 2.6 The layer's other changes

Each has an architectural reason (R-6.5c.4's refinement). Sources: handover §4.6 and M6.5c close §6.2.
- **A logging seam.** The layer's `eprintln!` is mute on the Worker, so a failed stamp in `archive_over_retry`
  can't report. This adds a `Log` seam on `Games`: stderr under `server`, the shell's log line on the Worker,
  the console in the browser host. Sites that print today print through it.
- **The version drift line is filtered** (review H3; R-6.5c2.19).
  - `Session::from_record` pushes "GameRecord.engine_version=… does not match running engine version … (v1 does
    not refuse on a version mismatch)" into `load_warnings`, and the layer copies those into
    `ClientState.warnings`, which the game screen draws.
  - After the bump, that line would sit on **every existing game**.
  - The layer drops **that line only** from `ClientState.warnings`. `engine_version` is provenance only, and the
    save carries it. Other warnings still show, and the CLI's own frame is unchanged.
  - A cell pins that a 0.5.0 record rebuilt at 0.6.0 carries no drift line in `warnings` while its record keeps
    `engine_version: "0.5.0"`.
- **The positional-array `create` is refused.** serde accepts a JSON array for a struct. The layer's body
  extraction refuses any non-object body, in its own 400 shape, as the module's `init` already does.
- **The whole-game gate's vs-bot livelock** (M6.3 close §6.3; M6.4 and M6.5c §6.1). The arm places nothing and
  leans on the bot to end the game (a 20,000-input cap). It gains **both** of the two-person arm's guards
  (review o5):
  - the placement probe;
  - the stuck-picker rule.

  Its seeds are recorded as **the bot's stream seeds** from each record's `run.seats`, not only the create
  seed, because the bot's seed is OS entropy. A run can then be reproduced. The gate **returns to CI** (§2.9)
  once a recorded run over many seeds shows it bounded.
- **The e2e's `CAP = 4000`** (in four specs: `plays_a_whole_game`, `offline`, `two_browsers`, `plays_locally`):
  the browser walk's driving policy gets the same probe, and the cap's hits are recorded in a sequence.
- **The restatements** (R1: `error_body`, `MAX_WAIT_SECONDS`, the house frame, the clock, the platform scan)
  become items the layer **exports** and the module imports, so the source-text pin retires with its reason.
- **A cell for `archive_over_retry`'s second `NotFound` arm** (Task 1's M6), now that the layer isn't fenced.

### 2.7 The Worker's shell and the module (handover §4.6)

Every item is **behaviour-preserving**: the pool and the corpus through the objects must stay byte-identical,
which is how that is shown rather than claimed.
- **`worker/src/table.ts` (2,082 lines) splits along its four concerns:** the transport helpers beside
  `frames.ts`; the slot half (~330 lines) as `delivery.ts`; the archive half (~380 lines) behind a narrow host
  interface; and one guarded-call helper replacing the three copies.
- **Read once per request (F-4).** `readDocs` runs up to three times per request today, about 470 rows read per
  move live; this reads once and threads the result. A cell counts document reads per request.
- **The picker invariant becomes structural.** `events_inner`, `stamp_archive_inner` and `expire_inner`'s
  no-creator arm reach `Games::handle` outside `picker::around`. Route them through it, or clear the list
  whenever the record changes.
- **The scrub's compile-time fence goes all the way down:** `SeatDoc::Human` and `TurnMirror`, not only
  `TableDoc` and `Lock`. This also catches the new mirror field.
- **`ReceiptArchive.pending` becomes a list**, or a second deferred plan in one call is refused. Either way, one
  plan per call becomes structural.
- **The operator listing uses one time unit.** Today `created_at` is in seconds beside `last_active` in
  milliseconds.
- **Task 14's L-8** (a one-row existence query in `deleting`) and **L-4** (the transitional `forced` line).
- **The Worker module's size figures get one place**, beside the browser module's `release-wasm` comment, and
  `copyOf`'s doc cites that place rather than carrying readings.

### 2.8 The scripts, the live tiers and the client's small items

From handover §4.6 (R3's §7, Task 15's deferrals, Task 8's minor items):
- **The scripts:**
  - `.dev.vars` is parsed in **one module**, `worker/scripts/dev-vars.mjs`, with a type declaration.
    `worker/scripts/live-replay.mjs` imports it, and `web/e2e/live.ts` imports it by relative path. The two
    live in two npm packages; the plan confirms both TypeScript projects accept the import (review m10).
    RUNBOOK §6.4's recipe cites it.
  - one origin constant;
  - the replay's five parts in separate modules;
  - `HOST_FIELDS`, `order()` and the credential's key names each stated once;
  - one parser of the precache list `web/scripts/precache.mjs` prints.
- **R3's LOW findings:**
  - `read-deploy.mjs` no longer echoes a non-200 `/sw.js` body (L-3);
  - the pool's host-field exemption requires the field to be present (L-5);
  - the outbox's drain and sweep stop after the first 404 (L-6);
  - `dealAnswers` is capped at about two per page (L-7);
  - a live create whose reply was never read leaves no table in the shared lobby (L-9).
- **Task 8's items:**
  - a probe answer landing after its attempt was aborted is dropped;
  - the link word is checked while a tab is hidden;
  - the two fake sockets become one;
  - `App.tsx`'s and `Abandon.tsx`'s self-posting sites remember "off" on a 404;
  - the probe gets a bound;
  - the Vite dev proxy gets `ws: true`.
- **Left, with its reason:** R-N7's computed-key escape (`page[k].get(…)`) stays: it's contrived, as ruled.

### 2.9 The build and CI (R-6.5c2.14)

- **`wasm-opt` 133 everywhere.** `deploy/Dockerfile` moves from 132 to 133 with binaryen's published SHA-256, as
  CI already pins. Every build step already prints `wasm-opt --version`, and that stays (I-87).
- In the same edit, the Dockerfile's stale comment "no build.rs anywhere, checked" is corrected, because
  `crates/yokohama-worker/build.rs` exists (review M5).
- **CI, `.github/workflows/test.yml`:**
  - the Worker's steps move out of `web-server` into a job of their own, `worker`;
  - the refusing-build step's `5 passed` count moves with §2.1's new identity cells (review m4);
  - the whole-game gate rejoins `web-server` once §2.6's fix is shown bounded;
  - the header comment is updated.
- **A new workflow, `.github/workflows/docker.yml`,** builds the self-host image:
  - It runs **on push and pull request when the image's inputs change** (review M5), which `deploy/Dockerfile`
    fixes: it copies `rust-toolchain.toml`, `Cargo.toml`, `Cargo.lock`, **all of `crates/`**, `web/package*.json`
    and `web/`. The filter is `deploy/**`, `crates/yokohama-core/**`, `crates/yokohama-cli/**`,
    `crates/yokohama-web/**`, `crates/yokohama-wasm/**`, **`crates/*/Cargo.toml`** (every member resolves),
    `web/**`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` and **`.github/workflows/docker.yml`**. It can
    also be dispatched by hand.
  - It builds with `docker buildx` and the GitHub Actions **layer** cache, then smoke-runs the image
    (`/api/health` 200, under `YOKOHAMA_WEB_IDENTITY=open`).
  - **The layer cache covers the toolchain stages above the source `COPY`, not the Rust build.** The
    Dockerfile's `--mount=type=cache` mounts aren't carried by `cache-to: type=gha`, so a triggered run pays a
    cold `cargo build` and module build. The first runs are measured (§8). A cache-mount export step is the
    plan's option only if the minutes warrant it.
  - GitHub applies path filters per workflow, which is why this is a workflow and not a job. The repo is
    private, so its minutes are counted.
- **Stays local, as today:**
  - the browser walk;
  - the live tiers and the headed game;
  - the fifteen release gates;
  - the deploy guard's build-dependent checks;
  - the per-edit Rust suites CI doesn't name (the CLI, the oracle, difftest, the bridge, the example targets,
    clippy).

### 2.10 The hosts (R-6.5c2.2, .5, .12)

- **The NAS** gets this cycle's image:
  - built with `--build-arg YOKOHAMA_PROJECT_URL` if the owner sets one, and `open-door`;
  - run with `YOKOHAMA_WEB_IDENTITY=open` behind the same Access application (Access stays the door that keeps
    it private; identity is the typed name);
  - the image is rebuilt (resolving R3's E-3) and its size and memory are re-measured and recorded.
- Existing NAS tables were seated under typed emails. Those still parse under the key grammar, so their seats
  still match. There is no TTL, marker or operator surface (R-6.5c2.2).
- **The Worker** is deployed at the close with `DEV_DOOR_CLIENT_ID` as a secret. Its RUNBOOK records the
  deployment.
  - **The deploy drops every open picker on the Worker**, because this cycle changes the layer's sources and
    `build.rs` digests them (I-85, I-91). That's safe, and logged `picker-dropped` (review m11).
- **The bucket** gets the `test/` lifecycle rule (R-6.5c2.12's command), read back.
- **The public door stays closed** (the owner: Access stays until the repo is public). Its preconditions are
  recorded in the lane document, not executed:
  - the path-scoped Access application;
  - the dev door off;
  - the art's publication decision;
  - the project link set;
  - requests per move under a public load;
  - the archive route's untrusted record (§7).

### 2.11 The documents (R-6.5c2.15, .16)

- **Each host directory gets a README (what and how) and a RUNBOOK (operations and records):**
  - `deploy/README.md` (new) is **the self-host guide**. It covers:
    - building the image and its build args;
    - running it, with its variables;
    - the identity modes and the open door's trust model;
    - the multiplayer switch;
    - the archive directory;
    - putting it behind a proxy or Cloudflare Access;
    - what's precached and why the project link is build-time.
  - `deploy/RUNBOOK.md` keeps §0–§5 with their numbers, the NAS's operations and records.
  - Its §6 moves **verbatim** to `worker/RUNBOOK.md`, beside `worker/README.md`. The old §6 becomes a pointer
    with a **relocation map** (`§6.x` → `worker/RUNBOOK.md §x`), so every frozen citation still resolves.
  - The review counted 75 lines citing RUNBOOK §6 outside it: 37 in frozen plans, the rest in code comments and
    documents.
  - **`worker/README.md`'s 18 citations** are updated to the new file directly (escalation 3).
  - Current code comments are updated where they are edited anyway. Frozen plans resolve through the map.
- **`docs/web-lane.md`** (new) takes the web lane's operating detail out of `CLAUDE.md` (105,857 bytes at
  `6ce33fd6`; the handover's "92 KB" predates the M6.5c close's additions — review m5). It covers:
  - the hosts;
  - the suites and gates;
  - the traps;
  - **the standing testing strategy**, from the handover §4.4 table, updated: which tier runs when, what it
    proves, what skipping it costs;
  - **"What a save file says"** (§2.4);
  - the release-version rule (R-6.5c2.19);
  - the public door's preconditions.

  `CLAUDE.md` keeps a short summary and a pointer, so AI-lane sessions stop paying for the web's detail.
- **`guidingpath.md`** gets an M6.5c2 row, and its current state is refreshed. **The board**
  (`docs/handover.md`) has its "previous update" history rewritten into pointers.
- **One "how the documents are built" note** in the lane document:
  - generated and checked: `guide_sync`, the wire corpus;
  - frozen at their close: the dated `docs/plans/` records;
  - current-state pointers: `CLAUDE.md`, the board, the READMEs.
- **The root `README.md`** gets the two-README balance, pointing at the self-host guide and at
  `worker/README.md`. `worker/README.md`'s screenshots of the door, the lobby, the join and the open table are
  recaptured after the redesign.
- **The address scrub** (R-6.5c2.16), including the two frozen plans under the recorded exception.

---

## 3. Testing

- **Per task:** test first, one green commit with its red run quoted, and the per-edit suites for everything the
  task touches (`CLAUDE.md`'s list; the Worker's `npm run build:module:debug` before its pool). The first task
  re-counts every suite at the branch's HEAD and records the reading.
- **The new cells the spec requires** (the plan names each file):
  - **Identity:**
    - the key grammar accepts emails and plain names and normalizes them by one rule;
    - **`.`, `..`, `. .`, `-`, `.x` and `anonymous` are refused as names**;
    - **the rewritten containment cell**: every accepted name's segment begins with `[a-z0-9]`, and a
      traversal probe (a `..` component) is refused by `archive::paths` and `DirArchive::put`;
    - the mode rule: under `cf-access` and `worker` a non-email header reads as absent;
    - **a table whose `unlocked` list holds a plain name reads on every host**;
    - the build without the feature refuses `open` by name;
    - `/api/me` and `/api/health` say `open`.
    - The client's normalization is pinned to `User::parse` over one shared table of inputs, emitted by
      `wire_fixtures`.
    - States A–E each render from committed wire payloads, with the discriminators of §2.1.
    - switch player.
  - **The bot version:**
    - the labels are exhaustive by construction (the compiler);
    - `production_bot.json` is refused when stale;
    - `TableView.bot` for a dealt table (from the mirror), an undealt one (production) and one dealt before the
      deploy (none);
    - `ClientState.bots` from the record;
    - **a not-over game with a foreign bot refuses on every game route with the typed refusal**, checked in
      `live_handle` before any rebuild;
    - **a finished foreign game opens, scores and downloads**, its `warnings` carry no conversion line, and
      `why` on its bot's move answers the foreign-bot refusal, never "a human's move";
    - a same-identity game opens;
    - the local "Will not open here" row, with export and Remove;
    - **`run.run_id` stamped by each host**;
    - **`run` byte-identical through every save path** (§2.4's pin);
    - **the drift-line filter** (§2.6).
  - **The refinish rule:**
    - a same-game conflict stamps, on both hosts;
    - a different-game conflict keeps each host's rule;
    - the pair stays one game's in the undo-then-refinish case;
    - the pool reproduces the race.
  - **The seed:** the client's check is pinned to `WireSeed`'s verdicts on one shared table of inputs.
  - **The project link:** the build refuses each class of bad value, naming the variable; the door draws the
    link only when it's set.
  - **The screens:**
    - jsdom cells over the committed corpus for every rebuilt component (nothing hand-authored, the M2.5
      rule);
    - browser cells at desktop width and 390 px for the door, the lobby, the name step, the forms, waiting and
      joining, with no sideways scroll;
    - a headed look, because no jsdom cell sees a stylesheet.
  - **The refactors:** the pool and the corpus through the objects stay byte-identical, and a cell counts
    document reads per request.
  - **The livelock:** a recorded run of the whole-game gate over many seeds, with the bot stream seeds recorded,
    shows its input count bounded before it rejoins CI.
- **The mutation battery**, at the reviewed head: every new catcher above gets a row, and each is reverted and
  must fail. The containment and foreign-bot rows go first.
- **The close, one recorded, machine-tagged sequence** (R-6.5c2.13):
  1. the per-edit suites of every crate and package;
  2. the fifteen release gates (core is compiled at a new version, and the merge follows);
  3. `npm run e2e`;
  4. the deploys, the Worker then the NAS, with `npm run build` in `web/` first and the `debug_trap` grep;
  5. **the live replay's reset recipe first** (trap 19: the version bump changes the walk's records, so the
     first sends would meet a 409 and stop; review M4), then the live replay twice;
  6. the quota guard, then the live Playwright tier and the headed exit game;
  7. the battery;
  8. the Fable whole-branch review.

  A live step follows the handover §4.4 rules: WARP's device login checked first, the quota guard, and one
  Playwright run at a time.

---

## 4. The order of work

This is proposed; the plan fixes the tasks.
1. **The layer and identity:**
   - the key grammar, the mode rule, containment and the rename;
   - the bot labels, the contract file, the mirror field, the two wire fields, `run_id` and the foreign-bot
     rule;
   - the refinish rule (the layer half) and the logging seam;
   - the drift-line filter, the positional `create` and the livelock;
   - the corpus regenerated and its diff read.
2. **The client:** the shared components and the board mark; the door and the project link; the identity states
   and the name step; the lobby; the forms and the seed check; waiting and joining; the "spectate" words; the
   local "Will not open here"; the phone layouts and their width cells; Task 8's items.
3. **The Worker and the module:** the refinish rule's three shell changes; the `table.ts` split; one read per
   request; the picker, the scrub fence, the receipt list, the restatements, the time units, L-8 and L-4; the
   module's figures.
4. **The scripts and the live tiers:** R3's list and the LOW findings.
5. **The build and CI:** `wasm-opt` 133, the CI split, the Docker workflow, the gate back in CI, and the version
   bump with its ripple (§5, §8.2).
6. **The hosts:** the secret, the `test/` rule, the deploys (the close's sequence).
7. **The documents:** last, so they describe the final state. The address scrub goes with them.
8. **The close:** the full sequence, the battery, the review, the close document, then the merge (R-6.5c2.17).

---

## 5. The fences

- **The five fenced crates** (`yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge`,
  `gen-data`): **source byte-identical.** R-6.5c2.19's version bump changes the version they're compiled with.
  The review corrected this spec's first claim that it reaches "nothing else" (H3). It reaches:
  - `engine_version` in new records (provenance only, by `GameRecord`'s own doc; `replay_strict` accepts the
    drift);
  - **the drift warning `Session::from_record` gives every older record.** The web filters it (§2.6). The CLI
    shows it in its opening frame of any 0.5.0 save it resumes, which is the CLI's documented behaviour,
    unchanged;
  - **`yokohama-bridge`'s served `"version"`** (`router.rs`, `env!("CARGO_PKG_VERSION")`). Its wire output
    changes while its source does not, and the bridge suite reads the constant itself, so it is unaffected;
  - `/api/health`'s `server_version`.
- **`SEMANTICS_REV` stays 7. No bot behaviour moves.**
- **The CLI: no change is needed.** `Session::set_run_id`, `PROVISIONAL_DEFAULT_BOT` and core's
  `Contestant::from_parts` are public, `Repl::bots()` is public, and `eligible` is untouched. If the plan finds
  a seam it truly needs, it is a behaviour-preserving widening and is named. The nine `assert_capture` goldens
  and `guide_sync` are how that is known.
- **The layer changes, each named with its reason:** §2.1, §2.4, §2.5, §2.6.
- **The wire contract changes, each read in the corpus diff:**
  - `/api/me`'s `mode` and `/api/health`'s `identity` (`open`);
  - `server_version` (`0.6.0`);
  - `TableView.bot` and `ClientState.bots`;
  - the new contract file `production_bot.json`;
  - the three reworded server sentences;
  - the foreign-bot refusals' bodies;
  - `warnings` arrays, which are unchanged by design (the filter).

---

## 6. What this cycle does NOT do

- **No public door.** Access stays on both hosts. The path-scoped application, the dev door's removal and the
  art's publication are recorded preconditions (§2.10).
- **No bot choice.** Players see the bot's version; they never pick it (R-6.5c2.10).
- **No resume across a bot change** for a game that isn't over, by the owner's ruling. The CLI's own resume
  behaviour (the seat comes back human, with a warning) is not changed.
- **No house password and no accounts** (R-6.5c2.3). The open door's trust model is "anyone who can reach the
  server".
- **No TTL, marker or operator surface on the NAS** (R-6.5c2.2).
- **The phones** stay the owner's: the iPhone exit and the Android handset, recorded, not gated.
- **No change to requests per move** (~13) beyond the rows-read work. Requests remain the Free plan's first
  binding limit, and are a public-door precondition.
- **Overlapping uploads' pair consistency** is best-effort (§2.5), not claimed.

## 7. Carried, and left with a reason

- **The clean-browser single-audience reading and R-F6's Access re-login:** readings taken in the close's
  headed walk if the window allows; otherwise carried as owner readings.
- **The no-Web-Locks fallback and background suspension:** left. Every browser the lane targets supports Web
  Locks, and a hidden tab lets its socket go by design (R-P8).
- **`walk.json`'s pretty-printed 419 KB:** left; it's a fixture's format.
- **The archive route handing an untrusted record to the engine's setup path** (M6.5b §6.3): left, and carried
  to the public-door preconditions. The route's strict replay is its present guard.
- **Pool fact 8** (`ctx.id.name` inside a production `alarm()`): read when a real alarm fires. Nothing to build.
- **The Origin check on WebSocket upgrades:** stays (the owner can reverse it).
- **`cargo clippy --target wasm32-unknown-unknown`** fails inside the CLI (`session.rs:439`,
  `drop_non_drop`): left. It's inside the fenced CLI, and no gate runs clippy for that target.
- **The CLI-worded warnings that already ride `ClientState.warnings`** (review escalation 1), a `rules_hash`
  drift for instance: left as they are, apart from §2.6's drift-line filter. A reworded warning surface is not
  this cycle's.

## 8. Risks the plan must answer

1. **The corpus diff is large:** two new keys on every table and state payload, plus the contract file. It's a
   contract: read it whole. A key appearing where it shouldn't is what the leakage pins exist for.
2. **The version bump's ripple, found before the bump** (review H3, m3, M4). Each item is either a record
   (kept) or an output (regenerated):
   - **outputs:** `Cargo.lock`; `web/package.json` and `worker/package.json` and their lock files (the npm
     packages follow the release, one number); the corpus's `health.json`; `web/src/wire-script/
     archive-walk.json`'s embedded records (so the live replay's reset recipe runs, §3); `deploy/
     Dockerfile.runtime`'s comment; `web/e2e/plays_locally.spec.ts`'s stale `yokohama-web 0.5.0-m6.5b`
     literal;
   - **records, kept as written:** `worker/test/fixtures/owed-game-over.json`, the web fixture pair
     `browser-played-2p.{json,table.json}`, `worker/scripts/smoke-release.test.mjs`'s fake body;
   - **the four bucket keys** under `test/browser/red_at_wire.test/rev7/over/` that the reset recipe removes.
3. **The Docker workflow's minutes:** the first runs are measured cold.
4. **The livelock's bound:** measured over recorded seeds before the gate rejoins CI (§2.6).

## 9. Process

The handover §8 in full, and R-6.5c2.18. Model dispatch follows the owner's standing rule: Opus is the floor for
implementers and reviewers, and Fable orchestrates and reviews the whole branch. Reports go to files; a reviewed
SHA is never amended; while an implementer works, nothing runs in its tree.

## 10. Review record

### 10.1 The Fable adversarial review of revision 1 (2026-09-26, `6ce33fd6`)

The report is `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-spec/review-spec-fable.md`.
- **Verdict:** REVISE BEFORE PLANNING.
- **Counts:** 0 CRITICAL · 4 HIGH · 6 MEDIUM · 12 MINOR · 6 OPEN.
- **Claims checked true:** 50.
- **Coverage:** every §4.6 debt and §5 owner item was found placed; no scope creep was found.
- **Method:** one finding (H1) was MEASURED in a `git archive` export.

Every finding is **taken**; the disposition says where.

| finding | disposition in revision 2 |
|---|---|
| **H1** — a plain name `..` became a path segment; a browser's save landed in the server's own `rev7/over/` (measured) | TAKEN: §2.1's name must begin with a letter or digit; containment restated; the cell rewritten; `archive::paths` and `DirArchive::put` refuse `.`/`..` as defence in depth; §3's cells |
| **H2** — "under `open` only" cannot live in `User::parse`, the mode-less key grammar at six sites | TAKEN: §2.1 is now two rules (the key grammar for every caller; the mode rule in `resolve`), the six sites listed, the new absent-mapping named as new, body-borne names under `cf-access`/`worker` accepted as unheld keys, the `unlocked` cell |
| **H3** — the version bump reaches more than `engine_version`: a drift warning on every existing game's screen, and the bridge's served version | TAKEN: §5 lists what it reaches; §2.6 filters the drift line in the layer, pinned; R-6.5c2.19 states the cost, and the owner ruled the bump (§10.2) |
| **H4** — the address is in nine lines across five files, two of them frozen plans | TAKEN: R-6.5c2.16 lists all nine; the owner ruled the exception for the two frozen plans (§10.2) |
| **M1** — the door's "current production" label had no source under R-6.5b2.7 and R-6.5b.12 | TAKEN: the `production_bot.json` contract file, emitted by `wire_fixtures`, refused when stale (§2.4). `TableView.bot` carries the per-table label for every row, undealt included, so no `Health` field is needed |
| **M2** — "every game route refuses" would block a finished game's download and score; the check has one site | TAKEN: R-6.5c2.10 narrowed to not-over games, read as the owner's words about resuming; the site is `live_handle` before `from_record`; a finished foreign game opens, the conversion warning is not shown, and `why` answers the foreign-bot refusal |
| **M3** — on the Worker the conflict never reaches the module; a different-game conflict is stranded, not owed | TAKEN: §2.5's three shell changes, `settleUpload`'s arm named; R-6.5c2.11's per-host wording |
| **M4** — the bump makes the live replay's first sends 409; the reset recipe must run first | TAKEN: §3 step 5; §8.2 lists the keys |
| **M5** — the Docker filter missed inputs the Dockerfile copies; the GHA cache doesn't carry cache mounts | TAKEN: §2.9's filter (`crates/*/Cargo.toml`, the workflow file), the cache statement, the Dockerfile's stale comment |
| **M6** — the states' discriminators; `YOKOHAMA_PROJECT_URL`'s injection | TAKEN: §2.1's A–E discriminators; §2.3's `define` with validation in the config |
| m1 `TurnMirror` is `Copy`; the schema answer | TAKEN: §2.4 (four reads take a reference; `TABLE_SCHEMA` stays 3) |
| m2 two more "watch" sentences | TAKEN: §2.2 lists three |
| m3 the bump's ripple list | TAKEN: §8.2 |
| m4 CI's identity-cell count and the feature-name cell | TAKEN: §2.1's ripple, §2.9 |
| m5 `CLAUDE.md` is 105,857 bytes | TAKEN: §2.11 |
| m6 `anonymous` under `open` | TAKEN: refused as a name by the key grammar (§2.1) |
| m7 `run_id` and the mirror's `bot` only after the deploy | TAKEN: stated in §2.4 and accepted |
| m8 the lifecycle command, not `lock add` | TAKEN: R-6.5c2.12 names both commands |
| m9 the client id stays in history; the test file's value | TAKEN: R-6.5c2.5, §2.1 |
| m10 where the `.dev.vars` module lives | TAKEN: §2.8 |
| m11 the deploy drops open pickers | TAKEN: §2.10 |
| m12 the mirror carried the name only | TAKEN: the mirror carries `(controller, version, params)` (§2.4) |
| o1 the pair property under overlap | TAKEN: holds and is pinned for the refinish race; best-effort under overlap, not claimed (§2.5, §6) |
| o2 the pool race cell's feasibility | TAKEN: §2.5 notes it; the plan confirms the `uploads` map |
| o3 an owed upload meeting a new bot | ANSWERED by M2's narrowing: the game is over, so it opens and stamps (§2.4) |
| o4 the NAS's TTL and marker | DECIDED: none (R-6.5c2.2) |
| o5 the livelock needs both guards and the bot's seeds | TAKEN: §2.6 |
| o6 the local foreign row from the listing | TAKEN: §2.4 |
| escalations 1–6 | 1 left with a reason (§7); 2 in H1; 3 in §2.11; 4 in §2.9; 5 in §2.1; 6 in R-6.5c2.5 |

### 10.2 The owner's review

The owner read revision 1 in parallel with the review and answered the two items revision 2 left open
(2026-09-26):

| item | the owner's answer | where |
|---|---|---|
| R-6.5c2.16: the address in two frozen plans | "Record a personal-data exception": edit both lines, and log the exception in the decision log | R-6.5c2.16 |
| R-6.5c2.19: the version | "Bump to 0.6.0": one number across the workspace and both npm packages, the release commit tagged, and a bump at every release that changes which bot plays | R-6.5c2.19 |

*(Any further notes from the owner's read are recorded here with their dispositions.)*
