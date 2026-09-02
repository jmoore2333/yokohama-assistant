// The public edge for the Yokohama browser client.
//
// **What this Worker is for, in one sentence:** it is the only thing on the
// Internet allowed to talk to the Modal container, and it is the thing
// Cloudflare Access authenticates in front of.
//
// The shape, and why each hop exists:
//
//   browser
//     -> yokohama.moore.nyc            Cloudflare Access authenticates HERE
//     -> this Worker                   identity available as ctx.access
//     -> https://<...>.modal.run       Modal proxy-auth rejects anyone else
//     -> yokohama-web (axum)           no auth of its own, by design
//
// **The middle hop is not decoration.** A Modal web endpoint is world-reachable
// by default, so an Access policy in front of a Worker that forwarded to an
// unauthenticated `*.modal.run` URL would protect nothing at all: anyone who
// learned the endpoint would skip the Worker entirely. `requires_proxy_auth`
// on the Modal side plus the two secrets below are what make the Worker the
// only door.
//
// **The origin server has no authentication of any kind and that is deliberate,
// not an oversight.** It has no notion of who is asking: there is no session,
// no header read, and `GET /api/games` lists every game id to any caller. It is
// a hot-seat driver for one operator at one screen. Everything that makes it
// safe to expose is in this file and in the Access policy — so if either is
// removed, the correct assumption is that the game store is public.

interface Env {
  /** Modal proxy-auth token id, `wk-…`. `wrangler secret put MODAL_KEY`. */
  MODAL_KEY: string;
  /** Modal proxy-auth token secret, `ws-…`. `wrangler secret put MODAL_SECRET`. */
  MODAL_SECRET: string;
  /** The Modal endpoint origin, e.g. `https://jamoore--yokohama-web-serve.modal.run`. */
  MODAL_ORIGIN: string;
}

/** Headers that describe ONE hop and must not be forwarded to the next. */
const HOP_BY_HOP = new Set([
  "connection",
  "keep-alive",
  "proxy-authenticate",
  "proxy-authorization",
  "te",
  "trailer",
  "transfer-encoding",
  "upgrade",
]);

/**
 * Headers a client must never be able to set, because the origin would have no
 * way to tell them from ours.
 *
 * The origin does not read these today — it reads no headers at all — but a
 * proxy that forwards a client-supplied `X-Yokohama-User` is a proxy that has
 * pre-built an impersonation bug for whoever adds the first `if` to the server.
 * Stripping on the way in is the cheap half; it is done here so that adding
 * identity to the origin later is a safe change rather than a dangerous one.
 */
const CLIENT_MUST_NOT_SET = ["x-yokohama-user", "modal-key", "modal-secret"];

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    if (!env.MODAL_ORIGIN || !env.MODAL_KEY || !env.MODAL_SECRET) {
      // Fail CLOSED and say which half is missing. A misconfigured proxy that
      // forwarded anyway would be an open door to the game store.
      return json(503, {
        error:
          "this deployment is not configured: MODAL_ORIGIN, MODAL_KEY and MODAL_SECRET must all be set",
      });
    }

    // **Identity without JWT verification.** Access attached to a Worker gives
    // the authenticated user directly, which is the documented alternative to
    // validating `Cf-Access-Jwt-Assertion` at an origin — and it is a stronger
    // position than trusting `Cf-Access-Authenticated-User-Email`, because a
    // header an origin merely trusts is spoofable by anything that can reach
    // that origin.
    //
    // Absent when Access is not configured. That is NOT treated as a failure
    // here: it is legitimate while the policy is being set up, and pretending
    // otherwise would mean the first deploy could never be tested. The
    // consequence is stated rather than hidden — with no Access policy, this
    // Worker is a public proxy to the game store.
    let user = "anonymous";
    if (ctx.access) {
      const identity = await ctx.access.getIdentity();
      user = identity?.email ?? "authenticated";
    }

    const url = new URL(request.url);
    const target = new URL(url.pathname + url.search, env.MODAL_ORIGIN);

    const headers = new Headers();
    for (const [k, v] of request.headers) {
      const key = k.toLowerCase();
      if (HOP_BY_HOP.has(key) || CLIENT_MUST_NOT_SET.includes(key)) continue;
      headers.set(k, v);
    }
    headers.set("Modal-Key", env.MODAL_KEY);
    headers.set("Modal-Secret", env.MODAL_SECRET);
    headers.set("X-Yokohama-User", user);
    // The origin builds absolute URLs for nothing, but a proxied request whose
    // Host says `*.modal.run` is a request whose logs lie about where it came
    // from.
    headers.set("X-Forwarded-Host", url.host);

    const upstream = new Request(target, {
      method: request.method,
      headers,
      body: request.body,
      redirect: "manual",
    });

    let response: Response;
    try {
      response = await fetch(upstream);
    } catch (e) {
      // The client parses JSON. An HTML error page here is exactly the failure
      // `web/src/api.ts` documents — the player saw `Unexpected token '<'` and
      // nothing about what actually happened — so a proxy failure answers in
      // this API's own shape.
      return json(502, {
        error: `the game server could not be reached: ${e instanceof Error ? e.message : String(e)}`,
      });
    }

    if (response.status === 401 || response.status === 403) {
      // Modal refusing OUR credentials is an operator problem, not the
      // player's, and it must not be mistaken for the game refusing a move.
      return json(502, {
        error:
          "the edge is not authorised to reach the game server — the Modal proxy token is missing, wrong, or revoked",
      });
    }

    // Body streamed through untouched; only the headers are rebuilt.
    const out = new Headers(response.headers);
    for (const k of HOP_BY_HOP) out.delete(k);
    return new Response(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers: out,
    });
  },
} satisfies ExportedHandler<Env>;

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}
