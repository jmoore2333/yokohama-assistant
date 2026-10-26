# Security S4b and S4c — the front door and the flip, one sequence: implementation plan (written 2026-10-26)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the repository its front door — the README from a clean sheet, the docs index, the method page, the agent-facing documents re-pointed at the AI lane — and then flip it: the version `0.7.3` on both hosts by the auth cycle's stage 2, the repository public, the settings of the window, the readings from the outside, the security project's close and the decision log's archive cut, in ONE branch, ONE plan, ONE whole-branch review, ONE pull request and ONE close.

**Architecture:** Two parts on one branch. Part 1 (the design §7) is documents and one browser sentence: five documents written or rewritten, one cell that holds their links to the tree, a clean-eyes walk by a reader who knows nothing else, a cross-model read triaged as a wave. Part 2 (the design §8) is the flip's preparation — the three workflows pinned and `test.yml`'s push trigger, the `docs/temp/` triage, the ledger's and the lane document's pre-window records, the version bump with stage 2's `YOKOHAMA_TURNSTILE` commit — then the suites, the fifteen-gate roster at the review head, the review, the merge, the tag, and the window with the owner present: the release, the NAS as the canary, the Worker's stage 2 verbatim from `worker/RUNBOOK.md` §23, the switch to public, the settings, the readings from the outside, the close with the archive cut, and branch protection as the last act. The engine, the bot, the oracle and `SEMANTICS_REV` do not move; the workspace version does, which is why the roster runs.

**Tech Stack:** git, `gh`, bash; Markdown; vitest node cells (`web/scripts/*.node.test.ts` under `npm run test:wasm`; `worker/scripts/*.test.mjs` under `npm run test:scripts`); Docker (Desktop, on this Mac); the Codex CLI 0.164.0 (`/opt/homebrew/bin/codex`) through the Codex plugin for the cross-model read; the Cloudflare API and `wrangler` for stage 2 (the runbook's recipes); the GitHub REST API through `gh` for the window's settings.

**Spec:** the design `docs/plans/2026-10-22-security-s4-public-release-design.md` — §7 is Part 1's content and §8 Part 2's, §5 the window's order, §2 the owner's settled inputs, §4 the rules that bind every cycle, §9–§11 what is left to the builder, the calls and the builder check, Appendix B the README research — read with S4a's close `docs/plans/2026-10-23-security-s4a-close.md` (§4 what S4a leaves, §5 the relocation map this plan completes, §8 and §9 what each part inherits, §10) and the "Settled at the brainstorm" section below. Every rule in this plan argues from one of them; line numbers are readings at master `ddb9287b` (the merge of PR #63) and are re-read at the branch's head.

## Settled at the brainstorm (2026-10-26, the owner, in session)

The owner's answers, in substance verbatim:

1. **PR #63 is merged** (`ddb9287b`, 2026-10-26, a merge commit). `security-s4b` is cut from it and pushed to the lab; it reaches origin with this plan (the design §2.10).
2. **S4b and S4c run as ONE sequence.** The owner: "if the size allows this to merge s4b and s4c into a joint sequence, we can do that." The orchestrator's sizing: the window's recipe is already written verbatim (`worker/RUNBOOK.md` §23 stage 2; the design §5), so the plan cites it rather than restating it, and the whole reads at about twelve tasks and four owner-present blocks — inside the one-agent rule (`feedback-plan-size-split-cycles`). The design's §2.13 "three cycles" is amended by the owner's word; the names S4b and S4c stay as the two PARTS of this plan, so every record that defers to "S4c" (the ledger's D17 row, the lane document's §8, S4a's close §9) stays true. The branch keeps its name, `security-s4b`.
3. **The social preview and GitHub's two e-mail settings move to the window.** The owner: "It appears that the social preview isn't showing up for a private repo, I see in my other public repos it's there, just not private, so we'll have to wait for those github items for after the switch in s4c." S4a's plan item 8 ("uploaded NOW") is superseded; the upload is C5's, read back there.
4. **The README's picture.** The owner: "for the readme showing a picture of the game in flight makes sense, with the context around it. So not a top of the repo page image since a random capture of the web module doesn't explain much of the repo's purpose, it does fit when discussing the web implementation." So: no picture at the top; one picture of a game in flight in the section about the browser client; the door stays the social preview.
5. **The whole-branch review is a Workflow, capped at eight agents.** The owner: "The workflow based review, which should be capped at no more than eight agents in the workflow overall, is the better solution for the whole branch review, although in this case … we could probably get that down to three unless we're just redoing the work for the rebuilds and cross model work which I don't think we should." For the joint sequence the orchestrator sizes it at FIVE reviewers (code arrived with Part 2: the workflows, the version bump, stage 2's commit); the clean-eyes walk and the cross-model read are NOT redone inside the review. The owner's words above are the opt-in the Workflow tool requires, recorded here.
6. **The shape and the three reviewers' lenses** the orchestrator proposed were accepted ("that works"): the README, the docs index and the method page as three tasks; the stale sentences as one small task; CLAUDE.md, guidingpath's Current state and the board as one careful task carrying the lab rule; the walk; the cross-model read as a wave; then the sequence, the review, the close, the PR.

The measured inputs (this session, at `ddb9287b`; the plans do not re-run the design's Appendix A):

- **The README** is 787 lines; S4a's close §5 maps each of its sections to its home; its lines 263–265 ("neither host's build sets it yet", contradicted by 0.7.2) retire with the rewrite and need no edit of their own. Its pictures: four CLI text renders under `docs/images/` (seven files there), three under `worker/docs/img/` (ten files there), two under `web/public/help/` (five files there). `docs/computer-player.md` already carries the results ladder as its §5 (lines 105–164, 38 figures as recorded — S4a Task 6).
- **The stale sentences** (S4a's close §4): `web/src/Help.tsx:193–195` ("behind Cloudflare Access it is your Access sign-in"); `deploy/README.md:106–107` ("The first screen asks for a name, and the name is who you are"); the board's "English rulebook" at line 110 (the one surviving site; the board is rewritten whole). S4a's wave already landed Task 5's three deferred minors (the port remedy sentence, `--stop` under a down daemon, the sign-in sentence — `ad13a1d5`, `868879f5`), so the walk starts with no known friction.
- **The workflows:** `test.yml` carries `actions/checkout@v4` and `dtolnay/rust-toolchain@master`; `web.yml` `actions/checkout@v4` ×3, `dtolnay/rust-toolchain@master` ×2, `actions/cache@v4` ×3, `actions/setup-node@v4` ×3; `docker.yml` `actions/checkout@v4`, `docker/setup-buildx-action@v4`, `docker/build-push-action@v7` ×2 — **fourteen tag pins and three `@master` pins** (the design's "seven and three" was read at `56460688`; `web.yml` has grown since). On GitHub `test.yml` and `docker.yml` read `disabled_manually`, `release.yml` and `web.yml` active. `worker/scripts/release-workflow.test.mjs` holds three cells over `release.yml` alone.
- **The version** `0.7.2` sits at `Cargo.toml:16`, `web/package.json:4`, `worker/package.json:4`; `YOKOHAMA_TURNSTILE` reads `"off"` at `worker/wrangler.jsonc:143` and `worker/wrangler.dev.jsonc:136`. The release `v0.7.2` carries five assets: the three CLI archives, `yokohama-cli-guide.html`, `SHA256SUMS`. The last NAS deployment record is `deploy/RUNBOOK.md` §3.23 (the twentieth, `yokohama-web:0.7.2-s3`).
- **The repository:** private; squash and rebase merges ALLOWED beside merge commits; `delete_branch_on_merge` false; issues on, wiki off, discussions off; the description, homepage and nine topics set (Block A of S4a); no custom social preview. Open issues: #53, #58, #59, #60, #62. Origin's heads: `master`, `docs-language-school-hotfix-handover`, `security-s4a` (merged).
- **The NAS sidecar sync (the design §2.25; the board's item 8) has never run on this Mac:** no crontab, no launch agent under `~/Library/LaunchAgents`, no mounted share — so the pre-flight ARRANGES the copy rather than reading it (Block B).
- **`docs/temp/`:** 55 top-level entries, 484 MB: eleven directories (`web-review` 370 MB, `assetreplacement` 57 MB, `security-review-2026-10-08` 31 MB, `step6-artifacts` 15 MB, `owner-games` 2.7 MB, `sdd-archive` 2.7 MB, `step10-artifacts` 2.2 MB, `webresearch` 572 KB, `2026-10-04-technology-review-evidence` 396 KB, `step11-artifacts`, `step12-artifacts`, `step13-roster`, `scratch` empty) and some forty-four files (the step-9 charters and assessments, the independent progress review, the research prompts, the engineer primer, the status deck, the Databricks notes, the refit test drive, two screenshots, the 2026-10-22 leftovers write-up).
- **The decision log:** 29 live rows, 2026-09-29 → 2026-10-24; the index names three archives; `docs/tools/guidingpath-relocate.py` relocates line ranges verbatim and proves it byte for byte.
- **The ledger's cycle columns** read `S4` at C4 (line 30), D17 (47) and D20 (50), fixed in S4a; D3's live column (33) defers the seat-aware admission's reading to the window; the lane document's §8 (lines 357–461) is the preconditions table the window fills.
- **CLAUDE.md:** the browser-lane bullet is line 85, one paragraph of about 1,900 words; the "Where things stand" paragraph lines 11–17; the lane document's §4 is 30 lines (144–173).
- **Tools on this Mac:** Docker Desktop; the Codex CLI 0.164.0; `gh` authenticated (the `user` scope absent — the e-mail settings are read in the browser).

The builder's calls (the design's §9 leaves them; each recorded here so the owner can check them):

- **C-1** One sequence (item 2 above): one branch, one plan, one review, one PR, one close; the parts keep the names S4b and S4c.
- **C-2** The results ladder lives ONCE, in `docs/computer-player.md` §5; the method page's results section names it and states no figure of its own (a figure stated twice drifts; the design §7.3 is met by reference).
- **C-3** No YAML parser (the text-structural ruling of S4a, P-14; `feedback-lean-dependencies-first-principles`): the pin cell generalizes over the `uses:` lines of all four workflows — a scalar a line regex reads without a spelling race — and `test.yml`'s `on:` block is pinned byte for byte; the structure cells (job heads, permissions, the tag job) stay `release.yml`'s. The parse-based question S4a recorded is answered: not needed.
- **C-4** The README's picture is `worker/docs/img/game-desktop-player.webp` (a game as the player sees it; an existing file in a directory `ASSETS.md` names, so S4a's notice cell stays green and no rights text moves), placed in the browser-client section; the top of the README carries no picture (item 4).
- **C-5** The new archive is `guidingpath-archive-security.md` — "the web lane's finish and the security project, 2026-09-29 → the public release" — cut at the close with `docs/tools/guidingpath-relocate.py`, holding verbatim every row older than the close's own.
- **C-6** The clean-eyes walk runs on a `git archive` export while the repository is private (a clone needs the owner's credential, which a stranger has not); the public walk (D2) is a clone. Two README lines are unprovable before the switch — the release archive's `curl` (404 without a token) and "Try it" at the hosted site (Access) — and are proven at D2 and D3.
- **C-7** The window's order: the release published first, the NAS deployed as the canary (the project's precedent, D-15), then the Worker's stage 2, then the switch.
- **C-8** The sidecar sync is a one-time copy at Block B with its recipe recorded; whether a standing job is arranged is the owner's call, asked there.
- **C-9** `security-s4a` leaves origin at A4 (the lab holds it at `2c2a9d96`; master contains it); the lab rule's sentence (Task 5) says a merged branch leaves origin once the lab holds it.
- **C-10** The window's two protections are rulesets: a tag ruleset on `refs/tags/v*` (creation, update and deletion restricted; the repository's admin role the one bypass actor, so the owner tags) and a branch ruleset on `refs/heads/master` (a pull request required, force-push and deletion blocked, NO bypass — "every commit to master arrives by a pull request, the closes included", the design §5.6).
- **C-11** The Workflow review is five reviewers (three Opus, two Sonnet), the orchestrator synthesizes (no refuters at this size), one Opus fixer lands the wave as its own commits, one Opus scoped re-review reads it.
- **C-12** `docker.yml` stays `disabled_manually` (dispatch-only by design; disabled for the owner's minutes) — recorded in the close; re-enabling it is the owner's.
- **C-13** Squash and rebase merges are turned OFF at C5 (S4a's close §9: either defeats the tag job's ancestry rule).
- **C-14** The link cell (`web/scripts/doc-links.node.test.ts`, Task 2) is the lasting catcher for G3's "every citation resolves" on the reader-facing documents; external URLs are curled once at D4, never in a cell.

## Global Constraints

- **The branch is `security-s4b`, cut from master `ddb9287b`**; every commit lands on it and the PR targets master. Stage with explicit paths (`git add <path>`), never `git add -A`. **Nothing deploys and nothing is tagged before the window** (Block C): Task 11's version commit is the bump, not the release.
- **The lab rule is in force from A1** (the design §2.10; Task 5 writes it into CLAUDE.md): `remote.pushDefault` is `lab`, so a bare `git push` goes to the lab; origin is named explicitly and receives the branch with its plan (A1) and the branch's later commits at each task's close; the lab runs no Actions.
- **Commits are authored under the GitHub no-reply address** (`59934696+jmoore2333@users.noreply.github.com`, this repository's configuration since 2026-10-23); history is not rewritten. The owner's test identity, the owner's address and any secret value appear in no committed document (the lane document's own rule; the design §4).
- **The STOPs are untouched:** `SEMANTICS_REV`, `crates/yokohama-oracle`, the freezes, the pre-registrations; no source file under `crates/yokohama-core` or `crates/yokohama-oracle` moves. **The workspace version moves** (Task 11), which recompiles every crate and re-stamps the CLI's and the layer's version strings — so the web lane's suites run at Task 11 (`docs/web-lane.md` §2; `CLAUDE.md`'s Commands) and **the fifteen-gate roster runs at the review head** (the design §5.1), one uninterrupted sequence, release binaries pre-built, machine-tagged, recorded in the close and the board's gates line.
- **The suites per edit** (`docs/web-lane.md` §2; `CLAUDE.md`'s two triggers P-20 and P-21): a change under `web/src` runs in `web/` `npm run typecheck && npm test` (and `npm run test:wasm` when `web/scripts/` moves, with `typecheck` beside it); a change under `deploy/` runs `(cd worker && npm run test:scripts)` beside `cargo test -p yokohama-web --test lane_document`; an edit to `docs/web-lane.md` runs both of those; a change under `.github/workflows/` runs `(cd worker && npm run test:scripts)`; Task 11's bump runs the whole of §2's rows for both hosts (`cargo test -p yokohama-web` with `wire_fixtures` regenerated ONCE by its step and read, `-wasm`, `-worker`; `worker/` `npm run build:module:debug` FIRST, then `npm run typecheck && npm test && npm run test:scripts`; `web/` `npm run build:wasm && npm run typecheck && npm test && npm run test:wasm`) plus `cargo test -p yokohama-cli` with the corpus present (`YOKOHAMA_BGA_CORPUS_OPTIONAL` is NOT set in this cycle) and `cargo clippy --workspace --all-targets -- -D warnings`. A documents-only task runs the link cell (`cd web && npm run test:wasm -- doc-links` once Task 2 exists) and the register grep.
- **Live documents are re-pointed; dated documents are not edited** (the design §4). The archives are verbatim; the one move this cycle makes in the decision log is the close's archive cut, verbatim and byte-checked. The relocation map in the close names every section that moved.
- **The public register** (the design's header; `feedback-public-register-no-legal-talk`): the documents state what is third-party and whose it is and stop there; no legal conclusions, no takedown sentences, no "why the owner wants it seen"; owner quotes only where one rules. The README's licence line points at `ASSETS.md`, the notice of facts, and adds no rights text of its own.
- **Figures** (the design §4): every number a page states is a recorded figure with its record named, quoted as recorded; a one-sided bound is called one-sided; "max observed under harness X", never a bound; no direction asserted where none was measured; **the bar (map §1.9) is NOT claimed** anywhere — the README's and the method page's sentence is `The bar (map §1.9) is NOT claimed: a shipped milestone is a narrower claim than the bar.` verbatim; no model or co-author table; the README states no counts.
- **Deletions and the window's switches are the orchestrator's or the owner's** (the design §4): the branch deletion on origin, the Cloudflare and GitHub settings, the deploys, the tag — in the orchestrator's own shell with the owner present where the step says so, never in a dispatch.
- **Dispatch rules (the owner's standing rules):** implementers work IN the checkout, one at a time; the orchestrator sends a working agent nothing and runs nothing in the tree while one works (bar the announced heartbeat check); every reviewer runs on an EXPORT (`git archive <sha> | tar -x -C <dir>`) with its own `target/`, with `web/node_modules`, `worker/node_modules`, `worker/gen`, `worker/assets` and `worker/.test-assets` symlinked from the checkout, the two `worker/test/*.gen.ts` COPIED, and `bridge-captures/` symlinked read-only where a task's suites read the corpus; every agent writes its full report to a transcript file and replies with the path and three lines; no agent runs `rm -rf`, `rm -r` or a bulk `rm -f`; no agent waits on an event, arms a Monitor, spawns a subagent or messages a peer; a reviewer's mutants run on its export, never in the checkout; an implementer PROBES the brief's code and commands before reporting (`feedback-sdd-implementer-vacuity-pass`).
- **Tiers:** as each task's header says (`impl: opus` for the judgment-laden documents — Tasks 1, 3, 5, 11; `impl: sonnet` for the precise briefs — Tasks 2, 4, 8, 9, 10, 12); every task reviewer `review: opus`; the walker HAIKU (the design §4: "a reader that weak is the point"); the cross-model reader the Codex plugin; the whole-branch review a Workflow of five (C-11); the pre-flight the orchestrator's, without mutants (`feedback-preflight-no-mutant-rerun`).
- **Outward-facing steps run on the owner's word in the session**, each marked **(outward)**: pushing to origin, the branch deletion, the PR and the merge, the tag, every deploy, every Cloudflare and GitHub setting, the switch itself. Items 1–6 above are the word for the branch, the merge of #63 and the shape; the rest is asked for at its step.
- **The window needs the owner present** (Blocks B, C and D are marked); its rollback is §23 stage 2's step 9 and the design §5's rollback paragraph, read before the first switch.
- **Commit messages** in the house style (a prose subject naming the change, its rule and its catcher), ending with the attribution trailer the committing session carries.

## Review Focus

Five readers or operators this cycle's documents and settings will meet, each pinned by a check in the step that owns it:

1. **A stranger on a clean Mac** follows the README's CLI quick start by the release archive: the `curl` line lands a binary that is NOT quarantined (the macOS build is ad-hoc signed; a browser download lands quarantined) and runs to the setup wizard's first prompt (Task 1 Step 2's expected screen; the walk, Task 6, proves the `cargo run` path now; D2 proves the `curl` line on the public release).
2. **A stranger with Docker** runs the browser quick start, `bash deploy/run-local.sh`, and lands on the door's three cards at `http://127.0.0.1:8080/` with no other command (Task 6, the walk, and D2 again on the public clone).
3. **A reader who follows any link in the README or the docs index** lands on the file and the heading named, anchors included (Task 2's cell, which reads every relative link and anchor of the seven reader-facing documents; D4 curls the external URLs once).
4. **A `v*` tag pushed on `security-s4b` before the merge publishes nothing, and `v0.7.3` pushed on master's merge commit publishes the five assets** (the tag job's ancestry rule, S4a's D17; the tag step below tags master only; C1 reads the assets back).
5. **A documents-only push to master after the flip runs no job, and a push that touches `crates/yokohama-core` runs the engine suite** (`test.yml`'s `push` trigger under its path filters — Task 8 pins the `on:` block; D5 reads the close's docs-only push as "no run" and reads the run on the first later core change).

---

## Block A — before Task 1 (the orchestrator)

Done at the branch cut, 2026-10-26: PR #63 merged (`ddb9287b`, a merge commit); local master fast-forwarded and pushed to the lab; `security-s4b` cut from it and pushed to the lab (`git push -u lab security-s4b`); the author address in this repository's configuration unchanged since 2026-10-23.

- [ ] **A1. The first commit:** this plan, with the board's `Updated` line and Track 2's heading brought to this session's state (PR #63 merged; the joint sequence planned; NEXT Block A then the run — the pattern of S4a's P-2, so the board on the branch is never stale). `git add docs/plans/2026-10-26-security-s4b-s4c-plan.md docs/handover.md && git commit` (the subject names the plan, the joint sequence and the brainstorm's settlements); `git push` (the lab); then **(outward)** `git push -u origin security-s4b` — the public repository receives a branch when its plan exists (the design §2.10; the owner's word, item 1).
- [ ] **A2. The SDD ledger** at `.superpowers/sdd/2026-10-26-security-s4b-s4c-plan/progress.md` (gitignored): one row per task with its tier, its base SHA, its export path, its transcript path and its verdict; the owner's answers above; `df -h /System/Volumes/Data` before the first export. S4a's SDD workspace `.superpowers/sdd/2026-10-23-security-s4a-plan/` STAYS (S4a's close cites its review record); S4a's exports under the previous session's scratchpad are the operating system's to clear.
- [ ] **A3. The before review (the pre-flight, no mutants):** an export of the head with its own `target/`; the baseline counts for the suites Tasks 4, 8 and 11 owe (the web lane's §2 rows less e2e; `cargo test -p yokohama-cli` with `bridge-captures/` symlinked; clippy) into the ledger; **the three measurements Task 11 needs** — (i) `git grep -n -E '0\.7\.2' -- . ':!docs/plans' ':!*.lock' ':!CHANGELOG*'` listing every live site that spells the current version, each classified "moves with the bump" or "a record, stays" (the runbooks' deployment records stay; a README example tag moves); (ii) in the export, `YOKOHAMA_TURNSTILE` flipped to `"on"` in `worker/wrangler.jsonc` and `worker/wrangler.dev.jsonc` and `(cd worker && npm run test:scripts)` run once: the red cells' NAMES and count (§23 stage 2 step 3 measured six on 2026-10-19 against the `access` files; the count at this head is the figure Task 11's Step 3 moves); (iii) `cargo test -p yokohama-web --test wire_fixtures` read green WITHOUT the flag at the head (so Task 11's regeneration diff is attributable to the version alone); Tasks 1–5's briefs read against the files they cite at the head, the line numbers re-read, every correction applied to this plan AT THE SOURCE and numbered P-n in the Review record.
- [ ] **A4. `security-s4a` off origin (outward; C-9):** `git ls-remote lab refs/heads/security-s4a` reads `2c2a9d96`; `git merge-base --is-ancestor 2c2a9d96 origin/master` exits 0; then `git push origin --delete security-s4a`; `git ls-remote --heads origin` reads exactly `master`, `docs-language-school-hotfix-handover` and `security-s4b`. The local branch is kept.

---

## Part 1 — S4b, the front door (the design §7)

### Task 1: `README.md` from a clean sheet · impl: opus · review: opus

The design §7.1, Appendix B and §10.15; the owner's item 4; C-4; S4a's close §5 (every section of the old README has a home — the rewrite retires them, the close's map records it). The sources an implementer reads: `docs/web-guide.md` (§2 the hosted site, §4 hosting your own — `deploy/run-local.sh`), `docs/computer-player.md` (§3 V4 production and provisional, §6 the bar), `CONTRIBUTING.md`, `SECURITY.md`, `ASSETS.md`, `deploy/README.md` §3–§4, the release `v0.7.2`'s asset names, `CLAUDE.md`'s Project section (the crates), and the old README at `ddb9287b` for what it said (not for its shape).

**Files:**
- Modify: `README.md` (replaced whole)
- Modify: `web/scripts/assets-notice.node.test.ts` — its anti-vacuity floor re-measured (P-5: a one-picture README leaves the four documents naming two picture directories, not three)
- Not touched: `ASSETS.md`, `NOTICE`, `CONTRIBUTING.md`, `SECURITY.md`, the dated documents

**Interfaces:**
- Produces: the README's sections, in this order, under about 200 lines: **the first line** — what it is and who it is for, in the plain nouns a search would match (a rules engine, a CLI game tracker and a browser client for the board game Yokohama, with a computer player built to play to win), no logo, no badge wall and no table of contents above it; **three or four badges** on one line under it, each linking live data — the latest release (`https://img.shields.io/github/v/release/jmoore2333/yokohama-assistant` → the Releases page), the licence (→ `LICENSE`), the test workflow (`https://github.com/jmoore2333/yokohama-assistant/actions/workflows/test.yml/badge.svg?branch=master` → the workflow's runs; it renders "no status" until the first run after the flip, which the close records) — nothing decorative; **Try it** — the hosted site `https://yokohama.moore.nyc`, where the bot plays in the browser with no account, one sentence; **Quick start** — the CLI two ways (the release archive: one `curl -L` line for the Apple-silicon archive from the Releases page's `latest/download/` path, `tar xzf`, run; the guide `yokohama-cli-guide.html` beside the binary; and `cargo run -p yokohama-cli` from a clone, with Rust via rustup) with its expected first screen quoted (the wizard's first prompt, as the implementer sees it); the browser client (one command, `bash deploy/run-local.sh`, with its expected last line "open http://127.0.0.1:8080/" and the door's three cards) and the sentence that a browser download of the macOS archive lands quarantined because the binary is ad-hoc signed, so fetch with `curl` or build with `cargo`; the test line `cargo test -p yokohama-core` and, for a clone without the two gitignored captures, `YOKOHAMA_BGA_CORPUS_OPTIONAL=1` for the CLI suite; **The browser client** — three or four sentences with THE ONE PICTURE (C-4: `worker/docs/img/game-desktop-player.webp`, its alt text describing the game in flight) and the link to `docs/web-guide.md`; **What is inside** — the crates in five lines (the engine; the CLI; the oracle and the difftest harness; the web layer, its wasm and the Worker host with the client; the data generator and the bridge — the bridge in one clause as "a read-only capture of the owner's own Board Game Arena sessions, translated by the CLI's `import`"); **Status, honestly** — the engine complete and differentially verified at `SEMANTICS_REV` 7; `v4-step9-draft1` the shipped bot, production and provisional; the bar sentence verbatim; next, V4's vetting and V5; **The record** — `guidingpath.md` (the authority; the decision log), `docs/README.md` (the index), `docs/method.md`, `docs/handover.md` (the board); **Contributing · Security · Licence** — one line each: `CONTRIBUTING.md`; `SECURITY.md`; Apache-2.0 on the code (`LICENSE`, `NOTICE`), the game's components are the rights holder's and `ASSETS.md` names them, not affiliated with or endorsed by the game's publishers. No status narrative, no counts, no roadmap framed as features, no owner quote.

- [ ] **Step 1: Write the README** as the Interfaces say, running each quick-start command once on this Mac to quote its first screen (`cargo run -p yokohama-cli` to the wizard's first prompt, then quit; `bash deploy/run-local.sh --dry-run` for the script's lines — the real build is the walk's, Task 6). The release archive line names the current asset (`yokohama-cli-v0.7.2-macos-apple-silicon.tar.gz` through the `releases/latest/download/` path) and says the newest build is always the one on the Releases page.

- [ ] **Step 2: Verify**

Run: `wc -l README.md && grep -c 'The bar (map §1.9) is NOT claimed: a shipped milestone is a narrower claim than the bar.' README.md && grep -n -o '](\([^)]*\))' README.md | grep -v 'http' | sed 's/.*](//; s/)$//; s/#.*//' | sort -u | while read -r p; do [ -e "$p" ] || echo "MISSING $p"; done; grep -n -i -w -E 'copyright|infring|takedown|dmca|fair use' README.md; git grep -n -c -E '[0-9]{3,}' -- README.md`
Expected: under 200 lines; `1`; no `MISSING` line but `docs/README.md` and `docs/method.md`, which Tasks 2 and 3 create next (P-2); the register grep prints nothing; the last count is small and every three-digit-or-more number is a version, a port, a year or a `SEMANTICS_REV` value (the implementer lists them in the transcript).

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: the README from a clean sheet — what it is in the first line, the badges, Try it, the quick start for the CLI and the browser client with their first screens, the browser client with its one picture, what is inside, status with the bar not claimed, the record, contributing, security and licence (S4b Task 1; the design §7.1)"
```

Checks for the reviewer (Review Focus 1 and 3): every command in the quick start run on the export as written (the `curl` line read as 404 on the private repository and recorded, not failed — C-6); every relative link resolves; the picture is in a directory `ASSETS.md` names (`cd web && npm run test:wasm -- assets-notice` green); the bar sentence verbatim; no figure but versions and the rev; no sentence claims the hosted site is open before the switch (the README is on master before the window, unread by strangers, so it carries no Access caveat — and none is a false sentence at the flip).

---

### Task 2: `docs/README.md`, the docs index, and the link cell · impl: sonnet · review: opus

The design §7.2 and §10.15; C-14. The index is one line per document a reader may need, grouped; the cell holds every reader-facing document's relative links and anchors to the tree.

**Files:**
- Create: `docs/README.md`
- Create: `web/scripts/doc-links.node.test.ts` (under `npm run test:wasm`, which includes `scripts/**/*.node.test.ts`; `tsconfig.node.json` types it, P-21)
- Not touched: the documents the index names

**Interfaces:**
- Produces: the index's groups, in order: **the guides** (`docs/cli-guide.md` with its `.html` twin — the CLI; `docs/web-guide.md` — the browser client; `docs/computer-player.md` — the bot; `docs/bridge-setup.md` — the BGA bridge, "a read-only capture of the owner's own Board Game Arena sessions, translated by the CLI's `import`"; `docs/forge-cli-testing.md` — visual CLI verification); **the authority** (`guidingpath.md` and the three archives `guidingpath-archive.md`, `guidingpath-archive-phase3.md`, `guidingpath-archive-web.md` with their date spans; the decision log's rule: append-only, the archives append-closed, archived rows authoritative where cited); **the board** (`docs/handover.md`, a pointer that never outranks the authority); **the method** (`docs/method.md`, Task 3); **the lane records** (`docs/web-lane.md`; `docs/security-ledger.md`); **the hosts** (`deploy/README.md` and `deploy/RUNBOOK.md` — the self-hosted binary and image; `worker/README.md` and `worker/RUNBOOK.md` — the Cloudflare host); **the plans directory** explained in three sentences (dated, frozen, corrections recorded forward; how to read a cycle: the spec, the plan with its Review record, the close as the record of record; the map `docs/plans/2026-07-25-phase3-map.md` as Phase 3's governing document); **the knowledge base** (`analysis-1/yokohama-kb/00-README.md`); **the tools** (`docs/tools/` — one line each for `guidingpath-relocate.py`, `mkguide.py`, `mkcardart.py`, `mkhelpart.py`, `tilerects.py`, `tts-extraction/`); **the community files** (`CONTRIBUTING.md`, `SECURITY.md`, `ASSETS.md`). Under about 80 lines.
- Produces: the cell `every relative link and anchor in the reader-facing documents resolves`: over the SIX files `README.md`, `docs/README.md`, `docs/web-guide.md`, `docs/computer-player.md`, `CONTRIBUTING.md`, `SECURITY.md` at this task (a file absent at the cell's run is a failure, so `docs/method.md` joins the list in Task 3 — P-1 — and the cell reads seven from then; no red cell lands on the branch), it collects every markdown reference `](<target>)` and every `<a href="<target>">` whose target carries no URL scheme and does not start with `//` or `#`-only; resolves a target from the referencing file's directory (a leading `/` from the repository root); asserts the file exists; for a `#anchor`, asserts the target file holds a heading whose GitHub slug equals the anchor (lowercase; spaces to `-`; every character that is not a letter, a digit, `-` or `_` removed; a repeated slug suffixed `-1`, `-2`, …); and asserts it read at least 40 links in all (anti-vacuity).

- [ ] **Step 1: Write the failing cell** as the Interfaces describe; run `cd web && npm run typecheck && npm run test:wasm -- doc-links`; expected: RED on `docs/README.md` absent (and `docs/method.md`); the implementer also proves it on a scratch copy with one anchor misspelled and one path broken, and says so in the transcript.
- [ ] **Step 2: Write the index** as the Interfaces say; every path it names is checked to exist by the cell.
- [ ] **Step 3: Verify**

Run: `cd web && npm run typecheck && npm run test:wasm -- doc-links; wc -l ../docs/README.md; git -C .. ls-files 'docs/*.md' 'docs/*.html' | while read -r f; do grep -q -F "$(basename "$f")" ../docs/README.md || echo "UNLISTED $f"; done`
Expected: the cell GREEN over the six files (P-1; Task 3 adds the seventh); the index under 80 lines; no `UNLISTED` line but `docs/suggest-walkthrough-2026-07-27.html` (a dated artifact; listed under the plans sentence or deliberately left — the implementer says which).

- [ ] **Step 4: Commit**

```bash
git add docs/README.md web/scripts/doc-links.node.test.ts
git commit -m "docs: the docs index — one line per document a reader may need, grouped, with a cell that holds every relative link and anchor of the reader-facing documents to the tree (S4b Task 2; the design §7.2)"
```

Mutants for the reviewer (Review Focus 3): a link's path broken (red); an anchor off by one character (red); a `<a href>` broken (red); a link under a directory that exists but a file that does not (red); the anti-vacuity floor raised above the count (red, proving the count is real).

---

### Task 3: `docs/method.md` — "How the work is organized" · impl: opus · review: opus

The design §7.3; C-2; the figures rule. The sources: `CLAUDE.md`'s Commands (the per-edit suites, the fifteen gates, the two-tier policy — quoted by command, never by count), `CONTRIBUTING.md` (how a change lands), the S4a close §1 and §7 (the shape of a task's review and a whole-branch review), the design §4 (the rules of evidence), `docs/plans/2026-07-25-phase3-map.md` §1.9 and §5, `docs/computer-player.md` §5 and §6.

**Files:**
- Create: `docs/method.md`
- Modify: `web/scripts/doc-links.node.test.ts` (the file list gains `docs/method.md` if Task 2 listed it ahead; otherwise nothing)

**Interfaces:**
- Produces: the page's sections, in order: **1 The workflow** (brainstorm → spec → plan → execute; dated artifacts frozen, corrections recorded forward; TDD, the failing test first; the per-edit suites and the lint gate after every task; the fifteen-gate release roster before a merge, measured as one sequence and recorded machine-tagged); **2 How a cycle lands** (a branch, a plan whose Review record carries every correction the run meets, implementers one at a time in the checkout, a fresh reviewer per task on an export with mutants, a whole-branch review, a fix wave, a scoped re-review, the pull request, the close as the record of record); **3 The decision log and the `SEMANTICS_REV` contract** (append-only, the archives append-closed; the rev bumped in the same commit as any behaviour-affecting change; the oracle authored from rules sources only); **4 Pre-registration and the rules of evidence** (a strength claim is registered before its field runs; every figure quoted as recorded with its record named; a one-sided bound called one-sided; "max observed under harness X"; no direction asserted where none was measured; the bar sentence verbatim); **5 Results** — one paragraph that NAMES `docs/computer-player.md` §5 as the results ladder and states no figure of its own (C-2), and the end result as it stands: the live site, the suites and the gates by their commands; **6 Reviews across models** — one sentence that reviews run across models where the record says so, and nothing more (no model table). Under about 120 lines.

- [ ] **Step 1: Write the page** as the Interfaces say.
- [ ] **Step 2: Verify**

Run: `wc -l docs/method.md && grep -c 'The bar (map §1.9) is NOT claimed: a shipped milestone is a narrower claim than the bar.' docs/method.md && grep -n -E '\b0\.[0-9]{2,}\b|\b[0-9]{2,} ?(s|games|units)\b' docs/method.md; cd web && npm run typecheck && npm run test:wasm -- doc-links`
Expected: under 120 lines; `1`; the figure grep prints nothing (no interval, no wall, no game count — the page carries none); the link cell green over seven files.

- [ ] **Step 3: Commit**

```bash
git add docs/method.md web/scripts/doc-links.node.test.ts
git commit -m "docs: the method page — the workflow, how a cycle lands, the decision log and the SEMANTICS_REV contract, pre-registration and the rules of evidence, results by reference to the computer player's ladder, reviews across models in one sentence (S4b Task 3; the design §7.3)"
```

Checks for the reviewer: every command the page names exists in `CLAUDE.md`'s Commands verbatim; no figure; no model table; the register grep; the page names no owner quote.

---

### Task 4: the stale sentences · impl: sonnet · review: opus

S4a's close §4 (the two sites that survive Task 1); the design §7.1's third item. The browser help panel and the self-host README each describe the first screen as it was before security S2b and the auth cycle.

**Files:**
- Modify: `web/src/Help.tsx:193–195` — the clause "on a server with no sign-in of its own you choose a name once and this browser remembers it; behind Cloudflare Access it is your Access sign-in" → the public day's truth, mirroring `docs/web-guide.md` §2's sign-in sentence (the hosted site signs a player in by an e-mail code or a provider; a self-hosted server with no sign-in of its own asks for a plain name once, which this browser remembers; a self-hosted server behind Cloudflare Access takes its sign-in). The implementer reads the guide's sentence first and keeps the panel's register.
- Modify: `deploy/README.md:106–107` — "The first screen asks for a name, and the name is who you are" → the door's three cards; the name is asked when **Play with friends** leads to the lobby (the open door's name step), and the name is who you are there.
- Test: `web/src/Help.test.tsx` (if it pins the sentence, the pin moves with it — test-first).

- [ ] **Step 1: Read `web/src/Help.test.tsx` for a pin on the sentence**; if one exists, move it to the new words first and run `cd web && npm test -- Help` to see it RED.
- [ ] **Step 2: Edit the two sites** as the Files block says.
- [ ] **Step 3: Verify**

Run: `cd web && npm run typecheck && npm test -- Help && cd ../worker && npm run test:scripts && cd .. && cargo test -p yokohama-web --test lane_document && git grep -n 'Access sign-in\|asks for a name' -- web/src deploy/README.md`
Expected: green at the baselines (A3's counts; `test:scripts` reads `deploy/*.sh`, which this task does not touch — P-20's rule is run, not assumed); the grep prints nothing.

- [ ] **Step 4: Commit**

```bash
git add web/src/Help.tsx web/src/Help.test.tsx deploy/README.md
git commit -m "web, deploy: the help panel's sign-in sentence and the self-host README's first screen say what the door does since the auth cycle — three cards, the name asked at the lobby's step, the hosted site's sign-in by code or provider (S4b Task 4; S4a's close §4)"
```

Checks for the reviewer: the panel's new sentence and the guide's §2 sentence agree in substance; `deploy/README.md`'s §2 still describes the open door correctly for a self-hoster (a name is a label, never a credential); no other stale "first screen" sentence in the two files (`grep -n 'first screen'`).

---

### Task 5: the agent-facing documents — `CLAUDE.md`, `guidingpath.md`'s Current state and roadmap, `docs/handover.md`, the lab rule · impl: opus · review: opus

The design §7.4–§7.6 and §2.10; C-9. The hardest brief of Part 1: a cut that loses no standing rule, a Current state one screen long, a board the session after the flip opens from.

**Files:**
- Modify: `CLAUDE.md` — (a) the browser-lane bullet (line 85) cut back to what it says it is: the pointer ("the layer ROUTES, SCOPES and SERIALIZES and computes nothing about the game"), the link to `docs/web-lane.md` §4, and the three facts that cross into the AI lane (the `bots.rs` match with no `_` arm; the `wire_fixtures` regeneration and the release-version rule; the `GRANDFATHERED_NAMED_IDENTITIES` append) — every other sentence of the bullet (the seed policy, the echo and the error cap, the header set and the CSP, B10's rule, the Worker's door and its rate limits and caps, the identity sources and the session cookie) MOVES to `docs/web-lane.md` §4 where it is not already there, verbatim; (b) the "Where things stand" paragraph (11–17) re-pointed at the AI lane: the engine complete; the focus the computer player (3.5c open; V4 provisional; the bar not claimed); the STOPs; the web lane complete and the security project CLOSED with the public release (this cycle, named by its close once it exists — the sentence is written for the state after the window and reads "closing" until the close's commit flips it, P-n records the flip); (c) one new standing rule under Worktree hygiene: **the lab rule** — "Work in progress lives on the lab remote (`lab`, `remote.pushDefault`), which runs no Actions; a branch reaches origin with its plan and at each task's close; a merged branch leaves origin once the lab holds it; raw work in progress never reaches origin." Standing rules only; no narrative; the licensing sentence (S4a) unchanged.
- Modify: `docs/web-lane.md` §4 (144–173) — receives the moved sentences, grouped under the three existing rule headings or one new "the security era's standing sentences" paragraph; nothing duplicated (a sentence already present stays once).
- Modify: `guidingpath.md` — the Current state (23–39) rewritten, one screen, for the AI lane's re-entry: the engine; the computer player (V4 shipped and provisional, the bar not claimed, Track 1's vetting next, V5 after); the web lane complete, both hosts (their versions at the time of writing; the close updates); the security project (its cycles landed; the public release this cycle; the window pending — the close flips it); the STOPs; the machines. The roadmap table (41–71): the web-lane row current; a new last row `*The security project and the public release* — NOT a phase` with its state and its records (the ledger; the S4 design; the closes); the Phase-3 rows unchanged. **The decision log is NOT cut here** (the design §7.5; the cut is the close's).
- Modify: `docs/handover.md` — rewritten whole for the session after the flip, in this order: the header's "What this is" paragraph kept; `Updated:` one entry (this cycle in flight, the window pending; the close rewrites it); **Gates at HEAD** — the latest reading (S4a's one sequence at `21033119` and the 2026-10-19 roster figure, 497 s on the Air M5) with the line "the roster's reading from this cycle's review head once it exists"; **Track 1 — V4's close-out** first, as it stands (the three games, the ledger artifact, the next steps 1–4, the V5 line, "You decide"); **Track 2 — the Language School hotfix** (issue #62; after the V4 vetting, before V5); **Track 3 — the art lane** (local, on `art-replacement-spec` in its worktree, by its rule); **Track 4 — the web lane, complete** (the versions live; the guides); **Track 5 — the security project, closing with the public release** (this plan; the window's steps as NEXT; after the close: a pointer to the ledger and the close); **Owner rulings outstanding** — the open ones carried over (1–9 and 13 as they stand; 10, 11 and 12 are CLOSED and leave the list, their records in the ledger and the decision log); **Entry artifacts by lane** — as now, plus `docs/README.md` and `docs/method.md` under "Everything". The three generic "English rulebook" sites read "the OKAZU Brand English rulebook (16 pages)" or "the Tasty Minstrel Games English edition (20 pages)" as each means.

- [ ] **Step 1: The CLAUDE.md cut, sentence by sentence.** Before editing, the implementer writes to the transcript a numbered list of every sentence in the browser-lane bullet and its destination: KEPT (the pointer and the three facts), MOVED to `docs/web-lane.md` §4 (verbatim), or ALREADY THERE (quoting the §4 or §15 line that carries it). Then the edit, in the listed order.
- [ ] **Step 2: `guidingpath.md`'s Current state and roadmap row**, then **the board**, as the Files block says.
- [ ] **Step 3: Verify**

Run: `cargo test -p yokohama-web --test lane_document && (cd worker && npm run test:scripts) && (cd web && npm run typecheck && npm run test:wasm -- doc-links) && wc -l CLAUDE.md docs/handover.md && awk '/^## Current state/{f=1} f&&/^## Roadmap/{exit} f' guidingpath.md | wc -l && git grep -n 'English rulebook' -- docs/handover.md && git grep -n -c 'remote.pushDefault' -- CLAUDE.md`
Expected: the cells green at their baselines; CLAUDE.md shorter than at `ddb9287b` by the moved sentences (the implementer states both counts); the Current state under 40 lines; the generic phrase absent from the board; the lab rule present once.

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md docs/web-lane.md guidingpath.md docs/handover.md
git commit -m "docs: the agent-facing documents re-pointed at the AI lane — CLAUDE.md's browser-lane bullet cut to the pointer and the three facts with the security era's sentences moved verbatim into the lane document's §4, Where things stand at the computer player, the lab rule as a standing rule; guidingpath's Current state one screen and a roadmap row for the public release; the board rewritten for the session after the flip (S4b Task 5; the design §7.4–§7.6)"
```

Checks for the reviewer (the catcher for "nothing lost"): the reviewer takes the browser-lane bullet at `ddb9287b`, splits it into sentences, and for each finds it in CLAUDE.md or in `docs/web-lane.md` at the head — a sentence found nowhere is Important; a rule re-worded in the move is Important (verbatim was the brief); every SHA, number and state in the board and the Current state read against the tree, the ledger and the issue tracker; the board names no closed ruling as open; the Current state claims nothing about the window before it happens.

---

### Task 6: the clean-eyes walk · the orchestrator dispatches a Haiku walker; fixes are the orchestrator's commits

The design §7.7 and §2.15; C-6. A reader who knows nothing but the README follows its quick start on this machine and reports every point of friction in its own words; the orchestrator fixes the README or the script and the walk runs again until it is clean.

**Files:**
- Modify (as the walk finds): `README.md`, `deploy/run-local.sh`, `docs/web-guide.md`
- Not touched: the engine, the CLI, the web layer

- [ ] **Step 1: The export.** `git archive HEAD | tar -x -C <scratchpad>/walk-1/` into an EMPTY directory of the session scratchpad (no `node_modules`, no `target/`, no symlinks — a stranger's tree); Docker Desktop running (`docker info` answers).
- [ ] **Step 2: The walker's brief** (Haiku; written to the SDD workspace and copied verbatim into the dispatch): "You are a developer who has never seen this repository. Your working directory is `<path>`. Read `README.md` and nothing else unless the README sends you there. Follow its quick start for the CLI by the `cargo run` path, and for the browser client, on this machine (Docker is installed). Report every point of friction in your own words — every command you ran verbatim, every output that surprised you, every sentence you had to read twice — to the file `<transcript path>`, then reply with the path and three lines. Do not fix anything. Do not run `rm -rf`, `rm -r` or a bulk `rm -f`. Do not wait on events or arm a monitor; poll a long command's output in the foreground. The release archive's download and the hosted site may be unreachable from here; say so and go on." The walker's wall is bounded: the Docker build took 117 s on this Mac in S4a, the engine's debug build a few minutes.
- [ ] **Step 3: Triage.** Each point of friction is one of: the README's fault (a sentence or a command — fixed as a commit on the branch); the script's fault (`deploy/run-local.sh` — fixed test-first where a cell covers it, then `(cd worker && npm run test:scripts)` and `cargo test -p yokohama-web --test lane_document`, P-20); expected before the switch (the `curl` 404, Access at the hosted site — recorded for D2 and D3); or the walker's own (recorded). Commit: `docs: the README (or deploy: the script) after the clean-eyes walk N — <what the walker met>`.
- [ ] **Step 4: Again** on a fresh export of the new head, until a walk reports no friction of the first two kinds. Bound: three walks; a fourth is a finding for the owner, not a fourth dispatch. The final transcript's path goes in the close.

Checks for the reviewer (the whole-branch review reads this task): the final walk's transcript shows the CLI reaching the wizard's first prompt and the browser client reaching the door's three cards with no command the README does not give (Review Focus 1 and 2).

---

### Task 7: the cross-model read · the Codex plugin (`codex:rescue`), triaged as a wave

The design §7.8. A model from another family reads the five reader-facing pages as the stranger of §1 and reports where they are unclear, where they overclaim, and where a command would not run.

- [ ] **Step 1: The dispatch** (the Codex rescue agent, read-only, on an export of the head): read `README.md`, `docs/README.md`, `docs/method.md`, `docs/web-guide.md`, `docs/computer-player.md` as a developer arriving from a search result; report, to a transcript file, three lists — unclear (the sentence and why), overclaiming (the sentence and the claim it makes beyond what the page's own evidence supports), would-not-run (the command and the reason) — with no fixes and no new text.
- [ ] **Step 2: Triage as a review wave.** Each finding is FIX (a sentence or a command — an Opus fixer lands the wave as its own commit; an overclaim about strength is checked against the record the sentence should have named — the figures rule), RECORD (true but out of this cycle's files — the close's "what the cycle leaves"), or REFUTE (the reader misread; the refutation quotes the page). The fixer's commit: `docs: the reader-facing pages after the cross-model read — <the findings fixed>`. Then the link cell and the register grep over the touched files.
- [ ] **Step 3: The record.** The transcript's path, the three lists' counts and each finding's disposition go in the SDD ledger now and the close later.

---

## Part 2 — S4c, the flip (the design §8)

### Task 8: the three workflows — `test.yml`'s push trigger, every pin a commit SHA, and the cells · impl: sonnet · review: opus

The design §2.19 and S4a's close §9; C-3, C-12. `release.yml` was pinned in S4a (D17); the other three carry fourteen tag pins and three `@master` pins at this head. `test.yml` runs again from the flip with a `push` trigger on master under its path filters; its comment is corrected forward.

**Files:**
- Modify: `.github/workflows/test.yml` (the `on:` block gains `push: branches: [master]` with the SAME `paths:` list as `pull_request`; the comment's "There is no `push` trigger" paragraph rewritten: a push to master runs the engine suite under the path filters from the public switch, so master carries a live reading; a docs-only push runs nothing; the lab runs no Actions; a pull request's run and its merge's run are two runs by design on a public repository, where Actions minutes are not metered; the two `uses:` lines pinned)
- Modify: `.github/workflows/web.yml` (eleven `uses:` lines pinned), `.github/workflows/docker.yml` (four pinned)
- Modify: `worker/scripts/release-workflow.test.mjs` (the pin cell generalized; the trigger cell added)

**Interfaces:**
- Produces: every `uses:` in the four workflows reads `<owner>/<repo>@<40-hex SHA> # <version or branch-at-date>`, resolved with `gh api repos/<owner>/<repo>/git/ref/tags/<tag>` (a lightweight tag's `object.sha`; an annotated tag's dereferenced commit via `git/tags/<sha>`) to the LATEST patch release of the major each `@vN` named on the day, and `dtolnay/rust-toolchain@master` to the SHA `release.yml` already pins (`b725eda525362e7cf34af9a7243cac1e3f01a7ec`, "the stable branch at 2026-10-16"); the tag → SHA table in the transcript; for every pinned `actions/*` release the `runs.using` value of its `action.yml` read and recorded (`node20` expected; a `node24` requirement is a finding, not a silent upgrade).
- Produces: the cell `every workflow pins every action to a commit SHA with its version beside it` — over `.github/workflows/*.yml` (all four, listed by name so a fifth workflow is a deliberate edit to the cell), every line matching `uses:` carries `@[0-9a-f]{40}` followed by ` # ` and a non-empty comment; a `uses: ./` local action is permitted; the count of `uses:` lines asserted (`release.yml` 12, `web.yml` 11, `docker.yml` 4, `test.yml` 2 at this head — the figures A3 re-reads). The existing release-only pin cell is replaced by this one.
- Produces: the cell `test.yml runs on a master push and a pull request under one path list` — the `on:` block pinned BYTE FOR BYTE (the text from `on:` to the line before the first comment), so a trigger drift through a reviewed merge is red; and the two `paths:` lists asserted equal.

- [ ] **Step 1: Write the two failing cells** in `worker/scripts/release-workflow.test.mjs` (a new `describe("the four workflows")`), run `cd worker && npm run test:scripts -- release-workflow`; expected: both RED at the head (unpinned lines; no `push:`).
- [ ] **Step 2: Resolve and pin**, as the Interfaces say; `test.yml`'s trigger and comment.
- [ ] **Step 3: Verify**

Run: `cd worker && npm run test:scripts && cd .. && grep -n 'uses:' .github/workflows/*.yml | grep -v -E '@[0-9a-f]{40} # ' ; ruby -ryaml -e 'Dir[".github/workflows/*.yml"].each { |f| YAML.load_file(f) }' && echo parse-ok`
Expected: `test:scripts` green at the baseline + 2 − 1 (one cell replaced by the generalized one, one added); the grep prints nothing; `parse-ok`.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/test.yml .github/workflows/web.yml .github/workflows/docker.yml worker/scripts/release-workflow.test.mjs
git commit -m "ci: every action in the four workflows pinned to a commit SHA with its version beside it, test.yml runs on a master push under its path filters from the switch with its comment corrected forward, two cells hold both (S4c Task 8; the design §2.19; S4a's close §9)"
```

Mutants for the reviewer: one pin reverted to a tag (red); a pin's comment removed (red); a `uses:` line added unpinned to `docker.yml` (red); `push:` removed from `test.yml` (red); one path added to `push.paths` only (red); the reviewer re-resolves three of the tag → SHA pairs with `gh api` and reads three `action.yml` runtimes.

---

### Task 9: the `docs/temp/` triage table — the draft · impl: sonnet (read-only; the output is a scratch file) · the owner marks at Block B

The design §8.1 and §2.4; the measured inventory above. The table is the owner's instrument; the default for every item is PRIVATE.

**Files:**
- Create (gitignored, the SDD workspace): `.superpowers/sdd/2026-10-26-security-s4b-s4c-plan/temp-triage.md`
- Not touched: anything under `docs/temp/` (read only; nothing moved, nothing deleted)

**Interfaces:**
- Produces: one row per top-level entry of `docs/temp/` (55 at this head): the name, the kind (file or directory), the size, the date, one sentence of what it is (from the file's own first lines or the directory's README or listing), and the recommendation — `private` for every row unless the sentence reads as a finished, dated record a reader of the plans would want and whose publication adds no person's handle, no secret value and no rights text (the implementer marks such a row `consider` with the reason, never `publish`); the owner's column empty.

- [ ] **Step 1: Write the table** by reading each entry's head (`head -20` for a file; `ls` and any README for a directory); no file is opened beyond what the sentence needs; nothing under `docs/temp/` is written.
- [ ] **Step 2: Verify**

Run: `ls -1A docs/temp | grep -v -x '.DS_Store' | wc -l; grep -c '^| ' .superpowers/sdd/2026-10-26-security-s4b-s4c-plan/temp-triage.md; git status --short | wc -l`
Expected: the row count equals the entry count plus the header's two lines; `git status` reads `0` (nothing in the tree moved).

No commit (the table is not in the tree; the close carries its final form with the marks).

---

### Task 10: the ledger's and the lane document's pre-window records · impl: sonnet · review: opus

The design §8.2 (the parts that are true before the window) and §11's answered questions (f) and (g); S4a's close §4. The live columns, the lane's §1 "where it stands" after stage 2 and the security project's closing row are the close's (post-window), not this task's.

**Files:**
- Modify: `docs/security-ledger.md` — the cycle column `S4` → `S4a` at C4 (line 30), D17 (47) and D20 (50) (fixed there); every other sentence that defers to "S4" read and either left (a reading the window owes — D3's live column, the AUTH row's) or re-recorded as answered (the implementer lists each in the transcript); the `Updated` line gains this cycle's pre-window entry in its existing shape, the previous entry demoted to "Before it".
- Modify: `docs/web-lane.md` §8 (357–461) — the precondition "a fresh-history repository" (wherever its row or sentence sits) marked superseded by the design §2.1 and §2.3 (the history is kept whole; the art crops stay in the tree, by the owner's ruling); the per-person ceiling sentence (R-3) re-recorded with the C1/D3 answer (writes are keyed by verified address, so per person is per address; the anonymous archive post stays by design); the sentence "rows 108 and 119 read with a second identity" pointed at this plan's Block D as the reading's site; nothing else in §8 moves before the window.

- [ ] **Step 1: Edit** as the Files block says.
- [ ] **Step 2: Verify**

Run: `cargo test -p yokohama-web --test lane_document && (cd worker && npm run test:scripts) && git grep -n -E '\| S4 \|' -- docs/security-ledger.md; git grep -n 'fresh-history' -- docs/web-lane.md`
Expected: the cells green at their baselines; the first grep prints nothing (no cycle column reads a bare `S4`); the second prints the superseded sentence with its "superseded" clause.

- [ ] **Step 3: Commit**

```bash
git add docs/security-ledger.md docs/web-lane.md
git commit -m "docs: the ledger's three S4a rows say S4a, every other S4 deferral read and re-recorded or left for the window, the lane's fresh-history precondition superseded by the kept history and the per-person ceiling re-recorded per address (S4c Task 10; the design §8.2)"
```

Checks for the reviewer: every re-recorded sentence cites the ruling it rests on; no live column is filled before the window; the lane's §13 and §16 cells still green (the file was edited).

---

### Task 11: the version `0.7.3` and stage 2's commit · impl: opus · review: opus

The design §8.3 and §2.23; `docs/web-lane.md` §7's bump steps 1–5 (6–8 are the window's); `worker/RUNBOOK.md` §23 stage 2 step 3; A3's three measurements. One release, both hosts; the bump is this commit, the release is the window's tag.

**Files:**
- Modify: `Cargo.toml:16` (`version = "0.7.3"`), `Cargo.lock` (by `cargo update --workspace --offline` — the members' lines and nothing else), `web/package.json` + `web/package-lock.json` and `worker/package.json` + `worker/package-lock.json` (by `npm version 0.7.3 --no-git-tag-version --ignore-scripts` in each), `deploy/Dockerfile.runtime` (the example tag), `README.md`'s release-archive name in the quick start (P-6: `yokohama-cli-v0.7.3-…` under the `releases/latest/download/` path, which answers 404 for a stale name once `0.7.3` is the latest release), every live site A3 (i) classified "moves with the bump"
- Modify: the wire fixtures — `web/src/wire/*.json` as `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` and `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --lib backup` rewrite them (health's `server_version`, `archive-walk.json`'s records, the backup envelope's `app_version` — trap 30)
- Modify: `worker/wrangler.jsonc:143` and `worker/wrangler.dev.jsonc:136` — `"YOKOHAMA_TURNSTILE": "on"` — with the moving cells A3 (ii) named (the deploy guard's and the variables' cells under `npm run test:scripts`), each moved to the stage-2 shape test-first

**Interfaces:**
- Produces: the version `0.7.3` in the three version files and the fixtures; `YOKOHAMA_TURNSTILE` `on` in the production and dev files (the open file already `on`; the private, test and open-test files `off`); the deploy guard REFUSES a keyless production build from this commit on (`worker/RUNBOOK.md` §23: "the keyed build, `on` and the rule arrive together at stage 2") — which is why nothing deploys from this branch before the window.

- [ ] **Step 1: The bump, §7's steps 1–4 in order**; then `git diff --stat` read: the version files, the two locks' root lines, the fixtures, the Dockerfile's example tag, A3's listed sites — and nothing else.
- [ ] **Step 2: The fixtures' diff read line by line** (`git diff web/src/wire/`): every changed line is the version string and nothing else; a changed line that is not the version is a STOP (the reviewer reads the same diff).
- [ ] **Step 3: Stage 2's shape, test-first:** flip the two files, run `cd worker && npm run test:scripts -- deploy-guard` (and the variables' cells) and see the A3 (ii) cells RED by name; move each cell to the stage-2 shape (the production and dev files pass the guard over a KEYED tree and are refused over a keyless one; "declare `YOKOHAMA_TURNSTILE` off in … and on in …" at the new counts); green.
- [ ] **Step 4: Verify, the whole web lane** (§7 step 5 first)

Run: `cd web && npm run build:wasm && cd ../worker && npm run build:module:debug && cd .. && cargo test -p yokohama-web && cargo test -p yokohama-wasm && cargo test -p yokohama-worker && cargo test -p yokohama-cli && cargo clippy --workspace --all-targets -- -D warnings && (cd worker && npm run typecheck && npm test && npm run test:scripts) && (cd web && npm run typecheck && npm test && npm run test:wasm) && git grep -n -E '0\.7\.2' -- . ':!docs/plans' ':!*.lock' | grep -v -F -f .superpowers/sdd/2026-10-26-security-s4b-s4c-plan/version-sites-that-stay.txt`
Expected: every suite green at A3's baseline (the moved cells counted); `wire_fixtures` green WITHOUT the flag; the CLI's 863 + 2 ignored with the corpus; the last grep prints nothing (the exclusion file is A3 (i)'s "a record, stays" list, one `path:line` per line, written to the SDD workspace at the pre-flight).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock web/package.json web/package-lock.json worker/package.json worker/package-lock.json web/src/wire deploy/Dockerfile.runtime worker/wrangler.jsonc worker/wrangler.dev.jsonc worker/scripts/<the moved cells' files> <A3's sites>
git commit -m "release: the version 0.7.3 on both hosts — the workspace, the two npm packages, the wire fixtures and the backup envelope re-stamped, the runtime Dockerfile's example tag; stage 2's shape committed with its moving cells: YOKOHAMA_TURNSTILE on in the production and dev Worker files, so the guard takes a keyed build alone from here (S4c Task 11; the design §8.3; web-lane §7; RUNBOOK §23 stage 2 step 3)"
```

Checks for the reviewer: the fixtures' diff is the version alone; `cargo test -p yokohama-web --lib` carries `the_two_npm_packages_carry_the_workspace_version` green; the guard's refusal of a keyless production build demonstrated on the export (`npm run deploy -- --dry-run` or the guard's own cell); no deploy, no tag.

---

### Task 12: the records at the branch's head · impl: sonnet · review: opus

The board, the ledger and guidingpath at the state the whole-branch review reads: both parts landed, the window pending.

**Files:**
- Modify: `docs/handover.md` — the `Updated` line (Part 1 and Part 2 landed with their commits; the sequence and the roster to come; NEXT the review, the PR, the window — Blocks B, C, D); Track 5's body with the tasks ticked by commit
- Modify: `guidingpath.md` — one decision-log row, dated, "SECURITY S4b AND S4c LANDED ON `security-s4b` — the window pending" naming the plan, the tasks' commits, the joint-sequence ruling (item 2), the preview ruling (item 3), the picture ruling (item 4), the review ruling (item 5), C-1..C-14; the close's row comes later (two rows, as cycle F had)
- Modify: `docs/security-ledger.md` — the `Updated` line's entry extended with Task 11's bump and stage 2's commit

- [ ] **Step 1: Edit** as the Files block says.
- [ ] **Step 2: Verify**

Run: `(cd web && npm run test:wasm -- doc-links) && git grep -n -E 'S4b/c|S4b and S4c' -- docs/handover.md guidingpath.md | wc -l`
Expected: the link cell green; at least two sites.

- [ ] **Step 3: Commit**

```bash
git add docs/handover.md guidingpath.md docs/security-ledger.md
git commit -m "docs: the records at the branch's head — the board's Updated line and Track 5 with every task's commit, guidingpath's row for the joint sequence landed with the window pending, the ledger's Updated line (S4c Task 12)"
```

---

## Block B — the owner present, before the review (the orchestrator's steps)

- [ ] **B1. The triage marks (the design §8.1).** The owner reads Task 9's table and marks each row `private` (the default), `publish` (copied into `docs/` under a dated name with a provenance line — its own commit, listed in the close; the register and the address grep over it first) or `discard` (the owner's own `rm`, in the owner's shell — never the orchestrator's bulk deletion). The marked table goes into the close verbatim.
- [ ] **B2. The sidecar sync (the design §2.25, §8.0; C-8).** The owner mounts the share (`smb://192.168.1.173/yokohama`, the owner's credential); the orchestrator copies ADDITIVELY (`rsync -a`, no `--delete`) `bridge-captures/`, `docs/temp/`, and the memory directory `~/.claude/projects/-Users-jmoore-Documents-Github-yokohama-assistant/memory/` into dated folders on the share, reads back `du -sh` of each against the source, and records the recipe, the sizes and the time; the owner says whether a standing job is arranged (a launch agent is the owner's to install; the recipe is recorded either way); the board's item 8 closes with the reading in the close.
- [ ] **B3. The window's inputs read before the review:** the Turnstile site key's source (the owner's; a mode-0600 scratch file at the window, never a document); the test identity's address (the owner's; named in the session's record only); the Access application's current hostname list read by the API (`worker/RUNBOOK.md` §10's read; the four hostnames expected — and, the application being hostname-wide, no destination path is added for `/api/leave`: S4a's close §9's "the Access application's destinations" is answered by that reading and recorded); the edge rule read by id, DISABLED; the zone's ruleset version noted; `wrangler whoami`; Docker Desktop and the NAS's Portainer reachable; the two testers' availability for D3's socket reading (optional).

---

## After Task 12 — the sequence, the roster, the review, the merge, the tag (the orchestrator)

- [ ] **The suites at the head, once, as one sequence in a fresh export with its own `target/`:** `cargo test -p yokohama-web`, `-wasm`, `-worker`, `-bridge`, `-cli` (with `bridge-captures/` symlinked), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p gen-data && git diff --exit-code crates/yokohama-core/src/game/data.rs`; in `worker/` `npm run build:module:debug && npm run typecheck && npm test && npm run test:scripts`; in `web/` `npm run build:wasm && npm run typecheck && npm test && npm run test:wasm && npm run e2e`; `bash deploy/run-local.sh --dry-run`. The counts and the wall go in the close.
- [ ] **The fifteen-gate roster at the same head** (the design §5.1; the version moved): the fifteen commands of `CLAUDE.md`'s Commands as ONE uninterrupted sequence, release binaries pre-built, single-threaded exactly where the command says so, on this Mac (Apple M5), machine-tagged, the per-gate walls recorded; the figure goes in the close and the board's gates line beside the 2026-10-19 reading, never against it.
- [ ] **The whole-branch review — a Workflow of five** (item 5; C-11): on exports of the head, each with its transcript in the SDD workspace — (1) Opus, the documents' truth: every command of the README and the two guides run, every relative link by the cell and every external one by `curl -I`, every figure of the computer-player page re-found in its record, the register and the bar sentence; (2) Opus, the agent-facing documents: Task 5's sentence ledger re-derived from `ddb9287b` (nothing lost, nothing re-worded), guidingpath and the board against the tree and the tracker; (3) Opus, the workflows and the release path: Task 8's pins re-resolved, the runtimes read, the cells' mutants, Task 11's diff (the version alone), the guard's refusal, the tag job's ancestry rule against this branch's shape; (4) Sonnet, scope and residue: the diff against the design §7 and §8 and this plan, the fenced crates unmoved bar the version, the dated documents unedited, the register and address greps over the whole diff; (5) Sonnet, the records: the ledger, the board, guidingpath's row and the relocation map's rows against the tree and the commits. The orchestrator synthesizes the five transcripts into one verdict (READY / READY WITH FIXES / NOT READY) with every Critical and Important named by site; one Opus fixer lands the wave as its own commits (a defect found at a site outside the brief is reported and landed in the same wave); one Opus scoped re-review on an export of the wave's head. No reviewer re-runs the walk or the cross-model read.
- [ ] **The PR (outward)** `security-s4b` → master, the plan and the SDD ledger's verdicts linked, merged on the owner's word as a MERGE COMMIT (never squash or rebase — Review Focus 4); then `git checkout master && git pull --ff-only origin master && git push lab master`.
- [ ] **The tag (outward; web-lane §7; the design §8.3):** on master's merge commit, `git tag -a v0.7.3 -m "Yokohama CLI v0.7.3 — the public release: the seat-aware admission, the keyed door, the front door's documents"`, `git push origin v0.7.3 && git push lab v0.7.3`; the release workflow's run watched to green (`gh run watch`); `gh release view v0.7.3 --json assets` reads the five assets. A tag off master's history is refused by the first job — the plan tags master only.

---

## Block C — the window (the owner present; the design §5 in order; the orchestrator at the keyboard with the owner)

The rollback is read FIRST, aloud: `worker/RUNBOOK.md` §23 stage 2 step 9 (the hostname back on the Access application first, then the rule disabled by id, then stage 1's keyless build and `off` redeployed, `DEV_DOOR_CLIENT_ID` put back if deleted) and the design §5's paragraph (the repository can be set private again, but a switch back does not unpublish what was forked or cached in the interval, which is why the switch waits for stage 2's readings).

- [ ] **C1. The release** is published (the tag step above) and read: the five assets, `SHA256SUMS` matching a downloaded archive.
- [ ] **C2. The NAS as the canary (C-7):** `deploy/README.md` §9 and `deploy/update.sh` — the image `yokohama-web:0.7.3-s4` built from the merge commit, deployed, the smoke (`deploy/smoke-image.sh`), the negative probe from the terminal container (B10's), the owner's read through Access (`/api/health` reads `0.7.3`; the door's three cards; a bot game dealt); the record `deploy/RUNBOOK.md` §3.24 "The twenty-first deployment — `yokohama-web:0.7.3-s4`" written in §3.23's shape.
- [ ] **C3. Stage 2 on the Worker, `worker/RUNBOOK.md` §23 stage 2, steps 1–8 VERBATIM** (the design §2.14: "S4c runs it verbatim with the owner present and does not redesign it"): the keyed build with the seven values (the site key from the owner's scratch file), `npm run build` in `worker/`; the two files already `on` (Task 11); the edge rule ENABLED by its id and read back through `node scripts/edge-rule.mjs --verify`; `npm run deploy` and the smoke over WARP; **`yokohama.moore.nyc` OFF the Access application LAST** (the NAS, CLI and TEST hostnames stay; a PUT replaces the whole list — read back); the readings as a stranger from a fresh profile off WARP (the overlay, the WorkOS page, the lobby, a dealt table, the socket, the sign-out; a pinned previously-released profile lands a waiting game anonymously); `DEV_DOOR_CLIENT_ID` deleted; the scratch key file removed. Then the private test Worker at `0.7.3` too — `npm run deploy:private`, its file staying `off` and its hostname staying on Access (security S3's precedent, `worker/RUNBOOK.md` §26) — so the three tiers read one version. §23's stage-2 record written as stage 1's was, timestamps included.
- [ ] **C4. The switch (outward; the design §5.3):** `gh repo edit jmoore2333/yokohama-assistant --visibility public --accept-visibility-change-consequences`; `gh repo view --json visibility` reads `PUBLIC`; the time recorded.
- [ ] **C5. Immediately after (the design §5.4; S4a's close §9), each read back:** fork pull-request workflows require approval for all outside contributors (`gh api -X PUT repos/jmoore2333/yokohama-assistant/actions/permissions/fork-pr-contributor-approval -f approval_policy=all_external_contributors`, read back by GET; if the endpoint answers 404 on the day, the browser: Settings → Actions → General → "Approval for running fork pull request workflows from outside collaborators"); `test.yml` enabled (`gh workflow enable test.yml`; `docker.yml` stays disabled, C-12); private vulnerability reporting (`gh api -X PUT repos/jmoore2333/yokohama-assistant/private-vulnerability-reporting`, read back — SECURITY.md's instruction becomes true here); issues on, discussions off (read, unchanged); content reports accepted (the browser: Settings → Moderation → Reported content); squash and rebase merges OFF (`gh repo edit --enable-squash-merge=false --enable-rebase-merge=false`, C-13); the tag ruleset (`gh api -X POST repos/jmoore2333/yokohama-assistant/rulesets` with `target: tag`, `enforcement: active`, `conditions.ref_name.include: ["refs/tags/v*"]`, `rules: [creation, update, deletion]`, `bypass_actors: [{actor_id: 5, actor_type: RepositoryRole, bypass_mode: always}]` — C-10; read back); the description, topics and homepage verified (`gh repo view --json description,homepageUrl,repositoryTopics`); **the social preview uploaded by the owner in the browser** (Settings → General → Social preview; the file from S4a, sha256 `63ed7d58…0a11`) and read back (`usesCustomOpenGraphImage` true by the GraphQL query); **the two e-mail settings** set by the owner (Settings → Emails: "Keep my email addresses private", "Block command line pushes that expose my email"); the community checklist read (`gh api repos/jmoore2333/yokohama-assistant/community/profile --jq '.health_percentage, .files'`).
- [ ] **C6. The site's link previews** (S4a's P-1; the close §9): `curl -s -A "facebookexternalhit/1.1" https://yokohama.moore.nyc/ | grep -o '<meta property="og:[^>]*>'` reads the Open Graph tags; the release feed's fetch of `api.github.com/repos/jmoore2333/yokohama-assistant/releases/latest` reads 200 (the console error the door showed while private is gone — read in the stranger's profile).

---

## Block D — the readings from the outside (the design §8.5; the owner present for D3 and D5)

- [ ] **D1. A fresh clone of the public repository builds:** in an empty scratch directory, `git clone https://github.com/jmoore2333/yokohama-assistant`, `cargo test -p yokohama-core` green, `YOKOHAMA_BGA_CORPUS_OPTIONAL=1 cargo test -p yokohama-cli` green (the nine corpus pins skipping by name), `cargo clippy --workspace --all-targets -- -D warnings` clean; the walls recorded.
- [ ] **D2. The walk repeated from the public URL** (Task 6's brief, a Haiku walker, on the clone of D1): now the `curl` line and "Try it" are provable — the release archive downloaded and run to the wizard's prompt without a quarantine prompt (Review Focus 1), the hosted site's door reached with no login. Friction fixed as commits on master by a pull request (branch protection is not yet on; the design §5.5 lets the close land directly — a README fix lands in the close's commit).
- [ ] **D3. The door as a stranger** (with the owner; a fresh profile off WARP, no cookies): the door; the bot game with no account; the sign-in by e-mail code to the test identity (§2.20); a dealt table; the socket under the cookie; the archive; the sign-out — the rows stage 1 left unread among them; the wave's Issue 1 as far as a window can: the watchers' sentence read at a watchers' figure the reading can reach (several tabs as a stranger at one table) and a seated reconnect admitted; D32's edge rule leg read as 401 then 429 at the stale-client shape if a pinned profile is at hand. Each reading's words recorded; the ledger's live columns take them at the close.
- [ ] **D4. Every link resolves:** the link cell on the clone (`cd web && npm ci && npm run test:wasm -- doc-links`), then every external URL in `README.md` and `docs/README.md` curled once (`curl -s -o /dev/null -w '%{http_code} %{url_effective}\n' -L <url>`): 200 each, the badges' images included.
- [ ] **D5. The push trigger read** (Review Focus 5): the close's docs-only push to master runs NO job (`gh run list --workflow test.yml --limit 3`); the first later push that touches a listed path runs the engine suite — read whenever it happens, recorded in the ledger's row then; `remote.pushDefault` read on both workstations (the owner reads the second).

---

## The close (the orchestrator; pushed to master directly — the design §5.5 — before C7)

- [ ] **The close, `docs/plans/2026-10-26-security-s4b-s4c-close.md`:** what this is; §1 the run — Block A, Part 1's seven tasks and Part 2's five with their reviewers, commits, rounds and verdicts, the walk's final transcript path and its walks' count, the cross-model read's three lists and dispositions, Block B; §2 the one recorded sequence AND the roster's fifteen walls, machine-tagged; §3 every ruling in order with its cost (items 1–6 and the run's P-n and R-n); §4 the window — C1–C6 with timestamps and every read-back value, D1–D5's readings in their words; §5 the relocation map COMPLETED — S4a's close §5's README rows with their final homes (See it running → the README's one picture and `docs/computer-player.md`; Status → `docs/computer-player.md`; Workspace crates → the README's "What is inside" and the index; Where truth lives → the index and `CONTRIBUTING.md`; Hosting → the two host READMEs; Download and run → the quick start), the triage table with the owner's marks and whatever was published; §6 the security project's close — the cycles from the 2026-10-08 review to the switch (the core PR, S1a–S1c, S2a–S2c, S3, the auth cycle's two stages, S4-pre, S4a, this cycle), each with its close named, and the ledger's every row's state; §7 what is deferred (passkeys behind a custom domain; the art lane's eventual merge; the Language School hotfix #62 with the eight fenced comment sites; the four parked polish minors; `docker.yml` disabled; whatever D2 and the cross-model read recorded); §8 the whole-branch review and its wave; §9 the handover for the AI lane's session (Track 1 first).
- [ ] **The records in the close's commit:** the board's `Updated` line and gates line (the roster figure beside the 2026-10-19 one), Track 5 → "the security project CLOSED; the public release LIVE" with the pointer to the ledger and this close, the owner's items that remain; `guidingpath.md`'s Current state's one sentence flipped (the window happened; the versions live) and TWO rows — the security project's closing row and the public release's row; the ledger's every live column filled from D3 and the `Updated` line; `docs/web-lane.md` §1 "where it stands" and §8's table brought to the state after stage 2 (the hostname off Access; the rule enabled; the readings); `CLAUDE.md`'s "Where things stand" sentence flipped (Task 5 wrote it for this moment) and ONE new standing rule only if the cycle earned one; `worker/RUNBOOK.md` §23's stage-2 record and `deploy/RUNBOOK.md` §3.24 (written at C2 and C3, committed here); **`docs/web-guide.md` §2's Access sentence** ("Until the switch that opens the site to the public, Cloudflare Access also stands in front of the hostname …") removed, since the switch has happened (S4a's C-7: "S4c's close re-points that sentence"), and the link cell run over the guide after the edit.
- [ ] **The archive cut (the design §8.2; C-5):** `python3 docs/tools/guidingpath-relocate.py` moving every decision-log row older than the close's two rows (2026-09-29 → the Task 12 row) VERBATIM into `guidingpath-archive-security.md` with the tool's byte check green; the index gains the row `| 2026-09-29 → 2026-10-<close day> | guidingpath-archive-security.md | The web lane's finish (M6.5c2 cycles D–F and the wrap) and the security project from its review to the public release … |` and the live file's row reads "from this close on"; `docs/README.md`'s authority group gains the fourth archive; `CLAUDE.md`'s Project paragraph names it among the four files; the three older archives untouched (`git diff --stat` reads none of them).
- [ ] **The commit and the push (outward):** `git add` by path (the close, the board, guidingpath, the new archive, the ledger, the lane document, CLAUDE.md, the two runbooks, `docs/README.md`, any D2 fix); the subject in the house style; `git push origin master && git push lab master`; D5's "no run" read.
- [ ] **C7. Branch protection on master, the window's LAST act (outward; the design §5.6; C-10):** `gh api -X POST repos/jmoore2333/yokohama-assistant/rulesets` with `target: branch`, `enforcement: active`, `conditions.ref_name.include: ["refs/heads/master"]`, `rules: [{type: pull_request, parameters: {required_approving_review_count: 0, dismiss_stale_reviews_on_push: false, require_code_owner_review: false, require_last_push_approval: false, required_review_thread_resolution: false}}, {type: non_fast_forward}, {type: deletion}]`, NO bypass actors; read back; the time recorded in the SDD ledger (the close is already on master — a later correction is a pull request).
- [ ] **The memory** (`feedback`/`project` notes in the orchestrator's memory directory): the cycle's state, the lab rule, the window's facts; the pickup note points at the close's §9.

---

## Review record

The before review (Block A, A3) and every correction the run meets are recorded here, each applied at its site above and tagged `(P-n)`; the SDD ledger holds the rulings as R-n.

- **P-1** (2026-10-26, the pre-flight's conflict scan) — Task 2's link cell listed seven files while `docs/method.md` is Task 3's, so the cell would have sat RED on the branch for one commit. Task 2 lists the six files that exist at its run; Task 3's Files block, which already names the cell, adds the seventh.
- **P-2** (2026-10-26, the pre-flight's conflict scan) — Task 1's link loop reads `docs/README.md` and `docs/method.md` as `MISSING` until Tasks 2 and 3 create them (S4a's P-17 shape); the expectation names the two as pending.
- **P-3** (2026-10-26, the pre-flight, A3 (i)) — the live sites that spell `0.7.2` at `8bcd94ba`, classified: MOVE with the bump — `Cargo.toml:16`, `web/package.json:4`, `worker/package.json:4` and the two locks' root lines, `web/src/wire/health.json:11`, `web/src/wire-script/backup-envelope.json:6`, `web/src/wire-script/archive-walk.json`'s `engine_version` lines (all by §7's steps 1–3), `deploy/Dockerfile.runtime:18` (the example tag, `0.7.3-s4`); STAY as records or facts — `deploy/README.md:243`, `deploy/RUNBOOK.md:287, 297, 301, 2563–2572`, `docs/handover.md` (three), `docs/security-ledger.md:5`, `docs/web-lane.md:34` (the lane's "where it stands" — the close's, not the bump's), `guidingpath.md:33, 110`, `worker/README.md:1505`, `worker/RUNBOOK.md:78, 359, 443, 2692–2698`. The exclusion file is `.superpowers/sdd/2026-10-26-security-s4b-s4c-plan/version-sites-that-stay.txt`.
- **P-4** (2026-10-26, the pre-flight, A3 (ii)) — `YOKOHAMA_TURNSTILE` flipped to `"on"` in `worker/wrangler.jsonc` and `worker/wrangler.dev.jsonc` on an export of `8bcd94ba` reddens FIVE cells of `worker/scripts/deploy-guard.test.mjs`: "access requires the Access pair; oidc requires the issuer and client id and forbids the pair; workos requires the client id and forbids the issuer", "wrangler.jsonc is held to its own record: name, bucket and route", "holds the private file to production's rules when it is deployed (R-49)", "declare YOKOHAMA_TURNSTILE off in five and on in the open file alone", "pass the whole guard: production, the dev file and the private file over a keyless build, the open file over a keyed one" — these are the cells Task 11's Step 3 moves. The three `lane-variables.test.mjs` cells also read red on that export, as on every export (the known git-cell reading, S4a's close §2), and are not the flip's.
- **P-5** (2026-10-26, Task 1's implementer, its probe of the brief's reviewer check) — the brief's check `npm run test:wasm -- assets-notice` read RED on the one-picture README: S4a's cell asserted at least THREE distinct picture directories across the four documents, a floor measured against the old README's pictures under `docs/images/`, `worker/docs/img/` and `web/public/help/`. The clean-sheet README keeps one picture in `worker/docs/img/`, so the four documents name two directories. The floor is re-measured at two in its own commit (`9e77b423`), ahead of the README's, so every commit stays green; Task 1's Files block names the cell. The reviewer judges whether a better anti-vacuity shape exists.
- **P-6** (2026-10-26, Task 1's implementer) — the quick start fetches the release archive by its versioned name under `releases/latest/download/`, which answers 404 for the name once a newer release is latest; the name MOVES with the bump: Task 11's Files block names `README.md` (its Step 4 grep already lists the line, `README.md` not being in the exclusion file). `curl -fLO` is the form (plain `-L` exits 0 on a 404 and saves the error page as the archive). The guide in the archive is `CLI-GUIDE.html` and the release's separate asset `yokohama-cli-guide.html`; the README names both.
- **P-7** (2026-10-26, Task 1's implementer) — the commit trailer names the model that wrote the commit (`Claude Opus 5.5`), not the orchestrating session's; dispatches from here say so, and the house style is the model that wrote it.
