// The runtime half of the wire contract: what a TYPE cannot check.
//
// `contract.ts` proves the client's declared fields are all PRESENT in the
// server's payloads. It cannot prove the reverse — TypeScript's excess-property
// check applies to object literals, and a JSON import is a value — so a field
// the server adds and the client never declares satisfies every assertion
// there. That is the direction three of M2's fields arrived in
// (`end_triggers`, `deck_remaining`, `end_game_triggered`), and nothing said so.
//
// So: the key lists in `contract.ts` are pinned to `keyof T` at compile time,
// and pinned to the payload's OWN keys here. Together they are an equality, and
// an equality fails whichever side moves.

import { describe, expect, it } from "vitest";
import { EVERY_STATE, KEYS, wire } from "./contract";
import { achievementsOf } from "../test/panels";
import type { ArtEntry, HandPanel, Panel } from "../types";

const keys = (v: unknown): string[] => Object.keys(v as object).sort();
const listed = (l: readonly string[]): string[] => [...l].sort();

describe("the payload roots carry exactly the fields the client declares", () => {
  it.each(
    EVERY_STATE.map((state) => [state.prompt.kind, state] as const),
  )("ClientState (%s)", (_kind, state) => {
    expect(keys(state)).toEqual(listed(KEYS.clientState));
  });

  it("FrameModel", () => {
    expect(keys(wire.options.frame)).toEqual(listed(KEYS.frame));
  });

  it("DecisionModel", () => {
    expect(keys(wire.options.frame.decision)).toEqual(listed(KEYS.decision));
  });

  it("OptionRow", () => {
    const prompt = wire.options.prompt;
    if (prompt.kind !== "options") throw new Error("the options fixture is not an options prompt");
    expect(prompt.rows.length).toBeGreaterThan(0);
    for (const row of prompt.rows) expect(keys(row)).toEqual(listed(KEYS.optionRow));
  });

  it("TileModel", () => {
    const tiles = wire.options.frame.board.tiles;
    expect(tiles.length).toBeGreaterThan(0);
    for (const tile of tiles) expect(keys(tile)).toEqual(listed(KEYS.tile));
  });

  it("PlayerModel", () => {
    const panel = wire.options.frame.dashboard.find((p): p is Extract<Panel, { Players: unknown }> =>
      "Players" in p,
    );
    expect(panel).toBeDefined();
    for (const player of panel!.Players.players) {
      expect(keys(player)).toEqual(listed(KEYS.player));
    }
  });

  it("TracksPanel", () => {
    const panel = wire.options.frame.dashboard.find((p): p is Extract<Panel, { Tracks: unknown }> =>
      "Tracks" in p,
    );
    expect(panel).toBeDefined();
    expect(keys(panel!.Tracks)).toEqual(listed(KEYS.tracks));
  });

  it.each([
    ["ports", wire.ports, KEYS.ports],
    ["labs", wire.labs, KEYS.labs],
    ["areas", wire.areas, KEYS.areas],
    ["log", wire.log, KEYS.log],
    ["labels", wire.labels, KEYS.labels],
    ["health", wire.health, KEYS.health],
    ["preview", wire.preview, KEYS.preview],
    ["error", wire.error, KEYS.apiError],
  ] as const)("%s", (_name, payload, list) => {
    expect(keys(payload)).toEqual(listed(list));
  });
});

// `Widen<T>` degrades a tuple to an unbounded array, because TypeScript types a
// JSON array as `T[]` and a mapped form that preserved `[number, number]` would
// reject every real fixture. So arity is checked HERE, where a value exists.
describe("the tuples the wire carries keep their arity", () => {
  it("grid_position is [row, col] and assistants are [seat, count]", () => {
    for (const tile of wire.options4p.frame.board.tiles) {
      expect(tile.grid_position).toHaveLength(2);
      for (const pair of tile.assistants) expect(pair).toHaveLength(2);
    }
  });

  it("achievement progress is [seat, current, needed]", () => {
    const cards = achievementsOf(wire.options);
    expect(cards.length).toBeGreaterThan(0);
    for (const card of cards) {
      for (const row of card.progress) expect(row).toHaveLength(3);
    }
  });

  it("a fee's recipients are [seat, yen]", () => {
    const prompt = wire.feeNonzero.prompt;
    if (prompt.kind !== "place_a") throw new Error("fixture moved");
    expect(prompt.fee!.to_players.length).toBeGreaterThan(0);
    for (const pair of prompt.fee!.to_players) expect(pair).toHaveLength(2);
  });
});

// `Widen<Record<string, X>>` is an index signature, and `{}` satisfies one. So
// the whole of `labels.json` could be emptied and every type-level check would
// stay green; these are what notice.
describe("the keyed payloads are not empty", () => {
  it("labels carries every static enum the wire names", () => {
    expect(Object.keys(wire.labels.area_types).length).toBeGreaterThanOrEqual(14);
    expect(Object.keys(wire.labels.tech_effects).length).toBeGreaterThanOrEqual(20);
    expect(Object.keys(wire.labels.achievements).length).toBeGreaterThanOrEqual(12);
    expect(wire.labels.phase_chain).toHaveLength(9);
    expect(Object.keys(wire.labels.phases)).toHaveLength(9);
  });

  it("the art manifest carries every family with something in it", () => {
    for (const [family, entries] of Object.entries(wire.artManifest)) {
      expect(Object.keys(entries).length, family).toBeGreaterThan(0);
    }
    // The committed pipeline emits 131 images; the manifest must reach them.
    const total = Object.values(wire.artManifest).reduce(
      (n, family) => n + Object.keys(family).length,
      0,
    );
    expect(total).toBeGreaterThanOrEqual(100);
  });
});

describe("what the corpus itself has to be true of", () => {
  // A payload built for a seat whose hand is masked must not carry the
  // identities anywhere. The mask is a property of the MODEL — `HandPanel.cards`
  // is empty when `hidden` — which is what makes a client that ignores the flag
  // still unable to leak. This asserts the property on the wire rather than
  // trusting the sentence.
  it("a masked hand carries a count and no identities", () => {
    const hand = wire.hiddenHand.frame.dashboard.find(
      (p): p is Extract<Panel, { Hand: HandPanel }> => "Hand" in p,
    );
    expect(hand).toBeDefined();
    expect(hand!.Hand.hidden).toBe(true);
    expect(hand!.Hand.cards).toEqual([]);
    // Truthful whether masked or not: the height of a hand is public at a table.
    expect(hand!.Hand.card_count).toBeGreaterThan(0);
    expect(JSON.stringify(wire.hiddenHand)).not.toContain('"is_starting_order"');
  });

  // The seed is what a rounded JSON number would silently corrupt, and the
  // setup flow round-trips it from preview to create. A STRING on the wire, and
  // `frame.title.seed` — the CLI model's `u64` — is the field not to read.
  it("the seed is a string, and the frame's number is the one not to read", () => {
    for (const state of EVERY_STATE) {
      expect(typeof state.seed).toBe("string");
      expect(state.seed).toMatch(/^\d+$/);
    }
    expect(typeof wire.preview.seed).toBe("string");
    expect(typeof wire.options.frame.title.seed).toBe("number");
  });

  // A payload that is not the actor's carries no answerable list, in BOTH
  // halves — the frame's decision zone and the prompt beside it. They were
  // built from different sources and disagreed once: the frame said
  // `options: []` while the prompt carried the actor's rows.
  it("a watching payload has no answerable list in either half", () => {
    expect(wire.watching.prompt.kind).toBe("watching");
    expect(wire.watching.frame.decision.options).toEqual([]);
    expect(wire.watching.frame.decision.viewer_is_deciding).toBe(false);
    expect(wire.watching.your_turn).toBe(false);
  });

  // A locked session accepts nothing, and says why. It used to report a live
  // `options` prompt with rows that could never be applied.
  it("a locked payload is locked rather than offering rows", () => {
    expect(wire.locked.prompt.kind).toBe("locked");
    if (wire.locked.prompt.kind !== "locked") return;
    expect(wire.locked.prompt.reason).not.toBe("");
  });

  it("a finished game reports scores and stops asking", () => {
    expect(wire.gameOver.over).toBe(true);
    expect(wire.gameOver.prompt.kind).toBe("game_over");
    expect(wire.gameOver.scores).toHaveLength(2);
  });

  // Every art key the manifest carries is a key `art.ts` looks up by, and the
  // client's own lookup returns `undefined` rather than a guessed filename for
  // anything absent. This pins the families themselves: a renamed family would
  // silently make every card in it art-less, which degrades to the typed
  // fallback and therefore never throws.
  it("the art manifest carries the families the client looks up", () => {
    expect(keys(wire.artManifest)).toEqual(
      ["achievements", "areas", "backs", "five_power", "mercenaries", "orders", "sites", "techs", "tokens"],
    );
    for (const family of Object.values(wire.artManifest)) {
      for (const entry of Object.values(family) as ArtEntry[]) {
        expect(entry.file).toMatch(/\.webp$/);
        expect(entry.w).toBeGreaterThan(0);
        expect(entry.h).toBeGreaterThan(0);
      }
    }
  });
});
