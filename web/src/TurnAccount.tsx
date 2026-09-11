// The account of the last turn (M5.5 spec §3.2.5), in the side column: one row
// per event of the run the reply added, the step on the table lit while the
// walk runs, and the walk's controls in the header. With no run — an undo, a
// picker step, a refusal, a cold load — the reply's prose lines, which is what
// this box held before the walk existed and still all it can hold there.
//
// The `role="status"` container stays mounted and empty rather than
// unmounted, because a live region inserted with its text is unreliably
// announced; folding hides it with `max-height: 0`, which keeps it in the tree.
//
// **That is also why the row buttons carry `tabIndex` (ruling R-T8-1).**
// `max-height: 0; overflow: hidden` CLIPS; it does not hide, so a folded
// account leaves its `.log-jump` buttons in the tab order and a keyboard user
// tabs into controls nobody can see. `tabIndex={-1}` takes them out of that
// order while leaving them in the accessibility tree — which is the whole
// point of folding this way rather than unmounting.

import { useEffect, useRef, useState } from "react";
import { seatClass } from "./Cards";
import { CADENCES, type Cadence } from "./playback";
import type { Labels, PlayerColor } from "./types";
import type { Run } from "./walk";

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

export function TurnAccount({
  messages, run, at, playing, cadence, labels, colors,
  onCadence, onPlay, onPause, onBack, onForward, onJump, onSkip, onReplay,
}: {
  messages: string[];
  run: Run | null;
  /** The step on the table while the walk runs; `null` when it is over or never ran. */
  at: number | null;
  playing: boolean;
  cadence: Cadence;
  labels: Labels | null;
  colors: PlayerColor[];
  onCadence: (c: Cadence) => void;
  onPlay: () => void;
  onPause: () => void;
  onBack: () => void;
  onForward: () => void;
  onJump: (step: number) => void;
  onSkip: () => void;
  onReplay: () => void;
}) {
  const [folded, setFolded] = useState(false);
  const last = run?.steps[run.steps.length - 1];
  const walking = run !== null && at !== null;
  const seat = (s: number) => colors[s] ?? `Seat ${s}`;

  // **The lit row is kept in view (spec §3.2.5), and the box below is the ONLY
  // thing that moves (ruling R-F11).** The account is capped — `.turn-account
  // .messages` is `max-height: 9rem; overflow: auto` at every width except
  // inside `@media (min-width: 1201px)`, where the cap is released and `.side`
  // becomes the scroller instead — so below 1201 the walk was lighting a row
  // inside this box's hidden part from about its fifth step.
  //
  // **`scrollIntoView` is what this must not use, and the reason is measured.**
  // It scrolls EVERY scrollable ancestor by specification, and below 1201px the
  // window is one of them: at 1000x523 the page went from 0 to 1826, 1851 and
  // 1877 over three steps, dragging the board the walk was highlighting off
  // screen and parking the lit row at the viewport's bottom edge under the
  // sticky walk strip. The page is the player's to scroll (M5 spec R8), so this
  // sets one `scrollTop` and touches nothing else.
  //
  // The arithmetic is "nearest" restated: a row already inside the box does not
  // move it, and a row outside moves it by the least that brings the row's near
  // edge in. When the cap is released the box is not a scroller
  // (`scrollHeight === clientHeight`) and this does nothing at all, which is
  // the ≥1201 arm. Keyed on the lit step and the run — never on every render,
  // which would fight a player scrolling by hand.
  const body = useRef<HTMLDivElement | null>(null);
  const rows = useRef<(HTMLLIElement | null)[]>([]);
  useEffect(() => {
    if (at === null) return;
    const box = body.current;
    const row = rows.current[at];
    if (!box || !row || box.scrollHeight <= box.clientHeight) return;
    // `offsetTop` is measured from a shared offset parent — neither the box nor
    // the list is positioned — so the difference is the row's offset inside the
    // box, which is the space `scrollTop` is expressed in.
    const top = row.offsetTop - box.offsetTop;
    const bottom = top + row.offsetHeight;
    if (top < box.scrollTop) box.scrollTop = top;
    else if (bottom > box.scrollTop + box.clientHeight) box.scrollTop = bottom - box.clientHeight;
  }, [at, run]);
  return (
    <section className="panel turn-account">
      <h2>
        {last ? (
          <span className={seatClass(colors[last.mover])}>
            {seat(last.mover)}'s turn {last.before.frame.decision.turn_number}
          </span>
        ) : (
          "Last move"
        )}
        <select
          aria-label="Cadence"
          value={cadence}
          onChange={(e) => onCadence(Number(e.currentTarget.value) as Cadence)}
        >
          {CADENCES.map((c) => (
            <option key={c} value={c}>
              {c / 1000} s
            </option>
          ))}
        </select>
        {walking && (
          <>
            <button type="button" className="ghost" aria-label="Step back" onClick={onBack} disabled={at <= 0}>
              &#9664;
            </button>
            {playing ? (
              <button type="button" className="ghost" aria-label="Pause" onClick={onPause}>pause</button>
            ) : (
              <button type="button" className="ghost" aria-label="Play" onClick={onPlay}>play</button>
            )}
            <button type="button" className="ghost" aria-label="Step forward" onClick={onForward}>
              &#9654;
            </button>
            <button type="button" className="ghost" onClick={onSkip}>skip</button>
          </>
        )}
        {run && !walking && (
          <button type="button" className="ghost" onClick={onReplay}>replay</button>
        )}
        <button
          type="button"
          className="ghost fold"
          aria-expanded={!folded}
          aria-controls="turn-account-body"
          onClick={() => setFolded((f) => !f)}
        >
          {folded ? "show" : "hide"}
        </button>
      </h2>
      <div
        id="turn-account-body"
        ref={body}
        className={`messages${folded ? " folded" : ""}`}
        role="status"
      >
        {run ? (
          <ol className="account">
            {run.steps.map((s, i) => (
              <li
                key={s.event}
                ref={(el) => {
                  rows.current[i] = el;
                }}
                className={i === at ? "at" : ""}
                aria-current={i === at ? "step" : undefined}
              >
                <span className="chip phase">{(s.phase && labels?.phases[s.phase]) ?? s.phase ?? ""}</span>
                <span className={`mover ${seatClass(colors[s.mover])}`}>{seat(s.mover)}</span>
                <button
                  type="button"
                  className="log-jump"
                  tabIndex={folded ? -1 : 0}
                  onClick={() => onJump(i)}
                >
                  {s.line}
                </button>
              </li>
            ))}
          </ol>
        ) : (
          messages.map((m, i) => (
            <div key={i} className={isRefusal(m) ? "warn" : ""}>
              {m}
            </div>
          ))
        )}
      </div>
    </section>
  );
}
