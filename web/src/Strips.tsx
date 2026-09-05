// The strips on the table (M5 spec §3.4): the physical boards' shapes, drawn
// from the same payloads the drawers used to draw, laid out the way the boards
// lie on the table — six slots in a row — rather than the way a 704px drawer
// stacked them. No board art yet (R6); the shape is the board.

import type {
  AchievementModel,
  AreaCard,
  AreasPanel,
  ArtManifest,
  Labels,
  LabsPanel,
  PlayerColor,
  PortsPanel,
  TracksPanel,
} from "./types";
import { AchievementCardView, OrderCardView, SiteFace, TechCardView } from "./Cards";
import { Tracks } from "./Dashboard";

/** The zone gates FOLDED FROM THE SLOTS, never typed as prose. */
function gateWords(panel: PortsPanel): string {
  const gates = new Map<string, number>();
  for (const b of panel.boards) {
    for (const s of b.slots) {
      const seen = gates.get(s.zone);
      if (seen === undefined || s.min_power < seen) gates.set(s.zone, s.min_power);
    }
  }
  return [...gates.entries()]
    .sort((a, b) => a[1] - b[1])
    .map(([zone, power]) => `${zone.toUpperCase()} needs power ${power}`)
    .join(", ");
}

/** The slot keys the walk is lighting, in `diffFrames`' own shape.
 *
 *  **`<board>` is the ARRAY INDEX, not `board_b`.** The diff builds the key by
 *  mapping `panel.boards`, so the reader has to index the same list the same
 *  way; at 2P the two readings coincide and at 3–4P they do not. */
const litSlot = (changed: Set<string> | undefined, kind: string, board: number, slot: number) =>
  changed?.has(`${kind}:${board}:${slot}`) ? " changed" : "";

export function PortStrip({
  panel,
  art,
  changed,
}: {
  panel: PortsPanel;
  art: ArtManifest | null;
  changed?: Set<string>;
}) {
  return (
    <>
      {panel.boards.map((b, i) => (
        <section className="table-sec ports" key={String(b.board_b)} aria-label={`Port ${b.board_b ? "B" : "A"}`}>
          <h2>
            Port {b.board_b ? "B" : "A"}
            <span className="meta">
              {gateWords(panel)} &middot; {panel.deck_remaining} in the deck
            </span>
          </h2>
          <div className="board-strip">
            {b.slots.map((s) => (
              <div
                className={`strip-slot zone-${s.zone.toLowerCase()}${litSlot(changed, "port", i, s.slot)}`}
                key={s.slot}
              >
                <div className="slot-head">
                  <span className={`chip zone-${s.zone.toLowerCase()}`}>{s.zone}</span>
                  <span className="chip">power {s.min_power}+</span>
                </div>
                {s.card == null ? (
                  <div className="card-blank empty">empty</div>
                ) : (
                  <OrderCardView art={art} card={s.printed} id={s.card} />
                )}
              </div>
            ))}
          </div>
        </section>
      ))}
    </>
  );
}

export function LabStrip({
  panel,
  art,
  labels,
  changed,
}: {
  panel: LabsPanel;
  art: ArtManifest | null;
  labels: Labels | null;
  changed?: Set<string>;
}) {
  return (
    <>
      {panel.boards.map((b, i) => (
        <section className="table-sec labs" key={String(b.board_b)} aria-label={`Laboratory ${b.board_b ? "B" : "A"}`}>
          <h2>
            Laboratory {b.board_b ? "B" : "A"}
            <span className="meta">pay the industry; the slot&rsquo;s surcharge is in the figure &middot; {panel.deck_remaining} in the deck</span>
          </h2>
          <div className="board-strip">
            {b.slots.map((s) => (
              <div className={`strip-slot${litSlot(changed, "lab", i, s.slot)}`} key={s.slot}>
                <div className="slot-head">
                  <span className="chip">slot {s.slot + 1}</span>
                  {s.surcharge > 0 && <span className="chip warn">+{s.surcharge}</span>}
                </div>
                {s.card == null ? (
                  <div className="card-blank empty">empty</div>
                ) : (
                  <TechCardView art={art} card={s.printed} labels={labels} cost={s.printed ? s.printed.industry_cost + s.surcharge : undefined} />
                )}
              </div>
            ))}
          </div>
        </section>
      ))}
    </>
  );
}

export function TracksStrip({ tracks, art, colors }: { tracks: TracksPanel; art: ArtManifest | null; colors: PlayerColor[] }) {
  return (
    <section className="table-sec tracks" aria-label="Tracks">
      <Tracks tracks={tracks} art={art} colors={colors} />
    </section>
  );
}

export function GoalsStrip({ cards, art, labels, colors }: { cards: AchievementModel[]; art: ArtManifest | null; labels: Labels | null; colors: PlayerColor[] }) {
  return (
    <section className="table-sec achievements" aria-label="Goals">
      <h2>Goals <span className="meta">one per category, first / later VP</span></h2>
      <div className="card-strip">
        {cards.map((c) => (
          <AchievementCardView key={c.achievement_type} model={c} art={art} labels={labels} colors={colors} />
        ))}
      </div>
    </section>
  );
}

function SiteCard({ card, art, colors }: { card: AreaCard; art: ArtManifest | null; colors: PlayerColor[] }) {
  const site = card.tile.site!;
  const id = `A${String(card.tile.area).padStart(2, "0")}`;
  return (
    <figure className="site-card">
      <SiteFace art={art} site={site} colors={colors} />
      <figcaption>
        {/* `index` is null where no printed card matches the tile's spaces —
            `SiteFace` guards it and this caption did not, so the strip printed
            "Site null" at the reader. Same guard, same words as the face. */}
        <span className="tile-id-words">{id}</span>{" "}
        <b>{site.index == null ? "site" : `Site ${site.index}`}</b>
        <span className="muted"> &middot; trading house &#9733;{site.trading_house_vp}</span>
      </figcaption>
    </figure>
  );
}

/** Reads the AREAS frame, never `/index/sites`: the index clones the pristine
 *  registry card and nothing is ever built on it (`types.ts`'s own caveat). */
export function SitesStrip({ panel, art, colors }: { panel: AreasPanel; art: ArtManifest | null; colors: PlayerColor[] }) {
  const cards = panel.rows.flat().filter((c) => c.tile.site !== null);
  return (
    <section className="table-sec buildings" aria-label="Building sites">
      <h2>Building sites <span className="meta">a space in a seat&rsquo;s colour is built on; the band is the trading house</span></h2>
      {cards.length === 0 ? (
        <div className="muted">This board has no building sites.</div>
      ) : (
        <div className="site-grid">
          {cards.map((card) => (
            <SiteCard key={card.tile.area} card={card} art={art} colors={colors} />
          ))}
        </div>
      )}
    </section>
  );
}
