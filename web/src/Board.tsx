import type { CSSProperties } from "react";
import type { ArtManifest, BoardModel, Labels, PlayerColor, SiteModel, TileModel } from "./types";
import { areaArt, fivePowerArt, fivePowerWords } from "./art";
import { Pawns, seatClass } from "./Cards";

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
      {/* **The picture, then the caption — not text over a dimmed picture.**
          The printed power ladder 1–5 and its per-rung yields run across the
          bottom of every area art, and they are the facts a Movement
          destination and an Area Action get chosen on. Under the old veil
          (`--art-veil: 0.82` as a whole-image opacity, plus a full-bleed
          six-stop scrim) they reached the viewer at a mean transmittance of
          0.100 — measured white-vs-black in real Chromium — and the board
          carried them NOWHERE else: the only surface that draws the ladder is
          the Areas panel, which covers 98.8% of the board it is explaining.

          The alternative considered and REJECTED on measurement was two scrims
          anchored to the text bands. It cannot work, for a structural reason
          rather than a tuning one: the printed ladder occupies y 0.630–0.910 of
          the tile and this client's own lower text block occupies y
          0.607–0.956. They are the same 35% of the tile, so a scrim on the text
          is a scrim on the ladder — constructed and measured at 0.087, WORSE
          than the veil it would replace.

          This is the pattern `Cards.tsx` already chose and defends in its own
          words, applied to the one surface that did not get it. */}
      {face && (
        <div className="tile-art-box">
          <img className="tile-art" src={face} alt="" loading="lazy" />
          {/* The 5-Power token, where the physical one sits.
              `01-rules-reference.md:680`: the tokens are shuffled face-down and
              then placed FACE-UP on each Area Board's 5-space, "so its effect is
              public information from the start" — so there is no masking
              question here and never was; the wire has always carried
              `tile.token` on every tile.
              The slot rect in `styles.css` is MEASURED, not guessed: each of the
              thirteen non-Canal arts minus its own box blur, then averaged, so a
              line printed on every card survives and every illustration cancels.
              An absent token needs no mark of its own — the printed frame with
              its red 5 showing IS the empty slot, exactly as on the table — and
              the words below carry WHICH good, which the picture does not
              resolve at a 4P tile's 160px. */}
          {tokenSrc && (
            <span className="token-slot">
              <img src={tokenSrc} alt="" loading="lazy" />
            </span>
          )}
        </div>
      )}
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
