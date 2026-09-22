import type {
  AreasPanel,
  ArchiveReply,
  ChatMessage,
  ChatPage,
  ChatPost,
  ChatScope,
  ClientState,
  DealView,
  Events,
  GamesList,
  Health,
  Labels,
  LabsPanel,
  HistoricalFrame,
  LogPanel,
  RunFrames,
  Me,
  PortsPanel,
  IndexModel,
  Outcome,
  SuggestResponse,
  TableView,
  WhyResponse,
} from "./types";
import { useSyncExternalStore } from "react";
import { localCall, localPair } from "./local/client";
import {
  archiveAnswer,
  facts,
  onServerFacts,
  patchServerFacts,
  recallServerFacts,
  rememberServerFacts,
  serverFacts,
  serverFactsTakenAt,
  type ArchiveKind,
  type Host,
} from "./local/facts";
import { gameIdOf, isLocal, LOCAL_PREFIX, LOCAL_USER } from "./local/ids";

/** Which host a game belongs to, and what a local one is called (R-6.5b.12).
 *
 *  **Defined in `local/ids.ts` and re-exported HERE**, which is where every
 *  page reader takes it from: `call`, `recordText`, the transport's arm and
 *  the screen's own reads all go through this one predicate, so "which host is
 *  this game's?" is asked one way. The definition itself lives one file down
 *  because the WORKER draws the ids and compiles without the DOM, while this
 *  module reads `localStorage`. **`LOCAL_USER` rides with them** (R-6.5b.1):
 *  the create form seats it, and the Worker hands it to `dispatch`. */
export { isLocal, LOCAL_PREFIX, LOCAL_USER };
export type { ArchiveKind, Host };
/** The server's remembered facts, re-exported for the same reason `isLocal` is:
 *  a page reader takes what it knows about the server from this module, and
 *  `local/facts.ts` is where the record and its timestamp live. `App` records a
 *  landed health read with `rememberServerFacts`; `recallServerFacts` is what
 *  reads it back without asking.
 *
 *  **`resetFactsForTest` is NOT among them** (review M-3): it is a test hook,
 *  and a cell that wants it imports it from `./local/facts` where it lives. */
export { recallServerFacts, rememberServerFacts };

/** A refusal from the server, carrying the sentence it refused with — and the
 *  STATUS it arrived with, because one caller has to branch on which refusal
 *  this is rather than on how it is worded.
 *
 *  **401 is the whole reason the status is here, and it is a total join**:
 *  `identity.rs` is the only place on this API that answers 401, so the status
 *  alone means "the server does not know who you are" and nothing else. The
 *  sentence cannot do that job — it is one of THREE, one per identity mode
 *  ("no signed-in user: send X-Yokohama-Claimed-User" under `claimed`, and two
 *  different "no identity reached the server…" sentences under Cloudflare
 *  Access and the edge Worker) — so a client matching the first would strand
 *  exactly the deployment that is hosted. */
export class ApiError extends Error {
  readonly status: number;
  /** The refusal body as it arrived, parsed — or `null` where the server sent
   *  nothing, or something that is not JSON.
   *
   *  **One caller branches on a FIELD of it, not on the sentence.** A 409 from
   *  `POST …/join` means two different things: `DealingErrorBody` carries a
   *  `status`, and that one says the table has moved on and the answer is the
   *  next screen; a 409 without one says the seat was taken, and the answer is
   *  to re-read the table. Matching the wording instead would be a client that
   *  breaks when a sentence is reworded. */
  readonly body: unknown;
  constructor(message: string, status: number, body: unknown = null) {
    super(message);
    this.status = status;
    this.body = body;
  }
}

/** The server no longer knows who we are. See `ApiError.status`. */
export const isSignedOut = (e: unknown): boolean => e instanceof ApiError && e.status === 401;

/** The ONE handler for "the server no longer knows who you are", invoked by
 *  `call` on every 401 before the refusal is thrown.
 *
 *  **Central, and not a wiring at each catch site (ruling R-7.1).** Spec §7.1
 *  says a 401 from ANY call returns the player to the door, and a client that
 *  meets that by adding two lines per catch is a client that meets it for the
 *  catches that existed on the day it was written. It was already three of
 *  seven: the opening walk reported into a game screen it had already drawn,
 *  the setup screen's list swallowed the refusal into an empty array and told a
 *  signed-out player they had no games, and `/result`'s catch does not even
 *  RECEIVE the error — `.catch(() => setOutcome(null))` discards it, so no
 *  per-site wiring could ever have routed that one. Here, every call routes by
 *  construction, including the calls no screen has written yet.
 *
 *  One handler, not a list: there is one door, and a second subscriber would be
 *  a second thing racing to decide which screen this is. `App` registers it in
 *  an effect and clears it on unmount. */
let unauthorized: ((sentence: string) => void) | null = null;

export function onUnauthorized(handler: ((sentence: string) => void) | null): void {
  unauthorized = handler;
}

/** Where the sign-in screen keeps the claimed user (R-plan-2). Read on every
 *  call; absent means no header. A value is only ever WRITTEN by the sign-in
 *  screen, which only exists in `claimed` mode, and the server ignores this
 *  header under any other mode (spec R-1.2). */
export const CLAIMED_USER_KEY = "yokohama.claimed_user";

/** The claimed user, or `null`. A storage that throws — a private window, a
 *  blocked site — reads as nobody and never as an error, the `sections.ts`
 *  idiom. */
export function claimedUser(): string | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage.getItem(CLAIMED_USER_KEY);
  } catch {
    return null;
  }
}

/** Remember who this browser plays as. Written by the sign-in screen and by
 *  nothing else, so the stored name is always one a person typed here.
 *
 *  A storage that refuses — a private window, a blocked site — is not an error
 *  to report: the header is simply not sent, the server answers `user: null`,
 *  and the player is asked again. Signing in is not a thing that can half
 *  happen. */
export function setClaimedUser(email: string): void {
  try {
    localStorage.setItem(CLAIMED_USER_KEY, email);
  } catch {
    /* storage refused: the header is simply not sent */
  }
}

/** Forget it. Sign-out, and also what a `no signed-in user` refusal means: the
 *  name we hold is not one this server will answer to, so holding it only
 *  buys another refusal. */
export function clearClaimedUser(): void {
  try {
    localStorage.removeItem(CLAIMED_USER_KEY);
  } catch {
    /* nothing to clear */
  }
}

/** Where the form keeps what this browser calls itself (spec §5.3).
 *
 *  **A CONVENIENCE, and never an identity.** Who you are is the server's answer
 *  at `/api/me`; this is only what a seat you take is LABELLED with, so it is
 *  sent as a field on a create or a join and never as a header. A browser with
 *  nothing here simply seats an address. */
export const DISPLAY_NAME_KEY = "yokohama.display_name";

/** What this browser calls itself, or `null`. A storage that throws — a private
 *  window, a blocked site — reads as nobody, the `claimedUser` idiom. */
export function displayName(): string | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage.getItem(DISPLAY_NAME_KEY);
  } catch {
    return null;
  }
}

/** Remember it, so the next table does not ask again. A storage that refuses is
 *  not an error to report: the field is simply blank next time. */
export function setDisplayName(name: string): void {
  try {
    localStorage.setItem(DISPLAY_NAME_KEY, name);
  } catch {
    /* storage refused: the field is blank next time */
  }
}

/** Forget it. The `clearClaimedUser` idiom, and wanted for the same reason
 *  signing out wants that one: what this browser calls itself is a convenience
 *  attached to whoever was signed in, and leaving it behind would label the next
 *  person's seats with the last person's name. */
export function clearDisplayName(): void {
  try {
    localStorage.removeItem(DISPLAY_NAME_KEY);
  } catch {
    /* nothing to clear */
  }
}

/** The identity header this browser sends, as a plain object — `{}` where
 *  there is nothing to send.
 *
 *  **Exported because the change stream is a `fetch` this module does not
 *  make** (M6.3 §5.1). `readStream` in `sse.ts` opens `…/stream` itself, since
 *  `call` reads a whole body and a stream has no end to read to; the header has
 *  to come from HERE rather than be restated there, because a second statement
 *  of it is a second thing to forget when the door changes. It is also why the
 *  reader is a `fetch` and not an `EventSource`: `EventSource` cannot carry a
 *  header at all, so the `claimed` door could not sign a stream. */
export function identityHeaders(): Record<string, string> {
  const who = claimedUser();
  return who ? { "x-yokohama-claimed-user": who } : {};
}

/** What EVERY request to the server carries: the identity header, and the one
 *  header that makes an expired Access session readable (M6.5b2, spec §2.1).
 *
 *  **`X-Requested-With` is a MEASUREMENT and not a convention** (spec §0.3,
 *  taken against the live deployment on 2026-09-21). Cloudflare Access answers
 *  a request without it with a `302` to `bdvc.cloudflareaccess.com`, which is
 *  another origin: a `fetch` follows it and then fails opaquely, so the client
 *  sees a network error and says so. With it, the answer is a same-origin `401`
 *  carrying `WWW-Authenticate: Cloudflare-Access …`, whose headers this client
 *  can read. `Accept: application/json`, `Sec-Fetch-Mode: cors` and
 *  `Sec-Fetch-Dest: empty` were each measured NOT to trigger it.
 *
 *  **Shared by all THREE fetch sites**, and that is the whole reason it is a
 *  function: `call` is not the only one — `readStream` opens the change stream
 *  itself and `recordText` reads a body as text — so a header added at one site
 *  would leave a stream reconnect and Save-game-file reporting an expired
 *  session as an ordinary network failure. */
export function serverHeaders(): Record<string, string> {
  return { ...identityHeaders(), "X-Requested-With": "XMLHttpRequest" };
}

/** Whether this response is the EDGE saying the session is gone, rather than
 *  the server saying nobody is signed in.
 *
 *  The header is the whole test: Access names itself in `WWW-Authenticate`, and
 *  this API never sends that header at all. The two 401s mean different things
 *  and must not render the same sentence — one is fixed by passing the
 *  turnstile, the other by signing in. */
export function isAccessChallenge(res: Response): boolean {
  return res.headers.get("www-authenticate")?.startsWith("Cloudflare-Access") ?? false;
}

/** What the player reads when the EDGE refused (spec §4).
 *
 *  A sentence and not a status, and the one string the screen matches on to
 *  draw the link to `/enter` beside it — a navigation, never a fetch, because
 *  passing the turnstile is what mints the cookie every later request carries. */
export const ACCESS_CHALLENGE = "Your session has expired.";

/** How long the SERVER is given to answer a read before this client acts on
 *  the silence.
 *
 *  **Because a read that HANGS never rejects**: a `fetch` against a server that
 *  accepts the connection and then says nothing settles neither way, so a
 *  `.catch` alone is a fallback that can never be reached. Long enough that a
 *  slow link is not read as an outage, short enough that a player is not left
 *  watching raw serde names while a dead connection expires on its own.
 *
 *  **Here rather than in `App`, since M6.5b2**, because the facts read below
 *  is bounded by the same number the words on the cards are — two reads, one
 *  deadline, and a second constant would be a second thing to keep in step. */
export const BOUND_MS = 5_000;

/** `p`, or a rejection once `ms` has passed.
 *
 *  A race rather than a catch, for the reason [`BOUND_MS`] gives. The loser's
 *  timer is left to fire and reject a promise nobody is holding, which is
 *  harmless: `Promise.race` has already settled. */
export const bounded = <T,>(p: Promise<T>, ms: number): Promise<T> =>
  Promise.race([
    p,
    new Promise<T>((_, reject) => setTimeout(() => reject(new Error("timed out")), ms)),
  ]);

/** Which change transport this DEPLOYMENT serves, read once from
 *  `GET /api/health` on entering the multiplayer channel (M6.5b2: no longer at
 *  boot — the door asks nothing) and then read by `usePolled` on every open.
 *
 *  **A deployment fact and never a preference** (spec R-3.1): the edge in front
 *  of this server may not hold a stream open, and the held request is the same
 *  server's answer to that. The default is the stream, so a health that does
 *  not answer leaves the client reading the transport the server prefers —
 *  a wrong guess here costs a failed open and one trip round the ladder, not a
 *  screen that never updates.
 *
 *  **Per SOURCE since M6.5b** (spec §2.3): the value lives in
 *  `local/facts.ts`, keyed by server or local, and this reads the SERVER's.
 *  Its `"sse" | "held"` union is untouched — the transport arm never asks this
 *  about a local game, because it reads `Source.host` instead. */
export const signalKind = (): "sse" | "held" => serverFacts().signal;

export const setSignalKind = (k: "sse" | "held"): void => {
  patchServerFacts({ signal: k });
};

/** Whether this DEPLOYMENT answers the house chat (M6.4, R-6.4.6), read once
 *  from the same `GET /api/health` the transport above comes from.
 *
 *  Module-level for the reason `signalKind` is: one value for the life of the
 *  page, read by components that must not re-render when it is set. The default
 *  is ON, so a health that does not answer leaves the client asking for a panel
 *  the server may refuse with a 404 it already renders — a wrong guess here
 *  costs one refusal, never a screen that never appears. */
export const lobbyChat = (): boolean => serverFacts().lobbyChat;

export const setLobbyChat = (on: boolean): void => {
  patchServerFacts({ lobbyChat: on });
};

/** Characters per message, from the same `/api/health` (R-6.4.8) — what the
 *  input is sized with.
 *
 *  **Read from the server and never restated as a constant here.** The cap is
 *  a launch variable, so a copy in this file is a number that can be wrong on
 *  the day an operator changes it — and wrong in the direction where the box
 *  accepts what the server will refuse. The default matches the server's own
 *  only so a client that has not reached `/api/health` yet has a box. */
export const chatText = (): number => serverFacts().chatText;

export const setChatText = (n: number): void => {
  patchServerFacts({ chatText: n });
};

/**
 * Which archive this deployment writes finished and abandoned games to, as
 * THIS BROWSER last heard it — or `"unknown"` (M6.5a R-6.5a.13; M6.5b2
 * R-6.5b2.13).
 *
 * **One named CROSS-READ** (spec §2.3): a LOCAL game's Abandon label reads
 * this — the server's — because "Abandon and save" sends the pair to the
 * server's archive. Every other fact a local screen reads is `facts("local")`'s.
 *
 * **A HOOK, and it is the only synchronous reader left** (review M-9). The
 * fact is filled at three moments a render has already happened at
 * (`ensureArchiveFact`), so a module value read during render would leave the
 * send control and the Abandon label drawing whatever they mounted with — and
 * a second accessor beside `App`'s own state was two readers kept in step by
 * hand. `onServerFacts` is what re-renders them.
 *
 * **`"unknown"` is not `"off"`**, which is the whole of R-6.5b2.13: `"off"` is
 * a claim about the deployment and a browser that has never asked has no
 * grounds for it.
 */
export const useArchiveFact = (): ArchiveKind | "unknown" =>
  useSyncExternalStore(onServerFacts, archiveAnswer, archiveAnswer);

/** Which DOOR this deployment reads, as this browser last heard it — `""`
 *  where nothing has ever said (M6.5b2, spec §2.6).
 *
 *  **A hook for [`useArchiveFact`]'s reason** (review M-5): the outbox's
 *  "not signed in" remedy is two steps under `claimed` and one under the other
 *  two doors, and a module read during render is a value that never redraws
 *  when the health answer lands. The same shape the archive fact retired one
 *  file over; restating it for the identity would have been the same defect. */
const identityMode = (): string => serverFacts().identity;

export const useIdentityMode = (): string =>
  useSyncExternalStore(onServerFacts, identityMode, identityMode);

/** How long a remembered health answer is trusted without asking again. */
const FACTS_FRESH_MS = 3_600_000;

/** The archive fact, read if it has to be (M6.5b2, spec §2.2).
 *
 *  **Three read points and no boot read.** The door costs the Worker nothing
 *  (R-6.5b2.7), so the health read moved to the moments the answer is actually
 *  used: entering the multiplayer channel, a local game reaching game over, and
 *  Abandon being clicked on a local game. Browsing and playing still cost zero
 *  invocations; the cost is at most one per browser, at the moment a game ends,
 *  on a person who has already decided to keep something.
 *
 *  **A remembered answer goes stale and must be able to.** A deployment that
 *  turns its archive on after a browser once read `"off"` would otherwise have
 *  that browser deleting finished games for ever, with no mechanism that could
 *  correct it — so anything older than an hour is re-read here, and `force` is
 *  for a decision that cannot be undone.
 *
 *  **`"unknown"` is not a synonym for `"off"`.** It is the answer when nothing
 *  has ever been read and nothing answers now: "off" is a claim about the
 *  deployment, and a browser that has not asked has no grounds for it. */
export async function ensureArchiveFact(opts?: {
  /** Read the server again however fresh the remembered answer is — what a
   *  delete that cannot be undone asks for. */
  force?: boolean;
}): Promise<ArchiveKind | "unknown"> {
  const at = serverFactsTakenAt();
  // **`Math.abs`, because a clock can move BACKWARDS** (review M-7). A
  // `takenAt` in the future read as fresh for ever, so a device whose clock had
  // jumped never revalidated — and never learned that the archive had been
  // turned on.
  const fresh = at !== null && Math.abs(Date.now() - at) < FACTS_FRESH_MS;
  if (fresh && !opts?.force) return facts("server").archive;
  try {
    // **Through `api.health`, and bounded by the same number the words are**: a
    // read that hangs would otherwise leave a caller waiting for ever for an
    // answer it is about to act on.
    const h = await bounded(api.health(), BOUND_MS);
    rememberServerFacts(h);
    return h.archive;
  } catch {
    // Nothing answered. A stale answer is still the last thing this deployment
    // said; with nothing remembered at all there is nothing to say.
    return at === null ? "unknown" : facts("server").archive;
  }
}

/** What a player can DO about a write this browser would not keep (M6.5b2,
 *  R-6.5b2.9; spec §2.4).
 *
 *  **One sentence appended, and no second screen.** A quota failure already
 *  had a path — the line below — and this cycle points that line at the
 *  envelope rather than inventing a place to say it again.
 *
 *  **It names the control that is ALREADY IN FRONT OF THE PLAYER**: this
 *  warning is drawn on the game screen, and `App.tsx:1834` puts `BackUpOne` on
 *  a local game's own banner. The same words also name the one on the row on
 *  the first screen, which is the same component; a sentence that said WHERE
 *  would be wrong on one of the two screens it is read on. */
const BACKUP_POINTER =
  'back this game up \u2014 the "Back up" button on it writes a file this app can restore.';

/** The move happened and the browser would not keep it (spec §4).
 *
 *  **The server's own wording, and the server's own place for it**
 *  (`api.rs:1105`): a warning on the applied reply, never a refusal — the move
 *  DID happen.
 *
 *  **One rule and one sentence across both hosts, up to the pointer** (review
 *  M-1, correcting a sentence that had claimed the whole line was identical).
 *  The prefix and the reason are carried VERBATIM (spec §4: "APPLIED BUT NOT
 *  SAVED" is carried from M6.5b, not restated), so a player who has run out of
 *  room reads the same refusal whichever host they are on. What M6.5b2 adds
 *  after them is a fact about THIS BROWSER and a control only this browser
 *  has, so it rides this arm alone — and this function has no other caller:
 *  `call`'s server branch never reaches it, because a hosted game's write is
 *  the server's problem and has the server's own remedy. */
function withUnsaved<T>(out: T, reason: string | null): T {
  const warned = out as { warnings?: unknown };
  if (out && typeof out === "object" && Array.isArray(warned.warnings)) {
    warned.warnings = [
      ...(warned.warnings as string[]),
      `APPLIED BUT NOT SAVED — ${reason ?? "the browser refused the write"} — ${BACKUP_POINTER}`,
    ];
  }
  return out;
}

async function call<T>(method: string, path: string, body?: unknown, host?: Host): Promise<T> {
  // **The id decides, unless the caller says** (R-6.5b.12). Five routes carry
  // no id — the create, the listing, `/api/health`, `/api/me`, `/api/labels` —
  // and for those the caller passes its intent; everything else is answered by
  // whichever host owns the game in the path.
  const local = host === "local" || (host === undefined && isLocalOf(path));
  if (local) {
    // `identityHeaders()` is not read here: the Worker adds `local@browser`
    // and nothing on this device signs in.
    const r = await localCall(method, path, body);
    // Nothing is in front of the module in this browser, so nothing can
    // challenge it: the local arm is never an expired session.
    const out = settle<T>(r.status, "", r.text, false);
    return r.saved ? out : withUnsaved(out, r.reason);
  }
  const headers: Record<string, string> = { ...serverHeaders() };
  if (body !== undefined) headers["content-type"] = "application/json";
  const res = await fetch(path, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  return settle<T>(res.status, res.statusText, await res.text(), isAccessChallenge(res));
}

/** Whether a path names a LOCAL game. Its own function so the two callers —
 *  `call` and nothing else today — read one sentence. */
const isLocalOf = (path: string): boolean => {
  const game = gameIdOf(path);
  return game !== null && isLocal(game);
};

/** Read the status, parse the body, refuse or hand it over — the tail both
 *  hosts share.
 *
 *  Extracted verbatim from `call` at M6.5b: a local reply arrives as a status
 *  and a string exactly as a fetched one does, and a second copy of this would
 *  be a second place where a refusal is worded or a 401 reaches the door. */
function settle<T>(
  status: number,
  statusText: string,
  text: string,
  /** Whether the response carried Access's own `WWW-Authenticate` — see
   *  [`isAccessChallenge`]. The two 401s are different refusals and the caller
   *  is the only one holding the `Response` that tells them apart. */
  challenged: boolean,
): T {
  // **The status is read BEFORE the body is parsed, and the parse cannot throw
  // over it.** This used to be an unguarded `JSON.parse` above the `res.ok`
  // check, so any non-JSON response became a `SyntaxError` and the status code —
  // the one actionable fact — was discarded. The documented deployment is behind
  // Cloudflare Access, where an expired session answers with an HTML
  // interstitial: the player saw "Unexpected token '<'" and nothing about being
  // logged out. A 502 from a proxy has the same shape.
  let parsed: unknown = null;
  let parseFailed = false;
  if (text) {
    try {
      parsed = JSON.parse(text);
    } catch {
      parseFailed = true;
    }
  }

  // `statusText` is empty over HTTP/2 and for a local reply, which has no
  // status LINE at all — so the fallback is trimmed rather than left with a
  // trailing space.
  const bare = `${status} ${statusText}`.trim();
  // **The EDGE's 401, which is not this API's** (M6.5b2, spec §2.1). The
  // session in front of the server expired; the name this browser holds is
  // fine. So the sentence is the expired-session line — the screen draws the
  // turnstile's link beside it — and `unauthorized` is deliberately NOT fired:
  // dropping the claimed user would sign the player out of a deployment that
  // never asked them to, and the form they would then be shown cannot mint the
  // cookie that is actually missing.
  if (status === 401 && challenged) throw new ApiError(ACCESS_CHALLENGE, status, null);
  if (status < 200 || status >= 300) {
    const message =
      !parseFailed && parsed && typeof parsed === "object" && "error" in parsed
        ? String((parsed as { error: unknown }).error)
        : bare;
    // **Before the throw, so it reaches the door whatever the caller does with
    // the refusal** — including discarding it, which one caller does.
    if (status === 401) unauthorized?.(message);
    throw new ApiError(message, status, parseFailed ? null : parsed);
  }
  if (parseFailed) {
    // A 2xx that is not JSON is this API answering with something it does not
    // serve — an interstitial, a proxy page. Say that, rather than a parser's
    // complaint about the first character of it.
    throw new ApiError(`${bare}: the server answered with something that is not JSON`, status);
  }
  return parsed as T;
}

/** One seat of a table being opened: a person's email, the bot, or nobody yet.
 *  Exactly one of the three — a seat naming two is a 400 that says so, and so
 *  is a seat naming none (`{bot: false}` alone is not a human seat with no
 *  email). */
export type SeatRequest = { user: string } | { bot: true } | { open: true };

export interface NewGameRequest {
  players: number;
  colors: string[];
  start_seat: number;
  /** A decimal STRING: a `u64` seed does not survive a JS number, so a client
   *  that sent its own `Number` would open a different game from the one it
   *  asked for. ONE-WAY: nothing on this wire ever sends a seed back. */
  seed?: string;
  /** One entry per seat, in seat order. **The creator must hold one of them**
   *  (R-1.6), and a bot is refused at three and four players (R-P10). */
  seats: SeatRequest[];
  /** The table's LABEL since M6.2 — free text the lobby prints, never the id. */
  name?: string;
  /** The storage id. The browser never sends one: the server draws it, and a
   *  client-chosen id is a name two people can collide on. Declared because the
   *  fixture harness and a hand-written `curl` do send one. */
  id?: string;
  /** What the CREATOR is called on screen, written onto every seat they hold
   *  and onto no other — a name is a claim about the person who gave it. */
  display_name?: string;
  /** The table password (R-2.1). Stored salted and hashed, and never served
   *  back in any form. */
  password?: string;
}

/** Taking an open seat at an open table. The password is the TABLE's, not the
 *  caller's: an unlocked table ignores one, and a locked table refuses without
 *  it. `name` is what this seat will be called on screen. */
export interface JoinRequest {
  seat: number;
  name?: string;
  password?: string;
}

/** Knocking at a locked table one is not seated at (M6.3).
 *
 *  The same password the table's seats were asked for — it gates JOINING and
 *  WATCHING, and nothing else — sent to be remembered, so the caller does not
 *  present it again on every poll. */
export interface UnlockRequest {
  password: string;
}

const id = (s: string) => encodeURIComponent(s);

/** The most positions one run call answers — the server's `RUN_CAP` (M6.5b).
 *  Above it the run is paged in cap-sized calls; the server never clamps. */
export const RUN_CAP = 64;

/** `?a=1&b=2` for the arguments that were given, and `""` for none. Written for
 *  `events`, which since M6.3 takes two optional numbers and would otherwise
 *  need four spellings of its own path. */
const query = (args: Record<string, number | undefined>): string => {
  const parts = Object.entries(args)
    .filter(([, v]) => v !== undefined)
    .map(([k, v]) => `${k}=${v}`);
  return parts.length ? `?${parts.join("&")}` : "";
};

/** The finished game's record as TEXT, never parsed (M6.5a, R-6.5a.13; umbrella
 *  R-6.5.6): `GameRecord.seed` is a u64 that serde writes at full precision and
 *  `JSON.parse` would round to a double, so a record that went through this
 *  client's parser and back would carry a DIFFERENT seed silently. The status
 *  is read exactly as `call` reads it — a refusal is the server's `{"error"}`
 *  sentence, a 401 reaches the door — but a 200 body is handed on as the string
 *  it arrived as. */
export async function recordText(game: string): Promise<string> {
  if (isLocal(game)) {
    // **The Worker's SIDE CHANNEL, not the `…/record` route** (spec §2.3): the
    // local dispatch answers that route exactly as the corpus pins it — a 409
    // while the game runs — and R-6.5b.7 offers "Save game file" at any point,
    // because it is the player's own seed on the player's own disk. The bytes
    // are the store's, and are not parsed on the way here either.
    // **ONE queued read of BOTH documents** (spec §2.2's export snapshot, review
    // finding #6): the record alone is what this call wants, but a second
    // message for the table — which "Send to the archive" needs beside it —
    // could fall either side of a move.
    const text = (await localPair(game)).record;
    if (text === null) {
      throw new ApiError("this game has no saved record on this device", 404, null);
    }
    return text;
  }
  const res = await fetch(`/api/games/${id(game)}/record`, { headers: { ...serverHeaders() } });
  const text = await res.text();
  if (!res.ok) {
    // **`settle`'s refusal half, rather than a second copy of it** (M6.5b2).
    // This site restated the sentence rule, the 401 rule and — once there were
    // two kinds of 401 — would have restated the challenge read too. It always
    // throws for a non-2xx, which is why the return below is still reached only
    // on success.
    settle<never>(res.status, res.statusText, text, isAccessChallenge(res));
  }
  return text;
}

/** Where a chat panel reads and writes: one table, or the house.
 *
 *  **One path function and not two calls**, because the two scopes differ only
 *  in their address and a client that branched at each call site would have the
 *  branch in as many places as it has panels. */

const chatPath = (scope: ChatScope, after?: number): string =>
  (scope.kind === "house" ? "/api/chat" : `/api/games/${id(scope.id)}/chat`) + query({ after });

export const api = {
  /** Who the server thinks we are, and which header it reads. Never refuses:
   *  `user: null` is the answer, not an error. */
  me: (host?: Host) => call<Me>("GET", "/api/me", undefined, host),
  /** The engine's compatibility keys, the identity mode, and — since M6.3 —
   *  which change transport to read. Fetched once at boot. */
  health: (host?: Host) => call<Health>("GET", "/api/health", undefined, host),
  /** The words on the cards.
   *
   *  **`host` is what makes them survive an unreachable API** (spec §2.3): the
   *  server is asked first, under a five-second bound, and the WORKER answers
   *  when that read fails or times out — the module carries the same registry
   *  the server does. */
  labels: (host?: Host) => call<Labels>("GET", "/api/labels", undefined, host),
  /** One page of the lobby, newest first.
   *
   *  Three scopes and not a filter: `mine` is every table you sit at, at every
   *  status, `open` is every table still filling its seats, whoever made it,
   *  and `playing` is every game under way or finished, whoever is in it
   *  (M6.3, R-3.9) — the list a watcher finds a game from without a link. A
   *  `playing` row does not promise the caller may watch it: a locked table is
   *  listed with `can_watch: false`, which is the unlock face.
   *
   *  `before` is the previous page's `next` cursor, sent back untouched — no
   *  `limit` from here, because the server's default page is the one this
   *  client asks for. */
  list: (scope: "mine" | "open" | "playing" = "mine", before?: string, host?: Host) =>
    call<GamesList>(
      "GET",
      `/api/games?scope=${scope}${before ? `&before=${encodeURIComponent(before)}` : ""}`,
      undefined,
      host,
    ),
  /** Opens a table, and starts NOTHING: the reply is `status: "dealing"` and
   *  every seat is owed a deal it fetches for itself. */
  create: (req: NewGameRequest, host?: Host) => call<TableView>("POST", "/api/games", req, host),

  /** Send a local game to the SERVER's archive (R-6.5b.6) — the record and the
   *  local table document, both as the strings they are stored as.
   *
   *  **Explicitly the server**, and the one call a local game makes over the
   *  network: the record's bytes are what the terminal digest is over, so they
   *  are sent exactly as the browser holds them. The server parses both,
   *  replays strictly and chooses the leaf from the replayed state — never
   *  from what this client says. */
  archive: (record: string, table: string) =>
    call<ArchiveReply>("POST", "/api/archive", { record, table }, "server"),
  /** Who sits where, and how far the dealing has got.
   *
   *  **The one route widened to any signed-in caller at an OPEN table**
   *  (R-2.7): a join link is an address, and the person following one has to
   *  see what they are joining. `my_seats` is then empty. */
  table: (game: string) => call<TableView>("GET", `/api/games/${id(game)}/table`),
  /** Take one open seat. Refused when the table is not open any more, when the
   *  seat is taken, and when the table is locked and the password is absent or
   *  wrong — each with the sentence that says which. The join that fills the
   *  LAST open seat flips the table to `dealing` in the same write. */
  join: (game: string, req: JoinRequest) =>
    call<TableView>("POST", `/api/games/${id(game)}/join`, req),
  /** Knock, and be remembered (M6.3, R-3.5). The reply is the table with
   *  `can_watch` turned over, so the screen that was drawing the unlock face
   *  has what it needs to draw the game without a second request.
   *
   *  Refused 403 on the wrong password, and 409 carrying the status at a table
   *  with nothing to watch yet — a table still filling its seats, or still
   *  dealing. A table with NO lock answers 200 and writes nothing. */
  unlock: (game: string, password: string) =>
    call<TableView>("POST", `/api/games/${id(game)}/unlock`, { password }),
  /** The caller's OWN two cards, over the set table. `seat` and `cards` are
   *  null together once every seat this caller holds has chosen. */
  deal: (game: string) => call<DealView>("GET", `/api/games/${id(game)}/deal`),
  /** Choose, for one seat the caller holds. The LAST keep at the table starts
   *  the game, and the reply then reads `status: "playing"`.
   *
   *  `name` is what THIS seat is called on screen (spec §5.6) — the browser's
   *  remembered display name, sent with the keep because a person who followed
   *  a link to a table somebody else opened for them has had no other moment to
   *  say it. Omitted where there is none: a seat with no name is listed by its
   *  address, which is the pre-M6.2 shape. */
  keep: (game: string, seat: number, card: 1 | 2, name?: string) =>
    call<TableView>("POST", `/api/games/${id(game)}/keep`, {
      seat,
      card,
      ...(name ? { name } : {}),
    }),
  get: (game: string) => call<ClientState>("GET", `/api/games/${id(game)}`),
  /** The record of a FINISHED game the caller sits at, as text — see
   *  `recordText`. 409 while the game runs, 403 for a watcher. */
  recordText,
  remove: (game: string) => call<null>("DELETE", `/api/games/${id(game)}`),
  /** The one mutation: an option's `number`, or a sub-prompt token. The server
   *  fences this to gameplay lines; anything else comes back as a refusal that
   *  says what to send instead.
   *
   *  `expect` is the `prompt.kind` this client was looking at. The server
   *  refuses with a 409 if it is asking something else — which is what a
   *  restart looks like from here, since an open picker lives only in the
   *  server's memory and a stale area id would otherwise be read as an option
   *  number. */
  input: (game: string, line: string, expect?: string) =>
    call<ClientState>("POST", `/api/games/${id(game)}/input`, { line, expect }),

  ports: (game: string) => call<PortsPanel>("GET", `/api/games/${id(game)}/ports`),
  labs: (game: string) => call<LabsPanel>("GET", `/api/games/${id(game)}/labs`),
  areas: (game: string) => call<AreasPanel>("GET", `/api/games/${id(game)}/areas`),
  log: (game: string, tail?: number) =>
    call<LogPanel>("GET", `/api/games/${id(game)}/log${tail === undefined ? "" : `?tail=${tail}`}`),

  /** The position as it stood after `k` events — the replay projection.
   *
   *  **`k` counts events APPLIED, so the domain is `0..=total` inclusive** and
   *  a track over it has `total + 1` stops. The last stop is the present: a
   *  real position with no event to name, and the one the server answers with
   *  a frame equal to the live one. A `k` past the end is a 404 naming the
   *  bound, not a clamp.
   *
   *  The frame is built for a seat the CALLER holds — the mover when it is
   *  theirs, otherwise their lowest (R-4.2) — so walking every `k` shows the
   *  caller's own hands and nobody else's. */
  at: (game: string, k: number) =>
    call<HistoricalFrame>("GET", `/api/games/${id(game)}/at/${k}`),

  /** The frames of `[from, to)` — ONE forward replay on the server instead of
   *  one `at/{k}` per position (M6.5b, R-6.5b.10). `to` may be `events + 1`:
   *  the run then ends on the live frame. Above `RUN_CAP` positions the server
   *  answers a 400 naming the cap, so `fetchRun` pages. */
  run: (game: string, from: number, to: number) =>
    call<RunFrames>("GET", `/api/games/${id(game)}/at?from=${from}&to=${to}`),

  /** What the production bot would play on this seat's own view.
   *
   *  Runs a real search, and is the only pair of calls on this API that does.
   *  It applies, records and saves NOTHING — so a refusal here (a bot is to
   *  move; the game is over) arrives as a 200 with `refusal` set, not as an
   *  error, and the panel renders the sentence. */
  suggest: (game: string) => call<SuggestResponse>("GET", `/api/games/${id(game)}/suggest`),

  /** A bot move, reconstructed by replay. `event` is the log's printed 0-based
   *  index; omitted means the most recent bot move.
   *
   *  Same refusal shape as `suggest`, and twelve typed reasons rather than one:
   *  the move was a human's, the seat played a sampled rung, the record does not
   *  replay, the reconstruction did not equal what was recorded. */
  why: (game: string, event?: number) =>
    call<WhyResponse>(
      "GET",
      `/api/games/${id(game)}/why${event === undefined ? "" : `?event=${event}`}`,
    ),

  /** A card index, as a VIEW: `kind` is one of orders/techs/goals/sites, and
   *  `filter` is the CLI's own grammar — an unparseable term is a 400 carrying
   *  the CLI's refusal, which names what it would have accepted. */
  index: (game: string, kind: string, filter?: string) =>
    call<IndexModel>(
      "GET",
      `/api/games/${id(game)}/index/${kind}${filter ? `?filter=${encodeURIComponent(filter)}` : ""}`,
    ),

  /** The final score. A 409 while the game is still running. */
  result: (game: string) => call<Outcome>("GET", `/api/games/${id(game)}/result`),

  /** The change signal: four fields, and no frame built (R-P9).
   *
   *  What a poll asks for while it is somebody else's turn — and what the
   *  setup screen asks to learn that the last seat has chosen, since a dealing
   *  table answers here rather than refusing.
   *
   *  `after` is the RESUME CURSOR. This server validates it and does not act on
   *  it: the reply is immediate whether or not `events` has moved past it. It
   *  is sent anyway, because M6.3's held request and stream answer this same
   *  shape with this same cursor, and a caller written against it keeps working
   *  when the transport underneath is replaced. */
  events: (game: string, after?: number, wait?: number) =>
    call<Events>(
      "GET",
      `/api/games/${id(game)}/events${query({ after, wait })}`,
    ),

  /** The retained messages above `after`, and the newest number ever assigned.
   *
   *  **`last` is the DOCUMENT's and not the page's**, which is what makes the
   *  cursor survive a trim: a client asking from a number retention has already
   *  dropped is told where the room has got to rather than left to infer it from
   *  an empty page. Admitted exactly as a read is — a seat holder at any status,
   *  a watcher once the table admits them — and NARROWER than seeing the table:
   *  a stranger on the seat picker reads the table and is 403 here.
   *
   *  At the house it is any signed-in caller, and a 404 where the deployment has
   *  the lobby chat off (see {@link lobbyChat}). */
  chat: (scope: ChatScope, after = 0) => call<ChatPage>("GET", chatPath(scope, after)),

  /** Say something. The reply is the MESSAGE, numbered, so the room can render
   *  it without re-reading the page.
   *
   *  `name` is a WATCHER's, and anyone's at the house: a seated caller is named
   *  by their seat and sending one is a 400 rather than a quiet override, so a
   *  client that believed it was setting a name hears about it. */
  postChat: (scope: ChatScope, body: ChatPost) =>
    call<ChatMessage>("POST", chatPath(scope), body),
};
