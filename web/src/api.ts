import type {
  AreasPanel,
  ArtManifest,
  ClientState,
  DealView,
  Events,
  GamesList,
  Labels,
  LabsPanel,
  HistoricalFrame,
  LogPanel,
  Me,
  PortsPanel,
  Preview,
  IndexModel,
  Outcome,
  SuggestResponse,
  TableView,
  WhyResponse,
} from "./types";

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
  constructor(message: string, status: number) {
    super(message);
    this.status = status;
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

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
  const headers: Record<string, string> = {};
  if (body !== undefined) headers["content-type"] = "application/json";
  const who = claimedUser();
  if (who) headers["x-yokohama-claimed-user"] = who;
  const res = await fetch(path, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();

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

  if (!res.ok) {
    const message =
      !parseFailed && parsed && typeof parsed === "object" && "error" in parsed
        ? String((parsed as { error: unknown }).error)
        : `${res.status} ${res.statusText}`;
    // **Before the throw, so it reaches the door whatever the caller does with
    // the refusal** — including discarding it, which one caller does.
    if (res.status === 401) unauthorized?.(message);
    throw new ApiError(message, res.status);
  }
  if (parseFailed) {
    // A 2xx that is not JSON is this API answering with something it does not
    // serve — an interstitial, a proxy page. Say that, rather than a parser's
    // complaint about the first character of it.
    throw new ApiError(
      `${res.status} ${res.statusText}: the server answered with something that is not JSON`,
      res.status,
    );
  }
  return parsed as T;
}

/** One seat of a table being opened: a person's email, or the bot. Exactly one
 *  of the two — a seat naming both is a 400 that says so. */
export type SeatRequest = { user: string } | { bot: true };

export interface NewGameRequest {
  players: number;
  colors: string[];
  start_seat: number;
  /** A decimal STRING: a `u64` seed does not survive a JS number, and the seed
   *  round-trips through this client on the way from preview to create. */
  seed?: string;
  /** One entry per seat, in seat order. **The creator must hold one of them**
   *  (R-1.6), and a bot is refused at three and four players (R-P10). */
  seats: SeatRequest[];
  name?: string;
}

const id = (s: string) => encodeURIComponent(s);

export const api = {
  /** Who the server thinks we are, and which header it reads. Never refuses:
   *  `user: null` is the answer, not an error. */
  me: () => call<Me>("GET", "/api/me"),
  labels: () => call<Labels>("GET", "/api/labels"),
  /** 404s when the server has no cropped art. That is a supported state, not an
   *  error: callers treat the rejection as "no art" and draw without it. */
  art: () => call<ArtManifest>("GET", "/api/art"),
  /** The tables you sit at, newest first — never anyone else's. */
  list: () => call<GamesList>("GET", "/api/games"),
  preview: (req: NewGameRequest) => call<Preview>("POST", "/api/games/preview", req),
  /** Opens a table, and starts NOTHING: the reply is `status: "dealing"` and
   *  every seat is owed a deal it fetches for itself. */
  create: (req: NewGameRequest) => call<TableView>("POST", "/api/games", req),
  /** Who sits where, and how far the dealing has got. */
  table: (game: string) => call<TableView>("GET", `/api/games/${id(game)}/table`),
  /** The caller's OWN two cards, over the set table. `seat` and `cards` are
   *  null together once every seat this caller holds has chosen. */
  deal: (game: string) => call<DealView>("GET", `/api/games/${id(game)}/deal`),
  /** Choose, for one seat the caller holds. The LAST keep at the table starts
   *  the game, and the reply then reads `status: "playing"`. */
  keep: (game: string, seat: number, card: 1 | 2) =>
    call<TableView>("POST", `/api/games/${id(game)}/keep`, { seat, card }),
  get: (game: string) => call<ClientState>("GET", `/api/games/${id(game)}`),
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
   *  The frame is built for ONE pinned seat that does not follow the mover, so
   *  walking every `k` shows one seat's hands and no other's. */
  at: (game: string, k: number) =>
    call<HistoricalFrame>("GET", `/api/games/${id(game)}/at/${k}`),

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
  events: (game: string, after?: number) =>
    call<Events>(
      "GET",
      `/api/games/${id(game)}/events${after === undefined ? "" : `?after=${after}`}`,
    ),
};
