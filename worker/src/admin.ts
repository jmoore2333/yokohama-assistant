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
 * Which route a request under `/api/admin` names (spec §2.9): `GET
 * /api/admin/tables`, the listing; `POST /api/admin/tables/{id}/expire`, an
 * expire — a force when the query names `force` — of the table `id`, where
 * `id` is a name a table object can have (`routableName`, the path's segment
 * as the layer binds it: raw); anything else is unrouted.
 */
export type AdminRoute = { kind: "listing" } | { kind: "expire"; name: string; force: boolean } | { kind: "unrouted" };

const EXPIRE = /^\/api\/admin\/tables\/([^/]+)\/expire$/;

export function adminRoute(method: string, pathname: string, query: string): AdminRoute {
  if (pathname === "/api/admin/tables") return method === "GET" ? { kind: "listing" } : { kind: "unrouted" };
  const name = EXPIRE.exec(pathname)?.[1];
  if (name === undefined || method !== "POST" || !routableName(name)) return { kind: "unrouted" };
  return { kind: "expire", name, force: new URLSearchParams(query).has("force") };
}

/**
 * One row of the operator's listing (spec §2.9) — an operator-only payload,
 * outside the corpus: the table's id, name, status, players, `created_at`,
 * `over`, `winner` and `archived_at`, read off the house's scrubbed copy, and
 * its `last_active` and `bytes` (the table object's SQLite size) as of the
 * table's last delivery (spec §2.5).
 */
export interface OperatorRow {
  id: string;
  name: string | null;
  status: string;
  players: number;
  created_at: number;
  last_active: number;
  bytes: number;
  over: boolean;
  winner: number | null;
  archived_at: number | null;
}
