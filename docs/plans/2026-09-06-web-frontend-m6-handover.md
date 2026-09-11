# Web frontend — M6 "the room": handover into the next session

**Written 2026-09-06 on branch `web-frontend` at `fbe268f`** — the corrections record's own
commit, the last of that wave — **immediately after those corrections shipped and went live.**
Nothing of M6 has been planned or executed. This file says where the lane stands, what to read,
what M6's charter is, what it inherits, and how the owner wants it run. It is a POINTER, not an
authority: every figure in it is sourced to the document that measured it, and where a claim is
not supported by the record it is marked as not claimed (§6).

## 0. Where the lane stands, at HEAD

- **LIVE: `yokohama-web:0.5.0-m5.5.4`**, built from `fb6cec7`, at `https://yokohama.moore.nyc`
  behind Cloudflare Access on the Synology. Deployed 2026-09-06 13:48Z; healthy at 30 s; the
  container is `d2dfda65` at 172.21.0.4, the same address the previous one held, so `cloudflared`
  was not restarted. **Eight tags now sit on the NAS**, each a rollback point (`deploy/RUNBOOK.md`
  §5): `0.5.0`, `0.5.0-m5`, `0.5.0-m5.1`, `0.5.0-m5.5`, `0.5.0-m5.5.1`, `0.5.0-m5.5.2`,
  `0.5.0-m5.5.3`, `0.5.0-m5.5.4`.
- **Branch `web-frontend`, worktree `.claude/worktrees/cli-web-api-port-936d56`.** Master is an
  ancestor of it and is 0 commits ahead, so the lane has never diverged. Every command runs from
  that worktree by absolute path; never `cd` to the main checkout, and never bare `git stash`
  (the stash stack is shared across worktrees).
- **The counts**, taken at `97b2cb5` (the commit before this file's base; the two since move
  documents only), measured there except where §6 says otherwise — full provenance in the
  corrections record's §5.2:

  | reading | value |
  |---|---|
  | `cd web && npm run typecheck` | clean, both TS projects |
  | `cd web && npm test` | **393** in **24** files |
  | `cd web && npm run e2e` | **37** in 6 spec files (+2 opt-in generators the glob drops) |
  | `cargo test -p yokohama-web` | **82** (76 lib + 1 `play_a_whole_game` + 5 `wire_fixtures`) |
  | `cargo clippy --workspace --all-targets -- -D warnings` | clean |
  | wire corpus `web/src/wire/*.json` | **44** files |
  | `SEMANTICS_REV` | **7** |
  | blast radius over the five fenced crates (`git diff e4f41e4..HEAD`) | **EMPTY** |

- **The deploy recipe is by REFERENCE, never retyped:** `deploy/RUNBOOK.md` §3.5 is the recipe and
  §3.6 is the record of every deployment so far, its last addendum covering `0.5.0-m5.5.3` and
  `0.5.0-m5.5.4`. It runs through the Portainer MCP (environment id 2) in a window the owner
  confirms; the musl binary rebuilds only when `crates/yokohama-web` moves.
- **After EVERY deploy, verify BOTH layouts on screen.** That is the lesson of 2026-09-05, when a
  fix wave's own scroll regression passed two scoped re-reviews and the desktop layout and was
  wrong at every width below 1201 px. And know the trap: **the Chrome window the agent tools open
  on this Mac may refuse to resize** — `resize_window` reports success while `innerWidth` stays
  where it was. `innerWidth` is the truth, not the call's return value. Two sessions hit it. The
  fallback that works is headless Chromium through Playwright from the shell against the same live
  server (WARP passes Access for it), plus a browser whose window happens to be the size you need.

## 1. What to read, in order

1. `CLAUDE.md` — the whole "Web (server)" and "Web (browser half)" block under Commands, and the
   `crates/yokohama-web` + `web/` bullet under Architecture.
2. `docs/plans/2026-09-04-web-frontend-m5.5-spec.md` (ratified) and its close record
   `docs/plans/2026-09-05-web-frontend-m5.5-close.md` — **the close's §4 first**: the spec's
   end-of-turn-highlight promise was FALSE for every bot turn while the cell pinning it was green.
3. `docs/plans/2026-09-06-web-frontend-m5.5-owner-corrections.md` — spec, plan and record in one
   for the corrections wave; §0 is the owner's direction verbatim, §1 the rulings, §5 the record.
4. `deploy/RUNBOOK.md` §3 and §5.
5. This file.

## 2. What M5.5 and the corrections shipped

**M5.5 "the table reads"** (close record `2026-09-05-web-frontend-m5.5-close.md`): the opponent's
turn WALKS on the table (`diffFrames` over `/at/{k}` frames, `TurnAccount` with the lit row
following the step, a cadence remembered per browser); a SLOT lights on ARRIVAL and never on the
shift a take causes; the pieces band prices glyphs instead of counting them; and the first card is
chosen over the SET TABLE, a preview built from a throwaway `Repl`. Two additive wire fields and
nothing else moved.

**The owner's first-run corrections** (`2026-09-06-web-frontend-m5.5-owner-corrections.md`, §5 the
record): another seat's deal never reaches the wire or the screen; the technologies and the goals
are withheld until the card is chosen, following the owner's edition of the rulebook; human seats
choose one at a time with the others' cards face down; and the mobile set table diagnosed rather
than changed. Seven rulings were taken in execution beyond the plan's seven, and the record's §5.6
lists all fourteen with their costs.

**Read both records' own warnings before quoting either milestone about itself.** M5.5's close §4
records a cell that was mutation-proof and vacuous at once — its fixture put the change on the
run's last step, which is the one shape a served turn never has — while the spec sentence it
pinned was false on the table for every bot turn. This wave's own reviews then falsified a
call-site comment, a `describe` comment, a test title, a harness comment, the setup form's
subtitle, and one implementer's self-review claim that every sentence it had touched was true;
and its final review found one mutation green in three arms. **The method that finds these is to
try the mechanism the sentence promises, not to re-read the sentence.**

## 3. M6's charter

### 3.1 Verbatim from the M5.5 spec's §6.3 (`docs/plans/2026-09-04-web-frontend-m5.5-spec.md:540-554`)

> The close doc of M5.5 is M6's entry artifact and carries, in this order:
>
> - **The walk as the spectator primitive.** A spectator's client feeds `[prev, next)` from
>   a stream of events instead of a reply; every component of §3.2 is reused as is.
> - **M4's hosting list**, inherited unchanged (`m4-spec.md:507-524`): Access JWT
>   verification does not exist and is not free; `GET /api/games` enumerates every game to
>   any caller; the Worker shell (`deploy/worker/src/index.ts`) already forwards the Access
>   identity as `X-Yokohama-User` and strips a client-supplied one.
> - **M6's first three questions**, with the lean stated in conversation on 2026-09-04:
>   the transport (server-sent events over the axum server, since both hosted shapes stream
>   and one process serves three players); identity (verify the Access assertion at the
>   origin, or let the Worker be the only door); the store (Neon Postgres only if the
>   Worker takes a share of the state, else files behind the existing `SessionStore`).
> - **M5's LEAVE list** items M6 inherits (handover §6): the unbounded saved-game list, the
>   "New game" in-flight guard, the popover's browser cell.

### 3.2 What the corrections wave adds to that charter

**A hidden deal is properly a per-seat VIEW, and M6's multi-browser room is what makes it
natural.** The corrections improved the SEQUENCE, not the payload: one browser serves every human
seat, so `POST /api/games/preview` still returns every human seat's two cards, and the secrecy
rests on the earlier chooser's cards being removed from the DOM when the screen is passed on. That
is honest hot-seat and it is stated as such in the record's §5.8 — **it is the interim, not the
answer.** When a seat is a browser rather than a turn, the deal is scoped the way
`game/observe.rs` already scopes everything else, and the set table becomes a per-seat projection
like every other payload this server serves.

Two things follow for M6's design, and neither is settled here:

- The preview route is where a per-seat deal would land, and it is **unauthenticated at the
  origin and now expensive** — it runs `Session::new_sim` and `HistoricalFrame::at` for a whole
  throwaway position on every call (the M5.5 close's §7 says so). Identity and this route meet.
- `client.rs`'s `replay_seat` already pins ONE seat for the life of a game, deliberately not
  `viewing_seat`. A room has a different answer: the seat follows the viewer, not the mover, and
  not a fixed index. Read M4's spec before changing it.

## 4. The LEAVE list M6 inherits

### 4.1 From the M5.5 close's §7 (`2026-09-05-web-frontend-m5.5-close.md`)

- The **Game menu stays open** after a game is chosen from it (M5's component).
- The **tile popover at 390 px** covers most of the screen (M5 behaviour).
- **`POST /api/games/preview` builds a full session per call, unauthenticated at the origin** —
  the same place on M4's hosting list, more work per anonymous request.
- **Above 1201 px nothing scrolls the account's lit row into view** (R-F13): the mechanism is
  guarded by `scrollHeight > clientHeight`, and above 1201 px the cap is released and the side
  column is the scroller, so §3.2.5's sentence is delivered below 1201 px and not above it. That
  ruling also names the generalization it deferred — scroll the nearest scrollable ancestor that
  is not the document (`.side` on desktop, `.messages` in the stack) — which is where a fix should
  start, under R-F11's constraint below.
- The **human's own closing event walks as step 1** (R-F7), and the **opening-run walk is
  unreachable from the wizard** (R-F8).
- The set table's **Keep hedge is an unpinned copy** of the CLI's `KEEP_SCORE_NOTE`: a reword in
  `wizard.rs` drifts the browser silently. Pinning it wants a `pub` and a wire field.
- **`useSections()` is a footgun by shape**: a third caller repeats the defect R-T12-4 fixed, and
  only a comment and a cell guard it.
- The **whole-game server gate is OS-entropy seeded and capped twice** in that milestone's runs
  (its 20,000-input cap hit at turn 7,295 with the bot hoarding ¥7,273), then passed 660
  consecutive times. Pre-existing, and it belongs to the AI lane as much as this one: a 2P game
  the bot cannot end.
- **M5.5's own deferred minors, which its close hands on as a list rather than as items**: the
  ledger recorded them at forty-four points across its Tasks 0–12 (several entries carrying more
  than one), plus the five the R-F11 re-review found and R-F12 parked for M6's entry — a folded
  account scrolled while clipped that self-corrects at the next step; `box.offsetTop` and the box
  border as untested arithmetic; the new e2e cell's best-effort pause; that cell's admission
  depending on the server-drawn seed; and one assertion in a carried cell that cannot fail. **That
  ledger is not in git** — the M5.5 workspace was archived to gitignored
  `docs/temp/sdd-archive/2026-09-04-web-frontend-m5.5-plan.tar.gz`, which is where the whole list
  is.

### 4.2 New from the corrections wave (`2026-09-06-…-owner-corrections.md` §5.5, §5.7)

- **R-C11 — the asking seat's label is half-clipped after the pass at 1000×563** (not at 390×844
  and not at 1440×900, where a cell gates it). Cosmetic at one viewport. **The lesson rides with
  it, and it is R-F11's:** the fix, if taken, scrolls the DECK's own scroller by a computed
  `scrollTop` — **never `scrollIntoView`**, which is right above 1201 px and scrolls the WINDOW
  below it, measured live on 2026-09-05 as a walk yanking the page 1826 → 1877 px and hiding the
  row it meant to show.
- **The 1200/1201 strip-card artifact.** A port or laboratory strip card is WIDER at 1200 px (144)
  than at 1201 px (128), because `--card-w`'s `12cqw` tracks the WINDOW below the breakpoint (no
  query container: `container-type: size` on `.table` is declared inside
  `@media (min-width: 1201px)`) and `.table` above it. **No cell sits at that pixel** — the width
  cells are at 390, 1024, 1201, 1440, 1710 and 2560.
- **The mobile stack does not scale, and that is a LAYOUT QUESTION rather than a defect.** Page
  overflow is 0 at every width from 320 to 2560 and the deal card is 128 px at all of them; what
  differs on a phone is everything around the cards — a `14rem` tile chosen by player count, the
  board scrolling 557 px inside its own column, and a sticky deck taking 504 of 844 px. Making the
  cards use the room re-opens the deck's height budget, the pinned-deck rule and the hedge's fit.
  Readings and pictures: the record's §5.5 and the gitignored
  `docs/temp/web-review/mobile/` (regenerate them rather than trust them; they are a throwaway
  script's).
- **Nine deferred minors**, every one a comment or an unpinned statement and none a behaviour,
  itemised in the record's §5.7.

## 5. The process the owner wants

- **Model dispatch (the plan's R-C7, and the owner's standing floor):** OPUS implementers and
  OPUS per-task reviewers; Fable orchestrates and runs ONE clean final review, with no prior
  context, against the LIVE site. No Sonnet, no Haiku, on anything with a plan behind it.
- **The final review is against the deployed thing, in BOTH layouts** (§0's trap included). It
  earns its keep: on this wave it tried 17 mutations and 16 were caught, so the one that survived
  became a cell in the fix commit; and a separate finding in the same review — the setup form's
  subtitle, which is PRODUCT TEXT — is what forced a second deployment, because the live site is
  kept equal to HEAD.
- **The SDD workspace convention:** `.superpowers/sdd/<plan-basename>/progress.md`, gitignored by
  `.superpowers/sdd/.gitignore`, in the shape this wave used — a pre-flight scan of the task
  pairs, then one ruling row per decision, one report per task, one review per task and a
  re-review per fix round. At the finish it is ARCHIVED to gitignored `docs/temp/sdd-archive/`
  rather than deleted; M5.5's sits there as
  `2026-09-04-web-frontend-m5.5-plan.tar.gz`. **The corrections wave's workspace was deliberately
  KEPT unarchived**, at
  `.superpowers/sdd/2026-09-06-web-frontend-m5.5-owner-corrections/`.
- **The decision log is APPEND-ONLY.** Corrections ride FORWARD as new rows; a row is never
  edited, and neither is a dated artifact after its phase closes.
- **The wire-corpus contract:** regenerate with
  `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, run it again WITHOUT
  the variable, and **read the diff and report it** — it is a contract, not a snapshot.
- **No hand-authored payload in a test.** Every fixture a component test renders comes from
  `web/src/wire/`, because a builder is a third statement of the wire shape that keeps compiling
  after the server drops a field. When a shape you need is not in the corpus, the answer is a
  corpus fixture or a committed one that already has the shape (ruling R-C8 on this wave), never a
  literal.
- **WARP passes Access from the shell**, so `curl` and Playwright can both reach the live origin
  without a browser session.
- **Gate per task:** `cd web && npm run typecheck && npm test && npm run e2e`; and when `crates/`
  moves, `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings`.
  The blast radius over the five fenced crates stays EMPTY and `SEMANTICS_REV` stays 7 — that
  constraint has held for the whole lane and there is no reason for M6 to be the first to break
  it. `git add` explicit paths; a clean `git status` is the steady state.

## 6. What this handover does NOT claim

- **M6 is not specified.** §3 is a charter quoted from a ratified spec plus one addition the
  corrections wave earned. The transport, the identity model and the store are OPEN QUESTIONS
  with leans, not decisions, and each wants its own brainstorm → spec → plan.
- **No estimate of the room's size.** Nothing here says whether SSE over the axum server is
  enough, whether the Worker should hold state, or what a lobby costs.
- **The counts in §0 are this HEAD's, with two exceptions the corrections record states in its
  §5.2**: the browser walk's 37, and the whole-game server gate's single test — both are readings
  taken by the final reviewer at `f5cb50e` and by the fix wave's implementer at `fb6cec7`, and
  neither `web/e2e/` nor `crates/` has moved since. The server suite's other 81 (76 lib + 5
  `wire_fixtures`) were re-run at `97b2cb5`. Re-count at your own HEAD anyway; the repo's standing
  rule is that a doc's count is a lineage, not an authority.
- **Nothing about the below-1201 stack is claimed from the deploying session's own browser.** The
  live evidence for that layout is the final review's Chrome at 1000×563 and Playwright at
  390×844 against the same live server.
- **This lane still has no gate and no roster place.** The fifteen release gates are untouched and
  nothing this lane added is `#[ignore]`d.
