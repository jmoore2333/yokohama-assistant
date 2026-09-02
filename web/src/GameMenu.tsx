// The session control (M5 spec §3.7): the current game, the other saved games,
// and a way back to the setup screen that is not a page reload.
//
// The smallest thing that removes the reload, and deliberately no more: M6 is
// the multi-game layer (lobby, chat, another player's game) and it grows from
// this menu. Nothing is deleted from here; the setup screen already does that.

import { useState } from "react";
import { api, ApiError } from "./api";
import type { ClientState } from "./types";

export function GameMenu({
  state,
  onSwitch,
  onNew,
}: {
  state: ClientState;
  onSwitch: (s: ClientState) => void;
  onNew: () => void;
}) {
  const [saved, setSaved] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const said = (e: unknown) => (e instanceof ApiError ? e.message : String(e));
  const load = () => {
    setError(null);
    api.list().then(setSaved).catch((e) => setError(said(e)));
  };
  const others = (saved ?? []).filter((id) => id !== state.id);
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
        {others.map((id) => (
          <button
            type="button"
            className="ghost"
            key={id}
            onClick={() => api.get(id).then(onSwitch).catch((e) => setError(said(e)))}
          >
            {id}
          </button>
        ))}
        {saved !== null && others.length === 0 && <div className="muted">no other saved game</div>}
      </div>
    </details>
  );
}
