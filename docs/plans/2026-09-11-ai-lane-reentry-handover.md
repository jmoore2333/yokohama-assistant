# AI lane — re-entry handover (2026-09-11): where the strength lane stands after the web-lane merge

**Written on master at the web-lane merge `abe4eec` (the doc-pass commit that lands this file
follows it).** It is a POINTER, not an authority: every claim below names the document, row or
command that carries it, and where the record is silent the silence is said out loud (§9). The
governing texts are unchanged by this file — `docs/plans/2026-08-11-phase3.5c-scoping-amendment.md`
(the forward authority for the rest of 3.5c), `docs/plans/2026-08-20-phase3.5c-value-architecture-checkpoint.md`
(the V4 forward authority), and `docs/plans/2026-08-30-phase3.5c-v4-close.md` (the V4 record of record).

Why it exists: between 2026-08-31 and 2026-09-11 every session worked the browser lane, and the
owner asked, at the merge, for a deep dive over the 3.5c ledger and the pre-web-lane guidingpath
rows to confirm what the strength lane still owes. This is that dive, written so the next AI-lane
session enters from one page.

## 0. Where the lane stands, at this HEAD

- **V4 — the sighted-evaluator milestone — PASSED AND SHIPPED 2026-08-30** (PR #30 → master
  `35b694d`). `v4-step9-draft1` is `Contestant::PRODUCTION`: the CLI wizard's default bot and
  `suggest`'s advisor; `lookahead-v1` / `greedy-v1` grandfathered byte-identically at V2;
  `SEMANTICS_REV` 7. Released as **v0.5.0** the same day (tag `v0.5.0`, three-platform assets).
- **The bar (map §1.9) is NOT claimed.** V4's pass is the milestone's, not the bar's (close §3.1,
  §3.3's last bullet; checkpoint §10).
- **The bot's bytes have not moved since the V4 merge head.** `git diff 35b694d master --
  crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge` is
  EMPTY at `abe4eec` — the release cut (workspace version 0.3.3 → 0.5.0, a CI fix, README) and the
  web lane touched none of them. A game played against master today is a game against the bot the
  close asked for.
- **The web lane merged 2026-09-11** with no bot behaviour moved — NOT a D2-major merge, so no owner
  game rides it (the 2026-09-11 decision-log row). It is a second frontend over the same engine and
  the same `Session` gate; its own next milestone (M6, "the room") is unplanned and enters from
  `docs/plans/2026-09-06-web-frontend-m6-handover.md`.
- **Counts at this HEAD** (M5, one sequence, 2026-09-11; re-count at your own HEAD):

  | suite | reading |
  |---|---|
  | `cargo test -p yokohama-core` | **1576** + 29 ignored across 21 binaries, + the doc-test (aggregate 1577) |
  | `cargo test -p yokohama-cli` | **850** (822 lib + 9 `bga_import` + 18 `guide_sync` + 1 dogfood) + 2 ignored |
  | `cargo test -p yokohama-oracle` | 191 |
  | `cargo test -p yokohama-difftest` | 52 + 5 ignored |
  | `cargo test -p yokohama-core --example eval_probe` | 214 |
  | `cargo test -p yokohama-core --example solver` | 132 |
  | `cargo test -p yokohama-bridge` | 81 |
  | `cargo test -p yokohama-web` | 82 (76 lib + 1 whole-game gate + 5 `wire_fixtures`) |
  | the four `bridge/tests/*.js` | 4/4 |
  | `cargo clippy --workspace --all-targets -- -D warnings` | clean |
  | `cd web && npm run typecheck && npm test && npm run e2e` | clean · 393 in 24 files · 37 in 6 spec files |

  The fifteen-gate release roster was NOT re-run for the web merge (no gated crate moved); the
  standing reading is the V4 merge head's **492.17s M5, 15/15** (close doc §2.7) — the first
  crossing of the ≤420s target, which is why OD-6 is ripe (§5 item 3).

## 1. Read order for the next AI-lane session

1. `CLAUDE.md` (the "Where things stand" paragraph is current to 2026-09-11).
2. `guidingpath.md` — the Current-state update paragraph, then the decision log's tail: the
   2026-08-30 rows from "S13 — THE ROSTER RAN CLEAN" through "THE CLOSE'S TWO DECISIONS: BOTH
   HOLD", and the 2026-09-11 merge row.
3. `docs/plans/2026-08-30-phase3.5c-v4-close.md` §3 (the outcome per the matrix, the two decisions
   side by side, the owed/carried/shelved list, the close's rulings).
4. `docs/plans/2026-08-30-phase3.5c-v4-identity-freeze.md` §1 (the identity in one block) and
   §9.6–§9.9 (the void-unit instrument, the master, the counted attempt's read).
5. `docs/plans/2026-08-20-phase3.5c-value-architecture-checkpoint.md` §7 steps 12–14, §9 (the
   ladder beyond the milestone), §11 (the supersession map).
6. `docs/plans/2026-08-11-phase3.5c-scoping-amendment.md` §2 D1/D6/D9 and §3 (Phases D–F).
7. The ledger `docs/plans/2026-08-09-phase3.5c-execution-ledger.md` — its last eight rows (V4
   steps 11–14) and the "Standing notes and carry-forwards" section.
8. This file.

## 2. The gating item: the owner's `why`-scribed diagnostic game

**What is owed (close doc §3.3, verbatim in substance):** one sim game vs the new default bot at
the merge head, `why`-scribed per the Phase-B pattern, deliberately revisiting placement-heavy (the
e113 class) and end-timing positions. The wizard's default seat IS the V4 bot; no setup beyond
`cargo run -p yokohama-cli`. **Both owner decisions (§3) are ruled TOGETHER after it** (close §3.4).

**What exists on disk** (gitignored `docs/temp/owner-games/`, M5 only — verified 2026-09-11):

| file | state |
|---|---|
| `game4-v4.json` (+ `game4-v4.FINAL.json`, identical) | **COMPLETE** — 247 events, seed 17569738361245370148, seat 0 human / seat 1 `v4-step9-draft1@v2{b=4,eval_impl=3,k=1,m=8,weights=6957643967275421640}`, `semantics_rev` 7 |
| `game5-v4.json` | **IN PROGRESS** — 40 events, seed 15337973953820921787, same seats; the 2026-08-30 session's record says the bot moves first in this one |

Per that session's own record — **not in any tracked document** — game 1 finished **owner 82, bot
81** (the owner's read: "clearly a step in the right direction", won by one after a self-described
endgame mistake; the bot lost base VP by 9, recovered through technology and both achievements,
and conceded both majority tracks). Verify by loading the save before quoting it. **No `why`
scribe log exists for either game** — the only scribe on disk is Phase B's
`docs/temp/owner-games/2026-08-17-notes.md`. Game over is not a `why` refusal, so game 1's scribe
is recoverable after the fact from the finished record (`why <n>` at every bot event).

**What "done" looks like, on the Phase-B precedent** (`docs/plans/2026-08-17-phase-b-owner-games-handover.md`):
the game(s) played (the owner in a Forge PTY — `docs/forge-cli-testing.md`, 200×50 floor; **the
Forge MCP failed to connect in the 2026-09-11 session, so check `http://127.0.0.1:3141/` first**),
the orchestrator scribing by reconstruct-by-replay on autosave copies, an event-anchored findings
package as a tracked dated document, the outcomes recorded in the decision log, and then the two
rulings. Nothing else in the lane opens meanwhile except the OD-0 harness work named in §6.

## 3. The two decisions, and the ladder beyond them

Both HOLD (owner, 2026-08-30, close §3.4), ruled together with the scribe log in hand.

- **DECISION 1 — the D3 opponent-reply DEPTH re-ask over the now-sighted leaf** (close §3.2). D3 was
  ruled IN-SCOPE for 3.5c on 2026-08-12 (amendment D3: a bounded cross-handoff capability inside S1
  is an implementation of "opponent-denial and end-timing depth", not above-bar work), and its
  go/no-go was sequenced AFTER representation (the 2026-08-18 ruling: a negative over a blind leaf
  is confounded). A YES opens the bounded cross-handoff diagnostic inside S1 under D3's six-clause
  instrument fence; the evidence waiting for it is family 3's structural finding, the F2 owner-game
  series, and V4's two cap tails. A NO leaves the lane open-with-evidence.
- **DECISION 2 — the knob-screening GO/WAIT (W6)**, capability-conditioned by the owner's 2026-08-20
  ruling (it waits for a version with a reasonable chance of playing to win); the freeze doc's own
  §9.9 sentence says an identity that beats the shipped bot at 0.69 is the kind of version that
  ruling waited for — the owner's call, never an automatic arrow. Admitted knobs per amendment D8:
  **b and m** (k stays 1 absent a positive world-sensitivity signal; production stays uncapped; the
  cap band 32–128 stays forbidden); tune by knob only, never by editing a default constant (D1's
  preserved-rung rule); a "meaningful improvement" is a number fixed BEFORE screens run; the regime
  check (a counters-only run at the final configuration against the R0-L/V2-play baselines) rides
  it. A GO opens a NEW development identity per checkpoint §9's ladder; a WAIT keeps the shelf.
- **The ladder after either** (checkpoint §9, amendment §3 Phases D–F): screening on the settled
  table → **amendment D1's identity freeze** (the executable identity tuple: the exact
  `contestant_echo` plus `SEMANTICS_REV`, `Rules::content_hash()`, the keep-function id and the
  observation scheme; no "behaviourally equivalent" exemption) → **W7** = the @V1 instrument (two
  of W7's five rungs are INEXPRESSIBLE today — the grammar carries no weight knob by rule L12, so
  frozen `@V1` identities need their own names or an in-crate extension: **blocking implementation
  work before the pre-registration can be authored**, amendment D6) → the OD-5 pre-registration
  (D6's checklist: OD-6 settled there, attempt accounting with teeth — ONE battery per frozen tuple,
  a miss is a result, the lifetime attempt count printed, capped at 3; the containment split; the
  all-sweep contingency with the one labelled sixth cell; post-freeze change control) → the ONE
  confirmatory battery, both arms, once → **Phase F**: OD-8 acceptance play (amendment D9 — the
  format was never set; the inherited rubric line "end-game timing (accelerate ahead, brake behind)"
  contradicts canon ruling 15 and must be re-authored before use) → **OD-11, the bar**, declared by
  the owner alone with the WHAT-IT-DOES-NOT-MEAN language verbatim.

## 4. The D2 cadence, restated so the next merge does not forget it

One owner diagnostic game per MAJOR (behaviour-moving) merge, `why`-scribed (owner ruling
2026-08-17; close §3.3's last bullet: "THE NEXT BEHAVIOUR-MOVING MERGE triggers the D2 cadence
again"). Harness-only, docs-only and frontend merges (PR #23, #24, #27, #29, the web lane) do not
trigger it; V2 (PR #20) and V4 (PR #30) did. The V4 game is the one still owed (§2).

## 5. Open owner items — rulings pending or "at leisure" (the list the dive was asked for)

1. **D3 depth re-ask** — HOLD (§3).
2. **W6 GO/WAIT** — HOLD (§3).
3. **OD-6 roster-target restatement — RIPE.** The ≤420s target is an M5 figure (3.4 spec §8); the V4
   merge head read 492.17s M5 (`evaluator_gate` 268.3s dominant), against the freeze-era 382.86s M5
   and the WSL box's 478–509s. A restatement is the owner's; the figures stay machine-tagged beside
   their baselines (close §3.3; amendment D6 folds it into the OD-5 event, where the W7 roster
   binary cannot land without one).
4. **OD-8 acceptance format (amendment D9)** — never set: game count (the proposal said ≥2, release
   build), rubric, scheduling; the rubric re-authoring above. Owner: "handle at any time".
5. **The CLI branch-4 owner register** (`docs/plans/2026-08-18-cli-branch4-touchup-spec.md` §5,
   open since the 2026-08-19 merge): decision 11 (the 2P opening frame at 51 rows crosses the
   200×50 floor — worth it, or fold orders back onto the President line?), decision 13 (the strip's
   `· FINAL ROUND` suffix contradicts the precise notice during the trigger round — a follow-up
   gating the suffix is recommended), and decisions 4/9 (the 3–4P Orders codes under-delivery and
   the `next take ¥N` yield policy at the swept corner).
6. **The 5-power port-bonus timing divergence** — BGA books the +3 VP at president PLACEMENT, the
   engine awards it inside the port action at the TAKE (engine bytes verified; BGA side inferred
   from the captures). Ruled 2026-08-23 a D-row candidate + phase-review item AFTER V4; the owner
   researches the rulebook side (the ledger's game-3 importer lane row).
7. **The sim-seat option** — a sim-playable named-identity bot seat (wizard surface + resume path +
   both guide formats in one `guide_sync`-enforced commit); routed at V4 step 8 T3, re-surfaced at
   the S11 freeze pause, "unfilled again, stays open". The natural home for the "difficulty levels"
   the CLI v1 deliberately left out.
8. **T2a-R2 (V4 step 9)** — the four M-PF record roots are FORCED one-candidate positions; if the
   fork's placement ranking is wanted as a step-9 record, that is a registration addition. Minor.
9. **D10's NAS sidecar sync** (`\\192.168.1.173\yokohama` for captures, `docs/temp`, memories; a
   credential "to be set up, where to store it to be decided") — **no row records it done**; the NAS
   is reachable (the web lane deploys to it through Portainer), the sync's status is unknown to
   this dive.
10. **Parked, deliberately, by earlier rulings** — the BGA read-conduit extension (DEFERRED,
    design thinking preserved in `docs/plans/2026-08-17-phase-b-owner-games-handover.md` §6);
    CLI-as-BGA-companion awareness (PARKED 2026-08-09, its own future PR); the tech-ledger's 16
    still-unobserved titles (W2 T3: 14 rows shipped; more BGA replays fill it, one small PR each).
11. **The two web-lane items the AI lane inherits** (guidingpath's 2026-09-05 rows): the **V5
    notes** — a SIGHTED Keep (`score_starting_order` reads only the card while the set table shows
    port, goals, lab and board at the choice) and PER-GOOD requirement weights (`KEEP_REQUIREMENT`
    prices every required good at 8; copper and silk are harder than tea and seafood — owner,
    2026-09-04); each is a bot-behaviour change owing its own pre-registration. And the **draw-order
    note**: the engine deals before it fills the ports, the rulebook fills first — equivalent in
    distribution, a `SEMANTICS_REV` bump for no rules fidelity, ruled NOT done, M7 at the earliest if
    ever. One unverified question the corrections wave raised and did not touch (spec R3 forbade a
    CLI change): does the CLI wizard's setup screen show a bot seat's two dealt cards?
12. **The V5 ship's one CLI line — DONE 2026-10-01** (added 2026-10-01, cycle F; R-F4; landed the
    same day by `docs/plans/2026-10-01-cli-grandfathered-identity-hotfix-spec.md`): a game opens under
    any build whose CLI resume reconstructs its exact recorded bot; a V5 build plays V4 games on the
    web exactly when it still derives V4's exact descriptor, `LOOKAHEAD_IMPL_VERSION` and
    `EVAL_IMPL_VERSION` unmoved — a weights-only V5; the admission is in place since 2026-10-01
    (`GRANDFATHERED_NAMED_IDENTITIES` in `crates/yokohama-cli/src/bot.rs`), and a re-point appends the
    identity it ships there, never removing one; a V5 that moves either refuses V4 games as DRIFT on
    the web by R-F4's own rule, since no build then reconstructs the exact recorded bot. And a NEW
    `Contestant` or `NamedIdentity` variant still needs its one-line label in
    `crates/yokohama-web/src/bots.rs` (`docs/web-lane.md` §7 and §15).

## 6. Carried instruments and rules (nobody has to re-derive these)

- **The step-9 self-play termination-health gate** — Wilson 95% UCB ≤ 1.0% on the candidate's
  self-play cap rate, for every future cycle (A-S11-1 item 6, owner-signed; draft 1 read 0.30%
  [0.14%, 0.65%], V2 0 in 2,000).
- **Identity-critical reads on CLEAN release builds** (`rm -rf target/release` — a scoped
  `cargo clean -p yokohama-core` does not remove release example binaries on the M5; the stale-target
  class bit three times). Register readings BEFORE results; masters committed before use.
- **The two cell-runner print NOTEs** — `x_explanations` and `applied_by_seat` in `arena cell`
  (S11c's read-surface gap; discharged by completion + pins that time; still owed as a print).
- **The first-mover-share observation** — self-play seat-0 share 65.5% at S11c, a later
  diagnostic's question, direction-free.
- **The K decision stays closed at k = 1** unless the armed tripwire fires: a nonzero weight on a
  world-sensitive candidate (X1/X2 are the suspects, at weight 1 EARLY in draft 1) arms its own
  raw-value invariance in the leakage determinization cell; a red there is checkpoint §4.3's
  re-opening input, routed to the ledger, never silenced. No red is recorded.
- **The exact-adjudicator lane** (PR #29; `docs/plans/2026-08-20-exact-adjudicator-results.md`
  §10): family 1's positive arm is retired to exact verdicts (ordinals 5 EXACT-PASS; 7, 9, 13, 15,
  17 EXACT-FAIL; 16 BUDGET-REFUSED at the 4,000,000-entry memo cap — a structural fact about the
  position); the attribution rider read INVARIANT at 7/9/13 and BUDGET-REFUSED at 17, so the arm's
  FAIL root is EXACT-FAIL at 20/20 and un-attributed; the nine control-arm roots are out of scope
  by the Phase-B predicate; **a trigger-round instrument is NOT proposed** (its feasibility was not
  priced). The worktree `../yokohama-assistant-adjudicator` is fully merged (0 commits ahead of
  master, 362 behind) — removable at the owner's word.
- **Smaller carries**: the padding-pin enumeration gap (`V2_REFIT_PADDED` absent from it);
  `minimize` on bare `decide` as a future ask; the s12_read parser's cosmetic double-echo
  (session-local script); Phase C's SHOULD-1 (the family render printing the generating ECOLOGY id
  — not confirmed discharged by this dive; check `run_family_block`'s render before assuming);
  T4-review NOTE-4 — no 3–4P adjudication has ever run, and the lane is 2P by charter (map §1.9:
  "provably plays to win, two-player").
- **Two debt items, dispositioned rather than open**: Placement-root value separation (the V4
  thesis — e113's 36-way tie resolves under the sighted table, +2 vs V2's −1 at draft 1's read) and
  terminal-margin awareness (V3's C1, washed at L2 as aggregate-margin double-pricing).

## 7. The V5 shelf (evidence, owner-ruled; entered only through a future cycle's pre-registration)

- **The produce-and-hoard self-play trait** — the locked identity deadlocks in ≈0.3% of self-play
  games (6/2,000; V2 0/2,000; never against V2 or the refit in 1,000 games each). D1 named the loop
  (place → move → produce → skip everything, ~7 decisions a turn, five-figure hoards, no clock ever
  closing); the late `assistants_in_hand_excess` 2 → 8 repair is confirmed sufficient BY THE LETTER
  and the single-world flip is NON-SPECIFIC (four other late zero-outs also terminate it) — a
  rate-level mechanism test is the owed instrument. Exhibits: the two counted cap tails at S12
  (ordinals 40/41), the f2 ordinal-40 sole-dissenter (−89.5), **and a fourth sighting from the web
  lane**: its whole-game server gate hit the 20,000-input cap at turn 7,295 with the bot hoarding
  ¥7,273 (twice, then 660 consecutive passes — the M5.5 close's §7; "a 2P game the bot cannot end").
- **Opponent latent-burst projection** (O3 removed for lack of ORGANIC evidence at V4 step 6 — the
  owner's hold-for-position game hides the strictly-completable instant inside the holder's own
  turn; inputs: the shipping O1/O2/O6 columns; search seed: `mcts-v1`'s in-tree opponent-reply
  channel) and **held-icon tempo** (O6's phenomenon — needs a HOLD-EXPRESSING ecology, the next
  cycle's corpus requirement).
- **The sighted Keep** and **per-good requirement weights** (§5 item 11).
- **The gap-0-credit proximity repair** and **a nonlinear / deficit-keyed margin shape** (the V3
  close's two named future candidates, each through its own ladder).

## 8. Escrow and master accounting at this HEAD (close doc §3.3, unchanged since)

BURNED: the three released post-fix arms in full (f2-pos, f3-ctrl, f4-pos at ordinals 36..41).
UNTOUCHED: the three unreleased post-fix arms (end-trigger control, exclusive-race positive,
achievement corner), every final-controller tier (42..48 / 29..32), family 1's 26..32. Masters:
`0xcf725598e9feb81f` spent and firewalled; `0x0c064c53e8a76b28` burned, never runs again; the A3E1
block masters live in the frozen declarations; the 9999-convention scratch masters recorded and
spent as scratch. The sealed invalid-run figures stay sealed and superseded.

## 9. What the record did not carry before this pass (found by the dive; what was fixed)

- Game 1's result and game 2's existence lived only in gitignored saves and a session memory —
  now stated in §2, still owed as a tracked findings package (§2's "done").
- `guidingpath.md`'s Current-state section and its 3.5c roadmap note had stopped at the V2 arc
  (2026-08-11); Phases A/B/C, V3, the checkpoint, the adjudicator and V4 lived only in the decision
  log and the ledger. **Fixed in the doc pass that lands this file** (a dated Current-state
  paragraph; a 3.5c landed note through V4 naming what remains).
- `CLAUDE.md`'s "Where things stand" was dated 2026-08-30 and named no web lane; README's Hosting
  section said "nothing is deployed yet" after eight deployments. **Fixed in the same pass.**
- The web-lane docs said "branch `web-frontend` only" at seven sites. **Fixed.**
- Not fixed, by design: the ledger has no row for the v0.5.0 release (CLAUDE.md and README carry
  it) and no row for the owner's game 1 (it is the owner's action, recorded when its package
  lands).

## 10. Worktrees and branches after this merge

- `web-frontend`: merged (`abe4eec`), pushed, preserved; its worktree removed after the gitignored
  artifacts were carried into `docs/temp/` (sdd-archive, web-review, webresearch).
- `phase3.5c-exact-adjudicator`: fully merged (PR #29); worktree `../yokohama-assistant-adjudicator`
  still checked out at its tip — nothing unmerged in it.
- Every phase branch since #16 is preserved on origin, per the standing convention.

## 11. What this handover does NOT claim

- No strength figure is new here; every number is a quotation with its source named. The bar is
  not claimed, V4's pass is the milestone's, and nothing about the web lane bears on either.
- The counts in §0 are this HEAD's readings and a lineage, not an authority — re-count.
- Game 1's score is the session record's, not a tracked document's, until the package lands.
- Nothing here is a ruling. The two HOLDs, OD-6, OD-8 and the branch-4 register are the owner's.
