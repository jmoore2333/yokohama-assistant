// The edge's check, as an overlay (cycle F, R-F6; spec §2.6; R-FP8).
//
// **The seam's one handler.** Every site that talks to the server asks
// `passChallenge()` when the edge answers with its Managed Challenge, and this
// is what that opens: the Turnstile widget, once, under one sentence and a
// Cancel. Concurrent challenges share the pass (`turnstile.ts`), so a person
// sees one widget however many requests met the edge together.
//
// **Mounted only in a build with a site key** (`main.tsx`, beside `Footer`): a
// build with none mounts no overlay, so the seam has no handler and a
// challenged request is the ordinary refusal at once, naming the edge
// (`EDGE_CHECK`). Nothing here loads anything until a challenge arrives: the
// widget's script is asked for when the overlay opens, never at mount.

import { useEffect, useId, useRef, useState } from "react";
import { challengeOnce, setChallengeHandler } from "./turnstile";

/** The words the overlay says while the widget works. */
export const CHECKING = "Checking that this is a person…";

/** A challenge being passed: the seam's promise, settled when the widget is. */
interface Asked {
  resolve: () => void;
  reject: (e: Error) => void;
}

export function Challenge(): React.ReactNode {
  const [asked, setAsked] = useState<Asked | null>(null);
  /** The pass still owed an answer, so an unmount can refuse it rather than
   *  leave every waiting request waiting for ever. */
  const owed = useRef<Asked | null>(null);
  const widget = useRef<HTMLDivElement | null>(null);
  const dialog = useRef<HTMLDivElement | null>(null);
  /** Cancel's lever on the widget in progress. */
  const cancel = useRef<AbortController | null>(null);
  const titleId = useId();

  // The seam: registered while this is mounted, and cleared — with any pass in
  // progress refused — when it is not.
  useEffect(() => {
    setChallengeHandler(
      () =>
        new Promise<void>((resolve, reject) => {
          const a = { resolve, reject };
          owed.current = a;
          setAsked(a);
        }),
    );
    return () => {
      setChallengeHandler(null);
      owed.current?.reject(new Error("the check was closed"));
      owed.current = null;
    };
  }, []);

  // The widget, rendered once the overlay is drawn and its container exists;
  // taken down by `challengeOnce` whichever way it ends.
  useEffect(() => {
    const el = widget.current;
    if (!asked || !el) return;
    const abort = new AbortController();
    cancel.current = abort;
    let live = true;
    dialog.current?.focus();
    const settle = (failure?: Error) => {
      if (!live) return;
      live = false;
      if (owed.current === asked) owed.current = null;
      setAsked(null);
      if (failure) asked.reject(failure);
      else asked.resolve();
    };
    challengeOnce(el, abort.signal).then(
      () => settle(),
      (e: unknown) => settle(e instanceof Error ? e : new Error(String(e))),
    );
    return () => {
      // A re-run or an unmount: this widget is over, and the answer (if one
      // is still owed) is the unmount's, above.
      live = false;
      abort.abort();
      if (cancel.current === abort) cancel.current = null;
    };
  }, [asked]);

  if (!asked) return null;
  return (
    <div className="challenge-backdrop">
      <div
        ref={dialog}
        className="challenge raised-card"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onKeyDown={(e) => {
          if (e.key === "Escape") cancel.current?.abort();
        }}
      >
        <p id={titleId} className="challenge-title">
          {CHECKING}
        </p>
        <div ref={widget} className="challenge-widget" />
        <button type="button" className="ghost" onClick={() => cancel.current?.abort()}>
          Cancel
        </button>
      </div>
    </div>
  );
}
