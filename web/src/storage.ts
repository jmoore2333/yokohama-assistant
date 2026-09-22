// What this browser has promised about the games on it (M6.5b2, R-6.5b2.9;
// spec §2.4).
//
// **The grant is asked for at the FIRST COMMITTED MOVE of a local game and
// never at boot**, so a visitor who never plays is never asked. In some
// browsers `persist()` is a prompt; in others it is decided silently from how
// the site has been used. Either way it is a question about games, and asking
// it before there are any is asking it of somebody with no reason to answer.
//
// **The grant is read BACK with `persisted()` rather than assumed.** A
// `persist()` that resolves `true` is this call's outcome; `persisted()` is
// the browser's standing answer, and it is the one the door reports. A line
// built from the request would tell a player their games are kept on a browser
// that never agreed to keep them.
//
// **Nothing here claims safety.** A grant means the browser will not clear
// this origin to reclaim space; it does not survive a cleared browser, a lost
// device, or an iOS home-screen install, which gets its own storage container
// and shares nothing with Safari. So the report is three facts and the backup
// control is offered beside it in every state.
//
// **Every call is guarded.** `navigator.storage` is absent in older WebKit and
// in some embedded browsers, `persisted()` and `estimate()` throw in a private
// window, and a door that died on either would take the local games with it.

/** What the door reports. `usage`/`quota` are 0 where the browser will not
 *  estimate — a browser that answers `persisted()` and not `estimate()` still
 *  knows the one thing the line is mostly about. */
export interface StorageReport {
  readonly persisted: boolean;
  readonly usage: number;
  readonly quota: number;
}

/** The half of `navigator.storage` this module uses, as the narrowest thing it
 *  reads — jsdom has none of it, so a module reaching for the real type could
 *  only ever be tested by not running. */
interface StorageManagerLike {
  persist?: () => Promise<boolean>;
  persisted?: () => Promise<boolean>;
  estimate?: () => Promise<{ usage?: number; quota?: number }>;
}

function manager(): StorageManagerLike | null {
  try {
    const n = globalThis.navigator as unknown as { storage?: StorageManagerLike } | undefined;
    return n?.storage ?? null;
  } catch {
    return null;
  }
}

/**
 * Ask the browser to keep this origin's storage.
 *
 * **Never throws and never reports.** A refusal, an absent API and a browser
 * that prompts and is told no are the same thing from here: the answer the
 * door draws comes from [`storageReport`] afterwards, which asks the browser
 * what it actually decided.
 */
export async function requestPersistence(): Promise<void> {
  try {
    await manager()?.persist?.();
  } catch {
    /* a browser that will not be asked is a browser that has not agreed */
  }
  // **Whatever it answered**, because the door's line is built from
  // `persisted()` and not from this call: the only thing known here is that
  // the standing answer may have moved, and re-reading is how anyone finds
  // out. Told after the await, so a subscriber that re-reads gets the new
  // answer and not the old one.
  for (const f of [...askedListeners]) f();
}

/** Told once the browser has been ASKED to keep this origin, whichever way it
 *  answered — the cue to read the report again (review M-5).
 *
 *  The ask happens at the first committed move, on the game screen; the line
 *  that reports it is on the door. Without this, a grant obtained mid-session
 *  did not reach the line until the section remounted, so a player coming back
 *  to the door could still read "has not promised" about a browser that had.
 *
 *  Returns its own removal. */
export function onPersistenceAsked(f: () => void): () => void {
  askedListeners.add(f);
  return () => askedListeners.delete(f);
}

const askedListeners = new Set<() => void>();

/** Asked once on this page already. A module flag, because the caller is a
 *  move and there are many moves. */
let asked = false;

/**
 * [`requestPersistence`], at most once for the life of this page.
 *
 * **The flag is set BEFORE the await**, which is the whole of why this is not
 * two lines at the call site: a queued run of moves reaches here in one tick,
 * and a flag set afterwards would let every one of them through.
 */
export async function requestPersistenceOnce(): Promise<void> {
  if (asked) return;
  asked = true;
  await requestPersistence();
}

/**
 * The three facts the door reports, or `null` where the browser will not say.
 *
 * `null` is the "unavailable" state and is NOT the same as `persisted: false`:
 * one is a browser that has not promised, the other is a browser that has no
 * opinion to give. They read differently on screen for that reason.
 */
export async function storageReport(): Promise<StorageReport | null> {
  const m = manager();
  if (!m?.persisted) return null;
  let persisted: boolean;
  try {
    persisted = (await m.persisted()) === true;
  } catch {
    // It threw: this browser will not say. A private window does exactly this.
    return null;
  }
  let usage = 0;
  let quota = 0;
  try {
    const e = await m.estimate?.();
    usage = Number(e?.usage ?? 0);
    quota = Number(e?.quota ?? 0);
  } catch {
    // The sizes are the softer half of the line; the grant still reports.
  }
  return { persisted, usage, quota };
}

/** A byte count a player reads, in the units a browser's own settings use
 *  (decimal, so a 2 GB quota reads as 2 GB). Two significant figures at most:
 *  the number is context for a decision, not a measurement. */
export function humanBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = n;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  // A decimal only where it says something. "1.2 GB" is worth a digit;
  // "54.1 KB" and "2.0 GB" are noise in a line whose job is to give a player a
  // sense of scale.
  const rounded = unit === 0 || value >= 10 ? Math.round(value) : Math.round(value * 10) / 10;
  const text = Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
  return `${text} ${units[unit]}`;
}

/** For the CELLS: the once-flag is a page's state with a page's lifetime, and
 *  one module registry serves a whole test file. */
export function resetPersistenceRequestForTests(): void {
  asked = false;
}
