// The run a reply added, and the walk over it (M5.5 spec §3.2.2).
//
// A RUN is the events `[from, to)` a reply applied, each with the position
// before and after it and the words the log gave it. The mover and the phase
// of an event are read off the position BEFORE it — what `/at/{k}` already
// serves — so no seat is ever parsed out of "Blue:" prose. The WALK over a run
// is a pure reducer; `App` owns the one timer.

import { diffFrames, type Changes } from "./diffFrames";
import { QUICK_DWELL } from "./playback";
import type { HistoricalFrame, LogPanel, Panel, PlayerId, TurnPhase } from "./types";

export interface Step {
  /** The event's index in the record. */
  event: number;
  /** The log's line for it, worded as it read when it applied. */
  line: string;
  mover: PlayerId;
  moverIsBot: boolean;
  /** The phase the mover was in when it decided — `null` only on a payload
   *  whose chain is empty, which no served frame has. */
  phase: TurnPhase | null;
  before: HistoricalFrame;
  after: HistoricalFrame;
  changes: Changes;
}

export interface Run {
  from: number;
  to: number;
  steps: Step[];
}

/** `frames[i]` is the position after `from + i` events, for `i` in
 *  `0..=to−from`; `log` carries at least the lines for `from..to`. */
export function buildRun(from: number, frames: HistoricalFrame[], log: LogPanel): Run {
  const to = from + frames.length - 1;
  const steps: Step[] = [];
  for (let e = from; e < to; e++) {
    const before = frames[e - from]!;
    const after = frames[e - from + 1]!;
    const seats =
      before.frame.dashboard.find((p): p is Extract<Panel, { Players: unknown }> => "Players" in p)
        ?.Players.players ?? [];
    const mover = before.to_move;
    const d = before.frame.decision;
    steps.push({
      event: e,
      line: log.entries.find((x) => x.index === e)?.text ?? `event ${e}`,
      mover,
      moverIsBot: seats.some((p) => p.seat === mover && p.is_bot),
      phase: d.phase_chain[d.current_phase] ?? null,
      before,
      after,
      changes: diffFrames(before, after),
    });
  }
  return { from, to, steps };
}

/** The walk plays only when the opponent moved: a run of your own events
 *  applies at once (spec §3.2.2). */
export const hasBotStep = (run: Run): boolean => run.steps.some((s) => s.moverIsBot);

/** How long a step stays on the table. */
export const dwellFor = (step: Step, cadence: number): number =>
  step.changes.any ? cadence : QUICK_DWELL;

export interface WalkState {
  /** The step on the table: an index into `run.steps`. */
  at: number;
  playing: boolean;
}

export type WalkAction =
  | { type: "tick" }
  | { type: "play" }
  | { type: "pause" }
  | { type: "back" }
  | { type: "forward" }
  | { type: "jump"; to: number }
  | { type: "skip" };

/** `last` is the index of the run's final step. `null` means the walk is over
 *  and the live state is what is on screen. */
export function walkReducer(s: WalkState, a: WalkAction, last: number): WalkState | null {
  switch (a.type) {
    case "tick":
      return s.at >= last ? null : { ...s, at: s.at + 1 };
    case "play":
      return { ...s, playing: true };
    case "pause":
      return { ...s, playing: false };
    case "back":
      return { at: Math.max(0, s.at - 1), playing: false };
    case "forward":
      return s.at >= last ? null : { at: s.at + 1, playing: false };
    case "jump":
      return { at: Math.max(0, Math.min(last, a.to)), playing: false };
    case "skip":
      return null;
  }
}
