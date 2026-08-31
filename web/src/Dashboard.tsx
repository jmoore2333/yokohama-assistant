import type {
  AchievementModel,
  ArtManifest,
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

export function Dashboard({
  panels,
  art,
  labels,
  colors,
}: {
  panels: Panel[];
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
}) {
  return (
    <>
      {panels.map((panel, i) => {
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
        return null;
      })}
    </>
  );
}

const good = (n: number, cls: string, label: string) =>
  n > 0 ? (
    <span className={`chip ${cls}`} key={cls}>
      {n} {label}
    </span>
  ) : null;

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
              {p.goods.copper + p.goods.silk + p.goods.tea + p.goods.seafood + p.goods.imports ===
                0 && <span className="chip muted">no goods</span>}
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
              <span className="chip">shop {p.shophouses.built}/8</span>
              <span className="chip">
                trade {p.trading_houses.built}/4 (next &#165;{p.trading_houses.next_take_cost})
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
              {(p.mercenaries_unused > 0 || p.mercenaries_used > 0) && (
                <span className="chip" title="face-up mercenaries score 1 VP each">
                  merc {p.mercenaries_unused} unused / {p.mercenaries_used} used
                </span>
              )}
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
  title,
}: {
  occupant: PlayerId | null;
  colors: PlayerColor[];
  children: React.ReactNode;
  title: string;
}) {
  const dummy = occupant === DUMMY_SEAT;
  const cls = ["space", occupant != null ? "taken" : "", dummy ? "dummy" : seatClass(colors[occupant ?? -1])]
    .filter(Boolean)
    .join(" ");
  return (
    <span className={cls} title={title}>
      {children}
      {dummy && <span className="occ">D</span>}
      {occupant != null && !dummy && <span className="occ">&#9679;</span>}
    </span>
  );
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

function Tracks({
  tracks,
  art,
  colors,
}: {
  tracks: TracksPanel;
  art: ArtManifest | null;
  colors: PlayerColor[];
}) {
  const m = tracks.mercenary_supply;
  const mercTotal = m.english + m.american + m.french + m.german + m.dutch;
  // The two summary figures the CLI prints beside each track: the top space's
  // reward, which is what the race is for.
  const church = tracks.church[tracks.church.length - 1];
  const customs = tracks.customs[tracks.customs.length - 1];
  return (
    <section className="panel">
      <h2>
        Tracks &mdash; ends at {tracks.trigger_at} of YOUR pawns &middot; D = dummy, not counted
      </h2>

      <div className="tile-row muted">
        Church faith &middot; {tracks.church_filled}/{tracks.trigger_at}
        {church && (
          <span className="chip vp">
            top ★{church.vp}
            {church.movement_rider > 0 ? ` +${church.movement_rider}mv` : ""}
          </span>
        )}
      </div>
      <div className="track">
        {tracks.church.map((s) => (
          <Space
            key={s.faith}
            occupant={s.occupant}
            colors={colors}
            title={`faith ${s.faith} — ★${s.vp}${s.movement_rider ? ` +${s.movement_rider} movement` : ""}`}
          >
            {s.faith}
          </Space>
        ))}
      </div>

      <div className="tile-row muted" style={{ marginTop: "0.5rem" }}>
        Customs &middot; {tracks.customs_filled}/{tracks.trigger_at}
        {customs && <span className="chip vp">top ★{customs.vp}</span>}
      </div>
      <div className="track">
        {tracks.customs.map((s) => (
          <Space
            key={s.space}
            occupant={s.occupant}
            colors={colors}
            title={`pay ${s.import_cost} imports — ★${s.vp}${s.warehouse_rider ? ` +${s.warehouse_rider} warehouse` : ""}${s.yen_rider ? ` +¥${s.yen_rider}` : ""}`}
          >
            {s.import_cost}
          </Space>
        ))}
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

function Achievements({
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
