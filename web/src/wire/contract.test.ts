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
import type { ArtEntry, Country, FlagCounts, HandPanel, Panel, PlayerModel } from "../types";

const keys = (v: unknown): string[] => Object.keys(v as object).sort();
const listed = (l: readonly string[]): string[] => [...l].sort();

/** Every Hand panel and every seat's row ANYWHERE in the committed corpus —
 *  the play states, the replay stops, the run's frames, the deal views — found
 *  by walking each payload rather than by naming where a frame sits in it, so
 *  a payload added later is covered without a list to extend. */
function corpusPanels(): { hands: [string, HandPanel][]; players: [string, PlayerModel][] } {
  const hands: [string, HandPanel][] = [];
  const players: [string, PlayerModel][] = [];
  const walk = (name: string, v: unknown): void => {
    if (Array.isArray(v)) {
      for (const x of v) walk(name, x);
      return;
    }
    if (v === null || typeof v !== "object") return;
    for (const [k, x] of Object.entries(v)) {
      if (k === "Hand") hands.push([name, x as HandPanel]);
      if (k === "Players") {
        for (const p of (x as { players: PlayerModel[] }).players) players.push([name, p]);
      }
      walk(name, x);
    }
  };
  for (const [name, payload] of Object.entries(wire)) walk(name, payload);
  return { hands, players };
}

const FLAG_KEY: Record<Country, keyof FlagCounts> = {
  English: "english", American: "american", French: "french", German: "german", Dutch: "dutch",
};

describe("the payload roots carry exactly the fields the client declares", () => {
  it.each(
    EVERY_STATE.map(
      (state) =>
        [`${state.prompt.kind}${state.seat === null ? " · watcher" : ""}`, state] as const,
    ),
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

  // The Church chooser's arm (plan 2a): its kinds and its preview carry the
  // fields `types.ts` declares, on every `church` state in the corpus.
  it.each(
    EVERY_STATE.filter((state) => state.prompt.kind === "church").map(
      (state, i) => [i, state] as const,
    ),
  )("ChurchKind and ChooserPreview (church state %s)", (_i, state) => {
    const prompt = state.prompt;
    if (prompt.kind !== "church") throw new Error("filtered to church states");
    expect(prompt.kinds.length).toBe(6);
    for (const kind of prompt.kinds) expect(keys(kind)).toEqual(listed(KEYS.churchKind));
    expect(prompt.kinds.map((k) => k.number)).toEqual([1, 2, 3, 4, 5, 6]);
    expect(keys(prompt.preview)).toEqual(listed(KEYS.chooserPreview));
    // The arm's own top (the fix wave's C1).
    expect(keys(prompt)).toEqual(listed(KEYS.churchPrompt));
  });

  it("the corpus's three church states: as it opens, with its first kind on, and its first legal one", () => {
    const churches = EVERY_STATE.filter((state) => state.prompt.kind === "church");
    expect(churches).toEqual([wire.church, wire.churchSelected, wire.churchLegal]);
    const prompt = (state: typeof wire.church) => {
      if (state.prompt.kind !== "church") throw new Error(`not a church prompt: ${state.prompt.kind}`);
      return state.prompt;
    };
    const selected = (state: typeof wire.church): number => prompt(state).kinds.filter((k) => k.selected).length;
    expect(selected(wire.church)).toBe(0);
    expect(selected(wire.churchSelected)).toBe(1);
    expect(selected(wire.churchLegal)).toBe(2);
    // Neither of the first two names a listed move.
    for (const state of [wire.church, wire.churchSelected]) {
      expect(prompt(state).preview.legal).toBe(false);
      expect(prompt(state).preview.words).toBeNull();
    }
    // The third does, in `describe`'s words, and so carries no offered-faiths line.
    const legal = prompt(wire.churchLegal);
    expect(legal.preview.legal).toBe(true);
    expect(legal.preview.words).toMatch(/^Church: pay /);
    expect(legal.offered_faiths).toBeNull();
  });

  it("a church preview has words exactly when it is legal", () => {
    for (const state of EVERY_STATE) {
      const prompt = state.prompt;
      if (prompt.kind !== "church") continue;
      if (prompt.preview.legal) {
        expect(typeof prompt.preview.words).toBe("string");
        expect(prompt.preview.words).not.toBe("");
      } else {
        expect(prompt.preview.words).toBeNull();
      }
    }
  });

  // The Chinatown chooser's arm (plan 2a, Task 4): its nine rows and its
  // preview carry the fields `types.ts` declares, on every `chinatown` state.
  it.each(
    EVERY_STATE.filter((state) => state.prompt.kind === "chinatown").map(
      (state, i) => [i, state] as const,
    ),
  )("ChinatownRow and ChooserPreview (chinatown state %s)", (_i, state) => {
    const prompt = state.prompt;
    if (prompt.kind !== "chinatown") throw new Error("filtered to chinatown states");
    expect(prompt.rows.length).toBe(9);
    for (const row of prompt.rows) expect(keys(row)).toEqual(listed(KEYS.chinatownRow));
    expect(prompt.rows.map((r) => r.number)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9]);
    // The four sales are rows 1 to 4, and only they.
    expect(prompt.rows.map((r) => r.sells)).toEqual([true, true, true, true, false, false, false, false, false]);
    expect(prompt.stock.map((a) => a.good)).toEqual(["Copper", "Silk", "Tea", "Seafood", "Yen", "Imports"]);
    expect(keys(prompt.preview)).toEqual(listed(KEYS.chooserPreview));
    // The arm's own top, a stock entry, and every row's spend and gain (the
    // fix wave's C1).
    expect(keys(prompt)).toEqual(listed(KEYS.chinatownPrompt));
    for (const amount of prompt.stock) expect(keys(amount)).toEqual(listed(KEYS.chooserAmount));
    for (const row of prompt.rows) {
      expect(keys(row.spend), `row ${row.number}'s spend`).toEqual(listed(KEYS.chooserAmount));
      expect(keys(row.gain), `row ${row.number}'s gain`).toEqual(listed(KEYS.chooserAmount));
    }
  });

  // The Port chooser's arm (plan 2b, Task 2): its top, every card and its
  // preview carry the fields `types.ts` declares, on every `port` state.
  it.each(
    EVERY_STATE.filter((state) => state.prompt.kind === "port").map(
      (state, i) => [i, state] as const,
    ),
  )("PortCard and PortPreview (port state %s)", (_i, state) => {
    const prompt = state.prompt;
    if (prompt.kind !== "port") throw new Error("filtered to port states");
    expect(keys(prompt)).toEqual(listed(KEYS.portPrompt));
    expect(prompt.cards.length).toBeGreaterThan(0);
    for (const card of prompt.cards) {
      expect(keys(card)).toEqual(listed(KEYS.portCard));
      // The number a click sends is the card's bare id, and its face is the card's.
      expect(card.number).toBe(card.id);
      expect(card.printed.id).toBe(card.id);
    }
    expect(keys(prompt.preview)).toEqual(listed(KEYS.portPreview));
  });

  it("the corpus's three port states: as it opens, with one card on, and with a pair on", () => {
    const ports = EVERY_STATE.filter((state) => state.prompt.kind === "port");
    expect(ports).toEqual([wire.port, wire.portSelected, wire.portPair]);
    const prompt = (state: typeof wire.port) => {
      if (state.prompt.kind !== "port") throw new Error(`not a port prompt: ${state.prompt.kind}`);
      return state.prompt;
    };
    const on = (state: typeof wire.port) => prompt(state).cards.filter((c) => c.selected);
    // As it opens: nothing selected, not legal, no way; four cards in reach.
    const opened = prompt(wire.port);
    expect(opened.cards.length).toBe(4);
    expect(on(wire.port)).toEqual([]);
    expect(opened.preview).toEqual({ legal: false, ways: [] });
    expect(opened.cards.every((c) => c.can_add)).toBe(true);
    // One card on: legal, one way, in `describe`'s words.
    const one = prompt(wire.portSelected);
    expect(on(wire.portSelected).length).toBe(1);
    expect(one.preview.legal).toBe(true);
    expect(one.preview.ways).toEqual([`Port: take O${String(on(wire.portSelected)[0]!.number).padStart(2, "0")}`]);
    // A pair on: legal, one way, and every other card's can_add false.
    const pair = prompt(wire.portPair);
    expect(on(wire.portPair).length).toBe(2);
    expect(pair.preview.legal).toBe(true);
    expect(pair.preview.ways.length).toBe(1);
    expect(pair.preview.ways[0]).toMatch(/^Port: take O\d\d, O\d\d, paying /);
    expect(pair.cards.filter((c) => !c.selected).length).toBe(2);
    expect(pair.cards.every((c) => !c.can_add)).toBe(true);
    for (const state of ports) expect(prompt(state).max_cards).toBe(2);
  });

  // **A chooser's terminal screen never rides `messages`** (design §5.6; the
  // fix wave's C2). The server strips it by identity and `api.rs`'s cells
  // catch a Rust-side leak, but a REGENERATED corpus would freeze the leak as
  // faithfully as a fix (measured: vitest read 226 passed over a corpus
  // regenerated with the screen in `messages`). So the corpus's own chooser
  // states are read for the lines only a screen carries: each screen's
  // heading and its `> ` hint, in "The words" of plans 2a and 2b.
  const SCREEN_LINES: { what: string; test: (line: string) => boolean }[] = [
    { what: "the Church's heading", test: (l) => l === "Church — pay any of these, at most one of each (toggle by number):" },
    { what: "the Church's hint", test: (l) => l === "> type a number to toggle, 'confirm' to pay, or 'cancel'" },
    { what: "Chinatown's heading", test: (l) => /^Chinatown — up to \d+ exchanges? \(add by number\):$/.test(l) },
    { what: "Chinatown's hint", test: (l) => l === "> a number adds one, 'back <number>' takes one back, 'confirm', or 'cancel'" },
    { what: "the Port's heading", test: (l) => /^Port [AB] — take 1 card(, or 2)? \(toggle by number\):$/.test(l) },
    { what: "the Port's hint", test: (l) => l === "> a card number toggles it, 'confirm' to take, or 'cancel'" },
  ];
  it("no chooser state's messages carry a line of any chooser's terminal screen", () => {
    // The patterns read the screens' own lines (anti-vacuity).
    for (const line of [
      "Church — pay any of these, at most one of each (toggle by number):",
      "> type a number to toggle, 'confirm' to pay, or 'cancel'",
      "Chinatown — up to 3 exchanges (add by number):",
      "Chinatown — up to 1 exchange (add by number):",
      "> a number adds one, 'back <number>' takes one back, 'confirm', or 'cancel'",
      "Port A — take 1 card, or 2 (toggle by number):",
      "Port B — take 1 card (toggle by number):",
      "> a card number toggles it, 'confirm' to take, or 'cancel'",
    ]) {
      expect(SCREEN_LINES.some((s) => s.test(line)), line).toBe(true);
    }
    const choosers = EVERY_STATE.filter(
      (s) => s.prompt.kind === "church" || s.prompt.kind === "chinatown" || s.prompt.kind === "port",
    );
    expect(choosers.length, "the corpus's chooser states").toBe(8);
    for (const state of choosers) {
      for (const message of state.messages) {
        for (const line of message.split("\n")) {
          for (const s of SCREEN_LINES) {
            expect(s.test(line), `a ${state.prompt.kind} state's messages carry ${s.what}: ${JSON.stringify(line)}`).toBe(false);
          }
        }
      }
    }
  });

  it("the corpus's two chinatown states: as it opens, and after one copper sale", () => {
    const towns = EVERY_STATE.filter((state) => state.prompt.kind === "chinatown");
    expect(towns).toEqual([wire.chinatown, wire.chinatownSelected]);
    const prompt = (state: typeof wire.chinatown) => {
      if (state.prompt.kind !== "chinatown") throw new Error(`not a chinatown prompt: ${state.prompt.kind}`);
      return state.prompt;
    };
    // As it opens: nothing chosen, and so nothing named.
    const opened = prompt(wire.chinatown);
    expect(opened.chosen).toBe(0);
    expect(opened.rows.map((r) => r.count)).toEqual([0, 0, 0, 0, 0, 0, 0, 0, 0]);
    expect(opened.preview.legal).toBe(false);
    expect(opened.preview.words).toBeNull();
    // After one copper sale: a listed set, in `describe`'s words.
    const sold = prompt(wire.chinatownSelected);
    expect(sold.chosen).toBe(1);
    expect(sold.rows.map((r) => r.count)).toEqual([1, 0, 0, 0, 0, 0, 0, 0, 0]);
    expect(sold.preview.legal).toBe(true);
    expect(sold.preview.words).toBe("Chinatown: 1 copper -> 2 yen");
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

  // The hand root, over every state (the play states the loop below reads) and
  // then over every Hand panel the corpus holds anywhere.
  it("HandPanel", () => {
    let seen = 0;
    for (const state of EVERY_STATE) {
      for (const p of state.frame.dashboard) {
        if (!("Hand" in p)) continue;
        expect(keys(p.Hand), `${state.prompt.kind} · seat ${state.seat}`).toEqual(listed(KEYS.hand));
        seen += 1;
      }
    }
    // Anti-vacuity: every seated state carries its viewer's hand.
    expect(seen).toBe(EVERY_STATE.filter((s) => s.seat !== null).length);
    expect(seen).toBeGreaterThan(0);
    const { hands } = corpusPanels();
    expect(hands.length).toBeGreaterThan(seen);
    for (const [name, hand] of hands) expect(keys(hand), name).toEqual(listed(KEYS.hand));
  });

  // Issue #76. The in-hand count is a function of the panel's OWN cards and of
  // nothing else, which is what keeps it inside the existing mask: recounting
  // `cards[].printed.country` must give the count exactly, and a hidden panel
  // (none reaches a browser today) carries none.
  it("a hand's flag count is the countries of its own cards", () => {
    const { hands } = corpusPanels();
    let flagged = 0;
    for (const [name, hand] of hands) {
      if (hand.hidden) {
        expect(hand.flags_in_hand, `${name}: a hidden hand`).toBeNull();
        continue;
      }
      const want: FlagCounts = { english: 0, american: 0, french: 0, german: 0, dutch: 0 };
      for (const card of hand.cards) {
        const country = card.printed?.country;
        if (country !== undefined) want[FLAG_KEY[country]] += 1;
      }
      expect(hand.flags_in_hand, `${name}: seat ${hand.seat}`).toEqual(want);
      if (hand.cards.length > 0) flagged += 1;
    }
    expect(flagged, "hands with cards to count").toBeGreaterThan(0);
  });

  // The seat's row reaches every reader, a watcher included, so the in-hand
  // count must never ride it: the row carries the face-up count alone.
  it("no seat's row carries an in-hand count", () => {
    const { players } = corpusPanels();
    expect(players.length).toBeGreaterThan(0);
    for (const [name, p] of players) {
      expect(Object.keys(p), `${name}: seat ${p.seat}`).not.toContain("flags_in_hand");
      expect(Object.keys(p), `${name}: seat ${p.seat}`).toContain("flags");
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
    ["at_k", wire.atK, KEYS.historicalFrame],
    ["labels", wire.labels, KEYS.labels],
    ["health", wire.health, KEYS.health],
    ["me", wire.me, KEYS.me],
    ["me_signed_out", wire.meSignedOut, KEYS.me],
    // **M6.1's seven.** Each is a payload shape the client has to draw and
    // nothing else in the corpus reaches: a table at each of its two statuses,
    // the deal with the caller's cards and the deal without them, the lobby, and
    // the two refusals — the dealing 409 the client BRANCHES on, and the 403 a
    // caller with no seat here is handed.
    ["table_dealing", wire.tableDealing, KEYS.table],
    ["table", wire.table, KEYS.table],
    ["deal", wire.deal, KEYS.dealView],
    ["deal_waiting", wire.dealWaiting, KEYS.dealView],
    ["deal_bot_table", wire.dealBotTable, KEYS.dealView],
    ["games", wire.games, KEYS.gamesList],
    ["dealing_409", wire.dealing409, KEYS.dealingError],
    ["not_seated_403", wire.notSeated403, KEYS.apiError],
    // M6.2's eight. The table at its third status, read by somebody with no
    // seat at it; the lobby's other list and its paged form; and the four ways
    // a caller is turned away from a table it may not open or sit at.
    ["table_open", wire.tableOpen, KEYS.table],
    ["table_two_open", wire.tableTwoOpen, KEYS.table],
    ["games_open", wire.gamesOpen, KEYS.gamesList],
    ["games_next", wire.gamesNext, KEYS.gamesList],
    ["open_409", wire.open409, KEYS.dealingError],
    ["join_409_not_open", wire.join409NotOpen, KEYS.dealingError],
    ["join_403_locked", wire.join403Locked, KEYS.apiError],
    ["join_403_password", wire.join403Password, KEYS.apiError],
    ["join_409_taken", wire.join409Taken, KEYS.apiError],
    // M6.3's six that are not `ClientState`s (those ride the row above, which
    // runs over EVERY_STATE): the signal at the table's first status, a replay
    // stop read by a watcher, the third lobby list, the table the unlock face
    // is drawn from, and the two refusals a would-be watcher meets.
    ["events_open", wire.eventsOpen, KEYS.events],
    ["spectator_at_k", wire.spectatorAtK, KEYS.historicalFrame],
    ["games_playing", wire.gamesPlaying, KEYS.gamesList],
    ["table_watchable", wire.tableWatchable, KEYS.table],
    ["watch_403_locked", wire.watch403Locked, KEYS.apiError],
    ["unlock_403", wire.unlock403, KEYS.apiError],
    ["unlock_409_open", wire.unlock409Open, KEYS.dealingError],
    // And the two a SEATED caller meets by acting out of turn (spec §8): an
    // ordinary line from the seat that is not to move, and `undo` after that
    // seat's turn has passed. One root key each, which is the whole of the
    // shape — what these files are for is the two SENTENCES, asserted below.
    ["wrong_seat_403", wire.wrongSeat403, KEYS.apiError],
    ["undo_refused_403", wire.undoRefused403, KEYS.apiError],
    // M6.4's five refusal bodies. One root key each, as every refusal on this
    // API has — they are here for the same reason the six above were added
    // after a review: a compile-time list without a runtime half cannot see a
    // key the SERVER adds, and a refusal that grew one would be the one payload
    // in the corpus nothing was comparing.
    ["chat_400_long", wire.chat400Long, KEYS.apiError],
    ["chat_400_seated_name", wire.chat400SeatedName, KEYS.apiError],
    ["chat_403_not_seated", wire.chat403NotSeated, KEYS.apiError],
    ["chat_403_locked", wire.chat403Locked, KEYS.apiError],
    ["house_chat_off_404", wire.houseChatOff404, KEYS.apiError],
    // M6.5c2 cycle D (R-D4): the delete's one refusal.
    ["chat_delete_403", wire.chatDelete403, KEYS.apiError],
    // M6.5b's five archive refusals, each one root key, here for the same
    // reason M6.4's five are: the sentences are asserted below, and this is
    // what would see a key the server grew on one of them.
    ["archive_409", wire.archive409, KEYS.apiError],
    ["archive_400_rev", wire.archive400Rev, KEYS.apiError],
    ["archive_400_digest", wire.archive400Digest, KEYS.apiError],
    ["archive_400_replay", wire.archive400Replay, KEYS.apiError],
    ["archive_400_table", wire.archive400Table, KEYS.apiError],
    // The signal at both statuses. Two rows rather than one because the fields
    // are the same four at each, and a corpus carrying only the playing shape
    // would let a `dealing` reply grow or lose a key unnoticed.
    ["events", wire.events, KEYS.events],
    ["events_dealing", wire.eventsDealing, KEYS.events],
    ["house_frame", wire.houseFrame, KEYS.houseFrame],
    // Issue #75: the four-claim fixture's root and each of its four seat rows.
    // `corpusPanels()` does not reach these rows — the root names its panels
    // `players` and `achievements`, not the `Players` tag a frame's dashboard
    // carries — so the row's key set, `flags` in and `flags_in_hand` out, is
    // checked here.
    ["goals_claimed_4p", wire.goalsClaimed4p, KEYS.goalsClaimed],
    ["goals_claimed_4p.players.0", wire.goalsClaimed4p.players.players[0], KEYS.player],
    ["goals_claimed_4p.players.1", wire.goalsClaimed4p.players.players[1], KEYS.player],
    ["goals_claimed_4p.players.2", wire.goalsClaimed4p.players.players[2], KEYS.player],
    ["goals_claimed_4p.players.3", wire.goalsClaimed4p.players.players[3], KEYS.player],
    // The dealt CARD, reached through the one payload that still carries one:
    // a caller's own `deal`. It used to be reached through `preview.deals`,
    // which served every seat's at once and is retired (M6.1 §8).
    ["deal.dealCard", wire.deal.cards![0], KEYS.dealCard],
    ["error", wire.error, KEYS.apiError],
    ["suggest", wire.suggest, KEYS.suggestResponse],
    ["suggest.ok", wire.suggest.ok, KEYS.suggestOk],
    ["suggest.explanation", wire.suggest.ok!.explanation, KEYS.explanation],
    ["why", wire.why, KEYS.whyResponse],
    ["why.ok", wire.why.ok, KEYS.whyOk],
    ["why.provenance", wire.why.ok!.provenance, KEYS.provenance],
    ["why_refused", wire.whyRefused, KEYS.whyResponse],
    ["why_refused.refusal", wire.whyRefused.refusal, KEYS.refusalRow],
    ["why_withheld", wire.whyWithheld, KEYS.whyResponse],
    ["why_withheld.withheld", wire.whyWithheld.withheld, KEYS.whyWithheld],
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
    // M6.5c2: the one table whose bot is not null, that bot, a bot SEAT, and
    // the door's contract file.
    ["table_bot", wire.tableBot, KEYS.table],
    ["table_bot.bot", wire.tableBot.bot, KEYS.tableBot],
    ["watching.bots", wire.watching.bots[0], KEYS.seatBot],
    ["production_bot", wire.productionBot, KEYS.productionBot],
    // M6.5c2: the foreign-bot 409 and its pair, the row that does not run
    // here, and `why`'s thirteenth refusal.
    ["foreign_bot_409", wire.foreignBot409, KEYS.foreignBotError],
    ["foreign_bot_409.foreign_bot", wire.foreignBot409.foreign_bot, KEYS.foreignBot],
    ["table_foreign", wire.tableForeign, KEYS.table],
    ["table_foreign.bot", wire.tableForeign.bot, KEYS.tableBot],
    ["why_foreign", wire.whyForeign, KEYS.whyResponse],
    ["why_foreign.refusal", wire.whyForeign.refusal, KEYS.refusalRow],
  ] as const)("%s", (_name, payload, list) => {
    expect(keys(payload)).toEqual(listed(list));
  });

  // **The chat, and the one root in this corpus whose key set depends on a
  // field's VALUE** (M6.4). A seat author carries a seat and a watcher does not,
  // so a single list checked against both would accept either shape at either
  // arm — the author's own `kind` picks the list instead, and an author kind the
  // client has no list for fails loudly rather than being skipped. That is what
  // keeps `bot`, which is declared and has no fixture, from arriving unnoticed.
  it("ChatPage, ChatMessage and each author by its kind", () => {
    const byKind = { seat: KEYS.chatAuthorSeat, watcher: KEYS.chatAuthorWatcher } as const;
    for (const page of [wire.chat, wire.chatAfter, wire.houseChat, wire.chatDeleted]) {
      expect(keys(page)).toEqual(listed(KEYS.chatPage));
      for (const m of page.messages) {
        expect(keys(m)).toEqual(listed(KEYS.chatMessage));
        const kind = m.author.kind as keyof typeof byKind;
        expect(byKind[kind], `an author kind the client knows: ${m.author.kind}`).toBeDefined();
        expect(keys(m.author)).toEqual(listed(byKind[kind]));
      }
    }
    for (const m of [wire.chatPost, wire.chatPostWatcher]) {
      expect(keys(m)).toEqual(listed(KEYS.chatMessage));
      // The post's own reply is the message as its POSTER reads it (M6.5c2
      // cycle D, R-6.5c2.35), so its author is checked by kind too — the
      // reply is where an author re-gaining `user` would show first.
      expect(keys(m.author)).toEqual(listed(byKind[m.author.kind as keyof typeof byKind]));
      expect(m.author.kind !== "bot" && m.author.you, "a post's reply is the poster's own").toBe(true);
    }
  });

  // **A delete answers with nothing** (M6.5c2 cycle D, R-D4): an empty 204,
  // which the walk records — and every host reads — as `null`, at a table and
  // at the house alike. A body here would be a second statement of the
  // tombstone the next page read carries.
  it("a delete's answer is empty, at a table and at the house", () => {
    expect([wire.chatDelete, wire.houseChatDelete]).toEqual([null, null]);
    expect(wire.chatDelete403.error).toBe("only the message's author or the table's creator may delete it");
  });

  // **The tombstone, as the server emits it** (M6.5c2 cycle D, R-D4): the one
  // committed message whose `deleted` reads true — so the key set above is
  // checked against a tombstone too — with its text the empty string (never
  // null), its author and number kept, and the page's `rev` one past its
  // `last`, where every page before the delete held them equal.
  it("the page after the delete carries one tombstone, and a revision one past its count", () => {
    const [gone, ...kept] = wire.chatDeleted.messages;
    expect(gone).toMatchObject({ seq: 1, text: "", deleted: true, author: wire.chat.messages[0]!.author });
    expect(kept.every((m) => !m.deleted)).toBe(true);
    expect(wire.chatDeleted.rev - wire.chatDeleted.last).toBe(1);
    expect(wire.chat.rev - wire.chat.last, "before the delete").toBe(0);
    expect(wire.events.chat_rev - wire.events.chat, "the signal says the same").toBe(1);
  });

  // **Every HUMAN seat in the corpus carries exactly four keys, and no
  // address** (M6.5c2 cycle D, R-6.5c2.35). `Widen<TableView>` distributes
  // over the seat union, so a human seat that re-gained `user` would still
  // match its widened arm at compile time: only this can see it. The seats
  // are FOUND, by walking every typed fixture for a `seats` array, rather than
  // listed, so a table payload added later is covered without an edit here.
  it("every human seat in the corpus is a name, a you and an invited", () => {
    const humans: unknown[] = [];
    const walk = (v: unknown, key: string): void => {
      if (Array.isArray(v)) {
        for (const item of v) {
          if (key === "seats" && (item as { kind?: unknown } | null)?.kind === "human") humans.push(item);
          walk(item, "");
        }
      } else if (v !== null && typeof v === "object") {
        for (const [k, item] of Object.entries(v)) walk(item, k);
      }
    };
    walk(wire, "");
    // Anti-vacuity: the corpus's tables hold human seats, named and not.
    expect(humans.length).toBeGreaterThan(10);
    for (const seat of humans) expect(keys(seat)).toEqual(listed(KEYS.seatHuman));
    expect(humans.some((h) => (h as { name: unknown }).name === null)).toBe(true);
  });

  // **`why`'s thirteenth refusal is the LAYER's** (M6.5c2, review R2-m5): a
  // kind of its own, outside the CLI's twelve that `why_refusals.json` carries
  // whole — and the lobby row and the 409 name the same bot.
  it("the foreign-bot refusal is a kind of its own, and the row and the 409 agree", () => {
    expect(wire.whyForeign.refusal?.kind).toBe("foreign_bot");
    expect(wire.whyRefusals.map((r) => r.kind)).not.toContain("foreign_bot");
    expect(wire.tableForeign.bot).toEqual({ label: wire.foreignBot409.foreign_bot.played, runs_here: false });
    expect(wire.foreignBot409.foreign_bot.runs).toBe(wire.productionBot.label);
  });

  // Anti-vacuity for the widened replay frame: the position the corpus
  // captured (the game's midpoint) must hold a port that DIFFERS from the
  // opening offer in `ports.json`, or the two fields could be a copy of the
  // OPENING panels and nothing here would know. What it catches is that
  // narrower thing, not any copy of the live session — `ports.json` is the
  // opening capture, and the cell that sees a live copy is the route cell
  // `the_replay_position_carries_the_offers_at_k`.
  it("at_k carries the port offer of ITS position, not the opening's", () => {
    expect(wire.atK.ports).not.toEqual(wire.ports);
    expect(wire.atK.ports.boards[0]!.slots).toHaveLength(6);
    expect(wire.atK.labs.boards[0]!.slots).toHaveLength(6);
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
  // **R-1.7, asserted over the WHOLE corpus rather than over one fixture.**
  //
  // Every payload is built for exactly one seat (M6.1 §5.1): the only Hand
  // panel in it is that seat's, open, and every other seat carries
  // `hand_hidden` on the Players panel instead. Stated here as a property of
  // the corpus because that is the strongest form available on this side of the
  // wire — a server that unmasked a second seat in ANY of the twenty payloads
  // fails, where a cell over one fixture would only have covered one.
  //
  // It replaces a cell over `hidden_hand`, the `privacy`-toggle payload, which
  // is retired: the server no longer reads that flag, so no fixture carries a
  // masked Hand panel and the mask lives entirely on the Players rows.
  //
  // **A WATCHER is the same rule read at zero** (M6.3, R-3.7). `seat` is null,
  // so the seat this payload unmasks is no seat: there is no Hand panel at all,
  // every row on the Players panel is hidden, and the starting-order mark —
  // which appears only inside a viewer's own panel — appears nowhere. Written
  // as a branch of this loop rather than as a cell of its own, because it is
  // one property (`p.hand_hidden === (p.seat !== state.seat)` is already TRUE
  // at every seat when `state.seat` is null) and a separate cell would let a
  // spectator fixture be dropped from `EVERY_STATE` without anything saying so.
  it("every payload unmasks its own seat and no other", () => {
    for (const state of EVERY_STATE) {
      const hands = state.frame.dashboard.filter(
        (p): p is Extract<Panel, { Hand: HandPanel }> => "Hand" in p,
      );
      expect(hands.length, `${state.id}: hand panels for seat ${state.seat}`).toBe(
        state.seat === null ? 0 : 1,
      );
      if (state.seat !== null) {
        expect(hands[0]!.Hand.seat).toBe(state.seat);
        expect(hands[0]!.Hand.hidden).toBe(false);
      }

      const players = state.frame.dashboard.find(
        (p): p is Extract<Panel, { Players: unknown }> => "Players" in p,
      );
      expect(players).toBeDefined();
      for (const p of players!.Players.players) {
        expect(p.hand_hidden, `seat ${p.seat} of ${state.seat}'s payload`).toBe(
          p.seat !== state.seat,
        );
      }
      // The mask is a property of the MODEL, not of the renderer: a masked hand
      // has no `cards` to leak, so the one card-level secret — which order was
      // dealt as a starting order — appears only inside the viewer's own panel.
      // A watcher has no panel, so the count on both sides is zero.
      const marks = JSON.stringify(state).split('"is_starting_order"').length - 1;
      expect(marks, `${state.seat}'s payload`).toBe(hands[0]?.Hand.cards.length ?? 0);
    }
  });

  // **The watcher, stated once as its own claim** (M6.3, spec §6).
  //
  // The loop above proves the corpus-wide rule and this proves the fixtures are
  // really the shape the rule is being read at: a property that holds VACUOUSLY
  // over payloads that turned out to be ordinary seated ones would look exactly
  // the same from there. So this asserts the three watcher payloads are
  // watchers — `seat` null on the live frame, the finished one and a REPLAY
  // stop — and that a watcher's `cards` are nowhere at any seat.
  //
  // **`spectator_at_k` is the one that could not be covered above**, because
  // `EVERY_STATE` is `ClientState`s and a replay frame is not one. It is the
  // payload M4's sweep argument is about: a watcher walking `0..=total` harvests
  // nothing at any stop, and `seat: null` is how the wire says so.
  it("a watcher holds no seat, and no seat's hand is open to one", () => {
    expect(wire.spectator.seat).toBeNull();
    expect(wire.spectatorOver.seat).toBeNull();
    expect(wire.spectatorAtK.seat).toBeNull();
    // ...and the seated fixtures beside them are not, or the three above are
    // satisfied by a corpus in which nothing is ever built for a seat.
    expect(wire.options.seat).not.toBeNull();
    expect(wire.atK.seat).not.toBeNull();

    for (const [name, frame] of [
      ["spectator", wire.spectator.frame],
      ["spectator_over", wire.spectatorOver.frame],
      ["spectator_at_k", wire.spectatorAtK.frame],
    ] as const) {
      expect(frame.dashboard.filter((p) => "Hand" in p), `${name}: no hand panel`).toHaveLength(0);
      const players = frame.dashboard.find(
        (p): p is Extract<Panel, { Players: unknown }> => "Players" in p,
      );
      expect(players).toBeDefined();
      for (const p of players!.Players.players) {
        expect(p.hand_hidden, `${name}: seat ${p.seat}`).toBe(true);
      }
      // The identities, not just the flag: a hidden hand is a COUNT and no
      // cards, so the mark that names a dealt starting order is absent from the
      // whole payload rather than merely unrendered.
      expect(JSON.stringify(frame), name).not.toContain('"is_starting_order"');
    }

    // The two things a watcher's live frame says about the reader rather than
    // about the game: nobody's turn is theirs, and the prompt carries no
    // answerable list — `watching` while it runs, `game_over` once it is over.
    expect(wire.spectator.your_turn).toBe(false);
    expect(wire.spectator.prompt.kind).toBe("watching");
    expect(wire.spectatorOver.your_turn).toBe(false);
    expect(wire.spectatorOver.prompt.kind).toBe("game_over");
    expect(wire.spectatorOver.scores).not.toBeNull();
  });

  // **The room's own refusals, and the table they are drawn beside** (M6.3).
  //
  // Four sentences for four different things to do next, which is the same
  // reason the join refusals are committed: a watcher who is shut out of a
  // locked table can knock, a watcher who typed the password wrong can try
  // again, a caller at a table that has not started has to wait, and a caller
  // with no business at the table at all can do nothing. A server that answered
  // any two of these with one sentence would fail here.
  it("a would-be watcher is turned away in four distinguishable ways", () => {
    const said = [
      wire.watch403Locked.error,
      wire.unlock403.error,
      wire.unlock409Open.error,
      wire.notSeated403.error,
    ];
    expect(new Set(said).size, said.join(" | ")).toBe(4);
    expect(wire.watch403Locked.error).toContain("unlock");
    expect(wire.unlock403.error).toContain("password");
    expect(wire.unlock409Open.status).toBe("open");

    // The payload the unlock face is drawn from: a table this caller can SEE
    // and may not watch. Both flags, because each is the other's anti-vacuity —
    // a screen that asked for a password at an unlocked table, or one that drew
    // the game at a locked one, reads the same single field and gets it wrong
    // in opposite directions.
    expect(wire.tableWatchable.locked).toBe(true);
    expect(wire.tableWatchable.can_watch).toBe(false);
    expect(wire.tableWatchable.my_seats).toEqual([]);
    expect(wire.tableWatchable.status).toBe("playing");
    // ...against a row of the same type in the watcher's own lobby list, read
    // after the knock: same table, same lock, and the boolean turned over. It is
    // a fact about the READER and not about the table, and one payload alone
    // cannot show that.
    const row = wire.gamesPlaying.games[0]!;
    expect(row.id).toBe(wire.tableWatchable.id);
    expect(row.locked).toBe(true);
    expect(row.can_watch).toBe(true);
    expect(row.my_seats).toEqual([]);
  });

  // The signal at the table's FIRST status, which is the one screen that polls
  // before there is a game to poll: a joiner watching an open table's seats
  // fill. `to_move` is null there, and `status` is what the client branches on.
  it("the signal reads at open, at dealing and in play", () => {
    expect(wire.eventsOpen.status).toBe("open");
    expect(wire.eventsOpen.to_move).toBeNull();
    expect(wire.eventsOpen.events).toBe(0);
    expect(wire.eventsDealing.status).toBe("dealing");
    expect(wire.events.status).toBe("playing");
    expect(wire.events.events).toBeGreaterThan(0);
  });

  // **A seat only ever sees its own deal (2026-09-06, R-C1), and since M6.1 §8
  // that is the SHAPE of the payload rather than a filter over it.** The wire
  // used to carry one `Deal` per seat and the reader picked its own; there is
  // no per-seat array any more, so a caller's `cards` is either its own two or
  // null.
  //
  // **Both payloads, because each is the other's anti-vacuity.** `deal_waiting`
  // is the null — every seat this caller holds has chosen — so a server that
  // always sent two cards would fail there, and one that never sent any would
  // fail on `deal`. Either payload alone is satisfied by a field that is always
  // the same thing.
  it("a deal carries the caller's own two cards, or none at all", () => {
    expect(wire.deal.seat).not.toBeNull();
    expect(wire.deal.cards).toHaveLength(2);
    expect(JSON.stringify(wire.deal)).not.toContain("bot_keep");
    expect(wire.dealWaiting.seat).toBeNull();
    expect(wire.dealWaiting.cards).toBeNull();
  });

  // **The technologies and the goals are not on the set table (2026-09-06,
  // R-C3).** The set table is the `frame` beside a seat's two cards now that
  // `preview` is retired (M6.1 §8), and it carries the laboratory with every
  // slot emptied and no `Achievements` panel at all — because the owner's
  // edition deals the two starting orders and takes the secret choice at its
  // step 9, before the technologies reach the Laboratory at step 10 and the
  // achievements the score track at step 11.
  //
  // `deck_remaining` stays: how many cards are left is public arithmetic, and
  // it is the identities that have not come out yet.
  //
  // **Both deal fixtures, because they are different moments** — the payload a
  // seat chooses over, and the one it is handed after choosing — and the screen
  // draws the table in both.
  //
  // **`at_k` is the anti-vacuity and it is load-bearing.** A `HistoricalFrame`
  // whose labs were empty and whose dashboard had no Achievements panel BY TYPE
  // would satisfy every line above while proving nothing about the set table;
  // the replay position at k shows both, so what is asserted here is a property
  // of these payloads and not of their shape.
  it("the set table withholds the laboratory and the goals, and a replay position does not", () => {
    const hasGoals = (frame: { dashboard: Panel[] }) =>
      frame.dashboard.some((p) => "Achievements" in p);
    for (const [name, d] of [
      ["deal", wire.deal],
      ["deal_waiting", wire.dealWaiting],
    ] as const) {
      const p = d.frame;
      expect(p.labs.boards.length, `${name}: the laboratory board is still drawn`)
        .toBeGreaterThan(0);
      expect(p.labs.boards.every((b) => b.slots.length === 0), name).toBe(true);
      expect(p.labs.deck_remaining, `${name}: the deck count is public`).toBeGreaterThan(0);
      expect(hasGoals(p.frame), `${name}: no Achievements panel`).toBe(false);
      // R-C4: what STAYS. The port is on the table at step 9, and the tracks
      // are printed board rather than cards.
      expect(p.ports.boards.flatMap((b) => b.slots).filter((s) => s.card !== null).length)
        .toBe(6);
      expect(p.frame.dashboard.some((d) => "Tracks" in d), name).toBe(true);
    }
    expect(wire.atK.labs.boards.some((b) => b.slots.length > 0)).toBe(true);
    expect(hasGoals(wire.atK.frame)).toBe(true);
  });

  // **NO payload carries the setup seed, and the exemption is gone (M6.1 §8).**
  //
  // The seed regenerates both decks, the boxed pile, the board and every deal —
  // hidden-set member #1 in `game/observe.rs` — so a frame carrying it hands the
  // whole hidden set to whoever holds the frame. It used to ride on every
  // payload twice, as `ClientState.seed` and again as `frame.title.seed`, and
  // then on `preview` alone, which had to carry one because the setup flow
  // round-tripped the value back into `create`. The flow is create-then-deal
  // now: the seed is written into the table document on the server and no
  // client ever reads it back.
  //
  // The server has its own fence over the same corpus
  // (`no_committed_payload_carries_the_setup_seed`). This is the client's half
  // and is deliberately not the same check: that one searches for the VALUE,
  // this one asserts the SHAPE the client is allowed to see, so a payload that
  // carried a seed under some other key would fail there and a client that
  // started reading one would fail here.
  //
  // **Every committed fixture, not only the play states.** `EVERY_STATE` is the
  // `ClientState` corpus; the setup payloads are where a seed would now have to
  // reappear, so they are named beside it.
  it("no payload carries a seed at all", () => {
    for (const state of EVERY_STATE) {
      expect(JSON.stringify(state)).not.toContain('"seed"');
    }
    for (const [name, payload] of [
      ["table_dealing", wire.tableDealing],
      ["table", wire.table],
      ["deal", wire.deal],
      ["deal_waiting", wire.dealWaiting],
      ["games", wire.games],
      ["at_k", wire.atK],
    ] as const) {
      expect(JSON.stringify(payload), name).not.toContain('"seed"');
    }
  });

  // **The open table is the one payload whose `my_seats` is empty**, and the
  // only place an unfilled seat reaches the client at all.
  //
  // R-2.7 widens `GET …/table` to any signed-in caller while a table is open,
  // because a join link is an address and the person following one has to see
  // what they are joining. Every other status refuses a caller with no seat, so
  // `my_seats: []` is not a shape the client can meet anywhere else — and this
  // is the payload its "you are not sitting here yet" screen renders from.
  //
  // **Both seat kinds, because each is the other's anti-vacuity.** A `seats`
  // array of two open seats would satisfy "there is an open seat" while proving
  // nothing about the arm that carries a person, and a payload with no open
  // seat at all is just `table.json`. This one holds a NAMED human beside an
  // open seat, which is the row a lobby draws.
  it("an open table has an empty my_seats, a seat nobody holds, and a lock", () => {
    expect(wire.tableOpen.status).toBe("open");
    expect(wire.tableOpen.my_seats).toEqual([]);
    expect(wire.tableOpen.seats.map((s) => s.kind)).toEqual(["human", "open"]);
    const held = wire.tableOpen.seats[0]!;
    if (held.kind !== "human") throw new Error("fixture moved");
    // The display name the creator gave — and, since M6.5c2 cycle D
    // (R-6.5c2.35), nothing else about them: the reader is a stranger, so the
    // seat is not theirs and no address crosses.
    expect(held.name).toBe("Red");
    expect(held.you).toBe(false);
    expect(held.invited).toBeNull();
    expect(wire.tableOpen.mine).toBe(false);
    expect(wire.tableOpen.creator).toBe("Red");
    expect(JSON.stringify(wire.tableOpen)).not.toContain("@");
    // `locked` is the boolean that crosses in the password's place: the salt
    // and the hash stay on the server, and `wire_fixtures.rs` searches every
    // payload of the run for all three.
    expect(wire.tableOpen.locked).toBe(true);
    expect(JSON.stringify(wire.tableOpen)).not.toContain("salt");
    expect(JSON.stringify(wire.tableOpen)).not.toContain("hash");
  });

  // **The open lobby lists both kinds of table, and the paged one has a
  // cursor.** Two separate properties, and each is what its fixture is for.
  //
  // `games_open` is the stranger's list: every row is a table they could sit
  // at, one locked and one not — so a client that ignored `locked` would draw
  // both rows identically and this is what would say so. `games_next` is the
  // ONLY payload anywhere whose `next` is a string, so without it the paged
  // arm of `GamesList` is declared by the client and rendered by nothing.
  it("the open list carries tables the caller has no seat at, locked and not", () => {
    expect(wire.gamesOpen.games).toHaveLength(2);
    for (const row of wire.gamesOpen.games) {
      expect(row.status).toBe("open");
      expect(row.my_seats).toEqual([]);
    }
    expect(wire.gamesOpen.games.map((g) => g.locked)).toEqual([true, false]);
    expect(wire.gamesOpen.next).toBeNull();
  });

  it("a paged list names the row it ended at", () => {
    expect(wire.gamesNext.games).toHaveLength(1);
    // `<created_at>,<id>` — opaque to the client, which sends it back as
    // `before`. The timestamp is frozen by the harness; the id is the server's.
    expect(wire.gamesNext.next).toBe(`${wire.gamesNext.games[0]!.created_at},wire`);
  });

  // **The four refusals the room adds say four different things**, and that is
  // the only reason four of them are committed: three share a single root key,
  // so a key-set row alone would let them collapse into one sentence without a
  // check going red. This is the same cell as the three 403s above, for the
  // three ways a join is turned down and the two 409s that are not a join.
  it("the room's refusals are four different sentences, and the two 409s name a status", () => {
    const said = [
      wire.join403Locked.error,
      wire.join403Password.error,
      wire.join409Taken.error,
      wire.open409.error,
    ];
    expect(new Set(said).size).toBe(4);
    // Not "403": what to send. A person who has the password can act on this.
    expect(wire.join403Locked.error).toContain("password");
    // And a different sentence from the one above, because a wrong password is
    // a different thing to do about than a missing one.
    expect(wire.join403Password.error).toContain("wrong password");
    expect(wire.join409Taken.error).toMatch(/seat \d+ is taken/);
    // The two the client BRANCHES on rather than displays: each names the
    // status the table is actually at, so the client draws that screen.
    expect(wire.open409.status).toBe("open");
    expect(wire.join409NotOpen.status).toBe("dealing");
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

  // **The three 403s a person can be handed say three different things**, and
  // that is the only reason three of them are committed: the shape is one root
  // key, so a key-set row alone would let all three collapse into one sentence
  // without a single check going red.
  //
  // Each one answers a different question. Not seated here: this table is not
  // yours. Not your turn: whose it is, so you know whether to wait. `undo`
  // after your turn: the button you pressed, which is R-5.1's ordering — the
  // undo rule runs in FRONT of the acting-seat fence precisely so this caller
  // is not told "it is seat 1's turn" about a question they did not ask.
  it("the three refusals a caller can be handed are three different sentences", () => {
    const said = [
      wire.notSeated403.error,
      wire.wrongSeat403.error,
      wire.undoRefused403.error,
    ];
    expect(new Set(said).size).toBe(3);
    for (const sentence of said) expect(sentence.length).toBeGreaterThan(0);
    expect(wire.notSeated403.error).toContain("not seated");
    // Whose turn it is, named — a bare "403" leaves a player with nothing to
    // do next.
    expect(wire.wrongSeat403.error).toMatch(/seat \d+'s turn/);
    // By the seat's NAME, never its holder's address (M6.5c2 cycle D,
    // R-6.5c2.35): this sentence goes to every other seated caller.
    expect(wire.wrongSeat403.error).toBe("it is seat 1's turn (Blue)");
    expect(wire.undoRefused403.error).toContain("undo");
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
    expect(wire.why.ok!.forced).toBe(false);
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

describe("the archive replies (M6.5b, R-6.5b.6)", () => {
  // The compile-time list in `contract.ts` checks the client's own type against
  // itself; only this comparison can see a key the SERVER added — which is the
  // direction the six payloads named above arrived in.
  it.each([
    ["archive_post", wire.archivePost],
    ["archive_post_identical", wire.archivePostIdentical],
    ["archive_post_abandoned", wire.archivePostAbandoned],
  ] as const)("%s carries leaf, path and identical and nothing else", (_name, payload) => {
    expect(keys(payload)).toEqual(listed(KEYS.archiveReply));
  });

  it("the leaf is the server's reading of the record, and the three fixtures show both", () => {
    // The same walk's record sent whole and sent PARTIAL, under one id: the two
    // leaves differ, so a client that filed by what it claimed rather than by
    // what the server replayed would have two fixtures reading alike.
    expect(wire.archivePost.leaf).toBe("over");
    expect(wire.archivePostAbandoned.leaf).toBe("abandoned");
    expect(wire.archivePost.path).toContain("/rev7/over/");
    expect(wire.archivePostAbandoned.path).toContain("/rev7/abandoned/");
    // The caller prefix is the ENCODED address (R-6.5b.15) — no `@`, no slash
    // inside the segment — and the path never spells the address itself.
    expect(wire.archivePost.path.startsWith("browser/")).toBe(true);
    expect(wire.archivePost.path).not.toContain("@");
  });

  it("identical rides EVERY reply, false on the 201 and true on the retry", () => {
    // Both values on the same path: a retry answers 200 `identical` rather than
    // a 409, which is the answer the client deletes its local game on.
    expect(wire.archivePost.identical).toBe(false);
    expect(wire.archivePostIdentical.identical).toBe(true);
    expect(wire.archivePostIdentical.path).toBe(wire.archivePost.path);
  });
});

describe("the run payload (M6.5b, R-6.5b.10)", () => {
  it("is a list of HistoricalFrames and nothing else, three of them at consecutive stops", () => {
    expect(keys(wire.run)).toEqual(listed(KEYS.run));
    expect(wire.run.frames).toHaveLength(3);
    for (const frame of wire.run.frames) expect(keys(frame)).toEqual(listed(KEYS.historicalFrame));
    const indexes = wire.run.frames.map((f) => f.index);
    expect(indexes).toEqual([indexes[0], indexes[0]! + 1, indexes[0]! + 2]);
    // The run ends where `at_k` stands: its last frame is the stop BEFORE the
    // committed `at_k` position, so the two fixtures are one walk's neighbours.
    expect(indexes[2]).toBe(wire.atK.index - 1);
  });
});
