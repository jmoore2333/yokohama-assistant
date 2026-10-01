// The run a reply added, and the walk over it: a pure reducer, so the timing
// in `App` is one `setTimeout` and every decision is testable without a clock.

import { describe, expect, it } from "vitest";
import { QUICK_DWELL } from "./playback";
import {
  buildRun,
  dwellFor,
  hasBotStep,
  hasOpponentStep,
  walkReducer,
  type WalkState,
} from "./walk";
import { wire } from "./wire/contract";
import type { HistoricalFrame, LogPanel, PlayerId } from "./types";

const clone = (f: HistoricalFrame): HistoricalFrame => JSON.parse(JSON.stringify(f));

/** Mark a seat a bot in ONE position's Players panel. Per position, because
 *  which seat is a bot is read off the position before each event — a helper
 *  that wrote every frame at once could not tell the two apart. */
function markBot(f: HistoricalFrame, seat: PlayerId): void {
  const players = f.frame.dashboard.find((p) => "Players" in p);
  if (!players || !("Players" in players)) throw new Error("no Players panel");
  players.Players.players.find((p) => p.seat === seat)!.is_bot = true;
}

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
    // Seat 1 is a bot in BOTH positions, and step 0's mover is seat 0. So a
    // step that answered "is ANY seat here a bot" rather than "is the MOVER a
    // bot" would read true for step 0, where the cell wants false.
    markBot(frames[0]!, 1);
    markBot(frames[1]!, 1);
    const run = buildRun(10, frames, log);
    expect(run.steps[1]!.moverIsBot).toBe(true);
    expect(run.steps[0]!.moverIsBot).toBe(false);
    expect(hasBotStep(run)).toBe(true);
  });

  // **The gate on a walk that arrived through the SIGNAL (ruling R-9.1), and
  // it is a different question from the one above.** A room is people: no seat
  // at a human table is a bot, so a run gated on `hasBotStep` never plays and
  // the opponent's turn snaps onto the table instead. What the walk is for is
  // watching somebody else play, and who that somebody is does not change the
  // picture.
  it("knows a step this viewer did not play, bot or person", () => {
    const { frames, log } = fixture();
    const { steps } = buildRun(10, frames, log);
    // The two steps are moved by seat 0 and seat 1, so each seat is an
    // opponent of the other — from either chair there is something to watch.
    expect(steps.map((s) => s.mover)).toEqual([0, 1]);
    expect(hasOpponentStep(steps, 0)).toBe(true);
    expect(hasOpponentStep(steps, 1)).toBe(true);

    // A run every step of which this viewer played: their OWN move, which
    // applies at once and is never walked.
    const mine = fixture();
    mine.frames.forEach((f) => (f.to_move = 0));
    const own = buildRun(10, mine.frames, mine.log).steps;
    expect(own.map((s) => s.mover)).toEqual([0, 0]);
    expect(hasOpponentStep(own, 0)).toBe(false);

    // **And the bot arm still carries.** It is not redundant with the seat
    // comparison: `moverIsBot` is the arm that answers for a seat a browser
    // holds itself, which is exactly the vs-bot game every other cell is about.
    mine.frames.forEach((f) => markBot(f, 0));
    expect(hasOpponentStep(buildRun(10, mine.frames, mine.log).steps, 0)).toBe(true);
  });

  // **A WATCHER has no seat, so every step is somebody else's** (spec §5.2).
  //
  // The fixture is the hardest shape for it: a run whose every step is played
  // by ONE human mover, and no bot anywhere. From that mover's own chair there
  // is nothing to watch — the cell above pins exactly that, `false` — so a
  // spectator answered by comparing against a seat would be handed the one
  // answer that leaves the table snapping instead of walking. `null` is not a
  // seat, and the run is theirs to watch whoever played it.
  it("walks every step for a viewer holding no seat", () => {
    const mine = fixture();
    mine.frames.forEach((f) => (f.to_move = 0));
    const own = buildRun(10, mine.frames, mine.log).steps;
    expect(own.map((s) => s.mover)).toEqual([0, 0]);
    expect(own.some((s) => s.moverIsBot)).toBe(false);
    // The same steps, read from the two chairs: the mover's, and nobody's.
    expect(hasOpponentStep(own, 0)).toBe(false);
    expect(hasOpponentStep(own, null)).toBe(true);
    // **And this cell cannot see the `viewer === null` arm, which is worth
    // saying out loud.** `0 !== null` is already true, so the arm is redundant
    // and dropping it leaves this green — probed. What the ARM buys is the
    // stated rule; what the widening buys is the type. The mutation that has a
    // witness is at the CALL SITE, where a spectator's arrival could still take
    // the bot branch, and `App.test.tsx` is where it is caught.
  });

  it("dwells the cadence on a changed frame and the short dwell on an unchanged one", () => {
    const { frames, log } = fixture();
    const run = buildRun(10, frames, log);
    expect(run.steps[0]!.changes.any).toBe(true);
    expect(run.steps[1]!.changes.any).toBe(false);
    expect(dwellFor(run.steps[0]!, 3000)).toBe(3000);
    expect(dwellFor(run.steps[1]!, 3000)).toBe(QUICK_DWELL);
    // A second cadence, so the dwell is read from the ARGUMENT and not from a
    // constant that happens to equal the only value ever passed.
    expect(dwellFor(run.steps[0]!, 1000)).toBe(1000);
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
    expect(walkReducer(s(0), { type: "jump", to: 1 }, last)).toEqual(s(1, false));
    expect(walkReducer(s(0), { type: "jump", to: 7 }, last)).toEqual(s(2, false));
    expect(walkReducer(s(2), { type: "jump", to: -3 }, last)).toEqual(s(0, false));
    expect(walkReducer(s(1), { type: "skip" }, last)).toBeNull();
  });
});
