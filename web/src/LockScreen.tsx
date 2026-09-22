// The two things a local game says when this tab is not its writer
// (R-6.5b.5, spec §4).
//
// **One component, because they are one fact from two sides.** A second tab
// opening a game reads that it is open elsewhere and is offered Take over; the
// tab it is taken FROM reads that it was opened elsewhere and is offered Take
// back. The words differ by a tense and a verb, and two components would be
// two places for that pair to drift apart.
//
// **Two shapes, though, and they are not interchangeable.** The second tab has
// no game on screen at all — nothing was read, which is the whole of P-5 — so
// it draws a screen of its own. The losing tab HAS one: the board, the log and
// every panel stay exactly where they are, and what is replaced is the answer
// bar. So this renders either a standalone screen or a strip for the deck,
// and the caller says which by where it puts it.

import { useRef, useState } from "react";
import type { ReactNode } from "react";
import type { LockState } from "./local/useLocalHold";
import { Refusal } from "./Refusal";

/** Which side of the steal this is. `"none"` and `"held"` never reach here —
 *  the caller draws the game instead — so the prop is the narrower pair. */
export type LockFace = Extract<LockState, "elsewhere" | "lost">;

/** The words, in ONE place, read by both shapes. */
const WORDS: Record<LockFace, { says: string; act: string; why: string }> = {
  elsewhere: {
    says: "This game is open in another tab",
    act: "Take over",
    why: "Only one tab plays a game at a time. Taking it over closes it in the other one.",
  },
  lost: {
    says: "This game was opened in another tab",
    act: "Take back",
    why: "That tab is playing it now. Taking it back stops it there and picks up every move it made.",
  },
};

/** The refusal `App` puts in the error strip when a move is attempted after the
 *  loss — the page's own answer, before the Worker is asked (spec §4). */
export const LOST_SENTENCE =
  "this game was opened in another tab — take it back before you play it here";

export function LockScreen({
  state,
  onTakeover,
  onLeave,
  inline = false,
  who,
}: {
  state: LockFace;
  /** Steal the lock. Rejecting is reported here rather than thrown: the local
   *  host is the only thing that can refuse, and it refuses in a sentence. */
  onTakeover: () => Promise<void> | void;
  /** Back to the lobby — the standalone shape only, where there is nothing
   *  else on screen to leave by. */
  onLeave?: () => void;
  /** Draw the strip for the answer deck rather than a screen of its own. */
  inline?: boolean;
  /** The "On this device" strip, built by `App` and merely placed here, as the
   *  other pre-game screens do it. */
  who?: ReactNode;
}) {
  const [error, setError] = useState<string | null>(null);
  // A ref and not a state, for the reason every other guard in this client is
  // one: React batches, so two clicks in a tick both read a flag as false and
  // both steal.
  const inFlight = useRef(false);
  const [busy, setBusy] = useState(false);
  const words = WORDS[state];

  const take = async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    setError(null);
    try {
      await onTakeover();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      inFlight.current = false;
      setBusy(false);
    }
  };

  const button = (
    <button type="button" className="primary" disabled={busy} onClick={() => void take()}>
      {words.act}
    </button>
  );

  if (inline) {
    return (
      <div className="decision locked-out" role="status">
        <span>{words.says}</span>
        {button}
        {error && <Refusal sentence={error} />}
      </div>
    );
  }
  return (
    <div className="setup locked-out">
      {who && <div className="who-strip">{who}</div>}
      <h2>{words.says}</h2>
      <p className="lede">{words.why}</p>
      <div className="field unlock-controls">
        {button}
        {onLeave && (
          <button type="button" className="ghost" onClick={onLeave}>
            Lobby
          </button>
        )}
      </div>
      {error && <Refusal sentence={error} />}
    </div>
  );
}
