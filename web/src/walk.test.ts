// The run a reply added, and the walk over it: a pure reducer, so the timing
// in `App` is one `setTimeout` and every decision is testable without a clock.

import { describe, expect, it } from "vitest";
import { QUICK_DWELL } from "./playback";
import { buildRun, dwellFor, hasBotStep, walkReducer, type WalkState } from "./walk";
import { wire } from "./wire/contract";
import type { HistoricalFrame, LogPanel } from "./types";

const clone = (f: HistoricalFrame): HistoricalFrame => JSON.parse(JSON.stringify(f));

/** Three positions after events 10, 11 and 12 of a game: the first step moves
 *  a cube, the second changes nothing on the table.
 *
 *  The MOVER changes between the first two positions (seat 0 was to move at
 *  event 10, seat 1 at event 11) so that a step reading its mover off the
 *  position AFTER its event is a different answer, not the same one. The
 *  corpus frame carries `to_move: 1`, so `f10` has to be moved off it. */
function fixture(): { frames: HistoricalFrame[]; log: LogPanel } {
  const f10 = clone(wire.atK);
  f10.index = 10;
  f10.to_move = 0;
  const f11 = clone(f10);
  f11.index = 11;
  f11.frame.board.tiles[2]!.assistants[1]![1] += 1;
  f11.to_move = 1;
  f11.frame.decision.current_phase = 1;
  const f12 = clone(f11);
  f12.index = 12;
  return {
    frames: [f10, f11, f12],
    log: {
      total: 12,
      entries: [
        { index: 10, text: "Red: Place 1 assistant in each of: Port A (A02)" },
        { index: 11, text: "Blue: End additional-action phase" },
      ],
    },
  };
}

describe("buildRun", () => {
  it("makes one step per event, moved by whoever was to move BEFORE it, in the phase it was in", () => {
    const { frames, log } = fixture();
    const run = buildRun(10, frames, log);
    expect(run.from).toBe(10);
    expect(run.to).toBe(12);
    expect(run.steps).toHaveLength(2);
    expect(run.steps[0]!.event).toBe(10);
    expect(run.steps[0]!.line).toMatch(/Place 1 assistant/);
    expect(run.steps[0]!.mover).toBe(0);
    expect(run.steps[0]!.mover).toBe(frames[0]!.to_move);
    expect(run.steps[0]!.phase).toBe(
      frames[0]!.frame.decision.phase_chain[frames[0]!.frame.decision.current_phase],
    );
    expect(run.steps[0]!.after).toBe(frames[1]);
    expect(run.steps[1]!.mover).toBe(1);
    expect(run.steps[1]!.before).toBe(frames[1]);
  });

  it("knows a bot's step from the Players panel of the position before it", () => {
    const { frames, log } = fixture();
    expect(hasBotStep(buildRun(10, frames, log))).toBe(false);
    const players = frames[1]!.frame.dashboard.find((p) => "Players" in p);
    if (!players || !("Players" in players)) throw new Error("no Players panel");
    players.Players.players.find((p) => p.seat === 1)!.is_bot = true;
    const run = buildRun(10, frames, log);
    expect(run.steps[1]!.moverIsBot).toBe(true);
    expect(run.steps[0]!.moverIsBot).toBe(false);
    expect(hasBotStep(run)).toBe(true);
  });

  it("dwells the cadence on a changed frame and the short dwell on an unchanged one", () => {
    const { frames, log } = fixture();
    const run = buildRun(10, frames, log);
    expect(run.steps[0]!.changes.any).toBe(true);
    expect(run.steps[1]!.changes.any).toBe(false);
    expect(dwellFor(run.steps[0]!, 3000)).toBe(3000);
    expect(dwellFor(run.steps[1]!, 3000)).toBe(QUICK_DWELL);
  });
});

describe("walkReducer", () => {
  const last = 2; // three steps
  const s = (at: number, playing = true): WalkState => ({ at, playing });
  it("ticks forward and ends after the last step", () => {
    expect(walkReducer(s(0), { type: "tick" }, last)).toEqual(s(1));
    expect(walkReducer(s(2), { type: "tick" }, last)).toBeNull();
  });
  it("pauses, plays, steps back and forward, and stepping pauses", () => {
    expect(walkReducer(s(1), { type: "pause" }, last)).toEqual(s(1, false));
    expect(walkReducer(s(1, false), { type: "play" }, last)).toEqual(s(1, true));
    expect(walkReducer(s(1), { type: "back" }, last)).toEqual(s(0, false));
    expect(walkReducer(s(0), { type: "back" }, last)).toEqual(s(0, false));
    expect(walkReducer(s(1), { type: "forward" }, last)).toEqual(s(2, false));
    expect(walkReducer(s(2), { type: "forward" }, last)).toBeNull();
  });
  it("jumps within the run, paused, and skip ends it", () => {
    expect(walkReducer(s(0), { type: "jump", to: 7 }, last)).toEqual(s(2, false));
    expect(walkReducer(s(2), { type: "jump", to: -3 }, last)).toEqual(s(0, false));
    expect(walkReducer(s(1), { type: "skip" }, last)).toBeNull();
  });
});
