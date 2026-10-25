# Security S4 — the public release: design

**Status: DRAFT, uncommitted.** Brainstormed with the owner on 2026-10-22 at HEAD `56460688` on `security-s3`
(PR #56 open against `security-auth`; master at `73f9ba8e`). It stays out of the tree until the owner has read
it. A fresh agent read it as the planner before the owner did; its questions and their answers are in §11.

**What this is.** The fourth and last cycle of the security project: the repository becomes public and the
Worker's door opens to everyone, in one window. The ledger (`docs/security-ledger.md`) names S4 "the public set,
outside this specification" and holds three items for it (C4, D17, D20); two older rows (C1, D3) and the lane
document's §8 hand it three more sentences, settled in §11. The board's Track 2 asked for this
brainstorm in these words: "the public split's shape and timing; the root layout (`analysis-1/`, `tts-assets/`,
`bridge/`, `deploy/`, `docs/temp/`); whether the two rulebook PDFs stay (the owner's edition is
`tts-assets/pdf/…`; `analysis-1/`'s is the outlier); what a public README claims." This document answers those
and sets the shape: **a precursor wave and three cycles**, each with its own plan written against the merged tree,
each landing by the project's rule of branch → PR → whole-branch review → fix → scoped re-review → merge. The wave
(§5a) clears the web lane's leftovers first; the third cycle ends with the switch.

The register of everything this cycle writes is professional: the documents describe the project and its
method; none of them says why the owner wants it seen.

---

## 1. Intent

A stranger who arrives from the owner's site should be able to tell from the first screen what the project is
and that it is real, tested and alive; be playing the game in a browser within a minute; and, wanting the code,
build it from a fresh clone with the quick start alone. A contributor should find the workflow, the suites and
the review discipline stated once. The record — the specs, the plans, the closes, the decision log, the commit
history with its trailers — stays whole, because it is the account of how every decision was made and it is
cited by commit SHA throughout.

The owner, on what a reader should notice (2026-10-22): "the end result, and the iterative improvements and
demonstrated gains. Ultimately, the engine works, is tested, we are closing in on our goal of a computer player that
can competitively play with good human players, playing to win — and eventually we will improve that pipeline
beyond what we have now through a variety of ways to make the best yokohama computer player we choose to."

The honesty fence that governs every sentence about strength is the map's: `docs/plans/2026-07-25-phase3-map.md`
§5 (line 168) states the milestone bar, "§1.9 — all four ingredients: scenario suite (grown through 3.3–3.5)
green · statistical superiority over **all** preserved rungs with CIs · decision explanations for every move ·
**owner acceptance play** through the CLI", and `CLAUDE.md` records that "**The bar (map §1.9) is NOT claimed**: a
shipped milestone is a narrower claim than the bar." The README and the method page say exactly that.

## 2. Settled inputs (the owner's direction of 2026-10-22; not up for re-litigation)

1. **History is kept whole; no rewrite.** The evidence (Appendix A): gitleaks over every commit found 21 hits,
   all fixtures or false positives; no credential file was ever committed; 178 Markdown files cite 1,460 distinct
   commit SHAs that resolve in this repository. A rewrite would orphan the record and would scrub nothing.

3. **`tts-assets/` leaves the public tip.**  The derived crops (`crates/yokohama-web/assets/art/`) and the help
   screenshots (`web/public/help/`) stay, because the application draws them. The English rulebook PDF under
   `analysis-1/` leaves with the sheets (a call, §10). 
4. **`docs/temp/` stays private.** S4c carries a triage table; the default for every item is private.
6. **CONTRIBUTING's ask.** The owner: "the contributing.md file should welcome AI assisted workflows, but the user
   making the submission should be comfortable with the request or contribution and be comfortable discussing it
   (even if they are being coached in that discussion by a model on the side)."
7. `ASSETS.md` becomes a notice of facts; the sweep in S4a reads every tracked line that
   mentions the subject (Appendix A lists them: outside the licence texts there are nine lines in `ASSETS.md`,
   three in `NOTICE`, and one or two lines in each of seven dated documents).
8. **Branches.** The fifty branches already merged into master leave origin after a backup; The three unmerged stay.
9. **Merge order.** PR #56 merges now; `security-auth` merges to master before S4a starts (every commit on it has
   had its review and is the code production runs at 0.7.2); each S4 cycle branches from master. The owner:
   "Agreed on the merge for 56."
10. **The lab.** A second, private GitHub repository is the default push target for work in progress; the public
    repository receives a branch when its plan exists, and "when we bring in the branch to the public repo the
    commit history rides along so the history keeps building." One checkout, two remotes: the memory directory,
    the worktrees and `CLAUDE.md` stay where they are.
11. **GitHub's own flows.** Issues on; `SECURITY.md` points at GitHub's private vulnerability reporting; fork
    pull-request workflows require approval; master is protected once the repository is public.
12. **The yokohama-web documentation is written in S4a, for both readers.** The owner: "I assume it's both — the
    person hosting it will eventually play it, and there are also people that may want to just read what it's
    capable of and how it works, and just use our free hosted version at yokohama.moore.nyc to see how it works
    as we go."
13. **Three cycles.** S4a the tree, S4b the
    front door, S4c the flip (§6–§8).
14. **The window.** The owner: "Cloudflare access will be removed at the same time the repo is changed to public
    in Github, or just before hand". Stage 2 of the auth cycle is the recipe already written in
    `worker/RUNBOOK.md` §23 ("Stage 2 — the keyed build, the rule, then the hostname off the Access application (a
    later session, with the owner)"); S4c runs it verbatim with the owner present and does not redesign it.
15. **The clean-eyes walk.** The owner: "absolutely have a clean eyes haiku agent follow those instructions
    locally and get it up and running in docker the cli and web version just to test it out, scripts we may need
    to help out we can add."
16. **Release notes and PR bodies are swept** with the same patterns as the tree; they become public with it.
17. **The repository's description is rewritten.** The AR origin stays in the dated documents as the trail; the
    owner: "if someone wanted to enough they could track the progression of the old docs to see how this
    evolved."
18. **The art lane's branch goes to the lab** (ruled 2026-10-22): the lab is a private backup, not the push the
    2026-10-05 rule forbids;
19. **`test.yml` runs again on origin from the flip**, with a `push` trigger on master under its path filters and
    its comment corrected forward; the lab runs no Actions at all. The owner: "once we're public we can let those
    run,".
20. **The test identity and the author address.** The stranger's sign-in at the window uses a test address of the
    owner's that forwards to the owner's inbox (named in the session's record, not in this document). From S4a
    on, commits are authored under the GitHub no-reply address on both workstations and the lab, with GitHub's
    private-e-mail settings on. 
21. **S4 is not code-focused** (the owner, 2026-10-22: "I agree S4 should not be code focused"); the web lane's
    open items are either tracked as GitHub issues and deliberately delayed, or cleared in a precursor wave with
    its own PR before S4a — ruled the same day, item 22.
22. **The web lane's leftovers: the middle course** (ruled 2026-10-22, after the write-up
    `docs/temp/2026-10-22-web-lane-leftovers-issues.md`, gitignored). Its Issues 1 to 4 — the seat-aware admission
    on both hosts, the marker head's wording, the NAS's "a keyed build is the open door's" sentences, the secrets
    guard's door check — are built as a precursor wave, `security-s4-pre`, its own branch and PR off master before
    S4a; Issues 5 to 7 are posted as issues at the wave's close, "leave us with something on the board". The four
    defaults stand: Issue 1 keeps the hard total of 64 and adds a watchers' figure of 32, so a seated caller is
    refused only by the total and its own 8 and four seats × 8 + 32 cannot exceed 64; Issue 2's wording is the
    owner's; the "record and leave" boxes stay recorded; the wave DEPLOYS WITH STAGE 2.
23. **One release, both hosts.** The owner: "we should update the version number all together anyway so we don't
    end up with drift." Stage 2's `0.7.3` carries the wave and the Turnstile commit, and the NAS deploys at `0.7.3`
    too; §8.3's "the NAS left at 0.7.2" is withdrawn.
24. **The issue style is the contributor style.** The owner: "the issue list for issues needs to be evaluated for style after we create the contributing.md file" The posted issues keep the write-up's headings —
    Context, What happens today, The change, Where, Acceptance, Size, Record to update when it lands — and S4a's
    issue templates are written to the same layout, so the tracker's first entries model the form.
25. **The NAS sidecar sync is read in S4c's pre-flight.** The write-up found that the board's item 8 is the 3.5c
    amendment's D10, a standing sync of the gitignored material to the NAS, and that the record never says whether it
    runs. That material has no other copy — the two BGA captures behind the fourteen corpus pins, the six owner-game
    saves, `docs/temp/`, the memory directory — and the lab remote carries none of it. The owner: "Confirm NAS sync parity in the preflight for the public flip." S4c's pre-flight reads it; the board's item 8 closes with the
    reading.

## 3. Goals, non-goals, anti-goals

**Goals.**
- G1 — The first screen tells a stranger what this is, that it works, and where to click: the hosted site, the
  quick start, the docs.
- G2 — Fresh-clone truth: every command the README gives runs on a clean machine, proven by a reader who knows
  nothing else (§7.7).
- G3 — The record stays whole and every citation in it still resolves.
- G4 — The Worker's door is open to everyone, by the auth cycle's stage 2, in the same window as the switch.
- G5 — The licence position is explicit and true: Apache-2.0 on the code, the game's components named as the
  rights holder's, the dependencies audited.
- G6 — The agent-facing documents (`CLAUDE.md`, `guidingpath.md`, the board) are re-pointed at the AI lane, so
  the session after S4c opens on V4's vetting.

**Non-goals.** No history rewrite. No art replacement (the lane continues on its local branch under its own
rule). No engine, bot or `SEMANTICS_REV` work. No new web features in S4a–S4c; the web lane's leftovers are the
precursor wave's (§5a). No wiki (a wiki is a separate, unreviewed
repository, and anyone can edit a public one). No per-file SPDX headers (§10). No repository rename.

**Anti-goals** (a failure even if everything else works): a quick start that fails on a clean machine; any
sentence that claims the bar or overstates V4; raw work in progress
reaching origin; a citation that no longer resolves; a real person's handle or any secret value in a fixture
(D20); the repository reading as self-promotion.

## 4. Constraints and the standing rules that bind every cycle

- **The project's conventions** hold unchanged: TDD; the per-edit suites and the lint gate after every task;
  the web lane's suites whenever a change reaches what the layer compiles (`docs/web-lane.md` §2); dated
  artifacts frozen, corrections recorded forward; the decision log append-only, the archives append-closed;
  `CLAUDE.md` standing rules only; stage with explicit paths.
- **Every cycle lands by the owner's branch rule** (2026-10-01): branch → PR → Fable whole-branch review → fix
  commit → scoped re-review → merge. 
- **The standing dispatch rules** apply to every agent: the full report to a transcript file with the path and
  three lines in the reply; no `rm -rf`, `rm -r` or bulk `rm -f` in any dispatched agent ("a single file may be
  deleted only by its explicit path"); no waits on events, no Monitor; no peer messaging; implementers Sonnet
  or Opus by the brief's judgment load, reviewers Opus, the whole-branch review Fable. The clean-eyes walker is
  Haiku by the owner's direction: a reader that weak is the point.
- **The deletions are the orchestrator's or the owner's.** `git rm -r tts-assets`, the branch deletions on
  origin and the removal of the stray untracked trees are run by the orchestrator in its own shell, never
  dispatched.
- **Live documents are re-pointed; dated documents are not edited** for a path that no longer exists. A frozen
  plan that cites `tts-assets/` is history. The two exceptions are named and recorded forward: D20's redaction
  of a third party's handle, and any dated line the copyright sweep finds that argues a legal position rather
  than stating a fact (none is expected; Appendix A).
- **The archives are verbatim.** `guidingpath-archive.md`, `-phase3.md` and `-web.md` are not touched; the new
  archive S4b creates is written the same way.
- **The lane document's cells.** `crates/yokohama-web/tests/lane_document.rs` holds `docs/web-lane.md` §13 and
  §16 to the code and `worker/scripts/lane-variables.test.mjs` holds §12; any edit to that file keeps them green.
  No document spells the owner's e-mail address or any secret value (the lane document's own rule).
- **Figures.** Every number the README or the method page states is a recorded figure with its record named,
  quoted as recorded; a one-sided bound is called one-sided; an observed maximum is "max observed under harness
  X", never a bound (ledger L9); no direction is asserted where none was measured.
- **Secrets never enter the tree**, and a secret that did would be rotated, not scrubbed (`.gitignore`'s own
  rule: "a credential in history is a credential rotated").

## 5. The window: the order of operations at the switch (S4c)

The one place where order matters. Nothing is public and nothing is open until the step before it has been read.

1. S4c's PR is merged to master; the whole-branch review is done; the fifteen-gate roster has been run at the
   merge head and recorded.
2. Stage 2, `worker/RUNBOOK.md` §23, verbatim, with the owner present: the keyed build with the seven values,
   `YOKOHAMA_TURNSTILE` on in both wrangler files, the edge rule enabled by its id, the deploy, the smoke, the
   hostname off the Access application last, the stranger's readings off WARP, `DEV_DOOR_CLIENT_ID` deleted.
   §23's stage-2 record is written as its stage-1 record was.
3. The repository is switched to public, in the same sitting.
4. Immediately after: "Approval for running fork pull request workflows" set to require approval from all outside
   contributors; `test.yml` re-enabled (its `push` trigger on master and the corrected comment are S4c's commit,
   merged in step 1); private vulnerability reporting enabled; issues on, discussions off; content reports accepted;
   the description, topics and homepage verified; the social preview uploaded (the one setting the API does not
   expose — the owner does it in the browser); the community checklist read.
5. The readings from the outside (§8.5), then the close, pushed to master as the project's closes have been.
6. Branch protection on master (pull requests only, no force-push) as the window's last act, after the close is
   on master. From then on every commit to master arrives by a pull request, the closes included.

Rollback: the door by §23 step 9 (the hostname back on the Access application first). The repository can be set
private again from the same setting, but a switch back does not unpublish what was forked or cached in the
interval, which is why step 3 waits for step 2's readings.

## 5a. S4-pre — the web lane's leftovers (branch `security-s4-pre`, off master after the stack lands, before S4a)

The owner's middle course (§2.22). Its specification is the write-up's Issues 1 to 4 and its Part 2, committed on the
branch as `docs/plans/<date>-security-s4-pre-spec.md`, trimmed from the gitignored draft so the plan cites a tracked
document; the plan is written against it by the project's workflow, and the cycle lands by the branch rule of §4.

1. **Issue 1, the seat-aware admission.** The layer says whether the caller is seated from the one read it already
   makes (an `events_at` sibling; no second table read, no new field on the wire object the committed corpus pins);
   the NAS's `Streams::admit` takes the bit, with a count of OPEN seated streams per room released with the slot, the
   maps still empty for a quiet room; the Worker module's reply carries the bit as a host-internal field, `admit`
   tags a seated socket, `capVerdict` gains the watchers' arm over OPEN sockets under that tag; the fifth sentence in
   the owner's words; classification by the caller's standing when the socket is admitted, said in the code and the
   lane document with a cell. Unchanged: the per-caller 8, OPEN-only counting, the 4429 and 429 shapes, the lobby's
   caps. Two code tasks, Opus implementer and Opus review.
2. **Issue 2, the marker head** for a table closed before it started: `worker/src/marker.ts`, the Worker alone (the
   NAS has no marker); the owner's wording; the write-up's option A, no new stored field, so no rollback hazard.
3. **Issue 3, the NAS's "a keyed build is the open door's" sentences** in the start refusal, its comments, the test
   literal beside it and `deploy/update.sh`.
4. **Issue 4, the secrets guard** refusing a Worker file with no door or an unknown door.
5. **The records task.** The nine "S4's" sites re-recorded (the write-up's Part 2 (a)–(d) and the five more it
   found, `docs/web-lane.md` lines 998, 1002 and 1004, the board's S2b paragraph, `worker/RUNBOOK.md` line 273); the
   lane's §8 "Still owed" paragraph brought to the present (stage 1 live; the fresh-history and art preconditions
   answered by §2.1 and §2.3; "on a public Worker that starts empty" retracted); D32's closing clause; the passkeys line
   (R-S3-6 dismissed them on 2026-10-20; the board's item 11 closes); the board's items 10 and 11 and S3's item (c).
   Issues 5 to 7 posted with `gh issue create` — outward-facing, on the owner's go — after the six labels are created,
   in the layout of §2.24, and cited by number from the sentences that defer to them.
6. **The suites and the close.** The web lane's per-edit set after every task (`docs/web-lane.md` §2; the five fenced
   crates and `SEMANTICS_REV` do not move, so no roster); the whole-branch review, the fix wave, the scoped
   re-review; the PR and merge to master. No deploy of its own: the wave ships with stage 2's `0.7.3` on both hosts
   (§2.23), and Issue 1's live reading joins the window's (§8.5).

## 6. S4a — the tree (branch `security-s4a`, off master once the stack has landed)

What a reader will find in the tree, and what a contributor needs, before the front door is written. Every
destination the README will point at exists at this cycle's close.

1. **The merges, first.** PR #56 → `security-auth`; a PR `security-auth` → master, merged on the owner's word
   (no new review: every commit on it was reviewed in its cycle and is what production runs). Then the one-commit
   branch `docs-language-school-hotfix-handover` → master (docs only: the handover document, two decision-log
   rows, a board track; conflicts in the board and the log resolved by keeping one board and appending the rows).
   `security-s4a` is cut after both.
2. **`tts-assets/` and the English rulebook leave the tip.** `git rm -r tts-assets` and
   `git rm analysis-1/00-yokohama-rulebook.pdf`, by the orchestrator. This repository's own files inside that
   tree move rather than leave: `parse_tts_mod.py`, `EXTRACTION_SUMMARY.md` and `asset_mapping.json` go to
   `docs/tools/tts-extraction/` with a one-paragraph header saying what the sheets were and that they are not in
   the tree. The mod's own save file (`1840951801.json`) and every sheet and PDF leave. Blast radius, measured
   (Appendix A): `docs/tools/mkcardart.py` reads the sheets (it keeps working for anyone who obtains them; its
   docstring says where they come from and that the tree does not carry them);
   `crates/yokohama-web/assets/art/manifest.json` carries a provenance string `"sheets": "tts-assets/organized/"`
   (re-worded to say the sheets are outside the tree; the wire corpus is re-read for the string before the change
   is called inert); `crates/yokohama-web/src/tables.rs` cites both PDFs in doc comments (re-worded to name the
   editions, not paths); `deploy/Dockerfile.dockerignore` lists the folder (harmless; the line goes). The live
   documents that name the paths — `CLAUDE.md` (the licensing paragraph and the edition paragraph), `ASSETS.md`,
   `README.md`, `docs/handover.md`, `docs/web-lane.md`, the KB's `00-README.md` — are re-pointed: the rulebook
   becomes "the 2017 Tasty Minstrel English rulebook, which a contributor obtains; its page numbers are the ones
   the record cites", and the owner's edition "the 20-page edition in the Tabletop Simulator upload, whose setup
   steps are on its p.5". The oracle independence rule's "rules sources" clause is re-worded the same way; the
   rule itself does not change. The twenty-three dated documents that cite the paths are not edited.
3. **The notice set.** `LICENSE` unchanged. `NOTICE`: the rights-holder paragraph rewritten in live interaction with a human in the loop. `ASSETS.md` rewritten as a notice of facts, in this order: what
   is third-party and where (`crates/yokohama-web/assets/art/`, `web/public/help/`, and — closing C4 —
   `worker/docs/img/` and any README picture that shows the art), whose it is (the lineage), that this project is
   unaffiliated with and not endorsed by any of them, what a fork must do, the two OFL font families. Gone: "private and invite-only",
   "the planned public split" and its fresh-history paragraph, every hedge about legal conclusions. The copyright
   sweep (§2.7) reads every remaining line in the tree that mentions the subject and reports what it found; it
   edits nothing dated unless the line argues a position.
4. **The community files.** `CONTRIBUTING.md`: how a change lands here (a branch, a PR, the per-edit suites and
   the lint gate named, the web suites when the layer is touched, the whole-branch review, the STOPs in force —
   `SEMANTICS_REV`, the oracle, the freezes, the pre-registrations), the corpus caveat (`YOKOHAMA_BGA_CORPUS_OPTIONAL=1`
   for a clone without the captures), the rules sources, and the ask of §2.6 in its own paragraph: AI-assisted
   work is welcome; the person submitting is comfortable with what they send and comfortable discussing it.
   `SECURITY.md`: report through GitHub's private vulnerability reporting, what to expect, what is in scope (the
   two hosts, the Worker, the CLI). `.github/ISSUE_TEMPLATE/`: a bug report (which surface, which version, the
   steps), a rules question (the engine disagrees with the table — the class of report this project most wants),
   and a `config.yml` that routes security to the private channel. `.github/pull_request_template.md`: the suites
   run, the docs updated, the sentence from §2.6 as a checkbox. The issue templates carry the layout the wave's
   posted issues already use (§2.24). No `CODE_OF_CONDUCT.md` in this cycle (§10).
5. **The ledger's S4 items.** D17: `release.yml`'s permissions, action pins and tag rules, "just before public"
   (R-S3-3), done before the first tag is pushed to a public repository. D20: the two real handles and the chat
   line live in the tracked fixture `crates/yokohama-bridge/tests/data/excerpt-895724349.ndjson`, inside escaped
   frame bodies (one `chat` frame, one `player_name` each; the documents under `docs/plans/2026-08-08-*` already
   read `Player_A`/`Player_B`); the fixture is redacted in place, the ledger row records the edit, and its readers
   re-run: `crates/yokohama-bridge/tests/feed_preprocess.rs` and the CLI's `bga/*` modules through
   `cargo test -p yokohama-cli`. C4 closes with the notice (item 3). The three sentences that still read "S4's" in
   older rows — C1's "a layer rule for seats is S4's (decision D-2)", D3's "a seat-aware admission is S4's (the
   Task 1 review)", and the lane's §8 "S4's controls (the owner's R-3)" — are re-recorded as §11 question 1 is
   answered; none is taken as code (§3's non-goal).
6. **The sweep of release notes, PR bodies and the owner's address.** All ten releases (`gh release view`) and
   fifty-five pull requests (`gh pr view`) read with the patterns of Appendix A; anything found is edited in place
   and recorded. The owner's e-mail address is spelled in five tracked files today (Appendix A): in three test
   files as the example of a non-`.test` address the minting fence must refuse — replaced by an `example.com`
   address, the cells unchanged in meaning — and in one ledger row and one runbook record, re-worded to "the
   owner's address" as the lane document's rule already requires. The address stays in the author field of 2,399
   commits: that is public with the history, and the only remedy is the rewrite §2.1 excludes.
7. **The branch backup and the lab.** In this order: the private repository created (same account; name a call,
   §10) with Actions DISABLED before anything is pushed — `release.yml` runs on `v*` tags, and ten tags would
   start thirty jobs on a private repository's metered minutes and create ten releases there; it is added as `lab`
   and set as `remote.pushDefault`; the six remote-only branches get local branches; every local branch and every
   tag is pushed to `lab`, the art lane's branch included (§2.18); a bundle of every
   ref is written outside the repository and verified (`git bundle verify`); the fifty merged branches are deleted
   on origin, by the orchestrator, from a list the close records. Local branches are kept (the owner's word). The
   second workstation receives the same two-remote configuration (a recipe in the close; the board names it).
8. **The art lane moved aside.** A worktree for `art-replacement-spec` beside the adjudicator's
   (`git worktree add ../yokohama-assistant-art art-replacement-spec`); the untracked `docs/brand-guide/` and
   `docs/tools/art/` trees moved into it, not deleted (the counts read before and after: 350 files under
   `brand-guide/`, of which 39 are tracked on the branch); the empty `.kiro/` and the `__pycache__` directories
   removed by the orchestrator. The lane's rule is unchanged toward origin: no push there, no PR, no merge; its
   documents are read for copyright talk before any push to origin, in that lane, not this one. The owner ruled
   on 2026-10-22 that the lab is a backup, not the push the rule forbids (§2.18).
9. **The repository's metadata.** Description (a call, §10), at least five topics, the homepage
   `https://yokohama.moore.nyc`, set through `gh`; the social preview image produced (a capture of the game
   screen) and left for the owner to upload at the switch.
10. **`docs/web-guide.md` — the browser client, for the player, the host and the reader.** Markdown beside the
    CLI guide, no HTML twin (§10). Sections: what it is and what it can do; playing at the hosted site (the door:
    the bot with no account, friends, this device; the sign-in; the lobby and a table; the game screen; saves,
    the archive, offline play); hosting your own (one Docker command for a home network, then the pointers to
    `deploy/README.md` and `worker/README.md`); what is by design (hidden information, the bot's seat, the legal-
    list gate, the Host rule). Its sources are the README's browser-client section (which it replaces), the lane
    document's §14 and §15, and the help panel; its pictures are the help screenshots already in the tree. The
    one Docker command is `deploy/run-local.sh` (a call, §10), built in this cycle beside the guide so the guide's
    sentence is true at its own review: it wraps `deploy/README.md`'s §3 build and §4 run —
    `YOKOHAMA_WEB_IDENTITY=open`, loopback in the default host set, a named volume. The image is `linux/amd64` by
    design, so a first build on Apple silicon runs under emulation and takes minutes; the guide says so.
11. **`docs/computer-player.md` — the computer player, current.** From the map, the freeze package, the
    execution ledger and the board: what a contestant is, the named identities, V4 as production and provisional,
    `suggest` and `why`, the results ladder in full (§7.3), the bar not claimed in the map's words, what is next
    (the V4 vetting on the board's Track 1, then V5). The README's status section (dated 2026-08-30) is retired
    into it.
12. **The relocation map.** A table in S4a's close: every section of the old README and every removed or moved
    path, and where its content now lives (the pattern of `docs/plans/2026-08-19-docs-consolidation.md`).
13. **The author address, going forward.** `user.email` set to the GitHub no-reply address in this repository's
    configuration on both workstations (and so for every push to the lab); "Keep my email addresses private" and
    "Block command line pushes that expose my email" switched on in the GitHub account; the close names the first
    commit under it. History is not rewritten (§2.20).

At close: the per-edit suites, the web suites (items 2 and 5 touch what the layer compiles), the lint gate, the
whole-branch review, merge.

## 7. S4b — the front door (branch `security-s4b`, off master)

1. **`README.md` from a clean sheet**, written for the reader of §1 and shaped by the research (Appendix B):
   under about 200 lines; the first line says what it is and who it is for in the plain nouns a search would
   match; one picture; "Try it" — the hosted site, where the bot plays in the browser with no account — then the
   quick start: the CLI (the release binary, or `cargo run -p yokohama-cli`) and the browser client (one Docker
   command), each with its expected first screen, and the test line with the corpus opt-out; what is inside (the
   crates in five lines); status, honestly (the engine complete at `SEMANTICS_REV` 7; V4 the shipped bot; the
   bar not claimed; next, V4's vetting and V5); the record (guidingpath, the decision log, the docs index, the
   method page); contributing, security, licence in one line each (Apache-2.0 on the code; the game's components
   are the rights holder's; not affiliated). Three or four badges that link to live data: the release and the
   licence for certain; the test workflow, live on master from the flip (§2.19), written in S4b against the
   workflow S4c re-enables. The
   release path is real (`v0.7.2` carries the three CLI packages, the guide and `SHA256SUMS`); the macOS binary is
   ad-hoc signed, so a browser download lands quarantined — the quick start fetches with `curl` and unpacks, or
   builds with `cargo run`, and the walk (§7.7) is the proof either way. No status narrative, no counts, no roadmap
   framed as features. Links out for everything deeper. The old README is in history and its content has a home
   (S4a's relocation map).
2. **`docs/README.md` — the docs index.** One line per document a reader may need, grouped: the guides (CLI,
   browser client, computer player), the authority (`guidingpath.md` and its archives, the decision log), the
   board, the lane records (the web lane, the security ledger), the plans directory explained (dated, frozen; how
   to read a cycle: spec → plan → close), the knowledge base, the tools.
3. **`docs/method.md` — "How the work is organized."** The workflow (brainstorm → spec → plan → execute; the
   per-edit suites and the fifteen-gate roster; subagent-driven implementation; the whole-branch review; the
   close document as the record of record; the decision log and the `SEMANTICS_REV` contract; pre-registration;
   the rules of evidence of §4). Then the results ladder: one row per strength milestone from the first rung to
   V4 — the claim as pre-registered, the figure as measured with its interval, the record that holds it — drawn
   from `guidingpath-archive-phase3.md`, the 3.3 freeze package, `docs/plans/2026-08-04-phase3.4-architecture-decision.md`
   (MCTS measured and not adopted), the V3 null close and `docs/plans/2026-08-09-phase3.5c-execution-ledger.md`.
   Beside it the end result as it stands: the live site, the suites and the gates, named by their commands. No
   model table. The page names that reviews run across models where the record says so, and leaves it there.
4. **`CLAUDE.md` refocused.** The browser-lane bullet cut back to what it already says it is — "a pointer and the
   three facts that cross into the AI lane" — with the security era's standing sentences moved to
   `docs/web-lane.md` §4 where they are not already there; the "Where things stand" paragraph re-pointed at the
   AI lane; the licensing sentence re-pointed at the notice's paths; one new standing rule for the public
   repository: work in progress lives on the lab remote, and a branch reaches origin with its plan. Standing
   rules only; no narrative.
5. **`guidingpath.md`.** Current state rewritten, one screen, for the AI lane's re-entry; the roadmap updated
   (the web lane complete, the security project closing, the public release in progress, the art lane on its
   branch, the Language School hotfix queued, V4's vetting, V5). The decision log is NOT cut here: an archive
   closed in S4b could not take the rows S4b and S4c write afterwards, and an append-closed archive is never
   reopened. The cut is S4c's (§8.2).
6. **The board, `docs/handover.md`**, rewritten for the session after S4c: Track 1, V4's close-out, first; the
   Language School hotfix; the art lane (local, by its rule); the web lane complete; the security project closed
   with a pointer to the ledger and the S4 close; the owner rulings outstanding carried over; the entry
   artifacts by lane; "Gates at HEAD" with the roster's reading from S4c's merge head once it exists.
7. **The clean-eyes walk.** A Haiku agent with a fresh context and no knowledge of the repository beyond the
   README, in a scratch directory holding a fresh clone (or `git archive` export) of the branch head, follows
   the README's quick start for the CLI and the browser client on this machine (Docker is installed here) and
   reports every point of friction in its own words, verbatim commands and output included. The orchestrator
   fixes the README, or the script, and the walk runs again until it is clean. The helper the owner anticipated,
   `deploy/run-local.sh`, is built in S4a beside the guide that names it (§6.10); the walk is what proves it; the
   CLI needs none beyond the binary or `cargo run`. The final transcript's path is recorded in the close.
8. **The cross-model read.** A model from another family, through the Codex plugin, reads the README, the
   browser guide and the computer-player page as the stranger of §1 and reports where they are unclear, where
   they overclaim, and where a command would not run; findings are triaged as a review wave.
9. At close: the suites, the lint gate, the whole-branch review in opus, merge.

## 8. S4c — the flip (branch `security-s4c`, off master)

0. **The pre-flight.** The NAS sidecar sync read (§2.25): whether the 3.5c amendment's standing sync runs, what it
   carries, and when it last ran — the gitignored material's copy confirmed, or arranged, before the window; the
   board's item 8 closes with the reading. The wave's Issue 1 cells green at the merge head; D17 closed; the issues'
   numbers cited where the record defers to them.
1. **The `docs/temp/` triage.** A table of every top-level item (Appendix A names them) with a recommendation —
   stays private, publish under `docs/` with a provenance line, or discard — and the owner's mark; the default is
   private. Whatever is published is copied in by its own commit and listed in the close.
2. **The ledger and the record.** C4, D17 and D20 read closed from S4a; C1's, D3's and §8's "S4's" sentences
   re-recorded as §11 question 1 is answered, each row saying so; the ledger's `Updated` line; the security
   project's closing row in guidingpath; the lane document's §1 and §8 brought to the state after stage 2 — §8's
   "a fresh-history repository" precondition is superseded by §2.1 and §2.3 (the history is kept whole and the art
   crops stay in the tree, by the owner's ruling), its "rows 108 and 119 read with a second identity" is read at the window (§8.5),
   and its per-person ceiling sentence is re-recorded with the C1/D3 answer. **The decision log's archive cut:**
   at the close, a new append-closed archive holding VERBATIM every row older than this close that the live file
   still carries (the web lane's last two cycles and the security project, 2026-09-29 on), the live file keeping
   the index and its rows from this close on, the relocation recorded in the index as the earlier archives were.
3. **The version.** Stage 2 commits `YOKOHAMA_TURNSTILE` on. The lane's rule (`docs/web-lane.md` §7) requires a
   bump only "at every release that can change what a save means or which bot plays", which this does not; but
   it also says "Tag `vX.Y.Z` on the commit the hosts run", and the practice has tagged every deployed window
   (`0.7.0`, `0.7.1`, `0.7.2`), so the Worker has never run an untagged commit. The call (§10), confirmed by the
   owner's "all together" (§2.23): stage 2 ships as `0.7.3` by §7's eight steps — the wire fixtures regenerated, the
   first `release.yml` run on a public repository (D17 done first) — on BOTH hosts, since the release carries the
   wave's NAS half (§5a); the stranger's `/api/health` reads a version that names the window on either host.
4. **The window**, §5, in order, with the owner present. Stage 2's record written in `worker/RUNBOOK.md` §23;
   the switch and the settings of §5.4 recorded in the close.
5. **The readings from the outside**, after the switch: a fresh clone of the public repository builds and its
   suites pass with the corpus opt-out; the Haiku walk repeated from the public URL and the public clone; the
   door read as a stranger (a fresh profile off WARP, no cookies: the door, the bot game with no account, the
   sign-in by e-mail code to the test identity of §2.20, a dealt table, the archive, the sign-out — the rows
   stage 1 left unread, "a dealt table and the socket under the cookie", among them; and the wave's Issue 1 read
   live as far as a window can: the new sentence at a watchers' figure the reading can reach, a seated reconnect
   admitted); every
   link in the README and the docs index
   resolves; the community checklist read; `remote.pushDefault` read on both workstations.
6. **The close.** The security project's close document: what the four cycles did, the readings, what is
   deferred (passkeys behind a custom domain, D32 if still open, the art lane's eventual merge); the board's
   final re-point at the AI lane; one new standing rule in `CLAUDE.md` if the cycle earned one.

## 9. Left to the builder

The README's wording and picture; the badge set; the docs index's order; the exact section order of the two
guides; the topics list beyond the examples below; the helper script's internals; the shape of the triage and
relocation tables; task tiering; the lab repository's settings beyond "private, same account"; the social preview
capture; the plan's split within a cycle if a plan grows past what one agent reads whole (the standing rule; flag
it at planning).

## 10. The calls made in this document (for the owner to check at the read)

1. **The English rulebook PDF leaves with `tts-assets/`.** Same footing: the publisher's printed material, read by
   nothing in the application; the record cites page numbers, which a contributor's own copy answers.
2. **No per-file SPDX headers.** The board's Track 2 left headers as "a public-split-day decision"; the root
   `LICENSE` and `NOTICE` carry the grant, and nine crates' manifests inherit the licence from the workspace.
3. **No `CODE_OF_CONDUCT.md` in S4a.** A code of conduct names a private reporting channel for conduct, and the
   project has none that is not the owner's inbox (which no document spells). GitHub's "repository admins accept
   content reports" is switched on at the flip. Revisit when a second contributor exists.
4. **The browser guide is Markdown only.** The CLI guide's HTML twin exists because the binary ships it offline;
   nothing ships the browser guide, and the client has its own help panel.
5. **One screenshot in the README, no terminal recording** in this cycle: a recording is new tooling, and the
   walk tests commands, not pictures.
6. **Issue and pull-request templates are included**; the templates are where §2.6's ask becomes a checkbox.
7. **The Language School handover merges in S4a**, so master's board shows what is queued.
8. **The lab repository is `yokohama-assistant-lab`**, private, same account; `remote.pushDefault = lab`; master
   tracks origin; both workstations configured alike.
9. **The helper script is `deploy/run-local.sh`**, beside `smoke-image.sh` and `update.sh`.
10. **The new guides are `docs/web-guide.md` and `docs/computer-player.md`; the index `docs/README.md`; the method
    page `docs/method.md`.**
11. **The description:** "Rules engine, CLI tracker and browser client for the board game Yokohama, with a
    computer player built to play to win." **Topics to start from:** `rust`, `board-game`, `game-engine`,
    `game-ai`, `yokohama`, `webassembly`, `cloudflare-workers`, `react`, `cli`.
12. **The dated documents are not edited for the removed paths** (§4); the two named exceptions are D20 and a
    sweep hit that argues a position.
13. **The decision log's archive is cut in S4c at the close**, not in S4b (§7.5, §8.2).
14. **Stage 2 ships as `0.7.3` on both hosts, the wave included** (§8.3, §2.23).
15. **The BGA bridge is presented as what it is and not on the first screen:** one line in the crate map and one
    in the docs index ("a read-only capture of the owner's own Board Game Arena sessions, translated by the CLI's
    `import`"; `docs/bridge-setup.md` listed); `analysis-1/` and `deploy/` keep their names and places.
16. **The notice keeps one fact about the transcribed data:** the rules references and card data under
    `analysis-1/yokohama-kb/` are this repository's own prose and data structures; the rules and card text they
    transcribe are the game's. The sentence that "records that fact rather than adjudicating it" goes.
17. **Branch protection is the window's last act**, after the close is on master (§5.6).
18. **`deploy/run-local.sh` is built in S4a** beside the guide that names it (§6.10), and the walk proves it (§7.7).
19. **Actions are disabled on the lab before the first push** (§6.7).
20. **The owner's address leaves the five tracked files** (§6.6); the author field is left as it is.
21. **The wave is `security-s4-pre`**, and its specification is a trimmed, committed copy of the write-up's Issues 1
    to 4 and Part 2 (§5a); Issues 5 to 7 keep their text in the gitignored draft until they are posted.

## 11. The builder check

Filled after a fresh agent read this document as the planner. Questions it raised that the conversation had
already answered were folded into the sections above; questions a sensible builder could settle were settled and
added to §10; questions only the owner can answer are listed here for the read.

**The check (2026-10-22).** A fresh agent read the document as the planner: "buildable as written"; eight ranked
questions, three internal inconsistencies, one mislocated item and two traps. Folded into the sections above:
the D20 location (§6.5), the archive boundary (§7.5 → §8.2), the lab's Actions (§6.7), the script's cycle (§6.10),
the protection order (§5), the release question (§8.3, a call), the bridge's placement and the notice's data line
(§10). Four questions went to the owner on 2026-10-22; three are ruled (§2.18–§2.20) and one is open.

**Open — the web lane's work "behind us" at the switch (§2.21).** The owner wants the lane's open items either
tracked as GitHub issues and deliberately delayed, or cleared in a precursor wave with its own PR before S4a. The
items, and what each involves, read from the ledger, the lane document, the board and the code on 2026-10-22:

- *Code, small.* (a) The `waited` head's wording (S3's item d): one sentence with its cells on both hosts, one
  task. (b) The auth cycle's four recorded-forward refactors — the `assets.rs` sentences about "a key is the
  door", the secrets guard's door reading, the `index.ts` sign-in block as its own module, the three `base64url`
  copies — two tasks, cosmetic. (c) The two cycles' deferred minors (the auth cycle's eighteen at its final wave,
  S3's list in its design §7.4: the smoke calls without a hostname, the stored-id key's cell, the client's 429
  sentence as a literal, the guard's items) — wording and test polish, read once, each folded or issued.
- *Code, medium.* (d) The seat-aware admission (D3): today a full room — 64 sockets on the Worker, 64 streams on
  the NAS — refuses a seated player as it refuses anyone until a slot closes (`streams.rs`: "it judges a seated
  player as it judges anyone"); a seat-aware admission reserves the seated players' slots so a crowd cannot lock
  a player out. Both hosts, a rule and its cells each, four to six tasks. Under the provider-signed door the
  crowd needs eight signed-in addresses on one table, since sockets are capped at eight per address.
- *Resolved, record only.* (e) D32 (the board's item 10): the ledger's row already reads "accepted (R-S3-4) …
  the edge rule's `/api/me` leg LIVE 2026-10-20 (the zone's one rate-limit rule; read 21 × 401 then 429), the two
  testers asked to open the front page once", and the owner confirmed on 2026-10-22 that the testers have; the
  row gets its closing clause, nothing is built.
- *Superseded by the auth cycle, record only.* (f) C1's "layer rule for seats": "a seat is a reservation, not a
  caller: the layer does not judge a seat's user by the door", so under the OPEN door anyone who types a seat's
  name may act in it. The public Worker's door is provider-signed, which closes the public case; under a
  self-hoster's open door it is the property of that door, stated in the web guide's "what is by design".
  (g) §8's per-person ceiling (R-3): ruled for an open-door public Worker that no longer exists; writes are keyed
  by verified address now, so per person is per address; the anonymous archive post stays by design.
- *The owner's own, not code.* (h) The providers' credentials in WorkOS (Google, GitHub, Apple) and the staging
  API key retired. (i) S3's items: the LAN DNS record for the test host, the off-WARP reading there, the two
  testers' front-page visit, the owner's browser reads (the reopen, D2's 429, the sweep line, the pinned
  browser's update). (j) Stage 1's unread rows, "a dealt table and the socket under the cookie" — folded into the
  window's readings (§8.5).
- *Deferred, accepted undone* (the owner, 2026-10-22). (k) Passkeys and the AuthKit custom domain (the board's
  item 11): recorded as such; no issue.

*Ruled (2026-10-22, evening).*  a Sonnet agent drafted
`docs/temp/2026-10-22-web-lane-leftovers-issues.md`: seven issues in GitHub-ready form, the re-recordings drafted as
sentences, two sizings and fourteen findings. On that read the owner chose the middle course (§2.22): Issues 1 to 4
built as the precursor wave `security-s4-pre` (§5a), Issues 5 to 7 posted at its close in the contributor layout
(§2.24), the wave shipping with stage 2's `0.7.3` on both hosts (§2.23). The write-up's findings that reach this
design: the "S4's" deferral sits at nine sites, not four (§5a.5); the marker head is the Worker's alone; passkeys
were already dismissed by R-S3-6 and the board's item 11 was never closed; D32 is accepted in the ledger and still
asked on the board; the auth cycle's recorded-forward list is eight items, four of them written only in gitignored
briefs, which the issues now carry; and the board's item 8 is the AI lane's NAS sidecar sync, read in S4c's
pre-flight (§2.25, §8.0).

**Ruled (2026-10-22).** The art lane's branch goes to the lab (§2.18). `test.yml` runs again from the flip with
a `push` trigger on master; the lab runs no Actions (§2.19). The stranger's code goes to a test address of the
owner's, and commits are authored under the no-reply address from S4a on, with no rewrite (§2.20).

---

## Appendix A — the recon of 2026-10-22 (measured at `56460688`; the plans do not re-run these)

**Tools:** gitleaks 8.30.1 (installed for the scan), jq, pdftotext, gh, docker present; no cargo-deny.

**History.** 2,453 commits since 2025-12-07 (2,311 on master); ten tagged releases, `v0.1.0-beta.1` →
`v0.7.2`; two author identities (the owner's address and the GitHub no-reply); 55 pull requests. gitleaks scanned
2,391 commits: 21 hits, every one a fixture or a false positive (unit-test secrets of the form `0123456789abcd…`
in `worker/scripts/live-replay.test.mjs`, `mint-session.test.mjs`, `session.unit.test.ts`,
`web/e2e/harness.node.test.ts` and the plan that quotes them; a WebSocket nonce in a curl recipe in both
runbooks; the Access audience tags in the three wrangler files; a fixture password in `web/src/wire-script/walk.json`;
a device key in `identity_cases.json`; three false positives on identifiers). A second pass, a pattern grep over
every added line in history (private-key blocks, `sk_live`, AWS, GitHub and Slack token shapes, `CLIENT_SECRET`,
`API_TOKEN`, `SESSION_SECRET`, `CF_ACCESS_CLIENT_SECRET`, `YOKOHAMA_DENIED`, bearer tokens) found the same fixtures
and nothing else. No `.dev.vars`, `.env`, key or credential file was ever added.

**Citations.** 1,750 hex tokens of 7–12 characters in Markdown, of which 1,460 resolve to commits in this
repository, cited across 178 files; the heaviest: `docs/plans/2026-08-09-phase3.5c-execution-ledger.md` (434),
`.superpowers/sdd/progress.md` (181), `docs/security-ledger.md` (109), `guidingpath.md` (88), `docs/handover.md`
(66). 32 more resolve from code and configs.

**Licence.** The workspace sets `license = "Apache-2.0"`, inherited by all nine crates; `gen-data`, `yokohama-web`,
`yokohama-wasm` and `yokohama-worker` carry `publish = false`. 96 external Rust packages: 55 `MIT OR Apache-2.0`,
24 `MIT`, 5 `Apache-2.0 OR MIT`, 2 `MIT/Apache-2.0`, 2 `BSL-1.0` (`clipboard-win`, `error-code`), 2
`BSD-2-Clause OR Apache-2.0 OR MIT`, `Unlicense OR MIT` (`memchr`), `MIT AND BSD-3-Clause`,
`Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`, `Apache-2.0 OR BSL-1.0` (`ryu`), `Apache-2.0`,
`(MIT OR Apache-2.0) AND Unicode-3.0`; none missing a licence field, none copyleft. npm: the browser client's
runtime dependencies are `react` and `react-dom`; the Worker package declares none. Dev-only trees carry
`caniuse-lite` (CC-BY-4.0, data), `lightningcss` (MPL-2.0) and `@img/sharp-libvips-darwin-arm64` (LGPL-3.0-or-later);
none reaches a shipped bundle. Fonts: Fraunces and JetBrains Mono under OFL 1.1, recorded in `NOTICE` and
`ASSETS.md` with a test that holds the two licence copies identical.

**The rights holder.** The Tabletop Simulator edition's PDF names only "Game design: Hisashi Hayashi"; the English
PDF names "Produced by: OKAZU Brand". BoardGameGeek lists the 2017 English edition under Tasty Minstrel Games
(artists Adam P. McIver, Ryo Nyamo) and the game's publishers as OKAZU Brand and Synapses Games among others; the
Asmodee Canada designer diary of 2024-08-09 announces the 2024 edition by Synapses Games with new illustrations.
The TTS mod this repository's sheets came from is dated 2021-10-16 (Steam Workshop 1840951801).

**The tree.** `tts-assets/`: 118 tracked files, 60 MB (84 jpg, 28 png, two PDFs, the mod's save JSON, a mapping
JSON, one script, one summary; `images/` 20 MB raw and `organized/` 25 MB sorted are the same sheets twice; the
mod JSON holds component nicknames and no person's name). `crates/yokohama-web/assets/art/`: 132 files, 7.4 MB.
`web/public/help/`: five screenshots. README pictures: four CLI captures under `docs/images/` (text renders, no
art), three under `worker/docs/img/` and two under `web/public/help/` that show the art (C4). `docs/temp/`
(gitignored): 484 MB, 1,203 files — `web-review` 370 MB, `assetreplacement` 57 MB, `security-review-2026-10-08`
31 MB, `step6-artifacts` 15 MB, `sdd-archive`, `owner-games`, `step10/11/12-artifacts`, `step13-roster`,
`webresearch`, `2026-10-04-technology-review-evidence`, `scratch`, and some forty small files (the step-9
charters and assessments, the independent progress review, the research prompts, the engineer primer, the status
deck, the Databricks notes, the refit test drive). Untracked in the main checkout: `.kiro/` (empty),
`docs/brand-guide/` (350 files, 114 MB; 39 tracked on `art-replacement-spec`), `docs/tools/art/` (only `__pycache__`
on disk at HEAD; 29 files tracked on the branch).

**Branches.** On origin: 50 merged into master; unmerged `security-auth` (60 ahead of master), `security-s3`
(92; 32 ahead of `security-auth`), `docs-language-school-hotfix-handover` (1). Local only: `art-replacement-spec`
(49 ahead, never pushed), `security-s1b`, `v4-s12a-escrow-release`, `v4-s12b-ship`, `v4-s12c-capcount`,
`w-a1-void-units`. Remote only: `docs-consolidation`, `phase3.5c-families`, `phase3.5c-harness`, `phase3.5c-v3`,
`phase3.5c-w2-tram`, `phase3.5c-w5`. Worktrees: the main checkout and `../yokohama-assistant-adjudicator`.

**The public face.** Visibility private. Description "AR assistant for the Yokohama board game with Rust game
engine and MCTS AI"; no topics, no homepage. Workflows: `test.yml` on `pull_request` and dispatch; `release.yml` on
tags (Ubuntu, macOS, Windows); `web.yml` and `docker.yml` by dispatch. On GitHub, `test.yml` and `docker.yml` read
`disabled_manually` (the former since 2026-09-28, by the workflow's own comment: "the owner's minutes at 90% for
the month"); `release.yml` and `web.yml` are active. `v0.7.2`'s release carries the three CLI packages, the guide
and `SHA256SUMS`; the macOS binary is ad-hoc signed. No CONTRIBUTING, SECURITY, code of conduct or templates.

**Personal data and fixtures.** The owner's e-mail address is the author of 2,399 commits and is spelled in five
tracked files: `docs/security-ledger.md` (row C1), `worker/RUNBOOK.md` (line 2682), and as the example a minting
fence must refuse in `web/e2e/harness.node.test.ts`, `worker/scripts/live-replay.test.mjs` and
`worker/scripts/mint-session.test.mjs`; fourteen commits in history added or removed it from a file. D20's two
real handles and chat line are in `crates/yokohama-bridge/tests/data/excerpt-895724349.ndjson` (read by
`crates/yokohama-bridge/tests/feed_preprocess.rs` and `crates/yokohama-cli/src/bga/*`), not under `docs/plans/`
as the ledger's older note said.

**Removed-path references.** In code and tooling: `docs/tools/mkcardart.py`, `crates/yokohama-web/assets/art/manifest.json`
(one provenance string), `crates/yokohama-web/src/tables.rs` (doc comments), `deploy/Dockerfile.dockerignore`,
`docs/tools/mkhelpart.py` (a sentence). In live documents: `ASSETS.md`, `CLAUDE.md`, `README.md`,
`docs/handover.md`, `docs/web-lane.md`, the KB's `00-README.md`, the three archives (verbatim; untouched),
`.superpowers/sdd/progress.md`. In dated documents: 23 files.



## Appendix B — the README research (2026-10-22)

The sources agree, from GitHub's own guidance to the empirical studies: Prana et al., "Categorizing the Content
of GitHub README Files" (Empirical Software Engineering 2019; 4,226 sections from 393 repositories: "information
discussing the 'What' and 'How' of a repository is very common, while many README files lack information
regarding the purpose and status"), and the 2022 study of 1,950 README files across ten languages (popular
projects' READMEs "are well organised using lists and images" and link-rich); the practitioner guides of 2025–2026
(GitHub Docs' best practices; RepoRanker; the 2026 best-practice compilations) converge on the same shape.

- A README is a gateway, not a manual: under about 350 lines; every section past usage halves its readership.
- The first line says what it is and who it is for, in the nouns a search would match; a logo, a badge wall or
  a table of contents above it buries the lead.
- Then, in order: one image or demo (terminal recordings beat screenshots for CLIs), install in three commands,
  a quick start with its expected output, a short why, honest status, links out for everything deeper, a licence
  line, one paragraph on contributing.
- The common gap is purpose and status; this repository's README has the opposite problem: 788 lines, 64 KB,
  everything present and both buried.
- Discovery lives outside the file: the description (keyword-first, 20–300 characters), five or more topics, a
  homepage URL, a social preview image. GitHub's community checklist for public repositories: description,
  README, code of conduct, contributing, licence, security policy, issue templates, pull-request template,
  content reports accepted.
- Three or four badges that link to live data; nothing decorative. A quick start that fails is the costliest
  defect a README can have, so it is tested on a clean environment.
