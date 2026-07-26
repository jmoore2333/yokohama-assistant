//! Measurement instruments for the Phase-3.2 arena: interval arithmetic, the
//! per-decision-point volume observation, and the tallies the gates report on
//! (spec `docs/plans/2026-07-25-phase3.2-arena-spec.md` §6).
//!
//! Landed BEFORE the policy it measures — the map's "metrics before policy"
//! ordering, which the plan realizes by sequencing this task ahead of the
//! `RolloutPolicy` that Tasks 5/6/9/10 gate on. A coverage floor or a trigger
//! gate whose VALUE was invented in the same commit as the thing it gates is a
//! gate calibrated against itself; the instrument exists first, and the floors
//! are read off it afterwards.
//!
//! ## The definitions (spec §6, condensed — the metric-definition bullets
//! near-verbatim minus the spec's critic-reference asides; §6's L11-discharge
//! bullet is a recording obligation, not a metric definition, and lives in the
//! spec alone)
//!
//! > **Per-decision-point volume metric** (M3's definition ≡ E1's, term for
//! > term): Σ over `families()` of the family's size — retreat by `retreat_len`
//! > (exact closed form), placement by `placement_upper_len` (an honest **upper
//! > bound**, stated on every row it touches), routes by the length of the
//! > policy's OWN collected enumeration (never a second `family_len` call —
//! > L10), delegated small families by realized count. **Reported per (player
//! > count × phase × policy)**; pooled quantiles only with the upper-bound
//! > caveat attached. The Placement bias direction is stated: it inflates
//! > uniform-like shapes' tails, i.e. biases AGAINST the weighted policy — the
//! > safe direction.
//! >
//! > **Trigger-diversity rates** per player count with 95% Wilson intervals
//! > (never pooled across counts — the pooling is exactly what blinded the
//! > existing fuzz assertion to the 4P collapse).
//! >
//! > **Coverage shares:** retreat-pick share and **nonzero-recovery share** of
//! > applied moves, per policy (the quantity the fuzz coverage floor gates on;
//! > measured before any floor value is set).
//! >
//! > **Per-phase decision-point share** published as the FIRST metric row (the
//! > lever's reach: Movement is 22.9% of 4P decision points under uniform,
//! > 14.9–15.6% under the low-p shapes — E1 held-out `mvpts_frac`; 0.159 is
//! > retreat-averse, a different class), confirming M3's ~77%-single-family
//! > estimate exactly.
//! >
//! > Every figure row carries policy, seed derivation, N, build profile;
//! > observations never ceilings.
//!
//! ## How the code realizes them
//!
//! **The volume metric is a WRAPPER, not a second derivation.** All four of
//! §6's terms are already [`DecisionHandle::size`]'s arithmetic — retreat's
//! closed form, placement's upper bound, routes' memoized enumeration length,
//! delegated families' realized count — so [`decision_volume`] adds exactly two
//! things: the phase the row is keyed by, and the refusal to call an absent
//! decision point a zero. Re-deriving the sum here would be a second arithmetic
//! authority AND would pay the routes enumeration twice, which is the L10
//! property the handle exists to hold.
//!
//! **Reporting is keyed per (player count × phase × policy), and this module
//! carries only the phase.** The other two keys are the CALLER's: a
//! `VolumeObs` knows the decision point it came from, not the run it belongs
//! to. Grouping a run's observations by [`VolumeObs::phase`] is also how §6's
//! FIRST metric row (the per-phase decision-point SHARE) is produced — count
//! the observations per phase, divide by the run's total. No API is needed for
//! it, and none is offered.
//!
//! **Never pool trigger rates across player counts** (§6, and it is the
//! caller's job here too: a [`TriggerTally`] is per cell, and nothing stops a
//! caller feeding it 2P and 4P games — nothing except this paragraph). The
//! reason is a live defect, not a hypothetical: `fuzz_full_games`'
//! `fuzz_500_games_per_player_count` inserts every trigger it sees into ONE
//! pooled `triggers_seen` set across all three player counts and then asserts
//! presence only, so 4P's ChurchTrackFull collapse to 0.35% is structurally
//! invisible to it — the trigger fires at 2P, the set contains it, the
//! assertion passes. A rate with a player count attached is what makes that
//! collapse visible; a pooled rate is what hid it.
//!
//! **An absent measurement is never reported as a measured zero.** Every
//! quantity here whose denominator can be empty returns `Option`:
//! [`decision_volume`] at a state with no families (Recovery and TurnEnd are
//! automatic — the engine advances them without a move, so they are not
//! decision points), [`CoverageTally`]'s shares before any move is observed,
//! [`QuantileSketch::quantiles`] over an empty sketch. `wilson_95(0, 0)` is the
//! same rule in interval form: the whole unit interval, because no games
//! constrain the rate. A caller that wants the safe-direction default writes
//! `.unwrap_or(0.0)` and is visibly choosing it.
//!
//! Figures produced with these instruments carry policy, seed derivation, N and
//! build profile on every row, and are recorded as OBSERVATIONS — never as
//! ceilings (the standing rule; `route_enum`'s label counts are the worked
//! example of why).

use std::collections::BTreeMap;

use crate::game::search_actions::DecisionHandle;
use crate::types::{EndGameTrigger, Move, TurnPhase};

// ---------------------------------------------------------------------------
// Wilson score intervals
// ---------------------------------------------------------------------------

/// The z multiplier for a 95% two-sided normal interval, to the precision the
/// spec's published constants were computed at.
const Z_95: f64 = 1.959964;

/// A 95% Wilson score interval on a binomial rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WilsonInterval {
    /// Lower bound, clamped into `[0, 1]`.
    pub lo: f64,
    /// Upper bound, clamped into `[0, 1]`.
    pub hi: f64,
}

/// The 95% Wilson score interval for `successes` out of `n` (no continuity
/// correction — the form every published figure in the spec and the research
/// notes was computed with).
///
/// Wilson rather than Wald because every rate this branch gates on is small and
/// close to a boundary: at the research notes' uniform 4P ChurchTrackFull rate
/// of 1/500 the Wald interval is `[−0.0019, 0.0059]` — a lower bound below zero
/// on a quantity that cannot be negative — while Wilson's endpoints are the
/// ones that actually solve the score equation `|p̂ − p| = z·√(p(1−p)/n)`, a
/// property the tests pin directly, so a mis-transcribed algebraic
/// simplification cannot pass by matching one hand-checked pair of numbers.
///
/// `n == 0` returns the whole unit interval: no observation constrains the
/// rate, and both a lower-bound gate (`lo > floor`) and an upper-bound gate
/// (`hi < ceiling`) then fail, which is the honest direction for each.
/// `successes > n` is caller misuse; it is `debug_assert`ed and, in release,
/// clamped to `n` rather than allowed to produce a NaN.
pub fn wilson_95(successes: u64, n: u64) -> WilsonInterval {
    debug_assert!(successes <= n, "wilson_95({successes}, {n}): successes cannot exceed n");
    if n == 0 {
        return WilsonInterval { lo: 0.0, hi: 1.0 };
    }
    // Clamped, not trusted: an over-count would otherwise make `p(1 − p)`
    // negative and hand every downstream row a NaN that compares false against
    // any bound it is gated on.
    let p_hat = successes.min(n) as f64 / n as f64;
    // The saturated ends return EXACTLY 0 / EXACTLY 1, not the float residual
    // the closed form produces there (T4 review finding 1: at 0/n the general
    // formula lands lo a few e-17 ABOVE zero for most n — inside [0, 1], so no
    // clamp can see it — which puts the interval's lower bound above the point
    // estimate, a thing a Wilson interval never does mathematically). Wilson's
    // true endpoint at p̂ = 0 is 0 and at p̂ = 1 is 1; return them.
    if successes == 0 {
        let n = n as f64;
        let z2 = Z_95 * Z_95;
        return WilsonInterval { lo: 0.0, hi: (z2 / (n + z2)).clamp(0.0, 1.0) };
    }
    if p_hat == 1.0 {
        let n = n as f64;
        let z2 = Z_95 * Z_95;
        return WilsonInterval { lo: (n / (n + z2)).clamp(0.0, 1.0), hi: 1.0 };
    }
    let n = n as f64;
    let z2 = Z_95 * Z_95;
    let denom = 1.0 + z2 / n;
    let centre = (p_hat + z2 / (2.0 * n)) / denom;
    let half = (Z_95 / denom) * (p_hat * (1.0 - p_hat) / n + z2 / (4.0 * n * n)).sqrt();
    // Wilson's endpoints are algebraically inside [0, 1]; the clamp is against
    // floating-point overshoot OUTSIDE that range. (Residuals INSIDE the range
    // at the saturated ends are handled by the early returns above.)
    WilsonInterval { lo: (centre - half).clamp(0.0, 1.0), hi: (centre + half).clamp(0.0, 1.0) }
}

// ---------------------------------------------------------------------------
// Per-decision-point volume
// ---------------------------------------------------------------------------

/// One decision point's move volume: §6's Σ over `families()`, with the phase
/// its report row is keyed by.
///
/// `total` is an EXACT count of the moves the factored surface would serve
/// unless `has_upper_bound_term` is set, in which case a placement family
/// contributed [`FamilySize::UpperBound`](crate::game::FamilySize::UpperBound)
/// and the total is a strict over-estimate wherever a fee gate punched a hole
/// in the affordable subsets. That caveat travels on every row this observation
/// reaches, and spec §6's BIAS DIRECTION travels with it: "it inflates
/// uniform-like shapes' tails, i.e. biases AGAINST the weighted policy — the
/// safe direction". Safe because the claim the metric is used to support is
/// that the weighted policy visits SMALLER decision points, and the bias works
/// against that claim rather than for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeObs {
    /// The turn phase this decision point was in — §6's per-phase reporting
    /// key, and the input to the per-phase decision-point SHARE row.
    pub phase: TurnPhase,
    /// Σ over the decision point's families of each family's size.
    pub total: u64,
    /// Whether an upper-bounded (placement) term entered the sum.
    pub has_upper_bound_term: bool,
}

/// §6's volume metric at one decision point, or `None` when `handle` was built
/// at a state that is not a decision point at all.
///
/// `None` is the T2 review's binding finding made typed: `DecisionHandle::volume`
/// answers `(0, false)` at a state with NO families — Recovery and TurnEnd are
/// automatic phases the engine advances without a move — and a metric that
/// recorded that as a measured zero would drag every quantile and mean toward
/// zero with points that were never decisions. A `None` is skipped by a
/// caller's `if let`; a `Some(0)` cannot be distinguished from it once summed.
///
/// A `Some` whose `total` is 0 is therefore a type-level distinction, not a
/// state known to be reachable: `families()`' Movement arm emits routes AND
/// retreat unconditionally, and `retreat_len` is an empty PRODUCT (1 even over
/// an empty board), so no organic decision point sums to zero — the T4 review
/// measured ~175k organic points with a realized minimum of 1. The type
/// separates measured-zero from not-a-decision-point anyway; the caller does
/// not have to.
///
/// Pays no enumeration the handle has not already paid: the routes term is
/// `handle`'s memoized `Vec`, so calling this after the policy has sampled
/// costs zero route DFSes (pinned by `decision_volume_pays_no_extra_enumeration`).
pub fn decision_volume(handle: &DecisionHandle) -> Option<VolumeObs> {
    if handle.families().is_empty() {
        return None;
    }
    let (total, has_upper_bound_term) = handle.volume();
    Some(VolumeObs { phase: handle.turn_phase(), total, has_upper_bound_term })
}

// ---------------------------------------------------------------------------
// Applied-move coverage
// ---------------------------------------------------------------------------

/// Counts of the applied-move classes §6's coverage shares are taken over, and
/// §7's fuzz floors gate on.
///
/// Every counter is over APPLIED moves (what a driver actually played), not
/// over legal ones — a policy that could have retreated and never did is
/// exactly the coverage hole these shares exist to expose (spec §7: retreat
/// aversion applies ZERO nonzero-recovery retreats, while uniform under-covers
/// area actions ~4× and builds 13–20× — the two policies cover disjoint halves
/// of the apply surface, which is why 3.2 ADDS weighted fuzz cells rather than
/// replacing the uniform ones).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoverageTally {
    // Private with read accessors (T4 review finding 7's class): `observe` is
    // the ONLY writer, so a caller cannot invent the denominator a §7 floor
    // gates on, and the counts cannot desync from each other.
    applied: u64,
    retreat_picks: u64,
    nonzero_recovery_retreats: u64,
    area_actions: u64,
    builds: u64,
}

impl CoverageTally {
    /// Classify one applied move.
    ///
    /// The match names only the classes §6 reports and falls through for the
    /// rest — deliberately a `_` arm, unlike `move_legality`'s exhaustive one:
    /// a new `Move` variant is a move that has no share row, not a compile
    /// error, and inventing a row for it silently would be the worse failure.
    pub fn observe(&mut self, mv: &Move) {
        self.applied += 1;
        // EXHAUSTIVE over `Move`, the repo's convention (`is_move_legal`,
        // `trigger_name`): a future build-class or retreat-class variant
        // falling through a `_` arm would silently under-count the exact
        // shares §7's floors gate on. The "counted nowhere" arms are a
        // one-token acknowledgment per variant, so a new variant is a compile
        // error and a deliberate classification, never an accident (T4 review
        // finding 5).
        match mv {
            Move::ReturnPresidentToHand { recover } => {
                self.retreat_picks += 1;
                // A hand-built map can carry a 0 count (the constructor drops
                // them, `apply` treats them as nothing recovered), so the test
                // is on the SUM, never on the map being non-empty.
                if recover.values().any(|&count| count > 0) {
                    self.nonzero_recovery_retreats += 1;
                }
            }
            Move::ExecuteAreaAction { .. } => self.area_actions += 1,
            Move::BuildShophouse { .. } | Move::BuildTradingHouse => self.builds += 1,
            // Counted nowhere, deliberately — movement/placement volume is the
            // volume metric's business, and the remaining variants carry no
            // coverage share the spec names.
            Move::UseMercenary { .. }
            | Move::UseUniversity
            | Move::UseSpinningMill
            | Move::FulfillAchievement { .. }
            | Move::FulfillOrder { .. }
            | Move::EndAdditionalPhase
            | Move::PlaceThreeAreas { .. }
            | Move::PlaceTwoSameArea { .. }
            | Move::MovePresident { .. }
            | Move::PlacePresident { .. }
            | Move::Take5PowerToken
            | Move::Skip5PowerBonus
            | Move::SkipBuilding
            | Move::ChooseAnyGood { .. }
            | Move::ChooseWarehouseTake { .. }
            | Move::DeclineWarehouseTake
            | Move::ChooseAssistantMove { .. }
            | Move::DeclineAssistantMove
            | Move::ChooseChurchSpace { .. }
            | Move::ChooseCustomsSpace { .. }
            | Move::ChooseRecoveryLeave { .. }
            | Move::ChooseStationPlace { .. } => {}
        }
    }

    /// Share of applied moves that were retreats, or `None` before any move is
    /// observed (no denominator, so no share — see the module doc's rule).
    pub fn retreat_pick_share(&self) -> Option<f64> {
        self.share(self.retreat_picks)
    }

    /// Share of applied moves that were retreats recovering at least one
    /// assistant — the quantity §7's fuzz coverage floor gates on.
    pub fn nonzero_recovery_share(&self) -> Option<f64> {
        self.share(self.nonzero_recovery_retreats)
    }

    /// Share of applied moves that were area actions.
    pub fn area_action_share(&self) -> Option<f64> {
        self.share(self.area_actions)
    }

    /// Share of applied moves that were builds.
    pub fn build_share(&self) -> Option<f64> {
        self.share(self.builds)
    }

    fn share(&self, count: u64) -> Option<f64> {
        (self.applied > 0).then(|| count as f64 / self.applied as f64)
    }

    /// Every move observed — the denominator of every share above.
    pub fn applied(&self) -> u64 {
        self.applied
    }

    /// `ReturnPresidentToHand` moves, whatever they recovered.
    pub fn retreat_picks(&self) -> u64 {
        self.retreat_picks
    }

    /// `ReturnPresidentToHand` moves that recovered at least one assistant —
    /// the interior of the retreat family, which retreat-aversion never
    /// reaches.
    pub fn nonzero_recovery_retreats(&self) -> u64 {
        self.nonzero_recovery_retreats
    }

    /// `ExecuteAreaAction` moves.
    pub fn area_actions(&self) -> u64 {
        self.area_actions
    }

    /// `BuildShophouse` / `BuildTradingHouse` moves. `SkipBuilding` is not a
    /// build and is not counted.
    pub fn builds(&self) -> u64 {
        self.builds
    }
}

// ---------------------------------------------------------------------------
// End-game trigger diversity
// ---------------------------------------------------------------------------

/// The stable report key for an end-game trigger: its variant name, with the
/// player payload deliberately dropped.
///
/// Trigger DIVERSITY is a property of the game's ending conditions, not of who
/// ended it, so `TradingHousesComplete(0)` and `TradingHousesComplete(3)` are
/// one row. The match is exhaustive with no `_` arm, so adding an
/// `EndGameTrigger` variant is a compile error here rather than a silently
/// unreported trigger (the same discipline `move_legality`'s `Move` match
/// carries).
pub const fn trigger_name(trigger: &EndGameTrigger) -> &'static str {
    match trigger {
        EndGameTrigger::TradingHousesComplete(_) => "TradingHousesComplete",
        EndGameTrigger::ShophousesComplete(_) => "ShophousesComplete",
        EndGameTrigger::OrderDeckEmpty => "OrderDeckEmpty",
        EndGameTrigger::ChurchTrackFull => "ChurchTrackFull",
        EndGameTrigger::CustomsTrackFull => "CustomsTrackFull",
    }
}

/// Per-trigger counts over a set of games, with the game count that is every
/// rate's denominator.
///
/// ONE PLAYER COUNT PER TALLY. Nothing in the type enforces it — a tally is a
/// counter, and the player count lives on the games the caller feeds it — so
/// the rule is stated here and belongs to the caller: pooling 2P and 4P games
/// into one tally reproduces exactly the blindness described in the module doc,
/// where a trigger that is healthy at 2P hides its 4P collapse.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TriggerTally {
    counts: BTreeMap<&'static str, u64>,
    // Private with a getter (T4 review finding 7): a writable denominator on
    // the exact type the R1 gate reads lets a caller invent the rate. `observe`
    // is the only writer.
    games: u64,
}

impl TriggerTally {
    /// Observe one COMPLETED game's end-game trigger.
    ///
    /// `None` is a game that completed with no trigger recorded; it counts in
    /// `games` because it is a genuine observation that no trigger fired. A
    /// game that did NOT complete (turn cap, driver error — spec §4's failure
    /// taxonomy) must not be observed at all: it is not an observation of the
    /// trigger distribution, and counting it would deflate every rate.
    pub fn observe(&mut self, trigger: Option<&EndGameTrigger>) {
        self.games += 1;
        if let Some(trigger) = trigger {
            *self.counts.entry(trigger_name(trigger)).or_insert(0) += 1;
        }
    }

    /// The count and 95% Wilson interval for one trigger name over the games
    /// observed. An unseen name is `0` out of `games` — a real measurement of
    /// "never fired here", whose interval width says how much that is worth.
    pub fn rate(&self, name: &str) -> (u64, WilsonInterval) {
        let seen = self.counts.get(name).copied().unwrap_or(0);
        (seen, wilson_95(seen, self.games))
    }

    /// Games observed — the denominator of every rate. Read-only: `observe`
    /// is the only writer (T4 review finding 7).
    pub fn games(&self) -> u64 {
        self.games
    }

    /// Every trigger name seen, with its count, in name order.
    pub fn counts(&self) -> &BTreeMap<&'static str, u64> {
        &self.counts
    }
}

// ---------------------------------------------------------------------------
// Quantiles
// ---------------------------------------------------------------------------

/// The order statistics [`QuantileSketch`] reports, with the N they came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantiles {
    /// Observations the quantiles were taken over.
    pub n: u64,
    /// Nearest-rank median.
    pub median: u64,
    /// Nearest-rank 90th percentile.
    pub p90: u64,
    /// Nearest-rank 99th percentile.
    pub p99: u64,
    /// Largest observation.
    pub max: u64,
}

/// An exact quantile sketch: it keeps every observation.
///
/// "Sketch" names the role, not an approximation — there is no sampling or
/// bucketing here. Cost, with its conditions (T4 review finding 4): uniform 4P
/// self-play runs ~9,900 decision points per game (measured, uncapped), so a
/// 500-game uniform cell holds ~5M `u64`s ≈ 40 MB — and `quantiles()` clones
/// before sorting, so peak is ~double that. Weighted-policy cells are ~3×
/// shorter. Tens of megabytes at the largest uniform cells, transient, and
/// still cheaper than defending an approximation's own error bars inside a
/// figure row that already carries an upper-bound caveat; revisit only if a
/// cell's scale grows past that.
///
/// Quantiles are NEAREST-RANK — the observation at rank `⌈p·n⌉`, never an
/// interpolation between two of them. An interpolated "3.5 moves" is a number
/// no decision point ever had, and these rows are read as tail sizes.
#[derive(Debug, Clone, Default)]
pub struct QuantileSketch {
    values: Vec<u64>,
}

impl QuantileSketch {
    /// Record one observation.
    pub fn observe(&mut self, value: u64) {
        self.values.push(value);
    }

    /// Observations recorded.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether nothing has been recorded yet.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The four reported order statistics, or `None` over an empty sketch.
    /// Sorts once for all four.
    pub fn quantiles(&self) -> Option<Quantiles> {
        let sorted = self.sorted()?;
        Some(Quantiles {
            n: sorted.len() as u64,
            median: sorted[nearest_rank_index(sorted.len(), 1, 2)],
            p90: sorted[nearest_rank_index(sorted.len(), 9, 10)],
            p99: sorted[nearest_rank_index(sorted.len(), 99, 100)],
            max: sorted[sorted.len() - 1],
        })
    }

    /// One arbitrary nearest-rank quantile `num/den` (e.g. `95, 100` for p95),
    /// or `None` over an empty sketch. Sorts on each call —
    /// [`quantiles`](Self::quantiles) is the batch form.
    pub fn quantile(&self, num: u64, den: u64) -> Option<u64> {
        let sorted = self.sorted()?;
        Some(sorted[nearest_rank_index(sorted.len(), num, den)])
    }

    /// The observations in ascending order, or `None` when there are none.
    fn sorted(&self) -> Option<Vec<u64>> {
        if self.values.is_empty() {
            return None;
        }
        let mut sorted = self.values.clone();
        sorted.sort_unstable();
        Some(sorted)
    }
}

/// The 0-based index of the nearest-rank quantile `num/den` over `n > 0`
/// sorted observations: rank `⌈n·num/den⌉`, minus one for 0-basing.
///
/// The ceiling is an INTEGER ceil-div, not `(p * n as f64).ceil()`. The two
/// agree on every quantile this module reports at every N tried, so no test
/// here discriminates them — the integer form is chosen so that the question
/// never has to be re-audited at a new N. A `p·n` that is mathematically
/// integral and lands one ULP high pushes the rank a whole observation up, and
/// a tail row that is silently one observation too optimistic is not a failure
/// anything downstream would notice.
fn nearest_rank_index(n: usize, num: u64, den: u64) -> usize {
    debug_assert!(n > 0 && den > 0 && num <= den, "nearest_rank_index({n}, {num}/{den})");
    let n = n as u64;
    let rank = (n * num).div_ceil(den).max(1);
    (rank.min(n) - 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::search_actions::tests::{
        at_phase, handle_fee_gated_placement_state, handle_movement_state,
    };
    use crate::game::{
        family_len, route_dfs_count, route_dfs_reset, DecisionHandle, MoveFamily, Rules,
    };
    use crate::types::{AreaAction, AreaId, PlayerId};

    /// Both endpoints of a Wilson interval solve the score equation
    /// `|p̂ − p| = z·√(p(1−p)/n)` — the DEFINING property, checked independently
    /// of the closed form that computes it.
    fn assert_solves_score_equation(successes: u64, n: u64) {
        let p_hat = successes as f64 / n as f64;
        let interval = wilson_95(successes, n);
        for (label, p) in [("lo", interval.lo), ("hi", interval.hi)] {
            let lhs = (p_hat - p).abs();
            let rhs = Z_95 * (p * (1.0 - p) / n as f64).sqrt();
            assert!(
                (lhs - rhs).abs() < 1e-12,
                "{successes}/{n} {label}={p}: |p̂ − p| = {lhs} must equal z·√(p(1−p)/n) = {rhs}",
            );
        }
    }

    fn assert_close(got: f64, want: f64, what: &str) {
        assert!((got - want).abs() < 5e-5, "{what}: got {got}, want {want} (4-decimal tolerance)");
    }

    /// The textbook 95% Wilson case, the two constants the spec's gates cite,
    /// and the two the research notes publish — five independent pairs, none of
    /// which this module's arithmetic was fitted to — plus the score-equation
    /// identity, which no transcription of a wrong closed form satisfies.
    #[test]
    fn wilson_95_matches_the_textbook_case() {
        // Textbook: 10/100 at 95%, the uncorrected Wilson score interval.
        let textbook = wilson_95(10, 100);
        assert_close(textbook.lo, 0.0552, "textbook 10/100 lo");
        assert_close(textbook.hi, 0.1744, "textbook 10/100 hi");

        // Spec §0-R1's FROZEN uniform 4P ChurchTrackFull ceiling: 7/2000 →
        // [0.17%, 0.72%]. The 0.0072 hi is the constant Task 10's gate asserts
        // against, so this pin is that gate's arithmetic.
        let frozen = wilson_95(7, 2000);
        assert_close(frozen.lo, 0.0017, "spec 7/2000 lo");
        assert_close(frozen.hi, 0.0072, "spec 7/2000 hi");

        // The shipped weighted shape's held-out 4P rate: 74/2000 → [2.96%,
        // 4.62%], the interval whose disjointness from the frozen ceiling is
        // the whole R1 argument.
        let weighted = wilson_95(74, 2000);
        assert_close(weighted.lo, 0.0296, "spec 74/2000 lo");
        assert_close(weighted.hi, 0.0462, "spec 74/2000 hi");

        // Research notes §1: uniform 4P 1/500 → [0.04%, 1.12%]; retreat-averse
        // 22/500 → [2.92%, 6.57%].
        let uniform_500 = wilson_95(1, 500);
        assert_close(uniform_500.lo, 0.0004, "notes 1/500 lo");
        assert_close(uniform_500.hi, 0.0112, "notes 1/500 hi");
        let averse_500 = wilson_95(22, 500);
        assert_close(averse_500.lo, 0.0292, "notes 22/500 lo");
        assert_close(averse_500.hi, 0.0657, "notes 22/500 hi");

        // The defining property, at both a small-rate and a mid-rate case.
        assert_solves_score_equation(10, 100);
        assert_solves_score_equation(7, 2000);

        // Boundaries: a zero count is a real measurement with a one-sided
        // reach; a saturated count is its mirror; no observation at all is the
        // whole unit interval, which fails a floor gate AND a ceiling gate.
        // The exact-end equalities hold BY CONSTRUCTION (the saturated-end
        // early returns), not by accident of n: before the T4-review fix,
        // 0/3's lo was ~5.6e-17 and 200/200's hi was 0.9999999999999999 —
        // inside [0, 1], invisible to the clamp, and putting lo above the
        // point estimate. The small-n cases below are the ones the general
        // closed form got wrong.
        let none_seen = wilson_95(0, 100);
        assert_eq!(none_seen.lo, 0.0, "0/100 lower bound is exactly zero");
        assert_close(none_seen.hi, 0.0370, "0/100 hi");
        let all_seen = wilson_95(100, 100);
        assert_close(all_seen.lo, 0.9630, "100/100 lo");
        assert_eq!(all_seen.hi, 1.0, "100/100 upper bound is exactly one");
        assert_eq!(wilson_95(0, 3).lo, 0.0, "0/3 lower bound is exactly zero (was 5.6e-17)");
        assert_eq!(wilson_95(4, 4).hi, 1.0, "4/4 upper bound is exactly one (was 1-1ulp)");
        assert_eq!(wilson_95(5000, 5000).hi, 1.0, "5000/5000 upper bound is exactly one");
        assert_eq!(wilson_95(0, 0), WilsonInterval { lo: 0.0, hi: 1.0 }, "no games, no constraint");
    }

    /// The placement upper-bound term must reach the report row as a FLAG, and
    /// a Movement point must not raise it.
    ///
    /// Anti-vacuity on the placement side: the fixture's bound is asserted
    /// STRICTLY above the realized move count here, so a future fixture change
    /// that removed the fee gate would fail loudly instead of leaving a flag
    /// that is technically set but measuring nothing.
    #[test]
    fn decision_volume_flags_the_placement_upper_bound_term() {
        let rules = Rules::standard();

        let placement = handle_fee_gated_placement_state();
        let handle = DecisionHandle::new(&placement, &rules);
        let realized = handle.iter(&MoveFamily::Placement).count() as u64;
        let obs = decision_volume(&handle).expect("a placement point is a decision point");
        assert_eq!(obs.phase, TurnPhase::Placement);
        assert!(obs.has_upper_bound_term, "a placement family upper-bounds the sum");
        assert!(
            obs.total > realized,
            "anti-vacuity: the fee gate must make the bound strict ({} vs {realized})",
            obs.total,
        );

        let movement = handle_movement_state();
        let handle = DecisionHandle::new(&movement, &rules);
        let obs = decision_volume(&handle).expect("a movement point is a decision point");
        assert_eq!(obs.phase, TurnPhase::Movement);
        assert!(!obs.has_upper_bound_term, "Movement has no upper-bounded family");
        let want: u64 = handle
            .families()
            .iter()
            .map(|f| family_len(&movement, &rules, f) as u64)
            .sum();
        assert_eq!(obs.total, want, "the total is §6's Σ over families()");
        assert!(obs.total > 1, "anti-vacuity: the fixture must serve a real family sum");
    }

    /// A state with no families is NOT a decision point, and must not be
    /// reported as one measuring zero (T2 review's binding finding:
    /// `DecisionHandle::volume` answers `(0, false)` there).
    ///
    /// MUTATION-PROVEN (dropping `decision_volume`'s empty-families guard makes
    /// it answer `Some(VolumeObs { phase: Recovery, total: 0, .. })` and this
    /// test fails); reverted by inverse edit.
    #[test]
    fn decision_volume_refuses_a_state_with_no_families() {
        let rules = Rules::standard();
        for phase in [TurnPhase::Recovery, TurnPhase::TurnEnd] {
            let state = at_phase(phase);
            let handle = DecisionHandle::new(&state, &rules);
            assert!(handle.families().is_empty(), "{phase:?} dispatches no family");
            assert_eq!(
                handle.volume(),
                (0, false),
                "{phase:?}: the handle's raw answer is the zero this must not report",
            );
            assert_eq!(decision_volume(&handle), None, "{phase:?} is not a decision point");
        }
    }

    /// The volume metric is the SECOND consumer of the routes enumeration the
    /// policy already paid for — so reading it must cost zero further route
    /// DFSes. Without this, "metrics before policy" would silently double every
    /// Movement decision point's enumeration cost.
    ///
    /// This test is a UNIT test rather than an integration one for the same
    /// reason `tests/l10_enumeration_pin.rs` is `#![cfg(debug_assertions)]`:
    /// the counter is `cfg(any(test, debug_assertions))`, and `cfg(test)` is
    /// FALSE for the library as an integration test links it — so an
    /// integration-test twin of this would read the unconditional 0-returning
    /// stub in `--release` and pass vacuously. Here `cfg(test)` is TRUE, so the
    /// counter is live in BOTH profiles and the anti-vacuity assertion below
    /// (the first read really costs 1) can be asserted rather than assumed.
    ///
    /// MUTATION-PROVEN (dropping `MovementRoutes` from `search_actions`'
    /// `is_memoized` takes the count to 2 and this test fails); reverted by
    /// inverse edit.
    #[test]
    fn decision_volume_pays_no_extra_enumeration() {
        let rules = Rules::standard();
        let movement = handle_movement_state();

        route_dfs_reset();
        let handle = DecisionHandle::new(&movement, &rules);
        assert_eq!(route_dfs_count(), 0, "construction enumerates nothing");

        // The policy's read: one enumeration.
        let served: Vec<Move> = handle.iter(&MoveFamily::MovementRoutes).collect();
        assert!(!served.is_empty(), "anti-vacuity: the fixture must serve real routes");
        let after_policy = route_dfs_count();
        assert_eq!(after_policy, 1, "anti-vacuity: the first read really pays a DFS");

        // The metric's read: none.
        let obs = decision_volume(&handle).expect("a movement point is a decision point");
        assert!(obs.total >= served.len() as u64);
        assert_eq!(
            route_dfs_count(),
            after_policy,
            "the metric must reuse the handle's memoized enumeration, not re-enumerate",
        );
    }

    /// Applied-move classification, including the three negatives that decide
    /// the shares' meaning: a retreat recovering nothing is a retreat but not a
    /// nonzero recovery, a hand-built zero-count entry is still nothing
    /// recovered, and `SkipBuilding` is not a build.
    #[test]
    fn coverage_tally_classifies_the_three_shares() {
        let mut tally = CoverageTally::default();
        let area = AreaId::new(1);

        let empty_retreat = Move::ReturnPresidentToHand { recover: BTreeMap::new() };
        let zero_count_retreat =
            Move::ReturnPresidentToHand { recover: BTreeMap::from([(area, 0)]) };
        let real_retreat = Move::ReturnPresidentToHand { recover: BTreeMap::from([(area, 2)]) };
        let moves = [
            empty_retreat,
            zero_count_retreat,
            real_retreat,
            Move::ExecuteAreaAction { action: AreaAction::Bank },
            Move::BuildShophouse { slot: 0 },
            Move::BuildTradingHouse,
            Move::SkipBuilding,
            Move::PlaceThreeAreas { placements: vec![area] },
        ];
        for mv in &moves {
            tally.observe(mv);
        }

        assert_eq!(tally.applied(), 8, "every observed move counts in the denominator");
        assert_eq!(tally.retreat_picks(), 3, "all three ReturnPresidentToHand moves");
        assert_eq!(tally.nonzero_recovery_retreats(), 1, "only the Σrecover > 0 one");
        assert_eq!(tally.area_actions(), 1);
        assert_eq!(tally.builds(), 2, "shophouse + trading house; SkipBuilding is not a build");

        assert_close(tally.retreat_pick_share().unwrap(), 3.0 / 8.0, "retreat share");
        assert_close(tally.nonzero_recovery_share().unwrap(), 1.0 / 8.0, "nonzero recovery share");
        assert_close(tally.area_action_share().unwrap(), 1.0 / 8.0, "area action share");
        assert_close(tally.build_share().unwrap(), 2.0 / 8.0, "build share");

        let fresh = CoverageTally::default();
        assert_eq!(fresh.nonzero_recovery_share(), None, "no moves, no share");
    }

    /// Trigger rates key by variant name (payload dropped), count every
    /// observed game in the denominator including trigger-less ones, and report
    /// an unseen trigger as a real 0-out-of-N rather than an absence.
    #[test]
    fn trigger_tally_keys_by_discriminant_name() {
        let mut tally = TriggerTally::default();
        tally.observe(Some(&EndGameTrigger::ChurchTrackFull));
        tally.observe(Some(&EndGameTrigger::TradingHousesComplete(PlayerId::new(0))));
        tally.observe(Some(&EndGameTrigger::TradingHousesComplete(PlayerId::new(3))));
        tally.observe(None);

        assert_eq!(tally.games(), 4, "a trigger-less completed game is still an observation");
        assert_eq!(tally.counts().get("TradingHousesComplete"), Some(&2), "payload dropped");
        assert_eq!(tally.counts().get("ChurchTrackFull"), Some(&1));
        assert_eq!(tally.counts().len(), 2, "only the triggers actually seen get rows");

        let (seen, interval) = tally.rate("ChurchTrackFull");
        assert_eq!(seen, 1);
        assert_eq!(interval, wilson_95(1, 4));
        let (unseen, unseen_interval) = tally.rate("CustomsTrackFull");
        assert_eq!(unseen, 0, "never fired is a measured zero, not a missing row");
        assert_eq!(unseen_interval, wilson_95(0, 4));

        assert_eq!(trigger_name(&EndGameTrigger::ShophousesComplete(PlayerId::new(1))), "ShophousesComplete");
        assert_eq!(trigger_name(&EndGameTrigger::OrderDeckEmpty), "OrderDeckEmpty");
        assert_eq!(trigger_name(&EndGameTrigger::CustomsTrackFull), "CustomsTrackFull");
    }

    /// Nearest rank `⌈p·n⌉`, no interpolation, on fixed vectors — including an
    /// unsorted input and an n=1 sketch, where every quantile is the single
    /// observation.
    #[test]
    fn quantiles_are_nearest_rank() {
        assert_eq!(QuantileSketch::default().quantiles(), None, "no observations, no quantiles");

        // 1..=10: median rank ⌈5⌉ = 5 → 5; p90 rank ⌈9⌉ = 9 → 9; p99 rank
        // ⌈9.9⌉ = 10 → 10.
        let mut ten = QuantileSketch::default();
        for v in 1..=10u64 {
            ten.observe(v);
        }
        assert_eq!(ten.len(), 10);
        assert_eq!(
            ten.quantiles(),
            Some(Quantiles { n: 10, median: 5, p90: 9, p99: 10, max: 10 }),
        );

        // Unsorted input, n = 4: sorted [1, 3, 5, 9]; median rank ⌈2⌉ = 2 → 3;
        // p90 rank ⌈3.6⌉ = 4 → 9; p99 rank ⌈3.96⌉ = 4 → 9.
        let mut four = QuantileSketch::default();
        for v in [5u64, 1, 9, 3] {
            four.observe(v);
        }
        assert_eq!(four.quantiles(), Some(Quantiles { n: 4, median: 3, p90: 9, p99: 9, max: 9 }));

        // n = 1: every rank is 1.
        let mut one = QuantileSketch::default();
        one.observe(7);
        assert_eq!(one.quantiles(), Some(Quantiles { n: 1, median: 7, p90: 7, p99: 7, max: 7 }));

        // The arbitrary-quantile form agrees with the reported four, and p100
        // is the max.
        assert_eq!(four.quantile(1, 2), Some(3));
        assert_eq!(four.quantile(9, 10), Some(9));
        assert_eq!(ten.quantile(1, 1), Some(10), "p100 is the largest observation");
        assert_eq!(QuantileSketch::default().quantile(1, 2), None);
    }
}
