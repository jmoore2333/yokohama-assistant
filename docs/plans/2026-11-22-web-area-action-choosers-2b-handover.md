# Handover — running plan 2b, the card choosers, from a clean session (2026-11-22)

**What this is.** The entry document for the session that builds plan 2b. It is a pointer: the plan (`docs/plans/2026-11-22-web-area-action-choosers-2b-plan.md`) is the authority for what is built and how it is proved, and the design (`docs/plans/2026-11-21-web-area-action-choosers-design.md`) is the authority the plan argues from. Where this page and either disagree, they win, and the disagreement is a finding to report.

## 1. What you are asked to do, and what you are not

By the owner's direction of 2026-11-22 (his words are in the run's ledger, §4):

1. **Run plan 2b** by subagent-driven development on the lab branch `web-choosers-cards`: its nine tasks, then its "After the tasks" in order.
2. **Review it whole, read the gates**, and **deploy the reviewed head to the private test Worker** (`yokohama-test.moore.nyc`). This deploy is already directed: it needs no further word.
3. **Verify it there**, by the plan's own list.
4. **If, and only if, the verification passes**, write the documents a separate session needs to take the work to main and to production: the cycle's close, `CLAUDE.md`'s lines, the board, and the wrap's handover.

**Not yours, whatever seems convenient:** anything on origin (no push, no pull request, no issue edit beyond what a task names); a tag; a version bump; a production host (`yokohama.moore.nyc`, the NAS); the live replay; a rewrite of pushed history; an edit under the five fenced crates or to `crates/yokohama-web/src/grammar.rs`. The wrap is a separate session with its own handover, which you write last.

## 2. Where things stand

| | |
|---|---|
| `web-evaluator-feedback` | cycle 1 (issues #75, #76), closed on the lab at `5abce3e3` |
| `web-choosers-goods` | cycle 2's plan 2a (the Church and Chinatown): built, reviewed at `04742e87`, gated, head `f46e8f76` |
| `web-choosers-cards` | cycle 2's plan 2b: cut from `f46e8f76` (= the plan's `BASE`); it holds the plan and this page, and nothing built |
| the private test Worker | version `eba04eca`, plan 2a's head, read and accepted by the owner |
| production | `0.7.3`, untouched since 2026-11-10 |

All three branches are on the lab only. The gates at plan 2a's head are on the board (`docs/handover.md`, "Gates at HEAD").

Settled by the owner on 2026-11-22, and not to be reopened: the release accepts the window in which a tab opened before a deploy meets a prompt kind it cannot draw (`docs/web-lane.md` §7), and the mechanic that would make open tabs refresh is issue #78; the production-door replay playing six lines on its own table is accepted; no commit message carries a session address; the terminal shows the flag counts in full in the `players` view and compactly on the dashboard's Mercenaries row (the plan's Task 8).

## 3. Read first, in this order

1. `CLAUDE.md`, then the board's Track 6 (`docs/handover.md`).
2. **The plan, whole.** Its sections "Global constraints", "What was measured before this plan was written" and "The words" bind every task.
3. The design: §2's anti-goals, §3, §5.1, §5.4 to §5.8, §6, §7, §9.
4. Plan 2a's last section, "The run … and what it taught, for plan 2b" (`docs/plans/2026-11-21-web-area-action-choosers-2a-plan.md`): the names this plan builds on and the eight lessons its constraints carry.
5. `docs/web-lane.md` §1, §2, §4 and §7; and, when you reach the deploy, §14's build values and `worker/RUNBOOK.md` §26 with its two records of a lab branch's head.

## 4. The run's workspace

`.superpowers/sdd/2026-11-22-web-area-action-choosers-2b-plan/`, gitignored, on the workstation that wrote the plan:

- `progress.md`, the ledger. Its section "Before the run" holds the owner's words, the calls made while the plan was written (P1 to P12) and the plan review's return. **No task has been dispatched**: the run starts at the pre-flight, then Task 1.
- `plan-review.md`, the independent review that measured the plan on an export (one Critical, five Important, seventeen Minor, all folded; its per-cell verdicts and its list of every site that answers a Port or Laboratory prompt by row are worth reading before Tasks 2 and 3).
- `chooser-mockups.html`, the accepted mockups, read as text: Task 4's reference for the Port panel's structure.
- `probe3.rs`, `probe4.rs`, the throwaway probes behind the plan's figures (they build INVALID states; the plan says how a valid one is built).
- `tools/per-edit.sh` and `tools/roster.sh`, the two gate scripts, with the fence checks set to this plan's `BASE`: copy them into a `gates/` folder of your own scratchpad and run them there, the per-edit one first and the roster after it, never beside it. `tools/read-wire4-chinatown.py`, the script that dealt a table on the private Worker and held its Chinatown chooser replies against the committed fixtures.

Plan 2a's workspace, `.superpowers/sdd/2026-11-21-web-area-action-choosers-2a-plan/`, is kept: the cycle's close takes its register of rulings from both ledgers. If either workspace is missing (another machine, a cleaned tree), the plan and this page are enough to run from; start a new ledger and say what was lost.

## 5. How a run goes here (the owner's standing rules, which the skill does not carry)

- **Tiers.** Each task's header names them: Sonnet where the brief is precise, Opus where judgment is needed and for every task review, Fable to lead the whole-branch review. Name the model in every dispatch.
- **One implementer at a time, in the checkout; every reviewer on an export** (`git archive` of the commit, its own `target/`), never in the checkout. While an implementer holds the tree the orchestrator prepares in its scratchpad and edits nothing.
- **Every agent writes its full report to a file** beside the ledger and replies with the path and a few lines. A dispatch forbids `rm -rf`, `rm -r` and a bulk `rm -f`, forbids waiting on an event or arming a monitor, and says: one suite at a time, a red saved before it is re-run alone.
- **No agent opens an image.** One Playwright runner at a time on the machine.
- **Mutants run three times by design** (the implementer's probe, the task review, the whole-branch review). The pre-flight runs no mutant: it executes the first tasks' cells and re-measures the figures.
- **Read `HEAD` after every DONE.** New commits only. A fix round resumes the implementer that did the task; a message sent to an agent mid-turn is delivered when its turn ends.
- **A commit's trailer names the model that wrote it**, one line, and no session address, whatever the harness suggests.
- **The plan is the single source.** A sentence of it found false is corrected there first, by the orchestrator, as its own docs commit; the ledger records the ruling. Every ruling made on the owner's behalf is collected in the session's last message with what it costs if wrong.
- **Push to the lab after each task's review is clean.** Nothing to origin.
- **The whole-branch review** is a Workflow of about three reviewers (the whole diff against the design; the mutation battery, the only Playwright runner; the wire, the mask and every host), one fix wave, one scoped re-review, no second wave.
- **A long wait** keeps the session's cache warm with a quiet heartbeat that reads nothing; an agent that is working is never messaged.

## 6. What plan 2a's run taught its orchestrator, beyond the plan

- **Test lines dictated in a ruling were wrong twice.** Ask the implementer to probe a dictated line against its mutant before it reports, and say so in the dispatch.
- **Replacing a brief's one-fact clone with a real fixture cost a round**: a real fixture co-varies its facts. Keep both.
- **Read the clock** (`date -u`) at every ledger entry; entries written from memory ran ahead of it.
- **Check a citation by content** before ruling on it, a reviewer's included.
- **The terminal driver** (Forge) may refuse to open a session with `posix_spawnp failed`: find the install that is running (`ps`) and restore the exec bit on THAT install's `node-pty` spawn helper. `docs/forge-cli-testing.md` has the recipe.

## 7. After the tasks

The plan's section of that name binds; this is its outline.

1. The whole-branch review, its fix wave and its re-review.
2. The gates at the head, both scripts, the walls machine-tagged.
3. The deploy to the private test Worker by `worker/RUNBOOK.md` §26, recorded there as a new record. The version it replaces, `eba04eca-d30c-427c-9651-ede6693e3f00`, is the rollback target.
4. The verification, line by line. A line that cannot be read is reported as not read.
5. On a pass only: the cycle's close, `CLAUDE.md`'s two lines, the board, and the wrap's handover, whose contents the plan lists. The owner asked that the work reach main as a clean branch and reach the production hosts in a separate session: the wrap's handover lays that choice out for his ruling there, under the two-repository rule (`CLAUDE.md`, "The two repositories"), and nothing in this session acts on it.

If the verification fails and the fault cannot be fixed inside the run's rules, stop there and report: what was read, what failed, what was tried. Do not write the wrap's documents over a failed verification.

## 8. What the owner will want at the end

A short account: what was built, the review's outcome, the gates, what the private Worker read line by line, and where the wrap's handover is. Then, under "Rulings I made", every ruling in the ledger with its cost. His own reading of the private Worker, in a browser and in the terminal, comes after that, and the wrap is his to open.
