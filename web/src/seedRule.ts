// The seed the new-table form checks AS IT IS TYPED (M6.5c2, spec §2.2 "The
// seed"; R-6.5c2.8), by the server's own rule.
//
// **A second statement of `WireSeed`, and held to it.** The server reads a seed
// sent as text with `str::trim` and then `u64::from_str` (`tables.rs`,
// `WireSeed`'s `Deserialize`): an optional `+`, ASCII digits, at most
// 18446744073709551615. The form runs the same rule so a bad value explains
// itself in place instead of arriving as the create's 400 — and a copy of a rule
// is a thing that drifts, so `seedRule.test.ts` asserts this file's verdict on
// every input in `wire/seed_cases.json`, which is the server's own.
//
// **A seed is never a number here.** A `u64` does not survive a JavaScript
// number (`Number("18446744073709551557")` is 18446744073709552000), so the value
// stays text from the keystroke to the wire and the range is compared as a
// `BigInt`.

/** The largest seed there is: `u64::MAX`. */
const MAX = 18_446_744_073_709_551_615n;

/** The rule, in the words both refusals end with (mock 07). */
const RULE =
  "a seed is a whole number from 0 to 18446744073709551615. Leave it blank to have the server draw one.";

/**
 * `str::trim` as Rust reads it: the Unicode `White_Space` property off both ends.
 *
 * **Not `String.prototype.trim`**, which strips a different set — it removes
 * U+FEFF (not `White_Space`) and keeps U+0085 (which is). Measured both ways
 * when this was written, and pinned by the committed table's rows for exactly
 * those two characters.
 */
export const trimSeed = (raw: string): string =>
  raw.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, "");

export type SeedVerdict = { ok: true } | { ok: false; reason: string };

/** Whether the server would take `raw` as a seed, and why not where it would
 *  not. **A blank field is the FORM's question, not this one's**: blank means
 *  "the server draws one" and is never sent, while `WireSeed` refuses an empty
 *  string like any other non-number — so the table says `false` for `""` and
 *  the form never asks. */
export function checkSeed(raw: string): SeedVerdict {
  const digits = /^\+?([0-9]+)$/.exec(trimSeed(raw));
  if (digits === null) return { ok: false, reason: `Not a whole number: ${RULE}` };
  if (BigInt(digits[1]!) > MAX) return { ok: false, reason: `Too large: ${RULE}` };
  return { ok: true };
}
