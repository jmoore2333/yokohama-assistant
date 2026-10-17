// The key grammar, as the server applies it (M6.5c2, R-6.5c2.3; spec §2.1).
//
// **A restatement, pinned.** `User::parse` (`crates/yokohama-web/src/identity.rs`)
// is the authority — it keys every seat, every watcher and every archive
// directory — and the name step normalizes exactly as it does, so the header
// this browser sends IS the key the server writes seats under.
// `nameRule.test.ts` holds this file to the layer's own verdicts over
// `web/src/wire/identity_cases.json`, which `wire_fixtures` emits and refuses
// when it is stale — so the two rules cannot drift apart in silence.
//
// **Rust's rules, character for character, and not JavaScript's** (measured
// against rustc when this plan was drafted):
// - `str::trim` strips the Unicode White_Space property, which is NOT what
//   `String.prototype.trim` strips: Rust trims U+0085 and keeps U+FEFF,
//   JavaScript the reverse;
// - `to_ascii_lowercase` folds A–Z only, where `toLowerCase` folds the Kelvin
//   sign U+212A into a `k` and would admit a name the server refuses;
// - `char::is_control` is the Cc category, U+0000–U+001F and U+007F–U+009F.
// A client built on the JavaScript built-ins would accept names the server
// refuses and send keys the server never writes.
//
// **Server parity (R-B2): an `@` takes the email half of the grammar**, exactly
// as `User::parse` does, so the pin holds two-way over every row. The hint the
// name step shows reads the plain-name rule, which is what a player types.
//
// **The name as TYPED is kept too**, as the display name: "  Ben   Castle " is
// the key `ben castle`, shown `Ben Castle` (Review Focus 1).

/** `char::is_whitespace` — the Unicode White_Space property — as a class body.
 *  Exported for its pin: `nameRule.test.ts` holds it to JavaScript's own
 *  `\p{White_Space}` over every BMP code point, both ways, so the set can
 *  neither gain nor lose a member in silence. Every member is written as an
 *  escape: most of them are invisible, and an editor may strip U+2028 and
 *  U+2029 as "unusual line terminators". */
export const WHITE = "\t\n\u000b\f\r \u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000";
const IS_WHITE = new RegExp(`[${WHITE}]`);
const EDGES = new RegExp(`^[${WHITE}]+|[${WHITE}]+$`, "g");
/** `char::is_control`: the Unicode Cc category. */
const IS_CONTROL = /[\u0000-\u001f\u007f-\u009f]/;

/** The longest plain name, in characters — all ASCII, so also in bytes. */
export const NAME_MAX = 32;
/** The longest address, in UTF-8 bytes (`User::parse`'s `s.len()`). */
const ADDRESS_MAX_BYTES = 254;

export type NameVerdict = { ok: true; key: string; display: string } | { ok: false; reason: string };

const asciiLower = (s: string): string => s.replace(/[A-Z]/g, (c) => c.toLowerCase());

/** The key a raw input is, and how it is shown — or why the server would
 *  refuse it. An `@` decides which half of the grammar applies: a plain name
 *  can never hold one, and an address always does. */
export function normalizeName(raw: string): NameVerdict {
  const typed = raw.replace(EDGES, "");
  if (typed === "") return { ok: false, reason: "Type a name to play under." };
  return typed.includes("@") ? address(typed) : plainName(typed);
}

/** The open door's own refusal of an address (security S2b, finding C1; decision D-2), in this client's words:
 *  the server's `ADDRESS_NOT_A_NAME` says the same thing in its own. */
export const ADDRESS_NOT_A_NAME_HERE = "Under the open door a name is a name — an address is not accepted here. Choose a plain name.";

/** The name step's verdict (it exists under the open door alone): any `@` in the RAW input is refused FIRST, in the
 *  door's own words — before the key grammar, whose sentence for a malformed address would tell an open-door
 *  player how to write one — and then the key grammar's verdict (the server's `ClaimPolicy::NamesOnly` refuses the
 *  same key with its own sentence). `normalizeName` itself stays the KEY grammar, pinned to `identity_cases.json`,
 *  which still lists addresses as valid KEYS: they are, under Access. */
export function openDoorVerdict(raw: string): NameVerdict {
  if (raw.includes("@")) return { ok: false, reason: ADDRESS_NOT_A_NAME_HERE };
  return normalizeName(raw);
}

/** The email shape `User::parse` has always taken: exactly one `@` with
 *  something on both sides, no whitespace or control character, at most 254
 *  bytes. */
function address(typed: string): NameVerdict {
  const key = asciiLower(typed);
  if (new TextEncoder().encode(key).length > ADDRESS_MAX_BYTES) {
    return { ok: false, reason: "An address is at most 254 bytes." };
  }
  if (IS_WHITE.test(key) || IS_CONTROL.test(key)) {
    return { ok: false, reason: "An address has no spaces or control characters in it." };
  }
  const parts = key.split("@");
  if (parts.length !== 2 || parts[0] === "" || parts[1] === "") {
    return { ok: false, reason: "An address has exactly one @, with something on both sides of it." };
  }
  return { ok: true, key, display: typed };
}

/** A plain name: 1–32 of ASCII letters, digits, space, `.`, `-` and `_`,
 *  beginning with a letter or a digit, internal runs of spaces collapsed to
 *  one, and never `anonymous`. The leading letter or digit is what keeps a
 *  name's archive directory from ever being `.` or `..` (spec §2.1, review H1). */
function plainName(typed: string): NameVerdict {
  const display = typed.replace(/ {2,}/g, " ");
  const key = asciiLower(display);
  if (!/^[a-z0-9 ._-]*$/.test(key)) {
    return { ok: false, reason: "A name uses letters, numbers, spaces, dots, dashes and underscores \u2014 nothing else." };
  }
  if (!/^[a-z0-9]/.test(key)) return { ok: false, reason: "A name begins with a letter or a digit." };
  if (key.length > NAME_MAX) return { ok: false, reason: `A name is at most ${NAME_MAX} characters.` };
  if (key === "anonymous") {
    return { ok: false, reason: "\u201canonymous\u201d means nobody on this server \u2014 choose another name." };
  }
  return { ok: true, key, display };
}
