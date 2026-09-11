// Resolving an engine identifier to a picture.
//
// The join itself is proven TOTAL by `crates/yokohama-web/src/art.rs` — every
// identifier the engine can produce resolves, and nothing in the manifest names
// one the engine lacks. Those tests prove the join is total, NOT true: a
// wrong-but-complete mapping is exactly what they cannot see, and two such
// errors (the crop origin, and yellow being COMMERCIAL rather than production
// on the category-C achievements) were caught only by looking at the pictures.
//
// What is left for this side is the LOOKUP: that a miss answers `undefined`
// rather than a guessed filename, and that the one derived key is derived
// rather than switched on.

import { describe, expect, it } from "vitest";
import {
  achievementArt,
  areaArt,
  artUrl,
  fivePowerArt,
  fivePowerKey,
  fivePowerWords,
  mercenaryArt,
  orderArt,
  siteArt,
  techArt,
} from "./art";
import { achievementsOf, handOf } from "./test/panels";
import { wire } from "./wire/contract";
import type { FivePowerEffect } from "./types";

describe("a miss", () => {
  // A broken image is a worse answer than no image, because only one of the two
  // can be styled — so a key that is not in the manifest has NO art and the
  // caller draws the typed alternative it would have drawn anyway.
  it("answers undefined rather than a URL that 404s", () => {
    expect(orderArt(wire.artManifest, 9999)).toBeUndefined();
    expect(techArt(wire.artManifest, "No Such Technology")).toBeUndefined();
    expect(siteArt(wire.artManifest, null)).toBeUndefined();
    expect(areaArt(wire.artManifest, "NoSuchArea")).toBeUndefined();
    expect(achievementArt(wire.artManifest, undefined)).toBeUndefined();
    expect(mercenaryArt(wire.artManifest, "Martian")).toBeUndefined();
    expect(fivePowerArt(wire.artManifest, null)).toBeUndefined();
  });

  it("an absent manifest is a miss everywhere, not a crash", () => {
    expect(orderArt(null, 1)).toBeUndefined();
    expect(techArt(null, "Tram")).toBeUndefined();
    expect(areaArt(null, "Bank")).toBeUndefined();
    expect(fivePowerArt(null, { Copper: 2 })).toBeUndefined();
    expect(artUrl(undefined)).toBeUndefined();
  });
});

describe("a hit", () => {
  it("is served from the same origin the server serves art on", () => {
    const url = orderArt(wire.artManifest, handOf(wire.options)!.cards[0]!.id);
    expect(url).toMatch(/^\/art\/.+\.webp$/);
  });

  it("resolves every identifier the fixtures actually carry", () => {
    for (const tile of wire.options4p.frame.board.tiles) {
      // The Canal has no area art and no token; every other tile has both.
      if (tile.area_type === "Canal") continue;
      expect(areaArt(wire.artManifest, tile.area_type), tile.area_type).toBeDefined();
      if (tile.token) expect(fivePowerArt(wire.artManifest, tile.token)).toBeDefined();
      if (tile.site?.index != null) {
        expect(siteArt(wire.artManifest, tile.site.index)).toBeDefined();
      }
    }
    for (const card of achievementsOf(wire.options)) {
      expect(achievementArt(wire.artManifest, card.achievement_type)).toBeDefined();
    }
    for (const slot of wire.labs.boards.flatMap((b) => b.slots)) {
      if (slot.printed) expect(techArt(wire.artManifest, slot.printed.name)).toBeDefined();
    }
    for (const slot of wire.ports.boards.flatMap((b) => b.slots)) {
      if (slot.card != null) expect(orderArt(wire.artManifest, slot.card)).toBeDefined();
    }
  });
});

describe("the 5-Power face", () => {
  // A face is a (variant, amount) PAIR, because `Copper(2)` and a re-priced
  // `Copper(3)` are different pictures. Derived from the object rather than
  // switched on, so a seventh variant resolves or MISSES — it cannot land on
  // the wrong picture.
  it("keys on both halves of the pair", () => {
    expect(fivePowerKey({ Copper: 2 })).toBe("Copper_2");
    expect(fivePowerKey({ ImportedGoods: 1 })).toBe("ImportedGoods_1");
    expect(fivePowerKey({ Money: 3 })).toBe("Money_3");
  });

  it("a variant the manifest does not carry misses rather than mis-resolving", () => {
    const unknown = { Plutonium: 4 } as unknown as FivePowerEffect;
    expect(fivePowerKey(unknown)).toBe("Plutonium_4");
    expect(fivePowerArt(wire.artManifest, unknown)).toBeUndefined();
  });

  // The one label the labels endpoint cannot carry: the amount is part of the
  // value, not the type, so there is no static enum to serve.
  it("is worded as the CLI words it", () => {
    expect(fivePowerWords({ Copper: 2 })).toBe("2 Copper");
    expect(fivePowerWords({ ImportedGoods: 1 })).toBe("1 Import");
    expect(fivePowerWords({ Money: 3 })).toBe("3 Yen");
    expect(fivePowerWords({ Seafood: 3 })).toBe("3 Seafood");
  });
});

describe("the manifest's shapes (M5, W0)", () => {
  // The three achievement sheets and the building-site sheet store their cards
  // SIDEWAYS; the generator rotates them a quarter turn counter-clockwise
  // before emitting, so a landscape entry is the proof the rotation happened
  // and a portrait one is the old crop drawn on its side wherever it is drawn
  // unrotated (the sites drawer, the card index, the tile).
  it("achievements and sites are landscape; orders and techs are portrait", () => {
    const m = wire.artManifest;
    for (const [family, entries] of [
      ["achievements", m.achievements],
      ["sites", m.sites],
    ] as const) {
      for (const [key, e] of Object.entries(entries)) {
        expect(e.w, `${family}/${key} is ${e.w}x${e.h}, not landscape`).toBeGreaterThan(e.h);
      }
    }
    for (const [family, entries] of [
      ["orders", m.orders],
      ["techs", m.techs],
    ] as const) {
      for (const [key, e] of Object.entries(entries)) {
        expect(e.h, `${family}/${key} is ${e.w}x${e.h}, not portrait`).toBeGreaterThan(e.w);
      }
    }
  });

  // Source widths, so the fit rule's 22rem ceiling is the art's native width on
  // a 2x display rather than a 1.6x upsample of a 440px crop. Every entry, not
  // the first: one family emitted at two widths would read as current from its
  // first key alone.
  it("is cut at source width", () => {
    const m = wire.artManifest;
    for (const [family, entries, width] of [
      ["areas", m.areas, 709],
      ["orders", m.orders, 530],
      ["techs", m.techs, 530],
      ["sites", m.sites, 799],
      ["achievements", m.achievements, 799],
    ] as const) {
      for (const [key, e] of Object.entries(entries)) {
        expect(e.w, `${family}/${key} is cut at ${e.w}`).toBe(width);
      }
    }
  });
});
