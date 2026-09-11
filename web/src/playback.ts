// How fast the opponent's turn plays out (M5.5 spec §3.2.3), and where that
// choice lives: per BROWSER, in localStorage, the `sections.ts` idiom — a
// convenience, never state; a storage that throws reads as the default.

export const CADENCES = [1000, 2000, 3000] as const;
export type Cadence = (typeof CADENCES)[number];
export const DEFAULT_CADENCE: Cadence = 2000;
/** The dwell on a step whose frame is identical to the previous one — the
 *  bookkeeping events ("End additional-action phase", a skipped 5-Power). */
export const QUICK_DWELL = 400;

const KEY = "yokohama.playback.cadence";

function browserStorage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

export function loadCadence(storage: Storage | null = browserStorage()): Cadence {
  try {
    const n = Number(storage?.getItem(KEY));
    return (CADENCES as readonly number[]).includes(n) ? (n as Cadence) : DEFAULT_CADENCE;
  } catch {
    return DEFAULT_CADENCE;
  }
}

export function saveCadence(ms: Cadence, storage: Storage | null = browserStorage()): void {
  try {
    storage?.setItem(KEY, String(ms));
  } catch {
    // A per-browser convenience; losing it loses nothing.
  }
}
