//! **Greedy-v1** — the one-ply evaluator rung (Phase 3.3 T3; spec §3 and §8).
//!
//! One decision = enumerate a bounded CANDIDATE set at the current decision
//! point, apply each candidate inside the agent's own sampled world, `observe`
//! the successor from the agent's own seat, and take the [`eval::score`] argmax.
//! No opponent model, no depth: the continuation is T5's business, and this rung
//! stays as the FLOOR every deeper scheme has to beat (spec L2).
//!
//! # The honest chain is [`PolicyAgent`]'s, verbatim
//!
//! `observe`d view in → own ChaCha8 play stream → a per-decision sampler seed
//! drawn from that stream → [`BeliefSampler::from_view`] (an `Err` is a typed
//! HARD error with **no fallback arm** — the same absence
//! [`crate::game::arena`]'s doc explains) → ONE sampled world → a
//! [`DecisionHandle`] over it → the volume observation FIRST, from that same
//! handle (the L10 bundling: a Movement decision point pays exactly one route
//! enumeration, and [`DecisionReport`] carries the volume so the driver never
//! needs a second handle over the true state) → candidates → argmax → a uniform
//! draw over the tied set. Everything the agent reads is a function of its own
//! view; nothing here can name the true state.
//!
//! # The candidate scheme (spec §3.3)
//!
//! | family | scheme |
//! |---|---|
//! | `MovementRoutes` | ALL, straight off the handle's memo (max OBSERVED 17 per set, R2) |
//! | `MovementRetreat` | NEVER materialized: the zero-recovery anchor + the all-recover anchor + `M` [`retreat_sample`] draws, deduped |
//! | `Placement` | the EMPTY placement anchored + all REALIZED moves when there are ≤ [`PLACEMENT_CANDIDATE_CAP`], else a uniform `cap−1`-subset of the rest |
//! | everything else | full enumeration (bounded-small, delegated families) |
//!
//! **The placement cap's operand is the REALIZED count, never
//! [`placement_upper_len`](crate::game::search_actions::placement_upper_len).**
//! [`FamilySize::UpperBound`](crate::game::search_actions::FamilySize) exists
//! precisely because the two diverge once a fee gate punches a hole in the
//! affordable subsets, and an upper-bound operand mis-fires the fallback arm on
//! a family that would have fitted whole
//! (`placement_candidates_use_the_realized_count_not_the_upper_bound` pins it on
//! a fixture where the bound reads 851 and the realization 484).
//!
//! **`M` (the retreat sample count) is a KNOB**, defaulted at
//! [`DEFAULT_RETREAT_SAMPLES`] = 8 and recorded in provenance. The two anchors
//! are not knobs: they are what makes "retreat nothing" and "retreat
//! everything" always available to the argmax, which is the pair spec §2.5's
//! recovery falsifier measures against — T4 MEASURES the resulting
//! nonzero-recovery pick share, and this module supplies the anchors and the
//! knob it measures. R2 found M ∈ {0, 8, 32} indistinguishable under
//! **eval-v0**, and L6 quarantines that as eval-v0-CONDITIONAL: it is not a
//! reason to drop the draws under v1's weights.
//!
//! # The tie-break (spec §3.2) is load-bearing, not a detail
//!
//! Ties are resolved by a UNIFORM DRAW over the FULL argmax set, from the
//! agent's own stream. R2 measured a 0.67 swing in θ̂ on this choice alone
//! (0.9292 random vs 0.2633 first-wins, same master/N/evaluator/candidates), and
//! the mechanism is structural rather than statistical: `placement_iter` emits
//! the EMPTY placement first, evaluators tie on ~30% of decisions, so
//! first-listed-wins repeatedly plays "place nothing" → no assistants on board →
//! the routes family is empty at every Movement point → an absorbing no-op loop.
//! `a_constant_evaluator_places_assistants_terminates_and_explains_every_move`
//! is that pin, and its doc records both observed outcomes.
//!
//! **Exactly ONE u64 is drawn per decision, whatever the tie count**
//! ([`uniform_index`]), so the stream position after a decision is a function
//! of the world and the candidate scheme — never of the weights. That keeps two
//! weight tables comparable move-for-move up to the first place they genuinely
//! disagree, instead of desynchronising the stream at the first tie.
//!
//! That property is a T3-review FIX, not a restatement: this paragraph
//! previously claimed it of `gen_range(0..tied.len())`, which cannot deliver
//! it. `gen_range` is Lemire rejection sampling — the review measured
//! acceptance at ~0.5–0.7 and a mean of ~2 words consumed, with the rate a
//! function of `tied.len()`, i.e. of how many candidates the WEIGHTS tied. Two
//! tables therefore desynchronised at the first tie of a different width, and
//! the comparability the paragraph promised was never there. Multiply-shift
//! draws one word unconditionally instead.
//!
//! # Cost, measured — and why the per-DECISION number is not R2's
//!
//! Conditions: organic 2P greedy-vs-greedy self-play through
//! [`run_arena_game_capped`](crate::game::arena::run_arena_game_capped) with the
//! same factory the arena uses, per-move invariants OFF, Apple M5, quiet
//! machine, three runs; harness = a scratch binary that path-depends on this
//! crate (out of tree — nothing here is asserted).
//!
//! | table | decisions | µs/decision | candidates/decision | µs/candidate |
//! |---|---|---|---|---|
//! | `ZERO` (release) | 2,013 | 58.5 – 69.8 | 35.08 | 1.67 – 1.99 |
//! | `PROVISIONAL_V1` (release) | 12,000 | 47.4 – 51.4 | 47.25 | 1.00 – 1.09 |
//! | `ZERO` (debug) | 2,013 | 851.3 | 35.08 | 24.27 |
//! | `PROVISIONAL_V1` (debug) | 12,000 | 737.4 | 47.25 | 15.61 |
//!
//! **THE TABLE ABOVE IS SUPERSEDED IN ITS `PROVISIONAL_V1` ROWS and is kept for
//! the ZERO rows and the per-candidate cost.** Those rows were taken under the
//! PRE-SPLIT weight table, which was degenerate: it placed nothing, so every
//! placement family stayed maximal and the candidate mean was inflated by a bug
//! rather than by the scheme (T3 review S2 / spec L14). Post-split re-measure,
//! ONE figure, labelled: 2P Greedy-vs-Greedy self-play under the shipped
//! `PROVISIONAL_V1` at master `0xA3E1_2026_0726_0009` runs **27.91
//! candidates/decision** (7,647 over 274 decisions), printed by
//! `the_shipped_table_plays_a_game_that_places_assistants_and_ends`. The mean is
//! deterministic in `(master, table)`, so it needs no timing conditions — and it
//! is still a PROFILE, not a shipped-cost claim, because T4 re-prices the table
//! and its probe re-measures this alongside the µs figures.
//!
//! **The per-CANDIDATE figure is the comparable one and it agrees with R2**:
//! 1.00–1.99 µs here against R2's isolate composition of 1.359 µs (apply 0.673 +
//! observe 0.609 + eval 0.077). **The per-DECISION figure is NOT comparable to
//! R2's 21.4–28.0 µs and the difference is arithmetic, not a regression**: R2
//! measured 15.31 candidates/decision under eval-v0, and 15.31 × 1.359 + 2.639
//! fixed = 23.5 µs, exactly its own quoted number. This rung at 28–35
//! candidates/decision costs what that arithmetic says it should. The candidate
//! MEAN is a property of the weight table's play, not of the scheme — `ZERO`
//! floods the board (947 assistants placed in one game) and keeps placement
//! families large that way — so **a representative candidate mean still has to
//! wait for T4's frozen table**. Debug runs 12–14×.
//!
//! # What is provisional
//!
//! [`WeightTable::PROVISIONAL_V1`] until T4's freeze — and every weight in
//! [`score_starting_order`]'s own vocabulary is hand-set and UNMEASURED, spec
//! §8's own terms, revisited at 3.5.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::game::apply_move::apply_move_unchecked;
use crate::game::arena::{ArenaAgent, ControllerProvenance, DecisionReport};
use crate::game::belief::BeliefSampler;
use crate::game::eval::{self, Explanation, WeightTable, EVAL_IMPL_VERSION};
use crate::game::metrics::decision_volume;
use crate::game::observe::observe;
use crate::game::rules::Rules;
use crate::game::search_actions::{retreat_move, retreat_sample, DecisionHandle, MoveFamily};
use crate::game::seeds::splitmix64;
use crate::types::{AreaId, Country, GameState, Move, OrderCard, OrderCardId, PlayerId, PlayerView};

// ---------------------------------------------------------------------------
// Identity and knobs
// ---------------------------------------------------------------------------

/// This CONTROLLER's implementation version (spec §0.5 / L12), stamped into
/// `SeatProvenance::version` for every seat Greedy-v1 plays.
///
/// THE BUMP RULE, in the words `SeatProvenance::version` already means: bump on
/// ANY behavior-affecting change to THIS controller — the candidate scheme, the
/// `M` default, the tie-break, or the evaluator weights it embeds — because
/// each of those makes previously-recorded Greedy-v1 figures non-comparable.
/// A weight change bumps [`EVAL_IMPL_VERSION`] AND every controller const that
/// embeds the evaluator, honestly: comparability is the criterion, not
/// authorship. `POLICY_IMPL_VERSION` is a different controller family's const
/// and is untouched by anything here.
pub const GREEDY_IMPL_VERSION: u32 = 1;

/// The stable controller name — provenance, runner grammar and corpus filters
/// all read it from here rather than hand-typing it (the `RolloutPolicy::as_str`
/// discipline, one rung over).
pub const GREEDY_CONTROLLER: &str = "greedy-v1";

/// `M`: how many [`retreat_sample`] draws join the two anchors. Spec §3.3's
/// documented knob.
pub const DEFAULT_RETREAT_SAMPLES: u8 = 8;

/// The placement candidate cap. Bounds the number of successors SCORED (each
/// costs an apply + `observe` + `score`), which is the work the cap exists to
/// bound — enumerating a placement move is ~2 orders of magnitude cheaper than
/// scoring one. R2 measured it as slack at 2P (0 binds in 48,458 decisions, max
/// realized 396, an OBSERVATION under greedy-v0 and not a ceiling) and LIVE at
/// 4P (3,231 realized on the shipped `action_cost` fixture); T4 re-measures the
/// 2P bind count under v1.
pub const PLACEMENT_CANDIDATE_CAP: usize = 512;

// ---------------------------------------------------------------------------
// Candidates
// ---------------------------------------------------------------------------

/// One decision point's candidate set, plus the one thing about HOW it was
/// built that a caller cannot re-derive from the moves.
#[derive(Debug, Clone)]
pub struct Candidates {
    /// The moves to score, in family order.
    pub moves: Vec<Move>,
    /// Whether the placement family's REALIZED count exceeded
    /// [`PLACEMENT_CANDIDATE_CAP`], so the sampling arm ran and the argmax is
    /// over a random subset rather than the whole family.
    ///
    /// Reported rather than inferred because the two arms COINCIDE at the
    /// boundary (the anchored empty placement plus a `cap−1`-reservoir over the
    /// remaining `n − 1 ≤ cap − 1` items yields all of them, in order), which is
    /// a good property — no behavioural cliff — and is exactly what makes the
    /// arm invisible in the output. T4's probe counts these (spec §2.4's 2P
    /// cap-bind figure), and the operand pin below asserts it.
    pub placement_cap_bound: bool,
}

/// Spec §3.3's candidate scheme over one decision point.
///
/// Deterministic in `(handle, retreat_samples, rng stream position)`: the number
/// of draws consumed is a function of the WORLD and the knob, never of any
/// score, so two weight tables walk the same stream.
pub fn candidates(
    handle: &DecisionHandle,
    retreat_samples: u8,
    rng: &mut impl Rng,
) -> Candidates {
    let mut moves: Vec<Move> = Vec::new();
    let mut placement_cap_bound = false;

    for family in handle.families() {
        match family {
            MoveFamily::MovementRetreat { on_board } => {
                // The exploding family (4P worst case 442,368) is served by
                // construction and by draw, never paged and never collected.
                // `retreat_move` drops zero counts, so on an EMPTY board both
                // anchors are the same recover-nothing move and the dedupe
                // collapses them — the family really does have one member there.
                push_unique(&mut moves, retreat_move(on_board, &[]));
                let all: Vec<(AreaId, u8)> = on_board.iter().map(|(&a, &c)| (a, c)).collect();
                push_unique(&mut moves, retreat_move(on_board, &all));
                for _ in 0..retreat_samples {
                    push_unique(&mut moves, retreat_sample(on_board, rng));
                }
            }
            MoveFamily::Placement => {
                // ONE lazy pass, with the EMPTY placement ANCHORED (T3 review
                // S3). `placement_iter` emits it first by contract — it is the
                // `k = 0` combination, cost 0, hence always affordable and
                // always the first thing the Option-A cursor yields — so
                // taking one item off the front is exactly "hold out the null
                // option", and the `debug_assert` below pins that reading.
                //
                // WHY it is held out rather than left in the pool: the
                // sampling arm keeps `cap` of `n` members, so a pooled null
                // option survives with probability `cap/n` and the argmax at a
                // 3,231-member point would be missing "place nothing" ~84% of
                // the time. Retreat already anchors both of ITS nulls (recover
                // nothing, recover everything); this gives placement the same
                // guarantee, and the rule the two arms now share is: the null
                // option is in the argmax set at EVERY family, always.
                //
                // The rest reservoirs at `cap − 1`, so the total stays ≤ cap
                // and every member is still distinct. When the family fits, the
                // reservoir is the whole remainder in enumeration order and the
                // candidate list is the family verbatim — the two arms coincide
                // at the boundary exactly as before.
                let mut rest = handle.iter(family);
                let anchor = rest.next();
                debug_assert!(
                    matches!(
                        anchor,
                        Some(Move::PlaceThreeAreas { ref placements }) if placements.is_empty()
                    ),
                    "placement_iter must emit the empty placement first; got {anchor:?}",
                );
                let (drawn, rest_len) = reservoir(rest, PLACEMENT_CANDIDATE_CAP - 1, rng);
                // The operand is the REALIZED count STRUCTURALLY: this arm has
                // no way to consult `placement_upper_len` even if a future edit
                // wanted to, and the anchor is counted into it so the reported
                // bind is a statement about the WHOLE family.
                let realized = rest_len + usize::from(anchor.is_some());
                placement_cap_bound = realized > PLACEMENT_CANDIDATE_CAP;
                moves.extend(anchor);
                moves.extend(drawn);
            }
            // Routes off the memo; the delegated families are bounded-small.
            //
            // EXHAUSTIVE ON PURPOSE, no `_` arm (the `family_kind` discipline):
            // a new `MoveFamily` variant must be given a candidate scheme HERE
            // or this stops compiling. A wildcard would silently enumerate the
            // next exploding family in full, which is the one failure mode the
            // whole scheme exists to prevent.
            MoveFamily::MovementRoutes
            | MoveFamily::AdditionalAction
            | MoveFamily::AreaAction
            | MoveFamily::FivePower
            | MoveFamily::Building
            | MoveFamily::ChoiceResolution => moves.extend(handle.iter(family)),
        }
    }

    Candidates { moves, placement_cap_bound }
}

/// Append `mv` unless an equal move is already present.
///
/// Linear, and deliberately so: the only caller is the retreat arm, whose set is
/// `2 + M` = 10 moves at the default knob. `Move` is `PartialEq` but neither
/// `Ord` nor `Hash`, so a set-based dedupe would need a key type; at this size
/// that would be machinery around a ten-element scan. The placement arm needs no
/// dedupe at all — [`reservoir`] samples WITHOUT replacement from an enumeration
/// whose members are distinct by construction, and the anchored empty placement
/// is held OUT of that pool rather than added back to it, so it cannot collide
/// with a draw either.
fn push_unique(out: &mut Vec<Move>, mv: Move) {
    if !out.contains(&mv) {
        out.push(mv);
    }
}

/// A size-`cap` uniform sample of `iter` WITHOUT replacement, plus the number of
/// items the iterator actually yielded.
///
/// Classic reservoir: the first `cap` items are kept, and the `i`-th (0-based)
/// thereafter replaces a uniformly chosen slot with probability `cap/(i+1)`. Two
/// properties the caller leans on: when `n ≤ cap` the result is the whole
/// sequence IN ORDER and no randomness is consumed; when `n > cap` exactly
/// `n − cap` draws are consumed and every `cap`-subset is equally likely.
///
/// **Deviation from the brief's literal `handle.sample` loop, recorded (plan
/// L11 discipline).** T3's brief says "take 512 `handle.sample` draws".
/// [`DecisionHandle::sample`] on the placement family is `placement_sample`'s
/// reservoir-of-ONE, which is `O(family)` PER DRAW: 512 of them on the shipped
/// 3,231 fixture is 1.65 M candidate constructions. Measured (release, three
/// runs, same fixture, scratch harness): the 512-draw loop **51.7 / 55.7 / 53.8
/// ms** against this one-pass reservoir's **0.11 / 0.12 / 0.11 ms** — **449–508×**
/// — i.e. three orders of magnitude past spec §3.4's whole-DECISION budget for
/// one family's candidates. Sampling without replacement in one pass is also
/// what makes the brief's OWN acceptance property ("its 512 draws are all
/// DISTINCT") true rather than false: the with-replacement loop was measured
/// carrying **39 duplicates** in its 512 draws, the birthday count for 512 draws
/// from 3,231. Same cap, same uniformity over the realized family, one pass.
fn reservoir(
    iter: impl Iterator<Item = Move>,
    cap: usize,
    rng: &mut impl Rng,
) -> (Vec<Move>, usize) {
    let mut kept: Vec<Move> = Vec::with_capacity(cap.min(64));
    let mut seen = 0usize;
    for mv in iter {
        if seen < cap {
            kept.push(mv);
        } else {
            let j = rng.gen_range(0..=seen);
            if j < cap {
                kept[j] = mv;
            }
        }
        seen += 1;
    }
    (kept, seen)
}

/// A uniform index into `0..len` from EXACTLY ONE u64 — Lemire's multiply-shift
/// with the rejection loop deliberately omitted.
///
/// **Constant consumption is the point** (T3 review M1). `Rng::gen_range` is
/// the same multiply-shift wrapped in a rejection loop that redraws whenever
/// the low word could bias the result: correct, but it consumes a
/// `len`-dependent number of words (measured acceptance ~0.5–0.7, mean ~2), and
/// `len` here is the size of the argmax set — a function of the weights. That
/// made the stream position after a decision weight-dependent, which is exactly
/// what this module's tie-break doc promises it is not.
///
/// **What omitting rejection costs.** The modulo bias is at most
/// `len / 2^64`: with `len` bounded by the candidate set (hundreds, and
/// [`PLACEMENT_CANDIDATE_CAP`]-bounded on its largest family), that is under
/// `2^-55` — no run this project will ever make can distinguish it from
/// uniform. A bias too small to measure is the right trade for a stream
/// property two weight tables have to share.
///
/// Three pins, one per claim: the constant consumption
/// (`the_tie_break_draw_consumes_exactly_one_word_whatever_the_tie_count`, with
/// the replaced `gen_range` measured as `len`-dependent in the same test), the
/// uniformity band (`the_tie_break_draw_is_uniform_over_the_tied_set`), and the
/// end-to-end reach through [`GreedyAgent::choose`]
/// (`the_tie_break_reaches_the_whole_argmax_set`, unchanged by this fix).
///
/// `len` must be nonzero; the only caller has already refused an empty
/// candidate set.
fn uniform_index(draw: u64, len: usize) -> usize {
    debug_assert!(len > 0, "a uniform index over an empty set is undefined");
    ((u128::from(draw) * len as u128) >> 64) as usize
}

// ---------------------------------------------------------------------------
// Scoring one candidate
// ---------------------------------------------------------------------------

/// `world` after `mv`, seen from `seat`. The successor VIEW is what the
/// evaluator is allowed to see — the successor STATE never leaves this function.
///
/// Both errors are HARD (spec §3.1's no-fallback rule reaching one level down):
/// the candidate came off the handle's own families, which are `get_legal_moves`
/// filtered, so a rejection here is an engine or scheme bug and skipping it
/// silently would let a broken candidate arm degrade the rung to "whatever
/// still applies" without a single failing test.
fn successor_view(
    world: &GameState,
    mv: &Move,
    seat: PlayerId,
    rules: &Rules,
) -> Result<PlayerView, String> {
    let next = apply_move_unchecked(world, mv, rules)
        .map_err(|e| format!("{GREEDY_CONTROLLER}: candidate {mv:?} failed to apply: {e}"))?;
    observe(&next, seat).map_err(|e| {
        format!("{GREEDY_CONTROLLER}: cannot observe seat {} after {mv:?}: {e}", seat.0)
    })
}

// ---------------------------------------------------------------------------
// The starting-order Keep (spec §8)
// ---------------------------------------------------------------------------

/// VP weight: the vocabulary's unit. Every other weight below is quoted as a
/// fraction of one printed victory point (10), so the table reads as "how many
/// tenths of a VP is this worth".
const KEEP_VP: i64 = 10;
/// Each required trade good, NEGATIVE: it is a turn of production the order
/// costs before it pays. Printed requirements run 3..=6, so this spans −24..−48
/// against rewards spanning 10..=110 — big enough to separate a cheap 3-VP order
/// from an expensive one, never big enough to outrank a 4-VP gap.
const KEEP_REQUIREMENT: i64 = 8;
/// One Yen (printed 0..=2).
const KEEP_MONEY: i64 = 3;
/// One imported good (printed 0..=1) — scarcer than a Yen and a Port/customs
/// currency, so it prices above one.
const KEEP_IMPORTED_GOOD: i64 = 6;
/// One trade good, whether a named one or an `any_trade_good` pick. **Unexercised
/// by the shipped deck** — all 36 order cards print 0 for every trade-good
/// reward — and kept for totality so a data change cannot silently drop a reward
/// class from the Keep.
const KEEP_TRADE_GOOD: i64 = 4;
/// One movement icon (printed 0 or 2).
const KEEP_MOVEMENT: i64 = 4;
/// One warehouse token moved to hand (printed 0..=1).
const KEEP_WAREHOUSE_TOKEN: i64 = 5;
/// Numerator of the country term, `KEEP_COUNTRY_NUMERATOR / icons_in_play`. At
/// the shipped registries (English 17 · American 15 · French 13 · German 11 ·
/// Dutch 10 icons across orders AND technologies) that prices an icon at 7..12,
/// i.e. between a half and a whole VP — deliberately sub-VP, so the country
/// breaks near-ties rather than overriding printed rewards.
const KEEP_COUNTRY_NUMERATOR: i64 = 120;

/// What one Order card is worth to keep at setup, from its PRINTED economics
/// (spec §8).
///
/// Three groups, all read off the card: the reward side (VP plus the money /
/// imported-good / trade-good / movement / warehouse rewards), the requirement
/// side (negative, per good), and the COUNTRY flag.
///
/// **The country term and the evaluator's country arithmetic.** The end bonus
/// pays 2/4/8/12 VP for a 2/3/4/5-DISTINCT-country set, and its icon sources are
/// the two face-up piles the evaluator's own `country_bonus_now` reads —
/// completed orders and owned technologies. A single card cannot form a set, so
/// what it carries is PROGRESS toward one, and the thing that varies between two
/// dealt cards is how hard that country is to obtain LATER: the term is
/// therefore the ladder's scarcity, `KEEP_COUNTRY_NUMERATOR` over the number of
/// icons of that country in play across both registries. The ladder itself is
/// country-SYMMETRIC, so a term read straight off it would price both dealt
/// cards identically and discriminate nothing; scarcity is the part of the same
/// arithmetic that does.
///
/// **UNMEASURED and hand-set, spec §8's own terms — revisited at 3.5.** Nothing
/// in the research base measures a starting-order policy; these weights are a
/// first cut whose only executable claims are the unit tests below (VP
/// dominates; requirements subtract; the function is a pure function of the
/// card).
pub fn score_starting_order(card: &OrderCard, rules: &Rules) -> i64 {
    let r = &card.rewards;
    let named_goods =
        i64::from(r.copper) + i64::from(r.silk) + i64::from(r.tea) + i64::from(r.seafood);
    KEEP_VP * i64::from(r.victory_points)
        + KEEP_MONEY * i64::from(r.money)
        + KEEP_IMPORTED_GOOD * i64::from(r.imported_goods)
        + KEEP_TRADE_GOOD * (named_goods + i64::from(r.any_trade_good))
        + KEEP_MOVEMENT * i64::from(r.movement)
        + KEEP_WAREHOUSE_TOKEN * i64::from(r.warehouse_tokens)
        + country_scarcity(card.country, rules)
        - KEEP_REQUIREMENT * i64::from(card.requirements.total())
}

/// `KEEP_COUNTRY_NUMERATOR / (icons of `country` across both registries)`, with
/// the denominator floored at 1 so a rules-data change that removes a country
/// cannot divide by zero.
fn country_scarcity(country: Country, rules: &Rules) -> i64 {
    let orders = rules.orders.iter().filter(|c| c.country == country).count();
    let techs = rules.techs.iter().filter(|c| c.country == country).count();
    KEEP_COUNTRY_NUMERATOR / (orders + techs).max(1) as i64
}

// ---------------------------------------------------------------------------
// Provenance identity
// ---------------------------------------------------------------------------

/// A 64-bit digest of both weight vectors, cast to `i64` for the provenance
/// params map (which is `i64`-valued).
///
/// It IDENTIFIES a table; it does not order or describe one. Two records
/// carrying the same `weights` value were produced by the same thirty numbers,
/// which is what a corpus reader needs to know before pooling them —
/// `EVAL_IMPL_VERSION` rides alongside for the coarser "same evaluator
/// implementation" question. Folded with `seeds::splitmix64`, the crate's one
/// mixing primitive, rather than a second hand-rolled constant set.
pub fn weight_table_identity(weights: &WeightTable) -> i64 {
    let mut acc: u64 = 0x3E1D_2026_0726_0003;
    for w in weights.early.iter().chain(weights.late.iter()) {
        let mut step = acc ^ (*w as u64);
        acc = splitmix64(&mut step);
    }
    acc as i64
}

// ---------------------------------------------------------------------------
// The agent
// ---------------------------------------------------------------------------

/// The per-decision counters a Greedy-v1 seat keeps, behind one `Arc` so
/// [`ExplanationTally`] hands out one cheap clone.
#[derive(Debug, Default)]
struct GreedyCounters {
    explained: AtomicU64,
    placement_cap_binds: AtomicU64,
    candidates_scored: AtomicU64,
}

/// A cloneable handle on one [`GreedyAgent`]'s counters — the
/// [`HonestPathTally`](crate::game::arena::HonestPathTally) clone-out
/// precedent, and for the same reason: a game's agents are built by the driver's
/// factory and never handed back, so cloning the handle BEFORE the agent moves
/// into the driver is the only way to read them afterwards.
///
/// It carries THREE counts because all three must survive that move.
/// [`explained`] is spec §0.3(b)'s: every move the bot plays is explained, and
/// the gate asserts this equals the X-seat applied-move count.
/// [`placement_cap_binds`] is spec §3.3's, which T4's probe needs per cell and
/// which is otherwise invisible (the two placement arms coincide at the
/// boundary — see [`Candidates`]). [`candidates_scored`] is what makes a µs
/// figure INTERPRETABLE rather than merely recorded: greedy cost is
/// candidate-bound, so §3.4's numbers are only comparable between two runs at
/// the same candidates-per-decision (R2 quotes 15.31, and the same rung under a
/// different table runs at a different mean; see this module's cost note).
///
/// [`explained`]: ExplanationTally::explained
/// [`placement_cap_binds`]: ExplanationTally::placement_cap_binds
/// [`candidates_scored`]: ExplanationTally::candidates_scored
#[derive(Clone, Default)]
pub struct ExplanationTally {
    counters: Arc<GreedyCounters>,
}

impl ExplanationTally {
    /// Decisions this agent explained — every decision it made, since
    /// [`GreedyAgent::choose`] cannot return without explaining.
    pub fn explained(&self) -> u64 {
        self.counters.explained.load(Ordering::Relaxed)
    }

    /// Decisions whose placement family exceeded [`PLACEMENT_CANDIDATE_CAP`].
    pub fn placement_cap_binds(&self) -> u64 {
        self.counters.placement_cap_binds.load(Ordering::Relaxed)
    }

    /// Successors this agent applied, observed and scored — the argmax pass
    /// only, NOT counting the one re-application the chosen move's explanation
    /// costs (that is exactly one per [`explained`](Self::explained)).
    pub fn candidates_scored(&self) -> u64 {
        self.counters.candidates_scored.load(Ordering::Relaxed)
    }
}

/// Greedy-v1: one-ply argmax of [`eval::score`] over spec §3.3's candidates.
pub struct GreedyAgent {
    weights: WeightTable,
    retreat_samples: u8,
    rng: ChaCha8Rng,
    /// Kept beside the stream it seeded because `ChaCha8Rng` will not hand its
    /// seed back and provenance has to record it (`PolicyAgent`'s reason).
    agent_seed: u64,
    /// Every Order card's Keep score, precomputed at construction.
    ///
    /// [`ArenaAgent::choose_starting_order`] takes no `&Rules` — deliberately,
    /// since a setup decision must be expressible from what the agent
    /// legitimately holds — and `Box<dyn ArenaAgent>` is `'static`, so the agent
    /// cannot borrow one either. A 36-entry table built once from `&Rules` at
    /// construction answers the Keep in two lookups and keeps
    /// [`score_starting_order`] a pure public function T4/T10 can call directly.
    /// An association LIST rather than a map: `OrderCardId` is `Hash` but not
    /// `Ord`, and thirty-six linear compares twice per game is not worth a
    /// hasher (nor a derive on an engine type).
    keep_scores: Vec<(OrderCardId, i64)>,
    counters: ExplanationTally,
    last_explanation: Option<Explanation>,
}

impl GreedyAgent {
    /// Greedy-v1 at the shipped knobs: [`WeightTable::PROVISIONAL_V1`] and
    /// `M` = [`DEFAULT_RETREAT_SAMPLES`]. `agent_seed` is the PLAY stream — key
    /// it with [`agent_stream_seed`](crate::game::arena::agent_stream_seed).
    pub fn new(rules: &Rules, agent_seed: u64) -> Self {
        Self::with_weights(rules, WeightTable::PROVISIONAL_V1, DEFAULT_RETREAT_SAMPLES, agent_seed)
    }

    /// Greedy-v1 with an explicit weight table and `M`. The weight parameter is
    /// what scenario mutations and the §3.2 degeneracy pin are built on
    /// ([`WeightTable::ZERO`]).
    pub fn with_weights(
        rules: &Rules,
        weights: WeightTable,
        retreat_samples: u8,
        agent_seed: u64,
    ) -> Self {
        let keep_scores = rules
            .orders
            .iter()
            .map(|card| (card.id, score_starting_order(card, rules)))
            .collect();
        Self {
            weights,
            retreat_samples,
            rng: ChaCha8Rng::seed_from_u64(agent_seed),
            agent_seed,
            keep_scores,
            counters: ExplanationTally::default(),
            last_explanation: None,
        }
    }

    /// The weights this agent evaluates with.
    pub fn weights(&self) -> &WeightTable {
        &self.weights
    }

    /// A cloneable handle on the counters, readable after the agent has been
    /// moved into the driver.
    pub fn tally(&self) -> ExplanationTally {
        self.counters.clone()
    }

    /// The explanation of the LAST move this agent chose — the chosen
    /// successor's full term-by-term breakdown (spec §0.3(b); T10 renders it,
    /// and the scenario suite asserts against it).
    ///
    /// Not on the clone-out handle: an `Explanation` owns a `Vec`, and the
    /// callers that want it (CLI `suggest`, a scenario) own the agent, while the
    /// caller that cannot (the arena driver) wants the COUNT.
    pub fn last_explanation(&self) -> Option<&Explanation> {
        self.last_explanation.as_ref()
    }
}

impl ArenaAgent for GreedyAgent {
    /// Argmax of [`score_starting_order`] over the two dealt cards, ties to
    /// index 0.
    ///
    /// **No stream draw, deliberately (spec §5.5).** The arena needs a unit's
    /// two seatings to resolve the same deal the same way, which holds exactly
    /// when a seat's Keep is a deterministic function of `(seat, deal)` and not
    /// of WHICH contestant sits there — `PolicyAgent::choose_starting_order`'s
    /// doc is the same rule from the random side, where the fix is a
    /// `(seat, deal)`-keyed stream instead of the slot-keyed play stream.
    /// Greedy-v1's Keep is a function of the DEAL alone, hence a fortiori of
    /// `(seat, deal)`: `seat` is unread, and the parameter stays because the
    /// trait's contract is about what an agent MAY see, not about what this one
    /// happens to use. A draw from `self.rng` here would key the choice by slot
    /// and break the property outright.
    ///
    /// §5.5's mixed-Keep consequence stands and is not this method's to solve: a
    /// cell pitting Greedy-v1 against a `PolicyAgent` is swapped-balanced but
    /// NOT setup-identical, because the two contestants resolve the same deal by
    /// different rules.
    fn choose_starting_order(&mut self, _seat: PlayerId, deal: &[OrderCardId; 2]) -> u8 {
        let score = |id: OrderCardId| {
            self.keep_scores.iter().find(|(k, _)| *k == id).map_or(0, |(_, v)| *v)
        };
        u8::from(score(deal[1]) > score(deal[0]))
    }

    fn choose(&mut self, view: &PlayerView, rules: &Rules) -> Result<DecisionReport, String> {
        let seat = view.viewer;
        // The sampler seed comes from THIS AGENT'S OWN stream and nothing else —
        // deriving it from anything state-flavoured would correlate the sample
        // with the truth it must be blind to.
        let sampler_seed: u64 = self.rng.gen();
        let mut sampler = BeliefSampler::from_view(view, rules, sampler_seed)
            .map_err(|e| format!("belief sampler refused seat {}'s view: {e}", seat.0))?;
        let world = sampler.sample();
        let handle = DecisionHandle::new(&world, rules);

        // Volume FIRST, off the same handle every later read uses (L10).
        let volume = decision_volume(&handle).ok_or_else(|| {
            format!(
                "seat {} was asked to move at {:?}, which offers no families at all",
                seat.0,
                handle.turn_phase(),
            )
        })?;

        let Candidates { moves, placement_cap_bound } =
            candidates(&handle, self.retreat_samples, &mut self.rng);
        if moves.is_empty() {
            return Err(format!(
                "dead end: {GREEDY_CONTROLLER} found no candidate at {:?} for seat {}",
                handle.turn_phase(),
                seat.0,
            ));
        }

        let mut best = i64::MIN;
        let mut tied: Vec<usize> = Vec::new();
        for (i, mv) in moves.iter().enumerate() {
            let value = eval::score(&successor_view(&world, mv, seat, rules)?, rules, &self.weights);
            if value > best {
                best = value;
                tied.clear();
                tied.push(i);
            } else if value == best {
                tied.push(i);
            }
        }

        // Spec §3.2: uniform over the FULL argmax set, off EXACTLY one u64
        // whatever the tie count (see `uniform_index`).
        let chosen = tied[uniform_index(self.rng.next_u64(), tied.len())];
        let mv = moves[chosen].clone();

        // Spec §0.3(b): the chosen move is explained ALWAYS, and there is no
        // path to a `DecisionReport` that skips it. One extra apply+observe
        // (~1.4 µs at R2's isolates, ~6% of a decision) buys the right to hold
        // exactly one successor view instead of all `|candidates|` of them —
        // and the exact-sum identity turns the recomputation into a free
        // cross-check against the score the argmax actually used.
        let explanation = eval::explain(
            &successor_view(&world, &mv, seat, rules)?,
            rules,
            &self.weights,
        );
        debug_assert_eq!(
            explanation.total, best,
            "explain/score disagreed on the chosen successor ({mv:?})",
        );
        self.last_explanation = Some(explanation);
        let counters = &self.counters.counters;
        counters.explained.fetch_add(1, Ordering::Relaxed);
        counters.candidates_scored.fetch_add(moves.len() as u64, Ordering::Relaxed);
        if placement_cap_bound {
            counters.placement_cap_binds.fetch_add(1, Ordering::Relaxed);
        }

        Ok(DecisionReport { mv, volume })
    }

    fn controller(&self) -> ControllerProvenance {
        ControllerProvenance {
            name: GREEDY_CONTROLLER.to_string(),
            version: GREEDY_IMPL_VERSION,
            params: BTreeMap::from([
                ("m".to_string(), i64::from(self.retreat_samples)),
                ("placement_cap".to_string(), PLACEMENT_CANDIDATE_CAP as i64),
                ("eval_impl".to_string(), i64::from(EVAL_IMPL_VERSION)),
                ("weights".to_string(), weight_table_identity(&self.weights)),
            ]),
            agent_seed: Some(self.agent_seed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::arena::{agent_stream_seed, run_arena_game_capped, ArenaGameStats, Slot};
    use crate::game::move_legality::{is_move_legal, PathMode};
    use crate::game::record::RecordedEvent;
    use crate::game::search_actions::tests::{handle_fee_gated_placement_state, handle_movement_state};
    use crate::game::search_actions::{placement_upper_len, retreat_len};
    use crate::game::seeds::WorldSeed;
    use crate::types::{
        AchievementCard, AchievementType, AreaConfig, AreaType, BoardLayout, ChurchTrack,
        CustomsTrack, GameConfig, LabState, MercenarySupply, PlayerColor, PlayerState, PortState,
        PresidentLocation, TechEffect, TurnPhase,
    };

    /// The committed diagnostic master for this file's driven games. Diagnostic
    /// BY NAME (spec §3's gate-cell exemption): a per-edit pin needs a fixed
    /// master, and these are pins rather than bot-facing evaluation runs.
    const MASTER: u64 = 0xA3E1_2026_0726_0009;

    // -- fixtures -------------------------------------------------------------

    /// A 4P board of 17 placement-allowing areas with minimal adjacency, the
    /// shape `examples/action_cost.rs` uses so the recorded family sizes here
    /// are the ones that example prints.
    fn wide_4p_state(phase: TurnPhase) -> GameState {
        let colors = [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Yellow, PlayerColor::Green];
        let mut areas = BTreeMap::new();
        for i in 0..17u8 {
            areas.insert(
                AreaId::new(i),
                AreaConfig { area_type: AreaType::CopperMine, grid_position: (0, i) },
            );
        }
        let mut adjacency = BTreeMap::new();
        adjacency.insert(AreaId::new(0), vec![AreaId::new(1)]);
        adjacency.insert(AreaId::new(1), vec![AreaId::new(0)]);

        GameState {
            config: GameConfig::for_player_count(4),
            board_layout: BoardLayout {
                areas,
                adjacency,
                canal_crossings: vec![],
                building_sites: BTreeMap::new(),
                five_power_tokens: BTreeMap::new(),
            },
            players: (0..4)
                .map(|i| PlayerState::new(PlayerId::new(i), colors[i as usize], i == 0))
                .collect(),
            current_player: PlayerId::new(0),
            turn_phase: phase,
            turn_number: 1,
            additional_action_state: Default::default(),
            current_action_power: None,
            is_mercenary_action: false,
            mercenary_target_area: None,
            mercenary_return_phase: None,
            president_start_area: None,
            port_state: PortState::new(false),
            lab_state: LabState::new(false),
            order_deck: Vec::new(),
            tech_deck: Vec::new(),
            orders_removed: Vec::new(),
            church_track: ChurchTrack::new(),
            customs_track: CustomsTrack::new(),
            achievements: [
                AchievementCard::new(AchievementType::Copper5),
                AchievementCard::new(AchievementType::Money10),
                AchievementCard::new(AchievementType::Production4),
            ],
            mercenary_supply: MercenarySupply::new(),
            station_location: None,
            end_game_triggered: false,
            trigger_reason: None,
            final_round: false,
            trigger_player: None,
            pending_choices: Vec::new(),
        }
    }

    /// `examples/action_cost.rs`'s 4P retreat worst case: ids 0..3 at count 2,
    /// 3..17 at count 1, so `∏(count+1) = 3³ · 2¹⁴ = 442,368`.
    fn worst_case_movement_state() -> GameState {
        let mut s = wide_4p_state(TurnPhase::Movement);
        let mut on_board = BTreeMap::new();
        for a in 0..3u8 {
            on_board.insert(AreaId::new(a), 2);
        }
        for a in 3..17u8 {
            on_board.insert(AreaId::new(a), 1);
        }
        s.players[0].president_location = PresidentLocation::OnBoard(AreaId::new(0));
        s.players[0].assistants.on_board = on_board;
        s
    }

    /// `examples/action_cost.rs`'s 3,231-combination placement family: the same
    /// wide board, President in hand, four assistants in hand plus Newspaper
    /// (so `k_max = 4`), and money enough that NOTHING is fee-gated.
    fn newspaper_placement_state(rules: &Rules) -> GameState {
        let mut s = wide_4p_state(TurnPhase::Placement);
        s.players[0].president_location = PresidentLocation::InHand;
        s.players[0].assistants.in_hand = 4;
        s.players[0].money = 1_000;
        let newspaper =
            rules.techs.iter().find(|t| t.effect == TechEffect::Newspaper).expect("Newspaper").id;
        s.players[0].technologies = vec![newspaper];
        s
    }

    /// The same wide board with a fee gate that punches the realized count BELOW
    /// the cap while the UPPER BOUND stays above it — the fixture the operand
    /// pin needs. Three assistants in hand (`k_max = 3`), ZERO money, and the
    /// three opponents' Presidents parked on areas 0/1/2, so every subset
    /// touching one of those three areas costs a Yen the player does not have.
    fn cap_straddling_placement_state() -> GameState {
        let mut s = wide_4p_state(TurnPhase::Placement);
        s.players[0].president_location = PresidentLocation::InHand;
        s.players[0].assistants.in_hand = 3;
        s.players[0].money = 0;
        for i in 1..4u8 {
            s.players[i as usize].president_location =
                PresidentLocation::OnBoard(AreaId::new(i - 1));
        }
        s
    }

    fn seeded_rng(n: u64) -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(n)
    }

    /// Drive one 2P game with `weights` at BOTH seats, capped well below
    /// `HONEST_MAX_TURNS` so a non-terminating tie-break fails in seconds rather
    /// than in minutes. Returns the per-seat tally handles cloned out BEFORE the
    /// agents were moved into the driver.
    fn greedy_selfplay_2p(
        weights: WeightTable,
        seed: u64,
        max_turns: u32,
    ) -> Result<(ArenaGameStats, Vec<ExplanationTally>), String> {
        let rules = Rules::standard();
        let world = WorldSeed(seed);
        let tallies: Arc<std::sync::Mutex<Vec<ExplanationTally>>> = Arc::default();
        let sink = Arc::clone(&tallies);
        let mut make = |seat: PlayerId, _n: u8| -> Box<dyn ArenaAgent> {
            let agent = GreedyAgent::with_weights(
                &rules,
                weights,
                DEFAULT_RETREAT_SAMPLES,
                agent_stream_seed(world, seat, Slot(seat.0)),
            );
            sink.lock().unwrap().push(agent.tally());
            Box::new(agent)
        };
        let stats = run_arena_game_capped(world, 2, &rules, &mut make, false, max_turns)
            .map_err(|e| e.to_string())?;
        let handles = tallies.lock().unwrap().clone();
        Ok((stats, handles))
    }

    /// Assistants a game actually put on the board, counted off the record's
    /// applied moves (not off a final state, where recovery may have taken them
    /// back).
    fn assistants_placed(stats: &ArenaGameStats) -> usize {
        stats
            .record
            .events
            .iter()
            .map(|e| match e {
                RecordedEvent::Move(Move::PlaceThreeAreas { placements }) => placements.len(),
                RecordedEvent::Move(Move::PlaceTwoSameArea { adjacent_bonus, .. }) => {
                    2 + usize::from(adjacent_bonus.is_some())
                }
                _ => 0,
            })
            .sum()
    }

    // -- the candidate scheme (spec §3.3) -------------------------------------

    /// The retreat family is served by CONSTRUCTION and by DRAW, never paged:
    /// at the 442,368-member worst case the whole candidate set stays in the
    /// low tens. A materializing scheme would report 442,369 here (the product
    /// plus the one route), which is exactly the ceiling the factored source
    /// exists to keep off this path.
    #[test]
    fn retreat_candidates_are_two_anchors_plus_m_draws_never_the_product() {
        let rules = Rules::standard();
        let state = worst_case_movement_state();
        let family_size = retreat_len(&state.players[0].assistants.on_board);
        assert_eq!(family_size, 442_368, "anti-vacuity: the fixture must be the worst case");

        let handle = DecisionHandle::new(&state, &rules);
        let routes = handle.iter(&MoveFamily::MovementRoutes).count();
        let mut rng = seeded_rng(7);
        let c = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut rng);

        let retreats: Vec<&Move> = c
            .moves
            .iter()
            .filter(|m| matches!(m, Move::ReturnPresidentToHand { .. }))
            .collect();
        assert!(
            retreats.len() <= 2 + usize::from(DEFAULT_RETREAT_SAMPLES),
            "retreat candidates must be bounded by the anchors plus M, got {}",
            retreats.len(),
        );
        assert_eq!(c.moves.len(), routes + retreats.len(), "every candidate is route or retreat");

        // Both anchors, by value.
        let zero = Move::ReturnPresidentToHand { recover: BTreeMap::new() };
        let all = Move::ReturnPresidentToHand {
            recover: state.players[0].assistants.on_board.clone(),
        };
        assert!(retreats.contains(&&zero), "the zero-recovery anchor must be a candidate");
        assert!(retreats.contains(&&all), "the all-recover anchor must be a candidate");
    }

    /// Anchors and draws are deduped: on a one-assistant board the retreat
    /// family has exactly two members, and 2 anchors + 8 draws must still
    /// produce two candidates, not ten.
    #[test]
    fn retreat_anchors_and_draws_are_deduped() {
        let rules = Rules::standard();
        let state = handle_movement_state();
        let on_board = &state.players[0].assistants.on_board;
        assert_eq!(retreat_len(on_board), 3, "fixture: one area holding two assistants");

        let handle = DecisionHandle::new(&state, &rules);
        let mut rng = seeded_rng(11);
        let c = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut rng);
        let retreats: Vec<&Move> = c
            .moves
            .iter()
            .filter(|m| matches!(m, Move::ReturnPresidentToHand { .. }))
            .collect();
        // Exactly the family, once each: 2 anchors + 8 draws must collapse to 3
        // (an undeduped scheme yields 10, and a scheme that only ever emitted the
        // anchors would yield 2 — so this one number pins both directions).
        assert_eq!(
            retreats.len(),
            3,
            "2 anchors + {DEFAULT_RETREAT_SAMPLES} draws over a 3-member family must dedupe to 3",
        );
        for (i, a) in retreats.iter().enumerate() {
            for b in &retreats[i + 1..] {
                assert_ne!(a, b, "candidates must be distinct");
            }
        }
    }

    /// **THE OPERAND PIN (spec §3.3).** On a fee-gated fixture the placement
    /// family's UPPER BOUND is above the cap while its REALIZED count is below
    /// it. The realized operand therefore takes the enumerate-all arm and the
    /// candidate list is the whole realized family, in order; an implementation
    /// reading `placement_upper_len` (or `DecisionHandle::size`, which reports
    /// it as `UpperBound`) would take the sampling arm and flip
    /// `placement_cap_bound`.
    ///
    /// Anti-vacuity is explicit and asserted, not assumed: the bound must
    /// straddle the cap, or the test proves nothing.
    #[test]
    fn placement_candidates_use_the_realized_count_not_the_upper_bound() {
        let rules = Rules::standard();
        let state = cap_straddling_placement_state();
        let upper = placement_upper_len(&state, &rules);
        let realized: Vec<Move> = crate::game::search_actions::placement_iter(&state, &rules).collect();

        assert!(
            upper > PLACEMENT_CANDIDATE_CAP,
            "anti-vacuity: the upper bound ({upper}) must exceed the cap",
        );
        assert!(
            realized.len() <= PLACEMENT_CANDIDATE_CAP,
            "anti-vacuity: the realized count ({}) must fit under the cap",
            realized.len(),
        );

        let handle = DecisionHandle::new(&state, &rules);
        let mut rng = seeded_rng(23);
        let c = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut rng);
        assert!(
            !c.placement_cap_bound,
            "the realized count fits, so the cap must NOT bind (upper {upper}, realized {})",
            realized.len(),
        );
        assert_eq!(c.moves, realized, "the candidate set is the realized family, in order");
    }

    /// **4P FALLBACK COVERAGE (spec §3.3's "at 4P the fallback is the live
    /// arm").** On the shipped `action_cost` 3,231-combination fixture the
    /// sampling arm really runs, and what it produces is the anchored empty
    /// placement plus a uniform `cap−1`-SUBSET of the rest: exactly 512 moves,
    /// all distinct, all legal placements, and demonstrably not the
    /// enumeration's first 512.
    ///
    /// The ANCHOR is asserted here and not merely at the small-family boundary
    /// (T3 review S3), because this is the only cell where it can fail: pooled
    /// with the other 3,230 members the null option would survive with
    /// probability 512/3,231 ≈ 0.16, so a passing test on a family that FITS
    /// would say nothing at all about the arm that drops it.
    #[test]
    fn placement_sampling_arm_draws_512_distinct_legal_moves_at_4p() {
        let rules = Rules::standard();
        let state = newspaper_placement_state(&rules);
        let handle = DecisionHandle::new(&state, &rules);

        let realized = handle.iter(&MoveFamily::Placement).count();
        assert_eq!(realized, 3_231, "the shipped Newspaper fixture's realized family");
        assert!(realized > PLACEMENT_CANDIDATE_CAP, "anti-vacuity: the cap must bind here");

        let mut rng = seeded_rng(101);
        let c = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut rng);
        assert!(c.placement_cap_bound, "the cap must bind and be reported");
        assert_eq!(c.moves.len(), PLACEMENT_CANDIDATE_CAP, "the sample is exactly cap-sized");

        // THE ANCHOR: the always-legal null option survives the sampling arm,
        // and survives it at the FRONT — the same guarantee retreat's zero and
        // all-recover anchors give at their own family.
        let empty = Move::PlaceThreeAreas { placements: Vec::new() };
        assert_eq!(
            c.moves.first(),
            Some(&empty),
            "the empty placement must be anchored, not left to a 512/3231 draw",
        );

        // Distinct, and legal in the state they were drawn from.
        for (i, a) in c.moves.iter().enumerate() {
            assert!(
                matches!(a, Move::PlaceThreeAreas { .. } | Move::PlaceTwoSameArea { .. }),
                "a placement point yields only placement candidates, got {a:?}",
            );
            assert!(
                is_move_legal(&state, a, &rules, PathMode::Canonical),
                "drawn candidate {a:?} must be legal in the state it came from",
            );
            for b in &c.moves[i + 1..] {
                assert_ne!(a, b, "reservoir sampling is WITHOUT replacement");
            }
        }

        // Distribution sanity: a degenerate sampler would return one combination
        // repeated, or the enumeration's prefix. Neither is what this is.
        let prefix: Vec<Move> = handle.iter(&MoveFamily::Placement).take(PLACEMENT_CANDIDATE_CAP).collect();
        assert_ne!(c.moves, prefix, "a cap-subset must not be the enumeration prefix");
        let sizes: std::collections::BTreeSet<usize> = c
            .moves
            .iter()
            .filter_map(|m| match m {
                Move::PlaceThreeAreas { placements } => Some(placements.len()),
                _ => None,
            })
            .collect();
        assert!(sizes.len() >= 3, "the draw must span several subset sizes, saw {sizes:?}");
    }

    /// A fee-gated family whose members are NOT contiguous still yields only
    /// affordable candidates — the realized enumeration is the authority, and
    /// the candidate arm never widens it.
    #[test]
    fn placement_candidates_never_include_an_unaffordable_combination() {
        let rules = Rules::standard();
        let state = handle_fee_gated_placement_state();
        let upper = placement_upper_len(&state, &rules);
        let handle = DecisionHandle::new(&state, &rules);
        let realized = handle.iter(&MoveFamily::Placement).count();
        assert!(upper > realized, "anti-vacuity: the fee gate must punch a hole ({upper} vs {realized})");

        let mut rng = seeded_rng(5);
        let c = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut rng);
        for mv in &c.moves {
            assert!(
                is_move_legal(&state, mv, &rules, PathMode::Canonical),
                "candidate {mv:?} is not legal at the fee-gated state",
            );
        }
    }

    /// Delegated families come through whole.
    #[test]
    fn delegated_families_are_enumerated_in_full() {
        let rules = Rules::standard();
        let mut state = handle_movement_state();
        state.turn_phase = TurnPhase::Building;
        let handle = DecisionHandle::new(&state, &rules);
        let all: Vec<Move> = handle.iter(&MoveFamily::Building).collect();
        assert!(!all.is_empty(), "anti-vacuity: the Building family must be non-empty");

        let mut rng = seeded_rng(3);
        let c = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut rng);
        assert_eq!(c.moves, all);
        assert!(!c.placement_cap_bound);
    }

    // -- the tie-break (spec §3.2) -------------------------------------------

    /// **THE CONSTANT-CONSUMPTION PROPERTY (T3 review M1), made executable.**
    ///
    /// The stream position after a tie-break must not depend on how many
    /// candidates tied, because that count is a function of the WEIGHTS — and
    /// this module's doc has always promised two weight tables walk the same
    /// stream up to the first place they genuinely disagree. The promise is
    /// checked by reading the NEXT word off the stream after the draw: it is
    /// the same word for every tie count exactly when the same number of words
    /// was spent.
    ///
    /// The anti-vacuity half is the review's own measurement, in place: the
    /// `gen_range` this replaced genuinely IS `len`-dependent (Lemire
    /// rejection), so if the second assertion ever went quiet the first would be
    /// pinning nothing.
    #[test]
    fn the_tie_break_draw_consumes_exactly_one_word_whatever_the_tie_count() {
        let next_word_after_shift = |len: usize| -> u64 {
            let mut rng = seeded_rng(0xB1A5);
            let i = uniform_index(rng.next_u64(), len);
            assert!(i < len, "index {i} is out of range for a tie set of {len}");
            rng.next_u64()
        };
        let baseline = next_word_after_shift(1);
        for len in [2usize, 3, 7, 17, 186, 512, 3_231] {
            assert_eq!(
                next_word_after_shift(len),
                baseline,
                "a tie set of {len} moved the stream differently than a lone argmax",
            );
        }

        // ANTI-VACUITY: the mechanism this replaced does not have the property.
        let next_word_after_gen_range = |len: usize| -> u64 {
            let mut rng = seeded_rng(0xB1A5);
            let _ = rng.gen_range(0..len);
            rng.next_u64()
        };
        assert_ne!(
            next_word_after_gen_range(3_231),
            next_word_after_gen_range(1),
            "anti-vacuity: if gen_range already consumed a constant number of words \
             there would have been nothing to fix",
        );
    }

    /// The draw is uniform over the argmax set — the distributional half of the
    /// tie-break, at the small `len` where a bias would be visible.
    ///
    /// A ±10% band over 60k seeded draws, the same shape
    /// `retreat_sample_is_uniform_over_the_product_family` uses one module over.
    /// Multiply-shift's modulo bias here is `len / 2^64 ≈ 3 · 10^-19`, which no
    /// band this side of the heat death would resolve; what the band actually
    /// kills is a mis-written index (an off-by-one that starves slot 0 or `len −
    /// 1`, or a shift that collapses the range), which is the failure mode
    /// omitting the rejection loop could plausibly introduce.
    #[test]
    fn the_tie_break_draw_is_uniform_over_the_tied_set() {
        const DRAWS: usize = 60_000;
        const LEN: usize = 6;
        let mut rng = seeded_rng(20_260_726);
        let mut counts = [0usize; LEN];
        for _ in 0..DRAWS {
            counts[uniform_index(rng.next_u64(), LEN)] += 1;
        }
        let expected = DRAWS / LEN;
        for (i, n) in counts.iter().enumerate() {
            assert!(
                (expected * 9 / 10..=expected * 11 / 10).contains(n),
                "slot {i} drawn {n} times, outside ±10% of {expected} — not uniform: {counts:?}",
            );
        }
        // The endpoints specifically: `0` and `len − 1` are what a shift error
        // starves, and they are inside the band above only if they were reached.
        assert!(counts[0] > 0 && counts[LEN - 1] > 0, "an endpoint was never drawn: {counts:?}");
    }

    /// **THE DEGENERACY PIN (spec §3.2).** A CONSTANT evaluator ties every
    /// candidate, so the tie-break is the only thing choosing — and a game under
    /// it must still put assistants on the board and finish.
    ///
    /// `WeightTable::ZERO` is constant on every POSITIONAL view; a terminal
    /// successor still scores its win/loss ladder (§2.6 short-circuits before
    /// the weights), which is correct and does not weaken the pin — terminal
    /// successors are ~1 in 8,000 decisions.
    ///
    /// **MUTATION-PROVEN AT AUTHORING** by swapping the uniform draw for
    /// first-listed-wins (`let chosen = tied[0];`) and reverting (the file was
    /// diffed byte-identical against its pre-mutation copy afterwards). Both
    /// observations, this exact test, same seed and cap:
    ///
    /// ```text
    /// shipped uniform draw : turns 382, assistants placed 947 -> PASS
    /// first-listed-wins    : "turn cap reached at turn 1501"  -> FAIL
    /// ```
    ///
    /// THREE tests catch the mutation, and the other two print the MECHANISM
    /// rather than merely detecting it:
    ///
    /// ```text
    /// the_tie_break_reaches_the_whole_argmax_set:
    ///   a uniform draw over 186 tied candidates must spread over 60 asks,
    ///   reached only 1
    /// same_seed_agrees_move_for_move_and_a_different_seed_can_diverge:
    ///   ... both seeds' move lists shown, every placement reading
    ///   PlaceThreeAreas { placements: [] }
    /// ```
    ///
    /// That is R2's chain verbatim — `placement_iter` emits the empty placement
    /// FIRST, a flat evaluator ties everything, first-wins plays "place nothing"
    /// every turn, no assistant ever reaches the board, the routes family is
    /// empty at every Movement point, and the game never ends.
    ///
    /// **The cap is MEASURED, not guessed.** Eight consecutive masters
    /// (`MASTER ^ k·0x11`, k = 0..8) all complete under the shipped draw, at
    /// turns 382 / 632 / 514 / 538 / 124 / 182 / 594 / 400 and 273..1,579
    /// assistants placed; 1,500 sits ~2.4× above the largest observed. It is a
    /// TEST budget, not a claimed bound — `HONEST_MAX_TURNS` (10,000) remains
    /// the engine's, and a cap this test can reach in ~2 s debug is what makes
    /// a non-terminating tie-break fail in seconds instead of minutes.
    ///
    /// **§0.3(b)'s tally identity rides the same game deliberately:** a driven
    /// game is the expensive per-edit resource here (~1.9 s debug), and "every
    /// applied move produced an explanation" is a statement about exactly this
    /// run, read off handles cloned out BEFORE the agents were consumed by the
    /// driver. The gate asserts the same identity per SEAT over the promotion
    /// cell.
    #[test]
    fn a_constant_evaluator_places_assistants_terminates_and_explains_every_move() {
        const CAP: u32 = 1_500;
        let (stats, tallies) = greedy_selfplay_2p(WeightTable::ZERO, MASTER, CAP)
            .expect("a constant-evaluator game must complete, not hit the cap");

        let placed = assistants_placed(&stats);
        assert!(
            placed > 0,
            "a constant evaluator must still place assistants (placed {placed} over {} turns)",
            stats.turns,
        );
        assert!(stats.turns < CAP, "the game must end inside the cap, took {}", stats.turns);

        assert_eq!(tallies.len(), 2, "one clone-out handle per seat");
        let explained: u64 = tallies.iter().map(|t| t.explained()).sum();
        assert!(explained > 0, "anti-vacuity: the game must have made decisions");
        assert_eq!(
            explained, stats.moves_applied,
            "every applied move was chosen by a decision that produced an explanation",
        );

        let run = stats.record.run.as_ref().expect("a driven game stamps provenance");
        assert_eq!(run.seats[0].controller, GREEDY_CONTROLLER, "provenance names the controller");
        assert_eq!(run.seats[0].version, GREEDY_IMPL_VERSION, "and its version");
    }

    /// The tie-break draws over the FULL argmax set, not over a prefix of it.
    ///
    /// Exercised through [`GreedyAgent::choose`] itself rather than simulated:
    /// a constant evaluator ties every candidate at an organic placement point,
    /// so re-asking the SAME agent (whose stream advances each time) must walk
    /// most of the set. A first-wins tie-break — or any draw over a truncation —
    /// collapses this to one distinct answer. The floor is deliberately loose
    /// (a quarter of the candidates over `ASKS` asks): the point is "many, and
    /// spread", not a distributional claim, which is `arena_gate`'s business.
    #[test]
    fn the_tie_break_reaches_the_whole_argmax_set() {
        const ASKS: usize = 60;
        let rules = Rules::standard();
        let (state, seat) = first_placement_point(&rules, MASTER);
        let view = observe(&state, seat).expect("observe");

        let mut agent =
            GreedyAgent::with_weights(&rules, WeightTable::ZERO, DEFAULT_RETREAT_SAMPLES, 0x5EED);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..ASKS {
            let report = agent.choose(&view, &rules).expect("a constant evaluator still answers");
            seen.insert(format!("{:?}", report.mv));
        }
        let candidate_count = agent.tally().candidates_scored() / ASKS as u64;
        assert!(
            candidate_count > 20,
            "anti-vacuity: the point must offer a wide argmax, saw {candidate_count}",
        );
        assert!(
            seen.len() > ASKS / 4,
            "a uniform draw over {candidate_count} tied candidates must spread over {ASKS} \
             asks, reached only {}",
            seen.len(),
        );
    }

    // -- determinism ----------------------------------------------------------

    /// Same seed ⇒ same moves over the same view sequence; a different seed CAN
    /// differ, and the divergence is EXHIBITED rather than asserted in the
    /// abstract.
    ///
    /// The divergence half runs under [`WeightTable::ZERO`] on purpose. The
    /// tie-break is the only place a seed can change an answer, so the property
    /// is only observable where ties exist — and under a table with a strict
    /// argmax it is not: two agent seeds measurably agree move-for-move over the
    /// first 12 decisions of both worlds under
    /// [`WeightTable::PROVISIONAL_V1`] (that run is the same-seed half's
    /// fixture, and its agreement across DIFFERENT seeds was observed while
    /// writing this test — the brief's "constructed tie" alternative, taken).
    /// A constant evaluator ties everything, so every decision is a draw and the
    /// two streams part immediately.
    #[test]
    fn same_seed_agrees_move_for_move_and_a_different_seed_can_diverge() {
        let rules = Rules::standard();
        let worlds = [MASTER, MASTER ^ 0xFFFF_FFFF];

        let play = |weights: WeightTable, agent_seed: u64| -> Vec<String> {
            let mut out = Vec::new();
            for &world in &worlds {
                let mut agent =
                    GreedyAgent::with_weights(&rules, weights, DEFAULT_RETREAT_SAMPLES, agent_seed);
                out.extend(first_decisions(&mut agent, world, 12, &rules));
            }
            out
        };

        // Same seed, twice, under the shipped table: bit-identical.
        let a = play(WeightTable::PROVISIONAL_V1, 0xA1);
        let b = play(WeightTable::PROVISIONAL_V1, 0xA1);
        assert_eq!(a.len(), 24, "the probe must reach twelve decisions in each world");
        assert_eq!(a, b, "the same agent seed must reproduce move for move");

        // Same seed, twice, with every decision a tie: still bit-identical.
        let c = play(WeightTable::ZERO, 0xA1);
        assert_eq!(c, play(WeightTable::ZERO, 0xA1), "determinism holds through the tie-break");

        // Different seed, with the draw live: parts company.
        assert_ne!(
            c,
            play(WeightTable::ZERO, 0xB2),
            "with the tie-break live, a different agent seed must diverge",
        );
    }

    /// The first `n` decisions `agent` makes driving BOTH seats of a fresh 2P
    /// game at `world_seed`, as debug strings.
    ///
    /// One agent for both seats is deliberate: the property under test is that
    /// an agent's stream determines its answers, and driving every decision
    /// through the one stream is the sharpest version of it. Setup keeps card 0
    /// at both seats so the state sequence is a function of the world seed
    /// alone.
    fn first_decisions(
        agent: &mut GreedyAgent,
        world_seed: u64,
        n: usize,
        rules: &Rules,
    ) -> Vec<String> {
        use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
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

    // -- the one-ply placement/recovery calculus (the T3 finding, discharged) --

    /// **THE T3 DEGENERACY, DISCHARGED.** Both a placement and a recovery pay at
    /// one ply under [`WeightTable::PROVISIONAL_V1`] — which a linear in-hand
    /// term made impossible, and which `eval`'s piecewise hand split fixed.
    ///
    /// # The finding this replaces (kept as history, spec ledger L14)
    ///
    /// At ONE PLY a placement moves `k` assistants from hand to board and does
    /// nothing else the positional vector can see — the area action a placement
    /// buys happens in a LATER phase, past this rung's horizon. With a SINGLE
    /// linear `assistants_in_hand` weight a placement's sign was therefore
    /// exactly `k · (w_on_board − w_in_hand)`, and the pre-split table priced
    /// in-hand **8** against on-board **7** so that a RETREAT could ever be
    /// worth taking (§2.5). Those are the same inequality. Observed at the time
    /// (master `0xA3E1_2026_0726_0009`, first organic 2P placement point, seat
    /// 0, 186 candidates): the empty placement scored 0 and EVERY single-area
    /// placement scored −1; a driven 2P self-play game returned
    /// `CapReached { turns: 401 }` having placed 0 assistants — R2's absorbing
    /// loop arriving from the WEIGHTS rather than from the tie-break. The
    /// recorded test's second assertion was written to FAIL once the table was
    /// re-priced; this is that re-pointing.
    ///
    /// # What is asserted now
    ///
    /// The four one-ply cells the split exists to make simultaneously true, on
    /// DOCTORED VIEW PAIRS built from a real observed view (only the viewer's
    /// `in_hand`/`on_board` move, so every other term — including the opponent
    /// each feature selects — is identical across a pair and the difference in
    /// `score` is exactly the assistant arithmetic):
    ///
    /// | cell | hand before → after | pays? | inequality |
    /// |---|---|---|---|
    /// | place 3 from a hand of 5 | 5 → 2 | YES | `w_board > w_excess` |
    /// | place 2 from a hand of 2 | 2 → 0 | NO | `w_core > w_board` |
    /// | recover 2 into an empty hand | 0 → 2 | YES | `w_core > w_board` |
    /// | recover 2 past the reserve | 2 → 4 | NO | `w_board > w_excess` |
    ///
    /// Two inequalities, four cells: rows 2 and 3 are one inequality read in
    /// each direction, and so are rows 1 and 4. They are spelled out separately
    /// anyway because the FOUR SCENARIOS are the claim — the split's content is
    /// that all four hold at once, which no single linear weight can arrange.
    ///
    /// The DRIVEN half is the other side of the same discharge: the same 2P
    /// self-play that returned `CapReached` with 0 assistants placed now
    /// finishes inside the cap and places assistants.
    #[test]
    fn the_hand_split_makes_placement_and_recovery_both_payable_at_one_ply() {
        let rules = Rules::standard();
        let (state, seat) = first_placement_point(&rules, MASTER);
        let view = observe(&state, seat).expect("observe");
        let area = AreaId::new(0);

        // One doctored view, with the viewer's assistants set to (in_hand,
        // on_board). Nothing else is touched, so any two of these differ ONLY
        // in the assistant terms.
        let doctored = |in_hand: u8, on_board: u8| -> PlayerView {
            let mut v = view.clone();
            let me = v.players.iter_mut().find(|p| p.id == seat).expect("the viewer's own row");
            me.assistants.in_hand = in_hand;
            me.assistants.on_board = BTreeMap::new();
            if on_board > 0 {
                me.assistants.on_board.insert(area, on_board);
            }
            v
        };
        let at = |in_hand: u8, on_board: u8| -> i64 {
            eval::score(&doctored(in_hand, on_board), &rules, &WeightTable::PROVISIONAL_V1)
        };

        // Anti-vacuity: the doctoring must actually move the score, or the four
        // comparisons below are comparisons between two identical numbers.
        assert_ne!(at(5, 0), at(2, 3), "the doctored pairs must differ at all");

        // (1) Placing 3 from a hand of 5 spends EXCESS and pays.
        assert!(
            at(2, 3) > at(5, 0),
            "placing from excess must pay at one ply ({} -> {})",
            at(5, 0),
            at(2, 3),
        );
        // (2) Placing 2 from a hand of 2 strips the RESERVE and does not.
        assert!(
            at(0, 2) < at(2, 0),
            "placing the last of the reserve must not pay ({} -> {})",
            at(2, 0),
            at(0, 2),
        );
        // (3) Recovering 2 into an EMPTY hand regains the reserve and pays.
        assert!(
            at(2, 0) > at(0, 2),
            "recovering into an empty hand must pay ({} -> {})",
            at(0, 2),
            at(2, 0),
        );
        // (4) Recovering 2 more on top of a full reserve does not.
        assert!(
            at(4, 0) < at(2, 2),
            "over-recovering past the reserve must not pay ({} -> {})",
            at(2, 2),
            at(4, 0),
        );

        // And the same four, END TO END through a real successor: the empty
        // placement is no longer the one-ply argmax at an organic point.
        let world = BeliefSampler::from_view(&view, &rules, 1).expect("sampler").sample();
        let handle = DecisionHandle::new(&world, &rules);
        let mut rng = seeded_rng(5);
        let moves = candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut rng).moves;
        let empty = Move::PlaceThreeAreas { placements: Vec::new() };
        assert!(moves.contains(&empty), "the empty placement is anchored into every candidate set");
        let scored = |mv: &Move| {
            eval::score(
                &successor_view(&world, mv, seat, &rules).expect("succ"),
                &rules,
                &WeightTable::PROVISIONAL_V1,
            )
        };
        let best_real = moves
            .iter()
            .filter(|m| !matches!(m, Move::PlaceThreeAreas { placements } if placements.is_empty()))
            .map(&scored)
            .max()
            .expect("anti-vacuity: the point must offer a real placement");
        assert!(
            best_real > scored(&empty),
            "some real placement must outscore doing nothing (best real {best_real}, empty {})",
            scored(&empty),
        );
    }

    /// The DRIVEN half of the T3 discharge: a 2P Greedy-vs-Greedy game under the
    /// shipped [`WeightTable::PROVISIONAL_V1`] completes inside the cap and puts
    /// assistants on the board.
    ///
    /// **This test's whole content is that its predecessor's observation is no
    /// longer reproducible.** Before the hand split, this exact call returned
    /// `CapReached { turns: 401 }` with 0 assistants placed, because every
    /// placement scored strictly negative and the empty placement was the argmax
    /// at every Placement point. That is quoted here rather than deleted: it is
    /// the measurement the split was made against, and a future re-pricing that
    /// re-broke the ordering would land back on it. Measured after the split, at
    /// the same master and cap: **turns 42 · moves 274 · assistants placed 58 ·
    /// 27.91 candidates/decision**. Only `placed > 0` and `turns < CAP` are
    /// asserted; the rest is printed, because the figures are properties of a
    /// provisional table T4 will replace.
    ///
    /// Distinct from the `ZERO` degeneracy pin, which it deliberately does not
    /// replace: that one holds the TIE-BREAK (a constant evaluator ties
    /// everything, so the draw is the only thing choosing), this one holds the
    /// WEIGHTS (a strict argmax that must not be systematically wrong).
    #[test]
    fn the_shipped_table_plays_a_game_that_places_assistants_and_ends() {
        const CAP: u32 = 1_500;
        let (stats, tallies) = greedy_selfplay_2p(WeightTable::PROVISIONAL_V1, MASTER, CAP)
            .expect("the shipped table must play a game that ENDS, not one that caps out");

        let placed = assistants_placed(&stats);
        // RECORDED, never asserted beyond "> 0": the exact figures are a
        // property of this master and this provisional table, and T4 re-prices.
        let scored: u64 = tallies.iter().map(|t| t.candidates_scored()).sum();
        let decisions: u64 = tallies.iter().map(|t| t.explained()).sum();
        println!(
            "shipped-table 2P self-play: turns {} · moves {} · assistants placed {placed} · \
             candidates/decision {:.2} ({scored}/{decisions})",
            stats.turns,
            stats.moves_applied,
            scored as f64 / decisions.max(1) as f64,
        );
        assert!(
            placed > 0,
            "the shipped table placed {placed} assistants over {} turns — the one-ply \
             placement sign has gone negative again (T3's degeneracy, spec L14)",
            stats.turns,
        );
        assert!(stats.turns < CAP, "the game must end inside the cap, took {}", stats.turns);

        // The honest-path identity rides this game too, as it does the ZERO one.
        let explained: u64 = tallies.iter().map(|t| t.explained()).sum();
        assert_eq!(
            explained, stats.moves_applied,
            "every applied move was chosen by a decision that produced an explanation",
        );
    }

    /// Play a fresh 2P game under `PROVISIONAL_V1` until the first genuine
    /// `Placement` decision point, and return the true state plus the seat to
    /// move. Callers `observe` it (to ask an agent) or sample a world from that
    /// view (to score candidates the way the agent would).
    fn first_placement_point(rules: &Rules, world_seed: u64) -> (GameState, PlayerId) {
        use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
        let pending =
            new_game(GameConfig::for_player_count(2), rules, world_seed).expect("2P setup");
        let mut state =
            finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
                .expect("finalize");
        let mut agent = GreedyAgent::new(rules, 0x77);
        for _ in 0..64 {
            if state.turn_phase == TurnPhase::Placement && state.pending_choices.is_empty() {
                break;
            }
            let seat = state.current_player;
            let view = observe(&state, seat).expect("observe");
            let r = agent.choose(&view, rules).expect("move");
            state = apply_move_unchecked(&state, &r.mv, rules).expect("apply");
        }
        assert_eq!(state.turn_phase, TurnPhase::Placement, "the walk must reach a placement point");
        let seat = state.current_player;
        let view = observe(&state, seat).expect("observe");
        let world = BeliefSampler::from_view(&view, rules, 1).expect("sampler").sample();
        (world, seat)
    }

    // -- explanations (spec §0.3b) -------------------------------------------

    /// Every decision is explained, the count survives the clone-out, and the
    /// stored explanation is the CHOSEN successor's — pinned by re-deriving it
    /// independently from the move the agent returned.
    #[test]
    fn every_decision_is_explained_and_the_tally_counts_it() {
        let rules = Rules::standard();
        let mut agent = GreedyAgent::new(&rules, 0xC0FFEE);
        let tally = agent.tally();
        assert_eq!(tally.explained(), 0);

        let moves = first_decisions(&mut agent, MASTER, 6, &rules);
        assert_eq!(moves.len(), 6, "the probe must reach six decisions");
        assert_eq!(tally.explained(), 6, "one explanation per decision, read off the clone");
        let last = agent.last_explanation().expect("a decision leaves an explanation");
        assert_eq!(last.terms.len(), eval::Feature::COUNT, "every feature is reported");
    }

    /// The cap-bind and candidate counters ride the same clone-out handle, and
    /// the candidate count is the argmax pass ONLY — the chosen move's
    /// explanation re-application is deliberately not counted, so
    /// `candidates_scored / explained` reads as a candidate MEAN and not as a
    /// mean plus one.
    #[test]
    fn the_cap_bind_and_candidate_counters_ride_the_same_handle() {
        let rules = Rules::standard();
        let agent = GreedyAgent::new(&rules, 1);
        let tally = agent.tally();
        assert_eq!(tally.placement_cap_binds(), 0, "a fresh agent has bound nothing");
        assert_eq!(tally.candidates_scored(), 0);

        // A driven decision sequence moves the candidate counter by exactly the
        // candidate sets it scored, and leaves the cap counter alone at 2P (R2:
        // 0 binds in 48,458 decisions, max realized 396).
        let mut agent = agent;
        let moves = first_decisions(&mut agent, MASTER, 6, &rules);
        assert_eq!(moves.len(), 6);
        assert_eq!(tally.explained(), 6);
        assert!(
            tally.candidates_scored() >= 6,
            "at least one candidate per decision, got {}",
            tally.candidates_scored(),
        );
        assert_eq!(tally.placement_cap_binds(), 0, "2P does not bind the cap here");

        // And the cap DOES bind where the family is genuinely oversized.
        let state = newspaper_placement_state(&rules);
        let handle = DecisionHandle::new(&state, &rules);
        let mut rng = seeded_rng(13);
        assert!(candidates(&handle, DEFAULT_RETREAT_SAMPLES, &mut rng).placement_cap_bound);
    }

    // -- the Keep (spec §8) ---------------------------------------------------

    /// The two claims the Keep vocabulary actually makes, over the SHIPPED deck
    /// rather than over invented cards: printed VP dominates (ceteris paribus),
    /// and each required good subtracts.
    #[test]
    fn keep_scores_printed_economics_with_vp_dominant() {
        let rules = Rules::standard();
        let cards: Vec<&OrderCard> = rules.orders.iter().collect();
        assert_eq!(cards.len(), 36, "the shipped order deck");

        // Ceteris paribus on a REAL card. The shipped deck contains no two cards
        // differing in printed VP alone (checked by the loop below, which is why
        // the comparison is constructed rather than searched for), so the pair
        // is a real card and that card with one more VP.
        let no_natural_pair = cards.iter().all(|a| {
            cards.iter().all(|b| {
                !(a.requirements == b.requirements
                    && rewards_equal_except_vp(&a.rewards, &b.rewards)
                    && a.rewards.victory_points != b.rewards.victory_points)
            })
        });
        assert!(no_natural_pair, "the deck grew a VP-only pair: assert on it directly instead");

        let base = cards[0];
        let mut richer = base.clone();
        richer.rewards.victory_points += 1;
        assert!(
            score_starting_order(&richer, &rules) > score_starting_order(base, &rules),
            "one more printed VP must score higher",
        );

        // Over the WHOLE shipped deck the argmax is a maximum-VP card (measured:
        // 74 for the 11-VP Dutch card, against a −7 floor for a 1-VP French one).
        let best = cards
            .iter()
            .max_by_key(|c| score_starting_order(c, &rules))
            .expect("a non-empty deck");
        let top_vp = cards.iter().map(|c| c.rewards.victory_points).max().unwrap();
        assert_eq!(
            best.rewards.victory_points, top_vp,
            "the deck's best Keep must be a maximum-VP card",
        );

        // Requirements subtract, and a whole VP outweighs one required good
        // (the vocabulary's stated proportion: 10 against 8).
        let mut heavier = base.clone();
        heavier.requirements.copper += 1;
        assert!(
            score_starting_order(&heavier, &rules) < score_starting_order(base, &rules),
            "an extra required good must lower the Keep score",
        );
        let mut heavier_but_richer = heavier.clone();
        heavier_but_richer.rewards.victory_points += 1;
        assert!(
            score_starting_order(&heavier_but_richer, &rules) > score_starting_order(base, &rules),
            "one printed VP must outweigh one required good",
        );

        // The country term discriminates and is sub-VP, as documented.
        let dutch = country_scarcity(Country::Dutch, &rules);
        let english = country_scarcity(Country::English, &rules);
        assert!(dutch > english, "the scarcer country must price higher ({dutch} vs {english})");
        assert!(dutch - english < KEEP_VP, "the country spread must stay under one printed VP");
    }

    /// True when `a` and `b` agree on every printed reward except the VP field.
    fn rewards_equal_except_vp(
        a: &crate::types::OrderRewards,
        b: &crate::types::OrderRewards,
    ) -> bool {
        let mut a2 = *a;
        a2.victory_points = b.victory_points;
        a2 == *b
    }

    /// The Keep is a pure function of the DEAL: no stream draw, no seat
    /// dependence, ties to index 0 (spec §8 + §5.5).
    #[test]
    fn keep_is_deterministic_in_the_deal_and_breaks_ties_by_index_zero() {
        let rules = Rules::standard();
        let ids: Vec<OrderCardId> = rules.orders.iter().map(|c| c.id).collect();
        let deal = [ids[0], ids[1]];

        // Same agent, repeated: no stream advance, so the answer cannot drift.
        let mut agent = GreedyAgent::new(&rules, 0xDEAD);
        let first = agent.choose_starting_order(PlayerId::new(0), &deal);
        for _ in 0..8 {
            assert_eq!(agent.choose_starting_order(PlayerId::new(0), &deal), first);
        }
        // Seat-independent.
        for seat in 0..2u8 {
            assert_eq!(agent.choose_starting_order(PlayerId::new(seat), &deal), first);
        }
        // Different agent seeds agree — the choice is not stream-keyed.
        let mut other = GreedyAgent::new(&rules, 0xBEEF);
        assert_eq!(other.choose_starting_order(PlayerId::new(0), &deal), first);

        // Ties go to index 0.
        let same = [ids[0], ids[0]];
        assert_eq!(agent.choose_starting_order(PlayerId::new(0), &same), 0);

        // And the answer IS the argmax.
        let a = score_starting_order(rules.orders.get(deal[0]).unwrap(), &rules);
        let b = score_starting_order(rules.orders.get(deal[1]).unwrap(), &rules);
        assert_eq!(first, u8::from(b > a));
    }

    // -- provenance -----------------------------------------------------------

    #[test]
    fn controller_provenance_names_greedy_v1_with_its_knobs() {
        let rules = Rules::standard();
        let agent = GreedyAgent::new(&rules, 0x1234);
        let p = agent.controller();
        assert_eq!(p.name, "greedy-v1");
        assert_eq!(p.version, GREEDY_IMPL_VERSION);
        assert_eq!(p.agent_seed, Some(0x1234));
        assert_eq!(p.params["m"], i64::from(DEFAULT_RETREAT_SAMPLES));
        assert_eq!(p.params["eval_impl"], i64::from(EVAL_IMPL_VERSION));
        assert_eq!(p.params["placement_cap"], PLACEMENT_CANDIDATE_CAP as i64);
        assert_eq!(
            p.params["weights"],
            weight_table_identity(&WeightTable::PROVISIONAL_V1),
            "the params map identifies the weight table it actually evaluates with",
        );

        // A different table is a different identity, and the same table twice is
        // the same one (the property a corpus reader pools on).
        let zeroed = GreedyAgent::with_weights(&rules, WeightTable::ZERO, 8, 1);
        assert_ne!(zeroed.controller().params["weights"], p.params["weights"]);
        assert_eq!(
            weight_table_identity(&WeightTable::PROVISIONAL_V1),
            weight_table_identity(&WeightTable::PROVISIONAL_V1),
        );
        let mut nudged = WeightTable::PROVISIONAL_V1;
        nudged.set(eval::Feature::VictoryPoints, 11, 40);
        assert_ne!(
            weight_table_identity(&nudged),
            weight_table_identity(&WeightTable::PROVISIONAL_V1),
            "one changed weight must change the identity",
        );
    }

    /// The M knob rides provenance, so two cells at different M are
    /// distinguishable in the corpus.
    #[test]
    fn the_m_knob_is_recorded() {
        let rules = Rules::standard();
        let agent = GreedyAgent::with_weights(&rules, WeightTable::PROVISIONAL_V1, 32, 9);
        assert_eq!(agent.controller().params["m"], 32);
    }

    // -- the honest chain -----------------------------------------------------
    //
    // Spec §3.1's no-fallback rule, exercised at all three of the chain's exits
    // (T3 review S1: this section shipped with a header and no tests under it).
    // Every one of them is the SAME shape — a typed `Err` carrying a message
    // that names the failure, no panic, no second arm, and NOTHING applied —
    // because a Greedy seat that quietly recovered from a broken chain would
    // emit privileged or arbitrary play into the training corpus, which is the
    // failure `arena`'s driver-level pin (`sampler_failure_is_a_hard_error…`)
    // describes one level up.

    /// (1) The BELIEF SAMPLER's exit. A view whose unseen-order sinks do not
    /// partition the unseen pool is refused, and the refusal reaches the
    /// caller — the `TrivialHonestAgent` precedent instead falls back to the
    /// TRUE legal list, which is exactly the privileged play an arena agent
    /// must never have.
    #[test]
    fn a_view_the_sampler_refuses_is_a_hard_error_with_no_fallback() {
        let rules = Rules::standard();
        let (state, seat) = first_placement_point(&rules, MASTER);
        let mut doctored = observe(&state, seat).expect("observe");
        // Break the reconciliation identity `BeliefSampler::from_view` checks.
        doctored.order_deck_len += 1;

        let mut agent = GreedyAgent::new(&rules, 0xBADF00D);
        let err = agent
            .choose(&doctored, &rules)
            .expect_err("a view whose sinks do not partition the pool must be refused");
        assert!(err.contains("belief sampler refused"), "the refusal must be named: {err}");
        assert!(
            agent.last_explanation().is_none() && agent.tally().explained() == 0,
            "a refused decision must not leave an explanation behind",
        );
        // Anti-vacuity: the SAME agent answers the undoctored view, so the
        // failure is the doctoring and not the fixture.
        let clean = observe(&state, seat).expect("observe");
        assert!(agent.choose(&clean, &rules).is_ok(), "the undoctored view must be answerable");
    }

    /// (2) The NO-FAMILIES exit. `TurnEnd` is not a decision point at all
    /// (`families()` returns empty there), so `decision_volume` reports `None`
    /// and the agent says so rather than inventing a move.
    #[test]
    fn a_view_at_a_phase_with_no_families_is_a_hard_error() {
        let rules = Rules::standard();
        let (state, seat) = first_placement_point(&rules, MASTER);
        let mut view = observe(&state, seat).expect("observe");
        view.turn_phase = TurnPhase::TurnEnd;
        assert!(view.pending_choices.is_empty(), "the queue must be empty, or TurnEnd still asks");

        let mut agent = GreedyAgent::new(&rules, 0x0DD1);
        let err = agent.choose(&view, &rules).expect_err("TurnEnd offers no families");
        assert!(err.contains("offers no families"), "the phase must be named: {err}");
        assert!(err.contains("TurnEnd"), "the message must report WHICH phase: {err}");
    }

    /// (3) The EMPTY-CANDIDATE exit — the third shape, and a genuinely different
    /// one: here the decision point HAS a family (so `decision_volume` answers),
    /// and the family turns out to be structurally empty. An `AreaAction` phase
    /// with the President in hand is the cheap witness
    /// (`generate_area_action_moves` returns nothing with nowhere to act), and
    /// it is worth having on its own rather than folding into case (2) because
    /// the two errors come from different guards: (2) is "not a decision point",
    /// (3) is "a decision point whose families served nothing".
    #[test]
    fn a_decision_point_whose_families_serve_nothing_is_a_hard_error() {
        let rules = Rules::standard();
        let (state, seat) = first_placement_point(&rules, MASTER);
        let mut view = observe(&state, seat).expect("observe");
        view.turn_phase = TurnPhase::AreaAction;
        view.players
            .iter_mut()
            .find(|p| p.id == seat)
            .expect("the viewer's own row")
            .president_location = PresidentLocation::InHand;

        let mut agent = GreedyAgent::new(&rules, 0x0DD2);
        let err = agent
            .choose(&view, &rules)
            .expect_err("an AreaAction point with the President in hand serves no move");
        assert!(err.contains("dead end"), "the dead end must be named: {err}");
        assert!(err.contains(GREEDY_CONTROLLER), "and attributed to this controller: {err}");
    }
}
