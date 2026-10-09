import { describe, expect, it } from "vitest";
import { moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";

// `shellConfig` asks only WHETHER a bucket is bound, so an empty object stands
// in for one; node has no R2.
const BUCKET = {} as R2Bucket;

/** An `Env` with `vars` set and, when asked, the bucket bound — at the WORKER
 *  door unless `vars` names another, or `undefined` for none (M6.5c2): the
 *  door has no default, and every other cell here is about another variable.
 *  The variables are `unknown` here because a wrangler `vars` entry can be a
 *  number or a boolean, and those are cells too. */
function env(vars: Record<string, unknown> = {}, bucket = false): Env {
  return { ASSETS: {} as Fetcher, ...(bucket ? { ARCHIVE: BUCKET } : {}), YOKOHAMA_WEB_IDENTITY: "worker", ...vars } as Env;
}

/** The configuration, or the cell fails with the refusal it got instead. */
function accepted(e: Env): ShellConfig {
  const c = shellConfig(e);
  if ("error" in c) throw new Error(`refused: ${c.error}`);
  return c;
}

/** The refusal's sentence, or the cell fails. */
function refused(e: Env): string {
  const c = shellConfig(e);
  if (!("error" in c)) throw new Error(`accepted: ${JSON.stringify(c)}`);
  return c.error;
}

describe("YOKOHAMA_WEB_ARCHIVE (spec §3.1, R-6.5c.14)", () => {
  it("is off when absent", () => {
    expect(accepted(env()).archive).toBe("off");
  });
  it("is r2 with the ARCHIVE bucket bound", () => {
    expect(accepted(env({ YOKOHAMA_WEB_ARCHIVE: "r2" }, true)).archive).toBe("r2");
  });
  it("refuses r2 without the bucket", () => {
    expect(refused(env({ YOKOHAMA_WEB_ARCHIVE: "r2" }))).toBe(
      "YOKOHAMA_WEB_ARCHIVE=r2 needs the ARCHIVE bucket binding",
    );
  });
  it("refuses any other value, naming it", () => {
    expect(refused(env({ YOKOHAMA_WEB_ARCHIVE: "s3" }))).toBe('YOKOHAMA_WEB_ARCHIVE must be r2 or off — got "s3"');
  });
  // The orchestrator's ruling E2: trimmed and ASCII-lowercased, as the layer
  // reads its chat switches, so `OFF` means off here as it does there.
  it("reads the switch as the layer reads its own: trimmed, in any case", () => {
    expect(accepted(env({ YOKOHAMA_WEB_ARCHIVE: " OFF " })).archive).toBe("off");
    expect(accepted(env({ YOKOHAMA_WEB_ARCHIVE: "R2" }, true)).archive).toBe("r2");
    expect(refused(env({ YOKOHAMA_WEB_ARCHIVE: " S3 " }))).toBe('YOKOHAMA_WEB_ARCHIVE must be r2 or off — got "s3"');
  });
});

describe("YOKOHAMA_WEB_TABLE_TTL_DAYS (R-6.5c.15)", () => {
  it("is null when absent: no table expires", () => {
    expect(accepted(env()).ttlDays).toBeNull();
  });
  it("is a whole number of days", () => {
    expect(accepted(env({ YOKOHAMA_WEB_TABLE_TTL_DAYS: "30" })).ttlDays).toBe(30);
    expect(accepted(env({ YOKOHAMA_WEB_TABLE_TTL_DAYS: " 90 " })).ttlDays).toBe(90);
  });
  it.each(["0", "-1", "2.5", "thirty", "", "1e3"])("refuses %j, naming the variable", (value) => {
    expect(refused(env({ YOKOHAMA_WEB_TABLE_TTL_DAYS: value }))).toMatch(
      /^YOKOHAMA_WEB_TABLE_TTL_DAYS must be a whole number of days above zero — got /,
    );
  });
});

// R-6.5c2.42: a table still WAITING for players expires after these days,
// default 7, refused as the first TTL is. With no first TTL it is inert — no
// table expires — which the module's configuration says (below). Mutants: no
// default; a default other than 7; the refusal naming the first TTL.
describe("YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS (R-6.5c2.42)", () => {
  it("is 7 when absent", () => {
    expect(accepted(env()).openTtlDays).toBe(7);
  });
  it("is a whole number of days", () => {
    expect(accepted(env({ YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: "3" })).openTtlDays).toBe(3);
    expect(accepted(env({ YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: " 14 " })).openTtlDays).toBe(14);
  });
  it.each(["0", "-1", "2.5", "seven", "", "1e3"])("refuses %j, naming the variable", (value) => {
    expect(refused(env({ YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: value }))).toBe(
      `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS must be a whole number of days above zero — got ${JSON.stringify(value)}`,
    );
  });
});

describe("YOKOHAMA_WEB_ADMINS (spec §2.9)", () => {
  it("is null when absent: the operator routes stay the layer's 404", () => {
    expect(accepted(env()).admins).toBeNull();
  });
  it("is the comma-separated addresses, each trimmed and lowercased as the layer reads a caller", () => {
    expect(accepted(env({ YOKOHAMA_WEB_ADMINS: " A@b.com, c@d.com " })).admins).toEqual(["a@b.com", "c@d.com"]);
  });
  // R3's M-6: the list is the operator's ADDRESS, and a refusal is the 500
  // body of every `/api/*` request, so it names the variable and the entry's
  // POSITION and never echoes a value. Mutant: the value, or an entry of it,
  // echoed.
  it.each<[string, string]>([
    ["a@b.com,,c@d.com", "entry 2 of 3"],
    ["a@b.com,", "entry 2 of 2"],
    ["", "entry 1 of 1"],
  ])("refuses an empty entry in %j, naming its position and echoing nothing", (value, position) => {
    const sentence = refused(env({ YOKOHAMA_WEB_ADMINS: value }));
    expect(sentence).toBe(`YOKOHAMA_WEB_ADMINS ${position} is empty`);
    for (const entry of value.split(",").filter((e) => e !== "")) expect(sentence).not.toContain(entry);
  });
  // An entry no caller could ever be would lock the operator out with a 403
  // that names nothing wrong; refused here instead, by its position.
  it.each<[string, string]>([
    ["operator", "entry 1 of 1"],
    ["a@b.com c@d.com", "entry 1 of 1"],
    ["a@b@c.com", "entry 1 of 1"],
    ["@b.com", "entry 1 of 1"],
    ["boss@t.test, not-an-address", "entry 2 of 2"],
    [`boss@t.test,${"a".repeat(248)}@t.test`, "entry 2 of 2"],
  ])("refuses %j, which holds no address, naming its position and echoing nothing", (value, position) => {
    const sentence = refused(env({ YOKOHAMA_WEB_ADMINS: value }));
    expect(sentence).toBe(
      `YOKOHAMA_WEB_ADMINS ${position} is not an address — one @ with something on both sides, ` +
        "no spaces or control characters, and at most 254 bytes",
    );
    for (const entry of value.split(",")) expect(sentence).not.toContain(entry.trim());
  });
});

describe.each([
  ["YOKOHAMA_WEB_COUNT_WAKES", "countWakes"],
  ["YOKOHAMA_DEV_DOOR", "devDoor"],
] as const)("%s", (name, field) => {
  it("is off when absent", () => {
    expect(accepted(env())[field]).toBe(false);
  });
  it("is on or off, read as the layer reads a switch", () => {
    expect(accepted(env({ [name]: "on" }))[field]).toBe(true);
    expect(accepted(env({ [name]: "off" }))[field]).toBe(false);
    expect(accepted(env({ [name]: " ON " }))[field]).toBe(true);
  });
  it("refuses anything else, naming the variable", () => {
    expect(refused(env({ [name]: "yes" }))).toBe(`${name} must be on or off — got "yes"`);
  });
});

describe("YOKOHAMA_WEB_TEST_CLOCK (the plan's ruling P-2)", () => {
  it("is the wall clock when absent", () => {
    expect(accepted(env()).clock).toEqual({ kind: "wall" });
  });
  it("freezes at a reading, or ticks from one", () => {
    expect(accepted(env({ YOKOHAMA_WEB_TEST_CLOCK: "frozen:1789000000" })).clock).toEqual({
      kind: "frozen",
      at: 1_789_000_000,
    });
    expect(accepted(env({ YOKOHAMA_WEB_TEST_CLOCK: "tick:1789000000" })).clock).toEqual({
      kind: "tick",
      next: 1_789_000_000,
    });
  });
  it.each(["frozen:x", "frozen:", "tick:1.5", "later:5", "1789000000"])("refuses %j, naming the variable", (value) => {
    expect(refused(env({ YOKOHAMA_WEB_TEST_CLOCK: value }))).toMatch(
      /^YOKOHAMA_WEB_TEST_CLOCK must be frozen:<seconds> or tick:<seconds> — got /,
    );
  });
});

// M6.5c2, spec §2.1 (R-6.5c2.5): the door has NO default, and this Worker has
// two. Read as the layer reads the NAS's variable of this name
// (`IdentityMode::from_str`): trimmed, then matched exactly — `Open` is not
// `open`, and `cf-access` and `claimed` are not this host's doors: here the
// shell verifies Access itself, which is `worker`.
describe("YOKOHAMA_WEB_IDENTITY (M6.5c2, spec §2.1)", () => {
  // Mutants: a default door; the absence read as `worker`.
  it("refuses a Worker that names no door, saying there is no default", () => {
    expect(refused(env({ YOKOHAMA_WEB_IDENTITY: undefined }))).toBe(
      "YOKOHAMA_WEB_IDENTITY must be worker or open — it is not set, and there is no default: a door is a deployment decision",
    );
  });
  it("is worker or open, trimmed", () => {
    expect(accepted(env()).door).toBe("worker");
    expect(accepted(env({ YOKOHAMA_WEB_IDENTITY: "open" })).door).toBe("open");
    expect(accepted(env({ YOKOHAMA_WEB_IDENTITY: " open " })).door).toBe("open");
  });
  // Mutants: any string read as a door; the value lowercased.
  it.each(["cf-access", "claimed", "Open", "WORKER", ""])("refuses %j, naming it", (value) => {
    expect(refused(env({ YOKOHAMA_WEB_IDENTITY: value }))).toBe(
      `YOKOHAMA_WEB_IDENTITY must be worker or open — got ${JSON.stringify(value)}`,
    );
  });
});

// M6.5c2, spec §2.1 (review R2.2-M1): what the OPEN door refuses beside it,
// at run time, naming BOTH variables — the Worker's form of refusing to start.
// An operator list is a SECRET, which the deploy guard cannot see, so this is
// where it is refused; the dev door on is refused here and by the guard. The
// dev door's client id is IGNORED under open, never refused: a credential
// means nothing where a claim needs none. Mutants: either refusal missing;
// either made whatever the door; the operator list's value echoed.
describe("what the open door refuses beside it (M6.5c2, spec §2.1)", () => {
  const ADMINS_BESIDE_OPEN =
    "YOKOHAMA_WEB_ADMINS is set beside YOKOHAMA_WEB_IDENTITY=open — a claimed name is never an operator: " +
    "delete the operator list (wrangler secret delete YOKOHAMA_WEB_ADMINS), or deploy the worker door";
  const DEV_DOOR_BESIDE_OPEN =
    "YOKOHAMA_DEV_DOOR=on beside YOKOHAMA_WEB_IDENTITY=open — the dev door is the worker door's test instrument, " +
    "and under open a claim needs no credential: set YOKOHAMA_DEV_DOOR to off, or deploy the worker door";
  it("refuses an operator list, naming both variables and echoing nothing", () => {
    const sentence = refused(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_WEB_ADMINS: "boss@t.test" }));
    expect(sentence).toBe(ADMINS_BESIDE_OPEN);
    expect(sentence).not.toContain("boss@t.test");
  });
  it("refuses the dev door on, read as every switch is read", () => {
    expect(refused(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_DEV_DOOR: " ON " }))).toBe(DEV_DOOR_BESIDE_OPEN);
  });
  it("keeps both under the worker door, and ignores the dev door off and its client id under open", () => {
    expect(accepted(env({ YOKOHAMA_WEB_ADMINS: "boss@t.test", YOKOHAMA_DEV_DOOR: "on" }))).toMatchObject({
      door: "worker",
      admins: ["boss@t.test"],
      devDoor: true,
    });
    expect(
      accepted(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_DEV_DOOR: "off", DEV_DOOR_CLIENT_ID: "some-client.access" })),
    ).toMatchObject({
      door: "open",
      admins: null,
      devDoor: false,
    });
  });
  // Two defects at once (the lane's M6.2 lesson): the list is named first,
  // and its value is echoed by neither sentence.
  it("names the operator list before the dev door", () => {
    const sentence = refused(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_WEB_ADMINS: "boss@t.test", YOKOHAMA_DEV_DOOR: "on" }));
    expect(sentence).toBe(ADMINS_BESIDE_OPEN);
    expect(sentence).not.toContain("boss@t.test");
  });
});

// The orchestrator's ruling E3: every variable is a string or absent. A
// wrangler `vars` entry written without quotes arrives as a number or a
// boolean — which the module's own parse would refuse naming no variable, and
// which `Env` says cannot happen — so the shell refuses every one, by name.
describe("a variable that is not a string", () => {
  it.each<[string, unknown, string]>([
    ["YOKOHAMA_WEB_TABLE_TTL_DAYS", 90, "the number 90"],
    ["YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS", 7, "the number 7"],
    ["YOKOHAMA_DEV_DOOR", true, "the boolean true"],
    ["YOKOHAMA_WEB_CHAT_KEEP", 500, "the number 500"],
    ["ACCESS_AUD", null, "null"],
    ["YOKOHAMA_WEB_IDENTITY", true, "the boolean true"],
    // Cycle F (R-FP2): the deploy declaration is held to a string like every
    // variable `Env` declares — the one thing the shell does with it.
    ["YOKOHAMA_TURNSTILE", true, "the boolean true"],
  ])("refuses %s given %j", (name, value, described) => {
    expect(refused(env({ [name]: value }))).toBe(`${name} must be a string, in quotes — got ${described}`);
  });
});

// Cycle F (R-FP2; spec §2.6): `YOKOHAMA_TURNSTILE` is a deploy DECLARATION
// in the configuration's `vars`, read by the deploy guard (check 12) and the
// post-deploy smoke, never by the Worker. `Env` declares it and `ROWS` holds
// it to a string (above), and that is all: `on`, `off` and absent leave the
// shell's configuration and the module's nine keys exactly as they were.
// Mutants: a field of `ShellConfig` read from it; a tenth key in the module's
// configuration.
describe("YOKOHAMA_TURNSTILE (cycle F, R-FP2): a deploy declaration the shell never reads", () => {
  const doors: Record<string, unknown>[] = [
    {},
    { YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_WEB_TABLE_TTL_DAYS: "90", YOKOHAMA_WEB_LOBBY_CHAT: "off" },
  ];
  it.each(doors)("leaves shellConfig and moduleConfig unchanged under %j", (vars) => {
    const without = accepted(env(vars, true));
    const module = moduleConfig(env(vars, true), "Red@T.test");
    expect(Object.keys(JSON.parse(module) as object)).toHaveLength(9);
    for (const declared of ["on", "off", undefined, " ON ", "anything"]) {
      const e = env({ ...vars, YOKOHAMA_TURNSTILE: declared }, true);
      expect(accepted(e), String(declared)).toEqual(without);
      expect(moduleConfig(e, "Red@T.test"), String(declared)).toBe(module);
    }
  });
  // `Env` declares it, as the string `ROWS` holds it to: a compile-time pin
  // (`npm run typecheck`) that the key is `Env`'s and optional.
  it("is declared in Env as an optional string", () => {
    const declared: Env["YOKOHAMA_TURNSTILE"] = "on";
    const absent: Env["YOKOHAMA_TURNSTILE"] = undefined;
    expect([declared, absent]).toEqual(["on", undefined]);
  });
});

// The refusal ORDER, pinned with two defects at once: a single-defect cell pins
// nothing about which of two refusals is named (the lane's M6.2 lesson; task 7
// review, Minor 2).
describe("which of two refusals is named", () => {
  it("a variable that is not a string, before any value is read", () => {
    expect(refused(env({ YOKOHAMA_WEB_ARCHIVE: "s3", YOKOHAMA_WEB_TABLE_TTL_DAYS: 90 }))).toBe(
      "YOKOHAMA_WEB_TABLE_TTL_DAYS must be a string, in quotes — got the number 90",
    );
  });
  it("then the first field in ShellConfig's order: the archive switch before the TTL", () => {
    expect(refused(env({ YOKOHAMA_WEB_ARCHIVE: "s3", YOKOHAMA_WEB_TABLE_TTL_DAYS: "0" }))).toBe(
      'YOKOHAMA_WEB_ARCHIVE must be r2 or off — got "s3"',
    );
  });
  // R-6.5c2.42: the waiting-table TTL follows the TTL and comes before the
  // operator list, in ShellConfig's order. Mutants: either pair read the other
  // way round.
  it("the TTL before the waiting-table TTL, and that before the operator list", () => {
    expect(refused(env({ YOKOHAMA_WEB_TABLE_TTL_DAYS: "0", YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: "0" }))).toBe(
      'YOKOHAMA_WEB_TABLE_TTL_DAYS must be a whole number of days above zero — got "0"',
    );
    expect(refused(env({ YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: "0", YOKOHAMA_WEB_ADMINS: "" }))).toBe(
      'YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS must be a whole number of days above zero — got "0"',
    );
  });
  // M6.5c2: the door is ShellConfig's FIRST field. Mutant: the door read after
  // the archive switch.
  it("the door before the archive switch", () => {
    expect(refused(env({ YOKOHAMA_WEB_IDENTITY: undefined, YOKOHAMA_WEB_ARCHIVE: "s3" }))).toMatch(
      /^YOKOHAMA_WEB_IDENTITY must be worker or open/,
    );
  });
});

describe("the module's init configuration (the plan's names table)", () => {
  it("carries the archive switch as exactly off or r2, and the rest raw", () => {
    const e = env(
      {
        YOKOHAMA_WEB_ARCHIVE: " R2 ",
        YOKOHAMA_WEB_LOBBY_CHAT: " Off ",
        YOKOHAMA_WEB_CHAT_KEEP: " All ",
        YOKOHAMA_WEB_CHAT_TEXT: " 250 ",
      },
      true,
    );
    expect(moduleConfig(e, "Red@T.test")).toBe(
      '{"identity":"worker","archive":"r2","caller":"Red@T.test","lobby_chat":" Off ","chat_keep":" All ","chat_text":" 250 ",' +
        '"table_ttl_days":null,"open_table_ttl_days":null,"diagnostic_seeds":false}',
    );
  });
  it("is off with no caller, no chat variable and no TTL set", () => {
    expect(moduleConfig(env(), null)).toBe(
      '{"identity":"worker","archive":"off","caller":null,"lobby_chat":null,"chat_keep":null,"chat_text":null,' +
        '"table_ttl_days":null,"open_table_ttl_days":null,"diagnostic_seeds":false}',
    );
  });
  // R-6.5c2.42: the two TTLs as the shell read them, for `/api/health` to
  // report — the waiting-table TTL at its default — and BOTH null with no
  // first TTL, where the second is inert: no table expires, so none expires
  // for waiting. Mutants: the second reported while inert; its default not
  // carried; the TTLs sent raw.
  it("carries the two TTLs as the shell read them, and neither while no table expires", () => {
    const ttls = (vars: Record<string, unknown>) => {
      const c = JSON.parse(moduleConfig(env(vars), null)) as Record<string, unknown>;
      return [c.table_ttl_days, c.open_table_ttl_days];
    };
    expect(ttls({ YOKOHAMA_WEB_TABLE_TTL_DAYS: " 90 " })).toEqual([90, 7]);
    expect(ttls({ YOKOHAMA_WEB_TABLE_TTL_DAYS: "30", YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: "3" })).toEqual([30, 3]);
    expect(ttls({ YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS: "3" }), "inert without the first").toEqual([null, null]);
  });
  // The door is carried as the shell read it. Mutant: the raw value sent.
  it("carries the door as the shell read it: trimmed", () => {
    expect(JSON.parse(moduleConfig(env({ YOKOHAMA_WEB_IDENTITY: " open " }), null)).identity).toBe("open");
  });
  // The module's own refusals name their variables, but it cannot see a
  // binding and would parse a number without naming one: a configuration the
  // shell refuses is never handed to it.
  it("throws the shell's refusal rather than build a configuration it refuses", () => {
    expect(() => moduleConfig(env({ YOKOHAMA_WEB_ARCHIVE: "r2" }), null)).toThrow(
      "YOKOHAMA_WEB_ARCHIVE=r2 needs the ARCHIVE bucket binding",
    );
    expect(() => moduleConfig(env({ YOKOHAMA_WEB_CHAT_TEXT: 500 }), null)).toThrow(
      "YOKOHAMA_WEB_CHAT_TEXT must be a string, in quotes — got the number 500",
    );
  });
});
