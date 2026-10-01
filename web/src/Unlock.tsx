// The unlock face (spec §5.3): the one screen between a link and a game you
// are here to SPECTATE.
//
// **Which face `#/t/<id>` draws is the SERVER's answer, not a guess.** `GET
// …/{id}/table` carries `can_watch`, which is true once this caller has
// presented the table's password (or the table has none) — so a table that is
// playing, holds no seat of yours and answers `can_watch: false` is the one
// state in which there is a password to ask for. `App`'s `open` asks the three
// questions in that order and puts this screen up; everything else falls
// through to the game, where a spectator's frame comes back with `seat: null`.
//
// **It is not `OpenTable`, and the difference is what is being asked for.**
// That screen offers a SEAT — a password and a seat card to sit in — and the
// person using it is joining a game. This one buys nothing but the right to
// read: no name is taken, nothing is written to the table, and what follows is
// the spectator screen. Two faces around one password box would be one
// component with two meanings.

import { useRef, useState } from "react";
import { api, ApiError } from "./api";
import { LobbyBand } from "./Band";
import { PasswordTag } from "./Chips";
import { TableFacts } from "./TableFacts";
import type { ReactNode } from "react";
import type { TableView } from "./types";
import { Refusal } from "./Refusal";

export function Unlock({
  game,
  table,
  onUnlocked,
  onLeave,
  who,
}: {
  /** The table, by id. */
  game: string;
  /** What `GET …/{id}/table` answered — the name, the size and the seats this
   *  screen states, so a person can tell whether it is the table they meant
   *  before they type anything. */
  table: TableView;
  /** The password was taken. `App` re-opens the table, whose `can_watch` has
   *  turned over, and the status picks the screen — deliberately not "show the
   *  game", which would be the wrong screen for a table that has meanwhile
   *  ended or been abandoned. */
  onUnlocked: () => void;
  /** Back to the lobby. */
  onLeave: () => void;
  /** The band's corner, built by `App` (the identity corner, `serverCorner`)
   *  and merely placed here — the open table's arrangement. */
  who?: ReactNode;
}) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // **A REF decides whether the request happens; the state only disables the
  // button.** React batches, so two clicks in one tick both read `busy` as
  // false and both post — the reasoning `App`'s `inFlight` and `OpenTable`'s
  // own guard carry, and the second request here can only race the hand-over
  // the first is about to make.
  const inFlight = useRef(false);

  const submit = async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    setError(null);
    try {
      await api.unlock(game, password.trim());
      onUnlocked();
    } catch (e) {
      // The server's own sentence — "wrong password for this table" — and the
      // screen stays put, because the next thing a person does is type it
      // again. A 401 needs nothing here: `api.ts` moves the door from inside
      // `call`, before this sees the refusal (ruling R-7.1).
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      inFlight.current = false;
      setBusy(false);
    }
  };

  return (
    <div className="band-screen">
      <LobbyBand crumbs={["Lobby", table.name ?? table.id]} corner={who} />
      <div className="setup unlock">
        <div className="title-row">
          <h3 className="page-title">{table.name ?? table.id}</h3>
          <PasswordTag />
        </div>
        {/* The same line every other list draws about a table, so this table
            cannot read one way here and another in the lobby. */}
        <p className="lede">
          <TableFacts table={table} /> &middot; password required to spectate
        </p>
        <div className="spectate-box">
          <div className="form-field">
            <label htmlFor="spectate-password">Table password</label>
            <input
              id="spectate-password"
              type="password"
              autoComplete="off"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
          </div>
          {/* Disabled on an EMPTY box as well as while busy: a locked table
              answers a blank password with the same 403 as a wrong one, so a
              live button there spends a request to be told what this screen
              knows. */}
          <button
            type="button"
            className="primary"
            disabled={busy || !password.trim()}
            onClick={() => void submit()}
          >
            Spectate
          </button>
        </div>
        <button type="button" className="quiet-link" onClick={onLeave}>
          <span aria-hidden="true">&larr; </span>Lobby
        </button>
        {error && <Refusal sentence={error} />}
      </div>
    </div>
  );
}
