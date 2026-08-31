import type { BoardModel, Labels, PlayerColor, TileModel } from "./types";

const seatClass = (c: PlayerColor | undefined) =>
  c ? `seat-${c.toLowerCase()}` : "";

/** The staggered pyramid, drawn from `grid_position` and nothing else.
 *
 * Row `r` is inset by half a tile, which is exactly the brick offset the
 * engine's adjacency encodes: row `r+1`'s cell `j` sits in the notch between
 * row `r`'s cells `j` and `j+1`. Nothing here assumes a pyramid — any geometry
 * the engine hands over renders, which is the same property the CLI's renderer
 * was built for. */
export function Board({
  board,
  labels,
  colors,
  onTileClick,
  selected,
}: {
  board: BoardModel;
  labels: Labels | null;
  colors: PlayerColor[];
  onTileClick?: (area: number) => void;
  selected?: number[];
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
              labels={labels}
              colors={colors}
              onClick={onTileClick}
              isSelected={selected?.includes(t.area) ?? false}
            />
          ))}
        </div>
      ))}
    </div>
  );
}

function Tile({
  tile,
  labels,
  colors,
  onClick,
  isSelected,
}: {
  tile: TileModel;
  labels: Labels | null;
  colors: PlayerColor[];
  onClick?: (area: number) => void;
  isSelected: boolean;
}) {
  const name = labels?.area_types[tile.area_type] ?? tile.area_type;
  const isCanal = tile.area_type === "Canal";
  const id = `A${String(tile.area).padStart(2, "0")}`;
  const occupants = tile.assistants.filter(([, n]) => n > 0);

  const body = (
    <>
      <div className="tile-head">
        <span className="tile-id">{id}</span>
        <span className="tile-name">{name}</span>
      </div>
      {isCanal ? (
        <div className="tile-row muted">crossing only · 1 yen to the bank</div>
      ) : (
        <>
          <div className="tile-row">
            {tile.presidents.length === 0 && occupants.length === 0 ? (
              <span className="tile-row muted">no pawns</span>
            ) : (
              <>
                {tile.presidents.map((seat) => (
                  <span key={`p${seat}`} className={`pawn ${seatClass(colors[seat])}`} title="President">
                    &#9650;
                  </span>
                ))}
                {occupants.map(([seat, n]) => (
                  <span key={`a${seat}`} className={`pawn ${seatClass(colors[seat])}`} title="assistants">
                    &#9679;{n}
                  </span>
                ))}
              </>
            )}
            {tile.has_station && <span className="chip">St</span>}
          </div>
          <div className="tile-row muted">
            {tile.site?.index != null && <span className="chip">Site {tile.site.index}</span>}
            {tile.site && <span className="chip vp">TH {tile.site.trading_house_vp}</span>}
            {tile.token != null && <span className="chip yen">5&#8209;Power</span>}
          </div>
        </>
      )}
    </>
  );

  const className = [
    "tile",
    tile.reachable ? "reachable" : "",
    isCanal ? "canal" : "",
    isSelected ? "reachable" : "",
  ]
    .filter(Boolean)
    .join(" ");

  if (!onClick || isCanal) return <div className={className}>{body}</div>;
  return (
    <button type="button" className={className} onClick={() => onClick(tile.area)}>
      {body}
    </button>
  );
}
