// Cards, drawn as the printed card where art exists and as typed values where
// it does not.
//
// **Both halves are always rendered.** The art is not a substitute for the
// numbers: a 180px order card is legible as an identity and not as a
// requirement list, and a player deciding whether they can fulfil an order is
// counting goods, not admiring a fish. So every card here is the picture with
// the values BESIDE it, which is also what makes an art-less checkout degrade
// to exactly the surface that existed before there was any art rather than to a
// row of broken images.

import type {
  AchievementModel,
  ArtManifest,
  Labels,
  OrderCard,
  PlayerColor,
  PlayerId,
  SiteModel,
  SpaceReward,
  TechCard,
} from "./types";
import { achievementArt, orderArt, siteArt, techArt } from "./art";

export const seatClass = (c: PlayerColor | undefined): string =>
  c ? `seat-${c.toLowerCase()}` : "";

/** The good names in the engine's own order, with their chip class. */
const GOODS = [
  ["copper", "copper"],
  ["silk", "silk"],
  ["tea", "tea"],
  ["seafood", "seafood"],
] as const;

/** What an order card asks for. Requirement keys are always present, zeros
 *  included, so the filter is what makes the list read as the printed card. */
function requirementChips(card: OrderCard) {
  return GOODS.filter(([k]) => card.requirements[k] > 0).map(([k, cls]) => (
    <span className={`chip ${cls}`} key={k}>
      {card.requirements[k]} {k}
    </span>
  ));
}

/** What it pays. Ten reward keys, all always present; only the non-zero ones
 *  are printed on the physical card and only those are printed here. */
function rewardChips(card: OrderCard) {
  const r = card.rewards;
  const rows: [number, string, string][] = [
    [r.victory_points, "vp", "VP"],
    [r.money, "yen", "¥"],
    [r.imported_goods, "imports", "import"],
    [r.copper, "copper", "copper"],
    [r.silk, "silk", "silk"],
    [r.tea, "tea", "tea"],
    [r.seafood, "seafood", "seafood"],
    [r.any_trade_good, "wild", "any good"],
    [r.movement, "", "move"],
    [r.warehouse_tokens, "", "from warehouse"],
  ];
  return rows
    .filter(([n]) => n > 0)
    .map(([n, cls, label], i) => (
      <span className={`chip ${cls}`} key={i}>
        {n} {label}
      </span>
    ));
}

export function OrderCardView({
  art,
  card,
  id,
  note,
  size = "md",
}: {
  art: ArtManifest | null;
  card: OrderCard | null;
  id: number;
  note?: string;
  size?: "sm" | "md";
}) {
  const src = orderArt(art, id);
  return (
    <figure className={`card-face order ${size}`}>
      {src ? (
        <img src={src} alt="" loading="lazy" />
      ) : (
        <div className="card-blank">O{String(id).padStart(2, "0")}</div>
      )}
      <figcaption>
        <div className="card-title">
          <span className="card-id">O{String(id).padStart(2, "0")}</span>
          <span>{card?.country ?? ""}</span>
        </div>
        {card && (
          <>
            <div className="chips">{requirementChips(card)}</div>
            <div className="chips pays">{rewardChips(card)}</div>
          </>
        )}
        {note && <div className="card-note">{note}</div>}
      </figcaption>
    </figure>
  );
}

export function TechCardView({
  art,
  card,
  labels,
  cost,
}: {
  art: ArtManifest | null;
  card: TechCard | null;
  labels: Labels | null;
  /** The industry the slot actually costs — the printed value PLUS the slot
   *  surcharge. Omit where there is no slot (a card already owned). */
  cost?: number;
}) {
  const src = techArt(art, card?.name);
  return (
    <figure className="card-face tech">
      {src ? (
        <img src={src} alt="" loading="lazy" />
      ) : (
        <div className="card-blank">{card?.name ?? "empty"}</div>
      )}
      <figcaption>
        <div className="card-title">
          <span>{card?.name ?? "empty slot"}</span>
          {card && (
            <span className="chip gear">
              {cost ?? card.industry_cost}
              {cost != null && cost !== card.industry_cost ? "↑" : ""}
            </span>
          )}
        </div>
        {card && (
          <div className="card-note">
            {labels?.tech_effects[card.effect] ?? card.effect}
          </div>
        )}
      </figcaption>
    </figure>
  );
}

export function AchievementCardView({
  art,
  model,
  labels,
  colors,
}: {
  art: ArtManifest | null;
  model: AchievementModel;
  labels: Labels | null;
  colors: PlayerColor[];
}) {
  const src = achievementArt(art, model.achievement_type);
  return (
    <figure className="card-face achievement">
      {src ? (
        <img src={src} alt="" loading="lazy" />
      ) : (
        <div className="card-blank">{model.category}</div>
      )}
      <figcaption>
        <div className="card-title">
          <span className="tag">{model.category}</span>
          <span>{labels?.achievements[model.achievement_type] ?? model.achievement_type}</span>
        </div>
        <div className="chips">
          <span className="chip vp">
            {model.first_place_vp} / {model.subsequent_vp} VP
          </span>
          {model.claimed_by.length === 0 ? (
            <span className="chip">unclaimed</span>
          ) : (
            model.claimed_by.map((seat, i) => (
              <span className={`chip claimed ${seatClass(colors[seat])}`} key={seat}>
                {i === 0 ? "first" : "also"} {colors[seat] ?? `seat ${seat}`}
              </span>
            ))
          )}
        </div>
        <Progress model={model} colors={colors} />
      </figcaption>
    </figure>
  );
}

/** Per-seat progress toward the condition.
 *
 *  `current >= needed` is "ready by the numbers", NOT "claimable" — a claim also
 *  needs an assistant in hand and no prior claim by that seat — so the bar fills
 *  and says `ready`, and never says `claim this`. */
function Progress({ model, colors }: { model: AchievementModel; colors: PlayerColor[] }) {
  if (model.progress.length === 0) return null;
  return (
    <div className="progress-rows">
      {model.progress.map(([seat, current, needed]) => (
        <div className="progress-row" key={seat}>
          <span className={`dot ${seatClass(colors[seat])}`} />
          <span className="bar">
            <span
              className={`fill ${seatClass(colors[seat])}`}
              style={{ width: `${needed > 0 ? Math.min(100, (current / needed) * 100) : 0}%` }}
            />
          </span>
          <span className="num">
            {current}/{needed}
          </span>
        </div>
      ))}
    </div>
  );
}

/** The printed building-site card for a tile.
 *
 *  `index` is `null` when no printed card matches the tile's spaces (or more
 *  than one does), and then there is no art and no number to show — which is a
 *  real state of the model, not a load failure. */
export function SiteCardView({
  art,
  index,
  children,
}: {
  art: ArtManifest | null;
  index: number | null;
  children?: React.ReactNode;
}) {
  const src = siteArt(art, index);
  return (
    <figure className="card-face site sm">
      {src ? (
        <img src={src} alt="" loading="lazy" />
      ) : (
        <div className="card-blank">{index == null ? "site" : `Site ${index}`}</div>
      )}
      <figcaption>{children}</figcaption>
    </figure>
  );
}

/** The printed building-site card with what is BUILT drawn on it.
 *
 *  The overlay rects are MEASURED (`styles.css`'s `.site-space.slot-*`): each
 *  of the 24 site arts minus its own 9px box blur, averaged, so the printed
 *  rule survives and every card's own content cancels. An unbuilt space draws
 *  NOTHING, so the printed card reads exactly as it does on the table. Shared by
 *  the building-sites strip and, since M5, by every tile. */
export function SiteFace({
  art,
  site,
  colors,
}: {
  art: ArtManifest | null;
  site: SiteModel;
  colors: PlayerColor[];
}) {
  const face = siteArt(art, site.index);
  const th = site.trading_house_owner;
  return (
    <div className="site-face">
      {face ? (
        <img src={face} alt="" loading="lazy" />
      ) : (
        <div className="card-blank">{site.index == null ? "site" : `Site ${site.index}`}</div>
      )}
      {site.slots.map((slot, i) => (
        <span
          key={i}
          className={`site-space slot-${i} ${
            slot.built_by != null ? `built ${seatClass(colors[slot.built_by])}` : ""
          }`}
        >
          {slot.built_by != null && <span className="built-mark">&#8962;</span>}
        </span>
      ))}
      <span className={`site-trading-house ${th != null ? `built ${seatClass(colors[th])}` : ""}`}>
        {th != null && <span className="built-mark">&#8962;</span>}
      </span>
    </div>
  );
}

/** One occupant mark: a president triangle or an assistant dot with its count.
 *
 *  The glyphs are the CLI's own legend — `▲` president, `●` assistants
 *  — so a player who has read one surface can read the other. */
export function Pawns({
  presidents,
  assistants,
  hasStation,
  colors,
}: {
  presidents: PlayerId[];
  assistants: [PlayerId, number][];
  hasStation: boolean;
  colors: PlayerColor[];
}) {
  const occupied = assistants.filter(([, n]) => n > 0);
  if (presidents.length === 0 && occupied.length === 0 && !hasStation) {
    return <span className="muted">no pawns</span>;
  }
  return (
    <>
      {presidents.map((seat) => (
        <span
          key={`p${seat}`}
          className={`pawn president ${seatClass(colors[seat])}`}
          title={`${colors[seat] ?? `seat ${seat}`} president`}
        >
          &#9650;
        </span>
      ))}
      {occupied.map(([seat, n]) => (
        <span
          key={`a${seat}`}
          className={`pawn ${seatClass(colors[seat])}`}
          title={`${n} ${colors[seat] ?? `seat ${seat}`} assistant${n === 1 ? "" : "s"}`}
        >
          &#9679;
          <span className="pawn-n">{n}</span>
        </span>
      ))}
      {hasStation && (
        <span className="chip station" title="Station — +1 Power for everyone here">
          St
        </span>
      )}
    </>
  );
}

/** The printed reward on a building-site space, in the CLI's own words.
 *
 *  **One vocabulary for the whole client, and it is the terminal's.**
 *  `Index.tsx` used to word these itself — `${n} ${kind.replace(/_/g, " ")}` —
 *  and it disagreed with `card_render::space_reward_tokens` on **55 of the 96
 *  printed spaces**, which is most of them. Two of the disagreements are not
 *  cosmetic:
 *
 *  * `movement` rendered as "movement", and `SpaceReward.movement` is the
 *    ASSISTANT-move icon (`types/board.rs:109-111`). "2 movement" on a space
 *    reads as President movement, which is a different rule.
 *  * `warehouse_tokens` rendered as "warehouse tokens", and the CLI's word is
 *    "warehouse take" — you take FROM the warehouse; the number is how many
 *    takes, not how many tokens sit there.
 *
 *  It also lost the plural on "any goods", the capitalised goods names, and the
 *  ` + ` that joins a space paying two things — and it printed an em-dash for a
 *  blank space, which is indistinguishable from an absent one. `card_render.rs`
 *  keeps the literal word "blank" for exactly that reason.
 *
 *  `Board.tsx:91-98` argues this case in its own comment — *"The CLI's alphabet,
 *  not one of our own … A browser inventing `m` and `w` gives a player
 *  cross-reading the two surfaces an unknown token"* — and the site surfaces
 *  were the place that had not taken it. */
export function spaceRewardTokens(r: SpaceReward): { word: string; kind: string }[] {
  const out: { word: string; kind: string }[] = [];
  if (r.vp > 0) out.push({ word: `${r.vp} VP`, kind: "vp" });
  if (r.yen > 0) out.push({ word: `${r.yen} yen`, kind: "yen" });
  const rest: [number, string, string, string][] = [
    [r.seafood, "Seafood", "Seafood", "seafood"],
    [r.tea, "Tea", "Tea", "tea"],
    [r.silk, "Silk", "Silk", "silk"],
    [r.copper, "Copper", "Copper", "copper"],
    [r.any_good, "any good", "any goods", "any_good"],
    [r.movement, "assistant move", "assistant moves", "movement"],
    [r.warehouse_tokens, "warehouse take", "warehouse takes", "warehouse_tokens"],
  ];
  for (const [n, one, many, kind] of rest) {
    if (n > 0) out.push({ word: `${n} ${n === 1 ? one : many}`, kind });
  }
  // Every card in the set prints one blank space, so this is a real face rather
  // than a defensive arm — and it is a WORD, because an em-dash for "pays
  // nothing" cannot be told from an em-dash for "there is no space here".
  if (out.length === 0) out.push({ word: "blank", kind: "dim" });
  return out;
}

/** The same words, joined as the terminal joins them. */
export const spaceWords = (r: SpaceReward): string =>
  spaceRewardTokens(r)
    .map((t) => t.word)
    .join(" + ");

/** Where a space sits on the printed card. The CLI's own words
 *  (`describe.rs:131-142`), not the TL/TR/ML/MR shorthand. */
export const SPACE_POSITIONS = ["top-left", "top-right", "mid-left", "mid-right"] as const;
