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
//     -> yokohama-web (axum)           reads the X-Yokohama-User we set
//
// **The middle hop is not decoration.** A Modal web endpoint is world-reachable
// by default, so an Access policy in front of a Worker that forwarded to an
// unauthenticated `*.modal.run` URL would protect nothing at all: anyone who
// learned the endpoint would skip the Worker entirely. `requires_proxy_auth`
// on the Modal side plus the two secrets below are what make the Worker the
// only door.
//
// **The origin now READS the header this Worker sets, which is what makes the
// stripping below load-bearing rather than precautionary.** Since M6.1 the
// server runs under `YOKOHAMA_WEB_IDENTITY`; on this path the value is
// `worker`, so `X-Yokohama-User` IS the caller's identity, every `/api/games`
// route requires it, and each table serves a caller only the seats they hold.
// `anonymous` — what this Worker sends when no Access policy is in front of it
// — is treated by the origin as no identity at all, so an unprotected Worker
// gets 401s rather than a wide-open store.
//
// **What that does NOT change.** The origin trusts this header completely: it
// verifies no signature and cannot tell our header from anyone else's. So the
// Access policy in front and the Modal proxy token below are still the whole of
// the security, and a client-supplied `X-Yokohama-User` that reached the origin
// would now be an impersonation of any seat at any table rather than a no-op.

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
 * **This stopped being precautionary at M6.1.** The comment here used to say
 * the origin "does not read these today — it reads no headers at all", and the
 * stated point was that adding identity to the origin later should be a safe
 * change rather than a dangerous one. That later has arrived: under
 * `YOKOHAMA_WEB_IDENTITY=worker` the origin takes `x-yokohama-user` as the
 * caller and scopes every table to the seats that caller holds. Forwarding a
 * client-supplied one would therefore be an impersonation of any player at any
 * table, and this line is the only thing between the two.
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
    // consequence is stated rather than hidden — with no Access policy every
    // caller arrives as `anonymous`, which the origin treats as no identity, so
    // the game store answers 401 and only the public surfaces are reachable.
    // That is fail-closed rather than safe: nothing here authenticates, and the
    // store is one Access policy away from being served to whoever asks.
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
      //
      // **KNOWN DEFECT since M6.1, recorded rather than fixed here because this
      // path is not deployed** (RUNBOOK.md §4; the NAS runs `claimed` and never
      // reaches this file). The origin itself now answers 401 for a request
      // with no identity and 403 for a caller with no seat at that table — two
      // statuses this branch cannot tell from Modal's, so it would rewrite an
      // honest "you are not seated at this table" into an operator's
      // credentials error. Modal refuses at its proxy before a container
      // starts, so the two are distinguishable in principle (the origin's
      // refusal carries this API's own `{"error": …}` JSON shape and Modal's
      // does not). Fixing it belongs with the first real Modal deployment,
      // where it can be tested; guessing at the discriminator now would mean
      // shipping an untested one.
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
