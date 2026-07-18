# Phase-2 entry backlog (2026-07-18)

Carried out of the engine-completion final review (verdict + triage in `.superpowers/sdd/final-review-report.md`) so Phase 2 starts from a durable list instead of ledger archaeology. Nothing here blocks the engine-completion merge; every item was explicitly triaged ride-to-Phase-2 or deferred with a decision-log row.

## Deferred rules functionality (decision-log rows exist for each)

1. **Choice-encoding effects** (need new Move variants / decision points — the standing Phase-2 choice-encoding work): University, Spinning Mill (+Textile Mill), Telephone's leave-one-assistant recovery choice; movement-icon rewards (orders/site spaces/church riders); warehouse-icon token choice (currently auto-assistant); any-good choice (currently auto-copper); church/customs placement choice (currently auto-highest/auto-leftmost-of-pair); Skip's forfeited area bonuses (5-power Port +3 VP on a declined take).
2. **Station token placement on acquisition** (deferred v1; the teleport consumption side is wired).
2b. **From-hand Tram** (re-review open point): Tram is wired for OnBoard movement only; whether it also grants a from-hand placement mode is an open interpretive question — needs a rules read + decision-log row before wiring.
3. **I4 — Canal**: crossings are topologically impossible in `board_gen` (adjacency never spans the canal), so the toll code is dead. Needs its own mini-plan: model the canal edge(s) in board generation, activate the 1-Yen toll (already coded in `calculate_movement_cost_full`), tests, and a fuzz sweep after. Deferral row in the decision log.

## Riding minor findings (verified present at final review; none merge-blocking)

- integration.rs `setup_2p_game()` returns an unused registry tuple.
- Some legal_moves.rs internal generators still take `Option<&TechRegistry>` — retire the Option.
- `apply_fulfill_order`'s `Option<&OrderRegistry>` param (callers always pass Some) + `check_mercenary_eligibility`'s unused `_country` param — collapse together.
- Lab surcharge table duplicated (`LabBoard::position_modifier` vs `lab_position_surcharge`).
- `execute_lab_action` returns `Result<(), MoveError>` vs the `&'static str` executor convention.
- No `LabBoard::find_card` helper (inline `.position()` vs `PortBoard::find_card`).
- Church rejection error string doesn't distinguish faith<4 from all-eligible-occupied — the Phase-2 CLI will want distinct reasons.
- Executor-level coverage gaps: church insufficient-resource paths / double-placement arrival order / all-8-full; customs direct 2-copy `customs_vp_bonus` unit test.
- `CustomsSlot.arrival_order` doc-comment claims tie-break use; scoring uses index position (propagated from `ChurchSlot`) — fix the comment or the field.
- Trading-house cost progression lives in three places (employment apply loop, legal_moves mirror, `TradingHouseState::next_cost`) — consolidate.
- Chinatown BFS expands seen-duplicate subtrees (perf nit only).
- Older mining tests inline `TechCard` literals instead of `mining_tech_fixture()`.
- Port pair-eligibility `hand_len + 2 <= 3` beside a clippy-simplified sibling (style).
- `replay()` eprintln side effect on version mismatch (dead branch today) — decide logging story with the CLI.
- `MoveError::WrongPhase` never constructed — decide: wire as defense-in-depth for untrusted `GameRecord`s (recommended once save files exist) or remove.

## Open external check

- Jeremy's physical box check (exceptions-only): `docs/plans/2026-07-16-card-verification/box-check-brief.md` + `analysis-1/yokohama-kb/01-rules-reference-engine-mapped.md`. Any deviation patches YAML → `cargo run -p gen-data` → fuzz gate rerun.
- I5 ("place 0 assistants" ruled legal by the fix wave) flagged for Jeremy's confirmation; independent reviewer adjudication recorded in the final-review report's re-review section.

## Standing regression net for all Phase-2 work

`cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored` (1,500 games, ~8s) before every merge; 5000×3 selfplay sweeps at phase boundaries; replay determinism as the engine-change tripwire.
