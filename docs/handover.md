# Handover — the owner's task board

**What this is.** The one screen to open before starting a session: the tracks in flight, the
next concrete step on each, and the rulings they are waiting on. It is a POINTER — every fact
here names the document that carries it, and it never outranks `guidingpath.md` (the authority),
`CLAUDE.md` (the operating facts), or a dated document in `docs/plans/`. Keep it current at every
session close: rewrite the stale lines, do not append history (the decision log is for history).

**Updated:** 2026-11-10 — THE PUBLIC RELEASE IS LIVE AND THE SECURITY PROJECT IS CLOSED (Track 5; the close `docs/plans/2026-10-26-security-s4b-s4c-close.md`; guidingpath's two 2026-11-10 rows). PR #64 merged as `9cd7f16e`, the release fix PR #65 as `56867df0` (the prerelease tag read the macOS job red on a retired runner image; it runs on `macos-15` with `apple-m1` now — P-36) and your README lead PR #66 as `a974a793`; `v0.7.3` published from `56867df0` at 01:55:32Z. **`0.7.3` is live on all three tiers:** the NAS first as the canary (`deploy/RUNBOOK.md` §3.24), then the Worker's stage 2 — the keyed build, the edge rule ENABLED by its id, `yokohama.moore.nyc` OFF the Access application last, `DEV_DOOR_CLIENT_ID` deleted (`worker/RUNBOOK.md` §23's stage-2 record) — and the private test Worker (§28). **The repository is PUBLIC** (read 02:33:50Z), with its settings applied and read back: fork pull-request workflows approved for every outside contributor, private vulnerability reporting, secret scanning with push protection, Dependabot alerts, merge commits only, the tag ruleset on `v*`. Your stranger's reading off WARP walked the door, a bot game, the WorkOS sign-in, a table, spectating and the sign-out. The decision log's rows 2026-09-29 → 2026-10-26 are now the fourth archive, `guidingpath-archive-security.md`. **Since the release, the dependency advisories are closed in the lockfiles and the dependencies rule is set** (issue #67's Rust and npm halves; guidingpath's third 2026-11-10 row; `CONTRIBUTING.md`'s Dependencies): nothing tagged or deployed, and the next Worker deploy is wrangler 4.156.2's first, so the private test Worker goes first. **NEXT: the AI lane — Track 1, V4's vetting.** Your items from the window are under "Owner rulings outstanding".

## Gates at HEAD

The latest reading: the dependency advisories' branch (`maintenance-dependency-advisories`; guidingpath's third 2026-11-10 row), in the checkout with the BGA corpus present, on the MacBook Air (Mac17,3, Apple M5), 2026-11-10. **The Rust side, read after `rand` (`21c19f95`) and again after `tokio` (`eff8e1bc`), past which no Rust input moves:** core 1,587 / 29 ignored, cli 863 + 2 ignored, oracle 191, difftest 52 / 5, eval_probe 214, solver 132, bridge 82 and its four node suites, web 609 (lib 567, browser_played_pin 5, lane_document 6, the whole-game gate 2, start_refusal 11, wire_fixtures 18), wasm 10, worker 84, the refusing build's `identity::` slice 9, the `wasm32` check and the Worker's release-wasm build, clippy clean, `gen-data` byte-identical. **The npm side:** web typecheck 0, vitest 1,587 in 76 and test:wasm 406 in 18 at `dedc3f20`, e2e 83 passed in 7.0 min at `6612c64a`; worker typecheck 0, test:scripts 801 in 33 and the pool 302 in 14 at `91497de2` (wrangler 4.156.2, workerd 1.20261102.1); `npm audit` 0 in both packages; the cells that read the edited documents — web 609 with lane_document 6, test:scripts 801, test:wasm 406 with its doc-links — read again at the branch's last commit. **One finding, not that branch's:** `cargo doc -p yokohama-worker --no-deps` reads 2 warnings (two public items' documentation linking private `receipt::…_inner` items), at master's lockfile as at the branch's, where [`web-lane.md`](web-lane.md) §2 says 0. **The latest ROSTER reading** is that branch's at `e41352a6` (its Rust inputs are `eff8e1bc`'s): **456 s**, 15/15, ONE uninterrupted sequence with the release binaries pre-built, on the MacBook Air (Mac17,3, Apple M5), 06:43:38–06:51:14Z on 2026-11-10 — fuzz 44, census 1, observation 3, action_equiv 26, action_worst 0, arena 20, policy_div 6, evaluator 255, architecture 47, recovery_lib 9, piped_dogfood 12, differential 26, routes3_diff 2, scoring_diff 1, routes3_worst 4. It stands beside the public release's 452 s at `2569f188` (the same machine, 2026-10-26), the 2026-10-19 row's 497 s and the 2026-10-17 row's 478 s, never against them; ≤420 s stays crossed (OD-6). Re-count at your own HEAD.

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

**You decide:** D3 · W6 · OD-6 (restate the ≤420 s roster target — 534 s at cycle F's close (an earlier reading; the latest roster sequence read 452 s at the public release's review head `2569f188`, the MacBook Air (Apple M5), the gates line above, beside 497 s at the 0.7.0 release on the same machine, the decision log's 2026-10-19 row)) · OD-8's
format (game count, rubric — its inherited "brake behind" line contradicts canon ruling 15).

## Track 2 — The Language School hotfix: issue #62, after V4's vetting and before V5

**State:** one rules defect at `SEMANTICS_REV` 7, found by the technology-card review of 2026-10-04 and ruled a defect by you: Language School's yen can miss its own order's warehouse take. Its fix moves legality, so it bumps the rev to 8 and is a cycle of its own, **after the V4 vetting (Track 1) and before any V5 planning** (rulings, 13). It is tracked as **issue #62**, which summarizes it. The entry document is `docs/plans/2026-10-04-language-school-hotfix-handover.md` on the branch `docs-language-school-hotfix-handover` (`fd3c2074`, on the lab, unmerged until the cycle opens; off origin since 2026-11-09, by the two-repositories rule): the ruling, the mechanism, a reproduction, the impact chain, the fix shapes and the open questions.

## Track 3 — The art lane: local, on `art-replacement-spec`, by its own rule

**State:** the art-replacement lane (opened 2026-10-05) works in its own worktree, `../yokohama-assistant-art`, on the branch `art-replacement-spec` (head `17fa1320`, 2026-10-06, the style round's record); `docs/brand-guide/` and `docs/tools/art/` moved there in S4a (its Block A). **Its rule (you, 2026-10-05): no push to origin, no PR and no merge until the parallel set is done** — stricter than the lab rule's "a branch reaches origin with its plan". The lab holds the branch as a private backup, which that rule allows (the S4 design §2.18, ruled 2026-10-22). Its entry documents are on the branch: the spec `docs/plans/2026-10-05-art-replacement-spec.md` (its §1.9: no scan, screenshot or crop of the published game is ever sent to a model), A1's close `docs/plans/2026-10-05-art-replacement-cycle-a1-close.md` and the A2 handover `docs/plans/2026-10-06-art-replacement-cycle-a2-handover.md`.

## Track 4 — The web lane: complete

**State:** finished at M6.5c2 cycle F and merged as `v0.6.0` on 2026-10-01; hardened since by the security project (Track 5). **Live at `0.7.3` since 2026-11-10 on all three tiers:** the NAS `yokohama-web:0.7.3-s4` at `yokohama-nas.moore.nyc` behind Cloudflare Access (`deploy/RUNBOOK.md` §3.24); the public Worker at `yokohama.moore.nyc` at identity source `workos`, OFF Access with the edge rule enabled in front (`worker/RUNBOOK.md` §23's stage-2 record, §28; [`web-lane.md`](web-lane.md) §1 and §8); and the private test Worker at `yokohama-test.moore.nyc`, behind Access (§28). The operating record is [`web-lane.md`](web-lane.md); the player-facing guides are [`web-guide.md`](web-guide.md) (the browser client) and [`computer-player.md`](computer-player.md) (the bot). The lane's open issues are polish, deliberately delayed: #53 (a turn alert when the window is not focused), #58, #59 and #60 (the debts the auth and S3 cycles recorded forward). **#67 (maintenance) stays open** for the toolchain pin, the workflows' action majors, the runner images and `unsafe-libyaml`; its `rand`, `tokio` and npm halves landed 2026-11-10, with the dependencies rule (`CONTRIBUTING.md`).

**Optional, if you want them:** (1) the handset's ten minutes (R-6.5c2.33): your own phone against either host, to confirm what the simulators read ([`web-lane.md`](web-lane.md) §3's phones row; the readings `worker/RUNBOOK.md` §19's phones bullet, which also records three owner's-eye layout readings, not fixed); (2) the blurb's words, if you want them changed (the placeholder text is quoted in [`web-lane.md`](web-lane.md) §14); (3) the REOPEN CORNER S4-pre recorded (its close `docs/plans/2026-10-22-security-s4-pre-close.md` §5): a stream or socket admitted for a seat holder whose seat you then reopen stays counted seated until it reconnects — recorded on both hosts and in the ledger, not changed; closing it would be a ruling and a cycle of its own.

## Track 5 — The security project: CLOSED; the public release LIVE

**State (2026-11-10):** the project opened 2026-10-08 with the web lane's review and closed with the public release; its records are the ledger `docs/security-ledger.md` and the last cycle's close [`plans/2026-10-26-security-s4b-s4c-close.md`](plans/2026-10-26-security-s4b-s4c-close.md) (its §6 names every cycle and its close), beside the spec `docs/plans/2026-10-08-security-remediation-spec.md`, the S4 design `docs/plans/2026-10-22-security-s4-public-release-design.md` and guidingpath's two 2026-11-10 rows. Merged to master: the core PR (#48), S1 (#49), S2 (#52), the auth cycle with S3 (#57), S4-pre (#61), S4a (#63) and S4b with S4c (#64, `9cd7f16e`; the release fix #65, `56867df0`). `v0.7.3` is live on all three tiers and the repository is public. What the window did not read is named in the ledger's rows and the close's §4 and §7; the issues the cycle routed are filed (origin #67–#72; the lab's #1 and #2). The fresh public clone and the walk from the public URL are read (the close's §4). Still with the orchestrator at this writing: the reading of this close's own documents-only push, and branch protection on master, the window's last act.

## Owner rulings outstanding (one line each; detail in the re-entry handover §5)

**Security S3's items (2026-10-20), still open:** (a) a LAN DNS record for `yokohama-test.moore.nyc` — the LAN resolver is authoritative for `moore.nyc` and answers the new name with nothing, so LAN devices off WARP cannot reach the test host (Cloudflare's resolver answers 104.21.92.62 / 172.67.187.23); (b) the off-WARP reading at the test host — a device off WARP meets `bdvc.cloudflareaccess.com`'s login (the one reading of S3's Task 7 only you can take); (g) your own browser reads, asynchronous and welcome: a creator's reopen of a real joiner's seat and D2's 429 on production (its door is WorkOS, so the window's curl could not sign in), the NAS's next hourly sweep line, and the update reading on a pinned browser.
**The public release's items (2026-11-10), in the browser:** the social preview upload (Settings → General; the door's picture, S4a's close §6), GitHub's two e-mail settings (Settings → Emails: "Keep my email addresses private", "Block command line pushes that expose my email") and content reports (Settings → Moderation → Reported content). **Readings the window did not take, welcome when you have them** (the close's §7): a second signed-in address at source `workos` for the lane document's rows 108 and 119; a browser holding the previous release for W-3's install and its anonymous upload; the seat-aware admission at a table's watchers' figure (several tabs as a stranger); the edge's `/api/me` leg at the stale-client shape (D32); the NAS's next hourly sweep line; the private test Worker's health over WARP (the lab's issue #2). **Security S4a's item (2026-10-24):** the second workstation's recipe, the lab as its push default and the no-reply author address (S4a's close §6 and §10; the lab's issue #2).
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
8. D10's NAS sidecar sync — run once at Block B (2026-11-10: three folders, 1,256 files, the byte totals equal on both sides); making it a standing habit is the lab's issue #1.
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
- Web lane: [`web-lane.md`](web-lane.md) (the lane's operating record; §11 its records of record); the player-facing guides [`web-guide.md`](web-guide.md) (the browser client) and [`computer-player.md`](computer-player.md) (the bot); the security project's last close [`plans/2026-10-26-security-s4b-s4c-close.md`](plans/2026-10-26-security-s4b-s4c-close.md) and its plan [`plans/2026-10-26-security-s4b-s4c-plan.md`](plans/2026-10-26-security-s4b-s4c-plan.md), beside `docs/security-ledger.md`; then cycle F's plan [`plans/2026-09-30-web-frontend-m6.5c2f-plan.md`](plans/2026-09-30-web-frontend-m6.5c2f-plan.md) and F's close [`plans/2026-10-01-web-frontend-m6.5c2f-close.md`](plans/2026-10-01-web-frontend-m6.5c2f-close.md), beside E's [close](plans/2026-09-30-web-frontend-m6.5c2e-close.md) (the last two rows of `web-lane.md` §11); operations `deploy/RUNBOOK.md` (§0–§3 and §5 the NAS) and `worker/RUNBOOK.md` (the Worker; `deploy/RUNBOOK.md` §6 points there).
- Bridge (BGA capture → import): `docs/bridge-setup.md`; ledger
  `docs/plans/2026-08-08-phase3.5b-execution-ledger.md`.
- Everything: `CLAUDE.md`, then `guidingpath.md`'s Current state and the decision log's tail; the docs index [`docs/README.md`](README.md) and the method page [`docs/method.md`](method.md) (how the work is organized).
