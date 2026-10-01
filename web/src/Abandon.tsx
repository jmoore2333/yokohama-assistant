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
import { api, ApiError, archiveKind, isLocal } from "./api";
import { localPair } from "./local/client";
import type { TableView } from "./types";

/** What a local abandon says when the send never reached the server
 *  (R-6.5b.7). The SERVER's own rule for a failed put, applied at the client —
 *  and a sentence of this client's own, because a fetch that did not land has
 *  no sentence to carry. */
export const UNREACHABLE = "the archive could not be reached — the game was kept";

export function Abandon({
  table,
  local = false,
  onDone,
  onError,
}: {
  /** The table to abandon. The whole row is taken rather than an id and a name,
   *  so the button's words and the row's words are read off one object. */
  table: TableView;
  /** This game lives in THIS BROWSER (M6.5b, R-6.5b.6). Abandoning it sends
   *  the record and its table document to the server's archive BEFORE the
   *  local delete, because a game on this device exists nowhere else. */
  local?: boolean;
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
  /** Whether abandoning THIS table results in a save — which is what the label
   *  is allowed to promise.
   *
   *  **Who does the saving differs by host, and so does the answer.** A server
   *  table is archived by the SERVER inside its own delete, so the label reads
   *  the deployment's archive and nothing else. A game in this browser is
   *  archived by this control, which only happens under `local` — the one
   *  caller that withholds it is the screen for a save that no longer
   *  rebuilds, where the archive would refuse the record and a control that
   *  said "and save" would promise a copy nobody can make. */
  const saves =
    archiveKind() !== "off" && table.status === "playing" && (local || !isLocal(table.id));

  /** Send the pair to the SERVER's archive, and say whether the delete may go
   *  ahead (R-6.5b.6, spec §4).
   *
   *  **A resolved call is the only licence to delete**, and that includes the
   *  server's `identical`: 201 means the copy landed now, 200 `identical`
   *  means an earlier send landed and only the delete failed, and both are a
   *  complete copy at a path the server names. Every refusal KEEPS the game —
   *  the 409 included, which says a DIFFERENT record is archived under this id
   *  and is therefore no evidence at all that THIS one is safe. */
  const sendFirst = async (): Promise<boolean> => {
    // **One snapshot of both documents** (spec §2.2): two queued reads can
    // fall either side of a move, and the server checks that the record and
    // the table agree.
    const pair = await localPair(table.id);
    // Nothing to send is not a refusal: a table with no record is a game with
    // no moves in it, and the archive has no use for one. The delete is then
    // what it would be on a server that keeps no archive.
    if (pair.record === null || pair.table === null) return true;
    try {
      await api.archive(pair.record, pair.table);
      return true;
    } catch (e) {
      // The server's own sentence where it gave one; this client's where the
      // request never got an answer at all.
      onError(e instanceof ApiError ? e.message : UNREACHABLE);
      return false;
    }
  };

  const abandon = async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    try {
      if (local && archiveKind() !== "off" && !(await sendFirst())) return;
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
        {/* **"and save" is the near truth** (M6.5a, R-6.5a.13): the server
            archives a PLAYING table with at least one event when its archive is
            on, and `TableView` carries no event count — so a table dealt a
            moment ago reads "and save" and saves nothing. The server's rule is
            the truth; adding a count to the view would move eight fixtures for
            one word. */}
        {saves ? "Abandon and save" : "Abandon"}
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
