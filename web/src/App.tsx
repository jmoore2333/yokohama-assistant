import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import { api, ApiError, isSignedOut, onUnauthorized } from "./api";
import { SignIn } from "./SignIn";
import { doorSentence, useMe } from "./whoami";
import { Board } from "./Board";
import { seatClass } from "./Cards";
import { Dashboard } from "./Dashboard";
import { TurnAccount } from "./TurnAccount";
import { Decision } from "./Decision";
import { GameMenu } from "./GameMenu";
import { NewGame } from "./NewGame";
import { loadCadence, saveCadence, type Cadence } from "./playback";
import { BoardsMenu, useSections } from "./BoardsMenu";
import { CARD_SURFACES, Surface, SURFACES, type SurfaceName } from "./Surfaces";
import { TableSections } from "./TableSections";
import {
  buildRun,
  dwellFor,
  hasBotStep,
  heldChanges,
  walkReducer,
  type Run,
  type WalkAction,
  type WalkState,
} from "./walk";
import type {
  ArtManifest,
  ClientState,
  HistoricalFrame,
  Labels,
  Outcome,
  PlayerColor,
} from "./types";

/** The positions `prev..=next` and the log's tail, in parallel.
 *
 *  **`prev` is fetched too**, and that is the whole reason this takes a range
 *  rather than the reply's own count: a step's highlight is a DIFF, so the
 *  first step needs a full position on the side the reply came from, and the
 *  live payload is not one — it carries no port and no laboratory.
 *
 *  Outside the component because it closes over nothing: `api` is a module and
 *  `buildRun` is pure, so a hook would only make its identity churn. */
async function fetchRun(game: string, prev: number, next: number): Promise<Run> {
  const ks = Array.from({ length: next - prev + 1 }, (_, i) => prev + i);
  const [frames, log] = await Promise.all([
    Promise.all(ks.map((k) => api.at(game, k))),
    api.log(game, next - prev),
  ]);
  return buildRun(prev, frames, log);
}

export function App() {
  // **The door, read before any screen is chosen** (M6.1 §7.1). Every
  // `/api/games` route is a 401 without an identity, so who the server thinks
  // we are decides which of four screens this is — see the branch below the
  // hooks, which is where it can finally be acted on.
  const door = useMe();
  const { refresh, signOut, signedOut } = door;
  // **Every 401 on this API routes here, and no screen has to remember to
  // (ruling R-7.1).** `api.ts` invokes this from inside `call`, before the
  // refusal is thrown — so it reaches the door whatever the caller then does
  // with the error, including `/result`'s catch, which discards it. Registered
  // in an effect and cleared on unmount: the module holds one handler, and a
  // handler left behind by a torn-down tree would move a door that is gone.
  useEffect(() => {
    onUnauthorized(signedOut);
    return () => onUnauthorized(null);
  }, [signedOut]);
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
  // **The last run a reply added, and the walk over it (M5.5 spec §3.2).**
  //
  // `run` outlives the walk: the account goes on showing it until the next run
  // replaces it, which is what makes "replay" mean something after the walk has
  // ended. `walk` is the step on the table and whether the timer runs, and
  // `null` is "over" — the live state is what is on screen. `fresh` is true
  // from the moment a run lands until the player's next answer, and is what
  // keeps the finished turn's highlights on the live table (spec §3.2.4)
  // instead of clearing them the instant the walk finishes; WHICH marks those
  // are is `heldChanges`, not the last step's own diff.
  const [run, setRun] = useState<Run | null>(null);
  const [walk, setWalk] = useState<WalkState | null>(null);
  const [fresh, setFresh] = useState(false);
  const [cadence, setCadence] = useState<Cadence>(() => loadCadence());
  const dispatch = useCallback(
    (a: WalkAction) => setWalk((w) => (w && run ? walkReducer(w, a, run.steps.length - 1) : w)),
    [run],
  );
  // **ONE timer, and `dwellFor` is the whole of the timing rule.** A second
  // clock anywhere — a per-step `setInterval`, a transition that also advances —
  // is how a playback ends up at a different step than the one it is lighting.
  // The effect re-runs on every action because `walk` is a new object each
  // time, so each step gets its own fresh dwell and the cleanup cancels the
  // one before it.
  useEffect(() => {
    if (!run || !walk || !walk.playing) return;
    const step = run.steps[walk.at];
    if (!step) return;
    const t = setTimeout(() => dispatch({ type: "tick" }), dwellFor(step, cadence));
    return () => clearTimeout(t);
  }, [run, walk, cadence, dispatch]);
  // **Which boards are on the table, and it is a per-BROWSER choice.** The set
  // is not game state and is deliberately not sent anywhere: `sections.ts`
  // keeps it in `localStorage`, a storage that throws reads as the default, and
  // a game opened on another machine opens with the default set.
  const [enabled, toggleSection] = useSections();
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
  /** A reply landed: if it added events, build the run and start the walk when
   *  a bot moved.
   *
   *  Called INSIDE the busy window, so the deck stays disabled until the walk
   *  is ready to show rather than offering the next prompt over a table that is
   *  about to rewind.
   *
   *  **A reply that added NOTHING clears the run as well as `fresh` (ruling
   *  R-P5).** `events` is legitimately non-monotone: a picker step applies no
   *  event, a refusal none, and an undo POPS them. Leaving the previous run
   *  standing there would keep an account of an earlier turn on screen in place
   *  of the prose that answers the click just made. */
  const landed = useCallback(
    async (game: string, prev: number, replied: ClientState) => {
      setFresh(false);
      if (replied.events <= prev) {
        setRun(null);
        setWalk(null);
        return;
      }
      const r = await fetchRun(game, prev, replied.events);
      // The same rule the reply itself is judged by: a run for a game the
      // player has left is dropped, never shown over the game they are on.
      if (gameId.current !== game) return;
      setRun(r);
      setFresh(true);
      setWalk(hasBotStep(r) ? { at: 0, playing: true } : null);
    },
    [],
  );

  /** Put a game on screen — the ONE way `state` changes identity, so the ref
   *  and the state cannot disagree. `null` is the setup screen.
   *
   *  The walk goes with the game: a run is an account of a turn in the game
   *  being left, and rows about it over a different board describe nothing.
   *
   *  `opening` is true only for a game just CREATED. A game whose start seat is
   *  a bot has already played by the time the create replies, and those events
   *  are a run like any other — the only one whose `prev` is 0. A saved game
   *  reopened passes false: its events are history, not something that just
   *  happened. */
  const show = useCallback(
    (s: ClientState | null, opening = false) => {
      gameId.current = s?.id ?? null;
      setState(s);
      setRun(null);
      setWalk(null);
      setFresh(false);
      if (opening && s && s.events > 0) {
        // The busy window is opened by hand here, because this run has no
        // request of its own to ride inside. It is what the browser harness
        // waits on before reading a screen — see `data-busy` on the root — and
        // the refusal goes where every other one goes rather than into an
        // unhandled rejection.
        setBusy(true);
        void landed(s.id, 0, s)
          .catch((e) => setError(e instanceof ApiError ? e.message : String(e)))
          .finally(() => setBusy(false));
      }
    },
    [landed],
  );

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
        // The run the reply added, fetched before the busy window closes. It
        // rides INSIDE the `try` on purpose: a failed fetch of the run reports
        // through the error strip like any other refusal, and the live state
        // stays on screen — the move was applied and persisted either way.
        if (replied.id === gameId.current) await landed(game, state.events, replied);
      } catch (e) {
        // The same rule for the refusal: a sentence about a game that is no
        // longer on screen would sit over a game it does not describe.
        if (gameId.current !== game) return;
        setError(e instanceof ApiError ? e.message : String(e));
        // **The re-read below is skipped on a 401.** The door has already been
        // moved — `api.ts` did that before this refusal was thrown — and a
        // re-read would only spend a request earning the same answer.
        if (isSignedOut(e)) return;
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
    [state, landed],
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

  // ---- the door, below every hook and above every screen (M6.1 §7.1) -------
  //
  // Four answers, in order, and the ORDER is the rule:
  //
  // 1. `/api/me` refused. Say so and offer nothing — a sign-in form over a
  //    server that cannot answer would take a name and lose it.
  // 2. It has not answered yet: draw NOTHING. A default either way flashes the
  //    wrong screen at half the players for one round trip, and the one that
  //    flashes is the sign-in form at people who are already signed in.
  // 3. Nobody is signed in under `claimed` — the one door this browser can
  //    knock on. The form.
  // 4. Nobody is signed in under Cloudflare Access or the edge Worker. There is
  //    no form to offer: the identity comes from in front of the server, so the
  //    only useful thing to say is which header was expected.
  if (door.error) return <div className="setup door-error">{door.error}</div>;
  if (!door.me) return null;
  if (door.me.mode === "claimed" && !door.me.user)
    return <SignIn onSignedIn={() => refresh()} notice={door.notice ?? undefined} />;
  if (!door.me.user) return <div className="setup door-error">{doorSentence(door.me.mode)}</div>;

  // **Who you are, and how to stop being them — built once, placed twice.**
  // The setup screens carry their own root and their own banner, so the strip
  // is handed to `NewGame` as a node rather than restated there: two copies
  // would be two places for the sign-out wiring to drift, and only one of them
  // would be covered by the door's cells.
  //
  // The button exists under `claimed` ONLY. Under the two edge doors there is
  // nothing stored to clear, and a control that looked like it logged you out
  // while Access kept letting you in would be a lie on screen.
  const who = (
    <>
      <span className="meta who">You are {door.me.user}</span>
      {door.me.mode === "claimed" && (
        <button type="button" className="ghost" onClick={signOut}>
          sign out
        </button>
      )}
    </>
  );

  // `enabled`/`toggleSection` are THIS component's, deliberately: the set table
  // and the game must agree about which boards are on, and a second
  // `useSections()` inside `NewGame` was two copies that stopped agreeing the
  // moment one of them was toggled (R-T12-4).
  if (!state) {
    return (
      <NewGame
        art={art}
        labels={labels}
        sections={enabled}
        onToggleSection={toggleSection}
        onStarted={show}
        who={who}
      />
    );
  }

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
  /** The player is in the PAST via the log's track. */
  const scrubbing = past !== null && !atPresent;

  // **The walk's step, and what the whole screen reads from while it runs.**
  //
  // `replaying` is "what is drawn is not the live position", which is what the
  // outline says and what withholds the board as an input. On the walk's LAST
  // step the frame IS the live position — `index === total` — so the outline
  // comes off one step before the walk ends, deliberately: the table has
  // arrived, and the strip below is the only thing still saying so.
  const stepNow = run && walk ? run.steps[walk.at] : undefined;
  const walking = stepNow !== undefined;
  const replaying = walking ? stepNow.after.index < stepNow.after.total : scrubbing;
  const view = walking ? stepNow.after : scrubbing ? past : state;

  // **The highlights outlive the walk by one answer (spec §3.2.4, ruling
  // R-F1).** While it runs they are the step's own diff. When it ends — by
  // ticking out or by `skip` — they are `heldChanges`: the union over the steps
  // the run's LAST mover played, held on the live table by `fresh` until the
  // player answers again, so what the opponent did is still marked on the board
  // you are about to play on.
  //
  // **Held is not "the last step's".** Every bot turn this server serves ends
  // with "End additional-action phase", whose diff is empty, so the last step's
  // marks are no marks and the promise above was kept for no real turn at all
  // (final review I1, reproduced live three times). `walk.ts` carries the rule
  // and the measurement.
  //
  // **ONE source for all four reads, and they stopped being one read.** The
  // MARKS are the step's own diff while the walk runs and the mover's union
  // once they are held. The COLOUR, the pulse's BEAT and its DWELL are the
  // step's on both sides, and that is deliberate rather than left over: the
  // held marks belong to the turn the walk just played, so a beat that moved at
  // the tick-out would restart every animation on a step that has already had
  // its pulse. One object is what keeps the two halves in step — four parallel
  // ternaries could drift apart, and the pulse's would drift silently, a pulse
  // timed to a step whose marks are not the ones on screen looking like nothing
  // at all. The cell ruling R-P4 shaped reads `{ beat: "0", dwell: "2000ms" }`
  // on both sides of the skip and is what pins it.
  const lastStep = run?.steps[run.steps.length - 1];
  const shown =
    walk && stepNow
      ? { step: stepNow, at: walk.at, changes: stepNow.changes, mover: stepNow.mover }
      : fresh && run && lastStep
        ? {
            step: lastStep,
            at: run.steps.length - 1,
            changes: heldChanges(run),
            mover: lastStep.mover,
          }
        : null;
  const highlights = shown?.changes ?? null;
  const highlightBy = shown ? colors[shown.mover] : undefined;

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
    // **`data-busy` is the in-flight state, in the DOM, and the browser walk
    // waits on it.**
    //
    // The harness used to wait on `.decision button[disabled]` alone, which is
    // the same condition for every screen that HAS a control — and the finished
    // game has none: its deck is a standing, not a prompt. So a read taken
    // between the reply landing and the run's positions arriving saw a settled
    // game-over screen, and the walk then replaced the deck a moment later.
    // Measured: the whole-game cell reached its winner, read "The game is
    // over.", and failed ten seconds afterwards looking for the sentence again.
    // The client's own `busy` covers the WHOLE window, the run fetch included.
    //
    // **The pulse's two attributes live HERE, on the one ancestor of both
    // columns (R-GC-1).** Every highlight pulses for the length of the step's
    // own dwell, so the CSS needs the duration and a beat to select the
    // keyframes by — and the four things that light are split across the DOM:
    // `.tile` and `.strip-slot` are inside `main.table`, `.seat` and the tech
    // chips are in `aside.side`, its SIBLING. A custom property cannot be read
    // out of a sibling subtree at all, so a `--dwell-ms` on the table would
    // leave the side column silently animating the 2000 ms fallback at every
    // other cadence. `.app` is where `data-busy` already lives for the same
    // reason: it is the element that contains the whole screen.
    //
    // **The BEAT is what makes a pulse re-run.** An element's animation
    // restarts only when its `animation-name` changes or the element is
    // re-created, and a tile lit by two consecutive steps is neither — so the
    // second step would land in silence. `at % 2` picks between two
    // byte-identical keyframe names, which is the whole mechanism.
    <div
      className="app"
      data-busy={busy ? "1" : undefined}
      data-beat={shown ? String(shown.at % 2) : undefined}
      style={shown ? ({ "--dwell-ms": `${dwellFor(shown.step, cadence)}ms` } as CSSProperties) : undefined}
    >
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
        {who}
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
          <BoardsMenu enabled={enabled} onToggle={toggleSection} />
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
            selected={replaying || walking ? [] : selected}
            busy={busy}
            changed={highlights?.tiles}
            changedBy={highlightBy}
            {...(picking && !replaying && !walking
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
            scrubbing={scrubbing}
            view={view}
            art={art}
            labels={labels}
            colors={colors}
            changedSlots={highlights?.slots}
          />
        </div>
      </main>
      <aside className="side">
        {/* The walk's state, its timer and its jumps are `App`'s; the account
            is the CONTROL surface over them and holds no clock of its own.
            With no run it renders the reply's prose lines, which is all this
            box could ever hold before the walk existed. */}
        <TurnAccount
          messages={state.messages}
          run={run}
          at={walk?.at ?? null}
          playing={walk?.playing ?? false}
          cadence={cadence}
          labels={labels}
          colors={colors}
          onCadence={(c) => {
            saveCadence(c);
            setCadence(c);
          }}
          onPlay={() => dispatch({ type: "play" })}
          onPause={() => dispatch({ type: "pause" })}
          onBack={() => dispatch({ type: "back" })}
          onForward={() => dispatch({ type: "forward" })}
          onJump={(i) => setWalk({ at: i, playing: false })}
          onSkip={() => dispatch({ type: "skip" })}
          onReplay={() => setWalk({ at: 0, playing: true })}
        />
        <Dashboard
          panels={view.frame.dashboard}
          art={art}
          labels={labels}
          colors={colors}
          only={["Players", "Hand"]}
          changedSeats={highlights?.seats}
          changedTechs={highlights?.techs}
          changedHand={highlights?.hand}
        />
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
        {/* **While the walk runs the deck is the strip, not the prompt.** The
            prompt underneath is answerable and would be answered — the walk is
            a picture of what already happened, and a click landing during it
            would be a move made against a table showing an earlier position.
            `skip` is the way out and lands on the live prompt at once.

            Driven by the WALK's own flag rather than by `replaying`, which
            comes off one step early by design (see the view block above). */}
        {walking ? (
          <div className="decision walking" role="status">
            <span className={seatClass(colors[stepNow.mover])}>
              {colors[stepNow.mover] ?? `Seat ${stepNow.mover}`}
            </span>
            {" is playing · step "}
            {walk!.at + 1}
            {" of "}
            {run!.steps.length}
            <button type="button" className="ghost" onClick={() => dispatch({ type: "skip" })}>
              skip
            </button>
          </div>
        ) : (
          <Decision
            state={state}
            labels={labels}
            colors={colors}
            busy={busy}
            outcome={outcome}
            onSend={(l) => void send(l)}
          />
        )}
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
