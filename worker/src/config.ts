import type { Env } from "./env";
import { isAddress } from "./identity";
import { openTtlDaysOf, ttlDaysOf } from "./marker";

/** Where the shell reads the time (the plan's ruling P-2): the wall clock, a
 *  frozen reading, or a reading that ticks one second per use. */
export type Clock = { kind: "wall" } | { kind: "frozen"; at: number } | { kind: "tick"; next: number };

/** The Worker's door (M6.5c2, spec §2.1; R-6.5c2.5). */
export type Door = "worker" | "open";

/** The Worker's variables as the shell uses them (spec §3.1). */
export interface ShellConfig {
  /** `YOKOHAMA_WEB_IDENTITY` (M6.5c2, spec §2.1): Cloudflare Access verified
   *  by this shell, or the open door. No default. */
  door: Door;
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
  clock: Clock;
}

/** Every variable `Env` declares: its string-typed keys, and none of its
 *  bindings. */
type Variable = { [K in keyof Env]-?: NonNullable<Env[K]> extends string ? K : never }[keyof Env];

/**
 * Every variable the Worker reads, in `Env`'s order — written as a record over
 * `Variable` so the compiler holds the two to ONE list (task 7 review, Minor
 * 3): a variable `Env` gains without a row here fails to compile, naming it,
 * and so does a row `Env` does not declare. A variable left out would skip the
 * string check (`strings`, below) without a word.
 */
const ROWS: { readonly [K in Variable]: true } = {
  YOKOHAMA_WEB_IDENTITY: true,
  ACCESS_TEAM_DOMAIN: true,
  ACCESS_AUD: true,
  DEV_DOOR_CLIENT_ID: true,
  YOKOHAMA_DEV_DOOR: true,
  YOKOHAMA_WEB_ARCHIVE: true,
  YOKOHAMA_WEB_TABLE_TTL_DAYS: true,
  YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: true,
  YOKOHAMA_WEB_ADMINS: true,
  YOKOHAMA_WEB_COUNT_WAKES: true,
  YOKOHAMA_WEB_LOBBY_CHAT: true,
  YOKOHAMA_WEB_CHAT_KEEP: true,
  YOKOHAMA_WEB_CHAT_TEXT: true,
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
function onOff(name: "YOKOHAMA_WEB_COUNT_WAKES" | "YOKOHAMA_DEV_DOOR", raw: string | null): boolean {
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
 *   the committed `vars` too.
 *
 * `DEV_DOOR_CLIENT_ID` is ignored under `open`, never refused: a credential
 * means nothing where a claim needs none. Under `worker` nothing here is
 * refused.
 */
function openDoorRefusals(c: ShellConfig): void {
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
    const config: ShellConfig = {
      door: door(text(env, "YOKOHAMA_WEB_IDENTITY")),
      archive: archive(env),
      ttlDays: ttlDays(text(env, "YOKOHAMA_WEB_TABLE_TTL_DAYS")),
      openTtlDays: openTtlDays(text(env, "YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS")),
      admins: admins(text(env, "YOKOHAMA_WEB_ADMINS")),
      countWakes: onOff("YOKOHAMA_WEB_COUNT_WAKES", text(env, "YOKOHAMA_WEB_COUNT_WAKES")),
      devDoor: onOff("YOKOHAMA_DEV_DOOR", text(env, "YOKOHAMA_DEV_DOOR")),
      clock: clock(text(env, "YOKOHAMA_WEB_TEST_CLOCK")),
    };
    openDoorRefusals(config);
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
 * both `null` while no table expires, the second being inert then.
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
  });
}
