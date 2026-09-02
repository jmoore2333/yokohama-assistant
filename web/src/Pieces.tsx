// The pieces standing on a tile, drawn as pieces.
//
// A president is a pawn silhouette and an assistant is a cube — the shape the
// TTS mod gives it (`BlockSquare`), to be confirmed against the box (M5 spec
// §3.2.1). Both are inline SVG in the seat's colour via `currentColor`, so a
// fifth seat colour is one CSS rule and not a new picture. Everything the old
// caption said in words rides on the group's accessible name.

import type { CSSProperties } from "react";
import type { ArtManifest, PlayerColor, PlayerId } from "./types";
import { seatClass } from "./Cards";
import { tokenArt } from "./art";

export function PresidentIcon() {
  return (
    <svg viewBox="0 0 20 28" className="piece president" aria-hidden="true">
      <path
        d="M10 1a4 4 0 0 1 2.6 7.1c2 1 3.2 2.9 3.2 5.2 0 1.6-.6 3-1.6 4.1l2.8 6.6H3l2.8-6.6A6.2 6.2 0 0 1 4.2 13.3c0-2.3 1.2-4.2 3.2-5.2A4 4 0 0 1 10 1z"
        fill="currentColor"
        stroke="#0b0e12"
        strokeWidth="1.2"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function CubeIcon() {
  return (
    <svg viewBox="0 0 20 20" className="piece cube" aria-hidden="true">
      <path d="M10 2 18 6v8l-8 4-8-4V6z" fill="currentColor" stroke="#0b0e12" strokeWidth="1.2" strokeLinejoin="round" />
      <path d="M2 6l8 4 8-4M10 10v8" fill="none" stroke="#0b0e12" strokeWidth="1" opacity=".55" />
    </svg>
  );
}

/** The words: `Red president · 2 Blue assistants · Station`, or `""` for nothing. */
export function piecesWords(
  presidents: PlayerId[],
  assistants: [PlayerId, number][],
  hasStation: boolean,
  colors: PlayerColor[],
): string {
  const seat = (s: PlayerId) => colors[s] ?? `seat ${s}`;
  return [
    ...presidents.map((s) => `${seat(s)} president`),
    ...assistants.filter(([, n]) => n > 0).map(([s, n]) => `${n} ${seat(s)} assistant${n === 1 ? "" : "s"}`),
    ...(hasStation ? ["Station"] : []),
  ].join(" · ");
}

export function Pieces({
  presidents,
  assistants,
  hasStation,
  colors,
  art,
}: {
  presidents: PlayerId[];
  assistants: [PlayerId, number][];
  hasStation: boolean;
  colors: PlayerColor[];
  art: ArtManifest | null;
}) {
  const words = piecesWords(presidents, assistants, hasStation, colors);
  if (!words) return null;
  const station = tokenArt(art, "station");
  // How many glyph widths stand in the row, which is what the band's CSS needs
  // to shrink them to fit rather than wrap (owner ruling R-G3): one per
  // president, one per assistant up to three, TWO from four (the cube plus its
  // count), one for the Station.
  const n =
    presidents.length +
    assistants.filter(([, k]) => k > 0).reduce((s, [, k]) => s + (k > 3 ? 2 : k), 0) +
    (hasStation ? 1 : 0);
  return (
    <div className="pieces" role="img" aria-label={words} style={{ "--piece-n": n } as CSSProperties}>
      {presidents.map((seat) => (
        <span key={`p${seat}`} className={`piece-group ${seatClass(colors[seat])}`}>
          <PresidentIcon />
        </span>
      ))}
      {assistants
        .filter(([, n]) => n > 0)
        .map(([seat, n]) => (
          <span key={`a${seat}`} className={`piece-group ${seatClass(colors[seat])}`}>
            {Array.from({ length: n > 3 ? 1 : n }, (_, i) => (
              <CubeIcon key={i} />
            ))}
            {n > 3 && <span className="piece-n">{n}</span>}
          </span>
        ))}
      {hasStation &&
        (station ? (
          <img className="piece station" src={station} alt="" loading="lazy" />
        ) : (
          <span className="chip station" title="Station — +1 Power for everyone here">
            St
          </span>
        ))}
    </div>
  );
}
