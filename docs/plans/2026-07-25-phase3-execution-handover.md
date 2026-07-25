# Phase 3 Execution — Handover (pause point, 2026-07-25)

**Purpose.** This is the state-of-the-work handover at a deliberate pause in the Phase-3 autonomous execution window. It answers, for a fresh orchestrator with no memory of this session: what was asked, what is done, what is in flight, what comes next, and where every piece of durable state lives.

**Read order for a fresh session:** `guidingpath.md` (authoritative status) → `docs/plans/2026-07-25-phase3-map.md` (the governing Phase-3 map) → this handover → the sub-phase spec/plan you are picking up.

---

## 1. The original assignment (unchanged)

Run the Phase-3 ladder **3.0 → 3.4**, one sub-phase at a time, each as: brainstorm/spec → plan → TDD execution → PR with detailed documentation → dynamic-workflow review → merge on proper review → next sub-phase. **Do not begin 3.5** — it is owner-in-the-loop; stop and hand back when 3.4's architecture decision is recorded.

Standing directives that governed every decision here:
- **Model dispatch:** Fable orchestrates and runs plan-development agents; Opus 5 default for development + review agents; Sonnet 5 permitted for PR-review fan-out under an Opus 5 adjudicator.
- **Review lifecycle per PR:** pr-review-toolkit suite fanned out via workflow; findings AND resolutions posted as PR comments (the audit trail is the point); Opus 5 adjudication; fixes applied under the receiving-code-review discipline (verify, don't blindly apply); all release-tier gates green before merge.
- **Autonomy envelope (3.0–3.4):** rules rulings, `SEMANTICS_REV` bumps, and `guidingpath.md` updates may pass through without stopping when highly confident, research-backed, and independently verified by a clean-eyes subagent — each recorded as it lands.
- **Hard stops (owner-only):** radical direction changes; genuinely table-dependent rules ambiguity; anything outside a sub-phase's mapped purpose/gate (the re-map signal); branch deletion (joint task — merged branches are preserved).
- **Standing rules:** oracle co-evolution under the independence rule; every published branching/perf figure states its measurement policy; the corpus is never precious; compute ladder M5 Air → 96 GB workstation → cloud, and hardware never selects the architecture.
- **Live gate game:** floats; dissolves 3.5's dependency whenever the owner plays it. Findings disposition per map §7.

---

## 2. Where the work stands

### 3.0 — Observation & belief: **COMPLETE, merged to master `b120af6`** (PRs #6 + #7)

The honesty boundary the rest of the ladder stands on. Pure read-side: **no `SEMANTICS_REV` bump** (still 3), engine-behavior files empty-diff vs master, oracle untouched.

Shipped: `PlayerView` + `observe()` (seat-relative mask, exhaustive-destructure drift guard so a new `GameState` field is a compile error, not a silent leak) · `BeliefSampler` (uniform conservation-consistent determinization) · `HonestAgent`/`TrivialHonestAgent`/`run_honest_game` (honesty enforced by types — no bot-facing method can name `GameState`/`PendingSetup`; agents built by a factory *inside* the driver) · the I11 starting-order invariant · two standing honesty suites (`observation_honesty.rs`, `observation_leakage.rs` — the leakage detector is canary-armed with a mutation-proof).

**Two map amendments carried** (§11 protocol, clean-eyes verified, recorded in the map + guidingpath):
1. The hidden set is **five** members, not four — `config.setup_seed` is a verified full-reconstruction oracle.
2. Masking that field does **not** close the channel: `new_game` builds the board from the seed *before* shuffling, so the public board fingerprints the seed (recovered in milliseconds at conventional small seeds). Closure is **operational** — full-`u64` recorded seeds for bot-facing evaluation, enforced at 3.2 — and kept executable by the `t8f` red-team test. Recorded as a permanent residual in map §8.

Gate note: the release-gate list grew 7→8 (the observation-honesty sweep), documented at four sites.

### 3.1 — Bot-facing action interface: **spec complete; split into 3.1a (done, PR open) + 3.1b (not started)**

Research (3 Opus agents, all figures measured with stated policy) and a two-lens clean-eyes review drove a **two-PR split**:

**3.1a — the factored core: COMPLETE on branch `phase3.1a-action-core`, PR opened, NOT merged.**
Discharges the mandatory handover item `guidingpath.md:172(b)`. Seven TDD tasks, each subagent-implemented and independently Opus-reviewed clean:
`MoveFamily` + `families()` dispatch · retreat **direct constructor + mixed-radix lazy iter/sample** (the six-figure family, never materialized) · placement lazy combination construction with **per-combination** affordability · unified `ActionSource` (small families delegate to the existing generators = equivalence by construction) · per-family factored↔exhaustive equivalence (two tiers, mutation-checked) · worst-case allocation-count bound + `action_cost` benchmark · docs/gates/PR.

**The measured discharge:** serving ONE retreat candidate off the 442,368-move family = **5 allocations / ~104 B peak-outstanding / ~274 ns**, versus exhaustive `get_legal_moves` = **1,017,594 allocations / 57.8 MB / ~38 ms** → **203,518× fewer allocations**. Hard gate is the `<64` allocation-COUNT bound (deterministic); peak-outstanding bytes are recorded, not gated. Discrimination proven live (an O(product) path = ~1.02M allocs) and by a reverted mutation (539,096 allocs → bound fails). The release test asserts the **measured** 442,369 total (442,368 retreat + exactly 1 route on the minimal-adjacency fixture) — not a reasoned estimate.

**3.1b — routes + candidate validation: NOT STARTED.** Two deliverables, both specced with honest risk flags:
- **The lazy route label enumerator** — *genuinely unsolved in spec and a possible re-map signal.* The obvious primitive (`find_reachable_areas_with_tech`) returns one min-cost path per destination and therefore **drops the L2 income-denial fee-vector labels** that `enumerate_president_routes` exists to capture. 3.1b must present a correct cheap enumerator of the distinct affordable `(destination, RouteVector)` labels and prove it reproduces the exhaustive label set; **if that proves infeasible, STOP and re-map — do not silently fall back to the gate-forbidden DFS or a quiet cap.** This matters: the route DFS is ~half the OnBoard worst case (~44 ms of the ~65 ms), so retreat alone only halves it.
- **`is_move_legal` / `is_president_path_legal`** — settles open-decision #3 by microbenchmark. Note the research finding that reframes it: `apply_move_unchecked` is **not** a legality oracle (measured false-accepts in four Tier-C dimensions — Building power, movement reachability, PlacePresident possible-area, and phase), so "design B" only works as *construction-correct* proposals. The single new predicate must check adjacency + simple-path + no-return-to-start + `can_step_onto` per hop + **destination is a Possible Action Area** + affordability, and compare by canonical label, not raw path.

### 3.2, 3.3, 3.4 — not started

Per map §5. 3.2 (arena, rungs, family-weighted policy — the corpus starts, the fuzzer promise is paid, and the seed-hygiene rule from 3.0's residual gets *enforced* here) · 3.3 (evaluator v1, shallow search, scenario suite) · 3.4 (the discriminating experiment ending in a recorded architecture decision — **then stop and hand back**).

---

## 3. Immediate next steps, in order

1. **Run the 3.1a PR review workflow and merge it.** The PR is open with all gates green; the pr-review-toolkit fan-out + Opus adjudication has not run yet. Post findings *and* resolutions as PR comments. **Push before merging** (see lesson L1). Preserve the branch.
2. **Update `guidingpath.md`/memory at the merge** (the close-out row is written; confirm it reflects the merged state).
3. **3.1b:** spec → clean-eyes review → plan → TDD → PR → review → merge, with the route-enumerator re-map signal explicitly live.
4. **3.2 → 3.3 → 3.4**, same pipeline. Stop after 3.4's architecture decision is recorded.

---

## 4. Where durable state lives

| What | Where |
|---|---|
| Authoritative project status | `guidingpath.md` (headline + decision log) |
| Governing Phase-3 map (+ amendments, §8 ledger) | `docs/plans/2026-07-25-phase3-map.md` |
| 3.0 spec / plan | `docs/plans/2026-07-25-phase3.0-observation-belief-{spec,plan}.md` |
| 3.1 spec (covers 3.1a + 3.1b) | `docs/plans/2026-07-25-phase3.1-action-interface-spec.md` |
| 3.1a plan | `docs/plans/2026-07-25-phase3.1a-action-core-plan.md` |
| Per-task execution ledgers, briefs, reports, review packages | `.superpowers/sdd/<plan-basename>/progress.md` (+ siblings) — **gitignored**, so they do NOT survive a fresh clone; the git history and this handover do |
| Cross-session memory | `~/.claude/projects/.../memory/` (`yokohama-true-engine-status.md` is the status memory) |

**Note:** the SDD ledgers are the richest record of *why* each task landed as it did (findings, adjudications, parked minors). They are local-only. Everything load-bearing has been mirrored into commit messages, the specs, `guidingpath.md`, and this document.

---

## 5. Lessons and process rules earned this window

- **L1 — Push before merging.** PR #6 was merged while the review's must-fix commit was still local-unpushed, so master briefly lacked it; recovered via PR #7. Always `git merge-base --is-ancestor <fix> master` after merging.
- **L2 — Don't let HEAD move under an in-flight review.** Happened twice (a Task-5 follow-up commit, a Task-6 amend that orphaned the package). Implementers must not amend or extend after reporting DONE; otherwise the controller must re-point the reviewer at a HEAD-matching package.
- **L3 — Harness double-spawns are real (4 occurrences).** Each time, one agent detected the collision and stood down without writing, and git verification (HEAD unmoved, only expected untracked files, empty tracked diff) settled ownership. Never kill processes; verify with git, not with the agent roster (stale names from earlier sub-phases appear there). Worth adding to task briefs: *check for untracked deliverables before your first write* — in one near-miss only the read-before-write guard prevented a clobber.
- **L4 — Reviews earn their keep.** Every task-level review passed, yet the whole-branch review of 3.0 still found a real must-fix (a gate that claimed release-tier evidence it did not have). A duplicate agent's read-only flag caught a reasoned-not-measured constant in 3.1a. Adversarial verification is where the defects surfaced.
- **L5 — Fix the gate, don't redefine it.** When 3.0's gate claim outran its evidence, the response was to make the evidence real (an agent-driven release sweep with anti-vacuity instrumentation), not to soften the claim.

---

## 6. Open carry-forwards (non-blocking, recorded)

- **3.0:** one comment line-number drifted during the fix wave (`invariants.rs:59` should cite `player.rs:251`; same for spec §2's F4 citation) — next truth pass. 17 other whole-branch minors parked with rulings in the 3.0 review record.
- **3.1a:** placement paging is O(k) generate-and-filter, not O(1) unrank (a 3.3 widening concern); `action_cost_meter` is a deliberate near-duplicate of the lib `alloc_probe` (build-unit separation); assorted per-task minors in the ledger.
- **Map:** §5 3.1's citation of `guidingpath.md:171(b)` should read `172(b)` (fixed in 3.1a's doc-sync task).
- **Live gate game** remains unplayed; it blocks only 3.5's bar declaration, not 3.0–3.4.
