//! The typed screen model: what a composed frame SHOWS, as pure data
//! (Phase-3.5a spec §2, ruling §0.11 / requirement R9).
//!
//! **The R9 contract, in one paragraph.** This module describes a frame; it
//! never draws one. It holds no ANSI, no glyphs, no colors, no column widths
//! and no prose — only engine values, re-shaped into the groupings a screen is
//! composed from. Its dependency set is deliberately `yokohama_core` + `std`
//! and nothing else: not [`crate::style`] (the ANSI authority), not
//! [`crate::render`] or [`crate::screen`] (the emitters that will consume
//! these models from T2 on), and not [`crate::describe`] (the WORDING layer,
//! which reaches `style` transitively). That is what makes this the seam a
//! later API or GUI layer can consume without a terminal — and it is the
//! import list a reviewer should check first. The corollary a caller must
//! honor: display names live where the words live. A [`TileModel`] carries
//! `area_type`, not `"Copper Mine"`; a [`DecisionModel`] carries `TurnPhase`
//! values, not `"Placement"`; wording each of those stays
//! `describe::area_type_name` / `describe::phase_name`'s job at render time.
//!
//! The builders are pure functions over `&GameState` + `&Rules` plus the
//! smallest presentation context each genuinely needs ([`SeatFacts`], the
//! viewing seat, the engine's legal list). They never call the engine's move
//! generator themselves: a caller passes the list it already has, so the model
//! can neither re-derive legality nor cost a second enumeration
//! (engine-as-sole-authority).
//!
//! **T0 scope, and what has since filled it.** Two fields were shaped at T0 and
//! filled at T4 — [`TileModel::reachable`] (the Movement `◈` feed) and
//! [`DecisionModel::not_offered`] (the dimmed near-miss rows) — and
//! [`IndexModel`], left a deliberate stub for the task that would know what the
//! input grammar needed, is designed at T5 alongside `card_prompt` and widened
//! at T6 to all four registries [`IndexKind`] names — orders, technologies,
//! achievements and building sites — over one [`CardFaceModel`] sum.
//!
//! **[`NUMBER_BASE`] lives here** — the one constant every numbered surface in
//! the crate reads, on the reasoning at its declaration.

use yokohama_core::*;

// ---------------------------------------------------------------------------
// The numbering base
// ---------------------------------------------------------------------------

/// **The number the FIRST item of any on-screen numbered list carries.**
///
/// The driver counted from ZERO everywhere until T5 — main-zone options, the
/// view menu, the route sub-prompt, the wizard's keep-a-card menu — while every
/// approved mock draws `[1]`. Spec §0's rule is that the mocks win, and the
/// owner's own reading is the plain one: people count from one. So this is 1,
/// and the ruling is recorded here rather than in the four places that print a
/// number, because a base that lives at its call sites is a base that drifts
/// per-surface — which is worse than either convention, since a player who has
/// learned that the first option is `[1]` will type `1` into the next list too.
///
/// **What this governs and what it does not.** It governs SELECTION numbers:
/// numbers the screen prints for the sole purpose of being typed back. It does
/// NOT govern
/// - **engine identities** — seat numbers (`Seat 0`), area ids (`A00`), order
///   and tech ids (`O14`, `T07`), building-site indexes. Those are printed on
///   the physical components or recorded in saves; renumbering them for
///   display would break the one thing an id is for.
/// - **quantities** — the guided retreat's `recover how many from … [0-4]`
///   answers with a COUNT, and zero is a real answer there ("recover none").
///   A count is not an index and does not shift.
///
/// Every numbered surface reads this constant. It lives in the MODEL rather
/// than in `style` (which owns ANSI, and a numbering base is not a color) or in
/// `repl` (which no renderer may depend on): the model is what assigns a
/// display number — see [`IndexCard::selection`] and [`OptionModel::number`] —
/// and it is the one module every surface, renderer and parser alike, is
/// already allowed to see.
pub const NUMBER_BASE: usize = 1;

/// The number shown for the 0-based `index` of a list.
pub const fn menu_number(index: usize) -> usize {
    index + NUMBER_BASE
}

/// The 0-based index a typed selection `number` refers to, or `None` when it is
/// outside a list of `len` entries (including the below-base numbers a 1-based
/// list makes possible — `0` is not an option any more, and saying so beats
/// wrapping it round to the last one).
pub fn menu_index(number: usize, len: usize) -> Option<usize> {
    let index = number.checked_sub(NUMBER_BASE)?;
    (index < len).then_some(index)
}

/// The inclusive range of numbers a list of `len` entries accepts, as the range
/// message prints it (`1-10`). Empty lists have no range and get `None`.
pub fn menu_range(len: usize) -> Option<(usize, usize)> {
    (len > 0).then(|| (NUMBER_BASE, menu_number(len - 1)))
}

// ---------------------------------------------------------------------------
// Presentation context
// ---------------------------------------------------------------------------

/// The per-seat presentation policy the panel builders need: whose hand
/// contents may be shown, and which seats a bot plays.
///
/// Deliberately NOT `render::SeatDisplay`, which holds the same two facts:
/// this module must not depend on the renderer it exists to feed (see the R9
/// contract above), so the call site adapts. `hand_hidden` is the EFFECTIVE
/// mask, already folded — the REPL's rule that a bot's hand stays masked
/// whatever the privacy toggle says is a policy decision, and policy belongs
/// at the call site, not in the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeatFacts {
    pub is_bot: bool,
    pub hand_hidden: bool,
}

impl SeatFacts {
    /// Every seat open and human — the pre-privacy, pre-bot default.
    pub fn all_open(seats: usize) -> Vec<SeatFacts> {
        vec![SeatFacts::default(); seats]
    }

    /// The policy for `seat`, or the safe direction for a seat the supplied
    /// table does not cover: masked (never leak a hand a caller forgot to
    /// describe) but not badged (never claim a bot nobody configured). Same
    /// split `render::SeatDisplay` takes for the same reason.
    fn for_seat(table: &[SeatFacts], seat: PlayerId) -> SeatFacts {
        table
            .get(usize::from(seat.0))
            .copied()
            .unwrap_or(SeatFacts { is_bot: false, hand_hidden: true })
    }
}

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------

/// One Shophouse space of a building site: who built there, and the reward
/// printed under them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiteSlotModel {
    pub built_by: Option<PlayerId>,
    pub reward: SpaceReward,
}

/// The building site card on a tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteModel {
    /// The card's printed index into `Rules::building_sites`, recovered by
    /// matching printed content (the runtime `BuildingSite` on the board is a
    /// CLONE of the registry card and carries no index of its own, and
    /// `board_gen` shuffles before dealing). `None` if no registry card
    /// matches, or if more than one does — a printed index nobody can name
    /// unambiguously is not one this model will assert.
    pub index: Option<u8>,
    /// The four spaces in printed TL, TR, ML, MR order.
    pub slots: [SiteSlotModel; SPACES_PER_SITE],
    pub trading_house_vp: u8,
    pub trading_house_owner: Option<PlayerId>,
}

/// One board tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileModel {
    pub area: AreaId,
    /// The tile's kind. The DISPLAY NAME is `describe::area_type_name` at
    /// render time — see this module's R9 contract on why no prose lands here.
    pub area_type: AreaType,
    /// `(row, col)` exactly as `AreaConfig` states it. The renderer groups by
    /// this and assumes no board shape, so a fixture geometry still draws.
    pub grid_position: (u8, u8),
    /// The face-up 5-Power token, or `None` once claimed (and on the Canal,
    /// which never carries one).
    pub token: Option<FivePowerEffect>,
    /// `None` on the Canal, the one tile with no building site.
    pub site: Option<SiteModel>,
    /// Every President standing here, in seat order. A LIST, not an `Option`:
    /// with Postal System a player may stop where an opponent's President
    /// already stands, so two pawns can share a tile and a model that could
    /// hold only one would silently drop the other.
    pub presidents: Vec<PlayerId>,
    /// The Station token stands here (every player reads +1 Power in this
    /// area, so it is tile state, not owner state).
    pub has_station: bool,
    /// One entry per seat in `PlayerId` order, ZEROS INCLUDED — the frame
    /// shows "Red 0 · Blue 0 assistants" at the opening, so suppressing empty
    /// seats here would push that decision into the renderer for no gain.
    pub assistants: Vec<(PlayerId, u8)>,
    /// This tile appears as a destination in the ENGINE's current legal list
    /// (spec §0.7's `◈` mark). **T4 feeds this**; T0 leaves every tile false,
    /// because the CLI computing its own reachability is exactly what
    /// engine-as-sole-authority forbids.
    pub reachable: bool,
    /// The areas this one connects to only by CROSSING the Canal, ascending —
    /// the tolled, transit-only edges (1 Yen to the bank).
    ///
    /// Read straight off `BoardLayout::canal_crossings`, never re-derived from
    /// geometry, on exactly the reasoning the retired `render::sites_view` gave:
    /// the crossing set is what `derive_canal_crossings` single-sources and what
    /// movement actually charges for, and a second derivation here could
    /// disagree with it. Empty on every tile of a 2-player board, which has no
    /// Canal at all.
    pub canal_crossings: Vec<AreaId>,
}

/// The board region of a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardModel {
    pub player_count: u8,
    /// Every area, in `AreaId` order.
    pub tiles: Vec<TileModel>,
}

// ---------------------------------------------------------------------------
// Player panel
// ---------------------------------------------------------------------------

/// A player's countable resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoodsModel {
    pub copper: u32,
    pub silk: u32,
    pub tea: u32,
    pub seafood: u32,
    pub imports: u32,
}

/// ALL SIX assistant zones, so the printed numbers reconcile to the player's
/// 20 pawns: showing hand/board/warehouse alone invites the table-side
/// conservation check and then fails it whenever a pawn is locked on a goal or
/// a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssistantZones {
    pub in_hand: u8,
    /// Summed across every area.
    pub on_board: u8,
    pub in_warehouse: u8,
    pub on_achievements: u8,
    pub on_church: u8,
    pub on_customs: u8,
}

impl AssistantZones {
    /// Pawns locked on goals and tracks — placed, but not on the board.
    pub fn locked(&self) -> u8 {
        self.on_achievements + self.on_church + self.on_customs
    }

    pub fn total(&self) -> u8 {
        self.in_hand + self.on_board + self.in_warehouse + self.locked()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShophouseZones {
    pub in_hand: u8,
    pub in_warehouse: u8,
    pub built: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradingHouseZones {
    pub in_hand: u8,
    pub in_warehouse: u8,
    pub built: u8,
    /// The printed Employment-Agency 4/5/6/7 progression's NEXT price, from
    /// the engine's own single source (`TradingHouseState::next_cost`).
    pub next_take_cost: u8,
}

/// One owned Technology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechModel {
    pub id: TechCardId,
    /// The printed card from `Rules::techs` (name, country, industry cost,
    /// effect). `None` only for an id the registry does not know — a corrupt
    /// record, or a fixture built on `Rules::empty_for_tests`. The model does
    /// not invent a name for that case; the renderer shows the id.
    pub printed: Option<TechCard>,
}

/// One seat's dashboard row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerModel {
    pub seat: PlayerId,
    pub color: PlayerColor,
    pub money: u32,
    pub victory_points: u32,
    pub goods: GoodsModel,
    pub assistants: AssistantZones,
    pub shophouses: ShophouseZones,
    pub trading_houses: TradingHouseZones,
    pub technologies: Vec<TechModel>,
    /// Face-up mercenaries — the ones end-game scoring pays 1 VP each for.
    pub mercenaries_unused: u8,
    /// Face-down (spent) mercenaries.
    pub mercenaries_used: u8,
    pub five_power_tokens: u8,
    pub president: PresidentLocation,
    /// The COUNT only. Card identities live in [`HandPanel`], which masks.
    pub orders_in_hand: usize,
    pub completed_orders: usize,
    pub is_current: bool,
    pub is_bot: bool,
    pub hand_hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayersPanel {
    /// Every seat in `PlayerId` order.
    pub players: Vec<PlayerModel>,
}

// ---------------------------------------------------------------------------
// Tracks panel
// ---------------------------------------------------------------------------

/// One Church faith space (4-11) with its printed rewards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChurchSpaceModel {
    pub faith: u8,
    pub vp: u8,
    pub movement_rider: u8,
    /// The occupant, which may legitimately be [`PlayerId::DUMMY`] — the 2P
    /// neutral marker holds faith 7 and competes in the majority bonus
    /// (finding D1). Consumers separate it with `PlayerId::is_dummy`.
    pub occupant: Option<PlayerId>,
}

/// One Customs space (1-7) with its printed cost and rewards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustomsSpaceModel {
    pub space: u8,
    /// Printed imported-goods cost.
    pub import_cost: u8,
    pub vp: u8,
    pub warehouse_rider: u8,
    pub yen_rider: u8,
    /// May be [`PlayerId::DUMMY`] — see [`ChurchSpaceModel::occupant`]; the 2P
    /// marker holds space 4.
    pub occupant: Option<PlayerId>,
}

/// The two management boards plus the board-level shared state that reads
/// beside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracksPanel {
    /// Faith 4-11, ascending.
    pub church: Vec<ChurchSpaceModel>,
    /// REAL pawns on the Church track — the end-game trigger's currency, which
    /// excludes the 2P dummy.
    pub church_filled: u8,
    /// Spaces 1-7, ascending.
    pub customs: Vec<CustomsSpaceModel>,
    /// Real pawns on the Customs track; see [`Self::church_filled`].
    pub customs_filled: u8,
    /// Real pawns that trigger the end game at this player count.
    pub trigger_at: u8,
    /// The Station token's area (+1 Power there for EVERY player), or `None`
    /// while unplaced.
    pub station: Option<AreaId>,
    pub mercenary_supply: MercenarySupply,
}

// ---------------------------------------------------------------------------
// Achievements panel
// ---------------------------------------------------------------------------

/// One Achievement card in play. Typed throughout: the condition's WORDING is
/// `describe::achievement_desc`'s job, not this model's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementModel {
    pub achievement_type: AchievementType,
    pub category: AchievementCategory,
    pub first_place_vp: u8,
    pub subsequent_vp: u8,
    /// Claimants in claim order — MORE than one player may claim the same
    /// card, the first at [`Self::first_place_vp`] and each later one at
    /// [`Self::subsequent_vp`].
    pub claimed_by: Vec<PlayerId>,
    /// `(seat, current, needed)` per seat, straight from the engine's
    /// `achievement_progress` — the same arithmetic the claim is gated on, so
    /// these numbers can never disagree with it. They are not the WHOLE gate:
    /// a claim also needs an assistant in hand and no prior claim by that
    /// seat, so `current >= needed` here is "ready by the numbers", not
    /// "claimable".
    pub progress: Vec<(PlayerId, u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementsPanel {
    pub cards: Vec<AchievementModel>,
}

// ---------------------------------------------------------------------------
// Hand panel
// ---------------------------------------------------------------------------

/// One Order card in a hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandCardModel {
    pub id: OrderCardId,
    /// The printed card from `Rules::orders` (country, requirements,
    /// rewards). `None` only for an id the registry does not know — see
    /// [`TechModel::printed`].
    pub printed: Option<OrderCard>,
    /// This is the seat's kept starting-order deal.
    pub is_starting_order: bool,
}

/// One seat's own hand, as the dashboard shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandPanel {
    pub seat: PlayerId,
    pub hidden: bool,
    /// EMPTY whenever `hidden`. The mask is a property of the MODEL, not of
    /// the renderer: a consumer that never reads `hidden` still cannot leak
    /// the cards, which is what makes this seam safe for a later API layer.
    pub cards: Vec<HandCardModel>,
    /// The true card count, masked or not.
    pub card_count: usize,
}

// ---------------------------------------------------------------------------
// Panels
// ---------------------------------------------------------------------------

/// One dashboard panel. The frame carries these in display order, so the
/// per-count arrangement (2P beside the board, 3-4P below it) is a rendering
/// decision over one ordered list rather than four hard-wired call sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Panel {
    Players(PlayersPanel),
    Tracks(TracksPanel),
    Achievements(AchievementsPanel),
    Hand(HandPanel),
}

// ---------------------------------------------------------------------------
// Decision zone
// ---------------------------------------------------------------------------

/// Every `TurnPhase` in turn order — the chain the decision zone lights the
/// current phase within. Exhaustive by construction: `phase_chain_index`
/// matches on `TurnPhase` with no `_` arm, so a new variant is a compile error
/// here rather than a phase that silently never lights.
pub const PHASE_CHAIN: [TurnPhase; 9] = [
    TurnPhase::AdditionalActionFirst,
    TurnPhase::Placement,
    TurnPhase::Movement,
    TurnPhase::AreaAction,
    TurnPhase::FivePowerBonus,
    TurnPhase::Building,
    TurnPhase::Recovery,
    TurnPhase::AdditionalActionSecond,
    TurnPhase::TurnEnd,
];

/// What the seat is being asked for. Typed, never worded: T4 owns the
/// "YOU ARE DECIDING · …" copy for each of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deciding {
    /// The ordinary case: the phase itself is the question.
    Phase(TurnPhase),
    /// A queued reward/recovery choice heads the queue, which outranks the
    /// phase exactly as the prompt banner does today.
    Choice { head: PendingChoice, queued: usize },
    /// The game is over; nothing is being decided.
    GameOver,
}

/// What an option costs, when the engine can price it. Kept to what the engine
/// itself computes: this model never re-derives a fee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeModel {
    pub total: u32,
    /// `(recipient, yen)` in seat order — the President owners paid.
    pub to_players: Vec<(PlayerId, u32)>,
    /// Canal crossings, paid to the BANK rather than to a player.
    pub canal_tolls: u32,
}

/// One numbered option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionModel {
    /// The number the prompt SHOWS for this option — global across pages, and
    /// 1-based since T5 ([`NUMBER_BASE`]). The option's position in the
    /// caller's legal list is `number - NUMBER_BASE`.
    pub number: usize,
    pub mv: Move,
    /// `Some` for the four moves that pay another PLAYER as a consequence of
    /// being chosen — the two President moves (`MovePresident`,
    /// `PlacePresident`) and the two placements (`PlaceThreeAreas`,
    /// `PlaceTwoSameArea`) — and `None` elsewhere, deliberately rather than a
    /// zeroed model, which would read as "priced, and free". A fee of zero on
    /// a move that CAN carry one is still `None` for the same reason; see
    /// [`option_fee`].
    ///
    /// **T0-review MF1 is discharged here** — the placement half used to be a
    /// stated gap. Its arithmetic is [`placement_fee`], which
    /// `describe::placement_fee_note` now computes its preview FROM, so the
    /// engine pin `placement_fee_preview_matches_the_engine_charge` guards the
    /// model and the words at once instead of only the words.
    pub fee: Option<FeeModel>,
}

/// Why a near-miss did not make the offered list.
///
/// **Every variant states a fact read straight off the state, never a verdict
/// re-derived from the rules** (spec §0.7: the CLI computes no legality). The
/// shape of every row is the same: the area is ADJACENT to the President and
/// ABSENT from the engine's destination list, and the variant names the one
/// observable fact that explains the absence. An area whose absence has no such
/// fact behind it gets NO row at all — [`build_not_offered`] is silent rather
/// than inventive, which is the whole reason this enum is small.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotOfferedReason {
    /// The President began this turn here, and the rulebook forbids returning
    /// to it in the same turn. Read off `GameState::president_start_area` —
    /// the engine's own field, and the one
    /// `apply_move::validate_movement_destination` refuses on.
    StartedThereThisTurn,
    /// The Canal is a crossing, never a stop (rulebook p.14, single-sourced in
    /// the engine's two shared movement predicates). Its own variant rather
    /// than falling through to [`Self::NoAssistantThere`], which would be
    /// actively misleading: no assistant can ever stand on a Canal, so
    /// "you have no assistant there" reads as a shortfall the player could fix.
    CanalIsNeverAStop,
    /// The viewing player has no assistant in the area, and an ordinary walk
    /// may only STOP where the mover has one. Read off the player's own
    /// `assistants.in_area` count, which is zero.
    ///
    /// Deliberately the LAST test of the three: it is the weakest of them,
    /// because an area can be absent for a reason this row does not know about
    /// (an unaffordable toll, an opponent's President) — so it is only ever
    /// claimed once the two facts above have been ruled out, and only when the
    /// count really is zero.
    NoAssistantThere,
}

/// A dimmed near-miss row under the offered options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotOffered {
    pub area: AreaId,
    pub reason: NotOfferedReason,
}

/// The decision zone: where the turn is, what is being asked, and what may be
/// answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionModel {
    pub turn_number: u32,
    pub seat: PlayerId,
    /// [`PHASE_CHAIN`], carried rather than referenced so a consumer holds a
    /// self-contained frame.
    pub phase_chain: Vec<TurnPhase>,
    /// Index into [`Self::phase_chain`] of the phase to light. Not an
    /// `Option`: every `TurnPhase` sits somewhere in the chain, including
    /// `TurnEnd` at a finished game, so there is no state in which nothing is
    /// lit.
    pub current_phase: usize,
    pub deciding: Deciding,
    /// One per move in the legal list the caller supplied, in that order.
    pub options: Vec<OptionModel>,
    /// The dimmed near-miss rows (see [`build_not_offered`]): empty at every
    /// decision but a Movement one, and empty there too whenever no adjacent
    /// area's absence has an observable fact behind it.
    pub not_offered: Vec<NotOffered>,
    /// How many further near misses [`NOT_OFFERED_ROWS`] cut from
    /// [`Self::not_offered`] — zero whenever the cap did not fire.
    ///
    /// Carried rather than left implicit because the renderer cannot recover
    /// it: a capped list and a complete one are the same three rows on screen,
    /// and a truncation nobody is told about reads as "those were all of them".
    pub not_offered_suppressed: usize,
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

/// The title strip's SESSION facts (the mocks' line 1: mode, seed,
/// autosave). Carried on the model precisely so T2's renderer never reaches
/// into `Session` — the R9 weld T0-review SF2 named. `TitleMode` mirrors
/// `session::TrackingMode` as a local two-variant enum so this module's
/// dependency list stays `yokohama_core` alone; the CALLER maps between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleMode {
    Sim,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TitleModel {
    pub mode: TitleMode,
    /// The game's identity seed; the renderer abbreviates it for display.
    pub seed: u64,
    /// Whether autosave has a live target (the strip prints "autosaved").
    pub autosaved: bool,
}

/// A whole composed frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameModel {
    pub title: TitleModel,
    pub board: BoardModel,
    /// The dashboard panels in display order.
    pub dashboard: Vec<Panel>,
    pub decision: DecisionModel,
}

// ---------------------------------------------------------------------------
// The card index (spec §0.6, R8)
// ---------------------------------------------------------------------------

/// Which registry an index is showing.
///
/// T5 shipped one variant and named the widening point; T6 took it. All four
/// registries R8 lists are here now, and the reason they share ONE enum rather
/// than four order-shaped surfaces is the same reason [`CardPrompt`] takes a
/// kind: the player learns one screen and one grammar, and which registry is
/// underneath changes what the cards say, never how the screen works.
///
/// [`CardPrompt`]: crate::card_prompt::CardPrompt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexKind {
    Orders,
    /// Technology TITLES, not physical cards. Ten of the twenty titles
    /// circulate as two identical copies, so a 30-card screen would ask a
    /// player to tell apart two cards the printer did not — see
    /// [`CardFaceModel::Tech`].
    Techs,
    Achievements,
    /// Building-site cards by the index the wizard names them with (`S<n>`).
    Sites,
}

impl IndexKind {
    /// Every kind, for sweeps. Spelled out with no wildcard anywhere that
    /// matches on this enum, so a fifth registry is a compile error rather
    /// than a screen that silently renders as an order index.
    pub const ALL: [IndexKind; 4] =
        [IndexKind::Orders, IndexKind::Techs, IndexKind::Achievements, IndexKind::Sites];
}

/// The identity a selection resolves to: which card, in which registry.
///
/// The four id types are genuinely different — an order card prints a corner
/// number, a technology prints only a name, an achievement prints only its
/// condition, a site card prints nothing at all — so this is a sum rather than
/// a `u8` the caller has to remember the meaning of. A prompt that asked for a
/// technology and got back a bare number could not tell a tech id from an order
/// id, and that is exactly the F4 confusion the one grammar exists to end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardRef {
    Order(OrderCardId),
    Tech(TechCardId),
    Achievement(AchievementType),
    /// A building-site card by its index into `Rules::building_sites` — the
    /// number `wizard::ask_sites` already accepts as `S<n>`, 0-based, and
    /// deliberately NOT renumbered by [`NUMBER_BASE`] (which excludes
    /// identities; this is one).
    Site(u8),
}

impl CardRef {
    /// The registry this identity belongs to. Lets a caller check that what
    /// came back is what its prompt asked for without matching twice.
    pub fn kind(&self) -> IndexKind {
        match self {
            CardRef::Order(_) => IndexKind::Orders,
            CardRef::Tech(_) => IndexKind::Techs,
            CardRef::Achievement(_) => IndexKind::Achievements,
            CardRef::Site(_) => IndexKind::Sites,
        }
    }
}

/// Where a card the index dims is ALREADY recorded (spec §0.6 / finding F6:
/// "every duplicate refusal names WHERE the card already is").
///
/// **Typed, never worded** — `describe::card_location` says these in English at
/// render time, and both the index's dim note and the refusal message a card
/// prompt prints go through that one function, so a card can never be
/// described two ways on two screens.
///
/// **What is NOT in this enum is the point of it.** There is no `Hand { seat }`
/// for an opponent: a location is only ever supplied by a caller that already
/// knows it from PUBLIC state or from its own recording context, so a live
/// game's index cannot annotate a card sitting in an opponent's hidden hand.
/// The model does not derive locations from `GameState` at all — see
/// [`build_order_index`].
///
/// **The three T6 variants keep that fence exactly where T5 put it.** A Lab
/// slot is face-up on the table; an owned Technology sits face-up in front of
/// its owner (`PlayerState::technologies` is public state, unlike
/// `order_cards_in_hand`); a site card is dealt face-up onto its area at setup.
/// Every one of them is something the person at the table can already see, which
/// is the test a new variant has to pass to belong here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardLocation {
    /// A seat's dealt starting pair (the wizard's own recording context).
    Deal { seat: PlayerId },
    /// A Port board's display slot, 0-based as the board stores it.
    PortSlot { board_b: bool, slot: usize },
    /// The VIEWING seat's own hand — supplied only when that hand is already
    /// on screen unmasked, so the index never says more than the dashboard.
    OwnHand,
    /// Already named in the reveal group being collected right now.
    ThisReveal,
    /// A Laboratory board's display slot, 0-based as the board stores it.
    LabSlot { board_b: bool, slot: usize },
    /// A face-up holding in front of a seat: a Technology that player bought.
    /// Never an order card — a hand is hidden and this variant must not be
    /// used to say otherwise.
    OwnedBy { seat: PlayerId },
    /// A building-site card already dealt onto an area of the board.
    OnArea { area: AreaId },
    /// The Achievement card in play for a category. Three cards are on the
    /// table for a whole game, one per category (KB §Achievement Cards), so the
    /// CATEGORY is the location — there is no slot, shelf or board they sit on,
    /// and numbering them 1/2/3 would invent a position nothing prints.
    ///
    /// Face-up from setup like every other variant here, so the fence holds.
    AchievementSlot { category: AchievementCategory },
}

/// What a card prompt is recording, shown as the index's context header ("which
/// card is physically in the slot?"). Typed for the same reason
/// [`CardLocation`] is.
///
/// **ONE positional convention, and it is the model's rather than the screen's**
/// (T5 review S3). Every POSITION here — `slot`, `nth`, `which` — is **0-based**,
/// exactly like [`CardLocation::PortSlot`] and every other index this crate
/// stores; every `of` is a **total**, so a position is always `< of`.
/// `describe::recording_target` is the ONE place that adds the `+ 1` a person
/// reads, and it adds it to all three uniformly.
///
/// That rule is written down because the alternative had already shipped: `slot`
/// arrived 0-based while `nth` and `which` arrived pre-incremented by their
/// callers, so the same struct held two conventions and the converter had to
/// know which field was which. The failure that invites is silent and
/// off-screen-looking — a caller that passes `of` where a position goes, or
/// increments twice, prints `slot 7 of 6`, which is a sentence about a board
/// that does not exist. `describe::tests::recording_targets_render_positions_in_range`
/// sweeps every variant at every position so T6/T7's new prompts cannot ship it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingTarget {
    /// A live Port refill: reveal `nth` of `of`, into `slot` of that board.
    PortRefill { board_b: bool, slot: usize, nth: usize, of: usize },
    /// A live Laboratory refill (one slot, always) — a TECH prompt.
    ///
    /// **Named for its registry, and renamed at T6 from `LabRefill`.** T5 shaped
    /// the variant for this event and deliberately left it unrouted: `?` at a
    /// tech reveal opened the ORDER index, a screen of thirty-six cards that
    /// cannot answer "which technology refilled the Lab", so it fell through to
    /// a view instead. Routing it needed the tech index, which is T6's. The name
    /// moved with the routing because the old one described the BOARD and the
    /// question is about the CARD, and a target whose name does not say which
    /// registry it belongs to is exactly how the mis-route happened once.
    TechRefill { board_b: bool },
    /// The wizard's setup entry for one Port display slot, `slot` of `of`.
    PortSetup { board_b: bool, slot: usize, of: usize },
    /// The wizard's setup entry for one seat's dealt card, `which` of `of`.
    DealSetup { seat: PlayerId, which: usize, of: usize },
    /// The wizard's setup entry for one Laboratory display slot — a TECH
    /// prompt, and the setup twin of [`Self::TechRefill`] (which is the
    /// mid-game event).
    LabSetup { board_b: bool, slot: usize, of: usize },
    /// The wizard's setup entry for the building-site card dealt onto one area.
    ///
    /// **Addressed by AREA, not by a position in the entry run**, and that is
    /// this variant's one design decision: an `AreaId` is an identity the whole
    /// driver already names (`A03`), so it is exempt from [`NUMBER_BASE`] and
    /// from the 0-based-position convention above — there is no `of` here to
    /// convert against, and inventing "cell 4 of 10" would renumber the one
    /// coordinate `wizard::cell_coords` deliberately keeps 0-based.
    SiteSetup { area: AreaId },
    /// The wizard's setup entry for the Achievement card in play for one
    /// category. Category rather than position, on [`CardLocation::
    /// AchievementSlot`]'s reasoning.
    AchievementSetup { category: AchievementCategory },
}

impl RecordingTarget {
    /// Which registry this target's prompt is collecting.
    ///
    /// **On the model rather than at a call site** because two now need it: the
    /// REPL filters a live capture's target to the index actually on screen (T6
    /// review M1), and the wizard picks the index a `?` opens from the target
    /// its question carries. Two copies of this match would be two places for a
    /// new variant to be mis-filed, and the failure — a screen numbering cards
    /// that cannot answer the question — is the exact defect M1 fixed.
    pub fn kind(&self) -> IndexKind {
        match self {
            RecordingTarget::TechRefill { .. } | RecordingTarget::LabSetup { .. } => {
                IndexKind::Techs
            }
            RecordingTarget::PortRefill { .. }
            | RecordingTarget::PortSetup { .. }
            | RecordingTarget::DealSetup { .. } => IndexKind::Orders,
            RecordingTarget::SiteSetup { .. } => IndexKind::Sites,
            RecordingTarget::AchievementSetup { .. } => IndexKind::Achievements,
        }
    }
}

/// Which good a `f <term>` filter narrowed the index to.
///
/// Its own enum rather than [`crate::style::GoodKind`]: that one is the
/// PALETTE's roster (it carries `Yen`, `Imports` and `Any`, none of which an
/// order card requires) and it lives in the module this one may not depend on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardGood {
    Seafood,
    Tea,
    Silk,
    Copper,
}

/// An active index filter, already parsed. `card_prompt::parse_filter` turns
/// the typed word into one of these, so the model holds no free text and the
/// renderer words the filter line from a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFilter {
    /// Cards requiring this good at all.
    Good(CardGood),
    /// Cards paying at least this many victory points.
    MinVp(u8),
    /// One country's cards.
    Country(Country),
    /// One achievement category.
    Category(AchievementCategory),
}

impl IndexFilter {
    /// Which registries this filter can be spoken about at all.
    ///
    /// **Stated rather than left to fall out of [`Self::admits`]**, because the
    /// two failure shapes read identically on screen and only one of them is
    /// honest: a filter that does not APPLY to a registry (`f 8vp` on the site
    /// index) and a filter that applies but matches nothing (`f 30vp` on the
    /// order index) would both draw an empty grid. The first is a question this
    /// screen cannot answer and belongs in a refusal that says so; the second is
    /// a real answer. `card_prompt::parse_filter` reads this to tell them apart.
    pub fn applies_to(&self, kind: IndexKind) -> bool {
        match (self, kind) {
            (IndexFilter::Good(_) | IndexFilter::MinVp(_), IndexKind::Orders) => true,
            (IndexFilter::Good(_) | IndexFilter::MinVp(_), _) => false,
            (IndexFilter::Country(_), IndexKind::Orders | IndexKind::Techs) => true,
            (IndexFilter::Country(_), _) => false,
            (IndexFilter::Category(_), IndexKind::Achievements) => true,
            (IndexFilter::Category(_), _) => false,
        }
    }

    /// Whether `face` survives this filter. A filter that does not apply to the
    /// face's registry admits everything rather than nothing — the screen it
    /// could not narrow is left as it was, and the refusal at the prompt is what
    /// tells the player why (see [`Self::applies_to`]).
    fn admits(&self, face: &CardFaceModel) -> bool {
        match (self, face) {
            (IndexFilter::Good(g), CardFaceModel::Order { printed: Some(card), .. }) => {
                let r = &card.requirements;
                match g {
                    CardGood::Seafood => r.seafood > 0,
                    CardGood::Tea => r.tea > 0,
                    CardGood::Silk => r.silk > 0,
                    CardGood::Copper => r.copper > 0,
                }
            }
            (IndexFilter::MinVp(n), CardFaceModel::Order { printed: Some(card), .. }) => {
                card.rewards.victory_points >= *n
            }
            // An id the registry does not know has no printed content to test,
            // so a content filter cannot claim it matches.
            (
                IndexFilter::Good(_) | IndexFilter::MinVp(_),
                CardFaceModel::Order { printed: None, .. },
            ) => false,
            (IndexFilter::Country(c), CardFaceModel::Order { printed, .. }) => {
                printed.as_ref().is_some_and(|card| card.country == *c)
            }
            (IndexFilter::Country(c), CardFaceModel::Tech { printed, .. }) => {
                printed.country == *c
            }
            (IndexFilter::Category(cat), CardFaceModel::Achievement(a)) => a.category == *cat,
            _ => true,
        }
    }
}

/// What one card in an index IS — its identity and its printed content.
///
/// One variant per [`IndexKind`], and they carry genuinely different things
/// because the four physical cards are genuinely different objects. The renderer
/// matches on this with no `_` arm, so a registry added to `IndexKind` cannot
/// reach a screen without someone deciding what its card looks like.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardFaceModel {
    Order {
        id: OrderCardId,
        /// The printed card from `Rules::orders`; `None` for an id the registry
        /// does not know, exactly as [`HandCardModel::printed`] means it.
        printed: Option<OrderCard>,
    },
    /// **One card per TITLE, not per physical card**, which is the whole reason
    /// this face carries a list of ids.
    ///
    /// Ten of the twenty titles circulate as two identical copies (CLAUDE.md's
    /// "20 tech titles × copies = 30 physical cards"), so an index over the 30
    /// registry entries would draw `Telegram` twice, side by side, indexed `T03`
    /// and `T04` — and then ask the player which of two identical pieces of
    /// cardboard they are holding. That is R8's defect wearing a different hat.
    /// The screen shows the title once; `copies` is what makes a selection
    /// resolvable, and `placed` is what makes "one copy is already in Lab A"
    /// sayable without pretending the other one is too.
    Tech {
        /// Every registry id printing this title, ascending.
        copies: Vec<TechCardId>,
        /// The printed card (all copies of a title are identical apart from id,
        /// which `tech_titles_are_identical_across_copies` pins).
        printed: TechCard,
        /// Where copies of this title already are, in `copies` order. Shorter
        /// than `copies` while any copy is still unaccounted for.
        placed: Vec<(TechCardId, CardLocation)>,
    },
    /// An achievement card with its live claim state and per-seat progress —
    /// the same [`AchievementModel`] the dashboard panel summarizes, so the
    /// panel and the full surface can never disagree about a number.
    Achievement(AchievementModel),
    Site {
        /// Index into `Rules::building_sites` — the number `wizard::ask_sites`
        /// takes as `S<n>`. Nothing is printed on the physical card, which is
        /// precisely why this index exists.
        index: u8,
        site: BuildingSite,
    },
}

impl CardFaceModel {
    /// The registry this face belongs to.
    pub fn kind(&self) -> IndexKind {
        match self {
            CardFaceModel::Order { .. } => IndexKind::Orders,
            CardFaceModel::Tech { .. } => IndexKind::Techs,
            CardFaceModel::Achievement(_) => IndexKind::Achievements,
            CardFaceModel::Site { .. } => IndexKind::Sites,
        }
    }

    /// What a selection on this card resolves to, or `None` when nothing on it
    /// is still available to name — a technology whose every copy is already
    /// placed being the one case that arises.
    pub fn card_ref(&self) -> Option<CardRef> {
        match self {
            CardFaceModel::Order { id, .. } => Some(CardRef::Order(*id)),
            // The first copy nobody has accounted for, or — when every copy is
            // spoken for — the first copy anyway, so that typing this card's
            // number reaches the duplicate refusal that names where it is
            // rather than a range message about the screen.
            CardFaceModel::Tech { copies, placed, .. } => copies
                .iter()
                .find(|id| !placed.iter().any(|(placed_id, _)| placed_id == *id))
                .or_else(|| copies.first())
                .map(|id| CardRef::Tech(*id)),
            CardFaceModel::Achievement(a) => Some(CardRef::Achievement(a.achievement_type)),
            CardFaceModel::Site { index, .. } => Some(CardRef::Site(*index)),
        }
    }
}

/// One card in an index grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexCard {
    pub face: CardFaceModel,
    /// The number shown on this card and typed to select it — 1-based like
    /// every other numbered surface ([`NUMBER_BASE`], which lives in this
    /// module), and assigned over the cards actually DISPLAYED, so a filtered
    /// screen numbers 1..n rather than showing gaps from the unfiltered run.
    ///
    /// `None` when the index is open as a standing VIEW: nothing is being
    /// recorded, so there is nothing a number could select, and a number
    /// printed on a card that cannot be picked is exactly the defect F3 names
    /// from the other side. Also `None` on a card that is fully spoken for —
    /// see [`Self::location`].
    pub selection: Option<usize>,
    /// This card is entirely accounted for elsewhere: it dims and says where.
    ///
    /// For a two-copy technology that means BOTH copies are placed; while one
    /// is still free the card stays live and its face reports the placed copy
    /// (see [`CardFaceModel::Tech`]).
    pub location: Option<CardLocation>,
}

/// How an index groups its cards. Orders group by flag (spec §0.6),
/// achievements by category, techs by flag, sites not at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexGroupKey {
    Country(Country),
    Category(AchievementCategory),
    /// One ungrouped run: the registry has no dimension worth splitting on.
    All,
}

/// One group of an index grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexGroup {
    pub key: IndexGroupKey,
    pub cards: Vec<IndexCard>,
}

/// The full-screen card index (spec §0.6, requirement R8) — "the canonical
/// identification mechanism".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexModel {
    pub kind: IndexKind,
    /// Cards in the whole population this index is over, filtered or not — the
    /// "all 36" the title states. For techs that counts TITLES, not the thirty
    /// physical cards; for achievements it is whatever population the caller
    /// passed (three in play, or all twelve at setup).
    pub total: usize,
    /// Cards this screen actually shows (equal to [`Self::total`] unfiltered).
    pub shown: usize,
    /// What is being recorded, or `None` when the index is open as a view.
    /// This is also what decides whether cards carry selection numbers.
    pub target: Option<RecordingTarget>,
    pub filter: Option<IndexFilter>,
    /// Non-empty groups in display order.
    pub groups: Vec<IndexGroup>,
}

impl IndexModel {
    /// The card a typed selection number picks, or `None` when no card on
    /// screen carries it.
    ///
    /// Resolved through the MODEL rather than by the caller re-deriving the
    /// display order: the number on screen and the number this answers are then
    /// the same number by construction, which is the whole promise F3 makes.
    pub fn select(&self, number: usize) -> Option<CardRef> {
        self.groups
            .iter()
            .flat_map(|g| g.cards.iter())
            .find(|c| c.selection == Some(number))
            .and_then(|c| c.face.card_ref())
    }

    /// Every displayed card in screen order — what a caller pages, counts, or
    /// (in tests) sweeps.
    pub fn cards(&self) -> impl Iterator<Item = &IndexCard> {
        self.groups.iter().flat_map(|g| g.cards.iter())
    }
}

/// One card as a builder hands it over: its printed content, plus where the
/// CALLER says the whole card already is (`None` while any of it is still
/// unaccounted for — see [`CardFaceModel::Tech`]).
type FacedCard = (CardFaceModel, Option<CardLocation>);

/// What a filter does to the numbers already on the cards.
///
/// **Two registries need two answers, and the difference is whether a SECOND
/// surface prints the same numbers** (T7 review M2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionNumbering {
    /// Numbers run `1..shown` over the cards this screen displays. The screen is
    /// the only thing printing them, so a filter compacts them and no number a
    /// player can currently see is missing.
    PerScreen,
    /// Numbers run `1..total` over the whole POPULATION, filtered or not: a card
    /// the filter drops takes its number with it and nothing renumbers.
    ///
    /// This is the achievement index, and it is not a preference. The wizard's
    /// category question prints its own four-card menu INLINE, from the same
    /// model, and that menu is still on screen after a `f b` — so under
    /// `PerScreen` the visible menu said `[1] Copper5` while the visible index
    /// said `[1] Money10`, and one bare number meant two cards at once. That is
    /// finding F3 verbatim, at the prompt F1 was found at. Stable numbers make
    /// the two agree unconditionally, and a number whose card the filter dropped
    /// is REFUSED with the range on screen rather than silently reused.
    PerRegistry,
}

/// Number the displayed cards and pack them into non-empty groups.
///
/// The one place selection numbers are assigned, shared by all four builders:
/// they run from [`NUMBER_BASE`] in screen order, across group boundaries, so no
/// builder can invent its own numbering rule. What a FILTER does to them is
/// [`SelectionNumbering`], which is a per-registry choice with a reason.
fn number_groups(
    faces: Vec<(IndexGroupKey, Vec<FacedCard>)>,
    target: Option<RecordingTarget>,
    filter: Option<IndexFilter>,
    numbering: SelectionNumbering,
) -> (Vec<IndexGroup>, usize) {
    let mut next = NUMBER_BASE;
    let mut groups = Vec::new();
    let mut shown = 0usize;
    for (key, faces) in faces {
        let mut cards = Vec::new();
        for (face, location) in faces {
            let admitted = filter.is_none_or(|f| f.admits(&face));
            if !admitted && numbering == SelectionNumbering::PerScreen {
                continue;
            }
            // A DIMMED card is numbered too, exactly as the approved mock draws
            // it: typing that number is how a player asks about the card they
            // are looking at, and the answer they get is F6's refusal naming
            // where it already is — which is more use than "not on this screen".
            let selection = target.is_some().then(|| {
                let n = next;
                next += 1;
                n
            });
            // Under `PerRegistry` the number is spent whether or not the card is
            // shown — that is what "nothing renumbers" costs, and it is the
            // whole point.
            if !admitted {
                continue;
            }
            cards.push(IndexCard { face, selection, location });
        }
        shown += cards.len();
        if !cards.is_empty() {
            groups.push(IndexGroup { key, cards });
        }
    }
    (groups, shown)
}

/// The order index: every registry card, grouped by flag, dimmed where the
/// caller says one is already recorded.
///
/// **`placed` is supplied, never derived, and that is a leak fence rather than
/// a convenience.** The obvious implementation reads locations out of
/// `GameState` — port slots, every seat's hand, the dealt starting orders — and
/// the last two of those are hidden information in a live game. Taking the list
/// from the caller means the index can only ever annotate what its caller
/// already knew: public board slots in play, the operator's own recording
/// context in the wizard. A model that cannot see the hidden set cannot leak
/// it.
///
/// `filter` narrows the grid; `target` decides whether cards are selectable
/// (see [`IndexCard::selection`]).
pub fn build_order_index(
    rules: &Rules,
    placed: &[(OrderCardId, CardLocation)],
    target: Option<RecordingTarget>,
    filter: Option<IndexFilter>,
) -> IndexModel {
    let mut all: Vec<&OrderCard> = rules.orders.iter().collect();
    all.sort_by_key(|c| c.id.0);
    let total = all.len();

    let faces = Country::ALL
        .into_iter()
        .map(|country| {
            let cards = all
                .iter()
                .filter(|c| c.country == country)
                .map(|card| {
                    let face = CardFaceModel::Order {
                        id: card.id,
                        printed: Some((*card).clone()),
                    };
                    let at = placed.iter().find(|(id, _)| *id == card.id).map(|(_, at)| *at);
                    (face, at)
                })
                .collect();
            (IndexGroupKey::Country(country), cards)
        })
        .collect();

    let (groups, shown) =
        number_groups(faces, target, filter, SelectionNumbering::PerScreen);
    IndexModel { kind: IndexKind::Orders, total, shown, target, filter, groups }
}

/// The technology index: every TITLE, grouped by flag, with the effect text a
/// purchase decision needs.
///
/// **NAME-first, because no technology card prints a number** (spec §0.6 / R8,
/// and F4 in the owner's own words: the tech prompt refused `12`, `1` and `2`
/// alike and wanted an id printed on nothing). `T<n>` stays on the card as the
/// small annotation every index carries, and the title is what the screen is
/// organized around.
///
/// **One card per title.** `Rules::techs` holds the thirty PHYSICAL cards, ten
/// titles twice over; this collapses them and keeps the copies on the face, so
/// "Telegram, one copy left, the other is in Lab A" is sayable — see
/// [`CardFaceModel::Tech`].
///
/// `placed` is supplied, never derived, on exactly the reasoning
/// [`build_order_index`] states. For techs the caller's honest sources are the
/// two face-up Lab boards and the seats' face-up `technologies`; a hand holds no
/// technology, so there is no hidden set to leak here at all.
pub fn build_tech_index(
    rules: &Rules,
    placed: &[(TechCardId, CardLocation)],
    target: Option<RecordingTarget>,
    filter: Option<IndexFilter>,
) -> IndexModel {
    // Titles in first-id order within each flag, which is the order the physical
    // deck is printed in and the order the lab boards display.
    let mut all: Vec<&TechCard> = rules.techs.iter().collect();
    all.sort_by_key(|c| c.id.0);

    let mut titles: Vec<(&TechCard, Vec<TechCardId>)> = Vec::new();
    for card in &all {
        match titles.iter_mut().find(|(first, _)| first.name == card.name) {
            Some((_, copies)) => copies.push(card.id),
            None => titles.push((card, vec![card.id])),
        }
    }
    let total = titles.len();

    let faces = Country::ALL
        .into_iter()
        .map(|country| {
            let cards = titles
                .iter()
                .filter(|(card, _)| card.country == country)
                .map(|(card, copies)| {
                    let copy_locations: Vec<(TechCardId, CardLocation)> = copies
                        .iter()
                        .filter_map(|id| {
                            placed.iter().find(|(placed_id, _)| placed_id == id).copied()
                        })
                        .collect();
                    // Dim only once EVERY copy is accounted for: a title with a
                    // copy still in the deck is a title the player can still be
                    // holding, and dimming it would refuse a true answer.
                    let at = (copy_locations.len() == copies.len())
                        .then(|| copy_locations.first().map(|(_, at)| *at))
                        .flatten();
                    let face = CardFaceModel::Tech {
                        copies: copies.clone(),
                        printed: (*card).clone(),
                        placed: copy_locations,
                    };
                    (face, at)
                })
                .collect();
            (IndexGroupKey::Country(country), cards)
        })
        .collect();

    let (groups, shown) =
        number_groups(faces, target, filter, SelectionNumbering::PerScreen);
    IndexModel { kind: IndexKind::Techs, total, shown, target, filter, groups }
}

/// The achievement index, grouped A / B / C.
///
/// **The population is the CALLER's**, and that is what makes one surface serve
/// two jobs: the `goals` command passes the three cards in play (with their live
/// claim state and per-seat progress, which is what supersedes the old
/// `achievements_view`), while a setup prompt passes all twelve types so the
/// operator can say which three are on the table. Deriving the population here
/// would have forced one of those two to be a second screen.
///
/// **The one index whose numbers survive a filter** — see
/// [`SelectionNumbering::PerRegistry`], and note that "the population" there
/// means whatever the caller passed, which is what keeps the two jobs on one
/// builder.
pub fn build_achievement_index(
    cards: &[AchievementModel],
    placed: &[(AchievementType, CardLocation)],
    target: Option<RecordingTarget>,
    filter: Option<IndexFilter>,
) -> IndexModel {
    let total = cards.len();
    let faces = [AchievementCategory::A, AchievementCategory::B, AchievementCategory::C]
        .into_iter()
        .map(|category| {
            let group = cards
                .iter()
                .filter(|c| c.category == category)
                .map(|card| {
                    let at = placed
                        .iter()
                        .find(|(t, _)| *t == card.achievement_type)
                        .map(|(_, at)| *at);
                    (CardFaceModel::Achievement(card.clone()), at)
                })
                .collect();
            (IndexGroupKey::Category(category), group)
        })
        .collect();

    let (groups, shown) =
        number_groups(faces, target, filter, SelectionNumbering::PerRegistry);
    IndexModel { kind: IndexKind::Achievements, total, shown, target, filter, groups }
}

/// The building-site card index: the whole registry in one ungrouped run,
/// numbered by the index `wizard::ask_sites` names (`S<n>`).
///
/// Ungrouped on purpose — the twenty-four cards differ only in their printed
/// rewards, so there is no dimension to split on that a player would recognize,
/// and inventing one (by trading-house VP, say) would organize the screen around
/// a fact nobody identifies a card by.
pub fn build_site_index(
    rules: &Rules,
    placed: &[(u8, CardLocation)],
    target: Option<RecordingTarget>,
    filter: Option<IndexFilter>,
) -> IndexModel {
    let total = rules.building_sites.len();
    let cards = rules
        .building_sites
        .iter()
        .enumerate()
        .map(|(i, site)| {
            let index = i as u8;
            let at = placed.iter().find(|(placed_index, _)| *placed_index == index).map(|(_, at)| *at);
            (CardFaceModel::Site { index, site: site.clone() }, at)
        })
        .collect();

    let (groups, shown) = number_groups(
        vec![(IndexGroupKey::All, cards)],
        target,
        filter,
        SelectionNumbering::PerScreen,
    );
    IndexModel { kind: IndexKind::Sites, total, shown, target, filter, groups }
}

/// The index for ONE registry, from a MIXED placement list — the entry point
/// both index-opening surfaces call.
///
/// The four builders below each take their own registry's placements, which is
/// what keeps their signatures honest; every caller, though, holds one list of
/// [`CardRef`]s and wants whichever registry is on screen. That split used to
/// live at the call site as a four-arm match plus three filters, and a second
/// caller (the wizard, T7) would have been a second copy of both — with the
/// achievement POPULATION, the one genuinely per-caller input, as the thing
/// most likely to drift. So it lives here once and the population is the
/// parameter: the three cards in play for a live `goals`, the whole twelve-card
/// registry for a setup prompt.
pub fn build_index(
    kind: IndexKind,
    rules: &Rules,
    placed: &[(CardRef, CardLocation)],
    achievements: &[AchievementModel],
    target: Option<RecordingTarget>,
    filter: Option<IndexFilter>,
) -> IndexModel {
    match kind {
        IndexKind::Orders => build_order_index(rules, &only_orders(placed), target, filter),
        IndexKind::Techs => build_tech_index(rules, &only_techs(placed), target, filter),
        IndexKind::Achievements => {
            build_achievement_index(achievements, &only_achievements(placed), target, filter)
        }
        IndexKind::Sites => build_site_index(rules, &only_sites(placed), target, filter),
    }
}

/// The order-card half of a mixed placement list, for the builder that takes
/// order ids. Splitting here rather than storing four lists keeps ONE leak
/// fence to audit at each caller (`Repl::index_placements`, the wizard's own
/// entries) instead of four.
fn only_orders(placed: &[(CardRef, CardLocation)]) -> Vec<(OrderCardId, CardLocation)> {
    placed
        .iter()
        .filter_map(|(card, at)| match card {
            CardRef::Order(id) => Some((*id, *at)),
            _ => None,
        })
        .collect()
}

fn only_techs(placed: &[(CardRef, CardLocation)]) -> Vec<(TechCardId, CardLocation)> {
    placed
        .iter()
        .filter_map(|(card, at)| match card {
            CardRef::Tech(id) => Some((*id, *at)),
            _ => None,
        })
        .collect()
}

fn only_sites(placed: &[(CardRef, CardLocation)]) -> Vec<(u8, CardLocation)> {
    placed
        .iter()
        .filter_map(|(card, at)| match card {
            CardRef::Site(index) => Some((*index, *at)),
            _ => None,
        })
        .collect()
}

fn only_achievements(placed: &[(CardRef, CardLocation)]) -> Vec<(AchievementType, CardLocation)> {
    placed
        .iter()
        .filter_map(|(card, at)| match card {
            CardRef::Achievement(ty) => Some((*ty, *at)),
            _ => None,
        })
        .collect()
}

/// ONE card, alone, in the shape the index draws it — the echo spec §0.6
/// requires an entry to print back before it commits.
///
/// **Built here rather than at the prompt** so the echo and the index cannot
/// draw the same card two ways: T5 shipped the echo for orders only, straight
/// off `card_render::order_card`, and the other three registries had no echo at
/// all because nothing assembled a face for them. This assembles all four from
/// an identity, and the renderer it feeds is the one the index feeds.
///
/// Deliberately unnumbered and unplaced: an echo is a statement about the card
/// just named, not a screen to select from (a `[1]` on it would be a number
/// nothing accepts — F3 from the other side), and where it is going is what the
/// prompt underneath is saying.
///
/// `None` for an identity no registry holds — a `T99` typed at a tech prompt.
/// The caller's own pool check is what refuses that; this just has nothing to
/// draw.
pub fn index_card_for(card: CardRef, rules: &Rules) -> Option<IndexCard> {
    let face = match card {
        // An order id the registry does not know still draws: `CardFace` prints
        // the unknown-card face for it, which is a truthful thing to echo.
        CardRef::Order(id) => CardFaceModel::Order { id, printed: rules.orders.get(id).cloned() },
        CardRef::Tech(id) => {
            let printed = rules.techs.get(id)?.clone();
            let mut copies: Vec<TechCardId> =
                rules.techs.iter().filter(|c| c.name == printed.name).map(|c| c.id).collect();
            copies.sort_unstable_by_key(|c| c.0);
            CardFaceModel::Tech { copies, printed, placed: Vec::new() }
        }
        CardRef::Site(index) => CardFaceModel::Site {
            index,
            site: rules.building_sites.get(usize::from(index))?.clone(),
        },
        CardRef::Achievement(ty) => CardFaceModel::Achievement(
            achievement_registry().into_iter().find(|a| a.achievement_type == ty)?,
        ),
    };
    Some(IndexCard { face, selection: None, location: None })
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

/// The board region: one [`TileModel`] per area, in `AreaId` order, with the
/// `◈` mark set on every area the ENGINE's `legal` list offers the President
/// (spec §0.7).
///
/// `legal` is the list the caller already holds, passed rather than generated
/// for the reason this module's header states: a model that called the move
/// generator itself would be both a second enumeration and a second opinion on
/// legality. A caller that has no list — or is drawing a board for something
/// other than the current decision — passes `&[]` and gets an unmarked board,
/// which is the truthful model of "nothing is offered here", not a defaulted
/// one.
pub fn build_board(state: &GameState, rules: &Rules, legal: &[Move]) -> BoardModel {
    let reachable = president_destinations(legal);
    BoardModel {
        player_count: state.config.player_count,
        tiles: state
            .board_layout
            .areas
            .iter()
            .map(|(&area, cfg)| {
                let mut tile = tile_from(state, rules, area, cfg);
                tile.reachable = reachable.contains(&area);
                tile
            })
            .collect(),
    }
}

/// Every area the President may end this decision on, straight off the legal
/// list: the two moves that put a President on a tile, and nothing else.
///
/// Exhaustive on the two variants by name rather than by a `destination` field
/// pattern, so a future move that also lands a President has to be added here
/// deliberately. Outside Movement the list holds neither variant and the set is
/// empty, which is why the `◈` mark needs no phase test of its own — it is
/// absent because the engine is offering nothing, not because the CLI decided
/// the phase was wrong.
pub fn president_destinations(legal: &[Move]) -> std::collections::BTreeSet<AreaId> {
    legal
        .iter()
        .filter_map(|mv| match mv {
            Move::MovePresident { destination, .. } => Some(*destination),
            Move::PlacePresident { destination } => Some(*destination),
            _ => None,
        })
        .collect()
}

/// One tile, or `None` for an area that is not on this board. An `Option`
/// rather than a defaulted tile: every field of a fabricated one would be a
/// lie about the board, and a display model is no place to panic over a stale
/// id either.
pub fn build_tile(state: &GameState, rules: &Rules, area: AreaId) -> Option<TileModel> {
    state.board_layout.areas.get(&area).map(|cfg| tile_from(state, rules, area, cfg))
}

/// How many near-miss rows [`build_not_offered`] will state.
///
/// The zone is an aid, not a report: the approved mock draws ONE row, a 4P
/// board gives a President up to six neighbours, and a screen that lists five
/// reasons has buried the two offered destinations it exists to help pick
/// between. Three is "the mock's row plus room for the other kinds to show up
/// together" — the cap is stated here rather than at the renderer because a
/// truncated list is a fact about the model, not about the drawing of it.
pub const NOT_OFFERED_ROWS: usize = 3;

/// The dimmed near-miss rows under a Movement decision (spec §0.7).
///
/// **What it looks at, exactly.** Only areas ADJACENT to where the President
/// stands — a near miss is a step the player was about to take, and every
/// non-adjacent area on the board is "not offered" in a sense nobody needs
/// explained. Only areas ABSENT from the engine's own destination list, taken
/// from `legal` (an area the engine offers is not a near miss however it got
/// there — Station, Tram and zero-cost teleports are all included for free by
/// asking the list rather than the rules). And only at a Movement decision with
/// the President on the board and no choice queued: a queued choice is a
/// different question, and a President in hand is answered by
/// `PlacePresident`, whose near misses are a different set with different
/// reasons.
///
/// **What it will not do.** Each candidate is matched against
/// [`NotOfferedReason`]'s facts in order, and an area that matches none gets no
/// row. That silence is the point: an unaffordable toll and a blocking
/// President are real reasons this function cannot read off the state without
/// re-deriving the movement rules, so it says nothing rather than guessing —
/// and the guess it would otherwise fall into is the tempting one,
/// `NoAssistantThere` on an area whose count is not zero at all.
///
/// **What it RETURNS is a pair**, and the second half is why: the rows to draw,
/// capped at [`NOT_OFFERED_ROWS`], and how many further near misses that cap
/// CUT. A caller cannot recover the second from the first — three rows look
/// identical whether they were all of them or the first three of five — so a
/// truncation that is not reported is a truncation that reads as completeness.
/// Both come off ONE walk of the neighbours rather than two.
pub fn build_not_offered(state: &GameState, legal: &[Move]) -> (Vec<NotOffered>, usize) {
    if state.turn_phase != TurnPhase::Movement || !state.pending_choices.is_empty() {
        return (Vec::new(), 0);
    }
    let PresidentLocation::OnBoard(here) = state.player(state.current_player).president_location
    else {
        return (Vec::new(), 0);
    };
    let offered = president_destinations(legal);
    let me = state.player(state.current_player);

    let mut rows = Vec::new();
    // `adjacent_areas` is the engine's own graph, already ascending by AreaId
    // (`board_gen` builds it into a BTreeMap of sorted vectors), so the rows
    // come out in board order without this function imposing one.
    for &area in state.board_layout.adjacent_areas(here) {
        if offered.contains(&area) {
            continue;
        }
        let is_canal =
            state.board_layout.areas.get(&area).map(|c| c.area_type) == Some(AreaType::Canal);
        let reason = if state.president_start_area == Some(area) {
            NotOfferedReason::StartedThereThisTurn
        } else if is_canal {
            NotOfferedReason::CanalIsNeverAStop
        } else if me.assistants.in_area(area) == 0 {
            NotOfferedReason::NoAssistantThere
        } else {
            continue;
        };
        rows.push(NotOffered { area, reason });
    }
    let suppressed = rows.len().saturating_sub(NOT_OFFERED_ROWS);
    rows.truncate(NOT_OFFERED_ROWS);
    (rows, suppressed)
}

fn tile_from(state: &GameState, rules: &Rules, area: AreaId, cfg: &AreaConfig) -> TileModel {
    let site = state.board_layout.building_sites.get(&area).map(|site| SiteModel {
        index: printed_site_index(site, rules),
        slots: std::array::from_fn(|i| SiteSlotModel {
            built_by: site.shophouses_built[i],
            reward: site.spaces[i],
        }),
        trading_house_vp: site.trading_house_reward,
        trading_house_owner: site.trading_house_owner,
    });

    TileModel {
        area,
        area_type: cfg.area_type,
        grid_position: cfg.grid_position,
        token: state.board_layout.five_power_tokens.get(&area).copied().flatten(),
        site,
        presidents: state
            .players
            .iter()
            .filter(|p| p.president_location == PresidentLocation::OnBoard(area))
            .map(|p| p.id)
            .collect(),
        has_station: state.station_location == Some(area),
        assistants: state.players.iter().map(|p| (p.id, p.assistants.in_area(area))).collect(),
        // Set by [`build_board`] from the engine's legal list; a tile built on
        // its own carries no mark, since a single tile has no list to read.
        reachable: false,
        canal_crossings: canal_partners(state, area),
    }
}

/// Every area `area` reaches only by crossing the Canal, ascending and deduped.
///
/// Each stored pair is scanned in BOTH orders so a crossing shows on both of its
/// members — the retired `render::adjacency_suffix` did the same, for the same
/// reason: `canal_crossings` stores one entry per pair, and an area that only
/// ever matched the first member would show half its crossings.
fn canal_partners(state: &GameState, area: AreaId) -> Vec<AreaId> {
    let mut out: Vec<AreaId> = state
        .board_layout
        .canal_crossings
        .iter()
        .filter_map(|&(a, b)| {
            if a == area {
                Some(b)
            } else if b == area {
                Some(a)
            } else {
                None
            }
        })
        .collect();
    out.sort_unstable_by_key(|a| a.0);
    out.dedup();
    out
}

/// The printed index of a runtime building site, by unique content match
/// against `Rules::building_sites` — see [`SiteModel::index`] on why the
/// runtime card cannot simply carry it. The 24 standard cards are pairwise
/// distinct in printed content, so the match is unique on a real board; a
/// registry with duplicates (or none, as in `Rules::empty_for_tests`) yields
/// `None` rather than an arbitrary first hit.
pub fn printed_site_index(site: &BuildingSite, rules: &Rules) -> Option<u8> {
    let mut found = None;
    for (i, printed) in rules.building_sites.iter().enumerate() {
        if printed.spaces == site.spaces
            && printed.trading_house_reward == site.trading_house_reward
        {
            if found.is_some() {
                return None;
            }
            found = Some(i as u8);
        }
    }
    found
}

/// The players panel. `seats` is indexed by seat; a seat it does not cover
/// masks (see [`SeatFacts::for_seat`]).
pub fn build_players(state: &GameState, rules: &Rules, seats: &[SeatFacts]) -> PlayersPanel {
    PlayersPanel {
        players: state.players.iter().map(|p| build_player(state, rules, p, seats)).collect(),
    }
}

fn build_player(
    state: &GameState,
    rules: &Rules,
    p: &PlayerState,
    seats: &[SeatFacts],
) -> PlayerModel {
    let facts = SeatFacts::for_seat(seats, p.id);
    PlayerModel {
        seat: p.id,
        color: p.color,
        money: p.money,
        victory_points: p.victory_points,
        goods: GoodsModel {
            copper: p.copper,
            silk: p.silk,
            tea: p.tea,
            seafood: p.seafood,
            imports: p.imported_goods,
        },
        assistants: AssistantZones {
            in_hand: p.assistants.in_hand,
            on_board: p.assistants.on_board.values().sum(),
            in_warehouse: p.assistants.in_warehouse,
            on_achievements: p.assistants.on_achievements,
            on_church: p.assistants.on_church,
            on_customs: p.assistants.on_customs,
        },
        shophouses: ShophouseZones {
            in_hand: p.shophouses.in_hand,
            in_warehouse: p.shophouses.in_warehouse,
            built: p.shophouses.total_built() as u8,
        },
        trading_houses: TradingHouseZones {
            in_hand: p.trading_houses.in_hand,
            in_warehouse: p.trading_houses.in_warehouse,
            built: p.trading_houses.total_built() as u8,
            next_take_cost: p.trading_houses.next_cost(),
        },
        technologies: p
            .technologies
            .iter()
            .map(|&id| TechModel { id, printed: rules.techs.get(id).cloned() })
            .collect(),
        mercenaries_unused: p.mercenaries.unused_count(),
        mercenaries_used: p.mercenaries.used.values().sum(),
        five_power_tokens: p.five_power_tokens,
        president: p.president_location,
        orders_in_hand: p.order_cards_in_hand.len(),
        completed_orders: p.completed_orders.len(),
        is_current: p.id == state.current_player,
        is_bot: facts.is_bot,
        hand_hidden: facts.hand_hidden,
    }
}

/// The tracks panel: both management boards, every printed space, plus the
/// Station and the shared mercenary supply that read beside them.
pub fn build_tracks(state: &GameState) -> TracksPanel {
    let church = (0..state.church_track.slots.len())
        .map(|i| {
            let faith = ChurchTrack::index_to_faith(i);
            ChurchSpaceModel {
                faith,
                vp: ChurchTrack::vp_reward(faith),
                movement_rider: ChurchTrack::movement_rider(faith),
                occupant: state.church_track.slots[i].map(|s| s.player),
            }
        })
        .collect();
    let customs = (0..state.customs_track.slots.len())
        .map(|i| {
            let space = CustomsTrack::index_to_space(i);
            let (warehouse_rider, yen_rider) = CustomsTrack::rider(space);
            CustomsSpaceModel {
                space,
                import_cost: CustomsTrack::space_cost(space),
                vp: CustomsTrack::vp_reward(space),
                warehouse_rider,
                yen_rider,
                occupant: state.customs_track.slots[i].map(|s| s.player),
            }
        })
        .collect();

    TracksPanel {
        church,
        church_filled: state.church_track.total_assistants(),
        customs,
        customs_filled: state.customs_track.total_assistants(),
        trigger_at: state.config.track_trigger(),
        station: state.station_location,
        mercenary_supply: state.mercenary_supply.clone(),
    }
}

/// The achievements panel: the three cards in play with per-seat progress.
/// **Every achievement type there is** — the population a SETUP prompt indexes,
/// as against the three in play [`build_achievements`] returns.
///
/// The roster is spelled here because core exposes no `AchievementType::ALL`
/// (and a core edit is out of scope for this phase), so it is held the way
/// `style`'s `AREA_TYPES` is held: named exhaustively, and pinned by
/// `index_render::tests::the_achievement_index_also_renders_the_whole_registry_for_setup`,
/// which counts the categories back out — twelve types, three groups, four per
/// category is the printed set, so a type dropped from this list fails there
/// rather than quietly vanishing from a setup screen.
///
/// Claim state and progress are EMPTY: none of these is on a table yet, and
/// zeros would read as "nobody has made progress" rather than "this card is not
/// in play". The renderer says the latter.
pub fn achievement_registry() -> Vec<AchievementModel> {
    [
        AchievementType::Copper5,
        AchievementType::Silk6,
        AchievementType::Tea7,
        AchievementType::Seafood7,
        AchievementType::Money10,
        AchievementType::Tokens5,
        AchievementType::Orders5,
        AchievementType::Technologies4,
        AchievementType::Production4,
        AchievementType::Commercial4,
        AchievementType::Prod3Comm2,
        AchievementType::Prod2Comm3,
    ]
    .into_iter()
    .map(|achievement_type| AchievementModel {
        achievement_type,
        category: achievement_type.category(),
        first_place_vp: achievement_type.first_place_vp(),
        subsequent_vp: achievement_type.subsequent_vp(),
        claimed_by: Vec::new(),
        progress: Vec::new(),
    })
    .collect()
}

pub fn build_achievements(state: &GameState) -> AchievementsPanel {
    AchievementsPanel {
        cards: state
            .achievements
            .iter()
            .map(|card| {
                let achievement_type = card.achievement_type;
                AchievementModel {
                    achievement_type,
                    category: achievement_type.category(),
                    first_place_vp: achievement_type.first_place_vp(),
                    subsequent_vp: achievement_type.subsequent_vp(),
                    claimed_by: card.claimed_by.clone(),
                    progress: state
                        .players
                        .iter()
                        .map(|p| {
                            let (current, needed) =
                                achievement_progress(state, p.id, achievement_type);
                            (p.id, current, needed)
                        })
                        .collect(),
                }
            })
            .collect(),
    }
}

/// One seat's hand. `hidden` is the EFFECTIVE mask the caller has already
/// resolved; a hidden panel carries the count and no cards at all. `None`
/// for a seat the state does not hold — the same totality posture as
/// [`build_tile`]: a display model is no place to panic over a stale id
/// (T0-review N2 aligned the two).
pub fn build_hand(
    state: &GameState,
    rules: &Rules,
    seat: PlayerId,
    hidden: bool,
) -> Option<HandPanel> {
    let p = state.players.iter().find(|p| p.id == seat)?;
    Some(HandPanel {
        seat,
        hidden,
        cards: if hidden {
            Vec::new()
        } else {
            p.order_cards_in_hand
                .iter()
                .map(|&id| HandCardModel {
                    id,
                    printed: rules.orders.get(id).cloned(),
                    is_starting_order: p.starting_order == Some(id),
                })
                .collect()
        },
        card_count: p.order_cards_in_hand.len(),
    })
}

/// The decision zone over the legal list the caller already holds. `legal` is
/// the ENGINE's list verbatim — this builder never generates or filters moves.
pub fn build_decision(state: &GameState, rules: &Rules, legal: &[Move]) -> DecisionModel {
    let deciding = if state.is_game_over() {
        Deciding::GameOver
    } else if let Some(head) = state.pending_choices.first() {
        Deciding::Choice { head: head.clone(), queued: state.pending_choices.len() }
    } else {
        Deciding::Phase(state.turn_phase)
    };

    let (not_offered, not_offered_suppressed) = build_not_offered(state, legal);

    DecisionModel {
        turn_number: state.turn_number,
        seat: state.current_player,
        phase_chain: PHASE_CHAIN.to_vec(),
        current_phase: phase_chain_index(state.turn_phase),
        deciding,
        options: legal
            .iter()
            .enumerate()
            .map(|(index, mv)| OptionModel {
                number: menu_number(index),
                mv: mv.clone(),
                fee: option_fee(state, rules, mv),
            })
            .collect(),
        not_offered,
        not_offered_suppressed,
    }
}

/// A phase's position in [`PHASE_CHAIN`]. Exhaustive on purpose (see the
/// constant's doc): adding a `TurnPhase` variant must not compile until the
/// chain names where it sits.
fn phase_chain_index(phase: TurnPhase) -> usize {
    match phase {
        TurnPhase::AdditionalActionFirst => 0,
        TurnPhase::Placement => 1,
        TurnPhase::Movement => 2,
        TurnPhase::AreaAction => 3,
        TurnPhase::FivePowerBonus => 4,
        TurnPhase::Building => 5,
        TurnPhase::Recovery => 6,
        TurnPhase::AdditionalActionSecond => 7,
        TurnPhase::TurnEnd => 8,
    }
}

/// The fee a set of assistant PLACEMENTS incurs: 1 Yen per assistant placed
/// into an area holding an opponent President, credited to that President's
/// owner. `placements` lists one entry PER ASSISTANT — Option B passes its area
/// twice, plus the Telegram bonus area — because the charge is per pawn, not
/// per area (rulebook: Option B's two-into-one-area costs 2, not 1).
///
/// **This is the arithmetic `describe::placement_fee_note` prints, and it lives
/// here so the two cannot say different numbers** (T0-review MF1). The engine's
/// own `calculate_placement_cost` / `credit_placement_fees` pair is
/// `pub(crate)` to yokohama-core and this crate may not widen it, so the
/// sourcing is this mirror plus the scratch-apply pin
/// `describe::placement_fee_preview_matches_the_engine_charge`, which applies a
/// real placement and compares the money actually charged: drift from the
/// engine fails that test, and it now guards the model as well as the note,
/// because the note is computed FROM the model.
///
/// `None` rather than a zeroed model when nothing is owed, matching
/// [`OptionModel::fee`]'s own convention — a zeroed fee reads as "priced, and
/// free", which for a placement into an empty area is not what happened.
pub fn placement_fee(state: &GameState, placements: &[AreaId]) -> Option<FeeModel> {
    let me = state.current_player;
    let mut per_owner: std::collections::BTreeMap<PlayerId, u32> =
        std::collections::BTreeMap::new();
    for &area in placements {
        for p in &state.players {
            if p.id != me && p.president_location == PresidentLocation::OnBoard(area) {
                *per_owner.entry(p.id).or_insert(0) += 1;
            }
        }
    }
    if per_owner.is_empty() {
        return None;
    }
    Some(FeeModel {
        total: per_owner.values().sum(),
        to_players: per_owner.into_iter().collect(),
        // Placement pays PLAYERS only; the Canal toll is a movement concept and
        // no placement ever crosses one.
        canal_tolls: 0,
    })
}

/// The fee an option carries, priced by the ENGINE.
///
/// `MovePresident` reads `route_fee_vector`, whose `path` already ends at the
/// destination — the same slice `apply_movement` prices, passed unchanged so
/// the last hop is never double-counted. `PlacePresident` reads
/// `calculate_place_president_fee`, which returns a total rather than an
/// attribution; the engine's rule IS 1 Yen to each opponent President owner
/// standing on the destination, so listing exactly those owners at 1 each sums
/// to the engine's total by construction — and the Stagecoach waiver, which
/// zeroes that total, yields no recipients at all. The two PLACEMENT moves go
/// through [`placement_fee`], one entry per assistant.
///
/// The wildcard arm is deliberate rather than the exhaustive match this repo
/// prefers elsewhere: these four moves are the only ones that move money to
/// another player as a side effect of being CHOSEN, and every other variant
/// would have to be listed as `None` for no gain. (Costs a player pays to the
/// BANK — a tech's price, a warehouse take — are part of what the option DOES,
/// and `describe` states them in the option's own words.)
pub fn option_fee(state: &GameState, rules: &Rules, mv: &Move) -> Option<FeeModel> {
    match mv {
        Move::PlaceThreeAreas { placements } => placement_fee(state, placements),
        Move::PlaceTwoSameArea { area, adjacent_bonus } => {
            let mut placements = vec![*area, *area];
            if let Some(adj) = adjacent_bonus {
                placements.push(*adj);
            }
            placement_fee(state, &placements)
        }
        Move::MovePresident { path, .. } => {
            let vector = route_fee_vector(
                state,
                state.current_player,
                path,
                Some(&rules.techs),
                false,
            );
            Some(FeeModel {
                total: vector.values().sum(),
                to_players: vector
                    .iter()
                    .filter_map(|(recipient, amount)| recipient.map(|id| (id, *amount)))
                    .collect(),
                canal_tolls: vector.get(&None).copied().unwrap_or(0),
            })
        }
        Move::PlacePresident { destination } => {
            let total = calculate_place_president_fee(
                state,
                state.current_player,
                *destination,
                Some(&rules.techs),
            );
            Some(FeeModel {
                total,
                to_players: if total == 0 {
                    Vec::new()
                } else {
                    state
                        .players
                        .iter()
                        .filter(|o| {
                            o.id != state.current_player
                                && o.president_location
                                    == PresidentLocation::OnBoard(*destination)
                        })
                        .map(|o| (o.id, 1))
                        .collect()
                },
                canal_tolls: 0,
            })
        }
        _ => None,
    }
}

/// The dashboard's panels in DISPLAY ORDER — players, tracks, achievements,
/// then the viewing seat's own hand.
///
/// The order is decided HERE and nowhere else, which is the point of the
/// function existing rather than the list being written out at each call site.
/// It has two callers with different needs — [`build_frame`], which wants a
/// whole frame, and the renderer's adapter, which wants the dashboard alone
/// because the composed screen still draws its decision zone the pre-redesign
/// way until T4 — and two inline copies of this list would be two places for
/// the frame's shape to drift apart.
///
/// The hand panel is absent, not empty, for a `seat` the state does not hold:
/// [`build_hand`] returns `None` there, and a panel captioned "YOUR HAND" over
/// nobody's hand is worse than no panel.
pub fn build_dashboard(
    state: &GameState,
    rules: &Rules,
    seats: &[SeatFacts],
    seat: PlayerId,
) -> Vec<Panel> {
    let mut panels = vec![
        Panel::Players(build_players(state, rules, seats)),
        Panel::Tracks(build_tracks(state)),
        Panel::Achievements(build_achievements(state)),
    ];
    if let Some(hand) = build_hand(state, rules, seat, SeatFacts::for_seat(seats, seat).hand_hidden)
    {
        panels.push(Panel::Hand(hand));
    }
    panels
}

/// A whole frame: the board, the dashboard in display order, and the decision
/// zone. `seat` is the seat whose own hand the dashboard shows.
pub fn build_frame(
    state: &GameState,
    rules: &Rules,
    seats: &[SeatFacts],
    seat: PlayerId,
    legal: &[Move],
    title: TitleModel,
) -> FrameModel {
    FrameModel {
        title,
        board: build_board(state, rules, legal),
        dashboard: build_dashboard(state, rules, seats, seat),
        decision: build_decision(state, rules, legal),
    }
}

#[cfg(test)]
mod tests {
    // `super::*` already re-exposes the `yokohama_core` glob this module
    // imports, so the engine types arrive with it.
    use super::*;

    /// A REAL generated board at `n` players, the same fixture shape
    /// `render.rs`'s tests use (seed 1): 3P/4P carry a Canal tile, 2P does not.
    fn state_np(n: u8) -> (GameState, Rules) {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(n), &rules, 1).unwrap();
        let state =
            finalize_setup(pending, &vec![StartingOrderChoice::Keep(0); n as usize]).unwrap();
        (state, rules)
    }

    #[test]
    fn the_board_model_carries_every_area_with_its_type_token_and_printed_site() {
        let (state, rules) = state_np(2);
        let board = build_board(&state, &rules, &[]);
        assert_eq!(board.player_count, 2);
        assert_eq!(board.tiles.len(), state.board_layout.areas.len());

        for tile in &board.tiles {
            let cfg = &state.board_layout.areas[&tile.area];
            assert_eq!(tile.area_type, cfg.area_type, "A{:02} type", tile.area.0);
            assert_eq!(tile.grid_position, cfg.grid_position, "A{:02} position", tile.area.0);
            assert_eq!(
                tile.token,
                state.board_layout.five_power_tokens.get(&tile.area).copied().flatten(),
                "A{:02} 5-Power face",
                tile.area.0,
            );

            let site =
                tile.site.as_ref().unwrap_or_else(|| panic!("A{:02} has a site", tile.area.0));
            let idx = site.index.unwrap_or_else(|| panic!("A{:02} site index", tile.area.0));
            let printed = &rules.building_sites[idx as usize];
            for (slot, printed_space) in site.slots.iter().zip(printed.spaces.iter()) {
                assert_eq!(&slot.reward, printed_space, "A{:02} printed space", tile.area.0);
            }
            assert_eq!(site.trading_house_vp, printed.trading_house_reward);
        }
    }

    #[test]
    fn tiles_open_with_no_pawns_no_station_and_no_reachable_mark() {
        let (state, rules) = state_np(2);
        for tile in build_board(&state, &rules, &[]).tiles {
            assert!(tile.presidents.is_empty(), "A{:02} presidents", tile.area.0);
            assert!(!tile.has_station, "A{:02} station", tile.area.0);
            assert_eq!(tile.assistants.len(), 2, "one entry per seat, zeros included");
            assert!(tile.assistants.iter().all(|&(_, n)| n == 0), "A{:02} pawns", tile.area.0);
            assert!(!tile.reachable, "the ◈ feed is T4's; T0 marks nothing reachable");
        }
    }

    /// The pawn fields are populated from state, not stubbed: a President and
    /// an assistant placed on a tile show up on that tile's model and nowhere
    /// else.
    #[test]
    fn tile_pawns_read_from_state() {
        let (mut state, rules) = state_np(2);
        let here = AreaId::new(0);
        let elsewhere = AreaId::new(1);
        state.players[1].president_location = PresidentLocation::OnBoard(here);
        state.players[0].assistants.on_board.insert(here, 3);
        state.station_location = Some(here);

        let board = build_board(&state, &rules, &[]);
        let tile = board.tiles.iter().find(|t| t.area == here).expect("A00");
        assert_eq!(tile.presidents, vec![PlayerId::new(1)]);
        assert!(tile.has_station);
        assert_eq!(tile.assistants, vec![(PlayerId::new(0), 3), (PlayerId::new(1), 0)]);

        let other = board.tiles.iter().find(|t| t.area == elsewhere).expect("A01");
        assert!(other.presidents.is_empty());
        assert!(!other.has_station);
        assert!(other.assistants.iter().all(|&(_, n)| n == 0));
    }

    /// A single tile builds the same model the board builder gives it, and an
    /// area that is not on this board yields nothing rather than a fabricated
    /// tile.
    #[test]
    fn a_single_tile_matches_the_board_and_an_unknown_area_yields_none() {
        let (state, rules) = state_np(2);
        let area = AreaId::new(0);
        let from_board = build_board(&state, &rules, &[])
            .tiles
            .into_iter()
            .find(|t| t.area == area)
            .expect("A00 is on the 2P board");
        assert_eq!(build_tile(&state, &rules, area), Some(from_board));
        assert_eq!(
            build_tile(&state, &rules, AreaId::new(17)),
            None,
            "the 2P board has 10 areas; A17 is not one of them",
        );
    }

    /// [`SiteModel::index`] recovers a printed index by CONTENT match, which
    /// is only well-defined because the 24 standard site cards are pairwise
    /// distinct in what they print. Pin that assumption where the lookup
    /// depends on it, rather than leaving it to the data file.
    #[test]
    fn every_standard_building_site_is_distinguishable_by_its_printed_face() {
        let rules = Rules::standard();
        assert_eq!(rules.building_sites.len(), 24);
        for (i, a) in rules.building_sites.iter().enumerate() {
            for (j, b) in rules.building_sites.iter().enumerate().skip(i + 1) {
                assert!(
                    a.spaces != b.spaces || a.trading_house_reward != b.trading_house_reward,
                    "sites {i} and {j} print the same face, so no index lookup can tell them apart",
                );
            }
        }
    }

    /// Two Presidents CAN share a tile (Postal System lets a player stop where
    /// an opponent stands), so the field is a list, not an `Option`.
    #[test]
    fn a_tile_can_carry_more_than_one_president() {
        let (mut state, rules) = state_np(2);
        let here = AreaId::new(0);
        state.players[0].president_location = PresidentLocation::OnBoard(here);
        state.players[1].president_location = PresidentLocation::OnBoard(here);
        let board = build_board(&state, &rules, &[]);
        let tile = board.tiles.iter().find(|t| t.area == here).expect("A00");
        assert_eq!(tile.presidents, vec![PlayerId::new(0), PlayerId::new(1)]);
    }

    /// The Canal holds no building site (and no 5-Power token), so its site
    /// model is `None` rather than a fabricated empty card.
    #[test]
    fn the_canal_tile_has_no_building_site() {
        let (state, rules) = state_np(4);
        let board = build_board(&state, &rules, &[]);
        let canal = board
            .tiles
            .iter()
            .find(|t| t.area_type == AreaType::Canal)
            .expect("4P board has a Canal");
        assert!(canal.site.is_none(), "the Canal has no building site");
        assert!(canal.token.is_none(), "the Canal has no 5-Power token");
        for tile in board.tiles.iter().filter(|t| t.area_type != AreaType::Canal) {
            assert!(tile.site.is_some(), "A{:02} must carry its site", tile.area.0);
        }
    }

    #[test]
    fn player_models_carry_the_printed_starting_zones() {
        let (state, rules) = state_np(2);
        let panel = build_players(&state, &rules, &SeatFacts::all_open(2));
        assert_eq!(panel.players.len(), 2);

        let red = &panel.players[0];
        assert_eq!(red.seat, PlayerId::new(0));
        assert_eq!(red.color, PlayerColor::Red);
        assert_eq!(red.money, 3, "the start player opens on 3 Yen");
        assert_eq!(red.victory_points, 0);
        assert_eq!(red.goods, GoodsModel { copper: 1, silk: 1, tea: 1, seafood: 1, imports: 0 });
        assert_eq!(red.assistants.in_hand, 8);
        assert_eq!(red.assistants.in_warehouse, 12);
        assert_eq!(red.assistants.on_board, 0);
        assert_eq!(red.assistants.locked(), 0);
        assert_eq!(red.shophouses.in_hand, 2);
        assert_eq!(red.shophouses.in_warehouse, 6);
        assert_eq!(red.shophouses.built, 0);
        assert_eq!(red.trading_houses.in_hand, 0);
        assert_eq!(red.trading_houses.in_warehouse, 4);
        assert_eq!(red.trading_houses.built, 0);
        assert_eq!(
            red.trading_houses.next_take_cost, 4,
            "the printed 4/5/6/7 progression starts at 4",
        );
        assert_eq!(red.president, PresidentLocation::InHand);
        assert_eq!(red.orders_in_hand, 1, "the kept starting order");
        assert!(red.is_current, "Red is the start player");
        assert!(!red.is_bot);
        assert!(!red.hand_hidden);

        assert_eq!(panel.players[1].money, 4, "every other seat opens on 4 Yen");
        assert!(!panel.players[1].is_current);
    }

    /// Assistant zones cover ALL SIX `AssistantState` locations, so the
    /// printed numbers reconcile to the player's 20 pawns.
    #[test]
    fn assistant_zones_account_for_every_pawn() {
        let (mut state, rules) = state_np(2);
        let p = &mut state.players[0];
        p.assistants.in_hand = 3;
        p.assistants.in_warehouse = 9;
        p.assistants.on_board.insert(AreaId::new(0), 4);
        p.assistants.on_board.insert(AreaId::new(1), 1);
        p.assistants.on_achievements = 1;
        p.assistants.on_church = 1;
        p.assistants.on_customs = 1;

        let zones = build_players(&state, &rules, &SeatFacts::all_open(2)).players[0].assistants;
        assert_eq!(zones.on_board, 5, "the on-board zone sums every area");
        assert_eq!(zones.locked(), 3, "goals + church + customs");
        assert_eq!(zones.total(), 20, "every pawn is accounted for");
    }

    #[test]
    fn player_models_name_owned_technologies_from_the_registry() {
        let (mut state, rules) = state_np(2);
        state.players[0].technologies.push(TechCardId(11));
        let panel = build_players(&state, &rules, &SeatFacts::all_open(2));
        let techs = &panel.players[0].technologies;
        assert_eq!(techs.len(), 1);
        assert_eq!(techs[0].id, TechCardId(11));
        let printed = techs[0].printed.as_ref().expect("the registry knows this id");
        assert_eq!(printed.name, "Ball");
        assert_eq!(printed.effect, TechEffect::Ball);
        assert!(panel.players[1].technologies.is_empty());
    }

    /// An id the registry does not know carries no invented name.
    #[test]
    fn an_unknown_technology_id_carries_no_printed_card() {
        let (mut state, rules) = state_np(2);
        state.players[0].technologies.push(TechCardId(254));
        let techs = &build_players(&state, &rules, &SeatFacts::all_open(2)).players[0].technologies;
        assert_eq!(techs[0].id, TechCardId(254));
        assert!(techs[0].printed.is_none());
    }

    #[test]
    fn player_models_split_used_from_unused_mercenaries() {
        let (mut state, rules) = state_np(2);
        state.players[0].mercenaries.available.insert(Country::English, 2);
        state.players[0].mercenaries.used.insert(Country::Dutch, 1);
        let red = &build_players(&state, &rules, &SeatFacts::all_open(2)).players[0];
        assert_eq!(red.mercenaries_unused, 2, "unused mercenaries are the VP-bearing ones");
        assert_eq!(red.mercenaries_used, 1);
    }

    #[test]
    fn seat_facts_ride_through_to_the_player_model() {
        let (state, rules) = state_np(2);
        let seats = vec![
            SeatFacts { is_bot: false, hand_hidden: true },
            SeatFacts { is_bot: true, hand_hidden: true },
        ];
        let panel = build_players(&state, &rules, &seats);
        assert!(panel.players[0].hand_hidden && !panel.players[0].is_bot);
        assert!(panel.players[1].hand_hidden && panel.players[1].is_bot);
    }

    /// A seat past the end of the supplied table reads as HIDDEN and human —
    /// the same safe direction `render::SeatDisplay` takes.
    #[test]
    fn a_missing_seat_fact_masks_rather_than_reveals() {
        let (state, rules) = state_np(2);
        let panel = build_players(&state, &rules, &[]);
        assert!(panel.players.iter().all(|p| p.hand_hidden), "an absent policy must mask");
        assert!(panel.players.iter().all(|p| !p.is_bot), "...but must not claim an unconfigured bot");
    }

    #[test]
    fn the_phase_chain_lists_every_turn_phase_in_order_with_the_current_one_marked() {
        let (mut state, rules) = state_np(2);
        assert_eq!(
            PHASE_CHAIN,
            [
                TurnPhase::AdditionalActionFirst,
                TurnPhase::Placement,
                TurnPhase::Movement,
                TurnPhase::AreaAction,
                TurnPhase::FivePowerBonus,
                TurnPhase::Building,
                TurnPhase::Recovery,
                TurnPhase::AdditionalActionSecond,
                TurnPhase::TurnEnd,
            ],
        );

        for (i, phase) in PHASE_CHAIN.iter().enumerate() {
            state.turn_phase = *phase;
            let decision = build_decision(&state, &rules, &[]);
            assert_eq!(decision.phase_chain, PHASE_CHAIN.to_vec());
            assert_eq!(decision.current_phase, i, "{phase:?} sits at chain index {i}");
            assert_eq!(decision.deciding, Deciding::Phase(*phase));
        }
    }

    #[test]
    fn the_decision_model_carries_the_turn_the_seat_and_the_queued_choice() {
        let (mut state, rules) = state_np(2);
        state.turn_number = 9;
        state.current_player = PlayerId::new(1);
        state.pending_choices = vec![PendingChoice::AnyGood, PendingChoice::WarehouseTake];

        let decision = build_decision(&state, &rules, &[]);
        assert_eq!(decision.turn_number, 9);
        assert_eq!(decision.seat, PlayerId::new(1));
        assert_eq!(
            decision.deciding,
            Deciding::Choice { head: PendingChoice::AnyGood, queued: 2 },
            "a queued choice outranks the phase, exactly as the prompt banner does",
        );
    }

    /// Options are numbered by their index in the list the ENGINE supplied —
    /// the model never re-derives legality (engine-as-sole-authority).
    #[test]
    fn options_are_numbered_by_their_index_in_the_supplied_legal_list() {
        let (state, rules) = state_np(2);
        let legal = get_legal_moves(&state, &rules);
        assert!(!legal.is_empty(), "the opening position offers moves");
        let decision = build_decision(&state, &rules, &legal);
        assert_eq!(decision.options.len(), legal.len());
        for (i, opt) in decision.options.iter().enumerate() {
            // The number SHOWN, which is the index plus the base (T5's
            // numbering ruling): the first option is `[1]`, not `[0]`.
            assert_eq!(opt.number, menu_number(i));
            assert_eq!(opt.mv, legal[i]);
        }
        assert_eq!(decision.options[0].number, NUMBER_BASE, "lists start at the base");
        assert!(
            decision.not_offered.is_empty(),
            "near-miss rows belong to a Movement decision; this is not one",
        );
    }

    /// The `◈` feed: exactly the areas the ENGINE's list offers a President,
    /// and nothing else — not "areas the player could reach", which is a
    /// question this crate is forbidden from answering.
    #[test]
    fn the_reachable_marks_are_exactly_the_engines_president_destinations() {
        let (state, rules) = state_np(2);
        let (a, b) = (AreaId::new(2), AreaId::new(5));
        let legal = vec![
            Move::MovePresident { path: vec![a], destination: a },
            Move::PlacePresident { destination: b },
            // Not a President move: must NOT mark anything.
            Move::PlaceTwoSameArea { area: AreaId::new(7), adjacent_bonus: None },
        ];
        let board = build_board(&state, &rules, &legal);
        let marked: Vec<AreaId> =
            board.tiles.iter().filter(|t| t.reachable).map(|t| t.area).collect();
        assert_eq!(marked, vec![a, b], "one mark per offered destination, and no others");

        let unmarked = build_board(&state, &rules, &[]);
        assert!(
            unmarked.tiles.iter().all(|t| !t.reachable),
            "no list, no marks -- an unmarked board is the truthful model of 'nothing offered'",
        );
    }

    /// The near-miss rows: adjacent, absent from the engine's list, and each
    /// one carrying the observable fact that explains the absence.
    #[test]
    fn the_near_miss_rows_state_the_fact_behind_each_absence() {
        let (mut state, _rules) = state_np(2);
        let here = AreaId::new(4);
        state.turn_phase = TurnPhase::Movement;
        state.players[0].president_location = PresidentLocation::OnBoard(here);
        let neighbours = state.board_layout.adjacent_areas(here).to_vec();
        assert!(
            neighbours.len() > NOT_OFFERED_ROWS,
            "the fixture needs MORE neighbours than the cap, so the cut count below is not \
             vacuously zero: {neighbours:?}",
        );

        // (a) An area the player has no assistant in.
        let (rows, cut) = build_not_offered(&state, &[]);
        assert!(!rows.is_empty(), "every neighbour is a near miss when nothing is offered");
        assert!(
            rows.iter().all(|r| r.reason == NotOfferedReason::NoAssistantThere),
            "{rows:?}",
        );
        assert!(rows.iter().all(|r| neighbours.contains(&r.area)), "rows are adjacent only");
        assert!(rows.len() <= NOT_OFFERED_ROWS, "the row cap holds: {rows:?}");
        // ...and when it fires it SAYS so, which is the whole of the second
        // return value: this fixture offers nothing, so every neighbour past
        // the third is a row the zone will not draw.
        assert_eq!(cut, neighbours.len() - rows.len(), "the cap must report what it cut");

        // (b) The start-area rule outranks it, on the SAME area — the row a
        // player is most likely to be confused by, and the one an
        // assistant-count test would mis-explain.
        let start = neighbours[0];
        state.president_start_area = Some(start);
        state.players[0].assistants.on_board.insert(start, 1);
        let (rows, _) = build_not_offered(&state, &[]);
        let start_row = rows.iter().find(|r| r.area == start).expect("the start area is a row");
        assert_eq!(start_row.reason, NotOfferedReason::StartedThereThisTurn);

        // (c) An area WITH an assistant and no other fact against it gets no
        // row at all: the reason would be an unaffordable toll or a blocking
        // President, neither of which this reads off the state.
        state.president_start_area = None;
        let (rows, _) = build_not_offered(&state, &[]);
        assert!(rows.iter().all(|r| r.area != start), "silence beats a guess: {rows:?}");

        // (d) An area the engine DOES offer is not a near miss.
        let offered = neighbours[1];
        let legal = vec![Move::MovePresident { path: vec![offered], destination: offered }];
        let (rows, _) = build_not_offered(&state, &legal);
        assert!(rows.iter().all(|r| r.area != offered), "{rows:?}");
    }

    /// The Canal gets its own reason rather than the assistant one, which would
    /// name a shortfall the player could never fix.
    #[test]
    fn a_canal_neighbour_is_never_explained_as_a_missing_assistant() {
        let (mut state, _rules) = state_np(4);
        let canal = *state
            .board_layout
            .areas
            .iter()
            .find(|(_, cfg)| cfg.area_type == AreaType::Canal)
            .map(|(id, _)| id)
            .expect("a 4P board carries a Canal");
        let here = *state
            .board_layout
            .adjacent_areas(canal)
            .first()
            .expect("the Canal has neighbours");
        state.turn_phase = TurnPhase::Movement;
        state.players[0].president_location = PresidentLocation::OnBoard(here);

        let (rows, _) = build_not_offered(&state, &[]);
        let canal_row = rows.iter().find(|r| r.area == canal);
        if let Some(row) = canal_row {
            assert_eq!(row.reason, NotOfferedReason::CanalIsNeverAStop, "{rows:?}");
        } else {
            // The cap can crowd it out; then nothing may claim it either.
            assert!(rows.len() == NOT_OFFERED_ROWS, "the Canal row was dropped for no reason");
        }
    }

    /// Near-miss rows are a MOVEMENT thing: a queued choice is a different
    /// question, and a President in hand is answered by `PlacePresident`.
    #[test]
    fn no_near_miss_rows_outside_a_movement_decision() {
        let (mut state, _rules) = state_np(2);
        let here = AreaId::new(4);
        state.players[0].president_location = PresidentLocation::OnBoard(here);

        state.turn_phase = TurnPhase::Placement;
        assert_eq!(build_not_offered(&state, &[]), (Vec::new(), 0), "not a Movement decision");

        state.turn_phase = TurnPhase::Movement;
        state.pending_choices = vec![PendingChoice::AnyGood];
        assert_eq!(
            build_not_offered(&state, &[]),
            (Vec::new(), 0),
            "a queued choice outranks the phase",
        );

        state.pending_choices.clear();
        state.players[0].president_location = PresidentLocation::InHand;
        assert_eq!(
            build_not_offered(&state, &[]),
            (Vec::new(), 0),
            "no President on the board, no near miss",
        );
    }

    /// T0-review MF1, discharged: a placement into an area an opponent
    /// President stands in carries the fee the CLI has always previewed, and
    /// the preview is now computed FROM this model — so the engine pin behind
    /// `describe::placement_fee_note` guards both.
    #[test]
    fn placement_options_carry_the_fee_the_preview_prints() {
        let (mut state, rules) = state_np(2);
        let blocked = AreaId::new(0);
        let free = AreaId::new(1);
        state.players[1].president_location = PresidentLocation::OnBoard(blocked);

        let spread = Move::PlaceThreeAreas { placements: vec![blocked, free] };
        let doubled = Move::PlaceTwoSameArea { area: blocked, adjacent_bonus: Some(free) };
        let decision = build_decision(&state, &rules, &[spread, doubled]);

        let one = decision.options[0].fee.clone().expect("one assistant into the blocked area");
        assert_eq!(one.total, 1);
        assert_eq!(one.to_players, vec![(PlayerId::new(1), 1)]);

        // Option B's two pawns into one blocked area cost 2, not 1 — the rule
        // the fee is per ASSISTANT rather than per area.
        let two = decision.options[1].fee.clone().expect("two assistants into the blocked area");
        assert_eq!(two.total, 2, "1 Yen PER ASSISTANT placed");
        assert_eq!(two.to_players, vec![(PlayerId::new(1), 2)]);

        // ...and the words the CLI prints are this model, formatted.
        let note = crate::describe::placement_fee_note(&state, &[blocked, blocked, free]);
        assert!(note.contains("fee 2 yen"), "{note}");

        // A placement nobody is standing in carries no fee model at all.
        let empty = Move::PlaceThreeAreas { placements: vec![free] };
        assert!(build_decision(&state, &rules, &[empty]).options[0].fee.is_none());
    }

    #[test]
    fn placing_the_president_where_an_opponent_stands_carries_its_fee() {
        let (mut state, rules) = state_np(2);
        let destination = AreaId::new(0);
        state.players[1].president_location = PresidentLocation::OnBoard(destination);

        let legal = vec![Move::PlacePresident { destination }];
        let fee = build_decision(&state, &rules, &legal).options[0]
            .fee
            .clone()
            .expect("a placement onto an occupied area carries a fee");
        assert_eq!(fee.total, 1, "1 Yen to each President owner standing there");
        assert_eq!(fee.to_players, vec![(PlayerId::new(1), 1)]);
        assert_eq!(fee.canal_tolls, 0);
    }

    /// Movement fees come from the engine's own `route_fee_vector`, split into
    /// per-owner entries and the canal toll the bank takes.
    #[test]
    fn movement_options_carry_the_engines_own_fee_vector() {
        let (mut state, rules) = state_np(2);
        let (start, destination) = (AreaId::new(0), AreaId::new(1));
        state.players[0].president_location = PresidentLocation::OnBoard(start);
        state.players[1].president_location = PresidentLocation::OnBoard(destination);

        let path = vec![destination];
        let legal = vec![Move::MovePresident { path: path.clone(), destination }];
        let fee = build_decision(&state, &rules, &legal).options[0]
            .fee
            .clone()
            .expect("a walked route carries a fee model");

        let vector = route_fee_vector(&state, PlayerId::new(0), &path, Some(&rules.techs), false);
        assert_eq!(fee.total, vector.values().sum::<u32>());
        assert_eq!(fee.total, 1, "stopping on Blue's President costs 1 Yen");
        assert_eq!(fee.to_players, vec![(PlayerId::new(1), 1)]);
        assert_eq!(fee.canal_tolls, vector.get(&None).copied().unwrap_or(0));
    }

    /// A move with no fee dimension carries no fee model at all, rather than a
    /// zeroed one that would read as "measured, and free".
    #[test]
    fn moves_without_a_fee_dimension_carry_no_fee_model() {
        let (state, rules) = state_np(2);
        let legal = vec![Move::EndAdditionalPhase, Move::SkipBuilding];
        let decision = build_decision(&state, &rules, &legal);
        assert!(decision.options.iter().all(|o| o.fee.is_none()));
    }

    #[test]
    fn the_tracks_panel_carries_the_two_player_dummy() {
        let (state, _rules) = state_np(2);
        let tracks = build_tracks(&state);

        let faith_seven = tracks.church.iter().find(|s| s.faith == 7).expect("faith 7");
        assert_eq!(faith_seven.occupant, Some(PlayerId::DUMMY), "the 2P dummy holds faith 7");
        let space_four = tracks.customs.iter().find(|s| s.space == 4).expect("space 4");
        assert_eq!(space_four.occupant, Some(PlayerId::DUMMY), "the 2P dummy holds customs space 4");

        assert_eq!(tracks.church_filled, 0, "the dummy is not a pawn the trigger counts");
        assert_eq!(tracks.customs_filled, 0);
        assert_eq!(tracks.trigger_at, 4, "2P tracks fill at 4 real pawns");
    }

    #[test]
    fn the_tracks_panel_carries_every_printed_space_reward() {
        let (state, _rules) = state_np(2);
        let tracks = build_tracks(&state);

        assert_eq!(tracks.church.len(), 8, "faith 4-11");
        for (i, space) in tracks.church.iter().enumerate() {
            assert_eq!(space.faith, ChurchTrack::index_to_faith(i));
            assert_eq!(space.vp, ChurchTrack::vp_reward(space.faith));
            assert_eq!(space.movement_rider, ChurchTrack::movement_rider(space.faith));
        }

        assert_eq!(tracks.customs.len(), 7, "spaces 1-7");
        for (i, space) in tracks.customs.iter().enumerate() {
            assert_eq!(space.space, CustomsTrack::index_to_space(i));
            assert_eq!(space.import_cost, CustomsTrack::space_cost(space.space));
            assert_eq!(space.vp, CustomsTrack::vp_reward(space.space));
            let (warehouse, yen) = CustomsTrack::rider(space.space);
            assert_eq!((space.warehouse_rider, space.yen_rider), (warehouse, yen));
        }
    }

    #[test]
    fn the_tracks_panel_carries_the_board_level_station_and_supply() {
        let (mut state, _rules) = state_np(2);
        assert_eq!(build_tracks(&state).station, None, "the Station opens unplaced");
        state.station_location = Some(AreaId::new(3));
        let tracks = build_tracks(&state);
        assert_eq!(tracks.station, Some(AreaId::new(3)));
        assert_eq!(tracks.mercenary_supply, state.mercenary_supply);
    }

    #[test]
    fn the_achievements_panel_carries_typed_cards_with_per_seat_progress() {
        let (mut state, _rules) = state_np(2);
        state.players[0].copper = 5;
        let panel = build_achievements(&state);
        assert_eq!(panel.cards.len(), 3, "three achievement cards are in play");

        for (card, in_play) in panel.cards.iter().zip(state.achievements.iter()) {
            assert_eq!(card.achievement_type, in_play.achievement_type);
            assert_eq!(card.category, in_play.achievement_type.category());
            assert_eq!(card.first_place_vp, in_play.achievement_type.first_place_vp());
            assert_eq!(card.subsequent_vp, in_play.achievement_type.subsequent_vp());
            assert_eq!(card.claimed_by, in_play.claimed_by);
            assert_eq!(card.progress.len(), 2, "one row per seat");
            for &(seat, current, needed) in &card.progress {
                assert_eq!(
                    (current, needed),
                    achievement_progress(&state, seat, card.achievement_type),
                    "progress is the engine's own arithmetic",
                );
            }
        }
    }

    #[test]
    fn the_hand_panel_renders_the_seats_own_cards() {
        let (state, rules) = state_np(2);
        let seat = PlayerId::new(0);
        let panel = build_hand(&state, &rules, seat, false).expect("a live seat");
        assert_eq!(panel.seat, seat);
        assert!(!panel.hidden);
        assert_eq!(panel.card_count, 1);
        assert_eq!(panel.cards.len(), 1);

        let card = &panel.cards[0];
        let printed = card.printed.as_ref().expect("the dealt order is in the registry");
        let registry = rules.orders.get(card.id).expect("the registry knows the dealt order");
        assert_eq!(printed.country, registry.country);
        assert_eq!(printed.requirements, registry.requirements);
        assert_eq!(printed.rewards, registry.rewards);
        assert!(card.is_starting_order, "the kept deal is the seat's starting order");
    }

    /// The mask is a MODEL property, not a render-time one: a hidden hand
    /// carries its count and no card data at all, so a consumer that never
    /// reads the `hidden` flag still cannot leak the cards.
    #[test]
    fn a_hidden_hand_carries_the_count_but_never_the_cards() {
        let (state, rules) = state_np(2);
        let panel = build_hand(&state, &rules, PlayerId::new(0), true).expect("a live seat");
        assert!(panel.hidden);
        assert_eq!(panel.card_count, 1, "the count still reads");
        assert!(panel.cards.is_empty(), "a masked hand must carry no card data");
    }

    #[test]
    fn the_frame_assembles_the_board_and_the_four_dashboard_panels() {
        let (state, rules) = state_np(2);
        let legal = get_legal_moves(&state, &rules);
        let frame = build_frame(&state, &rules, &SeatFacts::all_open(2), PlayerId::new(0), &legal,
            TitleModel { mode: TitleMode::Sim, seed: 1, autosaved: false });

        assert_eq!(frame.board.tiles.len(), state.board_layout.areas.len());
        assert_eq!(frame.decision.options.len(), legal.len());
        assert!(
            matches!(
                frame.dashboard.as_slice(),
                [Panel::Players(_), Panel::Tracks(_), Panel::Achievements(_), Panel::Hand(_)]
            ),
            "the dashboard reads top to bottom as the approved mock does",
        );
    }

    /// The frame's hand panel obeys the viewing seat's own mask — a privacy
    /// toggle must reach the frame, not just the players panel.
    #[test]
    fn the_frames_hand_panel_obeys_the_viewing_seats_mask() {
        let (state, rules) = state_np(2);
        let seats = vec![SeatFacts { is_bot: false, hand_hidden: true }; 2];
        let frame = build_frame(&state, &rules, &seats, PlayerId::new(0), &[],
            TitleModel { mode: TitleMode::Sim, seed: 1, autosaved: false });
        let Some(Panel::Hand(hand)) = frame.dashboard.last() else {
            panic!("the hand panel closes the dashboard");
        };
        assert!(hand.hidden && hand.cards.is_empty() && hand.card_count == 1);
    }

    #[test]
    fn player_model_discriminates_fields_an_opening_state_cannot() {
        // T0-review SF1: opening fixtures make silk == tea and church ==
        // customs, so a transposed field read survives the suite. Mutate a
        // generated state to pairwise-distinct values and assert every
        // mirrored field exactly.
        let (mut state, rules) = state_np(2);
        let extra_completed = state.order_deck.pop().expect("the deck is not empty at setup");
        let area0 = *state.board_layout.areas.keys().next().expect("a generated board has areas");
        {
            let p = &mut state.players[0];
            p.copper = 1;
            p.silk = 2;
            p.tea = 3;
            p.seafood = 4;
            p.imported_goods = 5;
            p.assistants.on_church = 2;
            p.assistants.on_customs = 3;
            p.five_power_tokens = 2;
            p.completed_orders.push(extra_completed);
            p.shophouses.built.insert(area0, 1);
        }
        let m = build_players(&state, &rules, &SeatFacts::all_open(2)).players[0].clone();
        assert_eq!(
            (m.goods.copper, m.goods.silk, m.goods.tea, m.goods.seafood, m.goods.imports),
            (1, 2, 3, 4, 5)
        );
        assert_eq!((m.assistants.on_church, m.assistants.on_customs), (2, 3));
        assert_eq!(m.five_power_tokens, 2);
        assert_eq!(m.completed_orders, 1);
        assert_eq!(m.shophouses.built, 1);
    }

    #[test]
    fn tile_model_carries_built_ownership_and_the_trading_house_owner() {
        // T0-review SF1: `built_by` and `trading_house_owner` were never
        // asserted; a builder hardcoding `None` passed.
        let (mut state, rules) = state_np(2);
        let area = *state
            .board_layout
            .building_sites
            .keys()
            .next()
            .expect("a generated board has sites");
        let owner = state.players[0].id;
        let other = state.players[1].id;
        let site = state.board_layout.building_sites.get_mut(&area).expect("the site exists");
        site.shophouses_built[2] = Some(other);
        site.trading_house_owner = Some(owner);
        let tile = build_tile(&state, &rules, area).expect("the area exists");
        let s = tile.site.expect("a non-Canal area carries a site");
        assert_eq!(s.slots[2].built_by, Some(other));
        assert_eq!(s.slots[0].built_by, None);
        assert_eq!(s.trading_house_owner, Some(owner));
    }

    #[test]
    fn a_non_starting_hand_card_reads_is_starting_order_false() {
        // T0-review SF1: `is_starting_order` was only ever asserted true.
        let (mut state, rules) = state_np(2);
        let seat = state.players[0].id;
        let extra = state.order_deck.pop().expect("the deck is not empty at setup");
        state.players[0].order_cards_in_hand.push(extra);
        let panel = build_hand(&state, &rules, seat, false).expect("a live seat");
        let pushed = panel.cards.iter().find(|c| c.id == extra).expect("the pushed card");
        assert!(!pushed.is_starting_order);
        assert!(panel.cards.iter().any(|c| c.is_starting_order));
    }

    #[test]
    fn a_stale_seat_yields_no_hand_panel() {
        // Seat 3 is a valid `PlayerId` (the type checks 0-3 at construction)
        // but holds no chair in a 2-player state.
        let (state, rules) = state_np(2);
        assert!(build_hand(&state, &rules, PlayerId::new(3), false).is_none());
    }

    #[test]
    fn three_player_builders_smoke() {
        // T0-review N1: 2P and 4P were exercised, 3P not at all.
        let (state, rules) = state_np(3);
        assert_eq!(build_board(&state, &rules, &[]).tiles.len(), 14);
        assert_eq!(build_players(&state, &rules, &SeatFacts::all_open(3)).players.len(), 3);
        assert!(build_tracks(&state).church.iter().all(|s| s.occupant.is_none()));
    }
}
