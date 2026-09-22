// `localStorage`, read the one way this client reads it.
//
// **One accessor, because two of them diverged.** `chatFold.ts` and
// `install.ts` each carried their own copy of the same four lines, and the
// copies were not the same: the chat fold treated a missing value as the
// default and landed on the safe side, while the install offer compared the
// read against `null` — and an ABSENT storage short-circuits `storage?.getItem`
// to `undefined`, which `!== null` answers true to. A browser with cookies
// blocked was therefore read as a browser that had already dismissed the
// install offer, and neither affordance was drawn (review I-1). The bug was in
// the copy, so the copy is what goes.
//
// **The two ways a browser has no storage are different code paths**, and
// nothing that reads storage gets to conflate them:
//
//   - ABSENT: `localStorage` is not defined at all (an old embed, a worker
//     scope). The property access itself is fine; the read short-circuits.
//   - THROWING: the property access raises. iOS Safari with "Block All
//     Cookies" does this, as does a sandboxed iframe without
//     `allow-same-origin` and Chrome with site data blocked.
//
// [`readItem`] answers `null` for both, which is the answer "this browser has
// not stored that" — and never `undefined`, which is what a `!== null` compare
// gets wrong.
//
// **Five other copies of this idiom remain** (`api.ts` twice, `sections.ts`,
// `playback.ts`, `local/facts.ts`). None of them has the defect — each reads a
// value and treats a falsy one as its default — and converting them is a sweep
// no issue asks for. This is where they belong when one is next touched.

/** The browser's `localStorage`, or `null` where there is none to be had. */
export function browserStorage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    // The ACCESS raised, which is a different failure from its absence and
    // reaches the same answer.
    return null;
  }
}

/**
 * What is stored under `key`, or `null` — never `undefined`.
 *
 * The normalisation is the whole point: a caller comparing against `null` is
 * the ordinary shape, and it is only correct if an unreadable storage answers
 * `null` rather than short-circuiting the member expression away.
 */
export function readItem(key: string, storage: Storage | null = browserStorage()): string | null {
  try {
    return storage?.getItem(key) ?? null;
  } catch {
    return null;
  }
}

/** Store `value` under `key`, and say whether it landed.
 *
 *  **The boolean is not decoration.** A caller that must still act on a
 *  dismissal, a fold or a preference when the browser will not remember it
 *  needs to know that it did not — which is the second half of the same review
 *  finding (I-2): a control whose only effect was a write that threw changed
 *  nothing a player could see. */
export function writeItem(
  key: string,
  value: string,
  storage: Storage | null = browserStorage(),
): boolean {
  try {
    if (!storage) return false;
    storage.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}
