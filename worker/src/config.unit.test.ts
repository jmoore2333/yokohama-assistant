import { describe, expect, it, vi } from "vitest";
import { isDenied, moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";

// `shellConfig` asks only WHETHER a bucket is bound, so an empty object stands
// in for one; node has no R2.
const BUCKET = {} as R2Bucket;

// The two Rate Limiting bindings (security S2b, C2), which `shellConfig`
// requires: TWO objects, so the uncrossed cell below can see a crossing — one
// shared fake could not. The `RateLimit` annotation is what keeps `env`'s
// `as Env` cast from TS2352.
const FAKE_CREATE: RateLimit = { limit: async () => ({ success: true }) };
const FAKE_WRITE: RateLimit = { limit: async () => ({ success: true }) };
const LIMITS = { CREATE_LIMIT: FAKE_CREATE, WRITE_LIMIT: FAKE_WRITE };

/** An `Env` with `vars` set, the two rate-limit bindings bound and, when
 *  asked, the bucket bound — at the WORKER door unless `vars` names another,
 *  or `undefined` for none (M6.5c2): the door has no default, and every other
 *  cell here is about another variable. `vars` may unbind a binding by naming
 *  it `undefined`. The variables are `unknown` here because a wrangler `vars`
 *  entry can be a number or a boolean, and those are cells too. */
function env(vars: Record<string, unknown> = {}, bucket = false): Env {
  const e: Record<string, unknown> = { ASSETS: {} as Fetcher, ...LIMITS, ...(bucket ? { ARCHIVE: BUCKET } : {}), YOKOHAMA_WEB_IDENTITY: "worker", ...vars };
  // The identity source (auth cycle, Task 1) has no default either, and the worker door requires it: every cell
  // here that is about another variable gets `access`, today's shape, unless `vars` names the variable — `undefined`
  // included, which unbinds it — or the door is `open`, which refuses a source beside it.
  if (!("YOKOHAMA_IDENTITY_SOURCE" in vars) && typeof e.YOKOHAMA_WEB_IDENTITY === "string" && e.YOKOHAMA_WEB_IDENTITY.trim() !== "open") {
    e.YOKOHAMA_IDENTITY_SOURCE = "access";
  }
  return e as unknown as Env;
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

// R-6.5c2.42: a table that has not started — open, or dealing with a keep
// outstanding (security S3, D9) — expires after these days, default 7,
// refused as the first TTL is. With no first TTL it is inert — no
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
  // Security S1a (B3; R-SEC8(b)): the seed switch, on in the pool's two files.
  ["YOKOHAMA_DIAGNOSTIC_SEEDS", "diagnosticSeeds"],
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

// The auth cycle, Task 1 (design §3): who vouches for an address is a deployment decision, so the source has NO
// default under the worker door, and the sign-in and session variables are read only for the sources that use them.
// Mutants: a default source; the source ignored (every source read as access); either reader run under access; the
// open door's refusal missing, or made whatever the door; a secret's length counted in characters; a days value
// that is not a whole number let through; the source read before the door.
describe("YOKOHAMA_IDENTITY_SOURCE (auth cycle, design §3)", () => {
  const OIDC = {
    ACCESS_TEAM_DOMAIN: undefined,
    ACCESS_AUD: undefined,
    YOKOHAMA_IDENTITY_SOURCE: "oidc",
    YOKOHAMA_OIDC_ISSUER: "https://issuer.test",
    YOKOHAMA_OIDC_CLIENT_ID: "client_1",
    YOKOHAMA_SESSION_SECRET: "s".repeat(32),
  };
  const WORKOS = {
    ACCESS_TEAM_DOMAIN: undefined,
    ACCESS_AUD: undefined,
    YOKOHAMA_IDENTITY_SOURCE: "workos",
    YOKOHAMA_OIDC_CLIENT_ID: "client_1",
    YOKOHAMA_OIDC_CLIENT_SECRET: "sk_test_x",
    YOKOHAMA_SESSION_SECRET: "s".repeat(32),
  };

  it("the worker door requires a source, with no default", () => {
    expect(refused(env({ YOKOHAMA_IDENTITY_SOURCE: undefined }, true))).toBe(
      "YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos under the worker door — it is not set, and there is no default: who vouches for an address is a deployment decision",
    );
  });
  it("access keeps today's shape", () => {
    expect(accepted(env({ YOKOHAMA_IDENTITY_SOURCE: "access" }, true))).toMatchObject({ source: "access", signin: null, session: null });
  });
  it("access ignores the sign-in and session variables when they happen to be set", () => {
    expect(
      accepted(env({ YOKOHAMA_IDENTITY_SOURCE: "access", YOKOHAMA_OIDC_ISSUER: "https://issuer.test", YOKOHAMA_OIDC_CLIENT_ID: "c", YOKOHAMA_SESSION_SECRET: "short", YOKOHAMA_SESSION_DAYS: "0" }, true)),
    ).toMatchObject({ source: "access", signin: null, session: null });
  });
  it("oidc needs the issuer, the client id and the session secret, and ignores the Access pair", () => {
    expect(accepted(env(OIDC, true))).toMatchObject({
      source: "oidc",
      signin: { issuer: "https://issuer.test", clientId: "client_1", clientSecret: null },
      session: { secret: "s".repeat(32), days: 30 },
    });
    // A SET Access pair is ignored, never refused: a self-hoster may leave it.
    expect(accepted(env({ ...OIDC, ACCESS_TEAM_DOMAIN: "t.cloudflareaccess.com", ACCESS_AUD: "aud" }, true))).toMatchObject({ source: "oidc" });
    expect(refused(env({ ...OIDC, YOKOHAMA_OIDC_ISSUER: undefined }, true))).toBe(
      "YOKOHAMA_OIDC_ISSUER must be set under YOKOHAMA_IDENTITY_SOURCE=oidc — the issuer whose /.well-known/openid-configuration names the endpoints",
    );
    expect(refused(env({ ...OIDC, YOKOHAMA_OIDC_CLIENT_ID: undefined }, true))).toBe(
      "YOKOHAMA_OIDC_CLIENT_ID must be set under YOKOHAMA_IDENTITY_SOURCE=oidc — the client id the issuer registered for this site",
    );
    expect(refused(env({ ...OIDC, YOKOHAMA_SESSION_SECRET: "short" }, true))).toBe(
      "YOKOHAMA_SESSION_SECRET must be at least 32 bytes — it signs every session cookie (wrangler secret put YOKOHAMA_SESSION_SECRET)",
    );
    expect(refused(env({ ...OIDC, YOKOHAMA_SESSION_SECRET: undefined }, true))).toBe(
      "YOKOHAMA_SESSION_SECRET must be at least 32 bytes — it signs every session cookie (wrangler secret put YOKOHAMA_SESSION_SECRET)",
    );
    expect(refused(env({ ...OIDC, YOKOHAMA_SESSION_DAYS: "0" }, true))).toBe(
      'YOKOHAMA_SESSION_DAYS must be a whole number of days above zero — got "0"',
    );
  });
  it("oidc passes a client secret through when one is set, and does not need one", () => {
    expect(accepted(env({ ...OIDC, YOKOHAMA_OIDC_CLIENT_SECRET: "shh" }, true))).toMatchObject({
      signin: { issuer: "https://issuer.test", clientId: "client_1", clientSecret: "shh" },
    });
  });
  it("workos needs the client id and the API key as the secret, and refuses an issuer", () => {
    expect(accepted(env(WORKOS, true))).toMatchObject({
      source: "workos",
      signin: { issuer: null, clientId: "client_1", clientSecret: "sk_test_x" },
      session: { secret: "s".repeat(32), days: 30 },
    });
    expect(refused(env({ ...WORKOS, YOKOHAMA_OIDC_CLIENT_SECRET: undefined }, true))).toBe(
      "YOKOHAMA_OIDC_CLIENT_SECRET must be set under YOKOHAMA_IDENTITY_SOURCE=workos — the WorkOS API key the code exchange is authenticated with (wrangler secret put YOKOHAMA_OIDC_CLIENT_SECRET)",
    );
    expect(refused(env({ ...WORKOS, YOKOHAMA_OIDC_CLIENT_ID: undefined }, true))).toBe(
      "YOKOHAMA_OIDC_CLIENT_ID must be set under YOKOHAMA_IDENTITY_SOURCE=workos — the client id the issuer registered for this site",
    );
    expect(refused(env({ ...WORKOS, YOKOHAMA_OIDC_ISSUER: "https://x" }, true))).toBe(
      "YOKOHAMA_OIDC_ISSUER is set beside YOKOHAMA_IDENTITY_SOURCE=workos — the WorkOS arm's issuer is https://api.workos.com and is not configurable",
    );
    expect(refused(env({ ...WORKOS, YOKOHAMA_SESSION_SECRET: "short" }, true))).toMatch(/^YOKOHAMA_SESSION_SECRET must be at least 32 bytes/);
  });
  // `present`: trimmed, and a blank is NOT a value. Mutants: the trim dropped; a blank kept as "".
  it("reads the client id trimmed and refuses a blank one", () => {
    expect(accepted(env({ ...OIDC, YOKOHAMA_OIDC_CLIENT_ID: " client_1 " }, true)).signin?.clientId).toBe("client_1");
    expect(refused(env({ ...OIDC, YOKOHAMA_OIDC_CLIENT_ID: "  " }, true))).toBe(
      "YOKOHAMA_OIDC_CLIENT_ID must be set under YOKOHAMA_IDENTITY_SOURCE=oidc — the client id the issuer registered for this site",
    );
  });
  it("refuses a blank WorkOS API key, and reads a blank issuer beside workos as none", () => {
    expect(refused(env({ ...WORKOS, YOKOHAMA_OIDC_CLIENT_SECRET: "" }, true))).toMatch(
      /^YOKOHAMA_OIDC_CLIENT_SECRET must be set under YOKOHAMA_IDENTITY_SOURCE=workos — the WorkOS API key/,
    );
    expect(accepted(env({ ...WORKOS, YOKOHAMA_OIDC_ISSUER: "  " }, true)).signin?.issuer).toBeNull();
  });
  it("open ignores an absent source and refuses a present one", () => {
    expect(accepted(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_IDENTITY_SOURCE: undefined }, true))).toMatchObject({ source: null, signin: null, session: null });
    expect(refused(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_IDENTITY_SOURCE: "access" }, true))).toBe(
      "YOKOHAMA_IDENTITY_SOURCE is set beside YOKOHAMA_WEB_IDENTITY=open — under the open door a name is a claim and no source vouches for it",
    );
    // Whatever it says: a present source is refused beside open, a bad value included.
    expect(refused(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_IDENTITY_SOURCE: "oidc" }, true))).toMatch(/^YOKOHAMA_IDENTITY_SOURCE is set beside YOKOHAMA_WEB_IDENTITY=open/);
    expect(refused(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_IDENTITY_SOURCE: "bogus" }, true))).toMatch(/^YOKOHAMA_IDENTITY_SOURCE is set beside YOKOHAMA_WEB_IDENTITY=open/);
  });
  it("open ignores the sign-in and session variables, as it ignores the Access pair", () => {
    expect(
      accepted(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_IDENTITY_SOURCE: undefined, YOKOHAMA_OIDC_CLIENT_ID: "c", YOKOHAMA_SESSION_SECRET: "short" }, true)),
    ).toMatchObject({ source: null, signin: null, session: null });
  });
  it("a source that is none of the three is refused by value", () => {
    expect(refused(env({ YOKOHAMA_IDENTITY_SOURCE: "Access" }, true))).toBe(
      'YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos — got "Access"',
    );
    expect(refused(env({ YOKOHAMA_IDENTITY_SOURCE: "" }, true))).toBe('YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos — got ""');
  });
  it("is read trimmed, as the door is", () => {
    expect(accepted(env({ YOKOHAMA_IDENTITY_SOURCE: " access " }, true))).toMatchObject({ source: "access" });
  });
});

describe("YOKOHAMA_SESSION_SECRET and YOKOHAMA_SESSION_DAYS (auth cycle, design §3)", () => {
  const OIDC = {
    YOKOHAMA_IDENTITY_SOURCE: "oidc",
    YOKOHAMA_OIDC_ISSUER: "https://issuer.test",
    YOKOHAMA_OIDC_CLIENT_ID: "client_1",
    YOKOHAMA_SESSION_SECRET: "s".repeat(32),
  };
  // Mutant: the length counted in characters (`.length`), not UTF-8 bytes.
  it("counts the secret in UTF-8 bytes: sixteen two-byte characters pass, fifteen do not", () => {
    expect(accepted(env({ ...OIDC, YOKOHAMA_SESSION_SECRET: "\u00e9".repeat(16) }, true)).session?.secret).toBe("\u00e9".repeat(16));
    expect(refused(env({ ...OIDC, YOKOHAMA_SESSION_SECRET: "\u00e9".repeat(15) }, true))).toMatch(/^YOKOHAMA_SESSION_SECRET must be at least 32 bytes/);
    expect(refused(env({ ...OIDC, YOKOHAMA_SESSION_SECRET: "s".repeat(31) }, true))).toMatch(/^YOKOHAMA_SESSION_SECRET must be at least 32 bytes/);
  });
  it("reads the days as a whole number above zero, 30 when absent", () => {
    expect(accepted(env(OIDC, true)).session?.days).toBe(30);
    expect(accepted(env({ ...OIDC, YOKOHAMA_SESSION_DAYS: "7" }, true)).session?.days).toBe(7);
    expect(accepted(env({ ...OIDC, YOKOHAMA_SESSION_DAYS: "365" }, true)).session?.days).toBe(365);
  });
  // The TTLs' own rule (`wholeDays`): trimmed, digits only, so a padded or zero-led number reads as the number.
  it.each(["007", " 7", "7 "])("reads the days %j as 7, as the two TTLs read theirs", (value) => {
    expect(accepted(env({ ...OIDC, YOKOHAMA_SESSION_DAYS: value }, true)).session?.days).toBe(7);
  });
  it.each(["0", "-1", "2.5", "thirty", "", "1e3", "99999999999999999999"])("refuses the days %j, quoting it", (value) => {
    expect(refused(env({ ...OIDC, YOKOHAMA_SESSION_DAYS: value }, true))).toBe(
      `YOKOHAMA_SESSION_DAYS must be a whole number of days above zero — got ${JSON.stringify(value)}`,
    );
  });
});

// The absolute cap and the red button (2026-10-19, the owner's rulings after stage 1's read). Mutants: the cap
// defaulted to the days or to none; the cap read under access; the deny-list read under access; an entry not
// lowercased or not trimmed; a newline not a separator; an empty entry refused, or a malformed one let through; a
// refusal echoing the value.
describe("YOKOHAMA_SESSION_MAX_DAYS and YOKOHAMA_DENIED (the absolute cap and the red button)", () => {
  const WORKOS = {
    ACCESS_TEAM_DOMAIN: undefined,
    ACCESS_AUD: undefined,
    YOKOHAMA_IDENTITY_SOURCE: "workos",
    YOKOHAMA_OIDC_CLIENT_ID: "client_1",
    YOKOHAMA_OIDC_CLIENT_SECRET: "sk_test_x",
    YOKOHAMA_SESSION_SECRET: "s".repeat(32),
  };
  it("reads the cap as whole days above zero, 90 when absent, under a sign-in arm alone", () => {
    expect(accepted(env(WORKOS, true)).session?.maxDays).toBe(90);
    expect(accepted(env({ ...WORKOS, YOKOHAMA_SESSION_MAX_DAYS: " 007 " }, true)).session?.maxDays).toBe(7);
    expect(accepted(env({ ...WORKOS, YOKOHAMA_IDENTITY_SOURCE: "access", YOKOHAMA_SESSION_MAX_DAYS: "0" }, true)).session).toBeNull();
  });
  it.each(["0", "-1", "2.5", "ninety", ""])("refuses the cap %j, quoting it", (value) => {
    expect(refused(env({ ...WORKOS, YOKOHAMA_SESSION_MAX_DAYS: value }, true))).toBe(
      `YOKOHAMA_SESSION_MAX_DAYS must be a whole number of days above zero — got ${JSON.stringify(value)}`,
    );
  });
  it("reads the deny-list as addresses and @domains, split on commas and newlines, trimmed and lowercased, empties dropped; null when absent or under access", () => {
    expect(accepted(env(WORKOS, true)).denied).toBeNull();
    const d = accepted(env({ ...WORKOS, YOKOHAMA_DENIED: " Spam@T.test ,\n@Bad.Example\n\n,c@d.test,\n" }, true)).denied;
    expect(d).toEqual({ addresses: ["spam@t.test", "c@d.test"], domains: ["bad.example"] });
    expect(accepted(env({ ...WORKOS, YOKOHAMA_DENIED: "\n, ,\n" }, true)).denied).toEqual({ addresses: [], domains: [] });
    // Newlines ALONE separate (review I-1): §24's recipe writes the secret one entry per line, with no comma anywhere.
    expect(accepted(env({ ...WORKOS, YOKOHAMA_DENIED: "a@b.test\nc@d.test\n@x.example\n" }, true)).denied).toEqual({
      addresses: ["a@b.test", "c@d.test"],
      domains: ["x.example"],
    });
    expect(accepted(env({ ...WORKOS, YOKOHAMA_IDENTITY_SOURCE: "access", YOKOHAMA_DENIED: "not an address" }, true)).denied).toBeNull();
  });
  // The owner's ruling (2026-10-19, on a side agent's note): a typo in the ban list must never be an outage. So an
  // entry that is neither shape is SKIPPED with a log line naming its position among the entries kept — never its
  // value — and the rest stand; `YOKOHAMA_WEB_ADMINS` keeps its refusal. Mutants: the refusal back; the value logged;
  // the entry kept; the rest dropped with it.
  it("skips an entry that is neither an address nor an @domain, logging its position and never its value, and keeps the rest", () => {
    const spy = vi.spyOn(console, "log").mockImplementation(() => {});
    try {
      const d = accepted(env({ ...WORKOS, YOKOHAMA_DENIED: "a@b.test,\n\nnot an address, @ok.example" }, true)).denied;
      expect(d).toEqual({ addresses: ["a@b.test"], domains: ["ok.example"] });
      expect(spy.mock.calls.map(([line]) => JSON.parse(String(line)))).toEqual([{ event: "denied-skipped", position: 2, of: 3 }]);
      expect(JSON.stringify(spy.mock.calls)).not.toContain("not an address");
      for (const bad of ["@", "@ spaced.example", "two@at@signs.test", "@a@b"]) {
        spy.mockClear();
        expect(accepted(env({ ...WORKOS, YOKOHAMA_DENIED: bad }, true)).denied, bad).toEqual({ addresses: [], domains: [] });
        expect(spy.mock.calls.map(([line]) => JSON.parse(String(line))), bad).toEqual([{ event: "denied-skipped", position: 1, of: 1 }]);
      }
    } finally {
      spy.mockRestore();
    }
  });
  // The match's SHAPE (review I-3): the whole address, folded, among the addresses; the part after its last `@`, folded,
  // EXACTLY among the domains — a subdomain is not its parent's (list it as its own entry), and a domain is never
  // matched as a suffix of a longer one. Mutants: no folding; the domain by `endsWith`; a domain entry matched against
  // the whole address; `null` denying.
  it("isDenied matches the folded address exactly, or the folded domain after the last @ exactly, and never a suffix or a subdomain", () => {
    const list = { addresses: ["spam@t.test"], domains: ["bad.example"] };
    for (const denied of ["spam@t.test", "SPAM@T.test", " spam@t.test ", "x@bad.example", "X@Bad.Example", "a@b@bad.example"]) {
      expect(isDenied(list, denied), denied).toBe(true);
    }
    for (const allowed of ["evil@notbad.example", "spam@t.test.example", "red@t.test", "bad.example@t.test", "a@mail.bad.example", "a@bad.example.evil", "spam@t.tes"]) {
      expect(isDenied(list, allowed), allowed).toBe(false);
    }
    expect(isDenied(null, "spam@t.test")).toBe(false);
    expect(isDenied({ addresses: [], domains: [] }, "spam@t.test")).toBe(false);
  });
});

describe("which of two refusals is named, with the source (auth cycle, Task 1)", () => {
  it("the door before the source", () => {
    expect(refused(env({ YOKOHAMA_WEB_IDENTITY: undefined, YOKOHAMA_IDENTITY_SOURCE: "bogus" }, true))).toMatch(/^YOKOHAMA_WEB_IDENTITY must be worker or open/);
  });
  it("the source before the sign-in variables, and those before the session's", () => {
    expect(refused(env({ YOKOHAMA_IDENTITY_SOURCE: "bogus", YOKOHAMA_SESSION_SECRET: "short" }, true))).toMatch(/^YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos — got/);
    expect(refused(env({ YOKOHAMA_IDENTITY_SOURCE: "oidc", YOKOHAMA_SESSION_SECRET: "short" }, true))).toMatch(/^YOKOHAMA_OIDC_ISSUER must be set/);
    expect(
      refused(env({ YOKOHAMA_IDENTITY_SOURCE: "oidc", YOKOHAMA_OIDC_ISSUER: "https://i", YOKOHAMA_OIDC_CLIENT_ID: "c", YOKOHAMA_SESSION_SECRET: "short", YOKOHAMA_SESSION_DAYS: "0" }, true)),
    ).toMatch(/^YOKOHAMA_SESSION_SECRET must be at least 32 bytes/);
  });
  it("the open door's older refusals before a source set beside it", () => {
    expect(refused(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_IDENTITY_SOURCE: "access", YOKOHAMA_WEB_ADMINS: "boss@t.test" }, true))).toMatch(/^YOKOHAMA_WEB_ADMINS is set beside/);
    expect(refused(env({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_IDENTITY_SOURCE: "access", YOKOHAMA_DEV_DOOR: "on" }, true))).toMatch(/^YOKOHAMA_DEV_DOOR=on beside/);
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
    // The auth cycle's keys (Task 1 review, M-3): the realistic slip is the days written without quotes.
    ["YOKOHAMA_SESSION_DAYS", 30, "the number 30"],
    ["YOKOHAMA_IDENTITY_SOURCE", true, "the boolean true"],
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

// Security S2b, finding C2 (decision D-3): the two Rate Limiting bindings are
// both required — a configuration lacking either is refused by name, the
// Worker's form of the NAS refusing to start, so no deployment runs
// unthrottled by omission (fail closed, as `ARCHIVE` under `r2`). Mutants:
// `limits()` not refusing an absent binding; the two bindings handed through
// crossed.
describe("the two Rate Limiting bindings (security S2b, C2)", () => {
  it("refuses a configuration lacking WRITE_LIMIT, naming it", () => {
    expect(refused(env({ WRITE_LIMIT: undefined }))).toBe(
      "the WRITE_LIMIT rate-limit binding is missing — every deployable configuration binds it in its ratelimits block (security S2b)",
    );
  });
  it("names CREATE_LIMIT first when both are missing", () => {
    expect(refused(env({ CREATE_LIMIT: undefined, WRITE_LIMIT: undefined }))).toBe(
      "the CREATE_LIMIT rate-limit binding is missing — every deployable configuration binds it in its ratelimits block (security S2b)",
    );
  });
  it("hands both bindings through by identity, uncrossed", () => {
    const c = accepted(env());
    expect(c.limits.create).toBe(FAKE_CREATE);
    expect(c.limits.write).toBe(FAKE_WRITE);
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
