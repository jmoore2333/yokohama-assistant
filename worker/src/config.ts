import type { Env } from "./env";
import { isAddress } from "./identity";
import type { Limiter } from "./limits";
import { openTtlDaysOf, ttlDaysOf } from "./marker";

/** Where the shell reads the time (the plan's ruling P-2): the wall clock, a
 *  frozen reading, or a reading that ticks one second per use. */
export type Clock = { kind: "wall" } | { kind: "frozen"; at: number } | { kind: "tick"; next: number };

/** The Worker's door (M6.5c2, spec §2.1; R-6.5c2.5). */
export type Door = "worker" | "open";

/** Who vouches for a caller's address (the auth cycle, design §3): Cloudflare Access (`access`, this shell's
 *  verification of the assertion), a generic OpenID Connect issuer (`oidc`) or WorkOS's first-party flow
 *  (`workos`). One source per deployment, and no default. */
export type Source = "access" | "oidc" | "workos";

/** The two Rate Limiting bindings as the shell uses them (security S2b, finding C2; decision D-3): each a
 *  `Limiter` — `limit` and nothing else, so a cell hands in a fake. */
export interface Limits { create: Limiter; write: Limiter }

/** The Worker's variables as the shell uses them (spec §3.1). */
export interface ShellConfig {
  /** `YOKOHAMA_WEB_IDENTITY` (M6.5c2, spec §2.1): Cloudflare Access verified
   *  by this shell, or the open door. No default. */
  door: Door;
  /** `YOKOHAMA_IDENTITY_SOURCE` (auth cycle, design §3): who vouches for an address under the `worker` door,
   *  with no default. `null` under `open`, where a name is a claim and no source vouches for it. */
  source: Source | null;
  /** The sign-in half of `oidc` and `workos`; `null` under `access` and `open`, where the variables, if set,
   *  are ignored. `issuer` is `null` under `workos` (its issuer is fixed and not configurable) and
   *  `clientSecret` is `null` under `oidc` when none is set (the PKCE flow); under `workos` it is the API key. */
  signin: { issuer: string | null; clientId: string; clientSecret: string | null } | null;
  /** The session cookie's signing secret (at least 32 bytes) and its lifetime in days, 30 when
   *  `YOKOHAMA_SESSION_DAYS` is absent; `null` under `access` and `open`. */
  session: { secret: string; days: number } | null;
  archive: "r2" | "off";
  /** `null`: no table ever expires. */
  ttlDays: number | null;
  /** How many days a table still WAITING for players — its status `open` —
   *  stands without activity (R-6.5c2.42); 7 when the variable is absent.
   *  Inert while `ttlDays` is `null`: then no table expires. */
  openTtlDays: number;
  /** `null`: the operator routes are the layer's 404 (spec §2.9). Always
   *  `null` under `open`, where a list is refused (`openDoorRefusals`). */
  admins: string[] | null;
  countWakes: boolean;
  devDoor: boolean;
  diagnosticSeeds: boolean;
  clock: Clock;
  /** `CREATE_LIMIT` and `WRITE_LIMIT` (security S2b, C2), both required. */
  limits: Limits;
}

/** Every variable `Env` declares: its string-typed keys, and none of its
 *  bindings. */
type Variable = { [K in keyof Env]-?: NonNullable<Env[K]> extends string ? K : never }[keyof Env];

/**
 * Every variable `Env` declares, in its order — written as a record over
 * `Variable` so the compiler holds the two to ONE list (task 7 review, Minor
 * 3): a variable `Env` gains without a row here fails to compile, naming it,
 * and so does a row `Env` does not declare. A variable left out would skip the
 * string check (`strings`, below) without a word. Every one is a variable the
 * Worker reads but `YOKOHAMA_TURNSTILE` (cycle F, R-FP2), the deploy
 * declaration: held to a string here, like the rest, and read by nothing else
 * in the Worker — the deploy guard and the smoke read it.
 */
const ROWS: { readonly [K in Variable]: true } = {
  YOKOHAMA_WEB_IDENTITY: true,
  YOKOHAMA_IDENTITY_SOURCE: true,
  YOKOHAMA_OIDC_ISSUER: true,
  YOKOHAMA_OIDC_CLIENT_ID: true,
  YOKOHAMA_OIDC_CLIENT_SECRET: true,
  YOKOHAMA_SESSION_SECRET: true,
  YOKOHAMA_SESSION_DAYS: true,
  ACCESS_TEAM_DOMAIN: true,
  ACCESS_AUD: true,
  DEV_DOOR_CLIENT_ID: true,
  YOKOHAMA_DEV_DOOR: true,
  YOKOHAMA_DIAGNOSTIC_SEEDS: true,
  YOKOHAMA_WEB_ARCHIVE: true,
  YOKOHAMA_WEB_TABLE_TTL_DAYS: true,
  YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: true,
  YOKOHAMA_WEB_ADMINS: true,
  YOKOHAMA_WEB_COUNT_WAKES: true,
  YOKOHAMA_WEB_LOBBY_CHAT: true,
  YOKOHAMA_WEB_CHAT_KEEP: true,
  YOKOHAMA_WEB_CHAT_TEXT: true,
  YOKOHAMA_TURNSTILE: true,
  YOKOHAMA_WEB_TEST_CLOCK: true,
};
const VARIABLES = Object.keys(ROWS) as Variable[];

/** A refusal on its way to `shellConfig`'s `{ error }`. */
class Refusal {
  constructor(readonly sentence: string) {}
}

function refuse(sentence: string): never {
  throw new Refusal(sentence);
}

/** A variable's text, or `null` when it is not set. Read only after
 *  `strings` has held every variable to a string. */
function text(env: Env, name: Variable): string | null {
  return env[name] ?? null;
}

/**
 * Every variable is a string or absent (the orchestrator's ruling E3). A
 * wrangler `vars` entry written without quotes arrives as a number or a
 * boolean: the module's own parse would refuse one naming no variable, and
 * `Env` says none can arrive.
 */
function strings(env: Env): void {
  for (const name of VARIABLES) {
    const value: unknown = env[name];
    if (value !== undefined && typeof value !== "string") {
      const described = value === null ? "null" : `the ${typeof value} ${JSON.stringify(value)}`;
      refuse(`${name} must be a string, in quotes — got ${described}`);
    }
  }
}

/** Trimmed and ASCII-lowercased: how the layer reads its chat switches
 *  (`ChatConfig::from_env_values`), so `OFF` means off here as it does there
 *  (the orchestrator's ruling E2). */
function folded(raw: string): string {
  return raw.trim().replace(/[A-Z]/g, (c) => c.toLowerCase());
}

/**
 * `worker` | `open`, with NO default (M6.5c2, spec §2.1; R-6.5c2.5).
 *
 * **Trimmed, then matched EXACTLY — never case-folded**, unlike the on/off
 * switches (`folded`): the layer's own parser reads this same variable on the
 * self-host (`IdentityMode::from_str`, `match s.trim()`), and one variable
 * reads one rule on both hosts, so `Open` or `WORKER`, which the NAS refuses,
 * is refused here too (the orchestrator's ruling R-C11). `cf-access` is the
 * NAS's door, not this host's: here the shell verifies Access itself, which is
 * `worker`.
 */
function door(raw: string | null): Door {
  const value = raw === null ? null : raw.trim();
  if (value === "worker" || value === "open") return value;
  return refuse(
    value === null
      ? "YOKOHAMA_WEB_IDENTITY must be worker or open — it is not set, and there is no default: a door is a deployment decision"
      : `YOKOHAMA_WEB_IDENTITY must be worker or open — got ${JSON.stringify(value)}`,
  );
}

/**
 * `access` | `oidc` | `workos`, with NO default (the auth cycle, design §3): who vouches for an address is a
 * deployment decision, as the door is. Trimmed, then matched EXACTLY, as the door is. **Under `open` it is not read
 * here**: a source set beside the open door is refused by `openDoorRefusals`, after the older pairs, so the
 * sentence that names the operator list or the dev door is still the one a configuration holding both gets.
 */
function source(door: Door, raw: string | null): Source | null {
  if (door === "open") return null;
  const value = raw === null ? null : raw.trim();
  if (value === "access" || value === "oidc" || value === "workos") return value;
  return refuse(
    value === null
      ? "YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos under the worker door — it is not set, and there is no default: who vouches for an address is a deployment decision"
      : `YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos — got ${JSON.stringify(value)}`,
  );
}

/** A variable's trimmed text, or `null` when it is not set or only whitespace. */
function present(env: Env, name: Variable): string | null {
  const value = text(env, name)?.trim();
  return value === undefined || value === "" ? null : value;
}

/**
 * The sign-in variables, read for `oidc` and `workos` alone: under `access` they are ignored when set, never
 * refused. The client id is required under both; the issuer under `oidc` only, and REFUSED beside `workos`, whose
 * issuer is fixed; the client secret is the WorkOS API key under `workos` (required) and optional under `oidc`.
 */
function signin(env: Env, src: Source | null): ShellConfig["signin"] {
  if (src !== "oidc" && src !== "workos") return null;
  const issuer = present(env, "YOKOHAMA_OIDC_ISSUER");
  if (src === "oidc" && issuer === null) {
    refuse(
      "YOKOHAMA_OIDC_ISSUER must be set under YOKOHAMA_IDENTITY_SOURCE=oidc — the issuer whose /.well-known/openid-configuration names the endpoints",
    );
  }
  if (src === "workos" && issuer !== null) {
    refuse(
      "YOKOHAMA_OIDC_ISSUER is set beside YOKOHAMA_IDENTITY_SOURCE=workos — the WorkOS arm's issuer is https://api.workos.com and is not configurable",
    );
  }
  const clientId = present(env, "YOKOHAMA_OIDC_CLIENT_ID");
  if (clientId === null) {
    refuse(
      `YOKOHAMA_OIDC_CLIENT_ID must be set under YOKOHAMA_IDENTITY_SOURCE=${src} — the client id the issuer registered for this site`,
    );
  }
  const clientSecret = present(env, "YOKOHAMA_OIDC_CLIENT_SECRET");
  if (src === "workos" && clientSecret === null) {
    refuse(
      "YOKOHAMA_OIDC_CLIENT_SECRET must be set under YOKOHAMA_IDENTITY_SOURCE=workos — the WorkOS API key the code exchange is authenticated with (wrangler secret put YOKOHAMA_OIDC_CLIENT_SECRET)",
    );
  }
  return { issuer, clientId, clientSecret };
}

/** A session cookie's lifetime when `YOKOHAMA_SESSION_DAYS` is absent. */
const SESSION_DAYS = 30;

/** The signing secret's floor, in UTF-8 bytes (counted as `isAddress` counts). */
const SESSION_SECRET_BYTES = 32;

/**
 * The session variables, read for `oidc` and `workos` alone. The secret is taken as it is set (never trimmed) and
 * counted in UTF-8 bytes; the days are a whole number above zero written as such, 30 when absent.
 */
function session(env: Env, src: Source | null): ShellConfig["session"] {
  if (src !== "oidc" && src !== "workos") return null;
  const secret = text(env, "YOKOHAMA_SESSION_SECRET") ?? "";
  if (new TextEncoder().encode(secret).length < SESSION_SECRET_BYTES) {
    refuse(
      "YOKOHAMA_SESSION_SECRET must be at least 32 bytes — it signs every session cookie (wrangler secret put YOKOHAMA_SESSION_SECRET)",
    );
  }
  const raw = text(env, "YOKOHAMA_SESSION_DAYS");
  if (raw === null) return { secret, days: SESSION_DAYS };
  const days = Number(raw);
  if (!/^[1-9][0-9]*$/.test(raw) || !Number.isSafeInteger(days)) {
    refuse(`YOKOHAMA_SESSION_DAYS must be a whole number of days above zero — got ${JSON.stringify(raw)}`);
  }
  return { secret, days };
}

/** `r2` | `off`, absent = off (R-6.5c.14). The module matches these two
 *  strings exactly, and `moduleConfig` hands it this value. */
function archive(env: Env): "r2" | "off" {
  const raw = text(env, "YOKOHAMA_WEB_ARCHIVE");
  const value = raw === null ? "off" : folded(raw);
  if (value === "off") return "off";
  if (value !== "r2") refuse(`YOKOHAMA_WEB_ARCHIVE must be r2 or off — got ${JSON.stringify(value)}`);
  if (env.ARCHIVE === undefined) refuse("YOKOHAMA_WEB_ARCHIVE=r2 needs the ARCHIVE bucket binding");
  return "r2";
}

/** A waiting table's TTL when its variable is absent (R-6.5c2.42). */
const OPEN_TTL_DAYS = 7;

/** `raw`, trimmed, as whole days above zero — or the refusal naming `name`.
 *  The two TTLs' one rule. */
function wholeDays(name: "YOKOHAMA_WEB_TABLE_TTL_DAYS" | "YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS", raw: string): number {
  const value = raw.trim();
  const days = Number(value);
  if (!/^\d+$/.test(value) || days === 0 || !Number.isSafeInteger(days)) {
    refuse(`${name} must be a whole number of days above zero — got ${JSON.stringify(value)}`);
  }
  return days;
}

/** Whole days above zero, absent = no table ever expires (R-6.5c.15). */
function ttlDays(raw: string | null): number | null {
  return raw === null ? null : wholeDays("YOKOHAMA_WEB_TABLE_TTL_DAYS", raw);
}

/** Whole days above zero, absent = 7 (R-6.5c2.42), refused as the first TTL
 *  is. Read whether or not the first is set, so a bad value is refused
 *  before it could ever be in force. */
function openTtlDays(raw: string | null): number {
  return raw === null ? OPEN_TTL_DAYS : wholeDays("YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS", raw);
}

/**
 * The operators (spec §2.9): comma-separated addresses, each trimmed and
 * lowercased as the layer's `User::parse` reads a caller, so an entry compares
 * with a caller's address as that caller would be read. An empty entry is
 * refused, and so is an entry that is no address by the layer's email shape
 * (`isAddress`, the mode rule's `User::is_email`): under `worker` the layer
 * admits no other caller, so the entry could match no one, and the operator
 * would meet a 403 that names nothing wrong. The
 * first empty entry is named before the first that is no address.
 *
 * **A refusal names the entry by its POSITION and never echoes a value** (R3's
 * M-6): the list is the operator's address, which the plan's ruling P-1 keeps
 * out of the repository, and a refusal is the 500 body of every `/api/*`
 * request and the `init-refused` line.
 */
function admins(raw: string | null): string[] | null {
  if (raw === null) return null;
  const entries = raw.split(",").map(folded);
  const position = (i: number) => `entry ${i + 1} of ${entries.length}`;
  const empty = entries.indexOf("");
  if (empty !== -1) refuse(`YOKOHAMA_WEB_ADMINS ${position(empty)} is empty`);
  const bad = entries.findIndex((entry) => !isAddress(entry));
  if (bad !== -1) {
    refuse(
      `YOKOHAMA_WEB_ADMINS ${position(bad)} is not an address — ` +
        "one @ with something on both sides, no spaces or control characters, and at most 254 bytes",
    );
  }
  return entries;
}

/** `on` | `off`, absent = off. */
function onOff(
  name: "YOKOHAMA_WEB_COUNT_WAKES" | "YOKOHAMA_DEV_DOOR" | "YOKOHAMA_DIAGNOSTIC_SEEDS",
  raw: string | null,
): boolean {
  const value = raw === null ? "off" : folded(raw);
  if (value !== "on" && value !== "off") refuse(`${name} must be on or off — got ${JSON.stringify(value)}`);
  return value === "on";
}

/** TEST ONLY (the plan's ruling P-2): `frozen:<seconds>` or `tick:<seconds>`,
 *  absent = the wall clock. */
function clock(raw: string | null): Clock {
  if (raw === null) return { kind: "wall" };
  const value = folded(raw);
  const m = /^(frozen|tick):(\d+)$/.exec(value);
  const seconds = m ? Number(m[2]) : NaN;
  if (!m || !Number.isSafeInteger(seconds)) {
    refuse(`YOKOHAMA_WEB_TEST_CLOCK must be frozen:<seconds> or tick:<seconds> — got ${JSON.stringify(value)}`);
  }
  return m[1] === "frozen" ? { kind: "frozen", at: seconds } : { kind: "tick", next: seconds };
}

/** The two bindings, both required (security S2b, C2): a configuration that lacks one is refused by name, the
 *  Worker's form of the NAS refusing to start, so no deployment runs unthrottled by omission. */
function limits(env: Env): Limits {
  for (const name of ["CREATE_LIMIT", "WRITE_LIMIT"] as const) {
    if (env[name] === undefined) refuse(`the ${name} rate-limit binding is missing — every deployable configuration binds it in its ratelimits block (security S2b)`);
  }
  return { create: env.CREATE_LIMIT!, write: env.WRITE_LIMIT! };
}

/**
 * What the OPEN door refuses beside it (M6.5c2, spec §2.1; review R2.2-M1),
 * each refusal naming BOTH variables — the Worker's form of refusing to start
 * — and the operator list named before the dev door:
 *
 * - an operator list: a claimed name is never an operator's rights. The list
 *   is a SECRET, which the deploy guard cannot see (it reads the committed
 *   configurations), so it is refused here, at run time, and the refusal
 *   echoes nothing of its value (R3's M-6);
 * - the dev door on: it is the `worker` door's test instrument, and under
 *   `open` a claim needs no credential. The deploy guard refuses this pair in
 *   the committed `vars` too;
 * - an identity source (auth cycle, design §3), last: under `open` a name is a
 *   claim and no source vouches for it.
 *
 * `DEV_DOOR_CLIENT_ID` is ignored under `open`, never refused: a credential
 * means nothing where a claim needs none. Under `worker` nothing here is
 * refused.
 */
function openDoorRefusals(c: ShellConfig, env: Env): void {
  if (c.door !== "open") return;
  if (c.admins !== null) {
    refuse(
      "YOKOHAMA_WEB_ADMINS is set beside YOKOHAMA_WEB_IDENTITY=open — a claimed name is never an operator: " +
        "delete the operator list (wrangler secret delete YOKOHAMA_WEB_ADMINS), or deploy the worker door",
    );
  }
  if (c.devDoor) {
    refuse(
      "YOKOHAMA_DEV_DOOR=on beside YOKOHAMA_WEB_IDENTITY=open — the dev door is the worker door's test instrument, " +
        "and under open a claim needs no credential: set YOKOHAMA_DEV_DOOR to off, or deploy the worker door",
    );
  }
  // Last, so a configuration holding an older pair as well still names that pair first. Presence, not value:
  // whatever the source says, no source vouches for a claim.
  if (text(env, "YOKOHAMA_IDENTITY_SOURCE") !== null) {
    refuse(
      "YOKOHAMA_IDENTITY_SOURCE is set beside YOKOHAMA_WEB_IDENTITY=open — under the open door a name is a claim and no source vouches for it",
    );
  }
}

/**
 * The Worker's configuration, or the sentence that refuses it — naming the
 * variable, the Worker's form of the NAS refusing to start. A variable that is
 * not a string is refused before any value is read; after that, the first
 * field refused, in the order `ShellConfig` lists them — the door first
 * (M6.5c2) — is the one named; then, once every field reads, the pairs the
 * open door refuses (`openDoorRefusals`).
 *
 * The three chat variables are held to strings here and parsed by the module
 * (`init`), with the layer's own function; nothing else here reads them.
 */
export function shellConfig(env: Env): ShellConfig | { error: string } {
  try {
    strings(env);
    // The door and the source are read before the object literal so `signin` and `session` can see the source;
    // the literal below still lists the fields in `ShellConfig`'s order, and the first refusal named is the door's.
    const d = door(text(env, "YOKOHAMA_WEB_IDENTITY"));
    const src = source(d, text(env, "YOKOHAMA_IDENTITY_SOURCE"));
    const config: ShellConfig = {
      door: d,
      source: src,
      signin: signin(env, src),
      session: session(env, src),
      archive: archive(env),
      ttlDays: ttlDays(text(env, "YOKOHAMA_WEB_TABLE_TTL_DAYS")),
      openTtlDays: openTtlDays(text(env, "YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS")),
      admins: admins(text(env, "YOKOHAMA_WEB_ADMINS")),
      countWakes: onOff("YOKOHAMA_WEB_COUNT_WAKES", text(env, "YOKOHAMA_WEB_COUNT_WAKES")),
      devDoor: onOff("YOKOHAMA_DEV_DOOR", text(env, "YOKOHAMA_DEV_DOOR")),
      diagnosticSeeds: onOff("YOKOHAMA_DIAGNOSTIC_SEEDS", text(env, "YOKOHAMA_DIAGNOSTIC_SEEDS")),
      clock: clock(text(env, "YOKOHAMA_WEB_TEST_CLOCK")),
      limits: limits(env),
    };
    openDoorRefusals(config, env);
    return config;
  } catch (e) {
    if (e instanceof Refusal) return { error: e.sentence };
    throw e;
  }
}

/**
 * The module's `init` configuration (the plan's names table): the door as
 * exactly `worker` or `open` (M6.5c2, spec §2.1), the archive
 * switch as exactly `off` or `r2`, `caller` verbatim (the module normalizes
 * it, under `r2`, as the layer normalizes an address), the three chat
 * variables RAW, for the module to parse with the layer's own function — and
 * the two TTLs as NUMBERS, the shell's own reading through its readers
 * (`ttlDaysOf`, `openTtlDaysOf`), for `/api/health` to report (R-6.5c2.42):
 * both `null` while no table expires, the second being inert then — and
 * `diagnostic_seeds` as a BOOLEAN, the shell's reading of
 * `YOKOHAMA_DIAGNOSTIC_SEEDS` (R-SEC8(b)): `true` in the pool's two test
 * files alone, which the deploy guard keeps out of a deployable one.
 *
 * It throws `shellConfig`'s refusal rather than build a configuration the
 * shell refuses: the module cannot see a binding, and would parse a number
 * without naming its variable. Every caller has asked `shellConfig` first.
 */
export function moduleConfig(env: Env, caller: string | null): string {
  const c = shellConfig(env);
  if ("error" in c) throw new Error(c.error);
  const ttl = ttlDaysOf(c);
  return JSON.stringify({
    identity: c.door,
    archive: c.archive,
    caller,
    lobby_chat: text(env, "YOKOHAMA_WEB_LOBBY_CHAT"),
    chat_keep: text(env, "YOKOHAMA_WEB_CHAT_KEEP"),
    chat_text: text(env, "YOKOHAMA_WEB_CHAT_TEXT"),
    table_ttl_days: ttl,
    open_table_ttl_days: ttl === null ? null : openTtlDaysOf(c),
    diagnostic_seeds: c.diagnosticSeeds,
  });
}
