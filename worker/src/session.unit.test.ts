import { afterEach, describe, expect, it, vi } from "vitest";
import {
  LOGIN_COOKIE,
  SESSION_COOKIE,
  clearCookie,
  cookieValue,
  mintLogin,
  mintSession,
  readLogin,
  readSession,
  renewDue,
  setCookie,
  type Session,
} from "./session";

const SECRET = "0123456789abcdef0123456789abcdef";
const NOW = 1_789_000_000;

afterEach(() => {
  vi.restoreAllMocks();
});

// The brief's five cells (the auth cycle's plan, Task 2 Step 1), verbatim but for one cast (correction C-7): under
// `noUncheckedIndexedAccess` a destructured `split` reads `string | undefined`, and `mac[0]` does not typecheck. The
// Set-Cookie cell spells the `__Host-` names (ruling R-14, correction C-6), pinning them on the writing side; the
// literal-name cell under "reading a cookie" pins them on the reading side, and every other cell uses the constants.
describe("the session and login cookies (auth cycle, Task 2)", () => {
  it("mints and reads back, with the expiry 30 days out", async () => {
    const v = await mintSession({ address: "red@t.test", sid: "session_01", issued: NOW }, 30, SECRET);
    expect(v.startsWith("v1.")).toBe(true);
    expect(await readSession(`other=1; ${SESSION_COOKIE}=${v}`, SECRET, NOW + 5)).toEqual({ address: "red@t.test", sid: "session_01", issued: NOW, expires: NOW + 30 * 86_400 });
  });
  it("a changed MAC byte, an expired payload, a non-JSON payload, another secret and a v0 prefix each read as nobody", async () => {
    const v = await mintSession({ address: "red@t.test", sid: null, issued: NOW }, 30, SECRET);
    const [p, body, mac] = v.split(".") as [string, string, string];
    const flipped = mac[0] === "A" ? "B" : "A";
    for (const bad of [`${p}.${body}.${flipped}${mac.slice(1)}`, `v0.${body}.${mac}`, `${p}.!!!.${mac}`, v.slice(0, -1)]) {
      expect(await readSession(`${SESSION_COOKIE}=${bad}`, SECRET, NOW)).toBeNull();
    }
    expect(await readSession(`${SESSION_COOKIE}=${v}`, "x".repeat(32), NOW)).toBeNull();
    expect(await readSession(`${SESSION_COOKIE}=${v}`, SECRET, NOW + 30 * 86_400 + 1)).toBeNull();
    expect(await readSession(null, SECRET, NOW)).toBeNull();
    expect(await readSession("a=b", SECRET, NOW)).toBeNull();
  });
  it("renewal is due past half the life and not before", async () => {
    const s = { address: "a@b.test", sid: null, issued: NOW, expires: NOW + 30 * 86_400 };
    expect(renewDue(s, NOW + 15 * 86_400 - 1, 30)).toBe(false);
    expect(renewDue(s, NOW + 15 * 86_400 + 1, 30)).toBe(true);
  });
  it("the login cookie lives ten minutes and carries the four fields", async () => {
    const v = await mintLogin({ state: "st", nonce: "no", verifier: "ve", returnTo: "/#/t/sim-1" }, NOW, SECRET);
    expect(await readLogin(`${LOGIN_COOKIE}=${v}`, SECRET, NOW + 599)).toEqual({ state: "st", nonce: "no", verifier: "ve", returnTo: "/#/t/sim-1", expires: NOW + 600 });
    expect(await readLogin(`${LOGIN_COOKIE}=${v}`, SECRET, NOW + 601)).toBeNull();
  });
  it("the Set-Cookie lines carry the attributes verbatim", () => {
    expect(setCookie(SESSION_COOKIE, "v", 60)).toBe("__Host-yokohama_session=v; Max-Age=60; Path=/; HttpOnly; Secure; SameSite=Lax");
    expect(clearCookie(LOGIN_COOKIE)).toBe("__Host-yokohama_login=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax");
  });
});

// The two goldens were computed OUTSIDE this module, by `openssl` alone (the payload through `openssl base64 -A`
// with `+/` mapped to `-_` and `=` dropped, the MAC through `openssl dgst -sha256 -hmac "$SECRET" -binary` over
// the bytes `v1.<payload>`), so a cell comparing a minted value to one compares two implementations, not one with
// itself. They are the reference the plan's Task 8 holds its `node:crypto` minter to.
const GOLDEN_SESSION_JSON = '{"address":"red@t.test","sid":null,"issued":1789000000,"expires":1791592000}';
const GOLDEN_SESSION =
  "v1.eyJhZGRyZXNzIjoicmVkQHQudGVzdCIsInNpZCI6bnVsbCwiaXNzdWVkIjoxNzg5MDAwMDAwLCJleHBpcmVzIjoxNzkxNTkyMDAwfQ" +
  ".C7Evl68QvNoLo7oHRHSQ0zUgg7untTb2bxOrXqFIsSo";
/** A `returnTo` chosen so the payload's encoding carries both `-` and `_` and its standard base64 would pad. */
const GOLDEN_RETURN_TO = "/#/t/sim-0?x=>>>???";
const GOLDEN_LOGIN_JSON = `{"state":"st","nonce":"no","verifier":"ve","returnTo":"${GOLDEN_RETURN_TO}","expires":1789000600}`;
const GOLDEN_LOGIN =
  "v1.eyJzdGF0ZSI6InN0Iiwibm9uY2UiOiJubyIsInZlcmlmaWVyIjoidmUiLCJyZXR1cm5UbyI6Ii8jL3Qvc2ltLTA_eD0-Pj4_Pz8iLCJleHBpcmVzIjoxNzg5MDAwNjAwfQ" +
  ".4V9jIj0ehl56iNIJCoKbtN4vxz7qSQNEycVw1wg5buc";

const utf8 = new TextEncoder();
const b64url = (b: Uint8Array): string =>
  btoa(String.fromCharCode(...b)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
const payloadText = (value: string): string => {
  const body = value.split(".")[1] ?? "";
  return new TextDecoder().decode(Uint8Array.from(atob(body.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0)));
};

/** The same session payload under a secret whose first six bytes are two CJK ideographs' UTF-8 (`e6 97 a5 e6 9c
 *  ac`), computed by openssl as above: the key is the secret's UTF-8 bytes, as `shellConfig` counts them. */
const WIDE_SECRET = `日本${SECRET}`;
const GOLDEN_WIDE_SESSION =
  "v1.eyJhZGRyZXNzIjoicmVkQHQudGVzdCIsInNpZCI6bnVsbCwiaXNzdWVkIjoxNzg5MDAwMDAwLCJleHBpcmVzIjoxNzkxNTkyMDAwfQ" +
  ".-3A7m_T9bmoqo3f00D02OtuX3w3UdXzSbVa89Ph6rvc";

/** The cells' own signer, for payloads the minter must never produce. The first cell under "reading" checks it
 *  against the openssl golden, so every hand-signed cell below rests on a signer that agrees with openssl. */
async function signedBytes(payload: Uint8Array, secret = SECRET, version = "v1"): Promise<string> {
  const body = b64url(payload);
  const key = await crypto.subtle.importKey("raw", utf8.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  const mac = new Uint8Array(await crypto.subtle.sign("HMAC", key, utf8.encode(`${version}.${body}`)));
  return `${version}.${body}.${b64url(mac)}`;
}
const signed = (json: string, secret = SECRET): Promise<string> => signedBytes(utf8.encode(json), secret);

describe("the canonical byte form (the reference for Task 8's minter)", () => {
  it("a minted session is exactly the bytes openssl computes, with sid null written out", async () => {
    const v = await mintSession({ address: "red@t.test", sid: null, issued: NOW }, 30, SECRET);
    expect(v).toBe(GOLDEN_SESSION);
    expect(payloadText(v)).toBe(GOLDEN_SESSION_JSON);
  });

  it("a minted login is exactly the bytes openssl computes, in the url alphabet with no padding", async () => {
    expect(GOLDEN_LOGIN.split(".")[1]).toMatch(/-/);
    expect(GOLDEN_LOGIN.split(".")[1]).toMatch(/_/);
    expect(GOLDEN_LOGIN).not.toMatch(/[=+/]/);
    const v = await mintLogin({ state: "st", nonce: "no", verifier: "ve", returnTo: GOLDEN_RETURN_TO }, NOW, SECRET);
    expect(v).toBe(GOLDEN_LOGIN);
    expect(payloadText(v)).toBe(GOLDEN_LOGIN_JSON);
  });

  it("the key is the secret's UTF-8 bytes", async () => {
    expect(await mintSession({ address: "red@t.test", sid: null, issued: NOW }, 30, WIDE_SECRET)).toBe(GOLDEN_WIDE_SESSION);
    expect(await readSession(`${SESSION_COOKIE}=${GOLDEN_WIDE_SESSION}`, WIDE_SECRET, NOW)).not.toBeNull();
  });

  it("neither the input's field order nor a stray field reaches the bytes", async () => {
    expect(await mintSession({ issued: NOW, sid: null, address: "red@t.test" }, 30, SECRET)).toBe(GOLDEN_SESSION);
    const stray = { address: "red@t.test", sid: null, issued: NOW, expires: 1, extra: 1 } as Omit<Session, "expires">;
    expect(await mintSession(stray, 30, SECRET)).toBe(GOLDEN_SESSION);
    expect(
      await mintLogin({ returnTo: GOLDEN_RETURN_TO, verifier: "ve", nonce: "no", state: "st" }, NOW, SECRET),
    ).toBe(GOLDEN_LOGIN);
  });

  it("sid null and a non-ASCII value round-trip", async () => {
    const v = await mintSession({ address: "red@t.test", sid: null, issued: NOW }, 30, SECRET);
    expect(await readSession(`${SESSION_COOKIE}=${v}`, SECRET, NOW)).toEqual({ address: "red@t.test", sid: null, issued: NOW, expires: NOW + 30 * 86_400 });
    const returnTo = "/#/t/日本";
    const l = await mintLogin({ state: "st", nonce: "no", verifier: "ve", returnTo }, NOW, SECRET);
    expect((await readLogin(`${LOGIN_COOKIE}=${l}`, SECRET, NOW))?.returnTo).toBe(returnTo);
  });

  it("a time that is not whole seconds is refused at the mint, never written", async () => {
    await expect(mintSession({ address: "red@t.test", sid: null, issued: NOW + 0.5 }, 30, SECRET)).rejects.toThrow(RangeError);
    await expect(mintSession({ address: "red@t.test", sid: null, issued: NOW }, 1.5, SECRET)).rejects.toThrow(RangeError);
    await expect(mintLogin({ state: "st", nonce: "no", verifier: "ve", returnTo: "/" }, NOW + 0.5, SECRET)).rejects.toThrow(RangeError);
  });

  it("days below one are refused at the mint, so no cookie is minted dead or expiring before it was issued", async () => {
    for (const days of [0, -1]) {
      await expect(mintSession({ address: "red@t.test", sid: null, issued: NOW }, days, SECRET), `${days}`).rejects.toThrow(RangeError);
    }
    const one = await mintSession({ address: "red@t.test", sid: null, issued: NOW }, 1, SECRET);
    expect((await readSession(`${SESSION_COOKIE}=${one}`, SECRET, NOW))?.expires).toBe(NOW + 86_400);
  });
});

describe("reading a cookie", () => {
  it("the cells' own signer reproduces the openssl golden, so the hand-signed cells rest on it", async () => {
    expect(await signed(GOLDEN_SESSION_JSON)).toBe(GOLDEN_SESSION);
    expect(await signed(GOLDEN_LOGIN_JSON)).toBe(GOLDEN_LOGIN);
  });

  it("a payload signed with its fields in another order reads, and only the declared fields come back", async () => {
    const s = await signed('{"expires":1791592000,"extra":1,"issued":1789000000,"sid":"s1","address":"a@b.test"}');
    expect(await readSession(`${SESSION_COOKIE}=${s}`, SECRET, NOW)).toEqual({ address: "a@b.test", sid: "s1", issued: NOW, expires: 1_791_592_000 });
    const l = await signed('{"expires":1789000600,"returnTo":"/","verifier":"v","nonce":"n","state":"s","extra":1}');
    expect(await readLogin(`${LOGIN_COOKIE}=${l}`, SECRET, NOW)).toEqual({ state: "s", nonce: "n", verifier: "v", returnTo: "/", expires: NOW + 600 });
  });

  it("a verified payload of any other shape reads as nobody", async () => {
    const sessions = [
      '{"address":5,"sid":null,"issued":1789000000,"expires":1791592000}',
      '{"sid":null,"issued":1789000000,"expires":1791592000}',
      '{"address":"a@b.test","sid":7,"issued":1789000000,"expires":1791592000}',
      '{"address":"a@b.test","issued":1789000000,"expires":1791592000}',
      '{"address":"a@b.test","sid":null,"issued":"1789000000","expires":1791592000}',
      '{"address":"a@b.test","sid":null,"issued":1789000000.5,"expires":1791592000}',
      '{"address":"a@b.test","sid":null,"issued":1789000000}',
      '{"address":"a@b.test","sid":null,"issued":1789000000,"expires":1791592000.5}',
      '{"address":"a@b.test","sid":null,"issued":1789000000,"expires":"1791592000"}',
      "[]",
      "null",
      '"a@b.test"',
      "{",
    ];
    for (const json of sessions) {
      expect(await readSession(`${SESSION_COOKIE}=${await signed(json)}`, SECRET, NOW), json).toBeNull();
    }
    const logins = [
      '{"state":"s","nonce":"n","returnTo":"/","expires":1789000600}',
      '{"state":"s","nonce":5,"verifier":"v","returnTo":"/","expires":1789000600}',
      '{"state":"s","nonce":"n","verifier":"v","returnTo":5,"expires":1789000600}',
      '{"state":"s","nonce":"n","verifier":"v","returnTo":"/","expires":"1789000600"}',
      '{"state":null,"nonce":"n","verifier":"v","returnTo":"/","expires":1789000600}',
      "{}",
      "{",
    ];
    for (const json of logins) {
      expect(await readLogin(`${LOGIN_COOKIE}=${await signed(json)}`, SECRET, NOW), json).toBeNull();
    }
  });

  it("a verified payload that is not UTF-8 reads as nobody, rather than as its replacement characters", async () => {
    const json = utf8.encode('{"address":"a?@b.test","sid":null,"issued":1789000000,"expires":1791592000}');
    expect(await readSession(`${SESSION_COOKIE}=${await signedBytes(json)}`, SECRET, NOW)).not.toBeNull();
    json[json.indexOf(0x3f)] = 0xff; // the "?" becomes a byte that begins no UTF-8 sequence
    expect(await readSession(`${SESSION_COOKIE}=${await signedBytes(json)}`, SECRET, NOW)).toBeNull();
  });

  it("a verified payload that opens with a byte-order mark reads as nobody: the mark is not stripped", async () => {
    const json = utf8.encode(GOLDEN_SESSION_JSON);
    expect(await readSession(`${SESSION_COOKIE}=${await signedBytes(json)}`, SECRET, NOW)).not.toBeNull();
    const marked = new Uint8Array([0xef, 0xbb, 0xbf, ...json]);
    expect(await readSession(`${SESSION_COOKIE}=${await signedBytes(marked)}`, SECRET, NOW)).toBeNull();
  });

  it("a cookie of another version reads as nobody even when its MAC verifies", async () => {
    const payload = utf8.encode(GOLDEN_SESSION_JSON);
    expect(await readSession(`${SESSION_COOKIE}=${await signedBytes(payload, SECRET, "v1")}`, SECRET, NOW)).not.toBeNull();
    for (const version of ["v2", "V1", "v1 ", ""]) {
      expect(await readSession(`${SESSION_COOKIE}=${await signedBytes(payload, SECRET, version)}`, SECRET, NOW), version).toBeNull();
    }
  });

  it("each reader reads its literal __Host- name, and neither the bare name nor another case of the prefix", async () => {
    expect(await readSession(`__Host-yokohama_session=${GOLDEN_SESSION}`, SECRET, NOW)).not.toBeNull();
    expect(await readLogin(`__Host-yokohama_login=${GOLDEN_LOGIN}`, SECRET, NOW)).not.toBeNull();
    // Names a sibling subdomain may still be able to set (the bare name, `__Secure-`, and in an older browser another
    // case of the prefix): none of them is either cookie.
    for (const name of ["yokohama_session", "__host-yokohama_session", "__HOST-yokohama_session", "__Secure-yokohama_session"]) {
      expect(await readSession(`${name}=${GOLDEN_SESSION}`, SECRET, NOW), name).toBeNull();
    }
    for (const name of ["yokohama_login", "__host-yokohama_login", "__Secure-yokohama_login"]) {
      expect(await readLogin(`${name}=${GOLDEN_LOGIN}`, SECRET, NOW), name).toBeNull();
    }
  });

  it("a session is no login and a login no session, under either name", async () => {
    const s = await mintSession({ address: "red@t.test", sid: null, issued: NOW }, 30, SECRET);
    const l = await mintLogin({ state: "st", nonce: "no", verifier: "ve", returnTo: "/" }, NOW, SECRET);
    expect(await readLogin(`${LOGIN_COOKIE}=${s}`, SECRET, NOW)).toBeNull();
    expect(await readSession(`${SESSION_COOKIE}=${l}`, SECRET, NOW)).toBeNull();
    // Each reader reads its own name and no other.
    expect(await readSession(`${LOGIN_COOKIE}=${s}`, SECRET, NOW)).toBeNull();
    expect(await readLogin(`${SESSION_COOKIE}=${l}`, SECRET, NOW)).toBeNull();
  });

  it("the expiry second itself reads as nobody and the second before it reads", async () => {
    const s = await mintSession({ address: "red@t.test", sid: null, issued: NOW }, 30, SECRET);
    const expires = NOW + 30 * 86_400;
    expect((await readSession(`${SESSION_COOKIE}=${s}`, SECRET, expires - 1))?.expires).toBe(expires);
    expect(await readSession(`${SESSION_COOKIE}=${s}`, SECRET, expires)).toBeNull();
    const l = await mintLogin({ state: "st", nonce: "no", verifier: "ve", returnTo: "/" }, NOW, SECRET);
    expect(await readLogin(`${LOGIN_COOKIE}=${l}`, SECRET, NOW + 600)).toBeNull();
  });

  it("a NaN clock reads every cookie as nobody: the expiry check fails closed", async () => {
    expect(await readSession(`${SESSION_COOKIE}=${GOLDEN_SESSION}`, SECRET, NOW)).not.toBeNull();
    expect(await readLogin(`${LOGIN_COOKIE}=${GOLDEN_LOGIN}`, SECRET, NOW)).not.toBeNull();
    expect(await readSession(`${SESSION_COOKIE}=${GOLDEN_SESSION}`, SECRET, NaN)).toBeNull();
    expect(await readLogin(`${LOGIN_COOKIE}=${GOLDEN_LOGIN}`, SECRET, NaN)).toBeNull();
  });

  it("another spelling of a valid MAC, or a wrong segment count, reads as nobody", async () => {
    // A cookie whose MAC segment carries `-` or `_`, found by walking the sid (deterministic: the same first hit
    // on every run), so the standard-alphabet spelling below differs from the minted one.
    let v = "";
    for (let n = 0; !/[-_]/.test(v.split(".")[2] ?? ""); n++) {
      v = await mintSession({ address: "red@t.test", sid: `s${n}`, issued: NOW }, 30, SECRET);
    }
    expect(await readSession(`${SESSION_COOKIE}=${v}`, SECRET, NOW)).not.toBeNull();
    const [p, body, mac] = v.split(".") as [string, string, string];
    const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    // A 32-byte MAC is 43 characters whose last one carries two unused bits: flipping the lowest decodes the same.
    const sameBytes = `${mac.slice(0, -1)}${ALPHABET[ALPHABET.indexOf(mac.slice(-1)) ^ 1]}`;
    const spellings = [
      `${p}.${body}.${mac}=`,
      `${p}.${body}.${sameBytes}`,
      `${p}.${body}.${mac.replace(/-/g, "+").replace(/_/g, "/")}`,
      `${p}.${body}.${mac.slice(0, 20)} ${mac.slice(20)}`,
      `${p}.${body}.${mac}.`,
      `${p}.${body}`,
      `${p}..${mac}`,
      `${p}.${body}.`,
      "",
    ];
    for (const bad of spellings) {
      expect(await readSession(`${SESSION_COOKIE}=${bad}`, SECRET, NOW), bad).toBeNull();
    }
  });

  it("a MAC segment of any length but 43 is refused before it is decoded or verified", async () => {
    const decodes = vi.spyOn(globalThis, "atob");
    const verify = vi.spyOn(crypto.subtle, "verify");
    const [p, body, mac] = GOLDEN_SESSION.split(".") as [string, string, string];
    expect(mac).toHaveLength(43);
    // 44 characters, none, and an attacker-sized segment: none of them is decoded or verified.
    for (const bad of [`${mac}A`, "", "A".repeat(128 * 1024)]) {
      expect(await readSession(`${SESSION_COOKIE}=${p}.${body}.${bad}`, SECRET, NOW), `${bad.length}`).toBeNull();
    }
    expect(decodes).not.toHaveBeenCalled();
    expect(verify).not.toHaveBeenCalled();
    // The anti-vacuity half: the spies do see a 43-character MAC decoded and verified.
    expect(await readSession(`${SESSION_COOKIE}=${GOLDEN_SESSION}`, SECRET, NOW)).not.toBeNull();
    expect(decodes).toHaveBeenCalled();
    expect(verify).toHaveBeenCalledTimes(1);
  });

  it("a key the platform refuses reads as nobody, with no exception, and refuses to mint", async () => {
    await expect(readSession(`${SESSION_COOKIE}=${GOLDEN_SESSION}`, "", NOW)).resolves.toBeNull();
    await expect(readLogin(`${LOGIN_COOKIE}=${GOLDEN_LOGIN}`, "", NOW)).resolves.toBeNull();
    await expect(mintSession({ address: "red@t.test", sid: null, issued: NOW }, 30, "")).rejects.toThrow();
  });

  it("the MAC is crypto.subtle.verify's, and the payload is parsed only once it verifies", async () => {
    const verify = vi.spyOn(crypto.subtle, "verify");
    const sign = vi.spyOn(crypto.subtle, "sign");
    const parse = vi.spyOn(JSON, "parse");
    const [p, body, mac] = GOLDEN_SESSION.split(".") as [string, string, string];
    const flipped = `${p}.${body}.${mac[0] === "A" ? "B" : "A"}${mac.slice(1)}`;
    expect(await readSession(`${SESSION_COOKIE}=${flipped}`, SECRET, NOW)).toBeNull();
    expect(parse.mock.calls.some(([text]) => text === GOLDEN_SESSION_JSON)).toBe(false);
    expect(await readSession(`${SESSION_COOKIE}=${GOLDEN_SESSION}`, SECRET, NOW)).not.toBeNull();
    // The anti-vacuity half: the spy does see the reader's parse once the MAC verifies.
    expect(parse.mock.calls.some(([text]) => text === GOLDEN_SESSION_JSON)).toBe(true);
    expect(verify).toHaveBeenCalledTimes(2);
    expect(sign).not.toHaveBeenCalled();
  });
});

describe("renewal", () => {
  it("exactly half the life left is not yet due", () => {
    const s = { address: "a@b.test", sid: null, issued: NOW, expires: NOW + 30 * 86_400 };
    expect(renewDue(s, NOW + 15 * 86_400, 30)).toBe(false);
  });

  it("is read from the life LEFT against the configured days, so a cookie minted under other days still comes due", () => {
    // Minted under 7 days, read under 30: less than 15 days are left, so it renews at once rather than lapse.
    const short = { address: "a@b.test", sid: null, issued: NOW, expires: NOW + 7 * 86_400 };
    expect(renewDue(short, NOW + 1, 30)).toBe(true);
    // Minted under 30 days, read under 7: it renews once less than three and a half days are left.
    const long = { address: "a@b.test", sid: null, issued: NOW, expires: NOW + 30 * 86_400 };
    expect(renewDue(long, NOW + 1, 7)).toBe(false);
    expect(renewDue(long, NOW + 30 * 86_400 - 3.5 * 86_400 + 1, 7)).toBe(true);
  });
});

describe("cookieValue", () => {
  it("matches a name exactly, case included, takes the first match and splits on the first '=' only", () => {
    expect(cookieValue("a=b=c", "a")).toBe("b=c");
    expect(cookieValue(` ; ;${SESSION_COOKIE}_x=1;  ${SESSION_COOKIE}=2 ; ${SESSION_COOKIE}=3`, SESSION_COOKIE)).toBe("2");
    expect(cookieValue(`x_${SESSION_COOKIE}=1`, SESSION_COOKIE)).toBeNull();
    expect(cookieValue(`${SESSION_COOKIE.toUpperCase()}=1`, SESSION_COOKIE)).toBeNull();
    expect(cookieValue(`${SESSION_COOKIE} =1`, SESSION_COOKIE)).toBeNull();
    expect(cookieValue(SESSION_COOKIE, SESSION_COOKIE)).toBeNull();
    expect(cookieValue(`${SESSION_COOKIE}=`, SESSION_COOKIE)).toBe("");
    expect(cookieValue("", SESSION_COOKIE)).toBeNull();
    expect(cookieValue(null, SESSION_COOKIE)).toBeNull();
  });

  it("reads a Set-Cookie line, which is how Task 4's cells hand one to readLogin", async () => {
    const v = await mintLogin({ state: "st", nonce: "no", verifier: "ve", returnTo: "/" }, NOW, SECRET);
    const line = setCookie(LOGIN_COOKIE, v, 600);
    expect(cookieValue(line, LOGIN_COOKIE)).toBe(v);
    expect((await readLogin(line, SECRET, NOW))?.state).toBe("st");
  });
});
