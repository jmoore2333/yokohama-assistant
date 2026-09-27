# M6.5c2 "the cleanup" — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. The execution shape is §"Execution shape" below, which records the owner's standing answers: every
> task subagent-driven, Opus for every implementer and reviewer, Fable for the whole-branch review. Steps use
> checkbox (`- [ ]`) syntax for tracking. **Read the spec first**; this plan argues from it and does not restate its
> reasons.

**Goal:** Finish the web lane.
- The front door, the lobby and their sibling screens become designed screens.
- The manual-identity "dev door" becomes a real open door on both hosts, while the repository's defaults stay
  Access.
- Every save names its bot, rules, release and host.
- Every design debt the M6.5c reviews listed is closed.
- The two hosts' code and documents are arranged by host.
- The cycle closes with one full recorded test sequence, both hosts deployed, the nine-PR stack merged and
  `v0.6.0` tagged.

**Architecture:** Change runs outward from the table layer (`crates/yokohama-web`), so every later task stands on
the final layer:
- **Phase A** changes the layer: identity, the bot's label and the foreign-bot rule, `run_id`, the logging seam,
  the refinish rule, and the small corrections, each with its cells and the committed corpus regenerated where the
  wire moves.
- **Phase B** rebuilds the client's setup screens in the approved look.
- **Phase C** brings the same changes to the Cloudflare Worker (the two doors, the refinish rule's shell half) and
  pays its refactor debts behind the pool and the corpus.
- **Phase D** is the build, CI and the release number.
- **Phase E** is the documents.
- **Phase F** is the orchestrator's account steps and the close.

**Tech Stack:**
- **Rust:** 1.97.1 (pinned), axum on the NAS, `wasm32-unknown-unknown` + `wasm-bindgen 0.2.128` for the two
  WebAssembly hosts, `wasm-opt` (binaryen **133** everywhere after Task 24).
- **The client:** React + Vite in `web/`, with vitest and jsdom for component cells, and Playwright for the browser
  walk.
- **The Worker:** TypeScript on Cloudflare Workers with SQLite Durable Objects and R2, tested in workerd through
  Cloudflare's Workers Vitest integration.
- **CI:** GitHub Actions; `docker buildx` for the self-host image.

**Spec:** `docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md` at **`ca68b35d`** (revision 2.4; R-6.5c2.1–19).
- §2 is the mechanism, §3 the cells every task ends in, §5 the fences, §8 the risks, §10 the review record.
- The approved mockups are `docs/plans/2026-09-26-web-frontend-m6.5c2-mocks/01-…11-*.webp`, with their HTML
  sources gitignored in `.superpowers/brainstorm/20585-1790461301/content/`.
- The entry artifact is `docs/plans/2026-09-26-web-frontend-m6.5c2-handover.md`, whose §6 lists the traps not to
  re-derive.

---

## Global Constraints

- **Branch `web-room-m6.5c2`**, stacked on `web-room-m6.5c`; **the base for every code diff is `1e16fe27`** (the M6.5c
  close head; the four spec commits after it touch only `docs/plans/`).
  - Stage with explicit paths, never `git add -A`. `git status` is clean after every commit.
  - Nothing is pushed until Task 28. The push, PR #39 against `web-room-m6.5c`, the merge and the tag are
    Task 28's, **each on the owner's word at the close**.
- **Fences** (spec §5). At every commit, this prints nothing:
  `git diff --stat 1e16fe27 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli`
  - **The CLI needs no change** (spec §5). If a task finds a seam it truly needs, it stops and asks the
    ORCHESTRATOR, which rules on it with a `Ruling:` line. It is then a behaviour-preserving widening, named.
  - `SEMANTICS_REV` stays **7**. **No bot behaviour moves.**
  - The version bump (Task 25) changes the ROOT `Cargo.toml` and the lock files, never a fenced crate's source.
- **The layer changes only as the spec names them** (§2.1, §2.4, §2.5, §2.6). Each task's reviewer reads
  `git diff 1e16fe27 -- crates/yokohama-web/src` against the task's own list.
- **The committed corpus** (`web/src/wire/*.json`, `web/src/wire-script/*`) is regenerated ONLY by the tasks that say
  so, with `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`. The diff is READ and quoted in
  the task's report. A changed payload the task did not predict is a FINDING: stop and report it rather than
  commit it.
- **The M6.5c standing rules hold for all Worker code** (CLAUDE.md, "The Worker host — the standing rules"):
  - the strings rule;
  - the two time bases;
  - one batch per request;
  - every client `x-yokohama-*` header stripped;
  - the pool facts (the M6.5c plan's "Pool facts", measured).
- **Every task that changes what the client SHOWS loads the `frontend-design:frontend-design` skill** (R-6.5c2.18).
  - It builds against the committed mockups, in the app's own stylesheet (`web/src/styles.css`) and tokens.
  - **No new colour roles.** The palette is the `:root` tokens at the top of `styles.css`.
- **The frontend's two hard rules:**
  - the door (`#/`) **asks the server nothing** (R-6.5b2.7);
  - **no card art on the setup screens** (`ASSETS.md`).
- **Per-edit gates after EVERY task.** Re-count at your own HEAD; a figure in any document is a reading, not a
  constant.
  - **Rust:**
    - `cargo test -p yokohama-web`, `cargo test -p yokohama-wasm` and `cargo test -p yokohama-worker`;
    - `cargo clippy --workspace --all-targets -- -D warnings`;
    - `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`;
    - the public build's identity cells, `cargo test -p yokohama-web --no-default-features --features server --lib identity::`,
      whose count CI asserts (Task 2 moves it);
    - `cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm` and
      `cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker`.
  - **In `web/`:** `npm run typecheck`, `npm test` and `npm run test:wasm`, which needs `npm run build:wasm` first
    whenever Rust the browser module compiles changed.
  - **In `worker/`:** **`npm run build:module:debug` FIRST** whenever `crates/yokohama-worker` or the layer changed,
    or the pool tests a stale module GREEN. Then `npm run typecheck`, `npm run test:scripts` and `npm test`, which
    runs **both pool projects** from Task 17 on.
  - **`cd web && npm run e2e`** is owed by any task that changes what a browser DOES or SHOWS. Its run is sequenced,
    never two Playwright runs at once.
  - **`cargo test -p yokohama-cli`** (with its goldens and `guide_sync`) at every task that touches something the CLI
    compiles: Task 25's version bump, and any task whose diff reaches `Cargo.toml`/`Cargo.lock`.
- **The whole-game gate's vs-bot arm livelocks on some OS-entropy seeds** until Task 8 fixes it: re-run it ALONE at
  the same commit before believing a red.
- **Any edit to a workflow file** runs `python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" <file>` in its
  gate.
- **Live actions on the owner's account are the ORCHESTRATOR's, never a subagent's.** A subagent prepares the code and
  the commands; the orchestrator runs them. That covers:
  - deploys, secrets and the bucket;
  - `wrangler tail`;
  - the live Playwright tier, the live replay and the headed game;
  - every WARP reading.
- **The quota guard** before every live run that plays games. The orchestrator reads the UTC day's Worker requests and
  Durable Object rows written from the account's GraphQL analytics (RUNBOOK §6.7), and HOLDS above **80,000** (the
  owner's line for M6.5c, standing).
- **Secrets:** `.dev.vars` holds the Service Auth credential (`CF-Access-Client-Id`, `CF-Access-Client-Secret`) and
  the read-only R2 key. **Never print it, never commit it, never paste it into a report.**
- **The owner's email address goes into no document**; write "the owner" (R-6.5c2.16).
- **The ledger** is `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/` (gitignored).
  - `progress.md` holds one line per task and one `Ruling:` line per deviation from this plan.
  - Every agent writes its full report to a file there and replies with the PATH and three lines.
- **Commit messages end with the EXECUTING agent's own attribution lines.**
- **`cargo test` takes ONE positional filter**: one command per cell, or a shared substring.

---

## Review Focus

The spec implies these five inputs, and no cell would meet them unless someone put it there. Each line says what a
person would expect; its test lives in the owning task and is marked **(Review Focus N)** there.

1. **An email with a `+` in it, or a name with a space, carried on the open-door Worker's WebSocket upgrade** as the
   `claimed=` parameter (`a+b@x.test`, `ben moore`). Expected: the socket's caller is exactly the `fetch` caller.
   The client encodes with `URLSearchParams`, the shell decodes, and a `+` never becomes a space. (Task 17)
2. **Seed text the server accepts that a naive client check would refuse, and the reverse**: `"+007 "`, `" 7"`,
   `"007"` are accepted (Rust's `u64::from_str` after `trim`); `"７"` (a full-width digit), `"1e3"`, `"-1"` and
   `"18446744073709551616"` are refused. Expected: the Advanced fold agrees with the server on every one, because
   both read the same committed table. (Task 13)
3. **A browser whose storage still holds an address typed under `claimed`, visiting a `cf-access` host** (the NAS
   after its redeploy). Expected: the corner shows the Access identity from `/api/me`, never the stale stored name.
   The stale header is ignored by the server, and nothing asks the player to "switch player". (Task 11)
4. **A name typed with capitals and extra spaces** (`"  Ben   Moore "`). Expected: one key, `ben moore`, shown as
   typed, `Ben Moore`. The same person typing `ben moore` later reaches the same seats. (Task 1 for the key,
   Task 11 for the display)
5. **A spectator (seated nowhere) opening a finished game played against a foreign bot.** Expected:
   - the Players panel says the bot (with its label);
   - `why` names the bot's version rather than "a human's move";
   - there's no undo control;
   - nothing is refused that a finished game shows today.
   (Task 4)

---

## Execution shape (the owner's standing answers)

- **Mode: SUBAGENT-DRIVEN, EVERY TASK** (the owner, 2026-09-21, standing; R-6.5c2.18).
  - Each task gets a fresh **Opus** implementer and a fresh **Opus** task reviewer. **Fable** does the whole-branch
    review in Task 28.
  - The orchestrator runs every step marked **ORCHESTRATOR** itself.
  - Standing process rules:
    - never follow up an implementer's DONE without reading HEAD first;
    - a reviewed SHA is never amended; a fix is a new commit;
    - while an implementer works, run nothing in its tree and arm no timers;
    - wake a stalled agent by message rather than spawning another;
    - a reviewer's mutants run on an export (`git archive`), never in a tree an implementer holds.
- **ONE GREEN COMMIT PER TASK.** Tests are written first and RUN RED, and the red run is quoted in the commit
  message's body and in the ledger. Every commit passes every per-edit gate.

---

## Names shared across tasks

An implementer sees only its own task. **These names are the contract between tasks and are used verbatim.** A task
that finds the code forces a different shape stops and asks the ORCHESTRATOR, which amends this table with a
`Ruling:` line. It never renames on its own.

### The layer (`crates/yokohama-web`)

| name | shape | task |
|---|---|---|
| `User::parse(raw: &str) -> Result<User, String>` | the KEY grammar: today's email shape **or** a plain name (1–32 of ASCII letters, digits, space, `.`, `-`, `_`; **begins with a letter or digit**; internal space runs collapse; `to_ascii_lowercase`; `anonymous` refused) | 1 |
| `User::is_email(&self) -> bool` | whether the key has the email shape | 1 |
| `archive::paths(rev, prefix, why, id) -> Result<(String, String), String>` | refuses any `.`/`..` component | 1 |
| `DirArchive::put` | + a LEXICAL check (no `Component::ParentDir` in the joined path) | 1 |
| `IdentityMode::Open` | replaces `Claimed`; the variable's value `open`; its wire name `open` | 2 |
| cargo feature `open-door` | replaces `dev-identity`; still a default feature | 2 |
| `identity::resolve(mode, headers)` | + **the mode rule**: under `CfAccess`/`Worker` a non-email reads as ABSENT | 2 |
| `bots::BotDescriptor { controller: String, version: u32, params: BTreeMap<String, i64> }` | `Serialize`, `Deserialize`, `Clone`, `PartialEq`, `Eq`, `Debug`; = a `SeatProvenance`'s triple | 3 |
| `bots::production() -> BotDescriptor` | `PROVISIONAL_DEFAULT_BOT`'s descriptor | 3 |
| `bots::label(d: &BotDescriptor) -> String` | through `Contestant::from_parts` and an exhaustive match with NO `_` arm; an unparseable descriptor answers its `controller` | 3 |
| `bots::recorded(record: &GameRecord) -> Vec<(u8, BotDescriptor)>` | the record's non-human seats | 3 |
| `TurnMirror.bot: Option<BotDescriptor>` | `#[serde(default)]`; `TurnMirror` loses `Copy` | 3 |
| `TableView.bot: Option<TableBot>`; `TableBot { label: String, runs_here: bool }` | wire | 3 |
| `ClientState.bots: Vec<SeatBot>`; `SeatBot { seat: u8, label: String }` | wire | 3 |
| `web/src/wire/production_bot.json` | `{"identity": "<controller>", "label": "<label>"}`, emitted by `wire_fixtures`, refused when stale | 3 |
| `GameError::ForeignBot { played: String, runs: String }` | → **409** `{"error": "<sentence>", "foreign_bot": {"played": "V4", "runs": "V5"}}` | 4 |
| `WhyResponse::foreign(played: &str) -> WhyResponse` | refusal `kind: "foreign_bot"`; the `WhyResponse::locked` precedent | 4 |
| `RunHost::{Server, Worker, Browser}`; `RunHost::run_id(&self) -> &'static str` | `"web/server"`, `"web/worker"`, `"web/browser"`; passed to `Games::with_parts` | 5 |
| `pub trait Log: Send + Sync { fn line(&self, event: &'static str, fields: serde_json::Value); }` | `Games::with_log(Arc<dyn Log>)`; `StderrLog` under `server`, `NoLog` default, `RecordingLog` in tests | 6 |
| `PutError::Conflict { key: String, held: Option<String> }` | the record already at the key, when known | 6 |
| `client::without_version_drift(warnings: Vec<String>) -> Vec<String>` | drops only lines beginning `warning: GameRecord.engine_version=` | 7 |
| `web/src/wire/identity_cases.json` | `[{"raw": str, "key": str \| null, "display": str \| null}]`, the layer's verdicts; emitted and refused like `site_space_words.json` | 1 |
| `web/src/wire/seed_cases.json` | `[{"raw": str, "ok": bool}]`, `WireSeed`'s verdicts; emitted and refused likewise | 13 |

### The client (`web/src`)

| name | shape | task |
|---|---|---|
| `nameRule.ts`: `normalizeName(raw: string): {ok: true; key: string; display: string} \| {ok: false; reason: string}` | pinned to `identity_cases.json` | 11 |
| `seedRule.ts`: `checkSeed(raw: string): {ok: true} \| {ok: false; reason: string}` | pinned to `seed_cases.json` | 13 |
| `projectLink.ts`: `projectLink(): {href: string; text: string} \| null` | reads the build's define `__YOKOHAMA_PROJECT_URL__` | 10 |
| `web/scripts/project-url.mjs`: `checkProjectUrl(raw: string \| undefined): string \| null` | throws an `Error` naming `YOKOHAMA_PROJECT_URL`; `vite.config.ts` calls it | 10 |
| `BoardMark.tsx`: `<BoardMark width={number} />` | the hero's original board | 9 |
| `Band.tsx`: `<LobbyBand crumbs={string[]} corner={ReactNode} />`, `<IdentityCorner me={Me} onChange={() => void} />` | the lobby band and its corner | 9 |
| `Chips.tsx`: `<SeatChip color label open? toMove? />`, `<PasswordTag />`, `<VersionTag label />` | | 9 |
| `SeatCard.tsx`: `<SeatCard color caption who note? action? open? />` | | 9 |
| `signal.ts`: `Link = "connecting" \| "live" \| "refreshing" \| "reconnecting"` | R-N3 | 11 |
| `types.ts`: `Me.mode: "open" \| "cf-access" \| "worker"`; `TableView.bot`; `ClientState.bots`; the foreign refusal | | 2, 3, 4 |

### The Worker (`worker/`, `crates/yokohama-worker`)

| name | shape | task |
|---|---|---|
| `Env.YOKOHAMA_WEB_IDENTITY?: string` | `"worker" \| "open"`, no default | 17 |
| `ShellConfig.door: "worker" \| "open"` | | 17 |
| `Identity` + `{ kind: "claimed"; name: string }` | `worker/src/identity.ts` | 17 |
| the module's init config + `"identity": "worker" \| "open"` | | 17 |
| `worker/wrangler.open.test.jsonc`; a vitest workspace of two `cloudflareTest` projects | | 17 |
| `Outcome` (`r2.ts`) + `held?: string`; the module's receipt JSON + `"held"` | | 19 |
| `worker/scripts/dev-vars.mjs`: `readDevVars(path?: string): Record<string, string>`, with `dev-vars.d.ts` | | 23 |

---

## File structure

**Created**
- `crates/yokohama-web/src/bots.rs` (Task 3)
- `web/src/wire/production_bot.json` (Task 3), `identity_cases.json` (Task 1), `seed_cases.json` (Task 13): generated,
  committed
- `web/src/BoardMark.tsx`, `Band.tsx`, `Chips.tsx`, `SeatCard.tsx`, and their `*.test.tsx` (Task 9)
- `web/src/nameRule.ts`, `seedRule.ts`, `projectLink.ts`, and their tests (Tasks 11, 13, 10)
- `web/scripts/project-url.mjs` and its node test (Task 10)
- `worker/wrangler.open.test.jsonc` and the vitest workspace file (Task 17)
- `worker/src/delivery.ts`, and the archive half's module (Task 20; its name is Task 20's)
- `worker/scripts/dev-vars.mjs`, `dev-vars.d.ts` (Task 23)
- `.github/workflows/docker.yml` (Task 24)
- `deploy/README.md`, `worker/RUNBOOK.md`, `docs/web-lane.md` (Tasks 26–27)
- the close, dated the day it is written (Task 28)

**Modified** (principal files; each task's own list is the authority)
- The layer: `identity.rs`, `archive.rs`, `api.rs`, `games.rs`, `tables.rs`, `table.rs`, `client.rs`, `explain.rs`,
  `lib.rs`, `Cargo.toml`, `tests/wire_fixtures.rs`, `tests/play_a_whole_game.rs`
- The module: `crates/yokohama-worker/src/lib.rs`, `config.rs`, `receipt.rs`, `house.rs`, `picker.rs`,
  `Cargo.toml`; `crates/yokohama-wasm/src/lib.rs` (the host value)
- The shell: `worker/src/env.ts`, `config.ts`, `identity.ts`, `index.ts`, `admin.ts`, `table.ts`, `house.ts`,
  `archive.ts`, `r2.ts`, `storage.ts`; `worker/wrangler.jsonc`, `wrangler.test.jsonc`, `vitest.config.ts`;
  `worker/scripts/deploy-guard.mjs`, `live-replay.mjs`, `read-deploy.mjs`
- The client: `FrontDoor.tsx`, `OnThisDevice.tsx`, `BackupControls.tsx`, `Lobby.tsx`, `NewGame.tsx`,
  `OpenTable.tsx`, `LockScreen.tsx`, `Unlock.tsx`, `SignIn.tsx`, `Door.tsx`, `App.tsx`, `Chat.tsx`, `Help.tsx`,
  `whoami.ts`, `api.ts`, `signal.ts`, `ws.ts`, `types.ts`, `wire/contract.ts`, `styles.css`, `vite.config.ts`,
  `vite-env.d.ts`, `playwright.config.ts`, `e2e/*`
- The build and CI: `deploy/Dockerfile`, `Dockerfile.runtime`, `.github/workflows/test.yml`, root `Cargo.toml`,
  `Cargo.lock`, `web/package.json`, `worker/package.json` and their lock files
- The documents: `deploy/RUNBOOK.md`, `worker/README.md`, `README.md`, `CLAUDE.md`, `guidingpath.md`,
  `docs/handover.md`, and two frozen plans' single lines under the recorded exception (R-6.5c2.16)

---

## Task order and why

```
Phase A  1 ─► 2 ─► 3 ─► 4 ─► 5 ─► 6 ─► 7 ─► 8
Phase B  9 ─► 10 ─► 11 ─► 12 ─► 13 ─► 14 ─► 15 ─► 16
Phase C  17 ─► 18 ─► 19 ─► 20 ─► 21 ─► 22 ─► 23
Phase D  24 ─► 25
Phase E  26 ─► 27
Phase F  28 (ORCHESTRATOR + the close)
```

1. **Phase A is the layer, first, whole.** Every host consumes it, so every later task builds on the final layer and
   the final corpus.
   - 1 before 2: the key grammar exists before the mode rule narrows it.
   - 3 before 4: the label exists before the refusal names it.
   - 6 before Task 19: the `PutError` shape and the log seam before the Worker's half.
2. **Phase B is the client**, on the final wire. The shared pieces (9) come before the screens that use them.
3. **Phase C is the Worker.** The two doors (17, 18) and the refinish rule's shell half (19) land BEFORE the
   `table.ts` split (20), so the split moves final code once. The module's restatements (22) import Task 7's
   exports.
4. **Phase D:** CI and the Docker workflow (24) once the image's door and the gate's fix exist; the version (25)
   after every wire change, so the corpus regenerates for it once.
5. **Phase E, the documents, last** among the code-bearing tasks, so they describe the final state.
6. **Phase F:** the orchestrator's account steps, the full sequence, the battery, the review, the close, and on the
   owner's word the push, the merge and the tag.

---

<!-- TASKS: the task sections below are assembled from the drafters' outputs. -->
