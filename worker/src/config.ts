import type { Env } from "./env";
import { isAddress } from "./identity";

/** Where the shell reads the time (the plan's ruling P-2): the wall clock, a
 *  frozen reading, or a reading that ticks one second per use. */
export type Clock = { kind: "wall" } | { kind: "frozen"; at: number } | { kind: "tick"; next: number };

/** The Worker's variables as the shell uses them (spec §3.1). */
export interface ShellConfig {
  archive: "r2" | "off";
  /** `null`: no table ever expires. */
  ttlDays: number | null;
  /** `null`: the operator routes are the layer's 404 (spec §2.9). */
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
  ACCESS_TEAM_DOMAIN: true,
  ACCESS_AUD: true,
  DEV_DOOR_CLIENT_ID: true,
  YOKOHAMA_DEV_DOOR: true,
  YOKOHAMA_WEB_ARCHIVE: true,
  YOKOHAMA_WEB_TABLE_TTL_DAYS: true,
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

/** Whole days above zero, absent = no table ever expires (R-6.5c.15). */
function ttlDays(raw: string | null): number | null {
  if (raw === null) return null;
  const value = raw.trim();
  const days = Number(value);
  if (!/^\d+$/.test(value) || days === 0 || !Number.isSafeInteger(days)) {
    refuse(`YOKOHAMA_WEB_TABLE_TTL_DAYS must be a whole number of days above zero — got ${JSON.stringify(value)}`);
  }
  return days;
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
 * The Worker's configuration, or the sentence that refuses it — naming the
 * variable, the Worker's form of the NAS refusing to start. A variable that is
 * not a string is refused before any value is read; after that, the first
 * field refused, in the order `ShellConfig` lists them, is the one named.
 *
 * The three chat variables are held to strings here and parsed by the module
 * (`init`), with the layer's own function; nothing else here reads them.
 */
export function shellConfig(env: Env): ShellConfig | { error: string } {
  try {
    strings(env);
    return {
      archive: archive(env),
      ttlDays: ttlDays(text(env, "YOKOHAMA_WEB_TABLE_TTL_DAYS")),
      admins: admins(text(env, "YOKOHAMA_WEB_ADMINS")),
      countWakes: onOff("YOKOHAMA_WEB_COUNT_WAKES", text(env, "YOKOHAMA_WEB_COUNT_WAKES")),
      devDoor: onOff("YOKOHAMA_DEV_DOOR", text(env, "YOKOHAMA_DEV_DOOR")),
      clock: clock(text(env, "YOKOHAMA_WEB_TEST_CLOCK")),
    };
  } catch (e) {
    if (e instanceof Refusal) return { error: e.sentence };
    throw e;
  }
}

/**
 * The module's `init` configuration (the plan's names table): the archive
 * switch as exactly `off` or `r2`, `caller` verbatim (the module normalizes
 * it, under `r2`, as the layer normalizes an address), and the three chat
 * variables RAW, for the module to parse with the layer's own function.
 *
 * It throws `shellConfig`'s refusal rather than build a configuration the
 * shell refuses: the module cannot see a binding, and would parse a number
 * without naming its variable. Every caller has asked `shellConfig` first.
 */
export function moduleConfig(env: Env, caller: string | null): string {
  const c = shellConfig(env);
  if ("error" in c) throw new Error(c.error);
  return JSON.stringify({
    archive: c.archive,
    caller,
    lobby_chat: text(env, "YOKOHAMA_WEB_LOBBY_CHAT"),
    chat_keep: text(env, "YOKOHAMA_WEB_CHAT_KEEP"),
    chat_text: text(env, "YOKOHAMA_WEB_CHAT_TEXT"),
  });
}
