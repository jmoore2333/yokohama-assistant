# `replay_strict` engine_version decoupling — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `SEMANTICS_REV` (+ `rules_hash` + terminal digest) the sole strict-replay compatibility key so a workspace-version release cut cannot split a rev-keyed corpus; `engine_version` becomes provenance-only.

**Architecture:** One deletion in `replay_strict` (record.rs), one pinned test flipped to the release-cut scenario, a doc-comment/comment sweep at the four sites that describe the old policy, one guidingpath decision-log row. No CLI code change, no `SEMANTICS_REV` bump (no legality/application/scoring/setup behavior moves; no record byte changes).

**Tech Stack:** Rust workspace (`cargo`), existing test harnesses only.

**Spec:** `docs/plans/2026-07-27-replay-strict-engine-version-decouple.md`

## Global Constraints

- TDD: the flipped test is edited FIRST and must FAIL against the current rejection before the implementation lands.
- Suite must stay green + zero clippy warnings after the task (per-edit tier), and the full thirteen-gate release roster runs as ONE sequence before the PR is declared merge-ready.
- Expected suite counts (unchanged — the test flip is 1:1): core **883 + 25 ignored**, cli **319 + 2**, oracle **181**, difftest **52 + 5**, eval_probe **11**.
- `SEMANTICS_REV` stays **5**. `crates/yokohama-oracle` stays byte-untouched.
- Branch: `replay-strict-version-decouple` off `master`. Two commits: (A) docs — spec + this plan; (B) the code change + comment sweep + guidingpath row.
- Commit trailer: `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.

---

### Task 1: Flip the pin, delete the gate, sweep the comments (TDD)

**Files:**
- Modify: `crates/yokohama-core/tests/game_record.rs:89-99` (test flip)
- Modify: `crates/yokohama-core/src/game/record.rs:150-156, 404-409, 446-459, 461-470`
- Modify: `crates/yokohama-core/examples/record_stats.rs:58-64` (comment only)

**Interfaces:**
- Consumes: `common::run_recorded_game(seed, players, &rules) -> (GameState, GameRecord)`; `GameRecord::stamp_terminal_digest(&GameState)`; `replay_strict(&GameRecord, &Rules) -> Result<GameState, ReplayError>`.
- Produces: no signature changes anywhere — `replay_strict`'s acceptance set widens to records that are same-`semantics_rev`, same-`rules_hash`, digest-verified regardless of `engine_version`.

- [ ] **Step 1: Flip the pinned test (failing first).** Replace `game_record.rs:89-99` (`strict_replay_rejects_engine_version_mismatch_instead_of_warning`, whole test incl. attribute) with:

```rust
/// THE RELEASE-CUT SCENARIO (2026-07-27 decoupling, spec
/// `docs/plans/2026-07-27-replay-strict-engine-version-decouple.md`):
/// `engine_version` is PROVENANCE, not a compatibility key. The workspace
/// version moves at a release cut while `SEMANTICS_REV` stays put, and the
/// corpus layout (`arena/corpus.rs`) files records by rev alone -- so a
/// record that is same-rev, same-rules and digest-stamped must strict-replay
/// clean across an engine_version drift, or every release cut splits a rev-N
/// corpus in half. (Replaces
/// `strict_replay_rejects_engine_version_mismatch_instead_of_warning`, the
/// 2026-07-19 engine-authority pin written one day before `SEMANTICS_REV`
/// existed; the semantic guards -- rules_hash, semantics_rev, checked
/// per-event apply + invariants, terminal digest -- all still run here.)
#[test]
fn strict_replay_accepts_engine_version_drift_at_same_semantics_rev() {
    let rules = Rules::standard();
    let (final_state, mut record) = common::run_recorded_game(7, 2, &rules);
    record.stamp_terminal_digest(&final_state);
    record.engine_version = "0.0.0-other-release".into();
    let replayed = replay_strict(&record, &rules)
        .expect("same semantics_rev + same rules_hash must strict-replay across a version drift");
    assert_eq!(replayed, final_state, "and reproduce the exact recorded terminal state");
    // Lenient replay keeps its warn-and-continue diagnostics (unchanged).
    assert!(replay(&record, &rules).is_ok());
}
```

- [ ] **Step 2: Run it — verify it FAILS on the current rejection.**

Run: `cargo test -p yokohama-core --test game_record strict_replay_accepts_engine_version_drift_at_same_semantics_rev`
Expected: FAIL — the `.expect` panics with `engine_version mismatch: record "0.0.0-other-release" vs running "0.1.0" (strict mode rejects)` at event_index 0.

- [ ] **Step 3: Delete the gate.** In `record.rs`, `replay_strict`'s opening becomes:

```rust
pub fn replay_strict(record: &GameRecord, rules: &Rules) -> Result<GameState, ReplayError> {
    let expected = rules.content_hash();
```

(removing `let running = env!("CARGO_PKG_VERSION");` and the whole `if record.engine_version != running { return Err(...) }` block).

- [ ] **Step 4: Update the four narrations of the old policy.**

(a) `GameRecord.engine_version` field doc (record.rs:150-155) →

```rust
    /// `env!("CARGO_PKG_VERSION")` of the engine build that produced this
    /// record. PROVENANCE ONLY, deliberately not a compatibility key
    /// (2026-07-27 decoupling): the workspace version moves at a release cut
    /// while executable semantics do not -- `semantics_rev` (with
    /// `rules_hash` and the terminal digest) is what gates `replay_strict`,
    /// and the corpus layout files records by rev alone. Lenient `replay`
    /// and the CLI's `Session::load` surface a drift as a warning so a
    /// human sees which build wrote the file; nothing refuses on it.
```

(b) lenient `replay`'s second doc paragraph (record.rs:404-409) →

```rust
/// Warns (to stderr) on any provenance drift -- engine_version, rules_hash,
/// semantics_rev -- and replays anyway. `replay_strict` (Task 3) is the
/// hard-rejecting alternative training pipelines should use instead; it
/// rejects on the SEMANTIC keys (rules_hash, semantics_rev, terminal
/// digest) but treats `engine_version` as provenance like this function
/// does (2026-07-27 decoupling -- a release cut must not split a
/// rev-keyed corpus).
```

(c) `replay_strict`'s doc list first bullet (record.rs:447-448) →

```rust
/// - `rules_hash` mismatch, or `semantics_rev` mismatch OR missing (`None`
///   -- unknown provenance, fail-closed) => Err at event_index 0.
///   `engine_version` is deliberately NOT a gate (2026-07-27 decoupling,
///   spec `docs/plans/2026-07-27-replay-strict-engine-version-decouple.md`):
///   the workspace version moves at a release cut while executable semantics
///   do not, and the corpus layout (`arena/corpus.rs`) promises rev-keyed
///   records strict-replay against their own `SEMANTICS_REV`; the guards
///   below protect a forgotten rev bump at per-record granularity, which a
///   release-cut version string cannot.
```

and its closing paragraph's last words "the version check and around each folded step" → "the provenance checks and around each folded step".

(d) `examples/record_stats.rs:58-64` comment, middle lines →

```rust
    // replay rejects a rules_hash / semantics_rev mismatch (engine_version
    // is provenance, not a gate -- 2026-07-27 decoupling), runs
    // check_invariants after every event, and (if the record was stamped)
    // verifies the terminal-state digest -- so a corpus produced under
```

- [ ] **Step 5: Run the flipped test + both record test surfaces.**

Run: `cargo test -p yokohama-core --test game_record` and `cargo test -p yokohama-core record::`
Expected: PASS (all game_record tests incl. the flip; record.rs unit tests untouched).

- [ ] **Step 6: Per-edit tier.**

Run: `cargo test -p yokohama-core && cargo test -p yokohama-cli && cargo test -p yokohama-oracle && cargo test -p yokohama-difftest && cargo test -p yokohama-core --example eval_probe && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 883+25 / 319+2 / 181 / 52+5 / 11, clippy clean.

### Task 2: guidingpath decision-log row + commits

**Files:**
- Modify: `guidingpath.md` (append one row after the 2026-07-27 "PHASE 3.3 LANDED" row, matching the `| date | **HEADLINE.** body | rationale |` format)

- [ ] **Step 1: Append the row** (content: strict gate keys on SEMANTICS_REV; engine_version rejection removed as a 07-19 stopgap predating SEMANTICS_REV; over/under-inclusive argument; what stays; NO rev bump and why; trigger = imminent release cut syncing tags to phases + 3.4 corpus generator).
- [ ] **Step 2: Commit A (docs):** `git checkout -b replay-strict-version-decouple && git add docs/plans/2026-07-27-replay-strict-engine-version-decouple*.md && git commit` — message `docs: spec + plan — record compatibility keys on SEMANTICS_REV, engine_version becomes provenance-only`.
- [ ] **Step 3: Commit B (code):** `git add crates/yokohama-core/src/game/record.rs crates/yokohama-core/tests/game_record.rs crates/yokohama-core/examples/record_stats.rs guidingpath.md && git commit` — message `fix(record): replay_strict keys on semantics_rev — engine_version rejection removed (provenance-only)` + body naming the flip and the no-rev-bump rationale.

### Task 3: Release roster + version-inheritance check

- [ ] **Step 1: Confirm the workspace version is a single edit point** (feeds the PR body's answer to "it affects each component"):

Run: `rg -n "^version|version\.workspace" Cargo.toml crates/*/Cargo.toml`
Expected: one `[workspace.package] version` in the root; every crate `version.workspace = true`.

- [ ] **Step 2: Thirteen-gate release roster as ONE sequence** (documented commands from CLAUDE.md, in its order: fuzz, census, observation, equivalence, action-source worst case, arena gate `--test-threads=1`, policy diversity, evaluator gate `--test-threads=1`, piped dogfood, differential, routes3, scoring, routes3 worst case).
Expected: all rc=0; record per-gate wall clock; ~5–6 min total on a warm release build.

### Task 4: Review + PR

- [ ] **Step 1:** Code-review pass (pr-review-toolkit:code-reviewer) over the branch diff; address anything real.
- [ ] **Step 2:** Push: `git push -u origin replay-strict-version-decouple`.
- [ ] **Step 3:** `gh pr create` — title `Record compatibility keys on SEMANTICS_REV — replay_strict engine_version decoupling`; body: the wart (strict gate vs rev-keyed corpus layout), the two-directions argument, what changed / deliberately didn't, gate results, the release-cut answer (post-merge a 0.3.3 bump touches only the workspace version; the only remaining version-drift surface is the CLI's benign load warning), footer `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
