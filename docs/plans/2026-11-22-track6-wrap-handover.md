# Handover — the wrap of Track 6: joint verification, then main and the public release (2026-11-22)

**What this is.** The entry document for the session that closes Track 6 (the browser client's play feedback, issues #75, #76 and #77): it takes three lab branches to the public repository and the three live tiers. It is written for a session that has read nothing else. It is a pointer: the closes and the runbooks it names are the authorities, and where this page and one of them disagree, the authority wins and the disagreement is a finding to report.

**The owner's direction (2026-11-22, paraphrased; his words are in the run's ledger):** after plan 2b's whole-branch review, its fixes, the deploy to the private test Worker and this handover, a FINAL session verifies the private Worker together with him; only once BOTH readings are done does that session package the branch for the public repository and make the public release.

## 0. First, and before anything else: the joint verification

Nothing in §2 to §6 starts until both of these are done and both pass.

1. **The owner's reading** of the private test Worker, `https://yokohama-test.moore.nyc`, in a browser and in the terminal (§1 says what is on it and what to look at).
2. **The session's own re-reading** of the same host: `worker/RUNBOOK.md` §26's record of 2026-11-22 for plan 2b lists what was read and how; re-read the same lines at the version that record names. If the host has been redeployed since, read at the version it carries and say so.

**What the plan 2b session read there, and what it could not** (the cycle's close §4 has the table; `worker/RUNBOOK.md` §26 the record): over WARP, health, identity, the page's headers and the release; Chinatown's two chooser replies equal to the committed fixtures; the Port's chooser by lines on a two-player table through to the take in the log; the Laboratory's fold on a table that holds both copies of a title, its two rows naming their slots; one press's round trip (about 0.1 s). NOT read: how a browser that holds plan 2a's release takes the new one (the front page should take it with no click; a game tab should show the update line, whose click reloads) — the browser that session could drive held nothing for the host, so this line is the owner's; and nothing about how anything LOOKS. The live replay was not run.

If either reading finds a fault: fix it on the lab branch through a reviewed fix (a task-sized fix has its review; the suites it touches green), redeploy to the private Worker (`npm run deploy:private`), and read again. The wrap does not start over a fault.

## 1. Where things stand

| | |
|---|---|
| `web-evaluator-feedback` | cycle 1: the claim on the card and the flag tally (issues #75, #76). Head `5abce3e3`, 34 commits on master. Closed on the lab: `docs/plans/2026-11-20-web-claims-and-flags-close.md`. |
| `web-choosers-goods` | cycle 2, plan 2a: the Church's and Chinatown's choosers (issue #77). Head `f46e8f76`, 51 commits on cycle 1's head; reviewed at `04742e87`. |
| `web-choosers-cards` | cycle 2, plan 2b: the Port's chooser, the Laboratory's fold by title, the slot named on a twin technology's rows, the terminal's flag counts (issue #77; the flag counts finish #76's design). Cut from `f46e8f76`. Reviewed code head `5ecad6f0`; the gates were read at `4946f827`; every commit after that moves documents only, and the branch's head is the commit that carries this page (`git log -1 web-choosers-cards`). **It contains the other two**: master → cycle 1 → plan 2a → plan 2b is one line of history. |
| master | `e600f5f4` on origin and on the lab, `0.7.3`, untouched since 2026-11-10. |
| the private test Worker | version `5204700b-1758-44d1-a207-66199ae6da40`, the release `yokohama-release-cfb3f42dde69314f`, built from `4946f827`: all of Track 6 (cycle 1, plan 2a, plan 2b). Behind Access. Rollback target: `eba04eca-d30c-427c-9651-ede6693e3f00` (plan 2a's head). |
| production | `0.7.3` on all three tiers (the NAS, the public Worker, the CLI terminal), untouched. |

All three branches are on the lab only. Nothing of Track 6 is on origin: no branch, no pull request, no tag.

**The cycle's close is the record of record:** `docs/plans/2026-11-22-web-area-action-choosers-close.md` — what the two plans built, both whole-branch reviews, the gates at plan 2b's head, every ruling of both ledgers, what was deferred, what the cycle does not claim, the lessons to weigh for `CLAUDE.md`, and what the wrap owes (its last section; this page's §2 to §6 follow it). The gates at the head are also on the board (`docs/handover.md`, "Gates at HEAD").

**What the owner will want to look at on the private Worker** (each was a call made for him during the run; the close's register has them with their costs):

- **The Port's chooser in a browser**: the card faces laid out flat (the art a small thumbnail beside the chips); on a phone every card in reach is whole; on a desktop window (1440 × 900) the cards scroll inside the answer bar; on a short landscape window the art shrinks. No agent looked at how it looks.
- **The slot marker**: when both copies of a technology are on the same Laboratory board, each of its rows reads `Laboratory: acquire Ball (slot 2)`. Its form (` (slot N)` after the name, only in that case) and one known gap: the browser's Suggest and Why panels word a suggested line's LATER steps at the position the line starts from, so such a step can read without its slot or, in one rare line shape, with a stale one. The row a player presses is always right. Recorded, not changed.
- **The Laboratory's entry and list over a twin**: the entry says "2 ways to pay" and the list is titled "choose the payment" although the two rows are two cards.
- **The terminal** (`cargo run -p yokohama-cli`): the `players` view's flags lines, and the counts on each seat's Mercenaries row in the dashboard; at four players the `used` count yields, and at extreme counts the flags yield too (never clipped).
- **The Port panel's empty state** is silent (no hint line), as Chinatown's is.
- The fourth help sentence (the slot marker), in the help and in `docs/web-guide.md`.

## 2. How the branches reach origin — the owner's ruling, asked for here

The rule (`CLAUDE.md`, "The two repositories"): a branch reaches origin ONCE, when its review is clean, as the pull request that merges it; master moves on origin only by that merge, a merge commit; a release tag sits on master's merge commit; master is then pushed to the lab, and the merged branch leaves origin once the lab holds it. All three branches' whole-branch reviews are clean.

The owner asked that the work reach main as a clean branch. Three ways to do that under the rule, with what each costs:

- **A. One pull request: `web-choosers-cards` against master (recommended).** The branch contains all three reviewed units, so one push, one pull request, one run of origin's Actions, one merge commit, and the release tag on it. Its description closes #75, #76 and #77 and links the two closes. It matches how the work was evaluated (the private Worker carried each head in turn and now carries the whole) and how it will be released (once). Cost: a pull request of about 130 commits whose reader needs the closes to find their way; the three reviews are on the lab, so the description must say where their records are.
- **B. One pull request per cycle, in order**: `web-evaluator-feedback` (closing #75, #76), then `web-choosers-cards` (closing #77; it contains plan 2a), or three with plan 2a's branch between. This is what the board planned before the owner's word of 2026-11-22. Cost: two or three pushes, Actions runs and merges for one release; master passes through heads that only the private Worker ever ran. Benefit: each pull request is one issue's story.
- **C. A re-packaged branch**: the three units rebuilt as a few commits on a new branch cut from master. Not recommended: it gives origin hashes the tree's own records do not know (the closes, the plans and the runbooks cite the lab's commits by hash throughout), it discards the test-first commit history the repository keeps on purpose, and nothing reviewed would be what is merged.

Whichever is ruled: push only when the joint verification has passed; dispatch `web.yml` on the pull request and read it; merge with a merge commit; push master to the lab; delete the merged branch on origin once the lab holds it.

## 3. The release

By `docs/web-lane.md` §7 (read it whole; its numbered steps are the recipe and its traps are real):

- **The number is the owner's.** The workspace is at `0.7.3`. §7 asks for a bump at every release that can change what a save means or which bot plays; this release changes neither (a chooser commits the same move its old row did; no bot behaviour moved; `SEMANTICS_REV` is 7). It does change what the terminal and the browser ask at four areas, and the CLI's release archive carries that. The session lays out `0.7.4` against `0.8.0` for his ruling; the recommendation of the plan 2b session is `0.8.0`, since the CLI's behaviour at its prompts changed on purpose.
- **Before the bump, the dependencies' batch** (§7, `CONTRIBUTING.md`'s Dependencies): the one-week floor, lockfiles only, targeted `cargo update --precise`.
- **The bump** by §7's steps 1 to 8, each diff read: the version and nothing else.
- **The tag** `vX.Y.Z` on master's merge commit, annotated; it runs `release.yml`.
- **The decision-log rows owed** (`guidingpath.md`, append-only), written at the wrap: the three-cycle direction of 2026-11-21 (Track 6 run out of the lab, evaluated on the private Worker, released once); this cycle's deliberate change to the REPL's behaviour (the exception recorded in `docs/web-lane.md` §1); the release itself. `CLAUDE.md`'s lessons from both closes are weighed there too (each close has a "Lessons" section).

## 4. The production deploy, in the owner's window

The two runbooks are the recipe; the last release's records are the model: `deploy/RUNBOOK.md` §3.24 (the NAS) and `worker/RUNBOOK.md` §28 (the public Worker, the thirty-ninth deployment). Order: **the NAS first, as the canary**, then the public Worker, then the CLI terminal's host. Each tier has its own client build (`docs/web-lane.md` §14's table gives each host's build values; production's Worker build is KEYED, the private one's is not). Things this track adds to what the window must know:

- **A deploy of this change drops a picker left open on a Worker table** (the picker's code digest moves with any CLI source): a player reopens it. Say so in the window's note.
- **A tab opened before the deploy at a shared table** meets a prompt kind it cannot draw (`church`, `chinatown`, `port`) until it takes the new release; the owner accepted that window on 2026-11-22 (`docs/web-lane.md` §7's paragraph on it; the plan's "How a player gets this release"). The mechanic that would make open tabs refresh is issue #78, left open.
- **The live replay** (`worker/scripts/live-replay.mjs`, `worker/RUNBOOK.md` §7) is run from the RELEASE's tree: its kept list (`worker/scripts/replay/fixtures/production-door-kept-rows.txt`, 32 kept rows) and the walk rows the runbook cites by number moved in this cycle (the walk gained 20 rows), and from plan 2a on a replay holds only against a Worker built from the same tree or a later one. After the bump, the replay's reset recipe first (§7, "The replay's reset").

## 5. The pictures

- **The help's pictures are re-cut ONCE, on the final tree**, after the merge and before the release build: every one, with the claim-band picture decided there (none of cycle 1's five shows a band). The Port's panel and the Laboratory's list are new surfaces a picture may want. No agent opens an image: the generator's `.aria.txt` snapshots are what an agent reads, and the look is the owner's reading.
- **The Worker README's ten pictures** (cycle 1's Task 9): where they are cut is the owner's ruling. Cycle 1's whole-branch review recommends the private test Worker with an empty lobby (on the public Worker strangers' tables would appear as masked boxes in committed pictures). Two rules ride that task's brief: the configured reporter only, never `--reporter html`, `json` or `blob`; and a failed run's `test-results/` is not read by an agent.

## 6. The closes, the issues and what else is owed

- **Issues**: #75, #76 and #77 closed by the pull request(s); #78 (the forced refresh) left open. A new issue on origin for the browser advisor's wording of later steps at the root position (the close's "does not claim" section has its reach), if the owner wants it tracked.
- **`docs/web-lane.md` §2's counts** re-read at the final tree (the last task of plan 2b re-counted them at its head; the bump and the merge move the tree again).
- **Cycle 1's deferred list** (`docs/plans/2026-11-20-web-claims-and-flags-close.md` §5) and **cycle 2's** (the close's "Deferred" section): each taken up or dropped, by name.
- **Left as they are, and to be said in the wrap's close**: the trailers of the lab commits before plan 2a's fix wave name the orchestrator's model whichever model wrote them; a handful of commit messages overstate or cite the wrong issue (the cycle's close lists them). A pushed commit is not amended.
- **The board** (`docs/handover.md`) rewritten for "Track 6 released", and Track 6's entry artifacts.
- **The sidecar backup**: offered at the session's close if `/Volumes/yokohama` is mounted (the owner's standing habit).

## 7. Operating notes for the session

- **Reading the private Worker needs the workstation's Access identity over WARP.** If every request answers 302 to Access's login, the WARP session has lapsed: the owner re-authenticates (`warp-cli debug access-reauth`, or the WARP menu). An agent does not sign in for him.
- The private Worker's recipe and its records: `worker/RUNBOOK.md` §26. Its deploy is `npm run deploy:private`, both guards; it is the orchestrator's with the owner's word, never an implementer's.
- The run's ledgers (gitignored, on the workstation that ran them): `.superpowers/sdd/2026-11-21-web-area-action-choosers-2a-plan/progress.md` and `.superpowers/sdd/2026-11-22-web-area-action-choosers-2b-plan/progress.md`, with every implementer's and reviewer's report beside them. The cycle's close carries their registers; the ledgers are the detail.
- The gate scripts used at plan 2b's head are in that second folder's `tools/` (`per-edit.sh`, `roster.sh`): copy them to the session's scratchpad, set the fence's base, run the per-edit one first and the roster after it, never beside it.

## 8. What the owner decides in that session

1. Whether his own reading of the private Worker passes (§0), and anything he wants changed before release (§1's list).
2. One pull request or one per cycle (§2).
3. The release number (§3).
4. The window for the three tiers (§4).
5. Where the README's pictures are cut, and whether the help carries a claim-band picture (§5).
6. Whether the browser advisor's wording gets an issue (§6).
