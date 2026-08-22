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
//! # The score-structure mirrors (§2.2, landed at T2)
//!
//! Five of the sixteen features re-derive a piece of the rulebook's END
//! SCORING from the view: the country-set bonus, the technology
//! industry-value majority, the church and customs pawn majorities (with the
//! 2-player dummy competitor priced exactly as the rules price it — finding
//! D1), and progress toward the three Achievement cards. They are DECOMPOSED,
//! one feature each, never one projected-score scalar — research notes R1-3
//! measured `country_bonus_now` at AUC 0.6212 early against the whole
//! projected scalar's 0.5806, so the aggregate is strictly worse than its own
//! best component.
//!
//! They are re-derivations, not calls: this file may not reach into the
//! engine's scoring code (§2.1's guard), so the arithmetic is written here
//! from the view plus the card registries and PINNED against the engine's own
//! per-player breakdown over organic play in `tests/eval_core.rs`. Mirror plus
//! drift pin is this repo's standard answer to a duplicated rule.
//!
//! **Cost shape, stated because it is easy to get wrong.** Four of the five
//! are properties of the WHOLE table (a majority is a ranking over seats), not
//! of one seat, so computing them inside a per-`(feature, player)` accessor
//! would pay each ranking once per feature per player — up to sixteen times a
//! call at 4P. [`ScoreStructure::derive`] computes every one of them ONCE per
//! [`score`] call into stack arrays indexed by seat, and the accessors just
//! index. Nothing here allocates.
//!
//! # Terminal views (§2.6)
//!
//! A finished game is not scored positionally: [`score`] detects terminality
//! from the view ([`view_is_terminal`], a mirror of the engine's own
//! derivation, pinned), re-derives EVERY seat's full final total (running VP
//! plus all five decomposed bonuses plus the remainder conversions), ranks the
//! viewer under the official clockwise tie-break, and returns a win/loss
//! dominant ladder ([`TerminalStanding`]). See that type for the magnitudes
//! and for the masked-opponent-order residual.
//!
//! # What is frozen, and what is still hand-set
//!
//! [`WeightTable::V2`] is the SHIPPED table since 2026-08-10 — the Phase-3.5c
//! evaluator-v2 freeze, three rounds of re-pricing on held-out fits over fixed
//! incumbent-play corpora, promoted on one pre-registered fresh cell (criterion
//! 1: lookahead@V2 vs lookahead@V1, θ̂ 0.6775, interval entirely above the
//! null). Its own doc carries the round trail and, more importantly, the
//! register of what the freeze does NOT establish — magnitudes are still
//! hand-set (now INFORMED by fits, which is not the same as tuned), round
//! screens do not compose, and every L1/L2 figure is off-policy.
//! [`WeightTable::V1`] is RETAINED VERBATIM beside it, exactly as
//! [`WeightTable::PRE_FREEZE_2026_07_27`] is retained beside `V1`.
//!
//! `V1` was FROZEN at T4 (2026-07-27): its sixteen SIGNS were re-measured on
//! 2,000 Greedy-v1 self-play games with game-clustered intervals and none was
//! contradicted, so §2.4's single adjustment pass moved nothing. The full
//! adjudication, the two recorded magnitude tensions — the warehouse decay and
//! the assistant triple's ordering — and every command that reproduces the run
//! are in `docs/plans/2026-07-27-phase3.3-weight-freeze.md`. Both of those
//! tensions were acted on in the v2 rounds; the artifact stays the record of
//! what was measured when.
//!
//! The FEATURE LIST is unchanged across both freezes at sixteen. The one
//! feature-level change v2 made is inside a value rather than in the list:
//! [`Feature::AchievementProgressSum`] now drops the legs of cards the valued
//! seat has already claimed (T3's post-claim filter).

use crate::game::rules::Rules;
// V4 spec §3.2's registered engine paths, and the ONE reason this module reaches
// past the registries: S1, S5 and S6 price an option at the reward the ENGINE
// pays for it. Re-deriving those tables here would be a second copy of the
// rulebook's own numbers — the exact duplication §13's dispatcher exists to
// prevent — so the shipped helpers are called instead. Nothing here reads state:
// all three take a power and return a printed quantity. The §2.1 guard's
// allowlist names this line, which is what makes widening it a deliberate act.
use crate::game::actions::{bank_reward, employment_agency_tokens_allowed, exported_reward};
use crate::types::{
    AchievementType, AreaId, AreaType, Country, HandCard, OrderCard, OrderCardId, PlayerId,
    PlayerView, PresidentLocation, TechCardId, TurnPhase, VisiblePlayer,
};

/// Bumped on ANY behavior-affecting change to the evaluator — the feature list,
/// the relative form, the stage shape, or the weight tables (T4's freeze is a
/// bump). Spec §0.5/L12: a weight change makes previously-recorded figures for
/// EVERY controller that embeds this evaluator non-comparable, which is what
/// `SeatProvenance::version` means; this const rides the params map so the
/// attribution is finer than the controller name alone.
///
/// **2 = the T4 FREEZE** (`docs/plans/2026-07-27-phase3.3-weight-freeze.md`),
/// and the honest reading of that particular bump: the measured adjustment pass
/// moved ZERO weights, so `1` and `2` name the same thirty numbers and are
/// behaviourally comparable. What changed is STATUS — provisional-until-measured
/// became frozen-and-measured. The bump was taken anyway because this const's own
/// rule pre-committed to it, and because a status change is exactly the thing a
/// corpus reader wants a version for; the finer key that says whether two
/// records were produced by the same thirty numbers is the weight-table identity
/// digest the greedy rung stamps beside this const in provenance, and that key
/// did NOT move (`0xe1fedd5efc49d0f6` on both sides of that freeze).
///
/// **3 = THE PHASE-3.5c v2 FREEZE** (2026-08-10; protocol
/// `docs/plans/2026-08-09-phase3.5c-evaluator-v2-protocol.md` §7, mechanics
/// fixed in `docs/plans/2026-08-09-phase3.5c-freeze-prereg.md` §2), and this
/// bump is the OPPOSITE of bump 2's: it names a real behaviour change, in two
/// parts landed as ONE auditable event.
///
/// * **[`WeightTable::V2`] ships** — six slots re-priced in round 1 (the early
///   re-mix off R0's two contradicted early terms, plus the warehouse flatten)
///   and seven more in round 2 (the late church/five-power crossing, the
///   hand-size softening, the early extension). Thirty-two numbers, weight
///   identity `0x9a240f2ff7624b0e` against V1's `0xe1fedd5efc49d0f6`.
/// * **T3's post-claim filter reaches the call site** —
///   [`Feature::AchievementProgressSum`] is computed by
///   [`achievement_progress_postclaim_sum`] in [`ScoreStructure::derive`], so a
///   card the VIEWER has already claimed stops paying a leg whose VP is already
///   priced by [`Feature::VictoryPoints`]. The feature COUNT does not move, so
///   [`Feature::ALL`] and both vector widths are untouched.
///
/// **The comparability key is composite and no single const carries it** (T16):
/// `SEMANTICS_REV` (UNCHANGED at 7 — the freeze touches no rule), the
/// weight-table identity digest (moved, above), and the per-controller version
/// consts (`GREEDY_IMPL_VERSION`, `LOOKAHEAD_IMPL_VERSION`,
/// `ATOMIC_DIAG_IMPL_VERSION`, `MCTS_IMPL_VERSION` — all bumped in the same
/// commit, per L12). The corollary, stated so nobody has to infer it: **every
/// figure recorded before this bump for a controller that embeds this evaluator
/// is non-comparable forward.** That includes the 3.3 freeze artifact's whole
/// table, the 3.4 ADR's cells, the R0 corpora, and every round-screen figure in
/// the ledger — each of which is still a valid record of what it measured, and
/// none of which may be pooled with a v2 figure.
pub const EVAL_IMPL_VERSION: u32 = 3;

/// Full progress, in the ppm units [`progress_ppm`] reports.
pub const PROGRESS_FULL_PPM: u32 = 1_000_000;

/// How many end-game clocks [`clock_ppm`] reports.
pub const CLOCK_COUNT: usize = 4;

/// Where the in-hand assistant count is SPLIT into
/// [`Feature::AssistantsInHandCore`] and [`Feature::AssistantsInHandExcess`]:
/// assistants up to the reserve are the ones a seat needs simply to keep
/// acting, and are priced ABOVE a board presence; everything past it is spare
/// tempo, priced BELOW one. The two inequalities together are what makes a
/// placement and a recovery both payable at one ply — see [`WeightTable`] for
/// the finding that forced the split.
///
/// **RULES-STRUCTURAL, and UNMEASURED per piece.** A church visit, a customs
/// visit and a placement each want a hand assistant, so a hand at zero is a
/// seat that cannot act; 2 is the smallest reserve that keeps one action AND
/// one placement live. Research note R1-4 measured the UNSPLIT
/// `assistants_in_hand` feature at AUC 0.6026 early — that figure says nothing
/// about where the knee belongs, because the feature it measured had no knee.
///
/// **T4 measured the two halves separately and did NOT move the reserve**
/// (`docs/plans/2026-07-27-phase3.3-weight-freeze.md`; 2,000 Greedy-v1 2P
/// self-play games, game-clustered 95%). What it found is worth carrying here,
/// because it looks like a contradiction until the scale row is read:
///
/// | half | relative AUC early / mid / late | mean raw value |
/// |---|---|---|
/// | core `min(in_hand, 2)` | 0.5278 / 0.5173 / 0.4877 | **1.994 at every stage** |
/// | excess `in_hand − core` | 0.6874 / 0.7200 / 0.6944 | 1.68 / 1.39 / 1.57 |
///
/// The core half separates winners from losers barely at all — because it is
/// very nearly the CONSTANT 2, and a constant separates nothing whatever weight
/// it carries. All of the unsplit feature's measured signal (0.6878 early) is in
/// the excess half. That is the reverse of the weight ORDERING
/// `core > board > excess`, and the ordering is held anyway: it is L14's
/// non-degeneracy condition on the one-ply argmax, not a claim about predictive
/// power, and pricing a near-constant highly costs a ranking nothing. The
/// tension is recorded in the freeze artifact and routed to 3.5, where weights
/// are tuned rather than hand-set.
pub const HAND_CORE_RESERVE: u8 = 2;

// ---------------------------------------------------------------------------
// Features
// ---------------------------------------------------------------------------

/// The sixteen relative features of evaluator v1, in THE canonical order
/// (spec §2.2's list, read top to bottom, AMENDED AT T3 by the in-hand split).
/// The order is load-bearing: it indexes both weight vectors, it is the order
/// `explain` reports terms in, and T4's probe reports per-feature figures
/// against it.
///
/// **What adding a feature costs, stated exactly** (six-lens review — the
/// previous wording claimed more than the types deliver). A new variant is a
/// COMPILE ERROR at [`index`](Self::index), [`name`](Self::name) and every other
/// exhaustive match over `Feature`, so those arms cannot be forgotten. And
/// [`COUNT`](Self::COUNT) is `ALL.len()`, which sizes both weight vectors, so a
/// variant added to [`ALL`](Self::ALL) without a weight beside it in each vector
/// is a compile error too — the array lengths really do move together. **The one
/// silent path is omission from `ALL` itself**: `ALL` is a hand-maintained
/// literal with no compiler-checked relation to the variant set, so a variant
/// given its match arms but left out of `ALL` compiles clean, keeps `COUNT` at
/// 16, and never appears in `score`/`explain`'s `for feature in Feature::ALL`
/// loops — scoring zero, silently. Adding a feature therefore means `ALL` FIRST,
/// then both vectors; the compiler covers the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Feature {
    /// Running victory points. R1-1: AUC 0.5625 early → 0.8612 late — the
    /// late-game separator, and deliberately not an early one.
    VictoryPoints,
    /// The country-set end bonus re-derived from the view. R1-3 measured it as
    /// the best EARLY separator (0.6212). See [`ScoreStructure`].
    CountryBonusNow,
    /// The technology end bonus re-derived from the view (R1-10's cluster
    /// winner over a raw tech count): the 10/5 industry-value majority.
    TechBonusNow,
    /// The church majority bonus re-derived from the view (6/3), dummy
    /// competitor included (finding D1).
    ChurchBonusNow,
    /// The customs majority bonus re-derived from the view (8/4), dummy
    /// competitor included (finding D1).
    CustomsBonusNow,
    /// Assistants in hand UP TO [`HAND_CORE_RESERVE`] — the reserve that keeps
    /// a seat able to act. Raw value `min(in_hand, HAND_CORE_RESERVE)`, priced
    /// ABOVE [`Feature::AssistantsOnBoard`].
    ///
    /// This and its Excess twin are the T3 amendment to §2.2's list: R1-4
    /// measured a SINGLE `assistants_in_hand` feature at 0.6026 early, and a
    /// single linear in-hand term cannot make a placement and a recovery both
    /// pay at one ply. [`WeightTable`] states the finding and the arithmetic.
    AssistantsInHandCore,
    /// Assistants in hand PAST the reserve — spare tempo, and the stock a
    /// placement ought to be spending. Raw value
    /// `in_hand − min(in_hand, HAND_CORE_RESERVE)`, priced BELOW
    /// [`Feature::AssistantsOnBoard`].
    AssistantsInHandExcess,
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
    /// Progress toward the three Achievement cards in play, each leg capped at
    /// its own threshold — and, since the v2 freeze, with the legs of cards the
    /// VALUED SEAT has already claimed dropped entirely (T3's post-claim filter:
    /// [`achievement_progress_postclaim_sum`], whose doc carries the scope
    /// decision). An OPPONENT'S claim leaves this seat's leg live, because the
    /// rules let a second seat claim the same card. See
    /// [`achievement_progress`] for the per-card arithmetic.
    AchievementProgressSum,
    /// Foreign mercenaries held (used and unused).
    MercenariesTotal,
    /// Order cards in hand. R1-7: 0.4745/0.4171/0.3753 — NEGATIVE and growing
    /// (an unfulfilled order is a commitment, not an asset).
    HandSize,
}

impl Feature {
    /// The canonical order. `ALL[f.index()] == f` for every feature.
    pub const ALL: [Feature; 16] = [
        Feature::VictoryPoints,
        Feature::CountryBonusNow,
        Feature::TechBonusNow,
        Feature::ChurchBonusNow,
        Feature::CustomsBonusNow,
        Feature::AssistantsInHandCore,
        Feature::AssistantsInHandExcess,
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
            Feature::AssistantsInHandCore => 5,
            Feature::AssistantsInHandExcess => 6,
            Feature::AssistantsInWarehouse => 7,
            Feature::AssistantsOnBoard => 8,
            Feature::ShophousesBuilt => 9,
            Feature::TradingHousesBuilt => 10,
            Feature::FivePowerTokens => 11,
            Feature::ImportedGoods => 12,
            Feature::AchievementProgressSum => 13,
            Feature::MercenariesTotal => 14,
            Feature::HandSize => 15,
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
            Feature::AssistantsInHandCore => "assistants_in_hand_core",
            Feature::AssistantsInHandExcess => "assistants_in_hand_excess",
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

    /// True for the five features whose value is re-derived END SCORING
    /// arithmetic rather than a field read (§2.2's four bonus mirrors plus
    /// achievement progress). Their values come from [`ScoreStructure`], which
    /// is computed once per [`score`] call — this predicate is what lets the
    /// tests assert that grouping instead of trusting a comment, and it is the
    /// set the drift pin holds against the engine's own breakdown.
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
    /// **FROZEN at T4** (spec §2.4; artifact
    /// `docs/plans/2026-07-27-phase3.3-weight-freeze.md`). The SIGNS come from
    /// the research notes §1 and are now RE-MEASURED under the bot's own play;
    /// the MAGNITUDES remain hand-set on the reasoning below, and the freeze did
    /// not measure them.
    ///
    /// **SUPERSEDED AS THE SHIPPED TABLE ON 2026-08-10 by [`WeightTable::V2`],
    /// and RETAINED VERBATIM.** Nothing below has moved a digit; what changed is
    /// what reads it. No constructor takes `V1` any more — `GreedyAgent::new`,
    /// the S1 rung, the MCTS diagnostic and every arena contestant take `V2` —
    /// and its remaining readers are the record: the screen harness's incumbent
    /// arm, `examples/eval_probe`'s `--weights v1` (still the DEFAULT there, so
    /// a committed corpus command still parses and runs under the TABLE its
    /// figures named — though NOT to the same corpus: T3's filter now rides the
    /// score structure unconditionally, so the recorded corpus figures belong
    /// to the pre-filter tree and reproduce only at their recorded commits —
    /// the freeze review's MUST-1), the sign and ordering pins
    /// below, and the scenarios whose claim is table-independent. Its identity
    /// digest is `0xe1fedd5efc49d0f6`. Everything this doc says about T4 is a
    /// statement about 2026-07-27 and stays true; the v2 freeze's own register
    /// is on `V2`.
    ///
    /// # What the freeze did, precisely
    ///
    /// §2.4's protocol: regenerate the corpus with Greedy-v1 playing these exact
    /// numbers, re-measure, take EXACTLY ONE adjustment pass, and move a weight
    /// only where the operationalized materiality rule fires — *a measured
    /// sign whose game-clustered 95% CI EXCLUDES the shipped direction.*
    ///
    /// Corpus: 2,000 Greedy-v1 2P self-play games, 326,972 view snapshots,
    /// 12,000 (game × seat × tercile) cells, diagnostic master
    /// `0xE7A1_2026_0727_0001`, bootstrap B = 2,000 clustered on games.
    ///
    /// **Result: the rule fired on ZERO of the sixteen features, so the pass
    /// moved zero weights and this table IS the first cut.** Twenty-NINE of the
    /// thirty-two (feature × endpoint) signs came back CONFIRMED — interval
    /// wholly on the shipped side of 0.5 — and the other three were
    /// no-signal, not contradictions: `assistants_in_hand_core` late
    /// (0.4877 [0.4734, 0.5017], a near-constant feature — see
    /// [`HAND_CORE_RESERVE`]), `hand_size` early (0.5049 [0.4797, 0.5301]), and
    /// `trading_houses_built` early, which is identically zero in the first
    /// tercile and so has no separation to have. **An empty pass is a result,
    /// not a skipped step**, and the per-feature adjudication table is in the
    /// artifact so the emptiness is auditable rather than asserted.
    ///
    /// **What the freeze did NOT establish.** Magnitudes. A confirmed sign says
    /// a term points the right way, not that 12 is better than 20; every
    /// magnitude below is still the hand-set reasoning it always was, and 3.5's
    /// tuning is where they stop being. Two places where the measurement and the
    /// hand-set magnitude visibly disagree are recorded rather than acted on:
    /// the warehouse term's separation does NOT decay late (relative AUC
    /// 0.3106 → 0.2742 while the weight decays −10 → −4), and the assistant
    /// triple's measured separation ordering is the REVERSE of its weight
    /// ordering (see [`HAND_CORE_RESERVE`] for why that is consistent rather
    /// than contradictory).
    ///
    /// - `victory_points` 10 → 40 dominates LATE and deliberately not early:
    ///   R1-1 measured 45.2% of eventual winners BEHIND on relative VP in the
    ///   first tercile, so an early VP-dominant vector would price the game
    ///   backwards. (The brief for this task said "vp dominant"; the research
    ///   it also cites says early VP is near-uninformative. Resolved toward the
    ///   measurement — dominant late — and recorded as a tension.) **T4
    ///   re-measure, one of §2.4's three named figures: early VP AUC
    ///   0.5604 [0.5367, 0.5859] under greedy self-play, against R1's 0.5625
    ///   under family-weighted — the shape survives the regime change intact,
    ///   and the late figure 0.8571 reproduces R1's 0.8612.**
    /// - `assistants_in_warehouse` −10 → −4 and `hand_size` −3 → −6 are the
    ///   two NEGATIVE features (R1-4, R1-7), warehouse largest early and hand
    ///   size growing late. **T4 re-measure, §2.4's third named figure: the
    ///   warehouse magnitude GREW under greedy play — relative AUC
    ///   0.3106 [0.2875, 0.3341] early against R1's 0.3932, i.e. |AUC − 0.5| up
    ///   from 0.107 to 0.189 — and it does not decay late (0.2742), which the
    ///   −10 → −4 weight decay does. Sign confirmed, magnitude shape unsupported
    ///   and left alone by the one-pass rule; recorded for 3.5.**
    /// - **THE ASSISTANT TRIPLE, and the T3 finding it resolves.**
    ///   `assistants_in_hand_core` 30 → 20, `assistants_on_board` 14 → 10 and
    ///   `assistants_in_hand_excess` 4 → 2 are ONE decision, not three: the
    ///   ordering `core > board > excess` holds in BOTH vectors (and therefore
    ///   at every interpolation between them, since the bands `[20,30]`,
    ///   `[10,14]` and `[2,4]` do not overlap), and those two inequalities are
    ///   the non-degeneracy conditions the rung needs.
    ///
    ///   The finding: at ONE PLY a placement moves `k` assistants from hand to
    ///   board and does nothing else the positional vector can see — the area
    ///   action a placement buys happens in a LATER phase, past the one-ply
    ///   horizon. With a LINEAR in-hand term a placement's sign is therefore
    ///   exactly `k · (w_on_board − w_in_hand)`, which is the same inequality,
    ///   negated, that §2.5 wanted the OTHER way so a retreat could ever pay.
    ///   One linear term cannot have both. The pre-T3 table took the retreat
    ///   side (in-hand 8 over on-board 7), and the consequence was measured,
    ///   not feared: every placement scored strictly negative, so 2P greedy
    ///   self-play placed 0 assistants and ran to the turn cap — R2's absorbing
    ///   loop arriving from the WEIGHTS instead of from the tie-break.
    ///
    ///   The PIECEWISE split has both, because a placement and a recovery spend
    ///   different parts of the hand. Placing FROM EXCESS pays
    ///   (`w_board > w_excess`); stripping the last of the reserve does not
    ///   (`w_core > w_board`); recovering into an EMPTY hand pays (the same
    ///   `w_core > w_board`, read the other way); over-recovering past the
    ///   reserve does not (`w_board > w_excess`, likewise). §2.5's falsifier is
    ///   unchanged and now achievable — T4 MEASURES the nonzero-recovery pick
    ///   share and it must not read 0, which under eval-v0's on_board(18) vs
    ///   in_hand(8) it did: argmax 0 of 48,458 times (notes §4.3), the
    ///   pathology `policy_diversity`'s gate exists to catch, arriving from the
    ///   evaluator side. **T4 measured it: 739 nonzero-recovery picks in
    ///   1,051,752 applied moves at 2P, share 0.000703 — not zero, so the
    ///   falsifier passes, and the per-edit floor is set at half that
    ///   (`greedy.rs`'s `RECOVERY_SHARE_FLOOR_2P`).** Split point and magnitudes
    ///   alike remain hand-set and unmeasured per piece; see
    ///   [`HAND_CORE_RESERVE`] for what T4 DID measure about the two halves and
    ///   why the ordering is held against it.
    /// - The building weights (12 → 20, 15 → 25) rise late because they carry
    ///   printed VP AND run an end trigger; trading houses are scarcer (4 vs 8)
    ///   and better rewarded, so they price higher.
    ///
    /// The five score-structure weights, landed at T2 — one line of grounds
    /// each, all hand-set on the same terms as the rest:
    ///
    /// - `country_bonus_now` 12 → 30. The largest EARLY weight in the table
    ///   and deliberately so: R1-3 measured it as the corpus's best early
    ///   separator (AUC 0.6212, against VP's 0.5625), so a token weight here
    ///   would throw away the one term that sees the early game. Late 30 sits
    ///   just under VP's 40 because completed orders and owned techs cannot be
    ///   taken back — this bonus is the least contingent of the four — but a
    ///   VP already banked still compounds and a bonus VP does not.
    /// - `tech_bonus_now` 6 → 20. A 10/5 majority an opponent can still
    ///   overtake, so it prices well under the banked country set; R1-10 makes
    ///   it the cluster's single shipped member, so it also carries whatever
    ///   the raw tech count used to.
    /// - `church_bonus_now` −4 → 16 is the one SIGN-FLIPPING feature, and it
    ///   is the reason §2.3 interpolates instead of switching. R1-5 measured
    ///   early church investment as a LOSING indicator (AUC 0.4280 early →
    ///   0.5234 late, a genuine flip) — weaker than the warehouse's 0.3932, so
    ///   the magnitude is smaller than that term's −10. The late weight is
    ///   positive on VP-structure grounds: the printed 6/3 is real VP once the
    ///   game stops. TENSION RECORDED, not hidden: R1's critic flagged this
    ///   exact sign as regime-conditional (measured under family-weighted
    ///   self-play; ~86% of greedy games end via ChurchTrackFull, where racing
    ///   the track may pay), and §2.4 names the church sign as one of the three
    ///   figures T4 re-measures under the greedy corpus before the freeze.
    ///   **T4 SETTLED IT, and the sign SURVIVES: under 2,000 Greedy-v1 2P
    ///   self-play games — 99.95% of which end via ChurchTrackFull, the regime
    ///   the critic worried about — `church_bonus_now` measures
    ///   0.4411 [0.4174, 0.4653] early and 0.5754 [0.5505, 0.6012] late. Both
    ///   intervals exclude 0.5 on the shipped side, so the flip is confirmed in
    ///   the very regime that was supposed to overturn it, and the tension above
    ///   is DISCHARGED rather than carried.**
    /// - `customs_bonus_now` 3 → 20. Positive at both ends: R1-5's early
    ///   negative is a CHURCH finding and does not transfer (no customs
    ///   equivalent was measured), and the printed 8/4 outranks the church's
    ///   6/3, so the late weight sits above it.
    /// - `achievement_progress_sum` 2 → 4. Positive (R1's recommended set) but
    ///   small, because its scale is not VP: it is a sum of three capped
    ///   progress legs in mixed units (goods counts, yen, building counts) that
    ///   can reach ~19, so a VP-scale weight would let it swamp the table.
    ///
    /// The column header below is a READING AID; [`Feature::ALL`] is the
    /// authority on the order, and the sign test
    /// (`frozen_v1_weight_signs_follow_the_research_notes`) is what would
    /// actually catch a mis-indexed literal — it pins the sign of every one of
    /// the sixteen slots, with
    /// `the_hand_split_orders_core_above_board_above_excess_in_both_vectors`
    /// pinning the two ordering inequalities the assistant triple turns on.
    pub const V1: WeightTable = WeightTable {
        //     [vp, ctry, tech, chur, cust, hCORE, hEXC, ware, brd, shop, trad, 5pw, imp, achv, merc, hnd#]
        early: [10,   12,    6,   -4,    3,    30,    4,  -10,  14,   12,   15,   8,   5,    2,    3,   -3],
        late:  [40,   30,   20,   16,   20,    20,    2,   -4,  10,   20,   25,  10,   8,    4,    4,   -6],
    };

    /// The hand-set first cut that GENERATED the T4 freeze corpus — the
    /// historical input to the measurement, shipped by no constructor.
    ///
    /// **It currently holds the same thirty numbers as [`WeightTable::V1`], and
    /// that is the freeze's RESULT rather than a duplication**: §2.4's one
    /// adjustment pass found no measured sign contradicting a shipped direction,
    /// so it moved nothing and the first cut became the frozen table unchanged
    /// (see `V1`'s doc for the adjudication).
    ///
    /// It exists as its own constant because the two have different JOBS. `V1`
    /// is what the bot plays and what a future task may re-price; this is a
    /// fixed record of what the corpus behind
    /// `docs/plans/2026-07-27-phase3.3-weight-freeze.md` was generated by, so
    /// every command in that artifact — each of which names `--weights
    /// pre-freeze` — still reproduces its output after `V1` moves. A measurement
    /// whose inputs cannot be re-run is a claim, not a record. The literal
    /// arrays are pinned by
    /// `the_pre_freeze_table_is_the_corpus_generator_and_v1_matches_it_today`,
    /// which also states what to do when `V1` legitimately moves away.
    ///
    /// Nothing else may read it: the sign and ordering pins hold `V1`, and the
    /// only other in-tree reader is `examples/eval_probe`'s reproduction flag
    /// (`--weights pre-freeze`). Since the v2 freeze no constructor takes `V1`
    /// either — they take [`WeightTable::V2`] — so this const's neighbour is a
    /// record too, and the equality pin below is a statement about 2026-07-27
    /// that both of them still satisfy.
    pub const PRE_FREEZE_2026_07_27: WeightTable = WeightTable {
        //     [vp, ctry, tech, chur, cust, hCORE, hEXC, ware, brd, shop, trad, 5pw, imp, achv, merc, hnd#]
        early: [10,   12,    6,   -4,    3,    30,    4,  -10,  14,   12,   15,   8,   5,    2,    3,   -3],
        late:  [40,   30,   20,   16,   20,    20,    2,   -4,  10,   20,   25,  10,   8,    4,    4,   -6],
    };

    /// **THE SHIPPED TABLE since the Phase-3.5c freeze (2026-08-10)** — what
    /// `GreedyAgent::new`, the S1 search rung, the MCTS diagnostic and every
    /// arena contestant evaluate with. It is the evaluator-v2 protocol's
    /// iteration candidate promoted verbatim: the numbers below are the ones
    /// round 2 adopted, unchanged by the promotion, weight identity
    /// **`0x9a240f2ff7624b0e`** (V1's is `0xe1fedd5efc49d0f6`).
    ///
    /// [`WeightTable::V1`] is RETAINED VERBATIM beside it — byte-identical to
    /// what it was before the freeze — exactly as
    /// [`WeightTable::PRE_FREEZE_2026_07_27`] is retained beside `V1`, and for
    /// the same reason: the round evidence, the R0 corpora and the 3.3 freeze
    /// artifact all name a table, and a measurement whose input cannot be re-run
    /// is a claim rather than a record. Nothing here re-prices either of them.
    ///
    /// # The freeze criteria, and which one this table's promotion rests on
    ///
    /// Protocol §7 pre-registered four criteria before any freeze result
    /// existed, and the pre-registration
    /// (`docs/plans/2026-08-09-phase3.5c-freeze-prereg.md`) fixed their
    /// mechanics — one OS-entropy CLOSE MASTER `0xF524FFB9D041DCAE`, drawn and
    /// recorded in the execution ledger's freeze row before any criterion ran,
    /// never redrawn. **Criterion 1, the one the promotion rests on:** the
    /// screen-harness cell `lookahead-v1@V2` vs `lookahead-v1@V1`, 2P, N = 200
    /// design units / 400 games, run PRE-COMMIT — **θ̂ 0.6775, bootstrap 95%
    /// [0.6275, 0.7250], the interval ENTIRELY ABOVE the 0.5 null**, which is
    /// the first of §7's two pre-committed passing shapes. **Criterion 2, the
    /// preserved rung, reported with no directional requirement:**
    /// `greedy-v1@V2` vs `greedy-v1@V1` on the same field reads θ̂ 0.482500,
    /// [0.432500, 0.530000] — SPANNING the null, no §4.2 pause (the hand-set
    /// 0.40 line is nowhere near) — and it is shown rather than hidden, which is
    /// what criterion 2 exists for. Criteria 3 (the promotion field re-held via
    /// the arena runner at the same master) and 4 (the grown scenario suite plus
    /// the fifteen-gate roster) run AFTER this commit, by the pre-registration's
    /// own order of operations; the freeze package carries all four.
    ///
    /// **A scope limit on criterion 1 that the pre-registration did not
    /// anticipate, recorded here rather than smoothed over.** Criteria 1–2 ran
    /// pre-commit, so BOTH their arms evaluated with T3's post-claim filter
    /// ABSENT — the cell isolates the WEIGHT package and says nothing about the
    /// filter. The filter ships on protocol §4.2 L1's named-structural-argument
    /// arm (a claimed card's VP is already priced by
    /// [`Feature::VictoryPoints`]), on its round-1 corpus column, and on
    /// scenario 12, which lands red-then-green against the call-site switch
    /// itself. Re-running the same cell post-commit would measure a THIRD thing
    /// (both arms filtered) and is not what §7 pre-registered.
    ///
    /// **ROUND 1 ADOPTED, 2026-08-09 — the table BECAME the candidate `m22`, no
    /// longer V1's numbers, and the CONJUNCTIVE adoption rule was discharged:**
    /// (round 2 has since moved it again — see the ROUND 2 block below for what
    /// the numbers above are today; this block is kept as the round's history,
    /// because a table that records only its latest edit cannot be audited.)
    /// scenarios + hard constraints by the suite (13/13, hand-split ok both
    /// vectors), and the two L4 screens at the round's pre-recorded master
    /// `0x3EE753CE7F4052C4` — lookahead θ̂ 0.5725, bootstrap [0.5250, 0.6200]
    /// ENTIRELY ABOVE the null; greedy θ̂ 0.5150 spanning it, no 0.40 pause —
    /// all quoted with full provenance in the execution ledger's ROUND-1 row
    /// and the round-1 evidence artifact's Appendix D. (A round's screen field
    /// does not compose across rounds; the accumulated total is cashed only at
    /// the freeze's own fresh cell.) It moved SIX slots and every one of them
    /// is evidenced in
    /// `docs/plans/2026-08-09-phase3.5c-round1-evidence.md` (the L2 pass) over
    /// `docs/plans/2026-08-09-phase3.5c-r0-report.md` (the R0 corpora):
    ///
    /// - **the EARLY re-mix**, off R0's findings F1/F2 — EVAL TOTAL early
    ///   measures 0.5314 in the shipped regime, near chance, and the two terms
    ///   the early vector leant hardest on are the two the corpus CONTRADICTED
    ///   (`victory_points` 0.4651, `assistants_on_board` 0.4464, both pointing
    ///   at the seat that goes on to lose). Weight moves off them and onto the
    ///   members both regimes confirmed: `vp 10 → 2`, `ctry 12 → 18`,
    ///   `tech 6 → 12`, `hEXC 4 → 5`, `brd 14 → 6`. The MIX moves, not the
    ///   scale, and that is forced rather than stylistic — R0's finding F8 is
    ///   that a uniform rescale of the early vector is play-IDENTICAL, so only
    ///   a change of ratios can move a decision.
    /// - **the WAREHOUSE FLATTEN**, `late −4 → −10` (early unchanged at −10) —
    ///   the magnitude tension the T4 freeze recorded rather than resolved:
    ///   the term's measured separation does not decay late, while the shipped
    ///   weight does. See `V1`'s doc, which deferred exactly this to 3.5.
    ///
    /// **SELECTION RULE, pre-registered before any figure existed** (the
    /// execution ledger's ROUND-1 row, committed ahead of the run): candidates
    /// are evaluated on the two FIXED incumbent-play corpora at their recorded
    /// masters, split fit/held-out by generation-index parity, and the winner is
    /// the held-out EARLY argmax on the shipped-regime corpus. `m22` took it —
    /// 0.5692 [0.5333, 0.6030] against the incumbent's 0.5313 [0.4944, 0.5651],
    /// i.e. its interval excludes chance where the incumbent's spans it — and it
    /// is in fact the best of the nineteen on ALL SIX (corpus × stage) held-out
    /// columns, which is what makes a nineteen-way held-out selection
    /// defensible at this N. **`m22`'s weight identity was
    /// `0x73bf3f54421717a8`** (the incumbent's is `0xe1fedd5efc49d0f6`); the two
    /// no longer collided, so a screen against the draft became a real cell
    /// rather than the null one it had been. The identity the table carries
    /// TODAY is round 2's, below.
    ///
    /// Read the honest limits with the numbers: those AUCs are measured on
    /// INCUMBENT-play corpora and are therefore off-policy for this table, the
    /// grid's winner sits at the grid's EDGE (recorded for round 2, deliberately
    /// not chased there — extending after seeing that would have been
    /// selection-after-results), and the on-policy checks are the round's arena
    /// screen and, for the accumulated total, the freeze's own fresh cell.
    ///
    /// **ROUND 2 ADOPTED, 2026-08-09 — this table is the candidate `M*`, and
    /// the CONJUNCTIVE adoption rule is discharged:** protocol §4.2's rungs
    /// L1–L3 (the R0 evidence, the pre-registered two-stage L2 fit, and the
    /// scenario suite — 14/14 with the hand-split ordering holding in both
    /// vectors) AND the two L4 screens at the round's pre-recorded master
    /// `0x3A6B04285CCB85E0` — lookahead θ̂ 0.6750, bootstrap [0.6275, 0.7225]
    /// ENTIRELY ABOVE the null (round 1 read 0.5725 on ITS field; rounds do not
    /// compose); greedy θ̂ 0.4875, [0.4400, 0.5325], SPANNING the null with no
    /// 0.40 pause — the preserved rung's near-null reading is REPORTED, not
    /// hidden, and is coherent with the round's content (the late church
    /// down-pricing pays where the shipped rung's half-length games live in the
    /// LATE vector). Full blocks: the round-2 evidence artifact's Appendix E;
    /// the ledger's ROUND-2 row carries the verdict. The round's own
    /// pre-commitment is what makes the landing order safe: the master was
    /// drawn from OS entropy and recorded in the execution ledger's ROUND-2 row
    /// before any evidence ran, and was never redrawn (§4.3), so nothing about
    /// landing the table first could shop for a field. It moved SEVEN weights across six
    /// features over `m22` — one of the six, `hand_size`, moves in BOTH vectors —
    /// and every one is evidenced in
    /// `docs/plans/2026-08-09-phase3.5c-round2-evidence.md` (the two-stage L2)
    /// over `docs/plans/2026-08-09-phase3.5c-r0-report.md` (the R0 corpora):
    ///
    /// - **the LATE BONUS CROSSING** — `church_bonus_now` late `16 → 8` and
    ///   `five_power_tokens` late `10 → 14`, the round's dominant pair and the
    ///   only two slots whose decision a scenario can separate. R0's finding F7,
    ///   in the SHIPPED lookahead regime: the church bonus late is NO-SIGNAL at
    ///   0.5180 [0.4915, 0.5437] while carrying a +16 weight — the church race is
    ///   near-universal (ChurchTrackFull is 5-for-5 across every corpus in
    ///   existence), so the AWARDED bonus stops separating winners from losers and
    ///   the weight was paying for a near-constant — while five-power tokens late
    ///   measure 0.8058 [0.7874, 0.8236] on a weight of 10, the second-strongest
    ///   late member of the table after `victory_points`. Note what is NOT claimed:
    ///   the church TERM is not down-priced because the church is unimportant, but
    ///   because the bonus-as-awarded carries no information in this regime — a
    ///   margin/denial FEATURE might, and that is a T5 admission candidate for a
    ///   later round, not a re-pricing.
    /// - **the HAND-SIZE SOFTENING**, `−3/−6 → −1/−2` (both vectors) — no-signal
    ///   at every stage under lookahead, though the greedy corpus still confirms
    ///   the SIGN late (0.4320). Softened, deliberately not zeroed: the sign has
    ///   evidence behind it and the suite's leak clauses compare weighted
    ///   PRODUCTS, so a zeroed term would silently stop separating fixtures.
    ///   Recorded honestly — this component's held-out read was MARGINAL
    ///   (+0.0001 on its home stage); it rides the round's pre-registered merge
    ///   rule rather than a demonstration of its own.
    /// - **the EARLY EXTENSION**, `vp 2 → 1`, `ctry 18 → 20`, `tech 12 → 14` —
    ///   round 1's grid winner sat at the grid's EDGE on a monotone axis, which
    ///   that round recorded and refused to chase after the fact; round 2
    ///   PRE-REGISTERED the follow-up step as a grid cell and it improved —
    ///   held-out EARLY on the shipped-regime corpus 0.5692 (`m22`) → 0.5748 for
    ///   this delta alone, and 0.5893 for the merged `M*`. Its direction is the
    ///   same one scenario 10 already pins, and `M*` widens that scenario's
    ///   margin (+165 → +205) rather than narrowing it.
    ///
    /// **SELECTION RULE, pre-registered before any figure existed** (the
    /// execution ledger's ROUND-2 row, committed ahead of the runs): a TWO-STAGE
    /// fit on the same two FIXED incumbent-play corpora at their recorded masters
    /// and the same parity split — stage 1 scores seven single-slot deltas plus
    /// their full merge, stage 2 evaluates the DETERMINISTIC merge of exactly
    /// those singles that improved on `m22` in their HOME stages, with no
    /// discretion in the merge rule. Primary = mean held-out MID+LATE EVAL-TOTAL
    /// AUC on the shipped-regime corpus: **`M*` 0.8188 > c1 0.8180 > K 0.8177 >
    /// `m22` 0.8112.** The margin over the runner-up is small (+0.0009, inside
    /// bootstrap widths) and the tie-break that carries it is the same
    /// cross-corpus unanimity round 1 leant on — `M*` leads all six (corpus ×
    /// stage) held-out columns. Two proposed deltas were EXCLUDED by the rule —
    /// `trad_l 25→18` and the early `brd 6→5, exc 5→4` step, both below baseline
    /// on their home stages, the second negative on the greedy corpus too — which
    /// is the merge rule doing its job rather than a grid that happened to agree
    /// with itself. **Weight identity `0x9a240f2ff7624b0e`**, verified at this
    /// commit against the probe's own digest of the two arrays below.
    ///
    /// Read the honest limits with these numbers too, and they are round 1's
    /// unchanged: the AUCs are measured on INCUMBENT-play corpora and are
    /// off-policy for this table (`m22`-play would be the on-policy corpus, and
    /// nobody generates one mid-round), so the on-policy checks remain the round's
    /// arena screens and, for the accumulated total, the freeze's own fresh cell.
    /// A round's screen field does not compose across rounds.
    ///
    /// **What is pinned regardless of what a round adopts** — L14/T1's
    /// `core > on_board > excess` in BOTH vectors, the one-ply non-degeneracy
    /// condition protocol §4.2 forbids relaxing at any rung of the evidence
    /// ladder. It survives BOTH adoptions unchanged (early 30 > 6 > 5; late
    /// 20 > 10 > 2): round 1 moved `brd` and `hEXC` INSIDE it rather than through
    /// it, and round 2 left the triple alone entirely. The screen harness's
    /// `hand_split_ordering_holds` is asserted against THIS table per edit
    /// (`the_iteration_candidate_holds_the_hand_split_ordering`), so a round
    /// that re-prices the assistant triple past the ordering fails at test time
    /// rather than four hundred games into a screen.
    ///
    /// **The scenarios that guard the moved slots** (protocol §4.2 rung L3), in
    /// `tests/scenario_suite.rs`: *early engine over early VP* landed
    /// red-then-green against round 1's re-mix, *warehouse decay flattened*
    /// against the flatten, and *late bonus re-pricing: tokens over church*
    /// against round 2's late crossing — each with the round's own edit
    /// REVERTED as its counterfactual rather than a zeroed term. Round 2's two
    /// minor components carry no red-then-green fixture of their own and are
    /// not claimed to (the round's pre-registered change-SET reading, ledger
    /// ROUND-2 row): the EARLY extension's slots are property-and-red-mutation
    /// guarded by the early-engine scenario as it now runs against this draft —
    /// which was already green before the extension, so that is direction
    /// coverage, not a round-2 red — and the hand-size softening is
    /// scenario-uncovered DELIBERATELY, because its evidence is
    /// no-signal-toward-zero and a fixture pretending to discriminate it would
    /// pin noise.
    ///
    /// **ROUND 3 ADOPTED NOTHING, and it is part of this table's provenance for
    /// exactly that reason** (ledger ROUND-3 row; evidence
    /// `docs/plans/2026-08-09-phase3.5c-round3-evidence.md`). It was an L1
    /// EVIDENCE round over seven admission-candidate COLUMNS computed in the
    /// probe rather than in this file — the church/customs/tech majority-race
    /// margins, achievement-race proximity, church clock control, empty Port
    /// slots, and money — because an admission moves [`Feature::ALL`] and that
    /// is freeze-commit blast radius. Six of the seven cleared the
    /// pre-registered interval bar BY THE LETTER and the honest reading killed
    /// them anyway: the customs and tech margins DUPLICATE the shipped bonus
    /// mirrors (Δ ≤ +0.009, the customs pair identical to four decimals on the
    /// greedy corpus), church margin ≡ clock control ≡ the shipped church shape
    /// at 2P, money separates late only (0.5627), and empty Port slots are
    /// structurally unmeasurable in a regime where ChurchTrackFull is 5-for-5.
    /// The ONE genuine find, `achievement_race_proximity` (+0.054 EARLY /
    /// +0.094 MID over the shipped progress-sum in the shipped regime), was
    /// deliberately NOT admitted pre-freeze — modest absolute level, unmeasured
    /// incremental value over the terms already here — and is recorded as the
    /// post-freeze V3 cycle's first named candidate. The round's recorded master
    /// was retired UNUSED. **The stopping rule (§4.5's second arm) fired with
    /// that evidence, which is what sent the lane to §7 and produced this
    /// table** rather than a fourth round.
    ///
    /// # What this freeze does NOT establish (protocol §7's honesty register)
    ///
    /// * **The magnitudes are still hand-set.** They are now hand-set INFORMED
    ///   BY held-out fits on fixed corpora — protocol §4.2 rung L2 is explicit
    ///   that the fit is EVIDENCE FOR a hand-set number and never a shipped
    ///   machine weight — which is a better epistemic position than V1's and is
    ///   not the same thing as a tuned table. Weight TUNING is W6, after this
    ///   freeze, against this table.
    /// * **The screen fields do not compose.** Round 1 read θ̂ 0.5725 on its
    ///   master, round 2 read 0.6750 on its own, and criterion 1 read 0.6775 on
    ///   the close master. Those are three incomparable fields by construction
    ///   (§4.3 rotates the master every round precisely so the iteration cannot
    ///   overfit one), so they may not be multiplied, averaged or read as a
    ///   trend. **Criterion 1's single fresh cell IS the direction's basis** —
    ///   the accumulated total cashed once, on a field drawn after the last
    ///   adoption and never redrawn.
    /// * **The L1/L2 evidence is off-policy.** Every AUC quoted above was
    ///   measured on INCUMBENT-play corpora (R0-G under greedy@V1, R0-L under
    ///   lookahead@V1), so it describes how a term separates in the incumbent's
    ///   game, not in this table's. The screens and criterion 1 are the only
    ///   on-policy evidence in the record.
    /// * **Nothing here says the evaluator is good, only that it is better on
    ///   this field than V1 was.** The bar (map §5-3.5 / §1.9) is the owner's to
    ///   declare, last, and this table is one input to it.
    /// * **Two-player only.** Every corpus, fit, scenario and screen in the
    ///   trail is 2P. Nothing has measured what these weights do at 3–4P.
    pub const V2: WeightTable = WeightTable {
        //     [vp, ctry, tech, chur, cust, hCORE, hEXC, ware, brd, shop, trad, 5pw, imp, achv, merc, hnd#]
        early: [ 1,   20,   14,   -4,    3,    30,    5,  -10,   6,   12,   15,   8,   5,    2,    3,   -1],
        late:  [40,   30,   20,    8,   20,    20,    2,  -10,  10,   20,   25,  14,   8,    4,    4,   -2],
    };

    /// **THE ITERATION CANDIDATE, retained as an ALIAS of [`WeightTable::V2`]
    /// now that its job is done.** Through rounds 1–3 this const WAS the
    /// candidate — it moved twice and every round instrument named it. The
    /// freeze promoted its final value to `V2` verbatim, so the two hold the
    /// same thirty-two numbers and the same identity `0x9a240f2ff7624b0e`.
    ///
    /// **Why it is kept rather than deleted, and what that buys.** The round
    /// commands are committed, in the ledger rows and in both evidence
    /// artifacts, and several of them name this spelling — `eval_probe`'s
    /// `--score-table v2-draft` and `examples/eval_screen_round`'s candidate
    /// arm. Deleting the const would break the commands that reproduce the
    /// record. This is `PRE_FREEZE_2026_07_27`'s reasoning applied one rung
    /// down, with one HONEST DIFFERENCE that has to be said out loud: PRE_FREEZE
    /// pins its own literals and so reproduces a specific historical table,
    /// while this const is an ALIAS and therefore tracks V2. It reproduces the
    /// FINAL draft (round 2's `M*`, which is what V2 is) and nothing earlier —
    /// round 1's `m22` was already unrecoverable from this tree the moment
    /// round 2 overwrote it, as `late_bonus_reprice_reverted`'s doc in the
    /// scenario suite records. A future re-pricing of `V2` moves this alias with
    /// it; if a reader ever needs the 3.5c draft as a fixed historical table,
    /// that is a new const with its own literals, not this one.
    ///
    /// Prefer `V2` in new code. This spelling exists for the committed commands.
    pub const V2_DRAFT: WeightTable = Self::V2;

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
/// one per feature, always — including the zero-weighted ones, so a reader can
/// see that a term contributed nothing rather than wonder whether it was
/// computed.
///
/// THE EXACT-SUM IDENTITY (§0.3(a)), in the one form that is true everywhere:
///
/// ```text
/// total == terms.sum(product) + terminal.map(|t| t.value).unwrap_or(0)
/// ```
///
/// On every position a search actually evaluates positionally that reduces to
/// the spec's `total == terms.sum(product)` verbatim, because `terminal` is
/// `None`. On a FINISHED game the positional vector does not govern at all
/// (§2.6), so `explain` reports every term at `weight == 0` / `product == 0` —
/// reporting an interpolated weight there would be a lie about what [`score`]
/// used — while the raw `viewer_value`/`opponent_value` stay populated so a
/// terminal explanation is still readable. The `terminal` value is then the
/// whole score, and it is a named additive contribution rather than a
/// seventeenth `Feature` because it is not a feature: §2.2's list is sixteen
/// entries and a rank is not one of them.
///
/// `PartialEq`/`Eq` are load-bearing, not conveniences: spec §0.4(ii)'s
/// determinization cell asserts that a candidate's explanation is EQUAL across
/// sampled belief worlds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    pub terms: Vec<TermContribution>,
    /// `Some` exactly when [`view_is_terminal`] holds — the finished game's
    /// re-derived standing, and then the whole of `total`.
    pub terminal: Option<TerminalStanding>,
    pub total: i64,
    /// The stage proxy the weights were interpolated at, in ppm. Reported so an
    /// explanation is self-contained: every `weight` above is recomputable from
    /// this and the table (T10 renders it) — except on a terminal view, where
    /// the reported weights are 0 by the rule above and this field is
    /// informational only.
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
///
/// A FINISHED game short-circuits into [`TerminalStanding`] (§2.6) — the
/// positional vector never runs there.
pub fn score(view: &PlayerView, rules: &Rules, weights: &WeightTable) -> i64 {
    let Some(me) = seated_viewer(view) else {
        return 0;
    };
    // ONCE per call, not once per (feature, player): four of the five
    // score-structure re-derivations are rankings over the whole table, so the
    // naive shape would pay each of them `players` times over.
    let structure = ScoreStructure::derive(view, rules);
    if let Some(terminal) = terminal_standing_of(view, me, &structure) {
        return terminal.value;
    }
    let progress = progress_ppm(view, rules);
    let mut total: i64 = 0;
    for feature in Feature::ALL {
        let (early, late) = weights.get(feature);
        let weight = interpolate(early, late, progress);
        let mine = structure.value_of(me, feature);
        let theirs = opponent_extreme(view, me, feature, weight, &structure);
        total = total.saturating_add(weight.saturating_mul(mine.saturating_sub(theirs)));
    }
    total
}

/// The same evaluation, reconstructed term by term.
///
/// A SEPARATE PATH from [`score`] on purpose (§0.3(a)): it re-derives every
/// component and accumulates its own total, so `explain().total == score()` is
/// a cross-check between two assemblies rather than a restatement of one. The
/// two share only the primitives ([`ScoreStructure`], [`interpolate`],
/// [`opponent_extreme`]) — that is the sharing the property is meant to police,
/// not the sharing it is meant to hide.
///
/// See [`Explanation`] for the exact-sum identity in the form that covers
/// terminal views too.
pub fn explain(view: &PlayerView, rules: &Rules, weights: &WeightTable) -> Explanation {
    let progress = progress_ppm(view, rules);
    let structure = ScoreStructure::derive(view, rules);
    // A view without its own seat is degenerate rather than fatal (`observe`
    // cannot make one; a fixture can). It reports sixteen zero terms and a zero
    // total — the same answer `score` gives — instead of an empty explanation.
    let me = seated_viewer(view);
    let terminal = me.and_then(|m| terminal_standing_of(view, m, &structure));

    let mut terms = Vec::with_capacity(Feature::COUNT);
    let mut total: i64 = 0;
    for feature in Feature::ALL {
        let (early, late) = weights.get(feature);
        let weight =
            if terminal.is_some() { 0 } else { interpolate(early, late, progress) };
        let viewer_value = me.map_or(0, |m| structure.value_of(m, feature));
        let opponent_value =
            me.map_or(0, |m| opponent_extreme(view, m, feature, weight, &structure));
        let product = weight.saturating_mul(viewer_value.saturating_sub(opponent_value));
        total = total.saturating_add(product);
        terms.push(TermContribution { feature, viewer_value, opponent_value, weight, product });
    }
    if let Some(t) = terminal {
        total = total.saturating_add(t.value);
    }

    Explanation { terms, terminal, total, progress_ppm: progress }
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
    weight: i64,
    structure: &ScoreStructure,
) -> i64 {
    let mut best: Option<i64> = None;
    for other in &view.players {
        if other.id == me.id || other.id.is_dummy() {
            continue;
        }
        let value = structure.value_of(other, feature);
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

/// One player's RAW (absolute, un-weighted) feature value, as T4's probe reads
/// it.
///
/// Public because the probe needs exactly this: per-feature AUC is a statement
/// about raw values, and a probe re-deriving them would be a second
/// implementation of the feature set.
///
/// NOT the hot path, and deliberately not what [`score`] calls: it builds a
/// whole [`ScoreStructure`] to answer about one feature of one seat. A caller
/// asking about more than one `(feature, player)` pair at the same view should
/// [`ScoreStructure::derive`] once and use [`ScoreStructure::value_of`], which
/// is what both evaluation paths do.
pub fn raw_value(
    view: &PlayerView,
    player: &VisiblePlayer,
    feature: Feature,
    rules: &Rules,
) -> i64 {
    ScoreStructure::derive(view, rules).value_of(player, feature)
}

// ---------------------------------------------------------------------------
// The score-structure re-derivations (§2.2), computed once per score call
// ---------------------------------------------------------------------------

/// The widest seat vector any real view carries: the printed player counts stop
/// at four and `PlayerId`'s own validated range is `0..=3`.
pub const MAX_SEATS: usize = 4;

/// Contender slots in a track majority: the four seats plus the 2-player dummy
/// (finding D1 — it competes for a place even though it can never be awarded
/// one).
const MAX_CONTENDERS: usize = MAX_SEATS + 1;
/// Where the dummy's tally lives in a [`MAX_CONTENDERS`]-wide array.
const DUMMY_SLOT: usize = MAX_SEATS;

// ---------------------------------------------------------------------------
// V4 §3.2 — the view context the candidate features ride
// ---------------------------------------------------------------------------

/// The widest area vector any real board carries: `AreaId`'s own validated
/// range is `0..18`, and 4P's staggered pyramid (6+5+4+3) fills it exactly.
/// 2P and 3P carry 10 and 14, so the tail of every array below is unset rather
/// than absent — the arrays are sized by the TYPE, not by the position.
pub const MAX_AREAS: usize = 18;

/// A set of areas, one bit per `AreaId`.
///
/// Eighteen areas fit in a `u32` with fourteen bits spare, so every set the V4
/// features build is ONE register: the reachable-set walk `S7` runs has a
/// fixed-size bitset frontier rather than a queue, which is what keeps
/// §0.3(a)'s `score` allocs==0 pin true with [`ScoreStructure::derive`] on its
/// path (§3.2's allocation contract, in the words it uses: "counters / fixed
/// arrays / bitsets").
pub type AreaSet = u32;

/// One area's bit, or the EMPTY set for an area outside the printed range.
///
/// `AreaId::unchecked` exists for deserialization and does not validate, so an
/// out-of-range id is representable. It reads here as "in no set", which is the
/// answer every consumer below wants — a phantom area holds no assistants, is
/// adjacent to nothing, and is not on the board — rather than a shift overflow.
pub fn area_bit(area: AreaId) -> AreaSet {
    let i = usize::from(area.0);
    if i < MAX_AREAS {
        1 << i
    } else {
        0
    }
}

/// The areas of a set, ascending by `AreaId`.
///
/// Ascending order is load-bearing where a card breaks ties by lowest `AreaId`
/// (S4's best re-entry anchor), and allocation-free everywhere: the iterator is
/// the `u32` itself, cleared one low bit at a time.
pub fn areas_in(set: AreaSet) -> impl Iterator<Item = AreaId> {
    let mut rest = set;
    std::iter::from_fn(move || {
        if rest == 0 {
            return None;
        }
        let i = rest.trailing_zeros();
        rest &= rest - 1;
        Some(AreaId(i as u8))
    })
}

/// A set of order cards, one bit per `OrderCardId`.
///
/// The printed manifest is thirty-six cards keyed by printed corner index
/// 1..=36, so a `u64` holds the whole deck in ONE register with bit 0 spare —
/// which is what lets O1's located set and its complement ride
/// [`ScoreStructure::derive`] under §3.2's allocation contract, exactly as
/// [`AreaSet`] does for the board.
pub type OrderSet = u64;

/// One order card's bit, or the EMPTY set for an id outside a `u64`'s reach.
///
/// `OrderCardId` is a bare newtype over `u8` with no validated range, so an
/// id past 63 is representable by a hand-built registry. It reads here as "in
/// no set" rather than as a shift overflow; the debug assertion is what says
/// that answer is a defence and not a supported width.
pub fn order_bit(card: OrderCardId) -> OrderSet {
    debug_assert!(card.0 < 64, "an order id past 63 has no bit in an OrderSet");
    if card.0 < 64 {
        1u64 << card.0
    } else {
        0
    }
}

/// A set of countries, one bit per flag — the five-wide axis O5's attainable
/// scan and the country-set bonus both speak.
pub type CountrySet = u8;

/// One country's bit. Public because §7's audit cells read
/// [`ViewContext::attainable`] against named flags, and a test that had to
/// know the bit layout would be asserting the implementation rather than the
/// set.
pub fn country_bit(country: Country) -> CountrySet {
    1u8 << country_index(country)
}

/// The countries of a set, in [`country_index`] order.
///
/// Order is not load-bearing — O5 takes a MAX over the set and the country-set
/// loop is symmetric in the flags — and the iterator is the `u8` itself,
/// cleared one low bit at a time, so nothing allocates.
fn countries_in(set: CountrySet) -> impl Iterator<Item = Country> {
    let mut rest = set;
    std::iter::from_fn(move || {
        if rest == 0 {
            return None;
        }
        let i = rest.trailing_zeros() as usize;
        rest &= rest - 1;
        Some(COUNTRY_BY_INDEX[i])
    })
}

/// [`country_index`]'s inverse, as a table so the two cannot drift: the index
/// is an array position, and this is the array.
const COUNTRY_BY_INDEX: [Country; 5] = [
    Country::English,
    Country::American,
    Country::French,
    Country::German,
    Country::Dutch,
];

/// One order card's face in O1's HALF-VP units — `2·victory_points +
/// 2·imported_goods + money` (§3.2's O1 card).
///
/// The doubling is what keeps the two amortized terminal identities integer:
/// one imported good is one VP, and two Yen are one VP, so the card's value in
/// half-VP is a sum of whole numbers rather than a rounded one. **Icon faces
/// are deliberately absent** — they are O6's, in ICON units, and §3.1's owner
/// ruling is that they are never VP-mapped.
fn half_vp_face(card: &OrderCard) -> i64 {
    i64::from(
        2 * u32::from(card.rewards.victory_points)
            + 2 * u32::from(card.rewards.imported_goods)
            + u32::from(card.rewards.money),
    )
}

/// Every order card the port displays, both boards. Allocation-free: an
/// `Option<PortBoard>` iterates as at most one element.
fn port_offers(view: &PlayerView) -> impl Iterator<Item = OrderCardId> + '_ {
    view.port_state
        .port_a
        .slots
        .iter()
        .flatten()
        .copied()
        .chain(view.port_state.port_b.iter().flat_map(|b| b.slots.iter().flatten().copied()))
}

/// Every technology card the labs display, both boards. Same shape, same
/// reason.
fn open_lab_techs(view: &PlayerView) -> impl Iterator<Item = TechCardId> + '_ {
    view.lab_state
        .lab_a
        .slots
        .iter()
        .flatten()
        .copied()
        .chain(view.lab_state.lab_b.iter().flat_map(|b| b.slots.iter().flatten().copied()))
}

/// The PUBLIC board and seat context V4 spec §3.2's view-context features read,
/// derived once per [`ScoreStructure::derive`] and indexed by `AreaId` or by
/// seat number.
///
/// # Why a context rather than seven private scans
///
/// §3.2 registers ten view-context features (S1–S7, X1–X3) plus two more
/// derivations, and every one of them starts from the same three questions:
/// what KIND of area is this, which areas hold this seat's assistants, and
/// which edges may a President cross for free. Deriving those once per call
/// makes each feature a fold over a `u32`; deriving them per feature would pay
/// the board scan up to eighteen times a call and would give eighteen places
/// for the Canal rule to be spelled differently.
///
/// # What is here, and what deliberately is not
///
/// This is CONTEXT, not values: the feature helpers compute from it on demand
/// rather than being folded into `derive` themselves. That split is the
/// milestone's own — the candidate features are probe-space until §7's step 8
/// moves `Feature::ALL`, and `score` must not pay for a reachability walk it
/// does not yet use.
///
/// **What `score` DOES pay, stated because the hot path is measured elsewhere.**
/// The context is derived unconditionally, so every `score` call now walks the
/// area map and the adjacency map once more — at most 18 areas and their ≤ 6
/// neighbours each, plus one pass over every seat's occupied areas. Fixed work,
/// no allocation (§0.3(a)'s pin still binds), and bounded by the board rather
/// than by the search. If a future measurement says that constant matters before
/// step 8 makes it load-bearing, the answer is to make the derivation lazy, not
/// to move it out of `derive` and give each feature its own scan.
///
/// **The seam for the later batches** (§3.2's remaining view-context work):
/// O1's located-card pool bitset and its running accumulators, O5's attainable
/// -source scan, and Group X's lab/port context all belong HERE as further
/// fields — a `u64` bitset and a pair of counters for the pool, masks for the
/// card boards — for the same reason the board masks do. Nothing in this type
/// is seat-relative except the two per-seat arrays, so adding a field costs one
/// pass in [`ScoreStructure::derive`] and nothing anywhere else.
///
/// # Honesty
///
/// Every input is public: the board layout and the station token are on the
/// table, and assistant positions and President locations are face-up for every
/// seat. Nothing here is masked information, and the type could not be handed
/// any — its only constructor reads a [`PlayerView`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewContext {
    /// Every area the layout carries. The bound on every other mask.
    pub present: AreaSet,
    /// Areas a pawn may stand on — [`Self::present`] minus the Canal. **A 2P
    /// board carries no Canal tile at all**, so this equals `present` there.
    pub placeable: AreaSet,
    /// The production areas (Copper Mine, Silk Mill, Tea Plantation, Fishing
    /// Ground) — S1's and X3's summation domain.
    pub production: AreaSet,
    /// The Church and the Customs House — S2's summation domain.
    pub tracks: AreaSet,
    /// The Bank — S5's.
    pub bank: AreaSet,
    /// The Employment Agency — S6's.
    pub employment_agency: AreaSet,
    /// FREE adjacency per area: the layout's `adjacency` MINUS its
    /// `canal_crossings`.
    ///
    /// A REAL subtraction, not a defensive one. The board generator adds each
    /// tolled crossing pair to BOTH maps (`board_gen.rs`'s
    /// `derive_canal_crossings` doc: "The caller adds each returned pair to both
    /// adjacency and `canal_crossings`"), so the raw adjacency map already
    /// contains the paid transits. S4's ring and S7's step relation are both
    /// declared over the FREE edges.
    pub free_adjacency: [AreaSet; MAX_AREAS],
    /// Per seat: the areas holding at least one of that seat's assistants —
    /// the engine's own action-legality condition (§3.2's presence gate).
    pub occupied: [AreaSet; MAX_SEATS],
    /// Per seat: the singleton set of that seat's President's area, EMPTY when
    /// the President is in hand. Read across seats: an opponent's President
    /// bars S4's re-entry targets and S7's counted areas.
    pub president: [AreaSet; MAX_SEATS],
    /// Every order card THIS view can point at: the completed piles of all
    /// seats, the Known entries of all hands, the face-up port offers, and the
    /// viewer's own starting order.
    ///
    /// Its COMPLEMENT within the printed registry is O1's unseen pool — the
    /// deck, the boxed setup discards, and every opponent's still-masked
    /// starting order, which is exactly the population a masked slot is drawn
    /// from. The viewer's own `starting_order` term is redundant on every view
    /// `observe` can produce (a kept card sits in the hand until it is
    /// fulfilled and in `completed_orders` after); §3.2's O1 card names it for
    /// the live-tracking case where a seat's own deal was recorded before its
    /// hand was reconciled, and the audit cell against
    /// `PlayerView::unseen_orders` is what keeps the redundancy true.
    pub located_orders: OrderSet,
    /// How many printed order cards [`Self::located_orders`] does NOT account
    /// for — the unseen pool's size.
    pub unseen_orders: u32,
    /// The summed [`half_vp_face`] of that pool. With the count above it is
    /// everything O1 needs from the pool, which is why the pool is two
    /// accumulators rather than a list.
    pub unseen_order_halfvp: i64,
    /// Per seat: the countries that seat could still ADD an icon of — O5's
    /// attainable set. The seat's KNOWN held orders, the face-up port offers,
    /// and the open lab titles the seat does not already own by EFFECT
    /// equality.
    ///
    /// Per SEAT rather than per table because the lab clause is: the rulebook
    /// lets each player own one card of a name, so the same open slot is a
    /// source for one seat and not for another.
    pub attainable_countries: [CountrySet; MAX_SEATS],
}

impl ViewContext {
    /// An empty context — no board, no seats.
    const EMPTY: Self = Self {
        present: 0,
        placeable: 0,
        production: 0,
        tracks: 0,
        bank: 0,
        employment_agency: 0,
        free_adjacency: [0; MAX_AREAS],
        occupied: [0; MAX_SEATS],
        president: [0; MAX_SEATS],
        located_orders: 0,
        unseen_orders: 0,
        unseen_order_halfvp: 0,
        attainable_countries: [0; MAX_SEATS],
    };

    /// The areas `area` reaches across a FREE edge (see
    /// [`free_adjacency`](Self::free_adjacency)); the empty set for an area
    /// outside the printed range.
    pub fn free_adjacent(&self, area: AreaId) -> AreaSet {
        let i = usize::from(area.0);
        if i < MAX_AREAS {
            self.free_adjacency[i]
        } else {
            0
        }
    }

    /// Areas holding some OTHER seat's President, from `seat`'s side. The
    /// registered cross-seat read of §5.2's family 5(a).
    pub fn opponent_presidents(&self, seat: usize) -> AreaSet {
        let mut set = 0;
        for (other, mask) in self.president.iter().enumerate() {
            if other != seat {
                set |= mask;
            }
        }
        set
    }

    /// The seat's own-assistant areas, Canal excluded — S3's count, and the
    /// domain S4 and S7 narrow further.
    pub fn occupied_placeable(&self, seat: usize) -> AreaSet {
        if seat >= MAX_SEATS {
            return 0;
        }
        self.occupied[seat] & self.placeable
    }

    /// The `PlacePresident` target class MODULO the placement fee: the seat's
    /// own-assistant, non-Canal, opponent-President-free areas
    /// (`legal_moves.rs:275-282` gates exactly these three, then filters by
    /// affordability, which §3.2's S4 card declares unmodeled).
    ///
    /// S4's in-hand branch maximizes over this set; S7's in-hand branch counts
    /// it.
    pub fn reentry_targets(&self, seat: usize) -> AreaSet {
        self.occupied_placeable(seat) & !self.opponent_presidents(seat)
    }

    /// The FLOOR of the unseen pool's mean half-VP face — what O1 pays for an
    /// opponent's masked `HiddenStartingOrder` slot, and 0 on an empty pool.
    ///
    /// **p-INDEPENDENT** (§5.2 family 5(a)'s registered cross-seat read): the
    /// pool is a property of the table, so every masked slot at a view reads
    /// this one number whichever seat's row is being valued.
    ///
    /// The floor rather than the mean because the feature axis is integer, and
    /// the estimator's two registered biases pull opposite ways anyway: a kept
    /// starting order is selection-biased UP out of two dealt cards, and the
    /// boxed twin biases the pool DOWN.
    pub fn unseen_order_floor(&self) -> i64 {
        if self.unseen_orders == 0 {
            return 0;
        }
        self.unseen_order_halfvp / i64::from(self.unseen_orders)
    }

    /// The countries `seat` could still add an icon of — O5's summation
    /// domain; the empty set for a seat outside the printed range.
    pub fn attainable(&self, seat: usize) -> CountrySet {
        if seat >= MAX_SEATS {
            return 0;
        }
        self.attainable_countries[seat]
    }
}

/// Every quantity that is a property of the WHOLE table rather than of one
/// seat, re-derived from the view ONCE and indexed by seat number.
///
/// # Why this type exists
///
/// Three of these (technology, church, customs) are RANKINGS: computing one
/// seat's value requires ranking every seat, so an accessor that computed them
/// on demand would pay `O(track + seats)` once per `(feature, player)` pair —
/// sixteen rankings per 4P `score` call for four features. Deriving them once
/// into stack arrays makes a `score` call pay each exactly once. The other two
/// (country, achievement progress) are per-seat but are folded in here anyway,
/// so that "the score-structure features come from the structure" has no
/// exceptions to remember.
///
/// # What the arrays hold
///
/// Everything is indexed by SEAT NUMBER (`PlayerId.0`), not by position in
/// `view.players`. A seat outside `0..MAX_SEATS`, and the dummy sentinel, are
/// skipped: neither has a row here, and only a hand-built view can contain one.
///
/// # Allocation
///
/// Nothing here allocates — fixed arrays, registry lookups, and iteration over
/// maps the view already owns. §0.3(a)'s pin depends on that.
///
/// # Honesty
///
/// Every input is a field of the view or of the static card registries. In
/// particular the country and technology mirrors read `completed_orders` and
/// `technologies`, which are face-up for every seat; the majorities read the
/// two management-board tracks, which are on the table. Nothing here can see a
/// masked hand card, and the signature could not hand it one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreStructure {
    /// Seats with a row here — `max(seat number) + 1` over the view's players.
    pub seats: usize,
    /// Running victory points (the same field [`Feature::VictoryPoints`]
    /// reads), carried so [`TerminalStanding`] can total a seat without a
    /// second pass over the players.
    pub base_vp: [i64; MAX_SEATS],
    /// The country-set end bonus.
    pub country: [i64; MAX_SEATS],
    /// The technology industry-value majority (10 / 5).
    pub tech: [i64; MAX_SEATS],
    /// The church pawn majority (6 / 3), dummy competitor priced.
    pub church: [i64; MAX_SEATS],
    /// The customs pawn majority (8 / 4), dummy competitor priced.
    pub customs: [i64; MAX_SEATS],
    /// The end-scoring conversions of leftovers: unused mercenaries, imported
    /// goods, 1 VP per 2 Yen, 1 VP per 3 trade goods. NOT a feature (§2.2
    /// excludes money and raw goods as uninformative, R1-7) — it exists only
    /// because §2.6's final total is not a final total without it.
    pub remainder: [i64; MAX_SEATS],
    /// Summed progress toward the three Achievement cards in play.
    pub achievement: [i64; MAX_SEATS],
    /// V4 §3.2's view context — the board and seat facts the candidate feature
    /// helpers read. Derived unconditionally beside the rest; see
    /// [`ViewContext`] for why it is context rather than values.
    pub context: ViewContext,
}

impl ScoreStructure {
    /// Re-derive every component from `view` plus the static registries.
    pub fn derive(view: &PlayerView, rules: &Rules) -> Self {
        let mut s = Self {
            seats: 0,
            base_vp: [0; MAX_SEATS],
            country: [0; MAX_SEATS],
            tech: [0; MAX_SEATS],
            church: [0; MAX_SEATS],
            customs: [0; MAX_SEATS],
            remainder: [0; MAX_SEATS],
            achievement: [0; MAX_SEATS],
            context: ViewContext::EMPTY,
        };

        // V4 §3.2's board context, BEFORE the seat loop: the per-seat masks
        // below are narrowed to the areas the layout actually carries, so the
        // board pass has to have run first.
        for (area, config) in &view.board_layout.areas {
            let bit = area_bit(*area);
            s.context.present |= bit;
            if config.area_type.allows_placement() {
                s.context.placeable |= bit;
            }
            if config.area_type.is_production() {
                s.context.production |= bit;
            }
            if matches!(config.area_type, AreaType::Church | AreaType::Customs) {
                s.context.tracks |= bit;
            }
            if config.area_type == AreaType::Bank {
                s.context.bank |= bit;
            }
            if config.area_type == AreaType::EmploymentAgency {
                s.context.employment_agency |= bit;
            }
        }
        for (area, neighbours) in &view.board_layout.adjacency {
            let i = usize::from(area.0);
            if i >= MAX_AREAS {
                continue;
            }
            for n in neighbours {
                s.context.free_adjacency[i] |= area_bit(*n);
            }
        }
        // The tolled pairs come OUT of both directions — the layout carries
        // them in `adjacency` too (see [`ViewContext::free_adjacency`]).
        for (a, b) in &view.board_layout.canal_crossings {
            let (i, j) = (usize::from(a.0), usize::from(b.0));
            if i < MAX_AREAS {
                s.context.free_adjacency[i] &= !area_bit(*b);
            }
            if j < MAX_AREAS {
                s.context.free_adjacency[j] &= !area_bit(*a);
            }
        }

        // V4 §3.2's ORDER-CARD context, also before the seat loop: O1's pool is
        // the complement of a set every seat contributes to, so it has to be
        // complete before any row can be valued against it. Port offers are
        // walked once here because they feed BOTH the located set and the
        // seat-independent half of O5's attainable sources.
        let mut offered_countries: CountrySet = 0;
        for id in port_offers(view) {
            s.context.located_orders |= order_bit(id);
            if let Some(card) = rules.orders.get(id) {
                offered_countries |= country_bit(card.country);
            }
        }
        for p in &view.players {
            for card in &p.hand {
                if let HandCard::Known(id) = card {
                    s.context.located_orders |= order_bit(*id);
                }
            }
            for id in &p.completed_orders {
                s.context.located_orders |= order_bit(*id);
            }
            if let Some(id) = p.starting_order {
                s.context.located_orders |= order_bit(id);
            }
        }
        // ONE pass over the registry, testing membership rather than looking
        // each located card up again: the accumulators are the pool's, so the
        // complement is taken here and never materialized.
        for card in rules.orders.iter() {
            if s.context.located_orders & order_bit(card.id) == 0 {
                s.context.unseen_orders += 1;
                s.context.unseen_order_halfvp += half_vp_face(card);
            }
        }

        for player in &view.players {
            let Some(i) = seat_index(player) else { continue };
            // O5's attainable sources: the offers every seat shares, this
            // seat's KNOWN hand, and the open lab titles it does not own.
            let mut attainable = offered_countries;
            for card in &player.hand {
                if let HandCard::Known(id) = card {
                    if let Some(order) = rules.orders.get(*id) {
                        attainable |= country_bit(order.country);
                    }
                }
            }
            for id in open_lab_techs(view) {
                let Some(tech) = rules.techs.get(id) else { continue };
                // EFFECT equality IS the ownership-by-title test the engine
                // applies (`actions/cards.rs:268-270`): effects are 1:1 with
                // printed names, and ten of the twenty titles circulate as two
                // physical copies precisely so two seats can each hold one.
                let owns_title = player.technologies.iter().any(|owned| {
                    rules.techs.get(*owned).is_some_and(|t| t.effect == tech.effect)
                });
                if !owns_title {
                    attainable |= country_bit(tech.country);
                }
            }
            s.context.attainable_countries[i] = attainable;
            s.seats = s.seats.max(i + 1);
            for (area, count) in &player.assistants.on_board {
                if *count >= 1 {
                    s.context.occupied[i] |= area_bit(*area);
                }
            }
            // Narrowed to the board: an `on_board` key the layout does not
            // carry is a phantom, and one dropped here is dropped for every
            // consumer at once.
            s.context.occupied[i] &= s.context.present;
            if let PresidentLocation::OnBoard(area) = player.president_location {
                s.context.president[i] = area_bit(area);
            }
            s.base_vp[i] = i64::from(player.victory_points);
            s.country[i] = country_set_bonus(player, rules);
            s.remainder[i] = remainder_bonus(player);
            // THE T3 CALL SITE, switched at the v2 freeze (2026-08-10;
            // [`EVAL_IMPL_VERSION`] 3). Per SEAT, so each row drops only the
            // cards THAT row's player has already claimed — an opponent's claim
            // leaves this seat's leg live, which is the rules' own shape.
            s.achievement[i] = achievement_progress_postclaim_sum(view, player);
        }

        technology_majority(view, rules, &mut s.tech);
        majority_bonus(
            view.church_track.slots.iter().map(|slot| slot.map(|occupant| occupant.player)),
            [6, 3],
            &mut s.church,
        );
        majority_bonus(
            view.customs_track.slots.iter().map(|slot| slot.map(|occupant| occupant.player)),
            [8, 4],
            &mut s.customs,
        );

        s
    }

    /// One player's raw feature value. The five score-structure features index
    /// this table; the other ten read a field of `player` directly.
    pub fn value_of(&self, player: &VisiblePlayer, feature: Feature) -> i64 {
        let seat = seat_index(player);
        let structured = |vector: &[i64; MAX_SEATS]| seat.map_or(0, |i| vector[i]);
        match feature {
            Feature::VictoryPoints => i64::from(player.victory_points),

            Feature::CountryBonusNow => structured(&self.country),
            Feature::TechBonusNow => structured(&self.tech),
            Feature::ChurchBonusNow => structured(&self.church),
            Feature::CustomsBonusNow => structured(&self.customs),
            Feature::AchievementProgressSum => structured(&self.achievement),

            // The piecewise split, and the only place its shape lives: the two
            // raw values SUM to `in_hand` at every count, so the pair carries
            // exactly the information the single feature did and the weights
            // decide what to do with it.
            Feature::AssistantsInHandCore => {
                i64::from(player.assistants.in_hand.min(HAND_CORE_RESERVE))
            }
            Feature::AssistantsInHandExcess => i64::from(
                player.assistants.in_hand - player.assistants.in_hand.min(HAND_CORE_RESERVE),
            ),
            Feature::AssistantsInWarehouse => i64::from(player.assistants.in_warehouse),
            // Summed as i64, not through `AssistantState::total`'s u8
            // arithmetic: this fold must not depend on the pawn supply staying
            // under 256.
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

    /// One seat's FULL final score: running VP plus every end bonus. This is
    /// the quantity §2.6 ranks, and the drift pin holds it against the engine's
    /// own per-player total.
    pub fn final_total(&self, seat: usize) -> i64 {
        if seat >= MAX_SEATS {
            return 0;
        }
        self.base_vp[seat]
            + self.country[seat]
            + self.tech[seat]
            + self.church[seat]
            + self.customs[seat]
            + self.remainder[seat]
    }
}

/// A player's row index, or `None` for the dummy sentinel and for any seat
/// number outside the printed range (only a hand-built view has one).
fn seat_index(player: &VisiblePlayer) -> Option<usize> {
    if player.id.is_dummy() {
        return None;
    }
    let i = usize::from(player.id.0);
    (i < MAX_SEATS).then_some(i)
}

/// A country's slot in the 5-wide count array. An index, not a rule — the
/// set-matching loop below is symmetric in the countries, so the ORDER here
/// cannot affect a result.
fn country_index(country: Country) -> usize {
    match country {
        Country::English => 0,
        Country::American => 1,
        Country::French => 2,
        Country::German => 3,
        Country::Dutch => 4,
    }
}

/// The country icons a seat's two FACE-UP piles carry, as a five-wide count
/// array indexed by [`country_index`].
///
/// Completed orders and owned technologies, and nothing else — hand cards are
/// commitments, not icons. Public information on both counts, which is why the
/// incumbent country bonus and O5's marginal can both be read from a view.
fn country_icons(player: &VisiblePlayer, rules: &Rules) -> [u8; 5] {
    let mut icons = [0u8; 5];
    for id in &player.completed_orders {
        if let Some(card) = rules.orders.get(*id) {
            let i = country_index(card.country);
            icons[i] = icons[i].saturating_add(1);
        }
    }
    for id in &player.technologies {
        if let Some(card) = rules.techs.get(*id) {
            let i = country_index(card.country);
            icons[i] = icons[i].saturating_add(1);
        }
    }
    icons
}

/// The country-set end bonus over an icon multiset: 2/3/4/5 DISTINCT countries
/// in one set score 2/4/8/12 VP, and a player forms as many sets as the icons
/// allow.
///
/// # ONE derivation (§13's refactor)
///
/// This is the CORE lifted out of [`country_set_bonus`], and it is `pub`
/// because O5 `FlagIconMarginal` is defined as a STEP of this function —
/// `set_bonus(icons + 1_c) − set_bonus(icons)`. A marginal computed against a
/// second copy of the ladder would be a second place for the 2/4/8/12 curve to
/// live; §3.2 says one, so there is one, and the incumbent feature and the
/// candidate marginal are the same arithmetic by construction.
///
/// The matching is the greedy one the rules describe and the engine implements:
/// repeatedly take one icon of every country still available, score that set's
/// size, and stop when fewer than two countries remain. Greedy is not a
/// heuristic here — taking one of each available country is simultaneously the
/// largest set formable and the one that leaves the most behind, so the loop is
/// order-independent and the array's country order cannot matter. Icons do NOT
/// inflate a set: four English icons are one English country, four times over
/// across four sets.
///
/// Termination: every iteration decrements at least two counters.
pub fn set_bonus(icons: [u8; 5]) -> i64 {
    let mut icons = icons;
    let mut total = 0i64;
    loop {
        let distinct = icons.iter().filter(|n| **n > 0).count();
        let set_vp = match distinct {
            5 => 12,
            4 => 8,
            3 => 4,
            2 => 2,
            _ => break,
        };
        total += set_vp;
        for n in icons.iter_mut() {
            if *n > 0 {
                *n -= 1;
            }
        }
    }
    total
}

/// The incumbent `CountryBonusNow`: [`set_bonus`] over the seat's own icons.
fn country_set_bonus(player: &VisiblePlayer, rules: &Rules) -> i64 {
    set_bonus(country_icons(player, rules))
}

/// The technology end bonus: 10 VP to the largest total industry value, 5 to
/// the second, ties broken CLOCKWISE from the start player (nearest wins) and
/// a player owning no technology never places (its industry value is 0, so it
/// sorts below every owner and the award is withheld).
///
/// D5 (owner ruling 2026-08-08, rulebook p.9 final-scoring NOTE): at two
/// players the setup dummy competes here too, at industry value EXACTLY 7.5 —
/// mirrored from `scoring::calculate_technology_bonus`, same doubled-integer
/// comparison (the dummy is 15; real doubled totals are even, so a tie
/// against the dummy is unrepresentable and its distance key is a
/// placeholder). A place the dummy takes is consumed and awarded to nobody —
/// `majority_bonus`'s finding-D1 semantics, restated over industry values.
fn technology_majority(view: &PlayerView, rules: &Rules, out: &mut [i64; MAX_SEATS]) {
    let seats = i32::from(view.config.player_count);
    if seats <= 0 {
        return;
    }
    /// Sentinel "seat" for the 2P dummy contender: never a real index, so the
    /// award loop below can consume its place without crediting anyone.
    const DUMMY_SEAT: usize = usize::MAX;
    // (seat, DOUBLED industry value, clockwise distance). Distance is unique
    // per real seat and unreachable for the dummy, so the ordering below is
    // total and no third key is needed.
    let mut first: Option<(usize, u32, i32)> = None;
    let mut second: Option<(usize, u32, i32)> = None;
    let better = |a: (usize, u32, i32), b: (usize, u32, i32)| a.1 > b.1 || (a.1 == b.1 && a.2 < b.2);
    let mut consider = |candidate: (usize, u32, i32)| {
        match first {
            Some(f) if !better(candidate, f) => match second {
                Some(s) if !better(candidate, s) => {}
                _ => second = Some(candidate),
            },
            _ => {
                second = first;
                first = Some(candidate);
            }
        }
    };

    for player in &view.players {
        let Some(i) = seat_index(player) else { continue };
        let industry: u32 = player
            .technologies
            .iter()
            .filter_map(|id| rules.techs.get(*id))
            .map(|card| u32::from(card.industry_cost))
            .sum();
        if industry == 0 {
            continue;
        }
        consider((i, 2 * industry, clockwise_distance(view, player.id)));
    }
    if view.config.player_count == 2 {
        consider((DUMMY_SEAT, 15, i32::MAX));
    }

    if let Some((i, _, _)) = first {
        if i != DUMMY_SEAT {
            out[i] += 10;
        }
    }
    if let Some((i, _, _)) = second {
        if i != DUMMY_SEAT {
            out[i] += 5;
        }
    }
}

/// A management-board majority: `vps[0]` to the most pawns, `vps[1]` to the
/// second, count ties broken by the RIGHTMOST occupied space.
///
/// FINDING D1, and the reason this takes owners rather than seats: at two
/// players a dummy pawn of an unused colour sits on the board and COMPETES. It
/// can out-count or out-position a real player and take a place — and that
/// place is then awarded to nobody, while still being CONSUMED, so the real
/// player behind it earns only the place it actually achieved. A seat holding
/// no pawn is not in the ranking at all, which is why a lone occupant collects
/// the first place and nobody collects the second.
///
/// The count-then-rightmost key is TOTAL: one space holds one pawn, so two
/// contenders cannot share a rightmost index.
fn majority_bonus(
    occupants: impl Iterator<Item = Option<PlayerId>>,
    vps: [i64; 2],
    out: &mut [i64; MAX_SEATS],
) {
    let mut tally = [(0u8, 0u8); MAX_CONTENDERS];
    for (index, owner) in occupants.enumerate() {
        let Some(owner) = owner else { continue };
        let slot = if owner.is_dummy() {
            DUMMY_SLOT
        } else if usize::from(owner.0) < MAX_SEATS {
            usize::from(owner.0)
        } else {
            continue;
        };
        tally[slot].0 = tally[slot].0.saturating_add(1);
        tally[slot].1 = tally[slot].1.max(index as u8);
    }

    let better = |a: (u8, u8), b: (u8, u8)| a.0 > b.0 || (a.0 == b.0 && a.1 > b.1);
    let mut first: Option<usize> = None;
    let mut second: Option<usize> = None;
    for (slot, entry) in tally.iter().enumerate() {
        if entry.0 == 0 {
            continue;
        }
        match first {
            Some(f) if !better(*entry, tally[f]) => match second {
                Some(s) if !better(*entry, tally[s]) => {}
                _ => second = Some(slot),
            },
            _ => {
                second = first;
                first = Some(slot);
            }
        }
    }

    if let Some(i) = first {
        if i != DUMMY_SLOT {
            out[i] += vps[0];
        }
    }
    if let Some(i) = second {
        if i != DUMMY_SLOT {
            out[i] += vps[1];
        }
    }
}

/// The leftover conversions the rulebook scores after the bonuses: 1 VP per
/// unused (face-up) mercenary, 1 per imported good, 1 per 2 Yen, 1 per 3 trade
/// goods. The trade-good division is over the SUM of the four goods, not per
/// good — four leftovers of 2 score 2, not 0.
fn remainder_bonus(player: &VisiblePlayer) -> i64 {
    let goods = i64::from(player.copper)
        + i64::from(player.silk)
        + i64::from(player.tea)
        + i64::from(player.seafood);
    i64::from(player.mercenaries.unused_count())
        + i64::from(player.imported_goods)
        + i64::from(player.money / 2)
        + goods / 3
}

/// The PRE-FREEZE achievement sum: each of the three cards in play contributes
/// its progress CAPPED at its own requirement, with no claim filter.
///
/// Capped because the feature measures nearness to a claim, not hoarding: a
/// player sitting on 12 copper is no closer to the 5-copper card than one
/// sitting on 5, and the VP for a card already taken has landed in
/// `victory_points` (which [`Feature::VictoryPoints`] already prices, so an
/// uncapped sum would double-count it and keep growing afterwards).
///
/// **NOT the shipped arithmetic since the v2 freeze** — the
/// [`Feature::AchievementProgressSum`] call site in [`ScoreStructure::derive`]
/// reads [`achievement_progress_postclaim_sum`] instead, because the cap stops a
/// claimed leg GROWING and never stopped it being COUNTED, which is the double
/// count T3 removed.
///
/// It stays, and it is `pub` since the freeze, because it is the base the filter
/// is defined as a filter OVER: `tests/eval_core.rs` holds the two against each
/// other and the post-claim SCENARIO uses it as its §0.2 counterfactual. Both of
/// those need the pre-freeze arithmetic to exist as ONE authored function rather
/// than as a copy re-typed in a fixture — a copy is exactly what would drift
/// from [`achievement_progress`]'s per-card caps the day those change.
pub fn achievement_progress_sum(view: &PlayerView, player: &VisiblePlayer) -> i64 {
    let mut sum = 0i64;
    for card in &view.achievements {
        let (current, needed) = achievement_progress(view, player, card.achievement_type);
        sum += i64::from(current.min(needed));
    }
    sum
}

/// **THE SHIPPED achievement feature since the v2 freeze** —
/// [`achievement_progress_sum`]'s value with each card's leg dropped for the
/// seat that has ALREADY CLAIMED it.
///
/// The evaluator-v2 protocol's ONE sanctioned feature change
/// (`docs/plans/2026-08-09-phase3.5c-evaluator-v2-protocol.md` §4.1(ii), §5's
/// T3 row). The pre-freeze sum kept paying a seat for progress toward a card it
/// had already taken, whose VP has already landed in
/// [`Feature::VictoryPoints`]; the cap only stops that leg GROWING, it does not
/// stop it being counted.
///
/// **The scope is EXACTLY the viewer's own claim, and the second clause is a
/// decision rather than an oversight:**
///
/// * a card whose `claimed_by` contains THE SEAT BEING VALUED contributes 0;
/// * a card claimed by an OPPONENT contributes exactly what the shipped
///   computation contributes. The rules let a second seat claim the same card
///   for the printed subsequent value — the engine's own claim gate refuses only
///   a repeat claim by the SAME seat — so an opponent's claim leaves this seat's
///   leg a live claim, and zeroing it would price a reachable reward at nothing.
///   Pinned in `tests/eval_core.rs` against that gate rather than against a
///   reading of the rulebook.
///
/// Everything else is the pre-freeze arithmetic verbatim,
/// [`achievement_progress`] included, so the AND-shaped cards' per-leg caps are
/// untouched: a claimed AND-card leaves at its CAPPED value.
///
/// **LANDED ADDITIVELY, SWITCHED AT THE FREEZE — one auditable event, as
/// planned.** In round 1 this function existed and nothing called it, so
/// `score`/`explain` were byte-inert and [`EVAL_IMPL_VERSION`] did not move.
/// The FREEZE COMMIT (2026-08-10) pointed
/// [`Feature::AchievementProgressSum`] at it inside [`ScoreStructure::derive`]
/// and absorbed the whole blast radius in the same commit — the drift pin's
/// achievement clause re-derived, the evaluator golden re-captured, the mcts
/// walk fixture re-probed, [`EVAL_IMPL_VERSION`] 2→3 and every controller const
/// that embeds this evaluator (protocol §7) — so the behaviour change is one
/// event rather than something smuggled inside a re-pricing round.
///
/// **View-measurable**, which is what makes it admissible at all: `claimed_by`
/// is public card state any seated player reads off the table, so nothing here
/// widens what an evaluator can see (§2.1).
pub fn achievement_progress_postclaim_sum(view: &PlayerView, player: &VisiblePlayer) -> i64 {
    let mut sum = 0i64;
    for card in &view.achievements {
        // The ONE difference from the shipped computation. Per-seat: an
        // opponent's claim is not this seat's business, see the doc.
        if card.claimed_by.contains(&player.id) {
            continue;
        }
        let (current, needed) = achievement_progress(view, player, card.achievement_type);
        sum += i64::from(current.min(needed));
    }
    sum
}

/// One player's `(current, needed)` progress toward one achievement, mirroring
/// the engine's own progress arithmetic (pinned against it per-edit).
///
/// The two AND-shaped cards are the only subtle arm and the encoding is the
/// engine's, deliberately: `needed` is the SUM of the two thresholds and
/// `current` sums each count CAPPED at its own threshold, so `current >=
/// needed` holds exactly when BOTH legs individually clear — reproducing the
/// AND rather than approximating it. An uncapped sum would call five
/// production areas plus one commercial a 3-production-plus-2-commercial claim.
///
/// This is progress ONLY. Whether a card can actually be claimed additionally
/// needs an assistant in hand and the card unclaimed by this player; neither is
/// a numeric progress and neither belongs in a feature value.
pub fn achievement_progress(
    view: &PlayerView,
    player: &VisiblePlayer,
    achievement: AchievementType,
) -> (u32, u32) {
    match achievement {
        // Category A: trade-good holdings.
        AchievementType::Copper5 => (player.copper, 5),
        AchievementType::Silk6 => (player.silk, 6),
        AchievementType::Tea7 => (player.tea, 7),
        AchievementType::Seafood7 => (player.seafood, 7),

        // Category B: collections.
        AchievementType::Money10 => (player.money, 10),
        AchievementType::Tokens5 => (
            u32::from(player.five_power_tokens) + u32::from(player.mercenaries.total()),
            5,
        ),
        AchievementType::Orders5 => (player.completed_orders.len() as u32, 5),
        AchievementType::Technologies4 => (player.technologies.len() as u32, 4),

        // Category C: building presence in DISTINCT areas.
        AchievementType::Production4 => (building_areas(view, player, AreaKind::Production), 4),
        AchievementType::Commercial4 => (building_areas(view, player, AreaKind::Commercial), 4),
        AchievementType::Prod3Comm2 => {
            let prod = building_areas(view, player, AreaKind::Production).min(3);
            let comm = building_areas(view, player, AreaKind::Commercial).min(2);
            (prod + comm, 5)
        }
        AchievementType::Prod2Comm3 => {
            let prod = building_areas(view, player, AreaKind::Production).min(2);
            let comm = building_areas(view, player, AreaKind::Commercial).min(3);
            (prod + comm, 5)
        }
    }
}

/// Which half of the board an achievement counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AreaKind {
    Production,
    Commercial,
}

/// DISTINCT areas of one kind where `player` has any building. Shophouses and
/// trading houses both count, and an area holding both counts ONCE — the
/// achievement is about board presence, not about buildings.
fn building_areas(view: &PlayerView, player: &VisiblePlayer, kind: AreaKind) -> u32 {
    let wanted = |area: &crate::types::AreaId| {
        view.board_layout.areas.get(area).is_some_and(|config| match kind {
            AreaKind::Production => config.area_type.is_production(),
            AreaKind::Commercial => config.area_type.is_commercial(),
        })
    };
    let mut count = 0u32;
    for area in player.shophouses.built.keys() {
        if wanted(area) {
            count += 1;
        }
    }
    for area in &player.trading_houses.built {
        if wanted(area) && !player.shophouses.built.contains_key(area) {
            count += 1;
        }
    }
    count
}

// ---------------------------------------------------------------------------
// V4 §3.2 — shared arithmetic: the arriving-power reading
// ---------------------------------------------------------------------------

/// The power `player` would ARRIVE at `area` with — V4 spec §3.2's shared
/// arithmetic, registered there as
/// `min(5, own_assistants(a) + 1 + own_shophouse(a) + own_trading_house(a) +
/// station(a))`.
///
/// Implemented ONCE, here, because §3.2 says once: S1, S2, S5, S6, X1, X2 and
/// X3 all price an option at the power the seat could act with, and seven
/// copies of one sum is seven things that can drift apart.
///
/// # A MIRROR, and it names what it mirrors
///
/// The engine's own power function (`power.rs:17-48`) is unreachable from this
/// module by §2.1's rule, so every term is restated here in the shipped ORDER —
/// president, assistants, shophouse, trading house, station token, then the cap
/// applied LAST — over fields the view already carries. Restating a rule is only
/// honest with a drift pin against its authority, and §7's audit contract part 4
/// is that pin: `tests/eval_core.rs`'s
/// `realizable_power_is_the_arriving_power_oracle_over_organic_play` compares
/// this reading, at every seat and every area of organic 2P and 3P play, against
/// the engine's own function evaluated on the same position with that seat's
/// president CONSTRUCTED AT the queried area.
///
/// # The one clause that is NOT the engine's, and why
///
/// **The president term is UNCONDITIONAL.** The engine adds it where the
/// president STANDS, because it is reporting the power of an action being taken
/// now; this reports the power an option would be REALIZED at, and reaching an
/// area is what a President move is for. The cap is applied after the sum, so
/// adding the term before capping is the arriving reading rather than a capped
/// reading plus one.
///
/// **Three DECLARED SHAPES ride that choice** (§3.2, verbatim in substance —
/// they are declared, not defects, and the bias they carry is positive and
/// known):
///
/// 1. These terms price an option PORTFOLIO. At most one area is activatable per
///    turn — two with an unused mercenary, whose power carries NO president +1
///    (`apply_move.rs:1226-1236`), so a mercenary activation is one power lower
///    than priced here.
/// 2. The president's CURRENT area is barred as next turn's destination by the
///    no-return-to-start rule, and still receives the +1.
/// 3. Reachability — the own-assistant chain and the fees along it — is not
///    applied inside the features that call this.
///
/// # Presence is the CALLER's gate
///
/// This function answers "with what power", never "may the seat act there":
/// `own_assistants >= 1` is the engine's action-legality condition and each
/// calling feature applies it itself (§3.2's presence-gate sentence). A caller
/// that skipped it would read a nonzero power at an area the seat has never
/// touched, since the president term alone is 1.
pub fn realizable_power(view: &PlayerView, player: &VisiblePlayer, area: AreaId) -> u8 {
    // 1. The president — UNCONDITIONAL; see above.
    let mut power: u8 = 1;
    // 2. Assistants.
    power += player.assistants.in_area(area);
    // 3. Shophouse.
    if player.shophouses.built.contains_key(&area) {
        power += 1;
    }
    // 4. Trading house.
    if player.trading_houses.built.contains(&area) {
        power += 1;
    }
    // 5. The station token, which is +1 for whoever acts there.
    if view.station_location == Some(area) {
        power += 1;
    }
    // 6. Cap LAST, over the summed terms.
    power.min(5)
}

// ---------------------------------------------------------------------------
// V4 §3.2 — Group S: spatial and action affordance
// ---------------------------------------------------------------------------

/// The seven Group-S candidate features of V4 spec §3.2, one method per formula
/// card.
///
/// # Shape, and why it is this one
///
/// Each is a per-seat `-> i64` over PUBLIC view state, riding
/// [`ScoreStructure::derive`]'s [`ViewContext`] — the same "derive once, answer
/// per seat" split [`ScoreStructure::value_of`] already uses for the five
/// score-structure mirrors. What differs is that these are CONTEXT-fed rather
/// than precomputed: they are probe-space until §7's step 8 moves
/// `Feature::ALL`, and `score` must not pay for a reachability walk it does not
/// yet use.
///
/// **The signatures are not uniform, deliberately.** Four of the seven take the
/// view because they price an option at [`realizable_power`] (whose station term
/// is a view field) or need an area's TYPE; the three that are pure topology
/// take only the seat's row. A signature that named an input the feature does
/// not read would weaken the §7 cleanliness claim it is there to support.
///
/// # The probe adapter's seam
///
/// §7's probe columns dispatch through a
/// `(view, player, candidate label, rules) -> Option<i64>` function. That
/// adapter belongs on the FIXTURE side, not here: the candidate labels are the
/// fixture module's enum, and an evaluator that imported them would invert the
/// dependency — the fixture harness reads `eval`, never the other way round.
/// So the adapter derives a [`ScoreStructure`] and matches its label onto these
/// methods, exactly as [`raw_value`] does for the incumbent sixteen.
///
/// # What Group S may read (§7's cross-group cleanliness assert)
///
/// Presence, power, position and the board. **No money, no imported goods, no
/// order cards** — those are Group E's and Group O's, and only Group X reads
/// both sides. `tests/eval_core.rs` scans these bodies for the forbidden fields.
///
/// # The declared shapes ride here
///
/// [`realizable_power`]'s three declared shapes (portfolio pricing, the barred
/// current area keeping its +1, unapplied reachability) are inherited by every
/// caller below, and S2 and S6 each add one of their own — both registered in
/// §3.2 as DECLARED SHAPES with a known positive bias, both pinned as
/// declarations in the audit cells rather than repaired.
impl ScoreStructure {
    /// **S1 `ProductionAccess`** — Σ over production areas the seat occupies of
    /// the engine's EXPORTED reward at [`realizable_power`].
    ///
    /// The reward comes from the §13 dispatcher (`actions/production.rs:37`),
    /// never from a table restated here: one job, one object. Units are
    /// goods per activation.
    ///
    /// Sparsity is real and is not a gate failure: a Copper Mine at power 2
    /// exports NOTHING, so a satisfied presence gate can still sum to zero
    /// (§3.2's cited fact; the dispatcher returns `Some(0)` there precisely so
    /// this reads as a price rather than as "not my area").
    pub fn production_access(&self, view: &PlayerView, player: &VisiblePlayer) -> i64 {
        let Some(seat) = seat_index(player) else { return 0 };
        let mut total = 0i64;
        for area in areas_in(self.context.occupied[seat] & self.context.production) {
            let Some(config) = view.board_layout.areas.get(&area) else { continue };
            let reward = exported_reward(config.area_type, realizable_power(view, player, area));
            debug_assert!(
                reward.is_some(),
                "the production mask admitted an area the dispatcher does not price"
            );
            total += i64::from(reward.unwrap_or(0));
        }
        total
    }

    /// **S2 `TrackAccess`** — Σ over the Church and the Customs House, where the
    /// seat has a presence, of [`realizable_power`]. Units are power-steps.
    ///
    /// **An ACCESS potential, deliberately NOT a legality mirror** (§3.2's
    /// OPTIMISTIC-PROXY DECLARATION). This reads > 0 at states where the track
    /// action is Skip-only — Church below total faith 4, Customs at zero
    /// imports, and full or blocked eligible spaces (the 2P dummy occupies
    /// faith-7 and space-4 all game). The positive bias is declared and known;
    /// `tests/eval_core.rs` pins the declaration against the engine's own
    /// eligibility functions so that "fixing" it into a legality mirror is a
    /// spec change rather than a tidy-up.
    pub fn track_access(&self, view: &PlayerView, player: &VisiblePlayer) -> i64 {
        let Some(seat) = seat_index(player) else { return 0 };
        areas_in(self.context.occupied[seat] & self.context.tracks)
            .map(|area| i64::from(realizable_power(view, player, area)))
            .sum()
    }

    /// **S3 `AssistantAreasDistinct`** — the count of non-Canal areas holding at
    /// least one of the seat's assistants. Units are areas.
    ///
    /// The SPREAD axis, in tension with the building legs by construction: a
    /// build needs power ≥ 4 at ONE area, which is a stacking requirement. A
    /// count has no cliff — every Recovery of a sole-pawn area, and every
    /// D6 track placement that empties one, is worth exactly one step.
    pub fn assistant_areas_distinct(&self, player: &VisiblePlayer) -> i64 {
        let Some(seat) = seat_index(player) else { return 0 };
        i64::from(self.context.occupied_placeable(seat).count_ones())
    }

    /// **S4 `PresidentLocalAccess`** — the assistants within one FREE step of the
    /// President's centre.
    ///
    /// Two branches, and they answer different questions:
    ///
    /// * `OnBoard(a)`: the ANCHOR `own_assistants(a)` plus the RING — the same
    ///   sum over `a`'s free-adjacent areas (free = the layout's adjacency MINUS
    ///   its tolled canal crossings).
    /// * `InHand`: the MAXIMUM of that same local sum over the seat's re-entry
    ///   targets ([`ViewContext::reentry_targets`] — the `PlacePresident` class
    ///   modulo the placement fee, which §3.2 declares unmodeled), and 0 when
    ///   the seat has no board assistants. Ties go to the lowest `AreaId`; the
    ///   ascending walk is what implements that, and the returned VALUE is
    ///   tie-invariant either way.
    ///
    /// **DECLARED ASYMMETRY** (§3.2): at the seat's own hand-off the anchor term
    /// is structurally 0 — Recovery drains the acted area, Telephone excepted —
    /// so the RING carries the hand-off signal, and the anchor is live at the
    /// mid-turn states the beam orders (`lookahead.rs:1230`).
    pub fn president_local_access(&self, player: &VisiblePlayer) -> i64 {
        let Some(seat) = seat_index(player) else { return 0 };
        match player.president_location {
            PresidentLocation::OnBoard(area) => self.local_assistant_mass(player, area),
            PresidentLocation::InHand => {
                let mut best = 0i64;
                for area in areas_in(self.context.reentry_targets(seat)) {
                    let mass = self.local_assistant_mass(player, area);
                    // STRICT: the first (lowest-`AreaId`) maximiser wins the tie.
                    if mass > best {
                        best = mass;
                    }
                }
                best
            }
        }
    }

    /// S4's local sum at one centre: the anchor plus its free-adjacent ring.
    ///
    /// The ring is NOT filtered to placeable areas and does not need to be — no
    /// pawn may ever stand on the Canal, so its term is structurally zero.
    fn local_assistant_mass(&self, player: &VisiblePlayer, area: AreaId) -> i64 {
        let anchor = i64::from(player.assistants.in_area(area));
        let ring: i64 = areas_in(self.context.free_adjacent(area))
            .map(|n| i64::from(player.assistants.in_area(n)))
            .sum();
        anchor + ring
    }

    /// **S5 `BankAccess`** — the engine's `bank_reward` at [`realizable_power`],
    /// gated on a presence at the Bank. Units are yen per activation (manual
    /// p.12: 1..5).
    pub fn bank_access(&self, view: &PlayerView, player: &VisiblePlayer) -> i64 {
        let Some(seat) = seat_index(player) else { return 0 };
        areas_in(self.context.occupied[seat] & self.context.bank)
            .map(|area| i64::from(bank_reward(realizable_power(view, player, area))))
            .sum()
    }

    /// **S6 `EmploymentAgencyAccess`** — the engine's token allowance at
    /// [`realizable_power`] (0/0/1/2/3/4 per power, manual p.14), capped by what
    /// the seat's warehouse actually holds, gated on a presence at the agency.
    /// Units are retrievable tokens.
    ///
    /// **The warehouse minimum repairs a structural zero** (§3.2): the warehouse
    /// is one-way and only drains, so an ungated allowance over-credits
    /// monotonically as the game runs.
    ///
    /// **OPTIMISTIC-PROXY DECLARATION**: the minimum gates SUPPLY, not
    /// AFFORDABILITY. A warehouse holding only trading-house tokens against an
    /// empty purse is Skip-only while this reads > 0 — token costs (assistants
    /// free, shophouses 2¥, trading houses 4/5/6/7) belong to the money term, and
    /// the affordability conditional is declared family-10-class. Pinned as a
    /// declaration in `tests/eval_core.rs`, not repaired.
    pub fn employment_agency_access(&self, view: &PlayerView, player: &VisiblePlayer) -> i64 {
        let Some(seat) = seat_index(player) else { return 0 };
        let warehouse = i64::from(player.assistants.in_warehouse)
            + i64::from(player.shophouses.in_warehouse)
            + i64::from(player.trading_houses.in_warehouse);
        areas_in(self.context.occupied[seat] & self.context.employment_agency)
            .map(|area| {
                let allowed =
                    employment_agency_tokens_allowed(realizable_power(view, player, area));
                i64::from(allowed).min(warehouse)
            })
            .sum()
    }

    /// **S7 `PresidentReachableAreas`** — how many areas are actionable in
    /// principle from where the President is. Units are areas.
    ///
    /// The count of [`Self::president_reachable_set`]; see that method for the
    /// walk, the two exclusions and the declared narrowings.
    pub fn president_reachable_areas(&self, player: &VisiblePlayer) -> i64 {
        i64::from(self.president_reachable_set(player).count_ones())
    }

    /// S7's counted SET, exposed because §7's audit contract compares it against
    /// the engine's own possible-action-area set rather than only its size.
    ///
    /// # On board: a walk, then two exclusions
    ///
    /// From the President's area, step along FREE edges ONTO areas holding at
    /// least one of the seat's assistants — the shape of the engine's own hop
    /// predicate (`can_step_onto` movement.rs:256-260), which has NO President
    /// clause. **Transit through an opponent's President is therefore
    /// UNRESTRICTED**; what an opponent's President blocks is STOPPING
    /// (`is_possible_action_area_full` movement.rs:92-101, waivable by Postal
    /// System), so those areas are walked through and then excluded from the
    /// count, along with the Canal.
    ///
    /// The President's OWN area is excluded at the step gate, which does double
    /// duty: it is `can_step_onto`'s `to != start` canonicalization, and it is
    /// §3.2's "MINUS the president's current area" — the no-return-to-start rule
    /// bars acting there next turn (`apply_move.rs:241-250`), and at a hand-off
    /// state the barred start area IS the current area.
    ///
    /// # In hand: no walk at all
    ///
    /// `PlacePresident` reaches every own-assistant, non-Canal,
    /// opponent-President-free area with no chain (`legal_moves.rs:275-282`), so
    /// the in-hand reading is the DIRECT [`ViewContext::reentry_targets`] set —
    /// no BFS, no anchor, and 0 with no board assistants.
    ///
    /// # Declared narrowings
    ///
    /// Tram, Postal System and mercenary bypasses, and every fee — placement,
    /// pass-through and canal tolls — are unmodeled (§3.1's scope).
    pub fn president_reachable_set(&self, player: &VisiblePlayer) -> AreaSet {
        let Some(seat) = seat_index(player) else { return 0 };
        match player.president_location {
            PresidentLocation::InHand => self.context.reentry_targets(seat),
            PresidentLocation::OnBoard(start) => {
                // The one bitset the walk needs: areas the seat occupies, minus
                // the President's own area (both exclusions, one mask), and
                // placeable — the Canal is not a hop the engine's own predicate
                // allows either (`can_step_onto`'s `allows_placement` test).
                // Card-literal conformance is unchanged: no view the rules can
                // reach stands a pawn on the Canal, so this narrows nothing in
                // play. It is here so a CONSTRUCTED view cannot transit it.
                let steppable =
                    self.context.occupied[seat] & self.context.placeable & !area_bit(start);
                let mut reached: AreaSet = 0;
                let mut frontier: AreaSet = area_bit(start);
                while frontier != 0 {
                    let here = frontier.trailing_zeros() as usize;
                    frontier &= frontier - 1;
                    let next = self.context.free_adjacency[here] & steppable & !reached;
                    reached |= next;
                    frontier |= next;
                }
                reached & self.context.placeable & !self.context.opponent_presidents(seat)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// V4 §3.2 — Group E: economic stocks and the order pipeline
// ---------------------------------------------------------------------------

/// The eight Group-E candidate features of V4 spec §3.2 — the three
/// E-resource cards (E1, E2, E3) and the five E-order cards (O1, O2, O3, O5,
/// O6) — one method per formula card.
///
/// # Shape, and how it differs from Group S's
///
/// Each is a per-seat `-> i64` over PUBLIC row state, in the same
/// "derive once, answer per seat" split. **Not one of them takes a
/// `&PlayerView`**: this group needs no board, no station token and no track.
/// What the five order cards take instead is `&Rules` — the printed order and
/// technology registries, which are on the table in the most literal sense.
/// Three take neither, being folds of the row's own integers.
///
/// The signature is the honesty claim, and §7's audit cell reads it off the
/// declaration rather than inferring it: a helper here can see a seat's money,
/// its goods, its face-up piles and the Known entries of its hand, and it can
/// see the manifest. It cannot see a masked card, because no argument carries
/// one.
///
/// # The two context terms
///
/// O1 and O5 ride [`ScoreStructure::derive`]'s order-card pass rather than
/// scanning for themselves, for the reason the board masks do: O1's pool is a
/// complement over the WHOLE table (every seat's piles and hands, and the port
/// display), and O5's attainable set folds the same offers for every seat. One
/// pass, two accumulators and a five-bit mask per seat; see [`ViewContext`].
///
/// # What Group E may read (§7's cross-group cleanliness assert)
///
/// Money, goods, tokens in hand, and the order pipeline. **No power, no
/// presence, no position, no board** — those are Group S's, and §3.2 registers
/// the interaction cell (Group X) as the ONLY group that reads both sides.
/// `tests/eval_core.rs` scans these bodies for the forbidden fields.
///
/// # The registered epistemic asymmetry
///
/// O1 is the one card here that prices something it cannot see. A seat's own
/// cards are valued at their true face; an opponent's masked slot is valued at
/// the unseen pool's floor. That is an ASYMMETRY, not an inconsistency, and it
/// is exactly what makes O1 a function of the view: substituting the hidden
/// card changes the holder's own reading and leaves every other seat's
/// untouched.
impl ScoreStructure {
    /// **E1 `MoneyStock`** — the purse, linear. Units are Yen.
    ///
    /// Every spend class (placement and movement fees, the church 2¥ leg, lab
    /// supplements at 2¥ per industry value, Employment-Agency token costs, the
    /// port's second card) debits this at the instant it is paid; nonlinear
    /// shapes — deficit pricing, timing — are a deferred layer, not a term.
    pub fn money_stock(&self, player: &VisiblePlayer) -> i64 {
        i64::from(player.money)
    }

    /// **E2 `TradeGoodsStock`** — `copper + silk + tea + seafood`, linear.
    /// Units are goods.
    ///
    /// Boundary: a fulfillment consumes exactly the printed requirements before
    /// the rewards land, and a church donation consumes singles. The
    /// after ≻ before ordering at those instants is a weight-side question
    /// (§5.2 families 6(a)/6(d)), not this term's.
    pub fn trade_goods_stock(&self, player: &VisiblePlayer) -> i64 {
        i64::from(player.copper)
            + i64::from(player.silk)
            + i64::from(player.tea)
            + i64::from(player.seafood)
    }

    /// **E3 `BuildingTokensReady`** — shophouse plus trading-house tokens IN
    /// HAND: retrieved from the warehouse and deployment-ready. Units are
    /// tokens.
    ///
    /// **The partition is the whole point** (§3.2): warehouse stock is S6's
    /// visible supply, hand tokens are E3's, and BUILT tokens are the
    /// incumbents' (`ShophousesBuilt` / `TradingHousesBuilt`). Without this
    /// middle term an Employment-Agency retrieval would trade S6-visible supply
    /// for nothing at all until the build lands — a boundary collapse, which is
    /// why the term is owner-ruled IN. The take that creates a token pays its
    /// printed cost, and E1 feels that.
    pub fn building_tokens_ready(&self, player: &VisiblePlayer) -> i64 {
        i64::from(player.shophouses.in_hand) + i64::from(player.trading_houses.in_hand)
    }

    /// **O1 `HeldOrderValue`** — the seat's hand in HALF-VP units: a Known card
    /// at `2·victory_points + 2·imported_goods + money`, an opponent's masked
    /// `HiddenStartingOrder` slot at the unseen pool's floor.
    ///
    /// **Icon faces are NOT priced here** — they are O6's, in ICON units, never
    /// VP-mapped (§3.1's owner ruling). What O1 carries is the resource-face
    /// residue: the printed VP star and the two amortized terminal identities.
    ///
    /// The hand is iterated WITHOUT assuming its length. The rules cap it at
    /// three; a live-tracking view can carry FOUR, because the masked slot is
    /// SYNTHESIZED ahead of the publicly-witnessed Port takes rather than
    /// replacing one of them.
    ///
    /// Partition: the commitment burden an unfulfilled order carries stays with
    /// the incumbent `HandSize`, which is negative and growing. O1 prices what
    /// the cards are worth; `HandSize` prices what holding them costs.
    pub fn held_order_value(&self, player: &VisiblePlayer, rules: &Rules) -> i64 {
        let floor = self.context.unseen_order_floor();
        player
            .hand
            .iter()
            .map(|card| match card {
                HandCard::Known(id) => rules.orders.get(*id).map_or(0, half_vp_face),
                HandCard::HiddenStartingOrder => floor,
            })
            .sum()
    }

    /// **O2 `OrderMatchedGoods`** — `Σ_g min(stock_g, total_req_g)` over the
    /// seat's KNOWN held orders. Units are matched goods.
    ///
    /// **The cap is per good and SUMMED across the hand**, which is what makes
    /// `O2 ≤ E2` an identity rather than a coincidence: each good contributes at
    /// most its own stock. A per-CARD cap would double-count a good two cards
    /// both need, and would break the bound.
    ///
    /// Masked slots contribute 0 — reading a requirement off a card the seat
    /// cannot see is exactly the honesty §3.1 excludes. Boundary: a fulfillment
    /// retires the card's contribution (§5.2 family 6(a)).
    pub fn order_matched_goods(&self, player: &VisiblePlayer, rules: &Rules) -> i64 {
        let mut required = [0u32; 4];
        for card in &player.hand {
            let HandCard::Known(id) = card else { continue };
            let Some(order) = rules.orders.get(*id) else { continue };
            required[0] += u32::from(order.requirements.copper);
            required[1] += u32::from(order.requirements.silk);
            required[2] += u32::from(order.requirements.tea);
            required[3] += u32::from(order.requirements.seafood);
        }
        let stock = [player.copper, player.silk, player.tea, player.seafood];
        stock.iter().zip(required.iter()).map(|(s, r)| i64::from(*s.min(r))).sum()
    }

    /// **O3 `ExecutableOrders`** — how many KNOWN held orders the seat could
    /// fulfil right now, each judged INDEPENDENTLY against the full stock.
    ///
    /// Independent rather than jointly allocated because the rules are: both
    /// Additional Action phases can fulfil, uncapped, and this is a static read
    /// of executability rather than of a sequence. Sequencing is the search's.
    ///
    /// **Imports and money never substitute for a trade good.** Masked slots
    /// are not counted.
    pub fn executable_orders(&self, player: &VisiblePlayer, rules: &Rules) -> i64 {
        player
            .hand
            .iter()
            .filter(|card| {
                let HandCard::Known(id) = card else { return false };
                let Some(order) = rules.orders.get(*id) else { return false };
                let r = order.requirements;
                player.copper >= u32::from(r.copper)
                    && player.silk >= u32::from(r.silk)
                    && player.tea >= u32::from(r.tea)
                    && player.seafood >= u32::from(r.seafood)
            })
            .count() as i64
    }

    /// **O5 `FlagIconMarginal`** — the best [`set_bonus`] STEP the seat could
    /// still buy: the maximum over its ATTAINABLE countries
    /// ([`ViewContext::attainable`]) of `set_bonus(icons + 1_c) −
    /// set_bonus(icons)`, where `icons` is the multiset over
    /// `completed_orders + technologies`. Units are VP.
    ///
    /// **0 on an empty attainable set** — a real zero, and the reason the
    /// computation is written over an `Option` rather than seeded at 0: the
    /// card says "max over the set, 0 when the set is empty", and those are two
    /// statements. (They happen to coincide, because adding an icon can never
    /// lower the bonus; writing the maximum exactly means a future curve where
    /// they did not coincide would be read correctly rather than clamped.)
    ///
    /// **No power or affordability gating** — that is Group X's half of the
    /// attribution split. This prices the STEP; whether the seat can reach and
    /// pay for the card is X1's and X2's question.
    ///
    /// Boundary: realizing the icon moves the step into `CountryBonusNow`, and
    /// the marginal then collapses — the 2/4/8/12 curve's own curvature, not the
    /// source disappearing.
    pub fn flag_icon_marginal(&self, player: &VisiblePlayer, rules: &Rules) -> i64 {
        let Some(seat) = seat_index(player) else { return 0 };
        let icons = country_icons(player, rules);
        let base = set_bonus(icons);
        let mut best: Option<i64> = None;
        for country in countries_in(self.context.attainable(seat)) {
            let mut with = icons;
            let i = country_index(country);
            with[i] = with[i].saturating_add(1);
            let step = set_bonus(with) - base;
            best = Some(best.map_or(step, |b: i64| b.max(step)));
        }
        best.unwrap_or(0)
    }

    /// **O6 `HeldOrderIcons`** — the movement and warehouse faces printed on the
    /// seat's KNOWN held orders, summed. Units are ICONS.
    ///
    /// **Never converted to VP**, and that is the owner's ruling with its
    /// grounds (§3.1): an icon's true value is action COMPRESSION — a
    /// fulfillment that also moves assistants or pulls tokens out of the
    /// warehouse can save a whole turn — and compression is sequencing value the
    /// §3 contract assigns to SEARCH. Within the search's horizon the icons
    /// price themselves exactly, because a fulfillment queues them as real
    /// choice-moves. This leaf term prices only the HELD OPTION, in its own
    /// unit, and the fit sets the tempo weight.
    ///
    /// The manifest carries two movement cards and five warehouse cards of
    /// thirty-six; the any-good face is structurally unreachable from an order.
    /// Masked slots show no face and contribute 0. Boundary: a fulfillment
    /// retires the count at the instant the icons' real value is realized
    /// through the choice queue — no double count, since the realized effects
    /// land in incumbent-priced stocks and positions.
    pub fn held_order_icons(&self, player: &VisiblePlayer, rules: &Rules) -> i64 {
        player
            .hand
            .iter()
            .map(|card| match card {
                HandCard::Known(id) => rules.orders.get(*id).map_or(0, |order| {
                    i64::from(
                        u32::from(order.rewards.movement)
                            + u32::from(order.rewards.warehouse_tokens),
                    )
                }),
                HandCard::HiddenStartingOrder => 0,
            })
            .sum()
    }
}

// ---------------------------------------------------------------------------
// Terminal views (§2.6)
// ---------------------------------------------------------------------------

/// The gap between adjacent finishing places in a [`TerminalStanding`]. Chosen
/// so that a place can never be outweighed: the margin refinement below is
/// clamped to under half a step, and the largest positional score the
/// provisional table produces on organic play is five orders of magnitude
/// smaller (measured and pinned in `tests/eval_core.rs`, which also states the
/// bound a future weight table must respect).
pub const TERMINAL_RANK_STEP: i64 = 1_000_000_000;
/// What one point of final-score margin is worth inside a place. Large enough
/// to dominate any positional score, small enough that the clamp below keeps
/// two margins from spanning a rank step.
pub const TERMINAL_MARGIN_STEP: i64 = 1_000_000;
/// The margin is clamped here so the ladder's ORDER is structural rather than
/// empirical: no final score, however extreme, can lift a second place above a
/// first. Two clamped margins span `2 · 4e8 = 8e8`, strictly under one rank
/// step, so even the worst-case pair of adjacent places cannot cross.
pub const TERMINAL_MARGIN_CLAMP: i64 = 400;

/// A finished game, re-derived from one seat's view (§2.6).
///
/// # What is scored
///
/// Not running VP — that is exactly the mis-ranking this type exists to
/// prevent, since the end bonuses are unbanked at the moment the game stops
/// and can be worth more than the VP already on the tracks. Every seat's total
/// is re-derived in full — running VP, country, technology, church, customs,
/// and the leftover conversions — the viewer is ranked under the official
/// clockwise tie-break, and the value returned is a WIN/LOSS ladder with the
/// final-score margin as a dominated refinement:
///
/// ```text
/// place  = if rank == 1 { +1 } else { 1 − rank }      // +1, −1, −2, −3
/// value  = TERMINAL_RANK_STEP · place
///        + TERMINAL_MARGIN_STEP · clamp(viewer_total − best_other_total)
/// ```
///
/// The asymmetry is the whole design and a symmetric place ladder would be a
/// BUG: with `seats + 1 − 2·rank`, second of four scores +1 step, so a bot
/// holding a mediocre running position would END the game to lock a
/// second place rather than play on for a win. Here every place but first is
/// negative and every running position scores between the two, so finishing
/// second is worse than continuing and finishing fourth is worse still.
///
/// # Magnitudes
///
/// Rank step 1e9, margin step 1e6, margin clamped to ±400 ⇒ |value| ≤ 3.4e9,
/// nine orders of magnitude below `i64::MAX`, and every arithmetic step
/// saturates anyway. Two clamped margins span 8e8 < one rank step, so no margin
/// can reorder two places. The dominance a caller may rely on: a win outscores,
/// and every loss undercuts, any non-terminal position whose positional total
/// stays under 6e8 — which the provisional table beats by ~10^4 on organic play
/// (measured and asserted in `tests/eval_core.rs`).
///
/// # The masked-order residual, stated precisely
///
/// The spec records that an opponent's unfulfilled starting order is masked and
/// that the bot therefore under-estimates them. The honest scoping: at a TRUE
/// terminal the re-derivation is EXACT, because a card still in hand scores
/// nothing — every component of the final total is face-up, which is why the
/// drift pin can hold this against the engine's own standings bit for bit. The
/// residual bites everywhere ELSE: the same five bonus mirrors are read while
/// the game is still running, as `*_now` features, and there an opponent's
/// masked order is a country icon and a VP reward the viewer cannot see coming.
/// The bot under-values opponents on the way to the end, not at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalStanding {
    /// The viewer's finishing place, 1 = winner.
    pub rank: u8,
    /// How many seats were ranked.
    pub seats: u8,
    /// The viewer's re-derived final total.
    pub viewer_total: i64,
    /// The best other seat's re-derived final total; equal to `viewer_total`
    /// when the view has no opponent (only a fixture can produce that).
    pub best_other_total: i64,
    /// What [`score`] returns for this view.
    pub value: i64,
}

/// Is this view a FINISHED game?
///
/// MIRROR of the engine's own derivation — end-game triggered, the final round
/// running with a recorded trigger player, and the turn resting at the end of
/// the seat before the start player, i.e. everyone has had their last turn.
/// Every input is public board state, so the mirror is exact rather than
/// approximate, and `tests/eval_core.rs` pins it against the engine over
/// organic play at 2/3/4P.
///
/// A `ViewConfig` carries no such method and building the full config to ask
/// would allocate inside §0.3(a)'s allocation-free path — mirror plus drift pin
/// is the same answer used for the two player-count constants above.
pub fn view_is_terminal(view: &PlayerView) -> bool {
    if !view.end_game_triggered || !view.final_round || view.trigger_player.is_none() {
        return false;
    }
    let seats = i32::from(view.config.player_count);
    if seats <= 0 {
        return false;
    }
    let last_seat = (i32::from(view.config.start_player.0) + seats - 1).rem_euclid(seats);
    i32::from(view.current_player.0) == last_seat && view.turn_phase == TurnPhase::TurnEnd
}

/// The viewer's finishing place and score, or `None` for a game still running.
///
/// Derives a whole [`ScoreStructure`]; [`score`] uses the internal form that
/// takes one already built.
pub fn terminal_standing(view: &PlayerView, rules: &Rules) -> Option<TerminalStanding> {
    let me = seated_viewer(view)?;
    terminal_standing_of(view, me, &ScoreStructure::derive(view, rules))
}

fn terminal_standing_of(
    view: &PlayerView,
    me: &VisiblePlayer,
    structure: &ScoreStructure,
) -> Option<TerminalStanding> {
    if !view_is_terminal(view) {
        return None;
    }
    let mine = seat_index(me)?;
    let viewer_total = structure.final_total(mine);
    let my_distance = clockwise_distance(view, me.id);

    let mut rank = 1u8;
    let mut seats = 0u8;
    let mut best_other: Option<i64> = None;
    for player in &view.players {
        let Some(i) = seat_index(player) else { continue };
        seats = seats.saturating_add(1);
        if i == mine {
            continue;
        }
        let total = structure.final_total(i);
        best_other = Some(best_other.map_or(total, |b: i64| b.max(total)));
        // The engine's ranking key verbatim: higher total first, then the
        // shorter clockwise distance from the start player. Distances are
        // unique per seat, so no pair can tie on the whole key.
        if total > viewer_total
            || (total == viewer_total && clockwise_distance(view, player.id) < my_distance)
        {
            rank = rank.saturating_add(1);
        }
    }

    let best_other_total = best_other.unwrap_or(viewer_total);
    let margin = viewer_total
        .saturating_sub(best_other_total)
        .clamp(-TERMINAL_MARGIN_CLAMP, TERMINAL_MARGIN_CLAMP);
    let value = TERMINAL_RANK_STEP
        .saturating_mul(place_signal(rank))
        .saturating_add(TERMINAL_MARGIN_STEP.saturating_mul(margin));

    Some(TerminalStanding { rank, seats, viewer_total, best_other_total, value })
}

/// The rank ladder's multiplier: `+1` for a win, `1 − rank` for everything
/// else. See [`TerminalStanding`] for why it is deliberately NOT symmetric.
pub fn place_signal(rank: u8) -> i64 {
    if rank <= 1 {
        1
    } else {
        1 - i64::from(rank)
    }
}

/// Seats clockwise from the start player, 0 for the start player itself — the
/// official tie-break key for both the technology majority and the final
/// ranking. `rem_euclid` rather than `%` so a hand-built config cannot underflow
/// the subtraction.
fn clockwise_distance(view: &PlayerView, player: PlayerId) -> i32 {
    let seats = i32::from(view.config.player_count);
    if seats <= 0 {
        return 0;
    }
    (i32::from(player.0) + seats - i32::from(view.config.start_player.0)).rem_euclid(seats)
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
///
/// MEASURED SHAPE (T1 review, FAMILY_WEIGHTED_DEFAULT full self-play, seed 77,
/// every step — inputs to T4's proxy-accuracy disposition, recorded here so
/// they are not re-derived): the proxy SATURATES at [`PROGRESS_FULL_PPM`] and
/// holds there for the last 7/86 turns at 2P, ~28% of turns at 3P and ~11% at
/// 4P (a consequence of the deck clock deliberately dropping the trigger's
/// port-empty-slot conjunct — the LATE vector alone governs that tail); and it
/// takes only SIX distinct values across an entire 2P run ({0, 250k, 500k,
/// 750k, 800k, 1M}; 9 at 3P, 11 at 4P), so at 2P — the promotion-cell player
/// count — the "continuous" interpolation is effectively a six-step staircase.
/// Both are observations under that harness, not ceilings.
///
/// T4'S PROXY VALIDATION (spec §2.4, artifact
/// `docs/plans/2026-07-27-phase3.3-weight-freeze.md`) extends those figures to a
/// corpus, under Greedy-v1 rather than a policy rung, and the extension moves
/// both of them — which is why the single-run figures above are labelled as
/// observations:
///
/// | | 2P (2,000 games, 163,486 turn starts) | 3P (200 games, 36,921) | 4P (200 games, 87,356) |
/// |---|---|---|---|
/// | distinct values | 16 | 18 | 12 |
/// | saturated share | 0.0311 | 0.0220 | 0.0129 |
/// | accuracy vs post-hoc terciles, fixed 1/3 cuts | 0.6773 | 0.6922 | 0.5574 |
/// | same, best IN-SAMPLE cuts (optimistic) | 0.6786 | 0.6985 | 0.7161 |
///
/// Base rate is 0.3373 / 0.3333 / 0.3341, so the proxy is well clear of chance
/// everywhere. THE DISPOSITION IT FEEDS: a HARD three-way stage switch would
/// mis-assign 32% of positions at 2P even with hindsight-fitted cuts (44% at
/// 4P, where the proxy runs ahead — 21,963 of 29,117 genuinely-MID turn starts
/// read as LATE). §2.3's smooth interpolation is therefore kept, and R1's
/// critic's "~35% mis-assignment" reproduces under the bot's own play instead of
/// under the research policy. Observations under a named harness, never
/// ceilings.
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
    fn the_feature_list_is_sixteen_features_in_a_canonical_order() {
        assert_eq!(
            Feature::COUNT,
            16,
            "spec §2.2's list is sixteen entries since T3's in-hand split",
        );
        for (i, feature) in Feature::ALL.into_iter().enumerate() {
            assert_eq!(feature.index(), i, "{} is not at its own index", feature.name());
        }
        // Names are the artifact/report key, so they must be unique.
        let mut names: Vec<&str> = Feature::ALL.iter().map(|f| f.name()).collect();
        names.sort_unstable();
        let unique = names.len();
        names.dedup();
        assert_eq!(names.len(), unique, "duplicate feature name");
        // The exact five whose values come from `ScoreStructure`.
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

    /// The signs are the part of [`WeightTable::V1`] that comes from
    /// measurement — research notes §1, RE-MEASURED and confirmed under the
    /// bot's own play at T4 — so they are pinned; the magnitudes are hand-set
    /// and deliberately NOT pinned here. The two magnitude RELATIONS that are
    /// load-bearing rather than taste — the assistant triple's
    /// `core > board > excess` — moved to
    /// `the_hand_split_orders_core_above_board_above_excess_in_both_vectors` at
    /// T3, because they are non-degeneracy conditions and deserve their own
    /// failure message.
    #[test]
    fn frozen_v1_weight_signs_follow_the_research_notes() {
        let w = WeightTable::V1;
        for feature in Feature::ALL {
            let (early, late) = w.get(feature);
            match feature {
                // R1-4 and R1-7: the two negative features, in BOTH vectors.
                Feature::AssistantsInWarehouse | Feature::HandSize => {
                    assert!(early < 0 && late < 0, "{} must be negative", feature.name());
                }
                // R1-5: the ONE feature whose measured sign flips across the
                // game — early church investment predicted losing under the
                // research corpus, while the printed 6/3 is real VP at the end.
                // This is the term §2.3's smooth interpolation exists for, and
                // §2.4 names it as one of the three figures T4 re-measures.
                Feature::ChurchBonusNow => {
                    assert!(early < 0, "church must be negative EARLY (R1-5)");
                    assert!(late > 0, "church must be positive LATE (printed 6/3 VP)");
                }
                _ => assert!(early > 0 && late > 0, "{} must be positive", feature.name()),
            }
        }

        // The score-structure terms are LIVE, not staged: a zeroed pair here
        // would silently switch one of the five re-derivations off (the T1
        // staging this replaced was exactly that shape, deliberately).
        for feature in Feature::ALL.into_iter().filter(|f| f.is_score_structure()) {
            let (early, late) = w.get(feature);
            assert!(
                (early, late) != (0, 0),
                "{} is a landed re-derivation and must carry a weight",
                feature.name()
            );
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
            .expect("sixteen features");
        assert!(vp_early < early_max, "early VP must NOT dominate (R1-1)");
    }

    /// **THE T3 NON-DEGENERACY PIN.** `core > board > excess`, in BOTH vectors.
    ///
    /// These are not two tidy facts about a hand-set table; they ARE the
    /// conditions that make Greedy-v1 playable, and the T3 finding is what
    /// happens when a table has only one of them. At one ply a placement's sign
    /// is `k · (w_board − w_hand)` and a recovery's is its negation, so with a
    /// SINGLE linear in-hand weight the two are the same inequality and exactly
    /// one of them can pay. The pre-split table priced in-hand 8 over on-board
    /// 7, which took the recovery side and made every placement a strict loss:
    /// 2P self-play placed 0 assistants and ran to the turn cap.
    ///
    /// The piecewise split buys both because the two moves spend different
    /// parts of the hand — `w_board > w_excess` makes placing from excess pay,
    /// `w_core > w_board` makes recovering into an empty hand pay. Break either
    /// inequality in either vector and one of the two moves goes back to never
    /// paying, which is why this is pinned rather than left to the sign test.
    /// The four one-ply consequences are asserted end-to-end on real successor
    /// views by `the_hand_split_makes_placement_and_recovery_both_payable_at_one_ply`.
    ///
    /// BOTH vectors, and that covers every stage: interpolation is monotone in
    /// `p`, so bands that do not overlap at the endpoints cannot cross between
    /// them (measured here: core `[20,30]`, board `[10,14]`, excess `[2,4]`).
    #[test]
    fn the_hand_split_orders_core_above_board_above_excess_in_both_vectors() {
        let w = WeightTable::V1;
        let (core_early, core_late) = w.get(Feature::AssistantsInHandCore);
        let (board_early, board_late) = w.get(Feature::AssistantsOnBoard);
        let (excess_early, excess_late) = w.get(Feature::AssistantsInHandExcess);

        assert!(
            core_early > board_early,
            "early: core ({core_early}) must outprice on-board ({board_early}), or recovering \
             into an empty hand never pays",
        );
        assert!(
            board_early > excess_early,
            "early: on-board ({board_early}) must outprice excess ({excess_early}), or placing \
             from a full hand never pays",
        );
        assert!(
            core_late > board_late,
            "late: core ({core_late}) must outprice on-board ({board_late})",
        );
        assert!(
            board_late > excess_late,
            "late: on-board ({board_late}) must outprice excess ({excess_late})",
        );

        // The bands the endpoints define do not overlap, so no interpolated
        // stage can reorder them. Checked at the crossing point too, since the
        // claim is about `interpolate`'s output and not about the literals.
        for p in [0, 1, 250_000, 500_000, 750_000, 999_999, PROGRESS_FULL_PPM] {
            let core = interpolate(core_early, core_late, p);
            let board = interpolate(board_early, board_late, p);
            let excess = interpolate(excess_early, excess_late, p);
            assert!(
                core > board && board > excess,
                "the ordering broke at p={p}: core {core}, board {board}, excess {excess}",
            );
        }
    }

    /// [`WeightTable::PRE_FREEZE_2026_07_27`] is a HISTORICAL RECORD: it is the
    /// table Greedy-v1 played while generating the T4 freeze corpus, and every
    /// command in `docs/plans/2026-07-27-phase3.3-weight-freeze.md` names it, so
    /// its literals may never move. They are written out here rather than
    /// compared to `V1`, because comparing to `V1` is exactly what would let
    /// them drift together.
    ///
    /// The second half asserts `V1 == PRE_FREEZE` today, which is the FREEZE'S
    /// OUTCOME and not an invariant: §2.4's single adjustment pass found no
    /// contradicted sign and therefore moved no weight. **If you are here
    /// because you deliberately re-priced `V1`, delete the second assertion and
    /// keep the first** — the historical arrays stay, the equality was only ever
    /// a statement about 2026-07-27.
    #[test]
    fn the_pre_freeze_table_is_the_corpus_generator_and_v1_matches_it_today() {
        let pre = WeightTable::PRE_FREEZE_2026_07_27;
        assert_eq!(
            pre.early,
            [10, 12, 6, -4, 3, 30, 4, -10, 14, 12, 15, 8, 5, 2, 3, -3],
            "the freeze corpus's EARLY vector is a historical fact and cannot be re-priced",
        );
        assert_eq!(
            pre.late,
            [40, 30, 20, 16, 20, 20, 2, -4, 10, 20, 25, 10, 8, 4, 4, -6],
            "the freeze corpus's LATE vector is a historical fact and cannot be re-priced",
        );
        assert_eq!(
            WeightTable::V1,
            pre,
            "V1 has moved off the pre-freeze table. That is allowed — see this test's doc — \
             but the freeze artifact's figures then describe PRE_FREEZE_2026_07_27 only, and \
             this assertion should be deleted in the same commit that moves V1.",
        );
    }

    #[test]
    fn weights_can_be_read_and_mutated_by_feature() {
        let mut w = WeightTable::V1;
        assert_eq!(w.get(Feature::VictoryPoints), (10, 40));
        w.set(Feature::AssistantsInWarehouse, 0, 0);
        assert_eq!(w.get(Feature::AssistantsInWarehouse), (0, 0), "the §0.2 mutation shape");
        assert_eq!(w.get(Feature::VictoryPoints), (10, 40), "and nothing else moved");
        assert_eq!(WeightTable::ZERO.get(Feature::VictoryPoints), (0, 0));
        assert_eq!(WeightTable::ZERO.early.iter().chain(WeightTable::ZERO.late.iter()).sum::<i64>(), 0);
    }
}
