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
import { AchievementCardView, OrderCardView, SiteCardView, TechCardView, spaceRewardTokens } from "./Cards";
import type {
  ArtManifest,
  CardLocation,
  IndexCard,
  IndexGroup,
  IndexModel,
  Labels,
  Outcome,
  PlayerColor,
  SpaceReward,
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

/** Where a card is, in the CLI's own words.
 *
 *  **Exhaustive over all eight `CardLocation` variants, with a `never` check
 *  rather than a fallback.** The first draft handled five and fell through to
 *  `String(tag)`, which printed the literal Rust tag `OnArea` under ten of the
 *  twenty-four site cards — a silent wrong answer with a payload behind it in
 *  the committed corpus. A fallback that renders a tag as prose is not an
 *  honest last resort; it is a defect that cannot fail a test. Now a ninth
 *  variant is a COMPILE ERROR here.
 *
 *  The wording MIRRORS `describe::card_location` (crates/yokohama-cli/src/
 *  describe.rs:88) word for word, and a divergence between the two is a defect
 *  rather than a style choice — the terminal and the browser must not describe
 *  the same card differently. */
function locationWords(location: CardLocation | null, colors: PlayerColor[]): string | null {
  if (location === null) return null;
  if (location === "OwnHand") return "in your hand";
  if (location === "ThisReveal") return "in this reveal";
  const seatName = (seat: number) => colors[seat] ?? `Seat ${seat}`;
  if ("Deal" in location) return `Seat ${location.Deal.seat}'s deal`;
  if ("PortSlot" in location) {
    return `Port ${location.PortSlot.board_b ? "B" : "A"} slot ${location.PortSlot.slot + 1}`;
  }
  if ("LabSlot" in location) {
    return `Laboratory ${location.LabSlot.board_b ? "B" : "A"} slot ${location.LabSlot.slot + 1}`;
  }
  if ("OwnedBy" in location) return `owned by ${seatName(location.OwnedBy.seat)}`;
  if ("OnArea" in location) return `on area A${String(location.OnArea.area).padStart(2, "0")}`;
  if ("AchievementSlot" in location) {
    return `category ${location.AchievementSlot.category} in play`;
  }
  // A ninth variant lands HERE as a type error, which is the point.
  const unreachable: never = location;
  return unreachable;
}

/** **Where every physical copy of a TITLE is.**
 *
 *  The index is over titles, not cards: ten of the twenty titles circulate as
 *  two identical copies so that two players can each own one. The nav blurb
 *  promises "who owns which copy", and the first draft showed only the card's
 *  single `location` — which is the FIRST copy's — so a title owned by both
 *  seats read as owned by one.
 *
 *  The terminal cannot fit this: `card_render::copy_note` renders the same fact
 *  as `= owned by Red +1 more`, because one interior line is what a card-sized
 *  cell has. A browser has the room, so it says which copy is where.
 *
 *  `copies` is every printed copy and `placed` only the ones the index has
 *  located, so the difference is the number still in the deck — a real fact and
 *  not an absence. */
function Copies({
  copies,
  placed,
  colors,
}: {
  copies: number[];
  placed: [number, CardLocation][];
  colors: PlayerColor[];
}) {
  const unseen = copies.length - placed.length;
  return (
    <ul className="copies">
      {placed.map(([id, at]) => (
        <li key={id}>
          <code>T{String(id).padStart(2, "0")}</code>
          <span>{locationWords(at, colors)}</span>
        </li>
      ))}
      {unseen > 0 && (
        <li className="muted">
          <code>{unseen === copies.length ? "—" : `+${unseen}`}</code>
          <span>{unseen === 1 ? "1 copy in the deck" : `${unseen} copies in the deck`}</span>
        </li>
      )}
    </ul>
  );
}

/** One printed space on a building-site card, as the reward it pays.
 *
 *  Without this every one of the twenty-four site cards rendered identically —
 *  the panel showed a count of spaces and no reward at all, while its nav blurb
 *  promised "what every space pays". The chip vocabulary is the house one the
 *  card faces already use. */
function SpaceChip({ reward }: { reward: SpaceReward }) {
  // The words are `Cards.tsx`'s `spaceRewardTokens`, which is the TERMINAL's
  // vocabulary. This function used to invent its own — `${n} ${kind.replace(/_/g,
  // " ")}` — and disagreed with `card_render::space_reward_tokens` on 55 of the
  // 96 printed spaces, including two that misstate a rule. See that function's
  // doc; the whole point is that there is one vocabulary and it is not this
  // file's.
  return (
    <>
      {spaceRewardTokens(reward).map((t, i) => (
        <span className={`chip ${t.kind}`} key={`${t.kind}-${i}`}>
          {t.word}
        </span>
      ))}
    </>
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
  // **The existing card views, not new ones.** `Cards.tsx` already draws every
  // one of these faces with its art, its chips and — for an achievement — the
  // per-seat progress bar whose doc states the rule the browser must not round
  // off ("`current >= needed` is 'ready by the numbers', NOT 'claimable'"). The
  // first draft of this file hand-rolled a goal and a site cell, and a review
  // found what that cost: no achievement art, no progress at all while the nav
  // blurb promised "how close every seat is", every site card rendering
  // identically because the space rewards were dropped, and the site number off
  // by one against every other surface in the project.
  const body =
    "Order" in face ? (
      <OrderCardView card={face.Order.printed} id={face.Order.id} art={art} size="sm" />
    ) : "Tech" in face ? (
      <>
        <TechCardView card={face.Tech.printed} art={art} labels={labels} />
        <Copies copies={face.Tech.copies} placed={face.Tech.placed} colors={colors} />
      </>
    ) : "Achievement" in face ? (
      <AchievementCardView model={face.Achievement} art={art} labels={labels} colors={colors} />
    ) : (
      <SiteCardView art={art} index={face.Site.index}>
        <div className="chips">
          {/* **The number, always.** `SiteCardView` prints `Site n` only in its
              art-less blank, so with the art present the panel showed no site
              identifier at all — worse than the off-by-one it replaced, because
              a card index a player cannot use to identify a card is not an
              index. 0-BASED, as the board tile, the CLI's tile, its capture
              name and its own site-index card all print it. */}
          <span className="chip site-id">Site {face.Site.index}</span>
          <span className="chip">trading house {face.Site.site.trading_house_reward} VP</span>
          {face.Site.site.spaces.map((sp, i) => (
            <SpaceChip key={i} reward={sp} />
          ))}
        </div>
      </SiteCardView>
    );

  // A tech's `location` is its FIRST copy's, and `Copies` above says where all
  // of them are — so repeating it here would state one copy's whereabouts twice
  // and the other's nowhere.
  const showWhere = where !== null && !("Tech" in face);
  return (
    <li className={where ? "index-card placed" : "index-card"}>
      {body}
      {showWhere && <span className="meta">{where}</span>}
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
  //
  // **The unfiltered case is skipped**, because `first` already IS that answer:
  // the panel shell fetched it before this component mounted, and refetching it
  // fired the identical GET twice on every open — a full card index, twice, for
  // nothing.
  useEffect(() => {
    if (term.trim() === "") {
      setModel(first);
      setError(null);
      return;
    }
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
  }, [term, game, kind, first]);

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
          {error ? `${model.total} cards, none shown` : `showing ${model.shown} of ${model.total}`}
        </span>
      </p>
      {/* **A refused filter clears the list rather than leaving the last one
          under it.** The count line and the cards below are the ANSWER to a
          filter; leaving the previous answer beside a refusal says the screen
          is showing something it is not, which is the same class of defect as a
          number that means a different move. */}
      {error && <div className="error">{error}</div>}
      {!error && model.shown === 0 && <p className="muted">Nothing matches that.</p>}
      {/* Orders are the only kind whose face is a small thumbnail; the other
          three carry prose, chips or a progress table and need a cell wide
          enough not to wrap every word. */}
      {!error && model.groups.map((g, i) => (
        <Group
          key={i}
          group={g}
          art={art}
          labels={labels}
          colors={colors}
          wide={kind !== "orders"}
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
