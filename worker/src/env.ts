import type { Archive } from "./archive";
import type { House } from "./house";
import type { Table } from "./table";

/**
 * The Worker's bindings and variables (spec §3.1, the plan's Global
 * Constraints), written by hand — `wrangler types` is not used, so this is the
 * one statement of what the shell reads from `env`. `ASSETS`, the three
 * object namespaces and the bucket `ARCHIVE` are here because both
 * configurations bind them. `ARCHIVE` is optional all the same: a
 * configuration with the archive `off` needs none, and `shellConfig` refuses
 * `r2` without it.
 *
 * **Every variable is optional and every variable is a string.** A variable a
 * configuration does not set is absent, and `shellConfig` gives each absence
 * its meaning. A wrangler `vars` entry written without quotes would arrive as a
 * number or a boolean, which this type says cannot happen, so `shellConfig`
 * refuses it by name (the orchestrator's ruling E3) and the claim holds
 * wherever a configuration it accepted is in force. It checks every variable
 * declared here: `config.ts` lists them in a record the compiler holds to this
 * interface, so a variable added here without a row there does not compile.
 */
export interface Env {
  /** The static assets, bound by both configurations (spec §2.3's
   *  `/api/art` reads the art manifest through it). */
  ASSETS: Fetcher;
  /** The table objects (spec §2.4), one per table, each named by its id. */
  TABLE: DurableObjectNamespace<Table>;
  /** The house object (spec §2.5): ONE, named `house`. */
  HOUSE: DurableObjectNamespace<House>;
  /** The archive objects (spec §2.6), one per caller, each named
   *  `archive:<address>`. Not `ARCHIVE`, which is the bucket's name. */
  ARCHIVER: DurableObjectNamespace<Archive>;
  /** The bucket `yokohama-archive` (spec §3.1). `YOKOHAMA_WEB_ARCHIVE=r2` is
   *  refused without it. */
  ARCHIVE?: R2Bucket;
  /** The two Rate Limiting bindings (security S2b, finding C2; decision D-3): creates, 10 per 60 s, and writes,
   *  120 per 10 s, in production; the pool binds both with a lenient figure. Optional in the TYPE because a cell
   *  builds an `Env` by hand; `shellConfig` refuses a configuration lacking either, so no deployment runs without
   *  them (fail closed, as `ARCHIVE` under `r2`). */
  CREATE_LIMIT?: RateLimit;
  WRITE_LIMIT?: RateLimit;

  /** `worker` | `open`, with NO default (M6.5c2, spec §2.1; R-6.5c2.5): the
   *  door. `worker` is Cloudflare Access, verified by this shell — this
   *  repository's configuration; `open` is a deployment with no identity
   *  provider, where the name a caller claims is who they are. Unset, every
   *  `/api/*` answers the 500 naming it. */
  YOKOHAMA_WEB_IDENTITY?: string;
  /** `access` | `oidc` | `workos`, with NO default under the `worker` door (auth cycle, design §3): who vouches
   *  for an address — Cloudflare Access, a generic OpenID Connect issuer, or WorkOS's first-party flow. Refused
   *  beside `open`, where a name is a claim. */
  YOKOHAMA_IDENTITY_SOURCE?: string;
  /** The OpenID Connect issuer, whose `/.well-known/openid-configuration` names the endpoints: required under
   *  `oidc`, refused beside `workos`, ignored otherwise. */
  YOKOHAMA_OIDC_ISSUER?: string;
  /** The client id the issuer (or WorkOS) registered for this site: required under `oidc` and `workos`. */
  YOKOHAMA_OIDC_CLIENT_ID?: string;
  /** A SECRET (`wrangler secret put`): the client secret under `oidc`, optional (absent = the PKCE flow); the
   *  WorkOS API key under `workos`, required. */
  YOKOHAMA_OIDC_CLIENT_SECRET?: string;
  /** A SECRET (`wrangler secret put`): at least 32 UTF-8 bytes, signing every session cookie; required under
   *  `oidc` and `workos`. */
  YOKOHAMA_SESSION_SECRET?: string;
  /** Whole days above zero, absent = 30: how long a session cookie stands under `oidc` and `workos`. */
  YOKOHAMA_SESSION_DAYS?: string;
  /** The Access team's domain; its keys are
   *  `https://<domain>/cdn-cgi/access/certs` (spec §2.3). */
  ACCESS_TEAM_DOMAIN?: string;
  /** The Access application's audience (spec §2.3). */
  ACCESS_AUD?: string;
  /** The Service Auth credential's client id — the dev door's (spec §2.3). A
   *  SECRET in production, never a committed value (M6.5c2, R-6.5c2.5; the
   *  deploy guard's check 7); a `vars` entry in the pool's
   *  `wrangler.test.jsonc`. With none, the dev door is shut. */
  DEV_DOOR_CLIENT_ID?: string;
  /** `on` | `off`, absent = off: whether an assertion naming only that
   *  credential may claim a user through `X-Yokohama-Claimed-User`
   *  (R-6.5c.3). */
  YOKOHAMA_DEV_DOOR?: string;
  /** `on` | `off`, absent = off (R-SEC8(b)): whether a creator's seed is
   *  honoured at a table that seats somebody else — the corpus walk's shape.
   *  Set in the pool's two test files and nowhere else: the deploy guard
   *  refuses it in a deployable configuration. */
  YOKOHAMA_DIAGNOSTIC_SEEDS?: string;
  /** `r2` | `off`, absent = off (R-6.5c.14). */
  YOKOHAMA_WEB_ARCHIVE?: string;
  /** Whole days, absent = no table ever expires (R-6.5c.15). */
  YOKOHAMA_WEB_TABLE_TTL_DAYS?: string;
  /** Whole days, absent = 7: how long a table still WAITING for players
   *  stands without activity (R-6.5c2.42). Inert while
   *  `YOKOHAMA_WEB_TABLE_TTL_DAYS` is absent. */
  YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS?: string;
  /** Comma-separated addresses, absent = the operator routes are the layer's
   *  404 (spec §2.9). Set with `wrangler secret put`, never in a committed
   *  file (the plan's ruling P-1). */
  YOKOHAMA_WEB_ADMINS?: string;
  /** `on` | `off`, absent = off: whether a wake is counted in `meta.wakes`
   *  (spec §2.4). */
  YOKOHAMA_WEB_COUNT_WAKES?: string;
  /** The NAS's three chat variables, at its names and defaults. The module
   *  parses them with the layer's own function, so the shell passes them on
   *  raw. */
  YOKOHAMA_WEB_LOBBY_CHAT?: string;
  YOKOHAMA_WEB_CHAT_KEEP?: string;
  YOKOHAMA_WEB_CHAT_TEXT?: string;
  /** `on` | `off`, absent = off: a DEPLOY DECLARATION the shell never reads
   *  (M6.5c2 cycle F, R-FP2; spec §2.6) — whether this deploy ships a client
   *  built with a Turnstile site key, at the open door, behind the zone's edge
   *  rule. The deploy guard's check 12 holds it to the assets tree's
   *  `build-values.json` and to `YOKOHAMA_WEB_IDENTITY=open`, and the
   *  post-deploy smoke's live mode (`scripts/smoke-release.mjs --live`) reads
   *  it. Declared here so `config.ts` holds it to a string like every other
   *  variable; nothing else in the Worker reads it. */
  YOKOHAMA_TURNSTILE?: string;
  /** TEST ONLY: `frozen:<seconds>` or `tick:<seconds>`, absent = the wall
   *  clock (the plan's ruling P-2). Set in `wrangler.test.jsonc` and never in
   *  production. */
  YOKOHAMA_WEB_TEST_CLOCK?: string;
}
