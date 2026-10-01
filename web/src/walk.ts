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
   *  whose chain is empty, or whose `current_phase` indexes past the end of
   *  that chain; neither is true of any frame this server serves. */
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
 *  applies at once (spec §3.2.2).
 *
 *  **The gate on a run that came back as the REPLY to your own move**, where
 *  the only other mover there can be is a bot seat this browser is playing
 *  against. A run that arrived through the signal asks {@link hasOpponentStep}
 *  instead. */
export const hasBotStep = (run: Run): boolean => run.steps.some((s) => s.moverIsBot);

/** The run contains a step this viewer did not play — the gate on a walk that
 *  arrived through the SIGNAL (ruling R-9.1, M6.1 §6 and §7.5).
 *
 *  **`viewer` is nullable, and `null` makes EVERY step an opponent's** (spec
 *  §5.2). A spectator holds no seat, so there is no seat to compare against and
 *  nothing on the table was played by them — which is the whole of what a
 *  watcher is. Every run they are handed is therefore walked, at the cadence
 *  they set, and that is the only picture a spectator screen has to offer.
 *
 *  **It is NOT the same `null` `landed` used to take.** That one meant "this
 *  did not arrive through the signal" and selected {@link hasBotStep}; the two
 *  facts are now carried apart, because a spectator's arrival is a signal
 *  arrival whose viewer is genuinely absent (`App`'s `Arrival`).
 *
 *  **The `viewer === null` arm is REDUNDANT, deliberately, and measured.**
 *  `s.mover !== viewer` is already true of every step when `viewer` is `null` —
 *  a seat number is never equal to it — so removing the arm changes no answer
 *  and no cell here can tell the two apart (probed: the spectator cell below
 *  stays green without it). It is written because the rule is "a watcher holds
 *  no seat", not "a seat number compares unequal to null", and the next person
 *  narrowing this predicate should have to delete a sentence rather than
 *  discover one. What the widening really costs is the TYPE, and the thing that
 *  fails without it is `tsc`, not a test.
 *
 *  **`hasBotStep` is the wrong question at a human table, and that is the whole
 *  reason this exists.** A room is people: no seat at a table of players is a
 *  bot, so a gate on `moverIsBot` never fires for a person's turn and the
 *  opponent's move SNAPS onto the board — M5.5's step-by-step account, the
 *  thing the room plan promises the poll will drive, silently not happening at
 *  exactly the table it was built for. What a walk is FOR is watching somebody
 *  else play; whether that somebody is a person or a program does not change
 *  the picture.
 *
 *  The bot arm stays in the OR rather than being replaced by it: it is the arm
 *  that answers for a seat the viewer's own browser holds, which is every
 *  vs-bot game this client has ever played.
 *
 *  Takes the steps rather than the `Run` its sibling takes, as the ruling
 *  states it: one question about a list, asked by a caller that has the list. */
export const hasOpponentStep = (steps: Step[], viewer: PlayerId | null): boolean =>
  steps.some((s) => s.moverIsBot || viewer === null || s.mover !== viewer);

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

/** The marks a finished turn leaves on the LIVE table (spec §3.2.4, ruling R-F1).
 *
 *  **Not the last step's own diff, and the difference is every real turn.** A
 *  bot's turn closes with "End additional-action phase", whose two frames are
 *  identical, so the run's final `changes` is empty and holding it wiped the
 *  board the instant the walk ended — the spec's purpose clause ("the end state
 *  of the turn is still readable after the walk") delivered for the one run
 *  shape a served turn never has. Measured live three times at the final
 *  review, against a component cell that was green because its fixture's cube
 *  rode the last step.
 *
 *  What is held is the UNION over the steps the run's LAST mover played: the
 *  seat whose turn just finished, everything it did, in that one seat's colour.
 *  A run of the player's OWN events falls out of the same rule rather than
 *  needing an arm — its last mover is the player, and the union is over its own
 *  steps.
 *
 *  **Every grain is unioned, `hand` included**, which is five sets and not the
 *  four a diff of tiles, seats, slots and technologies would name: a port take
 *  by the run's mover puts a card in the viewer's own hand, and the mark on it
 *  is one half of that move. `any` is recomputed from the union rather than
 *  or-ed across the steps, so it stays the statement the field is — whether
 *  these five sets name anything at all. */
export function heldChanges(run: Run): Changes | null {
  const last = run.steps[run.steps.length - 1];
  if (!last) return null;
  const mine = run.steps.filter((s) => s.mover === last.mover).map((s) => s.changes);
  const union = <T,>(pick: (c: Changes) => Set<T>): Set<T> =>
    new Set(mine.flatMap((c) => [...pick(c)]));
  const tiles = union((c) => c.tiles);
  const seats = union((c) => c.seats);
  const slots = union((c) => c.slots);
  const techs = union((c) => c.techs);
  const hand = union((c) => c.hand);
  return {
    tiles,
    seats,
    slots,
    techs,
    hand,
    any: tiles.size + seats.size + slots.size + techs.size + hand.size > 0,
  };
}
