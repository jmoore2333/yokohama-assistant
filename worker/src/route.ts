/**
 * **Where a request goes** (spec §2.3's routing table) — by method and path
 * alone. What each target then does is `handle`'s (`index.ts`): a create's
 * object name comes from its body, which this function never sees.
 *
 * The asset layer hands the Worker only `/api/*`, `/enter` (its
 * `run_worker_first`) and a path it has no file for, so every other path is
 * `missing`.
 */
export type Target =
  /** No file, no route: 404 with an empty body, never `index.html`. */
  | { kind: "missing" }
  /** `GET|HEAD /enter`: the turnstile's 302 to `/#/lobby`. */
  | { kind: "enter" }
  /** `GET|HEAD /api/art`: the static manifest's bytes. */
  | { kind: "art" }
  /** Any other method on `/enter` or `/api/art`: 405, as axum's router answers. */
  | { kind: "not-allowed" }
  /** `POST /api/games`: the table object the body names, or a drawn one. */
  | { kind: "create" }
  /** `/api/games/{name}` and below: the table object `name`. */
  | { kind: "table"; name: string }
  /** `POST /api/archive`: the caller's own archive object. */
  | { kind: "archive" }
  /** `/api/admin` and below (spec §2.9); `rest` is the path after
   *  `/api/admin`. */
  | { kind: "admin"; rest: string }
  /** Everything else under `/api/`: the house, whose `dispatch` answers. */
  | { kind: "house" };

/** The longest name a table object is ever reached by, in UTF-8 bytes: the
 *  layer's `is_safe_id` bound. */
export const TABLE_NAME_MAX_BYTES = 64;

const GAMES = "/api/games/";
const ADMIN = "/api/admin";

/** A UTF-16 surrogate with no partner: no UTF-8, and no percent-encoding,
 *  can carry it. */
const LONE_SURROGATE = /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/;

/**
 * Whether `name` may name a table object: not empty, no lone surrogate, and
 * at most 64 UTF-8 bytes (the orchestrator's ruling, Task 9's review).
 * Anything else goes to the house, whose `dispatch` answers the layer's OWN
 * refusal for an id it cannot use — no name over 64 bytes is one the layer
 * uses — so no request makes a table object for such a name, and none over
 * 1,024 bytes can reach an object the platform gives no name (pool fact 8).
 */
export function routableName(name: string): boolean {
  return name !== "" && !LONE_SURROGATE.test(name) && new TextEncoder().encode(name).length <= TABLE_NAME_MAX_BYTES;
}

/** Spec §2.3's table: where `method` on `pathname` goes. */
export function route(method: string, pathname: string): Target {
  const read = method === "GET" || method === "HEAD";
  if (pathname === "/enter") return read ? { kind: "enter" } : { kind: "not-allowed" };
  if (!pathname.startsWith("/api/")) return { kind: "missing" };
  if (pathname === "/api/art") return read ? { kind: "art" } : { kind: "not-allowed" };
  if (pathname === "/api/games") return method === "POST" ? { kind: "create" } : { kind: "house" };
  if (pathname.startsWith(GAMES)) {
    // The segment exactly as the layer binds it: raw, never percent-decoded
    // (spec §0.2).
    const name = pathname.slice(GAMES.length).split("/", 1)[0] ?? "";
    return routableName(name) ? { kind: "table", name } : { kind: "house" };
  }
  if (pathname === "/api/archive" && method === "POST") return { kind: "archive" };
  if (pathname === ADMIN || pathname.startsWith(`${ADMIN}/`)) return { kind: "admin", rest: pathname.slice(ADMIN.length) };
  return { kind: "house" };
}
