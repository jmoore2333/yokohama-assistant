# Security S1c — the plumbing: implementation plan, and the wrap of S1

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. Every code task subagent-driven, **Opus** for every implementer and every task reviewer (the
> owner's ruling of 2026-10-10 at the pre-flight's stop, over the 2026-10-09 draft tiers of Sonnet for Tasks 1,
> 3, 7 and 8: the reviewers' mutant runs and the after review carry the validation; Fable for nothing here), and a **Mythos** orchestrator that
> conducts the plan review at the top and the whole-branch review at the end ITSELF, on exports; the suites, the
> documents' close, the SDD ledger and the wrap of S1 are the orchestrator's too (Task 9). Steps use checkbox
> (`- [ ]`) syntax. **Read the spec first** (`docs/plans/2026-10-08-security-remediation-spec.md`: §1 the rulings
> R-SEC8, §2.1 the rows B8, B9, D1, D7, D12, D26, D28, D29, §6 the fences, §9.2 the stacked branch, §9.3 the
> cycle split, §9.4 the S1b residuals), then this plan's §"Names shared across tasks" — an implementer sees only
> its own task, and that section is how it learns what its neighbours produce. Every `file:line` here was read at
> `security-s1b`'s head `680737e5` (S1b's close; the code head `17085e72`); **find by content once an earlier
> task has moved a file.** The implementer PROBES the brief's code before its own review and reports
> `DONE_WITH_CONCERNS` when a sentence here proves false.

**Goal:** Close the eight plumbing rows of the security review and the four residuals the two earlier cycles
left — a per-id lock map that grows on refused writes (B8), API answers with no cache directive and a hosted id
that can end in a file extension (B9), a refused stream that keeps sending (D1), the watcher roster's cap (D7,
accepted), error bodies that echo untrusted values whole (D12), a caller segment that is not injective (D26), an
input refusal that echoes a megabyte (D28), a false doc sentence on `is_token` (D29), a `why` refusal that names
the reconstructed move to a requester the account is withheld from (S1a's M-3), the `events` route's hand-off
with no catcher and the replay permit taken before identity (S1b's residuals), and a false doc sentence in the
fenced CLI crate — then wrap S1: the version bump to 0.6.1, the fifteen-gate roster, a four-agent whole-stack
review, the merges to master, the deploy of both hosts in one window with the live acceptance and the archive's
rename, and the hand-off to S2.

**Architecture:** Six small bounds, each where its cost or its leak is made. (1) Every caller-supplied value an
error sentence repeats goes through ONE helper (`echo::echo`: at most 64 characters, escaped, a `…` after a
cut), every `ApiError` message is capped at 1,024 characters at construction, a gameplay line is at most 256
bytes with no control character and the refusal of a long one names its length and not its text. (2) A table's
per-id lock entry lives only while a writer holds it: every writer reads the table BEFORE taking the lock and
prunes the entry on the way out through one helper (`Games::with_table_lock`), and an `{id}` the store would
refuse is a 404 in `dispatch`, before any handler runs. (3) Every answer under `/api` carries
`Cache-Control: no-store` from one axum layer, and a creator-chosen table id with a dot in it is refused at
create. (4) A table stream ends after the frame that says the caller's admission is gone, the `events` route's
pool hand-off has a catcher, and a replay-route request with no identity takes no permit. (5) The archive's
caller segment is the readable segment it was, cut at 48, with a 12-hex-character digest of the whole address
appended — injective in practice, readable still — and the legacy mapping is kept as a function for the one-time
rename the deploy window performs. (6) A `why` mismatch refusal consults the same visibility read the account
does and names no move while the account is withheld.

**Tech Stack:** Rust 1.97.1 (`rust-toolchain.toml`): `crates/yokohama-web` (the layer, its route cells, one new
module `echo.rs`, a `segment` subcommand in `main.rs`), `crates/yokohama-worker` (three receipt cells),
`crates/yokohama-cli` (`screen_model.rs`, two doc comments, no behaviour); the committed wire corpus
`web/src/wire/` (one regeneration, Task 5's: the three archive replies' `path` carries the caller segment; nothing
else moves); the documents (`docs/web-lane.md`, the
spec's §9.5, both runbooks, `deploy/README.md`); the release bump's eight steps (`docs/web-lane.md` §7).

**Spec:** `docs/plans/2026-10-08-security-remediation-spec.md` — §2.1's rows B8, B9, D1, D7, D12, D26, D28,
D29 (`is_token`'s sentence); R-SEC8; §6 the fences; §9.2 the stacked branch; §9.3's cycle split (S1c's rows and
the two residuals S1a passed: M-3 and the served `why` chart's jsdom-only pin); §9.4's 2026-10-10 paragraph (S1b's
residuals 5, 6 and 8: `events`' hand-off, the permit before identity, `screen_model.rs`'s sentence). The
corrections this plan makes to it are §"Spec corrections (to be recorded as §9.5)" below — recorded forward in
the spec's §9 by Task 7.

## The owner's rulings for this cycle (2026-10-10), and what each binds

The brainstorm of 2026-10-10 settled seven questions; the plan transcribes them and nothing re-opens them.

1. **D7, the watcher roster's cap: `accepted`.** The 257th unlock answering 409 is the design (`tables.rs:756–760`,
   M6.3 §3.4): a roster that forgot its oldest entry would re-lock a watcher mid-game. Filling it needs 256
   distinct identities, which only the dev door (B6) or the public door (C1, C2) supplies — S2's rows. The ledger
   row reads `accepted` with the owner's sentence (Task 9 Step 6); no code moves; the cell that pins the 409 at
   the cap stays as it is.
2. **D26, the caller segment: readable plus a digest, and a rename.** The segment keeps its readable form (cut at
   48) and appends `-` and the first twelve hex characters of a SHA-256 over the whole normalized address. Every
   existing folder on the NAS and every key under `browser/` in R2 is renamed ONCE, by the orchestrator, in the
   deploy window, from the recipe Task 5 writes into both runbooks and the `segment` subcommand it adds to the
   server binary. No legacy read path is kept in the layer.
3. **B9: no dot in a creator-chosen id, and `no-store` under `/api`.** A hosted create refuses any id containing
   `.` rather than keeping Cloudflare's extension list current; the storage rule `is_safe_id` keeps admitting
   dots, so a table stored before this cycle still loads. Every answer under `/api` — the adapter's, the three
   transport routes', the art manifest's and the fallback's 404 — carries `Cache-Control: no-store`; the static
   release (`/assets/…`, `/art/…`, `/shell.bin`) keeps its own rule (`assets.rs`).
4. **D12 and D28: one echo rule.** One helper cuts an echoed caller value to 64 characters and escapes it; every
   site that formats one uses it; `ApiError`'s constructors cap any message at 1,024 characters. The gameplay line
   is at most 256 bytes and carries no control character; the refusal of a long line names the length, never the
   text. The pre-flight measures the longest legal line (Task 9 Step 1) and records the margin.
5. **M-3 and the `why` chart.** A mismatch refusal served to a requester the account is withheld from carries the
   typed kind and a fixed sentence naming no move. The served chart's real-browser pin stays a RECORDED residual:
   it is test coverage, not a finding, and the e2e tier already runs twelve minutes.
6. **The fenced CLI doc sentence: taken.** Two doc comments in `crates/yokohama-cli/src/screen_model.rs` are made
   true (Task 7); the fence is restated as "no CLI behaviour or user-visible text moves" and the CLI suite is the
   gate.
7. **The wrap of S1.** `security-s1c` is cut from `security-s1b`'s head with a stacked PR; the whole-stack review
   before the master merge is a **four-agent Sonnet workflow run, its findings verified by the orchestrator**
   (the owner's explicit opt-in to the Workflow tool for that step, 2026-10-10; in place of a Fable whole-stack
   review); the fifteen-gate roster runs at the stack's head, as a merge to master owes it; the workspace version
   bumps to **0.6.1** at the wrap (Task 8), since the CLI's `suggest` derivation and the web wire both changed in
   S1; **the deploys are approved** — both hosts in one maintenance window, with the live acceptance, the rename
   and the five configuration changes the board lists.

## The decisions this plan takes (the orchestrator's; the owner reads them at the plan review)

- **D-1 — the echo helper escapes without quoting, so every existing sentence is byte-identical for a short,
  clean value.** `echo(value)` returns the first 64 characters as `{:?}` renders a `str`, less its two quotes — NOT
  `str::escape_debug`, which writes `it\'s` and leaves a combining mark after the first character raw, so an
  apostrophe in a name would have read differently (the implementer's C1, 2026-10-10) — with a `…` appended when
  it cut; a site that quoted with `{x:?}` now writes `\"{x}\"` with `x = echo(x)` as a named argument, and
  `no such endpoint: {}` stays unquoted. The Worker's TS cells pin that 404's text through the module (`worker/test/worker.test.ts:677`,
  `open/door.test.ts:273`, `:280`), so a quoting change there would ripple across three suites for nothing.
- **D-2 — the message cap is TWO layers: the constructors, and the boundary.** A serde error names an unknown
  field verbatim (`deny_unknown_fields`), axum's path sentences name the segment, and a site the inventory misses
  is the one that bites. The first layer is 1,024 characters at `bad_request`/`not_found`/`server_error` and in
  the `From<GameError>` arms, so the in-memory sentence is bounded. The second (Task 1's fix round 1, 2026-10-10 —
  the owner's side note read the first layer's twenty-two holes: `api.rs`'s own struct literals, `busy()`,
  `unauthorized`, the Worker's one) is `api::error_value`, the ONE serializer behind `error_body`,
  `error_response` and `IntoResponse`, so whatever built an `ApiError` — a handler's literal included — its
  sentence leaves the layer capped, and the browser module (through `dispatch`) and the Worker module (through
  `error_body`) inherit it unedited. A private marker field was the alternative, refused: it would have forced an
  edit in the Worker crate (fenced until Task 5) and covered no host that serializes a message itself. The
  per-site 64 keeps the sentences readable. The one refusal written by hand outside both layers is the Worker's
  `identity_inner` 401 (its `lib.rs:381`, built from `User::parse`'s sentence, whose caller value the echo rule
  bounds) — recorded, S2's door rows' territory.
- **D-3 — B8's two halves are one helper and one check.** `Games::with_table_lock(id, body)` takes the lock,
  runs the body, drops its clone and prunes, so "prune on every exit" cannot be forgotten at a site; every writer
  reads the table before calling it (an absent table is a 404 that never touches the map); `tables.rs` then
  contains no `lock_for(` call, which a source-scan cell pins. The `{id}` check sits in `api::dispatch` after the
  match: an id `is_safe_id` refuses is a 404 in the words the layer already uses for an absent game — today it is
  the store's 500 (`matches`' doc, `api.rs:276–281`).
- **D-4 — the stream's end is a stream wrapper, not a dependency.** `tokio-stream` 0.1.19 has no inclusive
  `take_while`, and `futures-util` is not a direct dependency of the crate. A fifteen-line `UntilLast` stream over
  `Pin<Box<_>>` yields the frame marked last and then ends; the `then` closure marks both `end` arms.
- **D-5 — the permit is skipped when the identity did not resolve, and nothing else moves.** Public routes
  (`/api/health`, `/api/me`, `/api/labels`, `/api/art`) answer without identity through the layer, so the adapter
  cannot 401 before dispatch; the five replay routes all need identity, so a replay-route request whose identity
  is `Err` dispatches WITHOUT a permit and the layer answers its 401 having run no replay. The route's own
  refusals (a missing game, a stranger) stay under the permit: behind Access every caller is signed in, and a
  pre-read would be a second statement of the route's rule.
- **D-6 — the legacy segment is kept as a named function, in the layer, for the rename and for a cell.**
  `archive::legacy_caller_segment` is the pre-S1c mapping verbatim; `caller_segment` differs from it for EVERY
  address (a cell), so an unmigrated folder is detectable; the server binary's `segment <address>…` subcommand
  prints both for the window's recipe. No code reads a legacy prefix.
- **D-7 — the drifted record for M-3's route cell is built inside the cell**, from a vs-bot game the cell plays:
  the bot's last recorded move is swapped for another member of the legal list at that position, the record
  written back through the store, and a fresh `AppState` built over the same store (the restart that evicts the
  live handle). The pre-flight proves the construction reaches `Refusal::Mismatch` (Task 9 Step 1).
- **D-8 — the corpus moves ONCE, in Task 5, and nowhere else.** No frame, header or sentence a fixture carries
  changes for a short, clean value — except the archive route's reply, whose `path` names the caller segment:
  `archive_post.json`, `archive_post_identical.json` and `archive_post_abandoned.json` carry
  `browser/red_at_wire.test/…` and move to the digested segment (Task 5 regenerates once and reads the diff: three
  `path` values and nothing else). The `why` fixture that carries a `mismatch` (`why_refusals.json`) is a
  CONSTRUCTION — `wire_fixtures.rs::every_refusal_payload` builds the twelve from `explain::refusal_samples()`
  through `RefusalDto::of`, never through the route — so Task 6 moves nothing. A moved byte anywhere else is a
  finding. (The pre-flight's reading, 2026-10-10: the draft expected Task 6 to be the one candidate and Task 5
  none; the plan's own greps, run, read the opposite.)
- **D-9 — the Worker crate is NOT fenced this cycle.** Its receipt cells compute `caller_segment` dynamically
  but one pins the legacy cap (`receipt.rs:825–827`: `(70, 64)` and the stripped `.test`); Task 5 rewrites that
  premise. The five fenced crates of spec §6 (core, oracle, difftest, bridge, gen-data) and `yokohama-wasm` stay
  byte-identical.
- **D-10 — the version bump is a task of this cycle (Task 8), on the stack, before the roster**, so the roster,
  the workflow review and the merge all read the head that ships. The tag `v0.6.1` is cut on master after the
  merge (`docs/web-lane.md` §7).

## Spec corrections (to be recorded as §9.5 by Task 7, dated 2026-10-10)

1. **B9's hosted-id rule is "no dot", not "a known extension"** (the owner's ruling 3): the list is Cloudflare's
   to change and the client never sends an id.
2. **D7 is `accepted`, not fixed** (ruling 1), with the owner's sentence in the ledger.
3. **D26's segment is readable plus a digest, with a one-time rename on both hosts** (ruling 2); spec §2.1's
   "an injective caller segment" is met in practice (a 48-bit digest collision is the only collision left).
4. **S1b's residual list on the board was stale by two items**: the live replay's lobby skip IS guarded on an
   empty rule and its 26 kept rows ARE pinned (`worker/scripts/live-replay.test.mjs:297`, the final wave
   `17085e72`, W9 and W10). S1c does not take them; the board is corrected at the close.
5. **The `{id}` an unsafe path segment carries is a 404, no longer the store's 500** (D-3); `matches`' doc at
   `api.rs:276–281` is rewritten to say so.
6. **The CLI is touched in ONE file this cycle, `screen_model.rs`, two doc comments** (ruling 6) — spec §6's
   "the CLI is touched only at `suggest::stream_seed`" is read, as S1b read it for `session.rs`, as "nowhere the
   bot decides and nowhere the user sees".
7. **The served `why` chart's real-browser pin stays recorded** (ruling 5).
8. **The corpus moves once in S1c, in Task 5, not in Task 6** (the pre-flight, 2026-10-10): the archive route's
   reply names the caller segment in `path`, so the three `archive_post*.json` fixtures regenerate once; the
   `why_refusals.json` `mismatch` row is a construction and Task 6 moves nothing (decision D-8). Still one
   regeneration per cycle, as §9.3's correction 1 allows — a value moved, not the wire's shape.

## Global Constraints

- `SEMANTICS_REV` (`crates/yokohama-core/src/game/semantics.rs`) stays **7**; `crates/yokohama-core`,
  `crates/yokohama-oracle`, `crates/yokohama-difftest`, `crates/yokohama-bridge`, `crates/gen-data` and
  `crates/yokohama-wasm` are byte-identical to `680737e5` (`git diff 680737e5 -- crates/yokohama-core
  crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-wasm`
  empty at every commit). `crates/yokohama-worker` moves in Task 5 alone (three receipt cells).
- The CLI is touched in ONE file, `crates/yokohama-cli/src/screen_model.rs`, in Task 7, two doc comments — no
  behaviour, no user-visible text, so the guide pair does not move; `cargo test -p yokohama-cli` reads **862** and
  2 ignored at every commit.
- No bot behaviour moves: `decide`, `suggest`'s answer, `why`'s reconstruction (`why::account_parts`) and every
  arena figure are untouched; Task 6 changes what a REFUSAL says to a withheld requester, not what the search
  finds.
- The wire corpus (`web/src/wire/`) moves ONCE, in Task 5 (the three archive replies' `path`, decision D-8), run
  once, its diff read and recorded naming every file; any other moved byte, in any task, is a finding.
- Every refusal this plan adds is a plain sentence, a constant or a builder in the module that raises it, through
  the layer's own `ApiError` arms (400, 401, 404) — no new status shape; the `no-store` header is the ADAPTER
  host's (axum's), as the 503 is.
- Every constant carries the owner's comment rule from S1b: a concise in-code comment defending the figure
  (`ECHO_MAX`, `MESSAGE_MAX`, `LINE_MAX`, the segment's 48 and 12). The comments in this plan's blocks are that
  text; an implementer transcribes them.
- TDD: the failing cell first, then the code, then green; `cargo clippy --workspace --all-targets -- -D warnings`
  clean after every task; no `cargo fmt`; stage with explicit paths, never `git add -A`; one commit per task at
  its end, the task's suites green at that commit; a reported SHA is never amended — every later change is a new
  commit.
- The branch is `security-s1c`, cut from `security-s1b` at `680737e5` by the orchestrator (Task 9 Step 2); its PR
  is stacked on PR #50; nothing merges before Task 9's wrap (the owner, 2026-10-09 and 2026-10-10).
- Every task that touches what the layer compiles runs the lane's rows (`docs/web-lane.md` §2) whole before its
  commit — `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`; `cd web && npm run build:wasm
  && npm run typecheck && npm test -- --run && npm run test:wasm`; `cd worker && npm run build:module:debug && npm
  run typecheck && npm run test:scripts && npm test` — and records every count; Task 7 runs `cargo test -p
  yokohama-cli` too; Task 8 runs everything, `npm run e2e` included.
- Every dispatched agent: the full report to a file under `docs/temp/security-review-2026-10-08/s1c-sdd/`
  (gitignored), a three-line reply; no `rm -rf`, `rm -r` or bulk `rm -f`; no Monitor, no event waits; long
  commands in the foreground or polled; reviewers run their mutants on a `git archive` export, never in the
  implementer's tree.

## Review Focus

Five inputs the spec implies and this plan pins, each to the task that owns the code:

1. **The longest legal gameplay line** — a 4P retreat naming every area the mover has an assistant in, with its
   count (`r A17:5 A16:5 …`). It must pass the 256-byte bound with room. Task 1's cell composes it from the CLI's
   retreat spelling (the web client composes no retreat line — its longest line is an area token, `App.tsx:2424`)
   and asserts it passes; measured 99 bytes (109 zero-padded).
2. **A table stored before this cycle whose id carries a dot** (`old.js`). It still loads, plays, streams and
   deletes; only a CREATE refuses dots. Task 3's cell writes such a table through the store and drives it.
3. **A watcher whose admission is revoked mid-stream.** The stream writes `end` with `gone` and the connection
   CLOSES; a delete still closes with `deleted`; a transient store failure still writes a comment and stays open.
   Task 4's cells.
4. **Two addresses that collided under the legacy segment** (`alice+x@corp.test`, `alice_x@corp.test`;
   `a b`, `a_b`). They get two folders and two quotas now, and the legacy folder of an unmigrated address is never
   counted as the caller's. Task 5's cells.
5. **A `why` on a drifted record asked by the bot's opponent mid-game.** The refusal names no reconstructed move;
   the same refusal to a seat holder at game over names both. Task 6's cells.

## Execution shape

- Every implementer and every reviewer **Opus** (the owner's ruling of 2026-10-10, at the pre-flight's stop).
  Task 9: the orchestrator, **Mythos**, who conducts the "before" plan review (executing Tasks 1–6's cells against
  scratch implementations of this plan's code in an export, measuring the longest line, building the drifted
  record, reading the two `why` fixtures, reading every named mutant red) and the "after" whole-branch review
  itself, then the wrap.
- Every implementer dispatch carries the probe-and-name-concerns paragraph: try the mechanism on cases the brief's
  cells do not cover, with vacuity as the first lens — for each cell, which mutation makes it fail? — and report
  `DONE_WITH_CONCERNS` naming any sentence of this plan the code contradicts; a `DONE_WITH_CONCERNS` correctness
  item is ruled on and fixed as a new commit BEFORE the task's review. Every fix-round dispatch requires the hunks
  quoted and `git diff` run before the commit.
- The SDD ledger (`docs/temp/security-review-2026-10-08/s1c-sdd/progress.md`) records each task's tier, its
  commit, its review verdict and every `Ruling:` line, as S1b's did.

## Names shared across tasks

| name | where | meaning |
|---|---|---|
| `echo::ECHO_MAX: usize = 64` · `echo::echo(value: &str) -> String` | `crates/yokohama-web/src/echo.rs`, `pub`, re-exported as `crate::echo` | a caller-supplied value as an error sentence repeats it: the first 64 characters as `{:?}` renders them, less the quotes, `…` after a cut (Task 1) |
| `MESSAGE_MAX: usize = 1_024` · `ApiError::capped(message: String) -> String` | `crates/yokohama-web/src/lib.rs` beside `ApiError` (:122) | every constructor's message bound (Task 1) |
| `grammar::LINE_MAX: usize = 256` · `grammar::check_line(line: &str) -> Result<(), String>` | `crates/yokohama-web/src/grammar.rs`, `pub` | the byte bound and the control-character rule the `input` route asks BEFORE `is_gameplay_line`; the sentence names the length (Task 1) |
| `Games::with_table_lock<T>(&self, id: &str, body: impl FnOnce() -> T) -> T` | `crates/yokohama-web/src/games.rs`, `pub(crate)`, beside `lock_for` (:616) | the one way a writer holds a table's own lock: take, run, drop, prune (Task 2) |
| the `{id}` check in `api::dispatch` | `api.rs:2503` (after `find`) | an `{id}` segment `is_safe_id` refuses is `not_found("no game named \"<echo>\"")` before any handler (Task 2) |
| `no_store` (an `axum::middleware::from_fn` layer) | `crates/yokohama-web/src/lib.rs`, `app()` (:1114) | `Cache-Control: no-store` on every answer whose path is `/api` or under `/api/` (Task 3) |
| `tables::DOTTED_ID: &str` | `tables.rs` beside `SEED_NOT_SOLO` (:756) | the sentence a dotted creator-chosen id is refused with (Task 3) |
| `UntilLast<S>` | `lib.rs` beside `stream` (:804) | the SSE stream wrapper that ends after the frame marked last (Task 4) |
| `archive::caller_segment(user: &User) -> String` (changed) · `archive::legacy_caller_segment(user: &User) -> String` (new) · `archive::SEGMENT_READABLE: usize = 48` · `archive::SEGMENT_DIGEST: usize = 12` | `archive.rs:216` | the new segment and the pre-S1c one (Task 5) |
| `yokohama-web segment <address>…` | `crates/yokohama-web/src/main.rs` | prints `address`, `legacy`, `current` per address, for the rename recipe (Task 5) |
| `explain::MISMATCH_WITHHELD: &str` · `explain::Why::refused_at(r: &Refusal, now: &GameState, held: &[PlayerId]) -> Why` | `explain.rs` beside `refused` (:529) | the mismatch refusal at the requester's visibility (Task 6) |
| the route-cell helpers `post_as` (:2700), `get_as` (:2709), `delete_as` (:2717), `start_as` (:3104), `start_locked_as` (:3137), `play_turn` (:3182), `play_turn_vs_bot` (:3257), `a_finished_vs_bot_game` (:4481), `stream_frames` (:7423), `sse_frames` (:7476), `app`, `WHO`, `ENDED_SEED`, `slow_state` (:2754), `SlowArchive` (:2725), `archive_body` | `crates/yokohama-web/src/lib.rs` `mod tests` | what Tasks 2–6's route cells reuse |
| `yokohama_core::{GameRecord, RecordedEvent, state_at, get_legal_moves, Move}` | the crate root re-exports | what Task 6's drifted-record cell reaches for; none is edited |

## File structure

- Create `crates/yokohama-web/src/echo.rs` (the helper, its cells); modify `lib.rs` (`pub mod echo;` and
  `pub use echo::echo;` with the other modules at :60–75; `ApiError` :122–160 — `MESSAGE_MAX`, `capped`; the
  fallback's `no such endpoint` :1058), `api.rs` (:1752 the index kind, :1757 the filter term, :2252 `no game
  named`, :2511 and :2630 `no such endpoint`, :2644 and :2657 the path sentences, :1335 the line bound),
  `tables.rs` (every non-test `{id:?}` and `{key:?}`, the seed's `{t:?}`, the scope's `{other:?}`), `games.rs`
  (eight `{id:?}`), `store.rs` (four `{id:?}`, "is not a usable game name"), `identity.rs` (:169,
  :172, :184, :189), `grammar.rs` (:102 `is_token`'s doc; :148 the refusal's echo; `LINE_MAX`, `check_line`) —
  Task 1.
- Modify `games.rs` (`lock_for` :604–619 doc, `prune_lock` :621–636, `with_table_lock` new), `tables.rs`
  (`chat_append` :1061–1080, `delete_chat` :1133–1147, `unlock` :1352–1360, `settle` :1395–1402, `join`
  :1574–1588, `keep` :1659–1672, `delete` :1905–1917), `api.rs` (`dispatch` :2503–2510; `matches`' doc
  :276–281) — Task 2.
- Modify `lib.rs` (`app()` :1114–1134 — the layer; `no_store` new), `tables.rs` (`create` :826–830 — the
  dotted arm; `DOTTED_ID`) — Task 3.
- Modify `lib.rs` (`stream` :804–849 and its doc :761–803; `UntilLast` new; `adapter` :929–950 — the permit's
  condition; `events` :735–745 unchanged, its cell new) — Task 4.
- Modify `archive.rs` (`caller_segment` :196–232, `legacy_caller_segment` new, the two constants; the cells
  :575–605, :721–745), `main.rs` (the `segment` subcommand), `crates/yokohama-worker/src/receipt.rs` (the premise
  at :825–827; the doc at :34), `deploy/RUNBOOK.md` and `worker/RUNBOOK.md` (the rename recipe),
  `deploy/README.md:192` and `:314` (the path), `worker/test/archive.test.ts` and
  `worker/scripts/live-replay.test.mjs` (the hand-written legacy segments), the three `archive_post*.json`
  corpus files and `worker/test/corpus.gen.ts` (regenerated) — Task 5.
- Modify `explain.rs` (`refused` :529; `refused_at`, `MISMATCH_WITHHELD` new; a unit cell beside :1142), `api.rs`
  (`why` :1706 — the `Err` arm) — Task 6.
- Modify `docs/web-lane.md` (§2, §4, §13, §15), `docs/plans/2026-10-08-security-remediation-spec.md` (§9.5
  appended), `crates/yokohama-cli/src/screen_model.rs` (:33–34, :2463–2464) — Task 7.
- Modify `Cargo.toml`, `Cargo.lock`, `web/package.json`, `web/package-lock.json`, `worker/package.json`,
  `worker/package-lock.json`, `web/src/wire/health.json` and the envelope fixture `backup` regenerates,
  `deploy/Dockerfile.runtime:18` — Task 8.
- The ledger, guidingpath, the board, `CLAUDE.md` and this plan's §"Review record" — Task 9.

## Task order and why

Task 1 first: it owns `echo`, which Task 2's 404 and Task 3's refusal sentence use. Task 2 (the lock map) and
Task 3 (the header and the dotted id) are independent of each other. Task 4 (the stream, the two S1b residuals)
touches `lib.rs`'s server half, which Task 3's layer also touches — Task 4 waits for Task 3 so `app()` is edited
once. Task 5 (the segment) touches `archive.rs`, `main.rs` and the Worker, nothing the others touch. Task 6 (M-3)
touches `explain.rs` and the `why` route alone. Task 7 writes the documents the code changed and the CLI's two
comments. Task 8 bumps the version last, so the regenerated `server_version` fixtures read the shipping head.
Task 9 is the orchestrator's frame and the wrap.

---

### Task 1: The echo rule — `echo::echo`, the message cap, the line bound, D29's sentence — Opus (D12, D28, D29; Review Focus 1)

**Files:**
- Create: `crates/yokohama-web/src/echo.rs`
- Modify: `crates/yokohama-web/src/lib.rs` (the module list :53–77; `ApiError` :122–160; `fallback` :1058),
  `crates/yokohama-web/src/api.rs` (:1335 the line check; :1753 the index kind; :1759 the filter guard; :2252;
  :2511 and :2630 `req.path`; :2645 and :2658 `{raw}`),
  `crates/yokohama-web/src/tables.rs` (every non-test `{id:?}` — :827, :855, :858, :930, :932, :1072, :1140,
  :1287, :1322, :1357, :1381, :1482, :1500, :1586, :1647, :1673, :1733, :1922, :2017 — the positional `doc.id`
  at :907, `{key:?}` at :973, :975, :1002, the seed's `{t:?}` at :111 and the scope's `{other:?}` at :750),
  `crates/yokohama-web/src/games.rs` (`{id:?}` at :687, :713, :845, :911, :948, :962, :1012, :1151),
  `crates/yokohama-web/src/store.rs` (`{id:?}` at :278, :509, :528, :563 — "is not a usable game name", the
  sentence a 16 KB game id's 500 carries today),
  `crates/yokohama-web/src/identity.rs` (:169, :172, :184, :189), `crates/yokohama-web/src/grammar.rs` (:74
  `is_gameplay_line`; :102 `is_token`'s doc; :148 the refusal)
- Test: `echo.rs`'s, `grammar.rs`'s, `lib.rs`'s and `api.rs`'s `mod tests`

**Interfaces:**
- Consumes: nothing of another task.
- Produces: `echo::ECHO_MAX`, `echo::echo`; `MESSAGE_MAX`, `ApiError::capped`; `grammar::LINE_MAX`,
  `grammar::check_line`.

- [ ] **Step 1: The failing cells.**

(a) `crates/yokohama-web/src/echo.rs`, whole:

```rust
//! **The echo rule** (security S1c, findings D12 and D28). An error sentence
//! that repeats what the caller sent is the right sentence — a refusal that
//! only says "no" gets worked around — and it is also an amplifier and a
//! channel: the review measured a 940 KB line answered with a 3.3 MB body,
//! and read the raw path, the id, the filter term and the identity header
//! echoed whole. One helper bounds every echo; the sites keep their words.

/// How much of a caller's value a sentence repeats. Sixty-four characters is
/// the longest id or name this API admits by rule — a game id; a name's
/// thirty-two — so a legitimate one is never cut, and a hostile value is cut
/// to the size of a legitimate one. (An email may run to 254 bytes; the
/// sites that echo an address echo one they refused.)
pub const ECHO_MAX: usize = 64;

/// `value` as a sentence repeats it: the first [`ECHO_MAX`] characters,
/// escaped as `{:?}` escapes a `str` (double quotes, backslashes, controls,
/// combining marks and every non-printable code point as `\u{…}`), and `…`
/// after a cut. NO quotes of its own — the site writes them, so a site that
/// quoted with `{x:?}` writes `"{}"` and reads byte-identically for every
/// value of at most [`ECHO_MAX`] characters.
///
/// **`{:?}` itself, less its two quotes, and not `str::escape_debug`**: the
/// latter escapes a single quote (`it\'s`) and leaves a combining mark after
/// the first character raw, so an apostrophe in a name would have read
/// differently from the `{x:?}` sentence it replaced.
pub fn echo(value: &str) -> String {
    let shown: String = value.chars().take(ECHO_MAX).collect();
    let cut = value.chars().nth(ECHO_MAX).is_some();
    let mut out = format!("{shown:?}");
    out.pop();
    out.remove(0);
    if cut {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A short, clean value reads exactly as `{:?}` read it, less the quotes —
    /// the premise every pinned sentence in the crate rests on. The last two
    /// are where `str::escape_debug` reads differently: an apostrophe and a
    /// combining mark after the first character.
    #[test]
    fn a_short_clean_value_is_itself() {
        for v in ["wire", "/api/nope", "x.js", "red@wire.test", "A.b-c_d", "a b", "it's", "e\u{301}"] {
            assert_eq!(format!("\"{}\"", echo(v)), format!("{v:?}"));
        }
    }

    /// The cut is at sixty-four CHARACTERS, marked, and never inside a code
    /// point; the escape turns a control, a format character and a quote into
    /// their escapes, so no echo carries a raw newline or a soft hyphen.
    #[test]
    fn a_long_or_hostile_value_is_cut_and_escaped() {
        let long = "x".repeat(ECHO_MAX + 1);
        let e = echo(&long);
        assert_eq!(e.chars().count(), ECHO_MAX + 1, "{e}");
        assert!(e.ends_with('…'));
        assert_eq!(echo(&"x".repeat(ECHO_MAX)), "x".repeat(ECHO_MAX), "exactly the bound is not cut");
        let soft = "\u{ad}".repeat(1_000);
        let e = echo(&soft);
        assert!(!e.contains('\u{ad}'), "a format character is escaped: {e}");
        assert!(e.starts_with("\\u{ad}"), "{e}");
        assert!(e.len() < 64 * 8, "the escape of sixty-four code points is bounded: {}", e.len());
        assert_eq!(echo("1\nsave x"), "1\\nsave x");
        assert_eq!(echo("say \"hi\""), "say \\\"hi\\\"");
        let wide = "日本".repeat(40);
        assert_eq!(echo(&wide).chars().count(), ECHO_MAX + 1, "cut by character, not by byte");
    }
}
```

(b) In `grammar.rs`'s `mod tests`, beside `every_gameplay_line_the_client_can_produce_is_allowed`:

```rust
    /// **The line bound** (finding D28): the longest line the client's own
    /// grammar composes passes with room, and a line over the bound or
    /// carrying any control character is refused by a sentence that names the
    /// LENGTH and never the text — a 940 KB line of soft hyphens once came
    /// back as a 3.3 MB body. `1\nsave x` is refused here too (finding D29's
    /// note: `split_whitespace` read it as three tokens), by the control rule.
    #[test]
    fn the_line_bound_admits_the_longest_legal_line_and_refuses_a_long_or_control_one() {
        // The longest line the GRAMMAR admits with meaning — the CLI's retreat
        // at a 4P table, an `A`-prefixed area id and a count for every one of
        // the eighteen areas behind `r `, 99 bytes; the web client itself
        // composes nothing longer than an option number or an area token.
        let longest = format!("r {}", (0..18).map(|a| format!("A{a}:5")).collect::<Vec<_>>().join(" "));
        assert!(longest.len() <= LINE_MAX / 2, "the longest legal line is {} bytes, the bound {LINE_MAX}", longest.len());
        assert!(check_line(&longest).is_ok() && is_gameplay_line(&longest));
        assert!(check_line("").is_ok(), "the empty line applies the sole option");

        let long = "1".repeat(LINE_MAX + 1);
        let err = check_line(&long).expect_err("over the bound");
        assert!(err.contains(&format!("{} bytes", LINE_MAX + 1)), "{err}");
        assert!(!err.contains("111111"), "the sentence names the length, never the text: {err}");
        let huge = "\u{ad}".repeat(940 * 1024 / 2);
        let err = check_line(&huge).expect_err("over the bound");
        assert!(err.len() < 200, "the refusal of a {} byte line is {} bytes", huge.len(), err.len());

        for line in ["1\nsave x", "1\rsave", "a\tb", "1\u{0}"] {
            let err = check_line(line).expect_err("a control character");
            assert!(err.contains("control character"), "{line:?}: {err}");
            assert!(!is_gameplay_line(line), "{line:?} is refused by the grammar as well");
        }
    }

    /// `refusal` repeats the line through the echo rule: sixty-four characters,
    /// escaped — the mutant that keeps `{line:?}` fails the `…` line first,
    /// then the length line.
    #[test]
    fn a_refusal_echoes_at_most_sixty_four_characters() {
        let r = refusal(&"save ".repeat(100));
        assert!(r.starts_with("\"save save"), "{r}");
        assert!(r.contains('…'), "{r}");
        assert!(r.len() < 400, "{}", r.len());
        assert_eq!(refusal("save x"), format!("{:?} writes a file on the server; games persist automatically after every move. This endpoint takes an option number, a sub-prompt answer (confirm/cancel/an area/a count), a move grammar (a/b/m/r), more/list, undo/back, or an empty line to apply a sole option.", "save x"));
    }
```

(c) In `lib.rs`'s `mod tests`, after `const WHO` (:1153) — the module has no `ApiError` cells to sit beside:

```rust
    /// **No error body carries more than `MESSAGE_MAX` characters of message**
    /// (finding D12, decision D-2), by two layers. The FIRST is pinned here:
    /// the cap at construction, in the constructors and in the
    /// `From<GameError>` arms that build the struct literally, so the in-memory
    /// sentence is bounded too. Every constructor that takes a message is
    /// listed; a new one joins the list or fails the count line. The SECOND is
    /// the boundary, `api::error_value`, which caps whatever built the error —
    /// a handler's own struct literal (`busy`, `unauthorized`, `api.rs`'s own)
    /// included — and is pinned by `a_literal_api_error_is_capped_at_the_boundary`.
    #[test]
    fn every_api_error_message_is_capped_at_construction() {
        let long = "m".repeat(MESSAGE_MAX * 3);
        let built: Vec<ApiError> = vec![
            ApiError::bad_request(long.clone()),
            ApiError::not_found(long.clone()),
            ApiError::server_error(long.clone()),
        ];
        for e in &built {
            assert_eq!(e.message.chars().count(), MESSAGE_MAX + 1, "{}", e.status);
            assert!(e.message.ends_with('…'));
        }
        // Every constructor that TAKES a message passes it through `capped`: the
        // count of `message: impl Into<String>` signatures in the impl block is
        // the count of `Self::capped(message.into())` calls, and both are the
        // three built above. A fourth constructor that skips the cap fails the
        // second count before it fails a body; `dealing`, `open` and `playing`
        // carry constants, and `busy()` builds the struct literally from one.
        let source = include_str!("lib.rs");
        let impl_block = source.split("impl ApiError {").nth(1).and_then(|r| r.split("\n}\n").next()).expect("ApiError's impl");
        assert_eq!(impl_block.matches("message: impl Into<String>").count(), built.len(), "every message-taking constructor is built above: {impl_block}");
        assert_eq!(impl_block.matches("Self::capped(message.into())").count(), built.len(), "every message-taking constructor caps: {impl_block}");
        let short = ApiError::bad_request("short");
        assert_eq!(short.message, "short");
        // The `From<GameError>` arms that build the struct literally cap too,
        // so every `GameError`'s sentence reaches a body bounded.
        let literal = [
            ApiError::from(games::GameError::Forbidden(long.clone())),
            ApiError::from(games::GameError::Conflict(long.clone())),
            ApiError::from(games::GameError::ForeignBot { played: "x".into(), runs: "y".into(), why: long.clone() }),
        ];
        for e in &literal {
            assert_eq!(e.message.chars().count(), MESSAGE_MAX + 1, "{}", e.status);
        }
    }

    /// **A hostile value anywhere a sentence echoes it is bounded** (finding
    /// D12). Each row drives ONE echo site through the router with a 16 KB
    /// value — `z`s, or spaces where the grammar trims the ends — names a
    /// fragment of the sentence it must reach, and reads the body for its size
    /// and for the raw run's absence. The cap keeps every body near a kilobyte
    /// whatever the echo does, so the absence line is the one that tells the
    /// echo from the cap. The mutant that restores the `{x:?}` (or unwraps the
    /// plain `{x}`) at one of THESE sites fails its row: `dispatch`'s
    /// unmatched path, `MemStore::read`'s id, the index kind, the filter guard
    /// (dropped whole, `parse_filter` repeats the term raw),
    /// `parse_path_usize`, `parse_path_u64`, the input's `expect`, `create`'s
    /// id and its seed (`WireSeed`, through `parse_json`), the four sentences
    /// of `User::parse` (the header for "neither" and "whitespace"; a seat's
    /// user for "reserved" and "not an email", since the header is trimmed
    /// before the grammar reads it and only a trimmed end makes those two
    /// long), and `check_shape`'s watcher (an archive upload). The input
    /// route's `check_line` call and the filter guard's `echo` half are pinned
    /// by the lines after the loop. The inventory's other sites carry no row:
    /// each is wrapped by the same rule, and bounded by the cap besides.
    #[tokio::test]
    async fn every_echo_site_answers_a_bounded_body() {
        let state = Arc::new(AppState::in_memory());
        a_game(&app(Arc::clone(&state)), "echoed", &[]).await;
        let big = "z".repeat(16 * 1024);
        let pad = " ".repeat(16 * 1024);
        let create = |seats: serde_json::Value, more: serde_json::Value| {
            let mut body = serde_json::json!({ "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seats": seats, "name": "x" });
            for (k, v) in more.as_object().expect("an object") {
                body[k] = v.clone();
            }
            Some(body)
        };
        let two = serde_json::json!([{ "user": WHO }, { "user": WHO }]);
        // (method, uri, body, the identity header, a fragment of the sentence the row reaches)
        let rows: Vec<(&str, String, Option<serde_json::Value>, String, &str)> = vec![
            ("GET", format!("/api/{big}"), None, WHO.into(), "no such endpoint"),
            // The store's 500 until Task 2's `dispatch` check makes it a 404;
            // "game name" reads in both sentences.
            ("GET", format!("/api/games/{big}"), None, WHO.into(), "game name"),
            ("GET", format!("/api/games/echoed/index/{big}"), None, WHO.into(), "no card index named"),
            ("GET", format!("/api/games/echoed/index/orders?filter={big}"), None, WHO.into(), "nothing a quoted string would escape"),
            // A path number: `at/{k}` reads it with `parse_path_usize`, a chat
            // message's DELETE with `parse_path_u64` (a GET there is a bare 405).
            ("GET", format!("/api/games/echoed/at/{big}"), None, WHO.into(), "Cannot parse value at index 1"),
            ("DELETE", format!("/api/games/echoed/chat/{big}"), None, WHO.into(), "Cannot parse value at index 1"),
            ("POST", "/api/games/echoed/input".to_string(), Some(serde_json::json!({ "line": format!("save {big}") })), WHO.into(), "this one is"),
            // `expect` is the caller's own string, and the 409 that names a
            // prompt that moved on repeats it — through a literal `ApiError`
            // the constructors' cap never sees.
            ("POST", "/api/games/echoed/input".to_string(), Some(serde_json::json!({ "line": "1", "expect": big })), WHO.into(), "the prompt moved on"),
            ("POST", "/api/games".to_string(), create(two.clone(), serde_json::json!({ "id": format!("a/{big}") })), WHO.into(), "is not a usable game id"),
            // The seed's own sentence, inside serde's, inside `parse_json`'s.
            ("POST", "/api/games".to_string(), create(two.clone(), serde_json::json!({ "seed": big })), WHO.into(), "is not a u64 seed"),
            // A seat's user is parsed untrimmed: a trimmed end is what lets
            // the two sentences below repeat a long value at all.
            ("POST", "/api/games".to_string(), create(serde_json::json!([{ "user": format!("anonymous{pad}") }, { "user": WHO }]), serde_json::json!({})), WHO.into(), "is reserved"),
            ("POST", "/api/games".to_string(), create(serde_json::json!([{ "user": format!("a@b@c{pad}") }, { "user": WHO }]), serde_json::json!({})), WHO.into(), "is not an email"),
            // The identity header, under the open door: a value that is
            // neither an email nor a name, and one whose inner spaces
            // `typed_form` collapses before the whitespace rule refuses it.
            ("GET", "/api/games".to_string(), None, big.clone(), "is neither an email nor a name"),
            ("GET", "/api/games".to_string(), None, format!("a{pad}b@c"), "contains whitespace"),
        ];
        for (method, uri, body, who, reaches) in rows {
            let req = Request::builder().method(method).uri(&uri).header(identity::CLAIMED_HEADER, &who);
            let req = match body {
                Some(b) => req.header("content-type", "application/json").body(Body::from(serde_json::to_vec(&b).expect("a body"))),
                None => req.body(Body::empty()),
            }
            .expect("a request");
            let res = app(Arc::clone(&state)).oneshot(req).await.expect("the router answers");
            let status = res.status();
            let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
            let text = String::from_utf8_lossy(&bytes);
            let row = format!("{method} {} ({reaches})", uri.chars().take(60).collect::<String>());
            // Every row is a refusal; WHICH refusal is each route's own cell's
            // claim — the 16 KB game id is the store's 500 until Task 2's
            // `dispatch` check makes it the 404 that cell pins (finding B8).
            assert!(status.is_client_error() || status.is_server_error(), "{row}: {status}");
            assert!(bytes.len() <= 2_048, "{row}: a {} byte body", bytes.len());
            assert!(!text.contains(&"z".repeat(65)) && !text.contains(&" ".repeat(65)), "{row}: the raw value came back");
            assert!(text.contains(reaches), "{row}: the row reaches its sentence: {}", text.chars().take(200).collect::<String>());
        }
        // The input route asks `check_line` BEFORE the grammar: the long line
        // is refused by its LENGTH, not echoed back cut — `refusal`'s echo
        // alone would bound the body too, so this is the line that pins the
        // route's call.
        let (status, v) = post_as(&state, WHO, "/api/games/echoed/input", serde_json::json!({ "line": format!("save {big}") })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{v}");
        let said = v["error"].as_str().unwrap_or_default();
        assert!(said.contains(&format!("this one is {} bytes", big.len() + 5)) && !said.contains("save"), "{said}");
        // The filter guard refuses what `echo` would change — a double quote,
        // a backslash, a U+202E override, a combining mark — since the CLI's
        // `parse_filter` repeats the term raw; an apostrophe and a precomposed
        // `é` are plain text and pass, to `parse_filter`'s own sentence, so the
        // sentence's last word is pinned by `cafe` + U+0301 against `caf` +
        // U+00E9. The mutant that keeps only the length half of the guard fails
        // the refused rows.
        let guard = "a filter is at most 64 characters of plain text — nothing a quoted string would escape: no quote, backslash, control, format or combining character";
        let terms = [
            ("%22silk%22", true),
            ("si%5Clk", true),
            ("si%E2%80%AElk", true),
            ("cafe%CC%81", true),
            ("caf%C3%A9", false),
            ("o'brien", false),
        ];
        for (term, refused) in terms {
            let (status, v) = get_as(&state, WHO, &format!("/api/games/echoed/index/orders?filter={term}")).await;
            let said = v["error"].as_str().unwrap_or_default();
            assert_eq!(status, StatusCode::BAD_REQUEST, "{term}: {v}");
            assert_eq!(said == guard, refused, "{term}: {said}");
        }
        // A watcher entry in an UPLOADED table document is the caller's value
        // too: `check_shape` repeats the one that does not parse, and the
        // archive route reaches it. The message cap alone would keep this body
        // near a kilobyte, so the line that tells the echo from the cap is the
        // raw value's absence.
        let mut table: serde_json::Value = serde_json::from_str(PLAYED_TABLE).expect("the played table");
        table["unlocked"] = serde_json::json!([big]);
        let (status, v) = post_as(&slow_state(0, 1, 1_000), WHO, "/api/archive", serde_json::json!({ "record": PLAYED_RECORD, "table": table.to_string() })).await;
        let text = v.to_string();
        assert_eq!(status, StatusCode::BAD_REQUEST, "{}", text.chars().take(200).collect::<String>());
        assert!(text.len() <= 2_048 && !text.contains(&"z".repeat(65)), "POST /api/archive (on the watcher list): the raw value came back in a {} byte body", text.len());
        assert!(text.contains("on the watcher list"), "the row reaches the sentence: {}", text.chars().take(200).collect::<String>());
    }

    /// **The second layer: the cap at the BOUNDARY** (finding D12, decision
    /// D-2, S1c Task 1 fix round 1). A handler that builds `ApiError` as a
    /// struct literal passes no constructor, so the first layer never sees its
    /// message; `api::error_value` — the one serializer behind
    /// `IntoResponse` and `api::error_body`/`error_response`, which the server,
    /// the browser module and the Worker module write a refusal through — cuts
    /// it whatever built it. Read through both arms: the mutant that serializes
    /// `.message` raw reads 3,072 characters on both, while every
    /// constructor-built error stays capped by the first layer.
    #[tokio::test]
    async fn a_literal_api_error_is_capped_at_the_boundary() {
        let literal = || ApiError {
            status: StatusCode::CONFLICT,
            message: "m".repeat(MESSAGE_MAX * 3),
            table_status: None,
            foreign_bot: None,
        };
        let res = literal().into_response();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        let via_axum: serde_json::Value = serde_json::from_slice(&bytes).expect("a JSON body");
        let layer = api::error_response(literal());
        assert_eq!(layer.status, 409);
        let via_layer: serde_json::Value = serde_json::from_str(&layer.body).expect("a JSON body");
        for (arm, body) in [("IntoResponse", via_axum), ("error_response", via_layer)] {
            let error = body["error"].as_str().expect("an error sentence");
            assert_eq!(error.chars().count(), MESSAGE_MAX + 1, "{arm}");
            assert!(error.ends_with('…'), "{arm}");
        }
        // A message under the cap is written as it always was, byte for byte.
        let short = ApiError { status: StatusCode::CONFLICT, message: "short".into(), table_status: Some("open"), foreign_bot: None };
        assert_eq!(api::error_body(&short), r#"{"error":"short","status":"open"}"#);
    }
```

(The `index/{kind}` row today answers the kind's sentence with `{other:?}`; the `at/{k}` row reaches
`parse_path_usize` and the chat DELETE row `parse_path_u64`, axum's own path sentence with `{raw}` — a GET at
`chat/{seq}` is a bare 405 with an empty body, which the draft's row was (the implementer's C4); the `expect` row
reaches the 409 that names a prompt that moved on (C2); the archive block reaches `tables::check_shape`'s
watcher sentence through an upload (C3); the create row answers `tables::create`'s id sentence; the identity row
answers `User::parse`'s; the route line pins `input`'s `check_line` call (C6). The first cell counts
`message: impl Into<String>` signatures against `Self::capped(message.into())` calls — three and three — and
builds the three `From<GameError>` arms that construct the struct literally (C5): a message-taking constructor
that skips the cap fails the count before it fails a body; a handler that builds `ApiError` literally is NOT
capped and repeats a caller's text through `echo` or not at all.)

(d) In `api.rs`'s `mod tests`, the existing pins `assert_eq!((res.status, res.body), (404,
error_body(&ApiError::not_found("no such endpoint: /api/nope"))))` (:2791) and `lib.rs:2114`'s
`{"error":"no such endpoint: /api/nothing"}` stay as they are — decision D-1 — and are the cells that catch a
quoting change.

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --lib -- echo:: line_bound
sixty_four capped every_echo_site 2>&1 | grep -E '^error|test result|panicked' | head` — `echo`, `LINE_MAX`,
`check_line`, `MESSAGE_MAX` not found; stubbed (the function `value.to_string()`, the constants, `check_line`
`Ok(())`, `capped` the identity): (a)'s second cell RED on the cut; (b) RED on the long line (no refusal) and on
the control line; (c) RED on the cap and on the 16 KB rows (today's bodies carry the value whole — measured on the stubbed tree, 2026-10-10: `GET /api/<16 KB>` 16,419 bytes; `GET /api/games/<16 KB>` 16,426 (the
store's 500); the index kind 16,468; the filter 16,486; the chat DELETE 16,463; the input `save …` 16,661; the input
`expect` 16,537 (a 409 built literally, which the cap never sees); the create 16,426; the identity header 16,540 —
every row carried the raw value).

- [ ] **Step 3: The code.**

In `lib.rs`, with the modules (:60–75): `pub mod echo;` and `pub use echo::echo;`. Beside `ApiError` (:122):

```rust
/// The most characters any error body's `message` carries (security S1c,
/// finding D12, decision D-2). Sixteen echoes of `echo::ECHO_MAX` — no sentence
/// in this crate repeats more than two values — and a fence under every
/// sentence a site composes from a library's own words: serde names an unknown
/// field verbatim, axum names a path segment. Applied twice: at
/// construction — every constructor and the `From<GameError>` arms, so the
/// in-memory sentence is bounded — and at the boundary, in `api::error_value`,
/// the one serializer behind `IntoResponse` and [`api::error_body`] that the
/// server, the browser module and the Worker module write a refusal through.
/// So a handler that builds `ApiError` literally is bounded all the same, and
/// so is a site the echo inventory missed. (The Worker's `identity_inner`
/// writes its 401 by hand from `User::parse`'s sentence, outside both; that
/// sentence's one caller value is bounded by [`echo()`].)
pub const MESSAGE_MAX: usize = 1_024;

impl ApiError {
    /// `message` cut to [`MESSAGE_MAX`] characters with a `…`; every
    /// constructor below that takes a message passes through here (`dealing`,
    /// `open` and `playing` carry constants), and `api::error_value` caps at
    /// the boundary besides.
    pub fn capped(message: String) -> String {
        if message.chars().nth(MESSAGE_MAX).is_none() {
            return message;
        }
        let mut out: String = message.chars().take(MESSAGE_MAX).collect();
        out.push('…');
        out
    }
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: Self::capped(message.into()), table_status: None, foreign_bot: None }
    }
    // … `not_found`, `forbidden`, `conflict`, `server_error` the same way: each
    // existing constructor's body with `Self::capped(message.into())` in place
    // of `message.into()`. The struct's fields are what they are at :122–141;
    // the implementer reads them rather than this sketch.
}
```

`busy()` (:1017) builds the struct literally with `BUSY.to_string()` — a constant; since fix round 1 every
literal is capped at the boundary anyway. In `api.rs`, the one serializer (fix round 1):

```rust
/// **The message is cut to [`crate::MESSAGE_MAX`] HERE, at the boundary**
/// (security S1c, decision D-2's second layer): whatever built the error — a
/// constructor, which caps already, or a handler's own struct literal, which
/// does not — its sentence becomes bytes through this function and no other
/// in the layer, so no refusal leaves the layer longer than the cap.
pub(crate) fn error_value(e: &ApiError) -> serde_json::Value {
    let mut body = serde_json::json!({ "error": ApiError::capped(e.message.clone()) });
    // … the `status` and `foreign_bot` keys as before …
```

`error_body` is `error_value(e).to_string()`, `error_response` calls `error_body`, and `lib.rs`'s `IntoResponse` is
`Json(api::error_value(&self))`; the Worker module's `host_refusal`, `events.rs`, `picker.rs` and `receipt.rs` reach
the wire through `error_body`, re-exported, so no host is edited.

In `grammar.rs`, above `is_gameplay_line` (:84):

```rust
/// The most bytes a gameplay line may carry (security S1c, finding D28). The
/// web client composes nothing longer than an option number or an area token
/// (`A05`), under six bytes; the grammar also admits the CLI's four move
/// grammars for the terminal's sake, whose longest meaningful line is a 4P
/// retreat naming every area with its count — eighteen `A17:5` pairs behind
/// `r `, 99 bytes (109 zero-padded; the pre-flight's measurement) — so 256 is
/// more than twice the longest legal line and an eight-thousandth of the 2 MiB body
/// limit a hostile line could fill.
pub const LINE_MAX: usize = 256;

/// The shape of a line BEFORE its grammar is read: at most [`LINE_MAX`] bytes,
/// no control character (a newline inside a "line" once read as three tokens;
/// finding D29's note). The refusal names the LENGTH and never the text, since
/// a refusal that echoed the line was the review's 3.5× amplifier.
pub fn check_line(line: &str) -> Result<(), String> {
    if line.len() > LINE_MAX {
        return Err(format!("a line is at most {LINE_MAX} bytes — this one is {} bytes", line.len()));
    }
    if let Some(c) = line.chars().find(|c| c.is_control()) {
        return Err(format!("a line carries no control character — this one has {}", c.escape_unicode()));
    }
    Ok(())
}
```

In `is_gameplay_line` (:88–94), first: `if line.chars().any(char::is_control) { return false; }` — the grammar
refuses what `check_line` refuses, so the two fences agree and `refusal` is reached for a control line through
the route as well. In `refusal` (:148): `"{line:?} {why}. …"` becomes `"\"{}\" {why}. …"` with
`crate::echo::echo(line)` as the first argument.

`is_token`'s doc (:96–101) — finding D29 — rewritten:

```rust
/// One argument or one option number: ASCII alphanumerics and `:`, nothing else.
///
/// Covers every shape the grammars use — a bare option number (`7`), an area
/// with or without its prefix (`15`, `A15`, `a15`), a retreat pair (`3:2`), the
/// `t` separator in `b 5 t 6`, and `hand` as an assistant-move endpoint.
///
/// **What this fences is the SHAPE of an argument, not what a word means**:
/// it admits no `/`, `\`, `.` or space, so no token names a path's components
/// — but any alphanumeric token IS a relative path, and `save`, `undo` and
/// `a` are tokens too. What keeps a REPL word from reaching `step` as a
/// command is [`HEADS`] and [`is_area_or_number`]: a head that is not listed
/// is refused there, and the REPL matches whole token slices, so an admitted
/// `undo save` is `["undo", "save"]` — no command of that shape exists. A
/// line with a newline inside it is refused before this by [`check_line`].
fn is_token(token: &str) -> bool {
```

In `api.rs`, `input` (:1335): before `if !grammar::is_gameplay_line(&req.line)`:

```rust
    grammar::check_line(&req.line).map_err(ApiError::bad_request)?;
```

The echo sites — the RULE is mechanical (the pre-flight's `echo_sites.py` did every site and the lib read 463
green): a `{x:?}` inside a `format!` becomes `\"{x}\"` and the call gains `x = crate::echo::echo(x)` as a NAMED
argument (named, not positional, so a string that already carries positional `{}` — `games.rs:948`, `:962` —
keeps its order; a `String` site borrows, `echo(&x)`); a plain `{x}` keeps its spelling and gains the same named
argument; the positional `{:?}` of `doc.id` (`tables.rs:907`) becomes `\"{}\"` with `crate::echo::echo(&doc.id)`;
`req.path` and `uri.path()` are wrapped in place. A short, clean value reads byte-identically to `{x:?}` (D-1). The
inventory: `api.rs:1753` (`other`), `:2252` (`id`), `:2511` and `:2630` (`req.path`), `:2645` and `:2658` (`{raw}`,
plain); every non-test `{id:?}` in `tables.rs` (nineteen — the four at `create`, :827, :855, :858 and :907's
`doc.id`, are the BODY's id, bounded by nothing else), `tables.rs`'s `{key:?}` (:973, :975, :1002), `{t:?}` (:111,
the body's seed string) and `{other:?}` (:750, the query's scope); `games.rs`'s eight `{id:?}` (:687, :713, :845,
:911, :948, :962, :1012, :1151); `store.rs`'s four `{id:?}` (:278, :509, :528, :563 — "is not a usable game name",
the sentence a 16 KB game id's 500 carries today, which the draft inventory missed and the cell went red on);
`identity.rs:169`, `:172`, `:184`, `:189` (`raw`); `lib.rs:1058` (`uri.path()`); **and three the draft read wrong
(the implementer's C2, C3, C8)**: `api.rs:1437`'s `{expected:?}` is the caller's `Input.expect`, repeated whole
through a 409 built literally (16,537 bytes measured) — wrapped, `{actual:?}` beside it a `&'static str` left
alone; `tables.rs:413`'s `{bad:?}` is the watcher entry that FAILED `User::parse` in an UPLOADED table document
(`archive_post` → `check_pair_filed` → `check_shape`) — wrapped; `store.rs` holds EIGHT non-test `{id:?}` (the four
above plus `:245`, `:440`, `:461`, `:557`), all wrapped. **The `From<GameError>` arms** that build the struct
literally — `Forbidden`, `Conflict`, `ForeignBot` — pass their sentence through `Self::capped` too (C5): the
`Conflict` sentences carry ids. Left alone, by inspection: `api.rs:470` (`key_of`'s serde value), `:1438`'s
`{actual:?}` (a prompt kind), `tables.rs:239`, `:387` (a sort key, a status), `store.rs:245`'s and `:461`'s
`{:?}` beside the echoed id (the house constant, the `Doc` enum), the two environment-variable sentences
(`identity.rs:112`, `lib.rs:330`). The after review greps the seven files and accounts for exactly those hits. The filter term (`api.rs:1759`): before `parse_filter`,

```rust
        // **What `echo` would change is refused, not echoed**: the CLI's
        // `parse_filter` (a fenced crate) repeats the term raw in single
        // quotes, so a term `{:?}` would escape — a double quote, a backslash,
        // a control, a format character such as U+202E, a combining mark —
        // never reaches it. An apostrophe is plain text to `{:?}`, and passes.
        // (The length half is LOAD-BEARING, not subsumed: a term whose 65th
        // character is `…` echoes as itself — `x`×64+`…` — so without the
        // count a 129-character `\`×128+`…` would reach `parse_filter` raw.
        // The cell's terms carry one such term; mutant ML reads it.)
        Some(term) if term.chars().count() > crate::echo::ECHO_MAX || crate::echo::echo(term) != term => {
            return Err(ApiError::bad_request(format!(
                "a filter is at most {} characters of plain text — nothing a quoted string would escape: no quote, backslash, control, format or combining character",
                crate::echo::ECHO_MAX
            )))
        }
```

so the CLI's own `parse_filter` sentence (the fenced crate's, which embeds the term RAW in single quotes) is
bounded by what reaches it — and since fix round 2 the guard refuses any term `echo` would change, so nothing a
quoted string would escape (a quote, a backslash, a control, format or combining character) reaches the CLI's
sentence; the length half is load-bearing — a term whose 65th character is `…` echoes as itself (the re-reviewer's ML) —
and the cell pins it with such a term (the final fix wave). The Worker's `identity_of` and the wasm `init` call `User::parse` too and inherit the bounded sentences.

- [ ] **Step 4: Run the cells and the crate.** `cargo test -p yokohama-web` — every cell green, `wire_fixtures`
green WITHOUT the flag (no sentence a fixture carries changed: every value there is short and clean). Then the
lane's rows (the Worker's `npm test` pins the 404's text at three places — unchanged by D-1).

- [ ] **Step 5: The mutants** (the reviewer re-runs each on an export): **M1** `echo` returns `value.to_string()`
— `a_long_or_hostile_value_is_cut_and_escaped` red, `every_echo_site_answers_a_bounded_body` red on the first
row; **M2** `capped` the identity — `every_api_error_message_is_capped_at_construction` red; **M3** `check_line`
`Ok(())` — the line-bound cell red on the long line AND `every_echo_site…`'s route line red (the draft said "the
input row's body carries the value" — false, since `refusal`'s own echo bounds that body; the route line pins the
`check_line` CALL by its length sentence, the implementer's C6); **M3b** the route's `check_line` call deleted —
the same line red; **M4** the control rule dropped from `is_gameplay_line` alone — the line-bound cell's `is_gameplay_line`
line red; **M5** `refusal` back to `{line:?}` — `a_refusal_echoes_at_most_sixty_four_characters` red; **M6** the
filter guard dropped — `every_echo_site…`'s filter row red; **M7** one identity site back to `{raw:?}` — the
header row red. The implementer's own (2026-10-10, each red at `every_echo_site…` or the cap cell): **M8** `expect`
back to `{expected:?}`; **M9** `parse_path_usize` unwrapped (the `at/{k}` row); **M10** `tables.rs:413` back to
`{bad:?}` (the archive block); **M11** `echo` through `str::escape_debug` (the short-clean cell's `it's` and
`e\u{301}`); **M12** a `From<GameError>` arm uncapped (the cap cell); **M13** `parse_path_u64` unwrapped (the DELETE
row); **M15** `MemStore::read`'s sentence back; **M16** `create`'s; **M17** the index kind's; **M14a** `dispatch`'s
`NoRoute` path unwrapped (the first row). **Two equivalent mutants, recorded**: **M14b** `fallback`'s `uri.path()`
unwrapped and **M14c** `dispatch`'s inner `_` arm unwrapped — `/api/{*rest}` sends every non-empty tail to the
adapter and the inner arm fires only if the table and the match disagree, so no router-level cell reaches either;
both stay wrapped by the rule. Fix round 1's: **MB** the boundary cap dropped (`error_value` writes
`e.message` raw) — `a_literal_api_error_is_capped_at_the_boundary` red at its `IntoResponse` arm alone; **MC**
`bad_request`'s construction cap dropped — the construction cell alone; M2 (`capped` the identity) now reds both. Fix rounds 2 and 3's: **X1** `identity.rs:184` back to `{raw:?}` (the
reviewer's — GREEN at d6e12f54: reachable through a header of `a` + 16 KB of spaces + `b@c`, which `typed_form`
collapses), **X2**/**X3** `:172` and `:189` (reachable through `create`'s untrimmed seat `user`, never the header,
which `resolve` trims), **X6** the seed's `{t:?}` (through `parse_json`'s `{e}`), **MF** the guard's `echo` half
dropped (the `%22silk%22` line) — each red on "<row>: the raw value came back"; **M7** reds AGAIN (fix round 1's
boundary cap had made the old size-only header block pass it — the implementer's own catch); **M11** now also
reds the echo cell at the `cafe%CC%81` line. **ML**, the guard's LENGTH half dropped, is NOT equivalent (the re-reviewer, 2026-10-10 — the implementer's
"subsumed" claim was false): a term whose 65th character is `…` echoes as itself, so under ML `x`×64+`…` and
`\`×128+`…` pass the guard and `parse_filter` repeats 128 raw backslashes; at `ae89c36c` the lib reads 464 green
under ML. Deferred to the final fix wave as a Minor: the comment above corrected (transcribed here already) and a
refused term `x`×64+`%E2%80%A6` added to the cell's `terms`, so ML goes red.

- [ ] **Step 6: Commit.** `git add crates/yokohama-web/src/echo.rs crates/yokohama-web/src/lib.rs
crates/yokohama-web/src/api.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/games.rs
crates/yokohama-web/src/store.rs crates/yokohama-web/src/identity.rs crates/yokohama-web/src/grammar.rs` ·
`git commit -m "web: the echo rule — every echoed caller value through
echo::echo (64 characters, escaped), every ApiError message capped at 1,024, a gameplay line at most 256 bytes
with no control character and its refusal naming the length, is_token's doc made true (S1c Task 1; D12, D28,
D29)"`.

---

### Task 2: B8 — the lock entry lives only while a writer holds it; an unsafe `{id}` is a 404 before any handler — Opus (B8; decision D-3)

**Files:**
- Modify: `crates/yokohama-web/src/games.rs` (`lock_for` :604–619, `prune_lock` :621–636, `with_table_lock`
  new, the `locks` field's doc :342), `crates/yokohama-web/src/tables.rs` (`chat_append` :1061–1080,
  `delete_chat` :1133–1147, `unlock` :1352–1360, `settle` :1395–1402, `join` :1574–1588, `keep` :1659–1672,
  `delete` :1905–1917), `crates/yokohama-web/src/api.rs` (`dispatch` :2503–2510; `matches`' doc :276–281)
- Test: `games.rs`'s, `tables.rs`'s, `api.rs`'s and `lib.rs`'s `mod tests`

**Interfaces:**
- Consumes: `echo::echo` (Task 1); `games::enter` (:129); `store::is_safe_id`.
- Produces: `Games::with_table_lock`; the 404 for an unsafe `{id}` in `dispatch`.

- [ ] **Step 1: The failing cells.**

(a) In `lib.rs`'s `mod tests`, beside `a_deleted_game_is_gone_from_the_list_and_from_the_store` (:5717):

```rust
    /// **The per-id lock map holds only the tables a writer is INSIDE** (security
    /// S1c, finding B8: the review grew the map to 107 MB with 3,000 refused
    /// writes at 16 KB ids). Six writers — join, keep, unlock, delete, a chat
    /// post, a chat delete — fired at fabricated ids answer 404 and leave the
    /// map EMPTY; a 16 KB id and a 65-byte id are 404s in `dispatch` before any
    /// handler; and a writer that SUCCEEDS at a real table leaves no entry
    /// behind either, because the entry is pruned on every exit. A count taken
    /// after the writes cannot see the pre-read, because the helper prunes on
    /// every exit; its catcher is `tables.rs`'s
    /// `a_writer_at_an_absent_id_reads_before_it_takes_the_lock`. The mutant
    /// that prunes only on delete leaves `wire`'s entry, read at the premise.
    /// (The two chat rows are refused by `admitted` before their writers.)
    #[tokio::test]
    async fn refused_and_finished_writes_leave_the_lock_map_empty() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "wire", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        let (status, body) = post_as(&state, "red@wire.test", "/api/games", serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
            "seats": [{ "user": "red@wire.test" }, { "open": true }], "id": "openk", "name": "openk",
        })).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        assert_eq!(state.games.locks.lock().expect("the map").len(), 0, "premise: a create takes no per-id lock");

        for n in 0..50 {
            let id = format!("nosuch-{n}");
            let rows: Vec<(StatusCode, serde_json::Value)> = vec![
                post_as(&state, WHO, &format!("/api/games/{id}/join"), serde_json::json!({ "seat": 1 })).await,
                post_as(&state, WHO, &format!("/api/games/{id}/keep"), serde_json::json!({ "seat": 0, "card": 1 })).await,
                post_as(&state, WHO, &format!("/api/games/{id}/unlock"), serde_json::json!({ "password": "x" })).await,
                delete_as(&state, WHO, &format!("/api/games/{id}")).await,
                post_as(&state, WHO, &format!("/api/games/{id}/chat"), serde_json::json!({ "text": "hi" })).await,
                delete_as(&state, WHO, &format!("/api/games/{id}/chat/1")).await,
            ];
            for (status, body) in rows {
                assert_eq!(status, StatusCode::NOT_FOUND, "{id}: {body}");
            }
        }
        let big = "z".repeat(16 * 1024);
        let (status, body) = post_as(&state, WHO, &format!("/api/games/{big}/unlock"), serde_json::json!({ "password": "x" })).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert!(body["error"].as_str().unwrap_or("").starts_with("no game named \"zzzz"), "{body}");
        let (status, _) = post_as(&state, WHO, &format!("/api/games/{}/unlock", "a".repeat(65)), serde_json::json!({ "password": "x" })).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(state.games.locks.lock().expect("the map").len(), 0, "refused writes grew the map");

        // A successful write — blue joins the open seat — and a chat post at
        // the playing table: both leave the map empty on the way out.
        let (status, body) = post_as(&state, "blue@wire.test", "/api/games/openk/join", serde_json::json!({ "seat": 1 })).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = post_as(&state, "red@wire.test", "/api/games/wire/chat", serde_json::json!({ "text": "hello" })).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        assert_eq!(state.games.locks.lock().expect("the map").len(), 0, "a finished write left its entry");
    }
```

(b) In `tables.rs`'s `mod tests`, a source scan beside the `RaceStore` cells (:3489):

```rust
    /// **Every writer holds a table's lock through `with_table_lock`** (finding
    /// B8, decision D-3): the helper is where "drop, then prune" lives, so this
    /// file must not take a lock by hand. The one site allowed to call
    /// `lock_for` is the helper itself, in `games.rs`.
    ///
    /// The scan reads the CODE, cut at the file's first column-0
    /// `#[cfg(test)]` (the layer's scan convention): uncut, this cell's own
    /// two literals match, so the first assertion could never pass and the
    /// count would carry one helper call too many.
    #[test]
    fn no_writer_in_this_file_takes_a_lock_by_hand() {
        let whole = include_str!("tables.rs");
        let source = whole.split("\n#[cfg(test)]\n").next().expect("the code above the tests");
        assert!(source.len() < whole.len(), "the cut found the test module");
        assert!(!source.contains("lock_for("), "a writer calls lock_for directly — use with_table_lock");
        assert!(source.matches("with_table_lock(").count() >= 7, "the seven writers go through the helper");
    }
```

(c) In `games.rs`'s `mod tests`, beside the `prune_lock` cells (:3115–3150):

```rust
    /// `with_table_lock` serializes two bodies at one id, lets a body's early
    /// return through, and leaves no entry behind — while a SECOND holder keeps
    /// the entry alive until it too is done, so two writers in flight
    /// serialize on the SAME lock (the property `prune_lock`'s doc promises).
    ///
    /// **Both holders are the helper, so the end holds by construction**
    /// (the Task 2 review's I-2): each prunes after dropping its own clone, so
    /// whichever prunes LAST reads a count of one. A bare `lock_for` holder
    /// never prunes, and the waiter's prune could then land before that
    /// holder's drop and keep the entry — measured, 45 runs in 5,000 under
    /// load. The waiter's clone is read off the entry's count, with a bound,
    /// rather than assumed after a sleep: three is the map, the first holder
    /// and the waiter, one lock.
    #[test]
    fn with_table_lock_runs_the_body_serialized_and_prunes_on_the_way_out() {
        use std::sync::mpsc::channel;
        use std::time::{Duration, Instant};
        let games = Arc::new(games());
        let out: Result<u8, String> = games.with_table_lock("t", || Err("early".to_string()));
        assert_eq!(out, Err("early".to_string()));
        assert_eq!(games.locks.lock().expect("the map").len(), 0);

        let (arrived_tx, arrived_rx) = channel();
        let (release_tx, release_rx) = channel::<()>();
        let g1 = Arc::clone(&games);
        let first = std::thread::spawn(move || {
            g1.with_table_lock("t", || {
                arrived_tx.send(()).expect("the cell listens");
                release_rx.recv().expect("the cell releases");
                1u8
            })
        });
        arrived_rx.recv_timeout(Duration::from_secs(5)).expect("the first holder is inside its body");
        let g2 = Arc::clone(&games);
        let waiter = std::thread::spawn(move || g2.with_table_lock("t", || 7u8));
        let holders = || games.locks.lock().expect("the map").get("t").map_or(0, Arc::strong_count);
        let deadline = Instant::now() + Duration::from_secs(5);
        while holders() < 3 {
            assert!(Instant::now() < deadline, "the waiter took no clone of the entry within 5 s: {}", holders());
            std::thread::sleep(Duration::from_millis(2));
        }
        // Held a moment longer, so a waiter that runs without the lock has
        // the time to finish; one that waits on it cannot, by construction.
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(holders(), 3, "the entry is alive, one lock, while the waiter waits on it");
        assert!(!waiter.is_finished(), "the body ran while another holder had the lock");
        release_tx.send(()).expect("the first holder listens");
        assert_eq!(first.join().expect("the first holder"), 1);
        assert_eq!(waiter.join().expect("the waiter"), 7);
        assert_eq!(games.locks.lock().expect("the map").len(), 0, "the last one out pruned it");
    }
```

(`games()` is `test_support`'s constructor, `games.rs:3180`, in scope through `use super::test_support::*`.)

(d) In `api.rs`'s `mod tests` (the implementer's, 2026-10-10 — C9): the 404 read off the IDENTITY, since "before
any handler" means before `answer` asks for one; a refused HEAD bodiless (C2's catcher); every `{id}` pattern
binds `{id}` first (the `bound[0]` premise):

```rust
    /// **An unsafe `{id}` is `dispatch`'s own 404, BEFORE any handler**
    /// (security S1c, finding B8). "Before any handler" is read off the
    /// identity: `answer` asks for it ahead of every game route, so a refused
    /// identity at a SAFE id is the 401 and at an UNSAFE one the 404. The
    /// boundary is `is_safe_id`'s — 64 safe bytes reach the handler and a 65th
    /// does not — and the check reads `bound[0]` because every pattern that
    /// carries `{id}` binds it FIRST, which the last loop pins. A HEAD of an
    /// unsafe id leaves with no body, as every refused HEAD does.
    #[test]
    fn an_unsafe_id_is_dispatchs_own_404_before_any_handler() {
        fn ask(games: &Games, method: &str, path: &str, identity: Identity<'_>) -> ApiResponse {
            dispatch(games, &LOCAL_HOST, identity, &ApiRequest { method, path, query: "", content_type: None, body: b"" })
        }
        let games = crate::games::test_support::games();
        let red = User::parse("red@t.test").expect("an address");
        let (safe, unsafe_) = ("a".repeat(64), "a".repeat(65));
        let sentence = |id: &str| error_body(&ApiError::not_found(format!("no game named \"{id}\"", id = crate::echo::echo(id))));

        let res = ask(&games, "GET", &format!("/api/games/{safe}/table"), Err("no identity"));
        assert_eq!(res.status, 401, "64 safe bytes reach `answer`, which asks the identity first: {}", res.body);
        let res = ask(&games, "GET", &format!("/api/games/{unsafe_}/table"), Err("no identity"));
        assert_eq!((res.status, res.body), (404, sentence(&unsafe_)), "a 65th byte is refused before the identity is asked");
        for id in ["..", ".hidden", "a b", "-x", "_house", "caf\u{e9}"] {
            let res = ask(&games, "GET", &format!("/api/games/{id}"), Err("no identity"));
            assert_eq!((res.status, res.body), (404, sentence(id)), "{id:?} is not a name the store keeps");
        }

        let res = ask(&games, "GET", &format!("/api/games/{safe}/table"), Ok(&red));
        assert_eq!((res.status, res.body), (404, sentence(&safe)), "the handler's own absence, the same sentence");
        let res = dispatch(
            &games,
            &LOCAL_HOST,
            Ok(&red),
            &ApiRequest { method: "POST", path: &format!("/api/games/{safe}/join"), query: "", content_type: Some("application/json"), body: br#"{"seat":1}"# },
        );
        assert_eq!((res.status, res.body), (404, sentence(&safe)), "a join at an absent safe id is the handler's 404");
        assert_eq!(games.locks.lock().expect("the map").len(), 0, "and it never touched the lock map");

        let res = ask(&games, "HEAD", &format!("/api/games/{unsafe_}"), Ok(&red));
        assert_eq!((res.status, res.body.as_str()), (404, ""), "a refused HEAD carries no body");

        for r in routes(true).iter().filter(|r| r.pattern.contains("{id}")) {
            assert_eq!(r.pattern.find('{'), r.pattern.find("{id}"), "{} binds {{id}} first", r.pattern);
        }
    }
```

(e) In `tables.rs`'s `mod tests` (the implementer's — C3): the pre-read's one observable effect is that a
fabricated id never makes an entry even for the length of one call, which cell (a) CANNOT see, because the
helper prunes on every exit; this cell parks each writer's FIRST table read in a `ReadGate` store and counts the
map while it waits — red under each of the six dropped pre-reads, naming the writer (`settle` is no row: at an
absent id its own first read decides there is nothing to repair, and it never locks):

```rust
    /// A [`MemStore`] that parks the FIRST table read after it is armed, so a
    /// cell can look at the lock map while a writer waits on that read.
    struct ReadGate {
        inner: MemStore,
        armed: std::sync::atomic::AtomicBool,
        arrived: Mutex<std::sync::mpsc::Sender<()>>,
        release: Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl Store for ReadGate {
        fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
            self.inner.list(doc)
        }
        fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
            if doc == Doc::Table && self.armed.swap(false, std::sync::atomic::Ordering::SeqCst) {
                let _ = self.arrived.lock().expect("the gate").send(());
                let _ = self.release.lock().expect("the gate").recv();
            }
            self.inner.read(doc, id)
        }
        fn write_if_version(&self, doc: Doc, id: &str, json: &str, expected: Option<Version>) -> Result<Version, WriteError> {
            self.inner.write_if_version(doc, id, json, expected)
        }
        fn delete(&self, id: &str) -> Result<(), String> {
            self.inner.delete(id)
        }
    }

    /// **A writer at an absent id never makes a lock entry, not even for the
    /// length of its own call** (finding B8: the read before the lock). The
    /// helper prunes on every exit, so a count taken AFTER a refused write
    /// cannot tell a writer that read first from one that locked first — the
    /// entry is gone either way, and dropping a pre-read passes every other
    /// cell. This one parks each writer's FIRST table read and counts the map
    /// while it waits: read first, the map is empty; locked first, the entry
    /// is there. `settle` is not a row: at an absent id it never locks, since
    /// its own first read decides whether there is a repair to lock for.
    #[test]
    fn a_writer_at_an_absent_id_reads_before_it_takes_the_lock() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (arrived_tx, arrived_rx) = channel();
        let (release_tx, release_rx) = channel();
        let store = Arc::new(ReadGate {
            inner: MemStore::default(),
            armed: AtomicBool::new(false),
            arrived: Mutex::new(arrived_tx),
            release: Mutex::new(release_rx),
        });
        let games = Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>));
        type Writer = fn(&Games, &User) -> Result<(), GameError>;
        let rows: [(&str, Writer); 6] = [
            ("join", |g, u| g.join("nosuch", u, PlayerId::new(1), None, None).map(drop)),
            ("keep", |g, u| g.keep("nosuch", u, PlayerId::new(0), 1, None).map(drop)),
            ("unlock", |g, u| g.unlock("nosuch", u, "x").map(drop)),
            ("delete", |g, u| g.delete("nosuch", u)),
            ("chat_append", |g, _| g.chat_append("nosuch", red_seat(), "hi").map(drop)),
            ("delete_chat", |g, u| g.delete_chat("nosuch", 1, u)),
        ];
        for (name, writer) in rows {
            store.armed.store(true, Ordering::SeqCst);
            let running = {
                let games = Arc::clone(&games);
                std::thread::spawn(move || writer(&games, &who("red@t.test")))
            };
            arrived_rx.recv_timeout(Duration::from_secs(10)).unwrap_or_else(|_| panic!("{name} reads the table"));
            let entries = games.locks.lock().expect("the map").len();
            release_tx.send(()).expect("the parked read is listening");
            let answer = running.join().expect("the writer's thread");
            assert_eq!(entries, 0, "{name} held a lock entry during its first read: it locked before it read");
            assert!(matches!(answer, Err(GameError::NotFound(_))), "{name}: {answer:?}");
        }
        assert_eq!(games.locks.lock().expect("the map").len(), 0);
    }
```


- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web -- lock_map_empty takes_a_lock_by_hand
with_table_lock 2>&1 | grep -E '^error|test result|panicked' | head` — `with_table_lock` not found; stubbed as
`let lock = self.lock_for(id); let _s = enter(&lock); body()`: (a) RED at its PREMISE line (`start_as`'s keeps leave `wire`'s entry
under the stub — the plan's draft "300 after the loop; 302 after the big and the long ids" was never measured and is
false: the six writers of an iteration share ONE id, and `chat_post`/`chat_delete` call `admitted` before their
writers, so the stub reads 1 at the premise and 51 after the loop, and the 16 KB row is the store's 500); (b) RED
on `lock_for(` (and, uncut, on its own literal — C1); (c) RED at its FIRST count, after the early return (the stub
never prunes); (d) RED at the 65-byte row (a 401: the identity is asked first); (e) written after M9 survived.

- [ ] **Step 3: The code.** In `games.rs`, after `prune_lock` (:636):

```rust
    /// **The one way a writer holds a table's own lock** (security S1c, finding
    /// B8): take it, run `body` with it held, let go of this clone, and prune
    /// the entry if the map's is now the only reference. The review grew the
    /// map without bound with refused writes at fabricated ids — `lock_for`
    /// inserts on first use and only `delete` pruned — so now every writer
    /// READS the table before calling this (an absent table is a 404 that never
    /// touches the map) and every exit prunes. A second writer in flight holds
    /// its own clone, so the entry stays and the two serialize on one lock
    /// (`prune_lock`'s argument).
    pub(crate) fn with_table_lock<T>(&self, id: &str, body: impl FnOnce() -> T) -> T {
        let lock = self.lock_for(id);
        let out = {
            let _serialized = enter(&lock);
            body()
        };
        drop(lock);
        self.prune_lock(id);
        out
    }
```

`lock_for`'s doc (:604–615): "The entry is created on first use, so a table nobody has written costs nothing,
and a DELETE prunes it once nobody else holds it" → "The entry is created on first use and PRUNED by every
writer on its way out (`with_table_lock`), so a table nobody is writing costs nothing — true since security S1c,
finding B8; before it only a delete pruned, and a refused write at a fabricated id left an entry for ever."

In `tables.rs`, each writer: the table read FIRST, outside any lock, as the absence check; then
`self.with_table_lock(id, || { … the body as it is today, re-reading under the lock … })`. Written out for
`join` (:1574–1588):

```rust
    pub fn join(&self, id: &str, user: &User, seat: PlayerId, name: Option<&str>, password: Option<&str>) -> Result<TableView, GameError> {
        // Absent before locked (security S1c, finding B8): a fabricated id is a
        // 404 that never creates a lock entry. The read under the lock below is
        // the one the write is checked against; this one only says "there".
        self.read_table(id)?.ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
        self.with_table_lock(id, || {
            let (mut doc, version) = self
                .read_table(id)?
                .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
            // … the body from `match doc.status {` on, verbatim …
        })
    }
```

and the same shape for `chat_append`, `delete_chat`, `unlock`, `keep` (the pre-read is `read_table`; inside,
`settle_locked` as today), `delete` (the pre-read; inside, `delete_locked`; the trailing `if deleted.is_ok() {
self.prune_lock(id) }` removed — the helper prunes) and `settle`'s locked arm (`self.with_table_lock(id, ||
self.settle_locked(id))`). The `?` inside a closure returns from the closure, which is what every body wants;
each closure's return type is the function's `Result` — inferred when the `with_table_lock` call is the
function's TAIL expression (`join`, `keep`, `unlock`, `chat_append`, `delete_chat`, `delete`, `settle`'s arm all
can be); a call that is not the tail needs `|| -> Result<_, GameError> { … }` spelled out. Task 1 landed every
`no game named` sentence in the named-argument form (`\"{id}\"` with `id = crate::echo::echo(id)`); the pre-read
line and the dispatch check use the same form.

In `api.rs`, `dispatch` (:2503), after the match:

```rust
    // **An `{id}` the store would refuse is a 404 here, before any handler**
    // (security S1c, finding B8): the id is the raw segment, and a 16 KB one
    // reached `lock_for` as a key. `is_safe_id` is the store's own rule, so
    // nothing this refuses could have been a table. An arm of `out` and not an
    // early return, so the HEAD below leaves this refusal bodiless too.
    let out = if pattern.contains("{id}") && !crate::store::is_safe_id(bound[0]) {
        error_response(ApiError::not_found(format!("no game named \"{id}\"", id = crate::echo::echo(bound[0]))))
    } else {
        match answer(games, host, identity, req, pattern, &bound) {
            Ok(r) => r,
            Err(e) => error_response(e),
        }
    };
```

`matches`' doc (:276–281): the sentence "Bound, it reached `get_table` and came back as the store's 'not a usable
game name', a 500" gains "— and since security S1c an id the store would refuse is `dispatch`'s own 404 before
any handler (finding B8)". `dispatch`'s own doc (the reviewer's M-4) names its THREE pre-`answer` refusals: the
405, the no-route 404 and the unsafe-`{id}` 404, which comes before `answer`'s 401 and 400.

- [ ] **Step 4: Run the cells, the crate and the lane's rows.** `cargo test -p yokohama-web` — the route cells
that drove a dotted or odd id through a handler now read the 404 where they read a 500 (the pre-flight lists
them; expected: none — `matches`' own cell reads the empty segment, which is `NoRoute`); `wire_fixtures` green
without the flag.

- [ ] **Step 5: The mutants.** **M8** `with_table_lock` without the prune — (a) red at the PREMISE, (c) red at its FIRST count (after the
early return), `a_deleted_tables_lock_is_pruned` red too; **M9** `join`'s pre-read dropped — SURVIVES (a), (b) and
(c) (the draft's "the join row's 50 entries" cannot happen: the helper prunes on every exit, so a writer that locked
first leaves the map as empty as one that read first — the implementer's C3); (e) red, naming `join`; **M9.1–M9.5**
each other pre-read dropped (`chat_append`, `delete_chat`, `unlock`, `keep`, `delete`) — (e) red, naming the
writer; **M10** the `dispatch` check dropped — (a) red at the 16 KB row (500 ≠ 404; NO entry — the pre-read's
store refusal comes before any lock, so the draft's "and an entry" was false), (d) red at the 65-byte row (401 ≠
404); **M11** a writer restored to `lock_for` by hand — (b) red; **MH** the check as the draft's early `return`
— (d) red at the HEAD row (a 404 WITH a body: the early return skipped `dispatch`'s HEAD strip — C2); **MI** the
check behind the identity — (d) red at the 65-byte row; **MS** the helper never enters the lock — (c) red at
`!waiter.is_finished()` (fix round 1: (c) rewritten — the first holder a helper too, a bounded wait until the entry's
`strong_count` reads 3 and holds for 20 ms, so the last prune reads 1 by construction; the reviewer's race loop
read 5/5,000 idle and 45/5,000 at 2× load on the draft, 0 and 0 on the rewrite; **MP** red on it too). Noted (C8): a body that PANICS skips the prune — the entry can only be a real table's
(the pre-read guarantees it) and the next writer there prunes it; not a growth vector.

- [ ] **Step 6: Commit.** `git add crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs
crates/yokohama-web/src/api.rs crates/yokohama-web/src/lib.rs` · `git commit -m "web: B8 — every writer reads before it locks and prunes on the
way out through Games::with_table_lock; an unsafe {id} is dispatch's 404 before any handler (S1c Task 2)"`.

---

### Task 3: B9 — `Cache-Control: no-store` under `/api`; no dot in a creator-chosen id — Opus (B9; ruling 3; Review Focus 2)

**Files:**
- Modify: `crates/yokohama-web/src/lib.rs` (`app()` :1114–1134; `no_store` new beside it),
  `crates/yokohama-web/src/tables.rs` (`create` :826–830; `DOTTED_ID` beside `SEED_NOT_SOLO` :756)
- Test: `lib.rs`'s and `tables.rs`'s `mod tests`

**Interfaces:**
- Consumes: `echo::echo` (Task 1); `api::routes(true)`.
- Produces: the header on every `/api` answer; `tables::DOTTED_ID`.

- [ ] **Step 1: The failing cells.**

(a) In `lib.rs`'s `mod tests`, beside `session_routes_names_every_game_route_of_the_router` (:3339):

```rust
    /// **Every answer under `/api` is `no-store`** (security S1c, finding B9:
    /// the zone cached a `.js`-named table's private frame at the edge for a
    /// signed-in attacker, `HIT` after `HIT`). Every row of the layer's table
    /// with every method it takes and HEAD beside GET, the three transport
    /// routes, the art manifest, a 404 under `/api`, a 405 — all read the
    /// header; the static release outside `/api` does not read it from this
    /// layer (`/shell.bin` reads it from `assets.rs`'s own rule and is not
    /// asked here). The mutant that drops the layer fails the first row; the
    /// one that tests `starts_with("/api/")` alone fails `/api`, the one path
    /// that test changes. `/api/` and `/api` are the two rows the FALLBACK
    /// answers — every other path under `/api/` is the adapter's catch-all,
    /// `/api/nothing` included — so they are what fails `route_layer`, which
    /// wraps the routes and not the fallback; and `/apix` fails a bare
    /// `starts_with("/api")`, which would mark a path that is not the API.
    #[tokio::test]
    async fn every_answer_under_api_is_no_store_and_the_static_release_is_not() {
        let state = Arc::new(AppState::in_memory());
        a_game(&app(Arc::clone(&state)), "x", &[]).await;
        let mut rows: Vec<(String, String)> = api::routes(true)
            .into_iter()
            .map(|r| (r.method.to_string(), r.pattern.replace("{id}", "x").replace("{seq}", "1").replace("{k}", "0").replace("{kind}", "orders")))
            .collect();
        for (m, p) in rows.clone() {
            if m == "GET" {
                rows.push(("HEAD".to_string(), p));
            }
        }
        for p in ["/api/games/x/events", "/api/games/x/stream", "/api/stream", "/api/nothing", "/api/games/x/no/such", "/api/", "/api"] {
            rows.push(("GET".to_string(), p.to_string()));
        }
        rows.push(("PUT".to_string(), "/api/games".to_string()));
        for (method, path) in &rows {
            let req = Request::builder().method(method.as_str()).uri(path).header(identity::CLAIMED_HEADER, WHO)
                .header("content-type", "application/json").body(Body::from("{}")).expect("a request");
            let res = app(Arc::clone(&state)).oneshot(req).await.expect("the router answers");
            assert_eq!(
                res.headers().get(http::header::CACHE_CONTROL).and_then(|v| v.to_str().ok()),
                Some("no-store"),
                "{method} {path} answered {} without no-store",
                res.status()
            );
        }
        // Outside `/api` the layer marks nothing: the static release's headers
        // are `assets.rs`'s own rule (`cache_control`, :210), which marks the
        // shell `no-store` and leaves the fingerprinted assets alone. The
        // paths below are ones that rule leaves UNMARKED — the pre-flight
        // confirms each against `assets::cache_control` and swaps in another
        // unmarked path if one of these is not. `/apix` is the SPA fallback's,
        // which that rule leaves unmarked too: a name that only BEGINS like
        // the API's.
        for path in ["/art/nothing.png", "/assets/nothing-abc123.js", "/apix"] {
            let req = Request::builder().method("GET").uri(path).body(Body::empty()).expect("a request");
            let res = app(Arc::clone(&state)).oneshot(req).await.expect("the router answers");
            assert_ne!(
                res.headers().get(http::header::CACHE_CONTROL).and_then(|v| v.to_str().ok()),
                Some("no-store"),
                "{path}: the static release is not this layer's to mark"
            );
        }
    }
```

(The stream rows answer `text/event-stream` and hold the connection; `oneshot` returns once the headers are
written, and the body is dropped unread — the implementer confirms the cell returns, since a stream that never
yields a header would hang it; the pre-flight measures the cell's wall. `assets::cache_control` is private to
`assets.rs`; the pre-flight reads its rule rather than calling it.)

(b) In `tables.rs`'s `mod tests`, beside the seed cells (`a_creators_seed_is_honoured_at_a_solo_table_only`):

```rust
    /// **A creator-chosen id with a dot is refused at create** (finding B9's
    /// second half): `evening.js` named a private frame to the edge's
    /// extension rule. The storage rule is untouched — a table stored before
    /// this cycle under `old.js` still loads, plays and deletes — so the fence
    /// is the create's alone. Every id the corpus and the client use is dotless.
    #[test]
    fn a_dotted_id_is_refused_at_create_and_a_stored_one_still_plays() {
        let games = games();
        let red = who("red@t.test");
        for id in ["x.js", "x.css", "x.png", "x.bin", "evening.2", "a.b.c"] {
            let mut req = two_people("dotted", CORPUS_SEED);
            req.id = Some(id.to_string());
            let err = games.create(&req, &red).expect_err(id);
            assert!(matches!(&err, GameError::BadRequest(m) if m == &format!("\"{id}\" {DOTTED_ID}")), "{id}: {err:?}");
        }
        // **The ORDER, by two-defect probes** (the M6.2 lesson): an id the
        // storage rule refuses AND that carries a dot reads the storage rule's
        // sentence, so the dotted arm sits after `is_safe_id`'s and never
        // speaks for an id no table could have been stored under.
        for id in ["..", "x.table"] {
            let mut req = two_people("dotted", CORPUS_SEED);
            req.id = Some(id.to_string());
            let err = games.create(&req, &red).expect_err(id);
            assert!(matches!(&err, GameError::BadRequest(m) if m == &format!("\"{id}\" is not a usable game id")), "{id}: {err:?}");
        }
        let mut req = two_people("dash-ok", CORPUS_SEED);
        req.id = Some("x-js".to_string());
        games.create(&req, &red).expect("a dash is fine");

        // Stored before the rule: a real document — the dotless table's, re-keyed
        // through the store under `old.js` — reads and deletes; the route-level
        // drive (`lib.rs`'s `a_stored_dotted_table_still_loads_plays_and_deletes`)
        // plays and streams one.
        games.create(&two_people("oldjs", CORPUS_SEED), &red).expect("a table");
        let (json, _) = games.store().read(Doc::Table, "oldjs").unwrap().expect("the table");
        let mut doc: TableDoc = serde_json::from_str(&json).unwrap();
        doc.id = "old.js".to_string();
        games.store().write_if_version(Doc::Table, "old.js", &serde_json::to_string(&doc).unwrap(), None).unwrap();
        assert!(games.table_for_anyone("old.js", &red).is_ok(), "a stored dotted table still reads");
        games.delete("old.js", &red).expect("and deletes");
    }
```

(`games()`, `who()`, `two_people(id, seed)` and `CORPUS_SEED` are `test_support`'s, in scope through
`use crate::games::test_support::*`; no `TableDoc` builder exists, so the stored dotted document is a REAL one
re-keyed through `games.store()`.)

(c) In `lib.rs`'s `mod tests`, after `const WHO` — Review Focus 2's route-level half, since cell (b) reads and
deletes but plays nothing:

```rust
    /// **A table stored before the dot rule still loads, plays, streams and
    /// deletes through the routes** (Review Focus 2): a solo hot seat is
    /// re-keyed under `old.js` through the store — both documents, the table's
    /// `id` rewritten — and `GET`, an input, the two transport routes and the
    /// delete all answer, because the storage rule and `dispatch`'s `{id}`
    /// check admit a dot; only `create` refuses one. The transport routes never
    /// reach `dispatch`, so their rows pin that their own admission admits a
    /// dot too.
    #[tokio::test]
    async fn a_stored_dotted_table_still_loads_plays_and_deletes() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "oldjs", 20260831, &[WHO, WHO]).await;
        let store = Arc::clone(state.games.store());
        let (json, _) = store.read(Doc::Table, "oldjs").expect("a store").expect("the table");
        let mut doc: crate::table::TableDoc = serde_json::from_str(&json).expect("the table parses");
        doc.id = "old.js".to_string();
        store.write_if_version(Doc::Table, "old.js", &serde_json::to_string(&doc).expect("json"), None).expect("written");
        let (json, _) = store.read(Doc::Record, "oldjs").expect("a store").expect("the record");
        store.write_if_version(Doc::Record, "old.js", &json, None).expect("written");
        let (status, frame) = get_as(&state, WHO, "/api/games/old.js").await;
        assert_eq!(status, StatusCode::OK, "{frame}");
        let (status, after) = post_as(&state, WHO, "/api/games/old.js/input", serde_json::json!({ "line": "1" })).await;
        assert_eq!(status, StatusCode::OK, "{after}");
        assert_ne!(frame["events"], after["events"], "the input landed: {after}");
        let (status, events) = get_as(&state, WHO, "/api/games/old.js/events").await;
        assert_eq!(status, StatusCode::OK, "{events}");
        assert_eq!(events["events"], after["events"], "the reading is the stored dotted table's: {events}");
        let (status, text, _) =
            stream_frames(Arc::clone(&state), "/api/games/old.js/stream", WHO, std::time::Duration::from_millis(100)).await;
        assert_eq!(status, StatusCode::OK, "{text}");
        assert!(sse_frames(&text).iter().any(|f| f.starts_with("event: signal")), "the on-connect frame: {text}");
        let (status, body) = delete_as(&state, WHO, "/api/games/old.js").await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    }
```

(Unexecuted by the pre-flight: the implementer confirms `"1"` is an option at the opening and that `start_as`
with one address at both seats opens a solo hot seat, and adjusts the line or the seats if not.)

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web -- no_store dotted_id 2>&1 | grep -E
'^error|test result|panicked' | head` — `DOTTED_ID` not found; stubbed: (a) RED on the first row (no header);
(b) RED on `x.js` (created); (c) GREEN already — the storage rule admits dots, and the cell pins what the
dotted arm must NOT touch (the filter `no_store dotted_id` does not select (c): run it by name).

- [ ] **Step 3: The code.** In `lib.rs`, beside `app()`:

```rust
/// **Every answer under `/api` is `no-store`** (security S1c, finding B9). The
/// zone caches by file EXTENSION by default, and the review named a table
/// `evening.js` and read the victim's private frame from the edge; the header
/// is what keeps an API answer out of every cache, whatever the path spells.
/// One layer over the whole router, keyed on the path, so the adapter, the
/// three transport routes, the art manifest and the fallback's own 404 all
/// carry it, and the static release outside `/api` keeps `assets.rs`'s rule.
#[cfg(feature = "server")]
async fn no_store(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let path = req.uri().path();
    let api = path == "/api" || path.starts_with("/api/");
    let mut res = next.run(req).await;
    if api {
        res.headers_mut().insert(http::header::CACHE_CONTROL, http::HeaderValue::from_static("no-store"));
    }
    res
}
```

and in `app()`, after `.fallback(fallback)` and before `.with_state(state)`:
`.layer(axum::middleware::from_fn(no_store))` — `Router::layer`, not `route_layer`: the layer must wrap the
fallback, which answers `/api/nothing`.

In `tables.rs`, beside `SEED_NOT_SOLO` (:756):

```rust
/// The sentence a creator-chosen id with a dot is refused with (security S1c,
/// finding B9): a cache keys a path's last dot-suffix as a file type, and a
/// table named `evening.js` is a private frame the edge would keep. The
/// storage rule (`store::is_safe_id`) still admits dots, so a table stored
/// before this cycle loads; only a CREATE refuses them.
pub const DOTTED_ID: &str = "is not a usable game id: a dot names a file type to a cache, and a table's id has none";
```

and in `create` (:826–830), between the `!is_safe_id` arm and the `Some(id) => id.clone()` arm:

```rust
            Some(id) if id.contains('.') => {
                return Err(GameError::BadRequest(format!("\"{}\" {DOTTED_ID}", crate::echo::echo(id))))
            }
```

- [ ] **Step 4: Run the cells, the crate and the lane's rows**, `wire_fixtures` green without the flag (no fixture
carries headers; no fixture creates a dotted id — the pre-flight's grep over `web/src/wire/*.json` for `"id":
"[^"]*\.` reads nothing).

- [ ] **Step 5: The mutants.** **M12** the layer dropped — (a) red on the first row; **M12b** `route_layer` in place of `layer` — (a) red at
`/api/` (the reviewer's reading: the trailing slash is the fallback's; `/api` exactly reds M13) (the draft's reason, `/api/nothing`, is the adapter's catch-all, not the fallback's —
the implementer's C2, with rows `/api`, `/api/` and `/apix`); **M13** `starts_with("/api/")` alone — (a) red at
`/api` exactly (the draft's `POST /api/games` starts with `/api/` and could not tell); **M14** the dotted arm
dropped — (b) red at `x.js`; **M15** the dotted arm placed BEFORE the `is_safe_id` arm — (b) red at its two-defect
pin (`..` and `x.table` read `is_safe_id`'s sentence, never `DOTTED_ID`'s; the draft left the order unpinned and
M15 survived every cell — C3); (c) reads the events and one stream frame of the stored dotted table (Review Focus
2's "streams" — C4), which also refuses a Task 4 transport check stricter than `is_safe_id`; **M15b** the dot refused in `store::is_safe_id` instead of `create` — (b) red in its FIRST loop (the
reviewer's reading: `x.js` reads `is_safe_id`'s sentence, not `DOTTED_ID`'s) and (c) red (the stored table is
refused).

- [ ] **Step 6: Commit.** `git add crates/yokohama-web/src/lib.rs crates/yokohama-web/src/tables.rs` · `git
commit -m "web: B9 — every answer under /api is Cache-Control: no-store from one layer; a creator-chosen id with
a dot is refused at create (S1c Task 3)"`.

---

### Task 4: D1 — a refused stream ends; the `events` hand-off's catcher; no permit without an identity — Opus (D1; S1b's residuals 5 and 6; decisions D-4, D-5; Review Focus 3)

**Files:**
- Modify: `crates/yokohama-web/src/lib.rs` (`stream` :804–849 and its doc :761–803; `UntilLast` new; `adapter`
  :945–951), `crates/yokohama-web/src/api.rs` (`events_at` — the unsafe-id 404, Task 2's C6; `answer` — the
  HEAD fold, Task 3's C1), `worker/src/table/sockets.ts` (the `admit` doc) and `worker/test/sockets.test.ts` (the
  unsafe-id upgrade: the implementer's ruled patch — C6's 404 reaches the Worker's socket upgrade, which refuses an
  unsafe id as an unknown id's: accepted, the sentence, closed 4404, nothing written, where the store's 500 passed
  through unupgraded; the "otherwise" test keeps its 401 case alone, the pool unable to make `events` fail with a
  500 — a coverage residual)
- Test: `lib.rs`'s `mod tests`; the Worker pool's `sockets.test.ts`

**Interfaces:**
- Consumes: `stream_frames`, `start_as`, `post_as`, `get_as`, `slow_state`, `archive_body`, `WHO`.
- Produces: `UntilLast`; the permit condition.

- [ ] **Step 1: The failing cells.**

(a) Beside `the_stream_refuses_a_stranger_before_it_subscribes` (:7815):

```rust
    /// **A refusal ENDS the stream** (security S1c, finding D1): a stranger
    /// holding an open table's stream reads `end` with `gone` when a join fills
    /// the last seat — and the connection CLOSES behind that frame, where
    /// before it stayed open and wrote one `gone` per later bump: an activity
    /// oracle on a table the caller was refused. A delete still ends with
    /// `deleted` (the earlier cell), and a transient store failure still writes
    /// a comment and stays open
    /// (`a_failed_reading_writes_a_comment_and_the_stream_stays_open_for_the_next_bump`). The mutant that never marks
    /// the frame last reads `ended == false` and a second `gone` after the
    /// second bump.
    #[tokio::test]
    async fn a_refused_stream_writes_gone_once_and_closes() {
        let state = Arc::new(AppState::in_memory_with_heartbeat(std::time::Duration::from_secs(25)));
        let (status, body) = post_as(&state, "red@wire.test", "/api/games", serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
            "seats": [{ "user": "red@wire.test" }, { "open": true }], "id": "filling", "name": "filling",
        })).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let s = Arc::clone(&state);
        let reader = tokio::spawn(async move {
            stream_frames(s, "/api/games/filling/stream", "green@wire.test", std::time::Duration::from_secs(2)).await
        });
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let (status, body) = post_as(&state, "blue@wire.test", "/api/games/filling/join", serde_json::json!({ "seat": 1 })).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        // A second bump after the refusal: a stream that stayed open would
        // write a second `gone` here.
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let (status, body) = post_as(&state, "blue@wire.test", "/api/games/filling/keep", serde_json::json!({ "seat": 1, "card": 1 })).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, text, ended) = reader.await.expect("the reader task");
        assert_eq!(status, StatusCode::OK, "{text}");
        let frames = sse_frames(&text);
        assert!(frames.first().is_some_and(|f| f.starts_with("event: signal")), "the on-connect frame: {text}");
        let gone: Vec<&&str> = frames.iter().filter(|f| f.starts_with("event: end") && f.contains("gone")).collect();
        assert_eq!(gone.len(), 1, "exactly one gone frame: {text}");
        assert!(frames.last().is_some_and(|f| f.contains("gone")), "gone is the LAST frame: {text}");
        assert!(ended, "the gone frame arrived but the body stayed open: {text:?}");
    }
```

(b) Beside `two_slow_replays_leave_the_server_answering` (find by content):

```rust
    /// **The `events` route's reading runs on the blocking pool** (security S1b,
    /// finding D6; the catcher S1b left owed — spec §9.4 residual 5). On a
    /// ONE-thread runtime a table read that sleeps 300 ms inside `events`
    /// leaves a health read answering in a few milliseconds, because the sleep
    /// is on a pool thread; the mutant that reads inline holds the runtime's
    /// only thread for the 300 ms and the health read waits behind it. The
    /// clock runs from BEFORE the yield, because the yield is where the
    /// spawned reading first runs: with the reading inline the 300 ms pass
    /// inside it, and a clock started after it times the health read alone.
    #[tokio::test(flavor = "current_thread")]
    async fn the_events_reading_leaves_the_runtime_thread_free() {
        struct Slow(MemStore);
        impl Store for Slow {
            fn list(&self, doc: Doc) -> Result<Vec<crate::store::GameId>, String> { self.0.list(doc) }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, crate::store::Version)>, String> {
                if doc == Doc::Table && id == "slowt" { std::thread::sleep(std::time::Duration::from_millis(300)); }
                self.0.read(doc, id)
            }
            fn write_if_version(&self, doc: Doc, id: &str, json: &str, expected: Option<crate::store::Version>) -> Result<crate::store::Version, crate::store::WriteError> {
                self.0.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> { self.0.delete(id) }
        }
        let state = Arc::new(AppState::with_store(Arc::new(Slow(MemStore::default())), identity::IdentityMode::Open));
        start_as(&state, "slowt", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        let s = Arc::clone(&state);
        let events = tokio::spawn(async move { get_as(&s, "red@wire.test", "/api/games/slowt/events").await });
        // The clock starts before the yield; started after it, the inline
        // mutant passed this cell (S1c Task 4, measured).
        let started = std::time::Instant::now();
        tokio::task::yield_now().await;
        let (status, _) = get_as(&state, WHO, "/api/health").await;
        let took = started.elapsed();
        assert_eq!(status, StatusCode::OK);
        assert!(took < std::time::Duration::from_millis(150), "the health read waited {took:?} behind the events reading");
        let (status, body) = events.await.expect("the events task");
        assert_eq!(status, StatusCode::OK, "{body}");
    }
```

(`start_as` itself reads `slowt`'s table through the slow store several times — the implementer accepts the
cell's wall, about two seconds, and says so in the cell's doc if it is more; the pre-flight measures it.)

(c) Beside `a_replay_past_the_gates_wait_is_refused_busy_while_a_move_lands`:

```rust
    /// **A replay-route request with NO identity takes no permit** (S1b's
    /// residual 6, decision D-5): with the gate's one permit held by a slow
    /// upload and a two-second wait, an identity-less `GET …/why` reads the
    /// layer's 401 at once — a refused caller never queues behind a real one.
    /// The mutant that takes the permit regardless QUEUES: the holder's put
    /// (1.5 s) ends inside the wait, so it reads the same 401 only once the
    /// holder lets go, about 1.4 s in (measured), and the cell reds at its
    /// clock.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_replay_route_without_an_identity_is_refused_without_a_permit() {
        let state = slow_state(1_500, 1, 2_000);
        a_game(&app(Arc::clone(&state)), "gated2", &[]).await;
        let holder = {
            let s = Arc::clone(&state);
            tokio::spawn(async move { post_as(&s, "local@browser", "/api/archive", archive_body()).await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        assert_eq!(state.replay_gate.available_permits(), 0, "premise: the holder has the permit");
        let started = std::time::Instant::now();
        let req = Request::builder().method("GET").uri("/api/games/gated2/why").body(Body::empty()).expect("a request");
        let res = app(Arc::clone(&state)).oneshot(req).await.expect("the router answers");
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert!(started.elapsed() < std::time::Duration::from_millis(500), "the 401 waited for the permit");
        let (status, v) = holder.await.expect("the holder's post");
        assert!(matches!(status.as_u16(), 200 | 201), "{v}");
    }
```

(d) Beside (b) — **the two transport routes `app()` serves itself bypass `dispatch`** (Task 2's C6, the
implementer's probe: `GET /api/games/<65 a's>/events` and `/stream` answer the store's 500 "is not a usable game
name" where every other route answers Task 2's 404): the check goes at the head of `api::events_at`, the one
reading both handlers and both hosts share, so an `{id}` `is_safe_id` refuses is `not_found("no game named
\"<echo>\"")` before any store read:

```rust
    /// **An unsafe `{id}` at the two axum-native routes is the layer's 404, not
    /// the store's 500** (security S1c, Task 2's C6): `app()` serves
    /// `…/events` and `…/stream` itself, so `dispatch`'s check never sees them;
    /// `events_at` refuses the id first, in the same sentence. The mutant that
    /// drops the check reads the store's 500 at both.
    #[tokio::test]
    async fn an_unsafe_id_at_the_transport_routes_is_the_layers_404() {
        let state = Arc::new(AppState::in_memory());
        let big = "a".repeat(65);
        for path in [format!("/api/games/{big}/events"), format!("/api/games/{big}/stream")] {
            let (status, body) = get_as(&state, WHO, &path).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{path}: {body}");
            assert!(body["error"].as_str().unwrap_or("").starts_with("no game named \"aaaa"), "{path}: {body}");
        }
    }
```

(The stream row: `get_as` on a stream URI — a 404 is answered as JSON before any SSE body, so the helper's JSON
read holds; if the helper hangs on a 200 the implementer uses `stream_frames`.) In `api.rs`, at the head of
`events_at` (find by content): `if !crate::store::is_safe_id(id) { return Err(ApiError::not_found(format!("no game
named \"{id}\"", id = crate::echo::echo(id)))); }` with a one-line comment naming C6.

(e) **Task 3's finding C1 (HIGH, pre-existing, outside B9 — a HEAD runs a route's WRITE arm; found by Task 3's
implementer on this tree's router, 2026-10-10).** `find` folds HEAD onto GET (`api.rs`, `let method = if method ==
"HEAD" { "GET" } else { method };`) so a HEAD matches a GET row, but `answer` picks the arm by the LITERAL method
(`"/api/games/{id}" if req.method == "GET"`, and the same guard at `/api/games`, `…/chat`, `/api/chat`) — so a
HEAD falls into the OTHER arm at the four paths that carry two methods: the creator's `HEAD /api/games/{id}` runs
DELETE (204, the table gone), `HEAD /api/games` with a create body runs POST (201), `HEAD …/{id}/chat` and
`HEAD /api/chat` with a text body POST. The fix, in `answer`: ONE binding at its head — `let method = if req.method
== "HEAD" { "GET" } else { req.method };` — and every `if req.method == "GET"` arm guard reads `method`, so a HEAD
takes the GET arm and leaves bodiless through the strip, as before. The cell, in `lib.rs`'s `mod tests` after
`const WHO`:

```rust
    /// **A HEAD never runs a write arm** (security S1c, Task 3's finding — pre-
    /// existing): `find` folds HEAD onto GET, and `answer` must pick its arm by
    /// the SAME folded method, else a HEAD of a two-method path takes the other
    /// arm — the creator's HEAD of a game deleted it, a HEAD of `/api/games`
    /// with a create body created one. Four paths, each HEAD answers 200 with
    /// an empty body and writes nothing: the table remains, the list does not
    /// grow, no chat message lands. The mutant that drops the fold reads a 204
    /// at the first row and the table gone.
    #[tokio::test]
    async fn a_head_never_runs_a_write_arm() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "headed", 20260831, &[WHO, "blue@wire.test"]).await;
        let head = |uri: &str, body: Option<serde_json::Value>| {
            let s = Arc::clone(&state);
            let uri = uri.to_string();
            async move {
                let req = Request::builder().method("HEAD").uri(&uri).header(identity::CLAIMED_HEADER, WHO)
                    .header("content-type", "application/json")
                    .body(Body::from(body.map(|b| serde_json::to_vec(&b).expect("a body")).unwrap_or_default()))
                    .expect("a request");
                let res = app(s).oneshot(req).await.expect("the router answers");
                let status = res.status();
                let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
                (status, bytes.len())
            }
        };
        let create = serde_json::json!({ "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
            "seats": [{ "user": WHO }, { "user": WHO }], "id": "headmade", "name": "headmade" });
        assert_eq!(head("/api/games/headed", None).await, (StatusCode::OK, 0), "the creator's HEAD of a game");
        let (status, _) = get_as(&state, WHO, "/api/games/headed").await;
        assert_eq!(status, StatusCode::OK, "the table remains");
        let (_, before) = get_as(&state, WHO, "/api/games").await;
        assert_eq!(head("/api/games", Some(create)).await, (StatusCode::OK, 0), "a HEAD of the list with a create body");
        let (_, after) = get_as(&state, WHO, "/api/games").await;
        assert_eq!(before["games"].as_array().map(Vec::len), after["games"].as_array().map(Vec::len), "nothing created: {after}");
        let (status, _) = get_as(&state, WHO, "/api/games/headmade").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "nothing created");
        for uri in ["/api/games/headed/chat", "/api/chat"] {
            let (_, before) = get_as(&state, WHO, uri).await;
            assert_eq!(head(uri, Some(serde_json::json!({ "text": "hi" }))).await, (StatusCode::OK, 0), "{uri}");
            let (_, after) = get_as(&state, WHO, uri).await;
            assert_eq!(before, after, "{uri}: a HEAD posted a message");
        }
    }
```

(The list's and the chat's JSON field names are the implementer's to read off `client.rs`/`chat.rs` — `games`,
`messages` or whatever the frame spells; the shape of the cell is the claim. Mutant **M-HEAD**: `let method =
req.method;` — the cell red at its first row, a 204 and the table gone.) The security ledger gains a row for this
at the close (Task 9 Step 6) — the review's method inventory missed it; the live acceptance (Step 10(d)) adds a
HEAD of a scratch table that must survive, on both hosts.

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --lib -- writes_gone_once
leaves_the_runtime_thread_free without_an_identity transport_routes head_never 2>&1 | grep -E 'test result|panicked' | head`
— (e) RED at its first row today (a 204; the table gone). — (a) RED at
`ended` (false) and at the gone count (**the pre-flight's reading: two `gone` frames**); (b) GREEN already — the
hand-off exists; its mutant (M17) is what proves the cell: the pre-flight reads it red under the inline mutant
(**about 300 ms**); (c) RED: a 401 after about 1.4 s — the permit taken regardless waits behind the holder, then the layer
answers its 401; the 500 ms bound catches it (the draft said "a 503 after two seconds" — the implementer's reading).

- [ ] **Step 3: The code.** In `lib.rs`, above `stream` (:804):

```rust
/// **A stream that ENDS after the frame marked last** (security S1c, finding
/// D1). `tokio-stream`'s `take_while` stops BEFORE the failing item, and the
/// `gone` frame must be written and then the body closed — the refusal was
/// written on every later bump before this, an activity oracle on a table the
/// caller was refused. A few lines over a pinned box, rather than a
/// dependency on `futures-util` for `scan`.
#[cfg(feature = "server")]
struct UntilLast<S> {
    inner: S,
    done: bool,
}

#[cfg(feature = "server")]
impl<S> tokio_stream::Stream for UntilLast<S>
where
    S: tokio_stream::Stream<Item = (axum::response::sse::Event, bool)> + Unpin,
{
    type Item = Result<axum::response::sse::Event, std::convert::Infallible>;
    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        if self.done {
            return std::task::Poll::Ready(None);
        }
        match std::pin::Pin::new(&mut self.inner).poll_next(cx) {
            std::task::Poll::Ready(Some((event, last))) => {
                self.done = last;
                std::task::Poll::Ready(Some(Ok(event)))
            }
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
```

In `stream` (:819–846): the `then` closure returns `(Event, bool)` — the delete arm
`(Event::default().event("end").data(r#"{"reason":"deleted"}"#), true)`, the signal arm `(…, false)`, the
refusal arm `(Event::default().event("end").data(r#"{"reason":"gone"}"#), true)`, the comment arm `(…, false)`
— and the handler ends `Ok(Sse::new(UntilLast { inner: Box::pin(frames), done: false
}).keep_alive(KeepAlive::new().interval(heartbeat)))`. The doc paragraph "Neither arm closes anything by itself"
(:799–803) is rewritten; as committed (the implementer made it true of the delete arm marked last too): "is a worse answer than saying nothing.  **A refusal closes the connection** (security S1c, finding D1): the `gone` frame is the stream's LAST — `UntilLast` ends the body after it — so a caller the table refused learns nothing from its later bumps. The failure arm writes its comment and stays open, as before."

In `adapter` (:945–951): `let permit = if api::is_replay_route(&method, &path) && resolved.is_ok() {` — with
the doc's gate paragraph (:915–926) gaining "A request whose identity did not resolve takes NO permit (S1c): the
five gated routes all need one, so the layer answers its 401 having replayed nothing, and a refused caller never
queues behind a real one. The route's own refusals stay under the permit — behind Access every caller is signed
in."

- [ ] **Step 4: Run the cells, the crate and the lane's rows.** The stream cells (:7560–7955) stay green — the
delete arm's `ended` was already true (the sender dropped) and is now true by two mechanisms.

- [ ] **Step 5: The mutants.** **M16** `self.done = last` → `self.done = false` — (a) red (ended false; two gone
frames); **M17** `events` reads inline (`Ok(Json(api::events_at(&state.games, &id, &user)?))`) — (b) red (305 ms against
2.3 ms on the correct code; the draft's cell started its clock AFTER the yield and never failed under M17 — the
implementer moved it before); **M18** `&& resolved.is_ok()` dropped — (c) red (a 401 after ~1.4 s, past the 500 ms bound — never a 503); **M19** the delete arm marked `false`
— the earlier end-on-delete cell stays green (the channel closes itself) — recorded as a mutant NO cell
catches, by design: the wrapper is for the refusal arm; **M19b** `events_at`'s unsafe-id check dropped — (d) red
at both routes (the store's 500); **M-HEAD** the fold in `answer` dropped — (e) red at its first row.

- [ ] **Step 6: Commit.** `git add crates/yokohama-web/src/lib.rs crates/yokohama-web/src/api.rs
worker/src/table/sockets.ts worker/test/sockets.test.ts` · `git commit -m "web: D1 — a refused stream writes gone
once and closes (UntilLast); the events reading's pool hand-off has its catcher; a replay-route request with no
identity takes no permit; an unsafe id at the transport routes is the layer's 404; a HEAD never runs a write arm;
the Worker refuses an unsafe id's upgrade as an unknown id's (S1c Task 4)"`.

---

### Task 5: D26 — the caller segment: readable plus a digest; the legacy mapping kept for the rename; the `segment` subcommand; the recipe in both runbooks — Opus (D26; ruling 2; decisions D-6, D-9; Review Focus 4)

**Files:**
- Modify: `crates/yokohama-web/src/archive.rs` (`caller_segment` :196–232 — rewritten; `legacy_caller_segment`,
  `SEGMENT_READABLE`, `SEGMENT_DIGEST` new; the cells :575–605 and :721–745), `crates/yokohama-web/src/main.rs`
  (the `segment` subcommand), `crates/yokohama-worker/src/receipt.rs` (:34 the doc; :825–827 the premise),
  `deploy/RUNBOOK.md` (a new section "The archive's rename, S1c"), `worker/RUNBOOK.md` (the same),
  `deploy/README.md:192` and `:314` (the path), `worker/test/archive.test.ts` (:1179, :1207 — two hand-written
  legacy segments), `worker/scripts/live-replay.test.mjs` (:317–335 — the four keys and `resetRecipe`'s first key
  pinned as literal `red_at_wire.test` strings), the three corpus files `web/src/wire/archive_post.json`,
  `archive_post_identical.json`, `archive_post_abandoned.json` (regenerated once: their `path` moves) and the
  Worker's generated copy `worker/test/corpus.gen.ts` (`worker/scripts/test-assets.mjs` writes it)
- Test: `archive.rs`'s `mod tests`; the Worker's receipt cells; the two TS files above

**Interfaces:**
- Consumes: `sha2` (already a dependency; `table.rs:206` uses it); `identity::User`.
- Produces: the new `caller_segment`; `legacy_caller_segment`; the subcommand.

- [ ] **Step 1: The failing cells.** In `archive.rs`'s `mod tests`, `a_caller_segment_is_path_safe_and_readable`
(:575) rewritten and two cells beside it:

```rust
    /// The segment is READABLE still — the owner reviews saves by who sent them
    /// — and injective in practice (security S1c, finding D26): the readable
    /// part is the legacy mapping cut at 48, then `-` and twelve hex characters
    /// of a SHA-256 over the whole normalized address. Every pair the review
    /// named as colliding gets two folders; the alphabet stays `[a-z0-9._-]`,
    /// the first character `[a-z0-9]`, the length at most 61; the names that
    /// WOULD break the layout are refused by the grammar.
    #[test]
    fn a_caller_segment_is_path_safe_readable_and_injective() {
        use crate::identity::User;
        let seg = |s: &str| caller_segment(&User::parse(s).unwrap());
        assert!(seg("red@example.test").starts_with("red_at_example.test-"), "{}", seg("red@example.test"));
        assert_eq!(seg("red@example.test").len(), "red_at_example.test".len() + 1 + SEGMENT_DIGEST);
        for (a, b) in [
            ("alice+x@corp.test", "alice_x@corp.test"),
            ("a b", "a_b"),
            ("a/b@c", "a_b@c"),
            ("a.b@c", "a_b@c"),
        ] {
            assert_ne!(seg(a), seg(b), "{a:?} and {b:?} share a folder");
            assert_eq!(seg(a).split('-').next_back().map(str::len), Some(SEGMENT_DIGEST));
        }
        let long_a = format!("{}@{}", "a".repeat(200), "b".repeat(50));
        let long_b = format!("{}@{}", "a".repeat(200), "c".repeat(50));
        assert_ne!(seg(&long_a), seg(&long_b), "two long addresses with one 48-prefix");
        assert!(seg(&long_a).len() <= SEGMENT_READABLE + 1 + SEGMENT_DIGEST);
        for name in [".", "..", ". .", "-", ".x"] {
            assert!(User::parse(name).is_err(), "{name:?} must never be a key");
        }
        let n32 = "n".repeat(32);
        for name in ["ben castle", "7", "x", "a.test", "z..", "r2-d2_x.y", n32.as_str(), "../../x@y", "..@.."] {
            let v = seg(name);
            assert!(v.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit()), "{name:?} → {v:?}");
            assert!(v.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')), "{v:?}");
            assert!(!v.contains("/"), "{v:?}");
        }
        // Determinism and normalization: the digest is over the KEY, so two
        // spellings of one address are one segment.
        assert_eq!(seg("Red@Example.TEST"), seg("red@example.test"));
        // The directory half of the old cell, verbatim: every traversal-shaped
        // address writes inside the root.
        let dir = std::env::temp_dir().join(format!("yokohama_archive_root_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = DirArchive::new(&dir);
        let root = std::fs::canonicalize(&dir).unwrap();
        for who in ["../../x@y", "a/b@c", "..@.."] {
            let prefix = format!("browser/{}/", seg(who));
            a.put(&prefix, Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}").unwrap();
            let (record, _) = paths(yokohama_core::SEMANTICS_REV, &prefix, Why::Over, "local-1").unwrap();
            let written = std::fs::canonicalize(dir.join(&record)).unwrap();
            assert!(written.starts_with(&root), "{who:?} wrote {written:?} outside {root:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **The legacy mapping is what it was, and the new segment differs from it
    /// for every address** — so a folder the window did not rename is
    /// detectable, and the `segment` subcommand's two columns are never equal.
    #[test]
    fn the_legacy_segment_is_the_old_mapping_and_never_the_new_one() {
        use crate::identity::User;
        let old = |s: &str| legacy_caller_segment(&User::parse(s).unwrap());
        assert_eq!(old("red@example.test"), "red_at_example.test");
        assert_eq!(old("A/B@c"), "a_b_at_c");
        assert_eq!(old("../../x@y"), ".._.._x_at_y");
        assert_eq!(old("  Ben   Castle "), "ben_castle");
        let long = format!("{}@{}", "a".repeat(200), "b".repeat(50));
        assert_eq!(old(&long).len(), 64);
        assert_eq!(old("alice+x@corp.test"), old("alice_x@corp.test"), "premise: the legacy mapping collided");
        for s in ["red@example.test", "a b", "x", &long, "alice+x@corp.test"] {
            let u = User::parse(s).unwrap();
            assert_ne!(legacy_caller_segment(&u), caller_segment(&u), "{s:?}");
        }
    }

    /// **The quota counts the NEW prefix alone**: a legacy folder left in place
    /// for an address is not the caller's share (Review Focus 4) — the window
    /// renames it, and until then it is nobody's.
    #[test]
    fn a_legacy_folder_is_not_counted_against_the_callers_share() {
        use crate::identity::User;
        let dir = std::env::temp_dir().join(format!("yokohama_legacy_share_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = DirArchive::new(&dir).with_quota(Quota { records: 1, bytes: 1 << 20 });
        let u = User::parse("red@example.test").unwrap();
        let legacy = format!("browser/{}/", legacy_caller_segment(&u));
        a.put(&legacy, Why::Over, "old-1", "{\"r\":1}", "{\"t\":1}").unwrap();
        let current = format!("browser/{}/", caller_segment(&u));
        assert!(matches!(a.put(&current, Why::Over, "new-1", "{\"r\":2}", "{\"t\":2}"), Ok(PutOutcome::Landed)), "the legacy record counted");
        assert!(matches!(a.put(&current, Why::Over, "new-2", "{\"r\":3}", "{\"t\":3}"), Err(PutError::Quota { .. })), "the share is one record");
        let _ = std::fs::remove_dir_all(&dir);
    }
```

(`Quota`'s field names and `with_quota` are S1b's — `archive.rs:108–131`; the implementer reads them.) The
Worker's premise cell (`receipt.rs:822–827`):

```rust
        // A BROWSER's save reads the address its Archive object was initialized
        // with, WHOLE — never the segment in the path, whose readable half the
        // 48-character cut has stripped of its `.test` here (security S1c: the
        // readable part, then `-` and a twelve-character digest).
        let long = format!("{}@t.test", "a".repeat(63));
        let truncated = caller_segment(&User::parse(&long).unwrap());
        assert_eq!((long.len(), truncated.len()), (70, 48 + 1 + 12), "premise: the address is longer than the readable cut");
        assert!(!truncated.contains(".test"), "premise: the segment has lost `.test` — {truncated}");
```

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --lib -- caller_segment legacy_segment
legacy_folder 2>&1 | grep -E '^error|test result|panicked' | head` — `legacy_caller_segment`, the constants not
found; stubbed (the legacy function a copy of today's, the constants): the first cell RED on the `starts_with`
(no digest) and on the first collision pair; the second RED at `assert_ne!`; the third RED (the legacy put
counted — one record).

- [ ] **Step 3: The code.** In `archive.rs`, replacing `caller_segment` (:196–232):

```rust
/// How much of the readable half a segment keeps (security S1c, finding D26):
/// 48, so that with `-` and the digest a segment is at most 61 characters —
/// under the 64 the old cap allowed and every path rule was measured at.
pub const SEGMENT_READABLE: usize = 48;

/// The digest's length in hex characters: twelve, forty-eight bits. The
/// collision left is two addresses whose readable halves AND forty-eight
/// digest bits agree — not a thing a person can arrange — while the folder
/// still reads as who sent it.
pub const SEGMENT_DIGEST: usize = 12;

/// The directory a browser's saves are filed under: the address made readable
/// and path-safe, then `-` and the leading twelve hex characters of a SHA-256
/// over the whole normalized address (security S1c, finding D26). The
/// readable half is [`legacy_caller_segment`]'s mapping cut at
/// [`SEGMENT_READABLE`]; the digest is what makes `alice+x@corp.test` and
/// `alice_x@corp.test` — one folder and one quota before this — two.
///
/// The readable half's arguments stand: an email renders at least six
/// characters of this alphabet, a plain name begins `[a-z0-9]`, neither holds
/// a `/`, so no segment is empty, `.` or `..`; [`paths`] and `DirArchive::put`
/// refuse a `.` or `..` component besides. The digest is over the KEY
/// (`User::as_str`, already lowercased and trimmed), so two spellings of one
/// address are one segment.
///
/// **Every folder written before this cycle sits under the legacy segment**
/// and is renamed once, in the deploy window, from `deploy/RUNBOOK.md`'s and
/// `worker/RUNBOOK.md`'s recipe with the server binary's `segment` subcommand.
/// No code reads a legacy prefix.
pub fn caller_segment(user: &crate::identity::User) -> String {
    use sha2::{Digest, Sha256};
    let mut readable = legacy_caller_segment(user);
    readable.truncate(SEGMENT_READABLE);
    let digest = Sha256::digest(user.as_str().as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("{readable}-{}", &hex[..SEGMENT_DIGEST])
}

/// The segment as it was before security S1c — kept for the one-time rename
/// (the `segment` subcommand prints it beside the current one) and for the
/// cell that pins the two apart. It COLLIDES: `a/b@c` and `a_b@c` render alike,
/// and two addresses longer than the cap share a prefix. Nothing files under it.
pub fn legacy_caller_segment(user: &crate::identity::User) -> String {
    let mut out = String::new();
    for c in user.as_str().chars() {
        match c {
            '@' => out.push_str("_at_"),
            c if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-') => out.push(c),
            // Every replacement is ASCII, so `truncate` below is always on a
            // character boundary however odd the address was.
            _ => out.push('_'),
        }
    }
    out.truncate(64);
    out
}
```

(`truncate(48)` on the readable half is on a character boundary for the same reason the legacy cap is.) The
old cell at :721 (`a_traversal_shaped_caller_writes_inside_the_root`) computes segments through
`caller_segment` and stays green; the old `assert_eq!(seg(&long).len(), 64, "capped")` line lived in the
rewritten cell.

In `main.rs`, at the top of `main` (find `fn main` — it builds the state and serves):

```rust
    // `yokohama-web segment <address>…` prints the archive segment of each
    // address — the current one and the legacy one — for the one-time rename
    // security S1c's deploy performs (`deploy/RUNBOOK.md`, `worker/RUNBOOK.md`).
    // Three columns, tab-separated, one address per line; a value the grammar
    // refuses is a line that says so, and the exit code is 1 if any did.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("segment") {
        let mut failed = false;
        for raw in &args[2..] {
            match yokohama_web::identity::User::parse(raw) {
                Ok(user) => println!("{raw}\t{}\t{}", yokohama_web::archive::legacy_caller_segment(&user), yokohama_web::archive::caller_segment(&user)),
                Err(e) => { failed = true; eprintln!("{raw}\t{e}"); }
            }
        }
        std::process::exit(if failed { 1 } else { 0 });
    }
```

(`main` is an `async fn` under `#[tokio::main]`; the block runs before the runtime is needed — the implementer
places it as the first statements and confirms `start_refusal`'s cells, which spawn the binary with no
arguments, are untouched.)

The recipe, in `deploy/RUNBOOK.md` (a new section after the deploy's §3) and `worker/RUNBOOK.md` (after the
deploy §2), the same text adapted to each host:

> **The archive's rename (security S1c, finding D26) — once, in the deploy window, after the new build is
> serving.** A browser's saves are filed under `browser/<segment>/`, and the segment changed: the readable half
> cut at 48, then `-` and a twelve-character digest. Every folder written before this release sits under the
> legacy segment and nothing reads it until it is moved.
> 1. List the addresses that have uploaded: the Access allow-list (every player is on it), or on the NAS `ls
>    /data/archive/browser/` and match each folder to the address whose legacy segment it is. In R2 the live
>    replay's own test identities sit under `test/browser/<legacy>/` as well (spec §2.6's root for a `.test`
>    address): list that prefix too, so no legacy key is left anywhere — the replay's first sends land at the new
>    key either way, but an orphan under the old one is a key nothing reads.
> 2. For each address: `docker exec <container> yokohama-web segment <address>` (NAS) or the same binary locally
>    (`cargo run -p yokohama-web -- segment <address>`) prints `address  legacy  current`.
> 3. NAS: `mv /data/archive/browser/<legacy> /data/archive/browser/<current>` for each folder present; a folder
>    whose legacy segment matches two addresses (the collision) is split by the `created_by` of each table
>    document inside it, by hand, and the record of that split goes in the deploy entry. Worker: for each key
>    under `browser/<legacy>/` in `yokohama-archive` (the dashboard's object listing, or the API's
>    `GET /accounts/{account}/r2/buckets/yokohama-archive/objects?prefix=browser/<legacy>/`): `wrangler r2 object
>    get yokohama-archive/<key> --file <tmp>`, `wrangler r2 object put yokohama-archive/<new key> --file <tmp>`,
>    verify the new key answers, `wrangler r2 object delete yokohama-archive/<key>`.
> 4. Verify: a fresh upload from that address lands under `browser/<current>/`, and `browser/` holds no folder
>    whose name has no `-<12 hex>` tail. Record the moves in the deploy entry.

`deploy/README.md:192` and `:314`: `browser/<caller>/` → `browser/<segment>/` with one clause: "the segment is
the address made readable, then `-` and a twelve-character digest (security S1c)". `receipt.rs:34`'s doc clause
"which the 64-character cap can strip" → "whose readable half the 48-character cut can strip".

**The Worker's two test files and the corpus** (the pre-flight's reading, F2 — the draft said neither moved):
`worker/test/archive.test.ts:1179` writes `browser/saver_at_t.test/rev7/over/wire.json` by hand and `:1207`
`test/browser/failer_at_t.test/rev7/over/wire.json`; each takes its path from the module's own 201 reply
(`path`) where the cell has one, else from a small test helper computing readable-plus-digest with `node:crypto`'s
SHA-256 over the address — never a literal that spells the legacy mapping. `worker/scripts/live-replay.test.mjs:317–335`
pins `resetRecipe`'s four keys and its first key as literal `red_at_wire.test` strings; `resetRecipe` itself
computes nothing (`replay/exemptions.mjs:220` reads `path` off the committed reply), so those cells read the
expected keys from the corpus (`archive_post.json`'s and `archive_post_abandoned.json`'s `path`) rather than
spelling them. The three corpus replies carry the segment in `path` and are regenerated ONCE (Step 4); the
Worker's `test/corpus.gen.ts` is regenerated from them by `worker/scripts/test-assets.mjs`. The Worker's own
code (`worker/src/`) computes no segment — the module's Rust `caller_segment` is the one source (grep clean).

- [ ] **Step 4: Run the cells, both crates and the lane's rows.** `cargo test -p yokohama-web`, `cargo test -p
yokohama-worker` (77 — the three segment cells compute dynamically; the premise rewritten), the wasm32 check and
both wasm builds (the layer compiles to wasm; `sha2` builds there — `table.rs` already proves it), `worker/`'s
`build:module:debug`, `test:scripts` and the pool (`npm test`, after the two TS files are re-pointed and
`test/corpus.gen.ts` regenerated), and **the corpus regenerated ONCE** — `UPDATE_WIRE_FIXTURES=1 cargo test -p
yokohama-web --test wire_fixtures`, then `wire_fixtures` green WITHOUT the flag; the diff READ: the three
`archive_post*.json` `path` values move from `browser/red_at_wire.test/…` to `browser/red_at_wire.test-<12 hex>/…`
and NOTHING else moves (the task report names the three files and quotes the three lines; any other moved byte is
a finding — the M6.1 lesson: read a regeneration's diff).

- [ ] **Step 5: The mutants.** **M20** the digest dropped (`caller_segment` = the readable half) — the first cell
red at the first collision pair; **M21** the digest over `raw` spelling instead of the key — the normalization
line red; **M22** `legacy_caller_segment` edited (say `@` → `_`) — the legacy cell red at its first line; **M23**
the quota counting the legacy prefix too — the share cell red; **M24** the subcommand printing only the current
segment — no cell; the reviewer runs `cargo run -p yokohama-web -- segment red@example.test` and reads three
columns (recorded).

- [ ] **Step 6: Commit.** `git add crates/yokohama-web/src/archive.rs crates/yokohama-web/src/main.rs
crates/yokohama-worker/src/receipt.rs deploy/RUNBOOK.md worker/RUNBOOK.md deploy/README.md
worker/test/archive.test.ts worker/scripts/live-replay.test.mjs web/src/wire/archive_post.json
web/src/wire/archive_post_identical.json web/src/wire/archive_post_abandoned.json worker/test/corpus.gen.ts` ·
`git commit -m
"web+worker: D26 — the archive's caller segment is readable plus a twelve-character digest, injective in practice;
the legacy mapping kept for the rename; the segment subcommand and the window's recipe in both runbooks (S1c
Task 5)"`.

---

### Task 6: M-3 — a `why` mismatch refusal at the requester's visibility — Opus (S1a's residual M-3; ruling 5; decisions D-7, D-8; Review Focus 5)

**Files:**
- Modify: `crates/yokohama-web/src/explain.rs` (`refused` :529; `refused_at`, `MISMATCH_WITHHELD` new; a unit
  cell beside :1142), `crates/yokohama-web/src/api.rs` (`why` :1706)
- Test: `explain.rs`'s and `lib.rs`'s `mod tests`

**Interfaces:**
- Consumes: `explain::visibility_of` (:306), `kind_of` (:657), `RefusalDto`; `yokohama_cli::why::{Refusal,
  Mismatch}`; `yokohama_core::{GameRecord, RecordedEvent, state_at, get_legal_moves}`; the route helpers.
- Produces: `explain::MISMATCH_WITHHELD`, `explain::Why::refused_at`.

- [ ] **Step 1: The failing cells.**

(a) In `explain.rs`'s `mod tests`, beside `a_withheld_account_is_the_same_bytes_whatever_the_search_found`
(:1142):

```rust
    /// **A mismatch refusal is served at the requester's visibility** (S1a's
    /// residual M-3): the `Mismatch` arm names the RECONSTRUCTED move, which
    /// the search chose over the bot's hidden hand — the B1 channel in one
    /// sentence — so while the account is withheld the refusal keeps its kind
    /// and carries `MISMATCH_WITHHELD`, naming no move; at a public visibility
    /// it is the CLI's own sentence, both moves named. Every other refusal is
    /// served as it is at either visibility, since none carries what a search
    /// found. The mutant that serves `refused` regardless fails the withheld
    /// line.
    #[test]
    fn a_mismatch_refusal_names_no_move_while_the_account_is_withheld() {
        use yokohama_cli::why::{Mismatch, Refusal};
        let (repl, _) = vs_bot(1);
        let now = repl.session.state().clone();
        let bot = yokohama_core::PlayerId(0);
        let human = yokohama_core::PlayerId(1);
        let m = Refusal::Mismatch(Box::new(Mismatch {
            index: 0,
            seat: bot,
            controller: "v4-step9-draft1@v4".to_string(),
            reconstructed: "PlaceAssistants { areas: [3, 7] }".to_string(),
            recorded: "Place assistants in areas 2 and 5".to_string(),
            recorded_version: Some(4),
            current_version: 4,
        }));
        // Mid-game, the bot's starting order hidden from the human (vs_bot(1)
        // pumps one bot move): withheld.
        assert_eq!(visibility_of(&now, bot, &[human]), Visibility::Withheld, "premise");
        let withheld = Why::refused_at(&m, &now, &[human]);
        let r = withheld.refusal.as_ref().expect("a refusal");
        assert_eq!(r.kind, "mismatch");
        assert_eq!(r.message, MISMATCH_WITHHELD.replace("{index}", "0").replace("{seat}", "0"));
        assert!(!r.message.contains("PlaceAssistants") && !r.message.contains("areas 2 and 5"), "{}", r.message);
        // The holder of the bot's seat reads everything: the CLI's sentence.
        let public = Why::refused_at(&m, &now, &[bot]);
        assert_eq!(public.refusal.as_ref().expect("a refusal").message, m.to_string());
        assert!(public.refusal.as_ref().unwrap().message.contains("PlaceAssistants"));
        // Every other refusal is the same bytes at either visibility.
        let other = Refusal::HumanSeat { index: 3, seat: human };
        assert_eq!(
            serde_json::to_string(&Why::refused_at(&other, &now, &[human])).unwrap(),
            serde_json::to_string(&Why::refused(&other)).unwrap()
        );
    }
```

(`vs_bot(pumped)` at :1028 builds a `Repl` with a bot at seat 0 and a human at seat 1 and pumps `pumped` bot
moves — the implementer reads it and adjusts the seat numbers if the fixture seats them the other way.)

(b) In `lib.rs`'s `mod tests`, beside `why_is_withheld_while_the_bots_starting_order_is_hidden_and_served_once_it_is_public`
(find by content):

```rust
    /// **Through the route, on a DRIFTED record** (Review Focus 5; decision
    /// D-7): a vs-bot game is played to the bot's first move, the bot's last
    /// recorded move is swapped for ANOTHER member of the legal list at that
    /// position, the record written back and the handle rebuilt from the
    /// store; `why` at that event, asked by the human while the bot's starting
    /// order is hidden, is a `mismatch` refusal naming no move. The pre-flight
    /// proved the construction reaches `Refusal::Mismatch`; the mutant that
    /// serves `refused` regardless names the reconstructed move in `Debug`
    /// form, which the cell reads for.
    #[tokio::test]
    async fn a_why_on_a_drifted_record_names_no_reconstructed_move_to_a_withheld_requester() {
        use yokohama_core::{get_legal_moves, state_at, GameRecord, RecordedEvent};
        let store = Arc::new(MemStore::default());
        let state = Arc::new(AppState::with_store(Arc::clone(&store) as Arc<dyn Store>, identity::IdentityMode::Open));
        let (status, table) = post_as(&state, "blue@wire.test", "/api/games", serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": ENDED_SEED.to_string(),
            "seats": [{ "bot": true }, { "user": "blue@wire.test" }], "name": "drift", "id": "drift",
        })).await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        let (status, kept) = post_as(&state, "blue@wire.test", "/api/games/drift/keep", serde_json::json!({ "seat": 1, "card": 1 })).await;
        assert_eq!(status, StatusCode::OK, "{kept}");
        // The bot at seat 0 starts: its first turn is in the record once the
        // table is playing and the human's frame is served.
        let (status, frame) = get_as(&state, "blue@wire.test", "/api/games/drift").await;
        assert_eq!(status, StatusCode::OK, "{frame}");
        let (json, version) = store.read(Doc::Record, "drift").expect("a store").expect("the record");
        let mut record: GameRecord = serde_json::from_str(&json).expect("the record parses");
        let rules = Rules::standard();
        // The bot's LAST move: the greatest k whose mover is seat 0.
        let k = (0..record.events.len()).rev().find(|&k| {
            matches!(record.events[k], RecordedEvent::Move(_)) && state_at(&record, k, &rules).expect("a prefix").current_player == yokohama_core::PlayerId(0)
        }).expect("the bot has moved");
        let before = state_at(&record, k, &rules).expect("the position");
        let RecordedEvent::Move(recorded) = record.events[k].clone() else { unreachable!() };
        let other = get_legal_moves(&before, &rules).into_iter().find(|m| *m != recorded).expect("another legal move");
        record.events[k] = RecordedEvent::Move(other.clone());
        record.events.truncate(k + 1);
        store.write_if_version(Doc::Record, "drift", &serde_json::to_string(&record).expect("json"), Some(version)).expect("written back");
        // The restart: a fresh state over the same store rebuilds the handle.
        let restarted = Arc::new(AppState::with_store(Arc::clone(&store) as Arc<dyn Store>, identity::IdentityMode::Open));
        let (status, why) = get_as(&restarted, "blue@wire.test", &format!("/api/games/drift/why?event={k}")).await;
        assert_eq!(status, StatusCode::OK, "{why}");
        assert_eq!(why["refusal"]["kind"], "mismatch", "{why}");
        let message = why["refusal"]["message"].as_str().expect("a sentence");
        assert_eq!(message, explain::MISMATCH_WITHHELD.replace("{index}", &k.to_string()).replace("{seat}", "0"));
        assert!(!message.contains(&format!("{other:?}")) && !message.contains(&format!("{recorded:?}")), "{message}");
        assert!(why["ok"].is_null() && why["withheld"].is_null(), "{why}");
    }
```

(`record.events.truncate(k + 1)`: the events after the bot's last move, if any, are the human's and may not be
legal after the swap; the record ends at the drifted move and the rebuilt session waits for the human. If the
rebuild's strict replay refuses the truncated record for a reason the pre-flight finds — a terminal digest, a
choice queue — the pre-flight's construction is what the cell transcribes; this block is the plan's expectation.)

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web -- names_no_move drifted_record 2>&1 |
grep -E '^error|test result|panicked' | head` — `refused_at`, `MISMATCH_WITHHELD` not found; stubbed
(`refused_at` = `refused`): (a) RED at the withheld message; (b) RED at the message (**the pre-flight's reading:
the CLI's sentence with both moves**).

- [ ] **Step 3: The code.** In `explain.rs`, beside `WITHHELD_REASON` (:332):

```rust
/// The mismatch refusal's sentence while the account is withheld (S1a's
/// residual M-3): the kind stays `mismatch`, the index and the seat are named,
/// and NO move is — the reconstructed one was chosen over the bot's hidden
/// hand, and the recorded one is withheld with it so the sentence is one
/// sentence. `{index}` and `{seat}` are filled by [`Why::refused_at`].
pub const MISMATCH_WITHHELD: &str = "`why` cannot account for event {index} honestly: replaying seat {seat}'s \
    decision did not reproduce the move the record says was played, and both moves are withheld while the bot's \
    starting order is hidden from you — they are served once that card is public, or to a seat holder once the \
    game is over. The game is untouched.";
```

and beside `refused` (:529):

```rust
    /// [`Self::refused`] at the requester's visibility: a `Mismatch` whose seat's
    /// account [`visibility_of`] withholds carries [`MISMATCH_WITHHELD`]; every
    /// other refusal, and a mismatch at a public visibility, is `refused`'s own.
    pub fn refused_at(r: &Refusal, now: &GameState, held: &[PlayerId]) -> Self {
        if let Refusal::Mismatch(m) = r {
            if visibility_of(now, m.seat, held) == Visibility::Withheld {
                return Self {
                    ok: None,
                    withheld: None,
                    refusal: Some(RefusalDto {
                        kind: kind_of(r),
                        message: MISMATCH_WITHHELD.replace("{index}", &m.index.to_string()).replace("{seat}", &m.seat.0.to_string()),
                    }),
                };
            }
        }
        Self::refused(r)
    }
```

In `api.rs`, `why` (:1706): `Err(refusal) => explain::Why::refused(&refusal)` → `Err(refusal) =>
explain::Why::refused_at(&refusal, &now, &held)`. (`now` and `held` are in scope: :1667 and :1672–1685.)

- [ ] **Step 4: Run the cells, the crate and the lane's rows.** `wire_fixtures` green WITHOUT the flag is the
claim: the corpus's `why_refusals.json` DOES carry a `mismatch` row, and it is a CONSTRUCTION —
`wire_fixtures.rs::every_refusal_payload` builds the twelve from `explain::refusal_samples()` through
`RefusalDto::of`, never through the route or `Why::refused` — so this task moves no fixture (decision D-8); a moved
byte is a finding, reported, never regenerated here.

- [ ] **Step 5: The mutants.** **M25** `refused_at` = `refused` — (a) and (b) red; **M26** the visibility read
inverted — (a) red at the public line; **M27** `kind` changed — (a) red at the kind line.

- [ ] **Step 6: Commit.** `git add crates/yokohama-web/src/explain.rs crates/yokohama-web/src/api.rs` (and the
corpus files, if any moved) · `git commit -m "web: M-3 — a why mismatch refusal is served at the requester's
visibility and names no move while the account is withheld (S1c Task 6)"`.

---

### Task 7: The documents the code changed, and the CLI's two comments — Opus (D10's rule; ruling 6; spec corrections 1–8)

**Files:**
- Modify: `docs/web-lane.md` (§2 counts, §4 standing rules, §13 the API map, §15 by design),
  `docs/plans/2026-10-08-security-remediation-spec.md` (§9.5 appended — never an edit above it),
  `crates/yokohama-cli/src/screen_model.rs` (:33–34, :2463–2464)
- Test: `cargo test -p yokohama-web --test lane_document` (6), `worker/scripts/lane-variables.test.mjs`,
  `cargo test -p yokohama-cli` (862, 2 ignored)

- [ ] **Step 1: The lane document.** §4's standing rules gain four sentences, each in the register of the rows
around it: every caller-supplied value an error sentence repeats goes through `echo::echo` (64 characters,
escaped) and every `ApiError` message is capped at 1,024 (S1c, D12/D28); every answer under `/api` is
`Cache-Control: no-store` from one layer, and a creator-chosen id carries no dot (B9); a table's per-id lock entry
lives only while a writer is inside `with_table_lock`, and an `{id}` the store would refuse is `dispatch`'s 404
(B8); a table stream ends after a refusal's `gone` frame, and a replay-route request with no identity takes no
permit (D1, S1b's residual). §13's rows: `GET …/stream` ("a 403 or 404 ends the stream" is now TRUE — the row
says so with the finding's id); `POST /api/games` (the dotted id's 400); `POST …/input` (the 256-byte bound and
the control rule, with the sentence shape); the `{id}` 404 noted at the top of the game rows. §15 gains three
rows: D7 accepted (the owner's sentence: the cap is the design; the identity floods that could fill it are the
door's rows B6, C1, C2 — S2); the segment (readable plus a digest; the rename at S1's deploy; `legacy_caller_segment`
kept for it and read by nothing); the served `why` chart's real-browser pin, recorded. §2's counts re-read at
Task 6's head (the lib count moved by this cycle's cells; the implementer re-counts and writes the figure and
the head it was read at). `lane_document`'s six cells and `lane-variables.test.mjs` hold the map and the
ledger against the route table: run both.

- [ ] **Step 2: The spec's §9.5**, appended under §9 after §9.4, dated 2026-10-10, titled "S1c's plan: the
plumbing's rulings and seven corrections": the owner's seven rulings verbatim from this plan's section, the
decisions D-1–D-10 verbatim, the spec corrections 1–8 verbatim. Nothing above §9.5 is edited.

- [ ] **Step 3: The CLI's two comments** (`screen_model.rs`; the fenced crate; doc only). :2463–2464:

```rust
/// The decision zone over the legal list the caller already holds. `legal` is
/// the list the CALLER holds — the CLI passes the engine's list verbatim, the
/// web layer a folded, capped projection of it (`client::folded_legal_capped`,
/// security S1b) — and this builder never generates or filters moves: one row
/// per move it is handed, in that order.
```

:33–34: "the engine's legal list" → "the legal list the caller holds (the engine's, or a caller's projection of
it)". `cargo test -p yokohama-cli` reads 862 and 2 ignored; `guide_sync` 18 (no guide text moved).

- [ ] **Step 4: Commit.** `git add docs/web-lane.md docs/plans/2026-10-08-security-remediation-spec.md
crates/yokohama-cli/src/screen_model.rs` · `git commit -m "docs: the lane document's S1c rows, the spec's §9.5,
screen_model's two doc sentences made true of the web's folded rows (S1c Task 7)"`.

---

### Task 8: The release bump to 0.6.1 — Opus (ruling 7; decision D-10; `docs/web-lane.md` §7)

**Files:**
- Modify: `Cargo.toml` (`[workspace.package] version`), `Cargo.lock` (the nine members' lines),
  `web/package.json`, `web/package-lock.json`, `worker/package.json`, `worker/package-lock.json`,
  `web/src/wire/health.json` (`server_version`), the envelope fixture `backup` regenerates (`app_version`),
  `deploy/Dockerfile.runtime:18` (the example tag)

- [ ] **Step 1: The bump, as §7 measured it.** (1) `Cargo.toml`'s version to `0.6.1`, then `cargo update
--workspace --offline` — the nine members' lock lines and nothing else (read the diff). (2) `npm version 0.6.1
--no-git-tag-version --ignore-scripts` in `web/` and in `worker/`. (3) `UPDATE_WIRE_FIXTURES=1 cargo test -p
yokohama-web --test wire_fixtures` and `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --lib backup`; READ the
diff: the version and nothing else — any other byte is a finding reported `DONE_WITH_CONCERNS`. (4)
`deploy/Dockerfile.runtime`'s example tag `0.6.1-s1`. (5) `cd web && npm run build:wasm` and `cd worker && npm
run build:module:debug` BEFORE the suites.

- [ ] **Step 2: Every suite, at this head.** The per-edit suites of `CLAUDE.md` and every row of
`docs/web-lane.md` §2, `npm run e2e` included (the client's version string moved) — recorded with counts in the
task report; `the_two_npm_packages_carry_the_workspace_version` green.

- [ ] **Step 3: Commit.** `git add Cargo.toml Cargo.lock web/package.json web/package-lock.json
worker/package.json worker/package-lock.json web/src/wire/health.json deploy/Dockerfile.runtime` plus every
file `git status` shows the two regenerations rewrote (the envelope fixture `--lib backup` writes and any
`archive-walk.json` records that carry the version — each named in the task report) · `git commit -m "release:
0.6.1 — security S1 (the leaks, the cost bounds, the plumbing); the wire's server_version and the envelope's
app_version regenerated, the version alone (S1c Task 8)"`.

---

### Task 9: The pre-flight, the reviews, the suites, the close, and the WRAP of S1 — ORCHESTRATOR (spec §5, §9.2; ruling 7)

- [ ] **Step 1: The pre-flight — the orchestrator's own "before" review.** On a `git archive` export of
  `security-s1b` at `680737e5` (`scratchpad/export-680737e5`, `node_modules` linked in): (a) **Execute every
  cell of Tasks 1–6** against scratch implementations of this plan's blocks (lifted by script, as S1b's
  `blocks.py`), and read every named mutant M1–M27 red at the cell it names and nowhere else; (b) **measure**:
  the longest gameplay line the client composes (the retreat picker's spelling in `web/src/` — grep `"r "` and
  the `A{n}:{k}` join — at 4P: the figure goes into `LINE_MAX`'s comment and Task 1's cell); today's body sizes
  at each echo row (Task 1 Step 2's figures); the lock map's count after the refused writes under the stub (Task
  2 Step 2); the wall of Task 3's header sweep (the stream rows) and of Task 4's slow-store cell; the health
  read under M17 (Task 4 Step 2); (c) **build the drifted record** of Task 6 on the export and confirm
  `Refusal::Mismatch` is what `why` answers — if the rebuild refuses the truncated record, find the construction
  that works and transcribe it into Task 6's cell; (d) **read the corpus**: `grep -l '"kind": *"mismatch"'
  web/src/wire/why*.json` and whether that row's requester is withheld (decision D-8); `grep -E '"id": *"[^"]*\.'
  web/src/wire/*.json` for a dotted id; `grep _at_ worker/test -r` for a hand-computed segment; (e) record every
  cell that cannot go red on its mutant, every name here the tree does not spell, and every sentence of the
  spec's rows or this plan the code contradicts; fix them in this plan at the source BEFORE Task 1 is
  dispatched; the scratch is discarded with the export; the findings file is
  `docs/temp/security-review-2026-10-08/s1c-before/findings.md`. **Stopped by the owner on 2026-10-10 after
  Task 1's cells ran green (the lib 463) and before any mutant ran**: the reviewers' mutant runs and the after
  review carry the validation; Tasks 2–6's cells were not executed (their splice scripts are written, unrun);
  the eleven findings are in the Review record below.

- [ ] **Step 2: The branch.** `git checkout -b security-s1c 680737e5` (= `security-s1b`'s head); the plan and
  the board committed on it first ("docs: S1c's plan and the board…"); S1c's commits stack on it; PR #51
  (`security-s1c` → `security-s1b`) opened at Step 7.

- [ ] **Step 3: The dispatches.** every task Opus/Opus (the owner, 2026-10-10), each reviewer rerunning the task's mutants on
  an export; the probe-and-name-concerns paragraph in every implementer brief;
  the SDD ledger under `docs/temp/security-review-2026-10-08/s1c-sdd/`; the heartbeat protocol for every wait
  past an hour. A `DONE_WITH_CONCERNS` correctness item is ruled on and fixed as a new commit before the review;
  a fix round quotes its hunks; a reported SHA is never amended.

- [ ] **Step 4: The per-edit suites at the code head (after Task 8)**, in this order, every row rc 0, each count
  against the board's "Gates at HEAD": `cargo test -p yokohama-core` (unmoved: 1,587 / 29 ignored) · `cargo test
  -p yokohama-cli` (862 / 2 ignored — doc comments only) · oracle · difftest · `eval_probe` · `solver` · bridge
  and its four node suites · `cargo clippy --workspace --all-targets -- -D warnings` · **the web lane's rows**
  (`docs/web-lane.md` §2), `wire_fixtures` green WITHOUT the flag, `npm run e2e` included (the version moved).
  **Then the fifteen-gate roster**, as ONE uninterrupted sequence with release binaries pre-built (CLAUDE.md's
  Commands): a merge to master follows this cycle, and the roster is what a merge owes; machine-tagged walls
  recorded beside S1b's (451 s at the core PR; ≤ 420 s crossed, OD-6 outstanding — a restatement is the owner's,
  not this cycle's). Record every count; a count that differs is a finding.

- [ ] **Step 5: The "after" review — the orchestrator's own**, whole-branch, on a `git archive` export of the
  branch head: the fences of §"Global Constraints" (`git diff 680737e5 -- crates/yokohama-core
  crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-wasm`
  empty; the CLI's diff confined to `screen_model.rs`'s two comments; the Worker's to `receipt.rs`'s cells;
  `SEMANTICS_REV` 7); the corpus unmoved but for Task 5's permitted regeneration (three `path` values) and Task 8's version fields,
  each diff read; every mutant M1–M27 re-run; the argument of each row re-derived from the code — that every
  `format!` in the seven files (`lib.rs`, `api.rs`, `tables.rs`, `games.rs`, `store.rs`, `identity.rs`,
  `grammar.rs`) that repeats a caller value goes through `echo` (a grep for `{id:?}`, `{raw:?}`, `{line:?}`,
  `{key:?}`, `{t:?}`, `{other:?}` outside `key_of` and the environment sentences, and `{}` of `req.path`, reads
  nothing in non-test code), that `tables.rs` holds no `lock_for(`, that
  `no_store` is a `layer` and not a `route_layer`, that `UntilLast` wraps the table stream and the house stream
  is untouched (it has no refusal arm), that the permit's condition is exactly `is_replay_route && resolved.is_ok()`,
  that `caller_segment` is `legacy` cut at 48 plus the digest and that NOTHING reads a legacy prefix, that
  `refused_at` consults `visibility_of` with the mismatch's own seat. One fix wave as new commits; a scoped
  re-review of the fix.

- [ ] **Step 6: The close's documents.** The ledger's eight rows (`docs/security-ledger.md`) rewritten: B8, B9,
  D1, D12, D26, D28 `fixed <commit>` with the catcher named (Task 1's cells for D12 and D28; Task 2's for B8;
  Task 3's for B9; Task 4's for D1; Task 5's for D26); D29 `fixed` (both halves now); **D7 `accepted (the owner,
  2026-10-10): the cap is the design — the 257th unlock is a 409 that names the state, never a silent eviction;
  the identity floods that could fill it are the door's rows (B6, C1, C2 — S2)`**; the ledger's header line for
  2026-10-1x; guidingpath's decision-log row for S1c's close and its current-state sentence (S1 COMPLETE on the
  stack; the wrap's state); the board's Track 4 (S1 wrapped or the wrap's remaining steps; S2 next; the stale
  residual pair corrected — spec correction 4); `CLAUDE.md` at most one standing sentence, if one is genuinely
  new — the candidate: "Every caller-supplied value an error sentence repeats goes through `echo::echo` (64
  characters, escaped) and every `ApiError` message is capped at construction; every answer under `/api` is
  `no-store`; a table's per-id lock entry lives only while a writer is inside `Games::with_table_lock`; the
  archive's caller segment is readable plus a digest, and the legacy mapping exists for the rename alone
  (security S1c, 2026-10-10)"; this plan's §"Review record" with the readings. The project memory updated the
  same way.

- [ ] **Step 7: The pushes and the PR.** `git push origin security-s1c`; PR #51 (`security-s1c` →
  `security-s1b`, stacked under PR #50) with the cycle's section (the rulings; the eight rows and their
  catchers; the corpus's movement, if any; the suites' readings, the e2e and the roster included; the mutants'
  readings; the seven spec corrections; "the merge at S1's wrap, below"); PR #49's description appended with S1's
  three-cycle summary. Nothing merges yet.

- [ ] **Step 8: The whole-stack review — a four-agent Sonnet workflow, verified by the orchestrator** (ruling 7;
  the owner's explicit opt-in to the Workflow tool for this step). Load `workflow-authoring` first. The script:
  four `agent()` calls in `parallel`, Sonnet, each over a `git archive` export of `security-s1c`'s head with
  `master` (`e9824ec9`) checked out beside it for the diff, each writing its report to
  `docs/temp/security-review-2026-10-08/s1-stack-review/<dimension>.md` and returning a typed findings list
  (`file`, `line`, `claim`, `reproduction`, `severity`); the four dimensions: (1) **hidden information and
  identity** — S1a's rows and S1c's echo and M-3: every payload, sentence and refusal of the game and explain
  routes read for a value that narrows a hidden card or names a caller to another; (2) **the cost bounds** —
  S1b's rows and S1c's lock map, line bound and stream end: every bound tried past its figure, every permit and
  lock traced from take to release, every stream and hand-off traced to its end; (3) **the archive and the
  hosts** — B12, D25, D26, the rename recipe, both runbooks and the deploy records: every path from a caller's
  bytes to a key, the quota's count, the recipe executed on a scratch archive; (4) **the documents' claims
  against the code** — the lane document, the ledger, the spec's §9.3–§9.5, CLAUDE.md's new sentences, every
  doc comment the three cycles wrote: each sentence tried as a mechanism (the false-sentence lens, the top yield
  of every close). Each brief forbids messaging other agents, forbids `rm -rf`/`rm -r`/bulk `rm -f`, Monitors and
  event waits, and requires the full report to its file with a three-line reply. **The orchestrator verifies
  every finding by trying its mechanism on its own export** (a finding a reproduction does not back is recorded
  as such, not fixed); confirmed findings are fixed as new commits on `security-s1c` with their catchers, and
  the fix commits get a scoped second run of the dimension that found them. The record: this plan's review
  record and the ledger.

- [ ] **Step 9: The merges and the tag** (the owner's rulings of 2026-10-09 and 2026-10-10). In order: PR #50
  (`security-s1b` → `security-s1`) merged; PR #51 retargeted to `security-s1` (`gh pr edit 51 --base
  security-s1`) and merged; PR #49 (`security-s1` → `master`) merged — each a merge commit, never a squash, so
  every task's commit and SHA the records cite survives. Then on master: the annotated tag `v0.6.1`, subject
  `Yokohama CLI v0.6.1 — security S1: the leaks, the cost bounds, the plumbing`, pushed; `release.yml` builds and
  publishes. The board and guidingpath's current state record the merge SHA.

- [ ] **Step 10: The deploy window — the orchestrator's alone, both hosts, one window.** (a) The NAS: the runtime
  image from the tagged master (`deploy/RUNBOOK.md` §3; the tag `0.6.1-s1`), deployed through Portainer; health
  reads `server_version: 0.6.1`. (b) The Worker: `cd web && npm run build`, `cd worker && npm run build`, the
  deploy through the guard (`worker/RUNBOOK.md` §2); health reads 0.6.1. (c) **The archive's rename** on both
  hosts from Task 5's recipe, recorded. (d) **The live acceptance** (spec §5.3; every probe that creates state
  removes it; the quota guard first for anything on the Worker): A2 — the review's 23,606-event body refused by
  COUNT on both hosts in milliseconds, a 513 KiB body's 413; S1a's three probes (a scratch table's `suggest`
  carries no invertible field; `why` withheld at a scratch vs-bot table; a seeded create beside an open seat a
  400 on both hosts); B9 — V2 re-read on the NAS: a throwaway table `probe-s1c` (dotless; `probe.js` now refused
  with `DOTTED_ID`'s sentence) whose frame reads `cf-cache-status: DYNAMIC` and `cache-control: no-store` on
  every API answer, and the same on the Worker; B8 — a burst of 1,000 refused writes at fabricated ids on the
  NAS with the container's memory read through Portainer before and after (flat); D1 — a stream held through
  `curl -N` by a second identity at an open table, the seat filled, `curl` exiting after the `gone` frame; D28 —
  a 300-byte line's 400 naming the length; D12 — a 16 KB path's 404 body under 2 KB; D26 — a fresh upload lands
  under the new segment; the live corpus replay as the production-door walk, TWICE (`cd worker && node
  scripts/live-replay.mjs`), HOLDING. (e) **The five configuration changes** the board lists (spec §2.2 B4, B6):
  the Access cookie's SameSite to Lax, the application session to a day, `wrangler.gate.jsonc` deployed so the
  dev door is off outside live tiers, `chmod 600 .dev.vars` with its unused `TURNSTILE_SECRET` line removed, the
  stopped `cloudflare-d` container removed with its token rotated — each a dated line in the runbook. (f) The
  ledger's live column filled for every S1 row; the deploy records name the tag and both `server_version`
  readings; the update reading (`web/scripts/pinned-browsers.mjs`, one run per host).

- [ ] **Step 11: The hand-off into S2.** The board's Track 4 rewritten: S1 COMPLETE and deployed (the merge SHA,
  the tag, the window's readings); the next session's first step — "S2's brainstorm from the ledger
  (`docs/security-ledger.md`): B4, B5, B6, B10 (code), C1, C2, C3, D3, D8, D13–D16, D18, D23, D24, B9's Worker
  `respond` half, B12's Worker object quota; the Access configuration changes R-SEC8(g) are the orchestrator's";
  the project memory updated the same way; `docs/temp/security-review-2026-10-08/s1c-sdd/` and
  `s1-stack-review/` left in place with every report. The next session begins at the board.

## Review record

The record of record for this cycle, written in three parts as each lands: the "before" review's findings and
measurements (Task 9 Step 1, by the orchestrator before Task 1 is dispatched), what each task's review found
that this plan did not know (Tasks 1–8, from the SDD ledger), and the "after" review with the four-agent stack
review's verified findings (Task 9 Steps 5 and 8). Until each part is written, its section is absent, not
empty — a reader who finds no "before" section knows the pre-flight has not run.

### The "before" review (Task 9 Step 1), 2026-10-10 — the orchestrator's own, on an export of `680737e5`, STOPPED at the owner's direction after Task 1

**Method.** A `git archive` export (`scratchpad/export-680737e5`; `web/node_modules`, `worker/node_modules` and
`bridge-captures` linked in); this plan's code blocks lifted by `docs/temp/security-review-2026-10-08/s1c-before/blocks.py`
and keyed by their FIRST content line (`edit.py`'s `BY_FIRST`/`blk`, so a correction that moves the plan's lines
moves no reference), spliced by `apply_t<N>.py` through exact anchors, the echo sites by `echo_sites.py` (a
`format!` parser, named-argument form); the findings and readings in `s1c-before/findings.md`, the logs under
`s1c-before/logs/`. **The owner stopped the pre-flight after Task 1's cells** (2026-10-10): the reviewers re-run
every task's mutants on an export per Step 3, and the after review (Step 5) re-runs M1–M27 and re-derives every
row, so the pre-flight's mutant pass would have been the same work a third time; with it the owner ruled every
task **Opus/Opus**. What ran: Task 1's blocks spliced and its seven cells GREEN, the whole web lib **463** (456 + 7);
Task 1's mutants were queued and killed unrun; Tasks 2–6's splice scripts are written and unrun. The after review
carries a list of what the pre-flight did not execute (the SDD ledger's "What the after review looks for").

**The findings** (F-numbers as `findings.md` carries them; each corrected in this plan at the source before Task 1's
brief was cut):

- **F2 — the corpus carries the caller segment (Task 5; the load-bearing finding).** `archive_post.json`,
  `archive_post_identical.json` and `archive_post_abandoned.json` each carry `"path": "browser/red_at_wire.test/…"`;
  Task 5 regenerates the corpus once (three `path` lines), where the draft forbade any regeneration but Task 6's.
  `worker/test/archive.test.ts:1179`/`:1207` and `worker/scripts/live-replay.test.mjs:317–335` spell the legacy
  segment as literals (the draft: "the pool's archive rows compute no segment by hand"); `resetRecipe` reads the
  path off the committed reply. Task 5's files, Step 3, Step 4, Step 6, D-8 and the Global Constraint rewritten.
- **F4 — the echo inventory was short by thirty sites and a file.** `store.rs`'s four "is not a usable game
  name" sites are the sentence a 16 KB game id's 500 carries today — Task 1's bounded-body cell went RED there;
  `tables.rs` holds nineteen `{id:?}` (four at `create` the BODY's id), three `{key:?}`, the seed's `{t:?}` and the
  scope's `{other:?}`; `games.rs` eight. The rule is mechanical and named-argument (`\"{x}\"` with
  `x = crate::echo::echo(x)`), so a string with positional `{}` keeps its order; the after review greps seven files.
- **F1 — D-8's mechanism.** `why_refusals.json` DOES carry a `mismatch` row, and it is a construction built from
  `refusal_samples()` through `RefusalDto::of`, never served — Task 6 moves nothing, for that reason.
- **F3** `ApiError` has three message-taking constructors (no `forbidden`/`conflict`); the cap cell pins them by
  two source counts. **F5** the web client composes no retreat line (option numbers, counts, five words, an area
  token); the 99-byte 4P retreat is the GRAMMAR's longest meaningful line — `LINE_MAX`'s comment, cell (b) and
  Review Focus 1 rewritten. **F6** the 16 KB game-id row is the store's 500 until Task 2 — the cell asserts a
  refusal per row, the bound being its claim. **F7** an unused `mut` the lint gate refuses. **F8** `Games::in_memory()`
  does not exist (`test_support::games()`), `two_people(id, seed)` takes a seed, no `TableDoc` builder exists.
  **F9** Review Focus 2 promised a table that "plays" and the draft's cell read and deleted — a route-level cell
  added (Task 3 cell (c)). **F10** the `Store` trait's fifth method has a default (no change). **F11** `is_gameplay_line`
  at `grammar.rs:74`, `SEED_NOT_SOLO` at `:764`, no `ApiError` cells in `lib.rs` to anchor on, `deploy/README.md`
  names the path at `:192` too.

**Readings.** Task 1: the seven cells green on the spliced export; the web lib 463; the longest grammar line 99
bytes (109 zero-padded); the `GET /api/games/<16 KB>` row 16 KB from the store's sentence. Nothing else measured.
