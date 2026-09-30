// The enabled sections under the pyramid (M5 spec §3.4).
//
// **Refreshed when the POSITION moved, and from the run where there is one**
// (cycle E, Task 7; R-E9, ruling T7-a). M5 re-fetched every enabled board on
// every `state` the screen took — a picker step's reply included, although a
// step that applies no event cannot move a board — and a move's run already
// carries the port and the laboratory at its live frame, built by the same
// `build_ports`/`build_labs` the two routes answer with. Measured over the
// two-browser walk, that was 7.12 of 16.74 requests per applied move. So:
//
// - the port and the laboratory are keyed on the COUNT (`state.events`), not
//   on the state's identity — a new state at the same count asks nothing;
// - a count a run is on its way for (`runFor`) asks nothing, and the run's
//   last frame (`live`) is what they are drawn from once it lands;
// - a count with no run — the mount, an undo's shrink, a re-read that found
//   the game moved, a run whose fetch failed — asks each board ONCE, on the
//   scrubber's own drop shape: an `alive` flag, a stale answer dropped;
// - the building sites have no position-shaped version, and their strip reads
//   only each tile's `site`, which the live board carries: they are re-read
//   when those `site`s changed — a build, or an undo of one — and on nothing
//   else.
//
// Tracks and Goals need no fetch; they are on the frame.
//
// **The port and the laboratory draw from a position that carries them.**
// Since M5.5 `/at/{k}` serves the offers as they stood — face up at the time,
// and properties of the position rather than present-tense lies about it — so
// a strip beside a historical board is the strip that WAS there, whether the
// player is scrubbing or the opponent's turn is playing out. Tracks and Goals
// come off the same frame, as they always did.
//
// **The building sites still collapse.** Their board is read from the LIVE
// game (`/areas`), and there is no position-shaped version of it, so the
// section says it is hidden rather than showing today's builds under an
// earlier board.
//
// **The fetch is guarded by `scrubbing` and NOT by `replaying`, and the two
// differ only during a walk.** A walk turns `replaying` on and off again while
// `state` never moves, and a guard on a value that falls back to false at the
// end of the walk fires a `/ports` for a move whose run already carried it —
// the browser cell that pins NO `/ports` after a move counts exactly that
// (`the_table.spec.ts`) — and cancels any answer in flight on the way in.
// Scrubbing keeps M4's rule (no state change happens while you are in the
// past, so nothing needs re-reading) without either effect.

import { useEffect, useMemo, useState } from "react";
import { api, ApiError } from "./api";
import { SECTION_KEYS, SECTION_LABELS, type SectionKey } from "./sections";
import { GoalsStrip, LabStrip, PortStrip, SitesStrip, TracksStrip } from "./Strips";
import type {
  AreasPanel,
  ArtManifest,
  ClientState,
  HistoricalFrame,
  Labels,
  LabsPanel,
  PlayerColor,
  PortsPanel,
} from "./types";
import { Refusal } from "./Refusal";

export function TableSections({
  enabled,
  game,
  state,
  replaying,
  scrubbing,
  view,
  art,
  labels,
  colors,
  changedSlots,
  live = null,
  runFor = null,
}: {
  enabled: SectionKey[];
  /** `null` before a game is on screen — there is nothing to fetch and nothing
   *  a board could be about. */
  game: string | null;
  /** The live state — its identity is the refresh key. */
  state: ClientState | null;
  /** What is drawn is not the live position. Collapses the building sites. */
  replaying: boolean;
  /** The player is in the PAST via the log's track. Skips the fetch — see the
   *  header note on why this is not `replaying`. */
  scrubbing: boolean;
  /** What the board is drawing: the live state, or a historical position. */
  view: ClientState | HistoricalFrame;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  /** `port:<board>:<slot>` / `lab:<board>:<slot>` keys the walk is lighting. */
  changedSlots?: Set<string>;
  /** The newest run's LAST frame for this game — the live position when its
   *  `index` is the state's count, and then the port and the laboratory are
   *  drawn from it with no request (R-E9). */
  live?: HistoricalFrame | null;
  /** The count a run is being fetched for, while it is: at that count the
   *  boards wait for the run rather than ask (R-E9). */
  runFor?: number | null;
}) {
  const [ports, setPorts] = useState<PortsPanel | null>(null);
  const [labs, setLabs] = useState<LabsPanel | null>(null);
  const [areas, setAreas] = useState<AreasPanel | null>(null);
  const [error, setError] = useState<string | null>(null);
  /** The building sites' refusal, apart from the boards': the two are asked
   *  on different keys, and one's success must not clear the other's. */
  const [sitesError, setSitesError] = useState<string | null>(null);
  // What is on, as scalars, so each effect keys on the boards IT asks for
  // rather than on the array's identity — `App` rebuilds that array on every
  // toggle, and a raw dependency would ask every enabled board on every render.
  const portsOn = enabled.includes("ports");
  const labsOn = enabled.includes("labs");
  const sitesOn = enabled.includes("buildings");
  /** The position's COUNT: what the port and the laboratory are keyed on. */
  const events = state?.events ?? null;
  /** The run's last frame IS this count's live position. */
  const fromRun = live !== null && events !== null && live.index === events;
  /** A run for this count is on its way. */
  const coming = runFor !== null && runFor === events;
  /** What the sites strip reads off each tile — its `site` — for the live
   *  board: the building sites' key. */
  const sites = useMemo(
    () => (state === null ? "" : JSON.stringify(state.frame.board.tiles.map((t) => [t.area, t.site]))),
    [state],
  );

  // The run's boards, taken as the live ones when they are this count's —
  // into the same state a read writes, so an undo that asks again shows the
  // newest boards meanwhile rather than the mount's.
  useEffect(() => {
    if (!fromRun || live === null) return;
    setPorts(live.ports);
    setLabs(live.labs);
  }, [fromRun, live]);

  useEffect(() => {
    // **Cleared BEFORE the replaying return, not after it.** The other way
    // round, a refusal from the last fetch stayed on screen under the
    // collapsed sections for the whole time the player was in the past —
    // a sentence about a fetch that is no longer being made.
    setError(null);
    if (game === null || scrubbing || fromRun || coming) return;
    let alive = true;
    const said = (e: unknown) => alive && setError(e instanceof ApiError ? e.message : String(e));
    if (portsOn) api.ports(game).then((p) => alive && setPorts(p)).catch(said);
    if (labsOn) api.labs(game).then((p) => alive && setLabs(p)).catch(said);
    return () => {
      alive = false;
    };
  }, [game, events, scrubbing, fromRun, coming, portsOn, labsOn]);

  useEffect(() => {
    setSitesError(null);
    if (game === null || scrubbing || !sitesOn) return;
    let alive = true;
    api
      .areas(game)
      .then((p) => alive && setAreas(p))
      .catch((e: unknown) => alive && setSitesError(e instanceof ApiError ? e.message : String(e)));
    return () => {
      alive = false;
    };
  }, [game, sites, scrubbing, sitesOn]);

  /** The one refusal the sections carry: the boards', else the sites'. */
  const refusal = error ?? sitesError;
  const tracksPanel = view.frame.dashboard.find((p) => "Tracks" in p);
  const tracks = tracksPanel && "Tracks" in tracksPanel ? tracksPanel.Tracks : null;
  const goalsPanel = view.frame.dashboard.find((p) => "Achievements" in p);
  const goals = goalsPanel && "Achievements" in goalsPanel ? goalsPanel.Achievements.cards : [];

  /** A position that carries its own offers — every `HistoricalFrame` does,
   *  and no `ClientState` does. */
  const offers = "ports" in view ? view : null;

  /** A section that carries one LINE where a board would be.
   *
   *  Four things can put a section in this state and they are different
   *  things — the four wrappers below name them, and each states its own
   *  reason. What they share is the markup, which is this function: the same
   *  heading, one muted sentence, and the `collapsed` class for the three that
   *  are a STANDING absence.
   *
   *  `loading` is the one that does not take it, deliberately: it is a moment
   *  rather than a state, and `collapsed` is what the e2e reads to tell a
   *  section that is showing a line from one that is showing a board. */
  const note = (key: SectionKey, line: string, standing = true) => (
    <section
      className={`table-sec ${key}${standing ? " collapsed" : ""}`}
      key={key}
      aria-label={SECTION_LABELS[key]}
    >
      <h2>{SECTION_LABELS[key]}</h2>
      <p className="muted">{line}</p>
    </section>
  );
  /** Replaying: this board is a LIVE offer and the position on screen is not
   *  live, so showing it would state the present under a past. */
  const collapsed = (key: SectionKey) => note(key, "live offer, hidden while replaying");
  /** The fetch is in flight. A moment, not a state. */
  const loading = (key: SectionKey) => note(key, "loading…", false);
  /** No game yet, and no way to draw this board: the setup screen's table
   *  (Task 12) draws the boards it can and says so about the one it cannot,
   *  rather than spinning on a fetch that has no game to name. */
  const unavailable = (key: SectionKey) => note(key, "available once the game starts");
  /** Withheld until the first card is chosen (2026-09-06, R-C3) — a DIFFERENT
   *  thing from `unavailable` above, which is a board this screen has no data
   *  for. The server HAS the technologies and the achievements and deliberately
   *  does not send them: the owner's edition deals the two starting orders and
   *  takes the secret choice at its step 9, and the technologies reach the
   *  Laboratory at step 10 and the achievements the score track at step 11.
   *
   *  Which is why an empty strip would not do. With the slots emptied on the
   *  wire, `LabStrip` draws six blanks and `GoalsStrip` draws nothing, and both
   *  read as a screen that failed rather than as a table that is not there
   *  yet. */
  const withheld = (key: SectionKey) => note(key, "revealed once you have chosen your card");

  return (
    <div className="table-sections">
      {SECTION_KEYS.filter((k) => enabled.includes(k)).map((key) => {
        switch (key) {
          case "ports":
            return offers ? (
              <PortStrip key={key} panel={offers.ports} art={art} changed={changedSlots} />
            ) : replaying ? (
              collapsed(key)
            ) : ports ? (
              <PortStrip key={key} panel={ports} art={art} changed={changedSlots} />
            ) : (
              loading(key)
            );
          case "labs":
            // R-C3 first: on the set table there is no game, and the wire's own
            // laboratory is empty by construction.
            return game === null ? (
              withheld(key)
            ) : offers ? (
              <LabStrip key={key} panel={offers.labs} art={art} labels={labels} changed={changedSlots} />
            ) : replaying ? (
              collapsed(key)
            ) : labs ? (
              <LabStrip key={key} panel={labs} art={art} labels={labels} changed={changedSlots} />
            ) : (
              loading(key)
            );
          case "buildings":
            return replaying
              ? collapsed(key)
              : game === null
                ? unavailable(key)
                : areas
                  ? <SitesStrip key={key} panel={areas} art={art} colors={colors} />
                  : loading(key);
          case "tracks":
            return tracks ? <TracksStrip key={key} tracks={tracks} art={art} colors={colors} /> : null;
          case "achievements":
            return game === null ? (
              withheld(key)
            ) : (
              <GoalsStrip key={key} cards={goals} art={art} labels={labels} colors={colors} />
            );
        }
      })}
      {refusal && <Refusal sentence={refusal} />}
    </div>
  );
}
