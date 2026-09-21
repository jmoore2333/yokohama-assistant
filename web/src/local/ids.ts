// What a LOCAL game is called, and where a new one's name comes from
// (R-6.5b.12).
//
// **One definition, two importers, and that is why it is its own file.** The
// page's predicate belongs in `api.ts` — every reader that has an id goes
// through it, which is what keeps "which host is this game's?" from being
// asked two ways — and `api.ts` re-exports these. But the WORKER draws the
// ids, and the Worker compiles under `tsconfig.worker.json` with no DOM at
// all, while `api.ts` reads `localStorage`. A second copy of the prefix in
// `worker.ts` is the thing this file exists to avoid: two statements of one
// string, one of which can be changed alone.
//
// Nothing here touches a global that is not in BOTH libs: `crypto` is on
// `window` and on `WorkerGlobalScope`.

/** Every game played on this device is named for it. `is_safe_id`
 *  (`store.rs:173`) admits it: an alphanumeric first character, then
 *  alphanumerics, `.`, `_` and `-`. */
export const LOCAL_PREFIX = "local-";

/** The one address a local game is ever played by (R-6.5b.1). Never a person:
 *  the server's archive attributes a browser game to the SERVER's caller, and
 *  no listing on any server ever prints this.
 *
 *  **Here rather than in `worker.ts`, for this file's own reason.** The Worker
 *  hands it to `dispatch` and the PAGE seats it on a local create — the module
 *  answers as this address whatever door the browser signed in at, so a form
 *  that posted the signed-in name would open a table its own player holds no
 *  seat at. Two statements of one string is what this file exists to avoid. */
export const LOCAL_USER = "local@browser";

/** Is this game the browser's own?
 *
 *  **At its real size** (spec R-6.5b.12): a SERVER table can be given this
 *  prefix by a hand-written request — the layer reserves nothing — and the
 *  page would then route it to the Worker, which answers 404. A confusion the
 *  browser's own client never causes, and never a leak. */
export const isLocal = (id: string): boolean => id.startsWith(LOCAL_PREFIX);

/** The game id a path names, or `null` where it names none — the five routes
 *  that carry no id (R-6.5b.12) and `/api/archive`.
 *
 *  **One statement, three importers** (`api.ts`, `client.ts`, `worker.ts`).
 *  The page and the Worker each had their own, with different character
 *  classes, agreeing only because the page splits the query off first — which
 *  is the kind of quiet agreement that stops being one. The class admits a
 *  `?` nowhere, so a path with a query attached still answers the id. */
export function gameIdOf(path: string): string | null {
  const m = /^\/api\/games\/([^/?]+)/.exec(path);
  if (!m) return null;
  try {
    return decodeURIComponent(m[1]!);
  } catch {
    // A malformed escape is a hand-written path, not a bug here: hand the raw
    // segment on and let whoever owns the store refuse it in its own words.
    return m[1]!;
  }
}

/** A fresh local id: `local-` and sixteen hex digits from the platform's own
 *  entropy.
 *
 *  **64 bits and not a counter**, for the reason the server draws its ids that
 *  way: an id is a name two tabs, two devices and two archive sends can
 *  collide on, and the archive files a browser game under the caller's prefix
 *  and this id. The SEED is not drawn here — `draw_seed` inside the module
 *  draws it through `getrandom`, and it is never typed. */
export function drawLocalId(): string {
  const bytes = new Uint8Array(8);
  crypto.getRandomValues(bytes);
  return LOCAL_PREFIX + [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
}
