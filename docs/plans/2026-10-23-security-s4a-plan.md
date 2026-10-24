# Security S4a — the tree: implementation plan (written 2026-10-23)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the repository its public shape before the front door is written: the Tabletop Simulator sheets and the English rulebook leave the tip, the notice set states facts, the community files and the two guides exist, the ledger's three S4 items close with catchers, the lab and the full backup exist and the merged branches leave origin — so that every destination the S4b README will point at exists at this cycle's close.

**Architecture:** A documents-and-operations cycle with three code-adjacent edges and one new script. The removal of `tts-assets/` and the PDF is the orchestrator's own commit; an implementer re-words every live reference to the paths, with the KB's three YAML comments among them (inert for `gen-data`, proven by a byte-identical regeneration). The D20 fixture is redacted in place and gains a cell in the bridge's always-running excerpt tier; `release.yml` gains per-job permissions, SHA pins and a first job that refuses a tag off master's history, with a cell under the Worker's `test:scripts`; the notice gains a cell under the web's `test:wasm` that holds its picture rows to the tree. `deploy/run-local.sh` wraps the self-host guide's §3 build and §4 run under the open door on loopback. Nothing in the engine, the bot, the oracle, `SEMANTICS_REV` or the wire corpus moves; the layer's only edits are a doc comment and a manifest provenance string.

**Tech Stack:** git, `gh`, bash; Markdown; one Rust test cell (`crates/yokohama-bridge/tests/feed_preprocess.rs`); two vitest node cells (`worker/scripts/*.test.mjs` under `npm run test:scripts`; `web/scripts/*.node.test.ts` under `npm run test:wasm`); Docker (29.9.2 on this Mac); Pillow 13 for the preview image; `ruby -ryaml` as the workflow parser (no `actionlint`, `yamllint` or PyYAML on this machine).

**Spec:** the design `docs/plans/2026-10-22-security-s4-public-release-design.md` — §6 is this cycle's content, §2 the owner's settled inputs, §4 the rules that bind every cycle, §9–§11 what is left to the builder, the calls and the builder check, Appendix A the recon at `56460688` — read with the handover `docs/plans/2026-10-23-security-s4a-handover.md` (§2 what S4-pre routed here, §4 the brainstorm's agenda, §5 the binding rules) and the "Settled at the brainstorm" section below. Every rule in this plan argues from one of them; line numbers are readings at master `d5dd0007` (the merge of PR #61) and are re-read at the branch's head.

## Settled at the brainstorm (2026-10-23, the owner, in session)

The owner's answers, in substance verbatim:

1. **PR #61 is merged** (`d5dd0007`, 2026-10-23 17:15Z). `security-s4a` is cut from it.
2. **The Language School docs branch is NOT merged in this cycle.** The owner: "the language school merge would cause a semantic_rev change, and so that needs to wait and get put on the issues board." The hotfix is **issue #62** (posted 2026-10-23 in the contributor layout, label `bug`); the branch `docs-language-school-hotfix-handover` stays on origin unmerged and is the one branch the deletions below leave; the design's §6.1 and §10.7 merge step is superseded. (The branch carries only documents, so the merge itself would not have moved the rev; the issue route still fits the design's §2.21 shape.)
3. **The lab repository is `yokohama-lab`**, private, same account (not the design's §10.8 `yokohama-assistant-lab`).
4. **The description and the topics** are the design's §10.11, as proposed.
5. **The deletions:** every branch merged into master after #61 leaves origin — 52 at `d5dd0007` (Appendix A's merged set as it stands today, with `security-auth`, `security-s3` and `security-s4-pre` among them; P-3); the Language School branch stays. **With a FULL backup first** — the owner: "either in the lab or locally and offline otherwise". Both are taken: every branch and tag on the lab, verified ref by ref, and a verified bundle outside the repository that the owner copies offline.
6. **One plan, two owner-present blocks:** Block A at the start (the author address, the lab, the backup, the deletions, the art worktree, the metadata, the preview image), Block B at the end (the NOTICE and ASSETS.md rights text, the outward edits, the issues' style pass, the merge).
7. **Issue #62 keeps `bug`** for now; a `rules` label is created with the templates task and added to it.
8. **The repository's social preview is uploaded NOW**, not at the switch; GitHub shows it at once, so it is checked in GitHub at the upload. What waits for the switch is the SITE's link previews — the Open Graph, Twitter-card and JSON-LD metadata `web/scripts/presence.mjs` writes for `yokohama.moore.nyc`, which an external fetcher (a messaging app's or a social site's link preview) must read from the root domain, and Cloudflare Access refuses those fetchers today. The owner, 2026-10-23: "we can upload it now … nothing stopping us from laying it all down", and, correcting the first reading (P-1): "we'll want to validate in s4c all the social previews once the access controls are removed — github we can see prior." The upload is Block A's; the site's previews are S4c's reading.

The measured inputs (this session, at `d5dd0007`; the plans do not re-run Appendix A):

- **§6.2's blast radius** matches Appendix A, plus: the three KB YAMLs (`analysis-1/yokohama-kb/07-order-cards.yaml:4`, `08-tech-cards.yaml:4`, `09-building-sites.yaml:4`) each carry a `# Source sprite sheet: tts-assets/organized/...` comment, which `gen-data` does not emit (`crates/yokohama-core/src/game/data.rs` carries no `tts-assets`); the KB's `00-README.md` names neither path; the tracked ledger `.superpowers/sdd/progress.md` names both and is history (append-only), left alone; `analysis-1/Yokohama Assistant Rule Alignment Review.md:253` cites the rulebook by a URL, not the repository path, and stays. The manifest's `"sheets"` string (`crates/yokohama-web/assets/art/manifest.json:441`) has no reader: `art::ArtManifest` (`crates/yokohama-web/src/art.rs`) has no `provenance` field and `web/src/wire/` carries no `sheets` token; the raw file is served under `/art/manifest.json` and precached, so an edit is a new release's bytes and nothing else. The dated documents (23 files) and the three archives are untouched.
- **D20's shape:** of the fixture's 18 `player_name` sites, 16 already read `Player_A`/`Player_B`; two real handles survive, one each on lines 13 and 14, and line 13 is also the one chat frame (its `text`). The handles appear in no other tracked file. Readers: `crates/yokohama-bridge/tests/feed_preprocess.rs` (the excerpt tier, always runs) and eleven corpus-gated sites in `crates/yokohama-cli/src/bga/*.rs` plus `crates/yokohama-cli/tests/bga_import.rs`; both gitignored captures are present on this Mac.
- **D17's state:** `release.yml` carries `permissions: contents: write` at the top level; four actions pinned by tag (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `actions/upload-artifact@v4`, `actions/download-artifact@v4`); the trigger `on.push.tags: v*`; its corpus comment still says "ten pins" and one capture (CLAUDE.md: fourteen, two). The other three workflows carry seven more tag pins — S4c's, with `test.yml`'s re-enable.
- **The trees:** `docs/brand-guide/` 350 files (114 MB) and `docs/tools/art/` 27 files; none of the art branch's 68 tracked files under those paths exists on disk, so the move into the worktree collides with nothing; `.kiro/` holds two empty directories; `docs/tools/__pycache__/` is gitignored noise.
- **The branches:** origin has 54 heads; 52 are merged into master at `d5dd0007`; the six remote-only branches are `docs-consolidation`, `phase3.5c-families`, `phase3.5c-harness`, `phase3.5c-v3`, `phase3.5c-w2-tram`, `phase3.5c-w5`; 10 tags; the pack is 88 MB.
- **The author identities:** 2,369 commits under the owner's address, 56 under `59934696+jmoore2333@users.noreply.github.com` — the address §6.13 sets. Set in this repository's configuration on this workstation at the branch cut (2026-10-23); this plan's commit is the first under it.
- **The owner's address** is spelled in five tracked files, as Appendix A says: `docs/security-ledger.md:27` (row C1), `worker/RUNBOOK.md:2690`, `web/e2e/harness.node.test.ts:367–369, 427–428`, `worker/scripts/live-replay.test.mjs:505, 913, 915`, `worker/scripts/mint-session.test.mjs:91, 93, 145, 186`.

The builder's calls (the design's §9 leaves them; each recorded here so the owner can check them):

- **C-1** The `tts-assets/` line in `deploy/Dockerfile.dockerignore` STAYS (the design's §6.2 said "the line goes"): C-2 makes the path a contributor's local location, and a local copy of the sheets must not enter an image. Its comment says so.
- **C-2** `.gitignore` gains `tts-assets/` and `analysis-1/00-yokohama-rulebook.pdf`: the paths where a contributor who obtains the sheets and the rulebook places them for `docs/tools/mkcardart.py` and the record's page citations, never committed again.
- **C-3** `docs/tools/mkcardart.py` gains one sentence of code: with the sheets absent it refuses by name instead of a traceback (the docstring's promise, made executable).
- **C-4** D17's "tag rule" is a first job in the workflow that refuses a tag whose commit is not an ancestor of `origin/master` (ancestry, NOT first-parent — P-10: a tag pushed on a release branch before its merge, as the last three releases were tagged, is refused at tag time and publishes on a re-run after the merge; first-parent would refuse it forever), so the three builds and the publish never run for a stray tag; GitHub's tag ruleset (who may create `v*`) is a setting and goes to S4c's step 4.
- **C-5** Three catcher cells, because the ledger closes a row only with a catcher a reviewer mutation-proved: D20's in the bridge's excerpt tier, D17's under the Worker's `test:scripts` (where the deploy tooling's cells live), C4's under the web's `test:wasm` (where the repository-reading node cells live).
- **C-6** The issue templates are GitHub issue FORMS (`.yml`), so the surface and the version are fields a reporter cannot skip; the pull-request template is Markdown.
- **C-7** The web guide is written for the public day and is TRUE today: one sentence says Cloudflare Access also stands in front of the hostname until the switch and that the steps are the same with or without it; S4c's close re-points that sentence.
- **C-8** Task 6 does not edit the README's status section; S4b's rewrite retires it, and the close's relocation map records the move.
- **C-9** Task 9, after Block B, re-points `CLAUDE.md`'s licensing sentence at the notice's final paths; Task 1 re-points the edition paragraph, the rules-knowledge line and the oracle rule's clause, which concern the PDFs alone.
- **C-10** The run-local script's `--stop` keeps the volume and says so; a second `--stop` with nothing running exits 0 with a sentence.

## Global Constraints

- **The branch is `security-s4a`, cut from master `d5dd0007`**; every commit lands on it and the PR targets master. Stage with explicit paths (`git add <path>`), never `git add -A`. No version bump, no deploy, no tag: nothing here ships (the design §2.23 — stage 2's `0.7.3` is S4c's).
- **Commits are authored under the GitHub no-reply address** (`59934696+jmoore2333@users.noreply.github.com`, set in this repository's configuration on 2026-10-23; the second workstation's recipe is in the close); history is not rewritten (the design §2.20). The owner's test identity and the owner's address appear in no committed document.
- **The STOPs are untouched:** `SEMANTICS_REV`, `crates/yokohama-oracle`, the freezes, the pre-registrations; `crates/yokohama-core`'s bytes do not move (Task 1's `gen-data` regeneration is proven byte-identical); the CLI's code does not move (its tests read Task 2's fixture). So no release-tier roster runs in this cycle (the design §6's close line); the suites each task owes are named in its verify step.
- **The suites per edit** (`docs/web-lane.md` §2; `CLAUDE.md`'s Commands): a change under `crates/yokohama-web` runs `cargo test -p yokohama-web` (with `wire_fixtures` WITHOUT `UPDATE_WIRE_FIXTURES=1` — a task that needs to regenerate the corpus has misread this plan), `cargo test -p yokohama-wasm`, `cargo test -p yokohama-worker`, then in `worker/` `npm run build:module:debug` FIRST (the pool tests a stale module green), `npm run typecheck && npm test && npm run test:scripts`, and in `web/` `npm run build:wasm && npm run typecheck && npm test && npm run test:wasm`; a change under `crates/yokohama-bridge` runs `cargo test -p yokohama-bridge` and `cargo test -p yokohama-cli` WITH the corpus present (the fourteen pins must assert, so `YOKOHAMA_BGA_CORPUS_OPTIONAL` is NOT set in this cycle); an edit to `docs/web-lane.md` runs `cargo test -p yokohama-web --test lane_document` and the Worker's `npm run test:scripts` (its `lane-variables` cells); the lint gate `cargo clippy --workspace --all-targets -- -D warnings` after every task that touches a crate. The secrets guard's cells run ALONE, never beside a cargo build. `cargo fmt` is never run. The whole web set with `npm run e2e` runs once at the close.
- **Live documents are re-pointed; dated documents are not edited** for a path that no longer exists (the design §4, §10.12). The archives are verbatim. The two exceptions are D20's fixture (Task 2) and a copyright-sweep hit that argues a position (Task 7, none expected); each is recorded in the Review record and the close.
- **The public register** (the design's header; `feedback-public-register-no-legal-talk`): committed documents state what is third-party and whose it is and stop there — no legal conclusions or hedges, no takedown or removal-on-request sentences, no "why the owner wants it seen"; owner quotes only where one rules. The NOTICE and ASSETS.md rights text is written in Block B with the owner present, never drafted by an agent.
- **Figures** (the design §4): every number a page states is a recorded figure with its record named, quoted as recorded; a one-sided bound is called one-sided; an observed maximum is "max observed under harness X", never a bound; no direction is asserted where none was measured; **the bar (map §1.9) is NOT claimed** anywhere a document is rewritten; no model or co-author table.
- **Deletions are the orchestrator's or the owner's** (the design §4): `git rm -r tts-assets`, the branch deletions on origin, the removal of the untracked trees and the empty directories run in the orchestrator's own shell, never in a dispatch.
- **Dispatch rules (the owner's standing rules):** implementers work IN the checkout, one at a time, and the orchestrator sends them nothing and runs nothing in the tree while one works (bar the announced heartbeat check); every reviewer runs on an EXPORT (`git archive <sha> | tar -x -C <dir>`) with its own `target/`, `web/node_modules`, `worker/node_modules`, `worker/gen`, `worker/assets` and `worker/.test-assets` symlinked from the checkout, the two `worker/test/*.gen.ts` COPIED (never linked), and `bridge-captures/` symlinked read-only where a task's suites read the corpus; every agent writes its full report to a transcript file and replies with the path and three lines; no agent runs `rm -rf`, `rm -r` or a bulk `rm -f` (a single file by its explicit path only; exports are left in place and named); no agent waits on an event, arms a Monitor, spawns a subagent or messages a peer; an Opus implementer probes the brief's mechanisms before reporting and names concerns (DONE_WITH_CONCERNS); a fix report quotes its hunks and the implementer runs `git diff` before committing; once a SHA is reported every further change is a NEW commit; a commit's trailer carries the IMPLEMENTER's own model.
- **Tiers:** as each task's header says (`impl: sonnet` for the precise briefs — Tasks 1, 2, 3, 7, 8, 9; `impl: opus` for the judgment-laden ones — Tasks 4, 5, 6); every task reviewer `review: opus`; the whole-branch review Fable's, on an export of the head; the pre-flight in Block A the orchestrator's, without mutants (mutants run three times by design — the implementer's probe, the task review, the whole-branch review — and the overlap is accepted).
- **Outward-facing steps run on the owner's word in the session**, each marked **(outward)** below: creating the lab, pushing to it, deleting branches on origin, the repository's metadata, the preview upload, the label, editing issues, release notes and PR bodies, the PR and the merge. The owner's answers above are the word for the lab, the deletions, the metadata, the label and the preview; the rest is asked for at its step.
- **Commit messages** in the house style (a prose subject naming the change, its rule and its catcher), ending with the attribution trailer the committing session carries.

## Review Focus

Five readers or operators this cycle's documents will meet, each pinned by a check in the task that owns it:

1. **A fresh clone without the sheets** runs `python3 docs/tools/mkcardart.py --check`: it must refuse by name (the sheets are outside the tree, and where they go), never with a traceback (Task 1, Step 6).
2. **A fresh clone without the two captures** runs `cargo test -p yokohama-bridge`: the excerpt tier, D20's cell among it, is green with no corpus and no opt-out (Task 2, Step 6).
3. **A self-hoster on Apple silicon** runs `bash deploy/run-local.sh` once: the script says before the build that the image is `linux/amd64` and a first build runs under emulation for minutes, and it reads the server's health before it says "open http://127.0.0.1:8080/" (Task 5, Steps 2 and 8).
4. **A reader who quotes a figure from `docs/computer-player.md`** gets its interval and its record in the same sentence, and a one-sided bound is called one-sided (Task 6, Step 3's review check).
5. **A stray `v*` tag** pushed on a commit that is not on master's history publishes no release: the workflow's first job refuses before any build (Task 3, Step 5; the cell pins the job's presence, the reviewer dry-reads its logic).

---

## Block A — before Task 1 (the orchestrator; the owner present for the marked steps)

Done at the branch cut, 2026-10-23: PR #61 merged (`d5dd0007`); local master fast-forwarded; `security-s4a` cut from it; `git config user.email 59934696+jmoore2333@users.noreply.github.com` in this repository (the design §6.13); the Language School ruling applied (issue #62; the branch stays). Still the owner's, in the browser, any time before the first push to the lab: GitHub's "Keep my email addresses private" and "Block command line pushes that expose my email".

- [ ] **A1. The first commit:** this plan. `git add docs/plans/2026-10-23-security-s4a-plan.md && git commit` (the subject names the plan and the brainstorm's settlements); then **(outward)** `git push -u origin security-s4a` — the public repository receives a branch when its plan exists (the design §2.10).
- [ ] **A2. The SDD ledger** at `.superpowers/sdd/2026-10-23-security-s4a-plan/progress.md` (gitignored): one row per task with its tier, its export path, its transcript path and its verdict; the owner's answers above; `df -h /System/Volumes/Data` before the first export (124 GB free of 926 on 2026-10-23).
- [ ] **A3. The before review (the pre-flight, no mutants):** an export of the head with its own `target/`; the suites Tasks 1 and 2 owe run once for their baseline counts (the web lane's §2 rows less e2e; `cargo test -p yokohama-bridge`; `cargo test -p yokohama-cli` with `bridge-captures/` symlinked; clippy) into the ledger; Tasks 1–3's briefs read against the files they cite at the head, the line numbers re-read, every correction applied to this plan AT THE SOURCE and numbered P-n in the Review record.
- [ ] **A4. The lab (outward; the owner's word given 2026-10-23).** In this order, each step read back before the next:
  1. `gh repo create jmoore2333/yokohama-lab --private --disable-wiki --disable-issues --description "Private lab for yokohama-assistant: work in progress and the full backup of its branches and tags. The project's home is jmoore2333/yokohama-assistant."` — no `--source`, no `--push`.
  2. Actions OFF before anything is pushed (`release.yml` runs on `v*` tags; ten tags would start thirty jobs and create ten releases there — the design §6.7, §10.19): `gh api -X PUT repos/jmoore2333/yokohama-lab/actions/permissions -F enabled=false`, then `gh api repos/jmoore2333/yokohama-lab/actions/permissions --jq .enabled` reads `false`.
  3. `git remote add lab https://github.com/jmoore2333/yokohama-lab.git && git config remote.pushDefault lab`; master keeps `origin/master` as its upstream (`git rev-parse --abbrev-ref master@{upstream}` reads `origin/master`). From here a bare `git push` goes to the lab; a push to origin names it.
  4. The six remote-only branches get local branches: `for b in docs-consolidation phase3.5c-families phase3.5c-harness phase3.5c-v3 phase3.5c-w2-tram phase3.5c-w5; do git branch --no-track "$b" "origin/$b"; done`.
  5. `git push lab --all && git push lab --tags`, then the ref-by-ref check: `git for-each-ref --format='%(objectname) %(refname)' refs/heads refs/tags` against `git ls-remote lab` (the same name, the same SHA, for every local branch and tag — `git for-each-ref refs/heads | wc -l` reads 60 after step 4 and `refs/tags` 10; any mismatch is a STOP). The art lane's branch is among them (the design §2.18: a backup, not the push the lane's rule forbids).
  6. The bundle, outside the repository: `git bundle create ../yokohama-assistant-2026-10-23.bundle --all && git bundle verify ../yokohama-assistant-2026-10-23.bundle`; its path and size (about the pack's 88 MB) in the ledger; the owner copies it offline (the NAS or a drive — the owner's).
  7. **The deletions, only after steps 5 and 6 both read complete.** The list is computed, written to the ledger and the close BEFORE the first deletion: `git branch -r --merged origin/master --list 'origin/*' | sed 's|^ *origin/||' | grep -v -- '->' | grep -v -x -E 'master|docs-language-school-hotfix-handover'` — 52 names at `d5dd0007` (P-4: the `HEAD -> origin/master` line, which the exact-match filter never caught, is dropped by the `->` filter — without it the list reads 53 — and `--list 'origin/*'` keeps the lab's remote-tracking refs out, which step 5 created: without it the list read 109 and the count gate refused the run). Then, one by one, `git push origin --delete <name>`; then `git fetch --prune origin`, and `git ls-remote --heads origin` reads exactly `master`, `docs-language-school-hotfix-handover` and `security-s4a`. Local branches are kept (the owner's word).
- [ ] **A5. The art lane moved aside:** `git worktree add ../yokohama-assistant-art art-replacement-spec`; the counts before (`find docs/brand-guide -type f | wc -l` → 350; `find docs/tools/art -type f | wc -l` → 27); `rsync -a docs/brand-guide/ ../yokohama-assistant-art/docs/brand-guide/ && rsync -a docs/tools/art/ ../yokohama-assistant-art/docs/tools/art/`; `diff -r` of each source tree against its destination reads no difference; the counts after in the worktree (68 tracked + 377 moved); then, in the orchestrator's own shell, the two source trees, `.kiro/` and `docs/tools/__pycache__/` removed; `git status --short` reads clean. The lane's rule toward origin is unchanged: no push there, no PR, no merge.
- [ ] **A6. The metadata (outward; the owner's word given):** `gh repo edit jmoore2333/yokohama-assistant --description "Rules engine, CLI tracker and browser client for the board game Yokohama, with a computer player built to play to win." --homepage https://yokohama.moore.nyc --add-topic rust --add-topic board-game --add-topic game-engine --add-topic game-ai --add-topic yokohama --add-topic webassembly --add-topic cloudflare-workers --add-topic react --add-topic cli`; read back with `gh repo view --json description,homepageUrl,repositoryTopics`.
- [ ] **A7. The social preview (outward; the owner uploads):** the DOOR, not a game screen (P-8, the owner's correction of 2026-10-24 after reviewing the first file): the built client (`web/dist`, 0.7.2) served by the checkout's `yokohama-web` binary under the open door on loopback with the art directory as master `d5dd0007` shipped it (so the release's precache digest matches), captured by Playwright in a fresh profile at a 1280×640 viewport once the third card reads "Ready to play offline", PNG — into the session scratchpad, not the tree; the owner uploads it in the browser (Settings → General → Social preview) now and reads it back there (GitHub renders it at once; P-1). The site's own link previews are S4c's reading, once Access is off. The recipe and the file's digest go in the close.

---

### Task 1: `tts-assets/` and the English rulebook leave the tip; the references re-pointed · impl: sonnet · review: opus

The design §6.2 and §10.1, with the measured blast radius above. **Two commits: the orchestrator's removal first, then the implementer's re-pointing.**

**The orchestrator's commit (Task 1a), in its own shell, before the dispatch:**

```bash
mkdir -p docs/tools/tts-extraction
git mv tts-assets/parse_tts_mod.py tts-assets/EXTRACTION_SUMMARY.md tts-assets/asset_mapping.json docs/tools/tts-extraction/
git rm -r -q tts-assets
git rm -q analysis-1/00-yokohama-rulebook.pdf
git commit -m "tree: the Tabletop Simulator sheets and the English rulebook leave the public tip — 115 sheets, scans and the mod's save file deleted, the English PDF deleted, the three repository-authored files moved to docs/tools/tts-extraction/ (S4a Task 1a; the design §6.2, §10.1)"
```

Expected: `git show --stat HEAD | tail -1` reads 119 files changed (3 renames, 116 deletions); `git ls-files tts-assets` prints nothing; `git ls-files docs/tools/tts-extraction` prints the three.

**Files (the implementer's commit, Task 1b):**
- Create: `docs/tools/tts-extraction/README.md`
- Modify: `docs/tools/mkcardart.py` (the docstring at 7–9; `SHEETS` at 85; the comments at 180, 243 and 248; the provenance string at 350), `docs/tools/mkhelpart.py:8–10`
- Modify: `crates/yokohama-web/assets/art/manifest.json:441`
- Modify: `crates/yokohama-web/src/tables.rs:614–626` (the doc comment)
- Modify: `deploy/Dockerfile.dockerignore:29–36` (the comment at 29–34; the `tts-assets/` line at 35 stays — C-1; P-6 re-read the start line)
- Modify: `.gitignore` (a new section after the bridge-captures section at 44–49 — C-2)
- Modify: `analysis-1/yokohama-kb/07-order-cards.yaml:4`, `08-tech-cards.yaml:4`, `09-building-sites.yaml:4`
- Modify: `analysis-1/yokohama-kb/01-rules-reference.md:542, 549` — the two D1 notes locate the English PDF "bundled in `analysis-1/`"; they name the 2017 English rulebook instead, the line count unchanged (code cites this file by line number) — P-7, the review's finding
- Modify: `CLAUDE.md` — the Rules-knowledge "Complete reference" line, the edition paragraph (the two path spellings), and the ORACLE INDEPENDENCE RULE's "the rulebook PDF" clause; NOT the Worktree-hygiene licensing sentence (Task 9, C-9)
- Modify: `README.md:763–766` (one paragraph), `docs/web-lane.md:82` and `:422`
- Not touched: `ASSETS.md` and `NOTICE` (Block B); `docs/handover.md` (Task 9 rewrites Track 2); `.superpowers/sdd/progress.md` (a ledger); every file under `docs/plans/`; the three archives; `analysis-1/Yokohama Assistant Rule Alignment Review.md`

**Interfaces:**
- Produces: the sentence every live site uses for the sheets — *the Tabletop Simulator upload's sheets (Steam Workshop 1840951801), which are not in this tree; a contributor who has them places the organized sheets at `tts-assets/organized/`, with the raw `images/` cache beside them at `tts-assets/images/`, both gitignored* (P-7: the tool reads eleven files from `images/` too) — and for the rulebooks — *the 2017 Tasty Minstrel English rulebook (16 pages), whose page numbers are the ones the record cites, and the 20-page edition in the Tabletop Simulator upload, whose setup steps are on its p.5; neither is in the tree, and a contributor obtains their own copy*. The relocation map (the close) cites these words.

- [ ] **Step 1: Write `docs/tools/tts-extraction/README.md`** — one paragraph: what the sheets were (the sprite sheets, scans and two PDFs of the Tabletop Simulator upload, extracted on 2025-12-07 by `parse_tts_mod.py`; `EXTRACTION_SUMMARY.md` is that extraction's record and `asset_mapping.json` its map, both kept as written, so the paths inside them are the paths they had), that the tree does not carry them since 2026-10-23, and where a contributor who has them puts them for `docs/tools/mkcardart.py` (`tts-assets/organized/`, gitignored). No statement about rights — the notice is `ASSETS.md`'s.

- [ ] **Step 2: Re-word `docs/tools/mkcardart.py`.** The docstring's sentence at 7–9 says the sheets are the Tabletop Simulator upload's, are not in the tree, and are read from `tts-assets/organized/` (with the raw `images/` cache beside it) when present (`docs/tools/tts-extraction/README.md` says what they were); add, right after `SHEETS` is defined at 85, the guard (C-3):

```python
if not SHEETS.is_dir():
    sys.exit(
        f"the sprite sheets are not in this tree: place the Tabletop Simulator upload's "
        f"organized sheets at {SHEETS.relative_to(ROOT)} (gitignored) and run again "
        "(docs/tools/tts-extraction/README.md)"
    )
```

The comment at 180 names "the mod's save file (not in the tree; its tile naming is recorded in `docs/tools/tts-extraction/`)" in place of the path; 243 keeps its convention sentence ("writing into `tts-assets/` is what this file must not do" — the tree is a contributor's local copy now); 248's comment reads "relative to SHEETS, the raw `images/` beside `organized/`"; the provenance string at 350 becomes `"sheets": "the Tabletop Simulator upload's organized sheets, outside this tree (docs/tools/tts-extraction/README.md)"`. `docs/tools/mkhelpart.py:8–10`: "It never writes into `tts-assets/`" → "It never reads or writes the sprite sheets, which are not in the tree".

- [ ] **Step 3: The manifest, the layer's doc comment, the Dockerignore, the ignore file, the YAMLs.** `manifest.json:441` takes the SAME string as Step 2's (so a `--check` with the sheets present compares equal). `tables.rs:614–626` names the editions, not paths: "the 20-page edition in the Tabletop Simulator upload, whose setup steps are on its p.5" and "the 2017 Tasty Minstrel English rulebook (16 pages), which orders those three differently, is the outlier of the two editions the record cites; neither PDF is in the tree" (P-7: ", is" — the first wording left the parenthetical without a main verb; the shipped text is the record). `Dockerfile.dockerignore:29–36`: the comment says the sheets and the English rulebook left the public tip in S4a (2026-10-23), so `tts-assets/` and `analysis-1/00-yokohama-rulebook.pdf` exist only as a contributor's gitignored local copies, which must not enter an image (P-7 named the PDF's copy too); the `tts-assets/` line stays; the `analysis-1/` line stays. `.gitignore`: after the bridge-captures section, a section in its style — the two paths, the sentence that they left the public tip on 2026-10-23 (S4a), that a contributor who obtains them places them here for `docs/tools/mkcardart.py` and the record's page citations, and that they are never committed again. The three YAMLs' line 4: `# Source sprite sheet: order_cards_8x5.jpg in the Tabletop Simulator upload's organized sheets (not in this tree — docs/tools/tts-extraction/; 8 cols x 5 rows: 36 cards + 4 backs)` and its two siblings with their own names and grids.

- [ ] **Step 4: `CLAUDE.md`, `README.md`, `docs/web-lane.md`.** CLAUDE.md's "Complete reference" line: "rulebook PDF in `analysis-1/`; icon glossary rulebook p.10" → "the 2017 English rulebook, a contributor's own copy (not in the tree; its page numbers are the record's — icon glossary p.10)". The edition paragraph: the TTS path spelling → "the owner's edition, the 20-page edition in the Tabletop Simulator upload (setup steps on its **p.5**), not in the tree"; "`analysis-1/00-yokohama-rulebook.pdf` (16 pages, the file this document and the KB cite as 'the rulebook PDF')" → "the 2017 Tasty Minstrel English rulebook (16 pages, the edition this document and the KB cite as 'the rulebook PDF'; not in the tree since S4a)". The ORACLE INDEPENDENCE RULE: "the rulebook PDF" → "the English rulebook (the edition whose page numbers the record cites; the author's own copy)" — re-worded, not changed. `README.md:763–766`: the paragraph reads "The code in this repository is Apache-2.0 (`LICENSE`, `NOTICE`); [`ASSETS.md`](ASSETS.md) names the third-party paths that carry no grant and says what a fork has to do about them." `docs/web-lane.md:82`: "crops `tts-assets/organized/` into" → "crops the Tabletop Simulator upload's sheets, outside the tree (`docs/tools/tts-extraction/`), into"; `:422`: "while `tts-assets/` leaves the public tip" → "while `tts-assets/` leaves the public tip, which it did in S4a (2026-10-23)", re-wrapped to the neighbours' width (P-7: the first wording made the 2026-10-22 ruling report a 2026-10-23 fact).

- [ ] **Step 5: Verify the greps**

Run: `git grep -n 'tts-assets' -- . ':!docs/plans' ':!guidingpath-archive*.md' ':!.superpowers' ':!ASSETS.md' ':!docs/handover.md'`
Expected: only lines that say the path is a contributor's local location or that the sheets left — `.gitignore`'s two rule lines and their comment, `mkcardart.py`'s `SHEETS` line, its docstring sentence and its convention comment (243) — the guard spells the path through `SHEETS.relative_to(ROOT)`, so the grep does not list it (P-6) —, `docs/tools/tts-extraction/README.md`'s one sentence, `Dockerfile.dockerignore`'s line and its comment, `docs/web-lane.md:82` and `:422`, and `CLAUDE.md`'s Worktree-hygiene licensing sentence, which Task 9 re-points (C-9) — nothing else (`tables.rs`, `mkhelpart.py`, the YAML comments and the edition paragraph name no path after this task). The implementer lists every surviving line in the transcript with the word that makes it true.

Run: `git grep -n '00-yokohama-rulebook.pdf' -- . ':!docs/plans' ':!guidingpath-archive*.md' ':!.superpowers' ':!ASSETS.md' ':!analysis-1/Yokohama*'`
Expected: `.gitignore`'s rule, `CLAUDE.md:20` (the licensing sentence Task 9 re-points — C-9; P-7: the first expectation missed it) and `deploy/Dockerfile.dockerignore:32`, whose comment names the PDF's gitignored local copy since the fix round (M6).

Run (P-7, the third sweep — the sentences that locate the PDF without spelling its path): `git grep -n -i -E 'bundled PDF|English PDF|PDF in .?analysis-1|bundled in .?analysis-1|bundled .?analysis-1' -- . ':!docs/plans' ':!guidingpath-archive*.md' ':!.superpowers' ':!ASSETS.md' ':!NOTICE'`
Expected: `.gitignore`'s comment on the English PDF's page citations, and the eight code-comment sites in the two fenced crates — `crates/yokohama-core/src/game/scoring.rs:381`, `setup.rs:177`, `setup.rs:805`, `types/ids.rs:53`, `crates/yokohama-oracle/src/scoring.rs:25`, `:106`, `:179`, `:518` — which this cycle does NOT edit (core's bytes do not move; the oracle is a STOP) and the close records forward; nothing else.

- [ ] **Step 6: Verify the regeneration and the tooling**

Run: `cargo run -p gen-data && git diff --exit-code crates/yokohama-core/src/game/data.rs && python3 docs/tools/mkcardart.py --check; echo "exit $?"`
Expected: `data.rs` byte-identical (exit 0 on the diff); the script exits non-zero with ONE named sentence — PyYAML's if PyYAML is absent on the machine, else the sheets-absent sentence of Step 2 — and no traceback. Then (P-6; PyYAML IS absent on this Mac, so the run above stops at its sentence): `mkdir -p "$SCRATCH/yaml-stub" && : > "$SCRATCH/yaml-stub/yaml.py" && PYTHONPATH="$SCRATCH/yaml-stub" python3 docs/tools/mkcardart.py --check; echo "exit $?"` — the empty stub satisfies the import and the run reaches the guard, which must exit non-zero with the sheets-absent sentence of Step 2 and no traceback (Review Focus 1, made executable on a machine without PyYAML; the stub lives in the session scratchpad, never the tree).

- [ ] **Step 7: Run the suites**

Run: `cargo test -p yokohama-web && cargo test -p yokohama-wasm && cargo test -p yokohama-worker && cargo clippy --workspace --all-targets -- -D warnings && (cd worker && npm run build:module:debug && npm run typecheck && npm test && npm run test:scripts) && (cd web && npm run build:wasm && npm run typecheck && npm test && npm run test:wasm) && cargo test -p yokohama-web --test lane_document`
Expected: every suite green at its baseline count (the ledger's A3 reading); `wire_fixtures` 18 passed WITHOUT the flag.

- [ ] **Step 8: Commit**

```bash
git add docs/tools/tts-extraction/README.md docs/tools/mkcardart.py docs/tools/mkhelpart.py crates/yokohama-web/assets/art/manifest.json crates/yokohama-web/src/tables.rs deploy/Dockerfile.dockerignore .gitignore analysis-1/yokohama-kb/07-order-cards.yaml analysis-1/yokohama-kb/08-tech-cards.yaml analysis-1/yokohama-kb/09-building-sites.yaml analysis-1/yokohama-kb/01-rules-reference.md CLAUDE.md README.md docs/web-lane.md
git commit -m "docs, tooling: every live reference to the sheets and the rulebook names the editions and says the files are outside the tree — the manifest's provenance string, tables.rs's doc comment, mkcardart's docstring and a sheets-absent refusal, the KB's three source comments (gen-data byte-identical), CLAUDE.md's edition paragraph and oracle clause, the ignore rules for a contributor's own copy (S4a Task 1b; the design §6.2)"
```

Checks for the reviewer (on an export): the two greps' survivors each carry "not in the tree" or its equivalent; `git diff d5dd0007 HEAD -- crates/yokohama-core` is empty; `git diff HEAD~2 HEAD -- crates/yokohama-web/assets/art/manifest.json` is one line; the manifest still parses (`python3 -c 'import json; json.load(open("crates/yokohama-web/assets/art/manifest.json"))'`); `tables.rs`'s comment names no path; the dated documents and the archives are untouched (`git diff --stat HEAD~2 HEAD -- docs/plans guidingpath-archive*.md` is empty).

---

### Task 2: D20 — the fixture's two handles and its chat line redacted, with a catcher · impl: sonnet · review: opus

The design §6.5 (D20) and the measured shape above. The excerpt was redacted when it was committed (`feed_preprocess.rs:5–7`: "player ids and display names replaced by stable placeholders"); two sites and one chat text were missed.

**Files:**
- Modify: `crates/yokohama-bridge/tests/data/excerpt-895724349.ndjson` (lines 13 and 14 only)
- Test: `crates/yokohama-bridge/tests/feed_preprocess.rs` (the excerpt tier, after `the_excerpt_parses_with_every_envelope_arm_represented`)
- Modify: `docs/security-ledger.md:50` (row D20)

**Interfaces:**
- Produces: `fn values_after_key(line: &str, key: &str) -> Vec<String>` in `feed_preprocess.rs` (test-private): every value that follows an occurrence of `"` + `key` in `line` (the quote directly before the key, so `body_text` is not an occurrence of `text`) at ANY escaping depth — after the key, skip any run of `\`, `"`, `:` and spaces, then read to the next `"` or `\` — so `"player_name":"X"`, `\"player_name\":\"X\"` and `\\\"player_name\\\":\\\"X\\\"` all yield `X`; the cell `the_committed_excerpt_names_no_player_but_the_two_placeholders_and_its_chat_line_is_a_placeholder`; the chat placeholder sentence `a chat line, redacted`; and (P-5) the cell's id check — every `player_id` value in the excerpt, read the same way and trimmed of a trailing `,`, `}` or `]`, is one of `0`, `11111111`, `22222222`, all three present.

- [ ] **Step 1: Write the failing cell** in `feed_preprocess.rs`, beside the excerpt tier's first cell:

```rust
#[test]
fn the_committed_excerpt_names_no_player_but_the_two_placeholders_and_its_chat_line_is_a_placeholder() {
    let mut sites = 0;
    let mut chat_lines = 0;
    let mut ids = std::collections::BTreeSet::new();
    for line in EXCERPT.lines() {
        for name in values_after_key(line, "player_name") {
            sites += 1;
            assert!(name == "Player_A" || name == "Player_B", "a player_name that is not a placeholder: {name:?}");
        }
        if line.contains(r#"\"type\":\"chat\""#) || line.contains(r#""type":"chat""#) {
            chat_lines += 1;
            let texts = values_after_key(line, "text");
            assert!(!texts.is_empty() && texts.iter().all(|t| t == "a chat line, redacted"), "the chat frame's text is not the placeholder: {texts:?}");
        }
        for id in values_after_key(line, "player_id") {
            let id = id.trim_end_matches([',', '}', ']']);
            assert!(id == "0" || id == "11111111" || id == "22222222", "a player_id that is not a placeholder: {id:?}");
            ids.insert(id.to_string());
        }
    }
    assert_eq!(sites, 18, "the excerpt carries eighteen player_name sites (anti-vacuity: the reader found them all)");
    assert_eq!(chat_lines, 1, "one chat frame");
    assert_eq!(ids.len(), 3, "the three placeholder ids are all present (anti-vacuity: the id reader found them)");
}
```

(The implementer reads line 13 first: if the chat frame's `type` is spelled otherwise, the cell's marker follows the file, and the transcript says what it reads. If the chat `text` key is spelled `"text"` under `args` and once more under `log`, the placeholder goes in each.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p yokohama-bridge --test feed_preprocess the_committed_excerpt_names_no_player`
Expected: FAIL on the first non-placeholder name (line 13's), after `values_after_key` compiles.

- [ ] **Step 3: Redact lines 13 and 14 in place.** On each (P-5, R-2), the `player_name` value becomes `Player_B` and the `player_id` value beside it becomes `22222222`: neither real id is a seated player's — the excerpt's `gamedatas` pairs `11111111` with `Player_A` (the capturing player, whose channel line 14 already spells as `p11111111`) and `22222222` with `Player_B`, and the corpus shows line 13 is BGA's general-chat channel and line 14 a status notification about a third person — so the two frames are folded onto the one other participant the fixture knows, and a doc comment on the cell says so (the real ids are identifying too, and the file's header promised ids redacted); on line 13 every chat `text` becomes `a chat line, redacted`. Every other byte of the file is unchanged: the escaping depth of each site preserved, no reflow, no trailing-whitespace change, LF endings kept (`git diff --stat` reads 2 insertions, 2 deletions).

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p yokohama-bridge --test feed_preprocess`
Expected: every excerpt cell green, the new one included; the corpus tier green (the capture is present).

- [ ] **Step 5: The readers re-run, with the corpus**

Run: `cargo test -p yokohama-bridge && cargo test -p yokohama-cli && cargo clippy --workspace --all-targets -- -D warnings`
Expected: the bridge at its baseline + 1; the CLI at its baseline exactly (`bga_import` 9 passed and ASSERTING — `YOKOHAMA_BGA_CORPUS_OPTIONAL` unset); clippy clean. A CLI cell that read a redacted name is a finding: report it, do not patch it.

- [ ] **Step 6: The fresh-clone read (Review Focus 2)**

Run: `mv bridge-captures bridge-captures.aside && cargo test -p yokohama-bridge --test feed_preprocess the_committed_excerpt; mv bridge-captures.aside bridge-captures`
Expected: the cell passes with no corpus and no opt-out (it reads `EXCERPT` alone). (A rename, not a deletion; restored on the same line.)

- [ ] **Step 7: The ledger row.** `docs/security-ledger.md:50` becomes: `| D20 | the BGA capture excerpt keeps two real handles and a chat line | D | fixtures | S4 | fixed <sha> (S4a Task 2: the two surviving \`player_name\` values, the two real player ids beside them and the chat frame's text replaced by the placeholders the excerpt already used — the two frames' authors were not at the table, folded onto the opponent's placeholder (R-2); its readers re-run — the bridge's excerpt tier and the CLI's \`bga/*\` with the corpus) | \`crates/yokohama-bridge/tests/feed_preprocess.rs\`: \`the_committed_excerpt_names_no_player_but_the_two_placeholders_and_its_chat_line_is_a_placeholder\` | — |` with `<sha>` the fixture commit's (commit the fixture and the cell first, then the row as a second commit, so the row names a SHA that exists).

- [ ] **Step 8: Commit**

```bash
git add crates/yokohama-bridge/tests/data/excerpt-895724349.ndjson crates/yokohama-bridge/tests/feed_preprocess.rs
git commit -m "bridge: the committed excerpt names no player but Player_A and Player_B and its one chat line is a placeholder — two sites and a text the 2026-08 redaction missed, pinned by a cell in the always-running excerpt tier (S4a Task 2; the ledger's D20)"
git add docs/security-ledger.md
git commit -m "docs: D20 fixed in the security ledger, naming its catcher (S4a Task 2)"
```

Mutants for the reviewer (on an export with `bridge-captures/` symlinked): one handle restored on line 14 (red); the chat text restored (red); `values_after_key` made to skip double-escaped sites (the count reads below 18 — red); a third name `Player_C` written into one site (red); a real-shaped eight-digit id written into one `player_id` site (red). The reviewer also confirms, from the export's `git log -p` of the fixture, that exactly two lines moved.

---

### Task 3: D17 — `release.yml`'s permissions, action pins and the tag rule, with a catcher · impl: sonnet · review: opus

The design §6.5 (D17, "just before public", R-S3-3) and C-4, C-5. The reader of the workflow is a dry read: no runner runs here, and the first run on a public repository is S4c's `0.7.3` tag.

**Files:**
- Modify: `.github/workflows/release.yml`
- Create: `worker/scripts/release-workflow.test.mjs` (under `npm run test:scripts`, which includes `scripts/**/*.test.mjs`)
- Modify: `docs/security-ledger.md:47` (row D17)

**Interfaces:**
- Produces: the workflow's shape — `permissions: contents: read` at the top level; a first job `tag` (ubuntu-24.04; `actions/checkout` with `fetch-depth: 0`; one step `The tag must sit on master` running `git fetch --no-tags origin master && git merge-base --is-ancestor "$GITHUB_SHA" origin/master`); `needs: [tag]` on `ubuntu`, `macos` and `windows`; `publish` keeps `needs: [ubuntu, macos, windows]` and alone carries `permissions: contents: write`; every `uses:` pinned to a 40-hex commit SHA with the version as a trailing comment (`actions/checkout@<sha> # v4.x.y`, `actions/upload-artifact@<sha> # v4.x.y`, `actions/download-artifact@<sha> # v4.x.y`, `dtolnay/rust-toolchain@<sha> # the stable branch at <date>`); the corpus comment corrected to the NINE pins this workflow's `cargo test -p yokohama-cli` runs (the whole of `crates/yokohama-cli/tests/bga_import.rs`) backed by TWO captures (`bridge-captures/895724349.ndjson` and `896199544.ndjson`) — CLAUDE.md's fourteen counts the bridge's five too, which the release never runs (P-10).
- Produces: the cell file's exports, none; its `describe("release.yml")` cells: `pins every action to a commit SHA with its version beside it`, `grants contents: read at the top level and contents: write to the publish job alone`, `runs on v* tags and refuses a tag off master in a first job every build needs`.

- [ ] **Step 1: Write the failing cells** in `worker/scripts/release-workflow.test.mjs` (node, vitest; the file read with `readFileSync(new URL("../../.github/workflows/release.yml", import.meta.url), "utf8")`; text-level, no YAML library — `lane-variables.test.mjs` is the precedent for a cell that reads a repository file):

```js
// every `uses:` value is `<owner>/<repo>@<40 hex>` followed by a `# <version>` comment
const uses = text.split("\n").filter((l) => /^\s*-?\s*uses:/.test(l));
expect(uses.length).toBeGreaterThanOrEqual(6);
for (const l of uses) expect(l, l).toMatch(/uses:\s+\S+@[0-9a-f]{40}\s+#\s*\S/);
// (P-12, P-13) STRUCTURAL pins. Module level: `code` is the text with comments blanked to equal-length spaces
// (indices hold); `body` blanks the block-scalar bodies (`run: |`) the same way on top of it, so a shell glob
// cannot trip the anchor refusal and an anchor or alias token anywhere else is visible; every lookup is bounded
// under `jobs:` (a decoy `  publish:` line before it moves nothing); the job heads under `jobs:` are EXACTLY the
// five, bare block keys in order, and every job body is block style (its first line a bare four-space key), so a
// job of any other spelling, an inserted job, or a flow-style body hiding a key behind a quoted `#` is red
const code = text.replace(/(^|\s)#.*$/gm, (m, p) => p + " ".repeat(m.length - p.length));
const body = (() => {
  const lines = code.split("\n");
  let indent = -1;
  for (let i = 0; i < lines.length; i++) {
    if (indent >= 0) {
      if (lines[i].trim() === "" || lines[i].search(/\S/) > indent) { lines[i] = " ".repeat(lines[i].length); continue; }
      indent = -1;
    }
    const m = /^(\s*)[^\s#][^:]*:\s*[|>][-+]?\s*$/.exec(lines[i]);
    if (m) indent = m[1].length;
  }
  return lines.join("\n");
})();
const jobsStart = code.indexOf("\njobs:\n");
const HEADS = ["tag", "ubuntu", "macos", "windows", "publish"];
const jobsAt = (name) => code.indexOf(`\n  ${name}:\n`, jobsStart);
const span = (name, next) => code.slice(jobsAt(name), next ? jobsAt(next) : code.length);
const expectTheFiveJobHeads = () => {
  expect(jobsStart).toBeGreaterThan(0);
  expect(code.slice(jobsStart).match(/^  \S.*$/gm)).toEqual(HEADS.map((h) => `  ${h}:`));
  for (const h of HEADS) expect(code.slice(jobsAt(h) + 1)).toMatch(/^  [a-z]+:\n    [a-z-]+:/);
};
// --- in the permissions cell, after expectTheFiveJobHeads(): the token `permissions` occurs exactly twice in the
// file, case-blind — the top-level block, pinned as `contents: read` and nothing more, and publish's block,
// pinned as `contents: write` and nothing more — so a grant under any other key spelling, in any job, is red;
// the word write/write-all occurs once, in publish (a belt). Not pinned, recorded: a YAML escape that spells a
// key or a value without its letters
expect(code.match(/permissions/gi)).toHaveLength(2);
expect(code).toMatch(/\npermissions:\n  contents: read\n(?! )/);
expect(span("publish")).toMatch(/\n    permissions:\n      contents: write\n(?!      )/);
expect(code.match(/\bwrite(-all)?\b/gi)).toHaveLength(1);
expect(code.search(/\bwrite(-all)?\b/i)).toBeGreaterThan(jobsAt("publish"));
// nothing conditional, nothing tolerant, nothing by reference — the key `if` in every spelling (quoted, a space
// before the colon, the explicit `? if` form), `continue-on-error`, and any `&` anchor or `*` alias token at a
// YAML node start — after the indentation at a line's start, or after `:`, `?`, `,`, `[`, `{` or `-` and optional
// blanks — or a `<<` merge key, read on `body` so a shell glob or `&&` in a run script does not count (P-14: a
// plain one-line `run: … && …` is neither a block body nor quoted, so a whitespace-bound refusal read red on the
// shipped file; an alias cannot carry properties and always sits at a node start, where this one sees it)
expect(code).not.toMatch(/(^|[\s{,?-])["']?if["']?\s*:/m);
expect(code).not.toMatch(/continue-on-error/i);
expect(body).not.toMatch(/(^[ \t]*|[:?,\[{-][ \t]*)[&*]/m);
expect(body).not.toMatch(/<<\s*:/);
// --- in the tag cell, after expectTheFiveJobHeads(): the trigger; the tag job pinned WHOLE — its name, runner,
// the checkout at a 40-hex pin with `fetch-depth: 0`, and the one step — so no extra key (`defaults`, `env`,
// `container`), no pre-step, no wrap, no extra line and no `exit 0` can neuter it; the builds' `needs: [tag]`;
// publish's `needs`
expect(text).toMatch(/on:\n\s+push:\n\s+tags:\n\s+- "v\*"/);
expect(span("tag", "ubuntu")).toMatch(/^\n  tag:\n    name: Tag is on master\n    runs-on: ubuntu-24\.04\n    steps:\n      - uses: actions\/checkout@[0-9a-f]{40} *\n        with:\n          fetch-depth: 0\n      - name: The tag must sit on master\n        run: \|\n          git fetch --no-tags origin master\n          git merge-base --is-ancestor "\$GITHUB_SHA" origin\/master \|\| \{ echo "[^"\n]*"; exit 1; \}\n\s*$/);
for (const job of ["ubuntu", "macos", "windows"]) expect(span(job).slice(0, 120)).toContain("needs: [tag]");
expect(span("publish")).toContain("needs: [ubuntu, macos, windows]");
```

- [ ] **Step 2: Run to verify they fail**

Run: `cd worker && npm run test:scripts -- release-workflow`
Expected: the three cells fail (tag pins, the top-level `write`, no `tag` job).

- [ ] **Step 3: Resolve the four SHAs** and record them in the transcript with the command that produced each: `gh api repos/actions/checkout/git/ref/tags/v4 --jq .object.sha` (a tag object's SHA is dereferenced with `gh api repos/actions/checkout/git/tags/<sha> --jq .object.sha` when `.object.type` is `tag`), the same for `actions/upload-artifact` and `actions/download-artifact` at their `v4` tags — the version comment names the exact `v4.x.y` the tag resolves to (`gh api repos/actions/checkout/git/matching-refs/tags/v4.` lists them) —, and `gh api repos/dtolnay/rust-toolchain/commits/stable --jq .sha` with the date in the comment. (`rust-toolchain.toml` pins the compiler; the action's pin freezes the installer, as the workflow's own comment at 31–39 explains.)

- [ ] **Step 4: Rewrite `release.yml`** to the Interfaces shape: the top-level `permissions`, the `tag` job first, `needs: [tag]` on the three builds (the line directly under each job's `name:`, so the cell's window finds it), `permissions: contents: write` on `publish`, the four pins, the corpus comment corrected. Nothing else moves: the `--locked` commands, the packaging, the `SHA256SUMS`, `--generate-notes`, the prerelease rule.

- [ ] **Step 5: Run to verify they pass, and that the file parses**

Run: `cd worker && npm run test:scripts && cd .. && ruby -ryaml -e 'y = YAML.load_file(".github/workflows/release.yml"); puts y["jobs"].keys.join(" "); puts y["permissions"].inspect'`
Expected: `test:scripts` green at its baseline + 3 (run alone); ruby prints `tag ubuntu macos windows publish` and `{"contents"=>"read"}`.

- [ ] **Step 6: The ledger row.** `docs/security-ledger.md:47` becomes: `| D17 | \`release.yml\`'s permissions, pins and tag rules | D | CI | S4 | fixed \`<sha>\` (S4a Task 3: \`contents: read\` at the top level and \`contents: write\` on the publish job alone; every action pinned to a commit SHA with its version beside it; a first job refuses a tag whose commit is not an ancestor of master before any build, for every commit from this one on — a \`v*\` tag on an earlier commit runs the old file, which the GitHub tag ruleset, S4c's settings step, covers; the corpus comment corrected to the nine CLI pins — the first public run is S4c's \`0.7.3\`, tagged AFTER its merge) | \`worker/scripts/release-workflow.test.mjs\` (three cells) | — |` (P-10: the SHA in backticks like the other rows; the boundary of the job's reach stated; ancestry, not first-parent).

- [ ] **Step 7: Commit**

```bash
git add .github/workflows/release.yml worker/scripts/release-workflow.test.mjs
git commit -m "ci: release.yml reads by default and writes from the publish job alone, pins every action to a commit SHA with its version beside it, and refuses a tag off master's history in a first job the builds need; the corpus comment says fourteen pins and two captures; three cells under test:scripts hold the shape (S4a Task 3; the ledger's D17)"
git add docs/security-ledger.md
git commit -m "docs: D17 fixed in the security ledger, naming its catcher (S4a Task 3)"
```

Mutants for the reviewer (on an export): one `@v4` restored (red); `contents: write` moved to the top level (red); a second job given `contents: write` (red); the `tag` job's `merge-base` line removed (red); `needs: [tag]` dropped from `macos` (red); `permissions: write-all` on a build job (red); a quoted `contents: "write"` (red); the one write grant moved to a job placed after `publish` (red); `continue-on-error: true` on the `tag` job (red); the `merge-base` line commented out (red) — the last five P-10's; a flow mapping `permissions: { contents: write }` on a build job (red); `contents: write # x` with a trailing comment on a build job (red); a quoted key `"contents": write` on a build job (red); `contents: !!str write` on a build job (red); an anchor on publish's grant and its alias on a build job (red); `exit 0` or `|| true` appended to the `merge-base` line (red); the check moved into a job inserted between `tag` and `ubuntu` (red); `if: always()` on a build job (red) — these P-11's; the inserted job named `Gate`, `_gate`, `"gate"` or `gate :` (red each); publish's grant removed and a job `_notes:` or `Notes:` after publish carrying it (red); `"if": always()` on a build job, `"if": false`, `if : false` and `? if` / `: false` on the tag step (red each); a second scope `packages: write` inside publish's block (red); `permissions: { contents: "#", packages: write }` on a build job (red); the tag script wrapped in `if false; then … fi` or `; exit 0` appended to the fetch line (red) — these P-12's; a decoy `  publish:` line under the top-level `env:` (an env var named `publish` with its value on the next line) with publish's grant moved byte for byte into `ubuntu` (red); the same decoy as a block-scalar `name: |` (red); `ubuntu`'s body replaced by a flow mapping under its bare head with `name: "Ubuntu #1"` ahead of `permissions: write-all` or `if: always()` (red); an anchor on a second tag pattern `- &k if` with `*k : always()` on a job (red); `defaults: run: shell: true {0}` on the tag job (red); a pre-step on the tag job putting a stub `git` on `GITHUB_PATH` (red); `env:` or `container:` on the tag job (red) — these P-13's. The reviewer re-resolves the four SHAs with the Step 3 commands and compares; dry-reads the `tag` job's logic (a tag on a commit that is not on `origin/master` fails `--is-ancestor`; a tag on master passes); confirms `ruby -ryaml` parses; and notes the other three workflows' seven tag pins for S4c's record.

---

### Task 4: the community files — CONTRIBUTING, SECURITY, the issue forms, the pull-request template · impl: opus · review: opus

The design §6.4, §2.6, §2.11, §2.24 and §10.3, §10.6; C-6. The posted issues #58–#60 and #62 model the layout (Context · What happens today · The change · Where · Acceptance · Size · Record to update when it lands); the forms carry it. No `CODE_OF_CONDUCT.md` (§10.3).

**Files:**
- Create: `CONTRIBUTING.md`, `SECURITY.md`, `.github/ISSUE_TEMPLATE/bug_report.yml`, `.github/ISSUE_TEMPLATE/rules_question.yml`, `.github/ISSUE_TEMPLATE/config.yml`, `.github/pull_request_template.md`
- Not touched: `CLAUDE.md`, `README.md` (S4b links them)

**Interfaces:**
- Produces: the `rules` label's name (created by the orchestrator at Step 6); the sentence S4b's README links as "contributing, security … in one line each"; the forms' field ids `surface`, `version`, `today`, `steps`, `expected` (bug) and `source`, `page`, `table`, `engine`, `why` (rules).

- [ ] **Step 1: Write `CONTRIBUTING.md`** (at most ~120 lines; the house voice; no history, no owner quotes, no model table), in this order:
  1. What the project is in two sentences, and where the authority lives: `guidingpath.md` (the decision log's rulings), the board `docs/handover.md`, and `CLAUDE.md`, the standing rules every session and every contributor reads.
  2. How a change lands: a branch; a pull request to master; the per-edit suites named by command (`cargo test -p yokohama-core`, `-cli`, `-oracle`, `-difftest`, `-bridge`, `cargo test -p yokohama-core --example eval_probe` and `--example solver`, the four `node bridge/tests/*.js` suites) and the lint gate `cargo clippy --workspace --all-targets -- -D warnings`; the web lane's suites whenever a change reaches what the layer compiles (`docs/web-lane.md` §2); test-first (the history is strictly TDD); a whole-branch review before the merge; `rustfmt` deliberately not adopted.
  3. The STOPs: `SEMANTICS_REV` (bumped in the same commit as any change to legality, application, scoring or setup; saves are keyed to it), the oracle (`crates/yokohama-oracle` is authored from rules sources only, never from engine logic — the independence rule in `CLAUDE.md`), the freezes and pre-registrations of the computer player — a change touching one is proposed as an issue first and lands with its decision-log row.
  4. The corpus caveat: fourteen pins read two gitignored captures and panic without them, deliberately; `YOKOHAMA_BGA_CORPUS_OPTIONAL=1` turns absence into a quiet skip for a clone without them.
  5. The rules sources: the 2017 Tasty Minstrel English rulebook (the page numbers the record cites) and the 20-page edition in the Tabletop Simulator upload (its setup steps on p.5); neither is in the tree; the KB `analysis-1/yokohama-kb/01-rules-reference.md`; the decision log's rulings; the `rules` issue form for a case where the engine disagrees with the table.
  6. AI-assisted work, its own paragraph (the owner's ask, §2.6, paraphrased): it is welcome; the person submitting is comfortable with what they send and comfortable discussing it, even if a model coaches them in that discussion.
  7. Issues and pull requests: the forms and the template, and the layout in one line.

- [ ] **Step 2: Write `SECURITY.md`:** report through GitHub's private vulnerability reporting (the repository's Security tab, "Report a vulnerability"; enabled when the repository went public); what is in scope — the table layer `crates/yokohama-web` as the self-hosted binary and its Docker image (`deploy/`), the Cloudflare Worker host (`crates/yokohama-worker`, `worker/`), the browser client (`web/`), the CLI (`crates/yokohama-cli`), and the hosted sites `yokohama.moore.nyc` and `yokohama-nas.moore.nyc`; what to expect — an acknowledgement within a week, a fix on a branch with its review, a row in `docs/security-ledger.md`, credit if wanted; out of scope — Cloudflare's and WorkOS's own services, and the open door's property that a name is a label and a key (`deploy/README.md` §2), which is by design. No address of any kind.

- [ ] **Step 3: Write the issue forms.** `bug_report.yml`: `name: Bug report`, `labels: [bug]`, body — a dropdown `surface` (The CLI · The browser client at the hosted site · The browser client, self-hosted · The Cloudflare Worker host · The self-hosted binary or its Docker image · The engine or a save file), an input `version` (the release tag, or `/api/health`'s `server_version` for a web host), textareas `today` ("What happens today"), `steps` ("The steps, or a save file"), `expected` ("What you expected"). `rules_question.yml`: `name: "Rules question: the engine disagrees with the table"`, `labels: [rules]`, body — a dropdown `source` (The 2017 English rulebook · The Tabletop Simulator edition · Board Game Arena's behaviour · Table experience), an input `page` (the printed page), textareas `table` ("What the table does"), `engine` ("What the engine does — a save file or the move list"), `why` ("Why they differ, if you know"). `config.yml`: `blank_issues_enabled: true` and one contact link, "Report a security vulnerability privately" → `https://github.com/jmoore2333/yokohama-assistant/security/advisories/new`, about: "SECURITY.md says what to expect".

- [ ] **Step 4: Write `.github/pull_request_template.md`:** `### What changes` (prose); `### The suites run` with checkboxes — the per-edit suites and the lint gate; the web lane's suites if the layer is touched; a `SEMANTICS_REV` bump in the same commit, with its decision-log row, if legality, application, scoring or setup moved; `### The documents updated` — the CLI guide pair if the CLI's text moved, the lane document's cells if `docs/web-lane.md` moved; `### The ask` — `- [ ] I am comfortable with what this pull request sends and comfortable discussing it.`

- [ ] **Step 5: Verify**

Run: `for f in .github/ISSUE_TEMPLATE/*.yml; do ruby -ryaml -e 'y = YAML.load_file(ARGV[0]); puts ARGV[0] + ": " + (y["name"] || "config").to_s' "$f"; done && git add -N CONTRIBUTING.md SECURITY.md .github/ISSUE_TEMPLATE .github/pull_request_template.md && git grep -n -i -w -E 'legal|copyright|takedown' -- CONTRIBUTING.md SECURITY.md .github/ISSUE_TEMPLATE .github/pull_request_template.md; echo "grep exit $?"; git grep -n '@' -- CONTRIBUTING.md SECURITY.md .github/ISSUE_TEMPLATE .github/pull_request_template.md; echo "address grep exit $?"`
Expected: the three YAMLs parse and print their names; both greps print nothing (exit 1). (P-15: `git grep` reads tracked files only, so the new files are intent-added first; the register words are matched WHOLE — the first pattern's `legal` matched the project's own term "legality" in the STOP sentence, which stays "legality", the `SEMANTICS_REV` contract's word; and `@` alone catches any address, not two spellings.) Every relative link in the four Markdown files resolves (`ls` each path named).

- [ ] **Step 6 (the orchestrator, outward; the owner's yes of 2026-10-23):** `gh label create rules --color 0052CC --description "the engine disagrees with the table: a rules question or a rules defect"`, then `gh issue edit 62 --add-label rules`. The style pass over #58–#60 and #62 against the forms' layout is Block B's.

- [ ] **Step 7: Commit**

```bash
git add CONTRIBUTING.md SECURITY.md .github/ISSUE_TEMPLATE/bug_report.yml .github/ISSUE_TEMPLATE/rules_question.yml .github/ISSUE_TEMPLATE/config.yml .github/pull_request_template.md
git commit -m "docs: the community files — how a change lands, the STOPs, the corpus caveat, the rules sources and the one ask of AI-assisted work in CONTRIBUTING.md; private vulnerability reporting and the scope in SECURITY.md; a bug-report and a rules-question form, the security contact link and the pull-request template in the contributor layout (S4a Task 4; the design §6.4)"
```

Checks for the reviewer: every command CONTRIBUTING.md names exists in `CLAUDE.md`'s Commands section with the same spelling; the §2.6 ask is present as a paragraph in CONTRIBUTING and as a checkbox in the template; SECURITY.md spells no address; the forms' field ids are the Interfaces'; the register (no history, no quotes, no legal talk).

---

### Task 5: `docs/web-guide.md` and `deploy/run-local.sh` · impl: opus · review: opus

The design §6.10, §10.4, §10.5, §10.9, §10.18 and §11 (f); C-7, C-10. Markdown only, no HTML twin; one screenshot; the script built beside the guide so the guide's sentence is true at its own review — the clean-eyes walk that proves it end to end is S4b's (§7.7).

**Files:**
- Create: `deploy/run-local.sh` (the siblings' mode; invoked as `bash deploy/run-local.sh`)
- Create: `docs/web-guide.md`
- Modify: `deploy/README.md:12–19` (the directory's list gains the script, one bullet)

**Interfaces:**
- Produces: `bash deploy/run-local.sh [--dry-run | --stop]`. Environment, each with its default: `YOKOHAMA_WEB_IMAGE` (`yokohama-web:local`), `YOKOHAMA_WEB_CONTAINER` (`yokohama-web-local`), `YOKOHAMA_WEB_VOLUME` (`yokohama-web-local-data`), `YOKOHAMA_WEB_PUBLISH` (`127.0.0.1:8080:8080`). It needs `docker` and `curl` and refuses by name without them; it refuses when run outside a checkout (`deploy/Dockerfile` resolved from the script's own location) and when a container of its name exists ("run `--stop` first"); before the build it prints one line saying the image is `linux/amd64` by design and that a first build on Apple silicon runs under emulation and takes minutes; the build is `docker build --platform linux/amd64 -f deploy/Dockerfile -t "$IMAGE" .` with no build values; the run is `docker run -d --name "$CONTAINER" -p "$PUBLISH" -v "$VOLUME:/data" -e YOKOHAMA_WEB_IDENTITY=open -e YOKOHAMA_WEB_HOSTNAMES_ENABLED=on --restart unless-stopped --cap-drop ALL --security-opt no-new-privileges:true --memory 512m "$IMAGE"` (loopback is in the default host set, so no list); then it polls `http://<the publish address>/api/health` every second for up to 60 s and on a 200 prints `run-local: open http://127.0.0.1:8080/` and the trust-model line `run-local: this is the open door — anyone who can reach this port may use any name; it is bound to 127.0.0.1 (deploy/README.md §2 for a LAN)`, else prints the container's last 20 log lines and exits 1; `--stop` runs `docker stop` and `docker rm` on the container and prints that the volume `$VOLUME` stays with the games, exit 0 (a second `--stop` says nothing is running, exit 0); `--dry-run` prints every command and runs none (`update.sh`'s shape). `set -euo pipefail`; the header comment states this contract (the siblings' style).
- Produces: `docs/web-guide.md` with these sections, in order: 1 What it is, and what it can do · 2 Playing at the hosted site (the door's three cards; the sign-in; the lobby and a table; the game screen; saves, the archive, offline play) · 3 Asking the bot · 4 Hosting your own (the one command, then the pointers) · 5 What is by design · 6 Where to read next.

- [ ] **Step 1: Write `deploy/run-local.sh`** to the contract above, in `smoke-image.sh`'s style (a `die`, a `run` that prints then executes unless dry).

- [ ] **Step 2: The dry run and the syntax read**

Run: `bash -n deploy/run-local.sh && bash deploy/run-local.sh --dry-run`
Expected: the platform line, then the `docker build` and `docker run` lines exactly as the Interfaces spell them (with the defaults), then the poll described, and no Docker call made (`docker ps` unchanged).

- [ ] **Step 3: The real run on this Mac** (Docker 29.9.2; the build under emulation)

Run: `time bash deploy/run-local.sh && curl -s http://127.0.0.1:8080/api/health && curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8080/`
Expected: the build completes (the wall recorded in the transcript; the self-host guide read 230 s on the owner's workstation on 2026-09-28), the health answer carries `"identity":"open"` and `"signal":"sse"`, the door answers 200, and the two `run-local:` lines print.

- [ ] **Step 4: Stop, and stop again**

Run: `bash deploy/run-local.sh --stop && bash deploy/run-local.sh --stop; docker volume ls | grep yokohama-web-local-data`
Expected: the first prints the volume-stays line; the second says nothing is running; both exit 0; the volume is listed. (The reviewer removes the volume afterwards by its explicit name, `docker volume rm yokohama-web-local-data`, and the image by its tag — single named deletions.)

- [ ] **Step 5: Write `docs/web-guide.md`.** Sources: `README.md:211–403` (the browser-client section it supersedes), `docs/web-lane.md` §14–§15, `web/src/Help.tsx` (the client's own words for the screen, the bot and the replay), `deploy/README.md` §1–§4, `worker/README.md` §1.4–§1.5. The title "The browser client: a guide for players, hosts and readers". Section 1: the same engine, the same words and the same legal-move gate as the terminal; three ways it runs (the hosted site, your own server, the bot in the browser with no server). Section 2, at `https://yokohama.moore.nyc`: the door's three cards (You against the bot — no account; Play with friends — the sign-in; On this device); the sign-in (an email code through the sign-in page `/enter`; the sentence under the link names who signs a player in and what the site keeps; sign-out in the lobby's corner); C-7's one sentence on Access until the switch; the lobby and a table (New table; a seat is you, a friend by their address, an open seat behind the link or the bot — the bot at two players; a password; the link `#/t/<id>`; each person keeps one of two dealt cards from their own view, the technologies and the goals revealed after); the game screen (the table, the side column, the answer bar; Boards and Cards; the replay track; undo — against the bot as always, at a table your own move while your turn has not passed); saves, the archive and offline (a finished game is filed to the host's archive; your own finished record downloads with its seed, which is what replays it; a bot game lives in the browser and goes to the archive when it finishes; the release installs for offline play; the door updates itself, a game in play shows the line and waits for the click). Section 3: Suggest and Why in the help panel's words; the bot's arithmetic, not advice; it read one guess at the hidden cards; a Why that cannot reproduce the recorded move refuses. Section 4: the one command and what it does, the emulation sentence, `--stop`, the trust model; then `deploy/README.md` for a LAN, a tunnel or Access, and `worker/README.md` for the Cloudflare host — with the sentence the handover's §2 routes here, said outright: a first deploy of your own Worker records it in the deploy guard's `DEPLOYMENTS` table before the guard runs (`worker/README.md` §9.3; the secrets guard alone is skipped on a first deploy). Section 5, each as a property with its reason (the lane's §15 voice): hidden information (each browser is served its own seat; the hidden set is masked for the bot too); the bot's seat (the production identity, the same the CLI plays; a game that is not over opens exactly when this build reconstructs the bot recorded in it); the legal-list gate (every move goes through the engine's legal list; the server computes nothing about the game); the Host rule (the server answers only the names it is told; an unlisted `Host` is 421); the open door's seat property for self-hosters (a name is a label and a key, so a seat is acted in by whoever types its name — bind the port where that is fine). Section 6: the help panel inside the client; `docs/web-lane.md`; the two host guides and their runbooks; `docs/cli-guide.md`; `docs/computer-player.md`. The one picture: `web/public/help/screen.webp` at section 2's game screen, its alt text a full sentence. No cap figures unless named as cost bounds; no owner quotes; no history.

- [ ] **Step 6: Add the bullet to `deploy/README.md:12–19`:** "**`run-local.sh`** builds the image from this checkout and runs it on this machine under the open door, on loopback with a named volume — the one command `docs/web-guide.md` names (`--stop` stops it; the games stay in the volume)."

- [ ] **Step 7: Verify the guide**

Run: `grep -n '^## ' docs/web-guide.md && grep -c 'screen.webp' docs/web-guide.md && git grep -n -i 'legal\|copyright\|takedown' -- docs/web-guide.md; echo "grep exit $?"`
Expected: six section headers in the Interfaces' order; `1` picture; from the register grep only the engine's own term — "legal-move gate" and "legal list", the Interfaces' words for section 1 and section 5 (P-17: the pattern is a heuristic for legal TALK, and the game term survives it; the reviewer reads each survivor as the term or as a finding); every relative link resolves, except `docs/computer-player.md`, which Task 6 creates next (P-17; the reviewer reads it as pending, and Task 9's cell reads the two guides together). Every claim of section 2 is read against the help panel or the lane's §14/§15 (the reviewer's check).

- [ ] **Step 8: Commit** (two commits: the script with its README bullet, then the guide)

```bash
git add deploy/run-local.sh deploy/README.md
git commit -m "deploy: run-local.sh — one command that builds the image from this checkout and runs it under the open door on loopback with a named volume, says the platform before the build, reads the health before it says open, and stops without touching the games (S4a Task 5; the design §6.10, §10.9)"
git add docs/web-guide.md
git commit -m "docs: the browser client's guide for players, hosts and readers — the hosted site's door, sign-in, lobby, table and game screen, the bot's two questions, hosting your own with run-local.sh, and what is by design (S4a Task 5; the design §6.10)"
```

Checks for the reviewer (on an export, on this Mac): Steps 2–4 re-run (the build wall recorded again); the guide's section 5 against `docs/web-lane.md` §15 and `deploy/README.md` §2 (the seat property's sentence true of the open door and never claimed of the provider-signed one); C-7's sentence present and true; `README.md:211–403`'s facts that the guide repeats are current (the dev door is OFF, the door's three cards, the sign-in on `/enter`).

---

### Task 6: `docs/computer-player.md` — the computer player, current · impl: opus · review: opus

The design §6.11 and §4's figures rule; C-8. The sources: the map `docs/plans/2026-07-25-phase3-map.md` (§1.9, §5), the freeze `docs/plans/2026-08-30-phase3.5c-v4-identity-freeze.md` (§1, §9.4, §9.9), the V4 close `docs/plans/2026-08-30-phase3.5c-v4-close.md` (§2.6, §3), the 3.3 close `docs/plans/2026-07-27-phase3.3-close-report.md`, the 3.4 decision `docs/plans/2026-08-04-phase3.4-architecture-decision.md`, the V2 freeze package `docs/plans/2026-08-10-phase3.5c-freeze-package.md`, the V3 close handover `docs/plans/2026-08-19-phase3.5c-v3-close-handover.md`, the execution ledger `docs/plans/2026-08-09-phase3.5c-execution-ledger.md`, `README.md:501–648` (the status section this page retires, in S4b), the board's Track 1 and `CLAUDE.md`'s Project section.

**Files:**
- Create: `docs/computer-player.md`
- Not touched: `README.md` (S4b), `guidingpath.md`, the dated documents

**Interfaces:**
- Produces: the page's sections, in order: 1 What a contestant is · 2 The named identities · 3 V4, production and provisional · 4 Suggest and Why · 5 The results ladder · 6 The bar, not claimed · 7 What is next · 8 Known limits, direction-free; the sentence `The bar (map §1.9) is NOT claimed: a shipped milestone is a narrower claim than the bar.` verbatim.

- [ ] **Step 1: Write the page** (about 150–250 lines). Section 1: a contestant is a named identity — a controller, its knobs and a weight table; honest by construction (it takes a `PlayerView`, never a raw state; the hidden set is masked; one sampled world); the arena is the court every strength claim passes through (paired, seat-swapped, bootstrap intervals, a typed refusal in place of a printed ±0.000). Section 2: `v4-step9-draft1` (production: the CLI's default seat, `suggest`'s advisor, the web's bot; the echo `v4-step9-draft1@v2{b=4,eval_impl=3,k=1,m=8,weights=6957643967275421640}`); `lookahead-v1` and `greedy-v1` grandfathered byte-identically at the V2 table; the rungs `uniform`, `retreat-averse`, `family-weighted`; the diagnostics `atomic-diag-v1`, `mcts-v1`, `v2-padded`, `v2-refit-padded` (refused as seats); a name never re-points — a different table is a different named identity. Section 3: shipped as itself on 2026-08-30 (PR #30), the freeze, provisional relative to amendment D1's post-screening freeze; a V5 build plays V4 games exactly when it still derives V4's exact descriptor. Section 4: `suggest` and `why` as `CLAUDE.md` and the help panel describe them (the honesty gate: a reconstruction that does not match the record refuses). Section 5, one row per milestone, each with the claim as pre-registered, the figure as measured with its interval, and the record: 3.3 — `lookahead-v1` vs `family-weighted`, 200 of 200 units on the committed master and again on an OS-entropy master, the all-won refusal's rule-of-three bound 0.985 > 0.5 (ONE-SIDED, stated), VP margins +67.42 / +68.44 (the 3.3 close); 3.4 — MCTS-v1 measured and NOT adopted: θ̂ 0.045 [0.0275, 0.065] and 0.050 [0.030, 0.0725] node-matched, 0.070 [0.035, 0.105] wall-matched (the ADR); V2 — `lookahead-v1@V2` vs `@V1`, θ̂ 0.6775 [0.6275, 0.7250] at N = 200 (the freeze package); V3 — NULL, both candidates washed at their pre-registered gates, nothing shipped (the V3 close); V4 — θ̂ 0.691 [0.667, 0.715] vs `v2-padded` and 0.885 [0.867, 0.902] vs `v2-refit-padded`, the self-null θ̂ 0.498454 [0.473725, 0.523184] over 647 read units (the freeze §9.9's item (f), quoted at the record's own precision — P-16: the three-decimal form was this plan's rounding, and §4 says figures are quoted AS RECORDED), three capped games in 1,300 counted not refused (the freeze §9.9); the escrow re-read three non-blocking VOIDs with 0/20 at each admitted root (the V4 close §2.6). Every figure copied from its record, never from this plan. Section 6: the map's §1.9 four ingredients quoted, and the sentence of the Interfaces. Section 7: the V4 vetting (the board's Track 1: the owner's annotated games, the findings package, D3 and W6 ruled together), then V5 through its own pre-registration; a weight change is a different contestant. Section 8: the self-play deadlock rate 0.30% [0.14%, 0.65%] and the long-game tail (the freeze §9.4); the first-mover share in self-play 65.5% (recorded, direction-free); the roster target ≤420 s crossed on the M5 (OD-6 ripe); every maximum "max observed under harness X". No model table; no history beyond the dates the ladder needs.

- [ ] **Step 2: Verify the figures and the fences**

Run: `grep -c 'The bar (map §1.9) is NOT claimed: a shipped milestone is a narrower claim than the bar.' docs/computer-player.md && grep -n -E '0\.691|0\.885|0\.498|0\.6775|0\.045|0\.050|0\.070|0\.985|0\.30%' docs/computer-player.md | wc -l && git grep -n -i 'beats all\|plays to win' -- docs/computer-player.md`
Expected: `1`; nine or more figure lines; the last grep prints only the map's quoted bar sentence, if any.

- [ ] **Step 3: Commit**

```bash
git add docs/computer-player.md
git commit -m "docs: the computer player, current — what a contestant is, the named identities, V4 as production and provisional, suggest and why, the results ladder with every figure as recorded and its record named, the bar not claimed, what is next and the known limits (S4a Task 6; the design §6.11)"
```

Checks for the reviewer (Review Focus 4): every figure in section 5 and 8 is found in the record the row names (`grep` each number in that file); every interval is two-sided unless the sentence says one-sided; no sentence asserts a direction without a measured interval; the bar sentence verbatim; no model or co-author table; the page names no owner quote.

---

### Task 7: the copyright sweep — the report, and the live-document edits · impl: sonnet · review: opus

The design §2.7, §6.3's last sentence, §4 and §10.12. The rule, written once here, is the sweeping agent's whole instruction:

> **The subject** is the game's components and their rights. **A hit** is a tracked line matching, case-insensitively, `copyright|licen[cs]e|rights.holder|Asmodee|Tasty Minstrel|TMG|OKAZU|Okazu|Synapses|takedown|take-down|removal (on|upon) request|infring|(with|without|ask(ed|ing)? for|gives?|granted|no) permission|permission (to|of|from)|relicense|carve-out|carved out|excluded path|no grant|not legal advice|legal conclusion`, outside `LICENSE`, the two `OFL.txt` copies, the three archives, `.superpowers/`, `ASSETS.md`, `NOTICE` (Block B rewrites both) and the two `package-lock.json` files (P-18: the first pattern's bare `permission` and `removal` matched the API's `permissions` vocabulary, Docker's volume removal and the engine's "removed" pile, and the lockfiles' `license` fields alone made 377 of 1,309 lines; the refined pattern reads about 240 lines at the run's head). **A match whose subject is not the game's components and their rights** — a GitHub permission, a file's removal, a font's or a package's licence line, the OFL — is NOT-SUBJECT: listed by file with a count, never row by row. **A line is a FACT and stays** when it states what is third-party, whose it is, where it sits, or what a fork must do. **A line ARGUES A POSITION and is edited** when it draws a legal conclusion or hedges one ("not a legal conclusion", "not legal advice"), asserts a permission or its absence beyond the fact, states a removal-on-request or takedown policy, or gives the reason the material is here ("personal use", "private and invite-only"). A dated document (`docs/plans/`) or a code comment that argues a position is edited and the edit recorded in the Review record and the close; a dated document that states facts is not edited, whatever path it names.

**Files:**
- Modify: whichever live files the sweep finds arguing a position (expected: `deploy/Dockerfile.dockerignore:30–36`'s "pushing them to a registry is redistribution" — re-worded to the fact that the image carries them and is not pushed to a public registry; `deploy/README.md:21–22` reads as a fact and stays; the crates' `Cargo.toml` comments and `web/src/Help.tsx:39` read as facts and stay)
- Report: the transcript file, with the table below

- [ ] **Step 1: The hit list.** `git grep -n -i -E '<the pattern above>' -- . ':!LICENSE' ':!**/OFL.txt' ':!guidingpath-archive*.md' ':!.superpowers' ':!ASSETS.md' ':!NOTICE' ':!**/package-lock.json'` into the transcript; one row per SUBJECT hit: path:line · the line · FACT / POSITION / DATED-FACT (untouched) · the edit, if any; NOT-SUBJECT matches by file with a count (P-18). The font families' OFL sentences in `web/src/precache.test.ts` and the Help panel state facts about the fonts and stay; the two S4 documents under `docs/plans/` that state the register's own rules are DATED-FACT.

- [ ] **Step 2: Edit the POSITION hits** in live files, each to the fact it rests on, in one sentence; a dated or code-comment hit that argues a position is edited too and flagged in the report.

- [ ] **Step 3: Verify**

Run: `git diff --stat` and, if `docs/web-lane.md` moved, `cargo test -p yokohama-web --test lane_document && (cd worker && npm run test:scripts)`; if a crate's comment moved, `cargo clippy --workspace --all-targets -- -D warnings`.
Expected: the diff touches only files the report names as POSITION; the cells green.

- [ ] **Step 4: Commit**

```bash
git add <the files the report names>
git commit -m "docs: the copyright sweep — every tracked line on the game's components read; the lines that argued a position now state the fact they rested on, the facts untouched, the dated records untouched (S4a Task 7; the design §2.7, §6.3)"
```

Checks for the reviewer: re-run Step 1's grep on the export and read every row's verdict against the rule; a FACT edited or a POSITION left is a finding; no dated document moved unless the report flags it with the position it argued.

---

### Task 8: the sweep of releases, pull-request bodies and the owner's address · impl: sonnet · review: opus (the outward edits in Block B)

The design §6.6, §2.16, §10.20 and Appendix A's pattern list. The in-tree edits are this task's commit; the edits to release notes and PR bodies are drafted here and applied by the orchestrator in Block B.

**Files:**
- Modify: `web/e2e/harness.node.test.ts:367–369, 427–428`, `worker/scripts/live-replay.test.mjs:505, 913, 915`, `worker/scripts/mint-session.test.mjs:91, 93, 145, 186` — the owner's address, the example of a non-`.test` address the minting fence must refuse, becomes `someone@example.com` in the input AND in the expected sentence of each cell (their meaning unchanged: the fence still names the refused address)
- Modify: `docs/security-ledger.md:27` (row C1) and `worker/RUNBOOK.md:2690` — the address → "the owner's address"
- Report: the transcript file — every release note (10) and PR body (57) read, the hits and the proposed replacement text, each as a file under the session scratchpad the orchestrator applies

**Interfaces:**
- Consumes: the owner's address (the one `git log --format=%ae | sort | uniq -c` shows on 2,369 commits — the brief spells it; this plan does not), the owner's test identity (named in the orchestrator's brief, gitignored — this plan does not spell it), the two handles of the fixture at `d5dd0007` (read from `git show d5dd0007:crates/yokohama-bridge/tests/data/excerpt-895724349.ndjson`, the two non-placeholder `player_name` values — never written into any report in the clear; the report names the line, not the value).

- [ ] **Step 1: The in-tree edits** at the eleven sites; then

Run: `git grep -c -F '<the owner's address>'; echo "exit $?" && (cd web && npm run test:wasm) && (cd worker && npm run test:scripts)`
Expected: the grep prints nothing (exit 1); `test:wasm` and `test:scripts` green at their baselines (the cells' assertions moved with their inputs).

- [ ] **Step 2: The releases and the pull requests, read.** `for t in $(gh release list --limit 20 --json tagName --jq '.[].tagName'); do gh release view "$t" --json body --jq .body > "$OUT/release-$t.md"; done` and `for n in $(gh pr list --state all --limit 100 --json number --jq '.[].number'); do gh pr view "$n" --json title,body --jq '.title, "", .body' > "$OUT/pr-$n.md"; done` into the session scratchpad; then one grep over them with the patterns: the owner's address; the test identity; the two handles; the secret shapes of Appendix A (`-----BEGIN`, `sk_live`, `AKIA`, `ghp_`, `gho_`, `xox[bap]-`, `CLIENT_SECRET=`, `API_TOKEN=`, `SESSION_SECRET=`, `CF_ACCESS_CLIENT_SECRET=`, `YOKOHAMA_DENIED`'s values, `Bearer `); and Task 7's position patterns. For every hit: the file, the line, and the replacement text, in the report; the full replacement body written beside it as `release-<tag>.edited.md` or `pr-<n>.edited.md`. The replacement rule by class: the owner's address and the test identity → "the owner's address" / "a test address of the owner's"; a handle → the placeholder its seat carries in the fixture; a secret shape → the line removed and the report says so (a real secret would be ROTATED, not scrubbed — say so and stop); a position → the fact it rests on, by Task 7's rule.

- [ ] **Step 3: Commit the in-tree edits**

```bash
git add web/e2e/harness.node.test.ts worker/scripts/live-replay.test.mjs worker/scripts/mint-session.test.mjs docs/security-ledger.md worker/RUNBOOK.md
git commit -m "docs, tests: the owner's address leaves the five tracked files — an example.com address is the non-.test address the minting fence refuses in the three cells, and the ledger's C1 row and the runbook's record say 'the owner's address' (S4a Task 8; the design §6.6)"
```

Checks for the reviewer: the grep of Step 1 on the export; the three cells' expected sentences name the new example address (a cell whose assertion still named the old one would be red, so green IS the check); the report's hit list against a re-run of Step 2's grep on the reviewer's own download of the 67 bodies; no handle or address written in the clear anywhere in the report.

---

## Block B — the owner present (the orchestrator's edits)

- [ ] **B1. The notice set** (the design §6.3, §10.16; `feedback-public-register-no-legal-talk`). With the owner at the keyboard, `NOTICE`'s third-party paragraph (lines 12–17) is rewritten in the live exchange; `ASSETS.md` is rewritten as a notice of FACTS in this order: what is third-party and where — `crates/yokohama-web/assets/art/` (the derived crops and `manifest.json`), `web/public/help/` (the help screenshots), `worker/docs/img/` and every README picture that shows the art (closing C4; `docs/images/` holds text renders and says so); whose it is — the lineage, written with the owner (the designer; the publishers of the editions the art comes from; the current rights holder); that this project is unaffiliated with and not endorsed by any of them; what a fork must do; the two OFL font families (today's second section, facts only); the KB's data line (§10.16: the rules references and card data under `analysis-1/yokohama-kb/` are this repository's own prose and data structures; the rules and card text they transcribe are the game's). Gone: "private and invite-only", "the planned public split" and its fresh-history paragraph, "Why they are here", every hedge about legal conclusions, the rows of the paths that left. The font cell `the font licences` in `web/src/precache.test.ts` stays green (it reads the OFL copies). Commit: `git add NOTICE ASSETS.md`, the subject naming the notice of facts and C4.
- [ ] **B2. The outward edits (outward):** Task 8's `*.edited.md` applied — `gh release edit <tag> --notes-file …` and `gh pr edit <n> --body-file …` — each read back with `gh release view`/`gh pr view`; the list of what changed in the ledger and the close.
- [ ] **B3. The issues' style pass (outward):** #58, #59, #60 and #62 re-read against Task 4's forms; a body that departs from the layout is edited with `gh issue edit <n> --body-file …`; what moved recorded. (#53, the turn alert, is the owner's own and stays unless the owner wants it re-shaped.)
- [ ] **B4.** Anything of Block A the owner left for later (the preview upload; the GitHub e-mail settings).

---

### Task 9: the records — the ledger, the lane document, the board, CLAUDE.md's licensing sentence, and C4's cell · impl: sonnet · review: opus

The design §6.5 (C4 closes with the notice), the handover §2 (the two parked records items: `docs/web-lane.md` §10's deploy-scripts list omits `deploy:private`, corrected here; the first-deploy recipe's `DEPLOYMENTS` sentence is said outright by Task 5's guide), C-5 and C-9. The four polish minors the handover parks (T2-m2, T3-m1, T4-m1, T5-m3) sit in files no task of this cycle opens and stay parked; the close says so. Dated documents under `docs/plans/` are not edited.

**Files:**
- Modify: `docs/security-ledger.md` (row C4 at 30; the `Updated` line at 5; D17 and D20 read `fixed` from Tasks 2–3 — verify)
- Modify: `docs/web-lane.md` (§10 at ~498–500: "and `deploy:open` and `deploy:dev` the same three over their files" → "and `deploy:open`, `deploy:dev` and `deploy:private` the same three over their files"; any §14/§15 sentence Task 5's guide now carries is pointed at the guide in one clause, not duplicated)
- Modify: `docs/handover.md` (the `Updated` line; Track 2's heading and body → "S4a LANDED on `security-s4a` … NEXT: S4b, the front door (the design §7)", with the design's §6 items ticked by task; Track 4b's heading → merged as PR #61 `d5dd0007`; the rulings list gains "13. The Language School hotfix — tracked as issue #62 (2026-10-23): not merged in S4a, its branch stays on origin unmerged, the rev bump waits for its cycle after the V4 vetting"; the entry artifacts gain `docs/web-guide.md` and `docs/computer-player.md`)
- Modify: `CLAUDE.md` — the Worktree-hygiene licensing sentence: "[`ASSETS.md`](ASSETS.md) carves out three third-party paths that carry NO grant of any kind — `tts-assets/`, `crates/yokohama-web/assets/art/`, and `analysis-1/00-yokohama-rulebook.pdf`" → "[`ASSETS.md`](ASSETS.md) names the third-party paths that carry NO grant of any kind — `crates/yokohama-web/assets/art/`, `web/public/help/` and `worker/docs/img/` — so a crate whose package would bundle them declares `publish = false`" (C-9)
- Create: `web/scripts/assets-notice.node.test.ts` (under `npm run test:wasm`, which includes `scripts/**/*.node.test.ts`)

**Interfaces:**
- Produces: the cell `ASSETS.md names every directory a README or guide picture lives in`: it collects every `](<path>.png)` and `](<path>.webp)` reference from `README.md`, `worker/README.md`, `docs/web-guide.md` and `docs/computer-player.md` (a multi-line alt text ends at `](`), takes each path's directory, asserts at least three distinct directories were found (anti-vacuity), and asserts every one of them is spelled in `ASSETS.md` (as an excluded path, or in the sentence that says `docs/images/`'s pictures are text renders carrying no art).

- [ ] **Step 1: Write the failing cell** as the Interfaces describe; run `cd web && npm run test:wasm -- assets-notice`; expected: it fails only if Block B's ASSETS.md omits a directory — if it passes at once, the implementer proves the cell by removing the `worker/docs/img/` row in a scratch copy and seeing it fail, and says so in the transcript.
- [ ] **Step 2: The ledger.** Row C4: `| C4 | README pictures carry carved-out art outside \`ASSETS.md\`'s table | C | licensing | S4 | fixed <Block B's sha> (S4a: ASSETS.md names \`worker/docs/img/\` and the README pictures that show the art beside \`web/public/help/\` and the crops; \`docs/images/\` named as text renders) | \`web/scripts/assets-notice.node.test.ts\` | — |`. The `Updated` line: 2026-10-23/24, security S4a's records (the branch, the three rows closed, the sweeps, the notice) in the line's existing shape, the previous entry demoted to "Before it".
- [ ] **Step 3: The lane document, the board, CLAUDE.md** as the Files block says; the board's `Updated` line names S4a's close document (the orchestrator writes it after this task; the line names its path).
- [ ] **Step 4: Verify**

Run: `cargo test -p yokohama-web --test lane_document && (cd worker && npm run test:scripts) && (cd web && npm run test:wasm) && git grep -n -E 'deferred S4|open — S4' -- docs/security-ledger.md; echo "exit $?"`
Expected: the cells green (`test:wasm` at its baseline + 1); the grep prints nothing (exit 1).

- [ ] **Step 5: Commit**

```bash
git add docs/security-ledger.md docs/web-lane.md docs/handover.md CLAUDE.md web/scripts/assets-notice.node.test.ts
git commit -m "docs: S4a's records — C4 fixed with a cell that holds the notice's picture directories to the tree, the ledger's Updated line, the lane's deploy-scripts list naming deploy:private, the board's Track 2 and Track 4b and the Language School ruling, CLAUDE.md's licensing sentence at the notice's paths (S4a Task 9)"
```

Mutant for the reviewer: the `worker/docs/img/` row removed from ASSETS.md on the export (red); a picture reference added under a directory ASSETS.md does not name (red). The reviewer reads every rewritten board and ledger sentence against the tree and the issue tracker (the numbers, the SHAs, the states).

---

## After Task 9 (the orchestrator)

- [ ] **The suites at the head, once, as one sequence in a fresh export with its own `target/`:** `cargo test -p yokohama-web`, `-wasm`, `-worker`, `-bridge`, `-cli` (with `bridge-captures/` symlinked), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p gen-data && git diff --exit-code crates/yokohama-core/src/game/data.rs`; in `worker/` `npm run build:module:debug && npm run typecheck && npm test && npm run test:scripts`; in `web/` `npm run build:wasm && npm run typecheck && npm test && npm run test:wasm && npm run e2e`; `bash deploy/run-local.sh --dry-run`. The counts and the wall go in the close. No roster: the engine crates, the oracle and `SEMANTICS_REV` did not move.
- [ ] **The whole-branch review** (Fable, on an export of the head, the owner's branch rule): one wave of fixes as its own commits by an Opus fixer (a fixer who finds the same defect at a site outside the brief reports it, and the same wave lands it), then a scoped re-review on an export of the wave's head.
- [ ] **The close, `docs/plans/2026-10-23-security-s4a-close.md`:** the tasks, their reviews and verdicts; the one recorded sequence; every ruling in order with its cost; what the cycle leaves; **the relocation map** (the design §6.12, the pattern of `docs/plans/2026-08-19-docs-consolidation.md`): one row per removed or moved path — `tts-assets/` (118 files: the sheets, scans, both PDFs and the mod's save file → outside the tree, a contributor's own copy at the gitignored path; the three repository-authored files → `docs/tools/tts-extraction/`), `analysis-1/00-yokohama-rulebook.pdf` (→ a contributor's own copy; its page numbers are the ones the record cites) — and one row per section of the README at `d5dd0007` → its home (See it running → S4b's README picture and `docs/computer-player.md`; The browser client → `docs/web-guide.md`; Hosting → `deploy/README.md`, `worker/README.md`, the guide's §4; Status → `docs/computer-player.md`; Workspace crates → S4b's README and docs index; Download and run / Build from source → S4b's quick start and `CONTRIBUTING.md`; Data provenance → the KB's `00-README.md` and `ASSETS.md`'s data line; Where truth lives → S4b's docs index and `CONTRIBUTING.md`), S4b's destinations marked as S4b's; **the second workstation's recipe** (`git remote add lab https://github.com/jmoore2333/yokohama-lab.git && git config remote.pushDefault lab && git config user.email 59934696+jmoore2333@users.noreply.github.com && git fetch lab`); the branch deletion list; the first commit under the no-reply address (A1's); the preview image's recipe and digest; **what S4b inherits** (the README from a clean sheet, the docs index, the method page, the clean-eyes walk that proves `run-local.sh`, the cross-model read) and **what S4c inherits** (the tag ruleset; private vulnerability reporting; `test.yml`'s push trigger and the other three workflows' pins; the guide's Access sentence re-pointed at the switch; the site's link previews read by external fetchers once Access is off — the Open Graph, Twitter-card and JSON-LD metadata of `yokohama.moore.nyc`, which `web/scripts/presence.mjs` writes — the owner's "validate in s4c all the social previews once the access controls are removed").
- [ ] **The wrap commit:** the close; the board's `Updated` line and gates line; guidingpath's 2026-10-23 (or -24) row and its Current state's one sentence; the ledger's `Updated` line if the close moved it; one new standing rule or lesson in `CLAUDE.md` only if the cycle earned one (the lab rule itself is S4b's §7.4 sentence).
- [ ] **The PR (outward)** `security-s4a` → master with the close linked; merged on the owner's word. S4b is cut from master after it, and its planning session opens from the close's last section.

---

## Review record

The before review (Block A, A3) and every correction the run meets are recorded here, each applied at its site above and tagged `(P-n)`; the SDD ledger holds the rulings as R-n.

- **P-1** (2026-10-23, the owner, after a side agent's note) — the first commit's item 8, A7 and the S4c-inherits list read the owner's "validate it" as a validation of GitHub's social preview, which GitHub renders at once and never validates. The owner: "we'll want to validate in s4c all the social previews once the access controls are removed — github we can see prior." The three sites now say: the repository's preview is uploaded and read back in GitHub now; the SITE's link previews (the Open Graph, Twitter-card and JSON-LD metadata an external fetcher reads from `yokohama.moore.nyc`, refused by Access today) are S4c's reading after the switch.
- **P-2** (2026-10-23, the planning session's close) — the board was brought to this session's state in the branch's third commit: its `Updated` line, Track 2's and Track 4b's headings, and ruling 13 for the Language School hotfix. Task 9 rewrites the `Updated` line and Track 2 again at the cycle's end, and finds Track 4b's heading and ruling 13 already true. The A3 baseline suites ran at `e69c2216` and read equal to S4-pre's close on every count (the SDD ledger).
- **P-3** (2026-10-23, the planning session's close) — item 5 listed "the fifty, `security-auth`, `security-s3` and `security-s4-pre`", which sums to 53, while A4's computed list reads 52 at `d5dd0007` (the measured count; the three named branches are among the 52). Item 5 now gives the measured figure; the first commit's subject says "fifty-three" and stands as written. A4 deletes the list it computes and records, never a figure from prose.
- **P-4** (2026-10-24, the run's pre-flight, A3) — A4 step 7's list command read 53 at the run's start: `git branch -r` prints `origin/HEAD -> origin/master`, which `sed` turns into `HEAD -> origin/master`, and the exact-match filter for `HEAD` never matches that line. The command now drops it with `grep -v -- '->'`; the clean list reads the measured 52. Amended at the run (the same day): once step 5 has pushed to the lab, `git branch -r` also lists `lab/*` refs merged into master and the list read 109 — the count gate (52 or STOP) refused the run, and the command gained `--list 'origin/*'`.
- **P-5** (2026-10-24, the pre-flight; the ruling R-2 in the SDD ledger) — Task 2's Step 3 said the two surviving `player_name` values take "the placeholder that the same `player_id` carries in the file's other frames". The ids beside them are REAL BGA player ids and appear on no other line: the excerpt's `gamedatas` pairs `11111111` with `Player_A` and `22222222` with `Player_B`, and the gitignored corpus shows the capturing player's own channel is the one the excerpt spells as `p11111111`, line 13 is a message on BGA's general-chat channel and line 14 a status notification about a third person — neither author was seated at the table. Ruling: both lines take `Player_B` and `22222222` (the one other participant the fixture knows; a plausible table story that no reader distinguishes — `facts.rs` lists `chat` and `playerstatus` among the ignored frame types and `feed.rs` counts non-table channels), the real ids are redacted with the names (the file's header promised ids redacted, and a BGA id is identifying), the cell gains the id check and a doc comment records the fold, the ledger row says so, and the mutant list gains an id mutant. Cost if wrong: a fixture story a future seat-aware test could lean on; recorded where such a test would read it.
- **P-6** (2026-10-24, the pre-flight) — Task 1's Step 5 listed the sheets-absent guard among the grep's literal survivors; the guard spells the path through `SHEETS.relative_to(ROOT)` and the grep will not list it. Step 6 assumed the guard is exercised by `--check`; PyYAML is absent on this Mac, so the run stops at the PyYAML sentence first, and the step gains a second run with an empty `yaml.py` stub on `PYTHONPATH` from the scratchpad that reaches the guard. Line readings: the Dockerignore comment starts at 29, not 30; `release.yml`'s toolchain comment sits at 33–41, not 31–39 (Task 3's Step 3 names it by content, so no edit there).
- **P-7** (2026-10-24, Task 1b's review and its fix round) — the review (Opus, on an export of `00943a82`) read the task spec-compliant and found the plan's blast radius short: three live sentences locate the English PDF "bundled in `analysis-1/`" without spelling its path, so neither Step 5 grep saw them. The KB's two D1 notes (`01-rules-reference.md:542`, `:549`) are re-worded in the fix commit `a90642a4` with the line count held (code cites the KB by line number); the eight code-comment sites in `crates/yokohama-core` and `crates/yokohama-oracle` are fenced this cycle and are RECORDED FORWARD to the close's relocation map and the next cycle that opens those crates (the Language School rev-8 cycle, issue #62, is the natural one) — so no document of this cycle claims "every live reference". Also folded into the plan from the review: Step 5's second grep expectation omitted `CLAUDE.md:20` (Task 9's sentence); the Interfaces' sheets sentence names the raw `images/` cache the tool also reads; `tables.rs` shipped ", is" (the brief's "and is" left a fragment); `docs/web-lane.md:422`'s first wording made the 2026-10-22 ruling report a 2026-10-23 fact; the Dockerignore comment names the PDF's local copy too; the implementer's three extra `CLAUDE.md` phrase edits (phrases the briefed edit would have left false) were accepted by the reviewer. The fix round landed M1, M2, M3, M5, M6, M7 beside the Important; M4 is this record. The scoped re-review (Opus, on an export of `a90642a4`) read all seven addressed with no new breakage, and noted that Step 5's second grep now also lists the Dockerignore comment (true since M6; the expectation above says so) — two nits deferred to the whole-branch review in the SDD ledger.
- **P-8** (2026-10-24, the owner) — A7's first file was a 2:1 crop of the help panel's game screen; the owner, having reviewed it: "the social preview should be more like what we have on the home page, not a screenshot of an active game". A7 now captures the door — the built client served locally under the open door with the art as `d5dd0007` shipped it (the checkout's manifest carries Task 1b's edited provenance string, which the release's precache list does not expect, and the first local capture showed the third card's "could not prepare to play offline" line for that reason alone), a fresh Playwright profile at 1280×640, the third card settled at "Ready to play offline". The file and its digest are in the close; the game-screen file is withdrawn.
- **P-9** (2026-10-24, Task 2's implementer) — the cell's code block as written (P-5's id check, and the brief's reading rule for `values_after_key`) tripped the lint gate: clippy's `manual_pattern_char_comparison` refuses a `|c: char| matches!(c, …)` closure where a char array is the pattern. The implementer shipped `trim_end_matches([',', '}', ']'])` and `trim_start_matches(['\\', '"', ':', ' '])` with the same semantics (commit `8c66fc98`); the code block above now carries the shipped form, so a transcription from the plan is lint-clean.
- **P-10** (2026-10-24, Task 3's review; three rulings) — (i) The Interfaces said the corpus comment reads "FOURTEEN pins … as CLAUDE.md says"; the release workflow runs `cargo test -p yokohama-cli` alone, so the run holds the CLI's NINE (the whole of `bga_import.rs`) — CLAUDE.md's fourteen counts the bridge's five, which the release never runs. The Interfaces say nine now; the first workflow commit `76ae7a39` said fourteen in its comment and its message, which the fix round corrects in the file and this record keeps. (ii) C-4 said "first-parent history" while the Interfaces, the cell and the shipped job check ancestry (`git merge-base --is-ancestor`). Ruling: ANCESTRY — the last three releases were tagged on their branches before the merge; under ancestry such a tag is refused at push time and publishes on a re-run once the merge lands, under first-parent it would be refused forever. C-4 says so; S4c's `0.7.3` is tagged AFTER its merge (the close's S4c-inherits list). (iii) The permissions cell as written pinned one spelling of one scope: `write-all` on a build job, a quoted `"write"`, or the grant on a job placed after `publish` all passed it green (the reviewer's probes X1–X3). The code block carries the reviewer's walked form (exactly one write grant in any spelling, inside the publish job's span) and a span check on the `tag` job (the `merge-base` line uncommented inside it, no `continue-on-error`, no `if:`); the mutant list grew by five. Also folded in: the step's loud failure message (the reviewer's m-3) and the ledger row's boundary clause (m-4: the job covers commits from the workflow commit on; the ruleset covers earlier ones).
- **P-11** (2026-10-24, Task 3's scoped re-review) — P-10's matcher, a line regex for `<scope>: write`, still read green under spellings GitHub accepts that land write on a build job: a flow mapping, a trailing comment, a quoted key, an explicit `!!str` tag, an alias to publish's grant; and the tag line, anchored only at its start, passed `exit 0`, `|| true` and the check moved into a job inserted before `ubuntu`. The code block carries the re-reviewer's walked and measured form: comments blanked to equal-length spaces, the WORD `write`/`write-all` counted once inside publish's span bounded by the next job head, anchors/aliases/merge keys refused outright; the tag line anchored to end at `origin/master` or the exact loud clause, the span bounded by the next job head, and no `if:` or `continue-on-error` anywhere in the file (the shipped file has none; `if: always()` on a build job could let it run after a failed tag job). The fix round 2 transcribes it; the mutant list grew by eight.
- **P-12** (2026-10-24, Task 3's second scoped re-review) — P-11's span bounds read a job head as `[a-z][\w-]*:`, so a job named `Gate`, `_gate`, `"gate"` or `gate :` (all GitHub-accepted) carried the moved tag check or a write grant past both spans with the cells green (R-1), and the `if:` refusal was literal (`"if":`, `if :`, `? if` — R-2). A text-level cell cannot win a spelling race against YAML's grammar, so the block pins STRUCTURE instead: every line at two-space indent under `jobs:` is one of the five bare job heads in order (a job of any other spelling is red); the token `permissions` occurs exactly twice, each block pinned byte for byte with no second scope (a grant under any other key spelling is red); the tag step pinned whole as its span's last lines (no wrap, no extra line, no `exit 0`); the `if` key refused in every spelling on comment-blanked text; anchors, aliases and merge keys refused with or without a space after the colon. Recorded, not pinned (the re-review's R-3): a YAML escape that spells a key or a value without its letters — no plain edit produces it. The fix round 3 transcribes the block; the mutant list grew by thirteen. The whole-branch review may still rule for a parse-based cell; this is the strongest text-level form.
- **P-13** (2026-10-24, Task 3's third scoped re-review) — six of seven items settled; the round's own diff regressed one bound (`jobsAt` searched the whole file, so a decoy `  publish:` line under `env:` moved publish's span to it and a grant on `ubuntu` read green — N3-1), the quoted-`#` half of R-4 stayed open for a flow-style job body under a bare head, an anchor on a sequence entry with an alias used as a key passed the colon-bound refusal (N3-2), and the tag job's environment (`defaults.run.shell`, a pre-step stubbing `git`) could neuter the pinned step (N3-3). The block now bounds every lookup under `jobs:`, requires every job body to be block style, blanks the run bodies before refusing any `&` or `*` token at a node start, and pins the tag job WHOLE (an edit to that job is a deliberate edit to the cell, which is what a gate wants). Fix round 4 went to a fresh Opus implementer by the SDD escalation rule; the mutant list grew by eight; the re-review's twelve legitimate-edit checks are the future editor's guide (recorded in the report).
- **P-14** (2026-10-24, Task 3's fourth round, the Opus implementer's BLOCKED report) — P-13's anchor refusal `(^|[\s\[{,])[&*]` read RED on the shipped file: line 62's one-line `run: sudo apt-get update && sudo apt-get install --yes musl-tools` is neither a block-scalar body (which `body` blanks) nor quoted, and ` &&` matched. The implementer stopped without committing, as the brief said, and measured two candidates in a replica: both green on the shipped file and red on all twelve mutants of the round. The block takes candidate 1, `(^[ \t]*|[:?,\[{-][ \t]*)[&*]` — an anchor or alias token at a YAML node start only — because an alias always sits at a node start and an anchor without an alias is inert. **The orchestrator's ruling on the loop (the owner's question, 2026-10-24):** this is the cell's final text-level form; the cell guards DRIFT reaching master through a reviewed merge, not an adversary who can rewrite the file wholesale on a branch (a `v*` tag runs the workflow at its own commit — the ledger row says so), so any further spelling that needs a deliberate edit through a reviewed merge is RECORDED as Minor, never a fix round; the whole-branch review is asked whether a parse-based cell (a `yaml` dev dependency the Worker does not have today — neither lockfile carries a YAML parser) should replace it.
- **P-15** (2026-10-24, Task 4's implementer) — Step 5's register grep matched the project's own term "legality" (the STOP sentence, the `SEMANTICS_REV` contract's word), and the implementer reworded the sentence to "which moves are allowed" to make the grep pass; `git grep` also reads tracked files only, so on untracked new files the grep proved nothing until `git add -N`. The command now intent-adds the files and matches the register words whole; the term "legality" stays, restored in the review's fix round; `@` alone is the address pattern.
- **P-16** (2026-10-24, the orchestrator's pre-flight of Task 6) — every figure of Task 6's ladder was grepped in the record the plan names; all present, but the self-null was written at three decimals (`0.498 [0.474, 0.523]`), a rounding of this plan's, where the freeze §9.9 (f) records `θ̂ = 0.498454, [0.473725, 0.523184]`; the design §4 says figures are quoted as recorded, so the Step 1 text now carries the record's digits.
- **P-17** (2026-10-24, Task 5's implementer) — Step 7's register grep, a heuristic for legal talk, matches the engine's own term in the Interfaces' words ("legal-move gate", "legal list"); the expectation names those two as the survivors. The guide's section 6 links `docs/computer-player.md`, which Task 6 creates next, so the "every relative link resolves" check reads that one as pending until then.
- **P-18** (2026-10-24, the orchestrator's pre-flight of Task 7) — the sweep pattern as written read 1,309 lines at the run's head: the lockfiles' `license` fields (377), the API's `permissions` vocabulary in the Worker's tests, Docker's "removal", the engine's "removed" pile. The pattern now limits `permission` and `removal` to their prose forms, excludes the two lockfiles, and the rule names the NOT-SUBJECT class, listed by file with a count: about 240 lines remain, which a sweep can read line by line.
- **P-19** (2026-10-24, Task 7's sweep, commit `60dad0e4`) — the sweep read 231 matched lines, 104 on the subject, and edited two: `deploy/Dockerfile.dockerignore:34`, whose clause "and pushing it to a registry is redistribution" argued a position (a line the pattern itself does not match — the sweep found it by reading), now ending at the fact ("`ASSETS.md` carves the rest out of the Apache-2.0 grant"); and the dated `docs/plans/2026-08-31-web-frontend-wave0-evidence.md:845`, the 2026-08-31 review's recommendation, whose "reproduced for personal use" gave the reason the material is here — **the plan's second named exception to "dated documents are not edited", recorded here and in the close**: the clause is dropped, the recommendation's remaining words are the reviewer's own. Every other subject line states a fact and stays.
