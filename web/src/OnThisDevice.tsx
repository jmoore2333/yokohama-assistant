// The games in THIS BROWSER, and the button that starts one (R-6.5b.8).
//
// **The first section of the first screen, and it renders before the server
// has said anything.** Its rows come from the local dispatch's own
// `GET /api/games?scope=mine` — the module in this tab's Worker, over this
// browser's IndexedDB — so the guarantee R-6.5b.2 makes is architectural
// rather than a fallback: there is no reachable API in the path that draws
// this, and the cells prove it with every `fetch` failing.
//
// **The wording is the lobby's own**, drawn by `lobbyRow`, because a game is a
// game: "your turn", "over · you won", the name or the id, the link to
// `#/t/<id>`. What it does NOT draw is an abandon control (P-6) — a local game
// is abandoned on its own screen, where this tab holds its lock.

import { useCallback, useEffect, useState } from "react";
import { api, ApiError } from "./api";
import { unplayableIds } from "./backup";
import { BackUpEverything, BackUpOne, Restore, Unplayable } from "./BackupControls";
import { lobbyRow } from "./Lobby";
import { onLocalFault, onLocalWrite } from "./local/client";
import { navigate } from "./route";
import type { TableView } from "./types";

export function OnThisDevice() {
  const [games, setGames] = useState<TableView[] | null>(null);
  /** Why there are no rows, when the reason is the HOST and not the store.
   *
   *  **A visible line and never an empty list** (spec §2.3). A Worker that
   *  fails to start, a module that will not load and an IndexedDB that will
   *  not open all end here, and "no game on this device yet" would be a false
   *  sentence in every one of those cases — the games are there and this
   *  browser cannot reach them, which is a different thing and is actionable
   *  (a private window, a blocked worker, a bad build). */
  const [fault, setFault] = useState<string | null>(null);

  /** The ids this device cannot open, so a game is listed ONCE (review M-2).
   *
   *  **A separate read from the listing's**, because it comes from a different
   *  place: the layer builds a row from the TABLE document, which a restored
   *  cross-rev pair may have in a shape this build reads perfectly well, while
   *  the thing that says the game will not open is this browser's own
   *  compatibility row. A failure to read it is not a failure to list — the
   *  games are still there — so it falls back to hiding nothing. */
  const [hidden, setHidden] = useState<Set<string>>(() => new Set());

  const read = useCallback(
    () =>
      Promise.all([
        api.list("mine", undefined, "local"),
        unplayableIds().catch(() => new Set<string>()),
      ])
        .then(([l, cannot]) => {
          setGames(l.games);
          setHidden(cannot);
          setFault(null);
        })
        .catch((e: unknown) => setFault(e instanceof ApiError ? e.message : String(e))),
    [],
  );

  useEffect(() => {
    void read();
    // **Every local write re-reads this list.** A create, a move and a delete
    // all change what a row says — whose turn it is, whether it is over — and
    // nothing else tells this section that they happened: a local game has no
    // signal, because the bot's trailing run comes back inside the reply that
    // caused it.
    return onLocalWrite(() => void read());
  }, [read]);

  // The fault arrives on its own channel as well as through a rejected read:
  // the Worker can die while nothing of this section's is in flight, and a
  // list drawn a minute ago would otherwise go on looking like the truth.
  useEffect(() => onLocalFault(setFault), []);

  /** Bumped when a restore has landed, so the unplayable list re-reads
   *  (R-6.5b2.12). The PLAYABLE list re-reads on its own: `localAdopt` fires
   *  the write handlers this section already listens to. */
  const [restored, setRestored] = useState(0);

  /** The rows a player can actually open. A game with a compatibility row is
   *  drawn by `Unplayable` below instead, with the reason it will not. */
  const playable = (games ?? []).filter((t) => !hidden.has(t.id));

  return (
    <section className="saved on-this-device">
      <h3>On this device</h3>
      <button
        type="button"
        className="primary"
        onClick={() => navigate({ kind: "new", local: true })}
      >
        Play the bot here
      </button>
      {fault !== null ? (
        <p className="muted local-fault">the games on this device could not be read &mdash; {fault}</p>
      ) : games === null ? (
        <p className="muted">loading&hellip;</p>
      ) : games.length === 0 ? (
        <p className="muted">no game on this device yet &mdash; play the bot here.</p>
      ) : playable.length === 0 ? (
        /* **Games, but none this build can open.** "No game on this device yet"
           would be false — they are here and they are listed, under "Will not
           open here" below with the reason. So this draws nothing and lets that
           section be the answer, rather than saying there is nothing. */
        null
      ) : (
        <ul>{playable.map((t) => lobbyRow(t, <BackUpOne id={t.id} />))}</ul>
      )}
      {/* **Offered in EVERY state above, including the fault one** (spec §2.5):
          a module that will not load is when you most need your games out, and
          `backupEverything` reads IndexedDB from this page rather than through
          the Worker for exactly that reason. */}
      <div className="backup-controls">
        <BackUpEverything />
        <Restore onDone={() => setRestored((n) => n + 1)} />
      </div>
      <Unplayable reload={restored} />
    </section>
  );
}
