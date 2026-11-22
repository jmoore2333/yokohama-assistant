# The claim on the card and the flag tally — the lab close (2026-11-21)

**The record of record for cycle 1 of Track 6, as built on the lab.** The design is `docs/plans/2026-11-20-web-claims-and-flags-design.md` (accepted 2026-11-20) and the plan is `docs/plans/2026-11-20-web-claims-and-flags-plan.md`. The run's ledger is the gitignored `.superpowers/sdd/2026-11-20-web-claims-and-flags-plan/progress.md`, with every brief, implementer report and review beside it; this document carries what that ledger holds and git does not.

**Headline.** Cycle 1 is BUILT and REVIEWED on the lab branch `web-evaluator-feedback`, cut from master `e600f5f4`. It is NOT on origin and NOT released: by the owner's direction of 2026-11-21, Track 6 runs out of the lab in three cycles and reaches origin once, at the wrap (cycle 3). The reviewed code head is `dcf882dc`; the five re-cut help pictures and their two alt clauses follow as `2ae2af07`, on the owner's word. A claimed achievement draws its claimants' cubes on the card and lights in the turn walk (issue #75), and every seat's row shows its per-country flag counts, the viewer's own row with the flags still in hand (issue #76). No fenced crate moved and `SEMANTICS_REV` is 7.

**This is a lab close.** The plan's "After the tasks" steps 3 to 6 (the pull request, the release, the README's pictures, the origin-side close and the decision-log row) are the wrap's and are listed in §8. A dated record is frozen once written, so what the wrap does is recorded forward in the wrap's own close, not here.

## 1. By task

Eight tasks, run by subagent-driven development on 2026-11-20 and 2026-11-21. Each had an implementer and an Opus task reviewer who ran the task's mutants on an export.

| Task | What landed | Commits | Implementer | Review |
|---|---|---|---|---|
| 1 | `FlagCounts`, `face_up_flags`, `order_flags`, the two wire fields, the drift pin, the regenerated corpus (27 files, two keys) and the client's contract | `03ff0e74`, `e3860d79` | Opus | clean; 15 mutants caught |
| 2 | The tally on a seat's row, `+n` on the viewer's own row only | `1b5c791a`, `d801be18`, `9196c218` | Sonnet | clean after 2 fix rounds |
| 3 | The constructed fixture `goals_claimed_4p.json` (claim order 2, 0, 3, 1) and the lane document's two counts | `1abbde6c`, `42e63cc6` | Opus | clean; 9 of 12 mutants caught, 3 by design left to Task 4 |
| 4 | The claim band on the card; both one-sentence readings take `role="img"` | `926bcd1d`, `5d0b4587` | Opus | clean after 1 fix round; 26 mutants caught |
| 5 | The turn walk lights the card whose claims grew (`Changes.goals`) | `99bb122d`, `c498aa3b` | Sonnet | clean after 1 fix round |
| 6 | Two layout cells in Chromium: the band inside the picture box, the tally inside its row at 390 px | `c33d80d9`, `56cbe84f`, `f06b5575` | Opus | clean after 2 fix rounds |
| 7 | The words in `Help.tsx` and `docs/web-guide.md` | `c7b2d56e`, `7763e617` | Sonnet | clean after 1 fix round |
| 8 | A text sidecar (`.aria.txt`) beside every generated picture; the README sidecar leaves out what its picture masks | `a8025c5c`, `5fccb844` | Opus | clean after 1 fix round |

Task 8's Steps 2 and 3, the five re-cut pictures under `web/public/help/` with one alt clause each in `web/src/Help.tsx` and `docs/web-guide.md`, waited uncommitted for the owner's reading. The words were reviewed against the text sidecars (105 claims: 62 hold, none false, 43 only a picture can settle). The owner read the pictures on 2026-11-21 and they are committed as `2ae2af07`. The seven files were byte-identical to the copies checksummed before the fix wave.

## 2. The whole-branch review

A Workflow of three reviewers on exports of `5fccb844`: a Fable lead (the whole diff against the design), an Opus mutation battery (the only Playwright runner) and an Opus mask-and-wire reviewer. The lead found no Critical and one Important and called the branch ready after a fix wave. The battery found that every spec-list mutant, task mutant and fix-round survivor fails at the head, with two gaps of its own. The wire reviewer found the mask and the wire sound on every host and the README generator not ready for a live cut.

**The one fix wave** was five commits (`e41827ff`, `5770e6f1`, `38719c7c`, `52bd8ebb`, `04afbedc`):

- **Before merge.** `+n` moved from `--fg-faint` to `--fg-dim`, which reads 6.01 unlit and 5.08 on a walk-lit row (dark scheme), 5.77 and 4.85 (light). A five-country hand in the phone-width cell, so wrap and no-wrap differ. An in-hand hand of two American flags, so table order and count order differ. Index cells with four claims and no art. A `wire_fixtures` cell recounting every seat row's flags. The false sentences found by the sweep.
- **Before the README's live cut.** A folded chat's log block reads `[masked: chat]` in the sidecar. A failed run writes no screenshot, video or trace. The table's password is drawn per run. The guard's exact id, own-address and case cells. A local-only stranger arm (`YOKOHAMA_WEB_README_SHOTS_STRANGER`, refused live).

**The scoped re-review** read 16 items addressed and one not: a locator assertion that fails unmatched carries a snapshot of the whole page into `error-context.md`, which `PLAYWRIGHT_NO_COPY_PROMPT` does not guard. **One bounded follow-up** (`dcf882dc`) settled it: on a run that can meet a stranger, the test body rethrows a fresh `Error` naming only the last picture taken. The reviewer's own probe then read no page text in `error-context.md` or the terminal, a test timeout included.

## 3. The gates at the head

Read at `dcf882dc` on the MacBook Air (Mac17,3, Apple M5), 2026-11-21, 01:22Z to 01:54Z, with the BGA corpus present and the seven picture files in the tree, uncommitted.

- **Per-edit, every command green.** core 1,587 / 29 ignored; cli 869 + 2 ignored; oracle 191; difftest 52 / 5; eval_probe 214; solver 132; bridge 82 and its four node suites; web 610 (lib 567, browser_played_pin 5, lane_document 6, the whole-game gate 2, start_refusal 11, wire_fixtures 19); wasm 10; worker 84; the refusing build's `identity::` slice 9; the `wasm32` check and the Worker's release-wasm build; clippy clean; `gen-data` byte-identical. `cargo doc -p yokohama-worker --no-deps` reads its 2 standing warnings.
- **Web.** typecheck 0; vitest 1,644 in 76 files; `test:wasm` 428 in 20; e2e 85 passed in 9.5 min.
- **Worker.** typecheck 0; `test:scripts` 801 in 33; the pool 302 in 14.
- **The roster: 514 s, 15 of 15**, one uninterrupted sequence with the release binaries pre-built: fuzz 50, census 1, observation 4, action_equiv 31, action_worst 0, arena 23, policy_div 6, evaluator 288, architecture 53, recovery_lib 11, piped_dogfood 11, differential 29, routes3_diff 2, scoring_diff 2, routes3_worst 3. The machine was not quiet (an antivirus scan and the storage services held about two cores, and every gate read about 12% slower), so this is a reading under load that stands beside the 456 s at `e41352a6` on the same machine, never against it. No fenced crate moved.
- **After the pictures' commit** (`2ae2af07`, the same machine, 2026-11-21): the build and `test:wasm` 428 in 20 before it; then typecheck 0, vitest 1,644 in 76 and e2e 85 passed in 8.1 min. No Rust file moved in that commit, so the Rust suites stand as read above.

## 4. Every ruling of the ledger, in the order made

Each was made by the orchestrator on the owner's behalf during the run and is recorded with what it cost.

- **R1.** The three points the board had left open (Task 8's sidecar, Task 9 after the release, Task 6's reply-rewrite cell) ran as written.
- **R2.** Report files take the plan's names, not the skill's default.
- **R3.** A task reviewer runs the task's named mutants on its own export and re-runs no other suite. Cost: reviewer wall time.
- **R4.** Task 8 Step 1's sidecar commit gets a task review like any other; Steps 2 and 3 stay uncommitted for the owner.
- **R5.** Task 4's implementer names the picture-box wrapper's class and Task 6's dispatch carries it.
- **R6.** Each export lives under the scratchpad with its own `target/`; reviewers end with `cargo clean` inside the export and never `rm`.
- **R7.** The orchestrator pushes to the lab after each task's review is clean. Nothing goes to origin.
- **R8.** A 55-minute heartbeat of zero-tool fires while agents run.
- **R9.** A client cell whose corpus precondition no payload meets stops the implementer; no payload is built by hand.
- **R10.** A task that moves no Rust file runs the web suites its brief names and the fence check only.
- **R11.** The tally's mark is the country's word alone; token art is not threaded into the seat rows.
- **R12.** The tally's reading is one node with its cells hidden. Superseded by R17 as to the role.
- **R13.** Minors in a file a fix round already edits ride that round and do not count toward the breaker. Cost: a wider re-review.
- **R14.** A count in the lane document names the commit it was read at, and a commit cannot name itself, so such a task lands as two commits.
- **R15.** The claim band carries its own ground and its numbers meet the stylesheet's AA rule in both schemes (8.00:1 measured).
- **R16.** Each cube is hidden from assistive technology and keeps its title; the band's label is the one reading.
- **R17.** Both one-sentence readings (the claim band and the flag row) take `role="img"`, as a tile does: a `group` whose children are all hidden gives a browse-mode reader nothing to enter. A real screen-reader check is the owner's, non-blocking.
- **R18.** The plan's "in the manner of its `techs` cell" was false about the tree; the cell was written fresh and the plan's bullet corrected in the task's commit.
- **R19.** A card absent from the earlier frame's panel is not named as a changed goal.
- **R20.** The `App` to `TableSections` wiring of `changedGoals` had no cell; one entered Task 5's fix round. The `App` to `Dashboard` half is inert today (the dashboard's Achievements panel is never drawn) and was accepted as built.
- **R21.** "Inside" is all four edges with at most 0.5 px of rounding, stated in the test.
- **R22.** A failing measurement on the product as built is a finding: the implementer stops and does not loosen the assertion.
- **R23.** The claim cell gained "each number inside the band" and "on the cubes' line", with the plan's step corrected first.
- **R24.** The help's sentence listing what a walk step marks gained the goal a player has just claimed.
- **R25.** Task 8's uncommitted words got a review against the text sidecars while the owner read the pictures.
- **R26.** `docs/web-guide.md` embeds `screen.webp` with its own alt, so that alt was re-read and held uncommitted with the rest.
- **R27.** The README sidecar wrote strangers' lobby rows unmasked on a live run. Superseded by R28 as to when it was fixed.
- **R28.** That was fixed inside Task 8's own loop: the sidecar leaves out what the picture masks, and a guard refuses a surviving foreign id.
- **R29.** The whole-branch review started without waiting for the owner's reading of the pictures.
- **R30.** Four findings their reviewers marked "later" were ruled into the fix wave, since each was a few lines in a file the wave already edited.
- **R31.** The README generator's hardening was fixed in this branch's wave, not in Task 9's later pull request.
- **R32.** Where the README's pictures are cut is the owner's decision before Task 9 and was left out of the wave.
- **R33.** The five fix-wave commits and the follow-up carry the trailer of the model that wrote them (Opus 5.5) where the plan names Mythos 5.2. A reported commit is never amended, so the mismatch is recorded here.
- **R34.** One bounded follow-up past the single fix wave, for the page snapshot a failed locator assertion carries (§2). Cost: one round beyond the skill's "one wave".
- **R35.** A recommendation: cycle 2 on a branch of its own, cut from cycle 1's head. The cycle-2 design's §10 takes it.
- **R36.** A recommendation: this close, written before cycle 2 builds. Taken by the owner on 2026-11-21.

## 5. Deferred, with nothing built on them

For the wrap's closes to take up or drop:

- **The lane document's §2 counts.** The pool reads 298 there and has read 302 since before this branch; the fix wave's new cells are not counted there either.
- **Test helpers duplicated** between `web/src/Strips.test.tsx` and `web/src/Dashboard.test.tsx` (about 45 lines); their home is `web/src/test/panels.ts`.
- **Cells not written.** The mixed old-server case (`flags` present, `flags_in_hand` absent), which no real server produces. A layout cell for the goals index and for the dormant dashboard panel, where the band can be narrower than the strip's card. A settle step in `shots.spec.ts`. The Dashboard's four-claim cell does not assert the VP chip. An extra key on the four-claim fixture's panel objects passes.
- **Sentences.** `web/src/styles.css:87-93` names 4.87:1 as `--fg-faint`'s worst case; under the light scheme it measures 4.68:1, still AA. The `placement` alt quotes "Option B" where the generator accepts either option. The differ's paragraph gives no reason why a hand card leaving goes unmarked. The README spec's header is not exact for a test timeout.
- **The README generator, recorded and not looped.** The spec-source cell passes a second `test.use`, a one-page `finally`, an aliased password or a shadowed `LIVE`. The guard sees only ids and addresses, so a stranger's free text outside a chat log would pass, and its address pattern is ASCII-only. Two shapes unreachable at the head would lose the run's own rows.
- **Noise.** vitest prints one `localStorage` experimental warning per process. `start_refusal` collided on a port once in the fix wave and not again.
- **Colour.** The walk's pulse starts at a tint where `--fg-dim` reads 2.97 for a moment before easing to 5.08; reduced motion drops it.

## 6. What this cycle does NOT claim

- **No help picture shows a claim band.** No goal was claimed in either photographed game. A picture of one needs a generator change and is the wrap's decision.
- **`screen.webp` shows the `+1` in its old colour.** The fix wave changed it after the re-cut. The wrap re-cuts every picture on the final tree.
- **`strips.webp`'s framing has no text behind it.** Its sidecar is byte-identical to `screen`'s, since a snapshot cannot see scroll.
- **No real screen reader has read the band or the flag row.** jsdom checks the role and the name only.
- **The dashboard panel's lighting of a claimed card is dormant.** Nothing a player sees draws that panel; the strip's is the one pinned at the `App` level.
- **The README generator has never run live.** Its redaction has met a stranger's row only in the local stranger arm.
- **The plan's sentence "those two keys appended" is inexact.** The live wire appends them; the committed corpus sorts its keys.
- **The roster's 514 s is a reading under load**, not a comparison with 456 s.
- **Nothing here is on a host.** The private test Worker's reading of this head is recorded in `worker/RUNBOOK.md` when it is taken.

## 7. Lessons, for the wrap to weigh for `CLAUDE.md`

- **A cell's data must tell the right answer from the nearest wrong one.** "In the table's order" ran on data where three orderings agreed, then on data where order by count agreed. "Inside the picture box" passed for a number the band had clipped. The reviewers' own mutants found each; the plan's named mutants did not.
- **A forced-failure proof must take the real failure path.** A bare `throw` never reached the matcher's page snapshot.
- **A one-node accessible reading takes `role="img"`**, not a `group` whose children are all hidden.
- **A subagent that says it is waiting for a run is stalled.** It was woken at once with the facts.
- **A fix-wave implementer signs with its own model's name** unless the dispatch repeats the trailer line.
- **A reviewer's citation is checked by content.** One named the wrong file.
- **Picture work goes through text.** No agent opens an image, so each generator writes the page's accessibility snapshot beside its picture.

## 8. What is owed, and where

- **The wrap (cycle 3).** The pull request closing #75 and #76 and the release by `docs/web-lane.md` §7. Every help picture re-cut once on the final tree, with the claim-band picture decided there. The README's ten pictures (the plan's Task 9), which wait on where they are cut: the whole-branch review recommends the private test Worker with an empty lobby. Task 9's brief carries two rules: the configured reporter only, never `--reporter html`, `json` or `blob`; and a failed live run's `test-results/` is not read by an agent. The decision-log row for the three-cycle direction. §5's list.
- **Cycle 2** (`docs/plans/2026-11-21-web-area-action-choosers-design.md`). The terminal's flag counts, which the browser has had since this cycle: `screen_model::face_up_flags` and `HandPanel::flags_in_hand` are the counts, and the design's §5.8 puts them in its second plan.
