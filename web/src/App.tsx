import { useCallback, useEffect, useRef, useState } from "react";
import { api, ApiError } from "./api";
import { Board } from "./Board";
import { Dashboard } from "./Dashboard";
import { Decision } from "./Decision";
import { NewGame } from "./NewGame";
import { CARD_SURFACES, Surface, SURFACES, type SurfaceName } from "./Surfaces";
import type { ArtManifest, ClientState, Labels, PlayerColor } from "./types";

export function App() {
  const [labels, setLabels] = useState<Labels | null>(null);
  const [art, setArt] = useState<ArtManifest | null>(null);
  const [state, setState] = useState<ClientState | null>(null);
  const [surface, setSurface] = useState<SurfaceName | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // The guard against a SECOND request going out while the first is in flight.
  //
  // A ref and not the `busy` state, because state is what makes the bug: React
  // batches, so two clicks in the same tick both read `busy === false` and both
  // post. Two `/input` posts are then applied in whatever order the server's
  // per-game mutex takes them, and whichever RESPONSE settles last becomes the
  // client's truth. Measured on a double-clicked placement toggle: the client
  // showed "Selected: A03" while the server held an empty selection, and
  // Confirm then advanced the turn having placed nothing. `busy` still drives
  // the disabled styling; this decides whether the request happens at all.
  const inFlight = useRef(false);

  useEffect(() => {
    api.labels().then(setLabels).catch(() => setLabels(null));
    // A 404 here means the server has no cropped art. That is a supported
    // state, not a failure: every card falls back to its typed values, which is
    // exactly the surface that existed before there was any art.
    api.art().then(setArt).catch(() => setArt(null));
  }, []);

  const send = useCallback(
    async (line: string) => {
      if (!state || inFlight.current) return;
      inFlight.current = true;
      setBusy(true);
      setError(null);
      try {
        setState(await api.input(state.id, line, state.prompt.kind));
      } catch (e) {
        setError(e instanceof ApiError ? e.message : String(e));
      } finally {
        inFlight.current = false;
        setBusy(false);
      }
    },
    [state],
  );

  if (!state) return <NewGame art={art} onStarted={setState} />;

  const players = state.frame.dashboard.find((p) => "Players" in p);
  // The Church and Customs tracks are already on every frame, so the tracks
  // SURFACE reads them from here rather than from a route of its own.
  const tracksPanel = state.frame.dashboard.find((p) => "Tracks" in p);
  const tracks = tracksPanel && "Tracks" in tracksPanel ? tracksPanel.Tracks : null;
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
          {state.mode} &middot; {state.id}
          {state.frame.title.autosaved ? " · saved" : ""}
        </span>
        {/* Four surfaces inline, the five card references behind a disclosure,
            so the banner stays ONE ROW at every laptop width: measured 93 → 56px
            at 1440 and 174 → 93 at 1024, with the nav's bottom edge at 43px
            instead of 88/88/125 at 1440/1280/1024.

            **The benefit is the banner's height, and not what the review said
            it was.** Its argument was that the wrapped second row sat under an
            open panel, so you could not switch surfaces without pressing Escape
            first. That is true of the old panel and no longer reachable: the
            panel is a real `<dialog>` opened with `showModal()` now, so the
            whole page outside it is inert while it is open — which is what
            modality means, and is the containment the panel spent three
            milestones asserting without having. Escape then switch, by design.
            Written down because the tempting sentence is the review's, and it
            stopped being true one commit earlier.

            The `<details>` pattern is the one `Board.tsx`'s legend already uses,
            rather than a second disclosure idiom. */}
        <nav className="views">
          {SURFACES.filter((s) => !CARD_SURFACES.includes(s.key)).map((s) => (
            <button
              type="button"
              key={s.key}
              className={`ghost ${surface === s.key ? "on" : ""}`}
              onClick={() => setSurface(surface === s.key ? null : s.key)}
            >
              {s.label}
            </button>
          ))}
          <details className="cards-menu" open={CARD_SURFACES.includes(surface as SurfaceName)}>
            <summary className="ghost">Cards</summary>
            <div>
              {SURFACES.filter((s) => CARD_SURFACES.includes(s.key)).map((s) => (
                <button
                  type="button"
                  key={s.key}
                  className={`ghost ${surface === s.key ? "on" : ""}`}
                  onClick={() => setSurface(surface === s.key ? null : s.key)}
                >
                  {s.label}
                </button>
              ))}
            </div>
          </details>
        </nav>
        {/* The payload is built for the seat to move, so the banner names WHO
            is playing rather than saying "your turn" — at 3P or 4P one operator
            drives more than one human seat, exactly as the terminal does, and a
            screen that will not say which seat is a screen you can play the
            wrong turn on. */}
        <span className="meta turn">
          {state.over ? (
            "finished"
          ) : (
            <>
              <span className={`seat-${(colors[state.to_move] ?? "").toLowerCase()}`}>
                {colors[state.to_move] ?? `Seat ${state.to_move}`}
              </span>
              {" to move"}
            </>
          )}
        </span>
      </header>

      {/* How close the game is to ending, in the CLI's own words. Without it a
          player cannot see the game ending, which is not a polish issue.

          `role="status"` here and on the three blocks in the deck below. The
          page had ZERO live regions before this — `grep -rn 'aria-live|role=
          "alert"|role="status"' web/src` returned nothing — so every applied-move
          delta, every engine refusal and the game beginning to end were
          announced to no assistive technology at all.

          And the containers are mounted whether or not they have anything in
          them, which is the half that is easy to get wrong: a live region
          inserted into the DOM at the same instant as its text is unreliably
          announced, because there was no region to observe when the text
          arrived. They collapse to zero height while empty (`styles.css`)
          rather than being removed. */}
      <div
        className={`end-strip ${state.end_game_triggered ? "final" : ""}`}
        role="status"
      >
        {state.end_triggers}
      </div>

      <main className="stage">
        <div className="board-col">
          <Board
            board={state.frame.board}
            art={art}
            labels={labels}
            colors={colors}
            selected={selected}
            busy={busy}
            {...(picking
              ? { onTileClick: (area: number) => void send(`A${String(area).padStart(2, "0")}`) }
              : {})}
          />
        </div>
        <aside className="rail">
          <Dashboard panels={state.frame.dashboard} art={art} labels={labels} colors={colors} />
        </aside>
      </main>

      {surface && (
        <Surface
          name={surface}
          game={state.id}
          art={art}
          labels={labels}
          colors={colors}
          tracks={tracks}
          onClose={() => setSurface(null)}
        />
      )}

      {/* **The answer bar, pinned — and it is ONE bar, deliberately.**
          Measured at 4P before this: the decision zone began at y=2006 on a
          2179px page, 1,106px below the fold, so every move ended in a scroll.
          `.stage` is `max(board, rail)` and the rail runs 2.1–2.5× the board's
          height, so nothing done to the board shortens the page — pinning is
          what makes the page's height stop mattering.

          The three blocks share a container because pinning them separately is
          a measured defect, not a tidiness question. At one `bottom: 0` and one
          z-index they land on the SAME edge, and the error strip is last in the
          DOM: hit-tested, `elementFromPoint` on "Apply the only option" and on
          "Undo" returned `div.error`. Pinning only the middle block instead
          left the error strip at y=2237 — a permanently visible answer bar and
          no visible reason the click did nothing. One container, and neither
          failure is reachable. */}
      <div className="deck">
        <div className="decision notices" role="status">
          {state.warnings.map((w, i) => (
            <div className="warn" key={i}>
              {w}
            </div>
          ))}
        </div>
        <Decision
          state={state}
          labels={labels}
          colors={colors}
          busy={busy}
          onSend={(l) => void send(l)}
        />
        {/* `role="alert"` and not `status`: this is the one channel that is
            always a REFUSAL of something the player just did, and it is
            assertive for the same reason the terminal prints it immediately. */}
        <div className="decision notices" role="alert">
          {error && <div className="error">{error}</div>}
        </div>
      </div>
    </div>
  );
}
