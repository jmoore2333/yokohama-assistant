// The read-only surfaces: what is on every area, and what is in the log.
//
// These give the CLI's `areas`/`log` words a home in a browser. They are
// fetched on demand rather than folded into every frame: neither changes
// between decisions in a way a player is watching for, and a payload that
// carried them would pay for them on every move.
//
// **The port offer, the laboratory offer, the tracks and the building sites
// used to be drawers here and are not any more (M5 W5a).** They are boards, and
// a board belongs on the table rather than behind a button: `Strips.tsx` draws
// them in the physical boards' own shapes and `TableSections` mounts them. What
// is left in this file is what has no board — a list of areas, a log, two
// explanations, four card indexes, a final score and the help.

import { useEffect, useRef, useState } from "react";
import { api, ApiError } from "./api";
import { Help } from "./Help";
import type {
  ActionYield,
  AreaCard,
  AreasPanel,
  ArtManifest,
  Labels,
  LogPanel,
  PlayerColor,
  IndexModel,
  Outcome,
  SuggestResponse,
  WhyResponse,
} from "./types";
import { fivePowerWords } from "./art";
import {
  Pawns,
  seatClass,
  SPACE_POSITIONS,
  spaceWords,
} from "./Cards";
import { SuggestView, WhyView } from "./Explain";
import { IndexView, OutcomeView } from "./Index";

export type SurfaceName =
  | "areas"
  | "log"
  | "suggest"
  | "why"
  | "orders"
  | "techs"
  | "goals"
  | "sites"
  | "result"
  | "help";

/** Each surface carries its OWN loader.
 *
 *  It used to be indexed dispatch — `api[name](game)` — which works exactly as
 *  long as every surface is `(game) => Promise<T>` and no longer. M3's `why`
 *  takes an optional event index, so the registry carries a thunk instead. One
 *  panel shell for every surface rather than a second one beside it: the two
 *  explanation surfaces are the same thing as the read-only ones from the
 *  panel's point of view (fetch, draw, close on Escape), and a parallel
 *  component would mean two dialects of the same dialog. */
/** A surface whose data the client already holds. `Surface` renders on
 *  `data != null`, so a loader still has to resolve SOMETHING. */
const NOTHING_TO_FETCH = Symbol("already on the frame");

export const SURFACES: {
  key: SurfaceName;
  label: string;
  blurb: string;
  load: (game: string) => Promise<unknown>;
}[] = [
  { key: "areas", label: "Areas", blurb: "what every area's action pays at power 1–5", load: (g) => api.areas(g) },
  { key: "log", label: "Log", blurb: "what has happened, each line worded as it read when it applied", load: (g) => api.log(g) },
  { key: "suggest", label: "Suggest", blurb: "what the bot would play on your seat, and the arithmetic behind it", load: (g) => api.suggest(g) },
  { key: "why", label: "Why", blurb: "the account of the last move a bot played, reconstructed by replay", load: (g) => api.why(g) },
  { key: "orders", label: "Orders", blurb: "all 36 order cards, grouped by country and labelled by printed corner index, and where each one is", load: (g) => api.index(g, "orders") },
  { key: "techs", label: "Technologies", blurb: "all 20 technology titles, grouped by country, with the gear each costs and where every copy is", load: (g) => api.index(g, "techs") },
  { key: "goals", label: "Goals", blurb: "the achievements in play, what each pays, and how close every seat is", load: (g) => api.index(g, "goals") },
  { key: "sites", label: "Site cards", blurb: "the 24 printed building sites, what each space pays, and which are already on the board", load: (g) => api.index(g, "sites") },
  { key: "result", label: "Final score", blurb: "the finished game, ranked, with every category the engine scored", load: (g) => api.result(g) },
  // Nothing to fetch — the sentinel's one remaining user, since the tracks
  // became a section of the table. Help is static prose and the screenshots are
  // ordinary images the client asks for itself, so a route for it would be a
  // route that serves a constant.
  { key: "help", label: "Help", blurb: "how this screen works — the board, the deck, the panels, and the replay track", load: () => Promise.resolve(NOTHING_TO_FETCH) },
];

/** The four surfaces that are card INDEXES, and the kind each asks the server
 *  for. Kept beside the registry rather than inferred from the key, so a new
 *  surface cannot become an index by being named like one. */
export const INDEX_KINDS: Partial<Record<SurfaceName, string>> = {
  orders: "orders",
  techs: "techs",
  goals: "goals",
  sites: "sites",
};

/** The surfaces that live behind the banner's `Cards` disclosure.
 *
 *  **`INDEX_KINDS` alone is the wrong list, and this is the correction.** The
 *  review proposed deriving the group from it "rather than writing a third
 *  list", which sounds right and ships a four-item disclosure: `result` is NOT
 *  an index — it renders through its own `name === "result"` arm — so Final
 *  score would have stayed inline, which is not what the grouping is for.
 *  Derived from `INDEX_KINDS` PLUS the one named exception, so a new index still
 *  joins automatically and the exception is visible rather than implied.
 *
 *  The nav is grouped so the banner stays ONE ROW: eleven buttons wrapped it to
 *  a second row below 1280, and measured, grouping takes it from 93 to 56px at
 *  1440 and from 174 to 93 at 1024, with the nav's bottom edge at 43px rather
 *  than 88/88/125.
 *
 *  **Not because it lets you switch surfaces with a panel open** — that was the
 *  review's argument and `showModal()` superseded it, since a modal dialog makes
 *  the whole page outside it inert on purpose.
 *
 *  **900px is NOT fixed and no claim is made about it.** The review's
 *  "un-buries the nav at 1280, 1201, 1024 and 900" is false at the last one:
 *  measured, the banner still wraps there and the nav's bottom edge is 88px. */
export const CARD_SURFACES: SurfaceName[] = [
  ...(Object.keys(INDEX_KINDS) as SurfaceName[]),
  "result",
];

export function Surface({
  name,
  game,
  art,
  labels,
  colors,
  scrubK,
  onScrub,
  onClose,
}: {
  name: SurfaceName;
  game: string;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  /** The replay position, or `null` for the present.
   *
   *  **Props, not a widened loader.** The registry's `load` is
   *  `(game: string) => Promise<unknown>` and cannot carry a second argument;
   *  M3.5 met the same wall for the tracks panel and answered it with a
   *  sentinel loader plus a prop rather than by widening the contract for one
   *  caller (that surface has since become a section of the table, and this is
   *  the pattern it left behind). A scrub index is the same shape, and the
   *  payload it selects is fetched by `App`, which is what has to draw the
   *  board it names. */
  scrubK?: number | null;
  onScrub?: (k: number | null) => void;
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
    const entry = SURFACES.find((s) => s.key === name);
    if (!entry) return;
    entry
      .load(game)
      .then((d) => live && setLoaded({ of: name, data: d }))
      .catch((e) => live && setError(e instanceof ApiError ? e.message : String(e)));
    return () => {
      live = false;
    };
  }, [name, game]);

  const data = loaded?.of === name ? loaded.data : null;

  // **A real `<dialog>`, opened modally — because the old panel only SAID it
  // was modal.**
  //
  // It carried `role="dialog" aria-modal="true"` over a `<div>` and moved focus
  // into itself, and that was all. Measured in real Chromium: six Tab presses
  // from the open panel land on a nav `button.ghost` outside it, and the
  // accessibility tree still reports the background controls as
  // `ignored=false`. An assertion of containment that the DOM does not have is
  // worse than no assertion, because a screen reader believes it.
  //
  // The fix proposed in review was `inert` on `.stage`, and it does not work:
  // `nav.views` and all three `.decision` blocks are SIBLINGS of `.stage`, not
  // descendants, so setting it removes the board tiles and the legend and
  // nothing else — measured, the tab walk then escapes at press 2 instead of
  // press 6, strictly worse. `showModal()` is what closes both channels at once,
  // and it is the browser's job rather than this client's: focus is contained,
  // Escape is handled natively, and the background leaves the accessibility
  // tree. The backdrop is styled nearly clear (`styles.css`) so the board stays
  // READABLE beside the panel, which is the whole point of the drawer — it is
  // not clickable while a panel is open, and that is what "modal" means.
  const panel = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const el = panel.current;
    if (!el) return;
    // `showModal` on an already-open dialog throws, and this effect re-runs
    // whenever the surface changes.
    //
    // The `typeof` guard is for JSDOM, which renders `<dialog>` but implements
    // neither `showModal` nor the top layer. Stated rather than left implicit,
    // because it means the component suite CANNOT check the modality — it can
    // only check that the element is a `<dialog>`, which is what
    // `Surfaces.test.tsx` asserts. The containment itself is checked in the
    // browser walk, by tabbing.
    if (typeof el.showModal === "function") {
      if (!el.open) el.showModal();
    } else {
      // JSDOM. A `<dialog>` without `open` is `display: none`, so without this
      // every existing component test would query an empty body and fail for a
      // reason that has nothing to do with what it is testing. This is a
      // non-modal open: the content renders, and the modality is exactly the
      // part the component suite was never able to check anyway.
      el.open = true;
    }
    el.focus();
  }, [name]);

  // **The dependency is `[name]`, and neither `[onClose]` nor `[]` is right.**
  //
  // `[onClose]` was the bug: `App` passes a fresh arrow on every render, so any
  // state change re-ran the effect and dragged focus back into the panel
  // mid-interaction — reproduced by blurring to `<body>` and firing a
  // programmatic click that moves no focus of its own.
  //
  // `[]` is the tempting correction and is also wrong: `<Surface>` is NOT
  // remounted when you switch panels — same component, new `name` — so the
  // effect would never re-run and switching from Areas to Log would leave a
  // keyboard user behind on the nav button. `[name]` is the one that is true of
  // both cases, and `Surfaces.test.tsx` holds each half.
  useEffect(() => {
    const el = panel.current;
    if (!el) return;
    // **Escape is caught on the ELEMENT, and the native close is suppressed.**
    //
    // A `<dialog>` closes itself on Escape and raises `cancel`, which would take
    // the panel off screen while `App` still believed a surface was open — the
    // nav button would stay lit and the next press would do nothing. Preventing
    // the default on the keydown stops that AND stops `cancel` from firing, so
    // exactly one path runs and React stays the owner of the open state.
    //
    // The listener is on the dialog rather than on `window` because focus is
    // inside it (that is what `showModal` guarantees), and a window listener
    // would also fire for a second surface's keystrokes. `cancel` is still
    // handled for the routes that do not pass through a keydown at all.
    const close = (e: Event) => {
      e.preventDefault();
      onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close(e);
    };
    el.addEventListener("keydown", onKey);
    el.addEventListener("cancel", close);
    return () => {
      el.removeEventListener("keydown", onKey);
      el.removeEventListener("cancel", close);
    };
  }, [onClose]);

  const meta = SURFACES.find((s) => s.key === name);
  return (
    <dialog className="surface" aria-label={meta?.label} ref={panel} tabIndex={-1}>
      {/* Not a landmark: a bare <header> here is a second `banner` for the
          page, and a screen reader that offers two is offering a wrong one. */}
      <header role="presentation">
        <h2>{meta?.label}</h2>
        <span className="meta">{meta?.blurb}</span>
        <button type="button" className="ghost" onClick={onClose}>
          Close
        </button>
      </header>
      <div className="surface-body">
        {error && <div className="error">{error}</div>}
        {!data && !error && <div className="muted">loading…</div>}
        {data != null && name === "areas" && (
          <Areas panel={data as AreasPanel} labels={labels} colors={colors} />
        )}
        {data != null && name === "help" && <Help />}
        {data != null && name === "log" && (
          <Log panel={data as LogPanel} k={scrubK ?? null} onScrub={onScrub} />
        )}
        {data != null && name === "suggest" && (
          <SuggestView data={data as SuggestResponse} labels={labels} />
        )}
        {data != null && name === "why" && <WhyView data={data as WhyResponse} labels={labels} />}
        {data != null && INDEX_KINDS[name] && (
          <IndexView
            first={data as IndexModel}
            kind={INDEX_KINDS[name]!}
            game={game}
            art={art}
            labels={labels}
            colors={colors}
          />
        )}
        {data != null && name === "result" && (
          <OutcomeView outcome={data as Outcome} colors={colors} />
        )}
      </div>
    </dialog>
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
  // **The rows are ROWS, and flattening them threw away the board.**
  // The payload carries `[4,3,2,1]` at 2P and `[6,5,4,3]` at 4P — the staggered
  // pyramid's own shape — and `.flat()` handed the grid one long list, so the
  // panel could not tell you which areas sit beside each other. Grouping is what
  // makes this readable as a board rather than as an alphabetical index.
  return (
    <div className="area-rows">
      {panel.rows.map((row, r) => (
        <div className="area-row" key={r}>
          <div className="label">row {r + 1}</div>
          <div className="area-grid">
      {row.map((card: AreaCard) => (
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
          {/* The 5-Power token, which the CLI names on this surface and this
              panel did not carry at all. Three states, the CLI's own three
              (`view_render.rs:1078-1094`): the face, "already taken", and the
              Canal, which never had one. */}
          {card.ladder.length > 0 && (
            <div className="tile-row token-row">
              {card.tile.token ? (
                <span className="five-power">5-Power: {fivePowerWords(card.tile.token)}</span>
              ) : (
                <span className="muted">5-Power already taken</span>
              )}
            </div>
          )}

          {/* What the building site pays, and who has built on it. The words
              come from the same `spaceWords` the site index uses, so the two
              surfaces cannot describe one printed space differently. */}
          {card.tile.site && (
            <div className="area-site">
              <div className="label">
                Site {card.tile.site.index} &middot; trading house &#9733;
                {card.tile.site.trading_house_vp}
                {card.tile.site.trading_house_owner != null && (
                  <span className={`occ ${seatClass(colors[card.tile.site.trading_house_owner])}`}>
                    {" "}
                    built by {colors[card.tile.site.trading_house_owner]}
                  </span>
                )}
              </div>
              <ul className="site-words">
                {card.tile.site.slots.map((slot, i) => (
                  <li key={i}>
                    <span className="muted">{SPACE_POSITIONS[i]}</span> {spaceWords(slot.reward)}
                    {slot.built_by != null && (
                      <span className={`occ ${seatClass(colors[slot.built_by])}`}>
                        {" "}
                        &#8962; {colors[slot.built_by]}
                      </span>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* The tolled Canal crossings. The Canal joins ANY of its neighbours
              to any other, and a pair NOT already joined by a free brick-offset
              edge costs 1 yen to the BANK. Declared on the wire since M2
              (`types.ts:99`) and rendered nowhere until now. */}
          {card.tile.canal_crossings.length > 0 && (
            <div className="muted">
              canal crossing to{" "}
              {card.tile.canal_crossings
                .map((a) => `A${String(a).padStart(2, "0")}`)
                .join(", ")}{" "}
              &mdash; 1 yen each, to the bank
            </div>
          )}

          {/* A POINTER, never a duplicate: the offer on a Port board and the
              occupied spaces of a track are surfaces of their own, and a second
              copy here would be a second copy to keep true. */}
          {card.companion && (
            <div className="muted">detail lives in the {card.companion} section of the table</div>
          )}
        </section>
      ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function Log({
  panel,
  k,
  onScrub,
}: {
  panel: LogPanel;
  k: number | null;
  onScrub?: (k: number | null) => void;
}) {
  const omitted = panel.total - panel.entries.length;
  // **`total + 1` stops, not `total`.** A replay position counts events
  // APPLIED, so stop `n` is the position event `n` was decided in and stop
  // `total` is the present — a real position with no event to name. A track
  // that ran `0..total` and called it the whole game would be off by one at the
  // end.
  const at = k ?? panel.total;
  const move = (to: number) => onScrub?.(Math.max(0, Math.min(panel.total, to)));
  return (
    <>
      <p className="lede">
        {panel.total} event(s){omitted > 0 ? `, ${omitted} earlier not shown` : ""}.
      </p>

      {onScrub && panel.total > 0 && (
        <div className="scrub">
          <button
            type="button"
            className="ghost"
            aria-label="Previous move"
            onClick={() => move(at - 1)}
            disabled={at <= 0}
          >
            &#9664;
          </button>
          <input
            type="range"
            min={0}
            max={panel.total}
            value={at}
            aria-label="Replay position"
            aria-valuetext={at === panel.total ? "now" : `after ${at} of ${panel.total}`}
            onChange={(e) => move(Number(e.currentTarget.value))}
          />
          <button
            type="button"
            className="ghost"
            aria-label="Next move"
            onClick={() => move(at + 1)}
            disabled={at >= panel.total}
          >
            &#9654;
          </button>
          <span className="scrub-at">
            {at === panel.total ? "now" : `after ${at} of ${panel.total}`}
          </span>
          {/* Not at `total`: that stop IS the present, and this row labels it
              "now" three lines above. Offering "Back to live" there asked the
              reader to go back to where they already were. */}
          {k !== null && k < panel.total && (
            <button type="button" className="ghost" onClick={() => onScrub(null)}>
              Back to live
            </button>
          )}
        </div>
      )}

      <ol className="log">
        {panel.entries.map((e) => (
          <li key={e.index} className={k !== null && e.index === k ? "at" : ""}>
            <span className="num">{e.index}</span>
            <button
              type="button"
              className="log-jump"
              onClick={() => move(e.index)}
              disabled={!onScrub}
            >
              {e.text}
            </button>
          </li>
        ))}
      </ol>
    </>
  );
}
