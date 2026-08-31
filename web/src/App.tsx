import { useCallback, useEffect, useState } from "react";
import { api, ApiError } from "./api";
import { Board } from "./Board";
import { Dashboard } from "./Dashboard";
import { Decision } from "./Decision";
import { NewGame } from "./NewGame";
import type { ClientState, Labels, PlayerColor } from "./types";

export function App() {
  const [labels, setLabels] = useState<Labels | null>(null);
  const [state, setState] = useState<ClientState | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.labels().then(setLabels).catch(() => setLabels(null));
  }, []);

  const send = useCallback(
    async (line: string) => {
      if (!state) return;
      setBusy(true);
      setError(null);
      try {
        setState(await api.input(state.id, line));
      } catch (e) {
        setError(e instanceof ApiError ? e.message : String(e));
      } finally {
        setBusy(false);
      }
    },
    [state],
  );

  if (!state) return <NewGame onStarted={setState} />;

  const players = state.frame.dashboard.find((p) => "Players" in p);
  const colors: PlayerColor[] =
    players && "Players" in players ? players.Players.players.map((p) => p.color) : [];

  // The board is an input surface only while a placement picker is open — the
  // one place the engine is asking "which areas", rather than "which option".
  const picking = state.prompt.kind === "place_a" || state.prompt.kind === "place_b";
  const selected =
    state.prompt.kind === "place_a"
      ? state.prompt.selected
      : state.prompt.kind === "place_b" && state.prompt.selected != null
        ? [state.prompt.selected]
        : [];

  return (
    <div className="app">
      <header className="banner">
        <h1>Yokohama</h1>
        <span className="meta">
          {state.mode} &middot; seed {state.frame.title.seed} &middot; {state.id}
          {state.frame.title.autosaved ? " · saved" : ""}
        </span>
        <span className="meta" style={{ marginLeft: "auto" }}>
          {state.over ? "finished" : state.your_turn ? "your turn" : `seat ${state.to_move} thinking`}
        </span>
      </header>

      <div className="stage">
        <div className="board-col">
          <Board
            board={state.frame.board}
            labels={labels}
            colors={colors}
            selected={selected}
            {...(picking
              ? { onTileClick: (area: number) => void send(`A${String(area).padStart(2, "0")}`) }
              : {})}
          />
        </div>
        <aside className="rail">
          <Dashboard panels={state.frame.dashboard} labels={labels} />
        </aside>
      </div>

      <Decision state={state} labels={labels} busy={busy} onSend={(l) => void send(l)} />
      {error && (
        <div className="decision">
          <div className="error">{error}</div>
        </div>
      )}
    </div>
  );
}
