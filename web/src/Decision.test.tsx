// The decision zone: what the player is asked, and what they may answer.
//
// Every state below comes from `src/wire/` — the server emitted it and
// `wire_fixtures.rs` refuses to let it drift. Nothing here builds a payload.

import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Decision } from "./Decision";
import { wire } from "./wire/contract";
import type { ClientState, PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

function show(state: ClientState, busy = false, spectating = false) {
  const onSend = vi.fn();
  const { container } = render(
    <Decision
      state={state}
      labels={wire.labels}
      colors={COLORS}
      busy={busy}
      spectating={spectating}
      onSend={onSend}
    />,
  );
  return { onSend, user: userEvent.setup(), container };
}

describe("the numbered list", () => {
  // THE fact the whole client is built around. `prompt.rows` is the REPL's
  // GROUPED list; `frame.decision.options` is one entry per legal move.
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
