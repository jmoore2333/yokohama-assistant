//! Evaluator v1: a stage-interpolated linear evaluation of ONE seat's view
//! (Phase 3.3, spec §2.1-§2.3 and §0.3(a)).
//!
//! # The honesty mechanism (spec §2.1)
//!
//! Everything here is a function of a [`PlayerView`] plus static [`Rules`].
//! That is the SIGNATURE-level guarantee and it is what callers rely on: an
//! evaluator handed a seat's view cannot price the order deck's future order,
//! the boxed pile, an opponent's unfulfilled starting order, or the setup seed,
//! because none of them is in the argument. The signature is not a PROOF,
//! though — this module lives inside `yokohama-core`, so nothing in the
//! compiler stops it from naming any type in the crate. The other half is
//! therefore an executable guard: `tests/eval_core.rs` `include_str!`s this
//! file and refuses the forbidden identifiers, plus any reach into the engine's
//! own `game` modules outside a one-entry allowlist ([`Rules`], for the
//! registries §2.3's deck clock and T2's mirrors read). Spec §2.1 says so in
//! those words; the guard's doc repeats that it is a guard and not a compiler
//! fact. NOTE FOR ANY EDIT HERE: the guard scans for OCCURRENCES, so even a
//! prose mention of a forbidden identifier or of a non-allowlisted engine path
//! fails it. That is deliberate — an inline path is a reach-around with no
//! `use` line to catch — and it is why this doc talks about those things
//! obliquely.
//!
//! # Three rules, stated once
//!
//! **1. Relative form with signed orientation (§2.2).** Every term is
//! `w(p)·v_viewer − max over opponents of w(p)·v_opponent`. The SIGNED weight
//! IS the orientation the spec asks for: multiplying before the maximum makes a
//! negative-weight feature pick the opponent with the SMALLEST raw value —
//! that opponent's own best case — which is exactly "orient higher-is-better
//! before the max" without a second sign table to keep in step. See
//! [`opponent_extreme`]. Unexercised at 2P, where the maximum is over one
//! opponent and any orientation agrees.
//!
//! **2. Fixed-point stage interpolation (§2.3).** Progress `p` is a u32 in
//! PARTS PER MILLION, the maximum over the end-game clocks a view can compute
//! ([`clock_ppm`]), and `w(p) = w_early + ((w_late − w_early)·p) / 1_000_000`
//! in `i64` with Rust's ROUND-TOWARD-ZERO division ([`interpolate`]). No
//! floats: the same weights must come out on every machine, and `explain`
//! reports exactly the rounded `w(p)` that `score` used. Hard stage switches
//! were rejected at spec time (~35% mis-assignment at the measured proxy
//! accuracy).
//!
//! **3. Two paths, not one (§0.3(a)).** [`score`] is the hot path and
//! allocates NOTHING; [`explain`] is a separate term-by-term reconstruction
//! that allocates its `Vec`. The exact-sum property (`explain().total ==
//! score()`) only has content because of that separation — a `score` defined
//! as `explain().total` would discharge it as a tautology and pay an
//! allocation per search node. Both halves are pinned in `tests/eval_core.rs`.
//!
//! # What v1 does NOT do yet
//!
//! The five score-structure features (§2.2's country/tech/church/customs bonus
//! re-derivations and `achievement_progress_sum`) are DECLARED here, weighted
//! zero in both vectors, and compute zero: T2 lands the view-side
//! re-derivations plus the drift pin against `game/scoring.rs`. Every weight in
//! [`WeightTable::PROVISIONAL_V1`] is hand-set and PROVISIONAL until T4's
//! measured freeze. Terminal states are not special-cased here either (§2.6 is
//! T2's, and it needs the same re-derivations).

use crate::game::rules::Rules;
use crate::types::{PlayerView, VisiblePlayer};

/// Bumped on ANY behavior-affecting change to the evaluator — the feature list,
/// the relative form, the stage shape, or the weight tables (T4's freeze is a
/// bump). Spec §0.5/L12: a weight change makes previously-recorded figures for
/// EVERY controller that embeds this evaluator non-comparable, which is what
/// `SeatProvenance::version` means; this const rides the params map so the
/// attribution is finer than the controller name alone.
pub const EVAL_IMPL_VERSION: u32 = 1;

/// Full progress, in the ppm units [`progress_ppm`] reports.
pub const PROGRESS_FULL_PPM: u32 = 1_000_000;

/// How many end-game clocks [`clock_ppm`] reports.
pub const CLOCK_COUNT: usize = 4;

// ---------------------------------------------------------------------------
// Features
// ---------------------------------------------------------------------------

/// The fifteen relative features of evaluator v1, in THE canonical order
/// (spec §2.2's list, read top to bottom). The order is load-bearing: it
/// indexes both weight vectors, it is the order `explain` reports terms in,
/// and T4's probe reports per-feature figures against it. Adding a feature
/// means adding it to [`Feature::ALL`] and to both vectors — the array lengths
/// make that a compile error rather than a silent zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Feature {
    /// Running victory points. R1-1: AUC 0.5625 early → 0.8612 late — the
    /// late-game separator, and deliberately not an early one.
    VictoryPoints,
    /// The country-set end bonus re-derived from the view. R1-3 measured it as
    /// the best EARLY separator (0.6212). Staged — see the module doc.
    CountryBonusNow,
    /// The technology end bonus re-derived from the view (R1-10's cluster
    /// winner over a raw tech count). Staged.
    TechBonusNow,
    /// The church majority bonus re-derived from the view, dummy competitor
    /// included (finding D1). Staged.
    ChurchBonusNow,
    /// The customs majority bonus re-derived from the view. Staged.
    CustomsBonusNow,
    /// Assistants in hand — playable tempo. R1-4: 0.6026 early.
    AssistantsInHand,
    /// Assistants stranded in the warehouse. R1-4: 0.3932 early and the
    /// largest standardized weight in every multivariate fit — NEGATIVE.
    AssistantsInWarehouse,
    /// Assistants on the main board: presence, action power, and fee income.
    AssistantsOnBoard,
    /// Shophouses built (of 8) — VP and one of the two building end triggers.
    ShophousesBuilt,
    /// Trading houses built (of 4) — the scarcer, higher-reward building.
    TradingHousesBuilt,
    /// 5-Power tokens collected.
    FivePowerTokens,
    /// Imported goods — the customs track's currency.
    ImportedGoods,
    /// Progress toward the three Achievement cards. Staged.
    AchievementProgressSum,
    /// Foreign mercenaries held (used and unused).
    MercenariesTotal,
    /// Order cards in hand. R1-7: 0.4745/0.4171/0.3753 — NEGATIVE and growing
    /// (an unfulfilled order is a commitment, not an asset).
    HandSize,
}

impl Feature {
    /// The canonical order. `ALL[f.index()] == f` for every feature.
    pub const ALL: [Feature; 15] = [
        Feature::VictoryPoints,
        Feature::CountryBonusNow,
        Feature::TechBonusNow,
        Feature::ChurchBonusNow,
        Feature::CustomsBonusNow,
        Feature::AssistantsInHand,
        Feature::AssistantsInWarehouse,
        Feature::AssistantsOnBoard,
        Feature::ShophousesBuilt,
        Feature::TradingHousesBuilt,
        Feature::FivePowerTokens,
        Feature::ImportedGoods,
        Feature::AchievementProgressSum,
        Feature::MercenariesTotal,
        Feature::HandSize,
    ];

    /// Length of [`Feature::ALL`] and of each weight vector.
    pub const COUNT: usize = Feature::ALL.len();

    /// Position in [`Feature::ALL`] — the weight-vector index.
    ///
    /// Written as an exhaustive match rather than `as usize` on a discriminant
    /// so that reordering the enum without reordering `ALL` cannot silently
    /// mis-index a weight (the two would disagree and the round-trip test
    /// fails).
    pub fn index(self) -> usize {
        match self {
            Feature::VictoryPoints => 0,
            Feature::CountryBonusNow => 1,
            Feature::TechBonusNow => 2,
            Feature::ChurchBonusNow => 3,
            Feature::CustomsBonusNow => 4,
            Feature::AssistantsInHand => 5,
            Feature::AssistantsInWarehouse => 6,
            Feature::AssistantsOnBoard => 7,
            Feature::ShophousesBuilt => 8,
            Feature::TradingHousesBuilt => 9,
            Feature::FivePowerTokens => 10,
            Feature::ImportedGoods => 11,
            Feature::AchievementProgressSum => 12,
            Feature::MercenariesTotal => 13,
            Feature::HandSize => 14,
        }
    }

    /// The stable snake_case name: report columns, explanation rendering, and
    /// T4's probe output all come from here so none of them is hand-typed.
    pub fn name(self) -> &'static str {
        match self {
            Feature::VictoryPoints => "victory_points",
            Feature::CountryBonusNow => "country_bonus_now",
            Feature::TechBonusNow => "tech_bonus_now",
            Feature::ChurchBonusNow => "church_bonus_now",
            Feature::CustomsBonusNow => "customs_bonus_now",
            Feature::AssistantsInHand => "assistants_in_hand",
            Feature::AssistantsInWarehouse => "assistants_in_warehouse",
            Feature::AssistantsOnBoard => "assistants_on_board",
            Feature::ShophousesBuilt => "shophouses_built",
            Feature::TradingHousesBuilt => "trading_houses_built",
            Feature::FivePowerTokens => "five_power_tokens",
            Feature::ImportedGoods => "imported_goods",
            Feature::AchievementProgressSum => "achievement_progress_sum",
            Feature::MercenariesTotal => "mercenaries_total",
            Feature::HandSize => "hand_size",
        }
    }

    /// True for the five features that re-derive a piece of the game's own END
    /// SCORING from the view (§2.2's bonus re-derivations plus achievement
    /// progress). They are the T2 deliverable and are staged at zero in v1 —
    /// this predicate is what lets the tests assert the staging instead of
    /// trusting a comment.
    pub fn is_score_structure(self) -> bool {
        matches!(
            self,
            Feature::CountryBonusNow
                | Feature::TechBonusNow
                | Feature::ChurchBonusNow
                | Feature::CustomsBonusNow
                | Feature::AchievementProgressSum
        )
    }
}

// ---------------------------------------------------------------------------
// Weights
// ---------------------------------------------------------------------------

/// The two weight vectors [`interpolate`] blends between, indexed by
/// [`Feature::index`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightTable {
    /// Weights at `p == 0` (nothing on any end-game clock).
    pub early: [i64; Feature::COUNT],
    /// Weights at `p == PROGRESS_FULL_PPM` (a clock is full).
    pub late: [i64; Feature::COUNT],
}

impl WeightTable {
    /// **PROVISIONAL until the T4 weight-freeze** (spec §2.4). The SIGNS come
    /// from the research notes §1; the MAGNITUDES are hand-set on this
    /// reasoning and are the "hand-set first cut" §2.4 names — T4 regenerates
    /// the corpus under Greedy-v1 with exactly these numbers, re-measures, and
    /// takes exactly ONE adjustment pass. Nothing here is a measured claim.
    ///
    /// - `victory_points` 10 → 40 dominates LATE and deliberately not early:
    ///   R1-1 measured 45.2% of eventual winners BEHIND on relative VP in the
    ///   first tercile, so an early VP-dominant vector would price the game
    ///   backwards. (The brief for this task said "vp dominant"; the research
    ///   it also cites says early VP is near-uninformative. Resolved toward the
    ///   measurement — dominant late — and recorded as a tension.)
    /// - `assistants_in_warehouse` −10 → −4 and `hand_size` −3 → −6 are the
    ///   two NEGATIVE features (R1-4, R1-7), warehouse largest early and hand
    ///   size growing late.
    /// - `assistants_in_hand` 8 → 4 sits strictly ABOVE `assistants_on_board`
    ///   7 → 4 early. That gap is deliberate and is §2.5's whole point: under
    ///   eval-v0's on_board(18) vs in_hand(8) a recovery was a strict loss, so
    ///   nonzero-recovery retreats were argmax 0 of 48,458 times (notes §4.3) —
    ///   the pathology `policy_diversity`'s gate exists to catch, arriving from
    ///   the evaluator side. Pricing in-hand above on-board makes a retreat
    ///   sometimes worth taking; T4's falsifier measures the resulting share
    ///   and must not read 0.
    /// - The building weights (12 → 20, 15 → 25) rise late because they carry
    ///   printed VP AND run an end trigger; trading houses are scarcer (4 vs 8)
    ///   and better rewarded, so they price higher.
    /// - The five score-structure features are ZERO in both vectors while
    ///   their values are staged at zero, so v1's score is exactly the ten
    ///   computed terms. T2 lands the view-side re-derivation and its weights.
    ///
    /// The column header below is a READING AID; [`Feature::ALL`] is the
    /// authority on the order, and the sign test
    /// (`provisional_weight_signs_follow_the_research_notes`) is what would
    /// actually catch a mis-indexed literal — it pins the sign of every one of
    /// the fifteen slots.
    pub const PROVISIONAL_V1: WeightTable = WeightTable {
        //     [vp, ctry, tech, chur, cust, hand, ware, brd, shop, trad, 5pw, imp, achv, merc, hnd#]
        early: [10,    0,    0,    0,    0,    8,  -10,   7,   12,   15,   8,   5,    0,    3,   -3],
        late:  [40,    0,    0,    0,    0,    4,   -4,   4,   20,   25,  10,   8,    0,    4,   -6],
    };

    /// The all-zero table: a CONSTANT evaluator. Not a curiosity — spec §3.2's
    /// degeneracy pin and §0.2's targeted-mutation authoring rule both need one,
    /// and a table is how a caller builds a mutant without a second code path.
    pub const ZERO: WeightTable =
        WeightTable { early: [0; Feature::COUNT], late: [0; Feature::COUNT] };

    /// `(early, late)` for one feature.
    pub fn get(&self, feature: Feature) -> (i64, i64) {
        let i = feature.index();
        (self.early[i], self.late[i])
    }

    /// Overwrite one feature's pair. Exists so a scenario's targeted mutation
    /// ("the same evaluator with the warehouse term zeroed") is one line and
    /// cannot mis-index the vectors by hand.
    pub fn set(&mut self, feature: Feature, early: i64, late: i64) {
        let i = feature.index();
        self.early[i] = early;
        self.late[i] = late;
    }
}

// ---------------------------------------------------------------------------
// Explanations
// ---------------------------------------------------------------------------

/// One feature's contribution to a score, with everything needed to re-derive
/// it by hand: `product == weight · (viewer_value − opponent_value)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermContribution {
    pub feature: Feature,
    /// The viewer's own raw feature value.
    pub viewer_value: i64,
    /// The raw value of the opponent the SIGNED weight selected — the
    /// opponents' maximum for a positive weight, their minimum for a negative
    /// one (see [`opponent_extreme`]). Zero when the view has no opponent.
    pub opponent_value: i64,
    /// The interpolated weight actually used, already rounded (§2.3).
    pub weight: i64,
    /// `weight · (viewer_value − opponent_value)`.
    pub product: i64,
}

/// A score, reconstructed term by term. `total` is the same `i64` [`score`]
/// returns for the same arguments, and the terms are in [`Feature::ALL`] order,
/// one per feature, always — including the staged and zero-weighted ones, so
/// a reader can see that a term contributed nothing rather than wonder whether
/// it was computed.
///
/// `PartialEq`/`Eq` are load-bearing, not conveniences: spec §0.4(ii)'s
/// determinization cell asserts that a candidate's explanation is EQUAL across
/// sampled belief worlds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    pub terms: Vec<TermContribution>,
    pub total: i64,
    /// The stage proxy the weights were interpolated at, in ppm. Reported so an
    /// explanation is self-contained: every `weight` above is recomputable from
    /// this and the table (T10 renders it).
    pub progress_ppm: u32,
}

// ---------------------------------------------------------------------------
// The two paths (§0.3(a))
// ---------------------------------------------------------------------------

/// Evaluate `view` from its own viewer's seat. Higher is better for the viewer.
///
/// THE HOT PATH: no `Explanation`, no `Vec`, no heap traffic at all — every
/// value read is a field, a `len()`, or an integer fold over an already-owned
/// map (pinned by `score_allocates_nothing_and_explain_allocates`). Search
/// calls this once per candidate (§4.4's budget), and `observe` already costs
/// 45% of a candidate, so an allocation here would be the third-largest line
/// item in the whole rung.
///
/// Total by construction: arithmetic saturates rather than overflowing (real
/// feature values are two-digit and real weights three-digit, so saturation is
/// unreachable in play — it is there so a hostile weight table cannot panic a
/// search), and a view whose own viewer seat is missing scores 0 after a
/// `debug_assert`.
pub fn score(view: &PlayerView, rules: &Rules, weights: &WeightTable) -> i64 {
    let Some(me) = seated_viewer(view) else {
        return 0;
    };
    let progress = progress_ppm(view, rules);
    let mut total: i64 = 0;
    for feature in Feature::ALL {
        let (early, late) = weights.get(feature);
        let weight = interpolate(early, late, progress);
        let mine = raw_value(view, me, feature, rules);
        let theirs = opponent_extreme(view, me, feature, rules, weight);
        total = total.saturating_add(weight.saturating_mul(mine.saturating_sub(theirs)));
    }
    total
}

/// The same evaluation, reconstructed term by term.
///
/// A SEPARATE PATH from [`score`] on purpose (§0.3(a)): it re-derives every
/// component and accumulates its own total, so `explain().total == score()` is
/// a cross-check between two assemblies rather than a restatement of one. The
/// two share only the primitives ([`raw_value`], [`interpolate`],
/// [`opponent_extreme`]) — that is the sharing the property is meant to police,
/// not the sharing it is meant to hide.
pub fn explain(view: &PlayerView, rules: &Rules, weights: &WeightTable) -> Explanation {
    let progress = progress_ppm(view, rules);
    let mut terms = Vec::with_capacity(Feature::COUNT);
    let mut total: i64 = 0;

    let Some(me) = seated_viewer(view) else {
        // Same degenerate answer `score` gives, spelled out rather than
        // returning an empty explanation whose total would still be 0: a caller
        // rendering this sees fifteen zero terms and a zero total.
        for feature in Feature::ALL {
            let (early, late) = weights.get(feature);
            terms.push(TermContribution {
                feature,
                viewer_value: 0,
                opponent_value: 0,
                weight: interpolate(early, late, progress),
                product: 0,
            });
        }
        return Explanation { terms, total, progress_ppm: progress };
    };

    for feature in Feature::ALL {
        let (early, late) = weights.get(feature);
        let weight = interpolate(early, late, progress);
        let viewer_value = raw_value(view, me, feature, rules);
        let opponent_value = opponent_extreme(view, me, feature, rules, weight);
        let product = weight.saturating_mul(viewer_value.saturating_sub(opponent_value));
        total = total.saturating_add(product);
        terms.push(TermContribution { feature, viewer_value, opponent_value, weight, product });
    }

    Explanation { terms, total, progress_ppm: progress }
}

/// The viewer's own row, or `None` for a view that does not contain its own
/// seat. `observe` cannot produce such a view; a hand-built one can, and the
/// evaluator answers rather than panicking in release. The `debug_assert` is
/// what makes a test fixture with that defect fail loudly instead of scoring 0.
fn seated_viewer(view: &PlayerView) -> Option<&VisiblePlayer> {
    let me = view.players.iter().find(|p| p.id == view.viewer);
    debug_assert!(
        me.is_some(),
        "evaluator got a view whose own viewer seat {:?} is not among its players",
        view.viewer,
    );
    me
}

/// The opponent value the relative form subtracts (§2.2).
///
/// Maximizes `weight · value` over the opponents and returns the RAW value of
/// the winner. That single multiplication is the orientation rule: with a
/// positive weight it selects the opponents' largest raw value, with a negative
/// weight their smallest — the extreme that is BEST FOR THAT OPPONENT, which is
/// what "orient higher-is-better before the max" means. A raw maximum would
/// instead reward the viewer for the worst-off rival's clogged warehouse.
///
/// Determinism: ties (including every tie at `weight == 0`) keep the FIRST
/// opponent in seat order, so the returned value is a function of the view
/// alone. Which opponent a zero-weight term names is cosmetic — its product is
/// 0 either way.
///
/// Returns 0 when there is no opponent, which no engine-produced view has
/// (2-4 seats) but a fixture can; the term then reduces to `weight · v_viewer`.
///
/// UNEXERCISED AT 2P, and the spec records it as such: with one opponent the
/// maximum is that opponent whatever the sign, so the whole orientation rule is
/// a 3-4P statement. The organic sweep in `tests/eval_core.rs` counts how often
/// a negative weight genuinely selected a different opponent than a raw maximum
/// would have, and floors that count — otherwise this function's distinguishing
/// behavior could rot untested behind 2P cells.
fn opponent_extreme(
    view: &PlayerView,
    me: &VisiblePlayer,
    feature: Feature,
    rules: &Rules,
    weight: i64,
) -> i64 {
    let mut best: Option<i64> = None;
    for other in &view.players {
        if other.id == me.id || other.id.is_dummy() {
            continue;
        }
        let value = raw_value(view, other, feature, rules);
        let improves = match best {
            None => true,
            Some(current) => weight.saturating_mul(value) > weight.saturating_mul(current),
        };
        if improves {
            best = Some(value);
        }
    }
    best.unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Feature values
// ---------------------------------------------------------------------------

/// One player's RAW (absolute, un-weighted) feature value, as both paths and
/// T4's probe read it.
///
/// Public because the probe needs exactly this: per-feature AUC is a statement
/// about raw values, and a probe re-deriving them would be a second
/// implementation of the feature set.
pub fn raw_value(
    view: &PlayerView,
    player: &VisiblePlayer,
    feature: Feature,
    rules: &Rules,
) -> i64 {
    match feature {
        Feature::VictoryPoints => i64::from(player.victory_points),

        // The five staged arms. T2 lands the view-side re-derivation at each;
        // the arguments are already in the signature so that fill is additive.
        Feature::CountryBonusNow => staged_for_t2(view, player, rules),
        Feature::TechBonusNow => staged_for_t2(view, player, rules),
        Feature::ChurchBonusNow => staged_for_t2(view, player, rules),
        Feature::CustomsBonusNow => staged_for_t2(view, player, rules),
        Feature::AchievementProgressSum => staged_for_t2(view, player, rules),

        Feature::AssistantsInHand => i64::from(player.assistants.in_hand),
        Feature::AssistantsInWarehouse => i64::from(player.assistants.in_warehouse),
        // Summed as i64, not through `AssistantState::total`'s u8 arithmetic:
        // this fold must not depend on the pawn supply staying under 256.
        Feature::AssistantsOnBoard => {
            player.assistants.on_board.values().map(|n| i64::from(*n)).sum()
        }
        Feature::ShophousesBuilt => player.shophouses.total_built() as i64,
        Feature::TradingHousesBuilt => player.trading_houses.total_built() as i64,
        Feature::FivePowerTokens => i64::from(player.five_power_tokens),
        Feature::ImportedGoods => i64::from(player.imported_goods),
        Feature::MercenariesTotal => i64::from(player.mercenaries.total()),
        // Hand SIZE, not hand contents: the size is public for every seat
        // (`observe` masks a starting order's IDENTITY, never the slot), so
        // this term reads the same from every seat.
        Feature::HandSize => player.hand.len() as i64,
    }
}

/// The value of a score-structure feature in v1: zero, pending T2.
///
/// A named stub rather than a bare `0` at five match arms, so the staging is
/// visible at the point where it applies and T2's edit is "replace this call
/// with the real re-derivation" — one arm at a time.
fn staged_for_t2(_view: &PlayerView, _player: &VisiblePlayer, _rules: &Rules) -> i64 {
    0
}

// ---------------------------------------------------------------------------
// Stage interpolation (§2.3)
// ---------------------------------------------------------------------------

/// `w_early + ((w_late − w_early)·p) / 1_000_000`, ROUNDING TOWARD ZERO —
/// Rust's integer division, stated as the convention (§2.3) because it is
/// asymmetric: a weight falling 10 → 0 reads 1 at `p = 999_999` (−9.99999
/// truncates to −9), while one rising 0 → 10 reads 9 there. Endpoints are
/// exact: `p == 0` gives `early`, `p == PROGRESS_FULL_PPM` gives `late`.
///
/// `progress` above [`PROGRESS_FULL_PPM`] is clamped, so an out-of-range proxy
/// cannot extrapolate a weight past its late value.
pub fn interpolate(early: i64, late: i64, progress: u32) -> i64 {
    let p = i64::from(progress.min(PROGRESS_FULL_PPM));
    let span = late.saturating_sub(early);
    early.saturating_add(span.saturating_mul(p) / i64::from(PROGRESS_FULL_PPM))
}

/// The stage proxy: the MAXIMUM of the view-computable end-game clocks, in ppm
/// (§2.3). Zero at setup, [`PROGRESS_FULL_PPM`] once any clock is full.
///
/// A maximum, not a sum or an average, because the game ends when the FIRST
/// trigger fires — the nearest clock is the one that dates the position.
/// Seat-invariant by construction: every input is public board state, so all
/// seats at one position interpolate identically (pinned in the organic sweep).
pub fn progress_ppm(view: &PlayerView, rules: &Rules) -> u32 {
    clock_ppm(view, rules).into_iter().max().unwrap_or(0).min(PROGRESS_FULL_PPM)
}

/// The four clocks behind [`progress_ppm`], in a fixed order:
/// `[order-deck depletion, church track, customs track, building race]`.
///
/// Public so T4's probe can validate the PROXY (§2.4's proxy-accuracy work
/// needs the components, not just their maximum) and so the tests can assert
/// each clock ran at least once.
///
/// Each mirrors an end-game trigger in `game/end_game.rs`, and each is
/// deliberately a CLOCK rather than the trigger's predicate:
///
/// - **Deck depletion** `1 − deck/initial`. The real trigger also requires an
///   empty Port slot; the clock ignores that conjunct, because an empty deck
///   is late-game whether or not the last slot has emptied yet.
/// - **Church / customs occupancy** over `track_trigger(player_count)`, counted
///   with the tracks' own `total_assistants()` — which excludes the 2P dummy
///   exactly as the trigger does (finding D1: a 2P game ends on the 4th REAL
///   pawn), so the clock cannot read 25% full at setup.
/// - **Building race** the best any player has done toward 8 shophouses or 4
///   trading houses — the two triggers `end_game.rs` checks first.
pub fn clock_ppm(view: &PlayerView, rules: &Rules) -> [u32; CLOCK_COUNT] {
    let deck = match initial_order_deck_len(view, rules) {
        // The subtraction cannot underflow: `fraction_ppm` is capped at
        // `PROGRESS_FULL_PPM`, including the case of a deck LONGER than the
        // derived initial size (which only a hand-built view can produce).
        Some(initial) => {
            PROGRESS_FULL_PPM - fraction_ppm(view.order_deck_len as u64, initial as u64)
        }
        // No derivable denominator (an empty registry): the clock is
        // UNAVAILABLE, and a maximum over the others is still meaningful.
        None => 0,
    };

    let trigger = u64::from(track_trigger(view.config.player_count));
    let church = fraction_ppm(u64::from(view.church_track.total_assistants()), trigger);
    let customs = fraction_ppm(u64::from(view.customs_track.total_assistants()), trigger);

    let mut building = 0u32;
    for player in &view.players {
        let shophouses = fraction_ppm(player.shophouses.total_built() as u64, SHOPHOUSE_GOAL);
        let trading = fraction_ppm(player.trading_houses.total_built() as u64, TRADING_HOUSE_GOAL);
        building = building.max(shophouses).max(trading);
    }

    [deck, church, customs, building]
}

/// Shophouses one player must build to fire the end trigger
/// (`game/end_game.rs`'s `total_built() >= 8`; also the printed supply, pinned
/// against `ShophouseState::new()` in `tests/eval_core.rs`).
const SHOPHOUSE_GOAL: u64 = 8;
/// Trading houses one player must build to fire the end trigger
/// (`game/end_game.rs`'s `total_built() >= 4`; also the printed supply).
const TRADING_HOUSE_GOAL: u64 = 4;

/// `num/den` in ppm, capped at [`PROGRESS_FULL_PPM`]; 0 for a zero
/// denominator. u64 throughout so the `·1_000_000` cannot overflow.
fn fraction_ppm(num: u64, den: u64) -> u32 {
    if den == 0 {
        return 0;
    }
    let ppm = num.saturating_mul(u64::from(PROGRESS_FULL_PPM)) / den;
    ppm.min(u64::from(PROGRESS_FULL_PPM)) as u32
}

/// How many order cards the deck held when play began, DERIVED from the view
/// and the registry rather than tabulated: printed orders, minus the
/// player-count setup removal, minus the two-card deal per seat, minus the
/// initial Port fill (`refill_port` fills every slot of every board in play, so
/// the fill IS the slot count). `game/setup.rs`'s `new_game` is the order of
/// operations being mirrored; `tests/eval_core.rs` pins the result against a
/// fresh game at 2/3/4P (10/13/16).
///
/// `None` when the derivation cannot be trusted — an empty registry
/// (`Rules::empty_for_tests`) or any input that would make the initial size
/// zero or negative. The caller treats that as "clock unavailable"; nothing
/// here divides by it.
pub fn initial_order_deck_len(view: &PlayerView, rules: &Rules) -> Option<usize> {
    let printed = rules.orders.iter().count();
    let seats = view.config.player_count as usize;
    let port_boards = 1 + usize::from(view.config.has_port_b);
    let port_fill = port_boards * view.port_state.port_a.slots.len();

    printed
        .checked_sub(order_cards_to_remove(view.config.player_count) as usize)?
        .checked_sub(2 * seats)?
        .checked_sub(port_fill)
        .filter(|initial| *initial > 0)
}

/// Church/customs pawns that fire the end trigger, by player count.
///
/// MIRROR of `GameConfig::track_trigger` (`types/state.rs`), pinned against it
/// by `the_player_count_mirrors_match_the_engines_own_functions`. A view
/// carries a `ViewConfig`, which has neither method, and building a
/// `GameConfig` to ask would allocate its colour vector inside §0.3(a)'s
/// allocation-free path. Mirror + drift pin is this repo's standard answer.
pub fn track_trigger(player_count: u8) -> u8 {
    match player_count {
        2 => 4,
        3 => 5,
        4 => 6,
        _ => 6,
    }
}

/// Order cards boxed at setup, by player count. MIRROR of
/// `GameConfig::order_cards_to_remove`, pinned the same way.
pub fn order_cards_to_remove(player_count: u8) -> u8 {
    match player_count {
        2 => 16,
        3 => 5,
        4 => 0,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Unit tests: pure arithmetic and table shape only.
//
// Everything needing a real view lives in `tests/eval_core.rs` — see that
// file's header for why (the §2.1 guard scans THIS file, so a test module here
// that imported the setup/observe machinery would force the allowlist open).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_feature_list_is_fifteen_features_in_a_canonical_order() {
        assert_eq!(Feature::COUNT, 15, "spec §2.2's list is fifteen entries");
        for (i, feature) in Feature::ALL.into_iter().enumerate() {
            assert_eq!(feature.index(), i, "{} is not at its own index", feature.name());
        }
        // Names are the artifact/report key, so they must be unique.
        let mut names: Vec<&str> = Feature::ALL.iter().map(|f| f.name()).collect();
        names.sort_unstable();
        let unique = names.len();
        names.dedup();
        assert_eq!(names.len(), unique, "duplicate feature name");
        // The exact five that T2 fills.
        assert_eq!(Feature::ALL.iter().filter(|f| f.is_score_structure()).count(), 5);
    }

    #[test]
    fn interpolation_is_exact_at_both_endpoints() {
        for (early, late) in [(10i64, 40i64), (-10, -4), (7, 7), (0, 0), (-3, 6)] {
            assert_eq!(interpolate(early, late, 0), early);
            assert_eq!(interpolate(early, late, PROGRESS_FULL_PPM), late);
        }
    }

    /// §2.3's rounding convention, pinned in BOTH signs and at both ends of the
    /// range. Round-toward-zero is asymmetric, so the falling and rising cases
    /// disagree at the same `p` — that asymmetry is the thing being fixed in
    /// place, because `explain` reports whatever `score` used.
    #[test]
    fn interpolation_rounds_toward_zero_in_both_signs() {
        // Falling 10 -> 0: −9.99999 truncates toward zero to −9, so 1, not 0.
        assert_eq!(interpolate(10, 0, 999_999), 1);
        // Rising 0 -> 10: +9.99999 truncates to 9.
        assert_eq!(interpolate(0, 10, 999_999), 9);
        // Just off zero, both directions: the tiny step is rounded AWAY from
        // late and toward early.
        assert_eq!(interpolate(0, 10, 1), 0);
        assert_eq!(interpolate(10, 0, 1), 10);
        // A wholly negative pair (the warehouse shape, −10 -> −4): the span is
        // +6, and at the midpoint the rise is exactly 3.
        assert_eq!(interpolate(-10, -4, 500_000), -7);
        // A negative-going span from a negative start (the hand-size shape).
        // Both of these round TOWARD ZERO, i.e. toward the EARLY weight: the
        // exact values are −4.5 and −5.999997.
        assert_eq!(interpolate(-3, -6, 500_000), -4); // −3 + (−3·0.5 = −1.5 -> −1)
        assert_eq!(interpolate(-3, -6, 999_999), -5); // −3 + (−2.999997 -> −2)
        // Halfway on the VP shape.
        assert_eq!(interpolate(10, 40, 500_000), 25);
    }

    #[test]
    fn interpolation_clamps_an_out_of_range_proxy_and_cannot_overflow() {
        assert_eq!(interpolate(10, 40, PROGRESS_FULL_PPM * 3), 40, "no extrapolation");
        // Saturating arithmetic: a hostile table must not panic a search.
        let _ = interpolate(i64::MIN, i64::MAX, 500_000);
        let _ = interpolate(i64::MAX, i64::MIN, 999_999);
    }

    #[test]
    fn a_fraction_is_capped_and_total_over_a_zero_denominator() {
        assert_eq!(fraction_ppm(0, 8), 0);
        assert_eq!(fraction_ppm(4, 8), 500_000);
        assert_eq!(fraction_ppm(8, 8), PROGRESS_FULL_PPM);
        assert_eq!(fraction_ppm(99, 8), PROGRESS_FULL_PPM, "capped, never above full");
        assert_eq!(fraction_ppm(3, 0), 0, "no derivable denominator");
        assert_eq!(fraction_ppm(1, 3), 333_333, "toward zero, like the weights");
    }

    /// The signs are the part of [`WeightTable::PROVISIONAL_V1`] that comes
    /// from measurement (research notes §1), so they are pinned; the magnitudes
    /// are hand-set and deliberately NOT pinned beyond the two structural
    /// relations the doc argues for.
    #[test]
    fn provisional_weight_signs_follow_the_research_notes() {
        let w = WeightTable::PROVISIONAL_V1;
        for feature in Feature::ALL {
            let (early, late) = w.get(feature);
            match feature {
                // R1-4 and R1-7: the two negative features, in BOTH vectors.
                Feature::AssistantsInWarehouse | Feature::HandSize => {
                    assert!(early < 0 && late < 0, "{} must be negative", feature.name());
                }
                // Staged at zero until T2.
                f if f.is_score_structure() => {
                    assert_eq!((early, late), (0, 0), "{} is staged", feature.name());
                }
                _ => assert!(early > 0 && late > 0, "{} must be positive", feature.name()),
            }
        }

        // VP dominates LATE (and only late — R1-1).
        let (vp_early, vp_late) = w.get(Feature::VictoryPoints);
        for feature in Feature::ALL {
            if feature == Feature::VictoryPoints {
                continue;
            }
            let (_, late) = w.get(feature);
            assert!(vp_late > late.abs(), "vp must dominate {} late", feature.name());
        }
        let early_max = Feature::ALL
            .into_iter()
            .map(|f| w.get(f).0.abs())
            .max()
            .expect("fifteen features");
        assert!(vp_early < early_max, "early VP must NOT dominate (R1-1)");

        // §2.5: in-hand priced above on-board early, so a recovery is not a
        // strict loss the way it was under eval-v0.
        assert!(w.get(Feature::AssistantsInHand).0 > w.get(Feature::AssistantsOnBoard).0);
    }

    #[test]
    fn weights_can_be_read_and_mutated_by_feature() {
        let mut w = WeightTable::PROVISIONAL_V1;
        assert_eq!(w.get(Feature::VictoryPoints), (10, 40));
        w.set(Feature::AssistantsInWarehouse, 0, 0);
        assert_eq!(w.get(Feature::AssistantsInWarehouse), (0, 0), "the §0.2 mutation shape");
        assert_eq!(w.get(Feature::VictoryPoints), (10, 40), "and nothing else moved");
        assert_eq!(WeightTable::ZERO.get(Feature::VictoryPoints), (0, 0));
        assert_eq!(WeightTable::ZERO.early.iter().chain(WeightTable::ZERO.late.iter()).sum::<i64>(), 0);
    }
}
