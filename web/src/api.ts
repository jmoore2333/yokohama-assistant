import type {
  AreasPanel,
  ArchiveReply,
  LeaveReply,
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
import { deviceId } from "./device";
import { EDGE_CHECK, EDGE_CHECK_REPEATED, isEdgeChallenge, passChallenge, passesSoFar } from "./turnstile";

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
 *  ("no signed-in user: send X-Yokohama-Claimed-User" under `open`, and two
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

/** The EDGE's challenge, not passed (cycle F, R-F6; spec §2.6): the request met
 *  the no-Access door's Managed Challenge rule and no check was passed for it —
 *  none could be shown ([`EDGE_CHECK`]), or the one shown did not pass, or the
 *  retried request was challenged again ([`EDGE_CHECK_REPEATED`]).
 *
 *  **An `ApiError` with the edge's own status, 403**, so every reader that
 *  branches on the status already reads it as the refusal it is — a held
 *  signal ENDS on it (`signal.ts`) rather than climbing its ladder against a
 *  check only a person can pass. Its own class, because one reader must tell
 *  it from this API's 403s: the outbox marks a row "waiting for a human check"
 *  for this and for nothing else. */
export class EdgeRefusal extends ApiError {
  constructor(message: string) {
    super(message, 403, null);
  }
}

/** Whether this refusal is the edge's challenge rather than anything this API
 *  said — see [`EdgeRefusal`]. */
export const isEdgeRefusal = (e: unknown): boolean => e instanceof EdgeRefusal;

/** How a server request may meet the edge's challenge.
 *
 *  `interactive: false` is the outbox's background pass (the `online` event, a
 *  launch in a hidden tab): the widget is never shown for it, and a challenge
 *  is the refusal at once — which the outbox turns into its row's mark. The
 *  default, `true`, is everything a person is looking at. */
export interface CallOptions {
  interactive?: boolean;
}

/**
 * Send a request to the server, passing the edge's challenge ONCE if it meets
 * one (cycle F, R-FP8; spec §2.6).
 *
 * **The one place a fetched response is asked [`isEdgeChallenge`]** for `call`
 * and the record download, which both send through here — so the predicate,
 * the widget and the retry cannot drift between them. The stream reader asks
 * the same predicate of its own open (`sse.ts`), and the socket's probe is a
 * `call`.
 *
 * **The retry is ONE.** A response that is the edge's challenge shows the
 * widget through the seam ([`passChallenge`]) and sends the request again,
 * whole; a second challenge on that request is the refusal it is, and nothing
 * is sent a third time. Where no widget can be shown — a build with no site
 * key mounts no overlay, and `interactive: false` must not show one — the
 * challenge is the refusal at once, with no second request.
 *
 * **And the widget is not shown again for a challenge OLDER than a pass**: a
 * screen's requests go out together, and one sent before the cookie existed
 * can be answered with the challenge after another request's check has
 * passed. That one is simply sent again — its one retry — under the clearance.
 */
async function pastTheEdge(send: () => Promise<Response>, opts?: CallOptions): Promise<Response> {
  const before = passesSoFar();
  const first = await send();
  if (!isEdgeChallenge(first)) return first;
  if (opts?.interactive === false) throw new EdgeRefusal(EDGE_CHECK);
  if (passesSoFar() === before) {
    try {
      await passChallenge();
    } catch (e) {
      throw new EdgeRefusal(e instanceof Error && e.message === EDGE_CHECK ? EDGE_CHECK : EDGE_CHECK_REPEATED);
    }
  }
  const again = await send();
  if (isEdgeChallenge(again)) throw new EdgeRefusal(EDGE_CHECK_REPEATED);
  return again;
}

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
 *  screen, which only exists in `open` mode, and the server ignores this
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
 *  header at all, so the `open` door could not sign a stream. */
export function identityHeaders(): Record<string, string> {
  const who = claimedUser();
  return who ? { "x-yokohama-claimed-user": who } : {};
}

/** The identity a WebSocket UPGRADE carries, as query parameters — `{}` where
 *  there is nothing to send (M6.5c2, spec §2.1).
 *
 *  **A browser puts no header on an upgrade** (`new WebSocket(url)`, `ws.ts`),
 *  so under the OPEN door the stored name rides the upgrade's URL as
 *  `claimed=`, and the Worker reads it there, on upgrades only. It is the value
 *  `identityHeaders` sends on every fetch — one stored name, one statement of
 *  it — and nothing under any other door: behind Access the edge's own session
 *  is the identity, and a name left from an open door would only be a stale
 *  one in a URL. `ws.ts` encodes it with `URLSearchParams`, so a `+` travels as
 *  `%2B` and never becomes a space (Review Focus 1). */
export function identityParams(): Record<string, string> {
  const who = claimedUser();
  return who && identityMode() === "open" ? { claimed: who } : {};
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
 *  the layer never sends that header at all. The two 401s mean different things
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

/** Whether this response is a provider-signed Worker saying nobody is signed in
 *  (the auth cycle, Task 6; design §4.3): `Access`'s sibling.
 *
 *  Under a sign-in arm (`YOKOHAMA_IDENTITY_SOURCE` `oidc` or `workos`) the
 *  Worker puts `WWW-Authenticate: Yokohama-Session realm="/enter"` on every 401
 *  under `/api` it answers (Task 4's `withApiHeaders`) — every one but the
 *  archive target's, whose only 401 since Task 5 is the house refusing a
 *  malformed body, a defect to surface (ruling R-36). The scheme the header
 *  OPENS with is the whole test, as Access's is. */
export function isSessionChallenge(res: Response): boolean {
  return res.headers.get("www-authenticate")?.startsWith("Yokohama-Session") ?? false;
}

/** The words of [`SIGN_IN_NEEDED`] with no period: the door card's HEADING,
 *  which titles a card as every other card title does (ruling R-46). Written
 *  once, here, and the sentence below is built from it, so there is no second
 *  spelling. */
export const SIGN_IN_TITLE = "Sign in to play multiplayer";

/** What the player reads when a provider-signed Worker refused: one sentence
 *  for the table link, the lobby and every refusal strip, and the strip is
 *  where the player reads it (the plan's wording, replacing the design's
 *  §4.3). [`Refusal`](../Refusal) matches on it to draw the sign-in link, and
 *  the door matches on it to draw its first-visit card. */
export const SIGN_IN_NEEDED = `${SIGN_IN_TITLE}.`;

/** The sentence a 401 carrying a door's own challenge is read as — Access's
 *  expired session, or the Worker's "sign in" — or `null` for a 401 that is the
 *  layer's own refusal in the body's words.
 *
 *  **The one reader all three fetch sites ask** (`call`, `recordText` and the
 *  stream's open in `sse.ts`), so a fourth door's challenge is added once and
 *  the three cannot drift. */
export function challengeSentence(res: Response): string | null {
  if (isAccessChallenge(res)) return ACCESS_CHALLENGE;
  if (isSessionChallenge(res)) return SIGN_IN_NEEDED;
  return null;
}

/** Whether this refusal is a door's own challenge — the EDGE's expired session
 *  or the Worker's "sign in" — rather than anything the layer said: the one
 *  refusal a player can act on.
 *
 *  **A predicate and not a string compare at each site**, because there are
 *  several readers of it: [`Refusal`](../Refusal) draws the link beside the
 *  sentence, `whoami`'s `failureOf` draws the door's way in, and `usePolled`
 *  ENDS a held stream on it rather than climbing the reconnect ladder about a
 *  door the player has to walk through themselves (final review I-2). Both the
 *  status and the sentence are asked, since the layer's own 401 is a different
 *  refusal with a different answer. */
export const isSessionExpired = (e: unknown): boolean =>
  e instanceof ApiError && e.status === 401 && (e.message === ACCESS_CHALLENGE || e.message === SIGN_IN_NEEDED);

/** Where the sign-in link beside [`SIGN_IN_NEEDED`] goes: `/enter`, with the
 *  screen the player is on as `return_to` (the auth cycle, design §4.3).
 *
 *  **A NAVIGATION, never a fetch**: `/enter` with no `code` starts the
 *  provider's sign-in, and its callback lands the player on `return_to`. The
 *  Worker takes `/#/…` there and nothing else (Task 4's `safeReturnTo`; any
 *  other value is the lobby), so the hash goes with a leading `/` —
 *  `#/t/<id>` is `/#/t/<id>` — and a page with no hash returns to the lobby.
 *  Read when the link is drawn, so it names the screen it is drawn on. */
export function signInHref(): string {
  const hash = typeof location === "undefined" ? "" : location.hash;
  const back = hash.length > 1 ? `/${hash}` : "/#/lobby";
  return `/enter?return_to=${encodeURIComponent(back)}`;
}

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
 *  deadline, and a second constant would be a second thing to keep in step.
 *  A change signal's wait for its transport is bounded by it too (M6.5c,
 *  ruling I-86; [`transportWait`]): that wait is on the same health read. */
export const BOUND_MS = 5_000;

/** `p`, or a rejection once `ms` has passed.
 *
 *  A race rather than a catch, for the reason [`BOUND_MS`] gives. **The timer
 *  is cleared the moment `p` settles** (M6.5c2): the signal's probe is bounded
 *  too, inside a hook whose cells count what is left armed, and a bound whose
 *  timer outlived its read would stand there as a rung nobody scheduled. */
export const bounded = <T,>(p: Promise<T>, ms: number): Promise<T> =>
  new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("timed out")), ms);
    p.then(
      (v) => {
        clearTimeout(timer);
        resolve(v);
      },
      (e: unknown) => {
        clearTimeout(timer);
        reject(e);
      },
    );
  });

/** Which change transport this DEPLOYMENT serves, read once from
 *  `GET /api/health` on entering the multiplayer channel (M6.5b2: no longer at
 *  boot — the door asks nothing) and then read by `usePolled` on every open.
 *
 *  **A deployment fact and never a preference** (spec R-3.1): which transport
 *  a host serves is its own code decision. The default is the stream, so a
 *  health that does not answer leaves the client reading the transport the
 *  server prefers — a wrong guess here costs a failed open and one trip round
 *  the ladder, not a screen that never updates.
 *
 *  **`"ws"` is the Worker's (M6.5c, R-6.5c.6; spec §2.7)**: the stream's own
 *  frames over a hibernatable WebSocket. The NAS goes on answering `"sse"`.
 *  **There is no third value** (M6.5c2 cycle D, R-D5): the held request M6.3
 *  built beside the stream is retired on every host, and no host reports
 *  `"held"`.
 *
 *  **Per SOURCE since M6.5b** (spec §2.3): the value lives in
 *  `local/facts.ts`, keyed by server or local, and this reads the SERVER's.
 *  Its union never holds the local `"none"` — the transport arm never asks
 *  this about a local game, because it reads `Source.host` instead. */
export const signalKind = (): "sse" | "ws" => serverFacts().signal;

export const setSignalKind = (k: "sse" | "ws"): void => {
  patchServerFacts({ signal: k });
};

/** The SERVER's health read in flight — with the moment it was ASKED, which
 *  [`transportWait`]'s bound is measured from — and whether the last one to
 *  settle failed (M6.5c's live tier, finding 1; rulings I-81 and I-86). */
let healthRead: { read: Promise<Health>; at: number } | null = null;
let healthReadFailed = false;

/** `GET /api/health`, with the server's read in flight kept where
 *  [`transportWait`] can find it. A local read is the module's, and says
 *  nothing about the server's transport. */
function readHealth(host?: Host, opts?: CallOptions): Promise<Health> {
  const read = call<Health>("GET", "/api/health", undefined, host, opts);
  if (host === "local") return read;
  const asked = { read, at: Date.now() };
  healthRead = asked;
  read
    .then(
      () => {
        healthReadFailed = false;
      },
      () => {
        healthReadFailed = true;
      },
    )
    .finally(() => {
      if (healthRead === asked) healthRead = null;
    });
  return read;
}

/** Forget the server's health read, as a fresh page load has asked nothing
 *  (the whole-branch review's L-8): a cell that leaves a read hanging would
 *  otherwise be waited on by the next cell's signal. `resetFactsForTest` cannot
 *  do this — `local/facts.ts` is imported by this module and does not import
 *  it back. */
export function resetHealthReadForTest(): void {
  healthRead = null;
  healthReadFailed = false;
}

/**
 * What a change signal waits for before it picks its transport (M6.5c Task 15,
 * the live tier's finding 1; rulings I-81 and I-86): `null` — pick now — or a
 * wait that ends when the health answer lands, or when that read has had
 * [`BOUND_MS`] since it was ASKED, whichever comes first. A read that FAILS
 * inside its bound rejects the wait.
 *
 * The live run's lobby asked `/api/stream` as a STREAM and the Worker answered
 * 426: the facts' default is the stream until `GET /api/health` has answered,
 * and the lobby mounts while the read App makes on entering the multiplayer
 * channel is still out. So:
 *
 * - **Known** — a health answer landed, on this page or a visit before it —
 *   is `null`: the transport is read at once, exactly as it always was.
 * - **Being asked** is that read, awaited for what is left of its bound —
 *   App's own number for the same read. **The bound is ruling I-86**: a read
 *   that HANGS never rejects, and an unbounded wait on one opened nothing at
 *   all where the default was right — the NAS serves the stream, and its
 *   stream opened at once before this wait existed. Past the bound the
 *   transport this page HAS is used, the remembered one or the default: on the
 *   NAS its old behaviour, delayed by the bound at most; on the Worker one
 *   refused stream (426) per rung while health hangs, the price the ruling
 *   accepted.
 * - **Asked, and past its bound** is never waited on again: a first attempt
 *   uses the transport it has at once, and a `retrying` caller asks a NEW read
 *   rather than re-await the old one.
 * - **Asked and FAILED** is the ladder's business, never a guess (I-81): a
 *   caller's first attempt fails without a request of its own — the entry read
 *   is App's, and one that just failed is not asked again at once — and a
 *   `retrying` caller asks again itself, under the same bound.
 * - **Nothing out and nothing failed** is `null` too: the transport this page
 *   has. A screen reaches it only after an answer that carried nothing this
 *   client could make facts of — never UNASKED: a hook's first open waits a
 *   zero-delay timer, which runs only once the commit that mounted it has run
 *   its effects, App's among them, and App asks on entering the multiplayer
 *   channel. (Not the ORDER of those effects: React runs App's after its
 *   children's, so it is the timer that guarantees it.) The cells reach it by
 *   setting the transport themselves.
 *
 * **Whatever answer the waited read gives is written down when it lands, even
 * after the wait has stopped waiting for it**: the handler below stays on the
 * read for its whole life. A server that answers late is still a server that
 * answered, and a read a retry asked that answered after its wait gave up
 * would otherwise leave nothing out and nothing known — so every later open
 * would take the default for ever.
 */
export function transportWait(retrying: boolean): Promise<void> | null {
  if (serverFactsTakenAt() !== null) return null;
  const asked = healthRead;
  // How long it has been out, floored at nothing: a clock stepped back reads
  // as a read just asked, and `left` below caps the wait at the bound
  // whatever the clock did.
  const young = asked !== null && Math.max(0, Date.now() - asked.at) < BOUND_MS;
  let waitOn: { read: Promise<Health>; at: number };
  if (asked !== null && young) {
    waitOn = asked;
  } else if (retrying && (asked !== null || healthReadFailed)) {
    const read = api.health();
    waitOn = healthRead?.read === read ? healthRead : { read, at: Date.now() };
  } else if (asked !== null) {
    return null;
  } else if (healthReadFailed) {
    return Promise.reject(new Error("the transport is not known: the health read failed"));
  } else {
    return null;
  }
  const left = Math.min(BOUND_MS, Math.max(0, waitOn.at + BOUND_MS - Date.now()));
  const { read } = waitOn;
  return new Promise<void>((resolve, reject) => {
    // Cleared when the read settles first, so a wait that is over holds no
    // timer. When the timer wins, the read's settling later still writes its
    // answer down; the `resolve` and `reject` it calls then do nothing.
    const timer = setTimeout(resolve, left);
    read.then(
      (h) => {
        clearTimeout(timer);
        try {
          rememberServerFacts(h);
        } catch {
          /* nothing this client can make facts of: the transport it has stands */
        }
        resolve();
      },
      (e: unknown) => {
        clearTimeout(timer);
        reject(e);
      },
    );
  });
}

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
 *  "not signed in" remedy is two steps under `open` and one under the other
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
  /** `call`'s own option, for the health read (cycle F): the outbox's
   *  background pass reads with `false`, so a challenge there shows no
   *  widget. */
  interactive?: boolean;
  /** Told when the WAIT ends with the read still out (Task 6 fix round 1):
   *  handed the read, which settles once its answer is written down — or with
   *  the read's own failure. The caller's wait is over; the read is not. */
  onLate?: (landed: Promise<ArchiveKind>) => void;
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
    //
    // **The bound is the CALLER's wait, and never the read's** (cycle F, Task 6
    // fix round 1; `transportWait`'s own rule): a read the edge challenged
    // goes on while a person passes the widget, which can take longer than
    // the bound — and the answer that check paid for is written down WHEN IT
    // LANDS, however late. A caller that wants to act on a late answer says so
    // (`onLate`); the outbox does, and sends its rows again under the
    // clearance rather than show the widget a second time.
    const read = api.health(undefined, opts?.interactive === undefined ? undefined : { interactive: opts.interactive });
    const landed = read.then((h) => {
      rememberServerFacts(h);
      return h.archive;
    });
    let settled = false;
    landed.then(
      () => {
        settled = true;
      },
      () => {
        settled = true;
      },
    );
    try {
      return await bounded(landed, BOUND_MS);
    } catch (e) {
      // The wait ended and the read did not: that read is still the caller's,
      // if it asked for it.
      if (!settled) opts?.onLate?.(landed);
      throw e;
    }
  } catch (e) {
    // **The EDGE's refusal is handed on, and it is the one failure that is**
    // (cycle F, spec §2.6; F-15): it is not "nothing answered" — the edge
    // answered, and asked for a person. The outbox marks its rows "waiting for
    // a human check" on it, which a fallback to the remembered fact would hide
    // behind a send the same edge refuses again.
    if (isEdgeRefusal(e)) throw e;
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
  const line = `APPLIED BUT NOT SAVED — ${reason ?? "the browser refused the write"} — ${BACKUP_POINTER}`;
  // **A reply with no `warnings` of its own is GIVEN the array** (M6.5c2 cycle
  // D, dossier row 56). The line used to be appended only where the reply
  // already carried one, so it was DROPPED here from a create or a keep the
  // store refused — a table, which has none. It is carried now, on every reply
  // with a body. **Carried is not shown**: the screens that read those two
  // replies (the create's `onCreated`, the deal's keep) draw no `warnings`
  // today. And a reply with no body — a delete's 204 — has nothing here to
  // carry it on.
  if (out && typeof out === "object") {
    const warned = out as { warnings?: unknown };
    warned.warnings = Array.isArray(warned.warnings) ? [...(warned.warnings as string[]), line] : [line];
  }
  return out;
}

async function call<T>(
  method: string,
  path: string,
  body?: unknown,
  host?: Host,
  /** How a challenge at the EDGE is met (cycle F) — see [`CallOptions`]. A
   *  LOCAL call never meets one: nothing stands in front of the module. */
  opts?: CallOptions,
  /** Headers this ONE call carries beside what every server call does — the
   *  archive post's device id (the auth cycle, Task 6), and nothing else today.
   *  A local call sends none: nothing reads them there. */
  extra?: Record<string, string>,
): Promise<T> {
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
    const out = settle<T>(r.status, "", r.text, null);
    return r.saved ? out : withUnsaved(out, r.reason);
  }
  const headers: Record<string, string> = { ...serverHeaders(), ...extra };
  if (body !== undefined) headers["content-type"] = "application/json";
  const sent = body === undefined ? undefined : JSON.stringify(body);
  // Through the edge's one predicate and its one retry (R-FP8): the request is
  // built once and sent again WHOLE if the widget had to be shown.
  const res = await pastTheEdge(() => fetch(path, { method, headers, body: sent }), opts);
  return settle<T>(res.status, res.statusText, await res.text(), challengeSentence(res));
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
  /** The sentence a door's own `WWW-Authenticate` reads as, or `null` — see
   *  [`challengeSentence`]. The 401s are different refusals and the caller is
   *  the only one holding the `Response` that tells them apart. */
  challenge: string | null,
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
  // **A DOOR's 401, which is not the layer's** (M6.5b2, spec §2.1; the auth
  // cycle, Task 6). Access's: the session in front of the server expired, and
  // the name this browser holds is fine. A provider-signed Worker's: nobody
  // holds a session cookie, and no stored name was ever what it reads. Either
  // way the sentence is the door's own — the screen draws the sign-in link
  // beside it — and `unauthorized` is deliberately NOT fired: dropping the
  // claimed user would sign the player out of a deployment that never asked
  // them to, and the form they would then be shown cannot mint the cookie
  // that is actually missing.
  if (status === 401 && challenge !== null) throw new ApiError(challenge, status, null);
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
  // **A site of its own for the edge's challenge** (cycle F, R-FP8): this read
  // builds its own request, so it goes through the same predicate and the same
  // one retry `call` does, rather than a second copy of either.
  const res = await pastTheEdge(() =>
    fetch(`/api/games/${id(game)}/record`, { headers: { ...serverHeaders() } }),
  );
  const text = await res.text();
  if (!res.ok) {
    // **`settle`'s refusal half, rather than a second copy of it** (M6.5b2).
    // This site restated the sentence rule, the 401 rule and — once there were
    // two kinds of 401 — would have restated the challenge read too. It always
    // throws for a non-2xx, which is why the return below is still reached only
    // on success.
    settle<never>(res.status, res.statusText, text, challengeSentence(res));
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
   *  which change transport to read. Asked on entering the multiplayer channel
   *  (`App`), wherever `ensureArchiveFact` must, and by a change signal's
   *  retry while the transport is not known ([`transportWait`]), which is also
   *  what the server's read in flight is kept for. */
  health: (host?: Host, opts?: CallOptions) => readHealth(host, opts),
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
  archive: (record: string, table: string, opts?: CallOptions) =>
    // **The device id rides this post and no other** (the auth cycle, Task 6;
    // design §4.4): with no one signed in the Worker files the game under
    // `anon-<device>@anon.invalid`, one prefix per browser; signed in, it is
    // noted on the archived object. No game route reads it, so no other call
    // carries it.
    call<ArchiveReply>("POST", "/api/archive", { record, table }, "server", opts, {
      "X-Yokohama-Device": deviceId(),
    }),
  /** Sign out (the auth cycle, Task 6; design §4.3): `{ leave }`, where the
   *  player goes to end the session that signed them in — the issuer's logout
   *  under a sign-in arm (the session cookie cleared by the same answer),
   *  Access's own `/cdn-cgi/access/logout` under Access. A POST, so it carries
   *  every write's cross-site rules. **Explicitly the server**: there is no
   *  local session. The open door has no server session to end, and answers
   *  the layer's 404 — a refusal, which the caller's failure path reads. */
  leave: () => call<LeaveReply>("POST", "/api/leave", undefined, "server"),
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
   *  **The reply is immediate on every host** (M6.5c2 cycle D, R-D5): the
   *  held request, and the `wait` this took for it, are retired. `after` is a
   *  resume cursor every host parses and ignores; nothing here sends it today.
   *  An older release's `wait` is parsed and ignored the same way, which is
   *  what keeps its probe from meeting a 400. */
  events: (game: string, after?: number) =>
    call<Events>(
      "GET",
      `/api/games/${id(game)}/events${query({ after })}`,
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

  /** Take message `seq` at table `game` down (M6.5c2 cycle D, R-D4): an empty
   *  204 for its AUTHOR or the table's CREATOR — and again for a second delete,
   *  which changes nothing — and a 403 for anyone else. The message stays as a
   *  tombstone (`text: ""`, `deleted: true`), so its number is never reused;
   *  the table's `Events.chat_rev` moves and `Events.chat` does not. */
  deleteChat: (game: string, seq: number) =>
    call<null>("DELETE", `/api/games/${id(game)}/chat/${seq}`),

  /** Take the reader's own house message `seq` down (M6.5c2 cycle D, R-D4):
   *  the house has no creator, so its author alone may. */
  deleteHouseChat: (seq: number) => call<null>("DELETE", `/api/chat/${seq}`),
};
