// The polled change signal (M6.1 §6, room plan §1), and since M6.2 the polling
// rule ITSELF (spec §5.5).
//
// **One hook, four consumers.** `usePolled` owns the visibility rule (R-P8),
// the three-second cadence, the backoff and the two refusals that end a poll
// rather than slowing it down; `useSignal` is the instance whose resource is a
// game's `GET …/events`, and the dealing screen, the lobby and the open table
// are the other three. The generalization is what makes M6.3 one change: the
// transport under this hook is replaced — a held request, then a stream — and
// nothing downstream of a consumer's callback knows how the answer arrived.
//
// **The signal is deliberately separate from the fetch** (ruling R-P9).
// `GET …/events` answers four fields and builds no frame, so a client learns
// "this game now has N events" for a few bytes and goes and gets a board only
// when that moved.
//
// **Nothing is asked from a hidden tab** (ruling R-P8). A browser with eight
// backgrounded games is otherwise eight requests every three seconds for as
// long as it is open, for boards nobody is looking at. On return the client
// asks ONCE, at once, and resumes — and says "refreshing" while it does,
// because the board on screen is as old as the tab was hidden and "live" would
// be a claim it cannot make yet.

import { useEffect, useRef, useState } from "react";
import { api, ApiError } from "./api";
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

/** The resource a poll watches (spec §5.5).
 *
 *  **`key` is the effect's ONLY dependency beside `active`**: a stable string
 *  for the resource — a game id, the literal `"lobby"` — while `once` and every
 *  callback ride in refs. A consumer's closure is a new function on every
 *  render (see `usePolled` below), so a `Source` object is rebuilt each time and
 *  its identity must not matter.
 *
 *  **`stream` is carried and UNUSED in M6.2.** M6.3 reads it (its spec §5.1),
 *  which is why the shape is fixed here rather than there: a field added later
 *  is a field every consumer has to be revisited for. */
export interface Source<T> {
  key: string;
  once: () => Promise<T>;
  /** The resource's stream URL. */
  stream: string;
}

/** Why a poll STOPPED, as opposed to failing and backing off.
 *
 *  A 404 is permanent — the game was deleted, the table abandoned — and a 403
 *  is a refusal about who is asking, not about whether the server is reachable.
 *  Retrying either is a client asking a question that already has its final
 *  answer, while the screen goes on saying "reconnecting…" about something that
 *  is not coming back. */
export interface End {
  reason: "deleted" | "gone" | "refused";
  sentence?: string;
}

/**
 * Ask `source.once()` on a timer while the tab is VISIBLE and `active`, and
 * hand each answer to `onValue`.
 *
 * `active` is the caller's "there is something to wait for": a screen holding
 * the prompt has nothing to ask about, and a finished game has nothing to wait
 * for at all. A `source` of `null` is the same answer for "no resource on
 * screen".
 *
 * **`once`, `onValue` and `onEnd` are read through refs, so the effect installs
 * ONCE.** Every consumer's callback closes over state and is therefore a new
 * function on every render; an effect that depended on one would tear the timer
 * down and arm a new one each time, and a poll re-armed faster than its own
 * interval never reaches its first tick. The refs are written during render,
 * which is the idiom `Dealing`'s own hand-over callback already uses.
 *
 * **There is no 401 handling here and there must not be.** `api.ts` routes
 * every 401 to the door from inside `call` (ruling R-7.1), so a signed-out poll
 * unmounts the screen it was polling for; a second handler here would be a
 * second thing racing to decide which screen this is.
 *
 * **This hook does NOT ask on arm.** The first request is one interval away,
 * because every consumer already holds the thing it is watching — the game was
 * fetched to be drawn, the deal to be chosen from. A consumer that arms the
 * poll over nothing (the lobby) fetches once itself; making the hook ask at
 * zero instead would double every other consumer's opening request.
 */
export function usePolled<T>(
  source: Source<T> | null,
  active: boolean,
  onValue: (v: T) => void,
  onEnd?: (end: End) => void,
): Link {
  const [link, setLink] = useState<Link>("live");
  /** Consecutive failures, as a REF: it is read by the next attempt and shown
   *  by nothing, so a state variable would only re-render the whole consumer
   *  once per failed request. */
  const failures = useRef(0);
  const once = useRef(source?.once ?? null);
  once.current = source?.once ?? null;
  const latest = useRef(onValue);
  latest.current = onValue;
  const ended = useRef(onEnd);
  ended.current = onEnd;
  const key = source?.key ?? null;

  useEffect(() => {
    // **The link describes a poll that is RUNNING, so a poll that stops stops
    // saying anything.** Without this the last word stands after the effect is
    // torn down and nothing is left to clear it: switch games from the menu
    // while the server is unreachable and the new game's banner opens reading
    // "reconnecting…" about a request nobody made. A no-op on mount and on
    // every re-run that was already live — React bails on an identical value.
    setLink("live");
    if (!key || !active) return;
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
      if (stopped || !visible() || !once.current) return;
      try {
        const v = await once.current();
        if (stopped) return;
        failures.current = 0;
        setLink("live");
        latest.current(v);
        schedule(VISIBLE_MS);
      } catch (e) {
        if (stopped) return;
        // **The two refusals that END this rather than slowing it down.** The
        // resource is gone (404) or is not ours to read (403); neither changes
        // by being asked again, and nothing is scheduled after this line.
        if (e instanceof ApiError && (e.status === 404 || e.status === 403)) {
          ended.current?.({ reason: e.status === 404 ? "gone" : "refused", sentence: e.message });
          return;
        }
        // **A poll that cannot reach the server is not a refusal to show the
        // player.** The board on screen is still the last true thing this
        // client was told; what is wrong is the link, and that is what the
        // word says. The error itself is deliberately not carried up — a
        // sentence in the error strip every few seconds would bury the
        // refusals that are about a move the player actually made.
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
  }, [key, active]);

  return link;
}

/**
 * The game's change signal — `GET …/events` — as ONE INSTANCE of [`usePolled`].
 *
 * Its contract is unchanged and its cells are the same cells: what moved is
 * where the visibility rule, the cadence and the backoff live, so the lobby and
 * the open table get them without a second copy.
 *
 * `onEnd` is how a deleted game stops being polled: the game screen navigates
 * to the lobby with the sentence rather than reading "reconnecting…" forever.
 */
export function useSignal(
  game: string | null,
  active: boolean,
  onSignal: (e: Events) => void,
  onEnd?: (end: End) => void,
): Link {
  return usePolled<Events>(
    game
      ? {
          key: game,
          once: () => api.events(game),
          stream: `/api/games/${encodeURIComponent(game)}/stream`,
        }
      : null,
    active,
    onSignal,
    onEnd,
  );
}
