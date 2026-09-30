# M6.5c2 — the cleanup: spec

**Status: revision 2.6, 2026-09-27.** Written at the close of the M6.5c2 brainstorm with the owner, from
[the M6.5c2 handover](2026-09-26-web-frontend-m6.5c2-handover.md).
- **Revision 1** (`6ce33fd6`) went to the owner and, in parallel, to a Fable adversarial review. The review found
  0 CRITICAL, 4 HIGH, 6 MEDIUM, 12 MINOR and 6 OPEN, and checked 50 claims true.
- **Revision 2** applies every finding. §10 records each finding with its disposition.
- **Revision 2.1** records the owner's answers to the two items revision 2 left open (§10.2).
- **Revision 2.2** takes the owner's note on the dev door: the open door on the Worker as well, with the
  repository's defaults staying Access (R-6.5c2.5; §10.2).
- **Revision 2.3** applies the Fable re-read of revision 2 (§10.3).
- **Revision 2.4** applies the Fable re-read of revision 2.2, the Worker's two doors (§10.4). It is the one
  revision the owner asked for once the review was complete, with no further round.
- **Revision 2.5** (this text) corrects, in place, the sentences that writing the plan found wrong or incomplete.
  Most were MEASURED by the plan's drafters in `git archive` exports; the rest were read at HEAD. §10.5 records
  each one with its evidence. It changes no ruling.
- **Revision 2.6** (this text) records how the cycle is EXECUTED, ruled by the owner on 2026-09-27 after the single
  28-task plan drafted for revision 2.5 was retired (§10.6): **three cycles** in dependency order — A the layer,
  B the screens, C the hosts and the close — each on its own stacked branch with its own plan, its own
  orchestrator session and its own PR (§4, §9; R-6.5c2.17 and R-6.5c2.18 amended), and **the pure refactors of
  §2.7 and §2.8 dropped** (R-6.5c2.1 amended; each dropped item is marked where it stands). It changes no
  mechanism. The plan review's one CRITICAL finding — RUNBOOK §2's recipe served the repository root, where
  `.dev.vars` sits, to the LAN — is fixed in this revision's commit series, before any cycle's first task.
- **Revision 2.7** (this text, 2026-09-28) records the rulings of cycle C's planning session with the owner, taken
  against the tree cycle B left (`web-room-m6.5c2b` at `3ed27f07`, PR #40): cycle C is ONE cycle and carries the
  hosts' code, the host documents, the deploys and the live tiers through its close document; a **fourth, lighter
  cycle D** — the final wrap-up and polish — takes the lane document and the top-level document refresh, the
  visual polish the owner's eye decides, the investigation of what remains, and then the merge, the `v0.6.0` tag
  and the refocus (R-6.5c2.17 and R-6.5c2.19 amended; §4's table gains row D). The owner's 2026-09-28 ask — the
  bot's version in the saves and in the R2 key — is answered WITHOUT a key change: every save already names the
  bot, the release and the host, and an ingestion script sorts (§2.4 as amended). The lobby gains a folded
  "Finished" group (§2.2 as amended). CI stays lean (§2.9 as amended). §10.7 records each ruling with the owner's
  words. It changes no mechanism of §2.1, §2.5–§2.8 or §2.10.
- **Revision 2.9** (2026-09-29) records cycle E's planning session with the owner (§10.11): row E amended, the
  rulings R-6.5c2.37–.44, and the update-path defect found live during the session — the edge's JavaScript
  Detections snippet in `/index.html`, refused by the service worker's digest for every returning browser.

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
down, and closes with one full recorded test sequence, both hosts deployed and the PR stack merged — eleven PRs
since revision 2.6, the three cycle branches among them (§4).

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

**Amended 2026-09-27 (revision 2.6). Owner: "Drop the pure refactors."** The refactor tier is NARROWED to the items
with measured or correctness value, because the lane goes quiet after this cycle and a refactor of code nobody will
edit again buys nothing. **DROPPED**, and marked where each stands in §2.7 and §2.8: the `table.ts` split; the
read-once threading of the documents (measured to save 0–2 rows where the `hasTables` memo saves 30); the replay
script's split into five modules; and the four script dedups (the `.dev.vars` module, the one origin constant,
`HOST_FIELDS`/`order()`/the credential's key names, the one precache parser). **KEPT**: the `hasTables` memo (F-4's
measured fix), the picker invariant, the scrub fence, the receipt's pending list, the operator listing's one time
unit, L-8 and L-4, the module's figures in one place, the restatements imported, every LOW finding (L-3, L-5, L-6,
L-7, L-9) and Task 8's client items. The dropped items stay in the handover's §4.6 as debts the lane leaves, with this
ruling as their reason.

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

**R-6.5c2.5 — The manual door is first-class on both hosts, and the repository's defaults stay Access.** Owner,
reading revision 1's version of this ruling: *"it's likely that once this is released public - peopel are
unlikely to spin up an identity provider to play locally at home, so it's doing multiple things and we should
treat it like that - while it is very much our dev door, it's a first class feature for others with a different
intent"*; chosen: "Open door on the Worker too"; and *"but note our default settings in the repo and
documentation woudl be for access, we're building in the options for potential uptake in a more public
setting"*. So:
- **The Worker gains the open door as a deployment choice** (§2.1, "The Worker's two doors").
  `YOKOHAMA_WEB_IDENTITY` becomes a Worker variable with two values: `worker` (Access, verified by the shell,
  which is this repository's configuration) or `open` (players choose a name, and no identity provider is
  needed). It has no default.
- **Every committed configuration, and every document's first path, is Access:**
  - `worker/wrangler.jsonc` names `worker`;
  - `deploy/README.md` and `worker/README.md` lead with Access and give the open door its own section, trust
    model included;
  - the NAS runs `cf-access` (§2.10).
- **Our dev door stays our test instrument, under `worker` only.** It is the credentialed claim the live tiers
  use to play as several people over Access, and it stays ON in production's vars until the public door.
  - `DEV_DOOR_CLIENT_ID` moves from `worker/wrangler.jsonc`'s `vars` to a Worker secret (M6.5c close §6.3
    items 13–14).
  - Its value stays in git history and in RUNBOOK §6's records. That's accepted: it is the credential's
    *public* half by the M6.5c design.
  - Under `open` a credential means nothing, since a claim needs none. `DEV_DOOR_CLIENT_ID` is ignored there, and
    `YOKOHAMA_DEV_DOOR=on` beside `open` is refused: by the guard from `vars`, and by the shell at run time
    (§2.1).
- **The deploy guard refuses a configuration that names no door, and prints the one it names.** That answers the
  M6.5c close §6.3 item 14 in part. Requiring the dev door off remains a public-door precondition, if that door is
  Access.

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
**Amended 2026-09-27 (revision 2.6):** the three cycle branches (§4) are three stacked PRs — `web-room-m6.5c2a`
against `web-room-m6.5c`, `web-room-m6.5c2b` against `-a`, `web-room-m6.5c2c` against `-b` — expected #39, #40 and
#41 (GitHub numbers them at creation), so the stack merged at the close is #31 → #41. Owner: *"keeping up the
stacked branch and PR per cycle will be ideal if we are going to run each cycle with its own orchestrator, it makes
for the cleanest handoff and history tracking"*.
**Amended 2026-09-28 (revision 2.7):** the merge is cycle D's closing act, not cycle C's — the owner (2026-09-28):
*"it's much better to have a lighter phase d and then we can engage in a real in-depth final straw poll/investigation
on what's remaining and then wrap it up."* Cycle C ends as A and B did, with its PR (#41 against `-b`); cycle D's
branch `web-room-m6.5c2d` stacks on `-c` with its own PR (#42), and the stack merged at D's close is #31 → #42, in
order, with merge commits.

**R-6.5c2.18 — Process: the lane's standing rules** (handover §8). Every task is subagent-driven, with Opus
implementers and reviewers and a Fable whole-branch review. The orchestrator runs every step that touches the
account (deploys, secrets, the bucket, the live runs) and edits no code. Every agent writes its report to a
file. **Every task that changes what the client shows loads the `frontend-design` skill** (the owner named it
for this work), and builds against the committed mockups. **Amended 2026-09-27 (revision 2.6):** each of the three
cycles (§4) runs under its own orchestrator session, from a plan written at that cycle's start against the tree the
cycle before it leaves, on its own stacked branch; plans are written by the orchestrator or ONE agent, never by
parallel drafters (§10.6).

**R-6.5c2.19 — The release is versioned: `0.6.0`.** Proposed in revision 1 and **ruled by the owner** (2026-09-26,
"Bump to 0.6.0"): one release number across the workspace and both npm packages, and the release commit
tagged.
- **The tag is `v0.6.0`, made AFTER the merge, on master** (review R2-M2). A `v*` tag runs
  `.github/workflows/release.yml`: three OS builds of the CLI, its tests and the `piped_dogfood` gate, then
  `gh release create` from the tagged commit. So the tag publishes the 0.6.0 CLI release from the merged master,
  never from a web-lane head, and its run joins the close's sequence (§3).
- Every crate takes the workspace version, `[workspace.package] version` in the root `Cargo.toml`, still
  `0.5.0`. So every save any build has written since the V4 release says `engine_version: "0.5.0"`.
  `GameRecord`'s own doc names that field as how "a human sees which build wrote the file".
- This release bumps it. The lane document records the rule going forward: **bump at every release that can
  change what a save means or which bot plays**, the AI lane's V5 ship included, and tag the release commit.
- **The bump is not free** (review H3): §5 lists everything it reaches, the most visible is handled in §2.6,
  and the live replay's reset follows it (§3). The owner took it knowing its cost.
- **Amended 2026-09-28 (revision 2.7):** the bump itself is cycle C's (§8.2's ripple lands there, and C's release
  gates, deploys and live tiers run at 0.6.0); the **tag** follows the merge, so it is cycle D's closing act
  (R-6.5c2.17 as amended).

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
4. **The module's `identity_of` fallback follows the mode rule** (review R2-m1). Today a raw value no header can
   carry is answered by `User::parse(raw)` directly, which under the widened grammar would trim `"ben\r"` to
   `ben` and accept it, bypassing the email shape. The fallback now applies the configured mode's rule after the
   parse (or refuses outright), with a cell through `identity_inner`. That path is unreachable through the shell
   today, and the cell is what keeps it so. The browser host calls `User::parse` directly and needs no mode rule.

**The archive's containment is restated, not assumed** (review H1, escalation 2):
- `caller_segment` keeps `[a-z0-9._-]` and maps everything else to `_`. For an email, `@` becomes `_at_`. For a
  plain name, the leading letter or digit guarantees a segment that is never empty, `.` or `..`.
- Its cell (today's email-only containment test in `archive.rs`) is **rewritten**, not left green. It feeds the
  names `.`, `..`, `. .`, `-` and `.x` to `User::parse` and asserts each is refused, and asserts the segment of
  every accepted name begins with `[a-z0-9]`.
- **Defence in depth** (review R2-m3): `archive::paths` returns a `Result`, refusing any path component that is
  `.` or `..`, and its callers (the archive route, `DirArchive::put`, `ReceiptArchive::put`) handle the refusal.
  `DirArchive::put`'s "stays under its root" check is **lexical**, because `canonicalize` needs the file to
  exist: **every component of the relative path `paths` built is `Component::Normal`**, checked before the join
  (§10.5: a check over the joined path lets an absolute path through, and refuses every put on a host whose
  configured directory is itself relative with `..`). The review's export probe (`traversal_probe.rs`) is the
  regression cell's template.

**The rename.** The mode `claimed` becomes **`open`** (`YOKOHAMA_WEB_IDENTITY=open`), and the cargo feature
`dev-identity` becomes **`open-door`**, still a default feature.
- A build without it (`--no-default-features --features server`) still refuses the mode at start-up, by name.
  Its refusal sentence states the trust model instead of calling the mode a development convenience.
- The header stays `X-Yokohama-Claimed-User`: the *name* is claimed, and only the mode is renamed. There's no
  alias: the NAS is the only deployment on `claimed`, and it is redeployed on **`cf-access`** at the close
  (§2.10). So **the NAS is not redeployed from this branch before then**: once the rename lands, no build accepts
  the `claimed` its container sets, and it would exit 1 at start-up (§10.5).
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
  - `worker/src/identity.ts`'s and `identity.unit.test.ts`'s sentences that the shell's `isAddress` agrees with
    `User::parse` "character for character", which now describe the mode rule's email shape (review R2-m8);
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

**The Worker's two doors** (R-6.5c2.5).
- **The variable.** `YOKOHAMA_WEB_IDENTITY` becomes a Worker variable, `worker` or `open`, with **no default**.
  Unset, every request answers the host-written 500 naming the variable, as every other required variable does
  (the M6.5c spec §10's "every variable's 500"). `Env`'s `ROWS` record forces the new variable's row.
  `worker/wrangler.jsonc` names `worker`.
- **Every object reads the door from its own configuration, like every other variable**, because each object
  builds `shellConfig` and `moduleConfig` from its constructor's env. So the pool proves the two doors with
  **two pool projects** (review R2.2-M2): today's over `wrangler.test.jsonc` (`worker`), and a second over a new
  `wrangler.open.test.jsonc` (`open`), as two `cloudflareTest` projects under **`test.projects` in
  `worker/vitest.config.ts`** (Vitest 4 removed the workspace file; §10.5). The open project's cells live under
  `worker/test/open/`, which the `worker` project excludes, so the corpus walk runs under `worker` alone. The alternative,
  a per-request door header from the shell, would add a production mechanism for the tests' sake, and is not
  taken. The unset-door cell needs neither: the stateless Worker's `handle` refuses before any object is
  reached, so a cell's env override at `handle` shows the 500.
- **Two names move with the door** (review R2.2-m2), both fixed at the module's `init`:
  - `identity_of`'s mode is `IdentityMode::Worker` under `worker` and `IdentityMode::Open` under `open`.
  - The wire's door name, `Host.identity_name`, keeps reading `cf-access` under `worker`, as today and as
    `web/src/types.ts` and the client's `useIdentityMode` already know. It reads `open` under `open`.
- **The module's dependency on the layer enables the `open-door` feature** (review R2.2-m1). It depends on
  `yokohama-web` with `default-features = false`, so the feature would otherwise be off and `open` would refuse by
  name. With it on, no Worker build refuses `open` at compile time, which is the intent: the compile-time refusal
  belongs to the axum host built without the feature.
- **Under `worker`, nothing changes.** The shell verifies the Access assertion. The dev door (the credential
  plus `X-Yokohama-Claimed-User`) and a person's assertion both set `X-Yokohama-User`, which the objects' layer
  reads in `worker` mode. The one change is the mode rule's new absent-mapping in the shared `resolve`.
  - The live tier's claimed header (`web/e2e/live.ts`) is unaffected, because the header name stays.
  - `DEV_DOOR_CLIENT_ID` becomes a secret by **ONE `wrangler deploy --secrets-file`** at the close: the same
    version drops the var and adds the secret, so the dev door is never shut in between. `wrangler secret put`
    is refused while the deployed version holds the name as a var (cloudflare/workers-sdk#5756; §10.5). The file
    is written from `.dev.vars` with mode 600 and deleted after, never printed or committed. `env.ts` and
    `identity.ts` already read the value from `env`, so no code changes. The deploy guard's check 7 extends
    to it, and exempts `wrangler.test.jsonc`, whose test value stays for the pool (review m9).
- **Under `open`, the shell reads no Access assertion.** It strips every client `x-yokohama-*` header as today,
  and forwards the caller's claimed name as `X-Yokohama-Claimed-User`. So the objects' layer runs in `open` mode:
  the key grammar and `/api/me`'s `mode: "open"` come from the layer, and the client shows the name step as it
  does on the NAS.
  - There is one grammar, the layer's. The shell **never judges a name beyond what a header can carry**: the
    claimed value, from the header or the percent-decoded query and so possibly any Unicode, is held to
    `forwardable` (visible ASCII, tab and space) through the shell's existing `address()`, judged raw and then
    trimmed and ASCII-lowercased (the Kelvin lesson). That happens before `headers.set`, which would otherwise
    throw into the platform's plain 500 (review R2.2-m3).
  - An open-door claim is **its own `Identity` kind, `claimed`**, beside `person`, `dev` and `none` in
    `worker/src/identity.ts` (review R2.2-m6). `operatorGate`'s person-only rule and `objectFor("archive")`'s
    naming then read it without a special case, and `dev` stays the live tier's kind under `worker`.
- **The claim's carriers.**
  - A `fetch` carries the header the client already sends.
  - **A browser cannot put a custom header on a WebSocket upgrade** (`new WebSocket(url.href)` in
    `web/src/ws.ts`). So under `open`, the client adds the name to the upgrade's URL as a query parameter
    (`claimed=<name>`), and the shell reads it there, on upgrades only.
  - The axum host's stream is a `fetch` and needs neither change.
- **What stays under `open`:**
  - the Origin check on upgrades;
  - the strip of client `x-yokohama-*` headers;
- **What is refused under `open`: the operator surface.** A claimed name is never an operator's rights, as the dev
  door's claim never was (the M6.5c spec). An open deployment is operated through wrangler and the dashboard,
  which `worker/README.md` says. Two instruments carry the rule (review R2.2-M1):
  - **`operatorGate` is door-aware:** under `open` it answers the `off` kind, the layer's 404 "as if no operator
    were configured", for every claimed name. Today it answers `refused` (the 403) for every non-person once the
    list is set.
  - **`shellConfig` refuses a contradictory configuration at run time.** It refuses `YOKOHAMA_WEB_ADMINS` set
    beside `open`, and `YOKOHAMA_DEV_DOOR=on` beside `open`. Every `/api/*` request then answers the 500 naming
    both variables, which is the Worker's form of refusing to start. The deploy guard cannot do this for the
    operator list: it is a **secret**, and the guard reads only the committed configurations (its check 7 exists
    to keep that list *out* of `vars`).
- **The quota is the deployer's.** Anyone who can reach an open Worker on the Free plan can exhaust it. That's
  documented, with Cloudflare's own rate limiting as the deployer's tool.
- **The archive's `test/` root requires an email** (review R2.2-m4). `ReceiptArchive::root` files a save under
  `test/` when the creator's address ends with `.test`. A plain name such as `a.test` is legal under the key
  grammar, and would then have its saves expired by R-6.5c2.12's 30-day rule. The test root now requires an
  address with an `@`, so a plain name never takes it.
- **The deploy guard, per door** (reading the committed `vars` only):
  - it **refuses a configuration with no `YOKOHAMA_WEB_IDENTITY`**, since it can't print a door that isn't there,
    and it prints the one named (review R2.2-m5);
  - under `worker`, the Access variables are required and `workers_dev` must be false, because a `workers.dev`
    URL would go around the hostname-wide Access application;
  - under `open`, a `workers.dev` URL is allowed (there's no door to go around), and `YOKOHAMA_DEV_DOOR=on` in
    `vars` is refused;
  - **`preview_urls` stays false under both doors**: a preview version is not the deployed code, whatever the
    door.
- **No open-door Worker is deployed this cycle.** The owner's Worker stays `worker`, and the open door is proven
  in the test pool, where the shell runs in workerd with real sockets (§3).

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

- **Amended 2026-09-28 (revision 2.7) — the lobby's "Finished" group (cycle C).** The `playing` scope lists
  finished games by the status rule (`tables.rs`: "over is not a `TableStatus`"), and a finished game's row under
  "Being played" reads "Spectate" (`web/src/tables.ts`), so on the NAS — which has no TTL (R-6.5c2.2) — that list
  would accumulate finished games without end. The owner (2026-09-28): *"we would need the view to prune or
  collapse or otherwise no longer show finished games after some time so it doesn't add up, but I can see value if
  you have the password or entitlement to go back into a finished game."* Ruled: **client-only.** "Being played"
  lists only unfinished games; finished ones sit in a **"Finished" group below it, folded by default** with a count
  on its summary, newest first, "View" on every row, the padlock and the unlock face unchanged for a locked table.
  No wire change and no data change: a table's lifetime stays each host's rule, and the entitlement is already
  the rule (a seat holder finds it under "Your tables"; a stranger with the password passes the unlock face into
  the finished game's screen). Built in mock 02's row language and the one fold idiom the app has
  (`details.fold`, R-B16). A time rule ("after some time") would need a finished-at time the mirror does not
  carry; it is a cycle D item if the count ever bothers the owner. This closes B's polish item 1 ("Spectate" on a
  finished game's row) by construction.
- **Amended 2026-09-28 (revision 2.7) — polish, and where it lands.** The owner: *"polish is key, these are the
  last revisions here, and we're dedicating substantial resources so we want to get it well enough to last for
  some time while we dive deeper into the computer player."* The standard for every screen change in C: 1440 px
  and 390 px pictures beside its mock in the task's ledger, and a headed look in the whole-branch review. Of B's
  nine "owner's eye" items (the C handover §7), **1–4 are cycle C's**, folded into its a11y task — the finished
  row's word (above), the secondary controls' 44 px touch target at 390, the device card's h2 → h4 skip, the open
  table's refusal placed by its field — and **5–9 are cycle D's**, the polish phase, judged from C's pictures:
  `a.primary-link`'s box against `button.primary`'s, state E's two-up at 976 px, the band's crumbs wrapping at 390
  under a long address, the Players panel's tag order, and the device card's storage line and backup note with
  the board's bottom three hues (R-B12). Every carried client fix of B's whole-branch review and the a11y pass
  stay C's, as B's close routed them.
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
- **The owner's NAS** is not built from `deploy/Dockerfile` (§2.10), so it takes the link from the environment of
  the one `cd web && npm run build` that both of the owner's hosts ship.
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
    `{"identity": …, "label": …}` of `PROVISIONAL_DEFAULT_BOT`. It is **emitted and refused exactly as**
    `site_space_words.json` is (review R2-m6). Unlike that file, which is consumed as a pin, this one is imported
    by the three components. It joins `contract.ts`'s typed set and the leakage pins' scan; it carries no secret,
    and the anti-vacuity partners still enumerate every file.
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
  - `TurnMirror` loses `Copy`, so its by-value reads take a reference (review m1): **six sites**, not four
    (`table.rs` three times, `tables.rs`, `games.rs`'s mirror refresh, and one test; §10.5).
  - `TableDoc` carries no `deny_unknown_fields`, and `check_shape` bounds values only. A nested
    `#[serde(default)]` keeps **`TABLE_SCHEMA` at 3**. An older binary reads the new key and ignores it, and an
    older module's house copy drops it, which is harmless (review m1).
  - **An undealt table** shows the current production bot (`runs_here: true`), which is what `finalize` will
    deal.
  - **A table dealt before this deploy** has no `bot` in its mirror until its next applied move, and a
    **finished** one never gains it, because no `persist` follows game over. Its row shows "the bot" without a
    version. That's accepted, and honest (review m7).
- **The foreign-bot rule** (review M2; R-6.5c2.10). **The check has one site:** `Games::live_handle`, through
  which every rebuild on all three hosts passes. The descriptors are compared after the record's parse; the
  refusal returns **between `Session::from_record` and `Repl::new`**, because "not over" is a fact about the
  replayed state (§10.5). A converted human seat still never exists, even for one request: `Session::from_record`
  never reads `run` and builds no `Repl`, and the conversion is `Repl::new`'s. A bot seat's recorded
  `(controller, version, params)` that differs from `PROVISIONAL_DEFAULT_BOT`'s descriptor is **foreign**.
  - **A game that is not over, with a foreign bot, refuses to open.** Every game route answers a typed refusal
    naming both labels: "This game was played against V4. This server now runs V5 and can't continue it." The
    lobby row shows `runs_here: false` and says so. `delete` reads the record's event count and never rebuilds,
    so Abandon still archives it.
  - **A finished game with a foreign bot opens as today:** frame, replay, `/result` and the record download.
    The bot's label comes from the record.
  - `Repl::new` is the CLI's only constructor, and it converts the foreign seat to a human inside the resident
    `Repl` (`from_provenance`). So "opens as today" rests on **the layer masking that conversion at every place
    it shows** (review R2-M1). There are four places:
    1. **the warning:** the session's conversion warning ("…it resumes as a HUMAN seat, so you are now playing
       it") is not shown;
    2. **`why`:** on a move made by that seat, `why` answers the authored refusal "this move was made by V4, which
       this server doesn't run", built in `explain.rs` from `session.record().run` before `why::account_parts`
       is reached. It **never** says the move was a human's. This is a **thirteenth refusal shape** on `GET
       …/why`: it gets a corpus sample and a `contract.ts` entry, and `explain::kind_of`'s exhaustive match
       does **not** fence it, because it is the layer's, not the CLI's (review R2-m5);
    3. **undo:** `input`'s `undo` arm **refuses on any game whose recorded bot is foreign**, on the one-human path
       (`api::undo_allowed` admits a one-human table with no over-check today) and the shared path alike.
       Otherwise the CLI's undo, seeing no bot, pops the bot's final move and leaves an unplayable game;
    4. **the Players panel:** on the web, `is_bot` derives from the record's `run.seats`, the same source as
       `ClientState.bots`. That **re-rules `client::facts_for`'s sentence** that `is_bot` is "still the REPL's
       own reconciled answer ([`Repl::bots`]), never re-derived here". The re-ruling is scoped to a foreign seat
       on a finished game, and everywhere else the two agree.
  - **An owed game-over upload that was undone during its upload** (the §2.5 race), and then meets a deploy with a
    new bot, is not over and foreign. Its alarm's `stamp_archive` is refused, and it retries on its backoff until
    the creator's Abandon or expiry clears `archive_owed`. Both `delete` paths never rebuild. That is bounded, not
    a strand (review R2-o1).
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
  - The host is a value each host passes to `Games::with_parts`. That is the constructor on all three hosts: the
    server reaches it through `AppState::with_store_chat_and_archive` (review R2-m7).
  - A record written before this deploy keeps no `run_id` (review m7).
- **A pin, on every save path:** the store, the record route, the archive on both hosts, the browser's
  IndexedDB, the export and the backup envelope carry `run` through **byte-for-byte**. The lane document gets a
  "What a save file says" section for the evaluation pipeline, and the AI-lane handover points to it.

- **Amended 2026-09-28 (revision 2.7) — the bot's version in the key: NOT taken.** The owner asked (2026-09-28,
  the C handover §7) for the bot's version in the saves, digestibly, and in the file name or R2 key, so saves can
  be organized by bot version for evaluation. Read against the table above: every save already names the exact
  bot (`run.seats[i]`), the release (`engine_version`), the rules and, since cycle A, the host (`run.run_id`); the
  one fact in no file is the label "V4", a lookup from the controller name. A key segment would touch both
  archives, the Worker's receipt plans, the pool's key expectations, the live replay's reset keys and three
  documents, plus a one-time bucket move. The owner: *"a simple python script could ingest the saves and sort
  them … if this adds a substantial amount of work, it's likely then not worth the savings in just evaluating it
  when we bring in the save data, since we're not looking at large numbers anyway … if we do it in ingestion
  instead of just labeling that can work also."* Ruled: **no key change and no record change**; the ingestion
  script sorts by the fields the save carries, and two cheap items land instead:
  - **"What a save file says"** is the contract that script is written against: the fields above, the label
    table, and the byte-for-byte rule — in the lane document, which is cycle D's (§2.11 as amended);
  - **the versioned refusal sentence** (cycle C, `bots.rs`): where two labels tie, the foreign-bot refusal names
    the controller version — "played against V4.2. This server now runs V4.3" — instead of "V4 … V4". The screens
    keep "V4", `label` is unchanged, and this closes the same-name re-point edge cycle A carried (its handover
    §7).

  On the map's fence: the Phase-3 map §1 item 6 reads "Human-game benchmarking is a late, optional,
  comparison-only activity, never a training input" (`docs/plans/2026-07-25-phase3-map.md` line 18). The owner
  (2026-09-28): the fence exists *"because our corpus was so small in the initial development cycle — but, there
  will be a point where more human games will be valuable"*. Nothing here changes it; when it changes, it is a
  decision-log row amending map §1, and the lane document's contract is what the pipeline reads either way.
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

    The answer is read by a new `settledArm`, not today's `stampArm`: a different game's conflict answers
    `{"stamped": false, "over": true}`, which `stampArm` reads as `retry`, so it would retry hourly for ever
    (§10.5). `settledArm` keeps every conflict **final** except the one the layer stamped, which is
    R-6.5c2.11's Worker rule.
- **The archive's pair** (review o1). For the undo-then-refinish race, the pair already stays one game's:
  `executePlan` reads the record key **before** it puts the table, so a conflict writes nothing. A cell pins
  that. Two *overlapping* uploads of different records can still leave one completion's record beside another's
  table, and R2 offers no cross-key atomicity. That case is recorded as best-effort; no cell claims it.
- **Two pool cells reproduce the race**, each an undo during a held R2 put, then a different finish: one where
  the first upload settles before the second starts, and one where **both are in flight at once**. The table ends
  stamped, abandonable and expiring normally. The hold is the pool's measured `swapBucket` (`worker/test/gates.ts`,
  which every R2 race cell already uses), with a two-gate variant for the overlap; it is restored in `finally` and
  discarded by an eviction, per the pool facts (§10.5). `deliverArchive` runs outside `blockConcurrencyWhile`, so
  the undo can interleave. The shell's `uploads` map keys one in-flight upload per plan key, and two completions
  are two records, so two keys: the map does not serialize them, and the overlapping cell asserts both uploads in
  flight (review o2, measured `uploads.size === 2`).

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
  - The filter can only match the CLI's own wording, a prefix match on `warning: GameRecord.engine_version=`:
    `Session::load_warnings` is a `pub Vec<String>`, and a typed alternative would need a CLI change. So **the pin
    must not be vacuous** (review R2-m2). Its cell rewrites the fixture pair's (`browser-played-2p`: the
    production bot, the current `rules_hash`) `engine_version` to a value **derived to differ from the running
    version**, so the drift line fires before the bump as well as after it (§10.5). With the session's premise
    of exactly one warning, it asserts **`warnings == []`, no warnings at all**, while the record keeps the
    version it was given. A cell looking for the filter's own prefix would pass silently the day the CLI's wording
    moved; an empty-array assertion fails loudly.
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
  become items the layer **exports** and the module imports, so the source-text pin retires with its reason. The
  browser host (`crates/yokohama-wasm`) restates the clock and the scan too, and switches to the same exports, so
  each is stated once (§10.5). The exported `error_body` carries the foreign-bot refusal's arm (§2.4).
- **A cell for `archive_over_retry`'s second `NotFound` arm** (Task 1's M6), now that the layer isn't fenced.

### 2.7 The Worker's shell and the module (handover §4.6)

Every item is **behaviour-preserving**: the pool and the corpus through the objects must stay byte-identical,
which is how that is shown rather than claimed.
- **DROPPED (revision 2.6, R-6.5c2.1 as amended) — `worker/src/table.ts` (2,082 lines) splits along its four
  concerns:** the transport helpers beside `frames.ts`; the slot half (~330 lines) as `delivery.ts`; the archive
  half (~380 lines) behind a narrow host interface; and one guarded-call helper replacing the three copies.
- **The rows read per request (F-4).** About 470 rows are read per move live. **Measured in the pool, ~94% of a
  warm request's rows are `hasTables`' scan of `sqlite_master`**, which runs inside every `getMeta` and every
  `readDocs`: a warm request reads 32 rows, 30 of them that scan (§10.5). Revision 2.5 took both fixes below;
  **revision 2.6 keeps the first and drops the second** (R-6.5c2.1 as amended):
  - a per-object memo of `hasTables`, forgotten at `deleteAll()`, which takes a warm request from 32 rows to 2,
    with a cell that a warm request asks `sqlite_master` nothing;
  - **DROPPED (revision 2.6):** reading the documents once per request and threading the result (it saves 0–2
    rows, and is structural), with a cell that counts document reads per request.

  The close claims F-4 only by its measured live reading.
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
- **The scripts — DROPPED, every sub-item (revision 2.6, R-6.5c2.1 as amended); the list stays as the record of the
  debt the lane leaves:**
  - `.dev.vars` is parsed in **one module**, `worker/scripts/dev-vars.mjs`, with its type declaration
    **`dev-vars.d.mts`** (TypeScript reads no `.d.ts` for an `.mjs` import; measured TS7016; §10.5).
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
  - **every test binary of the layer is named**, because `browser_played_pin` (M6.5b) arrived after the
    `--lib --test wire_fixtures` filter and has run in no workflow; a step fails when `crates/yokohama-web/tests/`
    gains a binary no step names (§10.5);
  - the refusing-build step printed nothing on failure: GitHub runs a `run` step as `bash -e`, so a red `cargo`
    inside `out=$(…)` ends the step before its output is printed (measured). It gains `set +e`, as does the
    gate's step (§10.5);
  - the header comment is updated.
- **A new workflow, `.github/workflows/docker.yml`,** builds the self-host image:
  - It runs **on push and pull request when the image's inputs change** (review M5), which `deploy/Dockerfile`
    fixes: it copies `rust-toolchain.toml`, `Cargo.toml`, `Cargo.lock`, **all of `crates/`**, `web/package*.json`
    and `web/`. The filter is `deploy/Dockerfile*`, `deploy/smoke-image.sh`, `crates/yokohama-core/**`,
    `crates/yokohama-cli/**`, **`crates/yokohama-bridge/**`** (the CLI depends on it, so the image compiles it),
    `crates/yokohama-web/**`, `crates/yokohama-wasm/**`, **`crates/*/Cargo.toml`** (every member resolves),
    `web/**`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` and **`.github/workflows/docker.yml`**. A
    document edit under `deploy/` pays no cold build (§10.5). It can also be dispatched by hand.
  - It builds with `docker buildx` and the GitHub Actions **layer** cache, then smoke-runs the image under
    **both** doors (`/api/health` 200 and its `identity` field under `cf-access`, then under `open`), so the
    option is proven to ship in the image the repository's default runs. The smoke is a script,
    `deploy/smoke-image.sh <image> [port]`, so it runs first on a locally built image rather than for the first
    time in CI, and a self-hoster can run it too.
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

- **Amended 2026-09-28 (revision 2.7) — CI stays lean.** The owner: *"our dev process is heavy on build tests and
  e2e tests where relevant, computer player changes are not going to impact the web replays given how the rules
  engine is structured, so we should not overload CI with tests that we're already running heavily, it should be
  useful for CI specifically and not overburdened."* So this section's additions are read as a CEILING: CI runs
  what a clean checkout can build and what the per-edit process does not already run heavily, and cycle C adds
  no suite to `test.yml` beyond the whole-game gate's return (about 25 s at cycle B's close) and the step that
  names the layer's test binaries. The **Docker workflow's trigger is narrowed** to pull requests, pushes to
  master and manual dispatch over the image's WEB-LANE inputs — `deploy/Dockerfile*`, `deploy/smoke-image.sh`,
  `crates/yokohama-web/**`, `crates/yokohama-wasm/**`, `web/**`, `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml` and the workflow file — and **never on a change under `crates/yokohama-core`,
  `crates/yokohama-cli` or `crates/yokohama-bridge` alone**, so an AI-lane push pays no cold image build; a core
  change that breaks the image is caught at the next web-lane pull request or by dispatch, which the lane
  document records. The first runs are measured (§8.3), and if a cold build reads over fifteen minutes the
  trigger drops to master pushes and dispatch.
### 2.10 The hosts (R-6.5c2.2, .5, .12)

- **The NAS** gets this cycle's release:
  - built as it is today, from **`deploy/Dockerfile.runtime`**: a static musl binary and the `web/dist` of the
    one `npm run build` both hosts ship, built on the workstation and delivered as a context the NAS fetches,
    because the NAS runs kernel 3.10 (RUNBOOK §1–§3; §10.5). It takes `YOKOHAMA_PROJECT_URL` from that build's
    environment if the owner sets one, and the binary keeps the default `open-door` feature;
  - run with **`YOKOHAMA_WEB_IDENTITY=cf-access`** behind the same Access application: Access is its door and its
    identity, **the repository default** (R-6.5c2.5). Revision 2 said `open`; the owner's defaults note moved
    it. The open door is exercised by the local browser suite (the axum binary in `open`) and by the Docker
    workflow's smoke run;
  - its size and memory are re-measured and recorded. The full-build image of `deploy/Dockerfile` (R3's E-3)
    is proven by a local build and by the Docker workflow (§2.9), not by the NAS;
  - **the deployment is not done until `/api/me` is read over WARP, as the owner** (review R2.2-M3), recording
    `{"mode":"cf-access","user":…}` and the owner's own table listed under `scope=mine`. That is **the first
    live reading of the layer's `CfAccess` arm on any host**. Every NAS deployment so far ran `claimed`, and the
    Worker's live "cf-access" readings are the module's `Host` name, not that arm. A visit with the service
    credential (no `email` claim) reads as state B, by design.
- Existing NAS tables were seated under typed emails (`claimed`). Under `cf-access`, a seat matches only where
  the typed address is its holder's Access address. The NAS holds the owner's own tables, typed with the owner's
  address; any other seat is recorded at the deploy rather than migrated. There is no TTL, marker or operator
  surface (R-6.5c2.2).
- **The Worker** is deployed at the close with `DEV_DOOR_CLIENT_ID` as a secret, in the same version (§2.1's
  one `--secrets-file` deploy). Its RUNBOOK records the deployment.
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
  - `deploy/README.md` (new) is **the self-host guide**. It **leads with Access** (`cf-access` behind a Cloudflare
    Access application, the repository's default) and gives **the open door its own section**, for a home, LAN
    or VPN deployment with no identity provider. It covers:
    - building the image and its build args;
    - running it, with its variables;
    - the identity modes and the open door's trust model;
    - the multiplayer switch;
    - the archive directory;
    - putting it behind a proxy or Cloudflare Access;
    - what's precached and why the project link is build-time.
  - `deploy/RUNBOOK.md` keeps §0–§5 with their numbers, the NAS's operations and records. Its current-state
    text that describes `claimed` (§0, §2's Modal sentence, §3's recipe, §3.2's verification) and
    `deploy/cloudflared-config.yml`'s comment are rewritten for `cf-access`, because the close follows that
    recipe (§10.5).
  - Its §6 moves **verbatim** to `worker/RUNBOOK.md`, beside `worker/README.md`. The old §6 becomes a pointer
    with a **relocation map** (`§6.x` → `worker/RUNBOOK.md §x`), so every frozen citation still resolves.
  - The review counted 75 lines citing RUNBOOK §6 outside it: 37 in frozen plans, the rest in code comments and
    documents.
  - **`worker/README.md`'s 18 citations** are updated to the new file directly (escalation 3), and it gains
    **"The two doors"**: Access first, as this repository deploys it, then the open door for a deployment with
    no identity provider (the variable, the trust model, the quota exposure, what the guard allows).
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
- **`guidingpath.md`** gets two decision-log rows: R-6.5c2.16's personal-data exception, when the documents
  are written, and the M6.5c2 cycle row at the close, with its readings (§10.5). Its current state is refreshed.
  **The board** (`docs/handover.md`) has its "previous update" history rewritten into pointers.
- **One "how the documents are built" note** in the lane document:
  - generated and checked: `guide_sync`, the wire corpus;
  - frozen at their close: the dated `docs/plans/` records;
  - current-state pointers: `CLAUDE.md`, the board, the READMEs.
- **The root `README.md`** gets the two-README balance, pointing at the self-host guide and at
  `worker/README.md`. **Seven** of `worker/README.md`'s screenshots are recaptured after the redesign: the door,
  the lobby, the join and the open table, plus the watcher's game screen (it shows the old "You are watching"),
  `marker-404.png` (the old lobby) and `door-phone.png` (the old door) (§10.5).
- **The address scrub** (R-6.5c2.16), including the two frozen plans under the recorded exception.
- **Amended 2026-09-28 (revision 2.7) — what moves to cycle D.** The owner: *"you could probably move the lane
  document to D as well since we're likely to be editing those same docs in D which will be immediately after C,
  as we won't have full deploys so no use prior to then anyway."* So **cycle C keeps the host documents** — the
  two READMEs' and RUNBOOKs' rewrite above (the NAS recipe must read `cf-access` before the deploy that follows
  it), `worker/README.md`'s two doors and citations, the address scrub with its decision-log row, the stale lines
  the C handover names — plus a **truth pass on `CLAUDE.md` at C's close** (the sentences C makes false: the NAS's
  mode and build, `dev-identity`, the version, the counts) and the board's pointer, as every close does. **Cycle D
  takes** `docs/web-lane.md` and `CLAUDE.md`'s reduction to a pointer (one move, one commit), `guidingpath.md`'s
  current state, the board's history rewritten into pointers, the root README's two-README balance, and the
  seven screenshots — recaptured from the DEPLOYED site through an opt-in generator, after C's deploys.

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
    - the mode rule: under `cf-access` and `worker` a non-email header reads as absent, including the
      module's `identity_of` fallback (`"ben\r"` through `identity_inner`);
    - **a table whose `unlocked` list holds a plain name reads on every host**;
    - the build without the feature refuses `open` by name;
    - `/api/me` and `/api/health` say `open`.
    - The client's normalization is pinned to `User::parse` over one shared table of inputs, emitted by
      `wire_fixtures`.
    - States A–E each render from committed wire payloads, with the discriminators of §2.1.
    - switch player.
    - **The Worker's two doors, in the pool's two projects** (§2.1):
      - under `open`, a claim reaches the objects by the header on a `fetch` and by the query parameter on an
        upgrade, with no assertion needed;
      - `/api/me` says `open`;
      - a claim a header cannot carry (a non-ASCII query value) is no caller, never a platform 500;
      - under `worker`, the claimed header and parameter are ignored without the credential, and `/api/me`
        still says `cf-access`;
      - an unset `YOKOHAMA_WEB_IDENTITY` answers the 500 (an env override at `handle`);
      - under `open` with no operator list, `/api/admin` answers the layer's 404 whatever name is claimed;
      - under `open` with `YOKOHAMA_WEB_ADMINS` set, or `YOKOHAMA_DEV_DOOR=on`, every `/api/*` answers the 500
        naming both variables;
      - a plain name ending in `.test` files its saves outside `test/`;
    - the guard's per-door rules (the absent door, `workers_dev` by door, `preview_urls` under both, the dev
      door under `open`) each fail a mutant.
  - **The bot version:**
    - the labels are exhaustive by construction (the compiler);
    - `production_bot.json` is refused when stale;
    - `TableView.bot` for a dealt table (from the mirror), an undealt one (production) and one dealt before the
      deploy (none);
    - `ClientState.bots` from the record;
    - **a not-over game with a foreign bot refuses on every game route with the typed refusal**, checked in
      `live_handle` before any `Repl` is built (§2.4);
    - **a finished foreign game opens, scores and downloads**, its `warnings` carry no conversion line, and
      `why` on its bot's move answers the foreign-bot refusal (a corpus sample, and `contract.ts`'s entry),
      never "a human's move";
    - **undo on a foreign game is refused at a finished one-human table**, and the table stays finished;
    - **the Players panel's `is_bot` and `ClientState.bots` agree** on the finished fixture pair;
    - a same-identity game opens;
    - the local "Will not open here" row, with export and Remove;
    - **`run.run_id` stamped by each host**;
    - **`run` byte-identical through every save path** (§2.4's pin);
    - **the drift-line filter** (§2.6).
  - **The refinish rule:**
    - a same-game conflict stamps, on both hosts;
    - a different-game conflict keeps each host's rule;
    - the pair stays one game's in the undo-then-refinish case;
    - a different game's conflict on the Worker stays final (`settledArm`);
    - the pool reproduces the race, with the uploads overlapping and not.
  - **The seed:** the client's check is pinned to `WireSeed`'s verdicts on one shared table of inputs.
  - **The project link:** the build refuses each class of bad value, naming the variable; the door draws the
    link only when it's set.
  - **The screens:**
    - jsdom cells over the committed corpus for every rebuilt component (nothing hand-authored, the M2.5
      rule);
    - browser cells at desktop width and 390 px for the door, the lobby, the name step, the forms, waiting and
      joining, with no sideways scroll;
    - a headed look, because no jsdom cell sees a stylesheet.
  - **The refactors:** the pool and the corpus through the objects stay byte-identical; a cell counts
    document reads per request, and one shows a warm request asks `sqlite_master` nothing.
  - **The livelock:** a recorded run of the whole-game gate over many seeds, with the bot stream seeds recorded,
    shows its input count bounded before it rejoins CI.
- **The mutation battery**, at the reviewed head: every new catcher above gets a row, and each is reverted and
  must fail. The containment and foreign-bot rows go first.
- **The close runs in the M6.5c close's order**, so a fix found in review never forces a second deploy or a
  second live sequence (§10.5):
  1. the Fable whole-branch review, one fix wave, and the scoped re-review;
  2. the battery, at the reviewed head;
  3. **one recorded, machine-tagged sequence** (R-6.5c2.13): the per-edit suites of every crate and package; the
     fifteen release gates (core is compiled at a new version, and the merge follows); `npm run e2e`;
  4. the deploys, the Worker then the NAS, with `npm run build` in `web/` first and the `debug_trap` grep; the
     NAS's `/api/me` read over WARP as the owner, as §2.10 requires;
  5. **the live replay's reset recipe first** (trap 19: the version bump changes the walk's records, so the
     first sends would meet a 409 and stop; review M4), then the live replay twice;
  6. the quota guard, then the live Playwright tier and the headed exit game;
  7. the screenshots (§2.11), the close document and the cycle's decision-log row;
  8. after the close, the merge (R-6.5c2.17), then **the `v0.6.0` tag on master and its `release.yml` run**,
     read to green (R-6.5c2.19).

  **Amended 2026-09-28 (revision 2.7):** steps 1–6 and the close document of step 7 are cycle C's; **the
  screenshots of step 7 and the whole of step 8 are cycle D's** (§2.11 and R-6.5c2.17 as amended). C's close
  document and its decision-log row are written at C's reviewed head, as A's and B's were.

  A live step follows the handover §4.4 rules: WARP's device login checked first, the quota guard, and one
  Playwright run at a time.

---

## 4. The order of work — three cycles (revision 2.6)

**Ruled by the owner, 2026-09-27** ("Three cycles"; "Drop the pure refactors"; the branches `web-room-m6.5c2a`,
`-b`, `-c`), after the single 28-task plan drafted for revision 2.5 was retired (§10.6). Every host consumes the
table layer, and the client and the Worker are independent of each other once the layer's wire is committed as
corpus files — so the work is **three cycles in dependency order**, each on its own stacked branch with its own
plan, its own orchestrator session and its own PR, and **each planned AT ITS START against the tree the cycle
before it leaves**, by that cycle's orchestrator or one agent. A plan is written to be read whole by one agent; the
sizes this project has executed (1,000–3,300 lines, 9–16 tasks) are the measure, and cycle A's may run larger only
because its layer edits are file:line-precise Rust across three crates, where the code IS the contract.

| cycle | branch (its base) | plan | content | ends with |
|---|---|---|---|---|
| **A — the layer** | `web-room-m6.5c2a` (on `web-room-m6.5c`) | `2026-09-27-web-frontend-m6.5c2a-plan.md` | §2.1's key grammar, containment, rename and mode rule (the client's four literal sites and the walk's configuration move with the rename); §2.4 whole — `bots.rs`, the contract file, the mirror, the two wire fields, the foreign-bot rule and its four masks, `run_id` and the byte-for-byte pin on the server's save paths; §2.5's layer half; §2.6 whole — the log seam, the drift filter, the object-only body, the restatements EXPORTED, the `NotFound` cell, the whole-game gate's livelock and the browser walk's cap; §2.2's three reworded server sentences; **the `seed_cases.json` contract** (§2.2 "The seed"), emitted here so cycle B renders nothing hand-authored; the corpus regenerated and its diff read; every cell the two host crates and the pool need to stay green at each task | the Fable whole-branch review, ONE fix wave and a scoped re-review; the per-edit suites at the reviewed head; the branch pushed and its PR opened (pre-authorized); **the handover into B, which is cycle B's planning session's ENTRY** — written off the tree, naming the wire as it is, the drafts and their corrections, and what A does not claim. **No deploy and no live tier** (R-6.5c2.13). |
| **B — the screens** | `web-room-m6.5c2b` (on `-a`) | written at B's start | §2.2 and §2.3 whole — the shared pieces, the door and the project link, the identity states A–E and the name step (§2.1 "The client", pinned to `identity_cases.json`), the lobby, the forms and the seed check (pinned to `seed_cases.json`), waiting and joining, "Spectate" in the client, the single-player links, "Will not open here", the game screen's foreign refusal; §2.8's client items (Task 8's list); every rebuilt screen's jsdom, browser and 390 px cells | as A: review, fix wave, re-review, push and PR, a handover note into C; **no deploy** |
| **C — the hosts and the close** | `web-room-m6.5c2c` (on `-b`) | written at C's start | §2.1 "The Worker's two doors" whole (the client's `claimed=` upgrade parameter with it); §2.5's Worker half; §2.7 as amended — the `hasTables` memo, the picker invariant, the scrub fence, the pending list, the time unit, L-8, L-4, the module's figures, and the restatements IMPORTED (the module and the browser host switch to A's exports); §2.8 as amended — the LOW findings; §2.9 whole; R-6.5c2.19's bump and its ripple (§8.2); §2.10 and §2.11 whole; then §3's close | §3's close in full: the review, the battery, the ONE recorded sequence, the deploys, the live tiers, the screenshots, the close document, the merge (R-6.5c2.17) and the `v0.6.0` tag |
| **D — the wrap-up and polish** (revision 2.7) | `web-room-m6.5c2d` (on `-c`) | written at D's start, from C's handover | the lane document and the top-level refresh (§2.11 as amended); the seven screenshots from the deployed site; polish items 5–9 and whatever C's pictures show (§2.2 as amended); the phones, after C's deploy, with the Device Hub iOS simulator as an option (§6 as amended); **the investigation of what remains** — the LEAVE lists, the M6.5c close's "does NOT claim", the public door's preconditions, the AI lane's re-entry pointer — the owner's "straw poll" on what the lane leaves; whatever overflows C | the merge #31 → #42 with merge commits, the `v0.6.0` tag on master and its `release.yml` run read to green, the refocus on the computer player |

**Amended 2026-09-28 (revision 2.7):** cycle C is ONE cycle (the owner: *"one cycle, this is a substantial
catch-all type phase wrap"*), and **row D absorbs what grows past C's plan while it is drawn** (the owner: *"the
phase D content can be dependent if as we're drawing the scope for C that this grows too far it can get moved into
D"*). Row C's content, restated for revision 2.7: everything its row names LESS the screenshots, the merge and
the tag, PLUS the "Finished" group and polish 1–4 (§2.2 as amended) and the versioned refusal sentence (§2.4 as
amended); its documents are §2.11 as amended. The dropped refactors of R-6.5c2.1 stay dropped in D as well (the
owner: *"the refactors dropped don't have to come in even in D that's fine"*).

**The interfaces between cycles are the layer's, and this spec fixes them.** A produces, for B: the corpus files
`identity_cases.json`, `seed_cases.json`, `production_bot.json`, `table_bot.json`, `foreign_bot_409.json`,
`table_foreign.json` and `why_foreign.json`; `TableView.bot` and `ClientState.bots`; `/api/me`'s `mode: "open"` and
`health.json`'s `identity: "open"`; and the `types.ts` and `contract.ts` entries beside each. For C: `IdentityMode::Open`
and `identity::admit`; `bots::*`; `RunHost` as `Games::with_parts`'s last parameter; `log::{Log, NoLog, …}` and
`Games::with_log`; `PutError::Conflict { key, held }`; the exports `api::error_body`, `api::MAX_WAIT_SECONDS`,
`api::check_wait`, `api::NOT_AN_OBJECT`, `signal::house_frame`, `games::HostClock` and `platform::{CALLS, calls_in}`;
and the whole-game gate's measured cap with `YOKOHAMA_WEB_GATE_BOT_SEED`. Each plan names what it consumes verbatim in
its "Names shared across tasks", and B's and C's plans READ them off A's tree rather than off this list.

**Standing across all three cycles:** **the push and the PR at each cycle's close are pre-authorized** (the owner,
2026-09-27: *"push and PR are pre-approved at the orchestrator's discretion"*), and each cycle's close writes the
handover the NEXT cycle's planning session starts from; the NAS is NOT redeployed from any cycle branch before C's
close (after A no build accepts `YOKOHAMA_WEB_IDENTITY=claimed`, and the live container sets it); the fences of §5 hold
at every commit of every cycle; the per-edit suites of every crate and package a task touches run after every task, the Worker's
`npm run build:module:debug` before its pool; and the retired plan review's findings land in the cycle that owns
the code (§10.6's table).

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
    changes while its source does not. The bridge suite is unaffected because **no bridge cell asserts the served
    version at all** (review R2-m4);
  - `/api/health`'s `server_version`.
- **`SEMANTICS_REV` stays 7. No bot behaviour moves.**
- **The CLI: no change is needed.** `Session::set_run_id`, `PROVISIONAL_DEFAULT_BOT` and core's
  `Contestant::from_parts` are public, `Repl::bots()` is public, and `eligible` is untouched. If the plan finds
  a seam it truly needs, it is a behaviour-preserving widening and is named. The nine `assert_capture` goldens
  and `guide_sync` are how that is known.
- **The layer changes, each named with its reason:** §2.1, §2.4, §2.5, §2.6.
- **The Worker module's identity mode becomes configured** (§2.1, the two doors): `identity_of` takes the
  deployment's mode rather than `IdentityMode::Worker`, and the module's dependency on the layer enables
  `open-door`.
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
- **No open-door Worker is deployed.** The owner's Worker stays Access, and the open door is proven in the pool
  (R-6.5c2.5).
- **The phones** stay the owner's: the iPhone exit and the Android handset, recorded, not gated. **Amended
  2026-09-28 (revision 2.7):** they wait for C's deploy (the owner: *"The phones are delayed for an actual deploy
  or there is nothing to test"*), and cycle D may layer them in through the Device Hub iOS simulator — D's plan
  decides.
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
     archive-walk.json`'s embedded records (so the live replay's reset recipe runs, §3);
     **`web/src/wire-script/backup-envelope.json`'s `app_version`**, which a LIB cell writes
     (`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --lib backup`, not the `wire_fixtures` command; §10.5);
     `deploy/Dockerfile.runtime`'s comment; `web/e2e/plays_locally.spec.ts`'s stale `yokohama-web 0.5.0-m6.5b`
     literal;
   - **records, kept as written:** `worker/test/fixtures/owed-game-over.json` and `owed-pump.json`, the web
     fixture pair `browser-played-2p.{json,table.json}`, `worker/scripts/smoke-release.test.mjs`'s fake body;
   - **never touched:** `worker/package-lock.json`'s third-party `@cloudflare/kv-asset-handler@0.5.0`, which a
     blind replace would corrupt (`npm version` leaves it; measured);
   - **the four bucket keys** under `test/browser/red_at_wire.test/rev7/over/` that the reset recipe removes.
3. **The Docker workflow's minutes:** the first runs are measured cold.
4. **The livelock's bound:** measured over recorded seeds before the gate rejoins CI (§2.6).

## 9. Process

The handover §8 in full, and R-6.5c2.18. Model dispatch follows the owner's standing rule: Opus is the floor for
implementers and reviewers, and Fable orchestrates and reviews the whole branch. Reports go to files; a reviewed
SHA is never amended; while an implementer works, nothing runs in its tree. **The owner's answer on scale
(2026-09-27, revision 2.6, superseding 2026-09-26's "Opus, merged tasks" for one plan):** three cycles (§4), each
planned and executed by its own orchestrator session on its own stacked branch. Within a cycle every task is one
Opus implementer and one Opus reviewer, with its own green commit and its red run quoted; folding two small tasks
into one execution unit is the cycle orchestrator's call, and a unit's reviewer reviews its commits together.

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

| R-6.5c2.5, noted after the read: the dev door is "a first class feature for others with a different intent" | "Open door on the Worker too", and "our default settings in the repo and documentation woudl be for access": R-6.5c2.5 rewritten; §2.1 "The Worker's two doors"; the NAS on `cf-access` (§2.10); the documents lead with Access (§2.11) | R-6.5c2.5, §2.1, §2.9, §2.10, §2.11, §3, §5, §6 |
| The rest of the spec | "rest of the spec looks good" | — |

### 10.3 The Fable re-read of revision 2 (2026-09-26, `ba448284` + `e1173ef8`)

The re-read is appended to the same report.
- **Verdict:** PLANABLE WITH TWO CORRECTIONS.
- **Counts:** 0 CRITICAL · 0 HIGH · 2 MEDIUM · 8 MINOR · 1 OPEN.
- **Confirmed:** 26 of §10.1's 28 dispositions hold as mechanisms, and 16 of revision 2's new sentences check
  true.
- **Stability:** no code moved, so every file:line and the H1 measurement stand.

Every finding is **taken** in revision 2.3.

| finding | disposition in revision 2.3 |
|---|---|
| **R2-M1** — a finished foreign game at a one-human table can be undone into an unplayable one, and its Players panel calls the bot's seat a human | TAKEN: §2.4 lists the four places the layer masks the conversion (warning, `why`, **undo fenced on foreign games**, **`is_bot` from the record**, re-ruling `facts_for`'s sentence by name); §3's cells |
| **R2-M2** — a `v0.6.0` tag runs `release.yml` and publishes a CLI release | TAKEN: R-6.5c2.19 tags `v0.6.0` after the merge, on master, and the run joins §3's close |
| R2-m1 `identity_of`'s fallback bypasses the mode rule | TAKEN: §2.1 rule 4, and a cell |
| R2-m2 the drift filter's pin must not be vacuous | TAKEN: §2.6 asserts `warnings == []` on the 0.5.0 fixture pair |
| R2-m3 the defence in depth's precision | TAKEN: §2.1 (`Result` at the callers, a lexical check, the probe as template) |
| R2-m4 the bridge sentence's reason | TAKEN: §5 |
| R2-m5 `why`'s thirteenth refusal shape | TAKEN: §2.4 and §3 (a corpus sample, `contract.ts`, outside `kind_of`'s fence) |
| R2-m6 the contract file's precedent | TAKEN: §2.4 |
| R2-m7 the constructor | TAKEN: §2.4 (`with_parts` on all three hosts) |
| R2-m8 the shell's `isAddress` sentences | TAKEN: §2.1's ripple |
| R2-o1 an undone owed upload meeting a new bot | TAKEN: §2.4 states it bounded (the alarm retries until Abandon or expiry) |

Revision 2.2's Worker doors were not in this re-read's scope; their own re-read follows (§10.4).

### 10.4 The Fable re-read of revision 2.2 (2026-09-26, `fd10339c`, standing at `a23540f0`)

The re-read is appended to the same report.
- **Verdict:** PLANABLE WITH THREE CORRECTIONS.
- **Counts:** 0 CRITICAL · 0 HIGH · 3 MEDIUM · 6 MINOR · 0 OPEN.
- **Confirmed:** 14 mechanisms. Among them: a browser puts no header on an upgrade; the Table's stream admission
  hands the module an empty query, so the `claimed=` carrier trips nothing; the Origin check and `route()`
  ignore the query; and the operator surface has no other channel.
- **Against revision 2.3:** it touches none of 2.2's sections, its one contact harmonizes, and its dispositions
  of the revision-2 findings were read against the evidence already gathered, with no new false sentence found.

The owner asked that this be folded as **one revision once the review was complete**, not in rounds. Every finding
is **taken** in revision 2.4.

| finding | disposition in revision 2.4 |
|---|---|
| **R2.2-M1** — the deploy guard cannot see a secret; `operatorGate` would answer 403 under `open` | TAKEN: §2.1 splits the instruments (`operatorGate` answers `off` under `open`; `shellConfig` refuses the operator list or the dev door beside `open` at run time; the guard keeps the `vars` checks); R-6.5c2.5 and §3's cells |
| **R2.2-M2** — one pool project cannot exercise both doors | TAKEN: two pool projects (`wrangler.open.test.jsonc`); a per-request door header considered and not taken; the unset-door cell at `handle` |
| **R2.2-M3** — the NAS's `cf-access` has never been read live | TAKEN: §2.10 and §3's close read `/api/me` over WARP as the owner before the NAS deployment is done |
| R2.2-m1 the module builds the layer without `open-door` | TAKEN: the module enables the feature (§2.1, §5) |
| R2.2-m2 two names move at `init` | TAKEN: §2.1 (the mode, and `Host.identity_name`, which stays `cf-access` under `worker`) |
| R2.2-m3 the shell must hold a claim to what a header can carry | TAKEN: §2.1, through `address()` |
| R2.2-m4 a plain name ending `.test` would take the `test/` root | TAKEN: the test root requires an `@` (§2.1) and a cell |
| R2.2-m5 the guard's precision (`preview_urls`; an absent door) | TAKEN: §2.1's guard rules |
| R2.2-m6 the open claim's own `Identity` kind | TAKEN: `claimed` in `worker/src/identity.ts` (§2.1) |

**The spec review is complete:**
- **Revision 1:** 0 CRITICAL · 4 HIGH · 6 MEDIUM · 12 MINOR · 6 OPEN.
- **Revision 2:** 2 MEDIUM · 8 MINOR · 1 OPEN.
- **Revision 2.2:** 3 MEDIUM · 6 MINOR.

Every finding is taken, and 90 claims and mechanisms were checked true across the three reads.

### 10.5 Corrections found while planning (revision 2.5, 2026-09-26)

The plan (`docs/plans/2026-09-26-web-frontend-m6.5c2-plan.md`) was drafted against revision 2.4. Its drafters
executed most tasks in `git archive` exports and read every `file:line` at HEAD, and found the sentences below
wrong or incomplete. Each is corrected in place; the plan's Appendix A carries each drafter's note verbatim, and
its "Assembly notes" carry the reconciliation. **No ruling changes.**

| § | revision 2.4 said | what was found | the evidence (plan Appendix A) |
|---|---|---|---|
| 2.1 | the containment check is "no `Component::ParentDir` in the joined path" | an absolute path has no `ParentDir` and `join` with one replaces the base; a configured directory that is relative with `..` would refuse every put. The check is every component of the RELATIVE path `Component::Normal`, before the join | D1 amendment 3; mutant M2 (the old wording) fails the cell |
| 2.1 | the NAS "is redeployed with `open` at the close" | it contradicted §2.10's `cf-access` (revision 2.2); §2.10 stands, and the NAS is not redeployed from the branch before the close | D1's notes |
| 2.1 | two `cloudflareTest` projects "in one vitest workspace" | Vitest 4 removed `test.workspace`; the two projects are `test.projects` in `worker/vitest.config.ts`, the open project's cells under `worker/test/open/` | D6 amendment 1; measured, two workerds, `--project open` |
| 2.1, 2.10 | the dev door's id becomes a secret by `wrangler secret put` | refused while the deployed version holds the name as a var (workers-sdk#5756); one `wrangler deploy --secrets-file` drops the var and adds the secret in one version | D8 A-1; the pinned wrangler 4.136.3's own option text |
| 2.4 | `TurnMirror`'s `Copy` ripple is four by-value reads | six: `games.rs`'s mirror refresh and one test also move the value (E0382) | D2 A3; measured |
| 2.4 | the foreign-bot check runs "before `Session::from_record`" | "not over" needs the replayed state; the refusal returns between `Session::from_record` and `Repl::new`, which keeps the property (no `Repl`, so no conversion) without a second replay | D2 A4; its first cell measures the property |
| 2.5 | the Worker's `conflict` arm routes through `stamp_archive` | read by today's `stampArm`, another game's conflict is `retry`, hourly for ever; `settledArm` keeps it final | D6; a mutant turns both conflict rows red |
| 2.5 | the race cell holds "a patched `R2Bucket.prototype.put`" | the pool's measured hold is `swapBucket` (`worker/test/gates.ts`); the race is two cells, overlapping and not, and the map holds both uploads (`uploads.size === 2`) | D6 amendment 3; measured at the M6.5c code |
| 2.6 | the drift cell rebuilds the 0.5.0 pair "at 0.6.0" | vacuous until the bump; the cell derives a differing version, so it bites before and after | D3's Task 7; D8 A-7 |
| 2.6 | the restatements move for "the module" | the browser host restates the clock and the scan too; it switches as well, or the layer's "stated once" is false | D7 A-6; ruled at assembly |
| 2.7 | F-4's cost is `readDocs` running three times | `hasTables`' `sqlite_master` scan is ~94% of a warm request's rows; the memo takes a warm request 32 → 2 rows; read-once saves 0–2 | D7 A-3; measured in the pool; both fixes ruled at assembly |
| 2.8 | `dev-vars.mjs` "with a type declaration" | TypeScript reads no `.d.ts` for an `.mjs` import; it is `dev-vars.d.mts` | D7 A-1; measured TS7016 |
| 2.9 | the Docker filter | misses `crates/yokohama-bridge/**` (the CLI depends on it); `deploy/**` made every document edit pay a cold build, narrowed to `deploy/Dockerfile*` and `deploy/smoke-image.sh` | D8 A-2, A-3 |
| 2.9 | CI's layer steps | `browser_played_pin` has run in no workflow; the refusing-build step prints nothing on failure under `bash -e` | D8 A-4 (measured), A-5 |
| 2.10 | the NAS gets "this cycle's image", with a `--build-arg` | the NAS is deployed from `deploy/Dockerfile.runtime` with a workstation-built binary and `web/dist` (kernel 3.10); the link comes from the one `npm run build`; E-3 is proven by the Docker workflow | D8 A-11 |
| 2.11 | four screenshots | seven | D8 A-12 |
| 2.11 | only RUNBOOK §6 moves | §0, §2, §3 and §3.2 and `cloudflared-config.yml` describe `claimed`, and the close follows that recipe | D8 A-13 |
| 2.11 | "an M6.5c2 row" | two rows: the personal-data exception when the documents are written, the cycle row at the close with its readings | D8 A-9 |
| 3, 4 | the close's order put the review and the battery after the live tiers | a fix found then forces a second deploy and live sequence; the close runs in M6.5c's order | D8 A-10; ruled at assembly |
| 8.2 | the bump's ripple | adds `backup-envelope.json` (a LIB cell writes it), `owed-pump.json` (kept), and the third-party `0.5.0` in `worker/package-lock.json` (never touched) | D8 A-6 |

**The spec's silences, answered at assembly** (D4's and D5's lists; the orchestrator confirmed each, and each is
built as stated):
- the door's "How to play" opens the help without its screenshots, which show card art;
- the creator's lobby rows keep Abandon beside their one action (R-13); row actions are buttons, and the table's
  name is the one link;
- state C's discriminator tests state A first; state D is the door's alone, and the lobby shows "loading…" rows;
- the iPhone keeps its install instruction, because WebKit has no programmatic install;
- the joiner's password sits above the seats and the one join button below them;
- the open table's chat stays in the main column;
- other people's seat cards read "seated";
- test data never uses the owner's surname.

### 10.6 The single plan retired, and the three cycles (revision 2.6, 2026-09-27)

**What happened.** Revision 2.5 was planned as ONE implementation plan, `docs/plans/2026-09-26-web-frontend-m6.5c2-plan.md`
at `7422083e`: 28 tasks in 20 execution units, drafted in parallel by eight Opus drafters against one base
(`1e16fe27`) and assembled by the orchestrator — **27,315 lines**, against a largest executed plan in this project of
3,288 lines (M5) and an M6.5c plan of 2,778. One Opus adversarial reviewer then applied Tasks 1–7 in one tree and ran
the pool at each step — the first time the assembled tasks had met in one tree — and found **1 CRITICAL, 6 HIGH,
2 MEDIUM, 2 MINOR and 1 OPEN**, 24 claims checked true, most findings at the seams between drafters, before the owner
stopped the review. The owner judged the plan unmanageable and the approach flawed — one orchestrator session could
not hold, review or execute it, and every task past the first was written against a tree the earlier tasks would
change — and retired that session (2026-09-27).

**The rulings (the owner, 2026-09-27):**
1. **Three cycles**, A the layer, B the screens, C the hosts and the close, each with its own plan written at its
   start (§4).
2. **Drop the pure refactors** (R-6.5c2.1 as amended).
3. **One stacked branch and PR per cycle**, `web-room-m6.5c2a`, `-b`, `-c` (R-6.5c2.17 as amended), *"the cleanest
   handoff and history tracking"* when each cycle runs under its own orchestrator.
4. **The push and PR at a cycle's close are pre-approved at the orchestrator's discretion**, and each close writes
   the handover the next cycle's planning starts from (the owner, later the same day, on reading cycle A's plan).

**What is carried, and where.** The spec's mechanism is untouched — revision 2.6 changes §4 and §9, amends three
rulings and marks the dropped items. The eight drafts, the assembly's skeleton (its shared names, couplings and
assembly notes) and the review report stay on disk under the gitignored
`.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/` as the raw material each cycle's plan is built from; the
review's verification chain (per-task commits through Task 7, with its P-3 stand-in) and D1's trial are bundled into
cycle A's ledger, `.superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/`. **The visual work is kept for cycle B**
(the owner, 2026-09-27, asked): the eleven approved mockups are COMMITTED (`docs/plans/2026-09-26-web-frontend-m6.5c2-mocks/`);
their HTML sources and the two font files live in `.superpowers/brainstorm/20585-1790461301/content/` and are copied
beside the drafts as `mocks-html/`; the brainstorm's nineteen review shots (`check-*.png`, the iterations the owner
approved from) stay in `.superpowers/brainstorm/`; D4's eighteen RENDERED screens — the door, the lobby, the name step
and states A–D at 1440 and 390, read beside mocks 01, 02, 03, 05 and 11 — were rescued from the retired session's
`/tmp` scratchpad into `d4-look/`; and the drafters' measurement and working files (D7's rows-read logs behind the
`hasTables` memo, D4's component files, D5's seed crate, D6's and D2's exports' notes) into `drafter-artifacts/`.
Cycle A's close names these in the handover into B (its plan, Task 9 Step 5). **The retired plan is removed from the
tree in this revision's commit** — a plan that never executed is not a frozen record, and a 27k-line document under
`docs/plans/` would be read as one — and stays in history at `7422083e`. The review's findings land in the cycle that
owns the code:

| finding | severity | lands in |
|---|---|---|
| P-1 — RUNBOOK §2's NAS recipe served the repository root, where `.dev.vars` sits, to the LAN | CRITICAL | **fixed in this revision's commit series**, before any task: the context is built in and served from a temporary directory alone, and a `curl` for `.dev.vars` must read 404 before the build |
| P-2 — the assembly's surname substitution was case-sensitive and partial; the owner's surname in committed test data | HIGH | cycle A's plan: every test name is a neutral one throughout; the real hostnames stay |
| P-3 — the exported `error_value` dropped the `foreign_bot` arm | HIGH | cycle A, the restatements' export task |
| P-4 — `owed-game-over.table.json`'s mirror needs `bot`, or two pool cells go red | HIGH | cycle A, the label task (O-1 (a), measured green) |
| P-5 — the pool's corpus cell compares a `web/server` save with a `web/worker` one | HIGH | cycle A, the `run_id` task |
| P-6 — the pool's `Reply` fakes carry no `logs` | HIGH | cycle A, the log-seam task |
| P-7 — `the_table.spec.ts` drives the old table form | HIGH | cycle B, the forms task |
| P-8 — a `THIRD_COLUMN` literal pasted over an inserted name | MEDIUM | cycle A, inline in every task that adds a corpus file: INSERT, never paste |
| P-9 — the secret's hand-off contradicted itself | MEDIUM | cycle C: ONE `wrangler deploy --secrets-file` at the close, nothing before |
| P-10 — base-relative counts presented as expectations | MINOR | every cycle plan states counts as deltas over the implementer's own HEAD |
| P-11 — a grep run before the workflow line it counts is rewritten | MINOR | cycle A, the rename task |
| O-1 — P-4's shape | OPEN | (a) taken |

**The lesson, for the next spec of this size** (also in the orchestrator's memory): a spec that covers several
subsystems is split into cycles at SCOPING, before any drafting is paid for; a plan is written to be read whole by
one agent; and plans are written by the orchestrator or one agent, never by parallel drafters — eight of them
cross-messaged and reworked in circles until the owner stopped it.


### 10.7 Cycle C's planning session with the owner (revision 2.7, 2026-09-28)

Taken against the tree cycle B left (`web-room-m6.5c2b` at `3ed27f07`; the C handover
`docs/plans/2026-09-28-web-frontend-m6.5c2c-handover.md` is the entry). Each ruling carries the owner's words
where the owner gave them, so it can be found by search. None changes a mechanism of §2.1, §2.5–§2.8 or §2.10.

| ruling | the owner's words | where it lands |
|---|---|---|
| **R-6.5c2.20 — one cycle C, then a lighter cycle D.** C carries the hosts' code, the host documents, the deploys and the live tiers through its close document; D is the final wrap-up and polish, then the merge, the tag and the refocus. D absorbs what grows past C's plan. | *"one cycle, this is a substantial catch-all type phase wrap, it's much better to have a lighter phase d and then we can engage in a real in-depth final straw poll/investigation on what's remaining and then wrap it up"*; *"the phase D content can be dependent if as we're drawing the scope for C that this grows too far it can get moved into D"* | §4 row D; R-6.5c2.17, R-6.5c2.19 and §3 as amended |
| **R-6.5c2.21 — the bot's version: ingestion, not the key.** No key change, no record change; the lane document's "What a save file says" is the ingestion script's contract; the versioned refusal sentence lands in C. | *"a simple python script could ingest the saves and sort them … if this adds a substantial amount of work, it's likely then not worth the savings … if we do it in ingestion instead of just labeling that can work also"* | §2.4 as amended |
| **R-6.5c2.22 — the lobby's "Finished" group**, client-only, folded, "View" on its rows; a time rule is D's if ever. | *"we would need the view to prune or collapse or otherwise no longer show finished games after some time so it doesn't add up, but I can see value if you have the password or entitlement to go back into a finished game"*; *"I agree with the finished/being played breakout — although the visual effect of this is important"* | §2.2 as amended |
| **R-6.5c2.23 — polish: the standard, and the split.** Pictures at 1440 and 390 beside the mock for every screen change in C and a headed look in its review; B's items 1–4 in C, 5–9 in D. | *"polish is key, these are the last revisions here, and we're dedicating substantial resources so we want to get it well enough to last for some time"*; *"the polish items may matter … they can move to D as a final wrap up/polish phase"* | §2.2 as amended |
| **R-6.5c2.24 — CI stays lean.** §2.9 read as a ceiling; the Docker workflow triggers on web-lane inputs only, never a core-only change; the first runs measured. | *"we should not overload CI with tests that we're already running heavily, it should be useful for CI specifically and not overburdened"* | §2.9 as amended |
| **R-6.5c2.25 — the lane document and the top-level refresh are D's**; C keeps the host documents, the address scrub and a truth pass on `CLAUDE.md` at its close. | *"you could probably move the lane document to D as well since we're likely to be editing those same docs in D which will be immediately after C, as we won't have full deploys so no use prior to then anyway"* | §2.11 as amended |
| **R-6.5c2.26 — the dropped refactors stay dropped**, in D as in C. | *"the refactors dropped don't have to come in even in D that's fine"* | R-6.5c2.1 as amended stands |
| **R-6.5c2.27 — the phones wait for the deploy**; D may use the Device Hub iOS simulator. | *"The phones are delayed for an actual deploy or there is nothing to test. We could even layer into D testing with the Device Hub iOS simulator but that's for the plan phase of D"* | §6 as amended |

**Cycle C's tasks, as approved for the plan** (the plan is `docs/plans/2026-09-28-web-frontend-m6.5c2c-plan.md`,
written by the orchestrator against the tree at `3ed27f07`): (1) the Worker's two doors; (2) the open door's
refusals; (3) the refinish rule on the Worker; (4) the module's invariants and the imports; (5) the small debts,
shell and layer; (6) the scripts' LOWs; (7) build and CI; (8) the 0.6.0 bump; (9) the lobby's Finished group;
(10) the client's carried fixes, polish 1–4 and the a11y pass; (11) the host documents; (12) the close through
the close document and the handover into D. What D holds is §4's row D.

### 10.8 Corrections recorded in execution — cycle C (2026-09-28)

Recorded FORWARD at cycle C's close from its ledger (`.superpowers/sdd/2026-09-28-web-frontend-m6.5c2c-plan/progress.md`,
gitignored; the close `docs/plans/2026-09-28-web-frontend-m6.5c2c-close.md` §5 carries every ruling with its cost). The
plan `docs/plans/2026-09-28-web-frontend-m6.5c2c-plan.md` is a dated artifact and is not edited; each row is the
sentence as the plan or a review wrote it, what execution found, and the ruling that settled it. Nothing here changes a
mechanism of §2; every row is a sentence made true.

| where | the sentence as written | what execution found | the ruling |
|---|---|---|---|
| Task 1 Step 3 | `YOKOHAMA_WEB_IDENTITY`'s value "` Open ` folded as every switch is" | the layer's own parser for the SAME variable (`identity.rs` `from_str`, `match s.trim()`) refuses `Open` and `WORKER` on the NAS host; one variable must read one rule on both hosts | **R-C11**: trimmed and matched EXACTLY; `Open`/`WORKER` refused naming the value; stated in `door()`'s doc, no code change |
| Task 1 Step 4 | "Green: `npm run typecheck`" at Step 4 | unreachable there — the widened `Identity` union leaves `index.ts` red until Step 6's edit | green at Step 6; accepted |
| Task 1 Step 5 | "the two `worker.test.ts` cells red" | both GREEN at Step 5, as the draft predicted (the unset door's 500 is Step 4's; under `worker` the parameter is never read) | guard PINS with recorded mutant witnesses M1 and M7′ (the rubric's "passes at HEAD with a proven mutant") |
| Task 1 (the module's `open-door` feature) | the feature as the open door's mechanism | INERT at compile time — the module maps `Door::Open` itself and never calls the feature-gated `from_str` | **R-C12**: it STAYS as §5's declaration; the close never cites it as the mechanism |
| Task 2 Step 8 | the guard on the tree "fails checks 1, 2, 8, 9" | failed ONLY 1 and 9 on this workstation (a leftover assembled `worker/assets/` passes 2 against its own `sw.js`; the fresh debug stamp passes 8) | the clean-machine expectation; a note, no ruling |
| Task 3 Steps 5–7 | "red on the foreign cell (a throw inside the alarm)"; "a creator's DELETE 200"; "each ends with … abandonable … and expiring"; the conflict `it.each` as one row | `settleUpload` has guarded `parseStamp` behind `status === 200` since `ad476e8f` — the red was the MISSING `stamp-refused` line; the tree answers **204**; one table cannot both abandon and expire (the overlapping cell abandons, the sequential cell expires); the `it.each` is the draft's two rows; the log lists carry the layer's lines since cycle A | all five accepted as the tree's readings, no code consequence |
| Task 3 (the `stamp-refused` line) | `{id, status}` drops the body the old line carried | the shape is the contract's; the reason is on the wake's `restore-refused` line | **R-C13**: the shape STAYS; Task 12's log vocabulary says where the reason is |
| Task 4 Step 2 | `expire_inner`'s no-creator arm "cannot reach a rebuild" | it CAN: an unknown key holding `1e400` (or nesting past 128 levels, or a lone-surrogate escape) is read by the layer's `TableDoc` parse and refused by the module's `serde_json::Value` read, so the arm's 500 names the wrong reason there | **R-C14** (amended): accepted, no code change — no layer write makes such a document; the third Step 2 cell is built on exactly it; the docs state the CLASS (a deferred minor) |
| Task 6 Files | the 409's construction "in `api.rs` (`live_handle`'s refusal — find `cannot_continue(` by content)" | the tree builds `GameError::ForeignBot` in `Games::live_handle` in **`games.rs`**, non-test code | **R-C15**: the named place is read where the tree holds it; the five-file stat holds |
| Task 6 (`api.rs`'s own sentence) | `undo_allowed`'s refusal left on `label` | it would read "played against V4, which this server doesn't run" on a server running V4 — the defect class R-C5 closes | **R-C16**: routed through `distinguishing_labels(played, &production()).0`; no corpus payload moves |
| Task 6 "+2" | two cells | the three callers' routing was UNPINNED (mutant D stays green: every fixture pairs differing labels) | **R-C17**: ONE same-name re-point cell added (`games.rs:1273`); `({controller})` rendered when the parameter map is empty (a refinement of R-C5's third arm) |
| Task 6 (finished re-point) | — | a FINISHED same-name re-point shows the CLI's drift warning in `ClientState.warnings` | **R-C18**: left this cycle (§7 keeps the CLI-worded warnings apart from §2.6's filter); a D item |
| Task 7 Interfaces | `type Opening = { id: Promise<string \| null> }` | `abandonAsCreator(page, id)` must run in the creator's own context | **R-C19**: `{ page: Page; id: … }` — one field; the draft's `sent()` stays dropped (the context's 15 s `actionTimeout` bounds the armed `waitForResponse`) |
| Task 7 (L-5's third statement) | L-5 scoped to the pool's exemption | the hole SURVIVES in the browser host's replay (`corpus.node.test.ts:231-235`) | **R-C20**: left and recorded forward as D's item (a third statement is R-6.5c2.1's dropped-refactor territory) |
| Task 8 (six departures) | the draft's `ls`, an unbounded smoke step, `rm`, four comments, a `deploy/README.md` reference | the test-binary guard asks `cargo metadata` (`ls` misses `tests/<dir>/main.rs`); the gate runs `--nocapture` (a killed run otherwise logs NO seed); the no-door smoke step is BOUNDED (a foreground `docker run` hangs on a door-defaulting image); `rm -v` (a volume leaked per container); four stale comments corrected; no reference to a file Task 12 creates | accepted as measured |
| Task 8 / `docker.yml`'s header | "every run pays a cold `cargo build`" | FALSE: the build cache is exported in max mode; a `web/**`-only run comes back WARM | **R-C21** (amended twice): the cold set is a Dockerfile server-stage edit, the floating base tag moving, a ref that cannot read the cache, a dockerignore edit, and a `crates/yokohama-web/**` change; R-C9's wall is read only from a run whose log shows `[server 10/10]` executing |
| the plan's I.1 / H.1 (line 1762) and R-C21 | "the first wall is read at the push" | `docker.yml`'s `push` trigger is `branches: [master]`, so the branch's push fires `test.yml` alone | **R-C21 amended (M-1/E-1)**: the cold wall is read from **PR #41's `pull_request` run** (cold by construction; `cancel-in-progress` means no second push during it) |
| Task 9 Step 9 | "EXACTLY the cells cycle A names" go red with `without_version_drift` reverted | FOUR cells go red at 0.6.0, not two — the extra two show the real `engine_version="0.5.0"` vs `"0.6.0"` line and went LIVE at the bump | **R-C22**: accepted, no change; four catchers is a stronger guard (battery row 42) |
| Task 9 Files | `tests/browser_played_pin.rs` absent from the Files list while Step 2 mandated its doc edit | the doc edit is in the fence by the mandate | accepted; the fences name it |
| Task 9 (a draft comment) | `plays_locally`'s "the version had moved for three cycles" | false | reworded |
| Task 10 (four accepted facts) | one corpus field per cell; the witness's steps; "every game already over"; the marker leading the count | no corpus table is over with a winner, so two cells move `over` + `winner` (and `id`, `name`, `created_at`); the witness presses "switch player" before `signIn` (idempotent); "other people's finished games" (your own stay under Your tables); the marker FOLLOWS the count (leading, it pushed the head 24 px right) | accepted from the tree and the pictures |
| Task 11 item 1 | the heading "Lobby"/"On this device" at LEVEL 2 | `LobbyBand(["Lobby", name])` makes the TABLE'S NAME the `h2` by B's convention; "Lobby" is a crumb | accepted |
| Task 11 (six places beyond the brief's letter) | — | `h4`→`h3` on "Will not open here"; `isStranded` also filters the SERVER menu listing; focus returns after save and cancel; 44 px on the open table's Copy link and Abandon and the device card; one `joins_by_link` locator scoped to the menu; `BackupControls.tsx`/`GameMenu.test.tsx` changed | **R-C23**: each accepted as the ITEM'S RULE applied consistently |
| Task 11 item 7 | "Abandon's refusal reaches the same strip, one screen-height nearer its control" | FALSE at `105a196b`: the strip already sat beside Abandon; moved up it sat ~350 px above | **R-C24**: Abandon's `onError` gets its OWN strip after the controls, with a DOM-order cell |
| Task 11 item 2 | the `failed` class keyed on "text ≠ the one-line Ready…" | "Preparing…" is a one-line transient that needs no spanning | **R-C25**: keyed on FAILURE |
| Task 12 Step 7 | "`git grep -n "_at_gmail"` prints nothing" | cannot hold while the plan quotes the command (its own line 1536) | **R-C26**: read as "prints nothing outside the plan's own quote", which holds |
| Task 12 §11 | "the documents are still read up to three times on a cold path" | FALSE since Task 5: a cold request reads them TWICE | the bullet says twice |
| the whole-branch review's M-2 | "`worker` behind a proxy that verifies identity is the door for that case" | FALSE: `worker` trusts `X-Yokohama-User` unverified (`identity.rs:227-256`); the axum image has no verifying shell | **R-C28**: the true limit stated in `deploy/README.md:151-155` — no door this server has closes the host-process path |
| §3's close order, steps 2–3 | the battery, then the ONE recorded sequence | the sequence's walls are RECORDED figures and need the quiet machine; a battery finding is a test defect and forces no redeploy | **R-C29**: the sequence first, the battery after it in an export, concurrent with the deploys |
| §3 step 6 / the live tier | one run at a time; a second 409 a page is breakage | the first live run's `two_browsers` read ONE `/events` ask by the watcher (the socket arm's probe after a close the client did not make) at 13.9 s, with `wrangler tail` disconnected at that moment | **R-C30** (amended): re-run ONCE, ALONE, with a fresh tail attached; both outcomes are the record (the close §3, §7) |
| the cycle's e2e row | `npm run e2e` green in the sequence | 67 of 68 in the sequence (`plays_locally` "a quota failure is APPLIED BUT NOT SAVED": the option clicked opened a two-step choice's second step, so no move applied and no warning could appear — a seed-conditional premise, the SAME cell M6.5c's sequence recorded red under "load"); 68 of 68 alone | recorded, not retried into green; the cell's hardening is D's |

### 10.9 Cycle D's planning session with the owner (revision 2.8, 2026-09-28)

Taken against the tree cycle C left (`web-room-m6.5c2c` at `014433df`, the reviewed code head `ea4f174d`; the D
handover `docs/plans/2026-09-28-web-frontend-m6.5c2d-handover.md` is the entry). The session began from §4's row D
as revision 2.7 wrote it and ended with **row D REDEFINED and a row E ADDED**: the owner read the straw poll of what the
lane leaves (152 de-duplicated items, compiled read-only from every close since M5.5 by a researcher and published as a
private triage page, `https://claude.ai/artifact/Sg69JsMaQyXAEwoL4tLGrm`, the owner's decision per row saved on the
page and read back into the plans) and set a higher bar than "the wrap-up and polish". Each ruling carries the owner's
words where the owner gave them, so it can be found by search. None changes a mechanism of §2.1, §2.5–§2.8 or §2.10;
R-6.5c2.36 reverses one earlier ruling and says so.

| ruling | the owner's words | where it lands |
|---|---|---|
| **R-6.5c2.28 — the bar: no known debt is left behind.** Every straw-poll row gets a FINAL disposition — fix (D or E by area), the public door's (E), by design (stated once in the lane document), not observable (stated with the condition that would make it so), the AI lane's, or the owner's — and "record and leave" is not one of them. Play, polish and efficiency are user-facing and are in; a large refactor for a small gain on code the project is walking away from is out. | *"I'm not sure why we would leave so many of these items unaddressed, it's not backlog we'll want to get back to, we want to really get the yokohama-web into a long(er) term stable state after this week, so we can not think about revisiting these things for a while"*; *"the goal is to properly wrap up this up, not to sacrifice keeping known debt especially if it impacts the game/playability/user experience as we'll eventually be moving to a public repo and this is user facing, so polish and efficency is important (not large scale refactors for small gain on somethign we're going to walk away from, but getting to a highly stable, proven base so we can get back to our key focus)"* | the triage page; the straw-poll record (D's Task 2); the lane document's "what remains" (E) |
| **R-6.5c2.29 — two phases, D and E, grouped by AREA so each part of the code is opened, reviewed and deployed once; a third phase only if D's budget pushes too much into E.** D is the code: the layer and the Worker, then the client's defects, the local host and the test debts. E is the finish: polish, the client's remaining items, the phones, the public door, the documents, the merge and the tag. Each phase has its own plan, ledger, review, recorded sequence, deploys and close, and its PR on the stack. | *"I think that we should plan on the two phases, but we maintain the flexibility if we end up using the budget you note and we push too many items from D to E, then that may necessitate a third phase"*; earlier, *"we don't have to think about the barrier between D and E as strictly engineering vs non engineering … some items grouped apporpriately so we're not redoing the same work roughly can move into the E phase to balance it out a bit"* | §4 rows D and E as amended below |
| **R-6.5c2.30 — the dropped refactors are REOPENED under a budget** (this reverses R-6.5c2.26 and, for these four items, R-6.5c2.1's "DROPPED"): ONE task in D, one day of wall, the items in this order — the `table.ts` split, the four script dedups, the live-replay script's split — under the pool's 2,209-row replay and the corpus as the fence; read-once threading stays dropped (measured to save 0–2 rows, §10.5 row 2.7); whatever the budget leaves is recorded as by design. The R2 ladder (an outage costs each request one attempt and one failure batch while a save is owed) is DECIDED once in D — a short backoff after a failure, or its cost stated. | *"things like the refactor or r2 outages costing retries are either nonissues or known quantiies but real work we should consider as needing to be done to consider 6.5 wrapped"* | D's Tasks 8 and 9 |
| **R-6.5c2.31 — CI after D: the core job alone on pull requests, path-filtered to the Rust inputs, plus dispatch; the three web jobs in a workflow of their own and the Docker image on `workflow_dispatch` ONLY; cancel-in-progress on every workflow; no Rust build cache.** The merge of the stack runs with `test` and `docker` DISABLED and nothing dispatched — the local per-edit suites and the fifteen gates at the reviewed head are the proof — and both are re-enabled after the tag; `release.yml` stays enabled because the tag is its trigger. **Amended the same day (2026-09-28, after the D plan):** the two workflows are disabled on the GitHub side FROM NOW, through D and E, until after the merge — the owner: *"I'm at 90% anyway for the month with the aggressive use as of late so that decision from earlier of holding the github CI until after the stack merge should hold - by then the plan will have reset anyway as we'll move into October"* — so no run of any kind, dispatch included, happens in D; `web.yml`, new in D, is dispatch-only and is dispatched by nobody until E's merge step re-enables the set. The one coupling, stated so it is not feared: the lint gate compiles the layer with the workspace and the committed corpus pins the engine's wire-visible words, so a core change that reaches the layer is caught locally by the per-edit rule. | *"what would the point be of having web even on merge into master - if we're running those full suites locally? The key to ensure consistency are the core crates. We would push to master generally only on a PR merge to master from another branch, but even then when I looked at the CI being run much of it was duplicative"*; on the merge: *"Disable and dispatch nothing"*; the cache was explained and not taken up | D's Task 1; E's merge task |
| **R-6.5c2.32 — the documents: a MAJOR guidingpath archive by grouping, and a live file that starts nearly fresh.** The live `guidingpath.md` keeps the mission (with one dated line on why the web lane exists, in the owner's words below), a one-screen current state, the roadmap as a table (one row per phase and sub-phase: status, date, gate, record of record), and a decision log that opens with an INDEX (date range → file → what it holds) and resumes at D's close. The archive is broken out by grouping so a reader chooses by file name: `guidingpath-archive.md` untouched (the engine era); a new `guidingpath-archive-phase3.md` (the computer player's road to V4: the Phase 3 landed notes and the rows of 2026-08-09 → 2026-08-30, opening with the origins — the Dec-2025 stall, the July-2026 evidence base, the Phase 0–2.9 notes); a new `guidingpath-archive-web.md` (the rows of 2026-08-31 → the close). Every move is VERBATIM and byte-checked by script, on the 2026-08-19 and 2026-09-23 method, with a third relocation record. `CLAUDE.md`'s read-order line names each grouping and what it is for, its five web passages become pointers into `docs/web-lane.md` (§2.11), and the board's history becomes dated pointers. `README.md` in E is made true at 0.6.0 with a short "see it" of the new screenshots and links to the deep documents; the audience restructure is Track 2's (the public repository), recorded there. | *"would it make sense to be archiving it more in depth, so it's starting nearly fresh, or having it just contain pointers and we break out the archives by groupings so future agents can choose what to read and dig into?"* (both); *"the 3.5c arc is a key part of the last section of the computer player, but we also don't need the file to be too large … the work prior to 3.5c on the computer player has been nearly fully superceded … Things like the dec 2025 stall and the july 2026 evidence was superceded by the 3.5c v4 near completion - we pivoted to the web lane to make continued vetting and save game gathering easier and testing of future lanes easier than the CLI while still allowing automated play to be straightforward in continued development"*; on the README: *"it's gotten quite large so we may want to build out some links to sections around the CLI or web version … We could defer some of that work as a later task would be to prepare this project to become a public repo"* | E's document tasks |
| **R-6.5c2.33 — the phones are the SIMULATORS, driven by subagents through the mobile bridge, over WARP.** This Mac's WARP passes the owner's identity to both hosts, so the iPhone and iPad simulators (and the Android emulator, booted headed with the host GPU, for one short look at the Blink delta) reach the lobby signed in, with no login step — read on 2026-09-28 (`/api/me` names the owner from the Mac; the iPhone simulator's lobby corner shows it). The pass answers the owner's three questions — socket hibernation and reconnect against the Worker after backgrounding and a locked screen, persistent storage on a phone (the keep-storage prompt, a reload, Add to Home Screen and the home-screen app's own storage partition), and anything that blocks — plus one screenshot per screen for layout deltas; one agent per simulator, the two iOS ones in parallel and Android after; the brief carries the device id, the bridge's tool names and coordinate rule, the checklist, the capture rules and the report path. The owner's handset is a ten-minute optional confirmation at E's close while the wake window is open, with nothing built for it. | *"an earlier concern with mobile was more centered around websocket hibernation with the cloudflare implementation if that will work the same way as we have been testing it on local machines, same with the persistant storage or if there are key blocking concerns there"*; *"it likely makes sense to have a subagent running for each simulator so they don't collide - if the instructions including MCP handling details, in the brief are sufficent, I would guess sonnet would be sufficent"*; *"does it make sense to just use the simulators from here and allow that to drive through instead?"* (yes) | E's phones task |
| **R-6.5c2.34 — the public door is E's, VETTED then REVERTED.** E deploys the path-scoped Access application (M6.5b2 spec §3.2, built and never deployed), sets the access controls and the rate limits on the `join` and `unlock` oracles, turns the dev door off, tests as a stranger with a second identity, and then restores the full Access protection until the repository goes public. The precondition list is the FULL one — spec §2.10's six plus the umbrella's three Access-door items (R-6.5.3): the seat-email roster, the password oracles, chat moderation — and the lane document records it whole (the straw poll found C's close and the D handover carrying four). | *"We can also in the E phase actually vet out the public access on cloudflare, setup the access controls, manage the testing properly, and then revert to the full access protection until we're ready to go public with the repo"* | E's public-door tasks; the roster is D's Task 6 |
| **R-6.5c2.35 — the seat-email roster leaves the wire (D).** A human seat crosses as `{kind, name, you}` and the address only for the caller's own seats; `created_by` becomes a flag and a display name; the client's readers of `seat.user` key on `my_seats` and `you`; the corpus regenerates and its diff is read; a leakage pin asserts no payload built for one caller carries another seat's address. This supersedes M6.1's "seated players see each other's emails" as a by-design fact — it was by design behind a private allow-list and is a leak behind a public door. | (the owner asked for the elaboration and took the fix into D) | D's Task 6 |
| **R-6.5c2.36 — three model tiers, from the day Sonnet 5.5 shipped.** Sonnet 5.5 for implementation-focused tasks whose brief carries the judgment already made; Opus where judgment is needed (a refactor, a design choice, a wire-scoping change, every reviewer); Fable for orchestration, the whole-branch review and any read of a large context. Each plan marks every task's tier. The standing rules of R-6.5c2.18 otherwise hold: SDD for every task, reports to files, a fix is a new commit, nothing runs in an implementer's tree, the address in no document. | *"Sonnet 5.5 was released today, so we can leverage that sonnet model in tasks that are more implentation focused where we had been only using opus agents throughout 6.5 - opus is still a better choice where judgment is needed and fable is necessary for orchestrating large plans and reviewing large contexts"* | every D and E task header |

**§4's rows D and E, as amended (revision 2.8).** Row D's revision-2.7 content is REPLACED; the stack gains one PR
(#43, cycle E's), and the merge is E's.

| cycle | branch (its base) | plan | content | ends with |
|---|---|---|---|---|
| **D — the code** | `web-room-m6.5c2d` (on `-c`) | `2026-09-28-web-frontend-m6.5c2d-plan.md` | CI (R-6.5c2.31); the straw-poll record; the small debts (the module figures, the twelve `RUNBOOK §6.x` citations, C's comment minors, the four false `CLAUDE.md` sentences); the layer's defects and the NAS minors; the seat-email roster (R-6.5c2.35); chat delete and moderation, the archive route's size cap; the R2 backoff and the dark held-request arm; the Worker refactors under budget (R-6.5c2.30); the layer's test debts and a 500-seed livelock run; the client's defects by screen; the local host and backup; the client's test debts (`plays_locally`'s premise among them) | the sequence, the Fable whole-branch review and one fix wave, both hosts deployed, the live tiers with the Observability query API in place of the tail and three readings only a deploy affords (`picker-dropped`, a game on the NAS under `cf-access`, one large replay timed); the close document and the handover into E; the push and PR #42 (pre-authorized) |
| **E — the finish** | `web-room-m6.5c2e` (on `-d`) | written at E's start, from D's handover | polish 5–9 and the owner's-eye calls off a picture sheet; lobby paging, "Abandon and save" made true, the arrival round trip; requests per move, bounded to the client with a measured target; the phones (R-6.5c2.33); the public door vetted and reverted (R-6.5c2.34); the documents (R-6.5c2.32): the lane document, `CLAUDE.md`, the guidingpath archives, the board, the README, the seven screenshots off E's deployment; whatever D's budget pushed here | the same close, plus the e2e and a plain game's rows-read figure; then, on the owner's word, the merge #31 → #43 with merge commits under R-6.5c2.31's disabled workflows, the `v0.6.0` tag, `release.yml` read to green, the workflows re-enabled, the memory rewritten, and the refocus on the computer player |

**The triage, by count (the orchestrator's calls on the page, 2026-09-28, before the owner's changes):** fix in D 62 (35 in
the layer and the Worker, 27 in the client, the local host and the client's tests) · fix in E 20 · the public door 6 · the
documents 6 · by design 32 · not observable 7 · the AI lane's 11 · the owner's 8.
The page's decisions as they stand when each plan is written are that plan's input; a row the owner moves after a plan
is cut is the next phase's.

**Cycle D's tasks, as approved for the plan** (fourteen; the tier is the implementer's, every reviewer Opus):
(1) the CI reshape — Sonnet; (2) the straw-poll record as a dated document with the page's dispositions — Sonnet;
(3) the small debts — Sonnet; (4) the layer's defects — Opus; (5) the NAS server's defects and minors — Opus; (6) the
seat-email roster, layer and client — Opus; (7) chat delete and moderation, and the archive route's size cap — Opus;
(8) the R2 backoff and the held-request arm — Opus; (9) the Worker refactors under budget — Opus; (10) the layer's test
debts and the livelock run — Sonnet; (11) the client's defects by screen — Opus; (12) the local host and backup — Opus;
(13) the client's test debts — Sonnet; (14) the close. What E holds is its row above.

### 10.10 Corrections recorded in execution — cycle D (2026-09-29)

Found while executing `docs/plans/2026-09-28-web-frontend-m6.5c2d-plan.md` (fifteen tasks, the reviewed
head `4212f6a0`; the close is `docs/plans/2026-09-29-web-frontend-m6.5c2d-close.md`, whose §5 carries
every ruling with its cost). None changes a mechanism of §2; each is a sentence the plan or an earlier
section wrote that execution found false, or a shape the plan sketched that the tree already had.

| where | the correction |
|---|---|
| the plan's Names table (`undo_allowed`) | `Result<bool, UndoRefusal>` → `Result<(), UndoRefusal>`: nothing reads the bool. |
| the plan's two sentences for the poison refusal | the Names table and Task 5's Interfaces disagreed; the task's own sentence was used ("the game's handle was poisoned; the next request rebuilds it"). |
| the plan's `DirArchive` defect (Task 5) | `write_over` already wrote `.tmp` then renamed; the real defect was the read-then-write RACE — closed by publishing the record with `hard_link` (atomic create-if-absent); needs a filesystem with hard links (btrfs and APFS have them). |
| the plan's chat whitespace sentence | the route already refused whitespace with 400 "a message needs some words"; the stored-shape rule was what accepted it — the existing sentence stays, `check_shape` refuses whitespace-only stored text. |
| the plan's `trim(Some(0))` red cell | already total: the cell is a PIN, mutation-proven instead of red. |
| the plan's "`HOUSE` reserved by a named check" | `is_safe_id` refuses the constant's own VALUE (`_house`) by name; the plain id `house` stays legal. |
| the plan's row-11 sentence (the one-person table's archive residual) | the measured truth: a lost stamp AND either a stale mirror or an undo past game over at a one-person table. |
| the plan's "added to both exact-set lists" (Task 10's fixtures) | the exact-set lists hold only files no walk row names; walk-emitted fixtures join `contract.ts`'s rows. |
| the plan's Names table (`{ kind: "unreadable", sentence }`) | the host's single spelling `{ unreadable }` (PR #37's N-1). |
| the M6.2 close's "a seatless creator may abandon" | false at HEAD: `config_of` refuses a seatless creator (R-1.6); Abandon shows when `mine` AND the creator holds a seat. |
| `CLAUDE.md`'s whole-game-gate clause | the two-identities arm IS a witness to the acting-seat fence since Task 10 (every input probed). |
| §5 of `worker/RUNBOOK.md`, `picker-dropped` | "the player's next click is a 409" holds for the click a browser sends (with `expect`); a bare line is answered by the prompt it lands on — measured at the twenty-fourth deployment. |
| §7 of `worker/RUNBOOK.md`, the query API | an events-view `count` is bounded by `limit`; totals are `calculations`; a shell log line is found by the `needle`, its fields under `source`. |
| §7 of `worker/RUNBOOK.md`, "a PERMANENT house-chat post per replay run" | since cycle D the walk's delete row tombstones that post in the same run; a rollback to a pre-D version then takes the lobby's chat down (§6). |
| `deploy/RUNBOOK.md` §5 and the plan's brief | a rollback across cycle D does not heal "until a trim": the older binary answers 500 on every chat read and post of a tombstoned document until the roll forward (measured on the base). |
| this spec's Option A (the count hosts, §10.9's R-D rows) | Option A holds only where the frame's `chat`/`rev` ARE the document's `last`/`rev` — true on the Worker, and on the NAS after the close's C1 seed (the house frame's counters read from the stored document at start). |
| Task 14's commit subject ("hardened") | row 19's two once-flaky jsdom cells were NOT reproduced over five loaded runs and no cell was hardened; a reviewed SHA is never amended, so the truth is recorded forward here and in the close's §7. |
| the plan's Task 15.E.5 (`nas-build.sh`) | the prepared script cut a `git archive` and named the self-hoster's `Dockerfile`; the NAS's recipe is `deploy/RUNBOOK.md` §2's runtime image from the musl binary — rewritten before use. |

### 10.11 Cycle E's planning session with the owner (revision 2.9, 2026-09-29)

Taken against the tree cycle D left (`web-room-m6.5c2d` at `c06c4b36`; the reviewed code head `4212f6a0`; the E
handover `docs/plans/2026-09-29-web-frontend-m6.5c2e-handover.md` is the entry), the same day as D's close. The
session began from §4's row E as revision 2.8 wrote it and the 32 E rows of the triage page, and ended with **row E
amended (revision 2.9)**, nine rulings (the ninth given at the section's approval), and one defect found LIVE during the session that E now fixes first. Each
ruling carries the owner's words where the owner gave them, so it can be found by search. None changes a mechanism
of §2.1, §2.5–§2.8 or §2.10 beyond what the rulings name; R-6.5c2.39 amends R-6.5c2.5's "no open-door Worker is
deployed" for one vetting window and says so.

**The defect found during the session: the update path is broken on both hosts, and a returning browser is pinned
to the release it last installed.** The owner opened `yokohama.moore.nyc` and read `Could not prepare to play
offline — /index.html: its body is 460dc13…, not the release's f9d6c643…. Try again`, on a door WITHOUT the
project link D's release carries. Measured in the session, read-only: (1) the `moore.nyc` zone runs Bot Fight
Mode, whose JavaScript Detections is "automatically enabled and cannot be disabled" (Cloudflare's docs) and injects
a `<script>` loading `/cdn-cgi/challenge-platform/scripts/jsd/main.js` into every HTML response at the edge — every
fetch of `/index.html` on BOTH hosts reads 3,325 B against the release's 2,387, with a different digest per request
(a ray id and a timestamp ride the snippet), while the origin's bytes are untouched. (2) The docs say the snippet is
injected on "HTML pages or page views, excluding AJAX calls", issues a `cf_clearance` cookie, and "has a lifespan of
15 minutes. However, the code is injected again before the session expires." (3) In a fresh Chromium, a
worker-shaped fetch of `/index.html` (`cache: "reload"`, `credentials: "same-origin"`) WITH the cookie the first
page view issued reads 2,386 chars and the digest `f9d6c643…` exactly; with `cf_clearance` cleared it reads 3,324
chars and the snippet — and the deploy's own `read-deploy.mjs install`, in a fresh profile, installed the release
(154 entries, `/index.html` cached at 2,387 B). (4) A browser CONTROLLED by a release is answered from the cache and
never reaches the edge as HTML again (`deploy/RUNBOOK.md` §3.16 measured zero `/cdn-cgi` requests on a controlled
page), so its clearance goes stale and is never renewed, and every later release's install fetch of `/index.html`
is refused; "Try again" repeats it. The owner's Shift+Reload in Edge — a navigation reaching the edge, the snippet
re-run — let the install through, which confirms the mechanism in vivo. The M6.5b2.1 and M6.5c records read the
install only in FRESH profiles ("what makes the edge leave the worker's fetch alone is NOT identified"); D's close
§7 named the update path as unread. One instrument fact beside it: `read-deploy.mjs install` expects "Ready to play
offline." with a period, and cycle B dropped the period, so the reader prints DOES NOT HOLD over a good install.
The 15-minute figure is the docs' and was not waited out.

| ruling | the owner's words | where it lands |
|---|---|---|
| **R-6.5c2.37 — E's authorization: the deploys, the push and the PR are the orchestrator's at will; the merge #31 → #43 and the `v0.6.0` tag are decided at the wrap, on the owner's word, in E or as a short follow-on session.** The live tiers ride the deploys. | *"the deploys won't require my word, they orchestrating agent can deploy at will, same for push and PR"*; *"the merge process and release tagging would be either at the end of the cycle or potentially afterwards it depends on how the plan gets wrapped"* | E's Part 2, and its close |
| **R-6.5c2.38 — the owner's-eye polish rows (58–64, 66–68, 71–73, the game header's spelling, the device card's 33 px row, and row 21's four refusal strips) are the ORCHESTRATOR's calls**, each written in plain language in the plan and shipped with a before and after picture as the review's instrument; the owner rules on none individually. | *"58-73 it's hard to tell exactly what's being asked, the shorthand is so tight I'm not clear on what it is we're really trying to do with those changes, but if you've reviewed them and they seem appropriate then I can see them being straightforward seemingly mostly cosmetic changes"* | E's Tasks 4a and 4b |
| **R-6.5c2.39 — the public door is vetted in TWO states and then REVERTED to today's hostname-wide door; fully open waits for the public repository.** State 1, no door: the Worker deployed under `YOKOHAMA_WEB_IDENTITY=open` (the operator secret deleted for the window, since the shell refuses to start with it set beside `open`; the dev door off, since the guard refuses it under `open`), then `yokohama.moore.nyc` dropped from the `yokohama` Access application's domains and destinations (the NAS and CLI names stay on it), so a stranger names themself at the client's own name step and plays multiplayer. State 2, the gate on the paths: the Worker back on `worker`, the operator secret restored, the dev door still off, and the hostname back on the SAME application as path destinations only — the list derived from the router (every `/api/` route but `health`, plus `/enter`) — so the door, the release and the art are public, single player is zero-login, and "Play with others" goes through Access on the owner's allow-list. Then the revert: the three hostnames as today, the dev door on. This amends R-6.5c2.5 ("no open-door Worker is deployed") for the window alone. The Access changes are three PUTs on ONE application, so the audience and the Worker's verification variable never change; each PUT is rehearsed on a throwaway application and read back (R-6.5c2.44). | *"C is what I meant - we will remove the access protection, verify that we can play (expected) and see the front door in action then without access so anyone can just identify themselves and play multiplayer. Then we will also enable access for the specific paths (we will need to identify which paths would need to be protected) so that the multiplayer button from the main page will then go through the access gate - but this does mean we'll be testing how it behaves when access is not enabled, so this would have that dev door style user sets their own identity before coming in"*; *"What we'll do when we're done with testing State 2 is just remove the path specifics for the access application and put the full site back under access. Ultimately I want to hold on having the site be fully open until I have the repo made public."* | E's Task 8, and Part 2's window |
| **R-6.5c2.40 — the door's texts are BUILD values, three in one mechanism**: the project link as today (`YOKOHAMA_PROJECT_URL`, drawn on the door since D's release; a quiet copy on the lobby), `YOKOHAMA_SITE_BLURB` under the title and above the link (a couple of sentences of plain text; blank draws nothing), and `YOKOHAMA_SITE_NOTICE` as ONE footer line on every screen — the notice alone, no link, minimal; "Not affiliated with Asmodee" until the owner gives more. Build-time because the release is precached under a digest of its own bytes (§2.3's reason), and both hosts ship the one build, so the words agree by construction; the self-host image gains the two build arguments beside the first. | *"did we include a variable to have a link to https://github.com/jmoore2333/yokohama-assistant/ somewhere on the site or home page? Also we should add in a footer which may make sense to also have setup as an env variable so it's consistent for worker and NAS that says "Not affiliated with Asmodee" and it should support maybe another couple of words longer or if we change up that copyright notice for the bottom and that should persist for the games when the env is enabled"*; *"The footer should not carry the repo link, it will be on every screen when enabled for copyright purposes that's a lot of real estate we don't want to give it up lightly, ideally we keep it as minimal impact as possible"*; *"I do think we should have room for some text under the word yokohama, above or including the link, variable controlled, so we can modify that maybe a couple of sentences like we had in an earlier version seeing it now it looks a bit plain but maybe we want it just blank so it should be editable and I think the env variable approach is consistent with our overall config approach and easy to manage for both local and worker designs"* | E's Task 3 |
| **R-6.5c2.41 — the update path: the shell is precached from a STATIC, non-HTML asset, and Bot Fight Mode stays on.** The precache generator emits the shell's bytes a second time as `/shell.bin` (`application/octet-stream` on both hosts, typed by extension; not on the edge's default cache list), the inventory lists it in place of `/index.html`, the worker installs it and answers navigations from that body with the HTML content type it sets itself; the digest check is unchanged; the first navigation still gets and runs the edge's snippet, which is what Bot Fight Mode wants. The install reader's period is fixed with it. The proof is Part 2's update reading on both hosts with a browser controlled by D's release whose clearance is CLEARED (the state that fails today); a timed wait past the 15 minutes is the fallback proof if any doubt survives review. The header route (`X-Requested-With` on the worker's fetch, measured to work with no cookie at all) is the rejected alternative: it leans on the edge's "AJAX call" judgement rather than on content type. | *"I think your plan makes sense, I like the idea of using a static asset to have that version call that we're checking against so we don't get stuck in this - bot fight mode should remain on that's an important protection at the edge, so our approach should find a different route that works with it and not against it"*; *"It may not matter with the new approach the 15 minute delay, if it does we should test that during the dev with a timer to wait it out"* | E's Task 1, first in the plan |
| **R-6.5c2.42 — the three rows the page marks as the owner's are BY DESIGN, with two things built beside them.** Row 74: no finished-at rule; the Worker's 90-day idle expiry retires finished tables, the NAS keeps none (R-6.5c2.2); E adds the cell that a finished table can be abandoned (its game already in the archive from game over, so the delete files nothing twice), and one live read. Row 83: a seat holder's download of a finished record keeps its seed. Row 91: a finished local game uploads to the archive the deployment configures, nowhere when none is, no per-game prompt. And a WAITING table leaves the lobby: `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS`, default 7, a second TTL on the Worker's expiry for tables whose status is still `open`, its own marker sentence, reported on `/api/health`, in both READMEs; nothing crosses the wire. | *"For 74 agreed, the same expiry time can apply, we should confirm we can abondon a table that has a completed game. I'm not sure about the hide after N days from the lobby but I think it makes sense to bring in without enough players I can see that being a good control to have now and documented. Row 83 Agreed. Row 91 by design we're uploading the games to the host that is specified in the deploy config of the deployment. If none is configured they won't go anywhere but if it is configured like it is in our setups, it will upload no need to change and have per game consent prompts."*; *"7 is a good day to start/deploy/document, agreed with your reading"* | E's Task 6; the lane document's by-design list |
| **R-6.5c2.43 — the rate limit: the Free plan's ONE zone rule, widened to yokohama at a MEASURED value.** The zone's one rule (`Resume - Rate Limiting`: the path `/` on any host, 10 requests per 10 seconds per IP, block for 10 seconds; a second rule was refused in the session, "2 out of 1", and the phase restored) is widened to cover the oracles on `yokohama.moore.nyc` — join, unlock, `POST /api/archive` — at a threshold above the peak the game and its instruments produce, measured per 10-second window per IP across the live replay, the live tier and a human game, with the precache's parallel install fetches kept out of the expression; enabled before the window; read once by a deliberate burst blocked with the replay still holding under it; kept after the revert; the value recorded in the lane document as the Worker-side default. | *"The rate limits on the free plan is very simple it only allows for X over 10 seconds. I already have a rule for the root of moore.nyc for when rate exceeds 10 over 10 seconds to block and I could expand that later to yokohama with a more appropriate value but since we'll be gating back to access after testing we can jsut note that in our docs and readme for the web lane as a default reccomended value o the worker side. I'm not worried about the rate limiting blowing up with some brief online exposure I don't see it being pounced on. We could expand the rule to include yokohama.moore.nyc now but I think we should have to then bring it above 10 request value given how the tables work with requests when in action or testing so it does make sense to know what that number reasonably should be and then enable it so we can know it's tested/vetted."* | E's Task 7 (the number), Part 2 (the rule) |
| **R-6.5c2.44 — the Access changes are automated through the API, confirmed in the session**: the Cloudflare MCP's credential created a throwaway self-hosted application on a hostname nothing serves and deleted it in the same call, both successful, nothing left behind; the real application was read in full (three policies — two reusable by id, one the app's own — and one audience that stays with it). The owner need not be present for the window; the orchestrator messages at each state. | *"Unless you have the ability via the API key to make the modifications to the access application for protect and back then I would have to be there for that - we should confirm if that can be automated."* | Part 2's window |
| **R-6.5c2.45 — the README's shape at 0.6.0 (given at the spec's approval): section links at the top, since the file is long; and the web lane's section — or its sub-page — carries a getting-started section, the DEFAULT settings both deployments run, every variable the lane reads (the build values, the launch variables, the Worker's vars and secrets) present, defined and explained, and a MAP of the API pathways.** The map is checked against the route table by a cell, so a route added later fails the build until the map names it; the variables' table is checked the same way against the names the code reads, where a reader for them exists. | *"the readme may need linked to sections for more detail as it's getting fairly long and we should incude the web lane section of the readme or it's sub page the default settings we're using along with a getting started module/section and how all our variable options are present and defined and what they mean. Same too with a map of the api pathways etc."* | Part 3's README and lane-document tasks |

**§4's row E, as amended (revision 2.9).** Row E's revision-2.8 content is REPLACED; the stack's thirteenth PR is
#43 (`web-room-m6.5c2e` → `web-room-m6.5c2d`).

| cycle | branch (its base) | plan | content | ends with |
|---|---|---|---|---|
| **E — the finish** | `web-room-m6.5c2e` (on `-d`) | `2026-09-29-web-frontend-m6.5c2e-plan.md` | **Part 1, the code**, nine SDD tasks: (1) the shell as a static asset and the install reader's period (R-6.5c2.41); (2) D's leftovers — the `bots.rs` clause, a log line on the house seed's read failure, the `wire_fixtures` house-frame fixture (a named corpus move), the comments citing untracked reviews re-pointed at the close documents, the `ws.test.ts` hostname; (3) the door's three build values — the blurb, the footer, the lobby's link — and the self-host build arguments (R-6.5c2.40); (4a, 4b) the owner's-eye polish by screen with row 21's strips, pictures before and after (R-6.5c2.38); (5) lobby paging for Open and Being played, "Abandon and save" made true (a wire fact on `TableView`, a named corpus move), the arrival by link straight to the table; (6) the waiting-table expiry, default 7 days (a named corpus move: `health.json`), and the finished-table abandon cell (R-6.5c2.42); (7) requests per move measured by route, the client's refetch policy, the target ratified in-task, and the per-10-second peaks for R-6.5c2.43; (8) the door window's code side — a committed `wrangler.open.jsonc` under the guard's door rules, and the protected path list derived from the router with a drift cell (R-6.5c2.39); then the Fable whole-branch review, ONE fix wave, scoped re-reviews, a mutation battery over the new cells, and the ONE recorded sequence at the code head. **Part 2, the live readings**, in order: both deploys (at will, the NAS as one script); the update reading on both hosts with a D-release browser whose clearance is cleared; the quota guard, the live replay twice, the live Playwright tier, the seven screenshots through an opt-in live generator with the `.test` identities, all with the dev door on; the phones — the iPhone 17 Pro and iPad Pro simulators over WARP, one Sonnet agent each in parallel, Android after (R-6.5c2.33); the rate-limit rule widened at the measured value; the door window LAST — state 1 and the stranger test with two typed names (row 114's plain game and the rule's burst), state 2 with the owner's second identity through the gate (row 119, the single-audience login) and zero-login single player, then the revert with the dev door on and one more replay; every step a dated section of `worker/RUNBOOK.md`, the flip kept as a recipe; planned across two UTC days under the 80,000 line. **Part 3, the documents**, five SDD tasks: the lane document from the D8 draft re-read against E's tree, with the new trap and the full door record; `CLAUDE.md`'s five web passages to pointers and its counts in place; the guidingpath archive by grouping with the byte-check script and the third relocation record, the live file nearly fresh (R-6.5c2.32); the board's history to dated pointers and its stale lines; the README at 0.6.0 in R-6.5c2.45's shape (section links, the getting-started section, the defaults, every variable, the API map with its drift cell), the runbook's two "live build" lines and the seven captions. | the triage page's E rows written done with their commits; the close document with its §7; this spec's §10.12; the guidingpath row; the memory; the handover to the AI lane's re-entry; then, on the owner's word at the wrap, the merge #31 → #43 with merge commits under R-6.5c2.31's disabled workflows, the `v0.6.0` tag, `release.yml` read to green and the workflows re-enabled — in E or as a short follow-on session (R-6.5c2.37) |

**The tiers** (R-6.5c2.36): Sonnet 5.5 implements Tasks 2, 3 and 8, the phones, and the board and README
documents; Opus implements Tasks 1, 4a, 4b, 5, 6 and 7 and the lane, `CLAUDE.md` and guidingpath documents; every
reviewer is Opus; Fable orchestrates, reviews the whole branch and writes the close.

**The fences, every commit:** the five fenced crates and the CLI byte-identical to `c06c4b36`; `SEMANTICS_REV` 7;
no bot behaviour moved; the corpus moved only by Tasks 2, 5 and 6 (`health.json` carries the new TTL), its diff read.

**What E will not claim, written before the work:** the handsets (the simulators stand in; the owner's handset is a
ten-minute optional confirmation, R-6.5c2.33); the public door as an end state (vetted, reverted); a public LOAD
(one stranger, not many); the rate-limit threshold as tuned (measured once, chosen with margin); the 15-minute
window (the docs' figure) unless the timer arm runs; the NAS's waiting tables (no TTL by R-6.5c2.2); CI dark on
GitHub until the merge; the exit spec, optional.

**Assumptions the plan carries:** the owner can sign in a second address for the gate test; the footer reads "Not
affiliated with Asmodee"; the plan lands near D's size (fifteen tasks, 1,159 lines) and is read whole by one agent.

### 10.12 Corrections recorded in execution — cycle E (2026-09-30)

Cycle E ran by SDD from the plan `docs/plans/2026-09-29-web-frontend-m6.5c2e-plan.md` (16 tasks, R-E1–R-E17); the ledger is the plan's workspace under `.superpowers/sdd/`, gitignored; the record of record is the close `docs/plans/2026-09-30-web-frontend-m6.5c2e-close.md`. What execution found false or narrower in this specification's own text, recorded forward (the dated sections above are never edited):

- **§10.11's premise that `.bin` is not on the edge's default cache list is FALSE** (the plan's ruling 9-e). `.bin` IS on Cloudflare's default cached-extension list; what keeps `/shell.bin` out of the edge's cache is the `Cache-Control: no-store` both hosts send (`assets.rs::cache_control`, `worker/static/_headers`) — read as `cf-cache-status: BYPASS` on the NAS, twice a minute apart, and as a content-addressed `HIT` on Workers Assets that served the new bytes (interpreted, not measured across a second deploy). R-6.5c2.41's mechanism stands; its stated reason is corrected. The update reading PASSED on both hosts on 2026-09-30 (`worker/RUNBOOK.md` §19, `deploy/RUNBOOK.md` §3.19).
- **R-6.5c2.39's state 2 named `health`, `art` and `labels` as public.** The plan's R-E11 derived the protected list from the shell's router with `/api/health` the ONE public API path, so the window's state 2 put NINE destinations behind Access — `/api/admin`, `/api/archive`, `/api/art`, `/api/chat`, `/api/games`, `/api/labels`, `/api/me`, `/api/stream`, `/enter` — and the door, the shell and the release stayed public. The plan's ruling supersedes the session's list; the lane document's public-door section says which reading each row rests on.
- **R-E10's window configurations are FILES, not `--var`:** `wrangler deploy --var` prints `(hidden)` and the deploy guard cannot read it (Task 8 measured NO), so the door is a committed configuration's `vars` — `wrangler.open.jsonc` and `wrangler.gate.jsonc`, each held to production but for its door by the guard.
- **R-E15's two UTC days became one** (the plan's ruling 10-a): day 1's blocks left the quota at ~30,000 of the owner's 80,000, so day 2's blocks (the rate-limit rule, the door window) ran the same day with the quota read before each; the day closed at 29,211 Worker requests, 205,709 rows read, 20,839 written.
- **The dealing table's TTL** (ruling 9-b): R-6.5c2.42's waiting-table TTL applies to status `open` ("without enough players"); a table at `dealing` has every seat taken and waits on a keep, so it waits the 90-day TTL — by design, in the lane document's by-design list.
- **`TurnMirror.has_events` is a bool, not a count** (ruling T5-a): an exact events count in the table document would cost a document write and a House push per move (six pool cells read red); `saves` is decided from the bit for the view and from the record's truth at the delete.
- **The Workers Observability query API needs a `queryId` on every ad-hoc body** (the 400 every earlier body met); **the GraphQL `date` and `datetime` forms agree** — an earlier reading of "the `date` form returns empty today" was a reader looking under `data.viewer` where the MCP's shape is `result.viewer`; what lags is the periodic rollup; **`ends_with(…)` is a FUNCTION** in the rate-limit rules language. Recorded in `worker/RUNBOOK.md` §7.
- **The plan's code-head web figure 393 was D's**: the code head `c39da5a1` reads 397 (374 lib + 5 + 2 + 16), and Task 11's `lane_document` cell adds 4 (ruling 11-a); the plan's Task 11 named `worker/vitest.node.config.ts`, which does not exist — the cell runs under `vitest.scripts.config.ts` (11-b); the by-design list is `docs/web-lane.md` §15, its own section (11-c).
- **The plain two-person game's instrument printed a wrong applied count on its live run** (the counter credited a page with the game's whole history after a picker step); the archived record's 17 events are the figure (9.7 `/api` requests per event with the setup in; the harness game's 7.9 per move over 2,034 moves is the steady state). The fix (`5af3b7bd`) is verified by its local dry run, not a second live run (ruling 10-c).
- **The relocation of `guidingpath.md`** (Task 13) leaves the first archive's header saying the log "continues in `guidingpath.md`" — true, and left as the append-closed file it is (ruling 13-d); CLAUDE.md's "guidingpath's 3.2 landed note" phrase now points into `guidingpath-archive-phase3.md` (13-b, fixed at the close).

### 10.13 Cycle F's planning session with the owner (revision 2.10, 2026-09-30)

Taken against the tree cycle E left (`web-room-m6.5c2e` at `15de8117`, E's close documents; the code head `c39da5a1`), the same day as E's close, over the F handover `docs/plans/2026-09-30-web-frontend-m6.5c2f-handover.md`. **F has a specification of its own — `docs/plans/2026-09-30-web-frontend-m6.5c2f-spec.md` — because it adds mechanisms §2 never described** (the self-host sweep, Turnstile with the edge rule, the resume rule, the release notice, the web presence); this section is the session's rulings in the owner's words, so they can be found by search from this document, and the pointer. The session ran as a brainstorm through the handover's six items and four the owner added; the design was approved section by section; the thirteen rulings below are the F spec's §1, restated. None changes a mechanism of §2.1–§2.10 beyond what the rulings name; R-F4 amends R-6.5c2.10 forward and R-F5 corrects the M6.5c objects spec's assets block forward.

| ruling | the owner's words | the F spec |
|---|---|---|
| **R-F1 — the wording path: build values stand** (R-6.5c2.40); no `site.json`. | *"I agree with the build values, it helps keep parity as well with the docker/NAS version"*; *"We would not cut a new release on the repo because of a footer change, that's a build/deploy variable"* | §2.1 |
| **R-F2 — the release notice**: one build value naming a release feed, its default derived from the project link, read by the door and the lobby in the visitor's browser, no server involved. | *"we should include an opt-in variable at build time with the default value to check for that, same with build scrips for people the default value to be set and in the docs. Automatic notifcations won't be necessary, although the front door of the site on that local hosted version should say if it is enabled that a new version is available in a bubble/notice somewhere on the page so at least the operator can tell"* | §2.2 |
| **R-F3 — the update path**: the re-check on focus and every 15 minutes, the door applying a ready release itself, one passive click line elsewhere, nothing reloading by itself in a game, no Worker invocation anywhere in it. | *"The footer change 15 minute is good same with the blurb"*; *"critically, I do not want to take an approach that causes worker invocations, this will be a free tier release"* | §2.1 |
| **R-F4 — the resume rule, amending R-6.5c2.10 forward**: a build that reconstructs the exact recorded bot plays on; new games deal only the production bot; every game that truly cannot continue is refused with its reason; `SEMANTICS_REV` the marker, no shim for an older rev. | *"I think that we should adjust here so that a V5 build can still play V4, but only allows new games to be created in the most current version"*; *"if we can play it, we should allow them to continue"*; *"the resume rule we should be putting in F this is what wraps the web lane"* | §2.3 |
| **R-F5 — the door unmetered as a fence with instruments**: a static 404 page instead of a Worker-answered miss (the M6.5c objects spec §0.3 row 7 measured that a missing path is answered by the Worker; its §2.3 value corrected forward), the probe measured, a ledger of every metered action. | *"Building a 404 as a static asset is definitely soemthing we want to include here"*; *"I think F should include the broader probe measurement"*; *"note the metered-action ledger will suffer in the totals from the realities of the harness but even understanding the range and more importantly the actions will be valuable"* | §2.5 |
| **R-F6 — Turnstile as the no-Access deployment's standing shape behind a switch**: the widget at the multiplayer button and wherever a fetch meets a challenge, pre-clearance, siteverify on `/enter`, one WAF Managed Challenge rule on `/enter` and `/api/*`; three settings held together by the guard; Spin's form-gating shape not taken. | *"there is likely a point at which we would move the full site onto cloudflare without any access barrier"*; *"the internet is filled with scanners and bots and automations and working it out so we keep our workers free tier invocations and metered readings down isteh key goal"*; *"a third option in the worker as a build/deploy variable to enable or disable it entirely if people didn't want to enable it"* | §2.6 |
| **R-F7 — F's authorization**: the deploys, the push, the PR, the public window and its Access and zone changes at the orchestrator's discretion; the merge and the tag on the owner's word at the wrap. | *"The window and changes can be made at the orchestrator's discretion no need to wait for me on those calls, same too for deploys and push and PR"*; *"No problem making it public to check and trial"* | §4, §8 |
| **R-F8 — an R2 dropbox for saves: considered, not taken.** | *"we may want to consider a dropbox kind of approach in r2 as well but it seems we want to consider that in more depth"* | §6 |
| **R-F9 — the self-host sweep**: the Worker's two TTL names and meanings, hourly, through the creator's own delete; a TTL without an archive refuses to start; the listing before any end; no dry-run variable. | the handover's *"include the table TTL days clean up work for the docker release"*; the design accepted | §2.4 |
| **R-F10 — no marker on the self-host.** | *"For question 7, no marker is fine. While parity in many ways between the two is a goal, for areas where there are huge divergence outside what we're getting into, it's understandable"* | §2.4 |
| **R-F11 — one cycle, E's shape.** | *"approach 1 will work here even including the SEO/web presence optimziation task"* | §4 |
| **R-F12 — the web presence**: `robots.txt`, `llms.txt`, a sitemap, the page's metadata and structured data, static, with defaults, the repository link where it belongs. | *"I'd also like to include in this wave a task for SEO/robots.txt/llms.txt etc optimizations for search engines like google and llms so that's handy for any public deployments, default values, structured well and pointing to the GitHub page where appropriate."* | §2.7 |
| **R-F13 — the version stays `0.6.0`**, the tag `v0.6.0`. | approved with the wrap's section | §4 |

**§4's row F (new; row E stands as §10.11 amended it).** The stack's fourteenth PR is F's, `web-room-m6.5c2f` → `web-room-m6.5c2e`.

| cycle | branch (its base) | plan | content | ends with |
|---|---|---|---|---|
| **F — the clean finish** | `web-room-m6.5c2f` (on `-e`) | written from the F spec by the writing-plans skill | **Part 1, the code**: the update path and the guard's cache check; the release notice; the resume rule and the corpus; the sweep; the 404 page, the guard's checks, the presence files, health into the protected list; Turnstile's client half and Worker half with the switch; the ledger's section and cell, the fence cell, the probe instrument; E's minors; the whole-branch review, one fix wave, the battery and the ONE recorded sequence. **Part 2, the live readings**: both deploys, the update reading on both hosts, the NAS sweep's first listing, the public window (Access off the Worker's hostname, the rule on, the switch on; the probe, the curl-shaped readings, the owner's browser, the link arrival, the game-over upload, the outbox), the revert and one replay holding, the tally. **Part 3, the documents** (the F spec §2.9) and the close with its §7. | on the owner's word, the merge #31 → #43 → F's PR bottom-up, the `v0.6.0` tag, `release.yml` read to green, `test.yml` and `docker.yml` re-enabled, one documents commit on master; then the refocus on the computer player |

**What F will not claim, written before the work** (the F spec §6): the fully open end state (vetted in the window, reverted until the public repository); the real widget outside the window; the feed's notice live (the repository is private, so the feed answers 404); a public load; the handsets, still the simulators; rows 108 and 119, still owed a second identity.
