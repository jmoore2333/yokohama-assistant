// A tile at full size, on hover or keyboard focus.
//
// At the 10rem floor the site card on a tile is ~70px wide, below what its
// printed values need (M5 spec §3.2.2). This is the Areas drawer's row for ONE
// tile brought to the tile: the art at 22rem so the printed ladder is legible,
// the site card at 12rem, and the pieces in words. It is `role="tooltip"`,
// never takes focus, never sits in the tab order, and `pointer-events: none`
// so leaving the tile closes it. Positioned from the tile's rect in JS rather
// than with CSS anchor positioning, which Safari does not have.

import type { ArtManifest, Labels, PlayerColor, TileModel } from "./types";
import { areaArt, fivePowerWords } from "./art";
import { SiteFace } from "./Cards";
import { piecesWords } from "./Pieces";

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
 *  makes the flip to the left side happen a little early, never late. */
export function popoverPosition(
  anchor: Anchor,
  viewport: { width: number; height: number },
  size = { width: 368, height: 434 },
): { left: number; top: number } {
  const gap = 8;
  const left =
    anchor.right + gap + size.width <= viewport.width
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
}: {
  tile: TileModel;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  anchor: Anchor;
}) {
  const name = labels?.area_types[tile.area_type] ?? tile.area_type;
  const id = `A${String(tile.area).padStart(2, "0")}`;
  const face = areaArt(art, tile.area_type);
  const viewport =
    typeof window === "undefined"
      ? { width: 1440, height: 900 }
      : { width: window.innerWidth, height: window.innerHeight };
  const pos = popoverPosition(anchor, viewport);
  const pieces = piecesWords(tile.presidents, tile.assistants, tile.has_station, colors);
  return (
    <div className="tile-pop" role="tooltip" id={`pop-${id}`} style={{ left: pos.left, top: pos.top }}>
      <div className="tile-pop-head">
        <span className="tile-pop-id">{id}</span> <b>{name}</b>
      </div>
      {face && <img className="tile-pop-art" src={face} alt="" />}
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
