# Yokohama Assistant

A Rust engine, live-game CLI tracker, and verification stack for the board game **Yokohama**
(Hisashi Hayashi, 2016), built toward a computer player that provably plays to win.

> [!IMPORTANT]
> **Testing the CLI? Start with the [HTML CLI guide](docs/cli-guide.html).** Ready-to-run
> Ubuntu and Apple Silicon macOS builds are on the [Releases page](https://github.com/jmoore2333/yokohama-assistant/releases);
> each download also includes an offline copy of the guide.

**Priority order:** 1) engine-validated move-by-move tracking of live games via CLI →
2) a computer player → 3) computer vision / AR assist (deferred; the original 2025 AR-first
vision lives in `docs/plans/2025-12-07-yokohama-assistant-design.md`).

## Status (2026-07-25)

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

**Next:** live-track a physical game start to finish (the Phase-2 gate), then Phase 3 — the
computer player, with the differential harness as its standing regression net. Phase 3 is now
mapped: [`docs/plans/2026-07-25-phase3-map.md`](docs/plans/2026-07-25-phase3-map.md)
(accepted 2026-07-25) is its governing document — sub-phases, gates, and execution model.

PR #5 adds the last pre-gate setup corrections and hardening. Its CLI is ready for tester use;
start with the linked HTML guide and report any table/CLI mismatch.

## Workspace crates

| Crate | What it is |
|---|---|
| `yokohama-core` | The engine: serde-serializable `GameState` (history-free, cheap to clone for search), pure `get_legal_moves` / checked `apply_move`, seeded setup, `GameRecord` save/replay stamped with `rules_hash`, `semantics_rev`, and a terminal-state digest verified by `replay_strict` |
| `yokohama-cli` | The game driver: setup wizard → REPL; simulated games and live-table tracking (reconciliation, desync tripwire), save/load/undo, TTY-colored board, and **human-vs-bot play** in sim mode (any seat mix with ≥1 human; the bot runs the same honest view → belief-sampler → policy path the arena does) |
| `yokohama-oracle` | The independent referee: legal-move enumeration, final scoring + tie-break, and a prune-free route reference, authored from the rules sources under an audited independence rule (types-only dependency on the engine) |
| `yokohama-difftest` | The harness comparing engine vs oracle: per-decision-point sweeps under two policies, terminal scoring comparison, three-way route-pruning check, self-contained divergence artifacts + `repro` tool |
| `gen-data` | YAML → Rust generator for the verified card/site data (never hand-edit `game/data.rs`) |

## Download and run

Download the archive for your system from
[GitHub Releases](https://github.com/jmoore2333/yokohama-assistant/releases):

- `ubuntu-x86_64` is a static binary for 64-bit Ubuntu Linux.
- `macos-apple-silicon` is optimized for Apple Silicon Macs (M1 or newer).

Extract the archive, open a terminal in the extracted directory, and run:

```bash
./yokohama-cli
```

The macOS build is ad-hoc signed rather than notarized. If macOS quarantines it, approve it in
**System Settings → Privacy & Security**, or run
`xattr -d com.apple.quarantine ./yokohama-cli` before launching it.

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
`goals`, `tracks`, `labs`, `log`, `help`; plus `undo`, `save`, `q`. Full tester guide:
**[HTML](docs/cli-guide.html)** or [Markdown](docs/cli-guide.md).

**Gates.** Per-edit (debug): the four crate suites + `cargo clippy --workspace --all-targets
-- -D warnings`, all zero-warning. Pre-merge: all **twelve** release gates below
(**152.17s**, re-measured together as one sequence 2026-07-26 at the `phase3.2-arena` close —
Apple M5 10-core, quiet machine, release binaries pre-built, the arena gate single-threaded
because its documented command says so). Re-measure the whole table as one run at every close: a
sum of per-command times from separate sessions is not a measured total. This table is the
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
| Piped dogfood | `cargo test --release -p yokohama-cli --test piped_dogfood -- --ignored` | the shipped binary plays a full 2P game over pipes to a winner, zero ESC bytes |

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
- [`docs/plans/`](docs/plans/) — dated designs, plans, and reports;
  [`2026-07-20-phase-2.9-ruling-list.md`](docs/plans/2026-07-20-phase-2.9-ruling-list.md) is
  the rules-ambiguity register (fully ruled)
