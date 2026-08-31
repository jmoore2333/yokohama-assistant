// The four card indexes, and the final score.
//
// **The index is the CLI's canonical identification mechanism**, and this draws
// the same model the terminal draws. Two differences, both deliberate and both
// properties of the API rather than of this component:
//
//  - **Nothing is numbered.** In the terminal an index can be open INSIDE a card
//    capture, and its cards then carry selection numbers answering that capture.
//    This API has no capture — the input grammar refuses `?`/`index`/`orders`/
//    `techs` precisely so a digit cannot silently change meaning — so `target`
//    is always null and so is every `selection`.
//  - **The filter is the server's.** There is exactly one card-entry grammar in
//    this project. Typing here sends the term to `card_prompt::parse_filter`,
//    and an unparseable one comes back as the CLI's own refusal rather than as
//    a client-side guess.

import { useEffect, useState } from "react";
import { api, ApiError } from "./api";
import { achievementArt } from "./art";
import { OrderCardView, TechCardView } from "./Cards";
import type {
  AchievementModel,
  ArtManifest,
  IndexCard,
  IndexGroup,
  IndexModel,
  Labels,
  Outcome,
  PlayerColor,
} from "./types";

/** The group heading. `key` is serde's externally-tagged enum — `{Country: …}`
 *  for orders and techs, `{Category: …}` for goals, the bare string `"All"` for
 *  sites — so it is read structurally rather than assumed. */
function groupLabel(key: unknown, labels: Labels | null): string {
  if (typeof key === "string") return key === "All" ? "" : key;
  if (key && typeof key === "object") {
    const [tag, value] = Object.entries(key as Record<string, unknown>)[0] ?? [];
    if (tag === "Country") return String(value);
    if (tag === "Category") return `Category ${String(value)}`;
    if (tag === "Goods") return labels?.area_types?.[String(value)] ?? String(value);
    return `${tag} ${String(value)}`;
  }
  return "";
}

/** Where a card is, when the index is tracking it. `null` is NOT "nowhere": the
 *  index deliberately carries no other seat's hand. */
function locationWords(location: unknown, colors: PlayerColor[]): string | null {
  if (location === null || location === undefined) return null;
  if (location === "OwnHand") return "in your hand";
  if (location === "ThisReveal") return "in this reveal";
  if (typeof location === "object") {
    const [tag, value] = Object.entries(location as Record<string, unknown>)[0] ?? [];
    const inner = (value ?? {}) as Record<string, unknown>;
    if (tag === "OwnedBy") return `owned by ${colors[Number(inner.seat)] ?? `seat ${inner.seat}`}`;
    if (tag === "PortSlot") return `port ${inner.board_b ? "B" : "A"}, slot ${Number(inner.slot) + 1}`;
    if (tag === "LabSlot") return `lab ${inner.board_b ? "B" : "A"}, slot ${Number(inner.slot) + 1}`;
    return String(tag);
  }
  return null;
}

/** An achievement in play, with **how close every seat is to it**.
 *
 *  The progress vector is in the payload and the first draft of this component
 *  dropped it — while the nav blurb beside it promised "how close every seat
 *  is". That is the M2.5 lesson repeating: a milestone's own sentence claiming
 *  something the screen does not do. `current >= needed` is "ready by the
 *  numbers" and not "claimable" (a claim also needs an assistant in hand), so
 *  the row says READY rather than anything stronger. */
function Goal({
  a,
  art,
  labels,
  colors,
}: {
  a: AchievementModel;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
}) {
  const url = achievementArt(art, a.achievement_type);
  return (
    <div className="index-goal">
      {url && <img src={url} alt="" loading="lazy" />}
      <b>{labels?.achievements?.[a.achievement_type] ?? a.achievement_type}</b>
      <span className="meta">
        {a.first_place_vp} VP first, then {a.subsequent_vp}
      </span>
      <ul className="goal-progress">
        {a.progress.map(([seat, current, needed]) => (
          <li key={seat} className={current >= needed ? "ready" : undefined}>
            <span>{colors[seat] ?? `seat ${seat}`}</span>
            <span className="num">
              {current}/{needed}
            </span>
          </li>
        ))}
      </ul>
      {a.claimed_by.length > 0 && (
        <span className="meta">
          claimed by {a.claimed_by.map((s) => colors[s] ?? `seat ${s}`).join(", ")}
        </span>
      )}
    </div>
  );
}

function Card({
  card,
  art,
  labels,
  colors,
}: {
  card: IndexCard;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
}) {
  const where = locationWords(card.location, colors);
  const face = card.face;
  const body =
    "Order" in face ? (
      <OrderCardView card={face.Order.printed} id={face.Order.id} art={art} size="sm" />
    ) : "Tech" in face ? (
      <TechCardView card={face.Tech.printed} art={art} labels={labels} />
    ) : "Achievement" in face ? (
      <Goal a={face.Achievement} art={art} labels={labels} colors={colors} />
    ) : (
      <div className="index-site">
        <b>Site {face.Site.index + 1}</b>
        <span className="meta">
          {face.Site.site.spaces.length} spaces · trading house {face.Site.site.trading_house_reward} VP
        </span>
      </div>
    );

  return (
    <li className={where ? "index-card placed" : "index-card"}>
      {body}
      {where && <span className="meta">{where}</span>}
    </li>
  );
}

function Group({
  group,
  art,
  labels,
  colors,
  wide,
}: {
  group: IndexGroup;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  wide: boolean;
}) {
  const heading = groupLabel(group.key, labels);
  return (
    <section>
      {heading && <h3>{heading}</h3>}
      <ul className={wide ? "index-cards wide" : "index-cards"}>
        {group.cards.map((card, i) => (
          <Card key={i} card={card} art={art} labels={labels} colors={colors} />
        ))}
      </ul>
    </section>
  );
}

export function IndexView({
  first,
  kind,
  game,
  art,
  labels,
  colors,
}: {
  first: IndexModel;
  kind: string;
  game: string;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
}) {
  const [term, setTerm] = useState("");
  const [model, setModel] = useState(first);
  const [error, setError] = useState<string | null>(null);

  // The filter is a ROUND TRIP, deliberately. Narrowing in the browser would be
  // a second implementation of a grammar the CLI owns, and the two would
  // disagree on the first ambiguous prefix a player typed.
  useEffect(() => {
    let live = true;
    setError(null);
    const t = setTimeout(() => {
      api
        .index(game, kind, term.trim() || undefined)
        .then((m) => live && setModel(m))
        .catch((e) => live && setError(e instanceof ApiError ? e.message : String(e)));
    }, 200);
    return () => {
      live = false;
      clearTimeout(t);
    };
  }, [term, game, kind]);

  return (
    <div className="index">
      <p className="index-head">
        <label>
          Filter
          <input
            type="text"
            value={term}
            onChange={(e) => setTerm(e.target.value)}
            placeholder="a good, a country, a flag…"
          />
        </label>
        <span className="meta">
          showing {model.shown} of {model.total}
        </span>
      </p>
      {error && <div className="error">{error}</div>}
      {model.shown === 0 && !error && <p className="muted">Nothing matches that.</p>}
      {model.groups.map((g, i) => (
        <Group
          key={i}
          group={g}
          art={art}
          labels={labels}
          colors={colors}
          wide={kind === "goals" || kind === "sites"}
        />
      ))}
    </div>
  );
}

/** The final score, ranked, with every category the engine scored.
 *
 *  **The parts are shown beside the total** because a total with nothing behind
 *  it is a number a player cannot check — and several of these categories carry
 *  rulings (the church 6/3 and customs 8/4 majorities, the technology majority
 *  the 2P dummy competes in at industry value 7.5) that a player has every
 *  reason to want itemised. */
export function OutcomeView({ outcome, colors }: { outcome: Outcome; colors: PlayerColor[] }) {
  const parts = [
    ["base_vp", "VP"],
    ["church", "Church"],
    ["customs", "Customs"],
    ["technology", "Tech"],
    ["country", "Country"],
    ["remainder", "Remainder"],
  ] as const;
  return (
    <table className="outcome">
      <thead>
        <tr>
          <th>#</th>
          <th>Seat</th>
          {parts.map(([, label]) => (
            <th key={label} className="num">
              {label}
            </th>
          ))}
          <th className="num">Total</th>
          <th className="num">Margin</th>
        </tr>
      </thead>
      <tbody>
        {outcome.standings.map((p) => (
          <tr key={p.seat} className={p.won ? "won" : undefined}>
            <td>{p.rank}</td>
            <td>{colors[p.seat] ?? `seat ${p.seat}`}</td>
            {parts.map(([key, label]) => (
              <td key={label} className="num">
                {p.breakdown[key]}
              </td>
            ))}
            <td className="num total">{p.breakdown.total}</td>
            {/* A margin of 0 is NOT a win: two seats can tie on points and be
                separated by the engine's own tie-break, so the winner is marked
                by `won` and the margin is only the distance. */}
            <td className="num">{p.margin > 0 ? `+${p.margin}` : p.margin}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
