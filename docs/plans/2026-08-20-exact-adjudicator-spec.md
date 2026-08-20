# The exact bounded-tree adjudicator — SPEC

**Date:** 2026-08-20 · **Status: SPEC, awaiting orchestrator (cross-model) review. NO
implementation has begun.** Branch `phase3.5c-exact-adjudicator`, own worktree, per the
value-architecture checkpoint §9.

**Traceability — the authorities this spec executes, and nothing else rides in.** The instrument
is the named trigger of the Phase-C family freeze §9 ("*the exact bounded-tree adjudicator exists
only as a named trigger: a LIVE family-1 root whose per-policy top sets disagree AT THE FROZEN
PARAMETERS reads UNRESOLVED-pending-exact, and building the exact instrument is the named next
step if confirmatory reads produce that class on live roots*"), scoped by the value-architecture
checkpoint §9's second bullet ("*proceeds in parallel, NOT owner-gated (OD-0 — normal TDD, review,
and the merge-roster rule apply in full), own branch + worktree, `driven*`-only surface,
**final-round-small scope** — it retires family 1's 14 UNRESOLVED-pending-exact roots*"), and
routed by the family runs report §4.1 (the six recorded FAILs whose "*ATTRIBUTION is entangled
with that same contest*"). The Phase-B harness design §3's forward clause is the design
provenance: "*the exact bounded-tree upgrade is layer-2 work and only family 1's in-final-round
roots qualify for it.*"

**The checkpoint's forward correction is binding and is restated here so it cannot be lost:** the
V3 close handover §6's suggestion of an exact read of the g1:113 band **over-reached**. g1:113 is
midgame; no exact read of it is feasible under this contract; §2's structural refusal makes that a
property of the instrument rather than a promise not to try. Generalizing the solver is a separate
feasibility/scope decision and is **not** this lane's.

> ### ⚠ SCOPE FINDING — the "14 roots" figure needs an owner-visible correction
>
> The checkpoint §9 describes the lane as retiring "*family 1's 14 UNRESOLVED-pending-exact
> roots*." **Nine of those 14 are CONTROL-arm roots, drawn by `TriggerRoundNotFinal` — by name
> not the final round.** They sit in the TRIGGER round, with a whole further round after it, and
> the Phase-B harness design had already excluded them in the same sentence that authorised this
> instrument: "*only family 1's **in-final-round** roots qualify for it*" (design §3, line 113).
>
> §6.7's measurement confirms that exclusion independently and decisively: the **smallest**
> control root by every measure (control ordinal 9 — 205 mover-turn nodes, 46 distinct hand-offs)
> **exceeded a 600,000,000-node budget after 811 s on the FIRST of its 51 root candidates.**
>
> This spec therefore scopes admission to the in-final-round class (§2.1) and reports the
> retirable set as **5 UNRESOLVED roots, not 14** (positive 7, 9, 13, 15, 16 — of which 16 is at
> risk) plus **1 FAIL root** (positive 17, at risk). The remaining 9 UNRESOLVED and 5 FAIL roots
> are CONTROL-arm and get a **named structural refusal**, not a quiet omission. **This is the
> lane's first flagged-for-owner item** (§11.1) — it is a scope change against the checkpoint's
> wording, and per checkpoint §7 step 1 a scope change returns to the owner.

---

## 0. Fresh-session entry and what this document is

Read order: `CLAUDE.md` → the value-architecture checkpoint §9 → the family freeze §§2–4, 9 → the
family runs §§2, 4.1 → this spec. Then **verify, never trust**: the verdict map in §6.1 is a
re-measurement at this machine, and a re-runner should reproduce it before quoting anything
downstream of it.

This document fixes the instrument's contract, its epistemic claim, its bounds and refusals, its
surface, and its named pins. It also carries the **spec-phase feasibility measurement** (§6),
because the mandate required the tree scale to be measured before tractability is claimed. Every
figure in §6 is a **recorded observation under a named harness at a named machine, never a
ceiling** (ledger L9).

---

## 1. Problem and mandate

### 1.1 What the frozen instrument could not answer

Family 1's confirmatory blocks read, at the frozen parameters, **14 of 21 admitted roots
UNRESOLVED-pending-exact** — the four declared reply policies genuinely disagree about which
candidates are best. Six further roots read FAIL. The existing `adjudicate` verb is, by its own
first printed line, a **robustness read**: each candidate is forced and the remainder is *played
out* under named policies, so a margin says what those policies reach from the position — not what
the position is worth. When four policies disagree, that instrument has no further resolution to
offer: it has said everything it can say.

The family runs §4.1 named the fork precisely: a subject pick missing all four contested top sets
8–17 times of 20 "*is either poor final-round play or an under-instrumented ground truth*."

### 1.2 The mandate

1. **Retire the UNRESOLVED roots** by supplying an exact top set, so that "in the top set or not"
   stops being policy-relative and each root converts to a resolved verdict.
2. **Supply the attribution the FAILs await** — real final-round misplay versus an
   under-instrumented ground truth.

Both rest on the same fact that makes the instrument possible at all: at an in-final-round root only
**two turns** of play remain (measured, §6.2), so the remainder is genuinely small in the way that
matters.

**The mandate's reach, corrected by measurement (the header's scope finding).** Of the 14 roots the
checkpoint names, **5 are in the final round and are this instrument's** (positive 7, 9, 13, 15,
16); the other **9 are trigger-round CONTROL roots and are refused structurally** (§2.1, §6.7). Of
the six FAILs, **1 is this instrument's** (positive 17) and 5 are control-arm. So the honest
statement of what this lane can deliver is: *up to 5 UNRESOLVED roots retired and 1 FAIL attributed,
with positive 16 and 17 measurably at risk even inside that set* — not 14 and 6. The gap is
reported, never absorbed.

### 1.3 What this instrument does NOT do

It does not replace the robustness read; it **adds the game-theoretic pole** the policies were
proxying. The two answer different questions — "what is this position worth under optimal
opposition" versus "what do these policies reach from it" — and a root's full record wants both.
It changes no contestant, policy, weight table, evaluator, or rule of play; it is analysis-side
only (§9).

---

## 2. Scope fence — final-round-small only, and the refusals that enforce it

### 2.1 The structural admission predicate (checked before anything is expanded)

**A root is admissible iff `state.end_game_triggered && state.final_round` — the IN-FINAL-ROUND
class** (and the root is non-terminal, 2P, and otherwise driveable). This is exactly the positive
arm's own `FinalRoundAlreadyTriggered` predicate, and it is the literal reading of both governing
phrases: the checkpoint's "**final-round-small** scope" and the Phase-B design's "only family 1's
**in-final-round** roots qualify for it."

It is exact rather than heuristic. `apply_move`'s phase advance sets `final_round` only when the
turn passes back to the start player, and `GameState::is_game_over` requires both flags plus a
completed final round. So the predicate partitions cleanly into three classes with three different
remainders:

| root class | predicate | remaining play | admitted? |
|---|---|---|---|
| midgame (g1:113 and every ordinary position) | `!end_game_triggered` | **unbounded** | refused `NotTriggered` |
| trigger round (family 1 CONTROL arm) | `end_game_triggered && !final_round` | rest of this round **+ a full final round** — 3–4 turns at 2P | refused `NotFinalRound` |
| final round (family 1 POSITIVE arm) | `end_game_triggered && final_round` | **2 turns at 2P** (measured, §6.2) | **admitted** |

Two refusals rather than one, because they are different facts and a reader must not read either as
the other. `NotTriggered` says the remainder is unbounded. `NotFinalRound` says the remainder is
bounded but **one whole round too large** — and §6.7 measures how much too large, so the refusal
carries evidence rather than an assertion.

This is what makes "midgame roots refuse structurally" a property rather than a promise, and it is
why g1:113 can never enter this instrument by accident.

**The control arm is refused on measured grounds, not excluded by preference** — §6.7, and the
scope finding in the header. Should a later, materially better solver be built, the predicate is
the one line that would move, and it would move only with the owner ruling §11.1 asks for.

### 2.2 The bounds, and the typed refusal that is the only fallback

Precedent: `route_enum`'s totality is a **pre-flight probe** — `fits_fixed_arrays` checks every
bound BEFORE any write and delegates to the frozen path on failure. Here there is nothing to
delegate to, so **refusal is the fallback**, and it must be typed, printed, and impossible to
mistake for a value.

`ExactError`, a typed enum on the harness's own conventions (one variant per failure domain, a
doc-comment per variant stating why it exists, hand-written `Display` naming what was seen, no `_`
arm):

| variant | fires when | carries |
|---|---|---|
| `NotTriggered` | the root's `end_game_triggered` is false | the root digest; the phrase "midgame root" |
| `NotFinalRound` | triggered but `final_round` is false — the trigger-round class | the root digest; a pointer to §6.7's measurement |
| `NotTwoPlayer` | the root is 3–4P (§3.5 — minimax is unsound there) | the player count |
| `Root` | `RootSource::resolve` failed | the wrapped `RootError` |
| `Unadjudicable` | the root guards refused it (§8.1) | the wrapped `DrivenError` |
| `NodeBudget` | expanded decision nodes exceeded `max_nodes` | the bound, the **counted** node total, the candidate index reached |
| `DepthBound` | decisions from the root exceeded `max_depth` | the bound and the depth reached |
| `MemoBudget` | the transposition table exceeded `max_memo_entries` | the bound and the entry count |
| `ForcedMove` | a move `get_legal_moves` listed is refused by `is_move_legal(Canonical)` or by apply | the candidate and the detail |
| `Unscorable` | `final_result` yielded no standing for a seat | the seat |
| `DeadEnd` | a non-terminal state with an empty legal list | the seat |

**Never approximation, never silent truncation.** A budget hit does not produce a partial table
with the exceeded candidate marked "unknown"; it refuses **the whole table**, on
`AdjudicationError`'s existing grounds — a table missing a candidate reads exactly like a table of a
position with fewer options, and the reading riding on it would be wrong with no way to notice. The
one thing a refusal DOES carry is the **counted lower bound** reached, because that is a real
measurement about the root and it is what tells a later reader whether a bigger budget would help.

**A refusal at one of the 14 is a ROUTING, not a failure, and not a shrunken mandate.** §6.6 names
the roots the measurement already puts at risk and §7.4 fixes their disposition.

### 2.3 What stays out

Generalizing beyond the trigger-fired class; any solver that approximates; any change to the
frozen family instrument's own parameters (§9).

---

## 3. THE EXACTNESS DECISION — exact over what information

**This is the spec's hardest call. It is decided here, justified here, and FLAGGED for orchestrator
review as the lane's first owner-interest item.**

### 3.1 The decision

**Exact = a perfect-information minimax over the TRUE `GameState`, hidden set included, both seats
solved to terminal, values from `final_result`.** The instrument is declared **privileged /
selection-side**, and its first printed line says so (§8.3).

Riding with it, and pre-registered as part of the same decision: a bounded **world-invariance
rider** (§3.4) which is what converts a clairvoyant verdict into an attribution.

### 3.2 Why, in one paragraph

The instrument's job is to supply the pole the four reply policies were disagreeing about, and *a
pole whose value moves with a sampling seed is not a pole*. A per-sampled-world solve would be
"exact" only in the sense a single replication of the robustness read is exact — its answer is a
function of the drawn world, so "the exact top set" would immediately need a replication count and
an ε again, reintroducing precisely the noise model this instrument exists to remove. It would not
buy honesty in exchange, because a perfect-information solve *inside* a determinized world still
hands the opponent clairvoyance within that world; the only genuinely view-honest exact object is
an information-set solve, which this repo has already ruled out of scope on measured grounds
(`mcts-v1` REFUSES `K > 1` at the search precisely because aggregating across worlds is
information-set search). Meanwhile the repo already has the governing precedent for a truth-reading
analysis helper: `true_margin_now` is documented as "*Full-state truth, hidden cards included — a
SELECTION-side helper … it exists so family 2's generator can supply the arithmetic canon 15 is
about instead of testing whether the subject can reconstruct it*", fenced by "*nothing in the
evaluator may read it*." The exact adjudicator is that same kind of instrument one layer up, and it
inherits that fence verbatim.

### 3.3 The cost of the choice, stated rather than hidden

A clairvoyant top set is an **upper bound on achievable quality**. The subject is honest and cannot
see the hidden five; therefore **a pick outside the exact top set is NOT, by itself, misplay** — it
may be the correct decision under the information the subject actually had. Any reading that
collapses "outside the clairvoyant set" into "bad move" is a misuse of this instrument, and the
printed label (§8.3) says so in those terms.

Equally: both seats are solved **optimally**, so the exact value is "best against best defense."
A candidate that loses to perfect defense may still be the best practical choice against a fallible
opponent. This is a second reason the exact read *adds a pole* rather than replacing the robustness
read.

What the hidden set actually does here is not hypothetical: in the remaining round the opponent's
unfulfilled starting order can still be fulfilled and scored, and a port refill draws from the
`order_deck` in an order the subject cannot see. Both are causally live in the tree this solver
walks. That is exactly why the rider exists.

### 3.4 The world-invariance rider — what converts a verdict into an attribution

Pre-registered, bounded, and reported beside the primary read:

> Over `W` determinized worlds drawn from the **mover's own `PlayerView`** via the existing
> `BeliefSampler` (conservation-consistent, the honest chain's own sampler), solve exactly in each
> and record whether the exact top set is **identical** to the true-state top set.

Reading, fixed now:

- **Top set invariant across all `W` worlds and the true state** → hidden information does not bear
  on this decision, and a pick outside the set is **unambiguous misplay** at this root.
- **Top set moves** → the clairvoyant verdict is **confounded by hidden information** and is
  recorded as such. It still resolves the *policy disagreement* (the primary mandate), but it does
  not license a misplay claim.

`W`, the draw derivation, and which roots pay for the rider are pre-registration parameters. The
recommendation, sized off §6: **run the rider at the six FAIL roots (where attribution is the
question) and at any UNRESOLVED root whose exact table is close** (top two candidates within the
declared ε), not at all 21 — the rider multiplies solve cost by `W` and §6 shows the solve is
already the expensive half. This is a **flagged decision**: it is the one place the spec trades
completeness for wall, and a reviewer may reasonably want it everywhere or nowhere.

The rider is the ONE place a seed enters this lane. It draws worlds and therefore takes a
**recorded master** under the standing policy — declared here so §5.5's "no seeds" claim is exact
about which component it covers.

### 3.5 Both seats, and why the value is well formed

**Both seats are solved — a full minimax over the remaining rounds.** Anything less makes the
candidate's value a function of an assumed opponent policy, which is the robustness read's known
limitation and the reason this instrument exists.

At 2P the margin is **antisymmetric** — `margin(p0) = total(p0) − total(p1) = −margin(p1)` — so the
game is zero-sum in the margin and alpha-beta is valid. This is asserted by a pin (§8.1), not
assumed. **The instrument is 2P-only in this version**: at 3–4P "best rival by rank" is not
antisymmetric and minimax does not apply; a 3–4P root is refused. Family 1 is 2P throughout, so the
restriction costs the mandate nothing, and stating it keeps a future caller from walking into an
unsound solve.

---

## 4. Terminal values

The leaf value is **`final_result`'s official standings and nothing re-derived**, exactly as
`adjudicate_root`'s `verdict` computes it:

- **margin** = the root mover's final total minus the **best rival's by RANK** — the first non-mover
  row of standings already ordered by (total descending, then the clockwise tie-break from the
  start player). Never a bare `max` over totals, which would pick an arbitrary one of several tied
  rivals.
- **margin 0 is NOT a win.** `won` and `rank` ride beside the margin, both straight off
  `final_result`.

For minimax a strict total order is needed, so the exact value is the pair **`(margin, won)`
ordered lexicographically**, from the ROOT MOVER's perspective. At 2P `won` is a function of
`margin` plus a constant (the tie-break winner is fixed once the start player is), so the second
component only ever separates `margin == 0`; carrying it explicitly is deliberate belt-and-braces
so the "margin 0 is not a win" rule cannot be lost inside an arithmetic shortcut. A pin asserts the
ordering agrees with `final_result`'s own `winner`/`rank` at a constructed tie.

Reported values are in **VP**, signed, from the root mover's perspective.

---

## 5. Tree mechanics

### 5.1 What a node is

A node is a **decision point**, and the enumeration is **queue-first**: at a state with a pending
choice, the node's children are that choice's resolutions rather than the phase's moves. This is
not an implementation convenience — it is `adjudicate_root`'s documented convention and it is
correct: "*the decision the mover actually faces there IS the queued choice, and enumerating the
phase's moves instead would adjudicate a decision nobody is being asked to make.*" A turn is a
phase state machine with choice resolution points inside it, and every one of them is a real node.

Consecutive nodes are frequently the **same** player: a turn is a contiguous run of one player's
decisions (the `drive_turn` dual stop — `current_player` changes, or the game ends). Alpha-beta
handles this unchanged; it is minimax over a game where the mover happens to repeat. §6 shows this
structure is the single most important fact about the tree's shape.

### 5.2 The move source, and why enumeration is total here

**`get_legal_moves` — the engine's own authority — at every node, and nothing else.** Three reasons,
in order of weight:

1. `CLAUDE.md` states it plainly: "`get_legal_moves` stays the authority"; `search_actions` and
   `route_enum` are additive bot-facing sources. An *exact* claim must rest on the authority, not on
   a reconstruction of it, however well pinned.
2. **Totality.** `search_actions`' placement family reports `FamilySize::UpperBound`, not an exact
   count — it is built to page and sample within a family, not to guarantee an exhaustive walk.
   `get_legal_moves` is exhaustive by construction.
3. **Row-for-row comparability.** `adjudicate_root` takes its root candidates from
   `get_legal_moves`, so the exact table's rows join the frozen table's rows by construction rather
   than by trust — which is exactly what §7's per-root output needs.

Application uses **`apply_move_unchecked`** (the driver's discipline for moves already drawn from
the legal list), and **`is_move_legal(.., Canonical)` is checked at the ROOT candidates only** —
mirroring `adjudicate_root` exactly, including its `ForcedMove` refusal on a mismatch. It is not
paid at interior nodes, where it would re-verify an answer `get_legal_moves` just gave.

### 5.3 Search

**Alpha-beta**, full-window at the root, over the `(margin, won)` value.

**The root is searched with a FULL WINDOW per candidate — no cut is taken across root siblings.**
This is load-bearing and costs real time: the mandate needs *every* candidate's exact value (the
top set, the subject's pick's value, and the gap between them), and alpha-beta across root siblings
returns bounds for the non-best ones, not values. A best-move-only search would be far cheaper and
would not answer the question asked.

Interior nodes prune normally.

### 5.4 Memoization — a measured choice, not a default

Transposition table keyed on `state_digest`, storing `(value, bound-flag ∈ {Exact, Lower, Upper})`
— the standard alpha-beta TT discipline, because under a window a stored value may be a bound.

**Stored at TURN-ENTRY states only** (a state whose mover differs from its parent's), so the digest
cost is proportional to hand-offs rather than to nodes. This is the measured choice, and §6.5
records the experiment that decided it: storing **every** node cut the expanded-node count 3.2×
(108.1M → 33.9M) but made the solve **4.1× slower** in wall (121 s → 494 s) and grew the table to
10.6M entries, because `state_digest` per node costs more than the nodes it saves. The
turn-boundary table is also where the value is: a whole opponent-turn subtree is a pure function of
its hand-off state, and §6.3 measures 1.7×–597× hand-off transposition collapse depending on the
root.

Recorded as an implementation option, **not assumed**: a cheap fixed-width (u64) position hash
would change this arithmetic and may make wider memoization win. If it is tried, it is *measured*
against the turn-entry baseline, and a hash collision must be impossible-by-construction or checked
— an exact instrument may not carry a probabilistic key silently.

### 5.5 Determinism, and the seeds this instrument does not draw

**The exact solve draws NO randomness, takes NO master, and derives NO seed.** Its answer is a pure
function of `(root, rules, bounds)`. Two consequences, both stated because they are contracts:

- The runner's exact subcommand **takes no `--master`** and refuses one if supplied — the
  `adjudicate` verb's `--master` is required precisely because playouts need worlds; an exact table
  that accepted a master would be advertising a dependence it does not have.
- **No new salt serial is consumed.** `0xA3E1_D21E_0000_0009` remains the family's next free
  serial after this lane, and a pin asserts the instrument declares no salt. (The §3.4 rider, if
  built, draws worlds and DOES take a recorded master — that is the single exception, and it is
  scoped to the rider.)

**Ties are never broken.** Where several moves share the optimal exact value the instrument
**reports the full optimal SET**. This is not a display nicety: the family rule asks whether the
subject's pick is *in* the top set, so the set is the answer, and picking one representative would
manufacture a distinction the position does not contain.

### 5.6 Cost, stated because the bounds are what contain it

Cost is the exhaustive remainder under alpha-beta and is dominated by `final_result` evaluations at
terminal leaves. It is **not** knob-tunable: the sizing choice is the ROOT, and the bounds in §2.2
are a refusal mechanism, never a quality dial. §6 is the measurement.

---

## 6. FEASIBILITY — measured in the spec phase

**Machine tag: Apple M5 Mac (Darwin 25.6.0), release profile, single-threaded, toolchain 1.97.1.**
All figures are recorded observations under the harness named in each subsection — **never
ceilings** (L9). Harness: a throwaway measurement crate outside the repo with a path dependency on
this worktree's `yokohama-core`; it adds no repo surface and is not committed.

### 6.1 The verdict map REPRODUCED (the precondition for everything below)

Both frozen confirmatory blocks re-run at this machine, commands verbatim:

```
cargo run --release -p yokohama-core --example driven -- family final-round --block confirmatory
cargo run --release -p yokohama-core --example driven -- family final-round --block confirmatory --arm control
```

| arm | wall (M5) | wall (WSL, runs doc) | drawn | admitted | verdicts |
|---|---|---|---|---|---|
| positive | **56.334 s** | 1 m 9 s | 16 | 7 | **1 PASS · 5 UNRESOLVED · 1 FAIL** |
| control | **3 m 24.270 s** | 4 m 10 s | 16 | 14 | **9 UNRESOLVED · 5 FAIL** |

**This matches the runs report §4.1 exactly, including the per-root miss counts.** The FAIL misses
reproduce as positive ordinal 17 → 11 misses, and control ordinals 6/8/10/11/13 → 17/11/17/17/8 —
the runs doc's "17, 11, 17, 17, 8" in order. Counter check clean; no counter-refusal anywhere.
**No cross-machine divergence: nothing to flag.**

The resulting root map, which the rest of this spec addresses:

| arm | UNRESOLVED ordinals (14) | FAIL ordinals (6) | other |
|---|---|---|---|
| positive | 7, 9, 13, 15, 16 | 17 | 5 PASS; 4/6/8/10/11/12/14/19 FORCED, 18 DEAD |
| control | 4, 5, 7, 9, 12, 14, 15, 18, 19 | 6, 8, 10, 11, 13 | 16, 17 FORCED |

### 6.2 The structural finding that decides feasibility

A turn is a contiguous run of one player's decisions, so the remaining tree at a family-1 root is
**max over the mover's whole turn, then min over the opponent's whole turn**, with very few
alternations — not an alternating tree of depth 13.

**Stated at the grain it was measured, not one grain wider:** the opponent layer was walked
exhaustively at **one** in-final-round root (positive ordinal 9, 25 distinct hand-offs sampled —
§6.4), and **every one of those opponent turns ended the game: 0 further hand-offs**, so the
remainder there is exactly two turns. The mover layer was walked at **all** admitted roots (§6.3),
and at every in-final-round root it produced 0 terminal hand-offs — i.e. play always passes to the
opponent — which is consistent with the two-turn shape but does not by itself establish it beyond
ordinal 9. Confirming the two-turn remainder across the other in-final-round roots is cheap and is
implementation-phase work; the spec does not assume it.

This is why "final-round-small" is true in the way that matters, and it is also why the naive
first-impression numbers mislead: a first-legal-move principal line at these roots is only 13–14
decisions long but passes through Movement nodes 654, 870 and 968 wide, whose naive width product
is 10^6.7–10^9.6. The *tree* is not that, because the wide nodes sit inside one turn and their
subtrees converge on a small set of hand-off states.

### 6.3 The mover-turn layer (exhaustive, every admitted root)

Nodes expanded, hand-offs reached, **distinct** hand-off states, wall. Budget 40 M nodes.

**Positive arm**

| ordinal | verdict | root cands | nodes | hand-offs | distinct | collapse | wall |
|---|---|---|---|---|---|---|---|
| 5 | PASS | 139 | 85,431 | 36,237 | 6,520 | 5.6× | 601 ms |
| 7 | UNRESOLVED | 11 | 23,087 | 11,363 | 5,520 | 2.1× | 185 ms |
| 9 | UNRESOLVED | 11 | 3,569 | 1,543 | 886 | 1.7× | 25 ms |
| 13 | UNRESOLVED | 65 | 123,405 | 44,202 | 14,143 | 3.1× | 752 ms |
| 15 | UNRESOLVED | 139 | 569,539 | 281,191 | 28,064 | 10.0× | 4.71 s |
| **16** | UNRESOLVED | 2 | **>40 M** | ≥11,034,816 | **≥547,650** | — | 203.6 s, **EXCEEDED** |
| **17** | **FAIL** | 66 | **>40 M** | ≥11,034,816 | **≥547,650** | — | 210.9 s, **EXCEEDED** |

**Control arm**

| ordinal | verdict | root cands | nodes | hand-offs | distinct | collapse | wall |
|---|---|---|---|---|---|---|---|
| 4 | UNRESOLVED | 61 | 22,388 | 7,442 | 1,082 | 6.9× | 153 ms |
| 5 | UNRESOLVED | 61 | 367 | 122 | 92 | 1.3× | 2 ms |
| 6 | FAIL | 61 | 39,893 | 14,484 | 2,740 | 5.3× | 274 ms |
| 7 | UNRESOLVED | 61 | 672 | 244 | 220 | 1.1× | 4 ms |
| 8 | FAIL | 51 | 10,456 | 2,601 | 496 | 5.2× | 59 ms |
| 9 | UNRESOLVED | 51 | 205 | 51 | 46 | 1.1× | 1 ms |
| **10** | **FAIL** | 51 | **33,877,115** | 16,772,784 | 28,068 | **597.6×** | **322.9 s** |
| 11 | FAIL | 51 | 601,074 | 297,504 | 6,708 | 44.4× | 5.63 s |
| 12 | UNRESOLVED | 61 | 152,623 | 59,536 | 10,220 | 5.8× | 1.17 s |
| 13 | FAIL | 71 | 2,912 | 1,136 | 896 | 1.3× | 23 ms |
| 14 | UNRESOLVED | 91 | 79,702 | 31,844 | 4,864 | 6.5× | 617 ms |
| 15 | UNRESOLVED | 91 | 911 | 364 | 292 | 1.2× | 7 ms |
| 18 | UNRESOLVED | 61 | 27,509 | 7,842 | 1,172 | 6.7× | 156 ms |
| 19 | UNRESOLVED | 61 | 428 | 122 | 92 | 1.3× | 2 ms |

Two facts the arms do not share, and which is why §2.1 refuses to treat family 1 as one population:
the control arm's mover turns are **shallower** (max depth 4–7 decisions versus 11–17) because those
roots sit mid-turn in the trigger round, and its remainders are **longer** (3–4 turns, not 2),
because a whole final round still follows.

### 6.4 The opponent-turn layer

Positive ordinal 9, 25 distinct hand-offs sampled and solved exhaustively:

- **mean 387,579 nodes per opponent turn**, max depth 15, max branch 186;
- **every one terminated the game** — 0 further hand-offs;
- ~179 K terminal leaves per turn collapsing to ~23 K distinct states (**7.7×**), which is what
  makes `final_result` evaluation, not node expansion, the dominant cost.

Naive product for that root: 886 distinct hand-offs × ~387 K ≈ **343 M nodes**.

### 6.5 A COMPLETE EXACT SOLVE, and the memoization experiment

The alpha-beta + turn-entry-TT prototype run to completion at **positive ordinal 9** — an
UNRESOLVED root, i.e. one of the 14:

```
SOLVE COMPLETE  121,049 ms   nodes = 108,080,097   tt_entries = 886   tt_hits = 657
best margin     −15 VP
argmax set      1 of 11 candidates (ε = 0)
ε = 2 VP set    1 of 11 candidates
value spread    18 VP  (best −15 VP; runner-up 17 VP behind)
```

**The instrument answers, and it answers sharply**: a unique optimal candidate, 17 VP clear of
every rival, at a root where the four declared policies could not agree. That is the mandate's
central claim demonstrated once, end to end, rather than argued.

Alpha-beta pruned the 343 M naive walk to 108 M (≈3.2×) with **no move ordering at all** — the
engine's natural order. Move ordering is therefore the obvious and unexploited lever, and it is an
implementation-phase measurement, not an assumption in this spec.

**The memoization experiment (§5.4's evidence).** The same root with the TT storing *every* node:

| TT scope | nodes | wall | entries |
|---|---|---|---|
| turn-entry only | 108,080,097 | **121.0 s** | 886 |
| every node | **33,923,520** | 494.3 s | 10,642,412 |

3.2× fewer nodes, **4.1× more wall**. `state_digest` per node costs more than it saves. Turn-entry
memoization is adopted on this measurement.

### 6.6 Tractability across the POSITIVE (in-final-round) arm

Solve cost tracks **distinct hand-off count** far more closely than root width. Taking ordinal 9
(886 distinct → 121 s, measured) as the anchor, and stating the rest as the crude proportional
extrapolation it is rather than a prediction:

| root | verdict | distinct hand-offs | status |
|---|---|---|---|
| pos 9 | UNRESOLVED | 886 | **SOLVED, 121 s (measured)** |
| pos 7 | UNRESOLVED | 5,520 | extrapolates to tens of minutes |
| pos 5 | PASS (control read) | 6,520 | tens of minutes |
| pos 13 | UNRESOLVED | 14,143 | ~hours, uncertain |
| pos 15 | UNRESOLVED | 28,064 | ~hours, uncertain |
| **pos 16** | **UNRESOLVED** | **≥547,650** | **at risk — mover turn ALONE exceeds 40 M nodes** |
| **pos 17** | **FAIL** | **≥547,650** | **at risk — same, and it is an ATTRIBUTION root** |

**Naming the intractable, per the mandate's instruction not to shrink the scope quietly:** positive
ordinals 16 and 17 exceed a 40 M-node budget in the mover turn alone, before any opponent reply is
considered — and **ordinal 17 is the positive arm's only FAIL root**, so the attribution half of the
mandate is the half most at risk. Their disposition is §7.4's, and it is a permanent honest record,
not a smaller promise. Move ordering (§6.5: currently none, and alpha-beta still pruned 3.2×) is the
first unexploited lever and is measured before either root is declared refused.

### 6.7 The CONTROL arm, measured — the evidence behind §2.1's structural refusal

The control arm's roots are `TriggerRoundNotFinal`: the trigger has fired but the final round has
not begun, so **a whole further round follows this one** — 3–4 turns at 2P against the positive
arm's 2. Its mover turns are *smaller and shallower* than the positive arm's (§6.3: max depth 4–7
decisions, several roots under 1,000 nodes), which makes it the arm that looks cheapest and is in
fact the arm that is out of reach. The measurement:

> **Control ordinal 9 — the SMALLEST control root by every mover-turn measure (205 nodes, 46
> distinct hand-offs, 1 ms to walk its mover turn) — EXCEEDED a 600,000,000-node budget after
> 811,229 ms, still inside the FIRST of its 51 root candidates.** TT entries 245, hits 15.

Three things that reading tells us, none of them an extrapolation:

1. **The blow-up is entirely past the mover's turn**, in the extra alternations. The mover turn is
   1 ms; the remainder did not finish in 13.5 minutes.
2. **The transposition table cannot rescue it.** 245 entries after 600 M nodes, because a
   turn-entry value is stored only when its subtree *completes*, and at this root none did. The
   layer sizes grow multiplicatively across the extra round — 46 hand-offs × ~10³–10⁴ per
   subsequent turn, compounding — so there is no layer at which memoization collapses the problem.
3. **The cheapest control root is the strongest possible test**, and it failed by a wide margin
   rather than narrowly. This is not a tuning gap.

**Therefore the control arm is refused structurally (§2.1), with the measurement as the reason** —
and the Phase-B design's "only family 1's in-final-round roots qualify for it" is confirmed rather
than merely cited. **This removes 9 UNRESOLVED and 5 FAIL roots from the retirable set**, which is
the scope finding this document opens with and §11.1 flags.

Recorded honestly, because it bounds what this measurement licenses: it says *this* solver, without
move ordering, does not reach *this* root within 600 M nodes. It does not prove no solver could.
A materially different instrument for the trigger-round class is a separate feasibility and scope
question, and this lane does not open it.

---

## 7. Output design and how a verdict converts

### 7.1 The per-root exact table

Rendered as a `String` by the lib (never a stream of `println!`s — the runner's own rule, which is
what makes a report testable), with the epistemic label as line 1 (§8.3), then provenance in the
harness's existing shape (root as asked for and as resolved, semantics rev and rules hash, mover,
bounds), then:

```
  #   exact margin    gap    optimal?   candidate
  1        +12 VP    0 VP       *       <move>
  2         +9 VP    3 VP               <move>
  ...
  exact top set (eps=0)     k of n candidates
  exact top set (eps=2 VP)  k of n candidates
  nodes expanded            N        tt entries M / hits H
  subject picks (20 reps)   in-eps0 a/20   in-eps2 b/20   misses c/20
  frozen adjudicator        family-weighted k1 · greedy-at-final k2 · lookahead-at-final k3 · race-rusher k4
```

The frozen instrument's four contested top-set sizes are printed **beside** the exact set, because
the whole point of the root is that they disagreed and the reader wants to see the disagreement
next to its resolution.

`gap` is in VP, positive, from the best. Absent quantities print `-` and never `0` — the harness's
counter-absence rule.

### 7.2 The subject's picks are a distribution, not a move

Load-bearing and easy to get wrong: the family read runs **20 replications** and the subject may
pick differently in each (it samples a world per decision), which is why the frozen reports print
per-replication `pick` lines and count "misses N of 20". The exact instrument does not change that
— it changes only **what the picks are measured against**. UNRESOLVED existed because four policy
sets disagreed; the exact set is **one** set, so every replication is unambiguously in or out.

The frozen **per-root allowance F = 1 of 20 stays**, because the miss count is still a random
quantity over the read's replications even though the top set no longer is.

### 7.3 ε on an exact read

The freeze's **ε = 2 VP** was derived as noise cover (positive-arm per-candidate mean-margin sd
0.963, 95 % UCB 1.535). An exact read has **no sampling noise**, so ε is no longer noise cover —
but value-equal and near-value-equal optima are real, and the family's declared tolerance is a
separate thing from its noise model.

**Pre-registered: ε = 2 VP remains the PRIMARY verdict rule; ε = 0 (the true argmax set) is
recorded beside it.** The reason is comparability: changing the instrument *and* the tolerance in
one step would confound "the instrument got sharper" with "the bar moved," and the six FAILs must
be re-read on the tolerance they failed under. The ε = 2 set provably **contains** the ε = 0 set, so
the leniency direction is pinned exactly as the freeze pinned it. Recorded honestly: at ε = 2 on an
exact read, ε is a **declared indifference band**, not noise cover, and the spec calls it that.

**This is a flagged decision** — a reviewer may prefer ε = 0 primary on the grounds that an exact
instrument should not inherit a noise parameter.

### 7.4 The verdict conversion

| exact reading | converts to |
|---|---|
| ≤ 1 of 20 picks outside the exact ε-set | **EXACT-PASS** (retires UNRESOLVED) |
| ≥ 2 of 20 picks outside | **EXACT-FAIL**, reported with the mean VP gap of the missing picks |
| root exceeded a bound | **EXACT-REFUSED**, naming the bound and the counted lower bound reached |

An **EXACT-REFUSED** root **keeps its `UNRESOLVED-pending-exact` verdict**, now qualified with the
bound that refused it. It is not silently dropped, not re-classed, and not counted as resolved. The
family-1 restatement is then made over the roots the instrument actually resolved, **with the
refused ones named in the same sentence**.

**Two refusal kinds, reported apart** — they are different facts about a root and collapsing them
would misrepresent both:

- **OUT-OF-SCOPE** (`NotFinalRound`): a trigger-round root the instrument never attempts. The
  correct record is "this instrument does not answer here," and the family-1 control arm's 9
  UNRESOLVED and 5 FAIL roots keep their frozen verdicts **unchanged and unqualified by any exact
  claim**. Nothing about them is resolved, and nothing about them is worsened.
- **BUDGET-REFUSED** (`NodeBudget` / `DepthBound` / `MemoBudget`): an in-scope root the instrument
  attempted and could not finish, carrying its counted lower bound. This is the disposition
  positive 16 and 17 are at risk of, and the counted bound is what tells a later reader whether a
  better solver would change the answer.

For the six FAILs the output additionally carries the **attribution**, which is the pair (exact gap,
world-invariance verdict) and nothing more:

- pick outside the exact set **and** top set invariant across the rider's worlds → **real
  final-round misplay**, quantified in VP;
- pick outside **and** top set moves with the world → **confounded** — the frozen FAIL is not
  cleared and not confirmed;
- pick **inside** the exact set → the frozen FAIL was an artifact of an under-instrumented ground
  truth, which is the runs report's own stated alternative.

### 7.5 The eventual results document

`docs/plans/YYYY-MM-DD-exact-adjudicator-results.md`: the reproduced verdict map with machine-tagged
walls; the commands as record; per-root exact tables for all 21 admitted roots; the conversion table
above applied; the six FAILs' attribution; every refusal with its counted bound; the walls and node
counts in L9 language; and a **what this does not mean** block on the model of the freeze §10 — in
particular that a clairvoyant top set is an upper bound on achievable quality and that "best against
best defense" is not "best against this opponent."

---

## 8. Surface and tests

### 8.1 Surface

- **`crates/yokohama-core/src/game/driven_exact.rs`** — new module, all logic, all tests. Inside the
  `driven*`-only fence; a new file rather than growth in `driven.rs` (already 10,199 lines).
- **`crates/yokohama-core/src/game/driven.rs`** — ONE minimal change, named here so review can price
  it: the root guards the exact solver must share are currently private
  (`fn require_eligible_root`, `fn preflight` at driven.rs:2453/2491) and get **`pub(crate)`**.
  Re-implementing them would put a second copy of "what is a driveable root" in the crate, which is
  exactly the drift the harness's one-materialization-site discipline exists to prevent.
  `check_invariants` is already `pub` (invariants.rs:21) and needs nothing. **Note the asymmetry
  deliberately**: `preflight` also checks seat count and `max_decisions`, which the exact solver has
  no analogue for — it builds no agents and seats nobody — so the exact path reuses the
  root-eligibility half (the hidden-deal refusal, which matters *more* here: a live-tracked root's
  true state is genuinely incomplete, so there is nothing to be exact about) and states plainly in
  its own doc which half it does not use and why.
- **`crates/yokohama-core/src/game/driven_runner.rs`** — a new `exact` subcommand: grammar, flags,
  refusals and rendering, all lib-tested per §2's rule. `SUBCOMMANDS`, `USAGE` and the
  USAGE-completeness pin extend with it.
- **`crates/yokohama-core/examples/driven.rs` — UNCHANGED.** It is 54 lines of argv glue that
  dispatch to `driven_runner::run`; a new subcommand needs nothing from it. Its own header says
  "THIS FILE IS ARGV GLUE AND NOTHING ELSE / NO TESTS LIVE HERE," and that stays true.
- **NO new documented test command.** Lib tests ride the existing `cargo test -p yokohama-core`.
  Nothing joins the fifteen-gate roster; no `#[ignore]` tier; **tests are corpus-free** and must be
  green with the owner saves hidden.
- **No sidecar serialization** unless a consumer appears — the harness's deferred-until-a-consumer
  rule.

Grammar sketch (the spec fixes the shape; wording is the implementation's):

```
driven exact <root> [--max-nodes <n>] [--max-depth <n>] [--max-memo <n>] [--eps <vp>]
```

`--master` is **refused** with a typed message saying the exact solve draws no worlds (§5.5).

### 8.2 The named pins

1. **Exact-solver correctness on a hand-built tiny fixture** whose optimal set is computable by
   hand: a constructed near-terminal 2P state with a small candidate set and hand-checked finals.
2. **Alpha-beta ≡ pure minimax.** On constructed and small generated fixtures the pruned solve
   returns values identical to a naive exhaustive minimax. *The* correctness pin, and
   **mutation-proven** (a deliberately wrong cut must turn it red).
3. **Memo soundness.** TT on and TT off produce identical exact values at the same roots.
4. **Determinism / no seed.** The exact table is byte-identical across repeated runs; the entry
   point takes no `WorldSeed`; a supplied `--master` is refused; **no salt serial is declared** and
   `0xA3E1_D21E_0000_0009` is still free.
5. **The full optimal SET is reported, never a representative.** A fixture with two value-equal
   optima returns both.
6. **Terminal-value provenance.** Values come from `final_result`'s standings; best rival by RANK;
   **margin 0 is not a win** — pinned against a constructed tie.
7. **Antisymmetry at 2P.** `value(p0) == −value(p1)` over a seeded battery of terminals (the
   premise alpha-beta rests on), and a 3–4P root is REFUSED.
8. **Typed refusals.** `NotTriggered` on a midgame root by name and before any expansion;
   `NodeBudget` naming the bound and the counted total; whole-table refusal (never a partial table).
9. **Root-candidate agreement with `adjudicate_root`.** The exact table's candidate list equals the
   frozen adjudicator's at the same root, so the two tables join row-for-row.
10. **Directional agreement at an admitted PASS root.** At a root where all four policies agree, the
    exact argmax set intersects that unanimous top set. A *directional* pin, deliberately not an
    equality claim: the two instruments answer different questions and the pin must not assert
    otherwise.
11. **The label rides the output.** Line 1 of every exact report is the epistemic const, verbatim —
    the `ROBUSTNESS_READ` pattern and its reason.

Wall discipline: fixtures are constructed or small-generated so the module's debug wall stays
modest; the 22.5 s `why` pin is the standing cautionary figure.

### 8.3 The epistemic label

A single `pub const` — the claim written once so the runner cannot print a weaker one, exactly as
`ROBUSTNESS_READ` is. Its text must carry all four of: **perfect-information** solve over the
**true** state, hidden set included; **both seats optimal**; therefore an **upper bound on
achievable quality**, so a subject pick outside the set is **not by itself misplay**; and
**privileged / analysis-side — never an input to any evaluator, weight fit, or contestant.**

---

## 9. Non-goals and risks

**Non-goals.** Generalization beyond the trigger-fired class (g1:113 and every midgame root refuse
structurally). Any approximate mode. Any change to a contestant, policy, weight table, evaluator,
knob, or preserved rung. Any change to the frozen family instrument's parameters (R, F, δ, ε, the
menu, the admission rules) — the exact read is a **new read beside** the frozen block, and the
already-read confirmatory ordinals 4..20 are the only ones it touches. `SEMANTICS_REV` stays **7**.
No oracle work. No `GameRecord` is emitted — a rooted analysis is not a from-setup game.

**The escrow fence, absolute.** Extension ordinals 20..26 and escrow ordinals 26..29 / 29..32 are
**never** run — not `table`, not `adjudicate`, not `exact`, not a supply probe. Burning is running
at an ordinal. The runs report's commands-as-record is the audit and this lane's results doc joins
it.

**Masters.** The exact solve needs none. The §3.4 rider, if built, records one before use per
standing policy; it never redraws.

**Risks, named.**

1. **The mandate's reach is smaller than the checkpoint's wording.** Measured, not argued (§6.7);
   flagged at §11.1; an owner-visible scope item rather than an implementation detail. **The
   largest risk in this lane is that this finding travels as "the adjudicator retired family 1"
   when it did not.**
2. **Tractability at the wide roots, inside the admitted class.** Measured: positive 16 and 17
   exceed a 40 M budget in the mover turn alone, and 17 is the attribution root. Mitigations to
   *measure*: move ordering (currently none, and alpha-beta still pruned 3.2× without it), a
   cheaper position hash, iterative deepening used purely for ordering. Disposition if they still
   refuse is §7.4's BUDGET-REFUSED, and it is honest rather than quiet.
3. **Clairvoyance mis-read.** The single largest interpretive risk: a reader taking "outside the
   exact set" for "bad move." Mitigated by the label (§8.3), the rider (§3.4), and the results
   doc's what-this-does-not-mean block — and it is the reason §3 is flagged for review.
4. **The exact answer may not change family 1's verdict.** If the exact sets confirm the misses,
   family 1 stays FAIL — with attribution instead of entanglement. That is a legitimate and useful
   outcome, and the spec pre-commits to reporting it as readily as the other.

---

## 10. Implementation order (for the review to react to; no work has begun)

1. **Resolve §11.1 first** — the scope correction is an owner-facing question and the root set is
   an input to everything below it. Nothing is built against a root set that may move.
2. Red-first: the tiny hand-computable fixture and the alpha-beta ≡ minimax pin.
3. The solver, the typed refusals (both refusal kinds), the value type.
4. The runner subcommand, its rendering and its refusals.
5. **Move ordering, measured against the §6.5 baseline** — before positive 16 and 17 are declared
   refused, since they are the two roots it could rescue and one of them is the attribution root.
6. The §3.4 rider, at the pre-registered root set.
7. The admitted-root run and the results document, reporting the refused classes by name.

Per-edit suites plus the lint gate after every task; the fifteen-gate release roster at the merge
head; PR review as standing. Nothing here is owner-gated (OD-0), and nothing here relaxes those.

---

## 11. Flagged for orchestrator review

1. **THE SCOPE CORRECTION (header, §2.1, §6.7) — an OWNER item, not merely a review item.** The
   checkpoint says 14 roots; the measurement plus the Phase-B design's own "in-final-round" clause
   say 5, with 2 of those 5 at risk. Per checkpoint §7 step 1, a scope change returns to the owner.
   The three dispositions: **(a)** adopt the in-final-round predicate as specified and record the
   control arm as out of scope for this instrument (**recommended** — it is what both governing
   documents actually say, and the measurement is decisive); **(b)** keep `end_game_triggered` as
   the predicate and let the control arm BUDGET-REFUSE root by root, paying ~13.5 min per root to
   record 14 refusals we can already predict; **(c)** open a separate feasibility question for a
   trigger-round instrument, which this lane does not scope and did not price.
2. **§3 — the exactness decision.** Clairvoyant-over-truth (recommended, with the rider) versus
   per-sampled-world. The lane's primary epistemic call.
3. **§3.4 — the rider's scope.** The FAIL root plus close UNRESOLVED roots (recommended) versus
   every admitted root versus none; and `W`.
4. **§7.3 — ε on an exact read.** ε = 2 primary for comparability (recommended) versus ε = 0.
5. **§7.4 / §10.5 — positive 16 and 17.** Whether move ordering is measured before they are
   declared BUDGET-REFUSED (recommended: yes — 17 is the only in-scope attribution root, so
   refusing it costs the attribution mandate its entire subject).
6. **§5.4 — memoization**, if the reviewer wants the cheap-hash variant measured before the
   turn-entry choice is fixed.
