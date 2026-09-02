import { useState } from "react";
import type { CSSProperties } from "react";
import type React from "react";
import type { ArtManifest, BoardModel, Labels, PlayerColor, TileModel } from "./types";
import { areaArt, fivePowerArt, fivePowerWords } from "./art";
import { SiteFace } from "./Cards";
import { Pieces, piecesWords } from "./Pieces";
import { ID_BADGE, PIECES, pct, SITE_CARD } from "./tileRects";
import { TilePopover, type Anchor } from "./TilePopover";

/** How wide a tile may be, so that the whole board fits its column at 1440.
 *
 *  **Keyed on the REALIZED widest row, never on `board.player_count`.** The two
 *  give the same three answers on every board the engine generates, and the
 *  widest row is the term the width actually depends on: the board's min-content
 *  width is `n · tile + (n−1) · gap` for the widest row `n`. Reading
 *  `player_count` would also make this file the first thing in the client to
 *  assume the pyramid's shape, against this module's own doc below.
 *
 *  The arithmetic, at 1440 with a 992px board column (`1440 − 40 padding − 24
 *  gap − 384 rail`): 6 tiles at 10rem is `6×160 + 5×6.4 = 992`, 5 at 12rem is
 *  `985.6`, 4 at 14rem is `915.2`. Before this the board carried 384px (4P) and
 *  154px (3P) outside its column behind `overflow-x: auto`, and `.board` drew no
 *  scrollbar — so on overlay-scrollbar platforms a Movement destination in the
 *  last column was invisible and undiscoverable.
 *
 *  **The 4P fit is exact and its margin is 0.047px** — Chromium resolves the
 *  0.4rem gap to 6.390625px, so the row is 991.953 against 992. One pixel of
 *  viewport loss, or a classic 15px scrollbar, re-breaks it. That is why the
 *  guard in `Board.test.tsx` asserts THIS LADDER and not the pixel, and why the
 *  browser walk records the board's residual scroll rather than gating it: a
 *  pixel assertion here would be green on one machine and red on another.
 *  Nothing from 1201 to 1439 is fixed by this at all — the row layout holds
 *  across that whole band and 4P still clips 239px at 1201. */
function tileWidth(ordered: [number, TileModel[]][]): string {
  const widest = Math.max(0, ...ordered.map(([, list]) => list.length));
  if (widest >= 6) return "10rem";
  if (widest >= 5) return "12rem";
  return "14rem";
}

/** The id badge's box: the rect's `width` becomes a `min-width`, and the rect's
 *  horizontal CENTRE becomes the anchor (see the badge's own comment below). */
const idBadgeStyle: CSSProperties = {
  left: `${((ID_BADGE.left + ID_BADGE.width / 2) * 100).toFixed(2)}%`,
  top: pct(ID_BADGE).top,
  height: pct(ID_BADGE).height,
  minWidth: pct(ID_BADGE).width,
};

/** The staggered pyramid, drawn from `grid_position` and nothing else.
 *
 * Row `r` is inset by half a tile, which is exactly the brick offset the
 * engine's adjacency encodes: row `r+1`'s cell `j` sits in the notch between
 * row `r`'s cells `j` and `j+1`. Nothing here assumes a pyramid — any geometry
 * the engine hands over renders, which is the same property the CLI's renderer
 * was built for. */
export function Board({
  board,
  art,
  labels,
  colors,
  onTileClick,
  selected,
  busy,
}: {
  board: BoardModel;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  onTileClick?: (area: number) => void;
  selected?: number[];
  /** A request is in flight. The tile is a mutation control like any option
   *  button and takes the same guard — it was the one that did not, and a
   *  double-click on a placement toggle could leave the highlight disagreeing
   *  with the server. */
  busy?: boolean;
}) {
  const rows = new Map<number, TileModel[]>();
  for (const tile of board.tiles) {
    const r = tile.grid_position[0];
    const list = rows.get(r) ?? [];
    list.push(tile);
    rows.set(r, list);
  }
  const ordered = [...rows.entries()].sort((a, b) => a[0] - b[0]);
  for (const [, list] of ordered) list.sort((a, b) => a.grid_position[1] - b.grid_position[1]);

  // The popover's whole state: which tile, and where it is. One at a time.
  const [pop, setPop] = useState<{ area: number; anchor: Anchor } | null>(null);
  const show = (area: number, el: HTMLElement) => {
    const r = el.getBoundingClientRect();
    setPop({ area, anchor: { left: r.left, top: r.top, right: r.right, bottom: r.bottom } });
  };
  const hide = () => setPop(null);
  const popTile = pop ? board.tiles.find((t) => t.area === pop.area) : undefined;

  return (
    <div className="board" style={{ "--tile-w": tileWidth(ordered) } as CSSProperties}>
      {ordered.map(([r, tiles]) => (
        <div
          className="board-row"
          key={r}
          style={{ marginLeft: `calc((var(--tile-w) + 0.4rem) * ${r} / 2)` }}
        >
          {tiles.map((t) => (
            <Tile
              key={t.area}
              tile={t}
              art={art}
              labels={labels}
              colors={colors}
              onClick={onTileClick}
              isSelected={selected?.includes(t.area) ?? false}
              busy={busy ?? false}
              onShow={show}
              onHide={hide}
              popOpen={pop?.area === t.area}
            />
          ))}
        </div>
      ))}
      {pop && popTile && (
        <TilePopover tile={popTile} art={art} labels={labels} colors={colors} anchor={pop.anchor} />
      )}
      <Legend />
    </div>
  );
}

function Tile({
  tile,
  art,
  labels,
  colors,
  onClick,
  isSelected,
  busy,
  onShow,
  onHide,
  popOpen,
}: {
  tile: TileModel;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  onClick?: (area: number) => void;
  isSelected: boolean;
  busy: boolean;
  onShow: (area: number, el: HTMLElement) => void;
  onHide: () => void;
  popOpen: boolean;
}) {
  const name = labels?.area_types[tile.area_type] ?? tile.area_type;
  const isCanal = tile.area_type === "Canal";
  const id = `A${String(tile.area).padStart(2, "0")}`;
  const face = areaArt(art, tile.area_type);
  const tokenSrc = fivePowerArt(art, tile.token);

  // **The tile is the art, and the pieces sit on it where the physical ones do**
  // (M5 spec §3.2, owner ruling R4). No caption: the printed title IS the name,
  // the site card lies upright over the top-right quarter as in the owner's
  // photograph (W0b turned the crops, so nothing here rotates it), the pieces
  // stand in the full-width band below it, the id badge sits in the tile's
  // bottom margin centred under the ladder, and the 5-Power token sits in its
  // printed slot. Every rect comes from `tile_rects.json`, never typed here.
  // What the caption used to say in words is not gone to a screen reader, but
  // WHERE it rides depends on the element — which is the next paragraph, and it
  // is not one answer.
  // **The facts the caption used to carry, worded ONCE and spent twice.**
  //
  // On the inert `<div>` a screen reader walks the subtree, so these ride as
  // `.sr-only` spans and `Pieces` announces the pieces under its own
  // `role="img"` name. On a `<button>` neither happens: a button is a single
  // node to assistive technology and its `aria-label` overrides its contents,
  // so every span inside it and every nested name is suppressed. On an
  // interactive board that is EVERY non-Canal tile — so the same words are
  // folded into the label there, with the pieces prepended, and nothing a
  // sighted player can read on the art is missing from what is announced.
  const pieces = piecesWords(tile.presidents, tile.assistants, tile.has_station, colors);
  const facts = isCanal
    ? ["crossing only · 1 yen to the bank"]
    : [
        tile.token ? `5-Power ${fivePowerWords(tile.token)}` : "5-Power taken",
        ...(tile.site?.index != null ? [`Site ${tile.site.index}`] : []),
        ...(tile.reachable ? ["a Movement destination the engine offers"] : []),
      ];

  const body = (
    <>
      <div className="tile-art-box">
        {face ? (
          <img className="tile-art" src={face} alt="" loading="lazy" />
        ) : (
          <div className="tile-art-blank">
            <span className="tile-name">{name}</span>
          </div>
        )}
        {tokenSrc && (
          <span className="token-slot">
            <img src={tokenSrc} alt="" loading="lazy" />
          </span>
        )}
        {tile.site && (
          <div className="site-on-tile" style={pct(SITE_CARD)}>
            <SiteFace art={art} site={tile.site} colors={colors} />
          </div>
        )}
        {!isCanal && (
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
        {/* **The rect's width is a FLOOR here, not a ceiling, and that is the
            whole fix.** The badge's text is a fixed 8.5px while `id_badge.width`
            is a fraction of `--tile-w`, which drops from 14rem at 2P to 10rem
            at 4P — so the ground came out at 13.6px holding ~16.3px of "A17",
            and the last glyph spilled off the dark pill onto the pale print,
            white on cream and unreadable. (Seen at gate V1; 2P fitted with
            2.7px to spare, which is why it went unnoticed, and 3P sits on the
            boundary.) Sizing the ground to its content cannot drift with the
            tile, where no fixed fraction can be right at every player count.
            The anchor moves to the rect's own CENTRE so growing the box keeps
            it centred — derived from the rect, never a written-down 50%. */}
        <span className="tile-id" style={idBadgeStyle}>
          {id}
        </span>
      </div>
      <span className="sr-only">
        {id} {name}
      </span>
      {facts.map((f) => (
        <span className="sr-only" key={f}>
          {f}
        </span>
      ))}
    </>
  );

  // **The prefix is load-bearing and stays first**: five component cells find a
  // tile by `^A\d\d ` — `App.test.tsx`'s `tile()` helper, used four times, and
  // the disabled-board cell in `Board.test.tsx`. NOT the browser harness, which
  // selects `button.tile:not([disabled])` by CSS and never reads the label.
  const label = [
    `${id} ${name}${isSelected ? " — selected" : ""}`,
    ...(pieces ? [pieces] : []),
    ...facts,
  ].join(" · ");

  const className = [
    "tile",
    tile.reachable ? "reachable" : "",
    isCanal ? "canal" : "",
    isSelected ? "selected" : "",
    face ? "has-art" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const hoverable = {
    onMouseEnter: (e: React.MouseEvent<HTMLElement>) => onShow(tile.area, e.currentTarget),
    onMouseLeave: onHide,
    onFocus: (e: React.FocusEvent<HTMLElement>) => onShow(tile.area, e.currentTarget),
    onBlur: onHide,
    onKeyDown: (e: React.KeyboardEvent<HTMLElement>) => {
      if (e.key === "Escape") onHide();
    },
    ...(popOpen ? { "aria-describedby": `pop-${id}` } : {}),
  };

  if (!onClick || isCanal)
    return (
      <div className={className} tabIndex={0} {...hoverable}>
        {body}
      </div>
    );
  return (
    <button
      type="button"
      className={className}
      disabled={busy}
      aria-pressed={isSelected}
      aria-label={label}
      onClick={() => onClick(tile.area)}
      {...hoverable}
    >
      {body}
    </button>
  );
}

/** What the marks mean. Worded for the pieces, since the tile now draws them
 *  as pieces rather than as the terminal's glyph alphabet. */
function Legend() {
  return (
    <details className="legend">
      <summary>marks</summary>
      <div>
        a pawn is a president and a cube an assistant, each in its owner&rsquo;s colour (a number
        beside a cube counts a stack) &middot; the small card on a tile is its building site, with a
        built space in its builder&rsquo;s colour &middot; the token in the printed 5-space is the
        5-Power bonus, and an empty frame means it is taken
      </div>
      <div>
        a green border is a Movement destination the engine offers &middot; a blue border is a tile
        you have selected &middot; the id badge (A00&hellip;) is what every prompt names
      </div>
    </details>
  );
}
