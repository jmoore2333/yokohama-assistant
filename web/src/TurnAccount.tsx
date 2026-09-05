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

  // **The lit row is scrolled into view (spec §3.2.5, ruling R-F5).** The box
  // is capped — 9rem below 1201px, measured at 144px holding 396px of rows at
  // 390 — so from about the fifth step the walk was lighting a row inside the
  // scroller's hidden part, and the account said nothing about the step on the
  // table. Above 1201px a long side column has the same problem.
  //
  // `block: "nearest"` rather than `center`: the row is brought just inside the
  // scroller and a row already in view is not moved at all, so a walk does not
  // jerk the account on every step. Keyed on the lit step and the run — never
  // on every render, which would fight a player scrolling by hand — so nothing
  // moves for an account that is not walking, and nothing but the lit row is
  // ever asked to scroll.
  const rows = useRef<(HTMLLIElement | null)[]>([]);
  useEffect(() => {
    if (at === null) return;
    rows.current[at]?.scrollIntoView({ block: "nearest" });
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
      <div id="turn-account-body" className={`messages${folded ? " folded" : ""}`} role="status">
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
