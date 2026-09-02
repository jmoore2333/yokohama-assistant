import { useCallback, useEffect, useRef, useState } from "react";
import { api, ApiError } from "./api";
import { Board } from "./Board";
import { Dashboard } from "./Dashboard";
import { LastMove } from "./LastMove";
import { Decision } from "./Decision";
import { GameMenu } from "./GameMenu";
import { NewGame } from "./NewGame";
import { loadSections, saveSections, SECTION_KEYS, SECTION_LABELS, type SectionKey } from "./sections";
import { CARD_SURFACES, Surface, SURFACES, type SurfaceName } from "./Surfaces";
import { TableSections } from "./TableSections";
import type {
  ArtManifest,
  ClientState,
  HistoricalFrame,
  Labels,
  Outcome,
  PlayerColor,
} from "./types";

export function App() {
  const [labels, setLabels] = useState<Labels | null>(null);
  const [art, setArt] = useState<ArtManifest | null>(null);
  const [state, setState] = useState<ClientState | null>(null);
  const [surface, setSurface] = useState<SurfaceName | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // **The replay position, or `null` for "the present".** M4's whole state is
  // this one integer: the scrubber is a projection over a record the server
  // already holds, so nothing here is persisted and nothing is applied.
  const [scrubK, setScrubK] = useState<number | null>(null);
  const [past, setPast] = useState<HistoricalFrame | null>(null);
  // **Which boards are on the table, and it is a per-BROWSER choice.** The set
  // is not game state and is deliberately not sent anywhere: `sections.ts`
  // keeps it in `localStorage`, a storage that throws reads as the default, and
  // a game opened on another machine opens with the default set.
  const [enabled, setEnabled] = useState<SectionKey[]>(() => loadSections());
  const toggleSection = (k: SectionKey) =>
    setEnabled((e) => {
      const next = e.includes(k) ? e.filter((x) => x !== k) : [...e, k];
      saveSections(next);
      return next;
    });
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

  // **Which game is on screen, as a ref, because a reply is judged against it
  // inside a closure that was created before the switch.**
  //
  // Measured on the live client (final review I1): with `/input` delayed 3s,
  // switching games from the menu mid-request showed the new game after 181ms
  // and then reverted to the old one at 3,104ms when the reply landed — banner,
  // board and `state.id` — the chosen switch silently undone. A state variable
  // cannot answer this: `send` closed over the state it was called with, and
  // reading `state` there reads the game the request belongs to, never the game
  // the player is now looking at. So the id is kept in a ref that every change
  // of game updates, and `send` drops anything that no longer matches it.
  const gameId = useRef<string | null>(null);
  /** Put a game on screen — the ONE way `state` changes identity, so the ref
   *  and the state cannot disagree. `null` is the setup screen. */
  const show = useCallback((s: ClientState | null) => {
    gameId.current = s?.id ?? null;
    setState(s);
  }, []);

  // **The final standing comes from `/result`, because `scores` cannot carry
  // it.** `ClientState.scores` is `{seat, total}[]` in SEAT order with no rank
  // and no `won`, so from it alone the inline block could print a colour and a
  // sorted list and would have to INFER a winner from the margin — which is the
  // exact inference `Index.test.tsx` exists to forbid, since two seats can tie
  // on points and be separated by the engine's own tie-break. `/result` carries
  // `Placing.won` and the per-category breakdown the terminal prints.
  useEffect(() => {
    if (!state?.over) {
      setOutcome(null);
      return;
    }
    let live = true;
    api
      .result(state.id)
      .then((o) => live && setOutcome(o))
      .catch(() => live && setOutcome(null));
    return () => {
      live = false;
    };
  }, [state?.over, state?.id]);

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
      // The game this request belongs to, read once: `state` is re-created on
      // every render and the checks below are about the game we posted TO.
      const game = state.id;
      inFlight.current = true;
      setBusy(true);
      setError(null);
      try {
        const replied = await api.input(game, line, state.prompt.kind);
        // **A reply for a game you have left is dropped and never applied.**
        // See `gameId`. Nothing is lost by dropping it: the move was applied
        // and persisted server-side, and coming back to that game re-reads it.
        if (replied.id === gameId.current) setState(replied);
      } catch (e) {
        // The same rule for the refusal: a sentence about a game that is no
        // longer on screen would sit over a game it does not describe.
        if (gameId.current !== game) return;
        setError(e instanceof ApiError ? e.message : String(e));
        // **Re-read the game, because the server just told us to and this
        // client had no way to.**
        //
        // `api.get` had exactly one call site — the setup screen — so once a
        // game was running there was no re-read path at all. The catch branch
        // never called `setState`, so `prompt.kind` stayed stale and every
        // subsequent click re-sent the same `expect` for the same 409, while
        // the server's own message read "Re-read the game and answer what it
        // asks." The only escape was a page reload back into setup.
        //
        // M3.5 makes that worse rather than neutral: with the answer bar pinned,
        // the player now stares at a permanently visible row of buttons every
        // one of which refuses. So the refusal is shown AND the truth is
        // fetched — the message stays on screen beside a screen that now agrees
        // with the server.
        try {
          const fresh = await api.get(game);
          if (fresh.id === gameId.current) setState(fresh);
        } catch {
          // The re-read failed too. Keep the original refusal on screen: it is
          // the more useful of the two, and a second error would replace the
          // sentence that explains what happened with one about this recovery.
        }
      } finally {
        inFlight.current = false;
        setBusy(false);
      }
    },
    [state],
  );

  // **Debounced, because a drag is not a click.**
  //
  // The pattern is `Index.tsx`'s: a 200 ms timer plus a `live` flag, so a drag
  // across forty stops costs one request rather than forty, and a response that
  // arrives after you have moved on is dropped instead of painting a stale
  // position. The browser walk counts the requests a drag actually makes — a
  // cell that only checked the final frame would pass a scrubber firing one
  // request per pixel.
  useEffect(() => {
    if (scrubK === null || !state) {
      setPast(null);
      return;
    }
    let live = true;
    const id = state.id;
    const k = scrubK;
    const timer = setTimeout(() => {
      api
        .at(id, k)
        .then((p) => {
          if (!live) return;
          setPast(p);
          setError(null);
        })
        // **Say so.** Swallowing this put the LIVE board back on screen —
        // clickable, outline gone — while the track still read "after 500 of
        // 1036" and offered "Back to live" from a position it was no longer
        // showing. The server refuses in sentences (a 404 naming the bound, a
        // replay failure naming the event); carry one.
        .catch((e) => {
          if (!live) return;
          setPast(null);
          setError(e instanceof ApiError ? e.message : String(e));
        });
    }, 200);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [scrubK, state?.id]);

  // Leaving the log panel leaves the replay. The scrub track lives in that
  // panel, so a scrub position with the panel shut would be a board showing a
  // past nothing on screen explains.
  useEffect(() => {
    if (surface !== "log") setScrubK(null);
  }, [surface]);

  if (!state) return <NewGame art={art} onStarted={show} />;

  const players = state.frame.dashboard.find((p) => "Players" in p);
  const colors: PlayerColor[] =
    players && "Players" in players ? players.Players.players.map((p) => p.color) : [];

  // **The last stop is the PRESENT, not a past position.** The track's final
  // stop is `total`, which the panel itself labels "now" — so treating any
  // non-null `past` as "replaying" outlined the live board, disabled placement
  // and contradicted the help text, all while the panel said you were at the
  // present. At that stop the LIVE payload is also the fresher of the two: a
  // bot may have moved since the snapshot was fetched.
  const atPresent = past !== null && past.index >= past.total;
  const replaying = past !== null && !atPresent;
  const view = replaying ? past : state;

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
        <GameMenu
          state={state}
          busy={busy}
          onSwitch={(s) => {
            setSurface(null);
            setScrubK(null);
            // The refusal on screen was about the game being left, so it goes
            // with it: a sentence from game A over game B describes nothing
            // the player can see.
            setError(null);
            show(s);
          }}
          onNew={() => {
            setSurface(null);
            setScrubK(null);
            setError(null);
            show(null);
          }}
        />
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
          {/* **The section toggles, and they are a DISCLOSURE for a measured
              reason.** Eleven inline buttons wrapped the banner to a second row
              below 1280 at M3.5, and `the_screen_itself.spec.ts`'s banner cell
              pins its height to one row at 1440 and 1280; five more buttons
              would wrap it again. The summary carries the count, so the state
              is readable with the menu shut.

              **Escape closes it (ruling R-T12a).** An uncontrolled `<details>`
              ignores Escape — the key is the browser's affordance for a
              `<select>` and a `<dialog>`, not for a disclosure — so a menu left
              open sits over the table. Clicking the summary still toggles; this
              only adds the key. */}
          <details
            className="cards-menu boards-menu"
            onKeyDown={(e) => {
              if (e.key === "Escape") e.currentTarget.removeAttribute("open");
            }}
          >
            <summary className="ghost">Boards &middot; {enabled.length} on</summary>
            <div>
              {SECTION_KEYS.map((k) => (
                <button
                  type="button"
                  key={k}
                  className={`ghost ${enabled.includes(k) ? "on" : ""}`}
                  aria-pressed={enabled.includes(k)}
                  onClick={() => toggleSection(k)}
                >
                  {SECTION_LABELS[k]}
                </button>
              ))}
            </div>
          </details>
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
              <span className={`seat-${(colors[view.to_move] ?? "").toLowerCase()}`}>
                {colors[view.to_move] ?? `Seat ${view.to_move}`}
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
        className={`end-strip ${view.end_game_triggered ? "final" : ""}`}
        role="status"
      >
        {view.end_triggers}
      </div>

      {/* **The table draws the REPLAY position while you are scrubbing.**
          The board never goes inside the panel: measured, a 4P board in the
          704px drawer scrolls 322px horizontally and ~205px vertically, and
          shrinking the tile enough to fit drops the printed power ladder to
          ~5.2px against the 7.9px M3.5 measured as legible. So the panel holds
          the track and the log, and the full-size board behind it redraws at
          `k`.

          **No `onTileClick` on a historical board, ever.** A past position is
          not answerable, so every tile renders as an inert `<div>` rather than
          a button — which is also what makes the panel's modality correct here
          rather than costly: there is nothing behind it to click. */}
      {/* **The table and the side column (M5 spec §3.1).** Above 1201px `.app`
          is a grid: the table is column 1 across rows 3–4 and scrolls on its
          own; the deck is column 2 row 3, pinned by the grid rather than by
          `sticky`; the side column is column 2 row 4 and scrolls on its own.
          Below 1201px nothing here is a grid item — this is the DOM order the
          M3.5 column stack reads, table then side then the sticky deck, so R8
          holds by construction. `main` is the one landmark; the board never
          goes inside a panel (M4 §3.1), and it never has. */}
      <main className={`table ${replaying ? "replaying" : ""}`}>
        <div className="table-body">
          <Board
            board={view.frame.board}
            art={art}
            labels={labels}
            colors={colors}
            selected={replaying ? [] : selected}
            busy={busy}
            {...(picking && !replaying
              ? { onTileClick: (area: number) => void send(`A${String(area).padStart(2, "0")}`) }
              : {})}
          />
          {/* **Keyed on the game, so a switch does not paint the last game's
              boards.** The fetched strips (port, laboratory, building sites)
              are component state; without the key the previous game's Port
              offer stays on screen until the new game's fetch lands, under a
              board that has already changed. Remounting drops that cache and
              the refusal beside it. */}
          <TableSections
            key={state.id}
            enabled={enabled}
            game={state.id}
            state={state}
            replaying={replaying}
            view={view}
            art={art}
            labels={labels}
            colors={colors}
          />
        </div>
      </main>
      <aside className="side">
        <LastMove messages={state.messages} />
        <Dashboard panels={view.frame.dashboard} art={art} labels={labels} colors={colors} only={["Players", "Hand"]} />
      </aside>

      {surface && (
        <Surface
          name={surface}
          game={state.id}
          art={art}
          labels={labels}
          colors={colors}
          scrubK={scrubK}
          onScrub={setScrubK}
          onClose={() => setSurface(null)}
        />
      )}

      {/* **The answer bar, pinned — and it is ONE bar, deliberately.**
          Measured at 4P before this: the decision zone began at y=2006 on a
          2179px page, 1,106px below the fold, so every move ended in a scroll.
          `.stage` is `max(board, rail)` and the rail runs 2.1–2.5× the board's
          height, so nothing done to the board shortens the page — pinning is
          what makes the page's height stop mattering.

          **Since M5 it has two homes and only one of them is `sticky`.** Above
          1201px the grid puts it in column 2 row 3 with `position: static` and
          nothing scrolls past it; below, this is M3.5's bottom-edge sticky bar,
          unchanged. The e2e cell asserts both arms.

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
          outcome={outcome}
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
