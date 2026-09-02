// The enabled sections under the pyramid (M5 spec §3.4).
//
// **Refresh on every state change.** The drawers fetched on demand because
// "none of them changes between decisions in a way a player is watching for";
// a section on the table is exactly that watching, so an enabled fetched
// section re-fetches whenever `state` changes — every `/input`, re-read and
// create — on the scrubber's own debounce-and-drop shape: a `live` flag, a
// stale answer dropped. Tracks and Goals need no fetch; they are on the frame.
//
// **Replaying collapses the fetched ones.** `/at/{k}` is a narrow frame (M4
// Q5): board and dashboard only. The port at position k is not something the
// server serves, so the section says so rather than showing the live offer
// under a historical board. Tracks and Goals draw from the historical frame.

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

const FETCHED: SectionKey[] = ["ports", "labs", "buildings"];

export function TableSections({
  enabled,
  game,
  state,
  replaying,
  view,
  art,
  labels,
  colors,
}: {
  enabled: SectionKey[];
  game: string;
  /** The live state — its identity is the refresh key. */
  state: ClientState;
  replaying: boolean;
  /** What the board is drawing: the live state, or the replay position. */
  view: ClientState | HistoricalFrame;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
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
    if (replaying) return;
    let live = true;
    setError(null);
    const said = (e: unknown) => live && setError(e instanceof ApiError ? e.message : String(e));
    if (enabled.includes("ports")) api.ports(game).then((p) => live && setPorts(p)).catch(said);
    if (enabled.includes("labs")) api.labs(game).then((p) => live && setLabs(p)).catch(said);
    if (enabled.includes("buildings")) api.areas(game).then((p) => live && setAreas(p)).catch(said);
    return () => {
      live = false;
    };
    // `wanted` stands in for `enabled` here on purpose (see its comment above);
    // the array itself is rebuilt on every toggle and would refetch everything.
  }, [game, state, replaying, wanted]);

  const tracksPanel = view.frame.dashboard.find((p) => "Tracks" in p);
  const tracks = tracksPanel && "Tracks" in tracksPanel ? tracksPanel.Tracks : null;
  const goalsPanel = view.frame.dashboard.find((p) => "Achievements" in p);
  const goals = goalsPanel && "Achievements" in goalsPanel ? goalsPanel.Achievements.cards : [];

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

  return (
    <div className="table-sections">
      {SECTION_KEYS.filter((k) => enabled.includes(k)).map((key) => {
        if (replaying && FETCHED.includes(key)) return collapsed(key);
        switch (key) {
          case "ports":
            return ports ? <PortStrip key={key} panel={ports} art={art} /> : loading(key);
          case "labs":
            return labs ? <LabStrip key={key} panel={labs} art={art} labels={labels} /> : loading(key);
          case "buildings":
            return areas ? <SitesStrip key={key} panel={areas} art={art} colors={colors} /> : loading(key);
          case "tracks":
            return tracks ? <TracksStrip key={key} tracks={tracks} art={art} colors={colors} /> : null;
          case "achievements":
            return <GoalsStrip key={key} cards={goals} art={art} labels={labels} colors={colors} />;
        }
      })}
      {error && <div className="error">{error}</div>}
    </div>
  );
}
