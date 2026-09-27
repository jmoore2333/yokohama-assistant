# M6.5c2 — the cleanup: spec

**Status: DRAFT for review, 2026-09-26.** Written at the close of the M6.5c2 brainstorm with the owner, from
[the M6.5c2 handover](2026-09-26-web-frontend-m6.5c2-handover.md). Under review by the owner and, in parallel,
by a Fable reviewer; the review's findings and their dispositions will be recorded in §10, dated. **This
document is the authority for M6.5c2.** The handover is its entry and is not a spec. Where this spec and a
frozen artifact disagree, the frozen artifact wins unless a ruling here names it and supersedes it
(R-6.5c2.3 supersedes R-1.4's framing of `claimed`; R-6.5c2.11 supersedes the M6.5c spec §2.6 conflict
arm).

**The cycle in one paragraph.** This is the last cycle of the web lane before it goes quiet and the project
returns to the computer player (the owner, 2026-09-22 and 2026-09-26). It finishes the front door and the
lobby, turning the development placeholders into designed screens. It turns the manual-identity "dev door"
into a real self-host door that needs no identity service. It makes every save say which bot, which release
and which host produced it, so saves can be evaluated when later bots are trained. It closes every design
debt the M6.5c reviews listed. It arranges the two hosts' code and documents, writes the testing strategy
down, and closes with one full recorded test sequence, both hosts deployed and the nine-PR stack merged.

---

## 0. Read order and sources

1. **This spec.** §1 is the rulings, §2 the mechanism, §3 testing, §4 the order of work.
2. **The approved mockups**, committed beside this spec:
   `docs/plans/2026-09-26-web-frontend-m6.5c2-mocks/01-…11-*.webp`. Their HTML sources are in the gitignored
   `.superpowers/brainstorm/20585-1790461301/content/`. The screenshots are the record; the HTML is a
   working reference for colours and spacing. Neither is a pixel contract: the implementer builds the design
   in the app's own stylesheet and components (§2.2).
3. **The M6.5c2 handover**: §4.6 is the debt list this spec adopts whole (R-6.5c2.1); §6 is the traps not
   to re-derive.
4. **The M6.5c close**: §6 the LEAVE list and owner items, §7 what it does not claim.
5. **The M6.5c spec** (`2026-09-22-web-frontend-m6.5c-objects-spec.md`), read through its §10.
6. **`CLAUDE.md`**: the gate commands. **Re-count every suite at your own HEAD.**

---

## 1. The rulings

Every ruling below was taken with the owner in the 2026-09-26 brainstorm, unless it says otherwise. The owner's
words are quoted where the owner gave them, so they can be found by search.

**R-6.5c2.1 — Scope: the whole debt list.** All three tiers the brainstorm presented are in: the wrap-up itself
(frontend, hosts, docs, testing strategy, account chores, the merge), the correctness debts (the refinish
issue, the whole-game gate's livelock, the module's structural invariants), and the refactor and cost debts
(the `table.ts` split, the rows-read cost, the restatements, the scripts' duplication, the reviews' LOW
findings and Task 8's minor items). Owner: "All three tiers". Each debt keeps the source the handover gives it
(§4.6); §2.7–§2.8 list them.

**R-6.5c2.2 — The NAS stays live, as the self-host path's reference host,** and is redeployed at this cycle's
close with this cycle's build. Owner: *"it's worth keeping the NAS live since it's the self-host path, and it
costs nothing to have available, and is less work then redeployign when we want to verify the self host
change … while we'll be working with cloudflare moving forward, this is still going to be of interest to those
that want to host on more traditional channels/infrastructure or self host."* The Worker remains the hosted
deployment the project develops against.

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
the server…", E the door with multiplayer off. Owner: "yes, this holds well".

**R-6.5c2.5 — The Worker's dev door stays a test instrument.** It has no screen, it's the one way the live tiers
play as several people over Access, it stays ON in production's vars until the public door, and
`DEV_DOOR_CLIENT_ID` moves from `worker/wrangler.jsonc`'s `vars` to a Worker secret (M6.5c close §6.3
items 13–14).

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
  This closes the "who starts" item carried since M6.5b; the server already accepts `start_seat`.
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
give an error to that effect."* §2.4 is the mechanism (mock 10).

**R-6.5c2.11 — The refinish rule: stamp on a same-game conflict.** At game over, a put that meets **an earlier
completion of the same game** (equal `seed`, `config` and `choices`) under `over/<id>` stamps the table as
archived. The archive keeps the first ending and the table is no longer stranded. A record of a *different*
game at that key keeps today's behaviour: the save stays owed and logged. Only a caller-chosen id can reach that
case. Owner: "Stamp on conflict". **Supersedes** the M6.5c spec §2.6 arm that left such a table owed (§2.5).

**R-6.5c2.12 — The `test/` prefix expires after 30 days,** by an R2 object lifecycle rule set once by the
orchestrator and read back to confirm the prefix is exactly `test/` (handover §4.5).

**R-6.5c2.13 — The full test sequence runs at the close only.** Owner: "Close only". The refactors are measured
task by task by the per-edit suites (the test pool and the whole corpus through the objects) against the M6.5c
close's recorded counts, re-counted at this cycle's first task. The standing testing strategy is written into
the lane document (§2.11).

**R-6.5c2.14 — CI: the Worker gets its own job, and the Docker image is built when its inputs change.** Owner:
*"split plus the docker build but by weekly it would be depending on the code changes, if nothing changes then
it shouldn't need a rebuild - so it should be dependent on those crates or codebase having any changes"* (§2.9).

**R-6.5c2.15 — The documents are laid out by host, with a lane document** (§2.11). Owner: "By host + a lane doc".

**R-6.5c2.16 — The owner's address is scrubbed** from the five lines of `deploy/RUNBOOK.md` §3 that spell it
(§3.7 twice, §3.8, §3.14 Step 4b twice), replaced by "the owner's address" with a dated note in place. Owner:
"Scrub now".

**R-6.5c2.17 — The merge, after the close:** #31 → #39 in stack order, **with merge commits**, never squash or
rebase. The TDD history stays intact and each cycle's PR stays its record. Owner: "In order, merge commits".

**R-6.5c2.18 — Process: the lane's standing rules** (handover §8). Every task is subagent-driven, with Opus
implementers and reviewers and a Fable whole-branch review. The orchestrator runs every step that touches the
account (deploys, secrets, the bucket, the live runs) and edits no code. Every agent writes its report to a
file. **Every task that changes what the client shows loads the `frontend-design` skill** (the owner named it
for this work), and builds against the committed mockups.

**R-6.5c2.19 — The release is versioned: `0.6.0`.** The workspace version, `[workspace.package] version` in the
root `Cargo.toml`, is still `0.5.0`, so every save any build since the V4 release wrote says
`engine_version: "0.5.0"`. That's the field `GameRecord`'s own doc names as how "a human sees which build wrote
the file". This cycle's release bumps it, and the lane document records the rule going forward: **bump at every
release that can change what a save means or which bot plays**, the AI lane's V5 ship included. *Proposed in
this spec, flagged for the owner's review: the number is a release decision.*

---

## 2. The mechanism

### 2.1 Identity: the open door (R-6.5c2.3, .4, .5)

**The server.** `crates/yokohama-web/src/identity.rs`:
- The mode `claimed` becomes **`open`** (`YOKOHAMA_WEB_IDENTITY=open`), and the cargo feature `dev-identity`
  becomes **`open-door`**, still a default feature. A build without it (`--no-default-features --features
  server`) still refuses the mode at start-up, by name. Its refusal sentence changes from "a development
  door, not a security boundary" to a sentence stating the trust model: anyone who can reach the server may use
  any name.
- The header stays `X-Yokohama-Claimed-User`. The name is claimed; only the mode is renamed. No alias:
  the NAS is the only deployment on `claimed`, and it is redeployed with `open` at this cycle's close.
- `/api/me`'s `mode` and `/api/health`'s `identity` report `open`.
- **The name rule, under `open` only.** The header's value is trimmed, runs of internal spaces collapse to one,
  and it is lowercased by ASCII rules (`to_ascii_lowercase`, as today; the name travels in a header). It is then accepted if it is either today's
  email shape, or a **plain name** of 1–32 characters drawn from ASCII letters, digits, space, `.`, `-` and `_`.
  The literal `anonymous` is refused, as under `worker`. Under `cf-access` and `worker` an identity must still
  be an email: a plain name there means the edge is misconfigured, and it reads as absent (state B).
- `archive::caller_segment` extends to plain names, with a space becoming `_`. It stays a readable
  transliteration that can collide (the M6.5b default).

**The client.**
- `SignIn.tsx` becomes the **name step** (mock 03): one field, Continue, the hint stating the rule above, and
  the honest sentence: "This server doesn't check who anyone is… Your tables stay with the name you sat down as,
  so switching names later leaves them behind."
- The name step normalizes exactly as the server does. The typed form ("Ben") is kept as the display name, and
  the identity key is the normalized one ("ben").
- The storage key `yokohama.claimed_user` is kept, so a browser signed in before the rename stays signed in.
- The lobby corner reads "Playing as <display name or identity>" (the pill "Ben ▾" at phone width). Under
  `open` its action is **switch player**, which clears the stored name and returns to the name step. Under
  `cf-access` and `worker` it is **change name**, which edits only the display name.
- `Door.tsx`'s four answers, plus the door's multiplayer-off arm, become the designed states A–E (mock 05, 06):
  - A: the expired-Access refusal that today carries a "Sign in again" link becomes a card with that one
    action.
  - B: `door.me.user === null` under `cf-access` or `worker`. `whoami.ts`'s header sentence moves into a "For
    the operator" fold.
  - C: health unreachable, today's "not connected". "Try again" re-runs the health and `me` reads.
  - D: `me` not yet answered, today's "connecting…". "Connecting to the server…" with skeleton rows.
  - E: `VITE_MULTIPLAYER=false`, today's one line. The door without the "Play with friends" card.
- **R-N3:** a first connection's `Link` word reads "connecting…", never "reconnecting…" (M6.5c close §6.3
  item 11).

**The Worker.**
- Its identity (`worker` mode, the verified Access assertion) is unchanged.
- Its dev door is unchanged in behaviour. `DEV_DOOR_CLIENT_ID` becomes a secret (`wrangler secret put`, from
  stdin, as the operator list was). The deploy guard's configuration checks follow it.

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
- The door still **asks the server nothing** (R-6.5b2.7).
- **The lobby** (mock 02): L1's three sections as rows (name and age · seat chips · one action), a gold edge on
  your own tables, lobby chat in a right rail (below the lists on a phone), folded by default (M6.4's ruling).
  Rows use only what `TableView` carries, plus the new bot label (§2.4).
- **The forms** (mock 07), per R-6.5c2.8:
  - the name placeholder "ID shown in the lobby if left blank", with no subtext;
  - no password placeholder;
  - players and first player as segmented buttons;
  - seats as colour rows in the engine's order (Red, Blue, Yellow, Green), with the bot offered at two players
    only, its version tag beside it.
- **The seed** sits under an Advanced fold and is **validated as it is typed, in the fold**, against the
  server's own rule (`WireSeed`: trimmed; an optional `+` and decimal digits; at most
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
  marker, `Help.tsx`'s section and `Unlock.tsx`'s screen. One server-written sentence carries "watch":
  the record route's refusal to a spectator, "the record is the players' — watch, download nothing"
  (`api.rs:1844`, pinned at `lib.rs:1194`). It is reworded, and its pin moves with it.
- **The single-player build's "Lobby" controls** (M6.5b2 close §6.3; handover §5 item 12): under
  `VITE_MULTIPLAYER=false`, no control on a local screen leads to a lobby.

### 2.3 The project link (R-6.5c2.6)

- **One build variable, `YOKOHAMA_PROJECT_URL`.** `web`'s build reads it and a build step validates it:
  - it parses as an absolute `https://` or `http://` URL, with no whitespace or control characters and at most
    200 characters;
  - on failure, the build **exits non-zero naming the variable and the reason**;
  - unset, the door draws no link.
- The link text is the URL without its scheme; it opens in a new tab.
- **The Worker** takes it from the environment of `cd web && npm run build`. The deploy guard already refuses
  assets that aren't the current `web/dist`, and the RUNBOOK recipe names the value used.
- **The Docker image** takes it as `--build-arg YOKOHAMA_PROJECT_URL=…`. A self-hoster builds the image from the
  repository, so that is their install step.
- A run-time value is **not offered**. The release is precached under a digest of its own bytes, so a server
  rewriting `index.html` at start-up would fail the service worker's install check. `deploy/README.md` says so.

### 2.4 The bot's version, and what every save says (R-6.5c2.10, .19)

- **The label.** A new layer module (`crates/yokohama-web/src/bots.rs`) maps a recorded bot identity to its
  short label (`v4-step9-draft1` → "V4"). It works through `Contestant::from_parts(controller, params)` and an
  **exhaustive match with no `_` arm** over `Contestant` and `NamedIdentity`, the discipline of the CLI's
  `eligible`. When the AI lane adds a V5 identity to core, **the web layer will not compile until V5 has a
  label**: that one line is the only web edit a new bot needs.
- The labels and the foreign-bot refusal (below) join `suggest`'s four refusals as the only words the layer
  authors itself. This spec rules both in.
- **Where it shows:**
  - the door's bot card and the local form (the current production bot, `PROVISIONAL_DEFAULT_BOT`);
  - each saved game on this device, and each game's player panel (the bot the game was dealt, from its own
    record's `run.seats`);
  - the lobby's bot chip.
- **The wire gains two fields,** and the corpus is regenerated and its diff read:
  - `ClientState.bots: { seat, label }[]`, from the record (empty for an all-human game);
  - `TableView.bot: string | null`, the label of the table's bot seat.
- **Where the lobby gets it, without a second truth.** `SeatDoc::Bot` deliberately does not store which bot
  (`table.rs`: "a second statement of it would be a second thing to keep true"). So:
  - a dealt table's label comes from **`TurnMirror`**, the record's cache, which `Games::persist` refreshes.
    It gains `bot: Option<String>`, the identity from the record, not the label.
  - an undealt table shows the current production bot, which is what `finalize` will deal.
  - `TableDoc` doesn't deny unknown fields, so an older binary ignores the new key. The plan confirms this and
    whether `TABLE_SCHEMA` must move.
- **A game whose recorded bot this build doesn't run refuses to open.**
  - The rule: its bot seat's descriptor (`controller`, `version`, `params`) differs from the one this build
    seats.
  - On the server hosts, every game route answers a typed refusal naming both labels ("This game was played
    against V4. This server now runs V5 and can't continue it"). The lobby row says so, and the table can still
    be abandoned, which archives it.
  - In the browser, the local list shows it under **"Will not open here"**, the `SEMANTICS_REV` pattern, with
    export and a two-click Remove.
  - An all-human game has no bot to mismatch and opens as today.
  - Nothing converts the bot's seat into a human seat on the web. The CLI's own resume behaviour is unchanged.
- **What every save names**, inside the file:

  | fact | field | status |
  |---|---|---|
  | the rules | `semantics_rev`, `rules_hash`, `terminal_digest` | already |
  | the exact bot: identity, version, every knob, its seed | `run.seats[i]` | already |
  | the release that wrote it | `engine_version` | the field is already there; made meaningful by R-6.5c2.19 |
  | where it was played | `run.run_id` = `web/worker`, `web/server` or `web/browser` | **new** |

  The layer sets `run_id` through the CLI's existing public `Session::set_run_id`, which needs no CLI change.
  This follows the BGA importer's `bga-import/<table>` precedent for the field. The layer learns which host it
  runs on from a value the host passes when it builds `Games`.
- **A pin, on every save path:** the store, the record route, the archive on both hosts, the browser's
  IndexedDB, the export and the backup envelope carry `run` through **byte-for-byte**. The lane document gets a
  "What a save file says" section for the evaluation pipeline, and the AI-lane handover points to it.

### 2.5 The refinish rule (R-6.5c2.11)

- **Where the rule lives.** It is a layer rule at the game-over archive site (`games.rs`: `archive_if_over` and
  `archive_over_retry`), so both hosts share it. The NAS cannot reach the race, because it holds its lock across
  the put and the stamp.
- **The rule.** A `PutError::Conflict` at `Why::Over` is followed by a same-game check:
  - The archived record's `seed`, `config` and `choices` are compared with the current record's.
  - If they're equal, the table is stamped `archived_at` as on `Landed`, and the new logging seam (§2.6)
    records "the archive keeps an earlier completion of <id>".
  - If they differ, the save stays owed and is logged.
- **The comparison is done in Rust**, never in the shell (the strings rule).
  - On the Worker, the receipt carries the held record's text, which the R2 put already reads when it answers
    `conflict`, back to the module.
  - On the NAS, `DirArchive` reads the file.
- **The archive's pair stays one game's pair.** When the record key keeps an earlier completion, the table key
  must not be overwritten by a later completion's table document. The plan decides the mechanism, and a cell
  pins it.
- **A pool cell reproduces the race:** an undo during a held R2 put, then a different finish. The table ends
  stamped, abandonable and expiring normally.

### 2.6 The layer's other changes

Each has an architectural reason (R-6.5c.4's refinement). Sources: handover §4.6 and M6.5c close §6.2.
- **A logging seam.** The layer's `eprintln!` is mute on the Worker, so a failed stamp in `archive_over_retry`
  can't report. This adds a `Log` seam on `Games`: stderr under `server`, the shell's log line on the Worker,
  the console in the browser host. Sites that print today print through it.
- **The positional-array `create` is refused.** serde accepts a JSON array for a struct. The layer's body
  extraction refuses any non-object body, in its own 400 shape, as the module's `init` already does.
- **The whole-game gate's vs-bot livelock** (M6.3 close §6.3; M6.4 and M6.5c §6.1). The arm's driving policy
  gains the placement probe the two-person arm already has, so a bad OS-entropy seed can no longer run to the
  20,000-input cap. The gate **returns to CI** (§2.9) once a recorded run of seeds shows it bounded.
- **The e2e's `CAP = 4000`** (M6.3 §6.3): the browser walk's driving policy gets the same probe, and the cap's
  hits are recorded in a sequence.
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
  - `.dev.vars` parsed in one module (`web/e2e/live.ts`, `worker/scripts/live-replay.mjs` and RUNBOOK §6.4's
    recipe use it or cite it);
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

- **`wasm-opt` 133 everywhere.** `deploy/Dockerfile` moves from 132 to 133 with binaryen's published SHA-256,
  and every build step prints the `wasm-opt` version it used (I-87).
- **CI, `.github/workflows/test.yml`:**
  - the Worker's steps move out of `web-server` into a job of their own, `worker`;
  - the whole-game gate rejoins `web-server` once §2.6's fix is shown bounded;
  - the header comment is updated.
- **A new workflow, `.github/workflows/docker.yml`,** builds the self-host image:
  - It runs **on push and pull request when the image's inputs change**: `deploy/**`, `crates/yokohama-core/**`,
    `crates/yokohama-cli/**`, `crates/yokohama-web/**`, `crates/yokohama-wasm/**`, `web/**`, `Cargo.toml`,
    `Cargo.lock` and `rust-toolchain.toml`. It can also be dispatched by hand.
  - It uses `docker buildx` with the GitHub Actions layer cache, then smoke-runs the image (`/api/health` 200,
    under `YOKOHAMA_WEB_IDENTITY=open`).
  - The plan confirms the input list against the Dockerfile's `COPY` lines. A change to any workspace member's
    `Cargo.toml` is an input, because the workspace must resolve.
  - GitHub applies path filters per workflow, which is why this is a workflow and not a job. The repo is
    private, so its minutes are counted, and the cache keeps them down.
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
- Existing NAS tables were seated under typed emails. Those still parse under `open`, so their seats still
  match.
- **The Worker** is deployed at the close with `DEV_DOOR_CLIENT_ID` as a secret. Its RUNBOOK records the
  deployment.
- **The bucket** gets the `test/` lifecycle rule, 30 days, set with wrangler's `r2 bucket lifecycle` commands
  (the plan confirms the syntax on R2's page first) and read back.
- **The public door stays closed** (the owner: Access stays until the repo is public). Its preconditions are
  recorded in the lane document, not executed:
  - the path-scoped Access application;
  - the dev door off;
  - the art's publication decision;
  - the project link set;
  - requests per move under a public load.

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
    with a **relocation map** (`§6.x` → `worker/RUNBOOK.md §x`), so every frozen document's citation still
    resolves.
- **`docs/web-lane.md`** (new) takes the web lane's operating detail out of `CLAUDE.md` (about 36 KB of its
  92 KB). It covers:
  - the hosts;
  - the suites and gates;
  - the traps;
  - **the standing testing strategy**, from the handover §4.4 table, updated: which tier runs when, what it
    proves, what skipping it costs;
  - **"What a save file says"** (§2.4);
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
- **The address scrub** (R-6.5c2.16).

---

## 3. Testing

- **Per task:** test first, one green commit with its red run quoted, and the per-edit suites for everything the
  task touches (`CLAUDE.md`'s list; the Worker's `npm run build:module:debug` before its pool). The first task
  re-counts every suite at the branch's HEAD and records the reading.
- **The new cells the spec requires** (the plan names each file):
  - **Identity:**
    - under `open`, plain names and emails are accepted and normalized by one rule;
    - under `cf-access` and `worker`, names are refused;
    - the build without the feature refuses `open` by name;
    - `/api/me` and `/api/health` say `open`.
    - The client's normalization is pinned to the server's over one shared table of inputs.
    - States A–E each render from committed wire payloads.
    - switch player.
  - **The bot version:**
    - the labels are exhaustive by construction (the compiler);
    - `TableView.bot` for a dealt table (from the mirror) and an undealt one (production);
    - `ClientState.bots` from the record;
    - **a record naming another bot refuses on every game route with the typed refusal**, while a same-identity
      record opens;
    - the local "Will not open here" row;
    - **`run.run_id` stamped by each host**;
    - **`run` byte-identical through every save path** (§2.4's pin).
  - **The refinish rule:** a same-game conflict stamps; a different-game conflict stays owed; the archive's pair
    stays one game's; the pool reproduces the race.
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
  - **The livelock:** a recorded run of the whole-game gate over many seeds shows its input count bounded
    before it rejoins CI.
- **The mutation battery**, at the reviewed head: every new catcher above gets a row, and each is reverted and
  must fail.
- **The close, one recorded, machine-tagged sequence** (R-6.5c2.13):
  1. the per-edit suites of every crate and package;
  2. the fifteen release gates (core is compiled at a new version, and the merge follows);
  3. `npm run e2e`;
  4. the deploys, the Worker then the NAS, with `npm run build` in `web/` first and the `debug_trap` grep;
  5. the live replay twice;
  6. the quota guard, then the live Playwright tier and the headed exit game;
  7. the battery;
  8. the Fable whole-branch review.

  A live step follows the handover §4.4 rules: WARP's device login checked first, the quota guard, and one
  Playwright run at a time.

---

## 4. The order of work

This is proposed; the plan fixes the tasks.
1. **The layer and identity:** the `open` mode and plain names; the bot labels, the mirror field, the two wire
   fields, `run_id` and the foreign-bot refusal; the refinish rule and the logging seam; the positional
   `create`; the livelock; the corpus regenerated and its diff read.
2. **The client:** the shared components and the board mark; the door and the project link; the identity states
   and the name step; the lobby; the forms and the seed check; waiting and joining; the "spectate" words; the
   local "Will not open here"; the phone layouts and their width cells; Task 8's items.
3. **The Worker and the module:** the `table.ts` split, one read per request, the picker, the scrub fence, the
   receipt list, the restatements, the time units, L-8 and L-4, the module's figures.
4. **The scripts and the live tiers:** R3's list and the LOW findings.
5. **The build and CI:** `wasm-opt` 133, the CI split, the Docker workflow, the gate back in CI, the version
   bump.
6. **The hosts:** the secret, the `test/` rule, the deploys (the close's sequence).
7. **The documents:** last, so they describe the final state.
8. **The close:** the full sequence, the battery, the review, the close document, then the merge (R-6.5c2.17).

---

## 5. The fences

- **The five fenced crates** (`yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge`,
  `gen-data`): **source byte-identical**. R-6.5c2.19's version bump changes the version they're compiled
  with, which reaches `engine_version` in new records and nothing else. That is provenance only, by
  `GameRecord`'s own doc.
- **`SEMANTICS_REV` stays 7. No bot behaviour moves.**
- **The CLI: no change is needed.** `Session::set_run_id`, `PROVISIONAL_DEFAULT_BOT` and core's
  `Contestant::from_parts` are already public, and `eligible` is untouched. If the plan finds a seam it
  truly needs, it is a behaviour-preserving widening and is named. The nine `assert_capture` goldens and
  `guide_sync` are how that is known.
- **The layer changes, each named with its reason:** §2.1, §2.4, §2.5, §2.6.
- **The wire contract changes, each read in the corpus diff:**
  - `/api/me`'s `mode` and `/api/health`'s `identity` (`open`), and `server_version` (0.6.0, if R-6.5c2.19
    stands);
  - `TableView.bot` and `ClientState.bots`;
  - the reworded server sentence;
  - the foreign-bot refusal's body.

---

## 6. What this cycle does NOT do

- **No public door.** Access stays on both hosts. The path-scoped application, the dev door's removal and the
  art's publication are recorded preconditions (§2.10).
- **No bot choice.** Players see the bot's version; they never pick it (R-6.5c2.10).
- **No resume across a bot change.** A game whose recorded bot this build doesn't run refuses to open, by the
  owner's ruling. The CLI's own resume behaviour (the seat comes back human, with a warning) is not changed.
- **No house password and no accounts** (R-6.5c2.3). The open door's trust model is "anyone who can reach the
  server".
- **The phones** stay the owner's: the iPhone exit and the Android handset, recorded, not gated.
- **No change to requests per move** (~13) beyond the rows-read work. Requests remain the Free plan's first
  binding limit, and are a public-door precondition.

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

## 8. Risks the plan must answer

1. **The corpus diff is large** (two new keys on every table and state payload). It's a contract: read it whole.
   A key appearing where it shouldn't is what the leakage pins exist for.
2. **The mirror field and older binaries:** confirm `TableDoc` ignores unknown keys on read; if it doesn't,
   `TABLE_SCHEMA` moves and RUNBOOK §5 records a schema rollback.
3. **The foreign-bot refusal's reach:** confirm which routes rebuild a session (the reads included) and that the
   refusal comes before any rebuild, so a converted human seat never exists even for one request.
4. **The version bump's ripple:** goldens, fixtures and corpus files that spell `0.5.0` are found before the
   bump, and each is either a record (kept) or an output (regenerated).
5. **The Docker workflow's minutes:** the first runs are measured and the cache confirmed.

## 9. Process

The handover §8 in full, and R-6.5c2.18. Model dispatch follows the owner's standing rule: Opus is the floor for
implementers and reviewers, and Fable orchestrates and reviews the whole branch. Reports go to files; a reviewed
SHA is never amended; while an implementer works, nothing runs in its tree.

## 10. Review record

*(To be filled: the Fable review's findings and the owner's notes, each with its disposition, dated.)*
