import { useCallback, useEffect, useState } from "react";
import { api, ApiError } from "./api";
import { Board } from "./Board";
import { Dashboard } from "./Dashboard";
import { Decision } from "./Decision";
import { NewGame } from "./NewGame";
import { Surface, SURFACES, type SurfaceName } from "./Surfaces";
import type { ArtManifest, ClientState, Labels, PlayerColor } from "./types";

export function App() {
  const [labels, setLabels] = useState<Labels | null>(null);
  const [art, setArt] = useState<ArtManifest | null>(null);
  const [state, setState] = useState<ClientState | null>(null);
  const [surface, setSurface] = useState<SurfaceName | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.labels().then(setLabels).catch(() => setLabels(null));
    // A 404 here means the server has no cropped art. That is a supported
    // state, not a failure: every card falls back to its typed values, which is
    // exactly the surface that existed before there was any art.
    api.art().then(setArt).catch(() => setArt(null));
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

  if (!state) return <NewGame art={art} onStarted={setState} />;

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
          {state.mode} &middot; seed {state.seed} &middot; {state.id}
          {state.frame.title.autosaved ? " · saved" : ""}
        </span>
        <nav className="views">
          {SURFACES.map((s) => (
            <button
              type="button"
              key={s.key}
              className={`ghost ${surface === s.key ? "on" : ""}`}
              onClick={() => setSurface(surface === s.key ? null : s.key)}
            >
              {s.label}
            </button>
          ))}
        </nav>
        <span className="meta turn">
          {state.over ? "finished" : state.your_turn ? "your turn" : `seat ${state.to_move} thinking`}
        </span>
      </header>

      {/* How close the game is to ending, in the CLI's own words. Without it a
          player cannot see the game ending, which is not a polish issue. */}
      <div className={`end-strip ${state.end_game_triggered ? "final" : ""}`}>
        {state.end_triggers}
      </div>

      {state.warnings.length > 0 && (
        <div className="decision">
          {state.warnings.map((w, i) => (
            <div className="warn" key={i}>
              {w}
            </div>
          ))}
        </div>
      )}

      <div className="stage">
        <div className="board-col">
          <Board
            board={state.frame.board}
            art={art}
            labels={labels}
            colors={colors}
            selected={selected}
            {...(picking
              ? { onTileClick: (area: number) => void send(`A${String(area).padStart(2, "0")}`) }
              : {})}
          />
        </div>
        <aside className="rail">
          <Dashboard panels={state.frame.dashboard} art={art} labels={labels} colors={colors} />
        </aside>
      </div>

      {surface && (
        <Surface
          name={surface}
          game={state.id}
          art={art}
          labels={labels}
          colors={colors}
          onClose={() => setSurface(null)}
        />
      )}

      <Decision state={state} labels={labels} busy={busy} onSend={(l) => void send(l)} />
      {error && (
        <div className="decision">
          <div className="error">{error}</div>
        </div>
      )}
    </div>
  );
}
