// Which screen this is, as the URL (spec §5.1).
//
// **The hash, and no router.** A handful of routes is not a routing problem;
// what it is, is a SHAREABLE address — M6.2's whole point is that a table has a
// link somebody else can follow — plus back and forward working for free,
// because the browser already keeps a history of hashes. A path-based router
// would need the server to serve `index.html` under every path it does not
// know, which is a deployment change for a handful of screens — and since
// M6.5b2 `/enter` is a real route on that server, which such a fallback would
// have to be taught to leave alone.
//
// **`parseRoute` is total.** Anything it does not recognize is the DOOR, so a
// stale bookmark, a truncated paste and an empty hash all land on a screen a
// player can act from rather than on a blank one — and since M6.5b2 that screen
// is the one that needs no server, which is what makes the fallback true offline
// as well as on.

import { useEffect, useState } from "react";
import { isLocal } from "./local/ids";
import { isSafeId } from "./tableId";

export type Route =
  /** The front door at `#/` (R-6.5b2.1): play on this device, or play with
   *  others. **Everyone sees it, signed in or not** — it is drawn from this
   *  browser alone, so it is the one screen that renders with the API refused,
   *  never answering, or not deployed at all. */
  | { kind: "door" }
  /** The server's first screen, at its own address since M6.5b2. `/enter` (the
   *  Access turnstile) redirects here, so it has to BE somewhere: while the
   *  lobby was `#/` there was no hash a turnstile could send a player to that
   *  did not first land them on the door they had just come through. */
  | { kind: "lobby" }
  /** The setup form. **`local` says WHICH host the table is opened on**
   *  (M6.5b, R-6.5b.8): `#/new/local` seats you and the bot in this browser
   *  and is reachable with no door at all, which is what makes "Play the bot
   *  here" work on an API that cannot be reached. Absent rather than `false`
   *  for the server's create, so every existing caller's `{kind: "new"}` is
   *  still the same route. */
  | { kind: "new"; local?: true }
  | { kind: "table"; id: string };

/** `#/` the door, `#/lobby` the lobby, `#/new` and `#/new/local` the form,
 *  `#/t/<id>` a table (R-2.5, R-6.5b2.1). Anything else is the door, so a stale
 *  or mistyped link lands somewhere useful — and somewhere that draws without a
 *  server, which the lobby cannot. */
export function parseRoute(hash: string): Route {
  const path = hash.startsWith("#") ? hash.slice(1) : hash;
  if (path === "/lobby") return { kind: "lobby" };
  if (path === "/new") return { kind: "new" };
  if (path === "/new/local") return { kind: "new", local: true };
  const m = /^\/t\/([^/]+)$/.exec(path);
  // `decodeURIComponent` throws on a malformed escape (`%zz`), which is a
  // hand-typed link and not a bug in this client — so it reads as the door
  // like every other unrecognized hash, rather than as an exception thrown
  // during a render — and an id the store's grammar refuses is the door too
  // (security S2a, D14): `..` would otherwise reach `/api/games/../chat`.
  if (m) {
    try {
      const id = decodeURIComponent(m[1]!);
      return isSafeId(id) ? { kind: "table", id } : { kind: "door" };
    } catch {
      return { kind: "door" };
    }
  }
  return { kind: "door" };
}

/** Which of the two channels a screen belongs to (M6.5b2, R-6.5b2.1).
 *
 *  **Single player is a channel now, not a section.** The door, the local
 *  create and a `local-` table are drawn from this browser alone; everything
 *  else is the server's. The distinction is what decides who the client talks
 *  to at all: the LOCAL channel asks the server nothing, which is the whole of
 *  R-6.5b2.7 — a single-player visitor costs the Worker zero invocations. */
export type Channel = "local" | "multiplayer";

/** A route's channel. One function, because three effects and one hook ask the
 *  same question and three copies of the test are three places for the answer
 *  to drift the next time a route is added. */
export function channelOf(route: Route): Channel {
  if (route.kind === "door") return "local";
  if (route.kind === "new") return route.local ? "local" : "multiplayer";
  if (route.kind === "table") return isLocal(route.id) ? "local" : "multiplayer";
  return "multiplayer";
}

export function hashOf(route: Route): string {
  switch (route.kind) {
    case "door":
      return "#/";
    case "lobby":
      return "#/lobby";
    case "new":
      return route.local ? "#/new/local" : "#/new";
    case "table":
      return `#/t/${encodeURIComponent(route.id)}`;
  }
}

/** Go to a route.
 *
 *  **Guarded against setting the hash it already holds**, which is not a
 *  micro-optimization: an unconditional write pushes a duplicate history entry,
 *  so Back would step through the same screen twice — and `open()` calls this
 *  on every way into a table, including the one the route effect itself
 *  started.
 *
 *  **`replace` takes the current entry's place instead of adding one**
 *  (M6.5c2 cycle D): what `open` uses to put the address back after a refused
 *  switch, so Back does not lead to the table that refused. It still fires
 *  `hashchange`, so the route follows it as it follows any other move. */
export function navigate(route: Route, how: { replace?: boolean } = {}): void {
  const next = hashOf(route);
  if (window.location.hash === next) return;
  if (how.replace) window.location.replace(next);
  else window.location.hash = next;
}

/** Two routes are the same screen.
 *
 *  **The kind alone is not the question, for two of the three arms.** A table
 *  is named by its id, and since M6.5b a create is named by its HOST too: the
 *  two forms are two screens, and a sameness test that read the kind would
 *  hand a player who asked for the local form the server's, because the hook
 *  below would keep the object it already had. */
const same = (a: Route, b: Route): boolean => {
  if (a.kind !== b.kind) return false;
  if (a.kind === "table") return a.id === (b as { id: string }).id;
  if (a.kind === "new") return a.local === (b as { local?: true }).local;
  return true;
};

/** The route is the hash: read at mount, followed on `hashchange`, so back and
 *  forward work without any state of this component's own.
 *
 *  **The same route is the same OBJECT.** Consumers put this in an effect's
 *  dependency list, and a fresh object per `hashchange` would re-run those
 *  effects for a screen that has not moved — `App`'s route effect opens a table
 *  in one of its arms, so a churning identity there is a second request. */
export function useRoute(): Route {
  const [route, setRoute] = useState<Route>(() => parseRoute(window.location.hash));
  useEffect(() => {
    const onChange = () => {
      const next = parseRoute(window.location.hash);
      setRoute((r) => (same(r, next) ? r : next));
    };
    window.addEventListener("hashchange", onChange);
    // Read again on mount: a hash set between the initial state above and this
    // effect running would otherwise be missed for the life of the component.
    onChange();
    return () => window.removeEventListener("hashchange", onChange);
  }, []);
  return route;
}
