# The area-action choosers — the lab close (2026-11-22)

**The record of record for cycle 2 of Track 6, as built on the lab.** The design is `docs/plans/2026-11-21-web-area-action-choosers-design.md` (accepted 2026-11-21). The cycle ran as two plans, `docs/plans/2026-11-21-web-area-action-choosers-2a-plan.md` and `docs/plans/2026-11-22-web-area-action-choosers-2b-plan.md`, on 2026-11-21 and 2026-11-22. Their run ledgers are gitignored and live on one workstation, with every brief, implementer report and review beside them: `.superpowers/sdd/2026-11-21-web-area-action-choosers-2a-plan/progress.md` and `.superpowers/sdd/2026-11-22-web-area-action-choosers-2b-plan/progress.md`. This document carries what those ledgers hold and git does not.

**The branches.** Plan 2a: `web-choosers-goods`, cut from cycle 1's head `5abce3e3`; reviewed head `04742e87`; branch head `f46e8f76` (51 commits on `5abce3e3`; nothing after `04742e87` moves a file outside `docs/`, `worker/RUNBOOK.md` aside). Plan 2b: `web-choosers-cards`, cut from `f46e8f76`; reviewed code head `5ecad6f0`; the gates' head `4946f827` (45 commits on `f46e8f76`); the commits after it move documents only (the runbook's record of the private deploy, this close, the board, `CLAUDE.md`'s two sentences, the wrap's handover). Both are on the lab only. Neither is on origin or released: by the owner's direction of 2026-11-21, Track 6 runs out of the lab in three cycles and reaches origin once, at the wrap (cycle 3). No fenced crate moved and `SEMANTICS_REV` is 7.

**What the cycle built.** At four places on the board the game used to offer a long flat list of every legal way to use the action. Now one line opens a small chooser. At the Church the player switches on the goods they pay with. At Chinatown they add exchanges with plus and take them back with minus. At the Port they pick one or two order cards. At the Laboratory the rows of one technology fold into one line that opens the existing numbered pick. Each chooser only names one of the moves the engine already lists: the selection is looked up in the engine's legal list, and Confirm is live only when the list holds exactly what was chosen. The terminal and the browser both have the choosers. In the browser they are drawn with pictures of the goods and the cards and are laid out for phones held either way. When a Laboratory board holds both copies of one technology, each row now names its slot, so a player can tell which card a row takes. The terminal also shows each seat's face-up flag counts, as the browser has since cycle 1: in full in the `players` view and compactly on the dashboard's Mercenaries row. The work touched the hosted Worker, the in-browser engine, the wire corpus, the help and both guides.

**This is a lab close.** The pull requests, the release, the production deploy, the help's pictures and the decision-log rows are the wrap's and are listed in §10. A dated record is frozen once written, so what the wrap does is recorded forward in the wrap's own close, not here.

## 1. By plan and task

Each task had an implementer and an Opus task reviewer, who ran the task's mutants on an export of the commit. A fix round is counted when the reviewer asked for changes; a pre-review amendment ruled by the orchestrator is not a round.

### Plan 2a — the Church and Chinatown (2026-11-21)

| Task | What it built | Commits | Implementer | Fix rounds | What its review found that mattered |
|---|---|---|---|---|---|
| 1 | The Church's chooser as a pure model in `crates/yokohama-cli/src/chooser.rs`, read off the legal list | `160e01c2`..`da0b1ef4` | Sonnet | 1 | a "not offered" flag worked from the stock passed every cell (R12) |
| 2 | Chinatown's chooser as a pure model, with its fixtures | `61f3dff1`..`2e0df268` | Sonnet | 1 | the model matched through its own map, and a plan sentence the implementer added (the engine lists each set in row order) was false (R16) |
| 3 | The Church folds in the REPL and on the wire: the drivers, the corpus walk, the guide pair, the contract | `896f9bc5`..`c6890ebc` | Opus | 0 (approved after a pre-review amendment) | the production-door kept list moved with the walk (R18); a legal Church capture added (R20) |
| 4 | Chinatown folds in the REPL and on the wire, routed on the walk's four-player table | `fff28994`..`4bd7c31d` | Opus | 0 (approved after a pre-review amendment) | six walk lines joined the production-door list, and a production-door run against an older Worker now mismatches by design (R19) |
| 5 | The goods token and the Church's panel in the browser | `d5bef352`..`1decefc4` | Sonnet | 1 | the real fixtures co-varied their facts, so one-field clones were needed beside them (R32) |
| 6 | Chinatown's panel | `0554adb6`..`b8eee575` | Sonnet | 1 | a clone built to change one fact changed two (R34) |
| 7 | The browser's walks and the layout | `c7392a34`..`b22f6b72` | Opus | 1 | on a phone held sideways neither chooser could be confirmed or cancelled (R38); the focus ring could not be told from the pressed band (R39) |
| 8 | A chooser rebuilt on a new Worker instance | `a367fdd6`..`1f965abe` | Sonnet | 1 | neither `confirm` half told the named move from the nearest wrong one (R43) |
| 9 | The browser's words: two help sentences in `Help.tsx` and `docs/web-guide.md` | `69d31754`..`0d237fcd` | Sonnet | 1 | both sentences, the orchestrator's, overstated (R44) |

Seven of nine tasks took one fix round and none took a second. Task 8's first three commits were renamed by R41, so the table gives the new ids (`00c26576` → `a367fdd6`, `e5b95c38` → `b06a81b3`, `29e56df4` → `30ed9379`). The orchestrator's one-sentence docs commits sit inside the ranges.

### Plan 2b — the Port, the Laboratory, the slot marker and the terminal's flag counts (2026-11-22)

| Task | What it built | Commits | Implementer | Fix rounds | What its review found that mattered |
|---|---|---|---|---|---|
| 1 | The Port's chooser as a pure model, with three fixtures the engine's invariants accept | `2815e0d8`..`df45a54b` | Sonnet | 1 | two cells could not tell the list from a restated reach rule (T1-a) |
| 2 | The Port folds in the REPL and on the wire (the new prompt kind `port`); one helper closes all three choosers; every driver and the corpus walk learn it | `e5973fe9`..`dc7cd552` | Opus | 1 | three mutants survived: switching a card off, a second card refused for payment, the entry's count (T2-c) |
| 3 | The Laboratory folds by title into the existing pick (no new prompt kind) | `6bd8f5c3`..`2a54094f` | Opus | 1 (its last finding closed by Task 3b) | the entry's place was unpinned; two guide sentences were false (T3-b, T3-c) |
| 3b | Two copies of a technology are told apart: ` (slot N)` after the name, only when the acting board holds both | `aaf0ff9e`..`5bf15f6a` | Opus | 1 | the browser's Suggest and Why word a line's later steps at the root, so the claim was narrowed (T3b-a); the acting board was not told from the President's board (T3b-b) |
| 4 | The Port's panel in the browser | `22c9ac8f`..`799cae88` | Sonnet | 1 | the pressed state was readable from position; the zones were drawn in a fixed order (T4-a) |
| 5 | The browser's walks and the Port's layout, with a flat card face | `0982fb8b`..`7b94f4cb` | Opus | 1 | the walked game had no floor; a scripted scroll proved nothing about a box a player cannot scroll; cards were cut on short windows (T5-b to T5-d) |
| 6 | A Port chooser rebuilt on a new Worker instance | `c0c8c9a7` | Sonnet | 0 | approved at first review; the plan's mutant sentence was false |
| 7 | A whole three-player game through the web layer, answering every chooser, and what a press costs, read and recorded | `fe6ded0a`..`b8708c83` | Opus | 2 | the Chinatown arm could never build a set of two by construction (T7-b); the cap was sized from the shortest game (T7-e) |
| 8 | The terminal's flag counts: the `players` view and the Mercenaries row | `dec22f48` | Opus | 0 | approved at first review; three plan sentences corrected |
| 9 | The browser's words (four sentences, P1 to P4) and the lane document | `dddd65cf`..`b2c56288` | Sonnet | 2 | §1's exception sentence left out Task 8's terminal change, then carried two clauses the tree does not bear out (T9-b, T9-c) |

**Task 3b was added during the run on the owner's word** (2026-11-22): a player has to know which of two copies of a technology a row takes, so some marker is needed (T3-e). **The tasks ran out of order under rulings S1 to S3.** Task 4 ran while Task 3's review was out, Task 6 ran ahead of Task 5, and the lab push stopped waiting for each review. The order of dispatch was 1, 2, 3, 4, 6, 8, 3b, 5, 7, 9, with one implementer in the checkout at a time. Plan 2b's trailers name the model that wrote each commit (R50 of plan 2a).

## 2. The two whole-branch reviews

### Plan 2a (at `0d237fcd`)

A Workflow of three reviewers (`wf_b43b0f62-a8c`): a Fable lead (the whole diff against the design, the documents, the triage of the deferred list), an Opus mutation battery (the only Playwright runner) and an Opus wire reviewer (the mask, the corpus, compatibility, every host, the production-door list). Between them they found 34 findings and no Critical, and all three verdicts read "ready after the fix wave". The lead found six Important, five for the wave and one for plan 2b. The battery ran 120 mutants: 109 caught, 6 equivalent (each measured), 1 the recorded guide-only word change and 4 real gaps; the whole browser suite unmutated read 93 passed. The wire reviewer found two Important compatibility findings. It also measured the same-game claim against the corpus as it stood before the branch: all 111 base fixtures, `log.json` and the archived wire record unchanged.

**The one fix wave** was twenty items in four commits by an Opus implementer: `bc5ff55b` (the browser's product code: the focus kept on a pressed control, R45; a fallback for a prompt kind the client cannot draw, R46), `20774e8c` (Rust), `b1618071` (TypeScript cells), `32516422` (documents). The orchestrator's trailer correction followed as `21bf5dbc` (R50). **The one scoped re-review** read all twenty addressed and ran 27 mutants, every one caught. It tried the focus hand-off by keyboard in Chromium and found one new Important: a false sentence the wave wrote into the plan. That sentence and the residue were corrected by the orchestrator in one docs commit, `04742e87` (R51). **Parked:** a same-tick double press on two rows can lose a focus hand-off (pointer only); the browser's default ring on the chooser node; the fallback's Cancel drawn for any seated viewer; a nit in `harness.ts`'s comment.

### Plan 2b (at `b2c56288`)

A Workflow of three reviewers (`wf_da7b858f-344`, 86 minutes) on exports holding the head's tree: a Fable lead, an Opus mutation battery (the only Playwright runner) and an Opus wire-and-hosts reviewer. There was no Critical and every reviewer read "ready with fixes".

- **The lead** found design coverage complete (§5.1, §5.4 to §5.8 and every §6 bullet carried by code and a cell). It found the departures P1 to P12 and the run's rulings sound, challenging only T3b-a's wording. Its one Important (I-1): the browser's Suggest and Why can print a STALE slot on a later step, not only none. It measured the case: the second Ball reads `(slot 5)` at the root and `(slot 4)` stepped. It reasoned the reach: a mercenary acts at a Laboratory first, then the President there in the same turn. Nine Minor, sentences in the main, and M-8 for the owner's reading. It answered Task 6's open question: the Worker's pool pins the deletion of a chooser's lines at `"[]"` (`worker/test/table.test.ts:160`).
- **The battery** ran 93 mutants at the head and caught 91. One was equivalent: `client.rs` recomputing `can_add`, since the engine lists every pair in reach or none. One survived: `twin_slot`'s arm for a non-Laboratory acting area had no cell (BAT-1), and that mutant got its cell in the wave. Every mutant on every task's line and every fixed review survivor was caught.
- **The wire reviewer** found the same game held, the four new fixtures and the contract pinned, and nothing leaking: the `port` arm rides the acting seat only. All three hosts carry the Port and the Laboratory pick, and the kept list checked row by row. Its one Important: `wire_fixtures.rs`'s "three of them folded" was false (15 of the walk's 95 area-action prompts are folded: 3 Church, 11 Port, 1 Laboratory). It also found three Minor.

**The one fix wave** (FW-a) was thirteen items by one Opus implementer in `fc9be3f0` (the Rust half) and `5ecad6f0` (the documents and the browser half): 12 files, +209/−42. It made the three advisor sentences true, with `explain.rs` unchanged; it corrected the folded-prompt count; it added BAT-1's cell, a Laboratory pick restored on a new Worker instance, and a Port twin of the Chinatown cell through the compiled module; and it corrected eight comment and document sentences. Five of the thirteen proposed sentences were false when tried, and the implementer wrote the true ones (F1, F2, F3, F12, F13). **The scoped re-review** read all thirteen addressed, with no new Critical or Important. Its three mutants were all caught. Every `crates/` hunk was a comment or test-module code, and no fixture moved. Two Minor wording points were parked by FW-b and FW-c. FW-b's correction to the plan landed as `4946f827`.

## 3. The gates at plan 2b's head

Read at `4946f827` (the reviewed code head `5ecad6f0` plus one docs commit to the plan) on the MacBook Air (Mac17,3, Apple M5), 2026-11-22. The BGA corpus was present and the tree clean. `gates/per-edit.sh` ran one command at a time, 14:16:11Z to 14:38:15Z, and every command returned 0. The load average read 2.9 at the start.

- **Per-edit.** core 1,587 / 29 ignored; cli 955 + 2 ignored (lib 927, bga_import 9, guide_sync 18, piped_dogfood 1 + 2 ignored); oracle 191; difftest 52 / 5 ignored; eval_probe 214; solver 132; bridge 82 and its four node suites; web 627 (lib 582, browser_played_pin 5, lane_document 6, the whole-game gate 4, start_refusal 11, wire_fixtures 19); wasm 10; worker 92; the refusing build's `identity::` slice 9; the `wasm32` check and the Worker's release-wasm build; `gen-data` byte-identical; clippy clean; `cargo doc -p yokohama-worker --no-deps` the same 2 warnings.
- **Web.** typecheck 0; vitest 1,714 in 77; `test:wasm` 430 in 20; e2e 103 passed in 9.7 min (605 s wall).
- **Worker.** typecheck 0; `test:scripts` 801 in 33; the pool 302 in 14.
- **Fences.** Both empty against `f46e8f76` and against master `e600f5f4`; `grammar.rs` unmoved; 0 uncommitted paths.
- **Against plan 2a's head**, as the ledger computes it: core, oracle, difftest, eval_probe, solver and bridge unchanged, since no fenced crate moved. cli 915 → 955 (lib 887 → 927: Task 1 nine, Task 2 twelve, Task 3 seven, Task 3b six, Task 7 one, Task 8 five). web 619 → 627 (lib 576 → 582; the whole-game gate 2 → 4). worker 88 → 92. vitest 1,690 → 1,714. `test:wasm` 429 → 430. e2e 95 → 103. `test:scripts` and the pool unchanged.
- **The roster: 521 s, 15 of 15**, one uninterrupted sequence, 14:39:43Z to 14:48:24Z, with the release binaries pre-built first (70 s, untimed): fuzz 53, census 1, observation 4, action_equiv 30, action_worst 1, arena 23, policy_div 7, evaluator 289, architecture 53, recovery_lib 11, piped_dogfood 13 (2 passed: the release game with its three floors), differential 29, routes3_diff 2, scoring_diff 2, routes3_worst 4. The machine was not quiet. At the start an antivirus scan held about one and a half cores and the load average read 8.3, since the per-edit run had ended 18 s before; at the end it read 3.9. No fenced crate moved, so fourteen gates' tests are master's; the piped-dogfood gate is the one this cycle changes. The figure stands beside the 588 s at plan 2a's head (taken under more load), the 514 s at cycle 1's head and the 456 s at `e41352a6`, as a per-machine reading, never against them.

Plan 2a's own gates were read at `04742e87` on 2026-11-22 on the same machine: every per-edit command green, and the roster at 588 s, 15 of 15, under load. They are recorded in plan 2a's last section and in the board's "Gates at HEAD".

## 4. The private test Worker

By the owner's direction of 2026-11-22 for plan 2b's run, that plan's "After the tasks" includes the deploy of the reviewed head to the private test Worker and its verification there. The full record is `worker/RUNBOOK.md` §26 (its second record of 2026-11-22); this is its outline.

- **The deploy.** From a clean tree at `4946f827`: the private Worker's own keyless client build (the release `yokohama-release-cfb3f42dde69314f`), `npm run build` in `worker/` (the release smoke 200 at `server_version 0.7.3`, `semantics_rev 7`), and `npm run deploy:private` through both guards at 14:49:43Z to 14:50:00Z: version `5204700b-1758-44d1-a207-66199ae6da40`. The workspace version stays `0.7.3`, nothing is tagged, and production is untouched. The version it replaced, `eba04eca-d30c-427c-9651-ede6693e3f00` (plan 2a's head), is the rollback target.
- **A login in the way.** At 14:50Z every request from the workstation answered 302 to Access's login, on this host and on the NAS's alike, with WARP connected: the device's identity through WARP had lapsed since the morning's reading. Signing in as the owner is not an agent's to do, so the run waited; the owner re-authenticated WARP and the readings were taken at 16:08:33Z to 16:09:23Z.
- **The verification, line by line** (plan 2b, "After the tasks", step 4):

| The plan's line | What was read | |
|---|---|---|
| The recipe's readings over WARP | `/api/health` 200 at `0.7.3`, `semantics_rev` 7, `cache-control: no-store`; `/api/me` the caller under Access; `/` 200 with the enforced header set; `sw.js` naming the new release; the bundle carrying the Port panel's two asking lines | as stated |
| Chinatown, unmoved | a four-player solo table at the walk's `wire4` seed and lines: both chooser replies equal the committed `chinatown.json` and `chinatown_selected.json` in every top-level key but the table's `id`; `confirm` applied the exchange | as stated |
| The Port's chooser on a real host | a two-player solo table at the walk's seed: the entry `1` (`Port — take 1 or 2 order cards (4 in reach)`), kind `port`, cards 34, 19, 12 and 4, none selected; one card selected, legal, one way; a second selected, no other addable; `confirm` applied and the log's last line reads `Red: Port: take O34, O19, paying 2 yen`; no reply's `messages` carried a line of the terminal's screen | as stated |
| The Laboratory's fold on a real host | reachable by lines: a two-player table at seed 18, whose Laboratory A holds both copies of Letterpress Printing: the entry `… — 2 ways to pay`, the pick with rows `(slot 2)` and `(slot 3), slot fee: 2 yen`, `cancel` back to the main list | as stated |
| One press timed | the round trip of one `POST …/input` as `curl` timed it over WARP: a first Port card 92 and 94 ms, a Chinatown add 106 ms; the other inputs on those tables 90 to 192 ms | read |
| The update path | NOT READ by the session: the one browser it could drive held no service worker and no cache for the host. The plan names this line the owner's reading in that case, and it does not hold the pass | the owner's |
| The live replay | not run: it is the release window's | by the plan |

- **The verdict.** The verification passes as the plan defines it: every line that could be read reads as stated, and the one that could not is the line the plan gives to the owner. What remains on this host is the owner's own reading, in a browser and in the terminal, and the update path; both open the wrap's session (§10).

## 5. Every ruling, in the order made

Each ruling was made by the orchestrator on the owner's behalf during the run, except where a row says the owner made it. A cell condenses the ledger's own text, and "not stated" means the ledger gives no cost. The owner's words are paraphrased with their date; the ledgers keep them verbatim.

**A key to the ids, which collide across the plans.** Plan 2a's pre-run calls are P1–P5, and plan 2b's pre-run calls are P1–P12. Plan 2b's help sentences are also named P1–P4 in its "The words", and plan 2a's help sentences are S1 and S2; those four and two are sentences, not rulings. Plan 2b's sequencing rulings are S1–S5. Plan 2b's pre-flight rulings are F1–F5; the fix wave's F1–F13 are ITEMS of ruling FW-a, not rulings. Plan 2a's task reviewers named their mutants R1–R11, the same form as the rulings. **Plan 2a's ledger has no R24**: its numbers run from R23 to R25 and it says nothing of the gap.

### Plan 2a (orchestrator Mythos 5.2; the owner's word to begin the run by subagent-driven development, 2026-11-21)

| id | what was decided | why | what it costs if wrong | where it landed |
|---|---|---|---|---|
| P1 (pre-run) | The settle wait is fixed by a DOM mark: a control the payload disables carries `data-unavailable` (Tasks 5, 6), and the wait ignores those (Task 7); Cancel never carries it. | The plan review's Important 1; loosening the wait to `[data-busy]` alone would empty its second leg. | One attribute per control. | The plan, re-committed after `a7eb94b5`. |
| P2 (pre-run) | `confirm` closes then commits; Step 6 holds every committed fixture unmoved, `choice_ChurchSpace.json` the witness. | Important 2; stronger than the reviewer's three-file list. | Not stated. | The plan. |
| P3 (pre-run) | Task 7's cell 5 is a browser cell. | Important 3 (the cited precedent did not exist). | Not stated. | The plan. |
| P4 (pre-run) | The piped-dogfood driver gets no Chinatown arm; Chinatown's terminal coverage is Task 4's sweep. | Its gate is 2P, with no Chinatown on the board, so an arm could never run. | Not stated. | The plan. |
| P5 (pre-run) | The browser suite is red from Task 3 to Task 7 by design, stated in the Global constraints; no task between them runs `npm run e2e`. | Not stated beyond the design. | Not stated. | The plan. |
| — (pre-run) | The plan review's Minors 4–10 and 12–14 are folded as the reviewer proposed. | The reviewer's proposals. | Not stated. | The plan. |
| R1 | The points flagged for the owner that he left unchanged when he started the run run as written: the red browser suite in Tasks 3–7, no Chinatown arm in the terminal's whole-game gate, the empty Church selection as a payment, the design read as accepted, the arms' extra fields `stock`, `sells` and `words`, and no line printed on opening a chooser. | The instruction was to begin the plan. | A re-dispatch of the task concerned if the owner changes one later. | The dispatches. |
| R2 | Report files take the plan's names, not the skill's default. | The plan is the project's rule. | No cost. | The dispatches. |
| R3 | Task reviewers run the task's named mutants on their own export and re-run no other suite. | The plan and the owner's threefold overlap; this overrides the skill template for mutants only. | Reviewer wall time. | The review dispatches. |
| R4 | Each export lives under the scratchpad with its own `target/`. Reviewers scope cargo with `-p` and end with `cargo clean` inside the export, never `rm`. The orchestrator removes spent exports and reads `df` between tasks. | 16 GiB free against an 8.6 G `target/`. | A full disk mid-run. | The dispatches. |
| R5 | The orchestrator pushes to the lab after each task's review is clean; nothing goes to origin. | Not stated. | Not stated. Broken once, and it stood for the rest of the run (see the list after this table). | Practice. |
| R6 | A 55-minute heartbeat of zero-tool fires runs while agents run. | The owner's protocol. | Not stated. | A cron. |
| R7 | Task 1 writes `issue_position()` and Task 2 `ordinary_position()` in a `#[cfg(test)] pub(crate)` module of `chooser.rs` that `repl.rs`'s tests can reach. | Task 3's Step 1 asks for a shared helper but its file list omits `chooser.rs`; written reachable now, nothing moves later. | No cost. | `160e01c2`. |
| R8 | Task 2's cell 9 asserts the count (2,001), nine rows and `chosen == 1`, and prints its time unasserted. | A cell that asserts nothing is the rubric's defect; "no time is asserted" still holds. | A count that reads differently is a finding, not a number to edit. | `61f3dff1`. |
| R9 | A task that moves no Rust file runs the web commands its steps name and the fence checks only. | The plan's own sentence. | Not stated. | The dispatches. |
| R10 | Task 5's cell 8 clone changes `preview` and `offered_faiths` together, cloned from a corpus payload. **Superseded by R20** (a real legal fixture), then by R32 (real fixtures and one-field clones together). | It stays within "a payload comes from `web/src/wire/`". | No cost. | Task 5's additions. |
| R11 | An agent reads its brief and the plan's named sections by line range, never the whole plan, and nothing is retyped into a dispatch. | The skill; the plan stays the single source. | Not stated. | The dispatches. |
| R12 | Task 1's I-1 is accepted: cell 5 gains a third case (no Church move listed; copper and silk `offered: false`; `offered_faiths` `None`), and the plan's cell-5 text gains it with it. | The plan's cell was weaker than its own "nearest wrong one" constraint and design §6. | One fix round. | `da0b1ef4`. |
| R13 | Task 1's Minors 1–4 ride fix round 1 and do not count toward the breaker. | Same file and a live context; each pins a word or a wire value. | A slightly wider re-review. | `da0b1ef4`. |
| R14 | Task 2's cells 3 and 5 get a catcher before review: a new cell 10, six distinct holdings in cell 7, R8 in cell 9, and the Mutants paragraph rewritten. | R12's standard, applied before it is found again. | Not stated (a fix round saved); a figure that does not reproduce is reported, not adjusted. | `61f3dff1`, `02d06772`. |
| R15 | The Chinatown heading is singular at one (`up to 1 exchange`), and so is Task 6's asking line. | The implementer judged one exchange unreachable, but its own power-1 fixture lists seven one-exchange sets. | One small commit. | `0d64aa53`. |
| R16 | Task 2's I-1 and I-2 are accepted. An eleventh cell checks that every listed set's preview equals `describe`'s words of that move. The false "the engine lists each set in row order" is corrected, and cell 2 asserts that `[YenToSilk, SilkToYen]` is listed. | 10 of 102 sets are out of row order (measured); the false premise stays in `02d06772`'s message. | One fix round. | `0988e19c`. |
| R17 | A false mutant mapping in the plan is corrected as a docs commit. | The plan is the orchestrator's; no code moves. | No cost. | `2e0df268`. |
| R18 | Task 3 edits the production-door kept list and its cell (`[26, 2187, 2213]` → `[26, 2202, 2228]`) and adds both files to the plan's list. No kept row's text changes and none is added or dropped. | They pin the walk script the task regenerates; the plan and its review missed them. | A reviewed list moves unread (the reviewer re-derived it). | `896f9bc5`. |
| R21 | The tree-scanning cells of `lane-variables.test.mjs` get an explicit 30 s timeout. | A 5,000 ms limit read red three times in one session (5,684 ms under load). | A few lines outside the plan's list; one revert if the owner wants it at the wrap. | `cc5d0879`. |
| R20 | A third Church capture, `church_legal`, is added: the first `church` prompt whose preview is legal. The corpus goes to 114. | No corpus Church state had a legal preview. | One fixture, and a STOP if the kept-rows pin moves (it did not). | `3b8a258c`, `4af34500`. |
| R22 | `issue_position()` itself becomes the valid state, with faith 4–6 held by the other seat, and its copy goes away. | A committed payment from the old fixture locked the Session (invariant I9). | A re-run of Task 1's cells, which must read as before. | `3b8a258c`. |
| R23 | The plan's mutant sentence for `church_legal` is corrected; it had named the wrong catcher. | The orchestrator's dictated sentence was wrong. | Not stated. | `c6890ebc`. |
| R24 | **No ruling of this number appears in the ledger.** | — | — | — |
| R25 | The corpus walk grows no Chinatown branch. It refuses a folded Chinatown prompt on its 2P table with a panic that says why, and the `wire4` route sends its lines explicitly. | Its game has no Chinatown, so the branch would be code no seed runs. | If a later corpus walks a bigger table, the branch is written then. | `fff28994`. |
| R19 | The six `wire4` lines join the production-door kept list (26 → 32). The old 26 are unchanged by content, and the reviewer reads the added rows one by one. The question was opened at Task 3 and settled at Task 4's dispatch. **The owner accepted it on 2026-11-22**, the live replay's consequence included. | A live replay expires its walk tables before and after every run, so lines on its own solo table are repeatable. | The set an S1b review read changes in content; the alternative is a rule change in a security file. The Task 4 review found that a production-door run before the deploy now fails by design. | `fff28994`; the owner's word in `f46e8f76`. |
| R26 | Task 4's contract cells state per fixture what each is. | R20's standard. | Not stated. | `fff28994`. |
| R27 | The Worker cell's title and comment are reworded to what is true: the 26 rows the S1b review read and the six the walk plays at `wire4`. | R19's condition was too narrow. | One line each. | `4bd7c31d`. |
| R28 | The two 2P whole-game drivers' `chinatown` arms become refusals that say why, and the plan says where Chinatown is covered. | R25's rule. | Not stated (a 3P game was owed, and plan 2b's Task 7 built it). | `4bd7c31d`. |
| R29 | A suite is run alone, never beside another. A red under a self-made parallel run is re-run alone with its output saved, and is a finding only if it shows alone. | Four Worker script cells read red beside a running `cargo test`. | A real intermittent hiding behind "it was load". | Every later dispatch. |
| R30 | Each Church kind button carries `aria-label` exactly `{words}, you have {have}`, and `tokenLabel(good, amount)` is exported. | The button read its token's label and then its words, twice over. | One small commit. | `3774afe0`. |
| R32 | Task 5's I-1 is accepted: the cells keep the real fixtures and gain one-field clones. The fault is recorded as the orchestrator's: R20 took a real fixture for a clone. | A real fixture co-varies its facts, so the nearest client rule passed. | One fix round. | `1decefc4`. |
| R33 | Task 6's cells each gain a one-field clone or a second fixture reading, set before dispatch. | R32's lesson, applied before the review finds it again. | Nothing if right. | `0554adb6`. |
| R31 | In Chinatown's panel a row's pictures sit in one `aria-hidden` wrapper, and each stock entry is one `role="img"` node. | R30's rule that a thing is read once. | One attribute, if the owner would rather hear the pictures. | `0554adb6`. |
| R34 | Task 6's I-1 is accepted: two clones, one fact each, and `Tokens` gains `decorative`. | The commit's "one-field clones" was false of cell 1. | One fix round. | `3b19031e`, `6f1d45cd`. |
| R36 | The plan's Task 6 cell 2 is corrected: the row is drawn last under Sell, not last of all. | The re-review's new Minor. | Not stated. | `b8eee575` (from git; the ledger gives no hash). |
| R35 | Task 8's lines are chosen so that a wrong restore reads differently. **They were wrong as dictated**: the entry's own `1` repeated in the list, so the implementer changed cell 1's lines. | Made at Task 7's dispatch, for Task 8. | Not stated; the implementer's probe caught it. | Corrected in `b06a81b3`. |
| R37 | `hasMove` in `web/e2e/shared_table.ts` counts an open chooser, pinned in cell 5. | The settle wait's defect, in a second harness predicate; a two-browser game idled at its first Church. | A walk could act on a screen that is not the mover's. | `9c2398b4`. |
| R38 | The landscape defect is fixed in this task, cell first, in `web/src/styles.css` alone. At 844 × 390 neither chooser's Confirm or Cancel could be reached. The plan's STOP is lifted for that file. | A player who opened a chooser there could not leave it. | Reverted if the portrait or desktop look moved (the boxes read equal). | `d12b910a`. |
| R39 | The kinds' focus ring stays inside the button as `2px solid var(--fg)` at `outline-offset: -5px`, clear of the pressed band. Its rider adds cell 6 at 667 × 375. | R38 made the kinds' box a scroll container, and the moved ring could not be told from selection. | The boxes move by 3 px if the owner prefers an outside ring. | `3da87cf6`. |
| R40 | The plan's Task 7 cell 6 is corrected on how it scrolls. | The re-review's N-1. | Not stated. | `b22f6b72` (from git). |
| R42 | The plan's fourth Task 8 mutant is corrected: it is equivalent. | A refused line leaves no trace. | Not stated. | `69b585ee`. |
| R41 | Task 8's three commit messages are rewritten before push or review: the `Claude-Session:` line is removed and the co-author trailer set to the plan's rule, with the trees byte-identical. **R50 corrected its trailer half.** The owner confirmed the session-line half on 2026-11-22: no session address in a commit message. | Session addresses are kept out of what is published, and a new commit cannot take a line out of an old message. | Breaks "never an amend" on purpose. | `a367fdd6`, `b06a81b3`, `30ed9379`. |
| R43 | Task 8's I-1 is accepted: both `confirm` halves assert the reply's line by value. | Neither half told the named move from the nearest wrong one. | Not stated. | `1f965abe`. |
| R44 | The help's S1 and S2 are rewritten, the plan's Task 9 first. | Both sentences were the orchestrator's and overstated. | One fix round. | `0d237fcd`. |
| — | The whole-branch review is a Workflow of three reviewers with no refuters, and the orchestrator synthesizes. | The plan's size and the owner's cap of 2026-10-26 (at most eight agents, about three for a light cycle). | Not stated. | `wf_b43b0f62-a8c`. |
| R45 | While a request is in flight, a chooser's kinds and steppers are `aria-disabled`, with a focus hand-off. Confirm and Cancel stay `disabled`, and the older arms are untouched. | A `disabled` control dropped keyboard focus to the page. | The owner reads the feel on the private Worker. | `bc5ff55b`. |
| R46 | The wire reviewer's W-1 (marked for 2b) is ruled into the wave: a fallback arm for a prompt kind the client does not draw. | The first client that needs it is this branch's. | One small arm and a cell. | `bc5ff55b`. |
| R47 | The first half of the lead's L11 (marked for 2b) is ruled into the wave: `prompt_kind` answers without building the model. | A false comment on a path every press takes. | Not stated. | `20774e8c`. |
| R48 | Four review findings marked "later" (W-5, W-6, F5, F4) are ruled into the wave. | Each closes a gap of a kind the project's lessons name. | A larger wave. | `20774e8c`, `b1618071`, `bc5ff55b`. |
| R49 | The release window for an open 0.7.3 tab is recorded and the decision left to the owner. **He decided it on 2026-11-22**: old tabs are not expected to stay open, so a required refresh is accepted, and a later mechanic to force refreshes is issue #78. | A release reads the window where it is recorded. | The owner's. | `32516422`; the decision in `f46e8f76`. |
| R50 | A commit's trailer names the model that wrote it. The plan's constraint was wrong against the project's rule, and the 41 commits before the wave stay as pushed. The trailers' model names were left as pushed, not ruled. | The security cycle's rule (R-12), checked by content. | 41 commits name the orchestrator's model for Sonnet's or Opus's work (§1 records who built what). | `21bf5dbc`. |
| R51 | The re-review's residue is documents only, corrected in one orchestrator docs commit, and four items are parked. | A sentence corrected by its document's author needs no implementer. | The document suites and the owner's read are the net. | `04742e87`. |
| — | The ledger workspace is kept, against the skill's finish. | The close takes its register from it. | Not stated. | — |
| — | At the private deploy the live replay is not run. | It forces tables and writes saves; the owner's word was the deploy. | Not stated. | — |
| R52 | The deploy's reading uses a solo four-player table at the walk's seed and lines, held against the committed fixtures. | The recipe's table would show the deploy alive, not the change. | One abandoned table in the private archive. | `1117b1d0` (from git). |

**Plan 2a, recorded beside the rulings (not rulings):** the branch was pushed to the lab at `c6890ebc` before Task 3's review, against R5. Commit `02d06772`'s message states a premise its review falsified (R16). For the owner's reading: the pressed kind's `.have` text at 4.5005:1; the 28 px steppers; Chinatown's row 9 cut by its ellipsis at 390 px; one exchange at a time at 844 × 390; the focus ring's shape (R39).

### Plan 2b (orchestrator Mythos 5.2)

| id | what was decided | why | what it costs if wrong | where it landed |
|---|---|---|---|---|
| P1 (before the run) | The Port entry reads `Port — take 1 order card ({r} in reach)` when no listed move takes two, and `1 or 2` only when one does. | The count is read off the list, as 2a's R15 made Chinatown's heading singular. | A second entry wording. | The plan. |
| P2 | The `port` arm's cards carry `printed` and `words`. | The client then reads nothing from the port strip, which a reader can switch off. | A larger arm. | The plan. |
| P3 | The preview is `ways`: a list of none, one or two `describe_move` lines. | Two listed moves can take the same cards. | Not stated. | The plan. |
| P4 | `?` inside the Port's chooser is refused. **Reversed by P10.** | Design §9 left it to the builder. | Not stated. | — |
| P5 | The Port captures are taken at the walk's last folded Port prompt (event 1024, four cards in reach). | At the first, a pair leaves no unselected card. | Not stated. | The plan. |
| P6 | The corpus holds no capture of the payment pick; it is proved at built positions and by Task 7's 3P game. | The walk's game never draws a pair payable both ways (measured). | Not stated. | The plan. |
| P7 | The terminal's flag line is a line per seat, with a fallback. **Superseded**: neither fits the frame (P9, then D). | — | The fallback stated. | — |
| P8 | Plan 2b's branch is cut from 2a's head. | Design §10. | Not stated. | The branch. |
| P9 | The frame finding (C1) is the owner's to rule, and the plan sets out choices A, B and C with the measured table. **Ruled D.** | The frame's own record leaves the row's cost to the owner. | Task 8 re-planned if B or C. | The plan. |
| P10 | The measuring review is folded with these calls: P4 reversed, so `?` opens the order index; the older choosers' half-true refusal lines recorded, not changed; the drivers add a card on a draw before they confirm; Task 2 stays one task. | The reviewer's measurements. | Not stated. | `e71c16df`. |
| P11 | The plan proposes choice D: the `players` view in full and the counts on the dashboard's Mercenaries row. | The owner leaned to the view and found the dashboard useful (2026-11-22); D gives both at no frame row. | Task 8's dashboard half dropped. | The plan. |
| D (the owner's) | Shown the mock in a real terminal, the owner ruled for the Mercenaries row (2026-11-22). | His ruling. | — | `a2d1f550`. |
| P12 | The owner's direction of 2026-11-22 is read as follows. This run deploys the reviewed head to the private Worker without asking again. The verification reports a line it cannot read as not read. On a pass only, it writes the close, `CLAUDE.md`'s lines, the board and the wrap's handover. Nothing goes on origin, no tag, no version bump, no production host. | His direction: the row in the plan, then a handover for a clean session with a private deploy and its verification, and on a pass the documents a separate session needs for main and production. | One private deploy with a recorded rollback, or one more question at the wrap. | `a2d1f550`. |
| F1 (pre-flight) | The pre-flight re-measures no figure and executes no cell. | The measuring review reproduced every figure on code identical to the head. | Task 1's first cell reads red at once. | The ledger. |
| F2 | Task 2 may add fixtures to `chooser::test_support` or build them in `repl.rs`'s tests. | Its positions are not among Task 1's fixtures. | Nothing. | `e5973fe9`. |
| F3 | Task 7's two timing cells print and assert nothing. | Design §9: measured and recorded, not bounded; a wall-clock assertion flakes. | Two tests that cannot fail. | Task 7's dispatch. |
| F4 | Task 1's cell 7 gains an assertion that a Port A card in `selected` changes nothing at Port B. | The interface sentence had no cell. | A line. | `2815e0d8`. |
| F5 | `chooser.rs` and `client.rs` are in Task 3's scope, test code only. | The steps govern the list. | Not stated. | Task 3's dispatch. |
| T1-a | Task 1's I1 and I2 are fixed as measured, with the plan's cells corrected first. | A cell must separate its rival. | Nothing. | `10644d2d`, `df45a54b`. |
| T1-b | Task 1's Minors m1, m2, m3, m5 and m6 ride the fix commit. | A deferred surviving mutant would be re-found. | A wider re-review. | `df45a54b`. |
| T2-a | The piped-dogfood floors stand on the all-human game only. | The vs-bot game's seed is OS entropy, and it measured 0/0/0. | The vs-bot game asserts nothing about a chooser. | `e5973fe9`. |
| T2-b | The guide's Port screen is tied in `chooser.rs`'s guide-tie cell, not in `tests/guide_sync.rs`. | The integration test cannot reach `cfg(test)` fixtures. | Nothing. | `e5973fe9`. |
| T2-c | Task 2's three Importants are fixed, with the plan's cells corrected first. | Three surviving mutants. | Nothing. | `18006f21`, `dc7cd552`. |
| T2-d | Three Minor sentences ride the fix. | Each was false or misleading. | A wider re-review. | `dc7cd552`. |
| T3-a | No change to the two identical pick rows that same-surcharge copies show. **Reversed by the owner's word and replaced by T3-e.** | The rows keep `describe_move`'s words. | A player may see two identical rows. | — |
| S1 | Task 4 is dispatched while Task 3's review runs, with one implementer in the checkout at a time. | Disjoint files. | A round re-sequenced. | The run. |
| S2 | Task 6 is dispatched ahead of Task 5, and Task 5 waits for Task 4's review. | A panel fix would move what the walks read. | As S1. | The run. |
| T3-b | Task 3's I-1 is fixed in the sweep: the main list is checked against a by-hand layout, with a floor. | The entry's place was unpinned. | Not stated. | `c3990ce7`, `2a54094f`. |
| T3-c | Task 3's I-2 is fixed, with the reviewer's sentences tried clause by clause. Its half (b) stayed open (T3-f). | Dictated lines were wrong twice in plan 2a. | Not stated. | `2a54094f`. |
| T3-d | Three Minors ride; m-2 was later withdrawn by its reviewer. | The plan asks for a floor. | Not stated. | `2a54094f`. |
| T4-a | Task 4's I-1 is fixed by a one-field clone. Its I-2: the zones stand in the payload's first-named order. | Design §5.6: the client reads no rule. | A small rework; no visible change. | `01cf70e6`, `799cae88`. |
| T4-b | The empty selection's verdict is silent, as the mockup's is. **Its citation was false** (see the next-but-four row). | The hint line is the terminal's, not among the browser's words. | No hint in the panel; the owner reads it. | `01cf70e6`, `799cae88`. |
| T4-c | Four Minors ride; one plan mutant line is corrected. | — | Not stated. | `799cae88`, `01cf70e6`. |
| T3-e | **T3-a reversed on the owner's word** (2026-11-22: a player must know which of two copies a row takes, so some marker is needed). New Task 3b: ` (slot N)` after the name, only when the acting board holds another copy, in `describe_move`. | The number matches the `labs` view and the lab strip, and gives one wording everywhere. | One small task and a regeneration for another form. | `aa056b61`, `aaf0ff9e`. |
| S3 | The lab push carries interleaved commits while their re-reviews run. | The lab is the private working remote. | Nothing. | The run. |
| T3-f | Task 3's open I-2(b) is closed through Task 3b. | A second rewording would be rewritten an hour later. | Task 3 stays open until 3b lands. | `aaf0ff9e`. |
| The orchestrator's error | T4-b said the silent verdict matches "the Church's and Chinatown's panels". The Church's panel has no silent case. The citation was not checked by content. | — | A test comment and `799cae88`'s message repeat it. | Plan corrected in `a6a25304`; the comment in `5ecad6f0` (F11). |
| T8-a | The Mercenaries row's two further yields are accepted, and the plan says which part the owner ruled. | 125 of 49,575 swept rows reach the last step. | At such counts a four-player row shows no flags. | `dec22f48`, `a6a25304`, `eace0f0d`. |
| — | The owner's direction of 2026-11-22 on the end, read as follows. The wrap's handover opens with a joint verification, his reading of the private Worker beside the orchestrator's. Only then does that session package the branch and release. | His direction. | Not stated. | Step 5's handover. |
| T5-a | The Port's card face is compact. Every card is whole at 390 × 844; at 1440 × 900 each cut card is brought into view inside the deck. | Four readable cards cannot fit a 45vh rail. | Cards scroll inside the answer bar on a desktop. | `0982fb8b`, `40126d9a`. |
| T3b-a | The claim is narrowed and the web's wording is not changed. The browser's Suggest and Why later steps are worded at the root. **Sharpened by the whole-branch review (FW-a F1, then FW-b).** | Stepping the state is a task of its own. | A later Suggest step over a twin may name the wrong slot or none. | `40126d9a`, `5bf15f6a`, `4946f827`. |
| T3b-b | Task 3b's I-2 is fixed: the acting board is told from the President's board. | They differ only in a mercenary sub-turn. | Not stated. | `5bf15f6a`. |
| T5-b | Scripted presses use coins that would land on a pressed card, and the policy's answers get a floor in driven rounds. | A floor on a random walk's visits would flake. | Not stated. | `7b94f4cb`. |
| T5-c | Every scroller in a card's clip chain is asserted scrollable, and a real wheel gesture is used. | A scripted scroll moves a box a player cannot scroll. | Not stated. | `7b94f4cb`. |
| T5-d | A short-window rule keeps a card whole at 640 × 360. | 640 × 360 is a common landscape window. | A smaller thumbnail on short windows. | `7b94f4cb`. |
| T5-e | Cell 4 asserts the second card instead of skipping. | The cell cannot go silent. | Not stated. | `7b94f4cb`, `8f2209d4`. |
| T7-a | The 2P arms' tally counts what the server took. | A figure counting refused lines is false. | Not stated. | `7d7bc322`. |
| T7-b | The Chinatown arm adds on a draw before it confirms, a sixth floor is added, and the arm is re-seeded (seed 12). | The old arm could never build a set of two, so the gate proved the fold, not the chooser. | A re-measured seed, cap and counts. | `7d7bc322`, `8c64c3c0`, `30c7fd21`. |
| T7-c | Task 7's I-1 is fixed by assertions on committed fixtures, and seven Minors ride. | — | Not stated. | `7d7bc322`, `8c64c3c0`. |
| T7's mutant line (one row for three corrections) | The plan's sentence was corrected three times. First to the reviewer's measured sentence (`30c7fd21`). Then to claim only what both runs read (T7-d, `c09f0de3`). Then to "every mutant finishes and a floor reads red" at the new cap (`4a08cd92`). | Which assertion fires depends on the seed and the cap. | Not stated. | `30c7fd21`, `c09f0de3`, `4a08cd92`. |
| T7-d | Inside the row above; seed 12 stands. | — | Not stated. | `c09f0de3`. |
| T9-a | The help and the guide gain a fourth sentence, P4, for the slot marker. | A player-visible change the owner asked for belongs in the help. | One sentence in two documents. | `c09f0de3`, `dddd65cf`. |
| T7-e | The cap is sized from the longest healthy game: 25,000, three times 8,194. | 7,000 was three times the shortest game. | A degenerate arm reads red later. | `d1f5cd8d`, `b8708c83`. |
| T9-b | §1's exception sentence gains the terminal's flag counts. | It records every place the cycle moved the CLI on purpose. | Not stated. | `7f2da843`. |
| T9-c | Task 9's three Importants are fixed and three Minors ride; round 2 then fixes two clauses. | Three false or incomplete sentences. | Not stated. | `7f2da843`, `b2c56288`. |
| S4 | The whole-branch review is launched while Task 9's last one-sentence re-review is out. | The head's code was final. | The lead may re-find it. | `wf_da7b858f-344`. |
| FW-a | One fix wave of thirteen items (F1–F13). Eight items are left out and recorded (§7). | The review's findings, each on its own host. | The three cells widen the wave. | `fc9be3f0`, `5ecad6f0`. |
| FW-b | The re-review's N-1 (the advisor sentence lists two outcomes where three exist) is corrected in the plan. The `describe.rs` comment is parked as true but incomplete. | No second wave. | A deferred minor. | `4946f827`. |
| FW-c | The re-review's N-2 (a stylesheet comment's range is framed as measured) is parked. | Cosmetic; no rule reads it. | A deferred minor. | — |
| S5 | While the verification waits on a sign-in that only the owner can make, step 5's documents are prepared as drafts outside the tree, and none is committed. | Nothing of step 5 reaches the tree before a pass. | Wasted drafting, nothing to undo. | Outside the tree. |

**Plan 2b, recorded beside the rulings (not rulings):** the older choosers' "(cancel first to use views/help)" lines are half true (P10). Design §5.5's "told apart by price exactly as they are today" holds only as "as well as today" (Task 3's review). Task 5's commit messages `0982fb8b` and `7b94f4cb` cite issue #76 where the choosers' issue is #77 (Task 9's review). The commit messages `aaf0ff9e` and `799cae88` overstate (the deferred list). The owner's readings are listed in §8.

### Every ruling id the tree cites, resolved

The tree read at `4946f827`, `docs/plans/` excluded, on every line added since `5abce3e3` (the fix wave's commits included).

| id | one place cited | resolved by |
|---|---|---|
| R19 | `docs/web-lane.md:62` | plan 2a, R19 |
| R20 | `docs/web-lane.md:62` | plan 2a, R20 |
| R25 | `crates/yokohama-web/tests/wire_fixtures.rs:1571` (also `docs/web-lane.md:62`) | plan 2a, R25 |
| R28 | `crates/yokohama-web/src/lib.rs:8604` | plan 2a, R28 |
| R37 | `web/e2e/shared_table.ts:55` (also `web/e2e/the_table.spec.ts:1595`) | plan 2a, R37 |
| R38 | `web/src/styles.css:938` (also `:961`, `:987`; `the_table.spec.ts:1665`, `:1670`) | plan 2a, R38 |
| R39 | `web/src/styles.css:938` (also `the_table.spec.ts:1314`, `:1644`) | plan 2a, R39 |
| R45 | `web/src/Decision.tsx:191` (and 14 more lines across `Decision.tsx`, `Decision.test.tsx`, `harness.ts`, `the_table.spec.ts` and `styles.css`) | plan 2a, R45 |
| R46 | `web/src/Decision.tsx:134` (also `:549`; `Decision.test.tsx:1067`; `docs/web-lane.md:373`) | plan 2a, R46 |
| F2 | `crates/yokohama-cli/src/repl.rs:10818` | plan 2b, F2 (the pre-flight's, not the fix wave's item F2, by content) |
| T3b-a | `crates/yokohama-cli/src/describe.rs:689` | plan 2b, T3b-a |
| T5-a | `web/src/styles.css:1014` (also `the_table.spec.ts:1744`, `:1823`) | plan 2b, T5-a |
| T5-b | `web/e2e/the_table.spec.ts:2207` | plan 2b, T5-b |
| T5-c | `web/e2e/the_table.spec.ts:1752` (also `:1888`, `:1970`) | plan 2b, T5-c |
| T5-d | `web/src/styles.css:1054` (also `the_table.spec.ts:1647`) | plan 2b, T5-d |
| T5-e | `web/e2e/the_table.spec.ts:2102` | plan 2b, T5-e |
| D | `crates/yokohama-cli/src/panel_render.rs:611` ("the owner's ruling D") | plan 2b, D |

**Unresolved: none.** Several other matches are not this cycle's ruling ids, judged by each line's context:

- R2 is Cloudflare's R2 storage.
- S1, S2 and S3 in `docs/web-lane.md` and `worker/RUNBOOK.md` are the security cycles.
- S1 and S2 in `web/src/Help.test.tsx` are plan 2a's help sentences, and P1 there (`:398`) is plan 2b's help sentence.
- D1 (`crates/yokohama-cli/src/describe.rs:757`) is the 2026-08-08 parity ruling, on a moved older comment.
- R07 (`crates/yokohama-cli/src/panel_render.rs:2962`) is a Task 8 review mutant.
- The hyphenated ids (R-OWN-F1 and the like) belong to other cycles.

Plan 2a's own list of "the rulings this tree cites by id" names R18, which no tracked file outside `docs/plans/` cites; plan 2b cites it.

## 6. What plan 2a's last section recorded for the close

**"Recorded for the cycle's close or the wrap"**, item by item:

- **The register of rulings.** Done: §5 of this close.
- **"Every model is `screen_model`'s, every word `describe`'s" is no longer exact** in `docs/web-lane.md` (`:147`) and `CLAUDE.md`, since the choosers' models are `chooser`'s. `CLAUDE.md`'s sentence is made exact in the commit that carries this close (plan 2b's "After the tasks", step 5), with its grammars line. The lane document's is not on step 5's list, so it is the wrap's.
- **`repl.rs`'s size.** Simply recorded. It is 12,008 lines at this head, and plan 2b moved no cells out of it (plan 2a's review lead disagreed with moving them in 2b). It is a later cycle's.
- **The Chinatown sweep's name says "every listed move"** where it samples the larger families. Still open: the cell is `every_listed_chinatown_move_commits_through_the_chooser` at this head. Simply recorded.
- **The amount 1 for five of the Church's kinds is an unpinned display literal.** Simply recorded.
- **`hasMove` on a watcher's page** (older than plan 2a). Simply recorded.
- **Two tabs on one chooser are told apart by prompt kind only**, so a stale tab's Confirm commits what the other tab selected (the same class as the placement picker). Simply recorded; the Port's chooser is of the same class.
- **The older answer-bar arms still drop keyboard focus when pressed.** Still open, for the wrap: plan 2a's ledger names it a candidate issue on origin.
- **A same-tick double press on two rows can lose a focus hand-off** (pointer only). Simply recorded (parked by R51).
- **The browser's default ring on the chooser node** when a hand-off falls to it. Simply recorded (parked by R51).
- **The fallback's Cancel is drawn for any seated viewer.** Simply recorded (parked by R51).
- **`docs/web-lane.md` §2's suite counts.** Re-counted in plan 2b's Task 9 at `c09f0de3`. They are to be re-read at the final tree by the wrap.
- **Commit `02d06772`'s message** states a premise its review falsified. Left as committed; listed in §10.

**"Left as they are":** the trailers of plan 2a's commits before its fix wave name Mythos 5.2 whichever model wrote them, and the close is to record who built what. Done: §1's implementer columns.

**"Owed by plan 2b"**, for completeness:

- A whole game through the web layer on a table with a Chinatown: Task 7.
- Design §9's press cost, measured and recorded: Task 7, as readings in `docs/web-lane.md` §2.
- One helper for "close the chooser, then commit": Task 2's `leave_chooser`.
- An `isChooser` helper: Task 4.
- A floor on the piped-dogfood gate's chooser count: Task 2, the all-human game only (T2-a). Task 3 added a Laboratory floor.

## 7. Deferred, with nothing built on them

For the wrap's close or a later cycle to take up or drop. Everything here is plan 2b's; plan 2a's are in §6.

- **The `<figure>` inside a `<button>`** in the Port's panel (`web/src/Decision.tsx`): invalid HTML, plan-mandated; React prints no warning and the accessible name is the `aria-label`. Changing the element moves boxes the layout cells measured. The wrap's, with the owner's reading.
- **Task 5 (1).** The rounds' floor counts answers made with a card on, not presses made with one (`web/e2e/the_table.spec.ts`).
- **Task 5 (4).** The short-window narrowing of the heading column and the face's gap is pinned only on tables that deal a card whose chips wrap. The layout cells run on random tables, so their ranges are runs, not bounds (`web/e2e/the_table.spec.ts`, `web/src/styles.css`).
- **Task 6 (3).** A redundant rebinding in `crates/yokohama-worker/src/picker.rs`'s tests.
- **Task 6 (4).** The Port confirm's take is pinned by substrings where plan 2a's cells pin the message by value (`crates/yokohama-worker/src/picker.rs`'s tests).
- **Task 8 (m6).** A redundant render in the yield loop when no count is zero (`crates/yokohama-cli/src/panel_render.rs`).
- **FW-b.** `crates/yokohama-cli/src/describe.rs:683–691`'s comment gives two outcomes for an advisor step worded at the root. A third exists: the root's marker can be right. The comment is true as far as it goes. The plan's sentence is corrected in `4946f827`.
- **FW-c.** `web/src/styles.css:1030–1031` (and `:1084`) gives "220 to 260 px" at 390 × 844 under a paragraph that frames its ranges as what the runs read. The figure is the first run's range plus 1 px, with one measured reading.
- **The wire reviewer's m-1.** No pool cell restores a chooser across a real eviction through the compiled Worker module (`worker/test/`). The Port's restore is pinned natively in `picker.rs`, and the shell's round trip only with Option A placement (`worker/test/table.test.ts:138`). Plan 2a's choosers follow the same pattern.
- **Recorded for the close by the run:** the browser advisor's root-state wording (`crates/yokohama-web/src/explain.rs`; §8). No whole game pays a Port pair with the imported good (§8). No browser cell opens the two-way payment pick (§8). Six Port cards in reach were not measured in a browser (§8). Plan 2a's two refusal lines read "(cancel first to use views/help)", which is half true, since `?`, `ports` and `hand` open from inside (`crates/yokohama-cli/src/repl.rs`). The commit messages that overstate (§10).
- **Recorded by Task reviews, no change:** Task 1 m7 (`max_cards` reads the list a second time where Step 3 says the map; equivalent). Task 4 M-3 (`{n} ways to pay` pinned only at 2, the most a pair has). Task 4 M-4 (one clone changes two fields, plan-mandated). Task 7's re-review: a game that never ends would cost about 5 minutes of debug before the cap reads red.

## 8. What this cycle does NOT claim

- **The browser's Suggest and Why word a line's later steps at the position the line starts from** (`crates/yokohama-web/src/explain.rs`, unchanged by ruling T3b-a). Where both copies of a technology are on the board, such a step can read without its slot, with the right one, or with a stale one. The lead measured the stale slot on a built position. The reach in a real game (a mercenary acts at a Laboratory, then the President acquires there in the same turn) is REASONED from the move generator, not played. The row the player actually presses always names its slot.
- **No whole game through the web layer pays for a second Port card with an imported good.** At the three-player gate's seed every payment pick drew the first row.
- **No browser cell opens the two-way payment pick.** No pair payable both ways is reachable by lines at the opening.
- **A table with six Port cards in reach was not measured in a browser.** The cells' tables reach four.
- **The Port's stranding of a 0.7.3 tab is reasoned.** Plan 2a's two prompt kinds were measured, by rendering the 0.7.3 client's `Decision.tsx` under vitest. The Port's is the same mechanism read off the code; the wire reviewer could not read 0.7.3's client.
- **The three-player gate's readings are one seed's** (seed 12: 8 Chinatown confirms, 6 of them sets of two, 11 Port confirms, 3 pairs, 1 Port payment pick and 2 Laboratory picks over 2,141 inputs). Five seeds of the 31 swept reach all six floors.
- **The press costs are readings on one loaded machine, not bounds** (`docs/web-lane.md` §2: the MacBook Air while other agents ran suites, load average 9 to 12).
- **How the Port's cards look was read by no agent**, since no agent opens an image. The layout cells measure boxes, not faces. It is the owner's reading.
- **Design §5.5's "so the copies are told apart by price exactly as they are today"** is true only as "as well as before": copies in slots that add the same read alike, and so do copies whose surcharges the action's power covers. Task 3b's slot marker is what tells them apart now.
- **The verification on the private test Worker** is recorded in §4, and nothing else here is on a host. **How a browser holding the earlier release takes this one was not read on a real host**: the browser cells that pin the update path are green (`web/e2e/offline.spec.ts`), and on the host it is the owner's reading.
- **The roster's 521 s is a reading under load**, not a comparison.
- **The owner's to read**, each made on his behalf during the run:
  - the compact card face, and its scroll inside the answer bar at 1440 × 900 (T5-a);
  - the short-window rule, which also shrinks cards at 667 × 375 (T5-d);
  - the Mercenaries row's two further yields at a four-player seat (T8-a);
  - the silent empty verdict in the Port's panel (T4-b);
  - the entry's "ways to pay" and the pick's "choose the payment" over a twin whose rows are two cards (the lead's M-8; a change of words would regenerate the corpus);
  - the slot marker's form, ` (slot N)` shown only when both copies are on the acting board (T3-e).

## 9. Lessons, for the wrap to weigh for `CLAUDE.md`

- **A ruling's citation is checked by content before it is ruled on, the orchestrator's own included.** T4-b cited "the Church's and Chinatown's panels" for a silent verdict the Church's panel does not have. Cycle 1's lesson named a reviewer's citation; this one was the ruler's.
- **A sentence about which assertion fires under a mutant holds at one seed and one cap.** Task 7's mutant line was corrected three times. Write only what every run read, or name the seed and the cap.
- **A cap or a floor is sized from the measurement's extreme, not from the case chosen.** Task 7's cap was three times its chosen seed's game, which was the shortest of 31, so healthy games ran past it (N-1, T7-e).
- **"Scrolled into view" proven by a scripted scroll passes on a box a player cannot scroll.** Assert that every clip in the chain computes `overflow` `auto` or `scroll`, and use a real wheel gesture (Task 5, T5-c).
- **A reading "over a walked game" needs a floor reached by construction.** In two of three clean runs the walk met no Port chooser, so the count it printed proved nothing. Drive the rounds that reach it (Task 5, T5-b).
- **A driver arm that confirms as soon as the preview is legal can never build a multi-part selection**, so the gate proves the fold, not the chooser. An arm adds on a draw while it can, and a floor counts the multi-part sets (Task 7, T7-b).
- **A counter counts what the server took, not what was sent.** A mutant that confirmed every empty selection read 117 Port confirms with none applied, and a Laboratory counter that counted any pick read 20 and passed every floor, since a floor cannot see over-counting (Task 7: `231b6e04`, `0798d73f`, T7-a).
- **Proposed sentences in a fix list are proposals.** Five of the fix wave's thirteen were false when tried (F1, F2, F3, F12, F13), and each was caught because the list said to try every clause against the code.
- **The same display fact worded at two states can disagree.** The terminal's advisor steps the state and the browser's words every step at the root, so the slot marker reads differently on the two (T3b-a, FW-a F1).
- **A reading on a host behind a login needs that login alive before the deploy.** The private deploy landed at 14:50Z and could not be read until the owner re-authenticated WARP over an hour later; one `GET /api/me` before the deploy would have said so. An agent does not sign in for the owner (§4).
- **An implementer that backgrounds a long run and ends its turn stalls** (Task 5). It was woken with "poll the output in the foreground; never end the turn while a run is in flight". Cycle 1 recorded the same stall. A dispatch that says it up front saves the wake.

## 10. What the wrap owes

Everything this cycle leaves for cycle 3, with its source:

- **A joint verification first.** The wrap's session opens with the owner's own reading of the private test Worker (browser and terminal) beside the orchestrator's re-reading. It packages and releases only when both are done (the owner's direction of 2026-11-22; plan 2b's ledger).
- **The pull requests to origin** under the two-repository rule: each branch reaches origin once, review-clean, as the pull request that merges it, master moving only by a merge commit. The owner chooses between one clean branch to main and one pull request per cycle, laid out with costs and a recommendation (plan 2b, "After the tasks" step 5; P12). The three lab branches are `web-evaluator-feedback` (#75, #76), `web-choosers-goods` and `web-choosers-cards` (#77) (the board; cycle 1's close §8).
- **The release number and tag** by `docs/web-lane.md` §7 (plan 2b step 5; cycle 1's close §8).
- **The production deploy on the three tiers** in the owner's window, by the two runbooks, the NAS first as the canary (plan 2b step 5).
- **The live replay from the release's tree.** The kept list and the cited walk rows moved in this cycle (R18, R19). A production-door run against a Worker built before this cycle mismatches by design, and the owner accepted that on 2026-11-22 (plan 2b step 5; `worker/RUNBOOK.md` §7).
- **Every help picture re-cut once on the final tree**, with the claim-band picture decided there. **The Worker README's ten pictures** (cycle 1's Task 9) wait on where they are cut, with its brief's two rules (cycle 1's close §8; plan 2b step 5).
- **The decision-log rows**: the row for the three-cycle direction (the board, Track 6) and the rows the design owes at the wrap (design §7). The plan 2b session's suggestion, sourced from no earlier document: a row for this cycle's deliberate change to the REPL's behaviour, which `docs/web-lane.md` §1 records as the fence's exception.
- **The issues**: close #75, #76 and #77; leave #78 open (the forced-refresh mechanic) (plan 2b step 5).
- **Candidate issues on origin**: the browser advisor's root-state wording (T3b-a), and the older answer-bar arms' loss of keyboard focus (plan 2a's whole-branch review).
- **The lane document re-read at the final tree**: §2's counts (plan 2b step 5; cycle 1's close §5) and `:147`'s "every model is `screen_model`'s" (plan 2a's last section; §6).
- **The commit trailers before plan 2a's fix wave** name Mythos 5.2 whichever model wrote them, and stay as pushed: 41 commits of plan 2a (R50) and cycle 1's 23. Six of cycle 1's fix-wave commits name Opus 5.5 where its plan names Mythos 5.2 (the board).
- **Commit messages that overstate or cite the wrong issue**, left as committed: `02d06772` (plan 2a: a premise its review falsified), `aaf0ff9e` ("the main list, the pick, the log, suggest and why all carry it", true of the terminal only), `799cae88` ("as the Church's and Chinatown's"), and `0982fb8b` and `7b94f4cb` (cite #76; the choosers' issue is #77).
- **§7's deferred list**, cycle 1's close §5, and §8's readings for the owner.
- **What the owner decides there**: one branch or one pull request per cycle; where the README's pictures are cut; whether the help carries a claim-band picture; his readings in §8 (the board).

**Two notes on sources.** Cycle 1's 23 commits before its fix wave and its six fix-wave commits (§10's trailers line) are taken from plan 2a's ledger (R50) and the board, not from cycle 1's close. The commit hashes for R36, R40 and R52 are absent from plan 2a's ledger and were read from `git log` by their subjects (`b8eee575`, `b22f6b72`, `1117b1d0`).
