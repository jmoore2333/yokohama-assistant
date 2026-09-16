// Abandoning a table (R-13), in ONE place.
//
// **Two clicks, and both buttons NAME the table.** The shape is M5's final
// review's: `delete` beside each saved game removed the record on a single
// click, and by M5 the Game menu had put a running game one click away from
// that list — so a stray click beside the table you meant to open took a
// different one. There is no undo. The name is in the accessible name of every
// one of the three buttons, so a confirmation can never be about a row other
// than the one that opened it.
//
// **One component, because the two copies had already drifted.** The lobby's
// row and the open table's waiting face each carried this control, and only one
// of them guarded its DELETE against a second click — a difference neither
// file's cells could see, because each clicked confirm exactly once. The guard
// lives here now, and `Abandon.test.tsx` holds it with a double click.

import { useRef, useState } from "react";
import { api, ApiError } from "./api";
import type { TableView } from "./types";

export function Abandon({
  table,
  onDone,
  onError,
}: {
  /** The table to abandon. The whole row is taken rather than an id and a name,
   *  so the button's words and the row's words are read off one object. */
  table: TableView;
  /** The DELETE succeeded. What follows is the host's: the lobby re-reads its
   *  listings, the open table leaves for the lobby. **Awaited inside the
   *  attempt**, so a host whose follow-up fails reports through `onError`
   *  rather than into an unhandled rejection. */
  onDone: () => void | Promise<void>;
  /** A sentence for the host's own error strip.
   *
   *  Reported rather than rendered here, because both hosts already have a
   *  strip and a second one beside this button would put two refusals on one
   *  screen — in the lobby's case, one inside a list row. */
  onError: (sentence: string) => void;
}) {
  const [confirming, setConfirming] = useState(false);
  // The guard against a SECOND DELETE going out while the first is in flight —
  // a ref and not a state, for the reason every other guard in this client is
  // one: React batches, so two clicks in the same tick both read a state flag
  // as false and both post. The second then 404s and puts a refusal on a screen
  // the first is already navigating away from.
  const inFlight = useRef(false);
  const named = table.name ?? table.id;

  const abandon = async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    try {
      await api.remove(table.id);
      setConfirming(false);
      await onDone();
    } catch (e) {
      onError(e instanceof ApiError ? e.message : String(e));
    } finally {
      // **Released on failure as well**, so a refusal leaves a control that can
      // be tried again rather than one that has quietly stopped working.
      inFlight.current = false;
    }
  };

  if (!confirming) {
    return (
      <button
        type="button"
        className="ghost danger"
        aria-label={`abandon ${named}`}
        onClick={() => setConfirming(true)}
      >
        Abandon
      </button>
    );
  }
  return (
    <>
      <button
        type="button"
        className="ghost danger"
        aria-label={`confirm abandon ${named}`}
        onClick={() => void abandon()}
      >
        confirm abandon
      </button>
      <button
        type="button"
        className="ghost"
        aria-label={`cancel abandoning ${named}`}
        onClick={() => setConfirming(false)}
      >
        cancel
      </button>
    </>
  );
}
