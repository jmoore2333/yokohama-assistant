// Which chat panels this browser has OPENED (M6.4, R-6.4.10): ONE localStorage
// key holding a set of scopes — the `sections.ts` single-key idiom, never a key
// per table — newest 64 kept. Folded is the default everywhere (the panel is a
// bonus and play is the main attraction), so a storage that throws reads as
// folded and loses nothing.
//
// **The accessor moved to `browserStorage.ts` (M6.5b2, review M-8)**: this
// file's copy and `install.ts`'s were the same four lines and had diverged, so
// there is one of them now and the null handling travels with it.

import { browserStorage, readItem, writeItem } from "./browserStorage";

export const OPEN_KEY = "yokohama.chat.open";
const KEEP = 64;

function openSet(storage: Storage | null): string[] {
  try {
    const raw = readItem(OPEN_KEY, storage);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((s): s is string => typeof s === "string") : [];
  } catch {
    // Garbage under the key: the panel opens folded, which is the default.
    return [];
  }
}

/** `true` unless this browser has opened the panel for this scope. */
export function loadFolded(scope: string, storage: Storage | null = browserStorage()): boolean {
  return !openSet(storage).includes(scope);
}

export function saveFolded(
  scope: string,
  folded: boolean,
  storage: Storage | null = browserStorage(),
): void {
  const set = openSet(storage).filter((s) => s !== scope);
  if (!folded) set.push(scope);
  // A per-browser convenience; losing it loses nothing, so the write's answer
  // is not read here. `install.ts` reads it, because a dismissal that did not
  // land still has to change the screen.
  writeItem(OPEN_KEY, JSON.stringify(set.slice(-KEEP)), storage);
}
