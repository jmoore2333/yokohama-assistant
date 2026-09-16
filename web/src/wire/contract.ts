// **The client's half of the wire contract.**
//
// The JSON beside this file is emitted by the server, by
// `crates/yokohama-web/tests/wire_fixtures.rs`, and that test refuses when what
// the server emits is not what is committed. This file is the other end: it
// checks the SAME files against `types.ts`, which is hand-written and until now
// was checked against nothing at all. `npx tsc -b --noEmit` proves the client
// agrees with itself; these declarations make it prove the client agrees with
// the server.
//
// Nothing here runs. Every check below is a type-level assertion evaluated by
// the existing type gate, so there is no new command to remember — and
// `contract.test.ts` adds the one check a type cannot make (see §2).
//
// ## §1. Why the fixtures are checked against `Widen<T>` and not against `T`
//
// TypeScript types a JSON import with WIDENED primitives: `"sim"` in a file
// becomes `string`, not `"sim"`. So `const s: ClientState = optionsJson` fails
// on every string-literal union in the surface — `mode`, `prompt.kind`,
// `area_type`, `color`, `effect`, and a dozen more — for a reason that has
// nothing to do with the contract.
//
// `Widen<T>` walks the declared type and applies the same widening, so what is
// left to check is exactly what a JSON import CAN carry: the field names, the
// nesting, and the shape of every union member. That is the drift worth
// catching, because it is the drift that breaks a browser:
//
// - **A field the client reads that the server no longer emits FAILS.** The
//   fixture is missing the property and the assignment is rejected, naming it.
// - **A field renamed on either side FAILS**, from the same rule.
// - **A field the server adds that the client does not declare does NOT fail.**
//   Excess-property checking applies to object literals, not to values, and a
//   JSON import is a value. That is a real limit and it is stated rather than
//   discovered; §2 is what closes it at the payload roots.
//
// **`PromptState` is the one union widening RUINS, and it needed its own fix.**
// serde's external tagging keys most variants by their own field name
// (`{"Copper": 2}` versus `{"Tea": 3}`), so widening leaves them
// distinguishable. `PromptState` is `#[serde(tag = "kind")]` — INTERNALLY
// tagged, the one exception on the wire, and `types.ts` says so — so
// `Widen<{kind:"reveal"}>` and `Widen<{kind:"game_over"}>` both collapse to
// `{kind: string}`. A union accepts a value matching ANY member, so every
// prompt payload matched that one and all thirteen prompt fields the client
// renders were checked by NOTHING. Measured: replacing `retreat.json`'s whole
// prompt with `{"kind":"nonsense"}` type-checked clean.
//
// §1b below is the fix: each prompt fixture is asserted against ITS OWN member,
// picked out with `Extract`, where the non-discriminant fields are required
// again.

import type {
  AreasPanel,
  ArtManifest,
  ApiErrorBody,
  ClientState,
  Counter,
  DealCard,
  DealingErrorBody,
  DealView,
  Events,
  GamesList,
  IndexCard,
  IndexGroup,
  DecisionModel,
  Breakdown,
  Explanation,
  FrameModel,
  Health,
  HistoricalFrame,
  IndexModel,
  Labels,
  LabsPanel,
  LogPanel,
  Me,
  OptionRow,
  Outcome,
  Placing,
  PlayerModel,
  PortsPanel,
  PromptState,
  Provenance,
  Ranked,
  RefusalRow,
  SuggestOk,
  SuggestResponse,
  TableView,
  Term,
  Terminal,
  TileModel,
  TracksPanel,
  WhyOk,
  WhyResponse,
} from "../types";

import areas from "./areas.json";
import choiceAssistantMove from "./choice_AssistantMove.json";
import choiceChurchSpace from "./choice_ChurchSpace.json";
import choiceWarehouseTake from "./choice_WarehouseTake.json";
import areas4p from "./areas_4p.json";
import artManifest from "./art_manifest.json";
import siteSpaceWords from "./site_space_words.json";
import error from "./error.json";
import events from "./events.json";
import eventsDealing from "./events_dealing.json";
import feeNonzero from "./fee_nonzero.json";
import gameOver from "./game_over.json";
import health from "./health.json";
import labels from "./labels.json";
import labs from "./labs.json";
import locked from "./locked.json";
import log from "./log.json";
import me from "./me.json";
import meSignedOut from "./me_signed_out.json";
import atK from "./at_k.json";
import options from "./options.json";
import options4p from "./options_4p.json";
import optionsMany from "./options_many.json";
import optionsPriced from "./options_priced.json";
import pick from "./pick.json";
import placeA from "./place_a.json";
import placeASelected from "./place_a_selected.json";
import placeB from "./place_b.json";
import placeBSelected from "./place_b_selected.json";
import ports from "./ports.json";
import tableDealing from "./table_dealing.json";
import table from "./table.json";
import deal from "./deal.json";
import dealWaiting from "./deal_waiting.json";
import dealing409 from "./dealing_409.json";
import gamesList from "./games.json";
import gamesOpen from "./games_open.json";
import gamesNext from "./games_next.json";
import tableOpen from "./table_open.json";
import open409 from "./open_409.json";
import join403Locked from "./join_403_locked.json";
import join403Password from "./join_403_password.json";
import join409Taken from "./join_409_taken.json";
import join409NotOpen from "./join_409_not_open.json";
import notSeated403 from "./not_seated_403.json";
import eventsOpen from "./events_open.json";
import spectator from "./spectator.json";
import spectatorAtK from "./spectator_at_k.json";
import spectatorOver from "./spectator_over.json";
import gamesPlaying from "./games_playing.json";
import tableWatchable from "./table_watchable.json";
import unlock403 from "./unlock_403.json";
import unlock409Open from "./unlock_409_open.json";
import watch403Locked from "./watch_403_locked.json";
import undoRefused403 from "./undo_refused_403.json";
import wrongSeat403 from "./wrong_seat_403.json";
import refused from "./refused.json";
import retreat from "./retreat.json";
import retreatAnswered from "./retreat_answered.json";
import explanationTerminal from "./explanation_terminal.json";
import indexFiltered from "./index_filtered.json";
import indexGoals from "./index_goals.json";
import indexOrders from "./index_orders.json";
import indexSites from "./index_sites.json";
import indexTechs from "./index_techs.json";
import result from "./result.json";
import suggest from "./suggest.json";
import why from "./why.json";
import whyRefusals from "./why_refusals.json";
import whyRefused from "./why_refused.json";
import watching from "./watching.json";

/** The declared type, widened exactly as importing it from JSON would widen it.
 *
 *  **A TUPLE degrades to an unbounded array here, and that is forced rather
 *  than chosen**: TypeScript types a JSON array as `T[]`, never as a tuple, so
 *  a mapped form that preserved `[number, number]` would reject every real
 *  fixture. `grid_position`, `assistants`, `progress` and `fee.to_players` are
 *  therefore checked for their ELEMENT type here and for their ARITY at run
 *  time, in `contract.test.ts`. */
type Widen<T> = T extends string
  ? string
  : T extends number
    ? number
    : T extends boolean
      ? boolean
      : T extends null
        ? null
        : T extends readonly (infer E)[]
          ? Widen<E>[]
          : T extends object
            ? { [K in keyof T]: Widen<T[K]> }
            : T;

// The assignments. Each one is the whole check for its payload: if the file on
// the right is missing a field the type on the left declares, this line is a
// compile error naming it.
const _options: Widen<ClientState> = options;
const _options4p: Widen<ClientState> = options4p;
const _optionsMany: Widen<ClientState> = optionsMany;
const _optionsPriced: Widen<ClientState> = optionsPriced;
const _pick: Widen<ClientState> = pick;
const _retreat: Widen<ClientState> = retreat;
const _retreatAnswered: Widen<ClientState> = retreatAnswered;
const _placeA: Widen<ClientState> = placeA;
const _placeASelected: Widen<ClientState> = placeASelected;
const _placeB: Widen<ClientState> = placeB;
const _placeBSelected: Widen<ClientState> = placeBSelected;
const _gameOver: Widen<ClientState> = gameOver;
const _watching: Widen<ClientState> = watching;
const _locked: Widen<ClientState> = locked;
const _feeNonzero: Widen<ClientState> = feeNonzero;
const _choiceAssistantMove: Widen<ClientState> = choiceAssistantMove;
const _choiceChurchSpace: Widen<ClientState> = choiceChurchSpace;
const _choiceWarehouseTake: Widen<ClientState> = choiceWarehouseTake;
const _refused: Widen<ClientState> = refused;
const _labels: Widen<Labels> = labels;
// ---- M6.1: the table, the deal, the lobby, and the two refusals ----------
//
// `table_dealing` and `table` are the SAME type at its two statuses, and both
// are here because a widened `TableStatus` cannot tell them apart: `to_move`
// and `over` are null on one and present on the other, and only a payload of
// each proves both shapes are the type the client declares.
const _tableDealing: Widen<TableView> = tableDealing;
const _table: Widen<TableView> = table;
// Likewise the deal at its two shapes: the caller's cards, and the null pair
// that is the "waiting for the others" screen.
const _deal: Widen<DealView> = deal;
const _dealWaiting: Widen<DealView> = dealWaiting;
const _gamesList: Widen<GamesList> = gamesList;
// ---- M6.2: the room's opening ------------------------------------------
//
// The table at its THIRD status, read by somebody who is not seated at it —
// the one payload in the corpus whose `my_seats` is empty, and the only place
// an `open` seat and a `locked` table reach the client's declared types.
const _tableOpen: Widen<TableView> = tableOpen;
// The lobby's two lists. `games_open` is the OTHER question (tables I could sit
// at, whoever made them), and `games_next` is the only payload anywhere whose
// `next` is a cursor rather than null — so the string arm of that field is
// checked by a payload rather than by nothing.
const _gamesOpen: Widen<GamesList> = gamesOpen;
const _gamesNext: Widen<GamesList> = gamesNext;
// A session route asked of a table that is still OPEN. The same two-key shape
// as the dealing 409 and a different word in the field the client branches on.
const _open409: Widen<DealingErrorBody> = open409;
// ...and the join's own, at a table that has left `open`.
const _join409NotOpen: Widen<DealingErrorBody> = join409NotOpen;
// The three refusals a join meets, in the order the server applies them. Plain
// `ApiErrorBody`, committed for the SENTENCES — see `contract.test.ts`.
const _join403Locked: Widen<ApiErrorBody> = join403Locked;
const _join403Password: Widen<ApiErrorBody> = join403Password;
const _join409Taken: Widen<ApiErrorBody> = join409Taken;
// The signal at both of the statuses the client polls through. Two payloads
// because `to_move` is null on one and a seat on the other, and a widened
// `status` cannot tell the two apart on its own.
const _events: Widen<Events> = events;
const _eventsDealing: Widen<Events> = eventsDealing;
// ---- M6.3: the watcher --------------------------------------------------
//
// The signal at the table's FIRST status. A third payload rather than a third
// reading of the same one: `status` is the field the client branches the SCREEN
// on, and `open` is the only value of it that no other fixture carries.
const _eventsOpen: Widen<Events> = eventsOpen;
// **The three payloads whose `seat` is null**, which is the M6.3 shape and the
// reason that field became a union: a watcher holds no seat, so a type that
// declared a number here would have been wrong about every one of these while
// compiling perfectly. The live frame, the finished one and a replay stop.
const _spectator: Widen<ClientState> = spectator;
const _spectatorOver: Widen<ClientState> = spectatorOver;
const _spectatorAtK: Widen<HistoricalFrame> = spectatorAtK;
// The lobby's third list — every game under way, whoever made it (R-3.9).
const _gamesPlaying: Widen<GamesList> = gamesPlaying;
// A LOCKED, playing table read by somebody with no seat at it: the payload the
// unlock face is drawn from, and the only one in the corpus whose `can_watch`
// is false while `my_seats` is empty.
const _tableWatchable: Widen<TableView> = tableWatchable;
// The two refusals a would-be watcher meets, and the 409 at a table with
// nothing to watch yet. Committed for the SENTENCES, as the join refusals are:
// "this table is locked — unlock it first" is a different thing to do next from
// "wrong password for this table", and both differ from the not-seated 403 that
// tells a caller there is nothing they can do at all.
const _watch403Locked: Widen<ApiErrorBody> = watch403Locked;
const _unlock403: Widen<ApiErrorBody> = unlock403;
const _unlock409Open: Widen<DealingErrorBody> = unlock409Open;
const _dealing409: Widen<DealingErrorBody> = dealing409;
const _notSeated403: Widen<ApiErrorBody> = notSeated403;
// The two refusals a SEATED caller meets by acting out of turn (spec §8). The
// same `ApiErrorBody` shape as the 403 above, and committed for the SENTENCES:
// the undo rule runs in front of the acting-seat fence (R-5.1), so the two
// answer a player differently and a server that lost the ordering would answer
// both the same way.
const _wrongSeat403: Widen<ApiErrorBody> = wrongSeat403;
const _undoRefused403: Widen<ApiErrorBody> = undoRefused403;

const _health: Widen<Health> = health;
// Both sides of `/api/me`: signed in, and the payload a caller with no identity
// is handed. The second is a real response and not a construction — `user` is
// `null` there, and a type alone cannot say that the SERVER answers rather than
// refusing.
const _me: Widen<Me> = me;
const _meOut: Widen<Me> = meSignedOut;
const _error: Widen<ApiErrorBody> = error;
const _ports: Widen<PortsPanel> = ports;
const _labs: Widen<LabsPanel> = labs;
const _areas: Widen<AreasPanel> = areas;
const _areas4p: Widen<AreasPanel> = areas4p;
const _log: Widen<LogPanel> = log;
const _atK: Widen<HistoricalFrame> = atK;
const _artManifest: Widen<ArtManifest> = artManifest;
const _suggest: Widen<SuggestResponse> = suggest;
const _why: Widen<WhyResponse> = why;
const _whyRefused: Widen<WhyResponse> = whyRefused;
const _whyRefusals: Widen<RefusalRow[]> = whyRefusals;
const _indexOrders: Widen<IndexModel> = indexOrders;
const _indexTechs: Widen<IndexModel> = indexTechs;
const _indexGoals: Widen<IndexModel> = indexGoals;
const _indexSites: Widen<IndexModel> = indexSites;
const _indexFiltered: Widen<IndexModel> = indexFiltered;
const _result: Widen<Outcome> = result;
const _placing: Widen<Placing>[] = result.standings;

// **The TERMINAL branch, which nothing reached until a review proved it.**
// `terminal` is `null` in both search payloads, so `Widen<Explanation>` is
// satisfied without the union ever descending into `Widen<Terminal>` — renaming
// all five of its wire fields left the whole net green. This fixture is a real
// terminal explanation from a finished game, so the branch is now entered.
const _terminal: Widen<Explanation> = explanationTerminal;
const _terminalStanding: Widen<Terminal> = explanationTerminal.terminal;

// The two envelopes are `ok | null` unions, so `Widen<SuggestResponse>` is
// satisfied by `{ok: null, refusal: {...}}` — which means the assignments above
// check NOTHING about a granted payload's fields. Exactly the `PromptState`
// defect from M2.5's review, in a different union. So the granted halves are
// asserted against their own members, the way §1b does it for prompts.
const _suggestOk: Widen<SuggestOk> = suggest.ok;
const _whyOk: Widen<WhyOk> = why.ok;
const _whyProvenance: Widen<Provenance> = why.ok.provenance;
// The ARRAYS rather than an element: `noUncheckedIndexedAccess` types `a[0]` as
// `T | undefined`, and an assertion that tolerates `undefined` checks nothing
// about the element. Checking the array checks every element's shape.
const _whyRanked: Widen<Ranked>[] = why.ok.ranking;
const _suggestTerm: Widen<Term>[] = suggest.ok.explanation.terms;
const _refusedRow: Widen<RefusalRow> = whyRefused.refusal;

// ## §1b. The prompt, per member
//
// The assignments above check every field of `ClientState` EXCEPT `prompt`,
// for the reason in the header. These check `prompt` — one assertion per
// payload, against the member its `kind` names, so the fields that member
// declares are required again.

type PromptOf<K extends PromptState["kind"]> = Widen<Extract<PromptState, { kind: K }>>;

const _pOptions: PromptOf<"options"> = options.prompt;
const _pOptionsMany: PromptOf<"options"> = optionsMany.prompt;
const _pOptionsPriced: PromptOf<"options"> = optionsPriced.prompt;
const _pOptions4p: PromptOf<"options"> = options4p.prompt;
const _pRefused: PromptOf<"options"> = refused.prompt;
const _pPick: PromptOf<"pick"> = pick.prompt;
const _pRetreat: PromptOf<"retreat"> = retreat.prompt;
const _pRetreatAnswered: PromptOf<"retreat"> = retreatAnswered.prompt;
const _pPlaceA: PromptOf<"place_a"> = placeA.prompt;
const _pPlaceASelected: PromptOf<"place_a"> = placeASelected.prompt;
const _pFeeNonzero: PromptOf<"place_a"> = feeNonzero.prompt;
const _pChoiceAssistantMove: PromptOf<"options"> = choiceAssistantMove.prompt;
const _pChoiceWarehouseTake: PromptOf<"options"> = choiceWarehouseTake.prompt;
const _pChoiceChurchSpace: PromptOf<"options"> = choiceChurchSpace.prompt;
const _pPlaceB: PromptOf<"place_b"> = placeB.prompt;
const _pPlaceBSelected: PromptOf<"place_b"> = placeBSelected.prompt;
const _pGameOver: PromptOf<"game_over"> = gameOver.prompt;
const _pSpectatorOver: PromptOf<"game_over"> = spectatorOver.prompt;
const _pWatching: PromptOf<"watching"> = watching.prompt;
// The `watching` prompt again, from the OTHER thing that produces it. The
// fixture above is a seated caller waiting on a bot; this one is a watcher, who
// is waiting on everybody. The prompt is the same shape and the payload around
// it is not.
const _pSpectator: PromptOf<"watching"> = spectator.prompt;
const _pLocked: PromptOf<"locked"> = locked.prompt;
// `reveal` has no fixture — live mode only; see `wire_fixtures.rs`'s exemption.

void [
  _pOptions, _pOptionsMany, _pOptionsPriced, _pOptions4p, _pRefused,
  _pPick, _pRetreat, _pRetreatAnswered, _pPlaceA, _pPlaceASelected, _pPlaceB,
  _pPlaceBSelected, _pGameOver, _pWatching, _pLocked, _pFeeNonzero,
  _pSpectator, _pSpectatorOver,
  _pChoiceAssistantMove, _pChoiceWarehouseTake, _pChoiceChurchSpace,
];

// `noUnusedLocals` is on, and these exist to be checked rather than read. One
// reference keeps the compiler's other rule from deleting the check.
void [
  _options, _options4p, _optionsMany, _optionsPriced, _pick, _retreat, _retreatAnswered, _placeA, _placeASelected, _placeB, _placeBSelected,
  _gameOver, _watching, _locked, _refused, _labels, _health, _me, _meOut, _error,
  _tableDealing, _table, _deal, _dealWaiting, _gamesList, _dealing409, _notSeated403,
  _tableOpen, _gamesOpen, _gamesNext, _open409, _join409NotOpen,
  _join403Locked, _join403Password, _join409Taken,
  _wrongSeat403, _undoRefused403,
  _events, _eventsDealing, _eventsOpen,
  _spectator, _spectatorOver, _spectatorAtK, _gamesPlaying, _tableWatchable,
  _watch403Locked, _unlock403, _unlock409Open,
  _ports, _labs, _areas, _areas4p, _log, _atK, _artManifest, _feeNonzero,
  _choiceAssistantMove, _choiceChurchSpace, _choiceWarehouseTake,
  _suggest, _why, _whyRefused, _whyRefusals,
  _suggestOk, _whyOk, _whyProvenance, _whyRanked, _suggestTerm, _refusedRow,
  _indexOrders, _indexTechs, _indexGoals, _indexSites, _indexFiltered, _result, _placing,
  _terminal, _terminalStanding,
];

// ## §2. The key sets, so a field the SERVER adds is not silently unknown
//
// §1's limit is one-directional: a payload with an extra field still satisfies
// the type. At the roots that matters, because an added field is usually a
// field the client should be reading — the end-trigger strip, `deck_remaining`
// and `end_game_triggered` all arrived exactly that way in M2, and nothing
// would have said so.
//
// So each list below is asserted to EQUAL `keyof T` at compile time, and
// `contract.test.ts` compares it to `Object.keys(fixture)` at run time. The two
// halves together mean a field added on either side fails on the other:
//
// - added to `types.ts` only  -> the runtime comparison fails (no such key in
//   the payload)
// - added to the server only  -> the runtime comparison fails (an unlisted key
//   in the payload), and the fixture golden in `wire_fixtures.rs` has already
//   failed first
// - added to this list only   -> `KeysOf` fails and names it
//
// Only the ROOTS carry a list. Going deeper would restate most of `types.ts` as
// string arrays, which is a second copy to keep true; §1 already covers nesting
// in the direction that breaks a browser.

/** `K` must be exactly `keyof T`. Resolves to `K` when it is, and otherwise to
 *  a tuple whose text NAMES the offending keys — so the compile error reads as
 *  a sentence rather than as a mismatch between two long unions. */
type KeysOf<T, K extends readonly string[]> = [
  Exclude<keyof T & string, K[number]>,
] extends [never]
  ? [Exclude<K[number], keyof T & string>] extends [never]
    ? K
    : ["this list names keys the type does not have:", Exclude<K[number], keyof T & string>]
  : ["this list is missing keys the type has:", Exclude<keyof T & string, K[number]>];

const CLIENT_STATE = [
  "id", "mode", "seat", "to_move", "your_turn", "frame", "prompt",
  "messages", "over", "end_triggers", "deck_remaining", "end_game_triggered",
  "scores", "warnings", "events",
] as const;
const FRAME = ["title", "board", "dashboard", "decision"] as const;
const DECISION = [
  "turn_number", "seat", "phase_chain", "current_phase", "deciding", "options",
  "not_offered", "not_offered_suppressed", "viewer_is_deciding",
] as const;
const OPTION_ROW = ["number", "words", "opens_picker", "fee"] as const;
const PLAYER = [
  "seat", "color", "money", "victory_points", "goods", "assistants",
  "shophouses", "trading_houses", "technologies", "mercenaries_unused",
  "mercenaries_used", "five_power_tokens", "president", "orders_in_hand",
  "orders_filled", "is_current", "is_bot", "hand_hidden",
] as const;
const TILE = [
  "area", "area_type", "grid_position", "token", "site", "presidents",
  "has_station", "assistants", "reachable", "canal_crossings",
] as const;
const TRACKS = [
  "church", "church_filled", "customs", "customs_filled", "trigger_at",
  "station", "mercenary_supply",
] as const;
const PORTS = ["boards", "deck_remaining"] as const;
const LABS = ["boards", "deck_remaining"] as const;
const AREAS = ["rows"] as const;
const LOG = ["total", "entries"] as const;
// The replay payload. Its shape is defined by what it does NOT carry — see
// `HistoricalFrame` in `types.ts` — so this list is the fence that keeps a
// present-tense field from being quietly added back on the server.
const HISTORICAL_FRAME = [
  "index", "total", "seat", "to_move", "frame", "over", "end_triggers",
  "deck_remaining", "end_game_triggered", "scores", "ports", "labs",
] as const;
const LABELS = [
  "area_types", "phases", "phase_chain", "tech_effects", "achievements", "features",
] as const;
const HEALTH = ["server_version", "semantics_rev", "rules_hash", "identity", "signal"] as const;
const ME = ["mode", "user"] as const;
const TABLE = [
  "id", "name", "created_by", "created_at", "players", "colors", "start_seat",
  "seats", "chosen", "my_seats", "status", "to_move", "over", "locked", "winner",
  "can_watch",
] as const;
const DEAL_VIEW = ["table", "seat", "cards", "keep_note", "frame"] as const;
const GAMES_LIST = ["games", "next"] as const;
// The open and dealing 409s are the ONLY refusals that carry a second key, and
// the only ones this client branches on rather than displays.
const DEALING_ERROR = ["error", "status"] as const;
const EVENTS = ["events", "to_move", "status", "your_turn"] as const;
const DEAL_CARD = ["card", "keep_score"] as const;
const API_ERROR = ["error"] as const;
const SUGGEST_RESPONSE = ["ok", "refusal"] as const;
const SUGGEST_OK = [
  "seat", "text", "explanation", "continuation", "ranking", "stream_seed",
  "search_hit_ply_cap", "tied_at_top",
] as const;
const WHY_RESPONSE = ["ok", "refusal"] as const;
const WHY_OK = [
  "provenance", "text", "explanation", "continuation", "counters",
  "search_hit_ply_cap", "ranking",
] as const;
const EXPLANATION = ["terms", "terminal", "total", "progress_ppm", "nonzero"] as const;
const TERM = ["feature", "viewer_value", "opponent_value", "weight", "product"] as const;
const PROVENANCE = [
  "contestant", "evaluator_version", "index", "seat",
  "decision_seed", "forced", "recorded_version", "current_version",
] as const;
const RANKED = ["text", "value", "margin", "chosen"] as const;
const REFUSAL_ROW = ["kind", "message"] as const;
const INDEX = ["kind", "total", "shown", "target", "filter", "groups"] as const;
const OUTCOME = ["standings", "winner"] as const;
const PLACING = ["seat", "rank", "breakdown", "margin", "won"] as const;
const BREAKDOWN = [
  "base_vp", "church", "customs", "technology", "country", "remainder", "total",
] as const;
const TERMINAL = ["rank", "seats", "viewer_total", "best_other_total", "value"] as const;
const COUNTER = ["name", "value", "group"] as const;
const INDEX_GROUP = ["key", "cards"] as const;
const INDEX_CARD = ["face", "location", "selection"] as const;

// The compile-time half. A list that is not exactly `keyof T` fails HERE, with
// the offending key names in the message.
const _cs: KeysOf<ClientState, typeof CLIENT_STATE> = CLIENT_STATE;
const _fr: KeysOf<FrameModel, typeof FRAME> = FRAME;
const _de: KeysOf<DecisionModel, typeof DECISION> = DECISION;
const _or: KeysOf<OptionRow, typeof OPTION_ROW> = OPTION_ROW;
const _pl: KeysOf<PlayerModel, typeof PLAYER> = PLAYER;
const _ti: KeysOf<TileModel, typeof TILE> = TILE;
const _tr: KeysOf<TracksPanel, typeof TRACKS> = TRACKS;
const _po: KeysOf<PortsPanel, typeof PORTS> = PORTS;
const _la: KeysOf<LabsPanel, typeof LABS> = LABS;
const _ar: KeysOf<AreasPanel, typeof AREAS> = AREAS;
const _lo: KeysOf<LogPanel, typeof LOG> = LOG;
const _lb: KeysOf<Labels, typeof LABELS> = LABELS;
const _he: KeysOf<Health, typeof HEALTH> = HEALTH;
// `_me` is taken by the widened assignment above, so the key list takes `_mk`.
const _mk: KeysOf<Me, typeof ME> = ME;
const _tv: KeysOf<TableView, typeof TABLE> = TABLE;
const _dv: KeysOf<DealView, typeof DEAL_VIEW> = DEAL_VIEW;
const _gl: KeysOf<GamesList, typeof GAMES_LIST> = GAMES_LIST;
const _d4: KeysOf<DealingErrorBody, typeof DEALING_ERROR> = DEALING_ERROR;
const _ev: KeysOf<Events, typeof EVENTS> = EVENTS;
const _dc: KeysOf<DealCard, typeof DEAL_CARD> = DEAL_CARD;
const _er: KeysOf<ApiErrorBody, typeof API_ERROR> = API_ERROR;
const _sr: KeysOf<SuggestResponse, typeof SUGGEST_RESPONSE> = SUGGEST_RESPONSE;
const _so: KeysOf<SuggestOk, typeof SUGGEST_OK> = SUGGEST_OK;
const _wr: KeysOf<WhyResponse, typeof WHY_RESPONSE> = WHY_RESPONSE;
const _wo: KeysOf<WhyOk, typeof WHY_OK> = WHY_OK;
const _ex: KeysOf<Explanation, typeof EXPLANATION> = EXPLANATION;
const _tm: KeysOf<Term, typeof TERM> = TERM;
const _pr: KeysOf<Provenance, typeof PROVENANCE> = PROVENANCE;
const _rk: KeysOf<Ranked, typeof RANKED> = RANKED;
const _rf: KeysOf<RefusalRow, typeof REFUSAL_ROW> = REFUSAL_ROW;
const _ix: KeysOf<IndexModel, typeof INDEX> = INDEX;
const _oc: KeysOf<Outcome, typeof OUTCOME> = OUTCOME;
const _pc: KeysOf<Placing, typeof PLACING> = PLACING;
const _bd: KeysOf<Breakdown, typeof BREAKDOWN> = BREAKDOWN;
const _tl: KeysOf<Terminal, typeof TERMINAL> = TERMINAL;
const _ct: KeysOf<Counter, typeof COUNTER> = COUNTER;
const _ig: KeysOf<IndexGroup, typeof INDEX_GROUP> = INDEX_GROUP;
const _hf: KeysOf<HistoricalFrame, typeof HISTORICAL_FRAME> = HISTORICAL_FRAME;
const _ic: KeysOf<IndexCard, typeof INDEX_CARD> = INDEX_CARD;
void [_cs, _fr, _de, _or, _pl, _ti, _tr, _po, _la, _ar, _lo, _lb, _he, _mk, _dc, _er,
      _ev,
      _tv, _dv, _gl, _d4,
      _sr, _so, _wr, _wo, _ex, _tm, _pr, _rk, _rf, _ix, _oc, _pc, _bd,
      _tl, _ct, _ig, _ic, _hf];

/** The key lists, for the runtime half in `contract.test.ts`. */
export const KEYS = {
  clientState: CLIENT_STATE,
  frame: FRAME,
  historicalFrame: HISTORICAL_FRAME,
  decision: DECISION,
  optionRow: OPTION_ROW,
  player: PLAYER,
  tile: TILE,
  tracks: TRACKS,
  ports: PORTS,
  labs: LABS,
  areas: AREAS,
  log: LOG,
  labels: LABELS,
  health: HEALTH,
  me: ME,
  table: TABLE,
  dealView: DEAL_VIEW,
  gamesList: GAMES_LIST,
  events: EVENTS,
  dealingError: DEALING_ERROR,
  dealCard: DEAL_CARD,
  apiError: API_ERROR,
  suggestResponse: SUGGEST_RESPONSE,
  suggestOk: SUGGEST_OK,
  whyResponse: WHY_RESPONSE,
  whyOk: WHY_OK,
  explanation: EXPLANATION,
  term: TERM,
  provenance: PROVENANCE,
  ranked: RANKED,
  refusalRow: REFUSAL_ROW,
  index: INDEX,
  outcome: OUTCOME,
  placing: PLACING,
  breakdown: BREAKDOWN,
  terminal: TERMINAL,
  counter: COUNTER,
  indexGroup: INDEX_GROUP,
  indexCard: INDEX_CARD,
} as const;

// ## §3. The fixtures, typed, for every other test to render
//
// **The casts are what §1 and §1b earn.** A component test needs a real `ClientState`
// to pass as a prop, and a JSON import is not one — its `mode` is `string`. The
// assertion above has already proved this exact value has every field the type
// declares, in the right shape, so narrowing the widened primitives back is the
// one thing left and it cannot be done by inference. Written as
// `as unknown as T` in ONE place, so no test has to do it itself and no test
// can quietly cast something the contract has not checked.
//
// A payload here is therefore never hand-authored. The alternative — a
// `makeState({...})` builder — is a third statement of the wire shape, and it
// drifts silently in the direction that matters: a builder still compiles after
// the server drops a field, and every test using it keeps passing.

const as = <T>(v: unknown): T => v as T;

export const wire = {
  atK: as<HistoricalFrame>(atK),
  options: as<ClientState>(options),
  optionsMany: as<ClientState>(optionsMany),
  options4p: as<ClientState>(options4p),
  optionsPriced: as<ClientState>(optionsPriced),
  pick: as<ClientState>(pick),
  retreat: as<ClientState>(retreat),
  retreatAnswered: as<ClientState>(retreatAnswered),
  placeA: as<ClientState>(placeA),
  placeASelected: as<ClientState>(placeASelected),
  placeB: as<ClientState>(placeB),
  placeBSelected: as<ClientState>(placeBSelected),
  gameOver: as<ClientState>(gameOver),
  watching: as<ClientState>(watching),
  locked: as<ClientState>(locked),
  feeNonzero: as<ClientState>(feeNonzero),
  choiceAssistantMove: as<ClientState>(choiceAssistantMove),
  choiceChurchSpace: as<ClientState>(choiceChurchSpace),
  choiceWarehouseTake: as<ClientState>(choiceWarehouseTake),
  refused: as<ClientState>(refused),
  labels: as<Labels>(labels),
  tableDealing: as<TableView>(tableDealing),
  table: as<TableView>(table),
  deal: as<DealView>(deal),
  dealWaiting: as<DealView>(dealWaiting),
  games: as<GamesList>(gamesList),
  tableOpen: as<TableView>(tableOpen),
  gamesOpen: as<GamesList>(gamesOpen),
  gamesNext: as<GamesList>(gamesNext),
  open409: as<DealingErrorBody>(open409),
  join409NotOpen: as<DealingErrorBody>(join409NotOpen),
  join403Locked: as<ApiErrorBody>(join403Locked),
  join403Password: as<ApiErrorBody>(join403Password),
  join409Taken: as<ApiErrorBody>(join409Taken),
  events: as<Events>(events),
  eventsDealing: as<Events>(eventsDealing),
  eventsOpen: as<Events>(eventsOpen),
  spectator: as<ClientState>(spectator),
  spectatorOver: as<ClientState>(spectatorOver),
  spectatorAtK: as<HistoricalFrame>(spectatorAtK),
  gamesPlaying: as<GamesList>(gamesPlaying),
  tableWatchable: as<TableView>(tableWatchable),
  watch403Locked: as<ApiErrorBody>(watch403Locked),
  unlock403: as<ApiErrorBody>(unlock403),
  unlock409Open: as<DealingErrorBody>(unlock409Open),
  dealing409: as<DealingErrorBody>(dealing409),
  notSeated403: as<ApiErrorBody>(notSeated403),
  wrongSeat403: as<ApiErrorBody>(wrongSeat403),
  undoRefused403: as<ApiErrorBody>(undoRefused403),
  health: as<Health>(health),
  me: as<Me>(me),
  meSignedOut: as<Me>(meSignedOut),
  error: as<ApiErrorBody>(error),
  ports: as<PortsPanel>(ports),
  labs: as<LabsPanel>(labs),
  areas: as<AreasPanel>(areas),
  areas4p: as<AreasPanel>(areas4p),
  log: as<LogPanel>(log),
  artManifest: as<ArtManifest>(artManifest),
  /** Not a payload any route serves: the TERMINAL's own words for every printed
   *  building-site space, frozen so `Cards.spaceWords` cannot drift from
   *  `card_render::space_reward_words`. See that fixture's doc in
   *  `wire_fixtures.rs` — the client has to word these itself, and had 55 of the
   *  96 wrong. */
  siteSpaceWords: siteSpaceWords as { cards: { index: number; spaces: string[] }[] },
  suggest: as<SuggestResponse>(suggest),
  why: as<WhyResponse>(why),
  whyRefused: as<WhyResponse>(whyRefused),
  whyRefusals: as<RefusalRow[]>(whyRefusals),
  indexOrders: as<IndexModel>(indexOrders),
  indexTechs: as<IndexModel>(indexTechs),
  indexGoals: as<IndexModel>(indexGoals),
  indexSites: as<IndexModel>(indexSites),
  indexFiltered: as<IndexModel>(indexFiltered),
  result: as<Outcome>(result),
  explanationTerminal: as<Explanation>(explanationTerminal),
} as const;

/** The queued-choice heads the corpus DOES carry.
 *
 *  Three of seven. `AnyGood`, `RecoveryLeave`, `StationPlace` and `CustomsSpace`
 *  are reachable in principle and the walk has not met them, so the client's
 *  wording for those four is rendered by no payload — the raw key would be
 *  shown and nothing would fail. Named here rather than left to be discovered;
 *  the walk banks any head it meets, so a future seed may close the gap. */
export const CHOICE_HEADS_COVERED = ["AssistantMove", "ChurchSpace", "WarehouseTake"] as const;

/** Every `ClientState` in the corpus, for tests that assert something true of
 *  all of them — a leak check, or a render that must not throw.
 *
 *  **The two SPECTATOR payloads are in it** (M6.3), which is the list doing its
 *  job rather than an exception to it: a property asserted over "every payload"
 *  that quietly skipped the two whose `seat` is null would be silent about
 *  exactly the shape this cycle added. The mask row in `contract.test.ts`
 *  branches on `seat === null` instead — a watcher unmasks NO seat, which is
 *  the same rule read at zero. */
export const EVERY_STATE: readonly ClientState[] = [
  wire.options, wire.optionsMany, wire.options4p, wire.optionsPriced, wire.pick, wire.retreat,
  wire.retreatAnswered, wire.placeA, wire.placeASelected,
  wire.placeB, wire.placeBSelected, wire.gameOver, wire.watching, wire.locked,
  wire.refused, wire.feeNonzero,
  wire.choiceAssistantMove, wire.choiceChurchSpace, wire.choiceWarehouseTake,
  wire.spectator, wire.spectatorOver,
];
