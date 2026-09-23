/**
 * The Worker's bindings and variables (spec §3.1, the plan's Global
 * Constraints), written by hand — `wrangler types` is not used, so this is the
 * one statement of what the shell reads from `env`. `ASSETS` is here because
 * both configurations bind it, and `ARCHIVE` because `shellConfig` asks
 * whether it is bound — no configuration binds it yet, so it is optional. The
 * Durable Object namespaces arrive with their bindings.
 *
 * **Every variable is optional and every variable is a string.** A variable a
 * configuration does not set is absent, and `shellConfig` gives each absence
 * its meaning. A wrangler `vars` entry written without quotes would arrive as a
 * number or a boolean, which this type says cannot happen, so `shellConfig`
 * refuses it by name (the orchestrator's ruling E3) and the claim holds
 * wherever a configuration it accepted is in force.
 */
export interface Env {
  /** The static assets, bound by both configurations (spec §2.3's
   *  `/api/art` reads the art manifest through it). */
  ASSETS: Fetcher;
  /** The bucket `yokohama-archive` (spec §3.1). `YOKOHAMA_WEB_ARCHIVE=r2` is
   *  refused without it. */
  ARCHIVE?: R2Bucket;

  /** The Access team's domain; its keys are
   *  `https://<domain>/cdn-cgi/access/certs` (spec §2.3). */
  ACCESS_TEAM_DOMAIN?: string;
  /** The Access application's audience (spec §2.3). */
  ACCESS_AUD?: string;
  /** The Service Auth credential's client id — the dev door's (spec §2.3). */
  DEV_DOOR_CLIENT_ID?: string;
  /** `on` | `off`, absent = off: whether an assertion naming only that
   *  credential may claim a user through `X-Yokohama-Claimed-User`
   *  (R-6.5c.3). */
  YOKOHAMA_DEV_DOOR?: string;
  /** `r2` | `off`, absent = off (R-6.5c.14). */
  YOKOHAMA_WEB_ARCHIVE?: string;
  /** Whole days, absent = no table ever expires (R-6.5c.15). */
  YOKOHAMA_WEB_TABLE_TTL_DAYS?: string;
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
  /** TEST ONLY: `frozen:<seconds>` or `tick:<seconds>`, absent = the wall
   *  clock (the plan's ruling P-2). Set in `wrangler.test.jsonc` and never in
   *  production. */
  YOKOHAMA_WEB_TEST_CLOCK?: string;
}
