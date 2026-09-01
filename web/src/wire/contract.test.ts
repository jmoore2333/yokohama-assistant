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
    ["suggest", wire.suggest, KEYS.suggestResponse],
    ["suggest.ok", wire.suggest.ok, KEYS.suggestOk],
    ["suggest.explanation", wire.suggest.ok!.explanation, KEYS.explanation],
    ["why", wire.why, KEYS.whyResponse],
    ["why.ok", wire.why.ok, KEYS.whyOk],
    ["why.provenance", wire.why.ok!.provenance, KEYS.provenance],
    ["why_refused", wire.whyRefused, KEYS.whyResponse],
    ["why_refused.refusal", wire.whyRefused.refusal, KEYS.refusalRow],
    // **Six of these had a compile-time list and no runtime half**, which a
    // review found by adding `__server_added_a_field` to six committed payloads
    // and watching the typecheck and all 173 cells stay green. The compile-time
    // half checks the client's type against itself; only this comparison can
    // see a field the SERVER added — the exact direction three of M2's fields
    // arrived in.
    ["suggest.term", wire.suggest.ok!.explanation.terms[0], KEYS.term],
    ["suggest.ranked", wire.suggest.ok!.ranking[0], KEYS.ranked],
    ["why.counter", wire.why.ok!.counters![0], KEYS.counter],
    ["explanation_terminal", wire.explanationTerminal, KEYS.explanation],
    ["explanation_terminal.terminal", wire.explanationTerminal.terminal, KEYS.terminal],
    ["index_orders", wire.indexOrders, KEYS.index],
    ["index_orders.group", wire.indexOrders.groups[0], KEYS.indexGroup],
    ["index_orders.card", wire.indexOrders.groups[0]!.cards[0], KEYS.indexCard],
    ["result.placing", wire.result.standings[0], KEYS.placing],
    ["result.breakdown", wire.result.standings[0]!.breakdown, KEYS.breakdown],
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

  // **No play payload carries the setup seed; `preview` does, and must.**
  //
  // The seed regenerates both decks, the boxed pile, the board and every deal —
  // hidden-set member #1 in `game/observe.rs` — so a frame carrying it hands the
  // whole hidden set to whoever holds the frame. It used to ride on every
  // payload twice, as `ClientState.seed` and again as `frame.title.seed`.
  //
  // The server has its own fence over the same corpus
  // (`no_committed_payload_carries_the_setup_seed_except_the_preview`). This is
  // the client's half and is deliberately not the same check: that one searches
  // for the VALUE, this one asserts the SHAPE the client is allowed to see, so
  // a payload that carried a seed under some other key would fail there and a
  // client that started reading one would fail here.
  //
  // `preview` keeps it, and keeps it as a STRING: the setup flow round-trips
  // the value from preview into create, and a `u64` through a JS number rounds
  // silently — dealing a different game than the one whose cards were chosen.
  it("no play payload carries a seed, and preview's is a string", () => {
    for (const state of EVERY_STATE) {
      expect(JSON.stringify(state)).not.toContain('"seed"');
    }
    expect(typeof wire.preview.seed).toBe("string");
    expect(wire.preview.seed).toMatch(/^\d+$/);
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

// The explanation surface's own invariant. Not a shape check — a shape check
// would pass on a payload whose numbers do not add up, which is the one thing
// this surface cannot afford.
describe("the explanation foots", () => {
  const explanations = [
    ["suggest", wire.suggest.ok!.explanation],
    ["why", wire.why.ok!.explanation],
    // A FINISHED game's explanation. The identity is the same one, and it is
    // the half that only holds if `terminal.value` is included — which is why
    // this row matters more than the two above it.
    ["explanation_terminal", wire.explanationTerminal],
  ] as const;

  it.each(explanations)("%s: sum(product) + terminal === total", (_name, e) => {
    const sum = e.terms.reduce((acc, t) => acc + t.product, 0);
    expect(sum + (e.terminal ? e.terminal.value : 0)).toBe(e.total);
  });

  it.each(explanations)("%s: every feature is reported, zeros included", (_name, e) => {
    // The identity above holds only over ALL the terms, so a payload that
    // dropped the zero-product ones server-side would be unverifiable by the
    // reader. 34 is `Feature::COUNT`.
    expect(e.terms).toHaveLength(34);
    expect(new Set(e.terms.map((t) => t.feature)).size).toBe(34);
    expect(e.nonzero).toBe(e.terms.filter((t) => t.product !== 0).length);
  });

  it("a terminal explanation reports every weight and product as ZERO", () => {
    // Not an accident of this position: on a finished game the positional
    // vector does not govern, and reporting an interpolated weight there would
    // be a lie about what the evaluator used. The whole score is the standing.
    const e = wire.explanationTerminal;
    expect(e.terminal).not.toBeNull();
    expect(e.nonzero).toBe(0);
    for (const t of e.terms) {
      expect(t.weight).toBe(0);
      expect(t.product).toBe(0);
    }
    expect(e.total).toBe(e.terminal!.value);
    // And the raw values stay populated, so a terminal explanation is still
    // readable rather than a block of zeros.
    expect(e.terms.some((t) => t.viewer_value !== 0)).toBe(true);
  });

  it.each(explanations)("%s: product is weight x (you - best opponent)", (_name, e) => {
    // The client must never recompute this — but it must be able to trust it,
    // and this is where that trust is earned rather than assumed.
    //
    // The `+ 0` is not decoration. An `i64` has no signed zero and JavaScript
    // does: a zero weight times a negative difference is `-0` here, the wire
    // carries `0`, and `toBe` is `Object.is`, which tells them apart. Adding
    // zero normalises `-0` to `0` and changes nothing else. This is exactly the
    // class of difference this file exists to catch, so it is written down
    // rather than papered over with a looser matcher.
    for (const t of e.terms) {
      expect(t.product).toBe(t.weight * (t.viewer_value - t.opponent_value) + 0);
    }
  });

  it("the fixtures are not degenerate, because a degenerate one proves nothing", () => {
    // The first draft of both was a forced move with a one-row ranking, so the
    // runner-up table, the margin column and the multi-step continuation all
    // had no payload behind them. Asserted so a future reseed cannot quietly
    // reintroduce that.
    expect(wire.suggest.ok!.ranking.length).toBeGreaterThan(2);
    expect(wire.why.ok!.ranking.length).toBeGreaterThan(2);
    expect(wire.why.ok!.provenance.forced).toBe(false);
    expect(wire.suggest.ok!.continuation.length).toBeGreaterThan(1);
    expect(wire.why.ok!.explanation.nonzero).toBeGreaterThan(1);
  });

  it("the ranking is best first and the leader's margin against itself is zero", () => {
    for (const ok of [wire.suggest.ok!, wire.why.ok!]) {
      expect(ok.ranking[0]!.margin).toBe(0);
      const values = ok.ranking.map((r) => r.value);
      expect([...values].sort((a, b) => b - a)).toEqual(values);
      for (const r of ok.ranking) expect(r.margin).toBe(r.value - ok.ranking[0]!.value);
    }
  });
});

describe("every why refusal the client can be shown has a payload", () => {
  it("twelve of them, each tagged once and each carrying a sentence", () => {
    // The server side pins this against `explain::REFUSAL_KINDS`, which a
    // `match` with no `_` arm pins to the enum. This is the client's half: the
    // browser renders `message`, so a refusal with an empty one is a blank
    // screen where an explanation should be.
    expect(wire.whyRefusals).toHaveLength(12);
    const kinds = wire.whyRefusals.map((r) => r.kind);
    expect(new Set(kinds).size).toBe(12);
    for (const r of wire.whyRefusals) {
      expect(r.kind).toMatch(/^[a-z_]+$/);
      expect(r.message.length).toBeGreaterThan(20);
    }
  });

  it("a refused response carries a refusal and no account", () => {
    expect(wire.whyRefused.ok).toBeNull();
    expect(wire.whyRefused.refusal).not.toBeNull();
    expect(wire.whyRefused.refusal!.kind).toBe("no_bot_move_yet");
  });
});
