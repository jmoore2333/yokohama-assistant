# PR #5 — Pre-Phase-3 hardening + live-tracking observation gaps

**Status:** SPEC — written 2026-07-21 at the close of PR #4 (`efed61f`), from the third review's
deferred items plus two findings the PR #4 fix waves surfaced themselves.
**Predecessor:** PR #4 (staggered topology, canal any-to-any, finding D1, CLI v1-beta frontend,
SEMANTICS_REV 2). Everything below was deliberately *not* folded into it — see §0 for the rule used.
**Successor:** Phase 3 (computer-player ladder). This PR is the last engineering before that spec.

---

## 0. The line this spec inherits

PR #4's triage rule was: *fold in anything that can corrupt a live tracked game or that leaves the PR
internally incoherent; defer anything that needs design and cannot bite the gate game.* Everything
here is on the defer side of that line **except §1**, which is a live-tracking correctness gap that
was deferred only because it needs engine work and a full gate re-run.

**Sequencing decision required from Jeremy at kickoff:** §1 items affect the live physical gate game.
Either (a) land §1 before the gate game, or (b) play the gate game with the §1 caveats in hand (they
are stated in PR #4's testing notes and in `guidingpath.md`'s carry-forward). Both are defensible;
(a) is cleaner if the gate game is not imminent.

---

## 1. Live-tracking observation gaps (gate-game relevant) — **the headline**

The engine models several physical setup facts that the live wizard never observes. Where the engine's
assumption and the table disagree, the divergence is silent or late-detected.

### 1.1 5-Power token placement is decided by the RNG seed, not the table — **NEW-1**

`SetupObservations` (`crates/yokohama-core/src/game/setup.rs`) has no field for 5-Power tokens.
`reconcile_board` takes the *seeded* `layout.five_power_tokens` values and relays them over the new
non-Canal area ids **in ascending order**. So which 5-Power bonus sits on which area is a function of
the seed, and nothing lets an operator correct it. PR #4's new board-confirmation check cannot catch
it either: `board_grid` draws `*` for "unclaimed", not the token's face.

At the table this surfaces at the moment of use — the CLI names the bonus it is offering, so a
discrepancy against the physical token is visible — but there is no way to fix it mid-game.

**Work:** a new observation type carrying token face per area, `reconcile_board` support for it, and a
wizard section that asks. Consider whether the board render should show the token face once known
(that also closes the "can't verify at the check" half). Engine change ⇒ full release-gate re-run;
consider whether it warrants a SEMANTICS_REV bump (it changes setup state derivation, not legality —
decide deliberately and record the reasoning either way).

### 1.2 The 2P dummy pawns are unobserved — **NEW-2** (telling half already shipped)

Tracks are not in `SetupObservations` at all, so nothing observes or reconciles the two neutral dummy
assistants the engine places at 2P (Church faith-7, Customs space-4). An operator who never places the
physical pawns gets: a tool that refuses two spaces the table sees as open, a game that ends a pawn
early on either track, and both majority bonuses scored against a rival that is not physically there.

PR #4 shipped the **telling** half — the live wizard's board-confirmation screen now instructs a 2P
operator to place them and explains why (wave D, `4e2bc16`). The observing half remains: either add
track state to `SetupObservations` and reconcile it, or add an explicit confirmation the operator must
answer (weaker, but far cheaper).

### 1.3 Sweep for the same class

Both of the above are instances of one pattern: **a physical setup fact the engine derives rather than
observes.** Do a deliberate inventory of `PendingSetup`/`GameState` setup-time state against what the
live wizard actually asks, and record the complete list with a disposition for each (observe / instruct
/ accept-and-document). That inventory is the deliverable, not just the two fixes.

---

## 2. Hardening the third review de-rated from Critical

### 2.1 An invariant pinning "2P state ⟹ dummy present" — **I9**

`check_invariants` has **no clause about track-slot owners at all**. Since the D1 refactor the engine
derives the faith-7/space-4 block from `slots[3].is_some()` while the oracle derives it from
`player_count == 2` — so a 2P state with an empty faith-7 is representable and the two crates would
disagree about it.

The review confirmed **no production path can build such a state** (`new_game`, `reconcile_setup`,
`finalize_setup`, record replay and `Session::load` all funnel through `place_setup_dummies`;
`GameRecord` carries no serialized `GameState`) and the differential would go red almost immediately
if one appeared. So this is hardening — worth doing because ~15 lines moves the defence from the
release tier into the per-edit, mutation-proven, fuzz-exercised net, **and because Phase 3 will
construct states outside `new_game`.**

Related, same area: `PlayerId::DUMMY` is `u8::MAX` and `state.player()` is a raw index, so a future
consumer mapping a slot owner to a `PlayerState` panics; only `render::track_occupant` guards it today.
See also §4's type note — `try_new` *rejects* `u8::MAX`, so any future validation pass over a
deserialized slot owner would reject every legitimate 2P save.

### 2.2 A seed-free geometry property test — **I10**

The layout-fidelity pins have one specific blind spot, verified by re-implementing `neighbor_cells`
and mutating each branch: a **row-above off-by-one alone** keeps rows 6/5/4/3, exactly 38 edges, max
degree 6, and passes the notch pin. The cause is structural — `staggered_adjacency` emits an edge only
when `nid.0 > id.0`, so every cross-row edge comes from the upper cell's row-below branch, making the
row-above branch dead *for edge generation*. It is **not** dead for `cells_adjacent`, which
`derive_canal_crossings` uses as its disjointness test — so that mutation tolls a free edge and leaves
a genuine canal-neighbour pair connected by nothing, while every count stays perfect.

It is caught today only incidentally, by two seed-pinned exact-set tests whose failure message is a
bare list inequality — and whose cheapest "repair" is pasting in the new list, which is precisely the
failure mode that lets a topology regression through.

**Work:** a seed-free property test over every cell of every board — `neighbor_cells` symmetry,
`cells_adjacent ⟺ staggered_adjacency` membership, `canal_crossings ∩ brick_edges = ∅`, and every
canal-neighbour pair connected. It should *replace* the brittle seeded lists, not join them.

---

## 3. Minor findings from the third review (~35), triaged

Grouped by what they are, not by where they were found. Each is small; the value is in doing the sweep
deliberately rather than one at a time.

### 3.1 CLI behaviour
- A numeric-but-nonexistent id (`A99` on a 10-area board) is accepted into the Option A selection;
  `confirm` then submits it, the engine rejects it, and **the picker exits discarding the whole
  selection**. Validate at toggle time; keep the selection on rejection.
- Over-limit selection gets the same generic message as a bad id.
- Bank shows no yield preview; **Customs prints no outcome line at all** when applied, so it reads as
  a silent no-op.
- The §8 view restyle skipped `score`/`log`/`final_score`/`hand`, and its pinning test enumerates only
  the six views living in `render.rs`, so it structurally cannot see the four that do not.
- `reveal_cmd` discards its delta, swallowing an autosave warning (same family as the `undo` fix).
- A malformed `config` in a save **panics** rather than erroring (`row_lengths`, `player_colors`,
  `start_player`); `has_port_b: true` on a 2P save loads silently; and
  `validate_game_state_consistency` — which checks exactly that — has **zero production callers**.
- `reveal <seat>` is the only entry path whose key never appears on the game screen (learnable from the
  wizard transcript and `players` ordering). A seat number in the players panel closes it.

### 3.2 Rules text (`labs`, previews, track choices)
- Winery/Brickyard's "on the Church"/"on Customs" collides with the Placement step — they fire only on
  a *track action* seating a pawn.
- Track-space choices print VP but drop the printed rider that `tracks` already computes.
- The Laboratory "5th-tech fee" label is fixed at "5th" for the 6th and 7th slots.
- Postal System does not name the fee's recipient.
- **Upheld as correct, do not "fix":** Tram's omitted "may not move again" (structurally unreachable —
  one Movement move per turn, and Tram is emitted as a destination, not a route prefix) and
  Gaslight/Electrical Light's omitted timing clause (build-trigger phrasing is inherently
  non-retroactive). Both were independently challenged and survived.

### 3.3 Comments and citations
- `repl.rs` still carries retired rectangular widths in one test doc comment, contradicted 18 lines
  below by the corrected numbers.
- `render.rs` documents `-` for an empty build slot where the code emits `·` — in the same doc comment
  whose second paragraph says `·`.
- `render.rs:453-455` asserts a possibility the geometry excludes.
- Three `design.md:67` citations resolve to unrelated text in a file that does not exist.
- `piped_dogfood.rs` names a function (`deadline_hit`) that exists nowhere.
- The `legal_moves.rs` Telegram fixture still speaks "orthogonal"/"flank pair".
- New `pub` helpers that panic (`row_lengths`, `cell_to_area_id`, `area_id_to_cell`) carry no
  `# Panics` section, though `AreaId::new` sets the in-repo precedent.
- Worth preserving: **no TODO/FIXME/XXX/HACK anywhere in `crates/`.**

### 3.4 Types
- `PlayerId::DUMMY` is `u8::MAX` — a value the project's own boundary constructor `try_new` **rejects**
  (asserted in `types/tests.rs`). Any future validation pass over a deserialized slot owner would
  reject every legitimate 2P save. Decide: a reserved-but-valid sentinel, a separate occupant type, or
  an explicit documented exemption.
- `BoardLayout::build()` validates only endpoint existence, so a crossing that is **not** an adjacency
  edge is constructible — and the oracle's fixture helper takes adjacency and crossings as independent
  parameters, so such a fixture would test a topology `derive_canal_crossings` can never produce, and
  pass. The identical check already exists in `invariants.rs`; moving it into `build()` is where
  fixtures actually hit it.
- `player_assistant_count` is `pub` with zero callers and counts the dummy.

### 3.5 Test-infrastructure durability
- `screen::force` is thread-local only under `cfg(test)`, so integration tests share a **process-wide**
  flag. PR #4 serialized the two current callers behind a mutex after reproducing ~2 failures per 80
  runs — but any future integration test that toggles frames must take the same lock. The durable fix
  is making it thread-local unconditionally, which changes non-test behaviour of a `pub fn` and so was
  left out of a fix wave.

---

## 4. Carried forward from PR #4's own list

Already recorded in `guidingpath.md`; restated here so this spec is self-contained.

- **Bitmask-DP route reference.** The oracle's `route_labels_pruneless` enumerates all simple-path
  orderings, so densities beyond the 519 battery are infeasible. An exact DP keyed on
  `(current, visited-mask, toll-count)` is ≤ 18 × 2¹⁸ × maxtolls states with zero semantic change; it
  is graph theory, not engine knowledge, so it dispatches to an independence-controlled oracle agent.
  Spec in the ledger's "T4 PERF ANALYSIS" block. Run only if a future battery needs full density.
- **Picker-modal view passthrough** — modals refuse read-only `view`/`help` (cancel-first); the refusal
  now names the constraint, but passthrough would be better.
- **Frame-width narrowing** — the 140-column bound is layout-only; a warning embedding a long save path
  can still exceed it. Truncating paths in messages is worse; narrowing the layout is the real fix.
- **Fee-note engine accessor** — the placement-fee preview mirrors the rule because
  `calculate_placement_cost` is `pub(crate)`; it is pinned by a scratch-apply differential test.
  Exposing a public accessor removes the mirror.
- **Dummy-takes-2nd-place test**, **`track_awards` debug_assert**, **customs D1 coverage symmetry**,
  **battery literal-bound placement**, **key-purity per-edit variant** — small test-strength items.

---

## 5. Phase 3 prerequisites (do not let this PR absorb them)

Recorded in `guidingpath.md`'s Phase-3 section and re-confirmed by the third review. **These are Phase
3's first tasks, not this PR's** — listed so they are not forgotten and not accidentally started here.

1. **The factored/lazy bot-facing action interface.** `display_options` still materializes
   `session.legal()` before grouping; the measured worst case is 147,467 options (~99.99% retreats)
   under human-like play. Keep the exhaustive `Vec<Move>` surface for tests and live tracking; add the
   lazy interface alongside it, with allocation/latency benchmarks.
2. **Family-weighted rollout/fuzz policy**, so uniform-random retreat selection does not erase the
   states and end triggers the AI needs to learn and validate.
3. **The live physical gate game** (Phase 2's gate) comes before search work regardless.

---

## 6. Suggested execution shape

Subagent-per-task with a review after each, as PR #4 ran. Natural task boundaries:

1. §1.3 inventory first — it may change the shape of §1.1/§1.2.
2. §1.1 (engine + wizard + render) — the only task needing a full release-gate re-run.
3. §1.2 observing half.
4. §2.1 invariant + §2.2 property test (one task; both are `yokohama-core` test-strength work).
5. §3.1 + §3.4 (CLI behaviour + types) — one task, they overlap.
6. §3.2 rules text — **rules-sources-only authoring**, same discipline as PR #4's wave C.
7. §3.3 + §3.5 (comments, citations, test durability) — one mechanical sweep.
8. Docs closeout: decision-log rows, guidingpath carry-forward true-up, CLAUDE.md counts measured at
   head.

**Gate policy:** §1.1 is the only item touching engine semantics; everything else is CLI/test/docs, so
the per-edit tier plus a final full matrix suffices. Re-measure suite counts at head when publishing
them — they moved five times during PR #4 and every stale copy became a finding.
