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
import { CLAIMED_USER_KEY } from "../api";
import { wire } from "../wire/contract";
import type { ClientState, GamesList } from "../types";

/** What `GET /api/games` answers with, naming these games.
 *
 *  **Derived from the corpus's own row, never hand-authored** — the rule this
 *  whole file follows. `games.json` carries one real `TableView`; only its `id`
 *  moves, which is the same pattern the game-switch cells already use to make a
 *  second game out of a corpus payload. A builder that typed a `TableView` out
 *  by hand would keep compiling after the server dropped a field from it. */
export const gamesNamed = (...ids: string[]): GamesList => ({
  // **`name` is cleared as well as `id` being set, and that is what "naming"
  // means since M6.1**: a row is listed as `name ?? id`, so a table that kept
  // the corpus row's own name would be listed as `wire` however this helper was
  // called, and every cell that finds its game by the id it asked for would
  // find nothing. Clearing it is the `?? id` fallback — a table opened without
  // a name — which is the shape these cells have always assumed.
  games: ids.map((id) => ({ ...wire.games.games[0]!, id, name: null })),
  // One page, so there is nothing after it. A cursor here would name a row this
  // helper never built.
  next: null,
});

/** Is this call the LOBBY LISTING, and — when `scope` is given — that scope's?
 *
 *  **One matcher, because there are now two listings behind one path.** Every
 *  stub in this suite matched `GET /api/games` by its path alone, and M6.2's
 *  `?scope=open` walks straight through such a check: a cell stubbing the open
 *  list would silently be handed the `mine` reply and pass while asserting the
 *  wrong thing. A 28th restatement of the path test is how that happens twice. */
export const isGamesList = (call: ApiCall, scope?: "mine" | "open"): boolean => {
  const [path, query] = call.path.split("?");
  if (call.method !== "GET" || !path!.endsWith("/api/games")) return false;
  return scope === undefined || new URLSearchParams(query ?? "").get("scope") === scope;
};

/** What the server answers a request it cannot name a caller for.
 *
 *  The SHAPE is the corpus's refusal; the sentence is `identity.rs`'s own, for
 *  the `claimed` door. **The sentence is not what the client routes on** — the
 *  status is, because `identity.rs` is the only place on this API that answers
 *  401 at all, while the sentence is one of three (the other two doors say "no
 *  identity reached the server…" instead). Here so that the cells about the
 *  three catch sites all refuse the same way. */
export const signedOutReply = () => ({
  status: 401,
  body: { ...wire.error, error: "no signed-in user: send X-Yokohama-Claimed-User" },
});

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

/** The Game menu's summary: the banner control that opens the saved-game list.
 *
 *  **Queried by its element, and no longer by the game id.** Until M6.1 the id
 *  appeared exactly once on screen, so `getByText(/wire/)` found the menu. The
 *  "You are …" strip now carries the signed-in email beside it, and the corpus's
 *  user is `red@wire.test` while its game is `wire` — so that regex matches two
 *  elements and every cell using it fails on an ambiguity that has nothing to
 *  do with what it is testing. */
export const gameMenu = (): HTMLElement =>
  document.querySelector<HTMLElement>(".game-menu > summary")!;

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
  // **The door, answered the way the SERVER answers it: from the header this
  // browser is sending.** `App` draws nothing at all until `/api/me` replies,
  // so without this every cell here would render a blank page.
  //
  // Conditioned on storage rather than pinned to `wire.me`, because that is
  // what `/api/me` IS — it reads the same claimed-user header `api.ts` attaches
  // from storage. A fallback that answered `wire.me` regardless would model a
  // server ignoring its own header, and the 401 cell could never see sign-in:
  // the client drops the stored name and asks again, and a stub with no memory
  // would hand the user straight back.
  if (call.path.endsWith("/api/me"))
    return localStorage.getItem(CLAIMED_USER_KEY) ? wire.me : wire.meSignedOut;
  if (call.path.endsWith("/api/labels")) return wire.labels;
  if (call.path.endsWith("/api/art")) return wire.artManifest;
  // **`GET …/table` is now on the way into every game (M6.1 §7.4)**: `App`'s
  // `open` asks what a table IS before it fetches the game, because a table
  // still dealing refuses the game route with a 409. So every cell that opens a
  // game passes through here, whether or not it is about tables.
  //
  // The ID is taken from the PATH rather than left at the corpus's own, which is
  // the documented fixture-override shape — the smallest move that makes the
  // reply about the table the caller asked for. The status is the corpus's
  // (`playing`); a cell that wants the dealing screen answers `/table` itself.
  const table = /\/api\/games\/([^/]+)\/table$/.exec(call.path);
  if (table) return { ...wire.table, id: decodeURIComponent(table[1]!) };
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
    // **The two scopes answer differently, and they have to.** The lobby asks
    // for both; a stub answering one reply to both would list this game TWICE —
    // once under Your games and once under Open tables — and the click below
    // would fail on an ambiguity that has nothing to do with the cell.
    if (isGamesList(call, "open")) return gamesNamed();
    if (isGamesList(call)) return gamesNamed(state.id);
    if (call.method === "GET" && call.path.endsWith(`/api/games/${state.id}`)) return state;
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });

  // **Signed in before the app mounts**, because since M6.1 there is a door in
  // front of every screen: with nobody stored, `App` draws the sign-in form and
  // there is no saved-game list to click. The value is the corpus's own user —
  // the one `wire.me` names, and the one the fixtures' tables are seated by —
  // so the identity this browser claims and the identity the payloads were
  // built for are one string.
  localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);

  const view = render(<App />);
  // **Through the LOBBY now, which is the first screen after the door (spec
  // §5.1).** The row is the same row the saved list used to draw, in the
  // section that is now its home; the id is still what names it, because
  // `gamesNamed` clears the corpus row's own name.
  //
  // **A LINK since M6.2 (R-14)**: the row's job is to go to `#/t/<id>`, so it
  // carries that address in an `href` and the browser navigates. A `button`
  // query here would find the row's Abandon control or nothing at all.
  const open = await view.findByRole("link", { name: state.id });
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
