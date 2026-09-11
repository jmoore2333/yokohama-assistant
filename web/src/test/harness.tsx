// What a component test needs that is not a component: a server.
//
// **Nothing here fabricates a payload.** Every state a test renders comes from
// `src/wire/`, which the server emitted and `wire_fixtures.rs` refuses to let
// drift. A `makeState({...})` builder would be a third statement of the wire
// shape and would keep compiling — and keep passing — after the server dropped
// a field, which is the exact failure this milestone exists to close.
//
// What IS built here is the transport: a `fetch` the test controls, so a test
// can say what the server answers, WHEN it answers, and can then assert on what
// the client asked for. The timing half is not a nicety — one of the two
// defects M2 found by clicking was a second request going out while the first
// was in flight, and a stub that resolves immediately cannot see it.

import { render } from "@testing-library/react";
import { vi } from "vitest";
import { App } from "../App";
import { wire } from "../wire/contract";
import type { ClientState } from "../types";

/** One request the client made. */
export interface ApiCall {
  method: string;
  path: string;
  /** The parsed JSON body, or `undefined` for a GET. */
  body: unknown;
}

/** What a handler may answer with: a body, or a status and a body. */
export type Reply = unknown | { status: number; body: unknown };

export interface ApiStub {
  /** Every request, in order. */
  calls: ApiCall[];
  /** Just the ones matching `method` and a path substring. */
  matching(method: string, contains: string): ApiCall[];
}

const isStatused = (r: Reply): r is { status: number; body: unknown } =>
  typeof r === "object" && r !== null && "status" in r && "body" in r;

/**
 * Install a `fetch` that answers from `handler`.
 *
 * The handler is called with every request and may return a body, a
 * `{status, body}` pair, or a PROMISE of either — a pending promise is how a
 * test holds a request in flight while it clicks again.
 *
 * Unhandled paths throw rather than defaulting: a component that fetched
 * something the test did not think about should say so, in the test that caused
 * it, and not quietly render against `{}`.
 */
export function stubApi(handler: (call: ApiCall) => Reply | Promise<Reply>): ApiStub {
  const calls: ApiCall[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const call: ApiCall = {
        method: init?.method ?? "GET",
        path: String(input),
        body: typeof init?.body === "string" ? JSON.parse(init.body) : undefined,
      };
      calls.push(call);
      const reply = await handler(call);
      const status = isStatused(reply) ? reply.status : 200;
      const body = isStatused(reply) ? reply.body : reply;
      return new Response(body === undefined ? "" : JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
      });
    }),
  );
  return {
    calls,
    matching: (method, contains) =>
      calls.filter((c) => c.method === method && c.path.includes(contains)),
  };
}

/** A promise a test resolves by hand, to hold a request in flight. */
export function gate<T>(): { promise: Promise<T>; resolve: (v: T) => void } {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

/** The payloads `App` fetches to DRAW A SCREEN, answered from the corpus.
 *
 *  A FALLBACK, consulted only after the caller's own handler declines — see
 *  [`renderApp`]. It used to answer first, which silently stole `/api/art` from
 *  any test that wanted to see the 404 branch.
 *
 *  **The three board panels joined the two bootstrap routes at W5b**, because
 *  the sections moved onto the table: `TableSections` fetches `/ports` and
 *  `/labs` for the DEFAULT enabled set and `/areas` when Building sites is on,
 *  so every `renderApp` test now asks for them whether or not it is about them.
 *  Without this the fallback threw `unexpected GET /api/games/wire/labs`, the
 *  component carried the sentence into an `.error` div, and three cells that
 *  match on the game id — which the sentence contains — failed with "found
 *  multiple elements". A test that wants to see one of these refuse still
 *  supplies its own reply; the caller is asked first. */
function bootstrapReply(call: ApiCall): Reply | undefined {
  if (call.path.endsWith("/api/labels")) return wire.labels;
  if (call.path.endsWith("/api/art")) return wire.artManifest;
  if (call.path.endsWith("/ports")) return wire.ports;
  if (call.path.endsWith("/labs")) return wire.labs;
  if (call.path.endsWith("/areas")) return wire.areas;
  // The walk's two reads (M5.5): a position and the log's tail. Answered from
  // the corpus so a cell that is not about the walk does not have to be — a
  // corpus reply marks no seat as a bot, so no walk starts from it.
  if (/\/at\/\d+$/.test(call.path)) return wire.atK;
  if (/\/log(\?|$)/.test(call.path)) return wire.log;
  return undefined;
}

/**
 * Render the whole `App` already inside a game.
 *
 * `App` has no prop for an in-progress game — it starts on the setup screen —
 * so this goes in the way a player does: the saved-game list, one click. That
 * keeps the test honest about the wiring (`onStarted` really is what hands the
 * state over) and costs one interaction.
 */
export async function renderApp(
  state: ClientState,
  handler: (call: ApiCall) => Reply | undefined | Promise<Reply | undefined> = () => undefined,
): Promise<ApiStub> {
  // **The caller's handler is asked FIRST, and `undefined` means "not mine".**
  // The first draft answered the bootstrap routes before consulting it, so a
  // test that wanted to see `/api/art` refuse got the corpus manifest instead
  // and its 404 branch was never entered — the test passed while asserting
  // nothing about the case it named.
  const stub = stubApi(async (call) => {
    const mine = await handler(call);
    if (mine !== undefined) return mine;
    const boot = bootstrapReply(call);
    if (boot !== undefined) return boot;
    if (call.method === "GET" && call.path.endsWith("/api/games")) return [state.id];
    if (call.method === "GET" && call.path.endsWith(`/api/games/${state.id}`)) return state;
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });

  const view = render(<App />);
  const open = await view.findByRole("button", { name: state.id });
  const { default: userEvent } = await import("@testing-library/user-event");
  await userEvent.setup().click(open);
  // The banner only exists once a game is loaded, so finding it is the wait.
  // The end-trigger strip rather than the seed: the seed is no longer served on
  // a play payload (it regenerates the whole hidden set), and the strip is the
  // other thing that exists only once a game is on screen. Unambiguous, unlike
  // the id, which is also the name of the button just clicked.
  await view.findByText(/end triggers:/);
  return stub;
}
