import type { ClientState, Labels, Preview } from "./types";

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
  seed?: number;
  keeps: number[];
  bot_seats: number[];
  name?: string;
}

export const api = {
  labels: () => call<Labels>("GET", "/api/labels"),
  list: () => call<string[]>("GET", "/api/games"),
  preview: (req: NewGameRequest) => call<Preview>("POST", "/api/games/preview", req),
  create: (req: NewGameRequest) => call<ClientState>("POST", "/api/games", req),
  get: (id: string) => call<ClientState>("GET", `/api/games/${encodeURIComponent(id)}`),
  /** The one mutation: an option's `number`, or a sub-prompt token. */
  input: (id: string, line: string) =>
    call<ClientState>("POST", `/api/games/${encodeURIComponent(id)}/input`, { line }),
};
