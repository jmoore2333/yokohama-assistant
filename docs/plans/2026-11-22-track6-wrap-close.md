# The play feedback released — the wrap's close (2026-11-22)

**The record of record for cycle 3 of Track 6, the wrap.** It took the browser client's play feedback (issues #75, #76 and #77) from three lab branches to the public repository, the release `v0.7.4` and the hosts. Its entry document was `docs/plans/2026-11-22-track6-wrap-handover.md`; the two cycles it carried are closed in `docs/plans/2026-11-20-web-claims-and-flags-close.md` (the claim on the card and the flag tally) and `docs/plans/2026-11-22-web-area-action-choosers-close.md` (the area-action choosers). Those two are frozen; what they left for the wrap is answered here.

**Headline.** One pull request, #79, merged to master as the merge commit `ba4d5f4c` at 21:04:46Z; `v0.7.4` tagged on it and published by `release.yml` at 21:09:33Z, before any production host was touched; all three tiers run `0.7.4`. No fenced crate moved and `SEMANTICS_REV` is 7. Issues #75, #76 and #77 are closed, each with an account of what shipped; #78 stays open.

## 1. What the owner ruled in the session

Paraphrased; each was his own word in the session.

- **The joint verification passed.** His reading of the private test Worker, in a browser and in the terminal, beside the session's re-reading of the lines `worker/RUNBOOK.md` §26 records (health, identity, the page's headers and the release; Chinatown's two chooser replies against the committed fixtures; the Port's chooser to the take in the log; the Laboratory's fold with its slots).
- **One pull request** from `web-choosers-cards`, which holds all three reviewed units; **the number `0.7.4`**; **the window's order**, the NAS first as the canary; he did not need to be present for it.
- **A stop before origin for the pictures.** No agent opens a picture that holds the game's art, so before each picture reached the public repository it was put before him, the help's on the private test Worker, with what only an eye can settle listed per picture: the five whole screens before the pull request opened, the two chooser crops and the terminal animation (a file on the workstation) while it was open and before its second push.
- **The choosers are to be seen where they are described.** At that stop he asked for a picture of the Church's chooser and of the Port's in the help, and chose to hold the release for them over a follow-up.
- **The terminal's animation is to show the terminal as it now is**, and its recipe is to be a tool in the tree.
- **A private host's detail is the lab's.** The public record carries one line for each private host and the public Worker's record in full; the rest is in the private tracker.
- Not ruled, and left open in §7: whether the help carries a picture of a claim; where the Worker README's ten pictures are cut; whether the two candidate issues are filed.

## 2. What the wrap added to the branch

Eleven commits on top of the lab head `44adeee6`, 143 on master in all.

| Commit | What | Written by |
|---|---|---|
| `8638011e` | The version `0.7.4` by `docs/web-lane.md` §7's steps: the workspace and its nine lock lines, the two npm packages, the re-stamped fixtures and backup envelope, the README's archive name, the Dockerfile's example tag. Each regenerated file equals its old self with the version replaced. | the orchestrator |
| `1b77f7a2` | The help's five whole-screen pictures re-cut once from the final tree. No alt or caption needed a change. | the orchestrator; the alts checked by an Opus agent against each picture's accessibility snapshot |
| `754cf353`, `c9c13cba`, `df4f62c7` | The lane document: the layer sentence made exact (the choosers' models are `chooser`'s), two counts the fix wave had moved, and the review's corrections. | the orchestrator |
| `bf396e96`, `1298b383`, `2a1b8985` | Two new help pictures, the Church's chooser and the Port's, cut by the generator as crops of the answer bar and placed under their paragraphs in the help and in `docs/web-guide.md`. The route to a chooser moved from `the_table.spec.ts` to `harness.ts`, byte for byte. After the review, the generator holds each frame to the words beside it, or writes nothing. | an Opus implementer |
| `26c5ce29` | `web.yml`'s whole-game step asserts the gate's four tests (§4). | the orchestrator |
| `9c4ce276` | A stale precache figure out of the Worker's README. | the orchestrator |
| `b7651dae` | The README's terminal animation regenerated at `0.7.4` by a tool now in the tree, `docs/tools/mkcligif.py`, with a text sidecar and a `--check` that reads drift without opening a picture (issue #69's first half). | an Opus implementer |

**The dependencies' batch** ahead of the bump moved nothing and left no commit: no open alert, `npm audit` clean in both packages, and the one-week-floor dry run changed neither lockfile.

**The pictures, as the owner read them.** Seven help pictures: five whole screens at 1710 x 869, and the two chooser crops at twice the pixel density (958 x 650 and 958 x 768). No goal is claimed in any frame; a seat's flag count is non-zero in every whole screen. The terminal animation keeps its shape (1618 x 1108, eleven frames, the same timings; 678,573 B): every later-turn frame carries non-zero flag counts, and turn 9's frame moved to that turn's area action so that the Church's chooser entry is in frame. The README's words for it needed no change.

## 3. The reviews

- **The wrap's first four commits**, one Opus reviewer, read-only: no Critical, two Important, five Minor. The Important ones: the lane document said `cargo doc -p yokohama-worker --no-deps` reads 0 warnings where it reads 2, and `c9c13cba`'s message had just said every other count read as written; and nothing showed the dependencies' batch had run. Both answered in `df4f62c7`.
- **The chooser pictures**, an Opus task reviewer on an export: no Critical, one Important (the Worker README's figure, fixed in `9c4ce276`), eight Minor. The help's cells caught all seven of its mutants, the helper's move was byte-exact, and every clause of both alts matched its snapshot. Its weightiest Minor: the generator's two fallback arms could write, at a later re-cut, a frame the committed words contradict. One fix round (`2a1b8985`) made the fallbacks misses and asserts what the words claim; the scoped re-review approved it.
- **Recorded, not fixed:** the Church check counts one unavailable kind without checking it is the kind held at zero; the crop's overlay check tests five points and cannot see an overlay that ignores the pointer, so `2a1b8985`'s message overstates it; `1298b383`'s "each read red" is false of one cell, an absence check that could not read red; `1b77f7a2`'s "the help still carries no picture of a claim" reads as settled where it is the owner's to rule.
- **The terminal animation** had no separate reviewer. The orchestrator read the commit, opened two of its frames (they are text renders and hold no art), ran the document cells and checked that no private path is tracked.

## 4. The gates, and what origin read

On one workstation (a MacBook Air, Mac17,3, Apple M5), 2026-11-22, the BGA corpus present.

- **Per-edit at the bumped tree `754cf353`** (17:18–17:38Z), every command green, the counts those of plan 2b's head. **The roster there: 449 s, 15 of 15**, one uninterrupted sequence with the release binaries pre-built, 17:39:17–17:46:47Z: fuzz 44, census 1, observation 3, action_equiv 25, action_worst 1, arena 20, policy_div 6, evaluator 249, architecture 46, recovery_lib 9, piped_dogfood 13, differential 25, routes3_diff 2, scoring_diff 2, routes3_worst 3. No crate moved after it, so it was not run again.
- **Per-edit again at `9c4ce276`** (19:12–19:35Z), every command green: core 1,587 / 29 ignored; cli 955 + 2 ignored; oracle 191; difftest 52 / 5; eval_probe 214; solver 132; bridge 82 and its four node suites; web 627; wasm 10; worker 92; the identity slice 9; clippy clean; `gen-data` byte-identical; vitest 1,717 in 77 (three new help cells); `test:wasm` 430 in 20; e2e 103 passed in 13.0 min; worker `test:scripts` 801 in 33 and the pool 302 in 14; the fences empty against master. After `b7651dae`, which moves documents and a tool, `test:wasm` and `lane_document` were read again, green.
- **Origin.** The pull request's `test` check was green on both pushes. The dispatched `web.yml` run on the first push read the `web-server` job RED with every test green: its last step asserted the whole-game binary printed "2 passed", and plan 2b had added two tests to that binary. The fix is `26c5ce29`, and the run on the final head read all three jobs green, its `web-server` job at 21:08:25Z. **That was after the merge (21:04:46Z) and the tag:** the merge followed the pull request's green `test` check and did not wait on the dispatched workflow, on the owner's word that it need not be run again; the orchestrator dispatched it all the same, since its last run had been red and the fix had never run on origin. Master's `test` run and the tag's `release.yml` (five jobs) were green.
- **What reached origin after the pull request opened.** The pull request opened at `df4f62c7`, when the first review was answered, and was pushed once more at `b7651dae` with six commits written while it was open: the chooser pictures (task-reviewed, a fix round, re-reviewed), and three read by the orchestrator alone — the `web.yml` fix, the Worker README's sentence and the terminal animation. No whole-branch review covered the wrap's eleven commits. The two-repositories rule's "once, when its review is clean" held for the three units and for the first push; the second push is a departure from it, taken to hold the release for the pictures the owner asked for.

## 5. The window

- **Before any host:** the release published by `release.yml` from the tagged merge commit.
- **The NAS, the canary** (`deploy/RUNBOOK.md` §3.25, a pointer): its own client build, the image's smoke passed before the swap, and after it the health document read `0.7.4` through the tunnel, the Host check's two lines read as the runbook expects, the terminal container still could not reach the port, and the Port's, the Laboratory's and Chinatown's readings were played there.
- **The public Worker** (`worker/RUNBOOK.md` §29, the fortieth deployment): the keyed build `yokohama-release-a277dfd848fbdea9`, deployed 21:13:05–21:13:23Z as version `3a90bd77-74b9-4db6-a281-cb10fd6a008f`. Read without a cookie at 21:13:32Z: the page with its enforced header set, `sw.js` naming the release, the stamp with all seven values, the seven help pictures, and the edge's challenge on `/api/health`, `/api/me` and `/enter`.
- **The private test Worker** was the owner's review host: it carried the bumped tree before the pull request opened, and the release's tree, with the chooser pictures, before the pull request's second push. It reads `0.7.4`.
- **The terminal host.** The handover named a browser terminal as a third production tier at `0.7.3`. That was wrong: the three tiers the records mean are the NAS, the public Worker and the private test Worker, and the terminal host is a separate deployment whose record is kept outside the repository. It was updated to `0.7.4` on the owner's word in the session, and its record is the private tracker's.

**Not read in the window:** the public Worker's `/api/health` through a browser (the edge's verification page is not an agent's to pass, so `server_version` there is the owner's look); the live replay, which needs a credential the workstation does not hold; how a browser holding `0.7.3` takes the release on a production host; and the lane document's other live steps for a release that changes the client (`docs/web-lane.md` §3: the live Playwright tier, the headed game, the phones), none of which was run.

## 6. What the two closes left for the wrap, answered

- **Taken up:** the pull request, the number, the release and the window; the help's pictures re-cut once on the final tree; the lane document's §2 counts and its layer sentence; the decision-log rows (guidingpath's three 2026-11-22 rows); the issues; the lessons weighed for `CLAUDE.md`; the board.
- **Stand as the closes record them, for a later web cycle to take up or drop:** cycle 1's deferred list (its close §5) and cycle 2's (its close §6 and §7), every item. None was built on.
- **Left as pushed:** the trailers that name the orchestrator's model for an implementer's work, and the commit messages the closes list as overstating or citing the wrong issue. A pushed commit is not amended.

## 7. Open after the wrap

- **For the owner's ruling:** whether the help carries a picture of a claim (it needs a generator that plays deeper into a game than the present 9 to 14 moves); where the Worker README's ten pictures are cut (§8's second lesson has what was found); whether to file the two candidate issues — the browser advisor wording a suggested line's later steps at the position the line starts from, and the older answer-bar arms dropping keyboard focus when pressed.
- **For the owner's eye:** `server_version` on the public Worker; the update path on a production host; the live replay, from its reset recipe, with the session secret (`worker/RUNBOOK.md` §7, §29); the live steps §5 lists as not run.
- **Issue #69's second half:** the seven older terminal pictures no page shows, and pictures for the CLI guide.
- **`docs/web-lane.md` §3's row for the README pictures** still gives the live recipe as if it could run today.

## 8. Lessons

- **A count asserted in a workflow that runs on dispatch alone is a cell no gate runs.** Plan 2b added two tests to the whole-game binary; three reviews and two full gate runs later, the first dispatch of `web.yml` at the release's pull request read the job red. A test added to that binary moves the workflow's count in the same commit, and the workflow is dispatched at each cycle's close.
- **A generator that signs in several people cannot be pointed at a host with one identity.** The README's picture generator plays three people; the private test Worker has one identity and no dev door, and the generator's live mode is production's and needs its session secret. "Cut them on the private Worker" was a recommendation no one had tried.
- **A generator's fallback arm writes a picture the committed words deny.** The words beside a generated picture are claims; the generator asserts each of them before it writes, and anything else is a miss.
- **An inventory in a handover is checked against the hosts before a window is agreed.** The handover listed a host as a tier at the released version; it was neither.
- **"Lost" is a claim to check by content, not by name.** The animation's recipe was searched for by the picture's file name and reported gone; the scripts were in the tree's gitignored notes under names that do not mention it.
- **A picture stop belongs in the plan.** Pictures no agent may open need the owner's eye before they reach the public repository, and a new surface a player sees wants its picture where the words describe it: both were found at the wrap and held the release, where a plan could have carried them.
