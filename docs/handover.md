# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-11-09 — YOUR CLEAN-EYES SESSION on `security-s4b`, before the window (the plan's P-28..P-33): an independent pass over everything the switch makes visible read clean but for what you ruled the same day — a second address of yours redacted in three dated plans, the staging API keys expired and their line dropped, every claude.ai address out of the tree, history kept whole, the collaborator kept; the front door reworked for a visitor who arrives cold — `docs/engine.md` new, the README opening on the project's purpose, the computer player, with a terminal animation of a game against it, then the browser picture at the hosted site's section and a where-to-go table (by your word; P-29, P-34), the engine's status line softened to a belief with its evidence, the bot's page opening with its story in one paragraph — reviewed on an export (READY WITH FIXES, fixed); the two repositories ruled (work on the lab; a branch reaches origin once, review-clean, as its pull request; `CLAUDE.md`, `CONTRIBUTING.md`, `docs/method.md` §2); `test.yml` re-enabled and green on the branch with `web.yml`. **Nothing is deployed, tagged or public.** NEXT, with you: Block B (the triage marks, the sidecar copy, the window's reads); the merge; the prerelease tag that reads the release workflow, then `v0.7.3`; the window; the readings; the close. Before it: 2026-10-26 — security S4b and S4c LANDED on `security-s4b` as ONE sequence (cut from master `ddb9287b`, the merge of PR #63; the plan `docs/plans/2026-10-26-security-s4b-s4c-plan.md`) and read at the review head: the one sequence of suites at `2569f188` (13.0 min) and the fifteen-gate roster at the same head (452 s, 15/15) ran on the MacBook Air (Apple M5), their counts and walls in the gates line below; the five-reviewer Workflow review of `b0f88e9d` read READY WITH FIXES, 0 Critical in every lens; and its wave landed as four commits — `cd505a0c` (the standing document and the reader-facing pages), `abc1a1e8` (the two cells), `2a3caee6` (the relocation gaps) and this one (the records) — with the plan's corrections now P-1..P-27. **The window is pending, and nothing is deployed, tagged or public.** NEXT: the wave's scoped re-review; Block B with you (the triage marks, the sidecar copy, the window's reads; an item you mark `publish` lands as its own commit before the merge); the pull request merged as a merge commit on your word; the tag `v0.7.3` on master's merge commit; then the window, the readings and the close (Track 5). Then, the same evening: the wave's scoped re-review read ALL 21 ITEMS ADDRESSED with no new Critical or Important breakage; and on your ruling during the wave ("we should include bringing in these files at the next release, before the public switch" — P-27) Task 13 landed `721bf395` (fix round `43b851fb`): the three release archives pack `LICENSE` and `NOTICE` beside the binary from `v0.7.3`, held by a cell that binds both lines to each job's one archive command, and the README names the macOS quarantine remedy; its review read clean after one round. **The branch is complete at this head and waits for you: Block B** (Track 5's box); PR #64 is open against master and squash and rebase merges are off on origin, so the merge can only be a merge commit. The close rewrites this line, the gates line and Track 5. Before it, the same day: Block A (the plan and board `8bcd94ba` on the lab and origin, the pre-flight, `security-s4a` off origin), then Part 1 (Tasks 1–7: the README from a clean sheet, the docs index and its link cell, the method page, the help panel's and the self-host README's sign-in sentences, the agent-facing documents, the clean-eyes walk in three walks and the cross-model read with their fixes) and Part 2 (Tasks 8–12: the four workflows pinned to commit SHAs with `test.yml`'s push trigger, the `docs/temp/` triage draft, the ledger's and the lane document's pre-window records, `0.7.3` with stage 2's `YOKOHAMA_TURNSTILE` commit, and the records at the branch's head, `2569f188` and `de7a4351`), every task's commits ticked in Track 5.

## Gates at HEAD

The latest reading: this cycle's ONE sequence at `2569f188` on `security-s4b`, in a fresh export with its own `target/` on the MacBook Air (Apple M5), 20:42:06–20:55:08Z on 2026-10-26, 13.0 min (the SDD ledger's "After Task 12"; the close's §2 will record it): web 609 (lib 567, browser_played_pin 5, lane_document 6, the whole-game gate 2, start_refusal 11, wire_fixtures 18), wasm 10, worker 84, bridge 82, cli 863 + 2 ignored (the corpus present), clippy clean, `gen-data` byte-identical; worker typecheck 0, the pool 302 in 14, test:scripts 797 of 800 on the export (the three `lane-variables` git cells; 800 in the checkout); web typecheck 0, vitest 1,587 in 76, test:wasm 405 in 18, e2e 83 passed in 6.4 min; `run-local.sh --dry-run` as specified. Every count is the pre-flight's at `8bcd94ba`, which equalled S4a's close §2, plus this cycle's cells. The review's wave adds one `test:wasm` cell (`abc1a1e8`, the link cell's CRLF copy) and moves no other count. The engine's own suites last read at security S2c's close — core 1,587 / 29, oracle 191, difftest 52 / 5, eval_probe 214, solver 132 (the suites at `67e26a1c`; the S2c plan's Review record) — and `crates/yokohama-core`, `crates/yokohama-oracle` and `crates/yokohama-difftest` have not moved since. **The latest ROSTER reading** is this cycle's at `2569f188`, the crates' head (no crate has moved since): **452 s**, 15/15, ONE uninterrupted sequence with the release binaries pre-built, on the MacBook Air (Apple M5; Mac17,3), 20:55:52–21:03:24Z — fuzz 44, census 1, observation 3, action_equiv 27, action_worst 0, arena 20, policy_div 6, evaluator 251, architecture 46, recovery_lib 10, piped_dogfood 12, differential 25, routes3_diff 1, scoring_diff 2, routes3_worst 3 (the SDD ledger; the close's §2 will record it). It stands beside the 2026-10-19 row's 497 s (the MacBook Air, Apple M5, at the `0.7.0` bump `3991feb3`) and the 2026-10-17 row's 478 s (Mac17,3, Apple M5, at `ed8c7f6f`), never against them; ≤420 s stays crossed (OD-6). Re-count at your own HEAD.

## Track 1 — V4 close-out: the owner annotates the ledger, then the findings doc, then D3 + W6

**State (2026-09-11, second session):** three complete games against the shipped bot, all scribed
by reconstruct-by-replay (`why <n>` at every bot event on copies of the records, release build):

| game | file (gitignored `docs/temp/owner-games/`) | result | events | bot events / refusals |
|---|---|---|---|---|
| Game 4 (CLI, 2026-08-30, you started) | `game4-v4.FINAL.json` · `game4-why.log` | **you 82 – bot 81** | 247 | 119 / 0 |
| Game 6 (CLI in Forge, 2026-09-11, bot started) | `game6-v4.FINAL.json` · `game6-why.log` | **you 120 – bot 81** | 252 | 127 / 0 |
| Web game `sim-551da3b64523dbd1` (browser, 2026-09-11, you started) | `web-2026-09-11-sim-551da3b64523dbd1.json` · `web-2026-09-11-why.log` | **you 89 – bot 68** | 243 | 124 / 0 |

`game5-v4.json` (40 events) was abandoned by your choice at the table; `sim-016f847457c957a6`
(100–56, the first web game) stays an optional exhibit, unscribed. The scribe notes with the
setup echoes, self-read curves and event-anchored scribe-side reads for all three are
`docs/temp/owner-games/notes-2026-09-11.md` (its last two sections carry the reviewers'
corrections to the scribe's own claims — read them before quoting the earlier sections).

**THE LEDGER (where your annotation happens):** a private page of yours (its address is in your notes and the
session memory, not in this repository) — the three games turn by
turn: every move with its event index, the bot's own arithmetic for every decision (self-read,
stage, candidates, ties, nonzero feature rows, runner-ups, searched line), an Opus reviewer's
commentary on every turn (one reviewer per game, each having read the rules reference and the
Phase-B findings first), VP and self-read charts, key moments, and per-turn tag + note fields, a
verdict and a general field per game, and answer fields under the reviewer's questions. **Notes
save as you type into the artifact's database** (the status line top-right reads "notes saved";
if it reads "not connected", the note is in that tab only — reload and retry). The page's sources
and the scribe tooling are parked in `docs/temp/owner-games/ledger/` (README inside).

**Next session, in order:**

1. **Pull your notes from the ledger** (Artifact tool, `action: read_db`, the ledger's address,
   `db_op: list`, `collection: notes` → docs `game4`/`game6`/`web`; shape in the ledger README).
   Every note is anchored to a turn number; the datasets map turns to event ranges.
2. **Write the findings package on the Phase-B shape** (`docs/plans/2026-08-17-phase-b-owner-games-handover.md`
   is the precedent): a dated tracked document with event-anchored findings that pair YOUR line
   with the scribe/reviewer grounding, the three outcomes, the routing per D7's branches; plus
   decision-log rows for the three outcomes (game 4's 82–81 is in NO tracked document yet).
   Scribe-side headline findings already on file (notes file + the ledger's reviews): (a) threat
   blindness is now measurable in the V4 columns — game 6 e109 read +951 with your three
   fulfillable orders in its table at weight 0 (`held_order_value` is weighted 0 until the end
   trigger; `executable_orders`/`matched_production_access` 0 always; loading a threat RAISES its
   read), and the web game's e124–e148 customs landing (+22) was priced only through
   `imported_goods` w6; (b) game 4 e241 is terminal-exact and the shipped beam (b=4, m=8) had 12
   lines tied at lose-by-1 while `driven table …:241 --contestant lookahead-v1:b=16,m=32
   --decision-seed 1013458669601587549` finds "Place 2 in Bank" one VP better (82–82, still lost on
   the clockwise tie-break) — a search-breadth miss, weights irrelevant; the game-4 reviewer adds
   that the point was really lost at the Church (2-to-2 rightmost tie-break, a 3-point swing);
   (c) produce-and-hoard live in game 6 (7 tea held e68→end, one order all game); (d) the web
   game's final round HAD intent (e214–221) where game 6's last turn was dead; (e) instrument
   note: the stage proxy jumped 750000→1000000 inside one bot turn (game 6 e217→e219), and the
   forced turn-start node consistently reads worse than the real root one event later (both
   reviewers) — those reads are not part of the self-read curve.
3. **Rule D3 and W6 together** with the package in hand (close doc §3.2: a word suffices on each).
   The scribe's recommendation, offered 2026-09-11 and NOT a ruling: **D3 YES, W6 WAIT**, with a
   V5 (weights) cycle entering only through its own pre-registration after the D3 diagnostic read
   — because two of the three headline findings are representation (weights), the breadth exhibit
   is a strictly-better-not-winning line, and the close's own sentence is "W6 tunes the
   THEN-SETTLED table". A GO is a fully defensible reading of "beats the previous bot at 0.69".
4. Then the ladder as the amendment wrote it (§3 Phases D–F): W6 screening → D1 identity freeze →
   W7 (the @V1 instrument is blocking implementation work, then the OD-5 pre-registration you
   sign, then ONE confirmatory battery) → OD-8 acceptance play → OD-11, the bar. A weight change
   is a DIFFERENT contestant (D1), never a tune.

**The V5 ship's one CLI line is DONE (R-F4; the hotfix, 2026-10-01):** a V5 build plays V4 games
on the web exactly when it still derives V4's exact descriptor, `LOOKAHEAD_IMPL_VERSION` and
`EVAL_IMPL_VERSION` unmoved — a weights-only V5; the admission is in place since 2026-10-01
(`GRANDFATHERED_NAMED_IDENTITIES` in `crates/yokohama-cli/src/bot.rs`). **What the re-point owes
this line is the append:** the V5 ship adds its identity to that list and never removes one. A V5
that moves either refuses V4 games as DRIFT (the re-entry handover §5, item 12;
[`web-lane.md`](web-lane.md) §15; [the spec](plans/2026-10-01-cli-grandfathered-identity-hotfix-spec.md)).

**You decide:** D3 · W6 · OD-6 (restate the ≤420 s roster target — 534 s at cycle F's close (an earlier reading; the latest roster sequence read 452 s at this cycle's review head `2569f188`, the MacBook Air (Apple M5), the gates line above, beside 497 s at the 0.7.0 release on the same machine, guidingpath's 2026-10-19 row)) · OD-8's
format (game count, rubric — its inherited "brake behind" line contradicts canon ruling 15).

## Track 2 — The Language School hotfix: issue #62, after V4's vetting and before V5

**State:** one rules defect at `SEMANTICS_REV` 7, found by the technology-card review of 2026-10-04 and ruled a defect by you: Language School's yen can miss its own order's warehouse take. Its fix moves legality, so it bumps the rev to 8 and is a cycle of its own, **after the V4 vetting (Track 1) and before any V5 planning** (rulings, 13). It is tracked as **issue #62**, which summarizes it. The entry document is `docs/plans/2026-10-04-language-school-hotfix-handover.md` on the branch `docs-language-school-hotfix-handover` (`fd3c2074`, on the lab, unmerged until the cycle opens; off origin since 2026-11-09, by the two-repositories rule): the ruling, the mechanism, a reproduction, the impact chain, the fix shapes and the open questions.

## Track 3 — The art lane: local, on `art-replacement-spec`, by its own rule

**State:** the art-replacement lane (opened 2026-10-05) works in its own worktree, `../yokohama-assistant-art`, on the branch `art-replacement-spec` (head `17fa1320`, 2026-10-06, the style round's record); `docs/brand-guide/` and `docs/tools/art/` moved there in S4a (its Block A). **Its rule (you, 2026-10-05): no push to origin, no PR and no merge until the parallel set is done** — stricter than the lab rule's "a branch reaches origin with its plan". The lab holds the branch as a private backup, which that rule allows (the S4 design §2.18, ruled 2026-10-22). Its entry documents are on the branch: the spec `docs/plans/2026-10-05-art-replacement-spec.md` (its §1.9: no scan, screenshot or crop of the published game is ever sent to a model), A1's close `docs/plans/2026-10-05-art-replacement-cycle-a1-close.md` and the A2 handover `docs/plans/2026-10-06-art-replacement-cycle-a2-handover.md`.

## Track 4 — The web lane: complete

**State:** finished at M6.5c2 cycle F and merged as `v0.6.0` on 2026-10-01; hardened since by the security project (Track 5). **Live at `0.7.2` since 2026-10-20 on all three tiers, each behind Cloudflare Access:** the NAS `yokohama-web:0.7.2-s3` at `yokohama-nas.moore.nyc` (`deploy/RUNBOOK.md` §3.23), the Worker at `yokohama.moore.nyc` at identity source `workos` (`worker/RUNBOOK.md` §27; the edge rule disabled until stage 2, [`web-lane.md`](web-lane.md) §1), and the private test Worker at `yokohama-test.moore.nyc` (§26, §27). The operating record is [`web-lane.md`](web-lane.md); the player-facing guides are [`web-guide.md`](web-guide.md) (the browser client) and [`computer-player.md`](computer-player.md) (the bot). The lane's open issues are polish, deliberately delayed: #53 (a turn alert when the window is not focused), #58, #59 and #60 (the debts the auth and S3 cycles recorded forward).

**Optional, if you want them:** (1) the handset's ten minutes (R-6.5c2.33): your own phone against either host, to confirm what the simulators read ([`web-lane.md`](web-lane.md) §3's phones row; the readings `worker/RUNBOOK.md` §19's phones bullet, which also records three owner's-eye layout readings, not fixed); (2) the blurb's words, if you want them changed (the placeholder text is quoted in [`web-lane.md`](web-lane.md) §14); (3) the REOPEN CORNER S4-pre recorded (its close `docs/plans/2026-10-22-security-s4-pre-close.md` §5): a stream or socket admitted for a seat holder whose seat you then reopen stays counted seated until it reconnects — recorded on both hosts and in the ledger, not changed; closing it would be a ruling and a cycle of its own.

## Track 5 — The security project: closing with the public release (S4b and S4c, one sequence on `security-s4b`)

**State:** the project opened 2026-10-08 with the web lane's review; its record is the ledger `docs/security-ledger.md`, beside the spec `docs/plans/2026-10-08-security-remediation-spec.md` and the S4 design `docs/plans/2026-10-22-security-s4-public-release-design.md`. Landed and merged to master: the core PR (#48), S1 (#49), S2 (#52), the auth cycle with S3 (#57, S3's #56 inside it), S4-pre (#61) and S4a (#63, `ddb9287b`, 2026-10-26; the tree's public shape — `tts-assets/`, which held the Tasty Minstrel Games English edition (20 pages), and the OKAZU Brand English rulebook (16 pages) left the tip in `e72492c1` — with the close `docs/plans/2026-10-23-security-s4a-close.md`). Everything through S3 is live at `0.7.2`; S4-pre's seat-aware admission reaches the hosts with `0.7.3`.

**This cycle** — the plan `docs/plans/2026-10-26-security-s4b-s4c-plan.md` (its "Settled at the brainstorm" carries your rulings of 2026-10-26, its Review record the corrections P-n; the gitignored SDD ledger `.superpowers/sdd/2026-10-26-security-s4b-s4c-plan/progress.md` every task's verdicts):
- [x] Block A: the plan and board `8bcd94ba` on the lab and origin; the pre-flight `c7e2319b`; `security-s4a` off origin.
- [x] Part 1 (S4b), Tasks 1–7: the README `ed5aa437` (the assets-notice cell's floor `9e77b423`; fix round `63f781c7`); the method page `e4ecc0ae` (fix round `4a7ef928`); the docs index and link cell `6f34eaba` (fix round `41ad3ab8`); the sign-in sentences in the help panel and the self-host README `f7663920` (fix round `763f7ba4`); the agent-facing documents `bf7e48b9`; the clean-eyes walk (three walks; its fixes `e475dc27` and `7490c78d`); the cross-model read's wave `c5f12b3a` and its second round `73d396b7`.
- [x] Part 2 (S4c), Tasks 8–12: the four workflows pinned and `test.yml`'s push trigger `8afaaaca` (fix round `4b62c9a4`); the `docs/temp/` triage draft (Task 9, no commit — the table is your instrument at Block B); the pre-window records `08bf0509`; the version `0.7.3` with stage 2's `YOKOHAMA_TURNSTILE` commit `d6db6b3e`; the records at the branch's head (Task 12, `2569f188`, fix round `de7a4351`). The plan's corrections P-1..P-27 are committed beside them.
- [x] By the orchestrator: the suites as one sequence (`2569f188`, 13.0 min) and the fifteen-gate roster at the review head (`2569f188`, 452 s, 15/15; the gates line above); the five-reviewer Workflow review of `b0f88e9d` (READY WITH FIXES) and its wave (`cd505a0c`, `abc1a1e8`, `2a3caee6` and the records commit).
- [x] The wave's scoped re-review (one Opus reviewer, on an export of the wave's head `81426c27`): all 21 items addressed, no new Critical or Important breakage (the SDD workspace's `final-review-rereview.md`).
- [x] Task 13 (your ruling P-27): the three release archives pack `LICENSE` and `NOTICE` from `v0.7.3`, with a cell and the README's quarantine remedy — `721bf395`, fix round `43b851fb`; review clean after one round.
- [x] Your clean-eyes session (2026-11-09; the plan's P-28..P-33): the independent pass and its rulings (`86953509` and this commit's records); `docs/engine.md` `b3e85efe`, the README `e2a21294`, the bridge guide's paths `98ec9046`; the scoped review of `98ec9046` (READY WITH FIXES) and its fix round `448a23f3`; the claude.ai addresses out, the terminal picture and the two repositories' rule in the commits that follow; `test.yml` and `web.yml` green on the branch.
- [ ] **NEXT — Block B, with you, before the pull request merges:** the triage marks (an item marked `publish` lands as its own commit with a scoped review, before the merge); the NAS sidecar copy (rulings, 8); the window's inputs.
- [ ] PR #64 (`security-s4b` → master, open since 2026-10-26; squash and rebase merges are OFF on origin since the same day, C-13) merged as a MERGE COMMIT on your word; the prerelease tag `v0.7.3-rc.1` on master's merge commit (`release.yml`'s first run since S4a changed it, read before the release — P-30), then the tag `v0.7.3` on the same commit.
- [ ] Block C, the window, with you: the release; the NAS as the canary; the Worker's stage 2 verbatim from `worker/RUNBOOK.md` §23, the hostname off Access last; the switch to public; the settings, your social preview upload and the two e-mail settings among them.
- [ ] Block D, the readings from the outside: a fresh public clone; the walk again from the public URL; the door as a stranger; every link; the push trigger.
- [ ] The close `docs/plans/2026-10-26-security-s4b-s4c-close.md` with the decision log's archive cut; then branch protection on master, the window's last act.

**After the close** this track reads the security project CLOSED and the public release LIVE, with the ledger `docs/security-ledger.md` and that close as its records.

## Owner rulings outstanding (one line each; detail in the re-entry handover §5)

**Security S3's items (2026-10-20), still open:** (a) a LAN DNS record for `yokohama-test.moore.nyc` — the LAN resolver is authoritative for `moore.nyc` and answers the new name with nothing, so LAN devices off WARP cannot reach the test host (Cloudflare's resolver answers 104.21.92.62 / 172.67.187.23); (b) the off-WARP reading at the test host — a device off WARP meets `bdvc.cloudflareaccess.com`'s login (the one reading of S3's Task 7 only you can take); (g) your own browser reads, asynchronous and welcome: a creator's reopen of a real joiner's seat and D2's 429 on production (its door is WorkOS, so the window's curl could not sign in), the NAS's next hourly sweep line, and the update reading on a pinned browser.
**Security S4a's items (2026-10-24):** the social preview upload and GitHub's two e-mail settings ("Keep my email addresses private", "Block command line pushes that expose my email") — both now AT THE WINDOW (Block C) by your ruling of 2026-10-26, since a private repository shows no social preview — and the second workstation's recipe, the lab as its push default and the no-reply author address (S4a's close §6 and §10).
**The auth cycle's items (2026-10-19):** the providers — Google, GitHub and Apple need your OAuth credentials in the WorkOS production dashboard before their flags go on.

1. D3 depth re-ask — HOLD; the games are in hand, your ledger annotation is the last input (Track 1).
2. W6 GO/WAIT — HOLD, ruled together with D3 (Track 1).
3. OD-6 roster-target restatement — ripe.
4. OD-8 acceptance format — never set.
5. CLI branch-4 register (`docs/plans/2026-08-18-cli-branch4-touchup-spec.md` §5): decisions 11,
   13, and 4/9.
6. The 5-power port-bonus timing divergence (BGA books at placement, the engine at the take) —
   a D-row candidate you said you would research in the rulebook.
7. The sim-seat / named-identity bot seat option (where "difficulty levels" would live).
8. D10's NAS sidecar sync — no row says it was done.
9. The web lane's V5 notes (a sighted Keep; per-good requirement weights) — bot-behaviour changes
   owing their own pre-registration when a cycle takes them up.
13. The Language School hotfix (2026-10-23) — tracked as issue #62: NOT merged in S4a, the docs branch
    `docs-language-school-hotfix-handover` lives on the lab unmerged (off origin since 2026-11-09), and the `SEMANTICS_REV` 7 → 8 bump waits for its own
    cycle after the V4 vetting (Track 1) and before any V5 planning.

Rulings 10 (D32), 11 (passkeys) and 12 (the e2e's whole-game spec) are CLOSED; their records are the ledger's rows and the decision log.

## Entry artifacts by lane

- AI lane: the re-entry handover `docs/plans/2026-09-11-ai-lane-reentry-handover.md`; the ratified scoping amendment
  `docs/plans/2026-08-11-phase3.5c-scoping-amendment.md` (the forward authority for the rest of 3.5c) and the value-architecture
  checkpoint `docs/plans/2026-08-20-phase3.5c-value-architecture-checkpoint.md` (ratified 2026-08-20, the V4 forward authority);
  the V4 close `docs/plans/2026-08-30-phase3.5c-v4-close.md` §3. Phase 3 as a whole is governed by the accepted map
  `docs/plans/2026-07-25-phase3-map.md`, whose §1 settled inputs are owner direction.
- Web lane: [`web-lane.md`](web-lane.md) (the lane's operating record; §11 its records of record); the player-facing guides [`web-guide.md`](web-guide.md) (the browser client) and [`computer-player.md`](computer-player.md) (the bot); the security project's latest close [`plans/2026-10-23-security-s4a-close.md`](plans/2026-10-23-security-s4a-close.md) and this cycle's plan [`plans/2026-10-26-security-s4b-s4c-plan.md`](plans/2026-10-26-security-s4b-s4c-plan.md), beside `docs/security-ledger.md`; then cycle F's plan [`plans/2026-09-30-web-frontend-m6.5c2f-plan.md`](plans/2026-09-30-web-frontend-m6.5c2f-plan.md) and F's close [`plans/2026-10-01-web-frontend-m6.5c2f-close.md`](plans/2026-10-01-web-frontend-m6.5c2f-close.md), beside E's [close](plans/2026-09-30-web-frontend-m6.5c2e-close.md) (the last two rows of `web-lane.md` §11); operations `deploy/RUNBOOK.md` (§0–§3 and §5 the NAS) and `worker/RUNBOOK.md` (the Worker; `deploy/RUNBOOK.md` §6 points there).
- Bridge (BGA capture → import): `docs/bridge-setup.md`; ledger
  `docs/plans/2026-08-08-phase3.5b-execution-ledger.md`.
- Everything: `CLAUDE.md`, then `guidingpath.md`'s Current state and the decision log's tail; the docs index [`docs/README.md`](README.md) and the method page [`docs/method.md`](method.md) (how the work is organized).
