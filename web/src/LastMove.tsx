// The applied-move lines — what the CLI prints after every move — in the side
// column rather than in the deck (M5 spec §3.5).
//
// They were the deck's biggest variable: sixteen lines and 112px after a bot's
// turn, in the box that is pinned so that ANSWERING never costs a scroll. The
// answer is what the deck is for; the account of the last move is what the
// column is for. The `role="status"` container stays mounted and empty rather
// than unmounted, because a live region inserted with its text is unreliably
// announced; folding hides it with `max-height: 0`, which keeps it in the tree.

import { useState } from "react";

/** Does this line REFUSE something the player asked for, or merely report it?
 *
 *  It decides whether a message is coloured as a warning, and — since M3.5 —
 *  what a live region announces as a rejected action rather than an ordinary
 *  applied-move delta. It is a HEURISTIC over prose and says so, because the
 *  server hands the CLI's own words through without a typed refusal flag.
 *
 *  Two corrections over the original `/illegal|refus|cannot|not legal|out of
 *  range/i`:
 *
 *  1. **The REPL's own refusals start with a bang and matched none of it.**
 *     There are eighteen such literals in `repl.rs` — `! finish or cancel this
 *     prompt first`, `! pick a number 1-5, or 'cancel'`, `! A05 is not one of
 *     the eligible areas` — and every one of them rendered as an ordinary
 *     message.
 *  2. **A leading bang is not sufficient either**, which is why this is an OR
 *     and not a replacement. `repl.rs:1505` ("That retreat is not in the legal
 *     list (check areas/counts)") and `repl.rs:3469` ("'99' is not a valid area
 *     id (0-17)") carry no bang and match neither half. They are still missed,
 *     and that is recorded here rather than left as a silent gap: closing it
 *     properly means a typed refusal on the wire, not a longer regex. */
export function isRefusal(message: string): boolean {
  return (
    message.trimStart().startsWith("!") ||
    /illegal|refus|cannot|not legal|out of range/i.test(message)
  );
}

export function LastMove({ messages }: { messages: string[] }) {
  const [folded, setFolded] = useState(false);
  return (
    <section className="panel last-move">
      <h2>
        Last move
        <button
          type="button"
          className="ghost fold"
          aria-expanded={!folded}
          aria-controls="last-move-lines"
          onClick={() => setFolded((f) => !f)}
        >
          {folded ? "show" : "hide"}
        </button>
      </h2>
      <div id="last-move-lines" className={`messages ${folded ? "folded" : ""}`} role="status">
        {messages.map((m, i) => (
          <div key={i} className={isRefusal(m) ? "warn" : ""}>
            {m}
          </div>
        ))}
      </div>
    </section>
  );
}
