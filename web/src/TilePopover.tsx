// A tile at full size, on hover or keyboard focus.
//
// At the 10rem floor the site card on a tile is ~70px wide, below what its
// printed values need (M5 spec §3.2.2). This is the Areas drawer's row for ONE
// tile brought to the tile: the art at 22rem so the printed ladder is legible,
// the site card at 12rem, and the pieces as glyphs on the art AND in words —
// the glyphs being the SIGHTED look-closer view a crowded tile needs (owner
// ruling R-GA-1 at gate A of M5.5: "when hovering over a card the assistants
// don't appear while the workshop does"). The accessible sentence is now
// carried TWICE while the popover is open — by the band's own `aria-label`
// (`Pieces` is a `role="img"`) and by the words row, both inside the element
// the tile names in `aria-describedby` — so a screen reader hears the pieces
// described twice. That redundancy is accepted rather than fixed: the mount
// mirrors the board tile's exactly, and specialising `Pieces` for the popover
// is the drift this task exists to avoid. It is `role="tooltip"`,
// never takes focus, never sits in the tab order, and `pointer-events: none`
// so leaving the tile closes it. Positioned from the tile's rect in JS rather
// than with CSS anchor positioning, which Safari does not have.
//
// **`position: fixed` here is NOT re-parented by `.table`'s `container-type:
// size`, and that was measured rather than reasoned about** (final review):
// in Chromium 151 a fixed child of the size container stays at its viewport
// coordinates (a child at `top: 100` renders at 100, and `.table`'s
// `scrollWidth` is unmoved). So the popover is positioned against the viewport
// and kept inside the table by the arithmetic below, never by containment.

import type { ArtManifest, Labels, PlayerColor, TileModel } from "./types";
import { areaArt, fivePowerWords } from "./art";
import { SiteFace } from "./Cards";
import { Pieces, piecesWords } from "./Pieces";
import { PIECES, pct } from "./tileRects";

export interface Anchor {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** Right of the tile when there is room, else left; top-aligned, kept inside.
 *
 *  **The default height is MEASURED, not estimated**, because the bottom clamp
 *  is only as good as it: at the first draft's 420 the clamp put a popover on
 *  the board's last row at `top: 441` and its real bottom edge at 874.9 against
 *  an 869px viewport, so the words were cut and — the popover being `fixed` —
 *  nothing scrolled to reveal them. Read off the rendered box in Chromium at
 *  1710x869 with the art loaded: 352 x 433.875, of which the art is 250.1 and
 *  the site card 127.3. Rounded UP, so the estimate errs inside the viewport.
 *  The width stays the conservative 368 the flip was written against; it only
 *  makes the flip to the left side happen a little early, never late.
 *
 *  **`rightEdge` is the TABLE's right edge, and the flip is taken against it.**
 *  Against the viewport's, a right-column tile put the popover over the pinned
 *  answer bar: measured on the top-right tile at 1710x869, the popover spanned
 *  x 1122–1474 while `.table` ended at 1230, so it lay across the deck's own
 *  options while the pointer rested on the tile. It is one rule and not a
 *  breakpoint: below 1201px `.table` is `width: 100%` in the column stack with
 *  no padding of its own and `body` has no margin, so its right edge IS the
 *  viewport's and this term changes nothing there (spec R8). */
export function popoverPosition(
  anchor: Anchor,
  viewport: { width: number; height: number },
  size = { width: 368, height: 434 },
  rightEdge = viewport.width,
): { left: number; top: number } {
  const gap = 8;
  // Never outside the viewport either, whatever the caller measured.
  const rightBound = Math.min(rightEdge, viewport.width);
  const left =
    anchor.right + gap + size.width <= rightBound
      ? anchor.right + gap
      : Math.max(gap, anchor.left - gap - size.width);
  const top = Math.max(gap, Math.min(anchor.top, viewport.height - size.height - gap));
  return { left, top };
}

export function TilePopover({
  tile,
  art,
  labels,
  colors,
  anchor,
  rightEdge,
}: {
  tile: TileModel;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  anchor: Anchor;
  /** The table's right edge, measured when the popover was opened. Absent
   *  where there is no `.table` around the board (a component test), and the
   *  flip then falls back to the viewport. */
  rightEdge?: number | undefined;
}) {
  const name = labels?.area_types[tile.area_type] ?? tile.area_type;
  const id = `A${String(tile.area).padStart(2, "0")}`;
  const face = areaArt(art, tile.area_type);
  const viewport =
    typeof window === "undefined"
      ? { width: 1440, height: 900 }
      : { width: window.innerWidth, height: window.innerHeight };
  const pos = popoverPosition(anchor, viewport, undefined, rightEdge);
  const pieces = piecesWords(tile.presidents, tile.assistants, tile.has_station, colors);
  return (
    <div className="tile-pop" role="tooltip" id={`pop-${id}`} style={{ left: pos.left, top: pos.top }}>
      <div className="tile-pop-head">
        <span className="tile-pop-id">{id}</span> <b>{name}</b>
      </div>
      {/* The board tile's own mount, mirrored EXACTLY (`Board.tsx`'s
          `.tile-art-box`): the same props, the same rect, the same Canal
          exclusion. The popover must never decide anything the tile does not,
          so there is no second layout rule here and no popover-specific CSS
          for `.pieces` — the fit proof is one block. */}
      <div className="tile-pop-art-box">
        {face && <img className="tile-pop-art" src={face} alt="" />}
        {tile.area_type !== "Canal" && (
          <div className="pieces-band" style={pct(PIECES)}>
            <Pieces
              presidents={tile.presidents}
              assistants={tile.assistants}
              hasStation={tile.has_station}
              colors={colors}
              art={art}
            />
          </div>
        )}
      </div>
      <div className="tile-pop-row">
        {tile.site && <SiteFace art={art} site={tile.site} colors={colors} />}
        <div className="tile-pop-words">
          <div>{pieces || "no pieces here"}</div>
          {tile.area_type !== "Canal" && (
            <div>{tile.token ? `5-Power: ${fivePowerWords(tile.token)}` : "5-Power taken"}</div>
          )}
          {tile.site?.index != null && (
            <div>
              Site {tile.site.index} &middot; trading house &#9733;{tile.site.trading_house_vp}
            </div>
          )}
          {tile.reachable && <div className="reachable-words">a Movement destination the engine offers</div>}
        </div>
      </div>
    </div>
  );
}
