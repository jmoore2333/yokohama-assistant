// The read-only surfaces: what is on the port boards, in the laboratories, on
// every area, and in the log.
//
// These are the four `GET`s that give the CLI's `ports`/`labs`/`areas`/`log`
// words a home in a browser. They are fetched on demand rather than folded into
// every frame: none of them changes between decisions in a way a player is
// watching for, and a payload that carried all four would pay for them on every
// move.

import { useEffect, useRef, useState } from "react";
import { api, ApiError } from "./api";
import type {
  ActionYield,
  AreaCard,
  AreasPanel,
  ArtManifest,
  Labels,
  LabsPanel,
  LogPanel,
  PlayerColor,
  PortsPanel,
} from "./types";
import { OrderCardView, Pawns, TechCardView } from "./Cards";

export type SurfaceName = "ports" | "labs" | "areas" | "log";

export const SURFACES: { key: SurfaceName; label: string; blurb: string }[] = [
  { key: "ports", label: "Port offer", blurb: "the cards on display, the zone each sits in, and the action power that zone takes" },
  { key: "labs", label: "Laboratory offer", blurb: "the technologies on display, what each does, and the industry it costs in its slot" },
  { key: "areas", label: "Areas", blurb: "what every area's action pays at power 1–5" },
  { key: "log", label: "Log", blurb: "what has happened, each line worded as it read when it applied" },
];

export function Surface({
  name,
  game,
  art,
  labels,
  colors,
  onClose,
}: {
  name: SurfaceName;
  game: string;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  onClose: () => void;
}) {
  // The payload is stored WITH the surface it was fetched for, and only
  // rendered when the two agree.
  //
  // Not defensiveness — a real bug this had. React renders before it runs
  // effects, so switching from Labs to Areas rendered the Areas branch against
  // the Labs payload for one frame, and `panel.rows.flat()` threw on a `rows`
  // that a `LabsPanel` does not have. Clearing the state inside the effect is
  // one render too late; the state has to carry its own identity.
  const [loaded, setLoaded] = useState<{ of: SurfaceName; data: unknown } | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setError(null);
    api[name](game)
      .then((d) => live && setLoaded({ of: name, data: d }))
      .catch((e) => live && setError(e instanceof ApiError ? e.message : String(e)));
    return () => {
      live = false;
    };
  }, [name, game]);

  const data = loaded?.of === name ? loaded.data : null;

  // Escape closes it, and opening it moves focus into it.
  //
  // The panel is `position: fixed` over the whole board, so without this a
  // keyboard user who opens it is left with focus on the nav button behind it
  // and no way out but Tab — forward through every remaining nav control and,
  // during a placement prompt, all eighteen board tiles now hidden underneath.
  const panel = useRef<HTMLDivElement>(null);
  useEffect(() => {
    panel.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const meta = SURFACES.find((s) => s.key === name);
  return (
    <div
      className="surface"
      role="dialog"
      aria-modal="true"
      aria-label={meta?.label}
      ref={panel}
      tabIndex={-1}
    >
      <header>
        <h2>{meta?.label}</h2>
        <span className="meta">{meta?.blurb}</span>
        <button type="button" className="ghost" onClick={onClose}>
          Close
        </button>
      </header>
      <div className="surface-body">
        {error && <div className="error">{error}</div>}
        {!data && !error && <div className="muted">loading…</div>}
        {data != null && name === "ports" && <Ports panel={data as PortsPanel} art={art} />}
        {data != null && name === "labs" && (
          <Labs panel={data as LabsPanel} art={art} labels={labels} />
        )}
        {data != null && name === "areas" && (
          <Areas panel={data as AreasPanel} labels={labels} colors={colors} />
        )}
        {data != null && name === "log" && <Log panel={data as LogPanel} />}
      </div>
    </div>
  );
}

function Ports({ panel, art }: { panel: PortsPanel; art: ArtManifest | null }) {
  // The zone gates are FOLDED FROM THE SLOTS, never typed as prose.
  //
  // `min_power` is on the wire and the server derives it from the engine's own
  // `port_accessible_zones`, so a re-pricing moves the per-slot chip below with
  // it. A sentence that said "LOW needs power 2, MID 3, HIGH 4" would not move,
  // and the lede would then contradict the chip printed directly under it. The
  // CLI composes the identical sentence the same way, for the same reason.
  const gates = new Map<string, number>();
  for (const b of panel.boards) {
    for (const s of b.slots) {
      const seen = gates.get(s.zone);
      if (seen === undefined || s.min_power < seen) gates.set(s.zone, s.min_power);
    }
  }
  const words = [...gates.entries()]
    .sort((a, b) => a[1] - b[1])
    .map(([zone, power]) => `${zone.toUpperCase()} needs power ${power}`)
    .join(", ");
  return (
    <>
      <p className="lede">
        Take ONE card{words ? `. ${words}` : ""} &mdash; a card in a zone you cannot reach is not
        an offer to you at all. {panel.deck_remaining} cards left in the deck.
      </p>
      {panel.boards.map((b) => (
        <section key={String(b.board_b)}>
          <h3>Port {b.board_b ? "B" : "A"}</h3>
          <div className="card-strip">
            {b.slots.map((s) => (
              <div className="slot-cell" key={s.slot}>
                <div className="slot-head">
                  <span className={`chip zone-${s.zone.toLowerCase()}`}>{s.zone}</span>
                  <span className="chip">power {s.min_power}+</span>
                </div>
                {s.card == null ? (
                  <div className="card-blank empty">empty</div>
                ) : (
                  <OrderCardView art={art} card={s.printed} id={s.card} size="sm" />
                )}
              </div>
            ))}
          </div>
        </section>
      ))}
    </>
  );
}

function Labs({
  panel,
  art,
  labels,
}: {
  panel: LabsPanel;
  art: ArtManifest | null;
  labels: Labels | null;
}) {
  return (
    <>
      <p className="lede">
        Acquire ONE card by paying its industry; the slot&rsquo;s own surcharge is already in the
        figure. {panel.deck_remaining} cards left in the deck.
      </p>
      {panel.boards.map((b) => (
        <section key={String(b.board_b)}>
          <h3>Laboratory {b.board_b ? "B" : "A"}</h3>
          <div className="card-strip">
            {b.slots.map((s) => (
              <div className="slot-cell" key={s.slot}>
                <div className="slot-head">
                  <span className="chip">slot {s.slot + 1}</span>
                  {s.surcharge > 0 && <span className="chip warn">+{s.surcharge}</span>}
                </div>
                {s.card == null ? (
                  <div className="card-blank empty">empty</div>
                ) : (
                  <TechCardView
                    art={art}
                    card={s.printed}
                    labels={labels}
                    cost={s.printed ? s.printed.industry_cost + s.surcharge : undefined}
                  />
                )}
              </div>
            ))}
          </div>
        </section>
      ))}
    </>
  );
}

/** What one power rung pays, in words.
 *
 *  Read off the tagged value rather than switched on a string, so a new yield
 *  shape renders as its own JSON instead of silently as nothing. */
function yieldWords(y: ActionYield): string {
  if ("Produce" in y) return `${y.Produce.amount} ${y.Produce.good.toLowerCase()}`;
  if ("Money" in y) return `¥${y.Money.yen}`;
  if ("PortTake" in y)
    return `take from ${y.PortTake.zones.join("/")}${y.PortTake.vp ? ` · ★${y.PortTake.vp}` : ""}`;
  if ("Industry" in y) return `industry ${y.Industry.value}`;
  if ("Faith" in y)
    return `faith ${y.Faith.from_power}–${y.Faith.with_all_payment_kinds}`;
  if ("CustomsPayment" in y) return `pay up to ${y.CustomsPayment.max_imports} imports`;
  if ("WarehouseTakes" in y) return `${y.WarehouseTakes.items} from warehouse`;
  if ("Exchanges" in y) return `${y.Exchanges.count} exchange(s)`;
  return JSON.stringify(y);
}

function Areas({
  panel,
  labels,
  colors,
}: {
  panel: AreasPanel;
  labels: Labels | null;
  colors: PlayerColor[];
}) {
  return (
    <div className="area-grid">
      {panel.rows.flat().map((card: AreaCard) => (
        <section className="area-card" key={card.tile.area}>
          <h3>
            <span className="tile-id">A{String(card.tile.area).padStart(2, "0")}</span>{" "}
            {labels?.area_types[card.tile.area_type] ?? card.tile.area_type}
          </h3>
          <div className="tile-row">
            <Pawns
              presidents={card.tile.presidents}
              assistants={card.tile.assistants}
              hasStation={card.tile.has_station}
              colors={colors}
            />
          </div>
          {card.ladder.length === 0 ? (
            <div className="muted">never an action area</div>
          ) : (
            <table className="ladder">
              <tbody>
                {card.ladder.map((rung) => (
                  <tr key={rung.power}>
                    <td className="power">{rung.power}</td>
                    <td>{yieldWords(rung.pays)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          {/* A POINTER, never a duplicate: the offer on a Port board and the
              occupied spaces of a track are surfaces of their own, and a second
              copy here would be a second copy to keep true. */}
          {card.companion && (
            <div className="muted">detail lives on the {card.companion.toLowerCase()} surface</div>
          )}
        </section>
      ))}
    </div>
  );
}

function Log({ panel }: { panel: LogPanel }) {
  const omitted = panel.total - panel.entries.length;
  return (
    <>
      <p className="lede">
        {panel.total} event(s){omitted > 0 ? `, ${omitted} earlier not shown` : ""}.
      </p>
      <ol className="log">
        {panel.entries.map((e) => (
          <li key={e.index}>
            <span className="num">{e.index}</span>
            <span>{e.text}</span>
          </li>
        ))}
      </ol>
    </>
  );
}
