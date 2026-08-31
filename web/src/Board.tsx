import type { ArtManifest, BoardModel, Labels, PlayerColor, SiteModel, TileModel } from "./types";
import { areaArt, fivePowerArt, fivePowerWords } from "./art";
import { Pawns, seatClass } from "./Cards";

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

  return (
    <div className="board">
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
            />
          ))}
        </div>
      ))}
      <Legend />
    </div>
  );
}

/** The site-slot strip: the four printed spaces, who built on each, and the
 *  trading-house VP.
 *
 *  This is the CLI's `sites ★1 ★2¥1 — ★2  │ T8` line, and it is the densest
 *  thing on a tile for a reason — it is what a player is reading when they
 *  decide where to build. A BLANK space prints as `—` rather than as nothing,
 *  because a card with three rewards and a card with four are different cards
 *  and a gap that is not drawn cannot be counted. */
function SiteSlots({ site, colors }: { site: SiteModel; colors: PlayerColor[] }) {
  return (
    <div className="site-strip" title="the four shophouse spaces, then the trading-house VP">
      {site.slots.map((slot, i) => {
        const r = slot.reward;
        const parts: string[] = [];
        if (r.vp) parts.push(`★${r.vp}`);
        if (r.yen) parts.push(`¥${r.yen}`);
        if (r.seafood) parts.push(`f${r.seafood}`);
        if (r.tea) parts.push(`t${r.tea}`);
        if (r.silk) parts.push(`s${r.silk}`);
        if (r.copper) parts.push(`c${r.copper}`);
        if (r.any_good) parts.push(`*${r.any_good}`);
        // The CLI's alphabet, not one of our own. `board_render::push_reward`
        // prints `movement` as `a` — the YAML's `assistants` icon, which the
        // engine names `movement` because the icon MOVES an assistant — and
        // `warehouse_tokens` as the built-house glyph. A browser inventing `m`
        // and `w` gives a player cross-reading the two surfaces an unknown
        // token, which is the one thing a shared legend is for.
        if (r.movement) parts.push(`a${r.movement}`);
        if (r.warehouse_tokens) parts.push(`\u2302${r.warehouse_tokens}`);
        const built = slot.built_by;
        return (
          <span
            className={`slot ${built != null ? `built ${seatClass(colors[built])}` : ""}`}
            key={i}
          >
            {built != null && <span className="built-mark">&#8962;</span>}
            {parts.length === 0 ? "—" : parts.join("")}
          </span>
        );
      })}
      <span
        className={`slot th ${site.trading_house_owner != null ? `built ${seatClass(colors[site.trading_house_owner])}` : ""}`}
        title="trading-house space"
      >
        {site.trading_house_owner != null && <span className="built-mark">&#8962;</span>}T
        {site.trading_house_vp}
      </span>
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
}: {
  tile: TileModel;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  onClick?: (area: number) => void;
  isSelected: boolean;
  busy: boolean;
}) {
  const name = labels?.area_types[tile.area_type] ?? tile.area_type;
  const isCanal = tile.area_type === "Canal";
  const id = `A${String(tile.area).padStart(2, "0")}`;
  const face = areaArt(art, tile.area_type);
  const tokenSrc = fivePowerArt(art, tile.token);

  const body = (
    <>
      {face && <img className="tile-art" src={face} alt="" loading="lazy" />}
      <div className="tile-body">
        <div className="tile-head">
          <span className="tile-id">{id}</span>
          <span className="tile-name">{name}</span>
          {tile.reachable && (
            <span className="reach" title="a Movement destination the engine offers">
              &#9672;
            </span>
          )}
        </div>
        {isCanal ? (
          <div className="tile-row muted">crossing only &middot; 1 yen to the bank</div>
        ) : (
          <>
            <div className="tile-row pawns">
              <Pawns
                presidents={tile.presidents}
                assistants={tile.assistants}
                hasStation={tile.has_station}
                colors={colors}
              />
            </div>
            <div className="tile-row token-row">
              {tile.token ? (
                <span className="five-power" title="5-Power bonus token">
                  {tokenSrc && <img src={tokenSrc} alt="" loading="lazy" />}
                  {fivePowerWords(tile.token)}
                </span>
              ) : (
                <span className="muted">5-Power taken</span>
              )}
              {tile.site?.index != null && <span className="chip site-id">Site {tile.site.index}</span>}
            </div>
            {tile.site && <SiteSlots site={tile.site} colors={colors} />}
          </>
        )}
      </div>
    </>
  );

  const className = [
    "tile",
    tile.reachable ? "reachable" : "",
    isCanal ? "canal" : "",
    isSelected ? "selected" : "",
    face ? "has-art" : "",
  ]
    .filter(Boolean)
    .join(" ");

  if (!onClick || isCanal) return <div className={className}>{body}</div>;
  return (
    <button
      type="button"
      className={className}
      disabled={busy}
      aria-pressed={isSelected}
      aria-label={`${id} ${name}${isSelected ? " — selected" : ""}`}
      onClick={() => onClick(tile.area)}
    >
      {body}
    </button>
  );
}

/** What the marks mean.
 *
 *  The CLI prints this under every board, and it is not decoration: `★n` and
 *  `f2` and `⌂` are a private alphabet, and a surface that uses one without
 *  saying so is a surface you have to already know. Worded from the terminal's
 *  own legend so the two teach the same thing. */
function Legend() {
  return (
    <details className="legend">
      <summary>marks</summary>
      <div>
        <b>&#9650;</b> president (owner colour) &middot; <b>&#9679;n</b> assistants &middot;{" "}
        <b>&#9672;</b> a Movement destination the engine offers &middot; <b>St</b> Station (+1
        Power for everyone)
      </div>
      <div>
        site slots: <b>&#8962;</b> built (owner colour) &middot; <b>★n</b> VP &middot; <b>¥n</b>{" "}
        yen &middot; <b>f/t/s/c n</b> goods &middot; <b>*n</b> any good &middot; <b>an</b>{" "}
        assistants &middot; <b>&#8962;n</b> build &middot; <b>Tn</b> trading-house VP &middot;{" "}
        <b>—</b> a blank space
      </div>
    </details>
  );
}
