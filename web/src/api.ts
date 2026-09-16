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
    throw new ApiError(message, res.status, parseFailed ? null : parsed);
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

export const api = {
  /** Who the server thinks we are, and which header it reads. Never refuses:
   *  `user: null` is the answer, not an error. */
  me: () => call<Me>("GET", "/api/me"),
  labels: () => call<Labels>("GET", "/api/labels"),
  /** 404s when the server has no cropped art. That is a supported state, not an
   *  error: callers treat the rejection as "no art" and draw without it. */
  art: () => call<ArtManifest>("GET", "/api/art"),
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
  list: (scope: "mine" | "open" | "playing" = "mine", before?: string) =>
    call<GamesList>(
      "GET",
      `/api/games?scope=${scope}${before ? `&before=${encodeURIComponent(before)}` : ""}`,
    ),
  /** Opens a table, and starts NOTHING: the reply is `status: "dealing"` and
   *  every seat is owed a deal it fetches for itself. */
  create: (req: NewGameRequest) => call<TableView>("POST", "/api/games", req),
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
