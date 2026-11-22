// The decision zone: what the player is asked, and what they may answer.
//
// Every state below comes from `src/wire/` — the server emitted it and
// `wire_fixtures.rs` refuses to let it drift. Nothing here builds a payload.

import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Decision } from "./Decision";
import { EVERY_STATE, wire } from "./wire/contract";
import type { ArtManifest, ClientState, PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

function show(
  state: ClientState,
  busy = false,
  spectating = false,
  art: ArtManifest | null = null,
) {
  const onSend = vi.fn();
  const { container } = render(
    <Decision
      state={state}
      labels={wire.labels}
      colors={COLORS}
      busy={busy}
      spectating={spectating}
      art={art}
      onSend={onSend}
    />,
  );
  return { onSend, user: userEvent.setup(), container };
}

describe("the numbered list", () => {
  // THE fact the whole client is built around. `prompt.rows` is the REPL's
  // GROUPED list; `frame.decision.options` is one entry per LISTED legal move
  // (the server's folded, capped list: the retreat family at its
  // recover-nothing member alone, the rest counted in `options_omitted`).
  // Numbering the second sends back a number naming a different move — and in
  // Movement it is usually still a LEGAL one, so the game diverges from what
  // the player clicked without anything failing.
  //
  // The fixture makes the size of that gap concrete: three rows against 186
  // legal moves.
  it("is the prompt's grouped rows, never the legal-move list", () => {
    const state = wire.optionsMany;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    expect(state.frame.decision.options.length).toBeGreaterThan(state.prompt.rows.length);
    show(state);
    const buttons = screen.getAllByRole("button").filter((b) => b.classList.contains("option"));
    expect(buttons).toHaveLength(state.prompt.rows.length);
  });

  it("sends the number the row carries", async () => {
    const state = wire.optionsMany;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    const row = state.prompt.rows[1]!;
    const { onSend, user } = show(state);
    await user.click(screen.getByRole("button", { name: new RegExp(escape(row.words)) }));
    expect(onSend).toHaveBeenCalledExactlyOnceWith(String(row.number));
  });

  it("offers nothing while a request is in flight", () => {
    const state = wire.optionsMany;
    show(state, true);
    for (const b of screen.getAllByRole("button")) expect(b).toBeDisabled();
  });
});

describe("a price on a row", () => {
  // `null` and a zero total are DIFFERENT and the terminal prints them
  // differently: a move that cannot carry a fee prints nothing, and a move that
  // is free prints `free`, because a player comparing destinations needs a
  // price and an absent price reads as an unknown one. The fixture carries one
  // of each.
  it("prints free for a zero fee and nothing for a move that has none", () => {
    const state = wire.optionsPriced;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    const priced = state.prompt.rows.find((r) => r.fee !== null);
    const unpriced = state.prompt.rows.find((r) => r.fee === null);
    expect(priced, "the fixture must carry a priced row").toBeDefined();
    expect(unpriced, "the fixture must carry an unpriced row").toBeDefined();
    expect(priced!.fee!.total).toBe(0);

    show(state);
    const pricedButton = screen.getByRole("button", { name: new RegExp(escape(priced!.words)) });
    expect(pricedButton).toHaveTextContent("free");
    const unpricedButton = screen.getByRole("button", {
      name: new RegExp(escape(unpriced!.words)),
    });
    expect(unpricedButton.querySelector(".price")).toBeNull();
  });

  // **A fee that COSTS something.** Every President move carries a fee object
  // whether it is free or not, so `options_priced` alone leaves the client's
  // `pay ¥N (n to Blue)` wording rendered by nothing — measured over a whole
  // random 2P game, the largest fee anywhere in the corpus was zero until the
  // fixture walk was steered at a tile a President stands on.
  it("prints what a costed placement actually costs, and to whom", () => {
    const state = wire.feeNonzero;
    if (state.prompt.kind !== "place_a") throw new Error("fixture moved");
    const fee = state.prompt.fee;
    expect(fee, "the fixture must carry a real fee").not.toBeNull();
    expect(fee!.total).toBeGreaterThan(0);
    show(state);
    const line = screen.getByText(/This placement is pay ¥/);
    expect(line).toHaveTextContent(`pay ¥${fee!.total}`);
    for (const [seat, amount] of fee!.to_players) {
      expect(line).toHaveTextContent(`${amount} to ${COLORS[seat]}`);
    }
    // Priced placements are called out, not printed like a free one.
    expect(line).toHaveClass("warn");
  });
});

describe("the guided retreat", () => {
  // **The defect, exactly.** Once every area is answered, `area` and `max` are
  // FALLBACK values — `(AreaId::new(0), 0)` — so a client that draws
  // `0..=max` buttons unconditionally draws a single meaningless `0`, which
  // looks answerable and is not. The only live answer there is confirm.
  it("offers no counts once every area is answered", () => {
    const state = wire.retreatAnswered;
    if (state.prompt.kind !== "retreat") throw new Error("fixture moved");
    expect(state.prompt.remaining).toBe(0);
    show(state);
    expect(screen.getByText(/every area is answered/i)).toBeInTheDocument();
    const names = screen.getAllByRole("button").map((b) => b.textContent);
    expect(names).not.toContain("0");
    expect(names).toContain("Confirm");
  });

  it("offers one count per legal answer while areas remain", () => {
    const state = wire.retreat;
    if (state.prompt.kind !== "retreat") throw new Error("fixture moved");
    expect(state.prompt.remaining).toBeGreaterThan(0);
    const { onSend } = show(state);
    const counts = screen
      .getAllByRole("button")
      .filter((b) => /^\d+$/.test(b.textContent ?? ""));
    // `0` is a legal answer meaning "recover none", so the count is max + 1.
    expect(counts).toHaveLength(state.prompt.max + 1);
    expect(onSend).not.toHaveBeenCalled();
  });

  // Scoped to the client's OWN sentence rather than to the area label alone:
  // the server keeps a sub-prompt's terminal block in `messages` (it carries
  // the live legality verdict, which has no structured equivalent yet), so the
  // label appears twice on the page and a bare text query cannot tell the
  // client's rendering from the text it passed through.
  it("names the area it is asking about, in the engine's own words", () => {
    const state = wire.retreat;
    if (state.prompt.kind !== "retreat") throw new Error("fixture moved");
    show(state);
    expect(
      screen.getByText(
        new RegExp(`Recover how many assistants from ${escape(state.prompt.area_label)}`),
      ),
    ).toBeInTheDocument();
  });
});

describe("a placement picker", () => {
  // **The other defect.** A player pressed Confirm and paid a yen no surface
  // had stated. The fee of the CURRENT selection is on the wire and the
  // terminal recomputes it on every redraw; the browser showed only the ids.
  it("states the price of the current selection, and only once there is one", () => {
    const empty = wire.placeA;
    if (empty.prompt.kind !== "place_a") throw new Error("fixture moved");
    expect(empty.prompt.selected).toEqual([]);
    const { onSend } = show(empty);
    expect(screen.queryByText(/This placement is/)).toBeNull();
    expect(onSend).not.toHaveBeenCalled();
  });

  it("states the price once an area is selected", () => {
    const selected = wire.placeASelected;
    if (selected.prompt.kind !== "place_a") throw new Error("fixture moved");
    expect(selected.prompt.selected.length).toBeGreaterThan(0);
    show(selected);
    expect(screen.getByText(/This placement is/)).toBeInTheDocument();
  });

  it("names the selected areas as the ids the player clicked", () => {
    const selected = wire.placeASelected;
    if (selected.prompt.kind !== "place_a") throw new Error("fixture moved");
    show(selected);
    // The client's own line, not the sub-prompt text the server passes through.
    const line = screen.getByText(/click up to \d+ areas on the board/);
    for (const area of selected.prompt.selected) {
      expect(line).toHaveTextContent(`A${String(area).padStart(2, "0")}`);
    }
  });

  it("Option B states its price once its one area is chosen", () => {
    const chosen = wire.placeBSelected;
    if (chosen.prompt.kind !== "place_b") throw new Error("fixture moved");
    expect(chosen.prompt.selected).not.toBeNull();
    show(chosen);
    expect(screen.getByText(/This placement is/)).toBeInTheDocument();
  });
});

describe("the prompts that answer nothing", () => {
  // A locked session accepts NOTHING this client can send: the three lines
  // `Repl::step` still takes under lockdown are all refused by the API's
  // grammar fence. So the screen must not offer a control that cannot work —
  // it used to report a live `options` prompt with rows behind it.
  it("a locked game says so and offers no answer", () => {
    const state = wire.locked;
    if (state.prompt.kind !== "locked") throw new Error("fixture moved");
    show(state);
    expect(screen.getByText(/LOCKED after a driver or engine fault/)).toBeInTheDocument();
    expect(screen.getByText(new RegExp(escape(state.prompt.reason)))).toBeInTheDocument();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  // A payload built for a seat that is not the actor carries no answerable
  // list, and the screen says whose turn it is rather than offering one.
  it("a watched game names the seat deciding and offers no answer", () => {
    const state = wire.watching;
    if (state.prompt.kind !== "watching") throw new Error("fixture moved");
    show(state);
    expect(screen.getByText(/Nothing here is yours to answer/)).toBeInTheDocument();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  // ---- the SPECTATOR, who holds no seat at all (spec §5.2) ---------------
  //
  // **`seat: null` is the whole selector**, and it is a different fact from
  // "it is not your turn": a watcher never gets a turn, so nothing here is ever
  // theirs to answer and the screen must not carry a control that would be a
  // 403. The words are the `watching` arm's own, unchanged — a watcher and a
  // player waiting on somebody else are looking at the same table.
  it("a watcher is offered nothing and reads the seat that is deciding", () => {
    const state = wire.spectator;
    if (state.prompt.kind !== "watching") throw new Error("fixture moved");
    expect(state.seat).toBeNull();
    show(state, false, true);
    // `:390`'s line, which is the spectator's asking line (spec §5.2).
    expect(screen.getByText(`Seat ${state.prompt.to_move} is deciding.`)).toBeInTheDocument();
    expect(screen.getByText(/Nothing here is yours to answer/)).toBeInTheDocument();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
    expect(screen.queryByRole("button", { name: "Undo" })).toBeNull();
  });

  // **The fence is real and not merely unreached.** A watcher's payload carries
  // `watching` today, so the cell above would pass over a `Decision` that had
  // never heard of spectating. Handed a payload WITH rows — the same one a
  // player answers — the spectator arm must still offer nothing, which is the
  // assertion that a fence rather than a fixture is doing the work.
  it("offers no answer to a watcher even over a payload that carries rows", () => {
    const state = wire.optionsMany;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    expect(state.prompt.rows.length).toBeGreaterThan(1);
    // The same payload, first as a player: the rows and Undo are both there.
    const player = show(state);
    expect(player.container.querySelectorAll("button.option").length).toBe(
      state.prompt.rows.length,
    );
    expect(screen.getByRole("button", { name: "Undo" })).toBeInTheDocument();
    cleanup();
    // And then as a watcher.
    const watcher = show(state, false, true);
    expect(watcher.container.querySelectorAll("button.option")).toHaveLength(0);
    expect(screen.queryByRole("button", { name: "Undo" })).toBeNull();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  // **The finished game is the one thing a watcher still SEES** (spec §5.2:
  // the result works as it does for a player). Suppressing every block under
  // `spectating` would leave a watcher at a game that had ended with the words
  // "The game is over." and no standing — the one screen they came for.
  it("still shows a watcher the standing when the game is over", () => {
    const state = wire.spectatorOver;
    expect(state.seat).toBeNull();
    show(state, false, true);
    expect(screen.getByText("The game is over.")).toBeInTheDocument();
    for (const row of state.scores ?? []) {
      expect(screen.getByText(`${row.total} VP`)).toBeInTheDocument();
    }
  });

  it("a finished game lists the scores instead of a move", () => {
    const state = wire.gameOver;
    show(state);
    expect(screen.getByText("The game is over.")).toBeInTheDocument();
    for (const row of state.scores ?? []) {
      expect(screen.getByText(`${row.total} VP`)).toBeInTheDocument();
    }
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });
});

describe("what the game is asking", () => {
  // A queued choice OUTRANKS the phase — `Deciding::Choice` is what
  // `screen_model` reports whenever `pending_choices` is non-empty. The client
  // switched only on `prompt.kind`, which is `"options"` for every queued
  // choice, so a player picking a good was told "Area Action" and nothing named
  // the actual question.
  it("a queued choice is worded as the choice, not as the phase", () => {
    const state = wire.pick;
    const deciding = state.frame.decision.deciding;
    expect(typeof deciding === "object" && "Choice" in deciding).toBe(true);
    show(state);
    expect(screen.getByText(/You are deciding/)).toBeInTheDocument();
  });

  // Every queued-choice head the corpus carries renders WORDS, not the raw
  // enum key. `choiceWords` falls through to the key for an unknown head, which
  // is the right fallback and a silent one — nothing throws, and the player is
  // shown `WarehouseTake`.
  it.each([
    ["choiceAssistantMove", wire.choiceAssistantMove, "AssistantMove"],
    ["choiceWarehouseTake", wire.choiceWarehouseTake, "WarehouseTake"],
    ["choiceChurchSpace", wire.choiceChurchSpace, "ChurchSpace"],
  ] as const)("%s is worded, not keyed", (_name, state, head) => {
    const deciding = state.frame.decision.deciding;
    if (typeof deciding === "string" || !("Choice" in deciding)) throw new Error("fixture moved");
    const actual = deciding.Choice.head;
    expect(typeof actual === "string" ? actual : Object.keys(actual)[0]).toBe(head);
    show(state);
    const asking = screen.getByText(/You are deciding/);
    expect(asking).not.toHaveTextContent(head);
    expect(asking.textContent!.length).toBeGreaterThan(40);
  });

  // The applied-move lines left the deck for the side column (M5 §3.5); a deck
  // that still rendered them would take the board's height budget back.
  it("renders none of the applied-move lines itself", () => {
    const state = wire.refused;
    expect(state.messages.length).toBeGreaterThan(0);
    const { container } = show(state);
    for (const m of state.messages) {
      expect(container.textContent).not.toContain(m);
    }
  });
});

/** Escape a payload string for use inside a `RegExp`. The words come from the
 *  engine's describer and carry `(`, `)` and `—`. */
function escape(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// ---- M3.5: parity rows, and a button that could only ever refuse ----

describe("what the decision zone names", () => {
  // The terminal prints `TURN 1 · BLUE's turn`. A bare seat number is the one
  // thing on this screen a player has to translate, and at 3P or 4P one operator
  // drives several seats.
  it("names the seat by colour, not by number", () => {
    show(wire.options);
    const chain = document.querySelector(".chain")!.textContent ?? "";
    expect(chain).toMatch(/Red|Blue|Yellow|Green/);
    expect(chain, "a bare seat number is what this replaced").not.toMatch(/seat \d/);
  });

  // `decision_render.rs:352` resolves `area_name(board, row.area)`; the client
  // printed `A07` and left the player to find it on the board — which is already
  // on the frame, so this was a lookup rather than a payload change.
  it("names a not-offered area, not just its id", () => {
    // **Constructed, and the reason is recorded.** No committed fixture reaches
    // a near-miss row — verified over all 41 — so a cell that only rendered a
    // fixture would take an early return and prove nothing, which is the exact
    // "clause that silently never ran" shape this suite has been burned by
    // twice. The SHAPE still comes from the wire; one field is set.
    const withNearMiss = wire.options;
    const target = withNearMiss.frame.board.tiles.find((t) => t.area_type !== "Canal")!;
    const state = {
      ...withNearMiss,
      frame: {
        ...withNearMiss.frame,
        decision: {
          ...withNearMiss.frame.decision,
          not_offered: [{ area: target.area, reason: "NoAssistantThere" as const }],
          not_offered_suppressed: 0,
        },
      },
    };
    show(state);
    const row = document.querySelector(".near-miss")!.textContent ?? "";
    const name = wire.labels.area_types[target.area_type] ?? target.area_type;
    expect(row, "the near-miss row still prints only an id").toContain(name);
    // And it keeps the id, which is this project's canonical identification key.
    expect(row).toContain(`A${String(target.area).padStart(2, "0")}`);
  });
});

describe("Confirm is offered only where it is answerable", () => {
  // `repl::step_pick` has arms for cancel / a digit / more / list and a
  // catch-all answering "! finish or cancel this prompt first". There is no
  // `confirm` arm at all, so the button could only ever refuse — and the refusal
  // would then be announced by the live region added in the same milestone.
  it("is absent on a pick", () => {
    show(wire.pick);
    expect(screen.queryByRole("button", { name: "Confirm" })).toBeNull();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
  });

  // `step_retreat` gates it: `confirming` is `collected_len >= areas_len`.
  it("is absent on a retreat with areas still to answer, and present once they are", () => {
    if (wire.retreat.prompt.kind !== "retreat") throw new Error("fixture moved");
    expect(wire.retreat.prompt.remaining, "the fixture must have areas left").toBeGreaterThan(0);
    show(wire.retreat);
    expect(screen.queryByRole("button", { name: "Confirm" })).toBeNull();

    if (wire.retreatAnswered.prompt.kind !== "retreat") throw new Error("fixture moved");
    expect(wire.retreatAnswered.prompt.remaining, "the answered fixture must be done").toBe(0);
    cleanup();
    show(wire.retreatAnswered);
    expect(screen.getByRole("button", { name: "Confirm" })).toBeInTheDocument();
  });
});

describe("the Church's chooser", () => {
  // Three real replies, each a whole `ClientState` captured from a played game
  // (`wire/church*.json`). What each IS is asserted before a cell relies on it.
  const churchPrompt = (s: ClientState) => {
    if (s.prompt.kind !== "church") throw new Error("fixture moved");
    return s.prompt;
  };
  const kindButtons = (container: HTMLElement) =>
    [...container.querySelectorAll<HTMLButtonElement>(".chooser.church .kinds button.kind")];
  const clone = (s: ClientState): ClientState => JSON.parse(JSON.stringify(s)) as ClientState;
  const button = (name: string) => screen.getByRole("button", { name });

  it("draws six kind buttons in the payload's order, each with its words and what the seat has", () => {
    const p = churchPrompt(wire.church);
    expect(p.kinds).toHaveLength(6);
    const { container } = show(wire.church);
    const buttons = kindButtons(container);
    expect(buttons).toHaveLength(6);
    buttons.forEach((b, i) => {
      const k = p.kinds[i]!;
      expect(b.getAttribute("data-good")).toBe(k.good);
      expect(b.querySelector(".lbl")!.firstElementChild!.textContent).toBe(k.words);
      expect(b.querySelector(".lbl .have")!.textContent).toBe(`you have ${k.have}`);
      // Said once: the button names itself, so the token inside is not read again.
      expect(b).toHaveAccessibleName(`${k.words}, you have ${k.have}`);
    });
    // The yen kind is the one that costs two, and its token group says so.
    expect(p.kinds[4]!.amount).toBe(2);
    expect(buttons[4]!.querySelector("[role=img]")).toHaveAccessibleName("2 yen");
  });

  it("reads a kind's words from the payload, never rebuilt from its good and amount", () => {
    const changed = clone(wire.church);
    if (changed.prompt.kind !== "church") throw new Error("fixture moved");
    changed.prompt.kinds[4]!.words = "a pair of yen";
    const { container } = show(changed);
    const yen = kindButtons(container)[4]!;
    expect(yen.querySelector(".lbl")!.firstElementChild!.textContent).toBe("a pair of yen");
    expect(yen).toHaveAccessibleName(`a pair of yen, you have ${changed.prompt.kinds[4]!.have}`);
  });

  it("sends a kind's number and nothing else, and marks only the selected kind pressed", async () => {
    const p = churchPrompt(wire.churchSelected);
    expect(p.kinds.map((k) => k.selected)).toEqual([true, false, false, false, false, false]);
    const { container, onSend, user } = show(wire.churchSelected);
    expect(kindButtons(container).map((b) => b.getAttribute("aria-pressed"))).toEqual([
      "true", "false", "false", "false", "false", "false",
    ]);
    // Silk and the yen kind: their numbers (2 and 5) are neither their index
    // (1 and 4) nor anything their good's name could send.
    await user.click(kindButtons(container)[1]!);
    await user.click(kindButtons(container)[4]!);
    expect(onSend.mock.calls).toEqual([["2"], ["5"]]);
  });

  it("disables a kind the list does not offer, and says why it is unavailable", async () => {
    const p = churchPrompt(wire.church);
    expect(p.kinds.map((k) => k.offered)).toEqual([true, true, true, true, true, false]);
    const { container, onSend, user } = show(wire.church);
    const imports = kindButtons(container)[5]!;
    expect(imports).toBeDisabled();
    expect(imports).toHaveAttribute("data-unavailable");
    await user.click(imports);
    expect(onSend).not.toHaveBeenCalled();
    for (const b of kindButtons(container).slice(0, 5)) {
      expect(b).toBeEnabled();
      expect(b).not.toHaveAttribute("data-unavailable");
    }
  });

  it("marks a kind unavailable by the payload's `offered`, not by what the seat holds", async () => {
    // On the real fixture the one kind not offered is also the one the seat has
    // none of, so a rule worked from `have` would pass the reading above.
    const held = clone(wire.church);
    if (held.prompt.kind !== "church") throw new Error("fixture moved");
    const silk = held.prompt.kinds[1]!;
    expect(silk.good).toBe("Silk");
    expect(silk.have, "the seat holds silk").toBeGreaterThan(0);
    silk.offered = false;
    const { container, onSend, user } = show(held);
    const button = kindButtons(container)[1]!;
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("data-unavailable");
    await user.click(button);
    expect(onSend).not.toHaveBeenCalled();
  });

  // **In flight, an offered kind is `aria-disabled`, never `disabled`** (ruling
  // R45): a `disabled` button lets the browser drop its focus, so the kind a
  // keyboard player just pressed would leave them at the top of the page. It
  // sends nothing while busy, and `disabled` stays what the payload says.
  it("leaves an offered kind enabled but aria-disabled while a request is in flight, and a press then sends nothing", async () => {
    const { container, onSend, user } = show(wire.church, true);
    const buttons = kindButtons(container);
    for (const b of buttons.slice(0, 5)) {
      expect(b).toBeEnabled();
      expect(b).toHaveAttribute("aria-disabled", "true");
      expect(b).not.toHaveAttribute("data-unavailable");
      await user.click(b);
    }
    expect(onSend).not.toHaveBeenCalled();
    // The kind the list does not offer is still disabled, by the payload.
    expect(buttons[5]).toBeDisabled();
    expect(buttons[5]).toHaveAttribute("data-unavailable");
    cleanup();

    // Idle, the offered kinds carry no `aria-disabled` at all.
    const idle = kindButtons(show(wire.church).container);
    for (const b of idle.slice(0, 5)) expect(b).not.toHaveAttribute("aria-disabled");
  });

  it("enables Confirm only where the preview is legal, and sends confirm and cancel", async () => {
    const notLegal = churchPrompt(wire.churchSelected);
    expect(notLegal.kinds.some((k) => k.selected), "a kind is selected").toBe(true);
    expect(notLegal.preview.legal, "and the preview is not legal").toBe(false);
    const first = show(wire.churchSelected);
    expect(button("Confirm")).toBeDisabled();
    expect(button("Confirm")).toHaveAttribute("data-unavailable");
    await first.user.click(button("Confirm"));
    expect(first.onSend).not.toHaveBeenCalled();
    // Cancel is never unavailable, where Confirm is.
    expect(button("Cancel")).toBeEnabled();
    expect(button("Cancel")).not.toHaveAttribute("data-unavailable");
    cleanup();

    // The two fixtures differ in more than legality (how many kinds are
    // selected, whether the faith line is there). One field apart: the legal
    // fixture made not legal, two kinds still selected and no faith line.
    const unlegal = clone(wire.churchLegal);
    if (unlegal.prompt.kind !== "church") throw new Error("fixture moved");
    expect(unlegal.prompt.kinds.filter((k) => k.selected)).toHaveLength(2);
    expect(unlegal.prompt.offered_faiths).toBeNull();
    unlegal.prompt.preview = { legal: false, words: null };
    show(unlegal);
    expect(button("Confirm")).toBeDisabled();
    expect(button("Confirm")).toHaveAttribute("data-unavailable");
    cleanup();

    expect(churchPrompt(wire.churchLegal).preview.legal).toBe(true);
    const { onSend, user } = show(wire.churchLegal);
    expect(button("Confirm")).toBeEnabled();
    expect(button("Confirm")).not.toHaveAttribute("data-unavailable");
    await user.click(button("Confirm"));
    await user.click(button("Cancel"));
    expect(onSend.mock.calls).toEqual([["confirm"], ["cancel"]]);
    expect(button("Cancel")).not.toHaveAttribute("data-unavailable");
    cleanup();

    // In flight: disabled, but nothing the payload said made it unavailable.
    show(wire.churchLegal, true);
    expect(button("Confirm")).toBeDisabled();
    expect(button("Confirm")).not.toHaveAttribute("data-unavailable");
    expect(button("Cancel")).toBeDisabled();
    expect(button("Cancel")).not.toHaveAttribute("data-unavailable");
  });

  it("reads the verdict from the preview, and the faiths on their own line", () => {
    const legal = churchPrompt(wire.churchLegal);
    expect(legal.preview.words).toBe("Church: pay copper, silk -> faith 4 (power 2 + 2 kinds)");
    const first = show(wire.churchLegal);
    const verdict = first.container.querySelector(".verdict")!;
    expect(verdict).toHaveAttribute("role", "status");
    expect(verdict).toHaveClass("ok");
    expect(verdict).not.toHaveClass("bad");
    expect(verdict.textContent).toBe(`legal — ${legal.preview.words}`);
    cleanup();

    const notOffered = churchPrompt(wire.churchSelected);
    expect(notOffered.offered_faiths).toBe("the list offers faith 4 to 7 here");
    const second = show(wire.churchSelected);
    const v2 = second.container.querySelector(".verdict")!;
    expect(v2).toHaveClass("bad");
    expect(v2).not.toHaveClass("ok");
    const lines = [...v2.children].filter((c) => c.tagName === "DIV");
    expect(lines.map((l) => l.textContent)).toEqual([notOffered.offered_faiths]);
    // The first line is everything but that second one.
    const head = v2.cloneNode(true) as HTMLElement;
    head.querySelector("div")!.remove();
    expect(head.textContent).toBe("not offered — adjust the selection");
    cleanup();

    const bare = clone(wire.churchSelected);
    if (bare.prompt.kind !== "church") throw new Error("fixture moved");
    bare.prompt.offered_faiths = null;
    const third = show(bare);
    const v3 = third.container.querySelector(".verdict")!;
    expect(v3.textContent).toBe("not offered — adjust the selection");
    expect(v3.querySelector("div")).toBeNull();
    cleanup();

    // Nothing selected is still a verdict at the Church, never the empty form
    // Chinatown draws at `chosen` 0.
    expect(churchPrompt(wire.church).kinds.some((k) => k.selected)).toBe(false);
    const idle = show(wire.church);
    const v4 = idle.container.querySelector(".verdict")!;
    expect(v4).toHaveClass("bad");
    expect(v4.querySelector(".faiths")!.textContent).toBe(churchPrompt(wire.church).offered_faiths);
  });

  it("draws neither a kind button nor Confirm for a watcher", () => {
    const { container } = show(wire.churchLegal, false, true);
    expect(kindButtons(container)).toHaveLength(0);
    expect(container.querySelector(".chooser")).toBeNull();
    expect(screen.queryByRole("button", { name: "Confirm" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();
  });

  it("asks its own question and draws no row of the numbered list", () => {
    const { container } = show(wire.church);
    expect(container.querySelector(".asking")!.textContent).toBe("The Church — what do you pay?");
    expect(container.querySelectorAll("button.option")).toHaveLength(0);
    expect(container.querySelector(".decision > .chooser.church")).not.toBeNull();
  });

  it("hands the art manifest to its tokens", () => {
    const { container } = show(wire.church, false, false, wire.artManifest);
    for (const b of kindButtons(container)) expect(b.querySelector(".tokens img")).not.toBeNull();
    cleanup();
    const plain = show(wire.church);
    for (const b of kindButtons(plain.container)) expect(b.querySelector(".tokens img")).toBeNull();
  });
});

describe("Chinatown's chooser", () => {
  // Two real replies from a four-player table (`wire/chinatown*.json`). What
  // each IS is asserted before a cell relies on it. A clone changes ONE fact,
  // and each exists to tell a payload read from a rule worked out in the client.
  const chinaPrompt = (s: ClientState) => {
    if (s.prompt.kind !== "chinatown") throw new Error("fixture moved");
    return s.prompt;
  };
  const clone = (s: ClientState): ClientState => JSON.parse(JSON.stringify(s)) as ClientState;
  const cloneChina = (s: ClientState, edit: (p: ReturnType<typeof chinaPrompt>) => void) => {
    const c = clone(s);
    edit(chinaPrompt(c));
    return c;
  };
  const rowEls = (container: HTMLElement) =>
    [...container.querySelectorAll<HTMLElement>(".chooser.chinatown .ex")];
  const plus = (row: HTMLElement) => row.querySelector<HTMLButtonElement>("button.plus")!;
  const minus = (row: HTMLElement) => row.querySelector<HTMLButtonElement>("button.minus")!;
  const button = (name: string) => screen.getByRole("button", { name });

  /** Which heading each row stands under, read off the DOM in document order. */
  const grouped = (container: HTMLElement) => {
    const out: Record<string, number[]> = {};
    let heading = "";
    for (const el of container.querySelectorAll<HTMLElement>(".chooser.chinatown .side-h, .chooser.chinatown .ex")) {
      if (el.classList.contains("side-h")) {
        heading = el.textContent ?? "";
        out[heading] = [];
      } else {
        out[heading]!.push(Number(el.dataset.number));
      }
    }
    return out;
  };

  it("draws nine rows, sales under Sell and purchases under Buy, in the payload's words", () => {
    const p = chinaPrompt(wire.chinatown);
    expect(p.rows).toHaveLength(9);
    expect(p.rows.map((r) => r.sells)).toEqual([true, true, true, true, false, false, false, false, false]);
    const { container } = show(wire.chinatown);
    const els = rowEls(container);
    expect(els).toHaveLength(9);
    expect(grouped(container)).toEqual({ Sell: [1, 2, 3, 4], Buy: [5, 6, 7, 8, 9] });
    els.forEach((el, i) => {
      expect(el.dataset.number).toBe(String(p.rows[i]!.number));
      expect(el.querySelector(".what .words")!.textContent).toBe(p.rows[i]!.words);
    });

    // Said once: the pictures and the arrow sit inside ONE hidden wrapper and
    // the words after it are the row's reading. What a reader meets as an image
    // is the six stock entries and nothing in a row.
    for (const el of els) {
      const pics = el.querySelector(".what > [aria-hidden='true']")!;
      expect(pics.querySelectorAll(".tokens")).toHaveLength(2);
      expect(pics.querySelector(".arrow")).not.toBeNull();
      expect(pics.contains(el.querySelector(".words"))).toBe(false);
      expect(el.querySelector(".words")!.closest("[aria-hidden='true']")).toBeNull();
    }
    expect(screen.getAllByRole("img").map((n) => n.getAttribute("aria-label"))).toEqual([
      "1 copper", "1 silk", "1 tea", "1 seafood", "3 yen", "0 imported goods",
    ]);
    cleanup();

    // The split reads the flag, not the position: with the rows reversed the
    // four sales (now last in the payload) are still under Sell.
    const reversed = cloneChina(wire.chinatown, (c) => c.rows.reverse());
    const flipped = show(reversed);
    expect(chinaPrompt(reversed).rows.map((r) => r.number)).toEqual([9, 8, 7, 6, 5, 4, 3, 2, 1]);
    expect(grouped(flipped.container)).toEqual({ Sell: [4, 3, 2, 1], Buy: [9, 8, 7, 6, 5] });
    cleanup();

    // The words are the payload's. ONE fact apart: row 1's words changed and
    // nothing else, so its gain token still reads what the amount says.
    const reworded = cloneChina(wire.chinatown, (c) => {
      c.rows[0]!.words = "one copper for two yen";
    });
    const edited = show(reworded);
    const first = rowEls(edited.container)[0]!;
    expect(first.querySelector(".words")!.textContent).toBe("one copper for two yen");
    expect(plus(first)).toHaveAccessibleName("one more: one copper for two yen");
    expect(minus(first)).toHaveAccessibleName("one fewer: one copper for two yen");
    const [spend, gain] = [...first.querySelectorAll(".what .tokens")];
    expect(spend!.getAttribute("aria-label")).toBe("1 copper");
    expect(gain!.getAttribute("aria-label")).toBe("2 yen");
    cleanup();

    // And the amount is the payload's. ONE fact apart again: row 1's gain
    // amount set to 3 with its words untouched (still `... -> 2 yen`), so an
    // amount parsed out of the words would read 2.
    const regained = cloneChina(wire.chinatown, (c) => {
      c.rows[0]!.gain.amount = 3;
    });
    expect(chinaPrompt(regained).rows[0]!.words).toBe("1 copper -> 2 yen");
    const counted = show(regained);
    const row = rowEls(counted.container)[0]!;
    expect(row.querySelector(".words")!.textContent).toBe("1 copper -> 2 yen");
    const [, three] = [...row.querySelectorAll(".what .tokens")];
    expect(three!.getAttribute("aria-label")).toBe("3 yen");
    expect(three!.querySelectorAll(".tok")).toHaveLength(3);
    cleanup();

    // One row's flag flipped, nothing else: row 5 (a purchase, gaining copper)
    // now sells, so it sits under Sell; a split made by what a row gains or by
    // its place keeps it under Buy.
    const flippedRow = cloneChina(wire.chinatown, (c) => {
      c.rows[4]!.sells = true;
    });
    const flag = show(flippedRow);
    expect(grouped(flag.container)).toEqual({ Sell: [1, 2, 3, 4, 5], Buy: [6, 7, 8, 9] });
  });

  it("sends a plus's number and a minus's `back` and number, and marks the chosen row", async () => {
    const none = chinaPrompt(wire.chinatown);
    expect(none.rows.map((r) => r.count)).toEqual([0, 0, 0, 0, 0, 0, 0, 0, 0]);
    const first = show(wire.chinatown);
    for (const el of rowEls(first.container)) {
      expect(minus(el)).toBeDisabled();
      expect(el).not.toHaveClass("on");
      expect(el.querySelector(".n")!.textContent).toBe("0");
    }
    // Rows 2 and 6: their numbers are neither their index nor a count.
    await first.user.click(plus(rowEls(first.container)[1]!));
    await first.user.click(plus(rowEls(first.container)[5]!));
    expect(first.onSend.mock.calls).toEqual([["2"], ["6"]]);
    cleanup();

    // The rows reversed: row 8 is now the second purchase drawn, so its place
    // is neither 8 nor its index. The number it sends is the row's own.
    const reversed = cloneChina(wire.chinatown, (c) => c.rows.reverse());
    const flipped = show(reversed);
    const eight = rowEls(flipped.container).find((el) => el.dataset.number === "8")!;
    await flipped.user.click(plus(eight));
    expect(flipped.onSend.mock.calls).toEqual([["8"]]);
    cleanup();

    // The same for a minus: the selected fixture reversed draws row 1, the one
    // with a count, last. Its minus sends `back 1`, not `back 9`.
    const turned = cloneChina(wire.chinatownSelected, (c) => c.rows.reverse());
    const back = show(turned);
    const one = rowEls(back.container).find((el) => el.dataset.number === "1")!;
    expect(rowEls(back.container).indexOf(one), "row 1 is drawn last of its heading").toBe(3);
    await back.user.click(minus(one));
    expect(back.onSend.mock.calls).toEqual([["back 1"]]);
    cleanup();

    // A count that is not the row's number: row 6 chosen twice. Its minus
    // sends `back 6`, not `back 2` and not `back 5`.
    const twice = cloneChina(wire.chinatownSelected, (c) => {
      c.rows[5]!.count = 2;
    });
    const counted = show(twice);
    const six = rowEls(counted.container).find((el) => el.dataset.number === "6")!;
    expect(six.querySelector(".n")!.textContent).toBe("2");
    expect(minus(six)).toBeEnabled();
    await counted.user.click(minus(six));
    expect(counted.onSend.mock.calls).toEqual([["back 6"]]);
    cleanup();

    const some = chinaPrompt(wire.chinatownSelected);
    expect(some.rows.map((r) => r.count)).toEqual([1, 0, 0, 0, 0, 0, 0, 0, 0]);
    const { container, onSend, user } = show(wire.chinatownSelected);
    const els = rowEls(container);
    expect(els.map((el) => el.classList.contains("on"))).toEqual([true, false, false, false, false, false, false, false, false]);
    expect(els.map((el) => !minus(el).disabled)).toEqual([true, false, false, false, false, false, false, false, false]);
    expect(els.map((el) => el.querySelector(".n")!.textContent)).toEqual(["1", "0", "0", "0", "0", "0", "0", "0", "0"]);
    // A minus at zero is unavailable by the payload; one with something to
    // take back is neither unavailable nor disabled.
    expect(minus(els[0]!)).not.toHaveAttribute("data-unavailable");
    for (const el of els.slice(1)) expect(minus(el)).toHaveAttribute("data-unavailable");
    await user.click(minus(els[0]!));
    await user.click(minus(els[3]!));
    expect(onSend.mock.calls).toEqual([["back 1"]]);
    cleanup();

    // In flight (ruling R45): the chosen row's minus stays enabled, so it keeps
    // the focus, but is `aria-disabled` and sends nothing; nothing the payload
    // said made it unavailable. A minus at zero is still disabled by the payload.
    const busy = show(wire.chinatownSelected, true);
    const b = rowEls(busy.container);
    expect(minus(b[0]!)).toBeEnabled();
    expect(minus(b[0]!)).toHaveAttribute("aria-disabled", "true");
    expect(minus(b[0]!)).not.toHaveAttribute("data-unavailable");
    await busy.user.click(minus(b[0]!));
    expect(busy.onSend).not.toHaveBeenCalled();
    expect(minus(b[1]!)).toBeDisabled();
    expect(minus(b[1]!)).toHaveAttribute("data-unavailable");
    // Idle, a live minus carries no `aria-disabled`.
    cleanup();
    expect(minus(rowEls(show(wire.chinatownSelected).container)[0]!)).not.toHaveAttribute("aria-disabled");
  });

  it("disables a plus the list cannot take, by the payload's `can_add`", async () => {
    const p = chinaPrompt(wire.chinatown);
    expect(p.rows.map((r) => r.can_add)).toEqual([true, true, true, true, true, true, true, true, false]);
    expect(p.rows[8]!.offered, "the purchase is offered though it cannot be added").toBe(true);
    const first = show(wire.chinatown);
    const els = rowEls(first.container);
    expect(plus(els[8]!)).toBeDisabled();
    expect(plus(els[8]!)).toHaveAttribute("data-unavailable");
    expect(plus(els[8]!)).toHaveAccessibleName("one more: 4 yen -> 1 imported good");
    await first.user.click(plus(els[8]!));
    expect(first.onSend).not.toHaveBeenCalled();
    for (const el of els.slice(0, 8)) {
      expect(plus(el)).toBeEnabled();
      expect(plus(el)).not.toHaveAttribute("data-unavailable");
    }
    cleanup();

    // With the copper sale chosen, the one copper is spent: row 1 cannot take
    // another. The stock still reads 1 copper and 3 yen, so a rule worked from
    // the stock and the spend fails BOTH readings below: row 1 (1 copper held,
    // 1 to spend) would be addable, and row 9 (3 yen held, 4 to spend) not.
    const chosen = chinaPrompt(wire.chinatownSelected);
    expect(chosen.rows[0]!.count).toBe(1);
    expect(chosen.rows[0]!.can_add).toBe(false);
    expect(chosen.stock[0]).toEqual({ good: "Copper", amount: 1 });
    expect(chosen.rows[8]!.can_add).toBe(true);
    expect(chosen.stock[4]).toEqual({ good: "Yen", amount: 3 });
    expect(chosen.rows[8]!.spend.amount).toBe(4);
    const second = show(wire.chinatownSelected);
    const sel = rowEls(second.container);
    expect(plus(sel[0]!)).toBeDisabled();
    expect(plus(sel[0]!)).toHaveAttribute("data-unavailable");
    expect(plus(sel[8]!)).toBeEnabled();
    expect(plus(sel[8]!)).not.toHaveAttribute("data-unavailable");
    await second.user.click(plus(sel[0]!));
    await second.user.click(plus(sel[8]!));
    expect(second.onSend.mock.calls).toEqual([["9"]]);
    cleanup();

    // `chosen` at the most the list allows, nothing else moved: row 1's plus is
    // still live, because plus reads the payload's `can_add` and nothing more.
    const atMost = cloneChina(wire.chinatown, (c) => {
      c.chosen = c.max_exchanges;
    });
    const full = show(atMost);
    const top = rowEls(full.container)[0]!;
    expect(chinaPrompt(atMost).rows[0]!.can_add).toBe(true);
    expect(plus(top)).toBeEnabled();
    expect(plus(top)).not.toHaveAttribute("data-unavailable");
    cleanup();

    // In flight (ruling R45): a plus that can add stays enabled, so it keeps
    // the focus, but is `aria-disabled`, carries no `data-unavailable` and sends
    // nothing; the cannot-add one is still disabled by the payload.
    const busy = show(wire.chinatown, true);
    const b = rowEls(busy.container);
    for (const el of b.slice(0, 8)) {
      expect(plus(el)).toBeEnabled();
      expect(plus(el)).toHaveAttribute("aria-disabled", "true");
      expect(plus(el)).not.toHaveAttribute("data-unavailable");
      await busy.user.click(plus(el));
    }
    expect(busy.onSend).not.toHaveBeenCalled();
    expect(plus(b[8]!)).toBeDisabled();
    expect(plus(b[8]!)).toHaveAttribute("data-unavailable");
    // Idle, a live plus carries no `aria-disabled`.
    cleanup();
    expect(plus(rowEls(show(wire.chinatown).container)[0]!)).not.toHaveAttribute("aria-disabled");
  });

  it("reads the stock from the payload, one image node per good", () => {
    const p = chinaPrompt(wire.chinatown);
    expect(p.stock.map((s) => s.amount)).toEqual([1, 1, 1, 1, 3, 0]);
    const { container } = show(wire.chinatown);
    const entries = [...container.querySelectorAll<HTMLElement>(".chooser.chinatown .stock > *")];
    expect(entries).toHaveLength(6);
    expect(entries.map((e) => e.getAttribute("role"))).toEqual(Array(6).fill("img"));
    expect(entries.map((e) => e.getAttribute("aria-label"))).toEqual([
      "1 copper", "1 silk", "1 tea", "1 seafood", "3 yen", "0 imported goods",
    ]);
    expect(entries.map((e) => e.getAttribute("data-good"))).toEqual(p.stock.map((s) => s.good));
    expect(entries.map((e) => e.querySelector(".amt")!.textContent)).toEqual(["1", "1", "1", "1", "3", "0"]);
    // ONE small token, never `amount` copies (three yen draw one).
    // That token is decorative: no role and no label of its own, so nothing
    // hidden in an entry of 3 yen says `1 yen`.
    for (const e of entries) {
      expect(e.querySelectorAll(".tok")).toHaveLength(1);
      expect(e.querySelector(".tok")).toHaveClass("sm");
      const token = e.querySelector(".tokens")!;
      expect(token).toHaveAttribute("aria-hidden", "true");
      expect(token).not.toHaveAttribute("role");
      expect(token).not.toHaveAttribute("aria-label");
    }
    cleanup();

    // Four of the fixture's own amounts are 1; these six differ, so an entry
    // that took its amount from anywhere else reads wrong.
    const counted = cloneChina(wire.chinatown, (c) => {
      c.stock.forEach((s, i) => (s.amount = i + 1));
    });
    const edited = show(counted);
    const read = [...edited.container.querySelectorAll<HTMLElement>(".chooser.chinatown .stock > *")];
    expect(read.map((e) => e.querySelector(".amt")!.textContent)).toEqual(["1", "2", "3", "4", "5", "6"]);
    expect(read.map((e) => e.getAttribute("aria-label"))).toEqual([
      "1 copper", "2 silk", "3 tea", "4 seafood", "5 yen", "6 imported goods",
    ]);
    cleanup();

    // And the payload's order is the order drawn.
    const reversed = cloneChina(counted, (c) => c.stock.reverse());
    const flipped = show(reversed);
    const back = [...flipped.container.querySelectorAll<HTMLElement>(".chooser.chinatown .stock > *")];
    expect(back.map((e) => e.getAttribute("data-good"))).toEqual(["Imports", "Yen", "Seafood", "Tea", "Silk", "Copper"]);
    expect(back.map((e) => e.querySelector(".amt")!.textContent)).toEqual(["6", "5", "4", "3", "2", "1"]);
  });

  it("enables Confirm only where the preview is legal, and reads the verdict from it", async () => {
    const empty = chinaPrompt(wire.chinatown);
    expect(empty.chosen).toBe(0);
    expect(empty.preview).toEqual({ legal: false, words: null });
    const first = show(wire.chinatown);
    expect(button("Confirm")).toBeDisabled();
    expect(button("Confirm")).toHaveAttribute("data-unavailable");
    expect(button("Cancel")).toBeEnabled();
    expect(button("Cancel")).not.toHaveAttribute("data-unavailable");
    const quiet = first.container.querySelector(".chooser.chinatown .verdict")!;
    expect(quiet).toHaveAttribute("role", "status");
    expect(quiet.textContent).toBe("");
    cleanup();

    const legal = chinaPrompt(wire.chinatownSelected);
    expect(legal.preview).toEqual({ legal: true, words: "Chinatown: 1 copper -> 2 yen" });
    const second = show(wire.chinatownSelected);
    expect(button("Confirm")).toBeEnabled();
    expect(button("Confirm")).not.toHaveAttribute("data-unavailable");
    const verdict = second.container.querySelector(".chooser.chinatown .verdict")!;
    expect(verdict).toHaveAttribute("role", "status");
    expect(verdict).toHaveClass("ok");
    expect(verdict.textContent).toBe("legal — Chinatown: 1 copper -> 2 yen");
    // Confirm and Cancel stay outside the box that scrolls.
    expect(button("Confirm").closest(".rows")).toBeNull();
    expect(button("Confirm").closest(".chooser")).toBeNull();
    await second.user.click(button("Confirm"));
    await second.user.click(button("Cancel"));
    expect(second.onSend.mock.calls).toEqual([["confirm"], ["cancel"]]);
    cleanup();

    // In flight: disabled, but nothing the payload said made it unavailable.
    show(wire.chinatownSelected, true);
    expect(button("Confirm")).toBeDisabled();
    expect(button("Confirm")).not.toHaveAttribute("data-unavailable");
    expect(button("Cancel")).toBeDisabled();
    expect(button("Cancel")).not.toHaveAttribute("data-unavailable");
    cleanup();

    // `chosen` set apart from the rows' counts: the first fixture with
    // `chosen` 1 and every count still 0. The verdict reads `chosen`.
    const claimed = cloneChina(wire.chinatown, (c) => {
      c.chosen = 1;
    });
    expect(chinaPrompt(claimed).rows.every((r) => r.count === 0)).toBe(true);
    const fourth = show(claimed);
    const loud = fourth.container.querySelector(".chooser.chinatown .verdict")!;
    expect(loud).toHaveClass("bad");
    expect(loud.textContent).toBe("not offered — adjust the selection");
    cleanup();

    // One fact apart: the selected fixture with its preview not legal. A
    // Confirm worked out from `chosen` passes both fixtures and fails this.
    const unlegal = cloneChina(wire.chinatownSelected, (c) => {
      c.preview = { legal: false, words: null };
    });
    expect(chinaPrompt(unlegal).chosen).toBe(1);
    const third = show(unlegal);
    expect(button("Confirm")).toBeDisabled();
    expect(button("Confirm")).toHaveAttribute("data-unavailable");
    expect(button("Cancel")).toBeEnabled();
    expect(button("Cancel")).not.toHaveAttribute("data-unavailable");
    const v = third.container.querySelector(".chooser.chinatown .verdict")!;
    expect(v).toHaveClass("bad");
    expect(v).not.toHaveClass("ok");
    expect(v.textContent).toBe("not offered — adjust the selection");
    expect(v.querySelector("div")).toBeNull();
  });

  it("asks how many exchanges are chosen, in the singular at one", () => {
    const p = chinaPrompt(wire.chinatown);
    expect([p.max_exchanges, p.chosen]).toEqual([3, 0]);
    const first = show(wire.chinatown);
    expect(first.container.querySelector(".asking")!.textContent).toBe(
      "Chinatown — up to 3 exchanges · 0 of 3 chosen",
    );
    expect(first.container.querySelectorAll("button.option")).toHaveLength(0);
    expect(first.container.querySelector(".decision > .chooser.chinatown")).not.toBeNull();
    cleanup();

    const q = chinaPrompt(wire.chinatownSelected);
    expect([q.max_exchanges, q.chosen]).toEqual([3, 1]);
    const second = show(wire.chinatownSelected);
    expect(second.container.querySelector(".asking")!.textContent).toBe(
      "Chinatown — up to 3 exchanges · 1 of 3 chosen",
    );
    cleanup();

    const one = cloneChina(wire.chinatown, (c) => {
      c.max_exchanges = 1;
    });
    const third = show(one);
    expect(third.container.querySelector(".asking")!.textContent).toBe(
      "Chinatown — up to 1 exchange · 0 of 1 chosen",
    );
    cleanup();

    // `chosen` is the payload's, not a sum of the rows' counts (which total 1).
    const two = cloneChina(wire.chinatownSelected, (c) => {
      c.chosen = 2;
    });
    const fourth = show(two);
    expect(fourth.container.querySelector(".asking")!.textContent).toBe(
      "Chinatown — up to 3 exchanges · 2 of 3 chosen",
    );
  });

  it("draws no stepper and no Confirm for a watcher", () => {
    const { container } = show(wire.chinatownSelected, false, true);
    expect(container.querySelector(".chooser")).toBeNull();
    expect(container.querySelectorAll("button.plus, button.minus")).toHaveLength(0);
    expect(screen.queryByRole("button", { name: "Confirm" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Cancel" })).toBeNull();
  });
});

describe("a prompt kind this client cannot draw", () => {
  // A server newer than this client can answer a kind the union does not hold
  // (`port` was such a kind until plan 2b made it a known one, so the cells
  // name a kind no server sends). The client must not be left with nothing
  // to press (ruling R46): it says so and offers Cancel, which every sub-prompt
  // answers and the web's line fence admits. The payload is a real one with ONE
  // fact changed, its kind; its other prompt fields are left as they are.
  const FALLBACK_ASKING = "This screen needs a newer version.";
  const FALLBACK_TEXT =
    "The game asked something this version cannot draw. Take the update from the banner, or cancel to go back to the list.";
  const unknown = (): ClientState => {
    const c = JSON.parse(JSON.stringify(wire.church)) as { prompt: { kind: string } };
    expect(c.prompt.kind).toBe("church");
    c.prompt.kind = "a_kind_no_server_sends";
    return c as unknown as ClientState;
  };

  it("says the screen needs a newer version and offers Cancel alone, which sends cancel", async () => {
    const { container, onSend, user } = show(unknown());
    expect(container.querySelector(".asking")!.textContent).toBe(FALLBACK_ASKING);
    expect(container.querySelector(".unknown-prompt")!.textContent).toBe(FALLBACK_TEXT);
    const buttons = screen.getAllByRole("button");
    expect(buttons.map((b) => b.textContent)).toEqual(["Cancel"]);
    expect(buttons[0]).toBeEnabled();
    await user.click(buttons[0]!);
    expect(onSend.mock.calls).toEqual([["cancel"]]);
    cleanup();

    // In flight, its Cancel is disabled like the others.
    show(unknown(), true);
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
  });

  it("gives a watcher the two lines and no button", () => {
    const { container } = show(unknown(), false, true);
    expect(container.querySelector(".asking")!.textContent).toBe(FALLBACK_ASKING);
    expect(container.querySelector(".unknown-prompt")!.textContent).toBe(FALLBACK_TEXT);
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  it("is never what a kind the corpus holds draws", () => {
    for (const state of EVERY_STATE) {
      const { container } = show(state);
      expect(container.querySelector(".asking")!.textContent, `the ${state.prompt.kind} fixture`).not.toBe(FALLBACK_ASKING);
      expect(container.querySelector(".unknown-prompt"), `the ${state.prompt.kind} fixture`).toBeNull();
      cleanup();
    }
  });
});

describe("the Port's chooser until its panel (plan 2b, Task 2)", () => {
  // `port` is a known kind from Task 2 on, drawn by an interim branch of its
  // own until Task 4's panel: the panel's asking line and Cancel, under
  // `.chooser.port` — never the unknown-kind node. The asking line reads the
  // arm's `max_cards`, so the cell reads the fixture and a clone with that ONE
  // field changed.
  it("asks the panel's question and offers Cancel, which sends cancel", async () => {
    const { container, onSend, user } = show(wire.port);
    expect(container.querySelector(".asking")!.textContent).toBe("The Port — take one card, or two");
    expect(container.querySelector(".chooser.port")).not.toBeNull();
    expect(container.querySelector(".unknown-prompt")).toBeNull();
    const buttons = screen.getAllByRole("button");
    expect(buttons.map((b) => b.textContent)).toEqual(["Cancel"]);
    await user.click(buttons[0]!);
    expect(onSend.mock.calls).toEqual([["cancel"]]);
    cleanup();

    const one = JSON.parse(JSON.stringify(wire.port)) as { prompt: { kind: string; max_cards: number } };
    expect(one.prompt.max_cards).toBe(2);
    one.prompt.max_cards = 1;
    const { container: c1 } = show(one as unknown as ClientState);
    expect(c1.querySelector(".asking")!.textContent).toBe("The Port — take one card");
    cleanup();

    // In flight, its Cancel is disabled like the others; a watcher gets none.
    show(wire.port, true);
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    cleanup();
    const { container: watcher } = show(wire.port, false, true);
    expect(watcher.querySelector(".chooser.port")).toBeNull();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });
});
