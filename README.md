# Yokohama Assistant

A Rust engine, live-game CLI tracker, and verification stack for the board game **Yokohama**
(Hisashi Hayashi, 2016), built toward a computer player that provably plays to win.

> [!IMPORTANT]
> **Testing the CLI? Start with the [HTML CLI guide](docs/cli-guide.html).** Ready-to-run
> builds for Apple Silicon macOS, Ubuntu x86_64, and Windows x86_64 are on the
> [Releases page](https://github.com/jmoore2333/yokohama-assistant/releases) — current release
> **v0.5.0**, cut from the Phase-3.5c V4 close and the first release whose default bot is the
> sighted evaluator `v4-step9-draft1`; each download also includes an offline copy of the guide.

**Priority order:** 1) engine-validated move-by-move tracking of live games via CLI →
2) a computer player → 3) computer vision / AR assist (deferred; the original 2025 AR-first
vision lives in `docs/plans/2025-12-07-yokohama-assistant-design.md`).

## See it running

![The Yokohama CLI at 2 players against the built-in bot: the staggered pyramid board on the
left, both Presidents placed; on the right the players panel — RED (you) and GREEN marked [bot]
with its hand hidden — where each seat carries its own Orders line naming the order cards it has
filled, then the church and customs tracks with the 2-player dummy pawn, the three achievements,
and the order still in hand; along the bottom the end-trigger strip reads church 4/4 and FINAL
ROUND, and the message zone greets the reloaded save with GAME OVER — the Church track
filled](docs/images/cli-frame-b4.png)

One of the owner's own finished 2-player games, reloaded — real terminal output (v2 frontend,
200×50). The bot in this capture plays **`lookahead-v1`** over the frozen evaluator-v2 weights,
which was the production default on the day it was taken; **since v0.5.0 the wizard's default seat
is the sighted `v4-step9-draft1`** instead, and the machinery shown below explains either one
identically. It decides from the same masked view of the game a seated player gets: its Orders
line prints `0 in hand (hidden) · 2 filled (O04, O33)`, because which cards a seat has *filled*
is public and what it still holds is not. Every decision it makes can be interrogated after the
fact:

![The why command's output for one of the bot's placements: a header naming the seat, the full
contestant identity string, the event index and decision seed, and the statement that the move
was reconstructed by replay; then two labelled lines of search counters, the move it played, the
six-move line it searched, the term-by-term score table with a marker against each row that
carries a nonzero product and the zero rows dimmed, and the ranked runner-ups it rejected — here
four candidates tied at the top](docs/images/cli-why-counters-b4.png)

That is **`why 178`** asking about the bot's placement: the record is replayed to event 178 and
the bot's own decision function re-run at the same derived seed — so the account is a
*reconstruction the engine can vouch for*, not a stored note — then what the search cost
(139 root candidates, 1,401 nodes, 3,311 scored positions, and the budget and cap guards that
never bound), the move, the line it searched, the arithmetic, and what it rejected at what
margin. A `why` that cannot reproduce the recorded move refuses, naming both moves.
**`suggest`** is the same machinery pointed at *your* seat before you move; both apply nothing
and record nothing.

#### What the V4 bump bought, in one decision

That screenshot is the **previous** evaluator thinking — sixteen features, which was the entire
vocabulary it had. Here is the **shipped V4 bot** answering the same kind of question, a
three-area placement, in one of the owner's own games. Same command, same tables, no image
needed — this is the driver's real plain-text output, reproducible from the saved record:

```console
$ printf 'why 146\nquit\n' | yokohama-cli game.json
─ Why ──────────────────────────────────
Seat 1 (Blue) · v4-step9-draft1@v2{b=4,eval_impl=3,k=1,m=8,weights=6957643967275421640} · event 146
This move was reconstructed by replay under THIS build's consts — evaluator impl
3, v4-step9-draft1 impl v2 — from decision seed 0x60bd1d0c935a80a1, the key the
bot's own pump derived from the recorded stream seed and the event count.
Search cost: root_candidates 186 · nodes 1837 · handles 1837 · route_dfs_nodes
186 · forced_nodes 1630 · leaves 783 · scored 4328 · root_scored 186 · max_line
6 · free_extension_steps 0
Budget and guards: budget_binds 0 · budget_starved 0 · placement_cap_binds 0 ·
ply_cap_hits 0 · free_cap_hits 0
The move it played:
  Place 1 assistant in each of: Fishing Ground (A01), Bank (A04), Employment Agency (A09)

The line it searched, 6 move(s) to the hand-over (the move above is step 1):
  1. Place 1 assistant in each of: Fishing Ground (A01), Bank (A04), Employment Agen…
  2. Move President to Employment Agency (A09) — free
  3. Employment Agency: take 1 assistant (free)
  4. Skip the 5-Power token
  5. Skip the building
  6. End additional-action phase

Score at that point: 628   ·   stage 750000/1000000 ppm (0 = opening, 1000000 = end trigger)
  feature                        you   best opp   weight     product
▶ victory_points                  42         26       30         480
▶ country_bonus_now                2          4       35         -70
▶ tech_bonus_now                   5          0       18          90
▶ church_bonus_now                 6          0        5          30
  customs_bonus_now                0          0       15           0
  assistants_in_hand_core          2          2       23           0
▶ assistants_in_hand_excess        0          5        7         -35
▶ assistants_in_warehouse         10          9      -10         -10
▶ assistants_on_board              5          3        9          18
  shophouses_built                 2          2       18           0
  trading_houses_built             0          0       22           0
▶ five_power_tokens                2          3       12         -12
▶ imported_goods                   0          2        7         -14
  achievement_progress_sum         6          6        3           0
  mercenaries_total                0          0        3           0
▶ hand_size                        0          1       -1           1
▶ production_access                7          0       10          70
▶ track_access                     0          2        1          -2
▶ assistant_areas_distinct         4          3       -4          -4
▶ president_local_access           1          0       -1          -1
▶ bank_access                      3          2       15          15
  employment_agency_access         0          0        1           0
▶ president_reachable_areas        4          0        4          16
▶ money_stock                      6          2       16          64
  trade_goods_stock               12         12       11           0
  building_tokens_ready            0          0        0           0
  held_order_value                 0         12        0           0
▶ order_matched_goods              0          4        1          -4
  executable_orders                0          0        0           0
  flag_icon_marginal               2          4        0           0
  held_order_icons                 0          1        0           0
▶ lab_takeable                     0          4        1          -4
  port_takeable                    0          0        1           0
  matched_production_access        0          0        0           0
  TOTAL                                                          628
  19 of 34 rows carry a nonzero product — they are this move's whole story.

Runner-ups (margin against the suggestion, so 0 means tied) — 186 candidate(s) ranked:
          -5  Place 1 assistant in each of: Fishing Ground (A01), Tea Plantation (A02), Emplo…
          -6  Place 1 assistant in each of: Fishing Ground (A01), Tea Plantation (A02), Bank …
          -6  Place 1 assistant in each of: Fishing Ground (A01), Silk Mill (A07), Employment…
This is the bot's arithmetic, not advice: each row is (your value − the best
opponent's) × the weight for this stage, summed. It read ONE guess at the hidden
cards, and its weights were never tuned on human play.
```

Both are the same shape of decision — a one-assistant-in-each-of-three-areas placement, six
moves to the hand-over — which is what makes them worth putting side by side. Read that way, the
difference is legible without knowing the plan:

- **The table is 16 rows in the screenshot and 34 here** (it says so itself: *12 of 16 rows carry
  a nonzero product* there, *19 of 34* here). Those extra eighteen are the V4 "sighted" features,
  appended after the incumbent sixteen (`Feature::INCUMBENT_COUNT = 16` in `game/eval.rs`), and
  they are what the older evaluator was structurally unable to see: where assistants actually sit
  and what that reaches (`production_access`, `bank_access`, `president_reachable_areas`,
  `assistant_areas_distinct`), and what the economy affords both seats (`money_stock`,
  `order_matched_goods`, `lab_takeable`, `port_takeable`).
- **The older table ties; this one separates.** The screenshot ends with *four candidates tied at
  the top* — the search could not tell them apart, so a keyed draw picked one. Here the top
  choice beats its nearest rival by 5 with the next two at −6, out of **186** ranked candidates.
- **The new rows are carrying real weight.** Of this move's 628 points, the incumbent sixteen
  contribute 478 and the eighteen sighted features contribute **150** — with `production_access`
  alone worth +70 against an opponent sitting at 0.

What that does **not** establish is a strength claim. These are two different positions from two
different games, at different stages of the game (the screenshot is at the end trigger, this one
three-quarters in), so the weights themselves differ and no row-by-row comparison is being made —
they are here to show the *shape* of the change, not to measure it. The measurement is
the pre-registered confirmatory field in the status section below — θ̂ 0.691 against the
previously shipped evaluator — and even that is a head-to-head result, not the project's
"plays to win" bar, which stays unclaimed.

The driver is loud about the one moment a game can be lost by missing: the **end trigger**.

![The end-trigger banner at the moment it fires: the strip along the top reads church 4/4 and
FINAL ROUND, the delta lines below it show Red paying two yen and four goods and placing on
Church faith-8, and then a bordered warning block states END TRIGGERED — the Church track filled,
followed by the rule the engine implements — the current round finishes, then every player gets
one full final round, then the game ends and final scoring runs](docs/images/cli-banner-b4.png)

The banner fires once, on the transition, and states the rule rather than a Rust variant name; a
save reloaded after the fact gets the one-line notice instead (the `GAME OVER — the Church track
filled` line in the first screenshot above). And because "what does this area pay me at the
power I can bring?" is the question a new player asks most, every area's whole ladder is one
command away:

![The areas view: one card per area, each printing its action in a sentence, then the payout at
power 1 through 5+, its 5-Power token, the four printed site rewards, the trading-house VP, and
who is standing there — Laboratory A's industry-value ladder and the Church's faith ladder
beside the Silk Mill, Tea Plantation, Copper Mine and Fishing Ground production
ladders](docs/images/cli-areas-b4.png)

Every figure on that reference is computed from the engine's own reward functions rather than
re-derived in the display layer, so it cannot drift from what the action actually pays.

**[Full `suggest` walkthrough →](https://claude.ai/code/artifact/9dbb3a84-7994-4066-a400-7e83c049a19e)**
— a few minutes of live play against a bot seat (2026-07-27, v0.3.3), captured as the raw ANSI
stream from a 140×50 PTY, with an honest read on both sides: **what held up** (the search's
projected lines matched the played-out game move for move; it found genuine multi-step
combinations; determinism and the no-side-effect guards are real) and **what it flagged** (two
display-layer bugs in `describe.rs` — a Laboratory surcharge rendered as a reward, and production
previews that ignore the acting player's techs — plus the evaluator's structural positional
blindness, and the fact that the *playable* bot was, at that date, the 3.2 weighted-random rung).
Every finding is display-layer; the engine did the right thing in each case. Three of them have
since been retired: the Laboratory surcharge now prints as a `slot fee: 2 yen` clause instead of
reading as a reward, the weights are **evaluator v2** (frozen 2026-08-10), and since Phase A's W5
(2026-08-12) the playable bot IS the search — a wizard bot seat runs `lookahead-v1`, the same
controller `suggest` consults, and the **`why`** command replays and explains any bot move after
the fact.

That link is a hosted copy, so it opens and renders in a browser. The source of truth is
[`docs/suggest-walkthrough-2026-07-27.html`](docs/suggest-walkthrough-2026-07-27.html) in this
repo — GitHub shows `.html` as source rather than rendering it, so read that one by opening the
file locally.

## Status (2026-08-30)

The engine plays complete, rules-correct games at 2–4 players and has been verified
**differentially**: an independently-authored rules oracle — written from the rulebook and
knowledge base only, never from engine code — agrees with the engine on every legal-move set
(240/240 games after fixes), every president-route label (7,272 decision points plus
adversarial worst-case batteries), and every final score including the official tie-break
(540 games). The campaign (Phase 2.9) found and fixed four engine bugs and one oracle bug that
self-play fuzzing structurally could not see, hardened every raw apply arm against ill-formed
payloads, mutation-proved the invariant suite, and closed all four rules ambiguities it
surfaced with owner rulings. Full report:
[`docs/plans/2026-07-20-phase-2.9-final-report.md`](docs/plans/2026-07-20-phase-2.9-final-report.md).

**The computer player ladder (Phase 3) is deep into its final sub-phase**, governed by the
accepted map [`docs/plans/2026-07-25-phase3-map.md`](docs/plans/2026-07-25-phase3-map.md).
Sub-phases 3.0–3.4 (all merged to master, `SEMANTICS_REV` unmoved by any of them):

- **3.0 — honest observation** (PRs #6–#7): a seat-relative `PlayerView` masking the five-member
  hidden set, plus a conservation-consistent belief sampler — a bot takes a view, never a raw
  `GameState`.
- **3.1a/b — the bot-facing action interface** (PRs #8–#9): factored lazy move families (the
  442,368-move retreat worst case served one candidate at a time in ~5 allocations), an exact
  allocation-free route enumerator, and cheap single-candidate legality.
- **3.2 — the arena** (PR #12): the permanent strength court — paired seat-swapped cells, honest
  bootstrap intervals with a typed zero-width refusal, the family-weighted rollout policy (also
  the fuzzer's), and the self-play training corpus with full per-seat provenance.
- **3.3 — the first purposeful bot** (PR #13, merged 2026-07-27): **evaluator v1** (sixteen
  view-measurable features, transparent stage-interpolated weights, exact-sum explanations),
  the **Greedy-v1** and **Lookahead-v1** rungs (turn-boundary depth semantics, settled by a
  pre-registered experiment), scenario suite v1, and CLI `suggest my move`. **The map's gate was
  declared met on both binding arms**: Lookahead-v1 swept all 200 paired units against the
  family-weighted rung on the committed regression master AND on a freshly drawn OS-entropy
  master (rule-of-three bounds 0.985, VP margins +67/+68). Close report:
  [`docs/plans/2026-07-27-phase3.3-close-report.md`](docs/plans/2026-07-27-phase3.3-close-report.md).
- **3.4 — the discriminating experiment** (branch `phase3.4-search-arch`, 2026-08-05): a UCT
  contestant (`mcts-v1`) and a shared **scored-successor budget** that caps either architecture on
  one currency, then a **pre-registered** matched-budget experiment — the decision rule committed
  before any cell ran. Seven cells, 2,400 games, one attempt: **MCTS-v1 is NOT adopted; the 3.3
  turn-boundary shallow search is retained, on the merits.** Both node-matched arms landed entirely
  below the null (θ̂ 0.045 and 0.050, the second on a freshly drawn master) and so did the
  wall-matched cell against the shipped rung (θ̂ 0.070 at a 1.033 wall ratio) — the map's
  "strength-per-node and strength-per-wallclock can rank differently" was tested and here they
  agreed. MCTS-v1 stays as a labelled diagnostic under release gate #14. Decision record:
  [`docs/plans/2026-08-04-phase3.4-architecture-decision.md`](docs/plans/2026-08-04-phase3.4-architecture-decision.md);
  close report:
  [`docs/plans/2026-08-05-phase3.4-close-report.md`](docs/plans/2026-08-05-phase3.4-close-report.md).

**3.5a — the CLI redesign** (PR #16, merged 2026-08-07) rebuilt the driver into the v2 frontend:
a 200×50 raised-floor frame with a persistent dashboard, full words, a visual card index as the
canonical identification mechanism, guided pickers and menus everywhere, readline editing, and
full-screen views — with the piped protocol byte-identical and the engine crates untouched.

**3.5b — the BGA→CLI bridge** (PR #17, merged 2026-08-09): a read-only dev-mode Chrome extension
+ localhost relay + importer that turns the owner's own BoardGameArena games into
engine-validated records — cards identified by their unique requirement/reward vectors, hidden
information kept hidden, every unrecognized shape a typed refusal rather than a guess. **The
re-scoped Phase-2 live-tracking gate is CLAIMED on real table 896157897**: 145/145 BGA moves
replay engine-legal from move 1, 103 counter checkpoints green, both final scores matching BGA
exactly (62 == 62, 115 == 115). Along the way the bridged games surfaced five rules questions,
each closed by an owner ruling sourced from the physical rulebook and BGA's observed behaviour
(**rules-parity revs 6 and 7**, PRs #18/#19 — `SEMANTICS_REV` now 7, with two deliberate,
recorded BGA-parity divergences).

**3.5c — the strength lane — is where the work now lives.** So far, all merged:

- **Evaluator v2 frozen and shipped** (PR #20, 2026-08-11): three pre-registered iteration
  rounds over measured self-play corpora produced thirty-two re-priced weights;
  the pre-registered close criterion held — `lookahead-v1@V2` vs `@V1`, 2P, N=200 units,
  θ̂ 0.6775, bootstrap [0.6275, 0.7250] entirely above the null — and the owner signed it off
  under a deliberately **narrow claim**: a materially stronger successor for the retained 2P
  turn-boundary controller at shipped knobs. **V2 shipped ≠ the bar.**
- **The 3.5c scoping amendment ratified** (2026-08-12,
  [`docs/plans/2026-08-11-phase3.5c-scoping-amendment.md`](docs/plans/2026-08-11-phase3.5c-scoping-amendment.md))
  — eleven owner rulings that fix the route from V2 to the bar: development-field tuning →
  a frozen production identity → ONE confirmatory five-rung experiment, with a two-layer
  strength-reality checkpoint in between and hard attempt-accounting rules throughout.
- **Phase A of that route is complete** (PRs #21/#22, 2026-08-12): CLI bot seats are typed to
  `Contestant` and the wizard's default bot **plays the production-intent `lookahead-v1`**;
  the new **`why` / `why <n>`** command replays any bot move to its exact decision point and
  prints the same explained account `suggest` gives — refusing honestly whenever the
  reconstruction cannot vouch for the recorded move; and the BGA importer learned the Tram
  vocabulary (evidence-first, from the captured payloads), regenerating the comparison corpus
  at rev 7.
- **Phase B is complete** (PR #23, 2026-08-18): a driven-fixture harness that re-runs any
  recorded decision through the search at chosen knobs, a re-derived triage of the owner's two
  diagnostic games, a counters baseline for V2 play, and a readout on the four strategic
  families. Its measured result is the useful one. The bot's tie-mass defect turned out to be
  **two** mechanisms rather than one — one exemplar root still ties with the beam widened to 8
  and 16 (a *representation* gap: the evaluator cannot see the difference) while another resolves
  into a single best line at beam 8 (a *horizon* gap) — and three positions the owner had flagged
  as bad play measured as genuinely dead, no candidate winning in any replication, where
  indifference is the correct answer rather than a defect.

**Phase C followed** (PR #24, 2026-08-18): the four driven strategic families frozen, run, and
routed. Two evaluator cycles have since run on top of that harness:

- **The V3 mini-cycle closed NULL** (PR #27, 2026-08-19). Both ruled candidates washed at their
  pre-registered gates — terminal-margin at L2, `achievement_race_proximity` at L3. The probe
  instruments and the evidence merged; **no behaviour changed.** A cycle that measures its two
  candidates and ships neither is the process working rather than failing.
- **V4 — the sighted-evaluator milestone — PASSED AND SHIPPED** (PR #30, merged 2026-08-30;
  record of record
  [`docs/plans/2026-08-30-phase3.5c-v4-close.md`](docs/plans/2026-08-30-phase3.5c-v4-close.md)).
  The candidate `v4-step9-draft1` is an evaluator whose leaf can finally *see* — spatial/action
  affordance and order/economic opportunity, computed for both seats — where its predecessor
  scored positionally blind. In the one pre-registered confirmatory field, on the one attempt the
  freeze allowed, it cleared **jointly**: θ̂ **0.691** [0.667, 0.715] against `v2-padded` (the
  shipped behaviour) and **0.885** [0.867, 0.902] against `v2-refit-padded` (the refit control),
  with a healthy self-null (0.498 over 647 read units). The escrowed driven-family blocks were
  then re-read as a **regression guard an arena win cannot overrule** — GREEN: three non-blocking
  VOIDs, zero failed admitted roots, one clean 0/20 exhibit per guard. The fifteen-gate release
  roster ran as one uninterrupted sequence, 15/15. It ships **as itself**: `Contestant::PRODUCTION`
  *is* the confirmed named identity — the CLI's default bot seat and `suggest`'s advisor — with
  `lookahead-v1` and `greedy-v1` grandfathered byte-identically at the V2 table, so every figure
  ever recorded against those names stays comparable. **The bar is not claimed.**

**Where the work stands right now — the owner gate.** The milestone's close puts two decisions
side by side, and as of 2026-08-30 **both are deliberately on HOLD**, waiting on one cheap thing:
the owed `why`-scribed diagnostic game against the new default bot at the merge head, played by
the owner and deliberately revisiting placement-heavy and end-timing positions. That scribe log
then informs both rulings at once — the cadence's whole point being that watching the shipped
player actually play costs less than either wrong turn.

- **D3 — the opponent-reply depth re-ask.** Whether to open the bounded cross-handoff diagnostic
  inside the shallow search. It was deliberately sequenced *after* the representation work,
  because a negative measured over a still-blind leaf would have been confounded rather than
  clearance. The leaf can now see, so the answer will finally be interpretable either way.
- **W6 — the knob-screening GO/WAIT.** Capability-conditioned by owner ruling (2026-08-20):
  screening waits for a version with a reasonable chance of playing to win. An identity that
  beats the previously shipped bot at θ̂ 0.691 is the kind of version that ruling was waiting
  for — but it is the owner's call, **never an automatic arrow**.

Until those are ruled, the **exact bounded-tree adjudicator** is the one dispatchable lane. Two
further items are carried in the open rather than buried: **OD-6's roster-target restatement** is
ripe — the roster read 492.17s on the M5, its first crossing of the stated ≤420s target,
dominated by `evaluator_gate` at 268.3s (wall figures are per-machine and stay tagged beside
their own baselines) — and a **V5 shelf item**, a produce-and-hoard self-play trait that now
carries three printed mechanical exhibits instead of an inference.

The CLI is tester-ready; start with the linked HTML guide and report any table/CLI mismatch.

Releases track completed phases (**v0.5.0** ↔ the V4 close — the current build, whose default bot
is the sighted evaluator; **v0.3.3** ↔ Phase 3.3 was the previous full cut). Saves and self-play
records are compatibility-keyed to the game-semantics revision (`SEMANTICS_REV`, currently 7), not
the release version (PR #14), so they survive release cuts — though a save stops resuming across a
semantics bump by design, and rev-6-and-earlier saves no longer resume.

## Workspace crates

| Crate | What it is |
|---|---|
| `yokohama-core` | The engine: serde-serializable `GameState` (history-free, cheap to clone for search), pure `get_legal_moves` / checked `apply_move`, seeded setup, `GameRecord` save/replay stamped with `rules_hash`, `semantics_rev`, and a terminal-state digest verified by `replay_strict` — plus the Phase-3 player stack: the honest observation boundary (`PlayerView`/`observe`/`BeliefSampler`), the factored action families (`DecisionHandle`), the arena (contestants, paired cells, corpus emission), evaluator v1 (`game/eval.rs`), and the Greedy/Lookahead search rungs |
| `yokohama-cli` | The game driver: setup wizard → REPL; simulated games and live-table tracking (reconciliation, desync tripwire), save/load/undo, TTY-colored board, a persistent dashboard whose per-seat **Orders line** names the cards each seat has filled, an **`areas`** reference printing every area's payout ladder from the engine's own reward functions, a bordered **end-trigger banner** that fires once on the transition and states the two-phase end rule, the BGA `import`/`--follow` pipeline, and **human-vs-bot play** in sim mode (any seat mix with ≥1 human; a bot seat is a `Contestant` — the wizard default plays the confirmed production identity `v4-step9-draft1`, honest by construction: the agent sees only its seat's view) plus the explanation pair: **`suggest`** — the same search analyzes YOUR seat's view and explains its recommended move term by term — and **`why` / `why <n>`** — any bot move replayed to its exact decision point and accounted for with the same tables, refusing (never guessing) when the reconstruction does not match the record |
| `yokohama-oracle` | The independent referee: legal-move enumeration, final scoring + tie-break, and a prune-free route reference, authored from the rules sources under an audited independence rule (types-only dependency on the engine) |
| `yokohama-difftest` | The harness comparing engine vs oracle: per-decision-point sweeps under two policies, terminal scoring comparison, three-way route-pruning check, self-contained divergence artifacts + `repro` tool |
| `yokohama-bridge` | The BGA capture relay (Phase 3.5b): a localhost-only HTTP sink that appends what the read-only browser extension in `bridge/` observes to `bridge-captures/<table>.ndjson`, plus the feed pre-processor (typed NDJSON reader, payload-identity dedupe, structural supersession collapse for takebacks). Depends on NO engine crate and knows no rules — translating captures into engine moves is `yokohama-cli`'s `import`, deliberately on the other side of that boundary |
| `gen-data` | YAML → Rust generator for the verified card/site data (never hand-edit `game/data.rs`) |

## Download and run

Download the archive for your system from
[GitHub Releases](https://github.com/jmoore2333/yokohama-assistant/releases) — current release
**v0.5.0**:

- `macos-apple-silicon` is optimized for Apple Silicon Macs (M1 or newer).
- `ubuntu-x86_64` is a static binary for 64-bit Ubuntu Linux.
- `windows-x86_64` is a 64-bit Windows build.

Extract the archive, open a terminal in the extracted directory, and run:

```bash
./yokohama-cli
```

or on Windows (PowerShell):

```powershell
.\yokohama-cli.exe
```

The macOS build is ad-hoc signed rather than notarized. If macOS quarantines it, approve it in
**System Settings → Privacy & Security**, or run
`xattr -d com.apple.quarantine ./yokohama-cli` before launching it. On Windows, use
**Windows Terminal** for color output — classic conhost may show raw escape codes — or set
`NO_COLOR=1` to force plain text on any platform.

For the full walkthrough, setup instructions, commands, and troubleshooting, use the
**[HTML CLI guide](docs/cli-guide.html)**. The same guide is included as `CLI-GUIDE.html` in each
release archive.

## Build from source

```bash
# prerequisites: Rust via rustup, then from the repo root:
cargo test -p yokohama-core            # engine suite
cargo run  -p yokohama-cli             # setup wizard → play a game
cargo run  -p yokohama-cli -- save.json  # resume a save (autosaves every move)
```

Fastest playable game: answer the wizard `sim`, `2`, `red`, `blue`, `0`, Enter twice, `0`, `0`,
`confirm`. At the prompt: type an option number, or use the direct grammars — placement
`a 3 7 9` / `b 5`, assistant moves `m 4 7`, retreats `r 3:2 5:1`. Views: `board`, `areas`,
`sites`, `goals`, `tracks`, `labs`, `log`, `help`; plus `suggest` (the bot's explained
recommendation for your seat), `why` / `why <n>` (a bot move, replayed and accounted for),
`undo`, `save`, `q`.
Full tester guide: **[HTML](docs/cli-guide.html)** or [Markdown](docs/cli-guide.md).

**Gates.** Per-edit (debug): the six crate suites (`yokohama-core`, its `eval_probe` example
target, `yokohama-cli`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge`) — plus a
seventh, `yokohama-web`, on branch `web-frontend` only, where the browser lane lives — + the four
`bridge/tests/*.js` node suites (the BGA bridge's browser half has no cargo coverage and would
otherwise never run) + `cargo clippy --workspace --all-targets -- -D warnings`, all zero-warning.
**On `web-frontend` the browser CLIENT adds three more per-edit commands, from `web/`** (M2.5,
2026-08-31 — before it, "the suites are green" was not a claim about the browser at all):
`npm run typecheck` (both TS projects, and the committed wire corpus against the hand-written
client types), `npm test` (118 vitest component tests over jsdom, every payload of them
server-emitted), and `npm run e2e` (7 Playwright tests driving real Chromium against the real
release binary, including a whole 2P game clicked through to a winner). The last needs
`npx playwright install chromium` once; absence is a loud refusal naming that command, and
`YOKOHAMA_WEB_E2E_OPTIONAL=1` turns it into a skip that says out loud it tested nothing.
Pre-merge: all **fifteen** release gates below, measured as ONE uninterrupted sequence at every
close — a sum of per-command times from separate sessions is not a measured total. **Wall
figures are per-machine since 2026-08-11** (two workstations whose profiles INVERT by gate
shape; baselines live side by side and are never compared to each other): **Apple M5 —
382.86s** at the v2 freeze close, under the ≤420s target, which is an M5 reading; **Debian 13
WSL2 (Ultra 7 265K) — 508.9s** at the PR #20 merge head, then **498.5s** (W5 close) and
**511.7s** (W2 close, two gate walls carrying rebuild overhead) — 15/15 rc=0 in every run. The
fifteenth gate is the Phase-3.3 recovery falsifier, DEMOTED 2026-08-09 by owner ruling from the
per-edit tier it had outgrown (it buys ~90–100s off every per-edit core run for ~7s of roster);
the fourteenth is Phase 3.4's architecture gate, whose addition restated the roster target
≤360s → ≤420s. This table is the binding list — if it and CLAUDE.md's two-tier policy ever
disagree, they are both wrong until reconciled.

| Gate | Command | Proves |
|---|---|---|
| Fuzz | `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored` | 1,500 self-play games, per-move invariants |
| Choice census | `cargo test --release -p yokohama-core --test choice_encoding -- --ignored` | every choice variant fires organically |
| Observation honesty | `cargo test --release -p yokohama-core --test observation_honesty -- --ignored` | the view masks the hidden set, the sampler conserves, no leakage (Phase 3.0) |
| Action-source equivalence | `cargo test --release -p yokohama-core --test action_source_equivalence -- --ignored` | the factored action families reconstruct `get_legal_moves` exactly — as a multiset AND in ORDER, elementwise — at seeds 0..20 × 2/3/4P, plus the dense adversarial route cells (Phase 3.1a + 3.1b) |
| Action-source worst case | `cargo test --release -p yokohama-core --test action_source_worstcase -- --ignored` | pays both exhaustive ceilings once to record the gaps the factored source avoids: the 442,369-move retreat ceiling and the frozen route reference's 1,687,537 allocations at the dense cell (Phase 3.1a + 3.1b; both hard bounds — retreat serve-one `< 64`, routes `≤ 2·labels + 32` — are per-edit) |
| Differential | `cargo test --release -p yokohama-difftest --test differential -- --ignored` | engine ≡ oracle on every legal-move set |
| Routes3 | `cargo test --release -p yokohama-difftest --test routes3_differential -- --ignored` | route pruning ≡ prune-free reference |
| Scoring | `cargo test --release -p yokohama-difftest --test scoring_differential -- --ignored` | final scores + tie-break ≡ oracle |
| Worst-case routes | `cargo test --release -p yokohama-difftest --test routes3_worstcase -- --ignored` | maximal 17-area interior-Canal K6 battery (bound 519 — that harness's RECORDED observation, never a ceiling; ledger L9) + route-label key purity |
| Arena | `cargo test --release -p yokohama-core --test arena_gate -- --ignored --test-threads=1` | the arena ranks seat policies with HONEST intervals: a same-policy null cell containing its null with nonzero SE, a non-degenerate edge cell plus a guaranteed edge from two shipped knob settings, the zero-width refusal exercised on the real blowout (never a printed ±0.000), byte-identical re-runs at both unit shapes, interval-machinery validation against known truth (R=2,000 synthetic) AND against real clustered games (R=100 replicates), honest-play counters, and freshly emitted records that all `replay_strict`-clean (Phase 3.2) |
| Policy diversity | `cargo test --release -p yokohama-core --test policy_diversity -- --ignored` | the fuzzer runs the SHIPPED family-weighted policy at 2/3/4P (N=500/count, per-move invariants, raw-state driver): 4P trigger diversity recovers against a frozen held-out uniform ceiling (ChurchTrackFull Wilson95 lower bound 1.83% > 0.72%), applied-move coverage clears floors measured before they were set, and per-phase option-count tails are RECORDED — never bounded, since no bound is known (Phase 3.2) |
| Evaluator | `cargo test --release -p yokohama-core --test evaluator_gate -- --ignored --test-threads=1` | the evaluator bots beat family-weighted random with honest intervals (Phase 3.3; every cell's contestants run `WeightTable::V2` since the 2026-08-10 freeze, all figures re-recorded then), five 2P cells on one committed diagnostic master: the PROMOTION cell (Lookahead-v1 vs family-weighted, N=200 pair units) is the repo's ONE direction-asserting cell — passing on either a bootstrap interval entirely above 0.5 or the all-won refusal's rule-of-three bound 0.985 > 0.5, both pre-committed before the run (measured: θ̂ 1.000000, 400/400 games, the refusal arm) — with §0.3(b)'s explanation identity riding it (74,331 moves played, 74,331 explained); plus two RECORDED lattice cells (Greedy-v1 vs family-weighted θ̂ 0.6300 [0.5775, 0.6800]; Lookahead-v1 vs Greedy-v1 θ̂ 1.0000 at 100 units), a Keep-homogeneous NULL self-test asserting containment (θ̂ 0.5475 [0.5000, 0.5975] — the lower bound sits AT 0.5 exactly on the current capture, a standing flake risk with an in-source disposition, never an excuse to widen), and byte-identical determinism at N=12 (2,129 report bytes). Arm 2 — the close-time OS-entropy confirmation — is deliberately not in the roster |
| Architecture | `cargo test --release -p yokohama-core --test architecture_gate -- --ignored --test-threads=1` | the Phase-3.4 MCTS-v1 contestant and the shared scored-successor budget are registered honestly, three 2P cells on one committed diagnostic master and **no cell asserts a direction** (the architecture question is the pre-registered experiment's, not a roster test's): DETERMINISM on the node-matched C2 shape (`mcts-v1@750` vs `lookahead-v1:b=4,cap=750`, N=12 run twice — 2,336 byte-identical bytes, with a flipped-master anti-vacuity control backed by a play-derived figure); HONEST COUNTERS on the same shape at N=8 (`x_explanations == x_applied_moves`, 3,108 of 3,108, and the x/y budget meters on both sides at `0 < root < scored`, `binds > 0`, `starved <= binds`, with MCTS-v1's `(Some, None)` cap-counter shape); and the MCTS SELF-NULL at N=24 asserting nonzero SE and non-degenerate unit shares — the only detector a new contestant has for a CRN/keying collapse, which reproduces byte-identically — with containment of 0.5 printed, never asserted at that N (Phase 3.4) |
| Piped dogfood | `cargo test --release -p yokohama-cli --test piped_dogfood -- --ignored` | the shipped binary plays a full 2P game over pipes to a winner, zero ESC bytes |
| Recovery falsifier | `cargo test --release -p yokohama-core --lib -- --ignored` | under the SHIPPED weight table (`WeightTable::V2` since the freeze; both the table and the floor moved with it, deliberately — a floor left at the V1 value would have been half a share the shipped table no longer produces), a retreat that RECOVERS something is still sometimes the greedy argmax — the executable falsifier for L6's eval-v0 pathology, measured over the freeze corpus's first 312 seeded 2P games against a measured-then-halved floor: **7 picks in 175,782 applied moves, share 0.000040 vs floor 0.000020** at `SEMANTICS_REV` 7. Per-edit until 2026-08-09, demoted by owner ruling (OD-6 arm 1) once the rev-7 sample re-size took it to ~113s of debug wall; **runs EXACTLY 1 test** — it is the only `#[ignore]`d test in the core lib, which is why the command is `--lib` and why that count is stated here. Deliberately no small-N per-edit smoke: at ~4 events per ~176,000 applied moves the power IS the sample size (`policy_diversity`'s frozen-floor precedent); the per-edit ordering pin and one-ply cells keep guarding the assistant triple (Phase 3.3, demoted in Phase 3.5c) |

## Data provenance (cards, boards, tokens)

All static game data is **verified, three ways** (program of 2026-07-16/17, artifacts in
`docs/plans/2026-07-16-card-verification/`): blind per-cell re-transcription of the TTS sprite
sheets, diffed against the YAML sources, with counts anchored to the rulebook's printed
component list ("Contents of the Game": 36 order cards — corner indices 1–36 verified present
exactly once; 30 technology cards — per-title copy counts summing to the rulebook's 30; 24
building sites re-modeled to their positional space rewards). Jeremy's physical box check
(2026-07-18) closed every flagged exception — including the full 5-Power token pool, board
layouts, and canal geometry — and every owner ruling since is recorded in
[`guidingpath.md`](guidingpath.md)'s decision log. The engine's registries are generated from
the YAML by `gen-data`; a `rules_hash` stamps every saved game with the data that produced it.

## Where truth lives

- [`guidingpath.md`](guidingpath.md) — **authoritative** status, roadmap, and every rules
  ruling (decision log)
- [`CLAUDE.md`](CLAUDE.md) — working conventions (TDD, gates, the oracle independence rule,
  the `SEMANTICS_REV` bump contract)
- [`docs/cli-guide.md`](docs/cli-guide.md) / [`.html`](docs/cli-guide.html) — tester-facing
  CLI guide (kept word-identical, enforced by the `guide_sync` test)
- [`docs/suggest-walkthrough-2026-07-27.html`](docs/suggest-walkthrough-2026-07-27.html) — a
  hands-on review of `suggest` at v0.3.3 in real terminal output: what held up, and the
  display-layer defects it surfaced (a dated snapshot, not a maintained doc). Also
  [hosted, for a link that renders](https://claude.ai/code/artifact/9dbb3a84-7994-4066-a400-7e83c049a19e)
  — a copy, so edit the file above and re-publish rather than treating the page as the original
- [`docs/plans/`](docs/plans/) — dated designs, plans, and reports;
  [`2026-07-20-phase-2.9-ruling-list.md`](docs/plans/2026-07-20-phase-2.9-ruling-list.md) is
  the rules-ambiguity register (fully ruled)
