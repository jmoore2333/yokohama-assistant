import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import { Abandon } from "./Abandon";
import { BackUpOne } from "./BackupControls";
import {
  api,
  ApiError,
  bounded,
  BOUND_MS,
  displayName,
  isLocal,
  isSignedOut,
  LOCAL_USER,
  onUnauthorized,
  rememberServerFacts,
  RUN_CAP,
  useArchiveFact,
} from "./api";
import { rememberArchiveOff } from "./local/facts";
import { noteGameOver, onArchived, recordLanded, startOutbox } from "./outbox";
import { IdentityCorner, LobbyBand } from "./Band";
import { Door } from "./Door";
import { FrontDoor, multiplayerOffered } from "./FrontDoor";
import { projectLink } from "./buildValues";
import { useReadiness } from "./swRegister";
import { useMe } from "./whoami";
import { Board } from "./Board";
import { seatClass } from "./Cards";
import { Chat } from "./Chat";
import { Dashboard } from "./Dashboard";
import { TurnAccount } from "./TurnAccount";
import { Decision } from "./Decision";
import { GameMenu } from "./GameMenu";
import { Dealing } from "./Dealing";
import { Lobby } from "./Lobby";
import { BrokenStore } from "./BrokenStore";
import { LockScreen, LOST_SENTENCE } from "./LockScreen";
import { localPair } from "./local/client";
import { useLocalHold, type LockState } from "./local/useLocalHold";
import { NewGame } from "./NewGame";
import { OpenTable } from "./OpenTable";
import { Refusal } from "./Refusal";
import { Unlock } from "./Unlock";
import { channelOf, navigate, parseRoute, useRoute, type Route } from "./route";
import { loadCadence, saveCadence, type Cadence } from "./playback";
import { useSignal, type End } from "./signal";
import { BoardsMenu, useSections } from "./BoardsMenu";
import { CARD_SURFACES, Surface, SURFACES, type SurfaceName } from "./Surfaces";
import { TableSections } from "./TableSections";
import { mayAbandon } from "./tables";
import {
  buildRun,
  dwellFor,
  hasBotStep,
  hasOpponentStep,
  heldChanges,
  walkReducer,
  type Run,
  type WalkAction,
  type WalkState,
} from "./walk";
import type {
  ArtManifest,
  ClientState,
  Events,
  HistoricalFrame,
  Labels,
  Outcome,
  PlayerColor,
  PlayerId,
  TableView,
} from "./types";

/** The positions `prev..=next` and the log's tail.
 *
 *  **`prev` is fetched too**, and that is the whole reason this takes a range
 *  rather than the reply's own count: a step's highlight is a DIFF, so the
 *  first step needs a full position on the side the reply came from, and the
 *  live payload is not one — it carries no port and no laboratory.
 *
 *  **ONE call to the run route, since M6.5b (R-6.5b.10)**: the route is
 *  half-open and the run ends on the LIVE frame, so `to` is `next + 1`; the
 *  server replays the record once for the whole run where one `at/{k}` per
 *  position replayed it from the start `next - prev + 1` times. A run longer
 *  than the cap is PAGED in cap-sized calls (a spectator reconnecting after a
 *  long absence — one call is the common case); the server refuses a longer
 *  ask with a 400 rather than clamping it, so a page is never silently short.
 *
 *  Outside the component because it closes over nothing: `api` is a module and
 *  `buildRun` is pure, so a hook would only make its identity churn. */
async function fetchRun(game: string, prev: number, next: number): Promise<Run> {
  const frames: HistoricalFrame[] = [];
  for (let from = prev; from <= next; from += RUN_CAP) {
    const to = Math.min(next + 1, from + RUN_CAP);
    frames.push(...(await api.run(game, from, to)).frames);
  }
  const log = await api.log(game, next - prev);
  return buildRun(prev, frames, log);
}

/** How a run REACHED this browser, and who is watching it (spec §5.2).
 *
 *  **Two facts that used to be one nullable seat, and separating them is the
 *  whole of this type.** `landed`'s fourth parameter was `PlayerId | null`
 *  where `null` meant "this is the reply to my own move" and therefore selected
 *  the bot gate. That reading has no room for a SPECTATOR: a watcher's run
 *  arrives through the signal and their viewer is genuinely absent, so the one
 *  encoding says "reply" about a run this browser did not cause and the walk
 *  never starts (spec §9.5 mutation 10). A tag and a payload cannot collide
 *  that way. */
export type Arrival = { via: "reply" } | { via: "signal"; viewer: PlayerId | null };

/** Fetch the finished game's record as text and hand it to the browser as a
 *  download named `<id>.json`. An object URL over a Blob of the very string, so
 *  nothing between the server's bytes and the file parses or re-serializes; the
 *  URL is revoked once the click has taken it. */
async function saveGameFile(game: string): Promise<void> {
  const text = await api.recordText(game);
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = `${game}.json`;
  // Attached for the click and revoked on the next tick: a detached anchor and
  // a synchronous revoke both work in Chromium and have both cancelled the
  // download in other engines.
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

/** Does this refusal carry the foreign-bot fact (M6.5c2, spec §2.4)? By the
 *  FIELD, never the sentence — `OpenTable`'s `hasStatus` rule. */
const isForeignBot = (body: unknown): boolean =>
  typeof body === "object" && body !== null && "foreign_bot" in body;

/** Is this the refusal of a stored pair the host will not rebuild — Task 4's
 *  guard in `games::live_handle`, a table and a record that disagree about
 *  which seats are bots (M6.5c2 cycle D)?
 *
 *  **By the body's SHAPE, since it carries no field of its own**: a 409 on the
 *  game read of a table that reads `playing`, with neither `status` (the
 *  three table-status 409s, which a re-open answers) nor `foreign_bot`. The
 *  layer pins that body as exactly `{"error": …}`
 *  (`a_pair_whose_table_and_record_disagree_about_a_bot_is_refused_in_one_409`),
 *  and at HEAD it is the only plain 409 that read can answer. The cost of the
 *  shape, stated: a plain 409 added to that read later lands on this screen
 *  too — its sentence and an Abandon — rather than on the lobby. **One other
 *  plain 409 has the shape, and cannot reach this screen** (the Task 11
 *  review's m-4): the local Worker answers a write it could not keep with
 *  `{"error": <taken over>}` on any method, and a GET writes when its rebuild
 *  pumps a bot. That is the lock's refusal, and the lock screen outranks
 *  `broken` when the screen is drawn; Take back re-opens the game, and `show`
 *  clears `broken`. */
const isUnrebuilt = (e: ApiError): boolean =>
  e.status === 409 &&
  typeof e.body === "object" &&
  e.body !== null &&
  !("status" in e.body) &&
  !("foreign_bot" in e.body);

export function App() {
  // **Which screen this is, as the URL** (spec §5.1). Read here and acted on in
  // one effect below `open`, so there is ONE place that turns a route into a
  // screen — three call sites deciding for themselves is how a hash and a screen
  // stop agreeing.
  //
  // **First, since M6.5b2**, because the CHANNEL is read off it and the door
  // is read off the channel: a screen drawn from this browser alone asks the
  // server nothing, including who it thinks we are.
  const route = useRoute();
  /** Which of the two channels this screen belongs to (R-6.5b2.1). The local
   *  one — the front door, the local create, a `local-` table — costs the
   *  Worker zero invocations, which is R-6.5b2.7 and is why three reads below
   *  are gated on it. */
  const channel = channelOf(route);
  // **The door, read before any screen the SERVER owns is chosen** (M6.1 §7.1).
  // Every `/api/games` route is a 401 without an identity, so who the server
  // thinks we are decides which of four screens this is — see the branch below
  // the hooks, which is where it can finally be acted on. It is not read at all
  // on the local channel (spec §2.2): `/api/me` is a protected path now, and
  // the front door needs no identity.
  const door = useMe(channel);
  const { refresh, signOut, signedOut } = door;
  // **The release's worker, registered here and reported on the door**
  // (M6.5b2, R-6.5b2.8). It is unconditional and belongs to the APP rather
  // than to the door, because a player who arrived straight at a game should
  // still get the release cached; what is door-only is the LINE about it. In a
  // development build, in a browser with no service workers, and on a page the
  // browser walk has flagged, `useReadiness` registers nothing and answers
  // `"none"` — so this costs every existing cell one render and nothing else.
  const readiness = useReadiness();
  // **Every 401 on this API routes here, and no screen has to remember to
  // (ruling R-7.1).** `api.ts` invokes this from inside `call`, before the
  // refusal is thrown — so it reaches the door whatever the caller then does
  // with the error, including `/result`'s catch, which discards it. Registered
  // in an effect and cleared on unmount: the module holds one handler, and a
  // handler left behind by a torn-down tree would move a door that is gone.
  useEffect(() => {
    onUnauthorized(signedOut);
    return () => onUnauthorized(null);
  }, [signedOut]);
  const [labels, setLabels] = useState<Labels | null>(null);
  const [art, setArt] = useState<ArtManifest | null>(null);
  /** What the SERVER keeps, as this browser last heard it (M6.5b2, spec §2.2).
   *
   *  **The HOOK, and no state of its own** (review M-9). The fact is filled at
   *  moments a render has already happened at, so it has to re-render the
   *  screens that read it — and a `useState` here beside `Abandon`'s own module
   *  read was two accessors for one fact, kept in agreement only because both
   *  read points happened to write it down. `useArchiveFact` is the one reader;
   *  `onServerFacts` is what redraws. `"unknown"` is what nothing-has-ever-
   *  answered looks like, and is deliberately not `"off"`. */
  const archive = useArchiveFact();
  /** Whether `GET /api/health` LANDED — the one thing this client asks about
   *  the server's reachability, as opposed to about its answers (M6.5b,
   *  R-6.5b.2).
   *
   *  `null` until it answers, which is a real third state: nothing has come
   *  back yet, and "can't reach the server" would be a claim this browser
   *  cannot make. It is the health read and not the door's because the door
   *  has answers of its own and only one of them is about reachability. */
  const [serverUp, setServerUp] = useState<boolean | null>(null);
  /** Bumped by "Try again" (state C), which re-runs the health read below —
   *  its latch is already released by the failure or the bound that drew the
   *  card. */
  const [healthTick, setHealthTick] = useState(0);
  /** "Try again" on a server that could not be reached (M6.5c2, state C): both
   *  reads again — the door's and the health — and the screen back to
   *  "connecting" until one answers. A health read that LANDED keeps its answer:
   *  only a failed one is forgotten. */
  const retry = useCallback(() => {
    setServerUp((up) => (up === false ? null : up));
    setHealthTick((n) => n + 1);
    refresh();
  }, [refresh]);
  const [state, setState] = useState<ClientState | null>(null);
  /** The table being DEALT, when there is one (M6.1 §7.4).
   *
   *  **A third screen, not a mode of the other two.** A table exists before its
   *  game does — `create` opens one at `status: "dealing"` and every session
   *  route refuses it with a 409 until the last seat has kept — so "no game on
   *  screen" is no longer the same question as "the setup form". `state` and
   *  this are never both set: `show` clears this, and the one place that sets it
   *  is `open`. */
  const [dealing, setDealing] = useState<string | null>(null);
  /** The table still FILLING ITS SEATS, when there is one (spec §5.4).
   *
   *  The fourth screen, and a sibling of `dealing` rather than a mode of it: an
   *  open table has no deal to show and no game to play, and what a caller does
   *  there is take a seat. `show` clears it, exactly as it clears `dealing`.
   *
   *  **The document `open` read is held beside the id** (M6.5c2 cycle D,
   *  dossier row 17), `unlock`'s shape: the screen draws it at once instead of
   *  reading the same table a second time on the way in. */
  const [openTable, setOpenTable] = useState<{ id: string; table: TableView } | null>(null);
  /** The table whose PASSWORD is being asked for, when there is one (spec §5.3).
   *
   *  The fifth screen, and a sibling of the other two rather than a mode of
   *  them: there is no game to draw and no seat to take, and what a caller does
   *  here is present a password so they may WATCH. The document is held beside
   *  the id because the face states what the table is — `Unlock` does not
   *  re-fetch what `open` has already read. `show` clears it exactly as it
   *  clears the other two. */
  const [unlock, setUnlock] = useState<{ id: string; table: TableView } | null>(null);
  /** A game in THIS BROWSER whose save no longer rebuilds (spec §4).
   *
   *  **The sixth screen, and it exists so a refusal is not a trap.** A
   *  `SEMANTICS_REV` bump stops a save resuming by design, and
   *  `Session::from_record` then refuses with a 500 carrying the engine's own
   *  sentence — a REPLY, never the module trapping, so nothing is restarted and
   *  nothing is recovered. What is left is still the player's: the bytes can be
   *  saved to a file and the game can be abandoned, neither of which rebuilds
   *  anything. Bouncing to the lobby with the sentence would have offered them
   *  neither. The table document is held beside the id for the same reason
   *  `unlock` holds one: the abandon control states what it is about, and this
   *  screen cannot re-read a game that will not open.
   *
   *  **And a game of EITHER host that this build's bot cannot continue**
   *  (M6.5c2, spec §2.4): the same screen, because the same things are left —
   *  the sentence, and the Abandon that archives it: the server's own delete
   *  archives a server game, and a local one is SENT to the archive first, as
   *  every other local Abandon is (R-6.5b.6) — the archive takes its record,
   *  because its check is a strict replay and asks nothing of the bot. That is
   *  what `foreign` is for: a save that no longer rebuilds is the one game the
   *  archive would refuse, so its Abandon deletes without sending.
   *
   *  **And a stored pair whose table and record disagree about a bot seat**
   *  (M6.5c2 cycle D, Task 4's guard): refused on every host before anything
   *  is replayed, it is a save that does not open here, with `foreign: false`
   *  for the same reason — the archive refuses that pair too. A server game
   *  that refuses for any OTHER reason is the server's record on a server
   *  whose other games are fine, and keeps the bounce it has always had. */
  const [broken, setBroken] = useState<{
    id: string;
    table: TableView;
    sentence: string;
    /** The refusal was the FOREIGN-BOT one, not a rebuild that failed. */
    foreign: boolean;
  } | null>(null);
  /** Where the archive put a local game this session, keyed by id (spec §2.5).
   *
   *  **Why it is remembered at all**: the local table document carries no
   *  stamp — the send never goes through the module — so nothing on disk says
   *  a copy has been made, and the control would otherwise invite a second
   *  send of bytes the server already holds. It lives for the page, so a trip
   *  to the lobby and back does not put the button back; a RELOAD does, and the
   *  server answers that second click `identical`. */
  const [archived, setArchived] = useState<Record<string, string>>({});
  /** A send to the archive is OUT. A ref for `inFlight`'s reason and not the
   *  darkened control's: the control darkens on the ANSWER, and two clicks in
   *  one tick are both dispatched before any answer exists. */
  const archiving = useRef(false);
  /** The table the game on screen belongs to — who sits where, and which of
   *  those seats are the caller's.
   *
   *  **Held because a payload cannot say it.** `ClientState` reports ONE seat's
   *  view and `SeatFacts` says only whether a hand is hidden, so nothing in a
   *  play payload distinguishes "somebody else is to move" from "another of MY
   *  seats is to move" — and those are two different screens. `my_seats` is the
   *  server's own answer to that, read once when the game is opened. */
  const [table, setTable] = useState<TableView | null>(null);
  const [surface, setSurface] = useState<SurfaceName | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /** **Which visit to the lobby the notice above is about.**
   *
   *  Every sentence shown over the lobby is raised together with the navigation
   *  that shows it — a refused `#/t/<id>`, a table abandoned under this screen,
   *  a seat taken between the draw and the click — so clearing on any route
   *  change would wipe it in the same tick it was set. This records the route
   *  the notice belongs to, and the route effect clears it the next time the
   *  route is something else. Without it a 403 about a table survived a trip to
   *  the form and back and sat in the lobby's strip describing nothing. */
  const noticeFor = useRef<Route["kind"] | null>(null);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // **The replay position, or `null` for "the present".** M4's whole state is
  // this one integer: the scrubber is a projection over a record the server
  // already holds, so nothing here is persisted and nothing is applied.
  const [scrubK, setScrubK] = useState<number | null>(null);
  const [past, setPast] = useState<HistoricalFrame | null>(null);
  // **The last run a reply added, and the walk over it (M5.5 spec §3.2).**
  //
  // `run` outlives the walk: the account goes on showing it until the next run
  // replaces it, which is what makes "replay" mean something after the walk has
  // ended. `walk` is the step on the table and whether the timer runs, and
  // `null` is "over" — the live state is what is on screen. `fresh` is true
  // from the moment a run lands until the player's next answer, and is what
  // keeps the finished turn's highlights on the live table (spec §3.2.4)
  // instead of clearing them the instant the walk finishes; WHICH marks those
  // are is `heldChanges`, not the last step's own diff.
  const [run, setRun] = useState<Run | null>(null);
  /** **The live position, off the newest run's LAST frame, and the game it is
   *  of** (cycle E, Task 7; R-E9, ruling T7-a). A run ends on the live frame
   *  (`fetchRun`'s `to` is `next + 1`), and that frame carries the port and
   *  the laboratory `build_ports`/`build_labs` built for the position — the
   *  same two builders `…/ports` and `…/labs` answer with — so the sections
   *  draw them from here and ask neither route after a move. Held apart from
   *  `run`, which the screen clears for its own reasons: a cleared account is
   *  not a board that moved. */
  const [liveFrame, setLiveFrame] = useState<{ game: string; frame: HistoricalFrame } | null>(null);
  /** **The count a run is being fetched for, while it is** (R-E9). Set by
   *  `landed` BEFORE its first `await` — so it lands in the same render as the
   *  `state` that grew, which every caller sets in the same synchronous block —
   *  and cleared when the run lands or fails. It is what stops the sections
   *  asking for two boards in the gap between the new state and its run; a
   *  run that failed clears it with no live frame, and they ask once. */
  const [runFor, setRunFor] = useState<{ game: string; events: number } | null>(null);
  const [walk, setWalk] = useState<WalkState | null>(null);
  const [fresh, setFresh] = useState(false);
  const [cadence, setCadence] = useState<Cadence>(() => loadCadence());
  const dispatch = useCallback(
    (a: WalkAction) => setWalk((w) => (w && run ? walkReducer(w, a, run.steps.length - 1) : w)),
    [run],
  );
  // **ONE timer, and `dwellFor` is the whole of the timing rule.** A second
  // clock anywhere — a per-step `setInterval`, a transition that also advances —
  // is how a playback ends up at a different step than the one it is lighting.
  // The effect re-runs on every action because `walk` is a new object each
  // time, so each step gets its own fresh dwell and the cleanup cancels the
  // one before it.
  useEffect(() => {
    if (!run || !walk || !walk.playing) return;
    const step = run.steps[walk.at];
    if (!step) return;
    const t = setTimeout(() => dispatch({ type: "tick" }), dwellFor(step, cadence));
    return () => clearTimeout(t);
  }, [run, walk, cadence, dispatch]);
  // **Which boards are on the table, and it is a per-BROWSER choice.** The set
  // is not game state and is deliberately not sent anywhere: `sections.ts`
  // keeps it in `localStorage`, a storage that throws reads as the default, and
  // a game opened on another machine opens with the default set.
  const [enabled, toggleSection] = useSections();
  // The guard against a SECOND request going out while the first is in flight.
  //
  // A ref and not the `busy` state, because state is what makes the bug: React
  // batches, so two clicks in the same tick both read `busy === false` and both
  // post. Two `/input` posts are then applied in whatever order the server's
  // per-game mutex takes them, and whichever RESPONSE settles last becomes the
  // client's truth. Measured on a double-clicked placement toggle: the client
  // showed "Selected: A03" while the server held an empty selection, and
  // Confirm then advanced the turn having placed nothing. `busy` still drives
  // the disabled styling; this decides whether the request happens at all.
  const inFlight = useRef(false);

  // **Which game is on screen, as a ref, because a reply is judged against it
  // inside a closure that was created before the switch.**
  //
  // Measured on the live client (final review I1): with `/input` delayed 3s,
  // switching games from the menu mid-request showed the new game after 181ms
  // and then reverted to the old one at 3,104ms when the reply landed — banner,
  // board and `state.id` — the chosen switch silently undone. A state variable
  // cannot answer this: `send` closed over the state it was called with, and
  // reading `state` there reads the game the request belongs to, never the game
  // the player is now looking at. So the id is kept in a ref that every change
  // of game updates, and `send` drops anything that no longer matches it.
  const gameId = useRef<string | null>(null);
  /** The table on screen — as a game, a deal or an open table — or `null`.
   *
   *  **A ref, and NOT the three states it mirrors** (spec §5.1). The route
   *  effect below has to ask "is this table already on screen?", and asking it
   *  of `dealing` and `openTable` puts those in the effect's dependency list —
   *  which fires it in the window between a table being put on screen and the
   *  `hashchange` for it arriving. Measured: a create set `dealing`, the effect
   *  ran while `route` still read `#/new`, its own else-branch cleared the deal,
   *  and the hashchange then opened the table a SECOND time (two `GET …/table`,
   *  two deals). The route decides the screen; the screen does not re-decide the
   *  route. */
  const tableOnScreen = useRef<string | null>(null);
  /** The state the SIGNAL's handler reads, as a ref (ruling R-9.1).
   *
   *  The handler can now run a second pass for a frame that arrived under the
   *  first, and a closure's `state` is whatever it was when that closure was
   *  built — two events old by then. Written during render, like `signal.ts`'s
   *  own `src.current`, and written again at the assignment inside the handler
   *  so a pass that follows one which did not await cannot read a stale count. */
  const stateNow = useRef<ClientState | null>(null);
  /** A signal-driven arrival is in flight, and the newest frame that landed
   *  under it. Refs and not state, for `inFlight`'s reason: they are read by
   *  the pass that is running, not by a render. */
  const arriving = useRef(false);
  const pending = useRef<Events | null>(null);
  /** A move of THIS browser's is in flight (M6.4, R-6.4.4).
   *
   *  A ref for `arriving`'s reason, and the guard the held stream makes
   *  necessary: with `active` no longer falling to false during the player's
   *  own turn, the server's own bump for the move being posted reaches this
   *  browser WHILE the reply is in flight. Handled at once it would read the
   *  game again against a count the reply is about to change, and walk this
   *  browser's own move as somebody else's turn. So the frame is parked, and
   *  `send`'s `finally` compares it against what the reply produced. */
  const posting = useRef(false);
  /** The table's newest message number, off every frame (spec §6.2). Passed to
   *  the panel as its `count`: above what the panel holds it is a fetch when
   *  open and a badge when folded, at or below it is nothing at all. */
  const [chatCount, setChatCount] = useState(0);
  /** The table's chat revision off the same frame (M6.5c2 cycle D, R-D4): the
   *  panel's `rev`, which moving further than the count is a delete. Reset
   *  with the count on a switch. */
  const [chatRev, setChatRev] = useState(0);
  stateNow.current = state;
  /** The table `open` is CURRENTLY fetching, or `null`.
   *
   *  A ref and not a state, for the reason `inFlight` above is one: this is read
   *  by an effect that runs in the same tick as the navigation that started the
   *  request, and a state set would not be visible until the render after. See
   *  the route effect below `open`. */
  const openingId = useRef<string | null>(null);
  /** Events `prev..next` landed: build the run, and walk it when it holds a
   *  move this browser did not make.
   *
   *  Called INSIDE the busy window, so the deck stays disabled until the walk
   *  is ready to show rather than offering the next prompt over a table that is
   *  about to rewind.
   *
   *  **A COUNT, not the payload that carried it (M6.1 §6).** Until the poll
   *  there was exactly one way events arrived — a reply to something this
   *  browser sent — so taking the `ClientState` was the same thing as taking
   *  its `events`. The signal is the second way, and what IT knows is a number:
   *  the frame that goes with it is fetched separately, and threading a
   *  synthetic `ClientState` through here only to read one field off it would
   *  be a payload built to satisfy a signature.
   *
   *  **A run that added NOTHING clears the run as well as `fresh` (ruling
   *  R-P5).** `events` is legitimately non-monotone: a picker step applies no
   *  event, a refusal none, and an undo POPS them. Leaving the previous run
   *  standing there would keep an account of an earlier turn on screen in place
   *  of the prose that answers the click just made.
   *
   *  **`arrival` is what decides whether the run PLAYS, and the two answers
   *  are different questions (ruling R-9.1).** A run that came back as the
   *  reply to this browser's own move arrives `via: "reply"`: the only other
   *  mover there can be is a bot seat it is playing against, so `hasBotStep` is
   *  the whole question and a vs-bot game is unchanged. A run that arrived
   *  through the SIGNAL carries the seat it is being watched from, and every
   *  step that seat did not play is worth watching — because **a room is
   *  people**, and a gate on `moverIsBot` would leave the opponent's turn
   *  SNAPPING onto the board at exactly the table the walk was built for.
   *
   *  **And a SPECTATOR's viewer is `null`, which is why this is a tagged union
   *  and not a nullable seat** (spec §5.2). A watcher holds no seat, so the old
   *  encoding had their every run read as "the reply to my own move" — the bot
   *  gate, at a table of people, which never fires. Every served run walks for
   *  them instead. */
  const landed = useCallback(
    async (game: string, prev: number, next: number, arrival: Arrival = { via: "reply" }) => {
      setFresh(false);
      if (next <= prev) {
        setRun(null);
        setWalk(null);
        return;
      }
      // **Marked before the first `await`** (R-E9): see `runFor`. The mark is
      // this call's own object, so the `finally` clears it only if no later
      // run has replaced it.
      const mark = { game, events: next };
      setRunFor(mark);
      try {
        const r = await fetchRun(game, prev, next);
        // The same rule the reply itself is judged by: a run for a game the
        // player has left is dropped, never shown over the game they are on.
        if (gameId.current !== game) return;
        const plays =
          arrival.via === "reply" ? hasBotStep(r) : hasOpponentStep(r.steps, arrival.viewer);
        setRun(r);
        setFresh(true);
        setWalk(plays ? { at: 0, playing: true } : null);
        // The live frame, and with it the port and the laboratory (R-E9).
        setLiveFrame({ game, frame: r.steps[r.steps.length - 1]!.after });
      } finally {
        setRunFor((f) => (f === mark ? null : f));
      }
    },
    [],
  );

  /** Put a game on screen — the ONE way `state` changes identity, so the ref
   *  and the state cannot disagree. `null` is the setup screen.
   *
   *  The walk goes with the game: a run is an account of a turn in the game
   *  being left, and rows about it over a different board describe nothing.
   *
   *  `opening` is true only for a game just CREATED. A game whose start seat is
   *  a bot has already played by the time the create replies, and those events
   *  are a run like any other — the only one whose `prev` is 0. A saved game
   *  reopened passes false: its events are history, not something that just
   *  happened. */
  const show = useCallback(
    (s: ClientState | null, opening = false) => {
      gameId.current = s?.id ?? null;
      tableOnScreen.current = s?.id ?? null;
      setState(s);
      // The dealing screen goes with the game being shown, and with the lobby
      // too: a table id left behind here would draw the dealing screen again the
      // next time the player left for the lobby. The TABLE goes with it for the
      // same reason — `open` puts the new one back in the same tick.
      setDealing(null);
      setOpenTable(null);
      setUnlock(null);
      // The refusal about a save that would not rebuild goes with the game it
      // was about, exactly as the other four screens do.
      setBroken(null);
      setTable(null);
      setRun(null);
      // A game put on screen afresh draws its boards from its own reads, never
      // from a frame this screen held for it before (R-E9).
      setLiveFrame(null);
      setWalk(null);
      setFresh(false);
      // **The message count goes with the game too** (final review, minor 7).
      // It is the TABLE's newest number and the panel is re-keyed per game, so
      // a count carried across a switch is the old room's number read against
      // the new room's empty list: a folded panel flashes it as "N new", and if
      // it is above the new room's own `last` the count rule asks one empty
      // delta for it. The new game's first frame puts the real number back
      // milliseconds later; this is what the screen says in between.
      setChatCount(0);
      setChatRev(0);
      if (opening && s && s.events > 0) {
        // The busy window is opened by hand here, because this run has no
        // request of its own to ride inside. It is what the browser harness
        // waits on before reading a screen — see `data-busy` on the root — and
        // the refusal goes where every other one goes rather than into an
        // unhandled rejection.
        setBusy(true);
        void landed(s.id, 0, s.events)
          .catch((e) => setError(e instanceof ApiError ? e.message : String(e)))
          .finally(() => setBusy(false));
      }
    },
    [landed],
  );

  /** **Land on a first screen, with the server's sentence if there is one.**
   *
   *  The paths that put a player back on a first screen against their will — a
   *  refused `#/t/<id>`, a game that stopped answering, a deal or an open table
   *  that went out from under them, a hand-over that found the table filled —
   *  say the same two things in the same order, and they also have to agree
   *  with `noticeFor` about which visit the sentence belongs to. One callback,
   *  so that agreement is not several separate things to keep true.
   *
   *  **There are TWO first screens since M6.5b2** (R-6.5b2.1), and which one a
   *  game belongs to is not a preference: a local game is listed on the DOOR
   *  and a table of the server's in the LOBBY, so a sentence about a game shown
   *  on the other screen would be an explanation with nothing under it. The
   *  destination rides through `noticeFor` too, because the route effect clears
   *  a notice as soon as the route is not the one it was raised for.
   *
   *  A `null` or absent sentence CLEARS, which is what leaving by choice
   *  should do. */
  const leave = useCallback((to: "door" | "lobby", sentence?: string | null) => {
    setError(sentence ?? null);
    noticeFor.current = sentence == null ? null : to;
    navigate({ kind: to });
  }, []);
  /** Out to the SERVER's first screen. Every control in this client that says
   *  "Lobby" comes here, whatever it is leaving: the word on the button is the
   *  promise, and the lobby carries a link back to the door. */
  const toLobby = useCallback((sentence?: string | null) => leave("lobby", sentence), [leave]);
  /** Out to the DOOR, where the games in this browser are listed — the landing
   *  for a local game that ends, is abandoned, or is asked for and no longer
   *  there. */
  const toDoor = useCallback((sentence?: string | null) => leave("door", sentence), [leave]);
  /** Where a LOCAL screen's way out goes, and what it says (M6.5c2, spec §2.2;
   *  M6.5b2 close §6.3 item 5).
   *
   *  A build that serves a lobby keeps the four controls as they were. Under
   *  `VITE_MULTIPLAYER=false` there is no lobby to go to — its game routes do
   *  not exist — so each leads to the door, where this browser's games are
   *  listed, and says so in the local screens' own crumb. Read on every render,
   *  for `multiplayerOffered`'s reason: a cell stubs the build value. */
  const localExit: { to: "lobby" | "door"; label: string } = multiplayerOffered()
    ? { to: "lobby", label: "Lobby" }
    : { to: "door", label: "On this device" };

  /** Open a table BY ID — the one door into a game, and the reason it asks
   *  `GET …/table` first (M6.1 §7.4).
   *
   *  A table exists before its game does. While it is dealing every session
   *  route answers 409 by design, so a caller that went straight to
   *  `GET /api/games/{id}` would show a refusal for exactly the tables this
   *  cycle exists to open — and each of the three places that open one (the
   *  create, a saved row, the Game menu) would have to know that separately.
   *  One `open` asks what the table IS and picks the screen.
   *
   *  `opening` is true only where the events already on the record have just
   *  HAPPENED: the last keep finalizes the table, and a bot on the start seat
   *  has played by the time the dealing screen hands over. A saved game
   *  reopened passes false — its events are history, not something that just
   *  happened. */
  const open = useCallback(
    async (id: string, opening = false) => {
      // **Where the player was, for a refusal to put back** (M6.5c2 cycle D,
      // dossier row 17). A switch from the Game menu starts on another game's
      // address; a refusal left the hash naming the table that refused while
      // that game stayed drawn, and the next run of the route effect opened the
      // refused table again and took the player to the lobby. Only a GAME still
      // on screen is put back — the one thing a refusal here must not cost.
      const from = parseRoute(window.location.hash);
      const back = from.kind === "table" && from.id !== id && tableOnScreen.current === from.id ? from : null;
      // **The hash is set FIRST, before anything is fetched** (spec §5.1). A
      // table opened from a row, from the Game menu or by a create is the same
      // screen as one opened from a link, so the address is written the same
      // way.
      navigate({ kind: "table", id });
      // **The guard against opening the same table twice.** `navigate` changes
      // the hash synchronously, so the route effect runs while this call is
      // still awaiting its first response — and every "is it already open?"
      // test it can make is still false. The ref is the only thing that knows a
      // request is in flight for this id.
      openingId.current = id;
      /** **Does the address still name this table?** Asked after every await
       *  (M6.5c2 cycle D, dossier row 17): a player who pressed Back while a
       *  read was out is somewhere else by the time it answers, and drawing the
       *  table then put a board under `#/lobby`. Where the address has moved
       *  on, this open is over: nothing is drawn and nothing is reported — the
       *  route effect has already answered the address the player chose. */
      const here = () => {
        const now = parseRoute(window.location.hash);
        return now.kind === "table" && now.id === id;
      };
      try {
        // `doc`, not `table`: the state of the same name is what this puts BACK,
        // and one name for both would read as a self-assignment.
        const doc = await api.table(id);
        if (!here()) return;
        if (doc.status === "open") {
          // Nobody has dealt yet and there may be no seat here for this caller
          // at all. `show(null)` first, for the same reason the dealing arm
          // does it.
          show(null);
          setOpenTable({ id, table: doc });
          tableOnScreen.current = id;
          return;
        }
        if (doc.status === "dealing") {
          // `show(null)` first, so a game being left takes its walk and its
          // refusal with it; the dealing id is set in the same tick.
          show(null);
          setDealing(id);
          tableOnScreen.current = id;
          return;
        }
        // **The WATCHER who has not presented the password yet** (spec §5.3).
        //
        // **`playing` is a RESTATEMENT here, not a guard** (review T8 #1): the
        // type has three arms and the two above have returned, so by this line
        // it can be nothing else. It is written because the condition it
        // states is real and is what the next reader needs — an OPEN locked
        // table answers `can_watch: false`, and a draft that tested the other
        // two terms alone drew a password box in place of the seats somebody
        // came to take (Task 4 review M-7). Keeping it means the arms above
        // can be reordered or narrowed without this one quietly widening.
        // `my_seats` empty because a seated caller is a player and never needs
        // to unlock anything; `can_watch` false because it is the server's own
        // answer to "may this caller read this table", and once it turns over
        // the game route answers them. (A finished table is `playing` too —
        // its game is over, its status is not.)
        //
        // `show(null)` first and the ref set after, which is the shape the two
        // arms above have: a game being left takes its walk with it, and the
        // route effect must still know a table is on screen or it would open
        // this one again.
        if (doc.status === "playing" && doc.my_seats.length === 0 && !doc.can_watch) {
          show(null);
          setUnlock({ id, table: doc });
          tableOnScreen.current = id;
          return;
        }
        // **A local save that no longer rebuilds is a SCREEN, not a bounce**
        // (spec §4). The engine refuses the rebuild with a 500 carrying its own
        // sentence; the row stays listed, the bytes stay saveable and the game
        // stays abandonable, and none of that is reachable from the lobby.
        let next: ClientState;
        try {
          next = await api.get(id);
        } catch (e) {
          if (!here()) return;
          if (isLocal(id) && e instanceof ApiError && e.status >= 500) {
            show(null);
            setBroken({ id, table: doc, sentence: e.message, foreign: false });
            tableOnScreen.current = id;
            return;
          }
          // **A game this build's bot cannot continue** (M6.5c2, spec §2.4;
          // R-6.5c2.10): not over, dealt a bot this build does not run. Every
          // game route answers it with a 409 carrying `foreign_bot`, on both
          // hosts. A SCREEN and not a bounce, for the broken save's reason.
          if (e instanceof ApiError && e.status === 409 && isForeignBot(e.body)) {
            show(null);
            setBroken({ id, table: doc, sentence: e.message, foreign: true });
            tableOnScreen.current = id;
            return;
          }
          // **A stored pair this host will not rebuild** (M6.5c2 cycle D; Task
          // 4's guard in `games::live_handle`): a table and a record that
          // disagree about which seats are bots, refused on every host with a
          // plain 409 before anything is replayed. It took the bounce every
          // other refusal takes; it is the same screen as the two above, because
          // the same things are left — the sentence, and an Abandon, which
          // never rebuilds. `foreign: false`, the broken save's rule: the
          // archive's `check_pair` refuses this very pair, so a local game is
          // deleted without being sent. See `isUnrebuilt` for why it is told
          // apart by the body's shape.
          if (e instanceof ApiError && isUnrebuilt(e)) {
            show(null);
            setBroken({ id, table: doc, sentence: e.message, foreign: false });
            tableOnScreen.current = id;
            return;
          }
          throw e;
        }
        if (!here()) return;
        show(next, opening);
        // AFTER `show`, which clears it: React batches both into one render, so
        // the last write is the one that stands.
        setTable(doc);
      } catch (e) {
        // Refused, and the player moved on while it was out: nothing to say
        // about a table nobody is looking at, and nothing to put back.
        if (!here()) return;
        // The game that was on screen stays, and so does its address — with no
        // history entry left pointing at the table that refused. **The cost,
        // stated** (cycle D's close `2026-09-29-web-frontend-m6.5c2d-close.md` §1, the Task 11 row: m-2): the switch pushed an entry and
        // this REPLACES it, so the history holds the game twice, and the first
        // Back changes nothing a player can see. `history.back()` would pop it,
        // but it arrives as a queued `popstate` racing the route effect; one
        // inert Back is the cheaper of the two.
        if (back) navigate(back, { replace: true });
        throw e;
      } finally {
        openingId.current = null;
      }
    },
    [show],
  );

  /** The local game this tab must OWN before it reads anything — the ROUTE's
   *  id and never `state.id` (P-5, spec §2.2).
   *
   *  **The order is the claim.** A cold `GET /api/games/{id}` on a local game
   *  PUMPS the bot inside the module, so a read taken before the lock answers
   *  can change a record another tab is writing. Deriving ownership from a
   *  `ClientState` would mean reading first and asking afterwards. `null` for
   *  every other screen, which the hook answers `"none"` to. */
  const lockId = route.kind === "table" && isLocal(route.id) ? route.id : null;
  const lock = useLocalHold(lockId);
  const { takeover: steal } = lock;
  /** The lock's answer FOR THE GAME THE ADDRESS NAMES, and `"none"` for any
   *  other — which is what every reader below is entitled to.
   *
   *  **The id check is the fix, and a reset inside the hook could not be**
   *  (review Important 1). React flushes one commit's passive effects in a
   *  single pass, so on the commit where the route moves from one local game
   *  to another, the route effect's closure holds the state rendered for the
   *  game being LEFT. That state is `"held"`, and reading it would admit the
   *  cold `GET` that PUMPS THE BOT in a record this tab does not own — the
   *  exact harm P-5 exists to stop. Asking whether the answer is ABOUT this
   *  game closes it, because the stale value carries the stale id with it. */
  const lockState: LockState = lock.id === lockId ? lock.state : "none";

  /** Take the game anyway — Take over from the tab that found it open
   *  elsewhere, Take back from the one it was stolen from.
   *
   *  **Who re-reads afterwards depends on which of the two this is, and the
   *  ref is what tells them apart.** Coming from Take over there is no game on
   *  screen: the route effect opens it the moment the lock says `"held"`, and
   *  a second `open` here would be the same table fetched twice. Coming from
   *  Take back the game IS on screen and the route has not changed, so that
   *  effect does nothing — and without this re-read the board would stay at the
   *  position it held when it lost the lock, missing every move the other tab
   *  made. */
  const retake = useCallback(async () => {
    const id = lockId;
    if (id === null) return;
    await steal();
    if (tableOnScreen.current === id) await open(id);
  }, [lockId, steal, open]);

  /** **The route decides the screen, in ONE place.**
   *
   *  Every way into a table — a lobby row, a create, the Game menu, a link
   *  somebody sent — goes through the hash, and this is what the hash then
   *  does.
   *
   *  **Two refs and no state, so the effect runs on the ROUTE changing and on
   *  nothing else.** `tableOnScreen` is what is already drawn; `openingId` is a
   *  request in flight, and without it a create would open its own table twice —
   *  `open` navigates before it fetches, so this effect runs while the first
   *  call is still awaiting `GET …/table` and every state-shaped test is still
   *  false. Both refs' own doc comments carry the measurement. */
  useEffect(() => {
    if (route.kind === "table") {
      // **A LOCAL table needs no door** (M6.5b, spec §2.3). The games in this
      // browser are answered by a module in this browser: nobody signs in to
      // reach them, and the gate that holds every other route back would leave
      // a `#/t/local-…` link dead on a page whose server cannot be reached —
      // which is the whole of what R-6.5b.2 promises.
      //
      // **Inside the table arm since Task 5**, and that is not a move for
      // tidiness: as a guard over the whole effect it also held back the tail
      // below, so a signed-out player leaving a local game for `#/` or
      // `#/new/local` kept the game on screen at an address that no longer
      // named it.
      if (!door.me?.user && !isLocal(route.id)) return;
      // **Nothing is read until the lock has answered FOR THIS GAME** (P-5,
      // spec §2.2) — the dealing screen included, since a keep is a write like
      // any other. While the answer is `"elsewhere"` this table is another
      // tab's and the screen below draws that instead; `"lost"` leaves the game
      // exactly where it is and waits for Take back.
      if (isLocal(route.id) && lockState !== "held") {
        // **And a game the address no longer names goes off the screen.** The
        // arm returns before `open`, which is what would otherwise have cleared
        // it — so without this the game being LEFT stays drawn, answerable, at
        // an address that names a different table. `tableOnScreen` is the test
        // rather than `state`, because the deal and the open table are on
        // screen without one.
        if (tableOnScreen.current !== null && tableOnScreen.current !== route.id) show(null);
        return;
      }
      if (tableOnScreen.current !== route.id && openingId.current !== route.id) {
        // **A refusal is about the table you were opening, so opening another
        // one clears it.** Without this a "you are not seated at this table"
        // follows the player into the next game they open and sits in its error
        // strip describing nothing on screen. Cleared HERE and not at the top of
        // the effect: the arm below is the one a refused open navigates TO, so
        // an unconditional clear there would wipe the sentence in the same tick
        // it was set — which is why that arm asks `noticeFor` first.
        setError(null);
        noticeFor.current = null;
        open(route.id).catch((e) => {
          // A table that does not exist, or is not this caller's to see. The
          // sentence is the server's and a first screen is where it is shown —
          // a bare screen at a `#/t/<id>` that answers nothing is the one place
          // a player cannot act from.
          //
          // **WHICH first screen is the id's own answer** (R-6.5b2.1): a
          // `local-` table is listed on the door and nowhere else, so a local
          // game that has been deleted would otherwise explain itself on a
          // screen its siblings are not on.
          const said = e instanceof ApiError ? e.message : String(e);
          if (isLocal(route.id)) toDoor(said);
          else toLobby(said);
        });
      }
      return;
    }
    // **A notice does not outlive the screen it was about.** It is raised
    // together with the navigation that shows it, so `noticeFor` says which
    // visit it belongs to and the clear happens on the NEXT route change —
    // clearing unconditionally here would wipe the sentence in the same tick a
    // refused table set it, which is the case the arm above exists for.
    if (noticeFor.current !== route.kind) {
      setError(null);
      noticeFor.current = null;
    }
    // `#/`, `#/lobby` or `#/new`: no game, no deal and no table on screen.
    if (tableOnScreen.current) show(null);
  }, [route, door.me?.user, lockState, open, show, toDoor, toLobby]);

  // **The final standing comes from `/result`, because `scores` cannot carry
  // it.** `ClientState.scores` is `{seat, total}[]` in SEAT order with no rank
  // and no `won`, so from it alone the inline block could print a colour and a
  // sorted list and would have to INFER a winner from the margin — which is the
  // exact inference `Index.test.tsx` exists to forbid, since two seats can tie
  // on points and be separated by the engine's own tie-break. `/result` carries
  // `Placing.won` and the per-category breakdown the terminal prints.
  useEffect(() => {
    if (!state?.over) {
      setOutcome(null);
      return;
    }
    let live = true;
    api
      .result(state.id)
      .then((o) => live && setOutcome(o))
      .catch(() => live && setOutcome(null));
    return () => {
      live = false;
    };
  }, [state?.over, state?.id]);

  /** Where the words on the cards last came from, so the SERVER's answer is
   *  asked for once and never overwritten by the module's afterwards. */
  const wordsFrom = useRef<"local" | "server" | null>(null);
  useEffect(() => {
    // **The words on the cards survive an unreachable API** (spec §2.3,
    // R-6.5b.8), and since M6.5b2 they are LOCAL-FIRST on the local channel
    // (spec §2.2). `/api/labels` is computed from the engine's registries, is
    // deployment-independent, and the corpus replay proves both hosts emit it
    // byte-identically — so on the front door and in a local game the module in
    // this tab's Worker is the answer, and the server is not asked at all.
    // The Worker is already running there: the door's own listing starts it.
    if (wordsFrom.current === "server") return;
    // **The effect's cleanup** (M6.5c2 cycle D, dossier row 56: it had none).
    // An answer that lands after the effect is gone — the channel changed, the
    // door signed in or out, the screen unmounted — is dropped rather than
    // set. And a read that had NOT landed gives its claim on `wordsFrom` back,
    // so the next run asks again: React's StrictMode runs this effect, cleans
    // it up and runs it again before any answer, and a claim kept across that
    // would have dropped the only read ever made. A read that DID land keeps
    // its claim, which is what keeps the server's words from being asked for
    // twice or overwritten by the module's.
    let live = true;
    let landed = false;
    const land = (words: Labels | null) => {
      landed = true;
      if (live) setLabels(words);
    };
    const cleanup = () => {
      live = false;
      if (!landed) wordsFrom.current = null;
    };
    if (channel === "local") {
      if (wordsFrom.current === "local") return;
      wordsFrom.current = "local";
      api
        .labels("local")
        .then(land)
        .catch(() => land(null));
      return cleanup;
    }
    // **Past the door, the SERVER is asked** — it is the one that may have been
    // updated — under a bound, and the Worker answers when that read fails OR
    // times out. The bound is what makes the fallback reachable at all: a
    // hanging read never rejects, so a `.catch` alone would wait for ever and
    // the cards would keep their raw serde names for the life of the page.
    //
    // Gated on a signed-in session, because a caller the server will not name
    // is a caller its routes refuse: asking would buy a refusal and the
    // module's words are already on screen.
    if (!door.me?.user) return;
    wordsFrom.current = "server";
    bounded(api.labels(), BOUND_MS)
      .catch(() => api.labels("local"))
      .then(land)
      .catch(() => land(null));
    return cleanup;
  }, [channel, door.me?.user]);

  useEffect(() => {
    // **The art manifest is a FILE and no longer a route** (M6.5b2, spec §2.2).
    // `/api/art` read `assets/art/manifest.json` and re-serialized it, and the
    // identical file is already served as a static asset beside the pictures it
    // names — so the client fetches the file, and the door costs the Worker
    // nothing for it (R-6.5b2.7: static asset requests are free and unmetered).
    //
    // A 404 here means the server has no cropped art. That is a supported
    // state, not a failure: every card falls back to its typed values, which is
    // exactly the surface that existed before there was any art.
    fetch("/art/manifest.json")
      .then((r) => (r.ok ? (r.json() as Promise<ArtManifest>) : Promise.reject(new Error("no art"))))
      .then(setArt)
      .catch(() => setArt(null));
  }, []);

  /** Whether this page has asked the server what it is like. Once per load, on
   *  the first screen that is the server's.
   *
   *  A read that FAILS is not retried here and does not need to be: nothing is
   *  recorded, so `ensureArchiveFact` asks again at the next read point, and
   *  `Lobby`'s own `onReached` is what puts the sections back if a listing
   *  lands while this one did not. */
  const askedFacts = useRef(false);
  useEffect(() => {
    // **Entering the multiplayer channel is the first of the three read points**
    // (M6.5b2, spec §2.2). It used to be a boot read, which made it a cost every
    // single-player visitor paid; the screens that need what it says are the
    // server's, so this is where it is asked.
    //
    // **Everything it carries is one statement**: which change transport this
    // deployment serves (M6.3 §5.1), whether the house chat answers and how
    // long a message may be (M6.4), which door it reads, and which archive it
    // keeps. They are read ONCE and kept in `local/facts.ts` rather than in
    // state, because a hook reads the transport and must not re-run when it is
    // set. A health that does not answer leaves the defaults, which costs one
    // failed open or one refused request — never a screen that never moves.
    //
    // **And whether it LANDED is a second question, bounded** (R-6.5b.2,
    // review fix 1). The ruling names two ways for the read to fail — the API
    // refusing, and the API not answering — and only the first of them
    // rejects: a `fetch` accepted by a server that then says nothing settles
    // neither way, so an unbounded read left the lobby reading `connecting…`
    // for the life of the page for exactly the case the ruling is about.
    //
    // **The unbounded read keeps its own chain**, so a server that answers
    // LATE is still a server that answered: it sets the facts and puts the
    // sections back, where the race's loser would have been dropped on the
    // floor.
    if (channel !== "multiplayer" || askedFacts.current) return;
    askedFacts.current = true;
    const health = api.health();
    health
      // **Reachability first, then the payload.** A body this client cannot
      // make facts of is still evidence the server is there, and the throw
      // that a malformed one raises must not cost the screen its listings.
      .then((h) => {
        setServerUp(true);
        // Held in memory AND written down with the time it was taken, so the
        // next page load starts from the answer rather than from the guesses.
        // The screens that read it redraw through `useArchiveFact`.
        rememberServerFacts(h);
      })
      // **The latch is RELEASED on a failure** (review M-1). The ref is set
      // before the read so two renders cannot both ask; leaving it set after a
      // read that never landed left the transport, the chat caps, the door's
      // name and the archive at their defaults for the life of the page, even
      // after the server came back and the listings started landing. A read
      // that failed recorded nothing, so there is nothing to keep.
      .catch(() => {
        askedFacts.current = false;
      });
    // **Only from `null`** (whole-branch review, Minor #2). The bound answers
    // the question "has anything come back yet?", and a LISTING that landed
    // is an answer: `Lobby`'s `onReached` sets this true the moment one does,
    // whatever the health read is doing. An unconditional `false` here took a
    // working lobby off the screen and replaced it with "not connected"
    // whenever the health read alone was the slow one.
    //
    // **And the latch goes at the bound too** (M6.5c2, state C): a read that
    // has not landed by now has recorded nothing, exactly as a rejected one
    // has, and holding the latch for it left "Try again" re-running this
    // effect onto the early return above — "Connecting to the server…" over no
    // health read at all. A late answer still lands through the chain above.
    bounded(health, BOUND_MS).catch(() => {
      askedFacts.current = false;
      setServerUp((v) => v ?? false);
    });
  }, [channel, healthTick]);

  useEffect(() => {
    // **A local game reaching game over is the second read point** (spec §2.2).
    // It is the moment the answer is first USED on this channel: "Send to the
    // archive" is drawn only where the deployment keeps one, and with no boot
    // read the fact would otherwise stay at its `"off"` default for ever — the
    // measured bug of spec §0.3, which this closes for an ONLINE single-player
    // session as well as an offline one.
    //
    // **At most one request per HOUR per finished game OPENED** (whole-branch
    // review, false sentence 3), which is wider than "once per browser" and is
    // the accounting R-6.5b2.7 actually rests on: this effect keys on
    // `state.over && isLocal`, so re-opening a finished local game fires it
    // again, and `ensureArchiveFact` re-reads whenever the remembered answer is
    // over an hour old. Bounded, but bounded by the hour and not by the
    // browser.
    if (!state?.over || !isLocal(state.id)) return;
    // **The hook is `noteGameOver`, and the read is inside it** (M6.5b2, spec
    // §2.6). It asks what this deployment keeps, offers the game to the archive
    // where there is one, and writes the reason down where the send could not
    // be made or could not yet be attempted. It is idempotent against a reload
    // — the game is still over on the next visit and this fires again — and
    // the receipt is what makes it so.
    void noteGameOver(state.id);
  }, [state?.over, state?.id]);

  // **The outbox's three triggers** (spec §2.6): the browser's `online` event,
  // this launch, and the player's own button on the door. Mounted here rather
  // than at module load, for the reason every other listener in this client is:
  // a module that wired a window listener when it was imported could never take
  // it off again.
  useEffect(() => startOutbox(), []);

  // **And what the hook's own send landed as** (review M-6). `archived` is what
  // darkens "Send to the archive", and before this the only send it knew about
  // was the one its own button made — so a game the game-over hook had already
  // copied still offered the control, and clicking it bought the server's
  // `identical`. The path is the SERVER's, in the same words the button's own
  // reply is reported in.
  useEffect(
    () =>
      onArchived((id, path) => setArchived((a) => (a[id] ? a : { ...a, [id]: `archived under ${path}` }))),
    [],
  );

  const arrive = useCallback(
    async (e: Events) => {
      // **The count FIRST, before any check** (spec §6.2). A number is harmless
      // to apply at once — it costs a badge and never a request — and a frame
      // parked by the guard below must not hold the badge back with it, since a
      // message arriving while this browser posts is exactly the case the
      // parking is for.
      setChatCount(e.chat);
      setChatRev(e.chat_rev);
      // **The handler guards its own re-entrancy, and that is reachable only
      // now that the stream stays open** (ruling R-9.1). The server writes a
      // frame per move and a room can move twice while one `GET …/{id}` is in
      // flight; re-entered, this would read the same stale count, fetch the
      // game again and walk the same range twice — two accounts of one turn,
      // one drawn over the other. So the NEWEST frame that lands during a run
      // is remembered rather than run beside it, and the loop below takes it
      // once the first has landed. **The newest and not all of them** —
      // `pending` is one slot, and an earlier frame it overwrites is not lost
      // because `events` is a CUMULATIVE count: the newest frame's number
      // covers every frame before it, and the loop compares that number
      // against the state the first pass produced. (Before the fix above the
      // question could not arise: the reader was aborted for the whole
      // window, and those frames reached nobody at all.)
      // **`posting` joins `arriving` here (R-6.4.4), and it is a SECOND reason
      // to park.** They are different cases with one answer: an arrival already
      // running would read a stale count, and a move of this browser's own in
      // flight is a count the reply is about to carry. Both are answered by
      // remembering the newest frame and comparing it once the thing in flight
      // has landed — `send`'s `finally` is the other end of this one.
      if (arriving.current || posting.current) {
        pending.current = e;
        return;
      }
      arriving.current = true;
      try {
        // **Every read is of `stateNow`, never of the `state` this closure
        // was built with.** A second pass runs after `setState`, and a
        // closure's `state` is the one from the render that created it — so
        // the coalesced frame would be compared against a count two events
        // old and would walk what the first pass just walked. The ref is
        // written at the assignment as well as during render, so the ordering
        // does not depend on when React commits.
        for (let frame: Events | null = e; frame !== null; frame = pending.current) {
          pending.current = null;
          const now = stateNow.current;
          if (!now) return;
          const game = now.id;
          // The cheap answer, and the one this gives nearly every time: the
          // count has not moved, so nothing is fetched and nothing re-renders.
          // It is also what makes the coalesced pass a no-op when the read it
          // waited for already carried the count this frame is announcing.
          if (frame.events === now.events) continue;
          setBusy(true);
          try {
            const fresh = await api.get(game);
            // The same rule every other reply is judged by: a payload for a
            // game the player has left is dropped, never drawn over the game
            // they are on.
            if (gameId.current !== game) return;
            setState(fresh);
            stateNow.current = fresh;
            // **Grown: walk it. SHRUNK: reload and show nothing.** `events` is
            // non-monotone because `undo` pops them (R-P5), and at a shared table
            // the pop can be somebody else's. A run over a backwards range is not
            // a turn that happened — there are no positions to fetch for it — so
            // the account goes with the events it described.
            // **`fresh.seat`, and it is read off the payload the walk will be
            // drawn against.** `ClientState` is built for ONE seat, and at a
            // hot-seat table the server builds for the acting seat whenever this
            // caller holds it — so the seat this browser is watching from is the
            // fresh payload's, not the one the poll started under. At a SPECTATOR
            // it is `null`, and the arrival carries that through as a viewer
            // rather than as "no signal" — see `Arrival`.
            if (frame.events > now.events)
              await landed(game, now.events, fresh.events, {
                via: "signal",
                viewer: fresh.seat,
              });
            else {
              setRun(null);
              setWalk(null);
              setFresh(false);
            }
          } catch (err) {
            if (gameId.current === game)
              setError(err instanceof ApiError ? err.message : String(err));
          } finally {
            setBusy(false);
          }
        }
      } finally {
        arriving.current = false;
        pending.current = null;
      }
    },
    [landed],
  );

  const send = useCallback(
    async (line: string) => {
      if (!state || inFlight.current) return;
      // **A move made after this tab lost the lock is refused HERE, before the
      // Worker is asked** (spec §4). The answer bar has already been replaced
      // by the lock's own strip, but the BOARD is an input while a placement
      // picker is open and it is not inside that bar — so this is the guard
      // that makes "read-only" true rather than nearly true. Refusing in the
      // page is the point: a reply from a call in flight when the lock was
      // lost is not written back, so a move that reached the module would show
      // something that will not be kept.
      if (lockState === "lost") {
        setError(LOST_SENTENCE);
        return;
      }
      // The game this request belongs to, read once: `state` is re-created on
      // every render and the checks below are about the game we posted TO.
      const game = state.id;
      inFlight.current = true;
      setBusy(true);
      setError(null);
      // **Raised BEFORE the request goes out** (M6.4, R-6.4.4): the server's
      // own bump for this move can reach the stream before the reply reaches
      // here, and a frame handled in that window reads a count this browser is
      // itself changing. See `posting`, and the `finally` below.
      posting.current = true;
      try {
        const replied = await api.input(game, line, state.prompt.kind);
        // **A reply for a game you have left is dropped and never applied.**
        // See `gameId`. Nothing is lost by dropping it: the move was applied
        // and persisted server-side, and coming back to that game re-reads it.
        // **`stateNow` is written HERE as well as by the render** (spec §6.4).
        // React's commit is scheduled and not synchronous, so a parked frame
        // compared in the `finally` below would otherwise be read against the
        // PRE-move count and walked as somebody else's arrival — the exact
        // defect the guard exists to stop. The arrival loop's own assignment is
        // the precedent.
        if (replied.id === gameId.current) {
          setState(replied);
          stateNow.current = replied;
        }
        // The run the reply added, fetched before the busy window closes. It
        // rides INSIDE the `try` on purpose: a failed fetch of the run reports
        // through the error strip like any other refusal, and the live state
        // stays on screen — the move was applied and persisted either way.
        if (replied.id === gameId.current) await landed(game, state.events, replied.events);
        // **The hot-seat pass, and it is one re-read (spec §7.5).**
        //
        // A move that ends a turn answers its own mover with a `watching`
        // prompt: the reply is built for the seat that POSTED, which is the
        // true thing to say to a person who has just finished playing. At a
        // table where somebody else holds the next seat that is the whole of
        // it — the poll is what brings their move back. At a table where the
        // next seat is ALSO this person's, it leaves a browser holding an
        // unanswerable screen for a turn that is its own: `my_seats` is the
        // server's own answer to which of the two this is, and `GET
        // /api/games/{id}` builds for the acting seat whenever the caller
        // holds it, so one re-read IS the pass.
        //
        // **Not the poll, and not a substitute for it.** The poll (M6.1 §6)
        // waits on somebody else and runs on a timer; this is a single read
        // for a turn that never left this browser, and without it a hot-seat
        // table stalls on every seat change until a timer happens to fire.
        if (
          replied.id === gameId.current &&
          replied.prompt.kind === "watching" &&
          table?.my_seats.includes(replied.to_move)
        ) {
          const mine = await api.get(game);
          // The same assignment, for the same reason: this re-read is the
          // newest thing this browser knows, and the parked frame is compared
          // against it.
          if (mine.id === gameId.current) {
            setState(mine);
            stateNow.current = mine;
          }
        }
      } catch (e) {
        // The same rule for the refusal: a sentence about a game that is no
        // longer on screen would sit over a game it does not describe.
        if (gameId.current !== game) return;
        setError(e instanceof ApiError ? e.message : String(e));
        // **The re-read below is skipped on a 401.** The door has already been
        // moved — `api.ts` did that before this refusal was thrown — and a
        // re-read would only spend a request earning the same answer.
        if (isSignedOut(e)) return;
        // **Re-read the game, because the server just told us to and this
        // client had no way to.**
        //
        // `api.get` had exactly one call site — the setup screen — so once a
        // game was running there was no re-read path at all. The catch branch
        // never called `setState`, so `prompt.kind` stayed stale and every
        // subsequent click re-sent the same `expect` for the same 409, while
        // the server's own message read "Re-read the game and answer what it
        // asks." The only escape was a page reload back into setup.
        //
        // M3.5 makes that worse rather than neutral: with the answer bar pinned,
        // the player now stares at a permanently visible row of buttons every
        // one of which refuses. So the refusal is shown AND the truth is
        // fetched — the message stays on screen beside a screen that now agrees
        // with the server.
        try {
          const fresh = await api.get(game);
          if (fresh.id === gameId.current) setState(fresh);
        } catch {
          // The re-read failed too. Keep the original refusal on screen: it is
          // the more useful of the two, and a second error would replace the
          // sentence that explains what happened with one about this recovery.
        }
      } finally {
        posting.current = false;
        // A frame that landed during this post was parked (R-6.4.4); it is
        // compared now, against the state the reply produced, so a self-bump
        // is a no-op and an opponent's move made meanwhile walks.
        const parked = pending.current;
        pending.current = null;
        if (parked) void arrive(parked);
        inFlight.current = false;
        setBusy(false);
      }
    },
    [state, landed, table, arrive, lockState],
  );

  /** The OTHER way a change reaches this screen: somebody else made it (M6.1
   *  §6) — a move, or now a message.
   *
   *  **The consumer the room plan names, and it is one function** (R-P9). The
   *  signal knows only that this game now has N events and M messages;
   *  everything about turning that into a board happens in [`arrive`], and M6.3
   *  replaced the transport under `useSignal` without touching a line of it.
   *
   *  **`active` is "there is a game on screen", and nothing narrower (M6.4
   *  R-6.4.4).** It read "there is something to WAIT for" — not your own turn,
   *  not a finished game — and both exclusions were true of a screen where the
   *  only thing that could arrive was a move. A message is not: a table where
   *  the only person who can hear one is whoever is not thinking is a chat that
   *  reaches you when you stop playing, and a game that is over is exactly when
   *  people say something about it. The two Minors the M6.3 close left — no
   *  stream during your own turn, none after the game is over — close here.
   *  **Still read off the STATE and never off the signal**: a locked session
   *  answers `/events` with `your_turn: true` by the M3 projection rule, and a
   *  client that let the signal decide would have the lockdown hand it a move
   *  to make. `state?.id ?? null` is what holds the setup screen out.
   *
   *  **`busy` is NOT in there, and taking it out was ruling R-9.1.** It was,
   *  for a reason that belonged to the poll: while a request of this browser's
   *  own is in flight, the count the server is about to report is one this
   *  client is itself changing, and asking through that window races a reply
   *  against a signal for the same events. Under a HELD connection the same
   *  term means something else entirely — let go of the stream whenever a
   *  request is in flight — and `arrive` sets `busy` around the frame it
   *  fetches, so every arrival tore the reader down and opened another.
   *  **Measured by the browser gate**: a watching browser opened `/stream`
   *  2,643 times in one game, exactly once per applied move, while
   *  `signal.ts`'s own "a visible tab holds ONE open request per resource" said
   *  otherwise.
   *
   *  **What used to answer the race `busy` covered was the `your_turn` term,
   *  and that term has just gone — so the POSTING GUARD is what answers it
   *  now** (spec §6.4). A write on this screen happens only while `your_turn`
   *  is true (the answer bar is the only thing that posts, and R-P6's undo
   *  needs `current_player` to still be the caller), and that used to close the
   *  signal for the whole window. The stream now stays open through it, so
   *  `posting` parks the frame instead and `send`'s `finally` compares it
   *  against the state the reply produced: a self-bump reads equal and costs
   *  nothing, and an opponent's move made meanwhile walks. Every other request
   *  this screen makes is a GET that bumps nothing. `setBusy` stays where it
   *  is: `data-busy` has to cover the fetch and the walk for the browser
   *  harness; only `active` stops reading it. */
  const link = useSignal(
    state?.id ?? null,
    !!state,
    arrive,
    // **A game that is GONE stops being polled, and says so** (spec §5.5). The
    // reachable case is a shared table somebody abandoned while this browser was
    // looking at it: before `onEnd` the poll backed off against a permanent 404
    // and the banner read "reconnecting…" about a game that was never coming
    // back. The sentence is the CLIENT's for an `end` frame — `signal.ts`'s
    // `endSentence` words `deleted` and `gone` once for all four consumers
    // (final review I-2), since the frame carries a reason and no words — and
    // the server's own message for the two refusals that end a signal without
    // one. The fallback below is what is left if neither arrives.
    useCallback((e: End) => toLobby(e.sentence ?? "this game is no longer available"), [toLobby]),
  );

  // **Debounced, because a drag is not a click.**
  //
  // The pattern is `Index.tsx`'s: a 200 ms timer plus a `live` flag, so a drag
  // across forty stops costs one request rather than forty, and a response that
  // arrives after you have moved on is dropped instead of painting a stale
  // position. The browser walk counts the requests a drag actually makes — a
  // cell that only checked the final frame would pass a scrubber firing one
  // request per pixel.
  useEffect(() => {
    if (scrubK === null || !state) {
      setPast(null);
      return;
    }
    let live = true;
    const id = state.id;
    const k = scrubK;
    const timer = setTimeout(() => {
      api
        .at(id, k)
        .then((p) => {
          if (!live) return;
          setPast(p);
          setError(null);
        })
        // **Say so.** Swallowing this put the LIVE board back on screen —
        // clickable, outline gone — while the track still read "after 500 of
        // 1036" and offered "Back to live" from a position it was no longer
        // showing. The server refuses in sentences (a 404 naming the bound, a
        // replay failure naming the event); carry one.
        .catch((e) => {
          if (!live) return;
          setPast(null);
          setError(e instanceof ApiError ? e.message : String(e));
        });
    }, 200);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [scrubK, state?.id]);

  // Leaving the log panel leaves the replay. The scrub track lives in that
  // panel, so a scrub position with the panel shut would be a board showing a
  // past nothing on screen explains.
  useEffect(() => {
    if (surface !== "log") setScrubK(null);
  }, [surface]);

  // ---- the door, below every hook and no longer in front of the page ------
  //
  // **Four answers, and since M6.5b none of them is a GATE** (spec §2.5,
  // R-6.5b.8). They were four `return`s here — a refusal, a silence, the
  // sign-in form and the sentence naming the expected header — and each of
  // them drew nothing else at all, including the games in this browser's own
  // IndexedDB, which need no door to play. `Door` renders them as the CONTENT
  // of the part of the first screen the server owns — since M6.5c2 as states,
  // each a card (`Door.tsx` holds their order, which is the rule);
  // `lobbyScreen` below is what puts them there.
  const signedIn = door.me?.user ?? null;
  // **The server's part of the lobby while nobody is signed in** (M6.5c2, spec
  // §2.1): the door's states A–D, the name step and state B are all `Door`'s —
  // including the one decided here until this cycle, a server nothing answered,
  // which is state C and reads `serverUp` itself.
  const doorContent = signedIn ? null : (
    <Door door={door} serverUp={serverUp} onSignedIn={() => refresh()} onRetry={retry} />
  );

  // **Who you are, and how to stop being them.**
  //
  // The setup and dealing screens carry their own root and their own banner, so
  // the strip is handed to them as a node rather than restated there: two copies
  // would be two places for the sign-out wiring to drift.
  //
  // The button exists under `open` ONLY. Under the two edge doors there is
  // nothing stored to clear, and a control that looked like it logged you out
  // while Access kept letting you in would be a lie on screen.
  const signOutButton = signedIn && door.me?.mode === "open" && (
    <button type="button" className="ghost" onClick={signOut}>
      sign out
    </button>
  );
  /** The caller holds NO SEAT at the game on screen — a watcher (spec §5.2).
   *
   *  The SERVER's own statement of it: `ClientState.seat` is `null` only where
   *  `viewer_seat` found no seat for this caller, which is the one condition
   *  under which every play route would refuse them. Nothing is inferred from
   *  the prompt — a player waiting on somebody else reads `watching` too, and
   *  gets a turn back. */
  const spectating = !!state && state.seat === null;
  // **"You are spectating" (spec §5.2; M6.5c2 R-6.5c2.7).** A spectator
  // is not a seat at this table, and the strip is the one line on the banner
  // that says which of the two you are here — sign-out still lives in the Game
  // menu, where every screen with a menu keeps it.
  //
  // **And "On this device" wherever the screen is a local game's** (M6.5b,
  // R-6.5b.1). Not merely where there is nobody to be: the module answers as
  // `local@browser` whatever address this browser signed in at, so "You are
  // red@…" over a game played by this device names somebody who holds no seat
  // at it. The same word therefore covers the signed-out case, where "You are
  // null" is not a thing to print.
  const onThisDevice =
    (state !== null && isLocal(state.id)) ||
    (dealing !== null && isLocal(dealing)) ||
    (route.kind === "new" && route.local) ||
    (broken !== null && isLocal(broken.id)) ||
    lockId !== null;
  // **The name the player's seats are labelled with, where this browser keeps
  // one** (cycle E; the owner's first game read "You are ben" over a seat card
  // reading "Ben"). A seat card notes the name its seat was TYPED as — what
  // this browser sent with the create, the join or the keep, `displayName()` —
  // while this strip read the parsed KEY, which `User::parse` lowercases. The
  // key stays the answer where there is no name: it is still who you are.
  const shownAs = displayName()?.trim() || signedIn;
  const whoStrip = (
    <span className="meta who">
      {spectating
        ? "You are spectating"
        : onThisDevice || !signedIn
          ? "On this device"
          : `You are ${shownAs}`}
    </span>
  );
  const who = (
    <>
      {whoStrip}
      {signOutButton}
    </>
  );

  /** **The FRONT DOOR at `#/`, and everyone sees it** (R-6.5b2.1).
   *
   *  Two ways on: the games in this browser, and a link to the turnstile. It
   *  asks the server nothing — not even whether the other channel is served,
   *  which is a BUILD value (R-6.5b2.7) — so it is the one screen that renders
   *  with the API refused, never answering, or not deployed at all. It is NOT
   *  behind the sign-in gate below, and that is the whole product change: until
   *  this cycle the first screen belonged to the server. */
  const doorScreen = (notice?: string | null) => (
    <FrontDoor
      multiplayer={multiplayerOffered()}
      notice={notice ?? null}
      readiness={readiness}
      link={projectLink()}
    />
  );

  /** The band's corner on a SERVER setup screen (M6.5c2, spec §2.1–§2.2;
   *  mocks 02, 07, 08, 09): who is playing, and the one change the door allows
   *  — the identity corner the lobby draws, so the new-table form, the open
   *  table, the unlock face and a game that will not open (cycle C) say it in
   *  the lobby's own words. The "You are …" strip (`who`) stays the game
   *  banner's and the deal's. */
  const serverCorner = signedIn && door.me ? <IdentityCorner me={door.me} onChange={signOut} /> : undefined;

  /** The SERVER's first screen at `#/lobby`: its three listings, the door's own
   *  answer, or the card that says the server cannot be reached.
   *
   *  **Every screen behind a door lands here when there is no user** (spec
   *  §2.5). The four `return`s that used to do that job drew nothing else;
   *  this draws the door's words in the place the listings would be. The games
   *  in THIS BROWSER are no longer stacked on top of it — they are the other
   *  channel now, at `#/`, and the lobby carries one link back to them. */
  const lobbyScreen = (notice?: string | null) => (
    <Lobby
      me={signedIn}
      corner={serverCorner}
      notice={notice ?? null}
      door={doorContent}
      serverUp={serverUp}
      onReached={() => setServerUp(true)}
      onRetry={retry}
    />
  );

  /** **A table route's screen while its table is being read** (cycle E,
   *  R-E7; the M6.3 close's own fix, §6.2): a cold `#/t/<id>` — a link, a
   *  reload — drew the LOBBY for one round trip while `open(id)` awaited
   *  `GET …/table`, which mounted `<Lobby>` and so fired its three listings
   *  and opened the house stream, all thrown away. The lists load only where
   *  the lobby is the screen. Drawn while the door is still asking who this
   *  is, too: a signed-out caller is answered by the lobby's door content
   *  once the door has said so. In the table's own crumb, by its id — the
   *  name is what is being read. */
  const openingScreen = (id: string) => {
    const local = isLocal(id);
    return (
      <div className="band-screen">
        <LobbyBand crumbs={local ? ["On this device", id] : ["Lobby", id]} corner={local ? undefined : serverCorner} />
        <div className="setup opening-table">
          <p className="muted" role="status">
            opening&hellip;
          </p>
        </div>
      </div>
    );
  };
  /** The door has not answered yet: neither a caller nor a refusal. */
  const doorAsking = door.me === null && door.error === null;

  // `enabled`/`toggleSection` are THIS component's, deliberately: the set table
  // and the game must agree about which boards are on, and a second
  // `useSections()` inside `NewGame` was two copies that stopped agreeing the
  // moment one of them was toggled (R-T12-4).
  // **FOUR screens live in front of a game now** (spec §5.1), and the ROUTE is
  // what picks between them rather than a chain of "is this null" tests. `#/new`
  // is the form whether or not a table happens to be dealing; `#/` is the lobby;
  // `#/t/<id>` is whichever of the three a table's own status makes it, which is
  // `open`'s question and the route effect's answer.
  // **This browser's storage could not be read** (M6.5c2 cycle D, dossier row
  // 14): the game's acquisition — its generation, then the rebuild's read —
  // failed, and the Worker said so in the storage's own words rather than
  // trapping into a restart that would meet the same store. See `BrokenStore`
  // for the face, and why it is a file of its own.
  //
  // **Ahead of `!state`, so it replaces a game ON SCREEN too** (fix round 1,
  // review Minors 1 and 3): a restart whose fresh Worker cannot read the store
  // leaves a game this tab no longer owns and cannot write, and a Take back
  // the store refuses is the same fact. Where nothing was read — a hold
  // refused this way on the way in — there is no game under it to replace.
  if (lockId !== null && lockState === "unreadable")
    return (
      <BrokenStore
        sentence={lock.sentence ?? ""}
        onLeave={() => leave(localExit.to)}
        leaveLabel={localExit.label}
      />
    );
  if (!state) {
    // **Another tab has this game open** (R-6.5b.5), and nothing has been read:
    // FIRST here, before the deal and before the game, because the dealing
    // screen writes too and the lock is taken at OPEN time for all of it.
    if (lockId !== null && (lockState === "elsewhere" || lockState === "lost"))
      return (
        <LockScreen
          state={lockState}
          onTakeover={retake}
          onLeave={() => leave(localExit.to)}
          leaveLabel={localExit.label}
        />
      );
    // **A save in this browser that no longer rebuilds** (spec §4): the
    // engine's own sentence, the bytes still saveable and the game still
    // abandonable — neither of which asks the module to replay anything.
    // **And a game of either host this build's bot cannot continue** (M6.5c2,
    // spec §2.4): the same screen, host-aware — see `broken`'s own doc.
    if (broken) {
      const local = isLocal(broken.id);
      /** The layer's delete is the CREATOR's, seated (`mayAbandon`, the one
       *  predicate the open table and the lobby read too); a local game's is
       *  always this browser's own. */
      const abandonable = local || mayAbandon(broken.table);
      const name = broken.table.name ?? broken.id;
      // **The lock screen's shape** (M6.5c2 cycle C; the whole-branch review's
      // Minor 6): the band and ONE notice card, so a player who follows a row
      // to a game that will not open lands on a screen of this app and not
      // on the old who-strip over a bare column. A SERVER game's band is the
      // open table's — "Lobby / <table>", with the identity corner — and a
      // LOCAL game's is the lock screen's, "On this device" with no corner:
      // nothing on it is the server's. The card is titled by the table, as the
      // open table titles its page under the same band.
      return (
        <div className="band-screen">
          <LobbyBand
            crumbs={local ? ["On this device"] : ["Lobby", name]}
            corner={local ? undefined : serverCorner}
          />
          <div className="setup broken-save">
            <div className="notice-card">
              <h3 className="page-title">{name}</h3>
              <Refusal sentence={broken.sentence} className="error notice-reason" />
              <div className="notice-actions">
                {/* The bytes are the player's own on a LOCAL game at any point; a
                    server game's record is served only once the game is over. */}
                {local && (
                  <button
                    type="button"
                    className="ghost"
                    onClick={() =>
                      void saveGameFile(broken.id).catch((e) =>
                        setError(e instanceof Error ? e.message : String(e)),
                      )
                    }
                  >
                    Save game file
                  </button>
                )}
                {/* **`local` for a FOREIGN local game only** (M6.5c2 R-B19).
                    `local` means "send this game to the archive first" (R-6.5b.6),
                    and the archive takes a foreign game's record — its check is a
                    strict replay and asks nothing of the bot — so that game is sent
                    before it is deleted, like every other local Abandon. **NOT for
                    a save that no longer rebuilds** (spec §4: "Abandon still
                    works"): the archive would refuse exactly that record — it is the
                    rebuild that has stopped working — so sending first would leave
                    a game that can never be abandoned; the bytes beside this button
                    are the copy that still works. A server table's own delete
                    archives it (R-6.5a.11). A local game lands on the door, where
                    the rest of this browser's games are listed; a server game on
                    the lobby. */}
                {/* **The foreign screen FORCES the bit** (cycle E, ruling T5-c):
                    it exists for a record this build cannot drive, whose moves
                    are exactly what makes it worth saving, and its stored
                    mirror never heals here — the host refuses the game before
                    anything is persisted — so a game stored before the bit
                    would read no event for ever and be deleted without a copy.
                    A foreign game dealt a moment ago then sends a zero-move
                    copy, which is what every local Abandon did before cycle E.
                    Not the record's own count: that would be this client's
                    first parse of a record (`local/client.ts`, `local/db.ts`). */}
                {abandonable && (
                  <Abandon
                    table={broken.foreign ? { ...broken.table, has_events: true } : broken.table}
                    local={local && broken.foreign}
                    onDone={() => (local ? toDoor() : toLobby())}
                    onError={setError}
                  />
                )}
                {/* The way back, in the lock screen's quiet words: the lobby for
                    a server game, and for a local one the lobby where the build
                    serves one and the door where it does not (`localExit`). The
                    arrow is decoration, so the NAME is the destination. */}
                <button
                  type="button"
                  className="quiet-link"
                  onClick={() => (local ? leave(localExit.to) : toLobby())}
                >
                  <span aria-hidden="true">&larr; </span>
                  {local ? localExit.label : "Lobby"}
                </button>
              </div>
              {error && <Refusal sentence={error} />}
            </div>
          </div>
        </div>
      );
    }
    // **The two screens a game in this browser reaches with the door shut**
    // (spec §2.5): the local create, and the deal it opens. The create seats
    // `LOCAL_USER`, because the module answers as that one address whatever
    // this browser signed in at; the deal reads its own seats off the table's
    // `you` (M6.5c2 cycle D, R-6.5c2.35), so it is handed no address at all.
    if (route.kind === "new" && route.local)
      // **Keyed by HOST** (whole-branch review, Minor #1). The two forms sit at
      // the same position in this tree, so React reconciles one into the other
      // and the STATE goes with it: `#/new` with Players set to 4, then a
      // hand-typed `#/new/local`, kept the 4 beside two seats and the module
      // refused the create in `check_shape`'s words. A key is what makes the
      // switch a remount.
      return (
        <NewGame
          key="local"
          local
          me={LOCAL_USER}
          onCreated={(t) => open(t.id)}
          onCancel={() => toDoor()}
        />
      );
    if (dealing && isLocal(dealing))
      return (
        <Dealing
          game={dealing}
          art={art}
          labels={labels}
          sections={enabled}
          onToggleSection={toggleSection}
          onPlaying={() => open(dealing, true)}
          onLeave={(sentence) => leave(localExit.to, sentence)}
          leaveLabel={localExit.label}
          who={who}
        />
      );
    // **The front door, and it is in front of the sign-in gate rather than
    // behind it** (R-6.5b2.1). Below this line every screen is the SERVER's.
    if (route.kind === "door") return doorScreen(error);
    // The door the server owns stands in front of those, as content on the
    // lobby rather than as a gate — once it has answered. A table route waits
    // for that answer on its own screen (R-E7), and a LOCAL table needs no
    // door at all: its read is the lock's and the module's.
    if (!signedIn)
      return route.kind === "table" && (doorAsking || isLocal(route.id))
        ? openingScreen(route.id)
        : lobbyScreen(error);
    if (route.kind === "new")
      return (
        <NewGame
          key="server"
          me={signedIn}
          onCreated={(t) => open(t.id)}
          onCancel={() => toLobby()}
          who={serverCorner}
        />
      );
    if (dealing)
      return (
        <Dealing
          game={dealing}
          art={art}
          labels={labels}
          sections={enabled}
          onToggleSection={toggleSection}
          onPlaying={() => open(dealing, true)}
          // A sentence when the table went out from under this screen, none
          // when the player simply left — `toLobby` clears on the absence,
          // which is what leaving by choice should do.
          onLeave={toLobby}
          who={who}
        />
      );
    if (openTable)
      return (
        <OpenTable
          // KEYED by the table, because the document is its first state: a
          // switch from one open table to another is a new screen, not the old
          // one's state under a new id.
          key={openTable.id}
          game={openTable.id}
          table={openTable.table}
          // **Re-OPEN it, and let the status pick the screen** (spec §5.4).
          // What the open table knows is that it is no longer open; whether
          // that is the deal or a game already running is `open`'s own
          // question, and a table can pass through both while this screen is
          // up — the last seat fills, both people keep, and the flip this
          // browser sees is straight to `playing`.
          //
          // Safe to call for the id already on screen: the two refs the route
          // effect reads exist to stop that EFFECT echoing a navigation, and
          // neither is consulted here.
          // **The `catch` is not decoration.** `open` has none of its own, and
          // the reachable case is a stranger who clicked a seat that filled in
          // the same moment: `join` answers with a table that is no longer
          // open, the hand-over runs, and `GET …/table` refuses a caller who is
          // not seated. Without this that rejection is unhandled and the screen
          // recovers only when the poll's own refusal fires one tick later.
          // Same two lines the route effect uses for a refused `#/t/<id>`.
          onStarted={() =>
            void open(openTable.id, true).catch((e) =>
              toLobby(e instanceof ApiError ? e.message : String(e)),
            )
          }
          // The sentence is the server's — a table deleted under this screen —
          // and the lobby is where it is shown, for the reason a refused
          // `#/t/<id>` lands there: a bare screen that answers nothing is the
          // one place a player cannot act from.
          onLeave={toLobby}
          who={serverCorner}
        />
      );
    if (unlock)
      return (
        <Unlock
          game={unlock.id}
          table={unlock.table}
          // **Re-OPEN it, and let the server's answer pick the screen** (spec
          // §5.3), which is `OpenTable`'s own hand-over and for the same
          // reason: what this screen learns is that the password was taken, and
          // whether that leads to a game, a finished game or a table that has
          // meanwhile been abandoned is `open`'s question. The `catch` is the
          // two lines the route effect uses for a refused `#/t/<id>` — `open`
          // has none of its own, and a table deleted between the unlock and the
          // re-read is a rejection nothing else would handle.
          onUnlocked={() =>
            void open(unlock.id).catch((e) =>
              toLobby(e instanceof ApiError ? e.message : String(e)),
            )
          }
          onLeave={() => toLobby()}
          who={serverCorner}
        />
      );
    // **A table route with no screen yet is a table being read** (R-E7): the
    // route effect's `open(id)` is in flight, or a local game's lock has not
    // answered. A refused read navigates to the lobby or the door with the
    // sentence, so this never stands in for a refusal.
    if (route.kind === "table") return openingScreen(route.id);
    return lobbyScreen(error);
  }

  // **A SERVER game still needs a door, and a local one does not.** The screen
  // below asks the routes this caller is admitted to; with nobody at the door
  // there is no such caller, so the first screen — carrying the door's own
  // words — is where that lands. A `local-` game is answered by the module in
  // this browser and has no such question.
  if (!isLocal(state.id) && !signedIn) return lobbyScreen(error);

  const players = state.frame.dashboard.find((p) => "Players" in p);
  const colors: PlayerColor[] =
    players && "Players" in players ? players.Players.players.map((p) => p.color) : [];

  // **The last stop is the PRESENT, not a past position.** The track's final
  // stop is `total`, which the panel itself labels "now" — so treating any
  // non-null `past` as "replaying" outlined the live board, disabled placement
  // and contradicted the help text, all while the panel said you were at the
  // present. At that stop the LIVE payload is also the fresher of the two: a
  // bot may have moved since the snapshot was fetched.
  const atPresent = past !== null && past.index >= past.total;
  /** The player is in the PAST via the log's track. */
  const scrubbing = past !== null && !atPresent;

  // **The walk's step, and what the whole screen reads from while it runs.**
  //
  // `replaying` is "what is drawn is not the live position", which is what the
  // outline says and what withholds the board as an input. On the walk's LAST
  // step the frame IS the live position — `index === total` — so the outline
  // comes off one step before the walk ends, deliberately: the table has
  // arrived, and the strip below is the only thing still saying so.
  const stepNow = run && walk ? run.steps[walk.at] : undefined;
  const walking = stepNow !== undefined;
  const replaying = walking ? stepNow.after.index < stepNow.after.total : scrubbing;
  const view = walking ? stepNow.after : scrubbing ? past : state;

  // **The highlights outlive the walk by one answer (spec §3.2.4, ruling
  // R-F1).** While it runs they are the step's own diff. When it ends — by
  // ticking out or by `skip` — they are `heldChanges`: the union over the steps
  // the run's LAST mover played, held on the live table by `fresh` until the
  // player answers again, so what the opponent did is still marked on the board
  // you are about to play on.
  //
  // **Held is not "the last step's".** Every bot turn this server serves ends
  // with "End additional-action phase", whose diff is empty, so the last step's
  // marks are no marks and the promise above was kept for no real turn at all
  // (final review I1, reproduced live three times). `walk.ts` carries the rule
  // and the measurement.
  //
  // **ONE source for all four reads, and they stopped being one read.** The
  // MARKS are the step's own diff while the walk runs and the mover's union
  // once they are held. The COLOUR, the pulse's BEAT and its DWELL are the
  // step's on both sides, and that is deliberate rather than left over: the
  // held marks belong to the turn the walk just played, so a beat that moved at
  // the tick-out would restart every animation on a step that has already had
  // its pulse. One object is what keeps the two halves in step — four parallel
  // ternaries could drift apart, and the pulse's would drift silently, a pulse
  // timed to a step whose marks are not the ones on screen looking like nothing
  // at all. The cell ruling R-P4 shaped reads `{ beat: "0", dwell: "2000ms" }`
  // on both sides of the skip and is what pins it.
  const lastStep = run?.steps[run.steps.length - 1];
  const shown =
    walk && stepNow
      ? { step: stepNow, at: walk.at, changes: stepNow.changes, mover: stepNow.mover }
      : fresh && run && lastStep
        ? {
            step: lastStep,
            at: run.steps.length - 1,
            changes: heldChanges(run),
            mover: lastStep.mover,
          }
        : null;
  const highlights = shown?.changes ?? null;
  const highlightBy = shown ? colors[shown.mover] : undefined;

  /** Copy this finished local game to the SERVER's archive (R-6.5b.6, spec
   *  §2.5).
   *
   *  **Both documents, in ONE snapshot, unparsed** (spec §2.2): the record's
   *  bytes are what its terminal digest is over and its seed is a `u64` that
   *  `JSON.parse` would round, and the server checks the table against the
   *  record — two queued reads can fall either side of a move. The server
   *  chooses the leaf from the replayed state; nothing here says where the
   *  copy should go.
   *
   *  A 201 and a 200 `identical` both say a complete copy is at a path the
   *  server names, so both are reported and both darken the control. Every
   *  refusal is shown as the server sent it and leaves the control alone —
   *  nothing about this game has changed. */
  const sendToArchive = async () => {
    // **One send per click, and a ref because state would not do** (whole-
    // branch review, Minor #4; `inFlight`'s own reasoning above). React
    // batches, so two clicks in a tick both read a state flag as false and
    // both post — and the SECOND lands as `identical`, so a first send
    // reported itself as "already archived". The control darkens on the
    // answer, which is too late to be the guard.
    if (archiving.current) return;
    archiving.current = true;
    setError(null);
    try {
      // **The pair's own failure is named as its own** (PR #37 fix wave, fix
      // round 1, review I-1): a store this tab could not read rejects here
      // with a sentence about the STORE, and the `catch` below would have
      // called it an unreachable archive the request never went to.
      let pair: { record: string | null; table: string | null };
      try {
        pair = await localPair(state.id);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
        return;
      }
      if (pair.record === null || pair.table === null) {
        setError("this game has no saved record on this device");
        return;
      }
      const r = await api.archive(pair.record, pair.table);
      setArchived((a) => ({
        ...a,
        [state.id]: `${r.identical ? "already archived" : "archived"} under ${r.path}`,
      }));
      // **A landed send is a landed send, wherever it was made from** (spec
      // §2.6): the outbox's receipt goes down, any row this game was waiting on
      // is cleared, and the game-over hook does not offer the same bytes again.
      await recordLanded(state.id, pair.record);
    } catch (e) {
      // **A 404 is the deployment's answer** (M6.5c2; the outbox's rule): the
      // archive is OFF, written down, and this control goes with it.
      if (e instanceof ApiError && e.status === 404) rememberArchiveOff();
      // The server's own sentence where it gave one; spec §4's words where the
      // request never reached it. Nothing was sent, so nothing is claimed.
      setError(e instanceof ApiError ? e.message : "the archive could not be reached");
    } finally {
      // Released either way: a refusal leaves this game exactly as it was, and
      // sending again is the right thing to be able to do.
      archiving.current = false;
    }
  };

  // The board is an input surface only while a placement picker is open — the
  // one place the engine is asking "which areas", rather than "which option".
  const picking = state.prompt.kind === "place_a" || state.prompt.kind === "place_b";
  const selected =
    state.prompt.kind === "place_a"
      ? state.prompt.selected
      : state.prompt.kind === "place_b" && state.prompt.selected != null
        ? [state.prompt.selected]
        : [];

  return (
    // **`data-busy` is the in-flight state, in the DOM, and the browser walk
    // waits on it.**
    //
    // The harness used to wait on `.decision button[disabled]` alone, which is
    // the same condition for every screen that HAS a control — and the finished
    // game has none: its deck is a standing, not a prompt. So a read taken
    // between the reply landing and the run's positions arriving saw a settled
    // game-over screen, and the walk then replaced the deck a moment later.
    // Measured: the whole-game cell reached its winner, read "The game is
    // over.", and failed ten seconds afterwards looking for the sentence again.
    // The client's own `busy` covers the WHOLE window, the run fetch included.
    //
    // **The pulse's two attributes live HERE, on the one ancestor of both
    // columns (R-GC-1).** Every highlight pulses for the length of the step's
    // own dwell, so the CSS needs the duration and a beat to select the
    // keyframes by — and the four things that light are split across the DOM:
    // `.tile` and `.strip-slot` are inside `main.table`, `.seat` and the tech
    // chips are in `aside.side`, its SIBLING. A custom property cannot be read
    // out of a sibling subtree at all, so a `--dwell-ms` on the table would
    // leave the side column silently animating the 2000 ms fallback at every
    // other cadence. `.app` is where `data-busy` already lives for the same
    // reason: it is the element that contains the whole screen.
    //
    // **The BEAT is what makes a pulse re-run.** An element's animation
    // restarts only when its `animation-name` changes or the element is
    // re-created, and a tile lit by two consecutive steps is neither — so the
    // second step would land in silence. `at % 2` picks between two
    // byte-identical keyframe names, which is the whole mechanism.
    <div
      className="app"
      data-busy={busy ? "1" : undefined}
      data-beat={shown ? String(shown.at % 2) : undefined}
      style={shown ? ({ "--dwell-ms": `${dwellFor(shown.step, cadence)}ms` } as CSSProperties) : undefined}
    >
      <header className="banner">
        <h1>Yokohama</h1>
        <GameMenu
          state={state}
          busy={busy}
          signOut={signOutButton || undefined}
          onSwitch={(id) => {
            setSurface(null);
            setScrubK(null);
            // The refusal on screen was about the game being left, so it goes
            // with it: a sentence from game A over game B describes nothing
            // the player can see.
            setError(null);
            // Through `open`, so a table still DEALING in this list opens its
            // dealing screen instead of the 409 every session route answers it
            // with.
            return open(id);
          }}
          // **Out of the game is out to the LOBBY** (spec §5.1). `show(null)`
          // alone would leave the hash reading `#/t/<id>` for a game no longer
          // on screen, which is an address that lies and a Back button that
          // does nothing; the route effect is what clears the game.
          onLobby={() => {
            setSurface(null);
            setScrubK(null);
            setError(null);
            // **A local game's way out is its own channel's first screen where
            // this build serves no lobby** (`localExit`); a server game's is the
            // lobby, always.
            navigate({ kind: isLocal(state.id) ? localExit.to : "lobby" });
          }}
          leaveLabel={isLocal(state.id) ? localExit.label : "Lobby"}
          // **A build that serves no lobby lists this browser's games**: its
          // server has no game routes to list from (the whole-branch review's
          // Important 1), and every game this screen can show is a local one.
          // **And so does a LOCAL game's screen in a build that serves one**
          // (M6.5c2 cycle D, dossier row 56): the games a player switches to
          // from a game on this device are the other games on this device —
          // the server's tables are the lobby's list, and on a page whose
          // server cannot be reached asking it drew a refusal under a local
          // game.
          savedFrom={multiplayerOffered() && !isLocal(state.id) ? undefined : "local"}
        />
        {/* **The strip here, and the BUTTON inside the Game menu.**
            The banner is one row at 1440, 1280 and 1024 by a gate that measures it
            (`the_screen_itself.spec.ts`), and the door's two controls together
            cost 269px of a 1240px content box at 1280 — enough to break the
            line, measured: 1325px of children and gaps against 1240. The strip
            is a FACT a player reads at a glance and stays; the button is a
            control, and the Game menu is already where this screen's session
            controls live (which game, which other game, back to the form). The
            screens with no Game menu — the form and the deal — keep both. */}
        {whoStrip}
        {/* Four surfaces inline, the five card references behind a disclosure,
            so the banner stays ONE ROW at every laptop width: measured 93 → 56px
            at 1440 and 174 → 93 at 1024, with the nav's bottom edge at 43px
            instead of 88/88/125 at 1440/1280/1024.

            **The benefit is the banner's height, and not what the review said
            it was.** Its argument was that the wrapped second row sat under an
            open panel, so you could not switch surfaces without pressing Escape
            first. That is true of the old panel and no longer reachable: the
            panel is a real `<dialog>` opened with `showModal()` now, so the
            whole page outside it is inert while it is open — which is what
            modality means, and is the containment the panel spent three
            milestones asserting without having. Escape then switch, by design.
            Written down because the tempting sentence is the review's, and it
            stopped being true one commit earlier.

            The `<details>` pattern is the one `Board.tsx`'s legend already uses,
            rather than a second disclosure idiom. */}
        <nav className="views">
          <BoardsMenu enabled={enabled} onToggle={toggleSection} />
          {/* **Suggest is a PLAYER's control, so a watcher is not offered it**
              (spec §5.2). `/suggest` ranks the moves on YOUR seat, and a caller
              with no seat has none — the server answers a typed refusal and the
              panel would open on it. Every other surface here reads the
              position and is a watcher's to open. */}
          {SURFACES.filter(
            (s) => !CARD_SURFACES.includes(s.key) && !(spectating && s.key === "suggest"),
          ).map((s) => (
            <button
              type="button"
              key={s.key}
              className={`ghost ${surface === s.key ? "on" : ""}`}
              onClick={() => setSurface(surface === s.key ? null : s.key)}
            >
              {s.label}
            </button>
          ))}
          <details className="cards-menu" open={CARD_SURFACES.includes(surface as SurfaceName)}>
            <summary className="ghost">Cards</summary>
            <div>
              {SURFACES.filter((s) => CARD_SURFACES.includes(s.key)).map((s) => (
                <button
                  type="button"
                  key={s.key}
                  className={`ghost ${surface === s.key ? "on" : ""}`}
                  onClick={() => setSurface(surface === s.key ? null : s.key)}
                >
                  {s.label}
                </button>
              ))}
            </div>
          </details>
        </nav>
        {/* The payload is built for the CALLER's seat, which is not always the
            seat to move — so the banner names WHO is playing rather than saying
            "your turn". That is right on both counts: while somebody else is
            thinking it says whose turn it is, and at 3P or 4P one operator
            drives more than one human seat, exactly as the terminal does, and a
            screen that will not say which seat is a screen you can play the
            wrong turn on. `view.to_move` is the field it reads, and it means
            the mover whoever asked. */}
        <span className="meta turn">
          {state.over ? (
            "finished"
          ) : (
            <>
              <span className={`seat-${(colors[view.to_move] ?? "").toLowerCase()}`}>
                {colors[view.to_move] ?? `Seat ${view.to_move}`}
              </span>
              {" to move"}
            </>
          )}
        </span>
        {/* **Save game file** (M6.5a, R-6.5a.13): the game's record, as the
            bytes whoever holds it holds, downloaded as `<id>.json`. The text is
            never parsed on the way (see `api.recordText`), so the file a player
            saves is the file the CLI loads.

            **When it is offered differs by host, and the reason is whose seed
            it is.** A HOSTED record is the server's and carries the setup seed
            until the last move, so `…/record` answers 409 before then and the
            control waits for the game to end. A LOCAL one is the player's own
            seed on the player's own disk (R-6.5b.7), read off the Worker's side
            channel rather than through that route — so it is offered at any
            point. */}
        {state.over || isLocal(state.id) ? (
          <button
            type="button"
            className="ghost"
            title="the record alone — the file the terminal loads"
            onClick={() =>
              void saveGameFile(state.id).catch((e) =>
                setError(e instanceof Error ? e.message : String(e)),
              )
            }
          >
            Save game file
          </button>
        ) : null}
        {/* **A backup beside it, and the two say different things** (M6.5b2,
            R-6.5b2.10). "Save game file" is the RECORD, which is what the
            terminal loads; this is an envelope carrying the record AND the
            table document, which is what this app restores. Offered on a local
            game at any point, for "Save game file"'s own reason: the seed is
            the player's own and is read off the Worker's side channel. */}
        {isLocal(state.id) ? <BackUpOne id={state.id} /> : null}
        {/* **A tab that has LOST the lock writes nothing** (spec §4, review
            Important 2), so both controls below go the way the answer bar
            goes. The Worker fences a delete from a tab that does not hold the
            game — it answers `not-held` and keeps the pair — so a live Abandon
            would report a delete that did not happen and return to a lobby
            still listing the game; and with an archive on it would post a
            STALE record first. "Save game file" stays, because it reads bytes
            and writes nothing. */}
        {/* **Send to the archive** (spec §2.5): a finished game in this browser,
            copied to the server that keeps one. Dark where the server keeps
            none — a control promising a save it cannot make is what the label
            rule forbids — and dark again once the copy has landed, since the
            local document carries no stamp and there is nothing further to
            send. A reload puts it back, and the server answers that second
            click `identical`. */}
        {/* **`archive` is `useArchiveFact()`'s, and there is no module read
            left** (M6.5b2, spec §2.2): the fact is read when the game ENDS
            rather than at boot, so the value a render reads has to be one that
            changes this screen. `"unknown"` is dark for the same reason
            `"off"` is — nothing answered, so nothing can be promised — and is
            a different state, which is what Abandon's three arms need it to
            be. */}
        {state.over &&
        isLocal(state.id) &&
        archive !== "off" &&
        archive !== "unknown" &&
        lockState !== "lost" ? (
          archived[state.id] ? (
            <span className="meta archived" role="status">
              {archived[state.id]}
            </span>
          ) : (
            <button type="button" className="ghost" onClick={() => void sendToArchive()}>
              Send to the archive
            </button>
          )
        ) : null}
        {/* **The abandon lives on a LOCAL game's own screen** (P-6), where this
            tab holds its lock — the lobby's rows withhold it. It reads "Abandon
            and save" when the SERVER keeps an archive and the game has a move
            in it, because that is where the copy goes. **Whether it has one
            is the game's own fact** (cycle E, R-E5): `table` was read once, at
            open, and a game opened at its deal would go on reading no events
            after every move since; `state.events` is the record's length now. */}
        {isLocal(state.id) && table && lockState !== "lost" ? (
          <Abandon
            table={{ ...table, has_events: state.events > 0 }}
            local
            onDone={() => toDoor()}
            onError={setError}
          />
        ) : null}
      </header>

      {/* **What the poll can say for itself, and `live` says nothing** — on the
          line UNDER the banner, never in its row (cycle E, straw poll row 71).
          A permanent "connected" mark is a thing people stop seeing, so the
          resting state draws nothing. The words that do appear are the ones a
          player would otherwise have to guess at: the board is as old as the
          tab was hidden ("refreshing"), or this browser cannot reach the
          server ("connecting" before its first connection, "reconnecting" once
          one is lost). In the banner's row the word was one more child of a
          row GATED at one line (`the_screen_itself.spec.ts`), and measured at
          1280 it broke the line: 93px against a 56px one-row box. **The strip
          is the live region, mounted whatever the word** (Task 4b's review,
          Minor 5): a region inserted with its text is announced unreliably —
          the end-trigger strip's own reason below — so the words arrive in a
          region that was already there. `styles.css` says where it is drawn at
          each width (ruling 4b-1). */}
      <div className="link-strip" role="status">
        {link !== "live" && <span className="meta link">{link}&hellip;</span>}
      </div>

      {/* How close the game is to ending, in the CLI's own words. Without it a
          player cannot see the game ending, which is not a polish issue.

          `role="status"` here and on the three blocks in the deck below. The
          page had ZERO live regions before this — `grep -rn 'aria-live|role=
          "alert"|role="status"' web/src` returned nothing — so every applied-move
          delta, every engine refusal and the game beginning to end were
          announced to no assistive technology at all.

          And the containers are mounted whether or not they have anything in
          them, which is the half that is easy to get wrong: a live region
          inserted into the DOM at the same instant as its text is unreliably
          announced, because there was no region to observe when the text
          arrived. They collapse to zero height while empty (`styles.css`)
          rather than being removed. */}
      <div
        className={`end-strip ${view.end_game_triggered ? "final" : ""}`}
        role="status"
      >
        {view.end_triggers}
      </div>

      {/* **The table draws the REPLAY position while you are scrubbing.**
          The board never goes inside the panel: measured, a 4P board in the
          704px drawer scrolls 322px horizontally and ~205px vertically, and
          shrinking the tile enough to fit drops the printed power ladder to
          ~5.2px against the 7.9px M3.5 measured as legible. So the panel holds
          the track and the log, and the full-size board behind it redraws at
          `k`.

          **No `onTileClick` on a historical board, ever.** A past position is
          not answerable, so every tile renders as an inert `<div>` rather than
          a button — which is also what makes the panel's modality correct here
          rather than costly: there is nothing behind it to click. */}
      {/* **The table and the side column (M5 spec §3.1).** Above 1201px `.app`
          is a grid: the table is column 1 across rows 3–4 and scrolls on its
          own; the deck is column 2 row 3, pinned by the grid rather than by
          `sticky`; the side column is column 2 row 4 and scrolls on its own.
          Below 1201px nothing here is a grid item — this is the DOM order the
          M3.5 column stack reads, table then side then the sticky deck, so R8
          holds by construction. `main` is the one landmark; the board never
          goes inside a panel (M4 §3.1), and it never has. */}
      <main className={`table ${replaying ? "replaying" : ""}`}>
        <div className="table-body">
          <Board
            board={view.frame.board}
            art={art}
            labels={labels}
            colors={colors}
            selected={replaying || walking ? [] : selected}
            busy={busy}
            changed={highlights?.tiles}
            changedBy={highlightBy}
            {...(picking && !replaying && !walking
              ? { onTileClick: (area: number) => void send(`A${String(area).padStart(2, "0")}`) }
              : {})}
          />
          {/* **Keyed on the game, so a switch does not paint the last game's
              boards.** The fetched strips (port, laboratory, building sites)
              are component state; without the key the previous game's Port
              offer stays on screen until the new game's fetch lands, under a
              board that has already changed. Remounting drops that cache and
              the refusal beside it. */}
          <TableSections
            key={state.id}
            enabled={enabled}
            game={state.id}
            state={state}
            replaying={replaying}
            scrubbing={scrubbing}
            view={view}
            art={art}
            labels={labels}
            colors={colors}
            changedSlots={highlights?.slots}
            live={liveFrame?.game === state.id ? liveFrame.frame : null}
            runFor={runFor?.game === state.id ? runFor.events : null}
          />
        </div>
      </main>
      <aside className="side">
        {/* The walk's state, its timer and its jumps are `App`'s; the account
            is the CONTROL surface over them and holds no clock of its own.
            With no run it renders the reply's prose lines, which is all this
            box could ever hold before the walk existed. */}
        <TurnAccount
          messages={state.messages}
          run={run}
          at={walk?.at ?? null}
          playing={walk?.playing ?? false}
          cadence={cadence}
          labels={labels}
          colors={colors}
          onCadence={(c) => {
            saveCadence(c);
            setCadence(c);
          }}
          onPlay={() => dispatch({ type: "play" })}
          onPause={() => dispatch({ type: "pause" })}
          onBack={() => dispatch({ type: "back" })}
          onForward={() => dispatch({ type: "forward" })}
          onJump={(i) => setWalk({ at: i, playing: false })}
          onSkip={() => dispatch({ type: "skip" })}
          onReplay={() => setWalk({ at: 0, playing: true })}
        />
        <Dashboard
          panels={view.frame.dashboard}
          art={art}
          labels={labels}
          colors={colors}
          only={["Players", "Hand"]}
          changedSeats={highlights?.seats}
          changedTechs={highlights?.techs}
          changedHand={highlights?.hand}
          // **`state`, the live game, and never `view`**: the record's bot
          // seats do not change with the position walked or scrubbed, and a
          // replayed frame is not where they live (M6.5c2, spec §2.4).
          bots={state.bots}
        />
        {/* LAST, and folded (R-6.4.10): a bonus under the account and the
            players, not the main attraction. `key` remounts it per game so a
            switched table starts from its own list rather than carrying another
            room's messages and another room's cursor across.

            Drawn for a WATCHER as well as a player — the one route a spectator
            may write to (R-6.4.1) — and `sendName` is what tells the two apart:
            a seat is named by its seat, and sending a name for one is a 400
            rather than a quiet override (R-6.4.2). */}
        {/* **No Chat panel on a local game** (spec §2.5): the house is the
            server's, and a table in this browser has one person at it. */}
        {!isLocal(state.id) && (
          <Chat
            key={state.id}
            scope={{ kind: "table", id: state.id }}
            count={chatCount}
            rev={chatRev}
            canModerate={table?.mine === true}
            colors={colors}
            sendName={spectating}
          />
        )}
      </aside>

      {surface && (
        <Surface
          name={surface}
          game={state.id}
          art={art}
          labels={labels}
          colors={colors}
          scrubK={scrubK}
          onScrub={setScrubK}
          onClose={() => setSurface(null)}
        />
      )}

      {/* **The answer bar, pinned — and it is ONE bar, deliberately.**
          Measured at 4P before this: the decision zone began at y=2006 on a
          2179px page, 1,106px below the fold, so every move ended in a scroll.
          `.stage` is `max(board, rail)` and the rail runs 2.1–2.5× the board's
          height, so nothing done to the board shortens the page — pinning is
          what makes the page's height stop mattering.

          **Since M5 it has two homes and only one of them is `sticky`.** Above
          1201px the grid puts it in column 2 row 3 with `position: static` and
          nothing scrolls past it; below, this is M3.5's bottom-edge sticky bar,
          unchanged. The e2e cell asserts both arms.

          The three blocks share a container because pinning them separately is
          a measured defect, not a tidiness question. At one `bottom: 0` and one
          z-index they land on the SAME edge, and the error strip is last in the
          DOM: hit-tested, `elementFromPoint` on "Apply the only option" and on
          "Undo" returned `div.error`. Pinning only the middle block instead
          left the error strip at y=2237 — a permanently visible answer bar and
          no visible reason the click did nothing. One container, and neither
          failure is reachable. */}
      <div className="deck">
        <div className="decision notices" role="status">
          {state.warnings.map((w, i) => (
            <div className="warn" key={i}>
              {w}
            </div>
          ))}
        </div>
        {/* **While the walk runs the deck is the strip, not the prompt.** The
            prompt underneath is answerable and would be answered — the walk is
            a picture of what already happened, and a click landing during it
            would be a move made against a table showing an earlier position.
            `skip` is the way out and lands on the live prompt at once.

            Driven by the WALK's own flag rather than by `replaying`, which
            comes off one step early by design (see the view block above). */}
        {walking ? (
          <div className="decision walking" role="status">
            <span className={seatClass(colors[stepNow.mover])}>
              {colors[stepNow.mover] ?? `Seat ${stepNow.mover}`}
            </span>
            {" is playing · step "}
            {walk!.at + 1}
            {" of "}
            {run!.steps.length}
            <button type="button" className="ghost" onClick={() => dispatch({ type: "skip" })}>
              skip
            </button>
          </div>
        ) : lockState === "lost" ? (
          /* **Another tab took this game** (spec §4). The board, the log and
             every panel stay where they are — what is gone is the ability to
             answer, because the writer is elsewhere. `send` refuses the board's
             own clicks for the same reason, which is what makes this read-only
             rather than nearly so. */
          <LockScreen state="lost" onTakeover={retake} inline />
        ) : (
          <Decision
            state={state}
            labels={labels}
            colors={colors}
            busy={busy}
            outcome={outcome}
            spectating={spectating}
            onSend={(l) => void send(l)}
          />
        )}
        {/* `role="alert"` and not `status`: this is the one channel that is
            always a REFUSAL of something the player just did, and it is
            assertive for the same reason the terminal prints it immediately. */}
        <div className="decision notices" role="alert">
          {error && <Refusal sentence={error} />}
        </div>
      </div>
    </div>
  );
}
