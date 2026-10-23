/**
 * **Where a request goes** (spec §2.3's routing table) — by method and path
 * alone. What each target then does is `handle`'s (`index.ts`): a create's
 * object name comes from its body, which this function never sees.
 *
 * The asset layer hands the Worker `/api/*` and `/enter` (its
 * `run_worker_first`). It answers a miss with its own 404 page
 * (`not_found_handling: "404-page"`): read in every request shape for a miss
 * under an allowed prefix (`/assets/…`, `/art/…`), with no Worker
 * invocation, on 2026-10-01 (`worker/RUNBOOK.md` §20). What answers any other
 * miss without the edge rule was not counted, and the design expects the
 * Worker for a request that is not a navigation (spec §2.5). Any path outside
 * `/api/` and `/enter` that does reach the Worker is `missing`.
 */
export type Target =
  /** No file, no route: 404 with an empty body, never `index.html`. */
  | { kind: "missing" }
  /** `GET|HEAD /enter`: under `access` and the open door, the turnstile's 302 to `/#/lobby`; under `oidc` and `workos`,
   *  the sign-in's start, or with a `code` (or an issuer's `error`) its callback (the auth cycle, design §4.3;
   *  `index.ts`, `enter`). */
  | { kind: "enter" }
  /** `GET|HEAD /api/art`: the static art manifest, projected onto the
   *  layer's nine maps (`index.ts`, `art`). */
  | { kind: "art" }
  /** Any other method on `/enter`, `/api/art` or `/api/leave`: 405, as axum's router answers. */
  | { kind: "not-allowed" }
  /** `POST /api/leave`: sign-out, answered by the Worker itself (the auth cycle, design §4.2; `index.ts`, `leave`). */
  | { kind: "leave" }
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

/** The longest id, in bytes — every accepted id is ASCII, so in characters too (`store::is_safe_id`'s
 *  `id.len() <= 64`). */
export const ID_MAX = 64;

const GAMES = "/api/games/";
const ADMIN = "/api/admin";

/** The suffixes the store keeps for its own documents (`RESERVED_ID_SUFFIXES`), refused in any case. */
const RESERVED_SUFFIXES = [".table", ".chat"];

/**
 * **The store's id grammar, restated** (security S2b, finding C2; decision D-11): `store::is_safe_id`
 * (`crates/yokohama-web/src/store.rs`) is the authority, `web/src/tableId.ts` restates it for the hash router, and
 * this restates it for the shell's routing, held both ways to `web/src/wire/id_cases.json` by
 * `route.unit.test.ts`. Arm by arm: not empty, at most 64 bytes, a letter or digit first, then letters, digits,
 * `.`, `_` and `-` only, and not spelling another document's suffix. The alphabet test comes before the suffix
 * test and makes the ASCII fold exact. The store's sixth arm, `is_the_house` (`_house`), is not restated: its
 * leading `_` fails the first-character test, and the fixture carries the row.
 *
 * Before this a segment of any text up to 64 bytes named a Table object (`routableName`), so `a%20b`, `_x`,
 * `x.table` and `%C3%BC` each instantiated an object and a module, then stored nothing (the review's W1-4). A name
 * the store refuses is the house's now, whose `dispatch` answers the layer's own 404 for an unusable id (security
 * S1c, B8), and no object is made for it.
 */
export function isSafeId(id: string): boolean {
  if (id === "" || id.length > ID_MAX) return false;
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(id)) return false;
  const lower = id.replace(/[A-Z]/g, (c) => c.toLowerCase());
  return !RESERVED_SUFFIXES.some((suffix) => lower.endsWith(suffix));
}

/**
 * Whether a request a table object receives is the ABANDON — `DELETE
 * /api/games/{name}` and nothing below it (M6.5c2 cycle D, R-D4). The table
 * object runs the abandon WHOLE under `blockConcurrencyWhile`, with its save
 * uploaded between two dispatches (spec §2.4 step 1, §2.6); every other
 * request, a chat message's `DELETE /api/games/{name}/chat/{seq}` among them,
 * is the ordinary request path. Until the chat's delete there was no other
 * DELETE under a table, and the object read the method alone.
 */
export function abandons(method: string, pathname: string): boolean {
  return method === "DELETE" && pathname.startsWith(GAMES) && !pathname.slice(GAMES.length).includes("/");
}

/** Spec §2.3's table: where `method` on `pathname` goes. */
export function route(method: string, pathname: string): Target {
  const read = method === "GET" || method === "HEAD";
  if (pathname === "/enter") return read ? { kind: "enter" } : { kind: "not-allowed" };
  if (!pathname.startsWith("/api/")) return { kind: "missing" };
  if (pathname === "/api/art") return read ? { kind: "art" } : { kind: "not-allowed" };
  if (pathname === "/api/leave") return method === "POST" ? { kind: "leave" } : { kind: "not-allowed" };
  if (pathname === "/api/games") return method === "POST" ? { kind: "create" } : { kind: "house" };
  if (pathname.startsWith(GAMES)) {
    // The segment exactly as the layer binds it: raw, never percent-decoded
    // (spec §0.2).
    const name = pathname.slice(GAMES.length).split("/", 1)[0] ?? "";
    return isSafeId(name) ? { kind: "table", name } : { kind: "house" };
  }
  if (pathname === "/api/archive" && method === "POST") return { kind: "archive" };
  if (pathname === ADMIN || pathname.startsWith(`${ADMIN}/`)) return { kind: "admin", rest: pathname.slice(ADMIN.length) };
  return { kind: "house" };
}

/** Where a sign-in lands when it names nowhere the client would draw. */
const LOBBY = "/#/lobby";

/** The fixed hash routes besides a table's: `parseRoute`'s (`web/src/route.ts`) lobby and its two forms. */
const FIXED_ROUTES: ReadonlySet<string> = new Set([LOBBY, "/#/new", "/#/new/local"]);

/** A table's hash route: `/#/t/` and the rest, which must be one segment the store accepts. */
const TABLE_ROUTE = "/#/t/";

/**
 * **Where a finished sign-in lands** (the auth cycle, design §4.3; rulings R-23, C-12): `raw` when it is one of the
 * hash routes the client's `parseRoute` (`web/src/route.ts`) reads as somewhere other than its door — exactly
 * `/#/lobby`, `/#/new`, `/#/new/local`, or `/#/t/<id>` where the RAW `<id>` passes `isSafeId` — and `/#/lobby` for
 * anything else: another origin, a path, a hash outside the grammar, nothing at all. The segment is never
 * percent-decoded, so `%`, outside the store's alphabet, lands on the lobby even where the client would decode it to
 * a usable id; and `isSafeId`'s alphabet holds no `/`, so the segment is one segment. The landing is the browser's
 * `Location` after a sign-in, so it can only ever name this origin's own page.
 */
export function safeReturnTo(raw: string | null): string {
  if (raw === null) return LOBBY;
  if (FIXED_ROUTES.has(raw)) return raw;
  return raw.startsWith(TABLE_ROUTE) && isSafeId(raw.slice(TABLE_ROUTE.length)) ? raw : LOBBY;
}
