// The enabled sections under the pyramid (M5 spec §3.4).
//
// **Refresh on every state change.** The drawers fetched on demand because
// "none of them changes between decisions in a way a player is watching for";
// a section on the table is exactly that watching, so an enabled fetched
// section re-fetches whenever `state` changes — every `/input`, re-read and
// create — on the scrubber's own debounce-and-drop shape: a `live` flag, a
// stale answer dropped. Tracks and Goals need no fetch; they are on the frame.
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
// end of the walk fires a SECOND `/ports` for one move — the browser cell that
// pins one refresh per state-changing response counts exactly that — and
// cancels the first one's response on the way in. Scrubbing keeps M4's rule
// (no state change happens while you are in the past, so nothing needs
// re-reading) without either effect.

import { useEffect, useState } from "react";
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
}) {
  const [ports, setPorts] = useState<PortsPanel | null>(null);
  const [labs, setLabs] = useState<LabsPanel | null>(null);
  const [areas, setAreas] = useState<AreasPanel | null>(null);
  const [error, setError] = useState<string | null>(null);
  // The enabled SET as a scalar, so the effect keys on what is on rather than
  // on the array's identity — `App` rebuilds that array on every toggle and a
  // raw dependency would refetch every enabled section on every render.
  const wanted = enabled.join(",");

  useEffect(() => {
    // **Cleared BEFORE the replaying return, not after it.** The other way
    // round, a refusal from the last fetch stayed on screen under the
    // collapsed sections for the whole time the player was in the past —
    // a sentence about a fetch that is no longer being made.
    setError(null);
    if (game === null || scrubbing) return;
    let live = true;
    const said = (e: unknown) => live && setError(e instanceof ApiError ? e.message : String(e));
    if (enabled.includes("ports")) api.ports(game).then((p) => live && setPorts(p)).catch(said);
    if (enabled.includes("labs")) api.labs(game).then((p) => live && setLabs(p)).catch(said);
    if (enabled.includes("buildings")) api.areas(game).then((p) => live && setAreas(p)).catch(said);
    return () => {
      live = false;
    };
    // `wanted` stands in for `enabled` here on purpose (see its comment above);
    // the array itself is rebuilt on every toggle and would refetch everything.
  }, [game, state, scrubbing, wanted]);

  const tracksPanel = view.frame.dashboard.find((p) => "Tracks" in p);
  const tracks = tracksPanel && "Tracks" in tracksPanel ? tracksPanel.Tracks : null;
  const goalsPanel = view.frame.dashboard.find((p) => "Achievements" in p);
  const goals = goalsPanel && "Achievements" in goalsPanel ? goalsPanel.Achievements.cards : [];

  /** A position that carries its own offers — every `HistoricalFrame` does,
   *  and no `ClientState` does. */
  const offers = "ports" in view ? view : null;

  const collapsed = (key: SectionKey) => (
    <section className={`table-sec ${key} collapsed`} key={key} aria-label={SECTION_LABELS[key]}>
      <h2>{SECTION_LABELS[key]}</h2>
      <p className="muted">live offer, hidden while replaying</p>
    </section>
  );
  const loading = (key: SectionKey) => (
    <section className={`table-sec ${key}`} key={key} aria-label={SECTION_LABELS[key]}>
      <h2>{SECTION_LABELS[key]}</h2>
      <p className="muted">loading&hellip;</p>
    </section>
  );
  /** No game yet, and no way to draw this board: the setup screen's table
   *  (Task 12) draws the boards it can and says so about the one it cannot,
   *  rather than spinning on a fetch that has no game to name. */
  const unavailable = (key: SectionKey) => (
    <section className={`table-sec ${key} collapsed`} key={key} aria-label={SECTION_LABELS[key]}>
      <h2>{SECTION_LABELS[key]}</h2>
      <p className="muted">available once the game starts</p>
    </section>
  );
  /** Withheld until the first card is chosen (2026-09-06, R-C3) — a DIFFERENT
   *  thing from `unavailable` above, which is a board this screen has no data
   *  for. The server has the technologies and the achievements and deliberately
   *  does not send them: the owner's edition deals the two starting orders and
   *  takes the secret choice at its step 9, and the technologies reach the
   *  Laboratory at step 10 and the achievements the score track at step 11.
   *
   *  Which is why an empty strip would not do. With the slots emptied on the
   *  wire, `LabStrip` draws six blanks and `GoalsStrip` draws nothing, and both
   *  read as a screen that failed rather than as a table that is not there
   *  yet. */
  const withheld = (key: SectionKey) => (
    <section className={`table-sec ${key} collapsed`} key={key} aria-label={SECTION_LABELS[key]}>
      <h2>{SECTION_LABELS[key]}</h2>
      <p className="muted">revealed once you have chosen your card</p>
    </section>
  );

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
      {error && <div className="error">{error}</div>}
    </div>
  );
}
