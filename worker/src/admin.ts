/**
 * **The operator surface's rules** (spec §2.9, R-6.5c.16): who is an
 * operator, which of the surface's two routes a request names, what the
 * listing's rows carry, and the two host-written refusals. Pure — no Workers
 * API. The Worker runs the gate and routes (`index.ts`); the house answers
 * the listing (`house.ts`), and a table object an expire (`table.ts`).
 */
import type { Identity } from "./identity";
import { routableName } from "./route";

/**
 * The operator's address, on a request the gate passed (the plan's internal
 * headers). Only the Worker's `handle` sets it, after the gate; a client's is
 * removed with every other `x-yokohama-*` header before anything is
 * forwarded, so an object that reads it reads the gate's word.
 */
export const OPERATOR_HEADER = "X-Yokohama-Operator";

/** The gate's 403 (spec §2.9; host-written, §4). */
export const NOT_AN_OPERATOR = "this address is not an operator here";

/** A force's 503, while the house has not acknowledged its removal (spec
 *  §2.9; host-written, §4). */
export const UNCONFIRMED = "the lobby has not confirmed the removal — the alarm will finish it";

/** The 400 for `?force` given a value (the orchestrator's ruling I-72;
 *  host-written). */
export const FORCE_TAKES_NO_VALUE = "?force takes no value — send ?force alone, or no query for an expire that saves first";

/**
 * The gate's verdict on a request to `/api/admin` or below (spec §2.9):
 *
 * - `off`: `YOKOHAMA_WEB_ADMINS` is absent, and the routes are unrouted
 *   `/api/*` — the house's `dispatch`, which answers the layer's own 404;
 * - `operator`: a PERSON — an Access assertion that names an address — whose
 *   address the list holds. Both are trimmed and ASCII-lowercased as the
 *   layer's `User::parse` reads an address (`resolveIdentity`, `shellConfig`),
 *   so they compare as the layer would;
 * - `refused`: anyone else — a person not listed, no one, and an identity
 *   reached through the dev door, which NEVER counts, even when the address
 *   it claims is on the list.
 */
export type Gate = { kind: "off" } | { kind: "refused" } | { kind: "operator"; email: string };

export function operatorGate(admins: string[] | null, identity: Identity): Gate {
  if (admins === null) return { kind: "off" };
  if (identity.kind === "person" && admins.includes(identity.email)) return { kind: "operator", email: identity.email };
  return { kind: "refused" };
}

/**
 * Which route a request under `/api/admin` names (spec §2.9), by method, path
 * and query — keys compared exactly:
 *
 * - `GET /api/admin/tables` with no query: the listing;
 * - `POST /api/admin/tables/{id}/expire`, where `id` is a name a table object
 *   can have (`routableName`, the path's segment as the layer binds it: raw):
 *   with no query, a plain expire; with the BARE key `force` — `?force`, or
 *   `?force=` — a force; with `force` given a value, `refused` with its 400
 *   (the orchestrator's ruling I-72: `?force=false` forcing would drop a save
 *   the operator meant to keep, so a value is neither a force nor a plain
 *   expire);
 * - anything else — another method or path, or a query holding any other
 *   key, `?Force` among them — unrouted, the layer's own 404.
 */
export type AdminRoute =
  | { kind: "listing" }
  | { kind: "expire"; name: string; force: boolean }
  | { kind: "refused"; sentence: string }
  | { kind: "unrouted" };

const EXPIRE = /^\/api\/admin\/tables\/([^/]+)\/expire$/;

export function adminRoute(method: string, pathname: string, query: string): AdminRoute {
  const keys = [...new URLSearchParams(query)];
  if (pathname === "/api/admin/tables") return method === "GET" && keys.length === 0 ? { kind: "listing" } : { kind: "unrouted" };
  const name = EXPIRE.exec(pathname)?.[1];
  if (name === undefined || method !== "POST" || !routableName(name)) return { kind: "unrouted" };
  const [first, ...more] = keys;
  if (first === undefined) return { kind: "expire", name, force: false };
  const [key, value] = first;
  if (key !== "force" || more.length > 0) return { kind: "unrouted" };
  return value === "" ? { kind: "expire", name, force: true } : { kind: "refused", sentence: FORCE_TAKES_NO_VALUE };
}

/**
 * One row of the operator's listing (spec §2.9) — an operator-only payload,
 * outside the corpus: the table's id, name, status, players, `created_at` and
 * `archived_at` (the layer's unix SECONDS), `over` and `winner`, read off the
 * house's scrubbed copy, and its `last_active` (the shell's MILLISECONDS) and
 * `bytes` (the table object's SQLite size) as of the table's last delivery
 * (spec §2.5).
 */
export interface OperatorRow {
  id: string;
  name: string | null;
  /** The layer's `TableStatus`: "over" is not a status (`over` says it). */
  status: "open" | "dealing" | "playing";
  players: number;
  created_at: number;
  last_active: number;
  bytes: number;
  over: boolean;
  winner: number | null;
  archived_at: number | null;
}
