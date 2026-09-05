// The banner's board switches, in one place (M5.5 ruling R-T12-2).
//
// **Extracted rather than copied.** The set table wants the same menu the game
// banner has — spec §3.3.3 asks for it "so a player can read a site card at
// full size before choosing". A second copy of the JSX would drift on the next
// label, and a second copy of the toggle would drift on the next storage rule.
//
// **The state hook lives here too, next to the component that renders it** —
// three lines that are the whole of "toggle it and remember it", while
// `sections.ts` owns the storage and stays a React-free module its own cells
// can drive with a fake `Storage`.
//
// **CALL IT ONCE PER TREE (R-T12-4).** It seeds from storage at MOUNT, so two
// callers are two independent copies that agree only until one of them is
// toggled. `App` calls it and passes the pair down to `NewGame`; that is what
// makes "a board switched off during setup is still off when the game opens"
// true, and it was FALSE for one commit when the set table called it itself —
// storage was written, the set table redrew, and `App`, which never remounts,
// handed the game the array it had read at start-up.

import { useState } from "react";
import { loadSections, saveSections, SECTION_KEYS, SECTION_LABELS, type SectionKey } from "./sections";

/** The enabled set and a toggle that persists it. */
export function useSections(): [SectionKey[], (k: SectionKey) => void] {
  const [enabled, setEnabled] = useState<SectionKey[]>(() => loadSections());
  const toggle = (k: SectionKey) =>
    setEnabled((e) => {
      const next = e.includes(k) ? e.filter((x) => x !== k) : [...e, k];
      saveSections(next);
      return next;
    });
  return [enabled, toggle];
}

/** **The section toggles, and they are a DISCLOSURE for a measured reason.**
 *  Eleven inline buttons wrapped the banner to a second row below 1280 at M3.5,
 *  and `the_screen_itself.spec.ts`'s banner cell pins its height to one row at
 *  1440 and 1280; five more buttons would wrap it again. The summary carries
 *  the count, so the state is readable with the menu shut.
 *
 *  **Escape closes it (ruling R-T12a).** An uncontrolled `<details>` ignores
 *  Escape — the key is the browser's affordance for a `<select>` and a
 *  `<dialog>`, not for a disclosure — so a menu left open sits over the table.
 *  Clicking the summary still toggles; this only adds the key. */
export function BoardsMenu({
  enabled,
  onToggle,
}: {
  enabled: SectionKey[];
  onToggle: (k: SectionKey) => void;
}) {
  return (
    <details
      className="cards-menu boards-menu"
      onKeyDown={(e) => {
        if (e.key === "Escape") e.currentTarget.removeAttribute("open");
      }}
    >
      <summary className="ghost">Boards &middot; {enabled.length} on</summary>
      <div>
        {SECTION_KEYS.map((k) => (
          <button
            type="button"
            key={k}
            className={`ghost ${enabled.includes(k) ? "on" : ""}`}
            aria-pressed={enabled.includes(k)}
            onClick={() => onToggle(k)}
          >
            {SECTION_LABELS[k]}
          </button>
        ))}
      </div>
    </details>
  );
}
