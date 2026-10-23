import type { Door, Limits } from "./config";
import { callerName, type Identity } from "./identity";
import { callerTag } from "./redact";
import { errorBody, respond } from "./respond";
import type { Target } from "./route";

/** The 429's sentence (security S2b, finding C2; host-written). */
export const TOO_MANY = "too many requests from you just now — wait a moment and try again";
/** A refused create's `Retry-After`, in seconds: the creates' window (10 per 60 s). */
export const RETRY_AFTER_CREATE = "60";
/** A refused write's `Retry-After`, in seconds: the writes' window (120 per 10 s). */
export const RETRY_AFTER_WRITE = "10";
/** Where the edge puts the connecting IP. */
export const IP_HEADER = "cf-connecting-ip";
/** The IP's stand-in when the header is absent — the pool, or an edge that did not set it. */
export const UNKNOWN_IP = "unknown";

/** What a binding is to this module: `limit` and nothing else, so a cell hands in a fake. */
export type Limiter = Pick<RateLimit, "limit">;

/**
 * **The key a request is counted under** (decision D-3). Under `worker` every caller is a verified address, so
 * the caller's digest (`callerTag`), whatever their IP: a person on two devices is one budget. Under `open` a
 * claim is free, so a create is counted by the connecting IP alone (a flood of names from one address is one
 * budget) and a write by IP and claim together (a household's players each get their own). A request with no
 * caller is counted by IP under either door — the layer answers it 401, and its object call is still a cost. So is
 * an ANONYMOUS upload (the auth cycle, design §4.4's ceiling 2): its device or game id is whatever the client sends,
 * so a per-device key is one a client could rotate, and the connecting IP is the budget. An absent IP header reads
 * `unknown`: one shared budget for a request that reached this Worker without the edge.
 */
export async function limitKey(door: Door, identity: Identity, ip: string | null, kind: "create" | "write"): Promise<string> {
  const who = identity.kind === "anonymous" ? undefined : callerName(identity);
  const at = `ip:${ip ?? UNKNOWN_IP}`;
  if (who === undefined) return at;
  if (door === "worker") return callerTag(who);
  return kind === "create" ? at : `${at}|${await callerTag(who)}`;
}

/** Which binding an unsafe request spends: the create's for `POST /api/games`, and for an `anonymous` caller's
 *  `POST /api/archive` (the auth cycle, design §4.4: one machine mints devices no faster than games); the writes' for
 *  every other method but GET and HEAD — the operator's POST, a signed-in caller's `/api/archive` and every DELETE
 *  among them; `undefined` for a read. */
export function limiterFor(
  method: string,
  target: Target,
  limits: Limits,
  anonymous: boolean,
): { limiter: Limiter; kind: "create" | "write" } | undefined {
  if (method === "GET" || method === "HEAD") return undefined;
  const creates = target.kind === "create" || (anonymous && target.kind === "archive");
  return creates ? { limiter: limits.create, kind: "create" } : { limiter: limits.write, kind: "write" };
}

/**
 * The 429 a request past its limit answers, or `undefined` to proceed (security S2b, finding C2; decision D-3).
 * Asked in `answer` after the caller is known and BEFORE any object is reached — so a refused request costs the
 * stateless Worker's invocation and no Durable Object's. The platform counts per Cloudflare location, best
 * effort: a bound on cost, never a fence a feature may lean on. `Retry-After` is the refused kind's own window.
 */
export async function limited(req: Request, target: Target, identity: Identity, door: Door, limits: Limits): Promise<Response | undefined> {
  const spend = limiterFor(req.method, target, limits, identity.kind === "anonymous");
  if (spend === undefined) return undefined;
  const key = await limitKey(door, identity, req.headers.get(IP_HEADER), spend.kind);
  const { success } = await spend.limiter.limit({ key });
  if (success) return undefined;
  const res = respond(429, errorBody(TOO_MANY));
  res.headers.set("Retry-After", spend.kind === "create" ? RETRY_AFTER_CREATE : RETRY_AFTER_WRITE);
  return res;
}
