// A good's token, as the choosers draw it.
//
// **One accessible node.** A group of copies is `role="img"` named by the
// amount and the noun (`2 yen`), and every picture or mark inside it is hidden
// from assistive technology (`alt=""` on a picture, `aria-hidden` on a mark): a
// reader hears the group once, never one copy at a time. The tile and the
// claim band are this repository's precedent; a `group` whose children are all
// hidden is the shape cycle 1 rejected.
//
// **The art is a lookup, not a decision.** `tokenArt` answers from the
// server's manifest; a good the manifest does not hold, or no manifest at all,
// draws the typed alternative — a ring in the good's colour holding its
// letters — because a broken picture is worse than none (`art.ts`).

import { tokenArt } from "./art";
import type { ArtManifest, ChooserGood } from "./types";

/** The manifest key of each good's token picture. */
const ART_KEY: Record<ChooserGood, string> = {
  Copper: "good_copper",
  Silk: "good_silk",
  Tea: "good_tea",
  Seafood: "good_seafood",
  Imports: "good_imports",
  Yen: "yen_1",
};

/** The mark drawn where the manifest has no picture. */
const MARK: Record<ChooserGood, string> = {
  Copper: "Cu",
  Silk: "Si",
  Tea: "Te",
  Seafood: "Sf",
  Imports: "Im",
  Yen: "¥",
};

/** The noun a reader hears after the amount. */
const NOUN: Record<ChooserGood, string> = {
  Copper: "copper",
  Silk: "silk",
  Tea: "tea",
  Seafood: "seafood",
  Imports: "imported good",
  Yen: "yen",
};

/** What a reader hears for `amount` of `good`: `2 yen`, `1 imported good`,
 *  `2 imported goods`. The five mass nouns never change; `imported good` takes
 *  its plural when the amount is not 1 (0 included). Exported for a row that
 *  labels an amount which is not the number of pictures drawn. */
export function tokenLabel(good: ChooserGood, amount: number): string {
  const noun = good === "Imports" && amount !== 1 ? "imported goods" : NOUN[good];
  return `${amount} ${noun}`;
}

/** `amount` copies of one good's token, read as ONE node: `role="img"`,
 *  `aria-label` `2 yen`. With `decorative` the node is `aria-hidden` and has
 *  neither role nor label, for a place whose reading is something else's (a
 *  stock entry's own label says how many are held, and its one token only
 *  decorates it); the copies are drawn all the same. */
export function Tokens({
  good,
  amount,
  art,
  small = false,
  decorative = false,
}: {
  good: ChooserGood;
  amount: number;
  art: ArtManifest | null;
  small?: boolean;
  decorative?: boolean;
}) {
  const src = tokenArt(art, ART_KEY[good]);
  const kind = good.toLowerCase();
  return (
    <span
      className="tokens"
      {...(decorative
        ? { "aria-hidden": "true" as const }
        : { role: "img", "aria-label": tokenLabel(good, amount) })}
      data-good={good}
    >
      {Array.from({ length: amount }, (_, i) => (
        <span
          key={i}
          className={`tok ${kind}${small ? " sm" : ""}${src ? " has-img" : ""}`}
          aria-hidden="true"
        >
          {src ? <img src={src} alt="" aria-hidden="true" /> : MARK[good]}
        </span>
      ))}
    </span>
  );
}
