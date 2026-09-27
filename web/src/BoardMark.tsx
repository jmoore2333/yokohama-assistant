// The door's board: an ORIGINAL drawing, and the one picture on the setup
// screens (M6.5c2, R-6.5c2.6; spec §2.2).
//
// **Never a crop of the card art.** `ASSETS.md` carves the card and tile
// images out as carrying no grant of any kind, and these screens must stand
// without them for a self-hoster or a public release. So the board is DRAWN,
// in the app icon's spirit (`web/public/icons/icon.svg`): the ten areas of a
// two-player board in its 4/3/2/1 pyramid, brick-offset by centring the way
// `board_gen` lays one out, with two presidents and five assistants on it.
//
// **Tinted by what each area trades, in the stylesheet's own trade colours**:
// the four goods at the four production areas, the yen at the bank, victory
// points at the port and imported goods at customs. The three areas that trade
// no good — the church, the laboratory and the employment agency — are the
// neutral greys. Chinatown is not on a two-player board, so it is not here.
// The hues are CLASSES (`styles.css`, "the board mark"), so a token changed
// there changes the drawing and no colour is restated in this file.

import { seatClass } from "./Cards";
import type { PlayerColor } from "./types";

/** What a tile trades: a class `styles.css` colours. */
export type Trade = "copper" | "silk" | "tea" | "seafood" | "yen" | "vp" | "imports" | "neutral" | "faint";

/** One tile: its row from the top, its place in the row, what it trades, and
 *  how strongly it is tinted: the approved mock's weights, except customs and
 *  the employment agency, re-weighted with the hues this drawing gives them. */
export interface Tile {
  row: number;
  col: number;
  trade: Trade;
  alpha: number;
}

/** The two-player board: rows of four, three, two and one. */
export const TILES: readonly Tile[] = [
  { row: 0, col: 0, trade: "neutral", alpha: 0.55 }, // the church
  { row: 0, col: 1, trade: "tea", alpha: 0.62 }, // the tea plantation
  { row: 0, col: 2, trade: "seafood", alpha: 0.55 }, // the fishing ground
  { row: 0, col: 3, trade: "silk", alpha: 0.45 }, // the silk mill
  { row: 1, col: 0, trade: "vp", alpha: 0.6 }, // the port
  { row: 1, col: 1, trade: "copper", alpha: 0.65 }, // the copper mine
  { row: 1, col: 2, trade: "yen", alpha: 0.85 }, // the bank
  { row: 2, col: 0, trade: "imports", alpha: 0.6 }, // customs
  { row: 2, col: 1, trade: "faint", alpha: 0.5 }, // the laboratory
  { row: 3, col: 0, trade: "neutral", alpha: 0.45 }, // the employment agency
];

/** The drawing's own units: a tile is 96 by 68, a row 76 below the last and
 *  half a step (54) further in, a column 108 along. */
const W = 96;
const H = 68;
const STEP = 108;
const ROW = 76;
const INDENT = [0, 54, 108, 162] as const;

const PRESIDENTS: readonly { x: number; y: number; seat: PlayerColor }[] = [
  { x: 176, y: 30, seat: "Blue" },
  { x: 336, y: 104, seat: "Red" },
];

const CUBES: readonly { x: number; y: number; seat: PlayerColor }[] = [
  { x: 182, y: 116, seat: "Red" },
  { x: 199, y: 116, seat: "Red" },
  { x: 244, y: 40, seat: "Blue" },
  { x: 128, y: 192, seat: "Red" },
  { x: 236, y: 192, seat: "Blue" },
];

export function BoardMark({ width }: { width: number }) {
  return (
    <svg
      className="board-mark"
      width={width}
      height={Math.round((width * 296) / 420)}
      viewBox="0 0 420 296"
      aria-hidden="true"
      focusable="false"
    >
      <g className="bm-tiles">
        {TILES.map((t) => (
          <rect
            key={`${t.row}-${t.col}`}
            className={`bm-tile ${t.trade}`}
            data-row={t.row}
            x={INDENT[t.row]! + t.col * STEP}
            y={t.row * ROW}
            width={W}
            height={H}
            rx={7}
            fillOpacity={t.alpha}
          />
        ))}
      </g>
      {PRESIDENTS.map((p) => (
        <g key={`${p.x}-${p.y}`} className={`bm-president ${seatClass(p.seat)}`}>
          <circle className="bm-disc" cx={p.x} cy={p.y} r={12} />
          <circle className="bm-hole" cx={p.x} cy={p.y} r={5} />
        </g>
      ))}
      {CUBES.map((c) => (
        <rect
          key={`${c.x}-${c.y}`}
          className={`bm-cube ${seatClass(c.seat)}`}
          x={c.x}
          y={c.y}
          width={13}
          height={13}
          rx={2}
        />
      ))}
    </svg>
  );
}
