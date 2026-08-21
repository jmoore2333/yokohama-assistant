//! **The V4 §5 PAIR-FIXTURE PACKAGE — infrastructure** (spec/pre-registration
//! `docs/plans/2026-08-20-phase3.5c-v4-spec-prereg.md` §5.1/§5.2, checkpoint §7
//! step 2).
//!
//! This module is the FRAME the §5.2 pairs land in, and nothing else: the
//! registered-tuple declaration type, the fixture carrier with its construction
//! invariants, the two stage-realization builders, the package digest, and the
//! print seam. **The registry ships EMPTY** — every one of §5.2's thirty-four
//! gate pairs and two diagnostics is a later task's registration on top of this
//! surface, and [`REGISTERED_PAIR_TARGET`] is what says so out loud.
//!
//! # Why it lives in the LIBRARY
//!
//! `examples/eval_probe` reaches only the crate's PUBLIC surface, and the §5
//! instrument prints from the probe side (spec §13: "Steps 2–7 in probe space").
//! A fixture package written in `tests/` would be unreachable from the example
//! that has to print it; one written in the example would be untested by every
//! per-edit gate this repo documents (`examples/eval_probe` runs under exactly
//! one command, and `--all-targets` clippy compiles its tests without running
//! them). The library is the only place both readers can stand.
//!
//! # The four things §5.1 asks a fixture package to be
//!
//! 1. **DECLARED, not inferred.** Every fact §5.1 says a pair registers — stage
//!    endpoint, view class, margin class, BOTH seats' president locations,
//!    achievement draws, technology exclusions, the expected-nonzero rows — is a
//!    field of [`PairDeclaration`], and every one of them that is mechanically
//!    checkable against a constructed state IS checked, at
//!    [`PairFixture::new`]. A declaration a fixture does not honour is a
//!    construction error, never a print-time footnote.
//! 2. **ENDPOINT-EXACT.** §5.1's stage rule is arithmetic over the engine's own
//!    four clocks, and [`stage_base_2p`] realizes it against
//!    [`crate::game::eval::clock_ppm`] (`game/eval.rs:1956`) rather than against
//!    a restatement of it. Both sides of a pair must read the SAME
//!    [`crate::game::eval::progress_ppm`] (`game/eval.rs:1933`), and it must
//!    equal the declared endpoint.
//! 3. **DIGESTED.** [`package_digest`] hashes the registered TUPLES — the
//!    declarations and the feature axis they name rows on — so that "every
//!    Part-A/B artifact prints the digest" (§5.1) is one function call. State
//!    LITERAL bytes are a separate, per-fixture [`state_fingerprint`], because
//!    the two answer different questions: the digest says which package this is,
//!    the fingerprint says which bytes a re-read looked at.
//! 4. **SEAMED FOR STEP 3.** The print harness renders all thirty-four axis rows
//!    today. The sixteen INCUMBENT rows carry real numbers off the existing
//!    `eval` surface; the eighteen CANDIDATE rows render
//!    [`CANDIDATE_PENDING`] — deliberately NOT zero, because a zero is a
//!    measurement and "not built yet" is not. [`CandidateColumns`] is where step
//!    3 joins; nothing else in this file moves when it does.
//!
//! # What this module deliberately does NOT do
//!
//! It implements no candidate FORMULA (step 3), registers no pair (later step-2
//! tasks), fits nothing, and reads no root. It touches neither `eval`'s
//! behaviour nor `SEMANTICS_REV` nor the oracle: everything here is additive,
//! and every number it prints is read through `eval`'s existing public surface.

use crate::game::eval::{self, Feature, ScoreStructure, PROGRESS_FULL_PPM};
use crate::game::{finalize_setup, new_game, Rules, StartingOrderChoice};
use crate::types::{
    AchievementCategory, AchievementType, AreaId, GameConfig, GameState, PlayerId, PlayerView,
    PresidentLocation, TechEffect, TurnPhase, VisiblePlayer,
};

// ---------------------------------------------------------------------------
// The 34-feature axis (spec §3.2)
// ---------------------------------------------------------------------------

/// The EIGHTEEN candidate features of §3.2, by their registered labels.
///
/// **Identifiers only.** This enum names the rows a §5.2 registration lists as
/// expected-nonzero and the columns the print harness reserves; it carries no
/// formula, and step 3 is where `eval.rs` grows the helpers behind them. Naming
/// them now is what lets a pair registered at step 2 declare a candidate row —
/// the whole point of registering expected-nonzero rows over the FULL axis
/// rather than over the incumbent half.
///
/// **Order is the spec's own width statement** (§3.2's closing line: "S1–S7 ·
/// E1–E3 · O1–O3, O5, O6 · X1–X3"), which is LABEL order — §3.2's prose happens
/// to present E3 before E2, and the label order is the one every carrier map,
/// gate list and margin table in §5 speaks. **O4 does not exist**: §3.2 records
/// the numbering gap deliberately (rev-3's un-gated O4 has no successor slot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateFeature {
    /// S1 — exported reward at realizable power, over own-occupied production areas.
    ProductionAccess,
    /// S2 — realizable power over own-occupied {Church, Customs}.
    TrackAccess,
    /// S3 — count of non-Canal areas holding ≥ 1 own assistant.
    AssistantAreasDistinct,
    /// S4 — the president's ring mass (on-board), or the best re-entry ring (in hand).
    PresidentLocalAccess,
    /// S5 — bank reward at realizable power, gated on own presence.
    BankAccess,
    /// S6 — takeable Employment-Agency tokens, gated by the warehouse min.
    EmploymentAgencyAccess,
    /// S7 — areas actionable in principle from the president's position.
    PresidentReachableAreas,
    /// E1 — `player.money`, linear.
    MoneyStock,
    /// E2 — `copper + silk + tea + seafood`, linear.
    TradeGoodsStock,
    /// E3 — building tokens retrieved to hand and deployment-ready.
    BuildingTokensReady,
    /// O1 — held order value in HALF-VP units, opponent slots at the pool floor.
    HeldOrderValue,
    /// O2 — goods held against known held orders' summed requirements.
    OrderMatchedGoods,
    /// O3 — count of known held orders with every requirement met.
    ExecutableOrders,
    /// O5 — the best attainable country-set marginal.
    FlagIconMarginal,
    /// O6 — icon faces on known held orders, in ICON units.
    HeldOrderIcons,
    /// X1 — lab slots takeable ALL-IN at the seat's own lab presence.
    LabTakeable,
    /// X2 — matched goods over port offers in the zones open at the seat's power.
    PortTakeable,
    /// X3 — production access conditioned on held orders' residual need.
    MatchedProductionAccess,
}

impl CandidateFeature {
    /// The canonical order — the spec's own width statement.
    pub const ALL: [CandidateFeature; 18] = [
        CandidateFeature::ProductionAccess,
        CandidateFeature::TrackAccess,
        CandidateFeature::AssistantAreasDistinct,
        CandidateFeature::PresidentLocalAccess,
        CandidateFeature::BankAccess,
        CandidateFeature::EmploymentAgencyAccess,
        CandidateFeature::PresidentReachableAreas,
        CandidateFeature::MoneyStock,
        CandidateFeature::TradeGoodsStock,
        CandidateFeature::BuildingTokensReady,
        CandidateFeature::HeldOrderValue,
        CandidateFeature::OrderMatchedGoods,
        CandidateFeature::ExecutableOrders,
        CandidateFeature::FlagIconMarginal,
        CandidateFeature::HeldOrderIcons,
        CandidateFeature::LabTakeable,
        CandidateFeature::PortTakeable,
        CandidateFeature::MatchedProductionAccess,
    ];

    /// Length of [`CandidateFeature::ALL`].
    pub const COUNT: usize = CandidateFeature::ALL.len();

    /// The §5 registration LABEL (`"S1"`, `"X3"`, …) — the string every §5.2
    /// carrier line, cluster note and gate map is written in.
    pub fn label(self) -> &'static str {
        match self {
            CandidateFeature::ProductionAccess => "S1",
            CandidateFeature::TrackAccess => "S2",
            CandidateFeature::AssistantAreasDistinct => "S3",
            CandidateFeature::PresidentLocalAccess => "S4",
            CandidateFeature::BankAccess => "S5",
            CandidateFeature::EmploymentAgencyAccess => "S6",
            CandidateFeature::PresidentReachableAreas => "S7",
            CandidateFeature::MoneyStock => "E1",
            CandidateFeature::TradeGoodsStock => "E2",
            CandidateFeature::BuildingTokensReady => "E3",
            CandidateFeature::HeldOrderValue => "O1",
            CandidateFeature::OrderMatchedGoods => "O2",
            CandidateFeature::ExecutableOrders => "O3",
            CandidateFeature::FlagIconMarginal => "O5",
            CandidateFeature::HeldOrderIcons => "O6",
            CandidateFeature::LabTakeable => "X1",
            CandidateFeature::PortTakeable => "X2",
            CandidateFeature::MatchedProductionAccess => "X3",
        }
    }

    /// The stable snake_case name, on [`Feature::name`]'s discipline: report
    /// columns and digests come from here so none of them is hand-typed.
    pub fn name(self) -> &'static str {
        match self {
            CandidateFeature::ProductionAccess => "production_access",
            CandidateFeature::TrackAccess => "track_access",
            CandidateFeature::AssistantAreasDistinct => "assistant_areas_distinct",
            CandidateFeature::PresidentLocalAccess => "president_local_access",
            CandidateFeature::BankAccess => "bank_access",
            CandidateFeature::EmploymentAgencyAccess => "employment_agency_access",
            CandidateFeature::PresidentReachableAreas => "president_reachable_areas",
            CandidateFeature::MoneyStock => "money_stock",
            CandidateFeature::TradeGoodsStock => "trade_goods_stock",
            CandidateFeature::BuildingTokensReady => "building_tokens_ready",
            CandidateFeature::HeldOrderValue => "held_order_value",
            CandidateFeature::OrderMatchedGoods => "order_matched_goods",
            CandidateFeature::ExecutableOrders => "executable_orders",
            CandidateFeature::FlagIconMarginal => "flag_icon_marginal",
            CandidateFeature::HeldOrderIcons => "held_order_icons",
            CandidateFeature::LabTakeable => "lab_takeable",
            CandidateFeature::PortTakeable => "port_takeable",
            CandidateFeature::MatchedProductionAccess => "matched_production_access",
        }
    }

    /// Position in [`CandidateFeature::ALL`], as an exhaustive match rather
    /// than a discriminant cast — [`Feature::index`]'s reasoning verbatim: a
    /// reordering of the enum that did not reorder `ALL` would silently
    /// mis-index, and the round-trip pin catches it here instead.
    pub fn index(self) -> usize {
        match self {
            CandidateFeature::ProductionAccess => 0,
            CandidateFeature::TrackAccess => 1,
            CandidateFeature::AssistantAreasDistinct => 2,
            CandidateFeature::PresidentLocalAccess => 3,
            CandidateFeature::BankAccess => 4,
            CandidateFeature::EmploymentAgencyAccess => 5,
            CandidateFeature::PresidentReachableAreas => 6,
            CandidateFeature::MoneyStock => 7,
            CandidateFeature::TradeGoodsStock => 8,
            CandidateFeature::BuildingTokensReady => 9,
            CandidateFeature::HeldOrderValue => 10,
            CandidateFeature::OrderMatchedGoods => 11,
            CandidateFeature::ExecutableOrders => 12,
            CandidateFeature::FlagIconMarginal => 13,
            CandidateFeature::HeldOrderIcons => 14,
            CandidateFeature::LabTakeable => 15,
            CandidateFeature::PortTakeable => 16,
            CandidateFeature::MatchedProductionAccess => 17,
        }
    }
}

/// One row of the §5 THIRTY-FOUR-FEATURE AXIS: an incumbent or a candidate.
///
/// The incumbent arm carries [`Feature`] itself rather than a copy of its
/// sixteen names. That is the whole reason for the two-arm shape: `eval`'s
/// enum is the authority on the shipped sixteen, and a second spelling of them
/// here would be a second thing to keep in step. When `Feature::ALL` widens to
/// thirty-four at step 8 (§3.2's closing line), THIS type is what stops being
/// needed — and until then it is what lets a step-2 registration name a row
/// that does not exist in `eval` yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureId {
    /// One of the sixteen shipped features, by `eval`'s own identifier.
    Incumbent(Feature),
    /// One of the eighteen §3.2 candidates, by label.
    Candidate(CandidateFeature),
}

/// Width of the §5 signature axis: `Feature::COUNT` + `CandidateFeature::COUNT`
/// = 16 + 18. §3.2's "Width: 16 → 34".
pub const AXIS_WIDTH: usize = Feature::COUNT + CandidateFeature::COUNT;

impl FeatureId {
    /// The axis in canonical order: the sixteen incumbents in `Feature::ALL`
    /// order, then the eighteen candidates in [`CandidateFeature::ALL`] order.
    ///
    /// Incumbents FIRST so that the delta vector's first sixteen entries stay
    /// index-compatible with `eval`'s weight vectors while both widths are in
    /// play — the step-8 `Feature::ALL` move then appends rather than
    /// re-numbering.
    pub fn all() -> [FeatureId; AXIS_WIDTH] {
        let mut axis = [FeatureId::Incumbent(Feature::VictoryPoints); AXIS_WIDTH];
        for f in Feature::ALL {
            axis[f.index()] = FeatureId::Incumbent(f);
        }
        for c in CandidateFeature::ALL {
            axis[Feature::COUNT + c.index()] = FeatureId::Candidate(c);
        }
        axis
    }

    /// Position on the axis. `FeatureId::all()[id.index()] == id` for every row.
    pub fn index(self) -> usize {
        match self {
            FeatureId::Incumbent(f) => f.index(),
            FeatureId::Candidate(c) => Feature::COUNT + c.index(),
        }
    }

    /// The stable snake_case name — `eval`'s for an incumbent, §3.2's for a
    /// candidate.
    pub fn name(self) -> &'static str {
        match self {
            FeatureId::Incumbent(f) => f.name(),
            FeatureId::Candidate(c) => c.name(),
        }
    }

    /// The registration LABEL: `"S1"`-style for a candidate, and the snake_case
    /// name for an incumbent (the incumbents have no §5 label — §5.2 names them
    /// by `eval` name, e.g. 4(d)'s carrier `CountryBonusNow`).
    pub fn label(self) -> &'static str {
        match self {
            FeatureId::Incumbent(f) => f.name(),
            FeatureId::Candidate(c) => c.label(),
        }
    }

    /// True for the eighteen rows whose formulas land at step 3 — the rows the
    /// print harness renders as [`CANDIDATE_PENDING`] until then.
    pub fn is_candidate(self) -> bool {
        matches!(self, FeatureId::Candidate(_))
    }
}

// ---------------------------------------------------------------------------
// The registered tuple (spec §5.1)
// ---------------------------------------------------------------------------

/// §5.1's STAGE ENDPOINT: the two points at which `interpolate` is exact, so
/// that every registered row is linear in ONE stage vector.
///
/// There is no MID: §5.1 registers `p = 0` or `p = PROGRESS_FULL_PPM` and
/// nothing between, precisely because `eval::interpolate` returns `early` at 0
/// and `late` at full (`game/eval.rs:1887`) and any interior point mixes the
/// two vectors into one inequality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageEndpoint {
    /// `p = 0` — the EARLY weight vector, exactly.
    Early,
    /// `p = PROGRESS_FULL_PPM` — the LATE weight vector, exactly.
    Late,
}

impl StageEndpoint {
    /// Both endpoints, in registration order.
    pub const ALL: [StageEndpoint; 2] = [StageEndpoint::Early, StageEndpoint::Late];

    /// The exact `progress_ppm` this endpoint demands of BOTH sides.
    pub fn ppm(self) -> u32 {
        match self {
            StageEndpoint::Early => 0,
            StageEndpoint::Late => PROGRESS_FULL_PPM,
        }
    }

    /// The digest / report spelling.
    pub fn name(self) -> &'static str {
        match self {
            StageEndpoint::Early => "early",
            StageEndpoint::Late => "late",
        }
    }
}

/// §5.1's VIEW CLASS — "STAGE and VIEW CLASS are independent declarations, both
/// required".
///
/// HAND-OFF is the default and is a STRUCTURAL claim about the state, not a
/// label: a non-terminal player hand-off rests at
/// [`TurnPhase::AdditionalActionFirst`] with an empty choice queue and no
/// action power (`game/apply_move.rs:276–288` is the only site that advances
/// the seat, and it clears both). A MID-TURN pair names the phase it sits at
/// and whether the mercenary sub-turn is running, because those two fields are
/// what make a mid-turn state a different evaluation subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewClass {
    /// The post-Recovery player hand-off — §5.1's default.
    HandOff,
    /// A state inside a turn, at the named phase.
    MidTurn {
        /// The declared `turn_phase`, asserted on both sides.
        phase: TurnPhase,
        /// Whether the nested mercenary sub-turn is running.
        mercenary: bool,
    },
}

impl ViewClass {
    /// The digest / report spelling.
    fn canonical(self) -> String {
        match self {
            ViewClass::HandOff => "hand-off".to_string(),
            ViewClass::MidTurn { phase, mercenary } => {
                format!("mid-turn:{phase:?}:mercenary={mercenary}")
            }
        }
    }
}

/// §5.1's MARGIN CLASS — what the solve's inequality for this row demands.
///
/// **The early floor is a STRENGTHENING, not a loosening** (§5.1): a registered
/// MARGINED integer is `max(1, ceil(margin_vp × w_vp(stage)))`, so an early
/// amortized ⅓-VP carrier registers 1 and is therefore identical in force to
/// DIRECTION-ONLY there. That identity is why §5.4 keeps an early-stage rescale
/// diagnostic — an early-only infeasibility may be integer granularity rather
/// than a real contradiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarginClass {
    /// A registered integer margin at each declared endpoint, with a NAMED
    /// carrier. `None` at an endpoint the pair does not register.
    Margined {
        /// The `p = 0` margin, when the pair registers the early endpoint.
        early: Option<i64>,
        /// The `p = PROGRESS_FULL_PPM` margin.
        late: Option<i64>,
        /// The feature the margin is priced through — which may be an
        /// INCUMBENT (4(d) prices through `CountryBonusNow` and has no
        /// production carrier at all).
        carrier: FeatureId,
    },
    /// Margin 1: the inequality is strict, and nothing more is claimed.
    DirectionOnly,
    /// Margin 0: `greater ≥ lesser`. Family 7's anti-parking row.
    Weak,
}

impl MarginClass {
    /// The registered margin at one endpoint, or `None` where a MARGINED pair
    /// registers no integer for it.
    ///
    /// DIRECTION-ONLY and WEAK are stage-independent by construction — 1 and 0
    /// are not prices, they are the two shapes of "no price was registered".
    pub fn margin_at(self, stage: StageEndpoint) -> Option<i64> {
        match self {
            MarginClass::Margined { early, late, .. } => match stage {
                StageEndpoint::Early => early,
                StageEndpoint::Late => late,
            },
            MarginClass::DirectionOnly => Some(1),
            MarginClass::Weak => Some(0),
        }
    }

    /// The digest / report spelling.
    fn canonical(self) -> String {
        match self {
            MarginClass::Margined { early, late, carrier } => {
                let e = early.map_or_else(|| "-".to_string(), |v| v.to_string());
                let l = late.map_or_else(|| "-".to_string(), |v| v.to_string());
                format!("margined:early={e}:late={l}:carrier={}", carrier.label())
            }
            MarginClass::DirectionOnly => "direction-only".to_string(),
            MarginClass::Weak => "weak".to_string(),
        }
    }
}

/// §5.1's PRESIDENT DECLARATION for one seat — "BOTH seats' president locations
/// declared — DEFAULT: both InHand unless the registration states otherwise".
///
/// The two on-board arms are NOT a convenience split. §5.1's rule is that a
/// hand-off-class fixture may not stand a seat's own president on an area where
/// that seat holds assistants, because Recovery drains the acted area — the
/// shape is unreachable, "Telephone excepted". Making the exception a variant
/// with its own flag is what turns that sentence into a construction check
/// instead of a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresidentDecl {
    /// §5.1's DEFAULT.
    InHand,
    /// On the board, at an area where this seat holds NO assistants — the
    /// post-Recovery hand-off shape (1(c)/1(e)'s construction).
    OnBoardEmptyArea,
    /// On the board, at an area where this seat DOES hold assistants.
    OnBoardWithOwnAssistants {
        /// §5.1's named exception. A HAND-OFF fixture may only take this shape
        /// when the seat owns [`TechEffect::Telephone`] (Recovery may then leave
        /// one assistant in the action area) — asserted, not assumed. MID-TURN
        /// fixtures reach it without the tech, because Recovery has not run.
        telephone_excepted: bool,
    },
}

impl PresidentDecl {
    /// The digest / report spelling.
    fn canonical(self) -> &'static str {
        match self {
            PresidentDecl::InHand => "in-hand",
            PresidentDecl::OnBoardEmptyArea => "on-board-empty",
            PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: true } => {
                "on-board-occupied-telephone"
            }
            PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: false } => {
                "on-board-occupied"
            }
        }
    }
}

/// Both seats' president declarations. §5.1 requires both; the seat naming is
/// relative to the pair's SUBJECT (the mover whose view §5.3 signs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeatPresidents {
    /// The subject seat — the mover.
    pub subject: PresidentDecl,
    /// The other 2P seat. §5.1: "an opponent-president area in a fixture is
    /// declared, since it blocks S7-counting and taxes placement".
    pub opponent: PresidentDecl,
}

impl SeatPresidents {
    /// §5.1's default: both InHand.
    pub const BOTH_IN_HAND: SeatPresidents =
        SeatPresidents { subject: PresidentDecl::InHand, opponent: PresidentDecl::InHand };
}

/// §5.1's ACHIEVEMENT DRAWS — "setup always draws exactly one Category-A card
/// ('no goods achievement' is not a legal state)", plus the REGISTERED DRAW
/// CONSTRAINT binding every pair whose recipe moves goods or yen.
///
/// The constraint is stored as the DATA it constrains rather than as a boolean:
/// `goods_moved` names the Category-A cards whose good this pair's recipe
/// touches, so "the drawn A-card's good is outside the moved set" is a set
/// membership the constructor can check, and `moves_yen` is what makes "the
/// drawn B card is not Money10" checkable in the same breath. A pair whose
/// recipe moves neither leaves both at their empty defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AchievementDraws {
    /// `achievements[0]` — always Category A (`game/setup.rs:113–117`).
    pub category_a: AchievementType,
    /// `achievements[1]` — always Category B.
    pub category_b: AchievementType,
    /// `achievements[2]` — always Category C.
    pub category_c: AchievementType,
    /// The Category-A cards whose GOOD this pair's recipe moves. The drawn
    /// `category_a` may not be among them (§5.1's registered draw constraint).
    pub goods_moved: &'static [AchievementType],
    /// True when the recipe moves YEN — then `category_b` may not be
    /// [`AchievementType::Money10`] (§2's veto identity: the leg would shadow
    /// E1).
    pub moves_yen: bool,
}

/// §5.1's TECHNOLOGY DECLARATION.
///
/// Exclusions are by EFFECT, not by printed name, because effect equality is
/// the engine's own ownership test (§3.2's O5/X1 both say so) and ten of the
/// twenty titles circulate as two physical copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechDeclaration {
    /// Effects NO seat may own — 3(a)/3(b) exclude System of Stock,
    /// Spinning Mill and University, "each [of which] changes the marginal the
    /// pair calibrates".
    pub excluded_effects: &'static [TechEffect],
    /// An upper bound on either seat's owned-technology count where the
    /// registration states one — 11(d)'s "≤ 1 tech pre-purchase so
    /// `technologies.len()` never crosses 2 → 3 and the 4th-card duty stays off
    /// X1's future terms".
    pub max_owned: Option<usize>,
}

impl TechDeclaration {
    /// No exclusion, no bound.
    pub const UNCONSTRAINED: TechDeclaration =
        TechDeclaration { excluded_effects: &[], max_owned: None };
}

/// What a registered pair DOES in §5.1's pair→gate mapping.
///
/// Only [`PairRole::Constraint`] rows enter Part B's constraint system and Part
/// A's blocking set; the other three are registered so the package is complete
/// and so nothing quietly promotes itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairRole {
    /// Families 1–4, 6–8, 10(a)/(b)/(d) and 11: a solve row and a blocking side.
    Constraint,
    /// 2(c) and 10(c): RECORDED, never a gate.
    Diagnostic,
    /// Family 5: an audit property, not a solve row.
    Audit,
    /// Family 9: steps 5/9 positive controls.
    Control,
}

impl PairRole {
    /// The digest / report spelling.
    fn canonical(self) -> &'static str {
        match self {
            PairRole::Constraint => "constraint",
            PairRole::Diagnostic => "diagnostic",
            PairRole::Audit => "audit",
            PairRole::Control => "control",
        }
    }
}

/// Whether a pair sits on a generated board or on one of §5.1's HAND-BUILT
/// layouts.
///
/// §5.1 lists "the hand-built layouts of 1(c)/1(e)/11" among the parameters it
/// leaves open to step 2, so a registration has to say which it is: a
/// hand-built layout is a fixture input the digest must cover, and a generated
/// one is fully determined by the setup seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutClass {
    /// `Rules::standard()` on a seeded `new_game` board.
    StandardGenerated,
    /// A hand-constructed layout the registration names.
    HandBuilt,
}

impl LayoutClass {
    /// The digest / report spelling.
    fn canonical(self) -> &'static str {
        match self {
            LayoutClass::StandardGenerated => "standard-generated",
            LayoutClass::HandBuilt => "hand-built",
        }
    }
}

/// **ONE REGISTERED TUPLE** — everything §5.1 says a §5.2 pair declares.
///
/// Every field is `'static` so a registration is a `const`: the registry is
/// then a compile-time literal, the digest is a pure function of the source,
/// and "the realized exact-tuple registry IS the completed pre-registration"
/// (§5.1's strengthened step-2 gate) is a property of the crate rather than of
/// a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairDeclaration {
    /// The §5.2 registration id, e.g. `"1(a)"`. Unique across the registry.
    pub id: &'static str,
    /// The registration NAME, greater side first, e.g.
    /// `"PRODUCTIVE ≻ POWER-DEAD"`.
    pub name: &'static str,
    /// §5.2 family number, 1..=11.
    pub family: u8,
    /// The GREATER side's own name — "named FIRST in every registration;
    /// `Δv = greater − lesser`".
    pub greater_side: &'static str,
    /// The LESSER side's own name.
    pub lesser_side: &'static str,
    /// §5.1's pair→gate mapping.
    pub role: PairRole,
    /// The declared stage endpoints. A both-endpoints pair lists both and
    /// contributes one ROW to each system.
    pub stages: &'static [StageEndpoint],
    /// HAND-OFF unless the registration states otherwise.
    pub view_class: ViewClass,
    /// MARGINED / DIRECTION-ONLY / WEAK.
    pub margin: MarginClass,
    /// The §3.2 feature this pair is registered to constrain — §5.1's carrier
    /// map. `None` for the rows that carry no production feature at all (4(d)
    /// is "a REALIZED-set compatibility constraint on the INCUMBENT — no
    /// production carrier").
    pub carrier: Option<FeatureId>,
    /// BOTH seats' president locations.
    pub presidents: SeatPresidents,
    /// The three achievement cards in play, and the registered draw constraint.
    pub draws: AchievementDraws,
    /// Technology ownership declarations.
    pub techs: TechDeclaration,
    /// Generated or hand-built board.
    pub layout: LayoutClass,
    /// Registered scalar parameters a construction takes — 2(b)'s `n = 3` is
    /// "REGISTERED, not open" and this is where it lives.
    pub parameters: &'static [(&'static str, i64)],
    /// §5.1's EXPECTED-NONZERO ROWS over the full 34-feature axis. "An unlisted
    /// nonzero row at run time is a fixture defect."
    pub expected_nonzero: &'static [FeatureId],
    /// §5.1's `p = 0` parenthetical — "no port take ever — hands capped at the
    /// dealt starting order" — is a REACHABILITY clause: with a full deck no
    /// legal take has happened, so a seat can hold at most the card it was
    /// dealt. A pair that must exceed it registers WHY here, and the reason
    /// enters the digest. `None` means the cap holds and is enforced.
    ///
    /// **This field is an INTERPRETATION** and is flagged as one: §5.1 states
    /// the clause but registers no deviation mechanism, and family 4(c)'s
    /// "requirements EQUAL AS THE HAND-SUMMED VECTOR" at its EARLY endpoint is
    /// the known place the clause and the recipe pull against each other (the
    /// manifest's only requirement twins are orders 10/32, whose movement icons
    /// differ, so a one-card early hand cannot equalize requirements, differ on
    /// faces, and hold icons equal at once). Registering the deviation makes
    /// the tension visible to the bounded re-read rather than silent.
    pub early_hand_cap_deviation: Option<&'static str>,
    /// §5.2's registered collateral prose, verbatim, one entry per clause.
    pub notes: &'static [&'static str],
}

// ---------------------------------------------------------------------------
// The registry (spec §5.1's registered counts)
// ---------------------------------------------------------------------------

/// **THE REGISTERED PAIRS.** Empty at S2-T3: this task builds the frame, and
/// each §5.2 family lands as its own later registration.
///
/// The array's LENGTH is its own pin — [`REGISTERED_PAIR_COUNT`] — on
/// `tests/scenario_suite.rs`'s discipline: a registry whose size is a type is a
/// registry a deletion cannot quietly shrink. A family task bumps the constant
/// and the array together, and the count test's message says what the target
/// is.
pub const REGISTERED: [PairDeclaration; REGISTERED_PAIR_COUNT] = [];

/// Length of [`REGISTERED`], bumped by each family registration task.
pub const REGISTERED_PAIR_COUNT: usize = 0;

/// §5.1's registered counts, summed: family 1: 5 · 2: 3 · 3: 4 · 4: 7 · 6: 5 ·
/// 7: 1 · 8: 1 · 10-gates: 3 · 11: 5 = **34 gate pairs**, plus the two recorded
/// diagnostics 2(c) and 10(c). Families 5 and 9 register audit properties and
/// controls rather than pairs.
///
/// This is the number [`REGISTERED_PAIR_COUNT`] is walking toward; it is stated
/// here so the count test can say how far the package has to go rather than
/// only that it moved.
pub const REGISTERED_PAIR_TARGET: usize = 36;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why a fixture could not be admitted, or a stage base could not be built.
///
/// Every variant names the CLAUSE it enforces rather than the symptom, because
/// these refusals are read by a fixture author repairing a construction: "the
/// declared stage endpoint" is actionable and "assertion failed" is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureError {
    /// The S2-T3 stubs, before the green pass. Never constructed after it.
    Unimplemented(&'static str),
    /// `new_game` / `finalize_setup` refused.
    Setup(&'static str),
    /// The state is not one `check_invariants` accepts.
    NotEngineValid {
        /// `greater` or `lesser`.
        side: &'static str,
        /// The engine's own message.
        detail: String,
    },
    /// A declared clause the constructed states do not honour.
    Declaration {
        /// The §5.1 clause name.
        clause: &'static str,
        /// What was declared and what was found.
        detail: String,
    },
    /// `observe` refused a side for the subject seat.
    NotObservable {
        /// `greater` or `lesser`.
        side: &'static str,
        /// The view error, rendered.
        detail: String,
    },
    /// A pile the fixture wanted to draw from is empty or the card is missing.
    OutOfStock(&'static str),
}

impl std::fmt::Display for FixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixtureError::Unimplemented(what) => write!(f, "not implemented yet: {what}"),
            FixtureError::Setup(e) => write!(f, "setup refused: {e}"),
            FixtureError::NotEngineValid { side, detail } => {
                write!(f, "the {side} side is not a state the engine would accept: {detail}")
            }
            FixtureError::Declaration { clause, detail } => {
                write!(f, "declared clause '{clause}' is not honoured: {detail}")
            }
            FixtureError::NotObservable { side, detail } => {
                write!(f, "the {side} side is not observable from the subject seat: {detail}")
            }
            FixtureError::OutOfStock(what) => write!(f, "out of stock: {what}"),
        }
    }
}

// ---------------------------------------------------------------------------
// The fixture carrier
// ---------------------------------------------------------------------------

/// Which side of a pair a value belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The declared GREATER side — named first in every registration.
    Greater,
    /// The declared LESSER side.
    Lesser,
}

impl Side {
    /// The digest / report spelling, and the `side` field of a refusal.
    pub fn name(self) -> &'static str {
        match self {
            Side::Greater => "greater",
            Side::Lesser => "lesser",
        }
    }
}

/// **ONE REALIZED PAIR at one stage endpoint**: the two constructed states,
/// their views from the subject seat, and the declaration they answer to.
///
/// A `PairFixture` cannot exist unless every mechanically checkable clause of
/// its declaration holds — see [`PairFixture::new`]. That is the type's whole
/// job: downstream (the collision instrument, the solver, the re-read) reads
/// fixtures, and a fixture that lied about its stage or its president would
/// make every number computed from it a statement about a different position.
#[derive(Debug, Clone)]
pub struct PairFixture {
    declaration: &'static PairDeclaration,
    stage: StageEndpoint,
    subject: PlayerId,
    greater: GameState,
    lesser: GameState,
    greater_view: PlayerView,
    lesser_view: PlayerView,
}

impl PairFixture {
    /// Admit a pair at one declared endpoint, enforcing §5.1's clauses in the
    /// order they can be trusted.
    ///
    /// **The order is load-bearing.** Engine validity comes before anything
    /// reads the state; terminality comes before the view class, so a terminal
    /// fixture is refused as terminal rather than as a phase mismatch; the
    /// expected-nonzero check comes last, because it is the only clause that
    /// needs every other one to already hold to mean anything.
    ///
    /// **The clauses, in order.**
    ///
    /// 1. `stage` is one the declaration registers.
    /// 2. Both sides are 2-player. §5.1's stage realization is written "at 2P",
    ///    and §5.3's signature is 2P-only because `opponent_extreme` orients by
    ///    weight above two seats.
    /// 3. Both sides pass `check_invariants` — the constructibility mandate.
    /// 4. `subject` is a real seat, and is the MOVER on both sides (§5.3 signs
    ///    the mover's view; a pair signed from the idle seat would be a
    ///    different measurement wearing the same name).
    /// 5. Both sides `observe` from `subject`.
    /// 6. Neither side is terminal (`terminal_standing` is `None`).
    /// 7. `progress_ppm` is EQUAL across sides and equals `stage.ppm()`.
    /// 8. The view class holds on both sides.
    /// 9. Both seats' president declarations hold on both sides, including
    ///    §5.1's unreachable-post-Recovery rule and its Telephone exception.
    /// 10. The achievement draws match, the categories are the ones setup
    ///     deals, and the registered draw constraint holds.
    /// 11. The technology declaration holds.
    /// 12. The stage's own §5.1 construction clauses hold on the FINAL states,
    ///     not merely on the base they were built from.
    /// 13. Every INCUMBENT row whose delta is nonzero is listed in
    ///     `expected_nonzero`.
    pub fn new(
        declaration: &'static PairDeclaration,
        stage: StageEndpoint,
        greater: GameState,
        lesser: GameState,
        subject: PlayerId,
        rules: &Rules,
    ) -> Result<Self, FixtureError> {
        let _ = (declaration, stage, &greater, &lesser, subject, rules);
        Err(FixtureError::Unimplemented("PairFixture::new"))
    }

    /// The declaration this fixture answers to.
    pub fn declaration(&self) -> &'static PairDeclaration {
        self.declaration
    }

    /// The endpoint this realization sits at.
    pub fn stage(&self) -> StageEndpoint {
        self.stage
    }

    /// The MOVER seat whose view §5.3 signs.
    pub fn subject(&self) -> PlayerId {
        self.subject
    }

    /// One side's constructed state.
    pub fn state(&self, side: Side) -> &GameState {
        match side {
            Side::Greater => &self.greater,
            Side::Lesser => &self.lesser,
        }
    }

    /// One side's view from the subject seat.
    pub fn view(&self, side: Side) -> &PlayerView {
        match side {
            Side::Greater => &self.greater_view,
            Side::Lesser => &self.lesser_view,
        }
    }

    /// The INCUMBENT half of the §5.3 signature: `Δ(own − opponent)` across the
    /// two sides, for each of `eval`'s sixteen features, indexed by
    /// [`Feature::index`].
    ///
    /// **The RELATIVE form, differenced** — not the subject's raw values. The
    /// evaluator's contribution to a position is `w · (mine − theirs)`
    /// (`game/eval.rs:1006`), so the quantity a solve row constrains is the
    /// difference of that across the pair. §5.2's 4(c) writes the identity out:
    /// "the DIFFERENCED ΔO1 = Δfaces + Δfloor_own − Δfloor_opp". At 2P the
    /// "theirs" term is simply the other seat's value, which is what
    /// `opponent_extreme` reduces to when there is exactly one opponent —
    /// pinned in this module's tests through `explain`'s reported
    /// `opponent_value`, so the reduction is checked rather than assumed.
    pub fn incumbent_delta(&self, rules: &Rules) -> [i64; Feature::COUNT] {
        let g = relative_vector(&self.greater_view, self.subject, rules);
        let l = relative_vector(&self.lesser_view, self.subject, rules);
        let mut delta = [0i64; Feature::COUNT];
        for f in Feature::ALL {
            delta[f.index()] = g[f.index()] - l[f.index()];
        }
        delta
    }

    /// The exact `progress_ppm` both sides read — printed by every report so an
    /// endpoint claim is checkable rather than asserted.
    pub fn progress_ppm(&self, rules: &Rules) -> u32 {
        eval::progress_ppm(&self.greater_view, rules)
    }

    /// The other half of §5.1's expected-nonzero rule — "a LISTED row must be
    /// nonzero" — over the INCUMBENT rows only.
    ///
    /// [`PairFixture::new`] enforces the other direction (a realized nonzero
    /// incumbent row must be listed), because that one is complete today: it
    /// needs no candidate column. This direction is NOT complete until step 3,
    /// since a pair whose only expected-nonzero rows are candidates has nothing
    /// to check yet — so it is a separate call a caller makes deliberately,
    /// rather than a half-check hiding inside the constructor.
    pub fn listed_incumbent_rows_are_nonzero(&self, rules: &Rules) -> Result<(), FixtureError> {
        let delta = self.incumbent_delta(rules);
        for row in self.declaration.expected_nonzero {
            if let FeatureId::Incumbent(f) = row {
                if delta[f.index()] == 0 {
                    return Err(FixtureError::Declaration {
                        clause: "expected-nonzero rows",
                        detail: format!(
                            "{} is listed as expected-nonzero but its differenced delta is 0",
                            f.name()
                        ),
                    });
                }
            }
        }
        Ok(())
    }
}

/// One view's RELATIVE feature vector from `subject`'s seat: `own − opponent`
/// for each incumbent feature, at 2P.
///
/// One [`ScoreStructure::derive`] per view, not one per (feature, seat) pair —
/// `eval::raw_value`'s own doc says why, and four of the five score-structure
/// features are rankings over the whole table.
fn relative_vector(view: &PlayerView, subject: PlayerId, rules: &Rules) -> [i64; Feature::COUNT] {
    let structure = ScoreStructure::derive(view, rules);
    let mut out = [0i64; Feature::COUNT];
    let Some(me) = view.players.iter().find(|p| p.id == subject) else {
        return out;
    };
    let opponent = view.players.iter().find(|p| p.id != subject && !p.id.is_dummy());
    for f in Feature::ALL {
        let mine = structure.value_of(me, f);
        let theirs = opponent.map_or(0, |o| structure.value_of(o, f));
        out[f.index()] = mine - theirs;
    }
    out
}

// ---------------------------------------------------------------------------
// Stage realization (spec §5.1 lines 422–427)
// ---------------------------------------------------------------------------

/// A 2P base state at the declared endpoint, ready for a pair's own doctoring.
///
/// See [`early_base_2p`] and [`late_base_2p`] for what each endpoint's
/// construction is and what it costs.
pub fn stage_base_2p(
    stage: StageEndpoint,
    seed: u64,
    rules: &Rules,
) -> Result<GameState, FixtureError> {
    match stage {
        StageEndpoint::Early => early_base_2p(seed, rules),
        StageEndpoint::Late => late_base_2p(seed, rules),
    }
}

/// **§5.1's `p = 0` REALIZATION**: a FULL deck, ZERO real pawns on both tracks,
/// ZERO buildings — all four of `eval::clock_ppm`'s clocks at zero
/// (`game/eval.rs:1956`).
///
/// A finalized 2P setup already IS that state, and saying so rather than
/// doctoring toward it is the point: the deck holds its derived initial size
/// (`eval::initial_order_deck_len`, 10 at 2P), `total_assistants()` excludes the
/// D1 dummy so both tracks read empty, and nothing is built. The builder's job
/// is therefore to ASSERT the arithmetic against the engine's own derivation, so
/// that a change to any clock surfaces here instead of inside a pair.
pub fn early_base_2p(seed: u64, rules: &Rules) -> Result<GameState, FixtureError> {
    let _ = (seed, rules);
    Err(FixtureError::Unimplemented("early_base_2p"))
}

/// **§5.1's `p = PROGRESS_FULL_PPM` REALIZATION**, by DECK SATURATION: the deck
/// is emptied while the port stays full, so the deck clock alone reaches full
/// and "track/building deltas inside a pair are absorbed by the max".
///
/// **Where the drained cards go, and the caveat that carries.** They are parked
/// in `orders_removed` — the box — because it is the only sink that moves no
/// seat's holdings and no port slot, so the saturation moves exactly one clock
/// and a pair's own arithmetic is unchanged by the base. The box is NOT how a
/// real game empties its deck (play drains it through port refills into hands
/// and completed piles), so this base is a REACHABILITY compromise, stated
/// rather than hidden: §5.1 leaves "the construction each late pair names" to
/// the pair, and a late pair whose recipe needs the reachable distribution
/// writes its own sink on top of this one. Order conservation is exact either
/// way (`game/invariants.rs:89–107` totals all five piles to 36), and the view's
/// reconciliation identity holds because `orders_removed_len` is derived from
/// the same piles.
///
/// The end trigger does NOT fire: the real deck trigger also requires an empty
/// port slot, and this construction keeps the port full — which is exactly why
/// §5.1 phrases the recipe as "deck empty, port full **or** the trigger
/// declared".
pub fn late_base_2p(seed: u64, rules: &Rules) -> Result<GameState, FixtureError> {
    let _ = (seed, rules);
    Err(FixtureError::Unimplemented("late_base_2p"))
}

// ---------------------------------------------------------------------------
// Digest and fingerprints (spec §5.1)
// ---------------------------------------------------------------------------

/// FNV-1a 64, the repo's own content-hash primitive (`Rules::content_hash`,
/// `game/rules.rs:41–61`) — restated here so a digest and a rules hash are the
/// same kind of number, and so this module depends on no hashing crate.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// **THE PACKAGE DIGEST** — one content hash over the registered TUPLES.
///
/// §5.1: "The frozen package (all 34 + 2 as code, printed deltas, a digest over
/// the registered tuples) returns to the review cadence before any solver or
/// collision run; every Part-A/B artifact prints the digest."
///
/// # What it covers, and why the axis is in it
///
/// Every field of every [`PairDeclaration`] in [`REGISTERED`], in registry
/// order, plus the thirty-four-row AXIS by name. The axis is included
/// deliberately: a registration's `expected_nonzero` list names rows on that
/// axis, so a renamed or reordered feature changes what a registration MEANS
/// without changing a single declaration byte. A digest that did not move there
/// would certify a package that had silently become a different one.
///
/// # What it does NOT cover
///
/// State literal bytes. Those are a per-fixture [`state_fingerprint`], because
/// the two claims are different: this digest says WHICH PACKAGE, the fingerprint
/// says WHICH BYTES a re-read looked at. §5.1 leaves "each fixture's literal
/// state bytes" open to step 2 precisely because they are not part of the
/// pre-registered tuple.
pub fn package_digest() -> String {
    format!("{:016x}", fnv1a(canonical_package().as_bytes()))
}

/// The canonical serialization [`package_digest`] hashes — public so a review
/// can read the bytes rather than trust the hash.
///
/// **The format**, one field per line, `KEY|value`, pairs in registry order and
/// fields in declaration order:
///
/// ```text
/// VERSION|<DIGEST_FORMAT_VERSION>
/// AXIS|<index>|<label>|<name>          (one line per axis row, 34 of them)
/// COUNT|<REGISTERED_PAIR_COUNT>|<REGISTERED_PAIR_TARGET>
/// PAIR|<id>|<name>|<family>|<role>
/// SIDE|<id>|G=<greater_side>|L=<lesser_side>
/// STAGE|<id>|<stage>,<stage>
/// VIEW|<id>|<view class>
/// MARGIN|<id>|<margin class>
/// CARRIER|<id>|<carrier label or ->
/// PRES|<id>|subject=<decl>|opponent=<decl>
/// DRAWS|<id>|a=<type>|b=<type>|c=<type>|moved=[..]|yen=<bool>
/// TECH|<id>|excluded=[..]|max=<n or ->
/// LAYOUT|<id>|<layout>
/// PARAM|<id>|<key>=<value>             (one line per registered parameter)
/// NONZERO|<id>|<label>,<label>,…
/// HANDCAP|<id>|<deviation reason or ->
/// NOTE|<id>|<n>|<note>                 (one line per note)
/// ```
///
/// Ordering is total and nothing is a `Debug` of a collection whose iteration
/// order could drift: every list is walked in its declared slice order.
pub fn canonical_package() -> String {
    let mut out = String::new();
    out.push_str(&format!("VERSION|{DIGEST_FORMAT_VERSION}\n"));
    for row in FeatureId::all() {
        out.push_str(&format!("AXIS|{}|{}|{}\n", row.index(), row.label(), row.name()));
    }
    out.push_str(&format!("COUNT|{REGISTERED_PAIR_COUNT}|{REGISTERED_PAIR_TARGET}\n"));
    for pair in REGISTERED.iter() {
        out.push_str(&canonical_declaration(pair));
    }
    out
}

/// One declaration's lines of [`canonical_package`].
pub fn canonical_declaration(pair: &PairDeclaration) -> String {
    let id = pair.id;
    let mut out = String::new();
    out.push_str(&format!(
        "PAIR|{id}|{}|{}|{}\n",
        pair.name,
        pair.family,
        pair.role.canonical()
    ));
    out.push_str(&format!("SIDE|{id}|G={}|L={}\n", pair.greater_side, pair.lesser_side));
    let stages: Vec<&str> = pair.stages.iter().map(|s| s.name()).collect();
    out.push_str(&format!("STAGE|{id}|{}\n", stages.join(",")));
    out.push_str(&format!("VIEW|{id}|{}\n", pair.view_class.canonical()));
    out.push_str(&format!("MARGIN|{id}|{}\n", pair.margin.canonical()));
    out.push_str(&format!(
        "CARRIER|{id}|{}\n",
        pair.carrier.map_or("-", |c| c.label())
    ));
    out.push_str(&format!(
        "PRES|{id}|subject={}|opponent={}\n",
        pair.presidents.subject.canonical(),
        pair.presidents.opponent.canonical()
    ));
    let moved: Vec<String> =
        pair.draws.goods_moved.iter().map(|a| format!("{a:?}")).collect();
    out.push_str(&format!(
        "DRAWS|{id}|a={:?}|b={:?}|c={:?}|moved=[{}]|yen={}\n",
        pair.draws.category_a,
        pair.draws.category_b,
        pair.draws.category_c,
        moved.join(","),
        pair.draws.moves_yen
    ));
    let excluded: Vec<String> =
        pair.techs.excluded_effects.iter().map(|e| format!("{e:?}")).collect();
    out.push_str(&format!(
        "TECH|{id}|excluded=[{}]|max={}\n",
        excluded.join(","),
        pair.techs.max_owned.map_or_else(|| "-".to_string(), |n| n.to_string())
    ));
    out.push_str(&format!("LAYOUT|{id}|{}\n", pair.layout.canonical()));
    for (key, value) in pair.parameters {
        out.push_str(&format!("PARAM|{id}|{key}={value}\n"));
    }
    let nonzero: Vec<&str> = pair.expected_nonzero.iter().map(|f| f.label()).collect();
    out.push_str(&format!("NONZERO|{id}|{}\n", nonzero.join(",")));
    out.push_str(&format!(
        "HANDCAP|{id}|{}\n",
        pair.early_hand_cap_deviation.unwrap_or("-")
    ));
    for (i, note) in pair.notes.iter().enumerate() {
        out.push_str(&format!("NOTE|{id}|{i}|{note}\n"));
    }
    out
}

/// The digest serialization's own version.
///
/// A format change moves every digest, which is correct — a re-read pinning a
/// digest is pinning what the format meant when it was taken — and stamping it
/// is what makes that legible instead of mysterious.
pub const DIGEST_FORMAT_VERSION: u32 = 1;

/// **ONE FIXTURE SIDE'S LITERAL-BYTE FINGERPRINT**: FNV-1a over the state's
/// canonical JSON.
///
/// §5.1's step-2 gate leaves "each fixture's literal state bytes" open, and this
/// is what closes the loop on them: a bounded re-read records the fingerprint,
/// and a later run that reads different bytes says so. JSON rather than a
/// hand-rolled walk because `GameState` is `Serialize` and its collections are
/// `BTreeMap`s, so the encoding is total and deterministic without this module
/// re-deriving what a state contains.
pub fn state_fingerprint(state: &GameState) -> String {
    let _ = state;
    "0000000000000000".to_string()
}

// ---------------------------------------------------------------------------
// The print seam (spec §5.1's printed deltas; step-3 join)
// ---------------------------------------------------------------------------

/// What a candidate column renders before step 3 builds it.
///
/// **Deliberately not `0`.** A zero in a delta table is a MEASUREMENT — the
/// statement that a feature does not separate the pair — and "the formula does
/// not exist yet" is a different claim entirely. §5.1 makes an unlisted nonzero
/// row a fixture defect; printing zeros for eighteen unbuilt columns would make
/// every registration look defect-free for the wrong reason.
pub const CANDIDATE_PENDING: &str = "n/a-until-step-3";

/// **THE STEP-3 SEAM.** A function that answers one candidate feature for one
/// seat of one view, or `None` where it declines to.
///
/// Step 3 builds the eighteen helpers in `eval.rs` (spec §13) and passes an
/// adapter of this shape to [`render_pair_report_with`]; nothing else in this
/// module changes when it does. The signature is deliberately the same shape as
/// `eval::raw_value` — `(view, player, feature, rules) -> i64` — so the adapter
/// is a match, not a translation layer.
pub type CandidateColumns =
    fn(&PlayerView, &VisiblePlayer, CandidateFeature, &Rules) -> Option<i64>;

/// Render one fixture's report with the candidate columns still pending.
pub fn render_pair_report(fixture: &PairFixture, rules: &Rules) -> String {
    render_pair_report_with(fixture, rules, None)
}

/// **THE PRINT HARNESS** — one fixture's declaration, per-side fingerprints and
/// full 34-row delta table.
///
/// The sixteen INCUMBENT rows carry real numbers, read through `eval`'s existing
/// public surface ([`ScoreStructure::derive`] / [`ScoreStructure::value_of`])
/// rather than re-derived here. The eighteen CANDIDATE rows render through
/// `columns`; where it is `None`, or returns `None`, they render
/// [`CANDIDATE_PENDING`].
///
/// Every report prints [`package_digest`], because §5.1 says every Part-A/B
/// artifact does.
pub fn render_pair_report_with(
    fixture: &PairFixture,
    rules: &Rules,
    columns: Option<CandidateColumns>,
) -> String {
    let _ = (fixture, rules, columns);
    String::new()
}

/// The registry's own manifest: what is registered, how far it is from §5.1's
/// counts, and the digest.
pub fn render_registry_manifest() -> String {
    let mut out = String::new();
    out.push_str("pair-fixture package\n");
    out.push_str(&format!("digest              {}\n", package_digest()));
    out.push_str(&format!("digest-format       v{DIGEST_FORMAT_VERSION}\n"));
    out.push_str(&format!("axis-width          {AXIS_WIDTH}\n"));
    out.push_str(&format!(
        "registered          {REGISTERED_PAIR_COUNT} of {REGISTERED_PAIR_TARGET}\n"
    ));
    for pair in REGISTERED.iter() {
        let stages: Vec<&str> = pair.stages.iter().map(|s| s.name()).collect();
        out.push_str(&format!(
            "  {:<8} fam {:<2} {:<14} {:<16} {}\n",
            pair.id,
            pair.family,
            pair.role.canonical(),
            stages.join("+"),
            pair.name
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Construction helpers (restated for the LIBRARY, on `driven_families`' grain)
// ---------------------------------------------------------------------------

/// A finalized 2P game with both seats keeping their first dealt Order.
///
/// `tests/scenario_suite.rs:446`'s `keep_game`, restated here for the reason
/// `driven_families::keep_game_2p` states: these builders ship in the LIBRARY
/// and a test helper is not reachable from it. Keeping (rather than choosing
/// `Unknown`) is what makes both seats observable — `observe` refuses a seat
/// whose own deal is unrecorded, and §5.3 signs the mover's view.
pub fn keep_game_2p(seed: u64, rules: &Rules) -> Result<GameState, FixtureError> {
    let pending =
        new_game(GameConfig::for_player_count(2), rules, seed).map_err(FixtureError::Setup)?;
    finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
        .map_err(FixtureError::Setup)
}

/// The seat that is not `subject`, in a 2P state.
pub fn other_seat_2p(state: &GameState, subject: PlayerId) -> Result<PlayerId, FixtureError> {
    state
        .players
        .iter()
        .map(|p| p.id)
        .find(|id| *id != subject)
        .ok_or(FixtureError::OutOfStock("a rival seat"))
}

/// Whether every port board in play is FULL — §5.1's late recipe's other half.
pub fn port_is_full(state: &GameState) -> bool {
    let a = state.port_state.port_a.slots.iter().all(Option::is_some);
    let b = state
        .port_state
        .port_b
        .as_ref()
        .map_or(true, |p| p.slots.iter().all(Option::is_some));
    a && b
}

/// The president's area when it stands on the board.
fn president_area(player: &crate::types::PlayerState) -> Option<AreaId> {
    match player.president_location {
        PresidentLocation::InHand => None,
        PresidentLocation::OnBoard(a) => Some(a),
    }
}

/// Whether `seat` owns a technology with the given effect — the engine's own
/// ownership test (EFFECT equality; ten of twenty titles have two copies).
fn owns_effect(state: &GameState, seat: PlayerId, effect: TechEffect, rules: &Rules) -> bool {
    state.player(seat).technologies.iter().any(|id| {
        rules.techs.iter().any(|c| c.id == *id && c.effect == effect)
    })
}

/// The category a declared draw slot must carry, by index — the shape
/// `game/setup.rs:113–117` deals unconditionally.
fn declared_category(index: usize) -> AchievementCategory {
    match index {
        0 => AchievementCategory::A,
        1 => AchievementCategory::B,
        _ => AchievementCategory::C,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::eval::{explain, WeightTable};
    use crate::game::{check_invariants, observe};
    use crate::types::{OrderCardId, TechCardId};

    fn rules() -> Rules {
        Rules::standard()
    }

    // -----------------------------------------------------------------------
    // THROWAWAY declarations — infrastructure fixtures only, deliberately
    // OUTSIDE `REGISTERED`. Nothing here is a §5.2 pair; each exists to prove
    // one clause of the frame. A §5.2 registration lands in `REGISTERED`, in a
    // later task, and never here.
    // -----------------------------------------------------------------------

    const THROWAWAY_DRAWS: AchievementDraws = AchievementDraws {
        category_a: AchievementType::Copper5,
        category_b: AchievementType::Orders5,
        category_c: AchievementType::Commercial4,
        goods_moved: &[],
        moves_yen: false,
    };

    /// A hand-off, both-endpoints, direction-only throwaway whose only expected
    /// nonzero row is `victory_points`.
    const THROWAWAY_VP: PairDeclaration = PairDeclaration {
        id: "T(vp)",
        name: "THROWAWAY: one more VP ≻ one fewer",
        family: 0,
        greater_side: "one-vp",
        lesser_side: "zero-vp",
        role: PairRole::Diagnostic,
        stages: &[StageEndpoint::Early, StageEndpoint::Late],
        view_class: ViewClass::HandOff,
        margin: MarginClass::DirectionOnly,
        carrier: Some(FeatureId::Incumbent(Feature::VictoryPoints)),
        presidents: SeatPresidents::BOTH_IN_HAND,
        draws: THROWAWAY_DRAWS,
        techs: TechDeclaration::UNCONSTRAINED,
        layout: LayoutClass::StandardGenerated,
        parameters: &[],
        expected_nonzero: &[FeatureId::Incumbent(Feature::VictoryPoints)],
        early_hand_cap_deviation: None,
        notes: &["infrastructure throwaway; not a §5.2 pair"],
    };

    /// The same pair with an EMPTY expected-nonzero list — the fixture defect
    /// §5.1 calls out.
    const THROWAWAY_VP_UNLISTED: PairDeclaration =
        PairDeclaration { id: "T(vp-unlisted)", expected_nonzero: &[], ..THROWAWAY_VP };

    /// A throwaway declaring the subject's president on-board at an EMPTY area.
    const THROWAWAY_PRESIDENT_EMPTY: PairDeclaration = PairDeclaration {
        id: "T(pres-empty)",
        presidents: SeatPresidents {
            subject: PresidentDecl::OnBoardEmptyArea,
            opponent: PresidentDecl::InHand,
        },
        ..THROWAWAY_VP
    };

    /// A throwaway declaring the subject's president on-board WITH its own
    /// assistants and NO Telephone exception — §5.1's unreachable shape.
    const THROWAWAY_PRESIDENT_OCCUPIED: PairDeclaration = PairDeclaration {
        id: "T(pres-occupied)",
        presidents: SeatPresidents {
            subject: PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: false },
            opponent: PresidentDecl::InHand,
        },
        ..THROWAWAY_VP
    };

    /// The same, WITH the exception claimed.
    const THROWAWAY_PRESIDENT_TELEPHONE: PairDeclaration = PairDeclaration {
        id: "T(pres-telephone)",
        presidents: SeatPresidents {
            subject: PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: true },
            opponent: PresidentDecl::InHand,
        },
        ..THROWAWAY_VP
    };

    /// A mid-turn throwaway at the AreaAction phase.
    const THROWAWAY_MID_TURN: PairDeclaration = PairDeclaration {
        id: "T(mid)",
        view_class: ViewClass::MidTurn { phase: TurnPhase::AreaAction, mercenary: false },
        stages: &[StageEndpoint::Early],
        ..THROWAWAY_VP
    };

    /// A throwaway whose recipe moves copper and yen — the registered draw
    /// constraint's two halves in one declaration.
    const THROWAWAY_DRAW_CONSTRAINED: PairDeclaration = PairDeclaration {
        id: "T(draws)",
        draws: AchievementDraws {
            goods_moved: &[AchievementType::Copper5],
            moves_yen: true,
            ..THROWAWAY_DRAWS
        },
        ..THROWAWAY_VP
    };

    /// The same, with the A-card outside the moved set but the B-card on
    /// Money10 — the yen half of the registered draw constraint.
    const THROWAWAY_DRAWS_MONEY10: PairDeclaration = PairDeclaration {
        id: "T(draws-yen)",
        draws: AchievementDraws {
            category_a: AchievementType::Silk6,
            category_b: AchievementType::Money10,
            category_c: AchievementType::Commercial4,
            goods_moved: &[AchievementType::Copper5],
            moves_yen: true,
        },
        ..THROWAWAY_VP
    };

    /// A throwaway excluding System of Stock.
    const THROWAWAY_TECH_EXCLUDED: PairDeclaration = PairDeclaration {
        id: "T(tech)",
        techs: TechDeclaration {
            excluded_effects: &[TechEffect::SystemOfStock],
            max_owned: None,
        },
        ..THROWAWAY_VP
    };

    /// A throwaway registering an early hand-cap deviation.
    const THROWAWAY_HAND_CAP_DEVIATION: PairDeclaration = PairDeclaration {
        id: "T(handcap)",
        early_hand_cap_deviation: Some("infrastructure throwaway: a second hand card"),
        ..THROWAWAY_VP
    };

    // -----------------------------------------------------------------------
    // Throwaway construction helpers
    // -----------------------------------------------------------------------

    const SEED: u64 = 0x5A2F_0001;

    /// The two sides of `THROWAWAY_VP` at one endpoint: identical states except
    /// one victory point on the greater side.
    fn vp_pair(stage: StageEndpoint, rules: &Rules) -> (GameState, GameState, PlayerId) {
        let lesser = stage_base_2p(stage, SEED, rules).expect("a stage base builds");
        let subject = lesser.current_player;
        let mut greater = lesser.clone();
        greater.player_mut(subject).victory_points += 1;
        (greater, lesser, subject)
    }

    /// Move one of `seat`'s assistants from hand onto `area`.
    fn place_one(state: &mut GameState, seat: PlayerId, area: AreaId) {
        let p = state.player_mut(seat);
        assert!(p.assistants.in_hand >= 1, "the fixture needs a hand assistant");
        p.assistants.in_hand -= 1;
        *p.assistants.on_board.entry(area).or_insert(0) += 1;
    }

    /// The first non-Canal area of the board.
    fn some_non_canal_area(state: &GameState) -> AreaId {
        *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type != crate::types::AreaType::Canal)
            .expect("a board has non-Canal areas")
            .0
    }

    /// A second non-Canal area, distinct from `not`.
    fn another_non_canal_area(state: &GameState, not: AreaId) -> AreaId {
        *state
            .board_layout
            .areas
            .iter()
            .find(|(id, cfg)| **id != not && cfg.area_type != crate::types::AreaType::Canal)
            .expect("a board has two non-Canal areas")
            .0
    }

    /// Give `seat` one technology carrying `effect`, taken out of whichever
    /// pile setup left it in so conservation holds.
    fn grant_effect(state: &mut GameState, seat: PlayerId, effect: TechEffect, rules: &Rules) {
        let id: TechCardId = rules
            .techs
            .iter()
            .find(|c| c.effect == effect)
            .expect("the registry prints this effect")
            .id;
        if let Some(i) = state.tech_deck.iter().position(|c| *c == id) {
            state.tech_deck.remove(i);
        } else {
            let labs =
                std::iter::once(&mut state.lab_state.lab_a).chain(state.lab_state.lab_b.as_mut());
            let mut found = false;
            for slot in labs.flat_map(|lab| lab.slots.iter_mut()) {
                if *slot == Some(id) {
                    *slot = None;
                    found = true;
                    break;
                }
            }
            assert!(found, "the technology sits in no pile");
        }
        state.player_mut(seat).technologies.push(id);
    }

    /// Move one card from the box into `seat`'s hand — the only way to exceed
    /// the early hand cap without moving the deck clock.
    fn deal_from_box(state: &mut GameState, seat: PlayerId) -> OrderCardId {
        let id = state.orders_removed.pop().expect("the box holds cards");
        state.player_mut(seat).order_cards_in_hand.push(id);
        id
    }

    // -----------------------------------------------------------------------
    // The axis
    // -----------------------------------------------------------------------

    #[test]
    fn the_axis_is_thirty_four_wide_and_round_trips_every_index() {
        assert_eq!(AXIS_WIDTH, 34, "§3.2's width: 16 incumbents + 18 candidates");
        let axis = FeatureId::all();
        for (i, row) in axis.iter().enumerate() {
            assert_eq!(row.index(), i, "{} is not at its own index", row.name());
        }
        for c in CandidateFeature::ALL {
            assert_eq!(CandidateFeature::ALL[c.index()], c, "{} mis-indexes", c.label());
        }
    }

    #[test]
    fn the_axis_names_and_labels_are_unique() {
        let axis = FeatureId::all();
        let mut names: Vec<&str> = axis.iter().map(|r| r.name()).collect();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total, "two axis rows share a name");
        let mut labels: Vec<&str> = axis.iter().map(|r| r.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), total, "two axis rows share a label");
    }

    #[test]
    fn the_axis_holds_every_incumbent_first_then_every_candidate() {
        let axis = FeatureId::all();
        for f in Feature::ALL {
            assert_eq!(axis[f.index()], FeatureId::Incumbent(f));
            assert!(!axis[f.index()].is_candidate());
        }
        for c in CandidateFeature::ALL {
            let row = axis[Feature::COUNT + c.index()];
            assert_eq!(row, FeatureId::Candidate(c));
            assert!(row.is_candidate());
        }
        assert_eq!(CandidateFeature::COUNT, 18, "§3.2 registers eighteen candidates");
    }

    // -----------------------------------------------------------------------
    // The registry
    // -----------------------------------------------------------------------

    #[test]
    fn the_registry_is_empty_at_this_task_and_pinned_to_the_spec_target() {
        assert_eq!(
            REGISTERED.len(),
            REGISTERED_PAIR_COUNT,
            "the array's length IS the count pin",
        );
        assert_eq!(
            REGISTERED_PAIR_COUNT, 0,
            "S2-T3 builds the frame; every §5.2 pair lands in a later task, and this \
             number is what those tasks bump toward {REGISTERED_PAIR_TARGET}",
        );
        assert_eq!(
            REGISTERED_PAIR_TARGET, 36,
            "§5.1's registered counts: 34 gate pairs plus the 2(c)/10(c) diagnostics",
        );
    }

    #[test]
    fn every_registered_pair_is_uniquely_named_and_internally_consistent() {
        let mut ids: Vec<&str> = REGISTERED.iter().map(|p| p.id).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "two registered pairs share an id");
        for pair in REGISTERED.iter() {
            assert!(!pair.stages.is_empty(), "{} registers no stage endpoint", pair.id);
            let mut seen: Vec<StageEndpoint> = pair.stages.to_vec();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), pair.stages.len(), "{} repeats a stage", pair.id);
            for stage in pair.stages {
                assert!(
                    pair.margin.margin_at(*stage).is_some(),
                    "{} declares {} but registers no margin there",
                    pair.id,
                    stage.name(),
                );
            }
            assert!((1..=11).contains(&pair.family), "{} has family {}", pair.id, pair.family);
        }
    }

    // -----------------------------------------------------------------------
    // Stage realization
    // -----------------------------------------------------------------------

    #[test]
    fn the_early_base_realizes_all_four_clocks_at_zero() {
        let rules = rules();
        let state = early_base_2p(SEED, &rules).expect("an early base builds");
        check_invariants(&state, &rules).expect("engine-valid");
        let view = observe(&state, state.current_player).expect("observable");
        assert_eq!(
            eval::clock_ppm(&view, &rules),
            [0, 0, 0, 0],
            "§5.1's p = 0: full deck, zero real track pawns, zero buildings",
        );
        assert_eq!(eval::progress_ppm(&view, &rules), StageEndpoint::Early.ppm());
        assert_eq!(
            view.order_deck_len,
            eval::initial_order_deck_len(&view, &rules).expect("a derivable initial size"),
            "a FULL deck is the deck clock's own denominator",
        );
        for p in &state.players {
            assert!(
                p.order_cards_in_hand.len() <= 1
                    && p.order_cards_in_hand.iter().all(|c| Some(*c) == p.starting_order),
                "§5.1's hand cap: at a full deck no port take has happened",
            );
            assert_eq!(p.shophouses.total_built(), 0);
            assert_eq!(p.trading_houses.total_built(), 0);
        }
        assert_eq!(state.church_track.total_assistants(), 0, "the D1 dummy is not counted");
        assert_eq!(state.customs_track.total_assistants(), 0);
    }

    #[test]
    fn the_late_base_saturates_the_deck_and_leaves_the_game_running() {
        let rules = rules();
        let state = late_base_2p(SEED, &rules).expect("a late base builds");
        check_invariants(&state, &rules).expect("engine-valid");
        assert!(state.order_deck.is_empty(), "deck saturation");
        assert!(port_is_full(&state), "§5.1's late recipe keeps the port full");
        assert!(!state.end_game_triggered, "the deck trigger needs an empty port slot too");
        let view = observe(&state, state.current_player).expect("observable");
        assert_eq!(eval::progress_ppm(&view, &rules), StageEndpoint::Late.ppm());
        assert_eq!(
            eval::clock_ppm(&view, &rules)[0],
            PROGRESS_FULL_PPM,
            "the DECK clock is the one that reaches full",
        );
        assert_eq!(
            &eval::clock_ppm(&view, &rules)[1..],
            &[0, 0, 0],
            "and the other three stay at zero, so a pair's track/building deltas \
             are absorbed by the max",
        );
        assert!(eval::terminal_standing(&view, &rules).is_none(), "still running");
    }

    #[test]
    fn both_stage_bases_conserve_every_card_pile() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let state = stage_base_2p(stage, SEED, &rules).expect("a stage base builds");
            let ports = state.port_state.port_a.slots.iter().flatten().count();
            let hands: usize =
                state.players.iter().map(|p| p.order_cards_in_hand.len()).sum();
            let completed: usize = state.players.iter().map(|p| p.completed_orders.len()).sum();
            assert_eq!(
                state.order_deck.len() + ports + hands + completed + state.orders_removed.len(),
                36,
                "{} base broke order conservation",
                stage.name(),
            );
            for p in &state.players {
                assert_eq!(p.assistants.total(), 20);
            }
        }
    }

    #[test]
    fn the_stage_endpoints_are_the_two_points_interpolate_is_exact_at() {
        // The citation this module's stage rule rests on: `eval::interpolate`
        // returns `early` at 0 and `late` at full, so a row registered at an
        // endpoint is linear in ONE vector.
        assert_eq!(eval::interpolate(7, 99, StageEndpoint::Early.ppm()), 7);
        assert_eq!(eval::interpolate(7, 99, StageEndpoint::Late.ppm()), 99);
    }

    // -----------------------------------------------------------------------
    // Fixture admission
    // -----------------------------------------------------------------------

    #[test]
    fn a_fixture_admits_a_well_formed_pair_at_both_endpoints() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let (greater, lesser, subject) = vp_pair(stage, &rules);
            let fixture =
                PairFixture::new(&THROWAWAY_VP, stage, greater, lesser, subject, &rules)
                    .unwrap_or_else(|e| panic!("{} pair must admit: {e}", stage.name()));
            assert_eq!(fixture.stage(), stage);
            assert_eq!(fixture.subject(), subject);
            assert_eq!(fixture.progress_ppm(&rules), stage.ppm());
            let delta = fixture.incumbent_delta(&rules);
            assert_eq!(delta[Feature::VictoryPoints.index()], 1);
            for f in Feature::ALL {
                if f != Feature::VictoryPoints {
                    assert_eq!(delta[f.index()], 0, "{} moved and should not have", f.name());
                }
            }
            fixture.listed_incumbent_rows_are_nonzero(&rules).expect("VP is listed and nonzero");
        }
    }

    #[test]
    fn a_fixture_refuses_a_stage_the_declaration_does_not_register() {
        let rules = rules();
        let (greater, lesser, subject) = vp_pair(StageEndpoint::Late, &rules);
        let err = PairFixture::new(
            &THROWAWAY_MID_TURN,
            StageEndpoint::Late,
            greater,
            lesser,
            subject,
            &rules,
        )
        .expect_err("T(mid) registers EARLY only");
        assert!(format!("{err}").contains("stage"), "got: {err}");
    }

    #[test]
    fn a_fixture_refuses_sides_at_different_stages() {
        let rules = rules();
        let early = early_base_2p(SEED, &rules).expect("early base");
        let late = late_base_2p(SEED, &rules).expect("late base");
        let subject = early.current_player;
        let err =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Early, late, early, subject, &rules)
                .expect_err("the two sides read different progress");
        assert!(format!("{err}").contains("progress"), "got: {err}");
    }

    #[test]
    fn a_fixture_refuses_a_terminal_side() {
        let rules = rules();
        let (mut greater, lesser, subject) = vp_pair(StageEndpoint::Late, &rules);
        // The engine's own terminal shape (`eval::view_is_terminal`): triggered,
        // final round, a recorded trigger player, and the turn resting at the
        // end of the seat before the start player.
        let seats = i32::from(greater.config.player_count);
        let last = (i32::from(greater.config.start_player.0) + seats - 1).rem_euclid(seats);
        greater.end_game_triggered = true;
        greater.final_round = true;
        greater.trigger_player = Some(greater.config.start_player);
        greater.current_player = PlayerId(last as u8);
        greater.turn_phase = TurnPhase::TurnEnd;
        let _ = subject;
        let err = PairFixture::new(
            &THROWAWAY_VP,
            StageEndpoint::Late,
            greater,
            lesser,
            PlayerId(last as u8),
            &rules,
        )
        .expect_err("a terminal side is not a positional pair");
        assert!(format!("{err}").contains("terminal"), "got: {err}");
    }

    #[test]
    fn a_fixture_refuses_a_subject_that_is_not_the_mover() {
        let rules = rules();
        let (greater, lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        let idle = other_seat_2p(&lesser, subject).expect("a rival seat");
        let err =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Early, greater, lesser, idle, &rules)
                .expect_err("§5.3 signs the MOVER's view");
        assert!(format!("{err}").contains("mover"), "got: {err}");
    }

    #[test]
    fn a_fixture_refuses_a_state_that_is_not_two_player() {
        let rules = rules();
        let pending = new_game(GameConfig::for_player_count(3), &rules, SEED).expect("3P setup");
        let state = finalize_setup(
            pending,
            &[
                StartingOrderChoice::Keep(0),
                StartingOrderChoice::Keep(0),
                StartingOrderChoice::Keep(0),
            ],
        )
        .expect("finalize");
        let subject = state.current_player;
        let err = PairFixture::new(
            &THROWAWAY_VP,
            StageEndpoint::Early,
            state.clone(),
            state,
            subject,
            &rules,
        )
        .expect_err("§5.1's stage realization and §5.3's signature are 2P");
        assert!(format!("{err}").contains("2P") || format!("{err}").contains("player"), "got: {err}");
    }

    #[test]
    fn a_fixture_refuses_a_hand_off_state_at_the_wrong_phase() {
        let rules = rules();
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        greater.turn_phase = TurnPhase::AreaAction;
        lesser.turn_phase = TurnPhase::AreaAction;
        let err =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Early, greater, lesser, subject, &rules)
                .expect_err("a hand-off rests at AdditionalActionFirst");
        assert!(format!("{err}").contains("view class"), "got: {err}");
    }

    #[test]
    fn a_mid_turn_fixture_holds_its_declared_phase_and_mercenary_flag() {
        let rules = rules();
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        greater.turn_phase = TurnPhase::AreaAction;
        lesser.turn_phase = TurnPhase::AreaAction;
        PairFixture::new(
            &THROWAWAY_MID_TURN,
            StageEndpoint::Early,
            greater.clone(),
            lesser.clone(),
            subject,
            &rules,
        )
        .expect("a mid-turn pair at its declared phase admits");

        greater.is_mercenary_action = true;
        let err = PairFixture::new(
            &THROWAWAY_MID_TURN,
            StageEndpoint::Early,
            greater,
            lesser,
            subject,
            &rules,
        )
        .expect_err("the mercenary flag is declared");
        assert!(format!("{err}").contains("view class"), "got: {err}");
    }

    #[test]
    fn a_fixture_refuses_an_undeclared_president_location() {
        let rules = rules();
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        let area = some_non_canal_area(&lesser);
        greater.player_mut(subject).president_location = PresidentLocation::OnBoard(area);
        lesser.player_mut(subject).president_location = PresidentLocation::OnBoard(area);
        let err =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Early, greater, lesser, subject, &rules)
                .expect_err("§5.1's DEFAULT is InHand unless the registration says otherwise");
        assert!(format!("{err}").contains("president"), "got: {err}");
    }

    #[test]
    fn a_hand_off_president_may_stand_on_an_area_holding_none_of_its_own() {
        let rules = rules();
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        let area = some_non_canal_area(&lesser);
        for side in [&mut greater, &mut lesser] {
            side.player_mut(subject).president_location = PresidentLocation::OnBoard(area);
        }
        PairFixture::new(
            &THROWAWAY_PRESIDENT_EMPTY,
            StageEndpoint::Early,
            greater,
            lesser,
            subject,
            &rules,
        )
        .expect("the post-Recovery hand-off shape");
    }

    #[test]
    fn a_hand_off_president_may_not_stand_on_its_own_assistants() {
        let rules = rules();
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        let area = some_non_canal_area(&lesser);
        for side in [&mut greater, &mut lesser] {
            place_one(side, subject, area);
            side.player_mut(subject).president_location = PresidentLocation::OnBoard(area);
        }
        // Declared as an EMPTY area: the declaration and the state disagree.
        let err = PairFixture::new(
            &THROWAWAY_PRESIDENT_EMPTY,
            StageEndpoint::Early,
            greater.clone(),
            lesser.clone(),
            subject,
            &rules,
        )
        .expect_err("the area is not empty");
        assert!(format!("{err}").contains("president"), "got: {err}");

        // Declared honestly, but WITHOUT the Telephone exception: §5.1 calls
        // the shape unreachable post-Recovery.
        let err = PairFixture::new(
            &THROWAWAY_PRESIDENT_OCCUPIED,
            StageEndpoint::Early,
            greater,
            lesser,
            subject,
            &rules,
        )
        .expect_err("unreachable post-Recovery, Telephone excepted");
        assert!(format!("{err}").contains("Telephone"), "got: {err}");
    }

    #[test]
    fn the_telephone_exception_requires_the_seat_to_actually_own_telephone() {
        let rules = rules();
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        let area = some_non_canal_area(&lesser);
        for side in [&mut greater, &mut lesser] {
            place_one(side, subject, area);
            side.player_mut(subject).president_location = PresidentLocation::OnBoard(area);
        }
        let err = PairFixture::new(
            &THROWAWAY_PRESIDENT_TELEPHONE,
            StageEndpoint::Early,
            greater.clone(),
            lesser.clone(),
            subject,
            &rules,
        )
        .expect_err("the exception is claimed but the tech is not owned");
        assert!(format!("{err}").contains("Telephone"), "got: {err}");

        // Granted on BOTH sides, so the tech moves no incumbent delta.
        for side in [&mut greater, &mut lesser] {
            grant_effect(side, subject, TechEffect::Telephone, &rules);
        }
        PairFixture::new(
            &THROWAWAY_PRESIDENT_TELEPHONE,
            StageEndpoint::Early,
            greater,
            lesser,
            subject,
            &rules,
        )
        .expect("the exception holds once the tech is owned");
    }

    #[test]
    fn a_fixture_refuses_an_unlisted_nonzero_incumbent_row() {
        let rules = rules();
        let (greater, lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        let err = PairFixture::new(
            &THROWAWAY_VP_UNLISTED,
            StageEndpoint::Early,
            greater,
            lesser,
            subject,
            &rules,
        )
        .expect_err("§5.1: an unlisted nonzero row is a fixture defect");
        assert!(format!("{err}").contains("victory_points"), "got: {err}");
    }

    #[test]
    fn a_listed_row_that_does_not_move_is_reported_by_the_second_half_check() {
        let rules = rules();
        let lesser = early_base_2p(SEED, &rules).expect("early base");
        let subject = lesser.current_player;
        let greater = lesser.clone();
        let fixture =
            PairFixture::new(&THROWAWAY_VP_UNLISTED, StageEndpoint::Early, greater, lesser, subject, &rules)
                .expect("identical sides list nothing and move nothing");
        // The same identical-sides fixture under the LISTING declaration would
        // not admit at all, so the second half-check is exercised through the
        // one declaration that lists a row it cannot move.
        let _ = fixture;
        let lesser = early_base_2p(SEED, &rules).expect("early base");
        let greater = lesser.clone();
        let err = PairFixture::new(
            &THROWAWAY_VP,
            StageEndpoint::Early,
            greater,
            lesser,
            subject,
            &rules,
        )
        .map(|f| f.listed_incumbent_rows_are_nonzero(&rules))
        .expect("identical sides admit")
        .expect_err("victory_points is listed but does not move");
        assert!(format!("{err}").contains("victory_points"), "got: {err}");
    }

    #[test]
    fn a_fixture_refuses_a_draw_constraint_violation() {
        let rules = rules();
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        // The declaration says the recipe moves COPPER and YEN, so the drawn
        // A-card may not be Copper5 and the drawn B-card may not be Money10.
        for side in [&mut greater, &mut lesser] {
            side.achievements[0] =
                crate::types::AchievementCard::new(AchievementType::Copper5);
            side.achievements[1] = crate::types::AchievementCard::new(AchievementType::Orders5);
            side.achievements[2] =
                crate::types::AchievementCard::new(AchievementType::Commercial4);
        }
        let err = PairFixture::new(
            &THROWAWAY_DRAW_CONSTRAINED,
            StageEndpoint::Early,
            greater.clone(),
            lesser.clone(),
            subject,
            &rules,
        )
        .expect_err("the drawn A-card's good is inside the moved set");
        assert!(format!("{err}").contains("draw"), "got: {err}");

        // Move the A-card outside the set and the B-card onto Money10: the
        // other half of the same constraint.
        for side in [&mut greater, &mut lesser] {
            side.achievements[0] = crate::types::AchievementCard::new(AchievementType::Silk6);
            side.achievements[1] = crate::types::AchievementCard::new(AchievementType::Money10);
        }
        let err = PairFixture::new(
            &THROWAWAY_DRAWS_MONEY10,
            StageEndpoint::Early,
            greater,
            lesser,
            subject,
            &rules,
        )
        .expect_err("Money10 shadows E1 — §2's veto identity");
        assert!(format!("{err}").contains("Money10"), "got: {err}");
    }

    #[test]
    fn a_fixture_refuses_achievement_draws_the_states_do_not_carry() {
        let rules = rules();
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        for side in [&mut greater, &mut lesser] {
            side.achievements[0] = crate::types::AchievementCard::new(AchievementType::Tea7);
            side.achievements[1] = crate::types::AchievementCard::new(AchievementType::Orders5);
            side.achievements[2] =
                crate::types::AchievementCard::new(AchievementType::Commercial4);
        }
        let err =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Early, greater, lesser, subject, &rules)
                .expect_err("the declaration names Copper5");
        assert!(format!("{err}").contains("achievement"), "got: {err}");
    }

    #[test]
    fn a_fixture_refuses_an_excluded_technology() {
        let rules = rules();
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        for side in [&mut greater, &mut lesser] {
            grant_effect(side, subject, TechEffect::SystemOfStock, &rules);
        }
        let err = PairFixture::new(
            &THROWAWAY_TECH_EXCLUDED,
            StageEndpoint::Early,
            greater,
            lesser,
            subject,
            &rules,
        )
        .expect_err("3(a)/3(b)-class exclusions are declared and enforced");
        assert!(format!("{err}").contains("technolog"), "got: {err}");
    }

    #[test]
    fn the_early_hand_cap_is_enforced_unless_the_declaration_registers_a_deviation() {
        let rules = rules();
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        for side in [&mut greater, &mut lesser] {
            deal_from_box(side, subject);
        }
        let err =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Early, greater.clone(), lesser.clone(), subject, &rules)
                .expect_err("§5.1's p = 0 hand cap");
        assert!(format!("{err}").contains("hand"), "got: {err}");

        PairFixture::new(
            &THROWAWAY_HAND_CAP_DEVIATION,
            StageEndpoint::Early,
            greater,
            lesser,
            subject,
            &rules,
        )
        .expect("a registered deviation admits, and enters the digest");
    }

    #[test]
    fn a_fixture_refuses_a_side_the_engine_would_not_accept() {
        let rules = rules();
        let (mut greater, lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        greater.player_mut(subject).assistants.in_hand += 1; // 21 assistants
        let err =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Early, greater, lesser, subject, &rules)
                .expect_err("the constructibility mandate runs through check_invariants");
        assert!(format!("{err}").contains("engine"), "got: {err}");
    }

    // -----------------------------------------------------------------------
    // The delta read
    // -----------------------------------------------------------------------

    #[test]
    fn the_incumbent_delta_reads_the_relative_form_the_evaluator_scores() {
        let rules = rules();
        let (greater, lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        let fixture =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Early, greater, lesser, subject, &rules)
                .expect("admits");
        // At 2P `opponent_extreme` reduces to the single opponent's own value,
        // whatever the weight — the reduction §5.3's 2P-only scope rests on.
        // `explain` reports both halves, so the reduction is CHECKED here
        // rather than assumed by `relative_vector`.
        for side in [Side::Greater, Side::Lesser] {
            let view = fixture.view(side);
            let opponent = view
                .players
                .iter()
                .find(|p| p.id != subject && !p.id.is_dummy())
                .expect("a 2P view has one opponent");
            for table in [&WeightTable::V1, &WeightTable::V2] {
                let ex = explain(view, &rules, table);
                for term in &ex.terms {
                    assert_eq!(
                        term.opponent_value,
                        eval::raw_value(view, opponent, term.feature, &rules),
                        "{}'s opponent term is not the single rival's own value",
                        term.feature.name(),
                    );
                }
            }
        }
        let delta = fixture.incumbent_delta(&rules);
        assert_eq!(delta[Feature::VictoryPoints.index()], 1);
    }

    // -----------------------------------------------------------------------
    // Digest and fingerprints
    // -----------------------------------------------------------------------

    #[test]
    fn the_state_fingerprint_is_stable_and_separates_two_states() {
        let rules = rules();
        let a = early_base_2p(SEED, &rules).expect("early base");
        let mut b = a.clone();
        b.player_mut(b.current_player).victory_points += 1;
        assert_eq!(state_fingerprint(&a), state_fingerprint(&a), "deterministic");
        assert_eq!(state_fingerprint(&a).len(), 16, "fnv1a-64 hex");
        assert_ne!(state_fingerprint(&a), state_fingerprint(&b), "one VP is a different state");
    }

    #[test]
    fn the_package_digest_is_stable_and_covers_the_registered_tuples() {
        assert_eq!(package_digest(), package_digest(), "deterministic");
        assert_eq!(package_digest().len(), 16, "fnv1a-64 hex");
        let canon = canonical_package();
        assert!(canon.starts_with(&format!("VERSION|{DIGEST_FORMAT_VERSION}\n")));
        assert!(canon.contains(&format!(
            "COUNT|{REGISTERED_PAIR_COUNT}|{REGISTERED_PAIR_TARGET}\n"
        )));
        for pair in REGISTERED.iter() {
            assert!(canon.contains(&format!("PAIR|{}|", pair.id)), "{} is not digested", pair.id);
        }
    }

    #[test]
    fn the_package_digest_moves_with_a_declaration_and_with_the_axis() {
        // A declaration's bytes are what the digest is over: two declarations
        // differing in one field must serialize differently, or a registration
        // could change meaning without moving the hash.
        let a = canonical_declaration(&THROWAWAY_VP);
        let b = canonical_declaration(&THROWAWAY_VP_UNLISTED);
        assert_ne!(a, b, "the expected-nonzero list is part of the tuple");
        assert_ne!(fnv1a(a.as_bytes()), fnv1a(b.as_bytes()));
        let c = canonical_declaration(&THROWAWAY_PRESIDENT_EMPTY);
        assert_ne!(a, c, "the president declaration is part of the tuple");
        // The AXIS is in the package canon for the same reason: an
        // expected-nonzero list names rows on it.
        let canon = canonical_package();
        for row in FeatureId::all() {
            assert!(
                canon.contains(&format!("AXIS|{}|{}|{}\n", row.index(), row.label(), row.name())),
                "{} is missing from the digested axis",
                row.name(),
            );
        }
    }

    // -----------------------------------------------------------------------
    // The print seam
    // -----------------------------------------------------------------------

    #[test]
    fn the_report_prints_the_declaration_the_fingerprints_and_all_thirty_four_rows() {
        let rules = rules();
        let (greater, lesser, subject) = vp_pair(StageEndpoint::Late, &rules);
        let fixture =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Late, greater, lesser, subject, &rules)
                .expect("admits");
        let report = render_pair_report(&fixture, &rules);
        assert!(report.contains(THROWAWAY_VP.id), "the pair id");
        assert!(report.contains(THROWAWAY_VP.name), "the registration name");
        assert!(report.contains("late"), "the stage endpoint");
        assert!(report.contains(&format!("{}", PROGRESS_FULL_PPM)), "the exact ppm");
        assert!(report.contains(&package_digest()), "§5.1: every artifact prints the digest");
        assert!(
            report.contains(&state_fingerprint(fixture.state(Side::Greater))),
            "the greater side's literal-byte fingerprint",
        );
        assert!(
            report.contains(&state_fingerprint(fixture.state(Side::Lesser))),
            "the lesser side's fingerprint",
        );
        for row in FeatureId::all() {
            assert!(report.contains(row.name()), "{} has no row", row.name());
        }
        let pending = report.matches(CANDIDATE_PENDING).count();
        assert_eq!(
            pending,
            CandidateFeature::COUNT,
            "every candidate column is PENDING, and none of them prints a zero",
        );
        assert!(report.contains("hand-off"), "the view class");
        assert!(report.contains("direction-only"), "the margin class");
    }

    #[test]
    fn the_candidate_seam_renders_supplied_values_when_step_three_wires_it() {
        let rules = rules();
        let (greater, lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
        let fixture =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Early, greater, lesser, subject, &rules)
                .expect("admits");
        // A THROWAWAY stand-in for step 3's eighteen helpers: it answers one
        // column with a constant and declines the rest, so the report must show
        // both a real number and PENDING side by side.
        fn stub(
            _view: &PlayerView,
            _player: &VisiblePlayer,
            feature: CandidateFeature,
            _rules: &Rules,
        ) -> Option<i64> {
            match feature {
                CandidateFeature::MoneyStock => Some(0),
                _ => None,
            }
        }
        let report = render_pair_report_with(&fixture, &rules, Some(stub));
        assert_eq!(
            report.matches(CANDIDATE_PENDING).count(),
            CandidateFeature::COUNT - 1,
            "the one wired column stops printing PENDING",
        );
        assert!(report.contains(CandidateFeature::MoneyStock.name()));
    }

    #[test]
    fn the_registry_manifest_prints_the_digest_and_the_target() {
        let manifest = render_registry_manifest();
        assert!(manifest.contains(&package_digest()));
        assert!(manifest.contains(&format!("{REGISTERED_PAIR_COUNT} of {REGISTERED_PAIR_TARGET}")));
        assert!(manifest.contains(&format!("{AXIS_WIDTH}")));
    }

    // -----------------------------------------------------------------------
    // Small surface pins
    // -----------------------------------------------------------------------

    #[test]
    fn the_margin_classes_price_the_way_the_spec_states() {
        assert_eq!(MarginClass::DirectionOnly.margin_at(StageEndpoint::Early), Some(1));
        assert_eq!(MarginClass::DirectionOnly.margin_at(StageEndpoint::Late), Some(1));
        assert_eq!(MarginClass::Weak.margin_at(StageEndpoint::Late), Some(0));
        let m = MarginClass::Margined {
            early: Some(1),
            late: Some(14),
            carrier: FeatureId::Candidate(CandidateFeature::ProductionAccess),
        };
        assert_eq!(m.margin_at(StageEndpoint::Early), Some(1), "§5.1's early floor is 1");
        assert_eq!(m.margin_at(StageEndpoint::Late), Some(14));
        let late_only = MarginClass::Margined {
            early: None,
            late: Some(160),
            carrier: FeatureId::Incumbent(Feature::CountryBonusNow),
        };
        assert_eq!(late_only.margin_at(StageEndpoint::Early), None, "4(d) is LATE-only");
    }

    #[test]
    fn the_declared_achievement_categories_are_the_ones_setup_deals() {
        for (i, expected) in
            [AchievementCategory::A, AchievementCategory::B, AchievementCategory::C]
                .into_iter()
                .enumerate()
        {
            assert_eq!(declared_category(i), expected);
        }
        let rules = rules();
        let state = keep_game_2p(SEED, &rules).expect("setup");
        for (i, card) in state.achievements.iter().enumerate() {
            assert_eq!(
                card.achievement_type.category(),
                declared_category(i),
                "setup deals one card per category, in order",
            );
        }
    }

    #[test]
    fn the_helpers_read_the_engines_own_notions() {
        let rules = rules();
        let mut state = keep_game_2p(SEED, &rules).expect("setup");
        assert!(port_is_full(&state), "a fresh setup fills every port slot");
        let subject = state.current_player;
        let other = other_seat_2p(&state, subject).expect("a rival");
        assert_ne!(other, subject);
        assert_eq!(president_area(state.player(subject)), None, "presidents start in hand");
        let area = some_non_canal_area(&state);
        state.player_mut(subject).president_location = PresidentLocation::OnBoard(area);
        assert_eq!(president_area(state.player(subject)), Some(area));
        assert!(!owns_effect(&state, subject, TechEffect::Telephone, &rules));
        grant_effect(&mut state, subject, TechEffect::Telephone, &rules);
        assert!(owns_effect(&state, subject, TechEffect::Telephone, &rules));
        check_invariants(&state, &rules).expect("granting a tech keeps conservation");
        let second = another_non_canal_area(&state, area);
        assert_ne!(second, area);
    }
}
