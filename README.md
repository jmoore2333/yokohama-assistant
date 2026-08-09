# Yokohama Assistant

A Rust engine, live-game CLI tracker, and verification stack for the board game **Yokohama**
(Hisashi Hayashi, 2016), built toward a computer player that provably plays to win.

> [!IMPORTANT]
> **Testing the CLI? Start with the [HTML CLI guide](docs/cli-guide.html).** Ready-to-run
> builds for Apple Silicon macOS, Ubuntu x86_64, and (new in v0.3.3) Windows x86_64 are on the
> [Releases page](https://github.com/jmoore2333/yokohama-assistant/releases) — current release
> **v0.3.3**, the first full (non-pre) release; each download also includes an offline copy of
> the guide.

**Priority order:** 1) engine-validated move-by-move tracking of live games via CLI →
2) a computer player → 3) computer vision / AR assist (deferred; the original 2025 AR-first
vision lives in `docs/plans/2025-12-07-yokohama-assistant-design.md`).

## See it running

![The Yokohama CLI mid-game at 2 players: the staggered pyramid board, the players panel, and the
`suggest` output — recommended move, the line it searched, the term-by-term score table, and the
ranked runner-ups](docs/images/cli-suggest.png)

Turn 7 of a 2P game, real terminal output. Above the rule is the everyday screen; below it is
**`suggest`** — the Phase-3.3 evaluator reading *your own seat's view*, printing the move it
recommends, the six-move line it valued the position by, the arithmetic behind the 330 (each row
is your value minus the best opponent's, times this stage's weight), and the nearest runner-ups
out of 66 ranked candidates, each with its margin. It applies nothing and records nothing.

**[Full `suggest` walkthrough →](https://claude.ai/code/artifact/9dbb3a84-7994-4066-a400-7e83c049a19e)**
— a few minutes of live play against a bot seat (2026-07-27, v0.3.3), captured as the raw ANSI
stream from a 140×50 PTY, with an honest read on both sides: **what held up** (the search's
projected lines matched the played-out game move for move; it found genuine multi-step
combinations; determinism and the no-side-effect guards are real) and **what it flagged** (two
display-layer bugs in `describe.rs` — a Laboratory surcharge rendered as a reward, and production
previews that ignore the acting player's techs — plus the evaluator's structural positional
blindness, and the fact that the *playable* bot is the 3.2 weighted-random rung, not the 3.3
evaluator). Every finding is display-layer; the engine did the right thing in each case.

That link is a hosted copy, so it opens and renders in a browser. The source of truth is
[`docs/suggest-walkthrough-2026-07-27.html`](docs/suggest-walkthrough-2026-07-27.html) in this
repo — GitHub shows `.html` as source rather than rendering it, so read that one by opening the
file locally.

## Status (2026-08-07)

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

**The computer player ladder (Phase 3) is five sub-phases in**, governed by the accepted map
[`docs/plans/2026-07-25-phase3-map.md`](docs/plans/2026-07-25-phase3-map.md), every one of them
with `SEMANTICS_REV` unmoved at 5 (all merged to master):

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

**Next (renumbered by owner ruling, 2026-08-07):** **3.5b is the BGA→CLI bridge** — a dev-mode
Chrome extension + local relay + importer that turns the owner's own BoardGameArena games into
engine-validated live-tracked records (cards identified by their unique requirement/reward
vectors), with the re-scoped Phase-2 live-tracking gate claimed by finishing a real BGA table
tracked end-to-end (design: [`docs/plans/2026-08-07-bga-bridge-design.md`](docs/plans/2026-08-07-bga-bridge-design.md)).
**3.5c is the strength/automation lane** formerly called 3.5b (evaluator v2, tuning, the
five-rung experiment, productionized bot seats, acceptance play, the bar declaration).
The CLI is tester-ready; start with the linked HTML guide and report any table/CLI mismatch.

Releases now track completed phases (**v0.3.3** ↔ Phase 3.3; the first version bump in the
repo's history, and the first with a Windows build). Saves and self-play records are
compatibility-keyed to the game-semantics revision (`SEMANTICS_REV`), not the release version
(PR #14), so they survive release cuts.

## Workspace crates

| Crate | What it is |
|---|---|
| `yokohama-core` | The engine: serde-serializable `GameState` (history-free, cheap to clone for search), pure `get_legal_moves` / checked `apply_move`, seeded setup, `GameRecord` save/replay stamped with `rules_hash`, `semantics_rev`, and a terminal-state digest verified by `replay_strict` — plus the Phase-3 player stack: the honest observation boundary (`PlayerView`/`observe`/`BeliefSampler`), the factored action families (`DecisionHandle`), the arena (contestants, paired cells, corpus emission), evaluator v1 (`game/eval.rs`), and the Greedy/Lookahead search rungs |
| `yokohama-cli` | The game driver: setup wizard → REPL; simulated games and live-table tracking (reconciliation, desync tripwire), save/load/undo, TTY-colored board, and **human-vs-bot play** in sim mode (any seat mix with ≥1 human; the bot runs the same honest view → belief-sampler → policy path the arena does) plus **`suggest`** — the evaluator bot analyzes YOUR seat's view and explains its recommended move term by term, deterministic on repeat presses even across save/reload |
| `yokohama-oracle` | The independent referee: legal-move enumeration, final scoring + tie-break, and a prune-free route reference, authored from the rules sources under an audited independence rule (types-only dependency on the engine) |
| `yokohama-difftest` | The harness comparing engine vs oracle: per-decision-point sweeps under two policies, terminal scoring comparison, three-way route-pruning check, self-contained divergence artifacts + `repro` tool |
| `yokohama-bridge` | The BGA capture relay (Phase 3.5b): a localhost-only HTTP sink that appends what the read-only browser extension in `bridge/` observes to `bridge-captures/<table>.ndjson`, plus the feed pre-processor (typed NDJSON reader, payload-identity dedupe, structural supersession collapse for takebacks). Depends on NO engine crate and knows no rules — translating captures into engine moves is `yokohama-cli`'s `import`, deliberately on the other side of that boundary |
| `gen-data` | YAML → Rust generator for the verified card/site data (never hand-edit `game/data.rs`) |

## Download and run

Download the archive for your system from
[GitHub Releases](https://github.com/jmoore2333/yokohama-assistant/releases) — current release
**v0.3.3**:

- `macos-apple-silicon` is optimized for Apple Silicon Macs (M1 or newer).
- `ubuntu-x86_64` is a static binary for 64-bit Ubuntu Linux.
- `windows-x86_64` is a 64-bit Windows build (new in v0.3.3).

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
`a 3 7 9` / `b 5`, assistant moves `m 4 7`, retreats `r 3:2 5:1`. Views: `board`, `sites`,
`goals`, `tracks`, `labs`, `log`, `help`; plus `suggest` (the bot's explained recommendation for
your seat), `undo`, `save`, `q`. Full tester guide:
**[HTML](docs/cli-guide.html)** or [Markdown](docs/cli-guide.md).

**Gates.** Per-edit (debug): the crate suites + `cargo clippy --workspace --all-targets
-- -D warnings`, all zero-warning (five crates on master; on the 3.5b bridge branch also `cargo test -p yokohama-bridge` and the four `bridge/tests/*.js` node suites, which have no cargo coverage and would otherwise never run). Pre-merge: all **fifteen** release gates below — the fifteenth is the
Phase-3.3 recovery falsifier, which was not written 2026-08-09 but DEMOTED that day by owner ruling
(OD-6 arm 1) from the per-edit tier it had outgrown: it buys −88 to −102s on every per-edit core run
(191.11s before; two warm readings after, 89.18s and 102.68s) for ~7s on the roster, and **no target restatement rides it — the ≤420s target
stands** (OD-6 arm 2 deferred that), so the next close measures the fifteen as one sequence. The
fourteenth is Phase 3.4's
architecture gate, added 2026-08-04, which also restates the roster target
**≤360s → ≤420s** on the grounds of one added binary. The **fourteen**-gate roster (the roster as it
then stood — the figures below have not been re-run since) measured
**326.70s** together as one uninterrupted sequence 2026-08-05 at the `phase3.4-search-arch` close
(Apple M5, quiet machine, release binaries pre-built first, the arena/evaluator/architecture gates
single-threaded because their documented commands say so): fuzz 46.57 · census 0.78 · observation
2.95 · equivalence 27.69 · action-source worst case 0.45 · arena gate 21.05 · policy diversity
6.35 · **evaluator gate 144.53** · **architecture gate 32.88** · piped dogfood 12.60 ·
differential 23.03 · routes3 1.83 · scoring 2.10 · routes3 worst case 3.44 — all rc=0, and
**under** the restated ≤420s target. The thirteen pre-3.4 gates sum to 293.37s inside that run
against 313.40s at the 3.3 close, so the thirteen carried-over gates got FASTER inside this run (293.37 s vs 313.40 s at the 3.3 close) while the roster TOTAL rose to 326.70 s on the added binary (+13.30 s); the two
largest movers are both downward (differential 32.48 → 23.03, dogfood 19.70 → 12.60) and are the
same two rows that were up at the 3.3 close, while the evaluator gate reproduced to within 0.06s —
per-gate shapes are unchanged, so that is load variance. Re-measure the whole table as one run at every
close: a sum of per-command times from separate sessions is not a measured total. This table is the
binding list — if it and CLAUDE.md's two-tier policy ever disagree, they are both wrong until
reconciled.

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
| Evaluator | `cargo test --release -p yokohama-core --test evaluator_gate -- --ignored --test-threads=1` | the evaluator bots beat family-weighted random with honest intervals (Phase 3.3), five 2P cells on one committed diagnostic master: the PROMOTION cell (Lookahead-v1 vs family-weighted, N=200 pair units) is the repo's ONE direction-asserting cell — passing on either a bootstrap interval entirely above 0.5 or the all-won refusal's rule-of-three bound 0.985 > 0.5, both pre-committed before the run (measured: θ̂ 1.000000, 400/400 games, the refusal arm) — with §0.3(b)'s explanation identity riding it (71,707 moves played, 71,707 explained); plus two RECORDED lattice cells (Greedy-v1 vs family-weighted θ̂ 0.6425 [0.5950, 0.6900]; Lookahead-v1 vs Greedy-v1 θ̂ 1.0000 at 100 units), a Keep-homogeneous NULL self-test asserting containment (θ̂ 0.5150 [0.4650, 0.5625], SE 0.0250), and byte-identical determinism at N=12. Arm 2 — the close-time OS-entropy confirmation — is deliberately not in the roster |
| Architecture | `cargo test --release -p yokohama-core --test architecture_gate -- --ignored --test-threads=1` | the Phase-3.4 MCTS-v1 contestant and the shared scored-successor budget are registered honestly, three 2P cells on one committed diagnostic master and **no cell asserts a direction** (the architecture question is the pre-registered experiment's, not a roster test's): DETERMINISM on the node-matched C2 shape (`mcts-v1@750` vs `lookahead-v1:b=4,cap=750`, N=12 run twice — 2,336 byte-identical bytes, with a flipped-master anti-vacuity control backed by a play-derived figure); HONEST COUNTERS on the same shape at N=8 (`x_explanations == x_applied_moves`, 3,108 of 3,108, and the x/y budget meters on both sides at `0 < root < scored`, `binds > 0`, `starved <= binds`, with MCTS-v1's `(Some, None)` cap-counter shape); and the MCTS SELF-NULL at N=24 asserting nonzero SE and non-degenerate unit shares — the only detector a new contestant has for a CRN/keying collapse, which reproduces byte-identically — with containment of 0.5 printed, never asserted at that N (Phase 3.4) |
| Piped dogfood | `cargo test --release -p yokohama-cli --test piped_dogfood -- --ignored` | the shipped binary plays a full 2P game over pipes to a winner, zero ESC bytes |
| Recovery falsifier | `cargo test --release -p yokohama-core --lib -- --ignored` | under the frozen `WeightTable::V1`, a retreat that RECOVERS something is still sometimes the greedy argmax — the executable falsifier for L6's eval-v0 pathology (0 of 48,458 picks), measured over the freeze corpus's first 312 seeded 2P games against a measured-then-halved floor: **3 picks in 169,014 applied moves, share 0.000018 vs floor 0.000010** at `SEMANTICS_REV` 7. Per-edit until 2026-08-09, demoted by owner ruling (OD-6 arm 1) once the rev-7 sample re-size took it to ~113s of debug wall; **runs EXACTLY 1 test** — it is the only `#[ignore]`d test in the core lib, which is why the command is `--lib` and why that count is stated here (release wall 7.02 / 6.84 / 7.00s). Deliberately no small-N per-edit smoke: at ~3.6 events per 100,000 applied moves the power IS the sample size (`policy_diversity`'s frozen-floor precedent); the per-edit ordering pin and one-ply cells keep guarding the assistant triple (Phase 3.3, demoted in Phase 3.5c) |

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
