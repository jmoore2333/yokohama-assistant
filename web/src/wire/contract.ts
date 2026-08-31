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
  DecisionModel,
  Breakdown,
  Explanation,
  FrameModel,
  Health,
  IndexModel,
  Labels,
  LabsPanel,
  LogPanel,
  OptionRow,
  Outcome,
  Placing,
  PlayerModel,
  PortsPanel,
  Preview,
  PromptState,
  Provenance,
  Ranked,
  RefusalRow,
  SuggestOk,
  SuggestResponse,
  Term,
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
import error from "./error.json";
import feeNonzero from "./fee_nonzero.json";
import gameOver from "./game_over.json";
import health from "./health.json";
import hiddenHand from "./hidden_hand.json";
import labels from "./labels.json";
import labs from "./labs.json";
import locked from "./locked.json";
import log from "./log.json";
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
import preview from "./preview.json";
import refused from "./refused.json";
import retreat from "./retreat.json";
import retreatAnswered from "./retreat_answered.json";
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
const _hiddenHand: Widen<ClientState> = hiddenHand;
const _locked: Widen<ClientState> = locked;
const _feeNonzero: Widen<ClientState> = feeNonzero;
const _choiceAssistantMove: Widen<ClientState> = choiceAssistantMove;
const _choiceChurchSpace: Widen<ClientState> = choiceChurchSpace;
const _choiceWarehouseTake: Widen<ClientState> = choiceWarehouseTake;
const _refused: Widen<ClientState> = refused;
const _labels: Widen<Labels> = labels;
const _preview: Widen<Preview> = preview;
const _health: Widen<Health> = health;
const _error: Widen<ApiErrorBody> = error;
const _ports: Widen<PortsPanel> = ports;
const _labs: Widen<LabsPanel> = labs;
const _areas: Widen<AreasPanel> = areas;
const _areas4p: Widen<AreasPanel> = areas4p;
const _log: Widen<LogPanel> = log;
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
const _pHiddenHand: PromptOf<"options"> = hiddenHand.prompt;
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
const _pWatching: PromptOf<"watching"> = watching.prompt;
const _pLocked: PromptOf<"locked"> = locked.prompt;
// `reveal` has no fixture — live mode only; see `wire_fixtures.rs`'s exemption.

void [
  _pOptions, _pOptionsMany, _pOptionsPriced, _pOptions4p, _pRefused, _pHiddenHand,
  _pPick, _pRetreat, _pRetreatAnswered, _pPlaceA, _pPlaceASelected, _pPlaceB,
  _pPlaceBSelected, _pGameOver, _pWatching, _pLocked, _pFeeNonzero,
  _pChoiceAssistantMove, _pChoiceWarehouseTake, _pChoiceChurchSpace,
];

// `noUnusedLocals` is on, and these exist to be checked rather than read. One
// reference keeps the compiler's other rule from deleting the check.
void [
  _options, _options4p, _optionsMany, _optionsPriced, _pick, _retreat, _retreatAnswered, _placeA, _placeASelected, _placeB, _placeBSelected,
  _gameOver, _watching, _hiddenHand, _locked, _refused, _labels, _preview, _health, _error,
  _ports, _labs, _areas, _areas4p, _log, _artManifest, _feeNonzero,
  _choiceAssistantMove, _choiceChurchSpace, _choiceWarehouseTake,
  _suggest, _why, _whyRefused, _whyRefusals,
  _suggestOk, _whyOk, _whyProvenance, _whyRanked, _suggestTerm, _refusedRow,
  _indexOrders, _indexTechs, _indexGoals, _indexSites, _indexFiltered, _result, _placing,
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
  "id", "mode", "seed", "seat", "to_move", "your_turn", "frame", "prompt",
  "messages", "over", "end_triggers", "deck_remaining", "end_game_triggered",
  "scores", "warnings",
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
const LABELS = [
  "area_types", "phases", "phase_chain", "tech_effects", "achievements", "features",
] as const;
const HEALTH = ["server_version", "semantics_rev", "rules_hash"] as const;
const PREVIEW = ["seed", "deals"] as const;
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
  "contestant", "controller_version", "evaluator_version", "index", "seat",
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
const _pv: KeysOf<Preview, typeof PREVIEW> = PREVIEW;
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
void [_cs, _fr, _de, _or, _pl, _ti, _tr, _po, _la, _ar, _lo, _lb, _he, _pv, _er,
      _sr, _so, _wr, _wo, _ex, _tm, _pr, _rk, _rf, _ix, _oc, _pc, _bd];

/** The key lists, for the runtime half in `contract.test.ts`. */
export const KEYS = {
  clientState: CLIENT_STATE,
  frame: FRAME,
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
  preview: PREVIEW,
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
  hiddenHand: as<ClientState>(hiddenHand),
  locked: as<ClientState>(locked),
  feeNonzero: as<ClientState>(feeNonzero),
  choiceAssistantMove: as<ClientState>(choiceAssistantMove),
  choiceChurchSpace: as<ClientState>(choiceChurchSpace),
  choiceWarehouseTake: as<ClientState>(choiceWarehouseTake),
  refused: as<ClientState>(refused),
  labels: as<Labels>(labels),
  preview: as<Preview>(preview),
  health: as<Health>(health),
  error: as<ApiErrorBody>(error),
  ports: as<PortsPanel>(ports),
  labs: as<LabsPanel>(labs),
  areas: as<AreasPanel>(areas),
  areas4p: as<AreasPanel>(areas4p),
  log: as<LogPanel>(log),
  artManifest: as<ArtManifest>(artManifest),
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
 *  all of them — a leak check, or a render that must not throw. */
export const EVERY_STATE: readonly ClientState[] = [
  wire.options, wire.optionsMany, wire.options4p, wire.optionsPriced, wire.pick, wire.retreat,
  wire.retreatAnswered, wire.placeA, wire.placeASelected,
  wire.placeB, wire.placeBSelected, wire.gameOver, wire.watching, wire.locked,
  wire.refused, wire.hiddenHand, wire.feeNonzero,
  wire.choiceAssistantMove, wire.choiceChurchSpace, wire.choiceWarehouseTake,
];
