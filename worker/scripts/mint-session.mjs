// **A minted `.test` session cookie, for the live instruments** (auth cycle, Task 8). Under a sign-in arm the Worker
// names a caller by ITS OWN session cookie (`src/session.ts`), so the dev door's credential-plus-claimed-header no
// longer names a test identity there. The live replay and the live tier send this cookie instead
// (`YOKOHAMA_SESSION_COOKIE`, `dev-vars.mjs`):
//
//   YOKOHAMA_SESSION_SECRET=<the deployment's session secret> node scripts/mint-session.mjs <address> [--days N]
//
// prints one line, `__Host-yokohama_session=v1.<payload>.<mac>`, and nothing else.
//
// **The fence: `.test` addresses ONLY.** Whoever holds the secret can mint a cookie for any address, so a script that
// minted a real one would be a forged login with a convenient interface. The rule restates `src/identity.ts`'s
// `isTestAddress` — the bucket's own rule for its `test/` root — because plain `node` cannot import TypeScript;
// `mint-session.test.mjs` imports both and pins the restatement and the bytes to the module's, and to the openssl
// golden in `session.unit.test.ts`.
//
// **The bytes** (`src/session.ts`'s `seal`; Task 2): `payload` is the base64url (RFC 4648 §5, no padding) of the UTF-8
// bytes of `JSON.stringify({ address, sid, issued, expires })`, in that key order, `sid` written as `null`;
// `mac` is the base64url of HMAC-SHA256, keyed by the secret's UTF-8 bytes, over the ASCII bytes `"v1." + payload`.
//
// **Who else mints** (ruling R-50): the live replay and the live tier call `cookieLine` below when
// `YOKOHAMA_SESSION_SECRET` is in their environment, a cookie per identity, so the fence and the bytes are this file's
// alone — there is no second implementation to drift.
//
// **The secret** is read from the environment only — never an argument, never a file, and never printed.

import { createHmac } from "node:crypto";
import { pathToFileURL } from "node:url";

/** The cookie's name: `src/session.ts`'s `SESSION_COOKIE`, restated (the `__Host-` prefix is what the Worker reads). */
export const COOKIE_NAME = "__Host-yokohama_session";

const VERSION = "v1";
const DAY_SECONDS = 86_400;
const DEFAULT_DAYS = 30;
/** The first second `cookieLine` refuses as a clock: 2286-11-20, so a millisecond clock (thirteen digits today) is
 *  refused rather than minting a cookie that stands for millennia. */
const ISSUED_BOUND = 10_000_000_000;
const USAGE = "usage: node scripts/mint-session.mjs <address> [--days N]   (YOKOHAMA_SESSION_SECRET in the environment)";

/** `src/identity.ts`'s `isTestAddress`, restated: an `@` and the suffix `.test`. */
export function isTestAddress(email) {
  return email.includes("@") && email.endsWith(".test");
}

/** Whether `value` is shaped like an address — exactly one `@` — and so safe to echo in a refusal. */
const addressShaped = (value) => value.split("@").length === 2;

/**
 * The fence's sentence, without the script's `mint-session: ` prefix. **The value is echoed only when it is
 * address-shaped** (ruling R-52): a secret handed over as the address, by mistake, is not printed into a terminal, a
 * transcript or a report.
 */
export function fenceSentence(address) {
  return `only a .test address may be minted — got ${addressShaped(address) ? address : "a value that is not an address"}`;
}

/**
 * The whole cookie line for `address` — `__Host-yokohama_session=v1.…` — minted from `secret`, `sid` null, `days`
 * (default thirty) past `nowSeconds`. **The ONE function every minter calls** (this script, the live replay and the
 * live tier, ruling R-50): the `.test` fence is here, judged on the address exactly as it is minted — never trimmed or
 * lower-cased first, since the Worker's `is_test_identity` is case-sensitive — so no caller can mint a real address.
 * Throws the fence's sentence, prefixed `mint-session: `. Refuses a time that is not a whole number of seconds below
 * `ISSUED_BOUND`, so a millisecond clock is refused, and days that are not whole or are below one, as `session.ts`'s
 * `mintSession` refuses them. Neither refusal echoes the value it was handed (ruling R-52's rule, review I-5's class):
 * a secret passed in the clock's or the days' slot by mistake is never printed.
 */
export function cookieLine(address, secret, nowSeconds, days = DEFAULT_DAYS) {
  if (!isTestAddress(address)) throw new Error(`mint-session: ${fenceSentence(address)}`);
  if (!Number.isInteger(nowSeconds) || nowSeconds >= ISSUED_BOUND) {
    throw new RangeError(`mint-session: issued must be a whole number of seconds below ${ISSUED_BOUND} — a millisecond clock is refused`);
  }
  if (!Number.isInteger(days) || days < 1) throw new RangeError("mint-session: days must be a whole number above zero");
  return `${COOKIE_NAME}=${mintValue({ address, sid: null, issued: nowSeconds }, days, secret)}`;
}

/** The cookie's VALUE for `s`, expiring `days` after `s.issued`: `session.ts`'s `mintSession`, over `node:crypto`. */
export function mintValue(s, days, secret) {
  const payload = Buffer.from(JSON.stringify({ address: s.address, sid: s.sid, issued: s.issued, expires: s.issued + days * DAY_SECONDS }), "utf8").toString("base64url");
  const mac = createHmac("sha256", secret).update(`${VERSION}.${payload}`).digest("base64url");
  return `${VERSION}.${payload}.${mac}`;
}

/** An unknown flag, named by its NAME alone — the text before any `=`, with a glued value shown as `…` — and only when
 *  the name is shaped like one: lowercase letters and hyphens, at most 24 past the dashes, shorter than any session
 *  secret the shell accepts. Anything else is "an unknown argument". So a value glued on (`--days=<secret>`) or a
 *  secret handed over as a flag is never printed (ruling R-52). */
function unknownFlag(arg) {
  const eq = arg.indexOf("=");
  const name = eq === -1 ? arg : arg.slice(0, eq);
  if (!/^--[a-z]+(?:-[a-z]+)*$/.test(name) || name.length > 26) return "an unknown argument";
  return `unknown flag ${name}${eq === -1 ? "" : "=…"}`;
}

/**
 * The script as a function: `argv` after the script's name, the environment, and the clock in whole seconds.
 * Answers `{ code, out, err }`; the entry below prints them. Every refusal is exit 2 and a sentence on `err`.
 */
export function run(argv, env, nowSeconds) {
  const refuse = (sentence) => ({ code: 2, out: "", err: `mint-session: ${sentence}\n` });
  let days = DEFAULT_DAYS;
  let daysSeen = false;
  const positional = [];
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--days") {
      const value = argv[i + 1];
      if (daysSeen || value === undefined) return refuse(daysSeen ? `--days given twice — ${USAGE}` : `--days needs a value — ${USAGE}`);
      daysSeen = true;
      i += 1;
      if (!/^[1-9][0-9]*$/.test(value) || !Number.isSafeInteger(Number(value))) {
        // Only digits are echoed: a secret handed over as the value is never printed (ruling R-52).
        return refuse(`--days must be a whole number above zero — got ${/^[0-9]+$/.test(value) ? value : "a value that is not a whole number"}`);
      }
      days = Number(value);
    } else if (argv[i].startsWith("--")) {
      return refuse(`${unknownFlag(argv[i])} — ${USAGE}`);
    } else {
      positional.push(argv[i]);
    }
  }
  if (positional.length !== 1) return refuse(USAGE);
  const [address] = positional;
  // The fence comes before the secret is looked for: a real address is refused whatever the environment holds.
  if (!isTestAddress(address)) return refuse(fenceSentence(address));
  const secret = env.YOKOHAMA_SESSION_SECRET;
  if (secret === undefined || secret === "") return refuse("YOKOHAMA_SESSION_SECRET is not set");
  return { code: 0, out: `${cookieLine(address, secret, nowSeconds, days)}\n`, err: "" };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const { code, out, err } = run(process.argv.slice(2), process.env, Math.floor(Date.now() / 1000));
  if (out) process.stdout.write(out);
  if (err) process.stderr.write(err);
  process.exit(code);
}
