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
  /** No `seed`. The server passes `TitleModel::seed: None` on every frame it
   *  serves — the setup seed regenerates both decks, the boxed pile, the board
   *  and every deal, so a payload carrying it hands over the whole hidden set.
   *
   *  **Nor does anything else on this wire.** `POST …/preview` was the last
   *  payload that carried one, because the setup flow round-tripped the value
   *  back into `create`; the flow is create-then-deal now, the seed lives only
   *  in the table document on the server, and the fence that says so —
   *  `no_committed_payload_carries_the_setup_seed` — has no exemption left. */
  title: { mode: "Sim" | "Live"; autosaved: boolean };
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
  /** The seat this payload was built FOR — whose hand is unmasked.
   *
   *  **`null` for a spectator (M6.3)**: a watcher holds no seat, every hand on
   *  the board is hidden from them, and there is no Hand panel in the
   *  dashboard at all. A seated payload is unchanged. */
  seat: PlayerId | null;
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
  /** The record's length; see the server's doc. */
  events: number;
  /** Every bot seat the RECORD names, labelled (M6.5c2, R-6.5c2.10) — empty
   *  for an all-human game. The record's, not the resident session's. */
  bots: SeatBot[];
  /** The POSITION's legal moves the server folded or capped rather than
   *  listed (security S1b): the retreat family's members beyond its
   *  recover-nothing one — a count of public on-board assistants; a hot-seat
   *  table in the engine's worst-case state offered 442,368 of them, 58 MB a
   *  frame — and, on the DECIDING viewer's frame alone, anything past the
   *  server's cap. Counted for every viewer of the position, so a frame for a
   *  seat that is not deciding (whose `frame.decision.options` is empty)
   *  still says what was folded. The numbered rows are the prompt's and were
   *  always the REPL's grouped entries; the guided retreat
   *  (`kind: "retreat"`) is how a retreat is chosen. `0` on every frame
   *  outside a folded Movement decision. */
  options_omitted: number;
}

/** One bot seat of a game: which seat, and the bot's short label (`V4`). */
export interface SeatBot {
  seat: PlayerId;
  label: string;
}

/** A table's bot (M6.5c2, R-6.5c2.10): its short label, and whether THIS
 *  build runs it. An undealt table's is the bot `finalize` will deal, so it
 *  runs here; a dealt one's is its record's. `runs_here: false` is a game
 *  played against another version — finished, it opens; not finished, every
 *  game route refuses it. */
export interface TableBot {
  label: string;
  runs_here: boolean;
}

/** `web/src/wire/production_bot.json` (M6.5c2, spec §2.4): the bot this build
 *  deals, emitted by `wire_fixtures` and imported at build time by the screens
 *  that name it — the door asks the server nothing. */
export interface ProductionBot {
  /** The controller name its records carry (`v4-step9-draft1`). */
  identity: string;
  label: string;
}

/** The change SIGNAL, from `GET /api/games/{id}/events` — what a poll asks for
 *  while it is somebody else's turn (room plan R-P9).
 *
 *  **Deliberately not a frame.** The client learns that this game now has `N`
 *  events and seat `S` to move, and fetches a frame only when that moved — so
 *  the every-few-seconds request costs a few bytes rather than a whole board.
 *
 *  `status` is what lets ONE poll serve two screens: `dealing` is the setup
 *  screen waiting for the other seats to choose (`events` 0, `to_move` null),
 *  and the flip to `playing` is the client's cue to open the game. `your_turn`
 *  is answered per caller and is false once the game is `over`, so a finished
 *  game never reads as waiting for the reader. */
export interface Events {
  events: number;
  to_move: PlayerId | null;
  /** `open` joined the union in M6.3 (R-3.3): a table whose seats are still
   *  filling has no record, and the open-table screen watches this endpoint
   *  for its own start exactly as the setup screen watches for `dealing`. */
  status: "open" | "dealing" | "playing" | "over";
  your_turn: boolean;
  /** The table's newest message number, 0 with none (M6.4, R-6.4.7) — the
   *  fifth field, and the one that makes a message a count on the signal and a
   *  fetch for the text. It is the document's own `last`, so a trim never walks
   *  it backwards past a cursor the client is holding. */
  chat: number;
  /** The table's chat REVISION (M6.5c2 cycle D, R-D4): +1 per message AND per
   *  message deleted, so `chat` is a count for the text and this a revision
   *  for the shape — `chat_rev - chat` moves exactly when a message a reader
   *  may already hold was deleted. */
  chat_rev: number;
}

/** The house frame (M6.5c2 cycle E): what the lobby's stream or socket
 *  delivers as each `house` event's data, `signal::house_frame`'s own bytes.
 *  `seq` is the frame's own sequence and names nothing stored; `chat` and
 *  `rev` are the house chat document's `last` and `rev`. */
export interface HouseFrame {
  seq: number;
  chat: number;
  rev: number;
}

/** One historical position, from `GET /api/games/{id}/at/{k}` — the M4 replay
 *  payload.
 *
 *  **Deliberately NOT a `ClientState`.** Six of that type's fields are built
 *  from the LIVE state and would be present-tense lies at a past position, and
 *  three more have no honest historical value at all. What is missing here is
 *  the point:
 *
 *  - no `your_turn` — it is the one field that is not a function of the game
 *    state alone (it compares against a seat that lives on the server's REPL),
 *    and it is meaningless for a position nobody is playing;
 *  - no `prompt` — an in-flight picker is not in the record and cannot be;
 *  - no `warnings` — a load-time session fact, not a property of this position;
 *  - no `messages` — nothing was just applied.
 *
 *  `frame` is the ordinary {@link FrameModel}, so `Board` and `Dashboard` draw a
 *  historical position through exactly the same components. Pass no
 *  `onTileClick` and every tile is an inert `<div>`, which is what a read-only
 *  position should be. */
export interface HistoricalFrame {
  /** `k` — events APPLIED, not an event index. */
  index: number;
  /** The record's length. The track has `total + 1` stops, and the last one is
   *  the present: a real position with no event to name. */
  total: number;
  /** Whose view this is. **Chosen server-side, and always one of the CALLER's
   *  own seats** — the seat to move when the caller holds it, otherwise their
   *  lowest (R-4.2). It never names a seat somebody else holds, which is what
   *  stops a sweep over every `k` from harvesting another player's whole hand
   *  history in one pass. At a hot seat, where one person holds several, it
   *  does follow the mover among those.
   *
   *  **`null` for a spectator (M6.3)**: a watcher holds no seat, so a sweep
   *  over every `k` harvests nothing at any stop. */
  seat: PlayerId | null;
  /** Whose turn it was at `k`. */
  to_move: PlayerId;
  frame: FrameModel;
  over: boolean;
  /** The end-trigger line as it read AT `k`, worded by the CLI. */
  end_triggers: string;
  deck_remaining: number;
  end_game_triggered: boolean;
  scores: { seat: PlayerId; total: number }[] | null;
  /** The port and laboratory offers AT `k` — face up at the time, and
   *  properties of the position, which is M4's Q5 applied to keep this frame
   *  narrow: the fields it dropped were present-tense lies at a past position,
   *  and these are not. The turn playback needs them so a port take shows the
   *  card leaving. */
  ports: PortsPanel;
  labs: LabsPanel;
  /** The POSITION's legal moves the server folded or capped rather than
   *  listed (security S1b): the retreat family's members beyond its
   *  recover-nothing one — a count of public on-board assistants; a hot-seat
   *  table in the engine's worst-case state offered 442,368 of them, 58 MB a
   *  frame — and, on the DECIDING viewer's frame alone, anything past the
   *  server's cap. Counted for every viewer of the position, so a frame for a
   *  seat that is not deciding (whose `frame.decision.options` is empty)
   *  still says what was folded. The numbered rows are the prompt's and were
   *  always the REPL's grouped entries; the guided retreat
   *  (`kind: "retreat"`) is how a retreat is chosen. `0` at every position
   *  outside a folded Movement decision. */
  options_omitted: number;
}

export interface Labels {
  area_types: Record<string, string>;
  phases: Record<string, string>;
  phase_chain: string[];
  tech_effects: Record<string, string>;
  achievements: Record<string, string>;
  /** The evaluator's 34 features in words, keyed by the stable snake_case
   *  token the explanation payload carries. The token stays the identity — it
   *  is what greps against `eval_probe` and every recorded figure; this is
   *  presentation only. */
  features: Record<string, string>;
}

/** `GET /api/health` — the two compatibility keys the ENGINE uses, not the
 *  server version. A client that caches card art or option shapes checks
 *  these: a save stops resuming across a `semantics_rev` bump by design, and
 *  `rules_hash` moves whenever card, site, token or board data does. */
export interface Health {
  server_version: string;
  semantics_rev: number;
  rules_hash: string;
  /** Which identity source this deployment reads, by name. */
  identity: string;
  /** Which change transport `usePolled` reads: the stream, or — the Worker's,
   *  M6.5c — the socket. A deployment fact, not a preference. The held request
   *  M6.3 built is retired (M6.5c2 cycle D, R-D5), and no host reports it. */
  signal: "sse" | "ws";
  /** Which archive this deployment writes finished and abandoned games to
   *  (M6.5a): none, a directory beside the store, or (M6.5c) a bucket. Read at
   *  the three points spec §2.2 names since M6.5b2 — entering the multiplayer
   *  channel, a local game ending, and Abandon being clicked — and remembered
   *  with the time it was taken. */
  archive: "off" | "dir" | "r2";
  /** Whether the house chat answers (M6.4). Off, this client draws no lobby
   *  panel and asks for nothing. */
  lobby_chat: boolean;
  /** Messages retained per chat document; `null` when nothing is trimmed. */
  chat_keep: number | null;
  /** Characters per message. The input is sized from THIS and never from a
   *  copy here, which would drift the day the server's number changed. */
  chat_text: number;
  /** Whether this deployment offers multiplayer (M6.5b2). Off, the game, chat
   *  and stream routes are not served at all, so this is a fact about what
   *  EXISTS here and not a preference. The browser host reports `false`: it
   *  serves every game route, because a local game is played through them, and
   *  there is nobody else at the table. */
  multiplayer: boolean;
  /** How many days an idle table stands before it expires, and how many a
   *  table still waiting for players does (M6.5c2 cycle E, R-6.5c2.42): the
   *  Worker's two TTLs. `null` where no table expires — the NAS keeps no TTL,
   *  and the browser host has no tables to expire. */
  table_ttl_days: number | null;
  open_table_ttl_days: number | null;
}

/** Who the server thinks you are, and which header it reads to decide.
 *
 *  `GET /api/me` NEVER refuses: `user` is `null` when nobody is named, which is
 *  the answer that tells the client which screen to show. `mode` says whether
 *  signing in here is even meaningful — only `open` has a screen to offer,
 *  since the other two are set by an edge the browser cannot reach. */
export interface Me {
  mode: "open" | "cf-access" | "worker";
  user: string | null;
}

/** Every refusal on this API, in one shape. `api.ts` reads `error` off it and
 *  throws an `ApiError` carrying the sentence — so a second error shape would
 *  be reported to the player as a bare status code. */
export interface ApiErrorBody {
  error: string;
}

/** One dealt card, beside what the built-in bot would score it at. */
export interface DealCard {
  card: OrderCard;
  /** `greedy::score_starting_order` — the printed economics, hand-set and
   *  unmeasured by its own doc, so it is shown under the CLI's hedge and never
   *  as a recommendation the engine stands behind. */
  keep_score: number;
}

// ---- the table: who sits where (M6.1) ------------------------------------

/** Who plays one seat, as THIS reader is handed it: a person, the built-in
 *  bot, or nobody yet.
 *
 *  **No address crosses but the reader's own typing** (M6.5c2 cycle D,
 *  R-6.5c2.35, R-D3). A human seat carries the display `name` its holder gave
 *  (`null` on a seat bound before names existed and on a seat whose holder
 *  gave none), `you` — whether the reader holds it — and `invited`: the
 *  address the table's CREATOR typed for it, served to the creator alone and
 *  only while the seat has no name. Everyone else reads an unnamed seat as
 *  "invited" (`seatName`). Two people who pick one name are told apart by their
 *  colour, as they are on the board.
 *
 *  `open` is an unfilled seat at an open table (M6.2): anyone signed in may
 *  take it through `api.join`. It never appears once the table has left
 *  `open` — the server refuses to read back a document where the two disagree. */
export type SeatView =
  | { kind: "human"; name: string | null; you: boolean; invited: string | null }
  | { kind: "bot" }
  | { kind: "open" };

/** A table is OPEN while it is still filling its seats, DEALING until every
 *  human seat has chosen a starting order, and PLAYING afterwards. Every route
 *  that needs a session answers 409 at the first two, carrying the status so
 *  this client knows which screen to draw rather than showing an error. */
export type TableStatus = "open" | "dealing" | "playing";

/** The table as a seated player sees it.
 *
 *  **Two things the server holds are not here**: the setup seed, which
 *  regenerates every deal and both decks, and the `keeps` — what the OTHER
 *  seats chose. `chosen` is what crosses instead: whether a seat has answered
 *  is public at any table; which card it took is not. */
export interface TableView {
  id: string;
  name: string | null;
  /** Whether the READER created this table — what draws the creator's own
   *  controls. A fact about the reader, like `can_watch`. */
  mine: boolean;
  /** The creator's seat NAME, or `null` when they hold no seat or gave no name
   *  — never their address. */
  creator: string | null;
  /** Unix SECONDS, not a formatted string — the lobby sorts on it. */
  created_at: number;
  players: number;
  colors: PlayerColor[];
  start_seat: number;
  seats: SeatView[];
  /** True for a bot seat from the start, and for a human seat once it has kept. */
  chosen: boolean[];
  /** The seats the CALLER holds. Empty for a caller not seated at an OPEN
   *  table (R-2.7); never empty at any other status, where a caller with no
   *  seat is a 403. */
  my_seats: PlayerId[];
  status: TableStatus;
  /** Present once playing, `null` while dealing. */
  to_move: PlayerId | null;
  over: boolean | null;
  /** Whether a password is needed to join. The salt and the hash stay on the
   *  server — a password handed to a browser is a password to attack offline. */
  locked: boolean;
  /** Who won, once `over`. `null` at every other moment, a table that is still
   *  open or dealing included. */
  winner: PlayerId | null;
  /** Whether THIS caller may watch: true for a seat holder, for anyone at an
   *  unlocked table, or for a caller on the table's unlocked list — someone
   *  who has presented the password through `POST …/{id}/unlock`
   *  (M6.3, R-3.5). A fact about the reader, so two people reading one table
   *  get two different answers. */
  can_watch: boolean;
  /** The table's bot, or `null`: no bot seat, or a table dealt before the
   *  field existed (its row says "the bot" with no version). */
  bot: TableBot | null;
  /** Whether the game's record holds any event, off the table's own mirror
   *  of it: `false` before the first move, and at a table whose mirror was
   *  last written before the field (cycle E, R-E5; ruling T5-a). A bit and
   *  never a count — a count would cost the server a write per move. What
   *  this client's rule for a LOCAL game reads, since that game's save is
   *  this client's own upload. */
  has_events: boolean;
  /** Whether the creator's Abandon would SAVE this table before deleting it
   *  — the server's own rule, the one its delete applies (cycle E, R-E5):
   *  what "Abandon and save" is worded from for a hosted table. The server
   *  answers `false` for a local one (its host keeps no archive). */
  saves: boolean;
}

/** What `GET …/deal` serves: the set table, and the CALLER's own two cards.
 *
 *  **`seat` and `cards` are null together**, and that is the "waiting for the
 *  others" screen rather than an error — every seat this caller holds has
 *  chosen, and the table has not finished dealing. */
export interface DealView {
  table: TableView;
  seat: PlayerId | null;
  cards: DealCard[] | null;
  /** The terminal's own `KEEP_SCORE_NOTE`, served rather than copied here, so
   *  the hedge beside the Keep score cannot drift from the CLI's. */
  keep_note: string;
  frame: HistoricalFrame;
}

/** The lobby. An object rather than a bare array, so a cursor or a count can be
 *  added without every reader changing shape — and `next` is the field that
 *  was. */
export interface GamesList {
  games: TableView[];
  /** The cursor for the NEXT page, or `null` when this is the last one.
   *  `<created_at>,<id>`: opaque to this client, which sends it straight back
   *  as `before`. */
  next: string | null;
}

/** The 409 a session route answers while its table is still open or dealing:
 *  the ordinary error shape, plus the one status this client BRANCHES on.
 *
 *  Narrowed to [`TableStatus`] rather than left as a string, so a screen that
 *  branches on it is checked against the three the server can send. */
export interface DealingErrorBody extends ApiErrorBody {
  status: TableStatus;
}

/** The two labels a foreign-bot refusal names (M6.5c2, R-6.5c2.10): the bot
 *  the game was played against, and the one this server runs. */
export interface ForeignBot {
  played: string;
  runs: string;
}

/** The 409 every game route answers for a game NOT over whose recorded bot
 *  this build does not run — the sentence, and the pair beside it that the
 *  game screen branches on rather than displaying a banner. A FINISHED game
 *  against another bot opens as any finished game does. */
export interface ForeignBotErrorBody extends ApiErrorBody {
  foreign_bot: ForeignBot;
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

// ---- the explanation surface (M3) ----------------------------------------
//
// `GET /api/games/{id}/suggest` and `GET /api/games/{id}/why`. Both answer 200
// with exactly one of `ok` / `refusal` non-null (`why`: one of `ok` /
// `withheld` / `refusal`, security S1a): a refusal is the ANSWER (the last
// move was a human's; the game is over), not a client error, and the browser
// renders the sentence.
//
// Every number here is `i64` on the server and arrives as a JSON number. That
// is safe and the seed is not, because a contribution is bounded by the weight
// table while a seed is drawn over the full `u64`.

/** One feature's contribution, as `eval::explain` computed it.
 *
 *  `feature` is the STABLE snake_case token (`Feature::name()`), not a display
 *  string — the same key the arena report and `eval_probe` use. Wording comes
 *  from `/api/labels`. `product` is `weight × (viewer_value − opponent_value)`
 *  and is carried rather than recomputed here: re-multiplying in the browser
 *  would be a second implementation of the identity this surface exists to show. */
export interface Term {
  feature: string;
  viewer_value: number;
  opponent_value: number;
  weight: number;
  product: number;
}

/** A finished game's re-derived standing, and then the whole of `total`. */
export interface Terminal {
  rank: number;
  seats: number;
  viewer_total: number;
  best_other_total: number;
  value: number;
}

/** A score, term by term.
 *
 *  **THE IDENTITY THE CHART MUST FOOT:**
 *
 *      total === sum(terms[].product) + (terminal ? terminal.value : 0)
 *
 *  `terms` carries ALL 34 features, zero-product ones included — not because
 *  the sum needs them (a zero term contributes zero) but because a reader can
 *  only trust "8 of 34" if all 34 are there. Do not filter server data out of
 *  the sum, and do not treat the count as a caption.
 *
 *  When `terminal` is non-null every product is 0 by design (reporting an
 *  interpolated weight on a finished game would be a lie about what the
 *  evaluator used), so a bar chart renders EMPTY against a nonzero total. Draw
 *  the standing instead. */
export interface Explanation {
  terms: Term[];
  terminal: Terminal | null;
  total: number;
  progress_ppm: number;
  /** How many of `terms` carry a nonzero product — the terminal prints the same
   *  count, so the two surfaces cannot disagree about it. */
  nonzero: number;
}

/** One root candidate, worded and priced. `margin` is `value − best`, so **0
 *  means tied** — the sense the terminal's runner-up label already states. */
export interface Ranked {
  text: string;
  value: number;
  margin: number;
  /** The candidate that was actually played. **Not `index === 0`**: the ranking
   *  is sorted best-first but the pick is one of the MAXIMISERS, and ties are
   *  broken by a keyed draw. Filter on this to get the runner-ups. */
  chosen: boolean;
}

/** One search counter. `name` is the engine's own — carried verbatim so this
 *  screen's output can be grepped with the arena report's vocabulary.
 *  `group` is `"work" | "limit" | "ungrouped"`. */
export interface Counter {
  name: string;
  value: number;
  group: string;
}

/** What a `why` reconstruction CLAIMS. Arithmetic with no claim attached to it
 *  is arithmetic about nothing, so this is not decoration. When
 *  `recorded_version` differs from `current_version` the move still reproduced
 *  — the account is this build's reading of a move an older one played, and the
 *  client says so. */
export interface Provenance {
  /** `name@vN` or `name@vN{k=v,…}` — `Contestant::echo()`, the ONE display form,
   *  so the browser and the terminal cannot spell the same bot differently. */
  contestant: string;
  evaluator_version: number;
  /** The log's printed 0-based event index. */
  index: number;
  seat: PlayerId;
  recorded_version: number | null;
  current_version: number;
}

/** A typed refusal. `message` is the CLI's own sentence, never re-authored.
 *  `kind` is one of the twelve `why` tags, or one of `suggest`'s four
 *  (`live_mode`, `game_over`, `bot_seat`, `locked`, plus `search` for an agent
 *  error surfaced verbatim). */
export interface RefusalRow {
  kind: string;
  message: string;
}

export interface SuggestOk {
  seat: PlayerId;
  /** The chosen move, worded by the server. The client never constructs a move
   *  and never prices one. */
  text: string;
  explanation: Explanation;
  /** The principal continuation, root move first — so step 1 repeats `text`. */
  continuation: string[];
  /** EVERY root candidate, best first. Not truncated: showing more than three
   *  runner-ups is one of the things a terminal genuinely cannot do, so the
   *  presentation decision is the client's rather than the server's. */
  ranking: Ranked[];
  /** The ply guard stopped SOME continuation in this search — not necessarily
   *  the one in `continuation`. The counter carries no identity. */
  search_hit_ply_cap: boolean;
  tied_at_top: number;
}

export interface SuggestResponse {
  ok: SuggestOk | null;
  refusal: RefusalRow | null;
}

export interface WhyOk {
  provenance: Provenance;
  text: string;
  explanation: Explanation;
  continuation: string[];
  /** `null` for a greedy contestant, which searches no line at all — a
   *  different fact from a block of zeros, so it is not flattened to one. */
  counters: Counter[] | null;
  search_hit_ply_cap: boolean;
  ranking: Ranked[];
  /** The engine offered exactly one move here — on the ACCOUNT, not the
   *  claim: a fact about the mover's hand, withheld with the rest. */
  forced: boolean;
}

/** The withheld half (finding B1): the move played, the claim, and the one
 *  sentence — nothing the search found, which is a function of the mover's
 *  hidden starting order until that card is public, or the game is over and
 *  the reader holds a seat. */
export interface WhyWithheld {
  provenance: Provenance;
  text: string;
  reason: string;
}

/** Exactly one of the three is non-null; `withheld` and `refusal` are ANSWERS. */
export interface WhyResponse {
  ok: WhyOk | null;
  withheld: WhyWithheld | null;
  refusal: RefusalRow | null;
}

// ---- the card indexes and the final score (M3) ----------------------------

/** Where a card physically is, when the index knows.
 *
 *  Externally tagged, as serde writes a Rust enum. **All EIGHT variants of
 *  `screen_model::CardLocation` are declared, and there is no permissive
 *  catch-all arm.** The first draft declared five and ended in
 *  `| Record<string, unknown>`, and a review showed what that cost: the
 *  catch-all makes every object assignable, so `Widen<CardLocation>` accepted
 *  `{"TotalGarbage": {...}}` and the literal string `"utter nonsense"` without
 *  complaint — and it was already swallowing a real miss. `OnArea` was missing,
 *  ten of the twenty-four site cards carry it, and the browser printed the raw
 *  Rust tag `OnArea` where the terminal prints `on area A03`.
 *
 *  `null` means the index is not tracking it — **which is not the same as
 *  "nowhere"**: `Repl::index_placements` deliberately carries no other seat's
 *  hand, because in a live game an opponent's starting order is hidden and an
 *  index that dimmed it with "in Seat 1's hand" would leak the one card the
 *  observation boundary exists to protect. */
export type CardLocation =
  | "OwnHand"
  | "ThisReveal"
  | { Deal: { seat: PlayerId } }
  | { PortSlot: { board_b: boolean; slot: number } }
  | { LabSlot: { board_b: boolean; slot: number } }
  | { OwnedBy: { seat: PlayerId } }
  | { OnArea: { area: AreaId } }
  | { AchievementSlot: { category: AchievementCategory } };

/** The printed building site behind a site-index card — the raw registry card,
 *  not a board site: `build_site_index` reads `rules.building_sites`, so the
 *  built/owned fields are always empty even mid-game. */
export interface BuildingSiteCard {
  spaces: SpaceReward[];
  shophouses_built: (PlayerId | null)[];
  trading_house_owner: PlayerId | null;
  /** VP, a bare number — NOT a `SpaceReward`. The first draft of this type said
   *  `SpaceReward` and `contract.ts` rejected it against the committed payload
   *  on the first typecheck, which is the whole reason the corpus exists. */
  trading_house_reward: number;
}

/** One card's face. Externally tagged; exactly one key is present. */
export type CardFace =
  | { Order: { id: number; printed: OrderCard } }
  | {
      Tech: {
        printed: TechCard;
        /** Every physical copy of this TITLE. Ten of the twenty titles
         *  circulate as two identical cards, so the index is over titles. */
        copies: number[];
        placed: [number, CardLocation][];
      };
    }
  | { Achievement: AchievementModel }
  | { Site: { index: number; site: BuildingSiteCard } };

export interface IndexCard {
  face: CardFace;
  location: CardLocation | null;
  /** The number that would pick this card during a capture. **Always `null` on
   *  this API**: the web serves an index as a VIEW and has no card capture, so
   *  nothing here is numbered. */
  selection: number | null;
}

export interface IndexGroup {
  /** How the rows are grouped — by `{Country}` for orders and techs, by
   *  `{Category}` for goals, and the single `"All"` for sites. */
  key: unknown;
  cards: IndexCard[];
}

export type IndexKind = "Orders" | "Techs" | "Achievements" | "Sites";

export interface IndexModel {
  kind: IndexKind;
  /** The whole population — the "all 36" the title states. For techs that is
   *  TITLES, not the thirty physical cards. */
  total: number;
  /** What this screen shows; equal to `total` unfiltered. */
  shown: number;
  /** What is being recorded. Always `null` here — see `IndexCard.selection`. */
  target: unknown | null;
  filter: unknown | null;
  groups: IndexGroup[];
}

/** One seat's score, category by category — the engine's own field names.
 *  Several carry decision-log rulings behind them (church 6/3, customs 8/4, the
 *  technology majority the 2P dummy competes in), so the client shows the parts
 *  and never re-adds a total from parts it picked. */
export interface Breakdown {
  base_vp: number;
  church: number;
  customs: number;
  technology: number;
  country: number;
  remainder: number;
  total: number;
}

export interface Placing {
  seat: PlayerId;
  rank: number;
  breakdown: Breakdown;
  /** Against the best OTHER seat. **A margin of 0 is not a win** — two seats can
   *  tie on points and be separated by the engine's tie-break, so read `won`. */
  margin: number;
  won: boolean;
}

/** `GET /api/games/{id}/result`. A 409 while the game is still running. */
export interface Outcome {
  /** Rank order, so `standings[0]` is first place. */
  standings: Placing[];
  winner: PlayerId;
}

// ---- M6.4: the chat ------------------------------------------------------

/** Who said it (M6.4, R-6.4.2, R-6.4.15), as THIS reader is handed it.
 *  Tagged like `SeatView`, so an arm can be added on both sides of the wire;
 *  `bot` is read and rendered and produced by no route yet.
 *
 *  **No address** (M6.5c2 cycle D, R-6.5c2.35): `you` marks the reader's own
 *  messages, and a poster with no name reads as their seat or as "someone". */
export type ChatAuthor =
  | { kind: "seat"; seat: PlayerId; name: string | null; you: boolean }
  | { kind: "watcher"; name: string | null; you: boolean }
  | { kind: "bot"; seat: PlayerId; name: string | null };

export interface ChatMessage {
  seq: number;
  /** Unix SECONDS; named as the table's clock is, because the corpus freezer
   *  pins a clock by key name. */
  created_at: number;
  author: ChatAuthor;
  /** The empty string on a tombstone. */
  text: string;
  /** A TOMBSTONE (M6.5c2 cycle D, R-D4): taken down by its author or the
   *  table's creator. Its number, clock and author stay, so every cursor holds
   *  and the row still says who posted. */
  deleted: boolean;
}

/** `GET …/chat?after=N`: the retained messages above N, and the newest number
 *  ever assigned — the next cursor, which survives a trim. */
export interface ChatPage {
  messages: ChatMessage[];
  last: number;
  /** The document's REVISION (M6.5c2 cycle D, R-D4): +1 per message and per
   *  message deleted. `rev - last` moves exactly when a message a reader may
   *  already hold was deleted. The panel keeps this page's `rev - last` and
   *  compares each frame's with it: every chat host is a count host
   *  (`Events.chat`/`chat_rev` at a table, the house frame's `chat`/`rev` in
   *  the lobby), and those are the same document's `last` and `rev` on both
   *  hosts. */
  rev: number;
}

export interface ChatPost {
  text: string;
  /** A watcher's name, and anyone's at the house. A seated caller sends none:
   *  a seat's name is the seat's, and the server refuses one with a 400. */
  name?: string;
}

/** Where a chat panel is: one table, or the house. A client-side type. */
export type ChatScope = { kind: "table"; id: string } | { kind: "house" };

/** The frames of `[from, to)` in ONE reply — `GET …/at?from=&to=`, one forward
 *  replay on the server instead of one `at/{k}` per position (M6.5b,
 *  R-6.5b.10). Each frame is byte-equal to `/at/{k}`'s. `to` may be
 *  `events + 1`: the run then ends on the live frame. A run longer than the
 *  server's cap (64) is a 400 naming it, never a clamp — the client pages. */
export interface RunFrames {
  frames: HistoricalFrame[];
}

/** Where a game played in this browser landed when it was sent to the server's
 *  archive — `POST /api/archive` (M6.5b, R-6.5b.6).
 *
 *  The leaf is the SERVER's, chosen from a strict replay of the record rather
 *  than from anything the body said, so a client cannot file a running game as
 *  finished. `identical` marks a retry that found its own bytes already there —
 *  a 200 rather than a 201 — and the client proceeds on either; only a 409
 *  ("a different record is archived under this id") keeps the game. */
export interface ArchiveReply {
  leaf: "over" | "abandoned";
  path: string;
  identical: boolean;
}
