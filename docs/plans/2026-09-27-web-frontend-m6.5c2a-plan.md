# M6.5c2 cycle A — the layer: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. The execution shape is §"Execution shape" below, which records the owner's standing answers: every
> task subagent-driven, an Opus implementer and an Opus reviewer per task, Fable for the whole-branch review at the
> cycle's close. Steps use checkbox (`- [ ]`) syntax for tracking. **Read the spec first** (revision 2.6, §4 is the
> cycle map); this plan argues from it and does not restate its reasons. **Read §"Couplings between tasks" before any
> task**: an implementer sees only its own task, and the couplings are what no single task section can carry.

**Goal:** The table layer (`crates/yokohama-web`) as M6.5c2 leaves it — the key grammar and the open door's mode
rule, the bot's label and the foreign-bot rule, `run_id` on every save, the log seam and the refinish rule's layer
half, the drift filter, the object-only body, the restatements exported, the whole-game gate's livelock fixed and
measured — with the corpus regenerated once and its diff read, and both host crates and the Worker's pool green at
every commit. Cycle B builds its screens on this wire; cycle C brings the Worker and the browser host to these exports.

**Architecture:** Change runs outward from the layer. Every host consumes it, so every later cycle stands on the
final layer and the final corpus. Inside this cycle the order is: the key grammar before the mode rule that narrows
it; the label before the refusal that names it; the `PutError` shape and the log seam before anything that reports
through them; the gate's fix last, alone, because it is the least-verified task. The two host crates
(`crates/yokohama-wasm`, `crates/yokohama-worker`) and the Worker's shell are edited only where the layer's change
breaks a cell or forces a literal — every such repair is listed in the task that causes it — and the module and the
browser host keep RESTATING the layer's items until cycle C switches them to the exports this cycle makes.

**Tech Stack:** Rust 1.97.1 (pinned), axum on the NAS host, `wasm32-unknown-unknown` + `wasm-bindgen 0.2.128` for the
two WebAssembly hosts (`wasm-opt` 133 on the workstation); the client's corpus and tests in `web/` (vitest, jsdom,
Playwright for the browser walk); the Worker's shell and pool in `worker/` (TypeScript, Cloudflare's Workers Vitest
integration over workerd).

**Spec:** `docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md` at **revision 2.6** — §2.1 (the server's two rules, the
rename, the archive's containment), §2.2 (the three reworded server sentences; "The seed"), §2.4 whole, §2.5 the layer
half, §2.6 whole, §3 the cells, §4 the cycle map, §5 the fences, §8 the risks, §10.5–§10.6 the corrections and the
record. The entry artifact is `docs/plans/2026-09-26-web-frontend-m6.5c2-handover.md`; its §6 lists the traps not to
re-derive.

**How this plan was built, and what is measured.** It is assembled from the retired single plan's drafts D1 (Tasks 1–2),
D2 (Tasks 3–5) and D3 (Tasks 6–8) — under the gitignored `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/drafts/`
— with the retired plan review's findings applied at their seams (spec §10.6's table; Appendix A below maps each to
its step). **What was measured:**
- D1 ran Tasks 1–2 end to end in an export: every red run, every count, both regenerations' diffs, three containment
  mutants. D2 ran Tasks 3–5 likewise (five mutants). Their file:line citations were read at `1e16fe27`, which is
  HEAD's code byte for byte.
- The retired review then applied **Tasks 1–7 in ONE tree and ran the pool at each step** — the first time the tasks
  had met. Its per-task commits are cycle A's ledger, `.superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/`:
  `review-chain-patches/0001…0008` (`git am`-able over `1e16fe27`), `review-chain.bundle` and `d1-trial.bundle`
  (whole trees), `d1-trial-tasks-01-02.diff`, and the spec review's `traversal_probe.rs`. **Every count below marked
  "chain" was read there**; every count is still stated as a DELTA over the implementer's own HEAD, and a figure is a
  reading, not a constant.
- **Not measured by anyone: Task 8.** D3 compiled nothing, and the review stopped at Task 7. Task 8's reviewer measures
  it first and hardest.
- **Not run by anyone: the e2e walks** (Tasks 2, 6 and 8 owe one each), the full `cargo test -p yokohama-cli` at a
  task's state, and the pool with P-4's fixture fix (its shape was measured green in the review's bisect; the exact
  count is the implementer's to read).

---

## Global Constraints

- **Branch `web-room-m6.5c2a`**, stacked on `web-room-m6.5c`; **the base for every code diff is `1e16fe27`** (the M6.5c
  close head). Every commit after it on this branch before Task 1 touches documents only (`docs/plans/`,
  `deploy/RUNBOOK.md`, `docs/handover.md`), so a file:line in this plan is a line at HEAD.
  - Stage with explicit paths, never `git add -A`. `git status` is clean after every commit.
  - **Nothing is pushed until the cycle's close (Task 9).** The push and PR #39 against `web-room-m6.5c` are
    **PRE-AUTHORIZED** by the owner (2026-09-27: *"push and PR are pre-approved at the orchestrator's discretion"*)
    and are the orchestrator's to make at the close, without asking again.
- **Fences** (spec §5). At every commit, this prints nothing:
  `git diff --stat 1e16fe27 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli`
  - **The CLI needs no change** (spec §5). If a task finds a seam it truly needs, it STOPS and asks the ORCHESTRATOR,
    who rules with a `Ruling:` line in the ledger. It is then a behaviour-preserving widening, named.
  - `SEMANTICS_REV` stays **7**. **No bot behaviour moves.** The workspace version stays `0.5.0` (the bump is cycle C's).
- **The layer changes only as the spec names them** (§2.1, §2.4, §2.5, §2.6, §2.2's three sentences, the seed table).
  Each task's reviewer reads `git diff 1e16fe27 -- crates/yokohama-web/src` against the task's own "reviewer reads" list.
- **The committed corpus** (`web/src/wire/*.json`, `web/src/wire-script/*`) is regenerated ONLY by the tasks that say so
  (1, 2, 3, 4, 5, 7), with `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` (the regeneration
  run exits non-zero BY DESIGN — the writer asserts the variable is unset, and three cells refuse a regeneration run;
  the check run that follows is the green one). **The diff is READ and quoted in the task's report.** A changed payload
  the task did not predict is a FINDING: stop and report it rather than commit it.
  - **The two replays' third columns** — `worker/test/corpus.test.ts`'s `THIRD_COLUMN` (compared AS WRITTEN against a
    sorted derivation) and `web/src/local/corpus.node.test.ts`'s `NOT_SERVED` (compared sorted) — name every committed
    file no script replays. **A task that adds a corpus file INSERTS its name into the list as it stands, in sorted
    place, and never pastes a literal over it** (the retired review's P-8: a pasted literal dropped an earlier task's
    name with every test green until the pool ran).
  - **A regeneration's ripple is not complete until `cd worker && npm run test:scripts` has run**:
    `worker/scripts/live-replay.test.mjs` restates corpus values (D1 measured one break at Task 2).
- **The M6.5c standing rules hold for all Worker code** (CLAUDE.md, "The Worker host — the standing rules"): the strings
  rule (the shell never `JSON.parse`s a record or a table document; a fixture edit that touches a `u64` is a TEXT edit,
  never a JS round trip), the two time bases, one batch per request, every client `x-yokohama-*` header stripped, and
  the pool facts (the M6.5c plan's "Pool facts").
- **Per-edit gates after EVERY task. Re-count at your own HEAD; a figure in any document is a reading, not a constant.
  Counts below are stated as DELTAS over your own HEAD's reading.**
  - **Rust:**
    - `cargo test -p yokohama-web`, `cargo test -p yokohama-wasm`, `cargo test -p yokohama-worker`;
    - `cargo clippy --workspace --all-targets -- -D warnings`;
    - `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`;
    - the public build's identity cells, `cargo test -p yokohama-web --no-default-features --features server --lib identity::`,
      whose count CI asserts (Task 1 moves it 5 → 7, Task 2 moves it 7 → 8; D1 measured both), and from Task 2 on that
      shape's own lint, `cargo clippy -p yokohama-web --no-default-features --features server --all-targets -- -D warnings`;
    - `cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm` and
      `cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker`;
    - `cargo doc -p yokohama-worker --no-deps` — **0** warnings at every task that edits that crate's docs;
    - `cargo test -p yokohama-cli` at any task whose diff reaches a `Cargo.toml` or `Cargo.lock` (Tasks 2 and 6; the CLI
      does not depend on the layer, so the run is the constraint's letter — set `YOKOHAMA_BGA_CORPUS_OPTIONAL=1` only if
      the two captures are absent, and say so in the report).
  - **In `web/`:** `npm run typecheck`, `npm test` and `npm run test:wasm` — the last needs `npm run build:wasm` first
    whenever Rust the browser module compiles changed (every task here but 8).
  - **In `worker/`:** **`npm run build:module:debug` FIRST** whenever `crates/yokohama-worker` or the layer changed
    (every task here but 8), or the pool tests a stale module GREEN. Then `npm run typecheck`, `npm run test:scripts`
    and `npm test` (the pool).
  - **`cd web && npm run e2e`** is owed by any task that changes what a browser DOES or SHOWS — here Tasks 2, 6 and 8,
    each saying why. Its run is SEQUENCED: never two Playwright runs at once. It takes 6.5–15 min; if the tool's
    ten-minute ceiling would cut it, run it in the background with its output to a file in the ledger and POLL that
    file until the summary line appears — never wait on a notification. The DEBUG module it builds is left in
    `web/src/local/wasm/` and `web/dist/` afterwards (CLAUDE.md), which nothing in this cycle reads.
- **The whole-game gate's vs-bot arm livelocks on some OS-entropy seeds until Task 8 fixes it**: a red gate at Tasks 1–7
  is re-run ALONE at the same commit before it is believed.
- **Task 2 follows Task 1 directly.** Between their commits the two edge modes admit a plain name; nothing is deployed
  between them.
- **The NAS is NOT redeployed from this branch.** After Task 2 no build accepts `YOKOHAMA_WEB_IDENTITY=claimed`, and the
  live container sets it (RUNBOOK §3). Cycle C redeploys it on `cf-access` at the close.
- **Any edit to a workflow file** runs `python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" <file>` in its
  gate (Tasks 1 and 2 edit `.github/workflows/test.yml`).
- **No step in this cycle touches the owner's account**: no deploy, no secret, no bucket, no live tier. Task 9's push
  and PR are the orchestrator's, pre-authorized.
- **Entry for the executing session.** Read, in order: `CLAUDE.md`; `docs/handover.md`'s "Updated" line; the spec's
  §4 and §10.6; this plan WHOLE; the ledger's `progress.md`. Confirm `git branch --show-current` prints
  `web-room-m6.5c2a`, `git status --short` prints nothing, and `git log --oneline -1` is at or after `f43a15ae`. Then
  Task 1 Step 0, which records the baseline every delta in this plan is read against.
- **Test data never uses the owner's surname** (the retired review's P-2; R-6.5c2.16): every test person here is
  `Ben Castle` in some spelling (`"  Ben   Castle "`, `"ben castle"`, `"BEN CASTLE"`, `"ben\tcastle"`, `"b3n_c-a.stle"`),
  and the plan's expected diffs are written to that. The hostnames under `moore.nyc` are real and stay. **The owner's
  email address goes into no document.**
- **The ledger** is `.superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/` (gitignored). `progress.md` holds one
  line per task and one `Ruling:` line per deviation from this plan. **Every agent writes its full report to a file
  there and replies with the PATH and three lines.**
- **Commit messages** quote the task's red runs in the body and end with the EXECUTING agent's own attribution lines.
- **`cargo test` takes ONE positional filter**: one command per cell, or a shared substring.
- **A reviewer's mutants run on an EXPORT** (`git archive HEAD | tar -x -C <dir>`), never in the tree an implementer
  holds; while an implementer works, nothing runs in its tree and no timer is armed.

---

## Review Focus

The spec implies these five inputs, and no cell would meet them unless someone put it there. Each line says what a
person would expect; its test lives in the owning task and is marked **(Review Focus N)** there.

1. **A name typed with capitals and extra spaces** (`"  Ben   Castle "`). Expected: ONE key, `ben castle`, and the same
   person typing `ben castle` later reaches the same seats; the typed form is kept for the screen. The corpus row the
   client's name step will be pinned to carries both columns. (Task 1)
2. **A name that could be a path step, on every host** — `..`, `.`, `. .`, `-`, `.x` — and, past the grammar, an
   ABSOLUTE archive prefix. Expected: refused as a key; refused by the layout; refused by the directory archive with
   nothing written, on the NAS and through the Worker's receipt alike. The review MEASURED the hole an unconstrained
   `..` opened. (Task 1)
3. **A spectator (seated nowhere) opening a finished game played against a foreign bot.** Expected: the Players panel
   says the bot with its label, `why` names the bot's version rather than "a human's move", undo is the not-seated
   refusal, and nothing a finished game shows a watcher today is refused. (Task 4)
4. **An owed game-over upload that was UNDONE during its upload, and then meets a deploy with a new bot.** Expected:
   bounded, not stranded — the retry refuses (foreign, not over), and the creator's Abandon, which never rebuilds,
   archives and clears it. (Task 4)
5. **Seed text the server accepts that a naive client check would refuse, and the reverse**: `"+007 "`, `" 7"`, `"007"`
   accepted; `"７"`, `"1e3"`, `"-1"`, `"18446744073709551616"` refused; and the two inputs on which JavaScript's `trim`
   and Rust's `str::trim` part ways (`U+0085`, `U+FEFF`). Expected: the committed table says exactly what the server
   does, so cycle B's form cannot agree with itself and disagree with the server. (Task 7, the emitter and its cell)

---

## Execution shape (the owner's standing answers)

- **Mode: SUBAGENT-DRIVEN, EVERY TASK** (the owner, 2026-09-21, standing; R-6.5c2.18). Each task gets a fresh **Opus**
  implementer and a fresh **Opus** task reviewer; **Fable** does the whole-branch review in Task 9. Folding two small
  tasks into one execution unit is the orchestrator's call (spec §9); each task keeps its own green commit either way.
- Tests are written first and RUN RED; the red run is quoted in the commit body and in the ledger. Every commit passes
  every per-edit gate.
- Standing process rules: never follow up an implementer's DONE without reading HEAD first; a reviewed SHA is never
  amended — a fix is a NEW commit, and the acceptance reply that asks for it says so; while an implementer works, run
  nothing in its tree and arm no timers; wake a stalled agent by message rather than spawning another; a reviewer's
  mutants run on an export.
- **The orchestrator edits no code.** Its steps here are Task 9's (the review dispatches, the push and the PR —
  pre-authorized — the handover into cycle B, the board and the memory).

---

## Names shared across tasks

An implementer sees only its own task. **These names are the contract between tasks and are used verbatim.** A task
that finds the code forces a different shape STOPS and asks the ORCHESTRATOR, who amends this table with a `Ruling:`
line. It never renames on its own.

### The layer (`crates/yokohama-web`)

| name | shape | task |
|---|---|---|
| `User::parse(raw: &str) -> Result<User, String>` | the KEY grammar: today's email shape **or** a plain name — 1–32 ASCII letters, digits, space, `.`, `-`, `_`; **begins with a letter or a digit**; internal space runs collapse; `to_ascii_lowercase`; the literal `anonymous` refused | 1 |
| `User::typed_form(raw: &str) -> String` | `raw` trimmed, each run of internal U+0020 collapsed to one, case KEPT; `parse` is built on it, so on every accepted input `key == typed_form.to_ascii_lowercase()` | 1 |
| `User::is_email(&self) -> bool` | whether the key has the email shape (the `@`) | 1 |
| `archive::paths(rev, prefix, why, id) -> Result<(String, String), String>` | refuses any `.`/`..` component | 1 |
| `DirArchive::put` | + a LEXICAL check: **every component of the RELATIVE path `paths` built is `Component::Normal`**, checked before the join (an absolute path REPLACES the base and has no `ParentDir`; D1's mutant proved the weaker wording wrong) | 1 |
| `web/src/wire/identity_cases.json` | `[{"display": str \| null, "key": str \| null, "raw": str}]` (serde sorts the keys), 41 rows, the layer's verdicts; emitted and refused like `site_space_words.json` | 1 |
| `IdentityMode::Open` | replaces `Claimed`; the variable's value `open`; its wire name `open`; the header stays `X-Yokohama-Claimed-User` | 2 |
| cargo feature `open-door` | replaces `dev-identity`; still a default feature | 2 |
| `identity::resolve(mode, headers)` | + **the mode rule**: under `CfAccess`/`Worker` a non-email reads as ABSENT (the no-header sentence) | 2 |
| `identity::admit(mode: IdentityMode, user: User) -> Result<User, String>` | the mode rule in ONE function; `resolve` calls it, and so does the module's `identity_of` fallback (cycle C passes its configured mode) | 2 |
| `bots::BotDescriptor { controller: String, version: u32, params: BTreeMap<String, i64> }` | `Serialize, Deserialize, Clone, PartialEq, Eq, Debug`; a `SeatProvenance`'s triple | 3 |
| `bots::production() -> BotDescriptor` | `PROVISIONAL_DEFAULT_BOT`'s descriptor, through `Contestant::descriptor` | 3 |
| `bots::label(d: &BotDescriptor) -> String` | through `Contestant::from_parts` and `label_of`, an exhaustive match with NO `_` arm at either level; an unparseable descriptor answers its `controller` | 3 |
| `bots::recorded(record: &GameRecord) -> Vec<(u8, BotDescriptor)>` | the record's non-human seats, in seat order | 3 |
| `bots::TableBot { label: String, runs_here: bool }`, `bots::SeatBot { seat: u8, label: String }` | the two wire types | 3 |
| `bots::table_bot(doc: &TableDoc) -> Option<TableBot>`, `bots::seat_bots(record) -> Vec<SeatBot>` | `pub(crate)` | 3 |
| `TurnMirror.bot: Option<BotDescriptor>` | `#[serde(default, skip_serializing_if = "Option::is_none")]`; `TurnMirror` loses `Copy` (SIX by-value sites move) | 3 |
| `TableView.bot: Option<TableBot>`; `ClientState.bots: Vec<SeatBot>` | wire | 3 |
| `web/src/wire/production_bot.json` | `{"identity": "v4-step9-draft1", "label": "V4"}`, emitted by `wire_fixtures`, refused when stale; IMPORTED by cycle B's screens directly (never through `contract.ts`) | 3 |
| `web/src/wire/table_bot.json` | the corpus's one `TableView` whose `bot` is not null (an undealt vs-bot table) | 3 |
| `bots::foreign(record) -> Vec<(u8, BotDescriptor)>`, `bots::ForeignBot { played: String, runs: String }` (`Serialize`), `bots::cannot_continue(played, runs) -> String` | the rule's three items | 4 |
| `GameError::ForeignBot { played: String, runs: String }` | → **409** `{"error": "This game was played against V2. This server now runs V4 and can't continue it.", "foreign_bot": {"played": "V2", "runs": "V4"}}` | 4 |
| `ApiError.foreign_bot: Option<bots::ForeignBot>` | the 409's second key; every `ApiError { … }` literal gains `foreign_bot: None` (24 sites in two crates at the base) | 4 |
| `explain::Why::foreign(played: &str) -> Why`; `explain::FOREIGN_BOT_KIND = "foreign_bot"`; `explain::foreign_why(record, rules, event) -> Option<Why>` | `why`'s THIRTEENTH refusal shape, the layer's own, outside `kind_of`'s fence | 4 |
| `client::without_foreign_conversion(warnings: Vec<String>, record: &GameRecord) -> Vec<String>` | `pub(crate)`; **Task 7's `without_version_drift` WRAPS it at the same `warnings:` line** | 4 |
| `games::test_support::{PLAYED_RECORD, PLAYED_TABLE, PLAYED_ID, BOT_TO_MOVE_AT, foreign_pair}` | test-only | 4 |
| `web/src/wire/foreign_bot_409.json`, `table_foreign.json`, `why_foreign.json` | the client's three sources for the refusal, the row that does not run here, and `why`'s shape | 4 |
| `games::RunHost::{Server, Worker, Browser}`; `RunHost::run_id(&self) -> &'static str` | `"web/server"`, `"web/worker"`, `"web/browser"`; **`Games::with_parts`'s LAST parameter** | 5 |
| `log::{Log, NoLog, StderrLog, render}` and the five events `OVER_PUT_REFUSED`, `OVER_UNSTAMPED`, `OVER_TABLE_GONE`, `KEPT_EARLIER_COMPLETION`, `LISTING_SKIPPED` | `pub trait Log: Send + Sync { fn line(&self, event: &'static str, fields: serde_json::Value); }`; every field value a STRING; `StderrLog` under `server` | 6 |
| `Games::with_log(self, Arc<dyn Log>) -> Self` | a builder step, never a `with_parts` argument; `NoLog` default | 6 |
| `PutError::Conflict { key: String, held: Option<String> }` | the record already at the key, when the archive can read it (`DirArchive` always; the receipt `None` until cycle C) | 6 |
| `games::test_support::{RecordingLog, RecordingArchive::conflicting, PutAnswer}` | test-only | 6 |
| `client::without_version_drift(warnings: Vec<String>) -> Vec<String>` | drops only lines beginning `warning: GameRecord.engine_version=` | 7 |
| `api::error_body(e: &ApiError) -> String`, `api::error_value(e: &ApiError) -> serde_json::Value` (`pub(crate)`) | the body `dispatch` writes for a refusal — **all three arms**: `error`, `status`, `foreign_bot` | 7 |
| `api::MAX_WAIT_SECONDS: u64 = 25`, `api::check_wait(wait: Option<u64>) -> Result<(), ApiError>` | the cap and its sentence, out of `lib.rs`'s server half | 7 |
| `api::NOT_AN_OBJECT: &str = "the request body must be a JSON object"` | the object-only body's refusal | 7 |
| `signal::house_frame(seq: u64, chat: u64) -> String` | `{"seq":…,"chat":…}` | 7 |
| `games::HostClock` | `const fn new()`, `set(&self, Option<u64>) -> Option<u64>`, `now(&self, live: impl FnOnce() -> u64) -> u64`, `tick(&self) -> u64` | 7 |
| `platform::CALLS: [&str; 8]`, `platform::calls_in(file: &str, text: &str) -> Vec<String>` | the eight patterns stated once (each in two `concat!` halves), and the hosts' scan over one file's text | 7 |
| `web/src/wire/seed_cases.json` | `[{"ok": bool, "raw": str}]`, 26 rows, `WireSeed`'s verdicts; emitted and refused like `site_space_words.json` | 7 |
| `YOKOHAMA_WEB_GATE_BOT_SEED` | a TEST-ONLY variable read by `play_a_whole_game.rs`'s vs-bot arm, pinning the bot's stream seed | 8 |

### The Worker module and shell (`crates/yokohama-worker`, `worker/`), as this cycle touches them

| name | shape | task |
|---|---|---|
| the module's `identity_of` fallback | `User::parse(raw).and_then(|u| admit(IdentityMode::Worker, u))` — cycle C replaces the literal mode with the configured door | 2 |
| the module's restated `error_body` | gains the `foreign_bot` arm (the module answers some refusals itself; cycle C deletes the restatement for `api::error_body`) | 4 |
| `crates/yokohama-worker/src/log.rs`: `ShellLog`, `LogLine { event: &'static str, fields: Value }`; `receipt::DEFERRED` at `pub(crate)` | the module's `Log`: records every line but the one deferral it drops | 6 |
| `ReplyData.logs: Vec<LogLine>`; the getter `Reply.logs` → JSON `[{"event", "fields"}]`; `readReply` (`worker/src/module.ts`) logs each line through `respond.ts`'s `log` | every `ReplyData { … }` literal carries `logs` (three at the base) | 6 |
| `worker/test/fixtures/owed-game-over.table.json`'s mirror gains `bot` | the record's own descriptor, copied AS TEXT | 3 |

### The client (`web/src`), as this cycle touches it

| name | shape | task |
|---|---|---|
| `contract.ts`: `export interface IdentityCase { raw: string; key: string \| null; display: string \| null }`, `wire.identityCases: readonly IdentityCase[]` | what cycle B's `nameRule.test.ts` imports | 1 |
| `types.ts`: `Me.mode: "open" \| "cf-access" \| "worker"`; the four `claimed` literal sites renamed | | 2 |
| `types.ts`: `SeatBot`, `TableBot`, `ProductionBot`; `ClientState.bots`; `TableView.bot`; `wire.tableBot`, `wire.productionBot`; `KEYS.tableBot`, `KEYS.seatBot`, `KEYS.productionBot` | | 3 |
| `types.ts`: `ForeignBot`, `ForeignBotErrorBody`; `wire.foreignBot409`, `wire.tableForeign`, `wire.whyForeign`; `KEYS.foreignBotError`, `KEYS.foreignBot` | | 4 |
| `contract.ts`: `wire.seedCases: readonly {raw: string; ok: boolean}[]` | what cycle B's `seedRule.test.ts` imports | 7 |
| `web/e2e/harness.ts`: `capReached(page, game, moves): Promise<Error>`, `capSentence(id, moves, strip, chain): string`, `Screen.pressed: number`, `Screen.single: boolean`, `Affordance` + `"cancel"` | the browser walk's cap report and probe | 8 |

**What cycles B and C consume** is exactly the rows above; spec §4 lists them by name, and each later plan READS them
off this branch's tree rather than off this table.

---

## File structure

**Created**
- `crates/yokohama-web/src/bots.rs` (Task 3), `log.rs` (Task 6), `platform.rs` (Task 7)
- `crates/yokohama-worker/src/log.rs` (Task 6)
- `web/src/wire/identity_cases.json` (Task 1); `production_bot.json`, `table_bot.json` (Task 3); `foreign_bot_409.json`,
  `table_foreign.json`, `why_foreign.json` (Task 4); `seed_cases.json` (Task 7): all generated and committed
- `web/src/local/log.node.test.ts` (Task 6)
- in the ledger (gitignored): `task-8-gate-50.txt`, `task-8-e2e.txt` (Task 8)

**Modified** (principal files; each task's own list is the authority)
- The layer: `identity.rs`, `archive.rs`, `api.rs`, `games.rs`, `tables.rs`, `table.rs`, `chat.rs`, `client.rs`,
  `explain.rs`, `signal.rs`, `lib.rs`, `main.rs`, `Cargo.toml`, `tests/wire_fixtures.rs`, `tests/browser_played_pin.rs`,
  `tests/play_a_whole_game.rs`
- The Worker module: `crates/yokohama-worker/src/lib.rs`, `receipt.rs`, `config.rs`, `events.rs`, `create.rs`,
  `house.rs`; the shell: `worker/src/module.ts`; the pool: `worker/test/corpus.test.ts`, `worker.test.ts`,
  `sockets.test.ts`, `factory.test.ts`, `fixtures/owed-game-over.table.json`; the scripts: `worker/scripts/live-replay.test.mjs`
- The browser host: `crates/yokohama-wasm/src/lib.rs`, `Cargo.toml`
- The client: `web/src/types.ts`, `wire/contract.ts`, `wire/contract.test.ts`, `local/corpus.node.test.ts`, the four
  `claimed` literal sites (`App.tsx`, `Door.tsx`, `OnThisDevice.tsx`, and their tests), comments in ten more files;
  `web/playwright.config.ts`; `web/e2e/harness.ts`, `harness.node.test.ts`, five specs (Task 8)
- CI and documents: `.github/workflows/test.yml` (Tasks 1, 2); `deploy/Dockerfile`, `deploy/Dockerfile.runtime`
  (comments, Task 2); `CLAUDE.md` (two sentences, Task 6)

---

## Task order and why

```
1 ─► 2 ─► 3 ─► 4 ─► 5 ─► 6 ─► 7 ─► 8 ─► 9 (the close, ORCHESTRATOR)
```

1. **1 before 2:** the key grammar exists before the mode rule narrows it, and Task 2 follows directly.
2. **3 before 4:** the label exists before the refusal names it.
3. **4 before 7:** Task 7 wraps the `warnings:` expression Task 4 leaves, and its exported `error_value` carries the
   arm Task 4 adds.
4. **6 before 7:** the log seam and `PutError`'s shape exist before the exports and the `NotFound` cell that read them.
5. **8 last, alone:** the least-verified task, owing a 50-run measurement and an e2e; nothing later depends on it.
6. **9:** the orchestrator's close of the cycle.

---

## Couplings between tasks (binding; the later task's implementer does what its item says)

- **Tasks 4 → 7, the `warnings:` line** (`client.rs`, today `:498`). Task 4 makes it
  `without_foreign_conversion(session.load_warnings.clone(), session.record())`. Task 7 WRAPS that call:
  `without_version_drift(without_foreign_conversion(…))`. It never replaces the call.
- **Tasks 4 → 7, the error body's third arm.** Task 4 gives the body a `"foreign_bot"` arm in THREE places: the layer's
  `error_response`, `IntoResponse for ApiError`, and the module's restated `error_body`, and extends the module's pin
  `a_refusal_body_is_the_one_dispatch_writes`. **Task 7's exported `error_value` must carry all three arms** (the
  retired review's P-3: the first export dropped the arm and reddened one lib cell and six `wire_fixtures` cells), and
  its `error_body_is_the_body_dispatch_writes` cell has a third shape for it.
- **Tasks 1 → 3 → 4 → 7, the replays' third columns.** Each task INSERTS its names into `THIRD_COLUMN` and `NOT_SERVED`
  as they stand, in sorted place: Task 1 `identity_cases`; Task 3 `production_bot`, `table_bot`; Task 4
  `foreign_bot_409`, `table_foreign`, `why_foreign`; Task 7 `seed_cases`. The literals printed in Tasks 3 and 4 already
  include Task 1's name; the one in Task 7 includes all of them.
- **Tasks 1 and 2 break cells in both host crates.** Each task lists and repairs them in its own Step 1, so the red run
  shows only what is new.
- **Task 3 → the pool.** `TurnMirror.bot` makes `mirror_of` write the record's bot, so a rebuild's pump that ends the
  owed fixture game rewrites the table and owes the house a copy — two pool cells' premises break unless the fixture's
  mirror already carries the descriptor (P-4). Task 3 edits `worker/test/fixtures/owed-game-over.table.json` as TEXT.
- **Task 5 → the pool.** The pool's corpus cell compares the Worker's own game-over save with the axum host's committed
  record; `run_id` now differs by host (P-5). Task 5 edits `worker/test/corpus.test.ts` to compare through
  `replaceOnce`.
- **Task 6 → the pool's fakes.** `readReply` parses `logs`; every hand-built `Reply` in the pool gains `logs: "[]"` (P-6).
- **Task 6 → cycle C.** Every first game-over put on the Worker is DEFERRED, so the module's `ShellLog` drops that one
  line; cycle C's refinish cells on the Worker will meet `archive-kept-earlier-completion` in their `archiveLines`.
- **Task 2 → cycle C.** `identity_of`'s fallback calls `identity::admit(IdentityMode::Worker, …)`; cycle C threads the
  configured door through both arms and still calls `admit`, so the mode rule keeps one site.
- **Task 5 → cycle C's close.** The regenerated `archive-walk.json` carries `run_id`, so the live replay's reset recipe
  is owed at the close (the version bump owes it again for the same keys; one reset serves both).
- **Task 7 → cycle C.** The module and the browser host keep their restatements (`error_body`, `MAX_WAIT_SECONDS`, the
  house frame, the clock, the platform scan); Task 7 repoints only the two cells its exports break (`house.rs`'s frame
  cell, `lib.rs`'s array-create cell) and swaps `create.rs`'s host-written sentence for `api::NOT_AN_OBJECT`. Cycle C
  switches the rest.

---

### Task 1: The key grammar and the archive's containment (spec §2.1 "The server, as two rules" rule 1 and "The archive's containment is restated, not assumed"; §3's identity cells; R-6.5c2.16's test literal; Review Focus 1 and 2)

**Measured:** D1 ran every step of this task in an export; the retired review's chain carries it as `0001`. The one
change from that text is the test person's name (P-2): `Castle` throughout, so the emitted rows, the cells and the
expected diff below agree with each other.

**Files:**
- Modify: `crates/yokohama-web/src/identity.rs`:
  - `:1` — the module's first line says "key", not "email";
  - `:114-142` — `User`'s doc; `User::parse` (`:122-141`) widened into the KEY grammar; new `User::typed_form` and
    `User::is_email` (public), `User::email` (private, today's email rule moved verbatim), `NAME_MAX` and `NAME_RULE`;
  - `:215-223` — the email cell is REWRITTEN; two new cells beside it.
- Modify: `crates/yokohama-web/src/archive.rs`:
  - `:115-122` — `paths` returns `Result`, refusing a `.` or `..` component;
  - `:124-133` — `caller_segment`'s containment argument restated for two shapes of key (the `**It COLLIDES…**`
    paragraph at `:135-138` stays);
  - `:197` — `use std::path::{Component, Path, PathBuf};`; `:254` — `DirArchive::put` takes `paths`' `Result` and adds
    the lexical check;
  - tests: `:324-350` REWRITTEN (the owner's address at `:336` replaced by `red@example.test`, R-6.5c2.16); the
    `paths` call sites `:364-374`, `:384`, `:387`, `:413`, `:431`, `:444-451`, `:512` take the `Result`; three new
    cells.
- Modify: `crates/yokohama-web/src/api.rs:1936` — the archive route maps `paths`' refusal to the put's own 500.
- Modify: `crates/yokohama-web/src/tables.rs` — `:113` `SeatSpec`'s doc; `:250-253` the create's comment; `:398-401`
  the watcher-list comment; `:406` the watcher-list sentence; `:2748` the unparsable-watcher fixture; one new cell
  before `:4945`.
- Modify: `crates/yokohama-web/src/chat.rs` — `:291` the author sentence; `:427-430` the bad-user fixture, plus a
  far-side arm.
- Modify: `crates/yokohama-web/src/lib.rs:1607-1617` — the `/api/me` cell's invalid header, plus an open-door arm.
- Modify: `crates/yokohama-web/src/table.rs:33-35`, `:200-201` — two docs say "key".
- Modify: `crates/yokohama-web/src/games.rs:1928` — `who`'s `expect` says "a key".
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs` — `corpus()` pushes the new contract after `:2149`;
  `fn identity_cases()` and its cell directly before `:2299`.
- Create (generated, committed): `web/src/wire/identity_cases.json`.
- Modify: `web/src/wire/contract.ts` — import after `:106`; `IdentityCase` and `_identityCases` after `:374`; the
  `void` list `:493`; `wire.identityCases` after `:838`.
- Modify: `web/src/local/corpus.node.test.ts` — `:126-128` (doc) and `:143` (`NOT_SERVED` gains `"identity_cases"`).
- Modify: `worker/test/corpus.test.ts` — `:46` (doc) and `:59` (`THIRD_COLUMN` gains `"identity_cases"`, sorted).
- Modify: `crates/yokohama-wasm/src/lib.rs:285-290` — the malformed-caller arm; one new cell before `:308`.
- Modify: `crates/yokohama-worker/src/receipt.rs` — `:200` `paths`' refusal as `PutError::Failed`; `:1180-1183`
  the cell's doc; `:1205-1206` the refused caller and an anti-vacuity arm; one new cell before `:1180`.
- Modify: `crates/yokohama-worker/src/config.rs` — `:59-60` (doc) and `:95` (the refusal).
- Modify: `crates/yokohama-worker/src/lib.rs` — one new cell before `:699`.
- Modify: `.github/workflows/test.yml:226-228`, `:246`, `:256-257` — the refusing build's count, 5 → 7.
- Test: `identity::tests` (3 cells), `archive::tests` (1 rewritten, 3 new), `tables::tests` (1 new, 1 repaired),
  `chat::tests` (1 repaired), `tests::me_answers_without_identity_and_names_the_mode` (repaired), the three hosts'
  `a_table_whose_watcher_list_holds_a_plain_name_reads`, `receipt::tests` (1 new, 1 repaired), the wasm crate's
  401 cell (repaired), `wire_fixtures::the_identity_cases_carry_the_key_grammar_s_verdicts`.

**Interfaces:**
- Consumes: nothing new.
- Produces: `User::parse` (the KEY grammar), `User::is_email`, `User::typed_form`; `archive::paths -> Result`;
  `DirArchive::put`'s lexical check; `web/src/wire/identity_cases.json` (41 rows); `contract.ts`'s `IdentityCase` and
  `wire.identityCases`.
- **The mode rule is Task 2's**: at THIS commit `resolve` admits every key under every mode.

- [ ] **Step 0: The baseline** (spec §3: "The first task re-counts every suite at the branch's HEAD and records the
  reading"). Before any edit, at HEAD, run every per-edit gate in Global Constraints and record each count in the
  ledger's `progress.md` under "Readings", machine-tagged, with the commit: `cargo test -p yokohama-web` (per binary:
  lib, `browser_played_pin`, `play_a_whole_game`, `wire_fixtures`), `-p yokohama-wasm`, `-p yokohama-worker`, the
  public build's `identity::` (5 at the base), `web/`'s `npm test`, `npm run test:wasm` (after `npm run build:wasm`),
  `worker/`'s `npm run test:scripts` and `npm test` (after `npm run build:module:debug`); clippy and the wasm32
  builds green; the fence printing nothing. **Every count in this plan is a delta over these numbers.** A whole-game
  gate that goes red here is re-run ALONE before it is believed (the livelock Task 8 fixes). CLAUDE.md's readings at
  the M6.5c close (`754b80ae`) are the expected values: web 293 (278 + 2 + 2 + 11), wasm 5, worker 59, `npm test`
  1056 in 56 files, `test:wasm` 92 in 6, `test:scripts` 352 in 18, the pool 195 in 9. A different figure is not a
  finding by itself — it is the baseline — but a figure BELOW CLAUDE.md's is reported before Task 1 begins.

- [ ] **Step 1: Write the failing cells for the key grammar**, and repoint the five existing cells the widening
  would break. The repairs go in NOW, so the red run below shows only what is new.

  **(a)** `crates/yokohama-web/src/identity.rs` — replace the cell at `:215-223` with these three; the third is
  **(Review Focus 1)**:

```rust
    /// **The key grammar** (M6.5c2, spec §2.1 rule 1): a key is an email OR a
    /// plain name, and one rule normalizes both — trim, collapse each run of
    /// internal spaces to one, lowercase by ASCII rules. Both families' bounds
    /// are asserted from both sides, so a grammar that refused everything, or
    /// took everything, fails here rather than in a lobby.
    #[test]
    fn a_key_is_an_email_or_a_plain_name_normalized_by_one_rule() {
        for (raw, key) in [
            ("  A@Example.COM ", "a@example.com"),
            ("a+b@x.test", "a+b@x.test"),
            ("  Ben   Castle ", "ben castle"),
            ("BEN", "ben"),
            ("x", "x"),
            ("7 wonders", "7 wonders"),
            ("r2-d2_x.y", "r2-d2_x.y"),
            ("a.test", "a.test"),
            ("ben\t", "ben"),
        ] {
            assert_eq!(User::parse(raw).map(|u| u.as_str().to_string()), Ok(key.to_string()), "{raw:?}");
        }
        for bad in [
            "", "   ", "two@@signs", "a@b c", "@example.com", "a@", "a\u{7}@b.c",
            "Bén", "ben\tcastle", "a/b", "ben!",
        ] {
            assert!(User::parse(bad).is_err(), "{bad:?} must be refused");
        }
        // The two length bounds, each from both sides.
        let email = |n: usize| format!("{}@x.y", "a".repeat(n - 4));
        assert!(User::parse(&email(254)).is_ok(), "254 bytes is an email");
        assert!(User::parse(&email(255)).is_err(), "255 bytes is one too many");
        assert!(User::parse(&"n".repeat(32)).is_ok(), "32 characters is a name");
        assert!(User::parse(&"n".repeat(33)).is_err(), "33 is one too many");
    }

    /// **The names that would name a path step, or nobody, are never keys**
    /// (M6.5c2, spec §2.1; review H1, MEASURED: an unconstrained `..` became
    /// the archive segment `..` and landed a browser's save in the server's
    /// own `rev7/over/`). A plain name must BEGIN with a letter or a digit, and
    /// the literal `anonymous` — how an unprotected edge says "nobody" — is
    /// refused in any case. The accepted half is the anti-vacuity: a dot, a
    /// dash or the word itself later in a name is fine.
    #[test]
    fn a_name_that_could_be_a_path_step_or_nobody_is_never_a_key() {
        for bad in [".", "..", ". .", "-", ".x", "_x", "-x", " .. ", "anonymous", "Anonymous", " ANONYMOUS "] {
            assert!(User::parse(bad).is_err(), "{bad:?} must never be a key");
        }
        for good in ["x.", "a..b", "z-", "anonymous1", "anon ymous", "anonymous@x.test"] {
            assert!(User::parse(good).is_ok(), "{good:?} is a key");
        }
    }

    /// **(Review Focus 1)** A name typed with capitals and extra spaces is ONE
    /// key, and the same person typing it plainly later is the same key — so
    /// they reach the same seats. The typed form keeps the case, for the screen
    /// to show; the key is that form lowercased and nothing else.
    /// [`User::is_email`] says which of the two shapes a key has.
    #[test]
    fn a_name_typed_with_capitals_and_extra_spaces_is_one_key_shown_as_typed() {
        let typed = User::parse("  Ben   Castle ").unwrap();
        assert_eq!(typed.as_str(), "ben castle");
        assert_eq!(User::parse("ben castle").unwrap(), typed, "the same person, typed plainly");
        assert_eq!(User::parse("BEN CASTLE").unwrap(), typed);
        assert_eq!(User::typed_form("  Ben   Castle "), "Ben Castle", "shown as typed, tidied");
        assert_eq!(User::typed_form("  Red@Wire.TEST "), "Red@Wire.TEST");
        assert!(!typed.is_email());
        assert!(User::parse("Red@Wire.TEST").unwrap().is_email());
        assert!(!User::parse("a.test").unwrap().is_email(), "a dot is not an address");
    }
```

  **(b)** `crates/yokohama-web/src/archive.rs` — replace `a_caller_segment_is_path_safe_and_readable` (`:324-350`,
  doc included). The example at `:336` spelled the owner's address; it becomes `red@example.test` (R-6.5c2.16).
  **(Review Focus 2)**, the grammar's half:

```rust
    /// R-6.5b.15, restated for M6.5c2's key grammar (spec §2.1; review H1): a
    /// caller's key rendered into ONE path segment that is path-safe and still
    /// readable — the owner reviews saves by who sent them, and a hash would
    /// hide that.
    ///
    /// **The containment property is arithmetic, with one arm per shape of
    /// key.** An email's local part and domain are each non-empty and `@`
    /// becomes four characters, so its segment is at least six characters of
    /// `[a-z0-9._-]`. A plain name begins with a letter or a digit, which the
    /// segment keeps, so its segment begins with `[a-z0-9]`. Neither holds a
    /// `/`, and the cap can only shorten a segment that is already non-empty —
    /// so no segment is empty, `.` or `..`. The names that WOULD break the
    /// second arm (`.`, `..`, `. .`, `-`, `.x`) are fed to the grammar and
    /// must be refused there.
    #[test]
    fn a_caller_segment_is_path_safe_and_readable() {
        use crate::identity::User;
        let seg = |s: &str| caller_segment(&User::parse(s).unwrap());
        assert_eq!(seg("red@example.test"), "red_at_example.test");
        assert_eq!(seg("A/B@c"), "a_b_at_c", "a slash is replaced, the address lowercased");
        assert_eq!(seg("../../x@y"), ".._.._x_at_y");
        assert_eq!(seg("..@.."), ".._at_..");
        assert_eq!(seg("  Ben   Castle "), "ben_castle", "a name's one space becomes `_`");
        let long = format!("{}@{}", "a".repeat(200), "b".repeat(50));
        assert_eq!(seg(&long).len(), 64, "capped");
        for name in [".", "..", ". .", "-", ".x"] {
            assert!(User::parse(name).is_err(), "{name:?} must never be a key");
        }
        let n32 = "n".repeat(32);
        for name in ["ben castle", "7", "x", "a.test", "z..", "r2-d2_x.y", n32.as_str()] {
            let v = seg(name);
            assert!(v.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit()), "{name:?} → {v:?}");
        }
        for s in ["../../x@y", "..@..", long.as_str(), "a/b@c", "ben castle", "z..", "7"] {
            let v = seg(s);
            assert!(!v.is_empty() && v != "." && v != ".." && !v.contains('/'), "{v:?}");
            assert!(
                v.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')),
                "{v:?}"
            );
        }
    }
```

  **(c)** `crates/yokohama-web/src/tables.rs` — a new cell directly before the doc at `:4945` ("A seat holder never
  needs the password…"):

```rust
    /// **A plain name on the watcher list reads, and is one key however it is
    /// typed** (M6.5c2, spec §2.1 and §3; review H2). The key grammar is
    /// `check_shape`'s too, and it runs on every read — so a grammar the check
    /// did not share would take the whole table off the lobby (the arm
    /// `every_shape_arm_refuses_with_the_sentence_it_promises` pins). The name
    /// is written by the layer's own `unlock`, read back and listed.
    #[test]
    fn a_table_whose_watcher_list_holds_a_plain_name_reads() {
        let playing = a_locked_game("named");
        assert!(playing.unlock("named", &who("  Ben   Castle "), "sesame").expect("the password").can_watch);
        let (doc, _) = playing.read_table("named").expect("it reads").expect("it is there");
        assert_eq!(doc.unlocked, vec!["ben castle".to_string()], "the KEY, normalized once");
        assert!(doc.can_watch(&who("BEN CASTLE")), "the same key, typed otherwise");
        let lobby = playing.list_for(&who("red@t.test")).expect("the lobby");
        assert!(lobby.iter().any(|row| row.id == "named"), "listed");
    }
```

  and in `every_shape_arm_refuses_with_the_sentence_it_promises`, `:2748`: `"Not An Address"` is a valid plain name
  now, so the fixture becomes the H1 shape (still refused, and the pinned fragment `"watcher list"` stays in the new
  sentence):

```rust
            d.unlocked = vec!["..".to_string()];
```

  **(d)** `crates/yokohama-web/src/chat.rs`, `malformed_fields_are_refused_but_length_is_not` — `:427-430` becomes a
  refused key plus the far side (a watcher an open door writes under a name must still be an author the document
  reads, or that table's chat is refused on read):

```rust
        let mut bad_user = doc(vec![seat(1, 0, "a")]);
        bad_user.messages[0].author =
            Author::Seat { seat: 0, user: "..".into(), name: None };
        assert!(check_shape(&bad_user, Some(2)).is_err());
        // The far side (M6.5c2): a plain name is a key, so a watcher an open
        // door wrote under one is an author this document still reads.
        let mut named = doc(vec![seat(1, 0, "a")]);
        named.messages[0].author = Author::Watcher { user: "ben castle".into(), name: None };
        assert_eq!(check_shape(&named, Some(2)), Ok(()), "a plain-name watcher is an author");
```

  **(e)** `crates/yokohama-web/src/lib.rs`, `me_answers_without_identity_and_names_the_mode` — `:1607-1617`: the
  invalid header becomes one the grammar refuses, and an open-door name joins:

```rust
        let (_, body) = get_with(
            Arc::new(AppState::in_memory()),
            "/api/me",
            &[(CLAIMED_HEADER, "..")],
        )
        .await;
        assert_eq!(
            body["user"],
            serde_json::Value::Null,
            "an invalid header resolves to null here, never 401"
        );
        // M6.5c2: a plain name is a key, normalized once in the extractor.
        let (_, body) = get_with(
            Arc::new(AppState::in_memory()),
            "/api/me",
            &[(CLAIMED_HEADER, "  Ben   Castle ")],
        )
        .await;
        assert_eq!(body["user"], "ben castle", "a name is a key, normalized once");
```

  **(f)** `crates/yokohama-wasm/src/lib.rs` — `:285-290` (`not-an-email` is a valid name now; the arm compares the
  whole body with the grammar's own refusal, which is its claim):

```rust
        // A malformed caller is `User::parse`'s own sentence, which is what the
        // axum host's `identity::resolve` carries into ITS 401 — so the two
        // hosts refuse the same input in the same words. `..` is the name the
        // key grammar refuses by its leading-character rule (M6.5c2, review H1).
        let (status, body) = dispatch_inner(Some(".."), "GET", "/api/games", "", None, b"");
        assert_eq!(status, 401);
        assert_eq!(body, serde_json::json!({ "error": User::parse("..").unwrap_err() }).to_string());
```

  and a new cell directly before `#[test] fn the_clock_freezes_and_ticks` (`:308-309`):

```rust
    /// **A plain name on a table's watcher list reads on the browser host**
    /// (M6.5c2, spec §3's identity cells): a restored or loaded table runs the
    /// layer's `check_shape` here as everywhere, over the one key grammar.
    #[test]
    fn a_table_whose_watcher_list_holds_a_plain_name_reads() {
        let _s = serial();
        init();
        let body = br#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"user":"blue@t.test"}],"id":"named"}"#;
        let (status, created) = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", Some("application/json"), body);
        assert_eq!(status, 201, "{created}");
        for (who, keep) in [("red@t.test", br#"{"seat":0,"card":1}"#), ("blue@t.test", br#"{"seat":1,"card":1}"#)] {
            let (status, _) = dispatch_inner(Some(who), "POST", "/api/games/named/keep", "", Some("application/json"), keep);
            assert_eq!(status, 200, "{who}");
        }
        let mut doc: serde_json::Value = serde_json::from_str(&read_inner("table", "named").unwrap().unwrap()).unwrap();
        let record = read_inner("record", "named").unwrap().expect("the last keep dealt the game");
        for (watcher, want) in [("ben castle", 200), ("..", 500)] {
            doc["unlocked"] = serde_json::json!([watcher]);
            init();
            load_inner("table", "named", &doc.to_string()).unwrap();
            load_inner("record", "named", &record).unwrap();
            let (status, body) = dispatch_inner(Some("red@t.test"), "GET", "/api/games/named/table", "", None, b"");
            assert_eq!(status, want, "{watcher:?}: {body}");
        }
    }
```

  **(g)** `crates/yokohama-worker/src/lib.rs` — a new cell directly before the doc at `:699` ("`set_clock` hands back
  what it replaced…"). It hand-loads the roster rather than unlocking through the route, because the module's
  `worker` door will admit no plain-name CALLER once Task 2's mode rule lands — which is not this cell's property:

```rust
    /// **A plain name on a table's watcher list reads on this host too**
    /// (M6.5c2, spec §3's identity cells; review H2): `check_shape` runs on the
    /// module's every table read, over the key grammar an `open` host writes
    /// watchers under. The table is this host's own — created and dealt by its
    /// routes — and its roster is given a plain name by hand, because this
    /// module's `worker` door admits no plain-name CALLER to unlock with (the
    /// mode rule), which is not the property here.
    #[test]
    fn a_table_whose_watcher_list_holds_a_plain_name_reads() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let body = r#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"user":"blue@t.test"}],"password":"sesame"}"#;
        assert_eq!(create_inner(Some("red@t.test"), body.as_bytes(), "named", Some("application/json")).status, 201);
        for (who, seat) in [("red@t.test", 0), ("blue@t.test", 1)] {
            let keep = format!(r#"{{"seat":{seat},"card":1}}"#);
            assert_eq!(call(who, "POST", "/api/games/named/keep", Some(&keep)).status, 200, "{who}");
        }
        let mut doc: serde_json::Value = serde_json::from_str(&read_inner("table", "named").unwrap().unwrap()).unwrap();
        let record = read_inner("record", "named").unwrap().expect("the last keep dealt the game");
        for (watcher, status) in [("ben castle", 200), ("..", 500)] {
            doc["unlocked"] = serde_json::json!([watcher]);
            init_inner(&config("off")).unwrap();
            load_inner("table", "named", &doc.to_string()).unwrap();
            load_inner("record", "named", &record).unwrap();
            let r = call("red@t.test", "GET", "/api/games/named/table", None);
            // `..` is the anti-vacuity: the grammar still refuses it, so 200
            // above is a shape check that passed, not one that never ran.
            assert_eq!(r.status, status, "{watcher:?}: {}", r.body);
        }
    }
```

  **(h)** `crates/yokohama-worker/src/receipt.rs`, `a_receipt_is_the_outcome_object_and_needs_the_archive_on` —
  `:1205-1206` (`"nobody"` is a valid name now):

```rust
        let err = init_inner(&r2(Some(".."))).unwrap_err();
        assert!(err.contains("caller"), "{err}");
        // Anti-vacuity, and the open door's premise (M6.5c2): a plain name is a
        // caller the key grammar accepts.
        init_inner(&r2(Some("  Ben   Castle "))).expect("a plain name is a caller");
```

  and its doc's last clause (`:1182-1183`, "and `init` takes an archive caller only as an address.") becomes:

```rust
    /// only where a put can have been deferred; and `init` takes an archive
    /// caller only as a key — an email or (M6.5c2) a plain name.
```

- [ ] **Step 2: Run them to see them fail.**

```bash
cargo test -p yokohama-web --lib identity::
cargo test -p yokohama-wasm
cargo test -p yokohama-worker
```

  Expected (D1, measured):
  - the layer's lib test binary does NOT COMPILE: `error[E0599]: no associated function or constant named
    `typed_form` found for struct `identity::User`` (×2) and `error[E0599]: no method named `is_email` found for struct
    `identity::User`` (×3);
  - `yokohama-wasm`: 1 failed, `tests::a_table_whose_watcher_list_holds_a_plain_name_reads` — `"ben castle": … left:
    500 right: 200` (the emails-only grammar refuses the roster on read). The repaired 401 arm PASSES: `..` is refused
    by today's grammar too;
  - `yokohama-worker`: 2 failed — `tests::a_table_whose_watcher_list_holds_a_plain_name_reads` (500, not 200) and
    `receipt::tests::a_receipt_is_the_outcome_object_and_needs_the_archive_on` (`a plain name is a caller: "the
    archive's caller must be an address: …"`).

- [ ] **Step 3: Implement the key grammar.** In `crates/yokohama-web/src/identity.rs`, `:1` becomes
  `//! Who a request is from — one configured source, one header, one normalized key.`, and `:114-142` (from `User`'s
  doc up to `pub fn as_str` at `:143`, which stays) becomes:

```rust
/// A KEY (M6.5c2, spec §2.1 rule 1): what every seat binding, watcher entry
/// and archive directory is written under — an email, or a plain name.
/// Normalized HERE and nowhere else: [`User::parse`] is the one grammar, and
/// it knows no deployment mode; which keys a MODE admits as a caller is
/// [`resolve`]'s rule, not this one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User(String);

/// The longest plain name, in characters (every one of them ASCII).
const NAME_MAX: usize = 32;

/// A plain name's rule, as the refusal words it.
const NAME_RULE: &str = "a name is 1 to 32 letters, digits, spaces, dots, dashes or \
                         underscores, and begins with a letter or a digit";

impl User {
    /// The key grammar, for EVERY caller — `resolve`, a create's
    /// `SeatSpec.user`, `check_shape`'s watcher list,
    /// `archive::caller_segment`, the Worker module's `identity_of` and
    /// `init`, and `backup::check_pair` — none of which knows the mode.
    ///
    /// The value's [`Self::typed_form`] (trimmed, each run of internal spaces
    /// collapsed to one), lowercased by ASCII rules, is then one of two shapes:
    /// - **an email**, exactly today's shape: one `@` with something on both
    ///   sides, no whitespace or control character, at most 254 bytes. A shape
    ///   check, not RFC validation.
    /// - **a plain name**: 1 to 32 ASCII letters, digits, spaces, `.`, `-` and
    ///   `_`, **beginning with a letter or a digit**, and never the literal
    ///   `anonymous`. The leading character is what keeps a name's archive
    ///   segment from being `.` or `..` (review H1, measured); `anonymous` is
    ///   refused so it never becomes a key (the Worker's HEADER `anonymous`
    ///   reads as absent — a separate rule, in [`resolve`]).
    ///
    /// The `@` decides which shape is being checked, because a name's alphabet
    /// has none: an email is never a name, and a name never an email.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let s = Self::typed_form(raw).to_ascii_lowercase();
        if s.is_empty() {
            return Err("no name or email given".to_string());
        }
        if s.contains('@') {
            return Self::email(raw, s);
        }
        let alphabet = s
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b' ' | b'.' | b'-' | b'_'));
        if s.len() > NAME_MAX || !alphabet || !s.as_bytes()[0].is_ascii_alphanumeric() {
            return Err(format!("{raw:?} is neither an email nor a name: {NAME_RULE}"));
        }
        if s == "anonymous" {
            return Err(format!("{raw:?} is reserved: `anonymous` is how an edge says nobody"));
        }
        Ok(Self(s))
    }

    /// Today's email rule, over the already-normalized `s`; `raw` is quoted in
    /// the refusal as the caller sent it.
    fn email(raw: &str, s: String) -> Result<Self, String> {
        if s.len() > 254 {
            return Err("an email is at most 254 bytes".to_string());
        }
        if s.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(format!("{raw:?} contains whitespace or a control character"));
        }
        let mut parts = s.split('@');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(l), Some(d), None) if !l.is_empty() && !d.is_empty() => Ok(Self(s)),
            _ => Err(format!("{raw:?} is not an email: exactly one @ with something on both sides")),
        }
    }

    /// What a person typed, tidied: `raw` trimmed (Rust's `str::trim`, Unicode
    /// white space), each run of internal SPACES collapsed to one, case KEPT.
    /// Everything [`Self::parse`] does to a value before it lowercases and
    /// checks it, so an accepted key is exactly this, ASCII-lowercased — which
    /// is why the client may show it as the player's name (`"  Ben   Castle "`
    /// shows as `Ben Castle`, keyed `ben castle`). Emitted beside each key in
    /// `web/src/wire/identity_cases.json`, which the client's name step is
    /// pinned to.
    pub fn typed_form(raw: &str) -> String {
        raw.trim().split(' ').filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ")
    }

    /// Whether this key has the email shape — the only shape a key with an `@`
    /// can have. The mode rule's test: under `cf-access` and `worker` a caller
    /// must be one.
    pub fn is_email(&self) -> bool {
        self.0.contains('@')
    }

```

  The email rule is today's, moved verbatim into `email` (only its `match` lost a temporary). **`split(' ')`, not
  `split_whitespace`**: a tab inside a name must stay a tab, which the alphabet then refuses (`"ben\tcastle"`).

  The sentences and documents the widening makes false, in the same step:
  - `tables.rs:406`: `return Err(format!("{bad:?} on the watcher list is neither an email nor a name"));`
    (spec §2.1's rewording; the `"watcher list"` fragment the shape cell pins stays).
  - `tables.rs:398-401` ("Each entry must parse as a `User` for the plainer reason…"):
    `// join at that table. Each entry must parse as a `User` — an email or a` /
    `// plain name (M6.5c2) — for the plainer reason that `can_watch` compares` /
    `// it against one: an entry that never normalizes is an entry no caller can` /
    `// ever match, written as though it admitted somebody.`
  - `chat.rs:291`: `"message {i}'s author {u:?} is neither an email nor a name",`
  - `tables.rs:113`: `/// One seat of a requested table: a person's key (an email, or a plain name),` +
    `/// the bot, or nobody yet.`
  - `tables.rs:250-253` ends: `// knows what a key of this API's is — so a seat bound from a` / `// header reading
    `  Red@Wire.TEST ` is the seat the next request` / `// finds. A plain name is a key here under every mode
    (M6.5c2); only` / `// a CALLER is held to the mode's rule.`
  - `table.rs:33-35`: `/// A human seat carries the NORMALIZED key `identity::User::parse` produced — an` /
    `/// email, or (M6.5c2) a plain name — the one key every binding is written` / `/// under, so a seat bound from a
    header reading `  Red@Wire.TEST ` is the same` / `/// seat the next request finds.`
  - `table.rs:200-201`: `… (M6.3, R-3.5). Normalized keys, as every` / `/// other binding here is.`
  - `games.rs:1928`: `User::parse(s).expect("a key")`.
  - `crates/yokohama-worker/src/config.rs:95`: `.map_err(|e| format!("the archive's caller must be an email or a name:
    {e}"))`; and `:59-60`: `/// the parse that normalizes every sender, and refused if it is no key — an` /
    `/// email, or (M6.5c2) a plain name.`

- [ ] **Step 4: Run the grammar green**, then the three crates:

```bash
cargo test -p yokohama-web --lib identity::
cargo test -p yokohama-web --lib
cargo test -p yokohama-wasm
cargo test -p yokohama-worker
```

  Expected: `identity::` **7 passed**; the layer's lib **+3** over your HEAD's reading (the rewritten cell replaced,
  two new identity cells, the tables cell; D1 read 278 → 281); wasm **+1**; worker **+1**.

- [ ] **Step 5: Write the failing containment cells — (Review Focus 2), the archive's half.** In
  `crates/yokohama-web/src/archive.rs`'s test module, three new cells directly after the rewritten
  `a_caller_segment_is_path_safe_and_readable`:

```rust
    /// **The layout refuses a `.` or `..` component** (M6.5c2, spec §2.1's
    /// defence in depth; review R2-m3) — anywhere, the prefix and the id alike
    /// — and nothing else: a dot INSIDE a component is a name's business.
    #[test]
    fn the_layout_refuses_a_dot_or_dot_dot_component() {
        for (prefix, id) in [("browser/../", "sim-1"), ("browser/./", "sim-1"), ("", ".."), ("", "."), ("../", "x")] {
            let refused = paths(7, prefix, Why::Over, id);
            assert!(refused.is_err(), "{prefix:?} + {id:?}: {refused:?}");
        }
        assert_eq!(
            paths(7, "browser/z../", Why::Over, "a..b"),
            Ok(("browser/z../rev7/over/a..b.json".into(), "browser/z../rev7/over/a..b.table.json".into())),
            "dots inside a component are not a step",
        );
    }

    /// **The review's measured probe, as a regression cell** (M6.5c2, review
    /// H1). In an export of `6ce33fd6` a widened grammar with no leading-
    /// character rule let the name `..` become the segment `..`, and a put
    /// under `browser/../` LANDED at the server's own `rev7/over/<id>.json` —
    /// after which the server's own game-over put answered `Conflict`. Three
    /// fences now stand between a name and that path, each asserted where it
    /// stands: the grammar refuses the name, [`paths`] refuses the component,
    /// and the directory archive refuses the put and writes nothing.
    #[cfg(feature = "server")]
    #[test]
    fn a_dot_dot_segment_never_lands_a_save_in_the_servers_own_tree() {
        assert!(crate::identity::User::parse("..").is_err(), "fence 1: `..` is never a key");
        let hostile = "browser/../";
        assert!(paths(yokohama_core::SEMANTICS_REV, hostile, Why::Over, "sim-victim").is_err(), "fence 2");
        let dir = std::env::temp_dir().join(format!("yokohama_archive_probe_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = DirArchive::new(&dir);
        let put = a.put(hostile, Why::Over, "sim-victim", "{\"planted\":1}", "{\"t\":1}");
        assert!(matches!(&put, Err(PutError::Failed(_))), "fence 3 — a FAILURE, never a landing: {put:?}");
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none(), "nothing was written at all");
        // The server's own game-over put at that id lands, as it did not in the
        // probe: the archive keeps the GAME'S bytes and not the planted ones.
        assert!(matches!(a.put("", Why::Over, "sim-victim", "{\"game\":1}", "{\"t\":1}"), Ok(PutOutcome::Landed)));
        let (own, _) = paths(yokohama_core::SEMANTICS_REV, "", Why::Over, "sim-victim").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join(&own)).unwrap(), "{\"game\":1}");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **The directory archive's own check is LEXICAL, over the relative path**
    /// (M6.5c2, spec §2.1; review R2-m3), and this is the input that reaches it
    /// past [`paths`]: an ABSOLUTE prefix has no `.` or `..` component, and
    /// `Path::join` would let it REPLACE the archive root outright. No caller
    /// builds one; this is what holds when one does. (A check for
    /// `Component::ParentDir` alone would pass this path: it has none.)
    #[cfg(feature = "server")]
    #[test]
    fn the_directory_archive_refuses_a_path_that_would_leave_its_root() {
        let base = std::env::temp_dir().join(format!("yokohama_archive_lexical_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (root, elsewhere) = (base.join("root"), base.join("elsewhere"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        let absolute = format!("{}/", elsewhere.display());
        assert!(paths(7, &absolute, Why::Over, "sim-1").is_ok(), "premise: the layout alone lets it through");
        let put = DirArchive::new(&root).put(&absolute, Why::Over, "sim-1", "{\"r\":1}", "{\"t\":1}");
        assert!(matches!(&put, Err(PutError::Failed(_))), "{put:?}");
        assert!(std::fs::read_dir(&elsewhere).unwrap().next().is_none(), "nothing written outside the root");
        assert!(std::fs::read_dir(&root).unwrap().next().is_none(), "nor inside it");
        std::fs::remove_dir_all(&base).ok();
    }
```

  The existing call sites take the `Result`: `:364-374` and `:444-451` compare with `Ok((…, …))` (wrap each expected
  tuple in `Ok(…)`); `:384`, `:387`, `:413`, `:431` and `:512` append `.unwrap()` to the `paths(…)` call.

  In `crates/yokohama-worker/src/receipt.rs`'s test module, directly before the doc at `:1180` ("`receipt` takes
  exactly the outcome…"):

```rust
    /// **The layout's refusal reaches this archive as a FAILURE, and no plan is
    /// recorded** (M6.5c2, spec §2.1's defence in depth; review R2-m3). No
    /// caller can hand the layer a `.` or `..` component — the key grammar and
    /// `is_safe_id` close both — so the put is called directly, as the layer
    /// would call it with an id that slipped through: a `Failed`, never a
    /// conflict the shell would read as "a different record is there", and
    /// never a plan it would upload.
    #[test]
    fn a_path_the_layout_refuses_is_a_failure_and_records_no_plan() {
        let archive = ReceiptArchive::new(None);
        let table = json!({ "created_by": RED }).to_string();
        let put = archive.put("", Why::Over, "..", "{}", &table);
        assert!(matches!(&put, Err(PutError::Failed(why)) if why.contains("..")), "{put:?}");
        assert!(archive.pending.lock().unwrap().is_none(), "nothing deferred, so nothing uploads");
        // Anti-vacuity: the same put under a safe id IS deferred, so the arm
        // above read the layout's refusal and not an archive that plans nothing.
        let _ = archive.put("", Why::Over, "sim-1", "{}", &table);
        assert!(archive.pending.lock().unwrap().is_some(), "a safe id is planned");
    }
```

  (`pending` is private to `receipt`; its test module is a child and reads it.)

- [ ] **Step 6: Run them to see them fail.**

```bash
cargo test -p yokohama-web --lib archive::
cargo test -p yokohama-worker a_path_the_layout
```

  Expected: the layer's lib test binary does NOT COMPILE — `error[E0308]: mismatched types` (×5, the `Ok((…))`
  compares) and `error[E0599]: no method named `unwrap`` (×6) / `is_err` (×2) / `is_ok` (×1) `found for tuple
  `(std::string::String, std::string::String)``. The worker cell COMPILES (it never calls `paths`) and FAILS at its
  first assertion: the `..` put was deferred into a plan, `Err(Failed("deferred to the host"))` (`receipt.rs:59`)
  with no `..` in it.

- [ ] **Step 7: Implement the three fences.** `crates/yokohama-web/src/archive.rs:115-122`:

```rust
/// The ONE layout: `<prefix>rev<N>/<over|abandoned>/<id>.json` and
/// `<prefix>rev<N>/<over|abandoned>/<id>.table.json` — the file store's own two
/// names under a semantics-rev prefix, so a synced archive is a directory the
/// CLI and this server both already read.
///
/// **It refuses a path with a `.` or `..` component** (M6.5c2, spec §2.1's
/// defence in depth; review R2-m3). Every caller builds its arguments from a
/// segment [`caller_segment`] rendered and an id `is_safe_id` admitted, and
/// neither can spell one — so this is the second fence and not the first, and
/// an `Err` is a caller's bug named rather than a path written. The refusal
/// does not echo the path: it can reach a client's 500.
pub fn paths(rev: u32, prefix: &str, why: Why, id: &str) -> Result<(String, String), String> {
    let base = format!("{prefix}rev{rev}/{}/{id}", why.dir());
    if let Some(step) = base.split('/').find(|part| matches!(*part, "." | "..")) {
        return Err(format!("an archive path cannot hold a {step:?} component"));
    }
    Ok((format!("{base}.json"), format!("{base}.table.json")))
}
```

  `:124-133`, `caller_segment`'s doc, down to (not including) the blank line and the `**It COLLIDES, on purpose…**`
  paragraph at `:134-138`:

```rust
/// R-6.5b.15: a path-safe, READABLE rendering of a caller's key — `@` becomes
/// `_at_`, anything outside `[a-z0-9._-]` becomes `_` (a name's space too),
/// capped at 64 characters.
///
/// **Why the segment is never empty, `.` or `..`** — restated for M6.5c2's key
/// grammar (spec §2.1; review H1, which MEASURED the hole an unconstrained name
/// opened: `..` rendered as the segment `..`, and a browser's save landed in
/// this server's own `rev7/over/`). [`crate::identity::User::parse`] admits two
/// shapes, and each closes it by its own arithmetic:
/// - an EMAIL has a non-empty local part and domain either side of one `@`,
///   which renders as four characters, so its segment is at least six
///   characters of this alphabet;
/// - a PLAIN NAME begins with a letter or a digit — the grammar's rule, which
///   exists for exactly this — and that character is kept, so its segment
///   begins with `[a-z0-9]`.
///
/// Neither holds a `/`, and the cap only shortens a segment that is already
/// non-empty. [`paths`] and `DirArchive::put` refuse a `.` or `..` component
/// besides, so this argument is no longer the only fence between a key and a
/// path outside its prefix.
```

  `:197` becomes `use std::path::{Component, Path, PathBuf};`, and `:254` becomes:

```rust
            let (record, table) = paths(yokohama_core::SEMANTICS_REV, prefix, why, id).map_err(PutError::Failed)?;
            // **Contained, LEXICALLY** (M6.5c2, spec §2.1; review R2-m3): every
            // component of both RELATIVE paths is a plain name, so joining them
            // under `self.dir` lands under `self.dir` — no `..`, and no root,
            // which `Path::join` would let REPLACE `self.dir` outright. Lexical
            // because it runs before the write and `canonicalize` needs the file
            // to exist; over the relative paths, not the joined one, because the
            // configured directory is the operator's and may itself be relative
            // (`YOKOHAMA_WEB_ARCHIVE_DIR=../saves`).
            for rel in [&record, &table] {
                if !Path::new(rel).components().all(|c| matches!(c, Component::Normal(_))) {
                    return Err(PutError::Failed("an archive path must stay under its root".to_string()));
                }
            }
```

  `crates/yokohama-web/src/api.rs:1936` (the route's own `paths` call, which reports the leaf's path):

```rust
    // Unreachable as a refusal — the key grammar and `is_safe_id` admit no `.`
    // or `..` — and answered in the put's own failure words if it ever is.
    let (path, _) = paths(yokohama_core::SEMANTICS_REV, &prefix, why, &doc.id)
        .map_err(|e| ApiError::server_error(format!("the archive could not be written: {e}")))?;
```

  `crates/yokohama-worker/src/receipt.rs:200`:

```rust
        let (record_key, table_key) = paths(yokohama_core::SEMANTICS_REV, prefix, why, id).map_err(PutError::Failed)?;
```

- [ ] **Step 8: Run the fences green.**

```bash
cargo test -p yokohama-web --lib archive::
cargo test -p yokohama-web --lib
cargo test -p yokohama-worker
```

  Expected: `archive::` **11 passed** (8 + 3); the layer's lib **+6** over your HEAD's reading at the start of this
  task (D1 read 284); worker **+2** (D1 read 61). The route's refusal arm (`api.rs:1936`) has no cell and cannot have
  one — no `User` can render `.` or `..` and the id is `is_safe_id`'s — so it is compile-checked only, which this
  step's build is. **Three containment mutants, D1 measured, for the reviewer:** dropping the lexical check fails one
  cell; dropping `paths`' check fails two; the wording "no `Component::ParentDir` in the JOINED path" fails
  `the_directory_archive_refuses_a_path_that_would_leave_its_root`.

- [ ] **Step 9: The contract's cell, first — (Review Focus 1)**, the key half as the client will be held to it. In
  `crates/yokohama-web/tests/wire_fixtures.rs`, directly before the doc at `:2299` ("**No payload spells the setup
  seed…**"):

```rust
/// **What the rows say, where a person would check first** (M6.5c2) — and the
/// one relation the client's name step relies on between the two columns: on
/// every accepted row the key IS the display, ASCII-lowercased, and a row is
/// never accepted in one column and refused in the other. (Review Focus 1: one
/// key for the name typed two ways.)
#[test]
fn the_identity_cases_carry_the_key_grammar_s_verdicts() {
    let rows = identity_cases();
    let rows = rows.as_array().expect("a list");
    let row = |raw: &str| {
        rows.iter().find(|r| r["raw"] == raw).unwrap_or_else(|| panic!("no row for {raw:?}")).clone()
    };
    // (Review Focus 1) one key, shown as typed.
    assert_eq!(row("  Ben   Castle "), json!({ "raw": "  Ben   Castle ", "key": "ben castle", "display": "Ben Castle" }));
    assert_eq!(row("ben castle")["key"], "ben castle", "the same person, typed plainly, is the same key");
    // Review H1's names, and the reserved word, are never keys.
    for raw in [".", "..", ". .", "-", ".x", "anonymous"] {
        assert_eq!(row(raw), json!({ "raw": raw, "key": null, "display": null }), "{raw:?}");
    }
    for r in rows {
        match (r["key"].as_str(), r["display"].as_str()) {
            (Some(key), Some(display)) => assert_eq!(key, display.to_ascii_lowercase(), "{r}"),
            (None, None) => {}
            _ => panic!("accepted in one column and refused in the other: {r}"),
        }
    }
}
```

```bash
cargo test -p yokohama-web --test wire_fixtures the_identity_cases
```

  Expected: does NOT COMPILE — `error[E0425]: cannot find function `identity_cases` in this scope`.

- [ ] **Step 10: The emitter.** Directly above that cell:

```rust
/// **The key grammar's verdicts, frozen for the client's name step** (M6.5c2,
/// spec §2.1 and §3).
///
/// Not a payload any route serves — a CONTRACT, emitted and refused exactly as
/// [`site_space_words`] is. Under the open door the client normalizes a typed
/// name ITSELF, before any request (`web/src/nameRule.ts`, cycle B), and is
/// pinned to these rows, so the two cannot disagree about who a name is.
///
/// Each row is one committed input and the layer's two answers: `key`,
/// `User::parse`'s normalized key, and `display`, `User::typed_form` — the same
/// value with its case kept, which is what a player is shown. Both are `null`
/// where the layer refuses the input. The inputs split every rule the grammar
/// has, and they include every row the name step's own coverage cell requires
/// — among them the characters on which Rust and JavaScript disagree: U+0085
/// (Rust's `str::trim` removes it, JavaScript's `trim` does not), U+FEFF (the
/// reverse) and U+212A KELVIN SIGN (JavaScript's `toLowerCase` folds it to
/// `k`). serde_json writes non-ASCII unescaped: read this file's diff with
/// `cat -v`.
fn identity_cases() -> Value {
    use yokohama_web::identity::User;
    let (a32, a33) = ("a".repeat(32), "a".repeat(33));
    let inputs = [
        // The two shapes, trimmed and lowercased by one rule.
        "red@wire.test", "Red@Wire.TEST", " Red@Wire.TEST ", "a+b@x.test",
        "ben castle", "  Ben   Castle ", "Ben", "BEN", "b3n_c-a.stle", "a.test", "7 wonders",
        // The length bounds.
        "x", a32.as_str(), a33.as_str(),
        // The leading character (review H1), and the reserved word.
        ".", "..", ". .", "-", ".x", "_x", "x.", "anonymous", "ANONYMOUS", "anonymous1",
        // Nothing at all.
        "", "   ",
        // Outside both alphabets: a non-ASCII letter, full-width letters, the
        // Kelvin sign (which JavaScript's `toLowerCase` folds to `k`), and the
        // email shape's four refusals.
        "Bén", "ben!", "a/b", "\u{ff42}\u{ff45}\u{ff4e}", "\u{212a}elvin", "a@b@c", "@x.test", "a@", "a b@x.test",
        // Tabs and returns, and the white space the two trims disagree on.
        "ben\t", "ben\tcastle", "ben\r", "\u{a0}Ben", "\u{85}ben", "\u{feff}ben",
    ];
    Value::Array(
        inputs
            .iter()
            .map(|raw| match User::parse(raw) {
                Ok(key) => json!({ "raw": raw, "key": key.as_str(), "display": User::typed_form(raw) }),
                Err(_) => json!({ "raw": raw, "key": null, "display": null }),
            })
            .collect(),
    )
}
```

  and in `corpus()`, directly after `:2149` (`all.push(("site_space_words".to_string(), site_space_words()));`):

```rust
    all.push(("identity_cases".to_string(), identity_cases()));
```

  Being in `corpus()`'s payloads is what puts the file under all three leakage pins, which iterate exactly that list
  (`:2352`, `:2455`, `:2606`); none of the rows spells the seed, a password or a watcher's address.

```bash
cargo test -p yokohama-web --test wire_fixtures
```

  Expected: `the_identity_cases_carry_the_key_grammar_s_verdicts` passes; **1 failed**,
  `the_committed_wire_fixtures_are_what_the_server_emits`: `identity_cases.json — No such file or directory (os error
  2)`.

- [ ] **Step 11: Regenerate the corpus and READ the diff.**

```bash
UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures
git status --short web/src/wire web/src/wire-script
git diff --stat web/src/wire web/src/wire-script
cat -v web/src/wire/identity_cases.json | grep -n 'M-'
cargo test -p yokohama-web --test wire_fixtures
```

  The regeneration run exits NON-ZERO by design (8 passed, 4 failed: the writer asserts the variable is unset, and
  three cells refuse a regeneration run); the check run that follows reads **12 passed**. **Expected, and nothing
  else:**
  - ONE new file, `?? web/src/wire/identity_cases.json` — 41 rows, 207 lines, keys in serde's order (`display`, `key`,
    `raw`); `git diff --stat` prints NOTHING (every committed payload and all three scripts byte-identical);
  - **19 rows accepted** (`raw` → `key` / `display`): `"red@wire.test"` → `red@wire.test` / `red@wire.test`;
    `"Red@Wire.TEST"` and `" Red@Wire.TEST "` → `red@wire.test` / `Red@Wire.TEST`; `"a+b@x.test"` → itself;
    `"ben castle"` → itself; `"  Ben   Castle "` → `ben castle` / `Ben Castle`; `"Ben"` → `ben` / `Ben`; `"BEN"` →
    `ben` / `BEN`; `"b3n_c-a.stle"`, `"a.test"`, `"7 wonders"`, `"x"`, 32 × `a`, `"x."`, `"anonymous1"` → themselves;
    `"ben\t"` and `"ben\r"` → `ben` / `ben`; `" Ben"` → `ben` / `Ben`; `"\u0085ben"` → `ben` / `ben`;
  - **22 rows refused** (`key` and `display` both `null`): 33 × `a`, `"."`, `".."`, `". ."`, `"-"`, `".x"`, `"_x"`,
    `"anonymous"`, `"ANONYMOUS"`, `""`, `"   "`, `"Bén"`, `"ben!"`, `"a/b"`, `"ｂｅｎ"` (full-width
    `ben`), `"Kelvin"` (the Kelvin sign), `"a@b@c"`, `"@x.test"`, `"a@"`, `"a b@x.test"`, `"ben\tcastle"`,
    `"﻿ben"`;
  - `cat -v` shows the three raw characters JSON does not escape (NBSP, NEL, BOM) plus `é`, the Kelvin sign and the
    full-width letters as `M-` sequences.

  A changed payload, or a row that differs from the above, is a FINDING: stop and report it rather than commit it.

- [ ] **Step 12: The client's typed entry, and the two replays' third columns.** `web/src/wire/contract.ts`:
  - after `:106`: `import identityCases from "./identity_cases.json";`
  - after `:374` (`const _artManifest: …`):

```ts
/** One input of the key grammar and the layer's two answers (M6.5c2, spec §2.1
 *  and §3): `key` is `User::parse`'s normalized key, `display` its
 *  `User::typed_form` — the same value, case kept — and both are `null` where
 *  the layer refuses the input. Not a route's payload: a CONTRACT, emitted by
 *  `wire_fixtures`, which the client's name step is pinned to. */
export interface IdentityCase {
  raw: string;
  key: string | null;
  display: string | null;
}
const _identityCases: readonly IdentityCase[] = identityCases;
```

  - `:493`: `_artManifest,` becomes `_artManifest, _identityCases,`;
  - after `:838` (`siteSpaceWords: …`):

```ts
  /** Not a payload either: the key grammar's verdicts over a committed list of
   *  inputs (`User::parse` and `User::typed_form`), frozen so the name step's
   *  normalization cannot drift from the server's. See that fixture's doc in
   *  `wire_fixtures.rs`. */
  identityCases: identityCases as readonly IdentityCase[],
```

  The assignment `_identityCases: readonly IdentityCase[] = identityCases` is a real check, not a cast: MEASURED, a
  mutant `display: number | null` fails `tsc -b` with `TS2322 … Types of property 'display' are incompatible`.

  `web/src/local/corpus.node.test.ts` — `NOT_SERVED` (`:141-162`), directly after `"site_space_words",` (`:143`):
  `  "identity_cases",`; and its doc's first bullet (`:126-128`):

```ts
 * - `art_manifest`, `site_space_words` and `identity_cases` were never route
 *   payloads: the first is the host's own art directory, the other two
 *   CONTRACTS the client holds itself to (the terminal's words, the key
 *   grammar's verdicts).
```

  `worker/test/corpus.test.ts` — `THIRD_COLUMN` (`:57-66`) is compared SORTED and as written: INSERT
  `  "identity_cases",` between `"house_chat_off_404",` (`:59`) and `"locked",`; and `:46` becomes
  ` * - `site_space_words` and `identity_cases` are contracts, never a route's` / ` *   payload.`

  Both lists were derived the way each replay derives them (every `web/src/wire/*.json` less the fixtures its scripts
  name — `walk.json` for the node replay; all three scripts for the pool) and MEASURED equal to the edited lists:
  node 21 names, pool 9.

- [ ] **Step 13: Run the corpus and both clients' suites.**

```bash
cargo test -p yokohama-web --test wire_fixtures
cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm
cd ../worker && npm run build:module:debug && npm run typecheck && npm run test:scripts && npm test
```

  Expected: `wire_fixtures` **12 passed**; web typecheck clean, `npm test` unchanged (the contract adds a type and no
  cell; D1 read 1056), `test:wasm` unchanged (`NOT_SERVED` is a list; 92); worker typecheck clean, `test:scripts`
  unchanged (352), the pool unchanged (195). `build:wasm` and `build:module:debug` are owed first: the grammar is in
  both modules. MEASURED in D1's export — and dropping `"identity_cases"` from either list fails that replay's corpus
  cell (`+   "identity_cases",` in the diff it prints).

- [ ] **Step 14: CI's count, 5 → 7.** `.github/workflows/test.yml`:
  - `:226-228`: `# own cells never run there. Seven cells, one of which` /
    `# (`claimed_parses_only_where_the_development_feature_is_compiled_in`)` /
    `# takes its refusing branch only here; the other six prove the module`;
  - `:246`: `- name: the refusing build, and that its seven identity cells really ran`;
  - `:256-257`: `echo "$out" | grep -q "test result: ok. 7 passed" || {` /
    `echo "::error::the identity:: filter did not run seven cells — R-6.5b2.3's only CI witness"`.

```bash
python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/test.yml
cargo test -p yokohama-web --no-default-features --features server --lib "identity::" 2>&1 | grep "test result"
```

  Expected: the YAML loads; `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 277 filtered out`.

- [ ] **Step 15: The per-edit gates** (Global Constraints). Counts that move, as deltas over your HEAD's reading at
  the start of this task:
  - `cargo test -p yokohama-web` — lib **+6**, `wire_fixtures` **+1** (D1 read 300 = 284 + 2 + 2 + 12); a red
    whole-game gate is re-run ALONE before it is believed;
  - `cargo test -p yokohama-wasm` **+1**; `cargo test -p yokohama-worker` **+2**;
  - the public build's `identity::` — **7**;
  - `cargo clippy --workspace --all-targets -- -D warnings`; the wasm32 `cargo check`; both wasm32 builds;
  - `cargo doc -p yokohama-worker --no-deps` — **0** warnings (this task edits its docs);
  - web and worker as in Step 13;
  - **`npm run e2e` is NOT owed**: nothing a browser does or shows moves — the sign-in form still takes an address, a
    local game still plays as `local@browser`, and the new file is imported by tests alone;
  - the fence prints nothing;
  - **the reviewer reads** `git diff 1e16fe27 -- crates/yokohama-web/src` against exactly: `identity.rs` (the grammar
    and three cells), `archive.rs` (`paths`, the segment's doc, the lexical check, one rewritten and three new cells,
    seven call sites), `api.rs` (one call and its comment), `tables.rs` (two docs, one comment, one sentence, one
    fixture, one cell), `chat.rs` (one sentence, one fixture and its far side), `lib.rs` (the `/api/me` cell),
    `table.rs` (two docs), `games.rs` (one `expect`).

- [ ] **Step 16: Commit**, quoting Step 2's and Step 6's red runs in the body:

```bash
git add crates/yokohama-web/src/identity.rs crates/yokohama-web/src/archive.rs crates/yokohama-web/src/api.rs \
  crates/yokohama-web/src/tables.rs crates/yokohama-web/src/chat.rs crates/yokohama-web/src/lib.rs \
  crates/yokohama-web/src/table.rs crates/yokohama-web/src/games.rs crates/yokohama-web/tests/wire_fixtures.rs \
  web/src/wire/identity_cases.json web/src/wire/contract.ts web/src/local/corpus.node.test.ts \
  worker/test/corpus.test.ts crates/yokohama-wasm/src/lib.rs crates/yokohama-worker/src/receipt.rs \
  crates/yokohama-worker/src/config.rs crates/yokohama-worker/src/lib.rs .github/workflows/test.yml
git commit -m "feat(web): the key grammar takes plain names, and the archive's containment is restated in three fences"
git status --short   # clean
```

---

### Task 2: The rename and the mode rule (spec §2.1 "The rename" and "The server, as two rules" rules 2–4; R-6.5c2.3; review H2, R2-m1, R2-m8, m4)

**Measured:** D1 ran every step in an export (its trial's commits are in the ledger's `d1-trial.bundle`); the
review's chain carries it in `0001`. Two changes from that text: the test person is `Castle` (P-2), and the sweep's
grep runs AFTER the workflow line it counts is rewritten (P-11: at the earlier position `test.yml:224` also matched).

**Files:**
- Modify: `crates/yokohama-web/src/identity.rs`:
  - `:9-14` — the module doc states the open door's trust model (R-6.5c2.3 supersedes R-1.4's framing);
  - `:38-43`, `:46-60` — `IdentityMode::Claimed` → `IdentityMode::Open`, named `"open"`, header unchanged;
  - `:83-112` — `from_str`: the `open-door` feature, the trust-model refusal, the list naming `open`;
  - `:154-175` — `resolve` applies the mode rule; new `pub fn admit` and private `fn absent`;
  - tests: `:229-238` and `:240-248` take `Open`; `:250-270` and `:272-287` rewritten; one new cell.
- Modify: `crates/yokohama-web/Cargo.toml:16-25` — the feature `dev-identity` → `open-door`, still default.
- Modify: `crates/yokohama-web/src/lib.rs` — 21 × `IdentityMode::Claimed` → `IdentityMode::Open`; `:402` the doc;
  `:1599` the `/api/me` cell's mode, plus an edge-door arm after `:1624`; `:4411` the health cell; one new cell after
  the `/api/me` cell.
- Modify: `crates/yokohama-web/src/main.rs:74`, `:90` — two comments name `open`.
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs:291` — `IdentityMode::Open`.
- Modify: `crates/yokohama-worker/src/lib.rs` — `:47` imports `admit`; `:302-321` `identity_of`'s doc and fallback;
  one new cell before `:612`.
- Modify (regenerated): `web/src/wire/health.json`, `me.json`, `me_signed_out.json`.
- Modify: `worker/scripts/live-replay.test.mjs:484` — the fake live health's `identity` restates the axum host's
  committed value, which becomes `open` (a MEASURED break of `test:scripts`).
- Modify: `web/src/types.ts:483`, `:486`; `web/src/App.tsx:1292`, `:1295`; `web/src/Door.tsx:43`, `:45`;
  `web/src/OnThisDevice.tsx:318`, `:332`, `:336`; `web/src/OnThisDevice.test.tsx:422`, `:426`, `:429`, `:463`;
  `web/src/App.test.tsx:1916`, `:1921`, `:2067`, `:2069`.
- Modify (comments only): `web/src/api.ts:71`, `:122`, `:212`, `:505`; `web/src/sse.ts:4`; `web/src/ws.ts:11`;
  `web/src/signal.ts:25`; `web/src/whoami.ts:104`; `web/src/SignIn.tsx:1-3`; `web/src/test/harness.tsx:59`;
  `web/src/sse.test.ts:149`; `web/src/local/corpus.node.test.ts:91`; `worker/test/corpus.test.ts:31`;
  `worker/scripts/live-replay.mjs:81`; `worker/src/identity.ts:37-46`;
  `worker/src/identity.unit.test.ts:227-228`, `:246`; `worker/test/worker.test.ts:337-338`;
  `worker/src/config.ts:111-113`; `web/e2e/live.ts:230-231`, `:237`; `web/e2e/joins_by_link.spec.ts:141`;
  `deploy/Dockerfile:203`, `:210-211`; `deploy/Dockerfile.runtime:48-49`.
- Modify: `web/playwright.config.ts:242`, `:246` — the walk's server runs `YOKOHAMA_WEB_IDENTITY: "open"`.
- Modify: `.github/workflows/test.yml:220-229`, `:246`, `:256-257` — the feature's name, the cell's name, 7 → 8.
- Test: `identity::tests` (2 rewritten, 2 renamed, 1 new), `tests::me_answers_without_identity_and_names_the_mode`,
  `tests::health_reports_the_engines_own_compatibility_keys`, `tests::a_plain_name_seated_at_an_edge_door_is_a_key_nobody_holds`,
  the module's `a_value_no_header_can_carry_is_held_to_the_worker_modes_rule`, the client's 9 door cells, and
  `live-replay.test.mjs › refuses an exemption the Worker did not need: a host field reading as the axum host's`.

**Interfaces:**
- Consumes (Task 1): `User::parse` (the key grammar), `User::is_email`, `User::typed_form`.
- Produces: `IdentityMode::Open` (variable value `open`, wire name `open`); the cargo feature `open-door`, default;
  `identity::resolve(mode, headers)` + **the mode rule**; `identity::admit(mode, user) -> Result<User, String>` (cycle
  C passes its configured mode); `types.ts`'s `Me.mode: "open" | "cf-access" | "worker"`; the corpus's
  `health.json` `identity`, `me.json`'s and `me_signed_out.json`'s `mode` read `open`.

**No alias, and what that means for the live NAS**: once this commit is built, `YOKOHAMA_WEB_IDENTITY=claimed` exits 1
with `YOKOHAMA_WEB_IDENTITY must be one of open, cf-access, worker — got "claimed"`. The live NAS container sets
`claimed` (RUNBOOK §3) until cycle C's close redeploys it on `cf-access`; **it must not be redeployed from this branch.**

- [ ] **Step 1: Write the failing cells.** In `crates/yokohama-web/src/identity.rs`'s test module:
  - `:235`: `resolve(IdentityMode::Claimed, &all)` → `resolve(IdentityMode::Open, &all)`; `:242`:
    `resolve(IdentityMode::Claimed, &headers(&[]))` → `resolve(IdentityMode::Open, &headers(&[]))`;
  - replace `:250-287` (the two cells, doc comments included) with:

```rust
    #[test]
    fn the_mode_comes_from_the_environment_and_has_no_default() {
        // `open` is the one arm that depends on a feature (R-6.5b2.3, renamed
        // by R-6.5c2.3); both of its shapes are asserted by the cell below.
        #[cfg(feature = "open-door")]
        assert!(matches!(IdentityMode::from_str("open"), Ok(IdentityMode::Open)));
        assert!(matches!(IdentityMode::from_str("cf-access"), Ok(IdentityMode::CfAccess)));
        assert!(matches!(IdentityMode::from_str("worker"), Ok(IdentityMode::Worker)));
        let err = IdentityMode::from_str("").unwrap_err();
        assert!(err.contains("open") && err.contains("cf-access") && err.contains("worker"), "{err}");
        assert!(IdentityMode::from_str("neon").is_err(), "reserved, not built");
        // **No alias** (spec §2.1, "The rename"): a configuration still naming
        // the old mode is refused at start-up, and the refusal lists the three
        // it may say instead — `open` among them.
        let old = IdentityMode::from_str("claimed").unwrap_err();
        assert!(old.contains("\"claimed\"") && old.contains("open"), "{old}");
        // **The fail-closed path itself.** `from_env` is the only code that
        // turns an UNSET variable into a refusal, and until this line nothing
        // ran it: `main.rs` has no test content and the browser walk always
        // sets the variable, so `Err(_) => Ok(Self::Open)` would have
        // deleted R-1.3 with every suite green.
        assert!(IdentityMode::from_env_value(None).is_err(), "unset is a refusal, not a default");
    }

    /// R-6.5c2.3 (superseding R-1.4's framing, and renaming R-6.5b2.3's
    /// feature): `open` is a supported self-host door, and a build can still
    /// refuse it. The feature is in `default`, so this cell runs in the
    /// accepting shape and asserts the mode parses; the refusing shape runs
    /// only in the public build CI compiles, and there the refusal must NAME
    /// the feature, the two modes to use instead, and the trust model.
    #[test]
    fn open_parses_only_where_the_open_door_feature_is_compiled_in() {
        let parsed = IdentityMode::from_env_value(Some("open"));
        if cfg!(feature = "open-door") {
            assert!(matches!(parsed, Ok(IdentityMode::Open)));
        } else {
            let err = parsed.expect_err("a build without open-door refuses open");
            assert!(err.contains("open-door"), "the refusal NAMES the feature: {err}");
            assert!(err.contains("cf-access") && err.contains("worker"), "and what to use instead: {err}");
            assert!(err.contains("anyone who can reach"), "and the trust model it declines: {err}");
        }
    }

    /// **The mode rule** (M6.5c2, spec §2.1 rule 2), at `resolve`: under `open`
    /// every key the grammar accepts is a caller, a plain name included; under
    /// `cf-access` and `worker` a caller must also have the email shape, and a
    /// plain name reads as ABSENT — byte for byte the sentence no header gets,
    /// which is what makes `/api/me` answer `user: null` (state B) rather than
    /// the client drawing a refusal of the name. An address still resolves
    /// under every mode, so the rule is the shape and not the header.
    #[test]
    fn the_mode_rule_admits_a_plain_name_only_under_open() {
        let named = |mode: IdentityMode, value: &str| resolve(mode, &headers(&[(mode.header(), value)]));
        assert_eq!(named(IdentityMode::Open, "  Ben   Castle ").unwrap().as_str(), "ben castle");
        for mode in [IdentityMode::CfAccess, IdentityMode::Worker] {
            let absent = resolve(mode, &headers(&[])).unwrap_err();
            assert_eq!(named(mode, "ben").unwrap_err(), absent, "{}: a name is nobody here", mode.name());
            assert_eq!(named(mode, "Red@Wire.TEST").unwrap().as_str(), "red@wire.test", "{}", mode.name());
        }
        // The grammar still speaks first: what is no key at all is refused in
        // its own words under every mode, never mistaken for a name.
        assert_eq!(named(IdentityMode::CfAccess, "..").unwrap_err(), User::parse("..").unwrap_err());
    }
```

  In `crates/yokohama-web/src/lib.rs`:
  - `:1599`: `serde_json::json!({ "mode": "open", "user": null })`;
  - `:4411`: `assert_eq!(body["identity"], "open");`
  - at the end of `me_answers_without_identity_and_names_the_mode` (after its `cf-access` arm, `:1618-1624`):

```rust
        // M6.5c2's mode rule (spec §2.1 rule 2): at an edge door a plain name
        // names NOBODY — state B, `user: null` — where the open door above
        // took it as a key.
        let (_, body) = get_with(
            Arc::new(AppState::in_memory_with(M::CfAccess)),
            "/api/me",
            &[(CF_ACCESS_HEADER, "Ben Castle")],
        )
        .await;
        assert_eq!(body, serde_json::json!({ "mode": "cf-access", "user": null }));
```

  - and directly after that cell, a new one (spec §2.1 rule 3):

```rust
    /// **A body-borne plain name at an edge door is a key nobody holds**
    /// (M6.5c2, spec §2.1 rule 3; review H2). A create may seat `{"user":
    /// "ben"}` under `cf-access` exactly as it may seat a misspelled address —
    /// no new refusal — and the seat is one no caller can ever present, because
    /// the mode rule reads a header naming `ben` as nobody. The creator's own
    /// seat, an address, is unaffected.
    #[tokio::test]
    async fn a_plain_name_seated_at_an_edge_door_is_a_key_nobody_holds() {
        use identity::{IdentityMode as M, CF_ACCESS_HEADER};
        let router = app(Arc::new(AppState::in_memory_with(M::CfAccess)));
        let body = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "7",
            "seats": [{ "user": "red@t.test" }, { "user": "Ben" }], "id": "edge",
        });
        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/games")
                    .header("content-type", "application/json")
                    .header(CF_ACCESS_HEADER, "red@t.test")
                    .body(Body::from(body.to_string()))
                    .expect("a request"),
            )
            .await
            .expect("the router answers");
        assert_eq!(res.status(), StatusCode::CREATED, "accepted as a key, as a misspelled address is");
        let as_ben = |uri: &'static str| {
            router.clone().oneshot(
                Request::builder().uri(uri).header(CF_ACCESS_HEADER, "ben").body(Body::empty()).expect("a GET"),
            )
        };
        let res = as_ben("/api/games/edge/table").await.expect("the router answers");
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "a header naming `ben` is nobody here");
        let res = as_ben("/api/me").await.expect("the router answers");
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        let me: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
        assert_eq!(me["user"], serde_json::Value::Null, "and /api/me names nobody");
    }
```

  (`Body`, `Request`, `StatusCode` and `ServiceExt` are already imported by the test module, `lib.rs:911-913`.)

- [ ] **Step 2: Run them to see them fail.**

```bash
cargo test -p yokohama-web --lib identity::
```

  Expected: does NOT COMPILE — `error[E0599]: no variant, associated function, or constant named `Open` found for enum
  `identity::IdentityMode`` (×4). (With the variant alone renamed, the mode-rule cell's `named(mode, "ben")` would
  answer `Ok("ben")` — that is the behaviour this task removes.)

- [ ] **Step 3: Implement the rename and the mode rule.** `crates/yokohama-web/src/identity.rs`:
  - `:9-14` (the `claimed` paragraph of the module doc) becomes:

```rust
//! **`open` is the self-host door** (M6.5c2, R-6.5c2.3, superseding R-1.4's
//! framing of `claimed` as a development convenience): players choose a name,
//! and no identity provider is needed. Its trust model is stated rather than
//! hidden — anyone who can reach the server may use any name — and nothing
//! here verifies that a caller is who they say. It is also this project's dev
//! door. `cf-access` and `worker` read an address an edge in front has
//! verified, and the mode rule ([`admit`]) holds their callers to that shape.
//! Which mode a deployment runs under belongs in that deployment's own record,
//! beside the [`ENV_VAR`] its container sets. Reserved and unbuilt:
//! `cf-access-jwt` and `neon`, each a new arm here.
```

  - `:40` `Claimed,` → `Open,`; `:48` `Self::Claimed => "claimed",` → `Self::Open => "open",`; `:56`
    `Self::Claimed => CLAIMED_HEADER,` → `Self::Open => CLAIMED_HEADER,` (the header keeps its name: the NAME is
    claimed; only the mode is renamed);
  - `:87-109` (the `claimed` arms and the catch-all) become:

```rust
            // **`open` needs the build to have been asked for it** (M6.5b2's
            // R-6.5b2.3; the mode and its feature renamed by M6.5c2's
            // R-6.5c2.3). The mode itself stays — the variant, the header and
            // every cell that drives one are unconditional, because a mode that
            // vanished from the type would take the browser host's own tests
            // with it. What the feature gates is the one place a DEPLOYMENT can
            // choose it: this parse. A build without the feature refuses it
            // here and says why, so the operator reads the trust model rather
            // than guessing at a typo. There is no alias for the old name.
            #[cfg(feature = "open-door")]
            "open" => Ok(Self::Open),
            #[cfg(not(feature = "open-door"))]
            "open" => Err(format!(
                "{ENV_VAR}=open needs the `open-door` feature, which this build was compiled \
                 without. The open door trusts whatever name a caller sends: anyone who can \
                 reach this server may play as anyone, and nothing checks who they are. Use \
                 cf-access or worker behind an identity provider, or build with the feature."
            )),
            "cf-access" => Ok(Self::CfAccess),
            "worker" => Ok(Self::Worker),
            other => Err(format!(
                "{ENV_VAR} must be one of open, cf-access, worker — got {other:?}. \
                 There is no default: an identity source is a deployment decision."
            )),
```

  - `:154-175` (`resolve`, doc included) become:

```rust
/// The mode's ONE header, parsed by the key grammar and then held to the
/// mode's rule ([`admit`]); absent or invalid is `Err` with the sentence the
/// 401 carries. The Worker's `anonymous` — what it sends with no Access policy
/// in front of it — counts as absent.
pub fn resolve(mode: IdentityMode, headers: &HeaderMap) -> Result<User, String> {
    let raw = headers
        .get(mode.header())
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty() && !(mode == IdentityMode::Worker && *s == "anonymous"));
    match raw {
        Some(s) => User::parse(s).and_then(|user| admit(mode, user)),
        None => Err(absent(mode)),
    }
}

/// **The mode rule** (M6.5c2, spec §2.1 rule 2), in ONE place: under `open`
/// every key the grammar accepts is a caller; under `cf-access` and `worker` a
/// caller must also have the email shape, and a plain name reads as ABSENT —
/// the sentence no header at all gets, so `/api/me` answers `user: null` (the
/// client's state B) rather than the client drawing a refusal of the name. An
/// edge that asserts an identity asserts an address; anything else means the
/// edge in front is not the one this deployment expects.
///
/// A value the GRAMMAR refuses is refused before this, in the grammar's own
/// words, under every mode — as it was before this rule. `pub` for the Worker
/// module, whose `identity_of` takes these same two steps for a value no
/// header can carry (review R2-m1).
pub fn admit(mode: IdentityMode, user: User) -> Result<User, String> {
    match mode {
        IdentityMode::Open => Ok(user),
        IdentityMode::CfAccess | IdentityMode::Worker if user.is_email() => Ok(user),
        IdentityMode::CfAccess | IdentityMode::Worker => Err(absent(mode)),
    }
}

/// The 401's sentence when no caller reached the server, per mode.
fn absent(mode: IdentityMode) -> String {
    match mode {
        IdentityMode::Open => "no signed-in user: send X-Yokohama-Claimed-User".to_string(),
        IdentityMode::CfAccess => "no identity reached the server: this deployment expects \
                                   Cf-Access-Authenticated-User-Email from the Cloudflare tunnel"
            .to_string(),
        IdentityMode::Worker => "no identity reached the server: this deployment expects \
                                 X-Yokohama-User from the edge Worker"
            .to_string(),
    }
}
```

  `crates/yokohama-web/Cargo.toml:16-25`:

```toml
# M6.5b2 (R-6.5b2.3), renamed by M6.5c2 (R-6.5c2.3): `open-door` is the
# second feature, and it gates ONE thing — whether `YOKOHAMA_WEB_IDENTITY=open`
# parses. The open door is a supported self-host mode whose trust model is
# "anyone who can reach the server may use any name", so it stays a DEFAULT
# feature; a build that must not offer it drops it with
# `--no-default-features --features server` — the one spelling that keeps the
# server half and leaves only the two modes an edge asserts.
[features]
default = ["server", "open-door"]
server = ["dep:axum", "dep:tokio", "dep:tokio-stream"]
open-door = []
```

  The mechanical rename, and the three prose sites that name the mode in Rust:

```bash
perl -pi -e 's/IdentityMode::Claimed/IdentityMode::Open/g' crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs
grep -c 'IdentityMode::Open' crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs   # 21 and 1
grep -rn 'IdentityMode::Claimed' crates/   # nothing
```

  - `lib.rs:402`: `/// Backed by memory, in `open` mode — the mode every test drives with a`;
  - `main.rs:74`: `// - Under `open` the header is whatever the caller sends. It`;
  - `main.rs:90`: `// a deployment decision — `open` trusts anything past the door, and the`.

  **`Host.identity_name` needs no edit**: axum's host is built from `state.identity.name()` (`lib.rs:772`), so
  `/api/me`'s `mode` and `/api/health`'s `identity` read `open` from the renamed `name()`.

- [ ] **Step 4: Run it green**, in both build shapes:

```bash
cargo test -p yokohama-web --lib identity::
cargo test -p yokohama-web --no-default-features --features server --lib "identity::"
cargo test -p yokohama-web --lib
cargo test -p yokohama-web --test wire_fixtures
```

  Expected: `identity::` **8 passed** in BOTH shapes (the refusing build takes `open_parses_…`'s refusing branch and
  its trust-model sentence); the lib **+2** over Task 1's reading (D1 read 286); `wire_fixtures` FAILS as the next
  step's premise — `health.json — committed 239 bytes, server emits 236 bytes; first difference at byte 77`, `me.json
  — committed 51 … 48 … byte 13`, `me_signed_out.json — committed 40 … 37 … byte 13`.

- [ ] **Step 5: The module's fallback, first the failing cell** (spec §2.1 rule 4; review R2-m1).
  `crates/yokohama-worker/src/lib.rs`, directly before the doc at `:612` ("This host's four deployment facts…"):

```rust
    /// **The fallback follows the mode rule** (M6.5c2, spec §2.1 rule 4; review
    /// R2-m1). A value no header can carry never reaches `resolve` as a
    /// header, and the key grammar alone trims `"ben\r"` to the plain name
    /// `ben` — a key the `worker` door admits as nobody. So the fallback takes
    /// `resolve`'s two steps itself, and `"ben\r"` answers exactly what a
    /// header naming `ben` answers: the mode's absent sentence. An ADDRESS the
    /// same trim recovers is still that caller — the anti-vacuity, since a
    /// fallback that refused everything would pass the first half.
    #[test]
    fn a_value_no_header_can_carry_is_held_to_the_worker_modes_rule() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let absent = resolve(IdentityMode::Worker, &http::HeaderMap::new()).unwrap_err();
        let r = identity_inner(Some("ben\r"));
        assert_eq!((r.status, r.body.as_str()), (401, serde_json::json!({ "error": absent }).to_string().as_str()));
        let as_header = identity_inner(Some("ben"));
        assert_eq!((as_header.status, &as_header.body), (r.status, &r.body), "the carrier never decides who it is");
        let r = identity_inner(Some("Red@Wire.Test\r"));
        assert_eq!((r.status, r.body.as_str()), (200, r#"{"user":"red@wire.test"}"#));
    }
```

```bash
cargo test -p yokohama-worker a_value_no_header
```

  Expected (MEASURED): FAILS — `left: (200, "{\"user\":\"ben\"}")` / `right: (401,
  "{\"error\":\"no identity reached the server: this deployment expects X-Yokohama-User from the edge Worker\"}")`.

- [ ] **Step 6: Implement the fallback.** `crates/yokohama-worker/src/lib.rs:47`:
  `use yokohama_web::identity::{admit, resolve, IdentityMode, User, WORKER_HEADER};` — and `:302-321`:

```rust
/// Who the caller is, by the `worker` mode's ONE header (spec §2.3): the value
/// the shell hands over goes into a header map as `X-Yokohama-User` and is read
/// by the layer's own `identity::resolve` — so `anonymous`, like an absent or
/// blank value, is no caller; so is a plain name (the layer's mode rule,
/// M6.5c2); and a refusal is the layer's own sentence.
///
/// A value no header can carry — one holding a control character other than a
/// tab — never reaches `resolve`, so it takes `resolve`'s two steps here: the
/// key grammar (`User::parse`, trimming as `resolve` would and refusing any
/// control character that remains) and then the mode's rule (`identity::admit`),
/// so `"ben\r"` is nobody exactly as a header naming `ben` is (review R2-m1).
/// It is a 401 either way, with one difference in wording: a value whose only
/// such characters are at its ends, which the trim removes, and which trims to
/// nothing or to `anonymous` — `"anonymous\n"` — gets `User::parse`'s sentence,
/// where `resolve` would answer its absent-caller sentence had a header carried
/// it. No header can.
pub fn identity_of(user: Option<&str>) -> Result<User, String> {
    let mut headers = http::HeaderMap::new();
    if let Some(raw) = user {
        let Ok(value) = http::HeaderValue::from_str(raw) else {
            return User::parse(raw).and_then(|user| admit(IdentityMode::Worker, user));
        };
```

  (the rest of the function, `:322-325`, is unchanged). Cycle C replaces both `IdentityMode::Worker` in this function
  with the configured door.

- [ ] **Step 7: Run the module green.**

```bash
cargo test -p yokohama-worker
```

  Expected: **+1** over Task 1's reading (D1 read 62). `identity_answers_the_normalized_caller_or_the_worker_modes_401`'s
  DEL arm is unchanged and still green: a value the GRAMMAR refuses keeps the grammar's sentence, exactly as `resolve`
  keeps it.

- [ ] **Step 8: Regenerate the corpus and READ the diff.**

```bash
UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures
git status --short web/src/wire web/src/wire-script
git diff --stat web/src/wire web/src/wire-script
git diff web/src/wire | grep '^[-+] '
cargo test -p yokohama-web --test wire_fixtures
```

  **Expected, and nothing else** (MEASURED): three files, one line each —
  `health.json`: `-  "identity": "claimed",` / `+  "identity": "open",`; `me.json` and `me_signed_out.json`:
  `-  "mode": "claimed",` / `+  "mode": "open",`. `git diff --stat` reads `3 files changed, 3 insertions(+), 3
  deletions(-)`; the three request scripts are byte-identical (a script row carries no response); the check run then
  reads **12 passed**. Anything else is a FINDING: stop and report it.

- [ ] **Step 9: The live replay's script cell reads the corpus, and restates it.**
  `worker/scripts/live-replay.test.mjs:481-492` (`refuses an exemption the Worker did not need: a host field reading as
  the axum host's`) fakes the live health answer with the AXUM host's committed `identity`, to show the replay refuses
  a `HOST_FIELDS` exemption both hosts agree on. It spells that value as a literal, `"claimed"`, and the committed value
  is now `open`, so the fake no longer agrees with the corpus:

```bash
cd worker && npm run test:scripts
```

  Expected (MEASURED): **1 failed** — `scripts/live-replay.test.mjs > replay > refuses an exemption the Worker did
  not need: a host field reading as the axum host's` with `AssertionError: expected true to be false` (the replay
  HOLDS, because the "agreeing" field now differs). The repair, `:484`:

```js
        plan[planIndex(rows, "walk.json", 28)] = () => json(200, { ...liveBody("health"), identity: "open" });
```

```bash
npm run test:scripts
```

  Expected: all passed, the count unchanged (D1 read 352 in 18 files).

- [ ] **Step 10: The client's red.** The regenerated corpus now says `open` while the client tests `"claimed"`:

```bash
cd web && npm run typecheck && npm test
```

  Expected (MEASURED): the typecheck PASSES — `contract.ts` checks `Widen<Me>`, which widens the `mode` union to
  `string`, so no type fails; `npm test` **9 failed**, in 2 files:
  - `src/App.test.tsx > the door >`: `shows sign-in when the door is claimed and nobody is signed in, then the games
    list`; `draws the door and no sign-in at #/ when the server is up and nobody is signed in`; `a 401 mid-game returns
    to sign-in, carrying the sentence`; `a 401 from the Game menu's list returns to sign-in`; `a 401 from the replay
    scrubber returns to sign-in`; `a 401 from the final score returns to sign-in`; `a 401 from the saved-game list
    says why, rather than that you have no games`; `a 401 from the opening walk returns to sign-in`;
  - `src/GameMenu.test.tsx > the Game menu > carries the way out, beside the way to another game`.

- [ ] **Step 11: Rename the client's four literal sites, minimally** (the UI redesign is cycle B's):
  - `web/src/types.ts:486`: `mode: "open" | "cf-access" | "worker";` and `:483`: `… only `open` has a screen to
    offer,`;
  - `web/src/App.tsx:1295`: `const signOutButton = signedIn && door.me?.mode === "open" && (`, and `:1292`:
    `// The button exists under `open` ONLY. Under the two edge doors there is`;
  - `web/src/Door.tsx:45`: `if (door.me.mode === "open" && !door.me.user)`, and `:43`:
    `// 3. Nobody is signed in under `open` — the one door this browser can`;
  - `web/src/OnThisDevice.tsx:332-336`: `const open = useIdentityMode() === "open";` … `{open ? ", then sign in here
    with your email." : "."}`, and `:318`: `* What to DO about "not signed in" — and under `open` it is two steps, not`;
  - `web/src/OnThisDevice.test.tsx:429` and `:463`: `rememberServerFacts({ ...wire.health, identity: "open" });` —
    the component and its cell move TOGETHER, or the cell tests the other branch; `:422`: `// **"Not signed in" names
    BOTH doors under `open`** (spec §2.6): passing`; `:426`: `it("names both doors under open, and one under
    cf-access", async () => {`;
  - `web/src/App.test.tsx`: `:1916` `// flight, the sign-in form under an `open` deployment with nobody signed in,`;
    `:1921` `it("shows sign-in when the door is open and nobody is signed in, then the games list", async () => {`;
    `:2067` `it("names the door rather than offering a form when an edge-door deployment has no user", async () => {`;
    `:2069` `// The corpus carries only the `open` door — the fixture server runs in`.

  The storage key `yokohama.claimed_user`, `claimedUser()`, `CLAIMED_USER_KEY` and the header
  `x-yokohama-claimed-user` stay (spec §2.1: a browser signed in before the rename stays signed in).

```bash
cd web && npm run typecheck && npm test
```

  Expected: clean; all passed, the count unchanged (D1 read 1056).

- [ ] **Step 12: The shell's "character for character" sentences** (review R2-m8), and their twins:
  - `worker/src/identity.ts:37-46` (the doc above `isAddress`):

```ts
/**
 * Whether `email` — trimmed and ASCII-lowercased, as `address` answers it and
 * as `shellConfig` reads an operator — is an ADDRESS by the layer's email
 * shape (`User::is_email`, the mode rule's test): at most 254 bytes, no
 * whitespace or control character, and exactly one `@` with something on both
 * sides. `User::parse`'s key grammar also takes a plain name (M6.5c2), but
 * under `worker` the layer's mode rule admits an address and nothing else, so
 * a value outside this rule names no caller the layer would accept (R1's L-2)
 * and no operator one could match. Wherever a CALLER is read the value is
 * visible ASCII with tabs and spaces (`forwardable`), where this test and the
 * layer's email shape agree character for character.
 */
```

  - `worker/src/identity.unit.test.ts:227-228`: `// R1's L-2: the dev door's claimed value is an ADDRESS by the layer's
    email` / `// shape (`User::is_email`, the mode rule's test) — one `@` with something on both sides, no`; and
    `:246`: `// address, as it is to the layer's email shape.`
  - `worker/test/worker.test.ts:337-338`: `// R1's L-2, end to end: the dev door's claimed value must be an ADDRESS by`
    / `// the layer's email shape (`User::is_email`), or it is no one. Before, `not-an-email``;
  - `worker/src/config.ts:111-113`: ` * refused, and so is an entry that is no address by the layer's email shape` /
    ` * (`isAddress`, the mode rule's `User::is_email`): under `worker` the layer` / ` * admits no other caller, so
    the entry could match no one, and the operator` / ` * would meet a 403 that names nothing wrong. The`.

  The shell's BEHAVIOUR is unchanged: under `worker` its dev door still takes an address alone, now belt-and-braces
  with the layer's mode rule; cycle C adds the open door's own `claimed` identity kind.

- [ ] **Step 13: The mode's name in the remaining comments** (one word each; the header's and the storage key's
  names are left as they are):
  - `web/src/api.ts:71` `… under `open`, and two`; `:122` `… which only exists in `open` mode, and the server ignores
    this`; `:212` `… so the `open` door could not sign a stream. */`; `:505` `… two steps under `open` and one under the
    other`;
  - `web/src/sse.ts:4` `// it cannot carry a header. The `open` door is `X-Yokohama-Claimed-User` and`;
  - `web/src/ws.ts:11` `// carries no header of the page's choosing — neither the open door's nor`;
  - `web/src/signal.ts:25` `// `open` door is one. It is also the reason a hidden tab can let go at all,`;
  - `web/src/whoami.ts:104` `… how the player was supposed to arrive. `open` never reaches`;
  - `web/src/SignIn.tsx:1-3` `// The open door's sign-in.` / `//` / `// It exists under ONE deployment mode. `open` is
    the mode where the server` (cycle B rewrites the screen);
  - `web/src/test/harness.tsx:59` ` *  the `open` door. **The sentence is not what the client routes on** — the`;
  - `web/src/sse.test.ts:149` `    // `open` door is a header, and `EventSource` cannot carry one.`;
  - `web/src/local/corpus.node.test.ts:91` ` * reaches a browser — the axum host reads `open` and `sse`, the browser
    host`; `worker/test/corpus.test.ts:31` ` * from, `open`, `sse` and `off` — its primary `Api` keeps no archive.`;
    `worker/scripts/live-replay.mjs:81` ` * the axum host the corpus was cut from `open`, `sse` and `off`.` (all three
    replays' `HOST_FIELDS` exemptions still hold: the corpus says `open`, the browser host `local`, the Worker
    `cf-access` — each exempted field still DIFFERS, which each replay asserts);
  - `web/e2e/live.ts:230-231` ` * Locally it is NOTHING — exactly today's context. The local server's door is` /
    ` * `open`: the walk types the address into the sign-in form, and the client`; `:237` ` * draws one under `open`
    only), so it never holds an address and attaches`; `web/e2e/joins_by_link.spec.ts:141` `// `cf-access`, and
    `App.tsx` draws the button under `open` ONLY. So`;
  - `deploy/Dockerfile:203` `# their own edge can set, and `open` authenticates nobody: whoever reaches the door may
    take any name.`; `:210-211` `# naming the three accepted values. The NAS sets it on the container` /
    `# (`RUNBOOK.md` §3 records the value).` — no longer naming `claimed`, which an image built from this commit
    refuses; `deploy/Dockerfile.runtime:48-49` `# `worker` trust a header only their own edge can set, and `open`
    authenticates` / `# nobody: whoever reaches the door may take any name. So binding wide is still a statement`.
  - **Left, with its reason:** `deploy/cloudflared-config.yml:58` describes the DEPLOYED NAS, which runs `claimed`
    until cycle C's close; cycle C's documents own it.

- [ ] **Step 14: The walk's server runs `open`, and the e2e cells** (owed: the walk's server configuration and the
  client's door literal both moved). `web/playwright.config.ts:242`:
  `// `open` is the self-host door and this walk's — the client attaches the`; `:246`:
  `YOKOHAMA_WEB_IDENTITY: "open",`. The harness builds the binary itself (`command: "cargo run --release -p
  yokohama-web"`, `:229`), so it cannot run a stale build that refuses `open`.

```bash
cd web && npx tsc -p tsconfig.node.json && npm run e2e
```

  Expected: all passed (CLAUDE.md's reading is 61 in 11 spec files; 6.5–15 min; one Playwright run at a time). The
  cells that prove the rename end to end are every spec that signs in through `harness.ts`'s `signIn` — the sign-in
  screen is drawn only when the real server under `open` answers `/api/me` with `mode: "open"`. Record the run's line
  in the ledger. The generated DEBUG module is left in `web/dist` afterwards (CLAUDE.md), which no later step here reads.

- [ ] **Step 15: CI names the feature, the cell and the count, 7 → 8.** `.github/workflows/test.yml:220-229`:

```yaml
      # M6.5b2 (R-6.5b2.3; the feature renamed `open-door` by M6.5c2's
      # R-6.5c2.3): the REFUSING build shape, which nothing compiled before this
      # line. `--no-default-features --features server` is the one spelling
      # that keeps the server half and drops `open-door`, and the whole of the
      # rule is that it compiles AND refuses `YOKOHAMA_WEB_IDENTITY=open` by
      # name, stating the trust model. The wasm check above cannot see it: that
      # shape drops `server` too, so the identity module's own cells never run
      # there. Eight cells, one of which
      # (`open_parses_only_where_the_open_door_feature_is_compiled_in`) takes
      # its refusing branch only here; the other seven prove the key grammar and
      # the mode rule still work without the feature.
```

  `:246`: `- name: the refusing build, and that its eight identity cells really ran`; `:256-257`:
  `echo "$out" | grep -q "test result: ok. 8 passed" || {` / `echo "::error::the identity:: filter did not run eight
  cells — R-6.5b2.3's only CI witness"`.

```bash
python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/test.yml
cargo test -p yokohama-web --no-default-features --features server --lib "identity::" 2>&1 | grep "test result"
```

  Expected: the YAML loads; `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 278 filtered out`.

- [ ] **Step 16: The sweep's grep, AFTER the workflow is rewritten** (P-11: run before Step 15, it also matched
  `test.yml:224`'s "refuses `YOKOHAMA_WEB_IDENTITY=claimed` by name"):

```bash
grep -rn 'claimed`\|"claimed"\|claimed mode\|claimed door' web/src web/e2e worker/src worker/test worker/scripts \
  crates/yokohama-web crates/yokohama-worker crates/yokohama-wasm deploy/Dockerfile deploy/Dockerfile.runtime \
  web/playwright.config.ts .github/workflows/test.yml \
  | grep -v 'node_modules\|\.gen\.ts\|claimedUser\|CLAIMED_USER_KEY\|claimed_user\|claimed-user\|Claimed-User\|claimed header\|claimed user\|claimed name\|claimed value\|the claimed='
```

  Expected (MEASURED, exactly two lines): `crates/yokohama-web/src/identity.rs:10` (the module doc's "superseding
  R-1.4's framing of `claimed`") and the no-alias cell's `IdentityMode::from_str("claimed")`. The gitignored
  `worker/test/corpus.gen.ts` still spells `claimed` until the pool's `pretest` regenerates it from the corpus; the
  fenced CLI's own "claimed" (achievement slots) is outside the grep by construction. A third line is a site this task
  missed: fix it, and say so.

- [ ] **Step 17: The per-edit gates** (Global Constraints). Counts that move, as deltas over Task 1's reading:
  - `cargo test -p yokohama-web` — lib **+2** (D1 read 302 = 286 + 2 + 2 + 12); `cargo test -p yokohama-wasm` **±0**
    (the browser host names its own door, `local`, and calls `User::parse` directly — no mode rule, spec §2.1 rule 4);
    `cargo test -p yokohama-worker` **+1**;
  - the public build's `identity::` — **8**;
  - `cargo clippy --workspace --all-targets -- -D warnings` — clean; and the refusing shape's own lint,
    `cargo clippy -p yokohama-web --no-default-features --features server --all-targets -- -D warnings` — clean (the
    `not(open-door)` arm compiles only there);
  - the wasm32 `cargo check` and both wasm32 builds;
  - `cargo doc -p yokohama-worker --no-deps` — **0** warnings;
  - `cargo test -p yokohama-cli` — **re-count at HEAD** (this diff reaches a `Cargo.toml`, the layer's own; the CLI
    does not depend on the layer, so nothing the CLI compiles moved — the run is the constraint's letter);
  - web: `npm run build:wasm`, `npm run typecheck`, `npm test` (±0), `npm run test:wasm` (±0); worker:
    `npm run build:module:debug` FIRST, then `npm run typecheck`, `npm run test:scripts` (±0), `npm test` (±0);
  - `npm run e2e` — Step 14's run;
  - the fence prints nothing;
  - **the reviewer reads** `git diff 1e16fe27 -- crates/yokohama-web/src` against Task 1's list plus exactly:
    `identity.rs` (the rename, the mode rule, two rewritten cells, two renamed cells, one new cell), `lib.rs` (21
    renames, one doc, the `/api/me` and health cells, one new cell), `main.rs` (two comments).

- [ ] **Step 18: Commit**, quoting Step 2's, Step 5's, Step 9's and Step 10's red runs in the body:

```bash
git add crates/yokohama-web/src/identity.rs crates/yokohama-web/Cargo.toml crates/yokohama-web/src/lib.rs \
  crates/yokohama-web/src/main.rs crates/yokohama-web/tests/wire_fixtures.rs crates/yokohama-worker/src/lib.rs \
  web/src/wire/health.json web/src/wire/me.json web/src/wire/me_signed_out.json \
  web/src/types.ts web/src/App.tsx web/src/Door.tsx web/src/OnThisDevice.tsx web/src/OnThisDevice.test.tsx \
  web/src/App.test.tsx web/src/api.ts web/src/sse.ts web/src/ws.ts web/src/signal.ts web/src/whoami.ts \
  web/src/SignIn.tsx web/src/test/harness.tsx web/src/sse.test.ts web/src/local/corpus.node.test.ts \
  web/playwright.config.ts web/e2e/live.ts web/e2e/joins_by_link.spec.ts worker/src/identity.ts \
  worker/src/identity.unit.test.ts worker/test/worker.test.ts worker/src/config.ts worker/test/corpus.test.ts \
  worker/scripts/live-replay.test.mjs worker/scripts/live-replay.mjs deploy/Dockerfile deploy/Dockerfile.runtime \
  .github/workflows/test.yml
git commit -m "feat(web): the claimed mode becomes the open door, and the edge doors read a plain name as nobody"
git status --short   # clean
```

  RUNBOOK and `CLAUDE.md` name `claimed` and `dev-identity` too; they are cycle C's documents, not this task's.

---

### Task 3: The bot's label, its contract file, the mirror, and the two wire fields (spec §2.4 "The label", "The current production bot", "The wire gains two fields", "Where the lobby gets it"; review M1, m1, m7, m12, R2-m6)

**Measured:** D2 ran every step in an export over HEAD (Tasks 1–2 not applied there; nothing here depends on them);
the review's chain carries it in `0002`–`0003` and ran the pool over it. Two changes from D2's text: the pool fixture's
mirror carries the record's bot (P-4, measured green in the review's bisect), and the `THIRD_COLUMN` literal below
includes Task 1's `identity_cases` (P-8).

**Files:**
- Create: `crates/yokohama-web/src/bots.rs` — `BotDescriptor`, `TableBot`, `SeatBot`, `production`, `label`, `recorded`,
  `table_bot`, `seat_bots`, and four cells.
- Modify: `crates/yokohama-web/src/lib.rs:61` — `pub mod bots;` after `pub mod backup;` (ungated: the browser and
  Worker modules need it).
- Modify: `crates/yokohama-web/src/table.rs`:
  - `:83-91` — `TurnMirror` loses `Copy`, gains `bot`;
  - `:389-392` — the three mirror reads by reference, and `bot:` in `view`;
  - `:453-454` — `TableView.bot`;
  - tests: `:703` (by reference, plus a `bot` assertion), `:742` (the literal gains `bot: None`), a new cell before `:783`.
- Modify: `crates/yokohama-web/src/tables.rs:427` — `check_shape`'s destructure by reference; test literals `:2725`,
  `:2736`, `:5089` gain `bot: None`.
- Modify: `crates/yokohama-web/src/games.rs:98-105` (`mirror_of` fills `bot`), `:771` (`mirror_turn` compares by
  reference); a new cell before `:946`.
- Modify: `crates/yokohama-web/src/client.rs:182-190` (`ClientState.bots`), `:499` (built); a new cell before `:778`.
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs`:
  - `:747-757` — `Walk.aside`'s doc names the third second-`Api`;
  - `:1762-1767` — `walk_with`'s tail gains the vs-bot `Api` and its `table_bot` payload, and a new `bot_table_body()`;
  - `:2149` — `corpus()` pushes `production_bot`; a new `production_bot()` before `:2299`.
- Create (generated, committed): `web/src/wire/production_bot.json`, `web/src/wire/table_bot.json`. Modify (generated):
  31 corpus files (the expected diff, Step 7).
- Modify: `web/src/types.ts:345-348` (`ClientState.bots`, and `SeatBot`, `TableBot`, `ProductionBot` after it),
  `:568-569` (`TableView.bot`).
- Modify: `web/src/wire/contract.ts` — imports (`:85`, `:88`, `:91`, `:106`), assertions after `:342`, key lists
  (`:539-543`, `:589-593`), `KeysOf` after `:690`, the two `void` lists (`:483-503`, `:691-696`), `KEYS` (`:744`),
  `wire` (`:861`).
- Modify: `web/src/wire/contract.test.ts:172` — four key-set rows.
- Modify: `web/src/local/corpus.node.test.ts:137-162` — `NOT_SERVED` names the two new files, with their reason.
- Modify: `worker/test/corpus.test.ts:50-66` — `THIRD_COLUMN` likewise (sorted).
- Modify: `worker/test/fixtures/owed-game-over.table.json:29-33` — the mirror gains `bot` (P-4), as TEXT.

**Interfaces:**
- Consumes: nothing from Tasks 1–2.
- Produces (shared names): `bots::BotDescriptor { controller: String, version: u32, params: BTreeMap<String, i64> }`
  (`Serialize, Deserialize, Clone, PartialEq, Eq, Debug`); `bots::production() -> BotDescriptor`;
  `bots::label(d: &BotDescriptor) -> String`; `bots::recorded(record: &GameRecord) -> Vec<(u8, BotDescriptor)>`;
  `TurnMirror.bot: Option<BotDescriptor>`; `TableView.bot: Option<TableBot>`, `TableBot { label: String, runs_here:
  bool }`; `ClientState.bots: Vec<SeatBot>`, `SeatBot { seat: u8, label: String }`; `web/src/wire/production_bot.json`
  = `{"identity": "v4-step9-draft1", "label": "V4"}`; `bots::table_bot`, `bots::seat_bots` (`pub(crate)`);
  `web/src/wire/table_bot.json`; TS `SeatBot`, `TableBot`, `ProductionBot`; `wire.tableBot`, `wire.productionBot`;
  `KEYS.tableBot`, `KEYS.seatBot`, `KEYS.productionBot`.
- **The screens do not import the contract file here.** Cycle B's door, local form and table form import
  `web/src/wire/production_bot.json` directly (a Vite JSON import, typed by `ProductionBot`) — never through
  `contract.ts`, which would pull the whole corpus into the bundle.

**The labels, every one named** (`bots::label_of`, exhaustive at both levels; D2's A6):

| contestant (`crates/yokohama-core/src/game/arena/stats.rs:151-190`, `NamedIdentity` `:219-234`) | label |
|---|---|
| `NamedTable(NamedIdentity::V4Step9Draft1)` — the production bot since 2026-08-30 | `V4` |
| `NamedTable(NamedIdentity::V2Padded)` — `WeightTable::V2`, named | `V2` |
| `NamedTable(NamedIdentity::V2RefitPadded)` — a research refit, never shipped | `V2 refit` |
| `LookaheadV1 { .. }` — the V2-era production seat, grandfathered on `WeightTable::V2` | `V2 lookahead` |
| `GreedyV1 { .. }` | `V2 greedy` |
| `AtomicDiag { .. }` | `atomic diagnostic` |
| `MctsV1 { .. }` | `MCTS diagnostic` |
| `Policy(RolloutPolicy::Uniform / RetreatAverse / FamilyWeighted { .. })` | `uniform rung` / `retreat-averse rung` / `family-weighted rung` |
| a descriptor `from_parts` refuses | its own `controller` |

- [ ] **Step 1: Write the failing cells.**

`crates/yokohama-web/src/lib.rs:61`, after `pub mod backup;`:

```rust
/// The bot's label, and whether this build runs it (M6.5c2, R-6.5c2.10).
pub mod bots;
```

Create `crates/yokohama-web/src/bots.rs` holding, for now, only its test module (Step 3 writes the items above it):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use yokohama_cli::bot::BotSeats;

    fn descriptor_of(c: Contestant) -> BotDescriptor {
        let (controller, version, params) = c.descriptor();
        BotDescriptor { controller, version, params }
    }

    /// `production()` is the triple a bot seat is DEALT under — the provenance
    /// `finalize` stamps through `BotSeats::seat_provenance` — so a game this
    /// build deals always runs here.
    #[test]
    fn the_production_bot_is_the_triple_every_bot_seat_is_dealt_under() {
        let dealt = BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT)], 7).seat_provenance();
        let seat = &dealt[0];
        assert_eq!(
            production(),
            BotDescriptor { controller: seat.controller.clone(), version: seat.version, params: seat.params.clone() },
        );
    }

    /// Every identity core can name has a label, V4 is "V4", and no two
    /// labels are one word — the compiler holds the first (the `match` has no
    /// `_` arm) and this cell holds the words.
    #[test]
    fn every_identity_is_labelled_and_the_labels_are_distinct() {
        let table = [
            (Contestant::NamedTable(NamedIdentity::V4Step9Draft1), "V4"),
            (Contestant::NamedTable(NamedIdentity::V2Padded), "V2"),
            (Contestant::NamedTable(NamedIdentity::V2RefitPadded), "V2 refit"),
            (Contestant::LOOKAHEAD_V1_DEFAULT, "V2 lookahead"),
            (Contestant::GREEDY_V1_DEFAULT, "V2 greedy"),
            (Contestant::ATOMIC_DIAG_DEFAULT, "atomic diagnostic"),
            (Contestant::MCTS_V1_DEFAULT, "MCTS diagnostic"),
            (Contestant::Policy(RolloutPolicy::Uniform), "uniform rung"),
            (Contestant::Policy(RolloutPolicy::RetreatAverse), "retreat-averse rung"),
            (Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT), "family-weighted rung"),
        ];
        // Every named identity is in the table: `NamedIdentity::ALL` is core's
        // own list, so a fourth arm fails HERE as well as at the `match`.
        for arm in NamedIdentity::ALL {
            assert!(table.iter().any(|(c, _)| *c == Contestant::NamedTable(arm)), "{arm:?} has no row");
        }
        for (contestant, want) in table {
            // Through the RECORD's halves, as a stored game reaches `label`.
            assert_eq!(label(&descriptor_of(contestant)), want, "{}", contestant.echo());
        }
        let words: std::collections::BTreeSet<&str> = table.iter().map(|(_, w)| *w).collect();
        assert_eq!(words.len(), table.len(), "two identities read as one label");
    }

    /// A descriptor this build cannot parse answers its controller name.
    #[test]
    fn an_unparseable_descriptor_answers_its_controller() {
        let unknown = BotDescriptor { controller: "bga-human".into(), version: 1, params: BTreeMap::new() };
        assert_eq!(label(&unknown), "bga-human");
        let mut bad_knob = descriptor_of(Contestant::LOOKAHEAD_V1_DEFAULT);
        bad_knob.params.insert("b".into(), 0);
        assert_eq!(label(&bad_knob), "lookahead-v1", "`from_parts` refuses b=0, so the name stands");
    }

    /// `recorded` is the record's non-human seats, in seat order.
    #[test]
    fn recorded_names_the_record_s_bot_seats_in_order() {
        use yokohama_core::{GameConfig, Rules, StartingOrderChoice};
        let rules = Rules::standard();
        let mut record = GameRecord::new(
            7,
            GameConfig::for_player_count(2),
            vec![StartingOrderChoice::Keep(0); 2],
            &rules,
        );
        assert!(recorded(&record).is_empty(), "no run provenance: no bot");
        record.set_run_seats(BotSeats::new(vec![None, None], 7).seat_provenance());
        assert!(recorded(&record).is_empty(), "an all-human game");
        record.set_run_seats(BotSeats::new(vec![None, Some(PROVISIONAL_DEFAULT_BOT)], 7).seat_provenance());
        assert_eq!(recorded(&record), vec![(1, production())]);
        assert_eq!(seat_bots(&record), vec![SeatBot { seat: 1, label: "V4".into() }]);
    }
}
```

`crates/yokohama-web/src/table.rs`, in the test module, directly before the cell at `:783` (`/// The view carries the
BOOLEAN and never the list…`):

```rust
    /// **The view names the table's bot** (M6.5c2, R-6.5c2.10): an undealt
    /// table's is the one this build deals, a dealt one's is its mirror's — the
    /// record's — and `null` where there is no bot seat, or where a dealt
    /// table's mirror predates the field (review m7).
    #[test]
    fn the_view_names_the_bot_undealt_dealt_foreign_and_before_the_field() {
        use serde_json::{json, Value};
        let reader = User::parse("a@t.test").expect("an email");
        let bot_of = |d: &TableDoc| serde_json::to_value(d.view(&reader)).expect("a view")["bot"].clone();
        let production = crate::bots::production();
        let runs_here = json!({ "label": crate::bots::label(&production), "runs_here": true });

        let mut undealt = playing_doc();
        undealt.status = TableStatus::Dealing;
        undealt.keeps = vec![None, None];
        assert_eq!(bot_of(&undealt), runs_here, "undealt: the bot `finalize` will deal");

        let mut dealt = playing_doc();
        dealt.turn = Some(TurnMirror { to_move: 0, over: false, winner: None, bot: Some(production) });
        assert_eq!(bot_of(&dealt), runs_here, "dealt: the record's, from the mirror");

        let mut foreign = dealt.clone();
        foreign.turn.as_mut().and_then(|t| t.bot.as_mut()).expect("a bot").controller = "v2-padded".into();
        assert_eq!(bot_of(&foreign), json!({ "label": "V2", "runs_here": false }), "another bot: named, not run here");

        let mut older = dealt.clone();
        older.turn.as_mut().expect("a mirror").bot = None;
        assert_eq!(bot_of(&older), Value::Null, "a mirror written before the field names no version");

        let mut humans = dealt;
        humans.seats[1] = SeatDoc::Human { user: "b@t.test".into(), name: None };
        assert_eq!(bot_of(&humans), Value::Null, "no bot seat, whatever the mirror says");
    }
```

`crates/yokohama-web/src/games.rs`, in the test module, directly before `:946` (`/// A record this server writes carries
the terminal digest…`):

```rust
    /// **The mirror carries the record's bot from the deal on, and a persist
    /// refreshes it** (M6.5c2, R-6.5c2.10) — a lobby row's only source for a
    /// dealt table's label. An all-human table's mirror has none, and its
    /// stored document does not spell the key.
    #[test]
    fn the_mirror_carries_the_records_bot_from_the_deal_and_a_persist_restores_it() {
        let games = games();
        let red = who("red@t.test");
        let handle = playing(&games, &req("vsbot"), &["red@t.test"]);
        let mirror = games.table("vsbot", &red).unwrap().turn.expect("a dealt table's mirror");
        assert_eq!(mirror.bot, Some(crate::bots::production()), "the deal wrote the record's bot");

        // A mirror written before the field, as every table dealt before this
        // deploy has one.
        let (mut doc, version) = games.read_table("vsbot").unwrap().unwrap();
        doc.turn.as_mut().expect("a mirror").bot = None;
        games.write_table(&doc, Some(version)).unwrap();
        assert_eq!(games.table("vsbot", &red).unwrap().view(&red).bot, None, "premise: no version to name");

        // Red's next move is persisted, and the persist puts the bot back.
        {
            let mut repl = handle.lock().unwrap();
            let before = repl.session.record().events.len();
            repl.step("1");
            assert!(repl.session.record().events.len() > before, "premise: the step applied a move");
            games.persist("vsbot", &repl).unwrap();
        }
        let mirror = games.table("vsbot", &red).unwrap().turn.expect("a mirror");
        assert_eq!(mirror.bot, Some(crate::bots::production()), "the persist refreshed it");

        playing(&games, &two_people("humans", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        assert_eq!(games.table("humans", &red).unwrap().turn.expect("a mirror").bot, None);
        let (text, _) = games.store().read(Doc::Table, "humans").unwrap().unwrap();
        let stored: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(stored["turn"].get("bot").is_none(), "an all-human table's document spells no `bot`: {text}");
    }
```

`crates/yokohama-web/src/client.rs`, in the test module, directly before `:778` (`/// The three arms of
[`viewer_seat`]…`):

```rust
    /// **`ClientState.bots` is the record's bot seats, labelled** (M6.5c2,
    /// R-6.5c2.10): empty for an all-human game, the bot's seat and label for a
    /// vs-bot one.
    #[test]
    fn the_payload_names_the_records_bot_seats() {
        use yokohama_cli::bot::PROVISIONAL_DEFAULT_BOT;
        let humans = two_human_repl();
        let payload = ClientState::of("t", &humans, Viewer::Seat(PlayerId::new(0)), Vec::new());
        assert!(payload.bots.is_empty(), "an all-human game names no bot");

        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let mut session = Session::new_sim(config, 20_260_831, vec![StartingOrderChoice::Keep(0); 2], Rules::standard())
            .expect("a game");
        session.set_run_seats(BotSeats::new(vec![None, Some(PROVISIONAL_DEFAULT_BOT)], 7).seat_provenance());
        let vs_bot = Repl::new(session);
        let payload = ClientState::of("t", &vs_bot, Viewer::Seat(PlayerId::new(0)), Vec::new());
        assert_eq!(
            serde_json::to_value(&payload.bots).expect("serializable"),
            serde_json::json!([{ "seat": 1, "label": crate::bots::label(&crate::bots::production()) }]),
        );
    }
```

- [ ] **Step 2: Watch them fail.**

```bash
cargo test -p yokohama-web --lib bots::
```

Expected: COMPILE failure — `bots.rs` names `production`, `label`, `recorded`, `seat_bots`, `BotDescriptor`, `SeatBot`,
`PROVISIONAL_DEFAULT_BOT`, `Contestant`, `NamedIdentity`, `RolloutPolicy`, `GameRecord` and `BTreeMap`, none in scope;
`table.rs`'s new cell writes `bot:` on `TurnMirror` and reads `.bot` on `TableView`, and `client.rs`'s reads
`payload.bots`, none of which exist.

- [ ] **Step 3: The module.** In `bots.rs`, ABOVE the test module:

```rust
//! **Which bot played a game, in the words a person reads** (M6.5c2,
//! R-6.5c2.10).
//!
//! A bot seat's identity is the RECORD's: every `SeatProvenance` carries its
//! controller's name, that controller's version and every knob, and
//! `Contestant::from_parts` is the inverse that reads the name and the knobs
//! back. This module adds the two things a record cannot say for itself: a
//! SHORT LABEL for each identity (`v4-step9-draft1` is "V4"), and whether THIS
//! build runs it.
//!
//! **The labels are the layer's own words**, like `suggest`'s four authored
//! refusals, and they are total by CONSTRUCTION: [`label`] reads a
//! `Contestant` through an exhaustive `match` with no `_` arm at either level —
//! the CLI's `bot::eligible` discipline — so the day core gains a V5 identity
//! this crate stops compiling until V5 has a label. That one line is the only
//! web edit a new bot needs; everything else here follows
//! `bot::PROVISIONAL_DEFAULT_BOT`.
//!
//! **Nothing here stores a bot.** `SeatDoc::Bot` still does not say which one
//! (`table.rs`): a dealt table's is the record's, cached in the turn mirror
//! ([`crate::table::TurnMirror::bot`]), and an undealt table's is the one
//! `finalize` will deal, [`production`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use yokohama_cli::bot::PROVISIONAL_DEFAULT_BOT;
use yokohama_core::{Contestant, GameRecord, NamedIdentity, RolloutPolicy, CONTROLLER_HUMAN};

use crate::table::{SeatDoc, TableDoc, TableStatus};

/// One bot seat's identity as its record names it: a `SeatProvenance`'s
/// `(controller, version, params)`, without the stream seed (a fact about one
/// game, not about the bot) and the observation channel.
///
/// **The triple, not the name** (review m12): one name can ship at two
/// controller versions or two knob sets, and [`TableBot::runs_here`] compares
/// what a record would have to match to resume under this build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotDescriptor {
    pub controller: String,
    pub version: u32,
    #[serde(default)]
    pub params: BTreeMap<String, i64>,
}

/// The table's bot on the wire (`TableView.bot`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableBot {
    /// [`label`]'s answer.
    pub label: String,
    /// Whether this build runs that bot: the descriptor equals [`production`].
    pub runs_here: bool,
}

/// One bot seat of a game on the wire (`ClientState.bots`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SeatBot {
    pub seat: u8,
    pub label: String,
}

/// The bot this build deals — `bot::PROVISIONAL_DEFAULT_BOT` — as a record
/// names it.
///
/// Read through `Contestant::descriptor`, the function `BotSeats::seat_provenance`
/// writes every bot seat through, so a game dealt here compares equal to this
/// by construction, and a re-point of the const re-points this.
pub fn production() -> BotDescriptor {
    let (controller, version, params) = PROVISIONAL_DEFAULT_BOT.descriptor();
    BotDescriptor { controller, version, params }
}

/// The label a person reads for `d`.
///
/// Parsed through `Contestant::from_parts` over the record's two stored halves,
/// as the CLI's resume path reads them, and named by [`label_of`]. A
/// descriptor this build cannot parse — a name it does not know, a knob out of
/// range — answers its own controller name: an honest label for a bot nothing
/// here can describe better.
pub fn label(d: &BotDescriptor) -> String {
    match Contestant::from_parts(&d.controller, &d.params) {
        Ok(contestant) => label_of(contestant).to_string(),
        Err(_) => d.controller.clone(),
    }
}

/// **Exhaustive, with no `_` arm at either level.** See the module doc.
fn label_of(contestant: Contestant) -> &'static str {
    match contestant {
        Contestant::NamedTable(arm) => match arm {
            // The shipped production identity since 2026-08-30.
            NamedIdentity::V4Step9Draft1 => "V4",
            // `WeightTable::V2` — the table the V2-era bot played — named.
            NamedIdentity::V2Padded => "V2",
            // A research refit of V2's table, never shipped.
            NamedIdentity::V2RefitPadded => "V2 refit",
        },
        // The V2-era production seat (the CLI's default until the V4 ship) and
        // its one-ply sibling, both grandfathered on `WeightTable::V2`.
        Contestant::LookaheadV1 { .. } => "V2 lookahead",
        Contestant::GreedyV1 { .. } => "V2 greedy",
        // The two labelled diagnostics: reachable, never dealt to anyone.
        Contestant::AtomicDiag { .. } => "atomic diagnostic",
        Contestant::MctsV1 { .. } => "MCTS diagnostic",
        // The preserved rollout rungs, which sample a move rather than value it.
        Contestant::Policy(rung) => match rung {
            RolloutPolicy::Uniform => "uniform rung",
            RolloutPolicy::RetreatAverse => "retreat-averse rung",
            RolloutPolicy::FamilyWeighted { .. } => "family-weighted rung",
        },
    }
}

/// Every seat `record` names a non-human controller for, in seat order, with
/// its descriptor. Empty for an all-human game and for a record with no run
/// provenance. A seat index past `u8` is skipped: no record this layer writes
/// has one, and `recorded` is read before `Session::from_record` validates.
pub fn recorded(record: &GameRecord) -> Vec<(u8, BotDescriptor)> {
    let Some(run) = &record.run else { return Vec::new() };
    run.seats
        .iter()
        .enumerate()
        .filter(|(_, s)| s.controller != CONTROLLER_HUMAN)
        .filter_map(|(i, s)| {
            let seat = u8::try_from(i).ok()?;
            Some((
                seat,
                BotDescriptor { controller: s.controller.clone(), version: s.version, params: s.params.clone() },
            ))
        })
        .collect()
}

/// `TableView.bot` for `doc` (spec §2.4, "Where the lobby gets it").
///
/// - no bot seat: `None`;
/// - undealt (`open` or `dealing`): the production bot, which is what
///   `finalize` will deal, so it runs here;
/// - dealt: the mirror's descriptor — the record's, cached — or `None` for a
///   table whose mirror was last written before the field existed (its row
///   reads "the bot" with no version; review m7).
pub(crate) fn table_bot(doc: &TableDoc) -> Option<TableBot> {
    if !doc.seats.iter().any(|s| matches!(s, SeatDoc::Bot)) {
        return None;
    }
    let bot = match doc.status {
        TableStatus::Open | TableStatus::Dealing => production(),
        TableStatus::Playing => doc.turn.as_ref()?.bot.clone()?,
    };
    Some(TableBot { label: label(&bot), runs_here: bot == production() })
}

/// `ClientState.bots`: every bot seat the record names, labelled.
pub(crate) fn seat_bots(record: &GameRecord) -> Vec<SeatBot> {
    recorded(record).into_iter().map(|(seat, d)| SeatBot { seat, label: label(&d) }).collect()
}
```

`from_parts` is total (every knob helper answers `Err`, none panics — `stats.rs:1012-1100`), so a hand-edited mirror
descriptor cannot panic `view` under a table's lock, and `check_shape` gains no rule for it.

- [ ] **Step 4: The mirror, the view and the payload.**

`table.rs:83-91` becomes:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnMirror {
    pub to_move: u8,
    pub over: bool,
    /// The ENGINE's winner once `over` — the same value `/result` projects as
    /// `Placing.won`, never `standings[0]` (rank order; `outcome.rs:71`).
    #[serde(default)]
    pub winner: Option<u8>,
    /// The RECORD's bot (M6.5c2, R-6.5c2.10): the first non-human seat's
    /// `(controller, version, params)`, so a lobby row can label a dealt
    /// table's bot, and say whether this build runs it, without rebuilding the
    /// session that knows. `None` for an all-human game — and for a table whose
    /// mirror was last written before this field, which reads "the bot" with no
    /// version until its next applied move (a finished one never gains it; no
    /// persist follows game over — review m7).
    ///
    /// **Absent from the document when `None`**, so an all-human table's stored
    /// bytes are what they were; `#[serde(default)]` reads a document written
    /// before it, and neither `TableDoc` nor this type carries
    /// `deny_unknown_fields`, so an older binary reads one written after it —
    /// `TABLE_SCHEMA` stays 3. The field is also why this type is no longer
    /// `Copy`: its readers take a reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot: Option<crate::bots::BotDescriptor>,
}
```

(`skip_serializing_if` is D2's A2, measured: with it, this task's regeneration leaves `web/src/wire-script/`
byte-identical — `archive-walk.json`'s table bodies re-serialize the walk's all-human table — and every stored
all-human table keeps its bytes. Without it every stored table gains `"bot": null` and `archive-walk.json` moves here
as well as in Task 5.)

`table.rs:389-392` (inside `view`) become, with the new field after `can_watch`:

```rust
            to_move: self.turn.as_ref().map(|t| PlayerId::new(t.to_move)),
            over: self.turn.as_ref().map(|t| t.over),
            locked: self.lock.is_some(),
            winner: self.turn.as_ref().and_then(|t| t.winner).map(PlayerId::new),
            can_watch: self.can_watch(user),
            bot: crate::bots::table_bot(self),
```

`TableView` (`table.rs:447-454`) gains, after `can_watch`:

```rust
    /// The table's bot, or `null` (M6.5c2, R-6.5c2.10): its label and whether
    /// this build runs it — [`crate::bots::table_bot`]'s answer. An undealt
    /// table's is the production bot, which `finalize` will deal; a dealt one's
    /// is the record's, read off [`TurnMirror::bot`]; `null` at a table with no
    /// bot seat, and at a dealt one whose mirror predates the field.
    pub bot: Option<crate::bots::TableBot>,
```

`table.rs:703` (the schema-1 cell) becomes two lines — the fifth by-value site, and the old-document half of A2:

```rust
        assert_eq!(doc.turn.as_ref().expect("a mirror").winner, None);
        assert_eq!(doc.turn.as_ref().expect("a mirror").bot, None, "a mirror written before the field reads none");
```

`table.rs:742`: `d.turn = Some(TurnMirror { to_move: 1, over: true, winner: Some(2), bot: None });`

`tables.rs:427`:

```rust
    if let Some(&TurnMirror { to_move, winner, .. }) = doc.turn.as_ref() {
```

`tables.rs:2725`, `:2736`, `:5089`: each `TurnMirror { … }` literal gains `, bot: None` before its `}`.

`games.rs:98-105` (`mirror_of`):

```rust
pub(crate) fn mirror_of(repl: &Repl) -> TurnMirror {
    let over = repl.session.is_over();
    TurnMirror {
        to_move: repl.session.state().current_player.0,
        over,
        winner: over.then(|| repl.session.final_result().winner.0),
        // The record's first bot seat (M6.5c2, R-6.5c2.10). Every bot a web
        // table deals is one contestant (`tables::contestants_of`), so the
        // first names them all.
        bot: crate::bots::recorded(repl.session.record()).into_iter().next().map(|(_, d)| d),
    }
}
```

`games.rs:771` (`mirror_turn`) — the sixth site (D2's A3: `if doc.turn == Some(turn)` MOVES `turn`, and `:774` uses it
— E0382, measured):

```rust
        if doc.turn.as_ref() == Some(&turn) {
```

`mirror_of` is what `persist` refreshes through (`mirror_turn`), what `finalize` flips the table with
(`tables.rs:1230`, `:1244`) and what `settle_locked` repairs with (`:1196`), so the deal writes `bot` at once and every
applied move keeps it.

`client.rs:182-190` — `ClientState` gains, after `events`:

```rust
    /// Every bot seat the RECORD names, labelled (M6.5c2, R-6.5c2.10) — empty
    /// for an all-human game. From the record's `run.seats`, not from the
    /// resident `Repl`'s bot table.
    pub bots: Vec<crate::bots::SeatBot>,
```

and `client.rs:499`, after `events: …`, in `ClientState::of`:

```rust
            bots: crate::bots::seat_bots(session.record()),
```

- [ ] **Step 5: Watch them pass**, then the crate:

```bash
cargo test -p yokohama-web --lib bots::
cargo test -p yokohama-web --lib the_view_names_the_bot
cargo test -p yokohama-web --lib the_mirror_carries_the_records_bot
cargo test -p yokohama-web --lib the_payload_names_the_records_bot_seats
cargo test -p yokohama-web --lib
```

Expected: the seven new cells pass; the lib is **+7** over Task 2's reading. `wire_fixtures` is NOT green yet (Step 7).

**The Worker's house copy and scrub still compile, unchanged** (D2 verified: `cargo test -p yokohama-worker` green at
this step): `house.rs:100-123` destructures `TableDoc` naming `turn: _`, and `TurnMirror`'s new field sits inside
`turn`, which the scrub keeps as stored — so a Worker lobby row reads its dealt table's `bot` off the house copy, as
the NAS's reads it off the document. Extending the fence down to `TurnMirror` is cycle C's.

- [ ] **Step 6: The pool fixture's mirror carries the record's bot (P-4).** The pool's owed-game fixture pair,
  `worker/test/fixtures/owed-game-over.json` and `.table.json`, is a FINISHED vs-bot game whose table was written before
  the field existed. The rebuild's pump ends the game and `live_handle` persists when the pump played; `mirror_of` now
  fills `bot`, so `mirror_turn` REWRITES the table, the object owes the house its first copy and arms the push alarm —
  and two cells' premises break: `archive.test.ts › the switch › off: … the route is the layer's 404` ("no alarm armed:
  expected … to be null") and `operator.test.ts › the operator's force › … at once` ("premise (at once): the house was
  never sent the table's copy"). The review MEASURED both red, and both green with the fixture carrying the
  descriptor. So the fixture's mirror says what a persist under the build that dealt it would have written.

  **As TEXT, never through a JS parse**: `params.weights` is `6957643967275421640`, above 2^53, and the pool loads the
  fixture as a string. Read the descriptor off the record with Python (its integers are exact) and confirm it before
  pasting:

```bash
python3 - <<'EOF'
import json
s = json.load(open("worker/test/fixtures/owed-game-over.json"))["run"]["seats"][1]
print(json.dumps({"controller": s["controller"], "version": s["version"], "params": s["params"]}, indent=2))
EOF
```

  Expected: `v4-step9-draft1`, version `2`, params `b 4, eval_impl 3, k 1, m 8, weights 6957643967275421640`. Then
  `worker/test/fixtures/owed-game-over.table.json:29-33`, the `turn` block, becomes (the same keys, in the mirror's
  field order, at the file's own indentation):

```json
  "turn": {
    "to_move": 1,
    "over": true,
    "winner": 1,
    "bot": {
      "controller": "v4-step9-draft1",
      "version": 2,
      "params": {
        "b": 4,
        "eval_impl": 3,
        "k": 1,
        "m": 8,
        "weights": 6957643967275421640
      }
    }
  },
```

  `grep -c 6957643967275421640 worker/test/fixtures/owed-game-over.table.json` prints `1`, and
  `python3 -c "import json; json.load(open('worker/test/fixtures/owed-game-over.table.json'))"` says nothing (it
  parses). The alternative the review offered — rewriting the two cells' premises — is NOT taken (O-1 (a)): the
  fixture stays a dealt table whose document says what its record says, and both cells keep their questions.

- [ ] **Step 7: The contract file and the one bot table — first the failing check.**

`wire_fixtures.rs:2149`, in `corpus()`:

```rust
    all.push(("site_space_words".to_string(), site_space_words()));
    all.push(("production_bot".to_string(), production_bot()));
```

(Task 1's `identity_cases` push sits beside these; the order of pushes is not the order of the files.) Directly
before `:2299` (`/// **No payload spells the setup seed…`):

```rust
/// **The bot this build deals, for the screens that must name it without
/// asking** (M6.5c2, spec §2.4; review M1, R2-m6).
///
/// Not a payload any route serves — a CONTRACT, emitted and refused exactly as
/// [`site_space_words`] is. Unlike that file, which the client holds a function
/// to, this one is IMPORTED: the door's bot card, the local form and the table
/// form's bot option read the label from it at build time, because the door
/// asks the server nothing (R-6.5b2.7) and a fifth literal in
/// `web/src/local/facts.ts` would be a second truth. When the AI lane re-points
/// `PROVISIONAL_DEFAULT_BOT`, the writer test below refuses until this file is
/// regenerated — that is the "pull".
fn production_bot() -> Value {
    let bot = yokohama_web::bots::production();
    json!({ "identity": bot.controller, "label": yokohama_web::bots::label(&bot) })
}
```

`wire_fixtures.rs:1762-1767` (the tail of `walk_with`) becomes:

```rust
    // **A table with a BOT seat, as its creator reads it** (M6.5c2, spec §2.4),
    // from an `Api` of its own so the primary walk stays bot-free — a bot's
    // stream seed is OS entropy, drawn at the deal — and so no lobby page this
    // walk commits gains a row. Undealt, so its bot is the one `finalize` will
    // deal and the payload carries no seed of any kind. It is the corpus's one
    // `TableView` whose `bot` is not null: without it `Widen<TableView>` is
    // satisfied by `null` and checks nothing about `TableBot` (the terminal
    // explanation's lesson, `contract.ts` §1).
    let bots = Api::new();
    let (status, table_bot) = bots.call("POST", "/api/games", Some(bot_table_body())).await;
    assert_eq!(status, StatusCode::CREATED, "{table_bot}");
    assert_eq!(table_bot["status"], "dealing", "premise: undealt, so its bot is the one this build deals");
    let production = yokohama_web::bots::production();
    assert_eq!(
        table_bot["bot"],
        json!({ "label": yokohama_web::bots::label(&production), "runs_here": true }),
    );
    out.push(("table_bot".to_string(), table_bot));

    let (archived, archive_script) = archive_payloads(api).await;
    out.extend(archived);

    let mut aside = off.script.take();
    aside.extend(bots.script.take());
    aside.extend(archive_script.iter().cloned());
    (out, aside, archive_script)
}

/// The vs-bot table the `table_bot` fixture is read from: the walker at seat 0,
/// the bot at seat 1, at the corpus seed.
fn bot_table_body() -> Value {
    json!({
        "players": 2,
        "colors": ["Red", "Blue"],
        "start_seat": 0,
        "seed": SETUP_SEED,
        "seats": [{ "user": WALKER }, { "bot": true }],
        "name": "wire-bot",
        "id": "wire-bot",
    })
}
```

`Walk.aside`'s doc (`:747-756`): "one built with the lobby chat off, one with a real archive directory" becomes "one
built with the lobby chat off, one with a real archive directory, and one holding a vs-bot table (M6.5c2)".

```bash
cargo test -p yokohama-web --test wire_fixtures the_committed_wire_fixtures_are_what_the_server_emits
```

Expected: FAIL, "the wire contract moved", listing the 31 files of Step 8 (each `committed N bytes, server emits M
bytes`) and `production_bot.json — No such file or directory (os error 2)` and `table_bot.json — No such file or
directory (os error 2)`.

- [ ] **Step 8: Regenerate, and READ the diff.**

```bash
UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures the_committed_wire_fixtures_are_what_the_server_emits
cargo test -p yokohama-web --test wire_fixtures
git status --short web/src/wire web/src/wire-script
git diff --numstat web/src/wire
```

The first command ends red BY DESIGN ("UPDATE_WIRE_FIXTURES was set: fixtures were REWRITTEN, not checked"); the
filter keeps the corpus's other cells from reading files while they are rewritten (an unfiltered run reddens up to
three of them for that reason alone — measured). The second is the check: **12 passed** (Task 1's count, unchanged).

**Expected, and nothing else** (the emitter writes every payload key-SORTED, so each new key lands in sorted place):
- NEW `production_bot.json`: `{"identity": "v4-step9-draft1", "label": "V4"}`;
- NEW `table_bot.json`: a `dealing` 2P table `wire-bot`, `created_by` `red@wire.test`, seats `[human red@wire.test, bot]`,
  `chosen [false, true]`, `my_seats [0]`, and `"bot": {"label": "V4", "runs_here": true}`;
- `+  "bots": [],` in 20 `ClientState` files — `choice_AssistantMove`, `choice_ChurchSpace`, `choice_WarehouseTake`,
  `fee_nonzero`, `game_over`, `locked`, `options`, `options_4p`, `options_many`, `options_priced`, `pick`, `place_a`,
  `place_a_selected`, `place_b`, `place_b_selected`, `refused`, `retreat`, `retreat_answered`, `spectator`,
  `spectator_over` — one line each;
- `watching.json`: six lines, `"bots": [ { "label": "V4", "seat": 0 } ]` (its constructed `Repl` seats the bot at 0);
- `"bot": null,` once at the root of `table.json`, `table_dealing.json`, `table_open.json`, `table_watchable.json`;
  once inside `deal.json`'s and `deal_waiting.json`'s `table`; per row in `games.json` (2), `games_open.json` (2),
  `games_next.json` (1), `games_playing.json` (1);
- `git diff --numstat` totals 31 files, 38 insertions, 0 deletions; `web/src/wire-script/` byte-identical (A2);
  `identity_cases.json` untouched.

Any other change is a FINDING: stop and report it rather than commit it.

- [ ] **Step 9: The client's half — first the failing check.**

```bash
cd web && npx vitest run src/wire/contract.test.ts
```

Expected: FAIL — the key-set rows see the server's new keys: `ClientState (…)` rows report `bots` beyond
`KEYS.clientState`, and `table_dealing`, `table`, `table_open`, `table_watchable` report `bot` beyond `KEYS.table`
(the runtime half of §2 is the only half that sees a key the SERVER added; `npm run typecheck` is still green here).

- [ ] **Step 10: `types.ts` and the contract.**

`web/src/types.ts:345-348` — `ClientState` gains `bots`, and three types follow it:

```ts
  /** The record's length; see the server's doc. */
  events: number;
  /** Every bot seat the RECORD names, labelled (M6.5c2, R-6.5c2.10) — empty
   *  for an all-human game. The record's, not the resident session's. */
  bots: SeatBot[];
}

/** One bot seat of a game: which seat, and the bot's short label (`V4`). */
export interface SeatBot {
  seat: PlayerId;
  label: string;
}

/** A table's bot (M6.5c2, R-6.5c2.10): its short label, and whether THIS
 *  build runs it. An undealt table's is the bot `finalize` will deal, so it
 *  runs here; a dealt one's is its record's. `runs_here: false` is a game
 *  played against another version — finished, it opens; not finished, every
 *  game route refuses it. */
export interface TableBot {
  label: string;
  runs_here: boolean;
}

/** `web/src/wire/production_bot.json` (M6.5c2, spec §2.4): the bot this build
 *  deals, emitted by `wire_fixtures` and imported at build time by the screens
 *  that name it — the door asks the server nothing. */
export interface ProductionBot {
  /** The controller name its records carry (`v4-step9-draft1`). */
  identity: string;
  label: string;
}
```

`types.ts:568-569` — `TableView` gains, after `can_watch`:

```ts
  /** The table's bot, or `null`: no bot seat, or a table dealt before the
   *  field existed (its row says "the bot" with no version). */
  bot: TableBot | null;
```

`web/src/wire/contract.ts`:
- the type import (`:51-99`) gains `ProductionBot` (before `PromptState`, `:85`), `SeatBot` (after `RefusalRow`, `:88`)
  and `TableBot` (before `TableView`, `:91`);
- after `import siteSpaceWords …` (`:106`; Task 1's `identityCases` import sits there too):

```ts
import productionBot from "./production_bot.json";
import tableBot from "./table_bot.json";
```

- after `const _houseChatOff404 …` (`:342`):

```ts
// ---- M6.5c2: the bot's label ------------------------------------------------
//
// `bot` is `TableBot | null` and `null` in every walk payload, so the table
// assertions above are satisfied without ever reaching `Widen<TableBot>` — the
// terminal explanation's lesson below. `table_bot` is the vs-bot table that
// enters the branch, and `watching` the one `ClientState` whose `bots` is not
// empty (the ARRAY, for `noUncheckedIndexedAccess`'s reason under §1b).
const _tableBot: Widen<TableView> = tableBot;
const _tableBotBot: Widen<TableBot> = tableBot.bot;
const _seatBots: Widen<SeatBot>[] = watching.bots;
// The contract the door imports: not a route's payload, and typed here so the
// screens that read it read a checked shape.
const _productionBot: Widen<ProductionBot> = productionBot;
```

- the `void [ … ]` list ending `_houseChatOff404,` (`:502`) gains a line
  `  _tableBot, _tableBotBot, _seatBots, _productionBot,`;
- `CLIENT_STATE` (`:539-543`) ends `"scores", "warnings", "events", "bots",`; `TABLE` (`:589-593`) ends
  `"can_watch", "bot",`; after `TABLE`:

```ts
// M6.5c2: the table's bot, one bot seat, and the door's contract file.
const TABLE_BOT = ["label", "runs_here"] as const;
const SEAT_BOT = ["seat", "label"] as const;
const PRODUCTION_BOT = ["identity", "label"] as const;
```

- after `const _aw …` (`:689-690`):

```ts
const _tb: KeysOf<TableBot, typeof TABLE_BOT> = TABLE_BOT;
const _sb: KeysOf<SeatBot, typeof SEAT_BOT> = SEAT_BOT;
const _pb: KeysOf<ProductionBot, typeof PRODUCTION_BOT> = PRODUCTION_BOT;
```

  and `_cg, _cm, _as, _aw];` (`:696`) becomes `_cg, _cm, _as, _aw, _tb, _sb, _pb];`;
- `KEYS` (`:744`, after `chatAuthorWatcher`): `tableBot: TABLE_BOT, seatBot: SEAT_BOT, productionBot: PRODUCTION_BOT,`;
- `wire` (`:861`, after `houseChatOff404`):

```ts
  // ---- M6.5c2: the bot's label -------------------------------------------
  tableBot: as<TableView>(tableBot),
  productionBot: as<ProductionBot>(productionBot),
```

`web/src/wire/contract.test.ts`, after the `["result.breakdown", …]` row (`:172`):

```ts
    // M6.5c2: the one table whose bot is not null, that bot, a bot SEAT, and
    // the door's contract file.
    ["table_bot", wire.tableBot, KEYS.table],
    ["table_bot.bot", wire.tableBot.bot, KEYS.tableBot],
    ["watching.bots", wire.watching.bots[0], KEYS.seatBot],
    ["production_bot", wire.productionBot, KEYS.productionBot],
```

`web/src/local/corpus.node.test.ts` — the doc's last bullet (`:137-139`) gains, and `NOT_SERVED` (`:141-162`) ends with
(INSERTED after the names already there — Task 1's `identity_cases` among them; the list is compared sorted):

```ts
 * - `production_bot` is a CONTRACT the client imports (M6.5c2), never a
 *   route's payload.
 * - `table_bot` comes from one more `Api` of the emitter's (M6.5c2), holding a
 *   vs-bot table, which no script replays.
```

```ts
  "archive_400_table",
  "production_bot",
  "table_bot",
];
```

`worker/test/corpus.test.ts` — the doc (`:50-52`) gains the same two bullets, and `THIRD_COLUMN` (`:57-66`), compared
AS WRITTEN against a sorted derivation, gets `"production_bot",` and `"table_bot",` INSERTED in sorted place. **After
Task 1 the list holds `identity_cases`; the whole list then reads exactly:**

```ts
const THIRD_COLUMN = [
  "explanation_terminal",
  "house_chat_off_404",
  "identity_cases",
  "locked",
  "production_bot",
  "site_space_words",
  "table_bot",
  "watching",
  "why",
  "why_refusals",
  "why_refused",
];
```

(A list that lacks `identity_cases` is a list pasted over Task 1's edit — the retired review's P-8 — and the pool's
`expect(third).toEqual(THIRD_COLUMN)` names the missing file.)

- [ ] **Step 11: Watch the client's half pass.**

```bash
cd web && npx tsc -b --noEmit && npx tsc -p tsconfig.node.json && npx vitest run src/wire/contract.test.ts && npm test
```

(The app project and the node project — the one holding `corpus.node.test.ts`; the whole `npm run typecheck`, whose
worker project needs `npm run build:wasm`'s glue, runs in Step 13.) Expected: green; `npm test` is **+4** over Task 2's
reading (the four rows; D2 read `contract.test.ts` 135 → 139, the suite 1056 → 1060).

- [ ] **Step 12: Prove the stale-file refusal** (spec §3: "`production_bot.json` is refused when stale") — in this
tree, then restored:

```bash
sed -i.bak 's/"label": "V4"/"label": "V3"/' web/src/wire/production_bot.json
cargo test -p yokohama-web --test wire_fixtures the_committed_wire_fixtures_are_what_the_server_emits
mv web/src/wire/production_bot.json.bak web/src/wire/production_bot.json
git status --short web/src/wire
```

Expected: FAIL naming `production_bot.json — committed 53 bytes, server emits 53 bytes; first difference at byte 48`
(measured); `git status` shows the file as the regeneration left it once it is restored (new, untracked).

- [ ] **Step 13: The per-edit gates** (Global Constraints):

```bash
cargo test -p yokohama-web
cargo test -p yokohama-wasm
cargo test -p yokohama-worker
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown
cargo test -p yokohama-web --no-default-features --features server --lib identity::
cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm
cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker
cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm
cd ../worker && npm run build:module:debug && npm run typecheck && npm run test:scripts && npm test
git diff --stat 1e16fe27 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli
```

Counts that move, as deltas over Task 2's reading: the web lib **+7**; `npm test` **+4**. `wire_fixtures`, the
`identity::` count (8), `yokohama-wasm`, `yokohama-worker`, `test:scripts`, `test:wasm` and the pool keep their counts
(each third-column list grows by two names). **The pool's count is the reading that proves Step 6**: without the
fixture's `bot`, the review measured `archive.test.ts › the switch › off …` and `operator.test.ts › … at once` red.
The fence prints nothing. `npm run e2e` is NOT owed: no component renders the new fields yet (cycle B's).

**The reviewer reads** `git diff 1e16fe27 -- crates/yokohama-web/src` against Tasks 1–2's lists plus: `bots.rs` (new),
`lib.rs`'s module line, `table.rs` (the mirror, the view, the six sites), `tables.rs:427` and three test literals,
`games.rs` (`mirror_of`, `mirror_turn`, one cell), `client.rs` (the field, its line in `of`, one cell).

- [ ] **Step 14: Commit.**

```bash
git add crates/yokohama-web/src/bots.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/table.rs \
  crates/yokohama-web/src/tables.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/client.rs \
  crates/yokohama-web/tests/wire_fixtures.rs web/src/wire web/src/types.ts \
  web/src/local/corpus.node.test.ts worker/test/corpus.test.ts worker/test/fixtures/owed-game-over.table.json
git status --short
git commit -m "feat(web): the bot's label — bots.rs, the turn mirror's bot, TableView.bot and ClientState.bots, and the production_bot contract file"
```

(`web/src/wire` is staged as a directory: it holds exactly the 31 regenerated files, the two new ones, and
`contract.ts`/`contract.test.ts`. `git status --short` must be empty afterwards.)

---

### Task 4: The foreign-bot rule, and the four places it is masked (spec §2.4 "The foreign-bot rule"; review M2, R2-M1, R2-m5, R2-o1; Review Focus 3 and 4)

**Measured:** D2 ran every step in an export; the review's chain carries it in `0002`–`0003` and ran the pool over it
(no pool change). The `THIRD_COLUMN` literal below includes Task 1's `identity_cases` (P-8). **The check's site** is
the spec's corrected one (§10.5; D2's A4): the descriptors are compared off the parsed record, and the refusal returns
between `Session::from_record` and `Repl::new`, because "not over" is a fact about the replayed state and the seat
conversion is `Repl::new`'s — so no `Repl`, and no converted seat, ever exists for a refused game.

**The foreign identities the fixtures use** (D2's A7): the masks need an identity this CLI will NOT run.
`bot::eligible` admits `LookaheadV1` (`crates/yokohama-cli/src/bot.rs:379`), so `from_provenance` keeps that seat a
bot and converts nothing — no conversion line to filter, no `is_bot` disagreement to mask. So **`v2-padded`** (a named
research identity, refused by `eligible` at `bot.rs:402` and CONVERTED, exactly as a V4 record will be once V5 ships)
for the masks and the corpus, and **`lookahead-v1`** for the first cell, where "it would pump if a `Repl` were built"
is the measurement.

**Files:**
- Modify: `crates/yokohama-web/src/bots.rs` — `foreign`, `ForeignBot`, `cannot_continue`.
- Modify: `crates/yokohama-web/src/games.rs`:
  - `:41-60` — `GameError::ForeignBot { played, runs }`; `:62-75` — its `Display`;
  - `:425-429` — the rule's one site in `live_handle`;
  - test support (`:2149`, before `stored_json`) — `PLAYED_RECORD`, `PLAYED_TABLE`, `PLAYED_ID`, `BOT_TO_MOVE_AT`,
    `foreign_pair`;
  - three cells before `:946`.
- Modify: `crates/yokohama-web/src/lib.rs:117-129` (`ApiError.foreign_bot`), `:131-175` and `:184-200` (literals, the
  `From` arm), `:202-211` (`IntoResponse`).
- Modify: `crates/yokohama-web/src/api.rs:78-86` (`error_response`), `:89` and twelve more literals, `:600-607`
  (`undo_allowed`'s fence), `:1364-1366` (`why`'s pre-empt); three cells before `:2241`.
- Modify: `crates/yokohama-web/src/identity.rs:192` — the literal.
- Modify: `crates/yokohama-web/src/client.rs:265-290` — `facts_for`'s `is_bot` from the record, its doc re-ruled by
  name; `without_foreign_conversion`; `:498` — `warnings` through it.
- Modify: `crates/yokohama-web/src/explain.rs:435` — `Why::foreign` after `Why::locked`, in the same `impl`; `:437` —
  `FOREIGN_BOT_KIND` and `foreign_why` before `impl Provenance`.
- Modify: `crates/yokohama-worker/src/events.rs:52`, `crates/yokohama-worker/src/lib.rs:1051` — the literals;
  `lib.rs:264-279` — the restated `error_body` gains the third arm; `lib.rs:1039-1053` — the pin gains the foreign-bot
  case and a helper.
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs` — the foreign `Api` in `walk_with`'s tail, `PAIR_*` consts and a
  `foreign_pair` copy.
- Create (generated, committed): `web/src/wire/foreign_bot_409.json`, `table_foreign.json`, `why_foreign.json`.
- Modify: `web/src/types.ts:612-614` — `ForeignBot`, `ForeignBotErrorBody` after `DealingErrorBody`.
- Modify: `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `web/src/local/corpus.node.test.ts`,
  `worker/test/corpus.test.ts` — the three new files typed, key-listed, exposed on `wire`, and named in both replays'
  third columns (Steps 9–10).

**Interfaces:**
- Consumes (Task 3): `bots::{BotDescriptor, production, label, recorded}`, `TurnMirror.bot`, `TableView.bot`,
  `ClientState.bots`, `wire.productionBot`, `KEYS.tableBot`.
- Produces (shared names): `GameError::ForeignBot { played: String, runs: String }` → 409
  `{"error": "This game was played against V2. This server now runs V4 and can't continue it.", "foreign_bot": {"played": "V2", "runs": "V4"}}`;
  `explain::Why::foreign(played: &str) -> Why` (the Rust type is `explain::Why`, `explain.rs:378-382`, with the
  precedent `Why::locked` at `:420-435`; the client's `WhyResponse` is the same payload, and its `refusal: RefusalRow`
  already carries `kind: string`), refusal `{"kind": "foreign_bot", "message": "This move was made by V2, which this
  server doesn't run."}`; `bots::{foreign, ForeignBot, cannot_continue}`; `ApiError.foreign_bot`;
  `explain::{FOREIGN_BOT_KIND, foreign_why}`; `client::without_foreign_conversion` (**Task 7 wraps it**:
  `warnings: without_version_drift(without_foreign_conversion(…))`); the test support names; the three corpus files;
  TS `ForeignBot`/`ForeignBotErrorBody`; `wire.foreignBot409/tableForeign/whyForeign`;
  `KEYS.foreignBotError`/`KEYS.foreignBot`.
- **Reaches the Worker crate** (compile-forced): two `ApiError` literals and the RESTATED `error_body`, which gains the
  `foreign_bot` arm here. **Task 7's exported body builder must carry the arm** (P-3), and cycle C deletes the
  restatement.
- The client's game-screen handling of the 409 and the local list's "Will not open here" row are cycle B's.

- [ ] **Step 1: Write the failing cells.**

`games.rs`, in `test_support`, directly before `:2149` (`/// The record `id` is stored as, as bytes.`):

```rust
    /// The committed browser-played pair (595 events, a person at seat 0 and
    /// the V4 bot at seat 1), as `browser_played_pin.rs` holds it.
    pub(crate) const PLAYED_RECORD: &str = include_str!("../tests/fixtures/browser-played-2p.json");
    pub(crate) const PLAYED_TABLE: &str = include_str!("../tests/fixtures/browser-played-2p.table.json");
    /// The pair's own id, under which every cell stores it.
    pub(crate) const PLAYED_ID: &str = "local-242a5429453c593c";
    /// The event count at which [`foreign_pair`]'s cut leaves the BOT (seat 1)
    /// to move in a game not yet over — measured, and asserted as a premise by
    /// the cell that reads it.
    pub(crate) const BOT_TO_MOVE_AT: usize = 300;

    /// The committed pair re-seated as a game played against ANOTHER bot
    /// (M6.5c2, R-6.5c2.10): seat 1's recorded controller renamed to
    /// `controller`, its version and knobs kept — `Contestant::from_parts`
    /// reads `lookahead-v1`'s or a named identity's three knobs from them and
    /// accepts the identity keys — with the table's mirror naming the same
    /// bot, as a persist under the build that dealt it would have written.
    /// With `cut`, only the first `cut` events, re-stamped for the state they
    /// fold to, and the mirror moved with them: not over, and that state's
    /// seat to move. Answers `(record, table)` as JSON.
    pub(crate) fn foreign_pair(controller: &str, cut: Option<usize>) -> (String, String) {
        use yokohama_core::{state_at, GameRecord};
        let mut record: GameRecord = serde_json::from_str(PLAYED_RECORD).expect("the pair's record");
        let mut table: crate::table::TableDoc = serde_json::from_str(PLAYED_TABLE).expect("the pair's table");
        let seat = &mut record.run.as_mut().expect("the pair's run provenance").seats[1];
        seat.controller = controller.to_string();
        let bot = crate::bots::BotDescriptor {
            controller: seat.controller.clone(),
            version: seat.version,
            params: seat.params.clone(),
        };
        let mirror = table.turn.as_mut().expect("the pair's mirror");
        mirror.bot = Some(bot);
        if let Some(cut) = cut {
            record.events.truncate(cut);
            let state = state_at(&record, cut, &Rules::standard()).expect("the cut replays");
            record.stamp_terminal_digest(&state);
            *mirror = crate::table::TurnMirror {
                to_move: state.current_player.0,
                over: false,
                winner: None,
                bot: mirror.bot.take(),
            };
        }
        (serde_json::to_string(&record).expect("a record"), serde_json::to_string(&table).expect("a table"))
    }
```

`games.rs`, in the test module, directly before `:946`:

```rust
    /// **The foreign-bot rule, at its one site** (M6.5c2, R-6.5c2.10; review
    /// M2): a game NOT over whose recorded bot is not the one this build deals
    /// refuses with the two labels — and refuses BEFORE a `Repl` exists.
    ///
    /// The foreign bot is `lookahead-v1`, which this CLI CAN run, at a cut where
    /// it is to move: a rebuild that got as far as `Repl::new` would pump it and
    /// persist the move. So the untouched write count is the measurement — and
    /// the same cut under THIS build's bot, which does pump and persist, is what
    /// says the count could have moved.
    #[test]
    fn a_game_not_over_against_a_bot_this_build_does_not_run_refuses_before_any_rebuild() {
        let store = Arc::new(CountingStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let (record, table) = foreign_pair("lookahead-v1", Some(BOT_TO_MOVE_AT));
        games.load_document(Doc::Record, PLAYED_ID, &record).unwrap();
        games.load_document(Doc::Table, PLAYED_ID, &table).unwrap();
        let loaded = store.writes();
        match games.handle(PLAYED_ID) {
            Err(GameError::ForeignBot { played, runs }) => {
                assert_eq!(played, "V2 lookahead", "the recorded bot's label");
                assert_eq!(runs, crate::bots::label(&crate::bots::production()), "this build's");
            }
            other => panic!("expected the foreign-bot refusal, got {:?}", other.map(|_| "a handle")),
        }
        assert_eq!(store.writes(), loaded, "nothing persisted: no `Repl`, so no pump");
        assert!(games.live.lock().unwrap().is_empty(), "and nothing is resident");

        // The anti-vacuity: the SAME cut under this build's own bot rebuilds,
        // pumps the bot's owed move and persists it.
        let store = Arc::new(CountingStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let (own, table) = foreign_pair(&crate::bots::production().controller, Some(BOT_TO_MOVE_AT));
        games.load_document(Doc::Record, PLAYED_ID, &own).unwrap();
        games.load_document(Doc::Table, PLAYED_ID, &table).unwrap();
        let loaded = store.writes();
        let handle = games.handle(PLAYED_ID).expect("this build's own bot opens");
        assert!(handle.lock().unwrap().session.record().events.len() > BOT_TO_MOVE_AT, "the bot moved");
        assert!(store.writes() > loaded, "premise: a rebuild at this cut persists");
    }

    /// **A finished game against a foreign bot opens** (review M2): it has no
    /// bot left to move, so the rule is about resuming and not about reading.
    #[test]
    fn a_finished_game_against_a_foreign_bot_opens() {
        let games = games();
        let (record, table) = foreign_pair("v2-padded", None);
        games.load_document(Doc::Record, PLAYED_ID, &record).unwrap();
        games.load_document(Doc::Table, PLAYED_ID, &table).unwrap();
        let handle = games.handle(PLAYED_ID).expect("a finished game opens");
        assert!(handle.lock().unwrap().session.is_over());
    }

    /// **(Review Focus 4) R2-o1: an owed game-over upload that was undone and
    /// then meets a new bot is bounded, not stranded.** The Worker's retry
    /// reaches the layer as `archive_over_retry`, which answers the foreign-bot
    /// refusal (the shell's backoff retries it: `settleUpload`'s `parseStamp`
    /// throws on an error body), and the creator's Abandon — which reads the
    /// record's event count and never rebuilds — archives the game and clears
    /// it.
    #[test]
    fn an_unfinished_foreign_game_is_refused_by_the_retry_and_cleared_by_abandon() {
        let archive = RecordingArchive::recording();
        let games = games().with_archive(archive.clone());
        let (record, table) = foreign_pair("v2-padded", Some(BOT_TO_MOVE_AT));
        games.load_document(Doc::Record, PLAYED_ID, &record).unwrap();
        games.load_document(Doc::Table, PLAYED_ID, &table).unwrap();
        assert!(matches!(games.archive_over_retry(PLAYED_ID), Err(GameError::ForeignBot { .. })));
        assert!(archive.puts.lock().unwrap().is_empty(), "the retry put nothing");
        games.delete(PLAYED_ID, &who("local@browser")).expect("the creator's Abandon");
        let puts = archive.puts.lock().unwrap();
        assert_eq!(puts.len(), 1, "archived once");
        assert_eq!(puts[0].why, crate::archive::Why::Abandoned, "the mirror says not over");
        assert_eq!(puts[0].record, record, "the stored bytes");
        assert!(games.read_table(PLAYED_ID).unwrap().is_none(), "and gone");
    }
```

`api.rs`, in the test module, directly before `:2241` (`/// **The archive route, driven through `dispatch` itself**…`):

```rust
    // -----------------------------------------------------------------------
    // M6.5c2: the foreign-bot rule, through `dispatch` (R-6.5c2.10)
    // -----------------------------------------------------------------------

    /// One call through `dispatch`, as `who`, on the local host's facts.
    fn ask(games: &Games, who: &str, method: &str, path: &str, query: &str, body: &str) -> (u16, serde_json::Value) {
        let user = User::parse(who).expect("an address");
        let res = dispatch(
            games,
            &LOCAL_HOST,
            Ok(&user),
            &ApiRequest {
                method,
                path,
                query,
                content_type: (!body.is_empty()).then_some("application/json"),
                body: body.as_bytes(),
            },
        );
        (res.status, serde_json::from_str(&res.body).unwrap_or(serde_json::Value::Null))
    }

    /// A `Games` holding the committed pair as a game against `controller`,
    /// cut at `cut` events when given.
    fn holding_foreign(controller: &str, cut: Option<usize>) -> Games {
        let games = games_with(std::sync::Arc::new(crate::archive::NoArchive));
        let (record, table) = crate::games::test_support::foreign_pair(controller, cut);
        games.load_document(crate::store::Doc::Record, PLAYED_ID, &record).unwrap();
        games.load_document(crate::store::Doc::Table, PLAYED_ID, &table).unwrap();
        games
    }

    /// **Every game route refuses a game not over against a foreign bot, in
    /// one typed 409** (M6.5c2, R-6.5c2.10) — the sentence naming both labels,
    /// and the pair beside it for the client to branch on. The table still
    /// reads (the lobby row says `runs_here: false`), and Abandon, which never
    /// rebuilds, still deletes it.
    #[test]
    fn every_game_route_refuses_an_unfinished_foreign_game_in_one_typed_409() {
        let games = holding_foreign("lookahead-v1", Some(crate::games::test_support::BOT_TO_MOVE_AT));
        let runs = crate::bots::label(&crate::bots::production());
        let want = serde_json::json!({
            "error": format!("This game was played against V2 lookahead. This server now runs {runs} and can't continue it."),
            "foreign_bot": { "played": "V2 lookahead", "runs": runs },
        });
        let base = format!("/api/games/{PLAYED_ID}");
        for (method, route, query, body) in [
            ("GET", "", "", ""),
            ("POST", "/input", "", r#"{"line":"1"}"#),
            ("GET", "/ports", "", ""),
            ("GET", "/labs", "", ""),
            ("GET", "/areas", "", ""),
            ("GET", "/log", "", ""),
            ("GET", "/at", "from=0&to=1", ""),
            ("GET", "/at/0", "", ""),
            ("GET", "/suggest", "", ""),
            ("GET", "/why", "", ""),
            ("GET", "/index/orders", "", ""),
            ("GET", "/result", "", ""),
            ("GET", "/record", "", ""),
            ("GET", "/chat", "", ""),
        ] {
            let (status, v) = ask(&games, "local@browser", method, &format!("{base}{route}"), query, body);
            assert_eq!((status, &v), (409, &want), "{method} {base}{route}");
        }
        let (status, v) = ask(&games, "local@browser", "GET", &format!("{base}/table"), "", "");
        assert_eq!(status, 200, "{v}");
        assert_eq!(v["bot"], serde_json::json!({ "label": "V2 lookahead", "runs_here": false }));
        let (status, v) = ask(&games, "local@browser", "DELETE", &base, "", "");
        assert_eq!(status, 204, "Abandon never rebuilds, so it is not refused: {v}");
    }

    /// **A finished game against a foreign bot opens, scores and downloads —
    /// and the four places the CLI's seat conversion would show are masked**
    /// (M6.5c2, R-6.5c2.10; review R2-M1). `v2-padded` is a named identity
    /// this CLI refuses to run, so `Repl::new` turns seat 1 into a human one
    /// inside the resident `Repl`, exactly as a V4 game will read once V5 ships.
    #[test]
    fn a_finished_foreign_game_opens_scores_downloads_and_masks_the_conversion() {
        let games = holding_foreign("v2-padded", None);
        let base = format!("/api/games/{PLAYED_ID}");
        let (status, state) = ask(&games, "local@browser", "GET", &base, "", "");
        assert_eq!(status, 200, "{state}");
        assert_eq!(state["over"], true);
        // 1. the warning: the WHOLE array, so a reworded CLI line fails here.
        assert_eq!(state["warnings"], serde_json::json!([]), "no conversion line, and nothing else");
        // 4. the Players panel says bot where the record does, and agrees with `bots`.
        assert_eq!(state["bots"], serde_json::json!([{ "seat": 1, "label": "V2" }]));
        let players = state["frame"]["dashboard"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|p| p.get("Players"))
            .expect("a Players panel");
        let is_bot: Vec<bool> =
            players["players"].as_array().unwrap().iter().map(|p| p["is_bot"].as_bool().unwrap()).collect();
        assert_eq!(is_bot, [false, true], "the panel and `bots` agree");
        // It scores and downloads as today.
        let (status, _) = ask(&games, "local@browser", "GET", &format!("{base}/result"), "", "");
        assert_eq!(status, 200);
        let (status, _) = ask(&games, "local@browser", "GET", &format!("{base}/record"), "", "");
        assert_eq!(status, 200);
        // 2. `why`: bare, and on a move the bot made (event 4), names the bot —
        // never "a human's move"; a human's own move (event 0) is still the CLI's.
        let foreign = serde_json::json!({
            "ok": null,
            "refusal": { "kind": "foreign_bot", "message": "This move was made by V2, which this server doesn't run." },
        });
        for query in ["", "event=4"] {
            let (status, v) = ask(&games, "local@browser", "GET", &format!("{base}/why"), query, "");
            assert_eq!((status, &v), (200, &foreign), "why?{query}");
        }
        let (_, v) = ask(&games, "local@browser", "GET", &format!("{base}/why"), "event=0", "");
        assert_eq!(v["refusal"]["kind"], "human_seat", "a person's move is the CLI's to word: {v}");
        // 3. undo: refused at this one-human table, and the game stays finished.
        let (status, v) = ask(&games, "local@browser", "POST", &format!("{base}/input"), "", r#"{"line":"undo"}"#);
        assert_eq!(status, 403, "{v}");
        assert_eq!(v["error"], "undo is not offered on a game played against V2, which this server doesn't run");
        let (_, after) = ask(&games, "local@browser", "GET", &base, "", "");
        assert_eq!((after["over"].clone(), after["events"].clone()), (serde_json::json!(true), state["events"].clone()));
    }

    /// **(Review Focus 3) A SPECTATOR — seated nowhere — opening a finished game
    /// played against a foreign bot.** The Players panel says bot, `why` names
    /// the bot's version rather than "a human's move", the watcher's undo is the
    /// not-seated refusal, and nothing a finished game shows a watcher today is
    /// refused.
    #[test]
    fn a_spectator_reads_a_finished_foreign_game_as_the_bots() {
        let games = holding_foreign("v2-padded", None);
        let base = format!("/api/games/{PLAYED_ID}");
        let (status, state) = ask(&games, "grey@t.test", "GET", &base, "", "");
        assert_eq!(status, 200, "{state}");
        assert_eq!(state["seat"], serde_json::Value::Null, "premise: a watcher");
        assert_eq!(state["bots"], serde_json::json!([{ "seat": 1, "label": "V2" }]));
        let players = state["frame"]["dashboard"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|p| p.get("Players"))
            .expect("a Players panel");
        assert_eq!(players["players"][1]["is_bot"], true, "the panel says the bot played seat 1");
        let (_, why) = ask(&games, "grey@t.test", "GET", &format!("{base}/why"), "", "");
        assert_eq!(why["refusal"]["kind"], "foreign_bot", "{why}");
        assert_eq!(why["refusal"]["message"], "This move was made by V2, which this server doesn't run.");
        let (status, v) = ask(&games, "grey@t.test", "POST", &format!("{base}/input"), "", r#"{"line":"undo"}"#);
        assert_eq!((status, v["error"].as_str()), (403, Some(NOT_SEATED)), "a watcher writes nothing: {v}");
        for route in ["/result", "/log", "/at/0", "/ports", "/labs", "/areas", "/index/orders"] {
            let (status, v) = ask(&games, "grey@t.test", "GET", &format!("{base}{route}"), "", "");
            assert_eq!(status, 200, "{route}: {v}");
        }
    }
```

`crates/yokohama-worker/src/lib.rs`, the pin `a_refusal_body_is_the_one_dispatch_writes` (`:1039-1053`): after its
last assertion (`:1052`), before the closing `}`:

```rust

        // M6.5c2: the foreign-bot 409, whose pair rides beside the sentence —
        // the events route's body is the restatement's, dispatch's the layer's.
        let (record, table) = foreign_unfinished("foreign");
        load_inner("record", "foreign", &record).unwrap();
        load_inner("table", "foreign", &table).unwrap();
        let d = dispatch_inner(Some("local@browser"), "GET", "/api/games/foreign", "", None, b"");
        assert_eq!(d.status, 409, "premise: a game not over against a foreign bot — {}", d.body);
        assert!(d.body.contains(r#""foreign_bot":{"played":"V2","#), "premise: the pair rides the body — {}", d.body);
        let e = events_inner("foreign", Some("local@browser"), "");
        assert_eq!((e.status, e.body.as_str()), (d.status, d.body.as_str()), "the restated body, whole");
```

and, after the pin's closing `}`:

```rust
    /// The committed browser-played pair as a game NOT over against a bot this
    /// build does not run: seat 1 renamed `v2-padded`, cut at 300 events with
    /// the bot to move, re-stamped, and the table's mirror moved with it. COPIED
    /// from the layer's `games::test_support::foreign_pair` (ruling R-06: a
    /// test-only helper of another crate cannot be imported).
    fn foreign_unfinished(id: &str) -> (String, String) {
        use yokohama_core::{state_at, GameRecord};
        let pair = |file: &str| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../yokohama-web/tests/fixtures").join(file);
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        };
        let mut record: GameRecord = serde_json::from_str(&pair("browser-played-2p.json")).unwrap();
        let mut table: yokohama_web::table::TableDoc = serde_json::from_str(&pair("browser-played-2p.table.json")).unwrap();
        record.run.as_mut().unwrap().seats[1].controller = "v2-padded".to_string();
        record.events.truncate(300);
        let state = state_at(&record, 300, &yokohama_core::Rules::standard()).unwrap();
        record.stamp_terminal_digest(&state);
        table.id = id.to_string();
        table.turn = Some(yokohama_web::table::TurnMirror {
            to_move: state.current_player.0,
            over: false,
            winner: None,
            bot: None,
        });
        (serde_json::to_string(&record).unwrap(), serde_json::to_string(&table).unwrap())
    }
```

- [ ] **Step 2: Watch them fail.**

```bash
cargo test -p yokohama-web --lib refuses_before_any_rebuild
```

Expected: COMPILE failure — no variant `ForeignBot` on `GameError` (three cells match it).

- [ ] **Step 3: The rule, its refusal and its body.**

`bots.rs`, after `seat_bots`:

```rust
/// The FOREIGN bot seats of `record` (M6.5c2, R-6.5c2.10): every seat whose
/// recorded `(controller, version, params)` is not [`production`]'s. Empty for
/// an all-human game and for every game this build dealt.
pub fn foreign(record: &GameRecord) -> Vec<(u8, BotDescriptor)> {
    let running = production();
    recorded(record).into_iter().filter(|(_, d)| *d != running).collect()
}

/// The pair a foreign-bot refusal names beside its sentence
/// (`"foreign_bot": {"played", "runs"}`): the label of the bot the game was
/// played against, and of the one this build runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForeignBot {
    pub played: String,
    pub runs: String,
}

/// The sentence a not-over game against a foreign bot is refused with — the
/// layer's own words (spec §2.4), not the CLI's.
pub fn cannot_continue(played: &str, runs: &str) -> String {
    format!("This game was played against {played}. This server now runs {runs} and can't continue it.")
}
```

`games.rs:57-60` — `GameError` gains, after `Server(String),`:

```rust
    /// A game NOT OVER whose recorded bot this build does not run (M6.5c2,
    /// R-6.5c2.10): the labels of the bot it was played against and of the one
    /// this build deals. Typed rather than a `Conflict` for `Dealing`'s reason
    /// — a client BRANCHES on it, drawing the reason rather than a banner — so
    /// the pair rides beside the sentence as `"foreign_bot"`.
    ForeignBot { played: String, runs: String },
```

and its `Display` (`:72`, after the `Playing` arm):

```rust
            Self::ForeignBot { played, runs } => f.write_str(&crate::bots::cannot_continue(played, runs)),
```

`games.rs:425-429` (`live_handle`) becomes:

```rust
        let record: GameRecord =
            serde_json::from_str(&json).map_err(|e| GameError::Server(e.to_string()))?;
        // **The foreign-bot rule's one site** (M6.5c2, R-6.5c2.10), on every
        // host, for every rebuild. The descriptors are compared off the parsed
        // record; "not over" is a fact about the replayed state, so it is read
        // off the session — and the refusal returns BEFORE `Repl::new`, whose
        // `BotSeats::from_provenance` is what would turn the foreign seat into
        // a human one, and before `bring_alive`, which would let a foreign bot
        // this CLI can still run move again. No `Repl` is built, so a converted
        // seat never exists and nothing is persisted.
        let foreign = crate::bots::foreign(&record).into_iter().next();
        let session =
            Session::from_record(record, (*self.rules).clone()).map_err(GameError::Server)?;
        if let Some((_, played)) = &foreign {
            if !session.is_over() {
                return Err(GameError::ForeignBot {
                    played: crate::bots::label(played),
                    runs: crate::bots::label(&crate::bots::production()),
                });
            }
        }
        let mut repl = Repl::new(session);
```

Every rebuild passes here: `handle` (`:402`), `settle_locked`'s repair arm (`tables.rs:1195`), `finalize`'s conflict
arm (`:1243`), and on the Worker `picker.rs:246`, `receipt.rs:334` and `:406` through `handle`. `delete`
(`tables.rs:1664-1753`) reads the record's event count and never rebuilds, so Abandon still archives a refused game.
Three consequences, stated rather than fixed: a refused game is rebuilt (replayed) by every request that reaches it,
because no handle is cached for it — the client stops asking at the first 409 (cycle B); on the Worker each wake logs
`restore-refused {status: 409}` (`worker/src/table.ts:1251-1253`, the shell's rule for any layer refusal there) before
the request's own dispatch answers the same 409; and because `settle_locked`'s repair arm rebuilds too, a table left
mid-flip by a crash that straddles a bot re-point answers the refusal on its table read as well — Abandon, which reads
through `read_table` and never settles, still clears it.

`lib.rs:117-129` — `ApiError`'s field docs are corrected, and it gains the field:

```rust
    /// A refusal a client branches on rather than displays (M6.1): a table
    /// that is still open, dealing, or already playing. It rides beside the
    /// sentence as `"status"`, so a client that opened a game by id knows to
    /// draw the setup screen instead of an error banner.
    ///
    /// **`&'static str`, and absent on every other refusal.** A field that a
    /// handler could fill with anything would be a second, untyped error
    /// channel; the only values that reach it are [`games::GameError::Dealing`]'s,
    /// `Open`'s and `Playing`'s.
    pub table_status: Option<&'static str>,
    /// The other refusal a client branches on (M6.5c2, R-6.5c2.10): a game not
    /// over whose recorded bot this build does not run, answered with the two
    /// labels beside the sentence as `"foreign_bot": {"played", "runs"}`.
    /// Typed for `table_status`'s reason, and absent on every other refusal:
    /// the only value that reaches it is [`games::GameError::ForeignBot`]'s.
    pub foreign_bot: Option<bots::ForeignBot>,
```

Every `ApiError { … }` literal gains `foreign_bot: None` — 24 sites in two crates at the base (`lib.rs:133, 136, 148,
156, 164, 172, 193, 195`; `identity.rs:192`; `api.rs:89, 480, 489, 517, 1070, 1116, 1128, 1290, 1479, 1805, 1845,
1854, 1948`; `crates/yokohama-worker/src/events.rs:52`; `crates/yokohama-worker/src/lib.rs:1051`). BEFORE writing the
new `From` arm below, one mechanical pass and its count:

```bash
perl -pi -e 's/table_status: None/table_status: None, foreign_bot: None/g; s/table_status: Some\("(dealing|open|playing)"\)(,?)/table_status: Some("$1"), foreign_bot: None$2/g' \
  crates/yokohama-web/src/lib.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/identity.rs \
  crates/yokohama-worker/src/events.rs crates/yokohama-worker/src/lib.rs
grep -c "foreign_bot: None" crates/yokohama-web/src/lib.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/identity.rs \
  crates/yokohama-worker/src/events.rs crates/yokohama-worker/src/lib.rs
```

Expected counts: 8, 13, 1, 1, 1. (Tasks 1–2 added no `ApiError` literal; if your HEAD differs, every literal needs the
field, or the crate does not compile — the compiler is the count's check.)

`lib.rs:184-200` — the `From` impl gains its arm after `E::Server(m) => …`:

```rust
            E::ForeignBot { played, runs } => Self {
                status: StatusCode::CONFLICT,
                message: bots::cannot_continue(&played, &runs),
                table_status: None,
                foreign_bot: Some(bots::ForeignBot { played, runs }),
            },
```

`lib.rs:202-211` (`IntoResponse`, the axum host's transport routes):

```rust
    fn into_response(self) -> Response {
        let mut body = serde_json::json!({ "error": self.message });
        if let Some(status) = self.table_status {
            body["status"] = status.into();
        }
        if let Some(pair) = &self.foreign_bot {
            body["foreign_bot"] = serde_json::to_value(pair).expect("the pair serializes");
        }
        (self.status, Json(body)).into_response()
    }
```

`api.rs:78-86` (`error_response`, `dispatch`'s body on every host) — its doc's "on the one refusal a client branches
on" becomes "on a refusal a client branches on at a table's status, and `"foreign_bot"` beside it on a game played
against a bot this build does not run (M6.5c2)", and:

```rust
pub(crate) fn error_response(e: ApiError) -> ApiResponse {
    let mut body = serde_json::json!({ "error": e.message });
    if let Some(status) = e.table_status {
        body["status"] = status.into();
    }
    if let Some(pair) = &e.foreign_bot {
        body["foreign_bot"] = serde_json::to_value(pair).expect("the pair serializes");
    }
    ApiResponse { status: e.status.as_u16(), body: body.to_string() }
}
```

`serde_json`'s map sorts its keys either way, so the three bodies stay byte-identical to today's for every existing
refusal (the corpus's 409s and 403s are the witness: none moves).

- [ ] **Step 4: Watch the rule pass — and the Worker's restatement fail.**

```bash
cargo test -p yokohama-web --lib refuses_before_any_rebuild
cargo test -p yokohama-web --lib a_finished_game_against_a_foreign_bot_opens
cargo test -p yokohama-web --lib cleared_by_abandon
cargo test -p yokohama-web --lib every_game_route_refuses_an_unfinished_foreign_game
cargo test -p yokohama-web --lib masks_the_conversion
cargo test -p yokohama-web --lib a_spectator_reads_a_finished_foreign_game
cargo test -p yokohama-worker a_refusal_body_is_the_one_dispatch_writes
```

Expected: the first four PASS. `masks_the_conversion` FAILS at its first mask: `left: Array [String("warning: seat 1
was played by controller \"v2-padded\", but v2-padded is a named research identity for arena weight comparisons, not
available as a CLI bot — it resumes as a HUMAN seat, so you are now playing it")]`, `right: Array []`.
`a_spectator_reads…` FAILS on `players[1].is_bot` (false). The Worker pin FAILS: "the restated body, whole" — its
restated `error_body` drops the pair.

**Mutation, measured in D2's export:** moving the refusal to after `bring_alive`/`persist` makes
`refuses_before_any_rebuild` fail on "nothing persisted: no `Repl`, so no pump" — the write count is what catches a
late site.

- [ ] **Step 5: The four masks, and the restatement.**

**1. The warning** — `client.rs`, after `facts_for`:

```rust
/// `warnings` less the line `Repl::new` added for each seat it turned from a
/// FOREIGN bot into a human (M6.5c2, R-6.5c2.10; spec §2.4's first mask) —
/// `warning: seat N was played by controller "…", … it resumes as a HUMAN
/// seat, so you are now playing it`, which on the web is false: nothing here
/// converts a bot's seat, and nobody is now playing it.
///
/// **By the CLI's own prefix** (`bot.rs`, `BotSeats::from_provenance`'s two
/// conversion arms), per foreign seat and naming its controller, so no other
/// warning can match. `Session::load_warnings` is a `Vec<String>` and a typed
/// alternative would need a CLI change, so the cell that pins this asserts the
/// WHOLE array of a finished foreign game — empty — and fails loudly the day
/// the CLI's wording moves, rather than passing by matching the prefix itself.
pub(crate) fn without_foreign_conversion(
    warnings: Vec<String>,
    record: &yokohama_core::GameRecord,
) -> Vec<String> {
    let converted: Vec<String> = crate::bots::foreign(record)
        .into_iter()
        .map(|(seat, d)| format!("warning: seat {seat} was played by controller \"{}\"", d.controller))
        .collect();
    warnings.into_iter().filter(|w| !converted.iter().any(|p| w.starts_with(p.as_str()))).collect()
}
```

and `client.rs:498`, in `ClientState::of`:

```rust
            warnings: without_foreign_conversion(session.load_warnings.clone(), session.record()),
```

**4. The Players panel** — `client.rs:265-290`; the doc's last sentence ("`is_bot` is still the REPL's own
reconciled answer ([`Repl::bots`]), never re-derived here.") is replaced, BY NAME, and the body reads the record:

```rust
/// the screen to.
///
/// **`is_bot` is the RECORD's answer since M6.5c2** (R-6.5c2.10; spec §2.4's
/// fourth mask), and this re-rules the sentence that stood here — "`is_bot` is
/// still the REPL's own reconciled answer ([`Repl::bots`]), never re-derived
/// here" — by name. The record's `run.seats` is the source `ClientState.bots`
/// reads too, so the panel's badge and the payload's labels cannot disagree.
/// They differ from the `Repl`'s answer in exactly one place: a FINISHED game
/// played against a bot this build does not run, whose seat `Repl::new`
/// (`BotSeats::from_provenance`) has turned into a human one inside the
/// resident `Repl`, and whose Players panel must still say a bot played it.
/// Everywhere else the two agree, because every bot the web deals is
/// `PROVISIONAL_DEFAULT_BOT`, which `from_provenance` restores.
pub fn facts_for(repl: &Repl, viewer: Viewer) -> Vec<SeatFacts> {
    let bots: Vec<u8> =
        crate::bots::recorded(repl.session.record()).into_iter().map(|(seat, _)| seat).collect();
    repl.session
        .state()
        .players
        .iter()
        .map(|p| SeatFacts {
            is_bot: bots.contains(&p.id.0),
            hand_hidden: match viewer {
                Viewer::Seat(s) => p.id != s,
                Viewer::Spectator => true,
            },
        })
        .collect()
}
```

(`facts_for` also builds every replay stop, `client.rs:663`, so `…/at/{k}` agrees with the live frame.)

**2. `why`** — `explain.rs`, inside the existing `impl Why` (`:397-436`), after `Why::locked` (`:435`):

```rust
    /// A move made by `played`, a bot this build does not run (spec §2.4's
    /// second mask). Answered BEFORE `why::account_parts`, which would read the
    /// converted seat as "a controller this driver will not run … resumed as a
    /// HUMAN seat" — a CLI fact that is false on the web, where nothing
    /// converts a bot's seat. A deliberate exception of `suggest`'s kind: the
    /// twelve `Refusal`s stay `account_parts`' own strings, carried verbatim.
    pub fn foreign(played: &str) -> Self {
        Self {
            ok: None,
            refusal: Some(RefusalDto {
                kind: FOREIGN_BOT_KIND,
                message: format!("This move was made by {played}, which this server doesn't run."),
            }),
        }
    }
```

and after that `impl`'s closing brace, before `impl Provenance` (`:438`):

```rust
/// The tag of `why`'s THIRTEENTH refusal shape (M6.5c2, R-6.5c2.10): a move
/// made by a bot this build does not run. The layer's own, not the CLI's, so it
/// is outside [`kind_of`]'s fence and [`refusal_samples`]' list — the corpus
/// carries it as a sample of its own (`why_foreign.json`).
pub const FOREIGN_BOT_KIND: &str = "foreign_bot";

/// `why`'s foreign-bot pre-empt: `Some` when the event `why` addresses is a
/// move made by a FOREIGN bot seat of `record`, `None` to let
/// `why::account_parts` answer.
///
/// - **Bare `why`** addresses the most recent bot move, and every bot a web
///   table deals is one contestant (`tables::contestants_of`), so a record
///   that names a foreign bot names no other — it is answered here.
/// - **`why <n>`** is answered here only when event `n` is a move whose mover
///   — `state_at(record, n).current_player`, the CLI's own attribution — is a
///   foreign seat. A human's move, an event out of range, a setup reveal or a
///   replay that fails are `account_parts`' to word.
pub fn foreign_why(record: &yokohama_core::GameRecord, rules: &Rules, event: Option<usize>) -> Option<Why> {
    let foreign = crate::bots::foreign(record);
    let (_, first) = foreign.first()?;
    let Some(n) = event else { return Some(Why::foreign(&crate::bots::label(first))) };
    if !matches!(record.events.get(n), Some(yokohama_core::RecordedEvent::Move(_))) {
        return None;
    }
    let mover = yokohama_core::state_at(record, n, rules).ok()?.current_player.0;
    foreign.iter().find(|(seat, _)| *seat == mover).map(|(_, d)| Why::foreign(&crate::bots::label(d)))
}
```

and `api.rs:1364-1366` (`why`), between the lock's block and `account_parts`:

```rust
        (repl.session.mode, repl.session.record().clone(), repl.bots().clone())
    };
    // **A move a foreign bot made is the layer's to word** (M6.5c2,
    // R-6.5c2.10): `account_parts` would call that seat a diagnostic that
    // "resumed as a HUMAN seat". Outside the lock, over the clone, as the
    // search below is.
    if let Some(refused) = explain::foreign_why(&record, games.rules(), q.event) {
        return Ok(refused);
    }
    Ok(
```

The lockdown check stays FIRST (`:1361-1363`): a locked session's `why` refuses as locked, foreign or not.

**3. Undo** — `api.rs:604-605`, the first thing in `undo_allowed`:

```rust
) -> Result<(), String> {
    // **Nobody undoes a game played against a bot this build does not run**
    // (M6.5c2, R-6.5c2.10; spec §2.4's third mask), on the one-human path and
    // the shared path alike, and so FIRST. Such a game reaches here only
    // finished (`Games::live_handle` refuses one that is not), and its bot's
    // seat is a human one inside the resident `Repl` — so the CLI's undo, which
    // pops back to the last human move, would pop the BOT's final move and
    // leave a game nobody can play.
    if let Some((_, played)) = crate::bots::foreign(repl.session.record()).first() {
        return Err(format!(
            "undo is not offered on a game played against {}, which this server doesn't run",
            crate::bots::label(played),
        ));
    }
    if table.distinct_humans() <= 1 {
```

**The restatement** — `crates/yokohama-worker/src/lib.rs:264-279`: the doc's "`{"error": …}`, with `"status"` beside
it when `table_status` is set" gains "and `"foreign_bot"` beside it when `foreign_bot` is (M6.5c2)", "pins both arms"
becomes "pins all three arms", and:

```rust
pub(crate) fn error_body(e: &ApiError) -> String {
    let mut body = serde_json::json!({ "error": e.message });
    if let Some(status) = e.table_status {
        body["status"] = status.into();
    }
    if let Some(pair) = &e.foreign_bot {
        body["foreign_bot"] = serde_json::to_value(pair).expect("the pair serializes");
    }
    body.to_string()
}
```

- [ ] **Step 6: Watch them pass**, then the three crates:

```bash
cargo test -p yokohama-web --lib masks_the_conversion
cargo test -p yokohama-web --lib a_spectator_reads_a_finished_foreign_game
cargo test -p yokohama-worker a_refusal_body_is_the_one_dispatch_writes
cargo test -p yokohama-web --lib
cargo test -p yokohama-worker
```

**Mutations, measured in D2's export:** reverting the warnings line to `session.load_warnings.clone()` fails
`masks_the_conversion` on the array; `is_bot: repl.bots().is_bot(p.id)` fails it on `[false, false]`; dropping the
Worker's third arm fails its pin. (`browser_played_pin` stays green: the pair's own bot IS production, so no pre-empt
fires and every bot move still reconstructs.)

- [ ] **Step 7: The corpus's three samples — first the failing check.** In `wire_fixtures.rs`'s `walk_with` tail
(Task 3's), between `out.push(("table_bot"…` and `let (archived, archive_script) = …`:

```rust
    // **A game played against a bot this build does not run** (M6.5c2,
    // R-6.5c2.10), from an `Api` of its own: the committed browser-played pair
    // with its bot seat renamed to `v2-padded` — cut at 300 events (not over:
    // every game route refuses it, and its lobby row says so) and whole
    // (finished: it opens, and `why` names its bot). The three payloads are the
    // client's only sources for the refusal it branches on, the lobby row that
    // does not run here, and `why`'s thirteenth refusal shape.
    let foreign = Api::new();
    for (id, cut) in [("wire-foreign", Some(300)), ("wire-foreign-over", None)] {
        let (record, table) = foreign_pair(id, "v2-padded", cut);
        let games = &foreign.state.games;
        games.load_document(yokohama_web::store::Doc::Record, id, &record).expect("a fresh store");
        games.load_document(yokohama_web::store::Doc::Table, id, &table).expect("a fresh store");
    }
    let (status, foreign_bot_409) =
        foreign.call_as(Some(PAIR_PLAYER), "GET", "/api/games/wire-foreign", None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{foreign_bot_409}");
    assert_eq!(foreign_bot_409["foreign_bot"]["played"], "V2", "{foreign_bot_409}");
    out.push(("foreign_bot_409".to_string(), foreign_bot_409));
    let (status, table_foreign) =
        foreign.call_as(Some(PAIR_PLAYER), "GET", "/api/games/wire-foreign/table", None).await;
    assert_eq!(status, StatusCode::OK, "{table_foreign}");
    assert_eq!(table_foreign["bot"]["runs_here"], false, "premise: the row says this build does not run it");
    out.push(("table_foreign".to_string(), table_foreign));
    let (status, why_foreign) =
        foreign.call_as(Some(PAIR_PLAYER), "GET", "/api/games/wire-foreign-over/why", None).await;
    assert_eq!(status, StatusCode::OK, "{why_foreign}");
    assert_eq!(why_foreign["refusal"]["kind"], "foreign_bot", "{why_foreign}");
    out.push(("why_foreign".to_string(), why_foreign));
```

and `aside.extend(foreign.script.take());` after `aside.extend(bots.script.take());`. Before `bot_table_body`:

```rust
/// The committed browser-played pair, which carries a seed and therefore lives
/// OUTSIDE the corpus (`tests/browser_played_pin.rs`'s note); nothing it holds
/// but what a payload serves reaches `web/src/wire/`.
const PAIR_RECORD: &str = include_str!("fixtures/browser-played-2p.json");
const PAIR_TABLE: &str = include_str!("fixtures/browser-played-2p.table.json");
/// The one address a local game is played by, and so the pair's seat 0.
const PAIR_PLAYER: &str = "local@browser";

/// The pair re-seated under `id` as a game played against ANOTHER bot:
/// seat 1's recorded controller renamed, its version and knobs kept, the
/// table's mirror naming the same bot; with `cut`, the first `cut` events only,
/// re-stamped, and the mirror moved with them (not over). COPIED from the
/// layer's `games::test_support::foreign_pair`, a test-only helper of another
/// crate target, which cannot be imported (the receipt cells' ruling R-06).
fn foreign_pair(id: &str, controller: &str, cut: Option<usize>) -> (String, String) {
    use yokohama_core::{state_at, GameRecord};
    let mut record: GameRecord = serde_json::from_str(PAIR_RECORD).expect("the pair's record");
    let mut table: yokohama_web::table::TableDoc = serde_json::from_str(PAIR_TABLE).expect("the pair's table");
    table.id = id.to_string();
    let seat = &mut record.run.as_mut().expect("the pair's run provenance").seats[1];
    seat.controller = controller.to_string();
    let bot = yokohama_web::bots::BotDescriptor {
        controller: seat.controller.clone(),
        version: seat.version,
        params: seat.params.clone(),
    };
    let mirror = table.turn.as_mut().expect("the pair's mirror");
    mirror.bot = Some(bot);
    if let Some(cut) = cut {
        record.events.truncate(cut);
        let state = state_at(&record, cut, &Rules::standard()).expect("the cut replays");
        record.stamp_terminal_digest(&state);
        *mirror = yokohama_web::table::TurnMirror {
            to_move: state.current_player.0,
            over: false,
            winner: None,
            bot: mirror.bot.take(),
        };
    }
    (serde_json::to_string(&record).expect("a record"), serde_json::to_string(&table).expect("a table"))
}
```

`Walk.aside`'s doc gains "and one holding the committed pair re-seated against another bot". The pair's seed never
reaches a payload: the 409, the table view and the refusal carry none (the leakage pins read every committed file).

```bash
cargo test -p yokohama-web --test wire_fixtures the_committed_wire_fixtures_are_what_the_server_emits
```

Expected: FAIL, "the wire contract moved", naming `foreign_bot_409.json`, `table_foreign.json` and `why_foreign.json`
as `No such file or directory (os error 2)`, and nothing else.

- [ ] **Step 8: Regenerate, and READ the diff.**

```bash
UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures the_committed_wire_fixtures_are_what_the_server_emits
cargo test -p yokohama-web --test wire_fixtures
git status --short web/src/wire web/src/wire-script
```

**Expected, and nothing else**: three NEW files, no modified one, `web/src/wire-script/` untouched —
- `foreign_bot_409.json`: `{"error": "This game was played against V2. This server now runs V4 and can't continue it.", "foreign_bot": {"played": "V2", "runs": "V4"}}`;
- `table_foreign.json`: the `wire-foreign` table, `created_by` `local@browser`, `status` `playing`, `to_move` 1,
  `over` false, `winner` null, and `"bot": {"label": "V2", "runs_here": false}`;
- `why_foreign.json`: `{"ok": null, "refusal": {"kind": "foreign_bot", "message": "This move was made by V2, which this server doesn't run."}}`.

No existing payload moves: no corpus `ClientState` has a foreign seat, and `watching.json`'s bot is production, so the
warnings filter and the record-derived `is_bot` change nothing already committed. The check run reads **12 passed**.

- [ ] **Step 9: The client's half — first the failing check.** In `web/src/wire/contract.test.ts`, after Task 3's four
rows:

```ts
    // M6.5c2: the foreign-bot 409 and its pair, the row that does not run
    // here, and `why`'s thirteenth refusal.
    ["foreign_bot_409", wire.foreignBot409, KEYS.foreignBotError],
    ["foreign_bot_409.foreign_bot", wire.foreignBot409.foreign_bot, KEYS.foreignBot],
    ["table_foreign", wire.tableForeign, KEYS.table],
    ["table_foreign.bot", wire.tableForeign.bot, KEYS.tableBot],
    ["why_foreign", wire.whyForeign, KEYS.whyResponse],
    ["why_foreign.refusal", wire.whyForeign.refusal, KEYS.refusalRow],
```

and, before the `at_k carries the port offer…` cell (`:199`):

```ts
  // **`why`'s thirteenth refusal is the LAYER's** (M6.5c2, review R2-m5): a
  // kind of its own, outside the CLI's twelve that `why_refusals.json` carries
  // whole — and the lobby row and the 409 name the same bot.
  it("the foreign-bot refusal is a kind of its own, and the row and the 409 agree", () => {
    expect(wire.whyForeign.refusal?.kind).toBe("foreign_bot");
    expect(wire.whyRefusals.map((r) => r.kind)).not.toContain("foreign_bot");
    expect(wire.tableForeign.bot).toEqual({ label: wire.foreignBot409.foreign_bot.played, runs_here: false });
    expect(wire.foreignBot409.foreign_bot.runs).toBe(wire.productionBot.label);
  });
```

```bash
cd web && npx tsc -b --noEmit
```

Expected: FAIL — `Property 'foreignBot409' does not exist` (and `tableForeign`, `whyForeign`, `foreignBotError`,
`foreignBot`).

- [ ] **Step 10: `types.ts` and the contract.** `web/src/types.ts`, after `DealingErrorBody` (`:612-614`):

```ts
/** The two labels a foreign-bot refusal names (M6.5c2, R-6.5c2.10): the bot
 *  the game was played against, and the one this server runs. */
export interface ForeignBot {
  played: string;
  runs: string;
}

/** The 409 every game route answers for a game NOT over whose recorded bot
 *  this build does not run — the sentence, and the pair beside it that the
 *  game screen branches on rather than displaying a banner. A FINISHED game
 *  against another bot opens as any finished game does. */
export interface ForeignBotErrorBody extends ApiErrorBody {
  foreign_bot: ForeignBot;
}
```

`web/src/wire/contract.ts`:
- the type import gains `ForeignBot` and `ForeignBotErrorBody` (after `Explanation`, `:69`);
- after `import tableBot …` (Task 3):

```ts
import foreignBot409 from "./foreign_bot_409.json";
import tableForeign from "./table_foreign.json";
import whyForeign from "./why_foreign.json";
```

- after `const _productionBot …` (Task 3):

```ts
// ---- M6.5c2: the foreign-bot rule ---------------------------------------------
//
// The 409 a game not over against another bot answers on every game route —
// the one refusal besides the dealing and open ones that carries a second key —
// the lobby row that says this build does not run its bot, and `why`'s
// THIRTEENTH refusal shape, the layer's own (`explain::Why::foreign`), which
// `why_refusals.json` does not carry because it is not the CLI's.
const _foreignBot409: Widen<ForeignBotErrorBody> = foreignBot409;
const _tableForeign: Widen<TableView> = tableForeign;
const _tableForeignBot: Widen<TableBot> = tableForeign.bot;
const _whyForeign: Widen<WhyResponse> = whyForeign;
const _whyForeignRefusal: Widen<RefusalRow> = whyForeign.refusal;
```

- the `void` line Task 3 added gains a second line
  `  _foreignBot409, _tableForeign, _tableForeignBot, _whyForeign, _whyForeignRefusal,`;
- after `TABLE_BOT` (Task 3):

```ts
const FOREIGN_BOT_ERROR = ["error", "foreign_bot"] as const;
const FOREIGN_BOT = ["played", "runs"] as const;
```

- after `const _pb …` (Task 3):

```ts
const _fe: KeysOf<ForeignBotErrorBody, typeof FOREIGN_BOT_ERROR> = FOREIGN_BOT_ERROR;
const _fb: KeysOf<ForeignBot, typeof FOREIGN_BOT> = FOREIGN_BOT;
```

  and `_tb, _sb, _pb];` becomes `_tb, _sb, _pb, _fe, _fb];`;
- `KEYS` gains `foreignBotError: FOREIGN_BOT_ERROR, foreignBot: FOREIGN_BOT,`;
- `wire` gains, after `productionBot`:

```ts
  // ---- M6.5c2: the foreign-bot rule -------------------------------------
  foreignBot409: as<ForeignBotErrorBody>(foreignBot409),
  tableForeign: as<TableView>(tableForeign),
  whyForeign: as<WhyResponse>(whyForeign),
```

`web/src/local/corpus.node.test.ts`: Task 3's bullet "`table_bot` comes from one more `Api`…" becomes "`table_bot`,
`foreign_bot_409`, `table_foreign` and `why_foreign` come from two more `Api`s of the emitter's (M6.5c2) — one holding
a vs-bot table, one holding the committed pair re-seated against another bot — which no script replays", and
`NOT_SERVED` ends `"production_bot", "table_bot", "foreign_bot_409", "table_foreign", "why_foreign",` (INSERTED; the
list is compared sorted).

`worker/test/corpus.test.ts`: the same bullet, and `THIRD_COLUMN` gets `"foreign_bot_409",`, `"table_foreign",` and
`"why_foreign",` INSERTED in sorted place. **After Tasks 1 and 3 the whole list reads exactly:**

```ts
const THIRD_COLUMN = [
  "explanation_terminal",
  "foreign_bot_409",
  "house_chat_off_404",
  "identity_cases",
  "locked",
  "production_bot",
  "site_space_words",
  "table_bot",
  "table_foreign",
  "watching",
  "why",
  "why_foreign",
  "why_refusals",
  "why_refused",
];
```

```bash
cd web && npx tsc -b --noEmit && npx tsc -p tsconfig.node.json && npm test
```

Expected: green; `npm test` is **+7** over Task 3's reading (six rows and one cell; D2 read `contract.test.ts` 139 →
146, the suite 1060 → 1067).

- [ ] **Step 11: The per-edit gates** (Global Constraints; `npm run build:wasm` rebuilds the browser module the layer
change reaches, and `npm run build:module:debug` the Worker's, before their suites):

```bash
cargo test -p yokohama-web
cargo test -p yokohama-wasm
cargo test -p yokohama-worker
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown
cargo test -p yokohama-web --no-default-features --features server --lib identity::
cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm
cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker
cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm
cd ../worker && npm run build:module:debug && npm run typecheck && npm run test:scripts && npm test
git diff --stat 1e16fe27 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli
```

Counts that move, as deltas over Task 3's reading: the web lib **+6**; `npm test` **+7**; `yokohama-worker` **±0** (the
pin grew, the count did not); both replays' third columns grow by three names (the pool and `test:wasm` keep their
counts). `npm run e2e` is not owed. The fence prints nothing. **The reviewer reads** `git diff 1e16fe27 --
crates/yokohama-web/src` against Tasks 1–3's lists plus: `bots.rs` (three items), `games.rs` (the variant, `Display`,
the site, test support, three cells), `lib.rs` (the field and its docs, the literals, the arm, `IntoResponse`), `api.rs`
(the body, the literals, `undo_allowed`, `why`, three cells), `identity.rs` (one literal), `client.rs` (two masks),
`explain.rs` (the pre-empt).

- [ ] **Step 12: Commit.**

```bash
git add crates/yokohama-web/src/bots.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/lib.rs \
  crates/yokohama-web/src/api.rs crates/yokohama-web/src/identity.rs crates/yokohama-web/src/client.rs \
  crates/yokohama-web/src/explain.rs crates/yokohama-web/tests/wire_fixtures.rs \
  crates/yokohama-worker/src/events.rs crates/yokohama-worker/src/lib.rs \
  web/src/wire web/src/types.ts web/src/local/corpus.node.test.ts worker/test/corpus.test.ts
git status --short
git commit -m "feat(web): the foreign-bot rule — refused before any rebuild while not over, and the four places a finished game masks the CLI's seat conversion"
```

---

### Task 5: What every save names (spec §2.4 "What every save names", "A pin, on every save path"; review m7, R2-m7)

**Measured:** D2 ran every step in an export; the review's chain carries it in `0002` and `0004` and ran the pool over
it, where ONE cell went red for a reason D2 could not see (P-5): the pool's corpus cell compares the Worker's own
game-over save with the axum host's committed record, and `run_id` now differs by host. Step 6 repairs it, measured.

**Files:**
- Modify: `crates/yokohama-web/src/games.rs`:
  - before `:122` — `RunHost` and its `run_id`;
  - `:222-225` — `Games.run_host`;
  - `:255-264` — `with_limits` passes `RunHost::Server`; `:279-310` — `with_parts` takes `run_host` LAST;
  - test support `:1900-1912` (`watched_games`); a cell before `:946`.
- Modify: `crates/yokohama-web/src/tables.rs:1218-1219` — `finalize` stamps the id after the seats.
- Modify: `crates/yokohama-web/src/lib.rs:459-466` — `with_store_chat_and_archive` passes `RunHost::Server`; a cell
  before `:1207`.
- Modify: `crates/yokohama-web/src/api.rs:2190-2199` — the test `games_with` passes `RunHost::Server`.
- Modify: `crates/yokohama-web/tests/browser_played_pin.rs:60-67` — `RunHost::Browser` (it drives the layer as the
  browser host).
- Modify: `crates/yokohama-wasm/src/lib.rs:86-93` — `RunHost::Browser`; a cell before `:308`.
- Modify: `crates/yokohama-worker/src/lib.rs:167-174` — `RunHost::Worker`; `crates/yokohama-worker/src/receipt.rs` — a
  cell before `:671`.
- Modify (regenerated): `web/src/wire-script/archive-walk.json`.
- Modify: `worker/test/corpus.test.ts` — the walk cell's last assertion and its `./helpers` import (P-5).

**Interfaces:**
- Consumes: Task 2's `IdentityMode::Open`, in the `lib.rs` cell.
- Produces (shared names): `RunHost::{Server, Worker, Browser}`, `RunHost::run_id(&self) -> &'static str` →
  `"web/server"`, `"web/worker"`, `"web/browser"`; `Games::with_parts(rules, store, chat, signal, clock, archive,
  run_host: RunHost)` — the host is the LAST parameter.
- `Session::set_run_id` is public (`crates/yokohama-cli/src/session.rs:476-484`) and asserts `set_run_seats` came
  first; every web finalize calls `set_run_seats` for every game, all-human included (`tables.rs:1218`;
  `bot.rs:212-226` emits a `human` entry per seat), so no CLI change and the assertion cannot fire.
- The browser-side save paths (IndexedDB, the export, the backup envelope) are cycle B's pins.

- [ ] **Step 1: Write the failing cells.**

`games.rs`, in the test module, before `:946`:

```rust
    /// **Every game this layer deals names the host it was played on** (M6.5c2,
    /// spec §2.4) — `finalize` stamps `run.run_id` from the `RunHost` the host
    /// passed to [`Games::with_parts`] — and every later write keeps it: a move
    /// and an undo each persist a record whose `run` is the one dealt.
    #[test]
    fn every_game_dealt_here_names_its_host_and_every_later_write_keeps_it() {
        for host in [RunHost::Server, RunHost::Worker, RunHost::Browser] {
            let games = Games::with_parts(
                Arc::new(Rules::standard()),
                Arc::new(MemStore::default()),
                ChatConfig::default(),
                Arc::new(NoSignal),
                Games::ticking_clock(),
                Arc::new(NoArchive),
                host,
            );
            let run_id = |games: &Games| -> Option<String> {
                let (text, _) = games.store().read(Doc::Record, "named").unwrap().unwrap();
                serde_json::from_str::<GameRecord>(&text).unwrap().run.expect("the seats were stamped").run_id
            };
            let handle = playing(&games, &two_people("named", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
            assert_eq!(run_id(&games).as_deref(), Some(host.run_id()), "{host:?}: finalize stamps it");
            let mut repl = handle.lock().unwrap();
            let dealt = repl.session.record().events.len();
            repl.step("1");
            assert!(repl.session.record().events.len() > dealt, "premise: the step applied a move");
            games.persist("named", &repl).unwrap();
            assert_eq!(run_id(&games).as_deref(), Some(host.run_id()), "{host:?}: a move keeps it");
            repl.step("undo");
            assert_eq!(repl.session.record().events.len(), dealt, "premise: the undo popped it");
            games.persist("named", &repl).unwrap();
            assert_eq!(run_id(&games).as_deref(), Some(host.run_id()), "{host:?}: an undo keeps it");
        }
    }
```

`lib.rs`, in the test module, before `:1207` (the `// M6.5b: POST /api/archive` banner):

```rust
    /// **What every save on this host names, carried byte for byte** (M6.5c2,
    /// spec §2.4, "A pin, on every save path"): the record `finalize` writes
    /// through `AppState`'s constructor says `run_id: "web/server"`, and the
    /// three places a server-side save goes — the store, the game-over archive
    /// and the record route — hold the SAME bytes, `run` exactly as dealt.
    #[tokio::test]
    async fn every_save_this_host_writes_names_web_server_byte_for_byte() {
        use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
        use yokohama_cli::session::Session;
        use yokohama_core::{GameConfig, PlayerColor, PlayerId, StartingOrderChoice};

        let dir = tempdir("run-id");
        let store = Arc::new(MemStore::default());
        let state = || {
            Arc::new(AppState::with_store_chat_and_archive(
                store.clone(),
                identity::IdentityMode::Open,
                chat::ChatConfig::default(),
                Arc::new(archive::DirArchive::new(&dir)),
            ))
        };
        let dealer = state();
        start_as(&dealer, "named", ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;
        let (dealt, _) = store.read(Doc::Record, "named").unwrap().unwrap();
        let run = serde_json::from_str::<yokohama_core::GameRecord>(&dealt)
            .unwrap()
            .run
            .expect("finalize stamps the run");
        assert_eq!(run.run_id.as_deref(), Some("web/server"), "this host names itself");

        // Played to its end — the both-bot finish `a_finished_two_person_game`
        // uses — with the dealt `run` stamped back, and persisted through the
        // dealing state, so the game-over site archives the bytes it writes.
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let mut session =
            Session::new_sim(config, ENDED_SEED, vec![StartingOrderChoice::Keep(0); 2], Rules::standard())
                .expect("a game");
        session.set_run_seats(BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT); 2], ENDED_SEED).seat_provenance());
        let mut finished = yokohama_cli::repl::Repl::new(session);
        for _ in 0..400 {
            if finished.session.is_over() {
                break;
            }
            finished.pump_bots();
        }
        assert!(finished.session.is_over(), "the both-bot pump must reach a finished game");
        finished.session.set_run_seats(run.seats.clone());
        finished.session.set_run_id(run.run_id.as_deref().expect("a run id"));
        dealer.games.persist("named", &finished).expect("the finished record");

        let (stored, _) = store.read(Doc::Record, "named").unwrap().unwrap();
        let archived = std::fs::read_to_string(dir.join(format!("rev{}/over/named.json", yokohama_core::SEMANTICS_REV)))
            .expect("the game-over archive landed");
        assert_eq!(archived, stored, "the archive holds the stored bytes");
        // A fresh state over the same store, so the route reads the store.
        let (status, served) = raw(app(state()), "GET", "/api/games/named/record", Some("red@wire.test")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(served, stored.as_bytes(), "the route serves the stored bytes");
        let stored_run = serde_json::from_str::<serde_json::Value>(&stored).unwrap()["run"].clone();
        assert_eq!(stored_run, serde_json::to_value(&run).unwrap(), "`run` is what finalize wrote");
    }
```

(The path is spelled rather than taken from `archive::paths`, which Task 1 turned into a `Result`.)

`crates/yokohama-wasm/src/lib.rs`, in the test module, before `:308`:

```rust
    /// **A game dealt in the browser names it** (M6.5c2, spec §2.4): the
    /// record `finalize` writes on this host says `run_id: "web/browser"`.
    #[test]
    fn a_game_dealt_here_names_web_browser() {
        let _s = serial();
        init();
        let user = Some("local@browser");
        let create = br#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":"20260831","seats":[{"user":"local@browser"},{"bot":true}],"id":"local-named"}"#;
        let (status, body) = dispatch_inner(user, "POST", "/api/games", "", Some("application/json"), create);
        assert_eq!(status, 201, "{body}");
        let keep = br#"{"seat":0,"card":1}"#;
        let (status, body) =
            dispatch_inner(user, "POST", "/api/games/local-named/keep", "", Some("application/json"), keep);
        assert_eq!(status, 200, "{body}");
        let record = read_inner("record", "local-named").unwrap().expect("the keep dealt the game");
        let v: serde_json::Value = serde_json::from_str(&record).expect("a record");
        assert_eq!(v["run"]["run_id"], "web/browser");
    }
```

`crates/yokohama-worker/src/receipt.rs`, in the test module, before `:671`:

```rust
    /// **A game dealt on this host names it, and the plan carries it**
    /// (M6.5c2, spec §2.4): the record `finalize` wrote here says
    /// `run_id: "web/worker"`, and the plan an abandon defers holds those
    /// stored bytes — `run` and all — for R2.
    #[test]
    fn a_game_dealt_here_names_web_worker_and_its_plan_carries_the_stored_bytes() {
        let _s = serial();
        init_inner(&r2(None)).unwrap();
        playing_with_a_move("named", RED, BLUE);
        let record = stored("record", "named").expect("the game's record");
        let v: Value = serde_json::from_str(&record).unwrap();
        assert_eq!(v["run"]["run_id"], "web/worker", "finalize stamps where it was played");
        let _p = pin(1_789_000_950);
        let first = call(RED, "DELETE", "/api/games/named", None);
        assert_eq!(first.status, 500, "premise: the abandon defers its put — {}", first.body);
        assert_eq!(plan_of(&first)["record"], record, "the plan's record is the stored bytes, `run` and all");
    }
```

- [ ] **Step 2: Watch them fail.**

```bash
cargo test -p yokohama-web --lib every_game_dealt_here_names
cargo test -p yokohama-wasm a_game_dealt_here_names_web_browser
cargo test -p yokohama-worker a_game_dealt_here_names_web_worker
```

Expected: the first is a COMPILE failure — no `RunHost` in scope, and `with_parts` takes 6 arguments, 7 supplied. The
other two compile and FAIL on their assertion: `left: Null`, `right: "web/browser"` / `"web/worker"` (no host stamps
an id yet).

- [ ] **Step 3: The host value.** `games.rs`, directly before `/// Every live game, plus the store behind them.`
(`:122`):

```rust
/// **Where a game was played** — the value every save's `run.run_id` names
/// (M6.5c2, spec §2.4 "What every save names"), stamped by `finalize` into the
/// record of every game this layer deals.
///
/// A value each HOST passes to [`Games::with_parts`], the one constructor on
/// all three: the layer cannot tell a NAS from a Durable Object from a browser
/// tab, and a save being evaluated later has to. It follows the BGA importer's
/// `bga-import/<table>` precedent for the same field (`Session::set_run_id`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunHost {
    /// The axum host: the NAS, and a self-hoster's Docker image.
    Server,
    /// The Cloudflare Worker's Durable Objects.
    Worker,
    /// The browser's own module — single player, in a Web Worker.
    Browser,
}

impl RunHost {
    /// The `run.run_id` a record dealt on this host carries.
    pub fn run_id(&self) -> &'static str {
        match self {
            Self::Server => "web/server",
            Self::Worker => "web/worker",
            Self::Browser => "web/browser",
        }
    }
}
```

`Games` gains, after `archive` (`:224`):

```rust
    /// Which host this layer runs on — the `run.run_id` `finalize` stamps
    /// (M6.5c2).
    pub(crate) run_host: RunHost,
```

`with_parts` (`:279-310`): its doc gains "The [`RunHost`] is the host's too (M6.5c2): it is what every record dealt
here names as where it was played.", its signature ends `archive: Arc<dyn Archive>, run_host: RunHost,`, and the
struct literal ends `archive, run_host,`. `with_limits` (`:256-263`) passes, after `Arc::new(NoArchive),`:

```rust
            // The axum host's shape, with every part defaulted — and nothing
            // but a cell builds one this way.
            RunHost::Server,
```

`tables.rs:1218-1219` (`finalize`):

```rust
        session.set_run_seats(bots.seat_provenance());
        // Where it was played (M6.5c2, spec §2.4): AFTER the seats, whose
        // provenance the id lands in — `Session::set_run_id`'s own contract.
        session.set_run_id(self.run_host.run_id());
        let mut repl = Repl::new(session);
```

The six other callers of `with_parts`, each gaining a final argument AFTER the line named (each is the call's
archive argument):
- `lib.rs:465` (`with_store_chat_and_archive`): `games::RunHost::Server,`
- `crates/yokohama-wasm/src/lib.rs:92` (`init`): `yokohama_web::games::RunHost::Browser,`
- `crates/yokohama-worker/src/lib.rs:173` (`init_inner`): `yokohama_web::games::RunHost::Worker,`
- `games.rs:1908` (`watched_games`): `super::RunHost::Server,`
- `api.rs:2197` (the test `games_with`): `crate::games::RunHost::Server,`
- `tests/browser_played_pin.rs:66` (`host_with_the_pair`): `yokohama_web::games::RunHost::Browser,`

A record written before this deploy keeps no `run_id`, and an undo or a move never adds one: only `finalize` stamps
(review m7). `Session::undo` carries `record.run` untouched, which the games cell measures.

- [ ] **Step 4: Watch them pass**, then the three crates:

```bash
cargo test -p yokohama-web --lib every_game_dealt_here_names
cargo test -p yokohama-web --lib every_save_this_host_writes_names_web_server
cargo test -p yokohama-wasm a_game_dealt_here_names_web_browser
cargo test -p yokohama-worker a_game_dealt_here_names_web_worker
cargo test -p yokohama-web
cargo test -p yokohama-wasm
cargo test -p yokohama-worker
```

Expected: green. (`every_save_this_host…` takes about 9 s in a debug build: the both-bot finish, as every
finished-game cell here pays.)

- [ ] **Step 5: Regenerate, and READ the diff.** The walk's own record is dealt through `AppState`, so the archive
walk's request bodies now carry `run_id`:

```bash
cargo test -p yokohama-web --test wire_fixtures the_committed_wire_fixtures_are_what_the_server_emits
UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures the_committed_wire_fixtures_are_what_the_server_emits
cargo test -p yokohama-web --test wire_fixtures
git status --short web/src/wire web/src/wire-script
git diff --stat web/src/wire-script
```

Expected before the regeneration: FAIL naming only `wire-script/archive-walk.json — committed 500600 bytes, the walk
makes 500829 bytes`. **Expected after, and nothing else**: `archive-walk.json` alone, 8 insertions and 8 deletions —
each of its eight rows' `body.record` (the `wire` game's record, pretty or compact as each row sends it) gains
`"run_id": "web/server"` inside its `run` object and changes nowhere else; every `body.table` is byte-identical;
`web/src/wire/` and `walk.json` are untouched (no payload carries a record). A check that reads the eight rows:

```bash
python3 - <<'EOF'
import json, subprocess, difflib
new = json.load(open("web/src/wire-script/archive-walk.json"))
old = json.loads(subprocess.check_output(["git", "show", "HEAD:web/src/wire-script/archive-walk.json"]))
for o, n in zip(old, new):
    assert o["body"]["table"] == n["body"]["table"], n["fixture"]
    ro, rn = o["body"]["record"], n["body"]["record"]
    added = [rn[j1:j2] for t, i1, i2, j1, j2 in difflib.SequenceMatcher(None, ro, rn).get_opcodes() if t != "equal"]
    assert added and all("run_id" in a and "web/server" in a for a in added), (n["fixture"], added)
print("eight records, each gaining run_id alone")
EOF
```

**This moves the LIVE replay** (`worker/scripts/live-replay.mjs`): its bodies now differ from the bytes the bucket
holds under `test/browser/red_at_wire.test/rev7/over/`, so its first send meets a 409 and stops, printing its reset
recipe (`live-replay.mjs:549-558`). **The reset recipe is owed at cycle C's close, before the live replay runs** —
the version bump owes it again for the same keys, so one reset after the bump serves both. Nothing live runs in this
cycle.

- [ ] **Step 6: The pool's corpus cell compares this host's save with the axum host's record (P-5).** The Worker POOL
  replays the regenerated `archive-walk.json` against a fresh local R2 each run: the module's archive route
  strict-replays the new records and answers every `archive_*` payload as committed. But the walk cell's LAST
  assertion (`worker/test/corpus.test.ts`, the end of `replays the walk…`) compares the Worker's own game-over save of
  `wire` byte for byte with `archive-walk.json`'s first record — and the emitter is the axum host, so the saved bytes
  now read `"run_id": "web/worker"` where the committed record reads `"run_id": "web/server"`. The review MEASURED the
  cell red with exactly that one-line diff. Before the fix:

```bash
cd worker && npm run build:module:debug && npm test -- corpus
```

  Expected: `corpus.test.ts › replays the walk…` FAILS at its last assertion, the diff `-"run_id": "web/server"` /
  `+"run_id": "web/worker"`. Then, in `worker/test/corpus.test.ts`, the `./helpers` import (`:4`) becomes
  `import { replaceOnce, settle, tableStub } from "./helpers";` (`replaceOnce(text, from, to)` is `helpers.ts:199`,
  refusing unless `from` occurs exactly once), and the cell's last `expect` becomes:

```ts
    const committed = (ARCHIVE_WALK[0]!.body as { record: string }).record;
    expect(committed, "premise: the emitter is the axum host").toContain('"run_id": "web/server"');
    // The record is one game's on every host, and differs by exactly the host
    // that dealt it (M6.5c2, spec §2.4): the emitter is the axum host, this
    // pool is the Worker, and `run.run_id` says so.
    expect(await saved?.text(), "the walk's game over saved its record, as this host names itself")
      .toBe(replaceOnce(committed, '"run_id": "web/server"', '"run_id": "web/worker"'));
```

```bash
npm test -- corpus
```

  Expected: green — and the whole corpus still replays byte-identically but for that one field, which is the cell's
  own claim. (The first row's record is pretty-printed, so the key reads with a space; a compact row would fail the
  premise loudly rather than pass a weaker check.)

- [ ] **Step 7: The per-edit gates** (Global Constraints; the pool replays the regenerated `archive-walk.json`
through the rebuilt debug module):

```bash
cargo test -p yokohama-web
cargo test -p yokohama-wasm
cargo test -p yokohama-worker
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown
cargo test -p yokohama-web --no-default-features --features server --lib identity::
cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm
cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker
cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm
cd ../worker && npm run build:module:debug && npm run typecheck && npm run test:scripts && npm test
git diff --stat 1e16fe27 -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-cli
```

Counts that move, as deltas over Task 4's reading: the web lib **+2**; `yokohama-wasm` **+1**; `yokohama-worker` **+1**;
the pool **±0** (Step 6's cell repaired, not added). `npm run e2e` is not owed: nothing a browser shows moved (the
record a local game writes now carries `run_id`, which cycle B pins through IndexedDB, the export and the backup
envelope). **The reviewer reads** `git diff 1e16fe27 -- crates/yokohama-web/src` against Tasks 1–4's lists plus:
`RunHost`, the field, `with_parts`/`with_limits`, `finalize`'s one line, `AppState`'s argument, and the three cells'
files.

- [ ] **Step 8: Commit.**

```bash
git add crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/lib.rs \
  crates/yokohama-web/src/api.rs crates/yokohama-web/tests/browser_played_pin.rs \
  crates/yokohama-wasm/src/lib.rs crates/yokohama-worker/src/lib.rs crates/yokohama-worker/src/receipt.rs \
  web/src/wire-script/archive-walk.json worker/test/corpus.test.ts
git status --short
git commit -m "feat(web): every save names where it was played — RunHost on with_parts, run.run_id stamped at finalize on all three hosts"
```

---

### Task 6: The logging seam, and the refinish rule's layer half (spec §2.5, §2.6; review M3, R2-m3)

**Measured:** D3 wrote this task without compiling it; the review then applied it verbatim in its chain (`0005`, `0006`)
and MEASURED: the layer, the module and the browser host compile; its 8 lib cells, 1 module cell and the console cell
pass; the lib is +8 and the worker +1; `test:wasm` and `npm test` read as D3 predicted. One thing D3 could not see
(P-6): the pool's hand-built `Reply` fakes carry no `logs`, so the new `parseLogs` threw on three existing cells. Step
12 gives each fake `logs: "[]"`. **Tasks 1–5 land first and move some of the line numbers below**; each step names the
code it edits by content as well as by line, so an implementer finds it after the shift.

**Files:**
- Create:
  - `crates/yokohama-web/src/log.rs`: the seam, the five events, `StderrLog`, and three cells;
  - `crates/yokohama-worker/src/log.rs`: `ShellLog`, `LogLine`;
  - `web/src/local/log.node.test.ts`: the browser host's console line, read through the compiled module.
- Modify, in `crates/yokohama-web/src/`:
  - `lib.rs:57-72`: add `pub mod log;` after `pub mod identity;` (`:67`).
  - `lib.rs:459-466`: `AppState::with_store_chat_and_archive` appends `.with_log(Arc::new(log::StderrLog))` to the
    `Games::with_parts(…)` expression that initializes `games`.
  - `games.rs`:
    - the imports (`:27-31`) gain `crate::log::{Log, NoLog}` and `TableDoc`;
    - `struct Games` (`:123-225`) gains `log`;
    - `with_parts` (`:287-310`) sets it to `NoLog`, and `with_log` goes after `with_archive` (`:314-317`);
    - `archive_if_over` (`:672-729`) takes the refinish rule and moves its stamp into `stamp_over`;
    - `archive_over_retry`'s doc (`:738-740`) is corrected;
    - a new free function `same_game` goes beside `bring_alive` (`:834`);
    - the cells follow the M6.5c `archive_over_retry` cells, after `:1859`;
    - `test_support`: `RecordingArchive` (`:1998-2038`) gains a conflict answer, and `RecordingLog` is new.
  - `tables.rs`:
    - the doc sentences at `:369-370` and `:1549-1551`;
    - the three sites, in the listing loop's read-and-check block `:1577-1605`;
    - the cell at `:2521-2530` is extended, and one listing cell is new.
  - `archive.rs`:
    - `PutError` (`:53-77`) becomes `Conflict { key, held }`;
    - `DirArchive::put`'s marker read (`:270`) fills `held`;
    - the test pattern at `:427` changes shape, and the conflict assertion at `:547-551` is replaced.
  - `api.rs:1945`: `PutError::Conflict(_)` becomes `PutError::Conflict { .. }`.
- Modify, in `crates/yokohama-worker/src/`:
  - `lib.rs`: the module list (`:30-36`), the statics (`:65-75`), `init_inner` (`:153-181`); `ReplyData` (`:191-211`),
    the `Reply` getters (`:226-262`); `host_refusal` (`:291-300`), `identity_inner` (`:334-340`), `from_the_layer`
    (`:474-479`); one cell's assertion (`:609`); the scan cell's doc (`:1095-1097`).
  - `receipt.rs`: `DEFERRED` (`:59`) becomes `pub(crate)`; `:207` maps a conflict with `held: None`; the docs at
    `:299-302` and `:1151-1153`; one new cell.
  - `house.rs:28-30`: the module doc's sentence about `eprintln!`.
- Modify, in `worker/`:
  - `src/module.ts:85-110`: `readReply` reads `logs`, and `parseLogs` is new after `parseBumps`;
  - `test/worker.test.ts`: two pool cells, and the `Reply` type import;
  - `test/sockets.test.ts:331`, `:506`, `:599` and `test/factory.test.ts:118`, `:136`: the hand-built `Reply` fakes
    gain `logs: "[]"` (P-6).
- Modify, in `crates/yokohama-wasm/`:
  - `src/lib.rs`: the crate doc (`:1-5`); `ConsoleLog` and the `console.warn` import go after `now()` (`:56-68`);
    `init()` (`:84-95`); the scan cell (`:321-375`) is renamed, with a second anti-vacuity partner.
  - `Cargo.toml:30-38`: `serde_json` moves from `[dev-dependencies]` to `[dependencies]`, and the `js-sys` comment
    (`:30-31`) is updated.
- Modify: `CLAUDE.md:40` and `:108`, the two sentences that name the browser host's scan cell.

**Interfaces:**
- Consumes:
  - `archive::paths(rev, prefix, why, id) -> Result<(String, String), String>` and `DirArchive::put`'s lexical
    check (Task 1). This task's cells `.expect()` the `Result`.
  - `IdentityMode::Open` and `YOKOHAMA_WEB_IDENTITY=open` (Task 2), for the by-hand NAS step.
  - Task 5's `RunHost` argument inside every `Games::with_parts(…)` call. This task adds a builder step AFTER the call
    and never an argument.
- Produces:
  - `yokohama_web::log::{Log, NoLog, StderrLog, render, OVER_PUT_REFUSED, OVER_UNSTAMPED, OVER_TABLE_GONE,
    KEPT_EARLIER_COMPLETION, LISTING_SKIPPED}` and `Games::with_log(self, Arc<dyn Log>) -> Self`;
  - `PutError::Conflict { key: String, held: Option<String> }`;
  - **the refinish rule inside `archive_if_over`, which cycle C reaches through `stamp_archive` →
    `archive_over_retry`**;
  - the module's `log::{ShellLog, LogLine}` and `ReplyData.logs`/`Reply.logs`;
  - the shell's `readReply` logging each layer line;
  - `receipt::DEFERRED` at `pub(crate)`;
  - `test_support::{RecordingLog, RecordingArchive::conflicting, PutAnswer}` for later layer cells.

**Every print site the seam replaces** (the layer's non-`server` code at the base). `main.rs` keeps its own nine
lines, because it is the server binary and most of them fire before any `Games` exists.

| file:line | today's words | its line |
|---|---|---|
| `games.rs:716` | "…landed, but its stamp did not" (the retried stamp's write) | `archive-over-unstamped` |
| `games.rs:719` | "…landed, but its table is gone" | `archive-over-table-gone` |
| `games.rs:720` | "…landed, but its stamp did not" (the re-read) | `archive-over-unstamped` |
| `games.rs:723` | "…landed, but its stamp did not" (an I/O refusal) | `archive-over-unstamped` |
| `games.rs:727` | "archive of … at game over failed" | `archive-over-refused` |
| `tables.rs:1581` | "…table document does not parse…" | `listing-skipped`, `why: "does not parse"` |
| `tables.rs:1592` | "…could not be read…" | `listing-skipped`, `why: "could not be read"` |
| `tables.rs:1603` | "…is malformed…" | `listing-skipped`, `why: "is malformed"` |

**The browser host's wiring: the console, decided on evidence.**
- **The five archive sites cannot fire there.** `init()` passes `NoArchive`, and `archive_if_over` returns at
  `archive::is_on` (`games.rs:686`).
- **The three listing sites can.** The browser's local list is the module's own listing (spec §2.4, review o6). A
  table document that a different build wrote, or an IndexedDB entry gone bad, is skipped there with no trace
  anywhere — the defect `Games::list`'s own doc names ("Skipped, but never silently", `tables.rs:1545`). A restored
  pair cannot bring one in: `backup::check_pair` runs `check_shape` (`backup.rs:234`).
- **`NoLog` would keep that silence, and the spec rules the console.** Returning the lines to the JS Worker would
  change the host's `Reply`, `web/src/local/worker.ts`'s `call` (`:311-331`), and the `WasmHost` shape the corpus
  replay declares by hand; the console needs none of that.
- **`js_sys` binds no console.** The only route to it through `js_sys` is `js_sys::global()`, whose own code comment
  calls that probe "pretty heavy code-size wise" (`js-sys-0.3.105/src/lib.rs:13649`), in a module every visitor
  precaches. **So: one `wasm-bindgen` import of `console.warn`** — `warn` and never `error`, because the browser walk
  fails a run on any `console.error` (`web/e2e/harness.ts:78`). The scan cell is renamed, because its name would
  otherwise be a false sentence, and gains a partner that finds the import in CODE.

**The Worker module's wiring: through the reply.** `eprintln!` on `wasm32-unknown-unknown` prints nothing, so the
module records each line as it records a bump (`signal.rs`); `from_the_layer`, the one drain every layer call answers
through (`lib.rs:459-479`), hands the lines to `Reply.logs`; `readReply`, the one way the shell takes a reply
(`module.ts:85-110`), logs them.

**The same-game rule** (R-6.5c2.11) lives in `archive_if_over`, the one function both retriers call (`persist` at
`games.rs:663`, `archive_over_retry` at `:755`). A `Conflict` carrying a held record whose `seed`, `config` and
`choices` equal the current record's stamps as a landed put does, and logs `archive-kept-earlier-completion`.
Anything else stays owed and logs `archive-over-refused`: another game, a held text that is no record, or `held:
None` (the Worker's receipt until cycle C).

- [ ] **Step 1: The seam's own cells.**
  1. Create `crates/yokohama-web/src/log.rs` holding ONLY this test module.
  2. Add `pub mod log;` to `lib.rs` after `pub mod identity;`.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// A line is one JSON object, `event` first and then the fields, on ONE
    /// line — the Worker's shell's own shape — and a quote or a newline inside
    /// a field is escaped rather than breaking the line.
    #[test]
    fn a_line_is_one_object_event_first() {
        let line = render(OVER_PUT_REFUSED, &serde_json::json!({ "id": "t1", "error": "disk on fire" }));
        assert_eq!(line, r#"{"event":"archive-over-refused","error":"disk on fire","id":"t1"}"#);
        let odd = render(LISTING_SKIPPED, &serde_json::json!({ "id": "x", "why": "does not parse", "error": "a \"b\"\nc" }));
        assert!(!odd.contains('\n'), "one line: {odd}");
        let back: Value = serde_json::from_str(&odd).expect("one JSON object");
        assert_eq!((back["event"].as_str(), back["error"].as_str()), (Some(LISTING_SKIPPED), Some("a \"b\"\nc")));
        assert_eq!(render(OVER_TABLE_GONE, &Value::Null), r#"{"event":"archive-over-table-gone"}"#);
    }

    /// Five events, five names — and none is a line the Worker's shell writes
    /// ITSELF (`archive-${outcome}` in `worker/src/r2.ts`'s `logOutcome`,
    /// `archive-stamped` and `archive-owed-*` in `worker/src/table.ts`). The
    /// pool reads its archive lines as exact lists, so a layer line under a
    /// shell's name would read as the shell's.
    #[test]
    fn the_events_are_five_names_and_none_is_the_shells() {
        let events = [OVER_PUT_REFUSED, OVER_UNSTAMPED, OVER_TABLE_GONE, KEPT_EARLIER_COMPLETION, LISTING_SKIPPED];
        let mut unique = events.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), events.len(), "{events:?}");
        let shells = [
            "archive-landed",
            "archive-identical",
            "archive-conflict",
            "archive-failed",
            "archive-stamped",
            "archive-owed-dropped",
            "archive-owed-kept",
        ];
        for event in events {
            assert!(!shells.contains(&event), "{event} is a line the shell writes itself");
        }
    }

    /// **The layer prints through this seam and nowhere else** (M6.5c2, spec
    /// §2.6). Every `eprintln!` it had is a [`Log`] line now, and a new one
    /// would be a line the Worker and the browser never hear. Read from the
    /// crate's own source above each file's top-level test module. `main.rs`
    /// is the server binary's and prints its start-up; [`StderrLog`] is the
    /// one print the seam itself makes, and the anti-vacuity partner is that
    /// the scan SAW it.
    #[test]
    fn the_layer_prints_only_through_its_log() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let (mut prints, mut seam) = (Vec::new(), 0usize);
        for entry in std::fs::read_dir(&src).expect("src is readable") {
            let path = entry.expect("an entry").path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") || name == "main.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("readable");
            let lines: Vec<&str> = text.lines().collect();
            let end = lines.iter().position(|l| *l == "#[cfg(test)]").unwrap_or(lines.len());
            for (n, line) in lines[..end].iter().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                if ["println!", "eprint!(", "print!(", "dbg!("].iter().any(|p| line.contains(p)) {
                    if name == "log.rs" {
                        seam += 1;
                    } else {
                        prints.push(format!("{name}:{}: {}", n + 1, line.trim()));
                    }
                }
            }
        }
        assert!(prints.is_empty(), "prints outside the log seam: {prints:#?}");
        assert_eq!(seam, 1, "anti-vacuity: the scan saw the one print it allows, `StderrLog`'s");
    }
}
```

  (`"println!"` matches `eprintln!` as well.)

- [ ] **Step 2: The layer's archive and listing cells.**
  - In `games.rs`'s `test_support`: replace `RecordingArchive`'s `fail: bool` with the answer enum below.
    `recording()` and `failing()` keep their meaning, and `.fail` is read nowhere else (`grep -n '\.fail\b'
    crates/yokohama-web` finds only the impl); add `RecordingLog`.

```rust
    /// What a [`RecordingArchive`] answers every put with.
    pub(crate) enum PutAnswer {
        /// Records the put and answers `Landed`.
        Land,
        /// Refuses before recording: `Failed("disk on fire")`.
        Fail,
        /// Records the put and answers a CONFLICT at the record's key, carrying
        /// `held` — the record already there, as `DirArchive` reads it, or
        /// `None` where an archive cannot say (the Worker's receipt until
        /// M6.5c2's cycle C).
        Conflict(Option<String>),
    }

    /// An [`Archive`](crate::archive::Archive) that REMEMBERS its puts and
    /// answers each the one way it was built to — the shapes the archive-site
    /// cells need.
    pub(crate) struct RecordingArchive {
        pub puts: Mutex<Vec<Put>>,
        pub answer: PutAnswer,
    }

    impl RecordingArchive {
        pub(crate) fn recording() -> Arc<Self> {
            Arc::new(Self { puts: Mutex::new(Vec::new()), answer: PutAnswer::Land })
        }
        pub(crate) fn failing() -> Arc<Self> {
            Arc::new(Self { puts: Mutex::new(Vec::new()), answer: PutAnswer::Fail })
        }
        /// Every put refused as a conflict carrying `held` (M6.5c2, R-6.5c2.11).
        pub(crate) fn conflicting(held: Option<String>) -> Arc<Self> {
            Arc::new(Self { puts: Mutex::new(Vec::new()), answer: PutAnswer::Conflict(held) })
        }
    }

    impl crate::archive::Archive for RecordingArchive {
        fn put(
            &self,
            prefix: &str,
            why: crate::archive::Why,
            id: &str,
            r: &str,
            t: &str,
        ) -> Result<crate::archive::PutOutcome, crate::archive::PutError> {
            if let PutAnswer::Fail = self.answer {
                return Err(crate::archive::PutError::Failed("disk on fire".to_string()));
            }
            self.puts.lock().unwrap().push(Put {
                prefix: prefix.to_string(),
                why,
                id: id.to_string(),
                record: r.to_string(),
                table: t.to_string(),
            });
            match &self.answer {
                PutAnswer::Conflict(held) => Err(crate::archive::PutError::Conflict {
                    key: format!("{prefix}rev7/{}/{id}.json", why.dir()),
                    held: held.clone(),
                }),
                _ => Ok(crate::archive::PutOutcome::Landed),
            }
        }
        fn kind(&self) -> &'static str {
            "recording"
        }
    }

    /// A [`Log`](crate::log::Log) that REMEMBERS its lines — the double every
    /// cell about a log line reads (M6.5c2).
    #[derive(Default)]
    pub(crate) struct RecordingLog {
        pub lines: Mutex<Vec<(&'static str, serde_json::Value)>>,
    }

    impl RecordingLog {
        pub(crate) fn recording() -> Arc<Self> {
            Arc::new(Self::default())
        }
        /// The events logged so far, in order.
        pub(crate) fn events(&self) -> Vec<&'static str> {
            self.lines.lock().unwrap().iter().map(|(event, _)| *event).collect()
        }
        /// The `n`th line's fields.
        pub(crate) fn fields(&self, n: usize) -> serde_json::Value {
            self.lines.lock().unwrap()[n].1.clone()
        }
    }

    impl crate::log::Log for RecordingLog {
        fn line(&self, event: &'static str, fields: serde_json::Value) {
            self.lines.lock().unwrap().push((event, fields));
        }
    }
```

  - Then, in `games.rs`'s test module, after the last M6.5c `archive_over_retry` cell (`:1859`), add:

```rust
    // ---------------------------------------------------------------------
    // The refinish rule (M6.5c2, R-6.5c2.11, spec §2.5) and the log seam
    // (spec §2.6).
    // ---------------------------------------------------------------------

    /// **A same-game conflict at game over STAMPS** (R-6.5c2.11). The earlier
    /// completion is this very game — the same seed, config and choices —
    /// ending on other events, which is what an undo past the end and a
    /// second finish leave at `over/<id>` when the first stamp did not land.
    /// The archive keeps that first ending; the table is stamped as on a landed
    /// put, so it is no longer owed a save the archive will always refuse, and
    /// the line says so. Mutants: the rule removed (nothing stamped); the stamp
    /// without the line.
    #[test]
    fn a_same_game_conflict_at_game_over_stamps_and_says_the_archive_keeps_the_first_ending() {
        let games = games().with_archive(RecordingArchive::failing());
        play_to_the_end(&games, "again");
        // The resident handle is the opening's — `archive_over_retry_completes_…`
        // says why the retry must rebuild from the finished record.
        games.evict_for_test("again");
        let stored = games.store().read(Doc::Record, "again").unwrap().unwrap().0;
        let mut earlier: GameRecord = serde_json::from_str(&stored).expect("the finished record");
        earlier.events.pop();
        let held = serde_json::to_string_pretty(&earlier).expect("a record serializes");
        assert_ne!(held, stored, "premise: the held bytes are ANOTHER ending, which is what makes the put a conflict");

        let log = RecordingLog::recording();
        let archive = RecordingArchive::conflicting(Some(held));
        let games = games.with_archive(archive.clone()).with_log(log.clone());
        games.archive_over_retry("again").expect("the retry");
        assert_eq!(archive.puts.lock().unwrap().len(), 1, "premise: the put was made and met the conflict");
        let (doc, _) = games.read_table("again").unwrap().unwrap();
        assert!(doc.archived_at.is_some(), "a same-game conflict stamps, as a landed put does");
        assert_eq!(log.events(), [crate::log::KEPT_EARLIER_COMPLETION]);
        assert_eq!(
            log.fields(0),
            serde_json::json!({ "id": "again", "key": "rev7/over/again.json" }),
        );
        games.archive_over_retry("again").expect("a second retry");
        assert_eq!(archive.puts.lock().unwrap().len(), 1, "stamped, so it is not put again");
    }

    /// **A DIFFERENT game at the key keeps today's rule** (R-6.5c2.11): on this
    /// host the save stays OWED — no stamp — and the refusal is a line. Each
    /// identity field is varied ALONE, so a rule that compared fewer than all
    /// three would stamp one of these and fail; and the two shapes a held
    /// record takes besides a record — nothing the archive can say (the
    /// Worker's receipt until cycle C) and text that is no record — stay owed
    /// too. One game, retried five times, because each arm leaves it owed.
    #[test]
    fn a_different_game_at_the_key_stays_owed_and_is_said() {
        let games = games().with_archive(RecordingArchive::failing());
        play_to_the_end(&games, "clash");
        games.evict_for_test("clash");
        let current: GameRecord =
            serde_json::from_str(&games.store().read(Doc::Record, "clash").unwrap().unwrap().0).unwrap();
        let varied = |edit: &dyn Fn(&mut GameRecord)| {
            let mut record = current.clone();
            edit(&mut record);
            assert_ne!(record, current, "premise: the edit changed the record");
            serde_json::to_string_pretty(&record).unwrap()
        };
        let arms: [(&str, Option<String>); 5] = [
            ("another seed", Some(varied(&|r| r.seed ^= 1))),
            ("another config", Some(varied(&|r| r.config.has_port_b = !r.config.has_port_b))),
            ("another keep", Some(varied(&|r| r.choices[0] = yokohama_core::StartingOrderChoice::Keep(1)))),
            ("text that is no record", Some("not a record".to_string())),
            ("nothing the archive can say", None),
        ];
        let mut games = games;
        for (what, held) in arms {
            let log = RecordingLog::recording();
            games = games.with_archive(RecordingArchive::conflicting(held)).with_log(log.clone());
            games.archive_over_retry("clash").expect("the retry");
            assert_eq!(games.read_table("clash").unwrap().unwrap().0.archived_at, None, "{what}: the save stays owed");
            assert_eq!(log.events(), [crate::log::OVER_PUT_REFUSED], "{what}");
            assert_eq!(
                log.fields(0),
                serde_json::json!({
                    "id": "clash",
                    "error": "rev7/over/clash.json holds a different record — a game is archived once",
                }),
                "{what}",
            );
        }
    }

    /// **The rule through the NAS's own archive** (R-6.5c2.11): `DirArchive`
    /// READS the held record from the file, so `held` is the file's bytes and
    /// not a double's. A different game's file keeps the save owed; the same
    /// game's earlier ending stamps and is KEPT — its record and its table
    /// both untouched, because the put reads the record before it writes the
    /// table (a conflict writes nothing).
    #[cfg(feature = "server")]
    #[test]
    fn the_directory_archive_keeps_the_first_ending_and_stamps_a_refinish() {
        use crate::archive::{paths, DirArchive, Why};
        let dir = std::env::temp_dir().join(format!("yokohama_refinish_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let games = games().with_archive(RecordingArchive::failing());
        play_to_the_end(&games, "refinish");
        games.evict_for_test("refinish");
        let current: GameRecord =
            serde_json::from_str(&games.store().read(Doc::Record, "refinish").unwrap().unwrap().0).unwrap();
        let (record_key, table_key) =
            paths(yokohama_core::SEMANTICS_REV, "", Why::Over, "refinish").expect("a safe layout");
        std::fs::create_dir_all(dir.join(&record_key).parent().unwrap()).unwrap();
        let first_table = "{\"the first ending's table\":true}";
        std::fs::write(dir.join(&table_key), first_table).unwrap();

        // A DIFFERENT game's record at the key: owed, and said.
        let mut other = current.clone();
        other.seed ^= 1;
        std::fs::write(dir.join(&record_key), serde_json::to_string_pretty(&other).unwrap()).unwrap();
        let log = RecordingLog::recording();
        let games = games.with_archive(Arc::new(DirArchive::new(&dir))).with_log(log.clone());
        games.archive_over_retry("refinish").expect("the retry");
        assert_eq!(games.read_table("refinish").unwrap().unwrap().0.archived_at, None, "another game: owed");
        assert_eq!(log.events(), [crate::log::OVER_PUT_REFUSED]);

        // The SAME game ending earlier: stamped, said, and the archive keeps it.
        let mut earlier = current.clone();
        earlier.events.pop();
        let earlier = serde_json::to_string_pretty(&earlier).unwrap();
        std::fs::write(dir.join(&record_key), &earlier).unwrap();
        games.archive_over_retry("refinish").expect("the retry");
        assert!(games.read_table("refinish").unwrap().unwrap().0.archived_at.is_some(), "the same game: stamped");
        assert_eq!(log.events(), [crate::log::OVER_PUT_REFUSED, crate::log::KEPT_EARLIER_COMPLETION]);
        assert_eq!(std::fs::read_to_string(dir.join(&record_key)).unwrap(), earlier, "the first ending is kept");
        assert_eq!(
            std::fs::read_to_string(dir.join(&table_key)).unwrap(),
            first_table,
            "and its table beside it: a conflict writes nothing",
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **A landed copy whose stamp does not land is SAID** (spec §2.6) — two of
    /// `stamp_over`'s three arms, each through the seam: the table gone under
    /// the stamp, and a stamp the store refuses. The third arm (a retry's own
    /// write refused after a re-read) writes the second arm's event from the
    /// same shape; `the_layer_prints_only_through_its_log` holds it to the
    /// seam.
    #[test]
    fn a_stamp_that_does_not_land_is_said() {
        use std::sync::atomic::{AtomicBool, Ordering};

        /// An archive whose put LANDS after the game has been deleted
        /// underneath it — the table gone before the stamp.
        struct Vanishing {
            store: Arc<dyn Store>,
        }
        impl crate::archive::Archive for Vanishing {
            fn put(&self, _: &str, _: crate::archive::Why, id: &str, _: &str, _: &str)
                -> Result<crate::archive::PutOutcome, crate::archive::PutError> {
                self.store.delete(id).unwrap();
                Ok(crate::archive::PutOutcome::Landed)
            }
            fn kind(&self) -> &'static str {
                "vanishing"
            }
        }
        let games = games().with_archive(RecordingArchive::failing());
        play_to_the_end(&games, "gone");
        games.evict_for_test("gone");
        let store = Arc::clone(games.store());
        let log = RecordingLog::recording();
        let games = games.with_archive(Arc::new(Vanishing { store })).with_log(log.clone());
        games.archive_over_retry("gone").expect("the retry");
        assert_eq!(log.events(), [crate::log::OVER_TABLE_GONE]);
        assert_eq!(log.fields(0), serde_json::json!({ "id": "gone" }));

        /// A `MemStore` that refuses TABLE writes once `failing` is set — a
        /// stamp the disk will not take.
        #[derive(Default)]
        struct RefusedStamps {
            inner: MemStore,
            failing: AtomicBool,
        }
        impl Store for RefusedStamps {
            fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
                self.inner.list(doc)
            }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
                self.inner.read(doc, id)
            }
            fn write_if_version(&self, doc: Doc, id: &str, json: &str, expected: Option<Version>)
                -> Result<Version, WriteError> {
                if doc == Doc::Table && self.failing.load(Ordering::SeqCst) {
                    return Err(WriteError::Io("the disk went away".to_string()));
                }
                self.inner.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.inner.delete(id)
            }
        }
        let store = Arc::new(RefusedStamps::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>)
            .with_archive(RecordingArchive::failing());
        play_to_the_end(&games, "refused");
        games.evict_for_test("refused");
        store.failing.store(true, Ordering::SeqCst);
        let log = RecordingLog::recording();
        let games = games.with_archive(RecordingArchive::recording()).with_log(log.clone());
        games.archive_over_retry("refused").expect("the retry");
        assert_eq!(log.events(), [crate::log::OVER_UNSTAMPED]);
        assert_eq!(log.fields(0), serde_json::json!({ "id": "refused", "error": "the disk went away" }));
    }
```

  - In `tables.rs`'s test module, **extend** `a_failing_archive_at_game_over_still_answers_the_move` (`:2521-2530`):
    build it with a log, and add the last two asserts.

```rust
    /// The first failure rule: a put that fails at game over is SAID (M6.5c2:
    /// through the log seam, where it was an `eprintln!`), the move is still
    /// answered, and the stamp stays UNSET — the retry mark.
    #[test]
    fn a_failing_archive_at_game_over_still_answers_the_move() {
        let log = RecordingLog::recording();
        let games = games().with_archive(RecordingArchive::failing()).with_log(log.clone());
        let (_, _repl) = play_to_the_end(&games, "over-fails"); // every persist returned Ok
        let (doc, _) = games.read_table("over-fails").unwrap().unwrap();
        assert_eq!(doc.archived_at, None, "unset says a retry is owed");
        assert!(games.handle("over-fails").is_ok(), "the game is still served");
        assert_eq!(log.events(), [crate::log::OVER_PUT_REFUSED], "one line, at the persist that ended the game");
        assert_eq!(log.fields(0), serde_json::json!({ "id": "over-fails", "error": "disk on fire" }));
    }
```

  - And add, after `a_malformed_table_document_refuses_typed_and_leaves_the_server_alive`:

```rust
    /// **A table the listing skips is SAID** (M6.5c2, spec §2.6): one
    /// `listing-skipped` line per skipped document, naming it and why, where
    /// each was an `eprintln!` that no wasm host could hear. Three documents,
    /// one per site — one that does not parse, one the store cannot read, one
    /// `check_shape` refuses — in the store's own (sorted) order, and the
    /// healthy table still listed.
    #[test]
    fn a_listing_says_each_table_it_skips_and_why() {
        /// A `MemStore` whose read of one id FAILS — a present file this
        /// process may not read.
        struct OneUnreadable(MemStore);
        impl Store for OneUnreadable {
            fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
                self.0.list(doc)
            }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
                if id == "unread" {
                    return Err("permission denied".to_string());
                }
                self.0.read(doc, id)
            }
            fn write_if_version(&self, doc: Doc, id: &str, json: &str, expected: Option<Version>)
                -> Result<Version, WriteError> {
                self.0.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.0.delete(id)
            }
        }
        let log = RecordingLog::recording();
        let games = Games::new(Arc::new(Rules::standard()), Arc::new(OneUnreadable(MemStore::default())))
            .with_log(log.clone());
        let red = who("red@t.test");
        games.create(&two_people("healthy", CORPUS_SEED), &red).expect("a neighbour table");
        let malformed = serde_json::json!({
            "schema": 3, "id": "malformed", "name": null, "created_by": "red@t.test",
            "created_at": 1_789_000_000u64, "players": 2, "colors": ["Red", "Blue"],
            // `PlayerId::new(7)` panics; `check_shape` is what refuses it first.
            "start_seat": 7,
            "seats": [{ "kind": "human", "user": "red@t.test" }, { "kind": "human", "user": "blue@t.test" }],
            "keeps": [null, null], "seed": CORPUS_SEED, "status": "dealing", "turn": null,
        })
        .to_string();
        for (id, text) in [("unparsed", r#"{"schema":3}"#.to_string()), ("unread", "{}".to_string()), ("malformed", malformed)] {
            games.store().write_if_version(Doc::Table, id, &text, None).expect("the store takes what it is given");
        }
        let listed = games.list_for(&red).expect("one bad file does not fail the listing");
        assert_eq!(listed.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["healthy"]);
        let said: Vec<(&str, String, String)> = log
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|(event, f)| (*event, f["id"].as_str().unwrap().to_string(), f["why"].as_str().unwrap().to_string()))
            .collect();
        let line = |id: &str, why: &str| (crate::log::LISTING_SKIPPED, id.to_string(), why.to_string());
        assert_eq!(
            said,
            [line("malformed", "is malformed"), line("unparsed", "does not parse"), line("unread", "could not be read")],
        );
        let unread = log.fields(2);
        assert_eq!(unread["error"], "permission denied", "the store's own reason rides the line");
    }
```

  - In `archive.rs`'s test module, **replace** the conflict assertion in
    `the_directory_archive_writes_both_files_and_refuses_a_second_put` (`:547-551`) with:

```rust
        // **`held` is the file's bytes, and the sentence never carries them**
        // (M6.5c2, R-6.5c2.11): the game-over site compares the held record in
        // Rust, while this sentence is the abandon's refusal ON THE WIRE
        // (`Games::delete`), where a whole record — its setup seed in it — must
        // never ride.
        let Err(conflict) = a.put("", Why::Abandoned, "sim-3", "{\"record\":2}", "{\"table\":2}") else {
            panic!("a second put of a DIFFERENT record is refused, never versioned");
        };
        let said = conflict.to_string();
        let PutError::Conflict { key, held } = conflict else { panic!("a conflict, not a failure: {said}") };
        assert_eq!(held.as_deref(), Some("{\"record\":1}"), "the record already there, read from the file");
        assert_eq!(key, dir.join("rev7/abandoned/sim-3.json").display().to_string());
        assert_eq!(said, format!("{key} holds a different record — a game is archived once"));
        assert!(!said.contains("\"record\""), "the sentence names the key and never the held record: {said}");
```

  - Every other `PutError::Conflict(_)` pattern in `archive.rs`'s test module becomes `PutError::Conflict { .. }`.
    At the base that is `:427`; Task 1 may have moved it. `grep -n 'Conflict(' crates/yokohama-web/src/archive.rs`
    finds them.

- [ ] **Step 3: Watch the layer's cells fail.**

```bash
cargo test -p yokohama-web --lib log::
```

Expected: COMPILE failures, among them E0425 `cannot find function render` and `cannot find value OVER_PUT_REFUSED`;
E0599 `no method named with_log`; E0769/E0026, because `PutError::Conflict` is a tuple variant and the cells match it
by field. Quote the first dozen error lines in the ledger (they go into the commit body).

- [ ] **Step 4: The seam.** Fill `crates/yokohama-web/src/log.rs` above its test module:

```rust
//! **The layer's log** (M6.5c2, spec §2.6): the lines the layer writes about
//! itself — a game-over save that did not land, a table the listing skipped —
//! through ONE seam on [`Games`](crate::games::Games), so every host hears them.
//!
//! Until M6.5c2 they were `eprintln!`, which reaches the NAS's container log and
//! prints NOTHING on `wasm32-unknown-unknown`: on the Worker a failed stamp in
//! `archive_over_retry` could not report, and in the browser a local game the
//! listing skipped left no trace anywhere. Each host now hands
//! `Games::with_log` its own:
//!
//! - the axum host, [`StderrLog`] (under `server`), beside `main.rs`'s own lines;
//! - the Worker module, a recorder whose lines ride each reply to the shell's
//!   log (`crates/yokohama-worker/src/log.rs`);
//! - the browser host, the console (`crates/yokohama-wasm`);
//! - every other `Games`, [`NoLog`] — the default `Games::with_parts` builds.
//!
//! **A line is an EVENT and its FIELDS**, the shape the Worker's shell logs
//! (`{"event": …, …fields}`), so one line reads the same on every host. The
//! fields are a JSON object and **every value in it is a string**: the Worker's
//! shell parses the lines it is handed, and a string rounds nothing, so the
//! strings rule's reason cannot arise (the shell refuses a line that breaks
//! this). The events are the constants below, so a host that treats one
//! specially — the Worker's recorder, which drops its own deferral — matches a
//! name and never a literal.

use serde_json::Value;

/// Where the layer's own lines go.
///
/// One method over an event and its fields, object-safe, answering nothing —
/// a [`Signal`](crate::signal::Signal)'s shape, for its reason: a write site
/// owes the CALL, and what becomes of the line is the host's.
pub trait Log: Send + Sync {
    /// One line: `event` is one of this module's constants, `fields` a JSON
    /// object of strings.
    fn line(&self, event: &'static str, fields: Value);
}

/// The log of a host that says nothing — the default. Not a test double: it is
/// what every cell's layer runs with.
pub struct NoLog;

impl Log for NoLog {
    fn line(&self, _event: &'static str, _fields: Value) {}
}

/// `{"event": …}` and then `fields`' own keys, on ONE line — `event` first, as
/// the Worker's shell writes its lines. A newline inside a field is escaped, so
/// a line is one line. Anything but an object in `fields` adds nothing.
pub fn render(event: &'static str, fields: &Value) -> String {
    let mut out = format!("{{\"event\":{}", Value::from(event));
    if let Value::Object(map) = fields {
        for (key, value) in map {
            out.push(',');
            out.push_str(&Value::from(key.as_str()).to_string());
            out.push(':');
            out.push_str(&value.to_string());
        }
    }
    out.push('}');
    out
}

/// A game-over put was refused — `{id, error}`, the archive's own reason. The
/// save stays OWED: `archived_at` is unset, which is what says a retry is due.
pub const OVER_PUT_REFUSED: &str = "archive-over-refused";
/// A game-over put landed and its stamp did not — `{id, error}`.
pub const OVER_UNSTAMPED: &str = "archive-over-unstamped";
/// A game-over put landed and its table was gone before the stamp — `{id}`.
pub const OVER_TABLE_GONE: &str = "archive-over-table-gone";
/// A game-over put met an EARLIER COMPLETION of the same game at its key, and
/// the table was stamped (R-6.5c2.11) — `{id, key}`: the archive keeps the
/// first ending.
pub const KEPT_EARLIER_COMPLETION: &str = "archive-kept-earlier-completion";
/// The lobby's listing skipped a table document — `{id, why, error}`, `why`
/// one of `does not parse`, `could not be read`, `is malformed`.
pub const LISTING_SKIPPED: &str = "listing-skipped";

/// The NAS's log (M6.5c2): each line on stderr, beside `main.rs`'s own — the
/// container log an operator reads.
#[cfg(feature = "server")]
pub struct StderrLog;

#[cfg(feature = "server")]
impl Log for StderrLog {
    fn line(&self, event: &'static str, fields: Value) {
        eprintln!("{}", render(event, &fields));
    }
}
// ---- end server ----
```

  (Each `server` item carries its own attribute and the region closes with its marker, so
  `platform_calls_stay_behind_the_server_feature` reads it clean. `eprintln!` is none of its eight patterns.)

- [ ] **Step 5: `Games` holds a log.** In `games.rs`:
  1. **Imports.** Add `use crate::log::{Log, NoLog};` beside `use crate::chat::…`. `use crate::table::{TableStatus,
     TurnMirror};` gains `TableDoc`.
  2. **The field.** In `struct Games`, after `archive` (`:224`; Task 5's `run_host` sits beside it):

```rust
    /// Where the layer's own lines go (M6.5c2, spec §2.6) — [`crate::log`]'s
    /// events: a game-over save that did not land, a table the listing
    /// skipped. [`NoLog`] unless a host hands [`Games::with_log`] its own. They
    /// were `eprintln!`, which reaches the NAS's container log and nothing on
    /// `wasm32-unknown-unknown`.
    pub(crate) log: Arc<dyn Log>,
```

  3. **`with_parts`.** In its struct literal, after `run_host,`, add `log: Arc::new(NoLog),`. Append one sentence to
     its doc: "The log is the one part defaulted — [`NoLog`] — and a host sets it with [`Games::with_log`]
     (M6.5c2)."
  4. **`with_log`.** After `with_archive`:

```rust
    /// This `Games` with a [`Log`] (M6.5c2, spec §2.6) — the builder step each
    /// host takes: [`crate::log::StderrLog`] on the NAS, the Worker module's
    /// recorder, the browser's console. A builder and not an argument of
    /// [`Games::with_parts`], because a host that says nothing is every cell's
    /// layer, and a constructor should not make each of them say so.
    pub fn with_log(mut self, log: Arc<dyn Log>) -> Self {
        self.log = log;
        self
    }
```

- [ ] **Step 6: The refinish rule, and the stamp through the seam.**
  1. **`archive_if_over` (`:685-729`).** Replace its body from the `match self.archive.put(` line to its end:

```rust
        match self.archive.put("", crate::archive::Why::Over, id, record_json, &table_json) {
            // Both outcomes are a landed copy: `Identical` means an earlier put
            // of these very bytes landed and only its stamp did not (M6.5b,
            // spec §2.4), so the retry stamps rather than being refused.
            Ok(_) => self.stamp_over(id, doc, version),
            // **An earlier completion of the SAME game** (M6.5c2, R-6.5c2.11):
            // the archive keeps the first ending, and the table is stamped as on
            // a landed put — it is owed nothing the archive will ever take.
            Err(crate::archive::PutError::Conflict { key, held: Some(held) })
                if same_game(&held, repl.session.record()) =>
            {
                self.log.line(crate::log::KEPT_EARLIER_COMPLETION, serde_json::json!({ "id": id, "key": key }));
                self.stamp_over(id, doc, version);
            }
            // Another game at the key — reachable only through a caller-chosen
            // `NewGame.id` — a held text that is no record, no held text at all,
            // or a failed write: the save stays owed, and the line says why.
            Err(e) => {
                self.log.line(crate::log::OVER_PUT_REFUSED, serde_json::json!({ "id": id, "error": e.to_string() }))
            }
        }
    }

    /// The game-over stamp, after a landed put or a same-game conflict.
    ///
    /// The stamp is what keeps this game to ONE archive copy: the next abandon
    /// puts under `abandoned/`, a DIFFERENT path, so a put that landed without
    /// its stamp would be archived a second time there rather than refused
    /// (`archive::paths` is per reason). Retried once on a version conflict — an
    /// `unlock` or a `join` writing the document in the same instant is the only
    /// other writer left at game over — and every other outcome is a line,
    /// because a landed copy with no stamp is the case an operator most needs to
    /// hear.
    fn stamp_over(&self, id: &str, doc: TableDoc, version: Version) {
        match self.write_table(&doc, Some(version)) {
            Ok(_) => {}
            Err(WriteError::Conflict { .. }) => match self.read_table(id) {
                Ok(Some((mut again, v2))) => {
                    again.archived_at = doc.archived_at;
                    if let Err(e) = self.write_table(&again, Some(v2)) {
                        self.log.line(crate::log::OVER_UNSTAMPED, serde_json::json!({ "id": id, "error": e.to_string() }));
                    }
                }
                Ok(None) => self.log.line(crate::log::OVER_TABLE_GONE, serde_json::json!({ "id": id })),
                Err(e) => {
                    self.log.line(crate::log::OVER_UNSTAMPED, serde_json::json!({ "id": id, "error": e.to_string() }))
                }
            },
            Err(WriteError::Io(e)) => {
                self.log.line(crate::log::OVER_UNSTAMPED, serde_json::json!({ "id": id, "error": e }))
            }
        }
    }
```

  2. **`archive_if_over`'s doc** gains this paragraph after its first:

```rust
    /// **A conflict with an earlier completion of the SAME game stamps** (M6.5c2,
    /// R-6.5c2.11, superseding the M6.5c spec §2.6 arm that stranded it). A game
    /// undone past its end and finished again meets, at `over/<id>`, the record
    /// of its first finish — the same `seed`, `config` and `choices`, other
    /// events — whenever that first finish's stamp did not land (on the Worker,
    /// an undo during the deferred upload). A record of a DIFFERENT game at the
    /// key keeps the old answer: a line, and the save stays owed. Every outcome
    /// that is not a stamp is a [`crate::log`] line.
```

  3. **`archive_over_retry`'s doc** (`:738-740`). Replace "a refused put or a failed stamp is the layer's own log
     line, which a host that cannot hear `eprintln!` learns by reading `archived_at` afterwards." with:

     > a refused put or a failed stamp is a [`crate::log`] line — which since M6.5c2 reaches every host, the
     > Worker's shell included — and a host still learns the OUTCOME by reading `archived_at` afterwards, because
     > a line is a report and not an answer.

  4. **`same_game`**, a free function beside `bring_alive`:

```rust
/// Whether `held` — the record already archived at a finished game's key — is
/// an earlier completion of the SAME game as `record` (M6.5c2, R-6.5c2.11): the
/// same `seed`, `config` and `choices`, which is the whole of a game's setup.
/// The events may differ; that is what a refinish is. Parsed here and never in
/// a host (the strings rule), and a held text that does not parse as a record
/// is no game of this one's.
fn same_game(held: &str, record: &GameRecord) -> bool {
    serde_json::from_str::<GameRecord>(held)
        .is_ok_and(|h| h.seed == record.seed && h.config == record.config && h.choices == record.choices)
}
```

- [ ] **Step 7: The listing's three sites.** In `tables.rs`, the listing loop's read-and-check block (`:1577-1605`)
  becomes:

```rust
            let doc = match self.store.read(Doc::Table, &id) {
                Ok(Some((json, _))) => match serde_json::from_str::<TableDoc>(&json) {
                    Ok(doc) => doc,
                    Err(e) => {
                        self.log.line(
                            crate::log::LISTING_SKIPPED,
                            serde_json::json!({ "id": id.as_str(), "why": "does not parse", "error": e.to_string() }),
                        );
                        continue;
                    }
                },
                // Absent between the listing and the read — a delete that
                // landed in the window. Nothing is wrong and nothing is lost.
                Ok(None) => continue,
                Err(e) => {
                    self.log.line(
                        crate::log::LISTING_SKIPPED,
                        serde_json::json!({ "id": id.as_str(), "why": "could not be read", "error": e }),
                    );
                    continue;
                }
            };
            // **The same fence `read_table` applies**, because this path
            // deliberately does not go through it — and `seats_of` two lines
            // below is exactly the call [`check_shape`] exists to make total.
            if let Err(why) = check_shape(&doc) {
                self.log.line(
                    crate::log::LISTING_SKIPPED,
                    serde_json::json!({ "id": id.as_str(), "why": "is malformed", "error": why }),
                );
                continue;
            }
```

  Then correct the two doc sentences in the same file:
  - **`:1549-1551`**, in `list`'s doc. Replace "through the same `eprintln!` channel `main.rs` reports its own
    failures on, so a corrupt table is visible in the container log." with:

    > through the layer's log (`Games::with_log`, M6.5c2): stderr beside `main.rs`'s own lines on the NAS, the
    > shell's log on the Worker, the console in the browser — so a corrupt table is visible wherever the layer
    > runs.

  - **`:369-370`.** Replace "`pub` since M6.5c (R-6.5c.8): a host that cannot hear the layer's `eprintln!` refuses
    a copy its lobby would drop." with:

    > `pub` since M6.5c (R-6.5c.8): a host refuses at the source a copy its lobby would drop — the house's own
    > check (`crates/yokohama-worker`'s `house_copy_inner`) — because the listing's line arrives only once the row
    > is gone.

- [ ] **Step 8: `PutError`, the directory's `held`, and the match sites.**
  1. In `archive.rs` (`:53-77`):

```rust
/// Why a put was refused: a DIFFERENT record is already at this path, or the
/// write itself failed.
///
/// Two variants rather than one string, because the caller answers them
/// differently and must: a conflict is a 409 the client KEEPS its game on, and
/// a failure is "the archive could not be reached — the game was kept"
/// (R-6.5b.7). A 409 that read as a failure, or a failure that read as a 409,
/// is the shape that deletes somebody's only complete copy.
#[derive(Debug)]
pub enum PutError {
    /// The path holds a different record. `key` is the path, for the operator;
    /// `held` is the record already there where the archive read it —
    /// [`DirArchive`] always does, and `None` where it cannot say (the Worker's
    /// receipt, until its shell hands the held text back) — so the game-over
    /// site can tell an earlier completion of the SAME game from another game
    /// (M6.5c2, R-6.5c2.11). The ROUTE words its own sentence and never this
    /// one, and `held` is in neither: it is a whole record, the setup seed in it.
    Conflict { key: String, held: Option<String> },
    /// The write did not happen. Carries the implementation's own reason.
    Failed(String),
}

impl std::fmt::Display for PutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict { key, .. } => write!(f, "{key} holds a different record — a game is archived once"),
            Self::Failed(e) => f.write_str(e),
        }
    }
}
```

  2. In `DirArchive::put`'s marker read, the second arm `Ok(_) => return Err(PutError::Conflict(record_path.display()
     .to_string())),` (`:270`) becomes:

```rust
                Ok(existing) => {
                    return Err(PutError::Conflict { key: record_path.display().to_string(), held: Some(existing) })
                }
```

  3. **The match sites.**
     - `api.rs:1945`: `Err(PutError::Conflict(_)) => Err(ApiError {` becomes `Err(PutError::Conflict { .. }) =>
       Err(ApiError {`.
     - `crates/yokohama-worker/src/receipt.rs:207`:

```rust
                // `held` rides the receipt once the shell hands the held text back
                // (M6.5c2's cycle C); until then this archive cannot say.
                Kind::Conflict => Err(PutError::Conflict { key: record_key, held: None }),
```

     - At `receipt.rs:1151-1153`, the cell doc's "the conflict the layer words is `PutError::Conflict(record_key)`"
       becomes "…is `PutError::Conflict { key: record_key, .. }`".

- [ ] **Step 9: The NAS host's log.** In `lib.rs`'s `with_store_chat_and_archive`, the `games:` initializer becomes
  the `Games::with_parts(…)` expression as Task 5 left it (its last argument `games::RunHost::Server`), followed by
  the builder step `.with_log(Arc::new(log::StderrLog))`.

- [ ] **Step 10: Watch the layer pass.**

```bash
cargo test -p yokohama-web --lib log::
cargo test -p yokohama-web --lib same_game_conflict
cargo test -p yokohama-web --lib different_game_at_the_key
cargo test -p yokohama-web --lib the_directory_archive
cargo test -p yokohama-web --lib a_stamp_that_does_not_land
cargo test -p yokohama-web --lib a_listing_says_each_table
cargo test -p yokohama-web --lib a_failing_archive_at_game_over
cargo test -p yokohama-web
```

Then `grep -rn 'eprintln!\|println!' crates/yokohama-web/src | grep -v 'src/main.rs'` prints only `log.rs`'s
`StderrLog` line, plus comment lines.

- [ ] **Step 11: The Worker module's cells.**
  - In `crates/yokohama-worker/src/receipt.rs`'s test module, add the cell below.
  - In `lib.rs`'s `identity_answers_the_normalized_caller_or_the_worker_modes_401` (`:609`), extend the last assert
    to `assert!(r.bumps.is_empty() && r.archive.is_none() && r.picker.is_none() && r.logs.is_empty(), "an identity
    read writes nothing");`.

```rust
    /// **The layer's log reaches the reply — but a DEFERRED put is not a line**
    /// (M6.5c2, spec §2.6). On this host every game-over put is deferred first:
    /// `ReceiptArchive::put` records the plan and answers `Failed(DEFERRED)`,
    /// which the layer logs as `archive-over-refused` — true of the put and
    /// false of the save, whose plan rides this very reply for the shell to
    /// upload and log itself (`archive-landed` and the rest). So `ShellLog`
    /// drops that one line, by this crate's OWN word, and forwards every other:
    /// the retry that meets a `conflict` receipt is the layer's refusal, carried
    /// whole. Mutants: the drop removed (the first reply carries a line); every
    /// line dropped, or the drain left out of `from_the_layer` (the second
    /// carries none).
    #[test]
    fn a_deferred_put_is_not_a_line_and_a_refused_retry_is_the_layers() {
        let _s = serial();
        let _p = pin(1_789_000_700);
        init_inner(&r2(None)).unwrap();
        let r = pump_owed_game_over();
        let plan = plan_of(&r);
        assert!(r.logs.is_empty(), "the deferral is its plan's to report: {:?}", r.logs);

        receipt_inner(key(&plan), &outcome("conflict", "the key holds other bytes")).unwrap();
        let s = stamp_archive_inner(OWED);
        assert_eq!(s.body, r#"{"stamped":false,"over":true}"#, "premise: the conflict left the save unstamped");
        let error = format!("rev7/over/{OWED}.json holds a different record — a game is archived once");
        assert_eq!(
            s.logs,
            vec![crate::log::LogLine {
                event: yokohama_web::log::OVER_PUT_REFUSED,
                fields: json!({ "id": OWED, "error": &error }),
            }],
        );
        // Through the getter the shell reads: one `{"event", "fields"}` per line.
        let text = crate::Reply::from(s.clone()).logs();
        assert_eq!(
            serde_json::from_str::<Value>(&text).unwrap(),
            json!([{ "event": "archive-over-refused", "fields": { "id": OWED, "error": &error } }]),
        );
    }
```

- [ ] **Step 12: The two pool cells, and the fakes (P-6).**
  - In `worker/test/worker.test.ts`, the factory import becomes `import { createModule, type Reply } from
    "../gen/factory.js";`.
  - Add this block after `describe("the house, first cut (spec §2.5)", …)`:

```ts
describe("the layer's log (M6.5c2, spec §2.6)", () => {
  // **A line the layer logs during a call is a line of the shell's log.** The
  // layer's `eprintln!` printed nothing on this host. Now the module records
  // each line (`crates/yokohama-worker/src/log.rs`), the reply carries it
  // (`Reply.logs`), and `readReply` logs it as one of the shell's own. A copy
  // the layer's listing cannot read is the one line a bare module can be made
  // to write: `put_copy` stores what it is handed, unread (its doc), and the
  // listing then skips it and says so. Mutants: `readReply` dropping `logs`;
  // the drain left out of `from_the_layer`; `init` handing the layer no log.
  it("a table the listing skips is one line of the shell's log", async () => {
    const mod = createModule(compiled);
    mod.init(configModule.moduleConfig(env, null));
    mod.put_copy("unreadable", '{"schema":3}');
    const { result, lines } = await logsDuring(async () =>
      readReply(mod.dispatch(RED, "GET", "/api/games", "", undefined, new Uint8Array())),
    );
    expect(result.status, result.body).toBe(200);
    expect(lines).toEqual([
      { event: "listing-skipped", id: "unreadable", why: "does not parse", error: expect.stringContaining("missing field") },
    ]);
  });

  // **A field that is not a string is refused, named** — the strings rule's
  // reason made structural: the shell parses these lines, and a string rounds
  // nothing, where this number would. The reply is a stand-in built to the
  // glue's shape, since the module writes no such line. Mutant: the check
  // dropped (the line is logged with the number rounded).
  it("a log line whose field is not a string is refused", () => {
    const fake = {
      status: 200,
      body: "",
      bumps: "[]",
      archive: undefined,
      picker: undefined,
      dropped: undefined,
      logs: '[{"event":"listing-skipped","fields":{"id":"x","seed":18446744073709551615}}]',
      free() {},
    } as unknown as Reply;
    expect(() => readReply(fake)).toThrow(/which is no line/);
  });
});
```

  - **The existing fakes gain `logs: "[]"`** — the retired review's P-6, MEASURED: without this, `parseLogs` throws
    "the module's log lines do not parse: undefined" on three cells (`factory.test.ts › readReply … carries the
    archive plan and the picker row`; `sockets.test.ts › a framing call that carries bumps is logged`; `› a reading
    lost mid-stream to a 404 …`). Each hand-built `Reply` in the pool gets the field, beside its `bumps`:
    - `worker/test/sockets.test.ts:331` — `return { status: missing.status, body: missing.body, bumps: "[]", logs:
      "[]", free() {} } as unknown as Reply;`
    - `worker/test/sockets.test.ts:506` — `const failed = { status: 500, body: '{"error":"the store could not be
      read"}', bumps: "[]", logs: "[]", free() {} };`
    - `worker/test/sockets.test.ts:599` — `return { status: 200, body: reading, bumps: JSON.stringify([{ bump: id }]),
      logs: "[]", free() {} } as unknown as Reply;`
    - `worker/test/factory.test.ts:118` — the fake that carries `archive: '{"key":"k"}'` and `picker: "[]"` gains
      `logs: "[]",` beside them;
    - `worker/test/factory.test.ts:136` — `const reply = { status: 200, body: "", bumps, archive: undefined, picker:
      undefined, logs: "[]", free: () => {} };`

    A fake this list misses shows up as the same "do not parse: undefined" line naming its cell: give it the field
    too, and say so in the report.

- [ ] **Step 13: Watch the module's cells fail.**

```bash
cargo test -p yokohama-worker a_deferred_put_is_not_a_line
```

Expected: COMPILE failure — `no field logs on type ReplyData`, and an unresolved `crate::log`. (The pool cells go red
in Step 16, once there is a module to rebuild.)

- [ ] **Step 14: The module's recorder.** Create `crates/yokohama-worker/src/log.rs`:

```rust
//! **The layer's log on this host** (M6.5c2, spec §2.6): the layer's `Log`,
//! RECORDING each line for the call's reply — `from_the_layer` drains it into
//! `Reply.logs`, and the shell's `readReply` logs each as one of its own lines
//! (`worker/src/module.ts`). A line is recorded rather than printed for
//! `signal.rs`'s reason and one more: this module cannot print at all
//! (`eprintln!` is mute on `wasm32-unknown-unknown`, the reason the seam
//! exists).
//!
//! **One line is dropped, by this crate's own word.** On this host every
//! game-over put is deferred first: `ReceiptArchive::put` records the plan and
//! answers `Failed(DEFERRED)`, which the layer logs as `archive-over-refused` —
//! true of the put, false of the save, whose plan rides the same reply and
//! whose upload the shell logs itself (`archive-landed`, `archive-failed`, …).
//! Forwarded, it would be a "refused" line at every game over. The line is
//! recognised by the layer's event CONSTANT and this crate's own `DEFERRED`,
//! which only the put that records a plan answers — so nothing else can match.

use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;
use yokohama_web::log::{Log, OVER_PUT_REFUSED};

/// One line the layer logged, as `Reply.logs` lists them: `{"event", "fields"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogLine {
    pub event: &'static str,
    pub fields: Value,
}

/// The layer's `Log` for a host that answers one call at a time: every line is
/// appended in call order, and [`ShellLog::drain`] hands the list over and
/// leaves the buffer empty.
#[derive(Default)]
pub struct ShellLog {
    lines: Mutex<Vec<LogLine>>,
}

impl ShellLog {
    /// Every line since the last drain, in call order.
    pub fn drain(&self) -> Vec<LogLine> {
        std::mem::take(&mut *self.lines.lock().expect("the recorded lines"))
    }
}

impl Log for ShellLog {
    fn line(&self, event: &'static str, fields: Value) {
        if event == OVER_PUT_REFUSED && fields.get("error").and_then(Value::as_str) == Some(crate::receipt::DEFERRED) {
            return;
        }
        self.lines.lock().expect("the recorded lines").push(LogLine { event, fields });
    }
}
```

  Then, in `crates/yokohama-worker/src/`:
  - **`receipt.rs:59`** becomes `pub(crate) const DEFERRED: &str = "deferred to the host";`.
  - **The module list.** In `lib.rs`, add `pub mod log;` after `mod house;`, and `use crate::log::{LogLine,
    ShellLog};` beside `use crate::signal::…` (`:57`).
  - **The static.** After `SIGNAL` (`:75`):

```rust
/// The log the one `Games` holds — the SAME `Arc`, so what the layer logs
/// during a call is drained into that call's reply (M6.5c2).
pub(crate) static LOG: Mutex<Option<Arc<ShellLog>>> = Mutex::new(None);
```

  - **`init_inner`.** Before `let games`: `let log = Arc::new(ShellLog::default());`. Append `.with_log(log.clone())`
    to the `Games::with_parts(…)` expression, after its closing parenthesis. Beside `*SIGNAL…`:
    `*LOG.lock().expect("the host's log") = Some(log);`. Its doc's list "the `Games`, its store, its signal and its
    archive together" gains "its log".
  - **`ReplyData`**, after `dropped`:

```rust
    /// The layer's log lines during this call (M6.5c2, spec §2.6), in call
    /// order, less the one deferral `log.rs` drops. Empty from a call that
    /// never reached the layer.
    pub logs: Vec<LogLine>,
```

  - **`Reply`**, a getter after `dropped()`:

```rust
    /// The layer's log lines as JSON: `[]` when it logged nothing, else one
    /// `{"event", "fields"}` per line, which the shell logs as its own.
    #[wasm_bindgen(getter)]
    pub fn logs(&self) -> String {
        serde_json::to_string(&self.data.logs).expect("a log line serializes")
    }
```

  - **The three literals.** `host_refusal` and `identity_inner` gain `logs: Vec::new()`. `from_the_layer`'s becomes
    `ReplyData { status, body, bumps: drain_bumps(), archive: receipt::drain_plan(), picker, dropped: None, logs:
    drain_logs() }`, with, after `drain_bumps`:

```rust
/// Everything the layer logged since the last drain.
fn drain_logs() -> Vec<LogLine> {
    LOG.lock().expect("the host's log").as_ref().expect("init() before any call into the layer").drain()
}
```

  - **Two doc sentences made false by this step.**
    - `receipt.rs:299-302`: replace "because inside `archive_if_over` a refused put or a failed stamp is only an
      `eprintln!`, and that is mute on this host — so this reads the table the stamp is written to;" with "because
      inside `archive_if_over` a refused put or a failed stamp is a log line — which reaches the shell since
      M6.5c2, but is a report and not an answer — so this reads the table the stamp is written to;".
    - `house.rs:28-30`: replace "The house's `list` would drop a document it rejects, and would say so only through
      the layer's `eprintln!`, which is mute on this host." with "The house's `list` would drop a document it
      rejects, and would say so only in the layer's log line — which reaches the shell since M6.5c2, but only once
      the row has left the lobby."

- [ ] **Step 15: The shell logs the layer's lines.** In `worker/src/module.ts`:
  - add `import { log } from "./respond";` after the factory import;
  - `readReply`, with its doc (`:85-110`), becomes the code below, and `parseLogs` follows `parseBumps`.

```ts
/**
 * A reply as plain data — how the shell takes every `Reply` (the plan's names
 * table), so that no caller holds one: every getter is read, the reply is
 * FREED, and only then are the lists parsed, so a list that does not parse
 * cannot leak the reply it came from.
 *
 * A bump this shell does not know is refused, named, rather than dropped: a
 * bump nobody reads is a frame nobody sends.
 *
 * **The layer's own log lines are logged HERE, as the shell's** (M6.5c2, spec
 * §2.6): every reply passes through this function, so no caller can forget
 * them — a game-over stamp that did not land, a table the listing skipped. They
 * report what the layer did during the call, whatever the caller's batch then
 * does. Every field is a string (`crates/yokohama-web/src/log.rs`), and a line
 * that breaks that is refused, named, like a bump: a number here would round.
 */
export function readReply(r: Reply): Replied {
  const status = r.status;
  const body = r.body;
  const bumps = r.bumps;
  const archive = r.archive;
  const picker = r.picker;
  const dropped = r.dropped;
  const logs = r.logs;
  r.free();
  const replied: Replied = {
    status,
    body,
    bumps: parseBumps(bumps),
    ...(archive === undefined ? {} : { archive }),
    ...(picker === undefined ? {} : { picker }),
    ...(dropped === undefined ? {} : { dropped }),
  };
  for (const line of parseLogs(logs)) log(line.event, line.fields);
  return replied;
}
```

```ts
/** One line the layer logged during a call (`crates/yokohama-worker/src/log.rs`). */
type LayerLine = { event: string; fields: Record<string, string> };

function parseLogs(text: string): LayerLine[] {
  let list: unknown;
  try {
    list = JSON.parse(text);
  } catch {
    throw new Error(`the module's log lines do not parse: ${text}`);
  }
  if (!Array.isArray(list)) throw new Error(`the module's log lines are not a list: ${text}`);
  for (const line of list) {
    const shaped = line !== null && typeof line === "object" && !Array.isArray(line);
    const { event, fields } = (shaped ? line : {}) as { event?: unknown; fields?: unknown };
    const ok =
      shaped &&
      Object.keys(line as object).length === 2 &&
      typeof event === "string" &&
      fields !== null &&
      typeof fields === "object" &&
      !Array.isArray(fields) &&
      Object.values(fields as object).every((v) => typeof v === "string");
    if (!ok) {
      throw new Error(
        `the module's log lines carry ${JSON.stringify(line)}, which is no line ({event, fields}, every field a string)`,
      );
    }
  }
  return list as LayerLine[];
}
```

- [ ] **Step 16: Watch the Worker pass.**

```bash
cargo test -p yokohama-worker
cd worker && npm run build:module:debug && npm run typecheck && npm test
```

Expected:
- the new module cell passes, and the crate reads **+1** over Task 5's reading;
- the pool reads **+2** (the two new cells) — with Step 12's fakes; without them the review measured three cells red
  on "the module's log lines do not parse: undefined";
- **`archive.test.ts`'s exact `archiveLines` cells are untouched and green.** That is the second witness that the
  deferral is dropped: `a failing put is retried by the alarm` asserts one archive line across a game-ending move
  (`archive.test.ts:472-474`).

- [ ] **Step 17: The browser host's console, and its node cell first.** Create `web/src/local/log.node.test.ts`:

```ts
// **The browser host's log is the console** (M6.5c2, spec §2.6).
//
// The layer's lines were `eprintln!`, which prints NOTHING on
// `wasm32-unknown-unknown`: a local game the listing skipped — a document
// another build wrote, or a store gone bad — left no trace anywhere, the defect
// `Games::list`'s own doc names ("skipped, but never silently"). Since M6.5c2
// the browser host hands the layer a console log (`crates/yokohama-wasm`'s
// `ConsoleLog`). Natively that is a stub — the crate's own cells have no
// console to read — so the line is read HERE, through the compiled module,
// where it is real.
//
// Its own project (`vitest.node.config.ts`, `npm run test:wasm`), for
// `corpus.node.test.ts`'s reason.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { describe, expect, it, vi } from "vitest";

const here = path.dirname(fileURLToPath(import.meta.url));
const modulePath = path.resolve(here, "../../wasm-node/yokohama_wasm.js");

/** The three exports this file drives, declared HERE — `corpus.node.test.ts`'s
 *  ruling P-2, for its reason. */
interface WasmHost {
  init(): void;
  load(doc: string, id: string, text: string): void;
  dispatch(
    user: string | null | undefined,
    method: string,
    path: string,
    query: string,
    content_type: string | null | undefined,
    body: Uint8Array,
  ): { status: number; body: string; free(): void };
}

const have = fs.existsSync(modulePath);
if (!have && !process.env.YOKOHAMA_WASM_OPTIONAL) {
  throw new Error(
    "the browser host's module is not built, so this command tests nothing — run `npm run " +
      "build:wasm` first (or set YOKOHAMA_WASM_OPTIONAL=1 to skip it, which tests NOTHING)",
  );
}
if (!have) {
  // Straight to the file descriptor, for `corpus.node.test.ts`'s measured reason.
  process.stderr.write(
    "\nYOKOHAMA_WASM_OPTIONAL is set and the module is absent: the browser host's log ran NO\n" +
      "assertions at all.\n\n",
  );
}

describe.skipIf(!have)("the browser host's log", () => {
  // Mutants: `init` handing the layer no log (no warning); `console.error` for
  // `console.warn` (the spy sees nothing — and the browser walk would fail a
  // run on it); a line that is not one JSON object.
  it("a table the listing skips is one console warning, and the listing still answers", () => {
    const wasm = createRequire(import.meta.url)(modulePath) as WasmHost;
    wasm.init();
    wasm.load("table", "local-0000000000000bad", '{"schema":3}');
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      const reply = wasm.dispatch("local@browser", "GET", "/api/games", "", undefined, new Uint8Array());
      const [status, body] = [reply.status, reply.body];
      reply.free();
      expect(status, body).toBe(200);
      expect((JSON.parse(body) as { games: unknown[] }).games).toEqual([]);
      expect(warn.mock.calls.map(([line]) => JSON.parse(String(line)) as unknown)).toEqual([
        {
          event: "listing-skipped",
          id: "local-0000000000000bad",
          why: "does not parse",
          error: expect.stringContaining("missing field"),
        },
      ]);
    } finally {
      warn.mockRestore();
    }
  });
});
```

```bash
cd web && npm run build:wasm && npx vitest run -c vitest.node.config.ts src/local/log.node.test.ts
```

Expected: the one cell FAILS on `toEqual` — the spy saw `[]` (the host still runs `NoLog`).

- [ ] **Step 18: `ConsoleLog`, and the scan's new name.**
  1. **The manifest.** In `crates/yokohama-wasm/Cargo.toml`:
     - move `serde_json = "1"` from `[dev-dependencies]` to `[dependencies]`, commented as:

       ```toml
       # For ONE type name: the layer's `Log::line` takes a `serde_json::Value`
       # (M6.5c2). The layer already puts `serde_json` in the built `.wasm`, so
       # this adds no code — and the one cell reading the backup report's shape
       # still uses it.
       ```

     - the `js-sys` comment becomes: "The clock: `js_sys::Date::now`, one of this crate's two platform calls — the
       other is the console, a `wasm-bindgen` import in `src/lib.rs` (M6.5c2) — and its own scan says so."
  2. **`ConsoleLog`**, in `src/lib.rs` after `now()`:

```rust
/// The layer's log on this host (M6.5c2, spec §2.6): the browser's console.
///
/// **`console.warn`, never `console.error`.** A line here reports a table the
/// local listing skipped — a document another build wrote, or a store that
/// went bad — which the page survives; and the browser walk fails a run on any
/// `console.error` (`web/e2e/harness.ts`'s `watchForBreakage`). One JSON line,
/// the shape the Worker's shell logs, so a line reads the same on every host.
///
/// Natively — this crate's own cells and the lint gate — there is no console
/// and a `wasm-bindgen` import is a stub that panics, so the line goes nowhere;
/// `web/src/local/log.node.test.ts` reads it through the COMPILED module.
struct ConsoleLog;

impl yokohama_web::log::Log for ConsoleLog {
    fn line(&self, event: &'static str, fields: serde_json::Value) {
        #[cfg(target_arch = "wasm32")]
        {
            console_warn(&yokohama_web::log::render(event, &fields));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (event, fields);
        }
    }
}

/// `console.warn` — this host's second platform call, beside the clock
/// (M6.5c2). An import rather than a reach through `js_sys`: `js_sys` binds no
/// console, and the probe `js_sys::global()` would take is, by its own doc,
/// "pretty heavy code-size wise", in a module every visitor precaches.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = warn)]
    fn console_warn(line: &str);
}
```

  3. **`init()`.** Append `.with_log(Arc::new(ConsoleLog))` to its `Games::with_parts(…)` expression. Its doc
     becomes "Build a FRESH `Games` over a fresh `MemStore`, whose log is the console (M6.5c2), replacing any
     earlier one."
  4. **The crate doc** (`:1-5`): "`NoArchive` and a settable clock" becomes "`NoArchive`, a settable clock and the
     console for the layer's log"; "Its platform calls are the clock and the bindgen glue and nothing else —
     [`this_host_makes_no_platform_call_but_the_clock`] holds that" becomes "Its platform calls are the clock, the
     console and the bindgen glue, and nothing else —
     [`this_host_makes_no_platform_call_but_the_clock_and_the_console`] holds that".
  5. **The scan cell** (`:321-375`): renamed `this_host_makes_no_platform_call_but_the_clock_and_the_console`; its
     doc's first sentence reads "**The crate's own scan** (spec §2.1): this is host code, and its platform calls are
     the clock and, since M6.5c2, the console the layer's log writes to"; "Two anti-vacuity partners" becomes
     "Three"; it gains the partner below, and the rest is unchanged.

```rust
        let mut console_in_code = false;
        // … inside the per-line loop, beside the clock's partner:
                if !is_comment && line.contains("js_namespace = console") {
                    console_in_code = true;
                }
        // … after the clock's assert:
        assert!(console_in_code, "anti-vacuity: the console import is CODE, not prose");
```

  6. **The sentences that name the old cell.**
     - `crates/yokohama-worker/src/lib.rs:1095-1097`, "copied from `yokohama-wasm`'s
       `this_host_makes_no_platform_call_but_the_clock` (`crates/yokohama-wasm/src/lib.rs`)", becomes "copied from
       `yokohama-wasm`'s scan (`crates/yokohama-wasm/src/lib.rs` — named for the clock and the console there since
       M6.5c2, when that host gained the console)".
     - `CLAUDE.md:40`: "among them its platform-call scan with `js_sys` the one allowed call" becomes "among them its
       platform-call scan, whose two allowed calls are `js_sys`'s clock and the console import".
     - `CLAUDE.md:108`: "the wasm crate's `this_host_makes_no_platform_call_but_the_clock` holds that crate to the
       clock and the bindgen glue" becomes "the wasm crate's
       `this_host_makes_no_platform_call_but_the_clock_and_the_console` holds that crate to the clock, the console
       the layer's log writes to, and the bindgen glue". (Two one-line edits to a current-state document, made here
       so the sentences stay true; the cycle's other CLAUDE.md work is cycle C's.)

- [ ] **Step 19: Watch the browser host pass.**

```bash
cargo test -p yokohama-wasm
cargo build --release --target wasm32-unknown-unknown -p yokohama-wasm
cd web && npm run build:wasm && npm run test:wasm
```

Expected: `yokohama-wasm` **±0** over Task 5's reading (one cell renamed, none added); the wasm32 build compiles the
import; `test:wasm` **+1** in **+1** files (the new file).

- [ ] **Step 20: See the NAS's line once, by hand.** No cell observes stderr in-process, so this is how the wiring is
  tried rather than read. One foreground command, which ends the server itself; `curl --retry` does the waiting, so
  nothing sleeps:

```bash
cd "$(git rev-parse --show-toplevel)" && cargo build -q -p yokohama-web \
 && d=$(mktemp -d) && printf '{"schema":3}' > "$d/unreadable.table.json" \
 && (YOKOHAMA_WEB_DIR="$d" YOKOHAMA_WEB_IDENTITY=open YOKOHAMA_WEB_PORT=18731 \
      target/debug/yokohama-web >/dev/null 2>"$d/stderr.txt" & echo $! > "$d/pid") \
 && curl -s --retry 30 --retry-connrefused --retry-delay 1 \
      -H 'X-Yokohama-Claimed-User: red@t.test' http://127.0.0.1:18731/api/games; echo \
 ; kill "$(cat "$d/pid")"; cat "$d/stderr.txt"; rm -rf "$d"
```

Expected: the listing `{"games":[],"next":null}`; on stderr one line:
`{"event":"listing-skipped","error":"missing field `id` at line 1 column 12","id":"unreadable","why":"does not parse"}`
(serde's wording may differ; the event, id and why must not). Record the line in the ledger.

- [ ] **Step 21: The per-edit gates** (Global Constraints), all of them, including `cargo test -p yokohama-cli` (the
  browser host's manifest moved), the refusing build's identity cells, and both wasm32 builds. The corpus is not
  regenerated: nothing on the wire moves, and `cargo test -p yokohama-web --test wire_fixtures` stays green without
  the variable. Counts that move, as deltas over Task 5's reading: the web lib **+8** (three in `log.rs`, four in
  `games.rs`, one in `tables.rs`); `yokohama-worker` **+1**; the pool **+2**; `test:wasm` **+1**.

  **The reviewer reads** `git diff 1e16fe27 -- crates/yokohama-web/src` against Tasks 1–5's lists plus exactly:
  `log.rs`; `with_log`, `same_game`, `stamp_over` and the rule; the listing's three sites and the doc corrections;
  `PutError`; the one match site; `AppState`'s builder step; the cells.

- [ ] **Step 22: The e2e** — owed: the browser module gained an import, and the web target's glue is loaded in a
  browser only by the walk. `cd web && npm run e2e`, sequenced; if the tool's ten-minute ceiling would cut it, run it
  in the background with its output to `.superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/task-6-e2e.txt` and
  poll that file until the summary line appears. Expected: all passed (CLAUDE.md's reading 61 in 11), with no
  `console.error` — a warning is never a failure there.

- [ ] **Step 23: Commit.**

```bash
git add crates/yokohama-web/src/log.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/games.rs \
  crates/yokohama-web/src/tables.rs crates/yokohama-web/src/archive.rs crates/yokohama-web/src/api.rs \
  crates/yokohama-worker/src/log.rs crates/yokohama-worker/src/lib.rs crates/yokohama-worker/src/receipt.rs \
  crates/yokohama-worker/src/house.rs worker/src/module.ts worker/test/worker.test.ts \
  worker/test/sockets.test.ts worker/test/factory.test.ts \
  crates/yokohama-wasm/src/lib.rs crates/yokohama-wasm/Cargo.toml web/src/local/log.node.test.ts CLAUDE.md
git commit -m "feat(web): the layer's log seam on every host, and the refinish rule's layer half"
```

The body quotes Step 3's compile errors, Step 13's and Step 17's reds, and Step 20's stderr line.

---

### Task 7: The layer's small corrections, the restatements exported, and the seed table (spec §2.6, §2.2, R-6.5c2.7; review H3, R2-m2, m2; Review Focus 5)

**Measured:** D3 wrote the corrections and exports without compiling; the review applied them in its chain (`0007`)
and MEASURED: it compiles, and with ONE fix — the exported `error_value` had dropped Task 4's `foreign_bot` arm (P-3:
one lib cell and six `wire_fixtures` cells red) — the lib is +10, the corpus untouched and the worker ±0 (`0008` is
that fix). The seed table is D5's, moved here from the retired client task so cycle B renders nothing hand-authored;
D5 measured its Rust in a scratch crate and its 26 verdicts with a scratch program, and its `String.prototype.trim`
mutant disagrees on exactly two rows.

**Files:**
- Create: `crates/yokohama-web/src/platform.rs`, holding the eight patterns and the hosts' scan.
- Modify, in `crates/yokohama-web/src/`:
  - `lib.rs`: the module list (`:57-72`) gains `pub mod platform;`; `IntoResponse for ApiError` (`:202-211`) builds
    through `api::error_value`; the private `MAX_WAIT_SECONDS` (`:541-548`) goes, and `events` (`:578-603`) calls
    `api::check_wait`; `house_stream`'s frame (`:720`) becomes `signal::house_frame`; the axum-parity cell
    (`:1031-1079`) gains its fourth arm; the record route's pin (`:1194`) changes; the platform scan (`:6967-6980`)
    reads `crate::platform::CALLS`.
  - `api.rs`: `error_response` (`:78-86`) with the new `error_value`/`error_body`; `parse_json` (`:92-120`) and the
    new `NOT_AN_OBJECT`; `MAX_WAIT_SECONDS`/`check_wait` after `EventsQuery`; the record route's sentence (`:1844`);
    cells in the test module (`:2116+`).
  - `client.rs`: `without_version_drift` after `ClientState::of`, applied at `:498`, and a cell (test module `:709+`).
  - `signal.rs`: `house_frame` after `impl Signal for NoSignal` (which closes at `:62`) and before the first `server`
    `use` (`:64`), and a cell (`:171+`).
  - `games.rs`: `HostClock` after `pub type Clock` (`:120`), and two cells.
  - `tables.rs`: the roster sentence (`:1155`) and its pin (`:5021`).
- Modify: `crates/yokohama-web/tests/browser_played_pin.rs`, one cell.
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs` — `corpus()` pushes `seed_cases`; `SEED_INPUTS`, `seed_cases()`
  and its cell below `site_space_words()` (`:2279-2297`).
- Create (generated, committed): `web/src/wire/seed_cases.json`.
- Modify: `web/src/wire/contract.ts` — the import beside `siteSpaceWords`, a `Widen` check, the `void` list, and
  `wire.seedCases`; `web/src/local/corpus.node.test.ts` and `worker/test/corpus.test.ts` — `seed_cases` in the third
  columns and the doc bullets.
- Modify, in `crates/yokohama-worker/src/` (forced by this task's own gates): `create.rs` answers a non-object in the
  layer's words (the sentence list `:20-23`, the doc bullet `:36-40`, the second look `:62-69`); `lib.rs:949-974`:
  `an_array_body_the_layer_would_accept_is_refused` becomes `an_array_body_is_the_layers_own_400`; `house.rs:591-613`:
  the frame cell compares against the export, and its source-text read retires.

**Interfaces:**
- Consumes: the `ClientState::of` field initializer at `client.rs:498`, as Task 4 left it (this WRAPS whatever
  expression is there); `Games::with_parts` with Task 5's argument; Task 6's `test_support` (unchanged here); Task 4's
  `holding_foreign` and `BOT_TO_MOVE_AT` in `api.rs`'s test module.
- Produces: `client::without_version_drift(Vec<String>) -> Vec<String>`; `api::NOT_AN_OBJECT`;
  `api::error_body(&ApiError) -> String` and `api::error_value(&ApiError) -> serde_json::Value` (all three arms);
  `api::MAX_WAIT_SECONDS` and `api::check_wait(Option<u64>) -> Result<(), ApiError>`; `signal::house_frame(u64, u64) ->
  String`; `games::HostClock`; `platform::{CALLS, calls_in}`; `web/src/wire/seed_cases.json` and `wire.seedCases`.
- **Cycle C** switches the module and the browser host to the five exports; **cycle B**'s `seedRule.ts` is pinned to
  `seed_cases.json`. **Cycle C's bump** keeps the drift cell green (Step 2 says why).

- [ ] **Step 1: The drift filter's cells.**
  - First, in `client.rs`'s test module:

```rust
    /// **Only the engine-version drift line is dropped** (M6.5c2, spec §2.6,
    /// R-6.5c2.19). The match is a PREFIX on the CLI's own `format!`
    /// (`Session::from_record`), because `load_warnings` is a `pub Vec<String>`
    /// and a typed alternative would be a CLI change. So this cell holds the
    /// filter's PRECISION — every other warning stays, and a line that merely
    /// quotes the field is kept — while
    /// `browser_played_pin.rs`'s `a_record_from_another_release_opens_with_no_warning_and_keeps_its_version`
    /// holds that it matches the CLI at all.
    #[test]
    fn only_the_engine_version_drift_line_is_dropped() {
        let drift = "warning: GameRecord.engine_version=\"0.5.0\" does not match running engine version \
                     \"0.6.0\" — replaying anyway (v1 does not refuse on a version mismatch)"
            .to_string();
        let rules = "warning: GameRecord.rules_hash=\"x\" does not match running rules content hash \"y\" — \
                     replaying anyway (v1 does not refuse on a rules_hash mismatch)"
            .to_string();
        let seat = "warning: seat 1 was played by controller \"z\", which this build does not know how to run"
            .to_string();
        let quoted = format!("note: {drift}");
        assert_eq!(
            without_version_drift(vec![drift.clone(), rules.clone(), seat.clone(), quoted.clone()]),
            vec![rules, seat, quoted],
        );
        assert_eq!(without_version_drift(vec![drift]), Vec::<String>::new());
        assert_eq!(without_version_drift(Vec::new()), Vec::<String>::new());
    }
```

  - Then in `crates/yokohama-web/tests/browser_played_pin.rs`, after the first cell:

```rust
/// **The game screen never draws the CLI's version-drift line** (M6.5c2, spec
/// §2.6, R-6.5c2.19; review H3, R2-m2), and the save keeps the version it was
/// written at.
///
/// **An EMPTY array, never "no drift line".** The filter can only match the
/// CLI's own wording, so a cell that looked for the filter's prefix would pass
/// silently the day that wording moved; this one fails loudly. The premise
/// (`load_warnings.len() == 1`) is what makes the empty array a claim about the
/// filter: the CLI's session said exactly one thing about this record — its
/// release — and the screen shows nothing.
///
/// **It stays green across cycle C's bump to 0.6.0, by construction.**
/// - The rewritten version is DERIVED from the running one and differs from it
///   in every build, so the drift line fires at 0.5.0 and at 0.6.0 alike.
/// - The pair's other two provenance facts do not move with a release: its
///   `rules_hash` is a hash of the rules data, not the version, and its bot is
///   the production descriptor, which a bump does not re-point. So the premise
///   holds at 0.6.0 too.
/// - Its committed `engine_version` is never read here.
#[test]
fn a_record_from_another_release_opens_with_no_warning_and_keeps_its_version() {
    use yokohama_web::store::Store;
    let (games, id) = host_with_the_pair();
    let other = format!("{}-another-release", env!("CARGO_PKG_VERSION"));
    let store: Arc<dyn Store> = Arc::clone(games.store());
    let (json, version) = store.read(Doc::Record, &id).expect("a readable store").expect("the pair's record");
    let mut record: yokohama_core::GameRecord = serde_json::from_str(&json).expect("the committed record parses");
    record.engine_version = other.clone();
    let rewritten = serde_json::to_string_pretty(&record).expect("a record serializes");
    store.write_if_version(Doc::Record, &id, &rewritten, Some(version)).expect("the rewrite lands");

    let handle = games.handle(&id).expect("the finished game opens");
    let load = handle.lock().expect("a game").session.load_warnings.clone();
    assert_eq!(load.len(), 1, "premise: the session warns once, about the release: {load:?}");

    let user = User::parse(LOCAL_USER).expect("the local address parses");
    let get = |path: String| {
        dispatch(
            &games,
            &HOST,
            Ok(&user),
            &ApiRequest { method: "GET", path: &path, query: "", content_type: None, body: b"" },
        )
    };
    let res = get(format!("/api/games/{id}"));
    assert_eq!(res.status, 200, "{}", res.body);
    let state: serde_json::Value = serde_json::from_str(&res.body).expect("a ClientState");
    assert_eq!(state["warnings"], serde_json::json!([]), "the game screen carries no warning at all");

    let res = get(format!("/api/games/{id}/record"));
    assert_eq!(res.status, 200, "{}", res.body);
    let saved: yokohama_core::GameRecord = serde_json::from_str(&res.body).expect("the stored record");
    assert_eq!(saved.engine_version, other, "the save names the release that wrote it");
}
```

```bash
cargo test -p yokohama-web --lib only_the_engine_version_drift_line
cargo test -p yokohama-web --test browser_played_pin a_record_from_another_release
```

Expected: the first: COMPILE failure, `cannot find function without_version_drift`; the second: **FAILS**, with
`warnings` holding the CLI's drift sentence. Quote both.

- [ ] **Step 2: The filter.** In `client.rs`, after `impl ClientState { … }`:

```rust
/// The CLI's engine-version drift line, and nothing else, dropped from what the
/// game screen shows (M6.5c2, spec §2.6, R-6.5c2.19).
///
/// `engine_version` is provenance only (`GameRecord`'s own doc), and every save
/// carries it; after a release bump the line would sit on EVERY existing game.
/// The CLI's own frame is unchanged. **A prefix match on the CLI's `format!`**
/// (`Session::from_record`), because `load_warnings` is a `pub Vec<String>` and
/// a typed alternative would be a CLI change: the pin that keeps it honest
/// asserts an EMPTY array (`browser_played_pin.rs`), never "no drift line".
pub fn without_version_drift(warnings: Vec<String>) -> Vec<String> {
    const DRIFT: &str = "warning: GameRecord.engine_version=";
    warnings.into_iter().filter(|w| !w.starts_with(DRIFT)).collect()
}
```

  At `:498`, the `warnings:` field initializer is WRAPPED — Task 4 left it
  `without_foreign_conversion(session.load_warnings.clone(), session.record())`, so it becomes
  `warnings: without_version_drift(without_foreign_conversion(session.load_warnings.clone(), session.record())),`.
  Never replace the inner call.

```bash
cargo test -p yokohama-web --lib only_the_engine_version_drift_line
cargo test -p yokohama-web --test browser_played_pin
```

- [ ] **Step 3: The positional array's cells.** In `api.rs`'s test module:

```rust
    /// **A body that is not a JSON object is refused in the layer's own words**
    /// (M6.5c2, spec §2.6) — and ONLY where serde would have read it.
    /// - **The refused.** Every body type the route table reads, each as a
    ///   positional array serde accepts today (asserted, so the refusal is
    ///   never read off a body serde would refuse anyway), answers
    ///   [`NOT_AN_OBJECT`].
    /// - **Axum's words, kept (R-6.5a.4).** A malformed array and a short one
    ///   keep axum's own sentences, because the object rule runs after serde
    ///   has answered.
    #[test]
    fn a_positional_array_body_is_refused_and_every_other_body_keeps_axums_sentence() {
        fn refused<T: DeserializeOwned>(body: &[u8]) -> ApiError {
            let text = String::from_utf8_lossy(body).to_string();
            assert!(serde_json::from_slice::<T>(body).is_ok(), "premise: serde reads {text} as the struct");
            parse_json::<T>(Some("application/json"), body).err().unwrap_or_else(|| panic!("{text} was read"))
        }
        for e in [
            refused::<crate::tables::NewGame>(br#"[2,["Red","Blue"],0,null,[{"user":"red@t.test"},{"open":true}]]"#),
            refused::<Input>(br#"["1"]"#),
            refused::<Join>(b"[1]"),
            refused::<Keep>(b"[0, 1]"),
            refused::<ChatPost>(br#"["hi"]"#),
            refused::<Unlock>(br#"["sesame"]"#),
            refused::<ArchivePost>(br#"["{}", "{}"]"#),
            refused::<Input>(b"\n  [\"1\"]\n"),
        ] {
            assert_eq!((e.status, e.message.as_str()), (StatusCode::BAD_REQUEST, NOT_AN_OBJECT));
        }
        let ct = Some("application/json");
        let e = parse_json::<Input>(ct, br#"["1""#).err().expect("a malformed array");
        assert!(e.message.starts_with("Failed to parse the request body as JSON: "), "{}", e.message);
        let e = parse_json::<Keep>(ct, b"[0]").err().expect("a short array");
        assert!(e.message.starts_with("Failed to deserialize the JSON body into the target type: "), "{}", e.message);
        let read = parse_json::<Input>(ct, b" {\"line\":\"1\"} \n").ok().map(|i| i.line);
        assert_eq!(read.as_deref(), Some("1"), "an object, JSON whitespace around it, reads");
    }

    /// Through the front door: a positional create opens no table.
    #[test]
    fn a_positional_create_opens_no_table() {
        let games = crate::games::test_support::games();
        let red = User::parse("red@t.test").expect("an address");
        let body = br#"[2,["Red","Blue"],0,null,[{"user":"red@t.test"},{"open":true}]]"#;
        let res = dispatch(
            &games,
            &LOCAL_HOST,
            Ok(&red),
            &ApiRequest { method: "POST", path: "/api/games", query: "", content_type: Some("application/json"), body },
        );
        assert_eq!((res.status, res.body), (400, serde_json::json!({ "error": NOT_AN_OBJECT }).to_string()));
        assert!(games.store().list(crate::store::Doc::Table).expect("a listing").is_empty(), "no table opened");
    }
```

  In `lib.rs`'s `the_layers_json_rejection_is_axums_word_for_word`, after its third arm (`:1077-1078`):

```rust
        // **And the one DEPARTURE, named** (M6.5c2, spec §2.6): a positional
        // ARRAY that serde reads a struct from is ACCEPTED by axum and refused
        // by the layer in its own words — the only body whose answer is not
        // axum's, and only because axum's was a table opened from a list.
        let array = r#"[2,["Red","Blue"],0,null,[{"user":"red@t.test"},{"open":true}]]"#;
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/x")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(array))
            .unwrap();
        assert!(
            axum::Json::<tables::NewGame>::from_request(req, &()).await.is_ok(),
            "premise: axum reads the array as a create",
        );
        let ours = api::parse_json::<tables::NewGame>(Some("application/json"), array.as_bytes())
            .err()
            .expect("the layer refuses it");
        assert_eq!(ours.message, api::NOT_AN_OBJECT);
```

  In `crates/yokohama-worker/src/lib.rs`, **replace** `an_array_body_the_layer_would_accept_is_refused` (`:949-974`).
  Its premise, the layer's 201, is what this task removes.

```rust
    /// A positional ARRAY body is the LAYER's own 400 since M6.5c2 (spec §2.6):
    /// `api::parse_json` reads an object only, so this module forwards the body
    /// as it forwards every body the layer refuses, and the answer is the
    /// layer's, byte for byte — one sentence on every host.
    #[test]
    fn an_array_body_is_the_layers_own_400() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        // Positional, in `NewGame`'s FIELD ORDER — the body the M6.5c plan's
        // review measured at 201, before the layer refused it.
        let body = r#"[2,["Red","Blue"],0,null,[{"user":"red@t.test"},{"open":true}]]"#;
        let direct = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", Some("application/json"), body.as_bytes());
        init_inner(&config("off")).unwrap();
        let r = create_inner(Some("red@t.test"), body.as_bytes(), "sim-00000000000000ff", Some("application/json"));
        assert_eq!((r.status, &r.body), (direct.status, &direct.body), "the layer's own answer, byte for byte");
        assert_eq!((r.status, r.body.as_str()), (400, serde_json::json!({ "error": api::NOT_AN_OBJECT }).to_string().as_str()));
        assert!(stored(Doc::Table).is_empty() && stored(Doc::Record).is_empty(), "nothing stored");
        // Anti-vacuity: the same listing names a table the moment one IS stored.
        let r = create_inner(Some("red@t.test"), CREATE.as_bytes(), "sim-00000000000000ff", Some("application/json"));
        assert_eq!(r.status, 201, "{}", r.body);
        assert_eq!(stored(Doc::Table), ["sim-00000000000000ff"]);
    }
```

```bash
cargo test -p yokohama-web --lib positional
cargo test -p yokohama-worker an_array_body_is_the_layers_own_400
```

Expected: COMPILE failures, `cannot find value NOT_AN_OBJECT`.

- [ ] **Step 4: The object form only.**
  1. **`NOT_AN_OBJECT`.** In `api.rs`, above `parse_json`:

```rust
/// The one sentence a body that is not a JSON OBJECT is refused in (M6.5c2,
/// spec §2.6). Every body this API reads is an object — each `T` is a struct
/// under `deny_unknown_fields` — but serde's derived `Deserialize` ALSO reads a
/// struct from a positional ARRAY, a form `deny_unknown_fields` does not
/// govern: `[2,["Red","Blue"],0,null,[…]]` opened a table. `pub` so a host that
/// forwards a create — the Worker module — refuses in these words, not its own.
pub const NOT_AN_OBJECT: &str = "the request body must be a JSON object";
```

  2. **`parse_json`.** Its doc gains the sentence "And one rule of the layer's own, last: an object only (M6.5c2) —
     the one place its answer departs from axum's, and only where axum's is a table opened from a list." Its tail,
     after `de.end()…?;`, becomes:

```rust
    // **The object form only** (M6.5c2, spec §2.6), and checked LAST, after
    // serde has read the body, so every body axum refuses keeps axum's sentence
    // word for word (R-6.5a.4): this refuses only what axum ACCEPTS — a
    // positional array serde read a struct from. A body that parsed whole, with
    // nothing after it, is one JSON value, and its first byte past JSON's own
    // whitespace says which kind.
    if body.iter().find(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r')) != Some(&b'{') {
        return Err(ApiError::bad_request(NOT_AN_OBJECT));
    }
    Ok(value)
```

  3. **The module's `create.rs`.** Delete its host-written `NOT_AN_OBJECT` (`:21`); the list's comment (`:20`) reads
     "The two sentences this route writes itself — spec §4's host-written list, less the one the layer took over in
     M6.5c2". Its second look, the comment and the `let … else` (`:62-69`), becomes:

```rust
    // The second look, which only DECIDES: nothing it reads reaches the layer.
    // On an object it says whether an `id` key is there at all, which `game.id`
    // cannot: `"id": null` reads as `None`. **It meets nothing but an object
    // since M6.5c2** (spec §2.6): the layer's `parse_json` refuses any other
    // body before this line, and this module forwards that body like every
    // other the layer refuses. The arm stays as the invariant's guard, in the
    // layer's own words.
    let Ok(object) = serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(body) else {
        return host_refusal(ApiError::bad_request(api::NOT_AN_OBJECT));
    };
```

     Its fourth doc bullet (`:36-40`) becomes:

     > **A body that is not a JSON OBJECT is the layer's 400 since M6.5c2** (spec §2.6). serde's derived
     > `Deserialize` reads a struct from a positional ARRAY too, and the layer used to answer such a body 201, a
     > quirk of the NAS as well. The layer's `parse_json` now refuses it in its own words (`api::NOT_AN_OBJECT`), so
     > it reaches the layer unchanged, as every body the layer refuses does. The Worker names an object from an
     > object's `id` alone, and that holds by the layer's rule.

```bash
cargo test -p yokohama-web --lib positional
cargo test -p yokohama-web --lib the_layers_json_rejection_is_axums_word_for_word
cargo test -p yokohama-worker an_array_body_is_the_layers_own_400
```

- [ ] **Step 5: The five exports' cells.**
  1. **In `api.rs`'s test module** — `error_body`'s cell carries THREE shapes (P-3: the export's first draft dropped the
     `foreign_bot` arm, and only the corpus caught it):

```rust
    /// **`error_body` is the body `dispatch` writes for a refusal** (M6.5c2,
    /// spec §2.6's restatements) — all THREE shapes: bare (an unknown
    /// endpoint's 404), with `"status"` (a dealing table's 409), and with
    /// `"foreign_bot"` (a game not over against a bot this build does not run).
    /// The Worker module restates it today; once it imports this, a drift is
    /// impossible — and the third shape is the one a restatement dropped when
    /// this export was first written.
    #[test]
    fn error_body_is_the_body_dispatch_writes() {
        use crate::games::test_support::{games, two_people, BOT_TO_MOVE_AT, CORPUS_SEED};
        let games = games();
        let red = User::parse("red@t.test").expect("an address");
        let get = |games: &Games, path: &str| {
            dispatch(games, &LOCAL_HOST, Ok(&red), &ApiRequest { method: "GET", path, query: "", content_type: None, body: b"" })
        };
        let res = get(&games, "/api/nope");
        assert_eq!((res.status, res.body), (404, error_body(&ApiError::not_found("no such endpoint: /api/nope"))));
        games.create(&two_people("dealing", CORPUS_SEED), &red).expect("a dealing table");
        let res = get(&games, "/api/games/dealing");
        assert_eq!((res.status, res.body.clone()), (409, error_body(&ApiError::dealing())));
        assert!(res.body.contains(r#""status":"dealing""#), "anti-vacuity: the second shape: {}", res.body);
        // The third shape, through the same door Task 4's cells use.
        let foreign = holding_foreign("lookahead-v1", Some(BOT_TO_MOVE_AT));
        let local = User::parse("local@browser").expect("an address");
        let res = dispatch(
            &foreign,
            &LOCAL_HOST,
            Ok(&local),
            &ApiRequest { method: "GET", path: &format!("/api/games/{PLAYED_ID}"), query: "", content_type: None, body: b"" },
        );
        let e: ApiError = crate::games::GameError::ForeignBot {
            played: "V2 lookahead".to_string(),
            runs: crate::bots::label(&crate::bots::production()),
        }
        .into();
        assert_eq!((res.status, res.body.clone()), (409, error_body(&e)));
        assert!(res.body.contains(r#""foreign_bot":{"played":"V2 lookahead""#), "anti-vacuity: the third shape: {}", res.body);
    }

    /// **The wait cap is the layer's, exported** (M6.5c2): 25 holds, 26 is a
    /// 400 in the handler's own words — a refusal, never a clamp — and no
    /// `wait` holds nothing. The axum handler calls this;
    /// `events_can_be_held_until_a_bump_and_the_wait_is_bounded` (`lib.rs`)
    /// pins it there end to end.
    #[test]
    fn a_wait_above_the_cap_is_refused_never_clamped() {
        assert_eq!(MAX_WAIT_SECONDS, 25);
        assert!(check_wait(None).is_ok() && check_wait(Some(0)).is_ok() && check_wait(Some(25)).is_ok());
        let e = check_wait(Some(26)).err().expect("26 is over the cap");
        assert_eq!(
            (e.status, e.message.as_str()),
            (StatusCode::BAD_REQUEST, "a request can be held for at most 25 seconds, not 26"),
        );
        assert!(check_wait(Some(u64::MAX)).is_err());
    }
```

  2. **In `signal.rs`'s test module:**

```rust
    /// **The house frame's data, exported** (M6.5c2, spec §2.6's restatements):
    /// `{"seq":…,"chat":…}`, in that order, whole `u64`s — `house_stream`'s
    /// frame, and the Worker module's lobby-socket frame once it imports this.
    #[test]
    fn the_house_frame_is_seq_then_chat() {
        assert_eq!(house_frame(3, 7), r#"{"seq":3,"chat":7}"#);
        assert_eq!(house_frame(0, 0), r#"{"seq":0,"chat":0}"#);
        assert_eq!(house_frame(u64::MAX, 1), format!("{{\"seq\":{},\"chat\":1}}", u64::MAX));
    }
```

  3. **In `games.rs`'s test module:**

```rust
    /// **A host's clock, stated once** (M6.5c2, spec §2.6's restatements): what
    /// `crates/yokohama-wasm` and `crates/yokohama-worker` each carried as three
    /// statics and a `set_clock`. Pinned, it answers the pin and asks the
    /// platform nothing; `set` hands back what it replaced (`None` = live), so a
    /// caller that pins one call can restore a frozen test clock after it; and
    /// `tick` is the live reading of a build with no platform clock.
    #[test]
    fn a_host_clock_pins_answers_what_it_replaced_and_is_live_otherwise() {
        let clock = HostClock::new();
        assert_eq!(clock.now(|| 42), 42, "unpinned: the host's live reading");
        assert_eq!(clock.set(Some(1_789_000_000)), None, "it replaced the live clock");
        assert_eq!(clock.now(|| panic!("a pinned clock asks the platform nothing")), 1_789_000_000);
        assert_eq!(clock.set(Some(1_789_000_500)), Some(1_789_000_000));
        assert_eq!(clock.set(None), Some(1_789_000_500));
        assert_eq!(clock.now(|| 7), 7);
        assert_eq!((clock.tick(), clock.tick(), clock.tick()), (1, 2, 3));
    }
```

  4. **The new file `crates/yokohama-web/src/platform.rs`**, its test module only for now. Add `pub mod platform;`
     after `pub mod outcome;`.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// **The eight, stated once** (M6.5c2): the list every scan in this
    /// workspace refuses, in M6.5a's order, spelled whole HERE, where the
    /// layer's own scan does not read.
    #[test]
    fn the_eight_platform_calls() {
        assert_eq!(
            CALLS,
            ["SystemTime", "Instant::now", "std::fs", "std::env", "std::thread", "std::process", "tokio::", "axum::"],
        );
    }

    /// **A host's scan over one file** (M6.5c2): every line above the first
    /// column-0 `#[cfg(test)]` that names a call — prose included, since a
    /// host's sources name none — as `file:line: pattern: code`, and nothing
    /// below it. Two hits on one line are two entries.
    #[test]
    fn a_hosts_scan_names_each_call_above_its_test_module_and_nothing_below() {
        let text = "use std::sync::Arc;\n\
                    fn a() { let _ = std::env::var(\"X\"); }\n\
                    // std::fs, named in prose\n\
                    fn b() { let _ = (std::thread::current(), std::process::id()); }\n\
                    #[cfg(test)]\n\
                    mod tests { fn c() { let _ = std::fs::read(\"x\"); } }\n";
        assert_eq!(
            calls_in("host.rs", text),
            [
                "host.rs:2: std::env: fn a() { let _ = std::env::var(\"X\"); }",
                "host.rs:3: std::fs: // std::fs, named in prose",
                "host.rs:4: std::thread: fn b() { let _ = (std::thread::current(), std::process::id()); }",
                "host.rs:4: std::process: fn b() { let _ = (std::thread::current(), std::process::id()); }",
            ],
        );
        assert!(calls_in("clean.rs", "fn main() {}\n").is_empty());
    }
}
```

```bash
cargo test -p yokohama-web --lib error_body_is_the_body
cargo test -p yokohama-web --lib a_wait_above_the_cap
cargo test -p yokohama-web --lib the_house_frame_is_seq_then_chat
cargo test -p yokohama-web --lib a_host_clock
cargo test -p yokohama-web --lib platform::
```

Expected: COMPILE failures naming `error_body`, `check_wait`, `MAX_WAIT_SECONDS`, `house_frame`, `HostClock`, `CALLS`
and `calls_in`.

- [ ] **Step 6: The five exports.**
  1. **`api.rs`, `error_body` — with ALL THREE arms** (P-3). `error_response` (`:78-86`), as Task 4 left it, becomes:

```rust
/// The body of every refusal on this API, byte for byte — `{"error": …}`, with
/// `"status"` beside it on a refusal a client branches on at a table's status,
/// and `"foreign_bot"` beside it on a game played against a bot this build does
/// not run (M6.5c2). `pub` since M6.5c2 (spec §2.6's restatements): a host that
/// answers a refusal ITSELF — the Worker module's `error_body` — writes the
/// layer's bytes rather than a copy of the rule.
pub fn error_body(e: &ApiError) -> String {
    error_value(e).to_string()
}

/// [`error_body`] as a value, for `IntoResponse for ApiError`, whose `Json`
/// serializes it — the same bytes, one rule, THREE arms. (The first draft of
/// this export carried two; the corpus's foreign-bot 409 is what caught it.)
pub(crate) fn error_value(e: &ApiError) -> serde_json::Value {
    let mut body = serde_json::json!({ "error": e.message });
    if let Some(status) = e.table_status {
        body["status"] = status.into();
    }
    if let Some(pair) = &e.foreign_bot {
        body["foreign_bot"] = serde_json::to_value(pair).expect("the pair serializes");
    }
    body
}

/// The body `IntoResponse for ApiError` writes, byte for byte.
pub(crate) fn error_response(e: ApiError) -> ApiResponse {
    ApiResponse { status: e.status.as_u16(), body: error_body(&e) }
}
```

     And `lib.rs`'s `IntoResponse for ApiError` (as Task 4 left it, `:203-211`) becomes:

```rust
#[cfg(feature = "server")]
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(api::error_value(&self))).into_response()
    }
}
```

  2. **`api.rs`, the wait cap.** After `EventsQuery`'s definition:

```rust
/// The longest a `?wait` may hold (spec §3.3).
///
/// Above it is a **400 and not a clamp**: a caller that asked for 120 and was
/// silently given 25 would read the early answer as "nothing changed for two
/// minutes", which is the same failure `deny_unknown_fields` exists to prevent
/// one field over. `pub` since M6.5c2 (spec §2.6's restatements): the Worker
/// module's `events` route is its own and refuses with the layer's cap and
/// sentence through [`check_wait`] rather than a copy.
pub const MAX_WAIT_SECONDS: u64 = 25;

/// `?wait` above [`MAX_WAIT_SECONDS`] refused in the handler's own words; no
/// `wait`, or one at or below the cap, answers `Ok`.
pub fn check_wait(wait: Option<u64>) -> Result<(), ApiError> {
    match wait {
        Some(w) if w > MAX_WAIT_SECONDS => Err(ApiError::bad_request(format!(
            "a request can be held for at most {MAX_WAIT_SECONDS} seconds, not {w}"
        ))),
        _ => Ok(()),
    }
}
```

     In `lib.rs`: delete the private `MAX_WAIT_SECONDS` and its doc (`:541-548`); in `events`, delete the inner `if
     wait > MAX_WAIT_SECONDS { return Err(…); }` (`:586-590`), and make `api::check_wait(q.wait)?;` the handler's
     first statement, before `if let Some(wait) = q.wait {`. The order is unchanged (the cap before the admission),
     and the arm's remaining lines stay as they are:

```rust
) -> ApiResult<Events> {
    api::check_wait(q.wait)?;
    if let Some(wait) = q.wait {
        // **Admitted BEFORE it subscribes**, … (the existing comment, unchanged)
        let _admitted = api::events_at(&state.games, &id, &user)?;
        let mut rx = state.signals.subscribe(&id);
        // Either arm is an answer: … (the existing comment, unchanged)
        let _ = tokio::time::timeout(std::time::Duration::from_secs(wait), rx.changed()).await;
    }
    Ok(Json(api::events_at(&state.games, &id, &user)?))
}
```

  3. **`signal.rs`, the frame.** After `impl Signal for NoSignal { … }` and above the `#[cfg(feature = "server")]
     use` lines:

```rust
/// The house frame's DATA: the two counters as `{"seq":…,"chat":…}`, `seq`
/// first (M6.4, R-6.4.7). `pub` and outside `server` since M6.5c2 (spec §2.6's
/// restatements): `house_stream` writes it as each `house` event's data, and the
/// Worker module's lobby socket writes the same bytes by calling this, not a
/// copy pinned to this crate's source text.
pub fn house_frame(seq: u64, chat: u64) -> String {
    format!("{{\"seq\":{seq},\"chat\":{chat}}}")
}
```

     `lib.rs:720` becomes `.data(signal::house_frame(h.seq, h.chat)),`.
  4. **`games.rs`, the clock.** After `pub type Clock` (`:120`):

```rust
/// A HOST's clock, frozen or live (M6.5c2, spec §2.6's restatements): what
/// `crates/yokohama-wasm` and `crates/yokohama-worker` each carried as three
/// statics and a `set_clock`, stated once.
///
/// **It makes no platform call.** The host hands its live reading to
/// [`HostClock::now`]: `Date.now()` on the target, and [`HostClock::tick`]
/// natively, where the crate's own cells and the lint gate compile and a
/// `js_sys` call is a stub that panics. The ticking shape is
/// [`Games::ticking_clock`]'s, for its reason: a constant would make every
/// table's `created_at` a tie. `const`-constructed, so a host holds one in a
/// `static`.
pub struct HostClock {
    pinned: std::sync::atomic::AtomicBool,
    reading: std::sync::atomic::AtomicU64,
    ticks: std::sync::atomic::AtomicU64,
}

impl HostClock {
    pub const fn new() -> Self {
        Self {
            pinned: std::sync::atomic::AtomicBool::new(false),
            reading: std::sync::atomic::AtomicU64::new(0),
            ticks: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Pin the clock to `seconds`, or pass `None` for the live reading —
    /// answering the setting it replaced (`None` = live).
    pub fn set(&self, seconds: Option<u64>) -> Option<u64> {
        use std::sync::atomic::Ordering::Relaxed;
        let prior = self.pinned.load(Relaxed).then(|| self.reading.load(Relaxed));
        match seconds {
            Some(s) => {
                self.reading.store(s, Relaxed);
                self.pinned.store(true, Relaxed);
            }
            None => self.pinned.store(false, Relaxed),
        }
        prior
    }

    /// The pinned reading, else `live()` — which a pinned clock never calls.
    pub fn now(&self, live: impl FnOnce() -> u64) -> u64 {
        use std::sync::atomic::Ordering::Relaxed;
        if self.pinned.load(Relaxed) {
            self.reading.load(Relaxed)
        } else {
            live()
        }
    }

    /// 1, 2, 3, … — the live reading of a build with no platform clock.
    pub fn tick(&self) -> u64 {
        self.ticks.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    }
}

impl Default for HostClock {
    fn default() -> Self {
        Self::new()
    }
}
```

  5. **`platform.rs`, the scan.** Above its test module:

```rust
//! **The platform calls every scan in this workspace refuses** (M6.5c2, spec
//! §2.6's restatements).
//!
//! Three scans read source for them:
//! - this crate's own `platform_calls_stay_behind_the_server_feature`
//!   (`lib.rs`), which ENFORCES a `server` gate on each item that makes one;
//! - each host crate's — `crates/yokohama-wasm`'s and
//!   `crates/yokohama-worker`'s — which refuses them outright above its test
//!   module.
//!
//! Each carried its own copy of the eight. The list is stated HERE once, and
//! [`calls_in`] is the hosts' scan over one file's text.
//!
//! **Each pattern is written in two halves** (`concat!`), and that is not
//! decoration: the layer's own scan reads THIS file, and a list spelled whole
//! would be eight escaped calls to it. The test module below spells them
//! whole, where no scan reads.

/// The eight, in M6.5a's order.
pub const CALLS: [&str; 8] = [
    concat!("System", "Time"),
    concat!("Instant", "::now"),
    concat!("std", "::fs"),
    concat!("std", "::env"),
    concat!("std", "::thread"),
    concat!("std", "::process"),
    concat!("tokio", "::"),
    concat!("axum", "::"),
];

/// A HOST crate's scan of one source file (M6.5c2): every line above the
/// file's first column-0 `#[cfg(test)]` that names one of [`CALLS`] — prose
/// included, since a host's sources name none — as `file:line: pattern: code`.
///
/// It reads TEXT and no file: the host reads its own `src/`, so this builds
/// without `server` and makes no platform call itself. The host keeps its own
/// anti-vacuity partners (the calls it ALLOWS are its own).
pub fn calls_in(file: &str, text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let end = lines.iter().position(|l| *l == "#[cfg(test)]").unwrap_or(lines.len());
    let mut found = Vec::new();
    for (n, line) in lines[..end].iter().enumerate() {
        for pattern in CALLS {
            if line.contains(pattern) {
                found.push(format!("{file}:{}: {pattern}: {}", n + 1, line.trim()));
            }
        }
    }
    found
}
```

     And `lib.rs`'s `platform_calls_stay_behind_the_server_feature` (`:6970-6979`) replaces its `let patterns = [ …
     eight literals … ];` with `let patterns = crate::platform::CALLS;`.
  6. **The module cell the frame export forces** (cycle C keeps the switch of `house_frame_inner`'s body). In
     `crates/yokohama-worker/src/house.rs`, `house_frame_is_the_sse_frames_data_byte_for_byte` (`:591-613`) becomes:

```rust
    /// **`house_frame` is the NAS's lobby frame, byte for byte** (spec §2.5):
    /// the `data` of the `house` event `house_stream` writes on `/api/stream`,
    /// which since M6.5c2 is the layer's own `signal::house_frame` (spec §2.6's
    /// restatements). So this compares against the export, and the pin on the
    /// layer's source TEXT retires with its reason. The export takes `u32`
    /// (the orchestrator's ruling I-22) and widens it without loss.
    #[test]
    fn house_frame_is_the_sse_frames_data_byte_for_byte() {
        assert_eq!(house_frame_inner(3, 7), r#"{"seq":3,"chat":7}"#);
        for (seq, chat) in [(0, 0), (3, 7), (7, 3), (u64::from(u32::MAX) + 1, 1), (u64::MAX, u64::MAX)] {
            assert_eq!(house_frame_inner(seq, chat), yokohama_web::signal::house_frame(seq, chat), "({seq}, {chat})");
        }
        assert_eq!(crate::house_frame(3, 7), house_frame_inner(3, 7));
        assert_eq!(
            crate::house_frame(u32::MAX, u32::MAX),
            yokohama_web::signal::house_frame(u64::from(u32::MAX), u64::from(u32::MAX)),
        );
    }
```

     (`Path` stays in use at `house.rs:294`, so no import goes unused.)

```bash
cargo test -p yokohama-web --lib error_body_is_the_body
cargo test -p yokohama-web --lib a_wait_above_the_cap
cargo test -p yokohama-web --lib the_house_frame_is_seq_then_chat
cargo test -p yokohama-web --lib a_host_clock
cargo test -p yokohama-web --lib platform::
cargo test -p yokohama-web --lib platform_calls_stay_behind_the_server_feature
cargo test -p yokohama-web --lib events_can_be_held_until_a_bump_and_the_wait_is_bounded
cargo test -p yokohama-web --lib the_house_frame_carries_seq_and_chat_independently
cargo test -p yokohama-web --lib every_game_route_refuses_an_unfinished_foreign_game
cargo test -p yokohama-worker house_frame_is_the_sse_frames_data_byte_for_byte
```

  (Task 4's `every_game_route_refuses…` is the second witness that the third arm is there: the review measured it
  red, with six `wire_fixtures` cells, when the export dropped the arm.)

- [ ] **Step 7: The two "watch" sentences, reworded to the spectate family** (spec §2.2; the third, `check_shape`'s
  watcher-list sentence, was Task 1's).
  1. **The pins first.** `lib.rs:1194` becomes `assert_eq!(body["error"], "the record is the players' — a spectator
     downloads nothing");`; in `tables.rs`'s `the_two_hundred_and_fifty_seventh_watcher_is_refused`, the pin at
     `:5021` becomes `if m == "this table has as many spectators as it can remember"),`.

```bash
cargo test -p yokohama-web --lib the_record_route_is_409_while_running_403_for_a_watcher
cargo test -p yokohama-web --lib the_two_hundred_and_fifty_seventh_watcher_is_refused
```

     Expected: both FAIL on the sentence. (The first cell, `lib.rs:1184`, holds the pin at `:1194`.)
  2. **Then the words.** `api.rs:1844` becomes `message: "the record is the players' — a spectator downloads
     nothing".to_string(),`; `tables.rs:1155` becomes `"this table has as many spectators as it can
     remember".to_string(),`.
  3. **Neither sentence is in the corpus, the client or the Worker.** `git grep -n "watch, download nothing\|as many
     watchers as it can remember" -- . ':!docs/plans'` printed only the two sources and their two pins at the base.
     So there is no regeneration for them.
  4. Run the two cells again. They pass.

- [ ] **Step 8: `archive_over_retry`'s second `NotFound` arm** (M6.5c Task 1's M6; handover §4.6). In `games.rs`'s test
  module:

```rust
    /// **The second `NotFound`** (M6.5c Task 1's M6): a RESIDENT game whose
    /// record is gone from the store. `handle` answers from `live` and never
    /// reads the store, so the retry's own record read is what finds nothing —
    /// and it must say so, never answer `Ok` over a game it cannot archive.
    #[test]
    fn archive_over_retry_is_not_found_for_a_resident_game_whose_record_is_gone() {
        let archive = RecordingArchive::recording();
        let games = games().with_archive(archive.clone());
        let _handle = playing(&games, &two_people("vanished", CORPUS_SEED), &["red@t.test", "blue@t.test"]);
        games.store().delete("vanished").expect("the store's own delete, under a resident handle");
        assert!(games.handle("vanished").is_ok(), "premise: still RESIDENT, so `handle` reads no store");
        let err = games.archive_over_retry("vanished").expect_err("no record to archive");
        assert!(matches!(&err, GameError::NotFound(m) if m == "no game named \"vanished\""), "{err:?}");
        assert!(archive.puts.lock().unwrap().is_empty(), "nothing was put");
    }
```

  This pins EXISTING code, so its red is a mutant's: by hand, change the arm's `return
  Err(GameError::NotFound(format!("no game named {id:?}")));` (`games.rs:753` at the base) to `return Ok(());`; run
  `cargo test -p yokohama-web --lib archive_over_retry_is_not_found` and watch it FAIL on `expect_err`; restore the
  one line by hand, and run it again. It passes. Quote the mutant's red in the ledger.

- [ ] **Step 9: The seed table — its cell first (Review Focus 5).** In `crates/yokohama-web/tests/wire_fixtures.rs`,
  directly below `fn site_space_words()` (`:2279-2297`):

```rust
/// **The verdicts Review Focus 5 names, as the spec states them** (M6.5c2).
///
/// `seed_cases.json` is whatever [`WireSeed`] says TODAY, and the client is held
/// to that — so without this cell the two could agree with each other and both
/// disagree with what the spec promised the player. The last two asserts are
/// the anti-vacuity: the two inputs on which JavaScript's `String.prototype.trim`
/// and Rust's `str::trim` part ways really do get different verdicts from the
/// server, so a client rule written on the obvious trim fails cycle B's
/// `seedRule.test.ts`.
///
/// [`WireSeed`]: yokohama_web::tables::WireSeed
#[test]
fn the_seed_table_holds_the_verdicts_the_spec_names() {
    let cases = seed_cases();
    let verdict = |raw: &str| -> bool {
        cases
            .as_array()
            .expect("the table is an array")
            .iter()
            .find(|c| c["raw"] == raw)
            .unwrap_or_else(|| panic!("{raw:?} is not in the committed table"))["ok"]
            .as_bool()
            .expect("a verdict")
    };
    for raw in ["+007 ", " 7", "007", "18446744073709551615"] {
        assert!(verdict(raw), "the server takes {raw:?}, and the spec says it does");
    }
    for raw in ["\u{ff17}", "1e3", "-1", "18446744073709551616", "", " "] {
        assert!(!verdict(raw), "the server refuses {raw:?}, and the spec says it does");
    }
    assert!(verdict("\u{85}12"), "Rust's trim strips U+0085; JavaScript's does not");
    assert!(!verdict("\u{feff}12"), "JavaScript's trim strips U+FEFF; Rust's does not");
}
```

```bash
cargo test -p yokohama-web --test wire_fixtures the_seed_table
```

  Expected: a COMPILE error — `cannot find function 'seed_cases' in this scope`.

- [ ] **Step 10: The table.** Directly above the cell:

```rust
/// **What the new-table form checks a seed against, as the server reads one**
/// (M6.5c2, spec §2.2 "The seed"; Review Focus 5).
///
/// Not a payload any route serves — a CONTRACT, frozen as
/// [`site_space_words`] is and for its reason: the client has to decide
/// something itself. The form checks a typed seed AS IT IS TYPED
/// (`web/src/seedRule.ts`, cycle B), which is a second statement of
/// [`WireSeed`]'s rule — `str::trim`, then `u64::from_str`: an optional `+`,
/// ASCII digits, at most 18446744073709551615. The client cannot call Rust, so
/// the two are held together here: `seedRule.test.ts` asserts `checkSeed` gives
/// every verdict in this file, and this file is refused when the rule moves.
///
/// **The inputs sit where a hand-written check would part ways with the
/// server**: leading zeros and a `+` it takes; a full-width or an Arabic-Indic
/// digit, an exponent, a sign, a separator and one past the top it refuses;
/// and TWO that split JavaScript's own `String.prototype.trim` from Rust's —
/// U+FEFF, which JavaScript strips and Rust does not, and U+0085, which Rust
/// strips and JavaScript does not (measured on both sides when this was
/// written). None is the walk's [`SETUP_SEED`], in either spelling: the
/// leakage pin scans this file like every other.
///
/// [`WireSeed`]: yokohama_web::tables::WireSeed
const SEED_INPUTS: &[&str] = &[
    "+007 ",
    " 7",
    "007",
    "\u{ff17}",
    "1e3",
    "-1",
    "18446744073709551616",
    "18446744073709551615",
    "",
    " ",
    "0",
    "+0",
    "-0",
    "+",
    "12 34",
    "\t42\n",
    "\u{a0}12\u{a0}",
    "\u{feff}12",
    "\u{85}12",
    "\u{3000}12\u{3000}",
    "0x10",
    "1_000",
    "12.0",
    "\u{663}",
    "00000000000000000000018446744073709551615",
    "99999999999999999999",
];

/// Each input with [`WireSeed`]'s own verdict — the create's `seed` field
/// deserialized from the STRING a browser sends, exactly as `NewGame` reads it.
///
/// [`WireSeed`]: yokohama_web::tables::WireSeed
fn seed_cases() -> Value {
    Value::Array(
        SEED_INPUTS
            .iter()
            .map(|raw| {
                let ok = serde_json::from_value::<yokohama_web::tables::WireSeed>(Value::String(
                    (*raw).to_string(),
                ))
                .is_ok();
                serde_json::json!({ "raw": raw, "ok": ok })
            })
            .collect(),
    )
}
```

  And in `corpus()`, beside the other contract pushes (`site_space_words`, `identity_cases`, `production_bot`):

```rust
    all.push(("seed_cases".to_string(), seed_cases()));
```

```bash
cargo test -p yokohama-web --test wire_fixtures the_seed_table
cargo test -p yokohama-web --test wire_fixtures the_committed_wire_fixtures_are_what_the_server_emits
```

  Expected: the first passes. The second FAILS, naming exactly one drift: `seed_cases.json — No such file or directory
  (os error 2)`.

- [ ] **Step 11: Regenerate, and READ the diff.**

```bash
UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures
git status --short web/src/wire web/src/wire-script
git diff --stat -- web/src/wire web/src/wire-script
cargo test -p yokohama-web --test wire_fixtures
```

  **Expected, and nothing else:** ONE new file, `?? web/src/wire/seed_cases.json`; `git diff --stat` prints nothing
  (no committed payload or script moved — the exports and the reworded sentences reach no fixture, and `warnings`
  stays `[]` because every corpus record is at the running version); the check run reads **+1** (13). The file holds
  26 rows, each `{"ok": …, "raw": …}` (serde sorts the keys), in `SEED_INPUTS`' order:
  - `"ok": true` (11): `"+007 "`, `" 7"`, `"007"`, `"18446744073709551615"`, `"0"`, `"+0"`, `"\t42\n"`,
    `" 12 "`, `"\u008512"`, `"　12　"`, `"00000000000000000000018446744073709551615"`;
  - `"ok": false` (15): `"７"`, `"1e3"`, `"-1"`, `"18446744073709551616"`, `""`, `" "`, `"-0"`, `"+"`, `"12 34"`,
    `"﻿12"`, `"0x10"`, `"1_000"`, `"12.0"`, `"٣"`, `"99999999999999999999"`.

  (These are the verdicts `trim().parse::<u64>()` gave for the same inputs when D5 measured them with a scratch
  program; serde_json writes the tab and newline escaped and the other characters as raw UTF-8 — read with `cat -v`.)
  A different verdict or any other changed file is a FINDING: stop and report it rather than commit it.

- [ ] **Step 12: The client's typed entry, and the two replays' third columns.** `web/src/wire/contract.ts`: beside
  the `siteSpaceWords` and `identityCases` imports, `import seedCases from "./seed_cases.json";`; beside
  `_identityCases`, `const _seedCases: readonly { raw: string; ok: boolean }[] = seedCases;` and its name in the
  `void` list; and in `wire`, beside `identityCases`:

```ts
  /** Not a payload either: `WireSeed`'s verdicts over a committed list of
   *  inputs, frozen so the form's as-you-type check cannot drift from the
   *  server's rule. See that fixture's doc in `wire_fixtures.rs`. */
  seedCases: seedCases as readonly { raw: string; ok: boolean }[],
```

  `web/src/local/corpus.node.test.ts`: `"seed_cases",` INSERTED into `NOT_SERVED` (compared sorted), and the doc's
  first bullet becomes:

```ts
 * - `art_manifest`, `site_space_words`, `identity_cases` and `seed_cases`
 *   were never route payloads: the first is the host's own art directory,
 *   the other three CONTRACTS the client holds itself to (the terminal's
 *   words, the key grammar's verdicts, a seed's verdict).
```

  `worker/test/corpus.test.ts`: `"seed_cases",` INSERTED directly ABOVE `"site_space_words",` (its one sorted
  position), and the contract bullet becomes ` * - `site_space_words`, `identity_cases` and `seed_cases` are
  contracts,` / ` *   never a route's payload.`. **After Tasks 1, 3 and 4 the whole list reads exactly:**

```ts
const THIRD_COLUMN = [
  "explanation_terminal",
  "foreign_bot_409",
  "house_chat_off_404",
  "identity_cases",
  "locked",
  "production_bot",
  "seed_cases",
  "site_space_words",
  "table_bot",
  "table_foreign",
  "watching",
  "why",
  "why_foreign",
  "why_refusals",
  "why_refused",
];
```

  Before the edit, each replay fails its third-column equality naming `seed_cases`; run one with the edit reverted if
  a red is to be quoted.

- [ ] **Step 13: The whole layer, and the corpus as regenerated.**

```bash
cargo test -p yokohama-web
cargo test -p yokohama-web --test wire_fixtures
git status --short web/src/wire web/src/wire-script
```

Expected: green; `wire_fixtures` green WITHOUT `UPDATE_WIRE_FIXTURES`; `git status` shows only the new
`seed_cases.json` (and the edited `contract.ts`). **No payload in the corpus moves:** every refusal's bytes are
`error_body`'s, which is the old rule with its third arm; no walk row is an array body (`walk.json`'s 2,197 rows hold
none); the two sentences are in no fixture; `warnings` stays `[]`. A changed payload is a FINDING: stop and report it.

- [ ] **Step 14: The per-edit gates** (Global Constraints), all of them, including `cd worker && npm run
  build:module:debug && npm run typecheck && npm run test:scripts && npm test` (the layer and `create.rs` changed) and
  `cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm`. The e2e is **not owed**:
  nothing a browser does or shows moves in play; the two reworded sentences answer a spectator's download (the client
  offers a spectator none) and the 257th watcher's unlock; the module's replay (`test:wasm`) covers the layer compiled
  for wasm. The orchestrator may rule otherwise.

  Counts that move, as deltas over Task 6's reading: the layer's lib **+10** (1 `client.rs`, 4 `api.rs`, 1 `signal.rs`,
  2 `games.rs`, 2 `platform.rs`; the axum-parity cell is extended, not new); `browser_played_pin` **+1**;
  `wire_fixtures` **+1**; `yokohama-worker` **±0** (one cell replaced, one repointed); both replays' third columns grow
  by one name (their counts hold).

  **The reviewer reads** `git diff 1e16fe27 -- crates/yokohama-web/src` against Tasks 1–6's lists plus:
  `platform.rs` (new); `api.rs` (`error_body`/`error_value` with three arms, `NOT_AN_OBJECT`, `parse_json`'s tail,
  `MAX_WAIT_SECONDS`/`check_wait`, one sentence, four cells); `lib.rs` (the module line, `IntoResponse`, `events`,
  `house_stream`, the parity arm, one pin, the scan's list); `client.rs` (the filter, the wrap, one cell); `signal.rs`
  (`house_frame`, one cell); `games.rs` (`HostClock`, two cells); `tables.rs` (one sentence, one pin).

- [ ] **Step 15: Commit.**

```bash
git add crates/yokohama-web/src/platform.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/api.rs \
  crates/yokohama-web/src/client.rs crates/yokohama-web/src/signal.rs crates/yokohama-web/src/games.rs \
  crates/yokohama-web/src/tables.rs crates/yokohama-web/tests/browser_played_pin.rs \
  crates/yokohama-web/tests/wire_fixtures.rs web/src/wire/seed_cases.json web/src/wire/contract.ts \
  web/src/local/corpus.node.test.ts worker/test/corpus.test.ts \
  crates/yokohama-worker/src/create.rs crates/yokohama-worker/src/lib.rs crates/yokohama-worker/src/house.rs
git commit -m "feat(web): the layer's small corrections — the drift filter, the object form, the restatements exported with all three arms, spectate, and the seed table"
```

The body quotes Steps 1, 3, 5, 7 and 9's reds and Step 8's mutant red.

---

### Task 8: The whole-game gate's livelock, and the browser walk's cap (spec §2.6; review o5)

**THE LEAST-VERIFIED TASK IN THIS CYCLE.** D3 wrote it without compiling; the retired review stopped at Task 7. Every
`file:line` below was verified at HEAD (`play_a_whole_game.rs:54, 96, 106, 140, 226, 537, 547, 600, 616, 668`;
`harness.ts:78, 463, 474, 531, 549, 555, 587, 606, 637, 647`; the four specs' `const CAP = 4000` lines; the specs' cap
handling lines), but no one has run its code. **Its reviewer measures it first and hardest**: the gate's arm alone,
then the 50-run reading, then the walk. It lands last so that nothing depends on it.

**Files:**
- Modify: `crates/yokohama-web/tests/play_a_whole_game.rs`:
  - the module doc (`:15-23`) and the imports (`:25-31`);
  - `Api` (`:54-96`), plus `restarted` and the new helpers `Stuck`, `probe_a_placement` and `bot_stream_seed`;
  - `choose`'s doc (`:106-111`);
  - the vs-bot arm (`:140-226`);
  - the two-people arm's two guards (`:537-547`, `:600-668`), now through the shared helpers, byte-for-byte the same
    walk.
- Modify, in `web/e2e/`:
  - `harness.ts`: `Screen` (`:463-474`) and the object `readScreen` returns (`:531-549`); `Affordance` (`:555`); `act`'s
    placement branch (`:587-606`); `cancelPicker`, `capReached` and `capSentence` after `deadEnd` (`:637-647`).
  - The four specs' `CAP` comments and their cap handling: `plays_a_whole_game.spec.ts:26-51` (`const CAP` at `:28`;
    the loop `:38-44`); `offline.spec.ts:73-74` (`const CAP` at `:74`), `:186-195` (`playToOver`);
    `plays_locally.spec.ts:39-41`, `:257-268`; `two_browsers.spec.ts:75-78`, `:412`, `:422`.
  - `the_table.spec.ts:196-197`: one comment sentence, "a placement clicks up to three tiles".
  - `harness.node.test.ts:26` and a new `describe`.
- Create, in the ledger (gitignored), `.superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/`: `task-8-gate-50.txt`;
  `task-8-e2e.txt`.

**Interfaces:**
- Consumes: `AppState::in_memory()` in the `open` mode (Task 2), and `AppState::with_store(Arc<dyn Store>,
  IdentityMode)`; `GameRecord.run.seats[..].agent_seed` (`crates/yokohama-core/src/game/record.rs:135-138`); the layer
  as Tasks 1–7 left it.
- Produces: a gate whose vs-bot arm is bounded, with a MEASURED cap and every run's bot seed printed — **cycle C
  returns it to CI** on this reading; `YOKOHAMA_WEB_GATE_BOT_SEED`; the harness's `capReached`/`capSentence`,
  `Screen.pressed/single` and `Affordance` `"cancel"`.

**The diagnosis this task acts on.**
- **The server gate.** The vs-bot arm "places nothing and leans on the bot to end the game". It hit its 20,000-input
  cap at turn 7,295 with no end trigger advancing (M6.3 close §6.3). It fired again in M6.5b2 (1,558 s) and twice in
  M6.5c. M6.4's cycle never saw it, which is what an intermittent defect looks like. **Both guards are ported** (review
  o5): the placement probe and the stuck-picker rule — as ONE copy the two arms share, so they cannot drift. The
  two-people arm's walk stays byte-for-byte the same: the same requests and the same RNG draws in the same order. The
  create seed is fixed, and the bot's stream seed is OS entropy (`tables.rs:1210`), so every run prints the bot seed
  read off the record, and a variable pins it to reproduce one.
- **The browser walk ALREADY places** (`harness.ts:596-605`, since M2). Its two recorded cap hits were 4,001-move
  games "with its end triggers unmet" (M6.5c close §6.1) — long games, not a livelock. What the walk lacks is what
  the server probe has: it ASKS each tile and counts only what the server took (today it clicks blind, and can toggle
  a selected tile OFF). It also lacks the stuck-picker rule. And a cap hit reports through `deadEnd`'s stuck-table
  sentence in `two_browsers` (`:412`, `:422`), which "is also the message for a genuinely stuck table" (M6.4 close
  §6.3). So the probe and the rule are ported, and a cap hit becomes a sentence of its own that prints the
  end-trigger strip.

- [ ] **Step 1: The gate's failing claim.** In the vs-bot arm, after its loop (before `assert_eq!(state["over"], true,
  …)`), add:

```rust
    assert!(
        selections > 0,
        "the vs-bot arm never selected an area, so the placement probe the livelock fix rests on never ran",
    );
```

  with `let mut selections = 0usize;` beside `let mut inputs`.

```bash
cargo test -p yokohama-web --test play_a_whole_game a_whole_two_player_game_against_the_bot
```

  Expected: **FAILS** — "the vs-bot arm never selected an area…". The arm has no probe yet. Quote it. (If the old
  livelock happens to fire on this run instead, it panics at the cap after up to ~30 min. Re-run; the probe's absence
  is what this red is about.)

- [ ] **Step 2: The shared guards.** In `play_a_whole_game.rs`:
  1. **The imports** gain `use yokohama_core::GameRecord;` and `use yokohama_web::store::{Doc, Store};`.
  2. **`Api`** gains a `state` field and `restarted`:

```rust
struct Api {
    state: Arc<AppState>,
    router: axum::Router,
}

impl Api {
    fn new() -> Self {
        Self::over(Arc::new(AppState::in_memory()))
    }

    fn over(state: Arc<AppState>) -> Self {
        Self { router: app(Arc::clone(&state)), state }
    }

    /// The same store behind a FRESH server — every handle gone, as a
    /// container that slept (M6.5c2): the next request rebuilds each game from
    /// its record.
    fn restarted(&self) -> Self {
        let store: Arc<dyn Store> = Arc::clone(self.state.games.store());
        Self::over(Arc::new(AppState::with_store(store, self.state.identity)))
    }
```

     (`call` and `call_as` are unchanged.)
  3. **The helpers**, after `choose`:

```rust
/// **The stuck-picker rule, ONE copy for both arms** (M6.5c2; the rule and its
/// measurement are M6.1's). A prompt that does not move is a walk that does not
/// terminate: a pair the server accepted one area at a time can still be
/// unaffordable together, and then Confirm is refused and the same picker comes
/// back unchanged for ever — MEASURED: seats holding `A03, A04` with one yen owed
/// to the President in A03 and no money, refused 40,000 times. So three
/// identical prompts in a row means the answer is to back OUT of the picker
/// rather than to keep pushing — `wire_fixtures.rs`'s walk's rule, for its
/// reason.
#[derive(Default)]
struct Stuck {
    repeats: usize,
    last: Value,
}

impl Stuck {
    /// Whether `prompt` is the same prompt a third time running — the caller
    /// then backs out with `cancel`.
    fn again(&mut self, prompt: &Value) -> bool {
        if *prompt == self.last {
            self.repeats += 1;
        } else {
            self.repeats = 0;
            self.last = prompt.clone();
        }
        self.repeats >= 3
    }
}

/// **The placement probe, ONE copy for both arms** (M6.5c2). `place_a` does not
/// carry its legal set (Option B does), so this ASKS: tiles holding a
/// President first, so a placement sometimes costs a fee, then the area it
/// last placed in, then a rotated order so the walk does not place in area 0
/// every turn. The server decides what it will accept, and nothing here
/// restates a rule. The reply that GREW the selection, or `None` when the
/// picker already holds two or no area takes one — placing none is legal, so
/// the caller falls through to [`choose`].
async fn probe_a_placement(
    api: &Api,
    who: &str,
    id: &str,
    at: &Value,
    rng: &mut Rng,
    placed_before: &mut Option<u64>,
) -> Option<Value> {
    let already = at["prompt"]["selected"].as_array().map_or(0, Vec::len);
    let want = at["prompt"]["max_areas"].as_u64().unwrap_or(0).min(2) as usize;
    if already >= want {
        return None;
    }
    let occupied: Vec<u64> = at["frame"]["board"]["tiles"]
        .as_array()
        .map(|tiles| {
            tiles
                .iter()
                .filter(|t| !t["presidents"].as_array().map(Vec::is_empty).unwrap_or(true))
                .filter_map(|t| t["area"].as_u64())
                .collect()
        })
        .unwrap_or_default();
    let offset = rng.below(24) as u64;
    let order: Vec<u64> =
        occupied.into_iter().chain(*placed_before).chain((0..24).map(|i| (i + offset) % 24)).collect();
    let uri = format!("/api/games/{id}/input");
    for area in order {
        let (status, probe) =
            api.call_as(who, "POST", &uri, Some(serde_json::json!({ "line": format!("A{area:02}") }))).await;
        let grew = probe["prompt"]["kind"] == "place_a"
            && probe["prompt"]["selected"].as_array().map_or(0, Vec::len) > already;
        if status == StatusCode::OK && grew {
            *placed_before = Some(area);
            return Some(probe);
        }
    }
    None
}

/// The bot's stream seed at `id` — pinned by `YOKOHAMA_WEB_GATE_BOT_SEED` to
/// reproduce a recorded run, else the one the server drew.
///
/// **Why this exists** (M6.5c2, spec §2.6; review o5): the create's seed is
/// fixed and the walk's RNG is fixed, but the BOT's stream seed is drawn from OS
/// entropy at finalize (`tables.rs`'s `draw_seed`), so a run that fails is
/// reproducible only once that third seed is known. It is read off the stored
/// record's `run.seats`, which is what a rebuilt game's bot plays from. It is
/// PINNED by rewriting that record before the first move and restarting the
/// server over the same store, so the next request rebuilds the game from the
/// pinned seed exactly as a container that slept would.
fn bot_stream_seed(api: Api, id: &str) -> (Api, u64) {
    let store: Arc<dyn Store> = Arc::clone(api.state.games.store());
    let (json, version) = store.read(Doc::Record, id).expect("a readable store").expect("a dealt game's record");
    let mut record: GameRecord = serde_json::from_str(&json).expect("the stored record parses");
    let pinned = std::env::var("YOKOHAMA_WEB_GATE_BOT_SEED")
        .ok()
        .map(|s| s.trim().parse::<u64>().expect("YOKOHAMA_WEB_GATE_BOT_SEED is a u64"));
    let Some(seed) = pinned else {
        let drawn = record.run.as_ref().and_then(|run| run.seats.iter().find_map(|s| s.agent_seed));
        return (api, drawn.expect("the bot seat records its stream seed"));
    };
    assert!(record.events.is_empty(), "premise: the seed is pinned before the first move");
    let run = record.run.as_mut().expect("a web game records its seats");
    for seat in run.seats.iter_mut().filter(|s| s.agent_seed.is_some()) {
        seat.agent_seed = Some(seed);
    }
    let rewritten = serde_json::to_string_pretty(&record).expect("a record serializes");
    store.write_if_version(Doc::Record, id, &rewritten, Some(version)).expect("the pinned record lands");
    (api.restarted(), seed)
}
```

  4. **The two-people arm through the helpers** (one walk, unchanged):
     - `let mut stuck = 0usize; let mut last_prompt = Value::Null;` (`:546-547`) becomes `let mut stuck =
       Stuck::default();`. The comment above it (`:537-545`) now lives on `Stuck`, and is replaced by "// The
       stuck-picker rule — [`Stuck`]."
     - The stuck block (`:600-614`) becomes:

```rust
        if stuck.again(&mine["prompt"]) {
            let (status, next) = api
                .call_as(mover, "POST", "/api/games/two/input", Some(serde_json::json!({ "line": "cancel" })))
                .await;
            assert_eq!(status, StatusCode::OK, "backing out of a stuck picker: {next}");
            assert_built_for(&next, mover, seat);
            state = next;
            continue;
        }
```

     - The probe (`:616-668`) becomes:

```rust
        // **The placement probe** — [`probe_a_placement`].
        if mine["prompt"]["kind"] == "place_a" {
            let probed = probe_a_placement(&api, mover, "two", &mine, &mut rng, &mut placed_before).await;
            if let Some(next) = probed {
                selections += 1;
                assert_built_for(&next, mover, seat);
                state = next;
                continue;
            }
            // No area will take one — a real state late in a game, when the
            // assistants are gone. Placing none is legal, so fall through.
        }
```

     (The helper draws `rng.below(24)` only when the original did, after the same `already < want` test, so the arm's
     RNG sequence and its requests are the original's. **The reviewer checks that claim by diffing the two-people
     arm's request log before and after**, since nobody has.)

- [ ] **Step 3: The vs-bot arm, guarded.**
  1. **The cap.** Before the arm:

```rust
/// **The vs-bot arm's input cap.** 20,000 until M6.5c2's measurement, below,
/// replaces it.
const VS_BOT_CAP: usize = 20_000;
```

  2. **The seed and the loop.** After the keep, and before `let (status, mut state) = api.call("GET",
     "/api/games/gate", None).await;`:

```rust
    let (api, bot_seed) = bot_stream_seed(api, "gate");
    // Printed FIRST, so a run that never finishes still names its seed.
    eprintln!("gate: start bot_seed={bot_seed}");
```

     The loop becomes:

```rust
    let mut rng = Rng(0x594F_4B4F_5745_4200); // "YOKOWEB"
    let mut inputs = 0usize;
    let mut turns_seen = 0u64;
    let mut selections = 0usize;
    let mut backed_out = 0usize;
    let mut stuck = Stuck::default();
    let mut placed_before: Option<u64> = None;

    while state["over"] != serde_json::Value::Bool(true) {
        inputs += 1;
        assert!(inputs < VS_BOT_CAP, "a game must finish inside the cap (bot seed {bot_seed}); last state: {state}");
        turns_seen = turns_seen.max(state["frame"]["decision"]["turn_number"].as_u64().unwrap_or(0));

        // **The two guards the two-people arm has** (M6.5c2, spec §2.6;
        // review o5). Without them this arm placed nothing and leaned on the
        // bot to end the game, which on some OS-entropy bot seeds it never did
        // (M6.3 close §6.3: turn 7,295, no end trigger advancing).
        if stuck.again(&state["prompt"]) {
            let (status, next) =
                api.call("POST", "/api/games/gate/input", Some(serde_json::json!({ "line": "cancel" }))).await;
            assert_eq!(status, StatusCode::OK, "backing out of a stuck picker (bot seed {bot_seed}): {next}");
            backed_out += 1;
            state = next;
            continue;
        }
        if state["prompt"]["kind"] == "place_a" {
            // Bound first, so no borrow of `state` outlives the probe.
            let probed = probe_a_placement(&api, "red@wire.test", "gate", &state, &mut rng, &mut placed_before).await;
            if let Some(next) = probed {
                selections += 1;
                state = next;
                continue;
            }
        }

        let Some(line) = choose(&state, &mut rng) else { break };

        let (status, next) =
            api.call("POST", "/api/games/gate/input", Some(serde_json::json!({ "line": line }))).await;
        assert_eq!(status, StatusCode::OK, "input {line:?} was refused (bot seed {bot_seed}): {next}");
        state = next;
    }
    eprintln!(
        "gate: done bot_seed={bot_seed} inputs={inputs} events={} turns={turns_seen} selections={selections} backed_out={backed_out}",
        state["events"],
    );
```

     The `selections > 0` assert from Step 1 follows. Everything after it is unchanged.
  3. **The docs.** `choose`'s doc (`:106-111`) becomes: "**It places NOTHING**, and its callers make up for that with
     [`probe_a_placement`]. Placing none is legal on its own terms (Option A takes 0–3 areas), and it keeps the
     policy from having to know which tiles accept an assistant, a fact the engine owns and this test must not
     restate. The vs-bot arm once leaned on the bot to end the game instead, and livelocked on some OS-entropy
     seeds until M6.5c2." The module doc's paragraph (`:15-23`) gains: "**Both arms have the same two guards since
     M6.5c2**: [`probe_a_placement`] and [`Stuck`]. The vs-bot arm had neither, placed nothing, and livelocked (M6.3
     close §6.3). It prints the bot's stream seed, and `YOKOHAMA_WEB_GATE_BOT_SEED=<u64>` pins that seed to reproduce
     a run."

```bash
cargo test -p yokohama-web --test play_a_whole_game -- --nocapture
YOKOHAMA_WEB_GATE_BOT_SEED=7 cargo test -p yokohama-web --test play_a_whole_game a_whole_two_player_game_against_the_bot -- --nocapture 2>&1 | grep '^gate: '
YOKOHAMA_WEB_GATE_BOT_SEED=7 cargo test -p yokohama-web --test play_a_whole_game a_whole_two_player_game_against_the_bot -- --nocapture 2>&1 | grep '^gate: '
```

Expected: **both arms pass**; **the two pinned runs print IDENTICAL `gate: done` lines** (`bot_seed=7`, the same inputs
and events) — the reproduction claim, tried rather than read; an unpinned run prints a different `bot_seed`.

- [ ] **Step 4: The measurement — 50 runs, recorded.** Five foreground batches of ten, each inside the Bash tool's
  ten-minute ceiling; each batch appends to the ledger:

```bash
cd "$(git rev-parse --show-toplevel)"
for i in $(seq 1 10); do
  out=$(cargo test -q -p yokohama-web --test play_a_whole_game a_whole_two_player_game_against_the_bot -- --nocapture 2>&1); code=$?
  printf '%s\n' "$out" | grep '^gate: '
  [ "$code" -eq 0 ] || echo "gate: FAILED exit=$code"
done | tee -a .superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/task-8-gate-50.txt
```

  Then the reading:

```bash
f=.superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/task-8-gate-50.txt
echo "done $(grep -c '^gate: done' "$f")  failed $(grep -c '^gate: FAILED' "$f")"
grep '^gate: done' "$f" | sed -E 's/.* inputs=([0-9]+) .*/\1/' | sort -n \
  | awk '{v[NR]=$1} END {printf "runs %d  min %d  median %d  max %d\n", NR, v[1], v[int((NR+1)/2)], v[NR]}'
```

  Expected: `done 50  failed 0`, and a bounded spread. **A failed run, or a batch the ceiling cut, is a FINDING.** The
  last `gate: start bot_seed=…` without a `done` names its seed: reproduce it with `YOKOHAMA_WEB_GATE_BOT_SEED=<seed>
  cargo test -p yokohama-web --test play_a_whole_game a_whole_two_player_game_against_the_bot -- --nocapture`, then
  STOP and report to the orchestrator. **Never raise the cap to pass a run.**

- [ ] **Step 5: The cap, from the reading.** The rule: `VS_BOT_CAP` becomes the next thousand above four times the
  measured maximum, and never above 20,000. Its comment records the reading; the five `<…>` below are the Step 4
  reading's own figures and the commit it was taken at:

```rust
/// **The vs-bot arm's input cap, MEASURED** (M6.5c2, spec §2.6): 50 runs over
/// OS-entropy bot seeds at `<commit>`, after the placement probe and the
/// stuck-picker rule — inputs min <min>, median <median>, max <max>, and every
/// run's bot seed in the ledger's `task-8-gate-50.txt`. The cap is the next
/// thousand above four times that maximum, so a regression fails in minutes
/// where the old 20,000 cost half an hour (M6.3 close §6.3: 1,813 s).
const VS_BOT_CAP: usize = <cap>;
```

  Run the arm once more at the new cap: `cargo test -p yokohama-web --test play_a_whole_game`. Record the reading and
  the cap in `progress.md`.

- [ ] **Step 6: The browser walk's cap sentence — its node cell first.** In `web/e2e/harness.node.test.ts`, `:26`
  becomes `import { BEACON_PREFIX, capSentence, isBeaconCorsError, isDealAnswer, isDealAnswerLine } from "./harness";`.
  Then add:

```ts
describe("capSentence — a walk that reached its cap says so (M6.5c2, spec §2.6)", () => {
  // Never `deadEnd`'s words: a cap hit is a game still RUNNING — the two
  // recorded hits were 4,001-move games with their end triggers unmet — and
  // "nothing on the screen is answerable" named a stuck table instead, which
  // sent the reader after the wrong defect. Mutant: the sentence reverting to
  // `deadEnd`'s.
  it("names the cap, the game, its turn and its end triggers, and no dead end", () => {
    const text = capSentence("sim-00000000000000aa", 4000, "church 5/8 · customs 2/4", "TURN 212 · SEAT 1");
    expect(text).toContain("reached its cap at 4000 moves");
    expect(text).toContain("sim-00000000000000aa");
    expect(text).toContain("TURN 212 · SEAT 1");
    expect(text).toContain("church 5/8 · customs 2/4");
    expect(text).not.toContain("nothing on the screen is answerable");
  });

  it("says what it could not read rather than printing nothing", () => {
    const text = capSentence("local-1", 4001, "", "");
    expect(text).toContain("no turn on screen");
    expect(text).toContain("no end-trigger strip drawn");
  });
});
```

```bash
cd web && npx vitest run -c vitest.node.config.ts e2e/harness.node.test.ts
```

Expected: FAILS — `capSentence` is not exported.

- [ ] **Step 7: The harness — the probe, the stuck rule, and the cap's own sentence.** In `web/e2e/harness.ts`:
  1. **`Screen`** gains, after `tiles`:

```ts
  /** Tiles the open picker has SELECTED — `aria-pressed="true"` (M6.5c2). The
   *  walk confirms every picker it fills, and an accepted Confirm closes it, so
   *  a picker that opens with some already selected is one whose Confirm was
   *  REFUSED. */
  pressed: number;
  /** The open picker is Option B's, which takes ONE area (M6.5c2). */
  single: boolean;
```

  2. **`readScreen`**, in its returned object after `tiles`:

```ts
      pressed: many('.tile[role="button"][aria-pressed="true"]'),
      // Every `.decision`, because the class is drawn more than once
      // (`deadEnd`'s note) and the prompt's need not be the first.
      single: [...document.querySelectorAll(".decision")].some((d) =>
        (d.textContent ?? "").includes("click one area on the board"),
      ),
```

  3. **`Affordance`** becomes `"option" | "only" | "placement" | "count" | "confirm" | "cancel"`.
  4. **`act`'s placement branch.** The comment and the `if (screen.tiles > 0 && screen.confirm) { … }` block
     (`:587-606`) become:

```ts
  // A placement picker. The board is the input surface.
  //
  // Placing NOTHING is legal, but a walk that only ever did that would never
  // put an assistant on the board, and a game in which nobody places never ends
  // — measured on the fixture walk at turn 3,644 with nothing having happened
  // at all, and the server gate's vs-bot arm livelocked on exactly that until
  // M6.5c2. Option A takes up to three areas, so the count VARIES: a walk that
  // always placed exactly one never exercised a multi-area selection or the fee
  // that grows with it. Option B takes one.
  //
  // **Each tile is ASKED, as the server gate's placement probe asks** (M6.5c2,
  // spec §2.6). Every tile the client lets you toggle is clickable, but the
  // SERVER decides which it takes, and a refusal is a 200 carrying the engine's
  // line with the selection unchanged. So the walk clicks one UNPRESSED tile at
  // a time — a pressed one would toggle OFF — waits for the reply, and keeps
  // count of what the server took. A refusal costs one ask and never a Confirm.
  //
  // **And the server gate's stuck-picker rule, for its reason**: a set the server
  // took one area at a time can be unaffordable together, and the same refused
  // Confirm then comes back for ever (measured there 40,000 times).
  // - A picker that opens with a selection already made had its Confirm refused,
  //   since this walk confirms every picker it fills, so it backs OUT with Cancel.
  // - An Option B picker that took no area is backed out of the same way, since
  //   confirming it would be refused.
  // - An Option A picker that took none is confirmed empty: placing none is legal
  //   there, and it is how a walk with no assistants left gets past the phase.
  if (screen.tiles > 0 && screen.confirm) {
    if (screen.pressed > 0) return cancelPicker(page);
    const wanted = screen.single ? 1 : 1 + nth(Math.min(3, screen.tiles));
    let placed = 0;
    for (let asked = 0; placed < wanted && asked < wanted + 3; asked += 1) {
      const free = page.locator('.tile[role="button"][aria-pressed="false"]:not([aria-disabled="true"])');
      const n = await free.count();
      if (n === 0) break;
      await free.nth(nth(n)).click();
      placed = (await readScreen(page)).pressed;
    }
    if (placed === 0 && screen.single) return cancelPicker(page);
    await page.getByRole("button", { name: "Confirm" }).click();
    return "placement";
  }
```

  5. **After `deadEnd`:**

```ts
/** Back out of an open placement picker, through its own Cancel — always live
 *  there (`Decision.tsx`'s controls). */
async function cancelPicker(page: Page): Promise<Affordance> {
  await page.locator(".decision .controls button.ghost", { hasText: "Cancel" }).first().click();
  return "cancel";
}

/**
 * A walk that reached its cap, in a sentence that says so (M6.5c2, spec §2.6).
 *
 * **Never `deadEnd`'s sentence.** A cap hit is a game still RUNNING — the two
 * recorded hits (M6.3, M6.5c) were 4,001-move games with their end triggers
 * unmet — and "nothing on the screen is answerable" named a stuck table
 * instead. This names the cap, the turn and the end-trigger strip, so a hit
 * says how far the game got.
 */
export async function capReached(page: Page, game: Game, moves: number): Promise<Error> {
  const read = async (selector: string) =>
    (await page.locator(selector).first().innerText({ timeout: 2_000 }).catch(() => "")).trim();
  return new Error(capSentence(game.id, moves, await read(".end-strip"), await read(".decision .chain")));
}

/** `capReached`'s words — pure, so a node cell holds them apart from
 *  `deadEnd`'s. */
export function capSentence(id: string, moves: number, strip: string, chain: string): string {
  return (
    `the walk reached its cap at ${moves} moves and the game is still running — ` +
    `a game this walk has not ended, not a screen with nothing to click\n` +
    `  game ${id}, ${chain || "no turn on screen"}\n` +
    `  end triggers: ${strip || "no end-trigger strip drawn"}`
  );
}
```

  (The selectors `.end-strip` and `.decision .chain` are the harness's assumption about the game screen's classes;
  the implementer confirms both against `web/src/` before relying on them, and names the real ones if they differ.)

- [ ] **Step 8: The four specs report a cap hit as a cap hit.**
  1. **Each spec's harness import** gains `capReached`.
  2. **Each `CAP` doc** gains the sentence: "A hit throws `capReached`, which names the cap, the turn and the
     end-trigger strip — never `deadEnd`'s stuck-table sentence (M6.5c2)."
  3. **The three single-page loops** gain one line, directly after the `while (…) { … }` loop and before the spec's
     own `expect(moves, "the walk must not hit its own cap")…`: `plays_a_whole_game.spec.ts:38-44`; `offline.spec.ts`'s
     `playToOver`, `:189-193`; `plays_locally.spec.ts:260-266`:

```ts
  if (!screen.over) throw await capReached(page, game, moves);
```

  4. **`two_browsers.spec.ts`.** `:412` becomes `if (moves > CAP) throw await capReached(pages[first], { id },
     moves);`, and `:422` becomes `if (moves > CAP) throw await capReached(pages[second], { id }, moves);`.
  5. **`the_table.spec.ts:196-197`.** "a placement clicks up to three tiles and then Confirm, and every one of those
     four is its own `/input` post" becomes "a placement asks up to six tiles and then Confirms or Cancels, and every
     one of those is its own `/input` post".

```bash
cd web && npx vitest run -c vitest.node.config.ts e2e/harness.node.test.ts && npm run typecheck
```

- [ ] **Step 9: The e2e, the walk's own reading.** `cd web && npm run e2e`, sequenced: no other Playwright run at once.
  If the tool's ten-minute ceiling would cut it, run it in the background with its output to
  `.superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/task-8-e2e.txt`, and poll that file until the summary line
  appears. Expected: all passed (CLAUDE.md's reading 61 in 11). Then record every walk's own line:

```bash
grep -E 'finished in|LOCAL WALK|OFFLINE WALK|two-browser game|REOPEN|reached its cap' \
  .superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/task-8-e2e.txt
```

  Record each move count beside the M6.3–M6.5c readings (`two_browsers` 1,758–3,559 completed, two cap hits at 4,001).
  **A cap hit now prints its end-trigger strip.** Re-run that spec ALONE at the same commit (the lane's practice for
  this cell), and record both runs. A second hit is an orchestrator item, not something this task raises the cap for.

- [ ] **Step 10: The per-edit gates** (Global Constraints), all of them. Counts that move (deltas over Task 7's
  reading): `test:wasm` **+2** (the two `capSentence` cells); the gate binary stays **2** cells. CI is untouched:
  **cycle C** returns the gate to CI on Step 5's reading.

- [ ] **Step 11: Commit.**

```bash
git add crates/yokohama-web/tests/play_a_whole_game.rs web/e2e/harness.ts web/e2e/harness.node.test.ts \
  web/e2e/plays_a_whole_game.spec.ts web/e2e/offline.spec.ts web/e2e/plays_locally.spec.ts \
  web/e2e/two_browsers.spec.ts web/e2e/the_table.spec.ts
git commit -m "test(web): the whole-game gate's vs-bot arm guarded and measured; the browser walk asks its tiles"
```

The commit body quotes Step 1's red, Step 3's two identical pinned lines, Step 4's reading, Step 6's red, and Step 9's
walk lines.

---

### Task 9: The cycle's close (ORCHESTRATOR)

No code. The orchestrator's steps, in order. **The push and the PR are PRE-AUTHORIZED** (the owner, 2026-09-27:
*"push and PR are pre-approved at the orchestrator's discretion"*); nothing here waits on the owner. **Step 5's
handover is the ENTRY for cycle B's planning session** (the owner, 2026-09-27: the close "should provide the handover
to handle the plan for the B phase"): a fresh orchestrator writes cycle B's plan from it, the spec's §4 row B, the
drafts and the mocks, against this branch's real tree — so it is written for that reader and for nobody else.

- [ ] **Step 1: The Fable whole-branch review.** One Fable reviewer over `git diff 1e16fe27..HEAD`, the spec, this plan
  and the ledger, working in an EXPORT: the vacuity lens first (every new cell reverted against its fix — the
  containment mutants, the foreign-bot masks, the log seam's drop, the export's third arm); the corpus diff read whole
  against §4's interface list; the fences; the sentences this plan writes about itself tried rather than re-read
  (the top yield, every cycle). Its report goes to the ledger.
- [ ] **Step 2: ONE fix wave**, each fix a NEW commit by an Opus implementer, reviewed by an Opus reviewer, then a
  scoped Fable re-review of the fixes. A reviewed SHA is never amended.
- [ ] **Step 3: The per-edit suites at the reviewed head**, every crate and package this cycle touched, recorded in the
  ledger with the machine tag: `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`, `-p
  yokohama-cli`; clippy in both shapes; the wasm32 check and both builds; the public build's `identity::` (8); `web/`'s
  four commands; `worker/`'s four; the fence printing nothing. **No release gates, no deploy, no live tier**
  (R-6.5c2.13: the full sequence is cycle C's).
- [ ] **Step 4: The push and the PR, at the orchestrator's discretion.** `git push -u origin web-room-m6.5c2a`, then
  `gh pr create --base web-room-m6.5c --head web-room-m6.5c2a` with a description that is the cycle in one screen:
  what landed by task, the readings at the reviewed head, the interfaces cycle B consumes, what the cycle does NOT
  claim, ending with the attribution line. Record the PR's number in the ledger and on the board; the spec's §4
  table says "expected #39", and the number GitHub gives is the record.
- [ ] **Step 5: The handover into cycle B — its planning session's entry.** Write
  `docs/plans/<date>-web-frontend-m6.5c2b-handover.md`, under 200 lines, every fact read off the TREE at the reviewed
  head (`git grep`, the corpus files, the ledger), never off this plan. Its sections, in order:
  1. **Where cycle A left the tree:** the branch, its HEAD, the PR; every suite's reading at the reviewed head,
     machine-tagged; the fence check's (empty) output; the e2e runs' lines.
  2. **The wire as it IS**, which cycle B renders from and nothing else: each corpus file by name with one line on
     what it carries — `identity_cases.json`, `seed_cases.json`, `production_bot.json`, `table_bot.json`,
     `foreign_bot_409.json`, `table_foreign.json`, `why_foreign.json`; `TableView.bot` and `ClientState.bots`;
     `me.json` and `health.json` reading `open` — and the `types.ts` and `contract.ts` names beside each
     (`IdentityCase`, `wire.identityCases`, `wire.seedCases`, `SeatBot`, `TableBot`, `ProductionBot`, `ForeignBot`,
     `ForeignBotErrorBody`, `wire.tableBot`, `wire.productionBot`, `wire.foreignBot409`, `wire.tableForeign`,
     `wire.whyForeign`, `KEYS.*`), copied from the files.
  3. **What cycle B builds** (spec §4 row B, restated in one paragraph) **and its raw material:** the drafts D4
     (retired Tasks 9–12: the shared pieces, the door and the project link, the identity states and the name step,
     the lobby) and D5 (retired Tasks 13–16: the forms and the seed check, waiting and joining, "Spectate" and the
     single-player items, the client's small items) under `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/drafts/`
     — run in `web/`-only exports over stand-ins, their e2e cells type-checked but never run; the eleven mocks in
     `docs/plans/2026-09-26-web-frontend-m6.5c2-mocks/`; and the known corrections: the review's P-7
     (`web/e2e/the_table.spec.ts:430-433` drives the old table form; the forms task rewrites it), D4's amendment 1
     (the identity rows `nameRule.test.ts` requires are all in `identity_cases.json` now), D4's amendments 2–4 and
     D5's names as the retired plan's "Additions reconciled at assembly" listed them, and the assembly's spec-gap
     answers (spec §10.5's tail: "How to play" opens the help without its card-art screenshots; the creator's lobby
     rows keep Abandon beside their one action; row actions are buttons and the name is the one link; state C's
     discriminator tests state A first; state D is the door's alone and the lobby shows "loading…" rows; the iPhone
     keeps its install instruction; the joiner's password sits above the seats and the one join button below; the
     open table's chat stays in the main column; other people's seat cards read "seated"; test data never uses the
     owner's surname).
  4. **The rules cycle B's plan carries:** every task that changes what the client shows loads `frontend-design`
     and builds against the mocks in the app's own stylesheet and tokens, no new colour roles, no card art on the
     setup screens, the door asking the server nothing; every payload a component renders comes from `web/src/wire/`,
     nothing hand-authored (the M2.5 rule); `npm run e2e` owed by every task that changes what a browser does or
     shows, sequenced; a 390 px cell per rebuilt screen; a headed look, because no jsdom cell sees a stylesheet; counts
     as deltas; one Opus implementer and one Opus reviewer per task, Fable for the whole branch; **a plan sized to be
     read whole** — the project's executed range is 1,000–3,300 lines, and cycle B's code is React whose contract is
     the mock, so it needs far less inline code than cycle A did: intent, files, interfaces, the cells to write and the
     traps, with code only where it defines a contract.
  5. **What cycle A does NOT claim:** Task 8's cap rests on one run of 50; the NAS still runs `claimed` and is not
     redeployed before cycle C's close; the module and the browser host still RESTATE the layer's items (cycle C
     switches them); the live tiers were not run; and whatever the whole-branch review left open, named.
  6. **Every ruling taken in this cycle's ledger**, one line each, dated.
  7. **Cycle C's standing items this cycle created or found:** the live replay's reset recipe owed after Task 5's
     regeneration (once, after the bump); the whole-game gate back into CI on Task 8's reading; the import switch;
     anything else the ledger names for C.

  Then: `docs/handover.md`'s "Updated" line and Track 3 point at the handover and say cycle B is next; the
  orchestrator's memory (`yokohama-web-lane-status`) records the cycle's end and the PR number. Commit the handover
  and the board together; push (pre-authorized).
- [ ] **Step 6: Nothing else.** The documents, CLAUDE.md's counts and the decision-log rows are cycle C's.

---

## Appendix A — the retired plan review's findings, and the step each lands in

| finding | where it lands here |
|---|---|
| P-1 (CRITICAL) RUNBOOK §2 served the repository root | fixed before this plan, commit `9b4320f4` |
| P-2 (HIGH) the surname substitution | every test person is `Ben Castle`; Global Constraints; Tasks 1–2 rewritten to it |
| P-3 (HIGH) `error_value` dropped the `foreign_bot` arm | Task 7 Step 6.1 (three arms) and Step 5.1 (the third shape in the cell) |
| P-4 (HIGH) the owed fixture's mirror | Task 3 Step 6 |
| P-5 (HIGH) the pool's corpus cell and `run_id` | Task 5 Step 6 |
| P-6 (HIGH) the `Reply` fakes' `logs` | Task 6 Step 12 |
| P-7 (HIGH) `the_table.spec.ts`'s old form | cycle B |
| P-8 (MEDIUM) a pasted `THIRD_COLUMN` | the insert rule in Global Constraints and Couplings; the literals in Tasks 3, 4 and 7 |
| P-9 (MEDIUM) the secret's hand-off | cycle C |
| P-10 (MINOR) base-relative counts | every count a delta over the implementer's own HEAD |
| P-11 (MINOR) the grep before the rewrite | Task 2 Step 16 |
| O-1 (OPEN) P-4's shape | (a) taken, Task 3 Step 6 |

## Appendix B — the ledger's evidence, and how to use it

`.superpowers/sdd/2026-09-27-web-frontend-m6.5c2a-plan/` (gitignored):
- `review-chain-patches/0001…0008.patch` — the retired review's chain, `git format-patch` over its base commit: Tasks
  1–2 (D1's trial with the assembly's surname substitution), 3–5 (D2's trial), two alignments to the plan's text, Task
  6 (the layer, worker, wasm) and its node cell, Task 7 as written, and the P-3 stand-in. **Not an implementation to
  cherry-pick** — the commits are a reviewer's assembly, not TDD commits, and they carry P-2's inconsistent surname —
  but a reviewer's oracle: `git am` them over an export of `1e16fe27` to see the tree the review measured, and diff an
  implementer's commit against it.
- `review-chain.bundle`, `d1-trial.bundle` — the same trees whole (`git clone <bundle> <dir>`).
- `d1-trial-tasks-01-02.diff` — D1's combined diff (2,484 lines).
- `traversal_probe.rs` — the spec review's measured probe, the template for Task 1's regression cell.
- `progress.md` — this cycle's ledger: one line per task, one `Ruling:` line per deviation, every agent's report path.
