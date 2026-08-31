// Resolving an engine identifier to a picture.
//
// The manifest is the server's, fetched once. Everything here is a lookup —
// nothing decides what a card IS, and nothing falls back to a guessed filename:
// a key that is not in the manifest has no art, and the caller draws the typed
// alternative it would have drawn anyway. That is why `art()` returns
// `undefined` rather than a URL that 404s. A broken image is a worse answer than
// no image, because only one of the two can be styled.

import type { ArtEntry, ArtManifest, FivePowerEffect } from "./types";

/** Where the server serves the files. Same origin in both dev and production —
 *  Vite proxies `/art` exactly as it proxies `/api`. */
const BASE = "/art/";

export const artUrl = (entry: ArtEntry | undefined): string | undefined =>
  entry ? BASE + entry.file : undefined;

const look = (
  family: Record<string, ArtEntry> | undefined,
  key: string | number | null | undefined,
): ArtEntry | undefined =>
  family && key !== null && key !== undefined ? family[String(key)] : undefined;

export const orderArt = (m: ArtManifest | null, id: number | null | undefined) =>
  artUrl(look(m?.orders, id));

/** Keyed on the printed TITLE, not the id: both copies of a doubled title print
 *  one face, and the title is the field inside `rules_hash`. */
export const techArt = (m: ArtManifest | null, name: string | null | undefined) =>
  artUrl(look(m?.techs, name));

/** `SiteModel.index` — `null` means no printed card matched, so no art. */
export const siteArt = (m: ArtManifest | null, index: number | null | undefined) =>
  artUrl(look(m?.sites, index));

export const achievementArt = (m: ArtManifest | null, type: string | null | undefined) =>
  artUrl(look(m?.achievements, type));

export const areaArt = (m: ArtManifest | null, areaType: string | null | undefined) =>
  artUrl(look(m?.areas, areaType));

export const mercenaryArt = (m: ArtManifest | null, country: string | null | undefined) =>
  artUrl(look(m?.mercenaries, country));

export const backArt = (m: ArtManifest | null, which: string) => artUrl(look(m?.backs, which));

export const tokenArt = (m: ArtManifest | null, which: string) => artUrl(look(m?.tokens, which));

/** `{"Copper": 2}` -> `"Copper_2"`, the manifest's own key.
 *
 *  A 5-Power face is a (variant, amount) PAIR because `Copper(2)` and a
 *  re-priced `Copper(3)` would be different pictures, so the key spells both.
 *  Derived from the object rather than switched on, so a seventh variant
 *  resolves or misses — it cannot land on the wrong picture. */
export function fivePowerKey(effect: FivePowerEffect): string {
  const [variant, amount] = Object.entries(effect)[0] ?? [];
  return `${variant}_${amount}`;
}

export const fivePowerArt = (m: ArtManifest | null, effect: FivePowerEffect | null | undefined) =>
  effect ? artUrl(look(m?.five_power, fivePowerKey(effect))) : undefined;

/** The face in words: `2 Copper`, `1 Import`, `3 Yen`.
 *
 *  The CLI's own wording (`5-Power: 2 Copper`), restated here because it is the
 *  one label the labels endpoint cannot carry — the amount is part of the value,
 *  not part of the type, so there is no static enum to serve. */
export function fivePowerWords(effect: FivePowerEffect): string {
  const [variant, amount] = Object.entries(effect)[0] ?? ["", 0];
  const noun: Record<string, string> = {
    ImportedGoods: "Import",
    Copper: "Copper",
    Silk: "Silk",
    Tea: "Tea",
    Seafood: "Seafood",
    Money: "Yen",
  };
  return `${amount} ${noun[variant] ?? variant}`;
}
