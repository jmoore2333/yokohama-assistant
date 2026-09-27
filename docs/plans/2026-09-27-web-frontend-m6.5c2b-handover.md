# M6.5c2 cycle B — the handover from cycle A (the entry for cycle B's planning session)

**What this is.** Cycle A ("the layer") of M6.5c2 is closed on branch `web-room-m6.5c2a`. This document is the ENTRY for
the session that plans cycle B ("the screens"): a fresh orchestrator writes cycle B's plan from it, the spec's §4 row B,
the drafts and the mocks named below, against this branch's REAL tree. Every fact here was read off the tree at the
reviewed head, never off cycle A's plan. Authority order is unchanged: `guidingpath.md`, then `CLAUDE.md`, then the spec
(`docs/plans/2026-09-26-web-frontend-m6.5c2-spec.md`, revision 2.6), then this note.

## 1. Where cycle A left the tree

- **Branch** `web-room-m6.5c2a`, stacked on `web-room-m6.5c` (code base `1e16fe27`); **reviewed head `2e8e4d91`**
  (the eight tasks' twelve commits, then the fix wave's seven); **PR #39** (https://github.com/jmoore2333/yokohama-assistant/pull/39) against `web-room-m6.5c` (spec §4 said
  "expected #39"; the number GitHub gave is the record). Cycle B's branch is `web-room-m6.5c2b`, on `-a`.
- **The suites at the reviewed head** (`macOS arm64`, macOS 27.0, this workstation; every count a reading, not a
  constant):
  - `cargo test -p yokohama-web` **338** = lib **320** + `browser_played_pin` **3** + `play_a_whole_game` **2** (24.5 s; the
    vs-bot arm no longer livelocks) + `wire_fixtures` **13**; `cargo test -p yokohama-wasm` **7**; `cargo test -p
    yokohama-worker` **64**; `cargo test -p yokohama-cli` **851 + 2 ignored** (unmoved — the CLI is fenced);
  - `cargo clippy --workspace --all-targets -- -D warnings` and `cargo clippy -p yokohama-web --no-default-features
    --features server --all-targets -- -D warnings` clean; the wasm32 `cargo check`; the public build's `identity::` **8**
    (CI asserts 8); `yokohama-wasm`'s release build and `yokohama-worker`'s `release-wasm` build; `cargo doc -p
    yokohama-worker --no-deps` 0 warnings;
  - `web/`: `npm run typecheck` clean, `npm test` **1067 in 56**, `npm run test:wasm` **95 in 7**, `npm run e2e`
    **61 passed (7.1m)** (the two-browser game 1,562 moves; no cap hit); `worker/`: `npm run typecheck` clean, `npm run test:scripts` **352 in 18**, the pool `npm test` **197 in 9**.
- **The fence** (`git diff --stat 1e16fe27 -- <the five fenced crates + the CLI>`) printed nothing at every commit;
  `SEMANTICS_REV` is 7; the workspace version is `0.5.0`.
- **Four e2e runs** (61 specs in 11 files each): Task 2 `7.4m` (two-browser game 2,328 moves), Task 6 `5.3m` (1,546; a
  first run failed only `plays_locally`'s quota cell, a documented sibling flake), Task 8 `6.3m` (1,939), the close `7.1m`.

## 2. The wire as it IS — what cycle B renders from, and nothing else

Every payload a component renders comes from `web/src/wire/` (the M2.5 rule). The files cycle A added or changed for B,
each with its `types.ts` / `contract.ts` names:

| file | carries | names |
|---|---|---|
| `identity_cases.json` | 41 rows `{display, key, raw}`: the key grammar's verdicts (`User::parse`, `User::typed_form`) — the name step's pin | `IdentityCase` (`contract.ts:417`), `wire.identityCases` (`:911`) |
| `seed_cases.json` | 26 rows `{ok, raw}`: `WireSeed`'s verdicts — the seed check's pin (U+0085 accepted, U+FEFF refused) | `wire.seedCases: readonly {raw, ok}[]` (`:915`) |
| `production_bot.json` | `{"identity": "v4-step9-draft1", "label": "V4"}` — IMPORTED by the screens directly (a Vite JSON import), never through `contract.ts` | `ProductionBot` (`types.ts:372`), `wire.productionBot`, `KEYS.productionBot` |
| `table_bot.json` | the corpus's one `TableView` whose `bot` is not null (an undealt vs-bot table, `runs_here: true`) | `TableBot` (`types.ts:364`), `TableView.bot` (`:599`), `wire.tableBot`, `KEYS.tableBot` |
| `foreign_bot_409.json` | `{"error": "This game was played against V2. This server now runs V4 and can't continue it.", "foreign_bot": {"played": "V2", "runs": "V4"}}` | `ForeignBot` (`types.ts:649`), `ForeignBotErrorBody` (`:658`), `wire.foreignBot409`, `KEYS.foreignBotError`, `KEYS.foreignBot` |
| `table_foreign.json` | the lobby row that does not run here (`"bot": {"label": "V2", "runs_here": false}`) | `wire.tableForeign` |
| `why_foreign.json` | `why`'s THIRTEENTH refusal, the layer's own: `{"kind": "foreign_bot", "message": "This move was made by V2, which this server doesn't run."}` | `wire.whyForeign` |
| every `ClientState` file | `bots: SeatBot[]` — the record's bot seats, labelled (`watching.json` has one) | `SeatBot` (`types.ts:354`), `ClientState.bots` (`:350`), `KEYS.seatBot` |
| `me.json`, `me_signed_out.json` | `"mode": "open"` | `Me.mode: "open" \| "cf-access" \| "worker"` (`types.ts:514`) |
| `health.json` | `"identity": "open"` | — |

Every game route answers the foreign-bot **409** for a game NOT over whose recorded bot this build does not run;
`…/table` still reads (`runs_here: false`) and Abandon still deletes; a FINISHED foreign game opens with the four masks.
The third columns (`worker/test/corpus.test.ts` `THIRD_COLUMN`, `web/src/local/corpus.node.test.ts` `NOT_SERVED`) name
every file above that no script replays — a task that adds a corpus file INSERTS its name, never pastes a literal.

## 3. What cycle B builds, and its raw material

**Spec §4 row B, restated:** §2.2 and §2.3 whole — the shared pieces, the door and the project link, the identity states
A–E and the name step (§2.1 "The client", pinned to `identity_cases.json`), the lobby, the forms and the seed check
(pinned to `seed_cases.json`), waiting and joining, "Spectate" in the client, the single-player links, "Will not open
here", the game screen's foreign refusal; §2.8's client items; every rebuilt screen's jsdom, browser and 390 px cells.
It ends as A did: review, fix wave, re-review, push and PR, a handover into C; **no deploy**.

**Raw material** (all under `.superpowers/sdd/2026-09-26-web-frontend-m6.5c2-plan/` unless committed):
- **The drafts** D4 (`drafts/D4-tasks-09-12.md`: the shared pieces, the door and the project link, the identity states
  and the name step, the lobby) and D5 (`drafts/D5-tasks-13-16.md`: the forms and the seed check, waiting and joining,
  "Spectate" and the single-player items, the client's small items) — run in `web/`-only exports over stand-ins, their
  e2e cells type-checked but never run. D4's component files and D5's seed crate are under `drafter-artifacts/`.
- **The visual work:** the eleven approved mocks in `docs/plans/2026-09-26-web-frontend-m6.5c2-mocks/` (committed);
  their HTML sources and fonts in `mocks-html/`; the brainstorm's nineteen `check-*.png` review shots in
  `.superpowers/brainstorm/`; **D4's eighteen rendered screens in `d4-look/`** — the door, the lobby, the name step
  and states A–D at 1440 and 390 as D4's code drew them, beside mocks 01, 02, 03, 05 and 11.
- **Known corrections to carry:** the retired review's P-7 (`web/e2e/the_table.spec.ts:430-433` drives the OLD table
  form; the forms task rewrites it); D4's amendment 1 (the identity rows `nameRule.test.ts` requires are ALL in
  `identity_cases.json` now — 41 rows); D4's amendments 2–4 and D5's names as the retired plan's "Additions reconciled at
  assembly" listed them; the assembly's spec-gap answers (spec §10.5's tail): "How to play" opens the help without its
  card-art screenshots; the creator's lobby rows keep Abandon beside their one action; row actions are buttons and the
  name is the one link; state C's discriminator tests state A first; state D is the door's alone and the lobby shows
  "loading…" rows; the iPhone keeps its install instruction; the joiner's password sits above the seats and the one join
  button below; the open table's chat stays in the main column; other people's seat cards read "seated"; test data never
  uses the owner's surname (every test person is `Ben Castle` in some spelling).

## 4. The rules cycle B's plan carries

- Every task that changes what the client shows loads `frontend-design` and builds against the mocks in the app's own
  stylesheet and tokens: no new colour roles, no card art on the setup screens, the door asking the server NOTHING.
- Every payload a component renders comes from `web/src/wire/`; nothing hand-authored (M2.5); the fake Worker answers
  from the same corpus.
- `npm run e2e` is owed by every task that changes what a browser does or shows — SEQUENCED, one Playwright run at a
  time, in the background with its output to a file the implementer POLLS (never a Monitor, never an event wait: an
  implementer this cycle stalled five hours on one); a 390 px cell per rebuilt screen; a headed look, because no jsdom
  cell sees a stylesheet.
- Counts as DELTAS over the implementer's own HEAD; the gate reads 2 passed in ~25 s and a red gate is now a FINDING.
- One Opus implementer and one Opus reviewer per task, Fable for the whole branch; a reviewer's mutants run on an
  EXPORT with a shared `CARGO_TARGET_DIR` under the session's scratchpad (the disk ran to 401 MiB free in cycle A before
  that rule); every agent writes its full report to a file and replies with the path and three lines.
- A reviewer that restores a mutated file by `mv` from an older copy leaves cargo's fingerprint on the MUTANT (the
  mtime goes backwards); restore with `cp` and `touch` — a false reading cost one run in cycle A.
- **A plan sized to be read whole**: the project's executed range is 1,000–3,300 lines; cycle B's code is React whose
  contract is the mock, so it needs far less inline code than A did — intent, files, interfaces, the cells and the traps,
  with code only where it defines a contract.

## 5. What cycle A does NOT claim

- Task 8's cap (`VS_BOT_CAP = 5_000`) rests on ONE run of 50 (min 136 / median 386 / max 1212 inputs); its reproduction
  witness is "a pinned run equals the recorded UNPINNED run's line" (the whole-branch review corrected the plan's "two
  pinned runs identical", which a missing restart also passes); no post-fix livelock was reproduced, so "fails in
  minutes" is an estimate (a stuck-rule mutant reached the cap in 8 s).
- The NAS still runs `YOKOHAMA_WEB_IDENTITY=claimed` and is NOT redeployed before cycle C's close; every build from
  `b27acf11` on refuses that value by name.
- The Worker module and the browser host still RESTATE the layer's items (`error_body`, `MAX_WAIT_SECONDS`, the house
  frame, the clock, the platform scan); cycle C switches them to the exports.
- The live tiers were not run; the live replay's reset recipe is OWED before it next runs (Task 5 moved
  `archive-walk.json`'s records).
- `cancelPicker` (the browser walk's new Cancel branch) has run in no e2e walk yet.
- **The refinish rule's stamp is unreachable on the Worker** until cycle C hands the held text back (`ReceiptArchive`'s
  `Conflict` carries `held: None`): on the Worker today a same-game conflict still leaves the save owed, as before.
- An owed game-over upload undone and then foreign is bounded by Abandon or expiry, as the spec says — but on the
  Worker the retry's 409 body makes `settleUpload`'s `parseStamp` THROW inside the alarm; what the platform does past
  its alarm-retry ceiling is not established. Cycle C's shell work should read a `foreign_bot` body as final-for-now.
- The transports on the new 409: the SSE stream writes a comment and stays open; the Worker's socket sends nothing on a
  status other than 403/404. What the client shows is cycle B's.
- A finished game against `v4-step9-draft1` at another VERSION opens with the CLI's own `warning: seat 1 records
  controller … at version …` line on the wire (a drift line, not a conversion line — the mask leaves it, spec §7), and
  its refusal sentence while unfinished reads "played against V4 … now runs V4" (R-A8).
- **`ClientState.warnings` has a positive witness at the FILTER and not at the FIELD:** the fix wave's cell pins
  `without_foreign_conversion`; a `warnings: Vec::new()` forced in `ClientState::of` still ships green (the whole-branch
  re-review's one open residue, parked by R-A19). The measured closer is `a_rules_hash_drift_reaches_the_wire_as_a_warning`
  in `browser_played_pin.rs` (~30 lines, pin 3 → 4), specified in the ledger's `final-rereview.md`.
- A spectator may read and post chat at an unfinished foreign game (R-A7); the seat holder's chat is the 409.

## 6. Every ruling taken in cycle A's ledger (2026-09-27, the orchestrator; one line each)

- **Pre-flight (3):** `foreign_pair` copied three times across crate targets (the receipt cells' R-06) — accepted, each
  copy held by its own cell; the refusal body built in three places at Task 4 and unified by Task 7's `error_value` —
  accepted as transient by spec; the hosts' restatements kept through Task 7 — spec §4 row C assigns the switch to C.
- **R-A1:** the work ran in this checkout on the branch, not a worktree. **R-A2:** Opus for every implementer and
  reviewer, Fable for the whole branch. **R-A3:** the ledger's first line is the SDD identity line.
- **Task 1's ⚠️ (R2.2-m4):** the archive's `test/` root requiring an email is spec §2.1 "The Worker's two doors", cycle
  C's by §4 row C; only the configured archive caller can reach it — carried, not fixed.
- **R-A4 (Task 3):** the Worker's drift cell derives the pool fixture `owed-game-over.table.json` as the pair's table
  plus the RECORD's first bot descriptor in its mirror (`bots::recorded`), and its UPDATE path writes that derivation —
  the fixture stays generator-reproducible and O-1(a) holds; editing the browser-written pair instead was refused (it
  cascades into the backup envelope and `backup-report.json`). **R-A5:** three doc sentences made true; `TableBot`'s
  TS doc left for Task 4 to make true.
- **R-A6 (process):** reviewers' mutants build in exports with ONE shared `CARGO_TARGET_DIR`; earlier sessions' cargo
  caches under `/private/tmp` were removed (the disk had 401 MiB free).
- **R-A7 (Task 4):** "not finished, every game route refuses it" stands — the spec's words are "refuses to OPEN"; a
  watcher's chat at an unfinished foreign game opens nothing (a deferred minor with a cycle B note). **R-A8:** the
  `bga-human` edge and the same-name re-point sentence are accepted as designed and carried (§7).
- **R-A9 (Task 6):** two Worker code comments made true in a second commit; `RUNBOOK.md:2419` and `worker/README.md:168`
  to cycle C; the wasm scan cell's doc made true; `with_log`'s server-only doc link left; `log.node.test.ts` verified
  type-checked by the node project (the IDE's flag was its fallback project).
- **R-A10 (Task 7):** the e2e not owed (nothing a browser does or shows moved in play). **R-A11:** two Worker code
  comments made true in a second commit; `RUNBOOK.md:2115` to cycle C; clippy's `expect_err` rewrite accepted.
- **R-A12 (Task 8):** a livelock on the red run may stand as the red after one re-run. **R-A13:** the cap's comment
  records the base `605c3ddd` and "Task 8's tree", not its own SHA. **R-A14:** `Stuck`'s doc made true (it fires on the
  FOURTH identical prompt; M6.1's behaviour kept for the two-people walk's byte-identity); `cancelPicker`'s coverage a
  deferred observation; the two-people arm's doc rewrite accepted.
- **R-A15 (the close):** the fix wave took the eight document sentences the review found false (README's five, CLAUDE.md's
  three) because CLAUDE.md is current-state by its own rule and cycle B reads it first; RUNBOOK, cloudflared and the
  decision log stay cycle C's. **R-A16:** the wave's scope — seven findings, one commit each, the e2e not owed.
  **R-A17:** the nine items the review declined to judge, each ruled (§7 carries the ones with a consequence).
  **R-A18:** the plan's workspace is kept as the cycle's evidence store, not deleted. **R-A19:** Important 1's
  field-level residue PARKED — real, not load-bearing, one measured commit away (§5, §7).

## 7. Cycle C's standing items this cycle created or found

- **First, one measured commit:** `a_rules_hash_drift_reaches_the_wire_as_a_warning` in
  `crates/yokohama-web/tests/browser_played_pin.rs` — the field-level witness for `ClientState.warnings` (R-A19; the
  shape is in the ledger's `final-rereview.md`). Cycle B may take it as its opening task if the owner prefers.
- The live replay's reset recipe, once, after the version bump (Task 5's regeneration and the bump owe it for the same
  keys).
- The whole-game gate back into CI on Task 8's reading, with the pinned-equals-unpinned reproduction witness.
- The import switch: the module and the browser host take `api::error_body`, `api::MAX_WAIT_SECONDS`/`check_wait`,
  `api::NOT_AN_OBJECT`, `signal::house_frame`, `games::HostClock`, `platform::{CALLS, calls_in}`; `calls_in`'s
  stop-at-the-first-`#[cfg(test)]` rule adopted on purpose; `identity_of` threads the configured door through
  `identity::admit`; the Worker's receipt hands the held text back so `PutError::Conflict.held` is `Some` there.
- Spec R2.2-m4: the archive's `test/` root requires an EMAIL — `crates/yokohama-worker/src/receipt.rs` still picks
  `test/` by `ends_with(".test")`, and `a.test` is a legal plain name since Task 1.
- The house copy now carries `turn.bot.params.weights` = 6957643967275421640 (above 2^53) while `worker/src/house.ts:190`,
  `:441` and `worker/src/table.ts:2060` justify PARSING it by "carries no seed" — harmless today, the premise gone.
- Documents left for C by ruling: `deploy/RUNBOOK.md:2419` and `worker/README.md:168` (the layer's lines are no longer
  mute on the Worker), `deploy/RUNBOOK.md:2115` (the create's old `a game must be created from a JSON object`
  sentence), `deploy/cloudflared-config.yml:58` (`claimed`, the deployed NAS), RUNBOOK's and CLAUDE.md's `claimed` /
  `dev-identity`.
- `events`' refusal ORDER (`check_wait` before the admission) has no two-defect probe; it matters when C switches the
  Worker's `events`.
- Two `bots.rs` edges: a `bga-human` seat reads as a foreign bot; a same-name re-point (version or knobs alone) refuses
  as "played against V4. This server now runs V4" — a label that disambiguates by version is the AI lane's design item
  for its next re-point.
- The Worker's `settleUpload` reads a `foreign_bot` 409 body as final-for-now (today `parseStamp` throws inside the
  alarm and the platform's retry ceiling decides).
- "Only `finalize` stamps `run_id`" has no pin (a stamp on every persist survives every suite): load a `web/browser`
  record into a `Server` `Games`, move, persist, read the id unchanged. The print scan misses `std::io::stderr()` writes.
- `RUNBOOK.md:1427` still spells the owner's address (R-6.5c2.16: the address goes into no document) — the document
  pass removes it.
- **For the owner's eye, a spec question the review set aside:** under `open`, an EMAIL-shaped name is admitted like
  any key (spec §2.1 rule 2), so a self-host player can claim what looks like an Access identity; the trust model
  already says anyone may use any name. Nothing in cycle A changes it; cycle B's name step could refuse the `@` shape
  under `open` if the owner rules so.
