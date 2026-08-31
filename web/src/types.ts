// The wire shapes, hand-written for now.
//
// These mirror `yokohama-web`'s `ClientState` and `yokohama-cli`'s
// `screen_model`. They are hand-written deliberately and TEMPORARILY: the plan
// rules for generating them with `ts-rs`, but doing so requires `TS` derives on
// the engine's own types too (a `TileModel` holds an `AreaType`, an
// `OptionModel` holds a `Move`), which reaches into `yokohama-core`. That is a
// decision the owner has not been asked yet, so nothing here pretends to be
// generated.
//
// TWO TAGGING CONVENTIONS ARE IN PLAY and mixing them up is the mistake to
// avoid. Everything built by `screen_model` or `yokohama-core` uses serde's
// DEFAULT external tagging — a unit variant is a bare string (`"Placement"`), a
// newtype variant is a single-key object (`{"Copper": 2}`), a struct variant is
// `{"Choice": {...}}`. `yokohama-web`'s own `PromptState` is the exception: it
// carries `#[serde(tag = "kind")]`, so it is INTERNALLY tagged and reads
// `{"kind": "retreat", ...}`.
//
// Fields are never omitted. Nothing in the surface carries
// `skip_serializing_if`, so every field of every struct is present in every
// payload, zeros included — which is why nothing below is optional unless the
// Rust side is an `Option`.

export type PlayerId = number;
export type AreaId = number;
export type OrderCardId = number;
export type TechCardId = number;

/** `PlayerId::DUMMY`. The 2P neutral pawn: a real occupant that COMPETES in the
 *  church and customs majorities, excluded from the end-game pawn count, and
 *  never a seat. Check for it before indexing anything by seat. */
export const DUMMY_SEAT = 255;
export const isDummy = (seat: PlayerId | null): boolean => seat === DUMMY_SEAT;

export type PlayerColor = "Red" | "Blue" | "Yellow" | "Green";
export type Country = "English" | "American" | "French" | "German" | "Dutch";
export type PortZone = "Low" | "Mid" | "High";

export type AreaType =
  | "CopperMine" | "SilkMill" | "TeaPlantation" | "FishingGround"
  | "Bank" | "PortA" | "PortB" | "LaboratoryA" | "LaboratoryB"
  | "EmploymentAgency" | "Chinatown" | "Church" | "Customs" | "Canal";

export type TurnPhase =
  | "AdditionalActionFirst" | "Placement" | "Movement" | "AreaAction"
  | "FivePowerBonus" | "Building" | "Recovery" | "AdditionalActionSecond" | "TurnEnd";

export type AchievementType =
  | "Copper5" | "Silk6" | "Tea7" | "Seafood7"
  | "Money10" | "Tokens5" | "Orders5" | "Technologies4"
  | "Production4" | "Commercial4" | "Prod3Comm2" | "Prod2Comm3";
export type AchievementCategory = "A" | "B" | "C";

/** A newtype enum: one key, whose value is the printed amount. `Copper(2)` and
 *  a re-priced `Copper(3)` are different faces, so the art key spells both. */
export type FivePowerEffect =
  | { ImportedGoods: number } | { Copper: number } | { Silk: number }
  | { Tea: number } | { Seafood: number } | { Money: number };

/** Not an `Option` — a president is always somewhere. */
export type PresidentLocation = "InHand" | { OnBoard: AreaId };

export interface SpaceReward {
  vp: number; yen: number; seafood: number; tea: number;
  silk: number; copper: number; any_good: number;
  movement: number; warehouse_tokens: number;
}

export interface SiteSlotModel { built_by: PlayerId | null; reward: SpaceReward }

export interface SiteModel {
  /** The 0-based index into the printed site registry — the same number the
   *  CLI's tile prints as `Site n`, and the art join key. `null` when no
   *  printed card matches, or when more than one does; then there is no art. */
  index: number | null;
  /** The four spaces in printed TL, TR, ML, MR order. */
  slots: SiteSlotModel[];
  trading_house_vp: number;
  trading_house_owner: PlayerId | null;
}

export interface TileModel {
  area: AreaId;
  area_type: AreaType;
  /** `[row, col]` — row 0 is the widest, farthest row. */
  grid_position: [number, number];
  /** `null` when the token has been claimed, and on the Canal. */
  token: FivePowerEffect | null;
  /** `null` ONLY on the Canal. */
  site: SiteModel | null;
  presidents: PlayerId[];
  has_station: boolean;
  /** One entry per seat, zeros included. */
  assistants: [PlayerId, number][];
  /** The engine offered this tile as a Movement destination. Set only by the
   *  board builder, which is the one that sees the legal list. */
  reachable: boolean;
  /** Symmetric, on the PAIR members — never on the Canal tile itself. */
  canal_crossings: AreaId[];
}

export interface BoardModel { player_count: number; tiles: TileModel[] }

export interface GoodsModel {
  copper: number; silk: number; tea: number; seafood: number; imports: number;
}

/** `locked()` and `total()` are METHODS in Rust and are not on the wire. */
export interface AssistantZones {
  in_hand: number; on_board: number; in_warehouse: number;
  on_achievements: number; on_church: number; on_customs: number;
}
export const locked = (a: AssistantZones): number =>
  a.on_achievements + a.on_church + a.on_customs;
export const totalAssistants = (a: AssistantZones): number =>
  a.in_hand + a.on_board + a.in_warehouse + locked(a);

export type TechEffect =
  | "Tram" | "Stagecoach" | "PostalSystem" | "Station" | "Gaslight"
  | "ElectricalLight" | "MiningTechnology" | "SpinningMill" | "Ball"
  | "LanguageSchool" | "PatentSystem" | "LetterPressPrinting" | "Winery"
  | "Brickyard" | "SystemOfStock" | "University" | "Newspaper" | "Telegram"
  | "Telephone" | "Exposition";

export interface TechCard {
  id: TechCardId;
  name: string;
  country: Country;
  industry_cost: number;
  /** The key into `Labels.tech_effects`. */
  effect: TechEffect;
}
export interface TechModel { id: TechCardId; printed: TechCard | null }

export interface PlayerModel {
  seat: PlayerId;
  color: PlayerColor;
  money: number;
  victory_points: number;
  goods: GoodsModel;
  assistants: AssistantZones;
  shophouses: { in_hand: number; in_warehouse: number; built: number };
  trading_houses: { in_hand: number; in_warehouse: number; built: number; next_take_cost: number };
  technologies: TechModel[];
  /** Face-up: 1 VP each at scoring. */
  mercenaries_unused: number;
  /** Spent, face-down. */
  mercenaries_used: number;
  five_power_tokens: number;
  president: PresidentLocation;
  /** A count only; identities live in `HandPanel`. */
  orders_in_hand: number;
  /** Public on every seat, in completion order. */
  orders_filled: OrderCardId[];
  is_current: boolean;
  is_bot: boolean;
  hand_hidden: boolean;
}

/** `occupant` may be `DUMMY_SEAT`. */
export interface ChurchSpaceModel {
  faith: number; vp: number; movement_rider: number; occupant: PlayerId | null;
}
export interface CustomsSpaceModel {
  space: number; import_cost: number; vp: number;
  warehouse_rider: number; yen_rider: number; occupant: PlayerId | null;
}
export interface MercenarySupply {
  english: number; american: number; french: number; german: number; dutch: number;
}
export interface TracksPanel {
  /** Faith 4..11, ascending. */
  church: ChurchSpaceModel[];
  /** REAL pawns; the dummy is excluded. */
  church_filled: number;
  /** Spaces 1..7, ascending. */
  customs: CustomsSpaceModel[];
  customs_filled: number;
  /** The end-game pawn threshold for this player count. */
  trigger_at: number;
  station: AreaId | null;
  mercenary_supply: MercenarySupply;
}

export interface AchievementModel {
  achievement_type: AchievementType;
  category: AchievementCategory;
  first_place_vp: number;
  subsequent_vp: number;
  /** In claim order: the FIRST scores `first_place_vp`, every later one
   *  `subsequent_vp`. More than one player may claim the same card. */
  claimed_by: PlayerId[];
  /** `[seat, current, needed]`. `current >= needed` is "ready by the numbers",
   *  not "claimable" — a claim also needs an assistant in hand. */
  progress: [PlayerId, number, number][];
}

export interface TradeGoodRequirement {
  copper: number; silk: number; tea: number; seafood: number;
}
export interface OrderRewards {
  victory_points: number; money: number; imported_goods: number;
  copper: number; silk: number; tea: number; seafood: number;
  movement: number; any_trade_good: number;
  /** Items moved from the warehouse to hand. */
  warehouse_tokens: number;
}
export interface OrderCard {
  id: OrderCardId;
  country: Country;
  requirements: TradeGoodRequirement;
  rewards: OrderRewards;
}

export interface HandCardModel {
  id: OrderCardId;
  printed: OrderCard | null;
  is_starting_order: boolean;
}
export interface HandPanel {
  seat: PlayerId;
  hidden: boolean;
  /** EMPTY whenever `hidden` — the mask is a property of the MODEL, so a client
   *  that ignores the flag still cannot leak identities. */
  cards: HandCardModel[];
  /** Truthful whether masked or not. */
  card_count: number;
}

/** Externally tagged: exactly one key. `Hand` is ABSENT, not empty, for a seat
 *  the state does not hold. */
export type Panel =
  | { Players: { players: PlayerModel[] } }
  | { Tracks: TracksPanel }
  | { Achievements: { cards: AchievementModel[] } }
  | { Hand: HandPanel };

export type PendingChoice =
  | "AnyGood" | "WarehouseTake" | "AssistantMove" | "RecoveryLeave" | "StationPlace"
  | { ChurchSpace: { total_faith: number } }
  | { CustomsSpace: { cost: number } };

export type Deciding =
  | { Phase: TurnPhase }
  | { Choice: { head: PendingChoice; queued: number } }
  | "GameOver";

export interface FeeModel {
  total: number;
  /** `[recipient seat, yen]` — the president owners paid. */
  to_players: [PlayerId, number][];
  /** Canal crossings, paid to the BANK rather than to a player. */
  canal_tolls: number;
}

/** `fee` is `null` on every move that cannot carry one, and a ZERO fee is still
 *  an object on President moves — render `total === 0` as free, not unpriced. */
export interface OptionModel { number: number; mv: unknown; fee: FeeModel | null }

export type NotOfferedReason =
  | "StartedThereThisTurn" | "CanalIsNeverAStop" | "NoAssistantThere";

export interface DecisionModel {
  turn_number: number;
  seat: PlayerId;
  phase_chain: TurnPhase[];
  current_phase: number;
  deciding: Deciding;
  /** One per LEGAL MOVE — **not** the numbered list. See `PromptState`. */
  options: OptionModel[];
  /** Capped at three rows. */
  not_offered: { area: AreaId; reason: NotOfferedReason }[];
  /** How many further near-misses the cap cut. Surface it: three rows are
   *  indistinguishable from three-of-five otherwise. */
  not_offered_suppressed: number;
  /** `false` means you are watching; `options` is then empty BY CONSTRUCTION,
   *  which is what distinguishes it from acting with no legal moves. */
  viewer_is_deciding: boolean;
}

export interface FrameModel {
  /** `seed` here is a `u64` in a JSON number and is NOT SAFE to read — use
   *  `ClientState.seed`. */
  title: { mode: "Sim" | "Live"; seed: number; autosaved: boolean };
  board: BoardModel;
  dashboard: Panel[];
  decision: DecisionModel;
}

export interface OptionRow {
  number: number;
  words: string;
  opens_picker: boolean;
  /** What this row costs. `null` means the move cannot carry a fee at all; a
   *  `total` of 0 means FREE, which is not the same thing — every President
   *  move carries a zero fee when it is free, and the terminal prints `free`
   *  there because a player comparing destinations needs a price, not a gap. */
  fee: FeeModel | null;
}

/** INTERNALLY tagged — the one exception on the wire. */
export type PromptState =
  | { kind: "options"; rows: OptionRow[] }
  | { kind: "pick"; title: string; rows: OptionRow[] }
  | { kind: "retreat"; area: AreaId; area_label: string; max: number; answered: number[]; remaining: number }
  | { kind: "place_a"; selected: AreaId[]; max_areas: number; fee: FeeModel | null }
  | { kind: "place_b"; areas: AreaId[]; selected: AreaId | null; fee: FeeModel | null }
  | { kind: "reveal" }
  | { kind: "game_over" }
  /** The session is locked after a driver or engine fault. Nothing this client
   *  can send is accepted, and the three lines that would be are refused by the
   *  API's own grammar fence. */
  | { kind: "locked"; reason: string }
  /** Someone else is to move; this payload carries no answerable list. The
   *  frame's decision zone is scoped the same way, so the two agree. */
  | { kind: "watching"; to_move: PlayerId };

export interface ClientState {
  id: string;
  mode: "sim" | "live";
  /** The setup seed, as a decimal STRING. A `u64` does not fit a JS number. */
  seed: string;
  seat: PlayerId;
  to_move: PlayerId;
  your_turn: boolean;
  frame: FrameModel;
  prompt: PromptState;
  messages: string[];
  over: boolean;
  /** The CLI's own end-trigger line, already worded. */
  end_triggers: string;
  deck_remaining: number;
  end_game_triggered: boolean;
  scores: { seat: PlayerId; total: number }[] | null;
  warnings: string[];
}

export interface Labels {
  area_types: Record<string, string>;
  phases: Record<string, string>;
  phase_chain: string[];
  tech_effects: Record<string, string>;
  achievements: Record<string, string>;
}

/** `GET /api/health` — the two compatibility keys the ENGINE uses, not the
 *  server version. A client that caches card art or option shapes checks
 *  these: a save stops resuming across a `semantics_rev` bump by design, and
 *  `rules_hash` moves whenever card, site, token or board data does. */
export interface Health {
  server_version: string;
  semantics_rev: number;
  rules_hash: string;
}

/** Every refusal on this API, in one shape. `api.ts` reads `error` off it and
 *  throws an `ApiError` carrying the sentence — so a second error shape would
 *  be reported to the player as a bare status code. */
export interface ApiErrorBody {
  error: string;
}

export interface Preview {
  seed: string;
  deals: { seat: PlayerId; cards: OrderCard[] }[];
}

// ---- the read-only surfaces ---------------------------------------------

export interface PortSlotModel {
  slot: number;
  zone: PortZone;
  /** The lowest action power that reaches this zone — the engine's own gate. */
  min_power: number;
  card: OrderCardId | null;
  printed: OrderCard | null;
}
export interface PortsPanel {
  boards: { board_b: boolean; slots: PortSlotModel[] }[];
  deck_remaining: number;
}

export interface LabSlotModel {
  slot: number;
  /** The printed position surcharge: 0,0,1,1,2,2. */
  surcharge: number;
  card: TechCardId | null;
  printed: TechCard | null;
}
export interface LabsPanel {
  boards: { board_b: boolean; slots: LabSlotModel[] }[];
  deck_remaining: number;
}

export type ActionYield =
  | { Produce: { good: "Copper" | "Silk" | "Tea" | "Seafood"; amount: number } }
  | { Money: { yen: number } }
  | { PortTake: { zones: PortZone[]; vp: number } }
  | { Industry: { value: number } }
  | { Faith: { from_power: number; with_all_payment_kinds: number } }
  | { CustomsPayment: { max_imports: number } }
  | { WarehouseTakes: { items: number } }
  | { Exchanges: { count: number } };

export interface AreaCard {
  tile: TileModel;
  /** One rung per power 1..5 — EMPTY on the Canal. */
  ladder: { power: number; pays: ActionYield }[];
  companion: "Ports" | "Labs" | "Tracks" | null;
}
export interface AreasPanel { rows: AreaCard[][] }

export interface LogPanel {
  /** Every event in the record, including ones this panel does not carry. */
  total: number;
  entries: { index: number; text: string }[];
}

// ---- card art ------------------------------------------------------------

export interface ArtEntry { file: string; w: number; h: number }

/** Absent when the server has no cropped art; the client draws without it. */
export interface ArtManifest {
  /** Keyed by `OrderCardId` as a decimal string. */
  orders: Record<string, ArtEntry>;
  /** Keyed by the printed TITLE. */
  techs: Record<string, ArtEntry>;
  /** Keyed by `SiteModel.index` as a decimal string. */
  sites: Record<string, ArtEntry>;
  achievements: Record<string, ArtEntry>;
  areas: Record<string, ArtEntry>;
  /** Keyed `"<variant>_<payload>"`, e.g. `"Copper_2"`. */
  five_power: Record<string, ArtEntry>;
  /** Keyed by `Country`. */
  mercenaries: Record<string, ArtEntry>;
  backs: Record<string, ArtEntry>;
  tokens: Record<string, ArtEntry>;
}
