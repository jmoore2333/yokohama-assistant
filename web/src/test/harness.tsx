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

import { act, render, screen, waitFor } from "@testing-library/react";
import { expect, vi } from "vitest";
import { App } from "../App";
import { CLAIMED_USER_KEY } from "../api";
import { wire } from "../wire/contract";
import { currentWorker, stubWorker } from "./fakeWorker";
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
export const isGamesList = (call: ApiCall, scope?: "mine" | "open" | "playing"): boolean => {
  const [path, query] = call.path.split("?");
  if (call.method !== "GET" || !path!.endsWith("/api/games")) return false;
  return scope === undefined || new URLSearchParams(query ?? "").get("scope") === scope;
};

/** What the server answers a request it cannot name a caller for.
 *
 *  The SHAPE is the corpus's refusal; the sentence is `identity.rs`'s own, for
 *  the `open` door. **The sentence is not what the client routes on** — the
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
  /** Every header sent, keyed in lower case.
   *
   *  **Here since M6.3, because one header is now load-bearing at a URL this
   *  module does not build**: the change stream is opened by `sse.ts`'s own
   *  `fetch`, and the cell that says it is signed has nothing else to read. */
  headers: Record<string, string>;
  /** The reader's abort, where the caller passed one. `fakeStream` listens to
   *  it, which is what makes "the hidden tab stops reading" an observable fact
   *  rather than an unobserved intention. */
  signal?: AbortSignal;
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
      const headers: Record<string, string> = {};
      new Headers(init?.headers).forEach((v, k) => {
        headers[k] = v;
      });
      const call: ApiCall = {
        method: init?.method ?? "GET",
        path: String(input),
        body: typeof init?.body === "string" ? JSON.parse(init.body) : undefined,
        headers,
        signal: init?.signal ?? undefined,
      };
      calls.push(call);
      const reply = await handler(call);
      // **A `Response` is passed through untouched**, which is how a handler
      // answers with something that is not one JSON object: the change stream
      // is a body that arrives in pieces and stays open, and `JSON.stringify`
      // of it would be a string.
      if (reply instanceof Response) return reply;
      // **A stream nobody scripted stays OPEN and says nothing**, which is what
      // a reachable server with nothing to report does. Without this rule every
      // cell in this suite that is not about the signal would have its reader
      // read an empty body to the end, call that a dropped connection, and put
      // "reconnecting…" on a screen the cell is asserting about. Scoped to the
      // stream's own `Accept`, and only where the handler DECLINED — a cell
      // that wants to see an open refused answers it itself.
      if (reply === undefined && headers.accept === "text/event-stream") return openStream();
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

/** The REAL `setTimeout`, captured before any cell fakes the clock — modules
 *  are evaluated before the first `beforeEach`, so this is the platform's. */
const realSetTimeout = globalThis.setTimeout;

/**
 * Let a reply chain settle without waiting out an interval.
 *
 * **Two yields per round, and each one is here because the other cannot do its
 * job.** The real `setTimeout(0)` gives the event loop a GENUINE turn, which no
 * faked clock can: reading a stream body goes through the platform's own
 * machinery and not every step of it is a microtask, so a settle built from
 * fake-timer advances alone failed about one run in three on a loaded machine.
 * The fake advance fires the hook's own zero-delay open, which the real yield
 * never reaches because that timer is faked.
 *
 * **And the fake advance is ONE MILLISECOND, not zero, which is the whole bug
 * this helper was rewritten for.** A `setTimeout(fn, 0)` armed DURING a
 * zero-length tick is scheduled at exactly the clock's current instant, and
 * `advanceTimersByTimeAsync(0)` does not reach it however many times it is
 * called — the clock never moves, so the deadline is never passed. Measured
 * directly: at a failure with one timer pending, eighty advances of 0 left it
 * pending and a single advance of 1 fired it at once. The screen under test
 * simply never opened its stream, and the cell failed as a screen that had not
 * moved.
 *
 * A millisecond a round is not waiting anything out: the shortest real delay in
 * this client is the ladder's two seconds, so a cell asserting that nothing was
 * scheduled still means it.
 */
export const flush = async (rounds = 12): Promise<void> => {
  for (let i = 0; i < rounds; i += 1) {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
      await new Promise((r) => realSetTimeout(r, 0));
    });
  }
};

/**
 * Turn the loop until `ok()` is true, and THROW, naming `what`, if it never is.
 *
 * **The instrument a round count was standing in for.** A frame written to a
 * body that has not been opened yet reaches nobody — `fakeStream.push` has no
 * controller to write to — and no later settle re-delivers it, so a cell that
 * connected with a fixed number of rounds pushed into the void whenever the
 * machine was loaded enough to need one more. It failed as a screen that had
 * simply not moved, which is the least informative shape a failure can take.
 * Gate on the fact instead (`stream.opens() > 0`), and a genuine hang is named
 * rather than dumped as a DOM tree.
 */
export const until = async (
  ok: () => boolean,
  what = "the condition",
  rounds = 80,
): Promise<void> => {
  for (let i = 0; i < rounds; i += 1) {
    if (ok()) return;
    await flush(1);
  }
  if (!ok()) throw new Error(`${what} never became true after ${rounds} turns`);
};

/** The bytes of one `text/event-stream` frame, in this server's own shape:
 *  an `event:` line, a one-line `data:` line, and the blank line that ends it. */
const frameBytes = (event: string, data: unknown): Uint8Array =>
  new TextEncoder().encode(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);

const SSE_HEADERS = { "content-type": "text/event-stream" };

/** A stream that opens and then says nothing, for ever. The default answer to
 *  a `/stream` request in a cell that is not about the signal — see `stubApi`. */
export const openStream = (): Response =>
  new Response(
    new ReadableStream<Uint8Array>({
      start() {
        /* nothing to say, and nothing to close: a connected server at rest */
      },
    }),
    { status: 200, headers: SSE_HEADERS },
  );

/** A change stream a cell drives by hand.
 *
 *  **A fresh body per OPEN, because the hook reopens.** A `Response` body can
 *  be read once, and `usePolled` opens a new reader after every hidden tab and
 *  every trip round the ladder — a helper that handed the same body back twice
 *  would have the second read fail for a reason that is the helper's.
 *
 *  `onOpen` is the server's ON-CONNECT frame (spec §3.1), which is the whole of
 *  the R-P8 ask: it is given the open's 1-based number so a cell can answer the
 *  reconnect differently from the first connection.
 *
 *  The ABORT is honoured, and that is not decoration: without it a reader torn
 *  down by a hidden tab goes on holding the body, and the next `push` would be
 *  delivered twice — once to the live reader and once to a dead one — which is
 *  a double `onValue` no browser would produce. */
export function fakeStream(onOpen?: (open: number) => Array<[string, unknown]>) {
  const live: Array<ReadableStreamDefaultController<Uint8Array>> = [];
  let opens = 0;
  const drop = (c: ReadableStreamDefaultController<Uint8Array>) => {
    const i = live.indexOf(c);
    if (i >= 0) live.splice(i, 1);
  };
  return {
    /** How many times the stream has been opened. */
    opens: () => opens,
    /** The handler's reply. Pass the call so the abort can be heard. */
    reply(call?: ApiCall): Response {
      opens += 1;
      const n = opens;
      const body = new ReadableStream<Uint8Array>({
        start(c) {
          live.push(c);
          for (const [event, data] of onOpen?.(n) ?? []) c.enqueue(frameBytes(event, data));
          call?.signal?.addEventListener(
            "abort",
            () => {
              drop(c);
              try {
                c.error(new DOMException("The operation was aborted.", "AbortError"));
              } catch {
                /* already closed */
              }
            },
            { once: true },
          );
        },
      });
      return new Response(body, { status: 200, headers: SSE_HEADERS });
    },
    /** One frame, to every reader currently holding the body. */
    push(event: string, data: unknown) {
      for (const c of [...live]) {
        try {
          c.enqueue(frameBytes(event, data));
        } catch {
          drop(c);
        }
      }
    },
    /** The body ENDS, with no `end` frame — a dropped connection, which is what
     *  the ladder is for. */
    close() {
      for (const c of [...live]) {
        try {
          c.close();
        } catch {
          /* already closed */
        }
        drop(c);
      }
    },
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
 *  [`renderApp`]. It used to answer first, which silently stole `/art/manifest.json` from
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
  // **Boot reads the transport it should use** (M6.3 §5.1). The corpus's own
  // health says `sse`, which is the default and therefore the arm every cell in
  // this suite runs.
  if (call.path.endsWith("/api/health")) return wire.health;
  // Every screen with a signal opens one. A cell about the signal scripts its
  // own; this is the resting answer for the rest.
  if (call.path.endsWith("/stream")) return openStream();
  // **The art manifest is a static FILE since M6.5b2** (spec §2.2), not
  // `/api/art`: the door costs the Worker nothing, and a static asset is free.
  if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
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
  // **Every screen with a side column draws a chat panel since M6.4 (§6.2)**,
  // and the panel asks for the table's messages on mount whatever its fold
  // state — so every `renderApp` cell now makes this request whether or not it
  // is about the chat, exactly as the three board panels joined this list at
  // W5b. A table where nothing has been SAID, derived from the corpus's own
  // page rather than typed out here (the `gamesNamed` rule), so a cell that is
  // not about the chat has none of another room's messages in its DOM. A cell
  // that IS about it answers the route itself — the caller is asked first.
  if (call.method === "GET" && /\/chat(\?|$)/.test(call.path))
    return { ...wire.chat, messages: [], last: 0 };
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
  // test that wanted to see `/art/manifest.json` refuse got the corpus manifest instead
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
    // **And the third scope answers EMPTY, for the reason the open one does.**
    // The lobby asks all three since M6.3; a stub answering this game to
    // `playing` too would list it THREE times and the click below would fail on
    // an ambiguity that has nothing to do with the cell.
    if (isGamesList(call, "playing")) return gamesNamed();
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

  // **At `#/lobby`, because `#/` is the front door since M6.5b2** (R-6.5b2.1).
  // The door lists the games in THIS BROWSER and offers a link to the turnstile;
  // the server's tables are the lobby's, which is where the row below is. The
  // suite's own `beforeEach` leaves the hash empty, which now reads as the door.
  window.location.hash = "#/lobby";
  const view = render(<App />);
  // **Through the LOBBY, the server's own first screen (spec §5.1).** The row
  // is the same row the saved list used to draw, in the section that is now its
  // home; the id is still what names it, because `gamesNamed` clears the corpus
  // row's own name.
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

/** The positions a `GET …/at?from=&to=` call asks for — `[from, to)` — or
 *  `null` for any other path. Since M6.5b (R-6.5b.10) the walk fetches its run
 *  in ONE call, so a handler answers the whole range from this. */
export function runRange(path: string): number[] | null {
  const m = /\/at\?from=(\d+)&to=(\d+)$/.exec(path);
  if (!m) return null;
  const from = Number(m[1]);
  const to = Number(m[2]);
  return Array.from({ length: Math.max(0, to - from) }, (_, i) => from + i);
}

/** Every position the client has fetched through the run route, in order —
 *  what the list of `/at/{k}` requests used to be, read off the run calls. */
export const positionsWalked = (stub: ApiStub): number[] =>
  stub.matching("GET", "/at?").flatMap((c) => runRange(c.path) ?? []);

/**
 * The local host answering ONE playable game from the corpus (M6.5b2).
 *
 * **Call it BEFORE `render`**, like `stubWorker` itself: the door lists the
 * games in this browser on its first paint, and a Worker installed afterwards
 * is one the page has already given up asking.
 *
 * The state is `optionsMany` re-addressed to a local id — the corpus's own
 * payload, the `fakeWorker` rule — because it is the fixture with a row a
 * click can land on. Every input is answered with the SAME state, which is all
 * a cell about the wiring needs: what is being read is which calls went out,
 * not what the engine did with them.
 */
export function stubLocalGame(id = "local-1"): void {
  const state = { ...wire.optionsMany, id };
  stubWorker({
    call: (req) => {
      if (req.method === "GET" && req.path === `/api/games/${id}`) return { status: 200, body: state };
      if (req.path === `/api/games/${id}/input`) return { status: 200, body: state };
      return undefined;
    },
  });
}

/**
 * Open the local game on the door and commit ONE move, the way a player does.
 *
 * A row, a click, an option, and the `/input` the click sends — asserted to
 * have gone out, so a cell that then counts something downstream is counting
 * from a move that really happened rather than from a click that landed on
 * nothing. Pairs with [`stubLocalGame`].
 */
export async function playOneLocalMove(id = "local-1"): Promise<void> {
  const { default: userEvent } = await import("@testing-library/user-event");
  const user = userEvent.setup();
  await user.click(await screen.findByRole("link", { name: id }));
  // The banner strip: the one thing that exists only once a game is on screen.
  await screen.findByText(/end triggers:/);
  const prompt = wire.optionsMany.prompt;
  if (prompt.kind !== "options") throw new Error("the corpus's optionsMany fixture moved");
  const words = prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  await user.click(screen.getByRole("button", { name: new RegExp(words) }));
  await waitFor(() =>
    expect(
      currentWorker().posted.filter((p) => p.kind === "call" && p.path.endsWith("/input")),
    ).toHaveLength(1),
  );
}
