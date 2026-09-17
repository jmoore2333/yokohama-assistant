// Which chat panels this browser has OPENED (M6.4, R-6.4.10): ONE localStorage
// key holding a set of scopes — the `sections.ts` single-key idiom, never a key
// per table — newest 64 kept. Folded is the default everywhere (the panel is a
// bonus and play is the main attraction), so a storage that throws reads as
// folded and loses nothing.

export const OPEN_KEY = "yokohama.chat.open";
const KEEP = 64;

function browserStorage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

function openSet(storage: Storage | null): string[] {
  try {
    const raw = storage?.getItem(OPEN_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((s): s is string => typeof s === "string") : [];
  } catch {
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
  try {
    const set = openSet(storage).filter((s) => s !== scope);
    if (!folded) set.push(scope);
    storage?.setItem(OPEN_KEY, JSON.stringify(set.slice(-KEEP)));
  } catch {
    // A per-browser convenience; losing it loses nothing.
  }
}
