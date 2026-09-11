// Which sections the table draws (M5 spec §3.4), and where that choice lives.
//
// The set persists per BROWSER in localStorage: a convenience, not state — a
// game opened elsewhere shows the default set, and a storage that throws (a
// private window, a blocked site) reads as the default and never as an error.

export const SECTION_KEYS = ["ports", "labs", "tracks", "achievements", "buildings"] as const;
export type SectionKey = (typeof SECTION_KEYS)[number];

export const SECTION_LABELS: Record<SectionKey, string> = {
  ports: "Port",
  labs: "Laboratory",
  tracks: "Tracks",
  achievements: "Goals",
  buildings: "Building sites",
};

export const DEFAULT_SECTIONS: readonly SectionKey[] = ["ports", "labs", "tracks", "achievements"];

const KEY = "yokohama.table.sections";

function browserStorage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

/** The enabled keys in REGISTRY order, unknown keys dropped. */
export function loadSections(storage: Storage | null = browserStorage()): SectionKey[] {
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return [...DEFAULT_SECTIONS];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [...DEFAULT_SECTIONS];
    return SECTION_KEYS.filter((k) => parsed.includes(k));
  } catch {
    return [...DEFAULT_SECTIONS];
  }
}

export function saveSections(
  keys: readonly SectionKey[],
  storage: Storage | null = browserStorage(),
): void {
  try {
    storage?.setItem(KEY, JSON.stringify(SECTION_KEYS.filter((k) => keys.includes(k))));
  } catch {
    // A per-browser convenience; losing it loses nothing.
  }
}
