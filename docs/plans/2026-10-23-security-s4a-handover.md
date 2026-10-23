# Security S4a — the tree: the handover into its brainstorm and plan (written 2026-10-23, at S4-pre's close)

**What this is.** The entry document for the session that plans S4a — the first of the public release's three cycles
(the design `docs/plans/2026-10-22-security-s4-public-release-design.md`, approved by the owner on 2026-10-23: §6 is
S4a's content, §2 the owner's settled inputs, §4 the rules that bind every cycle, §9–§11 what is left to the builder,
the calls made and the builder check, Appendix A the recon measured at `56460688`). This document adds what the design
cannot know: where the tree stands after S4-pre, what S4-pre routed into S4a, the entry sequence, the short brainstorm's
agenda and the reading list. It is a pointer; the design is the authority, and §2 is not re-litigated.

## 1. Where things stand (2026-10-23, after security S4-pre)

- **master `00d93d72`** — the stack merged on the owner's word (PR #56 `30a9ff45`, PR #57 `00d93d72`): the auth cycle
  (stage 1 live since 2026-10-19) and S3 (`0.7.2` on both hosts and the private test Worker, 2026-10-20). Cloudflare
  Access still fronts every hostname; stage 2 is S4c's window (design §5, §8; `worker/RUNBOOK.md` §23).
- **PR #61 `security-s4-pre` → master is OPEN, mergeable and clean at `e76be966`**, awaiting the owner's word. The
  wave is NOT LIVE until stage 2's `0.7.3` (design §2.23). Its record of record:
  `docs/plans/2026-10-22-security-s4-pre-close.md` (§5 what it leaves — the reopen corner for the owner —, §7 its
  handover into S4a); the board's Track 4b; guidingpath's 2026-10-23 row; `docs/security-ledger.md`'s `Updated` line.
- **`origin/docs-language-school-hotfix-handover`** (`fd3c2074`, one commit ahead of master: 3 files, +285/−4 — a
  handover document, two decision-log rows, a board track) merges in S4a's first step (design §6.1, §10.7). A dry
  `merge-tree` against master read 0 conflicts on 2026-10-23; re-run it after #61 merges, since both touch the board
  and the log, and resolve by keeping one board and appending the rows.
- **Branches on origin:** 55 refs; 51 merged into master beyond master (Appendix A counted 50 before the stack merged;
  §6.7 re-counts at execution and the close records the list). Unmerged after #61: the Language School branch alone.
  `art-replacement-spec` exists LOCALLY only (the art lane's rule: no push to origin, no PR, no merge; the lab is its
  backup, §2.18).
- **Remotes:** `origin` only. No `lab` yet (§6.7; §10.8 names it `yokohama-assistant-lab`, Actions DISABLED before the
  first push, §10.19). `user.email` is still the owner's address on this workstation (§6.13 switches it, both
  workstations, with GitHub's private-e-mail settings).
- **Worktrees:** the checkout, and `../yokohama-assistant-adjudicator` (`phase3.5c-exact-adjudicator`, 5.2 GB). §6.8
  adds the art worktree beside it.
- **Untracked in the checkout today:** `docs/brand-guide/` (350 files), `docs/tools/art/` (27 files) — the art lane's,
  0 tracked on master (39 tracked on the art branch per the design) — and `.kiro/` (an empty directory holding one
  empty directory). §6.8 moves the two trees into the art worktree and removes the empty ones, counts read before
  and after.
- **`tts-assets/`:** 118 tracked files, 60 MB; `analysis-1/00-yokohama-rulebook.pdf` present. Both leave the tip in
  §6.2; the three repository-authored files inside `tts-assets/` move to `docs/tools/tts-extraction/`.
- **Tooling present:** gitleaks 8.30.1, gh 2.103.0, docker, jq, pdftotext (Appendix A).
- **The tracker:** #53 (the turn alert, `enhancement`); #58, #59, #60 (S4-pre's polish issues, in the contributor
  layout — Context, What happens today, The change, Where, Acceptance, Size, Record to update when it lands). Labels:
  GitHub's nine defaults plus `web`, `worker`, `nas`, `client`, `polish`, `security-ledger` (created 2026-10-23).

## 2. What S4-pre routed into S4a

- **The issues' style pass** once `CONTRIBUTING.md` and the templates exist (design §2.24, §6.4): #58–#60 are
  editable; the templates are written to their layout, and the three are re-read against the templates.
- **Two records items, parked at the wave's re-review** (the close §4): `docs/web-lane.md` §10's list of the deploy
  scripts' guards omits `deploy:private`; the first-deploy recipe (`worker/RUNBOOK.md` §2, the lane's §10, the Worker
  README's §9.3 — all three in §26's shape since the wave) says "record the file in `DEPLOYMENTS` first" only by
  implication. §6.10's "hosting your own" section says it outright, and the lane's §10 line is corrected when the host
  guide's pointers are written.
- **Four polish minors, parked** (the close §4): T2-m2 (`worker/test/caps.test.ts`'s helper doc says "this cell"),
  T3-m1 (`marker.unit.test.ts`'s mutant list omits the fourth), T4-m1 (the retired-phrase grep is line-bound;
  `assets.rs:56–57`'s wrapped, true negation is invisible to it — recorded, not reworded), T5-m3 (the secrets guard's
  door rows under no cell of their own). Fold into any task that opens the file; otherwise leave.
- **The reopen corner is the OWNER's, not S4a's code.** A stream or socket admitted for a seat holder whose seat the
  creator then reopens stays counted seated until it reconnects; in that corner a full room's 64 can refuse a current
  seat holder. Recorded on both hosts and in the ledger's D3 row; closing it would contradict the admission ruling
  (classification at admission, never again) and is a ruling and a cycle of its own. S4 is not code-focused (§2.21).
- **The lane's §2 counts** predate the auth cycle's and S3's files (vitest 1,586 in 76 and e2e 83 are S3's release
  readings and S4-pre's alike; `test:scripts` 796 in 32, the pool 302 in 14, worker 84, web 608 are S4-pre's). The
  re-read belongs to S4b's document refocus (design §7), not to S4a.
- **Recorded forward in the wave, to keep consistent:** S2b's D-7 ("a new Worker's first deploy runs with neither
  guard") is superseded by the `DEPLOYMENTS` record — the deploy guard runs on a first deploy, the secrets guard alone
  is skipped (`worker/RUNBOOK.md` §2; the board's S2b paragraph points forward). A guide that describes a first deploy
  says the same.
- **The wave's lessons, for S4a's briefs** (`CLAUDE.md`'s lessons list; the close §4): a cell's positive assertion
  pins a substring only the present sentence carries (P-15); a plan's grep expectation knows what the cell it checks is
  allowed to spell (P-14); a commit's trailer carries the IMPLEMENTER's own model (R-12); a fixer who finds the same
  defect at a site outside the brief reports it, and the same wave lands it (R-19).

## 3. The entry sequence for the next session

1. **Read, in this order:** this document; the design's §2, §4, §6, §9–§11 and Appendix A; the S4-pre close's §5 and
   §7; the board's Track 4b and Track 2; `docs/security-ledger.md` rows C4, D17, D20; the files §6 rewrites or
   replaces — `NOTICE`, `ASSETS.md`, `LICENSE`, `README.md` (its browser-client and status sections), `CONTRIBUTING`
   and `SECURITY` (absent today), `docs/web-lane.md` §14–§15, `deploy/README.md` and `worker/README.md` (the host
   guide's pointers), `docs/plans/2026-08-19-docs-consolidation.md` (the relocation-map pattern, §6.12), and for
   §6.11 the computer player's sources (`docs/plans/2026-07-25-phase3-map.md`, `docs/plans/2026-08-30-phase3.5c-v4-identity-freeze.md`,
   `docs/plans/2026-08-30-phase3.5c-v4-close.md`, the board's Track 1).
2. **Confirm PR #61 is merged** (`gh pr view 61`). If it is not, ask the owner; the merge is on the owner's word only.
3. **Merge the Language School branch** (§6.1, §10.7): a docs-only PR `docs-language-school-hotfix-handover` →
   master, `merge-tree` re-run first, the board and the log resolved by keeping one board and appending the rows,
   merged on the owner's word.
4. **Cut `security-s4a` from master.** Its first commit is the S4a plan (and this document's pointer on the board if
   the board moves).
5. **The short brainstorm** (superpowers:brainstorming), scoped to §4 below — the design is the design.
6. **The plan** (superpowers:writing-plans) from the design's §6 against the merged tree: each task with its tier in
   the header (impl: sonnet | opus; review: opus; Fable's whole-branch review on an export of the head), the
   pre-flight the orchestrator's without mutants, every agent's report in a file, exports with their own `target/`
   and the two `worker/test/*.gen.ts` copied in. **A plan must be readable whole by one agent — flag a split at
   scoping, not after drafting** (design §9; the plan-size rule). The run is SDD; implementers in the checkout one
   at a time, reviewers on exports (S3's and S4-pre's shape).
7. **Outward-facing steps wait for the owner's word in the session:** creating the lab repository, pushing to it,
   deleting branches on origin, setting the repository's metadata, merging. The plan marks each.

## 4. What the short brainstorm settles (everything else is §6 and §10)

- **The plan's shape.** §6 holds thirteen items of three kinds — code-adjacent (§6.2's blast radius: `mkcardart.py`,
  the manifest's provenance string with the wire corpus re-read BEFORE the change is called inert, `tables.rs`'s doc
  comments, the Dockerignore line; §6.5's D20 fixture redaction with `feed_preprocess.rs` and the CLI's `bga/*` re-run;
  §6.10's `deploy/run-local.sh`), operations (§6.7 the lab, the bundle, the deletions; §6.8 the art worktree; §6.9
  metadata; §6.13 the author address) and documents (§6.3 the notice set; §6.4 the community files; §6.10 the web
  guide; §6.11 `docs/computer-player.md`; §6.12 the relocation map; §6.6 the sweep of releases and PR bodies). Decide
  the order and whether S4a is one plan or two sequential ones (the readable-whole rule); the suites each kind owes
  (§6.2 and §6.5 touch what the layer compiles — the web lane's §2 set runs; D20's readers run under
  `cargo test -p yokohama-bridge` and `cargo test -p yokohama-cli` with the corpus).
- **The tasks that need the owner at the keyboard:** the NOTICE paragraph (§6.3, written live, never drafted alone);
  the lab's creation and the branch deletions (§6.7); the metadata (§6.9); the author-address switch on both
  workstations (§6.13). Schedule them where the owner is present; the plan says "owner present" on each.
- **The sweep's rule** (§2.7, §6.3, §6.6): the sweeping agent's instruction for what a line must do to be edited
  ("argues a position") and what it must not touch (dated documents, §10.12), written once in the plan; the hit list
  starts from Appendix A.
- **§6.2's blast radius, verified at planning:** grep the tree for `tts-assets/` and `00-yokohama-rulebook.pdf` at the
  merged head and compare with Appendix A's list; the oracle independence rule's "rules sources" clause is re-worded,
  not changed (`CLAUDE.md`'s Commands section).
- **The web guide's spine** (§6.10, §9): its sections in order; the one screenshot (§10.5); the host section's one
  Docker command built in the same cycle (§10.18) and proven by the walk (design §7.7 — the walk itself is S4b's);
  the "what is by design" list (hidden information, the bot's seat, the legal-list gate, the Host rule, the open door's
  seat property for self-hosters — design §11 (f)).
- **D17 before the first public tag** (§6.5): `release.yml`'s permissions, action pins and tag rules — one task, its
  reader a dry read of the workflow.
- **The second workstation's recipe** (§6.7): the two-remote configuration and the author address, written in the
  close and named on the board.
- **Questions only the owner answers at the read:** the lab repository's name (§10.8 proposes `yokohama-assistant-lab`);
  the description and the topics (§10.11 proposes them); the social preview capture (left for the owner to upload at
  the switch, §6.9); whether `security-auth` and `security-s3`, merged now, leave origin with the fifty (§2.8 said
  "the three unmerged stay" when they were unmerged).

## 5. The rules that bind S4a (one screen; the design's §4 is the full list)

Deletions by the orchestrator only. Dated documents are not edited for moved paths (§10.12; the two exceptions: D20's
fixture, and a sweep hit that argues a position). Archives verbatim; figures as recorded. The bar (map §1.9) is NOT
claimed anywhere a document is rewritten. No model or co-author table. The public register: committed documents carry
no rights-position discussion and few owner quotes; the NOTICE paragraph is written with the owner present. Commits
authored under the GitHub no-reply address from S4a on, no rewrite (§2.20); the owner's test identity stays out of
committed documents. The STOPs in force: `SEMANTICS_REV`, the oracle, the freezes, the pre-registrations — nothing in
S4a touches them; the fenced crates move only where §6.5 names a fixture, and its readers re-run. The lab runs no
Actions; `test.yml` returns on origin at the flip (§2.19). Every outward-facing step on the owner's word.

## 6. The session's memory, for the orchestrator

`feedback-tiered-model-dispatch` (Sonnet for precise briefs, Opus for judgment and every review seat, Fable for the
whole-branch review), `feedback-no-parallel-plan-drafting` (one agent writes the plan), `feedback-plan-size-split-cycles`,
`feedback-subagent-report-truncation`, `feedback-no-bulk-rm-in-dispatches`, `feedback-subagent-waits-on-events`,
`feedback-sdd-crossed-messages`, `feedback-preflight-no-mutant-rerun`, `feedback-public-register-no-legal-talk`,
`feedback-lean-dependencies-first-principles`, `feedback-art-lane-additive-no-cycle-start-suites` (the art lane's
rule toward origin), and the project memories `yokohama-security-s4-public-release`, `yokohama-art-replacement-lane`,
`yokohama-language-school-hotfix-branch`.
