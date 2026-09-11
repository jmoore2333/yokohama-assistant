// The pieces standing on a tile, drawn as pieces.
//
// A president is a pawn silhouette and an assistant is a cube — the shape the
// TTS mod gives it (`BlockSquare`), to be confirmed against the box (M5 spec
// §3.2.1). Both are inline SVG in the seat's colour via `currentColor`, so a
// fifth seat colour is one CSS rule and not a new picture. Everything the old
// caption said in words rides on the group's accessible name.
//
// Every cube is drawn while the row stays legible — the three-cube cap is gone
// (M5.5 spec §3.1.1), and from four a colour is a two-row PILE. WHICH colour is
// a pile, a row of singles, or one cube and a count is `piecesLayout.ts`'s
// decision and not this file's: the component draws what the rule returns and
// hands the band the same `units` the rule priced, so the drawing and the fit
// cannot disagree about the row.

import type { CSSProperties } from "react";
import type { ArtManifest, PlayerColor, PlayerId } from "./types";
import { seatClass } from "./Cards";
import { tokenArt } from "./art";
import { bandAspect, layoutPieces } from "./piecesLayout";
import { PIECES } from "./tileRects";

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

/** The band's shape, fixed by its rect: what `layoutPieces` fits the crowd to. */
const BAND_ASPECT = bandAspect(PIECES);

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
  const groups = assistants.filter(([, n]) => n > 0);
  // The decision — pile, singles, or a cube and a count per colour — is the
  // pure rule's (M5.5 spec §3.1.1), and the band's CSS is handed the same
  // row width the rule computed, so the two cannot disagree about the fit.
  const layout = layoutPieces(
    { presidents: presidents.length, cubes: groups.map(([, n]) => n), station: hasStation },
    BAND_ASPECT,
  );
  return (
    // `title` carries the same words as the accessible name: the caption W4b
    // deleted was also the hover text, and a mouse user lost the only reading
    // of the pieces there was. `role="img"` keeps it one object either way.
    <div
      className="pieces"
      role="img"
      aria-label={words}
      title={words}
      style={{ "--piece-units": layout.units.toFixed(3) } as CSSProperties}
    >
      {presidents.map((seat) => (
        <span key={`p${seat}`} className={`piece-group ${seatClass(colors[seat])}`}>
          <PresidentIcon />
        </span>
      ))}
      {groups.map(([seat, n], i) => {
        const cls = `piece-group ${seatClass(colors[seat])}`;
        if (layout.collapsed.includes(i)) {
          return (
            <span key={`a${seat}`} className={cls}>
              <CubeIcon />
              <span className="piece-n">{n}</span>
            </span>
          );
        }
        if (layout.piles.includes(i)) {
          // Two rows, the odd cube on top; cubes in a pile touch, as on a table.
          const top = Math.ceil(n / 2);
          return (
            <span key={`a${seat}`} className={`${cls} pile`}>
              <span className="pile-row">
                {Array.from({ length: top }, (_, j) => <CubeIcon key={j} />)}
              </span>
              <span className="pile-row">
                {Array.from({ length: n - top }, (_, j) => <CubeIcon key={j} />)}
              </span>
            </span>
          );
        }
        return (
          <span key={`a${seat}`} className={cls}>
            {Array.from({ length: n }, (_, j) => <CubeIcon key={j} />)}
          </span>
        );
      })}
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
