//! **The V4 §5 PAIR-FIXTURE PACKAGE — infrastructure** (spec/pre-registration
//! `docs/plans/2026-08-20-phase3.5c-v4-spec-prereg.md` §5.1/§5.2, checkpoint §7
//! step 2).
//!
//! This module is the FRAME the §5.2 pairs land in, and the pairs themselves as
//! each family task registers them: the registered-tuple declaration type, the
//! fixture carrier with its construction invariants, the two stage-realization
//! builders, the package digest, the print seam — and, since S2-T4, §5.2's
//! FAMILY 1 (placement location) and FAMILY 11 (activation choice) as ten
//! registered tuples with the constructions that realize them. The remaining
//! twenty-six of §5.2's thirty-four gate pairs and two diagnostics land the same
//! way, and [`REGISTERED_PAIR_TARGET`] is what says how far there is to go.
//!
//! # Why a registration and its construction live in ONE file
//!
//! §5.1's step-2 gate is "the realized exact-tuple registry IS the completed
//! pre-registration", and the review that gate sends the package to reads a
//! declaration against the bytes that realize it. A declaration in one file and
//! its construction in another is two halves of one claim, and the half a
//! reviewer skips is the half that drifts. So [`PAIR_1A`]'s tuple, its
//! construction and the state facts its registered rows rest on are one
//! contiguous read, and each later family appends its own such block.
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
use crate::game::{
    check_invariants, finalize_setup, new_game, observe, Rules, StartingOrderChoice,
};
use crate::types::{
    AchievementCard, AchievementCategory, AchievementType, AdditionalActionState, AreaAction,
    AreaId, AreaType, GameConfig, GameState, Move, OrderCardId, PendingChoice, PlayerId,
    PlayerView, PresidentLocation, TechCardId, TechEffect, TurnPhase, VisiblePlayer,
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

/// Both seats' president declarations AT ONE SIDE of a pair. §5.1 requires
/// both seats; the seat naming is relative to the pair's SUBJECT (the mover
/// whose view §5.3 signs).
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

/// A pair's president declarations, PER SIDE.
///
/// # Why this is sided rather than one declaration for the whole pair
///
/// §5.2 registers pairs whose two sides differ in president SHAPE while the
/// president itself never moves, and a single per-pair declaration cannot say
/// so. **Family 6(d)** is the case that forces it: church and customs are
/// acting-class, so the subject's president stands at the acted area on BOTH
/// sides — but the registration is "the EXACTLY-ONE-assistant full-collapse
/// case", so the PLACED side's area has been drained (that one assistant went
/// to the track) while the UNPLACED side still holds it. One declaration must
/// therefore describe an empty area and an occupied one at once:
/// [`PresidentDecl::OnBoardEmptyArea`] refuses the unplaced side,
/// [`PresidentDecl::OnBoardWithOwnAssistants`] refuses the placed side, and the
/// registration's own arithmetic forbids adding a second pawn to escape.
/// **Family 8** (RECALLED ≻ STRANDED) has the same shape from the other
/// direction: its recalled side is InHand by construction and its stranded side
/// is not.
///
/// # Why symmetric pairs stay terse
///
/// Most of §5.2 is symmetric — thirty-odd registrations whose two sides declare
/// the same thing, and §5.1's DEFAULT is "both InHand". Those spell
/// [`SidedPresidents::BOTH_IN_HAND`] or
/// [`SidedPresidents::symmetric`] and read exactly as they did before the
/// split; only a genuinely asymmetric pair pays the extra words, which is the
/// pair that has something extra to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidedPresidents {
    /// The declared GREATER side's two seats.
    pub greater: SeatPresidents,
    /// The declared LESSER side's two seats.
    pub lesser: SeatPresidents,
}

impl SidedPresidents {
    /// §5.1's DEFAULT on both sides: all four presidents in hand.
    pub const BOTH_IN_HAND: SidedPresidents = SidedPresidents {
        greater: SeatPresidents::BOTH_IN_HAND,
        lesser: SeatPresidents::BOTH_IN_HAND,
    };

    /// The same two-seat declaration on both sides — the shape every symmetric
    /// registration wants, spelled once.
    pub const fn symmetric(seats: SeatPresidents) -> SidedPresidents {
        SidedPresidents { greater: seats, lesser: seats }
    }

    /// One side's declarations.
    pub fn side(self, side: Side) -> SeatPresidents {
        match side {
            Side::Greater => self.greater,
            Side::Lesser => self.lesser,
        }
    }
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
    ///
    /// **Scope: BOTH SEATS, which is over-strict on purpose.** 11(d)'s
    /// constraint is about the SUBJECT's own duty threshold, but the bound is
    /// checked over every seat, so an opponent holding more technologies than
    /// the registration's number refuses too. A family-11 author who wants the
    /// bound on the subject alone must leave this `None` and register the
    /// intent as a note — the strict reading is the safe one here (it can only
    /// refuse a fixture, never admit a wrong one), and it is stated rather than
    /// discovered.
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
    pub presidents: SidedPresidents,
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
    /// **Scope: BOTH SEATS.** The cap is checked over every seat, so a
    /// registered deviation lifts it for the opponent as well as for the
    /// subject — a pair needing a second card only in the subject's hand still
    /// leaves the opponent's hand unchecked. Deliberate (the clause is about
    /// the DECK's reachability, which is a property of the position, not of one
    /// seat), and stated because the bounded re-read reads this doc.
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

impl PairDeclaration {
    /// The registration's own INTERNAL coherence — the clauses that are about
    /// the declaration alone, before any state exists to check it against.
    ///
    /// [`PairFixture::new`] runs this FIRST, so a family task that registers an
    /// incoherent tuple learns at the first fixture rather than at a registry
    /// test it might not have run: a stage it declares but does not price has
    /// no inequality for the solve to use, and a margin at an endpoint it does
    /// not declare is a dangling registration that would silently price a row
    /// no system reads.
    ///
    /// **Only the MARGINED arm can dangle.** DIRECTION-ONLY and WEAK answer at
    /// both endpoints by construction — 1 and 0 are not prices, they are the
    /// two shapes of "no price was registered" — so an unpriced-endpoint check
    /// against them would fire on every single-endpoint direction-only pair,
    /// which is most of §5.2.
    pub fn check_consistency(&self) -> Result<(), FixtureError> {
        let bad = |detail: String| FixtureError::Declaration {
            clause: "registration coherence",
            detail: format!("{}: {detail}", self.id),
        };
        if self.stages.is_empty() {
            return Err(bad("registers no stage endpoint".to_string()));
        }
        for stage in StageEndpoint::ALL {
            let declared = self.stages.contains(&stage);
            let priced = self.margin.margin_at(stage).is_some();
            if declared && !priced {
                return Err(bad(format!(
                    "declares the {} endpoint but registers no margin there",
                    stage.name(),
                )));
            }
            if !declared && priced && matches!(self.margin, MarginClass::Margined { .. }) {
                return Err(bad(format!(
                    "registers a {} margin but does not declare that endpoint",
                    stage.name(),
                )));
            }
        }
        if self.stages.len() > StageEndpoint::ALL.len()
            || (self.stages.len() == 2 && self.stages[0] == self.stages[1])
        {
            return Err(bad("repeats a stage endpoint".to_string()));
        }
        let mut rows: Vec<FeatureId> = self.expected_nonzero.to_vec();
        let listed = rows.len();
        rows.sort_unstable();
        rows.dedup();
        if rows.len() != listed {
            return Err(bad("lists an expected-nonzero row twice".to_string()));
        }
        if !(1..=11).contains(&self.family) {
            return Err(bad(format!("family {} is outside §5.2's 1..=11", self.family)));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Registered construction constants (§5.1's step-2 open parameters, closed)
// ---------------------------------------------------------------------------

/// The setup seed every family-1 and family-11 fixture is built from.
///
/// §5.1 leaves "each fixture's literal state bytes" open to step 2, and a seed
/// is the shortest honest way to close it: the whole base state — board types,
/// port and laboratory contents, the deal — follows from this number plus
/// `Rules::standard()`. ONE seed across both families so the ten fixtures share
/// a base and a re-read has one number to check rather than ten.
///
/// It is not arbitrary in one respect, and the pin says so: at this seed the
/// generated 2P board places the Silk Mill and the Copper Mine NON-ADJACENT,
/// which 1(b)'s registered ΔS4 = −1 requires. The construction asserts that
/// rather than trusting it, so a change to the board generator surfaces as a
/// named refusal instead of as a silently different collateral row.
pub const FIXTURE_SEED: u64 = 0x5A2F_0001;

/// The order card every fixture puts in the SUBJECT's hand as its dealt
/// starting order — printed corner index 22 (US; 4 seafood + 1 copper; 8 VP).
///
/// Registered rather than dealt because three separate rows depend on its
/// requirement vector. It needs NO silk and ONE copper against a seat holding
/// one of each, so `residual_need` is zero for both goods and X3 reads 0
/// wherever a fixture parks a pawn on the Silk Mill or the Copper Mine (1(a),
/// 1(b), 11(a)). Its four-seafood leg keeps it unexecutable, so O3 is 0 on both
/// sides everywhere; and its reward face is pure VP, so O6 is 0 and adding it
/// to a hand cannot queue an icon.
pub const FIXTURE_ORDER_CARD: crate::types::OrderCardId = crate::types::OrderCardId(22);

/// The order card 11(e)'s port take moves into the subject's hand — printed
/// corner index 2 (FR; 1 seafood + 2 tea; 3 VP).
///
/// Registered for the three properties 11(e)'s row list needs: it matches goods
/// the seat holds (so O2 moves), it needs more tea than the seat has (so O3
/// stays 0 and the take does not silently create an executable order), and its
/// reward face is pure VP (so O6 stays 0).
pub const FIXTURE_PORT_CARD: crate::types::OrderCardId = crate::types::OrderCardId(2);

/// Family 11's HIGH-ACCESS area on the shared hand-built layout — the area the
/// greater side acted at. See [`family_11_layout`] for the congruence argument.
pub const FAMILY_11_HIGH_ACCESS_AREA: AreaId = AreaId(7);

/// Family 11's Copper Mine on the shared hand-built layout — the power-dead
/// area the lesser side acted at.
pub const FAMILY_11_COPPER_AREA: AreaId = AreaId(8);

// ---------------------------------------------------------------------------
// §5.2 FAMILY 1 — placement location (5)
// ---------------------------------------------------------------------------

/// The achievement triple every family-1 and family-11 registration draws,
/// except where a recipe's moved good forces a different Category-A card.
///
/// **Tea7 / Orders5 / Commercial4 is a CHOICE, and these are its reasons.**
/// Category A is always dealt (§5.1: "no goods achievement" is not a legal
/// state) and its leg reads RAW GOODS, so the drawn A-card names the one good
/// no registered recipe here may move: Tea. Category B is `Orders5` rather than
/// `Money10` because Money10's leg is algebraically E1 over money ≤ 10 (§2's
/// veto identity) — 11(b) moves yen and would shadow its own carrier — and
/// rather than `Technologies4` because 11(d)'s registration excludes that card
/// by name. Category C is `Commercial4`, a BUILDING count: §5.1's p = 0
/// realization forbids buildings outright, so the leg reads 0 on both sides of
/// every pair here and cannot become collateral.
const DRAWS_TEA_LEG: AchievementDraws = AchievementDraws {
    category_a: AchievementType::Tea7,
    category_b: AchievementType::Orders5,
    category_c: AchievementType::Commercial4,
    goods_moved: &[],
    moves_yen: false,
};

/// The same triple for a recipe that moves SILK — 11(a)'s production tick.
/// §5.1's registered draw constraint is then checkable rather than asserted:
/// `goods_moved` names Silk6 and the constructor refuses a drawn A-card inside
/// that set.
const DRAWS_SILK_MOVED: AchievementDraws =
    AchievementDraws { goods_moved: &[AchievementType::Silk6], ..DRAWS_TEA_LEG };

/// The same triple for a recipe that moves YEN — 11(b)'s bank tick. The
/// constraint this switches on is the Money10 veto, not a goods set.
const DRAWS_YEN_MOVED: AchievementDraws =
    AchievementDraws { moves_yen: true, ..DRAWS_TEA_LEG };

/// Technologies no seat may own in family 1 or family 11.
///
/// Three of the four are the dossier's own fixture recommendation (2026-08-20,
/// the additional-action and residual-money entries): Stock Market's turn-start
/// refill to 2¥ makes a money-0 fixture one advance away from money 2 — 11(d)
/// builds exactly such a seat — while University (1¥ → a warehouse assistant)
/// and Textile Mill (1 silk → 2¥) are in-turn conversions between the two
/// stocks 11(b)/11(c)/11(a) are calibrating. The fourth is structural: §5.1's
/// Telephone exception is the ONE way a hand-off fixture may stand a seat's
/// president on its own assistants, and every pair here declares the ordinary
/// post-Recovery shape instead — excluding the technology is what makes that a
/// checked fact rather than an assumption about the construction.
const NO_CONVERSION_TECHS: &[TechEffect] = &[
    TechEffect::SystemOfStock,
    TechEffect::University,
    TechEffect::SpinningMill,
    TechEffect::Telephone,
];

/// §5.2 1(a) — **PRODUCTIVE ≻ POWER-DEAD**, S1's margin carrier.
pub const PAIR_1A: PairDeclaration = PairDeclaration {
    id: "1(a)",
    name: "PRODUCTIVE ≻ POWER-DEAD",
    family: 1,
    greater_side: "one assistant at the Silk Mill (realizable power 2 → 1 silk)",
    lesser_side: "one assistant at the Copper Mine (realizable power 2 → 0)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::Margined {
        early: Some(1),
        late: Some(14),
        carrier: FeatureId::Candidate(CandidateFeature::ProductionAccess),
    },
    carrier: Some(FeatureId::Candidate(CandidateFeature::ProductionAccess)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistants_per_side", 1),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::ProductionAccess)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: MARGINED (amortized ⅓ VP/good): early 1 / late 14. Both endpoints, \
         HAND-OFF. Presidents: both seats InHand. Collateral: none (S3/S4/S7 equal — \
         single parked pawn both sides; S4-in-hand = 1 both; S7-in-hand = 1 both).",
        "The single parked pawn is what holds the three collateral rows equal: S3 counts \
         ONE occupied area on each side, and both S4 and S7 run their in-hand branch over \
         that same one-area set (local sum 1, direct count 1).",
        "X3 is held at 0 on BOTH sides rather than left to the deal: the lesser side reads \
         min(copper_mine_reward(2) = 0, ·) = 0 by the reward table, and the greater side \
         reads min(1, residual silk need) = 0 because the registered order card needs no \
         silk. Without that card constraint the productive side would carry an unlisted X3 \
         row and 'Collateral: none' would be false.",
        "NO incumbent row moves at all: the two sides differ only in WHICH area holds the \
         one pawn, and `eval`'s sixteen count pawns rather than place them. The pair is \
         therefore a pure candidate-axis separator, which is what makes it S1's carrier.",
    ],
};

/// §5.2 1(b) — **SPREAD ≻ STACK**, S3's carrier, cluster-constrained.
pub const PAIR_1B: PairDeclaration = PairDeclaration {
    id: "1(b)",
    name: "SPREAD ≻ STACK",
    family: 1,
    greater_side: "one assistant at the Silk Mill + one at the Copper Mine (S1 = 1 + 0)",
    lesser_side: "two assistants at the Copper Mine (realizable power 3 → S1 = 1)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::AssistantAreasDistinct)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistants_per_side", 2),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Candidate(CandidateFeature::AssistantAreasDistinct),
        FeatureId::Candidate(CandidateFeature::PresidentLocalAccess),
        FeatureId::Candidate(CandidateFeature::PresidentReachableAreas),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: DIRECTION-ONLY (S3's registered sign prior at equal exported yield — the \
         rules-decidable rule: areas chosen so the summed exported reward at realizable \
         power is EQUAL and only the spread differs). Both endpoints, HAND-OFF, presidents \
         InHand, the two areas non-adjacent. Collateral: S7 +1 (spread side), S4 −1 (stack \
         side has the larger local max) — the pair constrains the {S3, S4, S7} cluster sum, \
         stated.",
        "The equal-yield arithmetic is computed WITH the president +1, per §3.2's \
         `realizable_power`: silk(1+1) = 1 and copper(1+1) = 0 sum to 1 on the spread side, \
         and copper(2+1) = 1 on the stack side.",
        "NON-ADJACENCY is load-bearing for the registered collateral, not decoration: S4's \
         ring term would give the spread side 1 + 1 = 2 if the two areas touched, and the \
         registered ΔS4 = −1 would become 0.",
    ],
};

/// §5.2 1(c) — **NEAR-PRESIDENT ≻ FAR**, S4's carrier (RING MASS).
pub const PAIR_1C: PairDeclaration = PairDeclaration {
    id: "1(c)",
    name: "NEAR-PRESIDENT ≻ FAR",
    family: 1,
    greater_side: "president on-board in the assistants' free-adjacent ring",
    lesser_side: "president on-board far from the same assistants",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::PresidentLocalAccess)),
    presidents: SidedPresidents::symmetric(SeatPresidents {
        subject: PresidentDecl::OnBoardEmptyArea,
        opponent: PresidentDecl::InHand,
    }),
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::HandBuilt,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistant_area", 1),
        ("assistants_stacked", 2),
        ("near_president_area", 0),
        ("far_president_area", 9),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Candidate(CandidateFeature::PresidentLocalAccess),
        FeatureId::Candidate(CandidateFeature::PresidentReachableAreas),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: same assistant multiset on one shared layout; the PRESIDENT differs — \
         on-board at an area whose free-adjacent ring holds the assistants vs on-board far \
         from them (president-moved construction: types identical by definition; the \
         president's own area empty both sides — the post-Recovery hand-off shape). \
         DIRECTION-ONLY, both endpoints, HAND-OFF. Collateral: S7 (different root) — \
         listed; S1/S2/S5/S6 equal (same assistants).",
        "S1/S2/S5/S6 are equal for a stronger reason than 'same types': §3.2's \
         `realizable_power` carries its president term UNCONDITIONALLY, so it does not read \
         `president_location` at all. Moving the president moves S4 and S7 and NOTHING else \
         on the whole axis.",
        "The two assistants are STACKED on one area so that S4 (which sums assistants) \
         separates from S7 (which counts areas): ΔS4 = 2 against ΔS7 = 1, and the pair is \
         S4-dominant. 1(e) is the same construction class with the geometry inverted.",
    ],
};

/// §5.2 1(d) — **CHURCH ACCESS ≻ POWER-DEAD**, S2's carrier.
pub const PAIR_1D: PairDeclaration = PairDeclaration {
    id: "1(d)",
    name: "CHURCH ACCESS ≻ POWER-DEAD",
    family: 1,
    greater_side: "one assistant at the Church, stocks {1 copper, 1 silk} (realizable faith 4)",
    lesser_side: "one assistant at the Copper Mine, same stocks",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::TrackAccess)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistants_per_side", 1),
        ("subject_copper", 1),
        ("subject_silk", 1),
        ("subject_money", 1),
        ("realizable_faith", 4),
        ("empty_eligible_church_faith", 4),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::TrackAccess)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: 1 assistant at Church, stocks {1 copper, 1 silk} making REALIZABLE total \
         faith 4 (the acting-turn reading — at the registered hand-off itself the president \
         is in hand and the actual church action is Skip-only, which is exactly S2's \
         declared optimistic proxy; the fixture builds for the realizable read, stated so no \
         author builds for actual faith) with a registered empty eligible space ≻ 1 at \
         Copper Mine, same stocks. DIRECTION-ONLY, both endpoints, HAND-OFF, presidents \
         InHand. Draws declared (the A-card's good held EQUALLY both sides). Collateral: \
         none.",
        "Faith 4 = `calculate_church_faith(power 2, copper, silk)` = 2 power-steps + 1 per \
         distinct affordable payment type. It fixes THREE stocks, not two: tea and seafood \
         are zeroed and money is held at 1, because each of those would buy another payment \
         type and read faith 5, 6 or 7. The 2-Yen yen leg (D1) is why money 1 and not the \
         dealt 3.",
        "The Church side is the registered one and the Customs side is not, on the dossier's \
         ruling: at zero imports a Customs action is Skip-only at every power, so S2's \
         carrier would be measured on its degenerate half.",
    ],
};

/// §5.2 1(e) — **CONNECTED ≻ DISCONNECTED**, S7's carrier.
pub const PAIR_1E: PairDeclaration = PairDeclaration {
    id: "1(e)",
    name: "CONNECTED ≻ DISCONNECTED",
    family: 1,
    greater_side: "president on-board at the head of an own-assistant chain",
    lesser_side: "president on-board disconnected from the same chain",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::PresidentReachableAreas)),
    presidents: SidedPresidents::symmetric(SeatPresidents {
        subject: PresidentDecl::OnBoardEmptyArea,
        opponent: PresidentDecl::InHand,
    }),
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::HandBuilt,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("chain_assistants", 3),
        ("chain_area_head", 1),
        ("chain_area_middle", 5),
        ("chain_area_tail", 8),
        ("connected_president_area", 0),
        ("disconnected_president_area", 3),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Candidate(CandidateFeature::PresidentReachableAreas),
        FeatureId::Candidate(CandidateFeature::PresidentLocalAccess),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: same multiset, president on-board; own-assistant chain from the president's \
         ring vs the same pawns placed disconnected (president-moved/shared-layout \
         construction). DIRECTION-ONLY, both endpoints, HAND-OFF. Collateral: S4 — listed \
         ({S4, S7} cluster).",
        "NOT PURSUED (an S2-T4 judgment call, flagged for the bounded re-read): the \
         registration's 'the same pawns placed disconnected' also admits a PAWNS-MOVED \
         reading, which this fixture does not build. It is HARDER at 2P, not impossible, and \
         an earlier draft of this note over-claimed. All ten 2P tiles carry DISTINCT types, \
         so a moved pawn always changes the type beneath it and each MOVED pawn needs a \
         partner area of matching contribution — but only the moved ones do, so a chain \
         whose tail stays put needs TWO matching pairs rather than three, and a 2P board has \
         at least two clean ones: Tea↔Fishing (equal 1:1 production reward) and \
         Church↔Customs (equal `realizable_power` under S2, with S1/S5/S6 and the whole X \
         group zero at both). Whether such a construction also lands the registered \
         ΔS7 = +3 shape is untested here. The conclusion is unaffected either way: the \
         registration's own parenthetical names the president-moved/shared-layout \
         construction class, which is 1(c)'s and is what this pair builds.",
        "The geometry is 1(c)'s inverted: three SINGLE pawns strung along a chain, so S7 \
         (which counts areas) moves by 3 while S4 (which sums the ring) moves by 1. The two \
         pairs are therefore linearly independent rows over the {S3, S4, S7} cluster rather \
         than one row written twice.",
    ],
};

// ---------------------------------------------------------------------------
// §5.2 FAMILY 2 — access persistence (3 gates + 1 diagnostic)
// ---------------------------------------------------------------------------

/// **THE PHASE §5.2 CALLS "AreaAction resolved"** — an S2-T5 judgment call the
/// frame deliberately left open, resolved here once for every pair that names
/// it (2(a), 6(d-church), 6(d-customs), 7).
///
/// It is [`TurnPhase::FivePowerBonus`], and the phase machine leaves no second
/// candidate. `Move::ExecuteAreaAction` is the ONLY move that resolves an area
/// action and it hard-assigns `turn_phase = FivePowerBonus`
/// (`game/apply_move.rs:118–121`); a state still resting AT
/// [`TurnPhase::AreaAction`] is one whose action is PENDING, since that is the
/// phase `MovePresident`/`PlacePresident` hand over (`:99`, `:106`). So
/// "resolved" and "AreaAction" name the two sides of one transition, and the
/// resolved side is the later phase.
///
/// Two consequences the pairs below rely on. `current_action_power` is
/// `Some(power)` at this phase — it is written when the president ARRIVES
/// (`:530`, `:591`) and cleared only at the hand-off (`:279`) — so these
/// fixtures carry it, which is exactly what makes them a different evaluation
/// subject from a hand-off. And Recovery has NOT run, so the acted area's
/// pawns are still standing: §5.2's "presence unchanged" is a property of the
/// phase, not something the construction arranges.
const AREA_ACTION_RESOLVED: ViewClass =
    ViewClass::MidTurn { phase: TurnPhase::FivePowerBonus, mercenary: false };

/// **THE PHASE §5.2 CALLS "AdditionalAction"** for 6(a)/(b)/(c) — the second
/// half of the same judgment call.
///
/// It is [`TurnPhase::AdditionalActionFirst`]. `FulfillOrder` and
/// `FulfillAchievement` are offered at AdditionalActionFirst and
/// AdditionalActionSecond alike (`game/legal_moves.rs:13–16, 74–90`) and
/// neither advances the phase (`game/apply_move.rs:148–154`), so the two
/// phases are interchangeable for what these pairs measure — and First is the
/// one whose surrounding state is simplest and matches §5.1's DEFAULT
/// president declaration (both InHand), since no president has been placed yet
/// this turn.
///
/// **The declaration is MID-TURN even though the enum value equals a
/// hand-off's**, which the frame supports deliberately: `check_view_class`
/// runs the structural hand-off clauses only under [`ViewClass::HandOff`], and
/// a state one fulfillment into its own additional-action phase is a mid-turn
/// state whatever its phase field happens to read. What makes it mid-turn is
/// the history, and §5.1 asks the registration — not the bytes — to say so.
const ADDITIONAL_ACTION: ViewClass =
    ViewClass::MidTurn { phase: TurnPhase::AdditionalActionFirst, mercenary: false };

/// The three PRODUCTIVE areas 2(b) and 2(c) deploy onto, and the reason the
/// Copper Mine is not among them.
///
/// §5.2 says "n pawns on productive areas", and at the realizable power one
/// pawn plus the president's +1 buys — power 2 — `copper_mine_reward(2)` is 0
/// (§3.2's own cited sparsity fact). A trio including the Copper Mine would
/// therefore put a third of the "productive" side on an area that produces
/// nothing, which is 1(a)'s POWER-DEAD side wearing the other pair's name. The
/// three chosen areas export 1 + 2 + 2 = 5 goods per activation between them.
const PRODUCTIVE_TRIO: [AreaType; 3] =
    [AreaType::SilkMill, AreaType::TeaPlantation, AreaType::FishingGround];

/// §5.1 line 479: "2(b)'s `n = 3` is REGISTERED, not open". 2(c) is registered
/// at the same n deliberately — the diagnostic exists to be read AGAINST 2(b)
/// ("the print shows how much S-access offsets the hand premium"), and two
/// prints at different n are not comparable.
const FAMILY_2_PAWNS: u8 = 3;

/// 2(d)'s Employment-Agency presence: FOUR own pawns, so
/// `realizable_power(seat, EA)` is `min(5, 4 + 1)` = 5 and
/// `employment_agency_tokens_allowed(5)` = 4.
///
/// # Why the power matters, and the constructibility finding behind it
///
/// §5.2 registers 2(d)'s collateral as "S6 −1 on the hand side (the
/// warehouse-min drops)". S6 is `min(tokens_allowed(power), warehouse basket)`
/// — a MIN, so moving one token out of the warehouse moves S6 only where the
/// BASKET is the binding side. At family 1's ordinary template power of 2,
/// `tokens_allowed` is 1 and a 22-token starting warehouse pins S6 at 1 on
/// both sides: the registered −1 would simply not fire, and the pair would
/// print a collateral row that never moves — §5.1's fixture defect from the
/// other direction.
///
/// So the basket has to be drained to the cap, and the cap is what decides how
/// much draining. At power 5 the cap is 4 and the fixture leaves THREE
/// assistants in the warehouse beside the one shophouse token; at power 2 the
/// cap would be 1 and the warehouse would have to hold nothing but that token.
/// The higher power is the less violent construction and the one registered.
const FAMILY_2D_EA_PAWNS: u8 = 4;

/// The assistants 2(d) leaves in the warehouse on BOTH sides — the three that
/// make the lesser side's basket exactly `tokens_allowed(5)` = 4 once the
/// registered shophouse token is counted, and the greater side's exactly 3.
const FAMILY_2D_WAREHOUSE_ASSISTANTS: u8 = 3;

/// §5.2 2(a) — **CONVERTED ≻ UNCONVERTED**, the activation instant, MARGINED
/// through E2.
pub const PAIR_2A: PairDeclaration = PairDeclaration {
    id: "2(a)",
    name: "CONVERTED ≻ UNCONVERTED",
    family: 2,
    greater_side: "the Silk Mill's two goods taken, both pawns still standing",
    lesser_side: "the same position with the yield not taken",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Late],
    view_class: AREA_ACTION_RESOLVED,
    margin: MarginClass::Margined {
        early: None,
        late: Some(27),
        carrier: FeatureId::Candidate(CandidateFeature::TradeGoodsStock),
    },
    carrier: Some(FeatureId::Candidate(CandidateFeature::TradeGoodsStock)),
    presidents: SidedPresidents::symmetric(SeatPresidents {
        subject: PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: false },
        opponent: PresidentDecl::InHand,
    }),
    draws: DRAWS_SILK_MOVED,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistants_at_the_silk_mill", 2),
        ("realizable_power", 3),
        ("silk_gained", 2),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::TradeGoodsStock)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: (MID-TURN, phase = AreaAction resolved, non-mercenary declared): \
         post-activation (goods +2, presence unchanged) ≻ pre-activation. MARGINED, \
         carrier E2 via the amortized ⅓ VP/good terminal rate: late 27. Late endpoint. \
         Tech exclusions apply. Collateral: E2 +2; O2/legs excluded by draw/hand \
         construction.",
        "§5.1's carrier MAP and §5.2's named carrier are DIFFERENT CLAIMS, and this pair \
         makes only the second. The map names, for each feature, the pair that witnesses \
         its non-implication — one infeasible at `w_f = 0` — and E2's witness there is \
         3(b). What §5.2 names here is the feature this pair's MARGIN is priced through. \
         Registering E2 in the second sense is not a claim to the first, and a re-read \
         diffing the two lists should expect to see E2 twice.",
        "The registered margin is the spec's own arithmetic, re-derived: 2 goods at the \
         amortized ⅓ VP/good rate against the pinned w_vp(late) = 40 is \
         max(1, ceil(2 × 40 / 3)) = 27. The rate is an AVERAGE, not a marginal — \
         `total_trade_goods() / 3` is an integer floor — which is why §5.1 forbids citing \
         a realized terminal delta as the carrier's evidence.",
        "RESOLVED READING (an S2-T5 judgment call): the registration's 'pre-activation' \
         names the UNCONVERTED VALUE, not a state at the earlier phase. A view class is \
         one declaration for the whole pair, so both sides sit at the resolved phase and \
         differ only in the declared way — the yield. Building the lesser side at \
         `TurnPhase::AreaAction` instead would make the two sides different evaluation \
         subjects and measure the phase rather than the conversion.",
        "The GREATER side is engine-driven end to end — the seat's own turn is played \
         from the hand-off through `EndAdditionalPhase`, `PlaceTwoSameArea`, \
         `PlacePresident` and `ExecuteAreaAction`, each through the CHECKED `apply_move` \
         — so its bytes are the bytes a real activation produces, `current_action_power` \
         included. The lesser side is that state with the two silk removed, which is the \
         one declared difference.",
        "SILK is the registered good, and three separate rows depend on it: the drawn \
         Category-A card is Tea7 so the achievement leg does not read it, the registered \
         order card needs no silk so O2 and X3 stay 0 on both sides, and Textile Mill \
         (`SpinningMill`, 1 silk → 2 Yen) is excluded so the stock cannot convert into \
         the money term. A production area has no `Skip`, so the two pawns' presence is \
         identical on both sides by construction.",
    ],
};

/// §5.2 2(b) — **ON-BOARD ≻ WAREHOUSED**, the retrieval economy.
pub const PAIR_2B: PairDeclaration = PairDeclaration {
    id: "2(b)",
    name: "ON-BOARD ≻ WAREHOUSED",
    family: 2,
    greater_side: "three pawns deployed on productive areas",
    lesser_side: "the same three still in the warehouse",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Incumbent(Feature::AssistantsInWarehouse)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("pawns_moved", FAMILY_2_PAWNS as i64),
        ("productive_areas_occupied", PRODUCTIVE_TRIO.len() as i64),
        ("realizable_power_per_area", 2),
        ("exported_goods_per_activation", 5),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::AssistantsInWarehouse),
        FeatureId::Incumbent(Feature::AssistantsOnBoard),
        FeatureId::Candidate(CandidateFeature::ProductionAccess),
        FeatureId::Candidate(CandidateFeature::AssistantAreasDistinct),
        FeatureId::Candidate(CandidateFeature::PresidentLocalAccess),
        FeatureId::Candidate(CandidateFeature::PresidentReachableAreas),
        FeatureId::Candidate(CandidateFeature::MatchedProductionAccess),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: n pawns on productive areas ≻ the same n still in the warehouse, totals \
         equal elsewhere. DIRECTION-ONLY, EARLY, HAND-OFF. Carrier: V2's warehouse weight \
         (−10) — the S access terms ride as collateral, listed.",
        "The carrier is an INCUMBENT, which is why the declaration's carrier field is not \
         a candidate here: the dossier's own re-reading of this pair (2026-08-20) is that \
         it measures the EA-retrieval gradient rather than the recovery transition, and \
         §5.2 kept the pair with V2's warehouse weight named as the standing carrier. The \
         board-vs-HAND comparison the dossier recommends alongside it is registered \
         separately, as the 2(c) diagnostic.",
        "n = 3 is REGISTERED, not open (§5.1 line 479), and the three areas are the Silk \
         Mill, the Tea Plantation and the Fishing Ground — see PRODUCTIVE_TRIO for why \
         the Copper Mine is excluded. The hand is untouched on both sides, so the whole \
         incumbent movement is the two pawn counters the move between warehouse and board \
         swaps.",
        "X3 is a LISTED collateral here rather than a zero held by construction, unlike \
         family 1's pairs: the Fishing Ground exports seafood and the registered order \
         card needs four of them, so the deployed side reads min(2, 4) = 2 while the \
         warehoused side reads 0. Naming it is the alternative to bending the trio away \
         from a genuinely productive area.",
    ],
};

/// §5.2 2(d) — **READY ≻ WAREHOUSED**, E3's carrier.
pub const PAIR_2D: PairDeclaration = PairDeclaration {
    id: "2(d)",
    name: "READY ≻ WAREHOUSED",
    family: 2,
    greater_side: "the shophouse token retrieved into hand",
    lesser_side: "the same token still in the warehouse",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::BuildingTokensReady)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistants_at_the_employment_agency", FAMILY_2D_EA_PAWNS as i64),
        ("realizable_power_at_the_ea", 5),
        ("employment_agency_tokens_allowed", 4),
        ("warehouse_assistants", FAMILY_2D_WAREHOUSE_ASSISTANTS as i64),
        ("warehouse_basket_greater", 3),
        ("warehouse_basket_lesser", 4),
        ("shophouses_in_hand_greater", 8),
        ("shophouses_in_hand_lesser", 7),
        ("trading_houses_in_hand", 4),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Candidate(CandidateFeature::BuildingTokensReady),
        FeatureId::Candidate(CandidateFeature::EmploymentAgencyAccess),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: one shophouse token in HAND vs the same token still in the WAREHOUSE, all \
         else equal (the EA cost already paid on both sides' histories is not a state \
         difference — the fixture sets equal money). DIRECTION-ONLY, EARLY, HAND-OFF, \
         presidents InHand. Collateral: S6 −1 on the hand side (the warehouse-min drops) \
         — listed, the {E3, S6} cluster's declared tension.",
        "CONSTRUCTIBILITY FINDING, resolved and reported (S2-T5): the registered ΔS6 = −1 \
         is realizable only where the warehouse BASKET binds S6's min, not the power cap \
         — see FAMILY_2D_EA_PAWNS for the arithmetic and for why the fixture buys the \
         binding basket with power 5 rather than by emptying the warehouse outright. The \
         registration is honoured as written; what step 2 adds is the state that makes it \
         true.",
        "The trading houses sit entirely in hand on BOTH sides for the same reason the \
         warehouse assistants are held at three: the basket is a SUM over all three token \
         classes, so four trading houses left in the warehouse would carry it past the \
         cap and pin S6 at 4 on both sides. They are equal across the pair, so E3's \
         +1 is the shophouse token's alone.",
        "No incumbent row moves at all. V2 prices shophouses only once they are BUILT, \
         and both sides have built nothing (§5.1's p = 0 realization forbids it), so the \
         retrieval this pair is about is invisible to all sixteen — which is precisely \
         why §3.2 rules E3 in.",
    ],
};

/// §5.2 2(c) — **BOARD-vs-HAND**, the recorded diagnostic.
pub const PAIR_2C: PairDeclaration = PairDeclaration {
    id: "2(c)",
    name: "BOARD-vs-HAND",
    family: 2,
    greater_side: "three pawns deployed on productive areas",
    lesser_side: "the same three still in hand",
    role: PairRole::Diagnostic,
    stages: &[StageEndpoint::Early],
    view_class: ViewClass::HandOff,
    margin: MarginClass::Weak,
    carrier: None,
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("pawns_moved", FAMILY_2_PAWNS as i64),
        ("productive_areas_occupied", PRODUCTIVE_TRIO.len() as i64),
        ("realizable_power_per_area", 2),
        ("exported_goods_per_activation", 5),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::AssistantsOnBoard),
        FeatureId::Incumbent(Feature::AssistantsInHandExcess),
        FeatureId::Candidate(CandidateFeature::ProductionAccess),
        FeatureId::Candidate(CandidateFeature::AssistantAreasDistinct),
        FeatureId::Candidate(CandidateFeature::PresidentLocalAccess),
        FeatureId::Candidate(CandidateFeature::PresidentReachableAreas),
        FeatureId::Candidate(CandidateFeature::MatchedProductionAccess),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: RECORDED DIAGNOSTIC, never a gate: n pawns at productive areas vs n in \
         hand at equal totals. The truth direction is position-dependent and V2's hand \
         ordering (`core > on_board > excess`, standing solve constraints) is the \
         compatibility bar; the diagnostic prints how much S-access offsets the hand \
         premium — the tension a sighted S group must price, measured rather than \
         pre-judged.",
        "THE MARGIN CLASS IS INERT AND THE SIDE NAMING IS A CONVENTION, both stated \
         because neither is a claim. §5.1's pair→gate mapping puts only \
         `PairRole::Constraint` rows in Part B's constraint system and Part A's blocking \
         set, so nothing reads a diagnostic's margin; WEAK is the frame's weakest of the \
         three and is registered for that reason alone. Naming the board side GREATER \
         fixes the print's sign — §5.1 defines Δv = greater − lesser — and asserts no \
         direction, which is the registration's own point.",
        "It is 2(b)'s construction with the pawns coming from HAND rather than from the \
         warehouse, at the same registered n and the same three areas, so the two prints \
         differ in exactly the comparison they are about. The incumbent side of the \
         difference is the whole tension: V2 prices hand-core above on-board (30 vs 6 \
         early) and hand-excess below it (5 vs 6), and at a hand of eight the reserve is \
         full on both sides, so the deployed pawns come out of `AssistantsInHandExcess`.",
    ],
};

// ---------------------------------------------------------------------------
// §5.2 FAMILY 3 — economics (4)
// ---------------------------------------------------------------------------

/// The achievement triple 3(b) draws — §5.2's own worked example, verbatim:
/// "+1 of a good whose Category-A card is NOT the drawn one (registered draw,
/// e.g. vary tea with Copper5 drawn)".
///
/// `goods_moved` names Tea7 so §5.1's registered draw constraint is CHECKED
/// rather than asserted: `check_draws` refuses a drawn A-card inside the moved
/// set, and Copper5 is outside it. Category B stays `Orders5` — its leg counts
/// completed orders, and 3(b)'s hand-emptying fulfillment moves that leg by one
/// on BOTH sides, so it differences to zero (family 6 excludes Orders5 for the
/// opposite reason: there the fulfillment IS the axis).
const DRAWS_TEA_MOVED: AchievementDraws = AchievementDraws {
    category_a: AchievementType::Copper5,
    category_b: AchievementType::Orders5,
    category_c: AchievementType::Commercial4,
    goods_moved: &[AchievementType::Tea7],
    moves_yen: false,
};

/// 3(a)'s two registered money levels.
///
/// **The EVEN-MONEY REALIZATION NOTE §5.2 asks for, made concrete.** The
/// carrier's amortized rate is ½ VP per yen, but the terminal conversion is
/// `money / 2` as an INTEGER FLOOR (scoring.rs:341–357), so 2 → 3 realizes
/// `floor(2/2) = 1` against `floor(3/2) = 1` — a terminal delta of ZERO. The
/// pair is registered at exactly that variant deliberately: §5.1 forbids citing
/// a realized terminal delta as the carrier's evidence ("a single unit realizes
/// 0-or-1, never ½"), and a fixture sitting on the odd step would let a reader
/// mistake the realization for the argument.
const FAMILY_3A_GREATER_MONEY: u32 = 3;
/// 3(a)'s lesser side — see [`FAMILY_3A_GREATER_MONEY`].
const FAMILY_3A_LESSER_MONEY: u32 = 2;

/// 3(b)'s two registered tea levels, after the hand-emptying fulfillment has
/// consumed the dealt stocks.
///
/// One apart, and neither at zero: the pair is "MORE GOODS ≻ FEWER", and a
/// lesser side holding nothing at all would additionally be the boundary case
/// where every good-reading term collapses for a second reason.
const FAMILY_3B_GREATER_TEA: u32 = 2;
/// 3(b)'s lesser side — see [`FAMILY_3B_GREATER_TEA`].
const FAMILY_3B_LESSER_TEA: u32 = 1;

/// The stocks 3(c) and 3(d) register EQUALLY on both sides — §5.2's "stocks
/// equal" written down rather than left to the deal.
///
/// The dealt one-of-each is what setup gives (`types/player.rs:245–248`), and
/// writing it is what makes the registration checkable: neither pair's carrier
/// reads goods, but X3 and the Category-A leg do, and "whatever the seed gave"
/// would be four unregistered facts underneath a pair whose collateral is
/// registered as NONE.
const FAMILY_3_STOCKS: [u32; 4] = [1, 1, 1, 1];

/// §5.2 3(a) — **MORE YEN ≻ FEWER**, E1's margin carrier.
pub const PAIR_3A: PairDeclaration = PairDeclaration {
    id: "3(a)",
    name: "MORE YEN ≻ FEWER",
    family: 3,
    greater_side: "three yen",
    lesser_side: "two yen, and nothing else different",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::Margined {
        early: Some(1),
        late: Some(20),
        carrier: FeatureId::Candidate(CandidateFeature::MoneyStock),
    },
    carrier: Some(FeatureId::Candidate(CandidateFeature::MoneyStock)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_YEN_MOVED,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("greater_money", FAMILY_3A_GREATER_MONEY as i64),
        ("lesser_money", FAMILY_3A_LESSER_MONEY as i64),
        ("assistants_on_board", 0),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::MoneyStock)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: +1 yen, all else identical. MARGINED (amortized ½ VP/yen): early 1 / late 20. \
         Both endpoints, HAND-OFF, presidents InHand. Constraints: the drawn Category-B card \
         is NOT Money10 (else the leg shadows E1 — §2's veto identity); tech exclusions; the \
         realization note (even-money variant recorded). Collateral: none.",
        "THE REGISTERED EVEN-MONEY VARIANT: 2 → 3 realizes floor(2/2) = 1 against \
         floor(3/2) = 1, a terminal delta of ZERO. That is deliberate — §5.1 states that the \
         amortized ½ VP/yen carrier may never be evidenced by a realized terminal delta, and \
         a pair sitting on the odd step would invite exactly that reading.",
        "'ALL ELSE IDENTICAL' is built as literally as it reads: NO assistant stands on the \
         board on either side. Money is not one of `eval`'s sixteen, and with no presence \
         anywhere every S term and every X term reads its own structural zero (X1 and X2 are \
         gated on own presence at the lab and the port, which is what stops money moving X1 \
         through its `floor(money/2) + imports` purse). E1 is the sole mover on the whole \
         thirty-four-row axis.",
        "The Money10 veto is CHECKED, not trusted: `moves_yen` is set on the registered \
         draws, and `check_draws` refuses the Money10 Category-B card outright.",
    ],
};

/// §5.2 3(b) — **MORE GOODS ≻ FEWER**, E2's margin carrier.
pub const PAIR_3B: PairDeclaration = PairDeclaration {
    id: "3(b)",
    name: "MORE GOODS ≻ FEWER",
    family: 3,
    greater_side: "two tea",
    lesser_side: "one tea, and nothing else different",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::Margined {
        early: Some(1),
        late: Some(14),
        carrier: FeatureId::Candidate(CandidateFeature::TradeGoodsStock),
    },
    carrier: Some(FeatureId::Candidate(CandidateFeature::TradeGoodsStock)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_MOVED,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("greater_tea", FAMILY_3B_GREATER_TEA as i64),
        ("lesser_tea", FAMILY_3B_LESSER_TEA as i64),
        ("other_goods", 0),
        ("assistants_on_board", 0),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
        ("hand_emptied_by_fulfilling", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::TradeGoodsStock)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: +1 of a good whose Category-A card is NOT the drawn one (registered draw, e.g. \
         vary tea with Copper5 drawn — \"no goods achievement live\" is not a legal state). \
         MARGINED: early 1 / late 14. Both endpoints, HAND-OFF. Tech exclusions. Collateral: \
         O2 if a held order requires the good — hand constructed empty.",
        "HOW AN EMPTY HAND IS CONSTRUCTED, and why the clause is honourable as written (the \
         S2-T6 resolution of §5.1's open byte). At p = 0 the hand cap admits exactly the \
         dealt starting order, so an EMPTY hand means that card has been FULFILLED — \
         invariant I11 wants the recorded `starting_order` in the hand OR in the completed \
         pile, and the completed pile is the second arm. The fulfillment moves none of \
         `clock_ppm`'s four clocks (it moves no deck card, no track pawn and no building), so \
         p = 0 survives it, and the LATE side needs no different device: the same fulfillment \
         runs on the deck-saturated base. The construction is 6(a)'s greater side exactly, \
         which S2-T5 already admitted through this frame.",
        "THE FULFILLED CARD'S OWN EFFECTS, walked. Order 22 pays 8 VP and nothing else — no \
         money, no imports, no goods back, and no movement or warehouse icon face, so no \
         choice is queued and the state stays a structural hand-off. Both sides fulfil it, so \
         VP, the completed pile, `CountryBonusNow` (one distinct country scores nothing) and \
         the Orders5 leg all difference to zero. The post-fulfillment American icon count is \
         1, ODD, so `check_mercenary_eligibility`'s sweep awards nothing.",
        "The registered stocks are written AFTER the fulfillment has consumed the dealt ones, \
         which is why 'other goods 0' is a registration rather than an accident: the \
         requirements are exactly what the seat held, so the fulfillment leaves all four \
         stocks at zero and the pair writes tea back on top.",
        "O2 is held at zero on BOTH sides by the empty hand rather than by a card that happens \
         not to need tea — the registration's own device, kept even though the dealt order \
         card would also have sufficed (it needs copper and seafood, never tea). Building the \
         weaker version would have made 'Collateral: O2 …' true by luck.",
    ],
};

/// §5.2 3(c) — **BANK ACCESS ≻ POWER-DEAD**, S5's margin carrier.
pub const PAIR_3C: PairDeclaration = PairDeclaration {
    id: "3(c)",
    name: "BANK ACCESS ≻ POWER-DEAD",
    family: 3,
    greater_side: "one assistant at the Bank (realizable power 2 → 2 yen)",
    lesser_side: "one assistant at the Copper Mine (realizable power 2 → 0)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::Margined {
        early: Some(1),
        late: Some(40),
        carrier: FeatureId::Candidate(CandidateFeature::BankAccess),
    },
    carrier: Some(FeatureId::Candidate(CandidateFeature::BankAccess)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistants_per_side", 1),
        ("subject_copper", FAMILY_3_STOCKS[0] as i64),
        ("subject_silk", FAMILY_3_STOCKS[1] as i64),
        ("subject_tea", FAMILY_3_STOCKS[2] as i64),
        ("subject_seafood", FAMILY_3_STOCKS[3] as i64),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::BankAccess)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: 1 assistant at Bank (power 2 → 2¥) ≻ 1 at Copper Mine. MARGINED (amortized, \
         one activation's 2¥ ≈ 1 VP): early 1 / late 40. Both endpoints, HAND-OFF, presidents \
         InHand, stocks equal. Collateral: none.",
        "It is 1(a)'s construction with the Bank in the Silk Mill's place, and the collateral \
         is none for the same three reasons: ONE parked pawn per side holds S3 at 1, and both \
         S4 and S7 run their in-hand branch over that same one-area set (local sum 1, direct \
         count 1).",
        "The two rewards are read off the engine's own tables, not restated: \
         `bank_reward(2) = 2` (commercial.rs:8, 1 yen per power) against \
         `copper_mine_reward(2) = 0` (production.rs:60, §3.2's cited sparsity fact). The \
         Bank is not a production area, so S1 is 0 on the greater side as well as on the \
         lesser — the pair moves S5 and NOTHING else on the axis.",
        "The margin's REALIZATION-PROBABILITY-1 assumption is §5.1's, named there as a \
         candidate margin cause for the Part-B core inspection: late 40 prices forty \
         activations' worth of access at 1 VP each, and the pair does not claim the \
         activations happen.",
    ],
};

/// §5.2 3(d) — **EA ACCESS ≻ POWER-DEAD**, S6's carrier.
pub const PAIR_3D: PairDeclaration = PairDeclaration {
    id: "3(d)",
    name: "EA ACCESS ≻ POWER-DEAD",
    family: 3,
    greater_side: "one assistant at the Employment Agency (realizable power 2 → 1 takeable token)",
    lesser_side: "one assistant at the Copper Mine (realizable power 2 → 0)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::EmploymentAgencyAccess)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistants_per_side", 1),
        ("warehouse_assistants", 12),
        ("warehouse_shophouses", 6),
        ("warehouse_trading_houses", 4),
        ("subject_copper", FAMILY_3_STOCKS[0] as i64),
        ("subject_silk", FAMILY_3_STOCKS[1] as i64),
        ("subject_tea", FAMILY_3_STOCKS[2] as i64),
        ("subject_seafood", FAMILY_3_STOCKS[3] as i64),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::EmploymentAgencyAccess)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: 1 assistant at EA (power 2 → 1 takeable token; warehouse non-empty declared, \
         matching S6's min-gate) ≻ 1 at Copper Mine. DIRECTION-ONLY, EARLY, HAND-OFF, \
         presidents InHand. Collateral: none.",
        "THE WAREHOUSE-NON-EMPTY DECLARATION is the untouched setup warehouse, and the three \
         parameters say so rather than leaving it implied: 12 assistants, 6 shophouse tokens \
         and 4 trading-house tokens (`types/player.rs`'s three constructors). S6 is a MIN, so \
         the declaration is what stops the term reading its structural zero for the wrong \
         reason — with an empty basket both sides would read 0 and the pair would separate \
         nothing.",
        "`employment_agency_tokens_allowed(2) = 1` (commercial.rs:13) is the binding side of \
         that min at this power, so S6 is 1 on the greater side and 0 on the lesser. Nothing \
         is TAKEN here — 3(d) is a pure ACCESS pair, and the take is 11(c)'s.",
        "EARLY only, transcribed from §5.2. The stage column is the registration's, not a \
         construction limit: the same two states are buildable on the deck-saturated base, \
         and registering the late endpoint would add a solve row §5.2 did not register.",
    ],
};

// ---------------------------------------------------------------------------
// §5.2 FAMILY 6 — post-action persistence (5): the C2 controls
// ---------------------------------------------------------------------------

/// The achievement triple 6(a) and 6(c) draw — the fulfillment pairs.
///
/// Category A is **Copper5**: both registered order cards' requirements are
/// drawn from silk, tea and seafood, and §5.1's registered draw constraint
/// wants the always-drawn A-leg's good OUTSIDE the moved set. Category B is
/// **Technologies4** because §5.2 registers Orders5 as NOT drawn for these
/// pairs — its leg counts completed orders and a fulfillment moves it by one —
/// and because Tokens5 would move on any mercenary award. Category C is
/// **Commercial4**, a building count, which §5.1's p = 0 realization pins at 0
/// on both sides.
const DRAWS_FULFILLMENT: AchievementDraws = AchievementDraws {
    category_a: AchievementType::Copper5,
    category_b: AchievementType::Technologies4,
    category_c: AchievementType::Commercial4,
    goods_moved: &[AchievementType::Silk6, AchievementType::Tea7, AchievementType::Seafood7],
    moves_yen: false,
};

/// 6(b)'s triple. **Money10 is the registered Category-B card** (§5.2: "the
/// clean E1-untouched interaction case"), and §5.1's blanket B ≠ Money10
/// constraint does not bind: it binds pairs whose recipe moves goods or yen,
/// and an achievement claim moves neither (dossier: "NO goods, money, tokens,
/// orders or techs are consumed"). `moves_yen: false` is what says so to the
/// constructor.
const DRAWS_MONEY10: AchievementDraws = AchievementDraws {
    category_a: AchievementType::Tea7,
    category_b: AchievementType::Money10,
    category_c: AchievementType::Commercial4,
    goods_moved: &[],
    moves_yen: false,
};

/// 6(d-church)'s triple: the registered church payment spends one copper and
/// one silk, so the drawn A-card may be neither.
const DRAWS_CHURCH_PAYMENT: AchievementDraws = AchievementDraws {
    goods_moved: &[AchievementType::Copper5, AchievementType::Silk6],
    ..DRAWS_TEA_LEG
};

/// **6(a)'s REGISTERED ORDER CARD** — printed corner index 4 (German; 2 silk +
/// 1 tea + 1 seafood; 5 VP; ONE warehouse icon).
///
/// Every clause of §5.2's collateral list picked it. The warehouse icon is
/// what makes "O6 retires" a row that actually moves — only 7 of the 36
/// printed orders carry any icon at all (5 warehouse, 2 movement), so an
/// icon-free card would leave O6 at 0 on both sides and the registration would
/// name a collateral that never fires. Its reward face is pure VP: no money
/// and no imports, so E1 and `ImportedGoods` stay still and the "E-group
/// unmoved but for E2" reading holds. Its requirements avoid copper, which is
/// what lets Copper5 be the drawn A-card.
pub const FIXTURE_FULFILLED_CARD: OrderCardId = OrderCardId(4);

/// **6(c)'s REGISTERED ORDER CARD** — printed corner index 11 (Dutch; 1 silk +
/// 1 tea + 1 seafood; 5 VP; icon-free).
///
/// Dutch because it is the country the four pre-completed orders do not carry,
/// so fulfilling it takes the seat from four distinct countries to five — the
/// step at which `country_set_bonus` moves 8 → 12 AND O5's best attainable
/// marginal collapses 4 → 0, which is the only step where §5.2's registered
/// "O5 −, `CountryBonusNow` +" both hold. Icon-free, so the fulfillment queues
/// nothing: 6(a) is where the queued-icon case is registered.
pub const FIXTURE_SET_CARD: OrderCardId = OrderCardId(11);

/// **6(c)'s FOUR PRE-COMPLETED ORDERS**, one per non-Dutch country — English,
/// French, German, American (printed corner indices 32, 5, 9, 12).
///
/// One icon each, so `check_mercenary_eligibility`'s sweep reads an ODD count
/// in every country and awards nothing on either side (the engine awards at
/// EVEN post-counts). The faces are otherwise inert: a completed card's reward
/// was paid when it was completed, and O6 reads HELD orders, never completed
/// ones.
///
/// The English slot is card 32 rather than card 3, and the reason is a
/// CONSTRUCTIBILITY COLLISION found at S2-T5 rather than a preference: at
/// [`FIXTURE_SEED`] card 3 is the OPPONENT's dealt starting order, and
/// [`complete_order_from_the_box`] refuses a card another seat already holds
/// rather than quietly conjuring a second copy. The refusal named the card and
/// the seat, which is what made the swap a one-line answer.
pub const FAMILY_6C_COMPLETED: [OrderCardId; 4] =
    [OrderCardId(32), OrderCardId(5), OrderCardId(9), OrderCardId(12)];

/// **6(d)'s REGISTERED ORDER CARD** — printed corner index 26 (French; 3 tea +
/// 1 seafood; 1 VP + 2 Yen; icon-free).
///
/// Registered for what it does NOT need. The church side pays copper and silk
/// and the customs side pays an imported good, and this card's requirements
/// name neither copper nor silk — so O2 and O3 read 0 on both sides of both
/// pairs and the payment cannot move an order row. Icon-free keeps O6 at 0.
pub const FIXTURE_TRACK_CARD: OrderCardId = OrderCardId(26);

/// The Church space 6(d-church) takes, and the majority arithmetic it buys.
///
/// Total faith is `min(power, 5) + 1 per distinct payment type` = 2 + 2 = 4,
/// and eligible spaces are the EMPTY ones with printed faith in `4..=4` — so
/// faith-4 is the only one and the placement resolves INLINE rather than
/// queueing a `ChurchSpace` choice. Against the 2P dummy at faith-7 the
/// majority key is (count desc, RIGHTMOST occupied index desc): one pawn each,
/// and index 0 loses to index 3, so the dummy takes first place (consumed,
/// awarded to nobody) and the subject takes second — 3 VP of `ChurchBonusNow`
/// where the unplaced side has none.
const CHURCH_FAITH_SPACE: u8 = 4;

/// The Customs space 6(d-customs) takes.
///
/// One imported good is the whole payable range at power 2
/// (`customs_max_imported_goods(2) = 1`), and cost-1 maps to spaces {1, 2} —
/// TWO eligible spaces, so the engine queues `CustomsSpace { cost: 1 }` and
/// the fixture answers it with this registered space. Space 1 is the
/// registered answer because its rider is TWO warehouse takes, which §5.2
/// registers as declined ("warehouse takes at 1/3 DECLINED in the registered
/// fixture"); space 2's +2 Yen rider would instead move the money term, and
/// the registration names that rider only to describe the table. Space 1 pays
/// 5 VP and, at index 0 against the dummy at index 3, takes second place for 4
/// VP of `CustomsBonusNow` on the same rightmost-tie rule as the church.
const CUSTOMS_SPACE: u8 = 1;

/// The movement riders faith-4 queues, all DECLINED: `ChurchTrack`'s rider
/// table is 1,1,2,2,2,2,3,3 for faith 4..11, so the registered space queues
/// exactly one.
const CHURCH_RIDERS_DECLINED: usize = 1;

/// The warehouse takes customs space 1 queues, all DECLINED: its rider is
/// `(2 warehouse takes, 0 yen)`.
const CUSTOMS_RIDERS_DECLINED: usize = 2;

/// The imported goods 6(d-customs) pays — the whole payable range at power 2,
/// since `customs_max_imported_goods(2)` is 1 and the payment must match a
/// printed space cost EXACTLY.
const CUSTOMS_IMPORTS_PAID: u8 = 1;

/// §5.2 6(a) — **FULFILLED ≻ UNFULFILLED**.
pub const PAIR_6A: PairDeclaration = PairDeclaration {
    id: "6(a)",
    name: "FULFILLED ≻ UNFULFILLED",
    family: 6,
    greater_side: "the registered order fulfilled, its warehouse icon still queued",
    lesser_side: "the same order in hand, its requirements still in stock",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ADDITIONAL_ACTION,
    margin: MarginClass::DirectionOnly,
    carrier: None,
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_FULFILLMENT,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("fulfilled_order_card", FIXTURE_FULFILLED_CARD.0 as i64),
        ("order_face_vp", 5),
        ("goods_paid", 4),
        ("warehouse_icons_queued", 1),
        ("german_icons_before", 0),
        ("german_icons_after", 1),
        ("german_mercenary_supply", 2),
        ("mercenaries_awarded", 0),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::VictoryPoints),
        FeatureId::Incumbent(Feature::HandSize),
        FeatureId::Candidate(CandidateFeature::TradeGoodsStock),
        FeatureId::Candidate(CandidateFeature::HeldOrderValue),
        FeatureId::Candidate(CandidateFeature::OrderMatchedGoods),
        FeatureId::Candidate(CandidateFeature::ExecutableOrders),
        FeatureId::Candidate(CandidateFeature::FlagIconMarginal),
        FeatureId::Candidate(CandidateFeature::HeldOrderIcons),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: (phase = AdditionalAction): both endpoints (early via the dealt starting \
         order — no clock moves at fulfillment). The card id REGISTERED; the seat's \
         pre-fulfillment icon PARITY for the card's country declared (ODD fires — the \
         engine awards at even POST-count, pooling completed orders AND owned \
         technologies) with the mercenary supply state (or an inert-parity card \
         registered with `MercenariesTotal` declared unmoved).",
        "THE INERT PARITY IS THE REGISTERED ONE — §5.2's own sanctioned alternative. The \
         seat holds no completed orders and no technologies, so its German icon count is \
         0 BEFORE and 1 after; `check_mercenary_eligibility` awards at each EVEN count, \
         so an odd post-count awards nothing and `MercenariesTotal` is declared unmoved \
         on both sides. The German supply is registered anyway, at its full 2, so the \
         other arm of §5.2's clause is closed too: the award does not fire, and it is not \
         an exhausted supply that stopped it. Firing it instead would need a \
         pre-completed German order, which would also move `CountryBonusNow` and make \
         this pair 6(c).",
        "`CountryBonusNow` is NOT listed, and the omission is the registration's, not an \
         oversight: §5.2 lists the row 'on a set step', and a FIRST completed order takes \
         the seat from zero distinct countries to one, where `country_set_bonus` still \
         reads 0 — it needs two. Keeping this pair off a set step is what makes 6(a) and \
         6(c) two independent rows rather than one written twice, and the set step itself \
         is 6(c)'s whole subject.",
        "THE QUEUED CHOICE IS CARRIED, NOT RESOLVED (an S2-T5 judgment call). The card's \
         warehouse icon pushes one `PendingChoice::WarehouseTake`, and the greater side is \
         the state the engine leaves behind at the instant of fulfillment — queue and \
         all. Resolving it either way would be a second decision the registration does \
         not make, and no row on the thirty-four-wide axis reads `pending_choices`. The \
         queue head keeps a live resolution set (assistants are free), so the state is one \
         `check_invariants` accepts.",
        "The card leaves hand and is pushed to `completed_orders`, and BOTH piles are \
         located sets O1's pool reads — so the pool floor is identical across the pair and \
         the differenced ΔO1 is the card's own value alone. O5 moves UP rather than down \
         here (0 → 2: a seat with one country's icon has a positive marginal for any \
         second country, a seat with none has none), which is what §5.2's 'recompute' \
         covers; the DOWNWARD case is 6(c)'s.",
        "`AchievementProgressSum` is registered as UNMOVED, and the draws are what make \
         it so: Copper5 reads copper and the card's requirements are silk, tea and \
         seafood; Technologies4 reads a technology count both seats hold at zero; \
         Commercial4 reads buildings. §5.2 lists the row 'where the drawn legs read the \
         moved quantities' — here, by registration, they do not.",
    ],
};

/// §5.2 6(b) — **CLAIMED ≻ UNCLAIMED**.
pub const PAIR_6B: PairDeclaration = PairDeclaration {
    id: "6(b)",
    name: "CLAIMED ≻ UNCLAIMED",
    family: 6,
    greater_side: "Money10 claimed, for its first-place 9 VP",
    lesser_side: "the same ten yen, the card unclaimed",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ADDITIONAL_ACTION,
    margin: MarginClass::DirectionOnly,
    carrier: None,
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_MONEY10,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("subject_money", 10),
        ("first_place_vp", 9),
        ("progress_leg_dropped", 10),
        ("hand_assistants_before", 8),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::VictoryPoints),
        FeatureId::Incumbent(Feature::AchievementProgressSum),
        FeatureId::Incumbent(Feature::AssistantsInHandExcess),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: (achievement; phase = AdditionalAction): both endpoints. Card = Money10 (E1 \
         untouched by the claim — the clean interaction case); collateral: VP +, \
         T3-filtered progress −, `AssistantsInHandCore/Excess` −1 (the claim spends a hand \
         assistant; the two differ by an order of magnitude in V2 — declared), E/O groups \
         unmoved (registered as an empty list).",
        "THE EXCESS BUCKET IS THE ONE THAT PAYS, declared rather than left to the reader: \
         the seat claims out of a hand of eight, well past V2's two-pawn core reserve, so \
         `AssistantsInHandCore` is 2 on both sides and the whole −1 lands in \
         `AssistantsInHandExcess` (weights 5 early / 2 late, against core's 30 / 20). A \
         fixture claiming from a hand of two would register the other row and would be a \
         different constraint.",
        "The progress row is exactly −10: Money10's leg is min(money, 10) = 10 at the \
         registered money, and the shipped T3 post-claim filter drops a claimed card's \
         leg ENTIRELY for the claiming seat. The other two legs read a tea stock and a \
         building count, both zero on both sides.",
        "§5.2's registered EMPTY collateral list for the E and O groups is honoured \
         literally — the claim consumes no money, goods, tokens, orders or technologies, \
         so this is the one pair here whose whole delta is incumbent. That is not a \
         weakness: it is what makes it a compatibility constraint on the achievement and \
         hand weights rather than on anything V4 is adding.",
    ],
};

/// §5.2 6(c) — **SET-COMPLETED ≻ PENDING**.
pub const PAIR_6C: PairDeclaration = PairDeclaration {
    id: "6(c)",
    name: "SET-COMPLETED ≻ PENDING",
    family: 6,
    greater_side: "the Dutch order fulfilled — the fifth country, the set closed",
    lesser_side: "the same order in hand, four countries completed",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ADDITIONAL_ACTION,
    margin: MarginClass::DirectionOnly,
    carrier: None,
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_FULFILLMENT,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("fulfilled_order_card", FIXTURE_SET_CARD.0 as i64),
        ("order_face_vp", 5),
        ("goods_paid", 3),
        ("completed_orders_before", 4),
        ("distinct_countries_before", 4),
        ("distinct_countries_after", 5),
        ("country_set_bonus_before", 8),
        ("country_set_bonus_after", 12),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::VictoryPoints),
        FeatureId::Incumbent(Feature::CountryBonusNow),
        FeatureId::Incumbent(Feature::HandSize),
        FeatureId::Candidate(CandidateFeature::TradeGoodsStock),
        FeatureId::Candidate(CandidateFeature::HeldOrderValue),
        FeatureId::Candidate(CandidateFeature::OrderMatchedGoods),
        FeatureId::Candidate(CandidateFeature::ExecutableOrders),
        FeatureId::Candidate(CandidateFeature::FlagIconMarginal),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: (flag realization via fulfillment; phase = AdditionalAction): both \
         endpoints. Collateral: O5 −, `CountryBonusNow` +, plus 6(a)'s rows for the \
         vehicle fulfillment.",
        "THE FIFTH COUNTRY IS THE ONLY STEP THAT SATISFIES BOTH REGISTERED ROWS, and the \
         2/4/8/12 curve is why. Adding a fresh country to three distinct takes the bonus \
         4 → 8 while the best attainable marginal stays 4, so O5 would not move; adding \
         the FIFTH takes the bonus 8 → 12 and consumes every icon, leaving a best \
         attainable marginal of 0. So O5 falls by 4 exactly where `CountryBonusNow` rises \
         by 4, and the pair is registered on that step.",
        "The four pre-completed orders come out of the BOX rather than off the deck, \
         which is what lets the pair register the EARLY endpoint at all: §5.1's p = 0 \
         realization wants a FULL deck, and any card taken from the deck would move the \
         deck clock. The construction is legal and clock-exact rather than reachable, \
         which is the standing trade this module's `build_registered_pair` doc states.",
        "O6 is NOT listed: the registered Dutch card is icon-free, so the row is 0 on \
         both sides. 6(a)'s card carries the icon and is where §5.2's 'O6 retires' clause \
         is exercised — 6(c) inherits the rows of 6(a) that its own vehicle actually \
         moves, which is what 'plus 6(a)'s rows' can mean once a card is registered.",
    ],
};

/// §5.2 6(d-church) — **PLACED ≻ UNPLACED**, the S2 full-collapse control.
pub const PAIR_6D_CHURCH: PairDeclaration = PairDeclaration {
    id: "6(d-church)",
    name: "PLACED ≻ UNPLACED (church)",
    family: 6,
    greater_side: "the faith-4 space taken, the acted area emptied",
    lesser_side: "the church action skipped, the pawn still standing",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Late],
    view_class: AREA_ACTION_RESOLVED,
    margin: MarginClass::DirectionOnly,
    carrier: None,
    // §5.2's EXACTLY-ONE-assistant case makes the two sides differ in whether
    // the acted area still holds the pawn — the per-side shape the frame's
    // `SidedPresidents` exists for, and the pair its doc names.
    presidents: SidedPresidents {
        greater: SeatPresidents {
            subject: PresidentDecl::OnBoardEmptyArea,
            opponent: PresidentDecl::InHand,
        },
        lesser: SeatPresidents {
            subject: PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: false },
            opponent: PresidentDecl::InHand,
        },
    },
    draws: DRAWS_CHURCH_PAYMENT,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistants_at_the_church", 1),
        ("realizable_power", 2),
        ("distinct_payment_types", 2),
        ("total_faith", 4),
        ("chosen_faith_space", CHURCH_FAITH_SPACE as i64),
        ("space_vp", 3),
        ("church_majority_place", 2),
        ("church_bonus_gained", 3),
        ("declined_movement_riders", CHURCH_RIDERS_DECLINED as i64),
        ("subject_order_card", FIXTURE_TRACK_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::VictoryPoints),
        FeatureId::Incumbent(Feature::ChurchBonusNow),
        FeatureId::Incumbent(Feature::AssistantsOnBoard),
        FeatureId::Candidate(CandidateFeature::TrackAccess),
        FeatureId::Candidate(CandidateFeature::AssistantAreasDistinct),
        FeatureId::Candidate(CandidateFeature::PresidentLocalAccess),
        FeatureId::Candidate(CandidateFeature::TradeGoodsStock),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: (church, phase = AreaAction resolved; LATE, deck-saturated construction): \
         the EXACTLY-ONE-assistant full-collapse case (S2 → 0 for the area; the pawn is \
         area-paid by construction whenever S2 > 0 — a hand-paying fixture would be \
         unreachable). Chosen space REGISTERED with its majority delta (the dummy at \
         faith-7). Collateral (all listed): VP +space, `ChurchBonusNow` ±, S2 →0, \
         `AssistantsOnBoard` −1, S3 −1, S4/S7 as constructed, goods/yen payment −.",
        "THE TWO SIDES ARE THE TWO RESOLUTIONS OF ONE AREA ACTION, both engine-driven: \
         the church always offers `AreaAction::Skip`, so the lesser side is a real move \
         from the same root rather than a subtraction. Both land at the same phase, which \
         is what lets one view class describe them.",
        "S7 is DECLARED UNMOVED and therefore NOT listed, which §5.2's 'S4/S7 as \
         constructed' leaves to the fixture. The seat's only board pawn is the one at the \
         acted area, and S7's on-board branch SUBTRACTS the president's current area — so \
         the unplaced side's reachable set is {the church} minus the church, which is \
         empty, exactly as the placed side's is. S4 does move (−1): its anchor term counts \
         the president's own area, which S7's subtraction removes.",
        "The payment is copper + silk, two distinct types, which is the whole of the \
         faith arithmetic: min(power 2, 5) + 2 = 4. Paying the YEN type instead would \
         cost 2 Yen for the same single faith step (D1) and move the money term; paying a \
         third type would carry faith past 4 and change which spaces qualify.",
        "D6 AREA-FIRST is what makes this the collapse case rather than a hand-paying \
         one: the track pawn is taken from the ACTIVATED area whenever the actor has one \
         standing there, so `AssistantsOnBoard` falls and \
         `AssistantsInHandCore/Excess` do not move at all — the correction the dossier \
         records against the pre-rev-7 model.",
    ],
};

/// §5.2 6(d-customs) — **PLACED ≻ UNPLACED**, the same control at the customs
/// track.
pub const PAIR_6D_CUSTOMS: PairDeclaration = PairDeclaration {
    id: "6(d-customs)",
    name: "PLACED ≻ UNPLACED (customs)",
    family: 6,
    greater_side: "the space-1 slot taken, the acted area emptied",
    lesser_side: "the customs action skipped, the pawn still standing",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Late],
    view_class: AREA_ACTION_RESOLVED,
    margin: MarginClass::DirectionOnly,
    carrier: None,
    presidents: SidedPresidents {
        greater: SeatPresidents {
            subject: PresidentDecl::OnBoardEmptyArea,
            opponent: PresidentDecl::InHand,
        },
        lesser: SeatPresidents {
            subject: PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: false },
            opponent: PresidentDecl::InHand,
        },
    },
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistants_at_the_customs", 1),
        ("realizable_power", 2),
        ("imported_goods_paid", 1),
        ("chosen_space", CUSTOMS_SPACE as i64),
        ("space_vp", 5),
        ("customs_majority_place", 2),
        ("customs_bonus_gained", 4),
        ("declined_warehouse_takes", CUSTOMS_RIDERS_DECLINED as i64),
        ("subject_order_card", FIXTURE_TRACK_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::VictoryPoints),
        FeatureId::Incumbent(Feature::CustomsBonusNow),
        FeatureId::Incumbent(Feature::ImportedGoods),
        FeatureId::Incumbent(Feature::AssistantsOnBoard),
        FeatureId::Candidate(CandidateFeature::TrackAccess),
        FeatureId::Candidate(CandidateFeature::AssistantAreasDistinct),
        FeatureId::Candidate(CandidateFeature::PresidentLocalAccess),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: (customs; LATE): same shape; collateral additionally `ImportedGoods` −N, \
         the space's yen/warehouse riders (+2¥ at space 2, warehouse takes at 1/3 — takes \
         declined in the registered fixture), `CustomsBonusNow` ± vs the dummy at \
         space-4.",
        "SPACE 1 IS THE REGISTERED ANSWER TO A REAL QUEUED CHOICE, not an inline \
         placement: cost-1 maps to spaces {1, 2} and both are empty at the late base, so \
         the engine queues `CustomsSpace { cost: 1 }` and the construction resolves it \
         with the registered space. Its two warehouse-take riders are then DECLINED, per \
         §5.2's own clause — the alternative, space 2, would credit 2 Yen and move the \
         money term instead.",
        "E2 is NOT listed and the church pair's is: customs is paid in IMPORTED GOODS, \
         which are their own incumbent counter and not one of the four trade goods. That \
         is also why this pair's draws carry no moved-goods set — a Category-A leg cannot \
         read an import.",
        "The majority arithmetic mirrors the church's: one pawn at index 0 against the 2P \
         dummy at index 3 ties on count and loses on RIGHTMOST, so the dummy consumes \
         first place and the subject takes second — 4 VP, where the unplaced side is not \
         in the ranking at all.",
    ],
};

// ---------------------------------------------------------------------------
// §5.2 FAMILY 7 — anti-parking (1)
// ---------------------------------------------------------------------------

/// §5.2 family 7 — **CONVERTED ≥ HELD**, the WEAK coverage row.
pub const PAIR_7: PairDeclaration = PairDeclaration {
    id: "7",
    name: "CONVERTED ≥ HELD",
    family: 7,
    greater_side: "the Silk Mill's one good taken, the pawn still standing",
    lesser_side: "the same access held un-converted",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Late],
    view_class: AREA_ACTION_RESOLVED,
    margin: MarginClass::Weak,
    carrier: None,
    presidents: SidedPresidents::symmetric(SeatPresidents {
        subject: PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: false },
        opponent: PresidentDecl::InHand,
    }),
    draws: DRAWS_SILK_MOVED,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("assistants_at_the_silk_mill", 1),
        ("realizable_power", 2),
        ("silk_gained", 1),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::TradeGoodsStock)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: CONVERTED ≥ HELD, WEAK (margin 0), MID-TURN, LATE. Recorded: implied by \
         3(b) under any positive `w_E2` — registered for the checkpoint clause's \
         coverage, not solve power.",
        "The implication is what fixes the construction: a pair implied by 3(b) (MORE \
         GOODS ≻ FEWER, all else identical) is one whose two sides differ in NOTHING BUT \
         the goods. So 'HELD' names the access still un-converted at the same instant, \
         and the presence is identical on both sides — a pair that also moved the pawns \
         would not be implied by 3(b) at all.",
        "It is 2(a)'s construction at ONE pawn rather than two, so the pair carries its \
         own bytes rather than re-registering 2(a)'s at a weaker margin: one assistant \
         plus the president's +1 is power 2, and `silk_mill_reward(2)` is 1. §5.2's own \
         'not solve power' is honoured by the margin class — WEAK asks only for \
         greater ≥ lesser.",
    ],
};

// ---------------------------------------------------------------------------
// §5.2 FAMILY 8 — recovery/hand ordering (1)
// ---------------------------------------------------------------------------

/// The area family 8's STRANDED president stands on — empty of its own
/// assistants, and free-adjacent to both of them.
///
/// Area 0's free ring is `{1, 4}` on the 2P staggered pyramid (the edge list is
/// [`RING_LAYOUT`]'s doc), which is exactly [`FAMILY_8_STRANDED_AREAS`]. That
/// adjacency is what makes the two PAA-class rows MOVE rather than read zero on
/// both sides: an isolated president would give S4 = 0 + 0 and S7 = 0, matching
/// the recalled side's structural zeroes and registering two collateral rows
/// that never fire.
pub const FAMILY_8_PRESIDENT_AREA: AreaId = AreaId(0);

/// The two EXHAUSTED areas family 8's last pawns stand on.
///
/// Exhausted is an ARITHMETIC claim here, not an adjective. Area 1 carries the
/// Copper Mine and one pawn plus the president's +1 is realizable power 2, where
/// `copper_mine_reward(2) = 0` — S1 and X3 both read zero off the reward table.
/// Area 4 carries Laboratory A, where the registered purse of
/// [`FAMILY_8_SUBJECT_MONEY`] yen buys `floor(0/2) + 0 = 0` against a cheapest
/// printed deficit of `3 + 0 − 2 = 1` — X1 reads zero because nothing on the
/// board is takeable. Every other S and X term is gated on presence the seat
/// does not have.
pub const FAMILY_8_STRANDED_AREAS: [AreaId; 2] = [AreaId(1), AreaId(4)];

/// Family 8's registered purse: nothing.
///
/// It is what makes the Laboratory pawn exhausted (see
/// [`FAMILY_8_STRANDED_AREAS`]), and it is equal on both sides, so E1
/// differences to zero and the registration's "exhausted" is a checkable fact
/// rather than a description.
pub const FAMILY_8_SUBJECT_MONEY: u32 = 0;

/// **FAMILY 8's HAND-BUILT LAYOUT.**
///
/// # Why family 8 needs one at all — an S2-T6 judgment call, flagged
///
/// §5.1 names "the hand-built layouts of 1(c)/1(e)/11" among its open
/// parameters and does not name family 8. This layout is registered anyway,
/// under the same clause's other half ("each fixture's literal state bytes"),
/// because the registration's own words are geometric: the pawns sit on
/// EXHAUSTED areas and the president is left standing among them, which is a
/// claim about which types sit next to which. On the seeded board the Copper
/// Mine and the Laboratory happen to fall where they fall, and a pair whose
/// registered collateral depends on a shuffle is a pair that re-rolls its own
/// rows. [`LayoutClass::HandBuilt`] is declared so the digest carries it.
///
/// # What it seats, and why each seat is load-bearing
///
/// Area 0 is the Church: the stranded president stands there holding none of
/// its own assistants, and a Church with no own pawn reads S2 = 0 by the term's
/// presence gate, so the president's own tile contributes nothing to either
/// side. Areas 1 and 4 are the two exhausted areas
/// ([`FAMILY_8_STRANDED_AREAS`]). The remaining seven types fill the remaining
/// seven areas, where no pawn of either seat ever stands.
pub const FAMILY_8_LAYOUT: [AreaType; TWO_PLAYER_AREAS] = [
    AreaType::Church,           // 0 — the stranded president's area, own-empty
    AreaType::CopperMine,       // 1 — exhausted: copper_mine_reward(2) = 0
    AreaType::SilkMill,         // 2
    AreaType::TeaPlantation,    // 3
    AreaType::LaboratoryA,      // 4 — exhausted: nothing takeable at money 0
    AreaType::FishingGround,    // 5
    AreaType::Bank,             // 6
    AreaType::PortA,            // 7
    AreaType::Customs,          // 8
    AreaType::EmploymentAgency, // 9
];

/// §5.2 family 8 — **RECALLED ≻ STRANDED**.
pub const PAIR_8: PairDeclaration = PairDeclaration {
    id: "8",
    name: "RECALLED ≻ STRANDED",
    family: 8,
    greater_side: "the president recalled to hand with the last two pawns",
    lesser_side: "the president left on the board and the two pawns left on exhausted areas",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    // §5.1's carrier map names no feature for family 8 — the registration is a
    // hand-ordering row, and the field's own doc reserves `None` for exactly
    // the rows that carry no production feature.
    carrier: None,
    presidents: SidedPresidents {
        greater: SeatPresidents::BOTH_IN_HAND,
        lesser: SeatPresidents {
            subject: PresidentDecl::OnBoardEmptyArea,
            opponent: PresidentDecl::InHand,
        },
    },
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::HandBuilt,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("president_area", FAMILY_8_PRESIDENT_AREA.0 as i64),
        ("stranded_area_a", FAMILY_8_STRANDED_AREAS[0].0 as i64),
        ("stranded_area_b", FAMILY_8_STRANDED_AREAS[1].0 as i64),
        ("recalled_pawns", 2),
        ("subject_money", FAMILY_8_SUBJECT_MONEY as i64),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::AssistantsInHandExcess),
        FeatureId::Incumbent(Feature::AssistantsOnBoard),
        FeatureId::Candidate(CandidateFeature::AssistantAreasDistinct),
        FeatureId::Candidate(CandidateFeature::PresidentLocalAccess),
        FeatureId::Candidate(CandidateFeature::PresidentReachableAreas),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: RECALLED ≻ STRANDED (late-game `ReturnPresidentToHand` refilling hand-core vs \
         leaving the last pawns on exhausted areas). DIRECTION-ONLY, LATE, HAND-OFF \
         (president InHand on the recalled side — S4/S7 switch to their in-hand forms; both \
         PAA-class counts, deltas listed). The V2 hand ordering (`core > on_board > excess`, \
         both vectors) rides the solve as standing constraints.",
        "THE PER-SIDE PRESIDENT DECLARATION IS THE NATURAL READING and is declared as one: \
         the recall is what the pair is about, so the recalled side's president is InHand and \
         the stranded side's is on the board — one `SidedPresidents` describing two shapes, \
         exactly the case the type was split for. The stranded president stands on an area \
         holding none of its own assistants, which is §5.1's post-Recovery hand-off shape.",
        "RESOLVED READING (an S2-T6 judgment call, 11(c)'s precedent): the registration says \
         'refilling hand-core', and at the constructed hand the refill lands in EXCESS. The \
         two recovered pawns arrive in a hand already holding six, past the two-pawn core \
         reserve, so `AssistantsInHandCore` reads 2 on both sides and does not move while \
         `AssistantsInHandExcess` moves by 2 — and only the row that moves is listed, since a \
         listed row that does not move is the other half of §5.1's defect rule. The task's own \
         framing settles it: the V2 hand ordering is a registered NOTE riding the solve, not a \
         fixture axis, so nothing here is built to make the core term fire.",
        "BOTH PAA-CLASS COUNTS MOVE, and their SIGN is the pair's whole difficulty rather than \
         a defect. The recalled side has no board assistants at all, so S4 and S7 both read \
         their in-hand-branch zero; the stranded side reads S4 = 0 + 1 + 1 = 2 (the president's \
         own area is empty, its free ring holds both pawns) and S7 = 2 (both pawns are \
         reachable, and the barred own area is not in the set to subtract). So ΔS4 = ΔS7 = −2, \
         and the registered direction asks the solve to pay for the hand refill and the \
         recovered flexibility ANYWAY. §3.2's S7 boundary names the branch switch as a \
         structural step in the recalled direction; that step is not what carries this pair, \
         and saying so is more useful than building around it.",
        "The recall is verified through the ENGINE's own `ReturnPresidentToHand`: the \
         construction applies it to the stranded layout at the Movement phase, where \
         `legal_moves.rs` offers a recovery option for every subset of the seat's board pawns, \
         and refuses the pair unless the resulting president location and assistant \
         distribution are the greater side's byte for byte.",
    ],
};

// ---------------------------------------------------------------------------
// §5.2 FAMILY 10 — crossed access-and-need (3 gates + 1 diagnostic)
// ---------------------------------------------------------------------------

/// **10(a)'s SIX PORT-A OFFERS**, in the greater side's slot order.
///
/// Slots 0–1 are the LOW zone, 2–3 MID, 4–5 HIGH
/// (`PortBoard::zone_of`, pinned in `actions/cards.rs:532`), and realizable
/// power 2 opens LOW alone (`port_accessible_zones(2)`). So the two GOODS-
/// MATCHING offers are seated at 0–1 on the greater side and the two inert ones
/// at 4–5; the lesser side is the same six cards with those two pairs swapped.
///
/// Matching is measured against [`FAMILY_10A_STOCKS`] — one seafood and nothing
/// else — through X2's own `Σ_g min(stock_g, req_g(offer))`:
///
/// | slot | card | requirement | match at one seafood |
/// |------|------|-------------|----------------------|
/// | 0 | 13 (English) | 2 copper, 1 silk, 2 seafood | 1 |
/// | 1 | 9 (German) | 3 silk, 1 seafood | 1 |
/// | 2 | 15 (English) | 3 copper, 1 silk, 1 tea | never read (MID closed) |
/// | 3 | 33 (German) | 3 silk, 3 tea | never read (MID closed) |
/// | 4 | 25 (German) | 3 silk, 1 tea | 0 |
/// | 5 | 30 (French) | 2 silk, 3 tea | 0 |
///
/// The MID pair is registered rather than left to the seed because the LOCATED
/// SET must be identical across the sides — O1's pool is the complement of the
/// located cards, and two different mid-zone offers would move the pool floor
/// underneath a pair whose collateral is registered as none.
///
/// **Order 3 is NOT among them, and the exclusion is a finding rather than a
/// preference:** at [`FIXTURE_SEED`] the OPPONENT is dealt order 3, and a held
/// card cannot be seated at a port slot. [`stock_port_a`] refuses such a
/// registration by name rather than silently seating something else, which is
/// how the collision surfaced.
pub const FAMILY_10A_PORT_OFFERS: [OrderCardId; 6] = [
    OrderCardId(13),
    OrderCardId(9),
    OrderCardId(15),
    OrderCardId(33),
    OrderCardId(25),
    OrderCardId(30),
];

/// 10(a)'s registered stocks, `[copper, silk, tea, seafood]`.
///
/// One seafood and nothing else, so an offer's X2 match is 1 exactly when it
/// requires seafood and 0 otherwise — the cleanest separation the term admits,
/// and the reason the zone swap moves X2 by 2 rather than by a number that
/// depends on how much of each good the seat happens to hold.
pub const FAMILY_10A_STOCKS: [u32; 4] = [0, 0, 0, 1];

/// **10(b)'s SIX LABORATORY-A CARDS**, in the greater side's slot order.
///
/// The position surcharges are `0|1 → +0`, `2|3 → +1`, `4|5 → +2`
/// (`lab_position_surcharge`, cards.rs:222, 0-BASED), the seat's realizable
/// power at the lab is 2 (one own assistant plus the president's +1), and the
/// registered purse of [`FAMILY_10B_MONEY`] yen buys `floor(2/2) + 0 = 1` of
/// deficit. X1's per-slot deficit is
/// `max(0, industry_cost + surcharge − min(5, power))`:
///
/// | slot | effect | cost | surcharge | deficit | takeable at purse 1 |
/// |------|--------|------|-----------|---------|---------------------|
/// | 0 | Brickyard | 3 | +0 | 1 | YES — the target |
/// | 1 | Exposition | 5 | +0 | 3 | no |
/// | 2 | Gaslight | 3 | +1 | 2 | no |
/// | 3 | Ball | 3 | +1 | 2 | no |
/// | 4 | Newspaper | 5 | +2 | 5 | no |
/// | 5 | Patent System | 3 | +2 | 3 | no |
///
/// The lesser side swaps slots 0 and 4, which puts Brickyard at deficit
/// `3 + 2 − 2 = 3` and Newspaper at `5 + 0 − 2 = 3`. Neither clears the purse,
/// so X1 falls to 0 — and Exposition's industry 5 is at slot 1 for exactly that
/// reason: a cheap card in the OTHER surcharge-free position would be takeable
/// on both sides and the pair would separate nothing.
pub const FAMILY_10B_LAB_STOCK: [TechEffect; 6] = [
    TechEffect::Brickyard,    // slot 0 — industry 3, the registered target
    TechEffect::Exposition,   // slot 1 — industry 5, the surcharge-free refusal
    TechEffect::Gaslight,     // slot 2 — industry 3
    TechEffect::Ball,         // slot 3 — industry 3
    TechEffect::Newspaper,    // slot 4 — industry 5
    TechEffect::PatentSystem, // slot 5 — industry 3
];

/// 10(b)'s registered purse: two yen, no imports.
///
/// X1 pools money and imports at the same 2¥-or-1-import rate, so this is
/// exactly ONE unit of deficit — enough for the cheap slot's 1 and short of the
/// dear slot's 3. §5.2's "money registered so the all-in clears only at the
/// cheap slot" is this number.
pub const FAMILY_10B_MONEY: u32 = 2;

/// The lab slot 10(b)'s target technology sits at on the GREATER side.
pub const FAMILY_10B_CHEAP_SLOT: usize = 0;
/// The lab slot it sits at on the LESSER side — surcharge +2.
pub const FAMILY_10B_DEAR_SLOT: usize = 4;

/// 10(c)'s SPREAD side, `[copper, silk, tea, seafood]`.
///
/// Four distinct good types plus the registered yen and imported good make SIX
/// distinct Church payment types, which is `calculate_church_faith`'s maximum
/// stock contribution — §5.2's "+6 faith is stock-buyable" realized rather than
/// described.
///
/// The five coppers are the trick that keeps the diagnostic's rows equal: the
/// drawn Category-A card is Copper5, whose leg is `min(copper, 5)`, so both
/// sides saturate it and `AchievementProgressSum` differences to zero even
/// though their copper holdings differ by three.
pub const FAMILY_10C_SPREAD_STOCKS: [u32; 4] = [5, 1, 1, 1];

/// 10(c)'s CONCENTRATED side — the SAME eight goods in ONE type.
///
/// E2 is a linear sum, so eight is eight on both sides and the goods term
/// cannot see the difference; the Church can, at three payment types against
/// six.
pub const FAMILY_10C_CONCENTRATED_STOCKS: [u32; 4] = [8, 0, 0, 0];

/// The achievement triple 10(c) draws.
///
/// **Copper5 is the ONLY Category-A card this diagnostic can carry.** The two
/// sides differ in silk, tea and seafood — the spread side holds one of each and
/// the concentrated side none — so a Silk6, Tea7 or Seafood7 leg would move and
/// the pair's whole claim (that NO row moves) would be false. `goods_moved`
/// names those three, so §5.1's registered draw constraint is checked by
/// `check_draws` rather than trusted.
///
/// **Copper differs too, 5 against 8, and is deliberately OUTSIDE the moved
/// set** — because its leg provably cannot move. `achievement_progress` reports
/// `min(current, needed)` and Copper5's threshold is 5, which BOTH sides are at
/// or past. That saturation is the construction's own device (it is why the
/// spread side carries five coppers rather than one), and the realized row is
/// checked by [`PairFixture::new`]'s expected-nonzero clause whatever this
/// declaration claims.
const DRAWS_10C_SPREAD: AchievementDraws = AchievementDraws {
    category_a: AchievementType::Copper5,
    category_b: AchievementType::Orders5,
    category_c: AchievementType::Commercial4,
    goods_moved: &[
        AchievementType::Silk6,
        AchievementType::Tea7,
        AchievementType::Seafood7,
    ],
    moves_yen: false,
};

/// 10(c)'s registered purse and imports, equal on both sides: they buy the yen
/// and imported-good payment types, which no distribution of GOODS can
/// concentrate away.
pub const FAMILY_10C_MONEY: u32 = 2;
/// See [`FAMILY_10C_MONEY`].
pub const FAMILY_10C_IMPORTS: u32 = 1;

/// 10(d)'s GREATER card — printed corner index 26 (French; 3 tea + 1 seafood;
/// 1 VP + 2 Yen).
pub const FAMILY_10D_MATCHED_CARD: OrderCardId = OrderCardId(26);
/// 10(d)'s LESSER card — printed corner index 5 (French; 1 copper + 2 tea;
/// 1 VP + 1 imported good).
pub const FAMILY_10D_UNMATCHED_CARD: OrderCardId = OrderCardId(5);

/// §5.2 10(a) — **OPEN-ZONE ≻ CLOSED-ZONE**, X2's gate.
pub const PAIR_10A: PairDeclaration = PairDeclaration {
    id: "10(a)",
    name: "OPEN-ZONE ≻ CLOSED-ZONE",
    family: 10,
    greater_side: "the two goods-matching offers in the LOW zone (port slots 0–1)",
    lesser_side: "the same six offers with those two in the HIGH zone (slots 4–5)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::PortTakeable)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("port_assistants", 1),
        ("realizable_power", 2),
        ("low_slot_0", FAMILY_10A_PORT_OFFERS[0].0 as i64),
        ("low_slot_1", FAMILY_10A_PORT_OFFERS[1].0 as i64),
        ("mid_slot_2", FAMILY_10A_PORT_OFFERS[2].0 as i64),
        ("mid_slot_3", FAMILY_10A_PORT_OFFERS[3].0 as i64),
        ("high_slot_4", FAMILY_10A_PORT_OFFERS[4].0 as i64),
        ("high_slot_5", FAMILY_10A_PORT_OFFERS[5].0 as i64),
        ("subject_seafood", FAMILY_10A_STOCKS[3] as i64),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::PortTakeable)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: identical stocks, hand, presence, realizable power 2; the SAME six Port-A cards \
         PERMUTED so the goods-matching offers sit in LOW (indices 0–1) vs HIGH (4–5). The \
         located-card SET is unchanged → O1's pool, O2, O3, HandSize all equal; no incumbent \
         reads slot positions (eval.rs:2017 reads only `slots.len()`); the OPPONENT holds no \
         port presence (registered — through the relative form its X2 would otherwise move \
         too). DIRECTION-ONLY, EARLY, HAND-OFF.",
        "THE eval.rs CITATION VERIFIED AT HEAD, with a line-number correction recorded rather \
         than silently followed: the only read of port state anywhere in `eval.rs` is \
         `view.port_state.port_a.slots.len()` inside `initial_order_deck_len`, which sits at \
         eval.rs:2095 today, not :2017. The CLAIM the citation makes is exact — the read is of \
         the slot COUNT, so a permutation of the six offers is invisible to all sixteen \
         incumbents — and the drift is in the line number alone.",
        "The permutation is a permutation, which is what keeps the located set fixed: the same \
         six cards stand in the port on both sides, so O1's 36-bit located bitset and its \
         complement pool are identical and the opponent's hidden-slot floor cannot move. The \
         MID pair is registered for the same reason even though power 2 never reads it.",
        "X2 reads 2 on the greater side (two offers each matching one seafood) and 0 on the \
         lesser, where the LOW zone holds the two seafood-free offers. The gate conjuncts are \
         both live: the seat holds one own assistant at the port and realizable power 2, and \
         its hand holds one card, clear of the `hand.len() >= 3` persistent collapse.",
    ],
};

/// §5.2 10(b) — **CHEAP-SLOT ≻ DEAR-SLOT**, X1's gate.
pub const PAIR_10B: PairDeclaration = PairDeclaration {
    id: "10(b)",
    name: "CHEAP-SLOT ≻ DEAR-SLOT",
    family: 10,
    greater_side: "the target technology at laboratory slot 0 (position surcharge +0)",
    lesser_side: "the same six cards with the target at slot 4 (surcharge +2)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::LabTakeable)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("lab_assistants", 1),
        ("realizable_power", 2),
        ("subject_money", FAMILY_10B_MONEY as i64),
        ("subject_imports", 0),
        ("all_in_purse", 1),
        ("cheap_slot", FAMILY_10B_CHEAP_SLOT as i64),
        ("dear_slot", FAMILY_10B_DEAR_SLOT as i64),
        ("target_industry_cost", 3),
        ("cheap_deficit", 1),
        ("dear_deficit", 3),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::LabTakeable)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: identical money, imports, power, presence; the SAME Lab-A cards permuted so the \
         target sits at slot index 0 (+0) vs 4 (+2), money registered so the all-in clears \
         only at the cheap slot. No incumbent reads `lab_state` (verified: zero hits in \
         eval.rs). DIRECTION-ONLY, EARLY, HAND-OFF.",
        "THE 'ZERO HITS' CLAIM RE-VERIFIED AT HEAD: `lab_state` appears nowhere in `eval.rs`, \
         so the entire sixteen-feature incumbent half is blind to which slot a technology \
         stands in and X1 is the sole mover by construction rather than by arithmetic.",
        "THE DEFICIT ARITHMETIC, re-derived from the engine's own helpers rather than from \
         §3.2's prose: `lab_position_surcharge` (cards.rs:222) is 0-BASED with 0|1 → +0, \
         2|3 → +1, 4|5 → +2; the cheapest printed industry value in the registry is 3; \
         realizable power at the lab is one own assistant plus the president's unconditional \
         +1. So the target's deficit is `max(0, 3 + 0 − 2) = 1` at the cheap slot and \
         `max(0, 3 + 2 − 2) = 3` at the dear one, and a purse of two yen buys \
         `floor(2/2) + 0 = 1`. The DUTY term is 0 on both sides: the seat owns no \
         technologies, so `technologies.len() >= 3` is false.",
        "The five non-target cards are chosen so the whole board is non-takeable on BOTH \
         sides — X1 reads 1 against 0 rather than n+1 against n. [`FAMILY_10B_LAB_STOCK`] \
         carries the per-slot table.",
    ],
};

/// §5.2 10(c) — **CHURCH STOCK-FAITH**, the RECORDED DIAGNOSTIC.
pub const PAIR_10C: PairDeclaration = PairDeclaration {
    id: "10(c)",
    name: "SPREAD ≻ CONCENTRATED (church stock-faith)",
    family: 10,
    greater_side: "eight goods spread over all four types, one yen leg and one import leg",
    lesser_side: "the same eight goods in ONE type, the same yen and import legs",
    role: PairRole::Diagnostic,
    stages: &[StageEndpoint::Early],
    view_class: ViewClass::HandOff,
    margin: MarginClass::Weak,
    carrier: None,
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_10C_SPREAD,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("church_assistants", 1),
        ("realizable_power", 2),
        ("greater_copper", FAMILY_10C_SPREAD_STOCKS[0] as i64),
        ("greater_silk", FAMILY_10C_SPREAD_STOCKS[1] as i64),
        ("greater_tea", FAMILY_10C_SPREAD_STOCKS[2] as i64),
        ("greater_seafood", FAMILY_10C_SPREAD_STOCKS[3] as i64),
        ("lesser_copper", FAMILY_10C_CONCENTRATED_STOCKS[0] as i64),
        ("subject_money", FAMILY_10C_MONEY as i64),
        ("subject_imports", FAMILY_10C_IMPORTS as i64),
        ("greater_realizable_faith", 8),
        ("lesser_realizable_faith", 5),
        ("residual_faith", 3),
        ("hand_emptied_by_fulfilling", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: CHURCH STOCK-FAITH cross — DIAGNOSTIC (the union residual where largest: up to \
         +6 faith is stock-buyable).",
        "THE CONSTRUCTION IS AN ALL-ROWS-EQUAL PAIR, and that is the diagnostic's whole point: \
         every one of the thirty-four rows differences to ZERO while the two sides' realizable \
         Church faith differs by 3. `calculate_church_faith` (tracks.rs:119) adds one faith per \
         DISTINCT affordable payment type — copper, silk, tea, seafood, yen, import — over \
         `church_faith_from_power`, and S2 reads the power term alone. So the spread side \
         realizes faith 2 + 6 = 8 and the concentrated side 2 + 3 = 5, and no term in the union \
         S+E identity, nor any of the sixteen incumbents, can tell them apart.",
        "HOW THE ROWS ARE HELD EQUAL, since it is not automatic. E2 is a linear SUM, so eight \
         goods are eight goods either way. The Category-A leg is the one that would otherwise \
         fire — it reads a RAW good — and Copper5's `min(copper, 5)` SATURATES at five on both \
         sides (5 against 8), which is why the spread side carries five coppers rather than \
         one. Money and imports are equal by registration, because no distribution of goods can \
         concentrate the yen and import payment types away. And the hand is emptied by the same \
         fulfillment 3(b) uses, because ANY held order would read a different O2 against two \
         stock vectors that differ.",
        "RECORDED: +3 IS THE MAXIMUM RESIDUAL AN ALL-ROWS-EQUAL PAIR CAN CARRY, and §5.2's \
         '+6' is the greater side's ABSOLUTE stock contribution, which this fixture realizes in \
         full. Four good types against one is a three-type spread, and the yen and import legs \
         cannot be moved without moving E1 and `ImportedGoods`. A +6 DELTA would need a lesser \
         side holding no goods, no money and no imports at all — three rows moving, and a \
         diagnostic whose rows move cannot say 'the union cannot see this'. The trade is \
         recorded rather than taken.",
        "Naming the spread side GREATER fixes the print's sign (§5.1: Δv = greater − lesser) \
         and asserts no direction; WEAK is the frame's inert margin class, and §5.1's pair→gate \
         mapping keeps a `PairRole::Diagnostic` row out of Part B's constraint system and Part \
         A's blocking set entirely.",
        "AN UNDERDETERMINED BYTE, flagged for the bounded re-read: §5.2 registers no stage for \
         10(c). EARLY is transcribed from the family — 10(a), (b) and (d) are all EARLY — and \
         is a choice, not a reading.",
    ],
};

/// §5.2 10(d) — **MATCHED ≻ UNMATCHED PRODUCTION ACCESS**, X3's gate.
pub const PAIR_10D: PairDeclaration = PairDeclaration {
    id: "10(d)",
    name: "MATCHED ≻ UNMATCHED PRODUCTION ACCESS",
    family: 10,
    greater_side: "holding order 26 (French; 3 tea + 1 seafood) at one Fishing Ground pawn",
    lesser_side: "holding order 5 (French; 1 copper + 2 tea) at the same pawn",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::MatchedProductionAccess)),
    presidents: SidedPresidents::BOTH_IN_HAND,
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::StandardGenerated,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("fishing_ground_assistants", 1),
        ("realizable_power", 2),
        ("greater_order_card", FAMILY_10D_MATCHED_CARD.0 as i64),
        ("lesser_order_card", FAMILY_10D_UNMATCHED_CARD.0 as i64),
        ("subject_copper", 0),
        ("subject_silk", 0),
        ("subject_tea", 0),
        ("subject_seafood", 0),
        ("fishing_ground_reward", 2),
        ("greater_residual_seafood", 1),
        ("x3_greater", 1),
        ("x3_lesser", 0),
    ],
    expected_nonzero: &[FeatureId::Candidate(CandidateFeature::MatchedProductionAccess)],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: 1 assistant at Fishing Ground, ZERO stocks, holding card 26 (FR — requires 1 \
         seafood) ≻ holding card 5 (FR — requires no seafood): equal face (4 both), equal \
         country (FR — O5's attainable set identical), equal icons (0 both), O2 = O3 = 0 at \
         zero stocks; X3 = min(fishing_ground_reward(2) = 2, residual seafood need 1) = 1 vs \
         0 — X3 the SOLE mover (the manifest guarantees 5/26's requirements differ: only \
         10/32 share a requirement vector). DIRECTION-ONLY, EARLY, HAND-OFF, presidents \
         InHand.",
        "THE MANIFEST FIGURES, re-read at HEAD rather than transcribed: card 26 is French, \
         requires 3 tea + 1 seafood and pays 1 VP + 2 Yen; card 5 is French, requires 1 copper \
         + 2 tea and pays 1 VP + 1 imported good. O1's half-VP face is \
         `2·VP + 2·imports + money` — 2 + 0 + 2 = 4 for card 26 and 2 + 2 + 0 = 4 for card 5 — \
         so the two are face-equal in the term's own units, not merely in printed VP. Neither \
         carries a movement or warehouse icon, so O6 is 0 on both sides.",
        "THE POOL FLOOR IS INVARIANT UNDER THE 5↔26 SWAP for 4(e)'s recorded reason: O1's pool \
         is the complement of the located set, the swap exchanges which of two EQUAL-half-VP \
         cards is located, and the pool's running (count, half-VP sum) is therefore unchanged. \
         The construction protects that by parking BOTH cards off the port before either is \
         dealt — a card taken out of a PORT SLOT would move the located set for real, since \
         face-up offers are located and deck and box cards are not.",
        "ZERO STOCKS IS CONSTRUCTION SURGERY, and the honest accounting is this: §5.1's p = 0 \
         base deals one of each good (`types/player.rs:245–248`), and this pair writes all \
         four to zero. Nothing is conserved by the write — trade goods come from a general \
         supply and appear in no invariant's conservation total, unlike the 36 orders and the \
         30 technologies — and a seat at zero stocks is entirely ordinary play, since goods \
         leave on every fulfillment and every Church donation. It is registered as four \
         parameters so a re-read sees the choice rather than inferring it.",
        "At zero stocks the residual need EQUALS each card's printed requirement, which is what \
         makes the pair a clean read of X3's conditional: the greater side's Fishing Ground \
         access is worth `min(2, 1) = 1` needed seafood and the lesser side's the same access \
         is worth `min(2, 0) = 0`. S1 reads `fishing_ground_reward(2) = 2` on BOTH sides — the \
         unconditional access term does not separate them, which is the whole reason X3 exists.",
    ],
};

// ---------------------------------------------------------------------------
// §5.2 FAMILY 11 — activation choice (5): the conservation gates
// ---------------------------------------------------------------------------

/// §5.2 11(a) — **ACTED-AT-SILK ≻ ACTED-AT-COPPER**.
pub const PAIR_11A: PairDeclaration = PairDeclaration {
    id: "11(a)",
    name: "ACTED-AT-SILK ≻ ACTED-AT-COPPER",
    family: 11,
    greater_side: "acted at the Silk Mill and recovered (+1 silk; the Copper pawn kept)",
    lesser_side: "acted at the Copper Mine and recovered (no yield; the Silk pawn kept)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::ProductionAccess)),
    presidents: SidedPresidents::symmetric(SeatPresidents {
        subject: PresidentDecl::OnBoardEmptyArea,
        opponent: PresidentDecl::InHand,
    }),
    draws: DRAWS_SILK_MOVED,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::HandBuilt,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("high_access_area", FAMILY_11_HIGH_ACCESS_AREA.0 as i64),
        ("copper_mine_area", FAMILY_11_COPPER_AREA.0 as i64),
        ("assistants_per_area", 1),
        ("silk_gained", 1),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Candidate(CandidateFeature::ProductionAccess),
        FeatureId::Candidate(CandidateFeature::TradeGoodsStock),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: Expected nonzero rows: E2 +1, S1 −1 (the winner keeps the 0-reward Copper \
         pawn, the loser the 1-reward Silk pawn — the row pair that makes this the ORDERING \
         constraint w_E2 − w_S1 ≥ 1). Both endpoints, HAND-OFF, DIRECTION-ONLY.",
        "The registered draw constraint binds here: the recipe moves SILK, so the drawn \
         Category-A card may not be Silk6 — otherwise the achievement leg would read the \
         produced good and `AchievementProgressSum` would move as unlisted collateral.",
        "The order card is registered needing no silk, which is what holds O2, O3 and X3 \
         equal across a side that owns one more silk than the other.",
    ],
};

/// §5.2 11(b) — **ACTED-AT-BANK ≻ ACTED-AT-COPPER**.
pub const PAIR_11B: PairDeclaration = PairDeclaration {
    id: "11(b)",
    name: "ACTED-AT-BANK ≻ ACTED-AT-COPPER",
    family: 11,
    greater_side: "acted at the Bank and recovered (+2 yen; the Copper pawn kept)",
    lesser_side: "acted at the Copper Mine and recovered (no yield; the Bank pawn kept)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::BankAccess)),
    presidents: SidedPresidents::symmetric(SeatPresidents {
        subject: PresidentDecl::OnBoardEmptyArea,
        opponent: PresidentDecl::InHand,
    }),
    draws: DRAWS_YEN_MOVED,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::HandBuilt,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("high_access_area", FAMILY_11_HIGH_ACCESS_AREA.0 as i64),
        ("copper_mine_area", FAMILY_11_COPPER_AREA.0 as i64),
        ("assistants_per_area", 1),
        ("yen_gained", 2),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Candidate(CandidateFeature::MoneyStock),
        FeatureId::Candidate(CandidateFeature::BankAccess),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: Expected nonzero: E1 +2, S5 −2. Both endpoints.",
        "`bank_reward(realizable power 2) = 2` is the whole of both rows: the yield is +2 \
         yen on the acting side, and the SAME 2 is what the non-acting side still has on \
         tap through its undrained Bank presence.",
        "The registered draw constraint's yen half binds: the drawn Category-B card may not \
         be Money10, whose leg is `min(money, 10)` and would shadow E1 exactly.",
    ],
};

/// §5.2 11(c) — **ACTED-AT-EA ≻ ACTED-AT-COPPER**.
pub const PAIR_11C: PairDeclaration = PairDeclaration {
    id: "11(c)",
    name: "ACTED-AT-EA ≻ ACTED-AT-COPPER",
    family: 11,
    greater_side: "acted at the Employment Agency and recovered (one assistant retrieved)",
    lesser_side: "acted at the Copper Mine and recovered (the EA pawn kept)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::EmploymentAgencyAccess)),
    presidents: SidedPresidents::symmetric(SeatPresidents {
        subject: PresidentDecl::OnBoardEmptyArea,
        opponent: PresidentDecl::InHand,
    }),
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::HandBuilt,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("high_access_area", FAMILY_11_HIGH_ACCESS_AREA.0 as i64),
        ("copper_mine_area", FAMILY_11_COPPER_AREA.0 as i64),
        ("assistants_per_area", 1),
        ("tokens_taken", 1),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::AssistantsInWarehouse),
        FeatureId::Incumbent(Feature::AssistantsInHandExcess),
        FeatureId::Candidate(CandidateFeature::EmploymentAgencyAccess),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: Expected nonzero: `AssistantsInWarehouse` −1, hand-core/excess +1, S6 −1. \
         Both endpoints.",
        "RESOLVED READING (an S2-T4 judgment call): the registration writes \
         'hand-core/excess +1' without saying which. The retrieved pawn lands in a hand \
         already holding seven, past the two-pawn core reserve, so it is \
         `AssistantsInHandExcess` that moves and `AssistantsInHandCore` that does not — and \
         only the row that moves is listed, since a listed row that does not move is the \
         other half of §5.1's defect rule.",
        "`employment_agency_tokens_allowed(realizable power 2) = 1` and assistants are the \
         free class in the basket, which is why the take moves the two pawn counters and \
         leaves E1 alone.",
    ],
};

/// §5.2 11(d) — **ACTED-AT-LAB ≻ ACTED-AT-COPPER**.
pub const PAIR_11D: PairDeclaration = PairDeclaration {
    id: "11(d)",
    name: "ACTED-AT-LAB ≻ ACTED-AT-COPPER",
    family: 11,
    greater_side: "acted at the Laboratory and recovered (a zero-deficit technology bought)",
    lesser_side: "acted at the Copper Mine and recovered (the Laboratory pawn kept)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Early, StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::LabTakeable)),
    presidents: SidedPresidents::symmetric(SeatPresidents {
        subject: PresidentDecl::OnBoardEmptyArea,
        opponent: PresidentDecl::InHand,
    }),
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration {
        excluded_effects: NO_CONVERSION_TECHS,
        // The registered constraint is "≤ 1 tech PRE-purchase" on the subject;
        // this field is checked over BOTH seats and over the FINAL states, so
        // the realized bound is 2 (the subject after its purchase). What the
        // registration actually protects — `technologies.len()` never reaching
        // 3, where D2's 4th-card duty would enter X1's future terms — is what
        // a bound of 2 enforces, on every seat at once.
        max_owned: Some(2),
    },
    layout: LayoutClass::HandBuilt,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("high_access_area", FAMILY_11_HIGH_ACCESS_AREA.0 as i64),
        ("copper_mine_area", FAMILY_11_COPPER_AREA.0 as i64),
        ("assistants_per_area", 1),
        ("station_power_at_the_lab", 1),
        ("realizable_power_at_the_lab", 3),
        ("purchased_industry_cost", 3),
        ("purchased_slot_index", 0),
        ("purchase_deficit", 0),
        ("subject_money", 0),
        ("techs_owned_pre_purchase", 1),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::CountryBonusNow),
        FeatureId::Incumbent(Feature::TechBonusNow),
        FeatureId::Candidate(CandidateFeature::LabTakeable),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: a zero-deficit, no-duty purchase registered; REGISTERED CONSTRAINTS: the seat \
         owns ≤ 1 tech pre-purchase so `technologies.len()` never crosses 2 → 3 and the \
         4th-card duty stays off X1's future terms; Technologies4 NOT among the drawn \
         achievements; the purchased flag's icon parity declared ODD post-purchase so \
         `check_mercenary_eligibility`'s sweep moves nothing — the award fires at EVEN \
         post-counts. Expected nonzero: `TechBonusNow` +industry, `CountryBonusNow`/O5 per \
         the flag, X1 −1 takeable; E1 unchanged.",
        "CONSTRUCTIBILITY COLLISION, resolved and reported (S2-T4): a zero-deficit purchase \
         is IMPOSSIBLE at family 11's own template power of 2, because the cheapest \
         technology in the printed registry costs 3 industry — `deficit = max(0, cost + \
         position surcharge − power)` is at least 1 for every card at every slot. The third \
         power point comes from a STATION TOKEN standing at the Laboratory (the token adds \
         +1 for all players; the OPPONENT owns the Station technology, so the subject's own \
         '≤ 1 tech pre-purchase' is untouched). That keeps the pre-action position \
         symmetric, keeps the Copper Mine at power 2, and leaves the congruence argument \
         alone, since S4 and S7 read own assistants and adjacency and never read power.",
        "RESOLVED READING: 'CountryBonusNow/O5 per the flag' is realized as a NEW country — \
         the purchase takes the seat from one distinct country to two, so `CountryBonusNow` \
         moves 0 → 2 and its post-purchase icon count is 1, ODD, exactly as the mercenary \
         clause registers. O5 was computed on both sides and does NOT move (a seat at one \
         distinct country and a seat at two both have a best attainable marginal of 2 VP), \
         so it is not listed.",
        "X1 = 1 on the lesser side by construction: money 0 and no imports mean only a \
         zero-deficit slot is takeable, and the Laboratory is stocked with exactly one \
         cost-3 card, in slot 0 where the position surcharge is 0.",
    ],
};

/// §5.2 11(e) — **ACTED-AT-PORT ≻ ACTED-AT-COPPER**.
pub const PAIR_11E: PairDeclaration = PairDeclaration {
    id: "11(e)",
    name: "ACTED-AT-PORT ≻ ACTED-AT-COPPER",
    family: 11,
    greater_side: "acted at the Port and recovered (one order card taken free)",
    lesser_side: "acted at the Copper Mine and recovered (the Port pawn kept)",
    role: PairRole::Constraint,
    stages: &[StageEndpoint::Late],
    view_class: ViewClass::HandOff,
    margin: MarginClass::DirectionOnly,
    carrier: Some(FeatureId::Candidate(CandidateFeature::PortTakeable)),
    presidents: SidedPresidents::symmetric(SeatPresidents {
        subject: PresidentDecl::OnBoardEmptyArea,
        opponent: PresidentDecl::InHand,
    }),
    draws: DRAWS_TEA_LEG,
    techs: TechDeclaration { excluded_effects: NO_CONVERSION_TECHS, max_owned: Some(0) },
    layout: LayoutClass::HandBuilt,
    parameters: &[
        ("setup_seed", FIXTURE_SEED as i64),
        ("high_access_area", FAMILY_11_HIGH_ACCESS_AREA.0 as i64),
        ("copper_mine_area", FAMILY_11_COPPER_AREA.0 as i64),
        ("assistants_per_area", 1),
        ("taken_port_slot", 0),
        ("taken_order_card", FIXTURE_PORT_CARD.0 as i64),
        ("subject_order_card", FIXTURE_ORDER_CARD.0 as i64),
    ],
    expected_nonzero: &[
        FeatureId::Incumbent(Feature::HandSize),
        FeatureId::Candidate(CandidateFeature::HeldOrderValue),
        FeatureId::Candidate(CandidateFeature::OrderMatchedGoods),
        FeatureId::Candidate(CandidateFeature::PortTakeable),
    ],
    early_hand_cap_deviation: None,
    notes: &[
        "§5.2: (a free take). Expected nonzero: O1/O2 +, HandSize +1, X2 per the hand count \
         and the drained presence. LATE (the refill clock, as 4(f)).",
        "The late endpoint is what makes the take a REGISTERED construction rather than an \
         accident: §5.1's late recipe empties the deck, so the taken slot cannot refill, and \
         the engine's own `OrderDeckEmpty` trigger (deck empty AND a port slot open) fires \
         on the taking side. §5.1's late clause reads \"deck empty, port full OR the trigger \
         declared\" precisely for this shape — the greater side declares the trigger, the \
         lesser side keeps a full port, and both sit at p = FULL.",
        "The trigger does NOT make the side terminal: `view_is_terminal` also wants \
         `final_round` and the turn resting at the last seat's TurnEnd, and neither holds \
         here.",
        "The taken card is registered rather than drawn: it must match a good the seat holds \
         (so O2 moves), stay unexecutable (so O3 does not), and carry no movement or \
         warehouse icon face (so O6 does not).",
    ],
};

// ---------------------------------------------------------------------------
// The registry (spec §5.1's registered counts)
// ---------------------------------------------------------------------------

/// **THE REGISTERED PAIRS.** Families 1 and 11 (S2-T4), families 2, 6 and 7
/// (S2-T5) and families 3, 8 and 10 (S2-T6) today; each remaining §5.2 family
/// lands as its own later registration.
///
/// The order is §5.2's own, family by family — and inside a family that means
/// the section's literal order, not a tidied one: family 2 prints its three
/// gates (a)/(b)/(d) before the (c) diagnostic, and family 10 prints its (c)
/// diagnostic BETWEEN its gates. Transcribing it rather than tidying it is what
/// lets the bounded re-read diff the registry against the section.
///
/// The array's LENGTH is its own pin — [`REGISTERED_PAIR_COUNT`] — on
/// `tests/scenario_suite.rs`'s discipline: a registry whose size is a type is a
/// registry a deletion cannot quietly shrink. A family task bumps the constant
/// and the array together, and the count test's message says what the target
/// is.
pub const REGISTERED: [PairDeclaration; REGISTERED_PAIR_COUNT] = [
    PAIR_1A,
    PAIR_1B,
    PAIR_1C,
    PAIR_1D,
    PAIR_1E,
    PAIR_2A,
    PAIR_2B,
    PAIR_2D,
    PAIR_2C,
    PAIR_3A,
    PAIR_3B,
    PAIR_3C,
    PAIR_3D,
    PAIR_6A,
    PAIR_6B,
    PAIR_6C,
    PAIR_6D_CHURCH,
    PAIR_6D_CUSTOMS,
    PAIR_7,
    PAIR_8,
    PAIR_10A,
    PAIR_10B,
    PAIR_10C,
    PAIR_10D,
    PAIR_11A,
    PAIR_11B,
    PAIR_11C,
    PAIR_11D,
    PAIR_11E,
];

/// Length of [`REGISTERED`], bumped by each family registration task.
pub const REGISTERED_PAIR_COUNT: usize = 29;

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
    /// 0. The registration is internally coherent
    ///    ([`PairDeclaration::check_consistency`]) — first, because it needs no
    ///    state at all and an incoherent tuple makes every later clause a
    ///    statement about a registration nothing can satisfy.
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
        // 0. The registration is coherent on its own terms.
        declaration.check_consistency()?;

        // 1. The endpoint is one this pair registers.
        if !declaration.stages.contains(&stage) {
            return Err(FixtureError::Declaration {
                clause: "declared stage endpoint",
                detail: format!(
                    "{} registers stages {:?} and this realization asks for {}",
                    declaration.id,
                    declaration.stages.iter().map(|s| s.name()).collect::<Vec<_>>(),
                    stage.name(),
                ),
            });
        }

        let sides = [(Side::Greater, &greater), (Side::Lesser, &lesser)];

        // 2–4. Player count, engine validity, and the seat that signs the view.
        for (side, state) in sides {
            if state.config.player_count != 2 {
                return Err(FixtureError::Declaration {
                    clause: "2P-only",
                    detail: format!(
                        "the {} side has {} players; §5.1's stage realization is written at 2P \
                         and §5.3's signature is 2P-only (above two seats `opponent_extreme` \
                         orients by weight)",
                        side.name(),
                        state.config.player_count,
                    ),
                });
            }
            check_invariants(state, rules).map_err(|detail| FixtureError::NotEngineValid {
                side: side.name(),
                detail,
            })?;
            if !state.players.iter().any(|p| p.id == subject) {
                return Err(FixtureError::Declaration {
                    clause: "subject seat",
                    detail: format!(
                        "seat {} has no row on the {} side",
                        subject.0,
                        side.name()
                    ),
                });
            }
            if state.current_player != subject {
                return Err(FixtureError::Declaration {
                    clause: "the subject is the mover",
                    detail: format!(
                        "the {} side's mover is seat {} but the pair is signed from seat {}; \
                         §5.3 signs the MOVER's view",
                        side.name(),
                        state.current_player.0,
                        subject.0,
                    ),
                });
            }
        }

        // 5. Both sides observable from the subject seat.
        let greater_view = observe(&greater, subject).map_err(|e| FixtureError::NotObservable {
            side: Side::Greater.name(),
            detail: format!("{e}"),
        })?;
        let lesser_view = observe(&lesser, subject).map_err(|e| FixtureError::NotObservable {
            side: Side::Lesser.name(),
            detail: format!("{e}"),
        })?;
        let views = [(Side::Greater, &greater_view), (Side::Lesser, &lesser_view)];

        // 6. Neither side is a finished game.
        for (side, view) in views {
            if let Some(standing) = eval::terminal_standing(view, rules) {
                return Err(FixtureError::Declaration {
                    clause: "non-terminal sides",
                    detail: format!(
                        "the {} side is terminal (rank {} of {}); a finished game's positional \
                         vector does not govern at all",
                        side.name(),
                        standing.rank,
                        standing.seats,
                    ),
                });
            }
        }

        // 7. One stage, both sides, exactly.
        let g_ppm = eval::progress_ppm(&greater_view, rules);
        let l_ppm = eval::progress_ppm(&lesser_view, rules);
        if g_ppm != l_ppm || g_ppm != stage.ppm() {
            return Err(FixtureError::Declaration {
                clause: "endpoint-exact progress",
                detail: format!(
                    "declared {} ({} ppm) but the greater side reads {g_ppm} and the lesser \
                     {l_ppm}",
                    stage.name(),
                    stage.ppm(),
                ),
            });
        }

        // 8. The declared view class.
        for (side, state) in sides {
            check_view_class(declaration.view_class, side, state)?;
        }

        // 9. Both seats' president declarations.
        let opponent = other_seat_2p(&greater, subject)?;
        for (side, state) in sides {
            // PER SIDE, not per pair: §5.2's 6(d) and family 8 register pairs
            // whose president SHAPE differs across the sides even though the
            // president itself does not move. See [`SidedPresidents`].
            let declared = declaration.presidents.side(side);
            check_president(
                declared.subject,
                declaration.view_class,
                side,
                state,
                subject,
                "subject",
                rules,
            )?;
            check_president(
                declared.opponent,
                declaration.view_class,
                side,
                state,
                opponent,
                "opponent",
                rules,
            )?;
        }

        // 10. The achievement draws and §5.1's registered draw constraint.
        check_draws(&declaration.draws)?;
        for (side, state) in sides {
            check_drawn_cards(&declaration.draws, side, state)?;
        }

        // 11. The technology declaration.
        for (side, state) in sides {
            check_techs(&declaration.techs, side, state, rules)?;
        }

        // 12. The stage's own construction clauses, on the FINAL states.
        for ((side, state), (_, view)) in sides.into_iter().zip(views) {
            check_stage_clauses(declaration, stage, side, state, view, rules)?;
        }

        let fixture = PairFixture {
            declaration,
            stage,
            subject,
            greater,
            lesser,
            greater_view,
            lesser_view,
        };

        // 13. §5.1: an unlisted nonzero row is a fixture defect. The INCUMBENT
        // half of that check is complete today; the candidate half joins at
        // step 3 with the columns themselves.
        let delta = fixture.incumbent_delta(rules);
        for f in Feature::ALL {
            if delta[f.index()] != 0
                && !declaration.expected_nonzero.contains(&FeatureId::Incumbent(f))
            {
                return Err(FixtureError::Declaration {
                    clause: "expected-nonzero rows",
                    detail: format!(
                        "{} moves by {} but is not listed as expected-nonzero — §5.1 calls an \
                         unlisted nonzero row a fixture defect",
                        f.name(),
                        delta[f.index()],
                    ),
                });
            }
        }

        Ok(fixture)
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

/// §5.1's VIEW CLASS, checked against one side's state.
///
/// HAND-OFF is checked STRUCTURALLY rather than by label. A non-terminal player
/// hand-off is the state `game/apply_move.rs:276–288` leaves behind when it
/// advances the seat: the phase is [`TurnPhase::AdditionalActionFirst`], the
/// choice queue is empty (the queue invariant's own hand-off clause), the
/// action power is cleared, and no mercenary sub-turn is running. `TurnEnd` is
/// the only other hand-off the machine has and nothing but the FINAL round ever
/// assigns it (`game/apply_move.rs:264`), so a non-terminal hand-off is exactly
/// the first shape.
fn check_view_class(
    declared: ViewClass,
    side: Side,
    state: &GameState,
) -> Result<(), FixtureError> {
    let mismatch = |detail: String| FixtureError::Declaration {
        clause: "view class",
        detail: format!("the {} side: {detail}", side.name()),
    };
    match declared {
        ViewClass::HandOff => {
            if state.turn_phase != TurnPhase::AdditionalActionFirst {
                return Err(mismatch(format!(
                    "declared HAND-OFF but rests at {:?}; a non-terminal hand-off rests at \
                     AdditionalActionFirst",
                    state.turn_phase,
                )));
            }
            if !state.pending_choices.is_empty() {
                return Err(mismatch(format!(
                    "declared HAND-OFF but {} choices are still queued",
                    state.pending_choices.len(),
                )));
            }
            if state.current_action_power.is_some() {
                return Err(mismatch(
                    "declared HAND-OFF but carries a current_action_power; the seat advance \
                     clears it"
                        .to_string(),
                ));
            }
            if state.is_mercenary_action {
                return Err(mismatch(
                    "declared HAND-OFF but a mercenary sub-turn is running".to_string(),
                ));
            }
            // The advance resets this in the same breath as the phase
            // (apply_move.rs:278), and the FULL equality is the right check
            // rather than `mercenary_used` alone: all three are once-per-turn
            // flags cleared together, so any one of them set describes a turn
            // already under way, not a seat that has just received one.
            if state.additional_action_state != AdditionalActionState::new() {
                return Err(mismatch(format!(
                    "declared HAND-OFF but carries used once-per-turn flags \
                     ({:?}); the seat advance resets them",
                    state.additional_action_state,
                )));
            }
        }
        ViewClass::MidTurn { phase, mercenary } => {
            if state.turn_phase != phase {
                return Err(mismatch(format!(
                    "declared MID-TURN at {phase:?} but rests at {:?}",
                    state.turn_phase,
                )));
            }
            if state.is_mercenary_action != mercenary {
                return Err(mismatch(format!(
                    "declared mercenary={mercenary} but the state reads {}",
                    state.is_mercenary_action,
                )));
            }
        }
    }
    Ok(())
}

/// One seat's president declaration on one side, including §5.1's
/// unreachable-post-Recovery rule and its Telephone exception.
fn check_president(
    declared: PresidentDecl,
    view_class: ViewClass,
    side: Side,
    state: &GameState,
    seat: PlayerId,
    role: &'static str,
    rules: &Rules,
) -> Result<(), FixtureError> {
    let mismatch = |detail: String| FixtureError::Declaration {
        clause: "president location",
        detail: format!("the {} side's {role} seat ({}): {detail}", side.name(), seat.0),
    };
    let player = state.player(seat);
    let area = president_area(player);
    match (declared, area) {
        (PresidentDecl::InHand, None) => Ok(()),
        (PresidentDecl::InHand, Some(a)) => Err(mismatch(format!(
            "declared InHand (§5.1's DEFAULT) but stands on area {}",
            a.0
        ))),
        (PresidentDecl::OnBoardEmptyArea, None) => {
            Err(mismatch("declared on-board at an empty area but sits in hand".to_string()))
        }
        (PresidentDecl::OnBoardEmptyArea, Some(a)) => {
            let held = player.assistants.in_area(a);
            if held == 0 {
                Ok(())
            } else {
                Err(mismatch(format!(
                    "declared on-board at an EMPTY area but holds {held} own assistant(s) at \
                     area {}",
                    a.0
                )))
            }
        }
        (PresidentDecl::OnBoardWithOwnAssistants { .. }, None) => Err(mismatch(
            "declared on-board with own assistants but sits in hand".to_string(),
        )),
        (PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted }, Some(a)) => {
            if player.assistants.in_area(a) == 0 {
                return Err(mismatch(format!(
                    "declared on-board WITH own assistants but holds none at area {}",
                    a.0
                )));
            }
            // The rule bites only at a hand-off: mid-turn, Recovery has not run.
            if matches!(view_class, ViewClass::HandOff) {
                if !telephone_excepted {
                    return Err(mismatch(format!(
                        "a HAND-OFF fixture stands this seat's president on area {} where it \
                         holds assistants — §5.1 calls that unreachable post-Recovery, \
                         Telephone excepted, and the exception is not claimed",
                        a.0
                    )));
                }
                if !owns_effect(state, seat, TechEffect::Telephone, rules) {
                    return Err(mismatch(
                        "claims §5.1's Telephone exception but the seat owns no Telephone \
                         technology"
                            .to_string(),
                    ));
                }
            }
            Ok(())
        }
    }
}

/// §5.1's REGISTERED DRAW CONSTRAINT, read off the declaration alone.
fn check_draws(draws: &AchievementDraws) -> Result<(), FixtureError> {
    let bad = |detail: String| FixtureError::Declaration { clause: "achievement draws", detail };
    for (i, drawn) in
        [draws.category_a, draws.category_b, draws.category_c].into_iter().enumerate()
    {
        if drawn.category() != declared_category(i) {
            return Err(bad(format!(
                "slot {i} declares {drawn:?}, which is category {:?}, but setup always deals \
                 category {:?} there",
                drawn.category(),
                declared_category(i),
            )));
        }
    }
    for moved in draws.goods_moved {
        if moved.category() != AchievementCategory::A {
            return Err(bad(format!(
                "{moved:?} is listed in the moved-goods set but is not a Category-A card"
            )));
        }
    }
    if draws.goods_moved.contains(&draws.category_a) {
        return Err(bad(format!(
            "the drawn Category-A card {:?} names a good this pair's recipe moves — §5.1's \
             registered draw constraint requires the drawn A-card's good to sit OUTSIDE the \
             moved set (the always-drawn A leg reads raw goods)",
            draws.category_a,
        )));
    }
    if draws.moves_yen && draws.category_b == AchievementType::Money10 {
        return Err(bad(
            "the recipe moves yen and the drawn Category-B card is Money10 — the leg would \
             shadow E1 (§2's veto identity), and §5.1's registered draw constraint forbids it"
                .to_string(),
        ));
    }
    Ok(())
}

/// The three declared cards against the ones one side actually carries.
fn check_drawn_cards(
    draws: &AchievementDraws,
    side: Side,
    state: &GameState,
) -> Result<(), FixtureError> {
    let declared = [draws.category_a, draws.category_b, draws.category_c];
    for (i, want) in declared.into_iter().enumerate() {
        let got = state.achievements[i].achievement_type;
        if got != want {
            return Err(FixtureError::Declaration {
                clause: "achievement draws",
                detail: format!(
                    "the {} side carries {got:?} in slot {i} but the registration declares \
                     {want:?}",
                    side.name(),
                ),
            });
        }
    }
    Ok(())
}

/// The technology declaration on one side.
fn check_techs(
    decl: &TechDeclaration,
    side: Side,
    state: &GameState,
    rules: &Rules,
) -> Result<(), FixtureError> {
    for player in &state.players {
        if let Some(max) = decl.max_owned {
            if player.technologies.len() > max {
                return Err(FixtureError::Declaration {
                    clause: "technology declaration",
                    detail: format!(
                        "the {} side's seat {} owns {} technologies, past the registered \
                         maximum of {max}",
                        side.name(),
                        player.id.0,
                        player.technologies.len(),
                    ),
                });
            }
        }
        for effect in decl.excluded_effects {
            if owns_effect(state, player.id, *effect, rules) {
                return Err(FixtureError::Declaration {
                    clause: "technology declaration",
                    detail: format!(
                        "the {} side's seat {} owns {effect:?}, which this registration \
                         excludes — it would change the marginal the pair calibrates",
                        side.name(),
                        player.id.0,
                    ),
                });
            }
        }
    }
    Ok(())
}

/// §5.1's stage-realization clauses, re-checked on the FINAL state rather than
/// on the base it was built from — a pair's own doctoring is exactly what could
/// move a clock back off its endpoint.
fn check_stage_clauses(
    declaration: &PairDeclaration,
    stage: StageEndpoint,
    side: Side,
    state: &GameState,
    view: &PlayerView,
    rules: &Rules,
) -> Result<(), FixtureError> {
    let bad = |clause: &'static str, detail: String| FixtureError::Declaration {
        clause,
        detail: format!("the {} side: {detail}", side.name()),
    };
    match stage {
        StageEndpoint::Early => {
            for player in &state.players {
                if player.shophouses.total_built() != 0 || player.trading_houses.total_built() != 0
                {
                    return Err(bad(
                        "p = 0 realization",
                        format!("seat {} has built; §5.1's p = 0 requires ZERO buildings", player.id.0),
                    ));
                }
                if declaration.early_hand_cap_deviation.is_none() {
                    let over_cap = player.order_cards_in_hand.len() > 1
                        || player
                            .order_cards_in_hand
                            .iter()
                            .any(|c| Some(*c) != player.starting_order);
                    if over_cap {
                        return Err(bad(
                            "p = 0 hand cap",
                            format!(
                                "seat {} holds {} order card(s) that are not its dealt starting \
                                 order; at a FULL deck no port take has happened, so §5.1 caps \
                                 the hand there — register an \
                                 `early_hand_cap_deviation` if the recipe genuinely needs more",
                                player.id.0,
                                player.order_cards_in_hand.len(),
                            ),
                        ));
                    }
                }
            }
            if state.church_track.total_assistants() != 0
                || state.customs_track.total_assistants() != 0
            {
                return Err(bad(
                    "p = 0 realization",
                    "a real pawn stands on a track; §5.1's p = 0 requires ZERO real pawns on \
                     both tracks (the D1 dummy is not counted)"
                        .to_string(),
                ));
            }
            // The deck clause is the one the endpoint arithmetic rests on, and
            // clause 7 has already checked the derived ppm; this names WHICH
            // clock a failure would have been about. The view is the caller's,
            // not a second `observe`: the two would be the same derivation and
            // only one of them would be the one clause 7 measured.
            let initial = eval::initial_order_deck_len(view, rules)
                .ok_or(FixtureError::OutOfStock("a derivable initial deck size"))?;
            if state.order_deck.len() != initial {
                return Err(bad(
                    "p = 0 realization",
                    format!(
                        "the deck holds {} of its initial {initial} cards; §5.1's p = 0 \
                         requires a FULL deck",
                        state.order_deck.len(),
                    ),
                ));
            }
        }
        StageEndpoint::Late => {
            if !state.order_deck.is_empty() {
                return Err(bad(
                    "p = FULL realization",
                    format!(
                        "the deck still holds {} card(s); §5.1's late recipe is DECK \
                         SATURATION",
                        state.order_deck.len(),
                    ),
                ));
            }
            if !port_is_full(state) && !state.end_game_triggered {
                return Err(bad(
                    "p = FULL realization",
                    "the port is not full and no end trigger is declared; §5.1's recipe is \
                     \"deck empty, port full OR the trigger declared\""
                        .to_string(),
                ));
            }
        }
    }
    Ok(())
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
    let state = keep_game_2p(seed, rules)?;
    assert_stage(&state, StageEndpoint::Early, rules)?;
    Ok(state)
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
    let mut state = keep_game_2p(seed, rules)?;
    saturate_deck_into_the_box(&mut state);
    assert_stage(&state, StageEndpoint::Late, rules)?;
    Ok(state)
}

/// Empty the order deck into `orders_removed`, leaving every other pile alone.
///
/// Exposed so a late pair that needs a different sink — cards in completed
/// piles or hands, the reachable shape real play produces — can build its own
/// construction on top of [`late_base_2p`]'s reasoning rather than re-deriving
/// it. Order conservation is exact: the total over deck, ports, hands,
/// completed piles and the box is unchanged (`game/invariants.rs:89–107`).
pub fn saturate_deck_into_the_box(state: &mut GameState) {
    let drained: Vec<_> = state.order_deck.drain(..).collect();
    state.orders_removed.extend(drained);
}

/// Assert a base state realizes its endpoint, against the ENGINE's own clock
/// derivation rather than a restatement of it.
///
/// [`eval::clock_ppm`] (`game/eval.rs:1956`) is the authority on all four
/// clocks — order-deck depletion, church occupancy, customs occupancy, and the
/// building race — and [`eval::progress_ppm`] (`game/eval.rs:1933`) is their
/// maximum. Checking the components as well as the maximum is what makes the
/// late base's claim specific: p = FULL by DECK saturation says the first clock
/// is full and the other three are at zero, so §5.1's "track/building deltas
/// inside a pair are absorbed by the max" is a fact about the base rather than
/// a hope about the pair.
fn assert_stage(
    state: &GameState,
    stage: StageEndpoint,
    rules: &Rules,
) -> Result<(), FixtureError> {
    let view = observe(state, state.current_player).map_err(|e| FixtureError::NotObservable {
        side: "base",
        detail: format!("{e}"),
    })?;
    let clocks = eval::clock_ppm(&view, rules);
    let expected = match stage {
        StageEndpoint::Early => [0, 0, 0, 0],
        StageEndpoint::Late => [PROGRESS_FULL_PPM, 0, 0, 0],
    };
    if clocks != expected {
        return Err(FixtureError::Declaration {
            clause: "stage realization",
            detail: format!(
                "the {} base reads clocks {clocks:?} (deck, church, customs, building) but \
                 §5.1's construction is {expected:?}",
                stage.name(),
            ),
        });
    }
    let ppm = eval::progress_ppm(&view, rules);
    if ppm != stage.ppm() {
        return Err(FixtureError::Declaration {
            clause: "stage realization",
            detail: format!(
                "the {} base reads {ppm} ppm, not {}",
                stage.name(),
                stage.ppm()
            ),
        });
    }
    check_invariants(state, rules)
        .map_err(|detail| FixtureError::NotEngineValid { side: "base", detail })
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
        "PRES|{id}|greater:subject={}|greater:opponent={}|lesser:subject={}|lesser:opponent={}\n",
        pair.presidents.greater.subject.canonical(),
        pair.presidents.greater.opponent.canonical(),
        pair.presidents.lesser.subject.canonical(),
        pair.presidents.lesser.opponent.canonical()
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
///
/// | v | change |
/// |---|--------|
/// | 1 | the initial format |
/// | 2 | the PRES line carries all FOUR president declarations (per side × per seat), not two — the M1 fix wave's [`SidedPresidents`] split |
///
/// The v2 bump is INERT today and is taken anyway. With [`REGISTERED`] empty
/// there is no PRES line to write, so the emitted bytes were identical either
/// way and no recorded digest is invalidated — but the same registration WOULD
/// serialize differently under the two formats, and leaving the stamp at 1
/// would make "v1" mean two different things. That is precisely the silent
/// drift the version exists to prevent, so it moves while the move is free.
pub const DIGEST_FORMAT_VERSION: u32 = 2;

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
    // `to_vec` cannot fail on a `GameState`: every field is a plain serde
    // derive over owned data with no non-string map keys. The fallback is a
    // NAMED sentinel rather than a panic or a silent zero, so a future field
    // that broke the encoding would print as broken instead of colliding with
    // every other broken fingerprint.
    match serde_json::to_vec(state) {
        Ok(bytes) => format!("{:016x}", fnv1a(&bytes)),
        Err(_) => "unencodable-state".to_string(),
    }
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
    let d = fixture.declaration;
    let stage = fixture.stage;
    let mut out = String::new();

    out.push_str(&format!("pair                {}  {}\n", d.id, d.name));
    out.push_str(&format!("family              {}  ({})\n", d.family, d.role.canonical()));
    out.push_str(&format!("greater             {}\n", d.greater_side));
    out.push_str(&format!("lesser              {}\n", d.lesser_side));
    out.push_str(&format!(
        "stage               {} (p = {} ppm; realized {})\n",
        stage.name(),
        stage.ppm(),
        fixture.progress_ppm(rules),
    ));
    out.push_str(&format!(
        "endpoints           {}\n",
        d.stages.iter().map(|s| s.name()).collect::<Vec<_>>().join("+"),
    ));
    out.push_str(&format!("view-class          {}\n", d.view_class.canonical()));
    out.push_str(&format!(
        "margin-class        {} (margin here: {})\n",
        d.margin.canonical(),
        d.margin.margin_at(stage).map_or_else(|| "-".to_string(), |m| m.to_string()),
    ));
    out.push_str(&format!("carrier             {}\n", d.carrier.map_or("-", |c| c.label())));
    out.push_str(&format!(
        "presidents/greater  subject={} opponent={}\n",
        d.presidents.greater.subject.canonical(),
        d.presidents.greater.opponent.canonical(),
    ));
    out.push_str(&format!(
        "presidents/lesser   subject={} opponent={}\n",
        d.presidents.lesser.subject.canonical(),
        d.presidents.lesser.opponent.canonical(),
    ));
    out.push_str(&format!(
        "draws               a={:?} b={:?} c={:?} (moved-goods {:?}, moves-yen {})\n",
        d.draws.category_a,
        d.draws.category_b,
        d.draws.category_c,
        d.draws.goods_moved,
        d.draws.moves_yen,
    ));
    out.push_str(&format!(
        "technologies        excluded={:?} max-owned={}\n",
        d.techs.excluded_effects,
        d.techs.max_owned.map_or_else(|| "-".to_string(), |n| n.to_string()),
    ));
    out.push_str(&format!("layout              {}\n", d.layout.canonical()));
    for (key, value) in d.parameters {
        out.push_str(&format!("parameter           {key} = {value}\n"));
    }
    if let Some(reason) = d.early_hand_cap_deviation {
        out.push_str(&format!("hand-cap deviation  {reason}\n"));
    }
    out.push_str(&format!("subject seat        {}\n", fixture.subject.0));
    out.push_str(&format!(
        "state bytes         greater={} lesser={}\n",
        state_fingerprint(&fixture.greater),
        state_fingerprint(&fixture.lesser),
    ));
    out.push_str(&format!("package digest      {}\n", package_digest()));

    // The delta table. Incumbents carry the relative form on both sides and its
    // difference; candidates carry the same shape once step 3 wires `columns`.
    let g_rel = relative_vector(&fixture.greater_view, fixture.subject, rules);
    let l_rel = relative_vector(&fixture.lesser_view, fixture.subject, rules);
    out.push('\n');
    out.push_str(&push_row("row", "feature", "greater", "lesser", "delta", "expected-nonzero"));
    for row in FeatureId::all() {
        let listed = if d.expected_nonzero.contains(&row) { "yes" } else { "" };
        match row {
            FeatureId::Incumbent(f) => {
                let g = g_rel[f.index()];
                let l = l_rel[f.index()];
                out.push_str(&push_row(
                    &row.index().to_string(),
                    f.name(),
                    &g.to_string(),
                    &l.to_string(),
                    &(g - l).to_string(),
                    listed,
                ));
            }
            FeatureId::Candidate(c) => {
                let cell = |view: &PlayerView| -> Option<i64> {
                    let f = columns?;
                    let me = view.players.iter().find(|p| p.id == fixture.subject)?;
                    let mine = f(view, me, c, rules)?;
                    let opponent =
                        view.players.iter().find(|p| p.id != fixture.subject && !p.id.is_dummy());
                    let theirs = match opponent {
                        Some(o) => f(view, o, c, rules)?,
                        None => 0,
                    };
                    Some(mine - theirs)
                };
                match (cell(&fixture.greater_view), cell(&fixture.lesser_view)) {
                    (Some(g), Some(l)) => out.push_str(&push_row(
                        &row.index().to_string(),
                        c.name(),
                        &g.to_string(),
                        &l.to_string(),
                        &(g - l).to_string(),
                        listed,
                    )),
                    // ONE marker per row, in the DELTA column: the delta is the
                    // number a solve row reads, so that is where "not built
                    // yet" belongs, and the value columns print `-` rather
                    // than a `0` that would read as a measurement.
                    _ => out.push_str(&push_row(
                        &row.index().to_string(),
                        c.name(),
                        "-",
                        "-",
                        CANDIDATE_PENDING,
                        listed,
                    )),
                }
            }
        }
    }

    if !d.notes.is_empty() {
        out.push('\n');
        for note in d.notes {
            out.push_str(&format!("note                {note}\n"));
        }
    }
    out
}

/// One line of the delta table.
///
/// Every cell is a STRING, and every row goes through here, so the header, the
/// incumbent rows and the candidate rows cannot drift into different widths —
/// which matters because the candidate cell is [`CANDIDATE_PENDING`] rather
/// than a number and a per-arm `format!` would silently ragged-edge the table
/// exactly where a reader is scanning for what is not built yet. Trailing
/// whitespace is trimmed so an unlisted row does not end in spaces.
fn push_row(
    index: &str,
    feature: &str,
    greater: &str,
    lesser: &str,
    delta: &str,
    listed: &str,
) -> String {
    let line = format!(
        "{index:<4} {feature:<28} {greater:>16} {lesser:>16} {delta:>16}  {listed}"
    );
    format!("{}\n", line.trim_end())
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
        .is_none_or(|p| p.slots.iter().all(Option::is_some));
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

// ---------------------------------------------------------------------------
// The constructions (§5.2 families 1 and 11)
// ---------------------------------------------------------------------------

/// The declaration registered under `id`, or `None`.
pub fn registered(id: &str) -> Option<&'static PairDeclaration> {
    match id {
        "1(a)" => Some(&PAIR_1A),
        "1(b)" => Some(&PAIR_1B),
        "1(c)" => Some(&PAIR_1C),
        "1(d)" => Some(&PAIR_1D),
        "1(e)" => Some(&PAIR_1E),
        "2(a)" => Some(&PAIR_2A),
        "2(b)" => Some(&PAIR_2B),
        "2(c)" => Some(&PAIR_2C),
        "2(d)" => Some(&PAIR_2D),
        "3(a)" => Some(&PAIR_3A),
        "3(b)" => Some(&PAIR_3B),
        "3(c)" => Some(&PAIR_3C),
        "3(d)" => Some(&PAIR_3D),
        "6(a)" => Some(&PAIR_6A),
        "6(b)" => Some(&PAIR_6B),
        "6(c)" => Some(&PAIR_6C),
        "6(d-church)" => Some(&PAIR_6D_CHURCH),
        "6(d-customs)" => Some(&PAIR_6D_CUSTOMS),
        "7" => Some(&PAIR_7),
        "8" => Some(&PAIR_8),
        "10(a)" => Some(&PAIR_10A),
        "10(b)" => Some(&PAIR_10B),
        "10(c)" => Some(&PAIR_10C),
        "10(d)" => Some(&PAIR_10D),
        "11(a)" => Some(&PAIR_11A),
        "11(b)" => Some(&PAIR_11B),
        "11(c)" => Some(&PAIR_11C),
        "11(d)" => Some(&PAIR_11D),
        "11(e)" => Some(&PAIR_11E),
        _ => None,
    }
}

/// **Realize one registered pair at one declared endpoint**, admitted through
/// [`PairFixture::new`]'s full clause list.
///
/// # How these states are reached, and what that costs
///
/// By DOCUMENTED DIRECT SURGERY on a stage base, not by replaying moves. The
/// reason is the shape §5.1 asks for: a pair is two states that differ in ONE
/// declared way at ONE exact `progress_ppm`, from the SAME mover's seat, and a
/// move sequence reaching either one drags a turn's worth of incidental
/// differences — a turn counter, a spent president move, whatever the opponent
/// did with its own turn — into a comparison that is supposed to isolate a
/// single axis. Every family-11 pair makes that concrete: its two sides are the
/// same position with one different ACTIVATION CHOICE behind them, and no move
/// sequence produces both from one root.
///
/// What replaces reachability is CHECKING. Every side goes through
/// `check_invariants` (clause 3), every declared fact is re-read off the
/// constructed state (clauses 6–12), and every realized incumbent row is
/// checked against the registration (clause 13). Where a construction stands in
/// for an engine effect it uses the ENGINE's own function rather than a
/// restatement — `remove_and_slide_lab`/`refill_lab` for 11(d)'s purchase, the
/// port's own total-compaction shape and `check_end_game_triggers` for 11(e)'s
/// take — so the byte the fixture carries is the byte a real action would have
/// produced.
pub fn build_registered_pair(
    declaration: &'static PairDeclaration,
    stage: StageEndpoint,
    rules: &Rules,
) -> Result<PairFixture, FixtureError> {
    match declaration.id {
        "1(a)" => build_1a(stage, rules),
        "1(b)" => build_1b(stage, rules),
        "1(c)" => build_1c(stage, rules),
        "1(d)" => build_1d(stage, rules),
        "1(e)" => build_1e(stage, rules),
        "2(a)" => build_2a(stage, rules),
        "2(b)" => build_2b(stage, rules),
        "2(c)" => build_2c(stage, rules),
        "2(d)" => build_2d(stage, rules),
        "3(a)" => build_3a(stage, rules),
        "3(b)" => build_3b(stage, rules),
        "3(c)" => build_3c(stage, rules),
        "3(d)" => build_3d(stage, rules),
        "6(a)" => build_6a(stage, rules),
        "6(b)" => build_6b(stage, rules),
        "6(c)" => build_6c(stage, rules),
        "6(d-church)" => build_6d_church(stage, rules),
        "6(d-customs)" => build_6d_customs(stage, rules),
        "7" => build_7(stage, rules),
        "8" => build_8(stage, rules),
        "10(a)" => build_10a(stage, rules),
        "10(b)" => build_10b(stage, rules),
        "10(c)" => build_10c(stage, rules),
        "10(d)" => build_10d(stage, rules),
        "11(a)" => build_11a(stage, rules),
        "11(b)" => build_11b(stage, rules),
        "11(c)" => build_11c(stage, rules),
        "11(d)" => build_11d(stage, rules),
        "11(e)" => build_11e(stage, rules),
        _ => Err(FixtureError::Declaration {
            clause: "registered construction",
            detail: format!("{} has a declaration but no construction", declaration.id),
        }),
    }
}

/// A stage base with its two seats named — the common prefix of every
/// construction here.
struct FamilyBase {
    state: GameState,
    subject: PlayerId,
    opponent: PlayerId,
}

/// [`stage_base_2p`] at [`FIXTURE_SEED`], optionally re-typed onto a hand-built
/// layout, carrying the registration's declared draws and the subject's
/// registered order card.
fn family_base(
    stage: StageEndpoint,
    declaration: &PairDeclaration,
    layout: Option<[AreaType; TWO_PLAYER_AREAS]>,
    rules: &Rules,
) -> Result<FamilyBase, FixtureError> {
    family_base_with_order(stage, declaration, layout, FIXTURE_ORDER_CARD, rules)
}

/// [`family_base`] with the subject's starting order NAMED.
///
/// Family 6's fulfillment pairs each register their own card, and §5.1's
/// p = 0 hand cap is what forces the card into the STARTING-ORDER slot rather
/// than beside it: at a full deck no port take has happened, so a seat may hold
/// exactly the card it was dealt. Swapping the deal is therefore the only way
/// to register a fulfilled card at the early endpoint without registering a
/// hand-cap deviation as well.
fn family_base_with_order(
    stage: StageEndpoint,
    declaration: &PairDeclaration,
    layout: Option<[AreaType; TWO_PLAYER_AREAS]>,
    order_card: OrderCardId,
    rules: &Rules,
) -> Result<FamilyBase, FixtureError> {
    let mut state = stage_base_2p(stage, FIXTURE_SEED, rules)?;
    if let Some(types) = layout {
        retype_board_2p(&mut state, types)?;
    }
    let subject = state.current_player;
    let opponent = other_seat_2p(&state, subject)?;
    set_declared_draws(&mut state, &declaration.draws);
    give_starting_order(&mut state, subject, order_card)?;
    Ok(FamilyBase { state, subject, opponent })
}

// --- family 1 -------------------------------------------------------------

fn build_1a(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let base = family_base(stage, &PAIR_1A, None, rules)?;
    let silk = find_area_of_type(&base.state, AreaType::SilkMill)?;
    let copper = find_area_of_type(&base.state, AreaType::CopperMine)?;
    let mut greater = base.state.clone();
    place_from_hand(&mut greater, base.subject, silk, 1)?;
    let mut lesser = base.state;
    place_from_hand(&mut lesser, base.subject, copper, 1)?;
    PairFixture::new(&PAIR_1A, stage, greater, lesser, base.subject, rules)
}

fn build_1b(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let base = family_base(stage, &PAIR_1B, None, rules)?;
    let silk = find_area_of_type(&base.state, AreaType::SilkMill)?;
    let copper = find_area_of_type(&base.state, AreaType::CopperMine)?;
    // §5.2's "the two areas non-adjacent" is what holds the registered ΔS4 at
    // −1: if the Silk Mill touched the Copper Mine, the spread side's ring
    // would sum 1 + 1 = 2 and match the stack side exactly.
    if base.state.board_layout.adjacent_areas(silk).contains(&copper) {
        return Err(FixtureError::Declaration {
            clause: "non-adjacent areas",
            detail: format!(
                "1(b) registers the Silk Mill (area {}) and the Copper Mine (area {}) as \
                 NON-ADJACENT, and the board generated at seed {FIXTURE_SEED:#x} makes them \
                 neighbours; the registered ΔS4 = −1 does not hold on that board",
                silk.0, copper.0,
            ),
        });
    }
    let mut greater = base.state.clone();
    place_from_hand(&mut greater, base.subject, silk, 1)?;
    place_from_hand(&mut greater, base.subject, copper, 1)?;
    let mut lesser = base.state;
    place_from_hand(&mut lesser, base.subject, copper, 2)?;
    PairFixture::new(&PAIR_1B, stage, greater, lesser, base.subject, rules)
}

fn build_1c(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let base = family_base(stage, &PAIR_1C, Some(RING_LAYOUT), rules)?;
    let mut shared = base.state;
    place_from_hand(&mut shared, base.subject, RING_ASSISTANT_AREA, 2)?;
    let mut greater = shared.clone();
    greater.player_mut(base.subject).president_location =
        PresidentLocation::OnBoard(RING_NEAR_PRESIDENT_AREA);
    let mut lesser = shared;
    lesser.player_mut(base.subject).president_location =
        PresidentLocation::OnBoard(RING_FAR_PRESIDENT_AREA);
    PairFixture::new(&PAIR_1C, stage, greater, lesser, base.subject, rules)
}

fn build_1d(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_1D, None, rules)?;
    // The registered stocks. All three zeroes are load-bearing: tea, seafood
    // and a second yen would each buy another distinct Church payment type and
    // carry the realizable total past the registered faith 4.
    {
        let p = base.state.player_mut(base.subject);
        p.copper = 1;
        p.silk = 1;
        p.tea = 0;
        p.seafood = 0;
        p.imported_goods = 0;
        p.money = 1;
    }
    let church = find_area_of_type(&base.state, AreaType::Church)?;
    let copper = find_area_of_type(&base.state, AreaType::CopperMine)?;
    let mut greater = base.state.clone();
    place_from_hand(&mut greater, base.subject, church, 1)?;
    let mut lesser = base.state;
    place_from_hand(&mut lesser, base.subject, copper, 1)?;
    PairFixture::new(&PAIR_1D, stage, greater, lesser, base.subject, rules)
}

fn build_1e(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let base = family_base(stage, &PAIR_1E, Some(RING_LAYOUT), rules)?;
    let mut shared = base.state;
    for area in RING_CHAIN_AREAS {
        place_from_hand(&mut shared, base.subject, area, 1)?;
    }
    let mut greater = shared.clone();
    greater.player_mut(base.subject).president_location =
        PresidentLocation::OnBoard(RING_NEAR_PRESIDENT_AREA);
    let mut lesser = shared;
    lesser.player_mut(base.subject).president_location =
        PresidentLocation::OnBoard(RING_DISCONNECTED_PRESIDENT_AREA);
    PairFixture::new(&PAIR_1E, stage, greater, lesser, base.subject, rules)
}

// --- family 3 -------------------------------------------------------------

fn build_3a(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let base = family_base(stage, &PAIR_3A, None, rules)?;
    // No placement, no stock write, no draw beyond the registered three: §5.2's
    // "all else identical" is honoured by touching exactly ONE field.
    let mut greater = base.state.clone();
    greater.player_mut(base.subject).money = FAMILY_3A_GREATER_MONEY;
    let mut lesser = base.state;
    lesser.player_mut(base.subject).money = FAMILY_3A_LESSER_MONEY;
    PairFixture::new(&PAIR_3A, stage, greater, lesser, base.subject, rules)
}

fn build_3b(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_3B, None, rules)?;
    // §5.2's "hand constructed empty", built the only way p = 0 admits: the
    // dealt starting order is stocked for and then FULFILLED through the
    // engine, which moves it to the completed pile (invariant I11's second
    // arm) and moves none of `clock_ppm`'s four clocks. The registered stocks
    // are written afterwards, on top of the zeroes the fulfillment leaves.
    stock_exactly_the_requirements(&mut base.state, base.subject, FIXTURE_ORDER_CARD, rules)?;
    let emptied = fulfill_through_the_engine(&base.state, FIXTURE_ORDER_CARD, rules)?;
    let mut greater = emptied.clone();
    set_trade_goods(&mut greater, base.subject, [0, 0, FAMILY_3B_GREATER_TEA, 0]);
    let mut lesser = emptied;
    set_trade_goods(&mut lesser, base.subject, [0, 0, FAMILY_3B_LESSER_TEA, 0]);
    PairFixture::new(&PAIR_3B, stage, greater, lesser, base.subject, rules)
}

fn build_3c(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_3C, None, rules)?;
    set_trade_goods(&mut base.state, base.subject, FAMILY_3_STOCKS);
    let bank = find_area_of_type(&base.state, AreaType::Bank)?;
    let copper = find_area_of_type(&base.state, AreaType::CopperMine)?;
    let mut greater = base.state.clone();
    place_from_hand(&mut greater, base.subject, bank, 1)?;
    let mut lesser = base.state;
    place_from_hand(&mut lesser, base.subject, copper, 1)?;
    PairFixture::new(&PAIR_3C, stage, greater, lesser, base.subject, rules)
}

fn build_3d(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_3D, None, rules)?;
    set_trade_goods(&mut base.state, base.subject, FAMILY_3_STOCKS);
    let ea = find_area_of_type(&base.state, AreaType::EmploymentAgency)?;
    let copper = find_area_of_type(&base.state, AreaType::CopperMine)?;
    // The warehouse is left exactly as setup dealt it — §5.2's registered
    // "warehouse non-empty", which is what makes the POWER CAP the binding
    // side of S6's min rather than the basket.
    let mut greater = base.state.clone();
    place_from_hand(&mut greater, base.subject, ea, 1)?;
    let mut lesser = base.state;
    place_from_hand(&mut lesser, base.subject, copper, 1)?;
    PairFixture::new(&PAIR_3D, stage, greater, lesser, base.subject, rules)
}

// --- family 8 -------------------------------------------------------------

fn build_8(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_8, Some(FAMILY_8_LAYOUT), rules)?;
    // The registered purse: nothing, which is what makes the Laboratory pawn
    // exhausted rather than merely idle.
    base.state.player_mut(base.subject).money = FAMILY_8_SUBJECT_MONEY;

    // The RECALLED side is the base itself — president in hand, no pawn
    // standing. The STRANDED side is the same seat one turn earlier in its
    // history, with the two pawns out and the president among them.
    let greater = base.state.clone();
    let mut lesser = base.state;
    for area in FAMILY_8_STRANDED_AREAS {
        place_from_hand(&mut lesser, base.subject, area, 1)?;
    }
    lesser.player_mut(base.subject).president_location =
        PresidentLocation::OnBoard(FAMILY_8_PRESIDENT_AREA);

    // And the two are one ENGINE recall apart, checked rather than asserted.
    let recalled = recall_through_the_engine(&lesser, base.subject, rules)?;
    {
        let want = greater.player(base.subject);
        if recalled.president_location != want.president_location
            || recalled.assistants != want.assistants
        {
            return Err(FixtureError::Declaration {
                clause: "engine-driven recall",
                detail: format!(
                    "the engine's own `ReturnPresidentToHand` leaves the seat at {:?} with \
                     {:?}, and the recalled side carries {:?} with {:?}",
                    recalled.president_location,
                    recalled.assistants,
                    want.president_location,
                    want.assistants,
                ),
            });
        }
    }
    PairFixture::new(&PAIR_8, stage, greater, lesser, base.subject, rules)
}

// --- family 10 ------------------------------------------------------------

fn build_10a(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_10A, None, rules)?;
    set_trade_goods(&mut base.state, base.subject, FAMILY_10A_STOCKS);
    let port = find_area_of_type(&base.state, AreaType::PortA)?;
    place_from_hand(&mut base.state, base.subject, port, 1)?;
    stock_port_a(&mut base.state, FAMILY_10A_PORT_OFFERS)?;

    let greater = base.state.clone();
    let mut lesser = base.state;
    // The PERMUTATION, and nothing else: the two LOW offers and the two HIGH
    // offers trade places, so the located set is fixed and only the zone each
    // card sits in moves.
    lesser.port_state.port_a.slots.swap(0, 4);
    lesser.port_state.port_a.slots.swap(1, 5);
    PairFixture::new(&PAIR_10A, stage, greater, lesser, base.subject, rules)
}

fn build_10b(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_10B, None, rules)?;
    let lab = find_area_of_type(&base.state, AreaType::LaboratoryA)?;
    place_from_hand(&mut base.state, base.subject, lab, 1)?;
    {
        let p = base.state.player_mut(base.subject);
        p.money = FAMILY_10B_MONEY;
        p.imported_goods = 0;
    }
    stock_lab_a(&mut base.state, FAMILY_10B_LAB_STOCK, rules)?;

    let greater = base.state.clone();
    let mut lesser = base.state;
    lesser.lab_state.lab_a.slots.swap(FAMILY_10B_CHEAP_SLOT, FAMILY_10B_DEAR_SLOT);
    PairFixture::new(&PAIR_10B, stage, greater, lesser, base.subject, rules)
}

fn build_10c(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_10C, None, rules)?;
    let church = find_area_of_type(&base.state, AreaType::Church)?;
    place_from_hand(&mut base.state, base.subject, church, 1)?;
    // 3(b)'s hand-emptying device, for a different reason: ANY held order
    // reads a different O2 against two stock vectors that differ, and the
    // diagnostic's whole claim is that NO row moves.
    stock_exactly_the_requirements(&mut base.state, base.subject, FIXTURE_ORDER_CARD, rules)?;
    let emptied = fulfill_through_the_engine(&base.state, FIXTURE_ORDER_CARD, rules)?;

    let mut greater = emptied.clone();
    set_trade_goods(&mut greater, base.subject, FAMILY_10C_SPREAD_STOCKS);
    let mut lesser = emptied;
    set_trade_goods(&mut lesser, base.subject, FAMILY_10C_CONCENTRATED_STOCKS);
    for side in [&mut greater, &mut lesser] {
        let p = side.player_mut(base.subject);
        p.money = FAMILY_10C_MONEY;
        p.imported_goods = FAMILY_10C_IMPORTS;
    }
    PairFixture::new(&PAIR_10C, stage, greater, lesser, base.subject, rules)
}

fn build_10d(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_10D, None, rules)?;
    // BOTH registered cards go off the face-up offers BEFORE either is dealt.
    // Port offers are LOCATED for O1's pool and deck and box cards are not, so
    // dealing one out of a port slot would move the pool underneath a pair
    // whose registered collateral is none.
    park_order_off_the_port(&mut base.state, FAMILY_10D_MATCHED_CARD)?;
    park_order_off_the_port(&mut base.state, FAMILY_10D_UNMATCHED_CARD)?;
    let fishing = find_area_of_type(&base.state, AreaType::FishingGround)?;
    place_from_hand(&mut base.state, base.subject, fishing, 1)?;
    set_trade_goods(&mut base.state, base.subject, [0, 0, 0, 0]);
    base.state.player_mut(base.subject).imported_goods = 0;

    let mut greater = base.state.clone();
    give_starting_order(&mut greater, base.subject, FAMILY_10D_MATCHED_CARD)?;
    let mut lesser = base.state;
    give_starting_order(&mut lesser, base.subject, FAMILY_10D_UNMATCHED_CARD)?;
    PairFixture::new(&PAIR_10D, stage, greater, lesser, base.subject, rules)
}

// --- families 2, 6 and 7: the engine-driven turn -------------------------

/// Play the subject's OWN TURN, through the engine's CHECKED boundary, from a
/// hand-off state to the instant the area action resolves.
///
/// # Why these pairs are driven rather than doctored
///
/// [`build_registered_pair`]'s doc explains why family 11 cannot be reached by
/// replay — its two sides are one position behind two different activation
/// choices. The acting-class pairs registered here are the opposite case: each
/// side IS the image of a real move sequence from one root, and `apply_move`
/// refuses anything absent from `get_legal_moves`, so driving them is a
/// LEGALITY PROOF the frame's clause list cannot otherwise give. Four moves do
/// it — end the additional-action phase, make the registered placement, bring
/// the president in from hand (which is what writes `current_action_power`,
/// `game/apply_move.rs:591`), and resolve the registered action into
/// [`TurnPhase::FivePowerBonus`].
///
/// The returned state may still carry a pending-choice queue: a church
/// placement queues its movement riders, a customs payment its space choice
/// and then that space's own riders. Resolving them is the CALLER's, because
/// which resolution a pair takes is a registered fact about the pair rather
/// than a property of the drive.
fn drive_to_resolved_area_action(
    state: &GameState,
    placement: Move,
    destination: AreaId,
    action: AreaAction,
    rules: &Rules,
) -> Result<GameState, FixtureError> {
    let script = [
        Move::EndAdditionalPhase,
        placement,
        Move::PlacePresident { destination },
        Move::ExecuteAreaAction { action },
    ];
    let mut out = state.clone();
    for mv in script {
        out = crate::game::apply_move(&out, &mv, rules).map_err(|e| FixtureError::Declaration {
            clause: "engine-driven activation",
            detail: format!("the engine refused {mv:?}: {e}"),
        })?;
    }
    Ok(out)
}

/// Answer the queue head with the pair's REGISTERED resolution, refusing if
/// the head is not the choice the registration expects.
///
/// The guard is the point: a fixture that answered whatever happened to be
/// queued would silently register a different fixture the day an engine change
/// queued something else first.
fn resolve_registered_choice(
    state: &mut GameState,
    expected: PendingChoice,
    resolution: Move,
    rules: &Rules,
) -> Result<(), FixtureError> {
    if state.pending_choices.first() != Some(&expected) {
        return Err(FixtureError::Declaration {
            clause: "registered choice resolution",
            detail: format!(
                "the registration answers {expected:?} but the queue head is {:?}",
                state.pending_choices.first(),
            ),
        });
    }
    *state = crate::game::apply_move(state, &resolution, rules).map_err(|e| {
        FixtureError::Declaration {
            clause: "registered choice resolution",
            detail: format!("the engine refused {resolution:?}: {e}"),
        }
    })?;
    Ok(())
}

/// DECLINE every remaining queued choice, and refuse unless EXACTLY the
/// registered number were there to decline.
///
/// The count is a registered parameter of both 6(d) pairs — the church space's
/// movement riders, the customs space's warehouse takes — so checking it here
/// is what keeps those parameters facts rather than annotations: a space whose
/// rider table changed would otherwise be declined away silently.
///
/// Only the two declinable heads are handled. `ChurchSpace`, `CustomsSpace`,
/// `AnyGood`, `RecoveryLeave` and `StationPlace` have no Decline arm at all,
/// so a pair whose action queues one registers the answer through
/// [`resolve_registered_choice`] instead — and meeting one here is a typed
/// refusal naming it rather than a silent wrong resolution.
fn decline_the_registered_riders(
    state: &mut GameState,
    expected: usize,
    rules: &Rules,
) -> Result<(), FixtureError> {
    let declined = decline_every_queued_choice(state, rules)?;
    if declined != expected {
        return Err(FixtureError::Declaration {
            clause: "declined choice riders",
            detail: format!(
                "the registration declines {expected} rider(s) and the action queued \
                 {declined}"
            ),
        });
    }
    Ok(())
}

/// The loop [`decline_the_registered_riders`] counts.
fn decline_every_queued_choice(
    state: &mut GameState,
    rules: &Rules,
) -> Result<usize, FixtureError> {
    let mut declined = 0;
    while let Some(head) = state.pending_choices.first() {
        let resolution = match head {
            PendingChoice::AssistantMove => Move::DeclineAssistantMove,
            PendingChoice::WarehouseTake => Move::DeclineWarehouseTake,
            other => {
                return Err(FixtureError::Declaration {
                    clause: "declined choice riders",
                    detail: format!(
                        "{other:?} heads the queue and carries no Decline arm; a pair whose \
                         action queues one registers its resolution"
                    ),
                })
            }
        };
        *state =
            crate::game::apply_move(state, &resolution, rules).map_err(|e| {
                FixtureError::Declaration {
                    clause: "declined choice riders",
                    detail: format!("the engine refused {resolution:?}: {e}"),
                }
            })?;
        declined += 1;
    }
    Ok(declined)
}

// --- family 2 -------------------------------------------------------------

fn build_2a(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_2A, None, rules)?;
    let silk = find_area_of_type(&base.state, AreaType::SilkMill)?;
    set_trade_goods(&mut base.state, base.subject, [0, 0, 0, 0]);
    // TWO pawns in one area is Option B's encoding, and two plus the
    // president's +1 is the realizable power 3 the registered yield rests on.
    let greater = drive_to_resolved_area_action(
        &base.state,
        Move::PlaceTwoSameArea { area: silk, adjacent_bonus: None },
        silk,
        AreaAction::ProduceSilk,
        rules,
    )?;
    let mut lesser = greater.clone();
    // The ONE declared difference, taken back off the engine-produced side —
    // read from the shipped table rather than restated.
    lesser.player_mut(base.subject).silk -= u32::from(crate::game::actions::silk_mill_reward(3));
    PairFixture::new(&PAIR_2A, stage, greater, lesser, base.subject, rules)
}

fn build_2b(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let base = family_base(stage, &PAIR_2B, None, rules)?;
    let areas = productive_trio_areas(&base.state)?;
    let lesser = base.state.clone();
    let mut greater = base.state;
    for area in areas {
        place_from_warehouse(&mut greater, base.subject, area, 1)?;
    }
    PairFixture::new(&PAIR_2B, stage, greater, lesser, base.subject, rules)
}

fn build_2c(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let base = family_base(stage, &PAIR_2C, None, rules)?;
    let areas = productive_trio_areas(&base.state)?;
    let lesser = base.state.clone();
    let mut greater = base.state;
    // 2(b)'s deployment, paid for out of the HAND — the one difference
    // between the gate and the diagnostic.
    for area in areas {
        place_from_hand(&mut greater, base.subject, area, 1)?;
    }
    PairFixture::new(&PAIR_2C, stage, greater, lesser, base.subject, rules)
}

fn build_2d(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_2D, None, rules)?;
    let ea = find_area_of_type(&base.state, AreaType::EmploymentAgency)?;
    place_from_hand(&mut base.state, base.subject, ea, FAMILY_2D_EA_PAWNS)?;
    {
        let p = base.state.player_mut(base.subject);
        // The warehouse drains to the registered three assistants; the rest
        // go to HAND, which is the only direction that keeps the twenty-pawn
        // total and moves no other counter this pair reads.
        let surplus = p
            .assistants
            .in_warehouse
            .checked_sub(FAMILY_2D_WAREHOUSE_ASSISTANTS)
            .ok_or(FixtureError::OutOfStock("warehouse assistants to drain to the registered three"))?;
        p.assistants.in_warehouse = FAMILY_2D_WAREHOUSE_ASSISTANTS;
        p.assistants.in_hand += surplus;
        // Both building-token classes leave the warehouse entirely: the EA's
        // basket is a SUM over all three, and any token left behind would
        // carry it past the power cap and pin S6 on both sides.
        p.trading_houses.in_hand += p.trading_houses.in_warehouse;
        p.trading_houses.in_warehouse = 0;
        p.shophouses.in_hand += p.shophouses.in_warehouse;
        p.shophouses.in_warehouse = 0;
    }
    let greater = base.state.clone();
    let mut lesser = base.state;
    {
        // The LESSER side is the one token short of ready — the whole pair.
        let p = lesser.player_mut(base.subject);
        p.shophouses.in_hand = p
            .shophouses
            .in_hand
            .checked_sub(1)
            .ok_or(FixtureError::OutOfStock("a shophouse token to leave in the warehouse"))?;
        p.shophouses.in_warehouse += 1;
    }
    PairFixture::new(&PAIR_2D, stage, greater, lesser, base.subject, rules)
}

// --- family 6 -------------------------------------------------------------

fn build_6a(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base_with_order(stage, &PAIR_6A, None, FIXTURE_FULFILLED_CARD, rules)?;
    stock_exactly_the_requirements(&mut base.state, base.subject, FIXTURE_FULFILLED_CARD, rules)?;
    let lesser = base.state;
    // Through the engine, so the queued warehouse icon, the completed pile and
    // `check_mercenary_eligibility`'s verdict are the engine's own bytes.
    let greater = fulfill_through_the_engine(&lesser, FIXTURE_FULFILLED_CARD, rules)?;
    PairFixture::new(&PAIR_6A, stage, greater, lesser, base.subject, rules)
}

fn build_6b(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_6B, None, rules)?;
    set_trade_goods(&mut base.state, base.subject, [0, 0, 0, 0]);
    {
        // The threshold is read off the engine's own progress arm rather than
        // written as a literal, so a re-priced card moves the fixture with it.
        let (_, needed) = crate::game::achievement_progress(
            &base.state,
            base.subject,
            AchievementType::Money10,
        );
        base.state.player_mut(base.subject).money = needed;
    }
    let lesser = base.state;
    let greater = crate::game::apply_move(
        &lesser,
        &Move::FulfillAchievement { achievement: AchievementType::Money10 },
        rules,
    )
    .map_err(|e| FixtureError::Declaration {
        clause: "engine-driven claim",
        detail: format!("the engine refused the Money10 claim: {e}"),
    })?;
    PairFixture::new(&PAIR_6B, stage, greater, lesser, base.subject, rules)
}

fn build_6c(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base_with_order(stage, &PAIR_6C, None, FIXTURE_SET_CARD, rules)?;
    for card in FAMILY_6C_COMPLETED {
        complete_order_from_the_box(&mut base.state, base.subject, card)?;
    }
    stock_exactly_the_requirements(&mut base.state, base.subject, FIXTURE_SET_CARD, rules)?;
    let lesser = base.state;
    let greater = fulfill_through_the_engine(&lesser, FIXTURE_SET_CARD, rules)?;
    PairFixture::new(&PAIR_6C, stage, greater, lesser, base.subject, rules)
}

fn build_6d_church(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base_with_order(stage, &PAIR_6D_CHURCH, None, FIXTURE_TRACK_CARD, rules)?;
    let church = find_area_of_type(&base.state, AreaType::Church)?;
    // ONE copper and ONE silk: two distinct payment types, and
    // `min(power 2, 5) + 2` is the registered total faith of 4. A third type
    // would carry the faith past 4 and change which spaces qualify.
    set_trade_goods(&mut base.state, base.subject, [1, 1, 0, 0]);
    base.state.player_mut(base.subject).imported_goods = 0;
    let placement = Move::PlaceThreeAreas { placements: vec![church] };

    let mut greater = drive_to_resolved_area_action(
        &base.state,
        placement.clone(),
        church,
        AreaAction::Church {
            copper: true,
            silk: true,
            tea: false,
            seafood: false,
            yen: false,
            imported_good: false,
        },
        rules,
    )?;
    // Faith 4 leaves exactly one eligible space, so the placement resolves
    // INLINE and what remains queued is the space's movement rider — one of
    // them, which is the registered count this refuses to differ from.
    decline_the_registered_riders(&mut greater, CHURCH_RIDERS_DECLINED, rules)?;

    // The LESSER side is the church's own Skip — a real move from the same
    // root, which is what lets one view class describe both sides.
    let lesser =
        drive_to_resolved_area_action(&base.state, placement, church, AreaAction::Skip, rules)?;
    PairFixture::new(&PAIR_6D_CHURCH, stage, greater, lesser, base.subject, rules)
}

fn build_6d_customs(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base =
        family_base_with_order(stage, &PAIR_6D_CUSTOMS, None, FIXTURE_TRACK_CARD, rules)?;
    let customs = find_area_of_type(&base.state, AreaType::Customs)?;
    set_trade_goods(&mut base.state, base.subject, [0, 0, 0, 0]);
    base.state.player_mut(base.subject).imported_goods = u32::from(CUSTOMS_IMPORTS_PAID);
    let placement = Move::PlaceThreeAreas { placements: vec![customs] };

    let mut greater = drive_to_resolved_area_action(
        &base.state,
        placement.clone(),
        customs,
        AreaAction::Customs { imported_goods_paid: CUSTOMS_IMPORTS_PAID },
        rules,
    )?;
    // Cost 1 maps to spaces {1, 2} and both are empty, so the engine queues
    // the choice and the registration answers it.
    resolve_registered_choice(
        &mut greater,
        PendingChoice::CustomsSpace { cost: CUSTOMS_IMPORTS_PAID },
        Move::ChooseCustomsSpace { space: CUSTOMS_SPACE },
        rules,
    )?;
    // Space 1's two warehouse takes, declined per §5.2.
    decline_the_registered_riders(&mut greater, CUSTOMS_RIDERS_DECLINED, rules)?;

    let lesser =
        drive_to_resolved_area_action(&base.state, placement, customs, AreaAction::Skip, rules)?;
    PairFixture::new(&PAIR_6D_CUSTOMS, stage, greater, lesser, base.subject, rules)
}

// --- family 7 -------------------------------------------------------------

fn build_7(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let mut base = family_base(stage, &PAIR_7, None, rules)?;
    let silk = find_area_of_type(&base.state, AreaType::SilkMill)?;
    set_trade_goods(&mut base.state, base.subject, [0, 0, 0, 0]);
    // 2(a)'s construction at ONE pawn: power 2, and `silk_mill_reward(2)` = 1.
    let greater = drive_to_resolved_area_action(
        &base.state,
        Move::PlaceThreeAreas { placements: vec![silk] },
        silk,
        AreaAction::ProduceSilk,
        rules,
    )?;
    let mut lesser = greater.clone();
    lesser.player_mut(base.subject).silk -= u32::from(crate::game::actions::silk_mill_reward(2));
    PairFixture::new(&PAIR_7, stage, greater, lesser, base.subject, rules)
}

// --- family 11 ------------------------------------------------------------

/// The two sides of a family-11 pair BEFORE the greater side's realized yield.
///
/// Both sides carry the same one surviving pawn count, the same hand, and the
/// president standing where that side acted — everything the five pairs share.
struct Family11Sides {
    greater: GameState,
    lesser: GameState,
    subject: PlayerId,
    opponent: PlayerId,
}

/// Family 11's shared construction: one pawn at each of the two acted areas
/// before the turn, one of them drained to hand by that turn's Recovery, and
/// the president standing on the area it acted at.
fn family_11_sides(
    declaration: &PairDeclaration,
    stage: StageEndpoint,
    high: AreaType,
    rules: &Rules,
) -> Result<Family11Sides, FixtureError> {
    let layout = family_11_layout(high)?;
    let base = family_base(stage, declaration, Some(layout), rules)?;
    let hi = FAMILY_11_HIGH_ACCESS_AREA;
    let copper = FAMILY_11_COPPER_AREA;

    // GREATER acted at the high-access area: Recovery took that area's pawn to
    // hand, so the surviving pawn is the one at the Copper Mine.
    let mut greater = base.state.clone();
    place_from_hand(&mut greater, base.subject, copper, 1)?;
    greater.player_mut(base.subject).president_location = PresidentLocation::OnBoard(hi);

    // LESSER acted at the Copper Mine, so the surviving pawn is the other one.
    let mut lesser = base.state;
    place_from_hand(&mut lesser, base.subject, hi, 1)?;
    lesser.player_mut(base.subject).president_location = PresidentLocation::OnBoard(copper);

    Ok(Family11Sides { greater, lesser, subject: base.subject, opponent: base.opponent })
}

fn build_11a(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let sides = family_11_sides(&PAIR_11A, stage, AreaType::SilkMill, rules)?;
    let Family11Sides { mut greater, lesser, subject, .. } = sides;
    // `silk_mill_reward(realizable power 2) == 1`.
    greater.player_mut(subject).silk += 1;
    PairFixture::new(&PAIR_11A, stage, greater, lesser, subject, rules)
}

fn build_11b(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let sides = family_11_sides(&PAIR_11B, stage, AreaType::Bank, rules)?;
    let Family11Sides { mut greater, lesser, subject, .. } = sides;
    // `bank_reward(realizable power 2) == 2`.
    greater.player_mut(subject).money += 2;
    PairFixture::new(&PAIR_11B, stage, greater, lesser, subject, rules)
}

fn build_11c(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let sides = family_11_sides(&PAIR_11C, stage, AreaType::EmploymentAgency, rules)?;
    let Family11Sides { mut greater, lesser, subject, .. } = sides;
    // `employment_agency_tokens_allowed(realizable power 2) == 1`, spent on the
    // free class: warehouse -> HAND, which is the only direction that counter
    // ever moves.
    {
        let p = greater.player_mut(subject);
        if p.assistants.in_warehouse == 0 {
            return Err(FixtureError::OutOfStock("a warehouse assistant to retrieve"));
        }
        p.assistants.in_warehouse -= 1;
        p.assistants.in_hand += 1;
    }
    PairFixture::new(&PAIR_11C, stage, greater, lesser, subject, rules)
}

fn build_11d(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let sides = family_11_sides(&PAIR_11D, stage, AreaType::LaboratoryA, rules)?;
    let Family11Sides { mut greater, mut lesser, subject, opponent } = sides;
    let lab = FAMILY_11_HIGH_ACCESS_AREA;

    // The shared PRE-PURCHASE position, identical on both sides.
    for side in [&mut greater, &mut lesser] {
        // Money 0 and no imports: the only takeable slot is then a ZERO-DEFICIT
        // one, which is what pins X1 at exactly 1 on the lesser side.
        side.player_mut(subject).money = 0;
        side.player_mut(subject).imported_goods = 0;
        grant_technology(side, subject, PRE_OWNED_TECH, rules)?;
        // The third power point. See PAIR_11D's collision note: no printed
        // technology costs less than 3 industry, so a zero-deficit purchase is
        // unreachable at the family's own template power of 2. The Station
        // token adds +1 for ALL players at its area, and the OPPONENT owns the
        // technology that placed it, so the subject's registered "≤ 1 tech
        // pre-purchase" is untouched.
        grant_technology(side, opponent, TechEffect::Station, rules)?;
        side.station_location = Some(lab);
        stock_lab_a(side, LAB_STOCK, rules)?;
    }

    // The GREATER side's purchase, through the engine's own slide and refill.
    let taken = crate::game::remove_and_slide_lab(&mut greater.lab_state.lab_a, PURCHASED_SLOT)
        .ok_or(FixtureError::OutOfStock("a technology in the purchased slot"))?;
    crate::game::refill_lab(&mut greater.lab_state.lab_a, &mut greater.tech_deck);
    greater.player_mut(subject).technologies.push(taken);

    PairFixture::new(&PAIR_11D, stage, greater, lesser, subject, rules)
}

fn build_11e(stage: StageEndpoint, rules: &Rules) -> Result<PairFixture, FixtureError> {
    let sides = family_11_sides(&PAIR_11E, stage, AreaType::PortA, rules)?;
    let Family11Sides { mut greater, mut lesser, subject, .. } = sides;

    // The registered offer, on BOTH sides: the lesser side still sees it in the
    // Low zone (that is what X2 reads), and the greater side is the one that
    // took it.
    for side in [&mut greater, &mut lesser] {
        move_order_card_to_port_slot(side, FIXTURE_PORT_CARD, TAKEN_PORT_SLOT)?;
    }

    // The take, mirroring `execute_port_action`: lift the card, TOTAL-compact
    // the remaining offers left, then refill rightmost-first from the deck —
    // which at §5.1's late endpoint is empty, so the sixth slot stays open.
    let port = &mut greater.port_state.port_a;
    let taken = port.slots[TAKEN_PORT_SLOT]
        .take()
        .ok_or(FixtureError::OutOfStock("the registered port offer"))?;
    let remaining: Vec<OrderCardId> = port.slots.iter_mut().filter_map(|s| s.take()).collect();
    for (i, card) in remaining.into_iter().enumerate() {
        port.slots[i] = Some(card);
    }
    crate::game::refill_port(&mut greater.port_state.port_a, &mut greater.order_deck);
    greater.player_mut(subject).order_cards_in_hand.push(taken);

    // The engine's own trigger check, on both sides rather than a hand-set
    // flag: an empty deck plus the open slot the take just made IS
    // `EndGameTrigger::OrderDeckEmpty`, and running the real function is what
    // makes the greater side's declared trigger the engine's answer instead of
    // this module's.
    crate::game::check_end_game_triggers(&mut greater);
    crate::game::check_end_game_triggers(&mut lesser);

    PairFixture::new(&PAIR_11E, stage, greater, lesser, subject, rules)
}

// ---------------------------------------------------------------------------
// The hand-built layouts (§5.1's step-2 open parameters for 1(c), 1(e), 11)
// ---------------------------------------------------------------------------

/// Areas on a 2-player board — `board_gen::expected_tile_count(2)`.
pub const TWO_PLAYER_AREAS: usize = 10;

/// **THE SHARED RING LAYOUT** that 1(c) and 1(e) are built on.
///
/// # The topology it sits on, which is NOT a choice
///
/// A 2P board is the staggered pyramid with rows 4/3/2/1 and AreaIds assigned
/// row-major from the widest row, so the eighteen free edges are fixed before
/// any type is placed: 0–{1,4} · 1–{0,2,4,5} · 2–{1,3,5,6} · 3–{2,6} ·
/// 4–{0,1,5,7} · 5–{1,2,4,6,7,8} · 6–{2,3,5,8} · 7–{4,5,8,9} · 8–{5,6,7,9} ·
/// 9–{7,8}. There is no Canal at 2P, so `canal_crossings` is empty and the free
/// edges ARE `adjacency` — the subtraction S4 and S7 specify is a no-op here,
/// stated so a reader does not go looking for it.
///
/// # What the layout chooses, and why it has to choose anything
///
/// Only WHICH TYPE SITS WHERE. That is the whole freedom a hand-built layout
/// has: the shape, the adjacency and the ten-tile multiset are the engine's.
/// And it is enough, because 1(c) and 1(e) need the areas their pawns stand on
/// to have STATED types — the two pairs move the president over a fixed
/// assistant set, so S1/S2/S5/S6 are equal by construction, and a reader
/// checking that reads it off this table instead of off a seed's shuffle.
///
/// The four production tiles sit on the top row and the two management boards
/// sit low, which puts 1(e)'s chain (areas 1 → 5 → 8) across three different
/// type classes and keeps both president areas (0 and 9 for 1(c), 0 and 3 for
/// 1(e)) free of own assistants in every construction here.
pub const RING_LAYOUT: [AreaType; TWO_PLAYER_AREAS] = [
    AreaType::CopperMine,       // 0 — 1(c)/1(e)'s NEAR president area
    AreaType::SilkMill,         // 1 — the assistants' area / the chain head
    AreaType::TeaPlantation,    // 2
    AreaType::FishingGround,    // 3 — 1(e)'s DISCONNECTED president area
    AreaType::Bank,             // 4
    AreaType::PortA,            // 5 — 1(e)'s chain middle
    AreaType::Church,           // 6
    AreaType::Customs,          // 7
    AreaType::LaboratoryA,      // 8 — 1(e)'s chain tail
    AreaType::EmploymentAgency, // 9 — 1(c)'s FAR president area
];

/// 1(c)'s single assistant area — in [`RING_NEAR_PRESIDENT_AREA`]'s free ring
/// and outside [`RING_FAR_PRESIDENT_AREA`]'s.
pub const RING_ASSISTANT_AREA: AreaId = AreaId(1);
/// The president area whose free ring holds the assistants (1(c) and 1(e)).
pub const RING_NEAR_PRESIDENT_AREA: AreaId = AreaId(0);
/// 1(c)'s far president area: its ring is {7, 8}, which holds none of them.
pub const RING_FAR_PRESIDENT_AREA: AreaId = AreaId(9);
/// 1(e)'s own-assistant chain, head first: 1–5 and 5–8 are both free edges, and
/// 1 is in [`RING_NEAR_PRESIDENT_AREA`]'s ring.
pub const RING_CHAIN_AREAS: [AreaId; 3] = [AreaId(1), AreaId(5), AreaId(8)];
/// 1(e)'s disconnected president area: its ring is {2, 6}, so no own assistant
/// is one free step away and the chain is unreachable.
pub const RING_DISCONNECTED_PRESIDENT_AREA: AreaId = AreaId(3);

/// The eight types family 11's layout distributes outside its two acted areas,
/// in the fixed order they fill areas 0–6 and 9.
const FAMILY_11_FILL: [AreaType; 9] = [
    AreaType::SilkMill,
    AreaType::TeaPlantation,
    AreaType::FishingGround,
    AreaType::Bank,
    AreaType::PortA,
    AreaType::LaboratoryA,
    AreaType::EmploymentAgency,
    AreaType::Church,
    AreaType::Customs,
];

/// The areas family 11's layout fills with [`FAMILY_11_FILL`], in order —
/// everything except the two acted areas.
const FAMILY_11_FILL_AREAS: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 9];

/// **FAMILY 11's SHARED HAND-BUILT LAYOUT**, one per pair: `high` at
/// [`FAMILY_11_HIGH_ACCESS_AREA`], the Copper Mine at
/// [`FAMILY_11_COPPER_AREA`], and the remaining eight types in a fixed order
/// everywhere else.
///
/// # THE CONGRUENCE ARGUMENT (§5.2's registered requirement)
///
/// Family 11 requires the two acted areas' president neighbourhoods to be
/// congruent, "so S4 and S7 are EQUAL by construction — a registered
/// requirement, not a disjunction". Here that is a graph AUTOMORPHISM, which is
/// the strongest form the claim can take and the one a re-read can check by
/// reading the edge list rather than by trusting an arithmetic coincidence.
///
/// Let σ = (0 3)(1 2)(4 6)(7 8), fixing 5 and 9. σ is an automorphism of the 2P
/// free-edge graph: it is the reflection of the staggered pyramid, and it maps
/// every one of the eighteen edges to an edge — 0–1↦3–2, 0–4↦3–6, 1–4↦2–6,
/// 1–5↦2–5, 1–2↦2–1, 2–6↦1–4, 4–5↦6–5, 4–7↦6–8, 5–7↦5–8, 5–6↦5–4, 6–8↦4–7,
/// 7–8↦8–7, 7–9↦8–9, 8–9↦7–9, and the three within-row edges of the top row map
/// among themselves. (`the_family_eleven_reflection_is_a_board_automorphism`
/// checks this against the constructed layout rather than against this
/// paragraph.)
///
/// σ swaps exactly the two acted areas, 7 and 8. So it carries the greater
/// side's board position — president at 7, one own assistant at 8, none
/// anywhere else — onto the lesser side's, president at 8, one own assistant at
/// 7. S4 and S7 read own-assistant counts and free adjacency and nothing else,
/// so an automorphism carrying one position onto the other makes them EQUAL,
/// not merely equal-by-arithmetic: S4 = 1 (the acted area holds nothing after
/// Recovery; its ring holds the one survivor) and S7 = 1 (that survivor is the
/// whole reachable set) on both sides. S3 = 1 on both sides for the same
/// reason.
///
/// The argument is indifferent to WHICH types sit at 7 and 8, because σ acts on
/// the graph and the S-group's ring terms do not read types. That is exactly
/// why the five pairs can vary the high-access type while keeping one
/// congruence proof.
pub fn family_11_layout(high: AreaType) -> Result<[AreaType; TWO_PLAYER_AREAS], FixtureError> {
    if !FAMILY_11_FILL.contains(&high) {
        return Err(FixtureError::Declaration {
            clause: "family 11 layout",
            detail: format!(
                "{high:?} is not one of the eight types family 11 may seat at its \
                 high-access area (the Copper Mine is the LESSER side's area, by \
                 registration)"
            ),
        });
    }
    let mut out = [AreaType::CopperMine; TWO_PLAYER_AREAS];
    let mut fill = FAMILY_11_FILL.iter().filter(|t| **t != high);
    for area in FAMILY_11_FILL_AREAS {
        out[area] = *fill
            .next()
            .ok_or(FixtureError::OutOfStock("a filler type for family 11's layout"))?;
    }
    out[FAMILY_11_HIGH_ACCESS_AREA.0 as usize] = high;
    out[FAMILY_11_COPPER_AREA.0 as usize] = AreaType::CopperMine;
    Ok(out)
}

// ---------------------------------------------------------------------------
// 11(d)'s registered technology and laboratory state
// ---------------------------------------------------------------------------

/// The technology 11(d)'s subject owns BEFORE its purchase.
///
/// Industry 6 and country US, and both numbers are doing work. Six is the
/// largest industry value that still leaves the seat SECOND behind the 2P
/// dummy's 7.5 (D5's doubled comparison: 12 against 15), so the cost-3 purchase
/// carries it to 9 — doubled 18, past the dummy — and `TechBonusNow` moves
/// 5 → 10. US is a country [`PURCHASED_TECH`] does not share, so the purchase
/// takes the seat from one distinct country to two and `CountryBonusNow` moves
/// 0 → 2 while the post-purchase icon count of each country stays at 1, ODD,
/// which is the parity §5.2 registers so no mercenary is due.
const PRE_OWNED_TECH: TechEffect = TechEffect::Telegram;

/// The technology 11(d)'s greater side buys: industry 3 (the cheapest printed
/// value) from slot 0 (position surcharge 0) at realizable power 3, so the
/// registered deficit is exactly 0.
const PURCHASED_TECH: TechEffect = TechEffect::Brickyard;

/// The laboratory slot the purchase comes from.
const PURCHASED_SLOT: usize = 0;

/// **11(d)'s REGISTERED LABORATORY**, slot 0 first.
///
/// The stock is what pins X1 at exactly 1 on the lesser side. With money 0 and
/// no imports, `floor(money/2) + imports = 0`, so a slot is takeable only at
/// deficit 0 — and against realizable power 3 the deficits here are
/// 3+0−3 = 0 · 5+0−3 = 2 · 5+1−3 = 3 · 3+1−3 = 1 · 3+2−3 = 2 · 3+2−3 = 2. One
/// zero. Slots 2–5 are non-takeable for free, since the cheapest printed
/// technology costs 3 and those positions surcharge 1 or 2; slot 1 is the one
/// that has to be BOUGHT non-takeable, and Exposition's industry 5 does it.
const LAB_STOCK: [TechEffect; 6] = [
    PURCHASED_TECH,           // slot 0 — industry 3, the registered purchase
    TechEffect::Exposition,   // slot 1 — industry 5, the surcharge-free refusal
    TechEffect::Newspaper,    // slot 2 — industry 5
    TechEffect::Gaslight,     // slot 3 — industry 3
    TechEffect::Ball,         // slot 4 — industry 3
    TechEffect::PatentSystem, // slot 5 — industry 3
];

/// 11(e)'s port slot: index 0 is a LOW-zone position, and Low is the only zone
/// realizable power 2 opens.
const TAKEN_PORT_SLOT: usize = 0;

// ---------------------------------------------------------------------------
// Construction surgery — every mutation these ten fixtures make
// ---------------------------------------------------------------------------

/// Re-type a 2P board onto a hand-built layout.
///
/// The layout freedom §5.1 grants is WHICH TYPE SITS WHERE and nothing else, so
/// this checks both halves of that: the board must have the shape it already
/// had (ten areas, whatever `board_gen` built), and `types` must be a
/// PERMUTATION of the tile multiset already on it. Adjacency, grid positions,
/// building sites and 5-Power tokens are keyed by `AreaId` and are left exactly
/// as generated — re-typing moves no edge and no site.
fn retype_board_2p(
    state: &mut GameState,
    types: [AreaType; TWO_PLAYER_AREAS],
) -> Result<(), FixtureError> {
    let existing: Vec<AreaType> =
        state.board_layout.areas.values().map(|cfg| cfg.area_type).collect();
    if existing.len() != TWO_PLAYER_AREAS {
        return Err(FixtureError::Declaration {
            clause: "hand-built layout",
            detail: format!(
                "the board carries {} areas; a hand-built 2P layout re-types the {} a 2P \
                 board has",
                existing.len(),
                TWO_PLAYER_AREAS,
            ),
        });
    }
    for t in &existing {
        let want = types.iter().filter(|x| *x == t).count();
        let have = existing.iter().filter(|x| *x == t).count();
        if want != have {
            return Err(FixtureError::Declaration {
                clause: "hand-built layout",
                detail: format!(
                    "the layout seats {want} {t:?} tile(s) where the generated 2P tile set \
                     has {have}; the layout freedom is which type sits WHERE, never the \
                     multiset"
                ),
            });
        }
    }
    for (index, area_type) in types.into_iter().enumerate() {
        let id = AreaId(index as u8);
        let cfg = state.board_layout.areas.get_mut(&id).ok_or(FixtureError::Declaration {
            clause: "hand-built layout",
            detail: format!("area {index} is not on this board"),
        })?;
        cfg.area_type = area_type;
    }
    Ok(())
}

/// The one area of a given type. At 2P every one of the ten tiles is a distinct
/// type (the removal keeps one of each production type), so "the" is exact.
fn find_area_of_type(state: &GameState, want: AreaType) -> Result<AreaId, FixtureError> {
    let mut found = state
        .board_layout
        .areas
        .iter()
        .filter(|(_, cfg)| cfg.area_type == want)
        .map(|(id, _)| *id);
    let first = found.next().ok_or(FixtureError::OutOfStock("an area of the registered type"))?;
    if found.next().is_some() {
        return Err(FixtureError::Declaration {
            clause: "registered area",
            detail: format!("{want:?} appears more than once; a 2P board carries one of each"),
        });
    }
    Ok(first)
}

/// Move `n` of a seat's hand assistants onto `area`.
fn place_from_hand(
    state: &mut GameState,
    seat: PlayerId,
    area: AreaId,
    n: u8,
) -> Result<(), FixtureError> {
    let p = state.player_mut(seat);
    if p.assistants.in_hand < n {
        return Err(FixtureError::OutOfStock("hand assistants to place"));
    }
    p.assistants.in_hand -= n;
    *p.assistants.on_board.entry(area).or_insert(0) += n;
    Ok(())
}

/// The board areas carrying [`PRODUCTIVE_TRIO`]'s three types, in `AreaId`
/// order.
fn productive_trio_areas(state: &GameState) -> Result<[AreaId; 3], FixtureError> {
    let mut out = [AreaId(0); 3];
    for (slot, want) in out.iter_mut().zip(PRODUCTIVE_TRIO) {
        *slot = find_area_of_type(state, want)?;
    }
    out.sort_unstable_by_key(|a| a.0);
    Ok(out)
}

/// Set a seat's four trade-good stocks outright, `[copper, silk, tea,
/// seafood]`.
///
/// Registered stocks are written rather than left to the deal in every pair
/// that reads them: the goods are what E2, O2, X3 and the Category-A
/// achievement leg all key on, so "whatever setup happened to give" is four
/// unregistered facts at once.
fn set_trade_goods(state: &mut GameState, seat: PlayerId, goods: [u32; 4]) {
    let p = state.player_mut(seat);
    p.copper = goods[0];
    p.silk = goods[1];
    p.tea = goods[2];
    p.seafood = goods[3];
}

/// Set a seat's stocks to EXACTLY one order card's printed requirements.
///
/// Exactly, not at least: a surplus would survive the fulfillment and leave
/// O2's per-good `min` reading the leftover rather than the retired
/// requirement, which is the row §5.2 registers as retiring.
fn stock_exactly_the_requirements(
    state: &mut GameState,
    seat: PlayerId,
    card: OrderCardId,
    rules: &Rules,
) -> Result<(), FixtureError> {
    let order = rules
        .orders
        .get(card)
        .ok_or(FixtureError::OutOfStock("the registered order card's printed face"))?;
    let r = order.requirements;
    set_trade_goods(
        state,
        seat,
        [u32::from(r.copper), u32::from(r.silk), u32::from(r.tea), u32::from(r.seafood)],
    );
    state.player_mut(seat).imported_goods = 0;
    Ok(())
}

/// Fulfil a registered order through the engine's CHECKED boundary.
///
/// `apply_move` refuses anything `get_legal_moves` does not offer, so this is
/// simultaneously the construction and the proof that the seat could legally
/// have made the move — including `can_fulfill_order`'s per-good check, the
/// phase gate, the queued icon choices and
/// `check_mercenary_eligibility`'s five-country sweep.
fn fulfill_through_the_engine(
    state: &GameState,
    card: OrderCardId,
    rules: &Rules,
) -> Result<GameState, FixtureError> {
    crate::game::apply_move(state, &Move::FulfillOrder { order_id: card }, rules).map_err(|e| {
        FixtureError::Declaration {
            clause: "engine-driven fulfillment",
            detail: format!("the engine refused fulfilling order {}: {e}", card.0),
        }
    })
}

/// Move `n` of a seat's WAREHOUSE assistants onto `area`.
///
/// **No rule does this**, and the helper's name is where that is said rather
/// than discovered. The warehouse is a one-way source — three code paths drain
/// it and none refills it — and Recovery returns board pawns to HAND, never to
/// the warehouse. That is exactly why §5.2's 2(b) is registered as the
/// RETRIEVAL-economy comparison rather than the recovery one (the dossier's
/// 2026-08-20 re-reading), and why its two sides are constructed rather than
/// played.
fn place_from_warehouse(
    state: &mut GameState,
    seat: PlayerId,
    area: AreaId,
    n: u8,
) -> Result<(), FixtureError> {
    let p = state.player_mut(seat);
    if p.assistants.in_warehouse < n {
        return Err(FixtureError::OutOfStock("warehouse assistants to deploy"));
    }
    p.assistants.in_warehouse -= n;
    *p.assistants.on_board.entry(area).or_insert(0) += n;
    Ok(())
}

/// Apply the ENGINE's own `ReturnPresidentToHand` to a stranded layout and hand
/// back the seat's resulting player state.
///
/// # Why family 8 is constructed AND engine-checked
///
/// Its two sides are one TURN apart, not one move: a driven recall runs
/// `ReturnPresidentToHand` → `AdditionalActionSecond` → the seat hand-off, and
/// at a 2P hand-off the mover is the OPPONENT, which
/// [`PairFixture::new`]'s clause 4 refuses. So the greater side is built rather
/// than played.
///
/// What is not restated is the RECALL. This drops the stranded state into the
/// Movement phase, where `game/legal_moves.rs:386` offers a
/// `ReturnPresidentToHand` for every subset of the seat's board pawns — the
/// "emergency exit, always available" arm — and lets `apply_move`'s checked
/// boundary produce the bytes. The construction then refuses unless the greater
/// side carries exactly those bytes, so "the president came home with the last
/// two pawns" is the engine's own answer rather than this module's.
///
/// The phase write is the only surgery, and it is the minimum: the move reads
/// neither `president_start_area` (`apply_move.rs:111` CLEARS it) nor
/// `current_action_power`, so nothing else needs staging to make the Movement
/// step describable.
fn recall_through_the_engine(
    stranded: &GameState,
    seat: PlayerId,
    rules: &Rules,
) -> Result<crate::types::PlayerState, FixtureError> {
    let mut mid_turn = stranded.clone();
    mid_turn.turn_phase = TurnPhase::Movement;
    let recover: std::collections::BTreeMap<AreaId, u8> = stranded
        .player(seat)
        .assistants
        .on_board
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(a, n)| (*a, *n))
        .collect();
    let recalled =
        crate::game::apply_move(&mid_turn, &Move::ReturnPresidentToHand { recover }, rules)
            .map_err(|e| FixtureError::Declaration {
                clause: "engine-driven recall",
                detail: format!("the engine refused the recall of every stranded pawn: {e}"),
            })?;
    Ok(recalled.player(seat).clone())
}

/// Push a registered order card onto a seat's COMPLETED pile.
///
/// **The BOX pays in every case**, and that is the whole design. A card
/// standing in the deck or at a port offer trades places with a boxed card
/// first, so the deck keeps its LENGTH (§5.1's p = 0 realization wants a full
/// one) and the port keeps its FILL (at 2P a non-full port implies an
/// exhausted deck, so a p = 0 fixture cannot show one). Net movement is always
/// box −1, completed +1, and the 36-card conservation total never moves.
///
/// It is a legality construction, not a reachability one: a real seat reaches
/// four completed orders by drawing through the port, which would have moved
/// the deck clock off zero. 6(c) registers both endpoints and this is what
/// lets the early one exist.
fn complete_order_from_the_box(
    state: &mut GameState,
    seat: PlayerId,
    card: OrderCardId,
) -> Result<(), FixtureError> {
    for p in &state.players {
        if p.order_cards_in_hand.contains(&card) || p.completed_orders.contains(&card) {
            return Err(FixtureError::Declaration {
                clause: "registered completed order",
                detail: format!(
                    "order {} is already held or completed by seat {}; a pre-completed \
                     order must be swappable out of the deck, the box or a port slot",
                    card.0, p.id.0,
                ),
            });
        }
    }
    if let Some(i) = state.orders_removed.iter().position(|c| *c == card) {
        state.orders_removed.remove(i);
    } else {
        let boxed = state
            .orders_removed
            .pop()
            .ok_or(FixtureError::OutOfStock("a boxed order to trade places with"))?;
        if let Some(i) = state.order_deck.iter().position(|c| *c == card) {
            state.order_deck[i] = boxed;
        } else if let Some(i) = port_a_slot_of(state, card) {
            state.port_state.port_a.slots[i] = Some(boxed);
        } else {
            state.orders_removed.push(boxed);
            return Err(FixtureError::OutOfStock("the registered completed-order card"));
        }
    }
    state.player_mut(seat).completed_orders.push(card);
    Ok(())
}

/// Write a registration's three declared achievement cards onto a state.
///
/// A registration DECLARES its draws (§5.1), so a fixture builds them rather
/// than accepting whatever the setup seed shuffled up. The cards are their own
/// components — not drawn from the order or technology decks — so no
/// conservation total moves, and both sides of a pair get the same three, so
/// the achievement feature's delta is untouched by the substitution itself.
fn set_declared_draws(state: &mut GameState, draws: &AchievementDraws) {
    state.achievements = [
        AchievementCard::new(draws.category_a),
        AchievementCard::new(draws.category_b),
        AchievementCard::new(draws.category_c),
    ];
}

/// Swap a seat's dealt starting order for the registered one.
///
/// A SWAP, not a deal: the card the seat was dealt goes to wherever the
/// registered card was standing, so every pile total is unchanged and order
/// conservation (36 across deck, ports, hands, completed piles and the box) is
/// exact. Both `order_cards_in_hand` and `starting_order` move together —
/// invariant I11 wants the recorded identity to be in the hand, and §5.1's
/// p = 0 hand cap wants the hand to hold nothing else.
fn give_starting_order(
    state: &mut GameState,
    seat: PlayerId,
    card: OrderCardId,
) -> Result<(), FixtureError> {
    let current = state
        .player(seat)
        .starting_order
        .ok_or(FixtureError::OutOfStock("a recorded starting order to swap"))?;
    if current == card {
        return Ok(());
    }
    for p in &state.players {
        if p.order_cards_in_hand.contains(&card) || p.completed_orders.contains(&card) {
            return Err(FixtureError::Declaration {
                clause: "registered order card",
                detail: format!(
                    "order {} is already held or completed by seat {}; the registered card \
                     must be swappable out of the deck, the box or a port slot",
                    card.0, p.id.0,
                ),
            });
        }
    }
    if let Some(i) = state.order_deck.iter().position(|c| *c == card) {
        state.order_deck[i] = current;
    } else if let Some(i) = state.orders_removed.iter().position(|c| *c == card) {
        state.orders_removed[i] = current;
    } else if let Some(i) = port_a_slot_of(state, card) {
        state.port_state.port_a.slots[i] = Some(current);
    } else {
        return Err(FixtureError::OutOfStock("the registered starting order card"));
    }
    let p = state.player_mut(seat);
    let held = p
        .order_cards_in_hand
        .iter()
        .position(|c| *c == current)
        .ok_or(FixtureError::OutOfStock("the dealt starting order in hand"))?;
    p.order_cards_in_hand[held] = card;
    p.starting_order = Some(card);
    Ok(())
}

/// Put a registered order card into a Port A slot, swapping out whatever stood
/// there. Conservation is exact for the same reason [`give_starting_order`]'s
/// is: nothing leaves a pile, two cards trade places.
fn move_order_card_to_port_slot(
    state: &mut GameState,
    card: OrderCardId,
    slot: usize,
) -> Result<(), FixtureError> {
    if port_a_slot_of(state, card) == Some(slot) {
        return Ok(());
    }
    let displaced = state.port_state.port_a.slots[slot]
        .ok_or(FixtureError::OutOfStock("a card in the registered port slot"))?;
    for p in &state.players {
        if p.order_cards_in_hand.contains(&card) || p.completed_orders.contains(&card) {
            return Err(FixtureError::Declaration {
                clause: "registered port offer",
                detail: format!("order {} is already held by seat {}", card.0, p.id.0),
            });
        }
    }
    if let Some(i) = state.order_deck.iter().position(|c| *c == card) {
        state.order_deck[i] = displaced;
    } else if let Some(i) = state.orders_removed.iter().position(|c| *c == card) {
        state.orders_removed[i] = displaced;
    } else if let Some(i) = port_a_slot_of(state, card) {
        state.port_state.port_a.slots[i] = Some(displaced);
    } else {
        return Err(FixtureError::OutOfStock("the registered port offer card"));
    }
    state.port_state.port_a.slots[slot] = Some(card);
    Ok(())
}

/// Seat six registered order cards in Port A's six slots, slot 0 first.
///
/// Sequential swaps are SAFE for a set of distinct cards, and the reason is
/// worth stating because it is not obvious. Step `k` places `cards[k]` at slot
/// `k` and sends the displaced card to `cards[k]`'s old home. That old home
/// cannot be an already-finalized slot `j < k`: slot `j` holds `cards[j]` after
/// step `j`, so `cards[k]` standing there would mean `cards[j] == cards[k]`,
/// which distinctness forbids. The final read-back is kept anyway — the
/// argument is about the input, and a caller passing a repeat should be refused
/// rather than left with a quietly wrong port.
fn stock_port_a(state: &mut GameState, cards: [OrderCardId; 6]) -> Result<(), FixtureError> {
    for (slot, card) in cards.into_iter().enumerate() {
        move_order_card_to_port_slot(state, card, slot)?;
    }
    for (slot, card) in cards.into_iter().enumerate() {
        if state.port_state.port_a.slots[slot] != Some(card) {
            return Err(FixtureError::Declaration {
                clause: "registered port offers",
                detail: format!(
                    "slot {slot} holds {:?} after stocking, not the registered order {}; the \
                     six registered offers must be DISTINCT",
                    state.port_state.port_a.slots[slot].map(|c| c.0),
                    card.0,
                ),
            });
        }
    }
    Ok(())
}

/// Move a registered order card OFF the face-up offers, into the box.
///
/// The inverse of [`move_order_card_to_port_slot`], and it exists for one
/// reason: face-up port offers are LOCATED for O1's pool while deck and box
/// cards are not, so a pair that deals a card out of a port slot moves the pool
/// on that side alone. 10(d) parks both of its registered cards first so the
/// two deals are pool-neutral.
///
/// A boxed card trades places with it, so the port keeps its FILL and the box
/// keeps its SIZE; the 36-card conservation total never moves. A card already
/// off the port is left exactly where it is.
fn park_order_off_the_port(state: &mut GameState, card: OrderCardId) -> Result<(), FixtureError> {
    let Some(slot) = port_a_slot_of(state, card) else {
        return Ok(());
    };
    let boxed = state
        .orders_removed
        .pop()
        .ok_or(FixtureError::OutOfStock("a boxed order to trade places with"))?;
    state.port_state.port_a.slots[slot] = Some(boxed);
    state.orders_removed.push(card);
    Ok(())
}

/// The Port A slot holding `card`, if any. Port B is `None` at 2P by
/// construction (`board_gen` removes it), so there is no second board to scan.
fn port_a_slot_of(state: &GameState, card: OrderCardId) -> Option<usize> {
    state.port_state.port_a.slots.iter().position(|s| *s == Some(card))
}

/// Give a seat one technology carrying `effect`, taken out of whichever pile it
/// is standing in so that tech conservation (30 physical cards across deck,
/// laboratories and owners) is exact.
///
/// Effect equality rather than title equality is the engine's own ownership
/// test, and ten of the twenty titles print two physical copies — so this takes
/// ANY available copy and reports which.
fn grant_technology(
    state: &mut GameState,
    seat: PlayerId,
    effect: TechEffect,
    rules: &Rules,
) -> Result<TechCardId, FixtureError> {
    let has_effect =
        |id: TechCardId| rules.techs.iter().any(|c| c.id == id && c.effect == effect);
    if let Some(i) = state.tech_deck.iter().position(|id| has_effect(*id)) {
        let id = state.tech_deck.remove(i);
        state.player_mut(seat).technologies.push(id);
        return Ok(id);
    }
    let labs = std::iter::once(&mut state.lab_state.lab_a).chain(state.lab_state.lab_b.as_mut());
    for slot in labs.flat_map(|lab| lab.slots.iter_mut()) {
        if slot.is_some_and(&has_effect) {
            let id = slot.take().expect("just matched Some");
            state.player_mut(seat).technologies.push(id);
            return Ok(id);
        }
    }
    Err(FixtureError::OutOfStock("an unowned copy of the registered technology"))
}

/// Stock Laboratory A with six named effects, slot 0 first.
///
/// Every card currently on the board goes back to the deck first and the six
/// are drawn out of it by effect, so the total across deck + laboratories +
/// owners never moves. A card already OWNED cannot be stocked — the refusal
/// says so rather than silently seating a seventh copy.
fn stock_lab_a(
    state: &mut GameState,
    effects: [TechEffect; 6],
    rules: &Rules,
) -> Result<(), FixtureError> {
    for slot in state.lab_state.lab_a.slots.iter_mut() {
        if let Some(id) = slot.take() {
            state.tech_deck.push(id);
        }
    }
    for (index, effect) in effects.into_iter().enumerate() {
        let position = state
            .tech_deck
            .iter()
            .position(|id| rules.techs.iter().any(|c| c.id == *id && c.effect == effect))
            .ok_or(FixtureError::OutOfStock("an unowned copy of a stocked technology"))?;
        let id = state.tech_deck.remove(position);
        state.lab_state.lab_a.slots[index] = Some(id);
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::eval::{explain, WeightTable};
    use crate::types::{OrderCardId, TechCardId};

    // -----------------------------------------------------------------------
    // FIXTURE-GEOMETRY READERS
    //
    // These read the constructed STATE the §3.2 candidates will later read;
    // they are NOT those features. Step 3 owns S4's and S7's implementations
    // (in `eval.rs`, against the spec's own cards), and nothing here is a
    // second derivation of them: `ring_own_assistants` folds a raw adjacency
    // list, `own_assistant_reach` walks one, and both exist so the geometric
    // claims a registration makes — "the ring holds the assistants", "the same
    // pawns disconnected", "S4 and S7 are equal BY CONSTRUCTION" — are checked
    // against the bytes instead of asserted in prose.
    // -----------------------------------------------------------------------

    /// The seat's assistants standing on `area`'s FREE-adjacent ring. At 2P
    /// `canal_crossings` is empty, so the free ring IS `adjacent_areas`.
    fn ring_own_assistants(state: &GameState, seat: PlayerId, area: AreaId) -> u32 {
        let p = state.player(seat);
        state
            .board_layout
            .adjacent_areas(area)
            .iter()
            .map(|n| u32::from(p.assistants.in_area(*n)))
            .sum()
    }

    /// The areas holding the seat's assistants that are reachable from `from`
    /// by stepping along free edges onto own-assistant areas — the chain
    /// 1(e)'s registration is about, excluding the starting area itself.
    fn own_assistant_reach(state: &GameState, seat: PlayerId, from: AreaId) -> Vec<AreaId> {
        let p = state.player(seat);
        let mut seen: Vec<AreaId> = Vec::new();
        let mut frontier = vec![from];
        while let Some(here) = frontier.pop() {
            for n in state.board_layout.adjacent_areas(here) {
                if p.assistants.in_area(*n) >= 1 && !seen.contains(n) {
                    seen.push(*n);
                    frontier.push(*n);
                }
            }
        }
        seen.retain(|a| *a != from);
        seen.sort_unstable_by_key(|a| a.0);
        seen
    }

    /// The areas holding at least one of the seat's assistants.
    fn occupied_areas(state: &GameState, seat: PlayerId) -> Vec<AreaId> {
        let p = state.player(seat);
        let mut areas: Vec<AreaId> =
            p.assistants.on_board.iter().filter(|(_, n)| **n > 0).map(|(a, _)| *a).collect();
        areas.sort_unstable_by_key(|a| a.0);
        areas
    }

    /// One side of a registered pair, built for a test to read.
    fn built(id: &str, stage: StageEndpoint, rules: &Rules) -> PairFixture {
        let declaration =
            registered(id).unwrap_or_else(|| panic!("{id} is registered"));
        build_registered_pair(declaration, stage, rules)
            .unwrap_or_else(|e| panic!("{id} at {}: {e}", stage.name()))
    }

    /// Every registered pair, at every endpoint it declares, admitted through
    /// the fourteen clauses.
    ///
    /// The ONE test that says the package is real: a tuple nothing can build is
    /// a pre-registration, and §5.1's step-2 gate asks for a realized registry.
    #[test]
    fn every_registered_pair_builds_at_every_endpoint_it_declares() {
        let rules = rules();
        for pair in REGISTERED.iter() {
            let declaration = registered(pair.id)
                .unwrap_or_else(|| panic!("{} is registered but has no lookup arm", pair.id));
            for stage in declaration.stages {
                let fixture = build_registered_pair(declaration, *stage, &rules)
                    .unwrap_or_else(|e| panic!("{} at {}: {e}", pair.id, stage.name()));
                assert_eq!(fixture.declaration().id, pair.id);
                assert_eq!(fixture.stage(), *stage);
                assert_eq!(fixture.progress_ppm(&rules), stage.ppm());
                fixture.listed_incumbent_rows_are_nonzero(&rules).unwrap_or_else(|e| {
                    panic!("{} at {}: {e}", pair.id, stage.name())
                });
            }
        }
    }

    #[test]
    fn the_registered_tuples_transcribe_the_specs_carriers_stages_and_margins() {
        // §5.1's carrier map and §5.2's stage and margin columns, restated here
        // as data and checked against the registrations. This is a
        // TRANSCRIPTION pin, and transcription is what the bounded re-read is
        // hunting: a pair silently registered against the wrong carrier would
        // still build, still print, and still solve — as a constraint on a
        // feature it was never designed to bind.
        //
        // The carrier column is an OPTION because §5.2 names a carrier for only
        // some pairs. Family 6 registers the C2 CONTROLS — §5.1's carrier map
        // assigns them nothing, and the field's own doc reserves `None` for
        // exactly that ("the rows that carry no production feature at all") —
        // and family 7's row is a coverage registration §5.2 itself calls
        // implied by 3(b). Where §5.2 DOES name one (2(a)'s E2, 2(b)'s
        // warehouse weight, 2(d)'s E3), it is transcribed here.
        //
        // Family 8 and 10(c) are the two other `None`s: §5.1's carrier map
        // runs S1→…→X3 without ever naming family 8, whose row is about hand
        // ordering rather than about a production feature, and 10(c) is one of
        // the two recorded diagnostics, which carry nothing by definition.
        use CandidateFeature::*;
        let candidate = |f: CandidateFeature| Some(FeatureId::Candidate(f));
        let expected: &[(&str, Option<FeatureId>, &[StageEndpoint], MarginClass)] = &[
            ("1(a)", candidate(ProductionAccess), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::Margined {
                early: Some(1),
                late: Some(14),
                carrier: FeatureId::Candidate(ProductionAccess),
            }),
            ("1(b)", candidate(AssistantAreasDistinct), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("1(c)", candidate(PresidentLocalAccess), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("1(d)", candidate(TrackAccess), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("1(e)", candidate(PresidentReachableAreas), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("2(a)", candidate(TradeGoodsStock), &[StageEndpoint::Late], MarginClass::Margined {
                early: None,
                late: Some(27),
                carrier: FeatureId::Candidate(TradeGoodsStock),
            }),
            ("2(b)", Some(FeatureId::Incumbent(Feature::AssistantsInWarehouse)), &[StageEndpoint::Early], MarginClass::DirectionOnly),
            ("2(d)", candidate(BuildingTokensReady), &[StageEndpoint::Early], MarginClass::DirectionOnly),
            ("2(c)", None, &[StageEndpoint::Early], MarginClass::Weak),
            ("3(a)", candidate(MoneyStock), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::Margined {
                early: Some(1),
                late: Some(20),
                carrier: FeatureId::Candidate(MoneyStock),
            }),
            ("3(b)", candidate(TradeGoodsStock), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::Margined {
                early: Some(1),
                late: Some(14),
                carrier: FeatureId::Candidate(TradeGoodsStock),
            }),
            ("3(c)", candidate(BankAccess), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::Margined {
                early: Some(1),
                late: Some(40),
                carrier: FeatureId::Candidate(BankAccess),
            }),
            ("3(d)", candidate(EmploymentAgencyAccess), &[StageEndpoint::Early], MarginClass::DirectionOnly),
            ("6(a)", None, &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("6(b)", None, &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("6(c)", None, &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("6(d-church)", None, &[StageEndpoint::Late], MarginClass::DirectionOnly),
            ("6(d-customs)", None, &[StageEndpoint::Late], MarginClass::DirectionOnly),
            ("7", None, &[StageEndpoint::Late], MarginClass::Weak),
            ("8", None, &[StageEndpoint::Late], MarginClass::DirectionOnly),
            ("10(a)", candidate(PortTakeable), &[StageEndpoint::Early], MarginClass::DirectionOnly),
            ("10(b)", candidate(LabTakeable), &[StageEndpoint::Early], MarginClass::DirectionOnly),
            ("10(c)", None, &[StageEndpoint::Early], MarginClass::Weak),
            ("10(d)", candidate(MatchedProductionAccess), &[StageEndpoint::Early], MarginClass::DirectionOnly),
            ("11(a)", candidate(ProductionAccess), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("11(b)", candidate(BankAccess), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("11(c)", candidate(EmploymentAgencyAccess), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("11(d)", candidate(LabTakeable), &[StageEndpoint::Early, StageEndpoint::Late], MarginClass::DirectionOnly),
            ("11(e)", candidate(PortTakeable), &[StageEndpoint::Late], MarginClass::DirectionOnly),
        ];
        assert_eq!(expected.len(), REGISTERED_PAIR_COUNT);
        for (id, carrier, stages, margin) in expected {
            let d = registered(id).unwrap_or_else(|| panic!("{id} is registered"));
            assert_eq!(d.carrier, *carrier, "{id}: §5.2's named carrier");
            assert_eq!(d.stages, *stages, "{id}: §5.2's stage column");
            assert_eq!(d.margin, *margin, "{id}: §5.2's margin class");
            if let Some(row) = carrier {
                assert!(
                    d.expected_nonzero.contains(row),
                    "{id}: a carrier that is not expected to move is not a carrier",
                );
            }
        }
        // §5.1's early floor makes 1(a)'s early row identical in force to
        // DIRECTION-ONLY — the reason §5.4 keeps an early-stage rescale
        // diagnostic. 2(a) is registered LATE-only and so never meets it.
        assert_eq!(PAIR_1A.margin.margin_at(StageEndpoint::Early), Some(1));
        assert_eq!(PAIR_1A.margin.margin_at(StageEndpoint::Late), Some(14));
        let two_a = registered("2(a)").expect("2(a) is registered");
        assert_eq!(two_a.margin.margin_at(StageEndpoint::Early), None);
        assert_eq!(two_a.margin.margin_at(StageEndpoint::Late), Some(27));
    }

    #[test]
    fn the_realized_incumbent_deltas_are_exactly_the_registered_ones() {
        // Clause 13 refuses an UNLISTED nonzero row and
        // `listed_incumbent_rows_are_nonzero` refuses a LISTED zero, so between
        // them the SET is already pinned. What is not pinned by either is the
        // VALUE, and the values are what the registrations actually say:
        // 11(c)'s "−1 / +1", 11(d)'s "+industry", 11(e)'s "+1", 6(a)'s "VP
        // +face", 6(b)'s "+9 first place", 6(d)'s "VP +space" and both
        // majority mirrors.
        //
        // Every value below was PREDICTED from the rules before the
        // construction existed, which is the point of pinning it: a
        // construction that lands a different number is telling the
        // registration something.
        let rules = rules();
        let expected: &[(&str, &[(Feature, i64)])] = &[
            ("1(a)", &[]),
            ("1(b)", &[]),
            ("1(c)", &[]),
            ("1(d)", &[]),
            ("1(e)", &[]),
            // 2(a) and 2(d) are pure candidate-axis separators: goods and
            // hand-held building tokens are both invisible to all sixteen.
            ("2(a)", &[]),
            (
                "2(b)",
                &[(Feature::AssistantsInWarehouse, -3), (Feature::AssistantsOnBoard, 3)],
            ),
            ("2(d)", &[]),
            (
                "2(c)",
                &[(Feature::AssistantsInHandExcess, -3), (Feature::AssistantsOnBoard, 3)],
            ),
            ("6(a)", &[(Feature::VictoryPoints, 5), (Feature::HandSize, -1)]),
            (
                "6(b)",
                &[
                    (Feature::VictoryPoints, 9),
                    (Feature::AchievementProgressSum, -10),
                    (Feature::AssistantsInHandExcess, -1),
                ],
            ),
            (
                "6(c)",
                &[
                    (Feature::VictoryPoints, 5),
                    (Feature::CountryBonusNow, 4),
                    (Feature::HandSize, -1),
                ],
            ),
            (
                "6(d-church)",
                &[
                    (Feature::VictoryPoints, 3),
                    (Feature::ChurchBonusNow, 3),
                    (Feature::AssistantsOnBoard, -1),
                ],
            ),
            (
                "6(d-customs)",
                &[
                    (Feature::VictoryPoints, 5),
                    (Feature::CustomsBonusNow, 4),
                    (Feature::ImportedGoods, -1),
                    (Feature::AssistantsOnBoard, -1),
                ],
            ),
            ("7", &[]),
            ("11(a)", &[]),
            ("11(b)", &[]),
            (
                "11(c)",
                &[
                    (Feature::AssistantsInWarehouse, -1),
                    (Feature::AssistantsInHandExcess, 1),
                ],
            ),
            ("11(d)", &[(Feature::CountryBonusNow, 2), (Feature::TechBonusNow, 5)]),
            ("11(e)", &[(Feature::HandSize, 1)]),
        ];
        for (id, rows) in expected {
            let declaration = registered(id).expect("registered");
            for stage in declaration.stages {
                let delta = built(id, *stage, &rules).incumbent_delta(&rules);
                for f in Feature::ALL {
                    let want = rows
                        .iter()
                        .find(|(row, _)| *row == f)
                        .map_or(0, |(_, value)| *value);
                    assert_eq!(
                        delta[f.index()],
                        want,
                        "{id} at {}: {} reads {} and the registration says {want}",
                        stage.name(),
                        f.name(),
                        delta[f.index()],
                    );
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // FAMILY 1 — the state facts each registration's rows rest on
    // -----------------------------------------------------------------------

    #[test]
    fn one_a_parks_one_pawn_on_a_productive_area_against_one_on_a_dead_one() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("1(a)", stage, &rules);
            let seat = f.subject();
            let silk = find_area_of_type(f.state(Side::Greater), AreaType::SilkMill).unwrap();
            let copper = find_area_of_type(f.state(Side::Lesser), AreaType::CopperMine).unwrap();
            assert_eq!(occupied_areas(f.state(Side::Greater), seat), vec![silk]);
            assert_eq!(occupied_areas(f.state(Side::Lesser), seat), vec![copper]);
            for side in [Side::Greater, Side::Lesser] {
                let state = f.state(side);
                assert_eq!(
                    state.player(seat).assistants.on_board.values().sum::<u8>(),
                    1,
                    "the single parked pawn is what holds S3/S4/S7 equal",
                );
                assert_eq!(state.player(seat).president_location, PresidentLocation::InHand);
            }
            // §5.2's own arithmetic, read off the engine's reward tables rather
            // than restated: realizable power is 1 assistant + the president's
            // unconditional +1.
            assert_eq!(crate::game::actions::silk_mill_reward(2), 1);
            assert_eq!(crate::game::actions::copper_mine_reward(2), 0);
        }
    }

    #[test]
    fn one_b_holds_the_exported_yield_equal_and_moves_only_the_spread() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("1(b)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);
            let silk = find_area_of_type(greater, AreaType::SilkMill).unwrap();
            let copper = find_area_of_type(greater, AreaType::CopperMine).unwrap();

            // The registered EQUAL exported yield, computed WITH the president
            // +1 — the fold-verification's correction, checked against the
            // engine's tables at the powers the fixture realizes.
            let spread = crate::game::actions::silk_mill_reward(1 + 1)
                + crate::game::actions::copper_mine_reward(1 + 1);
            let stack = crate::game::actions::copper_mine_reward(2 + 1);
            assert_eq!(spread, 1);
            assert_eq!(stack, 1, "split and stack export the SAME goods per activation");

            assert!(
                !greater.board_layout.adjacent_areas(silk).contains(&copper),
                "the two areas are registered NON-ADJACENT",
            );
            assert_eq!(occupied_areas(greater, seat), {
                let mut v = vec![silk, copper];
                v.sort_unstable_by_key(|a| a.0);
                v
            });
            assert_eq!(occupied_areas(lesser, seat), vec![copper]);
            assert_eq!(greater.player(seat).assistants.in_area(copper), 1);
            assert_eq!(lesser.player(seat).assistants.in_area(copper), 2);

            // The registered collateral, as board facts: the stack side owns
            // the larger local maximum (S4's in-hand branch reads a max over
            // own-assistant areas of that area's own count plus its ring), and
            // the spread side owns one more occupied area (S7's in-hand branch
            // is the direct count).
            let local = |state: &GameState, area: AreaId| {
                u32::from(state.player(seat).assistants.in_area(area))
                    + ring_own_assistants(state, seat, area)
            };
            assert_eq!(local(greater, silk).max(local(greater, copper)), 1);
            assert_eq!(local(lesser, copper), 2);
        }
    }

    #[test]
    fn one_c_moves_the_president_over_a_fixed_stack_and_nothing_else() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("1(c)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);

            // "same assistant multiset on one shared layout": byte-identical
            // pawns, byte-identical board.
            assert_eq!(greater.player(seat).assistants, lesser.player(seat).assistants);
            assert_eq!(greater.board_layout.areas, lesser.board_layout.areas);
            assert_eq!(occupied_areas(greater, seat), vec![RING_ASSISTANT_AREA]);
            assert_eq!(greater.player(seat).assistants.in_area(RING_ASSISTANT_AREA), 2);

            // The president's own area is EMPTY on both sides — §5.1's
            // post-Recovery hand-off shape, and the reason clause 9 admits it.
            for (side, area) in
                [(greater, RING_NEAR_PRESIDENT_AREA), (lesser, RING_FAR_PRESIDENT_AREA)]
            {
                assert_eq!(side.player(seat).president_location, PresidentLocation::OnBoard(area));
                assert_eq!(side.player(seat).assistants.in_area(area), 0);
            }

            // RING MASS is the carrier: two assistants one free step from the
            // near president, none from the far one.
            assert_eq!(ring_own_assistants(greater, seat, RING_NEAR_PRESIDENT_AREA), 2);
            assert_eq!(ring_own_assistants(lesser, seat, RING_FAR_PRESIDENT_AREA), 0);
            // S7's listed collateral moves by ONE — the stack is one area.
            assert_eq!(
                own_assistant_reach(greater, seat, RING_NEAR_PRESIDENT_AREA),
                vec![RING_ASSISTANT_AREA],
            );
            assert!(own_assistant_reach(lesser, seat, RING_FAR_PRESIDENT_AREA).is_empty());
        }
    }

    #[test]
    fn one_d_realizes_church_faith_four_at_a_registered_empty_space() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("1(d)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);
            let church = find_area_of_type(greater, AreaType::Church).unwrap();
            let copper = find_area_of_type(lesser, AreaType::CopperMine).unwrap();
            assert_eq!(occupied_areas(greater, seat), vec![church]);
            assert_eq!(occupied_areas(lesser, seat), vec![copper]);

            // The registered stocks, and the faith they realize — through the
            // engine's own faith function, at the power one pawn plus the
            // president's unconditional +1 gives.
            let p = greater.player(seat);
            assert_eq!((p.copper, p.silk, p.tea, p.seafood, p.imported_goods), (1, 1, 0, 0, 0));
            assert_eq!(p.money, 1, "below the 2-Yen yen leg, so it buys no fifth faith");
            assert_eq!(
                crate::game::actions::calculate_church_faith(2, true, true, false, false, false, false),
                4,
                "§5.2's registered realizable total faith",
            );
            // And the stocks the fixture actually carries can afford exactly
            // the two payment types the 4 is built from.
            assert_eq!(
                crate::game::actions::calculate_church_faith(
                    2,
                    p.copper >= 1,
                    p.silk >= 1,
                    p.tea >= 1,
                    p.seafood >= 1,
                    p.money >= 2,
                    p.imported_goods >= 1,
                ),
                4,
            );
            assert_eq!(
                crate::game::actions::eligible_church_spaces(&greater.church_track, 4),
                vec![4],
                "the registered empty eligible space, and the only one at this faith",
            );
            assert!(
                lesser.player(seat).president_location == PresidentLocation::InHand
                    && greater.player(seat).president_location == PresidentLocation::InHand,
            );
        }
    }

    #[test]
    fn one_e_moves_the_president_off_the_head_of_a_three_area_chain() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("1(e)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);

            assert_eq!(greater.player(seat).assistants, lesser.player(seat).assistants);
            assert_eq!(occupied_areas(greater, seat), RING_CHAIN_AREAS.to_vec());
            for area in RING_CHAIN_AREAS {
                assert_eq!(greater.player(seat).assistants.in_area(area), 1);
            }

            // CONNECTED: the whole chain is reachable from the president's ring.
            assert_eq!(
                own_assistant_reach(greater, seat, RING_NEAR_PRESIDENT_AREA),
                RING_CHAIN_AREAS.to_vec(),
                "1 -> 5 -> 8 are free edges, and 1 is one step from area 0",
            );
            // DISCONNECTED: the same pawns, no first step.
            assert!(
                own_assistant_reach(lesser, seat, RING_DISCONNECTED_PRESIDENT_AREA).is_empty(),
                "area 3's ring is {{2, 6}} and neither holds a pawn",
            );
            // S4 rides as the LISTED collateral, and moves by one against S7's
            // three — which is what makes 1(e) a different row from 1(c).
            assert_eq!(ring_own_assistants(greater, seat, RING_NEAR_PRESIDENT_AREA), 1);
            assert_eq!(ring_own_assistants(lesser, seat, RING_DISCONNECTED_PRESIDENT_AREA), 0);
            for (side, area) in [
                (greater, RING_NEAR_PRESIDENT_AREA),
                (lesser, RING_DISCONNECTED_PRESIDENT_AREA),
            ] {
                assert_eq!(side.player(seat).president_location, PresidentLocation::OnBoard(area));
                assert_eq!(side.player(seat).assistants.in_area(area), 0);
            }
        }
    }

    // -----------------------------------------------------------------------
    // FAMILY 11 — the congruence requirement, then each pair's own yield
    // -----------------------------------------------------------------------

    /// §5.2's reflection of the 2P staggered pyramid: (0 3)(1 2)(4 6)(7 8),
    /// fixing 5 and 9. See [`family_11_layout`]'s congruence argument.
    const FAMILY_11_REFLECTION: [u8; TWO_PLAYER_AREAS] = [3, 2, 1, 0, 6, 5, 4, 8, 7, 9];

    #[test]
    fn the_family_eleven_reflection_is_a_board_automorphism() {
        // The congruence claim is a GRAPH claim, so it is checked on the graph:
        // σ must carry every free edge to a free edge. If a future board-shape
        // change broke that, family 11's "S4 and S7 equal by construction"
        // would quietly become "equal by coincidence", and this is the cell
        // that would say so.
        let rules = rules();
        let state = built("11(a)", StageEndpoint::Early, &rules).state(Side::Greater).clone();
        let sigma = |a: AreaId| AreaId(FAMILY_11_REFLECTION[a.0 as usize]);
        assert_eq!(state.board_layout.areas.len(), TWO_PLAYER_AREAS);
        let mut images: Vec<u8> = FAMILY_11_REFLECTION.to_vec();
        images.sort_unstable();
        assert_eq!(images, (0..TWO_PLAYER_AREAS as u8).collect::<Vec<_>>(), "σ is a permutation");
        for (area, neighbours) in &state.board_layout.adjacency {
            for n in neighbours {
                assert!(
                    state.board_layout.adjacent_areas(sigma(*area)).contains(&sigma(*n)),
                    "σ sends the edge {}–{} to {}–{}, which is not an edge",
                    area.0,
                    n.0,
                    sigma(*area).0,
                    sigma(*n).0,
                );
            }
        }
        assert_eq!(sigma(FAMILY_11_HIGH_ACCESS_AREA), FAMILY_11_COPPER_AREA);
        assert_eq!(sigma(FAMILY_11_COPPER_AREA), FAMILY_11_HIGH_ACCESS_AREA);
        // And re-typing moved no edge: the constructed board's adjacency is
        // still the generator's own staggered graph.
        for (a, b) in crate::game::staggered_adjacency(&crate::game::row_lengths(2)) {
            assert!(state.board_layout.adjacent_areas(a).contains(&b));
            assert!(state.board_layout.adjacent_areas(b).contains(&a));
        }
        assert!(state.board_layout.canal_crossings.is_empty(), "no Canal at 2P");
    }

    #[test]
    fn every_family_eleven_pair_is_congruent_across_its_two_sides() {
        // The registered requirement, per pair and per endpoint: σ carries the
        // greater side's own-assistant position onto the lesser side's, so the
        // two ring sums and the two reachable sets are EQUAL rather than
        // separately computed to the same number.
        let rules = rules();
        let sigma = |a: AreaId| AreaId(FAMILY_11_REFLECTION[a.0 as usize]);
        for id in ["11(a)", "11(b)", "11(c)", "11(d)", "11(e)"] {
            let declaration = registered(id).expect("registered");
            for stage in declaration.stages {
                let f = built(id, *stage, &rules);
                let seat = f.subject();
                let greater = f.state(Side::Greater);
                let lesser = f.state(Side::Lesser);

                // The acted area is drained and the other one holds the
                // survivor — the post-Recovery shape, mirrored across the sides.
                assert_eq!(occupied_areas(greater, seat), vec![FAMILY_11_COPPER_AREA], "{id}");
                assert_eq!(
                    occupied_areas(lesser, seat),
                    vec![FAMILY_11_HIGH_ACCESS_AREA],
                    "{id}"
                );
                assert_eq!(
                    greater.player(seat).president_location,
                    PresidentLocation::OnBoard(FAMILY_11_HIGH_ACCESS_AREA),
                    "{id}: the actor's president stands where it acted",
                );
                assert_eq!(
                    lesser.player(seat).president_location,
                    PresidentLocation::OnBoard(FAMILY_11_COPPER_AREA),
                    "{id}",
                );

                // σ maps one position onto the other, pawn for pawn.
                for area in greater.board_layout.areas.keys() {
                    assert_eq!(
                        greater.player(seat).assistants.in_area(*area),
                        lesser.player(seat).assistants.in_area(sigma(*area)),
                        "{id} at {}: σ does not carry area {} onto area {}",
                        stage.name(),
                        area.0,
                        sigma(*area).0,
                    );
                }

                // So the two quantities the requirement names are equal, and
                // the ring is not empty — an all-zero congruence would satisfy
                // the equality while measuring nothing.
                let g_ring = ring_own_assistants(greater, seat, FAMILY_11_HIGH_ACCESS_AREA);
                let l_ring = ring_own_assistants(lesser, seat, FAMILY_11_COPPER_AREA);
                assert_eq!(g_ring, l_ring, "{id}: the ring sums differ");
                assert_eq!(g_ring, 1, "{id}: the two acted areas are neighbours, so the \
                     survivor stands in the actor's ring on both sides");
                assert_eq!(
                    own_assistant_reach(greater, seat, FAMILY_11_HIGH_ACCESS_AREA).len(),
                    own_assistant_reach(lesser, seat, FAMILY_11_COPPER_AREA).len(),
                    "{id}: the reachable sets differ",
                );
                assert_eq!(
                    own_assistant_reach(greater, seat, FAMILY_11_HIGH_ACCESS_AREA).len(),
                    1,
                    "{id}",
                );
            }
        }
    }

    #[test]
    fn eleven_a_trades_one_silk_of_access_for_one_silk_of_stock() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("11(a)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);
            assert_eq!(
                greater.board_layout.areas[&FAMILY_11_HIGH_ACCESS_AREA].area_type,
                AreaType::SilkMill,
            );
            assert_eq!(greater.player(seat).silk, lesser.player(seat).silk + 1);
            assert_eq!(crate::game::actions::silk_mill_reward(2), 1, "the realized yield");
            // The conservation the family exists to state: the winner keeps the
            // 0-reward pawn and the loser keeps the 1-reward pawn.
            assert_eq!(crate::game::actions::copper_mine_reward(2), 0);
            // And the registered order card is what keeps the extra silk out of
            // O2/O3/X3: it needs none.
            let card = rules.orders.get(FIXTURE_ORDER_CARD).expect("registered card");
            assert_eq!(card.requirements.silk, 0);
            assert!(card.requirements.copper <= 1);
        }
    }

    #[test]
    fn eleven_b_trades_two_yen_of_access_for_two_yen_of_stock() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("11(b)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);
            assert_eq!(
                greater.board_layout.areas[&FAMILY_11_HIGH_ACCESS_AREA].area_type,
                AreaType::Bank,
            );
            assert_eq!(crate::game::actions::bank_reward(2), 2);
            assert_eq!(greater.player(seat).money, lesser.player(seat).money + 2);
        }
    }

    #[test]
    fn eleven_c_trades_one_retrievable_token_for_one_retrieved_pawn() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("11(c)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);
            assert_eq!(
                greater.board_layout.areas[&FAMILY_11_HIGH_ACCESS_AREA].area_type,
                AreaType::EmploymentAgency,
            );
            assert_eq!(crate::game::actions::employment_agency_tokens_allowed(2), 1);
            let g = &greater.player(seat).assistants;
            let l = &lesser.player(seat).assistants;
            assert_eq!(g.in_warehouse + 1, l.in_warehouse, "warehouse only ever drains");
            assert_eq!(g.in_hand, l.in_hand + 1, "and Recovery-and-retrieval land in HAND");
            assert_eq!(g.total(), 20);
            assert_eq!(l.total(), 20);
            // The registration's "hand-core/excess" resolves to EXCESS: the
            // hand is already past the two-pawn core reserve on both sides.
            assert!(l.in_hand >= crate::game::eval::HAND_CORE_RESERVE);
        }
    }

    #[test]
    fn eleven_d_buys_at_zero_deficit_from_the_one_takeable_slot() {
        let rules = rules();
        let lab = FAMILY_11_HIGH_ACCESS_AREA;
        for stage in StageEndpoint::ALL {
            let f = built("11(d)", stage, &rules);
            let seat = f.subject();
            let rival = other_seat_2p(f.state(Side::Greater), seat).unwrap();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);
            assert_eq!(greater.board_layout.areas[&lab].area_type, AreaType::LaboratoryA);

            // The third power point, and where it comes from.
            assert_eq!(lesser.station_location, Some(lab));
            assert!(owns_effect(lesser, rival, TechEffect::Station, &rules));
            assert!(!owns_effect(lesser, seat, TechEffect::Station, &rules));
            assert_eq!(lesser.player(seat).assistants.in_area(lab), 1);
            // THREE, and cross-checked against the engine rather than restated.
            // `calculate_power` credits the president only where it actually
            // stands, and §3.2's `realizable_power` credits it EVERYWHERE — so
            // the number X1 will read at the lesser side's laboratory is the
            // number the engine returns for the same board with the president
            // standing there. That board is also the one the greater side's
            // purchase was made on, before its Recovery drained the pawn.
            let mut acting = lesser.clone();
            acting.player_mut(seat).president_location = PresidentLocation::OnBoard(lab);
            let power = crate::game::calculate_power(&acting, seat, lab);
            assert_eq!(
                power, 3,
                "one own assistant + the president + the station token, capped last",
            );
            // The post-Recovery states themselves read one lower at the lab,
            // which is the conservation this whole family is about.
            assert_eq!(crate::game::calculate_power(greater, seat, lab), 2);

            // Exactly ONE zero-deficit slot at that power, with money 0 and no
            // imports — which is the whole of X1 = 1 on the lesser side.
            assert_eq!(lesser.player(seat).money, 0);
            assert_eq!(lesser.player(seat).imported_goods, 0);
            let zero_deficit: Vec<usize> = (0..6)
                .filter(|slot| {
                    let id = lesser.lab_state.lab_a.slots[*slot].expect("a stocked slot");
                    let cost = rules.techs.get(id).expect("registry").industry_cost;
                    cost + crate::game::actions::lab_position_surcharge(*slot) <= power
                })
                .collect();
            assert_eq!(zero_deficit, vec![PURCHASED_SLOT], "{stage:?}");

            // The purchase itself: one more technology, of the registered
            // effect, at the registered industry cost.
            let owned_g = &greater.player(seat).technologies;
            let owned_l = &lesser.player(seat).technologies;
            assert_eq!(owned_l.len(), 1, "≤ 1 technology PRE-purchase");
            assert_eq!(owned_g.len(), 2, "and 2 after — never the 3 that owes D2's duty");
            assert!(owns_effect(greater, seat, PURCHASED_TECH, &rules));
            assert!(!owns_effect(lesser, seat, PURCHASED_TECH, &rules));
            let industry = |state: &GameState| -> u8 {
                state
                    .player(seat)
                    .technologies
                    .iter()
                    .map(|id| rules.techs.get(*id).expect("registry").industry_cost)
                    .sum()
            };
            assert_eq!(industry(lesser), 6, "the pre-owned technology's industry value");
            assert_eq!(industry(greater), 9, "past the 2P dummy's 7.5 — D5's comparison");

            // The registered icon parity: every country the seat holds is at an
            // ODD count post-purchase, so `check_mercenary_eligibility`'s sweep
            // has nothing to award.
            let mut counts = std::collections::BTreeMap::new();
            for id in owned_g {
                *counts.entry(rules.techs.get(*id).expect("registry").country).or_insert(0) += 1;
            }
            assert_eq!(counts.len(), 2, "two DISTINCT countries — the flag is a new one");
            for (country, n) in counts {
                assert_eq!(n % 2, 1, "{country:?} sits at an even icon count");
            }
            assert!(greater.player(seat).completed_orders.is_empty(), "no order icons either");
            // Technologies4 is not in play, per the registration.
            assert!(!greater
                .achievements
                .iter()
                .any(|c| c.achievement_type == AchievementType::Technologies4));
        }
    }

    #[test]
    fn eleven_e_takes_a_free_offer_and_fires_the_engines_own_trigger() {
        let rules = rules();
        let f = built("11(e)", StageEndpoint::Late, &rules);
        let seat = f.subject();
        let greater = f.state(Side::Greater);
        let lesser = f.state(Side::Lesser);
        assert_eq!(
            greater.board_layout.areas[&FAMILY_11_HIGH_ACCESS_AREA].area_type,
            AreaType::PortA,
        );

        // The lesser side still SEES the offer in the Low zone — index 0, the
        // only zone realizable power 2 opens — and the greater side holds it.
        assert_eq!(lesser.port_state.port_a.slots[TAKEN_PORT_SLOT], Some(FIXTURE_PORT_CARD));
        assert!(greater.player(seat).order_cards_in_hand.contains(&FIXTURE_PORT_CARD));
        assert_eq!(greater.player(seat).order_cards_in_hand.len(), 2);
        assert_eq!(lesser.player(seat).order_cards_in_hand.len(), 1);
        assert_eq!(greater.player(seat).money, lesser.player(seat).money, "the first card is free");

        // Total compaction, then a refill that finds nothing: five offers left
        // and the sixth slot open, which is exactly the trigger's condition.
        assert!(greater.order_deck.is_empty());
        assert_eq!(greater.port_state.port_a.slots.iter().flatten().count(), 5);
        assert!(greater.port_state.port_a.slots[5].is_none());
        assert!(port_is_full(lesser));
        assert!(greater.end_game_triggered, "the engine's own check fired it");
        assert_eq!(greater.trigger_reason, Some(crate::types::EndGameTrigger::OrderDeckEmpty));
        assert_eq!(greater.trigger_player, Some(seat));
        assert!(!greater.final_round, "and a trigger is not a finished game");
        assert!(!lesser.end_game_triggered, "the untaking side's port is still full");
        assert!(
            eval::terminal_standing(f.view(Side::Greater), &rules).is_none(),
            "clause 6 already refused a terminal side; this says WHY it is not one",
        );

        // The taken card's registered properties: it matches goods the seat
        // holds (so O2 moves), needs more than the seat has (so O3 does not),
        // and carries no icon face (so O6 does not).
        let card = rules.orders.get(FIXTURE_PORT_CARD).expect("registered card");
        let p = greater.player(seat);
        assert!(
            u32::from(card.requirements.tea) > p.tea
                || u32::from(card.requirements.seafood) > p.seafood,
            "the taken card stays unexecutable, so O3 does not move",
        );
        assert!(
            u32::from(card.requirements.tea).min(p.tea) > 0,
            "and it matches a good the seat holds, so O2 does",
        );
        assert_eq!((card.rewards.movement, card.rewards.warehouse_tokens), (0, 0));
    }

    // -----------------------------------------------------------------------
    // FAMILY 2 — the state facts each registration's rows rest on
    // -----------------------------------------------------------------------

    /// The subject's four trade-good stocks on one side, in the order
    /// `(copper, silk, tea, seafood)`.
    fn stocks(state: &GameState, seat: PlayerId) -> (u32, u32, u32, u32) {
        let p = state.player(seat);
        (p.copper, p.silk, p.tea, p.seafood)
    }

    /// One seat's OWN value of an incumbent feature on one side — the
    /// un-differenced half of what `incumbent_delta` subtracts.
    ///
    /// A difference of +4 is consistent with 8 → 12 and with 0 → 4, and the
    /// registrations that turn on a mirror's ABSOLUTE level (6(c)'s set step,
    /// 6(d)'s second place) mean the first. This is how those say so.
    fn own_value(f: &PairFixture, side: Side, seat: PlayerId, feature: Feature, rules: &Rules) -> i64 {
        let view = f.view(side);
        let structure = ScoreStructure::derive(view, rules);
        let player = view.players.iter().find(|p| p.id == seat).expect("the seat has a row");
        structure.value_of(player, feature)
    }

    #[test]
    fn two_a_takes_the_silk_mills_two_goods_and_leaves_the_presence_standing() {
        let rules = rules();
        let f = built("2(a)", StageEndpoint::Late, &rules);
        let seat = f.subject();
        let greater = f.state(Side::Greater);
        let lesser = f.state(Side::Lesser);
        let silk = find_area_of_type(greater, AreaType::SilkMill).unwrap();

        // The registered yield, read off the engine's own table at the power
        // the fixture realizes: two assistants plus the president's +1.
        assert_eq!(crate::game::actions::silk_mill_reward(2 + 1), 2);
        assert_eq!(stocks(greater, seat), (0, 2, 0, 0));
        assert_eq!(stocks(lesser, seat), (0, 0, 0, 0));

        for side in [Side::Greater, Side::Lesser] {
            let state = f.state(side);
            // PRESENCE UNCHANGED is the phase's doing, not the fixture's:
            // Recovery has not run at FivePowerBonus.
            assert_eq!(state.player(seat).assistants.in_area(silk), 2);
            assert_eq!(occupied_areas(state, seat), vec![silk]);
            assert_eq!(state.turn_phase, TurnPhase::FivePowerBonus);
            assert_eq!(
                state.current_action_power,
                Some(3),
                "the resolved-area-action phase carries the power the president arrived with",
            );
            assert_eq!(state.player(seat).president_location, PresidentLocation::OnBoard(silk));
            assert!(state.pending_choices.is_empty(), "a production action queues nothing");
        }
    }

    #[test]
    fn two_b_moves_three_pawns_out_of_the_warehouse_and_nothing_else() {
        let rules = rules();
        let f = built("2(b)", StageEndpoint::Early, &rules);
        let seat = f.subject();
        let greater = f.state(Side::Greater);
        let lesser = f.state(Side::Lesser);

        let mut want: Vec<AreaId> = PRODUCTIVE_TRIO
            .iter()
            .map(|t| find_area_of_type(greater, *t).unwrap())
            .collect();
        want.sort_unstable_by_key(|a| a.0);
        assert_eq!(occupied_areas(greater, seat), want);
        assert!(occupied_areas(lesser, seat).is_empty());

        let g = greater.player(seat);
        let l = lesser.player(seat);
        assert_eq!(u32::from(g.assistants.in_warehouse) + 3, u32::from(l.assistants.in_warehouse));
        assert_eq!(g.assistants.in_hand, l.assistants.in_hand, "the HAND is what stays equal");
        assert_eq!(g.assistants.total(), 20);
        assert_eq!(l.assistants.total(), 20);

        // The registered exported yield of the trio, at one pawn plus the
        // president's +1 apiece.
        let per_area: u8 = PRODUCTIVE_TRIO
            .iter()
            .map(|t| crate::game::actions::exported_reward(*t, 2).unwrap())
            .sum();
        assert_eq!(per_area, 5, "silk(2) + tea(2) + fishing(2) = 1 + 2 + 2");
    }

    #[test]
    fn two_c_is_two_bs_construction_with_the_pawns_coming_from_hand() {
        let rules = rules();
        let b = built("2(b)", StageEndpoint::Early, &rules);
        let c = built("2(c)", StageEndpoint::Early, &rules);
        let seat = c.subject();
        assert_eq!(b.subject(), seat);

        // The two GREATER sides are the same deployment; only the pawns'
        // origin differs, which is the whole comparison.
        assert_eq!(
            occupied_areas(b.state(Side::Greater), seat),
            occupied_areas(c.state(Side::Greater), seat),
        );
        let g = c.state(Side::Greater).player(seat);
        let l = c.state(Side::Lesser).player(seat);
        assert_eq!(u32::from(g.assistants.in_hand) + 3, u32::from(l.assistants.in_hand));
        assert_eq!(g.assistants.in_warehouse, l.assistants.in_warehouse, "the WAREHOUSE stays equal");
        // V2's core reserve is full on both sides at a hand of eight, so the
        // whole −3 lands in the excess bucket rather than splitting.
        assert!(g.assistants.in_hand >= 2 && l.assistants.in_hand >= 2);
        assert_eq!(c.declaration().role, PairRole::Diagnostic);
    }

    #[test]
    fn two_d_binds_the_employment_agencys_min_on_the_warehouse_basket() {
        let rules = rules();
        let f = built("2(d)", StageEndpoint::Early, &rules);
        let seat = f.subject();
        let ea = find_area_of_type(f.state(Side::Greater), AreaType::EmploymentAgency).unwrap();

        // The cap the basket has to bind against, from the engine's own table.
        assert_eq!(crate::game::actions::employment_agency_tokens_allowed(5), 4);

        let basket = |side: Side| {
            let p = f.state(side).player(seat);
            u32::from(p.assistants.in_warehouse)
                + u32::from(p.shophouses.in_warehouse)
                + u32::from(p.trading_houses.in_warehouse)
        };
        assert_eq!(basket(Side::Lesser), 4, "the lesser basket sits exactly AT the cap");
        assert_eq!(basket(Side::Greater), 3, "and the retrieval drops it below");

        for side in [Side::Greater, Side::Lesser] {
            let p = f.state(side).player(seat);
            assert_eq!(p.assistants.in_area(ea), 4, "four pawns is what buys power 5");
            assert_eq!(p.assistants.in_warehouse, FAMILY_2D_WAREHOUSE_ASSISTANTS);
            assert_eq!(p.trading_houses.in_warehouse, 0, "a trading house left here would unbind the min");
            assert_eq!(p.trading_houses.in_hand, 4);
            assert_eq!(p.shophouses.total_built(), 0);
        }
        let g = f.state(Side::Greater).player(seat);
        let l = f.state(Side::Lesser).player(seat);
        assert_eq!((g.shophouses.in_hand, g.shophouses.in_warehouse), (8, 0));
        assert_eq!((l.shophouses.in_hand, l.shophouses.in_warehouse), (7, 1));
        assert_eq!(g.money, l.money, "§5.2: the fixture sets equal money");
    }

    // -----------------------------------------------------------------------
    // FAMILY 3 — the state facts each registration's rows rest on
    // -----------------------------------------------------------------------

    #[test]
    fn three_a_moves_one_yen_and_stands_no_pawn_anywhere() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("3(a)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);

            assert_eq!(greater.player(seat).money, FAMILY_3A_GREATER_MONEY);
            assert_eq!(lesser.player(seat).money, FAMILY_3A_LESSER_MONEY);
            // The even-money realization the registration records: the
            // terminal conversion is an integer floor, and this step realizes
            // nothing. `money / 2` is scoring.rs's rate, restated here as the
            // arithmetic the note is about.
            assert_eq!(
                FAMILY_3A_GREATER_MONEY / 2,
                FAMILY_3A_LESSER_MONEY / 2,
                "the registered variant is the one whose realized terminal delta is ZERO",
            );

            for side in [Side::Greater, Side::Lesser] {
                let state = f.state(side);
                assert!(
                    occupied_areas(state, seat).is_empty(),
                    "'all else identical' is built as no presence at all",
                );
                assert_eq!(state.player(seat).president_location, PresidentLocation::InHand);
                assert_eq!(stocks(state, seat), stocks(greater, seat), "goods stay equal");
            }
            // Everything but money is byte-identical, which is what makes the
            // pair a pure E1 separator.
            assert_ne!(state_fingerprint(greater), state_fingerprint(lesser));
            assert_eq!(f.declaration().draws.moves_yen, true);
            assert_ne!(f.declaration().draws.category_b, AchievementType::Money10);
        }
    }

    #[test]
    fn three_b_empties_the_hand_by_fulfilling_and_writes_the_tea_back() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("3(b)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);

            for side in [Side::Greater, Side::Lesser] {
                let state = f.state(side);
                let p = state.player(seat);
                assert!(p.order_cards_in_hand.is_empty(), "§5.2: hand constructed empty");
                assert_eq!(
                    p.starting_order,
                    Some(FIXTURE_ORDER_CARD),
                    "the recorded identity survives the fulfillment",
                );
                assert!(
                    p.completed_orders.contains(&FIXTURE_ORDER_CARD),
                    "I11's second arm is where an emptied starting order lives",
                );
                assert!(state.pending_choices.is_empty(), "the registered card queues nothing");
                assert!(occupied_areas(state, seat).is_empty());
            }
            assert_eq!(stocks(greater, seat), (0, 0, FAMILY_3B_GREATER_TEA, 0));
            assert_eq!(stocks(lesser, seat), (0, 0, FAMILY_3B_LESSER_TEA, 0));
            assert_eq!(
                f.declaration().draws.category_a,
                AchievementType::Copper5,
                "the A-leg must read a good the recipe does not move",
            );
        }
    }

    #[test]
    fn three_c_parks_one_pawn_at_the_bank_against_one_on_a_dead_area() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("3(c)", stage, &rules);
            let seat = f.subject();
            let bank = find_area_of_type(f.state(Side::Greater), AreaType::Bank).unwrap();
            let copper = find_area_of_type(f.state(Side::Lesser), AreaType::CopperMine).unwrap();

            assert_eq!(occupied_areas(f.state(Side::Greater), seat), vec![bank]);
            assert_eq!(occupied_areas(f.state(Side::Lesser), seat), vec![copper]);
            // §5.2's own arithmetic, off the engine's tables at the realizable
            // power one pawn plus the president's +1 buys.
            assert_eq!(crate::game::actions::bank_reward(2), 2);
            assert_eq!(crate::game::actions::copper_mine_reward(2), 0);
            // The Bank is not production, so the greater side's S1 is zero too
            // — the pair moves S5 alone.
            assert!(crate::game::actions::exported_reward(AreaType::Bank, 2).is_none());

            for side in [Side::Greater, Side::Lesser] {
                let state = f.state(side);
                assert_eq!(state.player(seat).assistants.on_board.values().sum::<u8>(), 1);
                assert_eq!(state.player(seat).president_location, PresidentLocation::InHand);
                assert_eq!(
                    stocks(state, seat),
                    (
                        FAMILY_3_STOCKS[0],
                        FAMILY_3_STOCKS[1],
                        FAMILY_3_STOCKS[2],
                        FAMILY_3_STOCKS[3],
                    ),
                    "§5.2: stocks equal, and registered rather than dealt",
                );
            }
        }
    }

    #[test]
    fn three_d_binds_the_employment_agencys_power_cap_against_a_full_warehouse() {
        let rules = rules();
        let f = built("3(d)", StageEndpoint::Early, &rules);
        let seat = f.subject();
        let ea = find_area_of_type(f.state(Side::Greater), AreaType::EmploymentAgency).unwrap();
        let copper = find_area_of_type(f.state(Side::Lesser), AreaType::CopperMine).unwrap();

        assert_eq!(occupied_areas(f.state(Side::Greater), seat), vec![ea]);
        assert_eq!(occupied_areas(f.state(Side::Lesser), seat), vec![copper]);
        assert_eq!(crate::game::actions::employment_agency_tokens_allowed(2), 1);

        for side in [Side::Greater, Side::Lesser] {
            let p = f.state(side).player(seat);
            // §5.2's "warehouse non-empty declared, matching S6's min-gate" —
            // the untouched setup basket, so the CAP is what binds the min.
            assert_eq!(p.assistants.in_warehouse, 12);
            assert_eq!(p.shophouses.in_warehouse, 6);
            assert_eq!(p.trading_houses.in_warehouse, 4);
            assert_eq!(p.assistants.on_board.values().sum::<u8>(), 1);
            assert_eq!(p.president_location, PresidentLocation::InHand);
        }
        // Nothing is taken: 3(d) is the ACCESS pair and 11(c) is the take.
        let g = f.state(Side::Greater).player(seat);
        let l = f.state(Side::Lesser).player(seat);
        assert_eq!(g.assistants.in_hand, l.assistants.in_hand);
        assert_eq!(g.money, l.money);
    }

    // -----------------------------------------------------------------------
    // FAMILY 6 — the C2 controls
    // -----------------------------------------------------------------------

    #[test]
    fn six_a_fulfills_through_the_engine_and_leaves_the_icon_queued() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("6(a)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);
            let card = rules.orders.get(FIXTURE_FULFILLED_CARD).expect("the registered card");

            assert!(lesser.player(seat).order_cards_in_hand.contains(&FIXTURE_FULFILLED_CARD));
            assert!(greater.player(seat).completed_orders.contains(&FIXTURE_FULFILLED_CARD));
            assert!(greater.player(seat).order_cards_in_hand.is_empty());

            // The requirements were in stock on the lesser side and are spent
            // on the greater — exactly the printed amounts, no more.
            let r = card.requirements;
            assert_eq!(
                stocks(lesser, seat),
                (0, u32::from(r.silk), u32::from(r.tea), u32::from(r.seafood)),
            );
            assert_eq!(stocks(greater, seat), (0, 0, 0, 0));

            // The registered queue state: ONE warehouse take, unresolved.
            assert!(lesser.pending_choices.is_empty());
            assert_eq!(greater.pending_choices, vec![crate::types::PendingChoice::WarehouseTake]);
            assert_eq!(u32::from(card.rewards.warehouse_tokens), 1);

            // The inert parity: one German icon after, so no award fires.
            assert_eq!(
                (greater.mercenary_supply.german, lesser.mercenary_supply.german),
                (2, 2),
                "the shared German supply stands full on both sides: no award fired, and                  it was not an exhausted supply that stopped it",
            );
            for side in [Side::Greater, Side::Lesser] {
                assert_eq!(f.state(side).player(seat).mercenaries.total(), 0);
                assert_eq!(f.state(side).turn_phase, TurnPhase::AdditionalActionFirst);
            }
        }
    }

    #[test]
    fn six_b_claims_money_ten_out_of_the_hands_excess() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("6(b)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);

            assert_eq!(greater.achievements[1].achievement_type, AchievementType::Money10);
            assert!(greater.achievements[1].claimed_by.contains(&seat), "claimed, for FIRST place");
            assert!(lesser.achievements[1].claimed_by.is_empty());
            assert_eq!(AchievementType::Money10.first_place_vp(), 9);

            for side in [Side::Greater, Side::Lesser] {
                assert_eq!(f.state(side).player(seat).money, 10, "the claim costs no money");
            }
            let g = greater.player(seat);
            let l = lesser.player(seat);
            assert_eq!(u32::from(g.assistants.in_hand) + 1, u32::from(l.assistants.in_hand));
            assert_eq!(u32::from(g.assistants.on_achievements), u32::from(l.assistants.on_achievements) + 1);
            assert!(g.assistants.in_hand > 2, "past the core reserve, so the EXCESS bucket pays");
            assert_eq!(stocks(greater, seat), stocks(lesser, seat), "the E group is untouched");
        }
    }

    #[test]
    fn six_c_closes_the_fifth_country_set() {
        let rules = rules();
        for stage in StageEndpoint::ALL {
            let f = built("6(c)", stage, &rules);
            let seat = f.subject();
            let greater = f.state(Side::Greater);
            let lesser = f.state(Side::Lesser);

            let countries = |state: &GameState| {
                let mut cs: Vec<crate::types::Country> = state
                    .player(seat)
                    .completed_orders
                    .iter()
                    .map(|id| rules.orders.get(*id).expect("a completed order").country)
                    .collect();
                cs.sort_unstable_by_key(|c| format!("{c:?}"));
                cs.dedup();
                cs
            };
            assert_eq!(countries(lesser).len(), 4, "four distinct before");
            assert_eq!(countries(greater).len(), 5, "and the Dutch card closes the set");
            assert_eq!(lesser.player(seat).completed_orders.len(), 4);
            assert_eq!(greater.player(seat).completed_orders.len(), 5);
            assert!(lesser.player(seat).order_cards_in_hand.contains(&FIXTURE_SET_CARD));

            // The registered card queues nothing — 6(a) is where the icon case
            // lives.
            assert!(greater.pending_choices.is_empty());
            assert_eq!(
                rules.orders.get(FIXTURE_SET_CARD).expect("the registered card").rewards.warehouse_tokens,
                0,
            );
            // No mercenary on either side: every country's icon count is odd.
            assert_eq!(greater.player(seat).mercenaries.total(), 0);

            // The registered set step, at its ABSOLUTE levels — 8 → 12 is the
            // 2/4/8/12 curve's fifth-country move, and it is the only step
            // where the +4 the delta pin sees also collapses O5's marginal.
            assert_eq!(own_value(&f, Side::Lesser, seat, Feature::CountryBonusNow, &rules), 8);
            assert_eq!(own_value(&f, Side::Greater, seat, Feature::CountryBonusNow, &rules), 12);
            let rival = other_seat_2p(greater, seat).unwrap();
            assert_eq!(
                own_value(&f, Side::Greater, rival, Feature::CountryBonusNow, &rules),
                0,
                "the opponent completes nothing, so the relative form is the subject's own",
            );
        }
    }

    #[test]
    fn six_d_church_pays_the_area_first_pawn_onto_faith_four() {
        let rules = rules();
        let f = built("6(d-church)", StageEndpoint::Late, &rules);
        let seat = f.subject();
        let greater = f.state(Side::Greater);
        let lesser = f.state(Side::Lesser);
        let church = find_area_of_type(greater, AreaType::Church).unwrap();

        // D6 area-first: the pawn came off the ACTED AREA, not out of hand.
        assert_eq!(greater.player(seat).assistants.in_area(church), 0);
        assert_eq!(lesser.player(seat).assistants.in_area(church), 1);
        assert_eq!(
            greater.player(seat).assistants.in_hand,
            lesser.player(seat).assistants.in_hand,
            "the hand is untouched — the correction D6 makes to the pre-rev-7 model",
        );
        assert_eq!(greater.player(seat).assistants.on_church, 1);
        assert_eq!(lesser.player(seat).assistants.on_church, 0);

        // The registered space, and the faith arithmetic that forced it.
        let index = crate::types::ChurchTrack::faith_to_index(CHURCH_FAITH_SPACE).unwrap();
        assert!(greater.church_track.slots[index].is_some_and(|s| s.player == seat));
        assert!(lesser.church_track.slots[index].is_none());
        assert_eq!(crate::types::ChurchTrack::vp_reward(CHURCH_FAITH_SPACE), 3);
        assert_eq!(
            crate::game::actions::calculate_church_faith(2, true, true, false, false, false, false),
            CHURCH_FAITH_SPACE,
            "min(power 2, 5) + one per distinct payment type IS the registered total faith",
        );
        // SECOND place, not first: the dummy at faith-7 ties on count and wins
        // on RIGHTMOST, consuming the 6 and leaving the 3.
        assert_eq!(own_value(&f, Side::Greater, seat, Feature::ChurchBonusNow, &rules), 3);
        assert_eq!(own_value(&f, Side::Lesser, seat, Feature::ChurchBonusNow, &rules), 0);

        // The payment, and the empty queue the declined rider leaves.
        assert_eq!(stocks(lesser, seat), (1, 1, 0, 0));
        assert_eq!(stocks(greater, seat), (0, 0, 0, 0));
        for side in [Side::Greater, Side::Lesser] {
            assert!(f.state(side).pending_choices.is_empty());
            assert_eq!(f.state(side).turn_phase, TurnPhase::FivePowerBonus);
            assert_eq!(
                f.state(side).player(seat).president_location,
                PresidentLocation::OnBoard(church),
            );
        }
    }

    #[test]
    fn six_d_customs_answers_the_queued_space_choice_with_the_registered_one() {
        let rules = rules();
        let f = built("6(d-customs)", StageEndpoint::Late, &rules);
        let seat = f.subject();
        let greater = f.state(Side::Greater);
        let lesser = f.state(Side::Lesser);
        let customs = find_area_of_type(greater, AreaType::Customs).unwrap();

        assert_eq!(greater.player(seat).assistants.in_area(customs), 0);
        assert_eq!(lesser.player(seat).assistants.in_area(customs), 1);
        assert_eq!(greater.player(seat).assistants.on_customs, 1);
        assert_eq!(greater.player(seat).imported_goods, 0);
        assert_eq!(lesser.player(seat).imported_goods, 1);
        assert_eq!(crate::game::actions::customs_max_imported_goods(2), 1);

        let index = crate::types::CustomsTrack::space_to_index(CUSTOMS_SPACE).unwrap();
        assert!(greater.customs_track.slots[index].is_some_and(|s| s.player == seat));
        assert_eq!(crate::types::CustomsTrack::vp_reward(CUSTOMS_SPACE), 5);
        assert_eq!(own_value(&f, Side::Greater, seat, Feature::CustomsBonusNow, &rules), 4);
        assert_eq!(own_value(&f, Side::Lesser, seat, Feature::CustomsBonusNow, &rules), 0);
        // The registered space is the one whose riders are warehouse takes,
        // and they were DECLINED — so nothing left the warehouse.
        assert_eq!(
            greater.player(seat).assistants.in_warehouse,
            lesser.player(seat).assistants.in_warehouse,
        );
        assert_eq!(greater.player(seat).money, lesser.player(seat).money, "space 1 pays no yen rider");
        for side in [Side::Greater, Side::Lesser] {
            assert!(f.state(side).pending_choices.is_empty());
            assert_eq!(f.state(side).turn_phase, TurnPhase::FivePowerBonus);
        }
    }

    // -----------------------------------------------------------------------
    // FAMILY 7 — anti-parking
    // -----------------------------------------------------------------------

    #[test]
    fn seven_is_two_as_construction_at_one_pawn() {
        let rules = rules();
        let f = built("7", StageEndpoint::Late, &rules);
        let seat = f.subject();
        let silk = find_area_of_type(f.state(Side::Greater), AreaType::SilkMill).unwrap();

        assert_eq!(crate::game::actions::silk_mill_reward(1 + 1), 1);
        assert_eq!(stocks(f.state(Side::Greater), seat), (0, 1, 0, 0));
        assert_eq!(stocks(f.state(Side::Lesser), seat), (0, 0, 0, 0));
        for side in [Side::Greater, Side::Lesser] {
            let state = f.state(side);
            assert_eq!(state.player(seat).assistants.in_area(silk), 1);
            assert_eq!(state.current_action_power, Some(2));
        }
        // The two pairs are the same shape at different sizes, which is what
        // keeps family 7 from being 2(a) re-registered at a weaker margin.
        let two_a = built("2(a)", StageEndpoint::Late, &rules);
        assert_ne!(
            state_fingerprint(f.state(Side::Greater)),
            state_fingerprint(two_a.state(Side::Greater)),
        );
        assert_eq!(f.declaration().margin, MarginClass::Weak);
    }

    // -----------------------------------------------------------------------
    // FAMILY 8 — recovery/hand ordering
    // -----------------------------------------------------------------------

    #[test]
    fn eight_recalls_the_president_and_the_last_two_pawns_through_the_engine() {
        let rules = rules();
        let f = built("8", StageEndpoint::Late, &rules);
        let seat = f.subject();
        let greater = f.state(Side::Greater);
        let lesser = f.state(Side::Lesser);

        // The STRANDED side: president on an own-empty area, both pawns in its
        // free ring, and both areas exhausted by the engine's own arithmetic.
        assert_eq!(
            lesser.player(seat).president_location,
            PresidentLocation::OnBoard(FAMILY_8_PRESIDENT_AREA),
        );
        assert_eq!(
            lesser.player(seat).assistants.in_area(FAMILY_8_PRESIDENT_AREA),
            0,
            "§5.1's hand-off rule bars a seat's president from its own assistants",
        );
        assert_eq!(occupied_areas(lesser, seat), FAMILY_8_STRANDED_AREAS.to_vec());
        assert_eq!(
            ring_own_assistants(lesser, seat, FAMILY_8_PRESIDENT_AREA),
            2,
            "both pawns are one free step away — this is what makes S4 move",
        );
        assert_eq!(
            own_assistant_reach(lesser, seat, FAMILY_8_PRESIDENT_AREA),
            FAMILY_8_STRANDED_AREAS.to_vec(),
            "and both are reachable — this is what makes S7 move",
        );

        // EXHAUSTED, as arithmetic. The Copper Mine yields nothing at the
        // realizable power one pawn plus the president's +1 buys, and the
        // Laboratory is unaffordable at the registered purse.
        assert_eq!(crate::game::actions::copper_mine_reward(2), 0);
        assert_eq!(lesser.player(seat).money, FAMILY_8_SUBJECT_MONEY);
        assert_eq!(FAMILY_8_SUBJECT_MONEY / 2, 0, "the all-in purse buys no deficit at all");
        assert_eq!(lesser.player(seat).imported_goods, 0);

        // The RECALLED side: nothing on the board, president in hand, and the
        // two pawns landed in the hand rather than in the warehouse.
        assert_eq!(greater.player(seat).president_location, PresidentLocation::InHand);
        assert!(occupied_areas(greater, seat).is_empty());
        assert_eq!(
            u32::from(greater.player(seat).assistants.in_hand),
            u32::from(lesser.player(seat).assistants.in_hand) + 2,
        );
        assert_eq!(
            greater.player(seat).assistants.in_warehouse,
            lesser.player(seat).assistants.in_warehouse,
            "recovery returns pawns to HAND; the warehouse is a one-way source",
        );
        // The registration's own resolved reading: the refill lands in EXCESS.
        assert!(lesser.player(seat).assistants.in_hand >= eval::HAND_CORE_RESERVE);
        assert_eq!(greater.player(seat).assistants.total(), 20);
        assert_eq!(lesser.player(seat).assistants.total(), 20);

        // And the recall is the ENGINE's, not a restatement of it: applying
        // `ReturnPresidentToHand` to the stranded layout reproduces the
        // greater side's president location and assistant distribution.
        let recalled = recall_through_the_engine(lesser, seat, &rules)
            .expect("the stranded layout admits its own recall");
        assert_eq!(recalled.president_location, greater.player(seat).president_location);
        assert_eq!(recalled.assistants, greater.player(seat).assistants);
    }

    // -----------------------------------------------------------------------
    // FAMILY 10 — crossed access-and-need
    // -----------------------------------------------------------------------

    #[test]
    fn ten_a_permutes_the_same_six_offers_between_the_low_and_high_zones() {
        let rules = rules();
        let f = built("10(a)", StageEndpoint::Early, &rules);
        let seat = f.subject();
        let greater = f.state(Side::Greater);
        let lesser = f.state(Side::Lesser);
        let port = find_area_of_type(greater, AreaType::PortA).unwrap();

        let slots = |state: &GameState| -> Vec<OrderCardId> {
            state.port_state.port_a.slots.iter().flatten().copied().collect()
        };
        assert_eq!(slots(greater), FAMILY_10A_PORT_OFFERS.to_vec());
        assert_eq!(
            slots(lesser),
            vec![
                FAMILY_10A_PORT_OFFERS[4],
                FAMILY_10A_PORT_OFFERS[5],
                FAMILY_10A_PORT_OFFERS[2],
                FAMILY_10A_PORT_OFFERS[3],
                FAMILY_10A_PORT_OFFERS[0],
                FAMILY_10A_PORT_OFFERS[1],
            ],
            "the LOW pair and the HIGH pair trade places and nothing else moves",
        );
        // A permutation, so the located SET — what O1's pool complements — is
        // identical, which is the registration's own equality argument.
        let mut g = slots(greater);
        let mut l = slots(lesser);
        g.sort_unstable_by_key(|c| c.0);
        l.sort_unstable_by_key(|c| c.0);
        assert_eq!(g, l);

        // The zone map the permutation exploits, off the engine's own helpers.
        assert_eq!(crate::game::actions::port_accessible_zones(2), vec![crate::types::PortZone::Low]);
        for (index, zone) in [
            (0, crate::types::PortZone::Low),
            (1, crate::types::PortZone::Low),
            (4, crate::types::PortZone::High),
            (5, crate::types::PortZone::High),
        ] {
            assert_eq!(crate::types::PortBoard::zone_of(index), zone);
        }

        // The match arithmetic X2 reads, at the registered stocks.
        let matched = |card: OrderCardId| -> u32 {
            let r = rules.orders.get(card).expect("a printed face").requirements;
            [
                (FAMILY_10A_STOCKS[0], u32::from(r.copper)),
                (FAMILY_10A_STOCKS[1], u32::from(r.silk)),
                (FAMILY_10A_STOCKS[2], u32::from(r.tea)),
                (FAMILY_10A_STOCKS[3], u32::from(r.seafood)),
            ]
            .iter()
            .map(|(stock, req)| (*stock).min(*req))
            .sum()
        };
        assert_eq!(matched(FAMILY_10A_PORT_OFFERS[0]), 1);
        assert_eq!(matched(FAMILY_10A_PORT_OFFERS[1]), 1);
        assert_eq!(matched(FAMILY_10A_PORT_OFFERS[4]), 0);
        assert_eq!(matched(FAMILY_10A_PORT_OFFERS[5]), 0);

        for side in [Side::Greater, Side::Lesser] {
            let state = f.state(side);
            assert_eq!(occupied_areas(state, seat), vec![port]);
            assert_eq!(
                stocks(state, seat),
                (
                    FAMILY_10A_STOCKS[0],
                    FAMILY_10A_STOCKS[1],
                    FAMILY_10A_STOCKS[2],
                    FAMILY_10A_STOCKS[3],
                ),
            );
            assert_eq!(state.player(seat).order_cards_in_hand.len(), 1, "clear of the hand cap");
            // §5.2 registers the OPPONENT as holding no port presence.
            let other = other_seat_2p(state, seat).unwrap();
            assert_eq!(state.player(other).assistants.in_area(port), 0);
        }
    }

    #[test]
    fn ten_b_permutes_the_same_six_lab_cards_between_a_free_and_a_surcharged_slot() {
        let rules = rules();
        let f = built("10(b)", StageEndpoint::Early, &rules);
        let seat = f.subject();
        let lab = find_area_of_type(f.state(Side::Greater), AreaType::LaboratoryA).unwrap();

        let effect_at = |state: &GameState, slot: usize| -> TechEffect {
            let id = state.lab_state.lab_a.slots[slot].expect("a stocked slot");
            rules.techs.iter().find(|c| c.id == id).expect("a printed card").effect
        };
        assert_eq!(effect_at(f.state(Side::Greater), FAMILY_10B_CHEAP_SLOT), FAMILY_10B_LAB_STOCK[0]);
        assert_eq!(effect_at(f.state(Side::Lesser), FAMILY_10B_DEAR_SLOT), FAMILY_10B_LAB_STOCK[0]);
        assert_eq!(effect_at(f.state(Side::Lesser), FAMILY_10B_CHEAP_SLOT), FAMILY_10B_LAB_STOCK[4]);

        // The surcharge table and the deficit arithmetic, off the engine.
        assert_eq!(crate::game::actions::lab_position_surcharge(FAMILY_10B_CHEAP_SLOT), 0);
        assert_eq!(crate::game::actions::lab_position_surcharge(FAMILY_10B_DEAR_SLOT), 2);
        let cost_of = |effect: TechEffect| -> u8 {
            rules.techs.iter().find(|c| c.effect == effect).expect("a printed card").industry_cost
        };
        assert_eq!(cost_of(FAMILY_10B_LAB_STOCK[0]), 3, "the cheapest printed industry value");
        let purse = FAMILY_10B_MONEY / 2;
        assert_eq!(purse, 1, "§5.2's registered all-in purse");
        let deficit = |effect: TechEffect, slot: usize| -> u32 {
            u32::from(cost_of(effect) + crate::game::actions::lab_position_surcharge(slot))
                .saturating_sub(2)
        };
        assert_eq!(deficit(FAMILY_10B_LAB_STOCK[0], FAMILY_10B_CHEAP_SLOT), 1);
        assert_eq!(deficit(FAMILY_10B_LAB_STOCK[0], FAMILY_10B_DEAR_SLOT), 3);
        // Every OTHER slot is out of reach on BOTH sides, which is what makes
        // X1 read 1 against 0 rather than n+1 against n.
        for (slot, effect) in FAMILY_10B_LAB_STOCK.iter().enumerate().skip(1) {
            assert!(
                deficit(*effect, slot) > purse,
                "slot {slot} ({effect:?}) is takeable and would ride both sides",
            );
        }

        for side in [Side::Greater, Side::Lesser] {
            let state = f.state(side);
            assert_eq!(occupied_areas(state, seat), vec![lab]);
            assert_eq!(state.player(seat).money, FAMILY_10B_MONEY);
            assert_eq!(state.player(seat).imported_goods, 0);
            assert!(state.player(seat).technologies.is_empty(), "the 4th-card duty stays off");
            let other = other_seat_2p(state, seat).unwrap();
            assert_eq!(state.player(other).assistants.in_area(lab), 0);
        }
    }

    #[test]
    fn ten_c_spreads_the_same_eight_goods_over_four_types_against_one() {
        let rules = rules();
        let f = built("10(c)", StageEndpoint::Early, &rules);
        let seat = f.subject();
        let church = find_area_of_type(f.state(Side::Greater), AreaType::Church).unwrap();

        assert_eq!(f.declaration().role, PairRole::Diagnostic);
        let g = f.state(Side::Greater).player(seat);
        let l = f.state(Side::Lesser).player(seat);
        assert_eq!(
            (g.copper, g.silk, g.tea, g.seafood),
            (
                FAMILY_10C_SPREAD_STOCKS[0],
                FAMILY_10C_SPREAD_STOCKS[1],
                FAMILY_10C_SPREAD_STOCKS[2],
                FAMILY_10C_SPREAD_STOCKS[3],
            ),
        );
        assert_eq!(
            (l.copper, l.silk, l.tea, l.seafood),
            (
                FAMILY_10C_CONCENTRATED_STOCKS[0],
                FAMILY_10C_CONCENTRATED_STOCKS[1],
                FAMILY_10C_CONCENTRATED_STOCKS[2],
                FAMILY_10C_CONCENTRATED_STOCKS[3],
            ),
        );
        assert_eq!(
            g.total_trade_goods(),
            l.total_trade_goods(),
            "E2 is a linear sum: eight goods either way",
        );
        assert_eq!((g.money, g.imported_goods), (FAMILY_10C_MONEY, FAMILY_10C_IMPORTS));
        assert_eq!((l.money, l.imported_goods), (FAMILY_10C_MONEY, FAMILY_10C_IMPORTS));

        // The residual the diagnostic exists to print, through the engine's
        // own faith arithmetic at the shared realizable power.
        let faith = |p: &crate::types::PlayerState| {
            crate::game::actions::calculate_church_faith(
                2,
                p.copper >= 1,
                p.silk >= 1,
                p.tea >= 1,
                p.seafood >= 1,
                p.money >= 2,
                p.imported_goods >= 1,
            )
        };
        assert_eq!(faith(g), 8, "power 2 plus all SIX distinct payment types");
        assert_eq!(faith(l), 5, "power 2 plus copper, yen and import");
        assert_eq!(faith(g) - faith(l), 3, "§5.2's residual, at the maximum an equal-row pair allows");

        for side in [Side::Greater, Side::Lesser] {
            let state = f.state(side);
            assert_eq!(occupied_areas(state, seat), vec![church]);
            assert!(state.player(seat).order_cards_in_hand.is_empty(), "any held order moves O2");
            assert_eq!(state.player(seat).president_location, PresidentLocation::InHand);
        }
        // The Category-A leg saturates on BOTH sides, which is the trick that
        // keeps `AchievementProgressSum` from firing on the copper difference.
        for side in [Side::Greater, Side::Lesser] {
            let (current, needed) = crate::game::achievement_progress(
                f.state(side),
                seat,
                AchievementType::Copper5,
            );
            assert_eq!(current.min(needed), needed, "min(copper, 5) is at its cap on both sides");
        }
    }

    #[test]
    fn ten_d_crosses_one_fishing_ground_pawn_with_two_face_equal_orders() {
        let rules = rules();
        let f = built("10(d)", StageEndpoint::Early, &rules);
        let seat = f.subject();
        let fishing = find_area_of_type(f.state(Side::Greater), AreaType::FishingGround).unwrap();

        assert_eq!(
            f.state(Side::Greater).player(seat).order_cards_in_hand,
            vec![FAMILY_10D_MATCHED_CARD],
        );
        assert_eq!(
            f.state(Side::Lesser).player(seat).order_cards_in_hand,
            vec![FAMILY_10D_UNMATCHED_CARD],
        );

        let matched = rules.orders.get(FAMILY_10D_MATCHED_CARD).expect("card 26");
        let unmatched = rules.orders.get(FAMILY_10D_UNMATCHED_CARD).expect("card 5");
        // §5.2's four equalities, each read off the printed faces.
        assert_eq!(matched.country, unmatched.country, "equal country: O5's set is identical");
        let half_vp = |o: &crate::types::OrderCard| {
            2 * u32::from(o.rewards.victory_points)
                + 2 * u32::from(o.rewards.imported_goods)
                + u32::from(o.rewards.money)
        };
        assert_eq!(half_vp(matched), 4);
        assert_eq!(half_vp(unmatched), 4, "equal face in O1's own half-VP units");
        for o in [matched, unmatched] {
            assert_eq!((o.rewards.movement, o.rewards.warehouse_tokens), (0, 0), "equal icons");
        }
        // X3's arithmetic at ZERO stocks, where the residual need IS the
        // printed requirement.
        assert_eq!(crate::game::actions::fishing_ground_reward(2), 2);
        assert_eq!(u32::from(matched.requirements.seafood).min(2), 1);
        assert_eq!(u32::from(unmatched.requirements.seafood).min(2), 0);

        for side in [Side::Greater, Side::Lesser] {
            let state = f.state(side);
            assert_eq!(occupied_areas(state, seat), vec![fishing]);
            assert_eq!(stocks(state, seat), (0, 0, 0, 0), "§5.2: ZERO stocks");
            assert_eq!(state.player(seat).president_location, PresidentLocation::InHand);
            // Neither registered card may sit at a face-up offer: the located
            // set is what O1's pool complements, and a port swap would move it.
            assert_eq!(port_a_slot_of(state, FAMILY_10D_MATCHED_CARD), None);
            assert_eq!(port_a_slot_of(state, FAMILY_10D_UNMATCHED_CARD), None);
        }
        // And the port itself is byte-identical across the sides.
        let offers = |side: Side| f.state(side).port_state.port_a.slots;
        assert_eq!(offers(Side::Greater), offers(Side::Lesser));
    }

    // -----------------------------------------------------------------------
    // The hand-built layouts as objects
    // -----------------------------------------------------------------------

    #[test]
    fn a_hand_built_layout_is_a_permutation_of_the_generated_tile_set() {
        let rules = rules();
        let mut state = keep_game_2p(FIXTURE_SEED, &rules).expect("setup");
        let generated: Vec<AreaType> =
            state.board_layout.areas.values().map(|c| c.area_type).collect();
        retype_board_2p(&mut state, RING_LAYOUT).expect("the ring layout is a permutation");
        let mut before: Vec<String> = generated.iter().map(|t| format!("{t:?}")).collect();
        let mut after: Vec<String> = state
            .board_layout
            .areas
            .values()
            .map(|c| format!("{:?}", c.area_type))
            .collect();
        before.sort();
        after.sort();
        assert_eq!(before, after, "the multiset is the engine's, not the layout's");
        check_invariants(&state, &rules).expect("a re-typed 2P board is engine-valid");

        // And a layout that is NOT a permutation is refused rather than seated.
        let mut illegal = RING_LAYOUT;
        illegal[0] = AreaType::SilkMill; // two Silk Mills, no Copper Mine
        let err = retype_board_2p(&mut state, illegal).expect_err("multiset violation");
        assert!(format!("{err}").contains("multiset"), "got: {err}");
    }

    #[test]
    fn family_elevens_layout_seats_each_pairs_own_high_access_type() {
        for (id, high) in [
            ("11(a)", AreaType::SilkMill),
            ("11(b)", AreaType::Bank),
            ("11(c)", AreaType::EmploymentAgency),
            ("11(d)", AreaType::LaboratoryA),
            ("11(e)", AreaType::PortA),
        ] {
            let layout = family_11_layout(high).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(layout[FAMILY_11_HIGH_ACCESS_AREA.0 as usize], high, "{id}");
            assert_eq!(layout[FAMILY_11_COPPER_AREA.0 as usize], AreaType::CopperMine, "{id}");
            let mut seen: Vec<String> = layout.iter().map(|t| format!("{t:?}")).collect();
            seen.sort();
            seen.dedup();
            assert_eq!(seen.len(), TWO_PLAYER_AREAS, "{id}: every 2P type exactly once");
        }
        // The Copper Mine is the LESSER side's area by registration, so it is
        // not a high-access type and the builder says so.
        let err = family_11_layout(AreaType::CopperMine).expect_err("refused");
        assert!(format!("{err}").contains("CopperMine"), "got: {err}");
    }

    #[test]
    fn every_registered_pair_renders_a_report_over_the_full_axis() {
        let rules = rules();
        for pair in REGISTERED.iter() {
            let declaration = registered(pair.id).expect("registered");
            let stage = declaration.stages[0];
            let report = render_pair_report(&built(pair.id, stage, &rules), &rules);
            assert!(report.contains(pair.id), "{}: the id", pair.id);
            assert!(report.contains(&package_digest()), "{}: the digest", pair.id);
            for row in FeatureId::all() {
                assert!(report.contains(row.name()), "{}: {} has no row", pair.id, row.name());
            }
            assert_eq!(
                report.matches(CANDIDATE_PENDING).count(),
                CandidateFeature::COUNT,
                "{}: every candidate column is still PENDING at step 2",
                pair.id,
            );
        }
    }

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
        // A §5.2 family number is required of every registration (1..=11); a
        // throwaway borrows one without being that family, which is what the
        // name and the note say out loud.
        family: 1,
        greater_side: "one-vp",
        lesser_side: "zero-vp",
        role: PairRole::Diagnostic,
        stages: &[StageEndpoint::Early, StageEndpoint::Late],
        view_class: ViewClass::HandOff,
        margin: MarginClass::DirectionOnly,
        carrier: Some(FeatureId::Incumbent(Feature::VictoryPoints)),
        presidents: SidedPresidents::BOTH_IN_HAND,
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
        presidents: SidedPresidents::symmetric(SeatPresidents {
            subject: PresidentDecl::OnBoardEmptyArea,
            opponent: PresidentDecl::InHand,
        }),
        ..THROWAWAY_VP
    };

    /// A throwaway declaring the subject's president on-board WITH its own
    /// assistants and NO Telephone exception — §5.1's unreachable shape.
    const THROWAWAY_PRESIDENT_OCCUPIED: PairDeclaration = PairDeclaration {
        id: "T(pres-occupied)",
        presidents: SidedPresidents::symmetric(SeatPresidents {
            subject: PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: false },
            opponent: PresidentDecl::InHand,
        }),
        ..THROWAWAY_VP
    };

    /// The same, WITH the exception claimed.
    const THROWAWAY_PRESIDENT_TELEPHONE: PairDeclaration = PairDeclaration {
        id: "T(pres-telephone)",
        presidents: SidedPresidents::symmetric(SeatPresidents {
            subject: PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: true },
            opponent: PresidentDecl::InHand,
        }),
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

    /// Write the three declared achievement cards onto a state — the
    /// registered constructions' own [`set_declared_draws`], so the throwaways
    /// exercise the function the ten registered pairs actually use.
    fn set_draws(state: &mut GameState, draws: &AchievementDraws) {
        set_declared_draws(state, draws);
    }

    /// The two sides of `THROWAWAY_VP` at one endpoint: identical states except
    /// one victory point on the greater side.
    fn vp_pair(stage: StageEndpoint, rules: &Rules) -> (GameState, GameState, PlayerId) {
        let mut lesser = stage_base_2p(stage, SEED, rules).expect("a stage base builds");
        set_draws(&mut lesser, &THROWAWAY_DRAWS);
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
    fn the_registry_holds_the_landed_families_and_is_pinned_to_the_spec_target() {
        assert_eq!(
            REGISTERED.len(),
            REGISTERED_PAIR_COUNT,
            "the array's length IS the count pin",
        );
        assert_eq!(
            REGISTERED_PAIR_COUNT, 29,
            "S2-T4 registered §5.2's family 1 (5) and family 11 (5); S2-T5 adds family 2 \
             (3 gates + 1 diagnostic), family 6 (5) and family 7 (1); S2-T6 adds family 3 \
             (4), family 8 (1) and family 10 (3 gates + 1 diagnostic). The remaining \
             families land in later tasks, and this number is what they bump toward \
             {REGISTERED_PAIR_TARGET}",
        );
        assert_eq!(
            REGISTERED_PAIR_TARGET, 36,
            "§5.1's registered counts: 34 gate pairs plus the 2(c)/10(c) diagnostics",
        );
        let ids: Vec<&str> = REGISTERED.iter().map(|p| p.id).collect();
        assert_eq!(
            ids,
            vec![
                "1(a)",
                "1(b)",
                "1(c)",
                "1(d)",
                "1(e)",
                "2(a)",
                "2(b)",
                "2(d)",
                "2(c)",
                "3(a)",
                "3(b)",
                "3(c)",
                "3(d)",
                "6(a)",
                "6(b)",
                "6(c)",
                "6(d-church)",
                "6(d-customs)",
                "7",
                "8",
                "10(a)",
                "10(b)",
                "10(c)",
                "10(d)",
                "11(a)",
                "11(b)",
                "11(c)",
                "11(d)",
                "11(e)",
            ],
            "§5.2's own registration order — including family 2's, which prints its three \
             gates (a)/(b)/(d) before the (c) diagnostic, and family 10's, which prints its \
             (c) diagnostic BETWEEN the gates because that is where §5.2 writes it",
        );
        for pair in REGISTERED.iter() {
            // 2(c) and 10(c) are §5.1's two "RECORDED DIAGNOSTIC, never a gate"
            // rows; everything else registered so far is a constraint row.
            let expected = if matches!(pair.id, "2(c)" | "10(c)") {
                PairRole::Diagnostic
            } else {
                PairRole::Constraint
            };
            assert_eq!(pair.role, expected, "{} has the wrong pair→gate role", pair.id);
            assert!(
                matches!(pair.family, 1 | 2 | 3 | 6 | 7 | 8 | 10 | 11),
                "{} claims family {}",
                pair.id,
                pair.family
            );
        }
    }

    #[test]
    fn every_registered_pair_is_uniquely_named_and_internally_consistent() {
        let mut ids: Vec<&str> = REGISTERED.iter().map(|p| p.id).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "two registered pairs share an id");
        for pair in REGISTERED.iter() {
            pair.check_consistency().unwrap_or_else(|e| panic!("{e}"));
        }
        // The registry is EMPTY today, so the loop above proves nothing on its
        // own — the classic way a registry-hygiene test rots into a no-op. The
        // throwaways below are real declarations exercising every arm of the
        // rule, so the guard is proven working NOW rather than at whichever
        // family task first trips it.
        for pair in THROWAWAYS {
            pair.check_consistency()
                .unwrap_or_else(|e| panic!("a throwaway is incoherent: {e}"));
        }
    }

    /// Every throwaway declaration in this file — the non-vacuity material for
    /// the registry-hygiene guard, and the reason each arm of
    /// `check_consistency` is exercised while `REGISTERED` is still empty.
    const THROWAWAYS: [&PairDeclaration; 9] = [
        &THROWAWAY_VP,
        &THROWAWAY_VP_UNLISTED,
        &THROWAWAY_PRESIDENT_EMPTY,
        &THROWAWAY_PRESIDENT_OCCUPIED,
        &THROWAWAY_PRESIDENT_TELEPHONE,
        &THROWAWAY_MID_TURN,
        &THROWAWAY_DRAW_CONSTRAINED,
        &THROWAWAY_DRAWS_MONEY10,
        &THROWAWAY_TECH_EXCLUDED,
    ];

    #[test]
    fn an_incoherent_registration_is_refused_before_any_state_is_read() {
        // A margin at an endpoint the pair does not declare: the registration
        // would price a row no system reads.
        const DANGLING: PairDeclaration = PairDeclaration {
            id: "T(dangling)",
            stages: &[StageEndpoint::Late],
            margin: MarginClass::Margined {
                early: Some(1),
                late: Some(14),
                carrier: FeatureId::Candidate(CandidateFeature::ProductionAccess),
            },
            ..THROWAWAY_VP
        };
        let err = DANGLING.check_consistency().expect_err("early is priced but not declared");
        assert!(format!("{err}").contains("does not declare"), "got: {err}");

        // A declared endpoint with no margin: no inequality for the solve.
        const UNPRICED: PairDeclaration = PairDeclaration {
            id: "T(unpriced)",
            stages: &[StageEndpoint::Early, StageEndpoint::Late],
            margin: MarginClass::Margined {
                early: None,
                late: Some(14),
                carrier: FeatureId::Candidate(CandidateFeature::ProductionAccess),
            },
            ..THROWAWAY_VP
        };
        let err = UNPRICED.check_consistency().expect_err("early is declared but not priced");
        assert!(format!("{err}").contains("no margin"), "got: {err}");

        // A row listed twice, and a family outside §5.2's range.
        const DOUBLED: PairDeclaration = PairDeclaration {
            id: "T(doubled)",
            expected_nonzero: &[
                FeatureId::Incumbent(Feature::VictoryPoints),
                FeatureId::Incumbent(Feature::VictoryPoints),
            ],
            ..THROWAWAY_VP
        };
        assert!(format!("{}", DOUBLED.check_consistency().expect_err("doubled row"))
            .contains("twice"));
        const FAMILY_0: PairDeclaration =
            PairDeclaration { id: "T(fam0)", family: 0, ..THROWAWAY_VP };
        assert!(format!("{}", FAMILY_0.check_consistency().expect_err("family 0"))
            .contains("family 0"));

        // And the constructor runs it FIRST, before it reads either state.
        let rules = rules();
        let (greater, lesser, subject) = vp_pair(StageEndpoint::Late, &rules);
        let err =
            PairFixture::new(&DANGLING, StageEndpoint::Late, greater, lesser, subject, &rules)
                .expect_err("clause 0");
        assert!(format!("{err}").contains("registration coherence"), "got: {err}");
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
        let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Late, &rules);
        // The engine's own terminal shape (`eval::view_is_terminal`): triggered,
        // final round, a recorded trigger player, and the turn resting at the
        // end of the seat before the start player. Both sides move the MOVER to
        // that seat so the refusal is about terminality rather than about which
        // seat signs the view (clause 4 runs first, and correctly).
        let seats = i32::from(greater.config.player_count);
        let last = (i32::from(greater.config.start_player.0) + seats - 1).rem_euclid(seats);
        lesser.current_player = PlayerId(last as u8);
        greater.end_game_triggered = true;
        greater.final_round = true;
        greater.trigger_player = Some(greater.config.start_player);
        // The engine's own trigger-coherence clause wants the reason too, and
        // the deck is genuinely empty on a LATE base.
        greater.trigger_reason = Some(crate::types::EndGameTrigger::OrderDeckEmpty);
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
        assert!(format!("{err}").contains("2P-only"), "got: {err}");
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

    /// FAMILY 6(d)'s SHAPE — the M1 fix wave's subject.
    ///
    /// Acting-class, so the subject's president stands at the acted area on
    /// BOTH sides; the PLACED (greater) side's area has been drained by the
    /// track placement, the UNPLACED (lesser) side still holds its one
    /// assistant. Nothing about the president moves — only what stands beneath
    /// it — so the two sides need DIFFERENT [`PresidentDecl`]s.
    const THROWAWAY_ACTING_ASYMMETRIC: PairDeclaration = PairDeclaration {
        id: "T(6d-shape)",
        presidents: SidedPresidents {
            greater: SeatPresidents {
                subject: PresidentDecl::OnBoardEmptyArea,
                opponent: PresidentDecl::InHand,
            },
            lesser: SeatPresidents {
                subject: PresidentDecl::OnBoardWithOwnAssistants { telephone_excepted: false },
                opponent: PresidentDecl::InHand,
            },
        },
        // 6(d) is MID-TURN (phase = AreaAction resolved), which is also what
        // makes the occupied-area side legal without Telephone: Recovery has
        // not run.
        view_class: ViewClass::MidTurn { phase: TurnPhase::AreaAction, mercenary: false },
        stages: &[StageEndpoint::Early],
        // The drained pawn returns to a hand already holding 7, so it lands
        // PAST the two-assistant core reserve — `AssistantsInHandExcess` moves
        // and `AssistantsInHandCore` does not. Clause 13 caught this list
        // getting it backwards at authoring, which is the guard doing its job.
        expected_nonzero: &[
            FeatureId::Incumbent(Feature::VictoryPoints),
            FeatureId::Incumbent(Feature::AssistantsOnBoard),
            FeatureId::Incumbent(Feature::AssistantsInHandExcess),
        ],
        ..THROWAWAY_VP
    };

    /// The 6(d)-shaped pair: president on-board at one area both sides, the
    /// assistant beneath it present on the lesser side and gone on the greater.
    ///
    /// The drained pawn goes to HAND rather than to a track, deliberately: a
    /// real track placement would move the track clock off the early endpoint,
    /// and this fixture's subject is the DECLARATION shape, not 6(d)'s own
    /// arithmetic. The president-relevant fact — an occupied area on one side
    /// and an empty one on the other, with the president fixed — is identical.
    fn acting_asymmetric_pair(rules: &Rules) -> (GameState, GameState, PlayerId) {
        let mut lesser = early_base_2p(SEED, rules).expect("early base");
        set_draws(&mut lesser, &THROWAWAY_DRAWS);
        let subject = lesser.current_player;
        let area = some_non_canal_area(&lesser);
        place_one(&mut lesser, subject, area);
        lesser.player_mut(subject).president_location = PresidentLocation::OnBoard(area);
        lesser.turn_phase = TurnPhase::AreaAction;

        let mut greater = lesser.clone();
        // The acted area is drained on the greater side.
        let p = greater.player_mut(subject);
        p.assistants.on_board.remove(&area);
        p.assistants.in_hand += 1;
        p.victory_points += 1;
        (greater, lesser, subject)
    }

    #[test]
    fn a_pair_may_declare_a_different_president_shape_on_each_side() {
        // §5.2's family 6(d) and family 8 both need this: the president does
        // not move, but what stands beneath it differs across the sides, so a
        // single per-pair declaration cannot describe both.
        let rules = rules();
        let (greater, lesser, subject) = acting_asymmetric_pair(&rules);
        PairFixture::new(
            &THROWAWAY_ACTING_ASYMMETRIC,
            StageEndpoint::Early,
            greater,
            lesser,
            subject,
            &rules,
        )
        .unwrap_or_else(|e| panic!("the 6(d) shape must be declarable: {e}"));
    }

    #[test]
    fn a_sided_president_declaration_still_binds_each_side_separately() {
        // The other direction: per-side expressiveness must not become
        // per-side laxity. Swapping the two declarations refuses BOTH sides.
        const SWAPPED: PairDeclaration = PairDeclaration {
            id: "T(6d-swapped)",
            presidents: SidedPresidents {
                greater: SeatPresidents {
                    subject: PresidentDecl::OnBoardWithOwnAssistants {
                        telephone_excepted: false,
                    },
                    opponent: PresidentDecl::InHand,
                },
                lesser: SeatPresidents {
                    subject: PresidentDecl::OnBoardEmptyArea,
                    opponent: PresidentDecl::InHand,
                },
            },
            ..THROWAWAY_ACTING_ASYMMETRIC
        };
        let rules = rules();
        let (greater, lesser, subject) = acting_asymmetric_pair(&rules);
        let err =
            PairFixture::new(&SWAPPED, StageEndpoint::Early, greater, lesser, subject, &rules)
                .expect_err("the greater side's area is empty, not occupied");
        assert!(format!("{err}").contains("president"), "got: {err}");
        assert!(format!("{err}").contains("greater"), "the GREATER side is the one named: {err}");
    }

    #[test]
    fn the_symmetric_constructor_declares_the_same_shape_on_both_sides() {
        let both = SidedPresidents::symmetric(SeatPresidents::BOTH_IN_HAND);
        assert_eq!(both, SidedPresidents::BOTH_IN_HAND, "§5.1's default, spelled two ways");
        assert_eq!(both.side(Side::Greater), both.side(Side::Lesser));
        let sided = THROWAWAY_ACTING_ASYMMETRIC.presidents;
        assert_ne!(
            sided.side(Side::Greater),
            sided.side(Side::Lesser),
            "the 6(d) shape is asymmetric by construction",
        );
    }

    #[test]
    fn a_hand_off_refuses_a_state_whose_once_per_turn_flags_are_not_reset() {
        // The seat advance resets `additional_action_state` to `new()`
        // (apply_move.rs:278) in the same breath as it sets the phase, so a
        // hand-off carrying a used once-per-turn flag is not a state the
        // machine produces. Checking the phase alone let one through.
        let rules = rules();
        for dirty in [
            AdditionalActionState { mercenary_used: true, ..AdditionalActionState::new() },
            AdditionalActionState { spinning_mill_used: true, ..AdditionalActionState::new() },
            AdditionalActionState { university_used: true, ..AdditionalActionState::new() },
        ] {
            let (mut greater, mut lesser, subject) = vp_pair(StageEndpoint::Early, &rules);
            for side in [&mut greater, &mut lesser] {
                side.additional_action_state = dirty.clone();
            }
            let err = PairFixture::new(
                &THROWAWAY_VP,
                StageEndpoint::Early,
                greater,
                lesser,
                subject,
                &rules,
            )
            .expect_err("a hand-off is the POST-ADVANCE shape, flags included");
            assert!(format!("{err}").contains("view class"), "got: {err}");
        }
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
        // Two IDENTICAL sides: nothing moves, so `new`'s own direction (a
        // realized nonzero row must be listed) is vacuously satisfied and the
        // fixture admits. `victory_points` is nonetheless LISTED, which is the
        // other direction — and the one that only completes at step 3, so it
        // is a call a caller makes rather than a constructor clause.
        let mut lesser = early_base_2p(SEED, &rules).expect("early base");
        set_draws(&mut lesser, &THROWAWAY_DRAWS);
        let subject = lesser.current_player;
        let greater = lesser.clone();
        let fixture =
            PairFixture::new(&THROWAWAY_VP, StageEndpoint::Early, greater, lesser, subject, &rules)
                .expect("identical sides admit: no row moves, so none is unlisted");
        let err = fixture
            .listed_incumbent_rows_are_nonzero(&rules)
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
    // The drift pin: a registered field that reaches neither the digest nor
    // the report
    // -----------------------------------------------------------------------

    /// This module's own source, for the two structural pins below.
    ///
    /// Source scanning is this repo's standard answer to a hand-maintained
    /// relation the compiler cannot check — `tests/eval_core.rs:135` reads
    /// `eval.rs` for exactly that reason, and `Feature::ALL`'s doc names the
    /// same hazard ("a variant given its match arms but left out of `ALL`
    /// compiles clean").
    const SOURCE: &str = include_str!("pair_fixtures.rs");

    /// The `pub` field names of one struct in [`SOURCE`].
    fn fields_of(struct_name: &str) -> Vec<String> {
        let head = format!("pub struct {struct_name} {{");
        let start = SOURCE.find(&head).unwrap_or_else(|| panic!("{struct_name} is declared here"))
            + head.len();
        let body = &SOURCE[start..];
        let end = body.find("\n}").expect("the struct closes");
        body[..end]
            .lines()
            .filter_map(|line| {
                let t = line.trim_start();
                let rest = t.strip_prefix("pub ")?;
                let name = rest.split(':').next()?.trim();
                (!name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
                    .then(|| name.to_string())
            })
            .collect()
    }

    /// One function's body in [`SOURCE`], from its signature to the first
    /// column-zero `}`.
    fn body_of(fn_name: &str) -> &'static str {
        let head = format!("pub fn {fn_name}");
        let start =
            SOURCE.find(&head).unwrap_or_else(|| panic!("{fn_name} is declared here"));
        let body = &SOURCE[start..];
        let end = body.find("\n}").expect("the function closes");
        &body[..end]
    }

    /// The declaration structs NESTED inside [`PairDeclaration`], each with the
    /// field path a serializer or renderer must reach it by.
    ///
    /// SHARED by both drift guards deliberately: the digest guard scanned these
    /// from the start and the report guard did not, so a future edit dropping
    /// `opponent=` from the printed PRES line or `moved=` from DRAWS would have
    /// passed. One list, two readers, no way for the two to diverge again.
    const NESTED_DECLARATION_FIELDS: [(&str, &str); 5] = [
        ("AchievementDraws", "draws"),
        ("TechDeclaration", "techs"),
        ("SidedPresidents", "presidents"),
        ("SeatPresidents", "presidents.greater"),
        ("SeatPresidents", "presidents.lesser"),
    ];

    #[test]
    fn every_registered_field_reaches_the_digest() {
        // §5.1's digest is "a digest over the registered tuples". A field added
        // to a declaration but never serialized would let a registration change
        // meaning without moving the hash — the one failure mode a content hash
        // exists to prevent, and one no type check catches.
        let canon = body_of("canonical_declaration");
        for field in fields_of("PairDeclaration") {
            assert!(
                canon.contains(&format!("pair.{field}")),
                "PairDeclaration::{field} never reaches canonical_declaration, so the package \
                 digest does not cover it",
            );
        }
        for (owner, prefix) in NESTED_DECLARATION_FIELDS {
            for field in fields_of(owner) {
                assert!(
                    canon.contains(&format!("{prefix}.{field}")),
                    "{owner}::{field} never reaches canonical_declaration",
                );
            }
        }
    }

    #[test]
    fn every_registered_field_reaches_the_printed_report() {
        // The other half of the same hazard: §5.1 has every Part-A/B artifact
        // print the pair's declaration, and a field the report omits is a fact
        // the bounded re-read never sees.
        let report = body_of("render_pair_report_with");
        for field in fields_of("PairDeclaration") {
            assert!(
                report.contains(&format!("d.{field}")),
                "PairDeclaration::{field} is never printed by render_pair_report_with",
            );
        }
        for (owner, prefix) in NESTED_DECLARATION_FIELDS {
            for field in fields_of(owner) {
                assert!(
                    report.contains(&format!("{prefix}.{field}")),
                    "{owner}::{field} is never printed by render_pair_report_with",
                );
            }
        }
    }

    #[test]
    fn the_field_scanner_actually_finds_fields() {
        // The pins above are vacuous if the scanner returns nothing, which is
        // exactly how a source-scanning guard rots.
        let fields = fields_of("PairDeclaration");
        assert!(fields.len() >= 17, "found only {fields:?}");
        assert!(fields.contains(&"expected_nonzero".to_string()));
        assert!(fields.contains(&"early_hand_cap_deviation".to_string()));
        assert_eq!(fields_of("SeatPresidents"), vec!["subject", "opponent"]);
        assert_eq!(fields_of("SidedPresidents"), vec!["greater", "lesser"]);
        for (owner, _) in NESTED_DECLARATION_FIELDS {
            assert!(!fields_of(owner).is_empty(), "{owner} scanned to nothing");
        }
        assert!(body_of("canonical_declaration").contains("PAIR|"));
        assert!(body_of("render_pair_report_with").contains("package digest"));
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

