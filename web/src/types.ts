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
// Engine enums arrive as serde's externally-tagged JSON: a unit variant is a
// bare string (`"Placement"`), a data variant is a single-key object
// (`{"MovePresident": {...}}`). Anything this client does not need to read
// structurally is typed `unknown` rather than guessed at — a wrong type here
// would be a lie the compiler enforces.

export type PlayerId = number;
export type AreaId = number;
export type OrderCardId = number;

export type PlayerColor = "Red" | "Blue" | "Yellow" | "Green";

export interface SpaceReward {
  vp: number; yen: number; seafood: number; tea: number;
  silk: number; copper: number; any_good: number;
  movement: number; warehouse_tokens: number;
}

export interface SiteSlotModel { built_by: PlayerId | null; reward: SpaceReward }

export interface SiteModel {
  index: number | null;
  slots: SiteSlotModel[];
  trading_house_vp: number;
  trading_house_owner: PlayerId | null;
}

export interface TileModel {
  area: AreaId;
  /** serde name of the `AreaType` variant, e.g. `"CopperMine"`. */
  area_type: string;
  /** `[row, col]` — row 0 is the widest, farthest row. */
  grid_position: [number, number];
  token: unknown | null;
  site: SiteModel | null;
  presidents: PlayerId[];
  has_station: boolean;
  /** `[seat, count]`, zeros included. */
  assistants: [PlayerId, number][];
  /** The engine offered this tile as a Movement destination. */
  reachable: boolean;
  canal_crossings: AreaId[];
}

export interface BoardModel { player_count: number; tiles: TileModel[] }

export interface GoodsModel {
  copper: number; silk: number; tea: number; seafood: number; imports: number;
}

export interface AssistantZones {
  in_hand: number; on_board: number; in_warehouse: number;
  on_achievements: number; on_church: number; on_customs: number;
}

export interface PlayerModel {
  seat: PlayerId;
  color: PlayerColor;
  money: number;
  victory_points: number;
  goods: GoodsModel;
  assistants: AssistantZones;
  shophouses: { in_hand: number; in_warehouse: number; built: number };
  trading_houses: { in_hand: number; in_warehouse: number; built: number; next_take_cost: number };
  technologies: { id: number; printed: { name: string } | null }[];
  mercenaries_unused: number;
  mercenaries_used: number;
  five_power_tokens: number;
  president: unknown;
  orders_in_hand: number;
  orders_filled: OrderCardId[];
  is_current: boolean;
  is_bot: boolean;
  hand_hidden: boolean;
}

export interface ChurchSpaceModel {
  faith: number; vp: number; movement_rider: number; occupant: PlayerId | null;
}
export interface CustomsSpaceModel {
  space: number; import_cost: number; vp: number;
  warehouse_rider: number; yen_rider: number; occupant: PlayerId | null;
}
export interface TracksPanel {
  church: ChurchSpaceModel[]; church_filled: number;
  customs: CustomsSpaceModel[]; customs_filled: number;
  trigger_at: number; station: AreaId | null; mercenary_supply: unknown;
}

export interface AchievementModel {
  achievement_type: string;
  category: string;
  first_place_vp: number;
  subsequent_vp: number;
  claimed_by: PlayerId[];
  progress: [PlayerId, number, number][];
}

export interface OrderCard {
  id: OrderCardId;
  country: string;
  requirements: { copper?: number; silk?: number; tea?: number; seafood?: number };
  rewards: Record<string, number>;
}

export interface HandPanel {
  seat: PlayerId;
  hidden: boolean;
  cards: { id: OrderCardId; printed: OrderCard | null; is_starting_order: boolean }[];
  card_count: number;
}

/** `Panel` is an externally-tagged enum: exactly one key. */
export type Panel =
  | { Players: { players: PlayerModel[] } }
  | { Tracks: TracksPanel }
  | { Achievements: { cards: AchievementModel[] } }
  | { Hand: HandPanel };

export interface DecisionModel {
  turn_number: number;
  seat: PlayerId;
  /** serde names of the nine `TurnPhase` variants, in order. */
  phase_chain: string[];
  current_phase: number;
  deciding: unknown;
  /** One per LEGAL MOVE — **not** the numbered list. See `PromptState`. */
  options: unknown[];
  not_offered: { area: AreaId; reason: string }[];
  not_offered_suppressed: number;
  viewer_is_deciding: boolean;
}

export interface FrameModel {
  title: { mode: "Sim" | "Live"; seed: number; autosaved: boolean };
  board: BoardModel;
  dashboard: Panel[];
  decision: DecisionModel;
}

export interface OptionRow { number: number; words: string; opens_picker: boolean }

export type PromptState =
  | { kind: "options"; rows: OptionRow[] }
  | { kind: "pick"; title: string; rows: OptionRow[] }
  | { kind: "retreat"; area: AreaId; area_label: string; max: number; answered: number[]; remaining: number }
  | { kind: "place_a"; selected: AreaId[]; max_areas: number }
  | { kind: "place_b"; areas: AreaId[]; selected: AreaId | null }
  | { kind: "reveal" }
  | { kind: "game_over" };

export interface ClientState {
  id: string;
  mode: "sim" | "live";
  seat: PlayerId;
  to_move: PlayerId;
  your_turn: boolean;
  frame: FrameModel;
  prompt: PromptState;
  messages: string[];
  over: boolean;
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

export interface Preview {
  seed: number;
  deals: { seat: PlayerId; cards: OrderCard[] }[];
}
