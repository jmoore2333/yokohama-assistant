import type {
  AreasPanel,
  ArtManifest,
  ClientState,
  Labels,
  LabsPanel,
  HistoricalFrame,
  LogPanel,
  PortsPanel,
  Preview,
  IndexModel,
  Outcome,
  SuggestResponse,
  WhyResponse,
} from "./types";

/** A refusal from the server, carrying the sentence it refused with. */
export class ApiError extends Error {}

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method,
    headers: body === undefined ? {} : { "content-type": "application/json" },
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
    throw new ApiError(message);
  }
  if (parseFailed) {
    // A 2xx that is not JSON is this API answering with something it does not
    // serve — an interstitial, a proxy page. Say that, rather than a parser's
    // complaint about the first character of it.
    throw new ApiError(
      `${res.status} ${res.statusText}: the server answered with something that is not JSON`,
    );
  }
  return parsed as T;
}

export interface NewGameRequest {
  players: number;
  colors: string[];
  start_seat: number;
  /** A decimal STRING: a `u64` seed does not survive a JS number, and the seed
   *  round-trips through this client on the way from preview to create. */
  seed?: string;
  keeps: number[];
  bot_seats: number[];
  name?: string;
}

const id = (s: string) => encodeURIComponent(s);

export const api = {
  labels: () => call<Labels>("GET", "/api/labels"),
  /** 404s when the server has no cropped art. That is a supported state, not an
   *  error: callers treat the rejection as "no art" and draw without it. */
  art: () => call<ArtManifest>("GET", "/api/art"),
  list: () => call<string[]>("GET", "/api/games"),
  preview: (req: NewGameRequest) => call<Preview>("POST", "/api/games/preview", req),
  create: (req: NewGameRequest) => call<ClientState>("POST", "/api/games", req),
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
  log: (game: string) => call<LogPanel>("GET", `/api/games/${id(game)}/log`),

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
};
