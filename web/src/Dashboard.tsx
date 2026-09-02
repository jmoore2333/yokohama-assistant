import type {
  AchievementModel,
  ArtManifest,
  ChurchSpaceModel,
  CustomsSpaceModel,
  HandPanel,
  Labels,
  MercenarySupply,
  Panel,
  PlayerColor,
  PlayerModel,
  PlayerId,
  TracksPanel,
} from "./types";
import { DUMMY_SEAT, locked } from "./types";
import { mercenaryArt } from "./art";
import { AchievementCardView, OrderCardView, seatClass } from "./Cards";

/** The key of ONE member of a union, distributed.
 *
 *  The naked type parameter is load-bearing and is why this is a helper rather
 *  than written inline: a conditional distributes over a union only when the
 *  checked type is a bare type PARAMETER, so `Panel extends infer P ? keyof P
 *  : never` does not distribute at all — it binds `P` to the whole union, and
 *  `keyof` of a union is the INTERSECTION of its members' keys, which for four
 *  disjoint single-key objects is `never`. Measured: written that way, this
 *  compiled, `PanelName` became `never`, and every `only={[…]}` in the repo
 *  failed with "Type 'string' is not assignable to type 'never'". */
type NameOf<P> = P extends unknown ? keyof P : never;

/** The name of a panel this file knows how to draw.
 *
 *  DERIVED from the `Panel` union rather than spelled again: `Panel` mirrors
 *  `screen_model::Panel`, so a variant renamed on the server renames this, and
 *  every `only={[…]}` naming the old word stops compiling. A hand-written union
 *  would have gone on type-checking while the panel it named no longer existed. */
export type PanelName = NameOf<Panel>;

/** Every name `Dashboard` has an arm for, as a value.
 *
 *  It exists so `only` can be an ALLOWLIST of what to draw without also being a
 *  denylist of what this client has never heard of — see `wanted`. Typed as
 *  `PanelName[]`, so it moves with the wire for the same reason. */
const KNOWN: PanelName[] = ["Players", "Tracks", "Achievements", "Hand"];

export function Dashboard({
  panels,
  art,
  labels,
  colors,
  only,
}: {
  panels: Panel[];
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  /** Which panels to draw; absent means every one. The side column asks for
   *  Players and Hand (M5 §3.5); Tracks and Goals live on the table. */
  only?: PanelName[];
}) {
  // **A panel this file cannot draw is never filtered out.**
  //
  // `only` names what to DRAW; it must not double as a list of what may exist.
  // A fifth `screen_model::Panel` variant — or a rename of one of the four —
  // matches no name here, and a filter that dropped it would make the panel
  // VANISH with no message and no failing test: exactly what the raw-JSON
  // fallback below exists to prevent, defeated on the app's one live render,
  // which is the only one that passes `only`. So the unknown falls through to
  // that fallback and shows itself.
  const wanted = (p: Panel) =>
    !only || only.some((name) => name in p) || !KNOWN.some((name) => name in p);
  return (
    <>
      {panels.filter(wanted).map((panel, i) => {
        if ("Players" in panel)
          return <Players key={i} players={panel.Players.players} labels={labels} />;
        if ("Tracks" in panel)
          return <Tracks key={i} tracks={panel.Tracks} art={art} colors={colors} />;
        if ("Achievements" in panel)
          return (
            <Achievements
              key={i}
              cards={panel.Achievements.cards}
              art={art}
              labels={labels}
              colors={colors}
            />
          );
        if ("Hand" in panel) return <Hand key={i} hand={panel.Hand} art={art} colors={colors} />;
        // **A panel this file does not know renders as itself, not as nothing.**
        //
        // `return null` meant a fifth `screen_model::Panel` variant — or a rename
        // of one of the four — made a whole panel VANISH from the rail with no
        // message, no console output and no failing test: the money, the goods
        // or a track would simply stop being on screen. The pattern is
        // `Surfaces.tsx`'s `yieldWords`, which ends `return JSON.stringify(y)`
        // for the same reason and says so — a new shape should render as its own
        // JSON rather than silently as nothing.
        return (
          <section className="panel" key={i}>
            <h2>Unknown panel</h2>
            <p className="muted">
              This client does not know how to draw this panel yet. Its contents are below so
              nothing is lost.
            </p>
            <pre className="raw-panel">{JSON.stringify(panel, null, 1)}</pre>
          </section>
        );
      })}
    </>
  );
}

/** All five goods, ALWAYS, zeros included and dimmed.
 *
 *  `panel_render::goods_line` prints every one — the CLI's own note says a
 *  suppressed figure hides the only distinction that scores. A browser that
 *  drops `0 imports` cannot be read as "no imports" rather than "imports not
 *  shown on this panel", and imports are what the Customs track and one of the
 *  church's six faith payment types are counted in. */
const good = (n: number, cls: string, label: string) => (
  <span className={`chip ${cls} ${n === 0 ? "zero" : ""}`} key={cls}>
    {n} {label}
  </span>
);

/** Where a seat's president is: in hand, or on a named area.
 *
 *  Not an `Option` on the wire — a president is always somewhere — so this
 *  renders both arms and never a blank. */
function President({ p }: { p: PlayerModel }) {
  if (p.president === "InHand") return <>President in hand</>;
  return <>President at A{String(p.president.OnBoard).padStart(2, "0")}</>;
}

function Players({ players, labels }: { players: PlayerModel[]; labels: Labels | null }) {
  return (
    <section className="panel">
      <h2>Players</h2>
      {players.map((p) => {
        const lock = locked(p.assistants);
        return (
          <div className={`seat ${p.is_current ? "current" : ""}`} key={p.seat}>
            <div className="seat-head">
              <span className={`seat-name seat-${p.color.toLowerCase()}`}>{p.color}</span>
              {p.is_bot && <span className="tag">bot</span>}
              <span className="tag">seat {p.seat}</span>
              <span className="chip yen">&#165;{p.money}</span>
              <span className="chip vp">{p.victory_points} VP</span>
            </div>

            <div className="stat-row">
              {good(p.goods.copper, "copper", "copper")}
              {good(p.goods.silk, "silk", "silk")}
              {good(p.goods.tea, "tea", "tea")}
              {good(p.goods.seafood, "seafood", "seafood")}
              {good(p.goods.imports, "imports", "imports")}
            </div>

            <div className="stat-row">
              <span className="chip" title="in hand / on board / in warehouse">
                asst {p.assistants.in_hand}h / {p.assistants.on_board}b /{" "}
                {p.assistants.in_warehouse}w
              </span>
              {/* Locked assistants are OFF the board and unrecoverable — on an
                  achievement, the church or the customs track. A count that
                  folded them into "on board" would suggest they can come home. */}
              {lock > 0 && (
                <span
                  className="chip locked"
                  title={`${p.assistants.on_achievements} on achievements · ${p.assistants.on_church} on the church · ${p.assistants.on_customs} on customs`}
                >
                  {lock} locked
                </span>
              )}
              {/* All three zones, because IN HAND is the one you can build
                  from. `shop 2/8` cannot distinguish two in hand from none, and
                  a player reading it plans a build they cannot make; the CLI
                  prints in-hand first for exactly that reason. The `/8` and
                  `/4` are also engine end-game constants, and this layer does
                  not get to know them — the end-trigger strip already carries
                  them, worded by the engine's own renderer. */}
              <span className="chip" title="in hand / in warehouse / built">
                shop {p.shophouses.in_hand}h / {p.shophouses.in_warehouse}w /{" "}
                {p.shophouses.built}b
              </span>
              <span className="chip" title="in hand / in warehouse / built">
                trade {p.trading_houses.in_hand}h / {p.trading_houses.in_warehouse}w /{" "}
                {p.trading_houses.built}b (next &#165;{p.trading_houses.next_take_cost})
              </span>
            </div>

            <div className="stat-row">
              <span className="chip">
                <President p={p} />
              </span>
              <span className="chip">{p.orders_in_hand} order(s) in hand</span>
              {p.five_power_tokens > 0 && (
                <span className="chip yen">{p.five_power_tokens} 5-Power</span>
              )}
              {/* Always, both numbers: unused mercenaries score 1 VP each and
                  used ones score nothing, so the split IS the only thing about
                  them that scores. */}
              <span className="chip" title="face-up mercenaries score 1 VP each; used ones do not">
                merc {p.mercenaries_unused} unused / {p.mercenaries_used} used
              </span>
            </div>

            {/* Orders filled are public on every seat, in completion order —
                the one thing about a hidden hand that is not hidden. */}
            {p.orders_filled.length > 0 && (
              <div className="stat-row">
                <span className="label">filled</span>
                {p.orders_filled.map((id, i) => (
                  <span className="chip order" key={`${id}-${i}`}>
                    O{String(id).padStart(2, "0")}
                  </span>
                ))}
              </div>
            )}

            {/* A player may own only one card of a given NAME, which is why the
                name is the identity worth showing rather than the id. */}
            <div className="stat-row">
              <span className="label">tech</span>
              {p.technologies.length === 0 ? (
                <span className="chip muted">none yet</span>
              ) : (
                p.technologies.map((t) => (
                  <span
                    className="chip tech"
                    key={t.id}
                    title={
                      t.printed
                        ? (labels?.tech_effects[t.printed.effect] ?? t.printed.effect)
                        : undefined
                    }
                  >
                    {t.printed?.name ?? `#${t.id}`}
                  </span>
                ))
              )}
            </div>
          </div>
        );
      })}
    </section>
  );
}

/** One track space, coloured by whoever stands on it.
 *
 *  `occupant === 255` is the 2P neutral DUMMY, which competes in the majority
 *  bonus and is excluded from the end-game pawn count. It is marked `D` and
 *  never coloured as a seat — indexing a colour array with 255 is how it would
 *  silently become one. */
function Space({
  occupant,
  colors,
  children,
  sub,
  title,
}: {
  occupant: PlayerId | null;
  colors: PlayerColor[];
  children: React.ReactNode;
  /** What the space PAYS, printed under it. It used to live only in `title=`,
   *  which is invisible on a touch screen, invisible to a screen reader and
   *  invisible to any assertion — while the CLI prints it. */
  sub: string;
  title: string;
}) {
  const dummy = occupant === DUMMY_SEAT;
  const cls = ["space", occupant != null ? "taken" : "", dummy ? "dummy" : seatClass(colors[occupant ?? -1])]
    .filter(Boolean)
    .join(" ");
  return (
    <span className={cls} title={title}>
      {children}
      <span className="sub">{sub}</span>
      {dummy && <span className="occ">D</span>}
      {occupant != null && !dummy && <span className="occ">&#9679;</span>}
    </span>
  );
}

/** A track space's rider in the two lengths its two homes have room for. */
export interface RiderWords {
  /** For the space itself: compact, and it keeps the count. `""` when the space
   *  carries no rider — a rider-free space must print nothing, not a dash. */
  short: string;
  /** For the `title`, in the terminal's own longer words. */
  long: string;
}

/** What a track space's RIDER pays, in TWO lengths from ONE source.
 *
 *  **M5 spec §3.4 asks for the VP and the rider printed on every space, and the
 *  rider was the half that never was.** It lived only in `title=` — invisible
 *  on a touch screen, invisible to a screen reader, invisible to any assertion
 *  — while the terminal's own `tracks` view prints it in words
 *  (`view_render.rs`: `+N assistant moves`, `+N warehouse take(s)`, `+N yen`).
 *
 *  Two lengths, because the two places have different room: the space itself is
 *  a ~2rem box at 8.5px, so it takes the compact form, and the tooltip takes the
 *  terminal's. They are returned TOGETHER rather than composed at each call
 *  site — two independently written strings for one fact drift, which is how
 *  the rider came to be in the tooltip alone in the first place.
 *
 *  **The compact form keeps the COUNT.** A bare `+mv` would fit in less room and
 *  would throw away the difference between faith 4 (one move) and faith 11
 *  (three) — the very figure the terminal prints and this is here to restore.
 *  Mutation-tested: dropping the count is invisible to every cell that reads
 *  this function on both sides, so `Dashboard.test.tsx` carries a cell that
 *  pins distinct counts to distinct words. The strip's HEADING keeps its coarse
 *  `+mv` flag — that one is a "this track has riders" summary, and it is not
 *  the per-space reading. */
export function riderWords(space: ChurchSpaceModel | CustomsSpaceModel): RiderWords {
  // The church space is the one with a faith number; the customs space has a
  // space number and two riders of its own.
  if ("faith" in space) {
    const n = space.movement_rider;
    return n > 0 ? { short: `+${n}mv`, long: `+${n} assistant moves` } : { short: "", long: "" };
  }
  const short: string[] = [];
  const long: string[] = [];
  // Warehouse first, then yen: the order the engine's own `CustomsTrack::rider`
  // returns them, and the order the terminal prints them.
  if (space.warehouse_rider > 0) {
    short.push(`+${space.warehouse_rider}wh`);
    long.push(`+${space.warehouse_rider} warehouse take${space.warehouse_rider === 1 ? "" : "s"}`);
  }
  if (space.yen_rider > 0) {
    short.push(`+¥${space.yen_rider}`);
    long.push(`+${space.yen_rider} yen`);
  }
  return { short: short.join(" "), long: long.join(" ") };
}

/** `★3–14` — what the cheapest and dearest space on a track pay.
 *
 *  The CLI's `panel_render::vp_range`, restated: an empty track is `★—`, a
 *  uniform one is a single figure, and only a real spread prints a dash. */
function vpRange(vps: number[]): string {
  if (vps.length === 0) return "★—";
  const lo = Math.min(...vps);
  const hi = Math.max(...vps);
  return lo === hi ? `★${lo}` : `★${lo}–${hi}`;
}

/** The five mercenary countries in the engine's own `Country::ALL` order, with
 *  the supply field each maps to. Spelled as pairs rather than iterated over
 *  the object so the ORDER is the engine's and not the JSON's. */
const MERCENARIES: [string, keyof MercenarySupply, string][] = [
  ["English", "english", "Britain"],
  ["American", "american", "America"],
  ["French", "french", "France"],
  ["German", "german", "Germany"],
  ["Dutch", "dutch", "Netherlands"],
];

export function Tracks({
  tracks,
  art,
  colors,
}: {
  tracks: TracksPanel;
  art: ArtManifest | null;
  colors: PlayerColor[];
}) {
  const hasDummy =
    tracks.church.some((s) => s.occupant === DUMMY_SEAT) ||
    tracks.customs.some((s) => s.occupant === DUMMY_SEAT);
  const m = tracks.mercenary_supply;
  const mercTotal = m.english + m.american + m.french + m.german + m.dutch;
  // The summary figure the CLI prints beside each track: what the CHEAPEST and
  // DEAREST space on it pay. A range, not the top — the top alone would hide
  // that the church starts at 3 and the customs at 4, which is half of what a
  // player is weighing. Computed from the spaces the model carries, exactly as
  // `panel_render::vp_range` does, so a rules correction to a printed reward
  // moves both surfaces together.
  const churchMv = tracks.church.some((s) => s.movement_rider > 0);
  return (
    <section className="panel">
      {/* The dummy clause only when a dummy is actually on a track:
          `place_setup_dummies` returns early above 2 players, so at 3P or 4P
          the CLI prints no such clause and a browser that teaches a `D` marker
          which can never appear is teaching a rule that does not apply. */}
      <h2>
        Tracks &mdash; ends at {tracks.trigger_at} of YOUR pawns
        {hasDummy ? " · D = dummy, not counted" : ""}
      </h2>

      <div className="tile-row muted">
        Church faith &middot; {tracks.church_filled}/{tracks.trigger_at}
        <span className="chip vp">
          {vpRange(tracks.church.map((s) => s.vp))}
          {churchMv ? " +mv" : ""}
        </span>
      </div>
      <div className="track">
        {tracks.church.map((s) => {
          const rider = riderWords(s);
          return (
            <Space
              key={s.faith}
              occupant={s.occupant}
              colors={colors}
              sub={`★${s.vp}${rider.short ? ` · ${rider.short}` : ""}`}
              title={`faith ${s.faith} — ★${s.vp}${rider.long ? ` ${rider.long}` : ""}`}
            >
              {s.faith}
            </Space>
          );
        })}
      </div>

      <div className="tile-row muted" style={{ marginTop: "0.5rem" }}>
        Customs &middot; {tracks.customs_filled}/{tracks.trigger_at}
        <span className="chip vp">{vpRange(tracks.customs.map((s) => s.vp))}</span>
      </div>
      <div className="track">
        {tracks.customs.map((s) => {
          const rider = riderWords(s);
          return (
            <Space
              key={s.space}
              occupant={s.occupant}
              colors={colors}
              sub={`★${s.vp}${rider.short ? ` · ${rider.short}` : ""}`}
              title={`pay ${s.import_cost} imports — ★${s.vp}${rider.long ? ` ${rider.long}` : ""}`}
            >
              {s.import_cost}
            </Space>
          );
        })}
      </div>

      <div className="stat-row" style={{ marginTop: "0.5rem" }}>
        <span className="chip">Station {tracks.station == null ? "unplaced" : `at A${String(tracks.station).padStart(2, "0")}`}</span>
        <span className="chip">mercenary supply {mercTotal}</span>
      </div>
      {/* Per country, because a mercenary is taken BY country and the supply
          runs out one flag at a time — a single total cannot say which one. */}
      <div className="merc-strip">
        {MERCENARIES.map(([key, field, word]) => {
          const src = mercenaryArt(art, key);
          const left = m[field];
          return (
            <span className={`merc ${left === 0 ? "gone" : ""}`} key={key} title={`${word}: ${left} left`}>
              {src ? <img src={src} alt="" loading="lazy" /> : <span className="chip">{word}</span>}
              <span className="n">{left}</span>
            </span>
          );
        })}
      </div>
    </section>
  );
}

export function Achievements({
  cards,
  art,
  labels,
  colors,
}: {
  cards: AchievementModel[];
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
}) {
  return (
    <section className="panel">
      <h2>Achievements &mdash; one per category, first / later VP</h2>
      <div className="card-strip">
        {cards.map((c) => (
          <AchievementCardView
            key={c.achievement_type}
            model={c}
            art={art}
            labels={labels}
            colors={colors}
          />
        ))}
      </div>
    </section>
  );
}

function Hand({
  hand,
  art,
  colors,
}: {
  hand: HandPanel;
  art: ArtManifest | null;
  colors: PlayerColor[];
}) {
  return (
    <section className="panel">
      <h2>
        <span className={seatClass(colors[hand.seat])}>{colors[hand.seat] ?? `Seat ${hand.seat}`}</span>
        &rsquo;s hand
      </h2>
      {hand.hidden ? (
        // `cards` is empty in the MODEL when hidden, so the count is all there
        // is — and it is truthful, which is what the table shows too.
        <div className="tile-row muted">{hand.card_count} card(s) &mdash; hidden</div>
      ) : hand.cards.length === 0 ? (
        <div className="tile-row muted">empty</div>
      ) : (
        <div className="card-strip">
          {hand.cards.map((c) => (
            <OrderCardView
              key={c.id}
              art={art}
              card={c.printed}
              id={c.id}
              note={c.is_starting_order ? "your starting order" : undefined}
            />
          ))}
        </div>
      )}
    </section>
  );
}
