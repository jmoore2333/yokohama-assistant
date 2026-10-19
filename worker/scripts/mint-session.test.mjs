import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { COOKIE_NAME, cookieLine, fenceSentence, isTestAddress, mintValue } from "./mint-session.mjs";

// **The minted `.test` session's cells** (auth cycle, Task 8). The script re-implements `src/session.ts`'s
// `mintSession` with `node:crypto`, since plain `node` cannot import TypeScript; this file is where vitest, which
// transpiles, imports BOTH and holds them byte-identical — and holds the script to `session.unit.test.ts`'s openssl
// golden, a third witness that is neither implementation. The fence cells run the script as a process, since an exit
// code and a stream are what an operator reads.

const SCRIPT = fileURLToPath(new URL("./mint-session.mjs", import.meta.url));
const SECRET = "0123456789abcdef0123456789abcdef";
/** `session.unit.test.ts`'s `GOLDEN_SESSION`, copied verbatim: the bytes openssl computes for `red@t.test`, sid null,
 *  issued 1789000000, 30 days, under `SECRET`. */
const GOLDEN_SESSION =
  "v1.eyJhZGRyZXNzIjoicmVkQHQudGVzdCIsInNpZCI6bnVsbCwiaXNzdWVkIjoxNzg5MDAwMDAwLCJleHBpcmVzIjoxNzkxNTkyMDAwfQ" +
  ".C7Evl68QvNoLo7oHRHSQ0zUgg7untTb2bxOrXqFIsSo";

/** The script as a process, with exactly the environment a cell names. */
function mint(args, env = { YOKOHAMA_SESSION_SECRET: SECRET }) {
  const r = spawnSync(process.execPath, [SCRIPT, ...args], { env: { PATH: process.env.PATH, ...env }, encoding: "utf8" });
  return { code: r.status, out: r.stdout, err: r.stderr };
}

/** The payload a cookie value carries, parsed. */
const payloadOf = (value) => JSON.parse(Buffer.from(value.split(".")[1], "base64url").toString("utf8"));

describe("the cookie's bytes", () => {
  it("is exactly the bytes openssl computed for the golden payload", () => {
    expect(mintValue({ address: "red@t.test", sid: null, issued: 1_789_000_000 }, 30, SECRET)).toBe(GOLDEN_SESSION);
  });

  it("is the session module's own, byte for byte, across addresses, sids, issue times and days", async () => {
    const { mintSession } = await import("../src/session.ts");
    const cases = [
      [{ address: "red@t.test", sid: null, issued: 1_789_000_000 }, 30],
      [{ address: "a@e2e.test", sid: null, issued: 1_790_123_456 }, 1],
      [{ address: "ünï@çode.test", sid: null, issued: 1_790_000_001 }, 365],
      // The script writes sid null itself; the function underneath takes any, as the module does.
      [{ address: "b@b.test", sid: "s-1", issued: 1_789_000_000 }, 7],
    ];
    for (const [s, days] of cases) {
      expect(mintValue(s, days, SECRET), `${s.address} ${days}d`).toBe(await mintSession(s, days, SECRET));
    }
    // Another secret is another mac and the same payload.
    expect(mintValue(cases[0][0], 30, "another-secret-value-0123456789ab")).not.toBe(GOLDEN_SESSION);
    expect(mintValue(cases[0][0], 30, "another-secret-value-0123456789ab").split(".")[1]).toBe(GOLDEN_SESSION.split(".")[1]);
  });

  it("writes the url alphabet with no padding, the key order the module writes, and sid as null", () => {
    // A payload whose standard base64 would pad and carry + or /.
    const v = mintValue({ address: "???>>>@pad.test", sid: null, issued: 1_789_000_000 }, 30, SECRET);
    expect(v).toMatch(/^v1\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]{43}$/);
    expect(Object.keys(payloadOf(v))).toEqual(["address", "sid", "issued", "expires"]);
    expect(payloadOf(v).sid).toBeNull();
    expect(payloadOf(v).expires).toBe(1_789_000_000 + 30 * 86_400);
  });

  it("is signed over 'v1.' and the payload, so the mac is not the payload's alone", async () => {
    const { createHmac } = await import("node:crypto");
    const [, payload, mac] = GOLDEN_SESSION.split(".");
    expect(mac).toBe(createHmac("sha256", SECRET).update(`v1.${payload}`).digest("base64url"));
    expect(mac).not.toBe(createHmac("sha256", SECRET).update(payload).digest("base64url"));
  });
});

describe("the .test fence", () => {
  it("restates the identity module's isTestAddress exactly", async () => {
    const { isTestAddress: theirs } = await import("../src/identity.ts");
    for (const a of ["red@t.test", "a.test", "a@b.com", "a@b.test.com", "@.test", "a@test", "A@B.TEST", "a@b.test ", "", "x@y@z.test", "me@gmail.com"]) {
      expect(isTestAddress(a), JSON.stringify(a)).toBe(theirs(a));
    }
  });

  it("refuses a real address with exit 2, the brief's sentence, nothing on stdout, and no cookie", () => {
    const r = mint(["jamoore@gmail.com"]);
    expect(r.code).toBe(2);
    expect(r.err).toBe("mint-session: only a .test address may be minted — got jamoore@gmail.com\n");
    expect(r.out).toBe("");
  });

  // **The fence judges the raw argument** (review I-3): a fence judged after `trim()` or `toLowerCase()` while the raw
  // value is minted would let `RED@T.TEST` out of the `test/` root, since the Worker's `is_test_identity` is
  // case-sensitive. Both call sites — the script and `cookieLine`, which the replay and the live tier call — are held.
  it("refuses an address that is a .test one only once trimmed or lower-cased, as the process and as cookieLine", () => {
    for (const a of ["RED@T.TEST", "red@t.test ", "red@t.test\n", "Red@T.Test"]) {
      const r = mint([a]);
      expect(r.code, JSON.stringify(a)).toBe(2);
      expect(r.out, JSON.stringify(a)).toBe("");
      expect(r.err, JSON.stringify(a)).toContain("only a .test address may be minted");
      expect(() => cookieLine(a, SECRET, 1_789_000_000), JSON.stringify(a)).toThrow("mint-session: only a .test address may be minted");
    }
    expect(() => cookieLine("red@t.test", SECRET, 1_789_000_000)).not.toThrow();
  });

  it("cookieLine is the printed line's own function: the name, '=', and the golden value", () => {
    expect(cookieLine("red@t.test", SECRET, 1_789_000_000)).toBe(`${COOKIE_NAME}=${GOLDEN_SESSION}`);
  });

  it("refuses a time that is not whole seconds and days that are not whole or below one, as mintSession does", () => {
    expect(() => cookieLine("red@t.test", SECRET, 1_789_000_000_000.5)).toThrow("issued must be whole seconds");
    expect(() => cookieLine("red@t.test", SECRET, Number.NaN)).toThrow("issued must be whole seconds");
    for (const days of [0, -1, 1.5, Number.NaN]) {
      expect(() => cookieLine("red@t.test", SECRET, 1_789_000_000, days), String(days)).toThrow("days must be whole and at least one");
    }
  });

  it("names the cookie as the session module does", async () => {
    const { SESSION_COOKIE } = await import("../src/session.ts");
    expect(COOKIE_NAME).toBe(SESSION_COOKIE);
  });

  it("refuses a .test name with no @, and an address that merely contains .test", () => {
    for (const a of ["a.test", "a@b.test.com", "a@test"]) {
      const r = mint([a]);
      expect(r.code, a).toBe(2);
      expect(r.out, a).toBe("");
      expect(r.err, a).toContain("only a .test address may be minted");
    }
  });

  it("is judged before the secret is looked for", () => {
    const r = mint(["jamoore@gmail.com"], {});
    expect(r.err).toContain("only a .test address may be minted");
  });
});

// **A misplaced secret is never printed** (review I-5; ruling R-52): handed over as the address or as the `--days` value,
// by mistake, it would land in a terminal, a transcript or a report. A value is echoed only when it is address-shaped
// (one `@`), and a `--days` value only when it is digits.
describe("a misplaced secret", () => {
  const fragments = (err) => [SECRET, SECRET.slice(0, 8), SECRET.slice(-8)].filter((f) => err.includes(f));

  it("is not echoed when it is the only argument", () => {
    const r = mint([SECRET]);
    expect(r.code).toBe(2);
    expect(r.out).toBe("");
    expect(r.err).toBe("mint-session: only a .test address may be minted — got a value that is not an address\n");
    expect(fragments(r.err)).toEqual([]);
  });

  it("is not echoed when it is the --days value", () => {
    const r = mint(["red@t.test", "--days", SECRET]);
    expect(r.code).toBe(2);
    expect(r.err).toBe("mint-session: --days must be a whole number above zero — got a value that is not a whole number\n");
    expect(fragments(r.err)).toEqual([]);
  });

  it("an address-shaped value is still echoed, as the brief's sentence reads, and a non-address is not", () => {
    expect(fenceSentence("jamoore@gmail.com")).toBe("only a .test address may be minted — got jamoore@gmail.com");
    expect(fenceSentence("a.test")).toBe("only a .test address may be minted — got a value that is not an address");
    expect(fenceSentence("x@y@z")).toBe("only a .test address may be minted — got a value that is not an address");
  });
});

describe("the secret", () => {
  it("is refused when absent or empty, exit 2 and the brief's sentence", () => {
    for (const env of [{}, { YOKOHAMA_SESSION_SECRET: "" }]) {
      const r = mint(["red@t.test"], env);
      expect(r.code).toBe(2);
      expect(r.err).toBe("mint-session: YOKOHAMA_SESSION_SECRET is not set\n");
      expect(r.out).toBe("");
    }
  });
});

describe("--days", () => {
  it("refuses anything but a whole number above zero, exit 2", () => {
    for (const days of ["0", "00", "-1", "1.5", "abc", "", "30x", " 30", "1e2", "0x10", "+5"]) {
      const r = mint(["red@t.test", "--days", days]);
      expect(r.code, JSON.stringify(days)).toBe(2);
      // Only digits are echoed (ruling R-52).
      const got = /^[0-9]+$/.test(days) ? days : "a value that is not a whole number";
      expect(r.err, JSON.stringify(days)).toBe(`mint-session: --days must be a whole number above zero — got ${got}\n`);
      expect(r.out).toBe("");
    }
  });

  it("refuses a --days with no value, and a repeated one", () => {
    expect(mint(["red@t.test", "--days"]).code).toBe(2);
    expect(mint(["red@t.test", "--days", "3", "--days", "4"]).code).toBe(2);
  });

  it("sets the expiry that many days past the issue time, in either argument order", () => {
    for (const args of [["red@t.test", "--days", "7"], ["--days", "7", "red@t.test"]]) {
      const r = mint(args);
      expect(r.code).toBe(0);
      const p = payloadOf(r.out.trim().split("=")[1]);
      expect(p.expires - p.issued).toBe(7 * 86_400);
    }
  });
});

describe("the printed line", () => {
  it("is the __Host- cookie's name, '=', and the value the session module verifies, on one line and nothing else", async () => {
    const { mintSession } = await import("../src/session.ts");
    const before = Math.floor(Date.now() / 1000);
    const r = mint(["red@t.test"]);
    const after = Math.floor(Date.now() / 1000);
    expect(r.code).toBe(0);
    expect(r.err).toBe("");
    expect(COOKIE_NAME).toBe("__Host-yokohama_session");
    expect(r.out).toMatch(/^__Host-yokohama_session=v1\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]{43}\n$/);
    const value = r.out.trim().slice(`${COOKIE_NAME}=`.length);
    const p = payloadOf(value);
    expect(p.address).toBe("red@t.test");
    expect(p.sid).toBeNull();
    expect(p.issued, "the real clock").toBeGreaterThanOrEqual(before);
    expect(p.issued).toBeLessThanOrEqual(after);
    expect(p.expires - p.issued, "thirty days by default").toBe(30 * 86_400);
    expect(value).toBe(await mintSession({ address: "red@t.test", sid: null, issued: p.issued }, 30, SECRET));
    expect(r.out, "the secret is never printed").not.toContain(SECRET);
  });

  it("takes the secret from the environment only: an argument is a usage error", () => {
    const r = mint(["red@t.test", SECRET], {});
    expect(r.code).toBe(2);
    expect(r.out).toBe("");
    expect(r.err).not.toContain(SECRET);
  });
});

describe("usage", () => {
  it("is exit 2 with the usage line for no address, two addresses, or an unknown flag", () => {
    for (const args of [[], ["a@b.test", "c@d.test"], ["a@b.test", "--forever"]]) {
      const r = mint(args);
      expect(r.code, JSON.stringify(args)).toBe(2);
      expect(r.err).toContain("usage: node scripts/mint-session.mjs <address> [--days N]");
      expect(r.out).toBe("");
    }
  });
});
