import type {
  AreasPanel,
  ArtManifest,
  ClientState,
  Labels,
  LabsPanel,
  LogPanel,
  PortsPanel,
  Preview,
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
  const parsed: unknown = text ? JSON.parse(text) : null;
  if (!res.ok) {
    const message =
      parsed && typeof parsed === "object" && "error" in parsed
        ? String((parsed as { error: unknown }).error)
        : `${res.status} ${res.statusText}`;
    throw new ApiError(message);
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
};
