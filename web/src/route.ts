// Which screen this is, as the URL (spec §5.1).
//
// **The hash, and no router.** Three routes is not a routing problem; what it
// is, is a SHAREABLE address — M6.2's whole point is that a table has a link
// somebody else can follow — plus back and forward working for free, because
// the browser already keeps a history of hashes. A path-based router would need
// the server to serve `index.html` under every path it does not know, which is
// a deployment change for three screens.
//
// **`parseRoute` is total.** Anything it does not recognize is the lobby, so a
// stale bookmark, a truncated paste and an empty hash all land on a screen a
// player can act from rather than on a blank one.

import { useEffect, useState } from "react";

export type Route =
  | { kind: "lobby" }
  /** The setup form. **`local` says WHICH host the table is opened on**
   *  (M6.5b, R-6.5b.8): `#/new/local` seats you and the bot in this browser
   *  and is reachable with no door at all, which is what makes "Play the bot
   *  here" work on an API that cannot be reached. Absent rather than `false`
   *  for the server's create, so every existing caller's `{kind: "new"}` is
   *  still the same route. */
  | { kind: "new"; local?: true }
  | { kind: "table"; id: string };

/** `#/` the lobby, `#/new` and `#/new/local` the form, `#/t/<id>` a table
 *  (R-2.5). Anything else is the lobby, so a stale or mistyped link lands
 *  somewhere useful. */
export function parseRoute(hash: string): Route {
  const path = hash.startsWith("#") ? hash.slice(1) : hash;
  if (path === "/new") return { kind: "new" };
  if (path === "/new/local") return { kind: "new", local: true };
  const m = /^\/t\/([^/]+)$/.exec(path);
  // `decodeURIComponent` throws on a malformed escape (`%zz`), which is a
  // hand-typed link and not a bug in this client — so it reads as the lobby
  // like every other unrecognized hash, rather than as an exception thrown
  // during a render.
  if (m) {
    try {
      return { kind: "table", id: decodeURIComponent(m[1]!) };
    } catch {
      return { kind: "lobby" };
    }
  }
  return { kind: "lobby" };
}

export function hashOf(route: Route): string {
  switch (route.kind) {
    case "lobby":
      return "#/";
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
 *  started. */
export function navigate(route: Route): void {
  const next = hashOf(route);
  if (window.location.hash !== next) window.location.hash = next;
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
