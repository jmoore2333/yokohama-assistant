// The session control (M5 spec §3.7): the current game, the other saved games,
// and a way back to the lobby that is not a page reload.
//
// The smallest thing that removes the reload, and deliberately no more: M6 is
// the multi-game layer (lobby, chat, another player's game) and it grows from
// this menu. Nothing is deleted from here; M6.2 puts that on the open table.

import { useRef, useState, type ReactNode } from "react";
import { api, ApiError } from "./api";
import { TableFacts } from "./TableFacts";
import type { ClientState, TableView } from "./types";

export function GameMenu({
  state,
  busy,
  onSwitch,
  onLobby,
  signOut,
}: {
  state: ClientState;
  /** A move is in flight. See the rows' `disabled` below. */
  busy: boolean;
  /** A row was chosen. **The ID, not the game** (M6.1): a table that is still
   *  dealing refuses `GET /api/games/{id}` with a 409, so this menu can no
   *  longer fetch the thing it is switching to — `App`'s `open` asks
   *  `GET …/table` first and draws the dealing screen or the game. It is
   *  awaited here so the rows stay disabled for the whole switch and a refusal
   *  still lands in this menu's own line. */
  onSwitch: (id: string) => void | Promise<void>;
  /** The way OUT of this game, and since M6.2 it goes to the lobby rather than
   *  straight to the form (spec §5.1). The lobby is where a player picks
   *  between their tables and the open ones, and its own "New table" button is
   *  one click further on — so a control that jumped past it would be the only
   *  route in the client that skipped the first screen. */
  onLobby: () => void;
  /** The door's sign-out control, built by `App` and merely PLACED here.
   *
   *  **It lives in this menu on the game screen, and beside the strip on the
   *  two screens that have no menu.** The banner is gated at one row at 1440 and
   *  1280, and the strip and the button together cost 269px of a 1240px content
   *  box — enough to break the line, measured. This menu is already this
   *  screen's session control, so the control goes in it and the FACT ("You are
   *  …") stays in the banner where it is read at a glance. Absent under the two
   *  edge doors, where there is nothing stored to clear. */
  signOut?: ReactNode;
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
  const switchTo = async (id: string) => {
    if (switchInFlight.current) return;
    switchInFlight.current = true;
    setSwitching(true);
    try {
      await onSwitch(id);
    } catch (e) {
      setError(said(e));
    } finally {
      switchInFlight.current = false;
      setSwitching(false);
    }
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
        <button type="button" className="ghost" onClick={onLobby}>
          Lobby
        </button>
        <div className="label">Saved games</div>
        {saved === null && !error && <div className="muted">loading…</div>}
        {error && <div className="error">{error}</div>}
        {/* **Disabled while a move is in flight (final review I1).** A switch
            asked for mid-request is undone when the reply lands — measured —
            and the honest answer to "you cannot have both" is to let the move
            finish. `App` drops the stale reply as well; this is the half that
            keeps the player from asking for the state the guard has to throw
            away. **Lobby** stays live: it leaves for the lobby, where a reply
            for the game left behind is dropped by the same guard. */}
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
              onClick={() => void switchTo(t.id)}
            >
              {t.name ?? t.id}
            </button>
            <TableFacts table={t} />
          </div>
        ))}
        {saved !== null && others.length === 0 && <div className="muted">no other saved game</div>}
        {signOut && <div className="menu-door">{signOut}</div>}
      </div>
    </details>
  );
}
