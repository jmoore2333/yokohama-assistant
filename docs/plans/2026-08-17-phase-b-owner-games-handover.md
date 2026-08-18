# Phase B — the owner diagnostic games: feedback package + session handover

**Written 2026-08-17 at the close of the owner-games session.** Phase B item 3 (amendment §3 /
D2 layer 1, the two recorded owner games) is **PLAYED and PACKAGED**. This document is the
tracked distillation; the detailed scribe log (event-anchored flags, why-transcript excerpts,
verbatim owner statements) is `docs/temp/owner-games/2026-08-17-notes.md` (gitignored sidecar,
NAS-synced per D10). The next session opens Phase B's ANALYSIS: D7 triage over this package,
the canon batch, and the three dispatchable work items the kickoff already names.

**Build:** master `a106934`, debug profile, WSL box, Forge PTY 200×50. Bot in every game:
`lookahead-v1@v2{b=4,eval_impl=3,k=1,m=8,weights=-7339724794018772210}` (the provisional
default). Scribe method: reconstruct-by-replay — `why n` run against copies of the autosave in
a separate piped process (byte-identical property), never the owner's live PTY.

## 0. Package inventory

- **Game 1** `docs/temp/owner-games/game1.json` — 245 events, owner seat 0 first.
  **Red (owner) 132 — Blue (bot) 51.**
- **Game 3** `docs/temp/owner-games/game3.json` — 192 events, bot moved first (owner kept
  seat 0; start seat 1). **Red (owner) 85 — Green (bot) 49.** Margin half of game 1's — the
  bot paced this game far better (its church line banked church-first, 6).
- `game2.json` = abandoned one-turn false start (Forge idle timeout); ignore.
- The owner's casual side games (two gitignored saves in the repo root) were DISCARDED at
  session close by owner instruction — unfinished, not diagnostic material. F3's opening
  evidence from the newer one (the Textile-Mill lab rush and the −40/−42 Customs+Church
  runner-ups) is banked verbatim in the notes file; nothing else referenced them.
- The scribe notes file (above) — flags with event anchors, both games.

## 1. Findings (event-anchored; routing per D7)

**F1 — Placement tie-mass / value-separation degeneracy: CONFIRMED LIVE, REPLICATED, and
CONDITIONAL** (branch 0; the amendment D11 debt-list item "Placement-root value separation").
G1 e113: 139 candidates, 2,468 successors (uncapped, counters healthy — not starvation), **36
tied at the top**, pick = keyed draw. G3 e63: same class, **26-way tie**, second seed. The
counterpoint that makes it fixture-design gold — g3 e93: "Place 2 in Church" won by **+116**
over a runner-up wall tying at −116. **Separation is conditional: placements separate when
they enable a feature-visible scoring line inside the searched turn, and collapse into
tie-walls when they don't.** Rooted-fixture candidates: (g1, e113) and the (e63 vs e93) pair.
Scribe prediction for the mechanical separator: persists at larger beam (representation).

**F2 — The one-sided horizon (owner: "a failure of the imagination")** (branch 3; the D3 lane
is pre-ruled in-scope). G3: the owner's Language School buy, goods, and both port-taken orders
were PUBLIC (only starting orders are hidden) — a computable threat — and the bot's
self-read priced none of it: **+336-era → +36 at the owner's double fulfil (e106–107) → +194
after its church run (e144)**. Mechanism: S1 values its own hand-over snapshot; K=1 samples
hidden cards, not opponent behavior; no feature reads prospective opponent fulfilment.
Consequence: lead assessment systematically optimistic under visible incoming scoring — which
corrupts family 2's accelerate-when-ahead call (see F9). The cleanest branch-3 routing
evidence either game produced.

**F3 — Tech abilities are unrepresented: techs are majority chips** (representation; owner
hypothesis confirmed against the implementation). No ability exists as a feature. Exhibits:
g1 e10 Brickyard bought with its customs rider unpriced (`customs_bonus_now` 0/0 in its own
table); g1 e129–131 Gaslight = pure majority-tier recovery (the shophouse rider invisible; no
pipeline followed — e145/e213 EA visits took assistants only); g3 e3 **the Ball-vs-Patent-
System KEYED-DRAW COIN FLIP** — both cards valued identically, luck handed it the order-synergy
card, and it played a church line anyway. **The 100% Lab-rush opening:** games 1, 3, and three
casual games all open with the Laboratory rush (game-4 data: lab line −0 vs Customs+Church
alternatives at −40/−42 — real margins, real preference; fourth different card, same move).
The opening cannot read achievements or synergies, so the one feature-visible chip always wins.

**F4 — Money and basic goods are not features, and it shows** (representation; T7/T8 measured
context — money signal late-only, not admitted). Turn-1 lab surcharges (+2 yen, three games)
are literally costless in its model; production lines are near-invisible; fees paid to
opponents cost nothing. The owner's cash/tempo judgments are unrepresentable, not merely
disagreed with.

**F5 — Recovery's hand terms make do-nothing competitive** (representation, calibrated). With
`assistants_in_hand_core` at weight 30 (the table's largest), Return-President-to-hand values
within ~one move of real-but-weak moves (g3 why143: −146 beside EA −136 / Bank −150, far
under the church walk). When no strong line exists, do-nothing joins the top tie — g1 e228:
its final move was a president recall TIED at margin 0 with real moves, keyed-draw picked, in
a position that was plausibly genuinely dead (0 yen, 0 imports, nothing fulfillable). Second
dead-final-turn exhibit: g3 e177–183. Family 1's harness adjudicates "genuinely dead" by
exhaustive enumeration — both roots are candidates.

**F6 — The tech-majority step, decoded and corrected in-flight** (fact base for canon work).
`tech_bonus_now` is tiered **first 10 / second 5** with the dummy at 7.5 competing and
CONSUMING a place when it leads (eval.rs ~1440). The g1 arc: Brickyard = second (5) → owner's
Newspaper took second (0) → Gaslight retook second (5) → Stagecoach passed the dummy into
first (10). Every purchase moved a real tier — and the owner predicted the recovery arc
("it could just get another card") four turns before it happened.

**F7 — Stage vs felt-lateness** (instrument note, not a defect). The stage proxy is
clock-based and was CORRECT each time checked (500000 = church 2/4; 750000 = 3/4); the
owner's "getting late" was score-tempo. In-session scribe initially mis-hinted miscalibration
and corrected itself — do not conflate with the R0 self-play length finding.

**F8 — Family 4 fired GREEN, and T3 got its first on-policy contact** (positive). G1
e195–200: produced to 7 seafood and claimed the SAME turn, beating the owner (who claimed the
6 tier next turn) to the 8-VP first tier. The why-200 table shows the post-claim filter
behaving exactly as designed: `achievement_progress_sum` 6→1 at the claim, opponent's leg
untouched. The amendment notes T3 shipped with no on-policy evidence and first contact
expected via family 4 — this is that contact.

**F9 — The closing synthesis: correct family-2 policy on family-3-corrupted inputs.** G3: the
bot drove the ending clock all game (3 church pawns, church-first banked, self-read +194,
genuinely ahead 35–31 on the board) — acceleration-when-ahead is canon 15's own rule, and
under its lens the call was right. Its lens omitted the owner's visible pending ~50 points,
so it accelerated a game it was losing. The owner's finish demonstrated the blindness twice
over: the Language School yen rider paying on his fulfil, then 4 imports → customs space 7
for +22 VP — the "huge points bonus" his game-1 assessment had named, invisible to the bot
both as a plan and as a threat. Beside it, g1's contrast: base VP flatlined at 27 from ~e163
to the end while it chipped majorities. Owner's summary sentence for the record: **"ending
the game when you know you're going to lose on the next turn is a bad idea."**

## 2. Per-family impressions (owner-final)

1. **Final round:** dead feature-churn both games; likely genuinely dead positions (harness
   to adjudicate). "No final-round intent, but some marginal intent with progressing the
   game, even if it can't really prove why."
2. **End-trigger timing:** drove the clock believing it led — correct under its lens, wrong
   in truth. The church momentum had real pressure value ("forces me to focus my engine"),
   and three visits is a strategy — "a dangerous one if I'm going to blast by it."
3. **Race awareness:** public-move awareness works; prospective pricing absent. Owner
   priority: "don't overindex the majorities — the key is indexing what your opponent is up
   to," acknowledged as possibly beyond the current model class (the D3 lane exists).
4. **Achievement proximity:** valuable and it works; good players build their arc around
   achievements; 2P C-achievements are rare.

**The owner's closing verdict (added at his correction of the scribe's wrap-up, 2026-08-17):**
"explained or an honest coin flip" is NOT a satisfactory account of the play — a coin flip
*to the engine* is still a real deficiency at the table: pulling back a lone assistant while
holding others (the g1 recall class), taking whichever technology card the draw hands it —
those are deficiencies, not neutral facts, however honestly the instrument discloses them.
The forward momentum is real and scores points (pushing orders, pushing the church — "not
nothing"), but **playing to win requires the other awareness: not just the other player, but
how to STACK these things looking forward more than a single turn.** This is the owner's
summary of the controller's distance from the mission bar's own words ("provably plays to
win"), and it binds F2 (opponent awareness) and F3/the horizon (multi-turn composition)
together as the two halves of that distance.

## 3. Canon material (owner statements, pending the formal batch)

- **Majority participation value**: a majority ≈ one good turn of value, AND participation
  itself has value beyond winning — not taking a church space means the opponent gets 6 and
  you forgo 3, a doubled swing "in a binary game"; playing the dummy for second place is
  real value. Customs exhibit: owner 8, bot 0.
- **Tech abilities are situational** — the catalog with caveats is in the notes file
  (Exposition's flat 10 minus the track-premium caveat; University's 1-yen assistant;
  Winery's 3-VP-per-church-action — 9 VP against the bot's three visits had it been out;
  System of Stock's liquidity floor). Compounds with achievements (4-techs, 5-orders) into
  22-point-class swings. "Not a clean mapping all the time."
- Drafted questions carried from in-game: recoverable majority flips (full swing vs
  discounted); tech-ability prospective value vs majority-chip pricing.

## 4. The D2 ask — ANSWERED (owner ruling)

**One owner game per MAJOR merge** as a standing cadence — major merges only ("slight tweaks
with these sorts of core misalignments are not something I'm likely to detect"). Budget it in
phase planning from here.

## 5. CLI backlog (branch 4 — explicitly NOT blocking Phase B)

1. Forced-move `why` should lead with "only legal option — nothing was weighed" (owner hunted
   for justification in a 1-candidate table, twice).
2. `you`/`best opp` first-contact aid: surface nonzero-product rows (they are the move's
   whole story).
3. A browsable per-area action-payout reference (`areas` view; previews only price offered
   actions).
4. "acquire X, **+2 yen**" reads as a gain; it is the slot surcharge PAID — reword.
5. The end-trigger firing needs a loud one-time banner (owner missed it live; the strip's
   FINAL ROUND marker is quiet).

## 6. BGA-lane decisions recorded this session (owner rulings)

- **Bi-directional (writing into BGA): evaluated and REJECTED** — reverses the reviewed
  read-only fence, risks the capture account, high protocol fragility, near-zero payoff.
  The conduit stays read-only forever.
- **The read-conduit extension (perfect-information disclosure at import + a
  counterfactual-at-event analysis command + an interactive follow mode): DEFERRED** until
  the why/suggest surface settles under Phase-B-driven development. Design thinking preserved
  in brief: disclosure enters as owner answers on the `--achievement` pattern (opponent
  starting orders, validated at fulfilment, view-honest — `observe` re-masks them); the
  analysis command roots the contestant chain at `record::state_at(n)` under a
  did-not-make-this-move header; sync-on-Enter beats background redraw for a v1 interactive
  follower. Pick up from this paragraph when the lane reopens.

## 7. Next session (Phase B continues)

Read order: `CLAUDE.md` → guidingpath's 2026-08-17 row → the kickoff
(`docs/plans/2026-08-12-phase-b-kickoff.md`) → THIS document → the notes file. Then:

- **D7 triage over §1** — branch 0 first (F1's counter check is already done and clean;
  classify F1 defect vs F2/F3 representation-and-horizon routing), the mechanical separator
  on the F1/F5 rooted fixtures once the harness exists.
- **The driven-fixture harness** (kickoff item 1) — now with five session-produced rooted
  fixture candidates: (g1 e113), (g3 e63/e93 pair), (g1 e228), (g3 e177–183).
- **Exploratory probes** (item 2) and **the V2-play counters slice** (item 4) — unchanged,
  dispatchable.
- **The canon batch** — assemble §3 into ruling form for the owner alongside analysis.
- The bar for "coherent play" (D7's negative definition) took no hits on its MECHANICAL half:
  every flagged move was accounted for by `why`, zero mismatch refusals across ~25
  reconstructions in two games plus post-game pulls — a W5 field result about the INSTRUMENT,
  not the play. The judgment half records real deficiencies — read the owner's closing
  verdict at the end of §2 before quoting this bullet anywhere.

*Session hygiene: `.claude/settings.json` (chrome-devtools plugin registration) committed with
owner approval beside this doc; all Forge terminals closed at session end (the owner concluded
his casual games); the casual saves discarded per §0.*
