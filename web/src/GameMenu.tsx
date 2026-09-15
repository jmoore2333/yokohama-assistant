// The session control (M5 spec §3.7): the current game, the other saved games,
// and a way back to the setup screen that is not a page reload.
//
// The smallest thing that removes the reload, and deliberately no more: M6 is
// the multi-game layer (lobby, chat, another player's game) and it grows from
// this menu. Nothing is deleted from here; the setup screen already does that.

import { useRef, useState } from "react";
import { api, ApiError } from "./api";
import { TableFacts } from "./TableFacts";
import type { ClientState, TableView } from "./types";

export function GameMenu({
  state,
  busy,
  onSwitch,
  onNew,
}: {
  state: ClientState;
  /** A move is in flight. See the rows' `disabled` below. */
  busy: boolean;
  onSwitch: (s: ClientState) => void;
  onNew: () => void;
}) {
  const [saved, setSaved] = useState<TableView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  // A switch already asked for. Two rows clicked in the same breath would
  // otherwise both `GET`, and the one that answered LAST would win — which is
  // not the one clicked last.
  //
  // A REF decides whether the request happens, and the state only disables the
  // rows — `App`'s `inFlight` comment is the reasoning and this is the same
  // shape: React batches, so two handlers running before a re-render both read
  // `switching === false` and both fetch.
  const switchInFlight = useRef(false);
  const [switching, setSwitching] = useState(false);
  // Which listing is the current one. Opening and closing the menu twice sends
  // two `GET /api/games`, and the first to answer is not the first sent; a
  // stale list is one that offers a game already deleted.
  const listing = useRef(0);
  // The sentence to show. A 401 needs nothing here: `api.ts` moves the door
  // from inside `call`, before this ever sees the refusal (ruling R-7.1).
  const said = (e: unknown) => (e instanceof ApiError ? e.message : String(e));
  const load = () => {
    setError(null);
    const seq = (listing.current += 1);
    api
      // The list is a list of TABLES since M6.1, scoped to the caller by the
      // server, and the rows say so: a table has a name, a size and something
      // it is waiting on. The id alone was all this menu could show when a
      // saved game was a file.
      .list()
      .then((l) => seq === listing.current && setSaved(l.games))
      .catch((e) => seq === listing.current && setError(said(e)));
  };
  const switchTo = (id: string) => {
    if (switchInFlight.current) return;
    switchInFlight.current = true;
    setSwitching(true);
    api
      .get(id)
      .then(onSwitch)
      .catch((e) => setError(said(e)))
      .finally(() => {
        switchInFlight.current = false;
        setSwitching(false);
      });
  };
  const others = (saved ?? []).filter((t) => t.id !== state.id);
  return (
    <details
      className="cards-menu game-menu"
      onToggle={(e) => {
        if (e.currentTarget.open) load();
      }}
    >
      <summary className="ghost meta">
        {state.mode} &middot; {state.id}
        {state.frame.title.autosaved ? " · saved" : ""}
      </summary>
      <div>
        <button type="button" className="ghost" onClick={onNew}>
          New game
        </button>
        <div className="label">Saved games</div>
        {saved === null && !error && <div className="muted">loading…</div>}
        {error && <div className="error">{error}</div>}
        {/* **Disabled while a move is in flight (final review I1).** A switch
            asked for mid-request is undone when the reply lands — measured —
            and the honest answer to "you cannot have both" is to let the move
            finish. `App` drops the stale reply as well; this is the half that
            keeps the player from asking for the state the guard has to throw
            away. **New game** stays live: it leaves for the setup screen, where
            a reply for the game left behind is dropped by the same guard. */}
        {/* **A row is the table, not its id.** The button is what it is CALLED
            — the name whoever opened it typed, and the id when nobody typed one
            — and `TableFacts` is the rest: the size, what the table is waiting
            on, and who is at it. Shared with the setup screen's list, so the
            same table cannot read two different ways depending on which of the
            two you are looking at. */}
        {others.map((t) => (
          <div className="game-row" key={t.id}>
            <button
              type="button"
              className="ghost"
              disabled={busy || switching}
              onClick={() => switchTo(t.id)}
            >
              {t.name ?? t.id}
            </button>
            <TableFacts table={t} />
          </div>
        ))}
        {saved !== null && others.length === 0 && <div className="muted">no other saved game</div>}
      </div>
    </details>
  );
}
