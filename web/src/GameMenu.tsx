// The session control (M5 spec §3.7): the current game, the other saved games,
// and a way back to the lobby that is not a page reload.
//
// The smallest thing that removes the reload, and deliberately no more: M6 is
// the multi-game layer (lobby, chat, another player's game) and it grows from
// this menu. Nothing is deleted from here; M6.2 puts that on the open table.

import { useRef, useState, type ReactNode } from "react";
import { api, ApiError, type Host } from "./api";
import { unplayableIds } from "./backup";
import { isStranded } from "./tables";
import { TableFacts } from "./TableFacts";
import type { ClientState, TableView } from "./types";
import { Refusal } from "./Refusal";

export function GameMenu({
  state,
  busy,
  onSwitch,
  onLobby,
  leaveLabel = "Lobby",
  savedFrom,
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
   *  straight to the form (spec §5.1), or to the door in a build that serves
   *  no lobby (`leaveLabel`). The lobby is where a player picks
   *  between their tables and the open ones, and its own "New table" button is
   *  one click further on — so a control that jumped past it would be the only
   *  route in the client that skipped the first screen. */
  onLobby: () => void;
  /** What the way out SAYS (M6.5c2, spec §2.2): "Lobby", except on a local
   *  game in a build that serves no lobby, where `App` sends it to the door
   *  and says so. */
  leaveLabel?: string;
  /** Which host "Saved games" is read from. Absent is the SERVER, the lobby's
   *  own listing; `"local"` in a build that serves no lobby (the whole-branch
   *  review's Important 1), where the server has no game routes and a listing
   *  sent there came back as a 404 drawn under this list on every open. The
   *  local host answers the same `GamesList`, so the rows are this browser's
   *  games, from the listing the door's "On this device" reads. */
  savedFrom?: Host;
  /** The door's sign-out control, built by `App` and merely PLACED here.
   *
   *  **It lives in this menu on the game screen, and beside the strip on the
   *  two screens that have no menu.** The banner is gated at one row at 1440,
   *  1280 and (since cycle E) 1024, and the strip and the button together cost
   *  269px of a 1240px content box — enough to break the line, measured. This
   *  menu is already this screen's session control, so the control goes in it
   *  and the FACT ("You are …") stays in the banner where it is read at a
   *  glance. Absent under the two edge doors, where there is nothing stored to
   *  clear. */
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
  /** The menu itself — an uncontrolled `<details>`, whose `open` is the one
   *  thing a switch has to put back (see `switchTo`). */
  const menu = useRef<HTMLDetailsElement | null>(null);
  // The sentence to show. A 401 needs nothing here: `api.ts` moves the door
  // from inside `call`, before this ever sees the refusal (ruling R-7.1).
  const said = (e: unknown) => (e instanceof ApiError ? e.message : String(e));
  const load = () => {
    setError(null);
    const seq = (listing.current += 1);
    Promise.all([
      // The list is a list of TABLES since M6.1, scoped to the caller by the
      // server, and the rows say so: a table has a name, a size and something
      // it is waiting on. The id alone was all this menu could show when a
      // saved game was a file.
      api.list("mine", undefined, savedFrom),
      // **This browser's games from another revision**, which the door lists
      // under "Will not open here" by this browser's own compatibility rows
      // (`OnThisDevice`'s `hidden`) — a read of this browser's database, so
      // only when the rows ARE this browser's. It falls back to hiding
      // nothing, as the door's does: a failed read is not a failed listing.
      savedFrom === "local"
        ? unplayableIds().catch(() => new Set<string>())
        : Promise.resolve(new Set<string>()),
    ])
      // **A game offered here is a game that opens** (M6.5c2 cycle C; the fix
      // wave review's concern (a)): the door and the lobby set aside a game
      // this build cannot continue (`isStranded`), and the door one from
      // another revision; the menu offered both, each opening onto its
      // refusal. Neither is a game to switch to, so neither is listed.
      .then(
        ([l, cannot]) =>
          seq === listing.current && setSaved(l.games.filter((t) => !isStranded(t) && !cannot.has(t.id))),
      )
      .catch((e) => seq === listing.current && setError(said(e)));
  };
  const switchTo = async (id: string) => {
    if (switchInFlight.current) return;
    switchInFlight.current = true;
    setSwitching(true);
    try {
      await onSwitch(id);
      // **Closed once the switch has LANDED** (M6.5c2 cycle D, dossier row 2):
      // a switch between two games keeps this banner mounted, and the list was
      // left open over the game just chosen. A refusal keeps it open, because
      // the refusal is said on this menu's own line.
      if (menu.current) menu.current.open = false;
    } catch (e) {
      setError(said(e));
    } finally {
      switchInFlight.current = false;
      setSwitching(false);
    }
  };
  const others = (saved ?? []).filter((t) => t.id !== state.id);
  /** What the summary says: the mode, the game's id and whether it is saved. */
  const summaryWords = `${state.mode} \u00b7 ${state.id}${state.frame.title.autosaved ? " \u00b7 saved" : ""}`;
  return (
    <details
      ref={menu}
      className="cards-menu game-menu"
      onToggle={(e) => {
        if (e.currentTarget.open) load();
      }}
    >
      {/* **The words in a span of their own, and the whole of them its title**
          (cycle E, straw poll row 71): from 1000px to 1200px this summary is
          the banner's one child that gives way, ending in an ellipsis rather
          than breaking the row (`styles.css`, `.banner > .game-menu`), and an ellipsis needs
          a box of its own to end in. */}
      <summary className="ghost meta" title={summaryWords}>
        <span className="game-menu-name">{summaryWords}</span>
      </summary>
      <div>
        <button type="button" className="ghost" onClick={onLobby}>
          {leaveLabel}
        </button>
        <div className="label">Saved games</div>
        {saved === null && !error && <div className="muted">loading…</div>}
        {error && <Refusal sentence={error} />}
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
