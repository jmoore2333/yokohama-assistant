// The two requests behind one path, as a stub reads them.
//
// **One module, because two things need it and one of them is the fake
// Worker.** `harness.tsx` imports `fakeWorker.ts`, so a matcher the fake also
// reads cannot live in the harness without a cycle; the harness re-exports
// both. Each takes either shape a cell meets: an `ApiCall`, whose path carries
// its query, or a local host's call, which carries it apart (`query`).

/** One request, as a matcher reads it. */
export interface Asked {
  method: string;
  path: string;
  /** Present on a local host's call, where the query is not in the path. */
  query?: string;
}

const split = (call: Asked): [string, string] => {
  if (call.query !== undefined) return [call.path, call.query];
  const [path, query] = call.path.split("?");
  return [path!, query ?? ""];
};

/** Is this call the LOBBY LISTING, and — when `scope` is given — that scope's?
 *
 *  **One matcher, because there are now three listings behind one path.** Every
 *  stub in this suite matched `GET /api/games` by its path alone, and M6.2's
 *  `?scope=open` walks straight through such a check: a cell stubbing the open
 *  list would silently be handed the `mine` reply and pass while asserting the
 *  wrong thing. M6.5c2 cycle D (dossier row 38) moved the last path-only
 *  matchers, the fake Worker's included, onto this one. */
export const isGamesList = (call: Asked, scope?: "mine" | "open" | "playing"): boolean => {
  const [path, query] = split(call);
  if (call.method !== "GET" || !path.endsWith("/api/games")) return false;
  return scope === undefined || new URLSearchParams(query).get("scope") === scope;
};

/** Is this call a CREATE — `POST /api/games`, the listing's path with the other
 *  method? Its own matcher (dossier row 38) rather than a sixth restatement of
 *  `path.split("?")` beside a method test. */
export const isCreate = (call: Asked): boolean => {
  const [path] = split(call);
  return call.method === "POST" && path.endsWith("/api/games");
};
