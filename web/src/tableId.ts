// The store's id grammar, as the hash router applies it (security S2a, finding D14; decision D-6).
//
// **A restatement, pinned.** `store::is_safe_id` (`crates/yokohama-web/src/store.rs`) is the authority — it is
// what every game's documents are keyed under — and `tableId.test.ts` holds this file to its verdicts over
// `web/src/wire/id_cases.json`, which `wire_fixtures` emits and refuses when it is stale. The router asks it so a
// link whose id the store would refuse — `..`, `x.table`, a space — lands on the door and never becomes an API
// path: `encodeURIComponent("..")` is `..`, and `/api/games/../chat` resolves to the HOUSE chat.

/** The longest id, in bytes — every accepted id is ASCII, so in characters too (`is_safe_id`'s `id.len() <= 64`). */
export const ID_MAX = 64;

/** The suffixes the store keeps for its own documents (`RESERVED_ID_SUFFIXES`), refused in any case. */
const RESERVED_SUFFIXES = [".table", ".chat"];

/** `is_safe_id`, arm by arm: not empty, at most 64 bytes, a letter or digit first, then letters, digits, `.`, `_`
 *  and `-` only, and not spelling another document's suffix. The alphabet test comes before the suffix test and
 *  makes the ASCII fold below exact: nothing outside ASCII survives it. The store's sixth arm, `is_the_house` (the
 *  house's own id, `_house`), is not restated: its leading `_` already fails the first-character test here, and
 *  `id_cases.json` carries the row, so a house stem that moved would show on the Rust side first. */
export function isSafeId(id: string): boolean {
  if (id === "" || id.length > ID_MAX) return false;
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(id)) return false;
  const lower = id.replace(/[A-Z]/g, (c) => c.toLowerCase());
  return !RESERVED_SUFFIXES.some((suffix) => lower.endsWith(suffix));
}
