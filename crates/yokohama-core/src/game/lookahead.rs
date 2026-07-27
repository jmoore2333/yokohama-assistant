//! **Lookahead-v1** — the shallow-search rung (Phase 3.3 T5; spec §4.2, §4.4),
//! plus the **S2 atomic-with-quiescence DIAGNOSTIC arm** (spec §4.1) that the T7
//! depth experiment exists to adjudicate.
//!
//! # S1, the rung: complete your own turn, then look
//!
//! One decision = for each root candidate (spec §3.3's scheme, reused verbatim
//! from [`crate::game::greedy`]), apply it and then **finish the rest of the
//! root seat's own turn**
//! under a beam of width `B`, evaluating [`eval::score`] at the **hand-off** —
//! the first state whose `current_player` is no longer the root seat, with a
//! finished game counting as a hand-off (the engine leaves `current_player`
//! alone and sets `TurnPhase::TurnEnd`, so `is_game_over` is a second, separate
//! condition and not a special case of the first). The root's value is the best
//! hand-off its continuation reached; the root argmax then takes spec §3.2's
//! uniform draw.
//!
//! The mechanism this buys over [`crate::game::greedy`] is the one R3 named: a
//! per-move argmax has to commit at Placement blind to the AreaAction payoff the
//! placement bought, because that payoff happens in a LATER phase — past a
//! one-ply horizon but well inside a turn. `B` is a knob;
//! **`B = 1` is a greedy continuation** (the frontier holds one node, and its
//! truncation IS the greedy step), which is why the beam's cut is a tie point
//! and not a sort.
//!
//! # Every tie point draws, and the draws are KEYED, not consumed
//!
//! Spec §3.2 at every tie point (§4.2), which here means three:
//!
//! | tie point | site | what a first-wins version would do |
//! |---|---|---|
//! | root argmax | [`TieSite::Root`] | R2/R3's 0.59–0.67 θ̂ swing — the null placement wins every tie |
//! | beam-frontier truncation | [`TieSite::Beam`] | the research critic's unfixed residue: a stable sort keeps the enumeration's first `B` of a tied tail |
//! | S2's quiescence extension | [`TieSite::Extension`] | the same, one line at a time — and, until the T5 review, the same index at every SIBLING line too (the site now carries a [`descend_branch`] coordinate) |
//!
//! All four call sites (three here, one in [`crate::game::greedy`]) share ONE
//! primitive, [`uniform_index`] — Lemire multiply-shift, exactly one word per
//! draw whatever the tie count.
//!
//! **The word is KEYED, not drawn from the play stream** ([`TieStream`]), and
//! that is spec §4.3's requirement rather than a preference: T8's divergence
//! instrument compares the SAME decision across K sampled worlds, and lens 2
//! measured a naive instrument reading **7.66% false divergence at K = 1 from
//! tie-draws alone**. Keying per `(decision, candidate index, depth, slot)` off
//! ONE word drawn per decision makes the draw at a site identical in every
//! world.
//!
//! **Tie draws are only HALF of what has to be world-common, and the other half
//! was open until the T5 review (M2).** The candidate scheme draws too — `M`
//! retreat samples at every Movement node, a reservoir pass at every Placement
//! node — and those draws used to come off the agent's CONTINUING play stream
//! inside the world loop, so world `k` started at a shifted position and two
//! worlds disagreed even where their states were identical (measured: 20/20
//! organic Movement points where searching the SAME world twice under one
//! continuing stream produced different candidate lists, values and lines). Both
//! halves are now keyed the same way — the tie key by [`TieStream`], the
//! candidate dice by [`candidate_rng`] off a second per-decision word — so
//! residual cross-world divergence is determinization-driven and nothing else,
//! and T8's instrument measures determinization alone
//! (`the_candidate_dice_are_world_common_within_a_decision`).
//!
//! # The node and cache shape (spec §4.4, verbatim)
//!
//! A [`DecisionHandle`] borrows `&GameState`, so a node **cannot own one**. A
//! node therefore owns `(GameState, Vec<Move> candidates, i64 eval)` and the
//! handle is a **scoped temporary built at most once per node** — structurally,
//! not by discipline: [`Run::expand_point`] is the module's ONLY
//! `DecisionHandle::new` call site (pinned by
//! `the_module_builds_exactly_one_decision_handle_per_node`), it builds exactly
//! one, and every node is created through it. The pin that makes this
//! executable rather than architectural is the DFS counter below.
//!
//! **The forced fast path is the cached one.** R3 measured ~60% of decision
//! points offering ≤ 1 candidate (re-measured here: **51.9%** of ROOT points
//! and **84.7%** of all search nodes), and DISCOVERING that costs a full
//! enumeration — so the answer is kept on the node (`candidates` IS the cache)
//! and a forced chain re-reads it for free. The named hazard is the
//! duplicate-handle defect the research critic located in R3's own S2 at
//! pending-choice depth-0 states: ask "is this forced?" with one handle and
//! then enumerate with a second, and every forced point costs twice.
//!
//! Two shapes are NOT nodes, deliberately:
//! - a **hand-off** child is a LEAF — evaluated, never expanded — so it never
//!   builds a handle. Enumerating it would be enumerating the OPPONENT's
//!   options, which no v1 scheme reads, and at an opponent's on-board Movement
//!   point it would pay a route DFS the counter pin would then have to excuse;
//! - a **principal continuation** is reconstructed from a [`Trace`] of
//!   `(parent, move)` back-links rather than cloned down every branch: one
//!   `Move` clone per created node instead of a whole line per node.
//!
//! **No per-world trees.** K = 1 ships (spec §4.3). The K knob re-runs the WHOLE
//! per-world search and aggregates at the ROOT ([`LookaheadAgent::choose`]);
//! it never interleaves worlds into one tree, because a node's legal
//! continuation is a property of the world it lives in.
//!
//! # The DFS counter pin, scoped as spec §4.4 scopes it
//!
//! `route_enum`'s counter is thread-local, `cfg(any(test, debug_assertions))`,
//! and **never armed globally** — the arena driver legitimately pays extra in
//! debug. The harness-scoped expectation here is
//!
//! ```text
//! route DFS count == (cache-miss on-board-Movement NODES) × K
//! ```
//!
//! and NOT "nodes expanded": a node whose routes memo is never read costs 0
//! (every non-Movement node, and every in-hand Movement node — those emit
//! `PlacePresident` and walk nothing), and a re-read of a memo costs 0. The
//! left-hand side is [`SearchStats::route_dfs_nodes`], which counts exactly the
//! nodes that FORCE the memo; the `× K` is the sum over the K per-world
//! searches, since [`SearchStats::merge`] accumulates them.
//!
//! # Measured structural constants (spec §2.4's obligation, routed here)
//!
//! **These SUPERSEDE the research notes' §3 figures, which were taken under a
//! provisional weight table before the T3 hand split and — for the hand-off row
//! — under a first-wins tie-break (notes Amendment A1).** Conditions: release,
//! Apple M5, quiet machine, `Rules::standard()`, the FROZEN [`WeightTable::V1`],
//! 2P Lookahead-vs-Lookahead self-play at `K = 1`, diagnostic master
//! `0x1EA4_2026_0727_0001`, per-move invariants OFF; harness = an out-of-tree
//! scratch binary that path-depends on this crate and drives
//! `observe`/`choose`/`apply` itself (nothing below is asserted anywhere). All
//! ten games at each of `B = 4` and `B = 1` RAN TO A REAL GAME END (40.2 and
//! 38.8 turns mean), which is the completion evidence behind the timings.
//!
//! | quantity | S1 `B = 4` | S1 `B = 1` | S2 `d = 2` |
//! |---|---|---|---|
//! | **µs/decision** | **1,268.6** | **550.2** | **1,211.3** |
//! | decisions measured | 3,079 (10 games) | 2,811 (10) | 2,376 (4) |
//! | nodes/decision (= handles) | 315.8 · max 9,048 | 70.3 · max 1,426 | 610.8 · max 31,271 |
//! | successors scored/decision | 856.7 | 384.8 | 865.2 |
//! | root candidates/decision | 23.60 | 21.44 | 16.98 |
//! | forced share, ROOT points (≤ 1 candidate) | **51.9%** | 55.2% | 58.7% |
//! | forced share, ALL search nodes | **84.7%** | 67.9% | 86.7% |
//! | ply-cap hits ([`MAX_TURN_PLIES`]) | **0** | **0** | **0** |
//! | free-extension steps (quiescence) | n/a | n/a | 3.6/decision, max 5,368 in one |
//!
//! **EVERY FIGURE ABOVE IS ONE HARNESS RUN, AND THE SAMPLE SPREAD IS LARGE
//! ENOUGH TO CHANGE HOW YOU READ THEM** (T5 review S4/S5). Three independent
//! replicates of the S1 nodes/decision row now exist — this table, the T5
//! reviewer's run on the SAME master with different agent seeds, and the fix
//! wave's own re-run (10 games, world seed `MASTER + i` per game, seat seeds via
//! `arena::agent_stream_seed`) — and they read **315.8 / 260.1 / 269.1** at
//! `B = 4` and **70.3 / 65.8 / 64.7** at `B = 1`. That is ~15–18% of spread at
//! N = 10 games, which is the honest error bar on every cell here.
//!
//! Two readings the spread governs, both previously stated too strongly. The
//! `B = 1` pair against R3's model (70.3 vs 56.53, 24% apart) already read as a
//! disagreement; the `B = 4` pair (315.8 vs 315.01) agreed to three significant
//! figures **by coincidence**, not because the harness is that repeatable — and
//! the way to see that is that the SAME harness perturbed only by the M2
//! candidate-seed fix moved 272.0 → 269.1 (about 1%), i.e. within-harness noise
//! is an order of magnitude smaller than the 15% between harnesses. Read all of
//! these as observations under a named harness, never as constants.
//!
//! **The S2 column is worse than "spread", and the reason is structural: its
//! per-decision cost is so heavy-tailed that a mean over 4 games is not a
//! statistic.** Re-measured over 10 games (2,716 decisions) POST-REORDER, the
//! distribution is median **40.1 µs** · p90 2,798.5 · p99 40,268 · **max
//! 2,583,353 µs — 2.58 SECONDS in a single decision** (471,632 nodes, 464,686
//! free-extension steps), and dropping that one decision moves the mean from
//! 4,687.7 to 3,737.9. S1 `B = 4` over the same 10 games is median 50.2 · p90
//! 4,181 · p99 14,701 · max 63,917 µs, i.e. its worst decision is **40× smaller
//! than S2's**. The 5,368-free-step worst case this table records is the same
//! pathology at 1/86th the size. T7's cell will meet this tail; sizing it from a
//! mean would under-provision by two orders of magnitude on the worst decision.
//!
//! **The S2 reorder's own cost effect, measured the only way that isolates it**
//! (T5 review S2 — self-play means cannot, because the reorder changes S2's
//! values and therefore its whole game trajectory: mean turns moved 51.0 → 33.5
//! on the 4-game cell). 150 FIXED organic decision points from one Greedy-v1 walk,
//! searched pre and post with everything else held byte-identical:
//!
//! | S2 `d = 2` at 150 fixed points | pre-reorder | post-reorder |
//! |---|---|---|
//! | nodes/point | 1,527.5 | **356.9** (−76.6%) |
//! | successors scored/point | 2,177.7 | **1,085.1** (−50.2%) |
//! | µs/point | 2,748.7 | **1,467.7** (−46.6%) |
//! | median nodes | 13 | 3 |
//! | max nodes | 18,980 | 6,433 |
//!
//! S1 `B = 4` over the same points is the CONTROL and comes out byte-identical
//! either side (54,527 nodes · 273,455 scored · 363.5 nodes/point · median 13 ·
//! max 4,167), which is the evidence that nothing outside `Run::atomic` moved.
//! The line that matters for T7: S2 used to pay **4.2×** S1's nodes at the same
//! points (1,527.5 vs 363.5) and now pays **near parity** (356.9 vs 363.5). §4.1
//! adjudicates on deterministic counts over a no-edge interval, so a 4.2× gap
//! that lived entirely in unread bookkeeping could have decided the experiment.
//!
//! **Hand-off length** — the quantity that sizes turn-boundary cost, measured
//! two ways because they are two different things. The ACTUAL player turn
//! (moves between `current_player` transitions in the played game) is mean
//! **7.66** · med 7 · p90 10 · p99 13 · **max observed 18** at `B = 4`; the
//! SEARCHED line (root → hand-off, i.e. the REMAINDER of a turn from a decision
//! point that is usually already mid-turn) is mean **4.23** · med 4 · p90 7 ·
//! p99 11 · max 14. The research row (mean 8.01 / p90 11 / p99 14 / max 15) is
//! the first of these under eval-v0; note the max is now **18**, which is why
//! [`MAX_TURN_PLIES`] is set at 3.6× it rather than at 4× the research max.
//!
//! **Per-phase root branching** (mean candidates · µs/decision, `B = 4`):
//! AdditionalActionFirst 1.27 · 459 — Placement **148.65 · 8,144** —
//! Movement 14.38 · 164 — AreaAction 3.46 · 239 — FivePowerBonus 7.17 · 458 —
//! Building 1.48 · 37 — AdditionalActionSecond 1.80 · 46 — Recovery (a deferred
//! `RecoveryLeave` choice) 2.21 · 66.
//!
//! **Read that table before optimizing anything.** Placement is 13% of
//! decisions and **84% of the wall clock**: the phase offers ~149 root
//! candidates and each one is charged a whole turn completion, which is the
//! rung's definition rather than a defect. The 1,268.6 µs sits ~20% above spec
//! §4.4's corrected ~1.0–1.1 ms budget (the two later replicates, 1,125.1 and
//! 1,134.8 µs, sit INSIDE it — another reading the spread governs), and the shape
//! AGREES with R3's model rather than diverging from it: R3 measured 315.01
//! handles/decision at `B = 4` and 56.53 at `B = 1`, and the three replicates
//! here bracket both on the same definitions.
//!
//! **Sizing for T11, with the SELF-PLAY/MIXED distinction spelled out because
//! it is a factor of two and §10.2's table turns on it.** These games are
//! Lookahead-vs-Lookahead, so BOTH seats search: 308 decisions/game × 1.269 ms
//! = **~391 ms/game**. Every §10.2 strength cell except a Lookahead null pits
//! ONE Lookahead seat against a cheap opponent, and a seat takes **154 own
//! decisions/game** here, so a mixed cell costs **~196 ms/game** of search plus
//! the opponent's — which lands on lens 2's 209 ms/game (200.2 own decisions ×
//! 1.0378 ms) rather than contradicting it. §10.2's ~84 s promotion-cell
//! estimate therefore stands; what would NOT is a Lookahead-vs-Lookahead cell
//! at doctrine N, which is ~2× that and is exactly why §10.2's null self-test
//! is Greedy-vs-Greedy.
//!
//! **Debug costs ~15× release** — the band `crate::game::greedy` records for
//! itself (12–14×), one notch wider. ONE measured set, taken together at the T5
//! fix wave (2026-07-27, Apple M5, quiet machine), because two doc sites used to
//! carry two different numbers for the same thing:
//!
//! | | debug | release |
//! |---|---|---|
//! | the driven whole-game pin ALONE (310 moves) | **5.49 s** (5.58 s wall) | — |
//! | all 21 tests in this module, `--test-threads=1` | **11.04 s** | **0.78 s** |
//! | all 21, default parallelism | — | 0.48 s |
//!
//! The driven pin plays its 310-move 2P game through the arena driver — per-move
//! invariants ON, plus that driver's `is_move_legal` boundary and its two
//! debug-tier asserts — so 5.49 s is ~17.7 ms per decision against release's
//! 1.13 ms, a ratio of 15.6×. §10.1's pre-declared debug-cost hazard is real and
//! this is its size: HALF this module's debug time is that one pin.
//!
//! **The 4P cap-bound placement node, measured** (the T3-review note: a
//! cap-bound node costs ~512 scorings, and at `B = 4` the beam multiplies it).
//! On the shipped `action_cost` 4P Newspaper fixture — 3,231 realized
//! placements, so the cap binds and the root is exactly
//! `PLACEMENT_CANDIDATE_CAP` moves — ONE decision at `B = 4` costs **11.16 ms**
//! (512 roots, 4,862 nodes, 12,385 successors scored, 1 cap-bind) against
//! **6.81 ms** at `B = 1` (1,597 nodes, 7,591 scored). So the beam multiplies
//! that node by **1.64×**, not by 4: the 512-candidate ROOT ply is paid once
//! whatever `B` is, and only the four survivors' continuations widen. It is
//! **8.8× a mean 2P decision**. A fixture, not an organic rate — T4 measured the
//! cap binding on 9.19% of organic 4P decisions, and 3–4P strength cells are OUT
//! of 3.3 (spec §1).
//!
//! # What this rung does and does NOT claim about its streams
//!
//! **What it claims, and this is stronger than it was before the T5 review
//! (M2).** A decision spends EXACTLY THREE words of the agent's own play stream,
//! in this order: the sampler seed, the tie key, the candidate seed. Three, flat
//! — independent of `K` (one sampler seed serves all K worlds; `BeliefSampler`
//! draws its determinizations off its own stream), independent of the weight
//! table, and independent of everything the search does inside, because all three
//! words are drawn BEFORE the first world is sampled. So the agent's stream
//! POSITION after `n` decisions is `3n`, and that is pinned two ways: by
//! reconstruction (`the_agents_stream_consumption_per_decision_is_reconstructible`)
//! and by addressing decision `n` at mirror words `3n+1..3n+3` at both `K = 1`
//! and `K = 2` (`the_agents_play_stream_advances_exactly_three_words_per_decision`).
//! Before the fix the count was `2 +` however many draws the search happened to
//! make, so it was neither constant nor weight-independent.
//!
//! **What it still does NOT claim.** [`crate::game::greedy`]'s "two weight tables
//! walk the same stream" property does not survive into search in the form that
//! matters for candidate SETS: which nodes exist depends on which nodes the beam
//! kept, which depends on the weights, so two weight tables reach different nodes
//! and therefore draw different candidates there. The keyed candidate seed makes
//! the DICE common across worlds within one decision; it cannot make the node set
//! common across weight tables, and nothing here says it does.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::game::apply_move::apply_move_unchecked;
use crate::game::arena::{ArenaAgent, ControllerProvenance, DecisionReport};
use crate::game::belief::BeliefSampler;
use crate::game::eval::{self, Explanation, WeightTable, EVAL_IMPL_VERSION};
use crate::game::greedy::{
    candidates, observed_view, score_starting_order, uniform_index, weight_table_identity,
    Candidates, DEFAULT_RETREAT_SAMPLES,
};
use crate::game::metrics::{decision_volume, VolumeObs};
use crate::game::rules::Rules;
use crate::game::search_actions::DecisionHandle;
use crate::game::seeds::splitmix64;
use crate::types::{
    GameState, Move, OrderCardId, PlayerId, PlayerView, PresidentLocation, TurnPhase,
};

// ---------------------------------------------------------------------------
// Identity and knobs
// ---------------------------------------------------------------------------

/// The S1 CONTROLLER's implementation version (spec §0.5 / L12), stamped into
/// `SeatProvenance::version` for every seat Lookahead-v1 plays.
///
/// THE BUMP RULE, in the words `SeatProvenance::version` already means: bump on
/// ANY behavior-affecting change to THIS controller — the search shape (hand-off
/// definition, beam semantics, tie-site keying), the candidate scheme it
/// inherits, the `B`/`K`/`M` defaults, or the evaluator weights it embeds —
/// because each of those makes previously-recorded Lookahead-v1 figures
/// non-comparable. A weight change bumps [`EVAL_IMPL_VERSION`] AND every
/// controller const that embeds the evaluator (`GREEDY_IMPL_VERSION` too),
/// honestly: comparability is the criterion, not authorship. Per spec §0.5 each
/// controller NAME carries its own const, which is why the diagnostic arm below
/// has [`ATOMIC_DIAG_IMPL_VERSION`] rather than riding this one.
pub const LOOKAHEAD_IMPL_VERSION: u32 = 1;

/// The stable S1 controller name — provenance, runner grammar and corpus
/// filters read it from here rather than hand-typing it.
pub const LOOKAHEAD_CONTROLLER: &str = "lookahead-v1";

/// The S2 DIAGNOSTIC arm's own version const (spec §0.5's per-controller rule).
///
/// It is a separate const because it is a separate controller NAME, and the two
/// can move independently: a change to the quiescence rule makes S2's figures
/// non-comparable without touching S1 at all — which is exactly what happened
/// at **2 (T5 review S2)**: the evaluate-only leaf now returns before a node is
/// built, and a depth-spent FORCED phase boundary is valued where it used to
/// extend free (see [`Run::atomic`]). S1 did not move, so
/// [`LOOKAHEAD_IMPL_VERSION`] did not either.
pub const ATOMIC_DIAG_IMPL_VERSION: u32 = 2;

/// The S2 DIAGNOSTIC arm's controller name.
///
/// **`-diag` is in the NAME on purpose.** Spec §4.1 ships both schemes behind
/// the contestant surface and preserves only the winner as a rung; until T7
/// says otherwise this arm exists to be MEASURED against S1, and a corpus
/// reader filtering on `controller` must be able to see that without consulting
/// a document.
pub const ATOMIC_DIAG_CONTROLLER: &str = "lookahead-atomic-diag-v1";

/// S1's default beam width `B` (spec §4.2). `B = 1` is a greedy continuation.
pub const DEFAULT_BEAM: u16 = 4;

/// `K`: sampled worlds per decision. **3.3 ships 1** (spec §4.3) — the knob
/// exists so T8's divergence instrument and any future revisit have something
/// to turn, not because K > 1 is expected to pay.
pub const DEFAULT_WORLDS: u8 = 1;

/// S2's default atomic depth `d`, in MOVES (spec §4.1).
pub const DEFAULT_ATOMIC_DEPTH: u8 = 2;

/// Totality guard on the S1 continuation: how many beam plies one turn may take
/// before the search stops extending and values the frontier where it stands.
///
/// **Not a semantic bound — a guard.** The measured player turn is mean 7.66 /
/// **max observed 18** moves (module doc), and this sits 3.6× above that
/// maximum; the scratch harness recorded **0** hits in 5,890 decisions across
/// both beam widths.
///
/// A hit is not a wrong answer but it IS a quietly different one — the line is
/// valued where the guard stopped it, not where the seat handed off, so the
/// explanation and the principal continuation are taken mid-turn. That is why
/// [`SearchStats::ply_cap_hits`] is aggregated onto
/// [`SearchTally::ply_cap_hits`] and asserted at 0 by the driven whole-game pin
/// (T5 review S1): until that handle existed the counter was written and never
/// read, which made a cap regression exactly as silent as no counter at all.
pub const MAX_TURN_PLIES: u32 = 64;

/// Totality guard on S2's quiescence: how many FREE (depth-uncharged) plies one
/// line may extend through before it is valued where it stands.
///
/// Free extension is what quiescence is for (spec §4.1: "forced chains extend
/// free"), and a chain of pending choices can in principle be long; this bounds
/// it. **The guard is PER LINE** — `free_used` is threaded down one recursion
/// path, not summed across branches — so it bounds free DEPTH and not free
/// WIDTH. Measured over 2,376 S2 decisions: 3.6 free steps per decision summed
/// over all branches, with one decision reaching 5,368 (and 31,271 nodes).
///
/// **That worst case is recorded rather than designed against, and the T5 review
/// found a much larger instance of it: 464,686 free steps and 471,632 nodes in a
/// SINGLE decision, 2.58 s of release wall clock** (10-game S2 self-play,
/// post-reorder — module doc's tail table). Nothing about the guard changed; free
/// WIDTH is what grows, and this is 86× the previously recorded instance rather
/// than a new failure mode. It stays recorded because S2 is a diagnostic arm and
/// T7's cell will meet the tail, whereas inventing a total-work budget here would
/// be inventing a search semantic the spec does not price. What it does mean is
/// that T7 must be provisioned off the TAIL, not off a mean.
///
/// **Metered from the six-lens fix wave forward** ([`SearchStats::free_cap_hits`]
/// → [`SearchTally::free_cap_hits`]), on [`MAX_TURN_PLIES`]'s own grounds. Until
/// then this guard had no counter at all while the S1 cap had one, and
/// `free_extension_steps` cannot substitute: it counts steps TAKEN summed over
/// every branch (hence a recorded 5,368 in one decision against a per-line bound
/// of 64), so a line stopped at the guard is invisible in it. The counter was
/// added AFTER the depth-decision experiment ran, so no figure published before
/// this commit measured this cap — the artifact says so where it reports caps.
pub const MAX_FREE_EXTENSION: u32 = 64;

/// Domain salt for the keyed tie stream, in this crate's `mix` shape.
const SALT_TIE_STREAM: u64 = 0x1EA4_7123_0000_0001;

// ---------------------------------------------------------------------------
// The keyed tie stream (spec §3.2 at every tie point; §4.3's K-common keying)
// ---------------------------------------------------------------------------

/// WHERE a tie was broken, as a key T8 can hold common across sampled worlds.
///
/// The coordinates are deliberately WORLD-INDEPENDENT: a root candidate's
/// INDEX, a continuation depth, a slot within a tied block, and (for S2) the
/// index PATH that reached the site. Two worlds that offer the same candidate at
/// the same index break its ties the same way (spec §4.3; lens 2 measured 7.66%
/// false divergence from unkeyed draws).
///
/// That makes the TIE dice world-common. It is not on its own enough to make
/// T8's residual divergence determinization — the candidate dice have to be
/// world-common too, which is [`candidate_rng`]'s job and was an open channel
/// until the T5 review (M2). Cite the pair, never this half alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TieSite {
    /// The root argmax over hand-off-evaluated root candidates.
    Root,
    /// The beam's truncation cut inside root candidate `root`, at continuation
    /// `depth`, filling tied slot `slot`.
    Beam { root: u32, depth: u32, slot: u32 },
    /// S2's quiescence extension step inside root candidate `root` at `depth`,
    /// on the line whose index path folds to `branch`.
    ///
    /// `branch` is [`descend_branch`]'s coordinate and it is load-bearing:
    /// without it two SIBLING lines extending at the same depth draw the same
    /// index (T5 review S3).
    Extension { root: u32, depth: u32, branch: u32 },
}

impl TieSite {
    /// `(tag, a, b, c)` — the tag keeps the three site families disjoint even
    /// where their coordinates coincide.
    fn coords(self) -> [u64; 4] {
        match self {
            Self::Root => [0, 0, 0, 0],
            Self::Beam { root, depth, slot } => {
                [1, u64::from(root), u64::from(depth), u64::from(slot)]
            }
            Self::Extension { root, depth, branch } => {
                [2, u64::from(root), u64::from(depth), u64::from(branch)]
            }
        }
    }
}

/// One decision's tie randomness: ONE word from the agent's play stream, folded
/// per site.
///
/// Two properties, both load-bearing and both pinned:
/// - **constant consumption** — the search spends ZERO play-stream words on
///   ties however many it breaks, so the agent's stream position after a
///   decision is a function of `(K, the candidate scheme)` and not of the
///   search's internal shape;
/// - **world-common draws** — the key is drawn BEFORE the world loop, so the
///   `k`-th world's site-`s` draw equals the first world's (spec §4.3).
#[derive(Debug, Clone, Copy)]
pub struct TieStream {
    key: u64,
}

impl TieStream {
    /// Key this decision's ties. Callers draw `key` from the agent's own play
    /// stream, once, before any world is sampled.
    pub fn new(key: u64) -> Self {
        Self { key }
    }

    /// The (deterministic) word for `site`. Folded with [`splitmix64`], this
    /// crate's one mixing primitive, in `arena::mix`'s shape.
    pub fn word(&self, site: TieSite) -> u64 {
        let mut acc = self.key ^ SALT_TIE_STREAM;
        for component in site.coords() {
            let mut step = acc ^ component;
            acc = splitmix64(&mut step);
        }
        acc
    }

    /// A uniform index into `0..len` for `site` — [`uniform_index`], the same
    /// primitive [`crate::game::greedy`] breaks its ties with. `len` must be nonzero.
    pub fn pick(&self, site: TieSite, len: usize) -> usize {
        uniform_index(self.word(site), len)
    }
}

/// The candidate-scheme dice ONE world's search gets, keyed by this decision's
/// candidate-stream seed.
///
/// **World-COMMON by construction, and that is the point** (spec §4.3; T5 review
/// M2). The candidate scheme DRAWS — `M` retreat samples at every Movement node,
/// a reservoir pass at every Placement node — so handing the agent's CONTINUING
/// play stream to each of the K per-world searches would start world `k` at a
/// shifted stream position and two worlds would then draw different candidates
/// even where their states are IDENTICAL. One word drawn once per decision and
/// re-keyed per world makes the dice common; the candidate LISTS still differ
/// wherever the worlds' states differ, which is exactly the divergence T8's
/// instrument is trying to measure.
///
/// This is [`TieStream`]'s construction in the other half of the same argument,
/// and the two together are why the module doc can say cross-world divergence is
/// determinization-driven and nothing else.
fn candidate_rng(candidate_seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(candidate_seed)
}

/// Extend a branch coordinate by the candidate INDEX taken at this node.
///
/// S2's extension tie site has to tell SIBLINGS apart: two branches that reach a
/// free-extension step at the same `(root, free_used)` would otherwise key the
/// same word and pick the same index — perfectly correlated, which is the
/// first-wins residue one step sideways rather than one level down (T5 review
/// S3). S1's [`TieSite::Beam`] needs no equivalent: its `slot` already separates
/// the picks within one cut, and a cut happens once per `(root, depth)`.
///
/// The fold is a function of the INDEX PATH and nothing else, so the coordinate
/// stays world-independent in exactly the sense §4.3 needs: two worlds offering
/// the same candidate at the same index down the same path key the same word.
///
/// **Known limitation, recorded rather than fixed (six-lens review, accepted as
/// a doc note):** the fold is 32 bits, so two DIFFERENT index paths can collide
/// on one `branch` value. A collision is scoped by the rest of the key —
/// [`TieSite::Extension`] is `(root, free_used, branch)`, so both lines must
/// also share a root candidate and a free-extension depth — and its effect is
/// that two unrelated lines draw a CORRELATED tie word. Each site's marginal
/// pick stays uniform. It is deliberately NOT the defect this function was
/// added for, which was every sibling at one `(root, depth)` sharing one word.
/// Widening the coordinate would move S2's tie draws and hence the published
/// depth-decision figures, so it is handed to 3.4 — where S2 is raw material
/// and an [`ATOMIC_DIAG_IMPL_VERSION`] bump is already in scope.
fn descend_branch(branch: u32, index: usize) -> u32 {
    // FNV-1a's 32-bit prime. Wrapping because a coordinate is a key, not a count.
    branch.wrapping_mul(0x0100_0193).wrapping_add(index as u32).wrapping_add(1)
}

// ---------------------------------------------------------------------------
// Scheme, stats, outcome
// ---------------------------------------------------------------------------

/// Which search shape a contestant runs. Spec §4.1 ships BOTH; T7 adjudicates
/// which one is preserved as the rung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScheme {
    /// **S1, THE RUNG** (spec §4.2): complete the root seat's own turn under a
    /// beam of width `beam`, evaluate at the hand-off.
    TurnBoundary { beam: u16 },
    /// **S2, DIAGNOSTIC** (spec §4.1): atomic depth-`depth` in MOVES, with
    /// quiescence — forced chains extend FREE (no depth charge) and evaluation
    /// happens only at quiet points. It exists for the T7 experiment and is
    /// **not a preserved rung unless T7 flips**.
    AtomicDiag { depth: u8 },
}

impl SearchScheme {
    /// This scheme's controller name (spec §0.5: one version const per name).
    pub fn controller(self) -> &'static str {
        match self {
            Self::TurnBoundary { .. } => LOOKAHEAD_CONTROLLER,
            Self::AtomicDiag { .. } => ATOMIC_DIAG_CONTROLLER,
        }
    }

    /// This scheme's controller version const.
    pub fn version(self) -> u32 {
        match self {
            Self::TurnBoundary { .. } => LOOKAHEAD_IMPL_VERSION,
            Self::AtomicDiag { .. } => ATOMIC_DIAG_IMPL_VERSION,
        }
    }

    /// This scheme with its sizing knob RAISED TO 1 — the single site that
    /// answers a zero knob, so the floor is a stated fact about the knob rather
    /// than an inline `.max(1)` at whichever use site happened to need it.
    ///
    /// Called by [`LookaheadAgent::with_knobs`] (so an agent's
    /// [`scheme`](LookaheadAgent::scheme) reports the search it will actually
    /// run) AND by [`search_world`], which is public and reachable without an
    /// agent: a beam of 0 truncates every frontier away, and `Run::truncate`
    /// would index `children[beam - 1]`.
    pub fn normalized(self) -> Self {
        match self {
            Self::TurnBoundary { beam } => Self::TurnBoundary { beam: beam.max(1) },
            Self::AtomicDiag { depth } => Self::AtomicDiag { depth: depth.max(1) },
        }
    }
}

impl Default for SearchScheme {
    fn default() -> Self {
        Self::TurnBoundary { beam: DEFAULT_BEAM }
    }
}

/// THE provenance of a Lookahead seat at these knobs — name, version and knob
/// map in one place, for whichever scheme it runs.
///
/// A free function for [`crate::game::greedy::controller_provenance`]'s reason: the arena's
/// [`Contestant`](crate::game::arena::Contestant) must be able to say what a
/// controller calls itself without building an agent (a build needs `&Rules` for
/// the Keep table). Both callers route here, so the two controller names and
/// their two version consts are read at exactly one site (spec §5.2 / §0.5) —
/// and the SCHEME picks which pair, which is what makes S1 and the S2 diagnostic
/// arm separately versionable.
pub fn controller_provenance(
    scheme: SearchScheme,
    worlds: u8,
    retreat_samples: u8,
    weights: &WeightTable,
    agent_seed: Option<u64>,
) -> ControllerProvenance {
    let mut params = BTreeMap::from([
        // `worlds.max(1)` mirrors `with_knobs` exactly: a decision over zero
        // sampled worlds is not a decision, so the agent raises it — and a
        // descriptor that printed the raw 0 would name knobs the seat did not
        // play on.
        ("k".to_string(), i64::from(worlds.max(1))),
        ("m".to_string(), i64::from(retreat_samples)),
        ("eval_impl".to_string(), i64::from(EVAL_IMPL_VERSION)),
        ("weights".to_string(), weight_table_identity(weights)),
    ]);
    // `normalized()` for the same reason `worlds.max(1)` is applied above, now
    // that the sizing knob has one floor: a descriptor printing a raw `b=0` or
    // `d=0` would name a search no seat ran.
    match scheme.normalized() {
        SearchScheme::TurnBoundary { beam } => {
            params.insert("b".to_string(), i64::from(beam));
        }
        SearchScheme::AtomicDiag { depth } => {
            params.insert("d".to_string(), i64::from(depth));
        }
    }
    ControllerProvenance {
        name: scheme.controller().to_string(),
        version: scheme.version(),
        params,
        agent_seed,
    }
}

/// What one search cost and what shape it had — the counters the §4.4 pin, the
/// cost curve and T7's cost comparison all read.
///
/// Every field is a COUNT, deterministic in `(world, weights, knobs, stream
/// position)`; nothing here is a timing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchStats {
    /// Root candidates this decision offered (summed over the K worlds).
    /// The per-phase branching figure, and the denominator every other count
    /// here scales with.
    pub root_candidates: u64,
    /// Expandable nodes created. Each built exactly one [`DecisionHandle`].
    pub nodes: u64,
    /// `DecisionHandle`s built. **Equal to [`nodes`](Self::nodes) by
    /// construction** — the duplicate-handle hazard is precisely the two coming
    /// apart, which is why both are counted and pinned rather than one being
    /// inferred from the other.
    pub handles: u64,
    /// Nodes that FORCED the routes memo and therefore paid one President-route
    /// DFS: a Movement decision point, no pending choice, President on board.
    /// The right-hand side of the §4.4 counter pin.
    pub route_dfs_nodes: u64,
    /// Nodes offering ≤ 1 candidate — the forced fast path (R3: ~60%).
    pub forced_nodes: u64,
    /// Hand-off leaves evaluated (never expanded, never handled).
    pub leaves: u64,
    /// Successors applied + observed + scored, leaves included. The cost driver.
    pub scored: u64,
    /// Nodes whose placement family exceeded the candidate cap.
    pub placement_cap_binds: u64,
    /// The longest root→hand-off line this search completed, in moves.
    pub max_line: u32,
    /// Times [`MAX_TURN_PLIES`] stopped a continuation. Expected 0.
    pub ply_cap_hits: u64,
    /// S2 quiescence steps taken free of depth charge.
    pub free_extension_steps: u64,
    /// Times [`MAX_FREE_EXTENSION`] stopped a free extension. Expected 0.
    ///
    /// A DIFFERENT event from [`free_extension_steps`](Self::free_extension_steps),
    /// which counts steps TAKEN: that sum is decision-wide and a 64-deep line is
    /// invisible in it, which is exactly why the guard needed a counter of its
    /// own rather than a threshold read off the sum.
    pub free_cap_hits: u64,
}

impl SearchStats {
    /// Accumulate `other` into `self` — how the `K` per-world searches sum
    /// (the `× K` in §4.4's counter formula).
    pub fn merge(&mut self, other: &Self) {
        self.root_candidates += other.root_candidates;
        self.nodes += other.nodes;
        self.handles += other.handles;
        self.route_dfs_nodes += other.route_dfs_nodes;
        self.forced_nodes += other.forced_nodes;
        self.leaves += other.leaves;
        self.scored += other.scored;
        self.placement_cap_binds += other.placement_cap_binds;
        self.max_line = self.max_line.max(other.max_line);
        self.ply_cap_hits += other.ply_cap_hits;
        self.free_extension_steps += other.free_extension_steps;
        self.free_cap_hits += other.free_cap_hits;
    }
}

/// One root candidate, valued.
#[derive(Debug, Clone)]
pub struct RootValue {
    /// The candidate move itself.
    pub mv: Move,
    /// The value the scheme assigned it — for S1, the best hand-off its
    /// continuation reached.
    pub value: i64,
    /// The [`Trace`] index of the state that value was READ AT. Ask
    /// [`WorldSearch::line`] for the move sequence to it.
    ///
    /// An index rather than a materialized `Vec<Move>` because only the CHOSEN
    /// candidate's line is ever rendered, and materializing all ~27 of them
    /// would cost more `Move` clones than the whole trace does.
    pub leaf: u32,
}

/// One root candidate's value AFTER the K worlds are folded together — what
/// [`LookaheadAgent::last_roots`] reports.
///
/// A separate type from [`RootValue`] on purpose: that one is a per-WORLD
/// result and carries a `leaf` index into that world's own [`Trace`], which is
/// meaningless once K worlds have been aggregated. This one carries only what
/// survives the fold — the move and its K-mean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootScore {
    /// The candidate move.
    pub mv: Move,
    /// Its K-mean value (integer division, rounding toward zero — at the
    /// shipped `K = 1` the searched value itself). The agent's pick is one of
    /// the maximisers of this number, so a caller can read the MARGIN of any
    /// other candidate as `value − max`.
    pub value: i64,
}

/// One world's search: every root candidate valued, plus the decision-point
/// volume observation and the cost counters.
#[derive(Debug, Clone)]
pub struct WorldSearch {
    pub roots: Vec<RootValue>,
    /// L10's bundling: measured at the ROOT handle, the same one the root
    /// candidates came off, so the driver never needs a second enumeration.
    pub volume: VolumeObs,
    pub stats: SearchStats,
    trace: Trace,
}

impl WorldSearch {
    /// The move sequence from the root to `leaf`, root move first.
    pub fn line(&self, leaf: u32) -> Vec<Move> {
        self.trace.line(leaf)
    }
}

// ---------------------------------------------------------------------------
// The trace: principal continuations without per-node lines
// ---------------------------------------------------------------------------

/// Marks a trace entry with no parent — a root candidate.
const NO_PARENT: u32 = u32::MAX;

/// Append-only `(parent, move)` back-links for one decision's search.
///
/// A node that owned its whole line would clone a `Vec<Move>` (and every `Move`
/// in it — several carry `Vec`s of their own) down every branch of the beam.
/// This clones ONE move per created node and reconstructs a line on demand.
#[derive(Debug, Clone, Default)]
struct Trace {
    entries: Vec<(u32, Move)>,
}

impl Trace {
    fn push(&mut self, parent: u32, mv: Move) -> u32 {
        self.entries.push((parent, mv));
        (self.entries.len() - 1) as u32
    }

    /// Walk back to the root and reverse. `leaf` must be an index this trace
    /// issued.
    fn line(&self, leaf: u32) -> Vec<Move> {
        let mut out = Vec::new();
        let mut at = leaf;
        while at != NO_PARENT {
            let (parent, mv) = &self.entries[at as usize];
            out.push(mv.clone());
            at = *parent;
        }
        out.reverse();
        out
    }

    /// How many moves the line to `leaf` holds — [`line`](Self::line)'s length
    /// without building it.
    fn depth_of(&self, leaf: u32) -> u32 {
        let mut n = 0;
        let mut at = leaf;
        while at != NO_PARENT {
            n += 1;
            at = self.entries[at as usize].0;
        }
        n
    }
}

// ---------------------------------------------------------------------------
// The search
// ---------------------------------------------------------------------------

/// Everything one per-world search reads that is not the world itself.
///
/// Bundled rather than passed loose because the search's internals thread all
/// of it through every level, and because `ties` MUST be the same stream at
/// every site of one decision.
#[derive(Debug, Clone, Copy)]
pub struct SearchCtx<'a> {
    pub rules: &'a Rules,
    pub weights: &'a WeightTable,
    pub scheme: SearchScheme,
    /// `M`, spec §3.3's retreat sample count — inherited from [`crate::game::greedy`]'s
    /// candidate scheme, which this rung reuses verbatim.
    pub retreat_samples: u8,
    pub ties: TieStream,
}

/// Whether a node at `state` will pay one President-route DFS.
///
/// **Derived from the STATE, not from the handle** — deliberately, and it is
/// what makes the §4.4 counter pin discriminating rather than tautological. If
/// this were counted where handles are built, a duplicate handle would inflate
/// BOTH sides of `route DFS count == on-board-Movement nodes × K` and the pin
/// would pass through the exact defect it exists to catch (T5 verified that by
/// mutation, both ways round). Counted per NODE against an independently
/// derived expectation, a second handle moves only the instrument.
///
/// The predicate mirrors `search_actions::families`' dispatch exactly: a
/// pending choice short-circuits every phase to `ChoiceResolution`, Movement is
/// the only phase that emits `MovementRoutes`, and the routes family only
/// SEARCHES with the President on board (its in-hand arm emits `PlacePresident`
/// and walks nothing).
fn pays_route_dfs(state: &GameState) -> bool {
    state.pending_choices.is_empty()
        && state.turn_phase == TurnPhase::Movement
        && matches!(state.current_player().president_location, PresidentLocation::OnBoard(_))
}

/// One expandable search node — spec §4.4's shape, `(GameState, Vec<Move>,
/// i64)`, plus a `u32` back-link into the [`Trace`] (an index, not a fourth
/// heap structure).
struct Node {
    state: GameState,
    /// The cache: computed exactly once, at creation, off this node's one
    /// handle. A forced node's single candidate is read from here for free.
    candidates: Vec<Move>,
    /// [`eval::score`] at this node, from the ROOT seat's view. The beam orders
    /// on it; a hand-off leaf's copy of the same number is its value.
    eval: i64,
    trace: u32,
}

/// A SCORED CHILD that is not (yet) a node: applied, observed and evaluated,
/// but with no handle and no candidates.
///
/// **The distinction is where S1's cost lives.** Ranking the beam needs an
/// EVALUATION; only the survivors of the cut need an ENUMERATION. Promoting
/// every child would build a handle for every one of them: on one 186-candidate
/// Placement decision, promote-then-cut costs **11,614** handles against
/// cut-then-promote's **3,122** on byte-identical play — the same move chosen,
/// and the same 12,610 scorings either way (re-measured at the T5 review fix
/// wave; the authoring run read 11,604 / 3,122 / 12,600 before the M2 candidate
/// seed moved the dice). It is also the shape R3's cost model was measured on —
/// 315.01 handles/decision at `B = 4`, which the three replicates in the module
/// doc bracket — and that model is where spec §4.4's ~1.0–1.1 ms budget comes
/// from. `Node` still owns its candidates and still builds exactly one handle;
/// node CREATION just happens after the cut rather than before it.
///
/// This claim is what `one_pick_allocates_within_a_measured_sanity_bound`'s
/// `nodes × 2 < scored` assertion makes executable — and the T5 review's finding
/// was that until that assertion existed, NOTHING did (the allocation ceiling
/// passes under promote-then-cut).
struct Successor {
    state: GameState,
    eval: i64,
    trace: u32,
}

/// What one node's single [`DecisionHandle`] yielded.
struct Expansion {
    candidates: Vec<Move>,
    /// Only asked for at the root (L10: one enumeration serves both).
    volume: Option<VolumeObs>,
}

/// The mutable half of one per-world search.
struct Run<'a, R: Rng> {
    ctx: &'a SearchCtx<'a>,
    seat: PlayerId,
    rng: &'a mut R,
    trace: Trace,
    stats: SearchStats,
}

impl<R: Rng> Run<'_, R> {
    /// Whether `state` has handed off — the root seat is no longer to move, or
    /// the game is over.
    ///
    /// The second clause is not redundant: at game end `advance_phase` sets
    /// `TurnPhase::TurnEnd` and leaves `current_player` alone (`apply_move.rs`),
    /// so a finished game reached on the root seat's own turn would otherwise
    /// look like a continuation with no legal moves.
    fn is_hand_off(&self, state: &GameState) -> bool {
        state.current_player != self.seat || state.is_game_over()
    }

    /// [`eval::score`] at `state`, from the root seat's view.
    fn value_at(&self, state: &GameState) -> Result<i64, String> {
        let view = observed_view(state, self.seat, self.ctx.scheme.controller())?;
        Ok(eval::score(&view, self.ctx.rules, self.ctx.weights))
    }

    /// Build the ONE [`DecisionHandle`] this decision point ever gets and read
    /// everything off it in a single pass.
    ///
    /// **The module's only `DecisionHandle::new` call site.** Volume (when
    /// asked) is taken FIRST, then the candidates, so both read one memo — the
    /// L10 bundling, and the reason a Movement node pays exactly one route DFS.
    fn expand_point(
        &mut self,
        state: &GameState,
        want_volume: bool,
    ) -> Result<Expansion, String> {
        let handle = DecisionHandle::new(state, self.ctx.rules);
        self.stats.handles += 1;
        if handle.families().is_empty() {
            return Err(format!(
                "{}: seat {} was asked to move at {:?}, which offers no families at all",
                self.ctx.scheme.controller(),
                self.seat.0,
                handle.turn_phase(),
            ));
        }
        let volume = if want_volume { decision_volume(&handle) } else { None };
        let Candidates { moves, placement_cap_bound } =
            candidates(&handle, self.ctx.retreat_samples, self.rng);
        if placement_cap_bound {
            self.stats.placement_cap_binds += 1;
        }
        if moves.is_empty() {
            return Err(format!(
                "dead end: {} found no candidate at {:?} for seat {}",
                self.ctx.scheme.controller(),
                handle.turn_phase(),
                self.seat.0,
            ));
        }
        Ok(Expansion { candidates: moves, volume })
    }

    /// Create an expandable node at `state` — its one handle, its cached
    /// candidates, its evaluation.
    fn make_node(&mut self, state: GameState, trace: u32) -> Result<Node, String> {
        let eval = self.value_at(&state)?;
        self.promote(Successor { state, eval, trace })
    }

    /// Promote an already-scored [`Successor`] to a node: this is where — and
    /// the only place where — a search state pays for a handle.
    fn promote(&mut self, child: Successor) -> Result<Node, String> {
        let Expansion { candidates, .. } = self.expand_point(&child.state, false)?;
        self.stats.nodes += 1;
        if pays_route_dfs(&child.state) {
            self.stats.route_dfs_nodes += 1;
        }
        if candidates.len() <= 1 {
            self.stats.forced_nodes += 1;
        }
        Ok(Node { state: child.state, candidates, eval: child.eval, trace: child.trace })
    }

    /// Record a hand-off leaf: counted, never expanded, never handled.
    fn note_leaf(&mut self, trace: u32) {
        self.stats.leaves += 1;
        self.stats.max_line = self.stats.max_line.max(self.trace.depth_of(trace));
    }

    /// Apply `mv` at `state`, hard-erroring the way spec §3.1 requires (the
    /// candidate came off this state's own families, so a rejection is a bug).
    fn apply(&self, state: &GameState, mv: &Move) -> Result<GameState, String> {
        apply_move_unchecked(state, mv, self.ctx.rules).map_err(|e| {
            format!("{}: candidate {mv:?} failed to apply: {e}", self.ctx.scheme.controller())
        })
    }

    // -- S1: turn-boundary with a beam ------------------------------------

    /// Complete the root seat's turn from `start` under a beam of `beam`, and
    /// return `(best hand-off value, its trace index)`.
    ///
    /// `root` is the root candidate's INDEX, and it enters every tie site so
    /// that two worlds break the same candidate's ties identically (§4.3).
    fn complete_turn(
        &mut self,
        root: u32,
        start: GameState,
        start_trace: u32,
        beam: usize,
    ) -> Result<(i64, u32), String> {
        // The root candidate's own successor may already BE the hand-off — a
        // turn's last move usually is.
        if self.is_hand_off(&start) {
            let v = self.value_at(&start)?;
            self.note_leaf(start_trace);
            return Ok((v, start_trace));
        }

        let mut best: Option<(i64, u32)> = None;
        let mut frontier = vec![self.make_node(start, start_trace)?];
        let mut depth: u32 = 0;

        while !frontier.is_empty() {
            if depth >= MAX_TURN_PLIES {
                // Totality guard, never expected (module doc: 0 hits in the
                // measured runs). Value the frontier where it stands rather
                // than returning nothing.
                self.stats.ply_cap_hits += 1;
                for node in &frontier {
                    if best.is_none_or(|(v, _)| node.eval > v) {
                        best = Some((node.eval, node.trace));
                    }
                }
                break;
            }

            let mut children: Vec<Successor> = Vec::new();
            for node in std::mem::take(&mut frontier) {
                for mv in &node.candidates {
                    let next = self.apply(&node.state, mv)?;
                    let trace = self.trace.push(node.trace, mv.clone());
                    self.stats.scored += 1;
                    let value = self.value_at(&next)?;
                    if self.is_hand_off(&next) {
                        // A hand-off is a LEAF: valued, never expanded, so it
                        // can never be truncated away by the beam either.
                        self.note_leaf(trace);
                        // First-wins among EQUAL hand-off values, deliberately:
                        // this is not a decision tie point. The root
                        // candidate's value is the max either way, so nothing
                        // played changes — only which equally-valued line is
                        // REPORTED as the principal continuation.
                        if best.is_none_or(|(b, _)| value > b) {
                            best = Some((value, trace));
                        }
                    } else {
                        children.push(Successor { state: next, eval: value, trace });
                    }
                }
            }

            // Cut FIRST, promote after: only survivors pay for a handle.
            self.truncate(&mut children, beam, root, depth);
            for child in children {
                let node = self.promote(child)?;
                frontier.push(node);
            }
            depth += 1;
        }

        best.ok_or_else(|| {
            format!(
                "{}: seat {}'s turn never handed off within {MAX_TURN_PLIES} plies",
                self.ctx.scheme.controller(),
                self.seat.0,
            )
        })
    }

    /// Keep the `beam` best children, breaking the CUT's tie by uniform draw.
    ///
    /// The research critic's unfixed residue was exactly this: a stable sort
    /// keeps the enumeration's first `B` of a tied tail, which reintroduces
    /// first-wins one level below the argmax the spec fixed. Strictly-better
    /// children are kept outright; the tied block straddling the cut is
    /// partially Fisher-Yates'd with one keyed word per slot.
    fn truncate(&mut self, children: &mut Vec<Successor>, beam: usize, root: u32, depth: u32) {
        if children.len() <= beam {
            return; // no cut, no draw
        }
        children.sort_by(|a, b| b.eval.cmp(&a.eval));
        let cut = children[beam - 1].eval;
        let lo = children.partition_point(|n| n.eval > cut);
        let hi = children.partition_point(|n| n.eval >= cut);
        let need = beam - lo;
        for slot in 0..need {
            let remaining = hi - lo - slot;
            if remaining <= 1 {
                break;
            }
            let pick = slot
                + self.ctx.ties.pick(
                    TieSite::Beam { root, depth, slot: slot as u32 },
                    remaining,
                );
            children.swap(lo + slot, lo + pick);
        }
        children.truncate(beam);
    }

    // -- S2: atomic depth-d with quiescence (DIAGNOSTIC) -------------------

    /// Spec §4.1's atomic arm. Returns `(value, trace index of the state it was
    /// read at)`.
    ///
    /// Depth is charged in MOVES and only at genuine branch points. A point is
    /// FREE — extends without a depth charge — when it offers ≤ 1 candidate or
    /// when a pending choice heads the queue; those are the "forced chains" of
    /// §4.1, and pricing them would spend the whole budget resolving bookkeeping
    /// the player never chose. Evaluation happens at QUIET points: a hand-off,
    /// or a phase boundary reached with the budget spent. A budget spent
    /// mid-phase extends greedily (one line, no branching, tie-broken by keyed
    /// draw) until the phase turns over — bounded by [`MAX_FREE_EXTENSION`].
    ///
    /// # The evaluate-only leaf is decided BEFORE a handle is built
    ///
    /// **T5 review S2, and it is a behaviour change with a version bump behind
    /// it, not a pure reorder — read this before comparing S2 figures across the
    /// bump.** The arm used to build the node FIRST and test quietness second,
    /// so every line that ended at a spent-budget phase boundary paid a full
    /// enumeration it never read. S1 has no analogue (a hand-off leaf is valued
    /// and never expanded), and §4.1 adjudicates T7 on DETERMINISTIC COUNTS over
    /// a no-edge interval, so a cost asymmetry that exists only in the
    /// bookkeeping could decide the experiment.
    ///
    /// The test could not simply move: `free` is `candidates.len() <= 1 ||
    /// pending choices`, and its first disjunct IS the enumeration, so "quiet
    /// AND non-free" is not answerable before `make_node`. What moved is the
    /// answerable part — depth spent, an EMPTY choice queue, and the phase
    /// turned over (or the free guard tripped) — all three readable off the
    /// state, with [`eval::score`] needing no handle. The one behavioural
    /// consequence, stated exactly: a point that is depth-spent, queue-empty, at
    /// a phase boundary AND FORCED (≤ 1 candidate) is now VALUED where it used
    /// to extend free through its forced move. A pending-choice chain still
    /// extends free — a queued choice is bookkeeping and reads its freeness off
    /// the queue, not off an enumeration — and the mid-phase greedy extension
    /// below is untouched. [`ATOMIC_DIAG_IMPL_VERSION`] is bumped accordingly.
    // Eight: `state`/`trace` are the point, and `root`/`branch`/`depth_left`/
    // `prev_phase`/`free_used` are five INDEPENDENT descent coordinates threaded
    // down one recursion path. Bundling them into a struct would be a rewrite of
    // a hot recursive function whose costs were just measured, for no gain in
    // what any of them means.
    #[allow(clippy::too_many_arguments)]
    fn atomic(
        &mut self,
        state: GameState,
        trace: u32,
        root: u32,
        branch: u32,
        depth_left: u8,
        prev_phase: TurnPhase,
        free_used: u32,
    ) -> Result<(i64, u32), String> {
        if self.is_hand_off(&state) {
            let v = self.value_at(&state)?;
            self.note_leaf(trace);
            return Ok((v, trace));
        }

        // EVALUATE-ONLY, decided off the STATE with no handle built (see above).
        if depth_left == 0
            && state.pending_choices.is_empty()
            && (state.turn_phase != prev_phase || free_used >= MAX_FREE_EXTENSION)
        {
            // A cap HIT is the guard being the reason the line stopped, which is
            // not the same event as this arm firing: a line that reached a real
            // phase boundary is quiet by design however much free extension
            // preceded it. Inside this arm an unturned phase implies the guard
            // tripped, by the disjunction above.
            if state.turn_phase == prev_phase {
                self.stats.free_cap_hits += 1;
            }
            return Ok((self.value_at(&state)?, trace));
        }

        let node = self.make_node(state, trace)?;
        let free = node.candidates.len() <= 1 || !node.state.pending_choices.is_empty();

        if depth_left == 0 && !free {
            // `!free` implies an empty queue, and the boundary case returned
            // above, so what is left here is exactly "the budget ran out
            // MID-PHASE". Extend ONE greedy step for free, no branching, so the
            // evaluation lands on a phase boundary rather than inside a
            // half-made decision.
            debug_assert!(
                node.state.turn_phase == prev_phase && free_used < MAX_FREE_EXTENSION,
                "the quiet-boundary case must have returned before the node was built",
            );
            self.stats.free_extension_steps += 1;
            let mut best: Option<(i64, usize)> = None;
            let mut tied: Vec<usize> = Vec::new();
            let mut evals: Vec<(i64, GameState, u32)> = Vec::with_capacity(node.candidates.len());
            for mv in &node.candidates {
                let next = self.apply(&node.state, mv)?;
                let t = self.trace.push(node.trace, mv.clone());
                self.stats.scored += 1;
                let v = self.value_at(&next)?;
                let at = evals.len();
                evals.push((v, next, t));
                match best {
                    Some((b, _)) if v < b => {}
                    Some((b, _)) if v == b => tied.push(at),
                    _ => {
                        best = Some((v, at));
                        tied.clear();
                        tied.push(at);
                    }
                }
            }
            let site = TieSite::Extension { root, depth: free_used, branch };
            let chosen = tied[self.ctx.ties.pick(site, tied.len())];
            let (_, next, t) = evals.swap_remove(chosen);
            let phase = node.state.turn_phase;
            return self.atomic(
                next,
                t,
                root,
                descend_branch(branch, chosen),
                0,
                phase,
                free_used + 1,
            );
        }

        let charge = u8::from(!free);
        let free_used = free_used + u32::from(free);
        if free && free_used >= MAX_FREE_EXTENSION {
            self.stats.free_cap_hits += 1;
            return Ok((node.eval, node.trace));
        }

        let phase = node.state.turn_phase;
        let mut best: Option<(i64, u32)> = None;
        for (i, mv) in node.candidates.iter().enumerate() {
            let next = self.apply(&node.state, mv)?;
            let t = self.trace.push(node.trace, mv.clone());
            self.stats.scored += 1;
            let (v, at) = self.atomic(
                next,
                t,
                root,
                descend_branch(branch, i),
                depth_left - charge,
                phase,
                free_used,
            )?;
            // First-wins among equals: a VALUE aggregation, not a decision tie
            // point (see `complete_turn`'s note).
            if best.is_none_or(|(b, _)| v > b) {
                best = Some((v, at));
            }
        }
        best.ok_or_else(|| {
            format!("{}: an expanded node produced no child", self.ctx.scheme.controller())
        })
    }
}

/// Search ONE world: value every root candidate under `ctx.scheme`.
///
/// `seat` must be the world's player to move — the agent is only ever asked at
/// its own decision point, and a search rooted anywhere else would be
/// completing somebody else's turn.
pub fn search_world<R: Rng>(
    world: &GameState,
    seat: PlayerId,
    ctx: &SearchCtx<'_>,
    rng: &mut R,
) -> Result<WorldSearch, String> {
    if world.current_player != seat {
        return Err(format!(
            "{}: asked to search seat {}'s move in a world where seat {} is to play",
            ctx.scheme.controller(),
            seat.0,
            world.current_player.0,
        ));
    }
    let mut run = Run { ctx, seat, rng, trace: Trace::default(), stats: SearchStats::default() };

    // The ROOT node's one handle: volume and candidates off the same memo.
    let root = run.expand_point(world, true)?;
    let volume = root
        .volume
        .ok_or_else(|| format!("{}: the root offered no volume", ctx.scheme.controller()))?;
    run.stats.nodes += 1;
    run.stats.root_candidates = root.candidates.len() as u64;
    if pays_route_dfs(world) {
        run.stats.route_dfs_nodes += 1;
    }
    if root.candidates.len() <= 1 {
        run.stats.forced_nodes += 1;
    }

    let mut roots = Vec::with_capacity(root.candidates.len());
    for (i, mv) in root.candidates.iter().enumerate() {
        let next = run.apply(world, mv)?;
        let trace = run.trace.push(NO_PARENT, mv.clone());
        run.stats.scored += 1;
        // `normalized` is the knob floor (its doc says why it is applied here
        // too); nothing below re-clamps, so the search a caller reads off the
        // scheme is the search that runs.
        let (value, leaf) = match ctx.scheme.normalized() {
            SearchScheme::TurnBoundary { beam } => {
                run.complete_turn(i as u32, next, trace, usize::from(beam))?
            }
            SearchScheme::AtomicDiag { depth } => {
                let phase = world.turn_phase;
                // The branch coordinate starts at 0 for every root: the root
                // INDEX is already a coordinate of its own, so two roots never
                // share a site whatever their sub-branches fold to.
                run.atomic(next, trace, i as u32, 0, depth.saturating_sub(1), phase, 0)?
            }
        };
        roots.push(RootValue { mv: mv.clone(), value, leaf });
    }

    Ok(WorldSearch { roots, volume, stats: run.stats, trace: run.trace })
}

// ---------------------------------------------------------------------------
// The agent
// ---------------------------------------------------------------------------

/// The per-decision counters one Lookahead seat keeps, behind one `Arc`.
#[derive(Debug, Default)]
struct LookaheadCounters {
    explained: AtomicU64,
    nodes: AtomicU64,
    scored: AtomicU64,
    placement_cap_binds: AtomicU64,
    continuation_moves: AtomicU64,
    ply_cap_hits: AtomicU64,
    free_cap_hits: AtomicU64,
}

/// A cloneable handle on one [`LookaheadAgent`]'s counters — [`crate::game::greedy`]'s
/// `ExplanationTally` pattern (itself the `HonestPathTally` clone-out
/// precedent), and for the same reason: a game's agents are built by the
/// driver's factory and never handed back, so cloning the handle BEFORE the
/// agent moves into the driver is the only way to read them afterwards.
///
/// A separate TYPE from `greedy::ExplanationTally` rather than a shared one:
/// the counter SETS differ (a search has nodes and continuation lengths; a
/// one-ply argmax has neither), and a shared type would have to carry fields
/// one of its two owners never writes.
#[derive(Clone, Default)]
pub struct SearchTally {
    counters: Arc<LookaheadCounters>,
}

impl SearchTally {
    /// Decisions this agent explained — every decision it made, since
    /// [`LookaheadAgent::choose`] cannot return without explaining (spec
    /// §0.3(b)).
    pub fn explained(&self) -> u64 {
        self.counters.explained.load(Ordering::Relaxed)
    }

    /// Search nodes created across every decision — the cost figure that makes
    /// a µs/decision number interpretable.
    pub fn nodes(&self) -> u64 {
        self.counters.nodes.load(Ordering::Relaxed)
    }

    /// Successors applied, observed and scored across every decision.
    pub fn candidates_scored(&self) -> u64 {
        self.counters.scored.load(Ordering::Relaxed)
    }

    /// Decisions whose placement family exceeded the candidate cap, summed over
    /// nodes (spec §3.3's 4P arm — a search can bind it several times in one
    /// decision, which the one-ply rung cannot).
    pub fn placement_cap_binds(&self) -> u64 {
        self.counters.placement_cap_binds.load(Ordering::Relaxed)
    }

    /// Σ of the chosen principal continuations' lengths, in moves. Divided by
    /// [`explained`](Self::explained) it is the mean hand-off length THIS agent
    /// searched to.
    pub fn continuation_moves(&self) -> u64 {
        self.counters.continuation_moves.load(Ordering::Relaxed)
    }

    /// Continuations [`MAX_TURN_PLIES`] stopped, summed over every decision.
    ///
    /// **Expected 0, and readable so that a run can SAY so** (T5 review S1). A
    /// cap hit is not a wrong answer, but it is a quietly different one: the
    /// line it values ends where the GUARD stopped it and not where the seat
    /// handed off, so the explanation and the principal continuation are taken
    /// mid-turn (see [`LookaheadAgent::last_continuation`]). The counter existed
    /// on [`SearchStats`] from the start; until this handle it was written and
    /// never read, which made a silent cap regression exactly as silent as
    /// having no counter at all.
    pub fn ply_cap_hits(&self) -> u64 {
        self.counters.ply_cap_hits.load(Ordering::Relaxed)
    }

    /// Free extensions [`MAX_FREE_EXTENSION`] stopped, summed over every
    /// decision — S2's counterpart to [`ply_cap_hits`](Self::ply_cap_hits), and
    /// **0 by construction for an S1 agent**, which never enters the quiescence
    /// arm at all.
    ///
    /// Expected 0 on both schemes, and readable for the reason the ply counter
    /// is: a stopped free extension is valued where the guard stopped it rather
    /// than at a quiet point, and `free_extension_steps` cannot report that —
    /// it is a decision-wide sum over every branch, in which one 64-deep line
    /// is indistinguishable from sixty-four one-step ones.
    pub fn free_cap_hits(&self) -> u64 {
        self.counters.free_cap_hits.load(Ordering::Relaxed)
    }
}

/// Lookahead-v1: spec §4.2's turn-boundary rung (and, under
/// [`SearchScheme::AtomicDiag`], §4.1's diagnostic arm).
pub struct LookaheadAgent {
    scheme: SearchScheme,
    worlds: u8,
    retreat_samples: u8,
    weights: WeightTable,
    rng: ChaCha8Rng,
    /// Kept beside the stream it seeded because `ChaCha8Rng` will not hand its
    /// seed back and provenance has to record it (`PolicyAgent`'s reason).
    agent_seed: u64,
    /// Every Order card's Keep score, precomputed at construction —
    /// `greedy::score_starting_order`, SHARED rather than re-invented, which is
    /// also what makes a Lookahead-vs-Greedy cell Keep-HOMOGENEOUS and hence
    /// setup-identical under spec §5.5 (the T7 experiment's cell is
    /// Lookahead-vs-Lookahead, so it is homogeneous either way).
    keep_scores: Vec<(OrderCardId, i64)>,
    counters: SearchTally,
    last_explanation: Option<Explanation>,
    last_continuation: Vec<Move>,
    last_roots: Vec<RootScore>,
    last_stats: Option<SearchStats>,
}

impl LookaheadAgent {
    /// Lookahead-v1 at the shipped knobs: S1 at `B` = [`DEFAULT_BEAM`],
    /// `K` = [`DEFAULT_WORLDS`], `M` = [`DEFAULT_RETREAT_SAMPLES`], and the
    /// frozen [`WeightTable::V1`]. `agent_seed` is the PLAY stream — key it with
    /// [`agent_stream_seed`](crate::game::arena::agent_stream_seed).
    pub fn new(rules: &Rules, agent_seed: u64) -> Self {
        Self::with_knobs(
            rules,
            SearchScheme::default(),
            DEFAULT_WORLDS,
            DEFAULT_RETREAT_SAMPLES,
            WeightTable::V1,
            agent_seed,
        )
    }

    /// The diagnostic S2 arm at depth `d` (spec §4.1) — same evaluator, same
    /// candidates, same tie discipline.
    pub fn atomic_diag(rules: &Rules, depth: u8, agent_seed: u64) -> Self {
        Self::with_knobs(
            rules,
            SearchScheme::AtomicDiag { depth },
            DEFAULT_WORLDS,
            DEFAULT_RETREAT_SAMPLES,
            WeightTable::V1,
            agent_seed,
        )
    }

    /// Every knob explicit, and all three of the sizing knobs are RAISED TO 1
    /// here rather than defended at their use sites — one normalization point,
    /// so [`scheme`](Self::scheme) reports the search that will actually run.
    ///
    /// `worlds` (`K`) below 1 is raised to 1: a decision taken over zero sampled
    /// worlds is not a decision. `beam` below 1 is raised to 1: a beam of zero
    /// truncates every frontier away, so the search would complete no turn.
    /// `depth` below 1 is raised to 1: `atomic` charges `depth − 1` at the root,
    /// so 0 and 1 already searched identically and the clamp makes that
    /// equality a stated fact instead of an accident of `saturating_sub`. The
    /// runner refuses `b=0`/`d=0` as user errors before it ever gets here
    /// (`examples/arena.rs`); this is the library-side floor under callers that
    /// compute a knob rather than type one.
    pub fn with_knobs(
        rules: &Rules,
        scheme: SearchScheme,
        worlds: u8,
        retreat_samples: u8,
        weights: WeightTable,
        agent_seed: u64,
    ) -> Self {
        let scheme = scheme.normalized();
        let keep_scores =
            rules.orders.iter().map(|card| (card.id, score_starting_order(card, rules))).collect();
        Self {
            scheme,
            worlds: worlds.max(1),
            retreat_samples,
            weights,
            rng: ChaCha8Rng::seed_from_u64(agent_seed),
            agent_seed,
            keep_scores,
            counters: SearchTally::default(),
            last_explanation: None,
            last_continuation: Vec::new(),
            last_roots: Vec::new(),
            last_stats: None,
        }
    }

    /// The search shape this agent runs.
    pub fn scheme(&self) -> SearchScheme {
        self.scheme
    }

    /// The weights this agent evaluates with.
    pub fn weights(&self) -> &WeightTable {
        &self.weights
    }

    /// A cloneable handle on the counters, readable after the agent has been
    /// moved into the driver.
    pub fn tally(&self) -> SearchTally {
        self.counters.clone()
    }

    /// The explanation of the LAST move this agent chose, taken at the
    /// **hand-off state the search actually valued it at** — not at its
    /// one-ply successor, which is a different state and a different number
    /// (spec §0.3(b); T10 renders it). A ply-capped line is the one case where
    /// "hand-off" means the cap point instead — see
    /// [`last_continuation`](Self::last_continuation).
    pub fn last_explanation(&self) -> Option<&Explanation> {
        self.last_explanation.as_ref()
    }

    /// The PRINCIPAL CONTINUATION of the last decision: the move sequence from
    /// the chosen root candidate to the hand-off its value was read at, root
    /// move first. Stored for T10's rendering — an explanation of a searched
    /// move is incomplete without the line it was searched down.
    ///
    /// **One honest caveat, and [`SearchTally::ply_cap_hits`] is how a run says
    /// whether it applies** (T5 review S1): if [`MAX_TURN_PLIES`] stopped the
    /// continuation, the line ends at the CAP and not at a seat transition, so
    /// both this sequence and [`last_explanation`](Self::last_explanation) were
    /// taken MID-TURN. Nothing marks that in the line itself — a truncated line
    /// and a completed one are the same `Vec<Move>` — which is why the counter
    /// is read rather than merely written, and why the driven whole-game pin
    /// asserts it stayed at 0.
    pub fn last_continuation(&self) -> &[Move] {
        &self.last_continuation
    }

    /// EVERY root candidate of the last decision with its folded value, in the
    /// order the candidate scheme offered them (never re-sorted here — a caller
    /// that wants a ranking owns that presentation choice).
    ///
    /// Stored for the same reason [`last_continuation`](Self::last_continuation)
    /// is: T10's `suggest` has to render the MARGINS between the chosen move and
    /// its runner-ups, and spec §0.3 is explicit that a surface renders the
    /// search's own output rather than re-deriving it — a second, independently
    /// keyed search in the CLI could rank a different move first and there would
    /// be no fact of the matter about which answer the bot gave.
    ///
    /// It costs one `Vec` per decision and no `Move` clones: the aggregate the
    /// argmax already built is MOVED here rather than copied, so nothing on
    /// §4.4's budget grows by more than an allocation the decision was going to
    /// make anyway.
    ///
    /// The chosen move is always among these at the maximum value; which
    /// maximiser it is comes from the §3.2 keyed tie draw, so the runner-up list
    /// alone does not determine the pick.
    pub fn last_roots(&self) -> &[RootScore] {
        &self.last_roots
    }

    /// The last decision's cost counters.
    pub fn last_stats(&self) -> Option<&SearchStats> {
        self.last_stats.as_ref()
    }
}

/// One root MOVE's value aggregated across the K worlds it appeared in.
struct Aggregate {
    mv: Move,
    sum: i64,
    count: i64,
    /// The line, and the world index it is a line THROUGH. Kept from the first
    /// world that offered the move — at the shipped K = 1 that is the only one.
    line: Vec<Move>,
    world: usize,
}

impl ArenaAgent for LookaheadAgent {
    /// Argmax of `greedy::score_starting_order` over the two dealt cards, ties
    /// to index 0 — the SAME Keep the one-ply rung uses, for the reason
    /// `keep_scores` documents.
    ///
    /// No stream draw (spec §5.5): a unit's two seatings must resolve the same
    /// deal identically, which holds exactly when a seat's Keep is a
    /// deterministic function of `(seat, deal)` and not of which contestant sits
    /// there.
    fn choose_starting_order(&mut self, _seat: PlayerId, deal: &[OrderCardId; 2]) -> u8 {
        let score =
            |id: OrderCardId| self.keep_scores.iter().find(|(k, _)| *k == id).map_or(0, |(_, v)| *v);
        u8::from(score(deal[1]) > score(deal[0]))
    }

    fn choose(&mut self, view: &PlayerView, rules: &Rules) -> Result<DecisionReport, String> {
        let seat = view.viewer;
        // The sampler seed comes from THIS AGENT'S OWN stream and nothing else
        // — deriving it from anything state-flavoured would correlate the sample
        // with the truth it must be blind to.
        let sampler_seed: u64 = self.rng.gen();
        let mut sampler = BeliefSampler::from_view(view, rules, sampler_seed)
            .map_err(|e| format!("belief sampler refused seat {}'s view: {e}", seat.0))?;

        // ONE tie key for the whole decision, drawn BEFORE the world loop so
        // every world breaks the same site's tie the same way (spec §4.3).
        let ctx = SearchCtx {
            rules,
            weights: &self.weights,
            scheme: self.scheme,
            retreat_samples: self.retreat_samples,
            ties: TieStream::new(self.rng.next_u64()),
        };

        // ONE candidate-stream seed, drawn in the same place and for the same
        // reason as the tie key: every world's search is handed a FRESH
        // `candidate_rng` from this word, so the candidate DICE are world-common
        // too (spec §4.3; T5 review M2). Drawing it here — once, whatever K is —
        // is also what fixes the agent's per-decision play-stream consumption at
        // exactly three words.
        let candidate_seed = self.rng.next_u64();

        let k = usize::from(self.worlds);
        let mut agg: Vec<Aggregate> = Vec::new();
        let mut worlds: Vec<GameState> = Vec::with_capacity(k);
        let mut volume: Option<VolumeObs> = None;
        let mut stats = SearchStats::default();

        // THE K LOOP (spec §4.3): each world gets its OWN whole search and the
        // results meet at the ROOT. No per-world trees, no interleaving — a
        // node's continuation is a property of the world it lives in.
        for w in 0..k {
            let world = sampler.sample();
            let searched = search_world(&world, seat, &ctx, &mut candidate_rng(candidate_seed))?;
            // The volume is world 0's root handle's: an observation about THIS
            // decision point's size, and at the shipped K = 1 the only one.
            if volume.is_none() {
                volume = Some(searched.volume);
            }
            stats.merge(&searched.stats);
            for root in &searched.roots {
                match agg.iter_mut().find(|a| a.mv == root.mv) {
                    Some(a) => {
                        a.sum = a.sum.saturating_add(root.value);
                        a.count += 1;
                    }
                    None => agg.push(Aggregate {
                        mv: root.mv.clone(),
                        sum: root.value,
                        count: 1,
                        line: searched.line(root.leaf),
                        world: w,
                    }),
                }
            }
            worlds.push(world);
        }

        let volume = volume.ok_or_else(|| {
            format!("{}: no world produced a volume observation", self.scheme.controller())
        })?;
        if agg.is_empty() {
            return Err(format!(
                "dead end: {} found no candidate for seat {}",
                self.scheme.controller(),
                seat.0,
            ));
        }

        // Argmax over the K-mean (integer division, rounding toward zero — at
        // the shipped K = 1 it is the searched value itself), then spec §3.2's
        // uniform draw over the FULL tied set from the KEYED word.
        let mut best = i64::MIN;
        let mut tied: Vec<usize> = Vec::new();
        for (i, a) in agg.iter().enumerate() {
            let mean = a.sum / a.count;
            if mean > best {
                best = mean;
                tied.clear();
                tied.push(i);
            } else if mean == best {
                tied.push(i);
            }
        }
        // Read out of the aggregate what the rest of this method needs, then
        // MOVE the aggregate into `last_roots` below — the borrow ends here so
        // the runner-up list costs an allocation rather than a `Move` clone per
        // candidate.
        let chosen_index = tied[ctx.ties.pick(TieSite::Root, tied.len())];
        let (mv, chosen_line, chosen_world) = {
            let chosen = &agg[chosen_index];
            (chosen.mv.clone(), chosen.line.clone(), chosen.world)
        };

        // Spec §0.3(b): the chosen move is explained ALWAYS, and the
        // explanation is taken at the state the search VALUED — replay the
        // principal continuation in the world it was searched in. Replaying
        // (≤ ~17 moves) is cheaper than carrying a `PlayerView` per root
        // candidate through the search, and it cross-checks the value for free.
        let mut at = worlds[chosen_world].clone();
        for step in &chosen_line {
            at = apply_move_unchecked(&at, step, rules).map_err(|e| {
                format!(
                    "{}: principal continuation {step:?} failed to replay: {e}",
                    self.scheme.controller(),
                )
            })?;
        }
        let explanation = eval::explain(
            &observed_view(&at, seat, self.scheme.controller())?,
            rules,
            &self.weights,
        );
        debug_assert!(
            k > 1 || explanation.total == best,
            "explain/score disagreed at the hand-off the search valued ({mv:?}): {} vs {best}",
            explanation.total,
        );

        self.last_continuation = chosen_line;
        self.last_explanation = Some(explanation);
        self.last_roots =
            agg.into_iter().map(|a| RootScore { mv: a.mv, value: a.sum / a.count }).collect();
        self.last_stats = Some(stats);
        let counters = &self.counters.counters;
        counters.explained.fetch_add(1, Ordering::Relaxed);
        counters.nodes.fetch_add(stats.nodes, Ordering::Relaxed);
        counters.scored.fetch_add(stats.scored, Ordering::Relaxed);
        counters.placement_cap_binds.fetch_add(stats.placement_cap_binds, Ordering::Relaxed);
        counters.ply_cap_hits.fetch_add(stats.ply_cap_hits, Ordering::Relaxed);
        counters.free_cap_hits.fetch_add(stats.free_cap_hits, Ordering::Relaxed);
        counters
            .continuation_moves
            .fetch_add(self.last_continuation.len() as u64, Ordering::Relaxed);

        Ok(DecisionReport { mv, volume })
    }

    /// Delegated to [`controller_provenance`] so the agent and the arena's
    /// `Contestant` cannot drift about what a Lookahead seat is called.
    fn controller(&self) -> ControllerProvenance {
        controller_provenance(
            self.scheme,
            self.worlds,
            self.retreat_samples,
            &self.weights,
            Some(self.agent_seed),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::greedy::GreedyAgent;
    use crate::game::observe::observe;
    use crate::game::route_enum::{route_dfs_count, route_dfs_reset};
    use crate::game::search_actions::alloc_probe;
    use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
    use crate::types::GameConfig;

    /// The committed diagnostic master for this file's driven fixtures.
    /// Diagnostic BY NAME (spec §3's gate-cell exemption): a per-edit pin needs
    /// a fixed master, and these are pins rather than bot-facing evaluation
    /// runs.
    const MASTER: u64 = 0x1EA4_2026_0727_0001;

    fn ctx<'a>(
        rules: &'a Rules,
        weights: &'a WeightTable,
        scheme: SearchScheme,
        key: u64,
    ) -> SearchCtx<'a> {
        SearchCtx {
            rules,
            weights,
            scheme,
            retreat_samples: DEFAULT_RETREAT_SAMPLES,
            ties: TieStream::new(key),
        }
    }

    /// Play a fresh 2P game under Greedy-v1 until `want` holds, then hand back a
    /// SAMPLED world at that decision point plus the seat to move — the same
    /// shape the agent itself searches in.
    fn point_where(
        rules: &Rules,
        seed: u64,
        want: impl Fn(&GameState) -> bool,
    ) -> (GameState, PlayerId) {
        let pending = new_game(GameConfig::for_player_count(2), rules, seed).expect("2P setup");
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
                .expect("finalize");
        let mut agent = GreedyAgent::new(rules, 0x77);
        for _ in 0..400 {
            if want(&state) {
                break;
            }
            let seat = state.current_player;
            let view = observe(&state, seat).expect("observe");
            let r = agent.choose(&view, rules).expect("move");
            state = apply_move_unchecked(&state, &r.mv, rules).expect("apply");
        }
        assert!(want(&state), "the walk never reached the wanted decision point");
        let seat = state.current_player;
        let view = observe(&state, seat).expect("observe");
        let world = BeliefSampler::from_view(&view, rules, 1).expect("sampler").sample();
        (world, seat)
    }

    fn placement_point(rules: &Rules) -> (GameState, PlayerId) {
        point_where(rules, MASTER, |s| {
            s.turn_phase == TurnPhase::Placement && s.pending_choices.is_empty()
        })
    }

    /// A Movement point with the President ON BOARD — the only shape that pays
    /// a route DFS, and therefore the only shape the §4.4 counter pin can be
    /// non-vacuous at.
    fn on_board_movement_point(rules: &Rules) -> (GameState, PlayerId) {
        point_where(rules, MASTER, |s| {
            s.turn_phase == TurnPhase::Movement
                && s.pending_choices.is_empty()
                && matches!(s.current_player().president_location, PresidentLocation::OnBoard(_))
        })
    }

    /// A PLACEMENT point whose President is already on board, so the Movement
    /// node that pays the route DFS sits INSIDE the continuation rather than at
    /// the root. The president-location clause is what makes it non-vacuous: at
    /// an early Placement point the President is in hand, and the continuation's
    /// Movement node then emits `PlacePresident` and walks nothing.
    fn on_board_placement_point(rules: &Rules) -> (GameState, PlayerId) {
        point_where(rules, MASTER, |s| {
            s.turn_phase == TurnPhase::Placement
                && s.pending_choices.is_empty()
                && matches!(s.current_player().president_location, PresidentLocation::OnBoard(_))
        })
    }

    /// The first `n` decisions `agent` makes driving BOTH seats of a fresh 2P
    /// game at `world_seed`, as debug strings (`greedy`'s `first_decisions`,
    /// same shape and the same reason: one agent for both seats is the sharpest
    /// version of "an agent's stream determines its answers").
    fn first_decisions(
        agent: &mut LookaheadAgent,
        world_seed: u64,
        n: usize,
        rules: &Rules,
    ) -> Vec<String> {
        let pending =
            new_game(GameConfig::for_player_count(2), rules, world_seed).expect("2P setup");
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
                .expect("finalize");
        let mut out = Vec::new();
        while out.len() < n {
            let seat = state.current_player;
            let view = observe(&state, seat).expect("observe");
            let report = agent.choose(&view, rules).expect("the honest chain produces a move");
            out.push(format!("{:?}", report.mv));
            state = apply_move_unchecked(&state, &report.mv, rules).expect("a chosen move applies");
        }
        out
    }

    // -- S1: the hand-off is the horizon (spec §4.2) --------------------------

    /// **THE RUNG'S DEFINING PROPERTY.** A root candidate is valued at the
    /// hand-off — the end of the root seat's own turn — not at its own
    /// successor. The observable difference from [`GreedyAgent`] is that a
    /// root candidate's line CONTINUES past its own move.
    #[test]
    fn s1_values_root_candidates_at_the_hand_off() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let mut rng = ChaCha8Rng::seed_from_u64(5);
        let c = ctx(&rules, &WeightTable::V1, SearchScheme::default(), 0xB1A5);
        let searched = search_world(&world, seat, &c, &mut rng).expect("search");

        assert!(!searched.roots.is_empty(), "a decision point must offer root candidates");
        assert!(
            searched.roots.iter().any(|r| searched.line(r.leaf).len() > 1),
            "at least one root candidate's turn must continue past its own move",
        );
        // Every line ENDS at a hand-off: replay it and check the seat changed
        // (or the game finished). This is the property, not a proxy for it.
        for r in &searched.roots {
            let line = searched.line(r.leaf);
            let mut at = world.clone();
            for mv in &line {
                at = apply_move_unchecked(&at, mv, &rules).expect("the line replays");
            }
            assert!(
                at.current_player != seat || at.is_game_over(),
                "a searched line must end at a hand-off, ended at {:?} with seat {} still to play",
                at.turn_phase,
                seat.0,
            );
            // And the value is the evaluation OF that state, from this seat.
            let view = observe(&at, seat).expect("observe");
            assert_eq!(
                r.value,
                eval::score(&view, &rules, &WeightTable::V1),
                "the root's value must be its hand-off's score",
            );
        }
    }

    /// A finished game is a hand-off even though `current_player` never moved:
    /// `advance_phase` sets `TurnPhase::TurnEnd` and leaves the seat alone, so
    /// the `is_game_over` clause is load-bearing rather than belt-and-braces.
    #[test]
    fn a_finished_game_counts_as_a_hand_off() {
        let rules = Rules::standard();
        let (world, _) = placement_point(&rules);
        // `is_game_over` fires for the seat BEFORE the start player — the last
        // seat of the final round, whose `current_player` `advance_phase`
        // deliberately does not move on.
        let last = PlayerId::new(
            (world.config.start_player.0 + world.config.player_count - 1)
                % world.config.player_count,
        );
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let c = ctx(&rules, &WeightTable::V1, SearchScheme::default(), 1);
        let run = Run {
            ctx: &c,
            seat: last,
            rng: &mut rng,
            trace: Trace::default(),
            stats: SearchStats::default(),
        };

        let mut live = world.clone();
        live.current_player = last;
        assert!(!live.is_game_over(), "the live fixture must not be over");
        assert!(!run.is_hand_off(&live), "the seat's own decision point is not a hand-off");

        let mut ended = live.clone();
        ended.turn_phase = TurnPhase::TurnEnd;
        ended.end_game_triggered = true;
        ended.final_round = true;
        ended.trigger_player = Some(PlayerId::new(0));
        assert!(ended.is_game_over(), "the fixture must actually be over");
        assert_eq!(ended.current_player, last, "and must NOT have changed seat");
        assert!(
            run.is_hand_off(&ended),
            "a finished game is a hand-off even with the root seat still current",
        );
    }

    /// `B = 1` is a greedy continuation and `B = 4` is not: the wider beam
    /// keeps more nodes, so it costs more and can see a line the narrow one
    /// pruned.
    #[test]
    fn the_beam_width_knob_widens_the_frontier() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let run_at = |beam: u16| -> SearchStats {
            let mut rng = ChaCha8Rng::seed_from_u64(5);
            let c = ctx(&rules, &WeightTable::V1, SearchScheme::TurnBoundary { beam }, 0xB1A5);
            search_world(&world, seat, &c, &mut rng).expect("search").stats
        };
        let narrow = run_at(1);
        let wide = run_at(4);
        assert!(
            wide.nodes > narrow.nodes,
            "B=4 must expand more than B=1 ({} vs {})",
            wide.nodes,
            narrow.nodes,
        );
        assert!(narrow.leaves > 0 && wide.leaves > 0, "both must reach hand-offs");
    }

    // -- the node/cache shape (spec §4.4) -------------------------------------

    /// **THE DUPLICATE-HANDLE PIN, both halves.**
    ///
    /// STRUCTURAL: the non-test half of this file constructs a
    /// `DecisionHandle` at exactly ONE site. (The count matches on the `(` so
    /// prose mentions of the name do not enter — write them without a trailing
    /// paren, as this module's docs do.)
    ///
    /// RUNTIME: over a real search, handles built == nodes created. The two are
    /// counted at different places on purpose; a second `expand_point` for one
    /// node — R3's S2 defect at pending-choice depth-0 states, where forcedness
    /// was discovered with one handle and the moves enumerated with another —
    /// moves `handles` and not `nodes`.
    ///
    /// **MUTATION-PROVEN AT AUTHORING** (2026-07-27, debug) by writing exactly
    /// that defect into `promote` — `let _forced = expand_point(..)?.candidates
    /// .len() <= 1;` ahead of the real call — and reverting by inverse edit:
    ///
    /// ```text
    /// assertion `left == right` failed: one handle per node, no more
    ///   left: 6243
    ///  right: 3122
    /// ```
    #[test]
    fn the_module_builds_exactly_one_decision_handle_per_node() {
        let src = include_str!("lookahead.rs");
        let body = src.split("#[cfg(test)]").next().expect("a non-test half");
        assert_eq!(
            body.matches("DecisionHandle::new(").count(),
            1,
            "the search must have exactly one handle construction site",
        );

        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let mut rng = ChaCha8Rng::seed_from_u64(5);
        let c = ctx(&rules, &WeightTable::V1, SearchScheme::default(), 0xB1A5);
        let s = search_world(&world, seat, &c, &mut rng).expect("search").stats;
        assert!(s.nodes > 1, "anti-vacuity: the search must have expanded ({} nodes)", s.nodes);
        assert_eq!(s.handles, s.nodes, "one handle per node, no more");
        assert!(s.forced_nodes > 0, "anti-vacuity: some node must have been forced");
        assert!(s.leaves > 0, "anti-vacuity: the search must reach hand-offs");
        assert!(
            s.scored > s.nodes,
            "anti-vacuity: children are SCORED without being promoted ({} scored, {} nodes) — \
             if these were equal, every child would be paying for a handle",
            s.scored,
            s.nodes,
        );
    }

    /// **THE §4.4 COUNTER PIN, scoped exactly as the spec scopes it.**
    ///
    /// `route DFS count == cache-miss on-board-Movement NODES × K`, and NOT
    /// "nodes expanded": every non-Movement node and every IN-HAND Movement
    /// node forces no routes memo and pays 0 (the in-hand arm of
    /// `movement_routes_family` emits `PlacePresident` and walks nothing).
    ///
    /// HARNESS-SCOPED, never armed globally: this measures one `search_world`
    /// call with nothing else on the thread. The arena driver legitimately pays
    /// more in debug (its `is_move_legal` boundary plus a volume cross-check),
    /// which is why `route_enum`'s doc forbids arming the counter against it.
    ///
    /// The `× K` is the sum over the K per-world searches — `SearchStats::merge`
    /// accumulates it — so the K = 2 arm below compares against the sum rather
    /// than doubling the K = 1 figure.
    ///
    /// **MUTATION-PROVEN AT AUTHORING, and the first attempt did NOT
    /// discriminate — which is why the pin has the shape it has** (2026-07-27,
    /// debug). Writing R3's duplicate handle into `promote` and running this
    /// cell against a MOVEMENT root alone passed: a turn holds exactly one
    /// Movement phase, so at a Movement root the only DFS-paying node is the
    /// root itself, which `search_world` expands directly and the mutation never
    /// touches. Two repairs, both kept: `route_dfs_nodes` moved off the handle
    /// site to a STATE-derived predicate at the node sites (`pays_route_dfs` —
    /// counted at the handle it would have doubled with the instrument and
    /// proved nothing), and the second cell below roots at a PLACEMENT point
    /// with the President already on board so the paying node sits inside the
    /// continuation. The mutation then failed as it should:
    ///
    /// ```text
    /// placement root, DFS inside the continuation: one President-route DFS per
    /// cache-miss on-board-Movement node, at K=1
    ///   left: 372
    ///  right: 186
    /// ```
    #[test]
    fn the_route_dfs_count_matches_the_cache_miss_on_board_movement_nodes() {
        let rules = Rules::standard();
        // BOTH sites, because they are different code paths and a turn passes
        // through Movement exactly ONCE: at a Movement root the only
        // DFS-paying node is the root (`search_world`'s own expansion), and at
        // a Placement root it is a node inside the continuation (`promote`).
        // A cell with only the first reading passes a duplicate handle in
        // `promote` untouched — measured, not supposed (see the mutation note).
        let rules_ref = &rules;
        let cell = |world: &GameState, seat: PlayerId, label: &str| {
            let mut rng = ChaCha8Rng::seed_from_u64(11);
            let c = ctx(rules_ref, &WeightTable::V1, SearchScheme::default(), 0xB1A5);
            route_dfs_reset();
            let searched = search_world(world, seat, &c, &mut rng).expect("search");
            let counted = route_dfs_count();
            assert!(
                searched.stats.route_dfs_nodes > 0,
                "anti-vacuity ({label}): the cell must contain on-board Movement nodes",
            );
            assert_eq!(
                counted, searched.stats.route_dfs_nodes,
                "{label}: one President-route DFS per cache-miss on-board-Movement node, at K=1",
            );
            assert!(
                searched.stats.nodes > searched.stats.route_dfs_nodes,
                "anti-vacuity ({label}): 'nodes expanded' must NOT equal the DFS count \
                 ({} vs {}), or the pin would not distinguish the two readings",
                searched.stats.nodes,
                searched.stats.route_dfs_nodes,
            );
        };
        let (movement_world, movement_seat) = on_board_movement_point(&rules);
        cell(&movement_world, movement_seat, "movement root");
        let (world, seat) = on_board_placement_point(&rules);
        cell(&world, seat, "placement root, DFS inside the continuation");

        // K = 2 through the agent, where the sum is what the formula means.
        let view = observe(&world, seat).expect("observe");
        let mut agent = LookaheadAgent::with_knobs(
            &rules,
            SearchScheme::default(),
            2,
            DEFAULT_RETREAT_SAMPLES,
            WeightTable::V1,
            0xA11CE,
        );
        route_dfs_reset();
        agent.choose(&view, &rules).expect("a K=2 decision");
        let stats = *agent.last_stats().expect("a decision leaves stats");
        assert!(stats.route_dfs_nodes > 0, "anti-vacuity at K=2");
        assert_eq!(
            route_dfs_count(),
            stats.route_dfs_nodes,
            "the K worlds' DFS counts sum, and nothing else on the thread pays one",
        );
    }

    /// **ALLOCATION SANITY on ONE pick** (spec §4.4's cost half). The COUNT is
    /// asserted against a measured-then-set ceiling; the BYTES are RECORDED
    /// only — a sum of requested sizes is allocator-shape-sensitive and is not
    /// peak-outstanding, the same record-never-assert split
    /// `action_source_worstcase`'s meter states.
    ///
    /// MEASURED FIRST, at this fixture and seed (debug, re-measured 2026-07-27
    /// after the M2 candidate-seed fix moved the dice): **624,503 allocations ·
    /// 102,173,989 requested bytes** for one `B = 4` decision over 3,122 nodes
    /// and 12,610 scored successors — i.e. ~50 allocations per successor, which
    /// is what one `apply_move_unchecked` (a whole `GameState` clone, four board
    /// `BTreeMap`s included) plus one `observe` costs. The fixture is
    /// deliberately the EXPENSIVE shape: a 186-candidate organic Placement root,
    /// the phase the cost table shows carrying 84% of the wall clock.
    ///
    /// The ceiling is 1.5× the measurement rounded up — a SANITY ceiling, not a
    /// tight bound. It catches an order-of-magnitude regression: a per-node
    /// `PlayerView` kept alive, a line cloned down every branch, a retreat family
    /// materialized. **It does NOT catch the promote-then-cut shape, and it used
    /// to claim it did** (T5 review M1). Measured, both ways round, on this same
    /// fixture:
    ///
    /// | | shipped, cut-then-promote | mutated, promote-then-cut |
    /// |---|---|---|
    /// | allocations | 624,503 | 669,380 (+7.2%, ceiling 950,000 — PASSES) |
    /// | nodes | 3,122 | **11,614** |
    /// | successors scored | 12,610 | 12,610 |
    /// | move chosen | `PlaceThreeAreas [1, 6, 9]` | the same move |
    ///
    /// The reason is structural rather than a matter of ceiling tightness: an
    /// allocation total is dominated by the `apply` + `observe` charged per
    /// SCORED successor, and that count is identical under both shapes. Only the
    /// promotion count moves, and it moves by 3.7× while the allocation total
    /// moves by 7%. No ceiling drawn around 624,503 separates them.
    ///
    /// Hence the second assertion, which is the discriminator: **`nodes × 2 <
    /// scored`**. Measured-then-set from the pair above — shipped reads
    /// 6,244 < 12,610 (~2.0× of headroom), promote-then-cut reads 23,228 < 12,610
    /// and fails. It is the executable form of [`Successor`]'s claim that ranking
    /// needs an EVALUATION and only survivors need an ENUMERATION, and it is a
    /// ratio rather than a count, so it does not freeze a figure the weights
    /// legitimately move. **MUTATION-PROVEN** (2026-07-27, debug — promote every
    /// child at creation, then cut the promoted `Node`s on the same keyed draws,
    /// reverted by inverse edit; the same run drove the driven whole-game pin
    /// from 343.5 to 715.7 nodes/decision and that pin still passed, which is a
    /// second witness that no other pin nets this shape):
    ///
    /// ```text
    /// promote-then-cut must FAIL here: 11614 nodes against 12610 scored is
    /// a handle for all but a fraction of the children
    /// ```
    #[test]
    fn one_pick_allocates_within_a_measured_sanity_bound() {
        /// 1.5× the measured 624,503, rounded up. See the doc for its job — and
        /// for what it is NOT the net for.
        const CEILING: usize = 950_000;

        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let view = observe(&world, seat).expect("observe");
        let mut agent = LookaheadAgent::new(&rules, 0xA110C);

        let (report, allocs, bytes) =
            alloc_probe::count_allocs_bytes(|| agent.choose(&view, &rules));
        let report = report.expect("the honest chain produces a move");
        let stats = *agent.last_stats().expect("stats");
        println!(
            "one B={DEFAULT_BEAM} pick: {allocs} allocations · {bytes} requested bytes · \
             {} nodes · {} scored · move {:?}",
            stats.nodes, stats.scored, report.mv,
        );
        assert!(
            stats.nodes > 10,
            "anti-vacuity: the pick must have searched ({} nodes)",
            stats.nodes,
        );
        assert!(
            allocs < CEILING,
            "one pick allocated {allocs} times, over the {CEILING} sanity ceiling",
        );
        // THE DISCRIMINATOR the allocation ceiling is not (see the doc). Cut
        // first, promote after: most scored children must never buy a handle.
        assert!(
            stats.nodes * 2 < stats.scored,
            "promote-then-cut must FAIL here: {} nodes against {} scored is a handle for all \
             but a fraction of the children",
            stats.nodes,
            stats.scored,
        );
    }

    /// T10's rendering hook (spec §7): after a decision the agent reports EVERY
    /// root candidate with the value it folded to, and the move it actually
    /// returned is one of the maximisers — which is what makes
    /// `value − max` a MARGIN the CLI can print rather than a number of unclear
    /// sign.
    ///
    /// Anti-vacuity is load-bearing twice over: the fixture must offer more than
    /// one root (otherwise "the chosen move is a maximiser" is trivial), and at
    /// least two DISTINCT values must appear (otherwise a constant-valued
    /// reporter would pass — the shipped `WeightTable::V1` is what separates
    /// them, and `WeightTable::ZERO` is what would not).
    #[test]
    fn last_roots_reports_every_root_candidate_with_its_folded_value() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let view = observe(&world, seat).expect("observe");
        let mut agent = LookaheadAgent::new(&rules, 0x7100_0000_0000_0001);
        let report = agent.choose(&view, &rules).expect("the honest chain produces a move");
        let roots = agent.last_roots();
        let stats = *agent.last_stats().expect("stats");

        assert!(roots.len() > 1, "anti-vacuity: a one-candidate point cannot test a ranking");
        // Never MORE than the root candidates the search counted: the aggregate
        // folds equal moves across worlds (a no-op at K = 1) and the candidate
        // scheme already de-duplicates within a family, so at K = 1 this is an
        // equality in practice — asserted as `<=` because the fold, not the
        // count, is the property being pinned.
        assert!(
            roots.len() as u64 <= stats.root_candidates,
            "{} reported roots against {} root candidates searched",
            roots.len(),
            stats.root_candidates,
        );
        for (i, r) in roots.iter().enumerate() {
            assert!(
                !roots[..i].iter().any(|earlier| earlier.mv == r.mv),
                "a move appears twice in the ranking: {:?}",
                r.mv,
            );
        }

        let best = roots.iter().map(|r| r.value).max().expect("non-empty");
        let worst = roots.iter().map(|r| r.value).min().expect("non-empty");
        assert!(best > worst, "anti-vacuity: the evaluator must separate these candidates");
        let chosen = roots
            .iter()
            .find(|r| r.mv == report.mv)
            .expect("the returned move must be one of the reported roots");
        assert_eq!(chosen.value, best, "the agent picks a maximiser; the ranking must say so");
    }

    // -- the tie discipline (spec §3.2 at every tie point) --------------------

    /// **THE BEAM-FRONTIER TIE, the research critic's unfixed residue.**
    ///
    /// Under a CONSTANT evaluator every child ties, so the beam's cut is pure
    /// tie-breaking — and a stable-sort truncation would keep the enumeration's
    /// first `B` every time, whatever the key. Two different tie keys must
    /// therefore reach different lines; the same key must reproduce.
    ///
    /// The root argmax is not involved: the comparison is per ROOT INDEX, and
    /// `TieSite::Root` keys a different site.
    ///
    /// **MUTATION-PROVEN AT AUTHORING** (2026-07-27, debug) by emptying
    /// `truncate`'s partial Fisher-Yates loop so the cut fell back to the stable
    /// sort's order, and reverting by inverse edit (never `git checkout` — L7):
    ///
    /// ```text
    /// a different tie key must change which tied children the beam kept
    /// ```
    ///
    /// i.e. the two keys produced IDENTICAL lines at every root index, which is
    /// precisely the residue the research critic left unfixed.
    #[test]
    fn the_beam_truncation_draws_over_tied_tails() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let lines = |key: u64| -> Vec<Vec<Move>> {
            let mut rng = ChaCha8Rng::seed_from_u64(5);
            let c = ctx(&rules, &WeightTable::ZERO, SearchScheme::default(), key);
            let s = search_world(&world, seat, &c, &mut rng).expect("search");
            s.roots.iter().map(|r| s.line(r.leaf)).collect()
        };
        let a = lines(0x1111);
        assert_eq!(a, lines(0x1111), "the same tie key must reproduce every line");

        let b = lines(0x2222);
        assert_eq!(a.len(), b.len(), "the two runs must offer the same root candidates");
        assert!(
            a.iter().zip(&b).any(|(x, y)| x != y),
            "a different tie key must change which tied children the beam kept",
        );
    }

    /// The root argmax draws over the FULL tied set, from the keyed word.
    /// Under a constant evaluator every root candidate ties, so re-keying must
    /// reach many different moves — a first-wins root would return one.
    #[test]
    fn the_root_argmax_draws_over_the_whole_tied_set() {
        const KEYS: u64 = 40;
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let view = observe(&world, seat).expect("observe");

        let mut seen = std::collections::BTreeSet::new();
        for k in 0..KEYS {
            let mut agent = LookaheadAgent::with_knobs(
                &rules,
                SearchScheme::TurnBoundary { beam: 1 },
                DEFAULT_WORLDS,
                DEFAULT_RETREAT_SAMPLES,
                WeightTable::ZERO,
                0x5EED ^ k,
            );
            let r = agent.choose(&view, &rules).expect("a constant evaluator still answers");
            seen.insert(format!("{:?}", r.mv));
        }
        assert!(
            seen.len() > (KEYS / 4) as usize,
            "a uniform root draw must spread over {KEYS} keys, reached only {}",
            seen.len(),
        );
    }

    /// A tie draw costs exactly one KEYED word and ZERO play-stream words,
    /// whatever the tie count — `greedy`'s constant-consumption property, one
    /// rung up, where it is also what keys the K worlds in common.
        /// SCOPE (T8 review): this pin drives the search internals, NOT
    /// `LookaheadAgent::choose`'s K-world loop — the T8 review measured both
    /// canonical world-loop keying regressions passing this pin. The world
    /// loop's detector is `tests/observation_leakage.rs`'s divergence cell
    /// (`lookahead_cross_world_divergence_is_zero_and_the_dice_are_common`-
    /// class), whose triage message says so.
    #[test]
    fn a_tie_draw_is_one_keyed_word_and_is_world_common() {
        let ties = TieStream::new(0xFEED);
        for len in [1usize, 2, 3, 17, 512, 3_231] {
            let i = ties.pick(TieSite::Root, len);
            assert!(i < len, "index {i} out of range for {len}");
            assert_eq!(i, ties.pick(TieSite::Root, len), "the same site must redraw the same word");
        }
        // Distinct sites are distinct draws; the tag keeps the families apart
        // even where the coordinates coincide.
        assert_ne!(
            ties.word(TieSite::Beam { root: 1, depth: 0, slot: 0 }),
            ties.word(TieSite::Extension { root: 1, depth: 0, branch: 0 }),
        );
        assert_ne!(
            ties.word(TieSite::Beam { root: 1, depth: 0, slot: 0 }),
            ties.word(TieSite::Beam { root: 1, depth: 0, slot: 1 }),
        );
        // A different key is a different decision.
        assert_ne!(ties.word(TieSite::Root), TieStream::new(0xFEEE).word(TieSite::Root));

        // **T5 review S3: SIBLING extension steps must not share a word.** Two
        // lines that reach a free extension at the same `(root, depth)` differ
        // only in the index path that got them there, so `branch` is the whole
        // separation — without it the two picks are perfectly correlated.
        assert_ne!(
            ties.word(TieSite::Extension { root: 2, depth: 1, branch: descend_branch(0, 0) }),
            ties.word(TieSite::Extension { root: 2, depth: 1, branch: descend_branch(0, 1) }),
            "sibling extension steps must key different words",
        );
        // And the fold really does separate index PATHS, not just the last step:
        // (0 then 1) and (1 then 0) are different lines at the same depth.
        assert_ne!(
            descend_branch(descend_branch(0, 0), 1),
            descend_branch(descend_branch(0, 1), 0),
            "the branch fold must be path-sensitive, not order-blind",
        );
    }

    // -- determinism (spec §7's repeatability, and the gate's byte-identity) --

    /// Same seed ⇒ same moves; a different seed CAN differ, and the divergence
    /// is EXHIBITED rather than asserted in the abstract (`greedy`'s shape).
    /// The divergence half runs under a constant evaluator because the
    /// tie-break is the only place a seed can change an answer.
    #[test]
    fn same_seed_agrees_move_for_move_and_a_different_seed_can_diverge() {
        let rules = Rules::standard();
        let play = |weights: WeightTable, agent_seed: u64| -> Vec<String> {
            let mut agent = LookaheadAgent::with_knobs(
                &rules,
                SearchScheme::TurnBoundary { beam: 2 },
                DEFAULT_WORLDS,
                DEFAULT_RETREAT_SAMPLES,
                weights,
                agent_seed,
            );
            first_decisions(&mut agent, MASTER, 6, &rules)
        };
        let a = play(WeightTable::V1, 0xA1);
        assert_eq!(a.len(), 6, "the probe must reach six decisions");
        assert_eq!(a, play(WeightTable::V1, 0xA1), "the same agent seed must reproduce");

        let c = play(WeightTable::ZERO, 0xA1);
        assert_eq!(c, play(WeightTable::ZERO, 0xA1), "determinism holds through the tie-break");
        assert_ne!(
            c,
            play(WeightTable::ZERO, 0xB2),
            "with the draw live, a different agent seed must diverge",
        );
    }

    /// **THE STREAM-CONSUMPTION PATTERN, made executable end to end.**
    ///
    /// A decision spends, from the agent's own play stream and in this order:
    /// **word 1 = the sampler seed, word 2 = the tie key, word 3 = the candidate
    /// seed** — three words, and then nothing at all, whatever `K` is and
    /// whatever the search does inside. One sampler seed serves all K worlds
    /// (`BeliefSampler` is built once and sampled K times off its OWN stream);
    /// the tie machinery spends nothing at all ([`TieStream::pick`] takes `&self`
    /// and no `Rng`); and since the T5-review M2 fix the candidate scheme spends
    /// nothing either — each world's search runs on a fresh
    /// [`candidate_rng`] keyed by word 3, which is what made the count constant
    /// rather than merely bounded.
    ///
    /// The pin RECONSTRUCTS a decision from a mirrored stream: take the three
    /// words in that order, re-run the agent's own root argmax, and require the
    /// same move. That fails if the agent draws a different NUMBER of words,
    /// draws them in a different ORDER, keys the root draw off anything but word
    /// 2, or keys the candidate dice off anything but word 3. The exact COUNT is
    /// pinned separately by
    /// `the_agents_play_stream_advances_exactly_three_words_per_decision`, which
    /// is the half that catches a search reaching back into the play stream.
    ///
    /// What it does NOT claim (and `greedy`'s doc does): that two WEIGHT TABLES
    /// produce the same candidate SETS. They cannot here — the candidate scheme
    /// draws at every node, and which nodes exist depends on which ones the beam
    /// kept, which depends on the weights. What the fix did buy is one rung
    /// weaker and worth naming: the agent's own stream POSITION after a decision
    /// is now weight-independent, because all three words are drawn before the
    /// first world is searched.
    #[test]
    fn the_agents_stream_consumption_per_decision_is_reconstructible() {
        const SEED: u64 = 0xC0DE_5EED;
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let view = observe(&world, seat).expect("observe");

        let mut agent = LookaheadAgent::with_knobs(
            &rules,
            SearchScheme::TurnBoundary { beam: 3 },
            DEFAULT_WORLDS,
            DEFAULT_RETREAT_SAMPLES,
            WeightTable::V1,
            SEED,
        );
        let played = agent.choose(&view, &rules).expect("a decision").mv;

        // The mirror: word 1 = sampler seed, word 2 = tie key, word 3 = the
        // candidate seed every world's search is keyed from.
        let mut mirror = ChaCha8Rng::seed_from_u64(SEED);
        let sampler_seed: u64 = mirror.gen();
        let tie_key = mirror.next_u64();
        let candidate_seed = mirror.next_u64();
        let mirrored_world =
            BeliefSampler::from_view(&view, &rules, sampler_seed).expect("sampler").sample();
        let c = ctx(&rules, &WeightTable::V1, SearchScheme::TurnBoundary { beam: 3 }, tie_key);
        let searched = search_world(
            &mirrored_world,
            seat,
            &c,
            &mut candidate_rng(candidate_seed),
        )
        .expect("search");

        // The agent's own root argmax, re-run over the reconstruction.
        let best = searched.roots.iter().map(|r| r.value).max().expect("candidates");
        let tied: Vec<&RootValue> = searched.roots.iter().filter(|r| r.value == best).collect();
        let chosen = &tied[TieStream::new(tie_key).pick(TieSite::Root, tied.len())].mv;
        assert_eq!(
            &played, chosen,
            "the decision must reconstruct from (word 1 = sampler seed, word 2 = tie key)",
        );

        // ANTI-VACUITY: the reconstruction is sensitive to the consumption
        // pattern it claims. Reading the tie key FIRST — the same two words in
        // the other order, word 3 left exactly where it was — must not reproduce
        // the move, or the pin would pass whatever the agent did with its stream.
        let mut swapped = ChaCha8Rng::seed_from_u64(SEED);
        let wrong_tie = swapped.next_u64();
        let wrong_seed: u64 = swapped.gen();
        let same_candidate_seed = swapped.next_u64();
        assert_eq!(same_candidate_seed, candidate_seed, "only the first two words are swapped");
        let wrong_world =
            BeliefSampler::from_view(&view, &rules, wrong_seed).expect("sampler").sample();
        let c2 = ctx(&rules, &WeightTable::V1, SearchScheme::TurnBoundary { beam: 3 }, wrong_tie);
        let other = search_world(
            &wrong_world,
            seat,
            &c2,
            &mut candidate_rng(same_candidate_seed),
        )
        .expect("search");
        let best2 = other.roots.iter().map(|r| r.value).max().expect("candidates");
        let tied2: Vec<&RootValue> = other.roots.iter().filter(|r| r.value == best2).collect();
        let chosen2 = &tied2[TieStream::new(wrong_tie).pick(TieSite::Root, tied2.len())].mv;
        assert_ne!(
            &played, chosen2,
            "anti-vacuity: swapping the two words must change the answer, or the order \
             this pin claims is not being tested",
        );
    }

    /// Re-run ONE decision from a mirrored play stream, spelling out
    /// [`LookaheadAgent::choose`]'s own shape: `skip` decisions' worth of words
    /// consumed first (three each), then word 1 = the sampler seed for all `k`
    /// worlds, word 2 = the tie key, word 3 = the candidate seed every world's
    /// search is keyed from, then the argmax over the K-mean with the
    /// [`TieSite::Root`] draw.
    // A mirror has to match the agent's construction knob for knob, so every one
    // of them is spelled out here on purpose rather than defaulted.
    #[allow(clippy::too_many_arguments)]
    fn reconstruct_decision(
        rules: &Rules,
        view: &PlayerView,
        seat: PlayerId,
        scheme: SearchScheme,
        weights: WeightTable,
        k: u8,
        agent_seed: u64,
        skip: usize,
    ) -> Move {
        let mut mirror = ChaCha8Rng::seed_from_u64(agent_seed);
        for _ in 0..skip * 3 {
            let _ = mirror.next_u64();
        }
        let sampler_seed: u64 = mirror.gen();
        let tie_key = mirror.next_u64();
        let candidate_seed = mirror.next_u64();

        let mut sampler = BeliefSampler::from_view(view, rules, sampler_seed).expect("sampler");
        let c = ctx(rules, &weights, scheme, tie_key);
        // (move, value sum, count) — `Aggregate`'s arithmetic, no more.
        let mut agg: Vec<(Move, i64, i64)> = Vec::new();
        for _ in 0..k {
            let world = sampler.sample();
            let searched = search_world(&world, seat, &c, &mut candidate_rng(candidate_seed))
                .expect("search");
            for r in &searched.roots {
                match agg.iter_mut().find(|(m, _, _)| *m == r.mv) {
                    Some(a) => {
                        a.1 = a.1.saturating_add(r.value);
                        a.2 += 1;
                    }
                    None => agg.push((r.mv.clone(), r.value, 1)),
                }
            }
        }
        let best = agg.iter().map(|(_, s, n)| s / n).max().expect("candidates");
        let tied: Vec<&(Move, i64, i64)> =
            agg.iter().filter(|(_, s, n)| s / n == best).collect();
        tied[TieStream::new(tie_key).pick(TieSite::Root, tied.len())].0.clone()
    }

    /// **EXACTLY THREE WORDS PER DECISION, AND THE SEARCH TOUCHES NONE OF THEM**
    /// (T5 review M2). The count used to be variable: the search was handed the
    /// agent's own `&mut self.rng`, so a decision's consumption was `2 + (however
    /// many retreat samples and reservoir draws the search happened to make)` —
    /// and at `K = 2` the second world therefore started at a SHIFTED position.
    /// With each world keyed from word 3 instead, a decision spends 3 words flat.
    ///
    /// The pin addresses decision `n` by consuming `3n` mirror words first, so it
    /// fails on any drift in the count — which is what makes it, and not the
    /// reconstruction pin above, the half that catches a search reaching back
    /// into the play stream. Run at `K = 1` AND `K = 2`, because the K loop is
    /// where the old shape's drift was largest.
    ///
    /// Under [`WeightTable::ZERO`] every root candidate ties, so the answer is
    /// the keyed [`TieSite::Root`] draw and nothing else — which both maximizes
    /// sensitivity to word 2's POSITION and gives the anti-vacuity below its
    /// teeth (three consecutive decisions reach more than one distinct move).
    #[test]
    fn the_agents_play_stream_advances_exactly_three_words_per_decision() {
        const SEED: u64 = 0x3_0000_0003;
        const DECISIONS: usize = 3;
        let rules = Rules::standard();
        let (world, seat) = on_board_movement_point(&rules);
        let view = observe(&world, seat).expect("observe");
        let scheme = SearchScheme::TurnBoundary { beam: 2 };

        for k in [1u8, 2] {
            let mut agent = LookaheadAgent::with_knobs(
                &rules,
                scheme,
                k,
                DEFAULT_RETREAT_SAMPLES,
                WeightTable::ZERO,
                SEED,
            );
            let played: Vec<Move> = (0..DECISIONS)
                .map(|_| agent.choose(&view, &rules).expect("a decision").mv)
                .collect();
            let mirrored: Vec<Move> = (0..DECISIONS)
                .map(|n| {
                    reconstruct_decision(
                        &rules,
                        &view,
                        seat,
                        scheme,
                        WeightTable::ZERO,
                        k,
                        SEED,
                        n,
                    )
                })
                .collect();
            assert_eq!(
                played, mirrored,
                "at K={k} decision n must reconstruct from mirror words 3n+1..3n+3",
            );
            // ANTI-VACUITY: the OFFSETS have to matter. If three consecutive
            // decisions all answered the same, the pin would pass for any
            // consumption count at all.
            let distinct: std::collections::BTreeSet<String> =
                played.iter().map(|m| format!("{m:?}")).collect();
            assert!(
                distinct.len() > 1,
                "anti-vacuity at K={k}: {DECISIONS} keyed draws reached only {} distinct move(s)",
                distinct.len(),
            );
        }
    }

    /// The first `n` on-board Movement decision points one Greedy-v1 2P game
    /// reaches, each as a SAMPLED world plus the seat to move.
    ///
    /// [`point_where`]'s shape, harvested from ONE walk: the M2 repro needs a
    /// POPULATION of organic points rather than a single fixture, and Movement
    /// points are the cheap shape that still draws (the retreat family samples
    /// `M` times at the root, so the candidate dice are live there).
    fn movement_points(rules: &Rules, seed: u64, n: usize) -> Vec<(GameState, PlayerId)> {
        let pending = new_game(GameConfig::for_player_count(2), rules, seed).expect("2P setup");
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
                .expect("finalize");
        let mut agent = GreedyAgent::new(rules, 0x77);
        let mut out = Vec::new();
        for _ in 0..600 {
            if out.len() == n {
                break;
            }
            let seat = state.current_player;
            let view = observe(&state, seat).expect("observe");
            if state.turn_phase == TurnPhase::Movement
                && state.pending_choices.is_empty()
                && matches!(state.current_player().president_location, PresidentLocation::OnBoard(_))
            {
                let world =
                    BeliefSampler::from_view(&view, rules, 1).expect("sampler").sample();
                out.push((world, seat));
            }
            let r = agent.choose(&view, rules).expect("move");
            state = apply_move_unchecked(&state, &r.mv, rules).expect("apply");
        }
        out
    }

    /// **THE M2 REPRO, both directions** (T5 review M2). The module claimed
    /// residual cross-world divergence was "determinization-driven and nothing
    /// else"; that was true of the tie draws ([`TieStream`] is built before the
    /// world loop) and FALSE of the candidate scheme, which walked the agent's
    /// CONTINUING stream inside the `for w in 0..k` loop — so world `k` drew its
    /// retreat samples and reservoirs from a shifted position and two worlds
    /// disagreed even where their states were identical.
    ///
    /// The two halves are the same experiment run both ways over the same
    /// organic points, and the second is the anti-vacuity the first needs:
    /// searching ONE world twice must agree exactly under the dice the agent now
    /// hands every world, and must DISAGREE under one continuing stream. The
    /// reviewer measured 60/60 divergent at the old seam; this cell reproduces
    /// that direction and pins the new one at 0.
    ///
    /// Cross-world divergence at K > 1 is now determinization and nothing else,
    /// which is the property T8's instrument rests on. Note what is NOT claimed:
    /// two DIFFERENT worlds still get different candidate LISTS — common dice,
    /// different states — and that difference is the signal, not noise.
        /// SCOPE (T8 review): this pin drives the search internals, NOT
    /// `LookaheadAgent::choose`'s K-world loop — the T8 review measured both
    /// canonical world-loop keying regressions passing this pin. The world
    /// loop's detector is `tests/observation_leakage.rs`'s divergence cell
    /// (`lookahead_cross_world_divergence_is_zero_and_the_dice_are_common`-
    /// class), whose triage message says so.
    #[test]
    fn the_candidate_dice_are_world_common_within_a_decision() {
        const DICE: u64 = 0xD1CE_D1CE;
        const N: usize = 20;
        let rules = Rules::standard();
        let points = movement_points(&rules, MASTER, N);
        assert_eq!(points.len(), N, "the walk must reach {N} on-board Movement points");

        // Roots, values AND lines: the whole searched answer, not just the move.
        let fingerprint = |s: &WorldSearch| -> Vec<String> {
            s.roots
                .iter()
                .map(|r| format!("{:?}|{}|{:?}", r.mv, r.value, s.line(r.leaf)))
                .collect()
        };
        let mut common_diverged = 0usize;
        let mut continuing_diverged = 0usize;
        for (world, seat) in &points {
            let c = ctx(&rules, &WeightTable::V1, SearchScheme::default(), 0xB1A5);
            // What every world is handed since the fix: a fresh stream off ONE
            // per-decision seed.
            let a = search_world(world, *seat, &c, &mut candidate_rng(DICE)).expect("search");
            let b = search_world(world, *seat, &c, &mut candidate_rng(DICE)).expect("search");
            if fingerprint(&a) != fingerprint(&b) {
                common_diverged += 1;
            }
            // The pre-fix shape: ONE continuing stream across both worlds.
            let mut continuing = ChaCha8Rng::seed_from_u64(DICE);
            let a2 = search_world(world, *seat, &c, &mut continuing).expect("search");
            let b2 = search_world(world, *seat, &c, &mut continuing).expect("search");
            if fingerprint(&a2) != fingerprint(&b2) {
                continuing_diverged += 1;
            }
        }
        println!(
            "same-world double search over {N} organic Movement points: \
             world-common dice {common_diverged}/{N} divergent · \
             continuing stream {continuing_diverged}/{N} divergent",
        );
        assert_eq!(
            common_diverged, 0,
            "two searches of the SAME world under the per-decision candidate seed must agree",
        );
        assert_eq!(
            continuing_diverged, N,
            "anti-vacuity: a continuing stream must diverge, or this fixture cannot see the \
             channel the fix closed",
        );
    }

    // -- S2, the diagnostic arm (spec §4.1) -----------------------------------

    /// **QUIESCENCE: forced chains extend FREE.** With `d = 2` the atomic arm
    /// may spend two charged moves — but its lines reach further, because a
    /// point offering ≤ 1 candidate (or a pending choice) extends without a
    /// charge. A depth-only implementation would cap every line at `d`.
    #[test]
    fn the_atomic_arm_extends_forced_chains_free_of_depth_charge() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let at_depth = |d: u8| -> (SearchStats, usize) {
            let mut rng = ChaCha8Rng::seed_from_u64(5);
            let c = ctx(&rules, &WeightTable::V1, SearchScheme::AtomicDiag { depth: d }, 0xB1A5);
            let s = search_world(&world, seat, &c, &mut rng).expect("search");
            let longest = s.roots.iter().map(|r| s.line(r.leaf).len()).max().unwrap_or(0);
            (s.stats, longest)
        };
        let (stats1, long1) = at_depth(1);
        let (stats2, long2) = at_depth(2);

        assert!(
            long2 > 2,
            "quiescence must carry a d=2 line past 2 moves, longest was {long2}",
        );
        assert!(
            stats2.nodes > stats1.nodes,
            "depth must still be charged at branch points ({} nodes at d=2 vs {} at d=1)",
            stats2.nodes,
            stats1.nodes,
        );
        assert!(long1 >= 1 && stats1.nodes > 0, "anti-vacuity: d=1 must search something");
    }

    /// The diagnostic arm says so in its own controller NAME, so a corpus
    /// reader filtering on `controller` can see it without a document — and it
    /// carries its OWN version const (spec §0.5's per-controller rule).
    #[test]
    fn provenance_names_each_controller_with_its_own_knobs_and_version() {
        let rules = Rules::standard();

        let s1 = LookaheadAgent::new(&rules, 0x1234).controller();
        assert_eq!(s1.name, "lookahead-v1");
        assert_eq!(s1.version, LOOKAHEAD_IMPL_VERSION);
        assert_eq!(s1.agent_seed, Some(0x1234));
        assert_eq!(s1.params["b"], i64::from(DEFAULT_BEAM));
        assert_eq!(s1.params["k"], i64::from(DEFAULT_WORLDS));
        assert_eq!(s1.params["m"], i64::from(DEFAULT_RETREAT_SAMPLES));
        assert_eq!(s1.params["eval_impl"], i64::from(EVAL_IMPL_VERSION));
        assert_eq!(s1.params["weights"], weight_table_identity(&WeightTable::V1));
        assert!(!s1.params.contains_key("d"), "S1 has no depth knob");

        let s2 = LookaheadAgent::atomic_diag(&rules, DEFAULT_ATOMIC_DEPTH, 0x1234).controller();
        assert_eq!(s2.name, "lookahead-atomic-diag-v1");
        assert!(s2.name.contains("diag"), "the diagnostic arm is labelled in its own name");
        assert_eq!(s2.version, ATOMIC_DIAG_IMPL_VERSION);
        assert_eq!(s2.params["d"], i64::from(DEFAULT_ATOMIC_DEPTH));
        assert!(!s2.params.contains_key("b"), "S2 has no beam knob");

        // The knobs really ride: two cells at different B are distinguishable.
        let wide = LookaheadAgent::with_knobs(
            &rules,
            SearchScheme::TurnBoundary { beam: 8 },
            3,
            32,
            WeightTable::ZERO,
            9,
        )
        .controller();
        assert_eq!(wide.params["b"], 8);
        assert_eq!(wide.params["k"], 3);
        assert_eq!(wide.params["m"], 32);
        assert_ne!(wide.params["weights"], s1.params["weights"]);
    }

    /// All THREE sizing knobs floor at 1, and the agent REPORTS the floored
    /// value — six-lens review: `worlds` was clamped and documented while
    /// `beam` was clamped only inside `search_world` (undocumented, so not a
    /// knob fact) and `depth` was never clamped at all, leaving `d = 0` and
    /// `d = 1` accidentally equal through `saturating_sub`. The runner refuses
    /// all three zeros as user errors; this is the library floor beneath a
    /// caller that computes a knob.
    ///
    /// Both halves matter: the SCHEME must report what will run (a caller
    /// reading `scheme()` back is entitled to the real search), and the
    /// PROVENANCE must not stamp a corpus record with knobs no seat played on.
    #[test]
    fn every_sizing_knob_floors_at_one_and_says_so() {
        let rules = Rules::standard();

        let b0 = LookaheadAgent::with_knobs(
            &rules,
            SearchScheme::TurnBoundary { beam: 0 },
            0,
            8,
            WeightTable::V1,
            7,
        );
        assert_eq!(
            b0.scheme(),
            SearchScheme::TurnBoundary { beam: 1 },
            "a beam of 0 truncates every frontier away; it must be raised, and reported raised",
        );
        assert_eq!(b0.controller().params["b"], 1, "provenance prints the floored beam");
        assert_eq!(b0.controller().params["k"], 1, "and the long-standing worlds floor");

        let d0 = LookaheadAgent::atomic_diag(&rules, 0, 7);
        assert_eq!(
            d0.scheme(),
            SearchScheme::AtomicDiag { depth: 1 },
            "d=0 and d=1 already searched identically; the floor states it rather than hiding it",
        );
        assert_eq!(d0.controller().params["d"], 1, "provenance prints the floored depth");

        // Anti-vacuity: the floor is not clamping knobs that were already legal.
        let d3 = LookaheadAgent::atomic_diag(&rules, 3, 7);
        assert_eq!(d3.scheme(), SearchScheme::AtomicDiag { depth: 3 });
        assert_eq!(d3.controller().params["d"], 3);
    }

    // -- the agent (spec §0.3(b), §4.3) ---------------------------------------

    /// **SPEC §0.3(b) FOR THIS RUNG.** Every chosen move is explained, and the
    /// explanation is taken at the HAND-OFF the search valued — the state the
    /// search actually priced, not the move's own successor, which is a
    /// different state and (usually) a different number. The principal
    /// continuation is stored beside it, because an explanation of a searched
    /// move without its line is an explanation of the wrong thing.
    #[test]
    fn every_decision_is_explained_at_the_hand_off_it_was_valued_at() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let view = observe(&world, seat).expect("observe");
        let mut agent = LookaheadAgent::new(&rules, 0xC0FFEE);
        let tally = agent.tally();
        assert_eq!(tally.explained(), 0);

        let report = agent.choose(&view, &rules).expect("a decision");
        assert_eq!(tally.explained(), 1, "one explanation per decision, read off the clone");

        let line = agent.last_continuation().to_vec();
        assert!(!line.is_empty(), "a continuation always contains at least the chosen move");
        assert_eq!(line[0], report.mv, "the continuation starts at the move actually played");
        assert_eq!(
            tally.continuation_moves(),
            line.len() as u64,
            "the tally sums the continuations it explained",
        );

        // The explanation is the hand-off's: replay the line in the world the
        // agent searched and re-derive it independently.
        let mut at = agent_world(&rules, &view, 0xC0FFEE);
        for mv in &line {
            at = apply_move_unchecked(&at, mv, &rules).expect("replay");
        }
        assert!(at.current_player != seat || at.is_game_over(), "the line ends at a hand-off");
        let expected = eval::explain(
            &observe(&at, seat).expect("observe"),
            &rules,
            &WeightTable::V1,
        );
        let got = agent.last_explanation().expect("a decision leaves an explanation");
        assert_eq!(got.total, expected.total, "the explanation is the hand-off's");
        assert_eq!(got.terms.len(), eval::Feature::COUNT, "every feature is reported");

        // And a one-ply successor is NOT what was explained — the rung's whole
        // point (skipped in the rare case where the chosen move IS the hand-off).
        if line.len() > 1 {
            let one_ply = apply_move_unchecked(
                &agent_world(&rules, &view, 0xC0FFEE),
                &report.mv,
                &rules,
            )
            .expect("apply");
            let one_ply_total =
                eval::explain(&observe(&one_ply, seat).expect("observe"), &rules, &WeightTable::V1)
                    .total;
            assert_ne!(
                got.total, one_ply_total,
                "the searched value must differ from the one-ply value, or the rung is greedy",
            );
        }
    }

    /// Reproduce the world an agent with `agent_seed` samples on its FIRST
    /// decision: its stream's first word is the sampler seed.
    fn agent_world(rules: &Rules, view: &PlayerView, agent_seed: u64) -> GameState {
        let mut rng = ChaCha8Rng::seed_from_u64(agent_seed);
        let sampler_seed: u64 = rng.gen();
        BeliefSampler::from_view(view, rules, sampler_seed).expect("sampler").sample()
    }

    /// **THE K LOOP (spec §4.3): whole searches, aggregated at the ROOT.** At
    /// K = 2 the agent runs the search twice over two worlds and averages each
    /// root MOVE's value; it never interleaves the worlds into one tree.
    #[test]
    fn the_k_loop_reruns_the_whole_search_and_aggregates_at_the_root() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let view = observe(&world, seat).expect("observe");

        let stats_at = |k: u8| -> SearchStats {
            let mut agent = LookaheadAgent::with_knobs(
                &rules,
                SearchScheme::TurnBoundary { beam: 2 },
                k,
                DEFAULT_RETREAT_SAMPLES,
                WeightTable::V1,
                0x4B01,
            );
            agent.choose(&view, &rules).expect("a decision");
            *agent.last_stats().expect("stats")
        };
        let one = stats_at(1);
        let two = stats_at(2);
        assert!(
            two.nodes > one.nodes,
            "K=2 must pay for two whole searches ({} vs {} nodes)",
            two.nodes,
            one.nodes,
        );
        assert_eq!(two.handles, two.nodes, "one handle per node, in every world");
    }

    /// **THE DRIVEN PIN: the rung plays a whole game through the ARENA driver,
    /// in debug, with per-move invariants ON.**
    ///
    /// That driver is not a thin wrapper and this is not a smoke test of one
    /// call — `drive` adds three checks per move that no in-module test
    /// exercises: the `is_move_legal(PathMode::Canonical)` boundary against the
    /// TRUE state (the agent only ever saw a sampled world), a debug-tier
    /// `get_legal_moves(..).contains(&mv)` membership assert (which catches a
    /// non-canonical route witness), and a debug-tier cross-check of the
    /// AGENT-ATTESTED volume against the true state's. The third is the one
    /// this rung could plausibly fail on its own: the volume travels from the
    /// root handle of a SAMPLED world and is compared against the truth.
    ///
    /// Spec §0.3(b)'s tally identity rides the same game, as it does
    /// `crate::game::greedy`'s: every applied move was chosen by a decision
    /// that produced an explanation.
    ///
    /// **Cost, stated because §10.1 pre-declares this hazard, and re-measured at
    /// the T5 review (S5) because this site and the module doc used to disagree:**
    /// **5.49 s debug** for one 310-move 2P game (~17.7 ms per decision), which is
    /// HALF the whole module's 11.04 s debug run, against 0.78 s for all 21 tests
    /// in release. ONE game at ONE master, capped well below `HONEST_MAX_TURNS` so
    /// a non-terminating search fails in seconds; the strength cells are T11's.
    #[test]
    fn the_rung_plays_a_whole_game_through_the_arena_driver() {
        use crate::game::arena::{agent_stream_seed, run_arena_game_capped, Slot};
        const CAP: u32 = 1_500;

        let rules = Rules::standard();
        let world = crate::game::seeds::WorldSeed(MASTER);
        let tallies: Arc<std::sync::Mutex<Vec<SearchTally>>> = Arc::default();
        let sink = Arc::clone(&tallies);
        let mut make = |seat: PlayerId, _n: u8| -> Box<dyn ArenaAgent> {
            let agent =
                LookaheadAgent::new(&rules, agent_stream_seed(world, seat, Slot(seat.0)));
            sink.lock().unwrap().push(agent.tally());
            Box::new(agent)
        };
        let stats = run_arena_game_capped(world, 2, &rules, &mut make, true, CAP)
            .expect("the rung must play a game that ENDS, through the checked driver");

        let handles = tallies.lock().unwrap().clone();
        assert_eq!(handles.len(), 2, "one clone-out handle per seat");
        let explained: u64 = handles.iter().map(|t| t.explained()).sum();
        let continuation: u64 = handles.iter().map(|t| t.continuation_moves()).sum();
        let nodes: u64 = handles.iter().map(|t| t.nodes()).sum();
        let cap_hits: u64 = handles.iter().map(|t| t.ply_cap_hits()).sum();
        let free_hits: u64 = handles.iter().map(|t| t.free_cap_hits()).sum();
        println!(
            "arena-driven 2P Lookahead self-play: turns {} · moves {} · nodes/decision {:.1} · \
             mean continuation {:.2}",
            stats.turns,
            stats.moves_applied,
            nodes as f64 / explained.max(1) as f64,
            continuation as f64 / explained.max(1) as f64,
        );
        assert!(stats.turns < CAP, "the game must end inside the cap, took {}", stats.turns);
        assert!(explained > 0, "anti-vacuity: the game must have made decisions");
        assert_eq!(
            explained, stats.moves_applied,
            "every applied move was chosen by a decision that produced an explanation",
        );
        // T5 review S1: a ply-capped continuation is valued at the CAP and not at
        // a seat transition, so a run that starts hitting the guard has to say so
        // rather than quietly explaining a mid-turn state. Zero over a whole game
        // is the module doc's claim, made a failure condition.
        assert_eq!(
            cap_hits, 0,
            "MAX_TURN_PLIES stopped {cap_hits} continuation(s) — every one of them was \
             explained mid-turn",
        );
        // The S2 guard's counterpart, and read here for completeness rather than
        // as evidence about S2: this pin drives the S1 rung, which never enters
        // the quiescence arm, so a nonzero here would mean the counter is being
        // incremented from a path S1 should not have.
        assert_eq!(
            free_hits, 0,
            "MAX_FREE_EXTENSION recorded {free_hits} hit(s) under the TURN-BOUNDARY scheme, \
             which never runs the quiescence arm at all",
        );

        let run = stats.record.run.as_ref().expect("a driven game stamps provenance");
        assert_eq!(run.seats[0].controller, LOOKAHEAD_CONTROLLER, "provenance names the rung");
        assert_eq!(run.seats[0].version, LOOKAHEAD_IMPL_VERSION, "and its version");
    }

    // -- the honest chain (spec §3.1's no-fallback rule) ----------------------

    /// A view the sampler refuses is a typed hard error with no fallback arm,
    /// and it leaves no explanation behind.
    #[test]
    fn a_view_the_sampler_refuses_is_a_hard_error_with_no_fallback() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let mut doctored = observe(&world, seat).expect("observe");
        doctored.order_deck_len += 1;

        let mut agent = LookaheadAgent::new(&rules, 0xBADF00D);
        let err = agent.choose(&doctored, &rules).expect_err("the sampler must refuse");
        assert!(err.contains("belief sampler refused"), "the refusal must be named: {err}");
        assert!(
            agent.last_explanation().is_none() && agent.tally().explained() == 0,
            "a refused decision must not leave an explanation behind",
        );
        let clean = observe(&world, seat).expect("observe");
        assert!(agent.choose(&clean, &rules).is_ok(), "the undoctored view must be answerable");
    }

    /// A phase that is not a decision point at all is named, not invented
    /// around.
    #[test]
    fn a_view_at_a_phase_with_no_families_is_a_hard_error() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let mut view = observe(&world, seat).expect("observe");
        view.turn_phase = TurnPhase::TurnEnd;
        assert!(view.pending_choices.is_empty(), "the queue must be empty, or TurnEnd still asks");

        let mut agent = LookaheadAgent::new(&rules, 0x0DD1);
        let err = agent.choose(&view, &rules).expect_err("TurnEnd offers no families");
        assert!(err.contains("offers no families"), "the phase must be named: {err}");
        assert!(err.contains("TurnEnd"), "the message must report WHICH phase: {err}");
    }

    /// A search rooted at somebody else's decision point is refused rather than
    /// silently completing the wrong seat's turn.
    #[test]
    fn searching_another_seats_decision_point_is_refused() {
        let rules = Rules::standard();
        let (world, seat) = placement_point(&rules);
        let other = PlayerId::new(1 - seat.0);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let c = ctx(&rules, &WeightTable::V1, SearchScheme::default(), 1);
        let err = search_world(&world, other, &c, &mut rng).expect_err("wrong seat");
        assert!(err.contains("is to play"), "the refusal must name the mismatch: {err}");
    }
}
