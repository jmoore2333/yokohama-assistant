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
//! **T0 scope.** Nothing renders from these models yet. Two fields are shaped
//! here and filled by a later task, each marked at its declaration:
//! [`TileModel::reachable`] (the Movement `◈` feed) and
//! [`DecisionModel::not_offered`] (the dimmed near-miss rows) both land in T4,
//! and [`IndexModel`] is a stub T5 designs.

use yokohama_core::*;

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
    /// The option's index in the legal list the caller supplied — the same
    /// 0-based global numbering the prompt uses.
    pub number: usize,
    pub mv: Move,
    /// `Some` for the two moves that carry a President fee dimension
    /// (`MovePresident`, `PlacePresident`); `None` elsewhere, deliberately
    /// rather than a zeroed model, which would read as "priced, and free".
    pub fee: Option<FeeModel>,
}

/// Why a near-miss did not make the offered list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotOfferedReason {
    /// **T4 (the decision zone) replaces this** with the engine-derived reason
    /// set — spec §0.7's dimmed "not offered: <area> — <reason>" rows. T0
    /// defines the shape and populates nothing.
    Unspecified,
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
    /// **T4 fills this** (see [`NotOfferedReason`]); T0 always leaves it empty.
    pub not_offered: Vec<NotOffered>,
}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

/// A whole composed frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameModel {
    pub board: BoardModel,
    /// The dashboard panels in display order.
    pub dashboard: Vec<Panel>,
    pub decision: DecisionModel,
}

/// Placeholder for the full-screen card-index surface's model (spec §0.6:
/// order/tech/site/achievement grids, dim + location annotations, selection
/// numbers). **T5 designs and fills this** — it is deliberately not designed
/// here, so that T0 does not guess at a surface whose input grammar has not
/// been built yet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexModel;

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

/// The board region: one [`TileModel`] per area, in `AreaId` order.
pub fn build_board(state: &GameState, rules: &Rules) -> BoardModel {
    BoardModel {
        player_count: state.config.player_count,
        tiles: state
            .board_layout
            .areas
            .iter()
            .map(|(&area, cfg)| tile_from(state, rules, area, cfg))
            .collect(),
    }
}

/// One tile, or `None` for an area that is not on this board. An `Option`
/// rather than a defaulted tile: every field of a fabricated one would be a
/// lie about the board, and a display model is no place to panic over a stale
/// id either.
pub fn build_tile(state: &GameState, rules: &Rules, area: AreaId) -> Option<TileModel> {
    state.board_layout.areas.get(&area).map(|cfg| tile_from(state, rules, area, cfg))
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
        // T4 feeds the ◈ mark from the engine's legal list.
        reachable: false,
    }
}

/// The printed index of a runtime building site, by unique content match
/// against `Rules::building_sites` — see [`SiteModel::index`] on why the
/// runtime card cannot simply carry it. The 24 standard cards are pairwise
/// distinct in printed content, so the match is unique on a real board; a
/// registry with duplicates (or none, as in `Rules::empty_for_tests`) yields
/// `None` rather than an arbitrary first hit.
fn printed_site_index(site: &BuildingSite, rules: &Rules) -> Option<u8> {
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
/// resolved; a hidden panel carries the count and no cards at all.
pub fn build_hand(state: &GameState, rules: &Rules, seat: PlayerId, hidden: bool) -> HandPanel {
    let p = state.player(seat);
    HandPanel {
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
    }
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

    DecisionModel {
        turn_number: state.turn_number,
        seat: state.current_player,
        phase_chain: PHASE_CHAIN.to_vec(),
        current_phase: phase_chain_index(state.turn_phase),
        deciding,
        options: legal
            .iter()
            .enumerate()
            .map(|(number, mv)| OptionModel {
                number,
                mv: mv.clone(),
                fee: option_fee(state, rules, mv),
            })
            .collect(),
        // T4 derives the near-miss rows from the engine's list complement.
        not_offered: Vec::new(),
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

/// The President fee an option carries, priced by the ENGINE.
///
/// `MovePresident` reads `route_fee_vector`, whose `path` already ends at the
/// destination — the same slice `apply_movement` prices, passed unchanged so
/// the last hop is never double-counted. `PlacePresident` reads
/// `calculate_place_president_fee`, which returns a total rather than an
/// attribution; the engine's rule IS 1 Yen to each opponent President owner
/// standing on the destination, so listing exactly those owners at 1 each sums
/// to the engine's total by construction — and the Stagecoach waiver, which
/// zeroes that total, yields no recipients at all.
///
/// The wildcard arm is deliberate rather than the exhaustive match this repo
/// prefers elsewhere: a President fee is a President-MOVEMENT concept, these
/// two moves are the only ones the engine prices, and every other variant
/// would have to be listed as `None` for no gain.
fn option_fee(state: &GameState, rules: &Rules, mv: &Move) -> Option<FeeModel> {
    match mv {
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

/// A whole frame: the board, the dashboard in display order, and the decision
/// zone. `seat` is the seat whose own hand the dashboard shows.
pub fn build_frame(
    state: &GameState,
    rules: &Rules,
    seats: &[SeatFacts],
    seat: PlayerId,
    legal: &[Move],
) -> FrameModel {
    FrameModel {
        board: build_board(state, rules),
        dashboard: vec![
            Panel::Players(build_players(state, rules, seats)),
            Panel::Tracks(build_tracks(state)),
            Panel::Achievements(build_achievements(state)),
            Panel::Hand(build_hand(state, rules, seat, SeatFacts::for_seat(seats, seat).hand_hidden)),
        ],
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
        let board = build_board(&state, &rules);
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
        for tile in build_board(&state, &rules).tiles {
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

        let board = build_board(&state, &rules);
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
        let from_board = build_board(&state, &rules)
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
        let board = build_board(&state, &rules);
        let tile = board.tiles.iter().find(|t| t.area == here).expect("A00");
        assert_eq!(tile.presidents, vec![PlayerId::new(0), PlayerId::new(1)]);
    }

    /// The Canal holds no building site (and no 5-Power token), so its site
    /// model is `None` rather than a fabricated empty card.
    #[test]
    fn the_canal_tile_has_no_building_site() {
        let (state, rules) = state_np(4);
        let board = build_board(&state, &rules);
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
            assert_eq!(opt.number, i);
            assert_eq!(opt.mv, legal[i]);
        }
        assert!(decision.not_offered.is_empty(), "not-offered rows are T4's; T0 populates none");
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
        let panel = build_hand(&state, &rules, seat, false);
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
        let panel = build_hand(&state, &rules, PlayerId::new(0), true);
        assert!(panel.hidden);
        assert_eq!(panel.card_count, 1, "the count still reads");
        assert!(panel.cards.is_empty(), "a masked hand must carry no card data");
    }

    #[test]
    fn the_frame_assembles_the_board_and_the_four_dashboard_panels() {
        let (state, rules) = state_np(2);
        let legal = get_legal_moves(&state, &rules);
        let frame = build_frame(&state, &rules, &SeatFacts::all_open(2), PlayerId::new(0), &legal);

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
        let frame = build_frame(&state, &rules, &seats, PlayerId::new(0), &[]);
        let Some(Panel::Hand(hand)) = frame.dashboard.last() else {
            panic!("the hand panel closes the dashboard");
        };
        assert!(hand.hidden && hand.cards.is_empty() && hand.card_count == 1);
    }
}
