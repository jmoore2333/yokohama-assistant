// The change signal, in its POLLED form (M6.1 §6, room plan §1).
//
// **One consumer, and that is the point of the shape** (ruling R-P9). The
// signal is deliberately separate from the fetch: `GET …/events` answers four
// fields and builds no frame, so a client learns "this game now has N events"
// for a few bytes and goes and gets a board only when that moved. M6.3 replaces
// what is INSIDE this hook — a held request, then a stream — and nothing
// downstream of `onSignal` changes, because nothing downstream of it knows how
// the signal arrived.
//
// **Nothing is asked from a hidden tab** (ruling R-P8). A browser with eight
// backgrounded games is otherwise eight requests every three seconds for as
// long as it is open, for boards nobody is looking at. On return the client
// asks ONCE, at once, and resumes — and says "refreshing" while it does,
// because the board on screen is as old as the tab was hidden and "live" would
// be a claim it cannot make yet.

import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import type { Events } from "./types";

/** How often a visible, active tab asks. */
export const VISIBLE_MS = 3000;

/** The wait after the 1st, 2nd, … consecutive failure, and then forever.
 *
 *  **It ends at a ceiling rather than growing without bound**, because the
 *  thing on the other end is a home server behind Cloudflare Access: a client
 *  that backed off to ten minutes would leave a player staring at a stale board
 *  long after the server came back, and one that never backed off would hammer
 *  it hardest exactly while it was least able to answer. */
export const BACKOFF_MS = [2000, 4000, 8000, 16000, 30000] as const;

/** What the poll can currently say for itself.
 *
 *  `live` is the resting state and shows NOTHING on screen — a permanent green
 *  dot is a thing people stop seeing. The other two are worth a word each. */
export type Link = "live" | "refreshing" | "reconnecting";

/** How long to wait after `failures` consecutive failures. Clamped at both
 *  ends, so a caller can hand it any count including a negative one. */
export function nextDelay(failures: number): number {
  return BACKOFF_MS[Math.min(Math.max(failures, 0), BACKOFF_MS.length - 1)]!;
}

/**
 * Ask `GET …/events` on a timer while the tab is VISIBLE and `active`, and hand
 * each answer to `onSignal`.
 *
 * `active` is the caller's "there is something to wait for": a screen holding
 * the prompt has nothing to ask about, and a finished game has nothing to wait
 * for at all. `game` of `null` is the same answer for "no game on screen".
 *
 * **`onSignal` is read through a ref, so the effect installs ONCE.** Every
 * consumer's callback closes over state and is therefore a new function on
 * every render; an effect that depended on it would tear the timer down and
 * arm a new one each time, and a poll re-armed faster than its own interval
 * never reaches its first tick. The ref is written during render, which is the
 * idiom `Dealing`'s own hand-over callback already uses.
 *
 * **There is no 401 handling here and there must not be.** `api.ts` routes
 * every 401 to the door from inside `call` (ruling R-7.1), so a signed-out poll
 * unmounts the screen it was polling for; a second handler here would be a
 * second thing racing to decide which screen this is.
 */
export function useSignal(
  game: string | null,
  active: boolean,
  onSignal: (e: Events) => void,
): Link {
  const [link, setLink] = useState<Link>("live");
  /** Consecutive failures, as a REF: it is read by the next attempt and shown
   *  by nothing, so a state variable would only re-render the whole consumer
   *  once per failed request. */
  const failures = useRef(0);
  const latest = useRef(onSignal);
  latest.current = onSignal;

  useEffect(() => {
    // **The link describes a poll that is RUNNING, so a poll that stops stops
    // saying anything.** Without this the last word stands after the effect is
    // torn down and nothing is left to clear it: switch games from the menu
    // while the server is unreachable and the new game's banner opens reading
    // "reconnecting…" about a request nobody made. A no-op on mount and on
    // every re-run that was already live — React bails on an identical value.
    setLink("live");
    if (!game || !active) return;
    let timer: ReturnType<typeof setTimeout> | null = null;
    // **The cleanup's flag, and it is what makes a request in flight harmless.**
    // Clearing the timer stops the NEXT ask; it does nothing about the one
    // already awaiting a response, which would otherwise land after the screen
    // is gone, set state on an unmounted tree and — worse — schedule a
    // successor that no cleanup is left to cancel.
    let stopped = false;
    const visible = () => document.visibilityState === "visible";

    const schedule = (ms: number) => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => void ask(), ms);
    };

    const ask = async () => {
      if (stopped || !visible()) return;
      try {
        const e = await api.events(game);
        if (stopped) return;
        failures.current = 0;
        setLink("live");
        latest.current(e);
        schedule(VISIBLE_MS);
      } catch {
        // **A poll that cannot reach the server is not a refusal to show the
        // player.** The board on screen is still the last true thing this
        // client was told; what is wrong is the link, and that is what the
        // word says. The error itself is deliberately not carried up — a
        // sentence in the error strip every few seconds would bury the
        // refusals that are about a move the player actually made.
        if (stopped) return;
        setLink("reconnecting");
        const wait = nextDelay(failures.current);
        failures.current += 1;
        schedule(wait);
      }
    };

    const onVisibility = () => {
      if (visible()) {
        setLink("refreshing");
        // At once, and not on the next tick: the whole point of stopping while
        // hidden is that coming back is when you want to know.
        schedule(0);
      } else if (timer) {
        clearTimeout(timer);
        timer = null;
      }
    };

    document.addEventListener("visibilitychange", onVisibility);
    if (visible()) schedule(VISIBLE_MS);
    return () => {
      stopped = true;
      if (timer) clearTimeout(timer);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [game, active]);

  return link;
}
