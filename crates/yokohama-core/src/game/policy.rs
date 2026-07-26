//! Rollout policies over the factored action source (Phase 3.2 Task 5; spec
//! `docs/plans/2026-07-25-phase3.2-arena-spec.md` §2, §0-R2(b)).
//!
//! Three preserved rungs, one enum, one `pick`. Every rung consumes ONLY the
//! [`DecisionHandle`] surface (§8): the exploding retreat family is never
//! materialized, and a Movement decision point pays exactly ONE route
//! enumeration however many of the handle's readers run (L10, pinned by
//! `tests/l10_enumeration_pin.rs`).
//!
//! ## The three rungs
//!
//! - [`RolloutPolicy::Uniform`] — exactly uniform over the legal union, served
//!   families-first. At the one multi-family shape (Movement) both family sizes
//!   are EXACT — `retreat_len`'s closed form and the collected routes `Vec`'s
//!   length — so one draw over their SUM is uniform over the union; at every
//!   single-family point it is uniform-within-family. Placement's `UpperBound`
//!   never enters a weight, because Placement is always single-family.
//! - [`RolloutPolicy::RetreatAverse`] — never volunteer a retreat. At a Movement
//!   point the pool is the routes ∪ {**the single all-zero**
//!   `ReturnPresidentToHand`}; **when routes are structurally empty the all-zero
//!   vector is FORCED, never a nonzero-recovery one** (the pool is never empty,
//!   so difftest's full-list fallback is unreachable at Movement). Uniform
//!   elsewhere.
//! - [`RolloutPolicy::FamilyWeighted`] — the two-knob shape. Knob 1:
//!   P(retreat family | Movement point) = `retreat_ppm`/10⁶ (default 166_667 =
//!   1/6). Knob 2: each area's recovery count is the MIN of `recovery_min_of`
//!   iid `U{0..=count}` draws (default 5) — full support, so every recovery
//!   vector including the maximal one keeps positive probability, which is the
//!   zero-interior-coverage defect it exists to fix. Routes structurally empty
//!   ⇒ the weighted retreat draw proceeds; uniform elsewhere.
//!
//! ## Structural honesty: "family weighting" is one scalar, not a table
//!
//! [`families`] returns more than one tag at exactly ONE decision-point shape —
//! Movement, always `[MovementRoutes, MovementRetreat]` (verified by code read
//! and by E1's 133,800-game executable guard). So a per-family weight TABLE
//! would have exactly one row that ever branches, and is deliberately not built:
//! the knob is a routes-vs-retreat scalar plus within-family weighting. The
//! Movement arm carries a `debug_assert` on that two-tag shape, so a future
//! third tag fails loudly instead of silently skewing the draw.
//!
//! ## Sampler-side only
//!
//! The engine's legal list is never weighted (map §5-3.2 item 3, settled).
//! Everything here is a draw over families the frozen `get_legal_moves` already
//! defines; `get_legal_moves` stays the authority `apply_move` checks against.
//!
//! ## Fidelity to difftest's `pick_move`, and its one honest exception
//!
//! `yokohama-difftest`'s list-based `pick_move` is the implementation E2 measured
//! the retreat-averse rung with, and difftest now re-exports THIS enum
//! (`drive::Policy`). The two implementations agree move-for-move and
//! draw-for-draw from the same stream at every decision point EXCEPT Placement,
//! and the exception is structural rather than incidental:
//!
//! - at a Movement point both spend ONE `gen_range(0..routes_len + 1)` — the
//!   legal list's non-retreat prefix IS the routes family followed by the
//!   all-zero recovery (`enumerate_recovery_options` emits it at index 0), so
//!   the same index selects the same move;
//! - at every delegated single-family point both spend ONE
//!   `gen_range(0..family_len)` over the same slice of the same generator output;
//! - at a **Placement** point difftest indexes the materialized list once, while
//!   this module goes through `placement_sample`'s reservoir, which spends one
//!   draw PER candidate precisely so the family is never materialized. Both are
//!   exactly uniform over the realized placement moves; they are not the same
//!   draw, and no amount of care makes them one without materializing what the
//!   factored source exists not to materialize.
//!
//! `crates/yokohama-difftest/src/drive.rs`'s
//! `core_retreat_averse_agrees_with_pick_move_move_for_move_and_draw_for_draw`
//! is that statement made executable, partitioned exactly this way.

use std::collections::BTreeMap;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::game::search_actions::{retreat_move, retreat_sample, DecisionHandle, MoveFamily};
use crate::types::{AreaId, Move};

/// Bumped when a rung's DRAW changes in a way that makes previously-recorded
/// figures non-comparable. Stamped into `GameRecord.run` provenance
/// (`SeatProvenance::version`, Task 8) so a corpus record says which policy
/// implementation produced it, not merely which policy NAME.
pub const POLICY_IMPL_VERSION: u32 = 1;

/// One decision-point move-selection rule. Seat-blind and pure over
/// `(handle, rng)` — an agent's identity lives in the agent, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RolloutPolicy {
    /// Exactly uniform over the legal union (the fuzzer's policy).
    Uniform,
    /// Never volunteer a retreat (difftest's rung; E2's measured object).
    RetreatAverse,
    /// The two-knob weighted shape — see the module doc.
    FamilyWeighted {
        /// P(retreat family | Movement decision point), in parts per million.
        /// Must be ≤ 1_000_000 (`Rng::gen_ratio` asserts it in every profile).
        retreat_ppm: u32,
        /// How many iid `U{0..=count}` draws each area's recovery count is the
        /// MINIMUM of. Larger = more retreat-averse WITHIN the retreat family,
        /// while keeping full support. Treated as 1 if 0 (a k of zero has no
        /// minimum to take).
        recovery_min_of: u8,
    },
}

impl RolloutPolicy {
    /// The shipped weighted defaults: E1's best-measured cell (`low5_0.1667`,
    /// held-out 4P ChurchTrackFull 3.7% [2.96, 4.62]) inside its validated box
    /// p ∈ [0.05, 0.20] × k ∈ [3, 5]. The acceptance criteria (§0-R1, §7, §12)
    /// gate the SHAPE; these constants are the best cell inside it, not a
    /// separately-gated claim.
    pub const FAMILY_WEIGHTED_DEFAULT: Self =
        Self::FamilyWeighted { retreat_ppm: 166_667, recovery_min_of: 5 };

    /// The stable name — artifact filenames, CLI args, and provenance
    /// `controller` strings all come from here so none of them is hand-typed.
    /// The first two are byte-identical to the names difftest has always used
    /// (three checked-in artifacts under `crates/yokohama-difftest/fixtures/`
    /// carry them in filename AND payload).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uniform => "uniform",
            Self::RetreatAverse => "retreat-averse",
            Self::FamilyWeighted { .. } => "family-weighted",
        }
    }

    /// Parse a policy NAME. `"family-weighted"` yields
    /// [`FAMILY_WEIGHTED_DEFAULT`](Self::FAMILY_WEIGHTED_DEFAULT) — a name
    /// carries no knob values, so a caller wanting non-default knobs constructs
    /// the variant.
    ///
    /// `"retreat_averse"` is accepted as an alias because difftest's CLI has
    /// always accepted it. Note that difftest deliberately does NOT route its
    /// argv through this function (see `drive::parse_cli_policy`): its parser
    /// stays two-name so its `pick_move`'s family-weighted arm is unreachable.
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "uniform" => Some(Self::Uniform),
            "retreat-averse" | "retreat_averse" => Some(Self::RetreatAverse),
            "family-weighted" | "family_weighted" => Some(Self::FAMILY_WEIGHTED_DEFAULT),
            _ => None,
        }
    }

    /// `(name, impl version, knob values)` for provenance and report headers.
    ///
    /// The knobs are integers in their declared units (ppm for the probability,
    /// a count for k) rather than floats, so the map is `Eq`-safe and a record
    /// round-trips bit-exactly — a float would make `GameRecord` comparison
    /// depend on formatting.
    pub fn descriptor(&self) -> (String, u32, BTreeMap<String, i64>) {
        let mut params = BTreeMap::new();
        if let Self::FamilyWeighted { retreat_ppm, recovery_min_of } = *self {
            params.insert("retreat_ppm".to_string(), i64::from(retreat_ppm));
            params.insert("recovery_min_of".to_string(), i64::from(recovery_min_of));
        }
        (self.as_str().to_string(), POLICY_IMPL_VERSION, params)
    }

    /// Draw one move at `handle`'s decision point, or `None` when the point has
    /// no move to make.
    ///
    /// `None` has two causes and neither is an error here: `handle` was built at
    /// a state with no families at all (Recovery/TurnEnd — automatic phases),
    /// or its single family is structurally empty (e.g. a fully-blocked routes
    /// step). The caller decides which is a dead end — the arena driver reports
    /// `ArenaError::DeadEnd`, a search may simply have nothing to expand.
    ///
    /// Never `None` at a Movement point: the retreat family always contains at
    /// least the recover-nothing move (`retreat_len` is an empty product = 1).
    pub fn pick(&self, handle: &DecisionHandle, rng: &mut impl Rng) -> Option<Move> {
        let families = handle.families();

        // THE two-tag Movement shape — the only decision point where a rung has
        // a choice of FAMILY to make. Everything else is uniform-within-family
        // under all three rungs, because the retreat family exists nowhere else.
        // The shape is matched in FULL — both tags, in order — so anything else
        // takes the defended arm rather than being mistaken for Movement. The
        // `debug_assert` there is the two-tag pin spec §2 requires: a future
        // third tag (or a reordered pair) fails loudly in debug instead of
        // silently skewing the draw, and degrades to an approximately-uniform
        // union draw in release rather than to a panic or a wrong weighting.
        let [routes @ MoveFamily::MovementRoutes, retreat @ MoveFamily::MovementRetreat { on_board }] =
            families
        else {
            debug_assert!(
                families.len() <= 1,
                "RolloutPolicy::pick: families() produced the multi-family shape {families:?}, but \
                 the only one this module models is Movement's [MovementRoutes, MovementRetreat]. \
                 A new multi-family decision point needs its weighting decided here, not inherited \
                 (spec §2's two-tag pin).",
            );
            return match families {
                [] => None,
                [only] => handle.sample(only, rng),
                many => size_weighted_union_draw(handle, many, rng),
            };
        };

        // EXACT (the memoized enumeration's length), and reading it here is what
        // pays L10's single route DFS for the whole decision point — every
        // branch below reads the same memo. Read UNCONDITIONALLY: even the
        // weighted retreat branch pays the enumeration, because the L10 pin
        // asserts DFS == 1 at on-board Movement points and the volume metric
        // reads the same count. A 3.3 search wanting it lazy on the retreat
        // branch owns that trade-off (and the pin's rewrite) itself.
        let routes_len = handle.size(routes).value() as usize;

        match *self {
            // Two-stage by EXACT sizes, collapsed into ONE draw: `j` uniform over
            // the union's size picks the family, and where it lands in the routes
            // block it doubles as the uniform index WITHIN it (conditional on
            // `j < routes_len`, `j` is uniform on `0..routes_len` — valid
            // PRECISELY because the routes block occupies the contiguous
            // prefix `[0, routes_len)` of the draw range; a non-contiguous
            // mapping would need different index arithmetic — T5 review,
            // probe 1). So each route
            // has probability `1/total` and each recovery vector
            // `(retreat_len/total)·(1/retreat_len) = 1/total` — exactly uniform
            // over the union, which is what the composition pin measures.
            Self::Uniform => {
                let retreat_len = handle.size(retreat).value() as usize;
                let total = routes_len + retreat_len;
                if total == 0 {
                    return None;
                }
                let j = rng.gen_range(0..total);
                if j < routes_len {
                    handle.get(routes, j)
                } else {
                    Some(retreat_sample(on_board, rng))
                }
            }

            // The pool is routes ∪ {the SINGLE all-zero recovery} — one draw over
            // `routes_len + 1`, index `routes_len` being the sentinel. With routes
            // structurally empty the range is `0..1` and the all-zero vector is
            // FORCED: this arm can no more emit a nonzero recovery than it can
            // emit an illegal move, which is the rung's identity (§2) and what
            // `retreat_averse_never_applies_a_nonzero_recovery` pins.
            //
            // The draw is deliberately the same shape difftest's list-based
            // `pick_move` makes over its non-retreat subset, whose members are
            // these same routes followed by this same all-zero recovery.
            Self::RetreatAverse => {
                let i = rng.gen_range(0..routes_len + 1);
                if i < routes_len {
                    handle.get(routes, i)
                } else {
                    Some(retreat_move(on_board, &[]))
                }
            }

            // Knob 1 decides the family; knob 2 shapes the vector WITHIN the
            // retreat family. A structurally-empty routes family is not a special
            // case in the code because it is not one in the semantics: the
            // weighted retreat draw simply proceeds (`handle.sample` spends no
            // draw on an empty family — it tests emptiness before drawing).
            Self::FamilyWeighted { retreat_ppm, recovery_min_of } => {
                // Knob hygiene (T5 review, probe 5): `gen_ratio` PANICS on a
                // numerator above its denominator — an out-of-scale knob would
                // otherwise kill an evaluation at its first Movement decision
                // point while `descriptor()` had already recorded the bad
                // value into provenance. Loud in debug; in release it clamps
                // to always-retreat, the monotone reading of "more than
                // certain". (`recovery_min_of = 0` normalises inside
                // `weighted_retreat`, pinned by its test.)
                debug_assert!(
                    retreat_ppm <= 1_000_000,
                    "FamilyWeighted.retreat_ppm = {retreat_ppm} exceeds the ppm scale (1_000_000)",
                );
                if rng.gen_ratio(retreat_ppm.min(1_000_000), 1_000_000) {
                    return Some(weighted_retreat(on_board, recovery_min_of, rng));
                }
                match handle.sample(routes, rng) {
                    Some(mv) => Some(mv),
                    None => Some(weighted_retreat(on_board, recovery_min_of, rng)),
                }
            }
        }
    }
}

/// One recovery vector under knob 2: each occupied area's count is the MINIMUM
/// of `min_of` iid `U{0..=count}` draws, biasing toward recovering little while
/// keeping FULL SUPPORT — every vector, the maximal one included, stays
/// reachable. That full support is the whole point: retreat-aversion's zero
/// interior coverage is the defect this rung exists to fix.
///
/// `O(#areas)` draws and a HANDFUL of allocations — the pre-sized per-area
/// `Vec` plus [`retreat_move`]'s `recover` map, whose `BTreeMap` nodes make the
/// count data-dependent in `|recover|` (the T5 review measured 6–8 at the
/// 17-area worst case before the pre-sizing below) — and O(1) in the family's
/// `∏(count+1)`, which is the property `action_source_worstcase.rs`'s pick
/// bound gates and records at 442,368.
///
/// `min_of = 0` is treated as 1: a minimum over zero draws has no value, and
/// refusing to build a move for a knob a caller could set is worse than the
/// nearest meaningful reading.
fn weighted_retreat(
    on_board: &BTreeMap<AreaId, u8>,
    min_of: u8,
    rng: &mut impl Rng,
) -> Move {
    let draws = min_of.max(1);
    let mut per_area: Vec<(AreaId, u8)> = Vec::with_capacity(on_board.len());
    for (&area, &count) in on_board.iter().filter(|&(_, &count)| count > 0) {
        let mut chosen = rng.gen_range(0..=count);
        for _ in 1..draws {
            chosen = chosen.min(rng.gen_range(0..=count));
        }
        per_area.push((area, chosen));
    }
    retreat_move(on_board, &per_area)
}

/// The defended-unreachable arm of [`RolloutPolicy::pick`]: a size-weighted draw
/// — NOTE: no in-repo test exercises the RELEASE arm (the debug assertion fires
/// first in every per-edit run); the T5 review exercised it externally over a
/// doctored three-tag decision point — 108 distinct outcomes across the 108-move
/// legal list, zero illegal draws — so its degrade-to-legal claim is verified,
/// not assumed, but by a review harness rather than a gate.
/// across an arbitrary family list, retrying past a family that turns out empty.
///
/// No state reaches it today — `families()` emits 0, 1, or exactly Movement's
/// two tags — and the `debug_assert` at its call site says so loudly in debug.
/// It exists because the release alternative to a wrong-shape decision point is
/// either a panic or a silently skewed draw, and "approximately uniform over the
/// union" is the only third option. Uniform is exactly right here; the other two
/// rungs would need their own weighting decided, which is precisely what the
/// assertion demands of whoever adds the third tag.
fn size_weighted_union_draw(
    handle: &DecisionHandle,
    families: &[MoveFamily],
    rng: &mut impl Rng,
) -> Option<Move> {
    let mut weights: Vec<u64> = families.iter().map(|f| handle.size(f).value()).collect();
    loop {
        let total: u64 = weights.iter().sum();
        if total == 0 {
            return None;
        }
        let mut drawn = rng.gen_range(0..total);
        let at = weights.iter().position(|&w| {
            if drawn < w {
                true
            } else {
                drawn -= w;
                false
            }
        })?;
        match handle.sample(&families[at], rng) {
            Some(mv) => return Some(mv),
            // A family whose size said non-empty and whose sampler disagrees:
            // zero its weight and redraw rather than trust either.
            None => weights[at] = 0,
        }
    }
}

impl std::str::FromStr for RolloutPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_name(s).ok_or_else(|| {
            format!("unknown policy {s:?} (expected uniform | retreat-averse | family-weighted)")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::metrics::CoverageTally;
    use crate::game::search_actions::tests::{
        handle_fee_gated_placement_state, handle_movement_state,
    };
    use crate::game::{
        apply_move_unchecked, finalize_setup, get_legal_moves, is_game_over, new_game, Rules,
        SeedSchedule,
    };
    use crate::types::{GameConfig, PresidentLocation};
    use crate::StartingOrderChoice;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::collections::BTreeMap;

    /// The committed diagnostic master for this file's driven cells. Diagnostic
    /// BY NAME (`SeedSchedule::diagnostic`, spec §3): a fixed master is what a
    /// per-edit pin needs and what the seed-hygiene rule exempts, because these
    /// are gate cells rather than bot-facing evaluation runs.
    const MASTER: u64 = 0xA3E1_2026_0726_0005;

    /// Worlds per driven cell. Played at BOTH 2P and 4P, so this is the plan's
    /// "30 full driven games" = 15 worlds × {2P, 4P} (the plan's "seeds 0..30,
    /// 2P+4P" reads ambiguously as 30 or 60 games; N buys REACHABILITY here,
    /// not confidence — the identity is structural — so 30 games with floored
    /// partitions is the load-bearing shape; T5 review probe 4 confirmed 120
    /// independent games change nothing).
    const WORLDS: u32 = 15;

    // -- fixtures ----------------------------------------------------------

    /// The in-hand twin of `handle_movement_state`: same board, President in
    /// HAND, so the routes family becomes the `PlacePresident` arm while the
    /// retreat family grows an area. Both Movement arms matter — 88.6% of
    /// organic Movement points are in-hand under uniform play, and the two arms
    /// reach `movement_routes_family` through different code.
    fn movement_in_hand_state() -> crate::types::GameState {
        let mut s = handle_movement_state();
        s.players[0].president_location = PresidentLocation::InHand;
        s.players[0].assistants.on_board.insert(AreaId::new(0), 1);
        s
    }

    /// Empirical distribution over `format!("{mv:?}")` keys from `draws` picks.
    fn empirical(
        policy: RolloutPolicy,
        state: &crate::types::GameState,
        rules: &Rules,
        draws: u64,
        seed: u64,
    ) -> BTreeMap<String, u64> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut counts = BTreeMap::new();
        for _ in 0..draws {
            let handle = DecisionHandle::new(state, rules);
            let mv = policy.pick(&handle, &mut rng).expect("a decision point yields a move");
            *counts.entry(format!("{mv:?}")).or_insert(0) += 1;
        }
        counts
    }

    /// Total-variation distance between an empirical count map and the EXACT
    /// uniform distribution over `get_legal_moves` at the same state.
    ///
    /// TV is `½ Σ|p̂ − p|` over the union of both supports, so a move the policy
    /// never draws and a move it draws but the generator never emits both
    /// contribute in full — the metric cannot be satisfied by a policy that
    /// merely spreads its mass evenly over the WRONG set.
    fn tv_against_list_uniform(
        counts: &BTreeMap<String, u64>,
        state: &crate::types::GameState,
        rules: &Rules,
        draws: u64,
    ) -> f64 {
        let legal = get_legal_moves(state, rules);
        assert!(!legal.is_empty(), "a TV fixture must be a real decision point");
        let k = legal.len() as f64;
        let uniform: BTreeMap<String, f64> =
            legal.iter().map(|m| (format!("{m:?}"), 1.0 / k)).collect();

        let mut keys: Vec<&String> = uniform.keys().collect();
        keys.extend(counts.keys().filter(|key| !uniform.contains_key(*key)));
        keys.sort();
        keys.dedup();

        let total = draws as f64;
        keys.iter()
            .map(|key| {
                let p_hat = counts.get(*key).copied().unwrap_or(0) as f64 / total;
                let p = uniform.get(*key).copied().unwrap_or(0.0);
                (p_hat - p).abs()
            })
            .sum::<f64>()
            / 2.0
    }

    /// The TV threshold this file asserts at `draws` samples over `k` bins.
    ///
    /// Derived, not guessed: for a correct sampler the expected TV distance is
    /// approximately `½ · k · √(2p(1−p)/(πN))` with `p = 1/k` (the mean absolute
    /// deviation of a binomial, summed over bins and halved). This returns FOUR
    /// times that, so a correct sampler has ~4× headroom while the smallest
    /// realistic defect — mis-weighting the two Movement families, which moves
    /// TV by 0.1–0.3 on these fixtures — fails by an order of magnitude. The
    /// seeds are fixed, so the realized value is deterministic and the margin is
    /// reported by every call site.
    fn tv_threshold(k: usize, draws: u64) -> f64 {
        let k = k as f64;
        let p = 1.0 / k;
        4.0 * 0.5 * k * (2.0 * p * (1.0 - p) / (std::f64::consts::PI * draws as f64)).sqrt()
    }

    fn assert_matches_list_uniform(
        state: &crate::types::GameState,
        rules: &Rules,
        draws: u64,
        seed: u64,
        label: &str,
    ) {
        let k = get_legal_moves(state, rules).len();
        let counts = empirical(RolloutPolicy::Uniform, state, rules, draws, seed);
        let tv = tv_against_list_uniform(&counts, state, rules, draws);
        let threshold = tv_threshold(k, draws);
        println!(
            "{label}: {k} legal moves, {draws} draws, TV {tv:.5} against list-uniform \
             (threshold {threshold:.5})",
        );
        assert!(
            tv < threshold,
            "{label}: TV {tv:.5} >= {threshold:.5} — families-first Uniform is not uniform over \
             the legal union ({k} bins, {draws} draws, seed {seed})",
        );
    }

    // -- composition: families-first Uniform IS list uniform ----------------

    /// The composition pin the existing per-family sampler pins do NOT cover:
    /// each family's own sampler being uniform says nothing about the two-stage
    /// draw ACROSS families being uniform over their union. Both Movement arms.
    ///
    /// MEASURED at HEAD (debug): on-board 4 bins, TV 0.00280 against a 0.01128
    /// threshold; in-hand 8 bins, TV 0.00595 against 0.01724.
    ///
    /// MUTATION-PROVEN 2026-07-26 (the bound passes on first write, so this is
    /// where its discriminating power comes from). Replacing the size-weighted
    /// draw with a 50/50 family draw — `let j = if rng.gen_ratio(1, 2) { 0 }
    /// else { routes_len };`, the exact defect "two-stage by EXACT sizes" rules
    /// out, and one that leaves BOTH families perfectly uniform within
    /// themselves — took the on-board cell to **TV 0.24968, 22× the threshold**,
    /// and the assertion FAILED. The placement cell below was unaffected, which
    /// is the localization check. Reverted by inverse edit (L7).
    #[test]
    fn uniform_families_first_matches_list_uniform_at_a_movement_point() {
        let rules = Rules::empty_for_tests();
        assert_matches_list_uniform(
            &handle_movement_state(),
            &rules,
            60_000,
            0xA11_0001,
            "movement on-board",
        );
        assert_matches_list_uniform(
            &movement_in_hand_state(),
            &rules,
            60_000,
            0xA11_0002,
            "movement in-hand",
        );
    }

    /// The single-family half of the same property, at the fixture whose
    /// affordability hole makes `family_len` a STRICT over-estimate: the
    /// reservoir must be uniform over the REALIZED moves, and the upper bound
    /// must never leak into the draw.
    ///
    /// MEASURED at HEAD (debug): 18 realized moves, TV 0.00597 against a 0.02686
    /// threshold.
    ///
    /// MUTATION-PROVEN 2026-07-26 with the defect this cell exists for — drawing
    /// an index over `handle.size()`'s UPPER BOUND and falling back to the first
    /// move when it lands in an affordability hole (`handle.iter(only).nth(
    /// rng.gen_range(0..upper)).or_else(|| handle.iter(only).next())`). One hole
    /// out of nineteen is a 5% over-weight on a single move, and the assertion
    /// FAILED at **TV 0.04901, 1.8× the threshold**; the Movement cells above
    /// were unaffected. That is the sensitivity this fixture buys — a coarser
    /// mutation would have proven far less. Reverted by inverse edit (L7).
    #[test]
    fn uniform_families_first_matches_list_uniform_at_a_fee_gated_placement_point() {
        let rules = Rules::empty_for_tests();
        assert_matches_list_uniform(
            &handle_fee_gated_placement_state(),
            &rules,
            60_000,
            0xA11_0003,
            "fee-gated placement",
        );
    }

    // -- driven games -------------------------------------------------------

    /// What a run of driven games reports back: the applied-move tally plus the
    /// decision-point shape counts that make a coverage claim non-vacuous.
    ///
    /// One tally threaded through every game rather than per-game tallies
    /// merged, because `CoverageTally`'s counts are private with read-only
    /// accessors on purpose (T4 review): `observe` is its only writer, so a
    /// merge would have to re-invent the denominator a floor gates on.
    #[derive(Default, Debug)]
    struct Driven {
        coverage: CoverageTally,
        games: u64,
        movement_points: u64,
        placement_points: u64,
        other_points: u64,
    }

    /// Drive one full game with every seat on `policy`, accumulating into `out`.
    ///
    /// A raw-state driver on purpose: policy semantics are a property of the
    /// (handle, rng) pair, and the honest view→sampler→handle chain is Task 6's
    /// arena. `apply_move_unchecked` is sound here for the same reason it is in
    /// the fuzz drivers — every move comes from the factored source, whose
    /// equivalence to `get_legal_moves` is the equivalence gate's pin.
    fn drive(policy: RolloutPolicy, world: u64, players: u8, rules: &Rules, out: &mut Driven) {
        const MAX_DECISIONS: u32 = 40_000;

        let config = GameConfig::for_player_count(players);
        let pending = new_game(config, rules, world).expect("new_game");
        let mut rng = ChaCha8Rng::seed_from_u64(world);
        let choices: Vec<StartingOrderChoice> =
            (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
        let mut state = finalize_setup(pending, &choices).expect("finalize_setup");

        for _ in 0..MAX_DECISIONS {
            if is_game_over(&state) {
                break;
            }
            let handle = DecisionHandle::new(&state, rules);
            match handle.families() {
                [] => panic!("a non-terminal state must dispatch at least one family"),
                [MoveFamily::MovementRoutes, ..] => out.movement_points += 1,
                [MoveFamily::Placement] => out.placement_points += 1,
                _ => out.other_points += 1,
            }
            let mv = policy.pick(&handle, &mut rng).expect("a decision point yields a move");
            out.coverage.observe(&mv);
            state = apply_move_unchecked(&state, &mv, rules).unwrap_or_else(|e| {
                panic!("world {world} {players}P: {mv:?} failed to apply: {e:?}")
            });
        }
        assert!(is_game_over(&state), "world {world} {players}P hit the decision cap");
        out.games += 1;
    }

    /// Drive `games` worlds per player count off the committed diagnostic
    /// master, returning the accumulated tally.
    fn drive_cells(policy: RolloutPolicy, games: u32, counts: &[u8]) -> Driven {
        let rules = Rules::standard();
        let mut schedule = SeedSchedule::diagnostic(MASTER);
        let mut total = Driven::default();
        for _ in 0..games {
            let world = schedule.next_world().0;
            for &players in counts {
                drive(policy, world, players, &rules, &mut total);
            }
        }
        total
    }

    /// A `>=` floor set at roughly half a MEASURED value, so a benign drift in
    /// game length is not a false failure while a cell that stopped reaching the
    /// shape it claims to cover trips loudly. The measurement is quoted at every
    /// call site (spec's measured-floor discipline; never an invented number).
    fn assert_floor(got: u64, floor: u64, measured: u64, what: &str) {
        assert!(
            got >= floor,
            "{what}: {got} is below the floor {floor} (half of the {measured} this cell measured \
             at HEAD) — the coverage this pin claims is no longer being reached",
        );
    }

    /// The retreat-averse rung's IDENTITY, as a property of applied moves:
    /// across 30 driven games it applies ZERO nonzero-recovery retreats.
    ///
    /// The zero is STRUCTURAL, not statistical — the Movement arm can only ever
    /// emit the all-zero vector, so no N would make it "more true". The games
    /// are here to prove the arm is REACHED, which is why the floors below are
    /// the load-bearing half of this test: a zero over games that never reached
    /// a Movement point, or never took the retreat sentinel, would prove nothing
    /// at all.
    ///
    /// MEASURED at HEAD (debug, master `0xA3E1_2026_0726_0005`, 15 worlds ×
    /// {2P, 4P}): 35,950 applied moves, 5,602 Movement decision points, 1,655
    /// retreat picks, 0 nonzero-recovery retreats. E2 measured the same identity
    /// on difftest's implementation at 0 of 345,800 applied 4P moves.
    #[test]
    fn retreat_averse_never_applies_a_nonzero_recovery() {
        let d = drive_cells(RolloutPolicy::RetreatAverse, WORLDS, &[2, 4]);
        println!("retreat-averse driven cells: {d:?}");

        assert_eq!(d.games, 2 * u64::from(WORLDS), "every world must play at both counts");
        assert_floor(d.movement_points, 2_800, 5_602, "Movement decision points reached");
        assert_floor(d.coverage.retreat_picks(), 800, 1_655, "all-zero retreat sentinels taken");

        assert_eq!(
            d.coverage.nonzero_recovery_retreats(),
            0,
            "retreat-averse applied {} nonzero-recovery retreats over {} applied moves — the rung's \
             identity is that this is structurally impossible",
            d.coverage.nonzero_recovery_retreats(),
            d.coverage.applied(),
        );
    }

    /// Anti-vacuity for the weighted rung's INTERIOR, on the SAME cells: the
    /// draw that gives retreat-averse a hard zero must give the weighted policy
    /// a strictly positive count, or the two rungs would be indistinguishable by
    /// this measurement and the zero above would be a property of the harness
    /// rather than of the rung.
    ///
    /// MEASURED at HEAD (same master and cells): 34,589 applied moves, 1,369
    /// retreat picks, 452 of them nonzero-recovery.
    #[test]
    fn family_weighted_reaches_nonzero_recoveries() {
        let d = drive_cells(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT, WORLDS, &[2, 4]);
        println!("family-weighted driven cells: {d:?}");

        assert_floor(d.movement_points, 2_600, 5_322, "Movement decision points reached");
        assert!(
            d.coverage.nonzero_recovery_retreats() > 0,
            "family-weighted applied no nonzero-recovery retreat over {} applied moves: {d:?}",
            d.coverage.applied(),
        );
        assert_floor(
            d.coverage.nonzero_recovery_retreats(),
            226,
            452,
            "nonzero-recovery retreats applied",
        );
    }

    // -- the order identity the difftest agreement pin rests on -------------

    /// `handle.iter(family)` must be ELEMENTWISE equal to `sample_family`'s own
    /// serving order — which is `iter_family`'s, which is the generator's
    /// subsequence. RetreatAverse maps ONE drawn index onto the routes family,
    /// and difftest maps the same index onto the legal list's non-retreat
    /// prefix; if those two orders ever diverge the two implementations would
    /// still agree on the DISTRIBUTION and silently disagree on the move. This
    /// makes an ordering change fail loudly here rather than quietly there.
    #[test]
    fn handle_routes_order_matches_the_legal_lists_non_retreat_prefix() {
        let rules = Rules::empty_for_tests();
        for (label, state) in [
            ("on-board", handle_movement_state()),
            ("in-hand", movement_in_hand_state()),
        ] {
            let handle = DecisionHandle::new(&state, &rules);
            let routes: Vec<Move> = handle.iter(&MoveFamily::MovementRoutes).collect();
            let prefix: Vec<Move> = get_legal_moves(&state, &rules)
                .into_iter()
                .filter(|m| {
                    matches!(m, Move::MovePresident { .. } | Move::PlacePresident { .. })
                })
                .collect();
            assert!(!routes.is_empty(), "{label}: anti-vacuity — the routes family must be real");
            assert_eq!(routes, prefix, "{label}: routes order must be the generator's");
        }
    }

    // -- names and descriptors ---------------------------------------------

    #[test]
    fn names_round_trip_and_keep_difftests_two_byte_identical() {
        assert_eq!(RolloutPolicy::Uniform.as_str(), "uniform");
        assert_eq!(RolloutPolicy::RetreatAverse.as_str(), "retreat-averse");
        assert_eq!(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT.as_str(), "family-weighted");

        for p in [
            RolloutPolicy::Uniform,
            RolloutPolicy::RetreatAverse,
            RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
        ] {
            assert_eq!(RolloutPolicy::from_str_name(p.as_str()), Some(p), "{p:?} round-trips");
        }
        // The alias difftest's CLI has always accepted.
        assert_eq!(
            RolloutPolicy::from_str_name("retreat_averse"),
            Some(RolloutPolicy::RetreatAverse),
        );
        assert_eq!(RolloutPolicy::from_str_name("greedy"), None);
        assert!("greedy".parse::<RolloutPolicy>().is_err());
    }

    #[test]
    fn descriptor_reports_integer_knobs_only_for_the_weighted_rung() {
        let (name, version, params) = RolloutPolicy::RetreatAverse.descriptor();
        assert_eq!((name.as_str(), version), ("retreat-averse", POLICY_IMPL_VERSION));
        assert!(params.is_empty(), "a knobless rung reports no params");

        let (name, version, params) = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT.descriptor();
        assert_eq!((name.as_str(), version), ("family-weighted", POLICY_IMPL_VERSION));
        assert_eq!(params.get("retreat_ppm"), Some(&166_667));
        assert_eq!(params.get("recovery_min_of"), Some(&5));
    }

    /// `recovery_min_of = 0` normalises to 1 (a minimum over zero draws has no
    /// value) — pinned so a refactor cannot silently turn it into a panic or a
    /// max (T5 review probe 5). With count = 2 and k = 1 the per-area draw is
    /// uniform over {0, 1, 2}; generous bounds, fixed seed, zero flake.
    #[test]
    fn recovery_min_of_zero_is_normalised_to_one() {
        let on_board: BTreeMap<AreaId, u8> = [(AreaId::new(0), 2u8)].into_iter().collect();
        let mut rng = ChaCha8Rng::seed_from_u64(0x5EED_2026_0726_0007);
        let mut counts = [0u32; 3];
        for _ in 0..30_000 {
            match weighted_retreat(&on_board, 0, &mut rng) {
                Move::ReturnPresidentToHand { recover } => {
                    counts[recover.get(&AreaId::new(0)).copied().unwrap_or(0) as usize] += 1;
                }
                other => panic!("weighted_retreat built {other:?}"),
            }
        }
        for (value, &seen) in counts.iter().enumerate() {
            assert!(
                (8_500..=11_500).contains(&seen),
                "k=0 must draw uniformly (k=1): count {value} seen {seen} of 30,000",
            );
        }
    }

    /// An out-of-scale `retreat_ppm` is LOUD in debug (T5 review probe 5: it
    /// previously panicked inside `rand` at the first Movement decision point,
    /// after `descriptor()` had already recorded the bad knob). The release
    /// behavior — clamp to always-retreat — is one visible line in the arm.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "exceeds the ppm scale")]
    fn retreat_ppm_above_the_ppm_scale_is_loud_in_debug() {
        let state = handle_movement_state();
        let rules = Rules::empty_for_tests();
        let handle = DecisionHandle::new(&state, &rules);
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        let bad = RolloutPolicy::FamilyWeighted { retreat_ppm: 2_000_000, recovery_min_of: 5 };
        let _ = bad.pick(&handle, &mut rng);
    }

    /// The serde tag of every rung IS its `as_str()` name (T5 review note: a
    /// T8 implementer reaching for `serde_json::to_string` must emit the same
    /// names the artifacts, the CLI parser, and provenance's `controller`
    /// field use — `#[serde(rename_all = "kebab-case")]` is what makes that
    /// true, and this pin is what keeps it true).
    #[test]
    fn serde_names_match_as_str_for_every_rung() {
        for policy in [
            RolloutPolicy::Uniform,
            RolloutPolicy::RetreatAverse,
            RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
        ] {
            let json = serde_json::to_string(&policy).expect("serializes");
            assert!(
                json.contains(policy.as_str()),
                "serde form {json} must carry the as_str name {:?}",
                policy.as_str(),
            );
            let back: RolloutPolicy = serde_json::from_str(&json).expect("round-trips");
            assert_eq!(back, policy, "round-trip must be identity");
        }
    }
}
