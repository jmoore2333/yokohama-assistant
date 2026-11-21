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

/** `amount` copies of one good's token, read as ONE node: `role="img"`,
 *  `aria-label` `2 yen`. */
export function Tokens({
  good,
  amount,
  art,
  small = false,
}: {
  good: ChooserGood;
  amount: number;
  art: ArtManifest | null;
  small?: boolean;
}) {
  const src = tokenArt(art, ART_KEY[good]);
  const kind = good.toLowerCase();
  return (
    <span
      className="tokens"
      role="img"
      aria-label={`${amount} ${NOUN[good]}`}
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
