# Security S1b — the cost bounds: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. Every code task subagent-driven, the implementer's tier in each task's title — **Opus** for
> Tasks 1–7, **Sonnet** for Task 8 — every task reviewer **Opus** (the owner's tiers for the S1 series,
> 2026-10-09: Sonnet/Opus and Opus/Opus, Fable only for a task unusually undefined or long-running — none here is),
> and a **Mythos** orchestrator that conducts the plan review at the top and the whole-branch review at the end
> ITSELF, on exports; the suites, the documents' close and the SDD ledger are the orchestrator's too (Task 9).
> Steps use checkbox (`- [ ]`) syntax. **Read the spec first**
> (`docs/plans/2026-10-08-security-remediation-spec.md`: §1 the rulings R-SEC2, R-SEC7, R-SEC8(e)(f), §2.1 the rows
> A2, A3, B12, D6, D25, D27, D29, §6 the fences, §9.2 the stacked branch, §9.3 S1a's split and the S1 figures),
> then this plan's §"Names shared across tasks" — an implementer sees only its own task, and that section is how
> it learns what its neighbours produce. Every `file:line` here was read at `security-s1`'s head `604d7269`
> (S1a's close; the code head `8b7c24dd`); **find by content once an earlier task has moved a file.** The
> implementer PROBES the brief's code before its own review and reports `DONE_WITH_CONCERNS` when a sentence here
> proves false.

**Goal:** Bound what an untrusted record and a hostile table can cost the web lane's hosts — one archive upload
of legal moves costing tens of seconds of replay (A2), a hot-seat table in the engine's worst-case state answering
frames of tens of megabytes (A3), an archive with no per-caller quota (B12), compute routes on the async executor
(D6), an archive that files impossible configurations and forged attribution (D25), an out-of-range `/at` that
answers 500 (D27) and a false fence sentence (D29) — with `SEMANTICS_REV` at 7, the five fenced crates
byte-identical, the CLI touched in `session.rs` alone and nowhere the bot decides, and the wire corpus regenerated
ONCE, in Task 6, its diff read.

**Architecture:** Five bounds, each where the cost is made. (1) `backup::check_pair_with` — the ONE validator an
upload and a restore share — refuses a record by its event COUNT before anything is replayed (`EVENT_CAP`, 4,096:
seven times the longest game anyone has played here), replays through the core PR's predicate check
(`replay_strict_with(.., MoveCheck::Predicate)`, so an admitted replay costs tens of milliseconds whatever the
record's families), and the archive route answers a retry of a filed record before any replay and refuses a body
over 512 KiB. (2) `check_pair_with` also refuses what the engine would — an impossible configuration, a table
whose seats disagree with the record's bots, a locked-down record, a key the record does not have, a bot this
server does not run. (3) The directory archive counts a caller's records and
bytes under `browser/<caller>/` before every put (256 records, 128 MiB) and refuses with a 507 the outbox shows
and retries. (4) The NAS adapter runs every dispatch on the blocking pool and the five replay routes behind a
two-permit gate with a two-second bounded wait, then 503 `Retry-After: 1`. (5) The CLI's `Session` keeps a
ladder of states every sixty-four events and folds through the predicate, so `/at`, a run, an undo and a cold
rebuild never replay a record from setup through a materialized retreat family; the layer builds every frame's
option list from the factored action source with the retreat family FOLDED to its recover-nothing member and the
rest capped, the omitted count carried as `options_omitted`. Beside them: `/at` answers its documented 404 and an
empty run without replaying (D27), and the live corpus replay learns the production seed rule (S1a's residual).

**Tech Stack:** Rust 1.97.1 (`rust-toolchain.toml`): `crates/yokohama-web` (the layer, its route cells and
`tests/wire_fixtures.rs`), `crates/yokohama-cli` (`session.rs` alone); TypeScript: `web/src/types.ts`,
`web/src/wire/contract.ts`, `web/src/outbox.ts` and their vitest cells; node: `worker/scripts/live-replay.mjs`,
`worker/scripts/replay/exemptions.mjs` and `worker/scripts/live-replay.test.mjs`; the committed wire corpus
`web/src/wire/`; one new committed fixture pair under `crates/yokohama-web/tests/fixtures/`.

**Spec:** `docs/plans/2026-10-08-security-remediation-spec.md` — §2.1's rows A2, A3, B12, D6, D25, D27, D29;
R-SEC2, R-SEC7, R-SEC8(e), R-SEC8(f); §6 the fences; §9.2 the stacked branch; §9.3 the S1 figures. The
corrections this plan makes to it are §"Spec corrections (to be recorded as §9.4)" below — recorded forward in the
spec's §9 by Task 8.

## The owner's rulings for this cycle (2026-10-09), and what each binds

1. **The knobs (R-SEC8(f)): the recommended table of spec §9.3** — the A2 event cap **4,096 events**, the B12
   body cap **512 KiB**, the per-caller quota **256 records and 128 MiB**, the NAS semaphore **2 permits with a 2 s
   bounded wait, then 503 with `Retry-After: 1`**. The figures are §"The S1 figures" of S1a's plan
   (`docs/plans/2026-10-09-security-s1a-leaks-plan.md` lines 3494–3530; the spec's §9.3 lines 240–276). Under
   them the review's worst-case body (23,606 events) is refused by COUNT, so spec §5.3's probe "the worst-case
   record is refused by count before any replay, on both hosts" stands as written.
2. **The comment rule (the owner, 2026-10-09):** every knob constant carries a CONCISE in-code comment defending
   the choice — the refusal is based on real-world game testing (the measured finished games: 250–600 events,
   under 60 KB pretty-printed; the longest, the browser-played fixture, 595 events and 54 KB), and the upload
   path is not designed for worst-case imports — because the spec and the plans of earlier cycles may not be in
   the public repository. The comments in this plan's code blocks ARE that text; an implementer transcribes them
   and does not shorten them.

## The decisions this plan takes (the orchestrator's, through the PR and the hand-off; the owner reads them at the plan review)

- **D-1 — the predicate wiring's catcher is a cost bound, and it is the only one it can have.** The core PR proved
  `replay_strict_with(.., Predicate)` accepts exactly what `replay_strict` accepts, so no functional cell can tell
  a `Predicate` fold from a `Listed` one. The catcher is a DEBUG-profile ceiling over the committed cap-sized
  spread fixture (Task 1): under the predicate the 4,096-event replay folds in under a second; under the materialized
  list the 235 of its positions that are seat 0's Movement decisions at the 442,368-move family run to
  minutes (198 s in debug, measured). The bound is MEASURED in the before review and set at five times the
  predicate's reading (0.55 s → 3 s).
- **D-2 — the cap-sized spread fixture is cut from the review's own worst-case body**, the first 4,096 events of
  `docs/temp/security-review-2026-10-08/worst-case/spread_4p_11_body.json` re-stamped with their terminal digest
  and paired with a four-seat hot-seat table of the browser's own player — committed under
  `crates/yokohama-web/tests/fixtures/` (outside the corpus, as the browser-played pair is, since it carries a
  seed) with a provenance file naming the cut. The generator the review used (its `sec_probe_cost.rs`'s p3c probe)
  is DESCRIBED in the provenance file from the review's report — the source was the reviewer's scratch and is
  not retained; it is not a test.
- **D-3 — A3's CLI edit is `session.rs` alone, and it is where the fix belongs** (spec correction 2 of §9.3 left the
  choice to this plan, "preferring the layer where a layer-side change is equal"): the record has ONE owner, the
  `Session`, and only it can keep checkpoints consistent across `apply`, `undo` and a rebuild; a layer-side ladder
  would be a second structure invalidated by hand. The fold on the wire is the LAYER's, built from the factored
  action source (`families`/`iter_family`/`family_len`), and `screen_model.rs` is not touched: the frame builder
  takes the folded list as it takes any list.
- **D-4 — "members on demand" (spec A3) is the guided retreat the wire already serves**: `PromptState::Retreat`
  asks per occupied area how many to recover, so the retreat family's members never need listing; no new route.
- **D-5 — the quota's refusal is a 507 (`Insufficient Storage`)**, which the outbox retries and now shows: a full
  archive is the server's state, not the record's, and an operator who makes room should see the row land on the
  next drain. The outbox's contract (`web/src/outbox.ts:171–185`) ends a row on 400/409 alone; a 403 is the edge's
  challenge and a 429 the zone's rate limit, so neither can carry the quota.
- **D-6 — the live corpus replay narrows to the production-door walk under R-SEC8(b).** `walk.json`'s `wire` and
  `wire2` are seeded creates beside an OPEN seat, which a production host answers 400 (`SEED_NOT_SOLO`); 2,180 of
  the walk's 2,201 rows are at those two tables and cannot run there by the owner's own rule. The replay now
  sends each such create and holds it to the 400 in `SEED_NOT_SOLO`'s words — the live probe of the rule — skips
  every row at those tables and the lobby pages that name them (counted and printed), and runs the rest: health,
  me, labels, the house chat and its delete, `wire4` (a solo hot seat, honoured), the identity refusals and the
  eight archive rows (the once-rule's first sends included). The whole walk stays verified against the module
  through the Worker's pool (`YOKOHAMA_DIAGNOSTIC_SEEDS`, both doors) and against the browser module through
  `corpus.node.test.ts`; the lobby's and a two-person table's live reading is the live Playwright tier's. **The
  alternative, recorded:** a diagnostic-seeds deploy for the window through a `--var` override, as the wake window
  does — refused here because the owner ruled the switch off in every deployable configuration and the window
  would honour a creator's seed at a multi-person production table for its minutes.
- **D-7 — the Worker's double replay per upload stays** (≤ 110 ms after A2; the Worker's archive object takes its
  quota in S2 by spec B12) and the unknown-controller-beside-a-human residual of D25 is recorded, not closed: the
  rebuild path tolerates it the same way (`bots::seat_disagreement`), and a second rule would be a second statement.
- **D-8 — S1a's residuals:** the `record` route's lock scope gains its parked-read pin here (Task 5, the file it
  touches); M-3 (a `why` refusal bypasses the visibility read) and the served `why` chart's jsdom-only pin go to
  S1c with D12, the refusal-body row.

## Spec corrections (to be recorded as §9.4 by Task 8, dated 2026-10-09)

1. **§2.1 A2, the timing cell.** "A timing cell over the cap-sized worst case asserting a bound" is a DEBUG-profile
   ceiling over a committed cap-sized spread fixture, measured first and set at five times the predicate's
   reading (D-1) — and it is the predicate wiring's ONLY catcher, since the two replays accept the same records
   by the core PR's proof. The release-profile reading over the review's 23,606-event body stays a measurement
   in the review record, never a cell.
2. **§2.1 A3's files and §6's CLI fence.** The CLI is touched in `crates/yokohama-cli/src/session.rs` alone — the
   checkpoint ladder and the predicate fold, behaviour-preserving (the states a session reaches are the ones
   `replay` reaches, by the core PR's proof; pinned by the CLI's suite and a new agreement cell) — and NOT in
   `screen_model.rs`: the fold is the layer's, from the factored action source (D-3). "The CLI is touched only at
   `suggest::stream_seed`" reads, for S1b, "…and in `session.rs`, nowhere the bot decides".
3. **§2.1 A3, "members on demand".** Served by the wire's existing guided retreat (`PromptState::Retreat`); no new
   route (D-4).
4. **§2.1 B12, the quota's refusal.** A 507 the outbox shows and retries (D-5); the per-caller counting is the
   NAS directory archive's, under `browser/<caller>/` alone — a server's own games are never counted.
5. **§5.3, the live corpus replay.** Narrowed to the production-door walk under R-SEC8(b) (D-6); the probe "a
   seeded create beside an open seat is a 400 on both hosts" is now the replay's own row.
6. **§2.1 D25's scope.** `check_pair_with` gains the record mirror (`deny_unknown_fields` over the engine record's
   eleven keys), the locked-down refusal, `validate_config` and `bots::seat_disagreement`; `archive_post` gains
   the foreign-bot refusal; `tables::check_shape` gains the name rule on `name`. Two residuals recorded: a record
   naming an unparseable controller beside a HUMAN seat is accepted, as the rebuild accepts it (D-7); and a
   table document's `created_by` or seat address naming another person is accepted — the archive attributes a
   file by the UPLOADER's folder (`browser/<caller>/`), and the names inside a document are data. (The plan's
   draft refused such a pair as forged attribution; the pre-flight read that rule refusing the committed corpus's
   own eight archive rows and the live replay's, which post `red@wire.test`'s hosted tables by design, so it was
   dropped — F7.)
7. **§2.1 A2, "every untrusted-record entry".** `check_pair_with` is the ONE entry (its two callers, the archive
   route and a restore); the resident rebuild, `/at` and `undo` fold the server's OWN records through the same
   predicate fold (Task 5) — not untrusted, the same cost.
8. **§2.1 D6's "compute routes".** Every dispatch leaves the executor (the blocking pool), and the GATE is on the
   five replay routes — `POST /api/archive`, `GET …/at`, `…/at/{k}`, `…/suggest`, `…/why` — never on `input`: a
   move must land while a burst of replays waits, and an undo is cheap once the ladder stands.
9. **§2.1 A2's Worker half.** The Worker's archive object still replays an upload twice (D-7): ≤ 110 ms after this
   cycle; its quota and the receipt loop's single dispatch are S2's.

## Global Constraints

- `SEMANTICS_REV` (`crates/yokohama-core/src/game/semantics.rs`) stays **7**; `crates/yokohama-core`,
  `crates/yokohama-oracle`, `crates/yokohama-difftest`, `crates/yokohama-bridge` and `crates/gen-data` are
  byte-identical to `604d7269` (`git diff 604d7269 -- crates/yokohama-core crates/yokohama-oracle
  crates/yokohama-difftest crates/yokohama-bridge crates/gen-data` empty at every commit); `crates/yokohama-worker`
  and `crates/yokohama-wasm` are untouched (the `Archive` trait's new method is defaulted, so the receipt archive
  compiles unchanged).
- The CLI is touched in ONE module, `crates/yokohama-cli/src/session.rs` (the checkpoint ladder, the predicate
  fold, `state_before`; its cells) — `decide`, `bot.rs`, `why.rs`, `suggest.rs`, `repl.rs`, `screen_model.rs`,
  the arena and every bot figure untouched; no user-visible CLI text moves, so the guide pair does not move and
  `cargo test -p yokohama-cli` reads 857 + Task 5's cells, 2 ignored.
- **Every knob carries the owner's comment** (the ruling above): `backup::EVENT_CAP`, `api::ARCHIVE_BODY_LIMIT`,
  `archive::QUOTA_RECORDS`/`QUOTA_BYTES`, `REPLAY_PERMITS`/`REPLAY_WAIT` in `lib.rs`, `client::OPTIONS_CAP`,
  `session::CHECKPOINT_EVERY` — transcribed from this plan's blocks, not shortened.
- No bot behaviour moves: the bot's `decide`, `suggest`'s answer, `why`'s reconstruction and every arena figure are
  untouched; the predicate fold reaches the same states as the checked one (the core PR's proof).
- The wire corpus (`web/src/wire/`) changes in exactly ONE task, Task 6 (`options_omitted` on every frame-bearing
  payload; the three fixtures that carry a retreat family — `retreat.json`, `retreat_answered.json`,
  `options_priced.json` — folded to one member); the regeneration is run ONCE for its commit, its diff READ and the
  reading recorded in the task report, naming every file that moved and why — a file beyond that list, or a
  numeric field that moved anywhere but `options_omitted`, is a finding (the M6.1 lesson, and S1a's: read a
  regeneration's diff for a FENCE moving, not only a leak).
- Every refusal this plan adds is a plain sentence, a constant or a builder in the module that raises it, and a
  400, 404, 503 or 507 through the layer's own `ApiError` arms — no new status shape; the 503 is the ADAPTER's,
  with its `Retry-After` header, since the gate is the NAS's alone.
- TDD: the failing cell first, then the code, then green; `cargo clippy --workspace --all-targets -- -D warnings`
  clean after every task; no `cargo fmt`; stage with explicit paths, never `git add -A`; one commit per task at
  its end (S1a's shape), the task's suites green at that commit; a reported SHA is never amended — every later
  change is a new commit.
- The branch is `security-s1`, S1a's, at `604d7269`; S1b's commits stack on it and on PR #49, whose merge stays
  undone (the owner, 2026-10-09: S1b and S1c stack on the same branch and PR; the merge is S1's wrap after S1c).
- Every task that touches what the layer compiles runs the lane's rows (`docs/web-lane.md` §2) whole before its
  commit — `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`; `cd web && npm run build:wasm
  && npm run typecheck && npm test -- --run && npm run test:wasm`; `cd worker && npm run build:module:debug && npm
  run typecheck && npm run test:scripts && npm test` — and records every count; Task 5 runs `cargo test -p
  yokohama-cli` too; Task 6 runs `npm run e2e` (the client's types moved).
- Every dispatched agent: the full report to a file under `docs/temp/security-review-2026-10-08/s1b-sdd/`
  (gitignored), a three-line reply; no `rm -rf`, `rm -r` or bulk `rm -f`; no Monitor, no event waits; long
  commands in the foreground or polled; reviewers run their mutants on a `git archive` export, never in the
  implementer's tree.

## Review Focus

Five inputs the spec implies and this plan pins, each to the task that owns the code:

1. **A legitimate long local game — a browser hot seat played past 4,096 events.** Refused at upload AND at
   restore with the count sentence, a 400 the outbox ENDS the row on, and the game stays on the device. Task 1's
   route cell reads the sentence and the status; its restore cell reads `Rejected` with the same sentence.
2. **A retry of a filed record once the caller's quota is full.** 200 `identical`, never 507: the marker read
   precedes the quota count. Task 3's archive cell lands two records under a two-record quota, refuses a third,
   and answers the first's bytes again `Identical`.
3. **A burst of `/at` runs from a watcher at a spread table while the acting seat moves.** The runs queue for two
   seconds and then read 503 with `Retry-After: 1`; the seat's `input` lands meanwhile, ungated. Task 4's gate
   cell holds both permits and posts an input.
4. **A finished game whose record is read while another request undoes it.** The record route's store read holds
   the handle's lock across the read, so a parked read is answered by the finished record or by `RECORD_UNSAVED`,
   never by a running record. Task 5's parked-read double.
5. **A Movement decision whose retreat family is the lone recover-nothing move** (no assistant on the board).
   Listed as it is, `options_omitted` 0 — a singleton is not folded. Task 6's agreement cell CONSTRUCTS it — the
   opening, its first phase ended and nothing placed — because placement precedes movement within a turn, so the
   opening's Movement decisions have the just-placed assistants on the board and a random walk all but never meets
   a lone retreat (the pre-flight's F25: the draft said "the opening's every Movement decision", and the walk met none).

## Execution shape

- Tasks 1–7: implementer **Opus**, reviewer **Opus**; Task 8: implementer **Sonnet**, reviewer **Opus**. Task 9:
  the orchestrator, **Mythos**, who conducts the "before" plan review (executing Tasks 1–7's cells against scratch
  implementations of this plan's code in an export, cutting the spread fixture, measuring every bound this plan
  states as "measured", reading every named mutant red) and the "after" whole-branch review itself.
- Every implementer dispatch carries the probe-and-name-concerns paragraph: try the mechanism on cases the brief's
  cells do not cover, with vacuity as the first lens — for each cell, which mutation makes it fail? — and report
  `DONE_WITH_CONCERNS` naming any sentence of this plan the code contradicts; a `DONE_WITH_CONCERNS` correctness
  item is ruled on and fixed as a new commit BEFORE the task's review. Every fix-round dispatch requires the hunks
  quoted and `git diff` run before the commit.
- The SDD ledger (`docs/temp/security-review-2026-10-08/s1b-sdd/progress.md`) records each task's tier, its
  commit, its review verdict and every `Ruling:` line, as S1a's did.

## Names shared across tasks

| name | where | meaning |
|---|---|---|
| `backup::EVENT_CAP: usize = 4_096` · `backup::event_cap_sentence(events: usize) -> String` | `crates/yokohama-web/src/backup.rs`, `pub` | the most events a record may carry to be archived or restored, and the sentence the count refuses with (Task 1) |
| `check_pair_with` replays through `yokohama_core::replay_strict_with(&rec, rules, MoveCheck::Predicate)` | `backup.rs:249` (and the no-rev arm, `:230`) | the predicate replay at the one untrusted-record entry (Task 1) |
| `backup::check_pair_filed(record, table, rules, filed: &dyn Fn(&TableDoc, &GameRecord) -> Option<bool>) -> Result<PairFacts, String>` · `PairFacts.held: bool` · `check_pair_with` = `check_pair_filed(.., &|_, _| None)` | `backup.rs`, `pub(crate)` | the one validator with the archive route's hook, asked once every cheap check has passed and BEFORE the replay: `Some(over)` is a record the archive already holds, accepted with `held: true` and no replay (Task 1) |
| `archive::Archive::holds(&self, prefix: &str, why: Why, id: &str, record_json: &str) -> bool` | `crates/yokohama-web/src/archive.rs`, a trait method with a default `false`; `DirArchive` reads its marker | whether the record at that path holds exactly these bytes — what `archive_post`'s `filed` hook asks, for both leaves, before the replay and after every cheap check (Task 1) |
| `api::ARCHIVE_BODY_LIMIT = 512 << 10` · the 413 sentence `"the record is larger than 512 KiB"` | `crates/yokohama-web/src/api.rs:2246`, `:2256` | the body cap (Task 1) |
| `backup::LOCKED_DOWN: &str` · `backup::RecordMirror` (private, `deny_unknown_fields` over the engine record's eleven keys) | `backup.rs` | D25's refusals: a locked-down record, an unknown key (Task 2) |
| `archive::QUOTA_RECORDS: usize = 256` · `archive::QUOTA_BYTES: u64 = 128 << 20` · `archive::Quota { records, bytes }` · `Quota::PRODUCTION` · `DirArchive::with_quota(self, Quota) -> Self` · `PutError::Quota { records: usize, bytes: u64 }` · `api::archive_full_sentence(records, bytes) -> String` | `archive.rs`, `api.rs`, `pub` | the per-caller quota, counted under `browser/<caller>/` before a put; the route's 507 (Task 3) |
| `api::is_replay_route(method: &str, path: &str) -> bool` · `api::REPLAY_ROUTES: [(&str, &str); 5]` | `api.rs`, `pub` | the five routes the NAS gates: `POST /api/archive`, `GET /api/games/{id}/at`, `…/at/{k}`, `…/suggest`, `…/why` (Task 4) |
| `REPLAY_PERMITS: usize = 2` · `REPLAY_WAIT: Duration = 2 s` · `AppState { replay_gate: Arc<tokio::sync::Semaphore>, replay_wait: Duration, .. }` · `AppState::with_replay_gate(self, permits: usize, wait: Duration) -> Self` · `BUSY: &str` | `crates/yokohama-web/src/lib.rs`, `pub` (server feature) | the gate, its builder for cells, the 503's sentence; every dispatch on `spawn_blocking` (Task 4) |
| `session::CHECKPOINT_EVERY: usize = 64` · `Session::fold_event(state: &GameState, ev: &RecordedEvent, rules: &Rules) -> Result<GameState, String>` · `Session::state_before(&mut self, k: usize) -> Result<GameState, String>` · the field `ladder: Vec<(usize, GameState)>` | `crates/yokohama-cli/src/session.rs`, `pub` | the predicate fold, the ladder, the replay-to-position every layer replay now takes (Task 5) |
| `client::HistoricalFrame::at(repl: &mut Repl, k, viewer) -> Result<Self, String>` · `HistoricalFrame::run(repl: &mut Repl, from, to, viewer) -> Result<Vec<Self>, String>` | `crates/yokohama-web/src/client.rs:695`, `:707` | the two replay projections, through `state_before` (Task 5); `api::run` and `api::at_k` take `let mut repl` |
| `client::OPTIONS_CAP: usize = 4_096` · `client::folded_legal(state, rules) -> (Vec<Move>, usize)` · `client::folded_legal_capped(state, rules, cap) -> (Vec<Move>, usize)` · `ClientState.options_omitted: usize` · `HistoricalFrame.options_omitted: usize` | `client.rs`, `pub(crate)` the functions, `pub` the fields | the folded, capped option list every frame is built from, and the count it leaves out (Task 6) |
| `SEED_NOT_SOLO` (JS) · `productionSeedRule(walk) -> Set<string>` · `skippedUnderProductionSeedRule(row, skipped) -> boolean` · `isProductionSeedCreate(row, skipped) -> boolean` · `result.skipped` | `worker/scripts/replay/exemptions.mjs`, `worker/scripts/live-replay.mjs` | the live replay's production-door rule (Task 7) |
| the fixture pair `crates/yokohama-web/tests/fixtures/spread-4p-4096.json` + `.table.json` + `.provenance.json` | committed by the orchestrator at the pre-flight (Task 9 Step 1), read by Tasks 1, 5 and 6 | the cap-sized spread record (4,096 events, seed 11, 4P, the retreat family 442,368 at seat 0's Movement from event 419 on — 235 such positions, the last at 4,083; the record ends in seat 3's second additional-action phase) and its four-seat hot-seat table, id `local-spread4096` |
| `tables::SEED_NOT_SOLO` | `crates/yokohama-web/src/tables.rs:756` | the sentence the JS constant is held to |
| `yokohama_core::{families, iter_family, family_len, retreat_len, MoveFamily, is_listed_move, apply_move_unchecked, replay_from, state_at, replay_strict_with, MoveCheck, validate_config, MoveError}` | the crate root re-exports (`pub use game::*`) | what the layer and the CLI reach for; none is edited |
| the route-cell helpers `live_router` (:2273), `call` (:2277), `a_game` (:2366), `request_as` (:2376), `post_as` (:2397), `get_as` (:2406), `start_as` (:2431), `play_turn` (:2509), `ENDED_SEED` (:3722), `a_finished_two_person_game` (:3736), `a_finished_vs_bot_game` (:3808), `WHO` (:1043) | `crates/yokohama-web/src/lib.rs` `mod tests` | what Tasks 3–6's route cells reuse |
| the api-test helpers `games_with(archive)` (:2758), `scratch(name)` (:2771), `post_archive` (:2794), `post_archive_bytes` (:2801), `PLAYED_RECORD`/`PLAYED_TABLE`/`PLAYED_ID`, `keyed`, `without`, `replay_broken` | `crates/yokohama-web/src/api.rs` `mod tests` | what Tasks 1–3's archive cells reuse |

## File structure

- Modify `crates/yokohama-web/src/backup.rs` (`check_pair_with` :197–258: the cap, the predicate; later D25's
  checks), `crates/yokohama-web/src/api.rs` (`ARCHIVE_BODY_LIMIT` :2246, `check_archive_size` :2252,
  `archive_post` :2315–2357 and its doc :2291–2314; the order cell :3127–3238), `crates/yokohama-web/src/archive.rs`
  (the `Archive` trait :111–131, `DirArchive` :360–403) — Task 1. Add the fixture pair (the pre-flight).
- Modify `backup.rs` (`check_pair_with` again), `api.rs` (`archive_post`), `crates/yokohama-web/src/tables.rs`
  (`check_shape` :378–467) — Task 2.
- Modify `archive.rs` (`PutError` :62–77, `DirArchive` :245–403), `api.rs` (`archive_post`'s put arms),
  `web/src/outbox.ts` (`reasonFor` :179–185) and `web/src/outbox.test.ts` — Task 3.
- Modify `crates/yokohama-web/src/lib.rs` (`AppState` :354–399, `with_store_chat_archive_and_log` :528–584, the
  builders :586–630, `events` :697–705, `adapter` :854–919), `api.rs` (beside `routes` :199) — Task 4.
- Modify `crates/yokohama-cli/src/session.rs` (`Session` :67–110, `from_record` :218–391, `apply` :532–625,
  `apply_standalone` :651–679, `undo` :717–762, `for_test_from_state` :816–829), `crates/yokohama-web/src/client.rs`
  (`HistoricalFrame::at`/`run` :695–728), `api.rs` (`run` :2044–2069, `at_k` :2110–2138; `record`'s cells in
  `lib.rs`) — Task 5.
- Modify `client.rs` (`ClientState` :134–194 and `::of` :486–557, `HistoricalFrame` :625–676 and `from_state`
  :733–800), `web/src/types.ts` (`ClientState` :325–351, `HistoricalFrame` :439–471), `web/src/wire/contract.ts`
  (`CLIENT_STATE` :617, `HISTORICAL_FRAME` :655); regenerate `web/src/wire/` ONCE — Task 6.
- Modify `worker/scripts/replay/exemptions.mjs`, `worker/scripts/live-replay.mjs` (the loop :137–217),
  `worker/scripts/replay/judge.mjs` (`finish` :150–174), `worker/scripts/live-replay.test.mjs` (`fakeHost`
  :366–386, the driver cells :406–678) — Task 7.
- Modify `docs/web-lane.md` (§2, §3, §4, §13, §15), `docs/plans/2026-10-08-security-remediation-spec.md` (§9.4
  appended), `CLAUDE.md` (one parenthetical), `deploy/README.md` (:192, :314), `worker/RUNBOOK.md` (the live
  replay's section) — Task 8; the ledger, guidingpath, the board and the review record at the close (Task 9).

## Task order and why

Task 1 first: it owns the one validator and lands the fixture every later cost cell reads. Task 2 edits the same
function next, so the order cell is extended once more in one place. Task 3 (the quota) and Task 4 (the gate) are
independent of each other and of Tasks 5–6. Task 5 (the ladder) precedes Task 6 (the fold) because Task 6's spread
cells rebuild the fixture's game and run `/at` over it, which costs seconds until the ladder and the predicate
fold stand. Task 7 (the live replay) touches no Rust and could run beside any of them; it waits for Task 6 so one
corpus regeneration precedes it. Task 8 writes the documents the code changed. Task 9 is the orchestrator's frame.

---

### Task 1: A2 at the one validator — the event cap, the predicate replay, the body cap, a retry answered before any replay, D29's sentence — Opus (A2, B12's cap, D29; R-SEC2, R-SEC8(f); Review Focus 1)

**Files:**
- Modify: `crates/yokohama-web/src/backup.rs` (`check_pair_with` :197–258 — the count check after the parse, the
  predicate at :230 and :249; two constants and a sentence builder above it; cells in `mod tests` :442)
- Modify: `crates/yokohama-web/src/api.rs` (`ARCHIVE_BODY_LIMIT` :2237–2246 and `check_archive_size` :2248–2262;
  `archive_post` :2291–2357, its doc and the early answer; the order cell :3127–3238; cells beside it)
- Modify: `crates/yokohama-web/src/archive.rs` (the `Archive` trait :111–131 — `holds`; `DirArchive`'s impl
  :360–403; cells in `mod tests`)
- Read: `crates/yokohama-web/tests/fixtures/spread-4p-4096.json`, `.table.json`, `.provenance.json` (committed by
  the pre-flight, Task 9 Step 1)
- Test: `backup.rs`, `api.rs` and `archive.rs`'s `mod tests`

**Interfaces:**
- Consumes: `yokohama_core::{replay_strict_with, MoveCheck, retreat_len}`; the fixture pair; the api-test helpers
  `games_with`, `scratch`, `post_archive`, `post_archive_bytes`, `keyed`, `without`, `replay_broken`,
  `PLAYED_RECORD`, `PLAYED_TABLE`, `PLAYED_ID`; `backup::tests::envelope_of`.
- Produces: `backup::EVENT_CAP`, `backup::event_cap_sentence`; `Archive::holds` (default `false`; `DirArchive`
  reads its marker); `api::ARCHIVE_BODY_LIMIT = 512 << 10` and its sentence; `archive_post`'s early `identical`.

- [ ] **Step 1: The failing cells.**

(a) In `backup.rs`'s `mod tests`, after `the_table_is_checked_before_the_record_is_replayed` (:550):

```rust
    /// The cap-sized spread record the pre-flight cut from the review's
    /// worst-case body: 4,096 events of a four-seat hot seat, seed 11, every
    /// seat spread across the board and never recovering, then passing turns
    /// — seat 0's Movement decision offers 442,368 retreats from event 419 on,
    /// at 235 of the record's positions (the last at 4,083).
    const SPREAD_RECORD: &str = include_str!("../tests/fixtures/spread-4p-4096.json");
    const SPREAD_TABLE: &str = include_str!("../tests/fixtures/spread-4p-4096.table.json");

    /// The DEBUG-profile ceiling the predicate replay of [`SPREAD_RECORD`] is
    /// held under, in seconds — five times the pre-flight's reading on the Mac
    /// (0.55 s alone, 0.35 s inside the parallel lib run; the plan's §"Review
    /// record"), rounded up. **It is the predicate wiring's only catcher**
    /// (R-SEC2; the plan's D-1): the two strict replays accept the same records
    /// by the core PR's proof, so no functional cell can tell them apart — but
    /// 235 of the record's positions, seat 0's Movement decisions, each
    /// materialize the 442,368-move family under `Listed`, which took 198 s in
    /// debug (measured), and fold in under a second under `Predicate`.
    const SPREAD_REPLAY_BOUND_SECS: u64 = 3;

    /// **A record over the cap is refused by its COUNT, before anything is
    /// replayed** (finding A2; R-SEC8(f)). The probe carries TWO defects — one
    /// event over `EVENT_CAP`, and that event a placement at a phase that
    /// cannot take one, so the record would not replay — and reads the
    /// count's sentence: the two-defect shape that pins an ORDER (the M6.2
    /// lesson). At the cap exactly, the same record is accepted, so the bound
    /// is inclusive and not one below.
    #[test]
    fn a_record_over_the_event_cap_is_refused_by_count_before_any_replay() {
        let mut v: serde_json::Value = serde_json::from_str(SPREAD_RECORD).expect("a record");
        let events = v["events"].as_array_mut().expect("events");
        assert_eq!(events.len(), EVENT_CAP, "the premise: the fixture sits exactly at the cap");
        // An event legal at NO position — a President placed on an area the
        // board does not have (the review's `PlacePresident 255` probe, which
        // panics nothing) — so the record would also fail its replay.
        let illegal = serde_json::json!({ "Move": { "PlacePresident": { "destination": 255 } } });
        events.push(illegal.clone());
        let over = v.to_string();
        let said = check_pair(&over, SPREAD_TABLE).expect_err("one over the cap");
        assert_eq!(said, event_cap_sentence(EVENT_CAP + 1));
        assert!(said.contains("4097") && said.contains("4096"), "{said}");
        // The appended event really would fail a replay — in the LAST slot of
        // a record at the cap it is refused at event 4095 — so above, the
        // count spoke first.
        let mut same_len: serde_json::Value = serde_json::from_str(SPREAD_RECORD).expect("a record");
        let ev = same_len["events"].as_array_mut().expect("events");
        *ev.last_mut().expect("an event") = illegal;
        let said = check_pair(&same_len.to_string(), SPREAD_TABLE).expect_err("the premise: that event does not replay");
        assert!(said.starts_with("the record does not replay: at event 4095"), "{said}");
    }

    /// **The cap-sized worst case replays under the predicate in bounded time**
    /// (finding A2; R-SEC2) — the catcher for the layer replaying through
    /// `replay_strict` or `MoveCheck::Listed` (the plan's D-1). The record is
    /// read whole: 4,096 events accepted, not over, and its replayed state
    /// really is the spread state — some seat's retreat family is six figures,
    /// so a re-cut fixture that never spread cannot pass by being cheap.
    #[test]
    fn the_cap_sized_spread_record_replays_under_the_predicate_in_bounded_time() {
        let started = std::time::Instant::now();
        let facts = check_pair(SPREAD_RECORD, SPREAD_TABLE).expect("the fixture is a valid pair at the cap");
        let took = started.elapsed();
        assert_eq!((facts.players, facts.events, facts.over), (4, EVENT_CAP, false));
        assert!(
            took.as_secs() < SPREAD_REPLAY_BOUND_SECS,
            "the predicate replay took {took:?} — above the bound; a materialized replay is minutes",
        );
        let rec: yokohama_core::GameRecord = serde_json::from_str(SPREAD_RECORD).expect("a record");
        let rules = yokohama_core::Rules::standard();
        let state = yokohama_core::replay_strict_with(&rec, &rules, yokohama_core::MoveCheck::Predicate)
            .expect("the fixture replays");
        let widest = state.players.iter().map(|p| yokohama_core::retreat_len(&p.assistants.on_board)).max().unwrap_or(0);
        assert!(widest >= 100_000, "anti-vacuity: the fixture's end state is not the spread state (widest family {widest})");
    }

    /// A restore refuses the same record with the same sentence — the one
    /// validator, two callers (R-6.5b2.10): a backup over the cap is REJECTED,
    /// never written, and the row says why in the archive's own words.
    #[test]
    fn a_restore_refuses_a_record_over_the_event_cap_with_the_archives_sentence() {
        let mut v: serde_json::Value = serde_json::from_str(SPREAD_RECORD).expect("a record");
        let events = v["events"].as_array_mut().expect("events");
        events.push(serde_json::json!({ "Move": { "PlacePresident": { "destination": 255 } } }));
        let env = envelope_of(&[("local-spread4096", &v.to_string(), SPREAD_TABLE)]);
        let report = restore_read(&env).expect("the envelope is well formed");
        match &report[0].1 {
            Verdict::Rejected { reason } => assert_eq!(reason, &event_cap_sentence(EVENT_CAP + 1)),
            other => panic!("a record over the cap is rejected: {other:?}"),
        }
    }
```

(b) In `archive.rs`'s `mod tests`, after `no_archive_is_off_and_never_takes_a_put` (:679):

```rust
    /// **`holds` reads the marker and nothing else** (finding A2): true for the
    /// bytes a put landed at exactly that path, false for other bytes, another
    /// id, the other leaf, and an archive that keeps nothing — the default
    /// every implementation that cannot read its own files synchronously
    /// inherits, the Worker's receipt archive among them.
    #[cfg(feature = "server")]
    #[test]
    fn holds_is_true_for_the_landed_bytes_at_exactly_their_path() {
        let dir = std::env::temp_dir().join(format!("yokohama_archive_holds_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = DirArchive::new(&dir);
        assert!(!a.holds("browser/a_at_b/", Why::Over, "local-1", "{\"r\":1}"), "nothing landed yet");
        a.put("browser/a_at_b/", Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}").unwrap();
        assert!(a.holds("browser/a_at_b/", Why::Over, "local-1", "{\"r\":1}"));
        assert!(!a.holds("browser/a_at_b/", Why::Over, "local-1", "{\"r\":2}"), "other bytes");
        assert!(!a.holds("browser/a_at_b/", Why::Over, "local-2", "{\"r\":1}"), "another id");
        assert!(!a.holds("browser/a_at_b/", Why::Abandoned, "local-1", "{\"r\":1}"), "the other leaf");
        assert!(!a.holds("browser/c_at_d/", Why::Over, "local-1", "{\"r\":1}"), "another caller");
        assert!(!NoArchive.holds("", Why::Over, "local-1", "{\"r\":1}"), "the default is false");
        std::fs::remove_dir_all(&dir).ok();
    }
```

(c) In `api.rs`'s `mod tests`, after the order cell (:3238):

```rust
    /// An archive that claims to hold EVERYTHING — the instrument that reads
    /// where the retry's answer sits: a record that could not replay is
    /// answered `identical` through it only if the question was asked before
    /// the replay.
    struct HoldsEverything;

    impl crate::archive::Archive for HoldsEverything {
        fn put(
            &self,
            _prefix: &str,
            _why: crate::archive::Why,
            _id: &str,
            _record_json: &str,
            _table_json: &str,
        ) -> Result<crate::archive::PutOutcome, crate::archive::PutError> {
            unreachable!("a held record is answered before the put")
        }
        fn kind(&self) -> &'static str {
            "dir"
        }
        fn holds(&self, _prefix: &str, _why: crate::archive::Why, _id: &str, _record_json: &str) -> bool {
            true
        }
    }

    /// **A retry of a filed record is answered before any replay** (finding
    /// A2): once every cheap check has passed, the archive is asked whether
    /// either leaf already holds these very bytes under this caller, and a
    /// `true` is the 200 `identical` the put's once-rule gives — with no replay
    /// paid. The proof: a record that does NOT replay reads 200 `identical`
    /// through an archive that claims it, where the same pair on a real, empty
    /// archive reads the replay's 400 — so the answer came before the replay.
    /// And the cheap checks still speak first: the claiming archive beside a
    /// table that disagrees reads the agreement's 400, which is what keeps the
    /// committed corpus's `archive_400_table` row a 400 on every host, the
    /// Worker's included.
    #[test]
    fn a_retry_of_a_filed_record_is_answered_identical_before_any_replay() {
        let claims = games_with(std::sync::Arc::new(HoldsEverything));
        let dir = scratch("archive_retry");
        let real = games_with(std::sync::Arc::new(crate::archive::DirArchive::new(&dir)));
        let broken = replay_broken(PLAYED_RECORD);
        let (status, v) = post_archive(&real, &broken, PLAYED_TABLE);
        assert_eq!(status, 400, "the control, on a real archive holding nothing: {v}");
        assert!(v["error"].as_str().unwrap_or("").starts_with("the record does not replay"), "{v}");
        let (status, v) = post_archive(&claims, &broken, PLAYED_TABLE);
        assert_eq!((status, v["identical"].as_bool()), (200, Some(true)), "held, so never replayed: {v}");
        assert_eq!(v["leaf"], "over");
        assert_eq!(v["path"], format!("browser/local_at_browser/rev7/over/{PLAYED_ID}.json"));
        let disagreeing = keyed(PLAYED_TABLE, "start_seat", 1.into());
        let (status, v) = post_archive(&claims, &broken, &disagreeing);
        assert_eq!(status, 400, "the table's checks come first: {v}");
        assert!(v["error"].as_str().unwrap_or("").starts_with("the table document does not agree"), "{v}");
        // A real archive's retry of landed bytes is the same 200, through
        // the same hook (`DirArchive::holds` reads the marker).
        let (status, _) = post_archive(&real, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!(status, 201);
        let (status, v) = post_archive(&real, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!((status, v["identical"].as_bool()), (200, Some(true)), "{v}");
        std::fs::remove_dir_all(&dir).ok();
    }
```

(d) In the order cell `the_archive_route_keeps_its_statuses_sentences_and_refusal_order` (:3127): the 413's
sentence (:3154) becomes `"the record is larger than 512 KiB"`; the probes gain the cap's two pairs and the count
reads nine. Replace the `probes` array's first two rows after the archive-off row (:3178–3186) with:

```rust
            // archive-off  <  the record parses
            (&off, "not json at all".to_string(), PLAYED_TABLE.to_string(), "this server keeps no archive"),
            // the record parses  <  the count is within the cap
            (&on, keyed(&over_cap, "seed", "not a number".into()), PLAYED_TABLE.to_string(), "the record does not parse"),
            // the count is within the cap  <  the rev agrees
            (&on, keyed(&over_cap, "semantics_rev", 6.into()), PLAYED_TABLE.to_string(), "the record carries 4097 events"),
            // the rev agrees  <  a terminal digest is present
            (&on, without(&rev_wrong, "terminal_digest"), PLAYED_TABLE.to_string(),
             "this record is at semantics rev 6; this server archives rev 7"),
```

with, above the array, the over-cap record built from the spread fixture (the fixture's `include_str!`s are
`backup.rs`'s; add `SPREAD_RECORD` alone to `api.rs`'s `mod tests` — `SPREAD_TABLE` is read by nothing there, and
the lint gate refuses a dead constant):

```rust
        let over_cap = {
            let mut v: serde_json::Value = serde_json::from_str(SPREAD_RECORD).expect("a record");
            let events = v["events"].as_array_mut().expect("events");
            events.push(serde_json::json!({ "Move": { "PlacePresident": { "destination": 255 } } }));
            v.to_string()
        };
```

the array's type `[(&Games, String, String, &str); 9]`, and `assert_eq!(sentences.len(), 9, "nine checks, nine
different sentences")`; the comments "The nine checks, in order" and "The ninth, on its own" read "The checks, in
order" and "The last, on its own" (counts that move twice). The 413 row at :3152–3156 is re-sized at the LITERAL
and pins the constant — `assert_eq!(ARCHIVE_BODY_LIMIT, 512 << 10, "the cap R-SEC8(f) set (security S1b)");` above
it, the probe `vec![b'x'; (512 << 10) + 1]`, the at-the-cap slice `&over[..512 << 10]` — because a probe sized
from the constant lets M4 move it unseen (the pre-flight read M4 green that way); its sentence changes too.

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --lib -- event_cap spread_record
holds_is_true retry_of_a_filed refusal_order 2>&1 | grep -E '^error|test result|panicked' | head` — `EVENT_CAP`,
`event_cap_sentence` and `holds` are not found (one compile error blocks the lib); stub them (`EVENT_CAP`
`usize::MAX`, the builder any sentence, `holds` returning `false` on the trait) to READ: (a) RED on the sentence
(the replay's where the count's is wanted); (b) RED on the bound — **measured in the pre-flight: the `Listed`
replay of the fixture in debug took 198 s** (the review record; the predicate's 0.55 s alone, 0.35 s in the
parallel lib run); (c) RED (`Accepted`); the retry cell RED (400 where
200); the order cell RED on the 413 sentence.

- [ ] **Step 3: The code.** In `backup.rs`, above `check_pair` (:157):

```rust
/// The most events a record may carry to be archived or restored (security
/// S1b, finding A2; R-SEC8(f)) — a PLAUSIBILITY bound, set from real games.
/// Every finished game people or the bot have played here is 250–600 events
/// (the longest measured, the browser-played fixture, 595), so 4,096 is seven
/// times that. Random play and hand-built records run to 15,000–24,000
/// events, and the archive is a collection of played games, not an import
/// path for worst-case records: a record over the bound is refused by its
/// COUNT before anything is replayed, so what an upload can cost no longer
/// follows what a record can hold.
pub const EVENT_CAP: usize = 4_096;

/// The sentence the count refuses with — the archive route's and a restore's
/// alike, since both go through [`check_pair`].
pub fn event_cap_sentence(events: usize) -> String {
    format!("the record carries {events} events — a finished game here is at most {EVENT_CAP}")
}
```

In `check_pair_with` (:202–258), after the parse and before the rev match:

```rust
    let rec: yokohama_core::GameRecord =
        serde_json::from_str(record).map_err(|e| format!("the record does not parse: {e}"))?;
    // **The count, before anything else reads the record** (finding A2): the
    // cheapest fact about it and the one that bounds every later cost.
    if rec.events.len() > EVENT_CAP {
        return Err(event_cap_sentence(rec.events.len()));
    }
```

and both replays through the predicate — the no-rev arm (:230):

```rust
        None => {
            return Err(match yokohama_core::replay_strict_with(&rec, rules, yokohama_core::MoveCheck::Predicate) {
                Err(e) => e.message,
                Ok(_) => "the record carries no semantics provenance".to_string(),
            })
        }
```

and the last check (:249):

```rust
    // LAST, behind a table that agrees with the record (the doc above) — and
    // through the PREDICATE check (security S1b, finding A2; R-SEC2): the same
    // accepted set and the same states as `replay_strict`, proven by
    // `crates/yokohama-core/tests/replay_check_equivalence.rs`, without the
    // legal list materialized per event, so an admitted record costs tens of
    // milliseconds whatever its families. This is the ONE replay an untrusted
    // record gets — the archive route's and a restore's.
    let state = yokohama_core::replay_strict_with(&rec, rules, yokohama_core::MoveCheck::Predicate).map_err(|e| {
        format!("the record does not replay: at event {}, {}", e.event_index, e.message)
    })?;
```

`PairFacts` (:138) gains the hook's flag, after `semantics_rev`:

```rust
    /// `true` where the caller's `filed` hook said the archive already holds
    /// these very bytes under the leaf `over` names, so the replay was never
    /// paid (security S1b, finding A2) — the archive route's 200 `identical`.
    /// Always `false` on a restore, which holds nothing.
    pub held: bool,
```

`check_pair_with` (:254) becomes a wrapper and the body moves to `check_pair_filed`, with the archive route's hook
(security S1b, finding A2 — the spec's "answer identical by comparing stored bytes BEFORE REPLAYING", placed after
the cheap checks so a table that fails them reads its own refusal on every host; the pre-flight's F15):

```rust
pub(crate) fn check_pair_with(
    record: &str,
    table: &str,
    rules: &yokohama_core::Rules,
) -> Result<PairFacts, String> {
    check_pair_filed(record, table, rules, &|_, _| None)
}

/// [`check_pair_with`] with the archive route's one addition (security S1b,
/// finding A2): `filed(&doc, &rec)` is asked ONCE EVERY CHEAP CHECK HAS PASSED
/// and before the replay — `Some(over)` says the archive already holds these
/// very bytes under the leaf `over` names, and the pair is accepted with
/// `held: true` and no replay paid; `None` replays as always. A restore passes
/// a hook answering `None`. Asked after the table's checks and not before
/// them, so a table that fails its checks reads its own refusal whether or not
/// the record is filed — which is what keeps every host answering a row
/// alike: the NAS's archive can read its files and the Worker's cannot.
pub(crate) fn check_pair_filed(
    record: &str,
    table: &str,
    rules: &yokohama_core::Rules,
    filed: &dyn Fn(&crate::table::TableDoc, &yokohama_core::GameRecord) -> Option<bool>,
) -> Result<PairFacts, String> {
```

and the hook is asked LAST before the replay, after the `is_safe_id` refusal (after Task 2's two checks, once they
land):

```rust
    // **A record the archive already holds is not replayed** (security S1b,
    // finding A2): asked here, LAST before the replay, so a retry of a filed
    // record costs the cheap checks and a read.
    if let Some(over) = filed(&doc, &rec) {
        return Ok(PairFacts {
            players: doc.players,
            events: rec.events.len(),
            over,
            semantics_rev: yokohama_core::SEMANTICS_REV,
            held: true,
        });
    }
```

with the `Ok` literal at the end gaining `held: false,`.

`check_pair`'s doc (:157–189) names the checks "in `archive_post`'s own order"; after this task its list
reads: "the record parsing, its event count within [`EVENT_CAP`], a `semantics_rev` present and equal to this
build's, a terminal digest present, the table document parsing, passing [`tables::check_shape`] and AGREEING
with the record, the document's id being `is_safe_id`'s, because there it becomes a path component, and LAST
`replay_strict_with(.., Predicate)` clean."

In `archive.rs`, the trait (:111–131) gains, after `put`:

```rust
    /// Whether the record at `prefix/rev/why/id` holds exactly `record_json`
    /// (security S1b, finding A2) — asked by the archive route BEFORE the
    /// replay, so a retry of a filed record costs a read and no replay.
    /// `false` by default: an implementation that cannot read its own files
    /// synchronously (the Worker's receipt archive) replays as before and
    /// meets the once-rule's `Identical` at the put instead.
    fn holds(&self, _prefix: &str, _why: Why, _id: &str, _record_json: &str) -> bool {
        false
    }
```

and `DirArchive`'s impl (:360–403), after `put`:

```rust
        fn holds(&self, prefix: &str, why: Why, id: &str, record_json: &str) -> bool {
            let Ok((record, _)) = paths(yokohama_core::SEMANTICS_REV, prefix, why, id) else { return false };
            if !Path::new(&record).components().all(|c| matches!(c, Component::Normal(_))) {
                return false;
            }
            matches!(self.read_marker(&record, record_json), Ok(Some(PutOutcome::Identical)))
        }
```

In `api.rs`, the body cap (:2237–2262):

```rust
/// The largest body `POST /api/archive` reads (security S1b, finding B12;
/// R-SEC8(f)): 512 KiB, down from M6.5c2 cycle D's 1 MiB. Set from real
/// games: a finished game pretty-printed, which is what a browser sends, is
/// under 60 KB (the longest measured, the browser-played fixture, 54 KB), so
/// this is eight times that — and above 5,000 pretty-printed events, so the
/// event count ([`crate::backup::EVENT_CAP`]) speaks first on every real
/// record. Random play runs to 1.9 MB and the archive is not an import path
/// for it. Over the cap the route answers 413 BEFORE it parses anything
/// (`check_archive_size`), under both hosts' body limits — the NAS adapter's
/// 2 MiB and the Worker's `BODY_LIMIT`.
pub const ARCHIVE_BODY_LIMIT: usize = 512 << 10;

/// The archive route's size check: over [`ARCHIVE_BODY_LIMIT`] is a 413 that
/// names the bound, answered after the caller and before the body is parsed —
/// pinned first in
/// `tests::the_archive_route_keeps_its_statuses_sentences_and_refusal_order`.
fn check_archive_size(body: &[u8]) -> Result<(), ApiError> {
    if body.len() > ARCHIVE_BODY_LIMIT {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            message: "the record is larger than 512 KiB".to_string(),
            table_status: None,
            foreign_bot: None,
        });
    }
    Ok(())
}
```

`archive_post`'s doc (:2291–2314) — D29: the checks in the ORDER the code runs them, the count among them:

```rust
/// `POST /api/archive` (M6.5b, R-6.5b.6) — a game played in a browser, filed by
/// what a strict replay shows it to be.
///
/// **The NINE checks `backup::check_pair_with` makes, in the order it makes
/// them** (ten refusals through this route, because the archive-off 404 is
/// asked first, and a rev that disagrees and a rev that is absent are two
/// different sentences): the record parses; its event count is within
/// [`crate::backup::EVENT_CAP`] (security S1b); a `semantics_rev` present and
/// equal to this build's; a terminal digest present; the table document
/// parses; it passes [`tables::check_shape`]; it AGREES with the record; its
/// id is `is_safe_id`'s, because it becomes a path component; and LAST the
/// replay, `replay_strict_with(.., Predicate)` clean — behind a table that
/// agrees, so an untrusted record reaches setup only beside one (M6.5c2 cycle
/// D, straw poll row 81; this paragraph listed the replay before the table's
/// checks until security S1b, finding D29). **The count is stated here once**
/// — an earlier draft said four here, four again at the call below and a
/// third count in an earlier document (whole-branch review, false sentence
/// 4); the order is the one
/// `tests::the_archive_route_keeps_its_statuses_sentences_and_refusal_order`
/// probes pairwise. **Between the last cheap check and the replay, a retry of
/// a record already filed is answered** (security S1b, finding A2): the two
/// leaves are asked through [`Archive::holds`](crate::archive::Archive::holds)
/// whether they hold these very bytes under this caller — a `true` is the 200
/// `identical` the put's once-rule would give, with no replay paid — and a
/// table that fails its checks reads its own refusal first, as on every host,
/// so the NAS, whose archive reads its files, and the Worker, whose cannot,
/// answer every row alike (the corpus's `archive_400_table` among them). Then **the
/// leaf IS the `Why`** — `Over` iff the replayed state is over, else
/// `Abandoned` — and one put under
/// `browser/<`[`caller_segment`](crate::archive::caller_segment)`>/` with the
/// table RE-STRIPPED server-side from the parsed document, as `delete` does, so
/// a crafted body carrying a lock or a watcher roster never lands verbatim. The
/// record string is put exactly as sent, because its bytes are the digest's.
///
/// 404 where this host keeps no archive — which under `NoArchive`, the
/// browser's own host, is the answer by construction.
```

and `archive_post` passes the archive's `holds` through the validator's hook, in place of its
`check_pair_with` call (the earlier `let prefix` below moves up here):

```rust
    // **A retry of a filed record is answered before any replay** (security S1b,
    // finding A2; the doc above): once every cheap check has passed, the archive
    // is asked whether either leaf already holds these very bytes under this
    // caller — through the validator's `filed` hook, so the question sits
    // exactly before the replay and after nothing else.
    let prefix = format!("browser/{}/", caller_segment(user));
    let filed = |doc: &crate::table::TableDoc, _rec: &yokohama_core::GameRecord| {
        [Why::Over, Why::Abandoned]
            .into_iter()
            .find(|why| games.archive.holds(&prefix, *why, &doc.id, &req.record))
            .map(|why| why == Why::Over)
    };
    let facts = crate::backup::check_pair_filed(&req.record, &req.table, games.rules(), &filed)
        .map_err(ApiError::bad_request)?;
```

and answers a held record once the path is known, before the put:

```rust
    if facts.held {
        return Ok(ArchiveReply { leaf: why.dir(), path, identical: true });
    }
```

The call-site comment "**The eight checks above are `backup::check_pair`'s since M6.5b2**" drops its count —
"The checks above are…" — the count is stated once, in the doc.

- [ ] **Step 4: Run, mutants, lint, commit.** `cargo test -p yokohama-web --lib 2>&1 | grep -E 'test result|FAILED'`
green (431: 426 + the five cells; the order cell edited in place; `lib.rs`'s
`the_archive_route_checks_and_files_a_browser_game` keeps its agreement row, a 400 under the hook's placement). Mutants: **M1** — the count check deleted: (a) RED
(the replay's sentence), the restore cell RED (`Accepted`), the order cell RED on the cap's rows; **M2** —
`MoveCheck::Predicate` → `MoveCheck::Listed` at :249: (b) RED at the bound (the pre-flight's `Listed` reading: 198 s
in debug, the whole lib 198.5 s against 86 s — run it ONCE and record it; the reviewer reads the mutant on an export
with a `timeout`); **M3** — the hook never asked (`filed` ignored): the retry cell RED (the replay's 400 where
200); **M4** — `ARCHIVE_BODY_LIMIT` back to `1 << 20`: the order cell RED at its pin. `cargo clippy -p yokohama-web --all-targets -- -D warnings`
clean; the lane's rows whole (Global Constraints) — the corpus does not move in this task (the archive walk's
records are 54 KB and 595 events; `archive_post.json`'s reply is unchanged).

```bash
git add crates/yokohama-web/src/backup.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/archive.rs
git commit -m "web: an untrusted record is bounded by its event count and replayed through the predicate; the archive's body cap is 512 KiB, a retry of a filed record is answered before any replay, and archive_post's doc lists the checks in the order they run (A2, B12, D29; S1b Task 1)"
```

---

### Task 2: D25 — the archive refuses what the engine would — Opus (D25; spec correction 6)

**Files:**
- Modify: `crates/yokohama-web/src/backup.rs` (`check_pair_with` — Task 1's shape: the record mirror at the parse,
  the locked-down refusal after the digest, `validate_config` and `seat_disagreement` after the id; two
  constants and the mirror above it; cells in `mod tests`)
- Modify: `crates/yokohama-web/src/api.rs` (`archive_post` — the foreign-bot refusal after the checks; the order
  cell's three new rows; a cell beside Task 1's)
- Modify: `crates/yokohama-web/src/tables.rs` (`check_shape` :421–427 — the name rule on `doc.name`; a cell in
  `mod tests`)
- Test: the three files' `mod tests`

**Interfaces:**
- Consumes: Task 1's `check_pair_filed` (the count first, the hook and the predicate last);
  `yokohama_core::validate_config`; `crate::bots::{seat_disagreement, foreign, distinguishing_labels,
  production}`; `tables::check_name`; the api-test helpers.
- Produces: `backup::LOCKED_DOWN: &str`; `backup::RecordMirror` (private); the twelve-check order.

- [ ] **Step 1: The failing cells.**

(a) In `backup.rs`'s `mod tests`, after Task 1's three cells:

```rust
    /// **A key the engine's record does not have is refused** (finding D25):
    /// `GameRecord` stays lenient — it is the engine's type, and a reader of
    /// saves must not refuse a newer field — so the archive reads the text
    /// through a mirror that denies unknown keys FIRST. The two committed
    /// pairs pass it (the anti-drift arm: a field core adds to `GameRecord`
    /// fails here until the mirror names it), and a planted key reads as a
    /// record that does not parse, naming the key.
    #[test]
    fn a_record_with_a_key_the_engine_does_not_write_is_refused_and_the_committed_pairs_pass() {
        assert!(check_pair(PLAYED_RECORD, PLAYED_TABLE).is_ok());
        assert!(check_pair(SPREAD_RECORD, SPREAD_TABLE).is_ok());
        let planted = PLAYED_RECORD.replacen("{", "{\"planted_by_an_uploader\": 1,", 1);
        assert_ne!(planted, PLAYED_RECORD, "the mutation must actually change the fixture");
        let said = check_pair(&planted, PLAYED_TABLE).expect_err("an unknown key");
        assert!(said.starts_with("the record does not parse") && said.contains("planted_by_an_uploader"), "{said}");
    }

    /// **A locked-down record is not a game** (finding D25): the driver
    /// declared it faulty, and the archive is a collection of played games.
    /// Two defects — the lockdown and a table that does not parse — read the
    /// lockdown's sentence, so the refusal sits before the table's checks.
    #[test]
    fn a_locked_down_record_is_refused_before_the_table_is_read() {
        let locked = keyed(PLAYED_RECORD, "lockdown", "a desync the driver found".into());
        assert_eq!(check_pair(&locked, "not json at all").expect_err("locked down"), LOCKED_DOWN);
        assert_eq!(check_pair(&locked, PLAYED_TABLE).expect_err("locked down"), LOCKED_DOWN);
    }

    /// **A configuration the engine would refuse is refused before setup**
    /// (finding D25): `validate_config`'s own sentence, carried — the check
    /// `Session::from_record` makes before it builds anything, and which the
    /// archive never made. Two defects: the impossible config (Port B and Lab
    /// B at two players) also moves the board, so the record would fail its
    /// replay at the digest; the config's sentence comes first.
    #[test]
    fn an_impossible_configuration_is_refused_before_the_replay() {
        let mut v: serde_json::Value = serde_json::from_str(PLAYED_RECORD).expect("a record");
        v["config"]["has_port_b"] = true.into();
        v["config"]["has_lab_b"] = true.into();
        let impossible = v.to_string();
        let said = check_pair(&impossible, PLAYED_TABLE).expect_err("an impossible configuration");
        assert!(said.starts_with("the record's configuration is impossible: "), "{said}");
        assert!(!said.starts_with("the record does not replay"), "the config spoke before the replay: {said}");
    }

    /// **A table whose seats disagree with the record's bots is refused**
    /// (finding D25) — `bots::seat_disagreement`, the rule the resident
    /// rebuild applies to a pair already in a store, now applied to one
    /// arriving. The record's provenance removed beside a table seating a
    /// bot: "names no controller". Two defects with a record that would not
    /// replay: the seats' sentence comes first.
    #[test]
    fn a_table_whose_seats_disagree_with_the_records_bots_is_refused_before_the_replay() {
        let no_run = without(PLAYED_RECORD, "run");
        let said = check_pair(&no_run, PLAYED_TABLE).expect_err("the table seats a bot the record does not name");
        assert!(said.starts_with("the table's seats disagree with the record's bots: seat 1 is a bot at the table"), "{said}");
        let mut v: serde_json::Value = serde_json::from_str(&no_run).expect("a record");
        let events = v["events"].as_array_mut().expect("events");
        events[0] = events[1].clone();
        let said = check_pair(&v.to_string(), PLAYED_TABLE).expect_err("two defects");
        assert!(said.starts_with("the table's seats disagree"), "the seats spoke before the replay: {said}");
    }
```

(`keyed` and `without` are `api.rs`'s test helpers today (`:3159–3164`'s callers); move them into
`backup.rs`'s `mod tests` as `pub(crate)` and have `api.rs`'s tests import them, or copy the two six-line
helpers — the implementer's call, named in the report.)

(b) In `tables.rs`'s `mod tests`, beside the `check_shape` cells (find `fn check_shape_` by content):

```rust
    /// **The table's own name is bounded on read as a seat's is** (security
    /// S1b, finding D25): `create` and `join` apply `check_name` on write, and a
    /// document that arrives through the archive route or a restore never
    /// passed through them — a 900 KB `name` was filed verbatim.
    #[test]
    fn check_shape_bounds_the_tables_own_name() {
        let mut doc = crate::games::test_support::playing_doc_for_tests();
        doc.name = Some("x".repeat(65));
        let said = check_shape(&doc).expect_err("65 characters");
        assert_eq!(said, "the table's name: a name is 1 to 64 characters");
        doc.name = Some("a\u{0007}b".to_string());
        assert_eq!(check_shape(&doc).expect_err("a control character"), "the table's name: a name has no control characters");
        doc.name = Some("x".repeat(64));
        assert!(check_shape(&doc).is_ok(), "64 is the bound");
    }
```

(`playing_doc_for_tests` is the name to give whichever `TableDoc` builder `tables.rs`'s cells already use for a
playing document — find it by content (`TableDoc {` with `status: TableStatus::Playing` under `mod tests`); if
the module has none, build the document inline as `archive.rs`'s `locked_doc` does, with `lock: None`.)

(c) In `api.rs`'s `mod tests`, after Task 1's retry cell (the pre-flight dropped a second cell here, the
attribution rule's — F7):

```rust
    /// **A bot this server does not run is refused at the archive** (finding
    /// D25): a browser's game is played by the module's production bot, so a
    /// record naming another controller is forged, or a game this build could
    /// not have played — `bots::foreign`'s own answer, the rule the game
    /// routes apply. The sentence names the bot as the game route's 409 does.
    #[test]
    fn a_record_naming_a_bot_this_server_does_not_run_is_refused() {
        let dir = scratch("archive_foreign_bot");
        let on = games_with(std::sync::Arc::new(crate::archive::DirArchive::new(&dir)));
        let mut v: serde_json::Value = serde_json::from_str(PLAYED_RECORD).expect("a record");
        let seat = v.pointer_mut("/run/seats/1/controller").expect("the bot's controller");
        assert_eq!(seat.as_str(), Some("v4-step9-draft1"), "the premise");
        *seat = "mcts-v1".into();
        let (status, said) = post_archive(&on, &v.to_string(), PLAYED_TABLE);
        assert_eq!(status, 400, "{said}");
        assert!(said["error"].as_str().unwrap_or("").starts_with("seat 1 was played by "), "{said}");
        assert!(std::fs::read_dir(dir.join("browser")).is_err(), "nothing was written");
        std::fs::remove_dir_all(&dir).ok();
    }
```

(If `bots::foreign` does not name `mcts-v1` as foreign at this head — the implementer probes it — the
descriptor to plant is whichever `browser_played_pin.rs`'s re-point cell uses; the report names the one chosen.)

(d) The order cell gains three rows and reads twelve. After the "a terminal digest is present < the table parses"
row and before "the table parses < the table's shape", two rows:

```rust
            // a terminal digest is present  <  the record is not locked down
            (&on, without(&locked, "terminal_digest"), PLAYED_TABLE.to_string(), "the record carries no terminal digest"),
            // the record is not locked down  <  the table parses
            (&on, locked.clone(), "not json at all".to_string(), crate::backup::LOCKED_DOWN),
```

and after "the table agrees < the table's id is a safe name", in place of the id row:

```rust
            // the table's id is a safe name  <  the configuration is possible
            (&on, impossible.clone(), unsafe_id.clone(), "the table's id is not a safe name"),
            // the configuration is possible  <  the seats agree with the record's bots
            (&on, without(&impossible, "run"), PLAYED_TABLE.to_string(), "the record's configuration is impossible"),
            // the seats agree with the record's bots  <  the record replays
            (&on, replay_broken(&without(PLAYED_RECORD, "run")), PLAYED_TABLE.to_string(), "the table's seats disagree"),
```

with `locked` and `impossible` built above the array (`keyed(PLAYED_RECORD, "lockdown", "a fault".into())`; the
two config flags set as cell (a) sets them), the "the table parses < the table's shape" row's record now
`no_replay` as before, the `no_digest` binding dropped (read by no row once the digest row is the lockdown pair —
the lint gate), the array `[…; 12]`, and `assert_eq!(sentences.len(), 12, "twelve checks, twelve
different sentences")`. The lone thirteenth, the replay, stays as it is.

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --lib -- key_the_engine locked_down
impossible_configuration seats_disagree bounds_the_tables names_another_person bot_this_server refusal_order
2>&1 | grep -E '^error|test result|panicked' | head` — `LOCKED_DOWN` not found; stubbed as
any sentences: every cell RED on its status or sentence (the planted key accepted; the locked record accepted;
the impossible config read "does not replay"; the seats accepted; the name accepted; the foreign bot 201).

- [ ] **Step 3: The code.** In `backup.rs`, above `check_pair`:

```rust
/// The refusal of a record the driver locked down (security S1b, finding D25):
/// `GameRecord.lockdown` is `Some` once the interactive driver found a desync
/// or an invariant fault, and such a record is not a played game.
pub const LOCKED_DOWN: &str = "the record is locked down — a game the driver declared faulty is not archived";

/// The engine record's keys, and nothing else (security S1b, finding D25).
/// `GameRecord` carries no `deny_unknown_fields` — it is the engine's type,
/// and a reader of saves stays lenient toward a newer field — so the archive
/// reads the text through this mirror FIRST, which refuses a key the record
/// does not have; every field is read and ignored, the mirror says what keys
/// may be present and the real parse says what they mean. A field core adds
/// to `GameRecord` fails
/// `a_record_with_a_key_the_engine_does_not_write_is_refused_and_the_committed_pairs_pass`
/// until it is named here: the drift a lenient mirror would hide.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct RecordMirror {
    engine_version: serde::de::IgnoredAny,
    rules_hash: serde::de::IgnoredAny,
    seed: serde::de::IgnoredAny,
    config: serde::de::IgnoredAny,
    choices: serde::de::IgnoredAny,
    events: serde::de::IgnoredAny,
    #[serde(default)]
    setup_observations: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    semantics_rev: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    terminal_digest: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    lockdown: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    run: Option<serde::de::IgnoredAny>,
}
```

(`GameRecord`'s eleven fields at this head: `engine_version`, `rules_hash`, `seed`, `config`, `choices`,
`events`, `setup_observations`, `semantics_rev`, `terminal_digest`, `lockdown`, `run` —
`crates/yokohama-core/src/game/record.rs:149–215`; the implementer counts them against the struct.)

In `check_pair_with`, the parse becomes the mirror then the record:

```rust
    // The mirror first (finding D25): a key the engine's record does not have
    // is a record that does not parse, naming the key.
    serde_json::from_str::<RecordMirror>(record).map_err(|e| format!("the record does not parse: {e}"))?;
    let rec: yokohama_core::GameRecord =
        serde_json::from_str(record).map_err(|e| format!("the record does not parse: {e}"))?;
```

after the digest check (`if rec.terminal_digest.is_none() { … }`):

```rust
    if rec.lockdown.is_some() {
        return Err(LOCKED_DOWN.to_string());
    }
```

and after the `is_safe_id` refusal, before the replay:

```rust
    // **What the engine would refuse, refused before setup** (finding D25):
    // the configuration `Session::from_record` validates before it builds
    // anything, and the seat agreement the resident rebuild asks of a pair
    // already in a store (`Games::live_handle`) — asked here of one arriving.
    yokohama_core::validate_config(&rec.config)
        .map_err(|why| format!("the record's configuration is impossible: {why}"))?;
    if let Some(why) = crate::bots::seat_disagreement(&doc, &rec) {
        return Err(format!("the table's seats disagree with the record's bots: {why}"));
    }
```

`check_pair`'s doc paragraph (Task 1's wording) reads, in full: "the record parsing — through a mirror that
refuses a key the engine's record does not have, then as the engine's own type — its event count within
[`EVENT_CAP`], a `semantics_rev` present and equal to this build's, a terminal digest present, no lockdown, the
table document parsing, passing [`tables::check_shape`] and AGREEING with the record, the document's id being
`is_safe_id`'s, the configuration one `validate_config` accepts, the seats agreeing with the record's bots
([`crate::bots::seat_disagreement`]), and LAST `replay_strict_with(.., Predicate)` clean." `archive_post`'s doc
(Task 1's) counts "the TWELVE checks" and lists them in the same words.

In `api.rs`, `archive_post`, after `check_pair_filed` succeeds (a held record passes this too, cheaply — it landed through it once):

```rust
    // **A bot this server does not run is refused** (finding D25): the record
    // is the module's own production bot's game or it is not a browser's game;
    // `bots::foreign` is the game routes' rule, asked once more here. The
    // record parsed a line ago inside `check_pair_with`; this second parse is
    // the price of keeping `PairFacts` four scalars.
    let rec: yokohama_core::GameRecord = serde_json::from_str(&req.record)
        .map_err(|e| ApiError::bad_request(format!("the record does not parse: {e}")))?;
    if let Some((seat, played, why)) = crate::bots::foreign(&rec).into_iter().next() {
        let (played, _) = crate::bots::distinguishing_labels(&played, &crate::bots::production());
        return Err(ApiError::bad_request(format!(
            "seat {seat} was played by {played}, which this server does not run: {why}"
        )));
    }
```

In `tables.rs`, `check_shape`, after the seat-name loop (:421–427):

```rust
    // **The table's own name, by the same rule as a seat's** (security S1b,
    // finding D25): `create` and `join` apply it on write, and a document that
    // arrives through the archive route or a restore never passed through them.
    if let Some(name) = &doc.name {
        if let Err(why) = check_name(name) {
            return Err(format!("the table's name: {why}"));
        }
    }
```

- [ ] **Step 4: Run, mutants, lint, commit.** `cargo test -p yokohama-web --lib 2>&1 | grep -E 'test
result|FAILED'` green; `cargo test -p yokohama-web` whole (the house copy on the Worker and `check_table_alone`
read `check_shape` too — the lane's rows run whole, the Worker pool included). Mutants: **M5** — the mirror parse
deleted: the planted-key cell RED; **M6** — the lockdown refusal deleted: its cell RED (accepted), the order cell
RED at its two rows; **M7** — `validate_config` deleted: the config cell RED ("does not replay"), the order cell
RED; **M8** — `seat_disagreement` deleted: the seats cell RED, the order cell RED; **M9** — the attribution
— withdrawn with the attribution rule (F7); **M10** — the foreign-bot refusal deleted: its cell RED
(201); **M11** — `check_shape`'s name rule deleted: the tables cell RED. `cargo clippy -p yokohama-web
--all-targets -- -D warnings` clean.

```bash
git add crates/yokohama-web/src/backup.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/tables.rs
git commit -m "web: the archive refuses what the engine would — an unknown key, a lockdown, an impossible configuration, seats that disagree with the record's bots, a table name past the rule — and a bot this server does not run (D25; S1b Task 2)"
```

---

### Task 3: B12 — a browser caller's share of the archive, counted before every put; the 507 the outbox shows — Opus (B12; R-SEC8(f); spec correction 4; Review Focus 2)

**Files:**
- Modify: `crates/yokohama-web/src/archive.rs` (`PutError` :62–86 — a variant and its `Display`; two constants
  and `Quota` above the trait; `DirArchive` :245–263 — a field, `new`/`from_env`, `with_quota`; `usage` and the
  check in `put` :361–399; cells in `mod tests`)
- Modify: `crates/yokohama-web/src/api.rs` (`archive_post`'s put arms :2340–2356 — the `Quota` arm; a sentence
  builder beside `ARCHIVE_BODY_LIMIT`; a cell beside Task 1's)
- Modify: `web/src/outbox.ts` (`reasonFor` :179–185 — the 507), `web/src/outbox.test.ts` (a cell beside the
  409's at :238–246)
- Test: `archive.rs`'s and `api.rs`'s `mod tests`; `web/src/outbox.test.ts`

**Interfaces:**
- Consumes: Task 1's `holds` (the marker read precedes the count, so a retry survives a full share); the
  api-test helpers; the outbox test's `hosts`/`pending` seams (`outbox.test.ts:238–246`'s shape).
- Produces: `archive::{QUOTA_RECORDS, QUOTA_BYTES, Quota, Quota::PRODUCTION}`, `DirArchive::with_quota`,
  `PutError::Quota { records, bytes }`; `api::archive_full_sentence(records, bytes)`; the route's 507; the
  outbox's `offline` row carrying the server's sentence on a 507.

- [ ] **Step 1: The failing cells.**

(a) In `archive.rs`'s `mod tests`, after Task 1's `holds` cell:

```rust
    /// **A browser caller's share is counted before a put, and the count
    /// never touches a retry or the server's own games** (finding B12;
    /// R-SEC8(f); Review Focus 2). Under a two-record share two pairs land, a
    /// third is refused as `Quota` with nothing written, the first pair's
    /// bytes again are `Identical` (the marker read precedes the count),
    /// another caller's share is its own, and three server games land under
    /// the empty prefix. Then the BYTES: a hundred-byte share takes one
    /// eighty-byte pair and refuses the next.
    #[cfg(feature = "server")]
    #[test]
    fn a_browser_callers_share_is_counted_before_a_put_and_a_retry_survives_a_full_one() {
        let dir = std::env::temp_dir().join(format!("yokohama_archive_quota_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = DirArchive::new(&dir).with_quota(Quota { records: 2, bytes: 1 << 20 });
        let prefix = "browser/a_at_b/";
        assert!(matches!(a.put(prefix, Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}"), Ok(PutOutcome::Landed)));
        assert!(matches!(a.put(prefix, Why::Abandoned, "local-2", "{\"r\":2}", "{\"t\":2}"), Ok(PutOutcome::Landed)));
        let third = a.put(prefix, Why::Over, "local-3", "{\"r\":3}", "{\"t\":3}");
        assert!(matches!(third, Err(PutError::Quota { records: 2, bytes }) if bytes == 1 << 20), "{third:?}");
        let (record, table) = paths(yokohama_core::SEMANTICS_REV, prefix, Why::Over, "local-3").unwrap();
        assert!(!dir.join(&record).exists() && !dir.join(&table).exists(), "nothing was written");
        assert!(matches!(a.put(prefix, Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}"), Ok(PutOutcome::Identical)), "a retry survives a full share");
        assert!(matches!(a.put("browser/c_at_d/", Why::Over, "local-3", "{\"r\":3}", "{\"t\":3}"), Ok(PutOutcome::Landed)), "another caller's share");
        for id in ["sim-1", "sim-2", "sim-3"] {
            assert!(matches!(a.put("", Why::Over, id, "{\"r\":9}", "{\"t\":9}"), Ok(PutOutcome::Landed)), "the server's own games are never counted");
        }
        let b = DirArchive::new(&dir).with_quota(Quota { records: 100, bytes: 100 });
        let forty = format!("{{\"r\":\"{}\"}}", "x".repeat(32));
        assert_eq!(forty.len(), 40, "the premise");
        assert!(matches!(b.put("browser/e_at_f/", Why::Over, "local-1", &forty, &forty), Ok(PutOutcome::Landed)), "eighty bytes under a hundred");
        let over = b.put("browser/e_at_f/", Why::Over, "local-2", &forty, &forty);
        assert!(matches!(over, Err(PutError::Quota { records: 100, bytes: 100 })), "a hundred and sixty over a hundred: {over:?}");
        assert_eq!(Quota::PRODUCTION, Quota { records: QUOTA_RECORDS, bytes: QUOTA_BYTES });
        std::fs::remove_dir_all(&dir).ok();
    }
```

(b) In `api.rs`'s `mod tests`, after Task 2's cells:

```rust
    /// **A full share is a 507 in the archive's words, and a retry of a filed
    /// record is still 200** (finding B12; Review Focus 2): the archive's
    /// `Quota` is the server's state and not the record's, so it is neither
    /// the 400 nor the 409 the outbox ENDS a row on — the outbox keeps the row,
    /// shows this sentence and retries on its next drain.
    #[test]
    fn a_full_share_is_a_507_and_the_retry_of_a_filed_record_is_still_200() {
        let dir = scratch("archive_quota");
        let on = games_with(std::sync::Arc::new(
            crate::archive::DirArchive::new(&dir).with_quota(crate::archive::Quota { records: 1, bytes: 1 << 20 }),
        ));
        let (status, v) = post_archive(&on, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!(status, 201, "{v}");
        let second = keyed(PLAYED_TABLE, "id", "local-second".into());
        let (status, v) = post_archive(&on, PLAYED_RECORD, &second);
        assert_eq!(status, 507, "{v}");
        assert_eq!(v["error"], archive_full_sentence(1, 1 << 20));
        assert!(!dir.join("browser/local_at_browser/rev7/over/local-second.json").exists(), "nothing was written");
        let (status, v) = post_archive(&on, PLAYED_RECORD, PLAYED_TABLE);
        assert_eq!((status, v["identical"].as_bool()), (200, Some(true)), "{v}");
        std::fs::remove_dir_all(&dir).ok();
    }
```

(c) In `web/src/outbox.test.ts`, beside the 409 cell (:238–246):

```ts
  // A 507 says the caller's share of the archive is FULL — the server's state,
  // not the record's (security S1b, finding B12). The row is kept and retried
  // on the next drain, as an offline row is, and it carries the server's own
  // sentence so the player knows why it waits.
  it("reads a 507 as a wait that carries the server's sentence, not as a refusal", async () => {
    const full = "your archive here is full — 256 records or 128 MiB; nothing was written, and the game is kept on this device";
    hosts({ archive: "dir", onArchive: () => ({ status: 507, body: { error: full } }) });
    await noteGameOver(GAME);
    expect(await pending()).toEqual([expect.objectContaining({ id: GAME, reason: "offline", sentence: full })]);
    expect(reasonWords("offline", full)).toBe(full);
  });
```

(the cell is the 409 cell's shape with its names — `hosts`, `pending`, `GAME`, `noteGameOver` (the helper the 409
cell calls), `reasonWords` imported from `./outbox` — the implementer reads the 409 cell and mirrors it.)

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --lib -- callers_share full_share
2>&1 | grep -E '^error|test result|panicked' | head` — `Quota`, `with_quota`, `PutError::Quota`,
`archive_full_sentence` not found; stubbed (the builder `self`, the variant unused, the sentence any): the
archive cell RED at the third put (`Landed`), the route cell RED (201 where 507). `cd web && npx vitest run
src/outbox.test.ts -t 507 2>&1 | tail -5` — RED: the row reads `offline` with no sentence.

- [ ] **Step 3: The code.** In `archive.rs`, above the trait (:88):

```rust
/// A browser caller's share of this archive (security S1b, finding B12;
/// R-SEC8(f)): 256 records and 128 MiB under `browser/<caller>/`, counted
/// before every put. Set from real games: a finished game is 25–60 KB
/// pretty-printed, so 256 is years of one person's play, and 128 MiB is that
/// count at the body cap (`api::ARCHIVE_BODY_LIMIT`, 512 KiB) — so neither
/// bound is dead. The archive is a collection of played games, not an import
/// path for bulk uploads, and on the NAS it shares the volume every live game
/// saves to; the review filed 19 MB in 1.3 s against it.
pub const QUOTA_RECORDS: usize = 256;
/// [`QUOTA_RECORDS`]'s companion in bytes: 128 MiB.
pub const QUOTA_BYTES: u64 = 128 << 20;

/// What one browser caller may hold: records and bytes under its prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quota {
    pub records: usize,
    pub bytes: u64,
}

impl Quota {
    /// The production share — every `DirArchive` a server builds.
    pub const PRODUCTION: Quota = Quota { records: QUOTA_RECORDS, bytes: QUOTA_BYTES };
}
```

`PutError` (:62–77) gains, after `Failed`:

```rust
    /// The caller's share is full (security S1b, finding B12): nothing was
    /// written. Carries the share's two bounds for the route's sentence.
    Quota { records: usize, bytes: u64 },
```

with its `Display` arm `Self::Quota { records, bytes } => write!(f, "the caller's archive is full ({records}
records or {bytes} bytes)")`. `DirArchive` gains `quota: Quota`; `new` and `from_env` set
`Quota::PRODUCTION`; after `from_env`:

```rust
        /// This archive under a share of the caller's — the builder a cell takes
        /// to fill one; every server builds [`Quota::PRODUCTION`].
        pub fn with_quota(mut self, quota: Quota) -> Self {
            self.quota = quota;
            self
        }

        /// What `prefix` holds: records (`.json` files that are not
        /// `.table.json`) and the bytes of every `.json` file under it, at any
        /// depth. Zero where the prefix's directory does not exist yet; a read
        /// that fails is a `Failed`, never a zero.
        fn usage(&self, prefix: &str) -> Result<(usize, u64), PutError> {
            fn walk(dir: &Path, records: &mut usize, bytes: &mut u64) -> std::io::Result<()> {
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    let meta = entry.metadata()?;
                    if meta.is_dir() {
                        walk(&entry.path(), records, bytes)?;
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !name.ends_with(".json") {
                        continue;
                    }
                    *bytes += meta.len();
                    if !name.ends_with(".table.json") {
                        *records += 1;
                    }
                }
                Ok(())
            }
            let root = self.dir.join(prefix);
            if !root.exists() {
                return Ok((0, 0));
            }
            let (mut records, mut bytes) = (0usize, 0u64);
            walk(&root, &mut records, &mut bytes).map_err(|e| PutError::Failed(format!("count {prefix}: {e}")))?;
            Ok((records, bytes))
        }
```

and `put` (:361–399), after the marker read and before the table's write:

```rust
            // **A browser caller's share, counted before the writes** (security
            // S1b, finding B12) — AFTER the marker read, so a retry of a filed
            // record answers `Identical` once the share is full. A server's own
            // games (the empty prefix) are never counted.
            if prefix.starts_with("browser/") {
                let (records, bytes) = self.usage(prefix)?;
                let adding = (record_json.len() + table_json.len()) as u64;
                if records >= self.quota.records || bytes + adding > self.quota.bytes {
                    return Err(PutError::Quota { records: self.quota.records, bytes: self.quota.bytes });
                }
            }
```

In `api.rs`, beside `ARCHIVE_BODY_LIMIT`:

```rust
/// The 507 a browser's save meets once its caller's share of the archive is
/// full (security S1b, finding B12): the SERVER's state, not the record's —
/// the outbox keeps the row, shows this sentence and retries it on its next
/// drain, so an operator who makes room sees the game land.
pub fn archive_full_sentence(records: usize, bytes: u64) -> String {
    format!(
        "your archive here is full — {records} records or {} MiB; nothing was written, and the game is kept on this device",
        bytes >> 20
    )
}
```

and `archive_post`'s match gains, before the `Failed` arm:

```rust
        Err(PutError::Quota { records, bytes }) => Err(ApiError {
            status: StatusCode::INSUFFICIENT_STORAGE,
            message: archive_full_sentence(records, bytes),
            table_status: None,
            foreign_bot: None,
        }),
```

`games.rs`'s game-over put (:1062) needs no arm: its match ends in a catch-all `Err(e)` that logs
`e.to_string()`, which the `Display` arm above words (unreachable for the server's own prefix, never counted —
the pre-flight's F5). In `web/src/outbox.ts`, `reasonFor` (:179–185):

```ts
function reasonFor(e: unknown): { reason: PendingReason; sentence?: string } {
  if (e instanceof ApiError) {
    if (e.status === 401) return { reason: "signed-out" };
    if (e.status === 400 || e.status === 409) return { reason: "refused", sentence: e.message };
    // A 507 is the caller's share of the archive FULL (security S1b, finding
    // B12): the server's state, retried like an offline row, and shown.
    if (e.status === 507) return { reason: "offline", sentence: e.message };
  }
  return { reason: "offline" };
}
```

with its doc's last sentence reading "…a 403, a 5xx or a fetch that never arrived is about the moment, and is
retried — a 507 among them, which carries the server's sentence: the caller's share of the archive is full".

- [ ] **Step 4: Run, mutants, lint, commit.** `cargo test -p yokohama-web --lib` green; `cargo test -p
yokohama-web` whole; the lane's rows whole, `cd web && npm test -- --run` included (the outbox cell). Mutants:
**M29** — the count before the marker read: the archive cell RED at the retry (`Quota` where `Identical`);
**M30** — the bytes arm dropped (`records >=` alone): the archive cell RED at the hundred-byte share; **M31** —
the prefix test dropped (every prefix counted): the archive cell RED at the server's third game; **M32** — the
route's arm answering 500: the route cell RED; **M33** — the outbox's 507 line dropped: the vitest cell RED (no
sentence). `cargo clippy --workspace --all-targets -- -D warnings` clean.

```bash
git add crates/yokohama-web/src/archive.rs crates/yokohama-web/src/api.rs web/src/outbox.ts web/src/outbox.test.ts
git commit -m "web: a browser caller's share of the archive is 256 records and 128 MiB, counted before every put after the marker read; a full share is a 507 the outbox keeps, shows and retries (B12; S1b Task 3)"
```

---

### Task 4: D6 and the replay gate — every dispatch on the blocking pool, the five replay routes behind two permits — Opus (D6, A2's semaphore; R-SEC8(f); spec correction 8; Review Focus 3)

**Files:**
- Modify: `crates/yokohama-web/src/lib.rs` (`AppState` :354–399 — two fields; `with_store_chat_archive_and_log`
  :528–584 — their production values; a builder after `with_seed_policy` :612; `events` :697–705; `adapter`
  :854–919 — rewritten; three constants and `busy` beside it; cells in `mod tests`)
- Modify: `crates/yokohama-web/src/api.rs` (beside `routes` :199 — `REPLAY_ROUTES`, `is_replay_route`; a cell in
  `mod tests`)
- Test: `lib.rs`'s and `api.rs`'s `mod tests`

**Interfaces:**
- Consumes: `api::{find, Matched}` (private to `api.rs`, read by the new function beside them);
  `tokio::sync::Semaphore` and `tokio::time::timeout` (the crate's `tokio` features `sync` and `time`, already
  on); the route-cell helpers `post_as`, `get_as`, `a_game`, `play_turn`, `WHO`; the browser-played pair.
- Produces: `api::REPLAY_ROUTES`, `api::is_replay_route`; `REPLAY_PERMITS`, `REPLAY_WAIT`, `BUSY`,
  `AppState { replay_gate, replay_wait }`, `AppState::with_replay_gate`; a 503 with `Retry-After: 1`.

- [ ] **Step 1: The failing cells.**

(a) In `api.rs`'s `mod tests`, beside the route-table cells (find `fn every_route_` or the `routes(` cells by
content):

```rust
    /// **The replay gate's five routes, by the table's own match** (security
    /// S1b, findings A2 and D6): an archive upload, a run and a position of a
    /// game's history, the two searches — and nothing else: a move, a frame,
    /// the record, the health, a wrong method on a gated path, a path the
    /// table does not route.
    #[test]
    fn the_replay_routes_are_the_five_that_replay_or_search() {
        for (method, path) in [
            ("POST", "/api/archive"),
            ("GET", "/api/games/wire/at"),
            ("GET", "/api/games/wire/at/12"),
            ("GET", "/api/games/wire/suggest"),
            ("GET", "/api/games/wire/why"),
        ] {
            assert!(is_replay_route(method, path), "{method} {path} is gated");
        }
        for (method, path) in [
            ("POST", "/api/games/wire/input"),
            ("GET", "/api/games/wire"),
            ("GET", "/api/games/wire/record"),
            ("GET", "/api/health"),
            ("GET", "/api/archive"),
            ("POST", "/api/games/wire/at"),
            ("GET", "/api/games//at"),
            ("GET", "/api/games/wire/at/12/13"),
        ] {
            assert!(!is_replay_route(method, path), "{method} {path} is never gated");
        }
        assert_eq!(REPLAY_ROUTES.len(), 5);
    }
```

(b) In `lib.rs`'s `mod tests`, after `delete_as` (:2420) — the slow archive and the two cells:

```rust
    /// A test archive whose put SLEEPS — the one honest way to make a dispatch
    /// slow: the archive route's checks pass on the committed pair and the put
    /// is where the time goes. `kind` reads `dir`, so `is_on` is true.
    struct SlowArchive(std::time::Duration);

    impl archive::Archive for SlowArchive {
        fn put(
            &self,
            _prefix: &str,
            _why: archive::Why,
            _id: &str,
            _record_json: &str,
            _table_json: &str,
        ) -> Result<archive::PutOutcome, archive::PutError> {
            std::thread::sleep(self.0);
            Ok(archive::PutOutcome::Landed)
        }
        fn kind(&self) -> &'static str {
            "dir"
        }
    }

    const PLAYED_RECORD: &str = include_str!("../tests/fixtures/browser-played-2p.json");
    const PLAYED_TABLE: &str = include_str!("../tests/fixtures/browser-played-2p.table.json");

    fn archive_body() -> serde_json::Value {
        serde_json::json!({ "record": PLAYED_RECORD, "table": PLAYED_TABLE })
    }

    fn slow_state(delay_ms: u64, permits: usize, wait_ms: u64) -> Arc<AppState> {
        Arc::new(
            AppState::with_store_chat_and_archive(
                Arc::new(MemStore::default()),
                identity::IdentityMode::Open,
                chat::ChatConfig::default(),
                Arc::new(SlowArchive(std::time::Duration::from_millis(delay_ms))),
            )
            .with_replay_gate(permits, std::time::Duration::from_millis(wait_ms)),
        )
    }

    /// **Two slow replays do not stall the server** (finding D6; the review's
    /// measured stall: two uploads froze a two-worker server for 35 s). On a
    /// runtime of TWO workers, two archive posts whose put sleeps a second are
    /// in flight, and a pause of 150 ms followed by a health read completes in
    /// well under that second — because every dispatch runs on the blocking
    /// pool, never on the workers the two posts would otherwise hold. The clock
    /// runs from before the pause, since with the dispatch inline it is the
    /// pause's wake that waits for a worker: the mutant that dispatches inline
    /// reads about a second.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn two_slow_replays_leave_the_server_answering() {
        let state = slow_state(1_000, 2, 2_000);
        let a = {
            let s = Arc::clone(&state);
            tokio::spawn(async move { post_as(&s, "local@browser", "/api/archive", archive_body()).await })
        };
        let b = {
            let s = Arc::clone(&state);
            tokio::spawn(async move { post_as(&s, "local@browser", "/api/archive", archive_body()).await })
        };
        // The clock runs from BEFORE the pause: with the dispatch inline, both
        // workers sit in the two puts and the pause's own wake waits for one
        // of them — the stall shows there as much as in the read.
        let started = std::time::Instant::now();
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let (status, _) = get_as(&state, WHO, "/api/health").await;
        let took = started.elapsed();
        assert_eq!(status, StatusCode::OK);
        assert!(took < std::time::Duration::from_millis(400), "the pause and the health read waited {took:?} behind two replays");
        let (sa, va) = a.await.expect("the first post");
        let (sb, vb) = b.await.expect("the second post");
        assert!(matches!(sa.as_u16(), 200 | 201), "{va}");
        assert!(matches!(sb.as_u16(), 200 | 201), "{vb}");
    }

    /// **A replay past the gate's wait is a 503 with `Retry-After: 1`, and a
    /// move is never behind the gate** (findings A2 and D6; Review Focus 3).
    /// One permit and a fifty-millisecond wait; a post whose put sleeps a
    /// second holds the permit; a second post reads the 503 in `BUSY`'s words
    /// within the wait, while an `input` at a game and a health read answer
    /// 200 beside it. The mutant that gates nothing answers the second post
    /// 201 after the second; the one that gates every route answers the move
    /// 503; the one that drops the header fails the header line.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_replay_past_the_gates_wait_is_refused_busy_while_a_move_lands() {
        let state = slow_state(1_000, 1, 50);
        a_game(&app(Arc::clone(&state)), "gated", &[]).await;
        let holder = {
            let s = Arc::clone(&state);
            tokio::spawn(async move { post_as(&s, "local@browser", "/api/archive", archive_body()).await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let started = std::time::Instant::now();
        let req = Request::builder()
            .method("POST")
            .uri("/api/archive")
            .header(identity::CLAIMED_HEADER, "local@browser")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&archive_body()).expect("a body")))
            .expect("a request");
        let res = app(Arc::clone(&state)).oneshot(req).await.expect("the router answers");
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(res.headers().get(http::header::RETRY_AFTER).and_then(|v| v.to_str().ok()), Some("1"));
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
        assert_eq!(body["error"], BUSY);
        assert!(started.elapsed() < std::time::Duration::from_millis(600), "refused within the wait, not after the holder");
        play_turn(&state, "gated", WHO).await;
        let (status, _) = get_as(&state, WHO, "/api/health").await;
        assert_eq!(status, StatusCode::OK);
        let (status, v) = holder.await.expect("the holder's post");
        assert!(matches!(status.as_u16(), 200 | 201), "{v}");
    }
```

(`play_turn(state, id, user)` (:2509) drives one turn of `user` at `id` through `input` and asserts each
reply's status; if it reads a prompt kind the two-seat hot seat does not offer at its first turn, the implementer
posts the prompt's first row through `post_as(&state, WHO, "/api/games/gated/input", json!({"line": "1"}))` and
asserts 200 — the fact under test is the status, not the move.)

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --lib -- replay_routes two_slow
gates_wait 2>&1 | grep -E '^error|test result|panicked' | head` — `is_replay_route`, `REPLAY_ROUTES`,
`with_replay_gate` and `BUSY` not found; stubbed (the function `false`, the builder `self`, the constant any
sentence): (a) RED on the first gated route; the stall cell RED — **measured in the pre-flight: the health read
behind two inline replays took about a second — 1.05 s measured under the old adapter's body swapped in whole** (the review record); the gate cell RED (201 after the holder, no 503).

- [ ] **Step 3: The code.** In `api.rs`, after `routes` (:199–242):

```rust
/// The routes the NAS answers behind its replay gate (security S1b, findings
/// A2 and D6): the ones that replay a record or run a search — an archive
/// upload's strict replay, a run or a position of a game's history, the two
/// searches. `input` is NOT among them: a move must land while a burst of
/// replays waits, and an undo folds from the session's own ladder.
pub const REPLAY_ROUTES: [(&str, &str); 5] = [
    ("POST", "/api/archive"),
    ("GET", "/api/games/{id}/at"),
    ("GET", "/api/games/{id}/at/{k}"),
    ("GET", "/api/games/{id}/suggest"),
    ("GET", "/api/games/{id}/why"),
];

/// Whether `method` and `path` name one of [`REPLAY_ROUTES`] — by the table's
/// own match, so a path the table does not route, or a wrong method on a gated
/// one, is never gated.
pub fn is_replay_route(method: &str, path: &str) -> bool {
    match find(method, path, true) {
        Matched::Route(pattern, _) => REPLAY_ROUTES.iter().any(|(m, p)| *m == method && *p == pattern),
        Matched::WrongMethod | Matched::NoRoute => false,
    }
}
```

(`find`'s third argument is the host's `serves_games`; `true` reads the whole table, since the gate is a fact
about the route, not the deployment — a host with multiplayer off never routes the four game rows anyway.)

In `lib.rs`, above `AppState` (:346):

```rust
/// The replay gate's permits (security S1b, findings A2 and D6; R-SEC8(f)):
/// how many of the five replay routes (`api::REPLAY_ROUTES`) the NAS runs at
/// once. Two, from the NAS's four cores (the review's read): two blocking
/// replays leave two cores to the runtime, and at the ≤ 55 ms an admitted
/// replay costs under the predicate check two permits clear about
/// thirty-six worst-case uploads a second — far above the edge's twenty per
/// ten seconds per client. The gate bounds the server's work, never a
/// person's: a game's moves are not behind it.
#[cfg(feature = "server")]
pub const REPLAY_PERMITS: usize = 2;

/// How long a replay request waits for a permit before the adapter answers
/// 503 with `Retry-After: 1` (security S1b): two seconds, so queued bodies do
/// not pile up behind a burst — a client that meets the 503 asks again, as
/// the outbox and the turn playback both do on their own.
#[cfg(feature = "server")]
pub const REPLAY_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// The 503's sentence — the ADAPTER's own, since the gate is the NAS's and
/// not the layer's.
#[cfg(feature = "server")]
pub const BUSY: &str = "the server is busy replaying — try again in a second";
```

`AppState` gains two fields after `ttls`:

```rust
    /// The replay gate (security S1b, findings A2 and D6): the permits the
    /// five replay routes share — [`REPLAY_PERMITS`] on every constructor; a
    /// cell sets its own through [`Self::with_replay_gate`].
    pub replay_gate: Arc<tokio::sync::Semaphore>,
    /// How long a replay request waits for a permit — [`REPLAY_WAIT`].
    pub replay_wait: std::time::Duration,
```

`with_store_chat_archive_and_log`'s literal (:569–583) gains
`replay_gate: Arc::new(tokio::sync::Semaphore::new(REPLAY_PERMITS)), replay_wait: REPLAY_WAIT,`; after
`with_seed_policy` (:612–615):

```rust
    /// This state under its own replay gate (security S1b) — the builder a
    /// cell takes to hold the permits and read the 503, [`Self::with_clock`]'s
    /// shape.
    pub fn with_replay_gate(self, permits: usize, wait: std::time::Duration) -> Self {
        Self { replay_gate: Arc::new(tokio::sync::Semaphore::new(permits)), replay_wait: wait, ..self }
    }
```

`events` (:697–705) hands its reading to the pool as `stream` hands each bump's:

```rust
    api::check_wait(q.wait)?;
    let games = Arc::clone(&state);
    match tokio::task::spawn_blocking(move || api::events_at(&games.games, &id, &user)).await {
        Ok(reading) => Ok(Json(reading?)),
        Err(_) => Err(ApiError::server_error("the reading could not be taken — try again")),
    }
```

and `adapter` (:854–919) becomes:

```rust
/// The ONE piece of axum between a request and the layer (R-6.5a.3). Identity
/// is resolved here with today's extractor logic and handed to the layer as the
/// host's sentence; the body and query reach the layer as bytes and text. The
/// body limit is axum's own default (2 MiB), so nothing this API accepted
/// before is refused now.
///
/// **Every dispatch runs on the blocking pool, never on the executor**
/// (security S1b, finding D6): the layer's call takes a game's lock, reads a
/// store, and may replay or search, and inline on a runtime worker a slow one
/// stalled every other task on that worker — the review measured two uploads
/// freezing a two-worker server for 35 s. The hand-off costs a clone of the
/// state's `Arc`, the owned request parts, a thread wake and a join, as the
/// stream's per-bump reading already pays.
///
/// **The replay gate** (findings A2 and D6): one of the five replay routes
/// (`api::REPLAY_ROUTES`) takes a permit first, waiting at most
/// [`AppState::replay_wait`]; past the wait the answer is 503 with
/// `Retry-After: 1`, and nothing of the request has run. Every other route is
/// never behind it.
#[cfg(feature = "server")]
async fn adapter(State(state): State<Arc<AppState>>, req: axum::extract::Request) -> Response {
    let (parts, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, 2 << 20).await {
        Ok(b) => b,
        Err(e) => {
            return ApiError::bad_request(format!("the body could not be read: {e}")).into_response()
        }
    };
    let method = parts.method.as_str().to_string();
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().unwrap_or("").to_string();
    let resolved = identity::resolve(state.identity, &parts.headers);
    let content_type =
        parts.headers.get(http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(str::to_string);
    // Art is a server route (the manifest reads the gated `art` module), listed
    // in the layer's table for the 405/404 arithmetic and answered here.
    if path == "/api/art" && (parts.method == http::Method::GET || parts.method == http::Method::HEAD) {
        let mut response = match art_manifest().await {
            Ok(j) => j.into_response(),
            Err(e) => e.into_response(),
        };
        // HEAD leaves with an empty body here too, as it does through `dispatch`.
        if parts.method == http::Method::HEAD {
            *response.body_mut() = axum::body::Body::empty();
        }
        return response;
    }
    let _permit = if api::is_replay_route(&method, &path) {
        match tokio::time::timeout(state.replay_wait, Arc::clone(&state.replay_gate).acquire_owned()).await {
            Ok(Ok(permit)) => Some(permit),
            Ok(Err(_)) | Err(_) => return busy(),
        }
    } else {
        None
    };
    // On THIS host the two multiplayer facts are one answer: an axum deployment
    // that offers no multiplayer serves no game route either, because nothing
    // plays here locally (spec §3.1 — the browser host is where they differ).
    // The TTLs are the ones the sweep runs under (M6.5c2 cycle F, R-F9), so
    // `/api/health` reports what is in force — both `null` where none is set,
    // as before the sweep (R-6.5c2.42).
    let serves_games = state.multiplayer.0;
    let joined = {
        let state = Arc::clone(&state);
        tokio::task::spawn_blocking(move || {
            let identity: api::Identity<'_> = match &resolved {
                Ok(u) => Ok(u),
                Err(m) => Err(m.as_str()),
            };
            let request = api::ApiRequest {
                method: &method,
                path: &path,
                query: &query,
                content_type: content_type.as_deref(),
                body: &bytes,
            };
            let host = api::Host {
                identity_name: state.identity.name(),
                signal_kind: SIGNAL_KIND,
                serves_games,
                multiplayer_offered: serves_games,
                table_ttl_days: state.ttls.table,
                open_table_ttl_days: state.ttls.open,
            };
            let out = api::dispatch(&state.games, &host, identity, &request);
            let allowed = api::allowed_methods(&path, serves_games);
            (out, allowed)
        })
        .await
    };
    let (out, allowed) = match joined {
        Ok(pair) => pair,
        // A dispatch that PANICKED on the pool: the game's lock, if it held
        // one, is poisoned and evicted on the next take (`Games::lock_game`);
        // this request is a 500 that says so.
        Err(_) => return ApiError::server_error("the request could not be answered — try again").into_response(),
    };
    let mut response = Response::new(axum::body::Body::from(out.body.clone()));
    *response.status_mut() = StatusCode::from_u16(out.status).expect("a known status");
    if !out.body.is_empty() {
        response
            .headers_mut()
            .insert(http::header::CONTENT_TYPE, http::HeaderValue::from_static("application/json"));
    }
    // axum's 405 carried an `Allow` header naming the methods the path takes;
    // `any()` skips it, so the adapter puts it back from the layer's own table
    // (R-6.5a.3: "axum's 405 shape, reproduced").
    if out.status == 405 {
        response
            .headers_mut()
            .insert(http::header::ALLOW, http::HeaderValue::from_str(&allowed.join(",")).expect("ASCII method names"));
    }
    response
}

/// The adapter's 503 (security S1b): the replay gate's refusal, with the
/// header a client's retry reads.
#[cfg(feature = "server")]
fn busy() -> Response {
    let mut response = ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        message: BUSY.to_string(),
        table_status: None,
        foreign_bot: None,
    }
    .into_response();
    response.headers_mut().insert(http::header::RETRY_AFTER, http::HeaderValue::from_static("1"));
    response
}
```

(`identity::resolve` returns `Result<User, String>` (`identity.rs:227`), owned on both arms, so it crosses into
the closure; `IdentityMode::name` is a `&'static str`; `SIGNAL_KIND` a constant. If `api::Host` carries a field
this head's struct does not (`api.rs:54`), the implementer reads the struct and reports.)

- [ ] **Step 4: Run, mutants, lint, commit.** `cargo test -p yokohama-web --lib 2>&1 | grep -E 'test
result|FAILED'` green; the two timed cells are run THREE times to read their margins (each reading in the report; the
pre-flight's: the health read behind two slow replays 0.40 / 0.55 / 0.71 ms against the 400 ms bound, the 503
within 53.9 / 53.8 / 55.1 ms of a 50 ms wait against 600 ms). Mutants: **M12** — `spawn_blocking` removed (the
dispatch inline on the worker, the old adapter): the stall cell RED (the health read about a second — 1.05 s measured under the old adapter's body swapped in whole); **M13** — the gate
removed (`_permit` always `None`): the gate cell RED (201 after the holder, no 503); **M14** — `is_replay_route`
returning `true` for every route: the gate cell RED at the move (503), the route cell at its "never gated"
rows, and the stall cell, whose health read then waits for a permit; **M15** — the `Retry-After` line dropped: the gate cell RED at the header. `cargo
clippy -p yokohama-web --all-targets -- -D warnings` clean; the lane's rows whole; the NAS binary's cells
(`tests/start_refusal.rs`) green — the adapter is the path they run.

```bash
git add crates/yokohama-web/src/lib.rs crates/yokohama-web/src/api.rs
git commit -m "web: every dispatch runs on the blocking pool, and the five replay routes wait at most two seconds for one of two permits, then 503 with Retry-After (D6, A2's semaphore; S1b Task 4)"
```

---

### Task 5: The session's ladder and the predicate fold — a replay to any position from the nearest kept state; `/at`, a run, an undo and a rebuild through it; D27; the record route's parked-read pin — Opus (A3's replay checkpoint, D27; spec corrections 2 and 7; Review Focus 4)

**Files:**
- Modify: `crates/yokohama-cli/src/session.rs` (`Session` :67–110 — the `ladder` field; a constant and
  `fold_event` above `impl Session`; `from_record` :218–391 — the fold loop :331–343 and the literal :380–390;
  `apply` :532–625 and `apply_standalone` :651–679 — a call after the commit; `undo` :717–762 — the fold;
  `state_before` and a free `fold_to`; `for_test_from_state` :816–829; every other `Session { … }` literal; cells
  in `mod tests` :832)
- Modify: `crates/yokohama-web/src/client.rs` (`HistoricalFrame::at` :695–698 and `run` :707–728 — `&mut Repl`,
  `state_before`, the fold; their callers in `mod tests`)
- Modify: `crates/yokohama-web/src/api.rs` (`run` :2044–2069 — D27 and `let mut repl`; `at_k` :2110–2138 —
  `let mut repl`; the 500 sentences)
- Modify: `crates/yokohama-web/src/tables.rs` (:699 — the deal preview's `HistoricalFrame::at` caller: `let mut
  repl = Repl::new(session)`, `&mut repl`, its `map_err` reading the `String` — `format!("the setup position did
  not build: {e}")`; :2156 — a cell's `&mut handle.lock().unwrap()`; the pre-flight's F14)
- Test: `session.rs`'s `mod tests`; `lib.rs`'s `mod tests` (D27's two route cells; the parked-read double)

**Interfaces:**
- Consumes: `yokohama_core::{is_listed_move, apply_move_unchecked, replay_from, state_at, MoveError,
  GameState, RecordedEvent, GameRecord, Rules}` (all at the crate root); `Repl.session` (`pub`); the lib
  helpers `a_finished_vs_bot_game`, `get_as`, `post_as`, `a_game`, `WHO`; `store::{Store, Doc, MemStore}`.
- Produces: `session::CHECKPOINT_EVERY`; `Session::fold_event`; `Session::state_before(&mut self, k)`;
  `HistoricalFrame::at(&mut Repl, ..) -> Result<Self, String>` and `::run(&mut Repl, ..) -> Result<Vec<Self>,
  String>`; `api::run`'s empty answer and its 404 at `from > total`.

- [ ] **Step 1: The failing cells.**

(a) In `session.rs`'s `mod tests`, at its end — a record by random legal play, built with the engine alone so
the cell owes nothing to the CLI's own constructors:

```rust
    /// A finished-or-long record by uniform-random legal play, the engine's own
    /// primitives, `n` events or the game's end.
    fn random_record(players: u8, seed: u64, n: usize) -> (GameRecord, Rules) {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(players);
        let choices = vec![StartingOrderChoice::Keep(0); players as usize];
        let pending = new_game(config.clone(), &rules, seed).expect("new_game");
        let mut state = finalize_setup(pending, &choices).expect("finalize_setup");
        let mut record = GameRecord::new(seed, config, choices, &rules);
        let mut x = seed ^ 0x9E37_79B9_7F4A_7C15;
        while record.events.len() < n && !is_game_over(&state) {
            let legal = get_legal_moves(&state, &rules);
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let mv = legal[(x % legal.len() as u64) as usize].clone();
            state = apply_move_unchecked(&state, &mv, &rules).expect("a listed move applies");
            record.push_move(mv);
        }
        record.stamp_terminal_digest(&state);
        (record, rules)
    }

    /// **`state_before(k)` is `state_at(k)` at every prefix, and the ladder it
    /// leaves is the multiples of sixty-four** (security S1b, finding A3): a
    /// 300-event record rebuilt through `from_record` — whose fold keeps the
    /// ladder as it goes — read at every seventh position against the engine's
    /// own positioning primitive, and the number of events each read FOLDED
    /// under sixty-four — the ladder's pin, since a fold from setup reaches
    /// the same state by the proof and shows only in the count; then the
    /// ladder's indices read back. Mutants: the ladder ignored (the count), an
    /// entry kept at the wrong index, a `fold_event` that skips the predicate
    /// (a different state is impossible — the proof — so the mutant for THAT
    /// arm is cell (c)).
    #[test]
    fn state_before_agrees_with_state_at_at_every_prefix_and_keeps_the_ladder() {
        let (record, rules) = random_record(3, 2026_1009, 300);
        assert!(record.events.len() >= 260, "the premise: a long record ({} events)", record.events.len());
        let mut session = Session::from_record(record.clone(), rules.clone()).expect("rebuilds");
        let len = record.events.len();
        let kept: Vec<usize> = (0..=len / CHECKPOINT_EVERY).map(|i| i * CHECKPOINT_EVERY).filter(|i| *i > 0 && *i <= len).collect();
        assert_eq!(session.ladder_indices(), kept, "from_record keeps a state at every sixty-fourth event");
        for k in (0..=len).step_by(7).chain([len]) {
            let (ours, folded) = session.state_before_counted(k).expect("a position");
            let theirs = state_at(&record, k, &rules).expect("the engine's position");
            assert_eq!(ours, theirs, "position {k}");
            assert!(folded < CHECKPOINT_EVERY, "position {k} folded {folded} events — the ladder was not used");
        }
        assert_eq!(session.ladder_indices(), kept, "reading positions adds no entry the fold already kept");
        assert!(session.state_before(len + 1).is_err(), "past the end is a refusal, never a clamp");
    }

    /// **A fresh session keeps a state at every sixty-fourth apply, and an
    /// undo drops every entry past the popped length and folds from the one
    /// below it** (finding A3). Sixty-five random applies; the ladder reads
    /// `[64]`; sixty-four undos leave the record at one event and the ladder
    /// empty, the state equal to the engine's replay of the popped record at
    /// every step. Mutant: `undo` not dropping the stale entry — the state
    /// after the sixty-fourth undo is the stale checkpoint's.
    #[test]
    fn apply_keeps_a_checkpoint_and_undo_drops_the_entries_it_pops_past() {
        let (seed_record, rules) = random_record(2, 20260831, 0);
        let mut session = Session::from_record(seed_record, rules.clone()).expect("an empty record rebuilds");
        let mut x = 7u64;
        for _ in 0..65 {
            let legal = session.legal();
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let mv = legal[(x % legal.len() as u64) as usize].clone();
            session.apply(mv, Vec::new()).expect("a listed move applies");
        }
        assert_eq!(session.record().events.len(), 65);
        assert_eq!(session.ladder_indices(), vec![64]);
        for expected_len in (1..65).rev() {
            session.undo().expect("something to undo");
            assert_eq!(session.record().events.len(), expected_len);
            let theirs = replay(session.record(), &rules).expect("the popped record replays");
            assert_eq!(session.state(), &theirs, "after the undo to {expected_len} events");
            assert!(session.ladder_indices().iter().all(|i| *i <= expected_len), "no entry past the record");
        }
        assert!(session.ladder_indices().is_empty());
    }

    /// **`fold_event` refuses a non-member in `apply_move`'s own words**
    /// (R-SEC2): a placement at a position that does not list it, through the
    /// predicate fold and through the engine's checked `replay_from`, is the
    /// same sentence — so a refusal a run or a rebuild reports reads as it
    /// always did. The placement is taken from the first position that offers
    /// one (the opening is the first additional-action phase and offers none)
    /// and the position from the first after it that does not list it.
    #[test]
    fn fold_event_refuses_a_non_member_as_the_checked_fold_does() {
        let (record, rules) = random_record(2, 4, 40);
        let (k, placement) = (0..=record.events.len())
            .find_map(|k| {
                let state = state_at(&record, k, &rules).expect("a position");
                get_legal_moves(&state, &rules)
                    .into_iter()
                    .find(|m| matches!(m, Move::PlaceThreeAreas { .. }))
                    .map(|m| (k, m))
            })
            .expect("the record reaches a Placement decision");
        let later = (k + 1..=record.events.len())
            .map(|j| state_at(&record, j, &rules).expect("a position"))
            .find(|s| !get_legal_moves(s, &rules).contains(&placement))
            .expect("a later position where the placement is not listed");
        let ev = RecordedEvent::Move(placement);
        let ours = Session::fold_event(&later, &ev, &rules).expect_err("refused");
        let theirs = replay_from(&later, std::slice::from_ref(&ev), &rules).expect_err("refused by the checked fold");
        assert_eq!(ours, theirs.message);
        assert_eq!(ours, MoveError::NotLegal.to_string());
    }
```

(`ladder_indices` is a `#[cfg(test)] pub(crate) fn ladder_indices(&self) -> Vec<usize>` on `Session`,
Step 3. Cell (c) takes its placement from the first position that offers one — the opening is the first
additional-action phase and offers none (the pre-flight's F18). The three cells import what they use — `new_game`, `finalize_setup`, `GameConfig`,
`StartingOrderChoice`, `get_legal_moves`, `apply_move_unchecked`, `is_game_over`, `state_at`, `replay`,
`replay_from`, `Move`, `MoveError`, `RecordedEvent` — through the file's `use yokohama_core::*`.)

(b) In `lib.rs`'s `mod tests`, beside the `/at` cells (find `at?from=` by content):

```rust
    /// **An out-of-range run is the route's own 404, and an empty run answers
    /// without a replay** (finding D27). `from = to = total + 1` read a 500
    /// "replaying this game failed … past the end" — the range test let it
    /// through and the replay refused it; it reads the range's 404 now, in the
    /// sentence `run_404_range.json` pins. `from = to ≤ total` is an empty run:
    /// 200 and no frames.
    #[tokio::test]
    async fn an_out_of_range_run_is_a_404_and_an_empty_run_is_empty() {
        let router = live_router();
        let game = a_game(&router, "ranges", &[]).await;
        let total = game["events"].as_u64().expect("events") as usize;
        let (status, v) = call(&router, "GET", &format!("/api/games/ranges/at?from={}&to={}", total + 1, total + 1), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{v}");
        assert_eq!(
            v["error"],
            format!("this game has {total} events, so its last position is {total} — there is no run from {} to {}", total + 1, total + 1)
        );
        let (status, v) = call(&router, "GET", &format!("/api/games/ranges/at?from={total}&to={total}"), None).await;
        assert_eq!(status, StatusCode::OK, "{v}");
        assert_eq!(v["frames"], serde_json::json!([]));
        let (status, v) = call(&router, "GET", "/api/games/ranges/at?from=0&to=0", None).await;
        assert_eq!((status, v["frames"].as_array().map(Vec::len)), (StatusCode::OK, Some(0)), "{v}");
    }
```

(c) In `lib.rs`'s `mod tests`, beside `the_record_route_refuses_a_finished_game_whose_stored_record_is_behind_the_handle`
(find by content) — the store double and the pin (S1a's residual, Review Focus 4):

```rust
    /// A finished ONE-person hot seat over `store` — `a_finished_two_person_game`'s
    /// shape (both seats pumped as bots in process, the record installed and
    /// re-stamped as two people), both seats [`WHO`]'s, so an undo at game over
    /// is the CLI's own: a one-person table is outside R-P6's shared-table
    /// refusal. Built this way because driving a hot seat to a winner over the
    /// route with `play_turn` never ends — its last-row policy passes every turn.
    async fn a_finished_one_person_game_over(store: Arc<dyn Store>, name: &'static str) -> Arc<AppState> {
        use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
        use yokohama_cli::session::Session;
        use yokohama_core::{GameConfig, PlayerColor, PlayerId, StartingOrderChoice};

        let state = Arc::new(AppState::with_store(Arc::clone(&store), identity::IdentityMode::Open));
        start_as(&state, name, ENDED_SEED, &[WHO, WHO]).await;
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let mut session = Session::new_sim(
            config,
            ENDED_SEED,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)],
            Rules::standard(),
        )
        .expect("a game");
        let bots = BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)], ENDED_SEED);
        session.set_run_seats(bots.seat_provenance());
        let mut repl = yokohama_cli::repl::Repl::new(session);
        for _ in 0..400 {
            if repl.session.is_over() {
                break;
            }
            repl.pump_bots();
        }
        assert!(repl.session.is_over(), "the both-bot pump must reach a finished game");
        repl.session.set_run_seats(BotSeats::new(vec![None, None], ENDED_SEED).seat_provenance());
        let finished = repl.session.to_record_json().expect("a record");
        let (_, version) = store.read(Doc::Record, name).expect("a store").expect("a record");
        store.write_if_version(Doc::Record, name, &finished, Some(version)).expect("the finished record");
        // A fresh state over the same store, so the game is rebuilt from the
        // record just written rather than served from the resident handle.
        Arc::new(AppState::with_store(store, identity::IdentityMode::Open))
    }

    /// A store that delegates everything to a `MemStore` and PARKS a record
    /// read while the flag is up — the instrument that reads a lock's scope:
    /// a request parked inside its read holds whatever lock it took to get
    /// there, and a second request that needs that lock waits.
    struct ParkedRead {
        inner: MemStore,
        parked: std::sync::Mutex<bool>,
        wake: std::sync::Condvar,
    }

    impl ParkedRead {
        fn new() -> Arc<Self> {
            Arc::new(Self { inner: MemStore::default(), parked: std::sync::Mutex::new(false), wake: std::sync::Condvar::new() })
        }
        fn park(&self) {
            *self.parked.lock().expect("the flag") = true;
        }
        fn release(&self) {
            *self.parked.lock().expect("the flag") = false;
            self.wake.notify_all();
        }
        fn wait_if_parked(&self) {
            let mut parked = self.parked.lock().expect("the flag");
            while *parked {
                parked = self.wake.wait(parked).expect("the flag");
            }
        }
    }

    impl Store for ParkedRead {
        fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, store::Version)>, String> {
            if doc == Doc::Record {
                self.wait_if_parked();
            }
            self.inner.read(doc, id)
        }
        fn list(&self, doc: Doc) -> Result<Vec<store::GameId>, String> {
            self.inner.list(doc)
        }
        fn write_if_version(
            &self,
            doc: Doc,
            id: &str,
            json: &str,
            expected: Option<store::Version>,
        ) -> Result<store::Version, store::WriteError> {
            self.inner.write_if_version(doc, id, json, expected)
        }
        fn delete(&self, id: &str) -> Result<(), String> {
            self.inner.delete(id)
        }
        fn last_written(&self, id: &str) -> Result<Option<u64>, String> {
            self.inner.last_written(id)
        }
    }

    /// **The record route reads the store UNDER the handle's lock** (S1a's
    /// residual, the parked-read double; Review Focus 4). A finished
    /// one-person game (built in process — `a_finished_one_person_game_over`)
    /// has its record read with the store's record read PARKED;
    /// the same person's `undo` is posted meanwhile — at a one-person table the
    /// CLI's undo applies at game over — and the read is released. Under the
    /// lock the undo WAITED: the record served is the finished one, its event
    /// count the handle's at the time and its digest present, and the undo
    /// lands after. Read outside the lock the undo lands during the park and
    /// the bytes served are a running record — the shape S1a's D11 closed by
    /// a comparison, which this cell holds to the lock's scope and not to the
    /// comparison alone: the mutant drops the guard before the store read.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn the_record_route_holds_the_lock_across_its_store_read() {
        let store = ParkedRead::new();
        let id = "parked";
        let state = a_finished_one_person_game_over(Arc::clone(&store) as Arc<dyn Store>, id).await;
        let (status, finished) = get_as(&state, WHO, &format!("/api/games/{id}")).await;
        assert_eq!(status, StatusCode::OK, "{finished}");
        assert_eq!(finished["over"], serde_json::json!(true), "the premise: the game is over");
        let total = finished["events"].as_u64().expect("events");
        store.park();
        let reader = {
            let s = Arc::clone(&state);
            tokio::spawn(async move { get_as(&s, WHO, &format!("/api/games/{id}/record")).await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let undo = {
            let s = Arc::clone(&state);
            tokio::spawn(async move { post_as(&s, WHO, &format!("/api/games/{id}/input"), serde_json::json!({ "line": "undo" })).await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert!(!undo.is_finished(), "the undo waits for the lock the parked read holds");
        store.release();
        let (status, record) = reader.await.expect("the read");
        assert_eq!(status, StatusCode::OK, "{record}");
        let served: yokohama_core::GameRecord = serde_json::from_value(record).expect("the record's own JSON");
        assert_eq!(served.events.len() as u64, total, "the finished record, not the undone one");
        assert!(served.terminal_digest.is_some());
        let (status, after) = undo.await.expect("the undo");
        assert_eq!(status, StatusCode::OK, "{after}");
        assert_eq!(after["events"].as_u64(), Some(total - 1), "and the undo landed after the read");
    }
```

(The double delegates `list`, `write_if_version`, `delete` and `last_written` to its `MemStore` — the block
carries them; the finished game is BUILT in process by `a_finished_one_person_game_over`, the lib's finished-game
shape over the parked store, because a hot seat driven "to the end" with `play_turn` never ends — its last-row
policy passes every turn (the pre-flight's F19, an hour in the loop). The record route serves the bytes as a JSON STRING body — `record`'s doc —
so `record.as_str()` is the whole file. If `get_as`'s JSON parse of a string body yields `Value::String`, the
line reads as written; if the helper returns `Null` for a non-object body, read the body through
`request_as`'s raw bytes instead — the implementer reports which.)

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-cli --lib -- state_before apply_keeps
fold_event_refuses 2>&1 | grep -E '^error|test result' | head` — `CHECKPOINT_EVERY`, `ladder_indices`,
`state_before` and `fold_event` not found; stubbed (`state_before` as `state_at` on `self.record`, the ladder
empty): the first cell RED at the ladder's indices, the second at `[64]`. `cargo test -p yokohama-web --lib --
out_of_range_run holds_the_lock 2>&1 | grep -E 'test result|panicked'` — D27's cell RED (500 where 404); the
parked-read cell GREEN on the fixed S1a code (the read IS under the lock since D11) — it exists for the mutant,
which the pre-flight reads RED (the guard dropped before the read: the undo lands, the served record is one short
or `RECORD_UNSAVED`), and its premise lines (`!undo.is_finished()`) are what make it non-vacuous.

- [ ] **Step 3: The code.** In `session.rs`, above `pub struct Session` (:67):

```rust
/// How often a session keeps a state (security S1b, finding A3): every
/// sixty-four events. A replay to any position then folds at most sixty-three
/// events from the nearest kept state instead of the whole record from setup
/// — the cost `/at`, a run, an undo and a rebuild paid, which at a record
/// built to a large retreat family ran to seconds per request. Sixty-four is
/// a memory choice, not a measured threshold: a `GameState` is tens of
/// kilobytes, so a 4,096-event record keeps sixty-four of them.
pub const CHECKPOINT_EVERY: usize = 64;
```

the field, after `checkpoint` (:80):

```rust
    /// The states kept at the multiples of [`CHECKPOINT_EVERY`] (security
    /// S1b, finding A3), ascending by index: `from_record`'s fold keeps them as
    /// it goes, `apply` keeps one when the record crosses a multiple,
    /// `state_before` keeps every multiple it folds past, and `undo` drops
    /// every entry past the popped length — an entry is valid for any record
    /// that shares the prefix it was cut from, so the ones at or below the
    /// new length stay. Never serialized: a rebuild re-cuts them.
    ladder: Vec<(usize, GameState)>,
```

every `Session { … }` literal gains `ladder: Vec::new(),` (`new_sim`, `new_live`, `for_test_from_state`); in
`from_record` the loop (:331–343) keeps the ladder and folds through the predicate:

```rust
        let mut event_descriptions = Vec::with_capacity(record.events.len());
        let mut ladder = Vec::new();
        for (i, ev) in record.events.iter().enumerate() {
            event_descriptions.push(crate::describe::describe_recorded_event(ev, &state, &rules));
            // The predicate fold (security S1b; `fold_event`'s doc): the same
            // states as `replay_from`'s checked apply, without the legal list
            // materialized per event. The message carries the TRUE index `i`
            // (Task 15 review Finding 1's lesson, kept).
            state = Self::fold_event(&state, ev, &rules).map_err(|e| format!("replay failed at event {i}: {e}"))?;
            if (i + 1).is_multiple_of(CHECKPOINT_EVERY) {
                ladder.push((i + 1, state.clone()));
            }
        }
```

and its literal (:380–390) gains `ladder,`. Inside `impl Session`, two associated functions (after
`is_over`, :516):

```rust
    /// One recorded event folded as an untrusted record's strict replay folds
    /// it (core's `replay_strict_with(.., MoveCheck::Predicate)`, R-SEC2): a
    /// `Move` through `is_listed_move` and then the raw transition
    /// `apply_move_unchecked`, every other event through core's `replay_from`
    /// — the same accepted set and the same states as the checked apply
    /// (`crates/yokohama-core/tests/replay_check_equivalence.rs`), refusing a
    /// non-member in `apply_move`'s own words, without the legal list
    /// materialized per event. What `from_record`'s rebuild, the ladder and
    /// `undo` fold with since security S1b; `apply` keeps its own gate, which
    /// materializes the list ONCE per interactive move.
    pub fn fold_event(state: &GameState, ev: &RecordedEvent, rules: &Rules) -> Result<GameState, String> {
        match ev {
            RecordedEvent::Move(mv) => {
                if !is_listed_move(state, mv, rules) {
                    return Err(MoveError::NotLegal.to_string());
                }
                apply_move_unchecked(state, mv, rules).map_err(|e| e.to_string())
            }
            other => replay_from(state, std::slice::from_ref(other), rules).map_err(|e| e.message),
        }
    }

    /// The state after the first `k` events (`state_at`'s domain, `0..=len`),
    /// folded from the nearest kept state at or before `k` — a test session's
    /// `checkpoint` base, the setup for every other — with every multiple of
    /// [`CHECKPOINT_EVERY`] passed on the way kept (security S1b, finding A3).
    /// `k` past the end is a refusal, never a clamp.
    pub fn state_before(&mut self, k: usize) -> Result<GameState, String> {
        self.state_before_counted(k).map(|(state, _)| state)
    }

    /// [`Self::state_before`] with the number of events it folded — fewer
    /// than [`CHECKPOINT_EVERY`] whenever the ladder is used, which is the
    /// cell's pin on the ladder (a fold from the wrong entry reaches the same
    /// state, by the proof, and shows only here).
    pub(crate) fn state_before_counted(&mut self, k: usize) -> Result<(GameState, usize), String> {
        let Session { rules, record, checkpoint, ladder, .. } = self;
        fold_to(record, rules, checkpoint.as_ref(), ladder, k)
    }

    /// The ladder's indices, for the cells that read it.
    #[cfg(test)]
    pub(crate) fn ladder_indices(&self) -> Vec<usize> {
        self.ladder.iter().map(|(i, _)| *i).collect()
    }

    /// Keep the state just committed when the record's length is a multiple
    /// of [`CHECKPOINT_EVERY`] — free, the state is in hand.
    fn keep_checkpoint(&mut self) {
        let len = self.record.events.len();
        if len.is_multiple_of(CHECKPOINT_EVERY) && self.ladder.last().is_none_or(|(i, _)| *i < len) {
            self.ladder.push((len, self.state.clone()));
        }
    }
```

the free function, after `impl Session`'s end:

```rust
/// [`Session::state_before`]'s body over the parts, so `undo` can fold the
/// record it has already popped. `ladder` is read for the nearest kept state
/// at or before `k` and extended at every multiple of [`CHECKPOINT_EVERY`]
/// the fold passes — inserted in order, never twice. Answers the state and
/// the number of events folded to reach it.
fn fold_to(
    record: &GameRecord,
    rules: &Rules,
    base: Option<&GameState>,
    ladder: &mut Vec<(usize, GameState)>,
    k: usize,
) -> Result<(GameState, usize), String> {
    let available = record.events.len();
    if k > available {
        return Err(format!(
            "state_before({k}) is past the end of the event log: the record carries {available} events"
        ));
    }
    let (mut at, mut state) = match ladder.iter().rev().find(|(i, _)| *i <= k) {
        Some((i, s)) => (*i, s.clone()),
        None => (0, match base {
            Some(b) => b.clone(),
            None => state_at(record, 0, rules).map_err(|e| e.message)?,
        }),
    };
    let from = at;
    while at < k {
        state = Session::fold_event(&state, &record.events[at], rules)
            .map_err(|e| format!("replay failed at event {at}: {e}"))?;
        at += 1;
        if at.is_multiple_of(CHECKPOINT_EVERY) {
            if let Err(pos) = ladder.binary_search_by_key(&at, |(i, _)| *i) {
                ladder.insert(pos, (at, state.clone()));
            }
        }
    }
    Ok((state, k - from))
}
```

`apply` (:620, after `self.state = next;`) and `apply_standalone` (:674, after `self.state = staged;`) each gain
`self.keep_checkpoint();`. `undo`'s fold (:732–735) becomes:

```rust
        let new_len = record.events.len();
        // The entries past the popped length belong to the suffix that is
        // gone; the ones at or below it are this record's (security S1b).
        self.ladder.retain(|(i, _)| *i <= new_len);
        let (state, _) = {
            let Session { rules, checkpoint, ladder, .. } = self;
            fold_to(&record, rules, checkpoint.as_ref(), ladder, new_len)?
        };
```

(the `replay`/`replay_from` imports stay — `replay_from` is `fold_event`'s and `replay` the cells' — and
`fold_to` answers the number of events it folded, which `state_before_counted` (`pub(crate)`) exposes for cell
(a)'s pin on the ladder: a fold from setup reaches the same state by the proof and shows only in that count —
the pre-flight's F24; `is_multiple_of` and `is_none_or` are what this toolchain's lint gate asks for — F27). `undo`'s doc paragraph (:699–712) ends: "…via core's
`replay_from` … (Task 13 …). **Since security S1b both paths are one: `fold_to`, from the nearest kept state of
the ladder (or the test base), through the predicate fold.**"

In `client.rs`, `at` and `run` (:695–728):

```rust
    /// Project `repl`'s record at `k`, from the session's nearest kept state
    /// (`Session::state_before`; security S1b, finding A3). The caller
    /// range-checks `k` first; an error from here is a genuine fold failure.
    pub fn at(repl: &mut Repl, k: usize, viewer: Viewer) -> Result<Self, String> {
        let state = repl.session.state_before(k)?;
        Ok(Self::from_state(repl, k, &state, viewer))
    }

    /// The frames of every position in `[from, to)`, by ONE forward pass: the
    /// state at `from` from the session's ladder, then one event folded per
    /// position through the predicate (`Session::fold_event`). The caller
    /// range-checks `from` and `to`. An error names the RECORD's index.
    pub fn run(repl: &mut Repl, from: usize, to: usize, viewer: Viewer) -> Result<Vec<Self>, String> {
        let mut state = repl.session.state_before(from)?;
        let mut frames = Vec::with_capacity(to.saturating_sub(from));
        for k in from..to {
            frames.push(Self::from_state(repl, k, &state, viewer));
            if k + 1 < to {
                let ev = &repl.session.record().events[k];
                state = Session::fold_event(&state, ev, repl.session.rules()).map_err(|e| format!("at event {k}, {e}"))?;
            }
        }
        Ok(frames)
    }
```

(`use yokohama_cli::session::Session;` joins the file's imports; the test module's two callers at :1102 and
:1113 take `let mut repl` and pass `&mut repl`; `tables.rs`'s two callers as the file list says.)
In `api.rs`, `run` (:2044–2069):

```rust
pub fn run(games: &Games, user: &User, id: &str, q: &RunQuery) -> Result<RunFrames, ApiError> {
    let (_table, viewer) = admitted(games, id, user)?;
    let handle = games.handle(id)?;
    let mut repl = games.lock_game(&handle, id)?;
    let total = repl.session.record().events.len();
    if q.to > q.from && q.to - q.from > RUN_CAP {
        return Err(ApiError::bad_request(format!(
            "a run is at most {RUN_CAP} positions — ask for from={}&to={} and page",
            q.from,
            q.from + RUN_CAP
        )));
    }
    // `from` past the last position is the same 404 as `to` past it (finding
    // D27): the range test let `from = to = total + 1` through to a replay
    // that refused it as a 500.
    if q.from > q.to || q.from > total || q.to > total + 1 {
        return Err(ApiError::not_found(format!(
            "this game has {total} events, so its last position is {total} — there is no run from {} to {}",
            q.from, q.to
        )));
    }
    // An empty run answers without a replay (finding D27).
    if q.from == q.to {
        return Ok(RunFrames { frames: Vec::new() });
    }
    let frames = client::HistoricalFrame::run(&mut repl, q.from, q.to, viewer)
        .map_err(|e| ApiError::server_error(format!("replaying this game failed {e}")))?;
    Ok(RunFrames { frames })
}
```

and `at_k` takes `let mut repl` and ends
`client::HistoricalFrame::at(&mut repl, k, viewer).map_err(|e| ApiError { status: StatusCode::INTERNAL_SERVER_ERROR, message: format!("replaying this game to {k} failed: {e}"), table_status: None, foreign_bot: None })`;
`run`'s doc (:2028–2043) gains "the 404 begins at `from = total + 1` as at `to = total + 2`, and an empty run
is answered without a replay (security S1b, finding D27)".

- [ ] **Step 4: Run, mutants, lint, commit.** `cargo test -p yokohama-cli 2>&1 | grep -E 'test result|FAILED'`
— 857 + 3, 2 ignored, the nine `assert_capture` goldens and the 18 `guide_sync` untouched (no screen moves);
`cargo test -p yokohama-web --lib` green; the lane's rows whole (the corpus does not move: `at_k`, `run` and
`spectator_at_k` are the same positions by the same states). Mutants: **M16** — `fold_to` ignoring the ladder
(always from the base): every state still agrees (the proof) and the indices still read as `from_record` kept
them, and the TIMING cannot carry it either (the pre-flight read Task 6's run at 125 ms against 46 ms under it —
inside any honest ceiling), so its catcher is cell (a)'s fold COUNT (`state_before_counted`, fewer than
sixty-four at every position; under the mutant the count is `k`); cell (a)'s second index assertion catches an
entry inserted twice; **M17** — `undo` not
dropping the entries past the popped length: cell (b) RED (a stale checkpoint's state after the sixty-fourth
undo); **M18** — `fold_event` skipping `is_listed_move`: cell (c) RED (accepted), and the CLI's own
`load_of_tampered_record_names_the_true_failing_event_index`, since `from_record` now refuses through it; **M19** — D27's `from > total`
dropped: the 404 cell RED (500); **M20** — the lock dropped before the record route's store read: the
parked-read cell RED. `cargo clippy --workspace --all-targets -- -D warnings` clean.

```bash
git add crates/yokohama-cli/src/session.rs crates/yokohama-web/src/client.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/tables.rs
git commit -m "cli+web: the session keeps a state every sixty-four events and folds through the predicate, so a position, a run, an undo and a rebuild never replay from setup through a materialized family; an out-of-range run is its 404 and an empty run replays nothing; the record route's lock scope pinned (A3's checkpoint, D27; S1b Task 5)"
```

---

### Task 6: A3 on the wire — the retreat family folded, the option list capped, the omitted count carried; the corpus once — Opus (A3; R-SEC8(e); spec corrections 2, 3; Review Focus 5)

**Files:**
- Modify: `crates/yokohama-web/src/client.rs` (a constant and two functions above `ClientState`; `ClientState`
  :134–194 — a field; `ClientState::of` :486–557 — the legal list; `HistoricalFrame` :625–676 — a field;
  `from_state` :733–800 — the legal list; cells in `mod tests`)
- Modify: `web/src/types.ts` (`ClientState` :325–351, `HistoricalFrame` :439–471 — the field),
  `web/src/wire/contract.ts` (`CLIENT_STATE` :617–621, `HISTORICAL_FRAME` :655–658 — the key)
- Regenerate: `web/src/wire/` ONCE (`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`)
- Test: `client.rs`'s `mod tests`; `lib.rs`'s `mod tests` (the spread table's three route cells); the client's
  typecheck and `npm test`; `npm run e2e`

**Interfaces:**
- Consumes: `yokohama_core::{families, iter_family, family_len, MoveFamily, get_legal_moves, retreat_len}`;
  Task 5's ladder (the spread table's rebuild and `/at` run are cheap only with it); the fixture pair; the lib
  helpers `get_as`, `post_as`, `request_as`.
- Produces: `client::OPTIONS_CAP`, `client::folded_legal`, `client::folded_legal_capped`;
  `ClientState.options_omitted`, `HistoricalFrame.options_omitted`; the corpus at its new shape.

- [ ] **Step 1: The failing cells.**

(a) In `client.rs`'s `mod tests`, at its end:

```rust
    /// The fold done by HAND over the generator's list — the reference the
    /// factored fold is held to: the first `ReturnPresidentToHand` kept, every
    /// other dropped and counted.
    fn fold_by_hand(legal: Vec<Move>) -> (Vec<Move>, usize) {
        let mut out = Vec::new();
        let mut retreats = 0usize;
        for mv in legal {
            if matches!(mv, Move::ReturnPresidentToHand { .. }) {
                retreats += 1;
                if retreats > 1 {
                    continue;
                }
            }
            out.push(mv);
        }
        (out, retreats.saturating_sub(1))
    }

    /// **The factored fold IS the generator's list folded by hand, at every
    /// decision of random play at two, three and four players** (finding A3;
    /// R-SEC8(e)) — the same moves in the same order with the retreat family
    /// at its recover-nothing member, and the omitted count the family's size
    /// less one. A lone retreat — no assistant on the board — is listed as it
    /// is and omits nothing (Review Focus 5): CONSTRUCTED, since placement
    /// precedes movement within a turn and a random walk all but never
    /// reaches a Movement decision with nothing on the board; from the
    /// opening, the first phase ended and nothing placed, the retreat family
    /// is one. Mutants: the last member kept instead of the first; the count
    /// off by one; a family skipped.
    #[test]
    fn folded_legal_is_the_generators_list_folded_by_hand_at_every_decision() {
        let rules = Rules::standard();
        let mut folded_some = 0usize;
        for (players, seed) in [(2u8, 11u64), (3, 13), (4, 17)] {
            let config = GameConfig::for_player_count(players);
            let choices = vec![StartingOrderChoice::Keep(0); players as usize];
            let pending = new_game(config, &rules, seed).expect("new_game");
            let mut state = finalize_setup(pending, &choices).expect("finalize_setup");
            let mut x = seed ^ 0x5DEE_CE66_D1CE_4A1F;
            for _ in 0..240 {
                if yokohama_core::is_game_over(&state) {
                    break;
                }
                let legal = yokohama_core::get_legal_moves(&state, &rules);
                let expected = fold_by_hand(legal.clone());
                let got = folded_legal(&state, &rules);
                assert_eq!(got, expected, "{players}P seed {seed}: the fold differs from the generator's list folded by hand");
                if got.1 > 0 {
                    folded_some += 1;
                }
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let mv = legal[(x % legal.len() as u64) as usize].clone();
                state = yokohama_core::apply_move_unchecked(&state, &mv, &rules).expect("a listed move applies");
            }
        }
        assert!(folded_some > 0, "anti-vacuity: the walk met a family it folded");

        // The lone retreat, constructed (Review Focus 5).
        let pending = new_game(GameConfig::for_player_count(2), &rules, 11).expect("new_game");
        let opening = finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]).expect("finalize_setup");
        let placing = yokohama_core::apply_move(&opening, &Move::EndAdditionalPhase, &rules).expect("the first phase ends");
        let nothing = Move::PlaceThreeAreas { placements: Vec::new() };
        let moving = yokohama_core::apply_move(&placing, &nothing, &rules).expect("an empty placement is always legal");
        let legal = yokohama_core::get_legal_moves(&moving, &rules);
        let retreats = legal.iter().filter(|m| matches!(m, Move::ReturnPresidentToHand { .. })).count();
        assert_eq!(retreats, 1, "the premise: nothing on the board, one retreat");
        assert_eq!(folded_legal(&moving, &rules), (legal, 0), "a lone retreat is listed as it is and omits nothing");
    }

    /// **The cap counts what it cuts** (R-SEC8(e)): the first Placement's
    /// list under a cap of three lists three and counts the rest, and the
    /// uncapped list is whole. The production cap is never reached by today's
    /// generator — this is the mechanism, read at a cap a cell can reach. The
    /// opening itself is the first additional-action phase and offers ONE
    /// move, so the cell walks to the first position that offers more.
    #[test]
    fn folded_legal_capped_counts_what_the_cap_cuts() {
        let rules = Rules::standard();
        let config = GameConfig::for_player_count(2);
        let pending = new_game(config, &rules, 20260831).expect("new_game");
        let mut state = finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)]).expect("finalize_setup");
        let mut whole = yokohama_core::get_legal_moves(&state, &rules).len();
        for _ in 0..8 {
            if whole > 3 {
                break;
            }
            let only = yokohama_core::get_legal_moves(&state, &rules).swap_remove(0);
            state = yokohama_core::apply_move(&state, &only, &rules).expect("the one move applies");
            whole = yokohama_core::get_legal_moves(&state, &rules).len();
        }
        assert!(whole > 3, "the premise: a position offering more than three moves ({whole})");
        let (listed, omitted) = folded_legal_capped(&state, &rules, 3);
        assert_eq!((listed.len(), omitted), (3, whole - 3));
        let (all, none) = folded_legal_capped(&state, &rules, OPTIONS_CAP);
        assert_eq!((all.len(), none), (whole, 0));
        assert_eq!(listed, all[..3].to_vec(), "the cap keeps the list's head");
    }
```

(b) In `lib.rs`'s `mod tests`, beside Task 4's cells — the spread table:

```rust
    const SPREAD_RECORD: &str = include_str!("../tests/fixtures/spread-4p-4096.json");
    const SPREAD_TABLE: &str = include_str!("../tests/fixtures/spread-4p-4096.table.json");
    const SPREAD_ID: &str = "local-spread4096";
    const SPREAD_PROVENANCE: &str = include_str!("../tests/fixtures/spread-4p-4096.provenance.json");

    /// The position of the spread DECISION — seat 0 to move in Movement, the
    /// 442,368-move family — as the pre-flight recorded it when it cut the
    /// fixture (`spread_decision_at` in the provenance file, 4,083): the
    /// record's END sits in seat 3's second additional-action phase, where
    /// nothing folds.
    fn spread_decision_at() -> usize {
        let v: serde_json::Value = serde_json::from_str(SPREAD_PROVENANCE).expect("the provenance file");
        v["spread_decision_at"].as_u64().expect("spread_decision_at") as usize
    }

    /// The spread fixture's pair written into a fresh store, as a hosted game
    /// the browser's own player holds every seat of — the review's hot-seat
    /// table, in the spread state it reached at event 419 and holds to 4,096.
    fn a_spread_table() -> Arc<AppState> {
        let store = Arc::new(MemStore::default());
        store.write_if_version(Doc::Record, SPREAD_ID, SPREAD_RECORD, None).expect("the record");
        store.write_if_version(Doc::Table, SPREAD_ID, SPREAD_TABLE, None).expect("the table");
        Arc::new(AppState::with_store(store, identity::IdentityMode::Open))
    }

    /// The DEBUG-profile ceilings the spread table's three reads are held
    /// under, in milliseconds — each five times the pre-flight's reading on
    /// the Mac inside the parallel lib run (the plan's §"Review record": the
    /// first read 62–65 ms alone and 130 ms in-suite; the run 46–48 ms; the
    /// undo 14 ms), rounded up: the first read pays the rebuild (4,096 events
    /// through the predicate fold), the run folds sixty-three events from the
    /// ladder and builds sixty-four frames from the factored source, the undo
    /// folds from the nearest kept state. What they catch is a MATERIALIZED
    /// replay or frame — seconds, 235 positions at a six-figure family — not
    /// the ladder, whose pin is the session cell's fold count.
    const SPREAD_FIRST_READ_MS: u128 = 700;
    const SPREAD_RUN_MS: u128 = 500;
    const SPREAD_UNDO_MS: u128 = 150;

    /// **A frame at the spread state is small, and says what it folded**
    /// (finding A3; R-SEC8(e)): the review read 58,304,650 bytes here — one
    /// worded option per legal move, 442,368 retreats among them. The frame
    /// now carries `options_omitted` in six figures and its bytes are under
    /// 256 KiB; the numbered rows are the REPL's grouped entries, a handful.
    /// The first read, which rebuilds the game from its record, is under its
    /// bound — the predicate fold's. A hot seat is viewed as the seat to move,
    /// so the POSITION's frame lists no option rows (seat 3 is to move at the
    /// record's end) and carries the count alone; undone to the decision, the
    /// LIVE frame is seat 0's and lists the recover-nothing retreat alone.
    #[tokio::test]
    async fn a_frame_at_the_spread_state_is_small_and_counts_what_it_folded() {
        let state = a_spread_table();
        let started = std::time::Instant::now();
        let (status, raw) = request_raw(&state, "local@browser", "GET", &format!("/api/games/{SPREAD_ID}")).await;
        let took = started.elapsed().as_millis();
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&raw[..raw.len().min(400)]));
        assert!(took < SPREAD_FIRST_READ_MS, "the first read took {took} ms");
        assert!(raw.len() < 256 * 1024, "the frame is {} bytes", raw.len());
        let frame: serde_json::Value = serde_json::from_slice(&raw).expect("JSON");
        assert!(frame["options_omitted"].is_u64(), "the field rides every frame: {}", frame["options_omitted"]);
        assert_eq!(frame["prompt"]["kind"], "options");
        assert!(frame["prompt"]["rows"].as_array().is_some_and(|r| r.len() < 40), "the REPL's grouped rows");
        // The spread DECISION itself, at its position in the record.
        let k = spread_decision_at();
        let (status, raw) = request_raw(&state, "local@browser", "GET", &format!("/api/games/{SPREAD_ID}/at/{k}")).await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&raw[..raw.len().min(400)]));
        assert!(raw.len() < 256 * 1024, "the position's frame is {} bytes", raw.len());
        let at: serde_json::Value = serde_json::from_slice(&raw).expect("JSON");
        assert!(at["options_omitted"].as_u64().unwrap_or(0) >= 100_000, "{}", at["options_omitted"]);
        assert_eq!(at["frame"]["decision"]["viewer_is_deciding"], false, "the viewer is the seat to move now, not seat 0");
        assert_eq!(at["frame"]["decision"]["options"].as_array().map(Vec::len), Some(0), "no rows for another seat's decision");
        // Undone to the decision: the live frame is seat 0's, and lists the fold.
        let total = frame["events"].as_u64().expect("events") as usize;
        for _ in k..total {
            let (status, _) = request_raw_with_body(&state, "local@browser", "POST", &format!("/api/games/{SPREAD_ID}/input"), serde_json::json!({ "line": "undo" })).await;
            assert_eq!(status, StatusCode::OK);
        }
        let (status, raw) = request_raw(&state, "local@browser", "GET", &format!("/api/games/{SPREAD_ID}")).await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&raw[..raw.len().min(400)]));
        assert!(raw.len() < 256 * 1024, "the decision's live frame is {} bytes", raw.len());
        let live: serde_json::Value = serde_json::from_slice(&raw).expect("JSON");
        assert_eq!((live["events"].as_u64(), live["to_move"].as_u64()), (Some(k as u64), Some(0)));
        assert!(live["options_omitted"].as_u64().unwrap_or(0) >= 100_000, "{}", live["options_omitted"]);
        let retreats = live["frame"]["decision"]["options"].as_array().expect("options").iter().filter(|o| o["mv"].get("ReturnPresidentToHand").is_some()).count();
        assert_eq!(retreats, 1, "the recover-nothing member alone");
    }

    /// **A run of sixty-four at the spread state is bounded** (finding A3): the
    /// review read 234,674,680 bytes and two seconds; from the session's
    /// ladder and the factored source it is under 8 MiB and under its bound.
    #[tokio::test]
    async fn a_run_at_the_spread_state_is_bounded() {
        let state = a_spread_table();
        let (status, _) = request_raw(&state, "local@browser", "GET", &format!("/api/games/{SPREAD_ID}")).await;
        assert_eq!(status, StatusCode::OK, "the rebuild");
        let started = std::time::Instant::now();
        let k = spread_decision_at();
        let (status, raw) = request_raw(&state, "local@browser", "GET", &format!("/api/games/{SPREAD_ID}/at?from={}&to={}", k - 63, k + 1)).await;
        let took = started.elapsed().as_millis();
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&raw[..raw.len().min(400)]));
        assert!(took < SPREAD_RUN_MS, "the run took {took} ms");
        assert!(raw.len() < 8 * 1024 * 1024, "the run is {} bytes", raw.len());
        let run: serde_json::Value = serde_json::from_slice(&raw).expect("JSON");
        assert_eq!(run["frames"].as_array().map(Vec::len), Some(64));
        assert!(run["frames"][63]["options_omitted"].as_u64().unwrap_or(0) >= 100_000, "the run's last frame is the spread decision: {}", run["frames"][63]["options_omitted"]);
    }

    /// **An undo at the spread table is bounded and its reply small** (finding
    /// A3): the review read 1.9 s and 58 MB; from the ladder it is under its
    /// bound, and the frame it answers with is the folded one.
    #[tokio::test]
    async fn an_undo_at_the_spread_table_is_bounded() {
        let state = a_spread_table();
        let (status, _) = request_raw(&state, "local@browser", "GET", &format!("/api/games/{SPREAD_ID}")).await;
        assert_eq!(status, StatusCode::OK, "the rebuild");
        let started = std::time::Instant::now();
        let (status, raw) = request_raw_with_body(&state, "local@browser", "POST", &format!("/api/games/{SPREAD_ID}/input"), serde_json::json!({ "line": "undo" })).await;
        let took = started.elapsed().as_millis();
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&raw[..raw.len().min(400)]));
        assert!(took < SPREAD_UNDO_MS, "the undo took {took} ms");
        assert!(raw.len() < 256 * 1024, "the reply is {} bytes", raw.len());
        let frame: serde_json::Value = serde_json::from_slice(&raw).expect("JSON");
        assert_eq!(frame["events"].as_u64(), Some(4095));
    }
```

with two raw helpers beside `request_as` (the frame's SIZE is the fact, and `request_as` parses the body):

```rust
    /// `request_as` with the body returned as BYTES, for the cells whose fact
    /// is a reply's size.
    async fn request_raw(state: &Arc<AppState>, who: &str, method: &str, uri: &str) -> (StatusCode, Vec<u8>) {
        let req = Request::builder().method(method).uri(uri).header(identity::CLAIMED_HEADER, who).body(Body::empty()).expect("a request");
        let res = app(Arc::clone(state)).oneshot(req).await.expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        (status, bytes.to_vec())
    }

    async fn request_raw_with_body(state: &Arc<AppState>, who: &str, method: &str, uri: &str, body: serde_json::Value) -> (StatusCode, Vec<u8>) {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header(identity::CLAIMED_HEADER, who)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).expect("a body")))
            .expect("a request");
        let res = app(Arc::clone(state)).oneshot(req).await.expect("the router answers");
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        (status, bytes.to_vec())
    }
```

(c) The client: `web/src/types.ts` — `ClientState` gains, after `bots`:

```ts
  /** Legal moves `frame.decision.options` does NOT list (security S1b): the
   *  retreat family's members beyond its recover-nothing one — a hot-seat
   *  table in the engine's worst-case state offered 442,368 of them, 58 MB a
   *  frame — and anything past the server's cap. The numbered rows are the
   *  prompt's and were always the REPL's grouped entries; the guided retreat
   *  (`kind: "retreat"`) is how a retreat is chosen. `0` on every frame
   *  outside a folded Movement decision. */
  options_omitted: number;
```

and `HistoricalFrame` the same field after `labs` with "`0` at every position outside a folded Movement
decision". `contract.ts`'s `CLIENT_STATE` list gains `"options_omitted"` after `"bots"`, `HISTORICAL_FRAME` after
`"labs"`. No component reads the field (the client renders `prompt.rows`, never `decision.options`); the
contract's root cells (`contract.test.ts:22–100`) read the two lists.

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --lib -- folded_legal
spread_state spread_table 2>&1 | grep -E '^error|test result|panicked' | head` — `folded_legal`,
`folded_legal_capped`, `OPTIONS_CAP` not found; stubbed (`get_legal_moves` with `0` omitted): the fold cell RED
at the first folded family; the cap cell RED; the frame cell RED on the bytes (the unfolded frame) — the run and undo cells read their pre-flight figures
under Task 5's ladder alone: the first read 62–65 ms alone and 130 ms in the parallel lib run, the run 46–48 ms,
the undo 14 ms (the ceilings are five times the in-suite readings, rounded up: 700 / 500 / 150 ms). `cd web && npm run typecheck` RED on `KeysOf` once the lists move and the
types do not (the order the implementer takes: types first).

- [ ] **Step 3: The code.** In `client.rs`, above `ClientState` (:132):

```rust
/// The most legal moves a frame lists (security S1b, finding A3; R-SEC8(e)) —
/// the retreat family folded to its recover-nothing member, the rest capped.
/// From measured play: outside the retreat family the largest list the
/// generator offers is a placement list — 189 moves at the opening of the
/// corpus walk, and over 200 uniform-random games at each count the
/// pre-flight's sweep read at most 432 (2P), 1,154 (3P) and 3,303 (4P, deep in
/// a game no person plays) — so 4,096 is above every legal list the generator
/// has produced.
/// The cap is the fence against a family no ceiling is known for (the routes'
/// label count, L9), not a budget; what it or the fold leaves out is COUNTED
/// in `options_omitted`, never silent. A hot-seat table in the engine's
/// worst-case state offered 442,368 retreats — 58 MB a frame — before this.
pub const OPTIONS_CAP: usize = 4_096;

/// The legal list a frame is built from (security S1b, finding A3): every
/// family of the factored action source in the generator's order, the retreat
/// family at its recover-nothing member alone — the family's size less one
/// counted, never materialized — and the whole capped at [`OPTIONS_CAP`].
/// Equal to the generator's list folded by hand
/// (`folded_legal_is_the_generators_list_folded_by_hand_at_every_decision`).
pub(crate) fn folded_legal(state: &GameState, rules: &Rules) -> (Vec<Move>, usize) {
    folded_legal_capped(state, rules, OPTIONS_CAP)
}

/// [`folded_legal`] at a cap of the caller's — the production cap's mechanism,
/// read by a cell at a cap it can reach.
pub(crate) fn folded_legal_capped(state: &GameState, rules: &Rules, cap: usize) -> (Vec<Move>, usize) {
    let mut listed = Vec::new();
    let mut omitted = 0usize;
    for family in families(state, rules) {
        if let MoveFamily::MovementRetreat { .. } = &family {
            let len = family_len(state, rules, &family);
            if let Some(first) = iter_family(state, rules, &family).next() {
                listed.push(first);
                omitted += len.saturating_sub(1);
            }
        } else {
            listed.extend(iter_family(state, rules, &family));
        }
    }
    if listed.len() > cap {
        omitted += listed.len() - cap;
        listed.truncate(cap);
    }
    (listed, omitted)
}
```

(`use yokohama_core::{families, family_len, iter_family, MoveFamily, Move, Rules};` joins the file's imports —
`families` yields the tags in the generator's order and `iter_family` each family's moves in it, the module's
own contract; the retreat family's first member is index 0 of its mixed-radix decode, every digit zero: the
recover-nothing move.)

`ClientState` gains, after `bots`:

```rust
    /// Legal moves `frame.decision.options` does NOT list (security S1b,
    /// finding A3): the retreat family's members beyond its recover-nothing one
    /// and anything past [`OPTIONS_CAP`] — counted, so a frame says what it
    /// folded rather than reading as the whole list. `0` on every frame
    /// outside a folded Movement decision.
    pub options_omitted: usize,
```

and `HistoricalFrame` the same after `labs`. In `ClientState::of`, the frame is built from the fold:

```rust
        let (legal, options_omitted) = folded_legal(state, session.rules());
        let mut frame = screen_model::build_frame(
            state,
            session.rules(),
            &facts_for(repl, viewer),
            seat,
            &legal,
            TitleModel { … as it is … },
        );
```

and the literal gains `options_omitted,`; in `from_state` the comment at :745–751 and the line are replaced:

```rust
        // **The fold, not the generator** (security S1b, finding A3): the
        // board's `tile.reachable` is derived from the list's `MovePresident`
        // destinations, which the fold keeps whole; what it folds is the
        // retreat family, which names no tile. `state_at` already folded the
        // record to this position through the predicate; this is the one
        // enumeration a frame pays, and it never materializes the family.
        let (legal, options_omitted) = folded_legal(state, rules);
```

with `options_omitted,` in its literal. The two `prompt`/`option_rows` paths are untouched: the numbered rows
were always the REPL's grouped entries (`PromptState`'s doc), and `Repl::option_rows` reads
`session.legal()` — the generator's whole list, materialized once per prompt inside the CLI's own code, which
this task leaves as it is (recorded in the report: the 43 ms and the 22 MB the CLI pays at the spread decision
are its own, and the frame no longer carries them).

- [ ] **Step 4: The corpus, once.** `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`,
then `git diff --stat web/src/wire web/src/wire-script` and `git diff web/src/wire | grep '^[-+] ' | grep -v
options_omitted | grep -v '^[-+]\s*[{}\[\]],\?$'` — READ it: every `ClientState`- and `HistoricalFrame`-bearing
payload gains `"options_omitted": 0` (the pre-flight counted N files), and three payloads move otherwise —
`retreat.json`, `retreat_answered.json` (four retreats → one, `options_omitted` 3) and `options_priced.json`
(three → one, 2) — with their option NUMBERS renumbered below the fold (`OptionModel::number` is the position
in the folded list). **Any other moved line is a finding** (a numeric field, a chosen flag, an order). The
report names every file and its reason.

- [ ] **Step 5: Run, mutants, lint, commit.** `cargo test -p yokohama-web` whole (the corpus green WITHOUT the
flag, `browser_played_pin`, `play_a_whole_game`'s two arms — its two-identity arm compares `board`, `title` and
the public dashboard, not the roots); `cargo test -p yokohama-wasm`, `-p yokohama-worker`; `cd web && npm run
build:wasm && npm run typecheck && npm test -- --run && npm run test:wasm`; `cd worker && npm run
build:module:debug && npm run typecheck && npm run test:scripts && npm test`; **`cd web && npm run e2e`** (the
client's types moved; 79 expected). The three spread cells run THREE times for their margins (the pre-flight's: above). Mutants: **M21** —
the fold keeping the LAST retreat member: the fold cell RED; **M22** — `omitted` not counting the family
(`len.saturating_sub(1)` → `0`): the fold cell RED, the frame cell RED at `options_omitted` and the run cell at
its last frame's; **M23** —
`from_state` back to `get_legal_moves`: the frame cell for `/at` (the run) RED on bytes and the bound; **M24**
— `ClientState::of` back to `session.legal()`: the frame cell RED on bytes. `cargo clippy --workspace
--all-targets -- -D warnings` clean.

```bash
git add crates/yokohama-web/src/client.rs web/src/types.ts web/src/wire/contract.ts web/src/wire
git commit -m "web: every frame is built from the factored action source with the retreat family folded to its recover-nothing member and the list capped, the omitted count carried as options_omitted; the corpus regenerated once (A3; S1b Task 6)"
```

---

### Task 7: The live corpus replay under the production seed rule — the production-door walk — Opus (S1a's live-replay residual; R-SEC8(b); spec correction 5)

**Files:**
- Modify: `worker/scripts/replay/exemptions.mjs` (a constant and three functions; the file's header)
- Modify: `worker/scripts/live-replay.mjs` (the header's steps; the loop :137–217; the result's fields :91–104)
- Modify: `worker/scripts/replay/judge.mjs` (`finish` :150–174 — the verdict and a report line)
- Modify: `worker/scripts/live-replay.test.mjs` (`fakeHost` :366–386 and `planIndex` :404; the driver cells
  :406–678 re-pointed to kept rows; new cells for the rule and the drift)
- Modify: `worker/RUNBOOK.md` (the live replay's section: what the production-door walk sends and skips)
- Test: `worker/scripts/live-replay.test.mjs` (under `npm run test:scripts`)

**Interfaces:**
- Consumes: `walk.json`'s create rows (`web/src/wire-script/walk.json` rows 0, 1, 2179: `wire2`, `wire` —
  seeded beside an `{open: true}` seat — and `wire4`, seeded with every seat the creator's); `LOBBY` (the four
  lobby fixtures); `tables::SEED_NOT_SOLO`'s text (`crates/yokohama-web/src/tables.rs:756`); the kept-row list
  the pre-flight printed (Task 9 Step 1).
- Produces: `SEED_NOT_SOLO`, `productionSeedRule`, `skippedUnderProductionSeedRule`, `isProductionSeedCreate`;
  `result.skipped`; the report's line and the verdict's arithmetic.

- [ ] **Step 1: The failing cells.** In `live-replay.test.mjs`, a new `describe` after `firstSendAccepts`'s
(:205–234):

```js
describe("productionSeedRule", () => {
  // The walk's three creates: `wire2` and `wire` seat an open seat beside a
  // seed, `wire4` seats its creator four times — the solo shape the rule honours.
  it("names the walk's seeded creates beside an open seat, and never the solo one", () => {
    expect([...productionSeedRule(SCRIPTS.walk)].sort()).toEqual(["wire", "wire2"]);
    const creates = SCRIPTS.walk.filter((r) => r.method === "POST" && r.path === "/api/games");
    expect(creates.map((r) => r.body.id).sort(), "the premise").toEqual(["wire", "wire2", "wire4"]);
  });
  it("names a seeded create beside another person too, and an unseeded one never", () => {
    const seeded = (seats) => [{ method: "POST", path: "/api/games", who: "red@wire.test", body: { id: "t", seed: "1", seats } }];
    expect([...productionSeedRule(seeded([{ user: "red@wire.test" }, { user: "blue@wire.test" }]))]).toEqual(["t"]);
    expect([...productionSeedRule(seeded([{ user: "red@wire.test" }, { bot: true }]))]).toEqual([]);
    expect([...productionSeedRule(seeded([{ user: "red@wire.test" }, { user: "red@wire.test" }]))]).toEqual([]);
    const unseeded = [{ method: "POST", path: "/api/games", who: "red@wire.test", body: { id: "t", seats: [{ user: "red@wire.test" }, { open: true }] } }];
    expect([...productionSeedRule(unseeded)]).toEqual([]);
  });
  it("skips every row at a skipped table and every lobby page, and never the create itself or a kept table's row", () => {
    const skipped = new Set(["wire", "wire2"]);
    const at = (path, fixture = null, method = "GET") => ({ method, path, fixture, body: null, who: "red@wire.test" });
    expect(skippedUnderProductionSeedRule(at("/api/games/wire"), skipped)).toBe(true);
    expect(skippedUnderProductionSeedRule(at("/api/games/wire/input", null, "POST"), skipped)).toBe(true);
    expect(skippedUnderProductionSeedRule(at("/api/games/wire2/table"), skipped)).toBe(true);
    expect(skippedUnderProductionSeedRule(at("/api/games", "games_open"), skipped)).toBe(true);
    expect(skippedUnderProductionSeedRule(at("/api/games", "games_next"), skipped)).toBe(true);
    expect(skippedUnderProductionSeedRule(at("/api/games/wire4"), skipped)).toBe(false);
    expect(skippedUnderProductionSeedRule(at("/api/health", "health"), skipped)).toBe(false);
    expect(skippedUnderProductionSeedRule(at("/api/archive", "archive_post", "POST"), skipped)).toBe(false);
    const create = SCRIPTS.walk.find((r) => r.method === "POST" && r.path === "/api/games" && r.body.id === "wire");
    expect(skippedUnderProductionSeedRule(create, skipped), "the create is SENT, and held to its 400").toBe(false);
    expect(isProductionSeedCreate(create, skipped)).toBe(true);
    expect(isProductionSeedCreate(at("/api/games/wire"), skipped)).toBe(false);
  });
  // The sentence is `tables::SEED_NOT_SOLO`'s, read off the Rust source so
  // the two cannot drift (the edge rule's drift cell is the precedent).
  it("holds SEED_NOT_SOLO to the layer's own constant", () => {
    const source = readFileSync(new URL("../../crates/yokohama-web/src/tables.rs", import.meta.url), "utf8");
    const m = /pub const SEED_NOT_SOLO: &str =\s*"([^"]+)";/.exec(source);
    expect(m, "the constant is where the plan read it").not.toBeNull();
    expect(SEED_NOT_SOLO).toBe(m[1]);
  });
});
```

and, in `describe("replay")`, the driver's cells against the fake host rewritten for the production-door walk —
`fakeHost` plans an answer for every row the rule SENDS: a 400 `{ error: SEED_NOT_SOLO }` for a skipped
table's create, the committed answer for every kept row, and nothing for a skipped row; `planIndex` counts
sent rows alone. The first cell reads:

```js
  it("holds against a production host that refuses the two seeded creates and answers every kept row, and names every exemption it spent", async () => {
    const host = fakeHost();
    const { result, text } = await run(host);
    expect(result.mismatches).toEqual([]);
    expect(result.holds).toBe(true);
    const skipped = new Set(["wire", "wire2"]);
    const sent = host.rows.filter(({ row }) => !skippedUnderProductionSeedRule(row, skipped));
    expect(result.run, "every row the rule sends ran").toBe(sent.length);
    expect(result.skipped, "and the rest were skipped, counted").toBe(host.rows.length - sent.length);
    expect(result.run + result.skipped).toBe(result.total);
    expect(result.named).toBe(sent.filter(({ row }) => row.fixture !== null && !isProductionSeedCreate(row, skipped)).length);
    expect(result.spent.map((s) => s.split(":")[0]).sort()).toEqual(
      [
        "HOST_FIELDS health.archive",
        "HOST_FIELDS health.identity",
        "HOST_FIELDS health.open_table_ttl_days",
        "HOST_FIELDS health.signal",
        "HOST_FIELDS health.table_ttl_days",
        "HOST_FIELDS me.mode",
        "HOST_FIELDS me_signed_out.mode",
        "firstSendAccepts archive_post",
        "firstSendAccepts archive_post_abandoned",
        "normalizeHouseChat house_chat",
        "normalizeHouseChat house_chat_post",
        "houseDeleteRow house_chat_delete",
        "productionSeedRule wire",
        "productionSeedRule wire2",
      ].sort(),
    );
    expect(host.requests.filter((r) => /\/api\/games\/wire2?(\/|$)/.test(r.url) && !r.url.includes("/api/admin/"))).toEqual([]);
    expect(text).toContain("rows skipped under the production seed rule: ");
    expect(text).toContain("live-replay: HOLDS");
  });

  // A host that HONOURED a seeded create beside an open seat is not under
  // the production rule — a diagnostic configuration deployed by mistake.
  it("refuses a host that honours a seeded create beside an open seat", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        const i = planIndex(rows, "walk.json", 1);
        expect(rows.find((r) => r.script === "walk.json" && r.index === 1).row.body.id, "the premise").toBe("wire");
        plan[i] = () => json(201, { id: "wire" });
      },
    });
    const { result, text } = await run(host);
    expect(result.holds).toBe(false);
    expect(result.mismatches.map((m) => `${m.label.split(":")[0]} ${m.path}`)).toEqual(["walk.json row 1 (production seed rule)"]);
    expect(text).toContain("expected 400 in SEED_NOT_SOLO's words · got 201");
  });
```

The cells at :446–678 keep their names and intents and move to KEPT rows (the pre-flight's kept-row list, in the
review record — 26 rows of 2,213; the brief carries it verbatim): "sends the credential…" asserts `others.length`
is the base read plus the rows the rule sends (no longer `> 2000`) and walks `host.rows` filtered to them;
"reports a field that differs…" edits `walk.json` row 28 (`health`, kept): its `semantics_rev` + 1, the
mismatch `health semantics_rev`, the sentence "walk.json row 28: GET /api/health as red@wire.test — health
semantics_rev: committed 7 · got 8", and `result.run` read as `result.total - result.skipped`; "stops at a
status that differs…" uses `walk.json` row 43 (the house post, `POST /api/chat`, kept): the stopped request is
`POST ${ORIGIN}/api/chat`, its body's `text` is the premise and the report never prints it; the lobby re-read
cell is DELETED (the lobby pages are skipped; `filterLobby`'s own cells stay); "redacts a value…" edits row 28
(`health`) with a planted `seed` — `walk.json` has no `wire4` table read — the mismatch `health seed`; `planIndex`
counts SENT rows alone (`rows.filter(!skipped).findIndex(..)`), and a cell that read `rows[i - 1]` finds its row
by script and index instead; the rest edit rows the rule keeps (row 28, the archive walk's row 0, the base read).

- [ ] **Step 2: Run them to verify they fail.** `cd worker && npx vitest run scripts/live-replay.test.mjs 2>&1 |
tail -20` — the four `productionSeedRule` cells fail on the missing exports; the driver's first cell fails on
the fake host's shape (every row planned, 2,201 sent).

- [ ] **Step 3: The code.** In `exemptions.mjs`, after `FIRST_SENDS` (:26):

```js
/** `tables::SEED_NOT_SOLO` (`crates/yokohama-web/src/tables.rs`): the sentence
 *  a production host refuses a seeded create with when any human seat is not
 *  the creator's or a seat is open (security S1a, R-SEC8(b)). Held to the Rust
 *  source by `live-replay.test.mjs`. */
export const SEED_NOT_SOLO = "a seed is honoured only at a table whose every human seat is yours — leave it blank and the server draws one";

/**
 * The tables a PRODUCTION host cannot deal from the walk (security S1b, the
 * production-door walk): each create row that carries a seed and seats anyone
 * but its creator — an open seat, or a user other than its `who`. The pool
 * deals them under `YOKOHAMA_DIAGNOSTIC_SEEDS`; a deployed Worker answers the
 * create 400 in `SEED_NOT_SOLO`'s words, so the create is sent and held to
 * that answer, and every row at the table is skipped and counted.
 */
export function productionSeedRule(walk) {
  const ids = walk
    .filter((row) => row.method === "POST" && row.path === "/api/games" && row.body !== null && row.body.seed !== undefined)
    .filter((row) => (row.body.seats ?? []).some((s) => s.open === true || (typeof s.user === "string" && s.user !== row.who)))
    .map((row) => row.body.id);
  return new Set(ids);
}

/** The create of one of `skipped`'s tables: sent, and held to the 400. */
export function isProductionSeedCreate(row, skipped) {
  return row.method === "POST" && row.path === "/api/games" && row.body !== null && skipped.has(row.body.id);
}

/**
 * Whether a production host cannot be asked `row` under the rule: a row at
 * one of `skipped`'s tables (`/api/games/<id>` and everything under it, the
 * create excepted), or a lobby page, whose committed body names them.
 */
export function skippedUnderProductionSeedRule(row, skipped) {
  if (isProductionSeedCreate(row, skipped)) return false;
  const m = /^\/api\/games\/([^/?]+)/.exec(row.path);
  if (m !== null && skipped.has(m[1])) return true;
  return row.fixture !== null && LOBBY.has(row.fixture);
}
```

In `live-replay.mjs`: the header's step 3 reads "**Every row** of `walk.json` the production seed rule lets
through …: `wire` and `wire2` are seeded creates beside an open seat, which a production host answers 400
(`SEED_NOT_SOLO`) — the create is sent and held to that answer, every row at those tables and the lobby pages
that name them are skipped and counted (`productionSeedRule`, security S1b); the pool replays the whole walk
under its diagnostic switch"; `result` gains `skipped: 0`; before the loop:

```js
  const skipped = productionSeedRule(walk);
```

and inside it, after `houseDeleteRow` and the label, before the send:

```js
      if (skippedUnderProductionSeedRule(row, skipped)) {
        result.skipped += 1;
        continue;
      }
```

and after `result.run += 1`:

```js
      if (isProductionSeedCreate(row, skipped)) {
        const id = row.body.id;
        if (reply.status === 400 && parsed(reply)?.error === SEED_NOT_SOLO) {
          result.spent.push(`productionSeedRule ${id}: the seeded create beside an open seat answered 400 in SEED_NOT_SOLO's words`);
        } else {
          differs(label, null, { path: "(production seed rule)", reason: `expected 400 in SEED_NOT_SOLO's words · got ${reply.status}${said(reply)}` });
        }
        continue;
      }
```

(the `firstSendAccepts` block and the status check follow as they are). In `judge.mjs`, `finish`: `result.run
=== result.total - result.skipped` in the verdict, and a line after "rows run": ``log(`rows skipped under the
production seed rule: ${result.skipped}`)``. In `worker/RUNBOOK.md`'s live replay section (find "live-replay.mjs"
by content): one paragraph — what the production-door walk sends, what it skips and why, and that the whole
walk's live reading is the pool's and the two-browser tier's.

- [ ] **Step 4: Run, mutants, lint, commit.** `cd worker && npm run test:scripts` (537 + the four + the driver's
rewritten set — count recorded); `npm run typecheck`; `node scripts/live-replay.test.mjs`-style lint if the
repo runs one (`npm run lint` where it exists). Mutants: **M25** — `productionSeedRule` ignoring the open seat:
its first cell RED (`[]`); **M26** — the replay sending skipped rows anyway: the driver's first cell RED (the fake
host has no answer planned, `the fake host has no answer planned for …`); **M27** — the create's 201 accepted:
the "honours a seeded create" cell RED; **M28** — `finish` counting `run === total` as before: the first cell
RED (`DOES NOT HOLD`).

```bash
git add worker/scripts/replay/exemptions.mjs worker/scripts/live-replay.mjs worker/scripts/replay/judge.mjs worker/scripts/live-replay.test.mjs worker/RUNBOOK.md
git commit -m "worker: the live corpus replay is the production-door walk — the walk's seeded creates beside an open seat sent and held to SEED_NOT_SOLO's 400, their tables and the lobby pages skipped and counted (S1b Task 7; R-SEC8(b))"
```

---

### Task 8: The documents the code changed — Sonnet (the lane document, the spec's §9.4, CLAUDE.md's parenthetical, the self-host guide)

**Files:**
- Modify: `docs/web-lane.md` — §2 (:58, :65–69, :72–80: the counts at the code head, every row re-counted;
  :58's `wire_fixtures` paragraph: `options_omitted` on every frame payload and the fold); §3 (:136 the live
  replay row: the production-door walk); §4 (:159 the archive bullet: the twelve checks, the count cap, the body
  cap, the quota's 507, the retry answered before any replay; :152 the undo bullet: the ladder; a new bullet
  after :156 "Transport": **The NAS runs every dispatch on the blocking pool and the five replay routes behind
  a two-permit gate**, with the 503 and its header; :161 the explanation bullet: the two searches behind the
  gate); §13 (:686 `POST /api/archive`: "refused by count over 4,096 events, by size over 512 KiB (413), a
  caller's 257th record or 129th MiB a 507; a retry of a filed record 200 before any replay; 503 on the NAS
  past the gate"; :687 `…/at`: "the 404 at `from > total`, an empty run without a replay; 503 past the gate";
  :688 `…/at/{k}`, :689 `…/suggest`, :690 `…/why`: "503 on the NAS past the gate"; :668 `GET /api/games/{id}`:
  "`options_omitted` counts the retreat members and anything past the cap the frame does not list"); §15 (a
  line: the caps are plausibility bounds from measured games, never ceilings a feature may lean on)
- Modify: `docs/plans/2026-10-08-security-remediation-spec.md` — §9.4 appended, dated 2026-10-09: this plan's
  §"The decisions this plan takes" (D-1–D-8) and §"Spec corrections" (1–9), verbatim, under a heading "9.4 —
  2026-10-09, S1b's plan: the cost bounds' decisions and nine corrections"
- Modify: `CLAUDE.md` — the `game/` bullet's parenthetical "(an archive upload, a restored save — S1 wires the
  web layer to it)" reads "(an archive upload, a restored backup: `backup::check_pair`, since security S1b)";
  nothing else in that file (the close's standing sentence is Task 9's)
- Modify: `deploy/README.md` — :192 (`YOKOHAMA_WEB_ARCHIVE_DIR`'s row): "…as the record the CLI loads beside its
  table document; a browser's uploads are bounded — 4,096 events and 512 KiB a record, 256 records and 128 MiB
  a caller under `browser/<caller>/` (security S1b)"; :314 (the volume table's archive row): the same bound in
  a clause
- Test: `cargo test -p yokohama-web --test lane_document` (6: §13's rows and §16's ledger hold — §16 gains no
  row, the routes are the same five); `worker/scripts/lane-variables.test.mjs` under `npm run test:scripts` (no
  variable moved)

**Interfaces:**
- Consumes: the code head's readings (every count from Tasks 1–7's reports, re-counted by this task on the
  tree); this plan's two sections to transcribe.
- Produces: the documents at the code head; §9.4.

- [ ] **Step 1: Re-count on the tree.** `cargo test -p yokohama-web 2>&1 | grep 'test result'` per binary;
`cargo test -p yokohama-cli 2>&1 | grep 'test result'`; `cargo test -p yokohama-wasm`, `-p yokohama-worker`;
`cd web && npm test -- --run 2>&1 | tail -3 && npm run test:wasm 2>&1 | tail -3`; `cd worker && npm run
test:scripts 2>&1 | tail -3 && npm test 2>&1 | tail -3`; `ls web/src/wire/*.json | wc -l` (109: no file was
added). Every figure written is one this task read.

- [ ] **Step 2: The lane document.** The edits above, each a sentence or a clause in the row it names; §2's
header line ("The counts are security S1b's readings…") dated to this task's head.

- [ ] **Step 3: The spec's §9.4, CLAUDE.md, the self-host guide.** As listed. `grep -n 'S1 wires' CLAUDE.md`
reads nothing afterwards.

- [ ] **Step 4: Run, commit.** `cargo test -p yokohama-web --test lane_document`; `cd worker && npm run
test:scripts`; `grep -c options_omitted docs/web-lane.md` ≥ 2; `git diff --stat` names exactly the five files.

```bash
git add docs/web-lane.md docs/plans/2026-10-08-security-remediation-spec.md CLAUDE.md deploy/README.md
git commit -m "docs: the lane document's archive, undo, executor and route rows for S1b, the spec's §9.4 (the decisions and nine corrections), CLAUDE.md's parenthetical, the self-host guide's bounds (S1b Task 8)"
```

---

### Task 9: The pre-flight, the reviews, the suites, the close — ORCHESTRATOR (spec §5, §9.2)

- [ ] **Step 1: The pre-flight — the orchestrator's own "before" review.** On a `git archive` export of
  `security-s1` at `604d7269` (`scratchpad/export-604d7269`, `node_modules` and the BGA captures linked in):
  (a) **Cut the spread fixture** (D-2): a scratch Rust program over the export reads
  `docs/temp/security-review-2026-10-08/worst-case/spread_4p_11_body.json`, takes the record's first 4,096
  events, re-stamps the terminal digest through `replay_strict_with(.., Predicate)` (the same digest `replay`
  gives), writes it COMPACT as `crates/yokohama-web/tests/fixtures/spread-4p-4096.json`, writes
  `spread-4p-4096.table.json` as the body's table with four `{"kind": "human", "user": "local@browser", "name":
  null}` seats, `keeps: [1, 1, 1, 1]`, `id: "local-spread4096"`, `created_by: "local@browser"`, `status:
  "playing"`, `turn: null`, and `spread-4p-4096.provenance.json` naming the origin (the review's r2 body, seed
  11, the cut, the retreat family at the final state, the generator's policy in a sentence, and `spread_decision_at` — the greatest position at or below 4,096 at
  which seat 0 is to move in Movement, read off `state_at` while cutting; Task 6's cells address it), then checks the
  pair with `backup::check_pair` on the export (accepted; `events == 4096`; `over == false`) and with Task 2's
  checks applied (still accepted: `run` absent and no bot seats agree; the config is 4P's). Commit the three
  files on `security-s1` as the plan's pre-flight commit ("fixtures: the cap-sized spread record…") BEFORE Task
  1 is dispatched — the one commit of this cycle made by the orchestrator in the checkout before the dispatches.
  (b) **Measure every bound this plan states as "measured"**, on the export with the plan's code blocks applied
  (lifted by a script, never retyped — `s1b-before/blocks.py` as S1a's): the DEBUG-profile predicate replay of
  the fixture through `check_pair` (Task 1's `SPREAD_REPLAY_BOUND_SECS` = 5× it, rounded up); the `Listed`
  replay of the same, ONCE, with a timeout of ten minutes (the mutant's reading); the health read behind two
  inline replays on two workers (Task 4's stall cell under its mutant); the spread table's first read, run and
  undo (Task 6's three ceilings = 5× their readings); the unfolded frame's bytes (the Task 6 mutant's reading);
  the maximum non-retreat family over 200 random games at 2/3/4P (`OPTIONS_CAP`'s comment: "the pre-flight's
  sweep read at most N"). Write each reading into the plan's blocks (the constants) and into the review record.
  (c) **Execute every cell of Tasks 1–7** against the scratch implementation, read every named mutant M1–M28 red
  at the cell it names and nowhere else, run the TS and node cells (`vitest`, the typecheck, `test:scripts`),
  regenerate the corpus on the export and COUNT the files that gain `options_omitted` (Task 6 Step 4's N),
  and print the live replay's kept-row list (`node -e` over `walk.json` with `productionSeedRule` and
  `skippedUnderProductionSeedRule`: script, index, method, path, fixture) into the review record for Task 7's
  brief. Record any cell that cannot go red on its mutant, any name here the tree does not spell, and any
  sentence of the spec's rows or this plan the code contradicts; fix them in this plan at the source BEFORE Task
  1 is dispatched; the scratch is discarded with the export.

- [ ] **Step 2: The branch.** `security-s1` as it stands (S1a's close at `604d7269`); S1b's commits stack on
  it and on PR #49; nothing merges (spec §9.2 as the owner amended it).

- [ ] **Step 3: The dispatches.** Tasks 1–7 Opus, Task 8 Sonnet, each with an Opus reviewer who reruns the
  task's mutants on an export; the probe-and-name-concerns paragraph in every implementer brief; the SDD ledger
  under `docs/temp/security-review-2026-10-08/s1b-sdd/`; the heartbeat protocol for every wait past an hour. A
  `DONE_WITH_CONCERNS` correctness item is ruled on and fixed as a new commit before the review; a fix round
  quotes its hunks; a reported SHA is never amended.

- [ ] **Step 4: The per-edit suites at the code head**, in this order, every row rc 0, each count against the
  board's "Gates at HEAD": `cargo test -p yokohama-core` (unmoved: 1,587 / 29 ignored) · `cargo test -p
  yokohama-cli` (857 + Task 5's three, 2 ignored) · oracle · difftest · `eval_probe` · `solver` · bridge and its
  four node suites · `cargo clippy --workspace --all-targets -- -D warnings` · **the web lane's rows**
  (`docs/web-lane.md` §2): `cargo test -p yokohama-web` with `wire_fixtures` green WITHOUT `UPDATE_WIRE_FIXTURES`,
  `-p yokohama-wasm`, `-p yokohama-worker`, the public build's `identity::`, the `wasm32` check, both wasm
  builds, `web/`'s `build:wasm`, typecheck, `npm test` and `test:wasm`, `worker/`'s `build:module:debug`,
  typecheck, `test:scripts` and the pool — **and `npm run e2e`** (the client's types moved). The fifteen-gate
  roster is NOT owed: core and the workspace version are unmoved (spec §5.2). Record every count; a count that
  differs is a finding.

- [ ] **Step 5: The "after" review — the orchestrator's own**, whole-branch, on a `git archive` export of the
  branch head: the fences of §"Global Constraints" (`git diff 604d7269 -- crates/yokohama-core
  crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-worker
  crates/yokohama-wasm` empty; the CLI's diff confined to `session.rs`; `SEMANTICS_REV` 7); the corpus moved in
  exactly one commit (Task 6's), its diff read against the task report's account and the pre-flight's count;
  every mutant M1–M28 re-run; the argument of each row re-derived from the code, not from this plan — that
  `check_pair_with` counts before it reads and folds through `Predicate` at both arms, that every production
  constructor builds the gate at two and two seconds, that `folded_legal` never calls `get_legal_moves`, that
  the ladder's entries are valid for every record sharing their prefix, that the quota counts only
  `browser/` prefixes. The release-profile measurement over the review's 23,606-event body through the real
  binary — refused by count in milliseconds — recorded. One fix wave as new commits; a scoped re-review of the fix.

- [ ] **Step 6: The close's documents.** The ledger's seven rows (`docs/security-ledger.md`) rewritten `fixed
  <commit>` with the catcher named — A2 (the count cell, the bounded-replay cell, the retry cell), A3 (the fold
  cell, the three spread cells, the ladder cells), B12 (the quota cells, the 507 cell, the outbox cell), D6 (the
  stall cell, the gate cell), D25 (the six cells), D27 (the range cell), D29 (the doc paragraph, read in
  review); guidingpath's decision-log row for S1b's close and its current-state sentence; the board's Track 4
  (S1c next: B8, B9, D1, D7, D12, D26, D28, D29's `is_token` sentence, M-3 and the served `why` chart's pin with
  D12); `CLAUDE.md` at most one standing sentence, if one is genuinely new — the candidate: "The archive's
  bounds are plausibility bounds from measured games (4,096 events, 512 KiB, 256 records and 128 MiB a caller
  under `browser/`), the five replay routes run behind the NAS's two-permit gate and every dispatch on its
  blocking pool, and every frame's option list is the factored source's with the retreat family folded
  (`options_omitted` says what it left out) — a new cost on an untrusted record or a hostile table is measured
  against those knobs before it is accepted (security S1b, 2026-10-09)"; and this plan's §"Review record" with
  the readings. The project memory updated the same way.

- [ ] **Step 7: The PR.** `git push origin security-s1`; PR #49's description appended with S1b's section
  (the rulings: R-SEC8(e)(f) and the owner's two of 2026-10-09; the seven rows and their catchers; the corpus's
  one regeneration and what it moved; the suites' readings, the e2e included; the mutants' readings; the nine
  spec corrections; "merge at S1's wrap, after S1c"); a comment on the PR naming the S1b head. Nothing merges.

- [ ] **Step 8: The live tier, in a window (optional, the orchestrator's alone).** A test deploy of the branch's
  build to the NAS (`deploy/RUNBOOK.md` §3) and/or the Worker (`worker/RUNBOOK.md`, through the guard); then
  the probes from the owner's workstation: the review's 23,606-event body refused by COUNT on both hosts in
  milliseconds (spec §5.3's A2 probe, now true as written); a 513 KiB body's 413; S1a's three probes (a scratch
  table's `suggest` carries no invertible field; `why` withheld at a scratch vs-bot table; a seeded create
  beside an open seat a 400 on both hosts); the live corpus replay as the production-door walk, TWICE
  (`cd worker && node scripts/live-replay.mjs`), HOLDING; the quota guard first for anything on the Worker.
  Every probe that creates state removes it. Skipped if no window holds: owed at the stack's deploy.

- [ ] **Step 9: The hand-off into S1c's planning session.** The board's Track 4 rewritten for S1c: the stack's
  head, the rows S1c owns, the two residuals it takes (M-3; the served `why` chart's pin), the first step of
  the next session ("brainstorm nothing — write S1c's plan against `security-s1`'s head from spec §2.1's B8,
  B9, D1, D7, D12, D26, D28, D29's `is_token` sentence in this plan's shape; the merge at S1's wrap after it,
  on the owner's word"); the project memory updated the same way;
  `docs/temp/security-review-2026-10-08/s1b-sdd/` left in place with every report. The next session begins at
  the board.

## Review record

### The "before" review (Task 9 Step 1), 2026-10-09 — the orchestrator's own, on an export of `604d7269`

**Method.** A `git archive` export (`scratchpad/export-604d7269`; `web/node_modules`, `worker/node_modules` and `bridge-captures` linked in); this plan's code blocks lifted by a script (`docs/temp/security-review-2026-10-08/s1b-before/blocks.py`, never retyped) and spliced by exact anchors (`edit.py`, `apply_t<N>.py`, each anchor matching once); every named mutant applied by a sha-checked runner (`mutants.py`) over the WHOLE web lib (or the CLI lib, or the vitest file) so "red at its cell and nowhere else" is a reading, not a claim; the findings and every reading in `s1b-before/findings.md`, the logs under `s1b-before/logs/`. Readings are the export's own, debug profile unless said, on the Mac (Darwin 27.0.0 arm64). Every block this record names as corrected was transcribed BACK into this plan from the scratch's files, so the plan and the code it describes are the same bytes.

**The fixture (Step 1(a)).** Cut by a scratch example from the review's `spread_4p_11_body.json`: the first 4,096 of 23,606 events, the terminal digest re-stamped (`da476c9ac52c8169`) and verified through `replay_strict_with(.., Predicate)`; the record compact, 161,737 bytes; the table re-seated as four `local@browser` humans, `keeps [1,1,1,1]`, id `local-spread4096`, `turn` null; the provenance file names the origin, the generator (described from the review's report — its source was the reviewer's scratch, not retained), the cut, `spread_decision_at` 4,083 and the end state. Measured on the cut, against what this plan quoted from the review: seat 0's family reaches 442,368 at event **419** (not 1,633; every seat's at the same event); seat 0 is to move in Movement at 254 positions, **235** of them at a six-figure family — the cost under `Listed` — not "2,463 passing turns"; the greatest such position is **4,083**, where `get_legal_moves` lists 442,385 moves (442,368 retreats) in 13 ms release; the record ENDS in seat 3's `AdditionalActionSecond` (families `[442368, 256, 256, 256]`, areas occupied `[17, 8, 8, 8]`), not a Placement. `check_pair` on the export (the `Listed` replay at this head) accepts it in 5.8–8.3 s release; the predicate replay 11–16 ms release. Committed on `security-s1` before Task 1's dispatch.

**The findings** (F-numbers as `findings.md` carries them; each corrected in this plan at the source):

- **F15 — the early answer's placement (Task 1; the decisive finding).** Placed before every check, as drafted, it turned the committed archive walk's `archive_400_table` row — the landed record beside a table that fails its checks, status 400 — into a 200 `identical`: `wire_fixtures` read 8 of 17 cells RED, the corpus would have moved in Task 1, and on the Worker, whose receipt archive cannot read its files (`holds` is the trait's default `false`), the same row would stay 400 — two hosts answering a corpus row differently, and Task 7's live replay mismatching it. The spec's A2 row says "before REPLAYING"; the reshaped design meets it exactly: `check_pair_with` is a wrapper over `check_pair_filed(record, table, rules, filed)`, whose hook is asked once every cheap check has passed and before the replay, returning `PairFacts { held: true, .. }` with no replay paid; a restore passes `|_, _| None`; `archive_post` passes `Archive::holds` for both leaves and answers 200 `identical` on `held`. The retry cell's proof is an archive double that claims to hold everything: a record that does not replay reads 200 through it and the replay's 400 on a real empty archive; the same double beside a disagreeing table reads the agreement's 400. `wire_fixtures` back to 17 green, the corpus unmoved; `lib.rs`'s `the_archive_route_checks_and_files_a_browser_game` keeps its committed row (F3, first read as a cell to re-point, withdrawn).
- **F7 — the attribution rule dropped (Task 2).** "A browser's pair is its own local game" refused the corpus's eight archive rows and the live replay's, which post `red@wire.test`'s hosted tables by design; the review's D25 line asks for `validate_config`, `seat_disagreement` and the record mirror, never this; the archive attributes a file by the uploader's folder and the names inside a document are data. The rule, `NOT_A_LOCAL_PAIR`, its cell and M9 are out; spec correction 6 records the residual.
- **F16 — `OPTIONS_CAP` (Task 6).** "2,048 is never reached by a legal list of today's generator" was false: over 200 uniform-random games at each count, the largest REALIZED non-retreat legal list is 432 at 2P (seed 7, Placement, turn 601), 1,154 at 3P (seed 145, turn 944) and **3,303 at 4P** (seed 29, turn 1,928) — `family_len`'s upper bound agreed exactly. The cap is **4,096**, the next power of two above every measured list, so it stays the fence the plan intends; the comment carries the figures.
- **F8, F12** — the spread figures and the generator's provenance, above. **F11** — `SPREAD_REPLAY_BOUND_SECS` is **3**: the predicate replay through `check_pair` in debug took 0.49–0.55 s alone (three runs) and 0.345 s inside the parallel lib run; the `Listed` mutant took **198.08 s** (the whole lib 198.5 s against 86 s).
- **F1** `SPREAD_TABLE` was a dead constant in `api.rs`'s tests (the lint gate). **F2** the retry cell's control table (`players` 3) failed `check_shape` before `agrees_with`; `start_seat` 1 is the disagreeing table. **F4** M4 was invisible: the order cell sized its 413 probe from the constant it mutates; the cell now pins `ARCHIVE_BODY_LIMIT == 512 << 10` and sizes the probe at the literal. **F6** `no_digest` unused once the lockdown rows replace the digest row. **F9, F10** counts in comments that move. **F5** `games.rs` needs no `Quota` arm (a catch-all `Err(e)`). **F13** the TS cell's helper is `noteGameOver`. **F14** `HistoricalFrame::at` has two callers in `tables.rs` the plan did not name (one production, one cell).

**Readings, Tasks 1–4** (the lib count after each task; every mutant over the whole lib, red at the named cells ALONE unless said): Task 1 — lib **431** (426 + 5); M1 the count, restore and order cells; M2 the bounded cell; M3 (the hook deleted) the retry cell; M4 (after F4) the order cell. Task 2 — lib **437**; M5, M6, M7, M8, M10, M11 as the plan names them (M6/M7/M8 red the order cell too, as said); `mcts-v1` is foreign at this head. Task 3 — lib **439**; vitest `outbox.test.ts` 59; M29–M33 as named. Task 4 — lib **442**; the stall cell's health read behind two slow replays 0.40 / 0.55 / 0.71 ms (bound 400 ms), the gate's 503 within 53.9 / 53.8 / 55.1 ms of a 50 ms wait (bound 600 ms), three runs each; M12 (inline dispatch) the stall cell RED, the pause and the health read 1.05 s under the old adapter's body swapped in whole (the cell's clock moved before the pause — F21; 154 ms on the correct code, three runs); M13 and M15 the gate cell; M14 the gate cell at the move, the route cell's "never gated" rows and the stall cell (its health read waits for a permit).

**Readings, Tasks 5–7, and the gates on the scratch** (the whole lib after each task; mutants over the whole lib): Task 5 — the CLI suite **860** (832 lib = 829 + 3, 2 ignored), the web lib **444**; cell (c)'s placement taken from the first position that offers one (F18); the parked-read cell's game BUILT in process, never driven (F19: `play_turn`'s passing policy never ends a game — the chain sat in it an hour), its body read as the record's own JSON (F20); M16's catcher is cell (a)'s fold COUNT (F24; the run under M16 read 125 ms against 46, inside any honest ceiling), RED under the mutant, 831 green beside; M17 cell (b) alone; M18 cell (c) and the CLI's tampered-load cell; M19 the D27 cell alone; M20 the parked-read cell alone. Task 6 — the web lib **449**; the three spread cells three times each: the first read 60–67 ms alone / 127 ms in-suite, the run 47–48 / 102, the undo 14–19 / 31 — the ceilings 700 / 500 / 150 ms (five times the in-suite readings, rounded up; they catch a MATERIALIZED replay or frame, seconds, not the ladder); the cap cell walks to the first Placement (F22), the fold cell constructs the lone retreat (F25), the frame cell reads the decision LIVE after undoing to it, the position's frame carrying the count and no rows (F23); `OPTIONS_CAP` 4,096 (F16); the corpus regenerated ONCE: 27 payloads gain `options_omitted`, `retreat.json` (34 lines), `retreat_answered.json` (34) and `options_priced.json` (22) renumber under the fold, nothing else moves, `wire_fixtures` 17 green; M21 the fold cell and the run cell's ceiling (`.last()` walks the family); M22 the fold, frame and run cells; M23 the frame and run cells (bytes); M24 the frame cell. Task 7 — the rule keeps **26 of 2,213** rows (`wire`, `wire2` skipped); `test:scripts` **540** + 1 skipped after the 409 cell's premise (F26); M25 the rule's first cell, the driver's first and the honouring-host cell; M26 the driver's ten cells (the fake host has no answer for a skipped row); M27 the honouring-host cell alone; M28 the driver's first cell alone. The gates: `cargo clippy --workspace --all-targets -- -D warnings` CLEAN after F27 (`is_multiple_of`, `is_none_or`) and F28 (`is_some_and`); the five fenced crates and the Worker and wasm crates byte-identical to the checkout; the CLI's diff confined to `session.rs`; wasm **10**, Worker **77**, vitest **1,506**, `test:wasm` **353**, the pool **224**, both typechecks clean, `build:wasm` 1,241,394 B. Every code block of this plan was verified verbatim against the scratch after the corrections, bar the three that cannot be (Task 1's doc paragraph at its own stage, the tables cell's named builder that the fallback paragraph covers, and the `… as it is …` placeholder).

**The live replay's kept-row list (Task 7's brief carries it verbatim):**

```
skipped tables: wire, wire2; rows 2213 (walk 2201, host 4, archive 8)
walk-host.json row 0: GET /api/art as no one -> 200 art_manifest
walk.json row 0: POST /api/games as red@wire.test -> 201 (unnamed)  <- the create, held to its 400
walk.json row 1: POST /api/games as red@wire.test -> 201 (unnamed)  <- the create, held to its 400
walk.json row 28: GET /api/health as red@wire.test -> 200 health
walk.json row 29: GET /api/me as red@wire.test -> 200 me
walk.json row 30: GET /api/me as no one -> 200 me_signed_out
walk.json row 31: GET /api/labels as red@wire.test -> 200 labels
walk.json row 43: POST /api/chat as red@wire.test -> 201 house_chat_post
walk.json row 44: GET /api/chat as blue@wire.test -> 200 house_chat
walk.json row 48: DELETE /api/chat/1 as red@wire.test -> 204 house_chat_delete
walk.json row 2179: POST /api/games as red@wire.test -> 201 (unnamed)
walk.json row 2180: POST /api/games/wire4/keep as red@wire.test -> 200 (unnamed)
walk.json row 2181: POST /api/games/wire4/keep as red@wire.test -> 200 (unnamed)
walk.json row 2182: POST /api/games/wire4/keep as red@wire.test -> 200 (unnamed)
walk.json row 2183: POST /api/games/wire4/keep as red@wire.test -> 200 (unnamed)
walk.json row 2184: GET /api/games/wire4 as red@wire.test -> 200 options_4p
walk.json row 2185: GET /api/games/wire4/areas as red@wire.test -> 200 areas_4p
walk.json row 2200: GET /api/games/no-such-game as red@wire.test -> 404 error
archive-walk.json row 0: POST /api/archive as red@wire.test -> 201 archive_post
archive-walk.json row 1: POST /api/archive as red@wire.test -> 200 archive_post_identical
archive-walk.json row 2: POST /api/archive as red@wire.test -> 409 archive_409
archive-walk.json row 3: POST /api/archive as red@wire.test -> 201 archive_post_abandoned
archive-walk.json row 4: POST /api/archive as red@wire.test -> 400 archive_400_rev
archive-walk.json row 5: POST /api/archive as red@wire.test -> 400 archive_400_digest
archive-walk.json row 6: POST /api/archive as red@wire.test -> 400 archive_400_replay
archive-walk.json row 7: POST /api/archive as red@wire.test -> 400 archive_400_table
kept 26, skipped 2187, total 2213
```
