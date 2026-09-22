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
import { api, ApiError, ensureArchiveFact, isLocal, useArchiveFact } from "./api";
import { backupOne, download } from "./backup";
import { localPair } from "./local/client";
import { archiveDisposition } from "./outbox";
import type { TableView } from "./types";

/** What a local abandon says when the send never reached the server
 *  (R-6.5b.7). The SERVER's own rule for a failed put, applied at the client —
 *  and a sentence of this client's own, because a fetch that did not land has
 *  no sentence to carry. */
export const UNREACHABLE = "the archive could not be reached — the game was kept";

/** What a player reads when this browser cannot find out whether there is
 *  anywhere to save the game (M6.5b2, R-6.5b2.13; spec §2.6, §4).
 *
 *  **It refuses and offers, rather than choosing for them.** Throwing a game
 *  away must not happen implicitly when the thing that would have saved it was
 *  unreachable — and "unknown" is not "off": a browser that has never been told
 *  anything has no grounds for the claim that this deployment keeps no
 *  archive. */
export const CANNOT_ASK =
  "this browser cannot reach the server, so there is nowhere to save this game yet.";

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
  /** This browser could not learn whether there is an archive, and the player
   *  is being offered the two choices (R-6.5b2.13). A THIRD face, not a mode of
   *  the confirm one: what it asks is a different question. */
  const [choices, setChoices] = useState(false);
  /** What the SERVER keeps, as this browser last heard it — `"unknown"`
   *  included (M6.5b2).
   *
   *  **The hook and not a module read** (review M-9): the fact is filled at
   *  moments a render has already happened at, so a value read during render
   *  would leave this label saying whatever it mounted with, and `App`'s own
   *  copy of the same fact was a second reader kept in step by hand. */
  const archive = useArchiveFact();
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
    archiveDisposition(archive) === "send" &&
    table.status === "playing" &&
    (local || !isLocal(table.id));

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

  /** The DELETE, and what follows it. Reached from the confirm face once the
   *  disposition licenses it, and from "Abandon without saving". */
  const remove = async () => {
    await api.remove(table.id);
    setChoices(false);
    setConfirming(false);
    await onDone();
  };

  /**
   * The second click (R-6.5b2.13, spec §2.6) — and for a LOCAL game it now asks
   * the server what it keeps BEFORE doing anything it cannot undo.
   *
   * **Three states, and the middle one is the whole finding** (spec §0.3): a
   * deployment KNOWN to keep no archive deletes as it always has; one known to
   * keep an archive sends first and keeps the game if the send fails, which is
   * unchanged; and one this browser has never been told about REFUSES and
   * offers two named choices.
   *
   * **The read is FORCED, however fresh the remembered answer is** (spec §2.2).
   * A remembered `"off"` is the last thing this deployment said, and a
   * deployment that turned its archive on since would otherwise have this
   * browser deleting finished games for an hour. It is bounded, and falls back
   * to what is remembered only when nothing answers at all.
   *
   * **A SERVER table asks nothing**: the server archives it inside its own
   * delete, so there is no decision here to be wrong about.
   */
  const abandon = async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    try {
      if (local) {
        const disposition = archiveDisposition(await ensureArchiveFact({ force: true }));
        if (disposition === "ask") {
          setChoices(true);
          return;
        }
        if (disposition === "send" && !(await sendFirst())) return;
      }
      await remove();
    } catch (e) {
      onError(e instanceof ApiError ? e.message : String(e));
    } finally {
      // **Released on failure as well**, so a refusal leaves a control that can
      // be tried again rather than one that has quietly stopped working.
      inFlight.current = false;
    }
  };

  /** Write the game to a file instead — the choice that keeps it (spec §2.5's
   *  envelope, which is what this app restores). A separate guard from the
   *  delete's: a backup in flight must not stop the player abandoning, and two
   *  clicks would otherwise write two identical files. */
  const saving = useRef(false);
  const backUp = async () => {
    if (saving.current) return;
    saving.current = true;
    try {
      // **`.backup.json`, which is `BackUpOne`'s name and not the RECORD's**
      // (whole-branch review, false sentence 10 / Minor 3). This writes the
      // ENVELOPE — the file this app restores — and it was named exactly as
      // "Save game file" names the raw record the terminal loads, so the one
      // control on the refusal path produced a file a player could not tell
      // from the other by name and the terminal would have refused.
      download(`${table.id}.backup.json`, await backupOne(table.id));
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    } finally {
      saving.current = false;
    }
  };

  /** The other choice, and it is the one with no undo. **A third deliberate
   *  click**: the player has already asked to abandon, confirmed it, read a
   *  refusal naming what could not be reached, and is now saying it anyway. */
  const abandonAnyway = async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    try {
      await remove();
    } catch (e) {
      onError(e instanceof ApiError ? e.message : String(e));
    } finally {
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
  if (choices) {
    return (
      <>
        {/* The sentence is part of the control and not the host's error strip:
            it is a QUESTION, and the two answers are the buttons beside it. */}
        <span className="meta abandon-ask" role="status">
          {CANNOT_ASK}
        </span>
        {/* **Both name the table**, which is this component's own rule: there
            is no undo, and a confirmation must never be about a row other than
            the one that opened it. */}
        <button
          type="button"
          className="ghost"
          aria-label={`back it up to a file: ${named}`}
          onClick={() => void backUp()}
        >
          Back it up to a file
        </button>
        <button
          type="button"
          className="ghost danger"
          aria-label={`abandon ${named} without saving`}
          onClick={() => void abandonAnyway()}
        >
          Abandon without saving
        </button>
        <button
          type="button"
          className="ghost"
          aria-label={`cancel abandoning ${named}`}
          onClick={() => {
            setChoices(false);
            setConfirming(false);
          }}
        >
          cancel
        </button>
      </>
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
