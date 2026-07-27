# Record compatibility keys on SEMANTICS_REV, not the crate version — `replay_strict` decoupling

**Date:** 2026-07-27 · **Status:** accepted (owner-initiated; Jeremy raised the inconsistency ahead of cutting a release from master) · **Scope:** `yokohama-core` `game/record.rs` + tests + comment sweep. No CLI code change, no `SEMANTICS_REV` bump.

## Problem

`replay_strict` (record.rs) rejects a record whose `engine_version` (the crate's
`CARGO_PKG_VERSION`) differs from the running build's — independently of
`semantics_rev`. Meanwhile the corpus layout (`arena/corpus.rs::corpus_path`)
files records under `rev<SEMANTICS_REV>/…`, and its module doc states the
intended contract outright: *"a record replays strict-clean only against its own
`SEMANTICS_REV`."* Today that claim is falsified by the version check: cut a
release that bumps the workspace version in Cargo.toml and every pre-bump rev-5
record stops strict-replaying on the post-bump rev-5 build — the directory
layout says "one compatible corpus," the strict gate quietly splits it in half.

Two events make this the moment to fix it: Jeremy is about to cut a release
(the first workspace-version bump ever — the check has never fired outside
tests), and Phase 3.4 is about to introduce the corpus generator, after which a
large record set would sit on the wrong side of any future bump.

## Why the check exists (history) and why it is not protective

The rejection landed with engine-authority (2026-07-19) — **one day before
`SEMANTICS_REV` existed** (Phase 2.9, 2026-07-20). The 07-19 design was already
split against itself: its §"rules hash" said "`engine_version` stays
informational" while its strict-replay bullet said reject. The strict arm got
implemented; then 2.9 added the real semantics key (guidingpath.md's handover
item (a) records the exact rationale: *"`engine_version` is the never-bumped
workspace 0.1.0 … a semantics change to apply/legality/scoring moves neither
stamp"*) — and the stopgap was never retired.

As a semantics guard the version check is wrong in both directions:

- **Over-inclusive exactly when it fires.** The workspace version moves only at
  a release cut. If `SEMANTICS_REV` did not move in the same window, the
  contract says semantics are unchanged — so every rejection the check can ever
  issue is (by the repo's own bump contract) a false positive.
- **Under-inclusive exactly when it matters.** A behavior change whose author
  forgot the rev bump ships under the SAME crate version as its parent commit
  (dev builds don't bump per commit), so the check is silent for the one hazard
  it could be imagined to guard.

The real guards for that forgotten-bump hazard already run and are finer
grained than any version string: strict replay re-applies every event through
the **checked** `apply_move` boundary, runs `check_invariants` after every
event, and verifies the **terminal-state digest** — actual behavioral
divergence on a record's path fails that record loudly; a record whose path is
unaffected replays identically, and accepting it is correct.

## The compatibility model (three consumers already implement it)

Strict-replay compatibility is carried by, in order: `semantics_rev` (reject
mismatch AND `None` — fail-closed on unknown provenance), `rules_hash` (reject
mismatch — static data IS behavior for replay purposes; verified:
`Rules::content_hash()` does not embed the crate version), the per-event
checked fold + invariants, and the terminal digest. `engine_version` is
**provenance**: stamped on every record, readable by any consumer, gating
nothing strict.

This is already the deployed policy everywhere except `replay_strict`:
`Session::load` refuses on rev/digest and demotes engine_version + rules_hash
to `load_warnings`; difftest artifacts stamp `(rules_hash, SEMANTICS_REV)` and
no version at all; the corpus layout keys on `rev<N>` alone.

## Alternatives rejected

1. **Strict warns (eprintln) instead of rejecting.** After one release cut the
   entire pre-bump corpus would warn on every strict replay forever — warning
   on the sanctioned-normal case converts an honest channel into noise. The
   provenance question ("which build wrote this?") is answered by reading the
   field, not by a per-replay warning.
2. **Keep the rejection; regenerate the corpus at each release.** Pointless
   regeneration cost once 3.4's corpus exists, contradicts the corpus module
   doc and the rev-keyed layout, and keeps a check that is provably
   non-protective (above).

## The change

1. `game/record.rs`: delete `replay_strict`'s `engine_version` rejection block.
   Update the three doc comments that describe the old policy: the
   `GameRecord.engine_version` field doc (provenance-only; point at
   `semantics_rev` as the compatibility key), `replay_strict`'s doc list, and
   the paragraph in lenient `replay`'s doc that bills strict as the
   version-rejecting alternative.
2. `tests/game_record.rs`: **flip the pinned test**
   `strict_replay_rejects_engine_version_mismatch_instead_of_warning` into the
   release-cut scenario stated positively: same `semantics_rev`, same
   `rules_hash`, terminal digest stamped, `engine_version` tampered → strict
   replay ACCEPTS and reproduces the exact final state (digest verified en
   route). Lenient replay's warn-and-continue assertion stays.
3. Comment sweep: `examples/record_stats.rs` (its strict-gate comment
   enumerates engine_version among the rejections); nothing else found — the
   CLI's comments describe its own (unchanged) load policy.
4. `guidingpath.md`: one new decision-log row recording the ruling.

**Non-changes (deliberate):** the field stays on `GameRecord` and stays
stamped; lenient `replay` keeps its engine_version warning (interactive/debug
diagnostics; the CLI collects it into `load_warnings` and pins that in tests);
`Session::load` untouched; corpus layout untouched; **no `SEMANTICS_REV` bump**
— the bump contract covers behavior-affecting changes to legality /
application / scoring / setup, and this changes none of them: no record's
bytes change, no replayed state changes, the acceptance set only widens to
records that are same-rev, same-rules, digest-verified.

## Verification

TDD: the flipped test fails first against the current rejection, then the
implementation lands, then the full per-edit tier (core + cli + oracle +
difftest + eval_probe suites, clippy `--workspace --all-targets -D warnings`).
Before merge: the thirteen-gate release roster as one sequence, per the
two-tier policy (this change touches the replay path that `arena_gate`'s
corpus clause and the differential gates exercise).
