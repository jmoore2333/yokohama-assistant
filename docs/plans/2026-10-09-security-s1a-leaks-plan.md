# Security S1a — the leaks: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. Every code task subagent-driven, the implementer's tier in each task's title — **Opus** for
> Tasks 1–5, **Sonnet** for Task 6 — every task reviewer **Opus** (the owner, 2026-10-09: the core PR's Fable
> reviews were the protected core files' exception; for this sequence Sonnet/Opus and Opus/Opus are the
> convention, Fable only where a task is unusually undefined or long-running — none here is, Task 1 being the
> broadest but fully written), and a **Mythos** orchestrator that conducts the plan review at the top and the
> whole-branch review at the end ITSELF, on exports; the suites, the documents' close and the SDD ledger are the
> orchestrator's too (Task 7). Steps use checkbox (`- [ ]`) syntax. **Read the spec first**
> (`docs/plans/2026-10-08-security-remediation-spec.md`: §1 the rulings R-SEC3, R-SEC7, R-SEC8(a)(b), §2.1 the
> rows A1, B1, B2, B3, B7, D5, D10, D11, §6 the fences, §9.2 the stacked-branch ruling), then this plan's
> §"Names shared across tasks" — an implementer sees only its own task, and that section is how it learns what its
> neighbours produce. Every `file:line` here was read at master `e9824ec9` (the code of the layer, the CLI, the
> client and the Worker is byte-identical to the review's base `d0816172`; the diff is core and documents);
> **find by content once an earlier task has moved a file.** The implementer PROBES the brief's code before its
> own review and reports `DONE_WITH_CONCERNS` when a sentence here proves false.

**Goal:** Close the hidden-information leaks the 2026-10-08 review found on the web lane's explain, undo, create
and seat surfaces — the setup seed inverted from `suggest`'s stream key (A1), the bot's hidden starting order
served through `why` (B1), an own-turn undo that un-draws a hidden deck card at a shared table (B2), a
creator-chosen seed at a table that seats another person (B3), a nameless joiner's address served to the creator
(B7), the setup seed in a lockdown's `input` reply (D5), a not-over record served after a failed final save (D11)
and the lane document's understatement of who reads `…/table` (D10) — with `SEMANTICS_REV` at 7, the five fenced
crates byte-identical, the CLI touched at `suggest::stream_seed` and nowhere the bot decides, and the wire corpus
regenerated ONCE, in Task 1, its diff read.

**Architecture:** Four surfaces, each closed where the fact lives. (1) The stream key becomes a one-way digest at
the source (`crates/yokohama-cli/src/suggest.rs`, SHA-256 over the same components, determinism and keying
kept) and leaves the wire; `why` is served at the REQUESTER's visibility by the engine's own view: while
`observe(now, requester)` still shows a `HiddenStartingOrder` slot in the mover's hand and the game is not over,
the account — the feature table, the continuation, the ranking, the counters, the search facts — is WITHHELD and
a typed `withheld` half carries the move and the claim; once the card is public, or to a SEAT HOLDER once the
game is over (the record route's own condition — it refuses a spectator), the full account is served, its continuation's fulfil lines masked as the ranking's are, and `decision_seed` is gone.
(2) `undo_allowed` compares the deck lengths of the state before the trailing event with the current state and
refuses, at a table of more than one person, a unit that drew. (3) `Games` carries a `SeedPolicy` — `SoloOnly`
by default, the production rule: a creator's seed is honoured only when every human seat is the creator's and
none is open; `Diagnostic` is what the fixtures opt into by name, the engine's own `--master` convention — and
the Worker module reads the same switch from its configuration, `on` only in the pool's test files, refused by
the deploy guard. (4) `SeatDoc::Human` records `bound_at_create`, and `SeatView` fills `invited` for those seats
alone. Beside them: `input` replaces a lockdown's dump with the lockdown's reason, and `record` reads the store
under the game lock and refuses a stored record the handle has outrun.

**Tech Stack:** Rust 1.97.1 (`rust-toolchain.toml`): `crates/yokohama-web` (the layer and its route cells,
`tests/wire_fixtures.rs`), `crates/yokohama-cli` (`suggest.rs` + `sha2 = "0.10"`), `crates/yokohama-worker`
(`config.rs`, the seed switch); TypeScript: `web/src` (`Explain.tsx`, `NewGame.tsx`, `types.ts`,
`wire/contract.ts`, their vitest cells), `worker/src/config.ts`, `worker/scripts/deploy-guard.mjs` and its node
cells, the pool's `wrangler*.test.jsonc`; the committed wire corpus `web/src/wire/`.

**Spec:** `docs/plans/2026-10-08-security-remediation-spec.md` — §2.1's rows A1, B1, B2, B3, B7, D5, D10, D11;
R-SEC3, R-SEC7, R-SEC8(a), R-SEC8(b); §6 the fences; §9.2 the stacked branch. The corrections this plan makes to
it are §"Spec corrections (to be recorded as §9.3)" below — recorded forward in the spec's §9 by Task 6.

## The cycle split, stated once (the M6.5c2 lesson; the owner's 2026-10-09 corpus note)

S1 (spec §2.1) spans three subsystems and the board expected more than one cycle. The split is by FINDING FAMILY,
each cycle planned against the stack's head (`security-s1`, cut from master `e9824ec9`, the code of `9ab8c430`):

| cycle | rows | what moves |
|---|---|---|
| **S1a — the leaks (this plan)** | A1, B1, B2, B3, B7, D5, D10, D11 | the CLI's `suggest::stream_seed`; the layer's `explain.rs`, `api.rs` (`why`, `undo_allowed`, `input`, `record`), `tables.rs`, `table.rs`, `games.rs`; the client's explain and new-game surfaces; the Worker's seed switch; the corpus, once |
| **S1b — the cost bounds** | A2, A3, B12, D6, D25, D27, D29 (`archive_post`'s doc) | `backup.rs`, `api.rs` (archive, `/at`), `lib.rs` (the executor hand-off and the semaphore), `archive.rs` (the quota), `client.rs` (the frame's option list), the client's frame types; the corpus, once (A3's frames); the `CLAUDE.md` parenthetical "S1 wires the web layer to it" dropped (the core PR's note) |
| **S1c — the plumbing** | B8, B9, D1, D7, D12, D26, D28, D29 (`is_token`'s doc) | `games.rs` (the lock map), `lib.rs` (`Cache-Control`, the stream's end), `store.rs`/`tables.rs` (the hosted id rule), `grammar.rs`, `archive.rs` (`caller_segment`), the error echoes |

**The corpus regenerates once per cycle that changes the wire's shape**, read each time — the owner's note of
2026-10-09 ("if the plan ends up going through multiple cycles, it will be unavoidable to regenerate the corpus
… better than leaving everything red until that regeneration"), recorded as a forward correction to spec §6's
"once, in S1". Within THIS cycle it regenerates TWICE, each regeneration its own task's and read on its own: Task 1's
(A1's `suggest.json`, B1's `why.json` and the new `why_withheld.json`) and Task 3's (B7's `invited`, which flips
to `null` on the walk's nameless joiner in every table-bearing fixture); Tasks 2, 4 and 5 add no fixture and
change no committed byte, and Task 7's check holds the corpus to exactly those two commits.

**A2's figures travel with this plan** (the owner's ask; the measurement is §"The S1 figures" at the end) so
S1b's plan transcribes them; the CLAUDE.md parenthetical and the "`replay_strict` itself is unedited" snapshot
are S1b's to act on, when A2's wiring lands.

## Spec corrections (to be recorded as §9.3 by Task 6, dated 2026-10-09)

1. **§6, the corpus.** "The wire corpus regenerates once, in S1" reads "once per cycle of S1 that changes the
   wire's shape, read each time" (the owner, 2026-10-09).
2. **§6 and §3.2, the CLI fence, against §2.1 A3.** "The CLI is touched only at `suggest::stream_seed`" and
   A3's file list (`screen_model.rs`, `session.rs`) contradict each other. The fence's substance is "nowhere the
   bot decides" (§6's own last clause): S1a touches the CLI at `suggest::stream_seed` and its manifest alone;
   S1b's plan decides A3's CLI edits under that reading, preferring the layer where a layer-side change is
   equal.
3. **§2.1 B1, the served shape** (the owner confirmed it 2026-10-09, after asking whether the hidden card
   matters past the opening round: it does — the bot's kept starting order stays in its hand, masked from every
   other seat by `observe`, until the bot FULFILS it, mid-game in every recorded game, and the review named the
   card from the FIRST bot move's feature values alone). "Compute the served explanation at the requester's
   visibility" is met by WITHHOLDING the account while the mover's starting order is hidden from the requester
   (and, for a WATCHER, at game over too, since the record route refuses a spectator — the review of Task 1,
   I-1), not by a masked recompute: the explanation is read at the search's hand-off state, which the layer does not hold, a
   recompute at the decision state would be a second evaluation derivation (forbidden), and every served value —
   the terms, `total`, the ranking's values, `root_candidates`, `forced` — is a function of the hidden card.
   The continuation is withheld too, and the reason is on record: a plan computed with the card known is
   re-searchable per candidate card, so a served plan with masked fulfil lines still narrows (the review's own
   caveat on `rank_all`'s doc, generalized). The fulfil-line masking the spec asks for applies to the served
   (public) continuation, for consistency with the ranking.
4. **§2.1 B3 and R-SEC8(b), the fixtures** (the owner chose the named `Diagnostic` policy, 2026-10-09: "a good
   idea especially for future development and through these other phases"). The corpus walk (`wire_fixtures.rs`'s `new_game_body`: a seeded
   create with an OPEN seat), 79 lib cells (`two_people(id, seed)`), the Worker crate's `CREATE` constant and
   the Worker pool's replay of the walk all open seeded multi-person tables, and the corpus's determinism
   REQUIRES it. The rule is kept as ruled and the fixtures opt into a named `SeedPolicy::Diagnostic` — on the
   NAS binary never, on the Worker through `YOKOHAMA_DIAGNOSTIC_SEEDS`, `on` only in the pool's two test files
   and refused by the deploy guard in any deployable file. The alternative the review offered — honour the seed
   and carry `seeded: true` on `TableView` with a sentence on the join screen — is recorded as the fallback the
   owner may prefer; it needs no switch and leaves the creator's advantage disclosed rather than closed.
5. **§2.1 D5, the catcher.** The lockdown's dump is produced only at the moment of a fault inside `Repl::step`;
   the one fault a cell can reach through the API is a bot whose own deal is unrecorded (a record with an
   `Unknown` choice at the bot's seat), which Task 5's cell installs; the filter is unit-pinned beside it.
6. **§2.1 D11, the fix.** "The record route's 'over' read under the lock" is met and joined by a comparison:
   the stored record's event count against the handle's AND its terminal digest against the digest of the
   handle's LIVE state — what a successful final save stamps; the record's own `terminal_digest` field does not
   track the live state, `None` for a game dealt in process and the loaded value for a rebuilt one (Task 5's
   fix round, F1) — (the count alone is defeated by undos under failing store writes and a different line replayed to the same
   length — measured in Task 5, six undos and the same 356 events, the running record served with its seed), so a
   finished game whose last save failed answers a 409 naming the failed save rather than the running record with
   its seed.
7. **§5.3, the live acceptance, and §9.2's stacked branch.** Each S1 cycle ends with a pushed PR whose merge
   waits for S1's wrap (the owner, 2026-10-09: "the plan will need to end with a PR, it can leave the merge
   undone"); a test deploy of the branch's build to either host and the review's probes are allowed in the
   maintenance window the owner opened for 2026-10-09/10 ("pushing the PR and running any checks or deploys are
   fine for testing"). So S1a's probes — a scratch table's `suggest` carries no invertible field; `why` reads
   `withheld` at a scratch vs-bot table and the full account at a finished one; a seeded create beside an open
   seat is a 400 on both hosts — run at S1a's close from a test deploy when the window still holds, else at the
   stack's deploy after S3. Every probe that creates state removes it.
8. **§6 and §3.2, the CLI fence, and the guide pair** (the before review, 2026-10-09). The captured `suggest`
   screen in `docs/cli-guide.md`/`.html` is held BYTE-EXACT to the fixture's own render by the CLI's regeneration
   pin (`suggest::tests::the_guide_suggest_capture_is_what_the_fixture_renders`), and A1's digest moves the key
   line and, at that fixture, the answer (a tie's keyed draw). "The CLI is touched only at `suggest::stream_seed`"
   reads "…and the guide pair's captured `suggest` screen, re-captured from one run of the fixture, never
   edited" — CLAUDE.md's guide rule for a user-visible CLI change.
9. **§2.1 B7, the catcher and the stored document** (the before review). The corpus cannot carry the B7 case —
   the walk's joiner joins WITH a name — so the fence is the layer's cells (the unit cell's three shapes, the
   nameless-joiner route cell), not a corpus pin; a text rule on `invited` cannot tell a seat the creator typed
   from one that was joined. The STORED document of a creator-bound seat gains `"bound_at_create": true` (a
   joined seat's bytes are unchanged), and a table stored before the deploy stops echoing the creator's typed
   addresses for still-nameless seats, once — the compatibility default, chosen.
10. **§2.1 B3 and R-SEC8(b), the third host** (the before review). The browser host (`crates/yokohama-wasm`)
    builds its `Games` through `Games::with_parts` too and reads `SoloOnly` like the NAS and the Worker; its
    harness — the crate's seeded cell and the browser half's walk replay — opts in by name through a second
    export, `init_diagnostic()`, the Worker switch's shape. Correction 4's list of opt-ins reads: on the NAS
    binary never; on the Worker through `YOKOHAMA_DIAGNOSTIC_SEEDS`, `on` in the pool's two test files; on the
    browser host through `init_diagnostic()`, which the client never calls.

## Global Constraints

- `SEMANTICS_REV` (`crates/yokohama-core/src/game/semantics.rs`) stays **7**; `crates/yokohama-core`,
  `crates/yokohama-oracle`, `crates/yokohama-difftest`, `crates/yokohama-bridge` and `crates/gen-data` are
  byte-identical to `e9824ec9` (`git diff e9824ec9 -- crates/yokohama-core crates/yokohama-oracle
  crates/yokohama-difftest crates/yokohama-bridge crates/gen-data` empty at every commit).
- The CLI is touched in ONE module, `crates/yokohama-cli/src/suggest.rs` (`stream_seed`, its doc, its cells),
  and its manifest (`sha2`); `decide`, `bot.rs`, `why.rs`, the arena and every bot figure are untouched (spec
  §6). **The guide pair moves with it** (`docs/cli-guide.md` + `.html`): the captured `suggest` screen is held
  BYTE-EXACT to the fixture's own render by `suggest::tests::the_guide_suggest_capture_is_what_the_fixture_renders`,
  so it is RE-CAPTURED from one run of that fixture (Task 1 Step 4b), never edited — CLAUDE.md's guide rule for a
  user-visible CLI change. `cargo test -p yokohama-cli` reads 857 passed / 2 ignored across its binaries
  (855 + the two digest cells).
- No bot behaviour moves: `suggest`'s ANSWER at a given position changes with its key (a different belief
  sample), which is A1's intended effect and not a bot change — the production contestant, its knobs and
  `decide`'s stream are untouched.
- The wire corpus (`web/src/wire/`) changes in exactly TWO TASKS, Task 1 (its task commit and, by one `reason`
  line, its reviewed fix commit) and Task 3; each regeneration is run ONCE for its commit, its diff READ and the reading recorded in the task report, naming every file that
  moved and why (the M6.1 lesson: a regenerated corpus freezes a leak as faithfully as a fix). What each moves
  was MEASURED in the before review and is written into Step 10(c) and Task 3's Step 5; a file beyond those
  lists is a finding.
- Stored documents stay readable both ways: `SeatDoc::Human`'s new field is `#[serde(default,
  skip_serializing_if = "is_false")]`, so a document written before it reads `false` and a joined seat's bytes
  are what they were; `TABLE_SCHEMA` stays 3.
- Every refusal this plan adds is a plain sentence, a constant in the module that raises it, and a 400, 403 or
  409 through the layer's own `GameError`/`ApiError` arms — no new status shape.
- TDD: the failing cell first, then the code, then green; `cargo clippy --workspace --all-targets -- -D warnings`
  clean after every task; no `cargo fmt`; stage with explicit paths, never `git add -A`; one commit per task
  at its end (the core PR's shape), the task's suites green at that commit.
- The branch is `security-s1`, cut from master at `e9824ec9` (the docs close on top of the core PR's merge
  `9ab8c430`). **The cycle ends with a PUSHED PR against master, its merge left undone** (the owner, 2026-10-09):
  S1b and S1c stack their commits on the same branch and the same PR, and the merge is S1's wrap, after S1c, on
  the owner's word (spec §9.2 as amended by that note).
- Every task that touches what the layer compiles runs the lane's rows (`docs/web-lane.md` §2) whole before its
  commit — `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`; `cd web && npm run build:wasm
  && npm run typecheck && npm test -- --run && npm run test:wasm`; `cd worker && npm run build:module:debug && npm
  run typecheck && npm run test:scripts && npm test` — and records every count (Task 3's review made the subset
  the first drafts named a finding).
- Every dispatched agent: the full report to a file under `docs/temp/security-review-2026-10-08/s1a-sdd/`
  (gitignored), a three-line reply; no `rm -rf`, `rm -r` or bulk `rm -f`; no Monitor, no event waits; long
  commands in the foreground or polled; reviewers run their mutants on a `git archive` export, never in the
  implementer's tree.

## Review Focus

Seven inputs the spec implies and this plan pins, each to the task that owns the code:

1. **`why?event=n` for a bot move made while its starting order was hidden, asked AFTER the card is public
   or, by a seat holder, after the game is over.** Visibility is a fact about NOW, not about the decision state:
   served in full. Task 1's finished-game cell asks about the bot's FIRST move at a finished game and reads the
   full account. **A WATCHER at a finished game reads the engine's view** (Task 1's review, I-1): withheld while
   the bot's starting order is still in its hand — the record route refuses a spectator the record at every
   status, and `why` must not serve them what it refuses.
2. **A spectator's `why`.** A watcher holds no seat; the mover's hand is masked for every non-mover seat alike
   (`observe.rs`), so the spectator reads what any opponent reads: withheld while hidden, served once public.
   Task 1's withheld cell asks as a watcher too.
3. **A hot-seat creator who holds every human seat beside a bot.** The bot's seat is nobody's, so its card is
   hidden from the person at every seat they hold: withheld. Task 1's withheld cell is exactly that table
   (`a_game(.., &[0])` seats the one caller at seat 1 beside the bot).
4. **An undo whose unit is a trailing standalone reveal, or a move that drew nothing.** Only a shrunken deck
   refuses; Task 2's cell undoes a non-drawing move at the shared table first and reads 200.
5. **A seeded create whose other seats are BOTS.** A bot is not a person: honoured. Task 4's cell creates
   `[creator, bot]` with a seed and reads 201; `[creator, open]` and `[creator, another person]` read 400.
6. **A creator who typed their OWN address for a second seat (a hot seat).** `bound_at_create` is true and the
   address is the reader's own: `invited` shows it to them, as before. Task 3's unit cell covers the three
   shapes: creator-typed other, creator-typed self, joiner.
7. **A lockdown whose reason itself carries a number.** The reply carries the reason verbatim and nothing
   below it; the filter drops the DUMP (`seed:`, `turn:`, the log tail), never a line of the reason. Task 5's
   unit cell feeds a reason with digits in it.

## Execution shape

- Tasks 1–5: implementer **Opus**, reviewer **Opus**; Task 6: implementer **Sonnet**, reviewer **Opus** (the
  owner's tiers for this sequence, 2026-10-09). Task 7: the orchestrator, **Mythos**, who conducts the "before"
  plan review (executing Tasks 1–5's cells against scratch implementations of this plan's code in an export,
  reading every named mutant red) and the "after" whole-branch review itself — the two reviews that stand in for
  a wider-context reviewer on Task 1.
- Every implementer dispatch carries the probe-and-name-concerns paragraph (the A1 art-cycle lesson): try the
  mechanism on cases the brief's cells do not cover, with vacuity as the first lens — for each cell, which
  mutation makes it fail? — and report `DONE_WITH_CONCERNS` naming any sentence of this plan the code
  contradicts; a `DONE_WITH_CONCERNS` correctness item is ruled on and fixed as a new commit BEFORE the task's
  review. Every fix-round dispatch requires the hunks quoted and `git diff` run before the commit.
- The SDD ledger (`docs/temp/security-review-2026-10-08/s1a-sdd/progress.md`) records each task's tier, its
  commit, its review verdict and every `Ruling:` line, as the core PR's did.

## Names shared across tasks

| name | where | meaning |
|---|---|---|
| `stream_seed(record: &GameRecord, seat: PlayerId) -> u64` | `crates/yokohama-cli/src/suggest.rs:341`, `pub` | the suggest stream's key — from Task 1 a one-way digest: SHA-256 over `[SUGGEST_SALT, seed, player_count, start_player, choices.len(), each choice (Keep(k) → 1+k, Unknown → 0), seat, events.len()]` as big-endian `u64`s, the digest's first eight bytes big-endian |
| `SUGGEST_SALT: u64 = 0x5966_2026_0727_0001` | `suggest.rs:88`, private | the domain-separation salt, the digest's first component |
| `explain::Visibility { Public, Withheld }` · `explain::visibility_of(now: &GameState, mover: PlayerId, held: &[PlayerId]) -> Visibility` | `crates/yokohama-web/src/explain.rs`, `pub` | whether the mover's account may be served to a requester who holds `held` (empty for a spectator) at `now`: Public to a seat holder at game over or to the mover's own seat, else the engine's view (Task 1) |
| `explain::WhyWithheld { provenance: Provenance, text: String, reason: &'static str }` · `explain::WITHHELD_REASON: &str` | `explain.rs`, `pub` | the withheld half of `Why` and its one sentence (Task 1) |
| `explain::Why { ok: Option<WhyOk>, withheld: Option<WhyWithheld>, refusal: Option<RefusalDto> }` · `Why::account(a: &WhyAccount, rules: &Rules, visibility: Visibility) -> Why` | `explain.rs`, `pub` | exactly one of the three non-null; `account` replaces `granted` (Task 1) |
| `explain::Provenance { contestant, evaluator_version, index, seat, recorded_version, current_version }` · `explain::WhyOk { provenance, text, explanation, continuation, counters, search_hit_ply_cap, ranking, forced }` | `explain.rs`, `pub` | `decision_seed` gone; `forced` moved from the claim to the account (Task 1) |
| `api::UNDO_DREW: &str` | `crates/yokohama-web/src/api.rs`, `pub(crate)` | the sentence an undo that un-draws a deck card at a shared table is refused with (Task 2) |
| `SeatDoc::Human { user: String, name: Option<String>, bound_at_create: bool }` | `crates/yokohama-web/src/table.rs:46`, `pub` | `bound_at_create` serde-default `false`, skipped when false; `true` on every seat `config_of` binds at create, `false` on a join (Task 3) |
| `games::SeedPolicy { SoloOnly, Diagnostic }` · `Games::with_seed_policy(self, SeedPolicy) -> Self` · `Games::seed_policy(&self) -> SeedPolicy` · `tables::SEED_NOT_SOLO: &str` | `crates/yokohama-web/src/games.rs`, `tables.rs`, `pub` | the production rule and the fixtures' opt-in; the create's refusal sentence (Task 4) |
| `AppState::with_seed_policy(self, SeedPolicy) -> Self` | `crates/yokohama-web/src/lib.rs`, `pub` | the builder step the corpus walk and the gate's harness call (Task 4) |
| `YOKOHAMA_DIAGNOSTIC_SEEDS` · `InitConfig.diagnostic_seeds: Option<bool>` | `worker/src/config.ts`, `crates/yokohama-worker/src/config.rs` | the Worker's seed switch: `on` only in `wrangler.test.jsonc` and `wrangler.open.test.jsonc`; the deploy guard refuses it in a deployable file (Task 4) |
| `api::lockdown_messages(lines: Vec<String>, reason: &str) -> Vec<String>` | `api.rs`, `pub(crate)` | the `input` reply's messages when THIS step engaged a lockdown: the lines up to and including the lockdown line, or the reason alone (Task 5) |
| `api::RECORD_UNSAVED: &str` | `api.rs`, `pub(crate)` | the 409 a finished game whose stored record is not the handle's finished game answers — shorter, or of the same length without the digest of the handle's live state (Task 5) |
| `observe(state, seat) -> Result<PlayerView, ViewError>` · `HandCard::HiddenStartingOrder` · `PlayerView.players[i].hand` | `crates/yokohama-core/src/game/observe.rs:26`; `types/view.rs:28` — at the crate root as `yokohama_core::game::observe`, `yokohama_core::HandCard` | the engine's own masking rule Task 1's visibility reads: for a non-viewer seat the unfulfilled starting order is `HiddenStartingOrder` (`observe.rs:160-176`) |
| `state_at(record, k, rules)` · `GameState.order_deck` / `tech_deck` | `yokohama_core` | Task 2 compares the deck lengths of the state before the trailing event with the current state's |
| `wire_fixtures::constructed()` · `BOT_SEED` · `SETUP_SEED` | `crates/yokohama-web/tests/wire_fixtures.rs:2170`, `:99`, `:60` | where the `why` fixtures are built in process (Task 1 adds `why_withheld` and rebuilds `why` from a finished game) |
| the route-cell helpers `live_router`, `call`, `a_game(router, name, bots)`, `start_as(state, name, seed, seats)`, `play_turn(state, id, user)`, `get_as`, `post_as`, `a_finished_two_person_game(name)`, `ENDED_SEED` | `crates/yokohama-web/src/lib.rs` `mod tests`, lines 2093, 2097, 2186, 2251, 2329, 3324, 3338 | what Tasks 1, 2, 3 and 5's route cells reuse |

## File structure

- Modify `crates/yokohama-cli/src/suggest.rs` (`stream_seed` at :341–362 and its doc; two cells after
  `the_stream_key_is_pure_and_every_component_moves_it`, :1150) and `crates/yokohama-cli/Cargo.toml`
  (`sha2 = "0.10"` under `[dependencies]`) — Task 1.
- Modify `crates/yokohama-web/src/explain.rs` (`Suggest` :294–336, `Provenance` :340–369 and :501–514, `Why`/`WhyOk`
  :378–455, `word_line` :249; new `Visibility`, `visibility_of`, `WhyWithheld`, `WITHHELD_REASON`,
  `word_line_masked`; cells in `mod tests`, :809) and `crates/yokohama-web/src/api.rs` (`why` :1575–1614; route
  cells in `crates/yokohama-web/src/lib.rs` `mod tests`: :4551 and :4675 rewritten, one added) — Task 1.
- Modify `crates/yokohama-web/tests/wire_fixtures.rs` (`constructed()` :2170–2340: `why` from the finished game,
  `why_withheld` added; the inversion pin after `no_committed_payload_carries_the_setup_seed`, :2955) — Task 1.
- Modify `web/src/types.ts` (:852–915), `web/src/wire/contract.ts` (:193–196, :445–478, :688–703, :757–763,
  :821–827, :953–955), `web/src/Explain.tsx` (:349–352, :357–406), `web/src/Explain.test.tsx` (:128–129,
  :145–151); regenerate `web/src/wire/` ONCE — Task 1.
- Modify `crates/yokohama-web/src/api.rs` (`undo_allowed` :719–768, `trailing_mover` :799–813; cells in
  `lib.rs` `mod tests` after :3187) — Task 2.
- Modify `crates/yokohama-web/src/table.rs` (`SeatDoc::Human` :46–55, `SeatView::of` :500–510, every
  `SeatDoc::Human { .. }` pattern and construction: 15 in this file), `crates/yokohama-web/src/tables.rs`
  (`config_of` :255, `join` :1603, 14 sites), `api.rs` (4), `backup.rs` (1), `archive.rs` (2), `client.rs` (1),
  `bots.rs` (1), `games.rs` (1), `crates/yokohama-worker/src/house.rs` (:141, a pattern);
  `tests/wire_fixtures.rs` (the leakage pin's `invited` arm, :2885–2898) — Task 3.
- Modify `crates/yokohama-web/src/games.rs` (`Games` :264, `with_parts` :459–481, the `games()` test helper
  :3132), `tables.rs` (`create` :787–800), `lib.rs` (`AppState` builders :556–580; `live_router`/`start` keep
  the default), `tests/wire_fixtures.rs` (`Api::with_chat`/`with_archive`, :283–306), `tests/play_a_whole_game.rs`
  (`Api::new`/`over`, :104–110 — the gate's creates need NO opt-in; stated), `web/src/NewGame.tsx` (:185–240,
  :388–420), `web/src/NewGame.test.tsx` (:326+), `crates/yokohama-worker/src/config.rs` (:30–70, :143–167),
  `crates/yokohama-worker/src/lib.rs` (:191–200, the test helper :656–666), `worker/src/config.ts` (:46–66,
  :193, :256–300), `worker/wrangler.test.jsonc` and `worker/wrangler.open.test.jsonc` (`vars`),
  `worker/scripts/deploy-guard.mjs` (a rule beside check 7, :268) and `worker/scripts/deploy-guard.test.mjs` —
  Task 4.
- Modify `crates/yokohama-web/src/api.rs` (`input` :1368–1441, `record` :2110–2134; cells in `api.rs`'s
  `mod tests` :2444 and `lib.rs`'s) — Task 5.
- Modify `docs/web-lane.md` (§4 :151 and :161, §13 the `…/table`, `why` rows, §16 the `why` row's purpose),
  `docs/plans/2026-10-08-security-remediation-spec.md` (§9.3 appended), `docs/security-ledger.md` (the eight
  rows, at the close), `docs/handover.md`, `guidingpath.md` (the row, at the close) — Task 6 (the close's rows
  are the orchestrator's, Task 7).

## Task order and why

Task 1 first and alone on the wire: it holds every row that changes a committed byte, so the corpus
regenerates once. Task 2 (undo) and Task 3 (seats) are independent layer rules on files Task 1 did not open.
Task 4 (the seed rule) last among the code tasks because it is the one the owner may re-rule (§"Spec
corrections" 4) — dropping or reshaping it disturbs nothing before it. Task 5 (the two Lows) is small and
independent. Task 6 writes the documents the code changed. Task 7 is the orchestrator's frame.

---

### Task 1: The explain surface — the stream key one-way at the source, `suggest` without it, `why` at the requester's visibility, the corpus once — Opus (A1, B1; R-SEC3; Review Focus 1–3)

**Files:**
- Modify: `crates/yokohama-cli/Cargo.toml` (`[dependencies]`, after `rand_chacha.workspace = true`)
- Modify: `crates/yokohama-cli/src/suggest.rs` (:324–362 the doc and the function; `mod tests` after :1186)
- Modify: `crates/yokohama-web/src/explain.rs` (see the file structure)
- Modify: `crates/yokohama-web/src/api.rs:1575–1614` (`why`)
- Modify: `crates/yokohama-web/src/lib.rs` `mod tests` (:4551–4605 `suggest_carries_…`, :4675–4702
  `why_accounts_…`; one new cell and one new helper after `a_finished_two_person_game`, :3400)
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs` (:2170–2340 `constructed()`; a new pin after :3027)
- Modify: `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts` (:827), `web/src/Explain.tsx`,
  `web/src/Explain.test.tsx`
- Modify: three cells that compare a WHOLE `why` payload (Step 9(c)): `api.rs`'s
  `a_finished_foreign_game_opens_scores_downloads_and_masks_the_conversion` and
  `why_reconstructs_a_runnable_non_production_bot_and_refuses_a_foreign_one_with_its_reason`, `games.rs`'s
  `a_same_name_re_point_is_told_apart_in_every_refusal_and_keeps_its_label_on_every_screen`
- Re-capture: `docs/cli-guide.md` + `docs/cli-guide.html` (the captured `suggest` screen, Step 4b)
- Modify: `worker/test/corpus.test.ts` (`THIRD_COLUMN`, :99) and `web/src/local/corpus.node.test.ts` (`NOT_SERVED`,
  :155) — the two lists of the constructed fixtures, which gain `"why_withheld"` (Step 12)
- Regenerate: `web/src/wire/suggest.json`, `why.json`, `why_withheld.json` (new), `why_refused.json`,
  `why_foreign.json` — and nothing else
- Test: the CLI's `suggest::tests`, the layer's `explain::tests` and `lib.rs` route cells, `wire_fixtures`,
  `web/` vitest and typecheck

**Interfaces:**
- Consumes: `yokohama_core::game::observe`, `yokohama_core::HandCard`, `yokohama_core::is_game_over`;
  `yokohama_cli::why::WhyAccount` (`reconstruction.seat`, `mv`, `explain`, `state`); the route helpers named
  above; `sha2::{Digest, Sha256}`.
- Produces: everything in §"Names shared across tasks" marked Task 1.

- [ ] **Step 1: The CLI's failing cells.** In `crates/yokohama-cli/src/suggest.rs`'s `mod tests`, directly
  after `the_stream_key_is_pure_and_every_component_moves_it` (its closing brace is at :1186), add:

```rust
    /// The 2026-10-09 derivation (R-SEC3, finding A1): the stream key is a
    /// ONE-WAY digest, pinned by VALUE at one record so the derivation cannot
    /// drift silently — recomputed here by hand, with no `stream_seed` in the
    /// loop, so the two cannot agree by accident. The components are the ones
    /// the old fold mixed, with the choice count prefixed so the byte string
    /// is injective over records of different player counts.
    #[test]
    fn the_stream_key_is_the_leading_eight_bytes_of_a_sha256_over_the_components() {
        use sha2::{Digest, Sha256};
        let rules = Rules::standard();
        let mut record = GameRecord::new(
            7,
            GameConfig::for_player_count(2),
            vec![StartingOrderChoice::Keep(1), StartingOrderChoice::Unknown],
            &rules,
        );
        record.events.push(RecordedEvent::Move(Move::EndAdditionalPhase));
        let seat = PlayerId::new(1);
        let mut h = Sha256::new();
        for component in [
            SUGGEST_SALT,
            7,
            u64::from(record.config.player_count),
            u64::from(record.config.start_player.0),
            record.choices.len() as u64,
            2, // Keep(1) -> 1 + 1
            0, // Unknown -> 0
            u64::from(seat.0),
            record.events.len() as u64,
        ] {
            h.update(component.to_be_bytes());
        }
        let expected = u64::from_be_bytes(h.finalize()[..8].try_into().expect("eight bytes"));
        assert_eq!(stream_seed(&record, seat), expected);
    }

    /// The derivation this one REPLACES — a SplitMix64 fold, every step a
    /// bijection — inverted from the NEW key with every other component known
    /// does not name the seed: the invertibility is what A1 exploited and what
    /// is gone. The inverse is written from SplitMix64's published inverse
    /// constants and proven against the forward fold FIRST (anti-vacuity), so
    /// the `assert_ne` is about the derivation and not about a broken inverse.
    #[test]
    fn the_old_fold_inverted_from_the_new_key_does_not_name_the_seed() {
        const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
        fn unmix(mut z: u64) -> u64 {
            z = (z ^ (z >> 31) ^ (z >> 62)).wrapping_mul(0x3196_42B2_D24D_8EC3);
            z = (z ^ (z >> 27) ^ (z >> 54)).wrapping_mul(0x96DE_1B17_3F11_9089);
            z ^ (z >> 30) ^ (z >> 60)
        }
        // One old step: `step = acc ^ component; acc' = splitmix64(&mut step)`,
        // and `splitmix64` adds the gamma before mixing — so one step back is
        // `acc = (unmix(acc') - gamma) ^ component`, the seed's step last.
        fn unfold(key: u64, rest: &[u64]) -> u64 {
            let mut acc = key;
            for component in rest.iter().rev() {
                acc = unmix(acc).wrapping_sub(GAMMA) ^ component;
            }
            unmix(acc).wrapping_sub(GAMMA) ^ SUGGEST_SALT
        }
        fn old_fold(seed: u64, rest: &[u64]) -> u64 {
            let mut acc = SUGGEST_SALT;
            for component in std::iter::once(seed).chain(rest.iter().copied()) {
                let mut step = acc ^ component;
                acc = crate::bot::splitmix64(&mut step);
            }
            acc
        }
        let rules = Rules::standard();
        let seat = PlayerId::new(0);
        for seed in [7u64, 20_260_831, 0x9E37_79B9_7F4A_7C15, u64::MAX] {
            let record = GameRecord::new(
                seed,
                GameConfig::for_player_count(2),
                vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(1)],
                &rules,
            );
            // The old fold's components after the seed: players, start seat,
            // Keep(0) -> 1, Keep(1) -> 2, the seat, the event count.
            let rest = [
                u64::from(record.config.player_count),
                u64::from(record.config.start_player.0),
                1,
                2,
                u64::from(seat.0),
                0,
            ];
            assert_eq!(unfold(old_fold(seed, &rest), &rest), seed, "premise: the inverse inverts the old fold");
            assert_ne!(
                unfold(stream_seed(&record, seat), &rest),
                seed,
                "the new key inverted to the seed at {seed}: the derivation is a bijection chain again",
            );
        }
    }
```

- [ ] **Step 2: Run them to verify they fail.**

Run: `cargo test -p yokohama-cli --lib suggest::tests 2>&1 | grep -E 'the_stream_key|the_old_fold|test result'`
(the filter `suggest::tests::the_stream_key` would run the digest cell alone — the inversion cell is
`the_old_fold_…`).
Expected: a compile error naming `sha2` as an unresolved crate (the manifest has no such dependency yet); after
Step 3's manifest line alone, the digest cell FAILS on `assert_eq!` (the fold's value against the digest — measured
11672016970538341813 against 3441804343776740209) and the inversion cell FAILS on `assert_ne!` at seed 7 ("the
derivation is a bijection chain again": the old fold inverts, as the finding said).

- [ ] **Step 3: The manifest and the function.** In `crates/yokohama-cli/Cargo.toml`, after
  `rand_chacha.workspace = true`:

```toml
# The 2026-10-09 derivation of `suggest::stream_seed` (security S1a, finding A1,
# R-SEC3): a one-way digest in place of a SplitMix64 fold. The same crate and
# version `yokohama-web` already builds, on wasm32 too (its table password).
sha2 = "0.10"
```

Replace `stream_seed`'s body and the last paragraph of its doc (:336–340, "Deriving an agent stream from the
world seed is the arena's own shape … the summary shows it.") with:

```rust
/// **A ONE-WAY digest since 2026-10-09** (security S1a, finding A1; R-SEC3):
/// SHA-256 over the components above, as big-endian `u64`s with the salt first
/// and the choice count ahead of the choices, the digest's first eight bytes
/// read big-endian. Determinism and keying are kept — `stream_seed` is pure
/// and every component moves it, pinned below — and INVERTIBILITY is gone: the
/// earlier SplitMix64 fold was a chain of bijections, so a served key read with
/// the other components known NAMED `record.seed`, the first member of the
/// engine's hidden set (`game/observe.rs`), from one `GET …/suggest`. The web
/// layer no longer serves the key at all; this derivation closes the channel
/// at the source so that no future surface repeats it. Deriving the stream
/// from the world seed stays the arena's own shape (`agent_stream_seed`): what
/// the SEED HYGIENE rule forbids is a sampler seed derived from state the agent
/// may not see, and in sim mode the seed is the operator's own.
pub fn stream_seed(record: &GameRecord, seat: PlayerId) -> u64 {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    let mut component = |x: u64| h.update(x.to_be_bytes());
    component(SUGGEST_SALT);
    component(record.seed);
    component(u64::from(record.config.player_count));
    component(u64::from(record.config.start_player.0));
    component(record.choices.len() as u64);
    for choice in &record.choices {
        // `Keep(k)` maps to `1 + k` so it can never collide with `Unknown`'s 0.
        component(match choice {
            StartingOrderChoice::Keep(k) => 1 + u64::from(*k),
            StartingOrderChoice::Unknown => 0,
        });
    }
    component(u64::from(seat.0));
    component(record.events.len() as u64);
    u64::from_be_bytes(h.finalize()[..8].try_into().expect("a SHA-256 digest holds eight bytes"))
}
```

(`h` is borrowed mutably by the closure for the component calls and released before `finalize`; if the borrow
checker objects at the implementer's compiler, write the eleven `h.update(..to_be_bytes())` lines out — the
bytes are the contract, not the closure.)

- [ ] **Step 4: Run the suggest cells, then the CLI suite.**

Run: `cargo test -p yokohama-cli --lib suggest:: 2>&1 | tail -15`
Expected: every cell PASS but ONE — the purity/keying cell unchanged (a digest keeps both), the two new ones green,
and the guide pin (next) red until Step 4b: 17 passed / 1 failed.
Run: `cargo test -p yokohama-cli 2>&1 | grep -E 'test result|FAILED|panicked' | head`
Expected: ONE cell red — `suggest::tests::the_guide_suggest_capture_is_what_the_fixture_renders` (the lib reads
828 passed / 1 failed), the T11 regeneration pin that holds the guide's captured `suggest` screen
(`docs/cli-guide.md` :283) BYTE-EXACT to `guide_capture(&guide_capture_fixture())`. The digest moves the key line
AND, at this fixture, the answer: the two top candidates tie, and the keyed draw under `0xf360553bebf4ba9e` picks
"Port A (A00), Laboratory A (A01), Customs (A05)" where `0x7199df924be0e320` picked "… Tea Plantation (A02)";
the `production_access`/`track_access` rows swap with it (measured in the before review — the whole-answer
change A1 intends, exactly as Step 10(c) says of `suggest.json`). `guide_sync`'s own string pin
(`the_captured_suggest_screen_matches_the_renderers_own_strings`) stays green either way.

- [ ] **Step 4b: Re-capture the guide's suggest screen, in BOTH formats, from ONE run.** Take the fixture's
  render — the pin's panic prints it as `right:`, or print `guide_capture(&guide_capture_fixture())` — and
  replace the whole fenced block in `docs/cli-guide.md` (the block opening `─ Suggest ───…`) with it, never by
  editing a line; then `python3 docs/tools/mkguide.py` regenerates `docs/cli-guide.html` from the `.md` (the
  twin is generated, not hand-kept). No prose around the block names the suggested move, so nothing else in the
  guide moves. Run: `cargo test -p yokohama-cli 2>&1 | grep -E 'test result|FAILED'` — every binary green:
  **857 passed / 2 ignored** across them (829 lib + 9 + 18 + 1). Read `git diff docs/cli-guide.md`: the key
  line, the suggested move (three lines), the two swapped feature rows and the runner-up row — nothing else.

- [ ] **Step 5: The layer's failing cells.** In `crates/yokohama-web/src/explain.rs`'s `mod tests` (:809),
  after `only_the_played_fulfill_is_worded_when_the_hand_is_another_seats`, add the three cells and their
  helper:

```rust
    /// A vs-bot game built in process under a fixed bot seed (the corpus's
    /// shape, `wire_fixtures.rs`'s `constructed()`): seat 0 the production
    /// bot, seat 1 a person; `pumped` is how many times the bot pump runs —
    /// ONCE stops at the person's first turn with the bot's starting order
    /// surely unfulfilled, `usize::MAX` plays the game out with both seats
    /// pumped as bots.
    fn vs_bot(pumped: usize) -> (yokohama_cli::repl::Repl, yokohama_cli::bot::BotSeats) {
        use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
        use yokohama_cli::session::Session;
        use yokohama_core::{GameConfig, PlayerColor, StartingOrderChoice};
        const SEED: u64 = 20_260_831;
        const BOT_SEED: u64 = 0x4649_5854_5552_4553;
        let seats = if pumped == usize::MAX {
            vec![Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)]
        } else {
            vec![Some(PROVISIONAL_DEFAULT_BOT), None]
        };
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let choices = vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)];
        let mut session = Session::new_sim(config, SEED, choices, Rules::standard()).expect("a game");
        let bots = BotSeats::new(seats, BOT_SEED);
        session.set_run_seats(bots.seat_provenance());
        let mut repl = yokohama_cli::repl::Repl::new(session);
        for _ in 0..pumped.min(400) {
            if repl.session.is_over() {
                break;
            }
            repl.pump_bots();
        }
        (repl, bots)
    }

    /// `why`'s account of `event` (the last bot move when `None`) over the
    /// pumped game, as the route builds it.
    fn account_of(
        repl: &yokohama_cli::repl::Repl,
        bots: &yokohama_cli::bot::BotSeats,
        event: Option<usize>,
    ) -> WhyAccount {
        yokohama_cli::why::account_parts(repl.session.mode, repl.session.record(), repl.session.rules(), bots, event)
            .unwrap_or_else(|r| panic!("a bot move to account for: {r}"))
    }

    /// **Visibility is the engine's own view** (finding B1): withheld while
    /// the requester's view masks the mover's starting order, public to the
    /// mover's own seat, public once the card is out of the hand — and at game
    /// over the record route's own condition (review I-1): public to a SEAT
    /// HOLDER whatever the hand holds, while a watcher still reads the
    /// engine's view, withheld for a seat whose starting order never left its
    /// hand.
    #[test]
    fn visibility_follows_the_requesters_view_of_the_movers_card_and_the_record_routes_game_over_rule() {
        let (repl, _) = vs_bot(1);
        let now = repl.session.state();
        let mover = PlayerId::new(0);
        let view = yokohama_core::game::observe(now, PlayerId::new(1)).expect("seat 1's view");
        assert!(
            view.players[0].hand.contains(&yokohama_core::HandCard::HiddenStartingOrder),
            "premise: after one bot turn the bot's starting order is a masked slot in its hand",
        );
        assert_eq!(visibility_of(now, mover, &[PlayerId::new(1)]), Visibility::Withheld, "the opponent");
        assert_eq!(visibility_of(now, mover, &[]), Visibility::Withheld, "a spectator");
        assert_eq!(visibility_of(now, mover, &[PlayerId::new(0)]), Visibility::Public, "the mover's own seat");
        assert_eq!(
            visibility_of(now, mover, &[PlayerId::new(1), PlayerId::new(0)]),
            Visibility::Public,
            "a holder of both seats holds the mover's",
        );
        // The card out of the hand — fulfilled, in the engine's representation
        // `completed_orders` — and the same view no longer masks it.
        let mut fulfilled = now.clone();
        let card = fulfilled.players[0].starting_order.expect("a Keep-shaped seat records its card");
        fulfilled.players[0].order_cards_in_hand.retain(|c| *c != card);
        fulfilled.players[0].completed_orders.push(card);
        assert_eq!(visibility_of(&fulfilled, mover, &[PlayerId::new(1)]), Visibility::Public, "a public card");
        // Game over: the record route's own condition. The holder of EITHER
        // seat reads everything, whatever the hand still holds; a watcher
        // reads the engine's view, which still masks a starting order that
        // never left the hand — read here, seat by seat, not assumed.
        let (over, _) = vs_bot(usize::MAX);
        assert!(over.session.is_over(), "premise: the both-bot pump finishes the game");
        let end = over.session.state();
        let masked: Vec<bool> = [0u8, 1]
            .iter()
            .map(|&seat| {
                yokohama_core::game::observe(end, PlayerId::new(1 - seat))
                    .expect("the other seat's view")
                    .players
                    .iter()
                    .find(|p| p.id == PlayerId::new(seat))
                    .is_some_and(|p| p.hand.contains(&yokohama_core::HandCard::HiddenStartingOrder))
            })
            .collect();
        assert!(
            masked.contains(&true),
            "premise: at game over some seat still holds its starting order, or the holder rows prove nothing",
        );
        for seat in [0u8, 1] {
            let mover = PlayerId::new(seat);
            for holder in [0u8, 1] {
                assert_eq!(
                    visibility_of(end, mover, &[PlayerId::new(holder)]),
                    Visibility::Public,
                    "seat {seat}'s account to the holder of seat {holder} at game over",
                );
            }
            let watcher = if masked[usize::from(seat)] { Visibility::Withheld } else { Visibility::Public };
            assert_eq!(visibility_of(end, mover, &[]), watcher, "seat {seat}'s account to a watcher at game over");
        }
    }

    /// **A withheld account is the same bytes whatever the search found**
    /// (the independence pin, finding B1): two accounts of the same move that
    /// differ in every card-dependent field — the feature table, the total,
    /// the ranking, the continuation, the counters, the decision seed, the
    /// forced fact — serialize to one `withheld` payload. What survives is
    /// the move, the claim and the sentence.
    #[test]
    fn a_withheld_account_is_the_same_bytes_whatever_the_search_found() {
        let (repl, bots) = vs_bot(1);
        let rules = Rules::standard();
        let a = account_of(&repl, &bots, None);
        let mut b = account_of(&repl, &bots, None);
        b.explain.explanation.total += 1000;
        for t in &mut b.explain.explanation.terms {
            t.viewer_value += 1;
        }
        b.explain.ranking.clear();
        b.explain.continuation.clear();
        b.explain.stats = None;
        b.explain.hit_ply_cap = !b.explain.hit_ply_cap;
        b.reconstruction.decision_seed ^= 0xDEAD_BEEF;
        b.reconstruction.forced = !b.reconstruction.forced;
        let one = serde_json::to_string(&Why::account(&a, &rules, Visibility::Withheld)).expect("json");
        let two = serde_json::to_string(&Why::account(&b, &rules, Visibility::Withheld)).expect("json");
        assert_eq!(one, two, "the withheld payload depends on something the search found");
        // And the public shape of the same two accounts DOES differ — the
        // anti-vacuity: the fields above are fields the full account carries.
        let one = serde_json::to_string(&Why::account(&a, &rules, Visibility::Public)).expect("json");
        let two = serde_json::to_string(&Why::account(&b, &rules, Visibility::Public)).expect("json");
        assert_ne!(one, two, "premise: the public account carries what the mutation moved");
        let withheld: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&Why::account(&a, &rules, Visibility::Withheld)).expect("json"),
        )
        .expect("a value");
        assert!(withheld["ok"].is_null() && withheld["refusal"].is_null());
        assert_eq!(withheld["withheld"]["reason"], WITHHELD_REASON);
        assert_eq!(withheld["withheld"]["provenance"]["seat"], 0);
        for key in ["decision_seed", "forced"] {
            assert!(withheld["withheld"]["provenance"].get(key).is_none(), "{key} rides the claim: {withheld}");
        }
    }

    /// **The served continuation masks its fulfil lines as the ranking masks
    /// its rows** (finding B1's third channel, on the public shape): a
    /// `FulfillOrder` step past the first words as [`MASKED_FULFILL`]; the
    /// first step — the move PLAYED — is never masked, even when it is a
    /// fulfil. Found by scanning a finished game for a line that fulfils
    /// later than it starts and for a move that is itself a fulfil — the
    /// anti-vacuity asserts both, so a reseed cannot skip either half.
    #[test]
    fn the_served_continuation_masks_a_later_fulfil_and_never_the_played_move() {
        let (over, bots) = vs_bot(usize::MAX);
        let rules = Rules::standard();
        let record = over.session.record();
        let mut later_fulfil = None;
        let mut played_fulfil = None;
        for n in 0..record.events.len() {
            let Ok(a) = yokohama_cli::why::account_parts(over.session.mode, record, &rules, &bots, Some(n)) else { continue };
            let line = &a.explain.continuation;
            if later_fulfil.is_none() && line.iter().skip(1).any(|m| matches!(m, Move::FulfillOrder { .. })) {
                later_fulfil = Some(a);
                continue;
            }
            if played_fulfil.is_none() && matches!(a.mv, Move::FulfillOrder { .. }) {
                played_fulfil = Some(a);
            }
            if later_fulfil.is_some() && played_fulfil.is_some() {
                break;
            }
        }
        let a = later_fulfil.expect("some bot line in a finished game fulfils later than it starts");
        let out = Why::account(&a, &rules, Visibility::Public);
        let ok = out.ok.expect("public");
        let raw: Vec<String> = a.explain.continuation.iter().map(|m| yokohama_cli::describe::describe_move(m, &a.state, &rules)).collect();
        assert_eq!(ok.continuation.len(), raw.len());
        assert_eq!(ok.continuation[0], raw[0], "step 1 is the move played, never masked");
        let masked: Vec<usize> = (1..raw.len()).filter(|&i| ok.continuation[i] == MASKED_FULFILL).collect();
        assert!(!masked.is_empty(), "no later fulfil was masked: {:?}", ok.continuation);
        for i in masked {
            assert!(matches!(a.explain.continuation[i], Move::FulfillOrder { .. }), "step {i} masked but not a fulfil");
            assert_ne!(ok.continuation[i], raw[i], "the raw wording names the card");
        }
        let played = played_fulfil.expect("some bot move of the finished game is itself a fulfil");
        let ok = Why::account(&played, &rules, Visibility::Public).ok.expect("public");
        assert_eq!(ok.text, ok.continuation[0], "a played fulfil is worded in full at step 1");
        assert_ne!(ok.continuation[0], MASKED_FULFILL);
    }
```

- [ ] **Step 6: Run them to verify they fail to compile.**

Run: `cargo test -p yokohama-web --lib explain::tests 2>&1 | grep -E '^error' | sort | uniq -c | head`
Expected: errors naming `visibility_of`, `Visibility`, `WITHHELD_REASON` and `Why::account` as not found.

- [ ] **Step 7: The layer's code.** In `crates/yokohama-web/src/explain.rs`:

(a) `Suggest` (:294–319): delete the `stream_seed` field and its three doc lines; in `Suggest::of` (:331) delete
`stream_seed: s.stream_seed.to_string(),`. Add to the struct's doc, after "the viewing seat's own view.":

```rust
/// **No stream key** (security S1a, finding A1): the key this press was made
/// under was served as a string and was a bijective fold of the setup seed —
/// the first member of the hidden set — so any seat holder inverted it from
/// one request. The CLI's derivation is one-way since the same cycle and the
/// web wire carries nothing of it; the press is still byte-identical on
/// repeat, which is the property a reader could use the key for.
```

(b) After `word_line` (:249–251) add:

```rust
/// `why`'s continuation, worded with its fulfil lines masked as [`rank_all`]
/// masks the ranking's rows (finding B1's third channel): the FIRST step is the
/// move PLAYED — public, never masked, a fulfil included — and a later
/// `FulfillOrder` step may name a card still in the mover's hand, so it reads
/// as [`MASKED_FULFILL`]. A later fulfil may also name a card the mover took
/// from the PUBLIC port earlier in the same line ("Port: take O19", then the
/// fulfil of O19): it is masked too — over-masking, which is safe, and the
/// mask does not try to tell the two apart.
fn word_line_masked(line: &[Move], state: &GameState, rules: &Rules) -> Vec<String> {
    line.iter()
        .enumerate()
        .map(|(i, m)| {
            if i > 0 && matches!(m, Move::FulfillOrder { .. }) {
                MASKED_FULFILL.to_string()
            } else {
                yokohama_cli::describe::describe_move(m, state, rules)
            }
        })
        .collect()
}

/// Whether the mover's account may be served to the requester (the 2026-10-08
/// finding B1). The account — the feature table and its total, the
/// continuation, the ranking's values, the counters, the forced fact — is a
/// function of the mover's HAND: while the mover's starting order is hidden
/// from the requester, every one of them narrows the unseen cards to it (the
/// review read the card off four feature values from one payload). So the
/// account is WITHHELD until the card is public — or, to a seat holder, until
/// the game is over — and the `withheld` half carries the move and the claim
/// alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Withheld,
}

/// [`Visibility`] of `mover`'s account to a requester who holds `held` (empty
/// for a spectator) at `now` — **the engine's own view, plus the record
/// route's own game-over condition — nothing else restated here**: `Withheld`
/// exactly while `observe(now, seat)` for the requester shows a
/// `HiddenStartingOrder` slot in the mover's hand, unless the game is over and
/// the requester holds a seat. A holder of the mover's seat reads their own
/// hand; a spectator reads as any seat that is not the mover's, since every
/// non-mover seat sees the same masked hand (`observe.rs`); a SEAT HOLDER at
/// game over reads everything — the record route serves them the whole
/// record, seed included, from there, and refuses a spectator, so a watcher
/// reads the engine's view at game over too. A view the engine refuses is a
/// view that shows nothing: withheld.
pub fn visibility_of(now: &GameState, mover: PlayerId, held: &[PlayerId]) -> Visibility {
    if yokohama_core::is_game_over(now) && !held.is_empty() {
        return Visibility::Public;
    }
    if held.contains(&mover) {
        return Visibility::Public;
    }
    let as_seat = held
        .first()
        .copied()
        .or_else(|| now.players.iter().map(|p| p.id).find(|id| *id != mover));
    let Some(as_seat) = as_seat else { return Visibility::Withheld };
    match yokohama_core::game::observe(now, as_seat) {
        Ok(view) => {
            let masked = view
                .players
                .iter()
                .find(|p| p.id == mover)
                .is_some_and(|p| p.hand.contains(&yokohama_core::HandCard::HiddenStartingOrder));
            if masked { Visibility::Withheld } else { Visibility::Public }
        }
        Err(_) => Visibility::Withheld,
    }
}

/// The one sentence the withheld half carries.
pub const WITHHELD_REASON: &str = "the bot's account is withheld while its starting order is hidden from you — \
                                   it is served once that card is public, or to a seat holder once the game is over";
```

(c) `Provenance` (:340–369): delete the `decision_seed` field with its doc (:361–362) and `forced` with its
doc (:363–366); in `Provenance::of` (:502–513) delete the two lines. Replace the struct's doc's first line with
"What the reconstruction claims, so the arithmetic beside it has a claim attached — and NOTHING the search
found: no key (the pump's `decision_seed` inverted to the bot's stream seed, finding B1) and no forced fact,
which is the account's (`WhyOk::forced`)."

(d) `WhyOk` (:385–395): add, last:

```rust
    /// The engine's legal list held exactly one move at the decision state,
    /// so the valuation above is an account of a move nothing else was
    /// competing with. On the ACCOUNT since security S1a: a fact about the
    /// mover's hand (a fulfil in the list is a card in the hand), withheld with
    /// the rest.
    pub forced: bool,
```

(e) `Why` (:378–382) becomes three halves, and `granted` becomes `account`:

```rust
/// `GET /api/games/{id}/why` — a bot move, reconstructed by replay.
///
/// **Exactly one of `ok`, `withheld` and `refusal` is non-null**, and neither
/// of the last two is a 4xx: `why` declining because the last move was a
/// human's is the ANSWER, and so is `why` withholding the account while the
/// mover's starting order is hidden from the requester (finding B1;
/// [`Visibility`]). The twelve typed refusals are data the browser renders,
/// and M2.5's finding was that a branch nobody has ever seen is exactly where
/// a defect lives.
#[derive(Debug, Clone, Serialize)]
pub struct Why {
    pub ok: Option<WhyOk>,
    pub withheld: Option<WhyWithheld>,
    pub refusal: Option<RefusalDto>,
}

/// The withheld half: the move played, the claim, and the sentence. Nothing
/// the search found — pinned as bytes by
/// `a_withheld_account_is_the_same_bytes_whatever_the_search_found`.
#[derive(Debug, Clone, Serialize)]
pub struct WhyWithheld {
    pub provenance: Provenance,
    pub text: String,
    pub reason: &'static str,
}

impl Why {
    /// The account at `visibility` ([`visibility_of`] is the route's reading).
    pub fn account(a: &WhyAccount, rules: &Rules, visibility: Visibility) -> Self {
        let provenance = Provenance::of(&a.reconstruction);
        let text = yokohama_cli::describe::describe_move(&a.mv, &a.state, rules);
        match visibility {
            Visibility::Withheld => Self {
                ok: None,
                withheld: Some(WhyWithheld { provenance, text, reason: WITHHELD_REASON }),
                refusal: None,
            },
            Visibility::Public => {
                let e: &DecisionExplanation = &a.explain;
                Self {
                    ok: Some(WhyOk {
                        provenance,
                        text,
                        explanation: Explanation::of(&e.explanation),
                        continuation: word_line_masked(&e.continuation, &a.state, rules),
                        counters: e.stats.as_ref().map(counters),
                        search_hit_ply_cap: e.hit_ply_cap,
                        // `why` accounts for a move ANOTHER seat made, and the same
                        // server serves that seat's hand as `cards: Vec::new()`.
                        ranking: rank_all(&e.ranking, &a.mv, &a.state, rules, HandVisibility::Masked),
                        forced: a.reconstruction.forced,
                    }),
                    withheld: None,
                    refusal: None,
                }
            }
        }
    }
```

`refused` (:416–418), `locked` (:423–435) and `foreign` (:446–454) each gain `withheld: None,` between `ok` and
`refusal`. Delete the old `granted` (:398–414) — `Why::account` is its only successor, and `foreign_why`
(:484–499) is unchanged.

(g) Three sentences the change makes false are reworded (Task 1's review, M-4): `HandVisibility`'s doc no longer
cites "222 rows on the committed fixture" (the regenerated `why.json` carries 66 candidates, all Placement
moves), nor does `only_the_played_fulfill_is_worded_when_the_hand_is_another_seats`'s; `word_line_masked`'s doc
says a later fulfil may also name a card the mover took from the PUBLIC port this turn — over-masking, safe.

(f) In `crates/yokohama-web/src/api.rs`'s `why` (:1575–1614): the admission's answer is kept, the current
state is cloned beside the record under the lock, and the visibility is read after the reconstruction (the two
long comments are the existing ones, carried):

```rust
/// `GET /api/games/{id}/why[?event=n]` — a bot move, reconstructed by replay.
///
/// `event` is the log's printed 0-based index; omitted means the most recent bot
/// move, which is what bare `why` does in the terminal.
///
/// **The refusals are `why::account_parts`', and not one of them is re-derived
/// here.** That function's own doc is the instruction: *"it must not
/// reimplement the first half: the refusals ARE the honesty gate."* A
/// reconstruction that does not equal the recorded move refuses, naming both
/// moves and both versions — and that refusal reaches the browser as data,
/// because a branch nobody has ever seen is where defects live.
///
/// Same lock discipline as `suggest`, for the same reason: the reconstruction
/// runs the same `decide` path the pump ran.
pub fn why(games: &Games, user: &User, id: &str, q: &WhyQuery) -> Result<explain::Why, ApiError> {
    // Seated, with the seats the caller holds in hand: `why` is served at the
    // REQUESTER's visibility (finding B1) — withheld while the mover's starting
    // order is hidden from them, which the engine's own view decides, save
    // that a seat holder reads everything once the game is over, as the record
    // route serves them the record (`explain::visibility_of`); what this fence
    // adds is the seat one — an account of a search at a table the caller has
    // no place at is not theirs to read.
    let (table, _viewer) = admitted(games, id, user)?;
    let held = table.seats_of(user);
    let handle = games.handle(id)?;
    let (mode, record, bots, now) = {
        let repl = games.lock_game(&handle, id)?;
        // **The lockdown gate fences COMPUTATION, not projection**, and this is
        // the rule the three route families follow. M2 already settled that a
        // read-only GET serves a locked session — `ports`/`labs`/`areas`/`log`
        // do, and `ClientState` reports the lockdown as its prompt rather than
        // refusing. But `why` is not a projection: it re-runs the same `decide`
        // path against a record the driver has declared faulty, and answering
        // with an authoritative account of a search over a suspect record is
        // exactly what a lockdown exists to stop. `suggest` refuses for the same
        // reason; the card indexes and the final score do not, because they read
        // the state rather than an opinion about it.
        if let Some(reason) = repl.session.record().lockdown.clone() {
            return Ok(explain::Why::locked(reason));
        }
        (
            repl.session.mode,
            repl.session.record().clone(),
            repl.bots().clone(),
            // History-free and cheap to clone (the engine's own rule); the
            // visibility is a fact about NOW, read off this copy outside the lock.
            repl.session.state().clone(),
        )
    };
    // **A move a foreign bot made is the layer's to word** (M6.5c2,
    // R-6.5c2.10): `account_parts` would call that seat a diagnostic that
    // "resumed as a HUMAN seat". Outside the lock, over the clone, as the
    // search below is.
    if let Some(refused) = explain::foreign_why(&record, games.rules(), q.event) {
        return Ok(refused);
    }
    Ok(
        match yokohama_cli::why::account_parts(mode, &record, games.rules(), &bots, q.event) {
            Ok(account) => {
                let visibility = explain::visibility_of(&now, account.reconstruction.seat, &held);
                explain::Why::account(&account, games.rules(), visibility)
            }
            Err(refusal) => explain::Why::refused(&refusal),
        },
    )
}
```

(`TableDoc::seats_of(&self, user: &User) -> Vec<PlayerId>` is the existing reader `suggest` uses at :1521.)

- [ ] **Step 8: Run the explain cells and the layer's lib.**

Run: `cargo test -p yokohama-web --lib explain::tests 2>&1 | tail -12`
Expected: the module's cells PASS (9), the three new ones included; under `-- --nocapture` nothing is printed by
them. Then `cargo test -p yokohama-web --lib 2>&1 | grep -E 'test result|FAILED|panicked'` — FIVE cells RED
until Step 9: the two route cells at `lib.rs:4551` and `:4675` (`stream_seed` asserted a string; the old `why`
cell fails at its FIRST assertion — the one-bot-turn account is now withheld, so `ok` is null) and the three cells that compare a WHOLE `why` payload against a `json!` literal — `api.rs`'s
`a_finished_foreign_game_opens_scores_downloads_and_masks_the_conversion` and
`why_reconstructs_a_runnable_non_production_bot_and_refuses_a_foreign_one_with_its_reason`, `games.rs`'s
`a_same_name_re_point_is_told_apart_in_every_refusal_and_keeps_its_label_on_every_screen` — whose literals lack
the third half (`"withheld": null`); every other lib cell green (411 passed / 5 failed, measured). Try the mutants before the reviewer does (each reverted after its
reading): **M1** — `visibility_of` returning `Visibility::Public` unconditionally: the visibility cell
(`visibility_follows_the_requesters_view_of_the_movers_card_and_the_record_routes_game_over_rule`) RED on
"the opponent" and the withheld route cell of Step 9 RED (`ok` non-null); **M2** — `word_line_masked` calling
`describe_move` for every step: the continuation cell RED ("no later fulfil was masked"); **M3** — `Why::account`'s
withheld arm carrying `e.explanation.total` anywhere (say in `text`): the independence cell RED alone; carried in
`reason` instead, the withheld route cell's sentence pin reds beside it — both are catchers. Record the
three readings in the task report.

- [ ] **Step 9: The route cells.** In `crates/yokohama-web/src/lib.rs`'s `mod tests`:

(a) `suggest_carries_the_bots_own_arithmetic_and_the_sum_foots` (:4551): replace
`assert!(ok["stream_seed"].is_string(), "a u64 is a STRING on this wire");` (:4577) with

```rust
        assert!(
            ok.get("stream_seed").is_none(),
            "the stream key is off the wire (finding A1): it inverted to the setup seed: {ok}"
        );
```

(b) Replace `why_accounts_for_a_bot_move_under_a_named_claim` (:4667–4702, doc included) with the cell below,
and add the helper after `a_finished_two_person_game` (its closing brace near :3400):

```rust
    /// A table of the bot (seat 0, the start seat) and one person (seat 1),
    /// its record played out to a WINNER in process — both seats pumped as
    /// bots under `ENDED_SEED`, the record then labelled as the table is (the
    /// bot at 0, its stream seed `ENDED_SEED`, a person at 1) and installed —
    /// so `why` reconstructs seat 0's every move under the key the pump used
    /// (`BotSeats::decision_seed` reads the table's one seed and the event
    /// count, never the seat).
    ///
    /// **The game is the one the table dealt**: its config, its seed and both
    /// keeps — the person's card 1, and the bot's OWN keep, which at this seed
    /// and seat is card 2 (measured: the stored choices read `[Keep(1),
    /// Keep(0)]`) — are read back from the record the last keep stored, never
    /// assumed. **And its bot's starting order is still in its hand at game
    /// over** (measured: engine id 5), which the cell asserts as its premise:
    /// the finished game pins the game-over rule itself, a SEAT HOLDER reading
    /// the account the engine's view would still mask and a watcher not
    /// (review I-1, M-1).
    async fn a_finished_vs_bot_game(name: &'static str) -> Arc<AppState> {
        use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
        use yokohama_cli::session::Session;

        let store = Arc::new(MemStore::default());
        let state = Arc::new(AppState::with_store(store.clone(), identity::IdentityMode::Open));
        let (status, table) = post_as(
            &state,
            "blue@wire.test",
            "/api/games",
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": ENDED_SEED.to_string(),
                "seats": [{ "bot": true }, { "user": "blue@wire.test" }], "name": name, "id": name,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{table}");
        let (status, kept) = post_as(
            &state,
            "blue@wire.test",
            &format!("/api/games/{name}/keep"),
            serde_json::json!({ "seat": 1, "card": 1 }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{kept}");

        let (dealt, _) = store.read(Doc::Record, name).expect("a store").expect("a dealt game's record");
        let dealt: yokohama_core::GameRecord = serde_json::from_str(&dealt).expect("the stored record parses");
        let mut session = Session::new_sim(dealt.config.clone(), dealt.seed, dealt.choices.clone(), Rules::standard())
            .expect("the dealt game");
        let both = BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT), Some(PROVISIONAL_DEFAULT_BOT)], ENDED_SEED);
        session.set_run_seats(both.seat_provenance());
        let mut repl = yokohama_cli::repl::Repl::new(session);
        for _ in 0..400 {
            if repl.session.is_over() {
                break;
            }
            repl.pump_bots();
        }
        assert!(repl.session.is_over(), "the both-bot pump must reach a finished game");
        repl.session
            .set_run_seats(BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT), None], ENDED_SEED).seat_provenance());
        let finished = repl.session.to_record_json().expect("a record");
        let (_, version) = store.read(Doc::Record, name).expect("a store").expect("a record");
        store.write_if_version(Doc::Record, name, &finished, Some(version)).expect("the finished record");
        Arc::new(AppState::with_store(store, identity::IdentityMode::Open))
    }

    /// **`why` is served at the REQUESTER's visibility** (finding B1): while
    /// the bot's starting order is hidden from the caller the account is
    /// WITHHELD — the move played and the claim, and nothing the search found
    /// — for the opponent and for a spectator alike (Review Focus 2, 3). At a
    /// FINISHED game whose bot still holds its starting order, the record
    /// route's own condition decides (review I-1): a SEAT HOLDER, whom that
    /// route serves the whole record, reads the full account — the bot's FIRST
    /// move included (Review Focus 1: visibility is a fact about NOW), with no
    /// key in the claim and the forced fact on the account — and a WATCHER,
    /// whom it refuses, reads the withheld half. The reconstruction is
    /// `account_parts`' — this crate re-derives no part of the honesty gate.
    #[tokio::test]
    async fn why_is_withheld_while_the_bots_starting_order_is_hidden_and_served_once_it_is_public() {
        let state = Arc::new(AppState::in_memory());
        let router = app(Arc::clone(&state));
        // Seat 0 is the bot AND the start seat, so `create`'s `pump_bots` plays
        // a whole bot turn before anything else happens; the caller holds seat 1.
        a_game(&router, "withheld", &[0]).await;
        {
            // The premise, read off the engine's own view for seat 1.
            let handle = state.games.handle("withheld").expect("a playing game");
            let repl = state.games.lock_game(&handle, "withheld").expect("the lock");
            let view = yokohama_core::game::observe(repl.session.state(), yokohama_core::PlayerId::new(1))
                .expect("seat 1's view");
            assert!(
                view.players[0].hand.contains(&yokohama_core::HandCard::HiddenStartingOrder),
                "premise: after one bot turn its starting order is a masked slot in its hand",
            );
        }
        let (status, body) = call(&router, "GET", "/api/games/withheld/why", None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["ok"].is_null() && body["refusal"].is_null(), "{body}");
        let w = &body["withheld"];
        assert_eq!(w["provenance"]["seat"], 0);
        assert!(!w["provenance"]["contestant"].as_str().unwrap_or_default().is_empty(), "names the contestant");
        assert!(!w["text"].as_str().unwrap_or_default().is_empty(), "the move PLAYED is public");
        assert_eq!(w["reason"], explain::WITHHELD_REASON);
        let text = body.to_string();
        for key in [
            "decision_seed", "explanation", "ranking", "counters", "continuation", "forced",
            "search_hit_ply_cap", "stream_seed",
        ] {
            assert!(!text.contains(&format!("\"{key}\"")), "{key} reached the wire while withheld: {text}");
        }
        // A spectator reads what the opponent reads: the table has no lock, so
        // a stranger is admitted at `playing` as a watcher.
        let (status, seen) = get_as(&state, "watcher@wire.test", "/api/games/withheld/why").await;
        assert_eq!(status, StatusCode::OK, "{seen}");
        assert!(seen["withheld"].is_object() && seen["ok"].is_null(), "{seen}");

        // A FINISHED game whose bot (seat 0) still holds its starting order —
        // the premise, read off the engine's own view for the person's seat.
        let over = a_finished_vs_bot_game("explains").await;
        {
            let handle = over.games.handle("explains").expect("a finished game");
            let repl = over.games.lock_game(&handle, "explains").expect("the lock");
            assert!(repl.session.is_over(), "premise: the installed game is finished");
            let view = yokohama_core::game::observe(repl.session.state(), yokohama_core::PlayerId::new(1))
                .expect("seat 1's view");
            assert!(
                view.players[0].hand.contains(&yokohama_core::HandCard::HiddenStartingOrder),
                "premise: the bot's starting order is still a masked slot in its hand at game over",
            );
        }

        // The SEAT HOLDER: the record route serves them the whole record, so
        // `why` serves the whole account — and no key.
        let (status, _) = get_as(&over, "blue@wire.test", "/api/games/explains/record").await;
        assert_eq!(status, StatusCode::OK, "the seat holder downloads the record");
        let (status, body) = get_as(&over, "blue@wire.test", "/api/games/explains/why").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["withheld"].is_null() && body["refusal"].is_null(), "a seat holder at game over: {body}");
        let ok = &body["ok"];
        let p = &ok["provenance"];
        assert!(!p["contestant"].as_str().unwrap_or_default().is_empty(), "names the contestant");
        assert_eq!(p["seat"], 0);
        assert!(p.get("decision_seed").is_none(), "the pump's key is off the wire (finding B1): {p}");
        assert!(p.get("forced").is_none() && ok["forced"].is_boolean(), "the forced fact rides the account: {ok}");
        assert!(p["evaluator_version"].as_u64().is_some());
        let terms = ok["explanation"]["terms"].as_array().expect("terms");
        assert_eq!(terms.len(), 34, "every feature is reported, including the zero ones");
        let sum: i64 = terms.iter().map(|t| t["product"].as_i64().expect("i64")).sum();
        let terminal = ok["explanation"]["terminal"]["value"].as_i64().unwrap_or(0);
        assert_eq!(sum + terminal, ok["explanation"]["total"].as_i64().expect("a total"));
        let counters = ok["counters"].as_array().expect("a searching contestant reports counters");
        assert_eq!(counters.len(), 15, "the whole counter surface, not a chosen two of it");
        assert!(counters.iter().all(|c| c["group"] != "ungrouped"), "{counters:?}");
        assert!(!ok["continuation"].as_array().expect("a line").is_empty(), "the continuation is served");

        // A WATCHER at the same finished game: the record route refuses them,
        // and `why` serves them the withheld half — the parity (review I-1).
        let (status, refused) = get_as(&over, "watcher@wire.test", "/api/games/explains/record").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        let (status, seen) = get_as(&over, "watcher@wire.test", "/api/games/explains/why").await;
        assert_eq!(status, StatusCode::OK, "{seen}");
        assert!(seen["ok"].is_null() && seen["refusal"].is_null() && seen["withheld"].is_object(), "{seen}");
        assert_eq!(seen["withheld"]["reason"], explain::WITHHELD_REASON);
        let text = seen.to_string();
        for key in ["explanation", "ranking", "counters", "continuation", "forced"] {
            assert!(!text.contains(&format!("\"{key}\"")), "{key} reached a watcher at game over: {text}");
        }

        // The bot's FIRST move, decided while its card was hidden — the card
        // still in its hand now — served to the seat holder.
        let mut first = None;
        for n in 0..40usize {
            let (_, b) = get_as(&over, "blue@wire.test", &format!("/api/games/explains/why?event={n}")).await;
            assert!(b["withheld"].is_null(), "event {n} withheld from a seat holder at a finished game: {b}");
            if b["ok"].is_object() {
                first = Some((n, b));
                break;
            }
        }
        let (n, b) = first.expect("a bot move within the first 40 events");
        assert_eq!(b["ok"]["provenance"]["index"], n);
    }
```

(d) The pre-existing parity cell `why_reads_the_same_to_a_watcher_as_to_a_seated_caller` (Task 1's review, M-2):
its doc named the removed mechanism ("the route already masks the roads not taken for EVERY caller"); it now
states the requester-visibility rule, says the two readers part at a finished game, and asserts the half it
compares at that position is `withheld`.

(c) The three whole-payload cells named in Step 8: in each `serde_json::json!({ "ok": null, "refusal": { "kind":
"foreign_bot", … } })` literal (two in `api.rs`, one in `games.rs`), add `"withheld": null,` after `"ok": null,`.
A whole-value comparison is the right pin — it is what caught the shape change — so the literals are completed,
not loosened.

Run: `cargo test -p yokohama-web --lib 2>&1 | grep -E 'test result|FAILED|panicked'`
Expected: every lib cell PASS — 413 at the base, `suggest_carries_…` rewritten in place, `why_accounts_…`
REPLACED by `why_is_withheld_…`, Step 5's three added: **416**.
`tests/wire_fixtures` is RED here by construction (the wire moved) until Step 10.

- [ ] **Step 10: The corpus — the why fixtures, the new one, the inversion pin, the regeneration.** In
  `crates/yokohama-web/tests/wire_fixtures.rs`'s `constructed()` (:2170–2340):

(a) Build `finished` FIRST and take `why` from it: move the block that plays the pumped game out
(`let mut finished = played; for _ in 0..200 { … }`, :2286–2293) to directly after `played.pump_bots();` (:2244),
and change the scan (:2252–2263) to iterate `finished.session.record()` instead of `record` — the
`record`/`mode` bindings read off `finished` — keeping the non-forced, three-candidate condition; then:

```rust
    // Served at the requester's visibility (security S1a, finding B1): the
    // holder of seat 0 — a SEAT HOLDER who is NOT the mover, the account's
    // mover being seat 1 — reads the account of a finished game, so it is
    // PUBLIC by the record route's own game-over condition rather than by the
    // own-seat branch (review M-1). Asserted, since a withheld `why.json` would
    // leave the full account with no payload behind it; and the mover's
    // starting order is still in its hand at game over, asserted too, so the
    // engine's view alone would have withheld it and the PUBLIC is the
    // game-over rule's. `why_withheld.json` below is the other half.
    assert_eq!(account.reconstruction.seat, PlayerId::new(1), "premise: the account's mover is seat 1");
    let holders_view = yokohama_core::game::observe(finished.session.state(), PlayerId::new(0))
        .expect("seat 0's view");
    assert!(
        holders_view.players[1].hand.contains(&yokohama_core::HandCard::HiddenStartingOrder),
        "premise: seat 1's starting order is still a masked slot in its hand at game over",
    );
    let visibility = yokohama_web::explain::visibility_of(
        finished.session.state(),
        account.reconstruction.seat,
        &[PlayerId::new(0)],
    );
    assert_eq!(visibility, yokohama_web::explain::Visibility::Public, "a finished game serves a seat holder everything");
    let why = serde_json::to_value(yokohama_web::explain::Why::account(&account, &rules, visibility))
        .expect("a serializable account");

    // The WITHHELD half: one bot turn at a vs-bot table, the person at seat 1
    // asking about the bot's last move while the bot's starting order is
    // still a masked slot in its hand — asserted, as the premise.
    let mut one_turn = build(vec![Some(PROVISIONAL_DEFAULT_BOT), None]);
    one_turn.pump_bots();
    let one_bot = BotSeats::new(vec![Some(PROVISIONAL_DEFAULT_BOT), None], BOT_SEED);
    let early = yokohama_cli::why::account_parts(
        one_turn.session.mode, one_turn.session.record(), &rules, &one_bot, None,
    )
    .unwrap_or_else(|r| panic!("the bot's first turn holds a move to account for: {r}"));
    let visibility = yokohama_web::explain::visibility_of(
        one_turn.session.state(),
        early.reconstruction.seat,
        &[PlayerId::new(1)],
    );
    assert_eq!(visibility, yokohama_web::explain::Visibility::Withheld, "after one bot turn the card is hidden");
    let why_withheld = serde_json::to_value(yokohama_web::explain::Why::account(&early, &rules, visibility))
        .expect("a serializable withheld account");
```

and add `("why_withheld".to_string(), why_withheld),` after the `why` row of the returned vec (:2328). The
terminal-explanation block keeps reading `finished` as it does. In `CONSTRUCTED_WITH_AN_AT` (:2908–2912) add
`("why_withheld", Some("withheld.provenance.contestant"), "the same contestant identity, on the withheld half"),`.

(b) The inversion pin, after `no_committed_payload_carries_the_setup_seed` (its closing brace near :3015):

```rust
/// **No integer-looking field of the three explain payloads inverts to a
/// seed** (the 2026-10-08 finding A1; R-SEC3). `suggest`'s old `stream_seed`
/// was a SplitMix64 fold — every step a bijection — so the served key, read
/// with the other components known, NAMED the setup seed; `why`'s
/// `decision_seed` was one SplitMix64 step over `bot_seed ^ event_count` and
/// named the bot's stream seed. Both are off the wire and the CLI's key is a
/// digest. This pin runs BOTH inversions over every integer-looking value of
/// `suggest.json`, `why.json` and `why_withheld.json` — a decimal string, a
/// `0x` hex string or a JSON integer — against the walk's seed and the
/// fixtures' bot seed, at every seat and every event count a capture could
/// have been taken at, and finds nothing. The value-search pin above stays
/// beside it: that one sees a VALUE, this one sees a DERIVATION.
///
/// Anti-vacuity, proven first: the old fold recomputed here from the old
/// constants inverts to `SETUP_SEED`, and one SplitMix64 step over
/// `BOT_SEED ^ 5` inverts to `BOT_SEED` — so a pin that found nothing was
/// looking with a working inverse. A SplitMix64 of its own, because the CLI's
/// is `pub(crate)`.
#[tokio::test]
async fn no_integer_looking_field_of_the_explain_payloads_inverts_to_a_seed() {
    const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
    const OLD_SALT: u64 = 0x5966_2026_0727_0001;
    fn mix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(GAMMA);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unmix(mut z: u64) -> u64 {
        z = (z ^ (z >> 31) ^ (z >> 62)).wrapping_mul(0x3196_42B2_D24D_8EC3);
        z = (z ^ (z >> 27) ^ (z >> 54)).wrapping_mul(0x96DE_1B17_3F11_9089);
        z ^ (z >> 30) ^ (z >> 60)
    }
    fn old_fold(components: &[u64]) -> u64 {
        let mut acc = OLD_SALT;
        for c in components {
            let mut step = acc ^ c;
            acc = mix(&mut step);
        }
        acc
    }
    fn unfold(key: u64, rest: &[u64]) -> u64 {
        let mut acc = key;
        for c in rest.iter().rev() {
            acc = unmix(acc).wrapping_sub(GAMMA) ^ c;
        }
        unmix(acc).wrapping_sub(GAMMA) ^ OLD_SALT
    }
    fn values_of(v: &Value, out: &mut Vec<u64>) {
        match v {
            Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    out.push(u);
                } else if let Some(i) = n.as_i64() {
                    out.push(i as u64);
                }
            }
            Value::String(s) => {
                if let Some(h) = s.strip_prefix("0x") {
                    if let Ok(u) = u64::from_str_radix(h, 16) {
                        out.push(u);
                    }
                } else if let Ok(u) = s.parse::<u64>() {
                    out.push(u);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| values_of(x, out)),
            Value::Object(o) => o.values().for_each(|x| values_of(x, out)),
            Value::Bool(_) | Value::Null => {}
        }
    }
    // The walk's record identity after the seed: 2P, start seat 0, `Keep(0)`
    // at both seats (card 1 kept everywhere), then the seat and the count.
    let rest = |seat: u64, count: u64| [2u64, 0, 1, 1, seat, count];
    assert_eq!(unfold(old_fold(&[SETUP_SEED, 2, 0, 1, 1, 0, 7]), &rest(0, 7)), SETUP_SEED, "premise: the fold inverts");
    let mut s = BOT_SEED ^ 5;
    assert_eq!(unmix(mix(&mut s)).wrapping_sub(GAMMA) ^ 5, BOT_SEED, "premise: one step inverts");

    let Corpus { payloads, .. } = corpus().await;
    let mut seen = 0usize;
    for name in ["suggest", "why", "why_withheld"] {
        let (_, value) = payloads.iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("{name} is in the corpus"));
        let mut values = Vec::new();
        values_of(value, &mut values);
        for v in values {
            seen += 1;
            for seat in 0..2u64 {
                for count in 0..=4096u64 {
                    assert_ne!(unfold(v, &rest(seat, count)), SETUP_SEED, "{name}: {v} inverts to the setup seed at seat {seat}, count {count}");
                }
            }
            for count in 0..=4096u64 {
                assert_ne!(unmix(v).wrapping_sub(GAMMA) ^ count, BOT_SEED, "{name}: {v} inverts to the bot seed at event {count}");
            }
        }
    }
    assert!(seen > 100, "vacuous: only {seen} integer-looking values were read");
}
```

(c) Regenerate ONCE and read the diff:

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures 2>&1 | tail -3` (the regeneration
run reports its own refusal, `UPDATE_WIRE_FIXTURES was set: fixtures were REWRITTEN, not checked` — expected),
then `cargo test -p yokohama-web --test wire_fixtures 2>&1 | grep -E 'test result|FAILED'` — expected green,
**17** (16 + the pin). Then `git status --short web/src/wire/ && git diff --stat -- web/src/wire/`, and READ
`git diff -- web/src/wire/suggest.json web/src/wire/why.json`: `suggest.json` loses its `stream_seed` line and
its ranking/explanation MAY move (a different key is a different belief sample — the whole-answer change is A1's
intended effect, recorded, never "noise"; measured: the suggested move itself changed, 145 lines); `why.json`
loses `decision_seed`, moves `forced` onto `ok`, and is a DIFFERENT account (from the finished game; measured:
index 105 → 152, 242 added / 1,178 removed by `git diff --numstat`); `why_withheld.json` is new; **`why_refused.json` and `why_foreign.json` move by
exactly one line each — `"withheld": null`** — because every `Why`-shaped payload carries the third half
(`why_refusals.json`, a list of `RefusalDto`, does not move). **Any other file that moved is a finding: name it
in the task report with its first differing line** (B7's changes land in Task 3's own regeneration — so here a
table-bearing fixture must NOT have moved).

- [ ] **Step 11: The client.**

(a) `web/src/types.ts`: in `Provenance` (:852–866) delete the `decision_seed` line and its doc (:860–861) and
the `forced` line and its doc (:862–863); in `SuggestOk` (:875–895) delete `stream_seed` and its doc (:889–890);
in `WhyOk` (:902–912) add, last:

```ts
  /** The engine offered exactly one move here — on the ACCOUNT, not the
   *  claim: a fact about the mover's hand, withheld with the rest. */
  forced: boolean;
```

and replace `WhyResponse` (:914–917) with:

```ts
/** The withheld half (finding B1): the move played, the claim, and the one
 *  sentence — nothing the search found, which is a function of the mover's
 *  hidden starting order until that card is public, or the game is over and
 *  the reader holds a seat. */
export interface WhyWithheld {
  provenance: Provenance;
  text: string;
  reason: string;
}

/** Exactly one of the three is non-null; `withheld` and `refusal` are ANSWERS. */
export interface WhyResponse {
  ok: WhyOk | null;
  withheld: WhyWithheld | null;
  refusal: RefusalRow | null;
}
```

(b) `web/src/wire/contract.ts`: add `WhyWithheld` to the type import list; after `import why from "./why.json";`
(:194) add `import whyWithheld from "./why_withheld.json";`; after `const _whyRefused …` (:447) add
`const _whyWithheld: Widen<WhyResponse> = whyWithheld;`; after `const _whyProvenance …` (:472) add
`const _whyWithheldHalf: Widen<WhyWithheld> = whyWithheld.withheld;` and
`const _whyWithheldProvenance: Widen<Provenance> = whyWithheld.withheld.provenance;`; add the three names to the
export list at :567–568 that carries `_suggest`, `_why`, `_whyRefused` and `_whyProvenance`. Field lists (:689–703):

```ts
const SUGGEST_OK = [
  "seat", "text", "explanation", "continuation", "ranking",
  "search_hit_ply_cap", "tied_at_top",
] as const;
const WHY_RESPONSE = ["ok", "withheld", "refusal"] as const;
const WHY_OK = [
  "provenance", "text", "explanation", "continuation", "counters",
  "search_hit_ply_cap", "ranking", "forced",
] as const;
const WHY_WITHHELD = ["provenance", "text", "reason"] as const;
```

and `PROVENANCE` without `"decision_seed", "forced"`; after `const _wo …` (:760) add
`const _ww: KeysOf<WhyWithheld, typeof WHY_WITHHELD> = WHY_WITHHELD;` **and add `_ww` to the `KeysOf` export list
at :797 beside `_wo`** (an unexported const is TS6133 under the typecheck); in the `fields` object (:823–824) add
`whyWithheld: WHY_WITHHELD,`; in the `wire` object (:954) add `whyWithheld: as<WhyResponse>(whyWithheld),`.
**`contract.test.ts`'s root checks are an explicit `it.each` table of (name, payload, list) rows — a list with no row
is checked at compile time only** (a field planted in `why_withheld.json` passed vitest and `tsc`, measured in
Task 1): after the `["why_refused.refusal", …]` row add
`["why_withheld", wire.whyWithheld, KEYS.whyResponse],` and
`["why_withheld.withheld", wire.whyWithheld.withheld, KEYS.whyWithheld],` (the object the brief elsewhere calls
`fields` is `export const KEYS`; the two "export lists" at :567–568 and :797 are `void [...]` reference lists) —
vitest then reads 1,502; **its cell "the fixtures are not degenerate…" (:827) reads `wire.why.ok!.provenance.forced`
— it becomes `wire.why.ok!.forced`** (the fact moved onto the account).

(c) `web/src/Explain.tsx`: the Stream paragraph (:349–352) becomes `<p className="meta provenance">Nothing was
applied, recorded or saved.</p>` — the key goes, the reassurance stays (the terminal's own sentence). In `WhyView`
(:356–406): BEFORE its refusal
branch (`if (!data.ok) { return data.refusal ? … : null; }` — the same three lines open `SuggestView`, so anchor
on the function; placed after it, a withheld payload — `ok` null — returns `null` before the branch is reached,
measured) add

```tsx
  if (data.withheld) {
    const w = data.withheld;
    const p = w.provenance;
    return (
      <div className="explain">
        <section>
          <h3>Seat {p.seat} played</h3>
          <p className="headline">{w.text}</p>
          <p className="meta withheld">{w.reason}</p>
        </section>
        <p className="meta provenance">
          Reconstructed by replay under this build: <code>{p.contestant}</code>, evaluator impl{" "}
          {p.evaluator_version}. Event {p.index}. A reconstruction that did not equal the recorded move
          would have refused instead of showing you this.
        </p>
      </div>
    );
  }
```

replace `{p.forced && (` with `{w.forced && (`, and in the last paragraph delete `, from decision seed{" "}
<code>{p.decision_seed}</code> — the key the bot's own pump derived` so it reads "…evaluator impl
{p.evaluator_version}. Event {p.index}. A reconstruction…".

(d) `web/src/Explain.test.tsx`: replace :128–129 with `expect(screen.queryByText(/Stream/)).toBeNull();`;
rename the why test (:145) to `"names the contestant and the event it reconstructed, and no key"`, replace
:149 with `expect(screen.queryByText(/decision seed/)).toBeNull();`; the refusal-rendering cell at :208
hand-builds a `WhyResponse` — `data={{ ok: null, refusal }}` becomes `data={{ ok: null, withheld: null, refusal }}`
(TS2741 otherwise); and add after the renamed test:

```tsx
  it("shows a withheld account as the move, the reason and the claim — and no chart", () => {
    const w = wire.whyWithheld.withheld!;
    render(<WhyView data={wire.whyWithheld} labels={labels} />);
    expect(screen.getByText(w.text)).toBeInTheDocument();
    expect(screen.getByText(w.reason)).toBeInTheDocument();
    expect(screen.getByText(w.provenance.contestant)).toBeInTheDocument();
    expect(screen.queryByText(/^TOTAL/)).toBeNull();
    expect(screen.queryByRole("listitem")).toBeNull();
    expect(screen.getByText(/would have refused instead of showing you this/)).toBeInTheDocument();
  });
```

Run, from `web/`: `npm run build:wasm && npm run typecheck && npm test -- --run 2>&1 | tail -8` (the typecheck's
fourth project needs the wasm module — the lane's own order, §2; `YOKOHAMA_WASM_OPTIONAL=1` reads the three TS
projects alone).
Expected: the typecheck clean (the contract's `KeysOf` lines are what fail on a field list that drifted from
`types.ts`); vitest green, **1,502** in 71 files (cycle F's 1,499 + the withheld cell + the two contract rows).

- [ ] **Step 12: Lint, the suites, commit.**

Run: `cargo clippy --workspace --all-targets -- -D warnings` — clean. `cargo test -p yokohama-cli` and
`cargo test -p yokohama-web` — green, the counts recorded. `cargo test -p yokohama-wasm` and
`cargo test -p yokohama-worker` — green: the Worker's pool replays `suggest`'s row from the regenerated fixture
and its `why` reads are constructed, so nothing there moves but through the layer it links; the `wasm32` check
and both wasm builds (`docs/web-lane.md` §2's rows) — clean, since the CLI and the layer moved.

**On the corpus:** this is the cycle's FIRST of two regenerations (the other is Task 3's); a table-bearing
fixture that moved HERE is a finding, as Step 10(c) says. **Both hosts' walk replays enumerate the constructed
fixtures by name** — the Worker pool's `worker/test/corpus.test.ts` (`THIRD_COLUMN`) and the browser half's
`web/src/local/corpus.node.test.ts` (`NOT_SERVED`) — and refuse a file the lists do not name ("the files the
scripts do not name: expected […(18)] to deeply equal […(17)]", measured): each gains `"why_withheld"` beside
`"why_refused"`. With that, `cd worker && npm run build:module:debug && npm test` (224) and `cd web && npm run
build:wasm && npm run test:wasm` (353) read green.

```bash
git add crates/yokohama-cli/Cargo.toml Cargo.lock crates/yokohama-cli/src/suggest.rs \
  docs/cli-guide.md docs/cli-guide.html \
  crates/yokohama-web/src/explain.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/games.rs \
  crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire web/src/types.ts \
  web/src/Explain.tsx web/src/Explain.test.tsx worker/test/corpus.test.ts web/src/local/corpus.node.test.ts
git commit -m "web: the explain surface at the requester's visibility — the stream key one-way at the source and off the wire, why withheld while the bot's starting order is hidden, the corpus regenerated once (A1, B1; R-SEC3)"
```

(`Cargo.lock` moves by the CLI's new `sha2` edge only; read `git diff Cargo.lock` before staging it — one
dependency line added under `yokohama-cli`, no version moved.)

---

### Task 2: An undo that un-draws a hidden deck card is refused at a shared table — Opus (B2; R-SEC8(a); Review Focus 4)

**Files:**
- Modify: `crates/yokohama-web/src/api.rs` (`undo_allowed` :719–768, `trailing_mover` :797–813, the doc at :700–718)
- Test: `crates/yokohama-web/src/lib.rs` `mod tests`, after `undo_at_a_table_with_one_person_unwinds_the_bot_run_as_before`

**Interfaces:**
- Consumes: `yokohama_core::state_at`, `GameState.order_deck` / `tech_deck` (`Vec`s, their `len()` the deck's
  height — the public fact the game payload and the labs panel already serve as `deck_remaining`); the `/areas`
  panel (`rows[][].tile.area_type`, `tile.area` — how the drive finds the Port); the grammar's direct forms `b A<nn>`
  (Option B) and `r` (retreat, no recovery), `grammar.rs:177–186`; the helpers `start_as`, `get_as`, `post_as`,
  `first_option`, `start`, `call` of `lib.rs`'s tests.
- Produces: `pub(crate) const UNDO_DREW: &str` (api.rs); `undo_allowed` refusing with it; `trailing_mover`
  replaced by `trailing_state(record, rules) -> Result<GameState, UndoRefusal>`.

- [ ] **Step 1: The failing cell.** In `lib.rs`'s `mod tests`, after
  `undo_at_a_table_with_one_person_unwinds_the_bot_run_as_before` (:3189, its closing brace about forty lines
  on), add:

```rust
    /// Drive `user`'s turn at `id` through a move that DRAWS from a hidden
    /// deck, at the area `area_type` names, by the first row that starts
    /// `take`: `("PortA", "Port: take")` draws from the order deck and
    /// `("LaboratoryA", "Laboratory: acquire")` from the tech deck. Option B
    /// puts two assistants on that area (its id read off the `/areas` panel's
    /// `tile.area_type`), the President follows them, and the area action is
    /// the `take` row — a port take refills the port from the order deck and
    /// a lab acquire the lab from the tech deck, inside the move
    /// (`cards.rs`), and either at power 3 leaves the turn running (the
    /// 5-Power, building and additional prompts follow). Every other prompt
    /// takes its `Skip` row or its first; a Movement prompt whose only row is
    /// the retreat picker — the area holds the other President — is answered
    /// `r`, the grammar's direct retreat. Answers the deck heights BEFORE the
    /// drawing move and the reply, or `None` when the turn passed or the
    /// bound ran out with nothing drawn.
    async fn play_until_a_draw(
        state: &Arc<AppState>,
        id: &str,
        user: &str,
        (area_type, take): (&str, &str),
    ) -> Option<((usize, usize), serde_json::Value)> {
        let heights = |game: &serde_json::Value, labs: &serde_json::Value| {
            (
                game["deck_remaining"].as_u64().expect("the order deck's height") as usize,
                labs["deck_remaining"].as_u64().expect("the tech deck's height") as usize,
            )
        };
        let (_, areas) = get_as(state, user, &format!("/api/games/{id}/areas")).await;
        let area = areas["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row.as_array())
            .flatten()
            .find(|t| t["tile"]["area_type"] == area_type)
            .and_then(|t| t["tile"]["area"].as_u64())
            .map(|n| format!("A{n:02}"))
            .unwrap_or_else(|| panic!("a 2P board has a {area_type}"));
        let (_, start) = get_as(state, user, &format!("/api/games/{id}")).await;
        let mine = start["to_move"].as_u64().expect("a seat to move");
        for _ in 0..60 {
            let (_, now) = get_as(state, user, &format!("/api/games/{id}")).await;
            if now["to_move"].as_u64() != Some(mine) || now["over"] == serde_json::json!(true) {
                return None;
            }
            let (_, labs) = get_as(state, user, &format!("/api/games/{id}/labs")).await;
            let before = heights(&now, &labs);
            let rows = now["prompt"]["rows"].as_array().cloned().unwrap_or_default();
            let words = |r: &serde_json::Value| r["words"].as_str().unwrap_or_default().to_string();
            let number = |r: &serde_json::Value| r["number"].as_u64().map(|n| n.to_string());
            let line = if rows.iter().any(|r| words(r).starts_with("Option B")) {
                format!("b {area}")
            } else if let Some(r) = rows.iter().find(|r| {
                let w = words(r);
                w.starts_with(take) || (w.starts_with("Place President") && w.contains(&area))
            }) {
                number(r).expect("a numbered row")
            } else if !rows.is_empty() && rows.iter().all(|r| words(r).starts_with("Retreat President")) {
                "r".to_string()
            } else {
                rows.iter()
                    .find(|r| words(r).starts_with("Skip"))
                    .or(rows.first())
                    .and_then(number)
                    .unwrap_or_else(|| "confirm".to_string())
            };
            let (status, reply) = post_as(
                state,
                user,
                &format!("/api/games/{id}/input"),
                serde_json::json!({ "line": line }),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{line}: {reply}");
            let (_, labs) = get_as(state, user, &format!("/api/games/{id}/labs")).await;
            let after = heights(&reply, &labs);
            if after.0 < before.0 || after.1 < before.1 {
                return Some((before, reply));
            }
        }
        None
    }

    /// **An undo whose unit DREW from a hidden deck is refused at a shared
    /// table** (R-SEC8(a); finding B2): a port take or a lab acquire refills
    /// the board from the order or tech deck inside the move, so the mover
    /// read the deck's top — and an undo would put the card back where the
    /// other person never saw it. The sentence is `api::UNDO_DREW`; the
    /// record is unmoved by the refusal. The OTHER person's undo at the same
    /// moment hears the turn test's sentence instead, which pins the deck
    /// comparison AFTER the turn test. Beside it, the two things the rule
    /// must NOT touch (Review Focus 4): a non-drawing move at the same shared
    /// table still undoes, and a person alone at the table — one person
    /// holding both seats — undoes a drawing move exactly as before. The next
    /// two cells hold the lab's draw, from the tech deck, and the third table
    /// shape, one person beside a bot, on a drawing move.
    #[tokio::test]
    async fn an_undo_that_un_draws_a_deck_card_is_refused_at_a_shared_table_and_allowed_alone() {
        // The shared table: red and blue, two people. Red's FIRST move ends
        // the additional-action phase — nothing drawn — and undoes.
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "undraw", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        let (_, fresh) = get_as(&state, "red@wire.test", "/api/games/undraw").await;
        let n0 = fresh["events"].as_u64().unwrap();
        let line = first_option(&fresh);
        let (status, after) = post_as(&state, "red@wire.test", "/api/games/undraw/input", serde_json::json!({ "line": line })).await;
        assert_eq!(status, StatusCode::OK, "{after}");
        assert_eq!(after["deck_remaining"], fresh["deck_remaining"], "premise: ending the additional-action phase draws nothing");
        let (status, undone) = post_as(&state, "red@wire.test", "/api/games/undraw/input", serde_json::json!({ "line": "undo" })).await;
        assert_eq!(status, StatusCode::OK, "a non-drawing move still undoes at a shared table: {undone}");
        assert_eq!(undone["events"].as_u64().unwrap(), n0);

        // Play on to a DRAWING move in some turn of either person — the
        // first person to draw is the one whose undo is refused.
        let mut drawn = None;
        for _ in 0..12 {
            let (_, now) = get_as(&state, "red@wire.test", "/api/games/undraw").await;
            let mover = if now["to_move"].as_u64() == Some(0) { "red@wire.test" } else { "blue@wire.test" };
            if let Some(found) = play_until_a_draw(&state, "undraw", mover, ("PortA", "Port: take")).await {
                drawn = Some((mover, found));
                break;
            }
        }
        let (mover, (before, reply)) = drawn.expect("some turn in twelve held a port take");
        let events = reply["events"].as_u64().expect("an event count");
        assert!(reply["your_turn"] == serde_json::json!(true), "premise: the draw did not end the turn: {reply}");
        // The comparison's PLACE, by a two-defect probe: the OTHER person's
        // undo meets a unit that drew AND a caller who is not the one to move,
        // and hears the turn test's sentence — the comparison runs after it.
        let other = if mover == "red@wire.test" { "blue@wire.test" } else { "red@wire.test" };
        let (status, body) = post_as(&state, other, "/api/games/undraw/input", serde_json::json!({ "line": "undo" })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "undo is not offered once your turn has passed");
        let (status, body) = post_as(&state, mover, "/api/games/undraw/input", serde_json::json!({ "line": "undo" })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], api::UNDO_DREW);
        let (_, still) = get_as(&state, mover, "/api/games/undraw").await;
        assert_eq!(still["events"].as_u64(), Some(events), "a refused undo moved the record");
        assert_eq!(still["deck_remaining"].as_u64(), Some(before.0 as u64 - 1), "premise: the port take drew exactly one card");

        // Alone: one person holding both seats draws, and undoes.
        let solo = Arc::new(AppState::in_memory());
        let router = app(Arc::clone(&solo));
        start(&router, "solo", 2, &[]).await;
        let mut drawn = None;
        for _ in 0..12 {
            if let Some(found) = play_until_a_draw(&solo, "solo", WHO, ("PortA", "Port: take")).await {
                drawn = Some(found);
                break;
            }
        }
        let (_, reply) = drawn.expect("some turn in twelve held a draw");
        let events = reply["events"].as_u64().expect("an event count");
        let (status, undone) = post_as(&solo, WHO, "/api/games/solo/input", serde_json::json!({ "line": "undo" })).await;
        assert_eq!(status, StatusCode::OK, "one person at the table keeps today's undo: {undone}");
        assert!(undone["events"].as_u64().unwrap() < events);
    }

    /// **The other deck: an undo of a LAB acquire is refused the same way**
    /// (R-SEC8(a); finding B2's named catcher — "take at a lab, read the
    /// refill, undo → 403 with the sentence"). An acquire refills the lab from
    /// the TECH deck inside the move, so this cell holds the comparison's
    /// tech-deck half; the cell above draws from the order deck only. The
    /// premise that the acquire left the order deck alone is what makes the
    /// tech-deck half the one thing that can answer for the refusal.
    #[tokio::test]
    async fn an_undo_of_a_lab_acquire_is_refused_at_a_shared_table() {
        let state = Arc::new(AppState::in_memory());
        start_as(&state, "labdraw", 20260831, &["red@wire.test", "blue@wire.test"]).await;
        let mut drawn = None;
        for _ in 0..12 {
            let (_, now) = get_as(&state, "red@wire.test", "/api/games/labdraw").await;
            let mover = if now["to_move"].as_u64() == Some(0) { "red@wire.test" } else { "blue@wire.test" };
            if let Some(found) = play_until_a_draw(&state, "labdraw", mover, ("LaboratoryA", "Laboratory: acquire")).await {
                drawn = Some((mover, found));
                break;
            }
        }
        let (mover, (before, reply)) = drawn.expect("some turn in twelve held a lab acquire");
        let events = reply["events"].as_u64().expect("an event count");
        assert!(reply["your_turn"] == serde_json::json!(true), "premise: the acquire did not end the turn: {reply}");
        let tech = |labs: &serde_json::Value| labs["deck_remaining"].as_u64().map(|d| d as usize);
        let (_, labs) = get_as(&state, mover, "/api/games/labdraw/labs").await;
        assert_eq!(tech(&labs), Some(before.1 - 1), "premise: the acquire drew one card from the tech deck");
        assert_eq!(reply["deck_remaining"].as_u64().map(|d| d as usize), Some(before.0), "premise: and none from the order deck");
        let (status, body) = post_as(&state, mover, "/api/games/labdraw/input", serde_json::json!({ "line": "undo" })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], api::UNDO_DREW);
        let (_, still) = get_as(&state, mover, "/api/games/labdraw").await;
        assert_eq!(still["events"].as_u64(), Some(events), "a refused undo moved the record");
        let (_, labs) = get_as(&state, mover, "/api/games/labdraw/labs").await;
        assert_eq!(tech(&labs), Some(before.1 - 1), "the tech deck is still one short");
    }

    /// **One person beside a bot keeps today's undo of a DRAWING move** — the
    /// third table shape, which `api::undo_allowed`'s doc keeps by name ("one
    /// person and a bot — keeps today's undo, bot run and all"): the rule is
    /// about a card another PERSON never saw, and a bot is not one.
    /// `undo_at_a_table_with_one_person_unwinds_the_bot_run_as_before` holds
    /// this shape only on a unit that drew nothing, so a rule that refused a
    /// vs-bot player's own port take would pass it; this cell would not. The
    /// drive draws on red's first turn, before the bot has moved, so the
    /// trailing unit is red's take.
    #[tokio::test]
    async fn an_undo_of_a_port_take_is_allowed_at_a_table_of_one_person_and_a_bot() {
        let state = Arc::new(AppState::in_memory());
        start_vs_bot(&state, "undraw-bot", 20260831, "red@wire.test").await;
        let (before, reply) = play_until_a_draw(&state, "undraw-bot", "red@wire.test", ("PortA", "Port: take"))
            .await
            .expect("red's first turn holds a port take");
        let events = reply["events"].as_u64().expect("an event count");
        assert!(reply["your_turn"] == serde_json::json!(true), "premise: the draw did not end the turn: {reply}");
        assert_eq!(reply["deck_remaining"].as_u64(), Some(before.0 as u64 - 1), "premise: the port take drew exactly one card");
        let (status, undone) = post_as(&state, "red@wire.test", "/api/games/undraw-bot/input", serde_json::json!({ "line": "undo" })).await;
        assert_eq!(status, StatusCode::OK, "one person beside a bot keeps today's undo of a drawing move: {undone}");
        assert!(undone["events"].as_u64().unwrap() < events, "the take is gone: {undone}");
    }
```

(`WHO` is the identity `call` sends, `red@wire.test` — `start` seats it at both seats, so `solo` is one person
and `distinct_humans() == 1`. `post_as`/`get_as` take the state the router was built from, which is why the
solo table is driven through them with `WHO` after `start` opened it through the router.)

- [ ] **Step 2: Run it to verify it fails.**

Run: `cargo test -p yokohama-web --lib an_undo_that_un_draws 2>&1 | tail -12`
Expected: a compile error naming `api::UNDO_DREW`; with a placeholder constant it FAILS at the shared
table's `assert_eq!(status, StatusCode::FORBIDDEN …)` reading 200 — the un-draw the review measured. **The drive
was measured in the before review** (the plan's first draft never drew: the layer's Placement prompt is two PICKER
rows and "Place no assistants", and a last-row walk places nothing, so no area action ever came): at seed
20260831 red's first turn goes `b A02` → "Place President from hand in Port A (A02)" → "Port: take O34", the
order deck 10 → 9 with the turn still open; the solo table draws the same way. A drive that does not draw is a
finding for the task report, never a reason to assert less.

- [ ] **Step 3: The rule.** In `api.rs`, add after `NOT_SEATED`'s neighbourhood (any `pub(crate) const` of the
  file; the `UndoRefusal` enum at :772 is the anchor — put the constant directly above it):

```rust
/// The sentence an undo whose unit DREW from a hidden deck is refused with at a
/// table of more than one person (the 2026-10-08 ruling R-SEC8(a), finding
/// B2). A port take refills the port from the order deck and a lab acquire the
/// lab from the tech deck, inside the move itself (`game/actions/cards.rs`), so
/// the mover has read the deck's top; undoing would put the card back where the
/// other seats never saw it, which physical play cannot do either.
pub(crate) const UNDO_DREW: &str =
    "undo is not offered on a move that drew from a deck — the card it showed cannot be un-seen at a shared table";
```

Replace `trailing_mover` (:797–813) with the state it reads the mover off, and read both facts from it in
`undo_allowed`; **`api::tests::undo_on_a_record_that_will_not_replay_is_a_500_not_a_403` (:3391) calls
`trailing_mover(&record, &rules).expect_err(..)` — its call becomes `trailing_state(..)`** (the same `Replay`
sentence; `GameState: Debug`):

```rust
/// The state BEFORE `record`'s last event, replayed under `rules` — whose
/// `current_player` is who made that event (see [`undo_allowed`] on why by
/// replay), and whose decks are what the event drew from.
fn trailing_state(
    record: &yokohama_core::GameRecord,
    rules: &Rules,
) -> Result<yokohama_core::GameState, UndoRefusal> {
    // Not R-P6 speaking: the guard that keeps `len - 1` from underflowing, on
    // a game whose first prompt has not been answered yet. `Session::undo`'s
    // own words for the same condition, so the answer reads the same whichever
    // of the two paths reaches it.
    let Some(last) = record.events.len().checked_sub(1) else {
        return Err(UndoRefusal::Refused("nothing to undo".to_string()));
    };
    yokohama_core::state_at(record, last, rules).map_err(|e| UndoRefusal::Replay(e.to_string()))
}
```

and in `undo_allowed`, replace the tail (:762–767) with:

```rust
    let before = trailing_state(repl.session.record(), repl.session.rules())?;
    let now = repl.session.state();
    if before.current_player != viewer || now.current_player != viewer {
        return Err(UndoRefusal::Refused("undo is not offered once your turn has passed".to_string()));
    }
    // **A unit that DREW from a hidden deck stays** (R-SEC8(a), finding B2):
    // the decks of the state before the trailing event against the decks now
    // — shorter now means the event drew, and the mover has seen the card.
    // The unit `Session::undo` pops is that event with its bound reveals; a
    // record this layer deals carries no reveals (the grammar refuses
    // `reveal`, and the CLI captures bound reveals only in live mode); one
    // that arrives from elsewhere — a backup restored in the browser, or a
    // file placed in a store — can end on a standalone starting-order reveal,
    // which can take its card out of the order deck (`extract_order_from_deck`
    // searches it first) and so reads here as a draw: refused, the safe side.
    // AFTER the turn test, so a passed turn keeps its own sentence (R-P6).
    if before.order_deck.len() > now.order_deck.len() || before.tech_deck.len() > now.tech_deck.len() {
        return Err(UndoRefusal::Refused(UNDO_DREW.to_string()));
    }
    Ok(())
```

`UndoRefusal::Refused`'s doc (:773–777) lists the refusals it carries — it gains "or the un-draw rule (R-SEC8(a),
[`UNDO_DREW`])", and "exactly as every one of them has always answered" is qualified, since the new one has not
(Task 2's review, M-4). Add to `undo_allowed`'s doc (:700–718) one paragraph: "**And at a table of more than one person, a unit that
drew from a hidden deck is not rewound** (R-SEC8(a), 2026-10-08): the trailing state's decks against the
current state's, refused with [`UNDO_DREW`]. A solo table — one person, or one person and a bot — keeps today's
undo, bot run and all: the rule is about a card another PERSON never saw."

- [ ] **Step 4: Run the cell and the undo cells.**

Run: `cargo test -p yokohama-web --lib undo 2>&1 | grep -E 'test |test result'`
Expected: every undo cell PASS (10), the new one included. Try the mutant before the reviewer does: **M4** — the
deck comparison deleted: the new cell RED at the shared table (`200` against `403`), every other cell of the lib
green (measured: 416 green beside it). Two more mutants, from Task 2's report, each with a catcher the cell now
carries: **M-tech** — the tech-deck comparison alone deleted: the lab arm RED (the spec's own named catcher, a take
at a lab); **M-order** — the comparison moved ABOVE the turn test: the other person's undo after the draw reads
`UNDO_DREW` where R-P6's sentence is owed, RED. Record the three readings.

- [ ] **Step 5: Lint and commit.**

Run: `cargo clippy -p yokohama-web --all-targets -- -D warnings` — clean.

```bash
git add crates/yokohama-web/src/api.rs crates/yokohama-web/src/lib.rs
git commit -m "web: an undo whose unit drew from a hidden deck is refused at a shared table (B2; R-SEC8(a))"
```

---

### Task 3: `invited` names a seat the creator bound, never a joiner — Opus (B7; Review Focus 6)

**Files:**
- Modify: `crates/yokohama-web/src/table.rs` (`SeatDoc::Human` :46–55; `SeatView::of` :500–510 and the doc at
  :476–485; every `SeatDoc::Human { .. }` site — 15 in this file, the module's cells included)
- Modify: `crates/yokohama-web/src/tables.rs` (`config_of` :255; `join` :1603; 14 sites), `api.rs` (4 sites),
  `backup.rs` (1), `archive.rs` (2), `client.rs` (1), `bots.rs` (1), `games.rs` (1),
  `crates/yokohama-worker/src/house.rs:141` (a pattern)
- Regenerate: `web/src/wire/table_foreign.json` (one line) and `web/src/wire-script/archive-walk.json` (the walk's
  stored table document, in every archive body) — the cycle's SECOND regeneration, read (Step 5 says why those
  two and no other)
- Test: `table.rs`'s `mod tests`; `lib.rs` route cells; `wire_fixtures`

**Interfaces:**
- Consumes: `is_false` (`table.rs:138`, the `skip_serializing_if` helper `TurnMirror` uses).
- Produces: `SeatDoc::Human { user, name, bound_at_create }`.

- [ ] **Step 1: The failing cells.** In `table.rs`'s `mod tests` (:632), after
  `the_view_names_no_address_but_the_readers_own` (:1052–1109, the `invited` cell; find by
  `"the address the creator typed"`), add:

```rust
    /// **`invited` is the address the creator TYPED, and only that** (finding
    /// B7): a joiner's seat is stored under the joiner's own key the same way,
    /// and until this cycle a joiner who gave no name was shown to the creator
    /// by address — in `…/table`, the lobby row, the deal and every keep
    /// reply. Three seats, one reader (the creator): the seat they typed for
    /// somebody else shows its address; the seat they typed for THEMSELVES
    /// shows their own (Review Focus 6); the joined seat shows `null`.
    #[test]
    fn invited_names_a_creator_bound_seat_and_never_a_joiner() {
        let creator = User::parse("alice@t.test").unwrap();
        let mut doc = open_doc();
        doc.created_by = "alice@t.test".into();
        doc.seats = vec![
            SeatDoc::Human { user: "alice@t.test".into(), name: None, bound_at_create: true },
            SeatDoc::Human { user: "carol.secret@t.test".into(), name: None, bound_at_create: false },
            SeatDoc::Human { user: "typed@t.test".into(), name: None, bound_at_create: true },
        ];
        let view = doc.view(&creator, false);
        let invited: Vec<Option<String>> = view
            .seats
            .iter()
            .map(|s| match s {
                SeatView::Human { invited, .. } => invited.clone(),
                _ => None,
            })
            .collect();
        assert_eq!(
            invited,
            vec![Some("alice@t.test".into()), None, Some("typed@t.test".into())],
            "the creator's own typed seat and the seat they typed for another; never the joiner's",
        );
        // The document's bytes: a joined seat serializes as it always did —
        // the field is skipped when false — and a creator-bound seat names it.
        let json = serde_json::to_string(&doc.seats[1]).unwrap();
        assert!(!json.contains("bound_at_create"), "a joined seat's bytes are unchanged: {json}");
        let json = serde_json::to_string(&doc.seats[0]).unwrap();
        assert!(json.contains("\"bound_at_create\":true"), "{json}");
        // And a document written before the field reads every seat as joined.
        let old: SeatDoc = serde_json::from_str(r#"{"kind":"human","user":"x@t.test"}"#).unwrap();
        assert!(matches!(old, SeatDoc::Human { bound_at_create: false, .. }));
    }
```

(`open_doc()` is the module's own document builder, the one `the_view_names_no_address_but_the_readers_own`
at :1052 starts from; `User` is in the module's scope as that cell uses it.)

- [ ] **Step 2: Run it to verify it fails to compile.**

Run: `cargo test -p yokohama-web --lib table::tests::invited_names 2>&1 | grep -E '^error' | head -3`
Expected: `SeatDoc::Human` has no field named `bound_at_create`.

- [ ] **Step 3: The field, its writers, its reader.** In `table.rs`'s `SeatDoc::Human` (:47–55), after `name`:

```rust
        /// Whether the CREATOR bound this seat at create — typing an address
        /// for it — rather than a joiner taking an open seat (security S1a,
        /// finding B7). Only such a seat may show its address back to the
        /// creator as `invited` ([`SeatView`]): a joiner's key is the joiner's
        /// own, and the creator has no claim on it. Serde-default `false` and
        /// skipped when false, so a document written before the field reads
        /// every seat as joined and a joined seat's bytes are what they were;
        /// `TABLE_SCHEMA` stays 3.
        #[serde(default, skip_serializing_if = "is_false")]
        bound_at_create: bool,
```

In `SeatView::of` (:500–510):

```rust
            SeatDoc::Human { user, name, bound_at_create } => SeatView::Human {
                name: name.clone(),
                you: user == reader.as_str(),
                invited: (mine && name.is_none() && *bound_at_create).then(|| user.clone()),
            },
```

and in the `SeatView` doc (:479–483) make the `invited` line read "`invited` is the address the creator typed
at create — a seat the creator BOUND (`bound_at_create`), never a joiner's — served to the creator alone and
only while the seat has no name …". In `tables.rs`'s `config_of` (:255) the typed seat is
`SeatDoc::Human { user: user.as_str().to_string(), name: None, bound_at_create: true }`; in `join` (:1603)
`SeatDoc::Human { user: user.as_str().to_string(), name, bound_at_create: false }`. Every other site: a
PATTERN gains `..` (`SeatDoc::Human { user, name }` → `SeatDoc::Human { user, name, .. }`; `house.rs:141`'s
`SeatDoc::Human { user: _, name: _ }` → `SeatDoc::Human { user: _, name: _, bound_at_create: _ }` — NAMED, never
`..`: that match is the house scrub's guard (its doc: "every field is NAMED, with no `..` … a field added is a
compile error here (E0027)"), and `{ .. }` would disarm it for every later seat field; the decision it records
is "kept as stored", a bound flag carrying no address); a CONSTRUCTION in a cell gains
`bound_at_create: false` unless the cell is about the creator's typed seat — then `true`: the THREE constructions
of `the_view_names_no_address_but_the_readers_own` (red's own and blue's at its opening, both typed by red, and
red's own seat re-bound with a name at `:1104` — with `false` there the cell's "a named seat is its name, even to
the creator" holds for the wrong reason and a name-arm mutant passes; measured) and the creator's seats
`client.rs:829` builds (a test helper, `#[cfg(test)]` at :803). Measured: 41 occurrences, **28 edits** (the four
named above, the house pattern, 20 single-line constructions, one multi-line in `games.rs:3615`, the two patterns
`table.rs:426` and `tables.rs:307`), 13 already carrying `..`. `is_false`'s doc (`table.rs:138`) names
`TurnMirror::has_events` alone — it gains `SeatDoc::Human::bound_at_create`. The schema-1 convention cell
`a_schema_one_document_reads_with_every_new_field_at_its_default` (`table.rs:844–856`) names every field added
since schema 1 and holds each at its default — its `matches!` gains `bound_at_create: false` (and its doc a line),
so a serde default inverted to `true` reds it (Task 3's review, W3).
`cargo check -p yokohama-web --all-targets && cargo check -p yokohama-worker --all-targets` is the list of sites:
fix until clean, and name the count in the task report.

- [ ] **Step 4: The route cell and the pin.** In `lib.rs`'s `mod tests`, beside the seats cells (near
  `a_plain_name_seated_at_an_edge_door_is_a_key_nobody_holds`, :1874):

```rust
    /// **A nameless joiner's address is served to nobody** (finding B7): the
    /// creator's `…/table`, their lobby row (`?scope=mine`) and their deal all
    /// read `invited: null` for the joined seat, while the creator's own seat
    /// — typed at create — still names itself to them. No corpus payload
    /// carries this case — the walk's joiner gives a name — and the leakage
    /// pin in `tests/wire_fixtures.rs` cannot tell a typed seat from a joined
    /// one; the corpus golden catches a view that ignores the field, through
    /// `table_foreign.json`'s seat 0.
    #[tokio::test]
    async fn a_nameless_joiners_address_is_served_to_nobody() {
        let state = Arc::new(AppState::in_memory());
        let (status, created) = post_as(
            &state,
            "alice@wire.test",
            "/api/games",
            serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seats": [{ "user": "alice@wire.test" }, { "open": true }], "id": "nameless", "name": "nameless",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{created}");
        let (status, joined) = post_as(&state, "carol.secret@wire.test", "/api/games/nameless/join", serde_json::json!({ "seat": 1 })).await;
        assert_eq!(status, StatusCode::OK, "{joined}");
        for uri in ["/api/games/nameless/table", "/api/games?scope=mine", "/api/games/nameless/deal"] {
            let (status, body) = get_as(&state, "alice@wire.test", uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}: {body}");
            let text = body.to_string();
            assert!(!text.contains("carol.secret"), "{uri} serves the joiner's address to the creator: {text}");
        }
        let (_, table) = get_as(&state, "alice@wire.test", "/api/games/nameless/table").await;
        assert_eq!(table["seats"][0]["invited"], "alice@wire.test", "the creator's own typed seat still names itself");
        assert_eq!(table["seats"][1]["invited"], serde_json::Value::Null);
    }
```

**The leakage pin in `tests/wire_fixtures.rs` is NOT tightened** (the first draft added an arm reading an
`invited` equal to `OTHER` as a joiner's): payload text cannot tell a seat the creator TYPED from one that was
JOINED — the stored-table pin's own `addressed` table binds `OTHER` at create and legitimately serves
`invited: "blue@wire.test"`, and the arm made it an offender (measured). The walk's joiner joins WITH a name
(`"name": "Blue"`, :923), so no walk payload has ever carried the B7 case; the catchers are the unit cell above
(three shapes) and this route cell (a nameless joiner), with the corpus golden holding Step 5's one moved line.
The pin's own "What the rule trusts" paragraph (`wire_fixtures.rs:3196–3202`) names `mine` alone — it gains the
sentence: a joiner's address served as `invited` passes this pin; its catchers are
`table::tests::invited_names_a_creator_bound_seat_and_never_a_joiner`,
`tests::a_nameless_joiners_address_is_served_to_nobody` and the golden through `table_foreign.json`'s seat 0
(Task 3's review, Minor 2). The pin's CODE is untouched.

- [ ] **Step 5: Regenerate the corpus — the cycle's SECOND regeneration — and read it.**

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures 2>&1 | tail -3`, then
`cargo test -p yokohama-web --test wire_fixtures 2>&1 | grep -E 'test result|FAILED'` — green, 17. Then
`git status --short web/src/wire web/src/wire-script` and READ the diff. **Exactly two files move, measured in the
before review:** (1) `web/src/wire/table_foreign.json`, ONE line — seat 0's `"invited": "local@browser"` becomes
`null`, because `foreign_pair` loads a hand-written table DOCUMENT without the field (`load_document`), which the
compatibility rule reads as a joined seat (a table stored before this cycle stops echoing the creator's typed
addresses for still-nameless seats, once — the default this plan chose); (2) `web/src/wire-script/archive-walk.json`
— the archive walk's bodies carry the walk's STORED table document as a string, and the creator's seat (red,
typed at create) gains `"bound_at_create": true` in each; blue's seat, joined, carries no field. The five
fixtures with a non-null `invited` (`deal_bot_table`, `games`, `table_bot`, `table_foreign`, `table_two_open`)
all carry a CREATOR's own typed seat, which `config_of` binds `true` — the first four do not move. **Any other
changed line is a finding** for the task report. The Worker's pool replays the walk and compares the same
payloads: `cd worker && npm run build:module:debug && npm test` green.

- [ ] **Step 6: Run the suites, lint, commit.**

Run: `cargo test -p yokohama-web` and `cargo test -p yokohama-worker` green; `cargo clippy --workspace
--all-targets -- -D warnings` clean; **and the lane's rows the corpus move reaches, whole** — `cd web && npm run
build:wasm && npm run typecheck && npm test -- --run && npm run test:wasm` (the client's cells read
`table_foreign.json`) and `cd worker && npm run build:module:debug && npm run typecheck && npm run test:scripts &&
npm test` (`scripts/live-replay.test.mjs` reads the archive walk) — a subset was the plan's first draft's omission
(Task 3's review, Minor 3). Mutant **M5** — `SeatView::of` ignoring `bound_at_create`: the unit cell RED
(`Some("carol.secret@t.test")` where `None`), the route cell RED, and the corpus GOLDEN
`the_committed_wire_fixtures_are_what_the_server_emits` RED (`table_foreign.json`'s `null` against the address the
mutant serves) — three catchers, measured; nothing else.

```bash
git add crates/yokohama-web/src crates/yokohama-worker/src/house.rs web/src/wire/table_foreign.json web/src/wire-script/archive-walk.json
git commit -m "web: invited names a seat the creator bound, never a joiner — bound_at_create on the seat document, the corpus regenerated (B7)"
```

---

### Task 4: A creator's seed is honoured at a solo table only — Opus (B3; R-SEC8(b); Review Focus 5; spec correction 4)

**Ruled 2026-10-09:** the owner chose the named `SeedPolicy::Diagnostic` opt-in over the review's fallback
(honour the seed and disclose `seeded: true` on the join screen), for this cycle and the phases after it. This
task is the ruling as given.

**Files:**
- Modify: `crates/yokohama-web/src/games.rs` (`Games` :264–300 a field; `with_parts` :459–481; `with_limits`
  :421–435; two builder/reader methods beside `with_archive` :496), `crates/yokohama-web/src/api.rs` (two test-side
  `Games::with_parts` builders opt in)
- Modify: `web/e2e/the_forms.spec.ts` (:38 the widest-form cell, split) and `web/e2e/it_explains.spec.ts` (:119,
  :188 — Task 1's withheld `why` met the Playwright tier here, since the tier runs in this task; the shipped
  behaviour is what the specs now assert)
- Modify: `crates/yokohama-web/src/tables.rs` (`create` :787–800; a constant beside `UNLOCKED_CAP` :752)
- Modify: `crates/yokohama-web/src/lib.rs` (`AppState::new` :414–435 split into `new` + `production`;
  `with_store_chat_and_archive` :483–490 and `with_store_and_chat` :472–481 opt in; a builder beside `with_ttls`
  :573; cells)
- Modify: `crates/yokohama-worker/src/config.rs` (`InitConfig` :33–50, `Config` :53–58, `parse` :143–167),
  `crates/yokohama-worker/src/lib.rs` (:191–200; the test helper `door_config` :660–666; one cell)
- Modify: `crates/yokohama-wasm/src/lib.rs` (`init` :122 — the THIRD host; a new export `init_diagnostic`; the
  seeded cell `a_table_whose_watcher_list_holds_a_plain_name_reads`), `web/src/local/corpus.node.test.ts` (the
  `WasmHost` interface :56, `wasm.init()` at :243 and :335), `worker/src/marker.unit.test.ts` (:111),
  `worker/src/config.unit.test.ts` (four cells)
- Modify: `worker/src/config.ts` (`ROWS` :46–62, `onOff`'s name union :193, `ShellConfig` and `shellConfig`
  :256–275, `moduleConfig` :290–305), the `Env` declaration wrangler generates (find by
  `grep -rln YOKOHAMA_DEV_DOOR worker --include='*.d.ts'`), `worker/wrangler.test.jsonc` and
  `worker/wrangler.open.test.jsonc` (`vars`), `worker/src/env.ts` (:57), `worker/scripts/deploy-guard.mjs` (:37 the
  header list, :593 the rule beside the test clock's) and `worker/scripts/deploy-guard.test.mjs` (:262 the cell's
  twin); `crates/yokohama-worker/src/receipt.rs` (`r2_at`) and the positional-array cell (lib.rs:896)
- Modify: `docs/web-lane.md` §12 (one row, so `worker/scripts/lane-variables.test.mjs` stays green — the
  table is held to the code's `YOKOHAMA_*` names both ways); `worker/RUNBOOK.md` §3 (the variables table: the
  switch's row, the test clock's twin) and `worker/README.md` §9.3 (the guard's checks: 4b) and §9.4 (the "In
  brief" list) — the Worker's operator documents (Task 4's review, M-6)
- Modify: `crates/yokohama-web/tests/start_refusal.rs` (the NAS-path cell, Step 1 (b2))
- Modify: `web/src/NewGame.tsx` (:185–240, :388–420), `web/src/NewGame.test.tsx` (the "the seed" block, :326+; the
  gate's cell)
- Test: `tables.rs`'s `mod tests`, `lib.rs`'s route cells, the Worker crate's cells, `deploy-guard.test.mjs`,
  `lane-variables.test.mjs`, `NewGame.test.tsx`

**Interfaces:**
- Produces: `games::SeedPolicy` (no `Default` derive — nothing reads one, and a flipped attribute passed every
  suite; Task 4's review, M-3), `Games::with_seed_policy`, `Games::seed_policy`, `tables::SEED_NOT_SOLO`,
  `AppState::with_seed_policy`, `AppState::production(..)`, the Worker's `YOKOHAMA_DIAGNOSTIC_SEEDS`, the browser
  host's `init_diagnostic()`.

- [ ] **Step 1: The failing cells.**

(a) In `tables.rs`'s `mod tests` (:2033), beside `create_refuses_the_new_arms_in_order_and_an_open_seat_is_not_human` (:3798):

```rust
    /// **A creator's seed is honoured at a SOLO table only** (R-SEC8(b),
    /// finding B3): every human seat the creator's and none open. Under the
    /// production policy a seed beside an open seat or another person is a
    /// 400 in `SEED_NOT_SOLO`'s words and the same body without a seed is a
    /// 201; a seed beside a BOT is honoured, a bot being no person (Review
    /// Focus 5); and under `Diagnostic` — the fixtures' named opt-in, which
    /// `games()` sets — every shape is honoured, which is what every other
    /// cell in this module relies on. The refusal's place in `create`'s order
    /// — after `config_of`'s last, before the id's first — is pinned by two
    /// two-defect probes.
    #[test]
    fn a_creators_seed_is_honoured_at_a_solo_table_only() {
        use crate::games::SeedPolicy;
        let production = Games::new(Arc::new(Rules::standard()), Arc::new(MemStore::default()))
            .with_seed_policy(SeedPolicy::SoloOnly);
        let creator = who("red@t.test");
        // Another person.
        let err = production.create(&two_people("other", 7), &creator).expect_err("refused");
        assert!(matches!(&err, GameError::BadRequest(m) if m == SEED_NOT_SOLO), "{err:?}");
        // An open seat.
        let mut open = req_seeded("open", 7);
        open.seats[1] = SeatSpec { user: None, bot: false, open: true };
        let err = production.create(&open, &creator).expect_err("refused");
        assert!(matches!(&err, GameError::BadRequest(m) if m == SEED_NOT_SOLO), "{err:?}");
        // **The ORDER, by two-defect probes at the ADJACENT links** (the M6.2
        // lesson: a single-defect probe pins nothing about order). The seed is
        // judged after `config_of`'s last refusal, the password, and before the
        // id's first, an unusable id: wrong at the seed AND the link after, the
        // seed's sentence is the one handed back; wrong at the seed AND the link
        // before, `config_of`'s is.
        let mut after = open.clone();
        after.id = Some("bad id".into());
        let err = production.create(&after, &creator).expect_err("refused");
        assert!(matches!(&err, GameError::BadRequest(m) if m == SEED_NOT_SOLO), "the seed precedes the id: {err:?}");
        let mut before = open.clone();
        before.password = Some("   ".into());
        let err = production.create(&before, &creator).expect_err("refused");
        assert!(
            matches!(&err, GameError::BadRequest(m) if m == "a password is at least one character"),
            "config_of's last refusal precedes the seed: {err:?}"
        );
        // The other-person shape with NO seed: drawn, 201 (the open-seat shape unseeded is route cell (b)'s).
        let mut unseeded = two_people("other2", 7);
        unseeded.seed = None;
        assert!(production.create(&unseeded, &creator).is_ok(), "a drawn seed opens the table");
        // A bot is no person; a hot seat is one person.
        assert!(production.create(&req_seeded("bot", 7), &creator).is_ok(), "the bot's seat is nobody's");
        let mut hot = req_seeded("hot", 7);
        hot.seats[1] = SeatSpec { user: Some("red@t.test".into()), bot: false, open: false };
        assert!(production.create(&hot, &creator).is_ok(), "every human seat is the creator's");
        // The fixtures' opt-in: `games()` is `Diagnostic`, and the refused shape is honoured there.
        assert_eq!(games().seed_policy(), SeedPolicy::Diagnostic);
        assert!(games().create(&two_people("diag", 7), &creator).is_ok());
        // And a table opened under the refusal is NOT there: nothing was written.
        assert!(production.read_table("other").unwrap().is_none(), "a refused create wrote a table");
    }
```

(b) In `lib.rs`'s `mod tests`, beside `an_unknown_key_in_a_new_game_request_is_refused_rather_than_defaulted`
(:1922):

```rust
    /// **The production constructor's policy is `SoloOnly`** —
    /// `AppState::production`, the path `AppState::new` takes past its two
    /// environment reads, built here in process: a seeded create beside an
    /// open seat is a 400 through the router and the same without a seed a
    /// 201, so `production` never inherits a test constructor's `Diagnostic`.
    /// `new`'s own call to it — `main`'s path, which nothing in process runs —
    /// is pinned on the binary by `start_refusal.rs`'s
    /// `the_binary_refuses_a_seed_beside_an_open_seat_and_opens_the_same_table_unseeded`.
    #[tokio::test]
    async fn the_production_state_honours_a_seed_at_a_solo_table_only() {
        let state = Arc::new(
            AppState::production(
                Arc::new(MemStore::default()),
                identity::IdentityMode::Open,
                chat::ChatConfig::default(),
                Arc::new(archive::NoArchive),
                Multiplayer(true),
                sweep::Ttls::none(),
            )
            .expect("a production state over memory"),
        );
        assert_eq!(state.games.seed_policy(), games::SeedPolicy::SoloOnly);
        let body = |seed: bool| {
            let mut b = serde_json::json!({
                "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
                "seats": [{ "user": WHO }, { "open": true }], "id": "seeded", "name": "seeded",
            });
            if seed {
                b["seed"] = serde_json::json!("20260831");
            }
            b
        };
        let (status, refused) = post_as(&state, WHO, "/api/games", body(true)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        assert_eq!(refused["error"], tables::SEED_NOT_SOLO);
        let (status, opened) = post_as(&state, WHO, "/api/games", body(false)).await;
        assert_eq!(status, StatusCode::CREATED, "{opened}");
        // And the in-memory constructor every other cell uses is the fixtures' opt-in.
        assert_eq!(AppState::in_memory().games.seed_policy(), games::SeedPolicy::Diagnostic);
    }
```

(b2) In `crates/yokohama-web/tests/start_refusal.rs`, in its harness (which spawns `CARGO_BIN_EXE_yokohama-web`
with a controlled child environment) — **the NAS's own path**, `main` → `AppState::new` → `production`, which no
in-process cell runs (Task 4's review, I-1: `new`'s pre-task body through the Diagnostic wrapper passed every
suite); the harness binds port 0, so the cell picks a free port first:

```rust
/// A port nobody held a moment ago: the OS's pick for a listener bound to
/// port 0, released at once. Another process taking it in between fails the
/// start loudly — the bind's error and an exit — never silently.
#[cfg(feature = "open-door")]
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).expect("a free port").port()
}

/// One `POST` to the server at `port` as `user` at the open door (the claimed
/// header), over a plain socket — no client crate — with `Connection: close`,
/// so the response is everything until the server closes; each read and write
/// bounded by [`BOUND`]. The status, and the body parsed as JSON (`Null` when
/// it is not).
#[cfg(feature = "open-door")]
fn post(port: u16, user: &str, path: &str, body: &serde_json::Value) -> (u16, serde_json::Value) {
    use std::io::Write;
    use yokohama_web::identity::CLAIMED_HEADER;
    let body = body.to_string();
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("the server accepts");
    stream.set_read_timeout(Some(BOUND)).expect("a read bound");
    stream.set_write_timeout(Some(BOUND)).expect("a write bound");
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{CLAIMED_HEADER}: {user}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("the request writes");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("the response reads");
    let (head, rest) = response.split_once("\r\n\r\n").expect("a head and a body");
    assert!(
        !head.to_ascii_lowercase().contains("transfer-encoding: chunked"),
        "a chunked body, which this reader does not decode: {head}"
    );
    let status = head.split(' ').nth(1).and_then(|s| s.parse().ok()).expect("a status line");
    (status, serde_json::from_str(rest).unwrap_or(serde_json::Value::Null))
}

/// **The NAS's own path builds the production seed rule** (security S1a,
/// finding B3; R-SEC8(b); the review's I-1). The binary `main` runs —
/// `AppState::new`, which hands its file store to `AppState::production` —
/// refuses a creator's seed beside an open seat with `SEED_NOT_SOLO`'s
/// sentence and stores no table, and opens the same body unseeded with a 201.
///
/// **A cell on the wrong host is not coverage** (the M6.4 lesson):
/// `the_production_state_honours_a_seed_at_a_solo_table_only` pins
/// `production` in process, but nothing in process runs `main` → `new` →
/// `production`. `new` rebuilt through a test constructor — its pre-task body,
/// `with_store_chat_and_archive`, now the `Diagnostic` wrapper — left every
/// other suite green (the review's W13); here it reads 201 where 400. The
/// open door, so a claimed plain name is the creator; a port picked free,
/// because this cell must reach the server; the store read back by the
/// layer's own `FsStore` over the directory the child was given.
#[cfg(feature = "open-door")]
#[test]
fn the_binary_refuses_a_seed_beside_an_open_seat_and_opens_the_same_table_unseeded() {
    use yokohama_web::store::{Doc, FsStore, Store};
    use yokohama_web::tables::SEED_NOT_SOLO;

    let scratch = Scratch::new("seed-open");
    let port = free_port();
    let mut server = Server::spawn_on("open", &scratch, port);
    let line = server.listening("the seed rule at the open door");
    assert!(line.ends_with(&format!(":{port}")), "listening where it was told to: {line:?}");
    let store = FsStore::new(scratch.root.join("games"));
    let body = |seed: bool| {
        let mut b = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
            "seats": [{ "user": "red" }, { "open": true }], "id": "seeded", "name": "seeded",
        });
        if seed {
            b["seed"] = serde_json::json!("20260831");
        }
        b
    };
    let (status, refused) = post(port, "red", "/api/games", &body(true));
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["error"], SEED_NOT_SOLO, "the layer's sentence: {refused}");
    assert!(store.read(Doc::Table, "seeded").expect("the store reads").is_none(), "a refused create stored a table");
    let (status, opened) = post(port, "red", "/api/games", &body(false));
    assert_eq!(status, 201, "{opened}");
    // The anti-vacuity partner: the store read above is the one the server writes.
    assert!(store.read(Doc::Table, "seeded").expect("the store reads").is_some(), "the 201's table is in the store");
    assert!(server.running(), "still serving after both requests");
    let (status, _, stderr) = server.finish();
    assert_ne!(status.code(), Some(1), "killed, not refused; stderr {stderr:?}");
}
```

(c) In `crates/yokohama-worker/src/lib.rs`'s `mod tests`, beside the cell that uses `CREATE` (:1084):

```rust
    /// **The Worker honours a seed beside an open seat only under
    /// `YOKOHAMA_DIAGNOSTIC_SEEDS`** (R-SEC8(b); the pool's test files set it,
    /// no deployable file may — the deploy guard's rule): without the switch
    /// — `false`, or the key OMITTED, the fail-safe for a shell that sends
    /// none — the corpus's own create body is a 400 in the layer's sentence;
    /// with it, a 201.
    #[test]
    fn a_seeded_create_beside_an_open_seat_needs_the_diagnostic_switch() {
        let _s = serial();
        let omitted = r#"{"identity":"worker","archive":"off","caller":null,"lobby_chat":null,"chat_keep":null,"chat_text":null}"#;
        for (what, configuration) in [("false", door_config_with_seeds("off", "worker", false)), ("omitted", omitted.to_string())] {
            init_inner(&configuration).unwrap();
            let r = call("red@t.test", "POST", "/api/games", Some(CREATE));
            assert_eq!(r.status, 400, "the key {what}: {}", r.body);
            assert!(r.body.contains(yokohama_web::tables::SEED_NOT_SOLO), "the key {what}: {}", r.body);
            assert!(stored(Doc::Table).is_empty(), "the key {what}: a refused create wrote a table");
        }
        init_inner(&config("off")).unwrap();
        let r = call("red@t.test", "POST", "/api/games", Some(CREATE));
        assert_eq!(r.status, 201, "{}", r.body);
    }
```

(c2) In `crates/yokohama-wasm/src/lib.rs`'s `mod tests`, beside the watcher-list cell — the browser host's twin of (c),
pinning `init()`'s own rule (a Diagnostic mutant on `init()` survived the crate's 9 cells and `test:wasm`'s 353 until
this cell; Task 4's report, C4):

```rust
    /// **This host builds the production seed rule** (security S1a, finding
    /// B3; R-SEC8(b)) — the Worker switch's cell, on the browser host: under
    /// [`init`] a seeded create that seats another person is the layer's 400 in
    /// `SEED_NOT_SOLO`'s words and stores no table; under [`init_diagnostic`],
    /// the harness's opt-in by name, the same body is a 201.
    #[test]
    fn init_refuses_a_seed_beside_another_person_and_init_diagnostic_honours_it() {
        let _s = serial();
        let body = br#"{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"user":"blue@t.test"}],"id":"seeded"}"#;
        init();
        let (status, refused) = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", Some("application/json"), body);
        assert_eq!(status, 400, "{refused}");
        assert!(refused.contains(yokohama_web::tables::SEED_NOT_SOLO), "{refused}");
        assert_eq!(read_inner("table", "seeded").unwrap(), None, "a refused create stored a table");
        init_diagnostic();
        let (status, created) = dispatch_inner(Some("red@t.test"), "POST", "/api/games", "", Some("application/json"), body);
        assert_eq!(status, 201, "{created}");
    }
```

(d) In `worker/scripts/deploy-guard.test.mjs`, the twin of the test-clock cell (:262–265):

```js
  it("refuses a deployed configuration that names YOKOHAMA_DIAGNOSTIC_SEEDS", () => {
    const seeded = PRODUCTION.replace('"main"', '"vars": { "YOKOHAMA_DIAGNOSTIC_SEEDS": "on" },\n  "main"');
    expect(deployGuard(worker({ "wrangler.jsonc": seeded }))).toEqual([
      expect.stringMatching(/^wrangler\.jsonc names YOKOHAMA_DIAGNOSTIC_SEEDS/),
    ]);
  });
```

and the open-door clock cell at :614 gains the seeded twin of its clock line (rule 4b reaches the open and gate
files too; Task 4's review, M-4):

```js
    const seeded = OPEN.replace('"main"', '"vars": { "YOKOHAMA_DIAGNOSTIC_SEEDS": "on" },\n  "main"');
    expect(at(seeded)).toEqual(naming(/^wrangler\.open\.jsonc names YOKOHAMA_DIAGNOSTIC_SEEDS/));
```

(`deployGuard(worker({ … }))` is what the clock cell at :262 calls to judge a configuration's text, and `toEqual([…])`
its exact twin — the seeded fixture yields exactly one failure, measured; `toContainEqual` would let a spurious
second failure pass.)

(e) In `web/src/NewGame.test.tsx`'s `describe("the seed", …)` (:327):

```tsx
  it("hides the seed, and sends none, once a seat is another person or open", async () => {
    const { stub, user } = show();
    await user.click(screen.getByText("Advanced", { selector: "summary" }));
    await user.type(screen.getByLabelText("Seed"), "12");
    // Seat 2 becomes an OPEN seat, through the row's own select — the control
    // this file's "specific person" cells drive (`seat("Blue")`, :115).
    await user.selectOptions(seat("Blue"), "open");
    expect(screen.queryByLabelText("Seed")).toBeNull();
    expect(screen.queryByText("Advanced", { selector: "summary" })).toBeNull();
    // With a seat open the submit reads "Open table", not "Deal" (the cell at
    // :177 pins both arms).
    await user.click(screen.getByRole("button", { name: "Open table" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).seed).toBeUndefined();
    expect(posted(stub).seats).toEqual([{ user: ME }, { open: true }]);
  });
```

and beside it, the gate's own cell (Task 4's review, M-1: without `seedAllowed &&` on `seedError`, a refused seed typed
at a solo table keeps the button held after a seat opens, for a reason the fold no longer shows):

```tsx
  it("lets a refused seed go once a seat opens, so the button is never held for a reason nobody can see", async () => {
    const { stub, user } = show();
    await user.click(screen.getByText("Advanced", { selector: "summary" }));
    await user.type(screen.getByLabelText("Seed"), "1e3");
    // The premise: held, with its reason on screen.
    expect(screen.getByRole("button", { name: "Deal" })).toBeDisabled();
    expect(screen.getByText(/^Not a whole number:/)).toBeInTheDocument();
    await user.selectOptions(seat("Blue"), "open");
    const open = screen.getByRole("button", { name: "Open table" });
    expect(open.hasAttribute("disabled"), "held for a reason nobody can see").toBe(false);
    await user.click(open);
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect(posted(stub).seed).toBeUndefined();
  });
```

(`seat(colour)` is the file's helper for a seat row's kind select, defined at :54 and driven at :115, :126 and
:180; `posted(stub).seats` is `unknown` to `tsc`, so the list is compared whole, as every cell of the file does.)

- [ ] **Step 2: Run them to verify they fail.**

Run: `cargo test -p yokohama-web --lib a_creators_seed 2>&1 | grep -E '^error' | head -3` — `SeedPolicy`,
`with_seed_policy`, `SEED_NOT_SOLO` not found. `cargo test -p yokohama-worker --lib a_seeded_create 2>&1 | tail -3`
— `door_config_with_seeds` not found. `cd worker && npx vitest run --config vitest.scripts.config.ts
scripts/deploy-guard.test.mjs -t DIAGNOSTIC 2>&1 | tail -5` — the new cell RED (no failure matched). `cd web &&
npx vitest run src/NewGame.test.tsx -t "hides the seed" 2>&1 | tail -5` — RED (the field is there).

- [ ] **Step 3: The layer.** In `games.rs`, beside `RunHost` (find `pub enum RunHost`):

```rust
/// Whether a creator's `seed` is honoured at a table that seats somebody else
/// (the 2026-10-08 ruling R-SEC8(b), finding B3). **`SoloOnly` is the
/// production rule**: a seed is honoured only when every human seat is the
/// creator's and none is open — a creator who knows the seed knows every
/// deal and both decks (`tables::draw_seed`'s own threat model). `Diagnostic`
/// is the FIXTURES' opt-in, by name — the engine's `--master` convention: a
/// diagnostic value is named as one and is never a default a person reaches.
/// Who sets what: [`Games::with_parts`] (the hosts) builds `SoloOnly`;
/// [`Games::with_limits`] and so [`Games::new`] — "nothing but a cell builds
/// one this way" — build `Diagnostic`; `AppState`'s in-memory and `with_store`
/// constructors are `Diagnostic` and `AppState::production` (the NAS binary's
/// path) is `SoloOnly` — pinned in process by the route cell
/// `the_production_state_honours_a_seed_at_a_solo_table_only`, and on the
/// binary, through `main` and `AppState::new`, by `start_refusal.rs`'s
/// `the_binary_refuses_a_seed_beside_an_open_seat_and_opens_the_same_table_unseeded`;
/// the Worker reads `YOKOHAMA_DIAGNOSTIC_SEEDS`, `on` in its pool's test files
/// only; and the browser host's `init()` is `SoloOnly`, its `init_diagnostic()`
/// the harness's opt-in by name. No `Default`: every policy is named where it
/// is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedPolicy {
    SoloOnly,
    Diagnostic,
}
```

`Games` gains `pub(crate) seed_policy: SeedPolicy,` (set `SeedPolicy::SoloOnly` in `with_parts`'s literal);
`with_limits`'s `Self::with_parts(..)` call gains `.with_seed_policy(SeedPolicy::Diagnostic)`; beside
`with_archive`:

```rust
    /// This `Games` under `policy` — see [`SeedPolicy`] for who sets what.
    pub fn with_seed_policy(mut self, policy: SeedPolicy) -> Self {
        self.seed_policy = policy;
        self
    }

    pub fn seed_policy(&self) -> SeedPolicy {
        self.seed_policy
    }
```

In `tables.rs`, beside `UNLOCKED_CAP` (:752):

```rust
/// The sentence a seeded create is refused with at a table that seats
/// somebody else (R-SEC8(b), finding B3; [`crate::games::SeedPolicy`]).
pub const SEED_NOT_SOLO: &str =
    "a seed is honoured only at a table whose every human seat is yours — leave it blank and the server draws one";
```

and in `create` (:795–797), between `config_of` and `draw_seed`:

```rust
        // **A creator's seed is honoured at a SOLO table only** (R-SEC8(b),
        // finding B3): every human seat the creator's and none open, else the
        // creator knows every deal and both decks. The fixtures opt into
        // `SeedPolicy::Diagnostic` by name; nothing a person reaches does.
        if req.seed.is_some() && self.seed_policy == crate::games::SeedPolicy::SoloOnly {
            let somebody_else = seats.iter().any(|s| match s {
                SeatDoc::Open => true,
                SeatDoc::Human { user, .. } => user.as_str() != creator.as_str(),
                SeatDoc::Bot => false,
            });
            if somebody_else {
                return Err(GameError::BadRequest(SEED_NOT_SOLO.to_string()));
            }
        }
```

In `lib.rs`: `with_store_chat_and_archive` (:483–490) ends `.with_seed_policy(games::SeedPolicy::Diagnostic)`
— every `in_memory*` and `with_store*` constructor reaches it, and after the split below `new` no longer does —
with one doc sentence: "**Diagnostic seeds** (R-SEC8(b)): a test constructor — its stores keep no person's
game — honours a seed at any table; the production path is [`Self::production`], which builds the one
constructor directly." Split `new` (:414–435):

```rust
    pub fn new(
        identity: identity::IdentityMode,
        chat: chat::ChatConfig,
        multiplayer: Multiplayer,
        ttls: sweep::Ttls,
    ) -> Result<Self, String> {
        let archive: Arc<dyn archive::Archive> = match archive::DirArchive::from_env()? {
            Some(dir) => Arc::new(dir),
            None => Arc::new(archive::NoArchive),
        };
        Self::production(Arc::new(FsStore::new(store::default_dir())), identity, chat, archive, multiplayer, ttls)
    }

    /// [`Self::new`] past its two environment reads — the production state
    /// over the store and the archive it was handed: the one constructor
    /// directly (never a test constructor, whose `SeedPolicy` is
    /// `Diagnostic`), `SoloOnly` by construction. Pinned in process by
    /// `the_production_state_honours_a_seed_at_a_solo_table_only`, and on the
    /// binary — `main` → [`Self::new`] → here, the path no in-process cell
    /// runs — by `start_refusal.rs`'s
    /// `the_binary_refuses_a_seed_beside_an_open_seat_and_opens_the_same_table_unseeded`.
    pub fn production(
        store: Arc<dyn Store>,
        identity: identity::IdentityMode,
        chat: chat::ChatConfig,
        archive: Arc<dyn archive::Archive>,
        multiplayer: Multiplayer,
        ttls: sweep::Ttls,
    ) -> Result<Self, String> {
        sweep::check_archive(&ttls, archive::is_on(&archive))?;
        Ok(Self::with_store_chat_archive_and_log(store, identity, chat, archive, Arc::new(log::StderrLog))
            .with_multiplayer(multiplayer)
            .with_ttls(ttls))
    }

    /// Backed by memory, in `open` mode — the mode every test drives with a
    /// header. Tests, and any run that must leave no trace.
    pub fn in_memory() -> Self {
        Self::in_memory_with(identity::IdentityMode::Open)
    }

    pub fn in_memory_with(identity: identity::IdentityMode) -> Self {
        Self::with_store(Arc::new(MemStore::default()), identity)
    }

    /// In memory, with a heartbeat a cell can outlive — see [`AppState::heartbeat`].
    pub fn in_memory_with_heartbeat(heartbeat: std::time::Duration) -> Self {
        Self { heartbeat, ..Self::in_memory() }
    }

    /// In memory, with the chat configured rather than defaulted — how a cell
    /// asks for the lobby off, or for a keep small enough to watch a trim.
    pub fn in_memory_with_chat(chat: chat::ChatConfig) -> Self {
        Self::with_store_and_chat(
            Arc::new(MemStore::default()),
            identity::IdentityMode::Open,
            chat,
        )
    }

    pub fn with_store(store: Arc<dyn Store>, identity: identity::IdentityMode) -> Self {
        Self::with_store_and_chat(store, identity, chat::ChatConfig::default())
    }

    /// Every constructor above but [`Self::new`] and [`Self::production`] is
    /// this one with a store, an identity or a chat chosen for it, and this one
    /// is [`Self::with_store_chat_and_archive`] with
    /// no archive — since M6.5a THAT is the one constructor that builds anything.
    ///
    /// The chat facts live on [`Games`] and nowhere here (M6.5a, R-6.5a.1): the
    /// appender enforces them and `/api/health` reports them off the same
    /// object, so a host passes them in once and nothing on this state can drift
    /// from what the layer enforces.
    pub fn with_store_and_chat(
        store: Arc<dyn Store>,
        identity: identity::IdentityMode,
        chat: chat::ChatConfig,
    ) -> Self {
        Self::with_store_chat_and_archive(store, identity, chat, Arc::new(archive::NoArchive))
    }

    /// The one constructor that builds anything — the store, the identity door,
    /// the chat facts and the archive all given (M6.5a). Every constructor above
    /// but [`Self::new`] and [`Self::production`] is this one with something
    /// chosen for it; `in_memory*` keep no archive.
    ///
    /// **Diagnostic seeds** (R-SEC8(b)): a test constructor — its stores keep
    /// no person's game — honours a seed at any table; the production path is
    /// [`Self::production`], which calls `with_store_chat_archive_and_log`
    /// directly.
    pub fn with_store_chat_and_archive(
        store: Arc<dyn Store>,
        identity: identity::IdentityMode,
        chat: chat::ChatConfig,
        archive: Arc<dyn archive::Archive>,
    ) -> Self {
        Self::with_store_chat_archive_and_log(store, identity, chat, archive, Arc::new(log::StderrLog))
            .with_seed_policy(games::SeedPolicy::Diagnostic)
    }

    /// [`Self::with_store_chat_and_archive`] with the log chosen and without
    /// that constructor's `Diagnostic` opt-in — so under the production seed
    /// rule, `SoloOnly`, which [`Games::with_parts`] builds (R-SEC8(b)): the one
    /// seam a cell needs to read what the constructor says while it seeds
    /// (M6.5c2 cycle E). The NAS's binary reaches it through
    /// [`Self::production`], passing [`log::StderrLog`].
    pub(crate) fn with_store_chat_archive_and_log(
        store: Arc<dyn Store>,
        identity: identity::IdentityMode,
        chat: chat::ChatConfig,
        archive: Arc<dyn archive::Archive>,
        log: Arc<dyn log::Log>,
    ) -> Self {
        let rules = Arc::new(Rules::standard());
        let signals = Arc::new(signal::Signals::default());
        let games = Games::with_parts(
            rules,
            store,
            chat,
            signals.clone(),
            Arc::new(table::now_unix),
            archive,
            games::RunHost::Server,
        )
        .with_log(log);
        // **The house frame's counters start where the house chat document is**
        // (M6.5c2 cycle D; the close `2026-09-29-web-frontend-m6.5c2d-close.md`
        // §4 and §5, ruling 46): a lobby's chat panel reads when the frame's `chat` passes the `last` of the page it holds, so a
        // process that counted from 0 over a document at N told no lobby of a
        // message until it had taken N + 1. A document that cannot be read
        // seeds nothing: every house route refuses it out loud, and no message
        // can be appended to it, so there is no count to be wrong about — until
        // an operator repairs the file under a running process, which a
        // restart then seeds.
        //
        // **Said out loud** (cycle E; the close's §5, ruling 49): a failed
        // seed leaves this process's house counters OFFSET from the document
        // for the process's life (`seed_house` runs once and `bump_house_chat`
        // adds one per post), so a lobby misses house messages until the
        // process has taken N + 1 posts; a restart re-seeds. That is now a
        // line in the log rather than a silence.
        match games.house_counts() {
            Ok(counts) => signals.seed_house(counts),
            Err(e) => games
                .log
                .line(log::HOUSE_SEED_UNREAD, serde_json::json!({ "error": e.to_string() })),
        }
        Self {
            games,
            identity,
            heartbeat: HEARTBEAT,
            signals,
            // On, which is [`Multiplayer::from_env_value`]'s own answer to an
            // unset variable: every constructor above this one builds the
            // deployment that existed before M6.5b2, and only `new` and
            // [`Self::with_multiplayer`] can say otherwise.
            multiplayer: Multiplayer(true),
            dist: assets::dist_dir(),
            // No TTL, which is what an unset variable reads: only `new`,
            // given `main`'s reading, and [`Self::with_ttls`] say otherwise.
            ttls: sweep::Ttls::none(),
        }
    }

    /// This state serving the client from `dist` (M6.5b2) — [`Self::with_clock`]'s
    /// shape again, for the cells that need a real shell behind the fallback.
    pub fn with_dist(self, dist: impl Into<std::path::PathBuf>) -> Self {
        Self { dist: dist.into(), ..self }
    }

    /// This state with the multiplayer switch thrown (M6.5b2, R-6.5b2.4).
    ///
    /// A builder step over the field, [`Self::with_clock`]'s shape, for the two
    /// callers that need it: `main`, which read the variable, and the cells that
    /// assert what a single-player deployment serves.
    pub fn with_multiplayer(self, multiplayer: Multiplayer) -> Self {
        Self { multiplayer, ..self }
    }

    /// This state with its two table TTLs (M6.5c2 cycle F, R-F9) — the
    /// builder a cell takes for what `/api/health` reports and what the sweep
    /// runs under. It asks no archive question: [`Self::new`] is where a TTL
    /// without an archive refuses to start.
    pub fn with_ttls(self, ttls: sweep::Ttls) -> Self {
        Self { ttls, ..self }
    }

    /// This state under `policy` ([`games::SeedPolicy`]) — the builder step
    /// [`Self::with_store_chat_and_archive`] takes to opt every test
    /// constructor into `Diagnostic`, its one caller.
    pub fn with_seed_policy(mut self, policy: games::SeedPolicy) -> Self {
        self.games = self.games.with_seed_policy(policy);
        self
    }
```

**Seven TEST-side `Games::with_parts(..)` builders exist, five of which create seeded multi-person tables and
read RED under `SoloOnly` (18 cells, measured; `api.rs`'s `off` builder and `tables.rs`'s clock builder are
inert and opt in for uniformity) — each gains `.with_seed_policy(crate::games::SeedPolicy::Diagnostic)` after its
closing paren:** `api.rs`'s `games_with(..)` (:2662) and the `off` builder (:3287); `games.rs`'s
`every_bump_site_answers_under_no_signal` (:1279), the two builders at :1768 and :1808, and `watched_games`
(:3182); `tables.rs`'s builder (:2302). (`lib.rs:528` is the production constructor, which
`with_store_chat_and_archive` opts in above.)

The constructor split makes sentences elsewhere false — one doc pass (Task 4's review, M-5):
`with_store_chat_archive_and_log`'s doc ("the NAS's binary passes … through the constructor above and nothing
else": the NAS reaches it through `production`, and the two constructors differ by the seed policy too);
`signal.rs`'s `seed_house` and `tables.rs`'s citation of `with_store_chat_and_archive` as the NAS's seeding
constructor; `sweep.rs`'s "`AppState::new` asks it once the archive is resolved and before the store is built"
(`production` asks it, after `new` has built the `FsStore` value); `AppState::with_seed_policy`'s doc (its one
caller is `with_store_chat_and_archive`); `SeedPolicy`'s "who sets what" names the third host. And every sentence
saying the production rule is "pinned" names the cell that pins it: `the_production_state_…` for `production`,
the `start_refusal.rs` cell for `new`.

Run: `cargo test -p yokohama-web --lib 2>&1 | grep -E 'test result|FAILED|panicked'` — green, **421**. `cargo test
-p yokohama-web` — `wire_fixtures` and the gate green: the corpus's `Api::with_chat` goes through
`in_memory_with_chat` and `with_archive` through `with_store_chat_and_archive`, both `Diagnostic`;
`play_a_whole_game`'s two creates are a bot table and an unseeded two-person table (measured green).

- [ ] **Step 4: The Worker.** `crates/yokohama-worker/src/config.rs`: `InitConfig` gains
`diagnostic_seeds: Option<bool>,` (doc: "`YOKOHAMA_DIAGNOSTIC_SEEDS` as the shell read it — `true` in the pool's
test configurations alone; absent or `false` is the production rule, `SeedPolicy::SoloOnly`"); `Config` gains
`pub(crate) diagnostic_seeds: bool,`; `parse`'s key sentence reads "the object of its nine keys — …,
open_table_ttl_days and diagnostic_seeds" and `Ok(Config { door, chat, archive, ttls, diagnostic_seeds:
c.diagnostic_seeds.unwrap_or(false) })`. In `lib.rs:191–200` the `Games::with_parts(..).with_log(log.clone())`
chain gains `.with_seed_policy(if config.diagnostic_seeds { SeedPolicy::Diagnostic } else { SeedPolicy::SoloOnly })`
(`init_inner` binds it by destructuring — `let config::Config { door, chat, archive, ttls, diagnostic_seeds } =
config::parse(config)?;`, :174). **Two cells pin the old shapes and move with the code:**
`tests::init_refuses_its_configuration_as_a_positional_array` (:896) asserts the key sentence verbatim — it reads
"nine keys — …, table_ttl_days, open_table_ttl_days and diagnostic_seeds"; and `receipt.rs`'s own builder `r2_at`
(the `json!({"identity": door, "archive": "r2", …})` behind `r2(caller)`) gains `"diagnostic_seeds": true`, or
eight `receipt::tests` cells read the layer's 400 on their seeded open-seat `CREATE` (measured). The test helpers
(:656–666):

```rust
    pub(crate) fn config(archive: &str) -> String {
        door_config_with_seeds(archive, "worker", true)
    }

    /// `init`'s configuration at `door` (M6.5c2, spec §2.1): the object
    /// `worker/src/config.ts`'s `moduleConfig` builds, the door first.
    pub(crate) fn door_config(archive: &str, door: &str) -> String {
        door_config_with_seeds(archive, door, true)
    }

    /// `init`'s configuration with the diagnostic-seeds switch chosen — ON for
    /// every cell but the one about the switch, since the cells' `CREATE` is
    /// the corpus's seeded open-seat body.
    pub(crate) fn door_config_with_seeds(archive: &str, door: &str, seeds: bool) -> String {
        format!(
            r#"{{"identity":"{door}","archive":"{archive}","caller":null,"lobby_chat":null,"chat_keep":null,"chat_text":null,"diagnostic_seeds":{seeds}}}"#
        )
    }
```

**The browser host** (`crates/yokohama-wasm/src/lib.rs`): `init()` builds `Games::with_parts(..)` too and so reads
`SoloOnly` — one rule on three hosts (the client's form withholds the field for an open or another person's seat,
so nothing a person reaches changes); its harness opts in by name through a second export, the Worker switch's
shape, which the crate's one seeded cell and the browser half's walk replay call:

```rust
/// Build a FRESH `Games` over a fresh `MemStore`, whose log is the console
/// (M6.5c2), replacing any earlier one — under the production seed rule,
/// `SeedPolicy::SoloOnly` (R-SEC8(b)), as every host builds it: the client's
/// form withholds the seed for an open or another person's seat anyway, so
/// nothing a person reaches changes here.
#[wasm_bindgen]
pub fn init() {
    init_with(yokohama_web::games::SeedPolicy::SoloOnly);
}

/// [`init`] under `SeedPolicy::Diagnostic` — the HARNESS's opt-in by name
/// (security S1a, finding B3): the corpus walk opens a seeded table with an
/// open seat, which `corpus.node.test.ts`'s replay and this crate's own seeded
/// cells reproduce through this export; the browser client never calls it.
#[wasm_bindgen]
pub fn init_diagnostic() {
    init_with(yokohama_web::games::SeedPolicy::Diagnostic);
}

fn init_with(policy: yokohama_web::games::SeedPolicy) {
    let games = Games::with_parts(
        Arc::new(yokohama_core::Rules::standard()),
        Arc::new(MemStore::default()),
        yokohama_web::chat::ChatConfig::default(),
        Arc::new(yokohama_web::signal::NoSignal),
        Arc::new(now),
        Arc::new(yokohama_web::archive::NoArchive),
        yokohama_web::games::RunHost::Browser,
    )
    .with_log(Arc::new(ConsoleLog))
    .with_seed_policy(policy);
    *GAMES.lock().expect("the host's games") = Some(games);
}
```

(the body is `init`'s today, moved; `a_table_whose_watcher_list_holds_a_plain_name_reads` — a seeded two-person
create, 400 under `SoloOnly`, measured — calls `init_diagnostic()` at both of its `init()` sites; in
`web/src/local/corpus.node.test.ts` the `WasmHost` interface gains `init_diagnostic(): void;` and the two
`wasm.init();` become `wasm.init_diagnostic();`, or the replay's first create reads 400 where the script says 201).

**The Worker's TS cells that pin the configuration's shape move with it:** `worker/src/marker.unit.test.ts:111`
hand-builds a `ShellConfig` — it gains `diagnosticSeeds: false` (TS2741 otherwise); in
`worker/src/config.unit.test.ts` the two "leaves shellConfig and moduleConfig unchanged under …" cells assert
`Object.keys(JSON.parse(module))` has length 8 — now 9 — and "carries the archive switch as exactly off or r2, and
the rest raw" and "is off with no caller, no chat variable and no TTL set" compare the JSON string, which gains
`,"diagnostic_seeds":false` before its closing brace (four cells, measured).

`worker/src/config.ts`: `ROWS` gains `YOKOHAMA_DIAGNOSTIC_SEEDS: true,`; `onOff`'s name union gains
`"YOKOHAMA_DIAGNOSTIC_SEEDS"`; `ShellConfig` gains `diagnosticSeeds: boolean` and `shellConfig` sets
`diagnosticSeeds: onOff("YOKOHAMA_DIAGNOSTIC_SEEDS", text(env, "YOKOHAMA_DIAGNOSTIC_SEEDS"))`; `moduleConfig`'s
object gains `diagnostic_seeds: c.diagnosticSeeds,` (its doc's key list too). The `Env` declaration is the hand-kept
`worker/src/env.ts` (no generated `.d.ts` names these variables): `YOKOHAMA_DIAGNOSTIC_SEEDS?: string;` beside
`YOKOHAMA_DEV_DOOR` (:57), with a doc line — `ROWS` is a mapped type over `Env`'s string keys, so the row and the
declaration must move together. The two pool
files gain `"YOKOHAMA_DIAGNOSTIC_SEEDS": "on",` in `vars` with the comment "the corpus walk opens a seeded table
with an open seat (R-SEC8(b)); on in the pool and nowhere else — the deploy guard refuses it". The guard
(:593–595) gains its twin:

```js
    if (judged.text.includes("YOKOHAMA_DIAGNOSTIC_SEEDS")) {
      failures.push(`${config} names YOKOHAMA_DIAGNOSTIC_SEEDS — diagnostic seeds are set in the pool's configurations and nowhere else`);
    }
```

and its header list (:37) a line "4b. `YOKOHAMA_DIAGNOSTIC_SEEDS` appears nowhere in the deployed configuration
(R-SEC8(b))". `docs/web-lane.md` §12 gains the row, in the table's own columns:
`| `YOKOHAMA_DIAGNOSTIC_SEEDS` | test-only | `off` | `on`: a creator's seed is honoured at a table that seats
somebody else — the corpus walk's shape; the pool's two test files set it and the deploy guard refuses it in a
deployable file (R-SEC8(b)) | the pool's two files | `worker/src/config.ts`, `crates/yokohama-worker/src/config.rs` |`
(code spans, as every row — the parser skips a bare name; the kind and host are `YOKOHAMA_WEB_TEST_CLOCK`'s, its
twin). The guard's success line names rule 4b beside "no test clock"; `worker/src/config.unit.test.ts`'s
`describe.each` over the on/off variables gains the switch as a third row.

Run: `cargo test -p yokohama-worker 2>&1 | grep -E 'test result|FAILED'` — green with the new cell (**77**);
`cargo test -p yokohama-wasm` — **9**; `cd web && npm run build:wasm && npm run test:wasm` — **353** (the replay
under `init_diagnostic`); `cd worker && npm run build:module:debug && npm run typecheck && npm test 2>&1 | tail -6`
— the typecheck clean, the pool green, **224** (the walk's create honoured under the switch); `npm run
test:scripts` green, **533** (the guard's new cell; `lane-variables` with the new row — it lists files with
`git ls-files`, so it runs in a checkout, never in a bare export).

- [ ] **Step 5: The form.** In `web/src/NewGame.tsx`: after `const colors = …` (:188) add

```tsx
  /** A seed is honoured at a SOLO table only (R-SEC8(b)): every human seat
   *  mine, none open. The field is withheld — not disabled — the moment a seat
   *  is somebody else's, and a seed typed before that is not sent. */
  const seedAllowed = seats.every((r) => r.kind === "me" || r.kind === "bot");
```

`seedError` reads `seedAllowed && seedVerdict !== null && !seedVerdict.ok ? seedVerdict.reason : null`; the body's
seed spread reads `...(seedAllowed && seed ? { seed } : {})`; the `<details className="fold" …>` block (:388–420)
is wrapped in `{seedAllowed && ( … )}`. Run `cd web && npm run typecheck && npx vitest run src/NewGame.test.tsx
2>&1 | tail -5` — green, the new cell included.

- [ ] **Step 5b: The Playwright tier.** The client changed, so `cd web && npm run e2e` runs here (the full tier,
**79** — the split adds one cell at each width; cycle F's 77 before it, measured 8.0 min): `web/e2e/the_forms.spec.ts:38`'s widest-form cell (four seats, a person, open seats) asserts the Advanced
fold is ABSENT, and its "refuses a bad seed in its fold" half is its own cell on a solo shape (four seats all
"You"); `web/e2e/it_explains.spec.ts:119` asserts the withheld rendering at its seeded bot table (the move, the
reason sentence, no chart) and `:188` exercises the collapsed-features control on the SUGGEST panel, where the same
chart still draws (the old cell's chart-geometry checks move there with it; Task 1's B1 met the tier here; measured
73/4 before, the four being these). The two-defect probes of cell (a) take `config_of`'s LAST refusal, the blank
password, as the link before the seed's (adjacent), and the unusable id as the link after. The solo cell ticks the
password box too, so the form's "everything open" width is measured with the fold AND the password field open
(Task 4's review, M-7). What the old why cell pinned and nothing now does end to end — the served `why` chart's
geometry in a real browser (jsdom pins it; the same `ExplanationView` draws on the suggest panel) — is a RESIDUAL:
reaching a served `why` end to end needs a finished game, recorded for the close.

- [ ] **Step 6: Lint, suites, commit.** `cargo clippy --workspace --all-targets -- -D warnings` clean;
`cargo test -p yokohama-web`, `-p yokohama-worker`, `-p yokohama-wasm` green; the web lane's wasm check and
builds. Mutants: **M6** — `create`'s rule deleted: the tables cell RED (`Ok` where `Err`), the production route
cell RED (201 where 400), the Worker cell RED (measured: 419 + 76 green beside them); **M7** — `production` built
through `with_store_chat_and_archive` (the Diagnostic wrapper): the production route cell RED alone (420 green).

```bash
git add crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/lib.rs \
  crates/yokohama-worker/src/config.rs crates/yokohama-worker/src/lib.rs crates/yokohama-worker/src/receipt.rs \
  worker/src/config.ts worker/src/env.ts worker/wrangler.test.jsonc worker/wrangler.open.test.jsonc \
  worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs docs/web-lane.md \
  worker/src/marker.unit.test.ts worker/src/config.unit.test.ts \
  crates/yokohama-wasm/src/lib.rs web/src/local/corpus.node.test.ts \
  web/src/NewGame.tsx web/src/NewGame.test.tsx
git commit -m "web: a creator's seed is honoured at a solo table only — SeedPolicy, the fixtures' diagnostic opt-in, the Worker's switch and the guard's rule, the form withholds the field (B3; R-SEC8(b))"
```


---

### Task 5: Two Lows on the seed's two side doors — the lockdown dump off the `input` reply, the record route's stored bytes held to the handle — Opus (D5, D11; Review Focus 7)

**Files:**
- Modify: `crates/yokohama-web/src/api.rs` (`input` :1398–1427 the message builder; `record` :2110–2134; two
  constants/helpers beside them; cells in `api.rs`'s `mod tests` :2444 and `lib.rs`'s)
- Test: `api.rs`'s `mod tests`; `lib.rs`'s route cells

**Interfaces:**
- Consumes: `Repl::step`'s text (the three lockdown sites append `lockdown_dump()` — `seed: <seed>\nturn:
  <n>\n<log tail>` — under a `LOCKDOWN — <reason>` line, `repl.rs:2825`, `:2903`, `:2940`, `:3424–3431`);
  `games.store().read(Doc::Record, id)`; the helpers `a_finished_two_person_game`, `get_as`, `post_as`.
- Produces: `api::lockdown_messages(lines: Vec<String>, reason: &str) -> Vec<String>`; `api::RECORD_UNSAVED`; `record`
  holding the stored record to the handle by count AND terminal digest.

- [ ] **Step 1: The failing cells.**

(a) In `api.rs`'s `mod tests` (:2444), a unit cell on the filter:

```rust
    /// **A lockdown's dump never reaches the `input` reply** (finding D5):
    /// `Repl::step` appends `seed: <setup seed>`, the turn and the log tail
    /// under the `LOCKDOWN — …` line at every lockdown site, and `input`
    /// forwarded the whole text as `messages`. The filter keeps what came
    /// BEFORE the lockdown line — the mover's own deltas, public — and the
    /// line itself, and drops everything under it; a reason that carries
    /// digits is kept whole (Review Focus 7); a text with no lockdown line —
    /// a lockdown engaged somewhere the line was not printed — is replaced by
    /// the reason alone, never passed through.
    #[test]
    fn a_lockdowns_dump_is_cut_from_the_reply_and_the_reason_is_kept_whole() {
        let reason = "bot seat 1 could not decide honestly: the view was refused at turn 3 (code 4)";
        let lines: Vec<String> = [
            "Placed 1 assistant in Bank (A07)",
            &format!("LOCKDOWN — {reason}"),
            "seed: 20260831",
            "turn: 3",
            "  12  Blue: Move President to Bank (A07)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let kept = lockdown_messages(lines, reason);
        assert_eq!(kept, vec!["Placed 1 assistant in Bank (A07)".to_string(), format!("LOCKDOWN — {reason}")]);
        assert!(!kept.join("\n").contains("20260831"));
        // No lockdown line in the text: the reason alone.
        let kept = lockdown_messages(vec!["seed: 20260831".into(), "turn: 3".into()], reason);
        assert_eq!(kept, vec![format!("LOCKDOWN — {reason}")]);
    }
```

(b) In `lib.rs`'s `mod tests`, beside `a_locked_game_answers_every_callers_undo_with_the_lockdown_reason`
(:3556), the one fault a cell can reach through the API — a bot whose own deal is unrecorded:

```rust
    /// **The one lockdown a request can reach** (finding D5, spec correction 5):
    /// a record whose bot seat chose `Unknown` — its own deal unrecorded — makes
    /// the bot's `observe` refuse at its first turn, which the pump reports as
    /// a lockdown with the dump under it (`repl.rs:2903`). The table is opened
    /// through the router, the record then replaced by the crafted one and a
    /// fresh state built over the store, so the next request rebuilds from it;
    /// the person plays their turn and the reply that carries the lockdown
    /// carries NO `seed:` line and ends on the lockdown line, and the setup
    /// seed appears nowhere in that WHOLE reply — every key, `prompt.reason`
    /// included. The bot's own stream seed is another number (`777000111`), so
    /// that search reads the setup seed alone. **Measured
    /// reachable** (the plan's before review): the rebuild accepts the crafted
    /// record, and the bot's first turn reports "bot seat 1 could not decide
    /// honestly: the seat's view could not be taken: seat 1's own
    /// starting-order deal was never recorded; it cannot view itself" with the
    /// dump under it — this cell is the catcher on the wire, (a) the unit.
    #[tokio::test]
    async fn the_reply_that_carries_a_lockdown_carries_no_seed() {
        use yokohama_cli::bot::{BotSeats, PROVISIONAL_DEFAULT_BOT};
        use yokohama_core::{GameConfig, GameRecord, PlayerColor, PlayerId, StartingOrderChoice};
        let store = Arc::new(MemStore::default());
        let state = Arc::new(AppState::with_store(store.clone(), identity::IdentityMode::Open));
        let router = app(Arc::clone(&state));
        a_game(&router, "faulted", &[1]).await;
        // The crafted record: the same table, the bot's choice `Unknown`.
        let mut config = GameConfig::for_player_count(2);
        config.player_colors = vec![PlayerColor::Red, PlayerColor::Blue];
        config.start_player = PlayerId::new(0);
        let record = GameRecord::new(
            20260831,
            config,
            vec![StartingOrderChoice::Keep(0), StartingOrderChoice::Unknown],
            &Rules::standard(),
        );
        let mut session = yokohama_cli::session::Session::from_record(record, Rules::standard())
            .expect("a record with an Unknown choice resumes (the live-tracking shape)");
        // The bot's stream seed is NOT the setup seed, so the whole-reply search
        // below finds the setup seed or nothing.
        session.set_run_seats(BotSeats::new(vec![None, Some(PROVISIONAL_DEFAULT_BOT)], 777000111).seat_provenance());
        let crafted = session.to_record_json().expect("a record");
        let (_, version) = store.read(Doc::Record, "faulted").expect("a store").expect("the record");
        store.write_if_version(Doc::Record, "faulted", &crafted, Some(version)).expect("the crafted record");
        let state = Arc::new(AppState::with_store(store, identity::IdentityMode::Open));
        let (status, first) = get_as(&state, WHO, "/api/games/faulted").await;
        assert_eq!(status, StatusCode::OK, "premise: the crafted record rebuilds: {first}");
        let mut lockdown = None;
        for _ in 0..40 {
            let (_, now) = get_as(&state, WHO, "/api/games/faulted").await;
            let line = match now["prompt"]["rows"].as_array().and_then(|r| r.last()) {
                Some(row) => row["number"].as_u64().expect("a numbered row").to_string(),
                None => "confirm".to_string(),
            };
            let (status, reply) = post_as(&state, WHO, "/api/games/faulted/input", serde_json::json!({ "line": line })).await;
            assert_eq!(status, StatusCode::OK, "{line}: {reply}");
            let messages: Vec<String> = reply["messages"].as_array().expect("messages").iter().map(|m| m.as_str().unwrap_or_default().to_string()).collect();
            if messages.iter().any(|m| m.starts_with("LOCKDOWN")) {
                lockdown = Some((messages, reply));
                break;
            }
            if reply["prompt"]["kind"] == "locked" {
                lockdown = Some((messages, reply));
                break;
            }
        }
        let (messages, reply) = lockdown.expect("the bot's first turn engages a lockdown within the person's 40 inputs");
        assert!(messages.last().is_some_and(|m| m.starts_with("LOCKDOWN — ")), "ends on the lockdown line: {messages:?}");
        assert!(!messages.iter().any(|m| m.starts_with("seed:") || m.starts_with("turn:")), "the dump reached the wire: {messages:?}");
        assert!(!serde_json::to_string(&messages).unwrap().contains("20260831"), "{messages:?}");
        assert!(!reply.to_string().contains("20260831"), "the setup seed reached the locking reply: {reply}");
    }
```

(`Session::from_record(GameRecord, Rules)` is the resume path, `session.rs:218`; `set_run_seats` takes the
`Vec<SeatProvenance>` that `BotSeats::seat_provenance` returns, `session.rs:465` / `bot.rs:214`. If `from_record`
refuses an `Unknown` choice on a sim-shaped record, that refusal IS the report's finding for this cell.)

(c) In `lib.rs`'s `mod tests`, beside `the_record_route_is_409_while_running_403_for_a_watcher_and_the_stored_bytes_when_over` (:1328):

```rust
    /// **A finished game whose last save failed does not serve its running
    /// record** (finding D11): `record` read "over" off the resident handle,
    /// dropped the lock, and served whatever the store held — which, after a
    /// failed final persist, is the record from before the end, its seed in
    /// it. The route now reads the store under the handle's lock and holds the
    /// stored record to the handle by its event count AND its terminal digest:
    /// a shorter one is a 409 in `RECORD_UNSAVED`'s words, and so is one of the
    /// same length that is not the finished game, which carries no digest, or
    /// another. Measured reachable: a seat's own-turn undos while the store
    /// failed writes, then another line to the same count, left the store a
    /// running record of the handle's length. The one-short arm keeps the
    /// finished game's digest, so the COUNT refuses it. Of the two same-length
    /// arms, one carries ANOTHER real digest — the running state's, the shape
    /// every stamped server write leaves — so the digest's VALUE refuses it,
    /// and one carries none, so its PRESENCE does.
    #[tokio::test]
    async fn the_record_route_refuses_a_finished_game_whose_stored_record_is_behind_the_handle() {
        let (state, id, _) = a_finished_two_person_game("unsaved").await;
        // The handle is resident and over.
        let (_, game) = get_as(&state, "red@wire.test", &format!("/api/games/{id}")).await;
        assert_eq!(game["over"], serde_json::json!(true));
        // The store falls behind: the same record one event short.
        let store = Arc::clone(state.games.store());
        let (json, version) = store.read(Doc::Record, id).expect("a store").expect("the record");
        let mut record: yokohama_core::GameRecord = serde_json::from_str(&json).expect("a record");
        record.events.pop();
        let version = store
            .write_if_version(Doc::Record, id, &serde_json::to_string_pretty(&record).unwrap(), Some(version))
            .expect("the stale record");
        let (status, body) = get_as(&state, "red@wire.test", &format!("/api/games/{id}/record")).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["error"], api::RECORD_UNSAVED);
        assert!(!body.to_string().contains(&ENDED_SEED.to_string()), "the seed reached the wire: {body}");
        // The same length on another line, stamped with ANOTHER real digest:
        // the finished record with its last two events swapped, carrying the
        // digest of the finished line one event short — the running state a
        // stamped write names, and the digest the measured C1 record carried.
        let finished: yokohama_core::GameRecord = serde_json::from_str(&json).expect("a record");
        let mut other = finished.clone();
        let n = other.events.len();
        other.events.swap(n - 1, n - 2);
        assert_ne!(other.events, finished.events, "premise: the stored line differs from the handle's");
        let running = yokohama_core::state_at(&finished, n - 1, &Rules::standard()).expect("the line one event short");
        other.terminal_digest = Some(yokohama_core::state_digest(&running));
        assert!(finished.terminal_digest.is_some(), "premise: the finished record carries its digest");
        assert_ne!(other.terminal_digest, finished.terminal_digest, "premise: another digest, not the finished one");
        let version = store
            .write_if_version(Doc::Record, id, &serde_json::to_string_pretty(&other).unwrap(), Some(version))
            .expect("the same-length record, another digest");
        let (status, body) = get_as(&state, "red@wire.test", &format!("/api/games/{id}/record")).await;
        assert_eq!(status, StatusCode::CONFLICT, "the same length on another line, another digest: {body}");
        assert_eq!(body["error"], api::RECORD_UNSAVED);
        assert!(!body.to_string().contains(&ENDED_SEED.to_string()), "the seed reached the wire: {body}");
        // The same record with NO digest.
        other.terminal_digest = None;
        store
            .write_if_version(Doc::Record, id, &serde_json::to_string_pretty(&other).unwrap(), Some(version))
            .expect("the same-length record, no digest");
        let (status, body) = get_as(&state, "red@wire.test", &format!("/api/games/{id}/record")).await;
        assert_eq!(status, StatusCode::CONFLICT, "the same length on another line, no digest: {body}");
        assert_eq!(body["error"], api::RECORD_UNSAVED);
        assert!(!body.to_string().contains(&ENDED_SEED.to_string()), "the seed reached the wire: {body}");
    }
```

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --lib lockdown 2>&1 | grep -E
'^error' | head -3` — `lockdown_messages` and `api::RECORD_UNSAVED` not found: one compile error blocks all three
cells, so to READ the two route cells first stub the names (`lockdown_messages` returning its `lines`,
`RECORD_UNSAVED` any sentence, `record` unchanged) and run `… --lib -- the_record_route_refuses
the_reply_that_carries`: (c) RED, 200 where 409 (the stale record served, its seed in the body); (b) RED on the
dump — measured: the reply read `["Phase: Additional actions (after) -> Additional actions (before)", "Turn 1 Red
-> Turn 2 Blue", "LOCKDOWN — bot seat 1 could not decide honestly: the seat's view could not be taken: seat 1's
own starting-order deal was never recorded; it cannot view itself", "seed: 20260831", "turn: 2", "─ Log (4 of 4
event(s)) …", …]` — the D5 leak, through the API.

- [ ] **Step 3: The code.** In `api.rs`, beside `input` (above it):

```rust
/// `input`'s messages when THIS step engaged a lockdown (finding D5): the
/// lines before the lockdown line — the mover's own deltas — and the line,
/// and nothing under it. `Repl::step` appends `lockdown_dump()` there — the
/// setup seed, the turn and the log tail — for a terminal's operator, who owns
/// the game; a seat at a shared table does not. A text with no lockdown line
/// is replaced by the reason alone.
pub(crate) fn lockdown_messages(lines: Vec<String>, reason: &str) -> Vec<String> {
    match lines.iter().position(|l| l.starts_with("LOCKDOWN — ")) {
        Some(i) => lines.into_iter().take(i + 1).collect(),
        None => vec![format!("LOCKDOWN — {reason}")],
    }
}
```

and in `input`, replace `let messages: Vec<String> = body.lines()…collect();` (:1421–1422) with:

```rust
    let lines: Vec<String> =
        body.lines().map(str::trim_end).filter(|l| !l.is_empty()).map(str::to_string).collect();
    // **A lockdown's dump stays in the terminal** (finding D5): the reply
    // carries the lockdown line and nothing under it.
    let messages = match (repl.session.is_locked() && !locked_before, repl.session.record().lockdown.as_deref()) {
        (true, Some(reason)) => lockdown_messages(lines, reason),
        _ => lines,
    };
```

Beside `record` (:2103):

```rust
/// The 409 a finished game whose STORED record is not the finished game
/// answers (finding D11) — shorter than the handle's, or without the finished
/// state's terminal digest: its last save failed, so the store holds the game
/// from before the end — a running game's record, seed and all.
pub(crate) const RECORD_UNSAVED: &str =
    "the record is available once the game is over and its last move is saved — that save failed";
```

and `record`'s body from `let handle = …` (:2119) on:

```rust
    let handle = games.handle(id)?;
    // Under the lock, both facts: the handle's "over" and the stored bytes,
    // held to each other (finding D11) — a failed final persist leaves the
    // store behind a resident handle that says over: one move short, or, with
    // an undo and a replay before it, the same length on another line.
    let repl = games.lock_game(&handle, id)?;
    if !repl.session.is_over() {
        return Err(ApiError {
            status: StatusCode::CONFLICT,
            message: "the record is available once the game is over".to_string(),
            table_status: None, foreign_bot: None,
        });
    }
    let (json, _) = games
        .store()
        .read(crate::store::Doc::Record, id)
        .map_err(ApiError::server_error)?
        .ok_or_else(|| ApiError::not_found(format!("no game named {id:?}")))?;
    #[derive(Deserialize)]
    struct StoredRecord {
        events: Vec<serde::de::IgnoredAny>,
        terminal_digest: Option<String>,
    }
    let stored: StoredRecord = serde_json::from_str(&json)
        .map_err(|e| ApiError::server_error(format!("the stored record does not parse: {e}")))?;
    // **The digest of the handle's STATE, never the handle record's own
    // `terminal_digest` field.** `Session::to_record_json` stamps the live
    // state's digest on a clone at every write, so a stored record names the
    // state it was written from; the in-memory field is what the session was
    // loaded with — `None` for a game dealt in this process, the loaded
    // record's for a rebuilt one — and no apply or undo refreshes it.
    let finished = yokohama_core::state_digest(repl.session.state());
    if stored.events.len() < repl.session.record().events.len()
        || stored.terminal_digest.as_deref() != Some(finished.as_str())
    {
        return Err(ApiError { status: StatusCode::CONFLICT, message: RECORD_UNSAVED.to_string(), table_status: None, foreign_bot: None });
    }
    Ok(json)
```

(`record`'s doc gains, after "this is projection, not computation (the M3 lockdown rule).", the paragraph:

```rust
/// Since security S1a the store is read UNDER the handle's lock and its record
/// held to the handle by its event count AND its terminal digest — a stored
/// record of the same length that is not the finished game (undone and
/// replayed to the same count, a failed final save behind it) carries no
/// digest, or another. A shorter one, or one without the finished state's
/// digest, is [`RECORD_UNSAVED`]. The digest is computed under the lock, so the
/// lock keeps the answer consistent — which true sentence the caller hears —
/// and not the seed: read after the guard dropped, a store that moved in
/// between would carry another digest and be refused, or still hold the
/// finished game.
```

- [ ] **Step 4: Run, mutants, lint, commit.** `cargo test -p yokohama-web --lib 2>&1 | grep -E 'test
result|FAILED'` green (the lib as the tree reads it — 426 at Task 5's head, the scratch's 424 having predated
Tasks 3 and 4's cells). Mutants: **M8** — `input` passing `lines` through unfiltered: cell (a) untouched (a unit on
the filter), cell (b) RED alone, first at "ends on the lockdown line" (the `seed:` assertion catches it too) —
which is why (b) exists; **M9** — `record` without the comparison (both halves): cell (c) RED alone (200 where
409) at both arms; the digest half alone deleted: the one-short arm still reds through the count, the same-length
arm through the digest; **`Mnone`** — the digest half reading presence alone (`is_none()`): the same-length arm RED
(its record carries a real, DIFFERENT digest — the running record's at the handle's length minus one; a no-digest
record is the third arm, a shape no server writer produces; Task 5's review, I1). `cargo clippy -p yokohama-web --all-targets -- -D warnings` clean.

```bash
git add crates/yokohama-web/src/api.rs crates/yokohama-web/src/lib.rs
git commit -m "web: a lockdown's dump stays in the terminal, and the record route holds the stored record to the handle under its lock (D5, D11)"
```

---

### Task 6: The documents the code changed — Sonnet (D10; the lane document, the spec's §9.3)

**Files:**
- Modify: `docs/web-lane.md` — §4 :151 (the `admitted` bullet) and :161 (the explanation bullet); §13 :666
  (`POST /api/games`), :669 (`…/table`), :672–673 (`join`, `unlock`), :689 (`why`); §16 :933 (the `why` row's
  purpose column)
- Modify: `docs/plans/2026-10-08-security-remediation-spec.md` — §9.3 appended (this plan's
  §"Spec corrections", dated 2026-10-09, with the measured figures of §"The S1 figures")
- Test: `cargo test -p yokohama-web --test lane_document` (holds §13 and §16 to the route table) — the rows
  keep their routes, so it stays green; the point is the words

- [ ] **Step 1: §4.** In the bullet at :151, after "**`admitted(state, id, user) -> (TableDoc, Viewer)`** is
every route's door:" the clause "ANY signed-in caller at an `open` table, for `table`, `events` and `stream`
only" becomes "ANY signed-in caller at an `open`, `playing` or finished table for `table` (locked ones included
— R-3.13's widening; `table_for_anyone`), and at an `open` table for `events` and `stream`" and the bullet gains,
at its end: "**What a stranger learns at a `dealing` table, by design** (finding D10): `…/table` refuses them,
and `join`/`unlock` answer `409` with `status: dealing` — a table filling its seats is not a secret, its deal
is." Add to the bullet one more sentence: "**Two rules of security S1a:** an undo whose unit drew from a hidden
deck is refused at a table of more than one person (`api::UNDO_DREW`; R-SEC8(a)), and a creator's seed is
honoured at a SOLO table only — every human seat theirs, none open (`tables::SEED_NOT_SOLO`; R-SEC8(b)); the
fixtures opt into `SeedPolicy::Diagnostic` by name, the Worker's pool through `YOKOHAMA_DIAGNOSTIC_SEEDS` (§12), the
browser host's harness through its `init_diagnostic()` export; the NAS binary never."

In the explanation bullet at :161, replace "**`why` masks the roads not taken** under `HandVisibility::Masked`
(an unplayed `FulfillOrder` words as "Fulfill an order from hand"); the move PLAYED is never masked, and
`suggest` masks nothing." with: "**`why` is served at the REQUESTER's visibility** (security S1a, finding B1):
while the engine's own view for the requester (`observe`) still masks the mover's starting order and the game is
not over, the account is WITHHELD — `withheld` carries the move played, the claim and one sentence, and nothing
the search found, since every served value is a function of that card — and once the card is public, or to a
seat holder once the game is over (a watcher at a finished game reads the engine's view, as the record route
refuses them the record), the full account is served: the roads not taken masked under `HandVisibility::Masked` (an unplayed
`FulfillOrder` words as "Fulfill an order from hand"), the continuation's later fulfils masked the same way, the
move PLAYED never masked, and no key in the claim (`decision_seed` is gone; `suggest`'s `stream_seed` is gone
too — it inverted to the setup seed, finding A1 — and the CLI derives it one-way since). `suggest` masks nothing:
it ranks the caller's own hand."

- [ ] **Step 2: §13 and §16.** Row :666 (`POST /api/games`), its last column gains "; a `seed` is honoured at a
solo table only (R-SEC8(b))". Row :669's third column becomes "any signed-in caller at an `open`, `playing` or
finished table, locked included (R-3.13); nobody without a seat at a deal". Rows :672 and :673's last columns
gain "; `409 dealing` to a stranger while the table deals". Row :689's last column becomes "a bot move
reconstructed by replay, `?event=n`; `withheld` while the bot's starting order is hidden from the caller, the
full account once it is public or, to a seat holder, once the game is over; a typed refusal is a 200". Row :933 (§16, `why`), its
purpose column gains "— withheld while the bot's starting order is hidden from the caller". Run
`cargo test -p yokohama-web --test lane_document` — green.

- [ ] **Step 2b: The counts and a stale doc.** `docs/web-lane.md` §2's figures are re-counted at the code head
(Task 5's commit): the corpus's "**108** JSON files" (109 since Task 1), "the eighteen files no script names
(`THIRD_COLUMN`)" (nineteen), `cargo test -p yokohama-web`'s total, its lib and `wire_fixtures` counts, the Worker
crate's, vitest's — each as read, with the commit it was re-counted at; and `tests/wire_fixtures.rs`'s
`constructed()` doc ("**Six of them** … the four `why` payloads": seven since cycle E's `house_frame`, eight with
`why_withheld`, five `why` payloads) reworded to the count it builds; and `crates/yokohama-web/src/explain.rs`'s
`only_the_played_fulfill_…` doc drops its "66" figure ("the committed `why` fixture's candidates are all `Place …`")
as `HandVisibility`'s doc dropped its own — a count in a comment drifts (Task 1's re-review); and `explain.rs:763–764`'s
"`Session::new_sim` is the only constructor `games.rs` calls" is reworded to what is true at head — `games.rs`
rebuilds through `Session::from_record` alone, the dealing is `tables.rs`'s (`new_sim`), and a record also
arrives by the browser's restore, not only the store directory (Task 2's review); `lib.rs`'s three "one
constructor" sentences (`production` "the one constructor directly"; "since M6.5a THAT is the one constructor
that builds anything"; "The one constructor that builds anything") reworded to the builder as it stands —
`with_store_chat_archive_and_log` since cycle E, which `production` reaches directly (Task 4's re-review); and
`docs/web-lane.md` §2's `start_refusal` bullet (3 cells, the seed cell unnamed) re-counted at 4 with the binary cell
described. `cargo test -p yokohama-web --test wire_fixtures`, `--lib explain::tests` and `--test lane_document`
green.

- [ ] **Step 3: The spec's §9.3.** Append to `docs/plans/2026-10-08-security-remediation-spec.md` a
`### 9.3 — 2026-10-09, S1a's plan: the cycle split, the corpus, and six corrections` carrying this plan's
§"The cycle split" table in one paragraph, its §"Spec corrections" items 1–7 verbatim, and §"The S1 figures"'
two tables with their one-paragraph reading — the file's own instruction ("the plan review's findings and this
specification's corrections, dated, go here").

- [ ] **Step 4: Commit.**

```bash
git add docs/web-lane.md docs/plans/2026-10-08-security-remediation-spec.md crates/yokohama-web/tests/wire_fixtures.rs
git commit -m "docs: the lane document's admitted, explanation and route rows for S1a (D10), and the spec's §9.3 — the cycle split, the corpus rule and six corrections"
```

---

### Task 7: The reviews, the suites, the close — ORCHESTRATOR (spec §5, §9.2)

- [x] **Step 1: The "before" review — the orchestrator's own** (done 2026-10-09; the record is §"Review record"
  below and `docs/temp/security-review-2026-10-08/s1a-before/`). On a `git archive` export of master at
  `e9824ec9`, EXECUTE Tasks 1–5's cells against scratch implementations of this plan's own code blocks (a script
  that reads the blocks out of this file and applies them, never retyped), read every named mutant M1–M9 red at
  the cell it names and nowhere else, run the TS cells of Tasks 1 and 4 the same way (`vitest`, the typecheck,
  the guard's node cells), and record any cell that cannot go red on its mutant, any name here the tree does not
  spell, and any sentence of the spec's §2.1 rows or this plan that the code contradicts. Three cells are
  EXPECTED to teach something here and are written to say so: Task 2's drive (the words a take is described by),
  Task 5(b)'s reachability (the crafted record's rebuild) and Task 4(e)'s seat control. The findings go in the
  spec's §9.3 and are fixed in this plan BEFORE Task 1 is dispatched; the scratch implementation is discarded with
  the export, never carried into the branch.

- [x] **Step 2: The branch.** `git checkout -b security-s1 e9824ec9` (master's head: the code of `9ab8c430` plus
  the core PR's close documents, which the ledger, the board and the spec's §9 build on). S1b and S1c stack on
  it; nothing merges to master before S3 (spec §9.2).

- [ ] **Step 3: The dispatches.** Tasks 1–5 Opus, Task 6 Sonnet, each with an Opus reviewer who reruns the
  task's mutants on an export; the probe-and-name-concerns paragraph in every implementer brief; the SDD ledger
  under `docs/temp/security-review-2026-10-08/s1a-sdd/`. A `DONE_WITH_CONCERNS` correctness item is ruled on and
  fixed as a new commit before the review (the A1 art-cycle lesson); a fix round quotes its hunks.

- [ ] **Step 4: The per-edit suites at the code head**, in this order, every row rc 0, each count against the
  board's "Gates at HEAD": `cargo test -p yokohama-core` (unmoved: 1,587 / 29 ignored) · `cargo test -p
  yokohama-cli` (855 + 2) · oracle · difftest · `eval_probe` · `solver` · bridge and its four node suites ·
  `cargo clippy --workspace --all-targets -- -D warnings` · **the web lane's rows** (`docs/web-lane.md` §2 — the
  CLI and the layer moved): `cargo test -p yokohama-web` with `wire_fixtures` green WITHOUT
  `UPDATE_WIRE_FIXTURES`, `-p yokohama-wasm`, `-p yokohama-worker`, the public build's `identity::`, the `wasm32`
  check, both wasm builds, `web/`'s `build:wasm`, typecheck, `npm test` and `test:wasm`, `worker/`'s
  `build:module:debug`, typecheck, `test:scripts` and the pool — **and `npm run e2e`, since the client changed**
  (§2's rule; the Playwright tier the core PR did not owe). The fifteen-gate roster is NOT owed: core and the
  workspace version are unmoved (spec §5.2). Record every count; a count that differs is a finding.

- [ ] **Step 5: The "after" review — the orchestrator's own**, whole-branch, on a `git archive` export of the
  branch head: the fences of §"Global Constraints" (`git diff e9824ec9 -- crates/yokohama-core crates/yokohama-oracle
  crates/yokohama-difftest crates/yokohama-bridge crates/gen-data` empty; the CLI's diff confined to
  `suggest.rs` and its manifest; `SEMANTICS_REV` 7); the corpus moved in exactly two commits, each diff read
  against its task report's account; every mutant M1–M9 re-run; the argument of each row re-derived from the
  code, not from this plan — in particular that `visibility_of` is the engine's `observe` and nothing restated,
  that the withheld payload's bytes are a function of the move and the claim alone, that `SeedPolicy::SoloOnly`
  is what every production constructor builds (`Games::with_parts`, `AppState::production`, the Worker's
  `init` without the switch) and that the guard refuses the switch. One fix wave as new commits; a scoped
  re-review of the fix.

- [ ] **Step 6: The close's documents.** The ledger's eight rows (`docs/security-ledger.md`) rewritten `fixed <commit>` with
  the catcher named — A1 (the inversion pin and the CLI's digest cells), B1 (the withheld route cell and the
  independence pin), B2 (the un-draw cell), B3 (the three policy cells and the guard's), B7 (the seat cells and
  the leakage pin), D5 (the filter cell and, if reachable, the lockdown reply cell), D10 (the document rows, read),
  D11 (the stale-record cell); guidingpath's decision-log row for S1a's close and its current-state paragraph;
  the board's Track 4 (S1b next, planned against the stack's head, carrying §"The S1 figures"); `CLAUDE.md` at
  most one standing sentence, if one is genuinely new — the candidate: "the web lane's test constructors honour a
  creator's seed at any table (`SeedPolicy::Diagnostic`, a named opt-in); the production path never does, and a
  new constructor says which it is by name"; and this plan's §"Review record" with the readings.

- [ ] **Step 7: The PR, pushed, its merge left undone** (the owner, 2026-10-09). `git push -u origin
  security-s1`, then `gh pr create --base master --head security-s1` with a description naming the rulings
  (R-SEC3, R-SEC7, R-SEC8(a)(b); the owner's three of 2026-10-09), the eight rows and their catchers, the two
  corpus regenerations and what each moved, the suites' readings (the web lane's rows and the e2e tier included),
  the mutants' readings, the seven spec corrections, and the sentence "merge at S1's wrap, after S1c — S1b and
  S1c stack on this branch and this PR". The PR's checks run; nothing merges.

- [ ] **Step 8: The live tier, in the window (optional, the orchestrator's alone).** With the maintenance window
  of 2026-10-09/10 still open: a test deploy of the branch's build to the NAS (`deploy/RUNBOOK.md` §3's recipe,
  a dated line) and/or the Worker (`worker/RUNBOOK.md`'s, `npm run deploy` through the guard — the guard now
  refuses `YOKOHAMA_DIAGNOSTIC_SEEDS` in a deployable file, which is the first thing the deploy proves), then the
  §5.3 probes of spec correction 7 from the owner's workstation, every scratch table removed, the readings in the
  runbooks and this plan's §"Review record". **The live corpus replay** (`worker/scripts/live-replay.mjs`) sends the
  walk's rows verbatim, two of which are seeded open-seat creates scripted 201: under the production rule a deployed
  Worker answers them 400 (Task 4's report, C3), so the live replay needs a rule for those two rows (an expected
  `400` with `SEED_NOT_SOLO`'s sentence under a flag, or a walk variant) before the live tier runs — an S1b item if
  the window closes first. Skipped if the window has closed: then owed at the stack's deploy.

- [ ] **Step 9: The hand-off into S1b's planning session.** The board's Track 4 rewritten for S1b: the stack's
  head (the branch, the PR number), what S1b's plan transcribes from this one (§"The S1 figures" and the owner's
  knob choice under R-SEC8(f); the two notes the core PR left — the `CLAUDE.md` parenthetical to drop, the
  "`replay_strict` itself is unedited" snapshot to revise if `replay_strict` moves; spec correction 2's reading of
  the CLI fence for A3), the rows S1b and S1c own, and the first step of the next session ("brainstorm nothing —
  write S1b's plan against `security-s1`'s head from spec §2.1's A2, A3, B12, D6, D25, D27, D29 in this plan's
  shape"); the project memory updated the same way; `docs/temp/security-review-2026-10-08/s1a-sdd/` left in
  place with every report. The next session begins at the board.

## The S1 figures (measured 2026-10-09 for S1b's plan — A2's event-count cap and semaphore, B12's body cap and quota)

**Method.** A scratch example in a `git archive` export of `e9824ec9` (never committed), release profile, the
Mac (Darwin 27.0.0 arm64): uniform-random legal play to the game's end at 2/3/4P, 200 seeds each, every game
finishing (no guard hit at 60,000 moves); the record stamped and serialized compact and pretty — **pretty is what
a browser sends**, `Session::to_record_json` is `to_string_pretty`. The bot's own games through
`examples/arena cell s1sizes 2 v4-step9-draft1 v4-step9-draft1 3 --emit … --master 20261009` (six games).

| shape | events: median / p90 / p99 / max | bytes at the max game: compact / pretty | bytes per event (mean): compact / pretty |
|---|---|---|---|
| 2P random, 200 seeds | 5,099 / 7,624 / 12,853 / 15,342 (seed 104) | 655,008 / 1,496,479 | 42.9 / 98.3 |
| 3P random, 200 seeds | 6,736 / 10,071 / 14,080 / 18,625 (seed 160) | 812,811 / 1,882,221 | 43.4 / 99.9 |
| 4P random, 200 seeds | 8,849 / 11,774 / 15,621 / 16,867 (seed 4) | 736,517 / 1,695,364 | 43.8 / 101.0 |
| 2P bot vs bot (V4 × V4), 6 games | 287 – 416 | 19,127 / 41,984 at the longest | ~46 / ~101 |
| the owner's human-vs-bot games (3) | 243 – 252 | — / 25,818 – 26,075 on disk | — / ~104 |
| the browser fixture (`tests/fixtures/browser-played-2p.json`) | 595 | — / 54,103 | — / 91 |
| the review's worst-case body (`spread_4p_11`) | 23,606 | 1,046,122 (compact, passing turns) / — | 44 / — |

**Reading.** Two kinds of finished game exist: the ones people and the bot play (250–600 events, under 60 KB
pretty-printed) and random play (5,000–19,000 events, 0.5–1.9 MB), which no browser game produces. The present
1 MiB body cap already refuses a random 3P game pretty-printed (1.88 MB), and nobody has noticed, because no
browser game is one. Under the predicate replay the worst admitted record costs 54 ms whatever the cap, so the
event cap is a PLAUSIBILITY bound — what a finished game is — and a storage bound, not a CPU bound.

**What S1b's plan transcribes, unless the owner rules the other way:**

| knob | value | why |
|---|---|---|
| A2 event cap | **4,096 events** | seven times the longest game people or the bot have played (595), refused by COUNT before any replay; the worst-case body (23,606) and every random-play record fall outside |
| B12 body cap (`ARCHIVE_BODY_LIMIT`) | **512 KiB** | eight times the longest pretty-printed game measured (54 KB); above ~5,000 pretty-printed events, so the count cap speaks first on every real record |
| B12 per-caller quota | **256 records and 128 MiB** per caller segment | 256 finished games is years of play; 128 MiB is the count times the body cap, so neither bound is dead |
| A2 semaphore (the NAS) | **2 permits**, acquire bounded at **2 s** then `503` with `Retry-After: 1` | the NAS has 4 cores (the review's V9 read); two blocking replays leave two cores to the runtime; at ≤ 55 ms per admitted replay two permits clear ~36 worst-case uploads a second, far above the edge's 20 per 10 s per client; the bounded wait keeps queued bodies from piling up behind a burst |

The alternative sizing, recorded: an event cap of 24,000 with the body cap raised to the adapter's 2 MiB admits
every finished game of any play style measured, at the same ≤ 55 ms — a wider notion of "a finished game" and a
body cap moving UP, against the review's "lowered toward measured game sizes"; the choice is the owner's
(R-SEC8(f)), to be taken at S1b's plan.

## Review record

### The "before" review (Task 7 Step 1) — 2026-10-09, the orchestrator (Mythos), on an export of `e9824ec9`

**Method.** A `git archive` of master `e9824ec9` in the session's scratchpad (`node_modules` and the BGA captures
linked in); every code block lifted out of this file by `docs/temp/security-review-2026-10-08/s1a-before/blocks.py`
(by the line its fence opens on, never retyped) and applied by `apply.py`/`apply_more.py` — exact-text anchors,
each required to match ONCE, so a name the tree does not spell failed loudly; Tasks 1–5 applied in the plan's
order, each cell run for its fail-first reading, every named mutant M1–M9 applied and reverted by the same script
(`mutants.sh`); the TS cells, the typecheck, the guard's node cells, the lane's wasm and pool rows all run. The
findings (24, `s1a-before/findings.md`, with the raw logs beside it) were fixed in this plan at the source the same
day — above, every correction says "measured". The scratch is discarded with the export; nothing of it is carried
into the branch.

**The three cells written to teach.** Task 2's drive: as written it never drew (the layer's Placement prompt is
two picker rows and "Place no assistants"; the last-row walk places nothing) — the helper was rewritten to steer
Option B into the Port by area id, the President after it, the first `Port: take` row, measured at seed
20260831 (the order deck 10 → 9, the turn still open). Task 5(b)'s reachability: REACHABLE — the crafted record
rebuilds, and the bot's first turn reports "the seat's view could not be taken: seat 1's own starting-order deal
was never recorded" with the dump under it; the cell stays as the catcher on the wire. Task 4(e)'s seat control:
the control drove as written; what the cell had wrong was the submit's name — "Open table" once a seat is open —
and an `unknown` index that `tsc` refuses.

**What the plan had wrong, by task** (each corrected above). Task 1: the Step 2 filter missed the inversion cell
(F1); "the capture stays" was false — the CLI's regeneration pin holds the guide's suggest screen byte-exact and
the digest moves the key AND the answer, so the guide pair is re-captured (F2); three cells compare a whole `why`
payload and gain `withheld: null` (F3); the regeneration also moves `why_refused.json` and `why_foreign.json` by
one line (F4); the lib count is 416, not 417 (F5); the withheld branch was unreachable where placed (F6) and its
anchor matched twice (F7); three typecheck errors — a hand-built `WhyResponse`, `contract.test.ts`'s
`provenance.forced`, `_ww` unexported (F8); both walk replays enumerate the constructed files and lacked
`why_withheld` (F22). Task 2: the drive (F9); `undo_on_a_record_that_will_not_replay…` calls `trailing_mover`
(F10). Task 3: the walk's joiner is named, so the regeneration moves `table_foreign.json` (one line, a pre-field
document read as joined) and the archive walk's stored table document (the creator's seat gains the field), not
the walk's `invited` lines (F11, F20); the leakage pin's tightening misfired on a creator-typed address and is
dropped (F12); `client.rs:829` is a test helper whose creator seats read `true` (F13); the anchor
`the_creator_sees_the_address_they_typed` does not exist (the cell is `the_view_names_no_address_but_the_readers_own`);
27 of 41 sites are edits. Task 4: `SeedPolicy` out of scope in the tables cell (F15); the Worker cell lacked
`#[test]` (F14); `failuresOf` is not a name (`deployGuard(worker({…}))` is); seven test-side `with_parts` builders
must opt in (F18); `receipt.rs`'s `r2_at` is a third configuration builder (F16); the positional-array cell pins
the key sentence (F19); the form's submit reads "Open table" with a seat open and `posted(stub).seats[1]` is an
`unknown` index (F17); the `Env` declaration is the hand-kept `worker/src/env.ts`; the THIRD host — the browser
host reads `SoloOnly` through `with_parts` and needed its harness opt-in `init_diagnostic()` (F21); a hand-built
`ShellConfig` and four `moduleConfig` pins (F23, F24). Task 5: one compile error blocks all three cells' first
readings (the plan's three separate commands); 5(b) reachable. Seven of the plan's line anchors had drifted by a
few lines (`SETUP_SEED` :60, the suggest-cell closing braces); "find by content" covered them.

**Readings at the scratch's head, every task applied** (the Mac, debug): `cargo test -p yokohama-cli` 857 / 2
ignored (829 lib); `cargo test -p yokohama-web` — lib **424** (413 + 3 + 1 + 2 + 2 + 3), `browser_played_pin` 5,
`lane_document` 6, `play_a_whole_game` 2, `start_refusal` 3, `wire_fixtures` 17; `cargo test -p yokohama-worker`
77; `cargo test -p yokohama-wasm` 9; `cargo clippy --workspace --all-targets -- -D warnings` clean; the five fenced
crates byte-identical; `web/`: typecheck clean (with the module), vitest 1,500 in 71 files, `test:wasm` 353;
`worker/`: typecheck clean, `test:scripts` 533, the pool 224; the guard's cells and `lane-variables` (in a
checkout) green. The corpus: Task 1 moves `suggest.json` (145 lines, the answer), `why.json` (1,410, a different
account), `why_withheld.json` (new), `why_refused.json` and `why_foreign.json` (one line each); Task 3 moves
`table_foreign.json` (one line) and `wire-script/archive-walk.json` (the stored document's creator seat). The
guide pair: the key line, the suggested move, two swapped feature rows, one runner-up row.

**The mutants, each applied alone at the scratch's head** (the cells it reddens, and nothing else):

| mutant | reddens | green beside it |
|---|---|---|
| M1 `visibility_of` → Public | the visibility cell; the withheld route cell; the fixtures binary at `constructed()`'s own premise | 414 lib |
| M2 `word_line_masked` → `describe_move` throughout | the continuation cell | 415 |
| M3 `total` carried in the withheld text | the independence cell | 415 |
| M4 the deck comparison deleted | the un-draw cell (200 vs 403) | 417 |
| M5 `SeatView::of` ignoring `bound_at_create` | the unit cell; the route cell; the corpus golden (`table_foreign`) | 417 |
| M6 `create`'s rule deleted | the tables cell; the production route cell; the Worker's switch cell | 419 + 76 |
| M7 `production` through the Diagnostic wrapper | the production route cell | 420 |
| M8 `input` unfiltered | the lockdown reply cell (the unit untouched) | 423 |
| M9 `record` without the count comparison | the stale-record cell | 423 |

### Task 1 — the review (Opus), 2026-10-09

Task commit `60030076`; the ruled pre-review fix `0c353fe4` (C1: the contract's root-check table had no row for
`why_withheld`, a planted field passed vitest and `tsc` — two rows; C8: the suggest panel's reassurance kept).
Review verdict: spec ✅; quality Needs fixes — Important 2, both plan-mandated and ruled: **I-1** the plan's
game-over branch of `visibility_of` served a WATCHER at a finished game the bot's whole account (its unfulfilled
starting order's feature values) while `record` refuses a spectator at every status — fixed by mirroring the record
route's own condition (`is_game_over(now) && !held.is_empty()`), the sentence, the cells and the fixture premise
moving with it (the security hook's "sibling-path gate parity" was this); **I-2** the independence pin moved only
`total`, never the feature table it claims — three lines. Minors M-1 (the game-over rule pinned by one unit
assertion only — folded into I-1's cells), M-2, M-4, M-5 fixed; M-3 (refusals bypass the visibility read; a mismatch
refusal names the reconstructed move — pre-existing, narrow reach) PARKED for the whole-branch review. The
reviewer's own wave (W1–W9) and the brief's M1–M3 read as the plan says. Fix round 1 and its scoped re-review are
recorded below.

### Task 2 — the review (Opus), 2026-10-09

Task commit `7b29b5e3`; the pre-review fix `89640b86` (the implementer's probes: the plan's "a reveal draws nothing"
was false — a standalone starting-order reveal extracts its card and is refused, fail-safe; the tech-deck half had
NO catcher though spec B2's catcher names a lab take — the lab cell added; the comparison's place had none — the
other person's undo after the draw, R-P6's sentence). Review: spec ✅, Approved, Minor 4 — the reach sentence true
of what the layer deals, not what it reads (a restored backup); an `Option<usize>` premise passing on a missing key;
the one-person-and-a-bot shape pinned only on a non-drawing unit (W8 survived — a cell added); the refusal list —
all fixed in `3a5d935a`, re-review clean. Deferred: the bot cell could assert the undo removed exactly one event; the
reach comment's dropped clause on bound reveals.

### Task 3 — the review (Opus), 2026-10-09

Task commit `cfe37393`; the pre-review fix `9e01a0be` (the implementer's probes: the plan's `{ .. }` at
`house.rs:141` disarmed the house scrub's E0027 guard — the named pattern restored, which the background security
hook also flagged; the route cell's "the leakage pin holds the corpus" false — the golden does; red's re-named seat
read `false` and a name-arm mutant passed — `true`). Review: spec ✅, Approved, Minor 4 — the schema-1 convention
cell silent on the new field (W3 survived); the leakage pin's trust paragraph; the lane's rows not all run after the
corpus moved (now a plan constraint); "pristine" false of two pre-existing noises — fixed in `06889911`, re-review
clean. The regeneration moved exactly the two predicted files.

### Task 4 — the review (Opus), 2026-10-09

Task commit `808c9383`; the pre-review fix `2a02934b` (the Playwright tier met the shipped behaviours — the form's
fold withheld, the why withheld at the bot's opening move — two specs rewritten, 79; the browser host's `init()`
pinned; the guard twin stricter; the §12 row's kind; the order's two-defect probes; the switch's cells). Review:
spec ✅, Needs fixes — Important 1 (plan-mandated): the NAS path `main` → `new` → `production` was pinned by nothing
(W13: `new`'s pre-task body through the Diagnostic wrapper passed 456 + clippy) — a cell in `start_refusal.rs`'s
harness runs the real binary now; Minor 7 (the form's gate, the Worker's absent key, `Default` read by nothing —
dropped, rule 4b at the open door, the split's false sentences, the Worker's operator documents, the solo spec's
password) — fixed in `d7210605`, re-review clean. Residuals: the served `why` chart in a real browser is pinned in
jsdom only (a finished game is needed end to end); the live corpus replay's two seeded open-seat creates read 400
under the production rule (Step 8).

(Task 5's review and the "after" review follow here, as the core PR's plan kept them.)
