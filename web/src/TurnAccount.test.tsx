// The account of the last turn: the run a reply added, and — with no run — the
// reply's prose lines. The four cells that pinned this box before it grew a run
// are carried here (the live-region pair merged into one, keeping the
// unfolded-className assertion), and the run's own cells join them.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TurnAccount, isRefusal } from "./TurnAccount";
import { buildRun } from "./walk";
import { wire } from "./wire/contract";
import type { HistoricalFrame, PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];
const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const clone = (f: HistoricalFrame): HistoricalFrame => JSON.parse(JSON.stringify(f));

const noop = () => {};
const handlers = {
  onCadence: noop, onPlay: noop, onPause: noop, onBack: noop, onForward: noop,
  onJump: noop, onSkip: noop, onReplay: noop,
};

/** A two-step run by seat 1, whose Players row says it is a bot. `phase` moves
 *  it off the fixture's own `current_phase`, which is the only way to tell a
 *  phase LABEL from the wire key it is looked up by. */
function botRun(phase?: number) {
  const f0 = clone(wire.atK);
  f0.to_move = 1;
  if (phase !== undefined) f0.frame.decision.current_phase = phase;
  const players = f0.frame.dashboard.find((p) => "Players" in p);
  if (!players || !("Players" in players)) throw new Error("no Players panel");
  players.Players.players.find((p) => p.seat === 1)!.is_bot = true;
  const f1 = clone(f0);
  f1.frame.board.tiles[0]!.assistants[1]![1] += 2;
  const f2 = clone(f1);
  f2.frame.decision.current_phase += 1;
  return buildRun(20, [f0, f1, f2], {
    total: 22,
    entries: [
      { index: 20, text: "Blue: Place 2 assistants in Tea Plantation (A00)" },
      { index: 21, text: "Blue: Return President to hand (recover no assistants)" },
    ],
  });
}

function show(over: Partial<Parameters<typeof TurnAccount>[0]>) {
  return render(
    <TurnAccount
      messages={[]}
      run={null}
      at={null}
      playing={false}
      cadence={2000}
      labels={wire.labels}
      colors={COLORS}
      {...handlers}
      {...over}
    />,
  );
}

describe("the turn account with a run", () => {
  it("is headed by the mover's turn, one row per event with its phase and its line", () => {
    const run = botRun();
    show({ run });
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(/Blue's turn \d+/);
    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent(/Place 2 assistants in Tea Plantation/);
    const phaseKey = run.steps[0]!.phase!;
    expect(rows[0]).toHaveTextContent(wire.labels.phases[phaseKey] ?? phaseKey);
    expect(rows[0]!.querySelector(".mover")?.className).toMatch(/seat-blue/);
  });

  it("lights the step on the table and offers the walk's controls while it runs", async () => {
    const run = botRun();
    const onPause = vi.fn();
    const onSkip = vi.fn();
    const onForward = vi.fn();
    show({ run, at: 1, playing: true, onPause, onSkip, onForward });
    const rows = screen.getAllByRole("listitem");
    expect(rows[1]).toHaveClass("at");
    expect(rows[0]).not.toHaveClass("at");
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Pause" }));
    await user.click(screen.getByRole("button", { name: "Step forward" }));
    await user.click(screen.getByRole("button", { name: "skip" }));
    expect(onPause).toHaveBeenCalledTimes(1);
    expect(onForward).toHaveBeenCalledTimes(1);
    expect(onSkip).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: "replay" })).toBeNull();
  });

  it("offers replay once the walk is over, and a row jumps to its step", async () => {
    const run = botRun();
    const onReplay = vi.fn();
    const onJump = vi.fn();
    show({ run, at: null, onReplay, onJump });
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "replay" }));
    expect(onReplay).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: /Return President to hand/ }));
    expect(onJump).toHaveBeenCalledWith(1);
    expect(screen.queryByRole("button", { name: "skip" })).toBeNull();
  });

  // **The phase is shown in the labels' WORDS, not in the wire's key**, and the
  // cell above cannot see that: its fixture sits in `Building`, whose label is
  // the string "Building" as well, so replacing the lookup with `s.phase` alone
  // leaves that cell green — measured, not supposed. `AreaAction` labels as
  // "Area action" and is what separates them.
  it("words the phase from the labels, not from the wire's key", () => {
    const run = botRun(3);
    expect(run.steps[0]!.phase, "phase_chain[3] must be AreaAction").toBe("AreaAction");
    expect(wire.labels.phases.AreaAction, "its label must differ from its key").toBe("Area action");
    show({ run });
    const rows = screen.getAllByRole("listitem");
    expect(rows[0]).toHaveTextContent("Area action");
    expect(rows[0]!.textContent).not.toMatch(/AreaAction/);
  });

  // Paused, the header offers Play where it offered Pause. `Step back` is
  // exercised here and nowhere else: the cell above never clicks it and never
  // reads its `disabled`, so a Step back wired to any other handler — measured
  // against `onPause` — left the whole suite green.
  it("offers Play while paused, and Step back is its own control, dead at the first step", async () => {
    const run = botRun();
    const onPlay = vi.fn();
    const onBack = vi.fn();
    const onPause = vi.fn();
    const user = userEvent.setup();
    const first = show({ run, at: 0, playing: false, onPlay, onBack, onPause });
    expect(screen.queryByRole("button", { name: "Pause" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Play" }));
    expect(onPlay).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "Step back" })).toBeDisabled();
    first.unmount();
    show({ run, at: 1, playing: false, onPlay, onBack, onPause });
    await user.click(screen.getByRole("button", { name: "Step back" }));
    expect(onBack).toHaveBeenCalledTimes(1);
    expect(onPause).not.toHaveBeenCalled();
  });

  it("remembers the cadence the player picks", async () => {
    const onCadence = vi.fn();
    show({ onCadence });
    await userEvent.setup().selectOptions(screen.getByLabelText("Cadence"), "3000");
    expect(onCadence).toHaveBeenCalledWith(3000);
  });
});

describe("the turn account without a run", () => {
  it("holds every prose line the server sent, refusals as warnings", () => {
    const state = wire.refused;
    const refusals = state.messages.filter(isRefusal);
    expect(refusals.length, "the fixture must carry a refusal").toBeGreaterThan(0);
    show({ messages: state.messages });
    for (const m of state.messages) expect(screen.getByText(new RegExp(escape(m)))).toBeInTheDocument();
    for (const m of refusals) expect(screen.getByText(new RegExp(escape(m)))).toHaveClass("warn");
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent("Last move");
  });

  // **The leading-bang branch, which the prose regex misses entirely.**
  // `repl.rs` refuses with eighteen literals that start with a bang and contain
  // none of `illegal|refus|cannot|not legal|out of range`, so before that branch
  // every one of them rendered as an ordinary applied-move line. The regex is
  // asserted to MISS the sample first, or this cell would pass through the other
  // half of the OR and prove nothing about the branch it is named for.
  it("warns on a bang-prefixed refusal the prose regex cannot see", () => {
    const bang = "! A05 is not one of the eligible areas";
    expect(/illegal|refus|cannot|not legal|out of range/i.test(bang)).toBe(false);
    show({ messages: [bang, "Turn 1 Red -> Turn 2 Blue"] });
    expect(screen.getByText(new RegExp(escape(bang)))).toHaveClass("warn");
    expect(screen.getByText(/Turn 1 Red/)).not.toHaveClass("warn");
  });

  // A live region has to exist BEFORE its text arrives to be announced, and it
  // has to survive the fold — which is why folding is `max-height: 0` and not
  // an unmount. The unfolded className is asserted to be exactly `messages`,
  // with no trailing space left behind by the folded arm: the CSS does not
  // care, but anything reading the className back does.
  it("keeps its live region mounted while empty, and folds without unmounting it", async () => {
    const { container } = show({});
    const region = container.querySelector('[role="status"]');
    expect(region).not.toBeNull();
    expect(region!.textContent).toBe("");
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "hide" }));
    expect(container.querySelector(".messages.folded")).not.toBeNull();
    expect(container.querySelector('[role="status"]')).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "show" }));
    expect(container.querySelector(".messages.folded")).toBeNull();
    expect(container.querySelector('[role="status"]')!.className).toBe("messages");
  });
});
